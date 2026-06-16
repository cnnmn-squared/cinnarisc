# mypy: disable-error-code="no-redef"

from risclib import FileProcessor
from typing import NamedTuple, Literal
from dataclasses import dataclass

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


class CriticalError(BaseException):

    def __init__(self, *args):
        super().__init__(*args)


class GlobalSymbol:

    def __init__(self, symbol: str, at: int, size: int):
        self.symbol = symbol
        self.at = at
        self.size = size


class Line(NamedTuple):
    text: str
    lineno: int


class Instruction:

    def __init__(self, line: Line, assembly: str) -> None:
        self.assemble: str = assembly  # what the assembler sees
        self.line = line  # what the user sees

    def __repr__(self) -> str:
        return f"[instruction '{self.assemble}' ('{self.line.text}' @ {self.line.lineno})]"


class Error:

    def __init__(self, line: Line, etype: Literal["warn", "error"], code: int,
                 message: str, hint: str) -> None:
        self.code = code
        self.line = line
        self.type = etype
        self.message = message
        self.hint = hint


@dataclass
class Assembly:
    # symbols: dict[str, GlobalSymbol]
    origin: int

    instructions: list[Instruction]
    data_region: bytearray


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


def int_from_any(imm: str) -> int:
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
    if not a:
        raise Exception("")

    if a in label_cache.keys():
        return label_cache[a] - cinsta - 4

    if a in varl.keys():
        return varl[a]

    # it is the callers job to enforce that a is integer
    return int_from_any(a)


class AssembleError(Error):

    def __init__(self, line, etype, code, message, hint) -> None:
        super().__init__(line, etype, code, message, hint)


def assemble_instructions(
        instructions: list[Instruction]) -> tuple[bytes, list[Error]]:
    errors: list[Error] = []
    full: bytearray = bytearray()
    cinsta = 0
    for instr in instructions:
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
            errors.append(
                AssembleError(instr, "error", 0x0, "Invalid Instruction", ""))

        encoded: int = opcode
        match opcode:
            case 0b0110111:  # lui
                rdn, imms, *_ = args

                rd: int = getreg(rdn)

                imm20 = int_from_any(imms) & 0xfffff

                encoded |= (rd << 7)
                encoded |= (imm20 << 12)

            case 0b0010111:  # auipc
                rdn, imms, *_ = args

                rd: int = getreg(rdn)

                imm20 = int_from_any(imms) & 0xfffff

                encoded |= (rd << 7)
                encoded |= (imm20 << 12)

        fn3: int
        for k, v in FN3.items():
            if operator in k:
                fn3 = v

        match opcode:
            case 0b1101111:  # jal
                try:

                    rdn, j, *_ = args

                    rd: int = getreg(rdn)
                    offset: int = int_from_any(j)

                    # imm[20|10:1|11|19:12]
                    j12_19 = (offset >> 12) & 0xff
                    j11 = (offset >> 11) & 0x1
                    j1_10 = (offset >> 1) & 0x3ff
                    j20 = (offset >> 20) & 0x1

                    encoded |= (rd << 7)

                    encoded |= (j12_19 << 12)
                    encoded |= (j11 << 20)
                    encoded |= (j1_10 << 21)
                    encoded |= (j20 << 31)

                except ValueError:
                    errors.append(
                        AssembleError(1, instr.line.lineno, instr.line.text,
                                      "expected 2 operands but only got one.",
                                      "did you remember to add `ra`?"))

            case 0b1100111:  # jalr
                # log(args)
                try:
                    rdn, rs1n, offsets, *_ = args

                    rd: int = getreg(rdn)
                    rs1: int = getreg(rs1n)

                    imm12 = int_from_any(offsets) & 0xfff

                    encoded |= (rd << 7)
                    encoded |= (fn3 << 12)
                    encoded |= (rs1 << 15)
                    encoded |= (imm12 << 20)
                except LookupError as e:
                    errors.append(
                        AssembleError(999, instr.line.lineno, instr.line.text,
                                      e, ""))

            # general ainstr

            case 0b1100011:  # branch
                rs1n, rs2n, offsets, *_ = args

                rs1: int = getreg(rs1n)
                rs2: int = getreg(rs2n)

                offset = int_from_any(offsets)

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

                imm12 = int_from_any(offsets) & 0xfff

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
                imm: int = int_from_any(offsetty)

                encoded |= ((imm & 0b11111) << 7)
                encoded |= (rs1 << 15)
                encoded |= (fn3 << 12)
                encoded |= (rs2 << 20)
                encoded |= ((imm >> 5 & 0xf7) << 25)

            case 0b0010011:  # i
                rdn, rs1n, offsets, *_ = args

                rd: int = getreg(rdn)
                rs1: int = getreg(rs1n)

                imm12 = int_from_any(offsets) & 0xfff

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


