An extra thanks goes to all my sponsors, I wish you a great year 2022!

For gitoxide, this means to refocus on making it useful to application developers and carve out its space
in an area firmly ruled by `git2`. This should attract contributors and start implanting `gitoxide` into the crate ecosystem
and enable natural adoption.

In order to do that, we will focus development on `git-index`, `git-worktree` and `git-diff` to support non-bare clones,
fetches, as well as something like `git-status`. It's going to be a lot of work as `gitoxide` will aim to provide a feature-complete implementation that will not make compromises with performance or usability.

### MIDX is ready

The multi-index format acts as optional, single index for many packs at once which can greatly accelerate object lookups and finding abbreviated hashes.

The object database now supports using multi-indices if present.

Gitoxide is now also able to write and verify multi-pack indices, with such functionality exposed in `gix pack multi-index`,
and it successfully creates indices for repositories that require large-offset support.

Furthermore, the foundation for a complete `fsck` like command were set to deeply validate all data known in a git repository. For now `gix repository verify` will only verify object databases though, completely ignoring references at the moment.

### `gix` embrace hierarchical sub-commands

Due to the now lifted requirement to support the now abandoned `argh` parser, `gix` previously used a single level of
dashed sub-commands. These have now been re-worked into a hierarchy of sub-commands which greatly helps with discoverability.

### `git-index` reading and basic `git-bitmap` EWAH access

The git index is an essential building block to creating commits, handling merges and speeding up `git-status`, while also enabling more recent features like sparse clones to support large mono-repos.

At its current state, the `git-index` crate decode an any index file using one or more threads, along with all extensions and versions 2 through 4 (all currently known ones). To enable this, `git-bitmap` was created to add support for decoding and iterating EWAH compressed bitmaps.

Even though the current implementation just creates an in-memory version of what's on disk, next steps are to allow useful modifications to it while maintaining extension data correctly. 

### Stability and compatibility

In order to assure broad compatibility for when `git-repository` is used by other crates, the previous crate for opening
memory maps appeared to be unmaintained enough to be a liability. As its replaced, `mmap2` was chosen.

For stability it became necessary to rethink the strategy related to the minimum supported rust version (MSRV) whose
change is now considered a breaking change for `git-repository` and dependencies.
Other crates, like `gitoxide-core` and `cargo-smart-release` do not have a MSRV though, as before.

This step was taken to support the integration of `gitoxide` into `vergen`, which itself is a highly stable crate used by many.

### On the side: Cargo RFC PR nears completion

The review process of [RFC-3028](https://github.com/rust-lang/cargo/pull/9992) is in full motion and quite a few issues have been discovered and hopefully been fixed, too. The major one appears to be the need to add artifact information
into the registry itself. These changes I made with shaky hands as one doesn't touch code related to vital infrastructure
every day.

### On the agenda…

Next up on the path towards cloning a non-bare repository is to actually checkout a worktree. For that to happen, one would have to create an index from a tree and use that to perform the checkout. Thus far I have had a few sessions 
just trying to understand how git creates an index from a tree in practice, and was slowed down by the immense 
complexity of the code which seems to also deal with merges and a large amount of cases that must have accumulated 
over the years. I hope a debugger session can bring clarity on the actual code path taken through this massive forest
to gain confidence that gitoxide's implementation will do things correctly.

Have a great new year 2022,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2022.csv).
