### `cargo smart-release` auto-bumping and changelogs

 `cargo smart-release` was created to make regular releases possible, but grew into a tool that made releases safe, too. To accomplish that, it would have to help assuring that `semver` is used correctly, and breaking changes are communicated using minor version bumps in pre-release crates.

As its prime source of knowledge that is naturally easy to tap in, the git commit history was chosen, along with parsing commit messages for useful clues to derive the appropriate version bump. The aptly named 'conventional' commit message style was chosen to provide such clues, which allows to write `feat: to indicate a new feature` and `change!: a breaking change` to indicate breaking changes.

Along with simple change-detection `smart-release` is now able to automatically determine the required version bump for each crate involved in the release, and new releases of `gitoxide` are trivially done with a `cargo smart-release --execute` invocation.

But it doesn't stop there, as a major gripe remained: changelogs. These would still have to be handled manually, leading to most crates not actually having one. And those who had it where probably missing information due to lack of discipline or mere forgetfulness. Knowing that I am at the very least able to create useful commit messages, why not make use of them?

So a major undertaking was started to implement non-destructive changelog generation as part of the release process, using the existing git-commit history information to also generate beautiful changelogs along with statistical information for each release thus far. The new `cargo changelog` command is used to explicitly write changelogs to create the scaffolding, allowing the user to go in to polish them to completion. That way, nothing is ever missing from changelogs as long as 'conventional' git messages are used. A positive side-effect of this is that I write better commit messages, knowing that these are the foundation for a changelog.

The latter will not only be written into the `CHANGELOG.md` file, but also into the tag-object to mark the release, and _also_ into a github release that it creates using the `gh` command-line tool. As it's a generator, it's even able to re-generate the release notes as to be suitable for their target, getting the best possible look both for plain-text tag annotations as well as for the auto-linked markdown in github releases, or the manually linked markdown in the changelog file.

[Here is an example for a generated and polished changelog](https://github.com/Byron/gitoxide/releases/tag/cargo-smart-release-v0.4.0).

With the release of the version linked above I did think I was done to finally return to actual feature work. But I couldn't be more wrong as it turned out the dependency resolver grew out of control to the point where the code was unmaintainable enough to not warrant being fixed despite having issues that once again caused downstream breakage. 
Instead it had to be 'written into the clear' with everything learned so far, which didn't only fix the issue, but also led to greatly improved and more informative display of its release plan, as for the first time it actually new that in advance. This allowed more wide-spread refactorings to bring the entire codebase to the level it needs to have to be a tool to be relied upon.

The feature that didn't work properly certainly deserves more than a passing mention though: it's safety bumps. This means that with packages `a <-b <-c`, where `b` depends on `a` and `c` depends on `b`, a breaking change in pre-release versions of `a` will be propagated to minor version bumps in the entire dependency graph that potentially transitively depends on it. That way, downstream will always have to explicitly upgrade their minor version instead of being auto-updated to a dependency that ultimately causes the compilation to fail. And even though this will cause some 'version-churn', I think it's a price worth paying and we aren't going to run out of release numbers anytime soon.

### Gaining velocity by attracting contributors

The 'outreach program' is certainly only in its humble beginnings, but here is what happened so far:

* There is a new Youtube show with Sidney as guest where we ['Learn Rust with Gitoxide'](https://youtube.com/playlist?list=PLMHbQxe1e9Mk5kOHrm9v20-umkE2ck_gE) together
    * This may help to attract new contributors to take their first steps in Rust with Gitoxide.
    * Sidney is primed to not only learning the nitty and the gritty of Rust that way, but will hopefully soon feel comfortable driving oxidization efforts to spread gitoxide into the crate ecosystem
* I am on the lookout for experienced developers who could move in as core developer

This goes along with regular Reddit updates highlighting recent releases or features.

### Pack generation pushed back for sponsored work on Cargo

For the next two weeks or so I will be working full time on implementing two RFCs, for one of which an [early PR](https://github.com/rust-lang/cargo/pull/9992) is available in case you are interested. Primarily due to me pushing for wrapping up `cargo smart-release` I didn't get to work on further improvements to pack generation, and due to the need to gain ground for the sponsored work I have to push that topic back a little.

This also means that next month, there should be a bigger story to tell about packs.

### What, wait, that's it?

Looking back at the past month, shockingly, the answer it: yes! And as nice as it is to consider downstream safety, releases and change-communication via changelogs and release notes a solved problem, I was working relentlessly while being driven mad knowing that this is not actually the feature I _want_ to do, but merely an _epic_ side tale. A tale providing capabilities that make consumption of `gitoxide` crates safe, so I do believe it's worth having it moving forward.

There is still some unfinished business around the previous work block related to `git-fetch` (a feat `gitoxide` still cannot perform), which will certainly happen before moving back to the server to implement the other end of a fetch: `upload-pack`.

Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2021.csv).