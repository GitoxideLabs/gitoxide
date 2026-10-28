#!/usr/bin/env bash
set -eu -o pipefail

# A committed .gitmodules and an indexed gitlink make `submodule init` work
# without creating or cloning a real submodule. All tests share these frozen
# repositories read-only; only this script writes configuration or invokes Git.
git init -q seed
(cd seed
  git symbolic-ref HEAD refs/heads/main
  printf '%s\n' '[submodule "m1"]' 'path = m1' 'url = ../module1' >.gitmodules
  git add .gitmodules
  git commit -q -m 'add module configuration'
  seed_commit_id=$(git rev-parse HEAD)
  git update-index --add --cacheinfo "160000,$seed_commit_id,m1"
  git commit -q -m 'add module gitlink'
  mkdir m1
  git remote add origin ../seed
  git config branch.main.remote origin
  git config branch.main.merge refs/heads/main
)

# Run Git in the very worktree Rust will open: local fallbacks embed that path.
# Save the pre-init config, record status/output, and copy initialized state only
# after success. Restore the original config so worktree remains uninitialized;
# initialized retains Git's override referring to the original worktree path.
record_init() {
  local case_dir=$1 status=0
  cp "$case_dir/worktree/.git/config" "$case_dir/config-before-init"
  if git -C "$case_dir/worktree" submodule init >"$case_dir/init.stdout" 2>"$case_dir/init.stderr"; then
    git -C "$case_dir/worktree" config --get submodule.m1.url >"$case_dir/expected-url"
    cp -R "$case_dir/worktree" "$case_dir/initialized"
  else
    status=$?
  fi
  printf '%s\n' "$status" >"$case_dir/init-status"
  mv "$case_dir/config-before-init" "$case_dir/worktree/.git/config"
}

# Keep all 26 original URL pairs, including empty origins, excess parent
# components, trailing slashes, non-relative module URLs, and scp-style origins.
# Input files contain the raw strings plus one newline, not Git's resolved URL.
i=0
while IFS='|' read -r remote_url module_url; do
  i=$((i + 1))
  case_dir="resolution/$i"
  mkdir -p "$case_dir"
  cp -R seed "$case_dir/worktree"
  printf '%s\n' "$module_url" >"$case_dir/module-url"
  # Keep Git Bash from rewriting absolute URL paths as native command arguments.
  MSYS2_ARG_CONV_EXCL='*' git -C "$case_dir/worktree" config remote.origin.url "$remote_url"
  # Avoid command-substitution forks that can crash Git Bash on Windows ARM.
  git -C "$case_dir/worktree" config --get remote.origin.url >"$case_dir/remote-url"
  IFS= read -r configured_remote_url <"$case_dir/remote-url"
  test "$configured_remote_url" = "$remote_url"
  printf '%s\n' '[submodule "m1"]' 'path = m1' "url = $module_url" >"$case_dir/worktree/.gitmodules"
  record_init "$case_dir"
done <<'CASES'
ssh://host/org/super.git|./sub
ssh://host/org/super.git|../sub
ssh://host:22/org/super.git|../sub
git://host/org/super.git|./sub
git@host:org/super.git|./sub
git@host:org/super.git|../sub
git@host:org/super.git|../../sub
https://host/org/super.git|./sub/
https://host/org/super.git/|././sub
https://host/org/super.git|.././sub
https://host/org/super.git|../../../sub
file:///srv/git/super.git|../sub
/srv/git/super|../sub
/srv/git/super|../../../../sub
//share/super|../../sub
foo/bar|../sub
./foo|../sub
../foo/bar|../sub
ssh://host/org/super.git|.sub
ssh://host/org/super.git|https://other/sub
ssh://host/org/super.git|..//sub
ssh://host/org/super.git|./sub/../nested
foo|../../sub
git@host:super.git|../../../sub
../foo|../../sub
|../sub
CASES

