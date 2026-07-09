# Cinnamon's RISC-V Emulator
A risc-v emulator currently for the `rv32im` ISA.
Each example section supplies both riscv assembly and rust

<hr>

## Devices
|Name          |Start Address|End Address |Summary            |
|--------------|-------------|------------|-------------------|
|GPRAM         |`0x00000000` |`0x000b8000`|General Purpose RAM|
|VGA-Textbuffer|`0x000b8000` |`0x000b8f9f`|Write to the screen|


### `GPRAM`
General Purpose RAM, use any of the `store`/`load` instructions to access

    Examples

```rust
    let arb = 0x1000 as *mut u8;
    *arb = 100;
```
```
    lui t0, 0x1
    addi t1, zero, 100
    sb t1, t0, 0 # or 0(t0)
```

### `VGA textbuffer`
Write text to the screen.
Every character is in `[colour][char]` pairs

```
 15      14        12 11         8 7         0
+-------+------------+------------+-----------+
| blink | background | foreground | character |
+-------+------------+------------+-----------+
```

With colours being
|low|colour    |+bright|bright colour|
|---|----------|-------|-------------|
|0x0|black     |0x8    |dark grey    |
|0x1|blue      |0x9    |light blue   |
|0x2|green     |0xa    |lime         |
|0x3|cyan      |0xb    |light cyan   |
|0x4|red       |0xc    |light red    |
|0x5|magenta   |0xd    |pink         |
|0x6|brown     |0xe    |yellow       |
|0x7|light grey|0xf    |white        |

And characters part of `CP437`
|  |X0|X1|X2|X3|X4|X5|X6|X7|X8|X9|Xa|Xb|Xc|Xd|Xe|Xf|
|--|--|--|--|--|--|--|--|--|--|--|--|--|--|--|--|--|
|0X|  |☺ |☻ |♥ |♦ |♣ |♠ |• |◘ |○ |◙ |♂ |♀ |♪ |♫ |☼ |
|1X|► |◄ |↕ |‼ |¶ |§ |▬ |↨ |↑ |↓ |→ |← |∟ |↔ |▲ |▼ |
|2X|sp|! |" |# |$ |% |& |' |( |) |* |+ |, |- |. |/ |
|3X|0 |1 |2 |3 |4 |5 |6 |7 |8 |9 |: |; |< |= |> |? |
|4X|@ |A |B |C |D |E |F |G |H |I |J |K |L |M |N |O |
|5X|P |Q |R |S |T |U |V |W |X |Y |Z |[ |\ |] |^ |_ |
|6X|` |a |b |c |d |e |f |g |h |i |j |k |l |m |n |o |
|7X|p |q |r |s |t |u |v |w |x |y |z |{ |\||} |~ |⌂ |
|8X|Ç |ü |é |â |ä |à |å |ç |ê |ë |è |ï |î |ì |Ä |Å |
|9X|É |æ |Æ |ô |ö |ò |û |ù |ÿ |Ö |Ü |¢ |£ |¥ |₧ |ƒ |
|aX|á |í |ó |ú |ñ |Ñ |ª |º |¿ |⌐ |¬ |½ |¼ |¡ |« |» |
|bX|░ |▒ |▓ |│ |┤ |╡ |╢ |╖ |╕ |╣ |║ |╗ |╝ |╜ |╛ |┐ |
|cX|└ |┴ |┬ |├ |─ |┼ |╞ |╟ |╚ |╔ |╩ |╦ |╠ |═ |╬ |╧ |
|dX|╨ |╤ |╥ |╙ |╘ |╒ |╓ |╫ |╪ |┘ |┌ |█ |▄ |▌ |▐ |▀ |
|eX|ɑ |ϐ |ᴦ |ᴨ |∑  |ơ |µ |ᴛ |ɸ |ϴ |Ω |ẟ |∞ |∅ |∈ |∩ |
|fX|≡ |± |≥ |≤ |⌠ |⌡ |÷ |≈ |° |∙ |· |√ |ⁿ |² |■ |  |

    Examples

```rust
    // Write 'H' at (10, 0) in colour LIGHT_CYAN
    let vga = 0xb8000 as *mut u8;

    // 2 bytes per screen char
    *vga.offset(20) = 0x48; // 0xb800a = 'H'
    *vga.offset(21) = 0xb; // 0xb800b = LIGHT_CYAN
```

```
    # Write '⌂' at (5, 0) in colour YELLOW
    lui t0, 0xb8
    addi t0, t0, 10 # 2 bytes per screen char

    addi t1, zero, 0x7f
    addi t2, zero, 0xe

    sb t1, t0, 0
    sb t2, t0, 1
```
