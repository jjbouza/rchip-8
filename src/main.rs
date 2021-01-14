use rand;
use rand::Rng;
use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::env;
use log::{info, warn, error};

extern crate sdl2;

use sdl2::pixels::Color;
use sdl2::rect::{Rect};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::time::Duration;
use sdl2::render::Canvas;
use sdl2::video::Window;

struct InputMap{
    one: bool,
    two: bool,
    three: bool,
    C: bool,
    four: bool,
    five: bool,
    six: bool,
    D: bool,
    seven: bool,
    eight: bool,
    nine: bool,
    E: bool,
    A: bool,
    zero: bool,
    B: bool,
    F: bool
}

struct CPU{
    // Stores the CPU State.

    // RAM
    memory: [u8; 4096],
    //registers
    V: [u8; 16],
    I: u16,
    PC: u16,
    SP: u8,
    //stack
    S: [u16; 16],
    rng: rand::rngs::ThreadRng,
    DT: u8
}

fn init_cpu(rom: [u8; 3584]) -> CPU {
    // start by generating the hexadecimal sprites
    let hex_sprites : [u8; 80] = [0xF0, 0x90, 0x90, 0x90, 0xF0,
                                  0x20, 0x60, 0x20, 0x20, 0x70,
                                  0xF0, 0x10, 0xF0, 0x80, 0xF0,
                                  0xF0, 0x10, 0xF0, 0x10, 0xF0,
                                  0x90, 0x90, 0xF0, 0x10, 0x10,
                                  0xF0, 0x80, 0xF0, 0x10, 0xF0,
                                  0xF0, 0x90, 0xF0, 0x90, 0xF0,
                                  0xF0, 0x10, 0x20, 0x40, 0x40,
                                  0xF0, 0x90, 0xF0, 0x90, 0xF0,
                                  0xF0, 0x90, 0xF0, 0x10, 0xF0,
                                  0xF0, 0x90, 0xF0, 0x90, 0x90,
                                  0xE0, 0x90, 0xE0, 0x90, 0xE0,
                                  0xF0, 0x80, 0x80, 0x80, 0xF0,
                                  0xE0, 0x90, 0x90, 0x90, 0xE0,
                                  0xF0, 0x80, 0xF0, 0x80, 0xF0,
                                  0xF0, 0x80, 0xF0, 0x80, 0x80];

    // initialize CPU with correct values.
    let mut memory : [u8; 4096] = [0; 4096];
    memory[0..80].clone_from_slice(&hex_sprites);
    memory[512..].clone_from_slice(&rom);
    let register_file : [u8; 16] = [0; 16];
    let I : u16 = 0;
    let PC : u16 = 512;
    let SP : u8 = 0;
    let S : [u16; 16] = [0; 16];
    let rng : rand::rngs::ThreadRng = rand::thread_rng(); 
    let DT : u8 = 0;
    return CPU{memory: memory, 
                V: register_file, 
                I: I,
                PC: PC, 
                SP: SP, 
                S: S,
                rng: rng,
                DT: DT};
}

fn nib(instruction: [u8; 2], position: u8) -> u8{
    if position == 0{
        return (instruction[0] & 0xF0) >> 4;
    }
    else if position == 1{
        return instruction[0] & 0x0F;
    }
    else if position == 2{
        return (instruction[1] & 0xF0) >> 4;
    }
    else{
        return instruction[1] & 0x0F;
    }
}

fn byte(inp_byte: u8, position: u8) -> u8{
    return inp_byte & (0x1 << position);
}

fn inst_byte(instruction: [u8; 2]) -> u16{
    let byte_more : u16 = instruction[0] as u16;
    let byte_less : u16 = instruction[1] as u16;

    return (byte_more << 8) + byte_less;
}

