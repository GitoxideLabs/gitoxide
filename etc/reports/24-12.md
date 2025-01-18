Let's see what the last month of 2024 will have to offer!

## Tree-Merging - finishing touches

Now I'd consider the merging of trees to be feature-complete, and it even learned a new trick that isn't present in Git itself.

### `.git/index` support 

By default, the result of a tree-merge is a list of conflicts, if any, along with the merged tree which is provided as editor. The editor stores all edits that when applied will produce the merged tree object.

However, that's not enough to communicate conflicts to existing tooling, as that will need an index with entries in different stages.
And [now](https://github.com/GitoxideLabs/gitoxide/pull/1661) that's exactly what's provided as well, allowing an index based on the merged tree to be modified to hold conflict entries.

It's notable that the Git index is based on the merge it shipped with initially, the so-called trivial merge. It didn't have rename-tracking, so the index as data structured was perfectly suited to represent any merge conflict. Now this is different, as the Index entirely hides if conflicting entries have been renamed. Thus, if you thought what Git presents there is confusing to you, then it's likely it actually *is* confusing as it's not actually able to show everything it knows.

`gitoxide` also can't do better here, but one may hope that the additional information that it provides can one day be used to implement better tools for complex conflict resolution.

### Tree-Favor

`git2` and Git already allow to resolve merge conflicts during blob merges, automatically picking *our* or *their* hunk if hunks conflict, or even to produce the *union* of both which is useful for certain append-only file-formats for instance. This is called *File Favor*.

Git internally also supports a *Tree Favor*, which means that conflicts on tree-level, like ours modified, theirs deleted, can be auto-resolved as well. This feature is only used merging merge-bases of a tree-merge recursively, and it's auto-resolving to use the ancestor version only.

This is supported by `gitoxide` officially, along with resolving to *our* side.

Both auto-resolution modes, *Tree Favor* and *File Favor*, work together to allow for reasonable and fully automatic merges. As an added benefit, `gitoxide` still allows to detect if such an auto-resolution was applied. In practice, this is used in [GitButler](https://gitbutler.com), whose rebases always succeed. Conflicts are marked as such, but auto-resolved towards *our* version. At their convenience, users can enter the conflicting commit, with the conflicts applied to the index, allowing external tooling resolve the merge. From there the resolution is automatically propagated as future commits are re-applied. Neither `git2` nor Git itself can do that (yet?).

## `gix-protocol` cleanup

It was a long-standing task that I kept postponing, but now it [was finally done](https://github.com/GitoxideLabs/gitoxide/pull/1634). The problem was that the majority of the fetch implementation, including aspects of the pack negotiation, were implemented and tested in the `gix` crate, the highest level of abstraction. And even though plumbing crates likes `gix-protocol` and `gix-negotate` were involved, `gix-protocol` was lacking a lot of what it takes to fetch a pack.

On top of that, `gix-protocol` still contained an old implementation of `fetch` that used delegate traits, something that ultimately proved to be the wrong abstraction for callers.

It took me a while to clean all that up, but very early in it was already clear that it is the right thing to do that will make for better code in the end. Interestingly, the old implementation is still used in the `gix-protocol` test-suite which is fully mocked, and the new implementation, despite fully transferred to `gix-protocol`, is still tested in the `gix` crate. There no mock is used though, and it's all real interactions with real Git daemons or `git upload-pack` invocations.

So from a testing perspective, it's still a bit messy, but it's something I think I will be able to live with.

Something else of interest was the `gitoxide-core` crate which also depended on the old API which was now gone. Switching it to the new implementation was quite painless and removed a lot of now unnecessary code. What wasn't quite as painless was to get it to work right, as the end-of-interaction packets now aren't sent anymore by the base implementation. Instead, it controlled by the application code, and failure to do so may be a problem for a local Git daemon who makes journey tests surprisingly flaky.

Fortunately, all that could [ultimately be sorted out](https://github.com/GitoxideLabs/gitoxide/pull/1731), while being a very elusive issue to work on.

## Community

### Revspec with `HEAD@{<date>}` support

Thanks to this [community initiative](https://github.com/GitoxideLabs/gitoxide/pull/1645) the *revspec* parsing is finally complete (unless Git has gained new features in the meantime, of course). This is due to the added support of the `branch@{<date>}` syntax, where `<date>` can be any parsable date to use and find the closest reflog entry.

While wrapping up that implementation, I also fixed timestamp parsing, so ` main@{173213123}` now correctly sees this rather large number as timestamp, instead of trying to access the 173213123nth index of the reflog.

### Support for "months ago" and "years ago"

Thanks to Eliah, who also helped tremendously debugging plenty of related and unrelated problems this month, [we now will parse additional relative dates](https://github.com/GitoxideLabs/gitoxide/pull/1702) like `@{5 months ago` and `@{10 years ago}` like one would expect. 

This combines nicely with the PR in the previous section which makes date-based reflog lookups work.

### A very first and humble `gix log`

Christoph, on top of helping me with 1:1 sessions to finally get [gix blame](https://github.com/GitoxideLabs/gitoxide/pull/1453) reviewed, also contributed [`gix log`](https://github.com/GitoxideLabs/gitoxide/pull/1643).

Originally it was meant to help with `gix blame` debugging, but I thought that this first humble version should be merged this year as a basis for whatever people would like to add in the future. As `git log` is such a huge command, `gix log` will probablly never be more than a toy in comparison though.

### Gix in Cargo

Still, there is no news here, and I am still looking forward to finally getting to finalize `gix status` which is the next big step for an improved `Cargo` integration. At least very soon I will contribute a bug-fix before such an issue is posted on the Cargo issue tracker, see below.

#### Cargo-issue with non-ignored device-files in the package directory

Finally, `gix-dir` will properly classify everything that [isn't trackable by Git](https://github.com/GitoxideLabs/gitoxide/pull/1727), like sockets or named pipes, as `Untrackable`. This way downstream tooling can correctly deal with such directory entries, so that they can, for instance, avoid trying to read them.


Cheers  
Sebastian

PS: The latest timesheets can be found [here (2024)](https://github.com/Byron/byron/blob/main/timesheets/2024.csv).