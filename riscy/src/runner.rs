/*from devices import Region, Bus, UART, Memory, VGATextBuffer, Keyboard  # Display
from risclib import Trap, FileProcessor  # , TCause
from machine import Core, RESET_VECTOR
from config import CLOCK_SPEED
# from config import WINDOW_SIZE
import os

os.environ['PYGAME_HIDE_SUPPORT_PROMPT'] = 'yellow'
import pygame  # noqa: E402


def load_text(text: bytes, bus: Bus) -> None:
    for i, byte in enumerate(text):
        bus.store(i + RESET_VECTOR, byte, 0b000)

    return


def load_data(data: bytes, bus: Bus, zero: int = 0) -> None:
    for i, byte in enumerate(data):
        bus.store(i + zero, byte, 0x000)

    return


def run(src: str) -> None:
    file: bytes = open(src, "rb").read()
    data, text = FileProcessor.partition(file)

    screen: pygame.Surface = pygame.display.set_mode((640, 480))
    program_memory: Memory = Memory(2**16)
    uart: UART = UART()
    # display: Display = Display(screen, 0xAC10)
    vga_text: VGATextBuffer = VGATextBuffer(screen)
    keybr: Keyboard = Keyboard()
    ram: Memory = Memory(2**16)

    devices: list[Bus.Device] = [
        Bus.Device(Region(0x00000000, 0x0000ffff), program_memory),
        Bus.Device(Region(0x00010000, 0x00010010), uart),
        Bus.Device(Region(0x00010100, 0x000101ff), keybr),
        Bus.Device(Region(0x000B8000, 0x000C2000), vga_text),
        Bus.Device(Region(0x00020000, 0x0002ffff), ram),
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

            events = pygame.event.get()

            try:
                processor.step()

            except Trap as tcode:
                processor.dump()
                exit(f"trap: {tcode}")

            keybr.tick(events)
            vga_text.drawsc()

        except KeyboardInterrupt:
            print()
            processor.dump()
            exit("keyboard interrupt")

        clock.tick(CLOCK_SPEED)

        cycle += 1
*/