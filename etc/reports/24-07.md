Just 5 days after the progress report of June another strike hit, leading me away from the computer to help out in the real world, taking away another 47 hours of work while adding quite a bit of distraction. This new state is likely to persist for months to come, and all I can do is to get better in dealing with my new-found responsibilities.

My working hours between the last report and now went down from 175 to 128, with 39 spend on open-source and maintenance (down from 49). But of these, 32 hours were spent on `gitoxide`, a number very close to the 34 of last month.

Overall, once again, I didn't get to work on any `gitoxide` feature, while putting all 'feature-energy' into the two commercial projects I am also working on. I keep thinking that now that one of these should allow for more granular feature development, there should be more space for `gitoxide` as well.

## Another vulnerability: Executing imposter-executable on 32bit Windows machines

One would think that making `git.exe` more discoverable on Windows by hardcoding the two paths of standard installation directories isn't something where anything could go wrong. But that of course, isn't correct if one chooses the `Program Files (x86)` directory name which, on 32 bit Windows, doesn't exist. This also means that an adversary with user-level disk access can create it, and place a `git.exe` in just the right spot for it to be picked up by `gitoxide` to be executed.

`gitoxide` does that lazily, and requires the API-user to opt-in to it, but when using the `gix` executable for instance, many sub-commands will execute `git.exe` in order to learn additional paths that are needed for proper functioning, for example when performing a clone.

While discovered by Eliah, he kindly provided a fix as well which is a far superior way of determining installation directories programmatically, along with tests that show what kind of paths the algorithm will produce.

From this vulnerability, the `git.exe` discovery will soon be improved further to be en-par with the similarly implemented one in Visual Studio Code.

It's worth noting that the fix was distributed as patch-release of the `gix-path` crate, which is pleasant and easy to do, finally showing the benefits of the 'many-small-crates' model used here.

## Gitoxide and the Sovereign Tech Fund

Tracked in [this issue](https://github.com/Byron/gitoxide/issues/1406), there hasn't been much progress as it still requires me to write the application in a way that has a chance for it to succeed. It did take a moment to realise that the Sovereign Tech Fund (of course) wants me to equip them with a measure of success of their investment, which requires a more detailed plan than to just say: "Make `gitoxide` better".

For a while, this blocked me, but today I realised that this is a perfect opportunity to establish writing the application on-the-go, piece by piece, until it is complete. A gist can serve as storage which is then referenced in the PDF itself - it sports a lot of single-lines which are supposed to hold 300 words at most, so probably it's anticipated to put publicly available URLs there anyway.

With this out of the way, I hope to be able to finish writing the application in time for it to be considered, possibly in the last year this fund is available at all.

## Community

### Gix in Cargo

There is nothing to report here, once again :/.

Cheers  
Sebastian

PS: The latest timesheets can be found [here (2024)](https://github.com/Byron/byron/blob/main/timesheets/2024.csv).