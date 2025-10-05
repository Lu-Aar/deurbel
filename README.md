# deurbel
Making an ordinairy doorbell smart by having a notification through discord and ringing a klikaanklikuit chime.

When the doorbell is pressed there are three notifications:
- A mechanical chime
- A KlikAanKlikUit chime
- A Discord notification

There are also two extra inputs:
- A test button: Added to to test the doorbell functionality
- A mute switch: This can enable/disable the chimes

This project is fully designed in RUST no_std and it's designed to be used with a ESP32-WROOM-32.

## ToDo after clone
When this repo is cloned there is one step which has to be done before the code can be used.
Inside the bin folder the following file needs to be created:
global/mod.rs

This file should contain the following items:
``` rust
pub const WIFI_SSID: &str = "THE_SSID_TO_CONNECT_TO";
pub const WIFI_PASSWORD: &str = "THE_PASSWORD_OF_THE_SSID";
pub const DISCORD_WEBHOOK_URL: &str = "THE_URL_OF_THE_DISCORD_WEBHOOK_TO_USE";
pub const DISCORD_TTS: &str = false; // Set to true when to use Text To Speech
```

## PCB
The PCB was created in KiCad. It's a free opensource program for creating schematics and PCB's.
https://www.kicad.org/