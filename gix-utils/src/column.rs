//! Arrange byte strings into display columns without changing their contents.

use std::io::{self, Write};
use unicode_width::UnicodeWidthChar;

/// Order in which input items fill the output.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Layout {
    /// Fill a column before moving to its right.
    #[default]
    Column,
    /// Fill a row before moving down.
    Row,
    /// Print one item per line.
    Plain,
}

/// Layout and separators for [`write()`].
#[derive(Clone, Copy, Debug)]
pub struct Options<'a> {
    /// Available display columns, including indentation and cell padding.
    pub width: usize,
    /// Spaces between cells.
    pub padding: usize,
    /// Bytes before the first cell of each row.
    pub indent: &'a [u8],
    /// Bytes after the last cell of each row.
    pub newline: &'a [u8],
    /// Input traversal order.
    pub layout: Layout,
    /// Allow each column to have its own width.
    pub dense: bool,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Self {
            width: 79,
            padding: 1,
            indent: b"",
            newline: b"\n",
            layout: Layout::Column,
            dense: false,
        }
    }
}

/// Write `items` using Git's column layout, preserving their bytes and propagating output errors.
///
/// Cell widths count Unicode display columns and ignore ANSI color sequences. Invalid UTF-8
/// is measured in bytes. Zero-width inputs and narrow output still produce at least one column.
pub fn write(items: &[&[u8]], options: Options<'_>, mut out: impl Write) -> io::Result<()> {
    if items.is_empty() {
        return Ok(());
    }
    if options.layout == Layout::Plain {
        for item in items {
            out.write_all(options.indent)?;
            out.write_all(item)?;
            out.write_all(options.newline)?;
        }
        return Ok(());
    }
    let lengths: Vec<_> = items.iter().map(|item| display_width(item)).collect();
    let cell_width = lengths
        .iter()
        .copied()
        .max()
        .unwrap_or(0)
        .saturating_add(options.padding)
        .max(1);
    let available = options.width.saturating_sub(options.indent.len());
    let mut columns = (available / cell_width).clamp(1, items.len());
    let mut rows = items.len().div_ceil(columns);
    let position = |row, column, rows, columns| match options.layout {
        Layout::Column => column * rows + row,
        Layout::Row | Layout::Plain => row * columns + column,
    };
    let widths_for = |rows, columns| -> Vec<usize> {
        (0..columns)
            .map(|column| {
                (0..rows)
                    .filter_map(|row| lengths.get(position(row, column, rows, columns)))
                    .copied()
                    .max()
                    .unwrap_or(0)
            })
            .collect()
    };
    if options.dense {
        while rows > 1 {
            let proposed_rows = rows - 1;
            let proposed_columns = items.len().div_ceil(proposed_rows);
            let required = widths_for(proposed_rows, proposed_columns)
                .into_iter()
                .fold(options.indent.len(), |total, width| {
                    total.saturating_add(width).saturating_add(options.padding)
                });
            if required > options.width {
                break;
            }
            rows = proposed_rows;
            columns = proposed_columns;
        }
    }
    let widths = if options.dense {
        widths_for(rows, columns)
    } else {
        vec![cell_width.saturating_sub(options.padding); columns]
    };
    for row in 0..rows {
        for (column, width) in widths.iter().enumerate() {
            let index = position(row, column, rows, columns);
            let Some(item) = items.get(index) else { break };
            if column == 0 {
                out.write_all(options.indent)?;
            }
            out.write_all(item)?;
            if column + 1 == columns || position(row, column + 1, rows, columns) >= items.len() {
                out.write_all(options.newline)?;
            } else {
                let mut spaces = width.saturating_sub(lengths[index]).saturating_add(options.padding);
                while spaces != 0 {
                    let count = spaces.min(64);
                    out.write_all(&[b' '; 64][..count])?;
                    spaces -= count;
                }
            }
        }
    }
    Ok(())
}

fn display_width(input: &[u8]) -> usize {
    let mut visible = Vec::with_capacity(input.len());
    let mut remaining = input;
    while let Some((&first, rest)) = remaining.split_first() {
        if first == 0x1b && rest.starts_with(b"[") {
            let parameters = &rest[1..];
            let count = parameters
                .iter()
                .take_while(|byte| byte.is_ascii_digit() || **byte == b';')
                .count();
            if parameters.get(count) == Some(&b'm') {
                remaining = &parameters[count + 1..];
                continue;
            }
        }
        visible.push(first);
        remaining = rest;
    }
    std::str::from_utf8(&visible).map_or(input.len(), |text| text.chars().map(|ch| ch.width().unwrap_or(0)).sum())
}
