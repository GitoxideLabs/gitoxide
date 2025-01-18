This month was full of exciting progress, let's dive right in.

## Tree Merge (Work in Progress)

Now that blob-merging, including textual content merges, was finished, I started on a very feared topic of mine: merging the actual trees. Doing a trivial merge without rename tracking is easy, of course, but adding rename tracking to the mix adds a level of difficulty, especially when thinking about clean (by identity) directory renames. And on top of that, we can put all the fancy capabilities that were implemented in the Git default merge algorithm, merge-ORT, implemented in a single file with a little more than 5000 lines of code.

It would have taken ~~a lot of~~unbounded time to try to understand it, and so I limited my research to high-level capabilities only. All the rest, the actual behaviour, is something that is very observable via baseline tests which let Git produce the expectations for the implementation in Gitoxide. It's as simple as producing the correctly merged tree, so their hashes match.

Armed with this baseline test and debug-printing by default to see program state more clearly, e.g. what does Git say, and what are our changes and their changes, and a bunch of pattern matching, the problem turned out to be very approachable after all. The beauty of it was that one could focus on one capability at a time and just get it to work step by step. Then, add the next test, rinse and repeat until all test cases poached from the Git test suite pass.

### Blob-Merging 

Despite blob-merging [being available](https://github.com/Byron/gitoxide/pull/1585), one day I noticed that I missed a very important aspect of merging anything: `merge a b` should be the same as `merge b a`. Assuring this is easy as each baseline test, for blobs and for trees, can just be repeated in reverse. Fortunately, Git also works like that, but it did turn out that `gitoxide`'s blob merge did not in one particular case.

That one case showed a major shortcoming in the algorithm which fortunately could be fixed without much ado.

After being confronted with my own fallibility it was clear that more testing is needed. What can do that better than the beloved fuzzer, who with great probability will find ways to trigger all those `.expect()` calls. And like predicted, it immediately found a crash, then a case of OOM (trying to draw 4billion conflict markers isn't a great idea), and many more panics until finally, the code ran for 2 hours on a couple of cores. I'd argue that every algorithm should be fuzzed, especially when there is a lot of calculation and logic happening.

#### `gix merge file`

To make blob content merging more approachable, there is now a [`gix merge file`](https://github.com/Byron/gitoxide/pull/1611) not dissimilar to `git merge-file` available on the command-line.
It does what it says, but outputs the result straight to `stdout` so there are no surprises (by default, `git merge-file` writes the result back to one of the input files).

## Community

### Gitoxide and the Sovereign Tech Fund - Open Collective

 Having made another step, `gitoxide` is now owned by `GitoxideLabs`, a small organization that helps with the proliferation of the project. With this it was finally possible to be fiscally hosted on Open Collective, here is the link: https://opencollective.com/gitoxide.

Now the only thing that's missing is me approaching the Sovereign Tech Fund, which seems to be done through [their WebPortal](https://apply.sovereigntechfund.de). They need very concrete project descriptions about the work to be done, and it will take time to complete anything there. It would truly be lovely to be able to talk to someone and learn how much effort this actually needs.

### The GitMerge 2024 in Berlin - Videos

And there was yet another, wonderful conference and my very first [GitMerge](https://git-merge.com). This time I was even given a speak-slot along with the generous and tremendous opportunity to [talk about `gitoxide` for 20 minutes](https://www.youtube.com/watch?v=r1LwDYtghPM&t)! The audience was chock-full of Git core contributors, Google, GitHub and GitLab employees, Git enthusiasts, and of course the fine folks (and conference sponsors) of [GitButler](https://gitbutler.com).

And I also could meet in person, for the first time, the wonderful crew behind [Radicle - the sovereign Forge](https://radicle.xyz).

While at it, please do also check out recordings of the other speakers, one more fantastic than the next (here is the playlist): https://youtube.com/playlist?list=PLNXkW_le40U6Igw7FcHgQGnQNDQ8kWyuE&si=BEXe7GwgYJXiJHki

### Oldest-first traversal

There was [a very interesting PR](https://github.com/Byron/gitoxide/pull/1610) which added a new traversal sorting, oldest-first, to the simple traversal, which sped up their traversal from 2.6s to just 12ms. Of course, every commit-graph is different and your mileage will vary, but it's nice to see what's possible with such a seemingly trivial change.

### `gix cat <revspec>`

While watching a fun video about Git where the UX-issues of `git cat-file` where pointed out once again I decided to quickly hack together the answer: a `gix cat-file` with perfect UX, just called `gix cat` for short.

It really doesn't take much and [here is the code](https://github.com/GitoxideLabs/gitoxide/pull/1616) for those interested.

### Chrstoph Rüßler gets a dedicated section this month

After realizing that everything that follows was contributed by a single person, let me say thanks once more to [Christoph Rüßler](https://github.com/cruessler) for his continued contributions. The following three sections are his work.

#### `GIX_WORK_TREE` support

Brought to us by the same person that also pushed `gix blame` to completion (review pending), is this innocuously looking improvement that turned out to hell of a journey to actually get working: `GIT_WORK_TREE` support for the `gix` binary.

Despite finally working after '*only*' 2 hours, it felt much like rebuilding the house around the nail. And for those who wish to know why, I summed it up [in a PR-comment](https://github.com/GitoxideLabs/gitoxide/pull/1639#issuecomment-2427426118).

#### `gix diff tree`

And finally, in order to make real-world debugging for `gix blame` easier, there is now a [`gix diff tree`](https://github.com/GitoxideLabs/gitoxide/pull/1626) command which exposes an algorithm that has been long available, but in API only.

This is also a perfect example of what `gix` is meant to be for - a tool for  developers to facilitate to develop tools with `gitoxide` and `gitoxide` itself. Everything is allowed, and it can start (and stay) very simple, while it's useful.

#### 'Blame' is getting there

Thanks to the introduction of the new and recently contributed `Topo` traversal `gix blame` is now matches the Git baseline even better, to the point where mismatches *just* seem to be related to the [slider problem](https://github.com/mhagger/diff-slider-tools/blob/master/README.md).
There were also some optimizations inspired by Git which together bring it very close to being very usable.

Now I still owe the 2500 lines of code a thorough review, and thus far couldn't allow myself to expend the amount of focus time I'd feel I would need to do so properly.
After all, I couldn't even make `blame` work in my head yet, so there is much to learn - it still just feels like magic to me ✨.

### Gix in Cargo

Still, there is no news here, yet I am excited for the dam-break that is definitely going to happen. `gix status` just needs one component to be complete, and that can already be useful. Also, I will revisit it once `gix merge tree` is usable, maybe it can benefit already.

#### Cargo-issue with non-ignored device-files in the package directory

There is one [related PR](https://github.com/GitoxideLabs/gitoxide/pull/1629) which I will push to completion as soon as possible as it will address a real issue with current Cargo that can happen for some. A workaround is probably quite easy, but of course, this really needs fixing. And if there is anything I have learned, then iti is that Filesystems are hard, and working with files is hard.

Cheers  
Sebastian

PS: The latest timesheets can be found [here (2024)](https://github.com/Byron/byron/blob/main/timesheets/2024.csv).