This is a `tig` inspired completely generated program to do what I used `tig` for, namely:

- show project histories, but allow to trim them to hide given branches
- copy selected hashes
- but be faster and less memory hungry than `tig` when looking at big repositories.

The commits that created it are clearly identified as authored by GPT, without myself as co-author.
After all, I intentionally didn't look at the code.

And from what I can tell, it does what I want it to, and seems to be worth maintaining.

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

`wt switch --detach [COMMIT]` creates a detached worktree at the current HEAD
or a supplied commit. `--path PATH` chooses its directory; otherwise a sibling
directory uses the commit's short hash, adding a number if occupied. When the
source worktree already has Tix pins, it gains a symbolic pin following the new
worktree's HEAD so its experiment stays visible in the source's history.

`wt show` prints the fully populated worktree table without opening the picker.


It's also an experiment to see how long, or if at all, this is maintainable.
