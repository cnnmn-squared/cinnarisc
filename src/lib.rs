pub mod assembler;
pub mod devices;
pub mod machine;
pub mod risclib;

pub use assembler::assemble;

pub use risclib::Trap;

pub use machine::Core;
pub use machine::IALIGN;
pub use machine::RESET_VECTOR;

pub use devices::Bus;
pub use devices::DataEnsuredMemory;
pub use devices::StandardAccess;
pub use devices::StdMemory;
pub use devices::VGATextBuffer;
pub use devices::{Device, DeviceOption};
pub use devices::{HEIGHT, WIDTH};

pub use std::cell::RefCell;
pub use std::env::args;
pub use std::error::Error;
pub use std::fs::File;
pub use std::io::{Read, Write};
pub use std::rc::Rc;

pub use minifb::Window;
pub use minifb::WindowOptions;

pub mod random {
    use std::ops::RangeInclusive;

    use rand::RngExt;
    use rand::rng;

    pub fn rri(range: RangeInclusive<i32>) -> i32 {
        rng().random_range(range)
    }
}

pub mod elf {
    use crate::StandardAccess;
    use crate::StdMemory;
    use crate::Trap;

    use std::error::Error;

    const P_LOAD: u32 = 0x0000_0001;

    pub enum ElfErr {
        NoMagic,
        NoELF,
        InvalidArch,
        NotExecutable,
    }
    impl ElfErr {
        pub fn expose(err: &ElfErr) -> &str {
            match err {
                ElfErr::NoELF => "No ELF header in file (expecting b\"ELF\" in address 1 -> 4)",
                ElfErr::NoMagic => "No magic number in file (expecting 0x7f in address 0)",
                ElfErr::InvalidArch => {
                    "Invalid architecture! expected riscv (0xf3) but got something else!"
                }
                ElfErr::NotExecutable => {
                    "File type is not executable! expected 0x02 (ET_EXEC) but got something else!"
                }
            }
        }
        pub fn into<T>(result: Result<T, ElfErr>) -> Result<T, Box<dyn Error>> {
            if !result.is_ok() {
                println!("ElfErr {}!", ElfErr::expose(&result.err().unwrap()));
                return Err(Box::new(std::fmt::Error));
            }

            Ok(result.ok().unwrap())
        }
    }

    #[derive(Debug)]
    struct Ehdr {
        ident: [u8; 16],
        etype: u16,
        machine: u16,
        version: u32,

        entry: u32,
        phoff: u32, // program header offset
        shoff: u32, // section header offset

        flags: u32,
        ehsize: u16, // headersize

        phentsize: u16, // program header entry size
        phnum: u16,     // program header entry count

        shentsize: u16, // section header entry size
        shnum: u16,     // section header entry count
        shstrndx: u16,  // section header string index
    }

    impl Ehdr {
        fn new(
            ident: [u8; 16],
            etype: u16,
            machine: u16,
            version: u32,
            entry: u32,
            phoff: u32,
            shoff: u32,
            flags: u32,
            ehsize: u16,
            phentsize: u16,
            phnum: u16,
            shentsize: u16,
            shnum: u16,
            shstrndx: u16,
        ) -> Ehdr {
            Ehdr {
                ident,
                etype,
                machine,
                version,
                entry,
                phoff,
                shoff,
                flags,
                ehsize,
                phentsize,
                phnum,
                shentsize,
                shnum,
                shstrndx,
            }
        }
    }

    #[derive(Debug)]
    struct Phdr {
        ptype: u32,  // 0x00000001 => LOAD
        offset: u32, // where to find the connected section
        vaddr: u32,  // where is this for the processor (virt)
        paddr: u32,  // where is this for the processor (phys)
        filesz: u32, // size of segment within elf
        memsz: u32,  // size of segment within memory
        flags: u32,
        align: u32, // 0/1 == none, should be power of 2 (wiki says vaddr == offset % align)
    }

    /*struct Shdr {
        name: u32, // An offset to a string in the .shstrtab section that represents the name of this section.
        stype: u32,
        flags: u32,
        addr: u32,   // address of loaded section
        offset: u32, // offset within elf
        size: u32,   // section size
        link: u32,
        info: u32,
        align: u32, // alignment of the section (pow2)
        esize: u32, // size of entry
    }*/

    fn confirm(ehdr: &Ehdr /* , phdr: Phdr*/) -> Result<(), ElfErr> {
        //? https://en.wikipedia.org/wiki/Executable_and_Linkable_Format
        if ehdr.ident[0] != 0x7f {
            return Err(ElfErr::NoMagic);
        }
        if ehdr.ident[1..4] != *b"ELF" {
            return Err(ElfErr::NoELF);
        }
        if ehdr.etype != 0x02 {
            return Err(ElfErr::NotExecutable);
        }
        if ehdr.machine != 0xf3 {
            // RISCV
            return Err(ElfErr::InvalidArch);
        }
        // ignore entry.. confirm ialign?
        // rest are variable.

        // evers can be any (preferrably 1 ig.)
        Ok(())
    }

