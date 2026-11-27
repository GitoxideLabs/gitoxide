# Continuous Git re-integration (CGR)

Each bullet classifies one analyzed source commit from Git or git-for-windows, leading with its bold short hash, an explicit class, and one sentence explaining the behavior and decision.
Entries contain source hashes only.

| Class | Meaning |
| --- | --- |
| `PORTED` | Relevant behavior has been implemented and validated in Gitoxide. |
| `ALREADY-COVERED` | Existing implementation and tests cover the behavior, or a merge adds no behavior beyond separately analyzed commits. |
| `NOT-APPLICABLE` | The Git feature or change has no meaningful counterpart in Gitoxide. |
| `POSTPONED` | Relevant behavior needs infrastructure Gitoxide lacks; name the prerequisite and a concrete revisit condition. |

`POSTPONED` entries remain open for later sweeps; update their existing row when the decision changes or resolves.
Other unresolved analysis remains outside this document.

## Baselines

These local snapshots establish the explicitly requested baseline for future sweeps.
Baseline creation is allowed only when this document is absent; an existing document must retain its recorded baselines.
Commits reachable from either baseline are outside the default sweep scope and remain unexamined; no historical classification is recorded.
Future sweeps process commits reachable from their pinned source tips that are not reachable from either baseline, including newly merged side branches regardless of commit dates, unless other revision bounds are explicitly requested.

| Source repository | Source ref | Full baseline commit |
| --- | --- | --- |
| `git/git` | `refs/heads/master` | `6de20f6092dcf9bdb1c8efe03db4b70c82b423dd` |
| `git-for-windows/git` | `refs/heads/main` | `fa1a8a40e3d68689071e9e88d5a521a6c4ba07a4` |