def process_data_section(
        lines: list[Line]) -> tuple[dict[str, GlobalSymbol], bytearray]:
    # ({name: value}, region)
    # text is stored as {name: location(offset)}
    data_encoded: bytearray = bytearray()
    symbolmap: dict[str, GlobalSymbol] = {}

    data_ptr: int = 0

    def align(what: int, how_much: int, de: bytearray) -> int:
        rem = (how_much - (what % how_much)) % how_much
        de.extend([0] * rem)
        return what + rem

    for line in lines:
        var: str = line.text.strip(" ")
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
                vali = int_from_any(value)
                if vali > 0xff:
                    raise Exception(
                        f"value {vali} is too large for type {typeof} ({var})")

                symbolmap[name] = GlobalSymbol(name, data_ptr, 1)

                data_ptr += 1
                data_encoded.extend([vali])

            case "half":
                data_ptr = align(data_ptr, 2, data_encoded)
                vali = int_from_any(value)
                if vali > 0xffff:
                    raise Exception(
                        f"value {vali} is too large for type {typeof} ({var})")

                symbolmap[name] = GlobalSymbol(name, data_ptr, 2)

                data_ptr += 2
                data_encoded.extend(vali.to_bytes(2, "little"))

            case "word":
                data_ptr = align(data_ptr, 4, data_encoded)
                vali = int_from_any(value)
                if vali > 0xffff_ffff:
                    raise Exception(
                        f"value {vali} is too large for type {typeof} ({var})")

                symbolmap[name] = GlobalSymbol(name, data_ptr, 4)

                data_ptr += 4
                data_encoded.extend(vali.to_bytes(4, "little"))

            case "string":
                string = value.strip('"')

                length = len(string)

                symbolmap[name] = GlobalSymbol(name, data_ptr, length)

                data_encoded.extend([ord(ch) for ch in string])

            case "macro":
                # $name in the instructions will replace with the value given,
                # not the address of the value, useful for macros.
                # integer only,
                # usage:
                # name: macro = int (data region)
                # inst __, __, name (instruction region)

                vali = int_from_any(value)
                symbolmap[name] = GlobalSymbol(name, vali, 0)

            case "import":
                # import a bytes file into data
                path = value
                with open(path, "rb") as file:
                    data: bytes = file.read()
                    rem = (4 - (data.__len__() % 4)) % 4
                    data_ptr += data.__len__() + rem
                    data_encoded.extend(data)
                    data_encoded.extend([0 for _ in range(rem)])

    return (symbolmap, data_encoded)


def clean(input: list[str]) -> list[str]:
    inpt = remove_comments(input)
    inpt = [line.strip(" ") for line in inpt]
    return [line for line in inpt if line]


def remove_comment(input: str) -> str:
    return input.partition("#")[0]


def int_to_lui_addi_pair(register: str, number: int) -> list[str]:
    # simple optimisation
    pair: list[str] = []
    if number > 0xfff:  # cannot fit in addi
        pair.append(f"lui {register} {int(number) >> 12}")

    if number & 0xfff != 0:  # number already finished by lui
        pair.append(f"addi {register} {register} {int(number) & 0xfff}")

    return pair


