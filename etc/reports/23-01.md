Leading up to the new year, I succeeded to get the first `gitoxide`-integration PR ready for review after the last big push for the year 🎉. Since then, the work on `gitoxide` was in maintenance mode due to me having been busy the first two weeks of the year with fascinating contract work. Nonetheless, I can't help but feel that the actual `gitoxide` progress was smaller than I hoped. This feeling is somewhat amplified by me being generally slow to get back into `cargo` and addressing the review tasks, it's a matter of fear I suppose and the only way to go ahead and face it straight on.

This also means that today we skip right to the community section, for lack of anything substantial produced by me.

## Community

### A first hello with the team of the Josh-Project

[The Josh Project](https://github.com/josh-project/josh) is a proxy server with the ability to create virtual-mono-repos from small repos, apparently through magic, which is truly a unique capability.

It currently uses `git2` which comes with the usual issues around scaling, control and hackability so the team is looking for alternatives.

Let's just say that I am excited to provide best-possible support, and will welcome any contribution that makes their life easier.

### A `gitoxide`-powered `git-remote-icp` remote helper created by Paul@Codebase Labs

Codebase Labs has been hard at work to let `git` communicate to The Internet Computer and released a new remote helper to do that. It leverages plumbing crates of `gitoxide` directly to get the low-level functionality it needs and can maybe help to one day get a 'remote-helper' framework into `gitoxide` to make creating new helper as easy as possible.

Thank you Paul for making the code available!

### `gitoxide` can now read any `.git/index` file

The git index is a cache for the last seen state of the worktree. As such, it's read with most git commands, and will even be written after seemingly read-only commands like `git status` were executed. It has also seen a fair share of extensions over the years to better deal with huge repositories, and I am proudly announcing that, thanks to the work of Sidney, `gitoxide` can now read and handle all possible extensions.

This gives it an edge over `git2` which would have to error out when encountering mandatory index extensions. Awesome tools like `starship` and `onefetch` benefit as they can now read any git repository out there.

### Stacked-Git uses `gitoxide`

This change seemed pretty straightforward and to my surprise let to considerable improvements in the startup time of the `stg` binary. The aggregated time-savings of a few milliseconds each time lead to a vastly improved development cycle that made a real difference.

[Here is the full article](https://github.com/stacked-git/stgit/discussions/255) for even more improvements that `gitoxide` brought to the project.

### The removal of default names for authors and committers

When tests were written to perform commits, it initially seemed like a faithful-to-git implementation to default committer and author to a hard-coded value if nothing else is configured. This made implementing tests easier as one wouldn't have to take care of configuring user or committer names, and this was in times where doing so wasn't trivial or even possible from within `gitoxide`. The author of Stacked-Git, hunting another bug reported there, truthfully pointed out that hard-coding a value doesn't seem like the right choice, nor would it be much better to guess a name and email like git does. After all, how often is `git` actually correct in doing so, and how is the library user going to differentiate this at all?

I saw it similarly and took the chance to make an unconfigured committer or author a hard error, which also helped to fix a few tests which actually picked up the hosts configuration due to not being isolated properly.

With this issue gone, the API has we have it now feels significantly more mature and usable.

Thanks again!

### Rust Foundation sponsorship: cargo shallow clones update

This year the grant was renewed to continue supporting getting shallow clones to `cargo` *and* start replacing `git2`. Quite literally the end of last year, the [respective PR](https://github.com/rust-lang/cargo/pull/11448) was readied for review and soon after the first comments came in. And… there is still quite a stack of small issues to resolve which all in all should mean quite a bit of work. It's also clear that some time will soon have to be spent to implement a native SSH transport with `known_hosts` support, as well as native support for the `git` protocol to avoid launching any external programs while handling the crates index and avoid regressions.

It's a mind-boggling amount of work that frightens me quite a bit, but I also know it's *just* a matter of routine to chip away at it consistently to eventually see it all working.

Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2023.csv).
