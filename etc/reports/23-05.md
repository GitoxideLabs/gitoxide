This month felt quite productive as there are plenty of tangible outcomes to play with on the command-line, along with further improvements to existing features like `gix describe` and `gix clone`, along with pending performance improvements related to directory traversal.

## Attributes integration

The attributes matching implementation isn't all the useful or practical without integration into the `gix` development tool, and I did my best to provide ways to manually and automatically validate its correctness.

##### `gix index entries` with attributes and statistics

First of all, I never found `git check-attr` that appealing and always thought that attribute should be more integrated into more common tools like `git ls-files`. For that reason, the `gix` equivalent named `gix index entries` now displays the attributes assigned to an entry, along with potential `.gitignore` information. Did you know that
many major repositories actually have ignored files checked in? 
Something I also found valuable is the `--statistics` flag to show intricate details about the work done to produce the results. This makes it easy to see that in some projects, the time it takes to see if `.gitattributes` or `.gitignore` files are present in the worktree can be quite substantial, and that the `--attributes-from-index` flag can speed things up substantially to avoid hitting the disk like that.

##### `gix attributes query`

For completeness, there is also a version of `gix exclude query` aptly named `gix attributes query` which prints the attributes on all given paths, much like `git check-attr` but a bit more powerful as it also shows where the match was found along with more natural attribute display.

```
gix attributes query tests/fixtures/generated-archives/foo.tar.xz
./.gitattributes:1:**/generated-archives/*.tar.xz       tests/fixtures/generated-archives/foo.tar.xz    diff=lfs
./.gitattributes:1:**/generated-archives/*.tar.xz       tests/fixtures/generated-archives/foo.tar.xz    merge=lfs
./.gitattributes:1:**/generated-archives/*.tar.xz       tests/fixtures/generated-archives/foo.tar.xz    -text
./.gitattributes:1:**/generated-archives/*.tar.xz       tests/fixtures/generated-archives/foo.tar.xz    filter=lfs
```

##### `gix attributes validate-baseline`

And finally, I thought it would be useful to be able to validate the `gitoxide` implementation of attributes, i.e. `.gitattributes` and `.gitignore`, against the entire set of locally clones repositories, at the time that was (just) 480. This works by running `git check-ignore` and `git check-attrs`, parse their output as baseline and then compare this with our results. Unfortunately, `git check-ignore` has a performance issue on bigger repositories which makes it prohibitively slow, so most tests are conducted only for `.gitattributes`.

Running `gix attributes validate-baseline` on all of these did reveal a mismatch or two which subsequently be fixed, leading to having higher confidence in the correctness of the implementation.

However, it seems that I should amass more repositories to run `gitoxide` against more regularly, and soon I should have downloaded a corpus of the most popular GitHub repositories by stars to be able to run `gix` against. This was a fun experiment as it involved getting the top-repos of GitHub (from a Kaggle dataset), and then have GPT write all the little scripts to convert JSON to JSONL, figure out how many repos will fit on the disk space I have available using (new to me Jupyter notebooks), to finally download or update them with a little bash script. As a main take-away from that digression clearly was that data science in Python is grossly inefficient, which starts at JSON as file format and `pandas` for manipulating it where 1.8GB of JSON data needed 20GB of RAM. I did wonder why I can't just have a sqlite database for this to probably do the same in 100MB and a few lines of SQL? But clearly, I digress :).

## Improved pack resolution performance and memory consumption

This was an interesting issue around a repository that produced a pack file with a very imbalanced delta-tree, along with having 100MB+ append-only files with it and 110k changes on them. The imbalanced caused 1 thread to have to deal with about 3TB worth of data, while the large files caused a lot of buffers to be held concurrently that inflated memory usage up to 20GB, also dependent on the amount of threads used. All in all, it was a worst-case for the pack resolution algorithm.

The parallelization problem could be resolved by adding another layer of parallelization on top which only kicks in once a thread determines that it has more work and free threads to deal with it. Further, buffer usage was optimized to reduce the overlap of non-essential buffers, thus reducing peak memory usage to the extend possible.

With these changes, `gitoxide` was able to do in 8 or so minutes what git could do in about 50 minutes, even though admittedly the peak memory usage of `gitoxide` is still much higher even with smaller amounts of threads.

