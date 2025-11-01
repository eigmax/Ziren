//! Implementation to check that b * c = product.
//!
//! We first extend the operands to 64 bits. We sign-extend them if the op code is signed. Then we
//! calculate the un-carried product and propagate the carry. Finally, we check that the appropriate
//! bits of the product match the result.
//!
//! b_64 = sign_extend(b) if signed operation else b
//! c_64 = sign_extend(c) if signed operation else c
//!
//! m = []
//! # 64-bit integers have 8 limbs.
//! # Calculate un-carried product.
//! for i in 0..8:
//!     for j in 0..8:
//!         if i + j < 8:
//!             m[i + j] += b_64[i] * c_64[j]
//!
//! # Propagate carry
//! for i in 0..8:
//!     x = m[i]
//!     if i > 0:
//!         x += carry[i - 1]
//!     carry[i] = x / 256
//!     m[i] = x % 256
//!
//! assert_eq(a, m[0..4])
//!
//! if mult or multu:
//!     assert_eq(hi, m[4..8])

mod utils;

use core::{
    borrow::{Borrow, BorrowMut},
    mem::size_of,
};

use hashbrown::HashMap;
use p3_air::{Air, AirBuilder, BaseAir};
use p3_field::{FieldAlgebra, PrimeField32};
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use p3_maybe_rayon::prelude::{ParallelBridge, ParallelIterator, ParallelSlice};
use zkm_core_executor::{
    events::{ByteLookupEvent, ByteRecord, CompAluEvent, MemoryAccessPosition, MemoryRecordEnum},
    ByteOpcode, ExecutionRecord, Opcode, Program,
};
use zkm_derive::AlignedBorrow;
use zkm_primitives::consts::WORD_SIZE;
use zkm_stark::{air::MachineAir, Word};

use crate::{
    air::{WordAirBuilder, ZKMCoreAirBuilder},
    alu::mul::utils::get_msb,
    memory::{MemoryCols, MemoryReadWriteCols},
    utils::{next_power_of_two, zeroed_f_vec},
};

/// The number of main trace columns for `MulChip`.
pub const NUM_MUL_COLS: usize = size_of::<MulCols<u8>>();

/// The number of digits in the product is at most the sum of the number of digits in the
/// multiplicands.
const PRODUCT_SIZE: usize = 2 * WORD_SIZE;

/// The number of bits in a byte.
const BYTE_SIZE: usize = 8;

/// The mask for a byte.
const BYTE_MASK: u8 = 0xff;

/// A chip that implements multiplication for the opcode MUL, MULT and MULTU.
#[derive(Default)]
pub struct MulChip;

/// The column layout for the chip.
#[derive(AlignedBorrow, Default, Debug, Clone, Copy)]
#[repr(C)]
pub struct MulCols<T> {
    /// The current/next pc, used for instruction lookup table.
    pub pc: T,
    pub next_pc: T,

    /// The upper bits of the output operand.
    pub hi: Word<T>,

    /// The output operand.
    pub a: Word<T>,

    /// The first input operand.
    pub b: Word<T>,

    /// The second input operand.
    pub c: Word<T>,

    /// Trace.
    pub carry: [T; PRODUCT_SIZE],

    /// An array storing the product of `b * c` after the carry propagation.
    pub product: [T; PRODUCT_SIZE],

    /// The most significant bit of `b`.
    pub b_msb: T,

    /// The most significant bit of `c`.
    pub c_msb: T,

    /// The sign extension of `b`.
    pub b_sign_extend: T,

    /// The sign extension of `c`.
    pub c_sign_extend: T,

    /// Flag indicating whether the opcode is `MUL`.
    pub is_mul: T,

    /// Flag indicating whether the opcode is `MULT`.
    pub is_mult: T,

    /// Flag indicating whether the opcode is `MULTU`.
    pub is_multu: T,

    /// Selector to know whether this row is enabled.
    pub is_real: T,

    /// Access to hi register
    pub op_hi_access: MemoryReadWriteCols<T>,

    /// Flag indicating whether the hi_access record is real.
    pub hi_record_is_real: T,

    /// The shard number.
    pub shard: T,
    /// The clock cycle number.
    pub clk: T,
}

