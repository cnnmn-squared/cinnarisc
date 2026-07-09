pub const WIDTH: usize = 640;
pub const HEIGHT: usize = 400;
const VGA_GLYPHS: &[u8] = include_bytes!("resources/VGA8.F16");

pub const XLEN: u32 = 32;
const PAGESZ: u32 = 4096;

use std::collections::HashMap;

use crate::{Rc, RefCell, Trap, random};

pub trait StandardAccess {
    fn load_byte(&self, _addr: u32) -> Result<i32, Trap> {
        return Err(Trap::LOAD_ACCESS_FAULT);
    }
    fn load_half(&self, _addr: u32) -> Result<i32, Trap> {
        return Err(Trap::LOAD_ACCESS_FAULT);
    }
    fn load_word(&self, _addr: u32) -> Result<i32, Trap> {
        return Err(Trap::LOAD_ACCESS_FAULT);
    }

    fn store_byte(&mut self, _addr: u32, _value: i32) -> Result<(), Trap> {
        return Err(Trap::STORE_ACCESS_FAULT);
    }
    fn store_half(&mut self, _addr: u32, _value: i32) -> Result<(), Trap> {
        return Err(Trap::STORE_ACCESS_FAULT);
    }
    fn store_word(&mut self, _addr: u32, _value: i32) -> Result<(), Trap> {
        return Err(Trap::STORE_ACCESS_FAULT);
    }

    fn load_byte_us(&self, addr: u32) -> Result<i32, Trap> {
        Ok(self.load_byte(addr)? as u32 as i32)
    }

    fn load_half_us(&self, addr: u32) -> Result<i32, Trap> {
        Ok(self.load_half(addr)? as u32 as i32)
    }

    fn load_thru_type(&self, addr: u32, load_type: u32) -> Result<i32, Trap> {
        Ok(match load_type {
            0b000 => self.load_byte(addr)?,
            0b001 => self.load_half(addr)?,
            0b010 => self.load_word(addr)?,
            0b011 => return Err(Trap::ILLEGAL_INSTRUCTION), // doulbe
            0b100 => self.load_byte_us(addr)?,              // lbu
            0b101 => self.load_half_us(addr)?,              // lhu

            _ => return Err(Trap::ILLEGAL_INSTRUCTION),
        })
    }

    fn store_thru_type(&mut self, addr: u32, value: i32, store_type: u32) -> Result<(), Trap> {
        Ok(match store_type {
            0b000 => self.store_byte(addr, value)?,
            0b001 => self.store_half(addr, value)?,
            0b010 => self.store_word(addr, value)?,
            0b011 => return Err(Trap::ILLEGAL_INSTRUCTION),
            // double
            _ => return Err(Trap::ILLEGAL_INSTRUCTION),
        })
    }
}

pub struct StdMemory {
    pages: HashMap<u32, Page>, // baseaddr: assoc.page
}

// PMP?

impl StandardAccess for StdMemory {
    fn load_byte(&self, addr: u32) -> Result<i32, Trap> {
        let page = self.getpage(addr);
        if !page.is_ok() {
            return Ok(0);
        }

        return Ok(crate::risclib::sextend(
            page.ok().unwrap().read_byte(addr & 0xfff)?,
            8,
        ));
    }

    fn load_half(&self, addr: u32) -> Result<i32, Trap> {
        Ok(((self.load_byte(addr + 1)? & 0xff) << 8) | (self.load_byte(addr)? & 0xff))
    }

    fn load_word(&self, addr: u32) -> Result<i32, Trap> {
        Ok((self.load_half(addr + 2)? << 16) | self.load_half(addr)?)
    }

    fn load_byte_us(&self, addr: u32) -> Result<i32, Trap> {
        let page = self.getpage(addr);
        if !page.is_ok() {
            return Ok(0);
        }

        return page.ok().unwrap().read_byte(addr & 0xfff);
    }

    fn store_byte(&mut self, addr: u32, val: i32) -> Result<(), Trap> {
        println!("sb {} -> {}", val, addr);
        let page = self.getpage_mut(addr);
        if !page.is_ok() {
            let mut newpage = Page::new();
            newpage.write_byte(addr & 0xfff, val)?;
            self.pages.insert(addr >> 12, newpage);
            return Ok(());
        }

        page.ok().unwrap().write_byte(addr & 0xfff, val)
    }

    fn store_half(&mut self, addr: u32, val: i32) -> Result<(), Trap> {
        self.store_byte(addr + 1, val >> 8)?;
        self.store_byte(addr, val & 0xff)
    }

    fn store_word(&mut self, addr: u32, val: i32) -> Result<(), Trap> {
        self.store_half(addr + 2, val >> 16)?;
        self.store_half(addr, val & 0xffff)
    }
}

