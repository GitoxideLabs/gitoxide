#!/usr/bin/env bash
set -eu -o pipefail

# This fixture creates a single commit, so the `fetch` command called below
# transfers exactly three objects: a commit, a tree, and a blob. 
git init -q repo
cd repo
printf 'transport fixture\n' >file
git add file
git commit -qm 'initial commit'

mkdir .git/transport-baseline
git rev-parse HEAD >.git/transport-baseline/commit-id
git rev-list --objects HEAD | wc -l >.git/transport-baseline/object-count
commit_id=$(git rev-parse HEAD)
hash=$(git rev-parse --show-object-format)

# Packet payloads are ASCII text with a trailing line feed (LF, \n). The length includes the four-byte hexadecimal header.
packet() {
  printf '%04x%s\n' "$((${#1} + 5))" "$1"
}

request_header() {
  packet "command=$1"
  packet 'agent=git/transport-test'
  packet "object-format=$hash"
  printf '0001'
}

# Produce actual requests so we can ask Git to create the baseline.
{
  request_header ls-refs
  packet peel
  packet symrefs
  packet 'ref-prefix HEAD'
  packet 'ref-prefix refs/heads/'
  packet 'ref-prefix refs/tags'
  printf '0000'
} >.git/transport-baseline/ls-refs.request

{
  request_header fetch
  packet thin-pack
  packet ofs-delta
  packet "want $commit_id"
  packet done
  printf '0000'
} >.git/transport-baseline/fetch.request

GIT_PROTOCOL=version=2 git upload-pack --stateless-rpc --advertise-refs . >.git/transport-baseline/v2.response
GIT_PROTOCOL=version=2 git upload-pack --stateless-rpc . <.git/transport-baseline/ls-refs.request >>.git/transport-baseline/v2.response
GIT_PROTOCOL=version=2 git upload-pack --stateless-rpc . <.git/transport-baseline/fetch.request >>.git/transport-baseline/v2.response
