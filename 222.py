from devices import Region, Bus, UART
import machine
import pygame
import time, random

uart: UART = UART()
memory: 

bus = Bus([Bus.Device(Region(0, 100), uart), Bus.Device(Region(0, 90), None)])
