# Oxide-7
A SNES emulator written in Rust.

### How to run me
`cargo run --release --features="debug" -- [ROM NAME] [--debug (if desired)]`

### Games tested:
* Super Mario World (video: some bugged sprites, audio sounds good.)
* Super Metroid (Looks good. Audio sounds good.)
* Link to the Past (Intro triforce is glitched, gameplay seems ok. Audio sounds good.)
* Final Fantasy 2 (IV) (There is some sort of corrupted overlay (I think BG3 in Mode 1). Otherwise video looks fine. Audio sounds good.)
* Final Fantasy 3 (VI) (Title looks ok, reads out of bounds after. If this is ignored then the first "scene" doesn't show as memory reads are too slow. Then they look _mostly_ ok. Audio is audible now, but sounds pretty broken.)
* Earthbound (Works pretty well. Audio sounds mostly good. Demo video doesn't show. Lowest line of pixels looks odd.)
* Super Castlevania IV (Works pretty well. Audio sounds good.)
* Super Mario Kart (shows initial nintendo logo, fades, then crashes due to trying to swap bank 0x08,8479)
* Mortal Kombat (looks and plays well now. Audio sounds mostly good.)
* SimCity (visuals are fine. Audio is mostly fine, some clipping(?) in the menu music.)
* Super Mario All-Stars (works pretty well. sound is pretty good too.)
* Aladdin (intro works, title screen looks fine now, gameplay seems to work ok. sound is pretty good now, but seems a bit off (extra sounds/intensity?).)
* Zombies Ate My Neighbors! (works pretty well. Audio is mostly fine but with some glitches.)
* Mega Man X (Intro works, but title screen looks a bit glitched. The bottom line is incorrect. Audio is completely broken)
* Tetris & Dr. Mario (fixed anti-piracy screen (SRAM issue), now works great)
* Super Ghouls 'n Ghosts (Intro and title seem fine, except for mode 7 in intro. Actual gameplay looks a bit corrupted (BG3 in mode 1 issues again I suspect). Audio sounds decent, with some glitches.)
* Kirby's Dreamland 3 (Unrecognised ROM config (SA-1))
* Kirby's Super Star (Unrecognised ROM config (SA-1))
* Donkey Kong Country (looks good. audio sounds great now.)
* Donkey Kong Country 2 (same as above.)
* Chrono Trigger (works pretty well. Audio works ok in title, broken in first intro scene, then audio is recognisable but broken from then on.)
* Pilotwings (Title looks good, menu looks a bit broken. Gameplay doesn't work however looks mostly OK. no crashable offences.)
* Super Baseball 2020 (looks and sounds good)
* Dragon Quest 3 (now works. Graphics look good. Audio is mostly good, a bit glitched in the intro.)
* Final Fantasy V (quite a lot of incorrect tiles / priorities throughout, overworld is not visible. Audio is pretty good)
* FZero (looks good now. audio sounds pretty good.)
* Gradius III (looks mostly ok, some odd graphical glitches, audio sounds pretty good.)
* Mario Paint (title shows up ok, might need SNES mouse to see anything else. audio also sounds ok.)
* Super Mario RPG (unrecognised ROM config (SA-1))
* Super Punch-out!! (sound seems to work fine, menu graphics look OK, in game looks completely messed up)
* Warios Woods (audio seems fine, graphics look ok now. Some missing things and gameplay is broken due to (I think) no offset change per column.)
* Prince of Persia (graphics look good, audio is a bit skippy, gameplay is fine but used to crash (swapping in bank 0x54,0000), now crashes due to swapping bank 0x3F,53AC)
* Prince of Persia 2 (intro shows up and looks _mostly_ fine. Gameplay is black except when paused (?). audio repeats the same broken loop.)
* Shadowrun (Intro is fine, when gameplay begins sprites are bugged, audio sounds decent.)
* International Superstar Soccer (Intro and menus look good. Audio also sounds good)
* Super Star Wars (looks mostly fine. Audio is pretty good but some timings seem a bit off).
* Super Street Fighter II (Looks good (not entirely sure when it started being ok). Some audio plays but it's broken.)
* Ultima VI (title screen, name select and intro look good. audio sounds good. Black screen when gameplay begins (loading address 0x4F,7496))
* Ultima VII (intro, title screen looks good. audio sounds _mostly_ ok. Main gameplay menus show up but sprites appear not to.)
* Civilization (looks and sounds good.)
* Harvest Moon (unrecognised ROM)
* Sim City 2000 (unrecognised ROM)
* Terminator 2 (intro looks and sounds ok, used to crash upon start due to swapping bank 0x5B,5F6E, now crashes due to swapping bank 0x3F,7F07)
* Wolfenstein 3D (unrecognised ROM)
* Super Tennis (extremely temperamental, seems to sometimes work and sometimes not. a bit janky when it does work, but looks fine.)
* Breath of Fire (looks mostly ok, menu text is a bit glitchy, audio is ok but some sounds don't stop. seem unable to open in-game menu?)
* Breath of Fire 2 (looks mostly ok, very laggy intro, audio is completely broken.)
* Tetris Attack (sounds good. Title looks ok, regular gameplay is ok. vs mode just shows a black screen.)

### TODO:

##### Video
- Interlacing
- Improve dirtiness detection in VRAM / move cache creation to CPU side
- Test Modes 5 and 6 more extensively.
- Ensure IRQ correctness.
- Mode 7 mosaic support.
- Ensure offset change per tile correctness (only test so far is with Chrono Trigger)

##### Bugs
- Some background corruption.

##### Audio
- Echo: test
- Noise
- Pitch modulation: test
- Ensure sample correctness!
- Fix the slight lag on audio (resampler delay?)

##### System
- Cleanup BCD mode
- Ensure timing is correct

##### Carts
- Fix bug related to incorrect loading with LoROMs (see MegaManX, Ultima VI).

##### Extensions
- DSP
    - Bugfixes
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