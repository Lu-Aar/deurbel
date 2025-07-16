use anyhow::Result;
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration, };
use esp_idf_svc::hal::gpio::*;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::EspWifi;
use heapless::String;

mod gong;
mod discord;
mod global;
mod notifications;

use gong::Gongcontrol;
use discord::Discord;
use notifications::NOTIFICATIONS;
use global::{WIFI_SSID, WIFI_PASSWORD};


fn main() -> Result<()> {
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;
    let mut wifi = EspWifi::new(peripherals.modem, sysloop, Some(nvs))?;

    let my_ssid: String<32> = String::try_from(WIFI_SSID).unwrap();
    let my_password: String<64> = String::try_from(WIFI_PASSWORD).unwrap();
    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: my_ssid,
        password: my_password,
        auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    }))?;

    wifi.start()?;
    wifi.connect()?;

    while !wifi.is_connected().unwrap() {
        let config = wifi.get_configuration().unwrap();
        println!("Waiting for connection: {:?}", config);
    }

    println!("Connected to wifi");

    let mut gong = PinDriver::output(peripherals.pins.gpio2).unwrap();
    let _ = gong.set_low()?;


    let mut gong = Gongcontrol::new(37877946, 251, 1, gong);

    let discord = Discord::new();
    
    loop{
        println!("ding dong!");
        gong.ring();
        let _ = discord.send_message(NOTIFICATIONS[rand::random_range(0..=NOTIFICATIONS.len()-1)]);
        println!("I'm still alive!");
        FreeRtos::delay_ms(5000);
    }
    
}
