use std::{borrow::Cow, fmt::Display, str::FromStr};

use bstr::{BStr, BString};
use gix_error::{ErrorExt, ResultExt, ValidationError};

use crate::{Color, Error};

impl Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut write_space = None;
        if let Some(fg) = self.foreground {
            fg.fmt(f)?;
            write_space = Some(());
        }

        if let Some(bg) = self.background {
            if write_space.take().is_some() {
                write!(f, " ")?;
            }
            bg.fmt(f)?;
            write_space = Some(());
        }

        if !self.attributes.is_empty() {
            if write_space.take().is_some() {
                write!(f, " ")?;
            }
            self.attributes.fmt(f)?;
        }
        Ok(())
    }
}

fn color_err(input: impl Into<BString>) -> ValidationError {
    ValidationError::new_with_input(
        "Colors are specific color values and their attributes, like 'brightred', or 'blue'",
        input,
    )
}

impl TryFrom<&BStr> for Color {
    type Error = Error;

    fn try_from(s: &BStr) -> Result<Self, Self::Error> {
        let s = std::str::from_utf8(s).or_raise(|| color_err(s))?;
        enum ColorItem {
            Value(Name),
            Attr(Attribute),
        }

        let items = s.split_whitespace().filter_map(|s| {
            if s.is_empty() {
                return None;
            }

            Some(
                Name::from_str(s)
                    .map(ColorItem::Value)
                    .or_else(|_| Attribute::from_str(s).map(ColorItem::Attr)),
            )
        });

        let mut foreground = None;
        let mut background = None;
        let mut attributes = Attribute::empty();
        for item in items {
            match item {
                Ok(item) => match item {
                    ColorItem::Value(v) => {
                        if foreground.is_none() {
                            foreground = Some(v);
                        } else if background.is_none() {
                            background = Some(v);
                        } else {
                            return Err(color_err(s).raise());
                        }
                    }
                    ColorItem::Attr(a) => attributes |= a,
                },
                Err(_) => return Err(color_err(s).raise()),
            }
        }

        Ok(Color {
            foreground,
            background,
            attributes,
        })
    }
}

impl TryFrom<&str> for Color {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(BStr::new(value))
    }
}

impl TryFrom<Cow<'_, BStr>> for Color {
    type Error = Error;

    fn try_from(c: Cow<'_, BStr>) -> Result<Self, Self::Error> {
        Self::try_from(c.as_ref())
    }
}

impl TryFrom<BString> for Color {
    type Error = Error;

    fn try_from(value: BString) -> Result<Self, Self::Error> {
        Self::try_from(BStr::new(&value))
    }
}

/// Discriminating enum for names of [`Color`] values.
///
/// `git-config` supports the eight standard colors, their bright variants, an
/// ANSI color code, or a 24-bit hex value prefixed with an octothorpe/hash.
/// Color names and the `bright` prefix are matched case-insensitively, and
/// `bright` may only precede one of the eight standard colors.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Name {
    /// The `normal` color name.
    Normal,
    /// The terminal's default color.
    Default,
    /// Black.
    Black,
    /// Bright black.
    BrightBlack,
    /// Red.
    Red,
    /// Bright red.
    BrightRed,
    /// Green.
    Green,
    /// Bright green.
    BrightGreen,
    /// Yellow.
    Yellow,
    /// Bright yellow.
    BrightYellow,
    /// Blue.
    Blue,
    /// Bright blue.
    BrightBlue,
    /// Magenta.
    Magenta,
    /// Bright magenta.
    BrightMagenta,
    /// Cyan.
    Cyan,
    /// Bright cyan.
    BrightCyan,
    /// White.
    White,
    /// Bright white.
    BrightWhite,
    /// A color from the ANSI 256-color palette.
    Ansi(
        /// The palette index.
        u8,
    ),
    /// A 24-bit RGB color.
    Rgb(
        /// The red component.
        u8,
        /// The green component.
        u8,
        /// The blue component.
        u8,
    ),
}

impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::Default => write!(f, "default"),
            Self::Black => write!(f, "black"),
            Self::BrightBlack => write!(f, "brightblack"),
            Self::Red => write!(f, "red"),
            Self::BrightRed => write!(f, "brightred"),
            Self::Green => write!(f, "green"),
            Self::BrightGreen => write!(f, "brightgreen"),
            Self::Yellow => write!(f, "yellow"),
            Self::BrightYellow => write!(f, "brightyellow"),
            Self::Blue => write!(f, "blue"),
            Self::BrightBlue => write!(f, "brightblue"),
            Self::Magenta => write!(f, "magenta"),
            Self::BrightMagenta => write!(f, "brightmagenta"),
            Self::Cyan => write!(f, "cyan"),
            Self::BrightCyan => write!(f, "brightcyan"),
            Self::White => write!(f, "white"),
            Self::BrightWhite => write!(f, "brightwhite"),
            Self::Ansi(num) => num.fmt(f),
            Self::Rgb(r, g, b) => write!(f, "#{r:02x}{g:02x}{b:02x}"),
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Name {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl FromStr for Name {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const BASIC: &[(&str, Name, Name)] = &[
            ("black", Name::Black, Name::BrightBlack),
            ("red", Name::Red, Name::BrightRed),
            ("green", Name::Green, Name::BrightGreen),
            ("yellow", Name::Yellow, Name::BrightYellow),
            ("blue", Name::Blue, Name::BrightBlue),
            ("magenta", Name::Magenta, Name::BrightMagenta),
            ("cyan", Name::Cyan, Name::BrightCyan),
            ("white", Name::White, Name::BrightWhite),
        ];

        if s.eq_ignore_ascii_case("normal") {
            return Ok(Self::Normal);
        }

        let (name, is_bright) = match s.split_at_checked("bright".len()) {
            Some((prefix, rest)) if prefix.eq_ignore_ascii_case("bright") => (rest, true),
            _ => (s, false),
        };

        for &(basic, plain, brightened) in BASIC {
            if name.eq_ignore_ascii_case(basic) {
                return Ok(if is_bright { brightened } else { plain });
            }
        }

        if is_bright {
            return Err(color_err(s).raise());
        }

        if s.eq_ignore_ascii_case("normal") || s == "-1" {
            return Ok(Self::Normal);
        }

        if s.eq_ignore_ascii_case("default") {
            return Ok(Self::Default);
        }

        if let Ok(v) = u8::from_str(s) {
            return Ok(Self::Ansi(v));
        }

        if let Some(s) = s.strip_prefix('#') {
            if s.len() == 6 && s.is_char_boundary(2) && s.is_char_boundary(4) && s.is_char_boundary(6) {
                let rgb = (
                    u8::from_str_radix(&s[..2], 16),
                    u8::from_str_radix(&s[2..4], 16),
                    u8::from_str_radix(&s[4..], 16),
                );

                if let (Ok(r), Ok(g), Ok(b)) = rgb {
                    return Ok(Self::Rgb(r, g, b));
                }
            }
        }

        Err(color_err(s).raise())
    }
}

impl TryFrom<&BStr> for Name {
    type Error = Error;

    fn try_from(s: &BStr) -> Result<Self, Self::Error> {
        Self::from_str(std::str::from_utf8(s).or_raise(|| color_err(s))?)
    }
}