impl<F: PrimeField32> MachineAir<F> for MulChip {
    type Record = ExecutionRecord;

    type Program = Program;

    fn name(&self) -> String {
        "Mul".to_string()
    }

    fn generate_trace(
        &self,
        input: &ExecutionRecord,
        _: &mut ExecutionRecord,
    ) -> RowMajorMatrix<F> {
        // Generate the trace rows for each event.
        let nb_rows = input.mul_events.len();
        let size_log2 = input.fixed_log2_rows::<F, _>(self);
        let padded_nb_rows = next_power_of_two(nb_rows, size_log2);
        let mut values = zeroed_f_vec(padded_nb_rows * NUM_MUL_COLS);
        let chunk_size = std::cmp::max((nb_rows + 1) / num_cpus::get(), 1);

        values.chunks_mut(chunk_size * NUM_MUL_COLS).enumerate().par_bridge().for_each(
            |(i, rows)| {
                rows.chunks_mut(NUM_MUL_COLS).enumerate().for_each(|(j, row)| {
                    let idx = i * chunk_size + j;
                    let cols: &mut MulCols<F> = row.borrow_mut();

                    if idx < nb_rows {
                        let mut byte_lookup_events = Vec::new();
                        let event = &input.mul_events[idx];
                        self.event_to_row(event, cols, &mut byte_lookup_events);
                    }
                });
            },
        );

        // Convert the trace to a row major matrix.

        RowMajorMatrix::new(values, NUM_MUL_COLS)
    }

    fn generate_dependencies(&self, input: &Self::Record, output: &mut Self::Record) {
        let chunk_size = std::cmp::max(input.mul_events.len() / num_cpus::get(), 1);

        let blu_batches = input
            .mul_events
            .par_chunks(chunk_size)
            .map(|events| {
                let mut blu: HashMap<ByteLookupEvent, usize> = HashMap::new();
                events.iter().for_each(|event| {
                    let mut row = [F::ZERO; NUM_MUL_COLS];
                    let cols: &mut MulCols<F> = row.as_mut_slice().borrow_mut();
                    self.event_to_row(event, cols, &mut blu);
                });
                blu
            })
            .collect::<Vec<_>>();

        output.add_byte_lookup_events_from_maps(blu_batches.iter().collect::<Vec<_>>());
    }

    fn included(&self, shard: &Self::Record) -> bool {
        if let Some(shape) = shard.shape.as_ref() {
            shape.included::<F, _>(self)
        } else {
            !shard.mul_events.is_empty()
        }
    }

    fn local_only(&self) -> bool {
        true
    }
}

