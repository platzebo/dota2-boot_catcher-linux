# Boot Catcher Rust — CachyOS/Linux Dota 2 fork

This repository is a **local CachyOS/Linux fork/port** of DMustache's original Boot Catcher Rust project.

- Upstream repository: <https://github.com/DMustache/boot_catcher>
- Upstream source reference used for this port: <https://github.com/DMustache/boot_catcher/tree/main/src>

The upstream Rust version is Windows-only and uses WinAPI (`GDI BitBlt` + `SendInput`). This fork replaces those parts with Linux/X11/XWayland-compatible capture and input so it can target the Dota 2 Dark Carnival Boot Catcher minigame on CachyOS.

## Showcase

![Boot Catcher gameplay preview](assets/preview.gif)

> **Wayland note:** This is not a native Wayland capture implementation. On a CachyOS Wayland session it works by targeting Dota 2 as an **XWayland** window. If `--list-windows` can see `Dota 2`, this backend can capture it.

## Prerequisites

### System

- CachyOS/Arch Linux or another Linux distro with X11/XWayland libraries.
- A visible Dota 2 window in **Windowed** or **Borderless Window** mode.
- Dota 2 must be visible and not minimized.
- On Wayland, Dota 2 must run through XWayland. If needed, set this Steam launch option:

```text
SDL_VIDEODRIVER=x11 %command%
```

### Packages

Install the required runtime/development libraries:

```bash
sudo pacman -S --needed rustup libx11 libxtst libxext
```

This local machine already has the required X11 libraries installed. Rust is installed via `rustup` under `~/.cargo`.

### Rust toolchain

If `cargo` is not on your PATH in a new shell:

```bash
source ~/.cargo/env
```

Check:

```bash
cargo --version
rustc --version
```

## Build

```bash
cd /home/placebo/brojeggde/boot_catcher_cachyos
source ~/.cargo/env
cargo build --release
```

The binary will be here:

```bash
./target/release/boot_catcher_rs
```

## What changed from upstream

- Replaced Windows `GDI BitBlt` screen capture with X11/XWayland window capture.
- Replaced Windows `SendInput` with XTest key injection for `A`, `D`, and `Space`.
- Added visible-window discovery by title, defaulting to `Dota 2`.
- Added safe dry-run options for capture/FPS testing without sending input.
- Added a default crop for the Dark Carnival Boot Catcher playfield inside the Dota window.
- Default target FPS is `60`.

## Verify Dota window detection

```bash
./target/release/boot_catcher_rs --list-windows
```

You should see something like:

```text
0x140013d    2048x1152    map_state=2    Dota 2
```

If the title is different, pass it explicitly:

```bash
./target/release/boot_catcher_rs --window-title "Dota 2"
```

## Safe FPS/capture dry run — no input sent

Use this before enabling control:

```bash
./target/release/boot_catcher_rs \
  --fps 60 \
  --window-title "Dota 2" \
  --no-control \
  --no-startup \
  --no-click-focus \
  --dry-run-frames 180 \
  --debug
```

The debug output prints live FPS/tracking info and writes profiling to `boot_catcher_debug.tsv` unless `--log-file` is changed.

## Run against Dota 2 windowed mode

1. Start Dota 2 in **Windowed** or **Borderless Window** mode.
2. Open the Dark Carnival Boot Catcher minigame.
3. Keep the Dota window visible; do not minimize it.
4. Run:

```bash
cd /home/placebo/brojeggde/boot_catcher_cachyos
./target/release/boot_catcher_rs --fps 60 --window-title "Dota 2" --no-startup --debug
```

Remove `--no-startup` only if you want the tool to send its initial `D`/`Space`/`A`/`Space` sequence.

## Options

- `--fps 60` — target loop rate.
- `--window-title "Dota 2"` — select the Dota window by title substring.
- `--debug` — print live FPS/tracking info.
- `--log-file PATH` — write TSV profiling/tracking log to a custom path.
- `--template PATH` — use a custom boot template PNG size reference.
- `--no-startup` — do not send the initial startup key sequence.
- `--no-click-focus` — do not move/click the mouse to focus the window.
- `--no-control` — capture and track only; do not send `A`/`D`.
- `--dry-run-frames N` — exit after N frames.
- `--game-rect L,T,W,H` — override the playfield crop, relative to the Dota window.
- `--full-window` — old behavior; analyze the whole Dota window. This is slower and can pick up side UI/art as false objects.
- `--use-shm` — opt-in MIT-SHM capture. This can be faster on native X11, but can fail under some Wayland/XWayland/container setups, so it is disabled by default.
- `--list-windows` — print visible X11/XWayland windows and exit.

## Playfield crop / right-steering fix

The first Linux port captured the entire 2048x1152 Dota window. In Dark Carnival, the actual Boot Catcher playfield is only the centered modal area. The old detector could see the jester/sidebar art and other red UI elements as candidate objects, so the target frequently appeared to be on the right side of the full Dota window. That caused continuous `D`/right steering.

The default capture is now cropped to the playfield. On a 2048x1152 Dota window the automatic crop is approximately:

```text
left=682 top=165 width=688 height=916
```

If your UI scale differs and the cart is still wrong, run a dry test with `--debug` and override the crop:

```bash
./target/release/boot_catcher_rs \
  --fps 60 \
  --window-title "Dota 2" \
  --game-rect 682,165,688,916 \
  --no-control \
  --dry-run-frames 180 \
  --debug
```

Then inspect the log:

```bash
tail -40 boot_catcher_debug.tsv
```

Useful columns:

- `cart_track_x` — detected/estimated cart center.
- `target_x` — current boot target x-position.
- `error` — `target_x - cart_x`; positive means move right, negative means move left.
- `action` / `key` — chosen movement.

## Performance notes

On the Hermes test run with the cropped Dota playfield, the default XGetImage backend ran above 60 FPS:

```text
Capture backend: XGetImage
capture 688x916
fps avg ~84
```

A full 2048x1152 Dota window is much slower with plain XGetImage and previously measured around ~26 FPS. If you need stable 60 FPS:

- Keep the default crop enabled.
- Prefer a smaller Dota window, e.g. 1280x720 or 1600x900.
- Try native X11 instead of Wayland/XWayland if available.
- Try `--use-shm` only on native host X11; if it crashes/fails on your compositor, run without it.

## Fair-play warning

This tool sends automated keyboard input to the selected window. Use it only in contexts where this is allowed, such as local/custom/offline testing. Do not use it in matchmaking or anywhere it could violate Dota 2/Steam rules.
