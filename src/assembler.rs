use std::{collections::HashMap, error::Error};

const DEBUG: bool = false;

struct Symbol {
    symbol: String,
    at: u32,
}

impl Symbol {
    fn new(symbol: &str, at: u32) -> Symbol {
        Symbol {
            symbol: symbol.to_string(),
            at: at,
        }
    }
}

#[derive(Clone, Debug)]
struct Line {
    text: String,
    lineno: i32,
}

impl Line {
    fn new(text: &str, lineno: i32) -> Line {
        Line {
            text: text.to_string(),
            lineno: lineno,
        }
    }
}

#[derive(Debug)]
struct Instruction {
    assemble: String,
    line: Line,
}

impl Instruction {
    fn new(assemble: &str, line: Line) -> Instruction {
        Instruction {
            assemble: assemble.to_string(),
            line,
        }
    }
}

struct Assembly {
    _origin: u32,

    instructions: Vec<Instruction>,
    data_region: Vec<u8>,
}

pub fn assemble(lines: Vec<String>) -> Result<(Vec<u8>, Vec<u8>), Box<dyn Error>> {
    let (opcodes, fn3) = build_hashmaps();

    let assembly: Assembly = preprocess(lines)?;
    let mcode: Vec<u8> = assemble_instructions(assembly.instructions, opcodes, fn3)?;

    Ok((mcode, assembly.data_region))
}

fn int_from_any(sint: &str) -> i32 {
    if sint.len() < 2 {
        // must be decimal 1-10
        return u32::from_str_radix(sint, 10).expect("wrong radix for decimal") as i32;
    }
    let prefix: &str = &sint[0..2];
    let strint: &str = &sint[2..];

    println!("{} ({}){}", sint, prefix, strint);

    match prefix {
        "0x" => u32::from_str_radix(strint, 16).expect("wrong radix for 0x") as i32,
        "0b" => u32::from_str_radix(strint, 2).expect("wrong radix for 0b") as i32,
        "0o" => u32::from_str_radix(strint, 8).expect("wrong radix for 0o") as i32,
        _ => u32::from_str_radix(sint, 10).expect("wrong radix for decimal") as i32,
    }
}

