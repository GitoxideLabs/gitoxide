### The ultimate rabbit hole: Loose file reference DB

Even though in June the foundation was set with `git-tempfile` and `git-lock`, correctly writing loose references requires more than 'just writing a file', the convenient story I told myself. The culprit here is that `gitoxide` can never be intentionally worse than `git`, and studying the canonical implementation showed how much care is taken to assure consistency and decent operation even in the light of broken references.

One of the ways to accomplish this is to use transactions, a bundle of edits, which we try hard to perform in a way that can be rolled back on error. It works by preparing each updated or soon-to-be-deleted ref with a lock file to block other writes which receives updates in place of the actual reference. Under most circumstances, a reflog is written as well which is supposedly guarded by the same lock.  Once all lock files have been created, the transaction can be committed by moving all locks onto the reference they lock or by removing the reference accordingly. There is a ton of edge cases that are tested for and handled faithfully, even though ultimately these transactions aren't really transactional as many non-atomic operations are involved.

As a side-effect of this, reflogs can now be written, iterated in various ways, and packed-refs are incorporated into find results or when iterating references. As always, `gitoxide` chooses not to handle caches internally but delegates this to the user. For all one-off operation this feels very natural, whereas long-running tools will most certainly get to resort to `git-repository` in some shape or form.

For fun I have run very unscientific benchmarks and saw ~6mio packed references traversed per second, along with 500.000 packed reference lookups per second, per core.

Even though I have worked on it for more than a month already it's still not done as packed references aren't handled yet when doing transactions. These will have to be removed from there on deletion at the very least, and it's certainly possible to think of ways to prefer updating packed-refs instead of loose refs and avoid spamming the file system with many small files. However, due to the nature of the loose reference  DB, loose references are the source of truth and lock files are needed in any case, so performance improvements by handling packed-refs a little different than canonical git can't really happen. What can happen is to auto-pack refs and avoid creating loose file references which may be more suitable for servers that don't yet have access to the `ref-table`.

And of course, all `gix` and `gixp` tools were upgraded to make use of the new capabilities and became more convenient in the process.

### Get small packs fast*

The improvement for packs this months is very measurable. In the last letter I was talking about linux kernel packs weighing 45GB which are written at only 300MB/s. This was due to delta-objects being fully decompressed and then recompressed as base objects, with the latter operation being limited by `zlib-ng` which already is the fastest we know.

Now the tides have turned with the introduction of delta-object support as well as support for thin packs. The former feature makes it possible to directly copy existing packed delta objects to the new pack instead of recompressing them, which writes 3.6GB in 5.4 seconds. This is done by processing multiple chunks of pack entries in parallel and bringing them on order on the fly for writing.

The currently produced pack sizes are suitable for cloning and fetching, even though we still can't produce delta-objects ourselves.

Another weak spot is the counting stage as `gitoxide` currently can't use caches, nor is its single-threaded counting speed en par with canonical git. While counting the linux kernel pack is done in ~50s with multiple threads and thus about 2.4x faster than git, with a single thread we are ~20s behind. This gets worse on smaller repositories with canonical git clearly beating gitoxide which barely keeps up with a single thread of git even with all threads at its disposal.

One reason for this seems to be the use of a `DashSet` even for single-threaded operation which seems to cost about 40% of the runtime. In the next month this should be fixable by offering a single-threaded codepath for the counting stage which is required for deterministic packs, and try to make the existing multi-threaded version faster by using scoped threads instead of a threaded iterator. The iterator design for the counting stage was a mistake as at the time I wasn't aware that counts cannot be streamed, the amount of objects in the pack needs to be known and additional sorting has to happen on the list of input objects as well.

\*if the objects destined for the pack have already been in a pack.

### Bonus: Bad Objects

A significant amount of time allotted for my work on pack generation went into dealing with 'bad objects' adequately. These are to my mind _impossible_ objects that are referenced during commit graph traversal but don't actually exist in the repository. The Rust repository as nearly 1500 of them, the git repository has 4, and `git fsck` doesn't mind them nor does git care when producing its own packs.
In the statistics produced by `gitoxide` these now show up and when eventually implementing `gixp fsck` I will be sure to provide more information about them. Are they blobs or trees, which commit do they belong to, and any other information that can help understand how these have been created in the first place. My guess is that these are from a time long gone when it was possible to 'loose' objects, and `git` ignores them knowing that nothing can be done to get these objects back.

Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2021.csv).