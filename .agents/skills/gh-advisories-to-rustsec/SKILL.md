---
name: gh-advisories-to-rustsec
description: Review this repository's published GitHub security advisories and integrate all entries missing from a user-prepared local RustSec advisory-db worktree. Create Codex-authored local commits and comparisons for human review, reauthoring, and submission; do not publish advisories, push, or create a PR.
---

# GH Advisories to RustSec

Bring this repository's public advisories to RustSec so downstream Rust users can discover them through tools such as `cargo audit`. Finish with local commits for the user to review and make their own. Creating this skill or discussing an advisory does not itself request an import.

## Prepared Worktrees

- Use the current repository as the source, resolving its GitHub owner/name from its remote and security policy. For this repository that is `GitoxideLabs/gitoxide`; honor an explicit source override.
- Use the user's prepared RustSec checkout at the supplied path (for example, `/path/to/advisory-db`); verify it exists and its remotes identify the intended database. Do not clone, switch branches, or create a worktree to make the input fit.
- Read applicable instructions in both repositories and the destination's `CONTRIBUTING.md`, `README.md`, `EXAMPLE_ADVISORY.md`, PR template, and validation workflow. The destination's current format and tooling take precedence over examples below.
- Before destination edits, record its branch (or detached state), starting `HEAD`, and status including untracked files. Require a pristine index and worktree, with no unfinished Git operation. Verify `HEAD` contains the current upstream default-branch tip using a read-only remote check such as `git ls-remote --symref <upstream-remote> HEAD` plus an ancestry check. A stale local tracking ref alone does not prove freshness. If the checkout is dirty, stale, missing, or freshness cannot be verified, report the precondition failure; do not stash, reset, pull, or rebase it automatically.
- Keep source code and source GHSAs unchanged. Correct the RustSec drafts locally and report any recommended upstream corrections separately. Never push, open a PR, comment on GitHub, or change publication state in this workflow.

## Inventory and Coverage

Fetch every page of repository security advisories, not just a global GitHub search or the first page. For example, with `source_repo` set to the verified owner/name:

```bash
gh api --paginate --slurp "repos/$source_repo/security-advisories?state=published&per_page=100"
```

Retain the raw public source snapshot in a task-owned review directory outside the destination. Read each full advisory, including its description, identifiers, dates, package/ecosystem, affected ranges, patched versions, CVSS vector, and references. Do not import drafts, embargoed or withdrawn advisories, or non-Cargo packages; record exclusions. Report API/access/pagination failures rather than claiming complete coverage from partial data.

Inventory coverage per **package and vulnerability**. Search all existing RustSec entries for GHSA/CVE identifiers in IDs, aliases, and source URLs, then check package and issue context. An alias already present for one crate does not cover another affected crate. Conversely, an existing entry with a matching CVE or source URL may cover a GHSA even without its GHSA alias. Use `related` only for different related vulnerabilities. Do not overwrite existing entries; report material discrepancies separately. Repeat deduplication against this run's drafts as well.

Keep a ledger for every source advisory/package: already covered (with RustSec path/ID), imported, excluded, or blocked with a specific reason. Work through all candidates; one uncertain advisory should not prevent independent, well-supported imports.

## Review Before Translation

Check each missing advisory for significant inaccuracies before it enters another database. A short check is sufficient when the evidence already agrees; do not require a comprehensive new security audit or cosmetic perfection.

- **Claim and impact:** follow linked reports, fixing commits, relevant code/tests, and release history as needed. Verify the trigger, vulnerable behavior, consequence, and actual fix. Correct substantially unsupported claims, exaggerated impact, and contradictions when evidence establishes the correction. Do not infer exploitation, memory unsafety, or remote reachability just from a title or severity label.
- **Package and versions:** verify exact published Cargo package names, including plumbing crates versus their wrappers, and introduced/fixed versions on each affected release line. Cross-check GHSA bounds and first-patched versions with release evidence; do not assume all workspace crates share versions. Preserve platform or feature prerequisites supported by the evidence.
- **CVSS:** check the vector's metrics against the described attack and its computed score against the source score/severity. Use a trustworthy calculator or library if recomputing it. Preserve a supported vector in a version accepted by the current RustSec schema; do not invent one from a severity label. Omit optional CVSS with an explanation when it cannot be justified or represented.
- **Writing:** retain the root cause, trigger/prerequisites, impact, affected versions, fix/workaround, useful references, and relevant attribution. Remove repetitive filler, unsupported conclusions, and empty or formulaic headings. Prefer a descriptive title and short, concrete paragraphs; retain technical detail that helps users assess exposure.

Record significant factual/metadata corrections and their evidence, distinguishing verified facts from remaining uncertainty. If package, affected range, or core vulnerability claims remain materially uncertain, leave that candidate blocked rather than guess. Minor editorial imperfections do not block a useful, accurate import.

## RustSec Drafts and Validation

Create one entry per affected Cargo package and distinct vulnerability, following the current template. A GHSA covering several crates can require several entries. For a batch, the current draft convention permits:

```text
crates/<package>/RUSTSEC-0000-0000.GHSA-xxxx-xxxx-xxxx.md
```

The **dot** after `RUSTSEC-0000-0000` matters for draft recognition. Keep metadata `id = "RUSTSEC-0000-0000"`; maintainers assign real IDs. Use unique filenames and never overwrite a prior draft or invent a numbered RustSec ID.

