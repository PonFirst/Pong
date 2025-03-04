#![feature(sync_unsafe_cell)]
#![feature(abi_x86_interrupt)]
#![no_std] // don't link the Rust standard library
#![no_main] // disable all Rust-level entry points

extern crate alloc;

mod allocator;
mod frame_allocator;
mod gdt;
mod interrupts;
mod screen;

use crate::frame_allocator::BootInfoFrameAllocator;
use crate::screen::{Writer, screenwriter};
use alloc::boxed::Box;
use bootloader_api::config::Mapping::Dynamic;
use bootloader_api::info::MemoryRegionKind;
use bootloader_api::{BootInfo, BootloaderConfig, entry_point};
use core::fmt::Write;
use core::slice;
use kernel::{HandlerTable, serial};
use pc_keyboard::DecodedKey;
use pc_keyboard::KeyCode;
use x86_64::VirtAddr;
use x86_64::registers::control::Cr3;

pub static mut PADDLE_LEFT: usize = 100;
pub static mut PADDLE_RIGHT: usize = 100;
pub const PADDLE_WIDTH: usize = 10;
pub const PADDLE_HEIGHT: usize = 60;
pub static mut BALL_X: usize = 200;
pub static mut BALL_Y: usize = 150;
pub const BALL_SIZE: usize = 8;
const BALL_SPEED: usize = 5;

const BOOTLOADER_CONFIG: BootloaderConfig = {
    let mut config = BootloaderConfig::new_default();
    config.mappings.physical_memory = Some(Dynamic); // obtain physical memory offset
    config.kernel_stack_size = 256 * 1024; // 256 KiB kernel stack size
    config
};
entry_point!(kernel_main, config = &BOOTLOADER_CONFIG);

fn kernel_main(boot_info: &'static mut BootInfo) -> ! {
    writeln!(serial(), "Entered kernel with boot info: {boot_info:?}").unwrap();
    writeln!(
        serial(),
        "Frame Buffer: {:p}",
        boot_info.framebuffer.as_ref().unwrap().buffer()
    )
    .unwrap();

    let frame_info = boot_info.framebuffer.as_ref().unwrap().info();
    let framebuffer = boot_info.framebuffer.as_mut().unwrap();
    screen::init(framebuffer);
    for x in 0..frame_info.width {
        screenwriter().draw_pixel(x, frame_info.height - 15, 0xff, 0, 0);
        screenwriter().draw_pixel(x, frame_info.height - 10, 0, 0xff, 0);
        screenwriter().draw_pixel(x, frame_info.height - 5, 0, 0, 0xff);
    }

    for r in boot_info.memory_regions.iter() {
        writeln!(
            serial(),
            "{:?} {:?} {:?} {}",
            r,
            r.start as *mut u8,
            r.end as *mut usize,
            r.end - r.start
        )
        .unwrap();
    }

    let usable_region = boot_info
        .memory_regions
        .iter()
        .filter(|x| x.kind == MemoryRegionKind::Usable)
        .last()
        .unwrap();
    writeln!(serial(), "{usable_region:?}").unwrap();

    let physical_offset = boot_info
        .physical_memory_offset
        .take()
        .expect("Failed to find physical memory offset");
    let ptr = (physical_offset + usable_region.start) as *mut u8;
    writeln!(
        serial(),
        "Physical memory offset: {:X}; usable range: {:p}",
        physical_offset,
        ptr
    )
    .unwrap();

    //read CR3 for current page table
    let cr3 = Cr3::read().0.start_address().as_u64();
    writeln!(serial(), "CR3 read: {:#x}", cr3).unwrap();

    let cr3_page = unsafe { slice::from_raw_parts_mut((cr3 + physical_offset) as *mut usize, 6) };
    writeln!(serial(), "CR3 Page table virtual address {cr3_page:#p}").unwrap();

    allocator::init_heap((physical_offset + usable_region.start) as usize);

    let rsdp = boot_info.rsdp_addr.take();
    let mut mapper = frame_allocator::init(VirtAddr::new(physical_offset));
    let mut frame_allocator = BootInfoFrameAllocator::new(&boot_info.memory_regions);

    gdt::init();

    writeln!(serial(), "Starting kernel...").unwrap();

    let lapic_ptr = interrupts::init_apic(
        rsdp.expect("Failed to get RSDP address") as usize,
        physical_offset,
        &mut mapper,
        &mut frame_allocator,
    );
    HandlerTable::new()
        .keyboard(key)
        .timer(tick)
        .startup(start)
        .start(lapic_ptr)
}

fn start() {
    screenwriter().draw_pong_game();
    screenwriter().draw_mid_line();
}

fn tick() {
    unsafe {
        screenwriter().clear_ball(BALL_X, BALL_Y, BALL_SIZE);
    }
    // Move the ball horizontally
    unsafe {
        BALL_X += BALL_SPEED;

        if BALL_X + BALL_SIZE >= screenwriter().width() || BALL_X <= 0 {
            BALL_X = if BALL_X + BALL_SIZE >= screenwriter().width() {
                screenwriter().width() - BALL_SIZE
            } else {
                0
            };
        }
    }

    // Draw the ball at the new position
    unsafe {
        screenwriter().draw_ball(BALL_X, BALL_Y, BALL_SIZE);
    }

    screenwriter().draw_pong_game();
}

fn key(key: DecodedKey) {
    unsafe {
        match key {
            DecodedKey::Unicode(c) if c == 'W' || c == 'w' => {
                if PADDLE_LEFT > 25 {
                    PADDLE_LEFT -= 25;
                }
            }
            DecodedKey::Unicode(c) if c == 'S' || c == 's' => {
                if PADDLE_LEFT + 80 < screenwriter().height() {
                    PADDLE_LEFT += 25;
                }
            }
            DecodedKey::RawKey(KeyCode::ArrowUp) => {
                if PADDLE_RIGHT > 25 {
                    PADDLE_RIGHT -= 25;
                }
            }
            DecodedKey::RawKey(KeyCode::ArrowDown) => {
                if PADDLE_RIGHT + 80 < screenwriter().height() {
                    PADDLE_RIGHT += 25;
                }
            }
            _ => {}
        }
    }

    screenwriter().draw_pong_game();
}
