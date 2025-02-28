use ovmf_prebuilt::{Arch, FileType, Prebuilt, Source};

fn main() {
    // read env variables that were set in build script
    let uefi_path = env!("UEFI_PATH");
    println!("Using image: {}", uefi_path);

    let mut cmd = std::process::Command::new("qemu-system-x86_64");

    // This is the last known working version for edk2
    let edk = Source {
        tag: "edk2-stable202211-r1",
        sha256: "b085cfe18fd674bf70a31af1dc3e991bcd25cb882981c6d3523d81260f1e0d12",
    };
    let prebuilt = Prebuilt::fetch(edk, "target/ovmf").expect("failed to fetch prebuilt");
    cmd.arg("-drive").arg(format!("if=pflash,format=raw,unit=0,readonly=on,file={}", prebuilt.get_file(Arch::X64, FileType::Code).display()));
    cmd.arg("-drive").arg(format!("if=pflash,format=raw,unit=1,file={}", prebuilt.get_file(Arch::X64, FileType::Vars).display()));
    
    // set kernel image
    cmd.arg("-drive").arg(format!("format=raw,file={uefi_path}"));
    cmd.arg("-serial").arg("stdio");
    
    // launch qemu and wait until it terminates
    let mut child = cmd.spawn().unwrap();
    child.wait().unwrap();
}