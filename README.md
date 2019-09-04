# Oxide-7
A SNES emulator written in Rust.

### TODO:

##### Video
- Rendering modes
- Special features:
    - Color math
    - Windowing
    - Column shifting
    - Scroll
- Mode 7
- Sprite pattern mem
- Sprite mem
- Palettes
- Rendering start, end, and lines
- F-blank
- Backgrounds:
    - Better checking of dirty areas (pattern mem)

##### Audio
- Everything

##### System
- Check DMA and HDMA
- Check timings
    - CPU mid-line pause
- Read keyboard for joypad data
- HiROM
- RAM save files


#### Style guide
Order:
- Modules
- Use external
- Use internal (try and avoid super::*)
- enums
- traits
- structs
    - External impl
    - Traits
    - Internal impl