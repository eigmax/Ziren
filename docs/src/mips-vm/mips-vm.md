# MIPS VM
zkMIPS is a verifiable computation infrastructure based on the MIPS32, specifically designed to provide zero-knowledge proof generation for programs written in Rust and Golang. This enhances project auditing and the efficiency of security verification. Focusing on the extensive design experience of MIPS, zkMIPS adopts the MIPS32r2 instruction set as it offers significant hardware advantages. Compared to the RISC-V architecture, MIPS32r2 excels in several key features. Its J/JAL instructions support jump ranges of up to 256MiB, offering greater flexibility for large-scale data processing and complex control flow scenarios. Moreover, the rich set of bit manipulation instructions and additional conditional move instructions (such as MOVZ and MOVN) ensure precise data handling, while the inclusion of the MADDU instruction further improves arithmetic computation efficiency. Overall, by integrating mature MIPS design with zero-knowledge proof, zkMIPS provides an efficient and stable foundational platform for secure computing and decentralized applications.

**Execution Flow of zkMIPS**  
In the execution process of zkMIPS, a Rust program written by the developer is first transformed by a dedicated compiler into the MIPS instruction set, generating a corresponding ELF binary file. This process accurately maps the high-level logic of the program to low-level instructions, laying a solid foundation for subsequent verification. Next, the system employs a specially designed executor to simulate the execution of the ELF file, recording all state changes and computational steps to form a complete execution trace. This trace serves as the core data for generating the zero-knowledge proof, ensuring that the proof accurately reflects the real execution of the compiled program. Subsequently, the ZKM prover efficiently processes and compresses the generated trace to produce a zero-knowledge proof. This proof not only validates the correctness of the program execution but also significantly reduces the data volume, facilitating rapid verification and storage. Finally, the generated proof is jointly verified by multiple participants and ultimately recorded on-chain, ensuring that the entire computational process is transparent, secure, and trustworthy. With the nice property of zkSNARK, the proof is short and fast to verify.

**Circuit Module of zkMIPS**  
In terms of circuit design, zkMIPS adopts a highly modular strategy, breaking the entire MIPS program into multiple submodules with clearly defined functions to meticulously manage and verify the execution state of each part. The system begins with memory consistency checks and then divides the program into several sections: kernel, memory, CPU, arithmetic logic unit (ALU), hashing, and byte processing. 

***The kernel module*** is primarily responsible for managing the program’s entry and exit, ensuring the stability and consistency of the system state. 

***The memory module*** clearly distinguishes between global and local memory, establishing defined boundaries for data access and state transfer. 

***The CPU module*** handles core tasks such as branch and jump operations, memory I/O, register manipulation, and system calls, ensuring the precise execution of program logic. 

***The ALU module*** focuses on arithmetic and logical computations, supporting various specific operations including instructions like CLO/Z. 

Additionally, ***dedicated hashing and byte processing modules*** are employed to manage data validation and processing, while ***the bus module***, through cross-table lookup functionality, facilitates efficient communication and data exchange between the submodules. 

Through this detailed division of circuit modules, zkMIPS can accurately capture the behavior and state of each submodule during the generation of the zero-knowledge proof, to improve the overall security of the system.
