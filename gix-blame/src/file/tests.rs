use std::ops::Range;

use gix_hash::ObjectId;

use crate::file::UnblamedHunk;

impl From<(Range<u32>, ObjectId)> for UnblamedHunk {
    fn from(value: (Range<u32>, ObjectId)) -> Self {
        let (range_in_blamed_file, suspect) = value;
        let range_in_destination = range_in_blamed_file.clone();

        (range_in_blamed_file, suspect, range_in_destination).into()
    }
}

impl From<(Range<u32>, ObjectId, Range<u32>)> for UnblamedHunk {
    fn from(value: (Range<u32>, ObjectId, Range<u32>)) -> Self {
        let (range_in_blamed_file, suspect, range_in_destination) = value;

        assert!(
            range_in_blamed_file.end > range_in_blamed_file.start,
            "{range_in_blamed_file:?}"
        );
        assert!(
            range_in_destination.end > range_in_destination.start,
            "{range_in_destination:?}"
        );
        assert_eq!(range_in_blamed_file.len(), range_in_destination.len());

        UnblamedHunk {
            range_in_blamed_file,
            suspects: [(suspect, range_in_destination)].into(),
            source_file_name: None,
        }
    }
}

fn zero_sha() -> ObjectId {
    use std::str::FromStr;

    ObjectId::from_str("0000000000000000000000000000000000000000").unwrap()
}

fn one_sha() -> ObjectId {
    use std::str::FromStr;

    ObjectId::from_str("1111111111111111111111111111111111111111").unwrap()
}

mod process_change {
    use super::*;
    use crate::file::{process_change, Change, Offset};

