// Common utils
use bitflags::bitflags;

/**** BIT MACROS ****/
// Single bit selection.
macro_rules! bit {
    ($bit_num:expr) => {
        bit!($bit_num, u8)
    };
    ($bit_num:expr, u8) => {
        (1 << $bit_num) as u8
    };
    ($bit_num:expr, u16) => {
        (1 << $bit_num) as u16
    };
    ($bit_num:expr, u32) => {
        (1 << $bit_num) as u32
    };
}

// Multiple bit selection.
macro_rules! bits {
    [ $($bit_num:expr),* ] => {
        $(bit!($bit_num, u8))|*
    };
}

macro_rules! bits16 {
    [ $($bit_num:expr),* ] => {
        $(bit!($bit_num, u16))|*
    };
}

// Check if a bit is set.
macro_rules! test_bit {
    ($val:expr, $bit_num:expr) => {
        test_bit!($val, $bit_num, u16)
    };
    ($val:expr, $bit_num:expr, u8) => {
        ($val & bit!($bit_num, u8)) != 0
    };
    ($val:expr, $bit_num:expr, u16) => {
        ($val & bit!($bit_num, u16)) != 0
    };
    ($val:expr, $bit_num:expr, u32) => {
        ($val & bit!($bit_num, u32)) != 0
    };
}

/**** BYTE MACROS ****/
macro_rules! lo_nybble {
    ($val:expr) => {
        $val & 0xF
    };
}

macro_rules! hi_nybble {
    ($val:expr) => {
        $val >> 4
    };
}

// Make a 16-bit value from two 8-bit values.
macro_rules! make16 {
    ($hi:expr, $lo:expr) => {
        (($hi as u16) << 8) | ($lo as u16)
    };
}

// Get the low byte of a 16-bit value.
macro_rules! lo {
    ($val:expr) => {
        $val as u8
    };
}

// Set the low byte of a 16-bit value.
macro_rules! set_lo {
    ($val:expr, $lo:expr) => {
        ($val & 0xFF00) | ($lo as u16)
    };
}

// Get the high byte of a 16-bit value.
macro_rules! hi {
    ($val:expr) => {
        ($val >> 8) as u8
    };
}

// Set the high byte of a 16-bit value.
macro_rules! set_hi {
    ($val:expr, $hi:expr) => {
        ($val & 0x00FF) | (($hi as u16) << 8)
    };
}


// Make a 24-bit value from an 8-bit and a 16-bit value.
macro_rules! make24 {
    ($hi:expr, $lo:expr) => {
        (($hi as u32) << 16) | ($lo as u32)
    };
    ($hi:expr, $mid:expr, $lo:expr) => {
        (($hi as u32) << 16) | (($mid as u32) << 8) | ($lo as u32)
    };
}

// Get the lowest byte or 2 bytes of a 24-bit value.
macro_rules! lo24 {
    ($val:expr) => {
        lo24!($val, u16)
    };
    ($val:expr, u8) => {
        $val as u8
    };
    ($val:expr, u16) => {
        $val as u16
    };
}

// Set the lowest byte or 2 bytes of a 24-bit value.
macro_rules! set_lo24 {
    ($val:expr, $lo:expr) => {
        set_lo24!($val, $lo, u8)
    };
    ($val:expr, $lo:expr, u8) => {
        ($val & 0xFFFFFF00) | ($lo as u32)
    };
    ($val:expr, $lo:expr, u16) => {
        ($val & 0xFFFF0000) | ($lo as u32)
    };
}

// Get the middle byte of a 24-bit value.
macro_rules! mid24 {
    ($val:expr) => {
        ($val >> 8) as u8
    };
}

// Set the middle byte of a 24-bit value.
macro_rules! set_mid24 {
    ($val:expr, $mid:expr) => {
        ($val & 0xFFFF00FF) | (($mid as u32) << 8)
    };
}

// Get the highest byte of a 24-bit value (or, the second highest byte of a 32-bit value).
macro_rules! hi24 {
    ($val:expr) => {
        ($val >> 16) as u8
    };
}

// Set the high byte of a 24-bit value.
macro_rules! set_hi24 {
    ($val:expr, $hi:expr) => {
        ($val & 0xFF00FFFF) | (($hi as u32) << 16)
    };
}

// Get the low 16-bits of a 32-bit value.
macro_rules! lo32 {
    ($val:expr) => {
        lo24!($val, u16)
    };
}

// Interrupts that can be triggered from devices.
bitflags! {
    #[derive(Default)]
    pub struct Interrupt: u8 {
        const NMI   = bit!(0);  // Indicates that V-Blank and NMI have occurred.
        const IRQ   = bit!(1);  // Indicates that an IRQ was triggered.

        const VBLANK = bit!(2); // Indicates that V-Blank without NMI happened.
    }
}
