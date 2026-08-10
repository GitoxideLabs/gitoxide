use std::{collections::HashMap, time::Duration};

use gix::ObjectId;
use ratatui::{buffer::Buffer, layout::Rect, style::Modifier};

use crate::ui::FrameLayout;

const EMPHASIS_DURATION: Duration = Duration::from_millis(180);

#[derive(Clone, Debug)]
pub(crate) struct Row {
    pub id: ObjectId,
    pub tree: Option<ObjectId>,
    pub y: u16,
}

#[derive(Clone, Debug)]
pub(crate) struct Snapshot {
    pub buffer: Buffer,
    pub layout: FrameLayout,
    pub rows: Vec<Row>,
}

impl Snapshot {
    pub(crate) fn new(buffer: Buffer, layout: FrameLayout) -> Self {
        let rows = layout
            .rows
            .iter()
            .map(|(id, y)| Row {
                id: *id,
                tree: None,
                y: *y,
            })
            .collect();
        Snapshot { buffer, layout, rows }
    }

    pub(crate) fn set_trees(&mut self, trees: &HashMap<ObjectId, ObjectId>) {
        for row in &mut self.rows {
            row.tree = trees.get(&row.id).copied();
        }
    }
}

#[derive(Debug)]
pub(crate) struct Emphasis {
    target: Snapshot,
    displayed: Buffer,
    remaining: Duration,
}

impl Emphasis {
    pub(crate) fn new(source: Snapshot, target: Snapshot) -> Option<Self> {
        if source.buffer.area != target.buffer.area || source.buffer == target.buffer {
            return None;
        }
        let matches = row_matches(&source.rows, &target.rows);
        if matches.is_empty() {
            return None;
        }
        let mut displayed = target.buffer.clone();
        for (target_index, target_row) in target.rows.iter().enumerate() {
            let changed = matches
                .iter()
                .find(|(_, candidate)| *candidate == target_index)
                .is_none_or(|(source_index, _)| source.rows[*source_index].id != target_row.id);
            if changed && !covered_by_overlay(target_row.y, &target.layout.overlays) {
                modify_row(&mut displayed, target.layout.history, target_row.y, Modifier::BOLD);
            }
        }
        (displayed != target.buffer).then_some(Emphasis {
            target,
            displayed,
            remaining: EMPHASIS_DURATION,
        })
    }

    pub(crate) fn target(&self) -> &Snapshot {
        &self.target
    }

    pub(crate) fn displayed(&self) -> &Buffer {
        &self.displayed
    }

    pub(crate) fn timeout(&self) -> Duration {
        self.remaining
    }

    pub(crate) fn advance(&mut self, elapsed: Duration) -> Option<&Buffer> {
        self.remaining = self.remaining.saturating_sub(elapsed);
        if !self.is_complete() {
            return None;
        }
        self.displayed.clone_from(&self.target.buffer);
        Some(&self.displayed)
    }

    pub(crate) fn is_complete(&self) -> bool {
        self.remaining == Duration::ZERO
    }
}

fn row_matches(source: &[Row], target: &[Row]) -> Vec<(usize, usize)> {
    let width = target.len() + 1;
    let mut scores = vec![0u16; (source.len() + 1) * width];
    for source_index in (0..source.len()).rev() {
        for target_index in (0..target.len()).rev() {
            let same = row_score(&source[source_index], &target[target_index]);
            scores[source_index * width + target_index] = if same > 0 {
                (same + scores[(source_index + 1) * width + target_index + 1])
                    .max(scores[(source_index + 1) * width + target_index])
                    .max(scores[source_index * width + target_index + 1])
            } else {
                scores[(source_index + 1) * width + target_index].max(scores[source_index * width + target_index + 1])
            };
        }
    }
    let mut out = Vec::new();
    let (mut source_index, mut target_index) = (0, 0);
    while source_index < source.len() && target_index < target.len() {
        let same = row_score(&source[source_index], &target[target_index]);
        if same > 0
            && scores[source_index * width + target_index]
                == same + scores[(source_index + 1) * width + target_index + 1]
        {
            out.push((source_index, target_index));
            source_index += 1;
            target_index += 1;
        } else if scores[(source_index + 1) * width + target_index] >= scores[source_index * width + target_index + 1] {
            source_index += 1;
        } else {
            target_index += 1;
        }
    }
    out
}

fn row_score(source: &Row, target: &Row) -> u16 {
    if source.id == target.id {
        2
    } else if source.tree.is_some() && source.tree == target.tree {
        1
    } else {
        0
    }
}

fn covered_by_overlay(y: u16, overlays: &[Rect]) -> bool {
    overlays.iter().any(|area| y >= area.y && y < area.bottom())
}

