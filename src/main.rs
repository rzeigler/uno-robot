#![no_std]
#![no_main]

mod sonic;
mod wheel;

use panic_halt as _;

use arduino_hal::hal::wdt;

#[allow(unused_imports)]
use arduino_hal::prelude::*;
use arduino_hal::simple_pwm::*;

use sonic::{configure_timer, EchoError, EchoLocator, CM};
use wheel::WheelControl;

enum Throttle {
    Stop,
    Slow,
    Fast,
}

fn collision_avoid(readings: &[Result<CM, EchoError>], throttle: Throttle) -> Throttle {
    let mut valid = 0u16;
    let mut sum = 0u16;
    for reading in readings {
        if let Ok(cm) = reading {
            sum += cm.as_u16();
            valid += 1;
        }
    }
    if valid > u16::try_from(readings.len()).unwrap() / 2 {
        let average = sum / valid;
        if average < 15 {
            Throttle::Stop
        } else if average < 40 {
            // Disallow Fast
            match throttle {
                Throttle::Fast => Throttle::Slow,
                t => t,
            }
        } else {
            throttle
        }
    } else {
        // Assume if we arne't getting clear reading from the front then we are fine to continue
        throttle
    }
}

fn decide_throttle(readings: &[Result<CM, EchoError>]) -> Option<Throttle> {
    let mut valid = 0u16;
    let mut sum = 0u16;
    for reading in readings {
        if let Ok(cm) = reading {
            sum += cm.as_u16();
            valid += 1;
        }
    }
    if valid > u16::try_from(readings.len()).unwrap() / 2 {
        let average = sum / valid;
        Some(if average < 50 {
            Throttle::Fast
        } else if average < 120 {
            Throttle::Slow
        } else {
            Throttle::Stop
        })
    } else {
        None
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut signal = pins.d13.into_output();

    let mut timer1 = dp.TC1;
    configure_timer(&mut timer1);

    let mut chase_dist = EchoLocator::new(
        pins.d9.into_output(),
        pins.d10.into_floating_input().forget_imode(),
    );
    let mut bow_dist = EchoLocator::new(
        pins.d11.into_output(),
        pins.d12.into_floating_input().forget_imode(),
    );

    let timer0 = Timer0Pwm::new(dp.TC0, Prescaler::Prescale64);
    let mut right_wheel = WheelControl::new(
        pins.d3.into_output(),
        pins.d4.into_output(),
        pins.d5.into_output().into_pwm(&timer0),
    );
    let mut left_wheel = WheelControl::new(
        pins.d7.into_output(),
        pins.d8.into_output(),
        pins.d6.into_output().into_pwm(&timer0),
    );

    let mut readings: [Result<CM, EchoError>; 5] = [
        // We can initialize to an error state and trust it to be set
        Err(EchoError::NoPulse),
        Err(EchoError::NoPulse),
        Err(EchoError::NoPulse),
        Err(EchoError::NoPulse),
        Err(EchoError::NoPulse),
    ];

    let mut watchdog = wdt::Wdt::new(dp.WDT, &dp.CPU.mcusr);
    watchdog.start(wdt::Timeout::Ms8000).unwrap();

    loop {
        chase_dist.multi_pulse_distance_cm(&mut timer1, &mut readings);
        if let Some(throttle) = decide_throttle(&readings) {
            // We have a throttle, perform a check ahead to ensure that we don't need to halt
            bow_dist.multi_pulse_distance_cm(&mut timer1, &mut readings);
            let throttle = collision_avoid(&readings, throttle);
            signal.set_low();
            let duty = match throttle {
                Throttle::Stop => 0,
                Throttle::Slow => 100,
                Throttle::Fast => 255,
            };
            left_wheel.forward();
            right_wheel.forward();
            left_wheel.set_rotation(duty);
            right_wheel.set_rotation(duty);
        } else {
            signal.set_high();
            left_wheel.halt();
            right_wheel.halt();
        }
        watchdog.feed();
    }
}
