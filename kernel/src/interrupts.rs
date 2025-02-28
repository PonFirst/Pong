use core::fmt::Write;
use core::ptr::NonNull;
use crate::serial;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::{PhysAddr, VirtAddr};
use crate::HandlerTable;
use acpi::{AcpiHandler, AcpiTables, PhysicalMapping};
use pc_keyboard::{layouts, HandleControl, Keyboard, ScancodeSet1};
use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use x86_64::structures::paging::{FrameAllocator, Mapper, PhysFrame, Size4KiB};
use x86_64::instructions::port::Port;
// This code is largely Copyright (c) 2019 Philipp Oppermann.
// Gabriel Ferrer added:
// - HANDLERS variable.
// - Use of HANDLERS in init_idt, timer_interrupt_handler, keyboard_interrupt_handler

lazy_static! {
    pub static ref HANDLERS: Mutex<Option<HandlerTable>> = Mutex::new(None);
}

#[derive(Debug)]
pub struct LAPICAddress {
    address: *mut u32,
}
unsafe impl Send for LAPICAddress {}
unsafe impl Sync for LAPICAddress {}

impl LAPICAddress {
    pub fn new() -> Self {
        Self {
            address: core::ptr::null_mut()
        }
    }
}

lazy_static! {
    pub static ref LAPIC_ADDR: Mutex<LAPICAddress> = Mutex::new(LAPICAddress::new());
}

// https://wiki.osdev.org/APIC
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy)]
#[repr(isize)]
#[allow(dead_code)]
pub enum APICOffset {
    R0x00 = 0x0,      // RESERVED = 0x00
    R0x10 = 0x10,     // RESERVED = 0x10
    Ir = 0x20,        // ID Register
    Vr = 0x30,        // Version Register
    R0x40 = 0x40,     // RESERVED = 0x40
    R0x50 = 0x50,     // RESERVED = 0x50
    R0x60 = 0x60,     // RESERVED = 0x60
    R0x70 = 0x70,     // RESERVED = 0x70
    Tpr = 0x80,       // Text Priority Register
    Apr = 0x90,       // Arbitration Priority Register
    Ppr = 0xA0,       // Processor Priority Register
    Eoi = 0xB0,       // End of Interrupt
    Rrd = 0xC0,       // Remote Read Register
    Ldr = 0xD0,       // Logical Destination Register
    Dfr = 0xE0,       // DFR
    Svr = 0xF0,       // Spurious (Interrupt) Vector Register
    Isr1 = 0x100,     // In-Service Register 1
    Isr2 = 0x110,     // In-Service Register 2
    Isr3 = 0x120,     // In-Service Register 3
    Isr4 = 0x130,     // In-Service Register 4
    Isr5 = 0x140,     // In-Service Register 5
    Isr6 = 0x150,     // In-Service Register 6
    Isr7 = 0x160,     // In-Service Register 7
    Isr8 = 0x170,     // In-Service Register 8
    Tmr1 = 0x180,     // Trigger Mode Register 1
    Tmr2 = 0x190,     // Trigger Mode Register 2
    Tmr3 = 0x1A0,     // Trigger Mode Register 3
    Tmr4 = 0x1B0,     // Trigger Mode Register 4
    Tmr5 = 0x1C0,     // Trigger Mode Register 5
    Tmr6 = 0x1D0,     // Trigger Mode Register 6
    Tmr7 = 0x1E0,     // Trigger Mode Register 7
    Tmr8 = 0x1F0,     // Trigger Mode Register 8
    Irr1 = 0x200,     // Interrupt Request Register 1
    Irr2 = 0x210,     // Interrupt Request Register 2
    Irr3 = 0x220,     // Interrupt Request Register 3
    Irr4 = 0x230,     // Interrupt Request Register 4
    Irr5 = 0x240,     // Interrupt Request Register 5
    Irr6 = 0x250,     // Interrupt Request Register 6
    Irr7 = 0x260,     // Interrupt Request Register 7
    Irr8 = 0x270,     // Interrupt Request Register 8
    Esr = 0x280,      // Error Status Register
    R0x290 = 0x290,   // RESERVED = 0x290
    R0x2A0 = 0x2A0,   // RESERVED = 0x2A0
    R0x2B0 = 0x2B0,   // RESERVED = 0x2B0
    R0x2C0 = 0x2C0,   // RESERVED = 0x2C0
    R0x2D0 = 0x2D0,   // RESERVED = 0x2D0
    R0x2E0 = 0x2E0,   // RESERVED = 0x2E0
    LvtCmci = 0x2F0,  // LVT Corrected Machine Check Interrupt (CMCI) Register
    Icr1 = 0x300,     // Interrupt Command Register 1
    Icr2 = 0x310,     // Interrupt Command Register 2
    LvtT = 0x320,     // LVT Timer Register
    LvtTsr = 0x330,   // LVT Thermal Sensor Register
    LvtPmcr = 0x340,  // LVT Performance Monitoring Counters Register
    LvtLint0 = 0x350, // LVT LINT0 Register
    LvtLint1 = 0x360, // LVT LINT1 Register
    LvtE = 0x370,     // LVT Error Register
    Ticr = 0x380,     // Initial Count Register (for Timer)
    Tccr = 0x390,     // Current Count Register (for Timer)
    R0x3A0 = 0x3A0,   // RESERVED = 0x3A0
    R0x3B0 = 0x3B0,   // RESERVED = 0x3B0
    R0x3C0 = 0x3C0,   // RESERVED = 0x3C0
    R0x3D0 = 0x3D0,   // RESERVED = 0x3D0
    Tdcr = 0x3E0,     // Divide Configuration Register (for Timer)
    R0x3F0 = 0x3F0,   // RESERVED = 0x3F0
}

