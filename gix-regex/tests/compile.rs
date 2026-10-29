// Derived from posix-regex 0.1.4, upstream revision
// 063882aa6051b5075062d20d00b41ddc0fb8cd89.
// Copyright (c) 2018 jD91mZM2. Licensed under the MIT license; see LICENSE-MIT.

extern crate alloc;

use gix_regex::compile::*;
use gix_regex::ctype;

use alloc::{format, string::String};

fn compile(input: &[u8]) -> String {
    format!(
        "{:?}",
        PosixRegexBuilder::new(input)
            .with_default_classes()
            .compile_tokens()
            .expect("error compiling regex")
    )
}

#[test]
fn basic() {
    assert_eq!(
        compile(b"abc"),
        "\
Root 1..1
  Alternative 1..1
    'a' 1..1
    'b' 1..1
    'c' 1..1
"
    );
}
#[test]
fn groups() {
    assert_eq!(
        compile(br"\(abc\|bcd\|cde\)"),
        "\
Root 1..1
  Alternative 1..1
    Group(1) 1..1
      Alternative 1..1
        'a' 1..1
        'b' 1..1
        'c' 1..1
      Alternative 1..1
        'b' 1..1
        'c' 1..1
        'd' 1..1
      Alternative 1..1
        'c' 1..1
        'd' 1..1
        'e' 1..1
"
    );
    assert_eq!(
        compile(br"\(abc\|\(bcd\|cde\)\)"),
        "\
Root 1..1
  Alternative 1..1
    Group(1) 1..1
      Alternative 1..1
        'a' 1..1
        'b' 1..1
        'c' 1..1
      Alternative 1..1
        Group(2) 1..1
          Alternative 1..1
            'b' 1..1
            'c' 1..1
            'd' 1..1
          Alternative 1..1
            'c' 1..1
            'd' 1..1
            'e' 1..1
"
    );
}
#[test]
fn words() {
    assert_eq!(
        compile(br"\<word\>"),
        "\
Root 1..1
  Alternative 1..1
    < 1..1
    'w' 1..1
    'o' 1..1
    'r' 1..1
    'd' 1..1
    > 1..1
"
    );
}
#[test]
fn repetitions() {
    assert_eq!(
        compile(br"yeee*"),
        "\
Root 1..1
  Alternative 1..1
    'y' 1..1
    'e' 1..1
    'e' 1..1
    'e' 0..
"
    );
    assert_eq!(
        compile(br"yee\?"),
        "\
Root 1..1
  Alternative 1..1
    'y' 1..1
    'e' 1..1
    'e' 0..1
"
    );
    assert_eq!(
        compile(br"yee\+"),
        "\
Root 1..1
  Alternative 1..1
    'y' 1..1
    'e' 1..1
    'e' 1..
"
    );
    assert_eq!(
        compile(br"ye\{2}"),
        "\
Root 1..1
  Alternative 1..1
    'y' 1..1
    'e' 2..2
"
    );
    assert_eq!(
        compile(br"ye\{2,}"),
        "\
Root 1..1
  Alternative 1..1
    'y' 1..1
    'e' 2..
"
    );
    assert_eq!(
        compile(br"ye\{2,3}"),
        "\
Root 1..1
  Alternative 1..1
    'y' 1..1
    'e' 2..3
"
    );
}
#[test]
fn bracket() {
    assert_eq!(
        compile(b"[abc]"),
        "\
Root 1..1
  Alternative 1..1
    {invert: false, ['a', 'b', 'c']} 1..1
"
    );
    assert_eq!(
        compile(b"[^abc]"),
        "\
Root 1..1
  Alternative 1..1
    {invert: true, ['a', 'b', 'c']} 1..1
"
    );
    assert_eq!(
        compile(b"[]] [^]]"),
        "\
Root 1..1
  Alternative 1..1
    {invert: false, [']']} 1..1
    ' ' 1..1
    {invert: true, [']']} 1..1
"
    );
    assert_eq!(
        compile(b"[0-3] [a-c] [-1] [1-]"),
        "\
Root 1..1
  Alternative 1..1
    {invert: false, ['0', '1', '2', '3']} 1..1
    ' ' 1..1
    {invert: false, ['a', 'b', 'c']} 1..1
    ' ' 1..1
    {invert: false, ['-', '1']} 1..1
    ' ' 1..1
    {invert: false, ['1', '-']} 1..1
"
    );
    assert_eq!(
        compile(b"[[.-.]-/]"),
        "\
Root 1..1
  Alternative 1..1
    {invert: false, ['-', '.', '/']} 1..1
"
    );
    assert_eq!(
        compile(b"[[:digit:][:upper:]]"),
        format!(
            "\
Root 1..1
  Alternative 1..1
    {{invert: false, [{:p}, {:p}]}} 1..1
",
            ctype::is_digit as fn(u8) -> bool,
            ctype::is_upper as fn(u8) -> bool
        )
    );
}
#[test]
fn newline() {
    assert_eq!(
        compile(br"\r\n"),
        "\
Root 1..1
  Alternative 1..1
    '\\r' 1..1
    '\\n' 1..1
"
    );
}
#[test]
fn backref() {
    assert_eq!(
        compile(br"\([abc]\)\1"),
        "\
Root 1..1
  Alternative 1..1
    Group(1) 1..1
      Alternative 1..1
        {invert: false, ['a', 'b', 'c']} 1..1
    \\1 1..1
"
    )
}

