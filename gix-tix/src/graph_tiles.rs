//! Cell-sized graph geometry, carried through Ratatui as private-use symbols.

// Bright cyan remains exclusive to topology previews.
pub(crate) const ROUTE_COLORS: [[u8; 3]; 8] = [
    [230, 100, 220],
    [230, 200, 70],
    [100, 160, 255],
    [85, 205, 115],
    [255, 150, 75],
    [255, 100, 110],
    [190, 150, 255],
    [215, 215, 215],
];

pub(crate) const SUBLANES_PER_CELL: i32 = 3;
// Ninths of a column also represent third-cell offsets over a three-quarter-row straight run.
const DIRECTION_SCALE: i32 = SUBLANES_PER_CELL * 3;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Tile {
    pub paths: u16,
    pub highlighted_paths: u16,
    pub path_colors: [u8; 10],
    pub marker: char,
    // Incoming NODE_UP direction, in the same units as Edge::incoming.
    pub incoming: Option<i32>,
    pub edges: Vec<Edge>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Edge {
    pub from: i32,
    pub to: i32,
    // Offsets from the integer column center, in thirds of a cell; they can span multiple cells.
    pub from_offset: i32,
    pub to_offset: i32,
    pub from_node: bool,
    pub to_node: bool,
    pub start_vertical: bool,
    pub end_vertical: bool,
    pub highlighted: bool,
    pub color: u8,
    // Neighboring horizontal movement over a full row, in 1/DIRECTION_SCALE columns.
    pub incoming: Option<i32>,
    pub outgoing: Option<i32>,
}

pub(crate) fn route_edges(current: usize, edges: &[(usize, usize)]) -> Vec<Edge> {
    edges
        .iter()
        .map(|&(from, to)| Edge {
            from: (from * 2) as i32,
            to: (to * 2) as i32,
            from_offset: 0,
            to_offset: 0,
            from_node: from == current,
            to_node: false,
            start_vertical: false,
            end_vertical: false,
            highlighted: false,
            color: 0,
            incoming: None,
            outgoing: None,
        })
        .collect()
}

impl Edge {
    fn start(&self) -> i64 {
        i64::from(self.from) * i64::from(SUBLANES_PER_CELL) + i64::from(self.from_offset)
    }

    fn end(&self) -> i64 {
        i64::from(self.to) * i64::from(SUBLANES_PER_CELL) + i64::from(self.to_offset)
    }

    fn delta(&self) -> Option<i32> {
        (self.end() - self.start()).try_into().ok()
    }

    /// Cells touched by this route, including room for strokes and chamfers at fractional endpoints.
    pub(crate) fn columns(&self) -> std::ops::RangeInclusive<i32> {
        let scale = i64::from(SUBLANES_PER_CELL);
        let from = self.start().min(self.end()).div_euclid(scale);
        let to = (self.start().max(self.end()) + scale - 1).div_euclid(scale);
        let clamp = |value: i64| value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        clamp(from)..=clamp(to)
    }

    fn points(&self, width: f32, height: f32) -> Vec<(f32, f32)> {
        let start = (
            (self.from as f32 + 0.5 + self.from_offset as f32 / SUBLANES_PER_CELL as f32) * width,
            if self.from_node { height / 2.0 } else { 0.0 },
        );
        let end = (
            (self.to as f32 + 0.5 + self.to_offset as f32 / SUBLANES_PER_CELL as f32) * width,
            if self.to_node { height / 2.0 } else { height },
        );
        if self.start() == self.end() || (!self.start_vertical && !self.end_vertical) {
            return chamfered(start, end, self.incoming, self.outgoing, width, height);
        }
        let leg = (end.1 - start.1) / 2.0;
        let from = (start.0, start.1 + if self.start_vertical { leg } else { 0.0 });
        // Shared-parent arrivals must turn below the current node, which sits halfway down the row.
        let end_leg = if self.to_node { leg } else { height / 4.0 };
        let to = (end.0, end.1 - if self.end_vertical { end_leg } else { 0.0 });
        let inset = width.min(leg);
        let length = (to.0 - from.0).hypot(to.1 - from.1);
        let trim = ((to.0 - from.0) / length * inset, (to.1 - from.1) / length * inset);
        let mut points = chamfered(from, to, self.incoming, self.outgoing, width, height);
        if self.start_vertical {
            let before = (from.0, from.1 - inset);
            let after = (from.0 + trim.0, from.1 + trim.1);
            let mut prefix = Vec::with_capacity(points.len() + CURVE_SEGMENTS + 1);
            if start != before {
                prefix.push(start);
            }
            append_curve(&mut prefix, before, from, after);
            prefix.extend(points.into_iter().skip(1));
            points = prefix;
        }
        if self.end_vertical {
            let inset = width.min(end_leg);
            let trim = ((to.0 - from.0) / length * inset, (to.1 - from.1) / length * inset);
            let before = (to.0 - trim.0, to.1 - trim.1);
            let after = (to.0, to.1 + inset);
            points.truncate(points.len() - 1);
            append_curve(&mut points, before, to, after);
            if after != end {
                points.push(end);
            }
        }
        points
    }
}

const CURVE_SEGMENTS: usize = 6;

fn append_curve(points: &mut Vec<(f32, f32)>, start: (f32, f32), corner: (f32, f32), end: (f32, f32)) {
    points.push(start);
    for step in 1..=CURVE_SEGMENTS {
        let t = step as f32 / CURVE_SEGMENTS as f32;
        let s = 1.0 - t;
        points.push((
            s * s * start.0 + 2.0 * s * t * corner.0 + t * t * end.0,
            s * s * start.1 + 2.0 * s * t * corner.1 + t * t * end.1,
        ));
    }
}

fn append_chamfer(
    points: &mut Vec<(f32, f32)>,
    corner: (f32, f32),
    incoming: (f32, f32),
    outgoing: (f32, f32),
    width: f32,
) {
    let start = points.len();
    append_curve(points, (-incoming.0, -incoming.1), (0.0, 0.0), outgoing);
    let curve = &mut points[start..];
    let clearance = (width / 9.0).max(1.0) + 1.0;
    let limit = (width / 2.0 - clearance).max(0.0);
    let fits = |scale: f32| {
        curve.windows(2).all(|segment| {
            let (a, b) = (segment[0], segment[1]);
            let dy = (b.1 - a.1) * scale;
            let from = ((-clearance - a.1 * scale) / dy).max(0.0);
            let to = ((clearance - a.1 * scale) / dy).min(1.0);
            from > to
                || [from, to]
                    .into_iter()
                    .all(|t| ((a.0 + (b.0 - a.0) * t) * scale).abs() <= limit)
        })
    };
    let mut scale = 1.0;
    // Stroke coverage just outside a row can reach cells that only carry the adjoining route.
    if incoming.0 * outgoing.0 >= 0.0 && !fits(scale) {
        let (mut low, mut high) = (0.0, 1.0);
        for _ in 0..10 {
            let middle = f32::midpoint(low, high);
            if fits(middle) {
                low = middle;
            } else {
                high = middle;
            }
        }
        scale = low;
    }
    for point in curve {
        *point = (corner.0 + point.0 * scale, corner.1 + point.1 * scale);
    }
}

fn chamfered(
    start: (f32, f32),
    end: (f32, f32),
    incoming: Option<i32>,
    outgoing: Option<i32>,
    width: f32,
    height: f32,
) -> Vec<(f32, f32)> {
    let inset = width.min(height * 0.4);
    let direction = |dx: f32, dy: f32| {
        let length = dx.hypot(dy);
        (dx / length * inset, dy / length * inset)
    };
    let current = direction(end.0 - start.0, end.1 - start.1);
    let mut points =
        Vec::with_capacity(2 + CURVE_SEGMENTS * (usize::from(incoming.is_some()) + usize::from(outgoing.is_some())));
    if let Some(delta) = incoming {
        let previous = direction(delta as f32 * width / DIRECTION_SCALE as f32, height);
        append_chamfer(&mut points, start, previous, current, width);
    } else {
        points.push(start);
    }
    if let Some(delta) = outgoing {
        let next = direction(delta as f32 * width / DIRECTION_SCALE as f32, height);
        append_chamfer(&mut points, end, current, next, width);
    } else {
        points.push(end);
    }
    points
}

#[derive(Default)]
struct Junction {
    arrivals: usize,
    departures: usize,
    previous: Option<Edge>,
    next: Option<Edge>,
    stem: bool,
}

fn junctions(before: &[Tile], after: &[Tile], junctions: &mut std::collections::HashMap<i64, Junction>) {
    junctions.clear();
    let scale = i64::from(SUBLANES_PER_CELL);
    for (column, tile) in before.iter().enumerate() {
        for edge in &tile.edges {
            // Each endpoint has one owning tile, even when guard cells carry the same route.
            if !edge.to_node && (0..scale).contains(&edge.end()) {
                let junction = junctions.entry(column as i64 * scale + edge.end()).or_default();
                junction.arrivals += 1;
                junction.previous = Some(edge.clone());
            }
        }
    }
    for (column, tile) in after.iter().enumerate() {
        for edge in &tile.edges {
            if !edge.from_node && (0..scale).contains(&edge.start()) {
                let junction = junctions.entry(column as i64 * scale + edge.start()).or_default();
                junction.departures += 1;
                junction.next = Some(edge.clone());
            }
        }
        if tile.paths & Tile::NODE_UP != 0 && !tile.has_explicit_incoming() {
            let junction = junctions.entry(column as i64 * scale).or_default();
            junction.departures += 1;
            junction.stem = true;
        }
    }
}

#[cfg(test)]
pub(crate) fn chamfer_rows(rows: &mut [Vec<Tile>]) {
    chamfer(rows, true);
}

/// Link routes whose row-boundary endpoints each belong to a single strand.
pub(crate) fn chamfer_strands(rows: &mut [Vec<Tile>]) {
    chamfer(rows, false);
}

fn chamfer(rows: &mut [Vec<Tile>], shared_junctions: bool) {
    for tile in rows.iter_mut().flatten() {
        tile.incoming = None;
        for edge in &mut tile.edges {
            edge.incoming = None;
            edge.outgoing = None;
            edge.start_vertical = edge.from_node && edge.start() != edge.end();
            edge.end_vertical = false;
        }
    }
    let scale = i64::from(SUBLANES_PER_CELL);
    let mut boundary = std::collections::HashMap::new();
    if shared_junctions {
        for row in 1..rows.len() {
            let (before, after) = rows.split_at_mut(row);
            let before = &mut before[row - 1];
            let after = &mut after[0];
            junctions(before, after, &mut boundary);
            boundary.retain(|_, junction| {
                junction.arrivals != 0
                    && junction.departures != 0
                    && (junction.arrivals != 1 || junction.departures != 1)
            });
            if boundary.is_empty() {
                continue;
            }
            // Finish each branch's elbow within its own row; the shared junction remains vertical.
            for (column, tile) in before.iter_mut().enumerate() {
                for edge in &mut tile.edges {
                    if !edge.to_node && boundary.contains_key(&(column as i64 * scale + edge.end())) {
                        edge.end_vertical = true;
                    }
                }
            }
            for (column, tile) in after.iter_mut().enumerate() {
                for edge in &mut tile.edges {
                    if !edge.from_node && boundary.contains_key(&(column as i64 * scale + edge.start())) {
                        edge.start_vertical = true;
                    }
                }
            }
        }
    }
    let route_columns = |edge: &Edge, owner: i64| {
        let columns = edge.columns();
        (owner + i64::from(*columns.start())).max(0) as usize..(owner + i64::from(*columns.end()) + 1).max(0) as usize
    };
    for row in 1..rows.len() {
        let (before, after) = rows.split_at_mut(row);
        let before = &mut before[row - 1];
        let after = &mut after[0];
        junctions(before, after, &mut boundary);
        for (&position, junction) in &boundary {
            debug_assert!(shared_junctions || (junction.arrivals <= 1 && junction.departures <= 1));
            if junction.arrivals != 1 || junction.departures != 1 {
                continue;
            }
            let Some(previous) = junction.previous.as_ref() else {
                continue;
            };
            let Some(incoming) = previous.delta().and_then(|delta| {
                delta.checked_mul(if previous.end_vertical {
                    0
                } else if previous.start_vertical {
                    (if previous.from_node || previous.to_node { 4 } else { 2 }) * DIRECTION_SCALE / SUBLANES_PER_CELL
                } else if previous.from_node || previous.to_node {
                    2 * DIRECTION_SCALE / SUBLANES_PER_CELL
                } else {
                    DIRECTION_SCALE / SUBLANES_PER_CELL
                })
            }) else {
                continue;
            };
            let Some(outgoing) = junction.next.as_ref().map_or(Some(0), |edge| {
                edge.delta()?.checked_mul(if edge.start_vertical {
                    0
                } else if edge.end_vertical {
                    if edge.to_node {
                        4 * DIRECTION_SCALE / SUBLANES_PER_CELL
                    } else {
                        4 * DIRECTION_SCALE / (3 * SUBLANES_PER_CELL)
                    }
                } else if edge.to_node {
                    2 * DIRECTION_SCALE / SUBLANES_PER_CELL
                } else {
                    DIRECTION_SCALE / SUBLANES_PER_CELL
                })
            }) else {
                continue;
            };
            if incoming == outgoing {
                continue;
            }
            // Every cell on a route needs the same corner, including the half outside its row.
            let columns = route_columns(previous, (position - previous.end()) / scale);
            for (column, tile) in before.iter_mut().enumerate().take(columns.end).skip(columns.start) {
                for edge in &mut tile.edges {
                    if !edge.to_node && column as i64 * scale + edge.end() == position {
                        edge.outgoing = Some(outgoing);
                    }
                }
            }
            if junction.stem {
                after[(position / scale) as usize].incoming = Some(incoming);
            } else if let Some(next) = junction.next.as_ref() {
                let columns = route_columns(next, (position - next.start()) / scale);
                for (column, tile) in after.iter_mut().enumerate().take(columns.end).skip(columns.start) {
                    for edge in &mut tile.edges {
                        if !edge.from_node && column as i64 * scale + edge.start() == position {
                            edge.incoming = Some(incoming);
                        }
                    }
                }
            }
        }
    }
}

fn rounded(corners: &[(f32, f32)], radius: f32) -> Vec<(f32, f32)> {
    let mut points = vec![corners[0]];
    for corner in corners.windows(3) {
        let incoming = (corner[0].0 - corner[1].0, corner[0].1 - corner[1].1);
        let outgoing = (corner[2].0 - corner[1].0, corner[2].1 - corner[1].1);
        let before = incoming.0.hypot(incoming.1);
        let after = outgoing.0.hypot(outgoing.1);
        if before == 0.0 || after == 0.0 {
            continue;
        }
        let radius = radius.min(before / 2.0).min(after / 2.0);
        let start = (
            corner[1].0 + incoming.0 / before * radius,
            corner[1].1 + incoming.1 / before * radius,
        );
        let end = (
            corner[1].0 + outgoing.0 / after * radius,
            corner[1].1 + outgoing.1 / after * radius,
        );
        append_curve(&mut points, start, corner[1], end);
    }
    points.push(corners[corners.len() - 1]);
    points
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            paths: 0,
            highlighted_paths: 0,
            path_colors: [0; 10],
            marker: ' ',
            incoming: None,
            edges: Vec::new(),
        }
    }
}

