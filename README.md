# Bare Metal Starter

This is a starting implementation for a simple bare metal kernel for x86-64 architecture. The implementation is based on [rust-osdev/bootloader](https://github.com/rust-osdev/bootloader) crate,
the [pluggable interrupt OS](https://github.com/gjf2a/pluggable_interrupt_os), and [ArceOS](https://github.com/arceos-org/arceos).
This version has been modified to support UEFI (with OVMF image) and APIC.

## Requirements

You need a [QEMU](https://www.qemu.org/) machine emulator installed with accessible path to `qemu-system-x86_64`.
You also need nightly [Rust](https://www.rust-lang.org) compiler. But the project has been configured so `cargo run` should pull in dependencies,
building boot image and launching QEMU.

## Usage

To use this crate, you need to adjust your kernel to be bootable first. Then you can create a bootable disk image from your compiled kernel. These steps are explained in detail below.

If you're already using an older version of the `bootloader` crate, follow our [migration guides](docs/migration).

### Kernel

Your actual kernel implementation is in `kernel` directory.
- `main.rs` contains the entry point to the kernel.
- `lib.rs` contains the utility functions and implementation of the kernel `HandlerTable` containing the implementation of the main event loop.
- `interrupts.rs` contains initialization methods and interaction with [APIC (Advanced Programmable Interrupt Controller)](https://wiki.osdev.org/APIC) to set up interrupt behavior and [IDT](https://wiki.osdev.org/Interrupt_Descriptor_Table). The local APIC registers are memory-mapped to a physical frame.
- `allocator.rs` contains a placeholder implementation for the global memory allocator (which you must implement)
- `screen.rs` contains utility functions used to interact with the graphical framebuffer.
- `gdt.rs` contains the code to set up the [GDT (Global Descriptor Table)](https://wiki.osdev.org/GDT_Tutorial); originally used for memory segmentation, but mostly unused for 64-bit mode.
- `frame_allocator.rs` contains utility functions used to map the physical frame for APIC.
- Thanks to the `entry_point` macro, the compiled executable contains a special section with metadata and the serialized config, which will enable the `bootloader` crate to load it.

### Booting

The current `build.rs` will create the boot disk image based on your kernel implementation while the `src/main.rs` maintains
the launch configuration of the virtual machine with working OVMF image.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.