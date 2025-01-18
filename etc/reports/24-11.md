The previous month was so rich in news that this month basically had to pale when compared to it. But here we go!

## Tree Merge 

Tree merging is now finished and already used in production. This means that `merge-ORT` level tree-merging is now available to everyone both in terms of quality and even performance. In some cases, particularly when renames are happening on one side and modifications happen in another, these will naturally be carried over which is something that `merge-ORT` won't do as consistently.

It is available in two flavors, `gix merge tree` and `gix merge commit`. The `gix merge tree` CLI subcommand takes all three trees-ish and bascially invokes the tree-merge directly.

`gix merge commit` is sufficiently different to deserve its own paragraph though.

It's worth highlighting though that `gix-merge` really is a beast of an algorithm, with extremely high complexity to the point where most parts are motivated by a test. This also means it's quite hard to reason about what it does and how it would work, leaving me wondering if there is 'more natural' ways to accomplish the same output.

### `gix merge commit`

The more convenient way to trigger the new merge algorithm is through `gix merge commit`, which takes two committish and computes the merge-base itself. If there are multiple merge-bases, just like `merge-ORT`, it will merge the merge-bases recursively, producing a virtual merge-base which seems to be good for best-possible merge results.

As a command, it's still quite far away from `git merge` as it won't update the working tree, refs or the index, which makes it a nice side-effect free way of testing merges, much like `git merge-tree --write-tree <side1> <side2>`.

## Tree Merge with Index support (WIP)

What's brewing in [this PR](https://github.com/GitoxideLabs/gitoxide/pull/1661) is the ability to produce entries that should go into the index that itself was produced from the merged tree. If such a conflicted index is written, it will automatically instruct tooling that can assist with resolving such an index, fixing the merge-conflicts along the way.
It's also a required feature if one would like to perform a complete merge, which involves the merge itself, checking out the new tree while updating the index to it, and finally to add the conflicting entries to the index representing the checked-out tree.

As a side-note, I often find it difficult to reason about how conflicts should be marked in very complex cases that also involve renames. But I am sure I will get better intuition for it as the implementation comes along.

## `hasconfig`

And finally, I think it's fair to say that `gitoxide` is able to read the complete wealth of Git configuration thanks to the new [`hasconfig`-include-if support](https://github.com/GitoxideLabs/gitoxide/pull/1656). Conditional includes were supported before, but this is the latest addition which allows to include configuration if a remote-url matches. For now, this only works with remote URLs, just like in Git, but extending this should not be very troublesome if Git would add this in the future.

## merge-base: octopus

As a small addition with just a few lines of code, one can now calculate the merge-base of multiple commits using the 'octopus'-technique. And even though the normal merge-base algorithm can also find all the available merge-bases, the `octopus` one is actually stable. That is, permutations of input commits affect the outcome to the algorithm to point where it doesn't seem to find the correct (or at least desirable) output anymore.

With the 'octopus'-mergebase one will always get the same result, which indeed is the best-possible single merge-base for any input.

## Community

### Release notifications!

Thanks to Eliah Kagan, we now have a [discussion to subscribe to](https://github.com/GitoxideLabs/gitoxide/discussions/1693) which will trigger each time the binaries (along with the top-level `gitoxide` crate) are released.

That way, distributors have a way to get notified *without* having to subscribe to GitHub release notifications which are way too plentiful, given that one is generated for each of the 50 crates upon release.

### Greatly improved 32bit support

Previously the 32bit testing was very spotty, and not all tests were run. Fortunately, thanks to Eliah Kagan this [has now changed](https://github.com/GitoxideLabs/gitoxide/pull/1687), and testing is now done in full on 32 bit systems.

Something I found interesting was that certain structures didn't shrink by 50% in size, but were merly 80% of their 64 bit counterparts.

### Gix in Cargo

Still, there is no news here, and I am still looking forward to finally getting to finalize `gix status` which is the next big step for an improved `Cargo` integration.

#### Cargo-issue with non-ignored device-files in the package directory

This issue along with its [related PR](https://github.com/GitoxideLabs/gitoxide/pull/1629) is still present and I merely mention this here to indicate it's not forgotten. I will definitely try to tackle it in this year still.

Cheers  
Sebastian

PS: The latest timesheets can be found [here (2024)](https://github.com/Byron/byron/blob/main/timesheets/2024.csv).