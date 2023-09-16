use arduino_hal::{
    port::{
        mode::{Output, PwmOutput},
        Pin, PinOps,
    },
    simple_pwm::PwmPinOps,
};

pub struct WheelControl<Pin1, Pin2, TC, PinSpeed> {
    in1: Pin<Output, Pin1>,
    in2: Pin<Output, Pin2>,
    en: Pin<PwmOutput<TC>, PinSpeed>,
}

impl<Pin1, Pin2, TC, PinSpeed> WheelControl<Pin1, Pin2, TC, PinSpeed>
where
    Pin1: PinOps,
    Pin2: PinOps,
    PinSpeed: PwmPinOps<TC>,
{
    pub fn new(
        in1: Pin<Output, Pin1>,
        in2: Pin<Output, Pin2>,
        mut en: Pin<PwmOutput<TC>, PinSpeed>,
    ) -> WheelControl<Pin1, Pin2, TC, PinSpeed> {
        // Enable pwm, halt controlled by in1/in2
        en.enable();
        WheelControl {
            in1: in1,
            in2: in2,
            en,
        }
    }

    pub fn halt(&mut self) {
        self.in1.set_low();
        self.in2.set_low();
    }

    // Makes assumptions about how the motors are wired
    pub fn forward(&mut self) {
        self.in1.set_high();
        self.in2.set_low();
    }

    #[allow(dead_code)]
    pub fn reverse(&mut self) {
        self.in1.set_low();
        self.in2.set_high();
    }

    // Note: Anything below duty cycle 50 doens't seem to drive the motors successfully
    pub fn set_rotation(&mut self, rotation: u8) {
        self.en.set_duty(rotation);
    }
}
