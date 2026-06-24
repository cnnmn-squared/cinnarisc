/*from enum import IntEnum


class TCause(IntEnum):
    INSTRUCTION_ADDRESS_MISALIGNED = 0x0
    INSTRUCTION_ACCESS_FAULT = 0x1
    ILLEGAL_INSTRUCTION = 0x2
    BREAKPOINT = 0x3

    LOAD_ADDRESS_MISALIGNED = 0x4
    LOAD_ACCESS_FAULT = 0x5

    STORE_ADDRESS_MISALIGNED = 0x6
    STORE_ACCESS_FAULT = 0x7

    # Environment Faults

    ENV_CALL_FROM_U = 0x8
    ENV_CALL_FROM_S = 0x9
    # 0xA reserved
    ENV_CALL_FROM_M = 0xB

    # Page faults
    INSTRUCTION_PAGE_FAULT = 0xC
    LOAD_PAGE_FAULT = 0xD
    # 0xE reserved
    STORE_PAGE_FAULT = 0xf

    # Misc
    DOUBLE_TRAP = 0x10
    # 0x11 reserved
    SOFTWARE_CHECK = 0x12
    HARDWARE_ERROR = 0x13

    # 0x14-0x17 reserved
    # 0x18-0x1f designated for custom
    DEVICE_FAULT = 0x18
    INVALID_DEVICE_REGION = 0x19

    # 0x20-0x2f reserved
    # 0x30-0x3f designated for custom
    # 0x40>> reserved


class Trap(Exception):

    def __init__(self, cause: TCause):  # noqa
        self.cause = cause


class FileProcessor:
    FILE_HEADER = b'RV32I'
    DATA_START = XLEN

    @staticmethod
    def partition(file: bytes) -> tuple[bytes, bytes]:
        # handles file checks, and partitioning into (data, program)

        if file[0:5] != FileProcessor.FILE_HEADER:
            raise Exception("invalid file type (safety)")

        # ! RV32I[align XLEN bytes - dsig][DATA_SIGNATURE][..data..][DATA_SIGNATURE][align XLEN - tsig][TEXT_SIGNATURE]
        # ! [program]
        # RV32I\xff[half, datastart][half, dataend][word, text_start]

        def intfb(byts: bytes) -> int:
            cbp: int = 0
            out: int = 0

            for byte in byts:
                out += (byte << cbp)

                cbp += 8

            return out

        datastart = intfb(file[5:7])
        dataend = intfb(file[7:9])
        textstart = intfb(file[9:13])

        return (file[datastart:dataend], file[textstart:])

    @staticmethod
    def newfile(data: bytes, text: bytes) -> bytes:
        obj: bytearray = bytearray(FileProcessor.FILE_HEADER)

        if data.__len__() > 0xffff - FileProcessor.DATA_START:
            raise OverflowError("size of data has a limit of 0xffff.")

        regionst = FileProcessor.DATA_START
        xlenbytes = XLEN // 8

        obj.extend(regionst.to_bytes(2, "little"))

        data_end: int = regionst + data.__len__()
        # print(data_end)

        obj.extend(data_end.to_bytes(2, "little"))

        tsaligned: int = data_end + (xlenbytes - (data_end % xlenbytes))
        # print(tsaligned)

        obj.extend(tsaligned.to_bytes(4, "little"))

        obj.extend([0 for _ in range(regionst - obj.__len__())])
        obj.extend(data)

        obj.extend([0 for _ in range(tsaligned - data_end)])
        obj.extend(text)
        """
RV32I[rstlo][rsthi][dendlo][dendhi][tsal0][tsal1][tsal2][tsal3]
[padding to len 32][...data...][more padding until textstart][text]
"""

        return obj
*/

pub mod file_processor {
    use std::error::Error;

    const FILEHEADER: &[u8; 5] = b"RV32I";
    const DATA_START: u32 = 0x20;
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

        /*println!("{}->{}, {}", data_start, data_end, text_start);*/

        return Ok((&obj[data_start..data_end], &obj[text_start..]));
    }
}
