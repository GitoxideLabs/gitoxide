This month was the strangest of them all as I didn’t make substantial progress despite trying so hard. Nonetheless I believe the `gitoxide` got significantly better, and here is why.

## `ctime` correctness

While trying to get the little that currently is `gix status` to be correct I ran into an interesting issue I could call ‘ctime-war’. To understand it, we should probably take a step back and establish that `git status`, and by default, `gix status` actually are a modifying operation, which is quite unintuitive. But when performing a status check, there is a chance that in order to be sure that a file changed, or not, one will have to hash it. And that is an expensive operation which ideally shouldn’t be redone now that it’s established that the file was changed, or not. And this is when the `.git/index` file can be updated after running `gix status` or `git status`.

Now add to the story that I use `starship`, and that `starship` runs `git status` under the hood and we have the setup for `crime-war` - after each run of `gix status` that has potential to change the `.git/index` file, the terminal prompt will update and launch `git status` right after. Both better agree on what’s in the index file.

And as this is the real life, of course they didn't agree, which caused both of them to correct each other’s work.

The culprit turned out to be a difference in the `crime` used when querying the ‘creation’ date of a file. There are multiple candidates and `git` chooses the `crime` field for the file, but Rust picks the inode creation time. Both usually differ.

To fix it, I once again used the fantastic `rustix` crate to make a manual `lstat` call, from which one could extract the correct field by hand.

## `gix_object::Find` - the missing trait

When reviewing a PR related to partial clones (more about that in a moment), I was about to suggest a better way to query the object database without pulling in all of `gix-odb`, which thus far has been avoided throughout the codebase.

In that very moment it clicked and I new that I shouldn’t keep using `impl FnMut(&oid, &mut Vec<u8>) -> Result<…>`, but instead realize that the `gix` plumbing truly needs an easily accessible trait for this all too common operation.

At the end of quite an intense refactoring session, there is now the `Find` and the `Header` trait, all available from the ubiquitous `gix-object` crate, and they truly make a huge difference in convenience.

## Tree-entries now parse losslessly

The `gix-object` crate implements the parsing of all `git` objects, and it was one of the first crates written for the `gitoxide` project. Sometimes, this shows as I allowed myself certain ‘shortcuts’ that today would be utterly unacceptable.

Years later, having forgotten all about the luring tech-debt, a bug occurs and warts of the past are rediscovered. The issue at hand was that `gitoxide` couldn’t decode certain valid tree objects that `git` or `git2` had no problem with. It turned out that it didn’t consider certain modes of tree-entries valid just because they were ‘unusual’.

This was due to it parsing modes by comparison to strings, like `10755” to then turn them into the variant of an enum - after all, there are only a few valid ones. Thus, while it would do that, there was no way to assure it wouldn’t fail later on a similarly unusual tree entry mode. Also it wasn’t possible to assure trees can round-trip, as technically they could loose information during parsing.

Thus the decision was made to finally differentiate between the `Kind` of a tree-entry, and its `Mode`, with the former being an enum and the latter the original 16bit number that represents the mode in all its detail.

## Community

### `gix-fsck` - the first step towards partial clones

This is part of a bigger undertaking to make it not only possible to fetch packs with a given set of objects, but also to make more algorithms partial-clone aware from the beginning.

In order to do that, there should be a way to detect missing objects, and this contribution is an fsck-implementation which focuses on exactly that.

While looking into this, it also became clear how much `fsck` is actually validating, and that the current implementation is incredibly basic in comparison. But at least it’s a start which hopefully will grow more powerful over time, neatly contained in the `gix-fsck` crate.

### `git credential-helper` support and better shell handling on Windows

Once in a while there are issues which take quite a while to figure out, and this was one of them. During the investigation it also became clear that `gix —trace` is a very powerful remote debugging tool by now which will only grow better over time as more key-traces are added to it.

In this case, we compared the trace of `gix` with the trace of `git` and noticed that the invocation of `git credential-manager` differed in that `gix` used a shell, and `git` did not. Said shell it was that didn’t exist in `PATH` and thus prevented the execution of the credential helper program.

It turned out that `git` didn’t even try to use a shell on windows, and in this case relied on using its own `EXEC_PATH` to find the program in question. The solution was not to fully disable the use of a shell, but instead, and only on Windows, try harder to *not use a shell*. This procedure could also be steered by the caller, which may have additional knowledge about the program being invoked. Credential-helpers, for example, basically never have to use an intermediate shell, no matter which platform.

With this fix, even one Windows one could fetch using a rather complicated setup that I didn’t even know existed, which was a great show of what `gitoxide` was made to do. Even today, there is probably dozens of applications that use `git2` and which hade to re-implemented emulations of `git`s usage of credential helpers, which is likely to fall short somewhere, or do not work on windows at all.


### New Sponsorship by drips.network

`gitoxide` is now sponsored by the [Drips Network](https://www.drips.network/app/projects/github/Byron/gitoxide), and the sponsorship is substantial to say the least. It took me a while to get used to it as well as I felt like I should rather focus on cloning myself to be able to do all the work that needs to be done.

But now I have come to terms, and I will simply do my best like always while trying to help making `gitoxide` a fully fledged `git2` replacement, along with added correctness thanks to respecting `git` configuration by default. That should also help Radworks, the company behind the Drips Network, which has a couple of teams busy with making `git` forges as decentralised as `git` itself.
 Exiting times!

### Gix in Cargo

It took me a while to realize that in order to get a correct `gix reset`, which is ultimately what I want to use in `cargo`, I needed a complete `gix status` (untracked files, modified files, and changes of the index itself). And in order to get a complete `gix status`, one also needs rename and rewrite and copy tracking, a feature that was already implemented specifically for diffs of trees with trees.

In a big refactoring step, this engine was generalised and now lives in `gix-diff`, along with its own set of tests. And while being there, I decided to also make the blob-diff itself more correct by adding support for diff-drivers and text-converters, even though this is still in progress.

On the bright side, once done, it will be easier than ever to generated diffs programmatically just like `git` would, with all the *major* bells and whistles.


Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2023.csv).