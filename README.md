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
- Reintroduce video thread if possible - (seems to work on Windows but not MacOS - can take another look at this with metal)
- Use Metal for MacOS

##### Audio
- Everything

##### System
- Check DMA and HDMA
- Check timings
    - CPU mid-line pause
- HiROM
- RAM save files
- Finish joypad mapping
- Move joypad outside of PPU.
- Allow for custom mapping of controls.


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