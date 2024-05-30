use std::{
    env, fs, process,
    time::{Duration, Instant},
};

use instruction::Instruction;
use sdl2::{event::Event, keyboard::Keycode, pixels::Color, rect::Rect};

mod instruction;

const DISPLAY_WIDTH: usize = 64;
const DISPLAY_HEIGHT: usize = 32;

const MEMORY_SIZE: usize = 4096;

const PROGRAM_START: usize = 0x200;

const FONT_SPRITES: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

struct Emulator {
    v: [u8; 16],
    i: u16,
    pc: u16,
    sp: u8,
    stack: [u16; 16],
    delay_timer: u8,
    sound_timer: u8,
    memory: [u8; MEMORY_SIZE],
    buffer: [bool; DISPLAY_WIDTH * DISPLAY_HEIGHT],
    keys: [bool; 16],
    waiting_key_vx: Option<usize>,
}

impl Emulator {
    pub fn new() -> Self {
        Emulator {
            v: [0; 16],
            i: 0,
            pc: PROGRAM_START as u16,
            sp: 0,
            stack: [0; 16],
            delay_timer: 0,
            sound_timer: 0,
            memory: [0; MEMORY_SIZE],
            buffer: [false; DISPLAY_WIDTH * DISPLAY_HEIGHT],
            keys: [false; 16],
            waiting_key_vx: None,
        }
    }

    fn fetch(&mut self) -> u16 {
        let high_byte = self.memory[self.pc as usize] as u16;
        let low_byte = self.memory[self.pc as usize + 1] as u16;
        high_byte << 8 | low_byte
    }

    pub fn decode(&mut self, opcode: u16) -> Instruction {
        let address = opcode & 0x0FFF;
        let byte = (opcode & 0x00FF) as u8;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;
        let nibble = (opcode & 0x000F) as u8;

        match opcode & 0xF000 {
            0x0000 => match opcode & 0x00FF {
                0xE0 => Instruction::Clear,
                0xEE => Instruction::Return,
                _ => Instruction::System { address },
            },
            0x1000 => Instruction::Jump { address },
            0x2000 => Instruction::Call { address },
            0x3000 => Instruction::SkipEqual { x, byte },
            0x4000 => Instruction::SkipNotEqual { x, byte },
            0x5000 => Instruction::SkipEqualRegister { x, y },
            0x6000 => Instruction::Load { x, byte },
            0x7000 => Instruction::Add { x, byte },
            0x8000 => match opcode & 0x000F {
                0x0 => Instruction::LoadRegister { x, y },
                0x1 => Instruction::OrRegister { x, y },
                0x2 => Instruction::AndRegister { x, y },
                0x3 => Instruction::XorRegister { x, y },
                0x4 => Instruction::AddRegister { x, y },
                0x5 => Instruction::SubtractRegister { x, y },
                0x6 => Instruction::ShiftRight { x },
                0x7 => Instruction::SubtractReverseRegister { x, y },
                0xE => Instruction::ShiftLeft { x },
                _ => panic!("Unknown opcode: {:04X}", opcode),
            },
            0x9000 => Instruction::SkipNotEqualRegister { x, y },
            0xA000 => Instruction::LoadIndex { address },
            0xB000 => Instruction::JumpOffset { address },
            0xC000 => Instruction::Random { x, byte },
            0xD000 => Instruction::Draw { x, y, nibble },
            0xE000 => match opcode & 0x00FF {
                0x9E => Instruction::SkipKeyPressed { x },
                0xA1 => Instruction::SkipKeyNotPressed { x },
                _ => panic!("Unknown opcode: {:04X}", opcode),
            },
            0xF000 => match opcode & 0x00FF {
                0x07 => Instruction::LoadDelay { x },
                0x0A => Instruction::WaitKeyPress { x },
                0x15 => Instruction::SetDelay { x },
                0x18 => Instruction::SetSound { x },
                0x1E => Instruction::AddIndex { x },
                0x29 => Instruction::LoadSprite { x },
                0x33 => Instruction::LoadBCD { x },
                0x55 => Instruction::StoreRegisters { x },
                0x65 => Instruction::LoadRegisters { x },
                _ => panic!("Unknown opcode: {:04X}", opcode),
            },
            _ => panic!("Unknown opcode: {:04X}", opcode),
        }
    }

