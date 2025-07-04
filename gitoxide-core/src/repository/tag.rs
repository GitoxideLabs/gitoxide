pub fn list(repo: gix::Repository, out: &mut dyn std::io::Write) -> anyhow::Result<()> {
    let platform = repo.references()?;

    for mut reference in (platform.tags()?).flatten() {
        let tag = reference.peel_to_tag();
        let tag_ref = tag.as_ref().map(gix::Tag::decode);

        // `name` is the name of the file in `refs/tags/`. This applies to both lightweight as well
        // as annotated tags.
        let name = reference.name().shorten();

        match tag_ref {
            Ok(Ok(tag_ref)) => {
                // `tag_name` is the name provided by the user via `git tag -a/-s/-u`. It is only
                // present for annotated tags.
                let tag_name = tag_ref.name;

                if name == tag_name {
                    writeln!(out, "{name} *")?;
                } else {
                    writeln!(out, "{name} [tag name: {}]", tag_ref.name)?;
                }
            }
            _ => {
                writeln!(out, "{name}")?;
            }
        }
    }

    Ok(())
}