impl StdMemory {
    pub fn new() -> StdMemory {
        StdMemory {
            pages: HashMap::new(),
        }
    }

    pub fn getpage(&self, addr: u32) -> Result<&Page, Trap> {
        let pager: Option<&Page> = self.pages.get(&(addr >> 12));
        match pager {
            Some(page) => Ok(page),
            None => Err(Trap::LOAD_PAGE_FAULT),
        }
    }

    pub fn getpage_mut(&mut self, addr: u32) -> Result<&mut Page, Trap> {
        let pager: Option<&mut Page> = self.pages.get_mut(&(addr >> 12));
        match pager {
            Some(page) => Ok(page),
            None => Err(Trap::STORE_PAGE_FAULT),
        }
    }
}
pub struct Page {
    data: [u8; PAGESZ as usize],
}

impl Page {
    pub fn new() -> Page {
        Page {
            data: [0; PAGESZ as usize],
        }
    }

    fn read_byte(&self, addr: u32) -> Result<i32, Trap> {
        // page relative
        match self.data.get(addr as usize) {
            Some(fetched) => Ok(*fetched as i32),
            None => return Err(Trap::LOAD_ACCESS_FAULT),
        }
    }

    /*fn read_half(&self, addr: u32) -> Result<i32, Trap> {
        if addr % 2 != 0 {
            return Err(Trap::LOAD_ADDRESS_MISALIGNED);
        }

        Ok((self.read_byte(addr + 1)? << 8) | self.read_byte(addr)?)
    }

    fn read_word(&self, addr: u32) -> Result<i32, Trap> {
        if addr % 4 != 0 {
            return Err(Trap::LOAD_ADDRESS_MISALIGNED);
        }

        Ok((self.read_half(addr + 2)? << 16) | self.read_half(addr)?)
    }*/

    fn write_byte(&mut self, addr: u32, val: i32) -> Result<(), Trap> {
        // println!("{} {}", addr, val);
        match self.data.get_mut(addr as usize) {
            Some(v) => Ok(*v = (val & 0xff) as u8),
            None => Err(Trap::STORE_ACCESS_FAULT),
        }
    }

    /*fn write_half(&mut self, addr: u32, val: i32) -> Result<(), Trap> {
        if addr % 2 != 0 {
            return Err(Trap::STORE_ADDRESS_MISALIGNED);
        }
        let val = val & 0xffff;
        self.write_byte(addr + 1, val >> 8)?;
        self.write_byte(addr, val & 0xff);

        Ok(())
    }

    fn write_word(&mut self, addr: u32, val: i32) -> Result<(), Trap> {
        if addr % 4 != 0 {
            return Err(Trap::STORE_ADDRESS_MISALIGNED);
        }
        self.write_half(addr + 2, val >> 16)?;
        self.write_half(addr, val & 0xffff)?;

        Ok(())
    }*/
}

pub struct DataEnsuredMemory {
    // ! this is sure to change, ive just called it this because the address to data is ensured to be valid (unlike stdmem)
    data: Box<[u8]>,
}

impl StandardAccess for DataEnsuredMemory {
    fn load_byte(&self, addr: u32) -> Result<i32, Trap> {
        Ok(*self.data.get(addr as usize).unwrap() as i32)
    }

    fn load_half(&self, addr: u32) -> Result<i32, Trap> {
        if addr % 2 != 0 {
            return Err(Trap::LOAD_ADDRESS_MISALIGNED);
        }
        Ok((self.load_byte(addr + 1)? << 8) | self.load_byte(addr)?)
    }

    fn load_word(&self, addr: u32) -> Result<i32, Trap> {
        if addr % 4 != 0 {
            return Err(Trap::LOAD_ADDRESS_MISALIGNED);
        }

        Ok((self.load_half(addr + 2)? << 16) | self.load_half(addr)?)
    }

    fn store_byte(&mut self, addr: u32, val: i32) -> Result<(), Trap> {
        if let Some(byte) = self.data.get_mut(addr as usize) {
            *byte = val as u8;
        }

        Ok(())
    }

    fn store_half(&mut self, addr: u32, val: i32) -> Result<(), Trap> {
        if addr % 2 != 0 {
            return Err(Trap::STORE_ADDRESS_MISALIGNED);
        }

        self.store_byte(addr, val & 0xff)?;
        self.store_byte(addr + 1, (val >> 8) & 0xff)?;

        return Ok(());
    }

    fn store_word(&mut self, addr: u32, val: i32) -> Result<(), Trap> {
        if addr % 4 != 0 {
            return Err(Trap::STORE_ADDRESS_MISALIGNED);
        }

        self.store_half(addr, val & 0xffff)?;
        self.store_half(addr + 2, (val >> 16) & 0xffff)?;

        Ok(())
    }
}

