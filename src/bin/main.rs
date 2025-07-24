#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![allow(static_mut_refs)]
#![feature(impl_trait_in_assoc_type)]

mod gong_control;
mod discord;
mod global;

use esp_wifi::{
    init,
    wifi::{
        WifiDevice,
        WifiEvent
    },
    EspWifiController
};
use gong_control::Gongcontrol;
use discord::Discord;
use global::{
    WIFI_SSID,
    WIFI_PASSWORD,
};

use esp_hal::{
    clock::CpuClock,
    delay,
    gpio::{
        Input,
        InputConfig,
        Pull,
        Level,
        Output,
        OutputConfig
    },
    peripherals::{RSA, SHA},
    rng::Rng,
    timer::timg::TimerGroup
};
use log::info;
use esp_wifi::wifi::{
    WifiController,
    ClientConfiguration,
    Configuration,
    WifiState
};
use embassy_net::{
    DhcpConfig,
    StackResources,
    Runner,
    Stack
};
use embassy_time::{
    Timer,
    Duration
};
use embassy_executor::Spawner;
use heapless::String;


#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

mod notifications;
use notifications::NOTIFICATIONS;

macro_rules! mk_static {
    ($t:ty,$val:expr) => {{
        static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
        #[deny(unused_attributes)]
        let x = STATIC_CELL.uninit().write(($val));
        x
    }}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    let (ios, stack, rsa_sha, mut rng) = initialize(spawner).await;
    let (data_pin, bell, mute, test_bell) = ios;
    let mut gong = Gongcontrol::new(37877946, 251, 1, data_pin);

    let delay = delay::Delay::new();
    let mut discord = Discord::new(stack, rsa_sha);

    loop {
        if bell.is_low() || test_bell.is_low()
        {
            let _ = discord.send_message(NOTIFICATIONS[rng.random() as usize % NOTIFICATIONS.len()]).await;
            if mute.is_high()
            {
                gong.ring();
            }
            info!("ding dong!");
            delay.delay_millis(1000);
        }
        else {
            delay.delay_millis(50);
        }
    }
}

async fn initialize(spawner: Spawner) -> ((Output<'static>, Input<'static>, Input<'static>, Input<'static>), Stack<'static>, (RSA<'static>, SHA<'static>), Rng) {
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

    let (controller, interfaces) = esp_wifi::wifi::new(&esp_wifi_ctrl, peripherals.WIFI).unwrap();

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
        seed
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

    let data_pin = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());
    let bell = Input::new(peripherals.GPIO4, InputConfig::default().with_pull(Pull::Up));
    let mute = Input::new(peripherals.GPIO16, InputConfig::default().with_pull(Pull::Up));
    let test_bell = Input::new(peripherals.GPIO17, InputConfig::default().with_pull(Pull::Up));
    ((data_pin, bell, mute, test_bell), stack, (peripherals.RSA, peripherals.SHA), rng)
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
        match esp_wifi::wifi::wifi_state() {
            WifiState::StaConnected => {
                controller.wait_for_event(WifiEvent::StaConnected).await;
                Timer::after(Duration::from_millis(5000)).await;
            }
            _ =>{}
        }

        if !matches!(controller.is_started(), Ok(true)) {
            let client_config = Configuration::Client(ClientConfiguration {
                ssid: WIFI_SSID.into(),
                password: WIFI_PASSWORD.into(),
                ..Default::default()
            });
            controller.set_configuration(&client_config).unwrap();
            info!("Starting wifi");
            controller.start_async().await.unwrap();
            info!("Wifi started!");

            info!("Scanning for networks...");
            let result = controller.scan_n_async(10).await.unwrap();
            for ap in result {
                info!("{:?}", ap);
            }
        }
        info!("About to connect to WiFi");

        match controller.connect_async().await {
            Ok(_) => info!("Connected to WiFi"),
            Err(e) => {
                info!("Failed to connect to wifi: {e:?}");
                Timer::after(Duration::from_millis(5000)).await;
            }
        }
    }
}