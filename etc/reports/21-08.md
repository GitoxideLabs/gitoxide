### git-ref - welcome to the family

Last months rabbit hole can finally be waved good bye, now that packed-ref writing is implemented. It's interesting to realize that `packed-refs` are really only good for traversing large amount of references fast. When deleting refs, `packed-refs` have to be handled and potentially be rewritten if a ref-to-be-deleted is indeed present in that file.

Updating `packed-refs` is notably different from the canonical git implementation as it's implemented as part of the standard transaction system. This brings the benefit of
_pseudo_ transactions and rollbacks on error for free. The way this works is a special flag that can be set on the transaction to either enable write-through to `packed-refs`, and optionally trigger the deletion of the original ref. The latter essentially is a `git pack-refs --prune`, but with added safety in case of concurrent processes.

Another new feature is the addition of namespaces which can be activated in transactions to move all changes into their own, non-overlapping namespace. This is useful for servers that keep namespaced references, therefore allowing to use one repository for all forks of it. References can have their named adjusted by a namespace, or the namespace can be stripped of their name. Namespaces can naturally be used as prefix for iterating references, and generally need awareness and specific support by the application using them, making them a very explicit part of the crate's API surface.


### cargo smart-release - taking back control

With `gitoxide` now spanning over 20 published and interconnected crates, the trusty `cargo release` started to be far less helpful due to its one-crate-at-a-time mentality. This meant it couldn't help with releases, especially their order, at all, and I had to manually track the correct release order and pray for mercy each time I ran `cargo release`. Of course, mistakes would always happen and a release of all relevant crates could easily take 90 painful minutes. Way too much to be kept, so I set my sights on 'writing this quick hack in a day' that would handle publishes for me.

It would take three days and a few hours here and there to finally have a tool that seems to do the job exactly how I want it. Here are some highlights:

* publish one or more crates, and deal with dependencies automatically
* deal with dev dependencies which caused me a lot of trouble. Hint: Don't specify a version number to solve the problem easily.
* dry-run by default, providing an overview of what would be done
* automatic version bumping, but only if actually needed after consulting the crates.io index, along with automatic manifest updates of dependent crates
* it's very fast and doesn't impose wait times
* it's using the `git-repository` crate 

Now releases seem to be a solved problem, and as a positive side-effect `cargo release` also learned a few new tricks.

### git-repository - the final push

This crate is destined to provide the standard abstractions suitable for any kind of application or library. No matter whether it's single-threaded, multi-threaded, one-off CLI programs or long-running server applications. The performance is as good or not much worse than hand-rolling code using the plumbing crates while providing a convenient, high-level API similar to what's offered in the `git2` crate. Of course you pay a performance-for-convenience penalty only when needed.

None of the above was fleshed out, until now.

`git_repository::Repository` is now relegated to the plumbing API, a slightly more convenient way of accessing references and objects, but actually working with these needs a lot of knowledge and will feel cumbersome, especially in comparison to `git2`, but might give that extra percent of performance or control that some applications need. The majority of them, even high-performance servers, will use the `Easy` API. It comes in various flavours to cater to one-shot CLI applications on the one hand and similarly to long running multi-threaded servers on the other.

It works by segmenting all `git` data into those who are written rarely and consume system resources, and those who are read often, and those who are written often. The latter will always exist once per thread and are best described as 'caches and acceleration' structures, as well as memory to back objects. The former may be shared across threads, and depending on the `Easy*` in use, one may or may not acquire a mutable reference for altering them.

This system is needed to allow multiple objects to be _infused_ with access to the repository and occasionally change memory, while passing the borrow checker. You guessed it, we are using interior mutability and RefCells or RwLocks in various configurations to achieve exactly the performance trade off that certain applications need to perform best.

One major shortcoming is that there currently wouldn't be support for long-running single-threaded applications that are OK using Rc<RefCell> for shared mutable access due to the lack of generic associated types. These will, for now, have to use the multi-threaded handle called `EasyArcExclusive`.

`cargo smart-release` is already using it to great benefit, and we will keep dog-fooding the API in all of our current and future tooling.

### Gitoxide opens up

Now that `gitoxide`'s `git-protocol` crate is consumed by `radicle-link` it became clear that the way of working thus far is lacking in many ways. Namely, it lacks transparency for downstream consumer and which also makes it hard to impossible to influence. In other words, you won't know about a breaking change until it hits you, maybe even with a semver that suggests no breaking change at all.

`gitoxide` was already using a [project board](https://github.com/Byron/gitoxide/projects/1) to show what's going on and what's planned as well as issues to track overall work done, but it would be hard to see what code actually implemented said features or steps along the way.

To remedy this, we will now use PRs for planned breaking changes or for greater features that everyone is invited to influence. Of course, we will do our best to keep branches short-lived.

A new _collaborating_ guide was added to outline this workflow, and a stability guide is planned to define how to use versions and which stability guarantees can or should be made.

### Pack Generation - counting objects fast isn't as easy as it seems :/

Last month I left the topic thinking `gitoxide` can be faster than git when counting, but after remeasuring the linux pack performance it feels more like it's always slower, with multiple threads not even beating a single thread of canonical git even with this one optimization I had in mind already implemented. Thus, now the counting phase isn't implemented as iterator anymore but uses simple 'scoped-thread' based parallelism while allowing for an optimization for the single-threaded case with just a little more boilerplate.

Unfortunately it's absolutely ineffective, with the absolute majority of the time spent on decoding packed objects during tree-traversal. The algorithm already uses a skip-list to never process objects twice, but that's not enough. It still has to decode a lot of highly deltified packed objects, and the current caches for delta objects aren't very effective in this case. Not only do they take up some runtime for themselves, but also are they not hit often enough. Probably there should be some statistics on that, but even if one would know the hit-rates, there isn't so much you can do if the cache is too small and trashed too much.

The only two caches I have played with is a fixed size LRU cache based on `uluru` and a dynamically sized hashmap LRU with a memory cap. Now, one the cache is full all it's going to do is to trash memory like there is no tomorrow - a lot o memory is copied and allocated, and dropped and deallocated, probably 60 thousand times per second per core. When profiling the allocations I was quite surprised to see that they quickly went into the tenth of millions, with zlib allocations taking the crown, but cache trashing following right after. A test of the statically sized `uluru` with a free-list moved the cache trashing effectively but wasn't any faster either.

The best performance improvement could be witnessed with a 400MB dynamic cache which could bring down single-threaded performance to about 140s for 8 million objects, but git does it in 40s! Maybe a free-list for this cache can help and more careful tuning, but I feel I am running out of options :/.

Besides performance the `gixp pack-create` code was cleaned up and improved thanks to the 'counting objects' refactoring, and will soon benefit from `git-repository::Easy` as well.

### The bigger picture

From reading the above it's hard to see that we are still in the 'make git-fetch work' block. Now that `git-ref` is fully implemented to the extend needed, one can properly implement fetches that alter references after having received objects. `radicle-link` already does implement fetches, and this month's side-tracking was due to the desire to have it's tests be supported by `gitoxide` instead of `git2`, so a lot of work on `git-repository` was finally due. On the bright side this means that `git-repository` is now reasonably well defined to allow more and more features to be implemented on a higher level which in turn should better support existing applications as well as test setups.

For the coming month we will strive into the server side and build something akin to `upload-pack`, a first step towards having a git server.

Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2021.csv).