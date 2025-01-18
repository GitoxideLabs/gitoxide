Whereas last month I wondered where the time went, this month the answer is clear: directory-walk correctness and `gix clean`, for the most part. Oh, and about 7 days of declining productivity due to sickness that I am still fighting with, and that was grave enough to even write about here (this might be a first).

## Various Improvements

Before we get to the meaty stuff, let’s start two smaller but meaningful improvements that have landed since.

### Shebang on Windows

When looking at the Git source-code, what I am really looking at is the Linux version with enough modifications to be able to compile on various platforms, including windows. There it’s easy to miss certain adjustments for Windows that make Windows more similar to Linux actually.

So I learned that Git for Windows actually performs its own `PATH` variable lookup to find the file that Windows would try to execute. Then we will parse the shebang line and launch the program through the interpreter.

And now, `gix-command` does that too, which helped StackedGit to launch its editor just like Git can, without any extra fiddling by the application. And just like in Git, all invocations of external programs in `gitoxide` go through `gix-command` as well, and will thus be benefitting from this improvement.

### Proper Lockfile-Mode

Also in the StackedGit community, there was an issue that occurred when `sudo` would run certain `std` commands that create references. Theses would not be readable by the non-root user afterwards, even though this wasn’t a problem before (or in times when `StackedGit` wasn’t written in Rust or used `gitoxide`).

The problem here made me once again aware of a long-standing and long-forgotten shortcoming that I skipped over back in the days when the `gix-lock` crate was written. By default, the mode *should* be something permissive like `755` or `644`. However, due to the nature of lock-files,  them being built on temporary files, they were created with restricted permissions by default that didn’t allow any access by non-owners. 

It just took a long-overdue PR to the venerable `tempfile` crate to add a feature that would allow users to control the file mode of new tempfiles on the unix platforms, yes, it doesn’t actually apply anywhere else yet. From there it could trickle through `gix-tempfile` which offers mode-control. And `gix-lock` also offers mode-control but uses a permissive mode by default which is used naturally in `gix-ref` without any changes.

Being used by StackedGit has surfaces a treasure-trove of subtle bugs, and I am incredibly thankful for each and every one of them.

## `gix-dir` plumbing crate

The new `gix-dir` plumbing crate has been a long time in the making and doesn’t really contain more than a single public function: `walk(…)`. And this walk it is that is able to detect untracked files, ignored files and directories, and precious ones as well, while being able to collapse directories correctly while taking all that into account as well as `pathspecs`, which are deeply integrated into the traversal.

This has been a test-driven affair while initially learning strongly on the more encapsulated, graspable portions of the ‘dir.c’ file in the Git project. From there, and many, many tests later, it outgrew its original frame and a structure emerged that seems to just ‘fit’ for what it’s supposed to do. This made it quite easy to adjust behaviour and add features that were missing compared to Git - after all, most of dir.c is beyond my understanding and I lured out a lot of behaviour by manual testing with `git clean` and `git status` initially.

I also have to say that I was absolutely frightened by the collapse algorithm as I had no understanding on how it should work, even though in the end, it was trivial and the difficulty lied in taming the complex logic. But the set of right tests made that quite straightforward. The overall structure helped find the right spot for changes and I could hardly be happier with the outcome.

Speaking of tests, I definitely also ran into overfitting logic at least once, and there was a subtle bug of type `x = flag` that should have been `x |= flag` (but there was no test to detect it).

All in all, we are talking about 5200SLOC, of which only 1116SLOC are in `src/`.

Before I forget, it’s worth saying that even though it started out close to the ‘work-skipping’ version of the directory walk that Git performs, probably also for performance reasons, this version explicitly tries to fill in correct information about each entry at all times, partially to use it itself, but also to leave the caller with better data to base decisions on. From this place of correctness and predictability it should also be possible make some savings later-on, if callers opt-in and if these savings are relevant. Not to jump ahead, but in initial performance tests `gix-dir` was in the realm of 1% when pitching `gix clean` again `git clean` in an apples-to-apples comparison in huge repositories, but later this became more of a 7% to 25% slower kind of affair, depending on the repository. But I simply didn’t get to optimise anything either, which typically means not to do work that isn’t used downstream.

Last but not least, this small crate enables `gix clean`, `gix status`, `gix add`, and everything else that has to learn about untracked files within the working tree, so it’s fork in the path to supporting more of the common operations that are desperately needed to get closer to feature parity with `git2`.

### `gix clean`

Of course, the easiest way to test out the walk was by means of `gix clean`, and will it be impossible to explain in writing what a pleasant experience it was. The first (seemingly) working version of a neutered `gix clean` that couldn’t yet delete anything was done in less than two days, and from all I could tell, it worked really well.

Naturally it has advantages if tool and algorithm can be tuned to ones liking, so that for the first time I could have a `gix clean` that could deal with the ‘intricacies’ of the `gitoxide` repository for a greatly improved ‘cleaning’ experience .

But without wanting to spoil it, I am still working on making it the refined experience I want it to be, and I will release [another post](https://github.com/Byron/gitoxide/discussions/1308) just about this topic with much more detail.

### `gix status` improvements

Previously, `gix status` only offered an index-to-worktree check, so untracked files wouldn’t be found. Just out of curiosity, I quickly hacked in a dirwalk that would find said untracked files, and for fun, I let it run in parallel to the index-to-worktree check. And first tests indicate that this is already greatly boosting performance in comparison to the `git status` implementation which has to wait for the ‘index-to-worktree’ check to complete to leverage the ‘CE_UPTODATE’ flag. Just to make it clear, having this flag set isn’t required for correctness, but it helps to avoid `lstat` calls that can be inferred if the index is known to be matching the state on disk. That’s great to safe `lstat` calls, but with todays machines, it’s often faster to just perform them anyway if it helps to run more in parallel, which is what `gitoxide` does.

In a first near apples-to-apples comparison of `git status` on the WebKit repository we already see very promising results of 1.44x speedup. It’s a speedup that is likely to be maintainable as well as the still-to-be-implemented diff between `HEAD^{tree}` and the `index` won’t take anywhere near as long as the currently implemented checks, while being runnable in parallel to the other ones entirely.

This will be good times for `gix status`, and I also think that improvements can be made for rename tracking so that moving a local directory will soon detected as such. And with all these potential advantages, it might well be something I use every day and in place of `git status`, let’s see, it’s still early. 


## Community

It’s entirely on me that his section is emptier than it deserves to be, but my notes might not be complete, or they are overly focussed on one memorable event.

### Compile Time Reductions

These apply to `gitoxide-core` and the main `gitoxide` crate, as they have stripped off a few dependencies which overall can probably save more than 15 CPU-seconds on CI in particular. This was achieved by inlining the functionality or using alternative implementations that didn’t need them.
Most notably, the `tabled` crate was completely removed in favor of a simpler and overall more tidy-looking solution that also needs less code. It wins in all categories, thanks to [benmkw](https://github.com/benmkw) for driving this!

### Gix in Cargo

In order to make progress with the integration into Cargo, `gix status` and `gix reset` will be required, with `status` being required for a proper `reset` anyway. With all the progress of this month, I hope that soon this will also manifest itself in Cargo once again. I definitely can’t wait to make my first PR of the year.

Cheers
Sebastian

PS: The latest timesheets can be found [here (2024)](https://github.com/Byron/byron/blob/main/timesheets/2024.csv).