impl Tile {
    pub const UP_DOWN: u16 = 1;
    pub const UP_LEFT: u16 = 2;
    pub const UP_RIGHT: u16 = 4;
    pub const DOWN_LEFT: u16 = 8;
    pub const DOWN_RIGHT: u16 = 16;
    pub const LEFT_RIGHT: u16 = 32;
    pub const NODE_UP: u16 = 64;
    pub const NODE_DOWN: u16 = 128;
    pub const NODE_LEFT: u16 = 256;
    pub const NODE_RIGHT: u16 = 512;

    fn has_explicit_incoming(&self) -> bool {
        self.edges.iter().any(|edge| edge.to_node && edge.end() == 0)
    }

    pub fn encode(&self) -> String {
        let marker = match self.marker {
            '●' => 1,
            '○' => 2,
            '◆' => 3,
            '@' => 4,
            '0'..='9' => 5 + self.marker as u32 - '0' as u32,
            '+' => 15,
            _ => 0,
        };
        let mut out = char::from_u32(0xf0000 + u32::from(self.paths & 1023) * 16 + marker)
            .expect("ten path bits and four marker bits fit in the private-use plane")
            .to_string();
        // Supplementary variation selectors keep the cell one column wide until backend decoding.
        for value in std::iter::once(i32::from(self.highlighted_paths) | (i32::from(self.incoming.is_some()) << 10))
            .chain(self.incoming)
            .chain(
                self.path_colors
                    .iter()
                    .enumerate()
                    .filter_map(|(index, color)| (self.paths & (1 << index) != 0).then_some(i32::from(*color))),
            )
            .chain(self.edges.iter().flat_map(|edge| {
                [
                    Some(edge.from),
                    Some(edge.to),
                    Some(
                        i32::from(edge.from_node)
                            | (i32::from(edge.highlighted) << 1)
                            | (i32::from(edge.color) << 2)
                            | (i32::from(edge.incoming.is_some()) << 6)
                            | (i32::from(edge.outgoing.is_some()) << 7)
                            | (i32::from(edge.start_vertical) << 8)
                            | (i32::from(edge.end_vertical) << 9)
                            | (i32::from(edge.from_offset != 0) << 10)
                            | (i32::from(edge.to_offset != 0) << 11)
                            | (i32::from(edge.to_node) << 12),
                    ),
                    edge.incoming,
                    edge.outgoing,
                    (edge.from_offset != 0).then_some(edge.from_offset),
                    (edge.to_offset != 0).then_some(edge.to_offset),
                ]
                .into_iter()
                .flatten()
            }))
        {
            let mut value = ((value as u32) << 1) ^ ((value >> 31) as u32);
            loop {
                let more = value > 63;
                out.push(
                    char::from_u32(0xe0100 + (value & 63) + if more { 64 } else { 0 })
                        .expect("seven bits fit in supplementary variation selectors"),
                );
                value >>= 6;
                if !more {
                    break;
                }
            }
        }
        out
    }

    pub fn decode(symbol: &str) -> Option<Self> {
        let mut chars = symbol.chars();
        let value = (chars.next()? as u32).checked_sub(0xf0000)?;
        if value > 0x3fff {
            return None;
        }
        let mut values = Vec::new();
        let mut number = 0u32;
        let mut shift = 0;
        for ch in chars {
            let digit = (ch as u32).checked_sub(0xe0100).filter(|value| *value < 128)?;
            if shift > 30 || (shift == 30 && digit & 63 > 3) {
                return None;
            }
            number |= (digit & 63) << shift;
            if digit & 64 == 0 {
                values.push(((number >> 1) as i32) ^ -((number & 1) as i32));
                number = 0;
                shift = 0;
            } else {
                shift += 6;
            }
        }
        let (&header, mut values) = values.split_first()?;
        if shift != 0 || !(0..2048).contains(&header) {
            return None;
        }
        let incoming = if header & 1024 != 0 {
            let (&incoming, remaining) = values.split_first()?;
            values = remaining;
            Some(incoming)
        } else {
            None
        };
        let paths = (value / 16) as u16;
        let (colors, mut values) = values.split_at_checked(paths.count_ones() as usize)?;
        let mut path_colors = [0; 10];
        for (slot, &color) in path_colors
            .iter_mut()
            .enumerate()
            .filter_map(|(index, slot)| (paths & (1 << index) != 0).then_some(slot))
            .zip(colors)
        {
            if !(0..=ROUTE_COLORS.len() as i32).contains(&color) {
                return None;
            }
            *slot = color as u8;
        }
        let mut edges = Vec::with_capacity(values.len() / 3);
        while !values.is_empty() {
            let (parts, remaining) = values.split_at_checked(3)?;
            values = remaining;
            if !(0..8192).contains(&parts[2]) || parts[2] & 63 > ROUTE_COLORS.len() as i32 * 4 + 3 {
                return None;
            }
            let mut neighbor = |flag| {
                if parts[2] & flag == 0 {
                    Some(None)
                } else {
                    let (&delta, remaining) = values.split_first()?;
                    values = remaining;
                    Some(Some(delta))
                }
            };
            edges.push(Edge {
                from: parts[0],
                to: parts[1],
                from_node: parts[2] & 1 != 0,
                to_node: parts[2] & 4096 != 0,
                start_vertical: parts[2] & 256 != 0,
                end_vertical: parts[2] & 512 != 0,
                highlighted: parts[2] & 2 != 0,
                color: ((parts[2] & 63) >> 2) as u8,
                incoming: neighbor(64)?,
                outgoing: neighbor(128)?,
                from_offset: neighbor(1024)?.unwrap_or_default(),
                to_offset: neighbor(2048)?.unwrap_or_default(),
            });
        }
        Some(Self {
            edges,
            paths,
            highlighted_paths: (header & 1023) as u16,
            path_colors,
            incoming,
            marker: match value % 16 {
                1 => '●',
                2 => '○',
                3 => '◆',
                4 => '@',
                digit @ 5..=14 => char::from(b'0' + (digit - 5) as u8),
                15 => '+',
                _ => ' ',
            },
        })
    }

