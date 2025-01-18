##### The year in numbers

And 365 days later as of 2024-12-31, we are counting **178,156 SLOC, up by 37,151**, which is 108% *of the year before* (➡*OTYB*) in **15,271 commits up by 2,269** and 75%OTYB. There are **78 crates (up by 16)** and 2 binaries, with `ein` and `gix` as part of `gitoxide`. There are **151 unique authors (up by 46 and 96%OTYB)**. This means ~102 lines per day in ~6 commits each day. On GitHub there are **9279 stars (up by 2,013 which is 96%OTYB)** for ~5.5 stars per day.

The tool invocation `ein tool estimate-hours` now rates the project cost at **10629 hours (up by 1893 which is 99%OTYB) or ~1329 x 8 hour working days**, for an average working time of **5.18 hours in the past 365 days**.

My timetracker reveals that I **spent 892h on open source work** (which is 57%OTYB) which is (still) dominated by `gitoxide` and which is supported [_via GitHub sponsors_](https://github.com/sponsors/Byron) at 619h. **1020h were for commercial work and consulting** (which is 217%OTYB). The **total of 1912 hours worked** boils down to **5.2 hours of work per day (36.8h per week)**, which is 94%OTYB.

My open-source work is still financially sustainable even without income through commercial work, which is something I am most grateful for. 

Thus far, **I have spent the last 1719 days to getting `gitoxide` off the ground**, and it's still quite far from even reaching parity with `git2` despite making great strides. Even though feature-wise it's getting closer, it's still unclear how to get everything to stability while also making the API easy-to-use, yet powerful enough to squeeze out every last bit of performance.

### Plans and reality

When looking at the "What was planned for 2023" section in the [last year's retrospective](https://github.com/GitoxideLabs/gitoxide/discussions/1223) it's astonishing that I seemed to have accomplished none of the items left or planned, with the exception that `gix status` is nearly ready at the time of this writing.

Then again, none of the other unfulfilled items on the plan were actually attempted.

This brings us to the highlights of the features *actually* completed in 2024, as picked from the previous monthly reports, in order of (utterly subjective) importance. *And for brevity, I skip over the large amount of smaller improvements and fixes that happened in the course of the year.*

#### Blob and tree merge

In what felt like more than two months of work a complete implementation of a `merge-ORT` style tree-merge algorithm was implemented, along with a complete and fully-fuzzed blob-merge. It seems to be a bit faster than `git merge` in many (but not all) cases and a lot faster than the tree merge in `git2`, while offering additional features to help auto-resolve conflicts while still 'communicating' them.

This is particularly useful if one were to produce trees that make sense from *our* point of view *automatically* while knowing that the auto-resolution still has to be replaced by a user-controlled one *at some point in time*, a convenience implemented in [GitButler](https://gitbutler.com). They were also the sponsor for the entire feature so it could be geared to work particularly well for such a modern developer tool, speeding up their merges by up to 3x.

A smaller but no less important feature that powers all of that is the new tree-editor. It supports sparse immediate edits to later write out only the trees that changed for maximum efficiency.

#### `gix clean` with precious files

Definitely my personal favorite and a tool I use often if `gix clean` with its awareness of [precious files](https://github.com/GitoxideLabs/gitoxide/discussions/1308). These files are not to be tracked, but also not disposable, and a great way to keep your editor-configuration safe. 

It's powered by the new `gix-dirwalk` crate which helps to classify entire directories, and is the basis for `gix status` and future `gix add` as well.

#### `gix status` (*nearly there*) 

It has been such a long time in the making, and despite best attempts (i.e. my really trying a week before the year's end), it's still not quite there. But what's there is very promising as it seems to be 1.85 faster than Git on the WebKit repository already. Git is very optimised, but it's not as parallel as it could be which is where `gitoxide` has its major gains. The directory walk to find untracked files and the index-refresh are run in parallel, something that ultimately is faster even though Git would otherwise be a bit faster if run sequentially.

As a special feature, `gitoxide` implements the `status` query as iterator which *moves the work out of the consuming thread*, something that will further accelerate real-world applications without any added complexity on their side.

Of course, the complexity has to go somewhere and `status` as it's implemented now is a multi-layered monster of what seems like essential complexity. 

#### A very first `gix blame`

Thanks to [Christoph Rüßler](https://github.com/cruessler) and his tireless work (*as well as super-human patience with me*) we now have a very first working version of `gix blame`. It works!

Early next year we would expect its performance to become comparable to Git as well, which has many more optimizations that really make a difference. With a little luck it has a good chance to be faster as well.

It's well worth mentioning that I think that `gix blame` has to the potential to become the fastest blame implementation available, while being the most suitable for [user interfaces](https://github.com/extrawurst/gitui) as well.

#### Ephemeral objects and API improvements

Often it's useful to just *try* something without leaving any trace of it. This could, for example, be answering if something would merge cleanly or not.
Now it's possible to turn on [in-memory objects](https://docs.rs/gix/0.69.1/gix/struct.Repository.html#method.with_object_memory) so all new objects are kept in memory instead of ending up as garbage on disk. 

On top of that, various convenience methods have been added to make the API around these parts as easy to use as they are in `git2`.

#### A great year for security

Thanks to the generous sponsorship of [Radicle](https://radicle.xyz) via [Drips](https://www.drips.network) the fabulous [Eliah Kagan](https://github.com/EliahKagan) could join and make sure that [exploitable vulnerabilities](https://github.com/GitoxideLabs/gitoxide/security/advisories) don't stay hidden for long!

I sure learned a lot while working with him even though I still wouldn't claim to be able to write perfectly secure code, despite using only safe Rust. It's very tricky, and truly is an independent skill to develop.

#### Release Notifications

It was also him who added countless other improvements, among which also is *release notifications*. Just subscribe to [this discussion](https://github.com/GitoxideLabs/gitoxide/discussions/1693) to be informed only about `gitoxide` releases, something that typically happens once a month. 

### A word of Gratitude

By now I am able to sustain myself and my family while following my calling, and for that I am incredibly grateful - I simply couldn't imagine a better use of my (life)time. Doing so would not be possible without the generosity of my sponsors and the trust of my clients: thank you, thank you *very much*!

Another big thanks goes to the 46 new contributors whose fixes and improvements helped `gitoxide` get closer to the best possible version of itself.

Some shoutouts shall follow.

#### At your service, GitButler!

When GitButler got in touch, judging by my initial response, I must have *felt* that this could be a life-changing event. And even that might have been an understatement. 

After all it's not just helping to make [GitButler](https://gitbutler.com) the best Git client and developer tool. It's also feeling like being a part of the global Git community, with a massive opportunity [to meet all the people](https://merge.berlin), just before getting [to speak on GitMerge](https://www.youtube.com/watch?v=r1LwDYtghPM) about `gitoxide`. 

And if this was just the warmup, I can't even imagine how 2025 is going to be. Let's find out!

#### Thank you, Josh!

[Josh Triplett](https://joshtriplett.org/), back in May 2021 became my first sponsor and *patron*, which did no less than change my life to be able to follow my calling. `gitoxide`, me and my family wouldn't be what they are today without him, and I am deeply grateful for that. Nothing ever has to change about this sentence.

As 2024 turned out to be great in more ways than one, I am glad to say that [`buildit`](http://buildit.dev) came along well despite not yet being available to the public. But it's getting there, we will make sure of it 🙌.

#### Thanks, Radicle!

[Radworks](https://radworks.org) is dedicated to cultivate internet freedom. They created [a peer-to-peer network](https://radicle.xyz) for code collaboration based on Git, which is the reason we touched base back in 2021.

In September 2023 through 2024 `gitoxide` became an early benefactor of [Drips](https://www.drips.network), which alone would have been enough to secure its future. Thank you!

I am unlikely to be able to thank them enough, but will try by making `git2` a dependency they won't need anymore.

#### Thanks, `git2`!

Speaking of `git2`, it's a lighthouse project to me which shows how to do `libgit2` bindings right. It definitely sets the standard for convenient and easy-to-use APIs, it's not easy to match, and definitely something to aspire to.

Further, they split the majority of their donations on [Drips](https://www.drips.network) with `gitoxide`, which made bringing on [Eliah Kagan](https://github.com/EliahKagan) to greatly enhance its security posture possible.

I will do my best to do well by you and truly make `gitoxide` the project that makes it the go-to Git crate in the Rust ecosystem.

#### Thank you, Cargo team, for bearing with me!

It's taking me years to finish the integration work and implement all features needed to fully replace `git2` in `cargo`, and yet the `cargo` team stays onboard with this work!

Thanks so much, but… `gix status` is now unstoppably coming, and soon I will get to continue the integration. 
2025 - the year it continues!

#### Thanks Everyone

It’s very likely that I failed to call *you* out for no other reason than swiss-cheese like memory, so let me thank you for the net-positive interactions we undoubtedly had.

Let’s do that again in 2025 :)!

----

🎉🎉🎉 Thanks for reading, let's make 2025 a great year for everyone :)! 🎉🎉🎉

----

### Q&A

These really were questions I asked myself when writing the report, and I thought they might be interesting to share.

#### Why were there fewer commits than in the last year, despite more code overall?

The sole reason for this certainly os [StackedGit](https://stacked-git.github.io), a tool that facilitates maintaining patch-queues. That way, commits get rewritten over time to remain a logical unit with a clear commit message of what was done, instead of being a sequence of commits that merely document changes over time.

#### Why is the overall time worked down by 133h?

This amount of time seems to match the time I spend on 'special events' related to care-taking. As people get older, this will probably not get better. It's worth noting that these hours are focus-time, without breaks, so I'd think overall the pace of work is the same. Of course, I am always trying to do more, but it's probably not going to happen.

<details><summary>Data</summary>

##### State

```shell
❯  git rev-parse @
6ed9976abaa3915b50efa46c46b195f3a1fc4ff7
```

##### Commits

```shell
❯ git log --graph --pretty="%Cred%h%Creset -%C(auto)%d%Creset %s %Cgreen(%ar) %C(bold blue)<%an>%Creset" | wc -l
15271
```

##### Linecount

```
===============================================================================
 Language            Files        Lines         Code     Comments       Blanks
===============================================================================
 JSON                    1            7            7            0            0
 Makefile                1          158          112           10           36
 Shell                 162        13825        11481          589         1755
 SVG                     1           21           21            0            0
 Plain Text             34          686            0          548          138
 TOML                   95         4377         3195          454          728
-------------------------------------------------------------------------------
 HTML                    1          327          324            0            3
 |- CSS                  1           12            2           10            0
 |- JavaScript           1            1            1            0            0
 (Total)                            340          327           10            3
-------------------------------------------------------------------------------
 Markdown              136        85644            0        63866        21778
 |- Dockerfile           1            4            3            0            1
 |- Python               1           10            6            2            2
 |- Rust                 2           64           61            0            3
 |- Shell                2            8            7            1            0
 (Total)                          85730           77        63869        21784
-------------------------------------------------------------------------------
 Rust                 1346       198154       178156         1964        18034
 |- Markdown           841        18988            2        16172         2814
 (Total)                         217142       178158        18136        20848
===============================================================================
 Total                1777       303199       193296        67431        42472
===============================================================================
```

##### Authors

```shell
❯ ein t h
 08:39:54 traverse commit graph done 13.5K commits in 0.10s (141.9K commits/s)
 08:39:54        estimate-hours Extracted and organized data from 13540 commits in 55.75µs (242869968 commits/s)
total hours: 10629.23
total 8h days: 1328.65
total commits = 13540
total authors: 151
total unique authors: 146 (3.31% duplication)
```

</details>