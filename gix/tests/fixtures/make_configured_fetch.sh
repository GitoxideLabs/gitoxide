#!/usr/bin/env bash
set -eu -o pipefail

# Named remotes override global pruning policy; tag pruning also fetches tags
# even with --no-tags. Clone each client before deleting source refs.
git init -q --initial-branch=main --object-format="${1:?hash algorithm}" source
git -C source commit -qm initial --allow-empty
git -C source branch removed
git -C source tag removed
git -C source tag retained
git clone -q source client
git -C client config user.name fixture
git -C client config user.email fixture@example.com
git -C source branch -D removed
git -C source tag -d removed
git -C source tag added
for case in global remote no-prune prune-tags remote-tags-disabled; do
    cp -R client "native-$case"
    case "$case" in
        global)
            git -C "native-$case" config fetch.prune true
            ;;
        remote)
            git -C "native-$case" config fetch.prune false
            git -C "native-$case" config remote.origin.prune true
            ;;
        no-prune)
            git -C "native-$case" config fetch.prune true
            git -C "native-$case" config remote.origin.prune false
            ;;
        prune-tags)
            git -C "native-$case" config fetch.prune true
            git -C "native-$case" config fetch.pruneTags true
            git -C "native-$case" config remote.origin.tagOpt --no-tags
            ;;
        remote-tags-disabled)
            git -C "native-$case" config fetch.prune true
            git -C "native-$case" config fetch.pruneTags true
            git -C "native-$case" config remote.origin.pruneTags false
            ;;
    esac
    cp -R "native-$case" "git-$case"
    git -C "git-$case" fetch -q origin
done