    pub fn from_text(symbol: char) -> Option<Self> {
        let paths = match symbol {
            '│' => Self::UP_DOWN,
            '─' => Self::LEFT_RIGHT,
            '╭' | '┌' => Self::DOWN_RIGHT,
            '╮' | '┐' => Self::DOWN_LEFT,
            '╰' | '└' => Self::UP_RIGHT,
            '╯' | '┘' => Self::UP_LEFT,
            '┼' => Self::UP_DOWN | Self::LEFT_RIGHT,
            '├' => Self::UP_RIGHT | Self::DOWN_RIGHT,
            '┤' => Self::UP_LEFT | Self::DOWN_LEFT,
            '┬' => Self::DOWN_LEFT | Self::DOWN_RIGHT,
            '┴' => Self::UP_LEFT | Self::UP_RIGHT,
            '●' | '○' | '◆' | '@' | '0'..='9' | '+' => {
                return Some(Self {
                    marker: symbol,
                    ..Self::default()
                });
            }
            _ => return None,
        };
        Some(Self {
            paths,
            ..Self::default()
        })
    }

    pub fn text(&self) -> char {
        match self.marker {
            '@' | '0'..='9' | '+' => self.marker,
            _ => ' ',
        }
    }

    pub fn fallback(&self) -> char {
        if self.marker != ' ' {
            return self.marker;
        }
        let up = self.paths & (Self::UP_DOWN | Self::UP_LEFT | Self::UP_RIGHT | Self::NODE_UP) != 0;
        let down = self.paths & (Self::UP_DOWN | Self::DOWN_LEFT | Self::DOWN_RIGHT | Self::NODE_DOWN) != 0;
        let left = self.paths & (Self::LEFT_RIGHT | Self::UP_LEFT | Self::DOWN_LEFT | Self::NODE_LEFT) != 0;
        let right = self.paths & (Self::LEFT_RIGHT | Self::UP_RIGHT | Self::DOWN_RIGHT | Self::NODE_RIGHT) != 0;
        match (up, down, left, right) {
            (true, true, true, true) => '┼',
            (true, true, false, true) => '├',
            (true, true, true, false) => '┤',
            (false, true, true, true) => '┬',
            (true, false, true, true) => '┴',
            (false, true, false, true) => '╭',
            (false, true, true, false) => '╮',
            (true, false, false, true) => '╰',
            (true, false, true, false) => '╯',
            (false, false, true, true) => '─',
            (false, false, false, false) => ' ',
            _ => '│',
        }
    }

    pub fn rasterize(&self, width: u16, height: u16, foreground: [u8; 3], background: [u8; 3], dim: bool) -> Vec<u8> {
        let colors: [[u8; 3]; ROUTE_COLORS.len() + 1] = std::array::from_fn(|index| {
            let color = if index == 0 {
                foreground
            } else {
                ROUTE_COLORS[index - 1]
            };
            if dim {
                std::array::from_fn(|index| u8::midpoint(color[index], background[index]))
            } else {
                color
            }
        });
        let foreground = colors[0];
        let (w, h) = (f32::from(width), f32::from(height));
        let center = (w / 2.0, h / 2.0);
        let up = (center.0, 0.0);
        let down = (center.0, h);
        let left = (0.0, center.1);
        let right = (w, center.1);
        let stroke = (w / 9.0).max(1.0);
        let radius = w * 0.32;
        let marker_half_width = w
            / 2.0
            / if self.marker == '◆' {
                std::f32::consts::SQRT_2
            } else {
                1.0
            };
        let marker_clearance = (w * 0.48).min((marker_half_width - 0.5).max(0.0));
        let mut lines: Vec<_> = self.edges.iter().map(|edge| edge.points(w, h)).collect();
        if !self.edges.is_empty() && self.paths & Self::NODE_UP != 0 && !self.has_explicit_incoming() {
            lines.push(chamfered(up, center, self.incoming, None, w, h));
        }
        let highlighted = |index| {
            self.edges
                .get(index)
                .map_or(self.highlighted_paths & Self::NODE_UP != 0, |edge: &Edge| {
                    edge.highlighted
                })
        };
        let mut gaps = vec![Vec::new(); lines.len()];
        for (index, line) in lines.iter().enumerate() {
            for (other_index, other) in lines[..index].iter().enumerate() {
                let other_edge = &self.edges[other_index];
                let connected = self.edges.get(index).map_or_else(
                    || other_edge.from_node && other_edge.start() == 0,
                    |edge| {
                        (edge.end() == other_edge.end() && edge.to_node == other_edge.to_node)
                            || (edge.start() == other_edge.start() && edge.from_node == other_edge.from_node)
                            || (edge.from_node && other_edge.to_node && edge.start() == other_edge.end())
                            || (edge.to_node && other_edge.from_node && edge.end() == other_edge.start())
                    },
                );
                if connected {
                    continue;
                }
                for a in line.windows(2) {
                    for b in other.windows(2) {
                        if intersection(a[0], a[1], b[0], b[1]).is_some() {
                            let a_slope = ((a[1].1 - a[0].1) * (b[1].0 - b[0].0)).abs();
                            let b_slope = ((b[1].1 - b[0].1) * (a[1].0 - a[0].0)).abs();
                            let steepness = a_slope.total_cmp(&b_slope);
                            // Keep the chosen route continuous; otherwise the more vertical path passes underneath.
                            let underneath = if highlighted(index) != highlighted(other_index) {
                                if highlighted(index) { other_index } else { index }
                            } else if steepness.is_gt() || (steepness.is_eq() && a[1].0 > a[0].0) {
                                index
                            } else {
                                other_index
                            };
                            let clearance = stroke
                                * if highlighted(index) || highlighted(other_index) {
                                    2.5
                                } else {
                                    2.0
                                };
                            let overpass = if underneath == index { b } else { a };
                            gaps[underneath].push((overpass[0], overpass[1], clearance));
                        }
                    }
                }
            }
        }
        // Crossings need whole routes; pixels only need segments close enough to cover this cell.
        let margin = stroke + 0.5;
        let lines: Vec<Vec<_>> = lines
            .iter()
            .map(|line| {
                line.windows(2)
                    .filter_map(|points| {
                        let [a, b] = [points[0], points[1]];
                        (a.0.max(b.0) >= -margin
                            && a.0.min(b.0) <= w + margin
                            && a.1.max(b.1) >= -margin
                            && a.1.min(b.1) <= h + margin)
                            .then_some([a, b])
                    })
                    .collect()
            })
            .collect();
        let line_bounds: Vec<_> = lines
            .iter()
            .map(|line| {
                let mut bounds = (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY);
                for point in line.iter().flatten() {
                    bounds.0 = bounds.0.min(point.0);
                    bounds.1 = bounds.1.max(point.0);
                    bounds.2 = bounds.2.min(point.1);
                    bounds.3 = bounds.3.max(point.1);
                }
                let reach = margin + std::f32::consts::SQRT_2 / 4.0;
                (bounds.0 - reach, bounds.1 + reach, bounds.2 - reach, bounds.3 + reach)
            })
            .collect();
        let paths = if self.edges.is_empty() { self.paths } else { 0 };
        let mut segments = Vec::new();
        for (mask, start, end, curved) in [
            (Self::UP_DOWN, up, down, false),
            (Self::UP_LEFT, up, left, true),
            (Self::UP_RIGHT, up, right, true),
            (Self::DOWN_LEFT, down, left, true),
            (Self::DOWN_RIGHT, down, right, true),
            (Self::LEFT_RIGHT, left, right, false),
            (Self::NODE_UP, center, up, false),
            (Self::NODE_DOWN, center, down, false),
            (Self::NODE_LEFT, center, left, false),
            (Self::NODE_RIGHT, center, right, false),
        ] {
            if paths & mask == 0 {
                continue;
            }
            if curved || (mask == Self::NODE_UP && self.incoming.is_some()) {
                let points = if curved {
                    rounded(&[start, center, end], w * 0.16)
                } else {
                    chamfered(up, center, self.incoming, None, w, h)
                };
                for points in points.windows(2) {
                    segments.push((mask, points[0], points[1]));
                }
            } else {
                segments.push((mask, start, end));
            }
        }

        let crossing = paths & (Self::UP_DOWN | Self::LEFT_RIGHT) == Self::UP_DOWN | Self::LEFT_RIGHT;
        let vertical_highlight = self.highlighted_paths & Self::UP_DOWN != 0;
        let crossing_clearance = stroke * if self.highlighted_paths == 0 { 2.0 } else { 2.5 };
        let mut pixels = Vec::with_capacity(usize::from(width) * usize::from(height) * 4);
        for y in 0..height {
            for x in 0..width {
                let point = (f32::from(x) + 0.5, f32::from(y) + 0.5);
                let delta = (point.0 - center.0, point.1 - center.1);
                let circle = delta.0.hypot(delta.1);
                let shape_distance = if self.marker == '◆' {
                    (delta.0.abs() + delta.1.abs()) / std::f32::consts::SQRT_2
                } else {
                    circle
                };
                let mut coverage = [0.0f32; 2];
                let mut normal = foreground;
                // Antialiasing samples must not change which route supplies a join's color.
                let mut normal_priority = f32::NEG_INFINITY;
                for &(mask, start, end) in &segments {
                    if crossing
                        && if vertical_highlight {
                            mask == Self::LEFT_RIGHT && delta.0.abs() < crossing_clearance
                        } else {
                            mask == Self::UP_DOWN && delta.1.abs() < crossing_clearance
                        }
                    {
                        continue;
                    }
                    let selected = self.highlighted_paths & mask != 0;
                    let half_stroke = stroke * if selected { 1.0 } else { 0.5 };
                    let (edge, priority) =
                        stroke_coverage(point, half_stroke, |sample| distance_to_segment(sample, start, end));
                    if !selected && edge > 0.0 && priority > normal_priority {
                        normal = colors[usize::from(self.path_colors[mask.trailing_zeros() as usize])];
                        normal_priority = priority;
                    }
                    coverage[usize::from(selected)] = coverage[usize::from(selected)].max(edge);
                }
                for (index, line) in lines.iter().enumerate() {
                    let bounds = line_bounds[index];
                    if point.0 < bounds.0 || point.0 > bounds.1 || point.1 < bounds.2 || point.1 > bounds.3 {
                        continue;
                    }
                    if gaps[index]
                        .iter()
                        .any(|(start, end, clearance)| distance_to_segment(point, *start, *end) < *clearance)
                    {
                        continue;
                    }
                    let selected = highlighted(index);
                    let half_stroke = stroke * if selected { 1.0 } else { 0.5 };
                    let distance = |sample| {
                        line.iter()
                            .map(|p| distance_to_segment(sample, p[0], p[1]))
                            .fold(f32::INFINITY, f32::min)
                    };
                    let (mut edge, priority) = stroke_coverage(point, half_stroke, distance);
                    if self.marker != ' '
                        && self
                            .edges
                            .get(index)
                            .is_some_and(|edge| !edge.from_node && !edge.to_node)
                    {
                        // Passing rails have no connection to this marker; keep their halo inside its cell.
                        edge *= (shape_distance - marker_clearance + 0.5).clamp(0.0, 1.0);
                    }
                    if !selected && edge > 0.0 && priority > normal_priority {
                        normal =
                            colors[usize::from(self.edges.get(index).map_or(self.path_colors[6], |edge| edge.color))];
                        normal_priority = priority;
                    }
                    coverage[usize::from(selected)] = coverage[usize::from(selected)].max(edge);
                }
                let mut node = 0.0;
                if self.marker != ' ' {
                    if self.text() != ' ' || self.marker == '○' {
                        let clearance = if self.marker == '○' {
                            radius - stroke / 2.0
                        } else {
                            w * 0.48
                        };
                        for layer in &mut coverage {
                            *layer *= (shape_distance - clearance + 0.5).clamp(0.0, 1.0);
                        }
                    }
                    node = match self.marker {
                        '●' | '◆' => (radius + 0.5 - shape_distance).clamp(0.0, 1.0),
                        '○' => (stroke / 2.0 + 0.5 - (circle - radius).abs()).clamp(0.0, 1.0),
                        _ => 0.0,
                    };
                    coverage[0] = coverage[0].max(node);
                    coverage[1] *= 1.0 - node;
                }
                let alpha = coverage[1] + coverage[0] * (1.0 - coverage[1]);
                let highlight = if alpha == 0.0 { 0.0 } else { coverage[1] / alpha };
                let node = if coverage[0] == 0.0 { 0.0 } else { node / coverage[0] };
                for ((normal, foreground), selected) in normal.into_iter().zip(foreground).zip([0u8, 255, 255]) {
                    let normal = f32::from(normal) * (1.0 - node) + f32::from(foreground) * node;
                    pixels.push((normal * (1.0 - highlight) + f32::from(selected) * highlight).round() as u8);
                }
                pixels.push((alpha * 255.0).round() as u8);
            }
        }
        pixels
    }
}

