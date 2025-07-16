use anyhow::Result;
use embedded_svc::http::{client::Client, Method};
use esp_idf_svc::{http::client::EspHttpConnection, http::client::Configuration, http::client::Response};

use crate::global::{
    DISCORD_TTS,
    DISCORD_WEBHOOK_URL};

use esp_idf_svc::sys::esp_crt_bundle_attach;

pub struct Discord {

}

impl Discord {
    pub fn new() -> Self {
        println!("Discord control initialized");
        Self {
        }
    }

    pub fn send_discord(&self, content: &str, embed_json: &str) -> Result<()> {

        let config = Configuration {
            crt_bundle_attach: Some(esp_crt_bundle_attach),
            ..Default::default()
        };

        // let url = "http://192.168.1.105/rpc/Cover.Open";
        // let body = r#"{"id":0}"#;

        let url = DISCORD_WEBHOOK_URL;
        let body = format!(
            "{{\"content\": \"{}\", \"tts\": {}, \"embeds\": [{}]}}",
            content, DISCORD_TTS, embed_json
        );
        println!("Start sending");


        let connection = EspHttpConnection::new(&config)?; //Configuration::default())?;
        let mut client = Client::wrap(connection);

        let content_length = body.len().to_string();

        let headers = [
            ("content-type", "application/json"),
            ("content-length", &content_length),
        ];
    
        let mut request = client.request(Method::Post, url, &headers)?;
        request.write(body.as_bytes()).unwrap();
        let response = request.submit().unwrap();

        if response.status() != 200
        {
            self.print_response(response);
        }

        Ok(())
    }

    pub fn send_message(&self, message: &str) -> Result<()> {
        self.send_discord(message, "")
    }

    pub fn send_embed(&self, embed_json: &str) -> Result<()> {
        self.send_discord("", embed_json)
    }

    fn print_response(&self, mut response: Response<&mut EspHttpConnection>) {
        println!("Response status: {}", response.status());

        // Read and print response body (single read)
        let mut buf = [0u8; 512];
        let read = response.read(&mut buf).unwrap();
        if read > 0 {
            let body = std::str::from_utf8(&buf[..read]).unwrap();
            println!("Body: {}", body);
        }
    }
}