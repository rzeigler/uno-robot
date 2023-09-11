use arduino_hal::pac::TC1;
use arduino_hal::port::mode::{Input, Floating};

use arduino_hal::prelude::*;
use arduino_hal::{hal::wdt, port::{Pin, mode::Output, PinOps}};


pub enum EchoError {
    // The echo pin never lights up
    NoPulse,
    // Nothing was detected
    DistanceOverflow
}

pub struct CM(u16);

impl CM {
    pub fn as_u16(self) -> u16 {
        self.0
    }
}

pub struct EchoLocator<TrigPin: PinOps, EchoPin: PinOps> {
    timer1: TC1,
    trig: Pin<Output, TrigPin>,
    echo: Pin<Input, EchoPin>
}

impl <TrigPin: PinOps, EchoPin: PinOps> EchoLocator<TrigPin, EchoPin> {
    // Creates the locator, also sets up the expected scale factor
    pub fn new(timer: TC1, trig: Pin<Output, TrigPin>, echo: Pin<Input, EchoPin>) -> EchoLocator<TrigPin, EchoPin> {
        // Starting and initializing the timer with prescaling 64.
        // it gives one clock count every 4 µs.
        // since the clock register size is 16 bits, the timer is full every
        // 1/(16e6/64)*2^16 ≈ 260 ms
        timer.tccr1b.write(|w| w.cs1().prescale_64());
        EchoLocator { timer1: timer, trig: trig, echo: echo }
    }

    pub fn pulse_distance_cm(&mut self) -> Result<CM, EchoError> {
        // Start by resetting the timer
        self.timer1.tcnt1.write(|w| w.bits(0));

        // Set the timer to high for 10 us
        self.trig.set_high();
        arduino_hal::delay_us(10);
        self.trig.set_low();

        // Wait for the echo pulse to go high, if this doesn't happen its an error
        // 0.2s/4us = 50000
        while self.echo.is_low() {
            if self.timer1.tcnt1.read().bits() >= 50000 {
                return Err(EchoError::NoPulse)
            }
        }

        self.timer1.tcnt1.write(|w| w.bits(0));

        while self.echo.is_high() {}

         // 1 count == 4 µs, so the value is multiplied by 4.
        // 1/58 ≈ (34000 cm/s) * 1µs / 2
        // when no object is detected, instead of keeping the echo pin completely low,
        // some HC-SR04 labeled sensor holds the echo pin in high state for very long time,
        // thus overflowing the u16 value when multiplying the timer1 value with 4.
        // overflow during runtime causes panic! so it must be handled
        let result = self.timer1.tcnt1.read().bits().saturating_mul(4);
        if result == u16::MAX {
            return Err(EchoError::DistanceOverflow)
        }
        Ok(CM(result / 58))
    }
}

impl <TrigPin: PinOps, EchoPin: PinOps> IntoIterator for EchoLocator<TrigPin, EchoPin> {
    type Item = Result<CM, EchoError>;
    type IntoIter = DistanceIter<TrigPin, EchoPin>;

    fn into_iter(self) -> Self::IntoIter {
        DistanceIter::new(self)
    }
}

pub struct DistanceIter<TrigPin: PinOps, EchoPin: PinOps> {
    echo: EchoLocator<TrigPin, EchoPin>
}

impl <TrigPin: PinOps, EchoPin: PinOps> DistanceIter<TrigPin, EchoPin> {
    fn new(echo: EchoLocator<TrigPin, EchoPin>) -> DistanceIter<TrigPin, EchoPin> {
        DistanceIter { echo }
    }
}

impl <TrigPin: PinOps, EchoPin: PinOps> Iterator for DistanceIter<TrigPin, EchoPin> {
    type Item = Result<CM, EchoError>;

    fn next(&mut self) -> Option<Self::Item> {
        // First, sleep for 50us to make sure that we hit the maximum pulse frequency
        arduino_hal::delay_us(50);
        Some(self.echo.pulse_distance_cm())
    }
}