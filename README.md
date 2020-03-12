# Oxide-7
A SNES emulator written in Rust.

### How to run me
`cargo run --release --features="debug" -- [ROM NAME] [--debug (if desired)]`

### Games tested:
* Super Mario World (works pretty well)
* Super Metroid (Intro looks good, gameplay looks broken and eventually freezes.)
* Link to the Past (Intro is a bit messy, some scrolling issues in name select, gameplay seems ok)
* Final Fantasy 2 (IV) (Regressed. There is some sort of corrupted overlay (I think BG3 in Mode 1))
* Final Fantasy 3 (VI) (Title looks ok, reads out of bounds after. If this is ignored then the first "scene" doesn't show as memory reads are too slow. Then they look _mostly_ ok)
* Earthbound (Works pretty well)
* Super Castlevania IV (Works pretty well)
* Super Mario Kart (shows initial nintendo logo then freezes (SPC issues).)
* Mortal Kombat (developer intro plays and then it freezes - it used to get a bit further I think?)
* SimCity (works pretty well)
* Super Mario All-Stars (mostly works. game select menu is unresponsive for some reason, but actual games are ok)
* Aladdin (intro works, title screen is a bit glitchy, gameplay responds but then stops after a short while)
* Zombies Ate My Neighbors! (works pretty well)
* Mega Man X (first screen shows up ok, then freezes. This used to show the whole intro (before commit ~#100)
* Tetris & Dr. Mario (shows an anti-piracy screen!)
* Super Ghouls 'n Ghosts (Intro and title seem fine. Actual gameplay looks a bit corrupted (BG3 in mode 1 issues again I suspect))
* Kirby's Dreamland 3 (Unrecognised ROM config)
* Kirby's Super Star (Unrecognised ROM config)
* Donkey Kong Country (intro and title screen look good, gameplay doesn't show sprites (but some issues are resolved otherwise))
* Donkey Kong Country 2 (the same as above)
* Chrono Trigger (works pretty well)
* Pilotwings (uses interlacing)
* Super Baseball 2020 (works pretty well)
* Dragon Quest 3 (stuck waiting for SPC)
* Final Fantasy V (some odd graphical issues in first cutscene)
* FZero (graphics look completely corrupted)
* Gradius III (looks mostly ok, some odd graphical glitches)
* Mario Paint (title shows up ok, might need SNES mouse to see anything else)
* Super Mario RPG (unrecognised ROM config (SA-1))
* Super Punch-out!! (similar to Mario Kart - nintendo logo then no response)
* Warios Woods (no response, waiting for SPC)

### TODO:

##### Video
- Mode 2, 4 and 6 Offset change per column
- Interlacing
- Improve dirtiness detection in VRAM / move cache creation to CPU side
- Test Modes 5 and 6 more extensively.
- Some issues with things being one scanline "off".

##### Audio
- Bugfixing in SPC-700
- DSP
- Audio output (DAC emulation)

##### System
- Test - does BCD mode work? Also it could do with some cleanup.
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