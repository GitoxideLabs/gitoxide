use gix_error::ResultExt;
use gix_object::TreeRefIter;

use crate::{Repository, Result, Tree};

/// Diff-utilities
impl Repository {
    /// Prepare the configured terminal difftool to inspect the already-set resources in `diff`.
    ///
    /// Select the trusted `diff.tool`, falling back to `merge.tool`, ignoring GUI selections. Return `None` when neither
    /// is configured. Git must be installed: its `difftool--helper` launches both built-in tools and custom
    /// `difftool.<tool>.cmd` commands, with `mergetool.<tool>` fallbacks. Tool configuration comes from this repository's
    /// trusted configuration snapshot, including in-memory overrides.
    /// Paging is disabled unless trusted `pager.difftool` enables it or names a pager; when enabled, `core.pager` or
    /// Git's usual `less` default supplies the pager.
    ///
    /// The returned command owns temporary input files with their original filenames. Keep it alive until the tool exits.
    /// Standard streams are inherited, launch prompts are disabled, and the tool's exit status is retained for the caller.
    /// `count` is zero-based and `total` is the number of paths being shown. Binary resources cannot be passed to a tool
    /// unless the resource cache has converted them to text.
    #[cfg(feature = "command")]
    pub fn difftool_command(
        &self,
        diff: &gix_diff::blob::Platform,
        count: usize,
        total: usize,
    ) -> Result<Option<gix_diff::blob::platform::prepare_diff_command::Command>> {
        use std::{collections::BTreeSet, process::Stdio};

        use gix_error::bail;
        use gix_object::bstr::{BString, ByteSlice};

        let config = self.config_snapshot();
        let tool = ["diff.tool", "merge.tool"].into_iter().find_map(|key| {
            config
                .plumbing()
                .string_filter(key, self.filter_config_section())
                .filter(|tool| !tool.is_empty())
        });
        let Some(tool) = tool else {
            return Ok(None);
        };
        let pager = match config
            .plumbing()
            .boolean_filter("pager.difftool", self.filter_config_section())
        {
            Ok(Some(true)) => Some(config.trusted_program("core.pager")?.unwrap_or_else(|| "less".into())),
            Ok(Some(false) | None) => None,
            Err(_) => config.trusted_program("pager.difftool")?,
        };
        if diff.resources().is_some_and(|(old, new)| {
            old.data == gix_diff::blob::platform::resource::Data::Missing
                && new.data == gix_diff::blob::platform::resource::Data::Missing
        }) {
            bail!(gix_error::message("Neither side of the diff exists"));
        }
        let context = self.command_context()?;
        let mut command = diff.prepare_diff_command("git".into(), context.clone(), count, total)?;
        let mut launcher: std::process::Command = gix_command::prepare("git")
            .with_context(context)
            .arg(if pager.is_some() { "--paginate" } else { "--no-pager" })
            .arg("difftool--helper")
            .args(command.get_args())
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .try_into()?;
        launcher.envs(
            command
                .get_envs()
                .filter_map(|(key, value)| value.map(|value| (key, value))),
        );
        *command = launcher;
        command
            .current_dir(self.current_dir().join(self.workdir().unwrap_or(self.git_dir())))
            .env("GIT_DIFF_TOOL", gix_path::from_bstr(tool.as_bstr())?.as_ref())
            .env("GIT_MERGETOOL_GUI", "false")
            .env("GIT_DIFFTOOL_NO_PROMPT", "true")
            .env("GIT_DIFFTOOL_TRUST_EXIT_CODE", "true")
            .env("GIT_PAGER", pager.unwrap_or_else(|| "cat".into()))
            .env_remove("GIT_DIFFTOOL_EXTCMD")
            .env_remove("GIT_DIFFTOOL_DIRDIFF")
            .env_remove("GIT_DIFFTOOL_PROMPT")
            .env_remove("GIT_COMMON_DIR")
            .env_remove("GIT_CONFIG")
            .env("GIT_CONFIG_PARAMETERS", "")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", if cfg!(windows) { "NUL" } else { "/dev/null" });

        // Override every tool property, including rejected values, so Git cannot restore untrusted commands from disk.
        let mut properties = BTreeSet::from(["cmd".to_owned(), "path".to_owned()]);
        for section in ["difftool", "mergetool"] {
            for section in config.plumbing().sections_by_name(section).into_iter().flatten() {
                if section.header().subsection_name() == Some(tool.as_bstr()) {
                    properties.extend(section.value_names());
                }
            }
        }
        let mut index = 0;
        for property in properties {
            let key = |section_name| gix_config::KeyRef {
                section_name,
                subsection_name: Some(tool.as_bstr()),
                value_name: &property,
            };
            let merge_value = config.trusted_program(key("mergetool"))?;
            let diff_value = config.trusted_program(key("difftool"))?.or_else(|| merge_value.clone());
            for (section, value) in [("difftool", diff_value), ("mergetool", merge_value)] {
                let mut key = BString::from(section);
                key.push(b'.');
                key.extend_from_slice(&tool);
                key.push(b'.');
                key.extend_from_slice(property.as_bytes());
                command
                    .env(format!("GIT_CONFIG_KEY_{index}"), gix_path::from_bstring(key)?)
                    .env(format!("GIT_CONFIG_VALUE_{index}"), value.unwrap_or_default());
                index += 1;
            }
        }
        command.env("GIT_CONFIG_COUNT", index.to_string());
        Ok(Some(command))
    }

