mod notification;
use crate::local_defs;

use notification::NOTIFICATIONS;
use local_defs::{//DISCORD_CERTIFICATE,
    DISCORD_TTS,
    DISCORD_WEBHOOK_URL};
use log::info;

pub struct Discord {
    cert: String,

}

impl Discord {
    pub fn new() -> Self {
        info!("Discord control initialized");
        Self {
            cert = LittleFS.open("/discord.pem", "r").readString();
        }
    }

    pub fn send_message(&self, message: &str) {
        info!("Sending message to Discord: {}", message);
        // Here you would implement the actual logic to send a message to Discord
    }
}