fn intersection(a: (f32, f32), b: (f32, f32), c: (f32, f32), d: (f32, f32)) -> Option<(f32, f32)> {
    let r = (b.0 - a.0, b.1 - a.1);
    let s = (d.0 - c.0, d.1 - c.1);
    let cross = |a: (f32, f32), b: (f32, f32)| a.0 * b.1 - a.1 * b.0;
    let denominator = cross(r, s);
    if denominator.abs() < f32::EPSILON {
        return None;
    }
    let offset = (c.0 - a.0, c.1 - a.1);
    let t = cross(offset, s) / denominator;
    let u = cross(offset, r) / denominator;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then_some((a.0 + t * r.0, a.1 + t * r.1))
}

fn distance_to_segment(point: (f32, f32), start: (f32, f32), end: (f32, f32)) -> f32 {
    let delta = (end.0 - start.0, end.1 - start.1);
    let length_squared = delta.0 * delta.0 + delta.1 * delta.1;
    let t = if length_squared == 0.0 {
        0.0
    } else {
        (((point.0 - start.0) * delta.0 + (point.1 - start.1) * delta.1) / length_squared).clamp(0.0, 1.0)
    };
    (point.0 - start.0 - t * delta.0).hypot(point.1 - start.1 - t * delta.1)
}

fn stroke_coverage(point: (f32, f32), half_stroke: f32, distance: impl Fn((f32, f32)) -> f32) -> (f32, f32) {
    let center = half_stroke + 0.5 - distance(point);
    // Stroke interiors tie in route order even when thin strokes are never fully opaque.
    let priority = center.min(0.5);
    let reach = std::f32::consts::SQRT_2 / 4.0;
    // Distance changes by at most the sample offset; skip samples wholly inside or outside the stroke.
    if center <= -reach || center >= 1.0 + reach {
        return (center.clamp(0.0, 1.0), priority);
    }
    let mut coverage = 0.0;
    for dx in [-0.25, 0.25] {
        for dy in [-0.25, 0.25] {
            coverage += (half_stroke + 0.5 - distance((point.0 + dx, point.1 + dy))).clamp(0.0, 1.0);
        }
    }
    (coverage / 4.0, priority)
}

#[cfg(test)]
mod tests {
    use super::{CURVE_SEGMENTS, DIRECTION_SCALE, Edge, ROUTE_COLORS, Tile, chamfer_rows, chamfered, route_edges};

    fn tile_at(edges: &[Edge], column: i32) -> Tile {
        Tile {
            edges: edges
                .iter()
                .filter(|edge| (edge.from.min(edge.to)..=edge.from.max(edge.to)).contains(&column))
                .map(|edge| Edge {
                    from: edge.from - column,
                    to: edge.to - column,
                    ..*edge
                })
                .collect(),
            ..Tile::default()
        }
    }

    fn tiled_row(edges: &[Edge], width: usize) -> Vec<Tile> {
        (0..width).map(|column| tile_at(edges, column as i32)).collect()
    }

    fn guarded_row(edges: &[Edge], width: usize) -> Vec<Tile> {
        (0..width)
            .map(|column| Tile {
                edges: edges
                    .iter()
                    .filter(|edge| edge.columns().contains(&(column as i32)))
                    .map(|edge| Edge {
                        from: edge.from - column as i32,
                        to: edge.to - column as i32,
                        ..edge.clone()
                    })
                    .collect(),
                ..Tile::default()
            })
            .collect()
    }

