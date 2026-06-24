use crate::devices::Bus;

const RESET_VECTOR: u32 = 0x1000;

mod decoder {
    pub fn fetch(from: u32, lo: u32, hi: u32) -> i32 {
        // ヾ(＾∇＾)
        // println!("fetch");
        ((from >> lo) & ((1u64 << (hi - lo + 1)) - 1) as u32) as i32
    }
}

fn sextend(from: i32, bits: u32) -> i32 {
    let into: i32 = ((from << (32 - bits)) as i32) >> (32 - bits);

    println!("{:#b}", into);
    into
}

pub struct Core {
    general_registers: Vec<i32>,
    pc: u32,
    bus: Bus,
    trace: Vec<String>,
}

impl Core {
    pub fn new(bus: Bus) -> Core {
        let mut gr: Vec<i32> = Vec::new();
        for _ in 0..32 {
            gr.push(0)
        }

        Core {
            general_registers: gr,
            pc: 0,
            bus,
            trace: Vec::new(),
        }
    }

    pub fn step(&mut self) {
        let comp_pc = self.pc;
        let instruction = self.fetch();
        self.pc += 4;

        self.general_registers[0] = 0;

        //println!("{:#?}", self.trace);
        //println!("{:?}", self.general_registers);
        match decoder::fetch(instruction, 0, 6) {
            0b0110111 => {
                self.general_registers[decoder::fetch(instruction, 7, 11) as usize] =
                    (sextend(decoder::fetch(instruction, 12, 31), 20) << 12) as i32;

                // traceinst = f"lui x{decoded['7:11']}, {decoded['12:31']}"
                self.trace.push(format!(
                    "[{:#06x}]  lui x{}, {}",
                    comp_pc,
                    decoder::fetch(instruction, 7, 11),
                    decoder::fetch(instruction, 12, 31)
                ));
            }
            0b0010111 => {
                self.general_registers[decoder::fetch(instruction, 7, 11) as usize] = // rd
                    (self.pc as i32 + (decoder::fetch(instruction, 12, 31) << 12)) as i32;

                self.trace.push(format!(
                    "[{:#06x}]  auipc x{}, {}",
                    comp_pc,
                    decoder::fetch(instruction, 7, 11),
                    decoder::fetch(instruction, 12, 31)
                ));
            }
            0b1101111 => {
                // jal
                let s12_19 = decoder::fetch(instruction, 12, 19);
                let s11 = decoder::fetch(instruction, 20, 20);
                let s1_10 = decoder::fetch(instruction, 21, 30);
                let s20 = decoder::fetch(instruction, 31, 31);

                let recons = sextend(
                    (s20 << 20) | (s12_19 << 12) | (s11 << 11) | (s1_10 << 1),
                    20,
                );
                self.pc = (comp_pc as i32 + recons) as u32;
                self.general_registers[decoder::fetch(instruction, 7, 11) as usize] = // rd
                    (comp_pc + 4) as i32;

                self.trace.push(format!(
                    "[{:#06x}]  jal x{}, {}",
                    comp_pc,
                    decoder::fetch(instruction, 7, 11),
                    recons
                ));

                println!("{:#?}", self.trace);
            }
            0b1100111 => {
                // jalr
                self.pc = (self.general_registers[decoder::fetch(instruction, 15, 19) as usize]
                    + decoder::fetch(instruction, 20, 31)) as u32; // rs1 + imm (~1 ??)
                self.general_registers[decoder::fetch(instruction, 7, 11) as usize] =
                    comp_pc as i32 + 4;

                self.trace.push(format!(
                    "[{:#06x}]  jalr x{}, {}(x{})",
                    comp_pc,
                    decoder::fetch(instruction, 7, 11),
                    decoder::fetch(instruction, 20, 31),
                    decoder::fetch(instruction, 15, 19),
                ));
                println!("{:#?}", self.trace);
            }
            0b1100011 => {
                // branch
                let rs1: i32 = self.general_registers[decoder::fetch(instruction, 15, 19) as usize];
                let rs2: i32 = self.general_registers[decoder::fetch(instruction, 20, 24) as usize];

                let s11: u32 = decoder::fetch(instruction, 7, 7) as u32;
                let s1_4: u32 = decoder::fetch(instruction, 8, 11) as u32;
                let s5_10: u32 = decoder::fetch(instruction, 25, 30) as u32;
                let s12: u32 = decoder::fetch(instruction, 31, 31) as u32;

                let recons: i32 = sextend(
                    ((s12 << 12) | (s11 << 11) | (s5_10 << 5) | (s1_4 << 1)) as i32,
                    13,
                );

                let fn3: u32 = decoder::fetch(instruction, 12, 14) as u32;

                if match fn3 {
                    0b000 => rs1 == rs2,                   // beq
                    0b001 => rs1 != rs2,                   // bne
                    0b100 => rs1 < rs2,                    // blt
                    0b110 => (rs1 as u32) < (rs2 as u32),  // bltu
                    0b101 => rs1 >= rs2,                   // bge
                    0b111 => (rs1 as u32) >= (rs2 as u32), // bgeu
                    _ => panic!("fn3 was something unsupported!"),
                } {
                    self.pc = (comp_pc as i32 + recons) as u32
                } /* look at how much more efficient */

                self.trace.push(format!(
                    "[{:#06x}]  BNC x{}, x{}, {} # {} ?= {} # {:#12x}",
                    comp_pc,
                    decoder::fetch(instruction, 15, 19),
                    decoder::fetch(instruction, 20, 24),
                    recons,
                    rs1,
                    rs2,
                    instruction
                ));
                println!("{:#?}", self.trace);
            }

            0b0000011 => {
                // load
                let fn3 = decoder::fetch(instruction, 12, 14);
                /*if fn3 > 3 {
                    // unsigned vers
                    self.general_registers[decoder::fetch(instruction, 7, 11) as usize] =
                        self.bus.load(
                            (self.general_registers[decoder::fetch(instruction, 15, 19) as usize]
                                + decoder::fetch(instruction, 20, 31))
                                as u32,
                            fn3.try_into().unwrap(),
                        );
                    return;
                }*/

                self.general_registers[decoder::fetch(instruction, 7, 11) as usize] =
                    self.bus.load(
                        (self.general_registers[decoder::fetch(instruction, 15, 19) as usize]
                            + sextend(decoder::fetch(instruction, 20, 31), 12))
                            as u32,
                        fn3.try_into().unwrap(),
                    );

                self.trace.push(format!(
                    "[{:#06x}]  l{} x{}, {}(x{})",
                    comp_pc,
                    match fn3 {
                        0b000 => "b",
                        0b001 => "h",
                        0b010 => "w",
                        0b100 => "bu",
                        0b101 => "hu",
                        _ => panic!("load fn3 error while tracing"),
                    },
                    decoder::fetch(instruction, 7, 11),
                    decoder::fetch(instruction, 20, 31),
                    decoder::fetch(instruction, 15, 19),
                ));
            }

            0b0100011 => {
                // Store
                self.bus.store(
                    (self.general_registers[decoder::fetch(instruction, 15, 19) as usize]
                        + sextend(
                            (decoder::fetch(instruction, 25, 31) << 5)
                                | decoder::fetch(instruction, 7, 11),
                            12,
                        )) as u32,
                    self.general_registers[decoder::fetch(instruction, 20, 24) as usize],
                    (decoder::fetch(instruction, 12, 14) as u32)
                        .try_into()
                        .unwrap(),
                );

                self.trace.push(format!(
                    "[{:#06x}]  s{} x{}, {}(x{})",
                    comp_pc,
                    match (decoder::fetch(instruction, 12, 14) as u32)
                        .try_into()
                        .unwrap()
                    {
                        0b000 => "b",
                        0b001 => "h",
                        0b010 => "w",
                        _ => panic!("store fn3 error while tracing"),
                    },
                    decoder::fetch(instruction, 20, 24),
                    (decoder::fetch(instruction, 25, 31) << 5) | decoder::fetch(instruction, 7, 11),
                    decoder::fetch(instruction, 15, 19),
                ));
            }

            0b0010011 => {
                println!("{:#5x}", decoder::fetch(instruction, 20, 31));
                let imm: i32 = sextend(decoder::fetch(instruction, 20, 31), 12);

                println!("{}", imm);
                let rs1: i32 = self.general_registers[decoder::fetch(instruction, 15, 19) as usize];

                self.general_registers[decoder::fetch(instruction, 7, 11) as usize] =
                    match decoder::fetch(instruction, 12, 14) {
                        0b000 => rs1 + imm, // addi
                        0b010 => panic!("slti not impl"),
                        0b011 => panic!("sltiu not impl"),
                        0b100 => rs1 ^ imm, // xori
                        0b110 => rs1 | imm, // ori
                        0b111 => rs1 & imm, // andi
                        0b001 => rs1 << decoder::fetch(instruction, 20, 24), // slli (shamt)
                        0b101 => {
                            // srai / srli
                            let shamt = decoder::fetch(instruction, 20, 24); // who cares about sextend
                            match decoder::fetch(instruction, 25, 31) {
                                0b000_0000 => (rs1 as u32 >> shamt) as i32,
                                _ => rs1 >> shamt,
                            }
                        }
                        _ => panic!("unknown fn7 in i type!"),
                    };

                self.trace.push(format!(
                    "[{:#06x}]  {} x{}, {}(x{}) # {:#12x}",
                    comp_pc,
                    match (decoder::fetch(instruction, 12, 14) as u32)
                        .try_into()
                        .unwrap()
                    {
                        0b000 => "addi",
                        0b010 => "slti",
                        0b011 => "sltiu",
                        0b100 => "xori",
                        0b110 => "ori",
                        0b111 => "andi",
                        _ => panic!("itype fn3 error while tracing"),
                    },
                    decoder::fetch(instruction, 7, 11),
                    imm,
                    decoder::fetch(instruction, 15, 19),
                    instruction
                ));
            }

            0b0110011 => {
                // rtype
                let rs1: i32 = self.general_registers[decoder::fetch(instruction, 15, 19) as usize];
                let rs2: i32 = self.general_registers[decoder::fetch(instruction, 20, 24) as usize];

                self.general_registers[decoder::fetch(instruction, 7, 11) as usize] =
                    match decoder::fetch(instruction, 12, 14) {
                        0b000 => {
                            // add/sub
                            match decoder::fetch(instruction, 25, 31) {
                                0b000_0000 => rs1 + rs2,
                                0b010_0000 => rs1 - rs2,
                                _ => panic!("invalid fn7 in add/sub!"),
                            }
                        }

                        0b001 => rs1 << (rs2 & 0x1f), // sll
                        0b010 => {
                            if rs1 < rs2 {
                                1
                            } else {
                                0
                            }
                        } // slt
                        0b011 => {
                            if (rs1 as u32) < (rs2 as u32) {
                                1
                            } else {
                                0
                            }
                        } // sltu
                        0b100 => rs1 ^ rs2,           // xor
                        0b101 => {
                            // srl/sra
                            match decoder::fetch(instruction, 25, 31) {
                                0b000_0000 => (rs1 as u32 >> (rs2 & 0x1f)) as i32,
                                0b010_0000 => rs1 >> rs2,
                                _ => panic!("invalid fn7 in srl/sra"),
                            }
                        }
                        0b110 => rs1 | rs2, // or
                        0b111 => rs1 & rs2, // and

                        _ => panic!("unknown fn3 in rtype!"),
                    }
            } // wow! thats all the major rv31i opcodes.

            0b0001111 => return, // fence but we dont need to worry. (single-thread)
            0b1110011 => {
                // ebreak/ecall
                match decoder::fetch(instruction, 20, 31) {
                    0b0000_0000_0000 => panic!("ebreak"),
                    _ => panic!("ecall or other"),
                }
            }

            _ => panic!(
                "ILLEGAL {} {}",
                instruction,
                decoder::fetch(instruction, 0, 6)
            ), // Trap(TCause.ILLEGAL_INSTRUCTION)
        }
    }

