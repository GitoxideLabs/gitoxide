fn main() -> Result<(), Box<dyn std::error::Error>> {
    let prompt = std::env::args()
        .nth(1)
        .ok_or("First argument must be the prompt to display when asking for a password")?;
    let pass = gix_prompt::securely(prompt)?;
    println!("{pass}");
    Ok(())
}
