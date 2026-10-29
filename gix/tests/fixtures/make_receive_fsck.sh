#!/usr/bin/env bash
set -eu -o pipefail

# Raw historical commits exercise receive.fsck.<id> and skip-list policy without
# losing graph connectivity. Unsafe special blobs must be checked even when the
# receiver already has them and the push therefore does not transfer them.
git init -q source
cd source
tree=$(git mktree </dev/null)
printf 'tree %s\nauthor legacy\ncommitter legacy\n\nold object\n' "$tree" |
    git hash-object -w --literally -t commit --stdin >../legacy
printf '[submodule "unsafe"]\npath = sub\nurl = -unsafe\n' |
    git hash-object -w --stdin >../modules
modules=$(cat ../modules)
printf '100644 blob %s\t.gitmodules\n' "$modules" | git mktree >../modules-tree
printf 'special file\n' | git commit-tree "$(cat ../modules-tree)" >../special
printf 'ordinary\n' | git commit-tree "$tree" >../valid
# Demoting a structural warning must still retain the referenced blob.
printf '%s\n' content | git hash-object -w --stdin >../blob
printf '100644 blob %s\t.\n' "$(cat ../blob)" | git mktree >../dot-tree
printf 'dot entry\n' | git commit-tree "$(cat ../dot-tree)" >../dot
# A skip-list exempts historical syntax, never a missing object dependency.
missing=$(printf missing | git hash-object --stdin)
printf '100644 blob %s\tmissing\n' "$missing" | git mktree --missing >../missing-tree
printf 'incomplete graph\n' | git commit-tree "$(cat ../missing-tree)" >../missing
# Ancestry checks must tolerate metadata exempted by receive.fsck policy too.
printf 'tree %s\nparent %s\nauthor legacy\ncommitter legacy\n\nold child\n' "$tree" "$(cat ../valid)" |
    git hash-object -w --literally -t commit --stdin >../legacy-child
printf 'tree %s\nparent %s\nauthor legacy\ncommitter legacy\n\nold descendant\n' "$tree" "$(cat ../legacy)" |
    git hash-object -w --literally -t commit --stdin >../legacy-descendant
printf 'valid child of old metadata\n' | git commit-tree "$tree" -p "$(cat ../legacy)" >../valid-descendant
printf 'valid shallow receiver tip\n' | git commit-tree "$tree" -p "$(cat ../valid)" >../valid-child