## Source commits
- **d8e1ba5c4d6f** NOT-APPLICABLE: Share compiled C HTTP sources among Git's Meson executables because Gitoxide builds its HTTP implementation in the shared Cargo crate `gix-transport`.
- **cc839e97a3dc** NOT-APPLICABLE: Reuse objects when building Git's test-only remote helper aliases because Gitoxide dispatches HTTP through `gix-transport` and has no equivalent helper binaries to relink.
- **ae24379dd46b** NOT-APPLICABLE: Precompile Git's C compatibility header for its test helper because Gitoxide builds its shared test helpers as Rust crates.
- **3d7418e351a2** NOT-APPLICABLE: Precompile Git's C unit test headers while compiling Clar separately because Gitoxide uses Rust's test harness and Cargo nextest.
- **47780704699b** NOT-APPLICABLE: Fix the Meson copy order for Git's shell scripts because Gitoxide generates completions directly from its Clap command definitions.
- **e492974847e1** NOT-APPLICABLE: Refresh Git's Meson dependency wrappers because Gitoxide selects native dependencies through Cargo and uses separate Rust implementations for compression and regular expressions.
- **ce4a600322fa** NOT-APPLICABLE: Disable PortableGit's credential helper for GitLab Runner cleanup because Gitoxide's Windows checks run under GitHub Actions and do not use that installer or cleanup path.
- **1c1256682a54** NOT-APPLICABLE: Integrate the Meson series and resolve the Windows CI installer conflict by retaining the credential-helper cleanup without restoring the deleted Rust MSI step because Gitoxide provisions Rust through its GitHub Actions workflow.
- **72c41c3ee0bc** PORTED: Resolve `@{p}` and `@{P}` through the same push-tracking lookup as `@{push}`, including named branches and a following reflog selector.
- **bb5979706a7a** NOT-APPLICABLE: Compare Git's huge custom log-format output byte by byte because Gitoxide's log prints a fixed first-line summary and has no such formatter test.
- **4f956cf43ad8** NOT-APPLICABLE: Tune the Make and Perl prove job counts for Git's expensive-test matrix because Gitoxide schedules Cargo and nextest workloads whose concurrency is independent of that measured workload.
- **54fd0d4c5e3d** ALREADY-COVERED: Merge the separately analyzed huge-output comparator and build concurrency changes without further changes.
- **a68cca9953cf** NOT-APPLICABLE: Keep hardcoded historical reflogs in Git's pull-rebase test because Gitoxide does not run that pull workflow or its automatic reflog expiry during fixture generation.
- **08ab14b08456** ALREADY-COVERED: Merge the separately analyzed pull test fixture change without further changes.
- **c17b2d9f6b5a** ALREADY-COVERED: Convert signed Git timezone fields at the parsing and formatting boundaries through `gix-date`, whose offset tests cover positive and negative non-hour offsets.
- **f97936288dab** NOT-APPLICABLE: Translate hash IDs in Git's private reftable-dump helper because Gitoxide has no equivalent C test helper or hash-ID translation.
- **e14e2b6cf2db** POSTPONED: Encode reflog timezones as signed minutes once Gitoxide has a reftable backend that can round-trip those records against Git.
- **1ee477ab508d** ALREADY-COVERED: Merge the three separately analyzed timezone and reftable changes without an additional change of its own.
- **f22cc52e9df3** NOT-APPLICABLE: Free partially allocated C xdiff buffers on file-read errors because Gitoxide uses owned Rust buffers and automatic cleanup.
- **a28d2bfe1958** NOT-APPLICABLE: Unify xdiff's two C buffer structures because Gitoxide passes byte slices and vectors through its Rust merge pipeline.
- **3c7a31204ff3** ALREADY-COVERED: Use platform-width lengths for file and blob buffers because Gitoxide's merge and diff pipelines use Rust buffer lengths and test the handling of large resources.
- **25b334d8dbca** NOT-APPLICABLE: Add a defensive NUL after C xdiff input because Gitoxide's byte-slice readers carry their lengths and treat NUL as data.
- **c9a589810288** ALREADY-COVERED: Read external merge output with contextual I/O errors through Gitoxide's existing file open and `read_to_end` path.
- **3c00e11eabd0** ALREADY-COVERED: Interpret external merge driver exit status before reading its output, as Gitoxide already does for successful, conflicting, and failed exits.
- **5b0456361d23** ALREADY-COVERED: Treat missing external merge output as an I/O error even after a successful or conflicting driver exit, as verified through `gix merge file`.
- **81db32ddce56** ALREADY-COVERED: Integrate the xdiff series by retaining the temporary-file cleanup and reading from its registered output path, which Gitoxide already does with owned temporary files and its stored result path.
- **0c94f6297dd7** ALREADY-COVERED: Merge the separately analyzed reflog-expiry fixture change into the stash topic without further changes.
- **3a7995913ea6** NOT-APPLICABLE: Drop an unused C header from Git's stash implementation because Tix restores stashes through Git without compiling that code.
- **704069929493** NOT-APPLICABLE: Move C merge-option initialization earlier inside Git's stash driver because Tix delegates restoration to Git.
- **b15661a2cd35** ALREADY-COVERED: Verify that a stash index conflict aborts before worktree restoration, which Tix already tests while checking that the local change and saved stash survive.
- **8ae4c185a6d6** POSTPONED: Merge a saved index in memory once Gitoxide has a native stash-apply path that can validate index conflicts and autostash without the installed Git.
- **b78604666654** ALREADY-COVERED: Merge the separately analyzed in-memory stash index series without adding behavior of its own.
- **5e54624a8edf** NOT-APPLICABLE: Preserve distinct values inside `git var` because Gitoxide exposes configuration paths through its discovery and configuration APIs and has no `git var` output layer.
- **e19f0779c67f** NOT-APPLICABLE: Add NUL delimiters to `git var` output because Gitoxide's CLI does not expose that variable-listing command.
- **09726e777e98** NOT-APPLICABLE: Query multiple logical variables in one `git var` invocation because Gitoxide has no corresponding variable-lookup command or single-value process boundary.
- **598588c15584** NOT-APPLICABLE: Add identity components to `git var` because Gitoxide exposes name, email, and time as structured fields through its repository actor APIs.
- **0930ad7aab7b** ALREADY-COVERED: Merge the separately analyzed variable-listing and identity changes without further changes.
- **b837e5f4bc15** NOT-APPLICABLE: Read patches from standard input for Git's contributors-by-patch helper because Gitoxide exposes blame and hours for repository history and has no patch-recipient command.
- **a9bd346d8b65** ALREADY-COVERED: Merge the separately analyzed patch-input change without further changes.
- **07cc531ed3e3** ALREADY-COVERED: Merge the separately ported push-tracking shorthand without further changes.
- **7a7d4c461324** NOT-APPLICABLE: Suppress a leading separator in Git's completion-helper protocol because Gitoxide uses Clap to generate shell completions directly.
- **1c4ee0aaa152** NOT-APPLICABLE: Extract single-option formatting from Git's C help printer because Gitoxide delegates help rendering to Clap.
- **4a2e53930b7a** NOT-APPLICABLE: Add group metadata to Git's C subcommand descriptors because Gitoxide declares commands through Clap derives.
- **a69a982f48a1** NOT-APPLICABLE: Group help for the `git refs` command because Gitoxide exposes ref operations through repository APIs and distinct CLI commands instead of that command tree.
- **4e399f027c21** NOT-APPLICABLE: Resolve the C option-macro merge by retaining aliases alongside grouped subcommands because Gitoxide declares its command aliases and help using Clap.
- **778c73948a47** NOT-APPLICABLE: Correct capitalization in Git's lightweight-tag tutorial because Gitoxide does not reproduce that prose.
- **605bb2446532** ALREADY-COVERED: Merge the separately analyzed lightweight-tag tutorial edit without further changes.
- **c5f8f607191c** NOT-APPLICABLE: Clarify that Git's remote management commands only change local refs and configuration because Gitoxide's remote CLI exposes URL and ref queries and its default-branch API already identifies the local tracking record.
- **e37840933fdc** ALREADY-COVERED: Merge the separately analyzed remote-management documentation change without further changes.
- **42017ff9792f** NOT-APPLICABLE: Fix wording in Git's command-backed trailer examples because Gitoxide parses existing trailers and does not carry those examples.
- **0bde46266881** ALREADY-COVERED: Merge the separately analyzed trailer examples edit without further changes.
- **4ddd9f289deb** NOT-APPLICABLE: Simplify the C format-patch notes helper because Gitoxide has no such helper and exposes notes through the separate `gix-note` API.
- **5008bde9900d** POSTPONED: Support distinct notes for an emailed patch and its range diff once Gitoxide can generate patch series with range-diff summaries.
- **a116545aade8** ALREADY-COVERED: Merge the separately analyzed range-diff notes option without further changes.
- **28699cf58187** NOT-APPLICABLE: Replace a numeric stash option value with a named C constant because Tix passes explicit arguments to Git and does not parse those option values.
- **c2593bc14c8b** ALREADY-COVERED: Merge the separately analyzed stash option cleanup without further changes.
- **f57c9a738844** NOT-APPLICABLE: Collect changes in Git's draft release notes because Gitoxide generates its own releases and analyzes the underlying source changes individually.
- **4811dabed745** NOT-APPLICABLE: Restore the original-to-rewritten order when loading Git filter-branch state because Tix manages replay state in its own refs and does not consume that command's state-branch format.
- **2341e1d74c49** ALREADY-COVERED: Merge the separately analyzed filter-branch state-map fix without further changes.
- **c7860da7db9e** NOT-APPLICABLE: Document ways to override Git's injected `LESS` defaults because Tix launches `core.pager` with shell semantics without setting those defaults.
- **d8d02ce25f27** ALREADY-COVERED: Merge the separately analyzed pager documentation edit without further changes.
- **e2d64bd041a7** NOT-APPLICABLE: Fix shell quoting in Git's libcurl version test prerequisite because Gitoxide's HTTP transport tests do not use that prerequisite or its nested awk snippet.
- **cc6a26fab556** ALREADY-COVERED: Merge the separately analyzed HTTP test quoting fix without further changes.
- **fbbc33935a25** NOT-APPLICABLE: Allow section 7 manuals to omit an empty synopsis because Gitoxide relies on Rustdoc and Clap help and does not enforce Git's AsciiDoc manual conventions.
- **3fb69375f116** ALREADY-COVERED: Merge the separately analyzed section 7 manual cleanup without further changes.
- **6faaf59b8b97** NOT-APPLICABLE: Improve path-existence diagnostics in Git's help-to-manual consistency test because Gitoxide generates help from Clap and has no such manual-file manifest.
- **02c88ed90a9a** ALREADY-COVERED: Merge the separately analyzed manual-file test cleanup without further changes.
- **49e646f1c9a2** NOT-APPLICABLE: Replace the expired SHAttered project URL in Git's hash transition document because Gitoxide's hash and threat-model documentation do not cite that URL.
- **a3059f04e360** ALREADY-COVERED: Merge the separately analyzed hash transition link edit without further changes.
- **c51408889c3b** NOT-APPLICABLE: Update Git's draft release notes because Gitoxide generates its own releases and tracks behavior against the individual source changes.
- **4c32d2259b13** NOT-APPLICABLE: Move Git's C pack-close routine below its global cache definition because Gitoxide resolves packs through Rust stores with owned file handles.
- **86129fddb7e6** POSTPONED: Defer validation of `fetch.followRemoteHEAD` until it is used once ordinary Gitoxide fetches can discover and update the remote-tracking HEAD according to that policy.
- **80502091b8e7** ALREADY-COVERED: Merge the separately analyzed lazy remote HEAD setting without further changes.
- **a61035fab929** ALREADY-COVERED: Copy unchanged LF packed reference records and peeled lines verbatim during deletion, already verified byte for byte by the `gix-ref` transaction tests.
- **fc70f528aa19** ALREADY-COVERED: Merge the separately analyzed packed-reference passthrough without further changes.
- **72e75ffee70f** ALREADY-COVERED: Query commit presence and generation directly with `gix-commitgraph` lookup and accessors already used by its integration tests.
- **cef2d8c4cbca** POSTPONED: Seed post-fetch split commit-graph updates from changed refs once Gitoxide has an additive commit-graph writer and implements `fetch.writeCommitGraph`.
- **f2b751142236** ALREADY-COVERED: Merge the separately analyzed fetch commit-graph changes without further changes.
- **778d260c316b** NOT-APPLICABLE: Move pack index path resolution from the graph writer to Git's `--stdin-packs` caller because the Gitoxide graph API takes explicit read paths and has no corresponding writer boundary.
- **623d2c712559** NOT-APPLICABLE: Remove Git's internal object-source pointer from commit-graph APIs because Gitoxide's graph reader already takes an explicit path and hash kind independent of the object backend.
- **fc94ecab694d** POSTPONED: Preserve Windows CLI startup with Win32k calls disabled once the console-event dependency avoids eager User32 imports and the CLI can be verified under the Windows mitigation.
- **e766978a9db5** ALREADY-COVERED: Merge the separately analyzed Windows poll compatibility change without further changes.
- **8d596ea8e777** PORTED: Keep decoded delta entries scoped to the opened pack so a reopened or replacement pack cannot reuse entries associated with its old location ID.
- **6948b044581f** ALREADY-COVERED: Keep both submodule regression cases when merging the stale-cache fix because the resolution only preserves the parent tests and the behavior is covered by the separately ported cache isolation.
- **45b1324139d0** NOT-APPLICABLE: Group Git's loose and packed backends in a C directory descriptor because Gitoxide's dynamic store already models loose stores and packed indices without that source abstraction.
- **8db128e611a7** ALREADY-COVERED: Expose alternate directories as paths through Gitoxide's existing `alternate_db_paths` API and its linked-store tests.
- **4a9e8cab7db7** NOT-APPLICABLE: Change Git's directory lookup to return a files-backend descriptor because Gitoxide opens its object store from an explicit path and resolves alternate paths directly.
- **5c869c96a4e4** ALREADY-COVERED: Read loose and packed objects across several object directories through Gitoxide's existing alternate-store loading and iteration.
- **f80a79e23345** NOT-APPLICABLE: Relocate Git's temporary primary-source swap into its quarantine module because Gitoxide does not swap a global primary source and uses an object-memory overlay for isolated merge writes.
- **622a625abb97** NOT-APPLICABLE: Represent an existing Git quarantine transaction as a files-directory descriptor because Gitoxide's object-memory overlay neither uses that descriptor nor registers a new primary source.
- **a26ff4e576de** NOT-APPLICABLE: Activate Git's quarantine when it is created to combine two calls that Gitoxide's per-repository object-memory overlay already avoids.
- **6066be88b4a3** NOT-APPLICABLE: Move Git's temporary-source lifetime flag into the loose backend because Gitoxide represents transient merge writes with an owned in-memory overlay.
- **c96b05a9c26b** ALREADY-COVERED: Read alternates using an object-directory path through Gitoxide's existing path-based `gix_odb::alternate::resolve` API.
- **0ba59c845312** NOT-APPLICABLE: Remove Git's alternate-reading virtual callback because Gitoxide's file-store alternate resolver is a direct function.
- **b6d18b5d68a0** POSTPONED: Suggest the merge continuation command once Gitoxide's human status output explains in-progress operations and its worktree merge can be continued.
- **3bba248c2c2d** ALREADY-COVERED: Merge the separately analyzed status advice without further changes.
- **b1be790f3871** NOT-APPLICABLE: Avoid deleting a live GnuPG directory to test missing keys because Gitoxide's tag signature tests pass isolated homes to the signer and verifier without that deletion step.
- **b4b80b488503** ALREADY-COVERED: Merge the separately analyzed GnuPG fixture change without further changes.
- **47fdf2208c5b** NOT-APPLICABLE: Collect changes in Git's draft release notes because Gitoxide generates its own and reviews these source changes individually.
- **01de14365ddf** ALREADY-COVERED: Use Unicode 18 character widths for terminal-column alignment through the workspace's resolved `unicode-width` tables.
- **ad72032c8db3** ALREADY-COVERED: Merge the separately analyzed Unicode width table update without further changes.
- **cc97a912eb14** NOT-APPLICABLE: Stop and annotate a failing Git leak-sanitizer shell script because Gitoxide runs its CI tests through Cargo nextest and separate Rust sanitizer workflows.
- **470c6e9fe2f5** NOT-APPLICABLE: Locate Git's shell-test descriptions for GitHub annotations because Gitoxide's native QA uses nextest's test identities and does not use that shell annotation protocol.
- **494035192949** ALREADY-COVERED: Merge the separately analyzed shell-test annotations without further changes.
- **60a86bfc98d6** ALREADY-COVERED: Sign commits during replay through Tix's existing repository-configured replay signing and its descendant-signature tests.
- **1c0cec6657bf** ALREADY-COVERED: Sign directly rewritten and replayed Tix commits using `commit.gpgSign` and the configured key, with per-invocation configuration overrides.
- **8f352ee2dc57** ALREADY-COVERED: Merge the separately analyzed history-signing changes without further changes.
- **60e16b0387ae** NOT-APPLICABLE: Localize Git's delta-island count with plural forms because Gitoxide does not emit that pack progress message or use Git's gettext messages.
- **7ac84b726779** ALREADY-COVERED: Merge the separately analyzed delta-island progress message edit without further changes.
- **7e34d8174dcd** NOT-APPLICABLE: Collect changes in Git's draft release notes because Gitoxide generates its own and reviews these source changes individually.
- **ff099c5dc980** POSTPONED: Enforce bounded and validated ignore-list parsing once Gitoxide blame can skip specified revisions during attribution.
- **52890fcfd41d** POSTPONED: Discover the default ignore-revs file in a worktree or bare HEAD once Gitoxide blame can compute attribution with an ignored revision set.
- **eab342c82b6c** ALREADY-COVERED: Merge the separately analyzed default blame ignore list and parser changes without adding behavior of its own.
- **37b3475fb530** NOT-APPLICABLE: Collect Git's 9th draft release-note batch because Gitoxide has its own generated release notes and classifies the underlying changes separately.
- **8d1b215c0151** NOT-APPLICABLE: Collect Git's 10th draft release-note batch because Gitoxide has its own generated release notes and classifies the underlying changes separately.
- **b9e5cff1803a** NOT-APPLICABLE: Collect Git's 11th draft release-note batch because Gitoxide has its own generated release notes and classifies the underlying changes separately.
- **50fe5cc63f9c** NOT-APPLICABLE: Collect Git's 12th draft release-note batch because Gitoxide has its own generated release notes and classifies the underlying changes separately.
- **ebdc087dc99d** NOT-APPLICABLE: Collect Git's 13th draft release-note batch because Gitoxide has its own generated release notes and classifies the underlying changes separately.
- **750f5de7dfe9** NOT-APPLICABLE: Collect Git's 14th draft release-note batch because Gitoxide has its own generated release notes and classifies the underlying changes separately.
- **0ffa3e35c7c8** NOT-APPLICABLE: Collect Git's 15th draft release-note batch because Gitoxide has its own generated release notes and classifies the underlying changes separately.
- **fb255012acbc** NOT-APPLICABLE: Collect Git's 16th draft release-note batch because Gitoxide has its own generated release notes and classifies the underlying changes separately.
- **3c60ef1e38ee** NOT-APPLICABLE: Collect Git's 17th draft release-note batch because Gitoxide has its own generated release notes and classifies the underlying changes separately.
- **fbd2d1e765da** NOT-APPLICABLE: Collect Git's 18th draft release-note batch because Gitoxide has its own generated release notes and classifies the underlying changes separately.
- **dcb6cee7fbe7** NOT-APPLICABLE: Set the upstream Git 2.50.2 release metadata because Gitoxide derives its version and release notes from its own build and release metadata.
- **56332d1945e8** NOT-APPLICABLE: Set the upstream Git 2.51.3 release metadata because Gitoxide derives its version and release notes from its own build and release metadata.
- **c5dabd6987d1** NOT-APPLICABLE: Set the upstream Git 2.52.1 release metadata because Gitoxide derives its version and release notes from its own build and release metadata.
- **ed7e96191f58** NOT-APPLICABLE: Set the upstream Git 2.53.1 release metadata because Gitoxide derives its version and release notes from its own build and release metadata.
- **9291f4cfbfb5** NOT-APPLICABLE: Set the upstream Git 2.54.1 release metadata because Gitoxide derives its version and release notes from its own build and release metadata.
- **c060ae533689** NOT-APPLICABLE: Set the upstream Git 2.55.1 release metadata because Gitoxide derives its version and release notes from its own build and release metadata.
- **03b41523c576** NOT-APPLICABLE: Set the upstream Git 2.56.1 release metadata because Gitoxide derives its version and release notes from its own build and release metadata.
- **9354c5f04055** NOT-APPLICABLE: Collect Git's 19th draft release-note batch because Gitoxide has its own generated release notes and classifies the underlying changes separately.
- **7fb6055e678f** NOT-APPLICABLE: Collect Git's 20th draft release-note batch because Gitoxide has its own generated release notes and classifies the underlying changes separately.
- **eacee88cf8a2** NOT-APPLICABLE: Collect Git's 21st draft release-note batch because Gitoxide has its own generated release notes and classifies the underlying changes separately.
- **e1ec09b23c8e** NOT-APPLICABLE: Collect Git's 22nd draft release-note batch because Gitoxide has its own generated release notes and classifies the underlying changes separately.
- **f7de81456940** NOT-APPLICABLE: Collect Git's 23rd draft release-note batch because Gitoxide has its own generated release notes and classifies the underlying changes separately.
- **fbe541727e55** NOT-APPLICABLE: Inline Git's duplicate leading-directory function because Gitoxide uses direct Rust directory creation without that wrapper.
- **b428ea54ef89** ALREADY-COVERED: Create leading directories from an immutable path without applying sharing settings through Gitoxide's existing `create_dir_all` call.
- **a6d5ea281e75** ALREADY-COVERED: Preserve ordinary umask permissions on initialization ancestors, as Gitoxide already enforces and tests.
- **8f17ac5de4e1** ALREADY-COVERED: Pass repository sharing policy as a typed input during initialization, as Gitoxide already does before opening the repository handle.
- **8f74abc2de1c** ALREADY-COVERED: Leave clone destination ancestors at their umask permissions by using Gitoxide's common create-and-open path for init and clone.
- **b2adfd39a042** NOT-APPLICABLE: Zero Git's reusable C repository structure on cleanup because Gitoxide drops its Rust repository handle and initializes a fresh value.
- **0419d7d9c71a** ALREADY-COVERED: Prevent stale repository state from influencing initialization by constructing the Gitoxide repository from new paths and explicit options.
- **a9456705f188** ALREADY-COVERED: Accept a packed blob exactly at `core.bigFileThreshold`, as confirmed with Gitoxide's fsck and integrity verification commands.
- **adbe9b43f42b** ALREADY-COVERED: Merge the separately analyzed pack-verification boundary fix without further changes.
- **c1cdd3151e98** POSTPONED: Resolve unpublished commit and tag IDs from an importing session once Gitoxide has a native fast-import parser and an in-progress pack object table.
- **775dc1673e68** ALREADY-COVERED: Merge the separately analyzed fast-import object resolution without further changes.
- **56e490a9b501** ALREADY-COVERED: Preserve the checkout's branch identity during linked-worktree lookup using owned ref names and explicit ref edits, which Tix tests with checked-out linked branches.
- **bd52c7687460** ALREADY-COVERED: Merge the separately analyzed sequencer ref ownership fix without further changes.
- **5a7bcf02d0a7** NOT-APPLICABLE: Pin a ZIP64 test to SHA-1 because Gitoxide's archive tests use generated hash-aware fixtures and do not extract that precomputed SHA-1 pack.
- **3375f64ad4ed** NOT-APPLICABLE: Repair Git's Fedora Meson job switch because Gitoxide derives build variants from Cargo's QA metadata.
- **7097ce8cbe74** NOT-APPLICABLE: Drop Git's retired Linux Clang build case because Gitoxide does not use the corresponding shell job dispatcher.
- **a8dc3aa765f9** ALREADY-COVERED: Move 32-bit Linux testing to Debian because Gitoxide's QA catalogue already pins an i386 Debian container.
- **8f3199b6605d** NOT-APPLICABLE: Rename Git's feature-testing job because Gitoxide schedules its independent Cargo test variants through its own QA catalogue.
- **8173407ebc43** NOT-APPLICABLE: Move Git's special Linux feature job to a supported Ubuntu image because Gitoxide schedules affected Cargo checks on a separate Ubuntu matrix with no dependency on that GCC 8 job.
- **1476be81b7a1** NOT-APPLICABLE: Remove Git's historical Python 2 CI branch because Gitoxide's build and fixture validation have no corresponding Python 2 job.
- **3fff230cf5ee** POSTPONED: Add SHA-256 reftable test coverage once Gitoxide has a reftable backend that can be selected by its QA tests.
- **832192728cd8** NOT-APPLICABLE: Resolve the CI topic by retaining the reftable and SHA-256 matrix changes in Git's shell dispatcher because Gitoxide schedules variants from independent Cargo metadata.
- **e2e5e9f7405c** NOT-APPLICABLE: Document how to navigate Git's full manual and short help because Gitoxide exposes generated Clap help and does not ship that manual hierarchy.
- **5cdaf2b1a712** ALREADY-COVERED: Merge the separately analyzed Git help documentation without further changes.
- **de8036be8984** NOT-APPLICABLE: Expose the local hostname through `git var` because Gitoxide's CLI has no corresponding variable-query command.
- **a212c33c906b** NOT-APPLICABLE: Query the hostname consistently in Git's background GC lock test because Gitoxide does not use the `gc.pid` host-and-process lock protocol.
- **ba963df28289** ALREADY-COVERED: Merge the separately analyzed shell quoting fix into Git's prerequisite topic without further changes.
- **1ce26936d17d** NOT-APPLICABLE: Accept here-docs in Git's shell test prerequisite helper because Gitoxide's fixture scripts use a separate Rust harness.
- **84733a6de430** ALREADY-COVERED: Merge the separately analyzed shell prerequisite change without further changes.
- **e4bf62f7f701** POSTPONED: Wait for a bounded interval before failing to acquire `MERGE_RR.lock` once Gitoxide can record and reuse on-disk merge resolutions.
- **5f6b6aa3ab4a** POSTPONED: Skip a contended rerere lock during background maintenance once Gitoxide supports a native resolution cache and its automatic garbage collection.
- **8814f276c03f** ALREADY-COVERED: Merge the separately analyzed rerere lock changes without further changes.
- **facedc705c8a** POSTPONED: Return from a dry run before changing packs once Gitoxide can repack and drop filter-excluded objects from a promisor store.
- **8fe963845e7c** ALREADY-COVERED: Merge the separately analyzed filtered-repack dry-run change without further changes.
- **cc7510f3dbec** NOT-APPLICABLE: Initialize Git's copied C revision state before cleanup because Gitoxide's pack and bundle paths rely on initialized Rust values with scoped cleanup.
- **3a5cd30544d1** ALREADY-COVERED: Merge the separately analyzed bundle cleanup fix without further changes.
- **074aaaf4f03a** ALREADY-COVERED: Parse configuration bytes and the byte-order mark without interpreting high-bit bytes as EOF through the existing unsigned-slice parser.
- **21e0d223ed9d** ALREADY-COVERED: Merge the separately analyzed configuration byte fix without further changes.
- **24e00cde8660** POSTPONED: Omit out-of-range signed Unix timestamps once `rawzip` can write the DOS modification time separately from the extended timestamp.
- **dd2e9fb957da** ALREADY-COVERED: Merge the separately analyzed ZIP timestamp change without further changes.
- **16bd683014de** ALREADY-COVERED: Use supported macOS CI images because Gitoxide's QA catalogue uses macos-latest and macos-15-intel.
- **7d55df77906d** ALREADY-COVERED: Merge the separately analyzed macOS image migration without further changes.
- **7f4e8cd95684** POSTPONED: Make root reference inclusion per iterator once Gitoxide exposes iteration that explicitly includes root references alongside ordinary refs.
- **af456b19bafe** ALREADY-COVERED: Merge the separately analyzed root-reference iteration change without further changes.
- **010fd39c3e23** NOT-APPLICABLE: Restructure Git's existing promisor-fetch calls without changing behavior because Gitoxide's object lookup has no equivalent internal lazy-fetch routine.
- **96715284f4c2** NOT-APPLICABLE: Extract Git's C path allowlist callbacks for reuse because Gitoxide checks trusted directory settings in its repository opening path without those callbacks.
- **42896803342a** POSTPONED: Load the protected lazy-fetch allowlist once Gitoxide has a native upload-pack service and on-demand promisor fetch support.
- **4360734e907c** POSTPONED: Bound nested promisor fetch attempts once Gitoxide can fetch a missing object on demand.
- **85c7f5e7dac1** POSTPONED: Enable on-demand fetch only for allowed served repositories once Gitoxide implements native upload-pack with promisor fetching and a protected server trust policy.
- **d9c4679fa889** ALREADY-COVERED: Merge the separately analyzed trusted lazy-fetch changes without further changes.
- **ac6889a969aa** NOT-APPLICABLE: Change Git's branch-state cleanup argument into flags because Gitoxide does not share that C state cleanup interface.
- **63ea7cd6744b** NOT-APPLICABLE: Cache Git's per-path merge settings before recreating checkout conflicts because Gitoxide configures its merge platforms ahead of use and has no such checkout loop.
- **d5f4e42f0221** POSTPONED: Round-trip original conflict labels through `MERGE_LABELS` once Gitoxide supports recreating worktree conflicts from staged index entries.
- **b26dc1274c00** ALREADY-COVERED: Merge the separately analyzed checkout conflict-label changes without further changes.
- **440b7035bbe0** POSTPONED: Compute a tree even when another process holds the index lock once Gitoxide supports native index-to-tree writing with optional cache-tree writeback.
- **30e6f5e4e6a5** ALREADY-COVERED: Merge the separately analyzed index-lock behavior without further changes.
- **071f2e36bcba** NOT-APPLICABLE: Improve file-existence diagnostics in Git's pack sliding-window fixture because Gitoxide maps packs through its own reader and tests without that fixture script.
- **c5eda93f2719** ALREADY-COVERED: Merge the separately analyzed pack fixture change without further changes.
- **c3fc96de739b** NOT-APPLICABLE: Pin Cargo's output directory for Git's Makefile link step because Gitoxide builds its Rust binaries through Cargo directly and supports target directory overrides.
- **cb9cccc84362** ALREADY-COVERED: Merge the separately analyzed Cargo build directory fix without further changes.
- **b2608c407680** NOT-APPLICABLE: Check the exit status of Git's fixture `ls-files` call before comparing output because Gitoxide's index fixture checks use Rust assertions and command helpers that report failures.
- **84bea4324a2e** ALREADY-COVERED: Merge the separately analyzed index fixture change without further changes.
- **5e106cb22b6b** NOT-APPLICABLE: Correct the hash-width description of Git's subprocess report because Gitoxide gets pack names from typed index-write outcomes.
- **21bb64feefa3** NOT-APPLICABLE: Extract Git's subprocess report parser because Gitoxide's native pack writer returns paths and hash information directly.
- **dfb7f4977684** ALREADY-COVERED: Create the protective `.keep` explicitly through Gitoxide's native pack writer before publishing the pack.
- **b07099fff940** POSTPONED: Revisit aborting pack publication when another writer owns the matching `.keep` once Gitoxide has a native receive-pack quarantine transaction, because the bundle writer preserves the marker but still publishes the pack.
- **cc3b09a046f4** NOT-APPLICABLE: Preserve explicit pack-name processing while adapting Git's quarantine prepare call because Gitoxide publishes packs directly from native index-write outcomes.
- **d6d2d8557c32** ALREADY-COVERED: Pass the object database explicitly to the connectivity checker as Gitoxide already does through `gix-fsck`.
- **980e4de64134** NOT-APPLICABLE: Localize Git's verbose connectivity counts through gettext because Gitoxide's fsck command uses its own progress implementation.
- **96eb94a122cc** ALREADY-COVERED: Merge the separately analyzed fsck translation change without further changes.
- **8f3d6ad89f41** NOT-APPLICABLE: Localize two Git checkout count messages because Gitoxide has neither those messages nor Git's gettext system.
- **a04194e13422** ALREADY-COVERED: Merge the separately analyzed checkout translation change without further changes.
- **91ef176b8fb9** POSTPONED: Print a valid upstream recovery command after a configuration write failure once Gitoxide has a switch-and-create operation that sets tracking configuration.
- **d6370f765ea2** ALREADY-COVERED: Merge the separately analyzed tracking-configuration advice without further changes.
- **4ec526671145** NOT-APPLICABLE: Return a tracking-configuration error from Git's C branch setup instead of exiting because Gitoxide's Rust configuration APIs already return errors to callers.
- **ad74e02417f3** POSTPONED: Finish a successful switch when optional upstream configuration fails once Gitoxide can create and switch to a tracking branch in one operation.
- **c3764d15e61a** POSTPONED: Apply per-event disabling to configured and traditional hooks once Gitoxide has a native hook registry and runner for both sources.
- **3a7784d0d546** POSTPONED: Interpret bare hook booleans as true once Gitoxide implements named hook configuration and can validate the behavior through its hook runner.
- **aa139fdc67a0** ALREADY-COVERED: Merge the separately analyzed hook behavior changes without further changes.
