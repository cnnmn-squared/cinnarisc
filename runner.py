from devices import Region, Bus, UART, Memory, Display
from xlibx import Trap, FileProcessor  # , TCause
from machine import Core, RESET_VECTOR, CLOCK_SPEED
# from config import WINDOW_SIZE
import os

os.environ['PYGAME_HIDE_SUPPORT_PROMPT'] = 'yellow'
import pygame  # noqa: E402

file: bytes = open("test.rv", "rb").read()
data, text = FileProcessor.partition(file)

screen: pygame.Surface = pygame.display.set_mode((480, 360))


def load_text(text: bytes, bus: Bus) -> None:
    for i, byte in enumerate(text):
        bus.store(i + RESET_VECTOR, byte, 0b000)

    return


def load_data(data: bytes, bus: Bus, zero: int = 0) -> None:
    for i, byte in enumerate(data):
        bus.store(i + zero, byte, 0x000)

    return


def main(data: bytes, text: bytes) -> None:
    memory: Memory = Memory(2**20)
    uart: UART = UART()
    display: Display = Display(screen, 0xAC10)

    devices: list[Bus.Device] = [
        Bus.Device(Region(0x00000, 0x0ffff), memory),
        Bus.Device(Region(0x10000, 0x10010), uart),
        Bus.Device(Region(0x11000, 0x1BC10), display),
    ]

    bus = Bus(devices)
    processor = Core(bus)

    # loading
    load_data(data, bus)
    load_text(text, bus)

    # emulation
    clock = pygame.time.Clock()
    cycle = 0

    while True:
        try:
            clock.tick(CLOCK_SPEED)

            try:
                processor.step()

            except Trap as tcode:
                processor.dump()
                exit(f"trap: {tcode}")

            # display.vram.sviab(100, 255)
            # bus.store(0x11000 + 128, 255, 0x00)
            display.tick()

        except KeyboardInterrupt:
            print()
            processor.dump()
            exit("keyboard interrupt")

        cycle += 1


main(data, text)
