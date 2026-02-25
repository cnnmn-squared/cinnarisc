# mypy: disable-error-code="no-redef"

from dataclasses import dataclass
import argparse

parser = argparse.ArgumentParser(
    prog="rv32i assembler",
    description="assembles rv32i instructions into riscv machine code.",
)

parser.add_argument("source")
parser.add_argument("destination")

cla = parser.parse_args()

inpt = open(cla.filename, "r").read()

opcode_matching: dict[tuple[str, ...] | str, int] = {
    ("lui"): 0b0110111,
    ("auipc"): 0b0010111,
    ("jal"): 0b1101111,
    ("jalr"): 0b1100111,
    ("beq", "bne", "blt", "bge", "bltu", "bgeu"): 0b1100011,
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
    "sp": 2,
    "gp": 3,
    "tp": 4,
    "t0": 5,
    "t1": 6,
    "t2": 7,
    "s0": 8,
    "fp": 8,
    "s1": 9,
    "a0": 10,
    "a1": 11,
    "a2": 12,
    "a3": 13,
    "a4": 14,
    "a5": 15,
    "a6": 16,
    "a7": 17,
    "s2": 18,
    "s3": 19,
    "s4": 20,
    "s5": 21,
    "s6": 22,
    "s7": 23,
    "s8": 24,
    "s9": 25,
    "s10": 26,
    "s11": 27,
    "t3": 28,
    "t4": 29,
    "t5": 30,
    "t6": 31,
}

REGISTER_TNAMES = [f"x{i}" for i in range(32)]

print(REGISTER_TNAMES)


def getreg(name: str) -> int:
    if name in REGISTER_TNAMES:
        return REGISTER_TNAMES.index(name)

    elif name in REGISTER_XNAMES.keys():
        return REGISTER_XNAMES[name]

    raise Exception(f"invalid register '{name}'")


def findlabel(target: str, nextinstrs: list[str]) -> int:
    tinsar = 0  # Temp INStruction Address Relative
    for instr in nextinstrs:
        if instr.endswith(":"):
            if instr.removesuffix(":") == target:
                return tinsar

            continue

        tinsar += 4

    raise Exception(f"Couldn't find label: {target}")


def precache_labels(instructions: list[str]) -> dict[str, int]:
    cache: dict[str, int] = {}
    tinsa: int = 0

    for instr in instructions:
        if instr.endswith(":"):
            cache[instr.removesuffix(':')] = tinsa - 4
            continue

        tinsa += 4

    return cache


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


def remove_comments(instructions: list[str]) -> list[str]:
    a: list[str] = [ins.partition("#")[0] for ins in instructions]
    a = [ins for ins in a if ins]

    return a


def intfrv(a: str, varl: dict[str, int]) -> int:
    try:
        return intfdhbo(a)
    except ValueError:
        if a not in varl.keys():
            raise NameError(f"{a} is not defined in the data section")

        return varl[a]


normalised = inpt.replace(",", "")
normalised = normalised.lower()

file = normalised.splitlines()

data_region: list[str] = file[file.index(".data"):file.index(".text")]
inst_region: list[str] = file[file.index(".text"):]

instructions = remove_comments(inst_region)
instructions = [ins.strip(" ") for ins in instructions]
instructions = [ins for ins in instructions if ins != ""]

labelcache = precache_labels(instructions)

print(f"labels: {labelcache}")

ufriend: list[str] = []
i = 0
for ins in instructions:
    ufriend.append(
        f"[0x{hex(i).removeprefix('0x').rjust(4, '0')}] {'  ' if ins.endswith(':') else ''}{ins} {'->' if ins.endswith(':') else ''}"
    )

    if not ins.endswith(":"):
        i += 4

print("program:\n", '\n'.join(ufriend), sep="")


