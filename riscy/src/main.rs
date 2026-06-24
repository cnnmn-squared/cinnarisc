mod assembler;
mod devices;
mod machine;
mod risclib;
mod runner;

use std::cell::RefCell;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::rc::Rc;

use minifb::{Scale::X2, Window, WindowOptions};

use assembler::assemble;
use devices::Bus;
use devices::Memory;
use devices::{Device, DeviceOption};
use devices::{HEIGHT, VGATextBuffer, WIDTH};
use machine::Core;
use risclib::file_processor::{newfile, parsebin};

use crate::risclib::Trap;

fn trap<T>(result: Result<T, Trap>) -> Result<T, Box<dyn Error>> {
    // trap to stderror
    if !result.is_ok() {
        println!("!fault! {:?}", Trap::expose_err(&result.err().unwrap()));
        return Err(Box::new(std::fmt::Error));
    }

    Ok(result.ok().unwrap())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut asm: String = String::new();
    {
        let mut asmread: File = File::open("src/bench/arithmetic.s")?;
        asmread.read_to_string(&mut asm)?;
    }
    let asm_lines: Vec<String> = asm.lines().map(|v: &str| v.to_string()).collect();

    {
        let mut wfile: File = File::create("out.obj")?;
        wfile.write(&build_asm(asm_lines)?)?;
    }

    let mut readbuf: Vec<u8> = Vec::new();
    {
        let mut readfile: File = File::open("out.obj")?;
        readfile.read_to_end(&mut readbuf)?;
    }
    let (data, mcode) = parsebin(&readbuf)?;

    let memory: Memory = Memory::new(0x2000);
    let vgatb: Rc<RefCell<VGATextBuffer>> = Rc::new(RefCell::new(VGATextBuffer::new()));
    let mut bus: Bus = Bus::new(vec![
        Device::new(0, 0x1fff, DeviceOption::Memory(memory)),
        Device::new(
            0xb8000,
            0xb8000 + 4000,
            DeviceOption::VGATextBuffer(Rc::clone(&vgatb)),
        ),
    ]);

    for (addr, byte) in mcode.iter().enumerate() {
        trap(bus.store(addr as u32, *byte as i32, 0b000))?;
    }

    let mut cpu: Core = Core::new(bus);

    let mut window = Window::new(
        "a",
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: X2,
            ..WindowOptions::default()
        },
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    while window.is_open() {
        let cpures: Result<(), Trap> = cpu.step();
        let vgabuf: Result<Vec<u32>, Trap> = vgatb.borrow_mut().tick();
        if !cpures.is_ok() {
            println!("!fault! {:?}", Trap::expose_err(&cpures.err().unwrap()));
            return Ok(());
        };

        if !vgabuf.is_ok() {
            println!("!fault! {:?}", Trap::expose_err(&vgabuf.err().unwrap()));
            return Ok(());
        }

        window
            .update_with_buffer(&vgabuf.ok().unwrap(), WIDTH, HEIGHT)
            .unwrap();
    }

    Ok(())
}

fn build_asm(lines: Vec<String>) -> Result<Vec<u8>, Box<dyn Error>> {
    let (mcode, data) = assemble(lines)?;

    Ok(newfile(data, mcode.clone())?)
}

fn build_env() {}