fn build_hashmaps() -> (HashMap<Vec<String>, u8>, HashMap<Vec<String>, u8>) {
    let mut opcodes: HashMap<Vec<String>, u8> = HashMap::new();
    let mut fn3: HashMap<Vec<String>, u8> = HashMap::new();

    opcodes.insert(vec!["lui".to_string()], 0b0110111);
    opcodes.insert(vec!["auipc".to_string()], 0b0010111);
    opcodes.insert(vec!["jal".to_string()], 0b1101111);
    opcodes.insert(vec!["jalr".to_string()], 0b1100111);
    opcodes.insert(
        vec![
            "beq".to_string(),
            "bne".to_string(),
            "blt".to_string(),
            "bge".to_string(),
            "bltu".to_string(),
            "bgeu".to_string(),
        ],
        0b1100011,
    );
    opcodes.insert(
        vec![
            "lb".to_string(),
            "lh".to_string(),
            "lw".to_string(),
            "lbu".to_string(),
            "lhu".to_string(),
        ],
        0b0000011,
    );
    opcodes.insert(
        vec!["sb".to_string(), "sh".to_string(), "sw".to_string()],
        0b0100011,
    );
    opcodes.insert(
        vec![
            "addi".to_string(),
            "slti".to_string(),
            "sltiu".to_string(),
            "xori".to_string(),
            "ori".to_string(),
            "andi".to_string(),
            "srli".to_string(),
            "slli".to_string(),
            "srai".to_string(),
        ],
        0b0010011,
    );
    opcodes.insert(
        vec![
            "add".to_string(),
            "sub".to_string(),
            "sll".to_string(),
            "slt".to_string(),
            "sltu".to_string(),
            "xor".to_string(),
            "srl".to_string(),
            "sra".to_string(),
            "or".to_string(),
            "and".to_string(),
            // Ext:M
            "mul".to_string(),
            "mulh".to_string(),
            "mulhsu".to_string(),
            "mulhu".to_string(),
            "div".to_string(),
            "divu".to_string(),
            "rem".to_string(),
            "remu".to_string(),
        ],
        0b0110011,
    );
    opcodes.insert(
        vec![
            "fence".to_string(),
            "fence.tso".to_string(),
            "pause".to_string(),
        ],
        0b0001111,
    );
    opcodes.insert(vec!["ebreak".to_string(), "ecall".to_string()], 0b1110011);

    fn3.insert(
        vec![
            "jalr".to_string(),
            "beq".to_string(),
            "lb".to_string(),
            "sb".to_string(),
            "addi".to_string(),
            "add".to_string(),
            "sub".to_string(),
            "fence".to_string(),
            "fence.tso".to_string(),
            "pause".to_string(),
            "ecall".to_string(),
            "ebreak".to_string(),
            "mul".to_string(), // Ext:M
        ],
        0b000,
    );
    fn3.insert(
        vec![
            "bne".to_string(),
            "lh".to_string(),
            "sh".to_string(),
            "slli".to_string(),
            "sll".to_string(),
            "mulh".to_string(), // Ext:M
        ],
        0b001,
    );
    fn3.insert(
        vec![
            "lw".to_string(),
            "sw".to_string(),
            "slti".to_string(),
            "slt".to_string(),
            "mulhsu".to_string(), // Ext:M
        ],
        0b010,
    );
    fn3.insert(
        vec![
            "sltiu".to_string(),
            "sltu".to_string(),
            "mulhu".to_string(), /* Ext:M */
        ],
        0b011,
    );

    fn3.insert(
        vec![
            "blt".to_string(),
            "lbu".to_string(),
            "xori".to_string(),
            "xor".to_string(),
            "div".to_string(), // Ext:M
        ],
        0b100,
    );
    fn3.insert(
        vec![
            "bge".to_string(),
            "lhu".to_string(),
            "srli".to_string(),
            "srai".to_string(),
            "srl".to_string(),
            "sra".to_string(),
            "divu".to_string(), // Ext:M
        ],
        0b101,
    );
    fn3.insert(
        vec![
            "bltu".to_string(),
            "ori".to_string(),
            "rem".to_string(), /* Ext:M */
        ],
        0b110,
    );
    fn3.insert(
        vec![
            "bgeu".to_string(),
            "andi".to_string(),
            "and".to_string(),
            "remu".to_string(), // Ext:M
        ],
        0b111,
    );

    // EXTENSIONS (when separated from regular)

    return (opcodes, fn3);
}

fn tokenise_line(line: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut token: String = String::new();

    let mut within_string: bool = false;
    for ch in line.chars() {
        match ch {
            '"' => within_string = !within_string,
            ' ' => {
                if !within_string {
                    tokens.push(token.clone());
                    token = String::new();
                }
            }
            _ => token.push(ch),
        }
    }

    if !token.is_empty() {
        tokens.push(token)
    }

    tokens
}

