As hoped, after last months dip my total time worked went back to 156h, up from 128, with 57h spent on open source maintenance (up from 39), and 48 of which went directly into Gitoxide (up from 32).  
This is a surprisingly high number considering that the one feature that is still pending still wasn't worked on. Probably this means there was a lot of maintenance and smaller topics, with some of them hopefully big enough to highlight here.

## New `it` tool for better fixtures

With `jj` now ramping up its use of `gitoxide`, inevitably they run into shortcomings. Only one of them was reported and is related [to rename-tracking](https://github.com/Byron/gitoxide/issues/1524). It's known that it uses less heuristics than Git, but thus far I thought it produces decent-enough results nonetheless.

However, when using it on the real-world `jj` repository, the difference to what Git produces is large enough to warrant an improvement.

The problem was that in order to fix it, one should reproduce it in a test, and the only known way of doing that is the actual `jj` repository. How would it be possible to create a fixture based on it?

Enter the `internal-tools`, short `it`, which aren't published and can use `clap` and `anyhow` to provide a nice CLI experience, while making tool-development easy. It comes with a `git-to-sh` subcommand which can extract a portion of the linearized history of a Git repository, filtered by pathspecs and with all blobs 'degenerated' to prevent having to deal with licensing. Degenerated blobs will still diff exactly the same as before, which helped to create a fixture to reproduce the exact case `jj` wanted to fix. The output of the tool is some files and a shell-script which regenerates a Git repository with blobs, trees and commits.

With this, I hope that \`jj\` developers will be able to contribute improvements to the rename-tracker, and make it more similar to what Git currently offers.

It's notable that such real-world fixtures can also be used to have more meaningful benchmarks, something that right now is also notably absent in the rename-tracker.

## Various API improvements driven by GitButler

When coming from `git2`, recently one has been missing various little methods that were incredibly practical to have despite most of them being available 'differently' already. In order to make conversions from `git2` to `gix` simpler, some convenience was added, namely:

- `Repository::find_commit`, `find_tag`, `find_tree` and `find_blob`
    - Previously one would use `find_object(id).try_into_<kind>()` in order to do the conversion in a separate step.
- `Reference::peel_to_id_in_place()` now has `peel_to_kind(kind)` and `peel_to_commit()`, `peel_to_tag()`, `peel_to_tree()` and `peel_to_blob()`.
    - Emulating reference-based peeling wasn't possible previously as `peel_to_id_in_place()` was a very specific and is what Git thinks of peeling.
    - Having the new shortcuts definitely helps going from ref to usable object quickly, and without using a refspec.

More improvements are planned, and I see how this will greatly facilitate switching from `git2` to `gix`.

## GitButler - from so-so to 'wow'!

It took a while, but I am now very busy translating existing `git2` code to `gix`. It's done in such a way that not only specific tests are added to assure  
correctness at least around the boundary, but also there is the addition of benchmarks to assure the new implementation is at least as fast.

First off, to compare apples to apples `git2` is now [configured](https://github.com/gitbutlerapp/gitbutler/blob/84b2db96857145a8bbf8650ab015cf9cd41e2592/crates/gitbutler-project/src/lib.rs#L13) to be more similar to `gix`, which also doesn't constantly verify objects. This alone is the reason for a measurable speedup  
in GitButler, which still runs mostly on `git2` after all.

With that, it was possible to improve the [generation of branch details](https://github.com/gitbutlerapp/gitbutler/pull/4670) by a factor of *2.6x*, as a follow-up  
to [listing branches](https://github.com/gitbutlerapp/gitbutler/pull/4632) which got faster from 1.5x, up to over 20x to 35x.

And even though `gix` typically ends up faster, `git2` offers great value nonetheless as it's fast-enough by default while sporting a pretty simple and usable API.  
`gix` isn't like that at all for a lot of functionality and needs plenty of API improvements to get closer or just reach similar usability.  
Simple APIs also comes with the drawback that sometimes, that it is hiding so much under the hood that ultimately it becomes too slow in bigger repositories, which is something that a Git client has to be mindful of. So API complexity is also a strength as it provides great levels of control that often are needed to not fold in the face of real-world scenarios.

## Security

### `gitoxide` (CLI) and terminal-escape code

Thanks to Eliah there [is a new advisory](https://github.com/Byron/gitoxide/security/advisories/GHSA-88g2-r9rw-g55h) that points out what could happen if an attacker decided to abuse terminal escape codes to guide the `gix` (CLI) user into executing malicious commands.  
Git will escape these consistently across all of its commands, but `gix` does none of that right now.

Mitigations are planned for this low-risk advisory.

### Ongoing investigation: executing `git` and how it can be abused

`gix-path` has one very 'interesting' method that tries to find the location of the Git installation on the system by deriving it from special installation-wide  
git configuration files. In order to do that, it needs to invoke `git config`, and there is always a risk in that. Not only can attackers possibly control the binary, but they might also try to affect what the run returns, maybe to inject configuration into `gitoxide`.

A recent change now places the invocation into the `$TMP` directory of the system, which was originally meant to fix performance issues when it would try to detect a Git repository traversing upwards from the current working directory. As it turned out, starting in `/tmp` on most systems actually increases security as Git will reject repositories (even in `/tmp/.git`) that aren't from the current user. Also will it hit the ceiling very fast. Thus, said change actually increases security.

## Community

### 'Blame' is getting there

@cruessler, a long-time contributor to `gitui` has started contributing a [first implementation of the `git blame` algorithm](https://github.com/Byron/gitoxide/pull/1453), which is planned to one day revolutionize the way blames are done in `gitui`.

The idea is to start simple, but refine the control the caller has over the algorithm to get to the point of a fully-streaming implementation that provides results continutiously. This way, the user-interface can be populated interactively and most importantly, the user can change what's blamed at any time, or skip over commits that they don't want to see, all in real-time and without loosing progress.

There is no other way but for this to become the blame I always wanted!

### Git Meetup in Berlin

[On the 14th of August](https://github.com/Byron/gitoxide/pull/1453) a lot of like-minded people met to revive the Git Meetup Berlin group, which was generously hosted by GitButler.

There we learned about GitButler, RefTables, and things nobody ever knew about `gitoxide` (and probably will never hear again :D).

### Gitoxide and the Sovereign Tech Fund

No progress was made and it's a bit embarassing. In my mind I am blocked on not really wanted to do all the project-planning that they maybe needed. Another part just wants to get something done though, so I am still eying to try focus on this while I can't do anything else, like when in transit.

### Gix in Cargo

There is nothing to report here, except for [one issue](https://github.com/rust-lang/cargo/issues/14411) related to failing to correctly deal with `.gitignore`\-based excludes. There one would start out with a blanked ignore rule like `/*` and follow with `!<what you want to include>` to undo the exclude selectively.

Fortunately, the issue was already fixed previously in `gitoxide` as discovered by Eliah, and an upgrade to `gix` v0.64 would be all that was needed.

Cheers  
Sebastian

PS: The latest timesheets can be found [here (2024)](https://github.com/Byron/byron/blob/main/timesheets/2024.csv).