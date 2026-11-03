# FurryTel BadgeOS

Firmware for the [FurryTel badge](https://furrytel.net) (RP2040, 128×32 monochrome display, five
buttons, three LEDs and an RGB backlight). It shows a name card, QR code link pages, pictures and a
note, and includes five games with saved high scores. Content is set in `badge.yaml`.

![The home screen, the files list and the LED settings](docs/screens.png)

## What it does

- **Home**: the name and species or title from `badge.yaml`, with a paw graphic.
- **Files**: pictures (baked in from `badge.yaml`) and one note you type on the badge with the
  on-screen keyboard. The note is stored in flash.
- **Links**: one page per link, each with a 29×29 QR code, a title and a label. Example card:

  ![The GitHub link card](docs/link-card.png)

- **Games**: Snake, Flappy, Frogger, a knock-off Tetris (with standard SRS rotation), and Doom (a
  raycaster with three difficulties, three kinds of monster, and levels that get harder until you die). The D-pad plays; three Lefts leave a game (in Tetris,
  three presses of the button on the player's left). Tetris is drawn sideways, with the badge
  turned clockwise:

  ![The Tetris start screen and a game in progress, sideways](docs/tetris.png)
- **Scores**: every game keeps its top three, saved in flash. A score that makes the list asks for
  three initials (Up/Down change the letter, Left/Right move, Center confirms). The Scores screen
  in the Games menu shows each game's list.
- **Colors**: backlight presets and RGB sliders, plus a separate color for the flash on each button
  press.
- **LEDs**: the three LEDs can blink randomly, hold a color, chase, or light on a button press.
- **About / Status / Bootloader**: firmware info, battery and uptime, and a reboot into the USB
  bootloader.

## Download, Customize, Flash

### 1. Download

```sh
mkdir badgeos && cd badgeos
curl -L https://github.com/rainyvalley/FurryTel-BadgeOS/archive/refs/heads/main.tar.gz | tar -xz --strip-components=1
```

With git instead: `git clone https://github.com/rainyvalley/FurryTel-BadgeOS.git`.

### 2. Customize

Everything personal is in [`badge.yaml`](badge.yaml) — open it in any text editor:

```yaml
name: YCH             # up to 8 characters look best; longer names use a narrower font
species: Plank        # up to 10 characters

links:
  - title: GitHub
    url: https://github.com/rainyvalley

pictures:
  - name: Paws
    image: art/paws.png
  - name: FurryTel
    image: art/furrytel.png
```

A name of up to 8 characters uses the big font; longer names shrink to fit, and 16 characters is
the hard limit. Each link's URL must fit a 29×29 QR code, about 53 characters.

Pictures are fitted inside the screen, centred, and turned into black and white; small images stay
small unless `scale: 2` is set (or use `dither: true` / `threshold: 128` for a different look).
Logos and line art convert best.

Then build the firmware:

```sh
rustup target add thumbv6m-none-eabi                 # once; rust-toolchain.toml pins the rest
python3 -m pip install --user pyyaml pillow qrcode   # or use a virtualenv
python3 tools/build.py                               # writes dist/badgeos.uf2
```

`tools/build.py` regenerates the QR codes and pictures, compiles the firmware, and wraps it as a
UF2 file.

**Multiple badges:** copy `badge.yaml` to `badges/someone.yaml`, edit it, and build that one:

```sh
mkdir -p badges
cp badge.yaml badges/someone.yaml
python3 tools/build.py --config badges/someone.yaml   # writes dist/someone.uf2
```

Only one config is generated at a time (`src/generated.rs`); rebuild with the matching config
before flashing each badge.

### 3. Flash

1. Put the badge in bootloader mode — either hold the small **BOOTSEL** button on the board while
   plugging the USB cable in, or pick **Bootloader** in the badge's menu and press Center.
2. Copy your `.uf2` file onto the drive that appears, called `RPI-RP2`.
3. The badge restarts by itself.

## Controls

| Where | What the buttons do |
| --- | --- |
| Home | Center or Left opens the menu |
| Menu | Up/Down move, Center or Right open, Left goes back |
| Files | Up/Down pick a file, Center or Right opens it, any button closes a picture, Left goes back |
| Note | D-pad moves around the keyboard, `OK` saves, `esc` cancels |
| Links | Up/Down or Left/Right turn pages, Center goes back |
| Games | D-pad plays, Center starts/pauses/retries, three Lefts quit (Flappy flaps on Center) |
| Doom | Title: Up/Down pick the difficulty, Center starts. Up/Down walk, Left/Right turn, Center fires (and reloads when empty); Up Down Up Down pauses. Between levels Center continues and three Lefts stop with the score |
| Scores | Up/Down flip between games, Left goes back |
| Tetris | turn the badge clockwise — the buttons turn with it: up=right, down=left, left=pause, right=drops to the bottom, Center rotates |
| Colors | Left/Right pick a field, Up/Down change it, Center saves and finishes |
| LEDs | Left/Right pick a field, Up/Down change it, Center saves and finishes |
| About / Status | any button turns the page or leaves |
| Bootloader | Center reboots into USB bootloader, any other button leaves |

## Credits

This is an unofficial project, built on the [FurryTel badge firmware][original] and the
[FurryTel badge hardware][hardware] from [furrytel.net](https://furrytel.net). The
FurryTel name and logo are theirs, and the original display, menu and keyboard code is the
foundation everything here stands on.

The original firmware does not include a license file, so if you plan to redistribute this project,
please check with the FurryTel project first.

[FurryTel](https://furrytel.net) · [original firmware][original] · [hardware][hardware]

## A note on AI assistance

Parts of this project — code, configuration, and documentation — were written with the help of generative AI. Everything in here was reviewed, tested, and corrected by a human before being committed; nothing was accepted on the model's say-so.

## License

MIT — see [LICENSE](LICENSE).

[original]: https://github.com/FurryTel/badge-firmware
[hardware]: https://github.com/FurryTel/badge
