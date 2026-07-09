#![no_std]

pub mod colours {
    // BACKGROUND only has the low colours (0-7)
    // FOREGROUND has low and high (0-f)

    pub const BLACK: u8 = 0x0;
    pub const BLUE: u8 = 0x1;
    pub const GREEN: u8 = 0x2;
    pub const CYAN: u8 = 0x3;
    pub const RED: u8 = 0x4;
    pub const MAGENTA: u8 = 0x5;
    pub const BROWN: u8 = 0x6;
    pub const LGREY: u8 = 0x7;

    pub const DGREY: u8 = 0x8;
    pub const LBLUE: u8 = 0x9;
    pub const LIME: u8 = 0xa;
    pub const LCYAN: u8 = 0xb;
    pub const LRED: u8 = 0xc;
    pub const PINK: u8 = 0xd;
    pub const YELLOW: u8 = 0xe;
    pub const WHITE: u8 = 0xf;
}

pub fn draw_char(to: (u16, u16), chr: char, attr: (u8, u8)) {
    let ptrbase: *mut u16 =
        (0xb8000u32 + ((80 * to.1 as u32 * 2) + to.0 as u32 * 2u32)) as *mut u16;

    unsafe { *ptrbase = ((((attr.1 << 4) | attr.0) as u16) << 8) | chr as u16 }
}

pub fn print(string: &str, row: u16, col: u16, attr: (u8, u8)) {
    let string = string.as_bytes();
    let mut wrap: u8 = 0;
    for i in 0..string.len() as u16 {
        if (i + col) % 80 == 0 && i != 0 {
            wrap += 1
        }

        draw_char(
            ((col + i) % 80, row + wrap as u16),
            string[i as usize] as char,
            attr,
        );
    }
}
