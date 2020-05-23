// Processor tests.
use super::*;

#[test]
fn decimal_add_8bit() {
    let mut cpu = CPU::new(MemBus::new("./roms/SuperMetroid.sfc", "./empty.sav", None));
    cpu.pe = false;
    cpu.set_p(0x20);
    cpu.a = 0x1234;

    cpu.dec_add(0x6);

    assert_eq!(cpu.a, 0x1240);
    assert_eq!(cpu.p.bits(), 0x20);

    cpu.a = 0x1234;

    cpu.dec_add(0x81);

    assert_eq!(cpu.a, 0x1215);
    assert_eq!(cpu.p.bits(), 0x21); // Carry

    cpu.set_p(0x20);
    cpu.a = 0x9876;

    cpu.dec_add(0x24);

    assert_eq!(cpu.a, 0x9800);
    assert_eq!(cpu.p.bits(), 0x23); // Carry, Zero
}

#[test]
fn decimal_add_16bit() {
    let mut cpu = CPU::new(MemBus::new("./roms/SuperMetroid.sfc", "./empty.sav", None));
    cpu.pe = false;
    cpu.set_p(0x00);
    cpu.a = 0x1234;

    cpu.dec_add(0x6);

    assert_eq!(cpu.a, 0x1240);
    assert_eq!(cpu.p.bits(), 0x00);

    cpu.a = 0x1234;

    cpu.dec_add(0x81);

    assert_eq!(cpu.a, 0x1315);
    assert_eq!(cpu.p.bits(), 0x00);

    cpu.a = 0x9876;

    cpu.dec_add(0x124);

    assert_eq!(cpu.a, 0x0000);
    assert_eq!(cpu.p.bits(), 0x03); // Carry, Zero
}

#[test]
fn decimal_sub_8bit() {
    let mut cpu = CPU::new(MemBus::new("./roms/SuperMetroid.sfc", "./empty.sav", None));
    cpu.pe = false;
    cpu.set_p(0x21);
    cpu.a = 0x1234;

    cpu.dec_sub(0x6);

    assert_eq!(cpu.a, 0x1228);
    assert_eq!(cpu.p.bits(), 0x21);

    cpu.a = 0x1234;

    cpu.dec_sub(0x55);

    assert_eq!(cpu.a, 0x1279);
    assert_eq!(cpu.p.bits(), 0x20);

    cpu.set_p(0x20);
    cpu.a = 0x1230;

    cpu.dec_sub(0x29);

    assert_eq!(cpu.a, 0x1200);
    assert_eq!(cpu.p.bits(), 0x23);
}

#[test]
fn decimal_sub_16bit() {
    let mut cpu = CPU::new(MemBus::new("./roms/SuperMetroid.sfc", "./empty.sav", None));
    cpu.pe = false;
    cpu.set_p(0x00);
    cpu.a = 0x1234;

    cpu.dec_add(0x6);

    assert_eq!(cpu.a, 0x1240);
    assert_eq!(cpu.p.bits(), 0x00);

    cpu.a = 0x1234;

    cpu.dec_add(0x81);

    assert_eq!(cpu.a, 0x1315);
    assert_eq!(cpu.p.bits(), 0x00);

    cpu.a = 0x9876;

    cpu.dec_add(0x124);

    assert_eq!(cpu.a, 0x0000);
    assert_eq!(cpu.p.bits(), 0x03); // Carry, Zero
}