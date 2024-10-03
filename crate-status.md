### gitoxide

**gix** is a command-line interface (*CLI*) to access git repositories in various ways best described as low-level
for use by experts or those validating functionality in real-world scenarios. Performance and efficiency are staples
of the implementation.

**ein** is reserved for one-off tools that are useful to many, and will one day implement a truly unique workflow
with the potential to become the preferred way to interact with git repositories.

Please note that all functionality comes from the `gitoxide-core` library, which mirrors these capabilities
and itself relies on all `gix-*` crates. It's not meant for consumption, for application development, please use `gix`.

* **the `ein` program** - convenient and for humans
    * [x] **init** - initialize a new non-bare repository with a `main` branch
    * [ ] **clone** - initialize a local copy of a remote repository
    * **tools**
        * [x] **organize** - find all git repositories and place them in directories according to their remote paths
        * [x] **find** - find all git repositories in a given directory - useful for tools like [skim][skim]
        * [x] **estimate-hours** - estimate the time invested into a repository by evaluating commit dates.
            * Based on the [git-hours] algorithm.
            * See the [discussion][git-hours-discussion] for some performance data.
* **the `gix` program** _(plumbing)_ - lower level commands for use during development
    - As its main purpose is to help running the latest improvements in the real world, it's self-documenting without
      duplicating its features here. Use `gix --help` to start discovery.

[skim]: https://github.com/lotabout/skim
[git-hours]: https://github.com/kimmobrunfeldt/git-hours/blob/8aaeee237cb9d9028e7a2592a25ad8468b1f45e4/index.js#L114-L143
[git-hours-discussion]: https://github.com/GitoxideLabs/gitoxide/discussions/78

### gix

The top-level crate that acts as hub to all functionality provided by the `gix-*` plumbing crates.

