# Oxide-7
A SNES emulator written in Rust.

### How to run me
`cargo run --release --features="debug" -- [ROM NAME] [--debug (if desired)]`

### Games tested:
* Super Mario World (works pretty well)
* Super Metroid (Intro looks good, gameplay is a bit broken)
* Link to the Past (Intro is a bit messy, gameplay seems ok)
* Final Fantasy 2 (IV) (Mostly fine, some issues with masking. In actual gameplay there seem to be issues switching between mode 1 (gameplay) and mode 0 (menu))
* Final Fantasy 3 (VI) (Intro is completely corrupted & broken).
* Earthbound (Pattern memory is incorrect - I suspect this is due to me creating too small BG pattern caches)
* Super Castlevania IV (Works well up until BCD is needed)
* Super Mario Kart (breaks immediately because of BCD)
* Mortal Kombat (developer intro plays and then it freezes - it used to get a bit further I think?)
* SimCity (intro and menus work ok, in-game menu is blank. The in-game menu used to work (colour math issues?))
* Super Mario All-Stars (title screen is ok, menu is broken (BG pattern cache issue again?) and actual games seem unresponsive)
* Aladdin (intro works, title screen is a bit glitchy, gameplay responds but then stops after a short while)
* Zombies Ate My Neighbors! (intro is kinda broken, as is the menu, gameplay seems ok)
* Mega Man X (first screen shows up ok, then freezes. This used to show the whole intro (before the last commit))
* Tetris & Dr. Mario (shows an anti-piracy screen!)
* Super Ghouls 'n Ghosts (Intro and title seem fine. Actual gameplay looks a bit corrupted (either priority issues or pattern issues))
* Kirby's Dreamland 3 (Unrecognised ROM config)
* Kirby's Super Star (Unrecognised ROM config)
* Donkey Kong Country (intro mostly looks fine, title screen is broken, gameplay doesn't show sprites)
* Donkey Kong Country 2 (the same as above and eventually breaks due to BCD!)
* Chrono Trigger (uses interlacing and breaks anyway due to BCD)
* Pilotwings (uses interlacing)
* Super Baseball 2020 (intro and title look good, game breaks as soon as gameplay is about to begin due to BCD)

### TODO:

##### Video
- Mode 2, 4 and 6 Offset change per column
- Interlacing
- Performance: don't use current BG cache method
- Correctness: use full range for pattern memory
- Improve dirtiness detection in VRAM / move cache creation to CPU side
- Test Modes 5 and 6 more extensively.
- Screen brightness mode
- Some issues with things being one scanline "off".

##### Audio
- Bugfixing in SPC-700
- DSP
- Audio output (DAC emulation)

##### System
- BCD mode for ADC and SBC (lots of games actually use this)
- Ensure timing is correct

### Style guide (?)
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