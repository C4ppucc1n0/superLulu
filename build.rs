// The Rust GNU toolchain's bundled MinGW lacks libimm32.a (needed by miniquad's IME code) and
// has no windres for exe icons, so both are pre-built into mingw-libs/:
//   libimm32.a  - from imm32.def via `rust-lld -flavor link /lib /def:imm32.def /out:libimm32.a /machine:x64`
//   icon.o      - .rsrc object made by tools/make_icon.py (Lulu file icon for Explorer)
fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("mingw-libs");
    println!("cargo:rustc-link-search=native={}", dir.display());
    let icon = dir.join("icon.o");
    if icon.exists() && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
        println!("cargo:rustc-link-arg-bins={}", icon.display());
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=mingw-libs/icon.o");
}