def preprocess(lines: list[str]) -> tuple[Assembly, list[Error]]:
    origin: int = 0x0
    metalines: list[Line] = []
    issues: list[Error] = []
    for lineno, prelline in enumerate(lines):
        metalines.append(Line(prelline.strip(" "), lineno + 1))

    # * Segment into sections

    sections: dict[str, list[Line]] = {}
    within_section: str = ""

    for line in metalines:
        text, lineno = line
        # process directives
        if not text.startswith("."):
            if within_section != "":
                sections[within_section].append(line)
            continue

        iden, *other = text.split()
        match iden:
            case ".section":
                section_type, *_ = other
                sections[section_type] = []
                within_section = section_type

            case ".org":
                sorigin, *_ = other
                origin = int(sorigin)

    data_section: list[Line]
    text_section: list[Line]

    if ".data" not in sections.keys():
        issues.append(
            Error(metalines[0], "warn", 0, "no data section in file!",
                  "add `.section .data` to the top of your file."))
    else:
        data_section = sections[".data"]

    if origin == 0:
        issues.append(
            Error(metalines[0], "warn", 0,
                  "no origin directive in file! Guessed `0x1000`",
                  "add `.org [origin]` to the top of your file."))

        origin = 0x1000

    if ".text" not in sections.keys():
        issues.append(
            Error(metalines[0], "error", 1, "a .text section is required!",
                  "add `.section .text` before code."))

        raise CriticalError("yo")

    text_section = sections[".text"]

    symboltable: dict[str, GlobalSymbol] = {}
    encoded: bytearray = bytearray()

    if data_section:
        symboltable, encoded = process_data_section(data_section)

    # * Labels (1)
    # scan .text into a labeltable
    labeltable: dict[str, int] = {}  # addrs as mcode addr excluding resetv

    machloc: int = origin
    for line in text_section:
        if not line.text:
            continue

        if line.text.endswith(":"):
            labeltable[line.text.removesuffix(":")] = machloc
            continue

        machloc += 4

    # print(labeltable)

    # * Switch all lines to instructions & remove ,
    text_section = [line for line in text_section if line.text != ""]
    ntext: list[Line] = []
    for line in text_section:
        ntext.append(Line(line.text.strip(" "), line.lineno))

    text_section = ntext

    instructions: list[Instruction] = []
    for line in text_section:
        instructions.append(Instruction(line, line.text.replace(",", "")))

    # print(instructions)
    # * Labels (2)
    # convert all the references to labels into offsets
    # references beginning with * are absolute addresses (good to pair with li)

    machloc: int = origin
    for instruction in instructions:
        parts = instruction.assemble.split(" ")
        newparts: list[str] = []

        for part in parts:
            if part.startswith("*"):
                if part.removeprefix("*") in labeltable.keys():
                    part = str(labeltable[part.removeprefix("*")])

            if part in labeltable.keys():
                part = str(labeltable[part] - machloc - 4)

            newparts.append(part)

        # print(newparts)
        instruction.assemble = " ".join(newparts)

        machloc += 4

    # * Resolve constants & symbols

    if symboltable != {}:
        for instruction in instructions:
            parts = instruction.assemble.split(" ")
            newparts: list[str] = []

            for part in parts:
                if part in symboltable.keys():
                    part = str(symboltable[part].at)

                newparts.append(part)

            # print(newparts)
            instruction.assemble = " ".join(newparts)

            machloc += 4

    # * Make pseudoinstructions true

    ninstructions: list[Instruction] = []

    for instr in instructions:
        op, *rest = instr.assemble.split()

        match op:
            case "li":
                rd, val, *_ = rest
                pair = int_to_lui_addi_pair(rd, int_from_any(val))
                ninstructions.extend(
                    [Instruction(instr.line, ass) for ass in pair])

            case _:
                ninstructions.append(Instruction(instr.line, instr.assemble))

    instructions = ninstructions

    return Assembly(origin, instructions, encoded), issues


def assemble(file: str) -> tuple[bytes, list[Error]]:
    lines = file.splitlines()

    assembly, errors = preprocess(lines)

    mcode, errors = assemble_instructions(assembly.instructions)

    return FileProcessor.newfile(assembly.data_region, mcode), errors
