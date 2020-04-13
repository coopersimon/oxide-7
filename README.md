# Oxide-7
A SNES emulator written in Rust.

### How to run me
`cargo run --release --features="debug" -- [ROM NAME] [--debug (if desired)]`

### Games tested:
* Super Mario World (video: some bugged sprites, audio sounds good.)
* Super Metroid (Intro looks good, gameplay looks less broken (lower ~1/3 of screen is block colour) and eventually freezes. Audio sounds good.)
* Link to the Past (Intro triforce is glitched, gameplay seems ok. Audio sounds good.)
* Final Fantasy 2 (IV) (There is some sort of corrupted overlay (I think BG3 in Mode 1). Otherwise video looks fine. Audio sounds good.)
* Final Fantasy 3 (VI) (Title looks ok, reads out of bounds after. If this is ignored then the first "scene" doesn't show as memory reads are too slow. Then they look _mostly_ ok. Audio is audible now, but sounds pretty broken.)
* Earthbound (Works pretty well. Audio sounds good, some clipping(?) in the title music.)
* Super Castlevania IV (Works pretty well. Audio sounds good.)
* Super Mario Kart (shows initial nintendo logo then freezes (Might be SPC issues, not sure).)
* Mortal Kombat (looks and plays well now. Audio sounds mostly good.)
* SimCity (visuals are fine. Audio is mostly fine, some clipping(?) in the menu music.)
* Super Mario All-Stars (works pretty well. some video glitches in super mario bros 3 world select. sound is pretty good too.)
* Aladdin (intro works, title screen is a bit glitchy, gameplay responds but then stops after a short while. sound is pretty good now.)
* Zombies Ate My Neighbors! (works pretty well. Audio is mostly fine but with some glitches.)
* Mega Man X (Intro works, but title screen looks a bit glitched. The bottom line is incorrect. Audio is completely broken)
* Tetris & Dr. Mario (shows an anti-piracy screen!)
* Super Ghouls 'n Ghosts (Intro and title seem fine, except for mode 7 in intro. Actual gameplay looks a bit corrupted (BG3 in mode 1 issues again I suspect). Audio sounds decent, with some glitches.)
* Kirby's Dreamland 3 (Unrecognised ROM config (SA-1))
* Kirby's Super Star (Unrecognised ROM config (SA-1))
* Donkey Kong Country (looks good. audio sounds great now.)
* Donkey Kong Country 2 (same as above.)
* Chrono Trigger (works pretty well. Audio works ok in title, broken in first intro scene, then audio is recognisable but broken from then on.)
* Pilotwings (uses interlacing)
* Super Baseball 2020 (looks and sounds good)
* Dragon Quest 3 (now works. Graphics look good. Audio is mostly good, a bit glitched in the intro.)
* Final Fantasy V (quite a lot of incorrect tiles / priorities throughout, overworld is not visible. Audio is pretty good)
* FZero (mode 7 graphics look completely corrupted, sprites look ok. audio sounds pretty good.)
* Gradius III (looks mostly ok, some odd graphical glitches, audio sounds pretty good.)
* Mario Paint (title shows up ok, might need SNES mouse to see anything else. audio also sounds ok.)
* Super Mario RPG (unrecognised ROM config (SA-1))
* Super Punch-out!! (sound seems to work fine, graphics look OK, crashed when gameplay begins due to STP being called (I guess I have to implement it))
* Warios Woods (audio seems fine, graphics are a bit glitched (random lines and some missing graphics))
* Prince of Persia (graphics look good, audio is a bit skippy, gameplay is fine but eventually crashes swapping in bank 0x54,0000)
* Prince of Persia 2 (intro shows up and looks _mostly_ fine. Gameplay is black except when paused (?). audio repeats the same broken loop.)
* Shadowrun (Intro is fine, when gameplay begins sprites are bugged, audio sounds decent.)
* International Superstar Soccer (Intro and menus look good. Audio also sounds good)
* Super Star Wars (looks mostly fine. Audio is pretty good but some timings seem a bit off).
* Super Street Fighter II (Intro plays, at a good frame rate (frame rate was bad before audio branch). unresponsive to controls. Some significant graphical glitches in the intro. Some audio plays but it's broken. Gameplay demo looks ok)
* Ultima VI (title screen, name select and intro look good. audio sounds good. Black screen when gameplay begins (loading address 0x4F,7496))
* Ultima VII (intro, title screen looks good. audio sounds _mostly_ ok. Main gameplay menus show up but sprites appear not to.)
* Civilization (black screen, never starts)
* Harvest Moon (unrecognised ROM)
* Sim City 2000 (unrecognised ROM)
* Terminator 2 (intro looks and sounds ok, used to crash upon start due to swapping bank 0x5B,5F6E, now crashes due to swapping bank 0x3F,7F07)
* Wolfenstein 3D (unrecognised ROM)
* Super Tennis (extremely temperamental, seems to sometimes work and sometimes not. a bit janky when it does work, but looks fine.)

### TODO:

##### Video
- Mode 2, 4 and 6 Offset change per column
- Interlacing
- Improve dirtiness detection in VRAM / move cache creation to CPU side
- Test Modes 5 and 6 more extensively.
- Ensure IRQ correctness.
- Mode 7 mosaic support.

##### Bugs
- Some background corruption.

##### Audio
- Echo
- Noise
- Pitch modulation: test
- Ensure sample correctness!
- Fix the slight lag on audio (resampler delay?)

##### System
- Test - does BCD mode work (some tests say no)? Also it could do with some cleanup.
- Ensure timing is correct

##### Carts
- Only make .sav if necessary.
- Fix bug related to incorrect loading with LoROMs (see MegaManX, Ultima VI).

##### Extensions
- SA-1
- SuperFX

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