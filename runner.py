from devices import Region, Bus, UART, Memory
from xlibx import Trap, TCause
from machine import Core, RESET_VECTOR
from config import WINDOW_SIZE
import pygame

memory: Memory = Memory(2**20)

devices: list[Bus.Device] = [
    Bus.Device(Region(0x00000, 0x0ffff), memory),
]

bus = Bus(devices)

processor = Core(bus)

image: bytes = open("prog.img", "rb").read()

for i, byte in enumerate(image):
    bus.store(i + RESET_VECTOR, byte, 0b000)

clock = pygame.time.Clock()
cycle = 0
while True:
    dt = clock.tick(60)

    try:
        processor.step()

    except Trap as tcode:
        exit(f"trap: {tcode}")

    cycle += 1
