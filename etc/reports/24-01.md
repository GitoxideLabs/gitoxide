This month, was surprising as it felt I was always busy, yet there isn’t much to show for it, not at all. So what happened? This time I am desperate enough to actually look at my timesheet to see what happened between the 22nd of December and 22nd of January.

The good news is that I did manage to put in 130h of work, of which 95 were for open-source. From these, it’s 55 for `gitoxide`, 21 for general maintenance, 9 for GitPython, and 5 for `duo-cli`, 1 for Google-APIs and another hour for `starship`, `helix-editor` and `cargo` specific maintenance, probably `gix` upgrades. Thus, 43% of my open-source time went into maintenance, quite a high value that’s not entirely unsurprising given how active GitPython and `dua-cli` both felt.

Now that this is out of the way and I can feel better about myself 😁, let’s dig in.

## Precious Files

When using `.gitignore` patterns to mark files as ignored, what really happens is that they are classified as expendable. Expandable files won’t be tracked by default, but they also will be removed the next time Git gets a chance. Think about all the ignored-and-local editor configuration we must have lost over the years thanks to a thoughtless `git clean -fxd`.

Precious files are the solution to the problem as they are another class of ignored files, which Git will neither track by default, *nor* will it remove them. Hence it treats them like untracked files during worktree mutations, and as ignored anytime else.

The [proposition](https://lore.kernel.org/git/79901E6C-9839-4AB2-9360-9EBCA1AAE549@icloud.com/T/#u) is from the beautiful mind of Josh Triplett, who, as it turns out, wasn’t the first one to have the idea. More than 10 years ago the conversation was started already, but lost in the sands of time as well.

Josh and I, alongside Elijah Newren, a long-time Git contributor, managed to reinvigorate the feature, sketch out [a technical document](https://lore.kernel.org/git/pull.1627.git.1703643931314.gitgitgadget@gmail.com/#Z31Documentation:technical:precious-files.txt) to bring it to the point where it can be implemented in Git itself 🎉.

`gitoxide` is already able to parse the new `$precious` syntax, of course, even though it still has no opportunity to make use of this capability as neither worktree resets nor index updates are implemented yet.

The next step for me is to start the precious files implementation, in C, for Git, and I am excited and afraid at the same time 😅.

## Precompose Unicode

The idea is very simple: Unicode is powerful, so powerful that there isn’t one right way to encode a string, but many different ones. One common difference is how unicode characters are composed. The umlaut `”ä”` for example is precomposed, it’s a single code-point telling us it’s the “Umlaut for A”. Another form though is the decomposed one, which looks like `”a\u{308}”, two code points, an `a` and the double-dot on top of it.

Some filesystems, and as far as I know only the one on Apple platforms, treat decomposed and precomposed unicode the same. So if a file is called `ä`, it can be read by describing it as `ä` and `a\u{308}` alike. That’s great, actually, but the problem is that the Finder tends to decompose unicode, so creating a file called `ä` will actually create a file known as `a\u{308|`. When Git sees this file, for example when adding it to the repository, it will see exactly how it exists on the file-system which *can* be decomposed unicode. On other platforms that’s uncommon though, which can be the cause of confusion at the very least.

Thus, Git will *precompose* all *decomposed* files as it receives them as input from command-line arguments, the current working directory, and when traversing files as well.

`gitoxide` now respects `core.precomposeUnicode` and assures that whenever files are involved, quite common when handling references, it will also fold precomposed and decomposed strings so one will encounter the expected behaviour. Before setting this up there actually were subtle bugs that could be triggered in specifically written tests, so I am happy that `gix-ref` in particular has seen the amount of work that it did.

## Community

### Improvements to Unsafe - how to do it right

`gitoxide` has a couple of usages of `unsafe`, and they all stem from parallelisation. Most of these are just a couple of lines which are quite straightforward to vet. One of these though has it all, as it’s not less than the algorithm that makes clones and fetches incredibly fast and efficient. The idea is that one builds a tree of inter-dependent pack entries which are then resolved to obtain their hashes from which one will build the index that allows fast access to the received pack later.

The computation, decompressing entries, applying deltas, and hashing the result, is about as involved as it sounds and thus distributed to all cores of the machine in a lock-free fashion while doing all work only once.

What sounds simple becomes quite complicated when `unsafe` is involved and one effectively has to try and prove to fellow humans that this is indeed sound.

Fortunately, [Manish swooped in](https://github.com/Byron/gitoxide/pull/1237) and saved the day. The idea is to track invariants through all `unsafe` code blocks, while minimising them to leverage the normal capabilities of the Rust compiler to the greatest extent.

Overall I do admit that I still wouldn’t claim that I am able to do `unsafe` correctly, and simply hope I won’t have to use it again. But when I do, I will be sure to revisit Manish’s PR for a little chance to do better next time.

### Gix in Cargo

The following I wrote last month:

> Thus, I’d think that in January 2024, there will be a huge step towards finally completing `gix status`.

And of course, it came very differently as I got sucked into getting `core.precomposeUnicode` support done instead, a topic which was discovered naturally when starting to implement git-style dir-walking after having written all the tests I could think of. On the bright side, the next `gitoxide` session will pick up right where I left off and I’d expect to finally get that done, it’s nothing short of exciting to finally be able to implement it.

Cheers,
Sebastian

PS: The latest timesheets can be found [here (2024)](https://github.com/Byron/byron/blob/main/timesheets/2024.csv) and [here (2023)](https://github.com/Byron/byron/blob/main/timesheets/2023.csv).
