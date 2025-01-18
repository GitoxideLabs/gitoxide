This month felt way more productive than the last which was dominated by sickness unfortunately. However, there was a lot going on so I certainly spent less time on `gitoxide` because of it. Part of that is quite relevant though, and if all goes well I will be able to share more next month.

## Status - 66% done!

It's nothing new that `gitoxide` recently finished the `gix-dir` crate to traverse a directory and find untracked files, and that it could find tracked files that have been modified as well. That's two thirds of `git status`, with the difference between a tree and the index notably missing. 

But what didn't exist is that both can work together, in parallel, while also optionally allowing for rename tracking.

This capability has now been implemented in the `gix-status` crate, and in such a way that all data can still be referenced. The tradeoff here is that the API is clearly plumbing level, and users must implement a `Delegate` trait in order to receive the data.

To make that a bit easier to use, especially with a pre-implemented delegate which just collects all data by copying it, the `gix::Repository` type now provides a method to drive that functionality as well. On top of that, because everything is always more complicated, the `gix` abstraction level also provides status information on Submodules while respecting `submodule.<name>.ignore` to determine how much effort is spent there. `submodule.name.active` is of course respected as well (*which turned out to be a bug as Git doesn't do that*). Something that's very notable and noticeable is the performance difference of the implementation - in big repositories like the one of Rustc with plenty of submodules, `gix status` (66% of it) can easily be 40% faster than Git. It's probably attributable to aggressive parallelisation as a parallel index check may see submodules which then run a parallel status check to see what changed.

### `Repository::status()` with index to worktree iterator

As mentioned before, using a `Delegate` that needs implementation is always a bit cumbersome, and not quite worthy of the highest abstraction level that is `gix`. It does, however, have the advantage that data can be referenced despite a fair share of parallelism. Thus, such an API must remain for those who want to eek out every last bit of performance or reduce load on the memory allocator.

In many cases though, these optimisations aren't needed, and readable code is of greater value. For that reason, there is now an iterator that provides copies of the data obtained by  a delegate. It's made so it doesn't only support progress reporting, but can also integrate with external interrupt flags or when omitted, wires up its own interrupt flag that triggers as the iterator drops. That way, the execution that happens in its own thread to enable the lazy evaluation of the iterator will be interrupted automatically, while also decoupling the production of items with their consumption from a performance perspective, possibly yielding better performance than a typical `Delegate` implementation would.

### Dirwalk iterator

Now that the architecture of such iterators was explored, it seemed only natural to leverage it once more and apply it to the directory walk. Previously, it was only available as `Repository::dirwalk()` which needs a `Delegate`, but now it's also available as `Repository::dirwalk_iter()` with the same features as described above.

It's used to great effect in the Cargo PR, discussed further down, which otherwise would have been far more complicated to implement.

### is-dirty check

With two thirds of a `status` implementation, I thought it's worth providing a near-complete implementation of `is_dirty()` to enable typical usages like in `git describe --dirty`, or `gix commit describe --dirty` in my case. Thanks to the iterator API, it was very easy to implement akin to 'status().next().is_some()`, fully leveraging that the operation will be aborted once the iterator drops after the first change was found.

## Community

Thanks to the improved status, and `gix-dir` as it turned out, I was able to reach out again and bring the existing `gitoxide` integrations much further.

### Memory maps - the right way!

When reading [the corresponding issue](https://github.com/Byron/gitoxide/issues/1312) I was amazed of the environments that `gitoxide` is currently conquering - it's so complex and 'virtual' that I wouldn't dare to try explain it here, knowing I wouldn't even get close to what it really is.

Long story short, it turns out that `gitoxide` was creating memory maps in a way that used the wrong creation flags, which differed from what Git was doing as well. Truth to be told, I never noticed this even was the case, it seemed all the same to me.

But semantics are different in certain virtualized environments, and fortunately the venerable `mmap2` crate that `gitoxide` uses to abstract such details has just the right function to fix this issue for good.

### Onefetch, now without `git2`!

After *nearly 2 years* (!!) the integration of `gitoxide` is now finally done with [the last PR](https://github.com/o2sh/onefetch/pull/1285 ) which manages to remove `git2` entirely. The last bastion of `status`, and fortunately `onefetch` was never using the 'head-index' diff which `gitoxide` is still lacking. The performance difference is quite stark ranging from 3% faster when running `onefetch` in the Linux repository, 11% faster in the Git repository, and finally, 23% faster in the WebKit repository.

This is probably possible as `gitoxide` overall is more parallel, running the index modification check while the directory walk is active.

In any case, this is one of the few, maybe the first, project that achieves this, and I hope there will be many more to come.

### Gix in Cargo

Despite still being on the way to have a fully functional `status` implementation which could power the next PR, coincidentally I found a [niche application for `gix-dir`](https://github.com/rust-lang/cargo/pull/13592) where using `gitoxide` will solve [a real problem](https://github.com/rust-lang/cargo/issues/10150). The PR also helped to greatly improve the usability of the provided APIs in `gitoxide`, which I am always very thankful for.
It's definitely exciting to finally be able to integrate new features again, and with `status` and later `reset` I hope to be able to make big strides.

Cheers
Sebastian

PS: The latest timesheets can be found [here (2024)](https://github.com/Byron/byron/blob/main/timesheets/2024.csv).
