# vrchat_ant_hr

Bridges ANT+ heart-rate sensors to VRChat using OSC. Reads BPM from an ANT+ USB radio and forwards it as a normalized float to VRChat OSC service at `/avatar/parameters/Heartrate`.

## Overview

This Rust application listens for heart-rate data from ANT+ heart-rate monitors (chest straps or other ANT+ HR devices) via an ANT+ USB radio and forwards those measurements to VRChat clients using OSC (Open Sound Control) so you can drive avatar parameters from a real HR sensor.

- Detects ANT+ USB radios plugged into the host machine (selects one if multiple are present)
- Opens an ANT+ Heart Rate Display channel and listens for HR datapages
- Sends each received BPM as a normalized float (-1.0 to 1.0) to the OSC address `/avatar/parameters/Heartrate` for all VRChat clients

## Techstack

- ant (ANT / ANT+ support, via https://github.com/cujomalainey/ant-rs)
- vrchat_osc (sends OSC messages to VRChat clients)

## Requirements

- Hardware: an ANT+ USB radio (dongle) and an ANT+ heart-rate sensor
- System: libusb development library (`libusb-1.0`) available to build/run the `rusb` crate
  - Debian/Ubuntu: `sudo apt install libusb-1.0-0-dev pkg-config`
  - Fedora: `sudo dnf install libusbx-devel pkgconf-pkg-config`
  - macOS: `brew install libusb pkg-config`
  - Windows: install libusb and ensure the USB device driver allows libusb access (Zadig may help)
- Rust toolchain and `cargo` (stable channel recommended)

## USB Permissions (UDEV Setup)

Your user account needs permission to access the ANT+ USB dongle. Instead of running as root (which is a security risk), you can create a UDEV rule.

### Step 1: Find Your Device IDs

Plug in your ANT+ USB dongle, then run:

```bash
lsusb
```

Look for the vendor/product IDs (e.g., `ID 0fcf:1008` → vendor: `0fcf`, product: `1008`).

### Step 2: Create a UDEV Rule

Create a new UDEV rule file:

```bash
sudoedit /etc/udev/rules.d/99-ant-usb.rules
```

Paste the following (replace with your device IDs from Step 1):

```text
SUBSYSTEM=="usb", ATTR{idVendor}=="0fcf", ATTR{idProduct}=="1008", MODE="0666", TAG+="uaccess"
```

### Step 3: Apply the Rule

Reload UDEV rules:

```bash
sudo udevadm control --reload-rules
sudo udevadm trigger
```

### Step 4: Unplug and Replug Your Device

Unplug your ANT+ dongle and plug it back in.

### Troubleshooting UDEV

- Make sure you replaced the vendor/product IDs with your actual device IDs
- Try unplugging and replugging the device after applying the rule

**macOS:** USB permissions usually work automatically. If you encounter permission errors, try unplugging/replugging or run temporarily with `sudo`.

**Windows:** If you get "access denied" errors, use [Zadig](https://zadig.akeo.ie/) to replace the USB driver with libusb.

## Build

From the repository root:

```bash
cargo build --release
```

This creates an optimized binary at `target/release/vrchat_ant_hr`.

## Run

Start the application (ensure USB permissions are set so your user can open the dongle):

```bash
cargo run --release
```

Or run the compiled binary directly:

```bash
./target/release/vrchat_ant_hr
```

- If multiple ANT+ radios are found, the program prompts you to select one
- When the program receives a heart-rate datapoint it logs the BPM and forwards a normalized float to the OSC address `/avatar/parameters/Heartrate`

## Logging

By default, the app logs at `INFO` level. For more verbosity:

```bash
RUST_LOG=debug cargo run --release
```

Heart rate BPM events are logged at `DEBUG` level to avoid console spam.

## How BPM is Mapped to OSC

The raw BPM from the ANT+ datapage is an integer (0-255). The program converts it using:

```rust
normalized = (bpm as f32 / 255.0) * 2.0 - 1.0
```

This maps: 0 BPM → -1.0, 127 BPM → 0.0, 255 BPM → 1.0

OSC message sent:
- Address: `/avatar/parameters/Heartrate`
- Argument: single `Float` with the normalized value

You can change the mapping or the OSC address in `src/ant.rs` and `src/osc.rs`.

## Troubleshooting

**"No ANT+ USB devices found"**
- Ensure the dongle is plugged in and visible to the OS
  - Linux: Check `lsusb` to verify the device appears
  - macOS: Check `system_profiler SPUSBDataType`
- Verify USB permissions (see the UDEV section above)
- Try unplugging and replugging the device

**USB permission errors**
- Add a UDEV rule (see the guide above)
- For quick testing only, you can run with `sudo` (not recommended for regular use)
- On Windows, use Zadig to replace the driver (see the Windows section above)

**OSC messages not appearing in VRChat**
- Ensure VRChat is running and the OSC service/bridge is enabled
- Verify your avatar has a parameter named `Heartrate` (case-sensitive)
- Check that the OSC address in the code matches your avatar parameter name
- Try setting `RUST_LOG=debug` to see if BPM is being received

**"Device busy" or "Device in use" errors**
- Another process might be using the ANT+ dongle
- Close any other ANT+ or USB applications
- Unplug and replug the device

## Contributing

- Please open issues or pull requests for fixes and improvements
- Keep changes focused and document any new configuration options
