And 365 days later as of 2022-12-31, we are counting **106.492 SLOC (up by 44.354) and 170.544 lines total (up by 83.204 mostly due to auto-generated changelogs)** in **9.988 commits (up by 4.317)**. There are **51 crates (up by 24)** and 4 binaries, with`ein` and `gix` as part of `gitoxide` and `cargo smart-release` and `cargo-changelog` as part of tooling. There are **57 unique authors (up by 36)**. This means ~121 lines per day (down by 52) in ~12 commits each day (up by 1). On GitHub there are **5165 stars (up by 2386)** for ~6.5 stars per day.

The tool invocation `ein tools estimate-hours` now rates the project cost at **6822 hours (up by 3199) or ~853 in 8 hour working days**, for an average working time of **8.8 hours in the past 365 days**. My timetracker reveals that I **spent 1231h on open source work** which is dominated by `gitoxide` and which is supported [_via GitHub sponsors_](https://github.com/sponsors/Byron). **136h were for paid closed-source work and consulting**, whereas **the Rust Foundation grant motivated 460h** to improve `gitoxide` and drive grant-goals forward. The **total is 1828 hours worked** which boils down to **5 hours of work per day (35h per week)**. Coincidentally, this is inline with [my prediction from last year](https://github.com/Byron/gitoxide/discussions/285) where I was hoping for 5h per day of sustained work. More importantly, it is financially sustainable at a little more than two times the German minimal wage per net per hour.

Thus far, **I have dedicated the last 989 days to getting `gitoxide` off the ground**, and I feel **it's finally breaking through** 🎉.

### What's planned for 2023

The [`cargo` integration](https://github.com/rust-lang/cargo/pull/11448) will be the main driver in terms of features which should make `gitoxide` feasible for a great variety of projects. The anticipated feature-set would include…

- a GitHub action for faster clones and checkouts
- shallow clones
- fully functional worktree checkout and reset (with filters) and submodules
- native support for `git upload-pack` which would also be a building block for a git server
- a native `ssh` transport
- worktree status
- add worktree files to index and create a tree + commit

Probably we should add `push` support for good measure to complete everything related to transport git repositories over the wire. It would be an incredible feat to also have a first integrated `git` server up and running, on the level of `git-daemon` at least with options to turn it into a customisable HTTP server as well, even though it seems unlikely there is enough time for that.

In order to achieve all of the above, I hope that I can increase my sustained daily effective work time to 6h per day for 2190ish hours in total.

### Some words of Gratitude

By now I am able to humbly sustain myself and my family while following my calling, and for that I am incredibly grateful - I simply couldn't imagine a better use of my (life)time. Doing so would not be possible without the generosity of my sponsors: thank you, thank you so much!

This year also brought more contributions than ever before, and I am thankful for every single one of them, be it PRs with fixes and improvements, or discussions to help me see the problems `gitoxide` has to solve. Thank you for your contributions!

There are a few people and entities I would like to call out specifically this year, it's definitely personal :).

#### Thank you, Josh!

[Josh Triplett](https://joshtriplett.org/), a well-known member of the Rust community, back in May 2021 suggested to turn on the GitHub sponsorship feature  to become my first sponsor, and start making `gitoxide` financially sustainable. He supported me ever since in more ways than I can count and definitely changed my life to the better, having been incredibly impactful with maybe a few well-placed nudges here and there. `gitoxide` wouldn't be what it is today without him, and I am deeply grateful for that.

Let's make it happen in 2023 :)!

#### Thank you, Paul, of Codebase!

Earlier this year Paul approached me about bringing `git` onto the internet computer, an endeavour which since has resulted in the launch of [codebase.org](https://codebase.org). Thanks to him, `gitoxide` get's to go where no `git` has gone before, and to me the most notable artifact of our collaboration is the `max-pure` build target that became possible thanks to the sponsored `reqwest` HTTP transport. Thank you, Paul, for providing such a valuable perspective, and I am curious what 2023 brings :).

#### Thank you, Cargo team!

Even before `gitoxide` arrived in `cargo` I was implementing [my first `cargo` feature](https://github.com/rust-lang/cargo/pull/9992) thanks to the sponsorship of Profian that was arranged by, you guessed it, Josh Triplett. That was were I met the fine folks of the Cargo team whose humble and kind reviews are aspirational for the `gitoxide` project by now. Thank you all, for handling everyone's PRs the way you do, I absolutely appreciate your time and value every interaction.

I hope in 2023 we can bring `cargo` to new heights together :).

#### Thank you, Docs.rs team!

In 2022 I became a member of the docs.rs team - it all happened so fast! And as member of the team, we managed to lift the `crates-index-diff` crate that powers `docs.rs`'s build triggers onto a new quality level by bringing in `gitoxide` and an entirely new diffing engine. The latter wouldn't have been possible without the incredible work of [Pascal Kuthe](https://github.com/pascalkuthe), and I hope that won't be the last time we can excel together.
Now that the dust has settled I feel I can slowly grow into the docs.rs team, learn from its members and see how critical infrastructure is run in an open-source environment. Thanks for having me, I hope I can meet you one day.

#### Thank you, Rust Foundation!

With every breath I am turning `gitoxide` into _foundational_ software that is not only powering a growing number of applications but one day critical infrastructure as well. This year, the Rust Foundation provided grants to finance the development of `gitoxide` and its integration into existing software, and by continuing to do so it is a pillar of sustainable development. Thank you for your trust!

It is my hope that as the Rust Foundation evolves, it can help `gitoxide` to become more community driven without an over-reliance on a single person.

Have a great year 2023!

----

Thanks for reading, let's make 2023 a great year for everyone :)!
