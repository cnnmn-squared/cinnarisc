from enum import IntEnum
from typing import Self, Literal, TypeAlias
from devices import Bus

XLEN: int = 32
IALIGN: int = 32

RESET_VECTOR = 0x00001000

DECODEVALS: TypeAlias = dict[Literal["opcode", "7:11", "12:14", "15:19", "20:24",
                                     "25:31", "12:31", "20:31"], int]


class TCause(IntEnum):
    INSTRUCTION_ADDRESS_MISALIGNED = 0x0
    INSTRUCTION_ACCESS_FAULT = 0x1
    ILLEGAL_INSTRUCTION = 0x2
    BREAKPOINT = 0x3

    LOAD_ADDRESS_MISALIGNED = 0x4
    LOAD_ACCESS_FAULT = 0x5

    STORE_ADDRESS_MISALIGNED = 0x6
    STORE_ACCESS_FAULT = 0x7

    # Environment Faults

    ENV_CALL_FROM_U = 0x8
    ENV_CALL_FROM_S = 0x9
    # 0xA reserved
    ENV_CALL_FROM_M = 0xB

    # Page faults
    INSTRUCTION_PAGE_FAULT = 0xC
    LOAD_PAGE_FAULT = 0xD
    # 0xE reserved
    STORE_PAGE_FAULT = 0xf

    # Misc
    DOUBLE_TRAP = 0x10
    # 0x11 reserved
    SOFTWARE_CHECK = 0x12
    HARDWARE_ERROR = 0x13

    # 0x14-0x17 reserved
    # 0x18-0x1f designated for custom
    DEVICE_FAULT = 0x18
    INVALID_DEVICE_REGION = 0x19

    # 0x20-0x2f reserved
    # 0x30-0x3f designated for custom
    # 0x40>> reserved


class Trap(Exception):

    def __init__(self, cause: TCause):  # noqa
        self.cause = cause


class MintDBWbounded:

    def __init__(self, fromint: int, xlen: int) -> None:
        self.v = fromint
        self.xlen = xlen

    def _coerce(self, other: Self | int) -> int:
        return other.v if isinstance(other, MintDBWbounded) else other

    def _child(self, _from: int) -> Self:
        return type(self)(self._bounds(_from), self.xlen)

    def _childself(self, _from: int) -> None:
        self.v = self._bounds(_from)

    def __add__(self, other: Self | int) -> Self:
        return self._child(self.v + self._coerce(other))

    def __sub__(self, other: Self | int) -> Self:
        return self._child(self.v - self._coerce(other))

    def __mul__(self, other: Self | int) -> Self:
        return self._child(self.v - self._coerce(other))

    def __truediv__(self, other: Self | int) -> Self:
        return self._child(self.v - self._coerce(other))

    def __iadd__(self, other: Self | int) -> Self:
        self.v += self._coerce(other)
        return self

    def __isub__(self, other: Self | int) -> Self:
        self.v -= self._coerce(other)
        return self

    @staticmethod
    def _bounds(tobe: int) -> int:
        wrapped = ((tobe + 2**31) % 2**32) - 2**31

        return wrapped

    def _selfbounds(self):
        self.v = ((self.v + 2**31) % 2**32) - 2**31

    def __repr__(self) -> str:
        return str(self.v)

    def __str__(self) -> str:
        return self.__repr__()

    def __int__(self) -> int:
        return self.v

    def __eq__(self, value: object) -> bool:
        if isinstance(value, MintDBWbounded):
            return self.v == value.v

        elif isinstance(value, int):
            return self.v == value

        else:
            return NotImplemented

    def __ne__(self, value: object) -> bool:
        return not self.__eq__(value)


class ALU:

    @staticmethod
    def sign(val: MintDBWbounded) -> int:
        # MintDBWbounded carries bitwidth information so it makes it simpler
        msktty = 1 << (val.xlen - 1)
        return (int(val) ^ msktty) - msktty

    @staticmethod
    def usign(val: MintDBWbounded) -> int:
        # same stuff as sign
        return int(val) & ((1 << val.xlen) - 1)

    @staticmethod
    def add(a: int, b: int) -> int:
        return a + b

    @staticmethod
    def sub(a: int, b: int) -> int:
        return a - b

    @staticmethod
    def mul(a: int, b: int) -> int:
        return a * b

    @staticmethod
    def lshift(a: int, shamt: int) -> int:
        return a << shamt

    @staticmethod
    def rshift(a: int, shamt: int) -> int:
        return a >> shamt


