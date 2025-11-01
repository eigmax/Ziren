pub mod register;

use core::borrow::Borrow;
use p3_air::{Air, AirBuilder, AirBuilderWithPublicValues, BaseAir};
use p3_field::FieldAlgebra;
use p3_matrix::Matrix;
use zkm_core_executor::ByteOpcode;
use zkm_stark::{
    air::{BaseAirBuilder, PublicValues, ZKMAirBuilder, ZKM_PROOF_NUM_PV_ELTS},
    Word,
};

use crate::{
    air::{MemoryAirBuilder, ZKMCoreAirBuilder},
    cpu::{
        columns::{CpuCols, NUM_CPU_COLS},
        CpuChip,
    },
};

impl<AB> Air<AB> for CpuChip
where
    AB: ZKMCoreAirBuilder + AirBuilderWithPublicValues,
    AB::Var: Sized,
{
    #[inline(never)]
    fn eval(&self, builder: &mut AB) {
        let main = builder.main();
        let (local, next) = (main.row_slice(0), main.row_slice(1));
        let local: &CpuCols<AB::Var> = (*local).borrow();
        let next: &CpuCols<AB::Var> = (*next).borrow();

        let public_values_slice: [AB::PublicVar; ZKM_PROOF_NUM_PV_ELTS] =
            core::array::from_fn(|i| builder.public_values()[i]);
        let public_values: &PublicValues<Word<AB::PublicVar>, AB::PublicVar> =
            public_values_slice.as_slice().borrow();

        let clk =
            AB::Expr::from_canonical_u32(1u32 << 16) * local.clk_8bit_limb + local.clk_16bit_limb;

        // Program constraints.
        builder.send_program(local.pc, local.instruction, local.is_real);

        // Register constraints.
        self.eval_registers::<AB>(builder, local, clk.clone());

        // Assert the shard and clk to send.  Only the memory and syscall instructions need the
        // actual shard and clk values for memory access evals.
        // SAFETY: The usage of `builder.if_else` requires `is_memory + is_syscall` to be boolean.
        // The correctness of `is_memory` and `is_syscall` will be checked in the opcode specific chips.
        // In these correct cases, `is_memory + is_syscall` will be always boolean.
        let expected_shard_to_send =
            builder.if_else(local.is_rw_a + local.is_write_hi, local.shard, AB::Expr::zero());
        let expected_clk_to_send =
            builder.if_else(local.is_rw_a + local.is_write_hi, clk.clone(), AB::Expr::zero());
        builder.when(local.is_real).assert_eq(local.shard_to_send, expected_shard_to_send);
        builder.when(local.is_real).assert_eq(local.clk_to_send, expected_clk_to_send);

        builder.send_instruction(
            local.shard_to_send,
            local.clk_to_send,
            local.pc,
            local.next_pc,
            local.next_next_pc,
            local.num_extra_cycles,
            local.instruction.opcode,
            local.op_a_value,
            local.op_b_val(),
            local.op_c_val(),
            local.hi_or_prev_a,
            local.op_a_immutable,
            local.is_rw_a,
            local.is_write_hi,
            local.is_halt,
            local.is_sequential,
            local.is_real,
        );

        // Check that the shard and clk is updated correctly.
        self.eval_shard_clk(builder, local, next, clk.clone());

        // Check public values constraints.
        self.eval_pc(builder, local, next, public_values);

        // Check that the is_real flag is correct.
        self.eval_is_real(builder, local, next);

        let not_real = AB::Expr::one() - local.is_real;
        builder.when(not_real.clone()).assert_zero(AB::Expr::one() - local.instruction.imm_b);
        builder.when(not_real.clone()).assert_zero(AB::Expr::one() - local.instruction.imm_c);
        builder.when(not_real.clone()).assert_zero(AB::Expr::one() - local.is_rw_a);
    }
}

