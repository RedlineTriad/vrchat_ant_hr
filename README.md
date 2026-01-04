# vrchat_ant_hr

Bridges ANT+ heart-rate sensors to VRChat using OSC. Reads BPM from an ANT+ USB radio and forwards it as a normalized float to VRChat OSC service at `/avatar/parameters/Heartrate`.

## Requirements

- **Hardware**: ANT+ USB radio (dongle) and ANT+ heart-rate sensor
- **System**: libusb development library
  - Debian/Ubuntu: `sudo apt install libusb-1.0-0-dev pkg-config`
  - Fedora: `sudo dnf install libusbx-devel pkgconf-pkg-config`
  - macOS: `brew install libusb pkg-config`
  - Windows: Install libusb and use [Zadig](https://zadig.akeo.ie/) to replace the USB driver
- **Rust**: Toolchain and `cargo`

## USB Permissions (Linux)

Create a UDEV rule to access the USB dongle without root:

```bash
# Find your device IDs
lsusb  # Look for ID xxxx:yyyy

# Create rule (replace IDs with yours)
sudoedit /etc/udev/rules.d/99-ant-usb.rules
# Paste: SUBSYSTEM=="usb", ATTR{idVendor}=="xxxx", ATTR{idProduct}=="yyyy", MODE="0666", TAG+="uaccess"

# Apply and replug
sudo udevadm control --reload-rules && sudo udevadm trigger
```

## Build

```bash
cargo build --release
```

## Run

```bash
./target/release/vrchat_ant_hr
```

### Options

```bash
--bpm <MODE>            BPM calculation mode [default: intra-beat]
                        Options: computed, intra-beat, intra-beat-unfiltered

--output <MODE>         Output destination [default: vrchat]
                        Options: log, vrchat
```

### BPM Modes

| Mode | Description |
|------|-------------|
| `computed` | Sensor's filtered BPM (stable, more latency) |
| `intra-beat` | Calculated from beat time with anomaly filtering **[default]** |
| `intra-beat-unfiltered` | Raw beat time calculation (debugging) |

### Output Modes

| Mode | Description |
|------|-------------|
| `log` | Console logging only |
| `vrchat` | Send to VRChat via OSC **[default]** |

### Examples

```bash
# Log-only mode (testing)
./vrchat_ant_hr --output log

# Use sensor's filtered BPM
./vrchat_ant_hr --bpm computed

# Raw unfiltered BPM
./vrchat_ant_hr --bpm intra-beat-unfiltered --output log
```

### Avatar Setup

Add a parameter named `Heartrate` (Float) to your VRChat avatar. BPM is mapped 0-255 → -1.0 to 1.0.

### Logging

```bash
RUST_LOG=debug ./vrchat_ant_hr
```

## Troubleshooting

| Issue | Solution |
|-------|----------|
| No ANT+ devices found | Check `lsusb`, verify USB permissions, replug dongle |
| Permission denied | Set up UDEV rule (Linux) or use Zadig (Windows) |
| OSC not appearing in VRChat | Enable OSC in VRChat, verify avatar has `Heartrate` parameter |
| Device busy | Close other ANT+ apps, replug device |
