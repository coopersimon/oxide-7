use oxide7::SNES;

macro_rules! make24 {
    ($hi:expr, $lo:expr) => {
        (($hi as u32) << 16) | ($lo as u32)
    };
    ($hi:expr, $mid:expr, $lo:expr) => {
        (($hi as u32) << 16) | (($mid as u32) << 8) | ($lo as u32)
    };
}

pub fn debug_mode(snes: &mut SNES) {
    // Debug mode.
    snes.start_frame();
    println!("Debug mode.");
    println!("Enter 'h' for help.");
    let mut breaks = std::collections::BTreeSet::new();
    let mut stack_trace = Vec::new();
    loop {
        let mut input = String::new();
        match std::io::stdin().read_line(&mut input) {
            Ok(_) => if input.starts_with("b:") {
                // Add breakpoint
                match u32::from_str_radix(&input[2..].trim(), 16) {
                    Ok(num) => {
                        println!("Inserted breakpoint at ${:06X}", num);
                        breaks.insert(num);
                    },
                    Err(e) => println!("Invalid breakpoint: {}", e),
                }
            } else if input.starts_with("c:") {
                // Remove breakpoint
                match u32::from_str_radix(&input[2..].trim(), 16) {
                    Ok(num) => {
                        println!("Cleared breakpoint at ${:06X}", num);
                        breaks.remove(&num);
                    },
                    Err(e) => println!("Invalid breakpoint: {}", e),
                }
            } else if input.starts_with("c") {
                // Remove all breakpoints
                println!("Cleared all breakpoints");
                breaks.clear();
            } else if input.starts_with("r") {
                // Run
                loop {
                    let state = snes.get_state();
                    let loc = make24!(state.pb, state.pc);
                    if breaks.contains(&loc) {
                        println!("Break at ${:06X}", loc);
                        break;
                    } else {
                        step_and_trace(snes, &mut stack_trace, false);
                    }
                }
            } else if input.starts_with("s:") {
                // Step x times
                match usize::from_str_radix(&input[2..].trim(), 10) {
                    Ok(num) => {
                        for _ in 0..num {
                            step_and_trace(snes, &mut stack_trace, true);
                        }
                    },
                    Err(e) => println!("Invalid number of steps: {}", e),
                }
            } else if input.starts_with("s") {
                // Step
                step_and_trace(snes, &mut stack_trace, true);
            } else if input.starts_with("p:") {
                // Print cpu or mem state
                print(&input[2..].trim(), snes);
            } else if input.starts_with("p") {
                // Print state
                println!("{}", snes.get_state().to_string());
            } else if input.starts_with("t") {
                let trace = stack_trace.iter()
                    .map(|n| format!("${:06X}", n))
                    .collect::<Vec<_>>()
                    .join("\n");
                println!("{}", trace);
            } else if input.starts_with("h") {
                // Help
                help();
            } else if input.starts_with("q") {
                break;
            },
            Err(e) => println!("Input error: {}", e),
        }
    }
}

fn print(s: &str, snes: &mut SNES) {
    match s {
        "a" => println!("a: ${:04X}", snes.get_state().a),
        "x" => println!("x: ${:04X}", snes.get_state().x),
        "y" => println!("y: ${:04X}", snes.get_state().y),
        "s" => println!("s: ${:04X}", snes.get_state().s),
        "db" => println!("db: ${:02X}", snes.get_state().db),
        "dp" => println!("dp: ${:04X}", snes.get_state().dp),
        "pb" => println!("pb: ${:02X}", snes.get_state().pb),
        "pc" => println!("pc: ${:04X}", snes.get_state().pc),
        "p" => println!("p: b{:08b}", snes.get_state().p),
        "e" => println!("e: b{:08b}", snes.get_state().pe),
        s => {
            // Memory range
            if let Some(x) = s.find('-') {
                match u32::from_str_radix(&s[..x], 16) {
                    Ok(start) => match u32::from_str_radix(&s[(x+1)..], 16) {
                        Ok(end) => {
                            println!("${:06X} - ${:06X}:", start, end);
                            let mems = (start..end).map(|n| format!("{:02X}", snes.get_mem_at(n)))
                                .collect::<Vec<_>>()
                                .join(" ");
                            println!("{}", mems);
                        },
                        Err(e) => println!("Invalid p tag: {}", e),
                    },
                    Err(e) => println!("Invalid p tag: {}", e),
                }
            } else {    // Single location
                match u32::from_str_radix(s, 16) {
                    Ok(num) => println!("${:06X}: ${:02X}", num, snes.get_mem_at(num)),
                    Err(e) => println!("Invalid p tag: {}", e),
                }
            }
        }
    }
}

fn help() {
    println!("b:x: New breakpoint at memory location x (hex).");
    println!("c:x: Clear breakpoint at memory location x (hex).");
    println!("r: Keep running until a breakpoint is hit.");
    println!("s: Step a single instruction.");
    println!("s:x: Step multiple instructions (base 10).");
    println!("t: Print the stack trace (all the call locations).");
    println!("p: Print the current state of the CPU.");
    println!("p:x: Print x - if x is a number, print the contents of that address, otherwise print the register.");
    println!("p:x-y: Print the memory in the range x -> y.");
    println!("q: Quit execution.");
}

// Step the CPU, and add the PC to the stack trace if it calls.
fn step_and_trace(snes: &mut SNES, stack_trace: &mut Vec<u32>, print: bool) {
    let instr = snes.get_instr();
    match instr[0] {
        0x22 | 0x20 | 0xFC => {
            stack_trace.push(make24!(snes.get_state().pb, snes.get_state().pc));
        },
        0x6B | 0x60 => {
            stack_trace.pop();
        },
        _ => {}
    }

    if print {
        let state = snes.get_state();
        let pc = make24!(state.pb, state.pc);
        println!("${:06X}: ${:02X} ({:02X} {:02X} {:02X})", pc, instr[0], instr[1], instr[2], instr[3]);
    }

    if snes.step() {
        snes.start_frame();
    }
}