impl DataEnsuredMemory {
    pub fn new(size: u32) -> DataEnsuredMemory {
        if size > ((1u64 << XLEN) - 1) as u32 {
            panic!("`size` is larger than xlen! {}", size);
        }

        DataEnsuredMemory {
            data: vec![0u8; size as usize].into_boxed_slice(),
        }
    }
}

pub struct Random {}

impl StandardAccess for Random {
    fn load_byte(&self, _: u32) -> Result<i32, Trap> {
        Ok(random::rri(0..=u8::MAX as i32))
    }

    fn load_half(&self, _: u32) -> Result<i32, Trap> {
        Ok(random::rri(0..=u16::MAX as i32))
    }
    fn load_word(&self, _: u32) -> Result<i32, Trap> {
        Ok(random::rri(0..=u32::MAX as i32))
    }
}

impl Random {
    pub fn new() -> Random {
        Random {}
    }
}

struct Region {
    lo: u32,
    hi: u32,
}

impl Region {
    pub fn new(lo: u32, hi: u32) -> Region {
        if hi < lo {
            panic!("new region but hi ({}) is < lo ({})", hi, lo);
        }

        Region { lo, hi }
    }
}

pub enum DeviceOption {
    StdMem(StdMemory),
    Memory(DataEnsuredMemory),
    VGATextBuffer(Rc<RefCell<VGATextBuffer>>),
    Random(Random),
}
impl DeviceOption {
    fn expose(&self) -> &str {
        match &self {
            DeviceOption::Memory(_) => "Memory",
            DeviceOption::StdMem(_) => "Memory (virtual)",
            DeviceOption::VGATextBuffer(_) => "VGAtext",
            DeviceOption::Random(_) => "Random",
        }
    }
}
pub struct Device {
    region: Region,
    connect: DeviceOption,
}

impl Device {
    pub fn new(lo: u32, hi: u32, connected: DeviceOption) -> Device {
        Device {
            region: Region::new(lo, hi),
            connect: connected,
        }
    }
}
pub struct Bus {
    devices: Vec<Device>,
}

impl Bus {
    pub fn new(devices: Vec<Device>) -> Bus {
        // protect from collisions
        // impl pls
        // trace l8r
        Bus { devices }
    }

    fn mut_find_devr_from_addr(&mut self, addr: u32) -> Result<&mut Device, Trap> {
        for device in &mut self.devices {
            if addr >= device.region.lo && addr < device.region.hi {
                return Ok(device);
            }
        }

        return Err(Trap::STORE_ACCESS_FAULT); // mutable suggests that it is a store
    }

    fn find_devr_from_addr(&self, addr: u32) -> Result<&Device, Trap> {
        for device in &self.devices {
            if addr >= device.region.lo && addr < device.region.hi {
                return Ok(device);
            }
        }

        return Err(Trap::LOAD_ACCESS_FAULT); // immut suggests that is it a load
    }

    pub fn load(&self, addr: u32, ltype: u8) -> Result<i32, Trap> {
        let select = self.find_devr_from_addr(addr)?;

        Ok(match &select.connect {
            DeviceOption::Memory(memory) => {
                match ltype {
                    0b000 => memory.load_byte(addr - select.region.lo)?,
                    0b001 => memory.load_half(addr - select.region.lo)?,
                    0b010 => memory.load_word(addr - select.region.lo)?,
                    0b011 => return Err(Trap::ILLEGAL_INSTRUCTION),
                    0b100 => memory.load_byte_us(addr)?, // lbu
                    0b101 => memory.load_half_us(addr)?, // lhu

                    _ => return Err(Trap::ILLEGAL_INSTRUCTION),
                }
            }

            DeviceOption::VGATextBuffer(_) => {
                return Err(Trap::LOAD_ACCESS_FAULT);
            }

            DeviceOption::StdMem(memory) => match ltype {
                0b000 => memory.load_byte(addr - select.region.lo)?,
                0b001 => memory.load_half(addr - select.region.lo)?,
                0b010 => memory.load_word(addr - select.region.lo)?,
                0b011 => return Err(Trap::ILLEGAL_INSTRUCTION), // load double (unimpl)
                0b100 => memory.load_byte_us(addr - select.region.lo)?,
                0b101 => memory.load_half_us(addr - select.region.lo)?,
                _ => return Err(Trap::ILLEGAL_INSTRUCTION),
            },

            DeviceOption::Random(device) => device.load_thru_type(addr, ltype as u32)?,
        })
    }

