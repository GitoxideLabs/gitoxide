###  The new object database

After the design-sketch for the new object database turned all lights to go, I managed to move forward and develop the sketch into a proof of concept. Starting out with minimal tests, it quickly became functional enough to let it run on its own in the `object-access` benchmark program. There it performed so admirably to allow it to become the only object database implementation from this point forward. Here are its features:

- entirely lazy, creating an instance does no disk IO at all if [`Slots::Given`][store::init::Slots::Given] is used.
- multi-threaded lazy-loading of indices and packs
- per-thread pack and object caching avoiding cache trashing.
- most-recently-used packs are always first for speedups if objects are stored in the same pack, typical for packs organized by
commit graph and object age.
- lock-free reading for perfect scaling across all cores, and changes to it don't affect readers as long as these don't want to
enter the same branch.
- sync with the state on disk if objects aren't found to catch up with changes if an object seems to be missing.
- turn off the behaviour above for all handles if objects are expected to be missing due to spare checkouts.

As of now the previous implementations are deprecated and will eventually be removed from the `git-odb` crate, and `git-repository` already uses the new one

### Multi-pack index support

The new ODB already expects multi-pack indices and is merely waiting for an implementation of the multi-pack index file format. This one is now available and provides all the usual access methods. Integration it into the new ODB will start as soon as possible, and I don't expect any problems with that.

Once this work is completed, `gitoxide` finally has a production-ready object database with all the features one would expect, which will also serve as good foundation for adding support for additional file formats, like reachability bitmaps and reverse indices to speed up pack building.

### Community Outreach part 2

Now that "Learning Rust with Gitoxide` is completed, we moved onto season two's programming with `Getting into Gitoxide`, a format to show how to use `git-repository` and how to extend it.

Here is the playlist link: https://youtube.com/playlist?list=PLMHbQxe1e9MkEmuj9csczEK1O06l0Npy5

Sidney and I use it not only to improve the existing API surface, but also to show off latest improvements or undertakings as part of the usual `gitoxide` development. It also serves as preparation to help Sidney to get started in `git-repository` himself.

### `git-repository` is getting better

After the last rounds of refactoring it feels like it's coming together. Previously there were quite a few shortcomings and inconveniences, but all of them have been removed by now. All this was possible due to learnings in `git-ref` and `git-odb` which allowed a lot of complexity to move down into the plumbing crates, where it belongs.

`git-ref` and `git-odb` now expect to provide a usable experience all by themselves, which in turn lead to great simplifications on the side of `git-repository`.


### SHA256 support now has a tracking issue

In [#281](https://github.com/Byron/gitoxide/issues/281) one can now track the steps needed to get SHA256 support. In the past days I took the time to give it a push (and made the necessary breaking changes) to be parameterize most of the code-base.

At the end of this effort there could be a tool to convert repositories from one hash kind to another, with the intermediate step being the ability to read either one or the other, and write to it, too.

### So much is still to be done though…

While the major undertakings are shaping up, some work-in-progress seems to be less lucky and doesn't see much movement.

- git-fetch like functionality
    - despite being so close, no progress was made in that direction and it's still something I'd love to finally get done. The building blocks are all there.
- simple object deltification
    - even though the current pack creation capabilities aren't anything to sneeze at, it would be great to experiment with the infrastructure needed to support creating own deltas

A lot of work, and I am looking forward to all of it :).

Merry Christmas and a happy new year,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2021.csv).
