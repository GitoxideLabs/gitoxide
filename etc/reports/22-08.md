This month felt a little slow in times and getting into the 'groove' of handling git remotes took me longer than I had hoped. Let's see if feelings and facts line up.

## rev-spec resolution with perfect disambiguation

The parser emits callbacks to a delegate which can maintain all the state required to handle typical revision specification parsing, namely:

- disambiguation
- ranges
- helpful errors

###### Disambiguation

Disambiguation is an interesting topic as I didn't even see it at first, after all it's such a rarely used feature at least in 'typical' repositories. Thus I shouldn't have been surprised when I realized that `git` does a lot to assure disambiguation is happening and helpful, culminating in a nice listing of ambiguous objects should it fail.
For `gitoxide` to handle this at least as well, `git-odb` had to learn how to provide ambiguous objects to the caller instead of just baking general information into the return value. With that in place and test scenarios lifted from the git test suite (how else would one generate commits with a 000000 prefix?) it was possible to make the entire resolution engine aware of ambiguities from the ground up. And the algorithm is really simple: run the entire rev-spec against all candidates and if only one candidate remains, there is no ambiguity. This is why `gitoxide` is able to resolve rev-specs that are ambiguous for `git`, probably
because `git` has a lot of 'special cases' for that without disambiguation being built-in like it is in `gitoxide`.

###### Ranges

Ranges are interesting because a single rev-spec can actually contain of up to two specifications, combining both in a range. Building range support into the parser turned out to be a good idea as it made additional information useful for… disambiguation… easy to obtain and a natural part of the process. In other word, if a range is present, we know we are looking for commit objects.


###### Helpful Errors

Error handling is important and they are naturally split into parser errors, somewhat lower level, and errors happening during resolution. The goal here was to not degenerate error information so it maintains a list of errors which are chained together before presenting them to the user. This was the only way I saw to present them easily even though a typical application will show them as a chain of errors that caused each other, which is not actually correct. However, thanks to special casing of …disambiguation… it was possible to produce nice and helpful errors nonetheless that are typically far better than what git produces - no surprise there.

###### Testing

With something as complex as rev-parsing, it was clear that `git` needed to be consulted to create a baseline. This baseline is pre-produced and cached on disk, with the results only being parsed from a simple text file, making them available to the tests almost instantly without having to execute `git` again.
That way I could make sure that `gitoxide` agrees with all results that `git` produced, except for when it is better.


###### Regex support

Erm, what? Yes, `rev-specs` can use regexes to do a fuzzy-search on commits. `gitoxide` can do that too using the exemplary `regex` crate should it be compiled in - it's optional, or just doing a sub-string search otherwise. Thanks to this, we can actually compare the performance of both implementations as these searches can be …exhaustive. More on that in a bit.

###### Completeness

Nearly all of the various transformations are implemented, with a few postponed to later once certain information they require is more clear to me. Nonetheless, I believe even now it's possible to resolve 95% of all rev-specs correctly.

All in all, it's already something I use as `git rev-parse` replacement.

## `gix rev parse` command

To make rev-parsing more accessible I put it into `gix rev parse`, which can also `--explain` what the spec does (a feature I found very helpful) or `--cat-file` right away because more often than note you want to plug `rev-parse` into `cat-file`, at least that's what I end up doing.

What's notable is its performance, as when constructing worst-case scenarios with its regex support one can quickly see that in some cases, `gitoxide` finishes twice as fast. This is mostly due to it being able to traverse commits about 10% faster than `git` with `regex` being much faster than whatever `git` is using. Mentioning performance here is nothing more than a side-note though as most of the time with typical rev-specs, it won't matter at all.

## The flattening of `gix repo`

With `gix` gaining more and more commands that are benefiting from `git-repository`, it seemed only reasonable to make these commands more accessible as well by removing the `repo` sub-command and folding all commands that need a repository to the top-level. This also means that all commands that where there previously have now been placed under the `no-repo|free` sub-command. These of course still have their use but will be generally be more niche than what's now on the top-level. Definitely a win for ergonomics.

## complete `ref-spec` parsing

Note the `f` instead of the `v`, as `gitoxide` can now parse _reference specifications_ as they are used in git remotes. Working on this was interesting as I realized how little I knew, and how ambiguous/flexible these specs are. This made didn't affect the difficulty of parsing them, but it made it harder to understand all the valid state one had to consider when interpreting them. It didn't help that ref-specs for _fetching_ aren't the same as those for _pushing_.

Many tests with `git`-baseline later I managed to boil all valid states into an `Instruction` enum along with all the information I could gather as accompanying documentation, as not everything seemed feasible or worth it to put into the type system. As of now, this `Instruction` type is still not used for interpretation but I am optimistic that it's suitable enough to implement ref-spec matching correctly.

## The `Remote` abstraction with ref listing, also in 'async'