class Memory:

    def __init__(self, size: int) -> None:
        if size > 2**XLEN:
            raise Exception(f"`size` is too large! limit is 2^{XLEN}, size({size})")

        self.data = bytearray(size)

    def _addrsafe(self, addr: int) -> None:
        if addr < 0:
            raise Trap(TCause.HARDWARE_ERROR)

    def _addrbounds(self, addr: int) -> int:
        return addr % self.data.__len__()

    def lvfab(self, address: int) -> int:
        return self.data[address]

    def sviab(self, address: int, value: int) -> None:
        if value > 0xff:
            raise Trap(TCause.STORE_ACCESS_FAULT)
        self.data[address] = value

    def load_half(self, address: int) -> int:
        if address % 2 != 0:
            raise Trap(TCause.LOAD_ADDRESS_MISALIGNED)

        return (self.lvfab(address + 1) << 8) + self.lvfab(address)

    def load_word(self, address: int) -> int:
        if address % 4 != 0:
            raise Trap(TCause.LOAD_ADDRESS_MISALIGNED)

        return (self.data[address + 3] << 24) + (self.data[
            address + 2] << 16) + (self.data[address + 1] << 8) + self.data[address]

    def store_half(self, address: int, value: int) -> None:
        if address % 2 != 0:
            raise Trap(TCause.STORE_ADDRESS_MISALIGNED)

        self.data[address] = value & 0xff
        self.data[address + 1] = (value >> 8) & 0xff

    def store_word(self, address: int, value: int) -> None:
        if address % 4 != 0:
            raise Trap(TCause.STORE_ADDRESS_MISALIGNED)

        self.data[address] = value & 0xff
        self.data[address + 1] = (value >> 8) & 0xff
        self.data[address + 2] = (value >> 16) & 0xff
        self.data[address + 3] = (value >> 24) & 0xff


class InstructionDecoder:

    @staticmethod
    def gimme(cons: int, lo: int, hi: int) -> int:  # ヾ(＾∇＾)
        return (cons >> lo) & (2**((hi + 1) - lo) - 1)

    @staticmethod
    def opcode(instr: int) -> int:
        return InstructionDecoder.gimme(instr, 0, 6)

    @staticmethod
    def d711(instr: int) -> int:  # used as rd/imm[0:4]
        # preset 1, rd/imm[0:4]
        return InstructionDecoder.gimme(instr, 7, 11)

    @staticmethod
    def d12_14(instr: int) -> int:
        # preset 2, fn3 in most instr formats.
        return InstructionDecoder.gimme(instr, 12, 14)

    @staticmethod
    def d15_19(instr: int) -> int:
        # preset 3, rs1 in most instructions
        return InstructionDecoder.gimme(instr, 15, 19)

    @staticmethod
    def d20_24(instr: int) -> int:
        # preset 4, rs2 in S/R/B instr formats
        return InstructionDecoder.gimme(instr, 20, 24)

    @staticmethod
    def d25_31(instr: int) -> int:
        # preset 5, fn7 in rtype
        return InstructionDecoder.gimme(instr, 25, 31)

    @staticmethod
    def d12_31(instr: int) -> int:
        # preset 6, imm20 in U-type
        return InstructionDecoder.gimme(instr, 12, 31)

    @staticmethod
    def d20_31(instr: int) -> int:
        # preset 7, imm12
        return InstructionDecoder.gimme(instr, 12, 31)

    @staticmethod
    def decode_simples(instr: int) -> DECODEVALS:
        return {
            "opcode": InstructionDecoder.opcode(instr),
            "7:11": InstructionDecoder.d711(instr),
            "12:14": InstructionDecoder.d12_14(instr),
            "15:19": InstructionDecoder.d15_19(instr),
            "20:24": InstructionDecoder.d20_24(instr),
            "25:31": InstructionDecoder.d25_31(instr),
            "12:31": InstructionDecoder.d12_31(instr),
            "20:31": InstructionDecoder.d20_31(instr),
        }