    /// Create a resource cache for diffable objects, and configured with everything it needs to know to perform diffs
    /// faithfully just like `git` would.
    /// `mode` controls what version of a resource should be diffed.
    /// `worktree_roots` determine if files can be read from the worktree, where each side of the diff operation can
    /// be represented by its own worktree root. `.gitattributes` are automatically read from the worktree if at least
    /// one worktree is present.
    ///
    /// Note that attributes will always be obtained from the current `HEAD` index even if the resources being diffed
    /// might live in another tree. Further, if one of the `worktree_roots` are set, attributes will also be read from
    /// the worktree. Otherwise, it will be skipped and attributes are read from the index tree instead.
    pub fn diff_resource_cache(
        &self,
        mode: gix_diff::blob::pipeline::Mode,
        worktree_roots: gix_diff::blob::pipeline::WorktreeRoots,
    ) -> Result<gix_diff::blob::Platform> {
        let index = self.index_or_load_from_head_or_empty()?;
        crate::diff::resource_cache(
            self,
            mode,
            self.attributes_only(
                &index,
                if worktree_roots.is_unset() {
                    gix_worktree::stack::state::attributes::Source::IdMapping
                } else {
                    gix_worktree::stack::state::attributes::Source::WorktreeThenIdMapping
                },
            )?
            .inner,
            worktree_roots,
        )
    }

    /// Produce the changes that would need to be applied to `old_tree` to create `new_tree`.
    /// If `options` are unset, they will be filled in according to the git configuration of this repository, and with
    /// [full paths being tracked](crate::diff::Options::track_path()) as well, which typically means that
    /// rewrite tracking might be disabled if done so explicitly by the user.
    /// If `options` are set, the user can take full control over the settings.
    ///
    /// Note that this method exists to evoke similarity to `git2`, and makes it easier to fully control diff settings.
    /// A more fluent version [may be used as well](Tree::changes()).
    pub fn diff_tree_to_tree<'a, 'old_repo: 'a, 'new_repo: 'a>(
        &self,
        old_tree: impl Into<Option<&'a Tree<'old_repo>>>,
        new_tree: impl Into<Option<&'a Tree<'new_repo>>>,
        options: impl Into<Option<crate::diff::Options>>,
    ) -> Result<Vec<crate::object::tree::diff::ChangeDetached>> {
        let mut cache = self.diff_resource_cache(gix_diff::blob::pipeline::Mode::ToGit, Default::default())?;
        let opts = options
            .into()
            .map_or_else(|| crate::diff::Options::from_configuration(&self.config), Ok)?
            .into();

        let empty_tree = self.empty_tree();
        let old_tree = old_tree.into().unwrap_or(&empty_tree);
        let new_tree = new_tree.into().unwrap_or(&empty_tree);
        let mut out = Vec::new();
        gix_diff::tree_with_rewrites(
            TreeRefIter::from_bytes(&old_tree.data, old_tree.id.kind()),
            TreeRefIter::from_bytes(&new_tree.data, new_tree.id.kind()),
            &mut cache,
            &mut Default::default(),
            &self.objects,
            |change| -> Result<_> {
                out.push(change.into_owned());
                Ok(std::ops::ControlFlow::Continue(()))
            },
            opts,
        )
        .or_error()?;
        Ok(out)
    }

    /// Return a resource cache suitable for diffing blobs from trees directly, where no worktree checkout exists.
    ///
    /// For more control, see [`diff_resource_cache()`](Self::diff_resource_cache).
    pub fn diff_resource_cache_for_tree_diff(&self) -> Result<gix_diff::blob::Platform> {
        self.diff_resource_cache(
            gix_diff::blob::pipeline::Mode::ToGit,
            gix_diff::blob::pipeline::WorktreeRoots::default(),
        )
    }
}