fn run(cpu: &mut CPU, frame_buffer: &mut [[bool; 32]; 64], input: &mut InputMap){
    // run one instruction
    
    // instruction is 2 bytes long:
    let PC_usize = cpu.PC as usize;
    let ins : [u8; 2] = [cpu.memory[PC_usize], cpu.memory[PC_usize+1]];
    let mut PC_inc : bool = true;

    info!("STARTING CYCLE: PC: {}, Instruction: {:x?}", cpu.PC, ins);

    // start writing if statements...
    if nib(ins, 0) == 0x0 && nib(ins, 1) == 0x0 && nib(ins, 2) == 0xE && nib(ins, 3) == 0x0{
        //CLS - clear display
        info!("{}: Instruction Matched: CLS", {cpu.PC});

        for row in 0..frame_buffer.len(){
            for pixel in 0..frame_buffer[row].len(){
                frame_buffer[row][pixel] = false;
            }
        }
    }
    else if nib(ins, 0) == 0x0 && nib(ins, 1) == 0x0 && nib(ins, 2) == 0xE && nib(ins, 3) == 0xE{
        //RET - return from subrountine
        info!("{}: Instruction Matched: RET", {cpu.PC});

        cpu.SP = cpu.SP-1;
        cpu.PC = cpu.S[(cpu.SP as usize)];
        //PC_inc = false;
    }
    else if nib(ins, 0) == 0x1{
        //JP - Jump to address
        info!("{}: Instruction Matched: JP", {cpu.PC});

        let addr = inst_byte(ins) & 0x0FFF;
        cpu.PC = addr;
        PC_inc = false;
    }
    else if nib(ins, 0) == 0x2{
        //CALL - Call the subroutine at memory address 
        info!("{}: Instruction Matched: CALL", {cpu.PC});

        let addr = inst_byte(ins) & 0x0FFF;
        cpu.S[cpu.SP as usize] = cpu.PC;
        cpu.SP = cpu.SP+1;
        cpu.PC = addr;
        PC_inc = false;
    }
    else if nib(ins, 0) == 0x3{
        //SE - Skip next instruction if register equal to constant
        info!("{}: Instruction Matched: SE", {cpu.PC});

        let reg = nib(ins, 1);
        let cons = (inst_byte(ins) & 0x00FF) as u8;
        if cpu.V[reg as usize] == cons{
            cpu.PC = cpu.PC+2;
        }
    }
    else if nib(ins, 0) == 0x4{
        //SNE - Skip next instruction if register not equal to constant
        info!("{}: Instruction Matched: SNE", {cpu.PC});

        let reg = nib(ins, 1);
        let cons = (inst_byte(ins) & 0x00FF) as u8;
        if cpu.V[reg as usize] != cons{
            cpu.PC = cpu.PC+2;
        }
    }
    else if nib(ins, 0) == 0x5 && nib(ins, 3) == 0x0{
        //SE - Skip next instruction if register equal to other register
        info!("{}: Instruction Matched: SE", {cpu.PC});

        let reg1 = nib(ins, 1);
        let reg2 = nib(ins, 2);
        if cpu.V[reg1 as usize] == cpu.V[reg2 as usize]{
            cpu.PC = cpu.PC+2;
        }
    }
    else if nib(ins, 0) == 0x6{
        //LD - load constant into register
        info!("{}: Instruction Matched: LD", {cpu.PC});

        let reg = nib(ins, 1);
        let cons = (inst_byte(ins) & 0x00FF) as u8;
        cpu.V[reg as usize] = cons;
    }
    else if nib(ins, 0) == 0x7{
        //ADD - Adds a constant value to register
        info!("{}: Instruction Matched: ADD", {cpu.PC});

        let reg = nib(ins, 1); 
        let cons = (inst_byte(ins) & 0x00FF) as u8;
        cpu.V[reg as usize] = ((cpu.V[reg as usize] as u16) + (cons as u16)) as u8;
    }
    else if nib(ins, 0) == 0x8 && nib(ins, 3) == 0x0{
        //LD - Stores the value of register in another
        info!("{}: Instruction Matched: LD", {cpu.PC});

        let reg1 = nib(ins, 1);
        let reg2 = nib(ins, 2);
        cpu.V[reg1 as usize] = cpu.V[reg2 as usize];
    }
    else if nib(ins, 0) == 0x8 && nib(ins, 3) == 0x1{
        //OR - Perform bitwise or between registers and store it back into it.
        info!("{}: Instruction Matched: OR", {cpu.PC});

        let reg1 = nib(ins, 1);
        let reg2 = nib(ins, 2);

        cpu.V[reg1 as usize] = cpu.V[reg1 as usize] | cpu.V[reg2 as usize];
    }
    else if nib(ins, 0) == 0x8 && nib(ins, 3) == 0x2{
        //AND - Perform bitwise and between registers and store it back into it.
        info!("{}: Instruction Matched: AND", {cpu.PC});

        let reg1 = nib(ins, 1);
        let reg2 = nib(ins, 2);

        cpu.V[reg1 as usize] = cpu.V[reg1 as usize] & cpu.V[reg2 as usize];
    }
    else if nib(ins, 0) == 0x8 && nib(ins, 3) == 0x3{
        //XOR - Perform bitwise XOR between registers and store it back into it
        info!("{}: Instruction Matched: XOR", {cpu.PC});

        let reg1 = nib(ins, 1);
        let reg2 = nib(ins, 2);

        cpu.V[reg1 as usize] = cpu.V[reg1 as usize] ^ cpu.V[reg2 as usize];
    }
    else if nib(ins, 0) == 0x8 && nib(ins, 3) == 0x4{
        //ADD - Add two registers and set overflow register.
        info!("{}: Instruction Matched: ADD", {cpu.PC});

        let reg1 = nib(ins, 1);
        let reg2 = nib(ins, 2);
        cpu.V[reg1 as usize] = ((cpu.V[reg1 as usize] as u16)+(cpu.V[reg2 as usize] as u16)) as u8;
        if (cpu.V[reg1 as usize] as u16) + (cpu.V[reg2 as usize] as u16) > 255{
            cpu.V[0xF] = 1;
        }
    }
    else if nib(ins, 0) == 0x8 && nib(ins, 3) == 0x5{
        //SUB - Subtract one register from another and store it back into it.
        info!("{}: Instruction Matched: SUB", {cpu.PC});

        let reg1 = nib(ins, 1);
        let reg2 = nib(ins, 2);
        
        let V1 = cpu.V[reg1 as usize];
        let V2 = cpu.V[reg2 as usize];
        
        cpu.V[0xF] = if V1 > V2 { 1 } else { 0 };
        cpu.V[reg1 as usize] = ((V1 as i16)-(V2 as i16)) as u8;
    }
    else if nib(ins, 0) == 0x8 && nib(ins, 3) == 0x6{
        //SHR - If the least signficant of register is 1 then VF=1. Then register /= 2
        info!("{}: Instruction Matched: SHR", {cpu.PC});

        let reg1 = nib(ins, 1);
        let lsb = (cpu.V[reg1 as usize]) & 0x01;
        cpu.V[0xF] = if lsb == 1 { 1 } else { 0 };
        cpu.V[reg1 as usize] = cpu.V[reg1 as usize] >> 2;
    }
    else if nib(ins, 0) == 0x8 && nib(ins, 3) == 0x7{
        //SUBN
        info!("{}: Instruction Matched: SUBN", {cpu.PC});

        let reg1 = nib(ins, 1);
        let reg2 = nib(ins, 2);
        
        let V1 = cpu.V[reg1 as usize];
        let V2 = cpu.V[reg2 as usize];
        
        cpu.V[0xF] = if V2 > V1 { 1 } else { 0 };
        cpu.V[reg1 as usize] = V2-V1;
    }
    else if nib(ins, 0) == 0x8 && nib(ins, 3) == 0xE{
        //SHL
        info!("{}: Instruction Matched: SHL", {cpu.PC});

        let reg1 = nib(ins, 1);
        let msb = (cpu.V[reg1 as usize]) & 0x80;
        cpu.V[0xF] = if msb == 1 { 1 } else { 0 };
        cpu.V[reg1 as usize] = cpu.V[reg1 as usize] << 2;
    }
    else if nib(ins, 0) == 0x9 && nib(ins, 3) == 0x0{
        //SNE
        info!("{}: Instruction Matched: SNE", {cpu.PC});

        let reg1 = nib(ins, 1);
        let reg2 = nib(ins, 2);
        if cpu.V[reg1 as usize] != cpu.V[reg2 as usize]{
            cpu.PC = cpu.PC + 2;
        }
    }
    else if nib(ins, 0) == 0xA{
        //LD - Set the value of register to memory location
        info!("{}: Instruction Matched: LD", {cpu.PC});

        let value = inst_byte(ins) & 0x0FFF;
        cpu.I = value as u16;
    }
    else if nib(ins, 0) == 0xB{
        //JP - Jump to location in memory location plus V0
        info!("{}: Instruction Matched: JP", {cpu.PC});

        let memory_loc = inst_byte(ins) & 0x0FFF;
        cpu.PC = (memory_loc as  u16) + (cpu.V[0x0] as u16);
    }
    else if nib(ins, 0) == 0xC{
        //RND - Random byte and with constant
        info!("{}: Instruction Matched: RND", {cpu.PC});

        let reg1 = nib(ins, 1);
        let cons = inst_byte(ins) & 0x00FF;
        let rand_byte: u8 = cpu.rng.gen();
        cpu.V[reg1 as usize] = rand_byte & 0x00FF;
    }
    else if nib(ins, 0) == 0xD{
        //DRW
        info!("{}: Instruction Matched: DRW", {cpu.PC});

        cpu.V[0xF] = 0;
        let sprite_start = cpu.I as usize;
        let sprite_end = sprite_start+(nib(ins,3) as usize);
        let x = cpu.V[nib(ins, 1) as usize] as usize;
        let y = cpu.V[nib(ins, 2) as usize] as usize;
        for memory_location in sprite_start..sprite_end{
            let sprite_byte = cpu.memory[memory_location];
            for col in 0..8{
                let mut fb_x = x+col;
                let mut fb_y = y+memory_location-sprite_start;

                // wrap around
                if fb_x > frame_buffer.len(){
                    fb_x = fb_x-frame_buffer.len();
                }
                if fb_y > frame_buffer[0].len(){
                    fb_y = fb_y-frame_buffer.len()
                }
                
                // set collision register
                let sprite_bit = byte(sprite_byte, 7-col as u8) != 0;
                if frame_buffer[fb_x][fb_y] && sprite_bit{
                    cpu.V[0xF] = 1;
                }

                frame_buffer[fb_x][fb_y] = frame_buffer[fb_x][fb_y] ^ sprite_bit;
            }
        }
    }
    else if nib(ins, 0) == 0xE && nib(ins, 2) == 0x9 && nib(ins,3) == 0xE{
        //SKP
        info!("{}: Instruction Matched: SKP", {cpu.PC});

        let reg1 = nib(ins, 1);
        let key_seek = cpu.V[reg1 as usize];
        if input_key_seek(input, key_seek){
            cpu.PC += 2
        }
    }
    else if nib(ins, 0) == 0xE && nib(ins, 2) == 0xA && nib(ins,3) == 0x1{
        //SKNP
        info!("{}: Instruction Matched: SKNP", {cpu.PC});

        let reg1 = nib(ins, 1);
        let key_seek = cpu.V[reg1 as usize];
        if !input_key_seek(input, key_seek){
            cpu.PC += 2
        }
    }
    else if nib(ins, 0) == 0xF && nib(ins, 2) == 0x0 && nib(ins,3) == 0x7{
        //LD
        info!("{}: Instruction Matched: LD", {cpu.PC});

        let reg1 = nib(ins, 1);
        cpu.V[reg1 as usize] = cpu.DT;

    }
    else if nib(ins, 0) == 0xF && nib(ins, 2) == 0x0 && nib(ins,3) == 0xA{
        //LD
        info!("{}: Instruction Matched: LD", {cpu.PC});

        let reg1 = nib(ins, 1);
        let mut found = false;
        for key_seek in 0..16{
            let inp_key = input_key_seek(input, key_seek);
            if inp_key{
                cpu.V[reg1 as usize] = key_seek;
                found = true;
            }
        }
        if !found{
            cpu.PC -= 2;
        }
    }
    else if nib(ins, 0) == 0xF && nib(ins, 2) == 0x1 && nib(ins,3) == 0x5{
        //LD
        info!("{}: Instruction Matched: LD", {cpu.PC});

        let reg1 = nib(ins, 1);
        cpu.DT = cpu.V[reg1 as usize];
    }
    else if nib(ins, 0) == 0xF && nib(ins, 2) == 0x1 && nib(ins,3) == 0x8{
        //LD
        info!("{}: Instruction Matched: LD", {cpu.PC});
        // Sound not implemented.
    }
    else if nib(ins, 0) == 0xF && nib(ins, 2) == 0x1 && nib(ins,3) == 0xE{
        //ADD
        info!("{}: Instruction Matched: ADD", {cpu.PC});

        let reg1 = nib(ins, 1);
        cpu.I = cpu.I+(cpu.V[reg1 as usize] as u16)
    }
    else if nib(ins, 0) == 0xF && nib(ins, 2) == 0x2 && nib(ins,3) == 0x9{
        //LD
        info!("{}: Instruction Matched: LD", {cpu.PC});

        let reg1 = nib(ins, 1);
        let hex_request = cpu.V[reg1 as usize];
        let location = (hex_request as u16)*5;
        cpu.I = location;
    }
    else if nib(ins, 0) == 0xF && nib(ins, 2) == 0x3 && nib(ins,3) == 0x3{
        //LD - Need to check this...
        info!("{}: Instruction Matched: LD", {cpu.PC});

        let reg1 = nib(ins, 1);
        let hex_request = cpu.V[reg1 as usize];
        cpu.memory[cpu.I as usize] = hex_request/100 % 10;
        cpu.memory[(cpu.I+1) as usize] = hex_request/10 % 10;
        cpu.memory[(cpu.I+2) as usize] = hex_request % 10;
    }
    else if nib(ins, 0) == 0xF && nib(ins, 2) == 0x5 && nib(ins,3) == 0x5{
        //LD
        info!("{}: Instruction Matched: LD", {cpu.PC});

        let reg1 = nib(ins, 1);
        for x in 0..reg1{
            cpu.memory[(cpu.I+(x as u16)) as usize] = cpu.V[x as usize];
        }
    }
    else if nib(ins, 0) == 0xF && nib(ins, 2) == 0x6 && nib(ins,3) == 0x5{
        //LD
        info!("{}: Instruction Matched: LD", {cpu.PC});

        let reg1 = nib(ins, 1);
        for x in 0..reg1{
            cpu.V[x as usize] = cpu.memory[(cpu.I+(x as u16)) as usize];
        }
    }
    else{
        info!("{}: Instruction Matched: UNKNOWN", {cpu.PC}); 
    }
    
    // append PC
    if PC_inc{
        cpu.PC += 2;
    }
}

