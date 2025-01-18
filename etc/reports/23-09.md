
As opposed to last month, this month felt like *not* a lot was accomplished. This is certainly due to an incredible ramp-up of GitPython maintenance effort and a tax-office inquiry that took a lot of my time. Finally, working on `gix-status` and `gix-reset` wasn't easy for me even though as tangible outcome, I now know how to get it done even though it's going to be more work than originally thought. 

## Worktree Resets

I spent some time studying the feature, in terms of official documentation, its code and actual usage. Two things are worth highlighting here:

* the code of `git reset` seems more complicated than I thought it has to be to accomplish something like it
* `git reset` is twice as slow as I would think it should be.

The latter became obvious when running `git reset` on WebKit, a repository with nearly 400k files. A single `git status` takes about 2 seconds on my machine, but a `git reset`, even if it only affects a couple of files, take double that! This is surprising as technically, all you have to do is to learn about the current state of the worktree, including untracked files, to validate that the few files you would need to change don't have local modifications or otherwise would lead to data loss.

Thus, the goal here for `gix reset` clearly is to perform this operation in about 2.2s at most, and I am confident and looking forward to finally see this happen.

What's great is that in theory, one wouldn't have to check the entire index worth of files (and update the entire index), because all that matters is to validate the files that we are about to change. So in theory, such an operation on a worktree of WebKit could be as fast as ~0.3s when certain shortcuts are taken. I am still not entirely sure such shortcuts would be legal, but I will certainly find out while implementing it.

##### `gix status`

As already pointed out, obtaining the `status` of the worktree (or parts of it) is vital to safely perform a reset. Fortunately, Pascal Kuthe already provided a first, parallel implementation of worktree status, even though it never made it into the `gix` CLI. 

To learn more about it and get a better feeling for its performance, I quickly added a `gix status` just to notice that on WebKit, it was about half as fast as `git`. How could that be? `gix status` is effectively IO bound and the parallelisation is sound, how can it not be at least as fast as `git`? The surprising answer was that `git` uses 20 threads if the index has 10,000 files or more, whereas `gix` only uses up to the amount of logical cores. But even when setting `gix -t20 status` it would not get any faster.

It turns out that, for some reason, `gix` slows down the more it overcommits on what turns out to be `lstat64` calls, where `git` has no problems at all. `gix` works best with 3 to 4 threads, which is when it is about 10% faster than `git`, at least on MacOS.

On Linux, and my gratitude goes to Pascal Kuthe who tested it on his machine, `gix` ends up being twice as fast as `git` where `gix status` on WebKit takes only 150ms!! All this is very, very promising.

As mentioned before, I didn't get a lot of actual work done and my PR stands at only 450SLOC, and there is more safety checks to be done to be no less secure as `git` along with…a whole lot of work that finally leads towards an actual `reset`. 

On the other hand, here is where I want to apply no less than greatest care as it's about the state of our worktree :D.

## Better remote detection and `index.skipHash` support

I spent some time to improve `gix` (crate) API to find remotes for a particular branch so that `gix fetch` now operates more similarly to `git` - previously `gix` could have failed to find a remote even though `git` was fine.

Something more important for compatibility is the support for `index.skipHash`, which allows to read and write index files faster, that is, twice as fast when reading at least. Did you know that the WebKit index file is 53MB in size and loads in 25ms for a throughput of 2.2GB/s? Unfortunately, hashing with SHA1 only runs at 1.2GB/s which then makes it the bottleneck. To speed things up, `git` can now skip the hashing work, but that also means that it will write a zero-hash. If `gix` wouldn't be aware, it would fail validating the index as it compares the actual hash with all zeros. Now that `gix` is aware, it's even more ready for handling the biggest repositories.

## Improving compile times

What started last month saw a big push in this one towards completing the 'dynification' of most APIs along with the addition of component-based feature toggles in the `gix` crate. The former leads to some savings in binary sizes, even though marginal only for `gix`, and the former leads to 10% reduced compile times when only the minimal set of features is required.

Overall, it truly feels like a lot of work for only a little improvements, but on the bright side, the feature documentation got vastly improved in the process as well, along with me being much clearer on how people should interact with the provided features for maximum effect.

Another side-effect of this work is that we thought once again how error handling should work.

##### `therror` - an evolutionary step forward?

