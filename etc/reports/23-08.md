Another intense month with what feels like a massive amount of meaningful progress, and a lot of community interaction. Let's dive right in.

## Pathspec matching

Pathspecs are an interesting bunch as we all use them, even though I dare to say that fewest are aware of them. And with that lack of awareness, many might mistake them for features of the shell, after all, we can easily do `git add *.rs`, and all Rust files will be added to the index. But did you know that `git add '*.rs'` also works? Now the expansion is done by `git` instead of the shell.

And there is more, so much more! `gitoxide` previously could already parse pathspecs in all their `:(attr:export-ignore,icase,glob,exclude)glory` but in order to answer the question of whether a submodule is active or not, pathspec matching was required. The first time I stumbled over them proper matching seemed impossible due to a lack of basic building blocks, but these days in the presence of great `.gitattributes` and `.gitignore` matching, it seemed more than doable for the first time.

As usual, I setup a test suite that created a baseline with `git` against which `gix-pathspec` would then run to compare its results. That way, along with reading all related tests in the `git` repository, I was able to find and properly implement a couple of edge-cases. Further, I'd find strange behaviour in `git` where `gitoxide` definitely does better.

All this work culminated in all `gix` (CLI) commands with pathspecs now using the matching engine, the most notable being `gix index entries` (AKA `git ls-files`), which got [its own blog-post](https://github.com/Byron/gitoxide/discussions/978) to highlight the performance benefits. The result of this is that `gix index entries` is 1.5x to 2.5x faster than `git`. And all that without 'multi-threading trickery'.

## Submodule Support

With pathspecs out of the way, there was nothing in the way of implementing `git` submodules. Those have always been a weak-spot for me, at least last time I implemented them in Python, probably just 13 years ago, along with all the wrong abstractions.

This time I stuck to the specification so everything in `gix-submodule` now represents the `.gitmodules` file along with utilities to deal with everything else that is needed to properly interpret its data. It's like a type-safe `.gitmodules` file.  

##### `is-active`

Did you know that submodules can be active or inactive? Inactive ones don't participate in `git submodule` commands and, for example, are generally not fetched or otherwise handled. What's interesting was the innocent looking `submodule.active` configuration key which is a multi-string value that represents pathspecs (!) to match against submodule names. Now, pathspec matching alone wouldn't be difficult even though it needs a little state, but unfortunately pathspecs can also match against attributes. And with that, one needs a whole stack of additional state to deal with `.gitattributes` files being loaded and unloaded depending on the path (or name) being matched.

##### `gix` integration

With `gix-submodule` being low-level and a `.gitmodules` abstraction, now it was necessary to bend this into place to look like there is actual submodules that can be listed, similar to how `git2` does it. All the state that is needed to, mostly, answer the `is-active` question would neatly be tucked inside. With that, the new `gix::Submodule` type is the first its kind which avoids the `Platform` moniker to use internal shared, mutable state instead. All that is combined with accessors that produce a guaranteed-to-be-uptodate `.gitmodules` instance, which is then shared among all `gix::Submodule` instances, one per submodule name we encounter.

##### `gix submodule`

All that culminated in a new `gix subomdule` sub-command which can, for now, only list submodules. It does so very quickly though and is generally faster than `git`, even though it's playing in its own league as it doesn't try to be exactly what `git submodule` is, but what I want it to be.

Thus it now spits out all kinds of useful information that I always wanted to know. In the Rust repository, this looks like this:

```
❯ gix submodule
 ✓ src/tools/rust-installer config:yes head:300b5ec index:300b5ec (master) [https://github.com/rust-lang/rust-installer.git]
 ✓ src/doc/nomicon config:yes head:8d1e4dc index:8d1e4dc (8d1e4dc) [https://github.com/rust-lang/nomicon.git]
 ✓ src/tools/cargo config:yes head:efd4ca3dc index:efd4ca3dc (0.59.0-960-gefd4ca3dc) [https://github.com/rust-lang/cargo.git]
 ✓ src/doc/reference config:yes head:f3d3953 index:f3d3953 (origin/update-recursion_limit-1001-gf3d3953) [https://github.com/rust-lang/reference.git]
 ✓ src/doc/book config:yes head:36383b4d index:36383b4d (rand-0.8-588-g36383b4d) [https://github.com/rust-lang/book.git]
 ✓ src/tools/rls config:yes head:4d8b0a1 index:4d8b0a1 (master) [https://github.com/rust-lang/rls.git]
 ✓ src/tools/miri config:yes head:50ef22af index:50ef22af (50ef22af) [https://github.com/rust-lang/miri.git]
 ✓ src/doc/rust-by-example config:yes head:ee342dc index:ee342dc (ee342dc) [https://github.com/rust-lang/rust-by-example.git]
 ✓ library/stdarch config:yes head:28335054 index:28335054 (0.0.3-1278-g28335054) [https://github.com/rust-lang/stdarch.git]
 ✓ src/doc/rustc-dev-guide config:yes head:04f3cf0 index:04f3cf0 (04f3cf0) [https://github.com/rust-lang/rustc-dev-guide.git]
 ✓ src/doc/edition-guide config:yes head:c55611d index:c55611d (c55611d) [https://github.com/rust-lang/edition-guide.git]
 ✓ src/llvm-project config:yes head:e3be3f64ecac index:e3be3f64ecac (origin/rustc/15.0-2022-08-09) [https://github.com/rust-lang/llvm-project.git]
 ✓ src/doc/embedded-book config:yes head:befe684 index:befe684 (master) [https://github.com/rust-embedded/book.git]
 ✓ library/backtrace config:yes head:4e5a3f7 index:4e5a3f7 (0.3.65-2-g4e5a3f7) [https://github.com/rust-lang/backtrace-rs.git]
```

It's dense, but all information one might want about submodules and a good reminder that `gix` is a developer and debugging utility, not a `git` clone.


##### `gix index entries --recurse-submodules`

Of course, now that pathspecs are handled properly in `gix index entries`, I had to make use of submodules too. How cool would it be if `gix index entries '*.rs' -r` could show all Rust files across all submodules, while making it appear like one big tree?

The abstractions held up very well and effort went into the refactoring that was necessary to allow recursing into submodules. Interestingly the code became more readable due to the additional abstractions, and as a side-effect I also beefed up the statistics which are now recursive and collected for each submodule separately. Now it couldn't be easier to to not only see which attributes are applied to each path, but also to learn which attributes are used across all submodules.

And last but not least, performance isn't negatively affected so `gitoxide` is still twice as fast as `git` at listing all index entries with submodules of the Rust repository.

## Community

A lot happened in this realm, which was a major contributor to making this month feel amazingly productive.

### crates-index v2.0 powered by `gix`

Last month ended with [this PR](https://github.com/frewsxcv/rust-crates-index/pull/129) being ready for review. Interestingly I was proposed as new maintainer and ended up merging my own PR. Of course I couldn't resist to finally do all the changes and improvements I thought were additionally necessary while producing the first PR. All this led to a new 2.0 release just a couple of day after the 1.0 release produced by the previous maintainer. Not the most optimal timing, but necessary for me to feel comfortable maintaining the crate.

Finally, with `gix` powering `crates-index`, I was able to remove `git2` as dependency from `cargo smart-release` which now also moved into [its own repository](https://github.com/Byron/cargo-smart-release)

At this point I should definitely mention the alternative to `crates-index` which I also just recently became aware of: [`tame-index`](https://crates.io/crates/tame-index). To me it felt like `tame-index`, a fork of `crates-index`, has received all the updates and improvements that didn't make it into `crates-index` in time and I keep endorsing it as the more actively maintained crate of the two.

### Rustsec `gix` conversion

With `crates-index` making the jump to `gix`, `rust-sec` was finally unblocked and could start their own move. Then everything went very quickly and by now there seems to be no trace of `git2` left in the source code. 

### Signature and `gix commit verify`

Required for `rustsec` was also a feature that would allow to 'reverse' the signing process, to validate the signature. I never thought about it before, but what git does it technically impossible: It puts a signature in something that is signed. A typical case of recursive loop. Unless, that is, one inserts the signature after producing it in such a way that it can be removed reproducibly. With the signature removed, one will obtain the source data that produced the signature, a requirement for validation.

And just to be sure the implementation for separating the signature from signed data in case of a commit truly works, I couldn't resist to implement an MVP of `gix commit verify`. Just like `git` it runs the `gpg` program to perform the validation and to my big surprise, it just worked.

### Winnow migration finished

Thanks to Ed Page, all parsers are now up to 20% faster thanks to the switch from `nom` to `winnow` (pronounced 'winno', not 'win-now'). And as a welcome side-effect, the code is simpler and more readable. Thank you Ed for this major contribution, and for hunting performance phantoms with me ;)!

### `gix archive` blog post

A couple of days after pushing `gix archive` over the finishing line, I felt the urge to write a post that would collect performance numbers a bit more scientifically than before. To my great surprise, it turned out to be much faster than initially thought, especially after moving from `libflate` to `flate2`, easily making it 2 times faster than `git archive`.

And here it is if you want to take a look at the details:  https://github.com/Byron/gitoxide/discussions/969 .

### First step towards improving compile times

And last but not least, I decided to use the incredible momentum brought in by the maintainer of `cargo-binstall` and get started with working on de-monomorphization as well as feature toggles to allow reducing dependencies. To do that, I thought it might be useful to have [an overview of the relationships between gix components](https://github.com/Byron/gitoxide/issues/987#issuecomment-1685023223), which I am sure will be very helpful in deciding where to apply feature toggles.

I hope that over time, `gix` can start to compile faster for all of those who don't need all of its features, and that this light-ness will then be more easily maintained moving forward.

### Rust Foundation sponsorship is now "Gix in Cargo"

Yes, the sponsorship is finally concluded and I have written my report. Effectively the only feature that landed this sponsorship period is shallow cloning and a proper implementation of negotiation algorithms which finally allows `gix` to work just like `git` in all tested situations. Particularly, it will now work from behind a proxy.

With this months result, bringing submodule checkouts to `cargo` is close though, and research on what it would take will happen next. This happens outside of an active grant though, hence the new title: "Gix in Cargo".


Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2023.csv).
