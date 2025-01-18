This month, despite being very, very busy, feels fragmented and like there wasn't much progress towards outstanding `cargo` integration (the item with the red warning light blinking). In such moments I have to remind myself that every improvements helps the `cargo` integration at some point, and that not all of my schedule and work-items can be perfectly controlled. Sometimes one has to fix a bug, or interact with the growing community, and it's part of the process of a growing user base and, despite best attempts, shortcomings and oversights that show only once more people start using `gitoxide`.

## A personal note

Exactly a month ago, to the day, we set foot in Europe and arrived back in my hometown near Berlin after three years in China. As you can imagine, the days around the travel date didn't exactly help with productivity even though two weeks in I'd say everything was back to normal-enough.
Despite having had different plans, we are now in Germany permanently, and my git commits (and their local time offset) will bear witness to that :).

That also explains the briefness of the sponsor email last month, I shall correct that today ;).

## Advanced HTTP configuration via configurable transports

When analysing `cargo` it became clear that there are many uses of private crates indices or indices behind authenticated proxies, and thus far this was a problem.

First of all, there was no way to configure transports which are the only dynamic (aka `&dyn Transport`) type in the entire codebase. How do you configure something that isn't known? My answer to this was literally `&dyn Any` those in the know can try to runtime-cast to the concrete type accordingly. HTTP options are now provided by the 
`git-transport` crate to represent all common git http options, along with additional ones that I think are worth configuring. `git-repository` knows configuration by scheme and will assemble these structures to be passed to the transport.

When comparing this with `libgit2` it's so much cleaner - I could never really grasp why fetch options had proxy-settings littered into it. It was unclear to me that there is no general git proxy, but that it's only for HTTP transports. How would that ever scale to all the options and all possible transports, including those that are entirely application defined? With the `&dyn Any` approach there is perfect separation and flexibility, and it's probably as good as it gets.

What took longer than expected was to understand all options, reduce the set of options to the MVP that `cargo` would need and implement the settings in `git-transport`. Testing these is a problem as setting up test-cases would be very time-consuming. So for now, the application of settings is thoroughly under-tested and I hope one day
will be time be better than that.

##  The new blocking `reqwest` backend, sponsored by Codebase Labs

There has been a lot of movement in the HTTP space recently, and this one was triggered, and kindly sponsored by Codebase Labs. The initial implementation was pretty straightforward and at the time, I didn't understand its biggest value as to me it was 'just' another HTTP backend and somewhat redundant. What's wrong with `curl`, right?

Well, `curl` is written in C, needs a C compiler to build and pulls in `openssl-sys` which we know is more C that has trouble parsing things without messing up its memory.
For that reason, `reqwest` is one of the most exciting updates this month as it enables, for the first time, _pure Rust_ builds of `gitoxide`.

I dare to say that one day, it might be the default backend but until then there is along way to go as all common HTTP options would have to be faithfully implemented even though they are biased towards suiting `curl`, which is what `git` uses for HTTP as well.

## `max-pure` configuration to avoid all C dependencies

And here we are, the massive side-effect of the `reqwest` HTTP backend is the new `max-pure` build configuration of the `gitoxide` binaries. It coincidentally is exactly as it says on the tin and can build on a freshly installed Ubuntu 22 with just a `rustup` installed toolchain.

It's such an important property that I will soon add CI action to validate this will keep working.

As another side-effect of the side-effect I decided to push 'once more, with feeling', to get binary releases are back. These have been awfully absent for some months now as GitHub CI seemingly by itself decided to not trigger builds anymore. I couldn't figure out what happened and ended up adding a custom trigger, which is just one button to press after each `gitoxide` release. Well worth it, as these pre-built binary releases also include the `max-pure` build (along with `max`, `small` and `lean`).

## Various Fixes

With more people picking up `gitoxide` there was a noticeable uptick in issue reports. When these happen, I usually drop everything else to fix them as soon as possible.

Even though I am happy about `gitoxide` being better after that, the time I spend is removed from working on features that are important to `cargo` :/.

##### Fetching refs whose names collide on case-insensitive filesystems

