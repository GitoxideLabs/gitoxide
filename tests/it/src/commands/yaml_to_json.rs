use std::{
    fs,
    io::{self, Read, Write},
    path::Path,
};

use gix::{
    Result,
    error::{ResultExt, message},
};

pub fn yaml_to_json(file: Option<&Path>) -> Result<()> {
    let input = match file {
        Some(path) if path != Path::new("-") => {
            fs::read(path).or_raise(|| message!("Read YAML file {}", path.display()))?
        }
        _ => {
            let mut input = Vec::new();
            io::stdin()
                .lock()
                .read_to_end(&mut input)
                .or_raise(|| message("Read YAML from stdin"))?;
            input
        }
    };
    let document = internal_tools::yaml_to_json(&input)?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer_pretty(&mut stdout, &document).or_raise(|| message("Write JSON"))?;
    writeln!(stdout).or_raise(|| message("Write JSON newline"))
}
