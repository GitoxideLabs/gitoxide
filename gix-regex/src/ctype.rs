// Derived from posix-regex 0.1.4, upstream revision
// 063882aa6051b5075062d20d00b41ddc0fb8cd89.
// Copyright (c) 2018 jD91mZM2. Licensed under the MIT license; see LICENSE-MIT.

pub const fn is_alnum(c: u8) -> bool {
    is_alpha(c) || is_digit(c)
}
pub const fn is_alpha(c: u8) -> bool {
    c.is_ascii_alphabetic()
}
pub const fn is_blank(c: u8) -> bool {
    c == b' ' || c == b'\t'
}
pub const fn is_cntrl(c: u8) -> bool {
    c.is_ascii_control()
}
pub const fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}
pub const fn is_graph(c: u8) -> bool {
    c.is_ascii_graphic()
}
pub const fn is_lower(c: u8) -> bool {
    c.is_ascii_lowercase()
}
pub fn is_print(c: u8) -> bool {
    (0x20..=0x7e).contains(&c)
}
pub const fn is_punct(c: u8) -> bool {
    is_graph(c) && !is_alnum(c)
}
pub fn is_space(c: u8) -> bool {
    c == b' ' || (0x9..=0xD).contains(&c)
}
pub const fn is_upper(c: u8) -> bool {
    c.is_ascii_uppercase()
}
pub const fn is_xdigit(c: u8) -> bool {
    c.is_ascii_hexdigit()
}

pub const fn is_word(c: u8) -> bool {
    is_alnum(c) || c == b'_'
}

pub const fn is_word_boundary(c: u8) -> bool {
    !is_word(c)
}
