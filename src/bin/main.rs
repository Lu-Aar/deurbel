#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![allow(static_mut_refs)]
#![feature(impl_trait_in_assoc_type)]

mod discord;
mod global;
mod gong_control;
mod notifications;
mod pin_state;

use discord::Discord;
use global::{KAKU_ADDRESS, WIFI_PASSWORD, WIFI_SSID};
use gong_control::Gongcontrol;
use notifications::NOTIFICATIONS;
use pin_state::PinState;

use embassy_executor::Spawner;
use embassy_net::{DhcpConfig, Runner, Stack, StackResources};
use embassy_time::{Duration, Timer};
use esp_hal::{
    clock::CpuClock,
    gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull},
    peripherals::{RSA, SHA},
    rng::Rng,
    timer::timg::TimerGroup,
};
use esp_wifi::{
    init,
    wifi::{ClientConfiguration, Configuration, WifiController, WifiDevice, WifiEvent},
    EspWifiController,
};
use heapless::String;
use log::info;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }};
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    let (ios, stack, rsa_sha, mut rng) = initialize(spawner).await;
    let (data_pin, mut gong_pin, bell, mute, test_bell) = ios;
    let mut gong = Gongcontrol::new(KAKU_ADDRESS.into(), 251, 1, data_pin);

    let mut discord = Discord::new(stack, rsa_sha);

    let mut bell = PinState::new(bell);
    let mut test_bell = PinState::new(test_bell);
    let mut mute = PinState::new(mute);

    loop {
        if bell.rising_edge() || test_bell.rising_edge() {
            gong_pin.set_high();
            let _ = discord
                .send_message(NOTIFICATIONS[rng.random() as usize % NOTIFICATIONS.len()])
                .await;
            if mute.is_high() {
                gong.ring();
            }
            gong_pin.set_low();
            info!("ding dong!");
            Timer::after(Duration::from_millis(1000)).await;
        } else {
            Timer::after(Duration::from_millis(50)).await;
        }
    }
}

async fn initialize(
    spawner: Spawner,
) -> (
    (
        Output<'static>,
        Output<'static>,
        Input<'static>,
        Input<'static>,
        Input<'static>,
    ),
    Stack<'static>,
    (RSA<'static>, SHA<'static>),
    Rng,
) {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 128 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let mut rng = Rng::new(peripherals.RNG);

    let esp_wifi_ctrl = &*mk_static!(
        EspWifiController<'static>,
        init(timg0.timer0, rng.clone(), peripherals.RADIO_CLK).unwrap()
    );

    let (mut controller, interfaces) =
        esp_wifi::wifi::new(&esp_wifi_ctrl, peripherals.WIFI).unwrap();

    let client_config = Configuration::Client(ClientConfiguration {
        ssid: WIFI_SSID.into(),
        password: WIFI_PASSWORD.into(),
        ..Default::default()
    });
    controller.set_configuration(&client_config).unwrap();
    info!("Starting wifi");

    let wifi_interface = interfaces.sta;

    let timg1 = TimerGroup::new(peripherals.TIMG1);
    esp_hal_embassy::init(timg1.timer0);

    let mut dhcp_config = DhcpConfig::default();
    let host_name: String<32> = String::try_from("deurbel").unwrap();
    dhcp_config.hostname = Some(host_name);

    let config = embassy_net::Config::dhcpv4(dhcp_config);

    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    let (stack, runner) = embassy_net::new(
        wifi_interface,
        config,
        mk_static!(StackResources<3>, StackResources::<3>::new()),
        seed,
    );

    spawner.spawn(connection(controller)).ok();
    spawner.spawn(net_task(runner)).ok();

    loop {
        if stack.is_link_up() {
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    loop {
        if let Some(config) = stack.config_v4() {
            info!("IP address: {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    let data_pin = Output::new(peripherals.GPIO16, Level::Low, OutputConfig::default());
    let gong_pin = Output::new(peripherals.GPIO21, Level::Low, OutputConfig::default());
    let bell = Input::new(peripherals.GPIO17, InputConfig::default());
    let mute = Input::new(
        peripherals.GPIO18,
        InputConfig::default().with_pull(Pull::Up),
    );
    let test_bell = Input::new(
        peripherals.GPIO19,
        InputConfig::default().with_pull(Pull::Up),
    );
    (
        (data_pin, gong_pin, bell, mute, test_bell),
        stack,
        (peripherals.RSA, peripherals.SHA),
        rng,
    )
}

#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, WifiDevice<'static>>) {
    runner.run().await;
}

#[embassy_executor::task]
async fn connection(mut controller: WifiController<'static>) {
    info!("Start connection task");
    info!("Device capabilities: {:?}", controller.capabilities());

    loop {
        if !controller.is_started().unwrap_or(false) {
            controller.start_async().await.unwrap();
            info!("Wifi started!");
        }

        if controller.is_connected().unwrap_or(false) {
            controller.wait_for_event(WifiEvent::StaDisconnected).await;
        } else {
            let _ = controller.connect_async().await;
        }

        Timer::after(Duration::from_secs(5)).await;
    }
}
