use core::str;
use esp_hal::{
    peripherals::RSA,
    peripherals::SHA
};
use esp_mbedtls::Tls;
use crate::global::{
    DISCORD_TTS,
    DISCORD_WEBHOOK_URL
};
use log::{info, error};

use reqwless::{
    client::{HttpClient, TlsConfig},
    headers::ContentType,
    request::{Method, RequestBuilder},
    response::StatusCode
};

use alloc::{format, boxed::Box};

use embassy_net::{
    dns::DnsSocket,
    tcp::client::{TcpClient, TcpClientState},
    Stack};

pub struct Discord {
    client: HttpClient<'static, TcpClient<'static, 1>, DnsSocket<'static>>,
    buffer: [u8; 4096],
}

impl Discord {
    pub fn new(stack: Stack<'static>, rsa_sha_peripheral: (RSA<'static>, SHA<'static>)) -> Self {
        info!("Discord control initialized");

        let (rsa_peripheral, sha_peripheral) = rsa_sha_peripheral;

        let tls = Tls::new(sha_peripheral)
            .unwrap()
            .with_hardware_rsa(rsa_peripheral);
            // .set_debug(5);
        let tls_ref = Box::leak(Box::new(tls));

        let state = TcpClientState::<1, 1024, 1024>::new();
        let state_ref: &'static _ = Box::leak(Box::new(state));
        let tcp = TcpClient::new(stack, state_ref);
        let tcp_ref = Box::leak(Box::new(tcp));
        let dns = DnsSocket::new(stack);
        let dns_ref = Box::leak(Box::new(dns));

        let mut pem = include_bytes!("../certs/discord.pem").to_vec();
        pem.push(0);
        let pem_ref: &'static [u8] = Box::leak(pem.into_boxed_slice());

        let cert = match reqwless::X509::pem(pem_ref) {
            Ok(c) => {
                log::info!("✅ Certificate successfully parsed and loaded");
                Some(c)
            }
            Err(e) => {
                log::error!("❌ Certificate parse failed: {:?}", e);
                None
            }
        };

        let tls_config = TlsConfig::new(
                    reqwless::TlsVersion::Tls1_3,
                    reqwless::Certificates {
                        ca_chain: cert,
                        ..Default::default()
                    },
                    tls_ref.reference(),
                );

        let client = HttpClient::new_with_tls(
            tcp_ref, 
            dns_ref, 
            tls_config,
        );
        
        Discord {
            client,
            buffer: [0u8; 4096],
        }
    }

    pub async fn send_discord(&mut self, content: &str, embed_json: &str) -> Result<(), ()> {

        let json_payload = format!(
            "{{\"content\": \"{}\", \"tts\": {}, \"embeds\": [{}]}}",
            content, DISCORD_TTS, embed_json
        );

        let body = json_payload.as_bytes();

        info!("Sending webhook...");
        let request = match self.client
            .request(Method::POST, DISCORD_WEBHOOK_URL)
            .await {
                Ok(req) => req,
                Err(e) => {
                    error!("Failed to create HTTP request: {:?}", e);
                    return Err(());
                }
            };
        
        info!("Sending content...");
        let mut request = request
            .content_type(ContentType::ApplicationJson)
            .body(body);

        info!("Sending request...");
        let response = match request
            .send(&mut self.buffer)
            .await {
                Ok(res) => res,
                Err(e) => {
                    error!("Failed to send HTTP request: {:?}", e);
                    return Err(());
                }
            };

        if response.status != StatusCode::from(200 as u16) && response.status != StatusCode::from(204)
        {
            error!("{}", 
                core::str::from_utf8(
                    &response.body()
                    .read_to_end()
                    .await
                    .unwrap()
                )
                .unwrap()
            );
        }

        Ok(())
    }

    pub async fn send_message(&mut self, message: &str) -> Result<(), ()> {
        self.send_discord(message, "").await
    }

    pub async fn send_embed(&mut self, embed_json: &str) -> Result<(), ()> {
        self.send_discord("", embed_json).await
    }
}