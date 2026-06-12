# mypy: disable-error-code="no-redef"

from xlibx import FileProcessor

DEBUG: bool = False

OPCODES: dict[tuple[str, ...] | str, int] = {
    ("lui"): 0b0110111,
    ("auipc"): 0b0010111,
    ("jal", "jail"): 0b1101111,
    ("jalr", "jallr"): 0b1100111,
    ("beq", "bne", "blt", "bge", "bltu", "bgeu"): 0b1100011,
    ("lb", "lh", "lw", "lbu", "lhu"): 0b0000011,
    ("sb", "sh", "sw"): 0b0100011,
    ("addi", "slti", "sltiu", "xori", "ori", "andi", "srli", "slli", "srai"):
    0b0010011,
    ("add", "sub", "sll", "slt", "sltu", "xor", "srl", "sra", "or", "and"):
    0b0110011,
    ("fence", "fence.tso", "pause"): 0b0001111,
    ("ebreak", "ecall"): 0b1110011
}

FN3: dict[tuple[str, ...], int] = {
    ("jalr", "beq", "lb", "sb", "addi", "add", "sub", "fence", "fence.tso", "pause", "ecall", "ebreak"):
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
    0b111
}

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

SIMPLE_PSUEDO_INSTRUCTIONS: dict[str, list[str]] = {
    "call": ["jal ra, {}"],
    "ret": ["jalr zero, ra, 0"]
}

REGISTER_TNAMES = [f"x{i}" for i in range(32)]


class Instruction:

    def __init__(self, assemble: str, true: str, lineno: int) -> None:
        self.assemble: str = assemble  # what the assembler sees
        self.true: str = true  # what the user sees
        self.lineno: int = lineno  # the lineno in the file

    def __repr__(self) -> str:
        return f"[instruction '{self.assemble}' ('{self.true}' @ {self.lineno})]"


def log(*values: object) -> None:
    print(" ".join([str(v) for v in values])) if DEBUG else None


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


def precache_labels(instructions: list[Instruction]) -> dict[str, int]:
    cache: dict[str, int] = {}
    tinsa: int = 0

    for instr in instructions:
        if instr.assemble.endswith(":"):
            cache[instr.assemble.removesuffix(':')] = tinsa  # -4
            continue

        tinsa += 4

    return cache


def intfdhbo(imm: str) -> int:
    prefix = imm[:2]
    strimm = imm[2:]

    log(prefix, strimm)
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


def intfrv(a: str, varl: dict[str, int], label_cache: dict[str, int],
           cinsta: int) -> int:
    # ! adding offsets of labels into this is so wrong but i dont want to rewrite the dynwriter just for it.
    try:
        if a in label_cache.keys():
            return label_cache[a] - cinsta - 4

        return intfdhbo(a)
    except ValueError:
        if a not in varl.keys():
            raise NameError(f"{a} is not defined in the data section")

        return varl[a]


# def psuedo_pass(instructions: list[str]) -> list[str]:
#     new_set: list[str] = []
#     for instruction in instructions:
#         op, *args = instruction.split()
#
#         match op:
#             case "call":
#                 # call becomes jal ra, SYMBOL
#                 new_set.append(f"jal ra, {args[0]}")
#
#             case "ret":
#                 # ret becomes jalr zero, ra, 0  (jump back to ra and dont save)
#                 new_set.append("jalr zero, ra, 0")


class AssembleError:

    def __init__(self, code: int, lineno: int, linet: str, accomp: str,
                 hint: str) -> None:
        # code: errorcode
        # lineno: line number, eg. 189
        # linet: line text, eg. addi x1, x0, 512
        # accomp: more on the error, eg. addi expects an immediate, not a register
        self.code = code
        self.lineno = lineno
        self.linet = linet
        self.message = accomp
        self.hint = hint


