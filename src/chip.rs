#[derive(Default)]
struct Registers {
    i: u16,
    v: [u8; 16],
    pc: u16,
    sp: u8,
    dt: u8,
    st: u8
}

pub struct CHIP {
    ram: [u8; 4096], // 4K RAM
    vram: [bool; 2048], // 64x32 resolution
    opcode: u16,
    registers: Registers
}

impl CHIP {
    pub fn new() -> CHIP {
        CHIP {
            ram: [0; 4096],
            vram: [false; 2048],
            opcode: 0,
            registers: Registers::default()
        }
    }

    fn r_byte(&self, addr: u16) -> u8 {
        self.ram[addr as usize]
    }

    fn w_byte(&mut self, addr: u16, value: u8) {
        self.ram[addr as usize] = value;
    }

    fn r_word(&self, addr: u16) -> u16 {
        u16::from_be_bytes([self.r_byte(addr), self.r_byte(addr + 1)])
    }

    fn w_word(&mut self, addr: u16, value: u16) {
        let bytes = value.to_be_bytes();

        self.w_byte(addr, bytes[0]);
        self.w_byte(addr, bytes[1]);
    }

    fn fetch(&mut self) {
        self.opcode = self.r_word(self.registers.pc);
    }

    fn cycle(&mut self) {
        self.fetch();
    }

    fn set_pixel(&mut self, x: u16, y: u16) {
        let i = (x * y) + x;
        self.vram[i as usize] ^= true;
    }

    fn clear(&mut self) {
        self.vram = [false; 2048];
    }
}