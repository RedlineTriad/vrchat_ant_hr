# vrchat_ant_hr

Bridges ANT+ heart-rate sensors to VRChat using OSC. Reads BPM from an ANT+ USB radio and forwards it as a normalized float to the VRChat OSC service at `/avatar/parameters/Heartrate`.

## Overview

This small Rust application listens for heart-rate data from ANT+ heart-rate monitors (chest straps or other ANT+ HR devices) via an ANT+ USB radio and forwards those measurements to VRChat clients using OSC (Open Sound Control) so you can drive avatar parameters from a real HR sensor.

Key behaviors:
- Detects ANT+ USB radios plugged into the host machine (selects one if multiple are present).
- Opens an ANT+ Heart Rate Display channel and listens for HR datapages.
- Sends each received BPM as a normalized float (approx. -1.0..1.0) to the OSC address `/avatar/parameters/Heartrate` for all VRChat clients.

The program combines synchronous USB/ANT interactions (blocking thread) with async networking via `tokio` to integrate low-level hardware access and the VRChat OSC service.

## Technologies and crates used

- Rust (edition 2021)
- ant (ANT / ANT+ support, via https://github.com/cujomalainey/ant-rs)
- rusb (libusb bindings for USB access)
- vrchat_osc (sends OSC messages to VRChat clients)
- tokio (async runtime)
- dialoguer (interactive device selection)
- env_logger / log (logging)

See `Cargo.toml` for the full dependency list.

## Requirements

- Hardware: an ANT+ USB radio (dongle) and an ANT+ heart-rate sensor.
- System: libusb development library (`libusb-1.0`) available to build/run the `rusb` crate.
  - Debian/Ubuntu: `sudo apt install libusb-1.0-0-dev pkg-config`
  - Fedora: `sudo dnf install libusbx-devel pkgconf-pkg-config`
  - macOS: `brew install libusb pkg-config`
  - Windows: install libusb and ensure the USB device driver allows libusb access (Zadig may help).
- Rust toolchain and `cargo` (stable channel recommended).

USB permissions: the process must be able to open the ANT+ USB dongle. Prefer creating a `udev` rule for your device rather than running the app as root. Example udev rule (replace vendor/product IDs):

```text
SUBSYSTEM=="usb", ATTR{idVendor}=="XXXX", ATTR{idProduct}=="YYYY", MODE="0666", GROUP="plugdev"
```

After adding a udev rule: `sudo udevadm control --reload-rules && sudo udevadm trigger`.

## Build

From the repository root:

```bash
cargo build --release
```

## Run

Start the application (ensure USB permissions are set so your user can open the dongle):

```bash
cargo run --release
```

- If multiple ANT+ radios are found, the program prompts you to select one.
- When the program receives a heart-rate datapoint it logs the BPM and forwards a normalized float to the OSC address `/avatar/parameters/Heartrate`.
- Press `Ctrl+C` to stop.

## How BPM is mapped to OSC

- Raw BPM from the ANT+ datapage is an integer (0..255). The program converts it with:

```rust
normalized = (bpm as f32 / 255.0) * 2.0 - 1.0
```

- OSC message sent:
  - Address: `/avatar/parameters/Heartrate`
  - Argument: single `Float` with the normalized value

You can change the mapping or the OSC address in `src/main.rs`.

## Customization

- ANT+ network key: the code uses the typical ANT+ network key in `src/main.rs`—replace it if you have a different key.
- OSC recipients: the code sends to recipients matching `"VRChat-Client-*"`; adjust this pattern or address if needed.
- Logging: set `RUST_LOG` for verbosity, e.g. `RUST_LOG=debug cargo run --release`.

## Troubleshooting

- "No ANT+ USB devices found": ensure the dongle is plugged in and visible to the OS (check `lsusb` on Linux). Verify libusb permissions.
- USB permission errors: add a udev rule or run temporarily with `sudo` for testing.
- OSC messages not appearing in VRChat: ensure VRChat OSC service/bridge is running and your avatar has a parameter named `Heartrate`.
- Push/permission errors while running `cargo run`: check that no other process is holding the USB device.

## Files of interest

- `src/main.rs` — main application logic and comments.
- `Cargo.toml` — dependencies and crate versions.

## Contributing

- Please open issues or pull requests for fixes or improvements.
- Keep changes focused and document any new configuration options.

## Notes

- This program interacts with hardware and network services. Avoid running with elevated privileges whenever possible; prefer granting device access to your user via OS facilities.
- If you want, I can add a small `udev` helper file or a config file to make OSC address and normalization adjustable at runtime—tell me which you prefer and I can add it.
