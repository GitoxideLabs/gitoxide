This month was very productive, just not directly for `gitoxide`. And it's notably the first month in my recollection where that happened. There must be a special reason for it, and bluntly, it's just me getting on top of things which required me to change priorities. It's nothing I want to see happen again though, it should be the exception, not the rule.
Overall, I still think this is good news and here is why, for two reasons actually.

## 2 + 1 = 3

Besides developing `gitoxide` and maintaining way too many open source projects, I also work with Josh Triplett on [`buildit`](https://buildit.dev/). That's two jobs especially right now where I am working on a bigger `buildit` feature.

And since the good news I was alluding to last month panned out, I am now also helping to 'gitoxidize' the popular [GitButler App](https://gitbutler.com/). In order to get on top of it I am investing a lot of time right now, and pushed it so much that this month `gitoxide` didn't get any of the usual feature development. That's unfortunate  as `status` still isn't done, but it's only a minor setback knowing that `gitoxide` development and features will soon be driven by what will be the best Git experience possible. And in case there are doubts, the enterprise is headed by no other than Scott Chacon, who was among those who brought us GitHub.
 Exciting times!

## Welcome Eliah! Thank you, `git2` and Drips!

Did you know that `gitoxide` is sponsored [by Radicle on Drips](https://www.drips.network/app/drip-lists/34625983682950977210847096367816372822461201185275535522726531049130)? `git2` is also among the sponsored projects, and recently decided to effectively double the funds of `gitoxide` over night by 'dripping' 80% of their donations to it. The goal: help make `gix` the replacement for `git2`! Despite being incredible thankful for that, I also realized that without action, these funds would not contribute to getting there faster simply due to me being unable to double my output.

Now, it seemed, I needed to tackle recruiting, a topic that is just as hard in open-source as it is elsewhere.

Fortunately, I knew just the guy: Meet Eliah, who has been [contributing to GitPython](https://github.com/gitpython-developers/GitPython/pulls?q=is%3Apr+author%3Aeliahkagan+is%3Aclosed) for more than half a year now. But it's not quantity, it's the quality that is the deciding factor here - his attention to detail and diligence is best described as 'beyond-human'. I am not kidding, my work feels shoddy and I pale in comparison. Maybe I could deliver similar quality and attention to detail, but I would certainly take a prohibitive amount of time. He, though, he seems to 'just do it'.

And now, he does what he does for `gitoxide` as well, as I decided to just `drip` the `git2` part of the funds directly to him 🎉.

Thank you for joining, Eliah! `gitoxide` is lucky to have you. Predicting what that could mean, strictly from my experience with GitPython, I'd think that there will be a lot of improvements to correctness, security and compatibility, particularly for Windows. And all that is just the beginning.

## Community

### Hawkeye now uses `gitoxide`

Alright, even though there was no feature development, I couldn't resist to score another win in my conquest against `git2::Repository::is_path_ignored()` by [replacing it with `gitoxide` in Hawkeye](https://github.com/korandoru/hawkeye/pull/126).

Unfortunately I didn't go deep enough to be able to provide any performance numbers, so in a way, this change clearly was out of principle: `is_path_ignored()` is just too slow to be used.

### A true topo-order iterator for commit-graphs

There was a [wonderful PR](https://github.com/Byron/gitoxide/pull/1336) which added a whole new iterator that implements the actual `--topo-order` or `--date-order` that Git typically uses when running `git log`. It's the order that can prevent multiple lanes/branches to overlap, and is probably a major step towards being able to implement something like `gix log` faithfully.

### Better Windows compatibility

Thanks to Eliah, a clone of the `gitoxide` repository will now work out of the box. A few first failing tests were also fixed, and that's probably just the beginning of the compatibility story as tech-debt definitely has accumulated by having tests use fixtures that were created on Unix.

### Yet another security issue fixed in `gix-transport`

Some time ago it was discovered that URLs could be used for arbitrary command execution by smuggling arguments to the invoked `ssh` command. Eliah found out that even after fixing it, the issue was still present. Now one would have to smuggle arguments with the user-portion of the URL instead. Having made this mistake twice, while trying to *fix* the issue the second time, being extra diligent, I hopefully have finally understood how dangerous it is to launch these kinds of programs, or maybe any kind of program. So much can go wrong or be abused.

Thanks to him, we also have an advisory and [the PR with the fix](https://github.com/Byron/gitoxide/pull/1342).

### Gix in Cargo

The improvement [described last month](https://github.com/rust-lang/cargo/issues/10150) actually led to a bug which 'only' [made it impossible to run `cargo build`](https://github.com/rust-lang/cargo/pull/13777) in projects with a special combination of symlinks and Git repositories. The fix was available about a day later, and it revealed a shortcoming deep down in the pathspec matching code, an insight I only gained [thanks to @weihanglo](https://github.com/rust-lang/cargo/pull/13777#pullrequestreview-2011470503) making me think about it more.

This interaction truly shows the power of actually having a library used. Naturally it runs into all possible cases, which now can be fixed and improved. This makes me think that one day, `gitoxide` will truly be the best possible version of itself, and one that is worthy of being used in place of `git2` which, despite all its shortcomings, is a fantastic accomplishment.

Cheers
Sebastian

PS: The latest timesheets can be found [here (2024)](https://github.com/Byron/byron/blob/main/timesheets/2024.csv).