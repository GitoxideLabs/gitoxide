use std::mem::swap;

use expect_test::expect;
// use git::bstr::BStr;
// use git_repository as git;

use gix_imara_diff::BasicLineDiffPrinter;
use gix_imara_diff::InternedInput;
use gix_imara_diff::sources::words;
use gix_imara_diff::{Algorithm, Diff, UnifiedDiffConfig};

const ALL_ALGORITHMS: [Algorithm; 2] = [Algorithm::Histogram, Algorithm::Myers];

mod fuzzed {
    use std::time::{Duration, Instant};

    use arbitrary::Arbitrary;
    use gix_imara_diff::{Algorithm, Diff, InternedInput};

    #[derive(Debug, Arbitrary)]
    struct ComprehensiveDiffInput<'a> {
        before: &'a [u8],
        before_str: &'a str,
        after: &'a [u8],
        after_str: &'a str,
    }

    fn run_comprehensive_diff_fuzz_case(
        ComprehensiveDiffInput {
            before,
            before_str,
            after,
            after_str,
        }: ComprehensiveDiffInput<'_>,
    ) {
        let input = InternedInput::new(before, after);

        for algorithm in [Algorithm::Histogram, Algorithm::Myers, Algorithm::MyersMinimal] {
            let mut diff = Diff::compute(algorithm, &input);

            let _ = diff.count_additions();
            let _ = diff.count_removals();

            for hunk in diff.hunks() {
                let _ = hunk.is_pure_insertion();
                let _ = hunk.is_pure_removal();
                let _ = hunk.invert();
            }

            diff.postprocess_no_heuristic(&input);
            diff.postprocess_lines(&input);
        }

        let input = InternedInput::new(before_str, after_str);
        let mut word_input = InternedInput::default();
        let mut word_diff = Diff::default();

        let diff = Diff::compute(Algorithm::Myers, &input);
        for hunk in diff.hunks() {
            hunk.latin_word_diff(&input, &mut word_input, &mut word_diff);
        }
    }

    #[test]
    fn timeout_regression() {
        let input = ComprehensiveDiffInput::arbitrary(&mut arbitrary::Unstructured::new(include_bytes!(
            "../fixtures/clusterfuzz-testcase-minimized-gix-imara-diff-comprehensive_diff-6497314075377664"
        )))
        .expect("testcase matches the historical fuzz target input layout");

        let start = Instant::now();
        run_comprehensive_diff_fuzz_case(input);
        let elapsed = start.elapsed();

        let expected = Duration::from_secs(3);
        assert!(
            elapsed < expected,
            "clusterfuzz regression took {:?}, expected less than {:?}",
            elapsed,
            expected
        );
    }
}

#[test]
fn words_tokenizer() {
    let text = "Hello,  imara!\n (foo-bar_baz)";
    let tokens = words(text).collect::<Vec<_>>();
    assert_eq!(
        tokens,
        vec![
            "Hello", ",", "  ", "imara", "!", "\n", " ", "(", "foo", "-", "bar_baz", ")"
        ]
    );
}

#[test]
fn replace() {
    let before = r#"fn foo() -> Bar{
    let mut foo = 2.0;
    foo *= 100 / 2;
    println!("hello world")
}
"#;

    let after = r#"const TEST: i32 = 0;
fn foo() -> Bar{
    let mut foo = 2.0;
    foo *= 100 / 2;
    println!("hello world");
    println!("hello foo {TEST}");
}

"#;
    let input = InternedInput::new(before, after);
    for algorithm in ALL_ALGORITHMS {
        let mut diff = Diff::compute(algorithm, &input);
        diff.postprocess_lines(&input);
        expect![[r#"
            @@ -1,5 +1,8 @@
            +const TEST: i32 = 0;
             fn foo() -> Bar{
                 let mut foo = 2.0;
                 foo *= 100 / 2;
            -    println!("hello world")
            +    println!("hello world");
            +    println!("hello foo {TEST}");
             }
            +
        "#]]
        .assert_eq(
            &diff
                .unified_diff(
                    &BasicLineDiffPrinter(&input.interner),
                    UnifiedDiffConfig::default(),
                    &input,
                )
                .to_string(),
        );
    }
}

#[test]
fn identical_files() {
    let file = r#"fn foo() -> Bar{
    let mut foo = 2.0;
    foo *= 100 / 2;
}"#;

    for algorithm in ALL_ALGORITHMS {
        let input = InternedInput::new(file, file);
        let mut diff = Diff::compute(algorithm, &input);
        diff.postprocess_lines(&input);
        assert_eq!(
            diff.unified_diff(
                &BasicLineDiffPrinter(&input.interner),
                UnifiedDiffConfig::default(),
                &input,
            )
            .to_string(),
            ""
        );
    }
}

