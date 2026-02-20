import pygame
from machine import Memory, Trap, TCause
from typing import Self
from dataclasses import dataclass


@dataclass
class Region:
    start: int
    end: int


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
        sizex: int
        sizey: int
        renderframe: int

    def __init__(
        self,
        display: pygame.Surface,
        memsh: Memory,
        regst: int,
        regend: int,
    ) -> None:
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
        :param memsh: shared memory
        :type memsh: Memory
        :param regst: region start (memory location)
        :type regst: int
        :param regend: region end (always > regst + 6)
        :type regend: int
        """

        self.HEADERSIZE = 8  # bytes

        self.DEFAULTSCR = (160, 120)

        self.alloc_start = regst
        self.alloc_end = regend
        self.allocregion = (regst, regend)  # regend safety fallback
        self.framebuffer_region = (regst + self.HEADERSIZE, regend)

        if regend < (regst + 160 * 120) + self.HEADERSIZE:
            raise Trap(TCause.INVALID_DEVICE_REGION)

        if regend > memsh.data.__len__():
            raise Trap(TCause.INVALID_DEVICE_REGION)

        self.memsh: Memory = memsh
        self.rcycle: int = 0
        self.resx: int = self.DEFAULTSCR[0]
        self.resy: int = self.DEFAULTSCR[1]
        self.static_resy = self.resy

        self.rendermode: int = 0x00

        self.headeraddr = self.HeaderAddresses(
            self.alloc_start + 0, self.alloc_start + 1, self.alloc_start + 2,
            self.alloc_start + 4, self.alloc_start + 6
        )
        # b_connection:0 b_rendermode:1 h_sizex:2(3) h_sizey:4(5) b_renderf:6 fill

        self.connection_conf = False
        self.renderf = False

        self.horizlszm: int = self.resx * 3 if self.rendermode == 0x02 else self.resx
        self.static_horizlszm: int = self.horizlszm
        # (horiz)ontal l(ine) (s)i(z)e in (m)emory

        pygame.init()
        self.parentdisplay = display
        self.screen = pygame.Surface((self.resx, self.resy))
        self.reiter = 0

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
        self.connection_conf = True if self.memsh.lvfab(
            self.alloc_start
        ) == 1 else False

        self.reiter += 1
        if not self.connection_conf:

            self.rendermode = self.memsh.lvfab(self.headeraddr.rendermode)

            resxc = self.memsh.load_half(self.headeraddr.sizex)
            resyc = self.memsh.load_half(self.headeraddr.sizey)
            self.resx = resxc if resxc != 0 else self.DEFAULTSCR[0]
            self.resy = resyc if resyc != 0 else self.DEFAULTSCR[1]
            self.horizlszm = self.resx * 3 if self.rendermode == 0x02 else self.resx

            self.screen = pygame.Surface((self.resx, self.resy))

            self.parentdisplay.blit(self.NOCONNECTION, (0, 0))

            if self.reiter % 256 == 0:
                self.render(self.NOCONNECTION)
            return

        self.renderf = True if self.memsh.lvfab(
            self.headeraddr.renderframe
        ) == 1 else False

        self.horizlszm = self.resx * 3 if self.rendermode == 0x02 else self.resx

        secst = self.framebuffer_region[0] + self.static_horizlszm * self.rcycle
        section: bytes = self.memsh.data[secst:secst + self.horizlszm]
        #print(section)

        sfp: int = 0
        chp: int = 0
        slen: int = section.__len__()

        while sfp < slen:
            match self.rendermode:
                case 0x00:
                    # greyscale
                    self.rm0_greysc(section[sfp], chp, self.rcycle)
                    sfp += 1

                case 0x01:
                    # palette
                    self.rm1_pal(section[sfp], chp, self.rcycle)
                    sfp += 1

                case 0x02:
                    # rgb24
                    self.rm2_rgb24((section[sfp], section[sfp + 1], section[sfp + 2]),
                                   chp, self.rcycle)
                    sfp += 3

            chp += 1

        #print(self.rendermode)
        #self.rendermode = self.memsh.lvfab(self.headeraddr.rendermode)
        #print(self.rendermode)

        #print(self.renderf)
        if self.renderf == True or self.renderf == 1:
            #print("render")
            self.render(self.screen)

            self.memsh.sviab(self.headeraddr.renderframe, 0)
            self.rendermode = self.memsh.lvfab(self.headeraddr.rendermode)

        self.rcycle += 1
        if self.rcycle > self.static_resy:
            self.rcycle = 0

            self.static_horizlszm = self.horizlszm  # so resolution doesnt change until full frame complete
            self.static_resy = self.resy  # so it wont infinitely continue
            self.rendermode = self.memsh.lvfab(self.headeraddr.rendermode)

            self.screen = pygame.transform.scale(self.screen, (self.resx, self.resy))

    def render(self, tbsc: pygame.Surface) -> None:
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


class UART:
    pass


class Bus:

    @dataclass
    class Device:
        region: Region
        device: UART | Memory | None

    def __init__(self, devices: list[Device]) -> None:
        # collision scan
        regions = [device.region for device in devices]

        for pregion in regions:
            for cregion in regions:
                if (cregion.start > pregion.start
                        and cregion.start < pregion.end) or (cregion.end < pregion.end):
                    print(f"failure {cregion} {pregion}")

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

        raise Trap(TCause.LOAD_ACCESS_FAULT)

    def store(self, addr: int, value: int) -> None:
        pass