    #[test]
    fn nothing() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(0);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            None,
            None,
        );

        assert_eq!(hunk, None);
        assert_eq!(change, None);
        assert_eq!(offset_in_destination, Offset::Added(0));
    }

    #[test]
    fn added_hunk() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(0);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((0..5, suspect).into()),
            Some(Change::AddedOrReplaced(0..3, 0)),
        );

        assert_eq!(hunk, Some((3..5, suspect).into()));
        assert_eq!(change, None);
        assert_eq!(new_hunks_to_blame, [(0..3, suspect).into()]);
        assert_eq!(offset_in_destination, Offset::Added(3));
    }

    #[test]
    fn added_hunk_2() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(0);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((0..5, suspect).into()),
            Some(Change::AddedOrReplaced(2..3, 0)),
        );

        assert_eq!(hunk, Some((3..5, suspect).into()));
        assert_eq!(change, None);
        assert_eq!(new_hunks_to_blame, [(0..2, parent).into(), (2..3, suspect).into()]);
        assert_eq!(offset_in_destination, Offset::Added(1));
    }

    #[test]
    fn added_hunk_3() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(5);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((10..15, suspect).into()),
            Some(Change::AddedOrReplaced(12..13, 0)),
        );

        assert_eq!(hunk, Some((13..15, suspect).into()));
        assert_eq!(change, None);
        assert_eq!(
            new_hunks_to_blame,
            [(10..12, parent, 5..7).into(), (12..13, suspect).into()]
        );
        assert_eq!(offset_in_destination, Offset::Added(6));
    }

    #[test]
    fn added_hunk_4() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(0);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((12..17, suspect, 7..12).into()),
            Some(Change::AddedOrReplaced(9..10, 0)),
        );

        assert_eq!(hunk, Some((15..17, suspect, 10..12).into()));
        assert_eq!(change, None);
        assert_eq!(
            new_hunks_to_blame,
            [(12..14, parent, 7..9).into(), (14..15, suspect, 9..10).into()]
        );
        assert_eq!(offset_in_destination, Offset::Added(1));
    }

    #[test]
    fn added_hunk_5() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(0);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((0..5, suspect).into()),
            Some(Change::AddedOrReplaced(0..3, 1)),
        );

        assert_eq!(hunk, Some((3..5, suspect).into()));
        assert_eq!(change, None);
        assert_eq!(new_hunks_to_blame, [(0..3, suspect).into()]);
        assert_eq!(offset_in_destination, Offset::Added(2));
    }

    #[test]
    fn added_hunk_6() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(0);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((1..5, suspect, 0..4).into()),
            Some(Change::AddedOrReplaced(0..3, 1)),
        );

        assert_eq!(hunk, Some((4..5, suspect, 3..4).into()));
        assert_eq!(change, None);
        assert_eq!(new_hunks_to_blame, [(1..4, suspect, 0..3).into()]);
        assert_eq!(offset_in_destination, Offset::Added(2));
    }

    #[test]
    fn added_hunk_7() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(2);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((3..7, suspect, 2..6).into()),
            Some(Change::AddedOrReplaced(3..5, 1)),
        );

        assert_eq!(hunk, Some((6..7, suspect, 5..6).into()));
        assert_eq!(change, None);
        assert_eq!(
            new_hunks_to_blame,
            [(3..4, parent, 0..1).into(), (4..6, suspect, 3..5).into()]
        );
        assert_eq!(offset_in_destination, Offset::Added(3));
    }

    #[test]
    fn added_hunk_8() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(1);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((23..24, suspect, 25..26).into()),
            Some(Change::AddedOrReplaced(25..27, 1)),
        );

        assert_eq!(hunk, None);
        assert_eq!(change, Some(Change::AddedOrReplaced(25..27, 1)));
        assert_eq!(new_hunks_to_blame, [(23..24, suspect, 25..26).into()]);
        assert_eq!(offset_in_destination, Offset::Added(1));
    }

    #[test]
    fn added_hunk_9() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(0);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((23..24, suspect, 21..22).into()),
            Some(Change::AddedOrReplaced(18..22, 3)),
        );

        assert_eq!(hunk, None);
        assert_eq!(change, None);
        assert_eq!(new_hunks_to_blame, [(23..24, suspect, 21..22).into()]);
        assert_eq!(offset_in_destination, Offset::Added(1));
    }

    #[test]
    fn added_hunk_10() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(0);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((71..109, suspect, 70..108).into()),
            Some(Change::AddedOrReplaced(106..109, 0)),
        );

        assert_eq!(hunk, None);
        assert_eq!(change, Some(Change::AddedOrReplaced(106..109, 0)));
        assert_eq!(
            new_hunks_to_blame,
            [(71..107, parent, 70..106).into(), (107..109, suspect, 106..108).into()]
        );
        assert_eq!(offset_in_destination, Offset::Added(0));
    }

    #[test]
    fn added_hunk_11() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(0);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((149..156, suspect, 137..144).into()),
            Some(Change::AddedOrReplaced(143..146, 0)),
        );

        assert_eq!(hunk, None);
        assert_eq!(change, Some(Change::AddedOrReplaced(143..146, 0)));
        assert_eq!(
            new_hunks_to_blame,
            [
                (149..155, parent, 137..143).into(),
                (155..156, suspect, 143..144).into()
            ]
        );
        assert_eq!(offset_in_destination, Offset::Added(0));
    }

    #[test]
    fn no_overlap() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Deleted(3);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((3..6, suspect, 2..5).into()),
            Some(Change::AddedOrReplaced(7..10, 1)),
        );

        assert_eq!(hunk, None);
        assert_eq!(change, Some(Change::AddedOrReplaced(7..10, 1)));
        assert_eq!(new_hunks_to_blame, [(3..6, parent, 5..8).into()]);
        assert_eq!(offset_in_destination, Offset::Deleted(3));
    }

    #[test]
    fn no_overlap_2() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(0);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((9..11, suspect, 6..8).into()),
            Some(Change::AddedOrReplaced(2..5, 0)),
        );

        assert_eq!(hunk, Some((9..11, suspect, 6..8).into()));
        assert_eq!(change, None);
        assert_eq!(new_hunks_to_blame, []);
        assert_eq!(offset_in_destination, Offset::Added(3));
    }

    #[test]
    fn no_overlap_3() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(0);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((4..15, suspect, 5..16).into()),
            Some(Change::AddedOrReplaced(4..5, 1)),
        );

        assert_eq!(hunk, Some((4..15, suspect, 5..16).into()));
        assert_eq!(change, None);
        assert_eq!(new_hunks_to_blame, []);
        assert_eq!(offset_in_destination, Offset::Added(0));
    }

    #[test]
    fn no_overlap_4() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(1);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((23..25, suspect, 25..27).into()),
            Some(Change::Unchanged(21..22)),
        );

        assert_eq!(hunk, Some((23..25, suspect, 25..27).into()));
        assert_eq!(change, None);
        assert_eq!(new_hunks_to_blame, []);
        assert_eq!(offset_in_destination, Offset::Added(1));
    }

    #[test]
    fn no_overlap_5() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(1);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((15..16, suspect, 17..18).into()),
            Some(Change::Deleted(20, 1)),
        );

        assert_eq!(hunk, None);
        assert_eq!(change, Some(Change::Deleted(20, 1)));
        assert_eq!(new_hunks_to_blame, [(15..16, parent, 16..17).into()]);
        assert_eq!(offset_in_destination, Offset::Added(1));
    }

    #[test]
    fn no_overlap_6() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(0);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((23..25, suspect, 22..24).into()),
            Some(Change::Deleted(20, 1)),
        );

        assert_eq!(hunk, Some((23..25, suspect, 22..24).into()));
        assert_eq!(change, None);
        assert_eq!(new_hunks_to_blame, []);
        assert_eq!(offset_in_destination, Offset::Deleted(1));
    }

    #[test]
    fn enclosing_addition() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(3);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((2..5, suspect, 5..8).into()),
            Some(Change::AddedOrReplaced(3..12, 2)),
        );

        assert_eq!(hunk, None);
        assert_eq!(change, Some(Change::AddedOrReplaced(3..12, 2)));
        assert_eq!(new_hunks_to_blame, [(2..5, suspect, 5..8).into()]);
        assert_eq!(offset_in_destination, Offset::Added(3));
    }

    #[test]
    fn enclosing_deletion() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(3);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((12..19, suspect, 13..20).into()),
            Some(Change::Deleted(15, 2)),
        );

        assert_eq!(hunk, Some((14..19, suspect, 15..20).into()));
        assert_eq!(change, None);
        assert_eq!(new_hunks_to_blame, [(12..14, parent, 10..12).into()]);
        assert_eq!(offset_in_destination, Offset::Added(1));
    }

    #[test]
    fn enclosing_unchanged_lines() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(3);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((110..114, suspect, 109..113).into()),
            Some(Change::Unchanged(109..172)),
        );

        assert_eq!(hunk, None);
        assert_eq!(change, Some(Change::Unchanged(109..172)));
        assert_eq!(new_hunks_to_blame, [(110..114, parent, 106..110).into()]);
        assert_eq!(offset_in_destination, Offset::Added(3));
    }

    #[test]
    fn unchanged_hunk() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(0);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((0..5, suspect).into()),
            Some(Change::Unchanged(0..3)),
        );

        assert_eq!(hunk, Some((0..5, suspect).into()));
        assert_eq!(change, None);
        assert_eq!(new_hunks_to_blame, []);
        assert_eq!(offset_in_destination, Offset::Added(0));
    }

    #[test]
    fn unchanged_hunk_2() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(0);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((0..5, suspect).into()),
            Some(Change::Unchanged(0..7)),
        );

        assert_eq!(hunk, None);
        assert_eq!(change, Some(Change::Unchanged(0..7)));
        assert_eq!(new_hunks_to_blame, [(0..5, parent).into()]);
        assert_eq!(offset_in_destination, Offset::Added(0));
    }

    #[test]
    fn unchanged_hunk_3() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Deleted(2);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((22..30, suspect, 21..29).into()),
            Some(Change::Unchanged(21..23)),
        );

        assert_eq!(hunk, Some((22..30, suspect, 21..29).into()));
        assert_eq!(change, None);
        assert_eq!(new_hunks_to_blame, []);
        assert_eq!(offset_in_destination, Offset::Deleted(2));
    }

    #[test]
    fn deleted_hunk() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(0);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((0..5, suspect).into()),
            Some(Change::Deleted(5, 3)),
        );

        assert_eq!(hunk, None);
        assert_eq!(change, Some(Change::Deleted(5, 3)));
        assert_eq!(new_hunks_to_blame, [(0..5, parent).into()]);
        assert_eq!(offset_in_destination, Offset::Added(0));
    }

    #[test]
    fn deleted_hunk_2() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(0);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((2..16, suspect).into()),
            Some(Change::Deleted(0, 4)),
        );

        assert_eq!(hunk, Some((2..16, suspect).into()));
        assert_eq!(change, None);
        assert_eq!(new_hunks_to_blame, []);
        assert_eq!(offset_in_destination, Offset::Deleted(4));
    }

    #[test]
    fn deleted_hunk_3() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(0);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            Some((2..16, suspect).into()),
            Some(Change::Deleted(14, 4)),
        );

        assert_eq!(hunk, Some((14..16, suspect).into()));
        assert_eq!(change, None);
        assert_eq!(new_hunks_to_blame, [(2..14, parent).into()]);
        assert_eq!(offset_in_destination, Offset::Deleted(4));
    }

    #[test]
    fn addition_only() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(1);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            None,
            Some(Change::AddedOrReplaced(22..25, 1)),
        );

        assert_eq!(hunk, None);
        assert_eq!(change, None);
        assert_eq!(new_hunks_to_blame, []);
        assert_eq!(offset_in_destination, Offset::Added(3));
    }

    #[test]
    fn deletion_only() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(1);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            None,
            Some(Change::Deleted(11, 5)),
        );

        assert_eq!(hunk, None);
        assert_eq!(change, None);
        assert_eq!(new_hunks_to_blame, []);
        assert_eq!(offset_in_destination, Offset::Deleted(4));
    }

    #[test]
    fn unchanged_only() {
        let mut new_hunks_to_blame = Vec::new();
        let mut offset_in_destination: Offset = Offset::Added(1);
        let suspect = zero_sha();
        let parent = one_sha();

        let (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
            None,
            Some(Change::Unchanged(11..13)),
        );

        assert_eq!(hunk, None);
        assert_eq!(change, None);
        assert_eq!(new_hunks_to_blame, []);
        assert_eq!(offset_in_destination, Offset::Added(1));
    }
}

