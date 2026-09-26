use std::path::Path;

pub fn discover(repo: &Path, mut out: impl std::io::Write) -> anyhow::Result<()> {
    let mut has_err = false;
    writeln!(out, "open (strict) {}:", repo.display())?;
    has_err |= print_result(
        &mut out,
        gix::open_opts(repo, gix::open::Options::default().strict_config(true)),
    )?;

    if has_err {
        writeln!(out, "open (lenient) {}:", repo.display())?;
        has_err |= print_result(
            &mut out,
            gix::open_opts(repo, gix::open::Options::default().strict_config(false)),
        )?;
    }

    writeln!(out)?;
    writeln!(out, "discover from {}:", repo.display())?;
    has_err |= print_result(&mut out, gix::discover(repo))?;

    writeln!(out)?;
    writeln!(out, "discover (plumbing) from {}:", repo.display())?;
    has_err |= print_result(&mut out, gix::discover::upwards(repo))?;

    if has_err {
        writeln!(out)?;
        anyhow::bail!("At least one operation failed")
    }

    Ok(())
}

fn print_result<T, E>(mut out: impl std::io::Write, res: Result<T, E>) -> std::io::Result<bool>
where
    T: std::fmt::Debug,
    E: std::error::Error + Send + Sync + 'static,
{
    let mut has_err = false;
    let to_print = match res {
        Ok(good) => {
            format!("{good:#?}")
        }
        Err(err) => {
            has_err = true;
            format!("{:?}", anyhow::Error::from(err))
        }
    };
    indent(&mut out, to_print)?;
    Ok(has_err)
}

fn indent(mut out: impl std::io::Write, msg: impl Into<String>) -> std::io::Result<()> {
    for line in msg.into().lines() {
        writeln!(out, "\t{line}")?;
    }
    Ok(())
}
