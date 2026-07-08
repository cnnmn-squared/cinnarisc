use crate::devices::Bus;
use crate::risclib::Trap;
use crate::risclib::sextend;

pub const MXLEN: usize = 32;
pub const XLEN: usize = 32;
pub const IALIGN: usize = 32;

pub const RESET_VECTOR: u32 = 0x00000000;
pub const NMI_VECTOR: u32 = 0x00001000; // priv 3.5

mod decoder {
    pub fn fetch(from: u32, lo: u32, hi: u32) -> i32 {
        // ヾ(＾∇＾)
        // println!("fetch");
        ((from >> lo) & ((1u64 << (hi - lo + 1)) - 1) as u32) as i32
    }
}

mod csr_consts {
    pub const MXL: u32 = 0b01; // RV32
    pub const MISAE: u32 = 0b00_0000_0000_0001_0001_0000_0000; // Extensions IM

    pub mod indexes {
        // machine information (MRO)
        pub const MVENDORID: usize = 0xf11; // Vendor ID
        pub const MARCHID: usize = 0xf12; // Architecture ID
        pub const MIMPID: usize = 0xf13; // Implementation ID
        pub const MHARTID: usize = 0xf14; // Hardware Thread ID
        pub const MCONFIGPTR: usize = 0xf15; // ptr to configuration structure

        // machine trap setup (MRW)
        pub const MSTATUS: usize = 0x300; // machine status
        pub const MISA: usize = 0x301; // ISA and exceptions
        pub const MEDELEG: usize = 0x302; // exception delegation register
        pub const MIDELEG: usize = 0x303; // interrupt delegation register
        pub const MIE: usize = 0x304; // machine interrupt-enable
        pub const MTVEC: usize = 0x305; // trap handler base address
        pub const MCOUNTEREN: usize = 0x306; // counter enable
        pub const MSTATUSH: usize = 0x310; // additional machine status (hi)
        pub const MEDELEGH: usize = 0x312; // upper 32 bits

        // machine trap handling (MRW)
        pub const MSCRATCH: usize = 0x340; // scratch register
        pub const MEPC: usize = 0x341; // program counter @ exception
        pub const MCAUSE: usize = 0x342; // cause
        pub const MTVAL: usize = 0x343; // value
        pub const MIP: usize = 0x344; // interrupt pending
        pub const MTINST: usize = 0x34A; // instruction that trapped
        pub const MTVAL2: usize = 0x34B; // val2
    }
}

pub struct Core {
    pub general_registers: Vec<i32>,
    pub pc: u32,
    pub csr: [u32; 4096],
    pub bus: Bus,
    pub trace: Vec<String>,
    pub entry: u32, // doomed to change
}

impl Core {
    pub fn new(bus: Bus, entry: u32) -> Core {
        let mut gr: Vec<i32> = Vec::new();
        for _ in 0..32 {
            gr.push(0)
        }

        Core {
            general_registers: gr,
            pc: entry,
            csr: Core::setup_csr(),
            bus,
            trace: Vec::new(),
            entry,
        }
    }

    fn setup_csr() -> [u32; 4096] {
        let misa: u32 = (csr_consts::MXL << (MXLEN - 2)) | csr_consts::MISAE;
        // non-commercial specification lists vendor as 0, march is also 0 but can be allocated
        // mimpid 0 if you dont want to implement it (which i dont)
        // mhartid, there must be a hart with id 0 (3.1.5) and we only use 1 hart right?
        // mstatus/mstatush is confusing 3.1.6 come back
        // i dont think any other csrs are on-setup

        let mut csrs: [u32; 4096] = [0; 4096];

        csrs[csr_consts::indexes::MISA] = misa;
        csrs
    }

    pub fn step(&mut self) {
        let pc: u32 = self.pc;
        let fetchresult: Result<u32, Trap> = self.fetch();
        let instruction: u32 = match self.trap_or(fetchresult) {
            Some(instruction) => instruction,
            None => {
                self.pc = NMI_VECTOR;
                if !self.fetch().is_ok() {
                    self.double_trap();
                }
                u32::MAX
            }
        };

        if instruction == u32::MAX {
            return;
        }

        let compresult: Result<(), Trap> = self.compute(pc, instruction);
        self.trap_or(compresult);
    }