mod process_changes {
    use pretty_assertions::assert_eq;

    use crate::file::{
        process_changes,
        tests::{one_sha, zero_sha},
        Change,
    };

    #[test]
    fn nothing() {
        let suspect = zero_sha();
        let parent = one_sha();
        let new_hunks_to_blame = process_changes(vec![], vec![], suspect, parent);

        assert_eq!(new_hunks_to_blame, []);
    }

    #[test]
    fn added_hunk() {
        let suspect = zero_sha();
        let parent = one_sha();
        let hunks_to_blame = vec![(0..4, suspect).into()];
        let changes = vec![Change::AddedOrReplaced(0..4, 0)];
        let new_hunks_to_blame = process_changes(hunks_to_blame, changes, suspect, parent);

        assert_eq!(new_hunks_to_blame, [(0..4, suspect).into()]);
    }

    #[test]
    fn added_hunk_2() {
        let suspect = zero_sha();
        let parent = one_sha();
        let hunks_to_blame = vec![(0..6, suspect).into()];
        let changes = vec![Change::AddedOrReplaced(0..4, 0), Change::Unchanged(4..6)];
        let new_hunks_to_blame = process_changes(hunks_to_blame, changes, suspect, parent);

        assert_eq!(
            new_hunks_to_blame,
            [(0..4, suspect).into(), (4..6, parent, 0..2).into(),]
        );
    }

