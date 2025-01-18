This month was diverse with a lot of progress, let's dive right in.

## Progress on the `worktree` checkout block

The current main avenue of progress is to clone a working tree in full and correctly. After setting records on how fast files can be checked out with the current architecture, I am now working on supporting all features.

### `git-attributes`

When checking out files, git reads `.gitattributes` files from various location to learn about attribute assignments per path it deals with. These are used to determine if additional processing should be applied using built-in transformations or user defined filters.

The `git-attributes` crate now allows to zero-copy parse ignore patterns from `.gitignore` files as well as `.gitattributes` files.

### `git-glob`

The patterns obtained from `git-attributes`-like files can then be used to match on paths to see if they are excluded or have attributes assigned. Git supports various shortcuts depending on the particular pattern at hand, but ultimately falls back to actual wildcard matching.

There was prior work that runs on many installations, as `ripgrep` reads and handles `.gitignore` files as well via the `ignore` crate. It uses `globsets` to match multiple patterns at once and has a test suite of about 150 patterns.

I took these patterns and created a test-suite to pass them through `git check-ignore` to learn if git arrives at the same conclusion. The answer seemed to be negative as there were deviations, but in hindsight I have a feeling I interpreted the test-suite incorrectly as it also tests for certain flags which don't translate to `git check-ignore`.  In any case, this led me to implement globbing from scratch based on git's implementation with the goal of being 100% matching.

For that I took another 250 test patterns from the git test suite and used it as baseline for my own implementation. Starting out with 75% disagreement, soon it would reach 50%, to ultimately match git's results perfectly. It was quite impressive to see how sensitive the wildcard implementation is to deviations. Whenever I took creative liberties they soon proved to cause test failures. For the most part, it really needed to be exactly what was done in C.

On the bright side, the implementation there is now is by all means idiomatic and I am happy to solid pattern matching to rely on for something as important as `git-attributes` as well as other parts of git.

### Attributes stack coming together

Now I finally am back on track to implement the actual 'attribute stack', which brings all the pieces together and allows for efficient lazy parsing of attributes and ignore files. Great emphasis is put on the test suite which once again establishes a baseline using `git check-ignore` and `git check-attr` accordingly to assure 100% agreement.

It took me a long time to get to the point where this implementation could be started primarily because git codebase has a lot of complexities related to features `gitoxide` doesn't yet support, cone patterns and sparse checkouts being the most notable one. However, I think it's beneficial to implement each part by itself to hopefully end up with a code base that easier to understand and extend.

Lastly, the culmination of this attribute stack work will be `check-attr` and `check-ignore` like sub-commands of the`gix` plumbing tool to put it through its paces.

## git-sec

With the arrival of the rather infamous release v2.35.2 of git it will now refuse to operate, that is 'do anything', on repositories that are not owned by the user executing the git process. Despite it being meant to prevent attacks on shared file systems where git could be coerced into executing arbitrary binaries merely by running `git status`, it also led to a fallout on CI systems around the world. And I can tell from my own experience as maintainer of `GitPython`.

The `git-sec` crate was created in response to provide a shared model of security for use by gitoxide plumbing crates. The central type of the `Trust` enum which can be derived from the ownership of a path, yielding either `Full` or  `Reduced` trust. From there, one can derive various security related settings depending on which trust level is encountered.

The trust model has been integrated into `git-repository` which now allows to configure what `git_repository::discover()` does depending on the ownership of the repositories it encounters. A `PoC` of permissions was also added to the `git-config` crate to allow more fine grained control as to how configuration files themselves can be used as well as their values.

The idea is that `gitoxide` will be able to use a `secure` mode for repositories that aren't owned by the current user which prevents the use of dangerous configuration values like paths to executables. That way, `gitoxide` tools are operational even on repositories that could be used for attacks otherwise.

This system is flexible enough to easily configure similar behaviour to what git does, and there are plans to add something like `--strict` or `--paranoid` to turn git-like behaviour on for `gix` and `ein` as well.

### Beefed up windows support

As crucial part of determining the trust level of a git repository is to determine if the current path is owned by the user running the executable. On unix, it's a piece of cake, but on windows that's a little more involved.

I decided to try the `windows` crates as it's produced by the vendor itself. My attempt to reproduce exactly what git does (on Windows) failed and after hours of running tests on CI I gave up and [reached out](https://github.com/microsoft/windows-rs/issues/1697) to actually receive help promptly.
And even though the current implementation isn't en-par with what git does as it includes group ownership, it's a step in the right direction that can later be tightened

While I was at it, I setup an ARM windows VM to help me run tests on windows directly using the GNU toolchain. It was more difficult than I'd have liked to get all the dev tools installed there, and learned the hard way that `git for windows SDK` really is your friend.

## Don't squelch errors

Somewhat on the side and due to the previous race condition in the ODB it became clear that `gitoxide` plumbing crates may never squelch errors of any kind by downgrading them into `Option` to avoid hiding legitimate issues by masking them as something else, like 'object not found'.

Now all crates which use an `FnMut(oid, buf) -> Option<Object>` closure have been upgraded to return a `Result` instead.

## Object-decode-iterators for the win

While improving the API surface of `git-repository` to be more useful for prospect users of the crates I also had to find definitive answers to how to expose object information. The question really was whether to expose the underlying object, or to return wrapped higher-level objects instead for convenience.

