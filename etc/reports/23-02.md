This feels a bit strange as four weeks have been passed with me being busy at all times, yet it feels there isn't much to show for in terms of usable features, most of the month felt like maintenance due to plenty of bug-fixes, many of them contributed, and improvements to the codebase that were necessary. None of these feel like a great accomplishment though (even though future self will be thanking me for it, I am sure).

## Static git-config keys

The majority of the month I worked on what felt like the most arduous work in a long, long time. Triggered by a `cargo` review note, there had to be a way to avoid 'stringy' configuration entirely as it's generally error prone. The key could be wrong, or the value could be invalid, which would cause a failure only once the value is (lazily) used, further making debugging difficult.

The solution was to declare the entire tree of configuration keys statically, which allows keys like `core.bare` to be accessed like `gix::config::tree::Core::BARE`, which can be turned into a stringy key-value assignment `gix::config::tree::Core::BARE::validated_assignment_fmt(&false)?`. Furthermore, the `gix progress` sub-command was made so that it uses the configuration tree to discover all used keys at runtime, while being renamed to `gix config-tree` to finally make more sense.

Nearly 90 configuration keys were transformed that way and are now used within the codebase as well - thus each usage of a key can quickly be discovered with code-intelligence which should also help to, one day, document which method is affected by a configuration key.

Lastly, implementing one key at a time opened the configuration up for great documentation and unit-testing, which overall will indeed lead to higher quality and ease of maintenance.

So despite the the task having felt like a chore, it was definitely worth it. Thanks for making me!

## `git-repository` is now `gix` - or 'The Great Renaming'

Names are hard. So hard, that a project sometimes has to be renamed thrice! First it was called `grit`, which already used the `git-` prefix though, just to later be renamed to `gitoxide`. This turned the `grit` binary into `gix` and `gixp` (`p` for plumbing) as well and the world was alright. During that time, `git-repository` was introduced with a heavy heart since it never felt quite right - it should have been `git` but that name was long gone, unfortunately. This is when I started to to `use git_repository as git` by default to make up for that shortcoming.

Fast-forward to today and I tried to pull the same stunt, quite the norm for me, in `cargo` and after some convincing I finally confronted my daemons (read: tech-debt) and decided to rename `git-*` to `gix-*` and `git-repository` to `gix` while the name was still available! Of course it was a chore, but also … did it *feel good*! Besides using `gix` as entry point for `gix-*` plumbing crates seeming logical, it also feels good to finally 'stand on your own'. In a way, I doomed `gitoxide` to forever try to be something it is not and will never be: `git`. But it's its very own take on `git` and now gets to be called `gix` for that - wonderful!

Thanks again, for making me!

## 3x to 20x performance for `Git-Heat-Map` (and a good reason for rename-tracking)

One day on hackernews, there was link to a tool which could visualize paths of a git repository according to their 'hotness'. The tool worked in two stages, first a python script would read from `git log` and populate a sqlite database, then another python program would visualize that data.

Since it was an official goal to seek ways to improve performance (68 minutes for the linux kernel, 6 minutes for cpython), it felt like the project was calling out to me. It didn't even take a whole day to come up with the first working prototype at https://github.com/jmforsythe/Git-Heat-Map/pull/6 which, after some tuning, would greatly outperform `git log` thanks to … trivial multi-threading (which I will have to thank Rust for).

