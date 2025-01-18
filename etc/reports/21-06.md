### Meet the `git-repository` crate

It may be considered the last big piece in the puzzle as it represents the missing layer between the individual low-level `git-*` crates and the `gitoxide-core` crate that implements all functionality of the CLI. Hence it is now used by `gitoxide-core` to create demand to maintain and improve it. The latter serves as test-bed to see how applications would use `git-repository` with the goal to move `gitoxide-core` code into `git-repository` once its usefulness is sufficiently clear so that one day `gitoxide-core` is nothing more than a set of user facing tools with all plumbing being implemented in `git-repository` instead.

`git-repository` is made for applications, providing a higher-level convenience layer to combine all the plumbing crates into a coherent whole. In order to be usable for everyone it makes no sacrifices related to performance sometimes leading to a more complex call signature than other implementations like `git2`.

For that reason it's planned to provide an `easy` version of all interfaces which make decent choices for the user to increase usability while removing the possibility for certain optimizations as tunables are preconfigured. This is the API that ultimately will be used by the next generation of `GitPython`, a python frontend for `gitoxide`'s `git-repository` crate.

### An async version of `gixp pack-receive`

The work on the async client for fetch operations is completed to the point where `gixp pack-receive` can now be built using an async implementation. It serves more as a show-case on how blocking and async portions of the code can co-exists and how they differ.

Valuable insights where gained on how async-IO code will interoperate with the blocking `git-repository` and how all that integrates with cancellation.


### Reproducible pack creation

The biggest shortcoming of generated packs not being reproducible is now fixed at the cost of being able to count objects in single-threaded mode only. However, as this is a tuneable applications who don't need reproducibility can count with multiple threads to boost performance as before.

Both the counting and the compression phase now gather statistics to provide valuable insights into the improvements made by the next iteration. There it should become possible to stop copying existing packed entries into memory just to write them, and to be able to re-use existing delta-objects too. The latter will have a major impact to the size of the produced packs. For example, cloning the kernel pack currently produces 45GB as deltas are recompressed, while in future the pack will only be 1.3GB and come at nearly no additional cost.


### Signal-handling improvements to support `git-tempfile` and `git-lock`

On the way towards process-safe and race-aware implementations of writing git references as needed during the fetch/clone operation it's necessary to implement lock files the git way. This led to major improvements and greater usability of the way signals and interrupts are handled to the point where the current system will work for CLIs as well as servers which can have perfect control over what to do when application signals arrive along with fine-grained interrupt control for each running application. The latter will soon allow integrating an async server with blocking operations on the git-repository that are cancellable, for example to respond to timeouts.

With `git-lock` being out of the way the next step involves implementing transaction logic for writing loose refs, ref-logs and eventually packed refs.

### The bigger picture

All work drives towards supporting a complete client implementation to fully support fetches with negotiation as well as clones in an async fashion. This is likely going to be followed up with an async implementation of the server side of this, and the continuous pack generation work will be very useful for this.

From time to time I plan to 'take off' a day or two to scratch itches of my own by implementing small user facing utilities based on `git-repository`.

Cheers,
Sebastian

PS: The latest timesheets can be found [here](https://github.com/Byron/byron/blob/main/timesheets/2021.csv).