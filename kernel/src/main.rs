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
use core::sync::atomic::{AtomicI32, Ordering};
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
pub static mut BALL_SPEED_X: isize = 5;
pub static mut BALL_SPEED_Y: isize = 3;
static LEFT_SCORE: AtomicI32 = AtomicI32::new(0);
static RIGHT_SCORE: AtomicI32 = AtomicI32::new(0);
static GAME_STATE: AtomicI32 = AtomicI32::new(0); // 0: ongoing, 1: ended

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

fn draw_score(score: i32, x: usize, y: usize, size: usize) {
    match score {
        0 => screenwriter().draw_zero(x, y, size),
        1 => screenwriter().draw_one(x, y, size),
        2 => screenwriter().draw_two(x, y, size),
        3 => screenwriter().draw_three(x, y, size),
        _ => {}
    }
}

fn start() {
    screenwriter().draw_pong_game();
    screenwriter().draw_mid_line();
    draw_score(0, screenwriter().width() / 4, 10, 30); // Left player
    draw_score(0, 3 * screenwriter().width() / 4, 10, 30); // Right player
}

fn tick() {
    unsafe {
        if GAME_STATE.load(Ordering::Relaxed) == 1 {
            // Game has ended, display win message
            let message = if LEFT_SCORE.load(Ordering::Relaxed) >= 3 {
                "Left Player Wins! Press 'r' to restart"
            } else {
                "Right Player Wins! Press 'r' to restart"
            };
            let char_width = 8;
            let text_width = message.len() * char_width;
            let start_x = (screenwriter().width() / 2) - (text_width / 2); // Center horizontally
            let start_y = screenwriter().height() / 2; // Center vertically
            screenwriter().set_position(start_x, start_y);
            write!(screenwriter(), "{}", message).unwrap();
            return;
        }

        // Clear the ball's old position
        screenwriter().clear_ball(BALL_X, BALL_Y, BALL_SIZE);

        // Calculate new ball position
        let new_ball_x = (BALL_X as isize) + BALL_SPEED_X;
        let new_ball_y = (BALL_Y as isize) + BALL_SPEED_Y;

        // Check for scoring conditions
        if new_ball_x < 0 {
            // Right player scores
            let right_score = RIGHT_SCORE.fetch_add(1, Ordering::Relaxed) + 1; // New score
            let score_x = 3 * screenwriter().width() / 4;
            let score_y = 10;
            let size = 30;

            screenwriter().clear_score(score_x, score_y, size);
            draw_score(right_score, score_x, score_y, size);
            BALL_X = screenwriter().width() / 2;
            BALL_Y = screenwriter().height() / 2;
            BALL_SPEED_X = 5;
            BALL_SPEED_Y = 3;
            if right_score >= 3 {
                GAME_STATE.store(1, Ordering::Relaxed);
            }
        } else if new_ball_x + BALL_SIZE as isize > screenwriter().width() as isize {
            // Left player scores
            let left_score = LEFT_SCORE.fetch_add(1, Ordering::Relaxed) + 1; // New score
            let score_x = screenwriter().width() / 4;
            let score_y = 10;
            let size = 30;

            screenwriter().clear_score(score_x, score_y, size);
            draw_score(left_score, score_x, score_y, size);
            BALL_X = screenwriter().width() / 2;
            BALL_Y = screenwriter().height() / 2;
            BALL_SPEED_X = -5;
            BALL_SPEED_Y = -3;
            if left_score >= 3 {
                GAME_STATE.store(1, Ordering::Relaxed);
            }
        } else {
            BALL_X = new_ball_x as usize;

            if new_ball_y < 0 {
                BALL_Y = 0;
                BALL_SPEED_Y = -BALL_SPEED_Y; // Bounce downward
            } else if new_ball_y + BALL_SIZE as isize > screenwriter().height() as isize {
                BALL_Y = (screenwriter().height() - BALL_SIZE) as usize; // Clamp to bottom
                BALL_SPEED_Y = -BALL_SPEED_Y; // Bounce upward
            } else {
                BALL_Y = new_ball_y as usize; // Normal movement within bounds
            }

            // Right paddle collision
            if BALL_SPEED_X > 0
                && new_ball_x + (BALL_SIZE + 15) as isize
                    >= screenwriter().width() as isize - PADDLE_WIDTH as isize
                && new_ball_y + BALL_SIZE as isize > PADDLE_RIGHT as isize
                && new_ball_y < (PADDLE_RIGHT + PADDLE_HEIGHT) as isize
            {
                BALL_SPEED_X = -BALL_SPEED_X; // Bounce left
            }
            // Left paddle collision
            else if BALL_SPEED_X < 0
                && new_ball_x <= (PADDLE_WIDTH + 15) as isize
                && new_ball_y + BALL_SIZE as isize > PADDLE_LEFT as isize
                && new_ball_y < (PADDLE_LEFT + PADDLE_HEIGHT) as isize
            {
                BALL_SPEED_X = -BALL_SPEED_X; // Bounce right
            }
        }

        // Draw the ball at the new position
        screenwriter().draw_ball(BALL_X, BALL_Y, BALL_SIZE);
        screenwriter().draw_pong_game();

        // Redraw Mid Line
        screenwriter().draw_mid_line();
    }
}

fn key(key: DecodedKey) {
    unsafe {
        if GAME_STATE.load(Ordering::Relaxed) == 1 {
            if let DecodedKey::Unicode('r') = key {
                // Reset game state
                LEFT_SCORE.store(0, Ordering::Relaxed);
                RIGHT_SCORE.store(0, Ordering::Relaxed);
                BALL_X = screenwriter().width() / 2;
                BALL_Y = screenwriter().height() / 2;
                BALL_SPEED_X = 5;
                BALL_SPEED_Y = 3;
                PADDLE_LEFT = 100;
                PADDLE_RIGHT = 500;
                GAME_STATE.store(0, Ordering::Relaxed);
                screenwriter().clear();
                start();
            }
            return;
        }

        match key {
            DecodedKey::Unicode(c) if c == 'W' || c == 'w' => {
                if PADDLE_LEFT > 25 {
                    PADDLE_LEFT -= 25;
                }
            }
            DecodedKey::Unicode(c) if c == 'S' || c == 's' => {
                if PADDLE_LEFT + 75 < screenwriter().height() {
                    PADDLE_LEFT += 25;
                }
            }
            DecodedKey::RawKey(KeyCode::ArrowUp) => {
                if PADDLE_RIGHT > 25 {
                    PADDLE_RIGHT -= 25;
                }
            }
            DecodedKey::RawKey(KeyCode::ArrowDown) => {
                if PADDLE_RIGHT + 75 < screenwriter().height() {
                    PADDLE_RIGHT += 25;
                }
            }
            _ => {}
        }
    }

    screenwriter().draw_pong_game();
}
