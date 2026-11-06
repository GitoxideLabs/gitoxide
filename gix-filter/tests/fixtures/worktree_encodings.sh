#!/usr/bin/env bash
set -eu -o pipefail

# Exercise Git's worktree policy in addition to iconv itself: generic UTF encodings
# require a BOM, explicit endian variants forbid either BOM, empty files bypass
# conversion, and selected encodings must survive a byte-exact round trip.
# Keep the fixture unarchived because Git's generic UTF output is host dependent.
git init -q repo
cd repo
git config core.autocrlf false
mkdir ../cases
case_number=0

checkin() {
    local label="$1" escaped="$2" roundtrip="${3:-skip}"
    case_number=$((case_number + 1))
    local case_path="../cases/$case_number"
    mkdir "$case_path"
    printf '%s' "$label" >"$case_path/label"
    printf '%s' "$roundtrip" >"$case_path/roundtrip"
    printf '%b' "$escaped" >"$case_path/input"
    printf 'sample -text working-tree-encoding=%s\n' "$label" >.gitattributes
    if test "$roundtrip" = check; then
        git config core.checkRoundtripEncoding "$label"
    else
        git config core.checkRoundtripEncoding ''
    fi
    local blob_id
    if blob_id=$(git hash-object -w --path=sample "$case_path/input" 2>"$case_path/error"); then
        git cat-file blob "$blob_id" >"$case_path/expected"
        printf 'accept' >"$case_path/status"
    else
        printf 'reject' >"$case_path/status"
    fi
}

for label in UTF-16 UTF-16LE UTF-16BE UTF-16LE-BOM UTF-16BE-BOM UTF-32 UTF-32LE UTF-32BE; do
    checkin "$label" '' check
done
checkin UTF-16 '\xff\xfe\x61\0'
checkin UTF-16 '\xfe\xff\0\x61'
checkin UTF-16 '\x61\0'
checkin UTF-16 '\0\x61'
checkin UTF-16LE '\xff\xfe\x61\0'
checkin UTF-16LE '\xfe\xff\0\x61'
checkin UTF-16BE '\xff\xfe\x61\0'
checkin UTF-16BE '\xfe\xff\0\x61'
checkin UTF-16LE '\x61\0' check
checkin UTF-16BE '\0\x61' check
# PR #3163's ASCII, Japanese BMP character, and surrogate pair in each byte order.
checkin UTF-16LE '\x68\0\x69\0\x20\0\x42\x30\x3d\xd8\0\xde\x0a\0' check
checkin UTF-16BE '\0\x68\0\x69\0\x20\x30\x42\xd8\x3d\xde\0\0\x0a' check
# The review's exact opposite-endian BOM example must fail despite round-trip equality.
checkin UTF-16BE '\xff\xfe\x41\0' check
checkin UTF-16 '\xff\xfe\x61\0' check
checkin UTF-16 '\xfe\xff\0\x61' check
checkin UTF-16LE-BOM '\xff\xfe\x61\0' check
checkin UTF-16LE-BOM '\xfe\xff\0\x61' check
checkin UTF-16LE-BOM '\x61\0'
checkin UTF-16BE-BOM '\xfe\xff\0\x61'
checkin UTF-32 '\xff\xfe\0\0\x61\0\0\0'
checkin UTF-32 '\0\0\xfe\xff\0\0\0\x61'
checkin UTF-32 '\x61\0\0\0'
checkin UTF-32LE '\xff\xfe\0\0\x61\0\0\0'
checkin UTF-32LE '\0\0\xfe\xff\0\0\0\x61'
checkin UTF-32BE '\xff\xfe\0\0\x61\0\0\0'
checkin UTF-32BE '\0\0\xfe\xff\0\0\0\x61'
checkin UTF-32LE '\x61\0\0\0' check
checkin UTF-32BE '\0\0\0\x61' check
checkin UTF-32 '\xff\xfe\0\0\x61\0\0\0' check
checkin UTF-32 '\0\0\xfe\xff\0\0\0\x61' check
checkin ISO-8859-1 '\x80\x91\xff' check
checkin latin-1 '\x80\x91\xff' check
checkin WINDOWS-1252 '\x80\x91\xff' check
checkin SHIFT-JIS '\x5c\x7e\x93\xfa\x96\x7b\x8c\xea' check
