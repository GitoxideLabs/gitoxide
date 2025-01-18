Compared to last month that felt like not much has happened, this month felt like more even though it's apparently not when looking at the amount of new features.

## Shallow clones and fetch

It feels I have wanted to do this for a year already, especially since the integration into `cargo` has started, and now it's finally here.

There are two parts to shallow repositories. The first is shallowness on the protocol level and how it's communicated to to the remote, and the second is where information about the shallow history is taken into consideration to not break. The latter part is quite straightforward as it is a list of commits whose parents don't exist in the repository. This list is now available and used where it matters, for example while traversing the commit graph.

The first part was the more complicated one as to me, everything on the protocol level is a little more complicated as it involves understanding how client and remote talk to each other. One might think it's easy, but when looking at communication between `git` and a remote it appears to have some special cases initially whose replication made code very messy. Fortunately, after the initial confusion was overcome and it was generally working, in a clean-up operation I was able to bring the code back to a more moderate level of complexity.

It's worth noting that the negotiation phase is key to handling shallow repositories correctly, and that's still only available as naive implementations without support for multiple negotiation rounds, so it definitely won't be the last time this has to be dealt with. Hopefully by then everything will become (even) clearer.

##### `gix clone/fetch shallow options`

To take shallow clones and fetches out of their test-sandbox, there is now the usual suite of shallow-related arguments for both `gix clone` and `gix fetch` which work similar to their git equivalents. It does work as advertised.

## Passwords in URLs

When writing the `gix-url` crate as one of the first crates in `gitoxide` I thought that it's a good idea to not make passwords representable by the `gix_url::Url` type to help with security. When would one ever need it?

2.5 years later reality knocked and reminded me that there are indeed use-cases despite me not seeing them. And so it came that finally the `password` field was added
to the `Url` type to allow it to represent, and serialize, passwords. Those who use private GitLab repositories along with a token in the URL are now able to use `gix clone/fetch` as well.

## Community

This month was tame in terms of contributions, but that should change *as Svetlin has rejoined*. Previously he helped with `gix-config` (formerly `git-config`) and `gix-date`, and now he will lead the development of the `gix-archive` crate and its integration into `gix` (CLI) for `git archive`-like functionality. Welcome back!

### Rust Foundation sponsorship: cargo shallow clones update

> We are getting close, I feel, and after having addressed a lot of review notes  it feels like the review notes get less numerous and smaller in scope. Most definitely a good sign. Since addressing review notes doesn't take too much time these days, I am optimistic that soon I can implement shallow clone support and make that available to `cargo` as well, then in a much smaller PR which will be mergeable much more quickly.

The above was a major part of this section last month and the reason it's repeated here is to amplify that for the first time in a long time, my prediction seems to have come through. The first integration PR has indeed been merged and new one that brings *shallow clones and fetches* to `cargo` is ready for review.

This definitely puts me back on track for replacing more parts of `git2` with `gitoxide`, and the current (very major) topic is worktree checkouts. There is a lot of machinery that makes an ordinary checkout possible, and much of it still has to be built to ultimately be as correct as `git`.

Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2023.csv).