#[test]
fn simple_insert() {
    let before = r#"fn foo() -> Bar{
    let mut foo = 2.0;
    foo *= 100 / 2;
}"#;

    let after = r#"fn foo() -> Bar{
    let mut foo = 2.0;
    foo *= 100 / 2;
    println("hello world")
}"#;

    let mut input = InternedInput::new(before, after);
    for algorithm in ALL_ALGORITHMS {
        let mut diff = Diff::compute(algorithm, &input);
        diff.postprocess_lines(&input);
        expect![[r#"
          @@ -1,4 +1,5 @@
           fn foo() -> Bar{
               let mut foo = 2.0;
               foo *= 100 / 2;
          +    println("hello world")
           }
          "#]]
        .assert_eq(
            &diff
                .unified_diff(
                    &BasicLineDiffPrinter(&input.interner),
                    UnifiedDiffConfig::default(),
                    &input,
                )
                .to_string(),
        );

        swap(&mut input.before, &mut input.after);

        let mut diff = Diff::compute(algorithm, &input);
        diff.postprocess_lines(&input);
        expect![[r#"
            @@ -1,5 +1,4 @@
             fn foo() -> Bar{
                 let mut foo = 2.0;
                 foo *= 100 / 2;
            -    println("hello world")
             }
            "#]]
        .assert_eq(
            &diff
                .unified_diff(
                    &BasicLineDiffPrinter(&input.interner),
                    UnifiedDiffConfig::default(),
                    &input,
                )
                .to_string(),
        );
        swap(&mut input.before, &mut input.after);
    }
}

#[test]
fn unified_diff_context_lines_near_input_start_and_end() {
    let before = r#"a
b
c
d
e
f
g
h
i
"#;

    let after = r#"a
b
c
d
edit
f
g
h
i
"#;

    let input = InternedInput::new(before, after);
    for algorithm in ALL_ALGORITHMS {
        let mut diff = Diff::compute(algorithm, &input);
        diff.postprocess_lines(&input);
        expect![[r#"
          @@ -2,7 +2,7 @@
           b
           c
           d
          -e
          +edit
           f
           g
           h
          "#]]
        .assert_eq(
            &diff
                .unified_diff(
                    &BasicLineDiffPrinter(&input.interner),
                    UnifiedDiffConfig::default(),
                    &input,
                )
                .to_string(),
        );
    }
}

mod latin_word_diff {
    use crate::ALL_ALGORITHMS;

    use gix_imara_diff::sources::words;
    use gix_imara_diff::{Diff, InternedInput, Token};
    use std::mem::swap;
    use std::ops::Range;

    #[test]
    fn pure_insertion_or_removal() {
        let before = r#"fn foo() -> Bar{
    let mut foo = 2.0;
    foo *= 100 / 2;
}"#;
        let after = r#"fn foo() -> Bar{
    let mut foo = 2.0;
    foo *= 100 / 2;
    println("hello world")
}"#;
        let mut input = InternedInput::new(before, after);
        for algorithm in ALL_ALGORITHMS {
            let mut diff_input = InternedInput::default();
            let mut out = Diff::default();

            let mut diff = Diff::compute(algorithm, &input);
            diff.postprocess_lines(&input);

            let mut hunks = diff.hunks();
            let hunk = hunks.next().expect("missing first hunk");
            hunk.latin_word_diff(&input, &mut diff_input, &mut out);
            hunks = out.hunks();

            let first = hunks.next().expect("missing first inner hunk");
            assert!(first.is_pure_insertion());
            assert_eq!(first.before, 0..0);
            assert_eq!(first.after, 0..words("    println(\"hello world\")\n").count() as u32);
            assert_eq!(hunks.next(), None);
            assert_eq!(hunks.next(), None);
            assert_eq!(
                visualise(&diff_input, &first.after, &diff_input.after),
                "    |println|(|\"|hello| |world|\"|)|\n"
            );

            swap(&mut input.before, &mut input.after);

            let mut diff = Diff::compute(algorithm, &input);
            diff.postprocess_lines(&input);

            hunks = diff.hunks();
            let hunk = hunks.next().expect("missing first hunk");
            hunk.latin_word_diff(&input, &mut diff_input, &mut out);
            hunks = out.hunks();

            let first = hunks.next().expect("missing first inner hunk");
            assert!(first.is_pure_removal());
            assert_eq!(first.before, 0..words("    println(\"hello world\")\n").count() as u32);
            assert_eq!(first.after, 0..0);
            assert_eq!(hunks.next(), None);
            assert_eq!(hunks.next(), None);
            assert_eq!(
                visualise(&diff_input, &first.before, &diff_input.before),
                "    |println|(|\"|hello| |world|\"|)|\n"
            );

            swap(&mut input.before, &mut input.after);
        }
    }

    #[test]
    fn modification() {
        let before = r#"fn foo() -> Bar {
    let mut foo = 2.0;
    foo *= 100 / 2;
}"#;
        let after = r#"fn foo() -> Bar {
    let mut foo = 3.0 * 2.0;
    foo += 100 / 2;
}"#;
        let mut input = InternedInput::new(before, after);
        for algorithm in ALL_ALGORITHMS {
            let mut diff_input = InternedInput::default();
            let mut out = Diff::default();

            let mut diff = Diff::compute(algorithm, &input);
            diff.postprocess_lines(&input);

            let mut hunks = diff.hunks();
            let hunk = hunks.next().expect("missing first hunk");
            hunk.latin_word_diff(&input, &mut diff_input, &mut out);
            hunks = out.hunks();

            let first = hunks.next().expect("missing first inner hunk");
            assert!(first.is_pure_insertion());
            let off = words("    let mut foo = ").count() as u32;
            assert_eq!(first.before, off..off);
            let ins = words("3.0 * ").count() as u32;
            assert_eq!(first.after, off..ins + off);
            assert_eq!(visualise(&diff_input, &first.before, &diff_input.before), "");
            assert_eq!(visualise(&diff_input, &first.after, &diff_input.after), "3|.|0| |*| ");

            let second = hunks.next().expect("missing second inner hunk");
            let off = words(
                r#"    let mut foo = 2.0;
    foo "#,
            )
            .count() as u32;
            assert_eq!(second.before, off..1 + off);
            assert_eq!(second.after, ins + off..1 + ins + off);
            assert_eq!(visualise(&diff_input, &second.before, &diff_input.before), "*");
            assert_eq!(visualise(&diff_input, &second.after, &diff_input.after), "+");
            assert_eq!(hunks.next(), None);
            assert_eq!(hunks.next(), None);

            swap(&mut input.before, &mut input.after);

            let mut diff = Diff::compute(algorithm, &input);
            diff.postprocess_lines(&input);

            hunks = diff.hunks();
            let hunk = hunks.next().expect("missing first hunk");

            hunk.latin_word_diff(&input, &mut diff_input, &mut out);
            hunks = out.hunks();

            let first = hunks.next().expect("missing first inner hunk");
            assert!(first.is_pure_removal());
            let off = words("    let mut foo = ").count() as u32;
            let rem = words("3.0 * ").count() as u32;
            assert_eq!(first.before, off..rem + off);
            assert_eq!(first.after, off..off);
            let second = hunks.next().expect("missing second inner hunk");
            let off = words(
                r#"    let mut foo = 2.0;
    foo "#,
            )
            .count() as u32;
            assert_eq!(second.before, rem + off..1 + rem + off);
            assert_eq!(second.after, off..1 + off);
            assert_eq!(hunks.next(), None);
            assert_eq!(hunks.next(), None);

            swap(&mut input.before, &mut input.after);
        }
    }

    fn visualise(diff_input: &InternedInput<&str>, token_ids: &Range<u32>, tokens: &[Token]) -> String {
        token_ids
            .clone()
            .map(|id| {
                let id = id as usize;
                diff_input.interner[tokens[id]]
            })
            .collect::<Vec<_>>()
            .join("|")
    }
}

#[test]
#[cfg(not(miri))]
fn hand_checked_unidiffs() {
    let before = r#"use crate::{
    alpha::Alpha,
    beta::Beta,
    gamma::Gamma,
};

use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

pub struct Engine {
    cache: HashMap<String, usize>,
    steps: Vec<&'static str>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            steps: vec!["parse", "render"],
        }
    }

    pub fn update(&mut self, path: &Path) {
        let _ = path;
        self.steps.push("scan");
    }
}

