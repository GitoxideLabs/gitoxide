This month once again felt a bit slow, but as opposed to what happened in August and judging by the amount of bullet points in the outline, this time it's real.

The focus _should_ have been on finishing remote abstractions to get to fetch a pack and update refs, but little did I know… and look at these wonderful rabbit holes at the side of the road!

## git-credentials

The major rabbit hole which cost quite some time was caused by me realizing, while integrating authentication into the remote abstraction, that I didn't really understand credentials and their helpers. Thus far that wasn't necessary either as the previous implementation would simply call `git credential`, which turns out to call all helpers according to the git configuration. While this is the standard way, calling `git credential` also means it applies the restrictive git security policy which `gitoxide` treats more flexibly thanks to its notion of _trusted configuration_. Thus, using `git credential` may not work with refusing to work entirely.

I took this as an incentive to dig deeper to seek a complete understanding of what's happening, which ended up in me tricking git into 'leaking' the information I needed to construct a baseline against which I could test my own implementation, which is made to fully agree with all the complex ways git credentials can be configured.

The outcome of this endeavor is `gitoxide` using its own git credential helper driver, with options for custom implementations in the form of a simple closure, or the built-in credential helper like before.

#### new git-prompt crate

Before I forget, the hole is deeper. It turns out that `git credential` doesn't only run helper programs, but it also prompts for user input to fill in username and passwords, or calls other programs to do exactly that. Turns out that prompting in a cross-platform way and with the option to not print characters isn't super easy and I spent some time mixing and matching existing solutions, and limited myself to unix only to maintain sanity. Thus, `gitoxide` won't be able to prompt on windows, but in typical configurations it won't have to, either, as the credential helpers do that for me or because helper programs are configured by default.

#### new git-command crate

