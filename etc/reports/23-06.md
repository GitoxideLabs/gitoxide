And another seemingly productive month even though it was wrapped up by the first bout with COVID (that I am still recovering from). Maybe that's also why I keep it more brief than usual.

## Integration of `gix-negotiate` into `gix`

Low-level negotiation algorithms are now driven by the `gix` implementation of the fetch negotiation loop, right after implementing yet another surprise called `mark_complete_and_common_ref()`, a function that is the part of the negotiation algorithm that is shared among all implementations.

During testing, I found more opportunities to further protocol support with combinations of V1 and stateful/stateless transports which made V1 support much better than before to the point where it seems to be working just fine (and I am looking at you, `NAK` line from hell that needs intrinsic knowledge to get right).

## 64 bit date support

From a level of importance point of view, this improvement has to be mentioned early on as `gitoxide` can now easily represent dates further ahead than 2038, and is hereby future proof in the literal sense :). The trigger for this was further commit-graph work (read on) which led me to realize that `git` has learned 64 bit dates in the course of the past 2 years - time flies!

## Nicer in-memory configuration overrides

When writing tests I noticed that it was a bit clumsy to use configuration keys, which are the default way of accessing configuration values and validating them.
Now they can be used for setting values, too, even though setting the value itself still looks a bit clumsy unless on happens to have a `&BStr` around.

```Rust
config.set_value(
    &gix::config::tree::Fetch::NEGOTIATION_ALGORITHM,
    algorithm.to_string().as_str(),
)?;
```

## Deep commit-graph integration

With negotiation improvements came graph primitives, and with these came commit-graph integration not only in the `gix_revision::Graph`, but also in the venerable `gix_traverse::commit::Ancestors()` iterator which now optionally makes use of  a commit-graph. This is facilitated by providing more information about commits during traversal, like their parents, their commit-time and possibly their generation, as this information was obtained during traversal anyway and can be leveraged by the caller.

Thanks to integration all the way down the stack the commit-graph is now used automatically during any traversal, while respecting git configuration related to the commit-graph which could prevent that. Typical traversals are now easily 6.5 times faster with it, it really makes a difference.

##### `gix revision list` and `gix commit-graph list`

`gix revision list` now shows all information that is easily available from an iteration, adding the commit-time as well as the amount of parents for brevity, while `gix commit-graph list` sub-command was added to show all data that is present in the commit-graph if one is available.

## SVG-Graphs of Revisions

When working with negotiation algorithms I always wished there was a way to visualise these to understand them better. And thanks to the `layout-rs` crate that's now (mostly) possible, as long as the graph doesn't span more than 1000 revisions or so.

What's quite revealing is `gix fetch --open-negotiation-graph=path.svg` which will show visualise the negotiation graph as it was after the negotiation was finished along with all flags that were set per commit.
And for good measure, one can enable simple graph visualization for `gix revision list --svg` as well.

## `gix corpus` and `--trace`

I recently became aware of the power of the `tracing` ecosystem and wanted it badly. Thus far, `gitoxide` only had `progress` reporting which could also emit messages, but it's a limited system that lacks finesse outside of what it is good at. For a while I thought about adding `logging` integration to be able to add details about how certain portions of the code are running, but it seemed too simple. Then with `tracing` it became clear that `spans` is what I wanted, along with structured logging facilities.

`gitoxide` now integrates with `tracing-core` using the `gix-trace` crate which by default compiles to nothing at all. Only when `tracing` feature is enabled by the application will it actually incur a cost, and then only the bare minimal one as `gix-trace` is quite minimal compared to its `tracing` counterpart.

With that it's possible to see how fast certain parts of the code are running, and thanks to `tracing-forest` these visualize beautifully. All this can be activated with `gix --trace`.

`gix corpus` is a runner over a set of repositories which stores information about each run in a sqlite database for later analysis. What's neat about it is that it collects the tracing spans and serializes them into a database as well. With that, it should be possible to compare details of each computation with each other while only really taking care of tracing.

Now I just have to find a way to more regularly perform `gix corpus` runs and to finally figure out an easy way to analyse the information contained in it.

## Community

### Rust Foundation sponsorship

The negotiation upgrade was promptly merged mostly by upgrading `gix` to the latest version, so `cargo` should now work correctly with certain [crates.io](http://crates.io/) proxies which are particularly necessary in China. Next up is a filtering system to be able to perform correct worktree checkouts.


Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2023.csv).