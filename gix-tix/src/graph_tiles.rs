//! Cell-sized graph geometry, carried through Ratatui as private-use symbols.

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Tile {
    pub paths: u16,
    pub marker: char,
    pub edges: Vec<Edge>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Edge {
    pub from: i32,
    pub to: i32,
    pub from_node: bool,
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            paths: 0,
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
        for edge in &self.edges {
            for value in [edge.from, edge.to, i32::from(edge.from_node)] {
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
        if shift != 0 || values.len() % 3 != 0 {
            return None;
        }
        let mut edges = Vec::with_capacity(values.len() / 3);
        for parts in values.as_chunks::<3>().0 {
            if !(0..=1).contains(&parts[2]) {
                return None;
            }
            edges.push(Edge {
                from: parts[0],
                to: parts[1],
                from_node: parts[2] != 0,
            });
        }
        Some(Self {
            edges,
            paths: (value / 16) as u16,
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
                    paths: 0,
                    marker: symbol,
                    edges: Vec::new(),
                });
            }
            _ => return None,
        };
        Some(Self {
            paths,
            marker: ' ',
            edges: Vec::new(),
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

    pub fn rasterize(&self, width: u16, height: u16, foreground: [u8; 3], _background: [u8; 3]) -> Vec<u8> {
        let (w, h) = (f32::from(width), f32::from(height));
        let center = (w / 2.0, h / 2.0);
        let up = (center.0, 0.0);
        let down = (center.0, h);
        let left = (0.0, center.1);
        let right = (w, center.1);
        let stroke = (w / 9.0).max(1.0);
        let radius = w * 0.32;
        let curves: Vec<_> = self
            .edges
            .iter()
            .map(|edge| {
                let start = ((edge.from as f32 + 0.5) * w, if edge.from_node { h / 2.0 } else { 0.0 });
                let end = ((edge.to as f32 + 0.5) * w, h);
                let middle = start.1.midpoint(end.1);
                let mut points = Vec::with_capacity(33);
                for step in 0..=32 {
                    let t = step as f32 / 32.0;
                    let s = 1.0 - t;
                    points.push((
                        (s * s * s + 3.0 * s * s * t) * start.0 + (3.0 * s * t * t + t * t * t) * end.0,
                        s * s * s * start.1 + 3.0 * s * t * middle + t * t * t * end.1,
                    ));
                }
                points
            })
            .collect();
        let mut gaps: Vec<Vec<(f32, f32)>> = vec![Vec::new(); curves.len()];
        for (index, curve) in curves.iter().enumerate() {
            for (other_index, other) in curves[..index].iter().enumerate() {
                let edge = &self.edges[index];
                let other_edge = &self.edges[other_index];
                if edge.to == other_edge.to || (edge.from == other_edge.from && edge.from_node == other_edge.from_node)
                {
                    continue;
                }
                for a in curve.windows(2) {
                    for b in other.windows(2) {
                        if let Some(crossing) = intersection(a[0], a[1], b[0], b[1]) {
                            gaps[index].push(crossing);
                        }
                    }
                }
            }
        }
        let paths = if self.edges.is_empty() {
            self.paths
        } else {
            self.paths & Self::NODE_UP
        };
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
                let mut previous = start;
                for step in 1..=12 {
                    let t = step as f32 / 12.0;
                    let s = 1.0 - t;
                    let point = (
                        s * s * start.0 + 2.0 * s * t * center.0 + t * t * end.0,
                        s * s * start.1 + 2.0 * s * t * center.1 + t * t * end.1,
                    );
                    segments.push((mask, previous, point));
                    previous = point;
                }
            } else {
                segments.push((mask, start, end));
            }
        }

        let crossing = paths & (Self::UP_DOWN | Self::LEFT_RIGHT) == Self::UP_DOWN | Self::LEFT_RIGHT;
        let mut pixels = Vec::with_capacity(usize::from(width) * usize::from(height) * 4);
        for y in 0..height {
            for x in 0..width {
                let point = (f32::from(x) + 0.5, f32::from(y) + 0.5);
                let delta = (point.0 - center.0, point.1 - center.1);
                let circle = delta.0.hypot(delta.1);
                let mut coverage = 0.0f32;
                for &(mask, start, end) in &segments {
                    // A gap under the horizontal path distinguishes a crossing from a junction.
                    if crossing && mask == Self::UP_DOWN && delta.1.abs() < stroke * 2.0 {
                        continue;
                    }
                    let edge = (stroke / 2.0 + 0.5 - distance_to_segment(point, start, end)).clamp(0.0, 1.0);
                    coverage = coverage.max(edge);
                }
                for (index, curve) in curves.iter().enumerate() {
                    if gaps[index]
                        .iter()
                        .any(|gap| (point.0 - gap.0).hypot(point.1 - gap.1) < stroke * 2.0)
                    {
                        continue;
                    }
                    let distance = curve
                        .windows(2)
                        .map(|p| distance_to_segment(point, p[0], p[1]))
                        .fold(f32::INFINITY, f32::min);
                    coverage = coverage.max((stroke / 2.0 + 0.5 - distance).clamp(0.0, 1.0));
                }
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
                        coverage *= (shape_distance - clearance + 0.5).clamp(0.0, 1.0);
                    }
                    let node = match self.marker {
                        '●' | '◆' => (radius + 0.5 - shape_distance).clamp(0.0, 1.0),
                        '○' => (stroke / 2.0 + 0.5 - (circle - radius).abs()).clamp(0.0, 1.0),
                        _ => 0.0,
                    };
                    coverage = coverage.max(node);
                }
                pixels.extend_from_slice(&[
                    foreground[0],
                    foreground[1],
                    foreground[2],
                    (coverage * 255.0).round() as u8,
                ]);
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
    use super::{Edge, Tile};

    #[test]
    fn tiles_round_trip_without_confusing_ordinary_text() {
        for paths in 0..1024 {
            for marker in [' ', '●', '○', '◆', '@', '0', '9', '+'] {
                let tile = Tile {
                    paths,
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
            }],
            ..Tile::default()
        };
        assert_eq!(
            Tile::decode(&tile.encode()),
            Some(tile),
            "curve coordinates round trip without truncation"
        );
        assert_eq!(
            Tile::decode("\u{f0000}\u{e0140}"),
            None,
            "truncated curve data is rejected"
        );
    }

    #[test]
    fn connected_nodes_reach_cell_boundaries_but_tips_do_not() {
        let tile = Tile {
            paths: Tile::NODE_UP | Tile::NODE_DOWN,
            marker: '●',
            ..Tile::default()
        };
        let pixels = tile.rasterize(20, 40, [10, 20, 30], [0; 3]);
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
        .rasterize(20, 40, [10, 20, 30], [0; 3]);
        assert_eq!(alpha(&tip, 10, 0), 0, "tips have no invented incoming edge");
    }

    #[test]
    fn crossings_and_hollow_nodes_keep_their_gaps() {
        let crossing = Tile {
            paths: Tile::UP_DOWN | Tile::LEFT_RIGHT,
            ..Tile::default()
        };
        let pixels = crossing.rasterize(20, 40, [255; 3], [0; 3]);
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
        .rasterize(20, 40, [255; 3], [0; 3]);
        assert_eq!(
            ring[(20 * 20 + 10) * 4 + 3],
            0,
            "compressed segments retain a hollow marker"
        );
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
                },
                Edge {
                    from: -2,
                    to: 0,
                    from_node: true,
                },
                Edge {
                    from: -2,
                    to: 2,
                    from_node: true,
                },
            ],
            ..Tile::default()
        };
        let pixels = tile.rasterize(20, 40, [255; 3], [0; 3]);
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
