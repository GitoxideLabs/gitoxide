use unstable_macro_fixture::{Handle, Raw, module, module_without_reason, plumbing};

fn main() {
    let handle = Handle::new();
    let _ = handle.stable();
    let _ = handle.stable;
    let _ = handle.raw.0;
    let _ = handle.raw_without_reason.0;
    let _ = handle.raw_without_reason_with_parens.0;
    let _ = Raw(42);
    let _ = handle.raw();
    let _ = handle.individual();
    let _ = handle.without_reason();
    let _ = Handle::WITHOUT_REASON;
    let _ = plumbing::raw();
    let _ = module::raw();
    let _ = module_without_reason::raw();
    let _ = handle.to_string();
}