fn unchanged_one() {
    println!("one");
}

fn unchanged_two() {
    println!("two");
}

pub enum Error {
    InvalidPath,
    Unknown,
}

pub struct Layer {
    pub depth: usize,
}

impl Layer {
    pub fn parse(&self) -> Result<(), Error> {
        Ok(())
    }
}
"#;
    let after = r#"use crate::{
    alpha::Alpha,
    beta::Beta,
    gamma::Gamma,
};

use std::{
    collections::HashMap,
    mem::replace,
    path::Path,
};

pub struct Engine {
    cache: HashMap<String, usize>,
    steps: Vec<&'static str>,
    dirty: bool,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            steps: vec!["parse", "render"],
            dirty: false,
        }
    }

    pub fn update(&mut self, path: &Path) {
        let _previous = replace(&mut self.dirty, true);
        let _ = path;
        self.steps.push("scan");
    }
}

fn unchanged_one() {
    println!("one");
}

fn unchanged_two() {
    println!("two");
}

pub enum Error {
    InvalidPath,
    InvalidState,
    Unknown,
}

pub struct Layer {
    pub depth: u32,
}

impl Layer {
    pub fn parse(&self) -> Result<(), Error> {
        Ok(())
    }
}
"#;

    for algorithm in ALL_ALGORITHMS {
        println!("{algorithm:?}");
        let input = InternedInput::new(before, after);
        let mut diff = Diff::compute(algorithm, &input);
        diff.postprocess_lines(&input);
        expect![[r#"
@@ -5,13 +5,15 @@
 };
 
 use std::{
-    collections::{HashMap, HashSet},
+    collections::HashMap,
+    mem::replace,
     path::Path,
 };
 
 pub struct Engine {
     cache: HashMap<String, usize>,
     steps: Vec<&'static str>,
+    dirty: bool,
 }
 
 impl Engine {
@@ -19,10 +21,12 @@
         Self {
             cache: HashMap::new(),
             steps: vec!["parse", "render"],
+            dirty: false,
         }
     }
 
     pub fn update(&mut self, path: &Path) {
+        let _previous = replace(&mut self.dirty, true);
         let _ = path;
         self.steps.push("scan");
     }
@@ -38,11 +42,12 @@
 
 pub enum Error {
     InvalidPath,
+    InvalidState,
     Unknown,
 }
 
 pub struct Layer {
-    pub depth: usize,
+    pub depth: u32,
 }
 
 impl Layer {
"#]]
        .assert_eq(
            &diff
                .unified_diff(
                    &BasicLineDiffPrinter(&input.interner),
                    UnifiedDiffConfig::default(),
                    &input,
                )
                .to_string(),
        );
    }
}

#[test]
fn postprocess() {
    let before = r#"
       /*
        * Stay on the safe side. if read_directory() has run once on
        * "dir", some sticky flag may have been left. Clear them all.
        */
       clear_sticky(dir);

       /*
        * exclude patterns are treated like positive ones in
        * create_simplify. Usually exclude patterns should be a
        * subset of positive ones, which has no impacts on
        * foo
        * bar
        * test
        */
        foo
    "#;
    let after = r#"
       /*
        * exclude patterns are treated like positive ones in
        * create_simplify. Usually exclude patterns should be a
        * subset of positive ones, which has no impacts on
        * foo
        * bar
        * test
        */
        foo
    "#;

    let input = InternedInput::new(before, after);
    for algorithm in [Algorithm::Histogram, Algorithm::Myers] {
        let mut diff = Diff::compute(algorithm, &input);
        diff.postprocess_lines(&input);
        let diff = diff
            .unified_diff(
                &BasicLineDiffPrinter(&input.interner),
                UnifiedDiffConfig::default(),
                &input,
            )
            .to_string();
        expect![[r#"
            @@ -1,10 +1,4 @@
             
            -       /*
            -        * Stay on the safe side. if read_directory() has run once on
            -        * "dir", some sticky flag may have been left. Clear them all.
            -        */
            -       clear_sticky(dir);
            -
                    /*
                     * exclude patterns are treated like positive ones in
                     * create_simplify. Usually exclude patterns should be a
        "#]]
        .assert_eq(&diff);
    }
}
