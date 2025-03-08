# Prover

## ELF Loader

```rust
zkm_emulator::utils::load_elf_with_patch
```

## MIPS VM

```rust
zkm_emulator::utils::split_prog_into_segs
```

## Proving

```rust
zkm_utils::utils::prove_segments
```

## Example
```rust
use std::env;

use zkm_emulator::utils::{load_elf_with_patch, split_prog_into_segs};
use zkm_utils::utils::prove_segments;

const ELF_PATH: &str = "../guest/elf/mips-zkm-zkvm-elf";

/// This function sets up the state, loads the ELF file, adds inputs, splits the program into segments,
/// and then generates the zero-knowledge proof for the SHA-2 hash computation.
fn prove_sha2_rust() {
    // 1. Retrieve the segment output path from the environment variable.
    let seg_path = env::var("SEG_OUTPUT")
        .expect("Segment output path is missing");
    
    // 2. Retrieve segment size from the environment variable or use default (65536).
    let seg_size = env::var("SEG_SIZE").unwrap_or("65536".to_string());
    let seg_size = seg_size.parse::<_>().unwrap_or(0);

    // 3. Load the ELF file (with an empty patch vector) to initialize the program state.
    let mut state = load_elf_with_patch(ELF_PATH, vec![]);
    
    // 4. Load inputs from the environment variable "ARGS".
    // Expected format: "<public_hash_output> <data_to_hash>"
    let args = env::var("ARGS").unwrap_or("data-to-hash".to_string());
    // Split the arguments by whitespace.
    let args: Vec<&str> = args.split_whitespace().collect();
    // Ensure exactly 2 arguments are provided.
    assert_eq!(args.len(), 2);

    // 5. Process the first argument as the expected public hash output (in hexadecimal).
    let public_input: Vec<u8> = hex::decode(args[0]).unwrap();
    // Add the public input to the state as a public input stream.
    state.add_input_stream(&public_input);
    log::info!("expected public value in hex: {:X?}", args[0]);
    log::info!("expected public value: {:X?}", public_input);

    // 6. Process the second argument as the private input (data to be hashed).
    let private_input = args[1].as_bytes().to_vec();
    log::info!("private input value: {:X?}", private_input);
    // Add the private input to the state.
    state.add_input_stream(&private_input);

    // 7. Split the program into segments using the specified segment path and size.
    let (_total_steps, seg_num, mut state) = split_prog_into_segs(state, &seg_path, "", seg_size);

    // 8. Read the public output value (the computed hash) from the state.
    let value = state.read_public_values::<[u8; 32]>();
    log::info!("public value: {:X?}", value);
    log::info!("public value: {} in hex", hex::encode(value));

    // 9. Generate the zero-knowledge proof based on the program segments.
    // The parameters include segment path, empty configuration strings, segment number, and an offset of 0.
    let _ = prove_segments(&seg_path, "", "", "", seg_num, 0, vec![]).unwrap();
}

fn main() {
    // Initialize the logger for logging info and errors.
    env_logger::try_init().unwrap_or_default();
    // Execute the SHA-2 proof generation function.
    prove_sha2_rust();
}

}
```
