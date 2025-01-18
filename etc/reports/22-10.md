Without much ado, let's get right to it instead of claiming there wasn't much movement while writing swaths of text anyway 😅.

## The 'eureka' moment

Maybe it's best to start with this despite not chronological as this particular insight will have lasting impact on `gitoxide` as a whole. The API will not expose parameters anymore if these are available via git configuration files. This greatly simplifies calls as they will generally behave like `git` would. Overrides can easily be done by adjusting the loaded git configuration in memory, with the option to make these adjustments with automatic rollback.

I see this as a great baseline, with the option to…add options for things that most users end up overriding anyway, making that more convenient in the process and type-safe.
After having done this, a lot of the existing APIs to edit references became significantly simpler already, to me one of the best kinds of improvements one can make.

This also means that `gitoxide` is now significantly simpler than similar `git2` operations while generally doing what one would expect, which I believe will help adoption. At this point it also becomes clear how the ideologies behind both libraries differ, with `gitoxide` wanting to integrate with `git` and leverage all of it, whereas `git2` wants to be an API that offers a set of knobs and tunables, leaving the discovery of the correct settings for these to the caller of said API. 

With `gitoxide`, it's now as easy as never to write your own tool or application as using the API feels more like elaborate and type-safe scripting than having to know every nitty and gritty detail about `git`, to the extend possible.

#### `gix progress`

But the above also made keeping track of the implementation status of `git-config` variables essential, so `gix progress` was born which provides a listing of all `git-config` variables I have touched in one way or another. The list is surprisingly long already at nearly 80, with about half of that being used in one way or another.

## `gix fetch`

Yes, it's finally possible to do a proper fetch, which includes matching remote refs to determine which objects to request, fetching a patch, _resolving deltas_ and updating local tracking branches as per refspec (the most typical one being `refs/heads/*:refs/remotes/origin/*`). The _resolving deltas_ stage deserves extra mention as it makes **fetching the linux kernel** in full **1.5x** faster than `git` on a single core and **3x** faster than git when using all 8.5 cores (with `git` notably being stuck with 3 cores as it's unable to scale beyond that).

There were lots of little bits and pieces on the way and I have a feeling that there is still some bits to discover.

#### Improved pack resolution performance

Before I forget, because pack resolution is so important, it's not only optimal as in "it scales perfectly until the CPU maxes out" and in "it only does absolutely necessary work, wasting nearly nothing", but it now also uses ~15% less memory than before, and about 20% less than git for the same operation. If course, these numbers are somewhat estimates as it's a bit harder to look under the hood of git while it is running, but there you go anyway.

This will effectively mean that `gix` can perform operations that will cause `git` to be OOM killed, making it more accessible in the process.

## `gix clone` 

Despite not being quite done in all details and still in a PR, it's worth mentioning that I decided to go all the way by hooking in all the bits and pieces present thus far to have a prototype of cloning functionality. Currently it's able to checkout a working tree using the high-speed implementation that was able to checkout the linux kernel in under a second without breaking into sweat, and I expect it to mature over time, adding support for submodules and filters (e.g. line-feed conversions and calls to git-lfs).

`gix clone` of course has incredible potential, shaving off seconds to minutes from clones in our CI's and locally, each time it runs. The power bill savings are probably something very measurable and I think integrating it with a GitHub action will be most impactful.

Is it as big as I think? We will see….

## Community

### `crates-index-diff` uses `gitoxide` fully and `docs.rs` will use it too

With the ongoing efforts to rewrite the crates-index repository to perfectly match the actual database it was necessary to update `crates-index-diff` to be able to deal with a lot of changes which turn out to be none, and by now the PR to integrate with the latest `crates-index-diff` is being tested and waiting for the merge.

## Rust Foundation sponsorship: cargo shallow clones update

Last month I felt I had to rush towards an integration PR to get `gitoxide` into `cargo`, but now that the mid-term has passed I came to realize that I am trusted and there is no need for haste just because I didn't predict the mid-term perfectly a quarter in advance.

Thus I am now wrapping up `gix fetch` and `gix clone` which use APIs that will serve the first integration into `cargo` at a slightly later date, with the plan to add shallow cloning support either before the integration happened or afterwards depending on what's best at the time.

Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2022.csv).