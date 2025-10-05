use esp_hal::gpio::Input;

pub struct PinState {
    pin: Input<'static>,
    previous_state: bool,
}

impl PinState {
    pub fn new(pin: Input<'static>) -> Self {
        Self {
            pin,
            previous_state: false,
        }
    }

    pub fn rising_edge(&mut self) -> bool {
        let current_state = self.pin.is_high();
        let edge_detected = !self.previous_state && current_state;
        self.previous_state = current_state;
        edge_detected
    }

    pub fn falling_edge(&mut self) -> bool {
        let current_state = self.pin.is_high();
        let edge_detected = self.previous_state && !current_state;
        self.previous_state = current_state;
        edge_detected
    }

    pub fn is_high(&mut self) -> bool {
        let is_high = self.pin.is_high();
        if is_high {
            self.previous_state = true;
        }
        is_high
    }

    pub fn is_low(&mut self) -> bool {
        let is_low = self.pin.is_low();
        if is_low {
            self.previous_state = false;
        }
        is_low
    }
    
}