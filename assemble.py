# from machine import RESET_VECTOR
from time import time
from termcolor import colored
import argparse
import glob
# from dataclasses import dataclass

import assembler

parser = argparse.ArgumentParser(
    prog="rv32i assembler",
    description="assembles rv32i instructions into riscv machine code.",
)

parser.add_argument("source")
parser.add_argument("destination")

cla = parser.parse_args()

# @dataclass
# class cla:
#     source: str = "testfiles/text.s"
#     destination: str = "test.rv"

if glob.glob(cla.source).__len__() > 1:
    exit(colored("Too many sources!", "red"))
elif glob.glob(cla.source).__len__() == 0:
    exit(colored("No source found!", "red"))

if glob.glob(cla.destination).__len__() > 1:
    exit(colored("Too many destinations!", "red"))

file: str = open(cla.source, "r").read()

print(
    f"    {colored('Assembling', 'green', attrs=['bold'])} {glob.glob(cla.source)[0].split('/')[-1]} \
({glob.glob(cla.source)[0]})")
s: float = time()
if True:
    fileout, errors = assembler.assemble(file)
    for error in errors:
        print(
            colored(f"error[E{'0' * (3 - len(str(error.code)))}{error.code}]",
                    "red",
                    attrs=["bold"]) +
            colored(f": {error.message}", attrs=["bold"]))
        print(
            colored(" --> ", "cyan", attrs=["bold"]) +
            glob.glob(cla.source)[0])
        print(
            colored(
                f"{' ' * (len(str(error.lineno)) + 1)}|\n{error.lineno} |     ",
                "cyan",
                attrs=["bold"]) + error.linet.strip(" ") +
            colored(f"\n{' ' * (len(str(error.lineno)) + 1)}|",
                    "cyan",
                    attrs=["bold"]),
            colored(f"\n{' ' * (len(str(error.lineno)) + 2)}{error.hint}",
                    attrs=["bold"]))
        print()

    if errors.__len__() > 0:
        print(
            colored("error", "red", attrs=["bold"]) + colored(
                f": aborting due to previous error{'s' if len(errors) > 1 else ''}.",
                attrs=["bold"]))

        exit()

# except Exception as e:
#     print(f"Error encountered while assembling!\n{e}")
#    exit()

print(
    f"    {colored(f'Finished', 'green', attrs=['bold'])} in {round(time()-s, 2)}s"
)

# print(colored("Saving...", "yellow"), end="\r", flush="True")
try:
    with open(cla.destination, "wb") as dest:
        dest.write(fileout)

except FileNotFoundError:
    # cannot put the file into the destination
    # ? backwards ik

    exit(colored("Destination path does not exist!", "red"))

except OSError as e:
    exit(colored(f"OSError when writing: {e}", "red"))

# print(colored(f"Saved to `{glob.glob(cla.destination)[0]}`", "green"))
