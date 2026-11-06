//! In-process comparison against the iconv API used by Git, without subprocess startup costs.
//! Run `cargo bench -p gix-encoding --bench convert --all-features -- --check` for the complete regression gate.

#[cfg(all(target_os = "linux", target_env = "gnu"))]
mod linux {
    use std::{
        ffi::{CString, c_char, c_void},
        hint::black_box,
        time::{Duration, Instant},
    };

    use gix_encoding::{Converter, Encoding, convert, convert::Status};

    unsafe extern "C" {
        fn iconv_open(to: *const c_char, from: *const c_char) -> *mut c_void;
        fn iconv(
            descriptor: *mut c_void,
            input: *mut *mut c_char,
            input_left: *mut usize,
            output: *mut *mut c_char,
            output_left: *mut usize,
        ) -> usize;
        fn iconv_close(descriptor: *mut c_void) -> i32;
    }

    fn reference(input: &[u8], from: &CString, to: &CString) -> Vec<u8> {
        // SAFETY: Both names are valid NUL-terminated strings. The returned handle
        // is checked before use and belongs exclusively to this invocation.
        let descriptor = unsafe { iconv_open(to.as_ptr(), from.as_ptr()) };
        assert_ne!(
            descriptor as usize,
            usize::MAX,
            "benchmark encodings are supported by iconv"
        );
        // Git initially allocates one output byte per input byte plus its NUL
        // terminator. Write into spare capacity without charging libc for Rust's
        // zero initialization; only expose bytes that iconv actually initialized.
        let mut output: Vec<u8> = Vec::with_capacity(input.len() + 1);
        let mut input_pos = input.as_ptr().cast_mut().cast::<c_char>();
        let mut input_left = input.len();
        loop {
            let available = output.capacity() - output.len() - 1;
            // SAFETY: The starting pointer is within the allocation, and the
            // advertised size excludes all previously initialized bytes.
            let mut output_pos = unsafe { output.as_mut_ptr().add(output.len()) }.cast::<c_char>();
            let mut output_left = available;
            // SAFETY: The input is readable for input_left bytes and output_pos
            // points to available writable spare bytes. iconv initializes every
            // byte reflected by its updated output_left value.
            // Like Git, do not flush a terminal shift state after input ends.
            let result = unsafe {
                iconv(
                    descriptor,
                    &mut input_pos,
                    &mut input_left,
                    &mut output_pos,
                    &mut output_left,
                )
            };
            // SAFETY: iconv initialized precisely available - output_left bytes
            // beyond the previous initialized prefix.
            unsafe { output.set_len(output.len() + available - output_left) };
            if result != usize::MAX {
                break;
            }
            let error = std::io::Error::last_os_error();
            assert_eq!(
                error.raw_os_error(),
                Some(7),
                "only E2BIG may occur for valid benchmark inputs"
            );
            // Git grows to written + remaining_input * 2 + 32 bytes.
            let additional = input_left * 2 + 32;
            output.reserve_exact(additional);
        }
        // SAFETY: This closes the successfully opened descriptor exactly once.
        let closed = unsafe { iconv_close(descriptor) };
        assert_eq!(closed, 0, "iconv closes its descriptor");
        assert_eq!(input_left, 0, "iconv consumes the complete benchmark input");
        output
    }

    struct ReferenceStream {
        descriptor: *mut c_void,
        output: Vec<u8>,
    }

    impl ReferenceStream {
        fn new(from: Encoding, to: Encoding) -> Self {
            let from = CString::new(from.name()).expect("encoding names contain no NUL");
            let to = CString::new(to.name()).expect("encoding names contain no NUL");
            // SAFETY: The strings are NUL-terminated and iconv does not retain their pointers.
            let descriptor = unsafe { iconv_open(to.as_ptr(), from.as_ptr()) };
            assert_ne!(
                descriptor as usize,
                usize::MAX,
                "benchmark encodings are supported by iconv"
            );
            Self {
                descriptor,
                output: vec![0; 8192],
            }
        }