def assemble_instructions(
    instructions: list[str], labels: dict[str, int], data: dict[str, int]
) -> bytes:
    full: bytearray = bytearray()
    cinsta = 0
    for line, instr in enumerate(instructions):
        if not instr:
            continue

        if instr.endswith(":"):
            continue

        if instr not in ("ebreak", "ecall"):

            operator, *args = instr.split()

        else:
            operator = instr

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

                imm20 = intfrv(imms, data) & 0xfffff

                encoded |= (rd << 7)
                encoded |= (imm20 << 12)

            case 0b0010111:  # auipc
                rdn, imms, *_ = args

                rd: int = getreg(rdn)

                imm20 = intfrv(imms, data) & 0xfffff

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

                imm12 = intfrv(offsets, data) & 0xfff

                encoded |= (rd << 7)
                encoded |= (fn3 << 12)
                encoded |= (rs1 << 15)
                encoded |= (imm12 << 20)

            # general instr

            case 0b1100011:  # branch
                rs1n, rs2n, target, *_ = args

                rs1: int = getreg(rs1n)
                rs2: int = getreg(rs2n)

                offset: int = labels[target] - cinsta

                # if offset == 0:
                #     offset = findlabel(target, instructions[line:])

                print("####", offset)

                s12: int = (offset >> 12) & 0x1
                s11: int = (offset >> 11) & 0x1
                s5_10: int = (offset >> 5) & 0x3f
                s1_4: int = (offset >> 1) & 0xf

                print(s1_4, s5_10, s11, s12)
                print((s12 << 12) | (s11 << 11) | (s5_10 << 5) | (s1_4 << 1))

                encoded |= (s11 << 7)
                encoded |= (s1_4 << 8)
                encoded |= (fn3 << 12)
                encoded |= (rs1 << 15)
                encoded |= (rs2 << 20)
                encoded |= (s5_10 << 25)
                encoded |= (s12 << 31)

            case 0b0000011:  # l
                rdn, rs1n, offsets, *_ = args

                rd: int = getreg(rdn)
                rs1: int = getreg(rs1n)

                imm12 = intfrv(offsets, data) & 0xfff

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
                imm: int = intfrv(offsetty, data)

                encoded |= ((imm & 0b11111) << 7)
                encoded |= (rs1 << 15)
                encoded |= (fn3 << 12)
                encoded |= (rs2 << 20)
                encoded |= ((imm >> 5 & 0xf7) << 25)

            case 0b0010011:  # i
                rdn, rs1n, offsets, *_ = args

                rd: int = getreg(rdn)
                rs1: int = getreg(rs1n)

                imm12 = intfrv(offsets, data) & 0xfff

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

            case 0b1110011:  # ebreak/ecall
                pass

        a = encoded & 0xff
        b = (encoded >> 8) & 0xff
        c = (encoded >> 16) & 0xff
        d = (encoded >> 24) & 0xff

        full.extend([a, b, c, d])

        cinsta += 4

    return full


def interpret_datregion(data_text: list[str]) -> tuple[dict[str, int], bytearray]:
    # ({name: value}, region)
    # text is stored as {name: location(offset)}
    data_encoded: bytearray = bytearray()
    nv: dict[str, int] = {}

    data_ptr: int = 0

    for var in data_text:
        var = var.strip(" ")
        # partitioned into name: type = value
        name, typval = var.split(":")
        typeof, value = typval.split("=")

        name = name.strip(" ")
        typeof = typeof.strip(" ")
        value = value.strip(" ")

        match typeof:
            case "byte":
                vali = intfdhbo(value)
                if vali > 0xff:
                    raise Exception(
                        f"value {vali} is too large for type {typeof} ({var})"
                    )

                nv[name] = data_ptr

                data_ptr += 4
                data_encoded.extend([vali, 0, 0, 0])

            case "half":
                vali = intfdhbo(value)
                if vali > 0xffff:
                    raise Exception(
                        f"value {vali} is too large for type {typeof} ({var})"
                    )

                nv[name] = data_ptr

                data_ptr += 4
                data_encoded.extend([vali & 0xff, vali >> 8, 0, 0])

            case "word":
                vali = intfdhbo(value)
                if vali > 0xffff_ffff:
                    raise Exception(
                        f"value {vali} is too large for type {typeof} ({var})"
                    )

                nv[name] = data_ptr

                data_ptr += 4
                data_encoded.extend([
                    vali & 0xff, (vali >> 8) & 0xff, (vali >> 16) & 0xff,
                    (vali >> 24) & 0xff
                ])

            case "string":
                string = value.split("\"")

                nv[name] = data_ptr

                length = len(string[0])
                rem = (4 - (length % 4)) % 4
                data_ptr += length + rem

                data_encoded.extend([ord(ch) for ch in string[0]])

            case "macro":
                # $name in the instructions will replace with the value given, not the address of the value, useful for macros.
                # integer only,
                # usage:
                # name: macro = int (data region)
                # inst __, __, $name (instruction region)

                vali = intfdhbo(value)
                nv[name] = vali

    return (nv, data_encoded)


data, data_encoded = interpret_datregion(data_region)
full = assemble_instructions(instructions, labelcache, data)

print(full)
print(labelcache)

with open(cla.destination, "wb") as img:
    img.write(full)
