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

fn main() -> Result<(), Box<dyn Error>> {
    let lines: Vec<String> = vec![
        ".org 0x00001000".to_string(),
        ".section .data".to_string(),
        "byte abc = 250".to_string(),
        ".section .text".to_string(),
        "lui ra, 0x0001".to_string(),
    ];

    let mut asm_bytes: Vec<u8> = Vec::new();
    {
        let mut asmread: File = File::open("src/bench/arithmetic.s")?;
        asmread.read_to_end(&mut asm_bytes)?;
    }
    let asm_lines: Vec<String> = asm_bytes
        .iter()
        .map(|byte| (*byte as char).to_string())
        .collect::<Vec<String>>()
        .join("")
        .split("\n")
        .map(|string| string.to_string())
        .collect();

    {
        let mut wfile: File = File::create("out.obj")?;
        wfile.write(&build(asm_lines)?)?;
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
        bus.store(addr as u32, *byte as i32, 0b000);
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
        cpu.step();
        window
            .update_with_buffer(&vgatb.borrow_mut().tick(), WIDTH, HEIGHT)
            .unwrap();
    }

    Ok(())
}

fn build(lines: Vec<String>) -> Result<Vec<u8>, Box<dyn Error>> {
    let (mcode, data) = assemble(lines)?;

    Ok(newfile(data, mcode.clone())?)
}
/*fn setcharat(x: usize, y: usize, chid: usize, buffer: &mut Vec<u32>, from: &[u8]) {
    let cha: usize = chid * 16;
    for row in 0..16 {
        let byte = from[cha + row];
        println!("byte {} buffers {}", byte, row * WIDTH + y * WIDTH + x);
        for bitn in 0..8 {
            buffer[row * WIDTH + y * WIDTH + bitn + x] = if (byte >> (7 - bitn)) & 1 == 1 {
                0x00ff_ffff
            } else {
                0x0
            }
        }
    }
}
fn main() {
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let string = b"Hello, World!";
    for (i, ch) in string.iter().enumerate() {
        setcharat((i % 80) * 8, (i / 80) * 16, *ch as usize, &mut buffer, &vga);
    }
}*/
