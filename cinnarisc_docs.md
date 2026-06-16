# Assembler Documentation
    This is a work in progress. v0.1

## Directives

### `.section`
defines either the `.text` or `.data` section
**`.data`** defines global symbols, is loaded into memory with the program
**`.text`** defines the text section where all the code is, this is what is assembled into machine code.

Usage: `.section [section]`

### `.org`
This current version uses `0x1000` as it's reset vector.
Where the program would be loaded in, the assembler uses this to calculate offsets.
Usage: `.org [start]`

<hr>

## Data, Symbols, and You.

### Primitive Types

||size|stored?|
|-|-|-|
|byte|1|✔|
|half|2|✔
|word|4|✔
|string|any|✔
|const|none|✖|

`byte, half, word` numbers that are stored at different sizes
`string` a string that *only* has the inner bytes (ie. no length indicator)
`const` a symbol that isnt stored in the program but can be used as a constant

### Example data section
```
.section .data
    byte: byte = 255
    half: half = 65535
    word: word = -1
    string: string = "Hello, World!'
    string_length: const = 13
```

### References & Symbols
Say you wanted to actually use a label or data, how would you do that?
1. Only constants hold a value, eg. `addi t0, zero, string_length` would set `t0` to `13`, however `addi t0, zero, byte` would set `t0` to the address of the byte, *not* `255`
2. labels can either be absolute or relative, signified by `*`, so `li t0, *label` would set `t0` to be the absolute address of the label, while `li t0, label` would set `t0` to `*label - pc` or the jump offset.

Therefore,
```
.section .data
    value: byte = 17

.section .text
...
addi t0, zero, byte     ✖

li t0, zero, byte       
lb t0, t0, 0            ✔
```
```
...
label:
    ...
    li, t0, *label      <- t0 is the absolute address of the label, eg. 0x2000
    jalr zero, t0, 0    <- we then jump to the absolute address (t0)
    # this lets you jump anywhere within 32 bit space (~4GiB)
...
label2:
    jal zero, label2    <- shorter range (+-1MiB), label2 is an offset
```

<hr>

## Instructions
    note: usage is how it's written in code.

### RV32I

`lui` "`Load Upper Immediate`" **overwrites** `rd` with the `imm << 12`
Usage: `lui, imm[20]`

`auipc` "`Add Upper Immediate to PC`" performs `lui` on the `pc` into `rd` (ie. `rd = pc << 12`)
Usage: `auipc rd, imm[20]`

#### Load

`lb` "`Load Byte`" loads a byte from memory at address `rs1 + imm` into `rd`
Usage: `lb rd, rs1, imm[12]`

`lh` "`Load Half`" loads a half from memory at address `rs1 + imm` into `rd`
Usage: `lh rd, rs1, imm[12]`

`lw` "`Load Word`" loads a word from memory at address `rs1 + imm` into `rd`
Usage: `lw rd, rs1, imm[12]`

~~`lbu` "`Load Byte Unsigned`" loads a byte from memory at address `rs1 + imm` into `rd`
Usage: `lb rd, rs1, imm[12]`~~

~~`lhu` "`Load Half Unsigned`" loads a half from memory at address `rs1 + imm` into `rd`
Usage: `lh rd, rs1, imm[12]`~~

    load: loads as little endian 
        stored: [lowest][lower][higher][highest] (eg. "Hiya")
        rd = "ayiH"

#### Store

`sb` "`Store Byte`" stores lowest 8 bits of `rs1` into memory address `rs2 + imm`
Usage: `sb rs1, rs2, imm[12]`

`sh` "`Store Half`" stores lowest 16 bits of `rs1` into memory address `rs2 + imm`
Usage: `sh rs1, rs2, imm[12]`

`sw` "`Store Word`" stores `rs1` into memory address `rs2 + imm`
Usage: `sw rs1, rs2, imm[12]`

    store: stores as little endian 
        rs1 = "Hiya"
        memory = "ayiH"

#### Immediate

`addi` "`ADD Immediate`" adds `rs1 + imm` into `rd`
Usage: `addi rd, rs1, imm[12]`

("addi", "slti", "sltiu", "xori", "ori", "andi", "srli", "slli", "srai"):
`slti` "`Set Less Than Immediate`" sets `rd` to `1` if `rs1 < imm` otherwise `rd = 0`
Usage: `slti rd, rs1, imm[12]`

`sltiu` "`Set Less Than Immediate Unsigned`" sets `rd` to `1` if `rs1 < imm` otherwise `rd = 0`, unlike `slti` however, all values are treated as positive
Usage: `sltiu rd, rs1, imm[12]`

`xori` "`XOR Immediate`" sets `rd` to `rs1 ^ imm`
Usage: `xori rd, rs1, imm[12]`
Example: `xori a0, (7), 3 -> 0b111 ^ 0b11 -> 0b100 (a0 = 4)`

`ori` "`OR Immediate`" sets `rd` to `rs1 | imm`
Usage: `ori rd, rs1, imm[12]`
Example: `ori a0, (4), 3 -> 0b100 | 0b11 -> 0b111 (a0 = 7)`

`andi` "`AND Immediate`" sets `rd` to `rs1 & imm`
Usage: `andi rd, rs1, imm[12]`
Example: `andi a0, (9), 3 -> 0b1001 & 0b11 -> 0b0001 (a0 = 1)`