Probably it was this topic that made me feel I was slow moving this month. Part of it was definitely interesting as I could finally learn how remotes work, and how that relates to the remote configuration of branches in interplay with remotes themselves. `gitoxide` now makes it easy to get remotes based on the current head, as it implements the logic that `git` itself uses.

###### `gix remote refs`

Listing and finding remotes was quite straightforward to implement, but what was hard(er) at least in my head was making them do something useful. After all, what one wants is to connect to a url and start communicating with the remote side. The easiest interaction in the book is listing remote references, a capability that _was_ present via `gix free remote ref-list <url>`, which is now available via `gix remote refs`.

What made this harder what the requirement of supporting both blocking and async APIs which makes some sense for any kind of networked IO. It's not about performance at all, but about possibilities. Thank's to `gitoxide`s async support it's easy and straightforward to support frameworks like `quinn` for example to get `quic` support for connection with custom servers, which happens to be async only.

###### `git-config` improvements

What's cool is that in order to replace the previous _free_ command (`gix free remote ref-list`) with the new one I had to implement a way to set the transport protocol version. A first naive approach was to just add a `-p` flag, but it felt wrong in the light of `Remote::connect(…)` reading the `protocol.version` configuration variable. Wouldn't it be nice if one could set it on the command-line?

This required `gitoxide` to allow changing the configuration in memory via `Repository::config_snapshot_mut()`, which was trivial to implement due to the advanced state of `git-config`, but `git` had a surprise for me in the shape of a rule that I wasn't really aware of: booleans without an `=` sign are considered true and those with an equal sign but an empty value are considered false. Of course I baked this into the test for the feature that applies key-value pairs from the command-line to the configuration just to see that the test would not succeed: booleans worked a little differently.

Fortunately, after an hour of debugging, I was relieved by finding the exact right place to make booleans work while keeping the API consistent - the logic employed by `git-config` now feels right(er) and the last special case I was aware of related to how it stores keys and their values is much better understood.

## Community

### A slightly troubled `starship`

`starship`, a project with ~28k stars and probably _a lot_ of installations uses `gitoxide` since v1.10, and with such a massive install base it's no surprise that not everything worked perfectly. Something worth highlighting is that this leads to `gitoxide` being executed each time a developer presses enter in their terminal, so the amount of different configurations, machines and git repositories the code runs in must be enormous! But I digress :D.

The worst issue was a hang that would always happen if `starship` was executed on a machine with less than 6 usable cores, like, any laptop with 4 cores and hyper-threading disabled. It's probably even worse than it sounds. The cause was ultimately boiled down to [this issue](https://github.com/rayon-rs/rayon/issues/969), which was entirely in the dependency chain of `gitoxide` and once again flags `jwalk` in combination with `rayon` as troublesome. `jwalk` is used for parallel file system traversal, and it hangs due to an interaction with `rayon` if it runs within a `par_iter()`, something that `starship` happens to do. The overarching issue is that `gitoxide` now can become a hazard for any application that happens to run it within an outer `par_iter()`, and for now the only fix is for `gitoxide` to allow disabling parallel iteration in `jwalk` using a feature toggle. I truly hope this can be fixed at the root though as I am sure this will bite again in the future.

### Index V2 and V3 writing

Sidney has contributed index writing, a vital step when cloning repositories with a work-tree. It turned out to be a little harder than anticipated, but by now its robustly writing indices in V2 or V3 format along with the `tree` extension, the extension most important to quickly generate a tree from an index.
Thank you!

### `git_date::Time` formatting

Thanks to Svetlin there now is `git_date::Time::format(…)` which allows to use custom formats of the `time` crate along with many of the pre-defined formats available in `git log --date <format>`. It's a tiny, but powerful API and as always, there is much more work than anticipated and two additional standard formats are still to be implemented.

### [docs.rs](http://docs.rs/) crates with feature labels

Thanks to a contribution Poliorcetics the next time `gitoxide` gets released, there will be labels indicating the feature toggle that needs to be present for modules or methods to be available. Great work there, there is a ton I don't know about the Rust ecosystem and I am always happy to learn some new tricks :).

###  CI upgrades and performance improvements

Also thanks to Poliorcetics, the CI workflows have been updated to the latest versions of the actions they use, and that motivated me to try to improve their worst-case performance. It feels good to see CI get better, even though I'd love to make it faster, too :D.

## Rust Foundation sponsorship: cargo shallow clones update

This section I want to keep in all sponsor updates moving forward to the end of the year.

All work on Remotes is directly motivated by `cargo`, as `cargo` uses anonymous remotes to perform its operations. `gitoxide` supports these too now, and I aim my work towards achieving bare clones of repositories so the [crates.io](http://crates.io/) index can be cloned using `gitoixde`. This shouldn't take too long as there is only about 5 weeks left to integrate it into `cargo`. Fortunately I have found a good channel to communicate with the cargo team (Zulip), which allowed to clarify the next step, so all that's left is to get the actual (and a lot of) work done.

It's all very exciting and I can't wait to open the first PR in `cargo` - in September I should have a lot more to write about that.


Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2022.csv).