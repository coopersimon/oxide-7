# Oxide-7
A SNES emulator written in Rust.

### How to run me
`cargo run --release --features="debug" -- [ROM NAME] [--debug (if desired)]`

### Games tested:
* Super Mario World (video: some bugged sprites, audio sounds good.)
* Super Metroid (Looks good. Audio sounds good.)
* Link to the Past (Graphics seems ok. Audio sounds good.)
* Final Fantasy 2 (IV) (There is some sort of corrupted overlay (I think BG3 in Mode 1). Otherwise video looks fine. Audio sounds good.)
* Final Fantasy 3 (VI) (Title looks ok, reads out of bounds after. If this is ignored then the first "scene" doesn't show as memory reads are too slow. Then they look _mostly_ ok. Audio is ok, but some serious ringing artifacts.)
* Earthbound (Works pretty well. Audio sounds good. Lowest line of pixels looks odd (might not be a bug - this appears in a few games. Might just need to mask this line).)
* Super Castlevania IV (Works pretty well. Audio sounds good.)
* Mortal Kombat (looks and plays well now. Audio sounds mostly good.)
* SimCity (visuals are fine. Audio is fine.)
* Super Mario All-Stars (works pretty well. sound is pretty good too.)
* Aladdin (intro works, title screen looks fine now, gameplay seems to work ok - occasional black frames. sound is pretty good now, but seems a bit off (extra sounds/intensity?).)
* Zombies Ate My Neighbors! (works pretty well. Audio is mostly fine but with some glitches.)
* Mega Man X (Intro works, and title screen looks ok. The bottom line is incorrect. Audio sounds pretty good but seems to cut out in intro)
* Tetris & Dr. Mario (fixed anti-piracy screen (SRAM issue), now works great)
* Super Ghouls 'n Ghosts (Intro and title seem fine, except for mode 7 in intro. Gameplay mostly appears fine but seems to flicker a bit, and the background looks a bit odd sometimes. Audio sounds mostly fine.)
* Donkey Kong Country (looks good. audio sounds great now.)
* Donkey Kong Country 2 (same as above.)
* Chrono Trigger (works pretty well. Audio is better timed now.)
* Super Baseball 2020 (looks and sounds good)
* Dragon Quest 3 (now works. Graphics look good. Audio is mostly good.)
* Final Fantasy V (mostly looks fine. Some weird instances of colour math not looking quite right. sound is mostly fine but some bass sounds are incorrect)
* FZero (looks good now. audio sounds pretty good.)
* Gradius III (looks mostly ok, some odd graphical glitches, audio sounds pretty good.)
* Mario Paint (title shows up ok, might need SNES mouse to see anything else. audio also sounds ok.)
* Super Punch-out!! (sound seems to work fine, menu graphics look OK, in game looks completely messed up)
* Warios Woods (audio seems fine, graphics look ok now.)
* Prince of Persia (graphics look good, audio sounds good, now gameplay is ok.)
* Prince of Persia 2 (intro shows up and looks _mostly_ fine. Gameplay is black except when paused (?). audio repeats the same broken loop.)
* Shadowrun (Intro is fine, when gameplay begins sprites are bugged, audio sounds decent.)
* International Superstar Soccer (Intro and menus look good. Audio also sounds good)
* Super Star Wars (looks mostly fine. Audio is pretty good but some timings seem a bit off).
* Super Street Fighter II (Looks good (not entirely sure when it started being ok). Audio works good.)
* Ultima VI (title screen, name select and intro look good. audio sounds good. Now loads ok. Not sure when that changed...)
* Ultima VII (intro, title screen looks good. audio sounds _mostly_ ok. Main gameplay menus show up but sprites appear not to.)
* Civilization (looks and sounds good.)
* Harvest Moon (issues here were actually with the ROM format. looks ok. audio is ok but a little strange?)
* Sim City 2000 (issues here were actually with the ROM format. works absolutely fine.)
* Terminator 2 (intro looks and sounds ok, game now plays ok. Not sure when that changed...)
* Wolfenstein 3D (issues here were actually with the ROM format. looks ok, but heavy flickering on top and bottom of screen. music is slow(?) and sound effects have a large delay)
* Super Tennis (extremely temperamental, seems to sometimes work and sometimes not. a bit janky when it does work, but looks fine.)
* Breath of Fire (looks mostly ok, menu text is a bit glitchy, audio is mostly ok. seem unable to open in-game menu?)
* Breath of Fire 2 (looks mostly ok, very laggy intro, audio sounds ok.)
* Tetris Attack (sounds good. Title looks ok, regular gameplay is ok. vs mode just shows a black screen.)
* Contra III (ROM header has too long a name and so it can't recognise the ROM type.)
* Live a Live (Intro has no audio, select menu graphics are broken.)
* Out of this World (Graphics look ok, audio is a bit delayed, and music doesn't play properly.)
* Secret of Mana (Intro is ok, audio is good. Menu is incorrect (hi-res in mode 5/6). Gameplay is fine.)
* Shin Megami Tensei 2 (No audio. Intro and title looks ok. Black screen before main menu, and it stops there).

DSP Games
* Super Mario Kart (shows initial nintendo logo, fades, then crashes due to trying to swap bank 0x08,8479)
* Pilotwings (Title looks good, menu looks a bit broken. Gameplay doesn't work however looks mostly OK. no crashable offences.)

SA-1 Games
* Kirby's Dreamland 3 (Black screen.)
* Kirby's Super Star (Works pretty well. Some black vertical bars in tutorial. Need to test further.)
* Super Mario RPG (intro text works, opening music plays (but is skippy - square audio (see chrono trigger & FFVI)). Intro frame appears but then doesn't progress any further)
* PGA European Tour (Intro and music look ok (some crackling in audio), select menu works. Actual gameplay just shows black screen and stops.)
* PGA Tour 96 (Intro and music look ok, select menu works. Actual gameplay just shows black screen and stops.)


Super FX Games
* Yoshi's Island (intro looks ok in some scenes, BG3 is offset strangely though. audio sounds good. title looks good, with some odd lines on screen. gameplay doesn't seem to work at all yet.)
* StarFox (intro stars look ok but it ends too early. Ship is not visible in title or in control select. Starting main game seems to break completely. Training works ok, frame rate is very questionable. Music/sound effects sounds ok, but are heavily delayed.)
* Doom (title loads up ok. Intro music sounds mostly good. Main game is very broken (LJMP?))
* Stunt Race FX (all graphics look good, including rotating 3d car models. Music/effects sound ok. Main game uses interlace so it crashes.)

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
- Pitch modulation: test
- Fix the slight lag on audio (resampler delay?)
- SPC timing is still off - most noticeable in Chrono Trigger but some games have occasional random notes.
    - Apparently Square games are notorious for this - FF3(VI) and Super Mario RPG also have audio trouble.

##### System
- Cleanup BCD mode
- Ensure timing is correct
    - Should internal cycles take the same amount of time as a data load? (6 cycles for fast rom, 8 for slow)?

##### Extensions
- DSP
    - Bugfixes
- SA-1
    - DMA
    - Variable-length decoding
    - Save data
- SuperFX
    - Timing:
        - Pixel writes need to be buffered properly.
        - Currently has the effect of making the video appear too quickly when there are few polygons, too slowly when there are lots.
    - Re-implement write buffer
    - ROM read buffer (?)
    - Save data

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