`srli` "`Shift Right Logical Immediate`" sets `rd` to `rs1 >> imm`
Usage: `srli rd, rs1, imm[5]`
Example: `srli a0, (8), 1 -> 0b1000 >> 1 -> 0b0100 (a0 = 4)`

`slli` "`Shift Left Logical Immediate`" sets `rd` to `rs1 << imm`
Usage: `slli rd, rs1, imm[5]`
Example: `slli a0, (4), 1 -> 0b0100 << 1 -> 0b1000 (a0 = 8)`

`srai` "`Shift Right Arithmetic Immediate`" sets `rd` to `rs1 >> imm (arithmetic)`
Usage: `srai rd, rs1, imm[5]`
Example: `(example of sra) 1001 >>> 1 = 1100 (discarded get moved to the front)`

#### Register

`add` sets `rd` to `rs1 + rs2`
Usage: `add rd, rs1, rs2`

`sub` sets `rd` to `rs1 - rs2`
Usage: `sub rd, rs1, rs2`

`slt` "`Set Less Than`" sets `rd` to `1` if `rs1 < rs2` otherwise `0`
Usage: `slt rd, rs1, rs2`

`sltu` "`Set Less Than Unsigned`" sets `rd` to `1` if `rs1 < rs2` otherwise `0`, unlike `slt` however, this compares unsigned `rs1` and `rs2`
Usage: `sltu rd, rs1, rs2`

`sll` sets `rd` to `rs1 << rs2`
Usage: `sll rd, rs1, rs2`

`srl` sets `rd` to `rs1 >> rs2`
Usage: `srl rd, rs1, rs2`

`sra` sets `rd` to `rs1 >>> rs2`
Usage: `sra rd, rs1, rs2` *see `Immediate/srai`*

`xor` sets `rd` to `rs1 ^ rs2`
Usage: `xor rd, rs1, rs2`
Example: `xor a0, (6), (5) -> 0b110 ^ 0b101 -> 0b11 (a0 = 3)`

`or` sets `rd` to `rs1 | rs2`
Usage: `or rd, rs1, rs2`
Example: `or a0, (6), (3) -> 0b110 | 0b011 -> 0b111 (a0 = 7)`

`and` sets `rd` to `rs1 & rs2`
Usage: `or rd, rs1, rs2`
Example: `or a0, (12), (5) -> 0b1100 & 0b0101 -> 0b0100 (a0 = 4)`

#### Unconditional Jump

`jal` sets `rd` to the current `pc` + `4`, and sets `pc` to `pc + imm`
Usage: `jal rd, imm[20]` *(note: `imm[20]` can be an assemlabel)*

`jalr` sets `rd` to the current `pc` + `4`, and sets `pc` to `rs1 + imm`
Usage: `jalr rd, rs1, imm[12]`

    Examples:
    (jal)
    label:
        ...
        jal zero, label (jal zero, -4)
    
    (jalr)
    label1:
        jal ra, label2

    label2:
        jalr zero, ra, 0 (return)

#### Branch (Conditional Jump)

("beq", "bne", "blt", "bge", "bltu", "bgeu"): 0b1100011,

`beq` "`Branch if EQual`" jumps by offset `imm` if `rs1 == rs2`
Usage: `beq rs1, rs2, imm[12]`

`bne` "`Branch if Not Equal`" jumps by offset `imm` if `rs1 != rs2`
Usage: `bne rs1, rs2, imm[12]`

`blt` "`Branch if Less Than`" jumps by offset `imm` if `rs1 < rs2`
Usage: `blt rs1, rs2, imm[12]`

`bge` "`Branch if Greater Equal`" jumps by offset `imm` if `rs1 >= rs2`
Usage: `bge rs1, rs2, imm[12]`

`bltu` "`Branch if Less Than (Unsigned)`" jumps by offset `imm` if `rs1 == rs2`, compares unsigned `rs`
Usage: `bltu rs1, rs2, imm[12]`

`bgeu` "`Branch if Greater Equal (Unsigned)`" jumps by offset `imm` if `rs1 >= rs2`, compares unsigned `rs`
Usage: `bgeu rs1, rs2, imm[12]`

    note: bgt is not a real instruction because 
          it can be filled by an inverted blt (blt rs2, rs1, imm[12])

#### Environment

`ebreak` pass control to the debugger
`ecall` call a system function (will throw a `Fault`)

### Pseudo-Instructions
    note: True is what the assembler converts the code into.
          "(instr)" means that the instruction can be optimised away
          "instr;" means that it gets converted into multiple instructions

`li` loads an immediate from `0x00000000 -> 0xffffffff` into `rd`
The assembler optimises this for you.
Usage: `li rd, imm[32]`
True: `(lui rd, imm >> 12); (addi rd, imm & 0xfff)`


### Environment (EBREAK)

`ebreak` does not accept registers, however the debugger will read certain registers to perform special debug requests.
`x31` is the request type
`x29-x30` are parameters
|            |`x31` |`x30`         |`x29`        |
|------------|------|--------------|-------------|
|core dump   |`0x00`|`N/A`         |`N/A`        |
|memory dump |`0x01`|`region_start`|`region_size`|
|shutdown    |`0x02`|`N/A`         |`N/A`        |
|dump all    |`0x03`|`region_start`|`region_size`|