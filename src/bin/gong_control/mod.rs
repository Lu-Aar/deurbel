use esp_hal::{
    delay::{Delay},
    gpio::{self},
};

use core::iter::Iterator;
use log::info;

pub struct Gongcontrol {
    address: u32,
    period: u8,
    switch_type: u8,
    pin: gpio::Output<'static>,
    delay: Delay,
}

impl Gongcontrol {
    pub fn new(address: u32, period: u8, switch_type: u8, pin: gpio::Output<'static>,) -> Self {
        info!{
            "Gongcontrol initialized with address: {}, period: {}, switch_type: {}",
            address, period, switch_type
        }
        Self {
            address,
            period,
            switch_type,
            pin,
            delay: Delay::new(),
        }
    }

    pub fn set_address(&mut self, address: u32) {
        self.address = address;
    }

    pub fn set_period(&mut self, period: u8) {
        self.period = period;
    }

    pub fn set_switch_type(&mut self, switch_type: u8) {
        self.switch_type = switch_type;
    }

    pub fn ring(&mut self) {
        for _ in 0..16{
            self.send_start_pulse();
            self.send_address();
            self.send_bit(true);
            self.send_bit(true);
            self.send_unit(0);
            self.send_stop_pulse();
        }
    }

    fn send_start_pulse(&mut self) {
        self.pin.set_high();
        self.delay.delay_micros(self.period as u32);
        self.pin.set_low();
        self.delay.delay_micros(self.period as u32 * 10 + (self.period as u32 >> 1));
    }

    fn send_address(&mut self) {
        for i in (0..= 25).rev() {
            self.send_bit(((self.address >> i) & 1) != 0);
        }
    }

    fn send_bit(&mut self, bit: bool) {
        if bit {
            self.pin.set_high();
            self.delay.delay_micros(self.period as u32);
            self.pin.set_low();
            self.delay.delay_micros(self.period as u32 * 5);
            self.pin.set_high();
            self.delay.delay_micros(self.period as u32);
            self.pin.set_low();
            self.delay.delay_micros(self.period as u32);
        } else {
            self.pin.set_high();
            self.delay.delay_micros(self.period as u32);
            self.pin.set_low();
            self.delay.delay_micros(self.period as u32);
            self.pin.set_high();
            self.delay.delay_micros(self.period as u32);
            self.pin.set_low();
            self.delay.delay_micros(self.period as u32 * 5);
        }
    }

    fn send_unit(&mut self, unit: u8) {
        for i in (0..=3).rev() {
            self.send_bit(((unit >> i) & 1) != 0);
        }
    }

    fn send_stop_pulse(&mut self) {
        self.pin.set_high();
        self.delay.delay_micros(self.period as u32);
        self.pin.set_low();
        self.delay.delay_micros(self.period as u32 * 40);
    }
}