    fn fetch(&self) -> u32 {
        let fetched = self.bus.load(self.pc as u32, 0b010) as u32;

        println!("fetched {:#012x} from {:#04x}", fetched, self.pc);

        fetched

        // Trap(TCause.INSTRUCTION_ADDRESS_MISALIGNED)

        // log(instruction.to_bytes(length=4, byteorder="little"))
    }
}
/*

            case _:
                raise Trap(TCause.ILLEGAL_INSTRUCTION)

        self.trace.append((cpc, instruction, traceinst))

        if self.trace.__len__() > TRACE_MAX_LENGTH:
            self.trace.pop(0)

    def load_instruction(self) -> int:
        try:
            instruction = self.bus.load(self.pc, 0b010)
        except Exception:
            raise Trap(TCause.INSTRUCTION_ADDRESS_MISALIGNED)

        log(instruction.to_bytes(length=4, byteorder="little"))
        return instruction

    /*def dump(self) -> None:
        # rdumpread: str = "\n".join(f"{k} : {hex(int(v))}"
        #                            for k, v in self.gpr.items())
        # print(f"register dump: \n{rdumpread}\n")
        # print(f"instr pointer: {self.pc}")
        # print("-" * 40, end="\n\n")  # throwback
        # print(self.bus.devices[0].device.data[:256])

        rdump: str = ""
        for i, kv in enumerate(self.gpr.items()):
            if i % 4 == 0:
                rdump += "\n"

            k, v = kv
            rdump += f"[x{k} {' ' * (3 - len(f'x{k}'))}: 0x{hex(int(v)).removeprefix('0x').rjust(4, '0')}] "

        print(rdump)
        print(
            f"[pc  : 0x{hex(int(self.pc)).removeprefix('0x').rjust(4, '0')}]")

        print("\n<", "=" * 60, end=">\n\n", sep="")

        trace: str = "trace:\n"

        ppc: int = RESET_VECTOR
        for pc, heks, instruction in self.trace:
            # instruction = "instruction goes here"
            trace += (
                '\n' if pc < ppc or pc - 4 != ppc else ''
            ) + f"[0x{hex(int(pc)).removeprefix('0x').rjust(4, '0')}]  {instruction} \
{' ' * (24 - instruction.__len__())}# 0x{hex(int(heks)).removeprefix('0x').rjust(8, '0')}\n"

            ppc = pc

        print(trace)*/