    fn getbytes(from: &[u8], start: usize, end: usize) -> Vec<u8> {
        from[start..end].to_vec()
    }

    fn getu16(from: &[u8], at: usize) -> u16 {
        // println!("{:?}", &from[start..end]);
        u16::from_le_bytes(from[at..at + 2].try_into().unwrap())
    }

    fn getu32(from: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(from[at..at + 4].try_into().unwrap())
    }

    fn give_ehdr(obj: &[u8]) -> Ehdr {
        let ident: [u8; 16] = getbytes(obj, 0x00, 0x10).try_into().unwrap();
        let etype: u16 = u16::from_le_bytes(getbytes(obj, 0x10, 0x12).try_into().unwrap());
        let machine: u16 = getu16(obj, 0x12);
        let version: u32 = getu32(obj, 0x14);
        let entry: u32 = getu32(obj, 0x18);
        let phoff: u32 = getu32(obj, 0x1c);
        let shoff: u32 = getu32(obj, 0x20);
        let flags: u32 = getu32(obj, 0x24);
        let ehsize: u16 = getu16(obj, 0x28);
        let phentsize: u16 = getu16(obj, 0x2a);
        let phnum: u16 = getu16(obj, 0x2c);
        let shentsize: u16 = 0;
        let shnum: u16 = 0;
        let shstrndx: u16 = 0;

        Ehdr::new(
            ident, etype, machine, version, entry, phoff, shoff, flags, ehsize, phentsize, phnum,
            shentsize, shnum, shstrndx,
        )
    }
    fn load_prog(obj: &[u8], from: usize) -> Phdr {
        let ptype: u32 = getu32(obj, from + 0x00); // 0x00000001 => LOAD
        let offset: u32 = getu32(obj, from + 0x04); // where to find the connected section
        let vaddr: u32 = getu32(obj, from + 0x08); // where is this for the processor (virt);
        let paddr: u32 = getu32(obj, from + 0x0c); // where is this for the processor (phys);
        let filesz: u32 = getu32(obj, from + 0x10); // size of segment within elf
        let memsz: u32 = getu32(obj, from + 0x14); // size of segment within memory
        let flags: u32 = getu32(obj, from + 0x18);
        let align: u32 = getu32(obj, from + 0x1c); // 0/1 == none, should be power of 2 (wiki says vaddr == offset % align)

        Phdr {
            ptype,
            offset,
            vaddr,
            paddr,
            filesz,
            memsz,
            flags,
            align,
        }
    }

    pub fn load(obj: &[u8], mem: &mut StdMemory) -> Result<u32, ElfErr> {
        let ehdr = give_ehdr(obj);
        confirm(&ehdr)?;
        let mut pheaders: Vec<Phdr> = Vec::new();
        for ientry in 0..ehdr.phnum {
            let phdr = load_prog(
                obj,
                ehdr.phoff as usize + ientry as usize * ehdr.phentsize as usize,
            );
            pheaders.push(phdr);
        }

        for pheader in &pheaders {
            if pheader.ptype == P_LOAD {
                println!(
                    "P_LOAD region {}..{} into memory {} of size {}",
                    pheader.offset,
                    pheader.offset + pheader.filesz,
                    pheader.vaddr,
                    pheader.memsz
                );

                for (i, byte) in obj
                    [pheader.offset as usize..pheader.offset as usize + pheader.filesz as usize]
                    .iter()
                    .enumerate()
                {
                    let errorres: Result<(), crate::Trap> =
                        mem.store_byte(pheader.vaddr + i as u32, *byte as i32);
                    if !errorres.is_ok() {
                        println!("{}", Trap::expose_err(&errorres.err().unwrap()));
                        return Err(ElfErr::NotExecutable);
                    }
                }
            }
        }

        Ok(ehdr.entry)
    }

    /*fn raw(obj: &[u8]) -> Result<&[u8], ElfErr> {
        let ehdr = give_ehdr(obj);
        confirm(&ehdr)?;
        let mut pheaders: Vec<Phdr> = Vec::new();
        for ientry in 0..ehdr.phnum {
            let phdr = load_prog(
                obj,
                ehdr.phoff as usize + ientry as usize * ehdr.phentsize as usize,
            );
            pheaders.push(phdr);
        }

        for pheader in &pheaders {
            if pheader.ptype == P_LOAD {
                println!(
                    "P_LOAD region {}..{} into memory {} of size {}",
                    pheader.offset,
                    pheader.offset + pheader.filesz,
                    pheader.vaddr,
                    pheader.memsz
                );

                for (i, byte) in obj
                    [pheader.offset as usize..pheader.offset as usize + pheader.filesz as usize]
                    .iter()
                    .enumerate()
                {
                    let
                }
            }
        }

        Ok(obj)
    }*/
}
