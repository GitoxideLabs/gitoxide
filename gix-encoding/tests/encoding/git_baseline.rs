// Git delegates conversion to its host iconv. This crate pins GNU libc mappings;
// libiconv and musl differ, including the default byte order of UTF-16 and UTF-32.
#![cfg(all(target_os = "linux", target_env = "gnu"))]

use gix_encoding::{Encoding, convert};
use gix_error::TestResult;

#[test]
fn conversions_match_git() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("git-baseline.sh")?;
    let mut compared = std::collections::BTreeSet::new();
    let mut unsupported = std::collections::BTreeSet::new();
    for entry in std::fs::read_dir(fixture.join("cases"))? {
        let case = entry?.path();
        let label = std::fs::read(case.join("label"))?;
        let Some(encoding) = Encoding::for_label(&label) else {
            continue;
        };
        let label_display = String::from_utf8_lossy(&label);
        let status = std::fs::read_to_string(case.join("status"))?;
        if status.trim() == "unsupported" {
            unsupported.insert(encoding);
            eprintln!("Git's iconv does not support {label_display}; skipping this host-dependent baseline");
            continue;
        }

        let encoded = std::fs::read(case.join("encoded"))?;
        match status.trim() {
            "encode" => {
                let utf8 = std::fs::read(case.join("utf8"))?;
                assert_eq!(
                    convert(&utf8, Encoding::Utf8, encoding)?,
                    encoded,
                    "encoding {label_display} agrees with Git's checkout filter"
                );
            }
            "decode" => {
                let utf8 = std::fs::read(case.join("utf8"))?;
                assert_eq!(
                    convert(&encoded, encoding, Encoding::Utf8)?,
                    utf8,
                    "decoding {label_display} agrees with Git's check-in filter"
                );
            }
            "invalid-encode" => {
                let utf8 = std::fs::read(case.join("utf8"))?;
                assert!(
                    convert(&utf8, Encoding::Utf8, encoding).is_err(),
                    "invalid or unrepresentable input is rejected like Git's {label_display} checkout filter"
                );
            }
            "invalid" => assert!(
                convert(&encoded, encoding, Encoding::Utf8).is_err(),
                "invalid {label_display} is rejected like Git's check-in filter"
            ),
            status => panic!("unexpected fixture status {status}"),
        }
        compared.insert(encoding);
    }
    for &encoding in Encoding::ALL {
        assert!(
            compared.contains(&encoding) || unsupported.contains(&encoding),
            "the Git baseline exercises every enabled encoding supported by this host: {encoding}"
        );
    }
    for encoding in [Encoding::Utf8, Encoding::Utf16Le, Encoding::Ascii, Encoding::Iso8859_1] {
        assert!(
            compared.contains(&encoding),
            "core {encoding} comparisons must run even without optional features"
        );
    }
    Ok(())
}
