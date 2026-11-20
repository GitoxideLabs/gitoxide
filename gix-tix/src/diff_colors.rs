use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::{
    app::ChangeKind,
    structural_diff::outline::{Change, Language},
};

#[derive(Clone, Copy)]
pub(crate) struct Palette {
    pub(crate) colored: bool,
    rows: [Style; 4],
    emphasized: [Style; 2],
    syntax: [[Color; 6]; 4],
    pub(crate) header: Style,
    pub(crate) metadata: Style,
    pub(crate) warning: Style,
    positive: Style,
    negative: Style,
    type_changed: Style,
}

impl Default for Palette {
    fn default() -> Self {
        let context = Style::default();
        let added = context.fg(Color::Green);
        let removed = context.fg(Color::Red);
        let modified = context.fg(Color::Yellow);
        Self {
            colored: true,
            rows: [context, added, removed, modified],
            emphasized: [added, removed].map(|style| style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED)),
            syntax: [[
                Color::Magenta,
                Color::Yellow,
                Color::Blue,
                Color::Green,
                Color::Cyan,
                Color::DarkGray,
            ]; 4],
            header: context.fg(Color::Blue).add_modifier(Modifier::BOLD),
            metadata: context.fg(Color::Cyan),
            warning: modified,
            positive: added,
            negative: removed,
            type_changed: context.fg(Color::Magenta),
        }
    }
}

impl Palette {
    pub(crate) const PLAIN: Self = {
        let plain = Style::new();
        Self {
            colored: false,
            rows: [plain; 4],
            emphasized: [plain; 2],
            syntax: [[Color::Reset; 6]; 4],
            header: plain.add_modifier(Modifier::BOLD),
            metadata: plain,
            warning: plain,
            positive: plain,
            negative: plain,
            type_changed: plain,
        }
    };

    pub(crate) fn new(foreground: [u8; 3], background: [u8; 3], ansi: Option<&[[u8; 3]; 16]>) -> Self {
        let foreground = readable(foreground, background, foreground);
        let source = |color| crate::kitty::rgb(color, foreground, ansi);
        let rgb = |[red, green, blue]: [u8; 3]| Color::Rgb(red, green, blue);
        let foreground_on = |color, background| rgb(readable(color, background, foreground));
        let text_on = |background| {
            Style::default()
                .fg(foreground_on(foreground, background))
                .bg(rgb(background))
        };
        let backgrounds = [
            background,
            blend(background, source(Color::Green), 12),
            blend(background, source(Color::Red), 12),
            blend(background, source(Color::Yellow), 7),
        ];
        let mut rows = backgrounds.map(text_on);
        rows[0].bg = None;
        let emphasized = [Color::Green, Color::Red]
            .map(|color| text_on(blend(background, source(color), 35)).add_modifier(Modifier::BOLD));
        let syntax = backgrounds.map(|background| {
            [
                Color::Magenta,
                Color::Yellow,
                Color::Blue,
                Color::Green,
                Color::Cyan,
                Color::DarkGray,
            ]
            .map(|color| foreground_on(source(color), background))
        });
        let flat = |color| Style::default().fg(foreground_on(source(color), background));
        let header_background = blend(background, source(Color::Blue), 9);
        Self {
            colored: true,
            rows,
            emphasized,
            syntax,
            header: text_on(header_background)
                .fg(foreground_on(source(Color::Blue), header_background))
                .add_modifier(Modifier::BOLD),
            metadata: flat(Color::Cyan),
            warning: flat(Color::Yellow),
            positive: flat(Color::Green),
            negative: flat(Color::Red),
            type_changed: flat(Color::Magenta),
        }
    }

    pub(crate) fn row(&self, change: Change) -> Style {
        self.rows[change_index(change)]
    }

    pub(crate) fn emphasis(&self, change: Change) -> Style {
        match change {
            Change::Added => self.emphasized[0],
            Change::Removed => self.emphasized[1],
            _ => self.row(change),
        }
    }

