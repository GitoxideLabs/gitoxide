use unstable_macro_fixture::{Handle, module, plumbing};

fn main() {
    let handle = Handle::new();
    let _ = handle.stable();
    let _ = handle.stable;
    let _ = handle.raw.0;
    let _ = handle.raw();
    let _ = handle.individual();
    let _ = plumbing::raw();
    let _ = module::raw();
    let _ = handle.to_string();
}