    pub fn execute(&mut self, instruction: Instruction) {
        println!("{}", instruction.to_str());

        match instruction {
            Instruction::System { address: _ } => {}
            Instruction::Clear => {
                self.buffer = [false; 2048];
            }
            Instruction::Return => {
                if self.sp == 0 {
                    panic!("Stack underflow");
                }
                self.sp -= 1;
                self.pc = self.stack[self.sp as usize];
            }
            Instruction::Jump { address } => {
                self.pc = address;
            }
            Instruction::Call { address } => {
                if self.sp >= 16 {
                    panic!("Stack overflow");
                }
                self.stack[self.sp as usize] = self.pc;
                self.sp += 1;
                self.pc = address;
            }
            Instruction::SkipEqual { x, byte } => {
                if self.v[x] == byte {
                    self.pc += 2;
                }
            }
            Instruction::SkipNotEqual { x, byte } => {
                if self.v[x] != byte {
                    self.pc += 2;
                }
            }
            Instruction::SkipEqualRegister { x, y } => {
                if self.v[x] == self.v[y] {
                    self.pc += 2;
                }
            }
            Instruction::Load { x, byte } => {
                self.v[x] = byte;
            }
            Instruction::Add { x, byte } => {
                self.v[x] = self.v[x].wrapping_add(byte);
            }
            Instruction::LoadRegister { x, y } => {
                self.v[x] = self.v[y];
            }
            Instruction::OrRegister { x, y } => {
                self.v[x] |= self.v[y];
            }
            Instruction::AndRegister { x, y } => {
                self.v[x] &= self.v[y];
            }
            Instruction::XorRegister { x, y } => {
                self.v[x] ^= self.v[y];
            }
            Instruction::AddRegister { x, y } => {
                let (result, overflow) = self.v[x].overflowing_add(self.v[y]);
                self.v[x] = result;
                self.v[0xF] = overflow as u8;
            }
            Instruction::SubtractRegister { x, y } => {
                let (result, overflow) = self.v[x].overflowing_sub(self.v[y]);
                self.v[x] = result;
                self.v[0xF] = !overflow as u8;
            }
            Instruction::ShiftRight { x } => {
                self.v[0xF] = self.v[x] & 0x1;
                self.v[x] >>= 1;
            }
            Instruction::SubtractReverseRegister { x, y } => {
                let (result, overflow) = self.v[y].overflowing_sub(self.v[x]);
                self.v[x] = result;
                self.v[0xF] = !overflow as u8;
            }
            Instruction::ShiftLeft { x } => {
                self.v[0xF] = self.v[x] >> 7;
                self.v[x] <<= 1;
            }
            Instruction::SkipNotEqualRegister { x, y } => {
                if self.v[x] != self.v[y] {
                    self.pc += 2;
                }
            }
            Instruction::LoadIndex { address } => {
                self.i = address;
            }
            Instruction::JumpOffset { address } => {
                self.pc = address + self.v[0] as u16;
            }
            Instruction::Random { x, byte } => {
                self.v[x] = rand::random::<u8>() & byte;
            }
            Instruction::Draw { x, y, nibble } => {
                let vx = self.v[x] as usize;
                let vy = self.v[y] as usize;

                for byte_index in 0..nibble {
                    let byte = self.memory[self.i as usize + byte_index as usize];
                    let y = (vy + byte_index as usize) % DISPLAY_HEIGHT;

                    for bit_index in 0..8 {
                        let bit = (byte >> (7 - bit_index)) & 1;
                        let x = (vx + bit_index) % DISPLAY_WIDTH;

                        if bit == 1 {
                            let index = y * DISPLAY_WIDTH + x;
                            self.v[0xF] = self.buffer[index] as u8;
                            self.buffer[index] ^= true;
                        }
                    }
                }
            }
            Instruction::SkipKeyPressed { x } => {
                if self.keys[self.v[x] as usize] {
                    self.pc += 2;
                }
            }
            Instruction::SkipKeyNotPressed { x } => {
                if !self.keys[self.v[x] as usize] {
                    self.pc += 2;
                }
            }
            Instruction::LoadDelay { x } => {
                self.v[x] = self.delay_timer;
            }
            Instruction::WaitKeyPress { x } => {
                self.waiting_key_vx = Some(x);
            }
            Instruction::SetDelay { x } => {
                self.delay_timer = self.v[x];
            }
            Instruction::SetSound { x } => {
                self.sound_timer = self.v[x];
            }
            Instruction::AddIndex { x } => {
                self.i += self.v[x] as u16;
            }
            Instruction::LoadSprite { x } => {
                self.i = self.v[x] as u16 * 5;
            }
            Instruction::LoadBCD { x } => {
                let value = self.v[x];
                self.memory[self.i as usize] = value / 100;
                self.memory[self.i as usize + 1] = (value / 10) % 10;
                self.memory[self.i as usize + 2] = value % 10;
            }
            Instruction::StoreRegisters { x } => {
                for i in 0..=x {
                    self.memory[self.i as usize + i] = self.v[i];
                }
            }
            Instruction::LoadRegisters { x } => {
                for i in 0..=x {
                    self.v[i] = self.memory[self.i as usize + i];
                }
            }
        }
    }

