# Boot Catcher Linux

Linux/X11/XWayland fork of DMustache's Dota 2 Boot Catcher bot.

- Upstream repository: <https://github.com/DMustache/boot_catcher>
- Upstream source reference: <https://github.com/DMustache/boot_catcher/tree/main/src>
- This fork replaces the original Windows capture/input code with Linux X11/XWayland capture and XTest keyboard input.

## Showcase

![Boot Catcher gameplay preview](assets/preview.gif)

## Quick start

1. Download the latest Linux release from GitHub:

   <https://github.com/platzebo/dota2-boot_catcher-linux/releases>

2. Extract it:

   ```bash
   tar -xzf boot_catcher_rs-linux-x86_64.tar.gz
   chmod +x boot_catcher_rs
   ```

3. Start Dota 2 in **Windowed** or **Borderless Window** mode.

4. Set Dota 2 to **60 FPS** if possible.

5. Open the Dark Carnival **Boot Catcher** minigame.

6. Run the bot:

   ```bash
   ./boot_catcher_rs --fps 60 --window-title "Dota 2" --no-startup --debug
   ```

7. Switch back to Dota 2 and keep the **Dota 2 window active/focused and visible**.

8. Press **Space** once in Dota 2 to start Boot Catcher.

The bot watches the Boot Catcher playfield and sends `A` / `D` keyboard input to move the cart.

Stop it with `Ctrl+C`.

## Important notes

- The Dota 2 window must stay active/focused while the bot runs.
- The Dota 2 window must stay visible; minimized windows cannot be captured.
- On Wayland, Dota 2 must run through XWayland. If needed, use this Steam launch option:

  ```text
  SDL_VIDEODRIVER=x11 %command%
  ```

- Default target loop rate is **60 FPS**:

  ```bash
  --fps 60
  ```

- Tested setup:

  ```text
  Dota window:       2048x1152
  Capture/playfield: 688x916 at offset 682,165
  Measured capture:  ~84 FPS with XGetImage on CachyOS/XWayland
  ```

The bot automatically crops to the Boot Catcher playfield. This avoids false detections from the Dota UI/sidebar art and is faster than full-window capture.

## Check if the bot can see Dota 2

```bash
./boot_catcher_rs --list-windows
```

Expected output should include something like:

```text
0x140013d    2048x1152    map_state=2    Dota 2
```

If your Dota window title is different, pass a different title:

```bash
./boot_catcher_rs --fps 60 --window-title "YOUR WINDOW TITLE" --no-startup --debug
```

## Safe test run without keyboard control

Use this if you only want to test capture/tracking first:

```bash
./boot_catcher_rs \
  --fps 60 \
  --window-title "Dota 2" \
  --no-control \
  --no-startup \
  --no-click-focus \
  --dry-run-frames 180 \
  --debug
```

## Useful options

- `--fps 60` — target bot loop rate.
- `--window-title "Dota 2"` — select the Dota 2 window.
- `--debug` — print FPS/tracking info.
- `--no-startup` — do not send the automatic startup key sequence.
- `--no-control` — track only; do not send `A` / `D`.
- `--dry-run-frames N` — exit after N frames.
- `--game-rect L,T,W,H` — manually override the playfield crop.
- `--full-window` — capture the full Dota window. Not recommended for normal use.
- `--list-windows` — print visible X11/XWayland windows and exit.

## Requirements for building from source

If you only use the release download, you do not need Rust.

For building locally on CachyOS/Arch:

```bash
sudo pacman -S --needed rustup libx11 libxtst libxext
source ~/.cargo/env
cargo build --release
```

The binary will be:

```bash
target/release/boot_catcher_rs
```

## GitHub Actions releases

The GitHub Actions workflow builds a Linux x86_64 release artifact on every branch/tag. Version tags create a GitHub Release.

Example:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Release artifacts:

- `boot_catcher_rs-linux-x86_64.tar.gz`
- `boot_catcher_rs`
- `SHA256SUMS`

## Fair-play warning

This tool sends automated keyboard input to the selected Dota 2 window. Use it only in contexts where this is allowed, such as local/custom/offline testing. Do not use it in matchmaking or anywhere it could violate Dota 2/Steam rules.
