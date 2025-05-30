# Oasis Software

The software for this project is divided up into four subdirectories:

- esp32/ - the actual code that runs on the main control board of the terrarium.
- client/ - a command-line client program that runs on linux/macos/windows(?) for controlling the terrarium and querying its state.
- demoserver/ - a server program that runs on your computer (again linux/macos/windows(?)) that allows for developing/testing the web interface without requiring an esp32-based control board.
- terralib/ - shared code that is used by the above three programs.

## ESP32 code

> [!WARNING]
> It is possible to fry the mister circuit on this board by changing the firmware code. The mister circuit expects a ~110kHz 50% duty cycle signal from the esp32. If you instead turn it fully on, it will likely let out the magic smoke within just a few seconds.

### Initial setup

The initial project in the esp32/ directory was setup from a template:

`cargo generate esp-rs/esp-idf-template cargo`

If stuff needs updating, do one or all of the below:

```
rustup update nightly

cargo update

# manually update versions of packages by editing Cargo.toml
```

### Build a single binary

`cargo build --bin oasis` or `cargo build --bin blinky`

### Re-compile when something changes using cargo-watch

`cargo-watch -x 'build --bin oasis'`

### Run oasis program

`cd esp32 && cargo run --release --bin oasis`

### ESP32 OTA Flashing

As of 3/29/25, the oasis program is about 1.7M in size. The total flash size of the ESP32-C3 is 4M. For ota (over-the-air updates) to work, we have to have room for _two_ versions of the app plus the nvs partition (for wifi calibration data), data partition (oasis config file), and phy_init (RF Data, not sure?). Looks like unless we can significantly reduce the size of the oasis app binary, OTA is not an option - there just isn't enough room on the flash memory.
