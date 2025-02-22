pub mod changes {
    use gix_diff::tree::{
        visit::{Action, Change},
        Visit,
    };
    use gix_hash::ObjectId;
    use gix_object::bstr::BStr;

    use crate::data::output::count::objects_impl::util::InsertImmutable;

    pub struct AllNew<'a, H> {
        pub objects: Vec<ObjectId>,
        all_seen: &'a H,
    }

    impl<'a, H> AllNew<'a, H>
    where
        H: InsertImmutable,
    {
        pub fn new(all_seen: &'a H) -> Self {
            AllNew {
                objects: Default::default(),
                all_seen,
            }
        }
        pub fn clear(&mut self) {
            self.objects.clear();
        }
    }

    impl<H> Visit for AllNew<'_, H>
    where
        H: InsertImmutable,
    {
        fn pop_front_tracked_path_and_set_current(&mut self) {}

        fn push_back_tracked_path_component(&mut self, _component: &BStr) {}

        fn push_path_component(&mut self, _component: &BStr) {}

        fn pop_path_component(&mut self) {}

        fn visit(&mut self, change: Change) -> Action {
            match change {
                Change::Addition {
                    oid,
                    entry_mode,
                    relation: _,
                }
                | Change::Modification { oid, entry_mode, .. } => {
                    if entry_mode.is_commit() {
                        return Action::Continue;
                    }
                    let inserted = self.all_seen.insert(oid);
                    if inserted {
                        self.objects.push(oid);
                    }
                }
                Change::Deletion { .. } => {}
            }
            Action::Continue
        }
    }
}

pub mod traverse {
    use gix_hash::ObjectId;
    use gix_object::{bstr::BStr, tree::EntryRef};
    use gix_traverse::tree::{visit::Action, Visit};

    use crate::data::output::count::objects_impl::util::InsertImmutable;

    pub struct AllUnseen<'a, H> {
        pub non_trees: Vec<ObjectId>,
        all_seen: &'a H,
    }

    impl<'a, H> AllUnseen<'a, H>
    where
        H: InsertImmutable,
    {
        pub fn new(all_seen: &'a H) -> Self {
            AllUnseen {
                non_trees: Default::default(),
                all_seen,
            }
        }
        pub fn clear(&mut self) {
            self.non_trees.clear();
        }
    }

    impl<H> Visit for AllUnseen<'_, H>
    where
        H: InsertImmutable,
    {
        fn pop_back_tracked_path_and_set_current(&mut self) {}

        fn pop_front_tracked_path_and_set_current(&mut self) {}

        fn push_back_tracked_path_component(&mut self, _component: &BStr) {}

        fn push_path_component(&mut self, _component: &BStr) {}

        fn pop_path_component(&mut self) {}

        fn visit_tree(&mut self, entry: &EntryRef<'_>) -> Action {
            let inserted = self.all_seen.insert(entry.oid.to_owned());
            if inserted {
                Action::Continue
            } else {
                Action::Skip
            }
        }

        fn visit_nontree(&mut self, entry: &EntryRef<'_>) -> Action {
            if entry.mode.is_commit() {
                return Action::Continue;
            }
            let inserted = self.all_seen.insert(entry.oid.to_owned());
            if inserted {
                self.non_trees.push(entry.oid.to_owned());
            }
            Action::Continue
        }
    }
}
