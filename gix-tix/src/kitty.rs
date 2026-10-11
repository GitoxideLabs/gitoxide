use std::{
    collections::{HashMap, HashSet, hash_map::RandomState},
    hash::BuildHasher,
    io::{self, IsTerminal, Write},
};

use ratatui::{
    backend::{Backend as _, ClearType, CrosstermBackend, WindowSize},
    buffer::{Cell, CellDiffOption, CellWidth},
    layout::{Position, Size},
    style::{Color, Modifier},
};

use crate::graph_tiles::Tile;

const MAX_TILES: usize = 1024;
static OWNED_IMAGES: gix::parallel::Mutable<Vec<u32>> = gix::parallel::Mutable::new(Vec::new());

/// Delete only this process's allocations, including a partially transmitted image on panic.
pub(crate) fn cleanup_process_images(mut writer: impl Write) -> io::Result<()> {
    let numbers = gix::parallel::lock(&OWNED_IMAGES).clone();
    for number in numbers {
        delete_image(&mut writer, number)?;
        gix::parallel::lock(&OWNED_IMAGES).retain(|owned| *owned != number);
    }
    writer.flush()
}

pub(crate) fn supported() -> bool {
    // shortcut: conservative detection avoids APC replies becoming crossterm key events; use a query after it supports APC.
    std::io::stdout().is_terminal()
        && std::env::var_os("TMUX").is_none()
        && std::env::var_os("STY").is_none()
        && known_terminal(
            &std::env::var("TERM").unwrap_or_default(),
            &std::env::var("TERM_PROGRAM").unwrap_or_default(),
        )
        && cell_size().is_some()
}

fn known_terminal(term: &str, program: &str) -> bool {
    !matches!(term, "dumb" | "")
        && !term.starts_with("screen")
        && !term.starts_with("tmux")
        && (term == "xterm-kitty" || matches!(program, "kitty" | "WezTerm" | "ghostty"))
}

fn cell_size() -> Option<(u16, u16)> {
    let size = crossterm::terminal::window_size().ok()?;
    let width = size.width.checked_div(size.columns)?;
    let height = size.height.checked_div(size.rows)?;
    ((1..=128).contains(&width) && (1..=256).contains(&height)).then_some((width, height))
}

