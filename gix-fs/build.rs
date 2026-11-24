fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-check-cfg=cfg(reexport_stable_types)");
    if env!("CARGO_PKG_VERSION_MAJOR") == "0" {
        println!("cargo:rustc-cfg=reexport_stable_types");
    }
}