fn input_key_seek(input_map: &InputMap, key_seek: u8) -> bool{
    match key_seek{
        0x0 => {input_map.zero}
        0x1 => {input_map.one}
        0x2 => {input_map.two}
        0x3 => {input_map.three}
        0x4 => {input_map.four}
        0x5 => {input_map.five}
        0x6 => {input_map.six}
        0x7 => {input_map.seven}
        0x8 => {input_map.eight}
        0x9 => {input_map.nine}
        0xA => {input_map.A}
        0xB => {input_map.B}
        0xC => {input_map.C}
        0xD => {input_map.D}
        0xE => {input_map.E}
        0xF => {input_map.F}
        _ => {false}
    }
}

fn input_event_map(input_map: &mut InputMap, event: Event){
    match event {
        Event::KeyDown { keycode: Some(Keycode::Q), ..  } => {
            input_map.one = true;
        },
        Event::KeyDown { keycode: Some(Keycode::W), ..  } => {
            input_map.two = true;
        },
        Event::KeyDown { keycode: Some(Keycode::E), ..  } => {
            input_map.three = true;
        },
        Event::KeyDown { keycode: Some(Keycode::R), ..  } => {
            input_map.C = true;
        },
        Event::KeyDown { keycode: Some(Keycode::A), ..  } => {
            input_map.four = true;
        },
        Event::KeyDown { keycode: Some(Keycode::S), ..  } => {
            input_map.five = true;
        },
        Event::KeyDown { keycode: Some(Keycode::D), ..  } => {
            input_map.six = true;
        },
        Event::KeyDown { keycode: Some(Keycode::F), ..  } => {
            input_map.D = true;
        },
        Event::KeyDown { keycode: Some(Keycode::Z), ..  } => {
            input_map.seven = true;
        },
        Event::KeyDown { keycode: Some(Keycode::X), ..  } => {
            input_map.eight = true;
        },
        Event::KeyDown { keycode: Some(Keycode::C), ..  } => {
            input_map.nine = true;
        },
        Event::KeyDown { keycode: Some(Keycode::V), ..  } => {
            input_map.E = true;
        },
        Event::KeyDown { keycode: Some(Keycode::U), ..  } => {
            input_map.A = true;
        },
        Event::KeyDown { keycode: Some(Keycode::I), ..  } => {
            input_map.zero = true;
        },
        Event::KeyDown { keycode: Some(Keycode::O), ..  } => {
            input_map.B = true;
        },
        Event::KeyDown { keycode: Some(Keycode::P), ..  } => {
            input_map.F = true;
        },
        Event::KeyUp { keycode: Some(Keycode::Q), ..  } => {
            input_map.one = false;
        },
        Event::KeyUp { keycode: Some(Keycode::W), ..  } => {
            input_map.two = false;
        },
        Event::KeyUp { keycode: Some(Keycode::E), ..  } => {
            input_map.three = false;
        },
        Event::KeyUp { keycode: Some(Keycode::R), ..  } => {
            input_map.C = false;
        },
        Event::KeyUp { keycode: Some(Keycode::A), ..  } => {
            input_map.four = false;
        },
        Event::KeyUp { keycode: Some(Keycode::S), ..  } => {
            input_map.five = false;
        },
        Event::KeyUp { keycode: Some(Keycode::D), ..  } => {
            input_map.six = false;
        },
        Event::KeyUp { keycode: Some(Keycode::F), ..  } => {
            input_map.D = false;
        },
        Event::KeyUp { keycode: Some(Keycode::Z), ..  } => {
            input_map.seven = false;
        },
        Event::KeyUp { keycode: Some(Keycode::X), ..  } => {
            input_map.eight = false;
        },
        Event::KeyUp { keycode: Some(Keycode::C), ..  } => {
            input_map.nine = false;
        },
        Event::KeyUp { keycode: Some(Keycode::V), ..  } => {
            input_map.E = false;
        },
        Event::KeyUp { keycode: Some(Keycode::U), ..  } => {
            input_map.A = false;
        },
        Event::KeyUp { keycode: Some(Keycode::I), ..  } => {
            input_map.zero = false;
        },
        Event::KeyUp { keycode: Some(Keycode::O), ..  } => {
            input_map.B = false;
        },
        Event::KeyUp { keycode: Some(Keycode::P), ..  } => {
            input_map.F = false;
        },
        _ => {}
    }
}