fn compile_extended(input: &[u8]) -> String {
    format!(
        "{:?}",
        PosixRegexBuilder::new(input)
            .with_default_classes()
            .extended(true)
            .compile_tokens()
            .expect("error compiling regex")
    )
}

#[test]
fn basic_extended() {
    assert_eq!(
        compile_extended(b"abc"),
        "\
Root 1..1
  Alternative 1..1
    'a' 1..1
    'b' 1..1
    'c' 1..1
"
    );
}
#[test]
fn groups_extended() {
    assert_eq!(
        compile_extended(br"(abc|bcd|cde)"),
        "\
Root 1..1
  Alternative 1..1
    Group(1) 1..1
      Alternative 1..1
        'a' 1..1
        'b' 1..1
        'c' 1..1
      Alternative 1..1
        'b' 1..1
        'c' 1..1
        'd' 1..1
      Alternative 1..1
        'c' 1..1
        'd' 1..1
        'e' 1..1
"
    );
    assert_eq!(
        compile_extended(br"(abc|(bcd|cde))"),
        "\
Root 1..1
  Alternative 1..1
    Group(1) 1..1
      Alternative 1..1
        'a' 1..1
        'b' 1..1
        'c' 1..1
      Alternative 1..1
        Group(2) 1..1
          Alternative 1..1
            'b' 1..1
            'c' 1..1
            'd' 1..1
          Alternative 1..1
            'c' 1..1
            'd' 1..1
            'e' 1..1
"
    );
}
#[test]
fn words_extended() {
    assert_eq!(
        compile_extended(br"\<word\>"),
        "\
Root 1..1
  Alternative 1..1
    < 1..1
    'w' 1..1
    'o' 1..1
    'r' 1..1
    'd' 1..1
    > 1..1
"
    );
}

#[test]
fn repetitions_extended() {
    assert_eq!(
        compile_extended(br"yeee*"),
        "\
Root 1..1
  Alternative 1..1
    'y' 1..1
    'e' 1..1
    'e' 1..1
    'e' 0..
"
    );
    assert_eq!(
        compile_extended(br"yee?"),
        "\
Root 1..1
  Alternative 1..1
    'y' 1..1
    'e' 1..1
    'e' 0..1
"
    );
    assert_eq!(
        compile_extended(br"yee+"),
        "\
Root 1..1
  Alternative 1..1
    'y' 1..1
    'e' 1..1
    'e' 1..
"
    );
    assert_eq!(
        compile_extended(br"ye{2}"),
        "\
Root 1..1
  Alternative 1..1
    'y' 1..1
    'e' 2..2
"
    );
    assert_eq!(
        compile_extended(br"ye{2,}"),
        "\
Root 1..1
  Alternative 1..1
    'y' 1..1
    'e' 2..
"
    );
    assert_eq!(
        compile_extended(br"ye{2,3}"),
        "\
Root 1..1
  Alternative 1..1
    'y' 1..1
    'e' 2..3
"
    );
}
#[test]
fn bracket_extended() {
    assert_eq!(
        compile_extended(b"[abc]"),
        "\
Root 1..1
  Alternative 1..1
    {invert: false, ['a', 'b', 'c']} 1..1
"
    );
    assert_eq!(
        compile_extended(b"[^abc]"),
        "\
Root 1..1
  Alternative 1..1
    {invert: true, ['a', 'b', 'c']} 1..1
"
    );
    assert_eq!(
        compile_extended(b"[]] [^]]"),
        "\
Root 1..1
  Alternative 1..1
    {invert: false, [']']} 1..1
    ' ' 1..1
    {invert: true, [']']} 1..1
"
    );
    assert_eq!(
        compile_extended(b"[0-3] [a-c] [-1] [1-]"),
        "\
Root 1..1
  Alternative 1..1
    {invert: false, ['0', '1', '2', '3']} 1..1
    ' ' 1..1
    {invert: false, ['a', 'b', 'c']} 1..1
    ' ' 1..1
    {invert: false, ['-', '1']} 1..1
    ' ' 1..1
    {invert: false, ['1', '-']} 1..1
"
    );
    assert_eq!(
        compile_extended(b"[[.-.]-/]"),
        "\
Root 1..1
  Alternative 1..1
    {invert: false, ['-', '.', '/']} 1..1
"
    );
    assert_eq!(
        compile_extended(b"[[:digit:][:upper:]]"),
        format!(
            "\
Root 1..1
  Alternative 1..1
    {{invert: false, [{:p}, {:p}]}} 1..1
",
            ctype::is_digit as fn(u8) -> bool,
            ctype::is_upper as fn(u8) -> bool
        )
    );
}
#[test]
fn newline_extended() {
    assert_eq!(
        compile_extended(br"\r\n"),
        "\
Root 1..1
  Alternative 1..1
    '\\r' 1..1
    '\\n' 1..1
"
    );
}
#[test]
fn backref_extended() {
    assert_eq!(
        compile_extended(br"([abc])\1"),
        "\
Root 1..1
  Alternative 1..1
    Group(1) 1..1
      Alternative 1..1
        {invert: false, ['a', 'b', 'c']} 1..1
    \\1 1..1
"
    )
}
