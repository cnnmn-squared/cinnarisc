import pygame
from xlibx import Trap, TCause, XLEN
from config import RESOLUTION, WINDOW_SIZE
from typing import Self
from random import randbytes
from dataclasses import dataclass


@dataclass
class Region:
    start: int
    end: int


class Memory:

    def __init__(self, size: int) -> None:
        if size > 2**XLEN:
            raise Exception(f"`size` is too large! limit is 2^{XLEN}, size({size})")

        self.data = bytearray(size)

    def _addrsafe(self, addr: int) -> None:
        if addr < 0:
            raise Trap(TCause.HARDWARE_ERROR)

    def _addrbounds(self, addr: int) -> int:
        return addr % self.data.__len__()

    def lvfab(self, address: int) -> int:
        return self.data[address]

    def sviab(self, address: int, value: int) -> None:
        if value > 0xff:
            raise Trap(TCause.STORE_ACCESS_FAULT)
        self.data[address] = value

    def load_half(self, address: int) -> int:
        if address % 2 != 0:
            raise Trap(TCause.LOAD_ADDRESS_MISALIGNED)

        return (self.lvfab(address + 1) << 8) + self.lvfab(address)

    def load_word(self, address: int) -> int:
        if address % 4 != 0:
            raise Trap(TCause.LOAD_ADDRESS_MISALIGNED)

        return (self.data[address + 3] << 24) + (self.data[
            address + 2] << 16) + (self.data[address + 1] << 8) + self.data[address]

    def store_half(self, address: int, value: int) -> None:
        if address % 2 != 0:
            raise Trap(TCause.STORE_ADDRESS_MISALIGNED)

        self.data[address] = value & 0xff
        self.data[address + 1] = (value >> 8) & 0xff

    def store_word(self, address: int, value: int) -> None:
        if address % 4 != 0:
            raise Trap(TCause.STORE_ADDRESS_MISALIGNED)

        self.data[address] = value & 0xff
        self.data[address + 1] = (value >> 8) & 0xff
        self.data[address + 2] = (value >> 16) & 0xff
        self.data[address + 3] = (value >> 24) & 0xff


