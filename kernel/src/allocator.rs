#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator;

use alloc::alloc::{GlobalAlloc, Layout};
use core::fmt::Write;

use crate::serial;

pub static mut HEAP_START: usize = 0x0;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

pub struct BumpAllocator;

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let bump_ptr = HEAP_START;
        let new_heap_start = HEAP_START + layout.size();

        if new_heap_start > HEAP_START + HEAP_SIZE {
            panic!("Out of memory!");
        }

        HEAP_START = new_heap_start;
        bump_ptr as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        writeln!(serial(), "dealloc was called at {_ptr:?}").unwrap();
    }
}

pub fn init_heap(offset: usize) {
    unsafe {
        HEAP_START = offset;
    }
}
