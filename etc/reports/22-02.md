[Gitoxide in February]: `gix index` sub-command, more contributions along with a new close contributor

### The new `gix index` sub-command

With the ability to read git index files last month, this month these read-only capabilities were made available on the command-line.

One can now do the following:

- **list entries**
    - print what matters most about index entries, while printing paths as UTF-8. The latter is much nicer than gits default 'encoding' which prints a whole bunch of numbers instead, turning `"tests/fixtures/unicode-numbers/\357\274\221\357\274\222\357\274\223"` (via `git ls-files`) into `BASE    FILE e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 tests/fixtures/unicode-numbers/１２３` via `gix index entries`.
- **info**
    - learn about what's in the index, along with detailed information about the TREE extension if present.
- **verify**
    - validate the checksum and invariants on entries themselves as well as all extensions

Performance wise it doesn't have to hide, reading 570k entries along with the TREE extension in a 53MB index file in 40ms in single-threaded mode. It's hard to obtain similar values from git as there is no command that would just read the index, so as of now there is no comparison.

### Community

#### Welcome svetli-n

He became aware of gitoxide through the [Rustacean Station](https://rustacean-station.org/episode/055-sebastian-thiel/) podcast, reached out and after a fun chat decided to contribute to gitoxide for the entire year. We meet twice per week to discuss the ongoing PRs, all geared towards getting `git-config` to becoming ready for primetime.

Here is the [tracking issue](https://github.com/Byron/gitoxide/issues/331), and here is [the first PR](https://github.com/Byron/gitoxide/pull/327) 🎉.

#### A PR for work-tree checkouts

As advertised previous month, the current work centers around the index and the work tree, with one major step towards cloning a repository being the ability to checkout files from an index file.

AP2008 [contributed it single-handedly](https://github.com/Byron/gitoxide/pull/315) and we both learned a lot in the process.

Here are the learnings on my side:
- face to face introductions are beneficial for initial alignment of long-running PRs, and text-only ways of alignment seem to be less effective
- it's highly beneficial for me to have completed analysis on the topic to know exactly what's needed for guidance and direction. Otherwise I end up in ad-hoc studies of what's going on in git which provide only a partial view, which can be frustrating as I won't be able to put the newly gained knowledge to the test by coding but rather have to write about it in comments.
- the more review rounds a PR is getting, the more likely I am to be more efficient when reviewing, which unfortunately also changes the language and tone towards something more 'telling' which I am not fond of. 

This PR required a lot of work on both sides and definitely wasn't sustainable for me as it was difficult to get the quality I envisioned. Ultimately I felt the need to put it into shape myself to end it.

It's still on my list of work-items to finish analysis of work-tree and git-index related capabilities to better guide potential contributors and help them with smaller and well-defined tasks.

#### VERGEN integration and live-coding on YouTube

This month Sidney and I have produced 10 `Getting into Gitoxide` episodes and made good progress on our quest to provide the features needed to replace `git2` in `vergen` with `gitoxide` adequately.
The feature we are currently implementing is short-id support on the level of `git2`.


### Cleanup of path encoding and decoding

While reviewing the 'worktree checkout' PR it became once again evident that `gitoxide` doesn't _really_ know how to deal with paths correctly. This insight led to the creation of [issue 333](https://github.com/Byron/gitoxide/issues/333) which helped to unify all path conversions from and to bytes and finally create clarity.

In short, there is a systemic bug that creeped into the unicode standard due to issues with UTF-16 which led to the rise of the potential of ill-formed UTF-8. The latter can prevent the conversion to UTF-8 from succeeding on windows only, which would prevent both git and gitoxide from dealing with these paths.

On windows, git effectively stores paths in UTF-8, whereas on linux no encoding is assumed except for something that allows to find an ascii slash as separator and that doesn't contain null bytes.

With all path conversions being unified via `git-features::path`, `gitoxide` will be safe for now, making changes easy should they become necessary in the future.

### The first breaking change in the stable `git-tempfile` crate

`git-tempfile` - one of the two stable crates - got a fix with breaking changes while adhering to the stability guide that mandates stability for at least 6 months.

The crate optimized to be 'easy' to use so much that it auto-installed signal handlers that by default would abort the process after cleaning up tempfiles. Despite having been a well-meant means of assuring the core feature of `git-tempfile` is available to everyone, this also means that anyone having `git-tempfile` in its dependency tree will have handlers installed which can be surprising if process termination should be controlled in the applications own handlers.

A first step towards a fix was to remove the auto-installation of handlers, with the plan to remove the dependency to `signal-hook` entirely when the next stable release is possible.

### Cargo RFC PR was merged

Just today, 2022-02-22, the [PR adding 4k lines of code to cargo](https://github.com/rust-lang/cargo/pull/9992) and nearly 200 conversation items was finally merged. It was a tremendous endeavour for me, and I learned a lot about cargo's inner workings.

The feature itself, named artifact dependencies, isn't even fully realized yet as there is [another PR](https://github.com/rust-lang/cargo/pull/10061) waiting to be reviewed and to receive follow-up adjustments that have simply been postponed. Oh, and have I mentioned another [follow up issue to add artifact support to the cargo registry](https://github.com/rust-lang/cargo/issues/10405) 😅?

I do have hopes to get through all of it till May.

### Where did my time go?

Having felt like there wasn't much of progress in gitoxide itself, at least compared to previous months, I wondered what I did with my time.
Here is the breakdown of the total of ~124h worked since the last sponsor update (a little less than usual thanks to Christmas/New Year):

- ~94h of open-source work
    - ~83h for gitoxide
        - ~13.5h for reviewing PRs
        - ~10.5h for mentorship and/or outreach via YouTube
        - ~3h of what seems like pure 1:1 mentorship (but might be tagged incorrectly)
        - ~56h of development
    - ~4h of GitPython maintenance
    - ~7h for maintenance work in all other of my open source projects
- ~29.5h of other work
    - Of this value, ~19h went into getting [one out of two](https://github.com/rust-lang/cargo/pull/9992) cargo PRs merged.

What's new is that about 1/3 of my time in Gitoxide now go towards mentorship, PRs and outreach, while ~1/4th of my total time went into non-gitoxide related work. Or presented differently, only 45% of my time were actually spent on gitoxide development as in 'me writing code', which
seems like the explanation for my feeling of not having achieved that much.

As with all feelings, they are likely to lead one astray of not put into perspective. Spending ~27h on tasks that aren't pure development help building community around gitoxide and actively foster its adoption. ~29.5h of other work are of course just as important as they are usually a source of income.


Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2022.csv).