This was a major bummer: `pytorch` couldn't be cloned (bare) because the refs could not be created, even though `git` can do that just fine, somehow. 
The apparent cause of the issue was that 'a lock could not be acquired because: there are too many tempfiles'. What? `git-ref` is crafted specifically to avoid such resource exhaustion. This simply could not be yet there was the error. A little bit of digging showed the `too many tempfiles` error was incorrectly produced by the `tempfile` crate (and [here is the fix](https://github.com/Stebalien/tempfile/pull/199). But the underlying issue was still present, as a lockfile could not be acquired because it was already there. You will be right thinking that this is the filename collision due to case-insensivity. To me hard to grasp was how I could have missed that before, nowhere in the
ref-file implementation of `git` had I seen special handling of case folding, and a few tests quickly revealed that `git` indeed does not handle that specifically.

When fetching, however, it works because refs are written into `packed-refs` right away if it's an initial clone, and `git` does not acquire a lock for that. After a thorough review and many more tests in `git-ref` I could reproduce all of these issues and assure that `git-ref` doesn't do worse than git. As a positive side-effect, writing a lot of 
refs is now much faster as well as it can (probably race-free) acquire only a single `packed-refs` lock to perform all updates, saving 10k temporary files being created and removed when cloning or fetching `pytorch`.

##### Fetch support for async transports

Previously this would only work for blocking transports as the operation (besides the IO) is CPU bound and inherently blocking. Codebase Labs sponsored this upgrade and now one can fetch with custom (and async) transports.

##### Clone correctness

Despite an MVP being present, one day I woke up realizing that many aspects have not been validated at all as it's actually quite a bit different from a 'fetch + checkout'. And when comparing to what `git` does, the current implementation, then, had too many shortcomings to not do anything about it.

Now we set the `HEAD` correctly, have the correct ref-logs, which, by the way, write the 'wrong' log if the is a collision on case-insensitive file systems probably just like git does, and we can also update symrefs. Previously symrefs where excluded because I mistakenly took `git`'s special handling of `HEAD` as indication that all symrefs are special.

Lastly, there was a silly bug (due lack of test-input) which caused a failure if the fetch remote branch would have additional sub-directories in it like `refs/heads/feature/a`. This was a typical case of over-reliance on Rust and spotty tests, and despite trying hard to test everything I managed not to think of that as possible input. Snap!

##### Better `diff` API and 2x performance with `imara-diff`

The diff-API has seen a massive upgrade not only in terms of performance, but also in terms of API. This is due to the switch to `imara-diff` which doesn't only boost performance, but also comes with its own callback-based API to adapt to.

One of its major benefits is that its author, [Pascal Kuthe](https://github.com/pascalkuthe/imara-diff), took the `git` implementation as reference and baseline. Along with the plan for further improvements it's the diff implementation that `gitoxide` always wanted, and I am glad Pascal took charge of it with passion and incredible drive.

## `crates-index-diff` performance and correctness upgrade ([docs.rs](http://docs.rs/) team work)

`crates-index-diff` was able to miss changes which caused a single crate version not to be built (as far as we know, at least), and that was quite a shock as I spent a lot of time already to isolate the test suite and test everything…except for certain edge cases apparently. Thanks to said test system it was possible to add the issue at hand as fixture for a reproduction. Fixing the issue was possible, but it was clear that the current diff implementation couldn't really be trusted. Who could know if such an issue 
couldn't happen again?

So I started to [work furiously](https://github.com/Byron/crates-index-diff-rs/issues/26) on solving the issue… by asking for help! [Pascal Kuthe](https://github.com/pascalkuthe) answered the call and completely redesigned the diffing algorithm within hours (and mostly at night!) into something that is faster AND simpler AND logically correct.

On the other side of this, I introduced a real-world baseline test against the current state of `crates-index` AND support for correct ordering of changes to give us proof that the implementation is indeed correct and won't miss anything. It's rewarding to see indeed no matter how you step through the commits, it will always end up at the correct final state as provided by the `crates-index` crate. Wonderful!

## Community

### Meta uses `gitoxide` for importing git into `mononoke`

Recently I learned that Meta is actively using `gitoxide` in their codebase and replaced `git2` for `git-import` into `mononoke`, and they keep expanding its usage. Neat!

### Foundation for handling sparse indices

Sidney was busy working on laying the basis for supporting any index file (aka `.git/index` that `starship` might encounter. There was quite some research to into sparse indices which are the major feature still (somewhat) missing, and by now it's well understood, too. Writing of indices with sparse-index extension is also possible now to one day support index-altering operations as well.

### Sponsorships submitted

Somehow it nearly went past me that submissions for sponsorships of the Rust Foundation were already due before information around a time-extension reached my inbox. I'd call that lucky! Having made the acquaintance with Pascal through his work I thought that he should definitely be able to get a grant given the quality and impact of his previous work. And so we talked and found the most impactful grant project we could imagine: improving the performance of pack generation and potentially an MVP of `gix upload-pack`, which is essential work to lead to a `git` repository server.

This means there are now three pending submissions, including Sidney's and mine. Sidney would be working on spreading `gitoxide` and implementing the features that a necessary for that (or migrating existing projects), and I would finish the integration of `gitoxide` into `cargo` by fully replacing `git2`.


### Rust Foundation sponsorship: cargo shallow clones update

The critical phase has nearly begun. There are three issues that prevent me from starting the integration as I know it's lacking, and no matter what, I will use December to work on the integration of what's there as much as possible.

The items that I work on till then are…

- advanced HTTP proxy configuration for the `curl` backend
- identifiable progress via `prodash`
- be able to turn buffered reading on or off at runtime to allow progress messages to be received in real-time.

The last point on the list is the most frightening for sure. Even though I feel I am late for the game I am confident that at the end of December there will at least be an MVP that clones the 'crates-index' in full, but does so much faster than with `git2` even though it's not yet shallow due to me not having researched supporting it in `gitoxide` yet or the specifics of the `shallow` protocol.

The end of year is going to be very, very busy!

Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2022.csv).