When understanding how credential helpers are actually invoked and how these special 'scripted' helpers are implemented, it became clear that a custom version of `std::process::Command` was needed to emulate git's capabilities. This was fun and not too tricky at all.
I also opted for running credential helper programs using `git credential-<foo>` (as opposed to `git-credential-<foo>` where <foo> is the helper's name to auto-inherit the built-in git executable path. Without it, credential helpers can't be found, and detecting the executable path requires another git invocation which this successfully avoids.

#### Loading the 'special' git configuration

Finally, to be able to successfully mimic `git credential` with `gix credential`, the latter had to be launched with exactly the same configuration files as git itself. Unfortunately git distributions may be configured with a special 'distribution configuration', which sometimes sees itself as part of the system scope, sometimes the scope is entirely unknown. There seemed to be no other way to launch `git` (on demand) to see what it's top-most configuration is to extract said configuration path, and add it to the configuration stack. Ultimately this functionality was built-in directly into `git-config`, which is selectively used by `gix` to load that configuration when it thinks the sub-command may need it.

## RefMaps for remotes, and `gix remote ref-map`

Before fetching a pack one has to gather information about what the remote side has, and compare it to what _we_ have based on corresponding tracking branches, which serve as a handle into the local commit graph. The mapping is performed by, to me, one of the most important but probably least understood facility: ref-specs (with an 'f', not a 'v' :D).

The reason for this discrepancy might be that these are auto-configured, and typically never touched. And that's great because they turned out to be quite complex not only for parsing, but also for matching. The operation of matching allows to pass references advertised by the remote and obtain mappings to map remote references to their local counterparts, if available. This mapping is validated and sanitized, and now accessible via `gix remote ref-map`.

A neat optimization is also implemented, as the protocol V2 supports the special `ref-prefix` argument which tells the server to only return tips which have those prefixes, instead of all of them, which can improve performance greatly knowing that some servers have 100k tips and more. `gitoxide` now extracts prefixes from ref-specs and greatly reduces the amount of reference received to only what's needed.

## Community

### Celebrating 100k lines of code

In case you have missed it, here is the post [on reddit](https://www.reddit.com/r/rust/comments/xj6ncq/media_gitoxide_celebrating_100k_lines_of_rust_code/). This has to come with the correction that it is in fact just code, not Rust code exclusively. Tokei now reports 95k of Rust, so nearly there.

`codevis`, the tool behind the picture, was irresistible to me as it produces pretty visualizations and is hackable! After trying it I was inspired and wanted to play more, so I added `clap` to what was just a pre-configured program before. From there, I ended up adding `prodash` for nicer progress, multi-threading and various optimizations to be able to render pictures of any size even on memory constrained systems.

And somehow I feel that I am not quite done with it yet, let's see when I can pick it up once again.

### `crates-index-diff` now uses `gitoxide`, powering `docs.rs` updates (soon)

When I was pinged from a [docs.rs](http://docs.rs/) PR I was happy and sad. Happy because I love to be part of the [docs.rs](http://docs.rs/) team, sad because the issue at hand required me to touch `crates-index-diff` once again which has been on life-support for the past year or so. After all, tests weren't properly isolated and mostly deactivated, with most features being contributed. And due to diffing support and fetches, the whole thing was running on `git2` as well.

So I decided to go all in and replace `git2` with `gitoxide` where possible (that is in all parts that don't clone or fetch), while implementing the necessary diffing using the `gitoxide` plumbing that was already available. But before all that, I spent a moment to isolate all tests by generating a sub-set of the real index that could be checked into git, from which another script generates a partial repository on the fly leveraging `gitoxide`s test suite via `git-testtools`. After adapting the tests to also leverage fancy rev-specs for referring to commits using commit messages (a trick I had learned while implementing rev-specs), the whole suite of mostly rewritten tests was up and running, self-contained and more precise than ever before. From there it was just a couple of steps before `gitoxide` could do all the heavy lifting of diffing trees while `similar` was tasked with producing line diffs, vital to see which crates actually changed and how.

The test suite was even put to the…test as a couple of bugs were reported which could be reproduced by putting diffs on top of the pre-extracted parts - and all that worked without having to adjust most of the tests. Neat, now I love `crates-index-diff`, once again. 

### The Helix Editor gears up for `gitoxide`

What a nice surprise when I was pinged from yet another PR that introduced `gitoxide` to the helix editor to provide gutter information with diffs between the editor state and the baseline coming from git. Ultimately `gitoxide` is only used to pull out a single object from git, and it's the second occasion quite a lot of boilerplate was added to restrict discovery. This probably tells me something, and I will be looking at improving that which should benefit `starship` as well.

This PR, as it deals with diffing, also made me go on a little side-quest to finally push diffing in `gitoxide` further, especially since it's now much clearer after the work done on `crates-index-diff`.

#### `git-diff` with lines diffing, powered by `similar`

So I sat down with the goal to make existing diff code in `crates-index-diff` a bit nicer, which came down to a high-level `tree-to-tree` diff API in `git-repository` paired with a lower-level `blob-to-blob` diffing facility in `git-diff`, which really is nothing more than a couple of lines to directly expose the awesome `similar` crate.

With diffing being so easy, it felt like it needed to be applied a little more, maybe… on the linux kernel?

#### `ein tool hours --file-stats --line-stats`

And so the idea was born to upgrade `ein tool estimate-hours` to also collect information about the amount of added/remoted/modified files and added/removed lines (without rename tracking). What made me proud of this tool thus far was its peak memory usage being about 1/10th of what git needs for `git shortlog -sne`, and it's performance of being 15% faster than `git`. These attributes shouldn't change or at least not significantly when adding optional diffing support.

In the end the threading was kept simple, using my new favorite `std::thread::scope()` along with an upgrade `prodash` to share atomic counters directly across threads. The data structure were kept separate to only marginally increase the required memory for the commit-information collection, which is now amended with statistical data that is produced by worker threads. `prodash` works beautifully here as it visualizes exactly what's going on - all in parallel, with tasks operating at vastly different speeds.

After quite a couple of bug-fixes it's finally in a state where the line-counts are pretty close to what GitHub shows, and it probably only gets closer. Running it on the linux kernel takes about 30 minutes with line-counts enabled, and it was interesting to see how many lines Linus Torvalds actually changed over the years. It's less than I thought!

It's worth noting that the linux kernel repo also exposed an invalid expectation in the `git-odb` crate, which could fortunately easily be fixed despite being unable to reproduce it in testing as it involves a hard to reproduce situation across packs that… I felt unable to reproduce currently (`gitoxide` can't yet deltify packs). The underlying issue (even though it was brought up by `.expect()`) was that between calls, a data structure could change, but was subsequently (after the change) accessed using an index obtained before the change. I can hardly imagine how bad this bug might have been in other programming languages.

## Rust Foundation sponsorship: cargo shallow clones update

This section I want to keep in all sponsor updates moving forward to the end of the year.

I am in a hurry. There isn't much time left till the mid-term update and I didn't yet fetch a pack with the remote abstraction, nor did I update references. Doing so shouldn't be too hard after todays work showed me the way and cleared up a few things. My goal is to get it to work 'quickly' and post a first PR with `gitoxide` doing a shallow clone of the crates index, even though some details might still be 'hacky' to get there. It's doable, but it's a sprint for sure!

Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2022.csv).