def assemble_instructions(
        instructions: list[Instruction], labels: dict[str, int],
        data: dict[str, int]) -> tuple[bytes, list[AssembleError]]:
    errors: list[AssembleError] = []
    full: bytearray = bytearray()
    cinsta = 0
    for line, instr in enumerate(instructions):
        ainstr: str = instr.assemble
        if not ainstr:
            continue

        if ainstr.endswith(":"):
            continue

        if ainstr not in ("ebreak", "ecall"):

            operator, *args = ainstr.split()

        else:
            operator = ainstr

        opcode: int = 0
        for k, v in OPCODES.items():
            if operator in k:
                opcode = v

        if opcode == 0:
            # raise Exception(
            #     f"Assembler: Invalid Instruction {ainstr} on line {line}")
            errors.append(
                AssembleError(0x0, instr.lineno, instr.true,
                              "Invalid Instruction", ""))

        encoded: int = opcode
        match opcode:
            case 0b0110111:  # lui
                rdn, imms, *_ = args

                rd: int = getreg(rdn)

                imm20 = intfrv(imms, data, labels, cinsta) & 0xfffff

                encoded |= (rd << 7)
                encoded |= (imm20 << 12)

            case 0b0010111:  # auipc
                rdn, imms, *_ = args

                rd: int = getreg(rdn)

                imm20 = intfrv(imms, data, labels, cinsta) & 0xfffff

                encoded |= (rd << 7)
                encoded |= (imm20 << 12)

        fn3: int
        for k, v in FN3.items():
            if operator in k:
                fn3 = v

        match opcode:
            case 0b1101111:  # jal
                # ignore for now
                try:

                    rdn, j, *_ = args

                    rd: int = getreg(rdn)
                    tolabel: int = (intfrv(j, data, labels, cinsta))

                    # imm[20|10:1|11|19:12]
                    j12_19 = (tolabel >> 12) & 0xff
                    j11 = (tolabel >> 11) & 0x1
                    j1_10 = (tolabel >> 1) & 0x3ff
                    j20 = (tolabel >> 20) & 0x1

                    encoded |= (rd << 7)

                    encoded |= (j12_19 << 12)
                    encoded |= (j11 << 20)
                    encoded |= (j1_10 << 21)
                    encoded |= (j20 << 31)

                except ValueError:
                    errors.append(
                        AssembleError(1, instr.lineno, instr.true,
                                      "expected 2 operands but only got one.",
                                      "did you remember to add `ra`?"))

            case 0b1100111:  # jalr
                # log(args)
                try:
                    rdn, rs1n, offsets, *_ = args

                    rd: int = getreg(rdn)
                    rs1: int = getreg(rs1n)

                    imm12 = intfrv(offsets, data, labels, cinsta) & 0xfff

                    encoded |= (rd << 7)
                    encoded |= (fn3 << 12)
                    encoded |= (rs1 << 15)
                    encoded |= (imm12 << 20)
                except LookupError as e:
                    errors.append(
                        AssembleError(999, instr.lineno, instr.true, e, ""))

            # general ainstr

            case 0b1100011:  # branch
                rs1n, rs2n, target, *_ = args

                rs1: int = getreg(rs1n)
                rs2: int = getreg(rs2n)

                offset: int = labels[target] - cinsta - 4

                # if offset == 0:
                #     offset = findlabel(target, instructions[line:])

                log("####", offset)

                s12: int = (offset >> 12) & 0x1
                s11: int = (offset >> 11) & 0x1
                s5_10: int = (offset >> 5) & 0x3f
                s1_4: int = (offset >> 1) & 0xf

                log(s1_4, s5_10, s11, s12)
                log((s12 << 12) | (s11 << 11) | (s5_10 << 5) | (s1_4 << 1))

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

                imm12 = intfrv(offsets, data, labels, cinsta) & 0xfff

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
                imm: int = intfrv(offsetty, data, labels, cinsta)

                encoded |= ((imm & 0b11111) << 7)
                encoded |= (rs1 << 15)
                encoded |= (fn3 << 12)
                encoded |= (rs2 << 20)
                encoded |= ((imm >> 5 & 0xf7) << 25)

            case 0b0010011:  # i
                rdn, rs1n, offsets, *_ = args

                rd: int = getreg(rdn)
                rs1: int = getreg(rs1n)

                imm12 = intfrv(offsets, data, labels, cinsta) & 0xfff

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

    return full, errors


