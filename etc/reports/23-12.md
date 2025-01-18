This month didn’t make significant progress towards getting a complete `gix status`, but happened to be mostly about improving correctness, a work stream that is just as important and which tends to ‘sneak in’ in the form of issues reported.

## Greater correctness

Whenever a new issue is reported I will interrupt all feature work and investigate to fix the issue as quickly as possible (which [isn’t always the case](https://github.com/Byron/gitoxide/issues/1025#issuecomment-1729480387)). What follows is the most interesting cases.

### `gix-command` learns about ‘context’

The `gix-command` crate is used whenever `gix` needs to spawn a command as it implements `git`-specific specialties on top of `std::process::Command`. When checking files out to the worktree, filter programs are invoked using it as well. Previously, when cloning, this also happened, but as it turned out it during my real-world testing, it only worked coincidentally.
 Imagine cloning `gitoxide` while having a current working directory in a parent gitoxide clone - when invoking filters, these can accidentally work in the current working directory even though they affect the clone, and not fail because of it.

To fix this, `gix-command` now offers a `Context`, which is most of the information that `git` will typically set or inherit for programs it launches. This also includes the current git repository, and the working tree. `gix` now makes it easier to obtain such a `command_context()` and help spawning command in the right context right away.

### `GIT_SSL_NO_VERIFY` support

This environment variable prevents the authentication of the server side certificate, which is useful for self-signed certificates or the lack of a trusted root certificate that was used to create them. Previously there was no need to respect it, but as it turns out some `gix` users to require it. That’s the best of motivations to finally make it happen, at least for the `curl` transport.
It’s also interesting to re-thread Git’s steps this way and learn why the opposite of `http.sslVerify` was introduced: convenience for real-work and not quite niche use-cases.

### User-controllable handling of `stderr` of spawned programs

This is an interesting issue as Git will always let its own output for `stderr` be inherited to spawned programs, which typically is desirable as one will see errors produced by there programs in the terminal. Obviously `gix-command` does the same. However, there are cases where that’s actually a problem, like when `cargo` is running its journey tests and system-installed credential helpers happen to be run and output information to stderr which then breaks the test-expectations.

The solution was as simple as to make the handling of `stderr` configurable to be able to either inherit it from the parent, or to disable it entirely. By default, it’s still enabled though which should help most users to understand what’s going wrong, *if* something goes wrong.

### More fuzzing

This sections is a bit of a hack in itself as it should be in the Community section, given its nature of being fully contributed. But with that out of the way it’s fair to say that there was another push adding fuzz-programs that test more of the API surface. Now particularly `gix-config` has more coverage, which within a day of Google computers fuzzing the crate led to another vulnerability that would cause a stack overflow. Abusing it is certainly not easy as configuration files are exclusively local to a user, but you never know. The issue was fixed the day it was found by avoiding recursion.

The fuzzer also successfully exploited recursive code in `gix-glob` which caused runaway runtimes along with stack exhaustion if one would wait long enough. Now the implementation sets a hard limit on the recursion depth, which should still be high enough to deal with even the most complex patterns users would typically come up with, while preventing wrongdoing otherwise.

## `@{upstream}` and `@{push}`

The revspecs above would previously fail with something akin to ‘unimplemented’ simply because the simple the underlying logic to figure out the tracking branch of any branch was quite complex. Not only would one have to apply all the rules to get the remote configuration affecting a branch, but one would also have to be able to apply refspec mappings (like `refs/heads/*:refs/remotes/origin/*` to convert from remote branches to local ones and the other way around.

It was a small prompt-program cutely named `lilgit` which caused me to take a break from `gix-status` related tasks and finally research and implement this crucial capability. This way, no program using `gitoxide` will ever have to hardcode such mappings anymore which will be wrong once the default mappings are changed. Technically, this would probably have affected only a fraction of the users, most including myself probably leave the defaults untouched.

It’s funny that only after implementing the logic I finally understood how these mappings truly work, which for the first time would allow me to confidently adjust these refspecs. Even though, it definitely still makes smoke come out of my ears when thinking about push-refspecs 😅.

Last but not least, `gix rev parse` can now be used with the `—reference` flag, in addition to the resolved object ID of the refspec also prints the underlying reference name. Thus, `gix rev parse @{upstream} —reference` would typically print `refs/remotes/origin/main` followed by the object id it points to.

## Community

A lot of the above was based on issues reported by the community, whereas the additional fuzzing was entirely contributed. Nonetheless, this section is probably the emptiest in a long time, it’s just that most contributions have been handled in the ‘Greater correctness’ section above.

### Gix in Cargo

The work on diff-correctness was completed, meaning that now it’s easy and fast to obtain buffers that are fully processed just like Git would. From there, it’s easy to apply any algorithm of choice to create a diff, and it’s also part of the ‘system’ to naturally handle binary files as well as external diff programs.

`gix-index` also gained the ability to perform all lookups in a case-insensitive fashion, which is important to be able to check if local files (in a case-insensitive filesystem) are tracked already.

Now the work is at the doorsteps of a very important new crate called `gix-dir` which will implement a `git`-style directory walk which uses .gitignore information and the `index` file to segment its output into untracked files and folders, as well as ignored ones. In theory, there isn’t too much to it, but practically there will be a lot of effort put into baseline tests that allow comparing the output of Git with the one of `gitoxide` to assure correctness.

Thus, I’d think that in January 2024, there will be a huge step towards finally completing `gix status`.

Merry Christmas, and a happy new year,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2023.csv).
