use std::{borrow::Cow, io, io::BufWriter};

use anyhow::bail;
use gix::Tree;

use crate::OutputFormat;

mod entries {
    use std::collections::VecDeque;

    use gix::{
        bstr::{BStr, BString, ByteSlice, ByteVec},
        objs::tree::EntryRef,
        traverse::tree::visit::Action,
    };

    use crate::repository::tree::format_entry;

    #[cfg_attr(feature = "serde", derive(serde::Serialize))]
    #[derive(Default)]
    pub struct Statistics {
        pub num_trees: usize,
        pub num_links: usize,
        pub num_blobs: usize,
        pub num_blobs_exec: usize,
        pub num_submodules: usize,
        #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
        #[cfg_attr(not(feature = "serde"), allow(dead_code))]
        pub bytes: Option<u64>,
        #[cfg_attr(feature = "serde", serde(skip))]
        pub num_bytes: u64,
    }

    pub struct Traverse<'repo, 'a> {
        pub stats: Statistics,
        repo: Option<&'repo gix::Repository>,
        out: Option<&'a mut dyn std::io::Write>,
        path: BString,
        path_deque: VecDeque<BString>,
    }

    impl<'repo, 'a> Traverse<'repo, 'a> {
        pub fn new(repo: Option<&'repo gix::Repository>, out: Option<&'a mut dyn std::io::Write>) -> Self {
            Traverse {
                stats: Default::default(),
                repo,
                out,
                path: BString::default(),
                path_deque: VecDeque::new(),
            }
        }

        fn pop_element(&mut self) {
            if let Some(pos) = self.path.rfind_byte(b'/') {
                self.path.resize(pos, 0);
            } else {
                self.path.clear();
            }
        }

        fn push_element(&mut self, name: &BStr) {
            if name.is_empty() {
                return;
            }
            if !self.path.is_empty() {
                self.path.push(b'/');
            }
            self.path.push_str(name);
        }
    }

    impl gix::traverse::tree::Visit for Traverse<'_, '_> {
        fn pop_back_tracked_path_and_set_current(&mut self) {
            self.path = self.path_deque.pop_back().unwrap_or_default();
        }

        fn pop_front_tracked_path_and_set_current(&mut self) {
            self.path = self.path_deque.pop_front().expect("every parent is set only once");
        }

        fn push_back_tracked_path_component(&mut self, component: &BStr) {
            self.push_element(component);
            self.path_deque.push_back(self.path.clone());
        }

        fn push_path_component(&mut self, component: &BStr) {
            self.push_element(component);
        }

        fn pop_path_component(&mut self) {
            self.pop_element();
        }

        fn visit_tree(&mut self, _entry: &EntryRef<'_>) -> Action {
            self.stats.num_trees += 1;
            std::ops::ControlFlow::Continue(true)
        }

        fn visit_nontree(&mut self, entry: &EntryRef<'_>) -> Action {
            let size = self
                .repo
                .and_then(|repo| repo.find_header(entry.oid).map(|h| h.size()).ok());
            if let Some(out) = &mut self.out {
                format_entry(out, entry, self.path.as_bstr(), size).ok();
            }
            if let Some(size) = size {
                self.stats.num_bytes += size;
            }

            use gix::object::tree::EntryKind::*;
            match entry.mode.kind() {
                Commit => self.stats.num_submodules += 1,
                Blob => self.stats.num_blobs += 1,
                BlobExecutable => self.stats.num_blobs_exec += 1,
                Link => self.stats.num_links += 1,
                Tree => unreachable!("BUG"),
            }
            std::ops::ControlFlow::Continue(true)
        }
    }
}

#[cfg_attr(not(feature = "serde"), allow(unused_variables))]
pub fn info(
    repo: gix::Repository,
    treeish: Option<&str>,
    extended: bool,
    format: OutputFormat,
    out: impl io::Write,
    mut err: impl io::Write,
) -> anyhow::Result<()> {
    if format == OutputFormat::Human {
        writeln!(err, "Only JSON is implemented - using that instead")?;
    }

    let tree = treeish_to_tree(treeish, &repo)?;

    let mut delegate = entries::Traverse::new(extended.then_some(&repo), None);
    tree.traverse().breadthfirst(&mut delegate)?;

    #[cfg(feature = "serde")]
    {
        delegate.stats.bytes = extended.then_some(delegate.stats.num_bytes);
        serde_json::to_writer_pretty(out, &delegate.stats)?;
    }

    Ok(())
}

pub fn entries(
    repo: gix::Repository,
    treeish: Option<&str>,
    recursive: bool,
    extended: bool,
    format: OutputFormat,
    mut out: impl io::Write,
) -> anyhow::Result<()> {
    if format != OutputFormat::Human {
        bail!("Only human output format is supported at the moment");
    }

    let tree = treeish_to_tree(treeish, &repo)?;

    if recursive {
        let mut write = BufWriter::new(out);
        let mut delegate = entries::Traverse::new(extended.then_some(&repo), Some(&mut write));
        tree.traverse().depthfirst(&mut delegate)?;
    } else {
        for entry in tree.iter() {
            let entry = entry?;
            format_entry(
                &mut out,
                &entry.inner,
                entry.inner.filename,
                extended.then(|| entry.id().header().map(|o| o.size())).transpose()?,
            )?;
        }
    }

    Ok(())
}

fn treeish_to_tree<'repo>(treeish: Option<&str>, repo: &'repo gix::Repository) -> anyhow::Result<Tree<'repo>> {
    let spec = treeish.map_or_else(|| "@^{tree}".into(), |spec| format!("{spec}^{{tree}}"));
    Ok(repo.rev_parse_single(spec.as_str())?.object()?.into_tree())
}

fn format_entry(
    mut out: impl io::Write,
    entry: &gix::objs::tree::EntryRef<'_>,
    filename: &gix::bstr::BStr,
    size: Option<u64>,
) -> std::io::Result<()> {
    use gix::objs::tree::EntryKind::*;
    write!(
        out,
        "{} {}{} ",
        match entry.mode.kind() {
            Tree => "TREE",
            Blob => "BLOB",
            BlobExecutable => " EXE",
            Link => "LINK",
            Commit => "SUBM",
        },
        entry.oid,
        size.map_or_else(|| "".into(), |s| Cow::Owned(format!(" {s}")))
    )?;
    out.write_all(filename)?;
    out.write_all(b"\n")
}
