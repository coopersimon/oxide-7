# Oxide-7
A SNES emulator written in Rust.

### How to run me
`cargo run --release --features="debug" -- [ROM NAME] [--debug (if desired)]`

### Games tested:
* Super Mario World (video: some bugged sprites, audio: jump & yoshi sound effects are strange. also frame rate is now very slow in the gameplay)
* Super Metroid (Intro looks good, gameplay looks broken and eventually freezes. Audio sounds pretty good.)
* Link to the Past (Intro is a bit messy, gameplay seems ok. Sound effects and jingles sound pretty good, but no music plays.)
* Final Fantasy 2 (IV) (Regressed. There is some sort of corrupted overlay (I think BG3 in Mode 1). Audio samples are ok but fades in and out randomly)
* Final Fantasy 3 (VI) (Title looks ok, reads out of bounds after. If this is ignored then the first "scene" doesn't show as memory reads are too slow. Then they look _mostly_ ok. No audio at all.)
* Earthbound (Works pretty well. Audio is recognisable but broken throughout.)
* Super Castlevania IV (Works pretty well. Some sound effects play but no music.)
* Super Mario Kart (shows initial nintendo logo then freezes (SPC issues).)
* Mortal Kombat (developer intro plays and then it freezes - it used to get a bit further I think? No sound.)
* SimCity (visuals are fine. Sound is mostly fine but some odd issues (with amplitude seemingly))
* Super Mario All-Stars (works pretty well. game select menu now responds after fixing SPC issues! sound is pretty good too.)
* Aladdin (intro works, title screen is a bit glitchy, gameplay responds but then stops after a short while. sound is completely broken.)
* Zombies Ate My Neighbors! (works pretty well)
* Mega Man X (Intro works, but title screen looks a bit glitched. Some glitched audio. Gameplay breaks due to incorrectly reading bank 0x40.)
* Tetris & Dr. Mario (shows an anti-piracy screen!)
* Super Ghouls 'n Ghosts (Intro and title seem fine. Actual gameplay looks a bit corrupted (BG3 in mode 1 issues again I suspect). Audio is completely glitched.)
* Kirby's Dreamland 3 (Unrecognised ROM config (SA-1))
* Kirby's Super Star (Unrecognised ROM config (SA-1))
* Donkey Kong Country (intro and menus look ok, in gameplay sprites don't show up. sound effects sound good but audio plays half as fast as it should.)
* Donkey Kong Country 2 (same as above.)
* Chrono Trigger (works pretty well, audio is completely broken though)
* Pilotwings (uses interlacing)
* Super Baseball 2020 (looks and sounds good)
* Dragon Quest 3 (stuck waiting for SPC)
* Final Fantasy V (some odd graphical issues in first cutscene, no audio)
* FZero (graphics look completely corrupted, audio sounds pretty good.)
* Gradius III (looks mostly ok, some odd graphical glitches, sound effects seem to work but no music.)
* Mario Paint (title shows up ok, might need SNES mouse to see anything else. audio also sounds ok.)
* Super Mario RPG (unrecognised ROM config (SA-1))
* Super Punch-out!! (similar to Mario Kart - nintendo logo then no response)
* Warios Woods (no response, waiting for SPC)
* Prince of Persia (no response)
* Prince of Persia 2 (intro shows up, a bit broken, then black screen. audio repeats the same broken loop.)
* Shadowrun (Intro is fine, black screen instead of title, no audio.)
* International Superstar Soccer (Intro and menus look good, no audio. When gameplay is about to begin the SPC runs STOP which is completely wrong.)
* Super Star Wars (Visually looks good. Frame rate is choppy for some reason. no audio at all.)
* Super Street Fighter II (Intro plays, at a good frame rate (frame rate was bad before audio branch). unresponsive to controls. Some graphical glitches. Some audio plays but it's broken. In the gameplay demo it looks similarly glitched to Super Metroid)
* Ultima VI (title screen, name select and intro look good. audio sounds _mostly_ ok, but sound effects kill the music. Game dies when gameplay begins due to loading bank 0x4F)
* Ultima VII (intro, title screen looks good. audio sounds _mostly_ ok. Main gameplay menus show up but sprites appear not to.)

### TODO:

##### Video
- Mode 2, 4 and 6 Offset change per column
- Interlacing
- Improve dirtiness detection in VRAM / move cache creation to CPU side
- Test Modes 5 and 6 more extensively.
- Some issues with things being one scanline "off".
- Ensure IRQ correctness.

##### Audio
- Bugfixing in SPC-700 (SBC might be the culprit here.)
- Echo
- Pitch modulation
- Ensure sample correctness!

##### System
- Test - does BCD mode work? Also it could do with some cleanup.
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