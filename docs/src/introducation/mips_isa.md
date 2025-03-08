# MIPS ISA
The `Opcode` enum organizes MIPS instructions into several functional categories, each serving a specific role in the instruction set:
```rust
pub enum Opcode {
    // BinaryOperator
    ADD = 0,
    SUB = 1,
    MULT = 2,
    MULTU = 3,
    MUL = 4,
    DIV = 5,
    DIVU = 6,
    SLL = 7,
    SRL = 8,
    SRA = 9,
    SLT = 10,
    SLTU = 11,
    AND = 12,
    OR = 13,
    XOR = 14,
    NOR = 15,
    // count leading zeros
    CLZ = 16,
    // count leading ones
    CLO = 17,
    BEQ = 18,
    BGEZ = 19,
    BGTZ = 20,
    BLEZ = 21,
    BLTZ = 22,
    BNE = 23,
    // MovCond
    MEQ = 24,
    MNE = 25,
    // Memory Op
    LH = 26,
    LWL = 27,
    LW = 28,
    LB = 29,
    LBU = 30,
    LHU = 31,
    LWR = 32,
    LL = 33,
    SB = 34,
    SH = 35,
    SWL = 36,
    SW = 37,
    SWR = 38,
    SC = 39,
    Jump = 40,
    Jumpi = 41,
    JumpDirect = 42,
    NOP = 43,
    SYSCALL = 44,
    TEQ = 45,
    UNIMPL = 0xff,
}
```

All MIPS instructions can be divided into the following taxonomies:

**Binary Operators**  
This category includes the fundamental arithmetic and logical operations. It covers addition (ADD) and subtraction (SUB), several multiplication and division variants (MULT, MULTU, MUL, DIV, DIVU), as well as bit shifting operations (SLL, SRL, SRA), comparison operations like set less than (SLT, SLTU) and a range of bitwise logical operations (AND, OR, XOR, NOR). 

**Count Operations**  
Within this group, there are specialized instructions that analyze bit patterns: CLZ counts the number of leading zeros, while CLO counts the number of leading ones. These operations are useful in bit-level data analysis.

**Branching Instructions**  
Instructions BEQ (branch if equal), BGEZ (branch if greater than or equal to zero), BGTZ (branch if greater than zero), BLEZ (branch if less than or equal to zero), BLTZ (branch if less than zero), and BNE (branch if not equal) are used to change the flow of execution based on comparisons. These instructions are vital for implementing loops, conditionals, and other control structures.

**Conditional Move Instructions (MovCond)**  
This type of instruction includes MEQ (move if equal) and MNE (move if not equal) in this category. These instructions perform data transfers between registers based on the result of a comparison, allowing the program to conditionally update values without resorting to full branch instructions. This can lead to more efficient execution in frequent conditional operations.

**Memory Operations**  
This category is dedicated to moving data between memory and registers. It contains a comprehensive set of load instructions—such as LH (load halfword), LWL (load word left), LW (load word), LB (load byte), LBU (load byte unsigned), LHU (load halfword unsigned), LWR (load word right), and LL (load linked)—as well as corresponding store instructions like SB (store byte), SH (store halfword), SWL (store word left), SW (store word), SWR (store word right), and SC (store conditional). These operations ensure that data is correctly and efficiently read from or written to memory.

**Jump Instructions**  
Jump-related instructions, including Jump, Jumpi, and JumpDirect, are responsible for altering the execution flow by redirecting it to different parts of the program. They are used for implementing function calls, loops, and other control structures that require non-sequential execution, ensuring that the program can navigate its code dynamically.

**Special Instructions**  
This category includes instructions with unique roles. NOP (no operation) is used when no action is required—often for timing adjustments or to fill delay slots. SYSCALL triggers a system call, allowing the program to request services from the operating system. TEQ is typically used to test equality conditions between registers. UNIMPL represents an unimplemented or reserved opcode, serving as a placeholder for potential future instructions or as an indicator for unsupported operations.
