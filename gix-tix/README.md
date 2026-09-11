This is a `tig` inspired completely generated program to do what I used `tig` for, namely:

- show project histories, but allow to trim them to hide given branches
- copy selected hashes
- but be faster and less memory hungry than `tig` when looking at big repositories.

The commits that created it are clearly identified as authored by GPT, without myself as co-author.
After all, I intentionally didn't look at the code.

And from what I can tell, it does what I want it to, and seems to be worth maintaining.

## Keyboard shortcuts

Press `?` for keyboard help or `p` for the searchable command menu. The `v`, `a`,
and `n` groups contain display controls, actions, and enrichment commands.
These shortcuts also work directly, without a prefix:

| Key | Action |
| --- | --- |
| `H` (`Shift+H`) | Show or hide integration-branch history from history or a changes pane; infer the branches when `-x` is omitted. |
| `P` (`Shift+P`) | Push the active branch from history or Worktree. In Tree, cycle the comparison parent; `a P` pushes from there. |

See the [full keyboard reference](spec.md#navigation-and-display-controls) for
navigation and the remaining direct shortcuts.

## Marking reviewed patches

Press `n r` to mark the selected patch **refackiewed** (refactored and reviewed)
with `✨`, or to clear the mark. The command menu also finds `refackiewed`.
From the shell, use `tix enrich patch refackiewed [REVSPEC]` and add `--clear`
to remove the mark; the target defaults to `HEAD`.

The mark belongs to that version of a Tix change's patch. It survives rewording
and rebasing when the patch's edits stay the same, even when ancestor changes
move lines or alter unrelated files. Changing the patch hides its old mark;
returning to the approved version restores it. The existing `✔️` checks-pass
mark still belongs to one exact tree.

Tix stores the patch identity in a commit header and the mark in worktree-local
notes. Marking a commit without that header rewrites it and its descendants,
preserving staged changes and worktree files; previously final descendants stay
final. Lazy rebases hide the mark until replay refreshes the identity. Browsing
only reads existing metadata and never calculates patch hashes or backfills old
commits. See [patch identity and enrichment](spec.md#patch-identity-and-enrichment)
for identity, eligibility, and caching rules.

## Worktrees

`tix worktrunk` (or `tix wt`) opens a worktree picker with the selected
worktree's interactive history below it. Install its `wt` shell wrapper by
evaluating `tix worktrunk shell-init bash` or `zsh`, piping the `fish` output to
`source`, or loading the generated `nushell`/`powershell` script from the
corresponding shell profile. The same command below `gix tix` generates a
wrapper which uses `gix tix` throughout.

`wt switch BRANCH` switches to an existing worktree or creates one for an
unchecked-out local branch. `wt switch --new-branch BRANCH` creates a missing
branch at the logical Tix HEAD, or reuses it if it exists. `--path PATH`
overrides the default sibling path.

Selecting a worktree in the picker also returns to the shell and changes its
directory. Open `tix` there and press `Shift+H` (View **hide unrelated history**)
to hide commits reachable from the integration branches inferred by `tix show`.
The same action becomes **show related history** while filtered; press it again
to restore the full history. Explicit `-x` filters apply immediately when
opening Tix.

`wt switch --detach [COMMIT]` creates a detached worktree at the current HEAD
or a supplied commit. `--path PATH` chooses its directory; otherwise a sibling
directory uses the commit's short hash, adding a number if occupied. When the
source worktree already has Tix pins, it gains a symbolic pin following the new
worktree's HEAD so its experiment stays visible in the source's history.

`wt show` prints the fully populated worktree table without opening the picker.


It's also an experiment to see how long, or if at all, this is maintainable.
