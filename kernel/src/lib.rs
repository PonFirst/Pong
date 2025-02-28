// Original code from rust-osdev/bootloader crate https://github.com/rust-osdev/bootloader
#![no_std]
#![feature(abi_x86_interrupt)]

use core::cell::UnsafeCell;
use core::panic::PanicInfo;
use core::fmt::Write;
use uart_16550::SerialPort;
use pc_keyboard::DecodedKey;

mod interrupts;

extern crate alloc;

pub fn serial() -> SerialPort {
    let mut port = unsafe { SerialPort::new(0x3F8) };
    port.init();
    port
}

/// Table of interrupt handlers. This struct uses the
/// [Builder pattern](https://doc.rust-lang.org/1.0.0/style/ownership/builders.html).
/// Start by calling new() to create a new Handler table. Then use the appropriate methods to set
/// up the handlers. When ready, call the **.start()** method to start up your pluggable
/// interrupt operating system.
///
/// For now, it only includes timer and keyboard handlers.
pub struct HandlerTable {
    timer: Option<fn()>,
    keyboard: Option<fn(DecodedKey)>,
    startup: Option<fn()>,
    cpu_loop: fn() -> !,
}

impl HandlerTable {
    /// Creates a new HandlerTable with no handlers.
    pub fn new() -> Self {
        HandlerTable {timer: None, keyboard: None, startup: None, cpu_loop: hlt_loop}
    }

    /// Starts up a simple operating system using the specified handlers.
    pub fn start(self, lapic_ptr: *mut u32) -> ! {
        self.startup.map(|f| f());
        let fore = self.cpu_loop;
        
        interrupts::init_idt(self, lapic_ptr);
        
        (fore)();
    }

    /// Sets the timer handler.
    /// Returns Self for chained [Builder pattern construction](https://doc.rust-lang.org/1.0.0/style/ownership/builders.html).
    pub fn timer(mut self, timer_handler: fn()) -> Self {
        self.timer = Some(timer_handler);
        self
    }

    /// Called by the low-level interrupt routines to handle a timer event.
    pub fn handle_timer(&self) {
        if let Some(timer) = self.timer {
            (timer)()
        }
    }

    /// Sets the keyboard handler. The [DecodedKey](https://docs.rs/pc-keyboard/0.5.1/pc_keyboard/enum.DecodedKey.html)
    /// enum comes from the [pc_keyboard](https://crates.io/crates/pc-keyboard) crate.
    ///
    /// Returns Self for chained [Builder pattern construction](https://doc.rust-lang.org/1.0.0/style/ownership/builders.html).
    pub fn keyboard(mut self, keyboard_handler: fn(DecodedKey)) -> Self {
        self.keyboard = Some(keyboard_handler);
        self
    }

    /// Called by the low-level interrupt routines to handle a keyboard event.
    pub fn handle_keyboard(&self, key: DecodedKey) {
        if let Some(keyboard) = self.keyboard {
            (keyboard)(key)
        }
    }

    /// Sets the startup handler.
    /// Returns Self for chained [Builder pattern construction](https://doc.rust-lang.org/1.0.0/style/ownership/builders.html).
    pub fn startup(mut self, startup_handler: fn()) -> Self {
        self.startup = Some(startup_handler);
        self
    }

    /// Sets the cpu loop handler.
    /// This function should contain an infinite loop.
    /// Returns Self for chained [Builder pattern construction](https://doc.rust-lang.org/1.0.0/style/ownership/builders.html).
    pub fn cpu_loop(mut self, cpu_loop: fn() -> !) -> Self {
        self.cpu_loop = cpu_loop;
        self
    }
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let _ = writeln!(serial(), "PANIC: {info}");
    hlt_loop();
}

pub struct RacyCell<T>(UnsafeCell<T>);

impl<T> RacyCell<T> {
    pub const fn new(v: T) -> Self {
        Self(UnsafeCell::new(v))
    }

    /// Gets a mutable pointer to the wrapped value.
    ///
    /// ## Safety
    /// Ensure that the access is unique (no active references, mutable or not).
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }
}

unsafe impl<T> Send for RacyCell<T> where T: Send {}
unsafe impl<T: Sync> Sync for RacyCell<T> {}