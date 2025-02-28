#[global_allocator]
static ALLOCATOR: DummyAllocator = DummyAllocator;

use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr::null_mut;
use core::fmt::Write;

use crate::serial;
pub struct DummyAllocator;

pub static mut HEAP_START: usize = 0x0;
pub static mut OFFSET: usize = 0x0;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB

unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        unsafe {
            // TODO: Implement me!
            null_mut()
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        writeln!(serial(), "dealloc was called at {_ptr:?}").unwrap();
    }
}

pub fn init_heap(offset: usize) {
    unsafe {
        // TODO: Implement me!
    }
}