fn decode_tile(cell: &Cell) -> Option<Tile> {
    if matches!(cell.diff_option, CellDiffOption::ForcedWidth(width) if width.get() == 1) {
        Tile::decode(cell.symbol())
    } else {
        None
    }
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct TileKey {
    symbol: String,
    foreground: Color,
    background: Color,
    modifier: Modifier,
    size: (u16, u16),
}

pub(crate) struct Backend<W: Write> {
    inner: CrosstermBackend<W>,
    enabled: bool,
    size: Option<(u16, u16)>,
    foreground: [u8; 3],
    background: [u8; 3],
    tiles: HashMap<TileKey, u32>,
    placements: HashMap<(u16, u16), u32>,
    placement_counts: HashMap<u32, usize>,
    graph_cells: HashMap<(u16, u16), Cell>,
    next_number: u32,
}

impl<W: Write> Backend<W> {
    pub(crate) fn new(writer: W, enabled: bool) -> Self {
        Self {
            inner: CrosstermBackend::new(writer),
            enabled,
            size: None,
            foreground: [229, 229, 229],
            background: [0, 0, 0],
            tiles: HashMap::new(),
            placements: HashMap::new(),
            placement_counts: HashMap::new(),
            graph_cells: HashMap::new(),
            next_number: RandomState::new().hash_one(std::process::id()) as u32,
        }
    }

    pub(crate) fn set_default_colors(&mut self, foreground: [u8; 3], background: [u8; 3]) {
        self.foreground = foreground;
        self.background = background;
    }

    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn cleanup(&mut self) -> io::Result<()> {
        self.delete_images()?;
        self.graph_cells.clear();
        Write::flush(&mut self.inner)
    }

    pub(crate) fn cleanup_with_fallback(&mut self) -> io::Result<()> {
        let cells: Vec<_> = self
            .graph_cells
            .iter()
            .map(|(&(x, y), cell)| {
                let mut cell = cell.clone();
                if let Some(tile) = decode_tile(&cell) {
                    cell.set_char(tile.fallback());
                }
                (x, y, cell)
            })
            .collect();
        self.delete_images()?;
        if !cells.is_empty() {
            write!(self.inner, "\x1b7")?;
            let draw = self.inner.draw(cells.iter().map(|(x, y, cell)| (*x, *y, cell)));
            let restore = write!(self.inner, "\x1b8");
            draw.and(restore)?;
        }
        self.graph_cells.clear();
        Write::flush(&mut self.inner)
    }

    fn delete_images(&mut self) -> io::Result<()> {
        for number in self.tiles.values().copied() {
            delete_image(&mut self.inner, number)?;
            gix::parallel::lock(&OWNED_IMAGES).retain(|owned| *owned != number);
        }
        self.tiles.clear();
        self.placements.clear();
        self.placement_counts.clear();
        Ok(())
    }

    fn remove_placement(&mut self, x: u16, y: u16) -> io::Result<()> {
        if let Some(number) = self.placements.remove(&(x, y)) {
            let count = self
                .placement_counts
                .get_mut(&number)
                .expect("every placement keeps its cached image in use");
            *count -= 1;
            if *count == 0 {
                self.placement_counts.remove(&number);
            }
            write!(
                self.inner,
                "\x1b_Ga=d,d=n,I={number},p={},q=2\x1b\\",
                placement_id(x, y)
            )?;
        }
        Ok(())
    }

    fn image(&mut self, cell: &Cell, tile: &Tile, size: (u16, u16)) -> io::Result<Option<u32>> {
        let key = TileKey {
            symbol: cell.symbol().to_owned(),
            foreground: cell.fg,
            background: cell.bg,
            modifier: cell.modifier,
            size,
        };
        if let Some(number) = self.tiles.get(&key) {
            return Ok(Some(*number));
        }
        if self.tiles.len() >= MAX_TILES {
            let unused = if self.placement_counts.len() == self.tiles.len() {
                None
            } else {
                self.tiles
                    .iter()
                    .find(|(_, number)| !self.placement_counts.contains_key(number))
                    .map(|(key, number)| (key.clone(), *number))
            };
            let Some((unused, number)) = unused else {
                // shortcut: more than 1024 visible tile variants use text; raise the cap if real graphs need more.
                return Ok(None);
            };
            delete_image(&mut self.inner, number)?;
            self.tiles.remove(&unused);
            gix::parallel::lock(&OWNED_IMAGES).retain(|owned| *owned != number);
        }
        let mut foreground = rgb(cell.fg, self.foreground);
        let mut background = rgb(cell.bg, self.background);
        if cell.modifier.contains(Modifier::REVERSED) {
            std::mem::swap(&mut foreground, &mut background);
        }
        if cell.modifier.contains(Modifier::DIM) {
            foreground = std::array::from_fn(|index| u8::midpoint(foreground[index], background[index]));
        }
        let pixels = tile.rasterize(size.0, size.1, foreground, background);
        let number = {
            let mut owned = gix::parallel::lock(&OWNED_IMAGES);
            loop {
                self.next_number = self.next_number.wrapping_add(1).max(1);
                if !owned.contains(&self.next_number) {
                    owned.push(self.next_number);
                    break self.next_number;
                }
            }
        };
        // Image numbers allocate fresh storage even when another application chose the same number.
        self.tiles.insert(key, number);
        upload(&mut self.inner, number, size, &pixels)?;
        Ok(Some(number))
    }

    fn draw_with_size<'a>(
        &mut self,
        content: impl Iterator<Item = (u16, u16, &'a Cell)>,
        size: Option<(u16, u16)>,
    ) -> io::Result<()> {
        let size = size.filter(|_| self.enabled);
        let resized = self.size != size;
        if resized {
            self.delete_images()?;
            self.size = size;
        }
        let mut cells: Vec<_> = content.map(|(x, y, cell)| (x, y, cell.clone())).collect();
        for (x, y, cell) in &cells {
            if decode_tile(cell).is_some() {
                if self.graph_cells.get(&(*x, *y)) != Some(cell) {
                    self.remove_placement(*x, *y)?;
                }
                self.graph_cells.insert((*x, *y), cell.clone());
            } else {
                for covered_x in *x..x.saturating_add(cell.cell_width().max(1)) {
                    self.graph_cells.remove(&(covered_x, *y));
                    self.remove_placement(covered_x, *y)?;
                }
            }
        }
        if resized {
            let updated: HashSet<_> = cells.iter().map(|(x, y, _)| (*x, *y)).collect();
            cells.extend(
                self.graph_cells
                    .iter()
                    .filter(|(position, _)| !updated.contains(position))
                    .map(|(&(x, y), cell)| (x, y, cell.clone())),
            );
        }
        let mut images = Vec::new();
        for (x, y, cell) in &mut cells {
            let image = if let Some(tile) = decode_tile(cell) {
                let number = match size {
                    Some(size) => self.image(cell, &tile, size)?,
                    None => None,
                };
                cell.set_char(if number.is_some() { tile.text() } else { tile.fallback() });
                number
            } else {
                None
            };
            if image != self.placements.get(&(*x, *y)).copied() {
                self.remove_placement(*x, *y)?;
                if let Some(number) = image {
                    self.placements.insert((*x, *y), number);
                    *self.placement_counts.entry(number).or_default() += 1;
                    images.push((*x, *y, number));
                }
            }
        }
        self.inner.draw(cells.iter().map(|(x, y, cell)| (*x, *y, cell)))?;
        if !images.is_empty() {
            write!(self.inner, "\x1b7")?;
            for (x, y, number) in images {
                write!(
                    self.inner,
                    "\x1b[{};{}H\x1b_Ga=p,I={number},p={},c=1,r=1,C=1,z=-1,q=2\x1b\\",
                    u32::from(y) + 1,
                    u32::from(x) + 1,
                    placement_id(x, y)
                )?;
            }
            write!(self.inner, "\x1b8")?;
        }
        Ok(())
    }
}

impl<W: Write> Drop for Backend<W> {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

impl<W: Write> Write for Backend<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Write::flush(&mut self.inner)
    }
}

