use crate::devices::Bus;
use crate::devices::XLEN;
use crate::risclib::Trap;

pub const RESET_VECTOR: u32 = 0x00001000;
pub const IALIGN: usize = 32;

mod decoder {
    pub fn fetch(from: u32, lo: u32, hi: u32) -> i32 {
        // ヾ(＾∇＾)
        // println!("fetch");
        ((from >> lo) & ((1u64 << (hi - lo + 1)) - 1) as u32) as i32
    }
}

fn sextend(from: i32, bits: u32) -> i32 {
    let into: i32 = ((from << (32 - bits)) as i32) >> (32 - bits);

    // println!("{:#b}", into);
    into
}

pub struct Core {
    pub general_registers: Vec<i32>,
    pub pc: u32,
    pub bus: Bus,
    pub trace: Vec<String>,
}

impl Core {
    pub fn new(bus: Bus) -> Core {
        let mut gr: Vec<i32> = Vec::new();
        for _ in 0..32 {
            gr.push(0)
        }

        Core {
            general_registers: gr,
            pc: RESET_VECTOR,
            bus,
            trace: Vec::new(),
        }
    }

    pub fn step(&mut self) -> Result<(), Trap> {
        let comp_pc: u32 = self.pc;
        let instruction: u32 = self.fetch()?;
        self.pc += 4;

        self.general_registers[0] = 0;

        //println!("{:#?}", self.trace);
        //println!("{:?}", self.general_registers);
        match decoder::fetch(instruction, 0, 6) {
            0b0110111 => {
                self.general_registers[decoder::fetch(instruction, 7, 11) as usize] =
                    (sextend(decoder::fetch(instruction, 12, 31), 20) << 12) as i32;

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
                if recons % 4 != 0 {
                    /* The JAL and JALR instructions will generate an instruction-address-misaligned exception if the target
                    address is not aligned to a four-byte boundary. */
                    return Err(Trap::INSTRUCTION_ADDRESS_MISALIGNED);
                }
                self.pc = (comp_pc as i32 + recons) as u32;
                self.general_registers[decoder::fetch(instruction, 7, 11) as usize] = // rd
                    (comp_pc + 4) as i32;

                self.trace.push(format!(
                    "[{:#06x}]  jal x{}, {}",
                    comp_pc,
                    decoder::fetch(instruction, 7, 11),
                    recons
                ));

                // println!("{:#?}", self.trace);
            }
            0b1100111 => {
                // jalr
                if decoder::fetch(instruction, 20, 31) % 4 != 0 {
                    /* The JAL and JALR instructions will generate an instruction-address-misaligned exception if the target
                    address is not aligned to a four-byte boundary. */
                    return Err(Trap::INSTRUCTION_ADDRESS_MISALIGNED);
                }
                self.pc = (self.general_registers[decoder::fetch(instruction, 15, 19) as usize]
                    + sextend(decoder::fetch(instruction, 20, 31), 12))
                    as u32; // rs1 + imm (~1 ??)
                self.general_registers[decoder::fetch(instruction, 7, 11) as usize] =
                    comp_pc as i32 + 4;

                self.trace.push(format!(
                    "[{:#06x}]  jalr x{}, {}(x{})",
                    comp_pc,
                    decoder::fetch(instruction, 7, 11),
                    decoder::fetch(instruction, 20, 31),
                    decoder::fetch(instruction, 15, 19),
                ));
                // println!("{:#?}", self.trace);
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
                    0b000 => rs1 == rs2,                        // beq
                    0b001 => rs1 != rs2,                        // bne
                    0b100 => rs1 < rs2,                         // blt
                    0b110 => (rs1 as u32) < (rs2 as u32),       // bltu
                    0b101 => rs1 >= rs2,                        // bge
                    0b111 => (rs1 as u32) >= (rs2 as u32),      // bgeu
                    _ => return Err(Trap::ILLEGAL_INSTRUCTION), //panic!("fn3 was something unsupported!"),
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
                // println!("{:#?}", self.trace);
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
                    )?;

                self.trace.push(format!(
                    "[{:#06x}]  l{} x{}, {}(x{})",
                    comp_pc,
                    match fn3 {
                        0b000 => "b",
                        0b001 => "h",
                        0b010 => "w",
                        0b100 => "bu",
                        0b101 => "hu",
                        _ => return Err(Trap::ILLEGAL_INSTRUCTION),
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
                )?;

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
                        _ => return Err(Trap::ILLEGAL_INSTRUCTION),
                    },
                    decoder::fetch(instruction, 20, 24),
                    (decoder::fetch(instruction, 25, 31) << 5) | decoder::fetch(instruction, 7, 11),
                    decoder::fetch(instruction, 15, 19),
                ));
            }