fn process_data_section(lines: Vec<Line>) -> (HashMap<String, Symbol>, Vec<u8>) {
    // ({name: value}, region)
    // text is stored as {name: location(offset)}
    let mut data_encoded: Vec<u8> = Vec::new();
    let mut symbolmap: HashMap<String, Symbol> = HashMap::new();

    let mut data_ptr: u32 = 0;

    fn align(what: isize, how_much: isize, de: &mut Vec<u8>) -> isize {
        let rem: usize = ((how_much - (what % how_much)) % how_much) as usize;
        de.extend(vec![0; rem]);

        what + rem as isize
    }

    for line in lines {
        let var: &str = line.text.trim();
        // partitioned into type name = value
        // log("var", var)
        if var == "" {
            continue;
        }

        let mut tokens: Vec<String> = Vec::new();
        let mut token: String = String::new();

        let mut within_string: bool = false;
        for ch in var.chars() {
            match ch {
                '"' => within_string = !within_string,
                ' ' => {
                    if !within_string {
                        tokens.push(token.clone());
                        token = String::new();
                    }
                }
                _ => token.push(ch),
            }
        }
        if !token.is_empty() {
            tokens.push(token)
        }

        if tokens.len() < 4 {
            println!("{} {:?}", tokens.len(), tokens);
            panic!("variable declaration too short!")
        }

        let (vtype, name, _, value) = (
            tokens[0].as_str(),
            tokens[1].as_str(),
            tokens[2].as_str(),
            tokens[3].as_str(),
        );

        match vtype {
            "byte" => {
                let vali = int_from_any(value);
                if vali > 0xff {
                    panic!("{value} is too large for {vtype} (variable {name})")
                }

                symbolmap.insert(name.to_string(), Symbol::new(name, data_ptr));
                data_encoded.extend([vali as u8]);

                data_ptr += 1;
            }

            "half" => {
                data_ptr = align(data_ptr as isize, 2, &mut data_encoded) as u32;
                let vali = int_from_any(value);
                if vali > 0xffff {
                    panic!("{value} is too large for {vtype} (variable {name})")
                }

                symbolmap.insert(name.to_string(), Symbol::new(name, data_ptr));
                data_encoded.extend(vali.to_le_bytes());

                data_ptr += 2;
            }

            "word" => {
                data_ptr = align(data_ptr as isize, 4, &mut data_encoded) as u32;
                let vali = int_from_any(value);
                /*if vali as u32 > 0xffff_ffff {
                    panic!("{value} is too large for {vtype} (variable {name})")
                }*/

                symbolmap.insert(name.to_string(), Symbol::new(name, data_ptr));
                data_encoded.extend(vali.to_le_bytes());

                data_ptr += 4;
            }

            "string" => {
                todo!()
                /*case "string":
                string = value.strip('"')

                length = len(string)

                symbolmap[name] = GlobalSymbol(name, data_ptr, length)

                data_encoded.extend([ord(ch) for ch in string])*/
            }

            "const" => {
                todo!()
                /*
                case "const":
                    # $name in the instructions will replace with the value given,
                    # not the address of the value, useful for macros.
                    # integer only,
                    # usage:
                    # name: macro = int (data region)
                    # inst __, __, name (instruction region)

                    vali = int_from_any(value)
                    symbolmap[name] = GlobalSymbol(name, vali, 0) */
            }

            _ => todo!(), /*case "import":
                          # import a bytes file into data
                          path = value
                          with open(path, "rb") as file:
                              data: bytes = file.read()
                              rem = (4 - (data.__len__() % 4)) % 4
                              data_ptr += data.__len__() + rem
                              data_encoded.extend(data)
                              data_encoded.extend([0 for _ in range(rem)])
                              } */
        }
    }

    (symbolmap, data_encoded)
}

fn int_to_lui_addi_pair(register: &str, number: i32) -> Vec<String> {
    let mut pair: Vec<String> = Vec::new();
    let hi = (number + 0x800) >> 12;
    let lo = number - (hi << 12);
    // println!("{} {}", hi, lo);
    if hi != 0 {
        // cannot fit within addi
        pair.push(format!("lui {} {}", register, number >> 12))
    }
    if lo != 0 {
        // n = 0 or n is already finished by lui
        if hi == 0 {
            pair.push(format!("addi {0} x0 {1}", register, number & 0xfff)) // make sure it isnt incrementing
        } else {
            pair.push(format!("addi {0} {0} {1}", register, number & 0xfff))
        }
    }

    // println!("[int_to_lui_addi_pair] pair {:?}; num {}", pair, number);

    pair
}

