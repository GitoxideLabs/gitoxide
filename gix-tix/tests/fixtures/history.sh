#!/bin/sh
set -eu

# A merge plus an unmerged topic makes multi-tip reachability and de-duplication observable.
git init -q -b main .
git config user.name author
git config user.email author@example.com
printf '%s\n' 'Mailmapped Author <mapped@example.com> Codex <Codex@OpenAI.com>' >.mailmap

commit () {
  echo "$1" >"$1"
  git add "$1"
  GIT_AUTHOR_DATE="$2 +0000" GIT_COMMITTER_DATE="$2 +0000" git commit -q -m "$1"
}

commit root "2000-01-01T00:00:00"
git tag -a v1 -m v1
git switch -q -c merged
commit merged "2000-01-02T00:00:00"
git switch -q main
commit main "2000-01-03T00:00:00"
git merge -q --no-edit merged
git switch -q -c topic HEAD~1
echo topic >topic
git add topic
GIT_AUTHOR_DATE="2000-01-04T00:00:00 +0000" GIT_COMMITTER_DATE="2000-01-04T00:00:00 +0000" \
  git commit -q --author="Codex <Codex@OpenAI.com>" -m topic \
    -m "--- agent

Co-authored-by: Human Coauthor <human@example.com>
Co-authored-by: Claude <noreply@anthropic.com>
Assisted-by: Opus 4.7
rEvIeWeD-bY: Reviewer <reviewer@example.com>
Acked-by: Acknowledger <ack@example.com>
Tested-by: Tester <tester@example.com>
Signed-off-by: Signer <signer@example.com>
Co-authored-by: Broken <broken@example.com> trailing garbage"
git switch -q main

# Decorations are optional, so this unrelated stale remote HEAD must not break history loading.
git symbolic-ref refs/remotes/origin/HEAD refs/remotes/origin/missing
