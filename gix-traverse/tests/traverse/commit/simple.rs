use crate::hex_to_id;
use gix_hash::{oid, ObjectId};
use gix_object::bstr::{ByteSlice, ByteVec};
use gix_traverse::commit;
use std::path::PathBuf;

struct TraversalAssertion<'a> {
    init_script: &'a str,
    repo_name: &'a str,
    tips: &'a [&'a str],
    expected: &'a [&'a str],
    mode: commit::Parents,
    sorting: commit::simple::Sorting,
    expected_without_tips: bool,
    // commit-ids that should be hidden (along with all their history.
    hidden: &'a [&'a str],
}

impl<'a> TraversalAssertion<'a> {
    fn new(init_script: &'a str, tips: &'a [&'a str], expected: &'a [&'a str]) -> Self {
        Self::new_at(init_script, "", tips, expected)
    }

    fn new_at(init_script: &'a str, repo_name: &'a str, tips: &'a [&'a str], expected: &'a [&'a str]) -> Self {
        TraversalAssertion {
            init_script,
            repo_name,
            tips,
            expected,
            mode: Default::default(),
            sorting: Default::default(),
            hidden: Default::default(),
            expected_without_tips: false,
        }
    }

    fn with_parents(&mut self, mode: commit::Parents) -> &mut Self {
        self.mode = mode;
        self
    }

    fn with_sorting(&mut self, sorting: commit::simple::Sorting) -> &mut Self {
        self.sorting = sorting;
        self
    }

    /// Set the commits that should be hidden.
    fn with_hidden(&mut self, hidden: &'a [&'a str]) -> &mut Self {
        self.hidden = hidden;
        self
    }

    /// Do not automatically add tips to the set of expected items.
    fn expected_without_tips(&mut self) -> &mut Self {
        self.expected_without_tips = true;
        self
    }

    /// Execute the fixture and get the repository worktree path.
    pub fn worktree_dir(&self) -> crate::Result<PathBuf> {
        let dir = gix_testtools::scripted_fixture_read_only_standalone(self.init_script)?;
        Ok(dir.join(self.repo_name))
    }
}

impl TraversalAssertion<'_> {
    #[allow(clippy::type_complexity)]
    fn setup(&self) -> crate::Result<(gix_odb::Handle, Vec<ObjectId>, Vec<ObjectId>, Vec<ObjectId>)> {
        let repo_path = self.worktree_dir()?;
        let store = gix_odb::at(repo_path.join(".git").join("objects"))?;
        let tips: Vec<_> = self.tips.iter().copied().map(hex_to_id).collect();
        let expected: Vec<ObjectId> = if self.expected_without_tips {
            self.expected.iter().map(|hex_id| hex_to_id(hex_id)).collect()
        } else {
            tips.clone()
                .into_iter()
                .chain(self.expected.iter().map(|hex_id| hex_to_id(hex_id)))
                .collect()
        };
        let hidden: Vec<_> = self.hidden.iter().copied().map(hex_to_id).collect();
        Ok((store, tips, expected, hidden))
    }

    fn setup_commitgraph(&self, store: &gix_odb::Store, use_graph: bool) -> Option<gix_commitgraph::Graph> {
        use_graph
            .then(|| gix_commitgraph::at(store.path().join("info")))
            .transpose()
            .expect("graph can be loaded if it exists")
    }

    fn check_with_predicate(&mut self, predicate: impl FnMut(&oid) -> bool + Clone) -> crate::Result<()> {
        let (store, tips, expected, hidden) = self.setup()?;

        for use_commitgraph in [false, true] {
            let oids = commit::Simple::filtered(tips.clone(), &store, predicate.clone())
                .sorting(self.sorting)?
                .parents(self.mode)
                .hide(hidden.clone())?
                .commit_graph(self.setup_commitgraph(store.store_ref(), use_commitgraph))
                .map(|res| res.map(|info| info.id))
                .collect::<Result<Vec<_>, _>>()?;

            assert_eq!(oids, expected);
        }
        Ok(())
    }

    fn check(&self) -> crate::Result {
        let (store, tips, expected, hidden) = self.setup()?;

        for use_commitgraph in [false, true] {
            let oids = commit::Simple::new(tips.clone(), &store)
                .sorting(self.sorting)?
                .parents(self.mode)
                .hide(hidden.clone())?
                .commit_graph(self.setup_commitgraph(store.store_ref(), use_commitgraph))
                .map(|res| res.map(|info| info.id))
                .collect::<Result<Vec<_>, _>>()?;
            assert_eq!(
                oids, expected,
                "use_commitgraph = {use_commitgraph}, sorting = {:?}",
                self.sorting
            );
        }
        Ok(())
    }
}

