use riscy::Error;
use riscy::args;
use riscy::{File, Read}; // Write
use riscy::{Rc, RefCell};

use riscy::{Window, WindowOptions};

use riscy::Bus;
use riscy::Core;
use riscy::VGATextBuffer;
use riscy::devices::StdMemory;
// use riscy::risclib::file_processor::newfile;
use riscy::{Device, DeviceOption};
use riscy::{HEIGHT, WIDTH};

use riscy::Trap;
// use riscy::assemble;
use riscy::elf;

/*fn trap<T>(result: Result<T, Trap>) -> Result<T, Box<dyn Error>> {
    // trap to stderror
    if !result.is_ok() {
        println!("!fault! {:?}", Trap::expose_err(&result.err().unwrap()));
        return Err(Box::new(std::fmt::Error));
    }

    Ok(result.ok().unwrap())
}*/

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = args().collect(); // args is a Vec<stirng> wrapper

    let mut asm: Vec<u8> = Vec::new();
    {
        let mut readfile: File = File::open(args[1].clone())?;
        readfile.read_to_end(&mut asm)?;
    }

    let mut stdmem: StdMemory = StdMemory::new();

    let entry = elf::ElfErr::into(elf::load(&asm.to_vec(), &mut stdmem))?;

    let vgatb: Rc<RefCell<VGATextBuffer>> = Rc::new(RefCell::new(VGATextBuffer::new()));

    let bus: Bus = Bus::new(vec![
        Device::new(0, 0xb8000, DeviceOption::StdMem(stdmem)),
        Device::new(
            0xb8000,
            0xb8000 + 4000,
            DeviceOption::VGATextBuffer(Rc::clone(&vgatb)),
        ),
    ]);

    let mut cpu: Core = Core::new(bus, entry);
    let mut window = Window::new(
        "emulator",
        WIDTH,
        HEIGHT,
        WindowOptions {
            scale: minifb::Scale::X2,
            ..WindowOptions::default()
        },
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    println!(
        "all setup passed:\n    PC: {:#012x}\n    ELFEntry: {:#012x}",
        cpu.pc, entry
    );

    while window.is_open() {
        let cpures: Result<(), Trap> = Ok(cpu.step());
        let vgabuf: Result<Vec<u32>, Trap> = vgatb.borrow_mut().tick();
        if !cpures.is_ok() {
            println!("!fault! {:?}", Trap::expose_err(&cpures.err().unwrap()));
            println!("states: {:?}", cpu.general_registers);
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

/*fn build_asm(lines: Vec<String>) -> Result<Vec<u8>, Box<dyn Error>> {
    let (mcode, data) = assemble(lines)?;

    Ok(newfile(data, mcode.clone())?)
}*/

/*fn build_env(
    data: &[u8],
    mcode: &[u8],
) -> Result<(Core, Rc<RefCell<VGATextBuffer>>), Box<dyn Error>> {
    let stdmem: StdMemory = StdMemory::new();
    let vgatb: Rc<RefCell<VGATextBuffer>> = Rc::new(RefCell::new(VGATextBuffer::new()));
    let mut bus: Bus = Bus::new(vec![
        Device::new(0, 0x1fff, DeviceOption::StdMem(stdmem)),
        Device::new(
            0xb8000,
            0xb8000 + 4000,
            DeviceOption::VGATextBuffer(Rc::clone(&vgatb)),
        ),
    ]);

    for (addr, byte) in data.iter().enumerate() {
        trap(bus.store(addr as u32, *byte as i32, 0b000))?;
        // println!("addr {}, byte {:x}", addr, byte);
        // println!("{}", trap(bus.load(addr as u32, 0b000))?);
    }

    for (addr, byte) in mcode.iter().enumerate() {
        trap(bus.store(addr as u32 + RESET_VECTOR, *byte as i32, 0b000))?;
    }

    let cpu: Core = Core::new(bus);

    Ok((cpu, vgatb))
}

fn run(readbuf: &[u8]) -> Result<(), Box<dyn Error>> {
    {
        let (data, mcode) = parsebin(&readbuf)?;

        let (mut cpu, vgatb) = build_env(data, mcode)?;

        let mut window = Window::new(
            "emulator",
            WIDTH,
            HEIGHT,
            WindowOptions {
                scale: minifb::Scale::X2,
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
                println!("states: {:?}", cpu.general_registers);
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
    }

    Ok(())
}*/

/*let src = args[1].clone(); // include dest?
let destv: Vec<&str> = src.split(".").collect();

println!("{:?}", destv);
let dest = destv[..destv.len() - 1].join("") + ".obj";

let mut asm: String = String::new();
{
    let mut asmread: File = File::open(src.as_str())?;
    asmread.read_to_string(&mut asm)?;
}
let asm_lines: Vec<String> = asm.lines().map(|v: &str| v.to_string()).collect();

{
    let mut wfile: File = File::create(&dest)?;
    wfile.write(&build_asm(asm_lines)?)?;
}

let mut readbuf: Vec<u8> = Vec::new();
{
    let mut readfile: File = File::open(&dest)?;
    readfile.read_to_end(&mut readbuf)?;
}*/
