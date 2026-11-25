#[derive(Debug, Default)]
pub(crate) struct IndexSet {
    words: Vec<u64>,
}

impl IndexSet {
    pub(crate) fn contains(&self, index: usize) -> bool {
        self.words
            .get(index / 64)
            .is_some_and(|word| word & (1 << (index % 64)) != 0)
    }

    pub(crate) fn insert(&mut self, index: usize) -> bool {
        let word_index = index / 64;
        if word_index >= self.words.len() {
            self.words.resize(word_index + 1, 0);
        }
        let word = &mut self.words[word_index];
        let mask = 1 << (index % 64);
        let inserted = *word & mask == 0;
        *word |= mask;
        inserted
    }

    pub(crate) fn clear(&mut self) {
        self.words.fill(0);
    }
}

#[cfg(test)]
mod tests {
    use super::IndexSet;

    #[test]
    fn membership_survives_word_boundaries_growth_and_clearing() {
        let mut set = IndexSet::default();
        assert!(!set.contains(0), "an empty set has no members");
        for index in [0, 63, 64, 129] {
            assert!(set.insert(index), "the first insertion adds a member");
            assert!(!set.insert(index), "repeated insertion preserves membership");
        }
        for index in 0..193 {
            assert_eq!(
                set.contains(index),
                [0, 63, 64, 129].contains(&index),
                "growth preserves exact membership"
            );
        }
        assert_eq!(set.words.len(), 3, "membership uses one bit per index");
        let capacity = set.words.capacity();
        set.clear();
        assert!(
            (0..193).all(|index| !set.contains(index)),
            "clearing removes every member"
        );
        assert_eq!(set.words.capacity(), capacity, "view changes reuse the allocation");
        assert!(set.insert(64), "a cleared slot can become a member again");
        assert!(
            !set.contains(63) && !set.contains(129),
            "old neighboring memberships stay clear"
        );
    }
}
