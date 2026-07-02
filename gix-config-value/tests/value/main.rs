type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
fn b(s: &str) -> &bstr::BStr {
    s.into()
}

mod boolean;
mod color;
mod integer;
mod path;
