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
}