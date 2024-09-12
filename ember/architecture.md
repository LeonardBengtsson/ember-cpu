# Registers
| Name                       | Bit size |
|----------------------------|----------|
| General-purpose register A | 16       |
| General-purpose register B | 16       |
| General-purpose register C | 16       |
| Instruction register       | 16       |
| Instruction counter        | 16       |
| Stack counter              | 16       |
| Error code                 | 8        |
| Loop flag                  | 1        |
| Jumped flag                | 1        |
| Load const flag            | 1        |

# Flags
| Name       | Description                                                                      |
|------------|----------------------------------------------------------------------------------|
| Looping    | True by default, false if the cpu has been halted or paused                      |
| Jumped     | Set after jumping to prevent skipping the next instruction                       |
| Load const | If set, treats the next instruction as a binary value instead of an instruction. |
| Zero       | Whether the result of the last ALU instruction was 0                             |
| Negative   | Whether the result of the last ALU instruction underflowed                       |
| Overflow   | Whether the result of the last ALU instruction overflowed                        |

# Memory layout
65536 16-bit addresses, in groups of 16:

| Section              | Size | Address start | Adress end |
|----------------------|------|---------------|------------|
| VRAM                 | 1024 | 0x0000        | 0x3fff     |
| Program              | 512  | 0x4000        | 0x5fff     |
| Stack                | 256  | 0x6000        | 0x6fff     |
| Built-in subroutines | 128  | 0x7000        | 0x77ff     |
| Heap head            | 128  | 0x7800        | 0x7fff     |
| Heap body            | 2048 | 0x8000        | 0xffff     |
