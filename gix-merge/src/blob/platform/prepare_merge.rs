use gix_error::Result;
use std::{num::NonZeroU8, str::FromStr};

use bstr::{BStr, ByteSlice};
use gix_error::{ResultExt, bail, message};
use gix_filter::attributes;

use crate::blob::{
    BuiltinDriver, MAX_TEXT_MERGE_SIZE, Platform, PlatformRef, ResourceKind,
    builtin_driver::text::Conflict,
    pipeline::Mode,
    platform::{DriverChoice, ResourceRef, merge},
};

const MISSING_RESOURCES: &str = "The 'current', 'ancestor' or 'other' resource for the merge operation were not set";

/// Preparation
impl Platform {
    /// Prepare all state needed for performing a merge, using all [previously set](Self::set_resource()) resources.
    /// `objects` is used to possibly lookup attribute files when obtaining merge-related attributes.
    ///
    /// `options` are to be used when merging later, and they may be altered to implement correct binary merges
    /// in the present of [virtual merge bases](merge::Options::is_virtual_ancestor).
    ///
    /// Note that no additional validation is performed here to facilitate inspection, which means that
    /// resource buffers might still be too large to be merged, preventing a successful merge at a later time.
    pub fn prepare_merge(
        &mut self,
        objects: &impl gix_object::Find,
        mut options: merge::Options,
    ) -> Result<PlatformRef<'_>> {
        let [Some(current), Some(_), Some(_)] = [&self.current, &self.ancestor, &self.other] else {
            bail!(message(MISSING_RESOURCES));
        };
        let current_path = current.rela_path.clone();
        let driver = self.select_driver(current_path.as_bstr(), objects, &mut options)?;
        self.merge_ref(driver, options)
    }

    /// Prepare a blob merge for a tree merge, selecting the driver before loading resources.
    ///
    /// Unlike [`Self::prepare_merge()`], which uses already-loaded resources and their current path,
    /// this method selects the driver using the explicit `current_path`, then calls `load_resources`.
    /// The callback must set all three resources using [`Self::set_resource()`], with `current_path`
    /// matching the path of the current resource. Driver selection and adjustments to `options` are
    /// otherwise the same as in [`Self::prepare_merge()`].
    ///
    /// For built-in text and union drivers, loading unfiltered blobs from the object database
    /// ([`Mode::ToGit`] with no worktree roots) temporarily caps the large-file threshold at
    /// [`MAX_TEXT_MERGE_SIZE`], preserving any lower, nonzero threshold. Blobs exceeding this limit
    /// can then fall back to a binary merge and be picked by object ID without reading their contents.
    /// Renormalization, worktree resources, and other drivers retain the configured loading threshold.
    /// In contrast, [`Self::prepare_merge()`] does not change resource loading: oversized buffers may
    /// remain available for inspection even though they cannot be text-merged.
    ///
    /// The original threshold is restored before returning, including when `load_resources` fails.
    pub(crate) fn prepare_tree_merge(
        &mut self,
        current_path: &BStr,
        objects: &impl gix_object::Find,
        mut options: merge::Options,
        load_resources: impl FnOnce(&mut Self) -> Result<()>,
    ) -> Result<PlatformRef<'_>> {
        let driver = self.select_driver(current_path, objects, &mut options)?;
        let previous_limit = self.filter.options.large_file_threshold_bytes;
        let text_limit_when_merging = MAX_TEXT_MERGE_SIZE as u64;
        // Stored sizes are authoritative for unfiltered blobs, which can be picked by ID without reading them.
        if matches!(
            driver,
            DriverChoice::BuiltIn(BuiltinDriver::Text | BuiltinDriver::Union)
        ) && self.filter_mode == Mode::ToGit
            && self.filter.roots.is_unset()
            && (previous_limit == 0 || previous_limit > text_limit_when_merging)
        {
            self.filter.options.large_file_threshold_bytes = text_limit_when_merging;
        }
        let loaded = load_resources(self);
        self.filter.options.large_file_threshold_bytes = previous_limit;
        loaded?;
        self.merge_ref(driver, options)
    }

    fn select_driver(
        &mut self,
        current_path: &BStr,
        objects: &impl gix_object::Find,
        options: &mut merge::Options,
    ) -> Result<DriverChoice> {
        let entry = self.attr_stack.at_entry(current_path, None, objects).or_raise(|| {
            message!(
                "Failed to obtain attributes for {:?} resource at '{}'",
                ResourceKind::CurrentOrOurs,
                current_path
            )
        })?;
        entry.matching_attributes(&mut self.attrs);
        let mut attrs = self.attrs.iter_selected();
        let merge_attr = attrs.next().expect("pre-initialized with 'merge'");
        let marker_size_attr = attrs.next().expect("pre-initialized with 'conflict-marker-size'");
        let mut driver = match merge_attr.assignment.state {
            attributes::StateRef::Set => DriverChoice::BuiltIn(BuiltinDriver::Text),
            attributes::StateRef::Unset => DriverChoice::BuiltIn(BuiltinDriver::Binary),
            attributes::StateRef::Value(_) | attributes::StateRef::Unspecified => {
                let name = match merge_attr.assignment.state {
                    attributes::StateRef::Value(name) => Some(name.as_bstr()),
                    attributes::StateRef::Unspecified => {
                        self.options.default_driver.as_ref().map(|name| name.as_bstr())
                    }
                    _ => unreachable!("only value and unspecified are possible here"),
                };
                self.find_driver_by_name(name)
            }
        };
        if let attributes::StateRef::Value(value) = marker_size_attr.assignment.state
            && let Some(value) = u8::from_str(value.as_bstr().to_str_lossy().as_ref())
                .ok()
                .and_then(NonZeroU8::new)
        {
            match &mut options.text.conflict {
                Conflict::Keep { marker_size, .. } => *marker_size = value,
                Conflict::ResolveWithOurs | Conflict::ResolveWithTheirs | Conflict::ResolveWithUnion => {}
            }
        }
        if let Some(recursive_driver_name) = match driver {
            DriverChoice::Index(idx) => self.drivers.get(idx),
            _ => None,
        }
        .and_then(|driver| driver.recursive.as_deref())
        .filter(|_| options.is_virtual_ancestor)
        {
            driver = self.find_driver_by_name(Some(recursive_driver_name.as_bstr()));
            options.resolve_binary_with = Some(crate::blob::builtin_driver::binary::ResolveWith::Ours);
        }

        Ok(driver)
    }

    fn merge_ref(&self, driver: DriverChoice, options: merge::Options) -> Result<PlatformRef<'_>> {
        let [Some(current), Some(ancestor), Some(other)] = [&self.current, &self.ancestor, &self.other] else {
            bail!(message(MISSING_RESOURCES));
        };
        Ok(PlatformRef {
            parent: self,
            driver,
            current: ResourceRef::new(current),
            ancestor: ResourceRef::new(ancestor),
            other: ResourceRef::new(other),
            options,
        })
    }

    fn find_driver_by_name(&self, name: Option<&BStr>) -> DriverChoice {
        name.and_then(|name| {
            self.drivers
                .binary_search_by(|d| d.name.as_bstr().cmp(name))
                .ok()
                .map(DriverChoice::Index)
                .or_else(|| {
                    name.to_str()
                        .ok()
                        .and_then(BuiltinDriver::by_name)
                        .map(DriverChoice::BuiltIn)
                })
        })
        .unwrap_or_default()
    }
}
