//! Compiles the freestanding C11 shbt-os microkernel sources (SECDED
//! Hamming(72,64) ECC + AVX-512 interlock) into a static library for the
//! Rust FFI surface.  The `build-kernel` CLI command produces the dynamic
//! interface library `bin/shbt_reference.so` separately via the kernel
//! Makefile.

use std::path::PathBuf;

fn main() {
    let kernel_dir = PathBuf::from("../../kernel");
    let include = kernel_dir.join("include");

    let mut build = cc::Build::new();
    for src in [
        kernel_dir.join("src/shbt_ecc_avx512.c"),
        kernel_dir.join("src/shbt_core_runtime.c"),
    ] {
        println!("cargo:rerun-if-changed={}", src.display());
        build.file(&src);
    }
    println!(
        "cargo:rerun-if-changed={}",
        include.join("shbt_hardware.h").display()
    );

    build
        .include(&include)
        .flag("-O3")
        .flag("-ffreestanding")
        .warnings(false);
    if std::env::var("TARGET")
        .unwrap_or_default()
        .contains("x86_64")
    {
        build.flag("-mavx512f");
    }
    build.compile("shbt_ecc_avx512");
}
