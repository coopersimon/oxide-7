// Common utils

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

// Make a 24-bit value from an 8-bit and a 16-bit value.
macro_rules! make24 {
    ($hi:expr, $lo:expr) => {
        (($hi as u32) << 16) | ($lo as u32)
    };
    ($hi:expr, $mid:expr, $lo:expr) => {
        (($hi as u32) << 16) | (($mid as u32) << 8) | ($lo as u32)
    };
}

// Get the lowest two bytes of a 24-bit value.
macro_rules! lo24 {
    ($val:expr) => {
        $val as u16
    };
}

// Get the highest byte of a 24-bit value (or, the second highest byte of a 32-bit value).
macro_rules! hi24 {
    ($val:expr) => {
        ($val >> 16) as u8
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