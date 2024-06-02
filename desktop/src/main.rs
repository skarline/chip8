use std::{
    env, fs, process,
    time::{Duration, Instant},
};

use sdl2::{event::Event, keyboard::Keycode, pixels::Color, rect::Rect};

use core::{Emulator, DISPLAY_HEIGHT, DISPLAY_WIDTH};

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
    let mut event_pump = sdl_context.event_pump().unwrap();

    let window = video_subsystem
        .window(
            "CHIP-8",
            DISPLAY_WIDTH as u32 * 10,
            DISPLAY_HEIGHT as u32 * 10,
        )
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().accelerated().build().unwrap();

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
                        emulator.key_down(key);
                    }
                }
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => {
                    if let Some(key) = keycode_to_key(keycode) {
                        emulator.key_up(key);
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

            for (i, &pixel) in emulator.get_buffer().iter().enumerate() {
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
