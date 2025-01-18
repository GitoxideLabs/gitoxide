This month felt quite intense and I have been busy non-stop, and particularly in the last three days where `gix archive` came to life. After all, when you have the capability to properly checkout and filter worktree files, one should also be able to stream these straight into an archive format.
But let's start in order.

## The `gix filter` crate

In order to perform worktree checkouts, one has to apply filters that transform objects from a form suitable for storing git back to the worktree version. This process is entirely controlled by git attributes and git configuration with complex rules and a lot of legacy. These control things like end-of-line conversions, ident substitution (`$Id$`), encoding changes and lastly, external filter programs. This also means that filters can be applied in inverse when adding files from the worktree for storage in git.

Whereas the three built-in filters are relatively simple and operate on whole buffers, where objects can be assumed to be small enough to fit comfortably into memory, there are also user-defined *drivers* programs which can complicate things. It all starts simple with so-called clean and smudge filters, where each is a program to call, receiving the object on standard input and writing the filtered result to standard output. The filepath may also be passed for good measure, but that's all they can do.

Despite simple, here we complicate things a little when filter programs should support streaming. Think of `git-lfs` which receives a small manifest and turns that into potentially huge files that it downloads from a different storage. `git` itself unfortunately appears to be copying the entire output of the filter program to a buffer in memory with no streaming at all. `gitoxide` won't have problems with that though as streaming is fully supported.

Lastly, on windows those old clean and smudge filters tended to be more of a problem as each program invocation takes a lot of time. And so came the invention of long-running filter processes which communicate using protocol lines with each other, allowing one process to filter multiple files. And because that's apparently too synchronous, there is even a delayed filter mechanism which allows multiple objects to be sent, while the results will be received later.

All built-in filters as well as configured filter programs are arranged in a so-called pipeline, and once configured one can pass it a path and data, and it will output the filtered results.

#### worktree checkouts with filters

`gix` was already able to checkout worktrees as long as it could assume that it owned the directory, and that it was empty, which is the case when creating new clones. The `gix_filter::Pipeline` has now been integrated with the parallel worktree checkout implementation, and I am happy to claim that there is no cost to filters at all if most files aren't filtered. And if filters are active, they are as fast as they get.

`gix` once again benefits from multi-threaded checkouts, with each thread having their own filter pipeline and potentially long-running processes, that even support delayed processing on top of that. Checkouts are unlikely to ever get faster.

But wait, of course integrating `gix-filter` into `gix-worktree` wasn't quite such a smooth experience, as initially I wanted to leverage the `std::io::Read` implementation of the filter results. It abstracts over actual streams and in-memory buffers, and I was using `std::io::copy(&mut read, &mut file)` for convenience. However, it turned out that `std::io::copy` just uses a small buffer internally, and despite imposing an extra copy if the source is a buffer already, it also causes many small writes to be made which tanked performance. The solution was to check if a buffer is available as source, and `write_all(buf)` like before, and only use the streaming mode when a stream is present, along with a generous write buffer to further reduce the amount of issues write-calls.

## Backward-dating, or dates before 1970

Last month I wrote about the switch to u64 as date format, and was happy that future dates are now supported to the heat-death of the universe. But what about the past?
It turns out that supporting the past prior to 1970 is useful for git users, and this caused me to once again change the type for the time. Now the time can be negative, which negative dates being before 1970. `git` itself can't properly handle such times and actively rejects them, but `gitoxide` will allow them while handling them just fine.

I thought it was quite rewarding to get a chance to extend git like that, even though future alteration might hide the creation of commits prior to the unix epoch behind a configuration flag just to be sure we don't actually create repository that git can't properly display.

## `gix archive`

Three days, and a last-minute feature that I wanted to be able to talk about. In short, `gix archive` is `git archive`, but without legacy features and with up to 1.8 times more performance for `zip` archives, or about 18% performance for everything else.

