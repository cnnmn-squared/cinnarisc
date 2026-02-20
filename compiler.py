# mypy: disable-error-code="no-redef"

from dataclasses import dataclass

inpt = """
addi x1, x1, 24
sh x1, zero, 0
"""

opcode_matching: dict[tuple[str, ...], int] = {
    ("lb", "lh", "lw", "lbu", "lhu"): 0b0000011,
    ("sb", "sh", "sw"): 0b0100011,
    ("addi", "slti", "sltiu", "xori", "ori", "andi", "srli", "slli", "srai"): 0b0010011,
    ("add", "sub", "sll", "slt", "sltu", "xor", "srl", "sra", "or", "and"): 0b0110011,
    ("fence", "fence.tso", "pause"): 0b0001111,
    ("ebreak", "ecall"): 0b1110011
}

fn3_matching: dict[tuple[str, ...], int] = {(
    "jalr", "beq", "lb", "sb", "addi", "add", "sub", "fence", "fence.tso", "pause", "ecall", "ebreak"
):
                                                0b000,
                                            ("bne", "lh", "sh", "slli", "sll"):
                                                0b001,
                                            ("lw", "sw", "slti", "slt"):
                                                0b010,
                                            ("sltiu", "sltu"):
                                                0b011,
                                            ("blt", "lbu", "xori", "xor"):
                                                0b100,
                                            ("bge", "lhu", "srli", "srai", "srl", "sr"):
                                                0b101,
                                            ("bltu", "ori"):
                                                0b110,
                                            ("bgeu", "andi", "and"):
                                                0b111}

REGISTER_XNAMES = {
    "zero": 0,
    "ra": 1,
}

REGISTER_TNAMES = [f"x{i}" for i in range(32)]

print(REGISTER_TNAMES)


def getreg(name: str) -> int:
    if name in REGISTER_TNAMES:
        return REGISTER_TNAMES.index(name)

    elif name in REGISTER_XNAMES.keys():
        return REGISTER_XNAMES[name]

    raise Exception(f"invalid register '{name}'")


def intfdhbo(imm: str) -> int:
    prefix = imm[:2]
    strimm = imm[2:]

    print(prefix, strimm)
    match prefix:
        case "0x":
            return int(strimm, 16)

        case "0b":
            return int(strimm, 2)

        case "0o":
            return int(strimm, 8)

        case _:
            return int(prefix + strimm)


labelcache: list[tuple[str, int]] = []  # [(name, ciidx * 4)]
full: bytearray = bytearray()

normalised = inpt.replace(",", "")
normalised = normalised.lower()

ciidx = 0
for line, instr in enumerate(normalised.splitlines()):
    if not line:
        continue
    if instr.endswith(":"):
        labelcache.append((instr.removesuffix(":"), ciidx * 4))

    operator, *args = instr.split()

    opcode: int
    for k, v in opcode_matching.items():
        if operator in k:
            opcode = v

    if not opcode:
        raise Exception(f"Assembler: Invalid Instruction {instr} on line {line}")

    encoded: int = opcode
    match opcode:
        case 0b0110111:  # lui
            rdn, imms, *_ = args

            rd: int = getreg(rdn)

            imm20 = intfdhbo(imms) & 0xfffff

            encoded |= (rd << 7)
            encoded |= (imm20 << 12)

        case 0b0010111:  # auipc
            rdn, imms, *_ = args

            rd: int = getreg(rdn)

            imm20 = intfdhbo(imms) & 0xfffff

            encoded |= (rd << 7)
            encoded |= (imm20 << 12)

    fn3: int
    for k, v in fn3_matching.items():
        if operator in k:
            fn3 = v

    match opcode:
        case 0b1101111:  # jal
            # ignore for now
            pass

        case 0b1100111:  # jalr
            rdn, rs1n, offsets, *_ = args

            rd: int = getreg(rdn)
            rs1: int = getreg(rs1n)

            imm12 = intfdhbo(offsets) & 0xfff

            encoded |= (rd << 7)
            encoded |= (fn3 << 12)
            encoded |= (rs1 << 15)
            encoded |= (imm12 << 20)

        # general instr

        case 0b1100011:  # beq
            pass

        case 0b0000011:  # l
            rdn, rs1n, offsets, *_ = args

            rd: int = getreg(rdn)
            rs1: int = getreg(rs1n)

            imm12 = intfdhbo(offsets) & 0xfff

            encoded |= (rd << 7)
            encoded |= (rs1 << 15)
            fn3: int
            if operator == "lb":
                fn3 = 0b000
            elif operator == "lh":
                fn3 = 0b001
            elif operator == "lw":
                fn3 = 0b010
            elif operator == "lbu":
                fn3 = 0b100
            elif operator == "lhu":
                fn3 = 0b101
            encoded |= (fn3 << 12)
            encoded |= (imm12 << 20)

        case 0b0100011:  # s
            rs2n, rs1n, offsetty, *_ = args

            rs1: int = getreg(rs1n)
            rs2: int = getreg(rs2n)
            imm: int = intfdhbo(offsetty)

            encoded |= ((imm & 0b11111) << 7)
            encoded |= (rs1 << 15)
            encoded |= (fn3 << 12)
            encoded |= (rs2 << 20)
            encoded |= ((imm >> 5 & 0xf7) << 25)

        case 0b0010011:  # i
            rdn, rs1n, offsets, *_ = args

            rd: int = getreg(rdn)
            rs1: int = getreg(rs1n)

            imm12 = intfdhbo(offsets) & 0xfff

            encoded |= (rd << 7)
            encoded |= (rs1 << 15)
            encoded |= (fn3 << 12)
            encoded |= (imm12 << 20)

        case 0b0110011:  # r
            rdn, rs1n, rs2n, *_ = args

            rd: int = getreg(rdn)
            rs1: int = getreg(rs1n)
            rs2: int = getreg(rs2n)
            fn7: int = 0b0100000 if operator in ("sra", "sub") else 0b0

            encoded |= (rd << 7)
            encoded |= (rs1 << 15)
            encoded |= (fn3 << 12)
            encoded |= (rs2 << 20)
            encoded |= (fn7 << 25)

    a = encoded & 0xff
    b = (encoded >> 8) & 0xff
    c = (encoded >> 16) & 0xff
    d = (encoded >> 24) & 0xff

    full.extend([a, b, c, d])

    ciidx += 4

print(full)