mod hide {
    use crate::commit::simple::{git_graph, TraversalAssertion};
    use gix_traverse::commit::simple::{CommitTimeOrder, Sorting};
    use gix_traverse::commit::Parents;

    fn all_sortings() -> impl Iterator<Item = Sorting> {
        [
            Sorting::BreadthFirst,
            Sorting::ByCommitTime(CommitTimeOrder::NewestFirst),
            Sorting::ByCommitTime(CommitTimeOrder::OldestFirst),
        ]
        .into_iter()
    }

    #[test]
    fn disjoint_hidden_and_interesting() -> crate::Result {
        let mut assertion = TraversalAssertion::new_at(
            "make_repos.sh",
            "disjoint_branches",
            &["e07cf1277ff7c43090f1acfc85a46039e7de1272"], /* b3 */
            &[
                "94cf3f3a4c782b672173423e7a4157a02957dd48", /* b2 */
                "34e5ff5ce3d3ba9f0a00d11a7fad72551fff0861", /* b1 */
            ],
        );
        insta::assert_snapshot!(git_graph(assertion.worktree_dir()?)?, @r"
        * e07cf1277ff7c43090f1acfc85a46039e7de1272  (HEAD -> disjoint) b3
        * 94cf3f3a4c782b672173423e7a4157a02957dd48  b2
        * 34e5ff5ce3d3ba9f0a00d11a7fad72551fff0861  b1
        * b5665181bf4c338ab16b10da0524d81b96aff209  (main) a3
        * f0230ce37b83d8e9f51ea6322ed7e8bd148d8e28  a2
        * 674aca0765b935ac5e7f7e9ab83af7f79272b5b0  a1
        ");

        for sorting in all_sortings() {
            assertion
                .with_hidden(&["b5665181bf4c338ab16b10da0524d81b96aff209" /* a3 */])
                .with_sorting(sorting)
                .check()?;
        }
        Ok(())
    }

    #[test]
    fn all_hidden() -> crate::Result {
        let mut assertion = TraversalAssertion::new_at(
            "make_repos.sh",
            "disjoint_branches",
            &[
                "e07cf1277ff7c43090f1acfc85a46039e7de1272", /* b3 */
                "b5665181bf4c338ab16b10da0524d81b96aff209", /* a3 */
            ],
            // The start positions are also declared hidden, so nothing should be visible.
            &[],
        );
        insta::assert_snapshot!(git_graph(assertion.worktree_dir()?)?, @r"
        * e07cf1277ff7c43090f1acfc85a46039e7de1272  (HEAD -> disjoint) b3
        * 94cf3f3a4c782b672173423e7a4157a02957dd48  b2
        * 34e5ff5ce3d3ba9f0a00d11a7fad72551fff0861  b1
        * b5665181bf4c338ab16b10da0524d81b96aff209  (main) a3
        * f0230ce37b83d8e9f51ea6322ed7e8bd148d8e28  a2
        * 674aca0765b935ac5e7f7e9ab83af7f79272b5b0  a1
        ");

        for sorting in all_sortings() {
            assertion
                .with_hidden(&[
                    "e07cf1277ff7c43090f1acfc85a46039e7de1272", /* b3 */
                    "b5665181bf4c338ab16b10da0524d81b96aff209", /* a3 */
                ])
                .with_sorting(sorting)
                .expected_without_tips()
                .check()?;
        }
        Ok(())
    }

    #[test]
    fn some_hidden_and_all_hidden() -> crate::Result {
        // Hidden has to catch up with non-hidden.
        let mut assertion = TraversalAssertion::new_at(
            "make_repos.sh",
            "simple",
            &["ad33ff2d0c4fc77d56b5fbff6f86f332fe792d83"], /* c2 */
            &[],
        );

        insta::assert_snapshot!(git_graph(assertion.worktree_dir()?)?, @r"
        *-.   f49838d84281c3988eeadd988d97dd358c9f9dc4  (HEAD -> main) merge
        |\ \  
        | | * 48e8dac19508f4238f06c8de2b10301ce64a641c  (branch2) b2c2
        | | * cb6a6befc0a852ac74d74e0354e0f004af29cb79  b2c1
        | * | 66a309480201c4157b0eae86da69f2d606aadbe7  (branch1) b1c2
        | * | 80947acb398362d8236fcb8bf0f8a9dac640583f  b1c1
        | |/  
        * / 0edb95c0c0d9933d88f532ec08fcd405d0eee882  c5
        |/  
        * 8cb5f13b66ce52a49399a2c49f537ee2b812369c  c4
        * 33aa07785dd667c0196064e3be3c51dd9b4744ef  c3
        * ad33ff2d0c4fc77d56b5fbff6f86f332fe792d83  c2
        * 65d6af66f60b8e39fd1ba6a1423178831e764ec5  c1
        ");

        for sorting in all_sortings() {
            assertion
                .with_hidden(&["0edb95c0c0d9933d88f532ec08fcd405d0eee882" /* c5 */])
                .expected_without_tips()
                .with_sorting(sorting)
                .check()?;
        }
        let mut assertion = TraversalAssertion::new_at(
            "make_repos.sh",
            "simple",
            &["f49838d84281c3988eeadd988d97dd358c9f9dc4"], /* merge */
            &["0edb95c0c0d9933d88f532ec08fcd405d0eee882" /* c5 */],
        );

        for sorting in all_sortings() {
            assertion
                .with_hidden(&[
                    "48e8dac19508f4238f06c8de2b10301ce64a641c", /* b2c2 */
                    "66a309480201c4157b0eae86da69f2d606aadbe7", /* b1c2 */
                ])
                .with_sorting(sorting)
                .check()?;
        }

        let mut assertion = TraversalAssertion::new_at(
            "make_repos.sh",
            "simple",
            &["80947acb398362d8236fcb8bf0f8a9dac640583f"], /* b1c1 */
            // Single-parent is only for commits that we are/ought to be interested in.
            // Hence, hidden commits still catch up.
            &[],
        );

        assertion
            .with_hidden(&["f49838d84281c3988eeadd988d97dd358c9f9dc4" /* merge */])
            .with_parents(Parents::First)
            .expected_without_tips()
            .check()?;
        Ok(())
    }

    #[test]
    fn single_parent_with_hidden_tips_issue_2159() -> crate::Result {
        // This test demonstrates the issue from #2159
        // The issue is that when using Parents::First mode with hidden tips,
        // the current code breaks after the first parent instead of traversing all parents
        // to let hidden commits "catch up"
        
        // Test the existing case that should work but might not be working correctly
        let mut assertion = TraversalAssertion::new_at(
            "make_repos.sh",
            "simple",
            &["80947acb398362d8236fcb8bf0f8a9dac640583f"], /* b1c1 - start from a branch */
            // Expected: This should return nothing because the hidden merge should catch up
            &[],
        );

        assertion
            .with_hidden(&["f49838d84281c3988eeadd988d97dd358c9f9dc4" /* merge */])
            .with_parents(Parents::First)
            .expected_without_tips()
            .check()?;
        
        Ok(())
    }
}

mod different_date_intermixed {
    use gix_traverse::commit::simple::{CommitTimeOrder, Sorting};

    use crate::commit::simple::TraversalAssertion;

    #[test]
    fn head_breadth_first() -> crate::Result {
        TraversalAssertion::new_at(
            "make_repos.sh",
            "intermixed",
            &["58912d92944087dcb09dca79cdd2a937cc158bed"], /* merge */
            // This is very different from what git does as it keeps commits together,
            // whereas we spread them out breadth-first.
            &[
                "2dce37be587e07caef8c4a5ab60b423b13a8536a", /* c3 */
                "0f6632a5a7d81417488b86692b729e49c1b73056", /* b1c2 */
                "a9c28710e058af4e5163699960234adb9fb2abc7", /* b2c2 */
                "ad33ff2d0c4fc77d56b5fbff6f86f332fe792d83", /* c2 */
                "77fd3c6832c0cd542f7a39f3af9250c3268db979", /* b1c1 */
                "b648f955b930ca95352fae6f22cb593ee0244b27", /* b2c1 */
                "65d6af66f60b8e39fd1ba6a1423178831e764ec5", /* c1 */
            ],
        )
        .with_sorting(Sorting::BreadthFirst)
        .check()
    }

    #[test]
    fn head_date_order() -> crate::Result {
        TraversalAssertion::new_at(
            "make_repos.sh",
            "intermixed",
            &["58912d92944087dcb09dca79cdd2a937cc158bed"], /* merge */
            // This is exactly what git shows.
            &[
                "2dce37be587e07caef8c4a5ab60b423b13a8536a", /* c3 */
                "0f6632a5a7d81417488b86692b729e49c1b73056", /* b1c2 */
                "a9c28710e058af4e5163699960234adb9fb2abc7", /* b2c2 */
                "77fd3c6832c0cd542f7a39f3af9250c3268db979", /* b1c1 */
                "b648f955b930ca95352fae6f22cb593ee0244b27", /* b2c1 */
                "ad33ff2d0c4fc77d56b5fbff6f86f332fe792d83", /* c2 */
                "65d6af66f60b8e39fd1ba6a1423178831e764ec5", /* c1 */
            ],
        )
        .with_sorting(Sorting::ByCommitTime(CommitTimeOrder::NewestFirst))
        .check()?;

        TraversalAssertion::new_at(
            "make_repos.sh",
            "intermixed",
            &["58912d92944087dcb09dca79cdd2a937cc158bed"], /* merge */
            &[
                "a9c28710e058af4e5163699960234adb9fb2abc7", /* b2c2 */
                "b648f955b930ca95352fae6f22cb593ee0244b27", /* b2c1 */
                "ad33ff2d0c4fc77d56b5fbff6f86f332fe792d83", /* c2 */
                "65d6af66f60b8e39fd1ba6a1423178831e764ec5", /* c1 */
                "0f6632a5a7d81417488b86692b729e49c1b73056", /* b1c2 */
                "77fd3c6832c0cd542f7a39f3af9250c3268db979", /* b1c1 */
                "2dce37be587e07caef8c4a5ab60b423b13a8536a", /* c3 */
            ],
        )
        .with_sorting(Sorting::ByCommitTime(CommitTimeOrder::OldestFirst))
        .check()
    }
}

mod different_date {
    use gix_traverse::commit::simple::{CommitTimeOrder, Sorting};

    use crate::commit::simple::TraversalAssertion;

    #[test]
    fn head_breadth_first() -> crate::Result {
        TraversalAssertion::new_at(
            "make_repos.sh",
            "simple",
            &["f49838d84281c3988eeadd988d97dd358c9f9dc4"], /* merge */
            // This is very different from what git does as it keeps commits together,
            // whereas we spread them out breadth-first.
            &[
                "0edb95c0c0d9933d88f532ec08fcd405d0eee882", /* c5 */
                "66a309480201c4157b0eae86da69f2d606aadbe7", /* b1c2 */
                "48e8dac19508f4238f06c8de2b10301ce64a641c", /* b2c2 */
                "8cb5f13b66ce52a49399a2c49f537ee2b812369c", /* c4 */
                "80947acb398362d8236fcb8bf0f8a9dac640583f", /* b1c1 */
                "cb6a6befc0a852ac74d74e0354e0f004af29cb79", /* b2c1 */
                "33aa07785dd667c0196064e3be3c51dd9b4744ef", /* c3 */
                "ad33ff2d0c4fc77d56b5fbff6f86f332fe792d83", /* c2 */
                "65d6af66f60b8e39fd1ba6a1423178831e764ec5", /* c1 */
            ],
        )
        .check()
    }

    #[test]
    fn head_date_order() -> crate::Result {
        TraversalAssertion::new_at(
            "make_repos.sh",
            "simple",
            &["f49838d84281c3988eeadd988d97dd358c9f9dc4"], /* merge */
            // This is exactly what git shows.
            &[
                "0edb95c0c0d9933d88f532ec08fcd405d0eee882", /* c5 */
                "66a309480201c4157b0eae86da69f2d606aadbe7", /* b1c2 */
                "80947acb398362d8236fcb8bf0f8a9dac640583f", /* b1c1 */
                "48e8dac19508f4238f06c8de2b10301ce64a641c", /* b2c2 */
                "cb6a6befc0a852ac74d74e0354e0f004af29cb79", /* b2c1 */
                "8cb5f13b66ce52a49399a2c49f537ee2b812369c", /* c4 */
                "33aa07785dd667c0196064e3be3c51dd9b4744ef", /* c3 */
                "ad33ff2d0c4fc77d56b5fbff6f86f332fe792d83", /* c2 */
                "65d6af66f60b8e39fd1ba6a1423178831e764ec5", /* c1 */
            ],
        )
        .with_sorting(Sorting::ByCommitTime(CommitTimeOrder::NewestFirst))
        .check()?;
        TraversalAssertion::new_at(
            "make_repos.sh",
            "simple",
            &["f49838d84281c3988eeadd988d97dd358c9f9dc4"], /* merge */
            &[
                "48e8dac19508f4238f06c8de2b10301ce64a641c", /* b2c2 */
                "cb6a6befc0a852ac74d74e0354e0f004af29cb79", /* b2c1 */
                "8cb5f13b66ce52a49399a2c49f537ee2b812369c", /* c4 */
                "33aa07785dd667c0196064e3be3c51dd9b4744ef", /* c3 */
                "ad33ff2d0c4fc77d56b5fbff6f86f332fe792d83", /* c2 */
                "65d6af66f60b8e39fd1ba6a1423178831e764ec5", /* c1 */
                "66a309480201c4157b0eae86da69f2d606aadbe7", /* b1c2 */
                "80947acb398362d8236fcb8bf0f8a9dac640583f", /* b1c1 */
                "0edb95c0c0d9933d88f532ec08fcd405d0eee882", /* c5 */
            ],
        )
        .with_sorting(Sorting::ByCommitTime(CommitTimeOrder::OldestFirst))
        .check()
    }
}

/// Same dates are somewhat special as they show how sorting-details on priority queues affects ordering
mod same_date {
    use gix_traverse::commit::{
        simple::{CommitTimeOrder, Sorting},
        Parents,
    };

    use crate::{commit::simple::TraversalAssertion, hex_to_id};

    #[test]
    fn c4_breadth_first() -> crate::Result {
        TraversalAssertion::new(
            "make_traversal_repo_for_commits_same_date.sh",
            &["9556057aee5abb06912922e9f26c46386a816822"], /* c4 */
            &[
                "17d78c64cef6c33a10a604573fd2c429e477fd63", /* c3 */
                "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7", /* c2 */
                "134385f6d781b7e97062102c6a483440bfda2a03", /* c1 */
            ],
        )
        .with_sorting(Sorting::BreadthFirst)
        .check()
    }

    #[test]
    fn head_breadth_first() -> crate::Result {
        TraversalAssertion::new(
            "make_traversal_repo_for_commits_same_date.sh",
            &["01ec18a3ebf2855708ad3c9d244306bc1fae3e9b"], /* m1b1 */
            // We always take the first parent first, then the second, and so on.
            // Deviation: git for some reason displays b1c2 *before* c5, but I think it's better
            //            to have a strict parent order.
            &[
                "efd9a841189668f1bab5b8ebade9cd0a1b139a37", /* c5 */
                "ce2e8ffaa9608a26f7b21afc1db89cadb54fd353", /* b1c2 */
                "9556057aee5abb06912922e9f26c46386a816822", /* c4 */
                "9152eeee2328073cf23dcf8e90c949170b711659", /* b1c1 */
                "17d78c64cef6c33a10a604573fd2c429e477fd63", /* c3 */
                "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7", /* c2 */
                "134385f6d781b7e97062102c6a483440bfda2a03", /* c1 */
            ],
        )
        .with_sorting(Sorting::BreadthFirst)
        .check()
    }

    #[test]
    fn head_date_order() -> crate::Result {
        TraversalAssertion::new(
            "make_traversal_repo_for_commits_same_date.sh",
            &["01ec18a3ebf2855708ad3c9d244306bc1fae3e9b"], /* m1b1 */
            &[
                "efd9a841189668f1bab5b8ebade9cd0a1b139a37", /* c5 */
                "ce2e8ffaa9608a26f7b21afc1db89cadb54fd353", /* b1c2 */
                "9556057aee5abb06912922e9f26c46386a816822", /* c4 */
                "9152eeee2328073cf23dcf8e90c949170b711659", /* b1c1 */
                "17d78c64cef6c33a10a604573fd2c429e477fd63", /* c3 */
                "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7", /* c2 */
                "134385f6d781b7e97062102c6a483440bfda2a03", /* c1 */
            ],
        )
        .with_sorting(Sorting::ByCommitTime(CommitTimeOrder::NewestFirst))
        .check()?;

        TraversalAssertion::new(
            "make_traversal_repo_for_commits_same_date.sh",
            &["01ec18a3ebf2855708ad3c9d244306bc1fae3e9b"], /* m1b1 */
            &[
                "efd9a841189668f1bab5b8ebade9cd0a1b139a37", /* c5 */
                "ce2e8ffaa9608a26f7b21afc1db89cadb54fd353", /* b1c2 */
                "9556057aee5abb06912922e9f26c46386a816822", /* c4 */
                "9152eeee2328073cf23dcf8e90c949170b711659", /* b1c1 */
                "17d78c64cef6c33a10a604573fd2c429e477fd63", /* c3 */
                "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7", /* c2 */
                "134385f6d781b7e97062102c6a483440bfda2a03", /* c1 */
            ],
        )
        .with_sorting(Sorting::ByCommitTime(CommitTimeOrder::OldestFirst))
        .check()
    }

    #[test]
    fn head_first_parent_only_breadth_first() -> crate::Result {
        TraversalAssertion::new(
            "make_traversal_repo_for_commits_same_date.sh",
            &["01ec18a3ebf2855708ad3c9d244306bc1fae3e9b"], /* m1b1 */
            &[
                "efd9a841189668f1bab5b8ebade9cd0a1b139a37", /* c5 */
                "9556057aee5abb06912922e9f26c46386a816822", /* c4 */
                "17d78c64cef6c33a10a604573fd2c429e477fd63", /* c3 */
                "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7", /* c2 */
                "134385f6d781b7e97062102c6a483440bfda2a03", /* c1 */
            ],
        )
        .with_parents(Parents::First)
        .with_sorting(Sorting::BreadthFirst)
        .check()
    }

    #[test]
    fn head_c4_breadth_first() -> crate::Result {
        TraversalAssertion::new(
            "make_traversal_repo_for_commits_same_date.sh",
            &[
                "01ec18a3ebf2855708ad3c9d244306bc1fae3e9b", /* m1b1 */
                "9556057aee5abb06912922e9f26c46386a816822", /* c4 */
            ],
            &[
                "efd9a841189668f1bab5b8ebade9cd0a1b139a37", /* c5 */
                "ce2e8ffaa9608a26f7b21afc1db89cadb54fd353", /* b1c2 */
                "17d78c64cef6c33a10a604573fd2c429e477fd63", /* c3 */
                "9152eeee2328073cf23dcf8e90c949170b711659", /* b1c1 */
                "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7", /* c2 */
                "134385f6d781b7e97062102c6a483440bfda2a03", /* c1 */
            ],
        )
        .with_sorting(Sorting::BreadthFirst)
        .check()
    }

    #[test]
    fn filtered_commit_does_not_block_ancestors_reachable_from_another_commit() -> crate::Result {
        // I don't see a use case for the predicate returning false for a commit but return true for
        // at least one of its ancestors, so this test is kind of dubious. But we do want
        // `Ancestors` to not eagerly blacklist all of a commit's ancestors when blacklisting that
        // one commit, and this test happens to check that.
        TraversalAssertion::new(
            "make_traversal_repo_for_commits_same_date.sh",
            &["01ec18a3ebf2855708ad3c9d244306bc1fae3e9b"], /* m1b1 */
            &[
                "efd9a841189668f1bab5b8ebade9cd0a1b139a37", /* c5 */
                "ce2e8ffaa9608a26f7b21afc1db89cadb54fd353", /* b1c2 */
                "9556057aee5abb06912922e9f26c46386a816822", /* c4 */
                "17d78c64cef6c33a10a604573fd2c429e477fd63", /* c3 */
                "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7", /* c2 */
                "134385f6d781b7e97062102c6a483440bfda2a03", /* c1 */
            ],
        )
        .with_sorting(Sorting::BreadthFirst)
        .check_with_predicate(|id| id != hex_to_id("9152eeee2328073cf23dcf8e90c949170b711659"))
    }

    #[test]
    fn predicate_only_called_once_even_if_fork_point() -> crate::Result {
        // The `self.seen` check should come before the `self.predicate` check, as we don't know how
        // expensive calling `self.predicate` may be.
        let mut seen = false;
        TraversalAssertion::new(
            "make_traversal_repo_for_commits_same_date.sh",
            &["01ec18a3ebf2855708ad3c9d244306bc1fae3e9b"], /* m1b1 */
            &[
                "efd9a841189668f1bab5b8ebade9cd0a1b139a37", /* c5 */
                "ce2e8ffaa9608a26f7b21afc1db89cadb54fd353", /* b1c2 */
                "9152eeee2328073cf23dcf8e90c949170b711659", /* b1c1 */
            ],
        )
        .with_sorting(Sorting::BreadthFirst)
        .check_with_predicate(move |id| {
            if id == hex_to_id("9556057aee5abb06912922e9f26c46386a816822") {
                assert!(!seen);
                seen = true;
                false
            } else {
                true
            }
        })
    }
}

/// Some dates adjusted to be a year apart, but still 'c1' and 'c2' with the same date.
mod adjusted_dates {
    use gix_traverse::commit::{
        simple::{CommitTimeOrder, Sorting},
        Parents, Simple,
    };

    use crate::commit::simple::git_graph;
    use crate::{commit::simple::TraversalAssertion, hex_to_id};

    #[test]
    fn head_breadth_first() -> crate::Result {
        TraversalAssertion::new(
            "make_traversal_repo_for_commits_with_dates.sh",
            &["288e509293165cb5630d08f4185bdf2445bf6170"], /* m1b1 */
            // Here `git` also shows `b1c1` first, making topo-order similar to date order for some reason,
            // even though c2 *is* the first parent.
            &[
                "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7", /* c2 */
                "bcb05040a6925f2ff5e10d3ae1f9264f2e8c43ac", /* b1c1 */
                "134385f6d781b7e97062102c6a483440bfda2a03", /* c1 */
            ],
        )
        .with_sorting(Sorting::BreadthFirst)
        .check()
    }

    #[test]
    fn head_date_order() -> crate::Result {
        let mut assertion = TraversalAssertion::new(
            "make_traversal_repo_for_commits_with_dates.sh",
            &["288e509293165cb5630d08f4185bdf2445bf6170"], /* m1b1 */
            &[
                "bcb05040a6925f2ff5e10d3ae1f9264f2e8c43ac", /* b1c1 */
                "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7", /* c2 */
                "134385f6d781b7e97062102c6a483440bfda2a03", /* c1 */
            ],
        );
        insta::assert_snapshot!(git_graph(assertion.worktree_dir()?)?, @r"
        *   288e509293165cb5630d08f4185bdf2445bf6170  (HEAD -> main) m1b1
        |\  
        | * bcb05040a6925f2ff5e10d3ae1f9264f2e8c43ac  (branch1) b1c1
        * | 9902e3c3e8f0c569b4ab295ddf473e6de763e1e7  c2
        |/  
        * 134385f6d781b7e97062102c6a483440bfda2a03  c1
        ");
        assertion
            .with_sorting(Sorting::ByCommitTime(CommitTimeOrder::NewestFirst))
            .check()?;
        TraversalAssertion::new(
            "make_traversal_repo_for_commits_with_dates.sh",
            &["288e509293165cb5630d08f4185bdf2445bf6170"], /* m1b1 */
            &[
                "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7", /* c2 */
                "134385f6d781b7e97062102c6a483440bfda2a03", /* c1 */
                "bcb05040a6925f2ff5e10d3ae1f9264f2e8c43ac", /* b1c1 */
            ],
        )
        .with_sorting(Sorting::ByCommitTime(CommitTimeOrder::OldestFirst))
        .check()
    }

    #[test]
    fn head_date_order_with_cutoff() -> crate::Result {
        for order in all_commit_time_orderings() {
            TraversalAssertion::new(
                "make_traversal_repo_for_commits_with_dates.sh",
                &["288e509293165cb5630d08f4185bdf2445bf6170"], /* m1b1 */
                &["bcb05040a6925f2ff5e10d3ae1f9264f2e8c43ac"], /* b1c1 */
            )
            .with_sorting(Sorting::ByCommitTimeCutoff {
                order,
                seconds: 978393600, // =2001-01-02 00:00:00 +0000
            })
            .check()?;
        }
        Ok(())
    }

    #[test]
    fn head_date_order_with_cutoff_disabled() -> crate::Result {
        let very_early = 878393600; // an early date before any commit
        TraversalAssertion::new(
            "make_traversal_repo_for_commits_with_dates.sh",
            &["288e509293165cb5630d08f4185bdf2445bf6170"], /* m1b1 */
            &[
                "bcb05040a6925f2ff5e10d3ae1f9264f2e8c43ac", /* b1c1 */
                "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7", /* c2 */
                "134385f6d781b7e97062102c6a483440bfda2a03", /* c1 */
            ],
        )
        .with_sorting(Sorting::ByCommitTimeCutoff {
            order: CommitTimeOrder::NewestFirst,
            seconds: very_early,
        })
        .check()?;

        TraversalAssertion::new(
            "make_traversal_repo_for_commits_with_dates.sh",
            &["288e509293165cb5630d08f4185bdf2445bf6170"], /* m1b1 */
            &[
                "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7", /* c2 */
                "134385f6d781b7e97062102c6a483440bfda2a03", /* c1 */
                "bcb05040a6925f2ff5e10d3ae1f9264f2e8c43ac", /* b1c1 */
            ],
        )
        .with_sorting(Sorting::ByCommitTimeCutoff {
            order: CommitTimeOrder::OldestFirst,
            seconds: very_early,
        })
        .check()?;
        Ok(())
    }

    #[test]
    fn date_order_with_cutoff_is_applied_to_starting_position() -> crate::Result {
        for order in all_commit_time_orderings() {
            let dir =
                gix_testtools::scripted_fixture_read_only_standalone("make_traversal_repo_for_commits_with_dates.sh")?;
            let store = gix_odb::at(dir.join(".git").join("objects"))?;
            let iter = Simple::new(
                Some(hex_to_id("9902e3c3e8f0c569b4ab295ddf473e6de763e1e7" /* c2 */)),
                &store,
            )
            .sorting(Sorting::ByCommitTimeCutoff {
                order,
                seconds: 978393600, // =2001-01-02 00:00:00 +0000
            })?;
            assert_eq!(
                iter.count(),
                0,
                "initial tips that don't pass cutoff value are not returned either"
            );
        }
        Ok(())
    }

    #[test]
    fn head_date_order_first_parent_only() -> crate::Result {
        for order in all_commit_time_orderings() {
            TraversalAssertion::new(
                "make_traversal_repo_for_commits_with_dates.sh",
                &["288e509293165cb5630d08f4185bdf2445bf6170"], /* m1b1 */
                &[
                    "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7", /* c2 */
                    "134385f6d781b7e97062102c6a483440bfda2a03", /* c1 */
                ],
            )
            .with_sorting(Sorting::ByCommitTime(order))
            .with_parents(Parents::First)
            .check()?;
        }
        Ok(())
    }

    fn all_commit_time_orderings() -> [CommitTimeOrder; 2] {
        [CommitTimeOrder::NewestFirst, CommitTimeOrder::OldestFirst]
    }
}

/// Execute a git status in the given repository path.
fn git_graph(repo_dir: impl AsRef<std::path::Path>) -> crate::Result<String> {
    let out = std::process::Command::new(gix_path::env::exe_invocation())
        .current_dir(repo_dir)
        .args([
            "log",
            "--oneline",
            "--graph",
            "--decorate",
            "--all",
            "--pretty=format:%H %d %s",
        ])
        .output()?;
    if !out.status.success() {
        return Err(format!("git status failed: {err}", err = out.stderr.to_str_lossy()).into());
    }
    Ok(out.stdout.into_string_lossy())
}