class Display:

    @staticmethod
    def rwi_to_surface(rwibytes: bytes) -> pygame.Surface:
        grey: bool = True if rwibytes[0] == 0 else False

        newsur = pygame.Surface((160, 120))

        for i, pxl in enumerate(rwibytes[1:]):
            if grey:
                newsur.set_at((i % 160, i // 160), (pxl, pxl, pxl))
            else:
                newsur.set_at((i % 160, i // 160), ((pxl & 0b111) * 255 // 7,
                                                    ((pxl >> 3) & 0b111) * 255 // 7,
                                                    ((pxl >> 6) & 0b11) * 255 // 3))

        return newsur

    NOCONNECTION: pygame.Surface = rwi_to_surface(open("nocon.rwi", "rb").read())

    @dataclass
    class HeaderAddresses:
        connection: int
        rendermode: int
        flags: int
        framebuffr: int

    def __init__(self, display: pygame.Surface, size: int) -> None:
        """
        memory segmentation:
        Display does not use a bus interface, magic addresses are used.
        `[header(enable)][header(rendermode)][header(sizex_lo)][header(sizex_hi)][header(sizey_lo)][header(sizey_hi)][header(renderframe)][FILLER][framedata...]`

        if `regend` is too low to fit all the data, or exceeds memory size, a trap of `INVALID_DEVICE_REGION` will be raised

        rendermodes:
            `0x00` greyscale (8-bit)
            `0x01` palette (8-bit, 3-3-2)
            `0x02` rgb24 (24-bits, 3 bytes)

        :param display: parent display to be blit on
        :type display: pygame.Surface
        :param vram: shared memory
        :type vram: Memory
        :param regst: region start (memory location)
        :type regst: int
        :param regend: region end (always > regst + 6)
        :type regend: int
        """

        self.HEADERSIZE = 8  # bytes

        self.alloc_end = size
        self.framebuffer_region = (self.HEADERSIZE, size)

        if size < (160 * 120) + self.HEADERSIZE:
            raise Trap(TCause.INVALID_DEVICE_REGION)

        self.vram: Memory = Memory(size)  # 4c08
        self.resx: int = RESOLUTION[0]
        self.resy: int = RESOLUTION[1]

        self.rendermode: int = 0x00

        self.haddrs = self.HeaderAddresses(0, 1, 2, 16)

        self.vblank: bool = False
        self.hblank: bool = False

        self.clock: int = 0

        self.vblanking: int = 0
        self.hblanking: int = 0

        self.pxlsize: int = 1

        pygame.init()
        self.parentdisplay = display
        self.screen = pygame.Surface((self.resx, self.resy))

    @staticmethod
    def autopalette(value: int) -> tuple[int, int, int]:
        # 3-3-2
        rbits = value & 0b111
        gbits = value >> 3 & 0b111
        bbits = value >> 6 & 0b11

        rval = (rbits * 255) // 7
        gval = (gbits * 255) // 7
        bval = (bbits * 255) // 3

        return (rval, gval, bval)

    def tick(self) -> None:
        for event in pygame.event.get():
            print(event.type)  # pygame sideeffect

        if self.clock == 0:
            if self.vblanking < 16:
                self.vram.sviab(
                    self.haddrs.flags, self.vram.data[self.haddrs.flags] | 1
                )
                self.vblanking += 1
            else:
                self.vblanking = 0
                self.vram.sviab(
                    self.haddrs.flags, self.vram.data[self.haddrs.flags] & 0xfe
                )

                self.render(self.screen)

        if self.vram.lvfab(0) != 0xff:
            self.rendermode = self.vram.lvfab(self.haddrs.rendermode)
            self.pxlsize = 3 if self.rendermode == 0x02 else 1

        scrs: int = (self.resx * self.clock + self.haddrs.framebuffr) * self.pxlsize

        scanline = self.vram.data[scrs:scrs + self.resx * self.pxlsize]

        h = 0
        print(self.clock, scanline)
        while h < len(scanline):
            match self.rendermode:
                case 0x00:
                    self.rm0_greysc(scanline[h], h, self.clock)

                case 0x01:
                    self.rm1_pal(scanline[h], h, self.clock)

                case 0x02:
                    segment = scanline[h:h + self.pxlsize]
                    self.rm2_rgb24((segment[0], segment[1], segment[2]), h, self.clock)

            h += self.pxlsize

        self.clock += 1
        self.clock %= self.resy

    def render(self, tbsc: pygame.Surface) -> None:
        print("render", self.vram.data[16:255])
        scscr = pygame.transform.scale(tbsc, self.parentdisplay.get_size())
        self.parentdisplay.blit(scscr, (0, 0))
        pygame.display.flip()

    def rm0_greysc(self, value: int, x: int, y: int) -> None:
        """
        :param value: greyscale value
        :type value: int
        """
        vint = int(value)
        self.screen.set_at((x, y), (vint, vint, vint))

    def rm1_pal(self, value: int, x: int, y: int) -> None:
        """
        :param value: palette value
        :type value: int
        """
        vint = int(value)
        self.screen.set_at((x, y), self.autopalette(vint))
        # print(self.autopalette(vint))

    def rm2_rgb24(self, next_3: tuple[int, int, int], x: int, y: int) -> None:
        """
        :param next_3: 24 bit colour mode, next 3 bytes (r,g,b)
        :type next_3: bytes
        """
        self.screen.set_at((x, y), (next_3[0], next_3[1], next_3[2]))


class Random:

    @staticmethod
    def load(size: int) -> int:
        byte_s = randbytes(
            1 if size == 0 else (2 if size == 1 else (4 if size == 2 else -1))
        )
        mus = 0

        for i, byte in enumerate(byte_s):
            mus += byte << i * 8

        return mus


class UART:

    def __init__(self) -> None:
        pass

    def transmit(self, data: int) -> None:
        print(chr(data & 0x10ffff))


class Bus:

    @dataclass
    class Device:
        region: Region
        device: UART | Memory | Display | Random | None

    def __init__(self, devices: list[Device]) -> None:
        # collision scan
        regions = [device.region for device in devices]

        # pregion end > cregion start
        # pregion start < cregion end
        for i, pregion in enumerate(regions):
            for j, cregion in enumerate(regions):
                if i == j:
                    continue
                if (pregion.end > cregion.start) and (pregion.start < cregion.end):
                    raise Exception(f"region collision between: {cregion} {pregion}")

        self.devices = devices

    def _fdra(self, addr: int) -> Device:
        for device in self.devices:
            if addr >= device.region.start and addr <= device.region.end:
                return device

        raise Trap(TCause.INVALID_DEVICE_REGION)

    def load(self, addr: int, information: int) -> int:
        condev = self._fdra(addr)

        match condev.device:
            case UART():
                raise Trap(TCause.LOAD_ACCESS_FAULT)

            case Memory():
                load_type = information

                #print(load_type, addr)
                #print(condev.device.data[0x1000:0x1010])

                if load_type == 0:
                    return condev.device.lvfab(addr)
                elif load_type == 1:
                    return condev.device.load_half(addr)
                elif load_type == 2:
                    return condev.device.load_word(addr)

                elif load_type == 4:
                    return condev.device.lvfab(addr) << 8
                elif load_type == 5:
                    return condev.device.load_half(addr) << 16

            case Random():
                randomlen = information

                return condev.device.load(randomlen)

        raise Trap(TCause.LOAD_ACCESS_FAULT)

    def store(self, addr: int, value: int, information: int) -> None:
        condev = self._fdra(addr)

        match condev.device:
            case UART():
                condev.device.transmit(value)

            case Memory():
                load_type = information

                if load_type == 0b000:
                    condev.device.sviab(addr, value & 0xff)
                elif load_type == 0b001:
                    condev.device.store_half(addr, value & 0xffff)
                elif load_type == 0b010:
                    condev.device.store_word(addr, value & 0xffff_ffff)

                return

            case Display():
                store_type = information

                if store_type == 0b000:
                    condev.device.vram.sviab(addr - condev.region.start, value)
                elif store_type == 0b001:
                    condev.device.vram.store_half(addr - condev.region.start, value)
                elif store_type == 0b010:
                    condev.device.vram.store_word(addr - condev.region.start, value)

                return

        print(condev.device)
        raise Trap(TCause.STORE_ACCESS_FAULT)
