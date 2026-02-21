from devices import Region, Bus, UART, Memory, Display
from machine import Core, RESET_VECTOR
import pygame
import time, random

uart: UART = UART()
memory: Memory = Memory(2**20)
window = pygame.display.set_mode((512, 512))

display: Display = Display(window, 0x4c08)

devices: list[Bus.Device] = [
    Bus.Device(Region(0x00000, 0x0ffff), memory),
    Bus.Device(Region(0x10000, 0x10010), uart),
    Bus.Device(Region(0x10100, 0x14c08), display)
]

bus = Bus(devices)

processor = Core(bus)

image: bytes = b'\x93\x80\x80\x01#\x10\x10\x00s'

for i, byte in enumerate(image):
    print(byte, image, i + RESET_VECTOR)
    bus.store(i + RESET_VECTOR, byte, 0b000)

bus.store(0x10100, 1, 0b000)
bus.store(0x10101, 0, 0b000)
bus.store(0x10100 + devices[2].device.headeraddr.renderframe, 0xff, 0b000)

while True:
    try:
        processor.step()
    except:
        break

print(bus.devices[0].device.data[0:50])

while True:
    bus.devices[2].device.tick()
