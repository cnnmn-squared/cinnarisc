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
    DATA_SIGNATURE = b'rvdata'
    TEXT_SIGNATURE = b'rvtext'
    FILE_HEADER = b'RV32I'

    @staticmethod
    def partition(file: bytes) -> tuple[bytes, bytes]:
        # handles file checks, and partitioning into (data, program)

        if file[0:5] != FileProcessor.FILE_HEADER:
            raise Exception("invalid file type (safety)")

        # RV32I[align XLEN bytes - dsig][DATA_SIGNATURE][..data..][DATA_SIGNATURE][align XLEN - tsig][TEXT_SIGNATURE][program]
        # RV32I\xff[half, datastart][half, dataend][word, text_start]

        fp: int = FileProcessor.FILE_HEADER.__len__()

        while file[fp:fp + FileProcessor.DATA_SIGNATURE.__len__(
        )] != FileProcessor.DATA_SIGNATURE:
            fp += 1

        fp += FileProcessor.DATA_SIGNATURE.__len__()

        assert fp % (XLEN // 8) == 0

        data: bytes = file[fp:file.index(FileProcessor.DATA_SIGNATURE, fp)]
        print(data)
        fp += len(data)

        while file[fp:fp + FileProcessor.TEXT_SIGNATURE.__len__(
        )] != FileProcessor.TEXT_SIGNATURE:
            fp += 1

        fp += FileProcessor.TEXT_SIGNATURE.__len__()
        print(fp)

        assert fp % (XLEN // 8) == 0

        return (data, file[fp:])


FileProcessor.partition(b'RV32I___**rvdata76543210rvdata**rvtextbasilbasilbasilbasil')
