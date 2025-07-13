mod notification;
mod discord_cert;

use esp_wifi::wifi::WifiController;
use notification::NOTIFICATIONS;
use crate::global::{//DISCORD_CERTIFICATE,
    DISCORD_TTS,
    DISCORD_WEBHOOK_URL};
use log::info;

use reqwless::{
    client::HttpClient,
    request::{Method, RequestBuilder},
    headers::ContentType,
};
#[cfg(feature = "esp-mbedtls")]
use esp_mbedtls::{Certificates, TlsVersion, X509};

use discord_cert::DISCORD_CERT;
use alloc::format;

pub struct Discord {
    // wifi: &WifiController,

}

impl Discord {
    pub fn new(wifi: &WifiController) -> Self {
        info!("Discord control initialized");
        Self {
            // wifi: wifi
            // cert: self.read_pem_file("./certs/discord.pem"),
        }
    }

    pub async fn send_discord(&self, content: &str, embed_json: &str) -> Result<(), ()> {

        let mut client = HttpClient::new_with_tls(stack, dns, tls_config);
        // let mut client = HttpClient::new(self.wifi, TlsConfig {
        //     version: TlsVersion::Tls12,
        //     certificates: DISCORD_CERT,
        //     ..Default::default()
        // });

        let json_payload = format!(
            "{{\"content\": \"{}\", \"tts\": {}, \"embeds\": [{}]}}",
            content, DISCORD_TTS, embed_json
        );

        let mut buffer = [0u8; 1024];

        let mut response = client
            .request(Method::POST, DISCORD_WEBHOOK_URL)
            .await
            .unwrap()
            .content_type(ContentType::ApplicationJson)
            .headers(&[("Host", "discord.com")])
            .body(json_payload.as_bytes())
            .send(&mut buffer)
            .await
            .unwrap();
        Ok(())
    }

    pub fn send_message(&self, message: &str) {
        self.send_discord(message, "");
    }

    pub fn send_embed(&self, embed_json: &str) {
        self.send_discord("", embed_json);
    }
}