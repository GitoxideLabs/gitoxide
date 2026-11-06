use std::{sync::Barrier, thread};

use gix_encoding::{Converter, Encoding, convert, convert::Status};
use gix_error::TestResult;

#[test]
fn enabled_tables_support_concurrent_initialization_and_reuse() -> TestResult {
    // Other tests must not warm the process-wide lazy tables before the concurrent first accesses.
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let cases = [
        (Encoding::Ascii, "a\0z"),
        (Encoding::Iso8859_1, "é£"),
        #[cfg(feature = "legacy")]
        (Encoding::Iso8859_2, "Łódź"),
        #[cfg(feature = "legacy")]
        (Encoding::Iso8859_3, "Ħ"),
        #[cfg(feature = "legacy")]
        (Encoding::Iso8859_4, "Ā"),
        #[cfg(feature = "legacy")]
        (Encoding::Iso8859_5, "Я"),
        #[cfg(feature = "legacy")]
        (Encoding::Iso8859_6, "اب"),
        #[cfg(feature = "legacy")]
        (Encoding::Iso8859_7, "Ω"),
        #[cfg(feature = "legacy")]
        (Encoding::Iso8859_8, "אב"),
        #[cfg(feature = "legacy")]
        (Encoding::Iso8859_9, "Ğ"),
        #[cfg(feature = "legacy")]
        (Encoding::Iso8859_10, "Ŋ"),
        #[cfg(feature = "legacy")]
        (Encoding::Iso8859_11, "ก"),
        #[cfg(feature = "legacy")]
        (Encoding::Iso8859_13, "Ė"),
        #[cfg(feature = "legacy")]
        (Encoding::Iso8859_14, "Ḃ"),
        #[cfg(feature = "western")]
        (Encoding::Iso8859_15, "€"),
        #[cfg(feature = "legacy")]
        (Encoding::Iso8859_16, "Ș"),
        #[cfg(feature = "legacy")]
        (Encoding::Windows1250, "Ł"),
        #[cfg(feature = "legacy")]
        (Encoding::Windows1251, "Я"),
        #[cfg(feature = "western")]
        (Encoding::Windows1252, "“é” €"),
        #[cfg(feature = "legacy")]
        (Encoding::Windows1253, "Ω"),
        #[cfg(feature = "legacy")]
        (Encoding::Windows1254, "Ğ"),
        #[cfg(feature = "legacy")]
        (Encoding::Windows1255, "אב"),
        #[cfg(feature = "legacy")]
        (Encoding::Windows1256, "اب"),
        #[cfg(feature = "legacy")]
        (Encoding::Windows1257, "Ė"),
        #[cfg(feature = "legacy")]
        (Encoding::Windows1258, "ấ"),
        #[cfg(feature = "legacy")]
        (Encoding::Windows874, "ก"),
        #[cfg(feature = "legacy")]
        (Encoding::Tis620, "ก"),
        #[cfg(feature = "legacy")]
        (Encoding::Koi8R, "Я"),
        #[cfg(feature = "legacy")]
        (Encoding::Koi8U, "Ї"),
        #[cfg(feature = "legacy")]
        (Encoding::Macintosh, "é"),
        #[cfg(feature = "legacy")]
        (Encoding::MacCyrillic, "Я"),
        #[cfg(feature = "legacy")]
        (Encoding::Ibm437, "é"),
        #[cfg(feature = "legacy")]
        (Encoding::Ibm850, "ø"),
        #[cfg(feature = "legacy")]
        (Encoding::Ibm866, "Я"),
        #[cfg(feature = "legacy")]
        (Encoding::Cp1125, "Ґ"),
        #[cfg(feature = "asian")]
        (Encoding::ShiftJis, "日本語"),
        #[cfg(feature = "asian")]
        (Encoding::Cp932, "日本語"),
        #[cfg(feature = "asian")]
        (Encoding::EucJp, "日本語"),
        #[cfg(feature = "asian")]
        (Encoding::Iso2022Jp, "日本a語"),
        #[cfg(feature = "asian")]
        (Encoding::EucKr, "한국어"),
        #[cfg(feature = "asian")]
        (Encoding::Cp949, "갂한국어"),
        #[cfg(feature = "asian")]
        (Encoding::Gb2312, "中文"),
        #[cfg(feature = "asian")]
        (Encoding::Gbk, "中文"),
        #[cfg(feature = "asian")]
        (Encoding::Gb18030, "中文😀"),
        #[cfg(feature = "asian")]
        (Encoding::Big5, "中文"),
        #[cfg(feature = "asian")]
        (Encoding::Big5Hkscs, "Ê̄中Ê"),
    ];
    assert_eq!(
        cases.iter().map(|&(encoding, _)| encoding).collect::<Vec<_>>(),
        Encoding::ALL
            .iter()
            .copied()
            .filter(|encoding| !encoding.name().starts_with("UTF-"))
            .collect::<Vec<_>>(),
        "every enabled legacy codec has a non-ASCII sample, except ASCII itself"
    );
    for (encoding, text) in cases {
        let start = Barrier::new(8);
        thread::scope(|scope| -> TestResult {
            let workers: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| -> TestResult {
                        start.wait();
                        let mut encoder = Converter::new(Encoding::Utf8, encoding);
                        let mut decoder = Converter::new(encoding, Encoding::Utf8);
                        let mut encoded = [0; 64];
                        let mut decoded = [0; 64];
                        for _ in 0..16 {
                            let written = encoder.convert(text.as_bytes(), &mut encoded, true)?;
                            assert_eq!(
                                (written.read, written.status),
                                (text.len(), Status::Finished),
                                "{encoding}: each independent encoder consumes the complete sample"
                            );
                            let input = &encoded[..written.written];
                            let read = decoder.convert(input, &mut decoded, true)?;
                            assert_eq!(
                                (read.read, read.status),
                                (input.len(), Status::Finished),
                                "{encoding}: each independent decoder finishes, including pending composition"
                            );
                            assert_eq!(
                                &decoded[..read.written],
                                text.as_bytes(),
                                "{encoding}: concurrent table access preserves non-ASCII mappings"
                            );
                            assert_eq!(
                                convert(text.as_bytes(), Encoding::Utf8, encoding)?,
                                input,
                                "{encoding}: fresh converters agree with reusable encoders"
                            );
                            assert_eq!(
                                convert(input, encoding, Encoding::Utf8)?,
                                text.as_bytes(),
                                "{encoding}: fresh converters agree with reusable decoders"
                            );
                            encoder.reset();
                            decoder.reset();
                        }
                        Ok(())
                    })
                })
                .collect();
            for worker in workers {
                worker.join().expect("concurrent conversion must not panic")?;
            }
            Ok(())
        })?;
    }
    Ok(())
}