## The beginnings of negotiation algorithms

Now that attributes are integrated, I could start with the next big step which is to implement git filters, to ultimately be able to create the data to checkout into a worktree correctly. However, it seems more important to not waste time and finally tackle a topic I looked at anxiously. The reason for this was that graph algorithms seemed quite delicate to do as they relied on git's very intrinsic data structures. `gix describe` is the first and only algorithm that uses graph traversal, supported by nothing more than a HashMap and a VecDeque. Easy enough, sure, but I always feared turning this into primitives to support graph algorithms more easily.

After researching the existing negotiation algorithms more it became clear that trying to implement graph primitives and port `gix describe` over to them would be the lowest barrier of entry. At least that way, one could assure that the primitives are working correctly as there is plenty of coverage for `gix describe` already. And finally having a `gix_revision::Graph` had another advantage which served as motivation: it could make use of the commitrgaph, for which `gix-commitgraph` was already available and unused for more than a year.

The implementation and port of `gix describe` went quite painlessly and testing even turned up another bug in the priority queue implementation which was clearly overfitted to make tests pass. In practice, it would fail though and a `gix describe HEAD~100` on the linux kernel would traverse 250k commits, take a long time *and* end up with the wrong result. Turns out that having a correct priority queue implementation is very important, and that using a `BinaryHeap` is the way to go instead of using `VecDeque` wrongly. Thanks again, GPT, for helping me figure that out! Now `gix describe` is about 8% slower than `git` in the common case, but when the commitgraph is used, it's actually 25% faster, on top of no less than a 6.5 times speedup due to not having to decode objects anymore. Awesome! I should really upgrade other traversals as well to use the new `gix_revision::Graph` and it's transparent usage of the git commitgraph acceleration data structure.

From there I begun abstracting the negotiation algorithms exactly like git does in preparation for an exact translation to Rust. The most important requirement for this to work was a test to produce the expected values directly from git. That test was once again transcribed from the git test-suite, becoming 4 different test setups that yield results for all 3 currently available test algorithms.

With that in place, and with working primitives that mirror the ones used by git, translation should be rather trivial.

Unfortunately, it wasn't as for one using the graph data structure is a bit more cumbersome as it's lazy and optimal in nature, which means the borrow checkers has a great say in it. That is a mere nuisance though, and the real problem that (probably) let to the deviation half-way through the output of the `consecutive` algorithm in the most complex test case. The cause of this *may be* gits reliance on the `parsed` flag on commit objects. Something like it simply doesn't exist in `gitoxide` and it's unclear how it should be simulated, if at all, or what a good proxy would be.

Before spending more time on debugging this, I plan to translate the `skipping` algorithm in the hopes that it just works. However, chances are slim as it, too, relies on the `parsed` flag to determine when to stop a graph traversal.

## Community

### A lot of `moonwalk`

Pascal Kuthe, one of the most active maintainers of the `helix` editor won't live with anything that's not simply the best. That may explain why he started working on `moonwalk`, a highly parallel directory tree traversal crate that leverages the power of filesystem calls that do not resolve entire paths, but work with directory handles directly. That way the traversal is more secure and faster, even though the highest speedups were found only on modern linux kernels. Of course I couldn't wait to try it and got private access, which led to `dua-cli` and `ein t find` to be ported. `dua` got a lot faster in the process and even seems to be a little faster than `pdu` on MacOS, which is a big feat. I definitely can't wait to see this crate released to make `moonwalk` an official part of `dua` and `gitoxide` respectively.

I might add that `moonwalk` will also be the engine powering traversals needed for `gix status`, which should lead to this implementation to be the fastest one available. Stay definitely tuned.

### Rust Foundation sponsorship: shallow clones for cargo merged

The shallow clones PR has been merged 🎉, and now I focus my efforts on correctness with negotiation algorithms and multi-round support which will enable certain fetches through mirrors that are needed in China. After that I will implement the git filter framework to support correct and complete checkouts of worktrees, which is the next realm where `gitoxide` can replace `git2`. And of course, it's possible that at the same time `gix status` will make enough progress to allow it to be used in `cargo` as well, which is relevant for the publishing of crates.

Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2023.csv).