A major feature of `Git-Heat-Map` was rename tracking though, so I took this as an incentive to implement it. And even though rename-tracking was, in theory, available now with seemingly amazing performance (as in, it really doesn't slow you down and copy tracking can also be activated even), beyond a few unit-tests there wasn't much reason for confidence. Luckily, running the `db-gen` Rust program on the linux kernel quickly revealed shortcomings to the point where  database constraints showed how buggy the input was.

It took a whole day and a couple of more realistic tests to finally realize that the `git-diff` plumbing implementation is indeed buggy, with a fix being out of reach at least for today :/. Of course, it's going to work eventually and I can't wait to finally finalize the PR.

## `ein tool query trace-path`

Of course, `Git-Heat-Map` shouldn't be the only one one with that database-goodness, so I devised a tool I would want to use myself. Namely: following a file through the entire history of a repository and show its names, and copies, while we are at it, with diff-stats sprinkled in for good measure.

The query engine is a simple sqlite database which is updated only when necessary, to assure the 20 minutes wait for the linux kernel is only done once. With the database up-to-date, the first query, `trace-path`, is nearly implemented except that it really, really needs a working diff implementation to provide sensible results.

The tool was also useful to validate the underlying diff implementation, as even if the database creation doesn't fail, the values in the database are definitely too broken just yet. But we will get there, and once its done I am sure more queries can be added to make it even more useful.

To be continued!

## Community

### Helix (and rust-analyser) support

[Pascal Kuthe](https://github.com/pascalkuthe) rightfully pointed out that due to cyclical dependencies rust-analyser would fail to complete the analysis, making development of `gitoxide` in `helix` or VSCode less than stellar. We worked together to device a massive fix which reorganized dependencies and resolved cycles . Doing so also, once more, broken `cargo smart-release` which has seen a couple of fixes to deal with having duplicate workspace crates, one coming from [crates.io](http://crates.io/), and the other from the local drive. 
Thanks for all your help!

### OSS-Fuzz integration

It was an innocuous PR that [silvergasp](https://github.com/silvergasp) submitted one day (and wasn't the last one either), but the effects are now visible each time CI runs, and sometimes by new issues being opened by a certain fuzz-bot when new malformed input was found. `gitoxide` already did fuzzing, at least on paper, as there are fuzz-tests that were used to find a fair amount of bugs already - since we are using Rust, those are 'only' panics.

But now, thanks to OSS-Fuzz, `gitoxide` benefits from Google compute to get fuzzed regularly, with a 10 minute fuzz kicking off each time a PR is pushed as well.

As a positive side-effect, `silvergasp` also gets to help the `time` crate getting into the OSS-Fuzz project, which will help `gix-date` become more robust automatically in a typical win-win!

Thanks, `silvergasp`, for driving this.

### WASM is a thing now

For a long time I WASM was very … fuzzy … for me as I didn't really know what to do about that, except for postponing thinking about it too much. Of course, one day `gitoxide` should run in the browser, but what exactly it would take was unclear to me, for a lack of experience.

Thanks to Paul@Codebase Labs I got the chance to dig in and fill the knowledge gaps to the point where a new WASM pipeline is now verifying that the crates that already compile to `wasm32-unknown-unknown` and `wasm32-wasi` will stay that way. More interestingly, `git-pack` will now compile to WASM which should allow packs to be streaming-decoded and resolved even in a browser.  The jury is still out on that one though, after all the runtime is not actually tested just yet.

Thank you!

### Fixed ceiling directory logic during repository discovery

Did you know that when standing 'on' the ceiling, you are already past it? It's a typical boundary condition that was never tested for, and thus it anded up to be incorrect. Thanks to a contribution this issue is no more.

But that's not all, as it turned out that ceiling directories passed in by environment variable may come in all shapes, including UNC paths on windows. These couldn't work as they are essentially their very own namespace, but fortunately a fix is possible 90% of the time by bringing them back into the typical namespace for paths. `starship` is a great driver for even the fringiest feature around repository discovery, thank you for helping `gitoxide` live up to the task.

Oh, and before I forget, when ceiling dirs are set using the environment variable, `gix-discover` will allow the passed in directory to be outside the ceiling, which otherwise is not allowed, bringing it closer to how `git` performs repository discovery. There truly are more subtleties than one can count, for sure.

### Fixed SSH transport

I wasn't quite aware how broken the ssh transport was. It works by shelling out to `ssh` just like git, but unlike git I failed to properly quote the path to the remote repository, making it impossible to find it on the other side.

But that was not it, as the the url parser, something I didn't touch in a while, wasn't able to handle scp-like paths which rely on ssh-configuration and host mappings to work. Those host-aliases didn't have a `.` in them which the classifier relied on. Thank to everyone who called out the issues and made the ssh transport more usable.

### From X to 0 (?) typos

Another contributor seems to have applied a tool which was able to reliably eliminate all typos in the entire codebase. This also renamed a few variants of errors, technically a breaking change, but I gambled and hoped that nobody would actually match on these. So far it seems to have been a winning bet.

### Rust Foundation sponsorship: cargo shallow clones update

We are getting close, I feel, and after having addressed a lot of review notes  it feels like the review notes get less numerous and smaller in scope. Most definitely a good sign. Since addressing review notes doesn't take too much time these days, I am optimistic that soon I can implement shallow clone support and make that available to `cargo` as well, then in a much smaller PR which will be mergeable much more quickly.

This doesn't change the fact that at least three major features are still waiting to be implemented to be able to replace git2, but… we will get there :)!

Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2023.csv).