    pub(crate) fn path(&self, kind: ChangeKind) -> Style {
        match kind {
            ChangeKind::Added => self.positive,
            ChangeKind::Deleted => self.negative,
            ChangeKind::Modified => self.warning,
            ChangeKind::TypeChanged => self.type_changed,
            _ => self.metadata,
        }
    }

    // Highlight borrowed display text only for the visible page; cached rows keep no token strings.
    pub(crate) fn highlight<'a>(&self, line: &'a Line<'_>, language: Option<Language>) -> Line<'a> {
        let mut spans = Vec::new();
        let mut previous_word = "";
        let row = self.rows.iter().position(|row| *row == line.style).unwrap_or_default();
        // On ANSI-only terminals keep a changed row's foreground when there is no tinted background.
        let syntax_colors = self.colored && (row == 0 || line.style.bg.is_some());
        for span in &line.spans {
            let Some(language) =
                language.filter(|_| span.style.bg.is_none() && !span.style.add_modifier.contains(Modifier::UNDERLINED))
            else {
                previous_word = "";
                spans.push(Span::styled(span.content.as_ref(), span.style));
                continue;
            };
            if language == Language::Markdown {
                let mut style = span.style.add_modifier(Modifier::BOLD);
                if syntax_colors {
                    style = style.fg(self.syntax[row][Token::Name as usize]);
                }
                spans.push(Span::styled(span.content.as_ref(), style));
                continue;
            }
            let mut rest = span.content.as_ref();
            while !rest.is_empty() {
                let (mut length, kind) = token(rest, language, previous_word);
                if rest.as_bytes().get(length).is_some_and(|byte| !byte.is_ascii()) {
                    let source = Span::raw(rest);
                    let mut end = 0;
                    for grapheme in source.styled_graphemes(Style::default()) {
                        end += grapheme.symbol.len();
                        if end >= length {
                            length = end;
                            break;
                        }
                    }
                }
                let text = &rest[..length];
                let mut style = span.style;
                if let Some(kind) = kind {
                    if syntax_colors {
                        style = style.fg(self.syntax[row][kind as usize]);
                    }
                    if matches!(kind, Token::Keyword | Token::Name)
                        || (matches!(kind, Token::Type) && text.starts_with(char::is_uppercase))
                    {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                }
                if text.starts_with(|character: char| character.is_alphabetic() || character == '_') {
                    previous_word = text;
                }
                spans.push(Span::styled(text, style));
                rest = &rest[length..];
            }
        }
        Line::from(spans).style(line.style)
    }
}

fn change_index(change: Change) -> usize {
    match change {
        Change::Context => 0,
        Change::Added => 1,
        Change::Removed => 2,
        Change::Modified => 3,
    }
}

fn blend(background: [u8; 3], color: [u8; 3], percent: u16) -> [u8; 3] {
    std::array::from_fn(|index| {
        ((u16::from(background[index]) * (100 - percent) + u16::from(color[index]) * percent + 50) / 100) as u8
    })
}

fn luminance(color: [u8; 3]) -> f64 {
    let [red, green, blue] = color.map(|channel| {
        let channel = f64::from(channel) / 255.0;
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * red + 0.7152 * green + 0.0722 * blue
}

fn contrast(left: [u8; 3], right: [u8; 3]) -> f64 {
    let [left, right] = [left, right].map(luminance);
    (left.max(right) + 0.05) / (left.min(right) + 0.05)
}

fn readable(color: [u8; 3], background: [u8; 3], foreground: [u8; 3]) -> [u8; 3] {
    let target = if contrast(foreground, background) >= 4.5 {
        foreground
    } else if contrast([0; 3], background) >= contrast([255; 3], background) {
        [0; 3]
    } else {
        [255; 3]
    };
    for percent in (0..=100).step_by(5) {
        let candidate = blend(color, target, percent);
        if contrast(candidate, background) >= 4.5 {
            return candidate;
        }
    }
    target
}

#[derive(Clone, Copy)]
enum Token {
    Keyword,
    Type,
    Name,
    String,
    Number,
    Comment,
}

// shortcut: colors use declaration tokens; use a syntax parser if full source needs coloring.
fn token(text: &str, language: Language, previous_word: &str) -> (usize, Option<Token>) {
    let Some(first) = text.chars().next() else {
        return (0, None);
    };
    let python = language == Language::Python;
    if (python && first == '#') || (!python && text.starts_with("//")) {
        return (text.len(), Some(Token::Comment));
    }
    if !python && text.starts_with("/*") {
        return (text.find("*/").map_or(text.len(), |end| end + 2), Some(Token::Comment));
    }
    if language == Language::Toml {
        if first == '"' {
            return (quoted_length(text, first), Some(Token::Name));
        }
        if first == '\'' {
            return (
                text[1..].find('\'').map_or(text.len(), |end| end + 2),
                Some(Token::Name),
            );
        }
        let length = text
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
            .count();
        if length != 0 {
            return (length, Some(Token::Name));
        }
    }
    if first == '"' || (python && first == '\'') {
        return (quoted_length(text, first), Some(Token::String));
    }
    if first == '\'' {
        let following = &text[1..];
        if following.starts_with('\\')
            || following
                .chars()
                .next()
                .is_some_and(|character| following[character.len_utf8()..].starts_with('\''))
        {
            return (quoted_length(text, first), Some(Token::String));
        }
        let length = following
            .char_indices()
            .find(|(_, character)| !identifier(*character))
            .map_or(text.len(), |(index, _)| index + 1);
        return (length, Some(Token::Type));
    }
    if first.is_ascii_digit() {
        return (
            text.char_indices()
                .find(|(_, character)| !identifier(*character) && *character != '.')
                .map_or(text.len(), |(index, _)| index),
            Some(Token::Number),
        );
    }
    if first.is_alphabetic() || first == '_' {
        let name = text
            .strip_prefix("r#")
            .filter(|name| !python && name.starts_with(|ch: char| ch.is_alphabetic() || ch == '_'))
            .unwrap_or(text);
        let length = text.len() - name.len()
            + name
                .char_indices()
                .find(|(_, character)| !identifier(*character))
                .map_or(name.len(), |(index, _)| index);
        let word = &text[..length];
        let following = text[length..].trim_start();
        let rust_keyword = matches!(
            word,
            "Self"
                | "as"
                | "async"
                | "await"
                | "break"
                | "const"
                | "continue"
                | "crate"
                | "dyn"
                | "else"
                | "enum"
                | "extern"
                | "false"
                | "fn"
                | "for"
                | "if"
                | "impl"
                | "in"
                | "let"
                | "loop"
                | "macro"
                | "macro_rules"
                | "match"
                | "mod"
                | "move"
                | "mut"
                | "pub"
                | "ref"
                | "return"
                | "self"
                | "static"
                | "struct"
                | "super"
                | "trait"
                | "true"
                | "type"
                | "union"
                | "unsafe"
                | "use"
                | "where"
                | "while"
        );
        let keyword = if python {
            matches!(
                word,
                "False"
                    | "None"
                    | "True"
                    | "and"
                    | "as"
                    | "async"
                    | "await"
                    | "class"
                    | "def"
                    | "else"
                    | "for"
                    | "if"
                    | "in"
                    | "is"
                    | "lambda"
                    | "not"
                    | "or"
            )
        } else {
            rust_keyword
        };
        let kind = if keyword {
            Some(Token::Keyword)
        } else if word.starts_with("r#")
            || (python && matches!(previous_word, "class" | "def"))
            || matches!(
                previous_word,
                "fn" | "mod"
                    | "struct"
                    | "enum"
                    | "trait"
                    | "union"
                    | "type"
                    | "const"
                    | "static"
                    | "macro"
                    | "macro_rules"
            )
            || following.starts_with(['(', '!', '{', '='])
            || following.strip_prefix(':').is_some_and(|rest| !rest.starts_with(':'))
        {
            Some(Token::Name)
        } else if first.is_uppercase()
            || matches!(
                word,
                "bool"
                    | "char"
                    | "str"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "isize"
                    | "u8"
                    | "u16"
                    | "u32"
                    | "u64"
                    | "u128"
                    | "usize"
                    | "f32"
                    | "f64"
            )
        {
            Some(Token::Type)
        } else if previous_word.is_empty() && matches!(following, "" | ",") {
            Some(Token::Name)
        } else {
            None
        };
        return (length, kind);
    }
    (
        text.char_indices()
            .skip(1)
            .find(|(_, character)| {
                character.is_alphanumeric()
                    || matches!(character, '_' | '"' | '\'' | '/')
                    || (language == Language::Toml && *character == '-')
                    || (python && *character == '#')
            })
            .map_or(text.len(), |(index, _)| index),
        None,
    )
}

fn identifier(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

fn quoted_length(text: &str, quote: char) -> usize {
    let mut escaped = false;
    for (index, character) in text.char_indices().skip(1) {
        if escaped {
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == quote {
            return index + character.len_utf8();
        }
    }
    text.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(color: Color) -> [u8; 3] {
        match color {
            Color::Rgb(red, green, blue) => [red, green, blue],
            _ => panic!("detected palettes produce RGB colors"),
        }
    }

    #[test]
    fn dark_light_and_unusual_themes_keep_text_and_syntax_readable() {
        let mut ansi = [[120; 3]; 16];
        ansi[1] = [190, 30, 70];
        ansi[2] = [20, 160, 80];
        ansi[4] = [40, 70, 220];
        for (foreground, background) in [
            ([220; 3], [18, 22, 30]),
            ([25, 30, 35], [245, 242, 235]),
            ([120; 3], [120; 3]),
        ] {
            let palette = Palette::new(foreground, background, Some(&ansi));
            for (index, style) in palette.rows.into_iter().enumerate() {
                let background = style.bg.map_or(background, rgb);
                assert!(
                    contrast(rgb(style.fg.expect("sampled text colors are explicit")), background) >= 4.5,
                    "row text stays readable against every diff background"
                );
                for color in palette.syntax[index] {
                    assert!(
                        contrast(rgb(color), background) >= 4.5,
                        "syntax colors retain readable contrast"
                    );
                }
            }
            for style in palette.emphasized.into_iter().chain([palette.header]) {
                assert!(
                    contrast(
                        rgb(style.fg.expect("emphasis has a foreground")),
                        rgb(style.bg.expect("emphasis has a background"))
                    ) >= 4.5,
                    "word emphasis and headers preserve contrast"
                );
            }
            assert_ne!(
                palette.rows[1].bg, palette.rows[2].bg,
                "theme red and green produce different tints"
            );
            assert_eq!(
                palette.row(Change::Added).bg.map(rgb),
                Some(blend(background, ansi[2], 12)),
                "the row tint stays close to the terminal background"
            );
            assert_eq!(
                palette.emphasis(Change::Added).bg.map(rgb),
                Some(blend(background, ansi[2], 35)),
                "changed words are still more distinct than their row"
            );
            assert!(
                palette.rows[0].bg.is_none(),
                "unchanged rows preserve terminal background transparency"
            );
        }
    }

    #[test]
    fn names_and_keywords_are_bold_across_palettes() {
        for palette in [
            Palette::new([220; 3], [20; 3], None),
            Palette::default(),
            Palette::PLAIN,
        ] {
            for (source, language, bold) in [
                (
                    "pub fn r#type<T>(arg: Wrapper<T>) -> i32 { … }",
                    Language::Rust,
                    ["pub", "fn", "r#type", "arg", "Wrapper"].as_slice(),
                ),
                ("mod nested {", Language::Rust, &["mod", "nested"]),
                ("struct Data {", Language::Rust, &["struct", "Data"]),
                ("status: u8,", Language::Rust, &["status"]),
                ("class Service:", Language::Python, &["class", "Service"]),
                (
                    "async def fetch(path: str = 'local') -> str: …",
                    Language::Python,
                    &["async", "def", "fetch", "path"],
                ),
                ("[package.'quoted-key']", Language::Toml, &["package", "'quoted-key'"]),
                ("default-features = …", Language::Toml, &["default-features"]),
                ("# Project", Language::Markdown, &["# Project"]),
            ] {
                let line = Line::raw(source).style(palette.row(Change::Added));
                let highlighted = palette.highlight(&line, Some(language));
                assert_eq!(
                    highlighted.to_string(),
                    source,
                    "emphasis preserves the declaration text"
                );
                for name in bold {
                    assert!(
                        highlighted
                            .spans
                            .iter()
                            .any(|span| span.content == *name && span.style.add_modifier.contains(Modifier::BOLD)),
                        "{name} is bold in {source}"
                    );
                }
                if line.style.bg.is_none() {
                    assert!(
                        highlighted.style == line.style && highlighted.spans.iter().all(|span| span.style.fg.is_none()),
                        "the fallback keeps the row's foreground across tokens"
                    );
                }
            }
        }
    }

    #[test]
    fn syntax_preserves_unicode_lifetimes_strings_and_emphasized_words() {
        let palette = Palette::new([220; 3], [20; 3], None);
        let input = "pub fn café<'a>(arg: &'a str) -> [u32; 8] { … }";
        let line = Line::from(vec![Span::raw("~"), Span::raw(input)]).style(palette.row(Change::Modified));
        let highlighted = palette.highlight(&line, Some(Language::Rust));
        assert_eq!(
            highlighted.to_string(),
            format!("~{input}"),
            "coloring never changes visible source text"
        );
        for token in ["pub", "fn", "café", "'a", "str", "u32", "8"] {
            assert!(
                highlighted
                    .spans
                    .iter()
                    .any(|span| span.content == token && span.style.fg.is_some()),
                "{token} has a syntax color"
            );
        }
        let emphasis = Span::styled("{+u32+}", palette.emphasis(Change::Added));
        let line = Line::from(vec![Span::raw("~"), emphasis.clone()]).style(palette.row(Change::Modified));
        let highlighted = palette.highlight(&line, Some(Language::Rust));
        assert_eq!(
            highlighted.spans[1], emphasis,
            "word emphasis takes precedence over syntax"
        );
        for (text, language) in [
            ("name = …", Language::Toml),
            ("# Heading {+added+}", Language::Markdown),
            ("#[cfg(feature = \"a\\\"b\")] fn f() { … }", Language::Rust),
        ] {
            let line = Line::from(vec![Span::raw("+"), Span::raw(text)]);
            assert_eq!(
                palette.highlight(&line, Some(language)).to_string(),
                format!("+{text}"),
                "all supported languages keep the displayed declaration intact"
            );
        }
    }

    #[test]
    fn syntax_coloring_keeps_combining_marks_attached_to_their_rendered_character() -> gix::error::TestResult {
        use ratatui::{Terminal, backend::TestBackend, widgets::Paragraph};

        let palette = Palette::new([220; 3], [20; 3], None);
        let line = Line::from(vec![Span::raw("~"), Span::raw("fn cafe\u{301}() { … }")]);
        let highlighted = palette.highlight(&line, Some(Language::Rust));
        let mut terminal = Terminal::new(TestBackend::new(40, 1))?;
        terminal.draw(|frame| frame.render_widget(Paragraph::new(highlighted.clone()), frame.area()))?;
        assert_eq!(
            terminal.backend().buffer()[(7, 0)].symbol(),
            "e\u{301}",
            "color boundaries must not detach combining marks from their base character"
        );
        Ok(())
    }

    #[test]
    fn missing_terminal_replies_keep_ansi_colors_without_guessing_a_background() {
        let palette = Palette::default();
        assert!(
            palette.rows.iter().all(|style| style.bg.is_none()),
            "unknown themes retain terminal backgrounds"
        );
        assert_eq!(palette.path(ChangeKind::Added).fg, Some(Color::Green));
        assert_eq!(palette.path(ChangeKind::Deleted).fg, Some(Color::Red));
        assert!(
            palette
                .emphasis(Change::Added)
                .add_modifier
                .contains(Modifier::BOLD | Modifier::UNDERLINED),
            "word changes remain recognizable without RGB replies"
        );
    }
}