impl MulChip {
    /// Create a row from an event.
    fn event_to_row<F: PrimeField32>(
        &self,
        event: &CompAluEvent,
        cols: &mut MulCols<F>,
        blu: &mut impl ByteRecord,
    ) {
        cols.pc = F::from_canonical_u32(event.pc);
        cols.next_pc = F::from_canonical_u32(event.next_pc);

        cols.hi_record_is_real = F::from_bool(event.hi_record_is_real);
        if event.hi_record_is_real {
            // For madd[u]/msub[u] instructions, pass in a dummy byte lookup vector.  This madd[u]/msub[u]
            // instruction chip also has a op_hi_access field that will be populated and that will contribute
            // to the byte lookup dependencies.
            cols.op_hi_access.populate(MemoryRecordEnum::Write(event.hi_record), blu);
            cols.shard = F::from_canonical_u32(event.shard);
            cols.clk = F::from_canonical_u32(event.clk);
        }

        let hi_word = event.hi.to_le_bytes();
        let a_word = event.a.to_le_bytes();
        let b_word = event.b.to_le_bytes();
        let c_word = event.c.to_le_bytes();

        let mut b = b_word.to_vec();
        let mut c = c_word.to_vec();

        // Handle b and c's signs.
        {
            let b_msb = get_msb(b_word);
            cols.b_msb = F::from_canonical_u8(b_msb);
            let c_msb = get_msb(c_word);
            cols.c_msb = F::from_canonical_u8(c_msb);

            // If b is signed and it is negative, sign extend b.
            if event.opcode == Opcode::MULT && b_msb == 1 {
                cols.b_sign_extend = F::ONE;
                b.resize(PRODUCT_SIZE, BYTE_MASK);
            }

            // If c is signed and it is negative, sign extend c.
            if event.opcode == Opcode::MULT && c_msb == 1 {
                cols.c_sign_extend = F::ONE;
                c.resize(PRODUCT_SIZE, BYTE_MASK);
            }

            // Insert the MSB lookup events.
            {
                let words = [b_word, c_word];
                let mut blu_events: Vec<ByteLookupEvent> = vec![];
                for word in words.iter() {
                    let most_significant_byte = word[WORD_SIZE - 1];
                    blu_events.push(ByteLookupEvent {
                        opcode: ByteOpcode::MSB,
                        a1: get_msb(*word) as u16,
                        a2: 0,
                        b: most_significant_byte,
                        c: 0,
                    });
                }
                blu.add_byte_lookup_events(blu_events);
            }
        }

        let mut product = [0u32; PRODUCT_SIZE];
        for i in 0..b.len() {
            for j in 0..c.len() {
                if i + j < PRODUCT_SIZE {
                    product[i + j] += (b[i] as u32) * (c[j] as u32);
                }
            }
        }

        // Calculate the correct product using the `product` array. We store the
        // correct carry value for verification.
        let base = (1 << BYTE_SIZE) as u32;
        let mut carry = [0u32; PRODUCT_SIZE];
        for i in 0..PRODUCT_SIZE {
            carry[i] = product[i] / base;
            product[i] %= base;
            if i + 1 < PRODUCT_SIZE {
                product[i + 1] += carry[i];
            }
            cols.carry[i] = F::from_canonical_u32(carry[i]);
        }

        cols.product = product.map(F::from_canonical_u32);
        cols.hi = Word(hi_word.map(F::from_canonical_u8));
        cols.a = Word(a_word.map(F::from_canonical_u8));
        cols.b = Word(b_word.map(F::from_canonical_u8));
        cols.c = Word(c_word.map(F::from_canonical_u8));
        cols.is_real = F::ONE;
        cols.is_mul = F::from_bool(event.opcode == Opcode::MUL);
        cols.is_mult = F::from_bool(event.opcode == Opcode::MULT);
        cols.is_multu = F::from_bool(event.opcode == Opcode::MULTU);

        // Range check.
        {
            blu.add_u16_range_checks(&carry.map(|x| x as u16));
            blu.add_u8_range_checks(&product.map(|x| x as u8));
        }
    }
}

impl<F> BaseAir<F> for MulChip {
    fn width(&self) -> usize {
        NUM_MUL_COLS
    }
}