* [x] utilities for applications to make long running operations interruptible gracefully and to support timeouts in servers.
* [x] handle `core.repositoryFormatVersion` and extensions
* [x] support for unicode-precomposition of command-line arguments (needs explicit use in parent application)
* [ ] strict object creation (validate objects referenced by newly created objects exist)
* [ ] strict hash verification (validate that objects actually have the hashes they claim to have)
* **Repository**
    * [x] discovery
        * [x] option to not cross file systems (default)
        * [x] handle git-common-dir
        * [x] support for `GIT_CEILING_DIRECTORIES` environment variable
        * [ ] handle other non-discovery modes and provide control over environment variable usage required in applications
    * [x] rev-parse
       - [ ] handle relative paths as relative to working directory
       - [x] handle `upstream` and `push` resolution.
    * [x] rev-walk
        * [x] include tips
        * [ ] exclude commits
    * [x] instantiation
    * [x] access to refs and objects
    * [x] create a pathspec-search from a set of strings
        - [ ] allow to construct Pathspecs using data structure instead of enforcing them to be passed as strings.
    * **credentials**
        * [x] run `git credential` directly
        * [x] use credential helper configuration and to obtain credentials with `gix_credentials::helper::Cascade`
    * **traverse**
        * [x] commit graphs
        * [ ] make [git-notes](https://git-scm.com/docs/git-notes) accessible
        * [x] tree entries
    * **diffs/changes**
        * [x] tree with other tree
            * [x] respect case-sensitivity of host filesystem.
            * [x] a way to access various diff related settings or use them
            * [x] respect `diff.*.textconv`, `diff.*.cachetextconv` and external diff viewers with `diff.*.command`,
              [along with support for reading `diff` gitattributes](https://github.com/git/git/blob/73876f4861cd3d187a4682290ab75c9dccadbc56/Documentation/gitattributes.txt#L699:L699).
            * **rewrite tracking**
                * **deviation** - git keeps up to four candidates whereas we use the first-found candidate that matches the similarity percentage.
                  This can lead to different sources being found. As such, we also don't consider the filename at all.
                * [x] handle binary files correctly, and apply filters for that matter
                * [x] computation limit with observable reduction of precision when it is hit, for copies and renames separately
                * **by identity**
                    * [x] renames (sym-links are only ever compared by identity)
                    * [x] copies
                * **by similarity** - similarity factor controllable separately from renames
                    * [x] renames
                    * [x] copies
                * [x] 'find-copies-harder' - find copies with the source being the entire tree.
        * [x] tree or index with working tree
             - [x] rename tracking
             - [x] submodule status (recursive)
        * [x] diffs between modified blobs with various algorithms
        * [ ] tree with index (via index-from-tree and index)
            - [ ] rename tracking
            - [ ] submodule status (recursive)
    * [x] initialize
        * [x] Proper configuration depending on platform (e.g. ignorecase, filemode, …)
    * **Id**
        * [x] short hashes with detection of ambiguity.
    * **Commit**
        * [x] `git describe` like functionality, with optional commit-graph acceleration
        * [x] create new commit from tree
    * **Objects**
        * [x] lookup
        * [x] peel to object kind
        * [ ] create [signed commits and tags](https://github.com/GitoxideLabs/gitoxide/issues/12)
        * **trees**
            * [x] lookup path
            * [x] edit
    * **references**
        * [x] peel to end
        * [x] ref-log access
        * [x] remote name
        * [x] find remote itself
            - [ ] respect `branch.<name>.merge` in the returned remote.
    * **remotes**
        * [x] clone
            * [x] shallow
                * [ ] include-tags when shallow is used (needs separate fetch)
                * [ ] prune non-existing shallow commits
            * [ ] [bundles](https://git-scm.com/docs/git-bundle)
        * [x] fetch
            * [x] shallow (remains shallow, options to adjust shallow boundary)
            * [ ] a way to auto-explode small packs to avoid them to pile up
            * [x] 'ref-in-want'
            * [ ] 'wanted-ref'
            * [x] standard negotiation algorithms `consecutive`, `skipping` and `noop`.
        * [ ] push
        * [x] ls-refs
        * [x] ls-refs with ref-spec filter
        * [x] list, find by name
        * [x] create in memory
        * [ ] groups
        * [ ] [remote and branch files](https://github.com/git/git/blob/master/remote.c#L300)
    * [ ] execute hooks
    * **refs**
        * [ ] run transaction hooks and handle special repository states like quarantine
        * [ ] support for different backends like `files` and `reftable`
    * **main or linked worktree**
        * [ ] add files with `.gitignore` handling
        * [ ] checkout with conversions like clean + smudge as in `.gitattributes`
        * [ ] _diff_ index with working tree
        * [ ] sparse checkout support
        * [x] read per-worktree config if `extensions.worktreeConfig` is enabled.
        * **index**
            * [ ] tree from index
            * [x] index from tree
    * **worktrees**
        * [x] open a repository with worktrees
            * [x] read locked state
            * [ ] obtain 'prunable' information
        * [x] proper handling of worktree related refs
        * [x] create a byte stream and create archives for such a stream, including worktree filters and conversions
        * [ ] create, move, remove, and repair
        * [x] access exclude information
        * [x] access attribute information
        * [x] respect `core.worktree` configuration
            - **deviation**
                * The delicate interplay between `GIT_COMMON_DIR` and `GIT_WORK_TREE` isn't implemented.
    * **config**
        * [x] read the primitive types `boolean`, `integer`, `string`
        * [x] read and interpolate trusted paths
        * [x] low-level API for more elaborate access to all details of `git-config` files
        * [ ] a way to make changes to individual configuration files in memory
        * [ ] write configuration back
        * [ ] auto-refresh configuration values after they changed on disk
        * [ ] facilities to apply the [url-match](https://git-scm.com/docs/git-config#Documentation/git-config.txt-httplturlgt) algorithm and to [normalize urls](https://github.com/git/git/blob/be1a02a17ede4082a86dfbfee0f54f345e8b43ac/urlmatch.c#L109:L109) before comparison.
    * [x] mailmap
    * [x] object replacements (`git replace`)
    * [x] read git configuration
    * [ ] merging
    * [ ] stashing
    * [ ] Use _Commit Graph_ to speed up certain queries
    * [ ] subtree
    * [ ] interactive rebase status/manipulation
    * **submodules**
        * [x] handle 'old' form for reading and detect old form
        * [x] list
        * [ ] edit
* [ ] API documentation
    * [ ] Some examples

### gix-actor
* [x] read and write a signature that uniquely identifies an actor within a git repository
* [x] a way to parse `name <email>` tuples (instead of full signatures) to facilitate parsing
      commit trailers.
* [x] a way to write only actors, useful for commit trailers.

### gix-hash
* types to represent hash digests to identify git objects.
* used to abstract over different kinds of hashes, like SHA1 and the upcoming SHA256
* [x] API documentation
    * [ ] Some examples

### gix-chunk
* [x] decode the chunk file table of contents and provide convenient API
* [x] write the table of contents

### gix-hashtable
* [x] hashmap
* [x] hashset

### gix-utils
* **filesystem**
   * [x] probe capabilities
   * [x] symlink creation and removal
   * [x] file snapshots
* [ ] **BString Interner with Arena-Backing and arbitrary value association**
    - probably based on [`internment`](https://docs.rs/internment/latest/internment/struct.Arena.html#),
      but needs `bumpalo` support to avoid item allocations/boxing, and avoid internal `Mutex`. (key type is pointer based).

### gix-fs
* [x] probe capabilities
* [x] symlink creation and removal
* [x] file snapshots
* [x] stack abstraction

### gix-object
* *decode (zero-copy)* borrowed objects
    * [x] commit
      * [ ] parse [trailers](https://git-scm.com/docs/git-interpret-trailers#_description)
    * [x] tree
* encode owned objects
    * [x] commit
    * [x] tree
    * [x] tag
      * [x] [name validation][tagname-validation]
* [x] transform borrowed to owned objects
* [x] edit trees efficiently and write changes back
    - [ ] See if `gix-fs::InternedMap` improves performance.
* [x] API documentation
    * [ ] Some examples

### gix-pack
* **packs**
    * [x] traverse pack index
    * [x] 'object' abstraction
        * [x] decode (zero copy)
        * [x] verify checksum
    * [x] simple and fast pack traversal
    * [x] decode
        * [x] full objects
        * [x] deltified objects
    * **decode**
        * _decode a pack from `Read` input_
            * [x] Add support for zlib-ng for 20% faster _decompression_ performance
            * [x] `Read` to `Iterator` of entries
                * _read as is, verify hash, and restore partial packs_
        * [x] create index from pack alone (_much faster than git_)
            * [x] resolve 'thin' packs
    * **encode**
        * [x] Add support for zlib-ng for 2.5x _compression_ performance
        * [x] objects to entries iterator
            * [x] input objects as-is
            * [x] pack only changed objects as derived from input
            * [x] base object compression
            * [ ] delta compression
               * [ ] respect the `delta=false` attribute
            * [x] create 'thin' pack, i.e. deltas that are based on objects the other side has.
            * [x] parallel implementation that scales perfectly
        * [x] entries to pack data iterator
        * [ ] write index along with the new pack
    * [x] **verify** pack with statistics
        * [x] brute force - less memory
        * [x] indexed - optimal speed, but more memory
    * **advanced**
        * [x] Multi-Pack index file (MIDX)
            * [x] read
            * [x] write
            * [x] verify
        * [ ] 'bitmap' file
        * [ ] [special handling for networked packs](https://github.com/git/git/blob/89b43f80a514aee58b662ad606e6352e03eaeee4/packfile.c#L949:L949)
        * [ ] [detect and retry packed object reading](https://github.com/git/git/blob/89b43f80a514aee58b662ad606e6352e03eaeee4/packfile.c#L1268:L1268)
* [x] API documentation
    * [ ] Some examples

### gix-odb
* **loose object store**
    * [x] traverse
    * [x] read
        * [x] into memory
        * [x] streaming
        * [x] verify checksum
    * [x] streaming write for blobs
    * [x] buffer write for small in-memory objects/non-blobs to bring IO down to open-read-close == 3 syscalls
    * [ ] read object header (size + kind) without full decompression
* **dynamic store**
    * [x] auto-refresh of on-disk state
    * [x] handles alternates
    * [x] multi-pack indices
    * [x] perfect scaling with cores
    * [x] support for pack caches, object caches and MRU for best per-thread performance.
    * [x] prefix/short-id lookup, with optional listing of ambiguous objects.
    * [x] object replacements (`git replace`)
    * [x] high-speed packed object traversal without wasted CPU time
      - [ ] user defined filters
    * [ ] read object header (size + kind) without full decompression
* **sink**
    * [x] write objects and obtain id
* **alternates**
    * _resolve links between object databases_
    * [x] safe with cycles and recursive configurations
    * [x] multi-line with comments and quotes
* **promisor**
    * It's vague, but these seems to be like index files allowing to fetch objects from a server on demand.
* [x] API documentation
    * [ ] Some examples

### gix-diff

Check out the [performance discussion][gix-diff-performance] as well.

* **tree**
    * [x] changes needed to obtain _other tree_
* **blobs**
    * **patches**
        * There are various ways to generate a patch from two blobs.
        * [ ] text
        * [ ] binary
        * [ ] `git-apply` compatibility
        * [ ] merge hunks that are close enough based on line-setting (`interhunk-lines`)
        * [ ] white-space related settings
    * **lines**
        * [x] Simple line-by-line diffs powered by the `imara-diff` crate.
* **generic rename tracker to find renames and copies**
    * [x] find blobs by exact match
    * [x] find blobs by similarity check
    * [ ] heuristics to find best candidate
    * [ ] find by basename to support similarity check
    * [x] directory tracking
        - [x] by identity
        - [ ] by similarity
* **blob**
    * [x] a choice of to-worktree, to-git and to-worktree-if-needed conversions
    * [x] `textconv` filters
    * [x] special handling of files beyond the big-file threshold.
    * [x] detection of binary files by looking at header (first 8k bytes)
    * [x] caching of diff-able data
    * [x] prepare invocation of external diff program
        - [ ] pass meta-info
* [ ] working with hunks of data
* [ ] diff-heuristics match Git perfectly
* [x] API documentation
    * [ ] Examples

[gix-diff-performance]: https://github.com/GitoxideLabs/gitoxide/discussions/74

### gix-merge

* [x] three-way merge analysis of **blobs** with choice of how to resolve conflicts
    - [ ] choose how to resolve conflicts on the data-structure
    - [ ] produce a new blob based on data-structure containing possible resolutions
        - [x] `merge` style
        - [x] `diff3` style
        - [x] `zdiff` style
    - [ ] a way to control inter-hunk merging based on proximity (maybe via `gix-diff` feature which could use the same)
* [ ] diff-heuristics match Git perfectly
* [x] API documentation
    * [ ] Examples

### gix-blame

* [ ] commit-annotations for a single file
    - [ ] progress
    - [ ] interruptability
    - [ ] streaming
* [x] API documentation
    * [ ] Examples

### gix-traverse

Check out the [performance discussion][gix-traverse-performance] as well.

* **trees**
  * [x] nested traversal
* **commits**
  * [x] ancestor graph traversal similar to `git revlog`
  * [ ] `commitgraph` support
* [x] API documentation
    * [ ] Examples

[gix-traverse-performance]: https://github.com/GitoxideLabs/gitoxide/discussions/76

### gix-url
* As documented here: https://www.git-scm.com/docs/git-clone#_git_urls
* **parse**
    * [x] ssh URLs and SCP like syntax
    * [x] file, git, and SSH
    * [x] paths (OS paths, without need for UTF-8)
* [x] username expansion for ssh and git urls
* [x] convert URL to string
* [x] API documentation
    * [ ] Some examples

### gix-packetline
* [PKT-Line](https://github.com/git/git/blob/master/Documentation/technical/protocol-common.txt#L52:L52)
* [x] encode
* [x] decode (zero-copy)
* [x] [error line](https://github.com/git/git/blob/master/Documentation/technical/pack-protocol.txt#L28:L28)
* [x] [V2 additions](https://github.com/git/git/blob/master/Documentation/technical/protocol-v2.txt#L35:L36)
* [x] [side-band mode](https://github.com/git/git/blob/master/Documentation/technical/pack-protocol.txt#L467:L467)
* [x] `Read` from packet line with (optional) progress support via sidebands
* [x] `Write` with built-in packet line encoding
* [x] `async` support
* [x] API documentation
    * [ ] Some examples

### gix-transport
* No matter what we do here, timeouts must be supported to prevent hanging forever and to make interrupts destructor-safe.
* **client**
    * [x] general purpose `connect(…)` for clients
        * [x] _file://_ launches service application
        * [x] _ssh://_ launches service application in a remote shell using _ssh_
        * [x] _git://_ establishes a tcp connection to a git daemon
        * [x] _http(s)://_ establishes connections to web server
            * [x] via `curl` (blocking only)
            * [x] via `reqwest` (blocking only)
        * [ ] pass context for scheme specific configuration, like timeouts
    * [x] git://<service>
        * [x] V1 handshake
            * [x] send values + receive data with sidebands
            * [ ] ~~support for receiving 'shallow' refs in case the remote repository is shallow itself (I presume)~~
                * Since V2 doesn't seem to support that, let's skip this until there is an actual need. No completionist :D
        * [x] V2 handshake
            * [x] send command request, receive response with sideband support
    * [x] http(s)://<service>
        * [x] set identity for basic authentication
        * [x] V1 handshake
            * [x] send values + receive data with sidebands
        * [x] V2 handshake
            * [x] send command request, receive response with sideband support
        * [ ] ~~'dumb'~~ - _we opt out using this protocol seems too slow to be useful, unless it downloads entire packs for clones?_
    * [x] authentication failures are communicated by io::ErrorKind::PermissionDenied, allowing other layers to retry with authentication
    * [x] `async` support
* **server**
    * [ ] general purpose `accept(…)` for servers
* [x] API documentation
    * [ ] Some examples

#### Advanced HTTP transport features

| **feature** | **curl** | **reqwest** |
|-------------|----------|-------------|
|      01     |          |             |
|      02     |     X    |             |
|      03     |          |      X      |
|      04     |     X    |             |
|      05     |          |             |

* **01** -> async
* **02** -> proxy support
* **03** -> custom request configuration via fn(request)
* **04** -> proxy authentication
* **05** -> [reauthentication after redirect](https://github.com/git/git/blob/eea7033409a0ed713c78437fc76486983d211e25/http.c#L1931)

### gix-protocol
* _abstract over protocol versions to allow delegates to deal only with a single way of doing things_
* [x] **credentials**
    * [x] via gix-credentials
    * [ ] via pure Rust implementation if no git is installed
* [x] handshake
    * parse initial response of V1 and V2 servers
* [x] ls-refs
    * [x] parse V1 refs as provided during handshake
    * [x] parse V2 refs
    * [ ] handle empty refs, AKA PKT-LINE(zero-id SP "capabilities^{}" NUL capability-list)
* [x] fetch
    * [x] detailed progress
    * [x] control credentials provider to fill, approve and reject
    * [x] initialize and validate command arguments and features sanely
    * [x] abort early for ls-remote capabilities
    * [x] packfile negotiation
        * [x] delegate can support for all fetch features, including shallow, deepen, etc.
        * [x] receive parsed shallow refs
* [ ] push
* [x] API documentation
    * [ ] Some examples

### gix-attributes
* [x] parse `.gitattribute` files
* [ ] an attributes stack for matching paths to their attributes, with support for built-in `binary` macro for `-text -diff -merge`

### gix-ignore
* [x] parse `.gitignore` files
* [x] an attributes stack for checking if paths are excluded

### gix-quote
* **ansi-c**
  * [x] quote
  * [ ] unquote

### gix-mailmap
* [x] parsing
* [x] lookup and mapping of author names

### gix-path
* [x] transformations to and from bytes
* [x] conversions between different platforms
* [x] virtual canonicalization for more concise paths via `absolutize()`
* [x] more flexible canonicalization with symlink resolution for paths which are partially virtual via `realpath()`
* **spec**
    * [ ] parse
    * [ ] check for match

### gix-pathspec
* [x] parse single
* [ ] parse file line by line (with or without quoting, NUL and LF/CRLF line separation) (see `--pathspec-from-file` and `--pathspec-file-nul`)
* [x] matching of paths with git-attributes support
* [ ] programmatic creation of pathspecs
* [ ] `TryIntoPathspec` trait to parse strings or accept ready-made pathspecs as well, for use in APIs

### gix-refspec
* [x] parse
* [x] matching of references and object names
    * [x] for fetch
    * [ ] for push

### gix-command
* [x] execute commands directly
* [x] execute commands with `sh`
* [ ] support for `GIT_EXEC_PATH` environment variable with `gix-sec` filter

### gix-prompt
* [x] open prompts for usernames for example
* [x] secure prompts for password
* [x] use `askpass` program if available
* [ ] signal handling (resetting and restoring terminal settings)
* [ ] windows prompts for `cmd.exe` and mingw terminals

### gix-note

A mechanism to associate metadata with any object, and keep revisions of it using git itself.

* [ ] CRUD for git notes

### gix-negotiate
* **algorithms**
  - [x] `noop`
  - [x] `consecutive`
  - [x] `skipping`

### gix-fetchhead
* [ ] parse `FETCH_HEAD` information back entirely
* [ ] write typical fetch-head lines

### gix-discover

* [x] check if a git directory is a git repository
* [x] find a git repository by searching upward
   * [x] define ceilings that should not be surpassed
   * [x] prevent crossing file-systems (non-windows only)
* [x] handle linked worktrees
* [ ] a way to handle `safe.directory`
     - note that it's less critical to support it as `gitoxide` allows access but prevents untrusted configuration to become effective.

### gix-date
* [ ] parse git dates
* [ ] serialize `Time`

### gix-credentials
* [x] launch git credentials helpers with a given action
  - [x] built-in `git credential` program
  - [x] as scripts
  - [x] as absolute paths to programs with optional arguments
  - [x] program name with optional arguments, transformed into `git credential-<name>`
* [x] `helper::main()` for easy custom credential helper programs written in Rust

### gix-filter

Provide base-implementations for dealing with smudge and clean filters as well as filter processes, facilitating their development.

* [ ] clean filter base
* [ ] smudge filter base
* [ ] filter process base

### gix-sec

Provides a trust model to share across gitoxide crates. It helps configuring how to interact with external processes, among other things.

* **integrations**
   * [x] gix-config
   * [x] gix

### gix-rebase
* [ ] obtain rebase status
* [ ] drive a rebase operation

### gix-sequencer

Handle human-aided operations which cannot be completed in one command invocation.

### gix-lfs

Implement git large file support using the process protocol and make it flexible enough to handle a variety of cases.
Make it the best-performing implementation and the most convenient one.

### gix-glob
* [x] parse pattern
* [x] a type for pattern matching of paths and non-paths, optionally case-insensitively.

### gix-status
* [x] differences between index and worktree to turn index into worktree
    - [x] rename tracking
    - [x] untracked files
    - [ ] support for fs-monitor for modification checks
* [ ] differences between index and index to learn what changed
    - [ ] rename tracking

### gix-worktree-state
* handle the working **tree/checkout**
    - [x] checkout an index of files, executables and symlinks just as fast as git
        - [x] forbid symlinks in directories
        - [ ] handle submodules
        - [ ] handle sparse directories
        - [ ] handle sparse index
        - [x] linear scaling with multi-threading up to IO saturation
    - supported attributes to affect working tree and index contents
        - [x] eol
        - [x] working-tree-encoding
        - …more
    - **filtering**
        - [x] `text`
        - [x] `ident`
        - [x] filter processes
        - [x] single-invocation clean/smudge filters
* access to per-path information, like `.gitignore` and `.gitattributes` in a manner well suited for efficient lookups
    * [x] _exclude_ information
    * [x] attributes

### gix-worktree
* [x] A stack to to efficiently generate attribute lists for matching paths against.

### gix-revision
* [x] `describe()` (similar to `git name-rev`)
* parse specifications
    * [x] parsing and navigation
    * [x] revision ranges
    * [ ] full date parsing support (depends on `gix-date`)

### gix-revision
* [x] primitives to help with graph traversal, along with commit-graph acceleration.

### gix-submodule
* [x] read `.gitmodule` files, access all their fields, and apply overrides
* [x] check if a submodule is 'active'
* [ ] CRUD for submodules
* [ ] try to handle with all the nifty interactions and be a little more comfortable than what git offers, lay a foundation for smarter git submodules.

### gix-bitmap

A plumbing crate with shared functionality regarding EWAH compressed bitmaps, as well as other kinds of bitmap implementations.

* **EWAH**
  * `Array` type to read and write bits
     * [x] execute closure for each `true` bit
  * [x] decode on-disk representation
  * [ ] encode on-disk representation

### gix-dir

A git directory walk.

* [x] list untracked files
* [x] list ignored files
* [x] collapsing of untracked and ignored directories
* [x] pathspec based filtering
* [ ] multi-threaded initialization of icase hash table is always used to accelerate index lookups, even if ignoreCase = false for performance
* [ ] special handling of submodules (for now, submodules or nested repositories are detected, but they can't be walked into naturally)
* [ ] accelerated walk with `untracked`-cache (as provided by `UNTR` extension of `gix_index::File`)

### gix-index

The git staging area.

* read
  * [x] V2 - the default, including long-paths support
  * [x] V3 - extended flags
  * [x] V4 - delta-compression for paths
  * [ ] TODO(perf): multi-threaded implementation should boost performance, spends most time in storing paths, has barely any benefit right now.
  * optional threading
    * [x] concurrent loading of index extensions
    * [x] threaded entry reading
  * extensions
    * [x] TREE for speeding up tree generation
    * [x] REUC resolving undo
    * [x] UNTR untracked cache
    * [x] FSMN file system monitor cache V1 and V2
    * [x] EOIE end of index entry
    * [x] IEOT index entry offset table
    * [x] 'link' base indices to take information from, split index
    * [x] 'sdir' [sparse directory entries](https://github.blog/2021-08-16-highlights-from-git-2-33/) - marker
  * [x] verification of entries and extensions as well as checksum
  * [ ] expand sparse directory entries using information of the tree itself
* write
  * [x] V2
  * [x] V3 - extension bits
  * [ ] V4
  * extensions
      * [x] TREE
      * [ ] REUC
      * [ ] UNTR
      * [ ] FSMN
      * [x] EOIE
      * [x] 'sdir'
      * [ ] 'link'
          - **note** that we currently **dissolve** any shared index we read so when writing this extension is removed.
* `stat` update
    * [ ] optional threaded `stat` based on thread_cost (aka preload)
* [x] handling of `.gitignore` and system file exclude configuration
* [x] lookups that ignore the case
    * [ ] multi-threaded lookup table generation with the same algorithm as the one used by Git
    * [ ] expand sparse folders (don't know how this relates to traversals right now)
* maintain extensions when altering the cache
    * [ ] TREE for speeding up tree generation
    * [ ] REUC resolving undo
    * [ ] UNTR untracked cache
    * [ ] FSMN file system monitor cache V1 and V2
    * [ ] EOIE end of index entry
    * [ ] IEOT index entry offset table
    * [ ] 'link' base indices to take information from, split index
    * [ ] 'sdir' sparse directory entries
* [ ] add and remove entries
* [x] API documentation
    * [ ] Some examples

### gix-commitgraph

* [x] read-only access
    * [x] Graph lookup of commit information to obtain timestamps, generation and parents, and extra edges
    * [ ] [Corrected generation dates](https://github.com/git/git/commit/e8b63005c48696a26f976f5f9b0ccaf1983e439d)
    * [ ] Bloom filter index
    * [ ] Bloom filter data
* [ ] create and update graphs and graph files
* [x] API documentation
    * [ ] Some examples

### gix-tempfile

See its [README.md](https://github.com/GitoxideLabs/gitoxide/blob/main/gix-tempfile/README.md).

### gix-lock

See its [README.md](https://github.com/GitoxideLabs/gitoxide/blob/main/gix-lock/README.md).

### gix-config-value
* **parse**
    * [x] boolean
    * [x] integer
    * [x] color
       * [ ] ANSI code output for terminal colors
    * [x] path (incl. resolution)
    * [ ] date
    * [ ] [permission][https://github.com/git/git/blob/71a8fab31b70c417e8f5b5f716581f89955a7082/setup.c#L1526:L1526]

### gix-config
* [x] read
    * zero-copy parsing with event emission
    * all config values as per the `gix-config-value` crate
    * **includeIf**
      * [x] `gitdir`,  `gitdir/i`, and `onbranch`
      * [ ] `hasconfig`
* [x] access values and sections by name and sub-section
* [x] edit configuration in memory, non-destructively
    * cross-platform newline handling
* [x] write files back for lossless round-trips.
    * keep comments and whitespace, and only change lines that are affected by actual changes, to allow truly non-destructive editing
* [x] cascaded loading of various configuration files into one
    * [x] load from environment variables
    * [x] load from well-known sources for global configuration
    * [x] load repository configuration with all known sources
* [x] API documentation
    * [x] Some examples

### gix-worktree-stream

* [x] encode git-tree as stream of bytes (with large file support and actual streaming)
* [x] produce a stream of entries
* [x] add custom entries to the stream
* [x] respect `export-ignore` git attribute
* [x] apply standard worktree conversion to simulate an actual checkout
* [ ] support for submodule inclusion
* [x] API documentation
    * [ ] Some examples

### gix-archive

* [x] `write_to()` for creating an archive with various container formats
    * [x] `tar` and `tar.gz`
    * [x] `zip`
* [x] add prefix and modification date
* [ ] API documentation
    * [ ] Some examples

### gix-bundle
* [ ] create a bundle from an archive
   * [ ] respect `export-ignore` and `export-subst`
* [ ] extract a branch from a bundle into a repository
* [ ] API documentation
    * [ ] Some examples

### gix-validate
* [x] validate ref names
* [x] validate submodule names
* [x] [validate][tagname-validation] tag names

### gix-fsck
* [x] validate connectivity and find missing objects starting from…
    - [x] commits
    - [ ] tags
    - [ ] tree-cache in the `index` or any entry within
* [ ] validate object hashes during connectivity traversal
* [ ] progress reporting and interruptability
* [ ] skipList to exclude objects which are known to be broken
* [ ] validate blob hashes (connectivity check
* [ ] identify objects that exist but are not reachable (i.e. what remains after a full graph traversal from all valid starting points)
* [ ] write dangling objects to the `.git/log-found` directory structure
* [ ] `strict` mode, to check for tree objects with `g+w` permissions
* [ ] consider reflog entries from `ref` starting points
* [ ] when reporting reachable objects, provide the path through which they are reachable, i.e. ref-log@{3} -> commit -> tree -> path-in-tree
* [ ] limit search to ODB without alternates (default is equivalent to `git fsck --full` due to ODB implementation)
* [ ] all individual [checks available in `git fsck`](https://git-scm.com/docs/git-fsck#_fsck_messages) (*too many to print here*)

### gix-ref
* [ ] Prepare code for arrival of longer hashes like Sha256. It's part of the [V2 proposal][reftable-v2] but should work for loose refs as well.
* **Stores**
  * [ ] disable transactions during [quarantine]
  * [x] namespaces
    * a server-side feature to transparently isolate refs in a single shared repository, allowing all forks to live in the same condensed repository.
  * **loose file**
    * [x] ref validation
    * [x] find single ref by name
    * [ ] special handling of `FETCH_HEAD` and `MERGE_HEAD`
    * [x] iterate refs with optional prefix
    * **worktree support**
        * [x] support multiple bases and classify refs
        * [x] support for ref iteration merging common and private refs seamlessly.
        * [x] avoid packing refs which are worktree private
    * ~~symbolic ref support, using symbolic links~~
        * This is a legacy feature which is not in use anymore.
    * **transactions**
      * [x] delete, create or update single ref or multiple refs while handling the _reflog_
      * [x] set any valid ref value (not just object ids)
      * [x] reflog changes can be entirely disabled (i.e. for bare repos)
      * [ ] rename or copy references
      * [x] transparent handling of packed-refs during deletion
      * [x] writing loose refs into packed-refs and optionally delete them
      * [ ] initial transaction optimization (a faster way to create clones with a lot of refs)
    * **log**
      * [x] forward iteration
      * [x] backward iteration
      * [ ] expire
    * **ref**
      * [x] peel to id
    * **packed**
      * [x] find single ref by name
      * [x] iterate refs with optional prefix
      * [x] handle unsorted packed refs and those without a header
  * [ ] **[reftable][reftable-spec]**,
    * see [here for a Go/C implementation][reftable-impl]
* [x] API documentation
    * [ ] Some examples

[reftable-spec]: https://github.com/eclipse/jgit/blob/master/Documentation/technical/reftable.md
[reftable-impl]: https://github.com/google/reftable
[reftable-v2]: https://github.com/google/reftable/blob/master/reftable-v2-proposal.md
[quarantine]: https://github.com/git/git/blob/master/Documentation/git-receive-pack.txt#L223:L223


### gix-features
* **io-pipe** feature toggle
    * a unix like pipeline for bytes
* **parallel** feature toggle
    * _When on…_
        * `in_parallel`
        * `join`
    * _When off all functions execute serially_
* **fast-sha1**
    * provides a faster SHA1 implementation using CPU intrinsics
* [x] API documentation

### gix-tui
* _a terminal user interface seeking to replace and improve on `tig`_
* Can display complex history in novel ways to make them graspable. Maybe [this post] can be an inspiration.

### gix-tix

A re-implementation of a minimal `tig` like UI that aims to be fast and to the point.

### gix-lfs

Definitely optimize for performance and see how we fare compared to [oxen](https://github.com/Oxen-AI/oxen-release/blob/main/Performance.md).
Right now, `git lfs` is 40x slower, due to sequential uploads and lack of fast compression. It seems this can be greatly improved to get
close to 6min for 200k images (1.4GB). GitHub seems to cap upload speeds to 100kb/s, one major reason it's so slow, and it can only do
it sequentially as `git-lfs` doesn't use the new `filter-process` protocol which would allow parallelization.
Oxen uses the XXH3 (30gb/s) which greatly outperforms SHA1 - however, it doesn't look like the hash is necessarily the bottleneck in typical benchmarks.

[tagname-validation]: https://github.com/git/git/blob/master/Documentation/technical/protocol-common.txt#L23:L23
[this post]: http://blog.danieljanus.pl/2021/07/01/commit-groups/
