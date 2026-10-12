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

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Tile {
    pub paths: u16,
    pub highlighted_paths: u16,
    pub path_colors: [u8; 10],
    pub marker: char,
    pub edges: Vec<Edge>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Edge {
    pub from: i32,
    pub to: i32,
    pub from_node: bool,
    pub bend: u16,
    pub detour: bool,
    pub highlighted: bool,
    pub color: u8,
}

pub(crate) fn route_edges(current: usize, edges: &[(usize, usize)]) -> Vec<Edge> {
    let mut shifts: Vec<_> = edges
        .iter()
        .copied()
        .filter(|(from, to)| from != to && *from != current)
        .collect();
    shifts.sort_unstable_by_key(|&(from, to)| (from < to, if from < to { usize::MAX - from } else { from }));
    let mut routed: Vec<_> = edges
        .iter()
        .map(|&(from, to)| Edge {
            from: (from * 2) as i32,
            to: (to * 2) as i32,
            from_node: from == current,
            bend: if from == current {
                u16::MAX / 4 * 3
            } else {
                shifts.iter().position(|edge| *edge == (from, to)).map_or(0, |rank| {
                    (usize::from(u16::MAX) * (rank + 1) / (2 * (shifts.len() + 1))) as u16
                })
            },
            detour: false,
            highlighted: false,
            color: 0,
        })
        .collect();
    for index in 0..routed.len() {
        let edge = &routed[index];
        let detour = edge.from != edge.to
            && (edge.to == (current * 2) as i32
                || routed
                    .iter()
                    .any(|other| other.from == edge.to && other.to != edge.to && other.bend >= edge.bend));
        // Rejoin below the other lane's departure instead of sharing an unrelated vertical run.
        routed[index].detour = detour;
    }
    routed
}

impl Edge {
    fn points(&self, width: f32, height: f32) -> Vec<(f32, f32)> {
        let start = (
            (self.from as f32 + 0.5) * width,
            if self.from_node { height / 2.0 } else { 0.0 },
        );
        let end = ((self.to as f32 + 0.5) * width, height);
        if self.from == self.to {
            return vec![start, end];
        }
        let bend = f32::from(self.bend) / f32::from(u16::MAX) * height;
        let radius = width * 0.16;
        let mut corners = vec![start, (start.0, bend)];
        if self.detour {
            let side = end.0 + (start.0 - end.0).signum() * width * 0.75;
            let join = height * 0.92;
            corners.extend([(side, bend), (side, join), (end.0, join)]);
        } else {
            corners.push((end.0, bend));
        }
        corners.push(end);
        rounded(&corners, radius)
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
        points.push(start);
        for step in 1..=6 {
            let t = step as f32 / 6.0;
            let s = 1.0 - t;
            points.push((
                s * s * start.0 + 2.0 * s * t * corner[1].0 + t * t * end.0,
                s * s * start.1 + 2.0 * s * t * corner[1].1 + t * t * end.1,
            ));
        }
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
        for value in std::iter::once(i32::from(self.highlighted_paths))
            .chain(
                self.path_colors
                    .iter()
                    .enumerate()
                    .filter_map(|(index, color)| (self.paths & (1 << index) != 0).then_some(i32::from(*color))),
            )
            .chain(self.edges.iter().flat_map(|edge| {
                [
                    edge.from,
                    edge.to,
                    i32::from(edge.from_node) | (i32::from(edge.highlighted) << 1) | (i32::from(edge.color) << 2),
                    i32::from(edge.bend),
                    i32::from(edge.detour),
                ]
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
        let (&highlighted_paths, values) = values.split_first()?;
        let paths = (value / 16) as u16;
        let (colors, values) = values.split_at_checked(paths.count_ones() as usize)?;
        if shift != 0 || !(0..1024).contains(&highlighted_paths) || values.len() % 5 != 0 {
            return None;
        }
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
        let mut edges = Vec::with_capacity(values.len() / 5);
        for parts in values.as_chunks::<5>().0 {
            if !(0..=(ROUTE_COLORS.len() as i32 * 4 + 3)).contains(&parts[2])
                || !(0..=u16::MAX as i32).contains(&parts[3])
                || !(0..=1).contains(&parts[4])
            {
                return None;
            }
            edges.push(Edge {
                from: parts[0],
                to: parts[1],
                from_node: parts[2] & 1 != 0,
                highlighted: parts[2] & 2 != 0,
                color: (parts[2] >> 2) as u8,
                bend: parts[3] as u16,
                detour: parts[4] != 0,
            });
        }
        Some(Self {
            edges,
            paths,
            highlighted_paths: highlighted_paths as u16,
            path_colors,
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
        let mut lines: Vec<_> = self.edges.iter().map(|edge| edge.points(w, h)).collect();
        if !self.edges.is_empty() && self.paths & Self::NODE_UP != 0 {
            lines.push(vec![up, center]);
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
                    || other_edge.from_node && other_edge.from == 0,
                    |edge| {
                        edge.to == other_edge.to
                            || (edge.from == other_edge.from && edge.from_node == other_edge.from_node)
                    },
                );
                if connected {
                    continue;
                }
                for a in line.windows(2) {
                    for b in other.windows(2) {
                        if let Some(crossing) = intersection(a[0], a[1], b[0], b[1]) {
                            // Keep the chosen route continuous; otherwise the more vertical path passes underneath.
                            let underneath = if highlighted(index) != highlighted(other_index) {
                                if highlighted(index) { other_index } else { index }
                            } else if ((a[1].1 - a[0].1) * (b[1].0 - b[0].0)).abs()
                                >= ((b[1].1 - b[0].1) * (a[1].0 - a[0].0)).abs()
                            {
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
                            gaps[underneath].push((crossing, clearance));
                        }
                    }
                }
            }
        }
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
            if curved {
                for points in rounded(&[start, center, end], w * 0.16).windows(2) {
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
                let mut coverage = [0.0f32; 2];
                let mut normal = foreground;
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
                    let edge = (half_stroke + 0.5 - distance_to_segment(point, start, end)).clamp(0.0, 1.0);
                    if !selected && edge > coverage[0] {
                        normal = colors[usize::from(self.path_colors[mask.trailing_zeros() as usize])];
                    }
                    coverage[usize::from(selected)] = coverage[usize::from(selected)].max(edge);
                }
                for (index, line) in lines.iter().enumerate() {
                    if gaps[index]
                        .iter()
                        .any(|(gap, clearance)| (point.0 - gap.0).hypot(point.1 - gap.1) < *clearance)
                    {
                        continue;
                    }
                    let distance = line
                        .windows(2)
                        .map(|p| distance_to_segment(point, p[0], p[1]))
                        .fold(f32::INFINITY, f32::min);
                    let selected = highlighted(index);
                    let half_stroke = stroke * if selected { 1.0 } else { 0.5 };
                    let edge = (half_stroke + 0.5 - distance).clamp(0.0, 1.0);
                    if !selected && edge > coverage[0] {
                        normal =
                            colors[usize::from(self.edges.get(index).map_or(self.path_colors[6], |edge| edge.color))];
                    }
                    coverage[usize::from(selected)] = coverage[usize::from(selected)].max(edge);
                }
                let mut node = 0.0;
                if self.marker != ' ' {
                    let shape_distance = if self.marker == '◆' {
                        (delta.0.abs() + delta.1.abs()) / std::f32::consts::SQRT_2
                    } else {
                        circle
                    };
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

#[cfg(test)]
mod tests {
    use super::{Edge, ROUTE_COLORS, Tile, route_edges};

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
                from_node: true,
                bend: u16::MAX,
                detour: true,
                highlighted: true,
                color: ROUTE_COLORS.len() as u8,
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
                assert_eq!(
                    pixel(&pixels, x, 10),
                    color(0),
                    "one route keeps its color across cells with different foregrounds"
                );
            }
            if column == 2 {
                assert_eq!(
                    pixel(&pixels, 10, 0),
                    color(1),
                    "the crossing vertical route retains its own color"
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
            pixel(&pixels, 10, 39),
            color(0),
            "an existing route keeps its color below a join"
        );
        assert_eq!(
            pixel(&pixels, 19, 30),
            color(1),
            "the incoming route stays distinct up to the join"
        );
        let incumbent = tile_at(&joined[..1], 0).rasterize(20, 40, [0; 3], [0; 3], false);
        assert_eq!(
            &pixels[35 * 20 * 4..],
            &incumbent[35 * 20 * 4..],
            "the incumbent owns antialiased stroke boundaries as well as the center of the shared tail"
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
    fn moving_edges_round_only_corners_and_join_across_cells() {
        let edges = route_edges(3, &[(0, 2)]);
        let first = tile_at(&edges, 0).rasterize(20, 40, [255; 3], [0; 3], false);
        assert!(first[(3 * 20 + 10) * 4 + 3] > 0, "the departing stem remains vertical");
        assert_eq!(
            first[(3 * 20 + 12) * 4 + 3],
            0,
            "the stem does not bow towards its destination"
        );
        for column in 1..4 {
            let pixels = tile_at(&edges, column).rasterize(20, 40, [255; 3], [0; 3], false);
            for x in 0..20 {
                assert!(
                    pixels[(10 * 20 + x) * 4 + 3] > 0,
                    "the horizontal run reaches both cell boundaries"
                );
                assert_eq!(
                    pixels[(6 * 20 + x) * 4 + 3],
                    0,
                    "middle cells contain a straight horizontal run"
                );
            }
        }
    }

    #[test]
    fn crossings_keep_the_horizontal_path_continuous_regardless_of_edge_order() {
        let mut edges = route_edges(3, &[(0, 2), (1, 1)]);
        let pixels = tile_at(&edges, 2).rasterize(20, 40, [255; 3], [0; 3], false);
        assert_eq!(
            pixels[(6 * 20 + 10) * 4 + 3],
            0,
            "the vertical path has an underpass gap"
        );
        assert!(
            (0..20).all(|x| pixels[(10 * 20 + x) * 4 + 3] > 0),
            "the crossing stays horizontal through its whole cell"
        );
        edges.reverse();
        assert_eq!(
            pixels,
            tile_at(&edges, 2).rasterize(20, 40, [255; 3], [0; 3], false),
            "edge order cannot swap the overpass"
        );

        let mut node = tile_at(&route_edges(1, &[(0, 2)]), 2);
        node.paths = Tile::NODE_UP;
        node.marker = '●';
        let pixels = node.rasterize(20, 40, [255; 3], [0; 3], false);
        assert_eq!(
            pixels[(6 * 20 + 10) * 4 + 3],
            0,
            "incoming node stems also pass underneath unrelated lanes"
        );
    }

    #[test]
    fn adjacent_and_swapped_lanes_do_not_share_unrelated_vertical_runs() {
        let adjacent = route_edges(3, &[(0, 1), (1, 2)]);
        assert!(
            adjacent.iter().all(|edge| !edge.detour),
            "ordered bends suffice for neighboring shifts"
        );
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
            beside[(20 * 20 + 5) * 4 + 3] > 0,
            "the arrival remains visible beside that column"
        );
        assert!(
            center[(39 * 20 + 10) * 4 + 3] > 0,
            "the detour rejoins the destination before the next row"
        );
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
        assert_eq!(pixel(0, 10), [255; 4], "the unrelated horizontal route keeps its color");
        assert_eq!(
            pixel(5, 10)[3],
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
                &pixels[(39 * 20 + 10) * 4..(39 * 20 + 11) * 4],
                [255, 165, 0, 255],
                "the unselected outgoing stem retains its color"
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
                    from_node: false,
                    bend: u16::MAX / 4,
                    detour: false,
                    highlighted: false,
                    color: 0,
                },
                Edge {
                    from: -2,
                    to: 0,
                    from_node: true,
                    bend: u16::MAX / 4 * 3,
                    detour: false,
                    highlighted: false,
                    color: 0,
                },
                Edge {
                    from: -2,
                    to: 2,
                    from_node: true,
                    bend: u16::MAX / 4 * 3,
                    detour: false,
                    highlighted: false,
                    color: 0,
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