impl<AB> Air<AB> for MulChip
where
    AB: ZKMCoreAirBuilder,
{
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let local = main.row_slice(0);
        let local: &MulCols<AB::Var> = (*local).borrow();
        let base = AB::F::from_canonical_u32(1 << 8);

        let zero: AB::Expr = AB::F::ZERO.into();
        let one: AB::Expr = AB::F::ONE.into();
        let byte_mask = AB::F::from_canonical_u8(BYTE_MASK);

        // Calculate the MSBs.
        let (b_msb, c_msb) = {
            let msb_pairs =
                [(local.b_msb, local.b[WORD_SIZE - 1]), (local.c_msb, local.c[WORD_SIZE - 1])];
            let opcode = AB::F::from_canonical_u32(ByteOpcode::MSB as u32);
            for msb_pair in msb_pairs.iter() {
                let msb = msb_pair.0;
                let byte = msb_pair.1;
                builder.send_byte(opcode, msb, byte, zero.clone(), local.is_real);
            }
            (local.b_msb, local.c_msb)
        };

        // Calculate whether to extend b and c's sign.
        let (b_sign_extend, c_sign_extend) = {
            let is_b_i32 = local.is_mult;
            let is_c_i32 = local.is_mult;

            builder.assert_eq(local.b_sign_extend, is_b_i32 * b_msb);
            builder.assert_eq(local.c_sign_extend, is_c_i32 * c_msb);
            (local.b_sign_extend, local.c_sign_extend)
        };

        // Sign extend local.b and local.c whenever appropriate.
        let (b, c) = {
            let mut b: Vec<AB::Expr> = vec![AB::F::ZERO.into(); PRODUCT_SIZE];
            let mut c: Vec<AB::Expr> = vec![AB::F::ZERO.into(); PRODUCT_SIZE];
            for i in 0..PRODUCT_SIZE {
                if i < WORD_SIZE {
                    b[i] = local.b[i].into();
                    c[i] = local.c[i].into();
                } else {
                    b[i] = b_sign_extend * byte_mask;
                    c[i] = c_sign_extend * byte_mask;
                }
            }
            (b, c)
        };

        // Compute the uncarried product b(x) * c(x) = m(x).
        let mut m: Vec<AB::Expr> = vec![AB::F::ZERO.into(); PRODUCT_SIZE];
        for i in 0..PRODUCT_SIZE {
            for j in 0..PRODUCT_SIZE {
                if i + j < PRODUCT_SIZE {
                    m[i + j] = m[i + j].clone() + b[i].clone() * c[j].clone();
                }
            }
        }

        // Propagate carry.
        let product = {
            for i in 0..PRODUCT_SIZE {
                if i == 0 {
                    builder.assert_eq(local.product[i], m[i].clone() - local.carry[i] * base);
                } else {
                    builder.assert_eq(
                        local.product[i],
                        m[i].clone() + local.carry[i - 1] - local.carry[i] * base,
                    );
                }
            }
            local.product
        };

        // Compare the product's appropriate bytes with that of the result.
        {
            let has_hi = local.is_mult + local.is_multu;
            for i in 0..WORD_SIZE {
                builder.assert_eq(product[i], local.a[i]);
                builder.when(has_hi.clone()).assert_eq(product[i + WORD_SIZE], local.hi[i]);
            }
        }

        // Check that the boolean values are indeed boolean values.
        {
            let booleans = [
                local.b_msb,
                local.c_msb,
                local.b_sign_extend,
                local.c_sign_extend,
                local.is_mul,
                local.is_mult,
                local.is_multu,
                local.is_real,
                local.hi_record_is_real,
            ];
            for boolean in booleans.iter() {
                builder.assert_bool(*boolean);
            }
        }

        // If signed extended, the MSB better be 1.
        builder.when(local.b_sign_extend).assert_eq(local.b_msb, one.clone());
        builder.when(local.c_sign_extend).assert_eq(local.c_msb, one.clone());

        // Calculate the opcode.
        let opcode = {
            // Exactly one of the op codes must be on.
            builder.when(local.is_real).assert_one(local.is_mul + local.is_mult + local.is_multu);

            let mul: AB::Expr = AB::F::from_canonical_u32(Opcode::MUL as u32).into();
            let mult: AB::Expr = AB::F::from_canonical_u32(Opcode::MULT as u32).into();
            let multu: AB::Expr = AB::F::from_canonical_u32(Opcode::MULTU as u32).into();
            local.is_mul * mul + local.is_mult * mult + local.is_multu * multu
        };

        // Range check.
        {
            // Ensure that the carry is at most 2^16. This ensures that
            // product_before_carry_propagation - carry * base + last_carry never overflows or
            // underflows enough to "wrap" around to create a second solution.
            builder.slice_range_check_u16(&local.carry, local.is_real);

            builder.slice_range_check_u8(&local.product, local.is_real);
        }

        // Receive the arguments.
        builder.receive_instruction(
            local.shard,
            local.clk,
            local.pc,
            local.next_pc,
            local.next_pc + AB::Expr::from_canonical_u32(4),
            AB::Expr::zero(),
            opcode,
            local.a,
            local.b,
            local.c,
            local.hi,
            AB::Expr::zero(),
            AB::Expr::zero(),
            local.hi_record_is_real,
            AB::Expr::zero(),
            AB::Expr::one(),
            local.is_real,
        );

        // Write the HI register, the register can only be Register::HI（33）.
        builder.eval_memory_access(
            local.shard,
            local.clk + AB::F::from_canonical_u32(MemoryAccessPosition::HI as u32),
            AB::F::from_canonical_u32(33),
            &local.op_hi_access,
            local.hi_record_is_real,
        );

        // Check hi_record_is_real.
        // hi_record_is_real can only be set for MULT and MULTU instruction.
        // if hi_record_is_real = 0, both clk and shard should be zero.
        builder.when(local.hi_record_is_real).assert_one(local.is_mult + local.is_multu);
        builder.when(local.hi_record_is_real).assert_word_eq(local.hi, *local.op_hi_access.value());
        builder.when_not(local.hi_record_is_real).assert_zero(local.clk);
        builder.when_not(local.hi_record_is_real).assert_zero(local.shard);
    }
}

