use std::io::{self, Write};

use crate::transport::packetline::blocking_io::encode::{data_to_write, flush_to_write};

use crate::serve::RefAdvertisement;

/// Write a V1 ref advertisement to `writer`.
pub fn write_v1<W: Write>(writer: &mut W, refs: &[RefAdvertisement<'_>], capabilities: &[&str]) -> io::Result<()> {
    let mut all_caps: Vec<String> = refs
        .iter()
        .filter_map(|r| {
            r.symref_target.map(|target| {
                format!(
                    "symref={}:{}",
                    std::str::from_utf8(r.name).expect("valid UTF-8 ref name"),
                    std::str::from_utf8(target).expect("valid UTF-8 symref target")
                )
            })
        })
        .collect();
    all_caps.extend(capabilities.iter().map(ToString::to_string));
    let caps = all_caps.join(" ");

    if refs.is_empty() {
        let mut line = Vec::new();
        gix_hash::ObjectId::null(gix_hash::Kind::Sha1).write_hex_to(&mut line)?;
        line.extend_from_slice(b" capabilities^{}\0");
        line.extend_from_slice(caps.as_bytes());
        line.push(b'\n');
        data_to_write(&line, &mut *writer)?;
    } else {
        for (i, r) in refs.iter().enumerate() {
            let mut line = Vec::new();
            r.object_id.write_hex_to(&mut line)?;
            line.push(b' ');
            line.extend_from_slice(r.name);
            if i == 0 {
                line.push(b'\0');
                line.extend_from_slice(caps.as_bytes());
            }
            line.push(b'\n');
            data_to_write(&line, &mut *writer)?;

            if let Some(peeled) = r.peeled {
                let mut line = Vec::new();
                peeled.write_hex_to(&mut line)?;
                line.push(b' ');
                line.extend_from_slice(r.name);
                line.extend_from_slice(b"^{}\n");
                data_to_write(&line, &mut *writer)?;
            }
        }
    }

    flush_to_write(&mut *writer)?;
    Ok(())
}

/// Write a V2 ls-refs response to `writer`.
pub fn write_v2_ls_refs<W: Write>(writer: &mut W, refs: &[RefAdvertisement<'_>]) -> io::Result<()> {
    for r in refs.iter() {
        let mut line = Vec::new();
        r.object_id.write_hex_to(&mut line)?;
        line.push(b' ');
        line.extend_from_slice(r.name);

        if let Some(symref_target) = r.symref_target {
            line.extend_from_slice(b" symref-target:");
            line.extend_from_slice(symref_target);
        }

        if let Some(peeled) = r.peeled {
            line.extend_from_slice(b" peeled:");
            peeled.write_hex_to(&mut line)?;
        }
        line.push(b'\n');
        data_to_write(&line, &mut *writer)?;
    }
    flush_to_write(&mut *writer)?;
    Ok(())
}

/// Write a V2 capabilities advertisement to `writer`.
pub fn write_capabilities_v2<W: Write>(writer: &mut W, capabilities: &[(&str, Option<&str>)]) -> io::Result<()> {
    data_to_write(b"version 2\n", &mut *writer)?;
    for (name, val) in capabilities {
        let mut line = Vec::new();
        line.extend_from_slice(name.as_bytes());
        if let Some(value) = val {
            line.push(b'=');
            line.extend_from_slice(value.as_bytes());
        }
        line.push(b'\n');
        data_to_write(&line, &mut *writer)?;
    }
    flush_to_write(&mut *writer)?;
    Ok(())
}