fn modify_row(buffer: &mut Buffer, area: Rect, y: u16, modifier: Modifier) {
    if y < area.y || y >= area.bottom() {
        return;
    }
    for x in area.x.max(buffer.area.x)..area.right().min(buffer.area.right()) {
        buffer[(x, y)].modifier.insert(modifier);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> ObjectId {
        ObjectId::from_bytes_or_panic(&[n; 20])
    }

    fn snapshot(ids: &[u8], trees: &[u8]) -> Snapshot {
        history_snapshot(
            &ids.iter()
                .zip(trees)
                .map(|(id, tree)| (*id, *tree, id.to_string()))
                .collect::<Vec<_>>(),
        )
    }

    fn history_snapshot(rows: &[(u8, u8, String)]) -> Snapshot {
        let area = Rect::new(0, 0, 24, 4);
        let mut buffer = Buffer::empty(area);
        let rows = rows
            .iter()
            .enumerate()
            .map(|(index, (id_value, tree, text))| {
                for (x, symbol) in text.chars().take(area.width.into()).enumerate() {
                    buffer[(x as u16, index as u16)].set_char(symbol);
                }
                Row {
                    id: id(*id_value),
                    tree: Some(id(*tree)),
                    y: index as u16,
                }
            })
            .collect();
        Snapshot {
            buffer,
            layout: FrameLayout {
                history: area,
                ..FrameLayout::default()
            },
            rows,
        }
    }

    fn distinct_frames(mut emphasis: Emphasis) -> String {
        let first = emphasis.displayed().clone();
        assert!(
            emphasis
                .advance(EMPHASIS_DURATION.saturating_sub(Duration::from_nanos(1)))
                .is_none(),
            "hold ticks do not produce duplicate frames"
        );
        let final_frame = emphasis
            .advance(Duration::from_nanos(1))
            .expect("the emphasis settles after its hold")
            .clone();
        [first, final_frame]
            .iter()
            .enumerate()
            .map(|(index, frame)| format!("--- frame {index} ---\n{}", display_frame(frame)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn display_frame(buffer: &Buffer) -> String {
        let mut out = String::new();
        for y in buffer.area.y..buffer.area.bottom() {
            let cells = (buffer.area.x..buffer.area.right())
                .map(|x| &buffer[(x, y)])
                .collect::<Vec<_>>();
            let text = cells.iter().map(|cell| cell.symbol()).collect::<String>();
            out.push_str(text.trim_end());
            out.push('\n');
            let modifiers = cells
                .iter()
                .map(|cell| {
                    if cell.modifier.contains(Modifier::BOLD) {
                        'b'
                    } else {
                        ' '
                    }
                })
                .collect::<String>();
            if !modifiers.trim().is_empty() {
                out.push_str("style: ");
                out.push_str(modifiers.trim_end());
                out.push('\n');
            }
        }
        out
    }

    #[test]
    fn rewritten_rows_match_by_tree_even_if_the_author_changed_with_the_id() {
        let source = snapshot(&[1, 2], &[10, 20]);
        let target = snapshot(&[3, 2], &[10, 20]);
        assert_eq!(row_matches(&source.rows, &target.rows), [(0, 0), (1, 1)]);
    }

    #[test]
    fn duplicate_trees_are_matched_in_relative_order() {
        let source = snapshot(&[1, 2, 3], &[10, 10, 20]);
        let target = snapshot(&[4, 5, 3], &[10, 10, 20]);
        assert_eq!(row_matches(&source.rows, &target.rows), [(0, 0), (1, 1), (2, 2)]);
    }

    #[test]
    fn removals_and_unrelated_replacements_are_immediate() {
        let source = snapshot(&[1, 2], &[10, 20]);
        let removal = snapshot(&[2], &[20]);
        assert!(
            Emphasis::new(source, removal).is_none(),
            "a removal has no new row to emphasize"
        );

        let source = snapshot(&[1], &[10]);
        let unrelated = snapshot(&[2], &[20]);
        assert!(
            Emphasis::new(source, unrelated).is_none(),
            "an unrelated history has no visual anchor"
        );
    }

    #[test]
    fn filesystem_reword_frames() {
        let source = history_snapshot(&[(1, 10, "111 old title".into()), (2, 20, "222 unchanged".into())]);
        let target = history_snapshot(&[(3, 10, "333 new title".into()), (2, 20, "222 unchanged".into())]);
        insta::assert_snapshot!(distinct_frames(
            Emphasis::new(source, target).expect("a reword is emphasized")
        ));
    }

    #[test]
    fn filesystem_new_top_commit_frames() {
        let source = history_snapshot(&[
            (1, 10, "111 first".into()),
            (2, 20, "222 second".into()),
            (3, 30, "333 third".into()),
        ]);
        let target = history_snapshot(&[
            (4, 40, "444 new commit".into()),
            (1, 10, "111 first".into()),
            (2, 20, "222 second".into()),
            (3, 30, "333 third".into()),
        ]);
        insta::assert_snapshot!(distinct_frames(
            Emphasis::new(source, target).expect("a new top commit is emphasized")
        ));
    }
}
