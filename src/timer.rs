use core::{cell::RefCell, marker::PhantomData};
use pin_project::pin_project;

use arduino_hal::pac::TC2;
use avr_device::interrupt::Mutex;
use futures_micro::{Context, Future, Pin, Poll, Waker};

/// Marker for a Tick/Delay applying to Timer2
#[derive(Clone, Copy)]
pub struct Timer2 {}

/// An abstraction of the rate at which the timer is able to tick
#[derive(Clone, Copy)]
pub struct Tick<Timer> {
    phantom: PhantomData<Timer>,
}

impl<Timer> Tick<Timer> {
    fn new() -> Tick<Timer> {
        Tick {
            phantom: PhantomData {},
        }
    }
}

pub struct Delay<Timer> {
    count: u32,
    phantom: PhantomData<Timer>,
}

impl<Timer> Delay<Timer> {
    pub fn new(_: Tick<Timer>, count: u32) -> Delay<Timer> {
        Delay {
            count,
            phantom: PhantomData {},
        }
    }
}

struct SleepDrop2 {}

impl Drop for SleepDrop2 {
    fn drop(&mut self) {
        avr_device::interrupt::free(|cs| {
            let cell = TIMER2_INTERRUPT_STATE.borrow(cs);
            // When we are dropped, ensure our waker is cleared
            cell.borrow_mut().waker_opt.take();
        })
    }
}

#[pin_project]
pub struct Sleep2 {
    delay: Delay<Timer2>,
    start_ticks: Option<u32>,
    // Provide drop on
    _drop: SleepDrop2,
}

impl Sleep2 {
    pub fn new(delay: Delay<Timer2>) -> Sleep2 {
        Sleep2 {
            delay,
            start_ticks: None,
            _drop: SleepDrop2 {},
        }
    }
}

impl Future for Sleep2 {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // if let Some(start_ticks) = self.start_ticks {
        // let overflow_ct = avr_device::interrupt::free(|cs| {
        //     let cell = TIMER2_INTERRUPT_STATE.borrow(cs);
        //     cell.borrow().overflow_ct
        // });
        // let ticks = if overflow_ct < start_ticks {
        //     // overflow_ct is incremented with wrapping_add in the interrupt
        //     u32::MAX - (start_ticks - overflow_ct) + 1
        // } else {
        //     overflow_ct - start_ticks
        // };
        // if ticks >= self.delay.count {
        //     Poll::Ready(())
        // } else {
        //     Poll::Pending
        // }
        // } else {
        // We have not yet polled so capture the current overflow count and register our waker
        // avr_device::interrupt::free(|cs| {
        //     let cell = TIMER2_INTERRUPT_STATE.borrow(cs);
        //     let mut state = cell.borrow_mut();
        //     if state.waker_opt.is_some() {
        //         // Cannot have multiple Sleep2 instances running
        //         panic!()
        //     }
        //     state.waker_opt.replace(cx.waker().clone());
        //     self.project().start_ticks.replace(state.overflow_ct);
        // });
        // Poll::Pending
        // }
        Poll::Pending
    }
}

pub struct TimerInterruptState {
    waker_opt: Option<Waker>,
    overflow_ct: u32,
}

pub static TIMER2_INTERRUPT_STATE: Mutex<RefCell<TimerInterruptState>> =
    Mutex::new(RefCell::new(TimerInterruptState {
        waker_opt: None,
        overflow_ct: 0,
    }));

#[avr_device::interrupt(atmega328p)]
fn TIMER2_COMPA() {
    avr_device::interrupt::free(|cs| {
        let cell = TIMER2_INTERRUPT_STATE.borrow(cs);
        let mut state = cell.borrow_mut();
        // Wrapping add and we hope we don't ever try and sleep so long we end up with multiple wraps
        state.overflow_ct = state.overflow_ct.wrapping_add(1);
        if let Some(waker) = state.waker_opt.take() {
            waker.wake();
        }
    });
}

pub fn rig_sleep_timer2(tim2: &TC2, granularity_us: u8) -> Tick<Timer2> {
    // Any more and we will overflow
    if granularity_us > u8::MAX / 2 {
        panic!()
    }
    // At 8 pre-scaling, we have 2 ticks per us
    tim2.ocr2a.write(|w| w.bits(granularity_us * 2));
    tim2.tccr2a.write(|w| w.wgm2().bits(0b10));
    tim2.timsk2.write(|w| w.ocie2a().set_bit());
    tim2.tccr2b.write(|w| w.cs2().prescale_8());

    Tick::new()
}

/// Sleep for a given number of clock ticks defined by the tick length from rig_sleep_timer2
/// Note: Only 1 future of this type may be active at a time
/// Having more than 1 live Future will cause a panic.
/// In particular, a Sleep2 must be dropped before the following Sleep2 can be polled
pub fn sleep_timer2(delay: Delay<Timer2>) -> Sleep2 {
    Sleep2::new(delay)
}
