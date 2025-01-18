_You are reading an email sent out to sponsors previously. Lots of formatting and links got lost unfortunately._

----
Dear Sponsors,

### Goal achieved - the first shots at pack generation

Pack generation came a long way and is now able to build valid packs at quite a decent speed, at least compared to git in 2017 before various caches arrived to speed up the process.

Right now the system can 'enumerate objects' with all of your cores and count 7 million objects in 50s on my machine - the linux kernel. Back in 2017 with 4.5 million objects it took several minutes to do that.

From there it gets interesting, and even though it can re-use existing base objects, all deltified objects are recompressed. This results in about 2 minutes of additional time on all your cores to produce 45GB at about 280MB/s.

It does that at about 630MB in memory mostly to hold the previously counted objects along with a set of objects that are waiting to be written.

### And we have async

Considerable work is on the way in order to make the existing client code optionally async. This can be toggled using a cargo feature flag was implemented incrementally, without breaking the world.

For now git-packetline is usable in async programs and paved the way for git-transport to follow in its footsteps.

### A new release - gitoxide 0.7

A new release was cut with a bunch of new user-facing features. The most notable might be gix organize, a way to structure existing clones on disk based on their remote URLs.

### Everything else

GitPython took some time to maintain and there has been some back and forth with me trying to cut a release and immediately yanking it a few hours later due to unforeseen breakage. The main cause was the addition of types, and another my inability to properly review python code.

As a major change of attitude I stopped considering GitPython a burden that I keep around like the 30 year old child that lives in the basement and really doesn't want to leave. Instead I see its massive user base as asset and plan to overhaul it with a new, and hopefully the last, major version that is literally gitoxides python bindings. Even thinking about it, it seems a little bit forced to turn GitPython into something else entirely, but that seems more sustainable than keeping GitPython around in its current form any longer.
In other words, GitPython can help push gitoxide to the masses and maybe even become the best, fastest, safest and most convenient way to interact with git repositories (besides using Rust directly, of course ;)).

### Goals for the next month

Most of the time will go into the asyncification of remaining git-transport and git-protocol crates, with the hopes that maybe there will even be enough time to implement a little bit of push now that we can generate huge packs.

The latter is 'nearly there' and once it is able to reuse delta objects in existing packs, it should build smaller packs much faster - certainly something I'd like to try out as well.

Releases are cut to surface new user facing tools and it's possible another one will happen next month to allow folks to play with pack generation.

Cheers,
Sebastian

PS: The latest timesheets can be found here