fn preprocess(lines: Vec<String>) -> Result<Assembly, Box<dyn Error>> {
    let mut registers: HashMap<String, i32> = HashMap::new();
    for i in 0..32 {
        registers.insert(
            [
                "zero", "ra", "sp", "gp", "tp", "t0", "t1", "t2", "fp", "s1", "a0", "a1", "a2",
                "a3", "a4", "a5", "a6", "a7", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9",
                "s10", "s11", "t3", "t4", "t5", "t6",
            ][i]
                .to_string(),
            i as i32,
        );
    }
    // compensate for s0
    registers.insert("s0".to_string(), 8);

    let mut origin: u32 = 0x0;
    let mut metalines: Vec<Line> = Vec::new();
    // let issues: Vec<Error> = Vec::new();
    for (lineno, prelline) in lines.iter().enumerate() {
        metalines.push(Line::new(prelline.trim(), lineno as i32 + 1));
    }
    // * Remove comments
    // metalines = [line for line in metalines if not line.text.startswith("#")]
    metalines = metalines
        .into_iter()
        .filter(|line: &Line| !line.text.starts_with("#"))
        .collect();

    for (i, line) in metalines.clone().into_iter().enumerate() {
        metalines[i] = Line::new(&line.text.split("#").collect::<Vec<&str>>()[0], line.lineno) // break the "true" line text but comments would be stripped from the handler anyways
    }

    // * Directives

    let mut sections: HashMap<&str, Vec<Line>> = HashMap::new();
    let mut within_section: &str = "";

    for line in &metalines {
        let text: &str = line.text.as_str();
        // let lineno = line.lineno;

        println!("{} {} {:?}", text, within_section, sections);
        // process directives
        if !text.starts_with(".") {
            if !within_section.is_empty() {
                match sections.get_mut(within_section) {
                    Some(v) => v.push(line.clone()),
                    None => {
                        panic!("cant get withinsec")
                    }
                }
            }

            continue;
        }

        let mut split: Vec<&str> = text.split(" ").collect::<Vec<&str>>();
        let (iden, other) = (split.remove(0), split);
        match iden {
            ".section" => {
                let section_type = other[0];
                within_section = section_type;
                sections.insert(within_section, Vec::new());
            }

            ".org" => {
                let sorigin = other[0];
                origin = int_from_any(sorigin) as u32;
            }
            _ => todo!(),
        }
    }

    println!("{:#?} {:#?}", sections, metalines);

    let mut data_section: Vec<Line> = Vec::new();

    if !sections.contains_key(".data") {
        /*issues.append(
        Error(metalines[0], "warn", 0, "no data section in file!",
              "add `.section .data` to the top of your file."))*/

        println!("no data section in file! (warn). add `.section .data` to the top of your file.")
    } else {
        data_section = sections.get_mut(".data").expect("err").to_vec()
    }

    if origin == 0 {
        /*issues.append(
        Error(metalines[0], "warn", 0,
              "no origin directive in file! Guessed `0x1000`",
              "add `.org [origin]` to the top of your file."))*/

        origin = 0x1000
    }

    if !sections.contains_key(".text") {
        /*issues.append(
            Error(metalines[0], "error", 1, "a .text section is required!",
                  "add `.section .text` before code."))

        raise CriticalError("yo")*/

        panic!("text section is required!")
    }

    let mut text_section: Vec<Line> = sections
        .get(".text")
        .ok_or("cannot retrieve .text from section list!")?
        .clone();

    let mut symboltable: HashMap<String, Symbol> = HashMap::new();
    let mut encoded: Vec<u8> = Vec::new();

    if data_section.len() > 0 {
        (symboltable, encoded) = process_data_section(data_section) // add encoded
    }

    // * Labels (1)
    // scan .text into a labeltable
    let mut labeltable: HashMap<&str, u32> = HashMap::new(); // addrs as mcode addr

    let mut machloc: u32 = origin;
    let tsclone: Vec<Line> = text_section.clone();
    for line in &tsclone {
        if line.text.is_empty() {
            continue;
        }

        if let Some(label) = line.text.strip_suffix(":") {
            labeltable.insert(label, machloc);
            continue;
        }

        machloc += 4
    }

    // print(labeltable)

    // * Switch all lines to instructions & remove ,
    text_section = text_section
        .into_iter()
        .filter(|line| line.text != "")
        .collect();
    // text_section = [line for line in text_section if line.text != ""]
    let mut new_lines: Vec<Line> = Vec::new();
    for line in text_section {
        new_lines.push(Line::new(line.text.trim(), line.lineno))
    }

    text_section = new_lines;

    let mut instructions: Vec<Instruction> = Vec::new();
    for line in text_section {
        instructions.push(Instruction::new(line.text.replace(",", "").as_str(), line))
    }

    // * remove all labels to fix the labeljoiner (keeping the labels messes with the math)
    instructions = instructions
        .into_iter()
        .filter(|instruction| !instruction.assemble.ends_with(":"))
        .collect();
    /*instructions = [
        instruction for instruction in instructions
        if not instruction.assemble.endswith(":")
    ]*/

    //* Labels (2)
    // convert all the references to labels into offsets
    // references beginning with * are absolute addresses (good to pair with li)

    let mut machloc: u32 = origin;
    for instruction in &mut instructions {
        let parts: Vec<&str> = instruction.assemble.split(" ").collect();
        let mut newparts: Vec<String> = Vec::new();

        for part in parts {
            let mut npart: String = part.to_string();
            if part.starts_with("*") {
                if labeltable.contains_key(part.strip_prefix("*").ok_or("cannot strip *")?) {
                    npart = labeltable
                        .get(part.strip_prefix("*").expect("cannot strip *"))
                        .expect("cannot retrieve label!")
                        .to_string()
                }
            }

            if labeltable.contains_key(part) {
                // println!("{} {} {} {}", labeltable, part, labeltable[part], machloc)
                npart = (labeltable[part] as i32 - machloc as i32).to_string()
            }

            newparts.push(npart)
        }

        instruction.assemble = newparts.join(" ");

        machloc += 4
    }

    //* Resolve constants & symbols

    // print(symboltable)
    if symboltable.keys().len() > 0 {
        for instruction in &mut instructions {
            let parts: Vec<&str> = instruction.assemble.split(" ").collect();
            let mut newparts: Vec<String> = Vec::new();

            for part in parts {
                let mut npart = part.to_string();
                if symboltable.contains_key(part) {
                    npart = symboltable[part].at.to_string()
                }

                newparts.push(npart)
            }

            // print(newparts)
            instruction.assemble = newparts.join(" ");

            // machloc += 4
        }
    }

    //* Make pseudoinstructions true

    let mut ninstructions: Vec<Instruction> = Vec::new();

    for instr in &instructions {
        let mut instspl: Vec<&str> = instr.assemble.split(" ").collect();
        let (op, rest) = (instspl.remove(0), instspl);

        match op {
            "li" => {
                let (rd, val) = (rest[0], rest[1]);
                let pair = int_to_lui_addi_pair(rd, int_from_any(val));

                println!("pair {:?}", pair);

                ninstructions.extend(
                    pair.iter()
                        .map(|ass: &String| Instruction::new(ass.as_str(), instr.line.clone())),
                );
            }

            _ => {
                ninstructions.push(Instruction::new(
                    instr.assemble.as_str(),
                    instr.line.clone(),
                ));
            }
        }
    }

    instructions = ninstructions;

    //* Convert registernames to register numbers & convert all to regular int

    let mut new_instructions: Vec<Instruction> = Vec::new();
    {
        let register_true: Vec<String> = (0..32).map(|n| format!("x{}", n)).collect();
        for instr in &instructions {
            let mut newl: String = String::new();
            let instrtok: Vec<String> = tokenise_line(&instr.assemble);

            for token in instrtok {
                if registers.contains_key(&token) {
                    println!("contains key {}", token);
                    // println!("{}", register_true[*registers.get(&token).expect("register contains passed but get didnt succeed?") as usize].as_str());
                    newl.push_str(
                        register_true[*registers
                            .get(&token)
                            .expect("register contains passed but get didnt succeed?")
                            as usize]
                            .as_str(),
                    );
                } else if "0123456789".contains(token.chars().nth(0).expect("cannot get zeroth")) {
                    // any non-decimal starts with 0 (0x,0b,0o)
                    newl.push_str(int_from_any(&token).to_string().as_str());
                } else {
                    newl.push_str(&token.to_string());
                }

                newl.push(' ');
            }

            new_instructions.push(Instruction::new(&newl, instr.line.clone()))
        }
    }
    instructions = new_instructions;

    return Ok(Assembly {
        _origin: origin,
        instructions,
        data_region: encoded,
    });
}

