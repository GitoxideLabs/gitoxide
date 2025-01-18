### `git-repository` - handle git references and create commits

`git-repository` learned a lot of new tricks, allowing you to learn everything about references and their reference logs and traverse commit ancestries (like a simple rev-list). Now, for the first time, one can also get mutable access to the underlying shared repository to refresh its pack cache in case these changed in the meantime.

Reference namespacing is supported as well, and reference iteration with prefixes now also works correctly when using partial prefixes on loose references. This means, that before the fix a prefix like `refs/tags/foo-` would not yield any loose references named `refs/tags/foo-1`, but would work for their packed counterparts.

### Making `git-repository` 'safe' to use

With the recent push of `git-repository` to become more useful for application developers, it also became clear that not breaking their code is paramount.

The primary way to achieve this while in pre-production mode (i.e. major version zero) is to not only follow semantic versioning, but also impose very conservative versioning of _dependent_ workspace crates.

#### `cargo smart-release` - conservative versioning

`smart-release` was improved to, by default, bump the minor version of dependent pre-release crates in the workspace if one of their dependencies, in the workspace, signals a breaking change. That way, users of `git-repository` can run `cargo update` safely without the fear of pulling in breaking changes by surprise. Note that this feature is also enabled
for production crates, who would only increase their minor version if there are breaking changes in their dependencies, allowing consumers to use the `~<version>` version requirement in their manifests to allow patch-level upgrades only.

#### Stability Tiers

