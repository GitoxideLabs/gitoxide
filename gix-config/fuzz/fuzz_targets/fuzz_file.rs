#![no_main]

use anyhow::Result;

use bstr::{BStr, BString};

use gix_config::file::{Metadata, init::Options};
use libfuzzer_sys::fuzz_target;
use std::collections::BTreeSet;
use std::convert::TryInto;
use std::hint::black_box;
use std::str;

fn fuzz_immutable_section(section: &gix_config::file::Section<'_>, buf: &mut Vec<u8>) {
    for (key, value) in section.body() {
        let _ = black_box((key, value));
    }
    let mut seen = BTreeSet::new();
    for key in section.value_names() {
        if seen.insert(key.clone()) {
            let _ = black_box(section.values(key.as_ref()));
        }
    }
    buf.clear();
    let _ = black_box(section.write_to(buf));
}

fn fuzz_mutable_section(
    file: &mut gix_config::File,
    section_name: &str,
    subsection_name: Option<BString>,
) -> Result<()> {
    use bstr::ByteSlice;
    let subsection_name: Option<&BStr> = subsection_name.as_ref().map(|b| (**b).as_bstr());

    // Mutate section.
    let section_id = {
        let mut section = file.section_mut(section_name, subsection_name)?;
        let key = section.value_names().next();

        if let Some(key) = key {
            section.push_newline();
            section.set(key, BStr::new("Set value"));
            section.push_newline();
        }
        let kv_pair = section.pop().map(|(key, value)| (key.to_owned(), value));
        if let Some((key, value)) = kv_pair {
            section.push_with_comment(key, Some(value.as_bstr()), "Popped");
        } else {
            section.push("new-implicit".try_into()?, None);
            section.push("new".try_into()?, Some("value".into()));
        }
        section.id()
    };

    _ = black_box(file.section_mut_by_id(section_id));

    let new_section_name = section_name.to_string() + "_new";
    _ = black_box(file.section_mut_or_create_new(&new_section_name, subsection_name));

    _ = black_box(file.remove_section(&new_section_name, subsection_name));

    _ = black_box(file.new_section(&new_section_name, Option::<BString>::None));
    let renamed_section_name = section_name.to_string() + "_renamed";
    let renamed_subsection_name: Option<BString> = subsection_name.map(|name| {
        let mut renamed = name.to_owned();
        renamed.extend_from_slice(b"_renamed");
        renamed
    });
    _ = black_box(file.rename_section(
        &new_section_name,
        subsection_name,
        &renamed_section_name,
        renamed_subsection_name.clone(),
    ));

    _ = black_box(file.rename_section_filter(
        &new_section_name,
        subsection_name,
        &renamed_section_name,
        renamed_subsection_name.clone(),
        |_| false,
    ));

    Ok(())
}

fn fuzz(input: &[u8]) -> Result<()> {
    let meta = Metadata::default();
    let options = Options::default();
    let file = gix_config::File::from_bytes_no_includes(input, meta.clone(), options)?;

    // Sections and frontmatter.
    _ = black_box(file.sections_and_ids().count());
    _ = black_box(file.sections_and_postmatter().count());
    _ = black_box(file.sections_by_name("section").map(std::iter::Iterator::count));
    _ = black_box(file.frontmatter());

    let mut buf = Vec::new();
    let mut sections = Vec::new();
    // Don't perform too much work as this can blow up the size of the file.
    for section in file.sections().take(10) {
        fuzz_immutable_section(&section, &mut buf);

        let header = section.header();
        let section_name = str::from_utf8(header.name()).unwrap();
        let subsection_name = header.subsection_name().map(std::borrow::ToOwned::to_owned);
        sections.push((section_name, subsection_name));
    }

    let mut mutated_file = file.clone();
    for (section_name, subsection_name) in sections.into_iter() {
        let _ = black_box(fuzz_mutable_section(&mut mutated_file, section_name, subsection_name));
    }

    _ = black_box(mutated_file.append(file));
    _ = black_box(gix_config::File::from_bytes_no_includes(
        &mutated_file.to_bstring(),
        meta,
        options,
    )?);

    Ok(())
}

fuzz_target!(|input: &[u8]| {
    _ = black_box(fuzz(input));
});
