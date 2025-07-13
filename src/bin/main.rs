#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]

mod gong_control;
mod discord;

use gong_control::Gongcontrol;
use discord::Discord;

use esp_hal::{
    clock::CpuClock, delay, gpio::{Level, Output, OutputConfig}, main, time::{Duration, Instant}, timer::timg::TimerGroup
};
use log::info;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[main]
fn main() -> ! {
    // generator version: 0.4.0

    let data_pin = init();
    let mut gong = Gongcontrol::new(37877946, 251, 1, data_pin);

    let delay = delay::Delay::new();

    loop {
        gong.ring();
        info!("ding dong!");
        // let delay_start = Instant::now();
        // while delay_start.elapsed() < Duration::from_millis(500) {}
        
        // let delay_start = Instant::now();
        // while delay_start.elapsed() < Duration::from_millis(5000) {}
        delay.delay_millis(5000);
    }
}

fn init() -> Output<'static> {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 64 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let _init = esp_wifi::init(
        timg0.timer0,
        esp_hal::rng::Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    let led = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());
    led
}