    pub fn cycle(&mut self) {
        if let Some(x) = self.waiting_key_vx {
            for key in self.keys {
                if key {
                    self.v[x] = key as u8;
                    self.waiting_key_vx = None;
                }
            }
        } else {
            let opcode = self.fetch();
            let instruction = self.decode(opcode);
            self.pc += 2;
            self.execute(instruction);
        }
    }

    pub fn timers_cycle(&mut self) {
        self.delay_timer = self.delay_timer.saturating_sub(1);
        self.sound_timer = self.sound_timer.saturating_sub(1);
    }

    pub fn load(&mut self, program: &[u8]) {
        self.memory[0..FONT_SPRITES.len()].copy_from_slice(&FONT_SPRITES);
        self.memory[0x200..0x200 + program.len()].copy_from_slice(program);
    }
}

fn keycode_to_key(keycode: Keycode) -> Option<u8> {
    match keycode {
        Keycode::Num1 => Some(0x1),
        Keycode::Num2 => Some(0x2),
        Keycode::Num3 => Some(0x3),
        Keycode::Num4 => Some(0xC),
        Keycode::Q => Some(0x4),
        Keycode::W => Some(0x5),
        Keycode::E => Some(0x6),
        Keycode::R => Some(0xD),
        Keycode::A => Some(0x7),
        Keycode::S => Some(0x8),
        Keycode::D => Some(0x9),
        Keycode::F => Some(0xE),
        Keycode::Z => Some(0xA),
        Keycode::X => Some(0x0),
        Keycode::C => Some(0xB),
        Keycode::V => Some(0xF),
        _ => None,
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: chip8 <Program file>");
        process::exit(1);
    }

    let program_path = &args[1];
    let program = fs::read(program_path).expect("Failed to read program");

    let mut emulator = Emulator::new();
    emulator.load(&program);

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window(
            "CHIP-8",
            DISPLAY_WIDTH as u32 * 10,
            DISPLAY_HEIGHT as u32 * 10,
        )
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().accelerated().build().unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();

    let frame_interval = Duration::from_millis(1000 / 60);
    let timer_interval = Duration::from_millis(1000 / 60);
    let cpu_interval = Duration::from_millis(1000 / 500);

    let mut last_frame_update = Instant::now();
    let mut last_timer_update = Instant::now();
    let mut last_cpu_update = Instant::now();

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => {
                    if let Some(key) = keycode_to_key(keycode) {
                        emulator.keys[key as usize] = true;
                    }
                }
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => {
                    if let Some(key) = keycode_to_key(keycode) {
                        emulator.keys[key as usize] = false;
                    }
                }
                _ => {}
            }
        }

        let now = Instant::now();

        if now.duration_since(last_frame_update) >= frame_interval {
            canvas.set_draw_color(Color::BLACK);
            canvas.clear();

            canvas.set_draw_color(Color::WHITE);

            for (i, &pixel) in emulator.buffer.iter().enumerate() {
                if pixel {
                    let x = i % DISPLAY_WIDTH;
                    let y = i / DISPLAY_WIDTH;

                    canvas
                        .fill_rect(Rect::new(x as i32 * 10, y as i32 * 10, 10, 10))
                        .unwrap();
                }
            }

            canvas.present();
            last_frame_update = now;
        }

        if now.duration_since(last_timer_update) >= timer_interval {
            emulator.timers_cycle();
            last_timer_update = now;
        }

        if now.duration_since(last_cpu_update) >= cpu_interval {
            emulator.cycle();
            last_cpu_update = now;
        }
    }
}
