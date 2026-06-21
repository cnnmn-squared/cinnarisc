use std::cmp::min;

/*mod assembler;
mod devices;
mod machine;
mod risclib;
mod runner;

use assembler::assemble;
use devices::Memory;
use risclib::file_processor::newfile;
use std::error::Error;
use std::fs::File;
use std::io::Write;

use devices::Bus;
use devices::{Device, DeviceOption};
use machine::Core;

fn main() -> Result<(), Box<dyn Error>> {
    let lines: Vec<String> = vec![
        ".org 0x00001000".to_string(),
        ".section .data".to_string(),
        "byte abc = 250".to_string(),
        ".section .text".to_string(),
        "lui ra, 0x0001".to_string(),
    ];

    let (mcode, data) = assemble(lines)?;

    let file: Vec<u8> = newfile(data, mcode.clone())?;

    let mut wfile: File = File::create("out.obj")?;
    wfile.write(&file)?;

    let memory: Memory = Memory::new(0x2000);
    let mut bus: Bus = Bus::new(vec![Device::new(0, 0x1fff, DeviceOption::Memory(memory))]);

    for (addr, byte) in mcode.iter().enumerate() {
        bus.store(addr as u32, *byte as i32, 0b000);
    }

    let mut cpu: Core = Core::new(bus);

    cpu.step();
    Ok(())
}
*/
use minifb::{Scale::X2, Window, WindowOptions};

const WIDTH: usize = 640;
const HEIGHT: usize = 400;

fn setcharat(x: usize, y: usize, chid: usize, buffer: &mut Vec<u32>, from: &[u8]) {
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
    let vga: &[u8] = include_bytes!("resources/VGA8.F16");

    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];

    let string = b"Hello, World!";
    for (i, ch) in string.iter().enumerate() {
        setcharat((i % 80) * 8, (i / 80) * 16, *ch as usize, &mut buffer, &vga);
    }

    println!();
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
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    }
}
