Thinking about April and everything that was accomplished there, it feels like I burnt too hot and had to pay up for it by being sick with fever for the first two weeks of May. During that time, I couldn't do more than just most basic maintenance as with fever, my brain doesn't work 🤦‍♂️. And while writing this, I am still coughing while in slow recovery, it's insane and nothing I thought could possibly happen to me.

But with all that said, let's see what could get done in the time that remained.

## Safety First

Thanks to Eliah I was informed about two vulnerabilities, which are both already fixed in Git itself:

* [Traversal outside working tree enables arbitrary code execution](https://github.com/Byron/gitoxide/security/advisories/GHSA-7w47-3wg8-547c)
* [Refs and paths with reserved Windows device names access the devices](https://github.com/Byron/gitoxide/security/advisories/GHSA-49jc-r788-3fc9)

It took a lot of effort and more than 2000 lines of code (and tests) to fix both reliably, and admittedly I feel a little bit ashamed that I just skipped over all safety checks during the initial implementation… and then forgot about them.

Now, however, `gitoxide` respects the `core.protectNTFS` and `core.protectHFS` configuration keys, and even adds a new one, `gitoxide.core.protectWindows`, which allows Unix users to assure that their repositories are Windows-safe, at least in terms of file- and ref-names.

But that's not all - Eliah was also responsible for [properly fixing a previous vulnerability](https://github.com/Byron/gitoxide/pull/1342) which I left only partially fixed.

I cannot thank Eliah enough for doing this work - it's clearly something I need a lot of help with :)..

## Community

### A fix for sporadic 'object not found'

[NoseyParker](https://github.com/praetorian-inc/noseyparker) is using `gitoxide` to decode all objects of repositories in order to check them (and their history) for vulnerabilities.

Recently, it ran into [one of the worst issues that I can imagine](https://github.com/praetorian-inc/noseyparker/issues/179): sporadic, none-deterministic failures to retrieve an object that is known to exist.

The approach I took to get to the bottom of this was to adjust a `gix` command to approximate what happens in NoseyParker: a bunch of threads concurrently read all objects in a repository. The 'reads objects' part could be sped up by only reading the object header, so I would end up with an invocation that ran in just 200ms in the provided demo repository, after 50 or so tries, there would be one failure. Once again, I keep using `hyperfine` to do the 'repeated invocation' for me along with decent reporting, and with this workflow non-determinism isn't so bad anymore as it's just a matter of time until the issue is hit.

It quickly became evident that repositories with many packs don't have the issue, but those with a single pack easily did.

The cause of the issue turned out to be a stereotypical trampling herd, where a lot of threads try to read the same pack concurrently. It's implemented racily intentionally, as pack and index reading can be concurrent that way, i.e. multiple indices at once and in parallel. But as always, it's very difficult to do this correctly and the code did have some 'space' for a thread to not have work, and to not detect that work was just done, leading it astray to think there is no new index, and thus the object it looked for indeed isn't available.

The fix was a one-line change which provably worked, even though it 'just' changed the timing by yielding the respective thread another time. Part of me thinks that there should be a better way, but for now I can't imagine it (it must be lock-free after all).

### Gix in Cargo

Cargo once again [was suffering from a bug caused by `gix`](https://github.com/rust-lang/cargo/issues/13923) - this time, `cargo doc` would fail (but `cargo build` and friends would work as normal. Once again, the `list-files` code that is now powered by `gix` in stable even, ran into a condition where it would fail as it couldn't read the Git configuration if the `core.excludesFile` key was empty. Git would just ignore that, but `gix` did not.

The solution was simply to ignore empty paths (if the configuration is loaded in 'lenient' mode), which is now done by default for all paths to hopefully prevent this kind of issue in future.

Cheers
Sebastian

PS: The latest timesheets can be found [here (2024)](https://github.com/Byron/byron/blob/main/timesheets/2024.csv).
