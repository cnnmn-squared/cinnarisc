# from machine import RESET_VECTOR
from time import time
from termcolor import colored
from runner import run as cpurun
import argparse
import glob
# from dataclasses import dataclass

import assembler

parser = argparse.ArgumentParser(
    prog="rv32i assembler",
    description="assembles rv32i instructions into riscv machine code.",
)

parser.add_argument("function")
parser.add_argument("source")
parser.add_argument("-dp", default=False, action="store_true")

cla = parser.parse_args()


def assemble(src: str) -> tuple[bytes, assembler.Assembly]:
    file: str = open(src, "r").read()
    print(
        f"    {colored('Assembling', 'green', attrs=['bold'])} {glob.glob(src)[0].split('/')[-1]} \
    ({glob.glob(src)[0]})")
    s: float = time()

    fileout, errors, assembly = assembler.assemble(file)
    for error in errors:
        print(
            colored(f"error[E{'0' * (3 - len(str(error.code)))}{error.code}]",
                    "red",
                    attrs=["bold"]) +
            colored(f": {error.message}", attrs=["bold"]))
        print(colored(" --> ", "cyan", attrs=["bold"]) + glob.glob(src)[0])
        print(
            colored(
                f"{' ' * (len(str(error.line.lineno)) + 1)}|\n{error.line.lineno} |     ",
                "cyan",
                attrs=["bold"]) + error.line.text.strip(" ") +
            colored(f"\n{' ' * (len(str(error.line.lineno)) + 1)}|",
                    "cyan",
                    attrs=["bold"]),
            colored(f"\n{' ' * (len(str(error.line.lineno)) + 2)}{error.hint}",
                    attrs=["bold"]))
        print()

    if errors.__len__() > 0:
        print(
            colored("error", "red", attrs=["bold"]) + colored(
                f": aborting due to previous error{'s' if len(errors) > 1 else ''}.",
                attrs=["bold"]))

        exit()

    print(
        f"    {colored(f'Finished', 'green', attrs=['bold'])} in {round(time()-s, 2)}s\n"
    )

    return fileout, assembly


def writefile(inf: bytes, dest: str) -> None:
    try:
        with open(dest, "wb") as file:
            file.write(inf)
    except FileNotFoundError:
        # cannot put the file into the destination
        # ? backwards ik

        exit(colored("Destination path does not exist!", "red"))

    except OSError as e:
        exit(colored(f"OSError when writing: {e}", "red"))

    return


def build(src: str, filename: str) -> None:
    obj, assembly = assemble(src)
    if cla.dp:
        with open(".".join(filename.split(".")[:-1]) + ".ras", "w") as file:
            file.write("\n".join(instr.assemble
                                 for instr in assembly.instructions))

    writefile(obj, ".".join(filename.split(".")[:-1]) + ".obj")


def run(src: str) -> None:
    print(
        f"{colored('Running', 'green', attrs=['bold'])} {src.split('/')[-1].split('.')[0]} ({src})\n\n"
    )
    cpurun(src)


def main(function: str, src: str) -> None:
    srcg = glob.glob(src)[0]
    filename = srcg.split("/")[-1]
    match function:
        case "build":
            build(src, filename)

        case "run":
            build(src, filename)
            objpath = ".".join(srcg.split(".")[:-1]) + ".obj"
            # print(objpath)
            run(objpath)


main(cla.function, cla.source)