        fn run(&mut self, input: &[u8], mut consume: impl FnMut(&[u8])) {
            // SAFETY: With all pointers null, iconv resets this exclusively owned
            // descriptor without reading or writing memory.
            let reset = unsafe {
                iconv(
                    self.descriptor,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            assert_ne!(reset, usize::MAX, "the reusable iconv descriptor resets");
            let mut offset = 0;
            let mut end = input.len().min(8192);
            loop {
                // SAFETY: offset never exceeds end, which never exceeds input.len().
                let mut input_pos = unsafe { input.as_ptr().add(offset) }.cast_mut().cast::<c_char>();
                let mut input_left = end - offset;
                let mut output_pos = self.output.as_mut_ptr().cast::<c_char>();
                let mut output_left = self.output.len();
                // SAFETY: Input and output pointers refer to live buffers of the
                // advertised size. This descriptor is used by only this thread.
                let result = unsafe {
                    iconv(
                        self.descriptor,
                        &mut input_pos,
                        &mut input_left,
                        &mut output_pos,
                        &mut output_left,
                    )
                };
                let error = (result == usize::MAX).then(|| std::io::Error::last_os_error().raw_os_error());
                offset = end - input_left;
                consume(&self.output[..self.output.len() - output_left]);
                if let Some(error) = error {
                    match error {
                        Some(7) => continue,                // E2BIG: supply more output space.
                        Some(22) if end < input.len() => {} // EINVAL: retain a split character.
                        error => panic!("unexpected iconv error {error:?} for valid streaming input"),
                    }
                } else if offset == input.len() {
                    break;
                }
                end = (end + 8192).min(input.len());
            }
        }
    }

    impl Drop for ReferenceStream {
        fn drop(&mut self) {
            // SAFETY: This is the only owner of a successfully opened descriptor.
            let closed = unsafe { iconv_close(self.descriptor) };
            assert_eq!(closed, 0, "iconv closes the reusable descriptor");
        }
    }

    fn stream(converter: &mut Converter, input: &[u8], output: &mut [u8], mut consume: impl FnMut(&[u8])) {
        converter.reset();
        let mut offset = 0;
        let mut end = input.len().min(8192);
        loop {
            let progress = converter
                .convert(&input[offset..end], output, end == input.len())
                .expect("valid streaming benchmark input");
            offset += progress.read;
            consume(&output[..progress.written]);
            match progress.status {
                Status::OutputFull => {}
                Status::InputEmpty => end = (end + 8192).min(input.len()),
                Status::Finished => {
                    assert_eq!(offset, input.len(), "streaming consumes the entire input");
                    break;
                }
            }
        }
    }

    fn elapsed(mut run: impl FnMut(), iterations: u64) -> Duration {
        let start = Instant::now();
        for _ in 0..iterations {
            run();
        }
        start.elapsed()
    }

    fn median(samples: &mut [f64]) -> f64 {
        samples.sort_by(f64::total_cmp);
        samples[samples.len() / 2]
    }

    fn measure(input: &[u8], from: Encoding, to: Encoding, quick: bool) -> (f64, f64) {
        let from_name = CString::new(from.name()).expect("encoding names contain no NUL");
        let to_name = CString::new(to.name()).expect("encoding names contain no NUL");
        assert_eq!(
            convert(input, from, to).expect("valid benchmark input"),
            reference(input, &from_name, &to_name),
            "benchmark outputs must agree before measuring performance"
        );
        let ours = || {
            black_box(convert(black_box(input), from, to).expect("valid benchmark input"));
        };
        let theirs = || {
            black_box(reference(black_box(input), &from_name, &to_name));
        };
        measure_pair(ours, theirs, quick)
    }

    fn measure_stream(input: &[u8], from: Encoding, to: Encoding, quick: bool) -> (f64, f64) {
        let mut converter = Converter::new(from, to);
        let mut reference = ReferenceStream::new(from, to);
        let mut output = vec![0; 8192];
        let mut actual = Vec::new();
        let mut expected = Vec::new();
        stream(&mut converter, input, &mut output, |bytes| {
            actual.extend_from_slice(bytes);
        });
        reference.run(input, |bytes| expected.extend_from_slice(bytes));
        assert_eq!(
            actual, expected,
            "streamed benchmark outputs agree before measuring performance"
        );
        let ours = || {
            stream(&mut converter, black_box(input), &mut output, |bytes| {
                black_box(bytes);
            });
        };
        let theirs = || {
            reference.run(black_box(input), |bytes| {
                black_box(bytes);
            });
        };
        measure_pair(ours, theirs, quick)
    }

    fn measure_pair(mut ours: impl FnMut(), mut theirs: impl FnMut(), quick: bool) -> (f64, f64) {
        let target = Duration::from_millis(if quick { 10 } else { 100 });
        let mut iterations = 1;
        loop {
            let duration = elapsed(&mut ours, iterations).min(elapsed(&mut theirs, iterations));
            if duration >= target {
                break;
            }
            iterations *= 2;
        }
        let samples = if quick { 3 } else { 9 };
        let mut ours_samples = Vec::with_capacity(samples);
        let mut theirs_samples = Vec::with_capacity(samples);
        for sample in 0..samples {
            // Alternate order to reduce bias from warm caches and clock changes.
            let (ours_duration, theirs_duration) = if sample % 2 == 0 {
                (elapsed(&mut ours, iterations), elapsed(&mut theirs, iterations))
            } else {
                let theirs_duration = elapsed(&mut theirs, iterations);
                (elapsed(&mut ours, iterations), theirs_duration)
            };
            ours_samples.push(ours_duration.as_nanos() as f64 / iterations as f64);
            theirs_samples.push(theirs_duration.as_nanos() as f64 / iterations as f64);
        }
        (median(&mut ours_samples), median(&mut theirs_samples))
    }

    pub fn main() {
        let quick = std::env::args().any(|arg| arg == "--quick");
        let check = std::env::args().any(|arg| arg == "--check");
        let streaming = std::env::args().any(|arg| arg == "--stream");
        let only = std::env::var("GIX_ENCODING_BENCH_FILTER").ok();
        let cases = [
            (
                "UTF-16LE/ascii",
                "UTF-16LE",
                "A short ASCII line with words and numbers 12345.\n",
            ),
            ("UTF-16LE/text", "UTF-16LE", "Été Ελληνικά 日本語 😀\n"),
            ("UTF-16BE/text", "UTF-16BE", "Été Ελληνικά 日本語 😀\n"),
            ("UTF-32LE/text", "UTF-32LE", "Été Ελληνικά 日本語 😀\n"),
            (
                "ASCII/text",
                "ASCII",
                "A short ASCII line with words and numbers 12345.\n",
            ),
            (
                "ISO-8859-1/ascii",
                "ISO-8859-1",
                "A short ASCII line with words and numbers 12345.\n",
            ),
            ("ISO-8859-1/text", "ISO-8859-1", "Été à Noël, déjà vu. £100\n"),
            ("WINDOWS-1252/text", "WINDOWS-1252", "“Été à Noël” costs €100.\n"),
            ("ISO-8859-15/text", "ISO-8859-15", "Été à Noël costs €100.\n"),
            ("WINDOWS-1251/text", "WINDOWS-1251", "Привет мир\n"),
            ("KOI8-R/text", "KOI8-R", "Привет мир\n"),
            ("SHIFT-JIS/text", "SHIFT-JIS", "日本語の文章です。\n"),
            ("CP932/text", "CP932", "日本語の文章です。\n"),
            ("EUC-JP/text", "EUC-JP", "日本語の文章です。\n"),
            ("ISO-2022-JP/text", "ISO-2022-JP", "日本語の文章です。\n"),
            ("GB18030/text", "GB18030", "中文字符和表情😀\n"),
            ("BIG5/text", "BIG5", "中文字元\n"),
            ("EUC-KR/text", "EUC-KR", "한국어 문장입니다.\n"),
        ];
        println!("case,direction,input_bytes,gix_ns,iconv_ns,gix_over_iconv");
        let mut regressions = Vec::new();
        let mut measured = 0;
        for (name, label, sample) in cases {
            if only.as_ref().is_some_and(|filter| !name.contains(filter)) {
                continue;
            }
            measured += 1;
            let encoding = Encoding::for_label(label.as_bytes()).expect("benchmark encoding is enabled");
            for size in [1024_usize, 1024 * 1024] {
                let utf8 = sample.repeat(size.div_ceil(sample.len()));
                let encoded = convert(utf8.as_bytes(), Encoding::Utf8, encoding).expect("sample is representable");
                for (direction, input, from, to) in [
                    ("encode", utf8.as_bytes(), Encoding::Utf8, encoding),
                    ("decode", encoded.as_slice(), encoding, Encoding::Utf8),
                ] {
                    let (ours, theirs) = if streaming {
                        measure_stream(input, from, to, quick)
                    } else {
                        measure(input, from, to, quick)
                    };
                    let ratio = ours / theirs;
                    let mode = if streaming { "stream" } else { "buffer" };
                    println!(
                        "{mode}/{name},{direction},{},{ours:.1},{theirs:.1},{ratio:.3}",
                        input.len()
                    );
                    if ratio > 1.0 {
                        regressions.push(format!("{name}/{direction}/{}: {ratio:.3}x", input.len()));
                    }
                }
            }
        }
        if measured == 0 {
            eprintln!("No benchmark case matches GIX_ENCODING_BENCH_FILTER; check the filter.");
            std::process::exit(1);
        }
        if check && !regressions.is_empty() {
            eprintln!(
                "Slower than iconv (strict median comparison):\n{}",
                regressions.join("\n")
            );
            std::process::exit(1);
        }
    }
}

fn main() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    linux::main();
    #[cfg(not(all(target_os = "linux", target_env = "gnu")))]
    eprintln!("The iconv comparison targets GNU/Linux; run it there to obtain comparable measurements.");
}