pub struct AcpiHandlerImpl {
    physical_memory_offset: VirtAddr,
}

impl AcpiHandlerImpl {
    pub fn new(physical_memory_offset: VirtAddr) -> Self {
        Self {
            physical_memory_offset,
        }
    }
}

unsafe impl Send for AcpiHandlerImpl {}
unsafe impl Sync for AcpiHandlerImpl {}

impl Clone for AcpiHandlerImpl {
    fn clone(&self) -> Self {
        Self {
            physical_memory_offset: self.physical_memory_offset,
        }
    }
}

impl AcpiHandler for AcpiHandlerImpl {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        let phys_addr = PhysAddr::new(physical_address as u64);
        let virt_addr = self.physical_memory_offset + phys_addr.as_u64();

        unsafe {
            PhysicalMapping::new(
                physical_address,
                NonNull::new(virt_addr.as_mut_ptr()).expect("Failed to get virtual address"),
                size,
                size,
                self.clone(),
            )
        }
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {
        // No unmapping necessary as we didn't create any new mappings
    }
}

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.double_fault.set_handler_fn(double_fault_handler);

        idt[InterruptIndex::Timer as u8].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard as u8].set_handler_fn(keyboard_interrupt_handler);

        idt
    };

}

unsafe fn init_io_apic(
    ioapic_address: usize,
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    let virt_addr = map_apic(ioapic_address as u64, mapper, frame_allocator);

    let ioapic_pointer = virt_addr.as_mut_ptr::<u32>();

    unsafe {
        ioapic_pointer.offset(0).write_volatile(0x12);
        ioapic_pointer
            .offset(4)
            .write_volatile(InterruptIndex::Keyboard as u8 as u32);
    }
}

unsafe fn init_local_apic(
    local_apic_addr: usize,
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    let virtual_address = map_apic(local_apic_addr as u64, mapper, frame_allocator);

    let lapic_pointer = virtual_address.as_mut_ptr::<u32>();
    LAPIC_ADDR.lock().address = lapic_pointer;
    unsafe {
        init_timer(lapic_pointer);
        init_keyboard(lapic_pointer);
    }
    writeln!(serial(), "init LAPIC_ADDR {:?}", LAPIC_ADDR.lock()).unwrap();
}

unsafe fn init_timer(lapic_pointer: *mut u32) {
    unsafe {
        let svr = lapic_pointer.offset(APICOffset::Svr as isize / 4);
        svr.write_volatile(svr.read_volatile() | 0x100); // Set bit 8

        let lvt_lint1 = lapic_pointer.offset(APICOffset::LvtT as isize / 4);
        lvt_lint1.write_volatile(0x20 | (1 << 17)); // Vector 0x20, periodic mode

        let tdcr = lapic_pointer.offset(APICOffset::Tdcr as isize / 4);
        tdcr.write_volatile(0x3); // Divide by 16 mode

        let ticr = lapic_pointer.offset(APICOffset::Ticr as isize / 4);
        ticr.write_volatile(0x0400_0000); // An arbitrary value for the initial value of the timer
    }
}