    #[test]
    fn added_hunk_3() {
        let suspect = zero_sha();
        let parent = one_sha();
        let hunks_to_blame = vec![(0..6, suspect).into()];
        let changes = vec![
            Change::Unchanged(0..2),
            Change::AddedOrReplaced(2..4, 0),
            Change::Unchanged(4..6),
        ];
        let new_hunks_to_blame = process_changes(hunks_to_blame, changes, suspect, parent);

        assert_eq!(
            new_hunks_to_blame,
            [
                (0..2, parent).into(),
                (2..4, suspect).into(),
                (4..6, parent, 2..4).into(),
            ]
        );
    }

    #[test]
    fn added_hunk_4_0() {
        let suspect = zero_sha();
        let parent = one_sha();
        let hunks_to_blame = vec![(0..6, suspect).into()];
        let changes = vec![
            Change::AddedOrReplaced(0..1, 0),
            Change::AddedOrReplaced(1..4, 0),
            Change::Unchanged(4..6),
        ];
        let new_hunks_to_blame = process_changes(hunks_to_blame, changes, suspect, parent);

        assert_eq!(
            new_hunks_to_blame,
            [
                (0..1, suspect).into(),
                (1..4, suspect).into(),
                (4..6, parent, 0..2).into()
            ]
        );
    }