impl<W: Write> ratatui::backend::Backend for Backend<W> {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        self.draw_with_size(content, self.enabled.then(cell_size).flatten())
    }

    fn append_lines(&mut self, n: u16) -> io::Result<()> {
        self.cleanup()?;
        self.inner.append_lines(n)
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.inner.hide_cursor()
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.inner.show_cursor()
    }

    fn get_cursor_position(&mut self) -> io::Result<Position> {
        self.inner.get_cursor_position()
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        self.inner.set_cursor_position(position)
    }

    fn clear(&mut self) -> io::Result<()> {
        self.cleanup()?;
        self.inner.clear()
    }

    fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
        if clear_type == ClearType::All {
            return self.clear();
        }
        if !self.graph_cells.is_empty() {
            let cursor = self.inner.get_cursor_position()?;
            let cleared: Vec<_> = self
                .graph_cells
                .keys()
                .copied()
                .filter(|&(x, y)| match clear_type {
                    ClearType::AfterCursor => (y, x) >= (cursor.y, cursor.x),
                    ClearType::BeforeCursor => (y, x) <= (cursor.y, cursor.x),
                    ClearType::CurrentLine => y == cursor.y,
                    ClearType::UntilNewLine => y == cursor.y && x >= cursor.x,
                    ClearType::All => true,
                })
                .collect();
            for (x, y) in cleared {
                self.remove_placement(x, y)?;
                self.graph_cells.remove(&(x, y));
            }
        }
        self.inner.clear_region(clear_type)
    }

    fn size(&self) -> io::Result<Size> {
        self.inner.size()
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        self.inner.window_size()
    }

    fn flush(&mut self) -> io::Result<()> {
        Write::flush(&mut self.inner)
    }
}

