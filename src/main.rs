#![no_std]
#![no_main]
#![feature(maybe_uninit_uninit_array)]
#![feature(abi_avr_interrupt)]

mod executor;
mod sonic;
mod timer;
mod wheel;

use core::pin::pin;

use arduino_hal::{
    default_serial,
    hal::port::Dynamic,
    port::{mode::Output, Pin},
};

#[allow(unused_imports)]
use arduino_hal::prelude::*;

use executor::{Executor, Task};

use timer::{rig_sleep_timer2, sleep_timer2, Delay, Tick, Timer2};

#[cfg(not(doc))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // disable interrupts - firmware has panicked so no ISRs should continue running
    avr_device::interrupt::disable();

    // get the peripherals so we can access serial and the LED.
    //
    // SAFETY: Because main() already has references to the peripherals this is an unsafe
    // operation - but because no other code can run after the panic handler was called,
    // we know it is okay.
    let dp = unsafe { arduino_hal::Peripherals::steal() };
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    // Print out panic location
    ufmt::uwriteln!(&mut serial, "Firmware panic!\r").void_unwrap();
    if let Some(loc) = info.location() {
        ufmt::uwriteln!(
            &mut serial,
            "  At {}:{}:{}\r",
            loc.file(),
            loc.line(),
            loc.column(),
        )
        .void_unwrap();
    }

    // Blink LED rapidly
    let mut led = pins.d13.into_output();
    loop {
        led.toggle();
        arduino_hal::delay_ms(100);
    }
}

async fn blinker(mut pin: Pin<Output, Dynamic>, tick: Tick<Timer2>) {
    loop {
        pin.set_high();
        sleep_timer2(Delay::new(tick, 5000)).await;
        pin.set_low();
        sleep_timer2(Delay::new(tick, 5000)).await;
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = default_serial!(dp, pins, 57600);

    let busy = pins.d13.into_output().downgrade();
    let blink = pins.d12.into_output().downgrade();

    let tick2 = rig_sleep_timer2(&dp.TC2, 10);

    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    unsafe {
        // SAFETY: Not inside a critical section and any non-atomic operations have been completed
        // at this point.
        avr_device::interrupt::enable();
    }

    let fut = pin!(blinker(blink, tick2));

    let mut executor = Executor::<5>::new(busy);
    executor.submit(Task::new(fut)).unwrap();
    serial.write_str("Starting up\n\r").unwrap();
    executor.run();
}
