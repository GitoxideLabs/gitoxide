#!/usr/bin/env bash
set -eu -o pipefail

# Exercise Git's paragraph heuristic with generic and recognized trailers,
# its 25% cutoff, interleaved prose, continuation lines and non-UTF-8 bytes.
mkdir paragraphs baselines
baseline() {
    printf '%b' "$2" >"paragraphs/$1"
    # A subject makes each paragraph the body of a complete commit message.
    { printf 'subject\n\n'; cat "paragraphs/$1"; } | git interpret-trailers --parse >"baselines/$1"
}

baseline empty ''
baseline whitespace ' \t\r\n'
baseline generic-trailers 'Reqwest-Patch: patch
Reqwest-PR: https://example.com/pr/1'
baseline generic-trailer-with-prose 'Note: this is prose
more explanation'
baseline prose-with-generic-trailer 'some body text
token: value'
baseline indented-prose ' indented: prose
more explanation'
baseline recognized-trailer-at-cutoff 'Signed-off-by: Alice
not a trailer 1
not a trailer 2
not a trailer 3'
baseline recognized-trailer-below-cutoff 'Signed-off-by: Alice
not a trailer 1
not a trailer 2
not a trailer 3
not a trailer 4'
baseline interleaved-trailers-and-prose 'not a trailer
Signed-off-by: Alice
another note
Acked-by: Bob
 continuation'
baseline cherry-pick-marker '(cherry picked from commit 0123456789012345678901234567890123456789)
Reqwest-PR: https://example.com/pr/1
not a trailer'
baseline space-before-separator 'Acked-by : Alice
 continuation line'
baseline tab-continuation 'token: value
\tcontinuation
'
baseline non-utf8 'Review: \xff'
