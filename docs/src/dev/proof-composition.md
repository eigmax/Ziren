# Proof Composition

## What is a receipt?

A receipt gives the results of your program along with proof that they were produced honestly.

## What is Proof Composition

You can verify other receipts in the guest use ```zkm_runtime::io::verify```

## Example

```rust
#![no_std]
#![no_main]

// Import the necessary crates for SHA-256 hashing.
use sha2::{Digest, Sha256};

// Enable heap allocation by using the `alloc` crate.
extern crate alloc;
use alloc::vec::Vec;

// Define the program entry point using the zkm_runtime macro.
zkm_runtime::entrypoint!(main);

// The main function serves as the entry point for the program.
pub fn main() {
    // Read the public input from the runtime.
    // This is expected to be the correct SHA-256 hash (as a vector of bytes) that will be compared later.
    let public_input: Vec<u8> = zkm_runtime::io::read();
    
    // Read the private input data which will be hashed.
    let input: Vec<u8> = zkm_runtime::io::read();
    
    // Read the ELF identifier, which may be used to verify that the correct ELF binary is executing.
    let elf_id: Vec<u8> = zkm_runtime::io::read();

    // Verify the ELF binary using the provided elf_id and the input data.
    // This step likely ensures that the correct program or expected state is being executed.
    zkm_runtime::io::verify(elf_id, &input);
    
    // Create a new SHA-256 hasher instance.
    let mut hasher = Sha256::new();
    
    // Update the hasher with the private input data.
    hasher.update(input);
    
    // Finalize the hashing process and retrieve the resulting hash.
    let result = hasher.finalize();

    // Convert the resulting hash into a fixed-size array of 32 bytes.
    let output: [u8; 32] = result.into();
    
    // Assert that the computed hash matches the public input.
    // This check verifies that the private input, when hashed, produces the expected public hash value.
    assert_eq!(output.to_vec(), public_input);

    // Commit the output hash to the runtime.
    // This typically means that the computed proof (in this case, the hash) is recorded for further verification or on-chain storage.
    zkm_runtime::io::commit::<[u8; 32]>(&output);
}
}
```
