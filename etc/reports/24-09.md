This month was quite incredible in terms of features, but particularly so in terms of community.

## Merge Base support

There are now various functions [related to computing the merge-base](https://github.com/Byron/gitoxide/blob/0fe5133598c6f843fb3172a4e0c4f58932405647/gix/src/repository/revision.rs#L92-L97) of two or more commits, which an API that excells when multiple calls for queries on the same commit-graph are made.

The idea is that the commit-graph is used as cache so future queries will be able to re-use at least some parts of it.
It's notable that `git2` already does that implicitly and thus also performs very well in comparison, which if you think about it is really nice. `gix`, however, prefers to make the usage of caches explicit so they can be dropped easily, while making clear that memory usage will grow here.

The feature was sponsored by [GitButler](https://gitbutler.com) which can perform a lot of merge-base queries in a row - now it does that ever so slightly faster, but also can do so more easily in threads. Thanks to this implementation, listing branches and obtaining branch details is now fully driven by `gix`, and at least twice as fast as certain inter-dependent operations can now run in parallel.

## Tree Editing

To support the upcoming tree-merge feature, [high-performance tree-editing](https://github.com/Byron/gitoxide/pull/1566) was implemented. As it supports cursors as well, I'd hope that performance will never be an issue for it.

Cursors can be placed as sub-trees, allowing edit operations to start at this subtree right away to save some of the lookups required to find the in-memory structure that should receive the edit.

Except for merges, tree-editing also enables users to generate their own trees, for instance to create a procedural commit.

`Repository::edit_tree()` will get you started.

### In-Memory Object Writing

When editing trees, or performing any other operation that writes a lot of objects, it's often useful to be able to avoid writing them to disk. That way, they can either be conditionally discarded, or they could be written into a pack all at once, avoiding lots of IO that would happen when writing many loose objects serially.

`Repository::with_object_memory()` will enable it on the given repository instance, and it's typically used like `gix::open(path)?.with_object_memory()`.

## Blob-Merging (work-in-progress)

On the way towards doing a full multi-root tree merge, it's required to handle a blob-merge as well, with conflict markers and resolution modes and everything.

It took a while to finish the research and get started, but by now and 2300 lines of code later it's shaping up neatly with the ['big picture'](https://github.com/Byron/gitoxide/pull/1585) of the algorithm already implemented and all baseline tests (i.e. comparisons to Git) passing.

Still a lot of work to be done though to capture all the details and wire it up to `gix`.

## Security

### How configuration path resolution can be abused

Eliah was busy and found an (already fixed) [vulnerability](https://github.com/Byron/gitoxide/security/advisories/GHSA-m8rp-vv92-46c7) where an attacker could abuse the incorrect decoding of quoted configuration file paths to have a `gitoxide` program access a configuration file path under the attackers control.

## Community

### The GitMerge 2024 in Berlin

I have Scott Chacon and [GitButler](https://gitbutler.com) to thank for having been invited to the [GitMerge](https://git-merge.com) conference to talk about `gitoxide`, and meet many fantastic people, with each and everyone being involved in Git. It was like heaven and felt a little bit like a vacation, too.

During the second day, I could get 'upstairs' to the core developer session about Rust in Git and witness a possibly historic gettogether, and maybe one that will be the reason for Rust finally landing in Git to modernize it, and make it more maintainable.

Also as part of the day two unconference, I was sharing everything there is to know about precious files in `gitoxide`, maybe to help them along getting implemented in Git, it was great fun, particularly because Elijah Newren, the author of the technical document, joined us in just the right moment to chime in.

Overall, the conference was an event that is impossible to forget, with my only regret being that I didn't take enough pictures! Nobody will believe me otherwise :D.

#### Gitoxide and the Sovereign Tech Fund

Did you know that Ed Thomson, the maintainer of `libgit2`, was at GitMerge to present? His talk was my personal highlight, with the added bonus him announcing that the [`libgit2` Drips donation](https://www.drips.network/app/projects/github/libgit2/libgit2?exact) will now be made available via [Open Collective](https://opencollective.com/libgit2).

To make it even better he shared his intention to donate some of that to `git2` (the Rust wrapper crate) and `gitoxide` as well - this will be my incentive to get `gitoxide` onto Open Collective, and maybe that's a first step towards finally applying for funding from the Sovereign Tech Fund.

### Faster CI

It took @NobodyXu [just a few tweaks](https://github.com/Byron/gitoxide/pull/1556) to make CI run significantly faster. And as so often, it wasn't the faster CPU that caused it, but a reduction of waste!

And what's best is that one truly notices it.

With that I'd think that CI can now beome slower again, but by running more tests, and even that won't be an impediment as 'auto-merge' now has arrived as well, so no more waiting for CI is needed.

All in all, `gitoxide` CI definitely is in a very good spot now (particularly since Eliah Kagan [re-enabled fuzzing](https://github.com/Byron/gitoxide/pull/1596) as well).

### 'Blame' is getting there

Thanks to @cruessler continuous work 'blame' [is growing strong](https://github.com/Byron/gitoxide/pull/1453), and clocks in at nearly 2500 lines of code.

I still owe a review but would hope that this first PR can be merged without much delay so it can be merged available in higher layers.

### Gix in Cargo

Nothing changed, and it's a bit sad that none fo the [GitButler](https://gitbutler.com)-driven improvements thus far are applicable to push the Cargo integration forward. However, `reset` is done a lot, also in Cargo, so I'd hope this to be another avenue to finally that get last 'tree-index-status' part done which has been on my list for way too long already.

Cheers  
Sebastian

PS: The latest timesheets can be found [here (2024)](https://github.com/Byron/byron/blob/main/timesheets/2024.csv).