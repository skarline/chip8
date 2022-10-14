mod chip;

use std::env;
use std::fs::File;
use std::io::Read;

use sdl2::event::Event;

use chip::CHIP;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut rom = [0 as u8; 4096 - 512];

    match args.len() {
        1 => {}
        2 => {
            let mut file = File::open(&args[1]).expect("No such file or directory");
            file.read(&mut rom).unwrap();
        }
        _ => {
            println!("usage: chip8 [rom]");
            return
        }
    }

    let sdl_context = sdl2::init().expect("Couldn't initialise SDL2 context");
    let video_subsystem = sdl_context.video().expect("Couldn't intialise SDL2 video subsystem");

    let window = video_subsystem
        .window("CHIP-8", 640, 320)
        .resizable()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    CHIP::new();

    'event: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'event,
                _ => {}
            }
        }

        canvas.clear();
    }
}