This performance boost is possible by decoupling the part that reads data from the object database from the past that creates the archive, allowing both to work in parallel. It's accomplished by streaming worktree data (with filters applied and everything) from a floating thread to the main-thread, which will decode the simple file format and make each entry available behind a simple API with streaming and big-file support.

The `gix-worktree-stream` and `gix-archive` crates have additionally been made available in `gix` through high-level APIs that configure themselves like one would expect. Those could easily be consumed by the `gix archive` CLI implementation, that is quite trivial if you look at it.

Finally, I also added a workspace-streaming task into `gix corpus` and streamed the worktree in full of all ~68k repositories stored on the 4TB corpus disk. It took just an hour. It seems to work.

## Community

### Removal of GPL licensed files from the codebase

But how is that possible? Fortunately an eagle-eyed packager found these template files that were used when initialising a new repository. These where among the first files added to the repository, as `gix init` was the first command I implemented. Back in the days it was just a hobby project and I didn't know what would become of it.

In any case, I immediately remedied the situation and crafted my very own hook example files, and even removed those that are only useful for the server. Those who setup remote repositories typically know which hooks they need, and less is more for local repositories for sure.

I found it particularly rewarding to think of the usability of each hook and make the explanations as descriptive as possible.

### `git2` -> `gix` API mapping

In a GitHub discussion there was final remark, nearly even done in passing, that was exactly as the title suggest. And this is what I thought was really interesting to start early and maintain so that each time there is a comparable API, it will be listed there. Hopefully this will slowly make it easier to use `gitoxide` when coming from `git2`.

### `cargo-binstall` speedups: 9min with `git2` to 17s` with `gitoxide`

`cargo-binstall` is a `cargo` plugin that can find installable binary files for any crate and install them. This works by getting ahold of a crate manifest, and by using that to find the files in question. Previously it would use `git2` to perform a *temporary* clone of the entire crates-io index, along *with a whole* working tree checkout. You can imagine that this was slow, and particularly so on windows.

I submitted a PR to use `gitoxide` instead, which now performs shallow, bare checkouts instead and extracts the manifest directly from git trees instead. Neat.

### `crates-index` used `gix` (PR pending)

Inspired by the fruitful interaction with the `cargo-binstall` maintainers I decided to *quickly* port `crates-index` over to `gix` instead. And overall, it was indeed quite a straightforward transformation which came with many API improvements to make the conversion to `gix` appear just as natural as the `git2` API it replaces.
But little did I know and apparently one of the maintainers was in the middle of preparing for the 1.0 release, which adds a whole new feature implemented with `git2`.
Back to the drawing board it was, but a couple of hours later the conversion once once again completed.
Thile trying to create a new release of `gix` I ran into some problems with my local `cargo smart-release` version which already used a `crates-index` version without `git2`. Unfortunately, it turned out that it didn't find the most recent `HEAD` which would cause `cargo smart-release` to calculate outdated version numbers, causing all kinds of trouble. And I am glad this happened, as it turned out that the `crates-io` registry as crated by cargo stores the most recent HEAD solely in `FETCH_HEAD`, so it's definitely something to deal with. Furthermore, it turned out that the outdated reference positions caused `crates-index` (with `git2` this time) to fetch way more data on each update of the crates index, while seemingly failing to update the refs as well, causing high CPU and great delays. `gix` doesn't have that problem because it does negotiations properly (by now).

I think it's clear that it was more work than I thought, big surprise ;), and the reviews have not even begun. I can only hope that the maintainers are as open to such a switch as previously indicated in the GitHub issue that I was implementing.

Here is the PR if you are curious: https://github.com/frewsxcv/rust-crates-index/pull/129

### Rust Foundation sponsorship

With filtering being used by worktree checkouts, only submodule support is truly missing to allow replacing the `git2` checkout code in cargo. And that is up next, so one may hope there will be a new PR next month.


Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2023.csv).
