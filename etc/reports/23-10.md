This month was full of smaller updates with some progress on `gix status` which now works correctly. However, overall it feels like not a lot of substantial work was done while spending a lot of time once more on maintenance.

## Security Patch: arbitrary code execution by passing special SSH URLs for fetching or cloning

Who would have thought that I could fall for this, and particularly, *how* I could fall for this. The short story is that it was possible to craft SSH URLs that contained ssh flags instead of the host name. And these flags could instruct SSH to run a proxy program, which of course can also be an arbitrary shell script.

With this it doesn’t take much to hide URLs that `gix` (CLI) could try fetching from.

`ssh` is quite aware and the solution is simple - just pass `—` to make it stop processing arguments which makes it consider the following argument to be the host no matter what.

`gitoxide`, however, solves the problem just like `git` does by pre-validating the host before using it.

The lesson to be learned here is to always double and triple check what kind of arguments are passed to a command-line tool.

The same issue could possibly happen with `file://` urls, but fortunately here `gitoxide` is already validating that the path, which also could contain command-line flags, is a valid directory on disk.

## `gix status`

It took plenty of testing to finally iron out al the small issues that one has to deal with when doing a status check on the worktree, that is to compare if a tracked file changed compared to the time we recorded its data in the index file.
Now it’s at a state where it appears to be working just like `git` does, while being reasonably fast.

Something that needed special attention is the handling of conflicts,  as these may span multiple chunks. Fortunately this was straightforward to solve by deciding which chunk is ‘owning’ the conflict, while allowing one chunk to reach into another one for reading. Interestingly this change changed the whole architecture of the system away from having mutable access to the index towards producing events describing how *it should change*. This also turned out to be much cleaner and ultimately easier to work with.

It’s worth noting that I *wasn’t* able to figure out what the difference is in terms of threading i.e. `git` performs great with 20 threads, while `gix` gets slower even when going beyond 4 threads on MacOS. On Linux, there is no issue at all so it’s something very specific to the filesystem.

What makes it worse is that there are rare occasions when the index needs to be refreshed, which is when using many threads speeds up the operation tremendously even on MacOS, so just reducing the default amount of threads on MacOS also isn’t always the best.

I even tried to change the mode of parallelism to match the one of `git` so the pattern of access is more similar, but to no avail. Maybe it’s something related to how the Rust standard library abstracts over the respective C-library calls, who knows.

## API Improvements

### Packetline tracing

A hang could occur when fetching via `file://` or `ssh://` (and possibly `git://` ) *and* the V1 protocol and [it took a long time]((https://github.com/Byron/gitoxide/issues/1061)) to figure out a fix.

One step on the way was to finally make it possible to observe all packetlines as they are sent over the wire just like `GIT_TRACE_PACKET` does for `git`. As `gix` integrates this with its tracing engine which supports structure and hierarchy, it looks really *nice* and it becomes immediately obvious which lines are sent along with which ‘stage’ of the protocol.

[Here is an example](https://github.com/Byron/gitoxide/issues/1061#issuecomment-1772554011) for those interested.

The hanging bug could finally be resolved by ‘trying’ a patch that was applied by the original author of the bug which I could never reproduce, which in conjunction with the test-suite led to an even uglier V1 implementation which really only tries to not deal with the intricacies of of this old (but not outdated) version of the protocol.

The reason I don’t like V1 is that `gix` abstracts over all of these details which makes it way easier to implement fetches, but it also doesn’t allow for the incredible amount of special handling that V1 would otherwise need, leading to certain heuristics being used ‘overcome’ V1 complexities.

I just hope that this was it now and it will just keep working 😅.

### Split-worktree support

Opening a `git` repository is everything but trivial and I keep comparing it to a boot sequence rather than opening a bunch of files. And with that complexity it comes at no surprise that from time to time, I learn about new issues related to opening `git` repositories.

This one was reported in `GitPython` actually which has trouble opening a `git` repository with the `core.worktree = /path/to/worktree` configuration set.

This means one can have a repository at `/repo` and the worktree at `/worktree`, merely by configuration.

`gitoxide`, specifically `gix-discover` can now handle this in such a way that it *guesses* more correctly the kind of repository when encountering one of these, to allow `gix` later correct for the imprecision when ‘booting’ the repository for real.

It’s worth noting that `gix-discover` implements a high-performance guessing machinery to quickly see if a folder at hand contains a `git` repository, along with enough information to help speed up actually opening it (to avoid repeating most of the work already done).

### More intuitive `HEAD` conversions

Most programs that interact with `git` repositories will have to start somewhere, and that’s typically through `HEAD`. Its access functions have been revised to make typical conversions, like `HEAD` to tree and `HEAD` to commit (even) easier.

While at it, object conversions are also more consistent now, and there is a newly added `Blob` type for completeness. It’s useful to double-check that the retrieved object is indeed the desired kind of object: `repo.find_object(hash)?.try_into_blob()?` now is possible, for instance.

### Better 32bit support

Even though `gix` is meant to be run on 64 bit systems, 32bit systems will compile it just fine but wouldn’t be able to handle pack files larger than 4GB. Now it should work a little better by assuring that file-sizes are represented as `u64` instead of `usize`.

## Community

### `gix-url` engine rewrite

[This PR](https://github.com/Byron/gitoxide/pull/990) contributes a core-engine rewrite for the URL parser, which makes it more similar to `git` and more capable, which allows more of the baseline tests to be parsed correctly.

Interestingly that also led to a regression due to a test that was (always) missing, which was fixed later on. Now I’d think `nix-url` is in a good position to one day pass all baseline tests.

### rustsec admin tool uses `gitoxide`

For completeness, I decided to also [ported the `rustsec-admin` tool to `gitoxide`](https://github.com/rustsec/rustsec/pull/1017 ) which led to a [2.2x performance improvement on CI](https://github.com/rustsec/rustsec/pull/1017#issuecomment-1734243346).
Now all of the rustsec tooling, as far as I am aware, uses no `git2` anymore and could reduce its dependence on C-code quite a bit.

### Gix in Cargo

There was no progress here, once again, as there was no substantial progress in `gix-status`.

Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2023.csv).
