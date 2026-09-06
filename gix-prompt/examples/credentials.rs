fn main() -> Result<(), gix_error::Exn<gix_error::Message>> {
    let user = gix_prompt::openly("Username: ")?;
    eprintln!("{user:?}");
    let pass = gix_prompt::securely("Password: ")?;
    eprintln!("{pass:?}");
    Ok(())
}