    #[test]
    fn added_hunk_4_1() {
        let suspect = zero_sha();
        let parent = one_sha();
        let hunks_to_blame = vec![(0..6, suspect).into()];
        let changes = vec![Change::AddedOrReplaced(0..1, 0)];
        let new_hunks_to_blame = process_changes(hunks_to_blame, changes, suspect, parent);

        assert_eq!(
            new_hunks_to_blame,
            [(0..1, suspect).into(), (1..6, parent, 0..5).into()]
        );
    }

    #[test]
    fn added_hunk_4_2() {
        let suspect = zero_sha();
        let parent = one_sha();
        let hunks_to_blame = vec![(2..6, suspect, 0..4).into()];
        let changes = vec![Change::AddedOrReplaced(0..1, 0)];
        let new_hunks_to_blame = process_changes(hunks_to_blame, changes, suspect, parent);

        assert_eq!(
            new_hunks_to_blame,
            [(2..3, suspect, 0..1).into(), (3..6, parent, 0..3).into()]
        );
    }

    #[test]
    fn added_hunk_5() {
        let suspect = zero_sha();
        let parent = one_sha();
        let hunks_to_blame = vec![(0..6, suspect).into()];
        let changes = vec![Change::AddedOrReplaced(0..4, 3), Change::Unchanged(4..6)];
        let new_hunks_to_blame = process_changes(hunks_to_blame, changes, suspect, parent);

        assert_eq!(
            new_hunks_to_blame,
            [(0..4, suspect).into(), (4..6, parent, 3..5).into()]
        );
    }

    #[test]
    fn added_hunk_6() {
        let suspect = zero_sha();
        let parent = one_sha();
        let hunks_to_blame = vec![(4..6, suspect, 3..5).into()];
        let changes = vec![Change::AddedOrReplaced(0..3, 0), Change::Unchanged(3..5)];
        let new_hunks_to_blame = process_changes(hunks_to_blame, changes, suspect, parent);

        assert_eq!(new_hunks_to_blame, [(4..6, parent, 0..2).into()]);
    }

    #[test]
    fn added_hunk_7() {
        let suspect = zero_sha();
        let parent = one_sha();
        let hunks_to_blame = vec![(1..3, suspect, 0..2).into()];
        let changes = vec![Change::AddedOrReplaced(0..1, 2)];
        let new_hunks_to_blame = process_changes(hunks_to_blame, changes, suspect, parent);

        assert_eq!(
            new_hunks_to_blame,
            [(1..2, suspect, 0..1).into(), (2..3, parent).into()]
        );
    }

