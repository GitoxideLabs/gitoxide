[![CI](https://github.com/GitoxideLabs/gitoxide/workflows/ci/badge.svg)](https://github.com/GitoxideLabs/gitoxide/actions)
[![Crates.io](https://img.shields.io/crates/v/gitoxide.svg)](https://crates.io/crates/gitoxide)
<img src="etc/msrv-badge.svg">

`gitoxide` is an implementation of `git` written in Rust for developing future-proof applications which strive for correctness and
performance while providing a pleasant and unsurprising developer experience.

`gitoxide` provides the `gix` and `ein` binaries for use on the command-line to allow experimentation with key features
like `fetch` and `clone`, and to validate the usability and control of the API offered by the [`gix`] crate.

`gitoxide` aspires to be a production-grade server implementation and the `ein` binary aspires to become the default way to interact with git repositories.

[![asciicast](https://asciinema.org/a/542159.svg)](https://asciinema.org/a/542159)

[`gix`]: https://docs.rs/gix

## Development Status

The command-line tools as well as the status of each crate is described in
[the crate status document](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md).

For use in applications, look for the [`gix`](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix) crate,
which serves as entrypoint to the functionality provided by various lower-level plumbing crates like
[`gix-config`](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-config).

### Feature Discovery

> Can `gix` do what I need it to do?

The above can be hard to answer and this paragraph is here to help with feature discovery.

Look at [`crate-status.md`](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md) for a rather exhaustive document that contains
both implemented and planned features.

Further, the [`gix` crate documentation with the `git2` search term](https://docs.rs/gix/0.54.1/gix/?search=git2) helps to find all currently
known `git2` equivalent method calls. Please note that this list is definitely not exhaustive yet, but might help if you are coming from `git2`.

What follows is a high-level list of features and those which are planned:

* [x] clone
* [x] fetch
* [ ] blame
* [ ] push
* [ ] reset
* [ ] status
* [x] blob-diff
* [ ] merge
    - [x] blobs
    - [ ] trees
    - [ ] commits
* [ ] rebase
* [ ] commit
* [x] worktree checkout and worktree stream
* [x] reading and writing of objects
* [x] reading and writing of refs
* [x] reading and writing of `.git/index`
* [x] reading and writing of git configuration
* [x] pathspecs
* [x] revspecs
* [x] `.gitignore` and `.gitattributes`

### Crates

Follow linked crate name for detailed status. Please note that all crates follow [semver] as well as the [stability guide].

[semver]: https://semver.org

### Production Grade

* **Stability Tier 1**
  - [gix-lock](https://github.com/GitoxideLabs/gitoxide/blob/main/gix-lock/README.md)

* **Stability Tier 2**
  - [gix-tempfile](https://github.com/GitoxideLabs/gitoxide/blob/main/gix-tempfile/README.md)

### Stabilization Candidates

Crates that seem feature complete and need to see some more use before they can be released as 1.0.
Documentation is complete and was reviewed at least once.

* [gix-mailmap](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-mailmap)
* [gix-chunk](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-chunk)
* [gix-ref](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-ref)
* [gix-config](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-config)
* [gix-config-value](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-config-value)
* [gix-glob](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-glob)
* [gix-actor](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-actor)
* [gix-hash](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-hash)

### Initial Development

These crates may be missing some features and thus are somewhat incomplete, but what's there
is usable to some extent.

* **usable** _(with rough but complete docs, possibly incomplete functionality)_
  * [gix](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix) (**⬅ entrypoint**)
  * [gix-object](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-object)
  * [gix-validate](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-validate)
  * [gix-url](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-url)
  * [gix-packetline](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-packetline)
  * [gix-packetline-blocking](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-packetline)
  * [gix-transport](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-transport)
  * [gix-protocol](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-protocol)
  * [gix-pack](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-pack)
  * [gix-odb](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-odb)
  * [gix-commitgraph](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-commitgraph)
  * [gix-diff](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-diff)
  * [gix-traverse](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-traverse)
  * [gix-features](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-features)
  * [gix-credentials](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-credentials)
  * [gix-sec](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-sec)
  * [gix-quote](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-quote)
  * [gix-discover](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-discover)
  * [gix-path](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-path)
  * [gix-attributes](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-attributes)
  * [gix-ignore](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-ignore)
  * [gix-pathspec](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-pathspec)
  * [gix-index](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-index)
  * [gix-revision](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-revision)
  * [gix-revwalk](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-revwalk)
  * [gix-command](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-command)
  * [gix-prompt](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-prompt)
  * [gix-refspec](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-refspec)
  * [gix-fs](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-fs)
  * [gix-utils](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-utils)
  * [gix-hashtable](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-hashtable)
  * [gix-worktree](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-worktree)
  * [gix-bitmap](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-bitmap)
  * [gix-negotiate](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-negotiate)
  * [gix-filter](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-filter)
  * [gix-worktree-stream](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-worktree-stream)
  * [gix-archive](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-archive)
  * [gix-submodule](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-submodule)
  * [gix-status](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-status)
  * [gix-worktree-state](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-worktree-state)
  * [gix-date](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-date)
  * [gix-dir](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-dir)
  * `gitoxide-core`
* **very early**  _(possibly without any documentation and many rough edges)_
  * [gix-merge](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-merge)
  * [gix-blame](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-blame)
* **idea** _(just a name placeholder)_
  * [gix-note](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-note)
  * [gix-fetchhead](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-fetchhead)
  * [gix-lfs](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-lfs)
  * [gix-rebase](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-rebase)
  * [gix-sequencer](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-sequencer)
  * [gix-tui](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-tui)
  * [gix-tix](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-tix)
  * [gix-bundle](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-bundle)
  * [gix-fsck](https://github.com/GitoxideLabs/gitoxide/blob/main/crate-status.md#gix-fsck)

### Stress Testing
  * [x] Verify huge packs
  * [x] Explode a pack to disk
  * [x] Generate and verify large commit graphs
  * [ ] Generate huge pack from a lot of loose objects

### Stability and MSRV

Our [stability guide] helps to judge how much churn can be expected when depending on crates in this workspace.

[stability guide]: https://github.com/GitoxideLabs/gitoxide/blob/main/STABILITY.md

## Installation

### Download a Binary Release

Using `cargo binstall`, one is able to fetch [binary releases][releases]. You can install it via `cargo install cargo-binstall`, assuming
the [rust toolchain][rustup] is present.

Then install gitoxide with `cargo binstall gitoxide`.

See the [releases section][releases] for manual installation and various alternative builds that are _slimmer_ or _smaller_, depending
on your needs, for _Linux_, _MacOS_ and _Windows_.

[releases]: https://github.com/GitoxideLabs/gitoxide/releases

### Download from Arch-Repository

For Arch Linux you can download `gitoxide` from `community` repository:
```
pacman -S gitoxide
```

### Download from Exherbo Linux Rust repository

For Exherbo Linux you can download `gitoxide` from the [Rust](https://gitlab.exherbo.org/exherbo/rust/-/tree/master/packages/dev-scm/gitoxide) repository:
```
cave resolve -x repository/rust
cave resolve -x gitoxide
```

### From Source via Cargo

`cargo` is the Rust package manager which can easily be obtained through [rustup]. With it, you can build your own binary
effortlessly and for your particular CPU for additional performance gains.

The minimum supported Rust version is [documented in the CI configuration](https://github.com/GitoxideLabs/gitoxide/blob/main/.github/workflows/msrv.yml#L23),
the latest stable one will work as well.

There are various build configurations, all of them are [documented here](https://docs.rs/crate/gitoxide/latest). The documentation should also be useful
for packagers who need to tune external dependencies.

```
# A way to install `gitoxide` with just Rust and a C compiler installed.
# If there are problems with SSL certificates during clones, try to omit `--locked`.
cargo install gitoxide --locked --no-default-features --features max-pure

# The default installation, 'max', is the fastest, but also needs `cmake` to build successfully.
# Installing it is platform-dependent.
cargo install gitoxide

# For smaller binaries and even faster build times that are traded for a less fancy CLI implementation,
# use the `lean` feature.
cargo install gitoxide --locked --no-default-features --features lean
```

The following installs the latest unpublished `max` release directly from git:

```sh
cargo install --git https://github.com/GitoxideLabs/gitoxide  gitoxide
```

#### How to deal with build failures

On some platforms, installation may fail due to lack of tools required by `C` toolchains. This can generally be avoided by installation
with `cargo install gitoxide --no-default-features --features max-pure`.

What follows is a list of known failures.

- On Fedora, `perl` needs to be installed for `OpenSSL` to build properly. This can be done with the following command:
  `dnf install perl` (see [this issue](https://github.com/GitoxideLabs/gitoxide/issues/592)).
-
### Using Docker

Some CI/CD pipelines leverage repository cloning. Below is a copy-paste-able example to build docker images for such workflows.
As no official image exists (at this time), an image must first be built.

> [!NOTE]
> The dockerfile isn't continuously tested as it costs too much time and thus might already be broken.
> PRs are welcome.

#### Building the most compatible base image

```sh
docker build -f etc/docker/Dockerfile.alpine -t gitoxide:latest --compress . --target=pipeline
```

#### Basic usage in a Pipeline

For example, if a `Dockerfile` currently uses something like `RUN git clone https://github.com/GitoxideLabs/gitoxide`, first build the image:

```sh
docker build -f etc/docker/Dockerfile.alpine -t gitoxide:latest --compress .
```

Then copy the binaries into your image and replace the `git` directive with a `gix` equivalent.

```dockerfile
COPY --from gitoxide:latest /bin/gix /usr/local/bin/
COPY --from gitoxide:latest /bin/ein /usr/local/bin/

RUN /usr/local/bin/gix clone --depth 1 https://github.com/GitoxideLabs/gitoxide gitoxide
```


[releases]: https://github.com/GitoxideLabs/gitoxide/releases
[rustup]: https://rustup.rs

## Usage

Once installed, there are two binaries:

* **ein**
  * high level commands, _porcelain_, for every-day use, optimized for a pleasant user experience
* **gix**
  * low level commands, _plumbing_, for use in more specialized cases and to validate newly written code in real-world scenarios

## Project Goals

Project goals can change over time as we learn more, and they can be challenged.

 * **a pure-rust implementation of git**
   * including *transport*, *object database*, *references*, *cli* and *tui*
   * a simple command-line interface is provided for the most common git operations, optimized for
     user experience. A *simple-git* if you so will.
   * be the go-to implementation for anyone who wants to solve problems around git, and become
     *the* alternative to `GitPython` and *libgit2* in the process.
   * become the foundation for a distributed alternative to GitHub, and maybe even for use within GitHub itself
 * **learn from the best to write the best possible idiomatic Rust**
   * *libgit2* is a fantastic resource to see what abstractions work, we will use them
   * use Rust's type system to make misuse impossible
 * **be the best performing implementation**
   * use Rust's type system to optimize for work not done without being hard to use
   * make use of parallelism from the get go
   * _sparse checkout_ support from day one
 * **assure on-disk consistency**
   * assure reads never interfere with concurrent writes
   * assure multiple concurrent writes don't cause trouble
 * **take shortcuts, but not in quality**
   * binaries may use `anyhow::Error` exhaustively, knowing these errors are solely user-facing.
   * libraries use light-weight custom errors implemented using `quick-error` or `thiserror`.
   * internationalization is nothing we are concerned with right now.
   * IO errors due to insufficient amount of open file handles don't always lead to operation failure
 * **Cross platform support, including Windows**
   * With the tools and experience available here there is no reason not to support Windows.
   * [Windows is tested on CI](https://github.com/GitoxideLabs/gitoxide/blob/df66d74aa2a8cb62d8a03383135f08c8e8c579a8/.github/workflows/rust.yml#L34)
     and failures do prevent releases.

## Non-Goals

Project non-goals can change over time as we learn more, and they can be challenged.

 * **replicate `git` command functionality perfectly**
   * `git` is `git`, and there is no reason to not use it. Our path is the one of simplicity to make
     getting started with git easy.
 * **be incompatible to git**
   * the on-disk format must remain compatible, and we will never contend with it.
 * **use async IO everywhere**
   * for the most part, git operations are heavily reliant on memory mapped IO as well as CPU to decompress data,
     which doesn't lend itself well to async IO out of the box.
   * Use `blocking` as well as `gix-features::interrupt` to bring operations into the async world and to control
     long running operations.
   * When connecting or streaming over TCP connections, especially when receiving on the server, async seems like a must
     though, but behind a feature flag.

## Contributions

If what you have seen so far sparked your interest to contribute, then let us say: We are happy to have you and help you to get started.

We recommend running `just test` during the development process to assure CI is green before pushing.

A backlog for work ready to be picked up is [available in the Project's Kanban board][project-board], which contains instructions on how
to pick a task. If it's empty or you have other questions, feel free to [start a discussion][discussions] or reach out to @Byron [privately][keybase].

For additional details, also take a look at the [collaboration guide].

[collaboration guide]: https://github.com/GitoxideLabs/gitoxide/blob/main/COLLABORATING.md
[project-board]: https://github.com/GitoxideLabs/gitoxide/projects
[discussions]: https://github.com/GitoxideLabs/gitoxide/discussions
[keybase]: https://keybase.io/byronbates
[cargo-diet]: https://crates.io/crates/cargo-diet

### Getting started with Video Tutorials

- [Learning Rust with Gitoxide](https://youtube.com/playlist?list=PLMHbQxe1e9Mk5kOHrm9v20-umkE2ck_gE)
   - In 17 episodes you can learn all you need to meaningfully contribute to `gitoxide`.
- [Getting into Gitoxide](https://youtube.com/playlist?list=PLMHbQxe1e9MkEmuj9csczEK1O06l0Npy5)
   - Get an introduction to `gitoxide` itself which should be a good foundation for any contribution, but isn't a requirement for contributions either.
- [Gifting Gitoxide](https://www.youtube.com/playlist?list=PLMHbQxe1e9MlhyyZQXPi_dc-bKudE-WUw)
   - See how PRs are reviewed along with a lot of inner monologue.

#### Other Media

- [Rustacean Station Podcast](https://rustacean-station.org/episode/055-sebastian-thiel/)

## Roadmap

### Features for 1.0

Provide a CLI to for the most basic user journey:

* [x] initialize a repository
* [x] fetch
    * [ ] and update worktree
* clone a repository
   - [ ] bare
   - [ ] with working tree
* [ ] create a commit after adding worktree files
* [x] add a remote
* [ ] push
  * [x] create (thin) pack

### Ideas for Examples

* [ ] `gix tool open-remote` open the URL of the remote, possibly after applying known transformations to go from `ssh` to `https`.
* [ ] `tix` as example implementation of `tig`, displaying a version of the commit graph, useful for practicing how highly responsive GUIs can be made.
* [ ] Something like [`git-sizer`](https://github.com/github/git-sizer), but leveraging extreme decompression speeds of indexed packs.
* [ ] Open up SQL for git using [sqlite virtual tables](https://github.com/rusqlite/rusqlite/blob/master/tests/vtab.rs). Check out gitqlite
  as well. What would an MVP look like? Maybe even something that could ship with gitoxide. See [this go implementation as example](https://github.com/filhodanuvem/gitql).
* [ ] A truly awesome history rewriter which makes it easy to understand what happened while avoiding all pitfalls. Think BFG, but more awesome, if that's possible.
* [ ] `gix-tui` should learn a lot from [fossil-scm] regarding the presentation of data. Maybe [this](https://github.com/Lutetium-Vanadium/requestty/) can be used for prompts. Probably [magit] has a lot to offer, too.

### Ideas for Spin-Offs

* [ ] A system to integrate tightly with `gix-lfs` to allow a multi-tier architecture so that assets can be stored in git and are accessible quickly from an intranet location
  (for example by accessing the storage read-only over the network) while changes are pushed immediately by the server to other edge locations, like _the cloud_ or backups. Sparse checkouts along with explorer/finder integrations
  make it convenient to only work on a small subset of files locally. Clones can contain all configuration somebody would need to work efficiently from their location,
  and authentication for the git history as well as LFS resources make the system secure. One could imagine encryption support for untrusted locations in _the cloud_
  even though more research would have to be done to make it truly secure.
* [ ] A [syncthing] like client/server application. This is to demonstrate how lower-level crates can be combined into custom applications that use
  only part of git's technology to achieve their very own thing. Watch out for big file support, multi-device cross-syncing, the possibility for
  untrusted destinations using full-encryption, case-insensitive and sensitive filesystems, and extended file attributes as well as ignore files.
* An event-based database that uses commit messages to store deltas, while occasionally aggregating the actual state in a tree. Of course it's distributed by nature, allowing
  people to work offline.
    - It's abstracted to completely hide the actual data model behind it, allowing for all kinds of things to be implemented on top.
    - Commits probably need a nanosecond component for the timestamp, which can be added via custom header field.
    - having recording all changes allows for perfect merging, both on the client or on the server, while keeping a natural audit log which makes it useful for mission critical
      databases in business.
    * **Applications**
      - Can markdown be used as database so issue-trackers along with meta-data could just be markdown files which are mostly human-editable? Could user interfaces
        be meta-data aware and just hide the meta-data chunks which are now editable in the GUI itself? Doing this would make conflicts easier to resolve than an `sqlite`
        database.
      - A time tracker - simple data, very likely naturally conflict free, and interesting to see it in terms of teams or companies using it with maybe GitHub as Backing for authentication.
        - How about supporting multiple different trackers, as in different remotes?

[syncthing]: https://github.com/syncthing/syncthing
[fossil-scm]: https://www.fossil-scm.org
[magit]: https://magit.vc

## Shortcomings & Limitations

Please take a look at the [`SHORTCOMINGS.md` file](https://github.com/GitoxideLabs/gitoxide/blob/main/SHORTCOMINGS.md) for details.

## Credits

* **itertools** _(MIT Licensed)_
  * We use the `izip!` macro in code
* **deflate2** _(MIT Licensed)_
  * We use various abstractions to implement decompression and compression directly on top of the rather low-level `miniz_oxide` crate

## 🙏 Special Thanks 🙏

At least for now this section is exclusive to highlight the incredible support that [Josh Triplett](https://github.com/joshtriplett) has provided to me
in the form of advice, sponsorship and countless other benefits that were incredibly meaningful. Going full time with `gitoxide` would hardly have been
feasible without his involvement, and I couldn't be more grateful 😌.

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   http://opensource.org/licenses/MIT)

at your option.

## Fun facts

* Originally @Byron was really fascinated by [this problem](https://github.com/gitpython-developers/GitPython/issues/765#issuecomment-396072153)
  and believes that with `gitoxide` it will be possible to provide the fastest solution for it.
* @Byron has been absolutely blown away by `git` from the first time he experienced git more than 13 years ago, and
  tried to implement it in [various shapes](https://github.com/gitpython-developers/GitPython/pull/1028) and [forms](https://github.com/byron/gogit)
  multiple [times](https://github.com/Byron/gitplusplus). Now with Rust @Byron finally feels to have found the right tool for the job!