- Place fenced TOML first, then the Markdown title and description. Remove template comments and unused fields.
- Set `package` to the exact published name and `date` to the first public disclosure date, not the import/update date. Preserve GHSA and any CVE aliases.
- For imported GitHub Advisory Database material, set `license = "CC-BY-4.0"` and `url` to the original advisory for attribution, following the database's licensing rules. Keep useful fix/report links in `references`.
- RustSec records **safe version ranges**: `[versions].patched` plus optional `unaffected`. Their complement is vulnerable. Preserve introduced-version lower bounds, bounded backports, and separate release lines; never translate a vulnerable range directly into `patched`. Use `patched = []` if no fix exists, rather than guessing a future fixed release.
- Restrict OS/architecture and affected function paths only with evidence. Function paths use Rust crate identifiers with underscores where package names have hyphens. Explain feature-specific exposure in the description when the schema cannot express it.

Check the resulting vulnerable set against the source evidence at introduced/fixed boundaries, adjacent published versions, every supported backport line, and prereleases when relevant. Use current RustSec/Cargo semver tooling or an isolated assert-based check; TOML parsing alone does not verify range semantics.

Run the destination's current advisory validator before each commit and cumulatively at the end. Its present CI command is `rustsec-admin lint` from the database root; inspect the workflow's installer revision when selecting compatible tooling, especially for draft filename support. Use/install tooling outside the checkout. If validation is unavailable, report the exact blocker and leave the affected drafts uncommitted for review; do not call them validated. Fix task-caused failures and distinguish pre-existing database failures. Check staged diffs and `git diff --check` too.

## Local Commits and Review Artifacts

Create one self-contained commit per GHSA, including all its new package entries. Stage only that advisory's files and check that the whole index contains only intended changes before committing. Recheck destination state before mutations; preserve any concurrent work without absorbing it.

Use explicit agent authorship and both requested trailers, even though they repeat the author. Use `Codex <codex@openai.com>` for Codex, or the executing agent's actual identity if different; never use the user's identity for these commits. For Codex:

```text
Assisted-by: Codex <codex@openai.com>
Co-authored-by: Codex <codex@openai.com>
```

Write the full Markdown commit message to a task-owned file outside the checkout and pass `--file`; describe the motivation, source GHSA, and significant corrections, rather than only the added files. For example:

```bash
git -C "$advisory_db" -c user.name=Codex -c user.email=codex@openai.com \
  commit --author='Codex <codex@openai.com>' --file="$message_file"
```

Keep identity overrides command-local and honor signing settings. Verify each committed author and both trailers. Do not rewrite pre-existing commits. Finish on the prepared checkout with a clean worktree unless explicitly reported blocked drafts or concurrent changes remain.

In the external review directory, retain the raw GHSA snapshots and, per new entry, the original summary as an H1 followed by its description, the proposed RustSec title and description without TOML, and a source-versus-proposed metadata table with correction reasons/evidence. Save both prose versions as Markdown so title changes are visible without format-only diff noise. Do not commit these artifacts into advisory-db.

## Human Handoff

Report source/destination, starting and final hashes, every ledger outcome, new commit hashes/paths, corrections and uncertainty, and checks performed or blocked. State that the commits are agent-authored and require human review and reauthoring. Do not claim all advisories were integrated when any candidate or inventory page remains unresolved.

Provide commands with the **actual paths and hashes** for reviewing the changes:

- `git -C <db-path> log --reverse <start-hash>..HEAD` and `git -C <db-path> diff <start-hash>..HEAD -- crates/` for the complete series; `git -C <db-path> show <commit-hash>` for one advisory.
- `code --diff <original-description.md> <proposed-description.md>` for side-by-side edits, and `code <new-advisory.md>` followed by **Markdown: Open Preview** (`Cmd+Shift+V` on macOS, `Ctrl+Shift+V` elsewhere) to view the rendered advisory. These are optional VS Code examples; name the comparison and metadata files so another Markdown viewer can open them too.
- `gh browse --repo <source-owner/repo> security/advisories/<GHSA-ID>` to view the original published advisory. Point to the metadata comparison as well; prose diffs alone do not show version/CVSS corrections.

Remind the user to review every advisory and change the authors to themselves while keeping the agent trailers. For a verified linear series containing only this run's commits, show this **user-run** command with the recorded start hash and destination path, after checking their Git identity:

```bash
git -C "$advisory_db" var GIT_AUTHOR_IDENT
git -C "$advisory_db" rebase --force-rebase \
  --exec 'git commit --amend --no-edit --reset-author' "$start_hash"
```

Prepare a suggested PR title/body outside the checkout describing the downstream benefit, affected crates, source links, substantive corrections, validation, and AI assistance. The user creates the PR themselves **after reviewing and reauthoring**. Provide concrete commands for their configured writable fork remote and submission branch; do not assume `origin` is a writable fork or push directly to upstream. If the fork/branch is unknown, label the placeholders. For example, with those values chosen by the user:

```bash
git -C "$advisory_db" push "$fork_remote" "HEAD:refs/heads/$submission_branch"
gh pr create --repo RustSec/advisory-db --head "$fork_owner:$submission_branch" \
  --title "$pr_title" --body-file "$pr_body_file"
```

Leave review artifacts available and report their directory. End with the reminder that the user, after review and reauthoring, runs the push and `gh pr create` commands; the agent has performed neither.