Speaking of production crates, these now fall into one of two tiers as per [the stability guide](https://github.com/Byron/gitoxide/blob/a65c14f470bf8c0e3b15ff8695ef97b5c6ab738f/STABILITY.md). The `git-lock` crate, for instance, is already released into stability tier one which guarantees its major release to last at least 6 months - a low number to start with. `git-tempfile` has landed in stability tier 2, which grants it one month of stability, at least.

`git-repository` puts the system to the test as it uses crates from all tiers and combines them under one API. Sometimes, this API completely hides the underlying implementation, which allows any stability tier to be used, even pre-production crates. However, if the underlying crates leak and types escape the `git-repository` API, these must also be in stability tier 1. As there are plenty of useful pre-preproduction and tier 2 crates available via `git-repository`, these can optionally be exposed using the "unstable" cargo feature.

Using unstable or less-stable features via `git-repository` is indeed recommended, and it is safe thanks to conservative version bumping described earlier.


### The great refactor

The more `gitoxide` matures thanks to `git-repository` and increased adoption, the more some blunders of the past become visible and demand fixing. A major and noteworthy improvement was made to `git-object` which previously used the `immutable` and `mutable` categories to split its implementations of git object types. Doing so was helpful at the time, but also imposed arbitrary limits to what you can do with these objects - one category had to be treated as 'immutable' even though you might have a mutable references to this object with shared data into a backing buffer. This also led to the 'immutable' object classes to be deserializable, with references to their backing buffer, whereas the 'mutable' counterparts used owned types instead and could be serialized. If you wanted to deserialize an object, change a field, and serialize it, you had to convert the obtained 'immutable' object into a 'mutable' first in order to get access to the implementation for serialization for no other reason than wrong abstractions or mindset.

When working on `git-repository` and its `Easy*` types, it finally dawned on me that there already is a name for 'immutable' objects: `Ref`. Appending `Ref` to `Commit` perfectly indicates their differences without hanging the curtain of 'immutability' over one of them. This came with another change of implementing serialization for `*Ref` objects as well.

Similar changes where made throughout the `gitoxide` crate ecosystem for consistency.

### Pack File Generation

After last month improvements to object counting it was time to let some advancements to the ecosystem trickle down to where they can be most useful and pack-create uses `Easy` where possible, while practicing the conversion back to plumbing level as its use of git-pack demands it. It's nice to see that one day (when there is some demand) the pack-create functionality can be ported to git-repository to naturally interact with `Easy` state.

`pack-create` can now also create thin packs and display the amount of thin/ref-delta-objects in the pack. The usefulness of such a feature probably doesn't go beyond testing as thin-packs are really only valid in transit.
And as thin-packs are created most easily using tree-diffs, which naturally provides a lot of objects whose bases are then not part of the pack, tree-diffing performance in the context of pack creation was now sped up by factor ~2.5 using an LRU memory capped object cache whose size if configurable.

With pack-creation at its current state, there are three major avenues where progress can be made, in order of simplicity and maybe even usefulness: 

* create an index alongside the pack
    * This is most desirable as it would enable tasks like packing all local packs into a big one, which will need the index. Probably it will be best to write the index from the available data after the pack was created to keep memory usage in check.
* start working on creating own delta objects, probably the most arduous task related to pack file generation
* improve count-objects performance
    * Git is ridiculously fast on some repositories and can easily outpace gitoxide by 2.5x on a single thread. Multiple threads beat git by a small margin, but at high cost.
    * On the bright side, writing packs when counting is complete is always faster at ~500MB/s (single thread) to ~900MB/s (multiple threads) compared to ~400MB/s. This is definitely due to the simplicity of direct pack copies and shows that optimizations like these have a benefit.
    * I could break it down to showing that it's definitely tree-traversal which is too slow in comparison, and it's a hunch that the HashSet used to determine which objects to skip is still too costly. Exchanging it for a BTreeSet for example causes a 33% slowdown, so the right data structure seems to have huge impact here. The pack-cache is also only 42% efficient at 200MB size, and 44% efficient with 2GB of size, so apparently with the current LRU implementation there isn't too much to gain in that realm either.
    * Using a custom hash set hasher similar to the one git uses boosts performance a little, but it's still not even close to be fast enough, putting the single-threaded version to 12s and the multi-threaded one to 5.75s, whereas git takes 8.6s.

Lastly, these ominous missing objects are a little less ominous as these are simply git-submodules, a hash to an external repository which wont' exist in the current repository 🤦‍♂️.

Now that this is fixed I do have hopes that the counting phase may still be helped as well with similarly minor tweaks, and even if not bitmaps would probably help tremendously in this endeavour. 

### `cargo smart-release` grows up

It had humble beginning with simply being a tool that is able to publish workspace crates and their dependencies without causing a whole lot of errors down the line. In that it already succeeded and cut down the time for releases from 90minutes to however long it takes to verify all crates to be published, or 5 minutes tops. This also helps with releasing much more regularly as it's now entirely pain free.

However, previously each release would create one commit per release, which was somewhat unnecessary. With the dawn of the 'conservative bumping of dependent crates' feature (e.g. announcing a breaking change in `git-ref` will cause a similar version bump in all crates that use it), it seemed all manifest changes should rather be self-contained in a single commit, adding the respective release tag after each successful publish instead. And this is what happens now, and even publishing ten crates will only cause a single commit to appear.

#### Changelog generation (in progress)

With the recent formalization of stability tiers and `git-*` crates finding their first downstream user, changelogs were added to these crates. Their update, however, is still manual, and thus very easy to forget or worse, to get wrong. But wouldn't it be great if `cargo smart-release` could also create and update changelogs for you based on the commit history?

"Commit history as basis for changelogs??!" I hear you say, "isn't that known to create low-quality, noisy changelogs at best?!" added in agony. Having had my own failed experience with [conventional commits](https://www.conventionalcommits.org) myself I am quite aware these can't ever become a changelog directly, nor do I like to follow this convention all the time.

The plan here is to use conventional commit messages occasionally, with only these being used in changelogs. These commits will be grouped and _merged_ into the existing changelogs to facilitate writing and amending it by hand. That way, before a release, one would run `cargo changelog` to generate or update - none-destructively - the changelogs of all crates that would be released when running `cargo smart-release`. Then they would be adjusted and prettified by hand, before running `cargo smart-release` which figures out the required version bumps all by itself thanks to changelog access, and adds the actual version number to it right before release.

It will take some more experimentation to maybe even make the `cargo changelog` subcommand optional and integrate the entire workflow into `cargo smart-release`, maybe by having it stop if any new items showed up in the changelog for you to review and adjust them before continuing - that is it will only proceed with publishes if it all it had to do with a changelog was to adjust the most recent "Unreleased" headline to the name of the actual release to be made.

### On the horizon

This months goal is nearly the same as the last month one, which is to finish the `gixp-fetch` work block to prove `gitoxide` can now also fetch packs and to get started with working on something like `gixp pack-send` as `git upload-pack` alternative. This will be fun as it involves the server side of a `fetch` operation which we already have a client for, along with pack generation which we also happen to have ready. With that finally on the way, it can't be too long until `gitoxide` will serve its first pack over the wire :).

Besides that, I intend to produce more material around the `gitoxide` crates to hopefully find more adoption and contributors. Most of this work will begin once the release-process automation along with changelogs is done to support downstream users better.

Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2021.csv).
