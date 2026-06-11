from enum import IntEnum

XLEN: int = 32


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