fn placement_id(x: u16, y: u16) -> u32 {
    ((u32::from(y) << 16) | u32::from(x)).saturating_add(1)
}

fn delete_image(writer: &mut impl Write, number: u32) -> io::Result<()> {
    write!(writer, "\x1b_Ga=d,d=N,I={number},q=2\x1b\\")
}

fn upload(writer: &mut impl Write, number: u32, size: (u16, u16), pixels: &[u8]) -> io::Result<()> {
    let encoded = base64(pixels);
    let chunks = encoded.as_bytes().chunks(4096);
    let count = chunks.len();
    for (index, chunk) in chunks.enumerate() {
        let more = u8::from(index + 1 != count);
        if index == 0 {
            write!(
                writer,
                "\x1b_Ga=t,t=d,f=32,I={number},s={},v={},q=2,m={more};",
                size.0, size.1
            )?;
        } else {
            write!(writer, "\x1b_Gm={more},q=2;")?;
        }
        writer.write_all(chunk)?;
        writer.write_all(b"\x1b\\")?;
    }
    Ok(())
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or_default();
        let third = chunk.get(2).copied().unwrap_or_default();
        encoded.push(ALPHABET[usize::from(first >> 2)] as char);
        encoded.push(ALPHABET[usize::from(((first & 3) << 4) | (second >> 4))] as char);
        encoded.push(if chunk.len() > 1 {
            ALPHABET[usize::from(((second & 15) << 2) | (third >> 6))] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            ALPHABET[usize::from(third & 63)] as char
        } else {
            '='
        });
    }
    encoded
}

