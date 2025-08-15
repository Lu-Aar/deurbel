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
mod notifications;
mod pin_state;

use gong_control::Gongcontrol;
use discord::Discord;
use global::{
    WIFI_SSID,
    WIFI_PASSWORD,
};
use notifications::NOTIFICATIONS;
use pin_state::PinState;

use log::info;
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
use esp_wifi::{
    init,
    wifi::{
        WifiController,
        ClientConfiguration,
        Configuration,
        WifiState,
        WifiDevice,
        WifiEvent
    },
    EspWifiController
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
    let (data_pin, bell, mute, test_bell, mut physical_gong) = ios;
    let mut gong = Gongcontrol::new(37877946, 251, data_pin);

    let delay = delay::Delay::new();
    let mut discord = Discord::new(stack, rsa_sha);

    let mut bell = PinState::new(bell);
    let mut test_bell = PinState::new(test_bell);
    let mut mute = PinState::new(mute);

    loop {
        if bell.falling_edge() || test_bell.falling_edge()
        {
            let _ = discord.send_message(NOTIFICATIONS[rng.random() as usize % NOTIFICATIONS.len()]).await;
            if mute.is_high()
            {
                physical_gong.set_high();
                gong.ring();
                physical_gong.set_low();
            }
            info!("ding dong!");
            delay.delay_millis(1000);
        }
        else {
            delay.delay_millis(50);
        }
    }
}

async fn initialize(spawner: Spawner) -> ((Output<'static>, Input<'static>, Input<'static>, Input<'static>, Output<'static>),
                                            Stack<'static>,
                                            (RSA<'static>, SHA<'static>),
                                            Rng) {
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

    let gpio16_out = Output::new(peripherals.GPIO16, Level::Low, OutputConfig::default());
    let gpio17_in = Input::new(peripherals.GPIO17, InputConfig::default()); 
    let gpio18_in = Input::new(peripherals.GPIO18, InputConfig::default());
    let gpio19_in = Input::new(peripherals.GPIO19, InputConfig::default());
    let gpio21_out = Output::new(peripherals.GPIO21, Level::Low, OutputConfig::default());

    ((gpio16_out, gpio17_in, gpio18_in, gpio19_in, gpio21_out), stack, (peripherals.RSA, peripherals.SHA), rng)
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