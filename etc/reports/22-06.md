When looking at this months progress I feel like there wasn't too much progress visible, even though there of course is a lot to talk about.
Let's get to it!

## rev-spec parsing

A major capability of git is that when it sees names like `HEAD`, it actually parses it as revision specification. This is a micro-language to define revisions in various ways and apply transformations to it, and there is a heap of features is implements, many of which are probably unknown to most but the linux kernel developers ;).

I for sure learned a lot when implementing the parser in 'typical fashion', that is the transformation of the specification for 'rev-specs' into a checklist of capabilities to implement, which was then turned into test and the respective implementation that passes them.

The benefit of this workflow is that it takes only a short session of 30min to 1h to produce visible improvements and typically checking at least one box. The latter is a little gratification, too, and makes progress more obvious to oneself.

That way I was chugging away for weeks to end up with a faithful implementation down to the last detail, with some improvements over the git implementation as well.

## Beginnings of rev-spec resolution

The parser described above doesn't do anything though, it merely calls a delegate to provide information of what was just parsed. Implementing such a delegate so that objects can actually be resolved isn't quite so trivial either, but again it's the workflow that provides the needed confidence.

The test-setup is such that a shell script builds git repositories with interesting histories, and the one I chose to learn from was the disambiguation tests from the test-suite of git itself. With these I realized early that git does a lot of work to handle ambiguity and help the user to disambiguate, either automatically by context or by providing a helpful error message.

The tests themselves run the partial implementation of the delegate in `git-repository` against these test repositories and compare the result against the baseline provided by git itself. That way, we know if we should fail, or which object git resolves exactly. And like that one can be confident that the final result will be faithful especially in the tougher cases.

Starting with learning about the disambiguation of rev-specs helped to understand the otherwise hidden complexities early which will affect the architecture of the delegate and will trigger additional improvements to `git-odb` to actually allow to return all candidates for an object prefix search (e.g. `0000` returns `0000a` and `0000b`). More work is needed to finish this, but it's definitely at a state where one can 'chug away'.

## `gix repo revspec explain` and beginnings of `gix repo revspec parse`

As always, no new functionality feels quite like it's useful unless one can run it against real-world repositories easily. Thus `gix repo revspec explain` (actually repository-independent) allows to verbalize all calls the delegate receives from the parser with the net-effect of showing what a rev-spec really does.

In order to better grasp the quality of errors, I decided to also and immediately implement a minimal version of revspec parsing, the `gix` equivalent of `git rev-parse`. And even though most rev-spec features aren't implemented yet, it's already usable for some few and it's just nice to run it from time to time.

## Community

Currently there are three community-powered work-streams going on.

### git-config `includeIf` support matures

Despite not being merged yet as some validation and refactoring is still in progress, one can certainly claim that all the common conditional include patterns are now available, such as `gitdir:`, `gitdir/i:` and `onbranch:`.

The PR validation is a bit more involved as I have to map the specification (as documented) to the corresponding test while also trying to break it, while also gaining a better understanding of both the test setup as well as the requirements myself. A highlight of the test-setup clearly is that `gitdir` tests are also run against a git baseline to validate both implementations actually agree.

I hope that the PR can be merged soon so that an MVP of date parsing can be implemented in `git-date` - it's used both in rev-specs as well as in the git-config in the form of expiry dates.

### pathspec-parsing

Another kind of spec is the pathspec, which allows git to know which files to add to a repository for instance. Once again, I argue that many are unaware of the things they can actually do when running `git add <pathspec>`, but fortunately parsing these path specifications is much less complex than parsing rev-specs for example.

Thus they are parsed into a data structure which can later be used to actually match against them, a topic which is completely out of scope in this PR and which will probably have a lot of details to take care of when the time comes.

### Starship.rs and `gitoxide`

The maintainer of `starship` decided to replace some parts of `git2` with `gitoxide` and contributed a lot of functionality to `gitoxide` in the process. It's wonderful to see this happen as it's clearly one of the reasons `gitoxide` needs to exist: it enables Rust developers to implement what they need which isn't easily possible in `git2` and the underlying `libgit2` (in order of increasing difficulty).

Sidney and I hope to be able to help more with the transition which should allow `starship` remove its `git2` dependency and improve its performance, particularly on windows.

## Rust Foundation sponsorship

I am super happy that my application was granted which means that `gitoxide` will soon help `cargo` to perform shallow clones of the crates index as well as source checkouts of crates themselves. This should greatly reduce energy consumption and wait times on CI as well as locally, so promises huge payoffs for the time invested. It's also the first step to replace more parts of the `git2` usage in `cargo` with `gitoxide` for better performance and better code readability thanks to more convenient APIs.

It's shall not be forgotten to note that the Rust Foundation is also looking into sponsoring another team member to make `gitoxide`'s development more sustainable.

## Outlook

This year is going to be a big one for `gitoxide` as at the end of it, one will be able to clone repositories faster than is possible with git, and `cargo` will benefit immediately. From there I am sure more features will be implemented for which people usually reach to `git2` to kick-start community adoption beyond 'lighthouse' early adopters like `onefetch` and `starship`.

This also means from the first of July priorities will shift to make the grant goal come true, which not-so-coincidentally aligns perfectly with the worktree-checkout work I have been doing already. Attribute lookup per path to checkout is still to be implemented, which will happen as soon as possible along with the fist analysis and RFC work to align with the `cargo` team on how to proceed with the `gitoxide` integration.

Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2022.csv).
