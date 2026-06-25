pub mod file_processor {
    use std::error::Error;

    const FILEHEADER: &[u8; 5] = b"RV32I";
    pub const DATA_START: u32 = 0x20;
    const XLEN: u32 = 32;

    pub fn newfile(data: Vec<u8>, text: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut obj: Vec<u8> = FILEHEADER.to_vec();

        if data.len() as u32 > 0xffff - DATA_START {
            // raise OverflowError("size of data has a limit of 0xffff.")
            panic!("size of data has a limit of 65kb.");
        }

        let regionst: u16 = DATA_START as u16;
        let xlenbytes: u32 = XLEN; // 8 //? why is eight here

        let dataend: u16 = regionst as u16 + data.len() as u16;
        let tsaligned: u32 = dataend as u32 + (xlenbytes - (dataend as u32 % xlenbytes));

        println!("{} {} {} {}", regionst, xlenbytes, dataend, tsaligned);

        obj.extend(regionst.to_le_bytes());
        obj.extend(dataend.to_le_bytes());
        obj.extend(tsaligned.to_le_bytes());

        obj.extend(vec![0; regionst as usize - obj.len()]);
        obj.extend(data);

        obj.extend(vec![0; (tsaligned - dataend as u32) as usize]);
        obj.extend(text);

        /*println!("{} -> {}, {}", regionst, dataend, tsaligned);
        println!(
            "{:?}, {:?}, {:?}",
            &regionst.to_le_bytes(),
            &dataend.to_le_bytes(),
            &tsaligned.to_le_bytes()
        );*/
        Ok(obj)
    }

    pub fn parsebin(obj: &[u8]) -> Result<(&[u8], &[u8]), Box<dyn Error>> {
        if &obj[0..5] != FILEHEADER {
            panic!("invalid file type (safety)")
        }

        /*println!("{:#?}", &obj[0..13]);*/
        let data_start = u16::from_le_bytes(obj[5..7].try_into()?) as usize;
        let data_end = u16::from_le_bytes(obj[7..9].try_into()?) as usize;
        let text_start = u32::from_le_bytes(obj[9..13].try_into()?) as usize;

        // println!("{:x}..{:x}, {:x}", data_start, data_end, text_start);
        // println!("{:?}", &obj[data_start..data_end]);

        return Ok((&obj[data_start..data_end], &obj[text_start..]));
    }
}

#[allow(non_camel_case_types)]
pub enum Trap {
    INSTRUCTION_ADDRESS_MISALIGNED, // 0x0
    INSTRUCTION_ACCESS_FAULT,       // 0x1
    ILLEGAL_INSTRUCTION,            // 0x2
    BREAKPOINT,                     // 0x3

    LOAD_ADDRESS_MISALIGNED, // 0x4
    LOAD_ACCESS_FAULT,       // 0x5

    STORE_ADDRESS_MISALIGNED, // 0x6
    STORE_ACCESS_FAULT,       // 0x7

    ENV_CALL_FROM_U, // 0x8
    ENV_CALL_FROM_S, // 0x9
    // 0xa reserved
    ENV_CALL_FROM_M, // 0xb

    INSTRUCTION_PAGE_FAULT, // 0x0c
    LOAD_PAGE_FAULT,        // 0x0d
    // 0xe reserved
    STORE_PAGE_FAULT, // 0x0f

    DOUBLE_TRAP, // 0x10
    // 0x11 reserveed
    SOFTWARE_CHECK, // 0x12
    HARDWARE_ERROR, // 0x13

    // 0x14 - 0x17
    // 0x18 - 0x1f private
    DEVICE_FAULT, // 0x18     PRIVATE
    INVALID_DEVICE_REGION, // 0x19     PRIVATE

                  // 0x20 - 0x2f reserved
                  // 0x30 - 0x3f private
                  // 0x40...  reserved
}

impl Trap {
    pub fn expose_err(&self) -> &str {
        match self {
            Trap::INSTRUCTION_ADDRESS_MISALIGNED => "INSTRUCTION_ADDRESS_MISALIGNED",

            Trap::INSTRUCTION_ACCESS_FAULT => "INSTRUCTION_ADDRESS_FAULT",
            Trap::ILLEGAL_INSTRUCTION => "ILLEGAL_INSTRUCTION",
            Trap::BREAKPOINT => "BREAKPOINT",

            Trap::LOAD_ADDRESS_MISALIGNED => "LOAD_ADDRESS_MISALIGNED",
            Trap::LOAD_ACCESS_FAULT => "LOAD_ACCESS_FAULT",

            Trap::STORE_ADDRESS_MISALIGNED => "STORE_ACCESS_MISALIGNED",
            Trap::STORE_ACCESS_FAULT => "STORE_ACCESS_FAULT",

            Trap::ENV_CALL_FROM_U => "ENVIRONMENT CALL (USER)",
            Trap::ENV_CALL_FROM_S => "ENVIRONMENT CALL (SUPERVISOR)",
            Trap::ENV_CALL_FROM_M => "ENVIRONMENT CALL (MACHINE)",

            Trap::INSTRUCTION_PAGE_FAULT => "INSTRUCTION_PAGE_FAULT",
            Trap::LOAD_PAGE_FAULT => "LOAD_PAGE_FAULT",
            Trap::STORE_PAGE_FAULT => "STORE_PAGE_FAULT",

            Trap::DOUBLE_TRAP => "DOUBLE_TRAP",
            Trap::SOFTWARE_CHECK => "SOFTWARE_CHECK",
            Trap::HARDWARE_ERROR => "HARDWARE_ERROR",

            Trap::DEVICE_FAULT => "private DEVICE_FAULT",
            Trap::INVALID_DEVICE_REGION => "private INVALID_DEVICE_REGION",
        }
    }
}

/*enum LoadErr {
    // Later possible type, error occurred while loading, eg. RegionError
}*/