impl CpuChip {
    /// Constraints related to the shard and clk.
    ///
    /// This method ensures that all of the shard values are the same and that the clk starts at 0
    /// and is transitioned appropriately.  It will also check that shard values are within 16 bits
    /// and clk values are within 24 bits.  Those range checks are needed for the memory access
    /// timestamp check, which assumes those values are within 2^24.  See
    /// [`MemoryAirBuilder::verify_mem_access_ts`].
    pub(crate) fn eval_shard_clk<AB: ZKMAirBuilder>(
        &self,
        builder: &mut AB,
        local: &CpuCols<AB::Var>,
        next: &CpuCols<AB::Var>,
        clk: AB::Expr,
    ) {
        // Verify that all shard values are the same.
        builder.when_transition().when(next.is_real).assert_eq(local.shard, next.shard);

        // Verify that the shard value is within 16 bits.
        builder.send_byte(
            AB::Expr::from_canonical_u8(ByteOpcode::U16Range as u8),
            local.shard,
            AB::Expr::zero(),
            AB::Expr::zero(),
            local.is_real,
        );

        // Verify that the first row has a clk value of 0.
        builder.when_first_row().assert_zero(clk.clone());

        // We already assert that `local.clk < 2^24`. `num_extra_cycles` is an entry of a word and
        // therefore less than `2^8`, this means that the sum cannot overflow in a 31 bit field.
        let expected_next_clk =
            clk.clone() + AB::Expr::from_canonical_u32(5) + local.num_extra_cycles;

        let next_clk =
            AB::Expr::from_canonical_u32(1u32 << 16) * next.clk_8bit_limb + next.clk_16bit_limb;
        builder.when_transition().when(next.is_real).assert_eq(expected_next_clk, next_clk);

        // Range check that the clk is within 24 bits using it's limb values.
        builder.eval_range_check_24bits(
            clk,
            local.clk_16bit_limb,
            local.clk_8bit_limb,
            local.is_real,
        );
    }

    /// Constraints related to the public values.
    pub(crate) fn eval_pc<AB: ZKMAirBuilder>(
        &self,
        builder: &mut AB,
        local: &CpuCols<AB::Var>,
        next: &CpuCols<AB::Var>,
        public_values: &PublicValues<Word<AB::PublicVar>, AB::PublicVar>,
    ) {
        // Verify the public value's shard.
        builder.when(local.is_real).assert_eq(public_values.execution_shard, local.shard);

        // Verify the public value's start pc.
        builder.when_first_row().assert_eq(public_values.start_pc, local.pc);

        // Verify the relationship between initial start pc and initial next pc.
        builder
            .when_first_row()
            .when_not(local.is_halt)
            .assert_eq(local.pc + AB::Expr::from_canonical_u32(4), local.next_pc);

        // Verify the pc, next_pc, and next_next_pc
        builder.when_transition().when(next.is_real).assert_eq(local.next_pc, next.pc);
        builder
            .when_transition()
            .when(next.is_real)
            .when_not(next.is_halt)
            .assert_eq(local.next_next_pc, next.next_pc);

        builder
            .when_transition()
            .when(local.is_real)
            .when(local.is_sequential)
            .assert_eq(local.next_next_pc, local.next_pc + AB::Expr::from_canonical_u32(4));

        // Verify the public value's next pc.  We need to handle two cases:
        // 1. The last real row is a transition row.
        // 2. The last real row is the last row.

        // If the last real row is a transition row, verify the public value's next pc.
        builder
            .when_transition()
            .when(local.is_real - next.is_real)
            .assert_eq(public_values.next_pc, local.next_pc);

        // If the last real row is the last row, verify the public value's next pc.
        builder.when_last_row().when(local.is_real).assert_eq(public_values.next_pc, local.next_pc);
    }

    /// Constraints related to the is_real column.
    ///
    /// This method checks that the is_real column is a boolean.  It also checks that the first row
    /// is 1 and once its 0, it never changes value.
    pub(crate) fn eval_is_real<AB: ZKMAirBuilder>(
        &self,
        builder: &mut AB,
        local: &CpuCols<AB::Var>,
        next: &CpuCols<AB::Var>,
    ) {
        // Check the is_real flag.  It should be 1 for the first row.  Once its 0, it should never
        // change value.
        builder.assert_bool(local.is_real);
        builder.when_first_row().assert_one(local.is_real);
        builder.when_transition().when_not(local.is_real).assert_zero(next.is_real);
        // If we're halting and it's a transition, then the next.is_real should be 0.
        builder.when_transition().when(local.is_halt).assert_zero(next.is_real);
    }
}

impl<F> BaseAir<F> for CpuChip {
    fn width(&self) -> usize {
        NUM_CPU_COLS
    }
}