Enter [`therror`](https://github.com/Byron/therror), which could one day be the `thiserror` we always wanted. Because as it turns out, even though `thiserror` is exactly what I'd expect from error handling in a library, having these detailed, three-like errors made of enums is also a lot of work to maintain and quite cumbersome to match on. 

That's two problems that need solving. For one, what if there was a programmatically accessible tree of errors, so it can be queried at runtime? This would have the advantage that changes in errors don't break compiles anymore, even though failures would then be likely to occur at runtime or or consuming code simply doesn't work anymore.

Secondly, what if one could just declare and pass any kind of error information along right where it happens so building errors made of enums isn't needed in the first place?

What about being able to combine these modes of operation?

In theory, with a proc-macro all of this should be possible and `gitoxide` would definitely sign up for trying to take a step forward in terms of error handling. 
Lets see when there is more to say about this.

## Community

### `gengo` - now uses `gix` and is 40x to 60x faster!

As proof, let me link the PR right away: https://github.com/spenserblack/gengo/pull/157 . But why would that happen in the first place? It turns out that  `gengo` is going to be the next code classification engine for use in `onefetch`. `onefetch` already uses `gix` for great performance and compatibility , but `gengo` initially did not. It's nothing I could let happen even a second longer, so I converted `git2` to `gix`. Typically, speedups by this alone are in the 25% range, but in this case it turns out that the `git2` API for checking attributes is incredibly slow - so slow in fact that `gix` is about 8 times faster for attribute queries on a single core. Combine this with standard fearless multi-threading and one gets up to 60x the speed on huge repositories like WebKit. It did seem that `git2` really didn't scale well with increasing amounts of files.

### `cargo semver-check` uses `gix`

[In this PR](https://github.com/obi1kenobi/cargo-semver-checks/pull/531) I am adding `gix` to the mix, which makes more sense than ever as this binary also uses `tame-index`. Thanks to it, it actually compiles ~15% faster than before!

### `GQL` uses `gix`

The Git Query Language now uses `gix` thanks to [this PR](https://github.com/AmrDeveloper/GQL/pull/23). The story on how this one came to be is interesting as GQL breaks some news on Reddit regularly. The first time it happened I saw potential but wasn't sure if it will catch on *or* if it sees continued development. But after repeated progress updates I decided to finally check it out in greater depth and do the switch from `git2` to `gix` while at it.
In average, it's now 25% faster, but it's also 4 times faster when showing diffs. The reason for the latter might be more efficient object caching.

### `gix` + Azure DevOps Repos = 💔

Did you know that azure can host git repos? No? And did you know that `gix` can't clone bigger (or let's say, more deltified) ones when `git` can?

All this was new to me and an investigation shows that azure devops repos are hosted behind the V1 protocol (old) and using an old pack file format which I thought wasn't even allowed. It's clearly inefficient to refer to delta-bases by their full ID (`REF_DELTA`) if you could also refer to them by delta-encoded offset from the current position (`OFS_DELTA`). The entire pack-resolution machinery relies on this sane pack format, while support `REF_DELTAs` for use with the `thin-pack` format. But what azure devops does is, unfortunately, impossible to handle with the current architecture which definitely is optimised for performance and memory footprint.

This is memorable as it is the first time where I have to admit defeat and as it's prohibitive to implement an entirely new resolution engine, one that would be slower, just to support this. Maybe one day I will have the time, or maybe, by then, Azure Devops will host their repositories using a more recent pack format.

The whole conclusion [can be found in my issue-comment](https://github.com/Byron/gitoxide/issues/1025#issuecomment-1729480387).

### GitPython…

This month, [GitPython](https://github.com/gitpython-developers/GitPython) definitely deserves an honorable mention as it consumed a whopping 17.5h of my time. One day I even spent 3.5h with it. The reason for this is clearly a welcome one though as GitPython now makes great strides forward thanks to the great work [Eliah Kagan](https://github.com/EliahKagan).

All this GitPython exposure helped me arrive at a new vision for GitPython: instead of trying to replace it with the python bindings of `gitoxide`, maybe there is a way to let it *use and expose* `gitoxide`'s future python bindings. Even though python bindings of `gitoxide` would be very usable all by themselves, when integrating with GitPython one will have the benefit of using functionality exclusive to the `git` binary as well. GitPython could actually help transition more codebases to the spiritual successor which is `gitoxide`.

It's definitely exciting to think about that and this and I definitely found my interest in GitPython rekindled.

### Gix in Cargo

There was no progress here, unfortunately, `gix reset` is a prerequisite to bring `gitoxide` to more parts of `cargo` and that is still very much work in progress.

Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2023.csv).