The answer now is usually: 'both', while avoiding any allocation when extracting fields of objects like commits by default. This is a trade-off geared towards leveraging the incredibly fast object parsing performance while avoiding memory fragmentation for allocations which could otherwise occur. This is done using the object iterators, that lazily return one decoded token at a time. That way one can stop decoding once the field of interest is reached, but one will also decode portions of the object multiple times if more than one field is requested.

This also means that users who want to access all fields of a commit, for example, are probably better off decoding the commit once and using the lower-level commit from the `git-object` crate, which is fully decoded once.

Additionally, all these object-decode-iterators will not squelch decode errors anymore.

## Community

### `gitoxide` lands in `onefetch`

Out of a desire to improve performance, the `onefetch` maintainers reached out to see if they could use `gitoxide` instead.
I went right to work which results in this PR being merged: https://github.com/o2sh/onefetch/pull/635 and me being a collaborator. `onefetch` is now ~2.2x faster in the repositories I tested and is more correct as well.

`git2`, however, is still needed for accessing the git configuration, so some more work is still waiting to be done to complete the transition from `git2` to `gitoxide` ([tracking issue](https://github.com/Byron/gitoxide/issues/364)).

#### `git-mailmap`

As part of the `onefetch` work support for `.mailmap` files was added. It's a simple but powerful way to change author and committer information after the fact which is picked up by tools like `git log`.

As usual, a sub-command was added to allow running the code on real repositories, so `gix mailmap verify` can check mailmaps for errors.
Additionally `ein tool estimate-hours` now has mailmap support.

#### Replacement objects

While at it, I thought 'might as well' fix a long-standing `onefetch` issue around the lack of support for `replacement objects`. Such replacements are stored using refs and allow to map one object to another, transparently. Effectively when someone tries to `find()` and object `x`, one will instead receive the contents of `y`, making this fully transparent.

Replacement objects are supported in fully trusted repositories, but are disabled in those with reduced trust.

### `gitoxide` submits PR to `vergen`

The work on adding `gitoxide` support to `vergen` has been on the way for quite a while now, and has now been concluded with the submission of a PR to receive feedback on the implementation. It adds a feature toggle to use `gitoxide` instead of `git2`, which could be fully replaced if `gitoxide` could determine if the worktree is dirty ([tracking issue](https://github.com/Byron/gitoxide/issues/298)).

And even though it's unclear if there is any interest, doing this work resulted in a variety of improvements to the `gitoxide` API that are worth it either way.

#### `git-revision::describe()`

A major feature added was `git-revision::describe()` support, which es a carbon copy of the functionality provided by `git describe`. Performance wise it's sometimes en-par, and sometimes a little slower than git itself, and it also doesn't make use of commit-graphs either. So there is a lot of opportunity for improvements to speed up the graph traversal which are at the core of the algorithm. There are no plans to do this anytime soon though, as it's definitely fast enough already for running on the client side.

#### MSRV-learnings

`vergen` also forced `gitoxide` to think more strongly about the MSRV, the minimal supported rust version, as it has one itself. `vergen` considers MSRV changes a breaking change, which is a stance that `gitoxide` adopted. After all, these changes can be breaking downstream.

This also mean that `gitoxide` had to test against the MSRV toolchain to maintain compatibility  which isn't an easy feat unfortunately. Currently it's the `windows` crates which prevent the MSRV to be met and caused me to open [an issue to learn more about their perspective on the matter](https://github.com/microsoft/windows-rs/issues/1699). My takeaway is that… `windows` doesn't really say and it's probably better to use `winapi` instead once I have some room to touch the windows related code again.

### `git-config` - a rabbit hole in disguise

Even though support for `include` paths has landed, implementing `includeIf` isn't quite as straightforward. For one, it needs 'globbing' support which fortunately is now available via `git-glob`, but it now also needs further considerations related to `git-sec` and various changes related to that.

On top of that, there is early support for parsing dates which led us to learn that dates in git are quite complex and that a ton of different formats are supported in various places, not only `git-config`. This led to the creation of the `git-date` crate, and a viable path trough the myriad of features still has to be plotted.

On the bright side, having `git-date` implemented will also come in handy when using `rev-specs`.

### The beginnings of `state()` information

Another contribution added support for in progress operations, and there is more interest in providing even more information about what's going on beyond the usual `git status`.

This may also set the foundation for `gitoxide` to be adopted by no other than [`starship`](https://starship.rs).

#### An upgrade to the test tools: now less surprising and with archives, and faster CI

As part of working with the contributors of the 'state' PR I saw a test-suite setup that inspired me to beef up the test-suite of `gitoxide` as well. After fixture scripts ran, they will now create `xz` compressed archives which are optionally added to the `gitoxide` repository via `git-lfs`. 

On non-linux platform these archives are then extracted to disk instead of running the fixture scripts, which can speed up tests significantly, particularly on windows where one fixture script ran for more than 2 minutes. Now if one is lucky with the windows VM on CI, one can run all jobs in 20 minutes, instead of 30.

Note that additional improvements were made to assure errors in fixture scripts are reproducible when tests are re-run - previously this wasn't the case which was surprising at best. 

## Where we are headed

Once attributes can be read during checkout the next big thing is on the table: handling built-in filters as well as user-defined ones. There, one will have to implement two different protocols to communicate with them, and I believe it's going to be a lot of fun for sure.

Once that is done, one might be able to think about how to implement `git-submodules` and checking them out, which seems like a big tasks I am happy to leave for another time.

Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2022.csv).