def interpret_datregion(
        data_text: list[str]) -> tuple[dict[str, int], bytearray]:
    # ({name: value}, region)
    # text is stored as {name: location(offset)}
    data_encoded: bytearray = bytearray()
    nv: dict[str, int] = {}

    data_ptr: int = 0

    def align(what: int, how_much: int, de: bytearray) -> int:
        rem = (how_much - (what % how_much)) % how_much
        de.extend([0] * rem)
        return what + rem

    for var in data_text:
        var = var.strip(" ")
        # partitioned into name: type = value
        log("var", var)
        if not var:
            continue

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
                        f"value {vali} is too large for type {typeof} ({var})")

                nv[name] = data_ptr

                data_ptr += 1
                data_encoded.extend([vali])

            case "half":
                data_ptr = align(data_ptr, 2, data_encoded)
                vali = intfdhbo(value)
                if vali > 0xffff:
                    raise Exception(
                        f"value {vali} is too large for type {typeof} ({var})")

                nv[name] = data_ptr

                data_ptr += 2
                data_encoded.extend(vali.to_bytes(2, "little"))

            case "word":
                data_ptr = align(data_ptr, 4, data_encoded)
                vali = intfdhbo(value)
                if vali > 0xffff_ffff:
                    raise Exception(
                        f"value {vali} is too large for type {typeof} ({var})")

                nv[name] = data_ptr

                data_ptr += 4
                data_encoded.extend(vali.to_bytes(4, "little"))

            case "string":
                string = value.split("\"")

                nv[name] = data_ptr

                length = len(string[1])
                rem = (4 - (length % 4)) % 4
                log(rem)
                data_ptr += length + rem

                log(string, data_ptr)
                data_encoded.extend([ord(ch) for ch in string[1]])
                data_encoded.extend([0 for i in range(rem)])

                log(data_encoded)

            case "macro":
                # $name in the instructions will replace with the value given,
                # not the address of the value, useful for macros.
                # integer only,
                # usage:
                # name: macro = int (data region)
                # inst __, __, name (instruction region)

                vali = intfdhbo(value)
                nv[name] = vali

            case "import":
                # import a bytes file into data
                path = value
                with open(path, "rb") as file:
                    data: bytes = file.read()
                    rem = (4 - (data.__len__() % 4)) % 4
                    data_ptr += data.__len__() + rem
                    data_encoded.extend(data)
                    data_encoded.extend([0 for _ in range(rem)])

    return (nv, data_encoded)


def clean(input: list[str]) -> list[str]:
    inpt = remove_comments(input)
    inpt = [line.strip(" ") for line in inpt]
    return [line for line in inpt if line]


def remove_comment(input: str) -> str:
    return input.partition("#")[0]


def assemble(file: str) -> tuple[bytes, list[AssembleError]]:
    lines = file.splitlines()

    data_region: list[str] = lines[lines.index(".data"):lines.index(".text")]
    data_file_lines: int = len(data_region)
    data_region = clean(data_region)
    inst_region: list[str] = lines[lines.index(".text"):]

    instructions: list[Instruction] = []

    lineno = data_file_lines - 1

    for ins in inst_region:
        newins: str = ins.replace(",", " ")
        newins = remove_comment(newins)
        newins = newins.strip(" ")
        lineno += 1

        instructions.append(Instruction(newins, ins.strip(" "), lineno))

    instructions = [
        instruction for instruction in instructions
        if instruction.assemble != ""
    ]
    # print(instructions)

    labelcache = precache_labels(instructions)

    # data: pointers for all the data when the text section is assembled
    # data_encoded: the data section to be attached to the file
    data, data_encoded = interpret_datregion(data_region[1:])

    text, errors = assemble_instructions(instructions[1:], labelcache,
                                         data)  # text region is included

    return FileProcessor.newfile(data_encoded, text), errors


# ufriend: list[str] = []
# i = 0
# for ins in instructions:
#     ufriend.append(f"[0x{hex(i).removeprefix('0x').rjust(4, '0')}] \
# {'  ' if not ins.endswith(':') else ''}{ins} {'->' if ins.endswith(':') else ''}"
#                   )
#
#   if not ins.endswith(":"):
#        i += 4