unsafe fn init_keyboard(lapic_pointer: *mut u32) {
    unsafe {
        let keyboard_register = lapic_pointer.offset(APICOffset::LvtLint1 as isize / 4);
        keyboard_register.write_volatile(InterruptIndex::Keyboard as u8 as u32);
    }
}

fn map_apic(
    physical_address: u64,
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> VirtAddr {
    use x86_64::structures::paging::Page;
    use x86_64::structures::paging::PageTableFlags as Flags;

    let physical_address = PhysAddr::new(physical_address);
    let page = Page::containing_address(VirtAddr::new(physical_address.as_u64()));
    let frame = PhysFrame::containing_address(physical_address);

    let flags = Flags::PRESENT | Flags::WRITABLE | Flags::NO_CACHE;

    unsafe {
        mapper
            .map_to(page, frame, flags, frame_allocator)
            .expect("APIC mapping failed")
            .flush();
    }

    page.start_address()
}

pub fn init_apic(rsdp: usize, offset: u64, mapper: &mut impl Mapper<Size4KiB>, frame_allocator: &mut impl FrameAllocator<Size4KiB>) -> *mut u32 {
    let handler = AcpiHandlerImpl::new(VirtAddr::new(offset));
    let acpi_tables = unsafe { AcpiTables::from_rsdp(handler, rsdp).expect("Failed to parse ACPI tables") };
    let platform_info = acpi_tables.platform_info().expect("Failed to get platform info");

    match platform_info.interrupt_model {
        acpi::InterruptModel::Apic(apic) => {
            let io_apic_address = apic.io_apics[0].address;
            unsafe { init_io_apic(io_apic_address as usize, mapper, frame_allocator); }

            let local_apic_address = apic.local_apic_address;
            unsafe { init_local_apic(local_apic_address as usize, mapper, frame_allocator); }
        },
        _ => {
            // handler other interrupt models, if necessary
        }
    }

    disable_pic();

    writeln!(serial(), "APIC setup completed, pending interrupt and setup IDT.").unwrap();
    writeln!(serial(), "LAPIC address: {:?}", LAPIC_ADDR.lock()).unwrap();
    LAPIC_ADDR.lock().address
}

fn disable_pic() {
    // Disable any unneeded PIC features, such as timer or keyboard to prevent it from firing interrupts

    unsafe {
        Port::<u8>::new(0x21).write(0xFF);
        Port::<u8>::new(0xA1).write(0xFF);
    }
}

fn end_interrupt() {
    let binding = LAPIC_ADDR.lock();
    unsafe { binding.address.offset(APICOffset::Eoi as isize / 4).write_volatile(0); }
}

/// Initializes the interrupt table with the given interrupt handlers.
pub fn init_idt(handlers: HandlerTable, lapic_pointer: *mut u32) {
    LAPIC_ADDR.lock().address = lapic_pointer;
    writeln!(serial(), "initialize IDT with LAPIC_ADDR {:?}", LAPIC_ADDR.lock()).unwrap();
    *(HANDLERS.lock()) = Some(handlers);

    IDT.load();
    x86_64::instructions::interrupts::enable();
}

extern "x86-interrupt" fn breakpoint_handler(
    stack_frame: InterruptStackFrame)
{
    writeln!(serial(), "EXCEPTION: BREAKPOINT\n{:#?}", stack_frame).unwrap();
}

extern "x86-interrupt" fn page_fault_handler(stack_frame: InterruptStackFrame, error_code: PageFaultErrorCode) {
    panic!("EXCEPTION: PAGE FAULT access address: {:?}\n ErrorCode: {:?}\n{:#?}", Cr2::read(), error_code, stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame, _error_code: u64) -> !
{
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}

const PIC_1_OFFSET: u8 = 0x20;
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    
    let h = &*HANDLERS.lock();
    if let Some(handler) = h {
        handler.handle_timer();
    }

    end_interrupt();
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {

    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> =
            Mutex::new(Keyboard::new(ScancodeSet1::new(), layouts::Us104Key,
                HandleControl::Ignore)
            );
    }

    let mut keyboard = KEYBOARD.lock();
    let mut port = Port::new(0x60);

    let scancode: u8 = unsafe { port.read() };
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            let h = &*HANDLERS.lock();
            if let Some(handler) = h {
                handler.handle_keyboard(key);
            }
        }
    }

    end_interrupt();

}