# Git 2.51 began falling back to the only remote for detached HEAD or a branch
# without remote configuration. Always record the installed Git's baseline, but
# mark those two comparisons as skipped for older Git, just like the tests did.
git_version=$(git version)
git_version=${git_version#git version }
IFS=. read -r git_major git_minor _ <<<"$git_version"
only_remote_matches_git=false
if (( git_major > 2 || (git_major == 2 && git_minor >= 51) )); then
  only_remote_matches_git=true
fi

for name in upstream-branch upstream-detached missing own-upstream upstream-only multiple; do
  case_dir="remote-selection/$name"
  mkdir -p "$case_dir"
  cp -R seed "$case_dir/worktree"
  git -C "$case_dir/worktree" config --remove-section remote.origin
  git -C "$case_dir/worktree" config --remove-section branch.main
  compare_git=true
  case "$name" in
    upstream-branch|upstream-detached|upstream-only)
      git -C "$case_dir/worktree" config remote.upstream.url ssh://host/org/upstream.git
      if [[ "$name" != upstream-only ]]; then
        git -C "$case_dir/worktree" config branch.main.remote upstream
      fi
      if [[ "$name" == upstream-detached ]]; then
        git -C "$case_dir/worktree" checkout -q --detach
      fi
      if [[ "$name" != upstream-branch ]]; then
        compare_git=$only_remote_matches_git
      fi
      ;;
    missing)
      git -C "$case_dir/worktree" config branch.main.remote missing
      ;;
    own-upstream)
      git -C "$case_dir/worktree" config branch.main.remote .
      ;;
    multiple)
      git -C "$case_dir/worktree" config remote.one.url ssh://host/org/one.git
      git -C "$case_dir/worktree" config remote.two.url ssh://host/org/two.git
      ;;
  esac
  printf '%s\n' "$compare_git" >"$case_dir/compare-git"
  record_init "$case_dir"
  test "$(cat "$case_dir/init-status")" -eq 0
done

# Overrides are local configuration, not remote-relative .gitmodules URLs.
# Derive both from the last remote-selection worktree, before Git initialized it.
cp -R remote-selection/multiple/worktree override-absolute
cp -R remote-selection/multiple/worktree override-relative
git -C override-absolute config submodule.m1.url ssh://other/override.git
git -C override-relative config submodule.m1.url ../override

# Keep these HEADs unborn, with .gitmodules only on disk. A configured upstream
# resolves remotely; a missing upstream and no remotes resolve against the repo.
git init -q unborn-upstream
(cd unborn-upstream
  git symbolic-ref HEAD refs/heads/main
  printf '%s\n' '[submodule "module"]' 'path = module' 'url = ../sub' >.gitmodules
  git config branch.main.remote upstream
  git config remote.upstream.url ssh://host/org/super.git
)
cp -R unborn-upstream unborn-missing
git -C unborn-missing config --remove-section remote.upstream
cp -R unborn-missing unborn-local
git -C unborn-local config --remove-section branch.main
mkdir sub

# Commit a separate copy so bare HEAD supplies .gitmodules without making either
# unborn test repo born. A relative local origin resolves ../sub to fixture/sub.
cp -R unborn-missing bare-source
git -C bare-source add .gitmodules
git -C bare-source commit -q -m 'add module configuration'
git clone -q --bare bare-source bare.git
git -C bare.git config remote.origin.url ../unborn-missing

# Git Bash reports Windows via these uname prefixes. Native drive/separator
# handling belongs only there; arbitrary non-UTF-8 path bytes belong only on Unix.
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    mkdir native-relative
    cp -R seed native-relative/worktree
    git -C native-relative/worktree config remote.origin.url C:/repos/super.git
    # Two literal backslashes in Git config decode to one native separator.
    printf '%s\n' '[submodule "m1"]' 'path = m1' 'url = ..\\sub' >native-relative/worktree/.gitmodules
    record_init native-relative
    test "$(cat native-relative/init-status)" -eq 0
    ;;
  *)
    cp -R seed non-utf8
    # Retain origin ../seed: the non-UTF-8 destination must remain a local URL.
    printf '[submodule "m1"]\npath = m1\nurl = ../source-\xff\n' >non-utf8/.gitmodules
    ;;
esac