bitflags::bitflags! {
    /// Discriminating enum for [`Color`] attributes.
    ///
    /// `git-config` supports modifiers and their negators. The negating color
    /// attributes are equivalent to having a `no` or `no-` prefix to the normal
    /// variant.
    #[derive(Default, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
    pub struct Attribute: u32 {
        /// Use bold or increased-intensity text.
        const BOLD = 1 << 1;
        /// Use dim or decreased-intensity text.
        const DIM = 1 << 2;
        /// Use italic text.
        const ITALIC = 1 << 3;
        /// Underline text.
        const UL = 1 << 4;
        /// Blink text.
        const BLINK = 1 << 5;
        /// Reverse the foreground and background colors.
        const REVERSE = 1 << 6;
        /// Strike through text.
        const STRIKE = 1 << 7;
        /// Parse the `reset` attribute, which Git otherwise leaves without an effect here.
        const RESET = 1 << 8;

        /// Disable dim text.
        const NO_DIM = 1 << 21;
        /// Disable bold text.
        const NO_BOLD = 1 << 22;
        /// Disable italic text.
        const NO_ITALIC = 1 << 23;
        /// Disable underlining.
        const NO_UL = 1 << 24;
        /// Disable blinking.
        const NO_BLINK = 1 << 25;
        /// Disable reversed colors.
        const NO_REVERSE = 1 << 26;
        /// Disable strikethrough.
        const NO_STRIKE = 1 << 27;
    }
}

impl Display for Attribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut write_space = None;
        for bit in 1..std::mem::size_of::<Attribute>() * 8 {
            let attr = match Attribute::from_bits(1 << bit) {
                Some(attr) => attr,
                None => continue,
            };
            if self.contains(attr) {
                if write_space.take().is_some() {
                    write!(f, " ")?;
                }
                match attr {
                    Attribute::RESET => write!(f, "reset"),
                    Attribute::BOLD => write!(f, "bold"),
                    Attribute::NO_BOLD => write!(f, "nobold"),
                    Attribute::DIM => write!(f, "dim"),
                    Attribute::NO_DIM => write!(f, "nodim"),
                    Attribute::UL => write!(f, "ul"),
                    Attribute::NO_UL => write!(f, "noul"),
                    Attribute::BLINK => write!(f, "blink"),
                    Attribute::NO_BLINK => write!(f, "noblink"),
                    Attribute::REVERSE => write!(f, "reverse"),
                    Attribute::NO_REVERSE => write!(f, "noreverse"),
                    Attribute::ITALIC => write!(f, "italic"),
                    Attribute::NO_ITALIC => write!(f, "noitalic"),
                    Attribute::STRIKE => write!(f, "strike"),
                    Attribute::NO_STRIKE => write!(f, "nostrike"),
                    _ => unreachable!("BUG: add new attribute flag"),
                }?;
                write_space = Some(());
            }
        }
        Ok(())
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Attribute {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl FromStr for Attribute {
    type Err = Error;

    fn from_str(mut s: &str) -> Result<Self, Self::Err> {
        let inverted = if let Some(rest) = s.strip_prefix("no-").or_else(|| s.strip_prefix("no")) {
            s = rest;
            true
        } else {
            false
        };

        if s.eq_ignore_ascii_case("reset") {
            return if inverted {
                Err(color_err(s).raise())
            } else {
                Ok(Attribute::RESET)
            };
        }

        match s {
            "bold" if !inverted => Ok(Attribute::BOLD),
            "bold" if inverted => Ok(Attribute::NO_BOLD),
            "dim" if !inverted => Ok(Attribute::DIM),
            "dim" if inverted => Ok(Attribute::NO_DIM),
            "ul" if !inverted => Ok(Attribute::UL),
            "ul" if inverted => Ok(Attribute::NO_UL),
            "blink" if !inverted => Ok(Attribute::BLINK),
            "blink" if inverted => Ok(Attribute::NO_BLINK),
            "reverse" if !inverted => Ok(Attribute::REVERSE),
            "reverse" if inverted => Ok(Attribute::NO_REVERSE),
            "italic" if !inverted => Ok(Attribute::ITALIC),
            "italic" if inverted => Ok(Attribute::NO_ITALIC),
            "strike" if !inverted => Ok(Attribute::STRIKE),
            "strike" if inverted => Ok(Attribute::NO_STRIKE),
            _ => Err(color_err(s).raise()),
        }
    }
}

impl TryFrom<&BStr> for Attribute {
    type Error = Error;

    fn try_from(s: &BStr) -> Result<Self, Self::Error> {
        Self::from_str(std::str::from_utf8(s).or_raise(|| color_err(s))?)
    }
}
