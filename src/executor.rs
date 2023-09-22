use core::{
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, Ordering},
    task::RawWakerVTable,
};

use arduino_hal::{
    hal::port::Dynamic,
    port::{self, mode::Output},
};
use core::task::{RawWaker, Waker};
use futures_micro::{Context, Future, Pin, Poll};

#[derive(Debug)]
pub struct SpaceExhausted;

pub struct Executor<'a, const N: usize> {
    busy_pin: port::Pin<Output, Dynamic>,
    task_queue: [MaybeUninit<Option<Task<'a>>>; N],
    task_ready: [MaybeUninit<AtomicBool>; N],
    task_queue_limit: usize,
}

impl<'a, const N: usize> Executor<'a, N> {
    pub fn new(busy_pin: port::Pin<Output, Dynamic>) -> Executor<'a, N> {
        Executor {
            busy_pin,
            task_queue: MaybeUninit::uninit_array(),
            task_ready: MaybeUninit::uninit_array(),
            task_queue_limit: 0usize,
        }
    }

    pub fn submit(&mut self, task: Task<'a>) -> Result<(), SpaceExhausted> {
        // Attempt to pack in an existing slot
        for i in 0..self.task_queue_limit {
            // SAFETY: self.task_queue_limit defines the limit of initialization
            let slot = unsafe { self.task_queue[i].assume_init_mut() };
            if slot.is_none() {
                slot.replace(task);
                // Tasks start ready
                unsafe { self.task_ready[i].assume_init_ref() }.store(true, Ordering::SeqCst);
                return Ok(());
            }
        }
        if self.task_queue_limit < N {
            self.task_queue[self.task_queue_limit].write(Some(task));
            // Tasks start ready
            self.task_ready[self.task_queue_limit].write(AtomicBool::new(true));
            self.task_queue_limit += 1;
            return Ok(());
        }
        Err(SpaceExhausted)
    }

    pub fn run(&mut self) -> ! {
        loop {
            if let Some(idx) = self.next_ready_task() {
                self.busy_pin.set_high();
                // SAFETY: next_ready_task will not return an index out of initialization
                let ready = unsafe { self.task_ready[idx].assume_init_ref() };
                let maybe_task = unsafe { self.task_queue[idx].assume_init_mut() };

                let task = maybe_task
                    .as_mut()
                    // next_ready_task checks for if the task is defined
                    // The only way to clear a task is here in run
                    .unwrap();
                let waker = waker(ready);
                let mut context = Context::from_waker(&waker);
                match task.poll(&mut context) {
                    Poll::Ready(()) => {
                        maybe_task.take();
                    }
                    Poll::Pending => {}
                }
                self.busy_pin.set_low();
            }
        }
    }

    fn next_ready_task(&mut self) -> Option<usize> {
        // Find a task with with ready=true that is actually defined
        avr_device::interrupt::free(|_| {
            for i in 0..self.task_queue_limit {
                // SAFETY task_queue limit is the bounds of init
                let task_ready = unsafe { self.task_ready[i].assume_init_ref() };
                let task_opt = unsafe { self.task_queue[i].assume_init_mut() };
                if task_ready.load(Ordering::SeqCst) && task_opt.is_some() {
                    // Immediately store not ready before we exit critical section
                    task_ready.store(false, Ordering::SeqCst);
                    return Some(i);
                }
            }
            None
        })
    }
}

fn raw_waker<'a>(ready: &AtomicBool) -> RawWaker {
    // Nothing to do on drop
    fn no_op(_: *const ()) {}
    fn wake(data_ptr: *const ()) {
        // let bool_ptr: *const AtomicBool = data_ptr.cast();
        // let bool: &AtomicBool = unsafe { &*bool_ptr };
        // bool.store(true, Ordering::SeqCst);
    }
    fn clone(data_ptr: *const ()) -> RawWaker {
        let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
        RawWaker::new(0 as *const (), vtable)
    }

    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
    RawWaker::new(0 as *const (), vtable)
}

fn waker<'a>(task_slot: &AtomicBool) -> Waker {
    unsafe { Waker::from_raw(raw_waker(task_slot)) }
}

pub struct Task<'a> {
    future: Pin<&'a mut dyn Future<Output = ()>>,
}

impl<'a> Task<'a> {
    pub fn new(future: Pin<&'a mut dyn Future<Output = ()>>) -> Task<'a> {
        Task { future }
    }

    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}