    pub fn store(&mut self, addr: u32, value: i32, stype: u8) -> Result<(), Trap> {
        let select: &mut Device = self.mut_find_devr_from_addr(addr)?;
        /*println!(
            "[trace] store {:#x} -> {:#012x}, region responded with {}",
            value,
            addr,
            select.connect.expose()
        );*/

        //trace
        match &mut select.connect {
            DeviceOption::Memory(memory) => match stype {
                0b000 => memory.store_byte(addr, value)?,
                0b001 => memory.store_half(addr, value)?,
                0b010 => memory.store_word(addr, value)?,
                _ => return Err(Trap::ILLEGAL_INSTRUCTION),
            },
            DeviceOption::VGATextBuffer(vgatb) => {
                let mut vgatbb: std::cell::RefMut<'_, VGATextBuffer> = vgatb.borrow_mut();

                match stype {
                    0b000 => vgatbb.owning.store_byte(addr - select.region.lo, value)?,
                    0b001 => vgatbb.owning.store_half(addr - select.region.lo, value)?,
                    0b010 => vgatbb.owning.store_word(addr - select.region.lo, value)?,
                    _ => return Err(Trap::ILLEGAL_INSTRUCTION),
                };
            }
            DeviceOption::StdMem(memory) => match stype {
                0b000 => memory.store_byte(addr, value)?,
                0b001 => memory.store_half(addr, value)?,
                0b010 => memory.store_word(addr, value)?,
                _ => return Err(Trap::ILLEGAL_INSTRUCTION),
            },

            DeviceOption::Random(device) => device.store_thru_type(addr, value, stype as u32)?,
        }

        Ok(())
    }
}

pub struct VGATextBuffer {
    owning: DataEnsuredMemory,
    assocb: Vec<u32>,
    pub row: u32,
}

impl VGATextBuffer {
    pub fn new() -> VGATextBuffer {
        let buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
        VGATextBuffer {
            owning: DataEnsuredMemory::new(4000),
            assocb: buffer,
            row: 0,
        }
    }

    fn setchar(&mut self, x: usize, y: usize, ch: char, ffore: u8, fback: u8) {
        let chi: usize = (ch as u8 as u32 * 16) as usize;
        for row in 0..16 {
            let byte = VGA_GLYPHS[chi + row];
            for bitn in 0..8 {
                self.assocb[row * WIDTH + y * WIDTH + bitn + x] = if (byte >> (7 - bitn)) & 1 == 1 {
                    match ffore {
                        0x0 => 0x000000, // black / dark grey
                        0x1 => 0x0000ff, // blue  / light blue
                        0x2 => 0x00ff00, // green / lime
                        0x3 => 0x00ffff, // cyan?
                        0x4 => 0xff0000, // red
                        0x5 => 0xff00ff, // magenta
                        0x6 => 0xff5500, // brown (noimpl)
                        0x7 => 0x808080, // light grey
                        0x8 => 0x404040, // dark grey
                        0x9 => 0x5555FF, // thank u google light blue
                        0xa => 0x55ff55, // ig its all 55 light grreen
                        0xb => 0x55ffff, // light cyan
                        0xc => 0xff5555, // luight red
                        0xd => 0xff55ff, // pink
                        0xe => 0xffff00, // yellow
                        0xf => 0xffffff, // while
                        _ => panic!("wrong colour, this doesnt return a trap"),
                    } // move out of this for more opt?
                } else {
                    match fback {
                        0x0 => 0x000000, // black / dark grey
                        0x1 => 0x0000ff, // blue  / light blue
                        0x2 => 0x00ff00, // green / lime
                        0x3 => 0x00ffff, // cyan?
                        0x4 => 0xff0000, // red
                        0x5 => 0xff00ff, // magenta
                        0x6 => 0xff5500, // brown (noimpl)
                        0x7 => 0x808080, // light grey
                        _ => panic!("wrong colour, this function doesnt return a trap"),
                    }
                }
            }
        }
    }

    pub fn tick(&mut self) -> Result<Option<Vec<u32>>, Trap> {
        // expose the Vec so i dont have to share the buffer

        for halfi in self.row * 80..self.row * 80 + 80 {
            let half = self.owning.load_half(halfi * 2)?;
            let ch = (half & 0xff) as u8 as char;
            let flags = ((half >> 8) & 0xff) as u8;
            let ffore = flags & 0xf;
            let fback = (flags >> 4) & 0x7;
            // let blink = (flags >> 7) & 0b1; blinking would be a pain right now

            if ch == 0x00 as char {
                continue;
            }

            self.setchar(
                ((halfi % 80) * 8) as usize,
                ((halfi / 80) * 16) as usize,
                ch,
                ffore,
                fback,
            );
        }
        self.row = (self.row + 1) % 25;

        Ok(if self.row == 0 {
            Some(self.assocb.clone())
        } else {
            None
        }) // ! performance
    }
}
