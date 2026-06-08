from devices import Region, Bus, UART, Memory
from xlibx import Trap, TCause, FileProcessor
from machine import Core, RESET_VECTOR, CLOCK_SPEED
from config import WINDOW_SIZE
import pygame

file: bytes = open("test.rv", "rb").read()
data, text = FileProcessor.partition(file)


def load_text(text: bytes, bus: Bus) -> None:
    for i, byte in enumerate(text):
        bus.store(i + RESET_VECTOR, byte, 0b000)

    return


def load_data(data: bytes, bus: Bus, zero: int = 0) -> None:
    for i, byte in enumerate(data):
        bus.store(i + zero, byte, 0x000)

    return


def main(data: bytes, text: bytes) -> None:

    # devices/machine
    memory: Memory = Memory(2**20)
    uart: UART = UART()

    devices: list[Bus.Device] = [
        Bus.Device(Region(0x00000, 0x0ffff), memory),
        Bus.Device(Region(0x10000, 0x10010), uart)
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
            dt = clock.tick(CLOCK_SPEED)

            try:
                processor.step()

            except Trap as tcode:
                processor.dump()
                exit(f"trap: {tcode}")

        except KeyboardInterrupt:
            print()
            processor.dump()
            exit(f"keyboard interrupt")

        cycle += 1


main(data, text)
