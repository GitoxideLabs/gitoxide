###  Working on Cargo

For an entire month I have been working predominantly on two cargo RFCs, namely [RFC-3028](https://github.com/rust-lang/cargo/pull/9992) and [RFC-3176](https://github.com/rust-lang/cargo/pull/10061).

Suffice to say, it wasn't easy and especially at the beginning I was struggling a lot. It was less about understanding the code, but more about what it would do with the very complex data that it operates on, i.e. cargo manifests.

Nearly out of ideas I finally remembered that tool which I haven't used for what feels like 10 years: a debugger! It turns out that
with it, it's so much easier to introspect program state that I quickly gained ground to gain enough understanding to make 
the changes necessary to implement the first RFC.

Cargo's test system is a pleasure to work with, and very 'real' as it builds actual workspaces and runs actual cargo against it. There are more than 2400 of these integration tests along with only a handful of unit tests. It was a quite a surprise that I managed to never get into a situation where I'd break any of them, which most definitely is a testament to the codebase and Rust for the most part.

Both RFCs are now being reviewed and I hope they will be merged soon.

### Community outreach

#### Learning Rust with gitoxide is nearly completed

By now Sidney and I have recorded [16 episodes](https://youtube.com/playlist?list=PLMHbQxe1e9Mk5kOHrm9v20-umkE2ck_gE) and will soon switch gears to recording an actual introduction into `gitoxide` as project. This should help him, and anyone, to get
started with using `gitoxide` and hopefully replacing some usage of `git2` some time this year.

#### GitHub organization and website

Once we have a logo we place to move to a github organization, and also setup a first version of a project website. Some work on this is planned to be done towards the end of the year.

The logo should be representing the three major aspiring features of `gitoxide`:

- becoming a framework for custom apps operating with git
- providing a production ready server implementation
- aspiring to be the most usable git frontend

The latter led me to rename `gix` to `ein`, which is hinting at `Einstein` as the exact opposite of `git`, and shall remind me of that it must be no less than a UX revolution that on top of that, will handle the biggest of mono-repos better than git does.

### Object Database Redesign Inbound

The past weekend [I was busy](https://github.com/Byron/gitoxide/pull/259) to tackle a long-standing problem which made using `gitoxide` surprising at times: object databases were entirely immutable during operation. While useful for scaling nicely across threads, it also meant that changes
to the underlying packs wouldn't be picked up automatically, even though manual and expensive refreshes were possible.

Furthermore the usage of system resources could be unnecessarily high as it would eagerly map all indices and packs.

The discovery led to a spike which shows how all of the following will work in a single object database, without generics or boxes:

- interior mutability of caches per thread, allowing each thread to only see the indices and packs it typically needs for its access patterns.
- lazy loading of indices and packs
- efficient refreshes if all indices are exhausted
- pooling of memory mapped indices and pack data files within a repository, or across repositories
- limiting of held memory maps
- control over when auto-refreshes are done on a per-repository-per-handle basis

A compile time feature toggle in `git-features` can be used to turn thread-safety off for single-threaded applications. The major benefit of the new approach is that it marries fast thread-local caches of shared pack indices and shared pack data with occasional synchronized access to a shared resource, instead of forcing slow access through a lock all the time.

Also note that the there will be a refactoring of how namespaces are handled within the loose reference database to assure it won't accidentally cause races across threads, while also clearing the path for the eventual integration of the ref-table database.

### Pack generation pushed back once more

> This also means that next month, there should be a bigger story to tell about packs.

The above is what I said previous month, and this month I had to push it back one more time, leaving even more time for this to be spent in November. Now that I am back with gitoxide mostly full-time, I should get there once the refactorings above are done.

### Looking  back…

…at the past month it's clear that `gitoxide` didn't get a lot of attention. Looking back at the past few days it's also clear that its tires are squealing. Despite having been through a few refactorings before, after this one I am sure that `git-repository` and its abstractions will finally feel right across the board, so I can't wait to fix up and measure the performance of thew new object database implementation.

Furthermore I am looking forward to…finally…make some progress towards a fetch-like operation which really isn't missing much. Despite that, there is implementations of additional server features clearly written on the horizon, like multipack indices, ref-tables as well as reverse indices.

Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2021.csv).