    #[test]
    fn fractional_strands_remain_independent_until_their_parent_node() {
        let base = route_edges(2, &[(0, 0)]).remove(0);
        for count in [3usize, 20] {
            let routes = |from, to, to_node| {
                (0..count)
                    .map(|slot| Edge {
                        from,
                        to,
                        from_offset: slot as i32,
                        to_offset: if to_node { 0 } else { slot as i32 },
                        to_node,
                        color: (slot % ROUTE_COLORS.len() + 1) as u8,
                        highlighted: slot == 1,
                        ..base.clone()
                    })
                    .collect::<Vec<_>>()
            };
            let width = count.div_ceil(3) + 4;
            let mut rows = vec![
                guarded_row(&routes(0, 1, false), width),
                guarded_row(&routes(1, 1, false), width),
                guarded_row(&routes(1, 1, true), width),
            ];
            rows[2][1].marker = '●';
            rows[2][1].paths = Tile::NODE_UP;
            rows[2][1].highlighted_paths = Tile::NODE_UP;
            let mut shared = rows.clone();
            chamfer_rows(&mut shared);
            super::chamfer_strands(&mut rows);
            assert_eq!(rows, shared, "unique strands need no shared-junction easing pass");
            for (cell_width, cell_height) in [(8u16, 16u16), (16, 35), (24, 50)] {
                let (w, h) = (f32::from(cell_width), f32::from(cell_height));
                for slot in 0..count {
                    let column = 1 + slot / 3;
                    let route = |row: usize| {
                        rows[row][column]
                            .edges
                            .iter()
                            .find(|edge| edge.from_offset == slot as i32)
                            .expect("the endpoint tile carries its strand")
                    };
                    for row in 0..2 {
                        let above = route(row).points(w, h);
                        let below = route(row + 1).points(w, h);
                        if route(row).outgoing.is_some() {
                            for (a, b) in above[above.len() - CURVE_SEGMENTS - 1..]
                                .iter()
                                .zip(&below[..=CURVE_SEGMENTS])
                            {
                                assert!(
                                    (a.0 - b.0).abs() < 0.00001 && (a.1 - h - b.1).abs() < 0.00001,
                                    "fractional endpoints share identical chamfer samples across rows"
                                );
                            }
                        }
                    }
                    let incoming = route(2);
                    assert!(
                        !incoming.end_vertical,
                        "incoming strands meet only at the actual commit, without an early shared stem"
                    );
                    assert_eq!(
                        incoming.points(w, h).last(),
                        Some(&((1.5 - column as f32) * w, h / 2.0)),
                        "all incoming strands terminate at the parent marker"
                    );
                    let x = ((1.5 + slot as f32 / 3.0) * w) as usize;
                    let pixels = rows[1][x / usize::from(cell_width)].rasterize(
                        cell_width,
                        cell_height,
                        [255; 3],
                        [0; 3],
                        false,
                    );
                    let pixel =
                        (usize::from(cell_height) / 2 * usize::from(cell_width) + x % usize::from(cell_width)) * 4;
                    assert_eq!(
                        &pixels[pixel..pixel + 3],
                        if slot == 1 {
                            &[0, 255, 255]
                        } else {
                            &ROUTE_COLORS[slot % ROUTE_COLORS.len()]
                        },
                        "highlighting one strand leaves the adjacent rails in their own colors"
                    );
                    assert!(pixels[pixel + 3] > 0, "every strand retains a visible independent rail");
                }
                let parent = &rows[2][1];
                assert_eq!(
                    parent.rasterize(cell_width, cell_height, [255; 3], [0; 3], false),
                    Tile {
                        paths: 0,
                        ..parent.clone()
                    }
                    .rasterize(cell_width, cell_height, [255; 3], [0; 3], false),
                    "explicit incoming strands suppress the implicit highlighted NODE_UP stem"
                );
                for row in &rows {
                    let routes: Vec<_> = row
                        .iter()
                        .enumerate()
                        .flat_map(|(column, tile)| {
                            tile.edges
                                .iter()
                                .filter(|edge| (0..i64::from(super::SUBLANES_PER_CELL)).contains(&edge.start()))
                                .map(move |edge| Edge {
                                    from: edge.from + column as i32,
                                    to: edge.to + column as i32,
                                    ..edge.clone()
                                })
                        })
                        .collect();
                    for (column, tile) in row.iter().enumerate() {
                        let reference = Tile {
                            edges: routes
                                .iter()
                                .map(|edge| Edge {
                                    from: edge.from - column as i32,
                                    to: edge.to - column as i32,
                                    ..edge.clone()
                                })
                                .collect(),
                            ..tile.clone()
                        };
                        assert_eq!(
                            tile.rasterize(cell_width, cell_height, [255; 3], [0; 3], false),
                            reference.rasterize(cell_width, cell_height, [255; 3], [0; 3], false),
                            "guard cells retain all fractional strokes and cross-row chamfers"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn fractional_offsets_and_node_arrivals_survive_encoding() {
        for from_offset in [i32::MIN, -1, 0, 1, 2, 20, i32::MAX] {
            for to_offset in [i32::MIN, -1, 0, 1, 2, 20, i32::MAX] {
                let mut tile = tile_at(&route_edges(1, &[(0, 0)]), 0);
                tile.edges[0].from_offset = from_offset;
                tile.edges[0].to_offset = to_offset;
                tile.edges[0].to_node = true;
                assert_eq!(
                    Tile::decode(&tile.encode()),
                    Some(tile.clone()),
                    "fractional slots and explicit parent arrivals survive buffer encoding"
                );
                let mut truncated = tile.encode();
                truncated.pop();
                assert_eq!(Tile::decode(&truncated), None, "incomplete slot metadata is rejected");
            }
        }
    }

    #[test]
    fn fractional_tile_bounds_cover_turns_in_every_offset_phase() {
        let base = route_edges(2, &[(0, 0)]).remove(0);
        for positions in [[1, 1, 2, 3], [3, 2, 1, 1], [1, 2, 1, 2], [1, 3, 1, 3], [3, 1, 3, 1]] {
            for phase in 0..3 {
                let mut rows: Vec<_> = positions
                    .windows(2)
                    .enumerate()
                    .map(|(row, ends)| {
                        let edge = Edge {
                            from: ends[0],
                            to: ends[1],
                            from_offset: (phase + row as i32) % 3,
                            to_offset: (phase + row as i32 + 1) % 3,
                            from_node: row == 0,
                            to_node: row == 2,
                            highlighted: true,
                            ..base.clone()
                        };
                        guarded_row(&[edge], 6)
                    })
                    .collect();
                chamfer_rows(&mut rows);
                for (row, tiles) in rows.iter().enumerate() {
                    let source = positions[row];
                    let edge = &tiles[source as usize].edges[0];
                    for (column, tile) in tiles.iter().enumerate() {
                        let reference = Tile {
                            edges: vec![Edge {
                                from: edge.from + source - column as i32,
                                to: edge.to + source - column as i32,
                                ..edge.clone()
                            }],
                            ..Tile::default()
                        };
                        for (width, height) in [(8, 16), (16, 35), (24, 50)] {
                            assert_eq!(
                                tile.rasterize(width, height, [255; 3], [0; 3], false),
                                reference.rasterize(width, height, [255; 3], [0; 3], false),
                                "fractional turns keep their stroke coverage without extra integer-route columns"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn raster_bounds_keep_subpixel_coverage_and_crossing_routes() {
        let mut edges = route_edges(1, &[(0, 0)]);
        edges[0].highlighted = true;
        let pixels = tile_at(&edges, 0).rasterize(16, 35, [255; 3], [0; 3], false);
        assert_eq!(
            pixels[(17 * 16 + 10) * 4 + 3],
            4,
            "subpixel samples still cover a pixel whose center is outside the stroke's coverage ramp"
        );

        let mut edges = route_edges(3, &[(0, 2), (1, 1)]);
        edges[0].color = 1;
        edges[0].incoming = Some(0);
        edges[0].outgoing = Some(0);
        edges[1].color = 2;
        for highlighted in [false, true] {
            edges[1].highlighted = highlighted;
            let tile = tile_at(&edges, 2);
            let mut distant = tile.clone();
            distant.edges.insert(
                0,
                Edge {
                    from: 100,
                    to: 104,
                    color: 3,
                    ..edges[0].clone()
                },
            );
            assert_eq!(
                tile.rasterize(16, 35, [255; 3], [0; 3], false),
                distant.rasterize(16, 35, [255; 3], [0; 3], false),
                "discarding invisible segments preserves route colors, crossing gaps, and highlight priority"
            );
        }
    }

    #[test]
    fn chamfers_subdivide_the_turn_instead_of_leaving_bevel_edges() {
        let points = chamfered(
            (10.0, 0.0),
            (10.0, 40.0),
            Some(2 * DIRECTION_SCALE),
            Some(-2 * DIRECTION_SCALE),
            20.0,
            40.0,
        );
        for corner in points.windows(3) {
            let incoming = (corner[1].0 - corner[0].0, corner[1].1 - corner[0].1);
            let outgoing = (corner[2].0 - corner[1].0, corner[2].1 - corner[1].1);
            let cosine = (incoming.0 * outgoing.0 + incoming.1 * outgoing.1)
                / (incoming.0.hypot(incoming.1) * outgoing.0.hypot(outgoing.1));
            assert!(
                cosine > 15.0f32.to_radians().cos(),
                "each sampled turn is gentler than the original bevel's 22.5-degree edges"
            );
        }
    }

    #[test]
    fn chamfers_round_a_visible_part_of_the_row_without_reaching_the_node() {
        let points = chamfered((8.0, 0.0), (8.0, 17.5), Some(2 * DIRECTION_SCALE), None, 16.0, 35.0);
        assert!(
            points[CURVE_SEGMENTS].1 >= 10.0,
            "ordinary bends round a substantial part of the row rather than only a few pixels"
        );
        assert!(
            points[CURVE_SEGMENTS].1 < 17.5 && points.last() == Some(&(8.0, 17.5)),
            "even a half-row stem keeps a straight connection to its node"
        );
        let reversal = chamfered((0.0, 0.0), (32.0, 35.0), None, Some(-2 * DIRECTION_SCALE), 16.0, 35.0);
        assert!(
            (reversal[1].0 - 32.0).hypot(reversal[1].1 - 35.0) >= 10.0,
            "reversals can use the full curve because both rows already draw on the bend's side"
        );
    }

    #[test]
    fn forks_and_merges_turn_vertically_at_their_junctions() {
        for lanes in [1, 20] {
            let mut rows = vec![
                tiled_row(&route_edges(0, &[(0, 0), (0, lanes)]), lanes * 2 + 1),
                tiled_row(&route_edges(lanes + 1, &[(0, 0), (lanes, 0)]), lanes * 2 + 1),
                tiled_row(&route_edges(0, &[(0, 0)]), lanes * 2 + 1),
            ];
            rows[2][0].paths = Tile::NODE_UP;
            chamfer_rows(&mut rows);
            let branch = rows[0][0].edges[1].points(16.0, 35.0);
            assert_eq!(branch[0], (8.0, 17.5), "a fork starts at its actual node");
            let departure = (branch[1].0 - branch[0].0, branch[1].1 - branch[0].1);
            assert!(
                departure.1 > 0.0 && departure.0.abs() < departure.1 * 0.3,
                "even a long fork eases out vertically before turning toward its lane"
            );
            let merge = rows[1][0].edges[1].points(16.0, 35.0);
            let end = merge[merge.len() - 1];
            let before = merge[merge.len() - 2];
            assert_eq!(end, (8.0, 35.0), "a merge retains its shared endpoint");
            assert!(
                end.1 > before.1 && (end.0 - before.0).abs() < (end.1 - before.1) * 0.3,
                "merging routes ease into the shared vertical lane"
            );
        }
    }

    #[test]
    fn shared_parent_rails_join_below_child_markers() {
        for lanes in [1, 3, 20] {
            for (highlighted, marker_kind) in [false, true]
                .into_iter()
                .flat_map(|highlighted| ['●', '◆', '○', '@'].map(|marker| (highlighted, marker)))
            {
                let mut incoming = route_edges(0, &[(lanes, 0), (0, 0)]);
                incoming[0].color = 1;
                incoming[0].highlighted = highlighted;
                let mut rows = vec![
                    tiled_row(&route_edges(lanes + 2, &[(lanes + 1, lanes)]), lanes * 2 + 3),
                    tiled_row(&incoming, lanes * 2 + 3),
                    tiled_row(&route_edges(1, &[(0, 0)]), lanes * 2 + 3),
                ];
                rows[1][0].marker = marker_kind;
                rows[1][0].paths = Tile::NODE_UP;
                chamfer_rows(&mut rows);
                for (width, height) in [(8u16, 16u16), (16, 35), (24, 50)] {
                    let (w, h) = (f32::from(width), f32::from(height));
                    let above = rows[0][lanes * 2].edges[0].points(w, h);
                    let below = rows[1][lanes * 2].edges[0].points(w, h);
                    for (a, b) in above[above.len() - CURVE_SEGMENTS - 1..]
                        .iter()
                        .zip(&below[..=CURVE_SEGMENTS])
                    {
                        assert!(
                            (a.0 - b.0).abs() < 0.00001 && (a.1 - h - b.1).abs() < 0.00001,
                            "neighboring rows share the same curve into a three-quarter-row elbow"
                        );
                    }
                    let tile = &rows[1][0];
                    let rail = &tile.edges[0];
                    let points = rail.points(w, h);
                    let center = (w / 2.0, h / 2.0);
                    assert!(
                        points
                            .windows(2)
                            .all(|line| { super::distance_to_segment(center, line[0], line[1]) > w * 0.32 }),
                        "an existing parent's rail passes below this child instead of through its marker"
                    );
                    assert_eq!(
                        points.last(),
                        tile.edges[1].points(w, h).last(),
                        "both routes still join at the shared parent lane below the child"
                    );
                    let passing = Tile {
                        paths: 0,
                        edges: vec![rail.clone()],
                        ..tile.clone()
                    };
                    let pixels = passing.rasterize(width, height, [255; 3], [0; 3], false);
                    let marker = Tile {
                        marker: marker_kind,
                        ..Tile::default()
                    }
                    .rasterize(width, height, [255; 3], [0; 3], false);
                    for (actual, marker) in pixels.as_chunks::<4>().0.iter().zip(marker.as_chunks::<4>().0) {
                        if marker[3] == 255 || (marker_kind == '●' && marker[3] > 0) {
                            assert_eq!(
                                actual, marker,
                                "a highlighted passing rail cannot repaint an unrelated marker"
                            );
                        }
                    }
                    let pixels = tile.rasterize(width, height, [255; 3], [0; 3], false);
                    assert!(
                        (0..usize::from(height))
                            .filter(|y| marker_kind == '●' || (*y as f32 - h / 2.0).abs() > w / 2.0)
                            .all(|y| pixels[(y * usize::from(width) + usize::from(width) / 2) * 4 + 3] > 0),
                        "the child's own stems remain connected outside any intentional marker gap"
                    );
                }
            }
        }
    }

    #[test]
    fn wider_chamfers_keep_shallow_turns_inside_the_stem_cell() {
        for (width, height) in [(8.0f32, 16.0), (13.0, 29.0), (16.0, 35.0), (40.0, 80.0)] {
            for incoming in [-200, -40, -2, 2, 40, 200] {
                let points = chamfered(
                    (width / 2.0, 0.0),
                    (width / 2.0, height / 2.0),
                    Some(incoming * DIRECTION_SCALE),
                    None,
                    width,
                    height,
                );
                let above = chamfered(
                    (width / 2.0 - incoming as f32 * width, -height),
                    (width / 2.0, 0.0),
                    None,
                    Some(0),
                    width,
                    height,
                );
                assert_eq!(
                    &above[above.len() - CURVE_SEGMENTS - 1..],
                    &points[..=CURVE_SEGMENTS],
                    "both rows choose identical samples even when the curve needs shrinking"
                );
                let departing = chamfered(
                    (width / 2.0, -height),
                    (width / 2.0, 0.0),
                    None,
                    Some(incoming * DIRECTION_SCALE),
                    width,
                    height,
                );
                let coverage = (width / 9.0).max(1.0) + 1.0;
                for (curve, sign) in [(&points, 1.0), (&departing, -1.0)] {
                    for y in 0..height as i32 {
                        for x in [-0.5, width + 0.5] {
                            assert!(
                                curve.windows(2).all(|line| super::distance_to_segment(
                                    (x, (y as f32 + 0.5) * sign),
                                    line[0],
                                    line[1]
                                ) >= coverage),
                                "even a highlighted curve must not draw into a missing neighboring stem tile"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn chamfers_preserve_parallel_lanes_and_collinear_continuations() {
        let mut rows: Vec<_> = [(0, 1), (1, 2), (2, 2), (2, 1)]
            .into_iter()
            .map(|(from, to)| {
                tiled_row(
                    &route_edges(30, &(0..20).map(|lane| (lane + from, lane + to)).collect::<Vec<_>>()),
                    45,
                )
            })
            .collect();
        let first = rows[0].clone();
        chamfer_rows(&mut rows);
        assert_eq!(rows[0], first, "a long diagonal remains straight across row boundaries");
        for lane in 0..20 {
            let diagonal = rows[1][(lane + 1) * 2]
                .edges
                .iter()
                .find(|edge| edge.from == 0)
                .expect("each lane has a departing diagonal");
            let vertical = rows[2][(lane + 2) * 2]
                .edges
                .iter()
                .find(|edge| edge.from == 0)
                .expect("each lane has a vertical continuation");
            assert_eq!(diagonal.incoming, None, "collinear joins need no chamfer");
            assert_eq!(
                diagonal.outgoing,
                Some(0),
                "all parallel arrivals use the same next direction"
            );
            assert_eq!(
                vertical.incoming,
                Some(2 * DIRECTION_SCALE),
                "equal colors do not confuse adjacent routes"
            );
            assert_eq!(
                vertical.outgoing,
                Some(-2 * DIRECTION_SCALE),
                "the next corner can turn in the opposite direction"
            );
            for (width, height) in [(8.0, 16.0), (20.0, 40.0), (13.0, 29.0)] {
                let a = diagonal.points(width, height);
                let b = vertical.points(width, height);
                for (a, b) in a[a.len() - CURVE_SEGMENTS - 1..].iter().zip(&b[..=CURVE_SEGMENTS]) {
                    assert!(
                        (a.0 - 2.0 * width - b.0).abs() < 0.00001 && (a.1 - height - b.1).abs() < 0.00001,
                        "both rows clip the same shared curve"
                    );
                }
            }
        }
        for row in &rows {
            for (column, tile) in row.iter().enumerate() {
                for edge in &tile.edges {
                    let source = &row[column
                        .checked_add_signed(edge.from as isize)
                        .expect("route source is visible")];
                    let source = source
                        .edges
                        .iter()
                        .find(|candidate| candidate.from == 0 && candidate.to == edge.to - edge.from)
                        .expect("the source tile contains the same route");
                    assert_eq!(
                        edge.incoming, source.incoming,
                        "every horizontal tile shares the incoming chamfer"
                    );
                    assert_eq!(
                        edge.outgoing, source.outgoing,
                        "every horizontal tile shares the outgoing chamfer"
                    );
                }
            }
        }
        let once = rows.clone();
        chamfer_rows(&mut rows);
        assert_eq!(rows, once, "linking corners is repeatable");
    }

    #[test]
    fn chamfers_meet_half_row_node_stems_including_roots() {
        for outgoing in [false, true] {
            let mut rows = vec![
                tiled_row(&route_edges(0, &[(0, 1)]), 3),
                tiled_row(
                    &if outgoing {
                        route_edges(1, &[(1, 1)])
                    } else {
                        Vec::new()
                    },
                    3,
                ),
            ];
            rows[1][2].paths = Tile::NODE_UP;
            rows[1][2].marker = '●';
            chamfer_rows(&mut rows);
            assert_eq!(
                rows[1][2].incoming,
                Some(8 * DIRECTION_SCALE),
                "the straight run after the node elbow occupies a quarter row"
            );
            for (width, height) in [(8u16, 16u16), (20, 40), (13, 29)] {
                let (w, h) = (f32::from(width), f32::from(height));
                let above = rows[0][2].edges[0].points(w, h);
                let below = chamfered(
                    (w / 2.0, 0.0),
                    (w / 2.0, h / 2.0),
                    Some(8 * DIRECTION_SCALE),
                    None,
                    w,
                    h,
                );
                for (a, b) in above[above.len() - CURVE_SEGMENTS - 1..]
                    .iter()
                    .zip(&below[..=CURVE_SEGMENTS])
                {
                    assert!(
                        (a.0 - b.0).abs() < 0.00001 && (a.1 - h - b.1).abs() < 0.00001,
                        "node stems and arriving edges share the same curve"
                    );
                }
                let pixels = rows[1][2].rasterize(width, height, [255; 3], [0; 3], false);
                assert!(
                    pixels[..usize::from(width) * 4]
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .any(|pixel| pixel[3] > 0),
                    "the stem reaches the top row boundary even without an outgoing edge"
                );
                let straight = Tile {
                    incoming: None,
                    ..rows[1][2].clone()
                };
                assert_ne!(
                    pixels,
                    straight.rasterize(width, height, [255; 3], [0; 3], false),
                    "the stem is chamfered in both rendering paths"
                );
            }
        }
    }

    #[test]
    fn junction_easing_preserves_routes_and_disconnected_tips() {
        for (before, after, current) in [
            (vec![(0, 1), (2, 1)], vec![(1, 0)], 3),
            (vec![(0, 1)], vec![(1, 0), (1, 2)], 3),
            (vec![(0, 1)], vec![(1, 0)], 1),
        ] {
            let mut rows = vec![
                tiled_row(&route_edges(3, &before), 5),
                tiled_row(&route_edges(current, &after), 5),
            ];
            let original = rows.clone();
            chamfer_rows(&mut rows);
            assert!(
                rows[0]
                    .iter()
                    .flat_map(|tile| &tile.edges)
                    .all(|edge| edge.end_vertical == (current != 1)),
                "arrivals ease into connected junctions without joining a disconnected node below"
            );
            let once = rows.clone();
            chamfer_rows(&mut rows);
            assert_eq!(rows, once, "junction easing is repeatable");
            for edge in rows.iter_mut().flatten().flat_map(|tile| &mut tile.edges) {
                edge.start_vertical = false;
                edge.end_vertical = false;
            }
            assert_eq!(
                rows, original,
                "junction elbows leave the routes intact and need no shared curve outside their row"
            );
        }
    }

    #[test]
    fn chamfered_crossings_keep_their_colors_and_priority_when_reordered() {
        for lanes in [1, 20] {
            for selected in [false, true] {
                let mut rows: Vec<_> = [[(0, 0), (1, 1)], [(0, lanes), (1, 0)], [(lanes, lanes), (0, 0)]]
                    .into_iter()
                    .map(|routes| {
                        let mut edges = route_edges(lanes + 1, &routes);
                        edges[0].color = 1;
                        edges[1].color = 2;
                        edges[1].highlighted = selected;
                        tiled_row(&edges, lanes * 2 + 1)
                    })
                    .collect();
                chamfer_rows(&mut rows);
                assert!(
                    rows[1]
                        .iter()
                        .flat_map(|tile| &tile.edges)
                        .all(|edge| { edge.incoming == Some(0) && edge.outgoing == Some(0) }),
                    "both crossing paths have active endpoint chamfers"
                );
                for tile in &rows[1] {
                    let mut reversed = tile.clone();
                    reversed.edges.reverse();
                    for (width, height) in [(8, 16), (20, 40), (13, 29)] {
                        assert!(
                            tile.rasterize(width, height, [255; 3], [0; 3], false)
                                == reversed.rasterize(width, height, [255; 3], [0; 3], false),
                            "crossings near chamfers retain color and selection priority regardless of edge order"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn chamfer_metadata_survives_encoding_and_rejects_truncation() {
        for incoming in [None, Some(i32::MIN), Some(0), Some(i32::MAX)] {
            for outgoing in [None, Some(i32::MIN), Some(0), Some(i32::MAX)] {
                let mut tile = tile_at(&route_edges(0, &[(0, 1)]), 0);
                tile.incoming = incoming;
                tile.edges[0].incoming = incoming;
                tile.edges[0].outgoing = outgoing;
                tile.edges[0].start_vertical = incoming.is_some();
                tile.edges[0].end_vertical = outgoing.is_some();
                assert_eq!(
                    Tile::decode(&tile.encode()),
                    Some(tile.clone()),
                    "all optional corner directions round trip"
                );
                let mut truncated = tile.encode();
                truncated.pop();
                assert_eq!(
                    Tile::decode(&truncated),
                    None,
                    "incomplete edge or chamfer data is rejected"
                );
            }
        }
        let mut stem = Tile {
            incoming: Some(0),
            ..Tile::default()
        }
        .encode();
        stem.pop();
        assert_eq!(Tile::decode(&stem), None, "a stem flag requires its direction");
    }

    #[test]
    fn tiles_round_trip_without_confusing_ordinary_text() {
        for paths in 0..1024 {
            for marker in [' ', '●', '○', '◆', '@', '0', '9', '+'] {
                let tile = Tile {
                    paths,
                    highlighted_paths: paths,
                    path_colors: std::array::from_fn(|index| {
                        if paths & (1 << index) != 0 {
                            (index % ROUTE_COLORS.len() + 1) as u8
                        } else {
                            0
                        }
                    }),
                    marker,
                    ..Tile::default()
                };
                assert_eq!(
                    Tile::decode(&tile.encode().to_string()),
                    Some(tile),
                    "geometry survives buffer diffing"
                );
            }
        }
        assert_eq!(Tile::decode("●"), None, "ordinary UI symbols remain text");
        assert_eq!(Tile::decode(""), None, "empty text is not geometry");
        let tile = Tile {
            edges: vec![Edge {
                from: i32::MIN,
                to: i32::MAX,
                from_offset: 0,
                to_offset: 0,
                from_node: true,
                to_node: false,
                start_vertical: false,
                end_vertical: false,
                highlighted: true,
                color: ROUTE_COLORS.len() as u8,
                incoming: None,
                outgoing: None,
            }],
            ..Tile::default()
        };
        assert_eq!(
            Tile::decode(&tile.encode()),
            Some(tile),
            "edge coordinates and routing round trip without truncation"
        );
        assert_eq!(
            Tile::decode("\u{f0000}\u{e0140}"),
            None,
            "truncated edge data is rejected"
        );
        let mut invalid = Tile {
            paths: Tile::UP_DOWN,
            ..Tile::default()
        };
        invalid.path_colors[0] = ROUTE_COLORS.len() as u8 + 1;
        assert_eq!(
            Tile::decode(&invalid.encode()),
            None,
            "invalid path colors are rejected"
        );
        invalid.path_colors[0] = 0;
        invalid.edges = route_edges(0, &[(0, 0)]);
        invalid.edges[0].color = ROUTE_COLORS.len() as u8 + 1;
        assert_eq!(
            Tile::decode(&invalid.encode()),
            None,
            "invalid edge colors are rejected"
        );
    }

    #[test]
    fn route_colors_survive_horizontal_cell_boundaries_and_crossings() {
        let mut edges = route_edges(3, &[(0, 2), (1, 1)]);
        edges[0].color = 1;
        edges[1].color = 2;
        fn pixel(pixels: &[u8], x: usize, y: usize) -> &[u8] {
            &pixels[(y * 20 + x) * 4..(y * 20 + x + 1) * 4]
        }
        let color = |index: usize| {
            let [r, g, b] = ROUTE_COLORS[index];
            [r, g, b, 255]
        };
        for column in 1..4 {
            let pixels = tile_at(&edges, column).rasterize(20, 40, [column as u8; 3], [0; 3], false);
            for x in 0..20 {
                let y = (column as usize * 20 + x - 10) / 2;
                assert_eq!(
                    pixel(&pixels, x, y),
                    color(0),
                    "one route keeps its color across cells with different foregrounds"
                );
            }
            if column == 2 {
                assert_eq!(
                    &pixel(&pixels, 10, 0)[..3],
                    ROUTE_COLORS[1],
                    "the crossing vertical route retains its own color"
                );
                assert!(
                    pixel(&pixels, 10, 0)[3] > 200,
                    "the vertical route remains substantially covered"
                );
            }
        }
        let pixels = tile_at(&edges, 2).rasterize(20, 40, [0; 3], [0; 3], false);
        edges.reverse();
        assert_eq!(
            pixels,
            tile_at(&edges, 2).rasterize(20, 40, [0; 3], [0; 3], false),
            "crossing colors do not depend on edge iteration order"
        );

        let mut joined = route_edges(2, &[(0, 0), (2, 0)]);
        joined[0].color = 1;
        joined[1].color = 2;
        let pixels = tile_at(&joined, 0).rasterize(20, 40, [0; 3], [0; 3], false);
        assert_eq!(
            &pixel(&pixels, 10, 39)[..3],
            ROUTE_COLORS[0],
            "an existing route keeps its color below a join"
        );
        assert!(
            pixel(&pixels, 10, 39)[3] > 200,
            "the joined route remains substantially covered"
        );
        assert_eq!(
            pixel(&pixels, 19, 37),
            color(1),
            "the incoming route stays distinct up to the join"
        );
    }

    #[test]
    fn primitive_routes_preserve_colors_at_crossings_and_under_markers() {
        let mut tile = Tile {
            paths: Tile::UP_DOWN | Tile::LEFT_RIGHT,
            ..Tile::default()
        };
        tile.path_colors[0] = 1;
        tile.path_colors[5] = 2;
        let pixels = tile.rasterize(20, 40, [10, 20, 30], [0; 3], false);
        assert_eq!(
            &pixels[10 * 4..10 * 4 + 3],
            ROUTE_COLORS[0],
            "vertical paths retain their color"
        );
        assert_eq!(
            &pixels[(20 * 20 + 10) * 4..(20 * 20 + 10) * 4 + 3],
            ROUTE_COLORS[1],
            "horizontal paths keep their color over crossings"
        );
        tile.marker = '●';
        let pixels = tile.rasterize(20, 40, [10, 20, 30], [0; 3], false);
        assert_eq!(
            &pixels[(20 * 20 + 10) * 4..(20 * 20 + 10) * 4 + 4],
            [10, 20, 30, 255],
            "the marker uses its signature foreground above colored routes"
        );
    }

    #[test]
    fn dimming_preserves_route_colors_and_bright_highlights() {
        let mut tile = Tile {
            paths: Tile::NODE_UP,
            edges: route_edges(0, &[(0, 0)]),
            marker: '●',
            ..Tile::default()
        };
        tile.path_colors[6] = 1;
        tile.edges[0].color = 2;
        let foreground = [40, 80, 120];
        let background = [10, 20, 30];
        let pixels = tile.rasterize(20, 40, foreground, background, true);
        for (y, expected) in [(0, ROUTE_COLORS[0]), (20, foreground), (39, ROUTE_COLORS[1])] {
            assert_eq!(
                &pixels[(y * 20 + 10) * 4..(y * 20 + 10) * 4 + 3],
                std::array::from_fn::<_, 3, _>(|index| u8::midpoint(expected[index], background[index])),
                "DIM blends both independent route colors and the marker toward the background"
            );
        }
        tile.highlighted_paths = Tile::NODE_UP;
        tile.edges[0].highlighted = true;
        let pixels = tile.rasterize(20, 40, foreground, background, true);
        for y in [0, 39] {
            assert_eq!(
                &pixels[(y * 20 + 10) * 4..(y * 20 + 10) * 4 + 4],
                [0, 255, 255, 255],
                "highlighted incoming and outgoing routes remain bright cyan"
            );
        }
    }

    #[test]
    fn connected_nodes_reach_cell_boundaries_but_tips_do_not() {
        let tile = Tile {
            paths: Tile::NODE_UP | Tile::NODE_DOWN,
            marker: '●',
            ..Tile::default()
        };
        let pixels = tile.rasterize(20, 40, [10, 20, 30], [0; 3], false);
        let alpha = |data: &[u8], x: usize, y: usize| data[(y * 20 + x) * 4 + 3];
        assert!(
            alpha(&pixels, 10, 0) > 0 && alpha(&pixels, 10, 39) > 0,
            "linear edges join adjacent rows"
        );
        assert_eq!(alpha(&pixels, 10, 20), 255, "commit circles are filled");
        assert!(
            (0..40).all(|y| alpha(&pixels, 10, y) > 0),
            "lines meet the circle without a gap"
        );
        let tip = Tile {
            paths: Tile::NODE_DOWN,
            ..tile
        }
        .rasterize(20, 40, [10, 20, 30], [0; 3], false);
        assert_eq!(alpha(&tip, 10, 0), 0, "tips have no invented incoming edge");
    }

    #[test]
    fn crossings_and_hollow_nodes_keep_their_gaps() {
        let crossing = Tile {
            paths: Tile::UP_DOWN | Tile::LEFT_RIGHT,
            ..Tile::default()
        };
        let pixels = crossing.rasterize(20, 40, [255; 3], [0; 3], false);
        assert_eq!(pixels[(17 * 20 + 10) * 4 + 3], 0, "a crossing has an underpass gap");
        assert!(
            pixels[(20 * 20 + 10) * 4 + 3] > 0,
            "the crossing horizontal path is continuous"
        );
        let ring = Tile {
            paths: Tile::NODE_UP | Tile::NODE_DOWN,
            marker: '○',
            ..Tile::default()
        }
        .rasterize(20, 40, [255; 3], [0; 3], false);
        assert_eq!(
            ring[(20 * 20 + 10) * 4 + 3],
            0,
            "compressed segments retain a hollow marker"
        );
    }

    #[test]
    fn diagonal_strokes_keep_the_weight_of_vertical_strokes_across_pixel_phases() {
        for (width, height) in [(16u16, 32u16), (15, 30), (13, 26), (20, 40), (16, 35)] {
            let weight = |to| {
                let edges = route_edges(3, &[(0, to)]);
                let mut alpha = 0u32;
                for column in 0..=2 {
                    let pixels = tile_at(&edges, column).rasterize(width, height, [255; 3], [0; 3], false);
                    for row in pixels
                        .chunks_exact(usize::from(width) * 4)
                        .skip(4)
                        .take(usize::from(height) - 8)
                    {
                        alpha += row
                            .as_chunks::<4>()
                            .0
                            .iter()
                            .map(|pixel| u32::from(pixel[3]))
                            .sum::<u32>();
                    }
                }
                let length_per_row = (to as f32 * 2.0 * f32::from(width)).hypot(f32::from(height)) / f32::from(height);
                alpha as f32 / 255.0 / (f32::from(height) - 8.0) / length_per_row
            };
            let vertical = weight(0);
            let diagonal = weight(1);
            assert!(
                (diagonal / vertical - 1.0).abs() < 0.02,
                "diagonal coverage {diagonal} should match vertical coverage {vertical} at {width}x{height}"
            );
        }
    }

    #[test]
    fn moving_edges_follow_the_endpoint_angle_across_cells_and_rows() {
        for (width, height) in [(8, 16), (20, 40), (13, 29)] {
            for (from, to) in [(0, 1), (1, 2), (0, 4), (4, 0)] {
                for from_node in [false, true] {
                    let edges = route_edges(if from_node { from } else { 5 }, &[(from, to)]);
                    let start_x = (from as f32 * 2.0 + 0.5) * f32::from(width);
                    let end_x = (to as f32 * 2.0 + 0.5) * f32::from(width);
                    let start_y = if from_node { f32::from(height) / 2.0 } else { 0.0 };
                    for column in from.min(to) * 2..=from.max(to) * 2 {
                        let pixels = tile_at(&edges, column as i32).rasterize(width, height, [255; 3], [0; 3], false);
                        for x in 0..width {
                            let global_x = (column as f32 * f32::from(width)) + f32::from(x) + 0.5;
                            let t = (global_x - start_x) / (end_x - start_x);
                            if !(0.0..1.0).contains(&t) {
                                continue;
                            }
                            let y = (start_y + t * (f32::from(height) - start_y)).floor() as usize;
                            assert!(
                                pixels[(y * usize::from(width) + usize::from(x)) * 4 + 3] > 0,
                                "the direct route stays continuous across every cell at its actual slope"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn crossings_keep_the_diagonal_path_continuous_regardless_of_edge_order() {
        let mut edges = route_edges(3, &[(0, 2), (1, 1)]);
        let pixels = tile_at(&edges, 2).rasterize(20, 40, [255; 3], [0; 3], false);
        assert_eq!(
            pixels[(16 * 20 + 10) * 4 + 3],
            0,
            "the vertical path has an underpass gap"
        );
        assert!(
            (0..20).all(|x| pixels[((15 + x / 2) * 20 + x) * 4 + 3] > 0),
            "the crossing keeps its angle through the whole cell"
        );
        edges.reverse();
        assert_eq!(
            pixels,
            tile_at(&edges, 2).rasterize(20, 40, [255; 3], [0; 3], false),
            "edge order cannot swap the overpass"
        );

        let mut node = tile_at(&route_edges(1, &[(0, 3)]), 2);
        node.paths = Tile::NODE_UP;
        node.marker = '●';
        let pixels = node.rasterize(20, 40, [255; 3], [0; 3], false);
        assert_eq!(
            pixels[(10 * 20 + 10) * 4 + 3],
            0,
            "incoming node stems also pass underneath unrelated lanes"
        );
    }

    #[test]
    fn adjacent_and_swapped_lanes_do_not_share_unrelated_vertical_runs() {
        let adjacent = route_edges(3, &[(0, 1), (1, 2)]);
        let pixels = tile_at(&adjacent, 2).rasterize(20, 40, [255; 3], [0; 3], false);
        assert_eq!(
            pixels[(9 * 20 + 10) * 4 + 3],
            0,
            "arrival and departure in one column stay separate"
        );

        let swapped = route_edges(0, &[(2, 1), (1, 2), (0, 0), (0, 1), (0, 2)]);
        let arrival = &swapped[..1];
        let center = tile_at(arrival, 2).rasterize(20, 40, [255; 3], [0; 3], false);
        let beside = tile_at(arrival, 3).rasterize(20, 40, [255; 3], [0; 3], false);
        assert_eq!(
            center[(20 * 20 + 10) * 4 + 3],
            0,
            "a swapped arrival avoids the departing lane's column"
        );
        assert!(
            beside[(20 * 20 + 10) * 4 + 3] > 0,
            "the arrival remains visible beside that column"
        );
        assert!(
            center[(39 * 20 + 10) * 4 + 3] > 0,
            "the direct line reaches its destination at the next row"
        );
    }

    #[test]
    fn opposite_diagonals_keep_crossing_priority_when_edge_order_changes() {
        for lanes in [1, 20] {
            let mut edges = route_edges(lanes + 1, &[(0, lanes), (lanes, 0)]);
            edges[0].color = 1;
            edges[1].color = 2;
            for highlighted in [false, true] {
                edges[0].highlighted = highlighted;
                let pixels = tile_at(&edges, lanes as i32).rasterize(40, 80, [255; 3], [0; 3], false);
                edges.reverse();
                assert!(
                    pixels == tile_at(&edges, lanes as i32).rasterize(40, 80, [255; 3], [0; 3], false),
                    "perpendicular and shallow crossings keep their underpass and colors regardless of edge order"
                );
                edges.reverse();
            }
        }
    }

    #[test]
    fn selected_edges_cross_over_without_highlighting_the_other_route() {
        let mut edges = route_edges(3, &[(0, 2), (1, 1)]);
        edges[1].highlighted = true;
        let pixels = tile_at(&edges, 2).rasterize(20, 40, [255; 3], [0; 3], false);
        let pixel = |x: usize, y: usize| &pixels[(y * 20 + x) * 4..(y * 20 + x + 1) * 4];
        assert!(
            (0..40).all(|y| pixel(10, y) == [0, 255, 255, 255]),
            "the chosen vertical path stays cyan and continuous through the crossing"
        );
        assert_eq!(pixel(0, 15), [255; 4], "the unrelated diagonal route keeps its color");
        assert_eq!(
            pixel(6, 18)[3],
            0,
            "the unrelated route passes underneath the chosen path"
        );
        assert!(
            pixel(8, 0)[3] > 0,
            "the chosen line is thicker than ordinary graph edges"
        );
        edges.reverse();
        assert_eq!(
            pixels,
            tile_at(&edges, 2).rasterize(20, 40, [255; 3], [0; 3], false),
            "selection takes crossing priority independently of edge order"
        );
    }

    #[test]
    fn simple_path_highlights_preserve_node_colors_and_hollow_markers() {
        let tile = Tile {
            paths: Tile::UP_DOWN | Tile::LEFT_RIGHT,
            highlighted_paths: Tile::UP_DOWN,
            ..Tile::default()
        };
        let pixels = tile.rasterize(20, 40, [255; 3], [0; 3], false);
        assert_eq!(
            pixels[(20 * 20 + 5) * 4 + 3],
            0,
            "simple paths give selection crossing priority too"
        );
        for marker in ['●', '○', '@'] {
            let tile = Tile {
                paths: Tile::NODE_UP | Tile::NODE_DOWN,
                highlighted_paths: Tile::NODE_UP,
                marker,
                ..Tile::default()
            };
            let pixels = tile.rasterize(20, 40, [255, 165, 0], [0; 3], false);
            assert_eq!(
                &pixels[10 * 4..11 * 4],
                [0, 255, 255, 255],
                "the incoming selected stem is cyan"
            );
            assert_eq!(
                &pixels[(39 * 20 + 10) * 4..(39 * 20 + 10) * 4 + 3],
                [255, 165, 0],
                "the unselected outgoing stem retains its color"
            );
            assert!(
                pixels[(39 * 20 + 10) * 4 + 3] > 200,
                "the unselected outgoing stem remains substantially covered"
            );
            let center = &pixels[(20 * 20 + 10) * 4..(20 * 20 + 11) * 4];
            if marker == '●' {
                assert_eq!(
                    center,
                    [255, 165, 0, 255],
                    "selection preserves the signature marker color"
                );
            } else {
                assert_eq!(center[3], 0, "hollow markers and text retain their clearance");
            }
        }
    }

    #[test]
    fn shifting_lanes_do_not_join_an_unrelated_fanout() {
        let tile = Tile {
            paths: Tile::UP_RIGHT | Tile::DOWN_LEFT | Tile::LEFT_RIGHT,
            edges: vec![
                Edge {
                    from: 0,
                    to: 4,
                    from_offset: 0,
                    to_offset: 0,
                    from_node: false,
                    to_node: false,
                    start_vertical: false,
                    end_vertical: false,
                    highlighted: false,
                    color: 0,
                    incoming: None,
                    outgoing: None,
                },
                Edge {
                    from: -2,
                    to: 0,
                    from_offset: 0,
                    to_offset: 0,
                    from_node: true,
                    to_node: false,
                    start_vertical: false,
                    end_vertical: false,
                    highlighted: false,
                    color: 0,
                    incoming: None,
                    outgoing: None,
                },
                Edge {
                    from: -2,
                    to: 2,
                    from_offset: 0,
                    to_offset: 0,
                    from_node: true,
                    to_node: false,
                    start_vertical: false,
                    end_vertical: false,
                    highlighted: false,
                    color: 0,
                    incoming: None,
                    outgoing: None,
                },
            ],
            ..Tile::default()
        };
        let pixels = tile.rasterize(20, 40, [255; 3], [0; 3], false);
        assert!(pixels[10 * 4 + 3] > 0, "the unrelated lane enters from above");
        assert!(
            pixels[(39 * 20 + 10) * 4 + 3] > 0,
            "the fanout reaches its parent below"
        );
        assert_eq!(
            pixels[(20 * 20 + 19) * 4 + 3],
            0,
            "the old shared horizontal junction is absent"
        );
    }
}