fn assemble_instructions(
    instructions: Vec<Instruction>,
    opcodes: HashMap<Vec<String>, u8>,
    fn3s: HashMap<Vec<String>, u8>, /*, fn7s: HashMap<Vec<String>, u8>*/
) -> Result<Vec<u8>, Box<dyn Error>> /*, Vec<Error>*/ {
    // errors: list[Error] = []
    let mut full: Vec<u8> = Vec::new();
    for instr in &instructions {
        println!(
            "[{}] {} (as {})",
            instr.line.lineno, instr.line.text, instr.assemble
        );
        let ainstr: &str = &instr.assemble;
        if ainstr.is_empty() {
            continue;
        }
        if ainstr.ends_with(":") {
            continue;
        }

        let mut tokens: Vec<String> = tokenise_line(ainstr);
        let (operator, args) = (tokens.remove(0), tokens);

        let mut opcode: u8 = 0;
        let mut fn3: u32 = 0;

        for (operators, matching) in &opcodes {
            if operators.contains(&operator) {
                opcode = *matching
            }
        }

        for (operators, matching) in &fn3s {
            if operators.contains(&operator) {
                fn3 = (*matching).into()
            }
        }

        // let fn7: u32 = 0;

        if opcode == 0 {
            /*errors.append(
            AssembleError(instr.line, "error", 0x0, "Invalid Instruction",
                          ""))*/

            panic!("Invalid instruction");
        }
        println!("{:#?}", args);

        let mut encoded: u32 = opcode.into();
        match opcode {
            0b0110111 => {
                // lui
                let (rdn, imms) = (&args[0], &args[1]);
                let rd: u32 = u32::from_str_radix(&rdn[1..], 10)? & 0x1f;
                let imm20: u32 = (i32::from_str_radix(&imms, 10)? & 0xfffff) as u32;

                encoded |= rd << 7;
                encoded |= imm20 << 12;
            }
            0b0010111 => {
                // auipc
                let (rdn, imms) = (&args[0], &args[1]);
                let rd: u32 = u32::from_str_radix(&rdn[1..], 10)? & 0x1f;
                let imm20: u32 = (i32::from_str_radix(&imms, 10)? & 0xfffff) as u32;

                encoded |= rd << 7;
                encoded |= imm20 << 12;
            }
            0b1101111 => {
                // jal
                let (rdn, imms) = (&args[0], &args[1]);

                let rd: u32 = u32::from_str_radix(&rdn[1..], 10)? & 0x1f;
                let offset: u32 = (i32::from_str_radix(&imms, 10)? & 0xfffff) as u32;

                // imm[20|10:1|11|19:12]
                let j12_19 = (offset >> 12) & 0xff;
                let j11 = (offset >> 11) & 0x1;
                let j1_10 = (offset >> 1) & 0x3ff;
                let j20 = (offset >> 20) & 0x1;

                encoded |= rd << 7;
                encoded |= j12_19 << 12;
                encoded |= j11 << 20;
                encoded |= j1_10 << 21;
                encoded |= j20 << 31;

                /*except ValueError:
                errors.append(
                    AssembleError(1, instr.line.lineno, instr.line.text,
                                  "expected 2 operands but only got one.",
                                  "did you remember to add `ra`?"))*/
            }
            0b1100111 => {
                // jalr
                let (rdn, rs1n, imms) = (&args[0], &args[1], &args[2]);

                let rd: u32 = u32::from_str_radix(&rdn[1..], 10)? & 0x1f;
                let rs1: u32 = u32::from_str_radix(&rs1n[1..], 10)? & 0x1f;
                let imm12: u32 = (i32::from_str_radix(&imms, 10)? & 0xfff) as u32;

                encoded |= rd << 7;
                encoded |= fn3 << 12;
                encoded |= rs1 << 15;
                encoded |= imm12 << 20;
                /*except LookupError as e:
                errors.append(
                    AssembleError(999, instr.line.lineno, instr.line.text,
                                  e, ""))*/
            }
            0b1100011 => {
                // branch
                let (rs1n, rs2n, offsets) = (&args[0], &args[1], &args[2]);

                let rs1: u32 = u32::from_str_radix(&rs1n[1..], 10)? & 0x1f;
                let rs2: u32 = u32::from_str_radix(&rs2n[1..], 10)? & 0x1f;
                let offset: u32 = ((i32::from_str_radix(&offsets, 10)? & 0xfff) as u32) as u32;
                println!("offset");

                let s12: u32 = (offset >> 12) & 0x1;
                let s11: u32 = (offset >> 11) & 0x1;
                let s5_10: u32 = (offset >> 5) & 0x3f;
                let s1_4: u32 = (offset >> 1) & 0xf;

                encoded |= s11 << 7;
                encoded |= s1_4 << 8;
                encoded |= fn3 << 12;
                encoded |= rs1 << 15;
                encoded |= rs2 << 20;
                encoded |= s5_10 << 25;
                encoded |= s12 << 31;
            }
            0b0000011 => {
                // load
                let (rdn, rs1n, offsets) = (&args[0], &args[1], &args[2]);

                let rd: u32 = u32::from_str_radix(&rdn[1..], 10)? & 0x1f;
                let rs1: u32 = u32::from_str_radix(&rs1n[1..], 10)? & 0x1f;
                let imm12: u32 = (i32::from_str_radix(&offsets, 10)? & 0xfff) as u32;

                encoded |= rd << 7;
                encoded |= rs1 << 15;
                encoded |= fn3 << 12;
                encoded |= imm12 << 20;
            }
            0b0100011 => {
                // store
                let (rs2n, rs1n, offsets) = (&args[0], &args[1], &args[2]);

                let rs1: u32 = u32::from_str_radix(&rs1n[1..], 10)? & 0x1f;
                let rs2: u32 = u32::from_str_radix(&rs2n[1..], 10)? & 0x1f;
                let imm12: u32 = (i32::from_str_radix(&offsets, 10)? & 0xfff) as u32;

                encoded |= (imm12 & 0b11111) << 7;
                encoded |= rs1 << 15;
                encoded |= fn3 << 12;
                encoded |= rs2 << 20;
                encoded |= (imm12 >> 5 & 0xf7) << 25;
            }
            0b0010011 => {
                // r,i->r
                let (rdn, rs1n, imms) = (&args[0], &args[1], &args[2]);

                let rd: u32 = u32::from_str_radix(&rdn[1..], 10)? & 0x1f;
                let rs1: u32 = u32::from_str_radix(&rs1n[1..], 10)? & 0x1f;
                let imm12: u32 = (i32::from_str_radix(&imms, 10)? & 0xfff) as u32;
                encoded |= rd << 7;
                encoded |= rs1 << 15;
                encoded |= fn3 << 12;
                encoded |= imm12 << 20;
            }
            0b0110011 => {
                // r,r->r
                let (rdn, rs1n, rs2n) = (&args[0], &args[1], &args[2]);

                let rd: u32 = u32::from_str_radix(&rdn[1..], 10)? & 0x1f;
                let rs1: u32 = u32::from_str_radix(&rs1n[1..], 10)? & 0x1f;
                let rs2: u32 = u32::from_str_radix(&rs2n[1..], 10)? & 0x1f;

                let fn7: u32 = if vec!["sra", "sub"].contains(&operator.as_str()) {
                    0b010_0000
                } else if vec![
                    "mul", "mulh", "mulhsu", "mulhu", "div", "divu", "rem", "remu",
                ]
                .contains(&operator.as_str())
                {
                    0b000_0001
                } else {
                    0b000_0000
                };

                encoded |= rd << 7;
                encoded |= rs1 << 15;
                encoded |= fn3 << 12;
                encoded |= rs2 << 20;
                encoded |= fn7 << 25;
            }
            0b1110011 => {
                // ebreak/ecall
            }

            // ! EXTENSIONS ! //
            _ => panic!("uh oh!"),
        }

        /*a = encoded & 0xff
        b = (encoded >> 8) & 0xff
        c = (encoded >> 16) & 0xff
        d = (encoded >> 24) & 0xff*/

        full.extend(encoded.to_le_bytes());
    }

    return Ok(full);
}
