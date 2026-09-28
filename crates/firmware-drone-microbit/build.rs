//! Build script: copy `memory.x` into a place the cortex-m-rt linker script
//! (`link.x`) can find it, and tell rustc where that is.
//!
//! Standard boilerplate for cortex-m-rt projects; see the cortex-m-rt docs.

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR not set"));

    fs::File::create(out.join("memory.x"))
        .expect("failed to create memory.x in OUT_DIR")
        .write_all(include_bytes!("memory.x"))
        .expect("failed to write memory.x");

    let major: u32 = env::var("CARGO_PKG_VERSION_MAJOR")
        .expect("CARGO_PKG_VERSION_MAJOR not set")
        .parse()
        .expect("CARGO_PKG_VERSION_MAJOR not a number");

    let minor: u32 = env::var("CARGO_PKG_VERSION_MINOR")
        .expect("CARGO_PKG_VERSION_MINOR not set")
        .parse()
        .expect("CARGO_PKG_VERSION_MINOR not a number");

    let patch: u32 = env::var("CARGO_PKG_VERSION_PATCH")
        .expect("CARGO_PKG_VERSION_PATCH not set")
        .parse()
        .expect("CARGO_PKG_VERSION_PATCH not a number");

    let version_packed = major << 16 | minor << 8 | patch;

    fs::write(
        out.join("version.rs"),
        format!("pub const FIRMWARE_VERSION: u32 = {version_packed:#010x};\n",),
    )
    .expect("failed to write version.rs");

    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory.x");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
}
