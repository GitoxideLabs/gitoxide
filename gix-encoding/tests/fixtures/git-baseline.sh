#!/usr/bin/env bash
set -eu -o pipefail

# Git's worktree encoding path calls its iconv wrapper. Disable all other filters
# so these files capture only character conversion, including BOMs and terminal
# shift states. The fixture runs under gix-testtools' isolated Git environment.
# Keep this fixture unarchived: iconv aliases and native-endian output depend on
# the host, and the baseline should exercise the Git installed on that host.
git init -q repo
cd repo
git config core.autocrlf false
git config core.checkRoundtripEncoding ''
mkdir ../cases
case_number=0
printf 'a' >ascii
ascii_blob_id=$(git hash-object --no-filters -w ascii)

encode() {
    local label="$1" content="$2"
    case_number=$((case_number + 1))
    local case_path="../cases/$case_number"
    mkdir "$case_path"
    printf '%s' "$label" >"$case_path/label"
    printf '%s' "$content" >"$case_path/utf8"
    printf 'sample -text working-tree-encoding=%s\n' "$label" >.gitattributes
    local blob_id
    blob_id=$(git hash-object --no-filters -w "$case_path/utf8")
    git cat-file --filters --path=sample "$blob_id" >"$case_path/encoded" 2>"$case_path/error"
    # Checkout reports conversion failures on stderr while returning the input.
    if test -s "$case_path/error"; then
        # Distinguish a missing host codec from an unrepresentable character:
        # every encoding in this fixture can represent the ASCII letter a.
        git cat-file --filters --path=sample "$ascii_blob_id" > /dev/null 2>"$case_path/capability-error"
        if test -s "$case_path/capability-error"; then
            printf 'unsupported\n' >"$case_path/status"
        else
            printf 'invalid-encode\n' >"$case_path/status"
        fi
    else
        printf 'encode\n' >"$case_path/status"
    fi
}

decode() {
    local label="$1" escaped="$2"
    case_number=$((case_number + 1))
    local case_path="../cases/$case_number"
    mkdir "$case_path"
    printf '%s' "$label" >"$case_path/label"
    printf '%b' "$escaped" >"$case_path/encoded"
    printf 'sample -text working-tree-encoding=%s\n' "$label" >.gitattributes
    local blob_id
    if blob_id=$(git hash-object -w --path=sample "$case_path/encoded" 2>"$case_path/error"); then
        git cat-file blob "$blob_id" >"$case_path/utf8"
        printf 'decode\n' >"$case_path/status"
    else
        printf 'invalid\n' >"$case_path/status"
    fi
}

for label in UTF-8 UTF8 UTF-16 UTF-16LE UTF-16BE UTF-16LE-BOM UTF-16BE-BOM UTF-32 UTF-32LE UTF-32BE; do
    encode "$label" $'ASCII\r\nÉté Ελληνικά 日本語 😀\n'
done
for label in ISO-8859-1 latin-1 LATIN-1 latin1 WINDOWS-1252; do
    encode "$label" $'Été à Noël\r\n'
done
encode ASCII $'ASCII\r\n'
# Exercise every single-byte table even when its richer repertoire differs.
for part in 2 3 4 5 6 7 8 9 10 11 13 14 15 16; do
    encode "ISO-8859-$part" 'ASCII table coverage'
done
for label in WINDOWS-1250 WINDOWS-1251 WINDOWS-1253 WINDOWS-1254 WINDOWS-1255 WINDOWS-1256 WINDOWS-1257 WINDOWS-1258 WINDOWS-874 TIS-620 KOI8-U MACINTOSH MAC-CYRILLIC IBM437 IBM850 IBM866 CP1125; do
    encode "$label" 'ASCII table coverage'
done
encode ISO-8859-2 'Příliš žluťoučký kůň'
encode KOI8-R 'Привет мир'
encode SHIFT-JIS '日本語'
encode CP932 '日本語'
encode EUC-JP '日本語'
# No trailing ASCII: Git does not append the optional terminal ASCII shift.
encode ISO-2022-JP '日本語'
encode GBK '中文'
encode GB18030 '中文 😀'
encode BIG5 '中文'
encode BIG5-HKSCS 'Ê̄中文'
encode EUC-KR '한국어'
encode CP949 '한국어'
encode GB2312 '中文'
encode ASCII 'é'
encode ISO-8859-1 '€'
encode WINDOWS-1252 '😀'
encode SHIFT-JIS '😀'
encode UTF-16LE $'x\xff'

# Distinguish POSIX mappings from WHATWG aliases and Microsoft extensions.
for label in ISO-8859-1 latin-1 WINDOWS-1252; do
    decode "$label" '\x80\x91\xa0\xff'
done
for label in SHIFT-JIS CP932; do
    decode "$label" '\x5c\x7e'
    decode "$label" '\x81\x60\x81\x61\x81\x7c\x81\x91\x81\x92\x81\xca'
done
decode UTF-16 '\xff\xfe\x3d\xd8\x00\xde'
decode UTF-16 '\xfe\xff\xd8\x3d\xde\x00'
decode UTF-16LE '\x3d\xd8\x00\xde'
decode UTF-16BE '\xd8\x3d\xde\x00'
decode UTF-16LE-BOM '\xfe\xff\x00\x61'
decode UTF-32 '\xff\xfe\x00\x00\x00\xf6\x01\x00'
decode UTF-32 '\x00\x00\xfe\xff\x00\x01\xf6\x00'
decode UTF-32LE '\x00\xf6\x01\x00'
decode UTF-32BE '\x00\x01\xf6\x00'
# A final pair alone loses its combining mark in Git's unflushed iconv call.
# Include following ASCII here; the streaming tests retain the complete pair at EOF.
decode BIG5-HKSCS '\x88\x62\x78'

# Raw conversion rejects these too: failures do not rely on Git's additional
# working-tree BOM validation. UTF-8 is deliberately absent because Git bypasses
# its encoding filter entirely for that attribute, even for invalid UTF-8.
decode ASCII '\x80'
decode WINDOWS-1252 '\x81'
decode UTF-16LE '\x61'
decode UTF-16BE '\xd8\x00'
decode UTF-16LE '\x00\xdc'
decode UTF-32LE '\x00\x00\x11\x00'
decode UTF-32BE '\x00\x00\xd8\x00'
decode SHIFT-JIS '\x81'
decode SHIFT-JIS '\x80'
decode EUC-JP '\xa1'
decode ISO-2022-JP '\x1b\x24'
decode GB18030 '\x81\x30\x81'
decode BIG5 '\xa1'
