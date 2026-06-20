# An example "Hello, World!" program

.org 0x00001000

.section .data
    hellow: string = "Hello, World!"    # the string to write
    hellol: const = 13                  # length of "Hello, World!"

    uart: const = 0x00010000            # the address of UART

.section .text

_start: 
    li t0, uart                         # load 0x00010000 into t0

    addi t1, zero, -1                   # t1 = -1
    addi t2, zero, hellol               # t2 = 13
    li t3, hellow                       # t3 = *hellow

    _writel:
        addi t1, t1, 1                  # t1 += 1 (i)
        beq t1, t2, _writelsk           # if t1 == string.len { jump to _writelsk }

        add t5, t1, t3                  # t5* = *hellow + i
        lb t4, t5, 0                    # t4 = mem[t5]
        sb t4, t0, 0                    # write t4 to uart

        jal zero, _writel               # jump to _writel


    _writelsk:
        jal zero, _writelsk             # prevent a Fault (invalid instruction)