fn draw_grid(frame_buffer: &[[bool; 32]; 64], canvas: &mut Canvas<Window>, pixel_scaling: i32){
    for row_ind in 0..frame_buffer.len(){
        for pixel_ind in 0..frame_buffer[row_ind].len(){
            if frame_buffer[row_ind][pixel_ind]{
                canvas.set_draw_color(Color::RGB(255, 255, 255));
                canvas.fill_rect(Rect::new((row_ind as i32)*pixel_scaling, (pixel_ind as i32)*pixel_scaling, 9, 9));
            }
        }
    }
    canvas.present();
}

fn load_rom(fname: &str) -> io::Result<[u8; 3584]>{
    let mut file_handle = File::open(fname)?;
    let mut rom_buffer: [u8; 3584] = [0; 3584];
    file_handle.read(&mut rom_buffer)?;
    return Ok(rom_buffer);
}

fn main(){
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let rom_location = &args[1];
    let rom = load_rom(rom_location);
    let rom = match rom{
        Ok(rom_) => rom_,
        Err(error) => panic!("Problem opening ROM file: {}", error),
    };

    let mut cpu = init_cpu(rom);
    let mut input_map = InputMap{
        one: false,
        two: false,
        three: false,
        C: false,
        four: false,
        five: false,
        six: false,
        D: false,
        seven: false,
        eight: false,
        nine: false,
        E: false,
        A: false,
        zero: false,
        B: false,
        F: false
    };
    let mut frame_buffer : [[bool; 32]; 64]  = [[false; 32]; 64];

    let pixel_scaling = 10;
    // setup multimedia loop
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem.window("rust-sdl2 demo", pixel_scaling*64, pixel_scaling*32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'running;
                },
                _ => {
                    input_event_map(&mut input_map, event);
                }
            }
        }
        // RUN INSTRUCTION
        run(&mut cpu, &mut frame_buffer, &mut input_map);

        // Draw background:
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        // Draw frame buffer
        draw_grid(&frame_buffer, &mut canvas, pixel_scaling as i32);

        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
        if cpu.DT > 0{
            cpu.DT = cpu.DT - 1;
        }
    }
}