    #[test]
    fn added_hunk_8() {
        let suspect = zero_sha();
        let parent = one_sha();
        let hunks_to_blame = vec![(0..4, suspect).into()];
        let changes = vec![
            Change::AddedOrReplaced(0..2, 0),
            Change::Unchanged(2..3),
            Change::AddedOrReplaced(3..4, 0),
        ];
        let new_hunks_to_blame = process_changes(hunks_to_blame, changes, suspect, parent);

        assert_eq!(
            new_hunks_to_blame,
            [
                (0..2, suspect).into(),
                (2..3, parent, 0..1).into(),
                (3..4, suspect).into(),
            ]
        );
    }

    #[test]
    fn added_hunk_9() {
        let suspect = zero_sha();
        let parent = one_sha();
        let hunks_to_blame = vec![(0..30, suspect).into(), (31..37, suspect).into()];
        let changes = vec![
            Change::Unchanged(0..16),
            Change::AddedOrReplaced(16..17, 0),
            Change::Unchanged(17..37),
        ];
        let new_hunks_to_blame = process_changes(hunks_to_blame, changes, suspect, parent);

        assert_eq!(
            new_hunks_to_blame,
            [
                (0..16, parent).into(),
                (16..17, suspect).into(),
                (17..30, parent, 16..29).into(),
                (31..37, parent, 30..36).into()
            ]
        );
    }

    #[test]
    fn added_hunk_10() {
        let suspect = zero_sha();
        let parent = one_sha();
        let hunks_to_blame = vec![(1..3, suspect).into(), (5..7, suspect).into(), (8..10, suspect).into()];
        let changes = vec![
            Change::Unchanged(0..6),
            Change::AddedOrReplaced(6..9, 0),
            Change::Unchanged(9..11),
        ];
        let new_hunks_to_blame = process_changes(hunks_to_blame, changes, suspect, parent);

        assert_eq!(
            new_hunks_to_blame,
            [
                (1..3, parent).into(),
                (5..6, parent).into(),
                (6..7, suspect).into(),
                (8..9, suspect).into(),
                (9..10, parent, 6..7).into(),
            ]
        );
    }

    #[test]
    fn deleted_hunk() {
        let suspect = zero_sha();
        let parent = one_sha();
        let hunks_to_blame = vec![(0..4, suspect).into(), (4..7, suspect).into()];
        let changes = vec![Change::Deleted(0, 3), Change::AddedOrReplaced(0..4, 0)];
        let new_hunks_to_blame = process_changes(hunks_to_blame, changes, suspect, parent);

        assert_eq!(
            new_hunks_to_blame,
            [(0..4, suspect).into(), (4..7, parent, 3..6).into()]
        );
    }

    #[test]
    fn subsequent_hunks_overlapping_end_of_addition() {
        let suspect = zero_sha();
        let parent = one_sha();
        let hunks_to_blame = vec![(13..16, suspect).into(), (10..17, suspect).into()];
        let changes = vec![Change::AddedOrReplaced(10..14, 0)];
        let new_hunks_to_blame = process_changes(hunks_to_blame, changes, suspect, parent);

        assert_eq!(
            new_hunks_to_blame,
            [
                (13..14, suspect).into(),
                (14..16, parent, 10..12).into(),
                (10..14, suspect).into(),
                (14..17, parent, 10..13).into(),
            ]
        );
    }
}

mod blame_ranges {
    use crate::{BlameRanges, Error};

    #[test]
    fn create_with_invalid_range() {
        let ranges = BlameRanges::from_one_based_inclusive_range(0..=10);

        assert!(matches!(ranges, Err(Error::InvalidOneBasedLineRange)));
    }

    #[test]
    fn create_from_single_range() {
        let ranges = BlameRanges::from_one_based_inclusive_range(20..=40).unwrap();

        assert_eq!(ranges.to_zero_based_exclusive_ranges(100), vec![19..40]);
    }

    #[test]
    fn create_from_multiple_ranges() {
        let ranges = BlameRanges::from_one_based_inclusive_ranges(vec![1..=4, 10..=14]).unwrap();

        assert_eq!(ranges.to_zero_based_exclusive_ranges(100), vec![0..4, 9..14]);
    }