*/

/*# mypy: disable-error-code="no-redef"

from typing import Self, Literal, TypeAlias
from devices import Bus
from risclib import Trap, TCause, XLEN

DEBUG = False
TRACE_MAX_LENGTH = 64

IALIGN: int = 32
RESET_VECTOR = 0x00001000

DECODEVALS: TypeAlias = dict[Literal["opcode", "7:11", "12:14", "15:19",
                                     "20:24", "25:31", "12:31", "20:31"], int]


def log(*values: object) -> None:
    print(" ".join([str(v) for v in values])) if DEBUG else None


class MintDBWbounded:

    def __init__(self, fromint: int, xlen: int) -> None:
        self.v = self._bounds(fromint, xlen)
        self.xlen = xlen

    def _coerce(self, other: Self | int) -> int:
        return other.v if isinstance(other, MintDBWbounded) else other

    def _child(self, _from: int) -> Self:
        return type(self)(self._bounds(_from, self.xlen), self.xlen)

    def _childself(self, _from: int) -> None:
        self.v = self._bounds(_from, self.xlen)

    def __add__(self, other: Self | int) -> Self:
        return self._child(self.v + self._coerce(other))

    def __sub__(self, other: Self | int) -> Self:
        return self._child(self.v - self._coerce(other))

    def __mul__(self, other: Self | int) -> Self:
        return self._child(self.v * self._coerce(other))

    def __truediv__(self, other: Self | int) -> Self:
        return self._child(self.v // self._coerce(other))

    def __iadd__(self, other: Self | int) -> Self:
        self.v += self._coerce(other)
        return self

    def __isub__(self, other: Self | int) -> Self:
        self.v -= self._coerce(other)
        return self

    @staticmethod
    def _bounds(tobe: int, xlen: int) -> int:
        wrapped = ((tobe + 2**(xlen - 1)) % 2**xlen) - 2**(xlen - 1)

        return wrapped

    def _selfbounds(self):
        self.v = (
            (self.v + 2**(self.xlen - 1)) % 2**self.xlen) - 2**(self.xlen - 1)

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
        # msktty = 1 << (val.xlen - 1)
        # return (int(val) ^ msktty) - msktty
        return ((val.v + 2**(val.xlen - 1)) % 2**val.xlen) - 2**(val.xlen - 1)

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
        return InstructionDecoder.gimme(instr, 20, 31)

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

    def dump(self) -> None:
        # rdumpread: str = "\n".join(f"{k} : {hex(int(v))}"
        #                            for k, v in self.gpr.items())
        # print(f"register dump: \n{rdumpread}\n")
        # print(f"instr pointer: {self.pc}")
        # print("-" * 40, end="\n\n")  # throwback
        # print(self.bus.devices[0].device.data[:256])

        rdump: str = ""
        for i, kv in enumerate(self.gpr.items()):
            if i % 4 == 0:
                rdump += "\n"

            k, v = kv
            rdump += f"[x{k} {' ' * (3 - len(f'x{k}'))}: 0x{hex(int(v)).removeprefix('0x').rjust(4, '0')}] "

        print(rdump)
        print(
            f"[pc  : 0x{hex(int(self.pc)).removeprefix('0x').rjust(4, '0')}]")

        print("\n<", "=" * 60, end=">\n\n", sep="")

        trace: str = "trace:\n"

        ppc: int = RESET_VECTOR
        for pc, heks, instruction in self.trace:
            # instruction = "instruction goes here"
            trace += (
                '\n' if pc < ppc or pc - 4 != ppc else ''
            ) + f"[0x{hex(int(pc)).removeprefix('0x').rjust(4, '0')}]  {instruction} \
{' ' * (24 - instruction.__len__())}# 0x{hex(int(heks)).removeprefix('0x').rjust(8, '0')}\n"

            ppc = pc

        print(trace)

    def __init__(self, bus: Bus) -> None:
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

        self.trace: list[tuple[int, int, str]] = []  # pc, heks, decoded

    def step(self) -> None:
        cpc: int = self.pc
        instruction = self.load_instruction()
        self.pc += 4

        self.gpr[0].v = 0
        decoded: DECODEVALS = InstructionDecoder.decode_simples(instruction)

        traceinst: str = "unknown"

        log(f"instruction @ {hex(self.pc)} {hex(instruction)} {bin(instruction)}"
            )

        jump = False

        match decoded["opcode"]:
            case 0b0110111:  # lui
                log("lui")
                rd = self.gpr[decoded["7:11"]]
                rd._childself(decoded["12:31"] << 12)

                traceinst = f"lui x{decoded['7:11']}, {decoded['12:31']}"

            case 0b0010111:  # auipc
                log("auipc")

                rd = self.gpr[decoded["7:11"]]
                rd._childself((self.pc + decoded["12:31"]) << 12)

                traceinst = f"auipc x{decoded['7:11']}, {decoded['12:31']}"

            case 0b1101111:  # jal
                log("jal")

                rd = self.gpr[decoded["7:11"]]
                # complex deconstruction
                s12_19 = InstructionDecoder.gimme(instruction, 12, 19)
                s11 = InstructionDecoder.gimme(instruction, 20, 20)
                s1_10 = InstructionDecoder.gimme(instruction, 21, 30)
                s20 = InstructionDecoder.gimme(instruction, 31, 31)

                recons = MintDBWbounded(
                    (s20 << 20) | (s12_19 << 12) | (s11 << 11) | (s1_10 << 1),
                    21)

                rd._childself(cpc + 4)
                self.pc = cpc + ALU.sign(recons)

                traceinst = f"jal x{decoded['7:11']}, {ALU.sign(recons)}"

                jump = True

            case 0b1100111:  # jalr
                log("jalr")

                rd = self.gpr[decoded["7:11"]]
                rs1 = self.gpr[decoded["15:19"]].v
                imm = decoded["20:31"]

                rd._childself(cpc + 4)
                self.pc = (rs1 + imm) & ~1

                traceinst = f"jalr x{decoded['7:11']}, {imm}(x{decoded['15:19']})"

            case 0b1100011:  # branch
                log("branch")

                # imm = decoded["7:11"]
                _rs1: MintDBWbounded = self.gpr[decoded["15:19"]]
                rs2 = self.gpr[decoded["20:24"]]

                s11 = InstructionDecoder.gimme(instruction, 7, 7)
                s1_4 = InstructionDecoder.gimme(instruction, 8, 11)
                s5_10 = InstructionDecoder.gimme(instruction, 25, 30)
                s12 = InstructionDecoder.gimme(instruction, 31, 31)

                recons = MintDBWbounded(
                    (s12 << 12) | (s11 << 11) | (s5_10 << 5) | (s1_4 << 1), 13)

                fn3 = decoded["12:14"]

                log(_rs1, rs2, ALU.sign(recons), recons, fn3, bin(recons.v),
                    (recons.v ^ (1 << 12)) - (1 << 12))

                match fn3:
                    case 0b000:  # BEQ
                        jump = True if ALU.sign(_rs1) == ALU.sign(
                            rs2) else False

                        traceinst = f"beq x{decoded['15:19']}, x{decoded['20:24']}, {recons}"

                    case 0b001:  # BNE
                        jump = True if ALU.sign(_rs1) != ALU.sign(
                            rs2) else False

                        traceinst = f"bne x{decoded['15:19']}, x{decoded['20:24']}, {recons}"

                    case 0b100:  # BLT
                        jump = True if ALU.sign(_rs1) < ALU.sign(
                            rs2) else False

                        traceinst = f"blt x{decoded['15:19']}, x{decoded['20:24']}, {recons}"

                    case 0b110:  # BLTU
                        jump = True if ALU.usign(_rs1) < ALU.usign(
                            rs2) else False

                        traceinst = f"bltu x{decoded['15:19']}, x{decoded['20:24']}, {recons}"

                    case 0b101:  # BGE
                        jump = True if ALU.sign(_rs1) >= ALU.sign(
                            rs2) else False

                        traceinst = f"bge x{decoded['15:19']}, x{decoded['20:24']}, {recons}"

                    case 0b111:  # BGEU
                        jump = True if ALU.usign(_rs1) >= ALU.usign(
                            rs2) else False

                        traceinst = f"bgeu x{decoded['15:19']}, x{decoded['20:24']}, {recons}"

                if jump:
                    log("jump")
                    self.pc = cpc + ALU.sign(recons)
            case 0b0000011:  # load
                log("load")

                rd = self.gpr[decoded["7:11"]]
                rs1: int = ALU.usign(
                    self.gpr[decoded["15:19"]])  # type: ignore

                imm = decoded["20:31"]

                fn3 = decoded["12:14"]

                rd.v = ALU.sign(
                    MintDBWbounded(self.bus.load(rs1 + imm, fn3), XLEN))

                traceinst = f"{['lb','lh','lw','lbu','lhu'][fn3]} x{decoded['7:11']}, {imm}(x{decoded['15:19']})"

            case 0b0100011:  # store
                log("store")

                rs1 = int(ALU.usign(self.gpr[decoded["15:19"]]))  # base addr
                rs2 = self.gpr[decoded["20:24"]]  # src

                imm = (decoded["25:31"] << 5) + decoded["7:11"]

                fn3 = decoded["12:14"]

                # log(rs1, rs2, imm, fn3)

                self.bus.store(rs1 + imm, rs2.v, fn3)

                # match fn3:
                #   case 0b000:
                #      self.memsh.sviab(rs1 + imm, rs2.v & 0xff)

                traceinst = f"{['sb','sh','sw'][fn3]} x{decoded['20:24']}, {imm}(x{decoded['15:19']})"


#                    case 0b001:
#                       self.memsh.store_half(rs1 + imm, rs2.v & 0xffff)
#
#                   case 0b010:
#                      self.memsh.store_word(rs1 + imm, rs2.v & 0xffff_ffff)

            case 0b0010011:  # imm,rs1->rd
                log("I-type")

                imma = MintDBWbounded(decoded["20:31"], 12)
                imm = ALU.sign(imma)
                rd = self.gpr[decoded["7:11"]]

                fn3 = decoded["12:14"]
                rs1 = int(self.gpr[decoded["15:19"]])

                # log("immediate to rd")
                # log(imm, imma)

                if fn3 == 0b000:
                    # log("addi")
                    rd._childself(rs1 + imm)
                    traceinst = f"addi x{decoded['7:11']}, {imm}(x{decoded['15:19']})"

                elif fn3 == 0b010:
                    # slti
                    pass

                elif fn3 == 0b011:
                    # sltiu
                    pass

                elif fn3 == 0b100:
                    # xori
                    rd._childself(rs1 ^ imm)
                    traceinst = f"xori x{decoded['7:11']}, {imm}(x{decoded['15:19']})"

                elif fn3 == 0b110:
                    # ori
                    rd._childself(rs1 | imm)
                    traceinst = f"ori x{decoded['7:11']}, {imm}(x{decoded['15:19']})"

                elif fn3 == 0b111:
                    # andi
                    rd._childself(rs1 & imm)
                    traceinst = f"andi x{decoded['7:11']}, {imm}(x{decoded['15:19']})"

                elif fn3 == 0b001:
                    # slli
                    shamt = decoded["20:24"]
                    rd._childself(ALU.lshift(rs1, shamt))
                    traceinst = f"slli x{decoded['7:11']}, {imm}(x{decoded['15:19']})"

                elif fn3 == 0b101:
                    # srli / srai

                    fn7 = decoded["25:31"]
                    shamt = decoded["20:24"]
                    if fn7 == 0b0000_000:
                        # srli
                        rd.v = ALU.rshift(rs1, shamt)
                        traceinst = f"srli x{decoded['7:11']}, {imm}(x{decoded['15:19']})"

                    else:
                        # srai
                        rd.v = ALU.rshift(rs1, shamt)
                        traceinst = f"srai x{decoded['7:11']}, {imm}(x{decoded['15:19']})"

            case 0b0110011:  # rs2,rs1->rd
                log("rtype")

                rd = self.gpr[decoded["7:11"]]

                fn3 = decoded["12:14"]
                rs1 = int(self.gpr[decoded["15:19"]])
                rs2 = self.gpr[decoded["20:24"]]
                fn7 = decoded["25:31"]

                match fn3:
                    case 0b000:
                        if fn7 == 0b0:
                            rd._childself(rs1 + rs2.v)

                        if fn7 == 0b010000:
                            rd._childself(rs1 - rs2.v)

                    case 0b001:
                        rd._childself(rs1 << (rs2.v & 0x1f))

                    case 0b010:
                        rd.v = 1 if ALU.sign(MintDBWbounded(
                            rs1, XLEN)) < ALU.sign(rs2) else 0

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

                # print(self.gpr)

            case 0b0001111:
                pass  # this is a single-core single-thread system so it doesnt matter

            case 0b1110011:  # 0xc0ffee
                fn12 = decoded["20:31"]

                if fn12 == 0b0000_0000_0000:
                    print("execution passed to debugger")

                    match self.gpr[31]:  # x31
                        case 0x0:
                            # general
                            self.dump()

                        case 0x1:
                            # dump region (x29, x30)
                            rs = self.gpr[29].v  # x29
                            rz = self.gpr[30].v  # x30, Region siZe

                            with open("dump.dump", "wb") as dump:
                                dump.write(
                                    self.bus.devices[0].device.data[rs:rs + rz]
                                )  # could lead to arbitrary writing if 0 isnt gpm

                            print("execution halted: ram dumped")

                        case 0x2:
                            # shutdown
                            pass

                        case 0x3:
                            # dump memory & core
                            # dump region (x29, x30)
                            rs = self.gpr[29].v  # x29
                            rz = self.gpr[30].v  # x30, Region siZe

                            with open("dump.dump", "wb") as dump:
                                dump.write(
                                    self.bus.devices[0].device.data[rs:rs + rz]
                                )  # could lead to arbitrary writing if 0 isnt gpm

                            print("execution halted: ram dumped")
                            self.dump()

                        case 0x4:
                            print("execution halted: bus trace")
                            print(self.bus.trace)

                        case 0x5:
                            pass

                        case 0x6:
                            pass

                        case _:
                            self.dump()

                    # self.dump()

                    exit()
                else:
                    pass

            case _:
                raise Trap(TCause.ILLEGAL_INSTRUCTION)

        self.trace.append((cpc, instruction, traceinst))

        if self.trace.__len__() > TRACE_MAX_LENGTH:
            self.trace.pop(0)

    def load_instruction(self) -> int:
        try:
            instruction = self.bus.load(self.pc, 0b010)
        except Exception:
            raise Trap(TCause.INSTRUCTION_ADDRESS_MISALIGNED)

        log(instruction.to_bytes(length=4, byteorder="little"))
        return instruction
*/