fn rgb(color: Color, default: [u8; 3]) -> [u8; 3] {
    match color {
        Color::Reset => default,
        Color::Rgb(red, green, blue) => [red, green, blue],
        Color::Indexed(index @ 16..=231) => {
            let value = index - 16;
            let component = |value| if value == 0 { 0 } else { 55 + 40 * value };
            [component(value / 36), component(value / 6 % 6), component(value % 6)]
        }
        Color::Indexed(index @ 232..=255) => [8 + 10 * (index - 232); 3],
        color => {
            let index = match color {
                Color::Black => 0,
                Color::Red => 1,
                Color::Green => 2,
                Color::Yellow => 3,
                Color::Blue => 4,
                Color::Magenta => 5,
                Color::Cyan => 6,
                Color::Gray => 7,
                Color::DarkGray => 8,
                Color::LightRed => 9,
                Color::LightGreen => 10,
                Color::LightYellow => 11,
                Color::LightBlue => 12,
                Color::LightMagenta => 13,
                Color::LightCyan => 14,
                Color::White => 15,
                Color::Indexed(index) => usize::from(index),
                _ => return default,
            };
            const ANSI: [[u8; 3]; 16] = [
                [0, 0, 0],
                [205, 0, 0],
                [0, 205, 0],
                [205, 205, 0],
                [0, 0, 238],
                [205, 0, 205],
                [0, 205, 205],
                [229, 229, 229],
                [127, 127, 127],
                [255, 0, 0],
                [0, 255, 0],
                [255, 255, 0],
                [92, 92, 255],
                [255, 0, 255],
                [0, 255, 255],
                [255, 255, 255],
            ];
            ANSI[index]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Default)]
    struct Output(gix::parallel::OwnShared<gix::parallel::Mutable<Vec<u8>>>);

    impl Output {
        fn text(&self) -> String {
            String::from_utf8(gix::parallel::lock(&self.0).clone()).expect("terminal commands are UTF-8")
        }

        fn reset(&self) {
            gix::parallel::lock(&self.0).clear();
        }
    }

    impl Write for Output {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            gix::parallel::lock(&self.0).extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn graph_cell() -> Cell {
        let mut cell = Cell::default();
        cell.set_symbol(
            &Tile {
                paths: Tile::NODE_UP | Tile::NODE_DOWN,
                marker: '●',
                ..Tile::default()
            }
            .encode()
            .to_string(),
        );
        cell.fg = Color::Blue;
        cell.set_diff_option(CellDiffOption::ForcedWidth(std::num::NonZeroU16::MIN));
        cell
    }

    #[test]
    fn disabled_backend_preserves_graph_text_without_graphics_commands() -> gix_error::TestResult {
        let output = Output::default();
        let mut backend = Backend::new(output.clone(), false);
        let cell = graph_cell();
        backend.draw_with_size([(0, 0, &cell)].into_iter(), Some((8, 16)))?;
        backend.cleanup()?;
        assert!(
            output.text().contains('●'),
            "unsupported terminals receive the original commit marker"
        );
        assert!(!output.text().contains("\x1b_G"), "fallback emits no graphics protocol");
        assert!(
            !output.text().contains(cell.symbol()),
            "private-use transport symbols never reach the terminal"
        );
        Ok(())
    }

    #[test]
    fn conservative_detection_rejects_unknown_and_multiplexed_terminals() {
        assert!(
            known_terminal("xterm-kitty", ""),
            "Kitty advertises its graphics protocol"
        );
        assert!(
            known_terminal("xterm-256color", "ghostty"),
            "Ghostty supports Kitty graphics"
        );
        for (term, program) in [
            ("dumb", "ghostty"),
            ("screen-256color", "kitty"),
            ("tmux-256color", "WezTerm"),
            ("xterm-256color", "Apple_Terminal"),
        ] {
            assert!(
                !known_terminal(term, program),
                "ambiguous terminal capabilities preserve the text graph"
            );
        }
    }

    #[test]
    fn ordinary_private_use_text_is_not_interpreted_as_geometry() -> gix_error::TestResult {
        let output = Output::default();
        let mut backend = Backend::new(output.clone(), true);
        let mut ordinary = graph_cell();
        ordinary.set_diff_option(CellDiffOption::None);
        backend.draw_with_size([(0, 0, &ordinary)].into_iter(), Some((8, 16)))?;
        assert!(
            output.text().contains(ordinary.symbol()),
            "user-provided private-use text remains text"
        );
        assert!(
            !output.text().contains("\x1b_G"),
            "only explicitly marked graph cells allocate images"
        );
        assert!(
            backend.graph_cells.is_empty(),
            "ordinary text does not enter the renderer's graph state"
        );
        Ok(())
    }

    #[test]
    fn tiles_are_reused_and_overlays_delete_only_the_covered_placement() -> gix_error::TestResult {
        let output = Output::default();
        let mut backend = Backend::new(output.clone(), true);
        let cell = graph_cell();
        backend.draw_with_size([(0, 0, &cell), (0, 1, &cell)].into_iter(), Some((8, 16)))?;
        let first = output.text();
        assert_eq!(
            first.matches("a=t,").count(),
            1,
            "identical cells share one uploaded tile"
        );
        assert_eq!(first.matches("a=p,").count(), 2, "each cell owns a separate placement");
        assert!(
            first.contains("C=1,z=-1,q=2"),
            "placements preserve the cursor and suppress input responses"
        );

        output.reset();
        backend.draw_with_size([(0, 0, &cell), (0, 1, &cell)].into_iter(), Some((8, 16)))?;
        assert!(
            !output.text().contains("\x1b_G"),
            "unchanged cells need no graphics traffic"
        );

        backend.draw_with_size([(0, 0, &Cell::new("menu"))].into_iter(), Some((8, 16)))?;
        assert_eq!(
            output.text().matches("a=d,d=n,").count(),
            1,
            "an overlay removes its covered placement"
        );
        assert_eq!(backend.placements.len(), 1, "the other commit remains visible");

        output.reset();
        backend.cleanup()?;
        backend.cleanup()?;
        assert_eq!(
            output.text().matches("a=d,d=N,").count(),
            1,
            "closing frees uploaded data exactly once"
        );
        assert!(
            !output.text().contains("d=A"),
            "cleanup never targets another application's images"
        );
        assert!(
            backend.placement_counts.is_empty(),
            "cleanup releases every image reference"
        );
        Ok(())
    }

    #[test]
    fn full_cache_reuses_images_after_their_last_placement_is_replaced() -> gix_error::TestResult {
        let output = Output::default();
        let mut backend = Backend::new(output.clone(), true);
        let cells: Vec<_> = (0..MAX_TILES)
            .map(|index| {
                let mut cell = graph_cell();
                cell.fg = Color::Rgb(index as u8, (index / 256) as u8, 0);
                cell
            })
            .collect();
        backend.draw_with_size(
            (0..MAX_TILES * 2).map(|index| ((index % 64) as u16, (index / 64) as u16, &cells[index % MAX_TILES])),
            Some((1, 1)),
        )?;
        let old_number = backend.placements[&(0, 0)];
        assert_eq!(backend.tiles.len(), MAX_TILES, "the viewport fills the bounded cache");
        assert_eq!(
            backend.placements.len(),
            MAX_TILES * 2,
            "each cached image is shared by two placements"
        );
        assert_eq!(
            backend.placement_counts.get(&old_number),
            Some(&2),
            "both visible uses keep the image alive"
        );

        let mut replacement = graph_cell();
        replacement.fg = Color::Rgb(0, 0, 1);
        backend.draw_with_size([(0, 0, &Cell::new(" "))].into_iter(), Some((1, 1)))?;
        assert_eq!(
            backend.placement_counts.get(&old_number),
            Some(&1),
            "an overlay releases only the covered use of a shared image"
        );
        output.reset();
        backend.draw_with_size([(64, 0, &replacement)].into_iter(), Some((1, 1)))?;
        assert!(
            !output.text().contains("\x1b_G"),
            "a full cache falls back to text while every image still has a visible placement"
        );

        output.reset();
        backend.draw_with_size([(0, 16, &replacement)].into_iter(), Some((1, 1)))?;
        assert!(
            backend.placements.contains_key(&(0, 16)),
            "replacing the last placement makes its image available within the same frame"
        );
        assert!(
            output.text().contains(&format!("a=d,d=N,I={old_number},")),
            "only the image whose last placement disappeared is evicted"
        );
        assert_eq!(
            backend.tiles.len(),
            MAX_TILES,
            "replacement keeps the image cache bounded"
        );
        assert_eq!(
            backend.placements.len(),
            MAX_TILES * 2 - 1,
            "all other graph placements remain visible"
        );

        output.reset();
        backend.draw_with_size([(64, 0, &replacement)].into_iter(), Some((1, 1)))?;
        assert!(!output.text().contains("a=t,"), "the replacement image is shared too");
        assert_eq!(
            backend.placements[&(64, 0)],
            backend.placements[&(0, 16)],
            "both replacement cells refer to the same cached image"
        );
        assert_eq!(
            backend.placement_counts[&backend.placements[&(64, 0)]],
            2,
            "sharing the replacement records both live uses"
        );
        Ok(())
    }

    #[test]
    fn pixel_resize_rebuilds_all_visible_tiles_and_missing_geometry_restores_text() -> gix_error::TestResult {
        let output = Output::default();
        let mut backend = Backend::new(output.clone(), true);
        let cell = graph_cell();
        backend.draw_with_size([(0, 0, &cell), (0, 1, &cell)].into_iter(), Some((8, 16)))?;
        output.reset();
        backend.draw_with_size(std::iter::empty(), Some((12, 24)))?;
        let resized = output.text();
        assert!(
            resized.contains("s=12,v=24"),
            "font zoom recalibrates raster dimensions"
        );
        assert_eq!(
            resized.matches("a=p,").count(),
            2,
            "unchanged graph cells are restored after font zoom"
        );

        output.reset();
        backend.draw_with_size(std::iter::empty(), None)?;
        let fallback = output.text();
        assert_eq!(
            fallback.matches('●').count(),
            2,
            "losing pixel dimensions restores readable commit markers"
        );
        assert!(!fallback.contains("a=p,"), "missing dimensions disable placements");
        assert!(
            backend.placement_counts.is_empty(),
            "losing pixel dimensions releases all image references"
        );
        Ok(())
    }

    #[test]
    fn wide_overlay_removes_the_graph_beneath_its_unreported_trailing_cell() -> gix_error::TestResult {
        let output = Output::default();
        let mut backend = Backend::new(output.clone(), true);
        let cell = graph_cell();
        backend.draw_with_size([(0, 0, &cell), (1, 0, &cell)].into_iter(), Some((8, 16)))?;
        output.reset();
        backend.draw_with_size([(0, 0, &Cell::new("界"))].into_iter(), Some((8, 16)))?;
        assert_eq!(
            output.text().matches("a=d,d=n,").count(),
            2,
            "the wide glyph covers both graph placements"
        );
        assert!(
            backend.placements.is_empty() && backend.graph_cells.is_empty(),
            "covered graph cells cannot return after a pixel resize"
        );
        output.reset();
        backend.draw_with_size(std::iter::empty(), Some((12, 24)))?;
        assert!(
            !output.text().contains("a=p,"),
            "font zoom leaves the overlay untouched"
        );
        Ok(())
    }

    #[test]
    fn inline_cleanup_keeps_readable_history_and_restores_the_cursor() -> gix_error::TestResult {
        let output = Output::default();
        let mut backend = Backend::new(output.clone(), true);
        let cell = graph_cell();
        backend.draw_with_size([(0, 0, &cell), (0, 1, &cell)].into_iter(), Some((8, 16)))?;
        output.reset();
        backend.cleanup_with_fallback()?;
        let closed = output.text();
        assert_eq!(
            closed.matches('●').count(),
            2,
            "inline history keeps its commit markers after image cleanup"
        );
        assert_eq!(
            closed.matches("a=d,d=N,").count(),
            1,
            "inline exit releases tile pixels"
        );
        assert!(
            closed.contains("\x1b7") && closed.ends_with("\x1b8"),
            "cleanup preserves the caller's cursor"
        );
        assert!(
            backend.tiles.is_empty() && backend.graph_cells.is_empty(),
            "cleanup leaves no renderer resources"
        );
        Ok(())
    }

    #[test]
    fn upload_chunks_and_base64_match_the_protocol() -> gix_error::TestResult {
        for (bytes, encoded) in [
            (b"".as_slice(), ""),
            (b"f", "Zg=="),
            (b"fo", "Zm8="),
            (b"foo", "Zm9v"),
            (b"foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(bytes), encoded, "base64 padding follows RFC 4648");
        }
        let mut output = Vec::new();
        upload(&mut output, 123, (64, 64), &vec![255; 64 * 64 * 4])?;
        let output = String::from_utf8(output)?;
        let commands: Vec<_> = output.split("\x1b\\").filter(|command| !command.is_empty()).collect();
        assert!(commands.len() > 1, "a large tile exercises continuation chunks");
        let mut payload = String::new();
        for (index, command) in commands.iter().enumerate() {
            let (control, data) = command.split_once(';').expect("each upload chunk has a payload");
            assert!(
                data.len() <= 4096 && data.len() % 4 == 0,
                "encoded chunks fit the protocol limit"
            );
            assert!(control.contains("q=2"), "all chunks suppress responses");
            assert!(
                control.contains(if index + 1 == commands.len() { "m=0" } else { "m=1" }),
                "only the final chunk terminates the upload"
            );
            if index != 0 {
                assert!(
                    !control.contains("I="),
                    "continuation chunks carry no new image identity"
                );
            }
            payload.push_str(data);
        }
        assert_eq!(
            payload,
            base64(&vec![255; 64 * 64 * 4]),
            "chunking preserves the complete pixels"
        );
        Ok(())
    }
}
