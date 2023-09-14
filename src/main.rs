#![no_std]
#![no_main]

mod sonic;
mod wheel;

use arduino_hal::port::mode::{Output, PwmOutput};
use arduino_hal::port::{Pin, PinOps};
use panic_halt as _;

use arduino_hal::hal::wdt;

#[allow(unused_imports)]
use arduino_hal::prelude::*;
use arduino_hal::{delay_ms, simple_pwm::*};

use sonic::EchoLocator;
use wheel::WheelControl;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // let locator = EchoLocator::new(
    //     dp.TC1,
    //     pins.d8.into_output(),
    //     pins.d9.into_floating_input().forget_imode(),
    // );

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

    left_wheel.forward();
    left_wheel.set_rotation(60);

    delay_ms(10000);

    left_wheel.set_rotation(40);

    delay_ms(10000);

    left_wheel.set_rotation(20);
    loop {}

    // let enb = pins.d6.into_output().into_pwm(&timer0);

    // let timer0 = Timer0Pwm::new(dp.TC0, Prescaler::Prescale64);
    // let mut green = pins.d5.into_output().into_pwm(&timer0);
    // let mut red = pins.d6.into_output().into_pwm(&timer0);

    // green.enable();
    // green.set_duty(0);

    // red.enable();
    // red.set_duty(0);

    // // Set the watchdog and never clear it so it should reset
    // let mut watchdog = wdt::Wdt::new(dp.WDT, &dp.CPU.mcusr);
    // watchdog.start(wdt::Timeout::Ms8000).unwrap();

    // let mut err_count: u8 = 0;
    // let mut distance_bucket: u8 = 0;
    // for measure in locator {
    //     match measure {
    //         Ok(distance) => {
    //             // Reset the error count
    //             err_count = 0;
    //             // We consider distances in in 10 buckets of 2cm ranges out to 20cm
    //             // Past 20cm we shut we treat as too far
    //             let distance = distance.as_u16().min(20);
    //             // Further distances should reduce the brightness
    //             distance_bucket = 10 - u8::try_from(distance / 2).unwrap();
    //         }
    //         Err(_) => err_count = (err_count + 1).min(4),
    //     }
    //     // Distance status is [1, 10]
    //     if distance_bucket == 0 {
    //         green.disable();
    //     } else {
    //         green.enable();
    //         green.set_duty(distance_bucket * 25);
    //     }
    //     // error status a status of [1,4]
    //     if err_count == 0 {
    //         red.disable();
    //     } else {
    //         red.enable();
    //         red.set_duty(err_count * 63);

    //     }
    //     watchdog.feed()
    // }
    // // locator should never end
    // unreachable!()
}