    fn trap_or<T>(&mut self, trap: Result<T, Trap>) -> Option<T> {
        if !trap.is_ok() {
            let code = match trap.err().unwrap() {
                Trap::INSTRUCTION_ADDRESS_MISALIGNED => 0x00,
                Trap::INSTRUCTION_ACCESS_FAULT => 0x01,
                Trap::ILLEGAL_INSTRUCTION => 0x02,
                Trap::BREAKPOINT => 0x03,

                Trap::LOAD_ADDRESS_MISALIGNED => 0x04,
                Trap::LOAD_ACCESS_FAULT => 0x05,

                Trap::STORE_ADDRESS_MISALIGNED => 0x06,
                Trap::STORE_ACCESS_FAULT => 0x07,

                Trap::ENV_CALL_FROM_U => 0x08,
                Trap::ENV_CALL_FROM_S => 0x09,
                // reserved 0x0a
                Trap::ENV_CALL_FROM_M => 0x0b,

                Trap::INSTRUCTION_PAGE_FAULT => 0x0c,
                Trap::LOAD_PAGE_FAULT => 0x0d,
                // reserved 0x0e
                Trap::STORE_PAGE_FAULT => 0x0f,

                Trap::DOUBLE_TRAP => 0x10,
                // reserved 0x11
                Trap::SOFTWARE_CHECK => 0x12,
                Trap::HARDWARE_ERROR => 0x13,
                // 0x14 -> 0x17 reserved
                // 0x18 -> 0x1f dfcu (private)
                // 0x20 -> 0x2f reserved
                // 0x30 -> 0x3f dfcu (private)
                // 0x40 ->> reserved
            };

            self.csr[csr_consts::indexes::MCAUSE] = code;
            self.csr[csr_consts::indexes::MEPC] = self.pc;
            None
        } else {
            trap.ok()
        }
    }

    fn double_trap(&mut self) {
        // send it to boot
        self.reset(true);
    }

