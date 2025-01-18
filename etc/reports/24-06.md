With the sickness of last month mostly overcome, I did return to ordinary working hours and found the month productive thus far. However, I still didn't find the peace to work continue on `gix status` and the diffs of indices with trees, unfortunately, but at least there are some smaller improvements to share.

And just to be sure, of 175 working hours between the last report and now, only 49 went into open-source and maintenance, of which 34 were accounted directly to `gitoxide`. My commercial work has been intensifying, and it will probably stay that way for a while.

## Bugfixes and no Performance improvements

I have a rule that says that when there is a bug that can reasonably be fixed (like 99% of them), then it must be fixed with haste.

The [first issue](https://github.com/Byron/gitoxide/issues/1405) was about an error message related to failing to receive a pack, which seemed like a usability issue on the surface. When digging in though it became clear that this was due to a logic error that cause the remote side to not respond with a pack at all. It turned out that `gitoxide` was able to try to negotiate despite not knowing a single reference that it wants from the remote. Now it can abort early enough just like Git.

The [second issue](https://github.com/Byron/gitoxide/issues/1404) was related to Windows and what seemed to be a `tempfile` issue at first. Fortunately, it was related to how newly received packs are written to disk and moved into place. The problem here was that if a pack *that already exists* is received, something that can happen with shallow fetch settings enabled, it would try to overwrite an existing pack which has been memory mapped due to the negotiation stage to figure out which objects should be in said pack. The solution here was to not write empty packs at all, while also not trying to overwrite existing packs or indices. This is probably very desirable as well, as we know that duplicate files have the same content, and there is no need to meddle with packs that are already present and known to be valid.

## The Merge

The fine folks at GitButler invited me to [The Merge](https://merge.berlin/), two days full of excitement, fantastic speakers, and even more fantastic visitors. By mere chance I managed to also bring Johannes Schindelin, maintainer of Git for Windows, and Sebastian Schuberth, the author of the original Inno-based installer which still installs Git for Windows today.

On the Git contributor side, which I shall restrict myself here today, I also med Ed Thomson, the owner of the fabulously portable and powerful `libgit2` project. We had good conversation which left us, I hope to claim, mutually inspired to improve how our projects are maintained, while hopefully also affecting other Git libraries positively in the process.

There is also the plan to run more Git related meetups in Berlin from now on and return to pre-covid times - overall there was a lot of excitement for Git as one could imagine on a conference like this, and a lot of good will towards Gitoxide.

### Gitoxide - the next level

On the Merge I met Daniel from the Tauri project, which in so many ways is something I want to achieve for `gitoxide` as well. They have their own foundation which can deal with donations, but is also the entity that has ownership of the project and IP.

It's also painfully clear that the more I work commercially, the less time there is for `gitoxide` which delays progress and maybe makes people wonder if it's ever going to get where it needs to get.

And it seems there is only one solution to that: trying to scale beyond me. With the onboarding of Eliah this already happened to some extend, but I also think there is room for more. There are funds that would probably be happy to invest in core technology like `gitoxide`, and all that's missing is me to ask and… to be able to receive and spend money in the name of the project  without making me or my tax office unhappy.

So, just today I [did more research](https://github.com/Byron/gitoxide/issues/1406) in viable next steps and concluded that the Open Collective would solve the financial host problem for a fee of 5% to 10% depending on the fiscal host. That's absolutely viable and would solve a huge problem fast, without inhibiting future evolution towards a dedicated foundation, similar to the one `tauri` is having.

Of course, all of this means nothing without money to spend which is something I will apply for very soon.

## `gix-config` - API initiative

In an effort to 'soft-restart' `gitoxide` and get back into it, I thought it was a good idea to rid myself of as many PRs as possible. The oldest of them [was this 6 months old PR](https://github.com/Byron/gitoxide/pull/1236) which I could finally push over the finishing line.

As a result, one is now able to much more conveniently specify configuration keys like `remote.origin.url` both when getting values, but also when setting them, which finally brings both sides of the API in sync.

I do hope that this is finally it with `gix-config`, which already has a massive API surface with a lot of flexibility. It seems I'd have to use it much more to know for sure.

## Community

### Checkout specific references like `git clone --branch`

The second PR to accelerate merging [was the one that implements `--branch`](https://github.com/Byron/gitoxide/pull/1403). Once taking the helm it quickly turned out to be pretty complex to implement and even I struggled to find all the right places that needed modifications. Fortunately, the current structure and abstractions held up very well so it wasn't a problem… it's just a lot of code in many places and I could feel how my prior knowledge decompressed.

in the end, one is now able to do `gix clone --ref foo …`  to download just a single ref, and it acts just like Git from there and as far as I can tell. On the bright side, `gitoxide` solves this with a refspec like one would expect, which then automates some details that Git seems to deal with more 'manually' (and if I read the code correctly). So overall, it was a pleasant experience even though a contributor would need a lot of prior knowledge for even a chance of implementing such a simple-sounding feature correctly.

Probably not everything one can implement in `gitoxide` would turn out to be difficult like this, but I suppose nothing is easy once it involves the Git protocol.

### Gix in Cargo

There is nothing to report here, which seems like a first. If I would be able to finally finish the `status` implementation, a lot of integration tasks both in Cargo and other projects would be unlocked. Let's hope this can happen soon - the only one in the way is me and me spending time on scaling certainly doesn't help in the short term, but one may see it as investment.

Cheers
Sebastian

PS: The latest timesheets can be found [here (2024)](https://github.com/Byron/byron/blob/main/timesheets/2024.csv).