#[cfg(test)]
mod tests {

    use crate::utils::{uni_stark_prove as prove, uni_stark_verify as verify};
    use p3_koala_bear::KoalaBear;
    use p3_matrix::dense::RowMajorMatrix;
    use zkm_core_executor::{events::CompAluEvent, ExecutionRecord, Opcode};
    use zkm_stark::{
        air::MachineAir, koala_bear_poseidon2::KoalaBearPoseidon2, StarkGenericConfig,
    };

    use super::MulChip;

    #[test]
    fn generate_trace_mul() {
        let mut shard = ExecutionRecord::default();

        // Fill mul_events with 10 MUL events.
        let mut mul_events: Vec<CompAluEvent> = Vec::new();
        for _ in 0..10 {
            mul_events.push(CompAluEvent::new(0, Opcode::MUL, 0x80004000, 0x80000000, 0xffff8000));
        }
        shard.mul_events = mul_events;
        let chip = MulChip::default();
        let _trace: RowMajorMatrix<KoalaBear> =
            chip.generate_trace(&shard, &mut ExecutionRecord::default());
    }

    #[test]
    fn prove_koalabear() {
        let config = KoalaBearPoseidon2::new();
        let mut challenger = config.challenger();

        let mut shard = ExecutionRecord::default();
        let mut mul_events: Vec<CompAluEvent> = Vec::new();

        let mul_instructions: Vec<(Opcode, u32, u32, u32)> = vec![
            (Opcode::MUL, 0x00001200, 0x00007e00, 0xb6db6db7),
            (Opcode::MUL, 0x00001240, 0x00007fc0, 0xb6db6db7),
            (Opcode::MUL, 0x00000000, 0x00000000, 0x00000000),
            (Opcode::MUL, 0x00000001, 0x00000001, 0x00000001),
            (Opcode::MUL, 0x00000015, 0x00000003, 0x00000007),
            (Opcode::MUL, 0x00000000, 0x00000000, 0xffff8000),
            (Opcode::MUL, 0x00000000, 0x80000000, 0x00000000),
            (Opcode::MUL, 0x00000000, 0x80000000, 0xffff8000),
            (Opcode::MUL, 0x0000ff7f, 0xaaaaaaab, 0x0002fe7d),
            (Opcode::MUL, 0x0000ff7f, 0x0002fe7d, 0xaaaaaaab),
            (Opcode::MUL, 0x00000000, 0xff000000, 0xff000000),
            (Opcode::MUL, 0x00000001, 0xffffffff, 0xffffffff),
            (Opcode::MUL, 0xffffffff, 0xffffffff, 0x00000001),
            (Opcode::MUL, 0xffffffff, 0x00000001, 0xffffffff),
        ];
        for t in mul_instructions.iter() {
            mul_events.push(CompAluEvent::new(0, t.0, t.1, t.2, t.3));
        }

        // Append more events until we have 1000 tests.
        for _ in 0..(1000 - mul_instructions.len()) {
            mul_events.push(CompAluEvent::new(0, Opcode::MUL, 1, 1, 1));
        }

        shard.mul_events = mul_events;
        let chip = MulChip::default();
        let trace: RowMajorMatrix<KoalaBear> =
            chip.generate_trace(&shard, &mut ExecutionRecord::default());
        let proof = prove::<KoalaBearPoseidon2, _>(&config, &chip, &mut challenger, trace);

        let mut challenger = config.challenger();
        verify(&config, &chip, &mut challenger, &proof).unwrap();
    }
}
