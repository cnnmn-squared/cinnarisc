.org 0x1000

.section .text

# branch & jump tests

jal zero, skip

branchfail:
    li x31, 0x10000
    jalr zero, x31, 0

skip:

bne zero, zero, branchfail
beq ra, ra, rtests

rtests:

# register tests

li x31, 0xffffffff      # -1 (u32::max)

# only 2 sign tests are needed because all behave the same aside from if a register getch could be broken
addi ra, zero, -1
bne ra, x31, fail

addi x2, zero, -1
bne x2, x31, fail

# add (-), addi (+-), sub (-)

addi ra, zero, 500      # ra = 500
add ra, ra, zero        # nop (fancy)
addi t0, ra, 0          # t0 = ra

bne t0, ra, fail        # t0 != ra
beq ra, zero, fail      # ra == 0

addi s0, zero, -5       # s0 = -5
add t0, ra, s0          # t0 = 500 + -5

addi x31, zero, 495 
bne t0, x31, fail       # t0(exp495) != 495

sub t0, ra, s0          # t0 = 500 - -5 (500 + 5)
addi x31, zero, 505
bne t0, x31, fail       # t0(exp505) != 505

# ori, xori, andi, (& counterparts)

addi ra, zero, 0x8
ori ra, ra, 0x2         # expecting 0b1010 (0xa)
addi x31, zero, 0xa
bne x31, ra, fail       # assert 0x8 | 0x2 == 0xa

xori ra, ra, 0x2        # expecting 0b1000 (0x8)
addi x31, zero, 0x8
bne x31, ra, fail       # assert 0xa | 0x2 == 8

addi ra, zero, -1
andi ra, ra, 1          # 0xffff_ffff & 0x0000_0001 = 1
addi x31, zero, 1
bne ra, x31, fail

jal zero, rtests

fail:
    li x31, 0x10000
    jalr zero, x31, 0       # force a trap
