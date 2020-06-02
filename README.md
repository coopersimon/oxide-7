# Oxide-7
A SNES emulator written in Rust.

### How to run me
`cargo run --release --features="debug" -- [ROM NAME] [--debug (if desired)]`

### Games tested:
* Super Mario World (video: some bugged sprites, audio sounds good.)
* Super Metroid (Looks good. Audio sounds good.)
* Link to the Past (Intro triforce is glitched, gameplay seems ok. Audio sounds good.)
* Final Fantasy 2 (IV) (There is some sort of corrupted overlay (I think BG3 in Mode 1). Otherwise video looks fine. Audio sounds good.)
* Final Fantasy 3 (VI) (Title looks ok, reads out of bounds after. If this is ignored then the first "scene" doesn't show as memory reads are too slow. Then they look _mostly_ ok. Audio is ok, but some serious ringing artifacts.)
* Earthbound (Works pretty well. Audio sounds good. Lowest line of pixels looks odd (might not be a bug - this appears in a few games. Might just need to mask this line).)
* Super Castlevania IV (Works pretty well. Audio sounds good.)
* Super Mario Kart (shows initial nintendo logo, fades, then crashes due to trying to swap bank 0x08,8479)
* Mortal Kombat (looks and plays well now. Audio sounds mostly good.)
* SimCity (visuals are fine. Audio is fine.)
* Super Mario All-Stars (works pretty well. sound is pretty good too.)
* Aladdin (intro works, title screen looks fine now, gameplay seems to work ok - occasional black frames. sound is pretty good now, but seems a bit off (extra sounds/intensity?).)
* Zombies Ate My Neighbors! (works pretty well. Audio is mostly fine but with some glitches.)
* Mega Man X (Intro works, and title screen looks ok. The bottom line is incorrect. Audio sounds pretty good but seems to cut out in intro)
* Tetris & Dr. Mario (fixed anti-piracy screen (SRAM issue), now works great)
* Super Ghouls 'n Ghosts (Intro and title seem fine, except for mode 7 in intro. Gameplay mostly appears fine but seems to flicker a bit, and the background looks a bit odd sometimes. Audio sounds mostly fine.)
* Kirby's Dreamland 3 (Unrecognised ROM config (SA-1))
* Kirby's Super Star (Unrecognised ROM config (SA-1))
* Donkey Kong Country (looks good. audio sounds great now.)
* Donkey Kong Country 2 (same as above.)
* Chrono Trigger (works pretty well. Audio sounds a lot better - birds in opening are a bit bugged (also lacking white noise). Audio has some artifacts and seems to non-deterministically cut out / lose voices from time to time.)
* Pilotwings (Title looks good, menu looks a bit broken. Gameplay doesn't work however looks mostly OK. no crashable offences.)
* Super Baseball 2020 (looks and sounds good)
* Dragon Quest 3 (now works. Graphics look good. Audio is mostly good. Looks a bit dark?)
* Final Fantasy V (mostly looks fine. Some weird instances of colour math not looking quite right. sound is mostly fine but some bass sounds are incorrect)
* FZero (looks good now. audio sounds pretty good.)
* Gradius III (looks mostly ok, some odd graphical glitches, audio sounds pretty good.)
* Mario Paint (title shows up ok, might need SNES mouse to see anything else. audio also sounds ok.)
* Super Mario RPG (unrecognised ROM config (SA-1))
* Super Punch-out!! (sound seems to work fine, menu graphics look OK, in game looks completely messed up)
* Warios Woods (audio seems fine, graphics look ok now. Some missing things and gameplay is broken due to (I think) no offset change per column.)
* Prince of Persia (graphics look good, audio sounds good, now gameplay is ok.)
* Prince of Persia 2 (intro shows up and looks _mostly_ fine. Gameplay is black except when paused (?). audio repeats the same broken loop.)
* Shadowrun (Intro is fine, when gameplay begins sprites are bugged, audio sounds decent.)
* International Superstar Soccer (Intro and menus look good. Audio also sounds good)
* Super Star Wars (looks mostly fine. Audio is pretty good but some timings seem a bit off).
* Super Street Fighter II (Looks good (not entirely sure when it started being ok). Audio works good.)
* Ultima VI (title screen, name select and intro look good. audio sounds good. Now loads ok. Not sure when that changed...)
* Ultima VII (intro, title screen looks good. audio sounds _mostly_ ok. Main gameplay menus show up but sprites appear not to.)
* Civilization (looks and sounds good.)
* Harvest Moon (issues here were actually with the ROM format. looks ok, seems a bit dark though. audio is ok but a little strange?)
* Sim City 2000 (issues here were actually with the ROM format. works absolutely fine.)
* Terminator 2 (intro looks and sounds ok, game now plays ok. Not sure when that changed...)
* Wolfenstein 3D (issues here were actually with the ROM format. looks ok, but heavy flickering on top and bottom of screen. music is slow(?) and sound effects have a large delay)
* Super Tennis (extremely temperamental, seems to sometimes work and sometimes not. a bit janky when it does work, but looks fine.)
* Breath of Fire (looks mostly ok, menu text is a bit glitchy, audio is mostly ok. seem unable to open in-game menu?)
* Breath of Fire 2 (looks mostly ok, very laggy intro, audio sounds ok.)
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
- Fix bug related to incorrect loading with LoROMs (see MegaManX, Ultima VI) - this _seems_ to have gone and I'm not sure when that happened.

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