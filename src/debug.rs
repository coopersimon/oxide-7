// For stepping through the CPU.
use crate::cpu::CPU;

// Capture of CPU internal state.
pub struct CPUState {
    // Registers
    pub a:      u16,    // Accumulator
    pub x:      u16,    // X-Index
    pub y:      u16,    // Y-Index
    pub s:      u16,    // Stack Pointer
    pub db:     u8,     // Data Bank
    pub dp:     u16,    // Direct Page
    pub pb:     u8,     // Program Bank
    pub p:      u8,     // Processor Status
    pub pe:     u8,     // 6502 Emulator Processor Status
    pub pc:     u16,    // Program Counter
}

impl CPUState {
    pub fn to_string(&self) -> String {
        format!("a: ${:04X} x: ${:04X} y: ${:04X} sp: ${:04X} db: ${:02X} dp: ${:04X}\n\
                pb: ${:02X} pc: ${:04X}\n\
                nvmxdizc: {:08b} e: {:08b}",
                self.a, self.x, self.y, self.s, self.db, self.dp,
                self.pb, self.pc,
                self.p, self.pe)
    }
}