            0b0010011 => {
                // println!("{:#5x}", decoder::fetch(instruction, 20, 31));
                let imm: i32 = sextend(decoder::fetch(instruction, 20, 31), 12);

                // println!("{}", imm);
                let rs1: i32 = self.general_registers[decoder::fetch(instruction, 15, 19) as usize];

                self.general_registers[decoder::fetch(instruction, 7, 11) as usize] =
                    match decoder::fetch(instruction, 12, 14) {
                        0b000 => rs1 + imm, // addi
                        0b010 => {
                            if rs1 < imm {
                                1
                            } else {
                                0
                            }
                        }
                        0b011 => {
                            if (rs1 as u32) < (imm as u32) {
                                1
                            } else {
                                0
                            }
                        }
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
                        _ => return Err(Trap::ILLEGAL_INSTRUCTION),
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
                        _ => return Err(Trap::ILLEGAL_INSTRUCTION),
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

                self.general_registers[decoder::fetch(instruction, 7, 11) as usize] = match (
                    decoder::fetch(instruction, 25, 31),
                    decoder::fetch(instruction, 12, 14),
                ) {
                    (0b000_0000, 0b000) => rs1 + rs2,
                    (0b010_0000, 0b000) => rs1 - rs2,
                    (0b000_0001, 0b000) => (rs1 * rs2) & ((1i64 << XLEN) - 1) as i32, // Ext:M mul

                    (0b000_0000, 0b001) => rs1 << (rs2 & 0x1f),
                    (0b000_0001, 0b001) => ((rs1 as i64 * rs2 as i64) >> XLEN) as i32, /* u32 * u32 maxes out at u64 */

                    (0b000_0000, 0b010) => {
                        if rs1 < rs2 {
                            1
                        } else {
                            0
                        }
                    }
                    (0b000_0001, 0b010) => ((rs1 as i64 * (rs2 as u32 as i64)) >> XLEN) as i32, /* rs1 x unsigned rs2 */ // Ext:M mulhsu

                    (0b000_0000, 0b011) => {
                        if (rs1 as u32) < (rs2 as u32) {
                            1
                        } else {
                            0
                        }
                    }
                    (0b000_0001, 0b011) => ((rs1 as u64 * rs2 as u64) >> XLEN) as i32,

                    (0b000_0000, 0b100) => rs1 ^ rs2,
                    (0b000_0001, 0b100) => rs1 / rs2, // Ext:M div

                    (0b000_0000, 0b101) => (rs1 as u32 >> (rs2 & 0x1f)) as i32,
                    (0b010_0000, 0b101) => rs1 >> rs2,
                    (0b000_0001, 0b101) => ((rs1 as u32) / (rs2 as u32)) as i32, // Ext:M divu

                    (0b000_0000, 0b110) => rs1 | rs2,
                    (0b000_0001, 0b110) => rs1 % rs2, // Ext:M rem

                    (0b000_0000, 0b111) => rs1 & rs2,
                    (0b000_0001, 0b111) => ((rs1 as u32) % (rs2 as u32)) as i32, // Ext:M remu

                    _ => return Err(Trap::ILLEGAL_INSTRUCTION),
                };

                self.trace.push(format!(
                    "[{:#06x}]  rr x{}, x{}, x{}",
                    comp_pc,
                    /*match fn3 {
                        0b000 => "b",
                        0b001 => "h",
                        0b010 => "w",
                        0b100 => "bu",
                        0b101 => "hu",
                        _ => panic!("load fn3 error while tracing"),
                    },*/
                    decoder::fetch(instruction, 7, 11),
                    decoder::fetch(instruction, 15, 19),
                    decoder::fetch(instruction, 20, 24),
                ));

                // println!("{:#?}", self.trace);
            } // wow! thats all the major rv31i opcodes.

            0b0001111 => return Ok(()), // fence but we dont need to worry. (single-thread)
            0b1110011 => {
                // ebreak/ecall
                match decoder::fetch(instruction, 20, 31) {
                    0b0000_0000_0000 => return Err(Trap::BREAKPOINT), // panic!("ebreak"),
                    _ => panic!("ecall or other"),
                }
            }

            _ => return Err(Trap::ILLEGAL_INSTRUCTION),
        }

        return Ok(());
    }

    fn fetch(&self) -> Result<u32, Trap> {
        if self.pc % (IALIGN / 8) as u32 != 0 {
            return Err(Trap::INSTRUCTION_ADDRESS_MISALIGNED);
        }
        let fetched = self.bus.load(self.pc as u32, 0b010)? as u32;

        // println!("fetched {:#012x} from {:#04x}", fetched, self.pc);

        Ok(fetched)

        // Trap(TCause.INSTRUCTION_ADDRESS_MISALIGNED)

        // log(instruction.to_bytes(length=4, byteorder="little"))
    }
}
/*

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
*/
