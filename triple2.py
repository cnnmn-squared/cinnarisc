import devices
import machine
import pygame
import time, random
# import riscv_assembler

informatron = bytearray(b'\x93\x80\x80\x01#\x10\x10\x00')
glblmem = machine.Memory(0x10ff)
for i, byte in enumerate(informatron):
    glblmem.sviab(i + 0x1000, byte)

processor = machine.Core(glblmem)

processor.step()
processor.step()
print(list(glblmem.data[:0xff]))

clock = pygame.time.Clock()

WIDTH = 160
HEIGHT = 120

shmems = machine.Memory(160 * 120 * 3 + 8)

print("loading file...")
file = open("video.o", "rb")

window = pygame.display.set_mode((512, 512))

print("device load start")
displ = devices.Display(window, shmems, 0, 160 * 120 * 3 + 8)
print("device load finish")

while True:
    displ.tick()
exit()
cycles = 0
print("loooop")

shmems.sviab(displ.headeraddr.rendermode, 1)

shmems.sviab(displ.headeraddr.connection, 1)


def write_chunk(memsh: machine.Memory) -> None:
    video_entry_point = displ.HEADERSIZE

    segment: bytes = file.read(WIDTH * HEIGHT)

    memptr: int = 0
    for byte in segment:
        memsh.sviab(video_entry_point + memptr, byte)
        memptr += 1


while True:

    dt = clock.tick(60)
    shmems.sviab(displ.headeraddr.renderframe, 1)
    displ.tick()

    write_chunk(shmems)

    cycles += 1
