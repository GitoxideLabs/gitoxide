### Checking out the linux kernel in under a second

The main accomplishment was definitely that, which actually led to checking out the linux kernel in 400 milliseconds, which is quite an accomplishment. I even tried to improve the way it parallelizes
but had to learn the hard way that the current system seems to play best with all parts involved. Maybe [the new way of parallelization](https://github.com/Byron/gitoxide/blob/0b4b90fa498d9e07a55b72af2f799da4cd2da81f/git-features/src/parallel/in_parallel.rs#L91-L90) has a place when decoding packs though, which is why I left
it for later use.

And finally, [here is the article](https://github.com/Byron/gitoxide/discussions/349).

### Community

#### "Getting into Gitoxide" delivers 'short_id()`-like functionality

Short IDs are deceptively simple - after all we only have to take the hexadecimal representation of an object ID and truncate it to a given length. The hard part though is to assure that this truncated version still unambiguously points to a the input object.

This is where `lookup_prefix()` comes into play, a new method on the object database and all of its underlying primitives to find a single object given a prefix.
It's worth noting that `git` has a somewhat more optimized implementation than ours, but a more complex one as well. We opted for simple and can always work on something that can perform better
in some cases once we learn about performance issues in the wild (which I presume won't happen).

#### Work on git-config progresses at constant pace

After making some simplification to how values are handled, we are now working on making `"include.path"` directives work. Considering that we are bound to be as correct and lenient as `git`, there is
lots of little details to consider which have caused quite a few additional tests to properly replicate git's behaviour.

It's fantastic that we make progress continuously as it means that in due time, `git-config` will be ready for prime time which is absolutely needed before `git-repository` can be delivering similar outputs like git - figuring out the correct committer and author names is just one example.


### Tidbits

#### Multi-pack indices and tricky ref-delta bases

While working on checking out the git kernel tree, the first thing it did was to claim that a certain object didn't exist. How could that be? It obviously did exist and git could see it just fine.
Some investigation showed that the multi-index was the issue. Deleting it made the object resolution work, creating it made it fail with the very same object.

The first issue was that the current `find` closure passed to the `checkout` method was intentionally downgrading errors to `None` which it would interpret as `does not exist`, which in turn
led to questionmarks above my head. After fixing this to properly propagate errors it became clear that a ref-delta object couldn't be resolved. The cause for that was that the previous implementation
of object lookup would only allow ref-deltas to objects in the same pack, as everything else would constitute a thin-pack at rest which I thought was forbidden.

But we didn't have a thin-pack at rest, otherwise deleting the multi-pack index wouldn't work - so what's the problem? The problem is that objects may exist in multiple packs, and when creating a multi-pack index only one object may be stored in the index. This means that when resolving the ref-delta, we may find the object to live in another pack than our own. With multi-pack indices, we aren't able
to differentiate these anymore unless we load the original index for the pack we are currently resolving objects in.

The way this is resolved is, I believe, somewhat similar to git as we are resolving such bases (which should be rare, mind you) recursively at the cost of one allocation per base. This also means that currently, if we would meet thin packs at rest, we would be able to resolve them even though those shouldn't exist. And if multi-pack indices are used, we wouldn't be able to differentiate between a thin-pack and a legitimate ref-delta unless we load the original index file which can be done but would make the code even more complex.

This means that right now, gitoxide will happily and generally resolve thin-packs at rest, which probably is a good thing even as it's merely convention to resolve them before writing them to disk, but if that doesn't happen I think users would be happy to be able to read their data nonetheless.

#### The first race in the ODB implementation

Sometimes, CI could fail with the ominous: object not found error. This one seemed legitimate though as we don't squelch errors on lookup anymore. So what happened, and how can it be reproduce locally?

Luckily, `hyperfine` is great in executing a binary many times and collecting useful information on the way. Using `hyperfine ./target/debug/<test>` after a thousand runs or so which happens in just a couple of seconds one would reproduce the error. Some classic `debug-print` style debugging later, it could be pinned on a race during initialization of the ODB which could cause the loosing thread to see no data at all.

The fix was simple, and one can only hope that this was the last one of these kinds of issues. Developing the ODB as is took a long time particularly because races have to be handled with extreme care,
but ultimately only running code is able to see if race-freedom was truly achieved. CI finding more of these is actually the desirable case as that makes spurious issues reproducible all the time with 
the help of `hyperfine`.

### Rich repository information

In order to help with debugging the 'tidbits' above, I have added a whole slew of commands to print information about pieces of a repository.

- `gix repo tree info`
    - just for fun to see how fast a tree can be traversed recursively (something required to build an index eventually), but allows to count the number of bytes for all of its contents as well.
- `gix index info`
    - Provide interesting information about the index which as far as I know won't be provided by git itself either
- `gix repo odb info`
    - Insight into the current status of the object database, and I could imagine it to be extended in the future to provide more object counts.


### What's cooking

We are still on the `gix clone` journey to try making a non-bare clone possible. The building blocks for the _bare_ one are already there but I think it's worth waiting for `git-config` to be able to write values back as well. Hence there is no need to rush towards _bare_ clones just yet.

_Non-bare_ clones require a lot of work as it's so much more involved than just writing files to disk. In order to checkout anything properly on windows, for example, one needs to be able to apply built-in filters at least, which in turn can be configured by git-attributes files. And from there, a whole world of handling user filters opens up.

Did I mention that we also need to checkout submodules to some extend? It's very involved… but definitely doable and as we know now, there is performance gains waiting to be unlocked.

Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2022.csv).