class Core:

    def __init__(self, glblmem: Memory, bus: Bus) -> None:
        self.gpr: dict[int, MintDBWbounded] = {
            0: MintDBWbounded(0, XLEN),
            1: MintDBWbounded(0, XLEN),
            2: MintDBWbounded(0, XLEN),
            3: MintDBWbounded(0, XLEN),
            4: MintDBWbounded(0, XLEN),
            5: MintDBWbounded(0, XLEN),
            6: MintDBWbounded(0, XLEN),
            7: MintDBWbounded(0, XLEN),
            8: MintDBWbounded(0, XLEN),
            9: MintDBWbounded(0, XLEN),
            10: MintDBWbounded(0, XLEN),
            11: MintDBWbounded(0, XLEN),
            12: MintDBWbounded(0, XLEN),
            13: MintDBWbounded(0, XLEN),
            14: MintDBWbounded(0, XLEN),
            15: MintDBWbounded(0, XLEN),
            16: MintDBWbounded(0, XLEN),
            17: MintDBWbounded(0, XLEN),
            18: MintDBWbounded(0, XLEN),
            19: MintDBWbounded(0, XLEN),
            20: MintDBWbounded(0, XLEN),
            21: MintDBWbounded(0, XLEN),
            22: MintDBWbounded(0, XLEN),
            23: MintDBWbounded(0, XLEN),
            24: MintDBWbounded(0, XLEN),
            25: MintDBWbounded(0, XLEN),
            26: MintDBWbounded(0, XLEN),
            27: MintDBWbounded(0, XLEN),
            28: MintDBWbounded(0, XLEN),
            29: MintDBWbounded(0, XLEN),
            30: MintDBWbounded(0, XLEN),
            31: MintDBWbounded(0, XLEN),
        }

        # self.memsh: Memory = glblmem
        self.bus: Bus = bus
        self.pc: int = RESET_VECTOR

    def step(self) -> None:
        instruction = self.load_instruction()

        self.gpr[0].v = 0
        decoded: DECODEVALS = InstructionDecoder.decode_simples(instruction)

        match decoded["opcode"]:
            case 0b0110111:  # lui
                rd = self.gpr[decoded["7:11"]]
                rd = rd._child(decoded["12:31"] << 12)

            case 0b0010111:  # auipc
                rd = self.gpr[decoded["7:11"]]
                rd = rd._child((self.pc + decoded["12:31"]) << 12)

            case 0b1101111:  # jal
                rd = self.gpr[decoded["7:11"]]
                # complex deconstruction
                s12_19 = InstructionDecoder.gimme(instruction, 12, 19)
                s11 = InstructionDecoder.gimme(instruction, 20, 20)
                s1_10 = InstructionDecoder.gimme(instruction, 21, 30)
                s20 = InstructionDecoder.gimme(instruction, 31, 31)

                recons = MintDBWbounded(((s20 << 20) + (s12_19 << 12) +
                                         (s11 << 11) + s1_10) << 1, 20)

                rd = rd._child(self.pc + 2)
                self.pc += ALU.sign(recons)

            case 0b1100111:  # jalr
                rd = self.gpr[decoded["7:11"]]
                rs1 = int(self.gpr[decoded["15:19"]])
                imm = decoded["20:31"]

                rd = rd._child(self.pc + 2)
                self.pc = rs1 + imm

            case 0b1100011:  # branch
                rd = self.gpr[decoded["7:11"]]
                _rs1: MintDBWbounded = self.gpr[decoded["15:19"]]
                rs2 = self.gpr[decoded["20:24"]]

                s11 = InstructionDecoder.gimme(instruction, 7, 7)
                s1_4 = InstructionDecoder.gimme(instruction, 8, 11)
                s5_10 = InstructionDecoder.gimme(instruction, 25, 30)
                s12 = InstructionDecoder.gimme(instruction, 31, 31)

                recons = MintDBWbounded(((s12 << 12) + (s11 << 11) +
                                         (s5_10 << 5) + s1_4) << 1, 12)

                fn3 = decoded["12:14"]

                match fn3:
                    case 0b000:  # BEQ
                        self.pc += ALU.sign(recons) if _rs1 == rs2 else 0

                    case 0b001:  # BNE
                        self.pc += ALU.sign(recons) if _rs1 != rs2 else 0

                    case 0b010:  # BLT
                        self.pc += ALU.sign(recons) if ALU.sign(_rs1
                                                               ) < ALU.sign(rs2) else 0

                    case 0b011:  # BLTU
                        self.pc += ALU.sign(recons) if _rs1.v < rs2.v else 0

                    case 0b011:  # BGE
                        self.pc += ALU.sign(recons
                                           ) if ALU.sign(_rs1) >= ALU.sign(rs2) else 0

                    case 0b011:  # BGEU
                        self.pc += ALU.sign(recons) if _rs1.v >= rs2.v else 0

            case 0b0000011:  # load
                rd = self.gpr[decoded["7:11"]]
                rs1: int = ALU.usign(self.gpr[decoded["15:19"]])  # type: ignore

                imm = decoded["20:31"]

                fn3 = decoded["12:14"]

                rd.v = ALU.sign(MintDBWbounded(self.bus.load(rs1 + imm, fn3), XLEN))

            case 0b0100011:  # store
                rs1 = int(ALU.usign(self.gpr[decoded["15:19"]]))  # base addr
                rs2 = self.gpr[decoded["20:24"]]  # src

                imm = (decoded["25:31"] << 5) + decoded["7:11"]

                fn3 = decoded["12:14"]

                print(rs1, rs2, imm, fn3)

                self.bus.store(rs1 + imm, rs2.v)

                #match fn3:
                #   case 0b000:
                #      self.memsh.sviab(rs1 + imm, rs2.v & 0xff)


