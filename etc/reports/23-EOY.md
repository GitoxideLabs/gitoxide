##### The year in numbers

And 365 days later as of 2023-12-31, we are counting **141,005 SLOC, up by 34,513**, which is 75% *of the year before* (➡*OTYB*) in **13,002 commits up by 3,014** and 70%OTYB. There are **62 crates (up by 11)** and 2 binaries, with `ein` and `gix` as part of `gitoxide`. There are **105 unique authors (up by 48 and 133%OTYB)**. This means ~95 lines per day in ~8 commits each day. On GitHub there are **7,266 stars (up by 2,101 which is 88%OTYB)** for ~5.8 stars per day.

The tool invocation `ein tool estimate-hours` now rates the project cost at **8736 hours (up by 1914) or ~1092 x 8 hour working days**, for an average working time of **5.24 hours in the past 365 days**.

My timetracker reveals that I **spent 1576h on open source work** which is dominated by `gitoxide` and which is supported [_via GitHub sponsors_](https://github.com/sponsors/Byron) at 997h. **469h were for commercial work and consulting**. **The grant of the Rust Foundation grant motivated 241h** to improve `gitoxide` and drive grant-goals forward. The **total of 2045 hours worked** boils down to **5.6 hours of work per day (39.2h per week)**, 112%OTYB.

My open-source work is financially sustainable at 2.5 times the German minimal wage net per hour, or 125%OTYB *(note that there is also income through commercial work which isn't included in this value)*.

Thus far, **I have spent the last 1354 days to getting `gitoxide` off the ground**, and it's still quite far from even reaching parity with `git2`.

### What was planned for 2023 - a retrospective

There was a (probably unreasonably long) list of items to be done in 2023, let’s have a look to see what actually happened.

##### The previous list

- [ ] a GitHub action for faster clones and checkouts
    - I didn’t even work on it probably out of fear it opens up a rabbit hole of massive proportions, and it was easy to rationalise it further by saying that it’s good to not spread `gix` even more and deal with the additional support this would entail.
    - I still think it’s absolutely worth doing though.
- [x] shallow clones
- [x] fully functional worktree checkout and reset (with filters) and submodules
    - It was quite a bit of work to implement everything that was needed, like attributes, filters, pathspecs.
    - Submodule handling during checkout isn’t actually implemented, but submodules can be read and traversed and with that it should be quite straightforward to add this capability.
- [ ] native support for `git upload-pack`
- [ ] a native `ssh` transport
	- I am absolutely not looking forward to this one as it will need me to dabble in FFI and a lot of `unsafe`, but it's a requirement for `cargo`.
- [x] worktree status (*partial*)
    - index-to-worktree diffs are actually implemented and seem production ready, but additional pieces of a full status are still missing.
- [ ] add worktree files to index and create a tree + commit

It does look like only about 40% have been achieved, or less, but I also think that the list was meant to be more of a wish-list than anything that could be reasonably be achieved.

[Last year I said](https://github.com/Byron/gitoxide/discussions/681) right below the list:

> In order to achieve all of the above, I hope that I can increase my sustained daily effective work time to 6h per day for 2190ish hours in total.

Not too bad, the actual value is 5.6h per day which could generously be rounded. It's clear though that even with that additional time these lofty goals would not have been achieved.

### What's planned for 2024

Having learned from last year, I will do my best to keep the list of this year (*more*) reasonable and achievable.

* complete worktree status
* basic worktree reset
* add worktree files to index and create a tree + commit
* Clone by hard-link
* support for built-in `file://` protocol

With the above, all of `git2` in `cargo` could be replaced with `gix`, while at the same time moving `gix` up to near feature parity with `git2`. When that comes through it's probably time for a stable release, which in itself is a massive undertaking that's not possible with the way `gix` is currently built.

Nonetheless, looking at this list along with the major progress with the `cargo` integration that it enables makes me very happy and excited for what's to come :).


### Some words of Gratitude

By now I am able to sustain myself and my family while following my calling, and for that I am incredibly grateful - I simply couldn't imagine a better use of my (life)time. Doing so would not be possible without the generosity of my sponsors: thank you, thank you very much!

Judging by the 48 new contributors, this year brought even more contributions than the previous one, and I am thankful for every single one of them, be it PRs with fixes and improvements, or discussions to help `gitoxide` become more useful and usable.

Additionally I'd like to call out the contributed [OSS-fuzzing support](https://oss-fuzz.com) which has found many bugs already and hopefully keeps finding more due to ever-increasing (and contributed) coverage. Thanks so much!

There is one person and entities I would like to thank individually just like last year :).

#### Thank you, Josh!

[Josh Triplett](https://joshtriplett.org/), back in May 2021 became my first sponsor and *patron*, which did no less than change my life to be able to follow my calling. `gitoxide`, me and my family wouldn't be what they are today without him, and I am deeply grateful for that.

As if this wasn't enough, we doubled-down on [`buildit`](http://buildit.dev), making incredible strides, and I remain hopeful that 2024 will be the year "to make it happen" :)!

#### Thanks, Radworks!

[Radworks](https://radworks.org) is dedicated to cultivate internet freedom. They created a peer-to-peer network for code collaboration based on Git, which is the reason we touched base back in 2021.

Two years later they are back, this time with a peer-to-peer fund sharing and splitting solution that `gitoxide` is an early benefactor of, and so much so that its future is secured just by that alone.

I am unlikely to be able to thank them enough, but will try by making `git2` a dependency they won't need anymore.

#### Thank you, Cargo team, for bearing with me!

It's taking me years to finish the integration work and implement all features needed to fully replace `git2` in `cargo`, and yet the `cargo` team stays onboard with this work!

Thanks so much, but… I will need just a little more time 😅.

#### Thank you, Rust Foundation!

With every breath I am turning `gitoxide` into _foundational_ software that is not only powering a growing number of applications but one day critical infrastructure as well. This year, the Rust Foundation kept providing a grant to finance the development of `gitoxide` and its integration into existing software, and has been a pillar of sustainable development. Thank you again for your trust!

It is my hope that as the Rust Foundation evolves, it can help `gitoxide` to become more community driven without an over-reliance on a single person.

#### Thanks Everyone

It’s very likely that I failed to call *you* out for no other reason than swiss-cheese like memory, so let me thank you for the net-positive interactions we undoubtedly had.

Let’s do that again in 2024 :)!

----

🎉🎉🎉 Thanks for reading, let's make 2024 a great year for everyone :)! 🎉🎉🎉

----

### Q&A

#### Why did the development velocity decrease?

The pure line-of-code produced is down by 25% and the amount of commits is down by 30%. They might be correlated, even though I'd think that [Stacked Git](https://stacked-git.github.io) is the main reason for the reduction in commits.

As for the reduced amount of code, I *think* that overall it's not less, but more or less the same. It might be that most of the 'missing' code is in commercial projects or went into `git2->gix` conversions. Of course, having a 140k SLOC project should make development slower, but as most code is still written from scratch I think the effects of the amount of code are small. Having tests for everything also is a key-enabler for fearless changes, and so is Rust.

Maybe it's just a feeling, but I do think that the problems to solve are getting more complex as well,  and I feel I have to research more to grasp how to implement a certain Git capability. That probably contributes to taking quite a bit longer.

#### Will `gitoxide` ever be done?

Yes, definitely! Even though done won't mean absolute feature parity with Git, as I only plan to implement what's immediately useful to me and most others.

Knowing my velocity in lines of code and the size of `libgit2` and Git respectively, a silly estimation would be that it takes another 2 to 3.5 years for `gitoxide` to be complete. Stabilising `gitoxide` in 2 years would definitely be swell!


<details><summary>Data</summary>

##### State
```
❯ git rev-parse @
c3983c6b8d63d85ec713ae8d661723f9cf0bd55b
```

##### commit count
```
❯ git log --graph --pretty="%Cred%h%Creset -%C(auto)%d%Creset %s %Cgreen(%ar) %C(bold blue)<%an>%Creset"' | wc -l
   13002
```

##### Linecount

```
===============================================================================
 Language            Files        Lines         Code     Comments       Blanks
===============================================================================
 JSON                    1            7            7            0            0
 Makefile                1          158          112           10           36
 Shell                 134         7358         5995          283         1080
 SVG                     1            1            1            0            0
 Plain Text             29          637            0          504          133
 TOML                   90         3628         2606          425          597
-------------------------------------------------------------------------------
 Markdown               86        61714            0        47401        14313
 |- Python               1           10            6            2            2
 |- Shell                2            8            7            1            0
 (Total)                          61732           13        47404        14315
-------------------------------------------------------------------------------
 Rust                 1203       157566       141005         1408        15153
 |- Markdown           746        14442            2        12343         2097
 (Total)                         172008       141007        13751        17250
===============================================================================
 Total                1545       231069       149726        50031        31312
===============================================================================
```

##### Authors

```
❯ ein t h
 15:55:46 traverse commit graph done 11.9k commits in 0.11s (113.6k commits/s)
 15:55:46        estimate-hours Extracted and organized data from 11935 commits in 63.375µs (188323472 commits/s)
total hours: 8736.36
total 8h days: 1092.04
total commits = 11935
total authors: 108
total unique authors: 105 (2.78% duplication)
```

</details>