    #[test]
    fn create_with_empty_ranges() {
        let ranges = BlameRanges::from_one_based_inclusive_ranges(vec![]).unwrap();

        assert_eq!(ranges.to_zero_based_exclusive_ranges(100), vec![0..100]);
    }

    #[test]
    fn add_range_merges_overlapping() {
        let mut ranges = BlameRanges::from_one_based_inclusive_range(1..=5).unwrap();
        ranges.add_one_based_inclusive_range(3..=7).unwrap();

        assert_eq!(ranges.to_zero_based_exclusive_ranges(100), vec![0..7]);
    }

    #[test]
    fn add_range_merges_overlapping_both() {
        let mut ranges = BlameRanges::from_one_based_inclusive_range(1..=3).unwrap();
        ranges.add_one_based_inclusive_range(5..=7).unwrap();
        ranges.add_one_based_inclusive_range(2..=6).unwrap();

        assert_eq!(ranges.to_zero_based_exclusive_ranges(100), vec![0..7]);
    }

    #[test]
    fn add_range_non_sorted() {
        let mut ranges = BlameRanges::from_one_based_inclusive_range(5..=7).unwrap();
        ranges.add_one_based_inclusive_range(1..=3).unwrap();

        assert_eq!(ranges.to_zero_based_exclusive_ranges(100), vec![0..3, 4..7]);
    }

    #[test]
    fn add_range_merges_adjacent() {
        let mut ranges = BlameRanges::from_one_based_inclusive_range(1..=5).unwrap();
        ranges.add_one_based_inclusive_range(6..=10).unwrap();

        assert_eq!(ranges.to_zero_based_exclusive_ranges(100), vec![0..10]);
    }

    #[test]
    fn non_sorted_ranges() {
        let ranges = BlameRanges::from_one_based_inclusive_ranges(vec![10..=15, 1..=5]).unwrap();

        assert_eq!(ranges.to_zero_based_exclusive_ranges(100), vec![0..5, 9..15]);
    }

    #[test]
    fn convert_to_zero_based_exclusive() {
        let ranges = BlameRanges::from_one_based_inclusive_ranges(vec![1..=5, 10..=15]).unwrap();

        assert_eq!(ranges.to_zero_based_exclusive_ranges(100), vec![0..5, 9..15]);
    }

    #[test]
    fn convert_full_file_to_zero_based() {
        let ranges = BlameRanges::WholeFile;

        assert_eq!(ranges.to_zero_based_exclusive_ranges(100), vec![0..100]);
    }

    #[test]
    fn adding_a_range_turns_whole_file_into_partial_file() {
        let mut ranges = BlameRanges::default();

        ranges.add_one_based_inclusive_range(1..=10).unwrap();

        assert_eq!(ranges.to_zero_based_exclusive_ranges(100), vec![0..10]);
    }

    #[test]
    fn to_zero_based_exclusive_ignores_range_past_max_lines() {
        let mut ranges = BlameRanges::from_one_based_inclusive_range(1..=5).unwrap();
        ranges.add_one_based_inclusive_range(16..=20).unwrap();

        assert_eq!(ranges.to_zero_based_exclusive_ranges(7), vec![0..5]);
    }

    #[test]
    fn to_zero_based_exclusive_range_doesnt_exceed_max_lines() {
        let mut ranges = BlameRanges::from_one_based_inclusive_range(1..=5).unwrap();
        ranges.add_one_based_inclusive_range(6..=10).unwrap();

        assert_eq!(ranges.to_zero_based_exclusive_ranges(7), vec![0..7]);
    }

    #[test]
    fn to_zero_based_exclusive_merged_ranges_dont_exceed_max_lines() {
        let mut ranges = BlameRanges::from_one_based_inclusive_range(1..=4).unwrap();
        ranges.add_one_based_inclusive_range(6..=10).unwrap();

        assert_eq!(ranges.to_zero_based_exclusive_ranges(7), vec![0..4, 5..7]);
    }

    #[test]
    fn default_is_full_file() {
        let ranges = BlameRanges::default();

        assert!(matches!(ranges, BlameRanges::WholeFile));
    }
}