    fn compute(&mut self, pc: u32, instruction: u32) -> Result<(), Trap> {
        self.general_registers[0] = 0;

        match decoder::fetch(instruction, 0, 6) {
            0b0110111 => {
                self.general_registers[decoder::fetch(instruction, 7, 11) as usize] =
                    (sextend(decoder::fetch(instruction, 12, 31), 20) << 12) as i32;

                self.trace.push(format!(
                    "[{:#06x}]  lui x{}, {}",
                    pc,
                    decoder::fetch(instruction, 7, 11),
                    decoder::fetch(instruction, 12, 31)
                ));
            }
            0b0010111 => {
                self.general_registers[decoder::fetch(instruction, 7, 11) as usize] = // rd
                    (self.pc as i32 + (decoder::fetch(instruction, 12, 31) << 12)) as i32;

                self.trace.push(format!(
                    "[{:#06x}]  auipc x{}, {}",
                    pc,
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
                self.pc = (pc as i32 + recons) as u32;
                self.general_registers[decoder::fetch(instruction, 7, 11) as usize] = // rd
                    (pc + 4) as i32;

                self.trace.push(format!(
                    "[{:#06x}]  jal x{}, {}",
                    pc,
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
                self.general_registers[decoder::fetch(instruction, 7, 11) as usize] = pc as i32 + 4;

                self.trace.push(format!(
                    "[{:#06x}]  jalr x{}, {}(x{})",
                    pc,
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
                    self.pc = (pc as i32 + recons) as u32
                } /* look at how much more efficient */

                self.trace.push(format!(
                    "[{:#06x}]  BNC x{}, x{}, {} # {} ?= {} # {:#12x}",
                    pc,
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
                if fn3 <= 3 {
                    self.general_registers[decoder::fetch(instruction, 7, 11) as usize] = sextend(
                        self.bus.load(
                            (self.general_registers[decoder::fetch(instruction, 15, 19) as usize]
                                + sextend(decoder::fetch(instruction, 20, 31), 12))
                                as u32,
                            fn3.try_into().unwrap(),
                        )?,
                        match fn3 {
                            0b000 => 8,
                            0b001 => 16,
                            0b010 => 32,
                            _ => return Err(Trap::ILLEGAL_INSTRUCTION),
                        },
                    );
                } else {
                    self.general_registers[decoder::fetch(instruction, 7, 11) as usize] =
                        self.bus.load(
                            (self.general_registers[decoder::fetch(instruction, 15, 19) as usize]
                                + sextend(decoder::fetch(instruction, 20, 31), 12))
                                as u32,
                            fn3.try_into().unwrap(),
                        )?;
                }

                self.trace.push(format!(
                    "[{:#06x}]  l{} x{}, {}(x{})",
                    pc,
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
                    pc,
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

                /*self.trace.push(format!(
                    "[{:#06x}]  {} x{}, {}(x{}) # {:#12x}",
                    pc,
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
                ));*/
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
                    pc,
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
            } // wow! thats all the major rv31i opcodes.

            0b0001111 => return Ok(()), // fence but we dont need to worry. (single-thread)
            0b1110011 => {
                // ebreak/ecall
                // 3.3.1
                self.csr[csr_consts::indexes::MEPC] = pc; // 3.3.1
                match decoder::fetch(instruction, 20, 31) {
                    0b0000_0000_0000 => return Err(Trap::ENV_CALL_FROM_M),
                    0b0000_0000_0001 => return Err(Trap::BREAKPOINT),
                    0b0011_0000_0010 => self.pc = self.csr[csr_consts::indexes::MEPC],
                    _ => {
                        self.general_registers[decoder::fetch(instruction, 7, 11) as usize] =
                            match decoder::fetch(instruction, 12, 14) {
                                0b001 => {
                                    let csrt =
                                        self.csr[decoder::fetch(instruction, 20, 31) as usize];
                                    self.csr[decoder::fetch(instruction, 20, 31) as usize] = self
                                        .general_registers
                                        [decoder::fetch(instruction, 15, 19) as usize]
                                        as u32;
                                    // If rd=x0, then the instruction shall not read the CSR and shall not cause any of the side effects that might occur on a If rd=x0, then the instruction shall not read the CSR and shall not cause any of the side effects that might occur on a CSR read.CSR read. /? ??

                                    csrt as i32
                                }
                                // CSRRW
                                0b010 => {
                                    let csrt =
                                        self.csr[decoder::fetch(instruction, 20, 31) as usize];
                                    self.csr[decoder::fetch(instruction, 20, 31) as usize] = self
                                        .general_registers
                                        [decoder::fetch(instruction, 15, 19) as usize]
                                        as u32
                                        | csrt;

                                    csrt as i32
                                } // CSRRS
                                0b011 => {
                                    let csrt =
                                        self.csr[decoder::fetch(instruction, 20, 31) as usize];
                                    self.csr[decoder::fetch(instruction, 20, 31) as usize] = csrt
                                        & !self.general_registers
                                            [decoder::fetch(instruction, 15, 19) as usize]
                                            as u32;

                                    csrt as i32 // remember to zeroextend but all of these are u32 anyways so what could you possibly extend?
                                } // CSRRC
                                0b101 => {
                                    let csrt =
                                        self.csr[decoder::fetch(instruction, 20, 31) as usize];
                                    self.csr[decoder::fetch(instruction, 20, 31) as usize] =
                                        decoder::fetch(instruction, 15, 19) as u32;
                                    // If rd=x0, then the instruction shall not read the CSR and shall not cause any of the side effects that might occur on a If rd=x0, then the instruction shall not read the CSR and shall not cause any of the side effects that might occur on a CSR read.CSR read. //???

                                    csrt as i32
                                }
                                // CSRRWI
                                0b110 => {
                                    let csrt =
                                        self.csr[decoder::fetch(instruction, 20, 31) as usize];
                                    self.csr[decoder::fetch(instruction, 20, 31) as usize] =
                                        decoder::fetch(instruction, 15, 19) as u32 | csrt;

                                    csrt as i32
                                } // CSRRSI
                                0b111 => {
                                    let csrt =
                                        self.csr[decoder::fetch(instruction, 20, 31) as usize];
                                    self.csr[decoder::fetch(instruction, 20, 31) as usize] =
                                        csrt & !decoder::fetch(instruction, 15, 19) as u32;

                                    csrt as i32 // remember to zeroextend but all of these are u32 anyways so what could you possibly extend?
                                } // CSRRCI
                                _ => return Err(Trap::ILLEGAL_INSTRUCTION),
                            }
                    }
                }
            }

            _ => return Err(Trap::ILLEGAL_INSTRUCTION),
        }

        return Ok(());
    }

    fn reset(&mut self, isdouble: bool) {
        // remind me to make a bootloader when i add elf loading or something
        // 3.4 Reset
        self.csr[csr_consts::indexes::MIE] = 0;
        // ! mprv
        // ! mstatus stuff

        self.pc = RESET_VECTOR;
        // other stuff ig.
        // mcause after reset MAY have implementation spec, 0 is default though?
        self.csr[csr_consts::indexes::MCAUSE] = if isdouble { 0x10 } else { 0x00 }; // DOUBLE_TRAP
    }

    fn fetch(&mut self) -> Result<u32, Trap> {
        if self.pc % (IALIGN / 8) as u32 != 0 {
            return Err(Trap::INSTRUCTION_ADDRESS_MISALIGNED);
        }
        let fetched = match self.bus.load(self.pc as u32, 0b010) {
            Ok(v) => v as u32,
            Err(cause) => {
                return Err(match cause {
                    Trap::LOAD_ACCESS_FAULT => Trap::INSTRUCTION_ACCESS_FAULT,
                    Trap::LOAD_ADDRESS_MISALIGNED => Trap::INSTRUCTION_ADDRESS_MISALIGNED, // should be caught by first check
                    Trap::LOAD_PAGE_FAULT => Trap::INSTRUCTION_PAGE_FAULT, // idk what this is
                    _cause => _cause,                                      // transparent the rest
                });
            } // actually map it for once
        };

        self.pc += 4;

        Ok(fetched)
    }
}