#                    case 0b001:
#                       self.memsh.store_half(rs1 + imm, rs2.v & 0xffff)
#
#                   case 0b010:
#                      self.memsh.store_word(rs1 + imm, rs2.v & 0xffff_ffff)

            case 0b0010011:  # imm,rs1->rd
                imm = ALU.sign(MintDBWbounded(decoded["20:31"], 12))
                rd = self.gpr[decoded["7:11"]]

                fn3 = decoded["12:14"]
                rs1 = int(self.gpr[decoded["15:19"]])

                print(imm, rd, rs1, fn3)

                if fn3 == 0b000:
                    rd = rd._child(rs1 + imm)
                elif fn3 == 0b010:
                    # slti
                    pass

                elif fn3 == 0b011:
                    # sltiu
                    pass

                elif fn3 == 0b100:
                    # xori
                    rd = rd._child(rs1 ^ imm)

                elif fn3 == 0b110:
                    # ori
                    rd = rd._child(rs1 | imm)

                elif fn3 == 0b111:
                    # andi
                    rd = rd._child(rs1 & imm)

                elif fn3 == 0b001:
                    # slli
                    shamt = decoded["20:24"]
                    rd = rd._child(ALU.lshift(rs1, shamt))

                elif fn3 == 0b101:
                    # srli / srai

                    fn7 = decoded["25:31"]
                    shamt = decoded["20:24"]
                    if fn7 == 0b0000_000:
                        # srli
                        rd.v = ALU.rshift(rs1, shamt)

                    else:
                        # srai
                        rd.v = ALU.rshift(rs1, shamt)

            case 0b0110011:  # rs2,rs1->rd
                rd = self.gpr[decoded["7:11"]]

                fn3 = decoded["12:14"]
                rs1 = int(self.gpr[decoded["15:19"]])
                rs2 = self.gpr[decoded["20:24"]]
                fn7 = decoded["25:31"]

                match fn3:
                    case 0b000:
                        if fn7 == 0b0:
                            rd = rd._child(rs1 + rs2.v)

                        if fn7 == 0b010000:
                            rd = rd._child(rs1 - rs2.v)

                    case 0b001:
                        rd._childself(rs1 << (rs2.v & 0x1f))

                    case 0b010:
                        rd.v = 1 if ALU.sign(MintDBWbounded(rs1, XLEN)
                                            ) < ALU.sign(rs2) else 0

                    case 0b011:
                        rd.v = 1 if rs1 < rs2.v else 0

                    case 0b100:
                        rd._childself(rs1 ^ rs2.v)

                    case 0b101:  # SRL SRA
                        if fn7 == 0b0:
                            rd._childself(rs1 >> (rs2.v & 0x1f))
                        else:
                            rd._childself(rs1 >> (rs2.v & 0x1f))

                    case 110:  # or
                        rd._childself(rs1 | rs2.v)

                    case 111:  # and
                        rd._childself(rs1 & rs2.v)

            case 0b0001111:
                pass  # this is a single-core single-thread system so it doesnt matter

            case 0xc0ffee:
                fn12 = decoded["20:31"]

                if fn12 == 0b0000_0000_0000:
                    pass
                else:
                    pass

            case _:
                raise Trap(TCause.ILLEGAL_INSTRUCTION)

        self.pc += 4

    def load_instruction(self) -> int:
        try:
            instruction = self.bus.load(self.pc, 0b010)
        except:
            raise Trap(TCause.INSTRUCTION_ADDRESS_MISALIGNED)

        print(instruction.to_bytes(length=4, byteorder="little"))

        return instruction
