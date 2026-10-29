#!/usr/bin/env bash
set -eu

# Adapted from t8013-blame-ignore-revs.sh: changed lines should trace through
# formatting commits, while unrelated additions stay attributed to those commits.
git init --initial-branch=main
printf 'line1\n' >file
git add file
git commit -m A
git tag A
printf 'line2\n' >>file
git commit -am B
git tag B
printf 'line-one\nline-two\n' >file
git commit -am X
git tag X
printf 'line-one-change\nline-two-changed\ny3\ny4\n' >file
git commit -am Y
git tag Y
printf '# formatting revisions\n\n  %s # first rewrite\n%s\n' "$(git rev-parse X)" "$(git rev-parse Y)" >ignored
git config blame.ignoreRevsFile ignored
printf 'not-an-object\n' >invalid
git rev-parse X^{tree} >noncommit
git tag -a -m 'formatting annotation' XT X
git rev-parse X >only-x
git rev-parse Y >only-y
git rev-parse XT >annotated
printf '0000000000000000000000000000000000000000\n' >>noncommit

# The ignored merge must prefer exact matches from any parent, then try each
# parent for fuzzy attribution. The first parent cannot explain the added line.
for mode in exact fuzzy; do
    git init --initial-branch=main "merge-$mode"
    (
        cd "merge-$mode"
        printf 'base\n' >file
        git add file
        git commit -m root
        git branch side
        printf 'QQQQ\n' >file
        git commit -am first-parent
        git checkout side
        printf 'zebra\n' >file
        git commit -am second-parent
        git checkout main
        git merge -s ours --no-commit side
        if test "$mode" = exact; then
            printf 'zebra\n' >file
        else
            printf 'zebra changed\n' >file
        fi
        git commit -am ignored-merge
        git rev-parse HEAD >ignored
        git config blame.ignoreRevsFile ignored
    )
done

# Renaming and reformatting in one commit must keep the source path.
git init --initial-branch=main renamed
(
    cd renamed
    printf 'alpha value\nbeta value\nkeep 1\nkeep 2\nkeep 3\nkeep 4\n' >original
    git add original
    git commit -m original
    git mv original file
    printf 'ALPHA VALUE\nBETA VALUE\nkeep 1\nkeep 2\nkeep 3\nkeep 4\n' >file
    git commit -am formatting-rename
    git rev-parse HEAD >ignored
    git config blame.ignoreRevsFile ignored
)

# Fuzzy matching cases from Git's t8014: joining and splitting lines, repeated
# matches, preserving order, and offset ranges when many lines disappear.
for case in join split repeated order offset; do
    git init --initial-branch=main "fuzzy-$case"
    (
        cd "fuzzy-$case"
        case "$case" in
        join) printf 'if ((maxgrow==0) ||\n\t( single_line_field && (field->dcols < maxgrow)) ||\n\t(!single_line_field && (field->drows < maxgrow)))\n' >file ;;
        split) printf '\tif (rows) *rows = field->rows;\n\tif (cols) *cols = field->cols;\n' >file ;;
        repeated) printf 'abc\nabc\n' >file ;;
        order) printf 'abcdef\nghijkl\nab\n' >file ;;
        offset) printf '1\n2\n3\nabcdef\n5\n6\n7\nijkl\n9\n10\n11\npqrs\n13\n14\n15\nwxyz\n17\n18\n19\n' >file ;;
        esac
        git add file
        git commit -m original
        case "$case" in
        join) printf 'if ((maxgrow == 0) || (single_line_field && (field->dcols < maxgrow)) ||\n\t(!single_line_field && (field->drows < maxgrow))) {\n' >file ;;
        split) printf '\tif (rows) {\n\t\t*rows = field->rows;\n\t}\n\tif (cols) {\n\t\t*cols = field->cols;\n\t}\n' >file ;;
        repeated) printf 'abcd\nabcd\n' >file ;;
        order) printf 'ghijk\nabcd\n' >file ;;
        offset) printf 'abcde\nijk\npqr\nwxy\n' >file ;;
        esac
        git commit -am formatting
        git rev-parse HEAD >ignored
        git config blame.ignoreRevsFile ignored
    )
done

# Insertion-only changes split by a newer change used to produce negative
# parent ranges in Git. They must remain safe and retain unmatched additions.
git init --initial-branch=main split-addition
(
    cd split-addition
    printf 'L1\nL2\nL7\nL8\nL9\n' >file
    git add file
    git commit -m original
    printf 'L1\nL2\nL3\nL4\nL5\nL6\nL7\nL8\nL9\n' >file
    git commit -am insert
    git rev-parse HEAD >ignored
    git config blame.ignoreRevsFile ignored
    printf 'L1\nL2\nL3\nL4\nxxx\nL6\nL7\nL8\nL9\n' >file
    git commit -am newer-change
)
