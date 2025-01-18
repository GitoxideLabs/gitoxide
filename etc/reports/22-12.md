This month is all about getting started with integrating `gitoxide` into `cargo`, and as you can imagine there is a lot of small (and bigger) things that are needed in `gitoxide` to make this happen without anybody noticing any difference except that everything is faster and better. 

Here come the details.

## The Cargo Integration has begun

On the first of December [the official integration PR](https://github.com/rust-lang/cargo/pull/11448) was created with the goal of doing all fetches using `gitoxide`. For that to work, plenty of improvements were required.

### Prodash with IDs

`gitoxide` uses `prodash` for hierarchical progress reporting. Each `progress` can have child-progress to correctly visualize complex tasks with a lot going on concurrently or in lock-step, as a tree of progress. `cargo` needs to show progress using its own progress bars and providing only select output, like how many compressed bytes are received and how many objects are included.

To enable this, `prodash` had to learn the concept of an ID which is a stable way to identify progress information within the progress tree, and `gitoxide` now provides IDs for all progress it creates.

`cargo` also had to changes  little as it was built around getting callbacks for progress, and now it's observing progress from a separate thread. This has the advantage that progress reporting can be as fast as one can increment an atomic counter, while progress reporting can happen at its own pace.

### Various fixes

With `cargo` having it's own and quite exhaustive test suite it showed plenty of shortcomings in the way `gitoxide` was implemented at the time. All it took were about 195 git-specific tests, with a few of them running into errors that needed fixing on `gitoxide`'s side.

##### Reuse connections of HTTP transport

A test against a stubbed HTTP server showed that `gitoxide` would not keep the connection alive despite this being the default for HTTP 1.1, causing the test to fail.

The fix was quite simple and just an oversight in the implementation, so I am really happy there was a test catching it. And of course, a similar test was added to the `gitoxide` test suite to prevent regression.

##### No UTF8 for ls-refs line parsing

When scouring through the `git-protocol` code it became evident that a token of tech-debt was still left: references would be forced into UTF-8 Strings before parsing them (as bytes) due to using the standard `lines()` iterator of a buffered reader. This is a problem as technically, UTF-8 isn't a requirement for git references at all so the conversion can fail. Further, the bytes to string conversion is another copy and allocation that is unnecessary.

With the introduction of a new trait it was possible to expose line-by-line reading that provides buffer-backed bytes instead to fix all prior problems in one go.

##### `git-url` improved handling of windows paths

`git-url` as a crate which makes the various kinds of allowed URLs accessible was quite stable recently, but `cargo` did manage to throw some new URLs at it that caused it to choke. Fixes were trivial, fortunately, but it remains to be seen what other URLs are able to go beyond any expectation.

##### Improved discovery of non-typical repositories

Cargo does it's best to be resilient against breaking git repositories and should never choke on corrupt or unexpected data coming from disk. And of course, there is a test for that which caused the repository-local configuration file to go missing. `git2` was fine with that, `git` was fine with that, but `gitoxide` really thought a `config` file has to be there.

Now `gitoxide` will assume default values which seems to be equivalent to what `git` does as well.

##### auto-tags

When implementing `fetch` I intentionally skipped the 'auto-tag' feature to save some time in that moment, thinking that something this 'obscure' probably isn't required from day one. However, it turns out that 'auto-tags' aren't obscure at all and implementing them is needed for correct fetches and clones.

`auto-tags` as a capability need to be enabled by the client so the server can automatically send along all tag objects that point to any commit it would normally be sending. That way, the client will have all objects necessary to automatically create tags that point to any object it received. This is a somewhat hidden feature when doing `git clone` as git won't inform about the tags it created

The `cargo` test, however, was performing a fetch with ref-specs that only refer to branches, but assumed that tag references would also be present automatically.

Now `gitoxide` behaves like it should and properly keeps track of implicit ref-specs like these, respecting typical `git` defaults out of the box.

## Another one of these eurekas: environment-to-config-mapping

It still amazes me that this far into the project, it's still possible to make changes that make it seem that a big piece of the puzzle finally fell into place. Previously, environment variables, despite already being gated behind a permission flag, could possibly be read lazily in various places of the `git-repository` case. For a library, that always troubled me as this effectively adds global state to the equation, littered all of the codebase. It never felt quite right.

However, since `git-config` rising to incredible importance within `gitoxide` there also was an advent of `gitoxide.` prefixed configuration variables for configuration of `gitoxide` specific values. Value being there are in the right place, the in that moment I connected the dots.

Why not have a single step which transforms environment variables, based on permissions, into respective configuration values? That way there is one place to rule all the access to environment variables, and it's done exactly once when the `Repository` is opened.

Now all code needing these values could just read it from the git configuration, which also makes these values accessible to API users who want to override them, now without having to set environment variables in their process at all.

## `gix` improvements

Due to the changes to integrate with `gitoxide`, a few improvements also landed in `gix` to make them available to a broader audience and to additional testing on actual repositories.

### `odb stats`

Having said the above, that *everything* was about `cargo`, it's probably fair to start out with an exception. With the publication of `noseyparker`, I couldn't resist to throw `gitoxide` at it to see how much faster it can go. It turned out, [quite a lot](https://github.com/praetorian-inc/noseyparker/pull/2)!

However, there was one feature missing to completely remove `git2` from there, which was related to quickly enumerating all objects to know the bounds of the pending scan.

For that, `repo.objects.header()` was added to only decode object headers, as fast as possible, without decoding the whole objects. In average, doing that as opposed to a full decode is about 8 times faster, which is also faster than what `git2` can offer.

With it, and full parallelism, an M1 Pro with 8.5 cores can calculate statistics on all objects within the linux kernel repo in 1.4s, or 6.7 million objects per second. This is too fast for any delta-caches by the way, and no matter what, my cache implementations that work fine when decoding objects wouldn't be able to speed up this computation at all (that is, the cost of the cache outweighed its benefits in all tested workloads) - interesting.

With it, there was also an option to change the iteration of all objects within an object database to be in pack order, which allows to use pack-delta caches much better when the entire object is decoded later.

All in all, I am happy `noseyparker` motivated me to finally implement the last missing feature in the object database. 

### `--strict` and `--no-verbose`

For completeness and control, the `--strict` flag was added to ensure that commands that are 'lenient' on the git configuration by default are forced to be strict. Being strict is the same way that `git` treats configuration as it will fail to perform any action if it contains malformed or invalid values. `gitoxide` implements a degree of leniency to allow commands like `gix config` to always display configuration despite the presence of malformed values.

With `gix fetch` and `gix clone` being actually useful, it was important to have a way to automatically be verbose to show progress information. To provide more control over it, there is now a `--no-verbose` flag to turn off progress reporting in case of sub-commands that turn it on by default.

### `clone` with auto-tags

Finally, `gix clone` performs clones that have a similar result as if `git clone`  would have been used, due to the implementation of `auto-tags`. To counter that and allow no tags to be cloned to keep the repo more minimal, one can now specify `--no-tags` as well.

## Community

### Worktree configuration support

Sidney contributed the ability to layer worktree-specific configuration on top of the one coming from the shared repository, providing a path to supporting sparse checkouts correctly. Thanks to his work, `gitoxide` will be supporting sparse checkouts by default as it's aware of all features of the `index` file from the very start.

### A fix for relative SCP URLs

Thanks to a user who employs `gitoxide` to clone and fetch repositories a fix was provided to deal with SCP-like git urls with relative paths correctly.

### Various impactful fixes to `git-date` crate

Thanks to the repeated engagement of one industrious individual `git-date` received a lot of impactful bugfixes. First to mention is a fix for incorrect handling of timezone offsets which made roundtrips impossible. Baseline tests now compare to the results in git more thoroughly as well which should prevent regressions and help future development.

Lastly, due to a method being entirely untested and due to being dependent on the system's local timezone it was never run in a place where the timezone offset would become negative. And exactly that wasn't possible, until now.

Thanks for all your work!

### Rust Foundation sponsorship: cargo shallow clones update

The last month on the grant to integrate `gitoxide` into cargo for shallow clones has begun and my plan is to make the PR reviewable this year 🎉. Doing mere fetches with `gitoxide` already provides substantial performance improvements particularly on Windows, so it's worth having even before shallow clone support lands.

It will definitely take more time to finish the integration and set `cargo` up to phase out `git2` as well.

Merry Christmas, and a happy new year,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2022.csv).
