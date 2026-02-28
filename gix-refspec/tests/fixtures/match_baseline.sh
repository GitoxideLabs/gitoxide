#!/usr/bin/env bash
set -eu -o pipefail

git init;

function baseline() {
  {
    git fetch --refmap= --dry-run -v origin "$@" 2>&1 || :
    echo specs: "$@"
  } >> baseline.git
}

mkdir base
(cd base
  git init
  touch file
  git add file
  git commit -m "initial commit"
  git tag -m "message" annotated-v0.0

  git checkout -b f1
  git commit -m "f1" --allow-empty
  git tag v0.0-f1

  git checkout -b f2 main
  git commit -m "f2" --allow-empty
  git tag v0.0-f2

  git checkout -b f3 main
  git commit -m "f3" --allow-empty
  git tag v0.0-f3

  git checkout -b sub/f4 main
  git checkout -b sub/subdir/f5 main
  git checkout -b suub/f6 main
)

git clone --shared ./base clone
(cd clone
  git ls-remote 2>&1 > remote-refs.list
  baseline "refs/heads/main"
  baseline "heads/main"
  baseline "main"
  baseline "v0.0-f1"
  baseline "tags/v0.0-f2"
  case $GIX_TEST_FIXTURE_HASH in
    sha1)
      baseline "78b1c1be9421b33a49a7a8176d93eeeafa112da1"
      baseline "9d2fab1a0ba3585d0bc50922bfdd04ebb59361df"
      baseline "78b1c1be9421b33a49a7a8176d93eeeafa112da1:special"
      baseline "78b1c1be9421b33a49a7a8176d93eeeafa112da1:1111111111111111111111111111111111111111"
      baseline "9d2fab1a0ba3585d0bc50922bfdd04ebb59361df:tags/special"
      baseline "9d2fab1a0ba3585d0bc50922bfdd04ebb59361df:refs/tags/special" ;;
    sha256)
      baseline "b071221ea854da2958fba3a37527ca5cf32c4ebcd71ab0b68b6b8f10f04e93ad"
      baseline "ac050883b75422e0d03bfee760c591b292cbc10cee8ad934480ea5fb2ebc44fe"
      baseline "b071221ea854da2958fba3a37527ca5cf32c4ebcd71ab0b68b6b8f10f04e93ad:special"
      baseline "b071221ea854da2958fba3a37527ca5cf32c4ebcd71ab0b68b6b8f10f04e93ad:1111111111111111111111111111111111111111"
      baseline "ac050883b75422e0d03bfee760c591b292cbc10cee8ad934480ea5fb2ebc44fe:tags/special"
      baseline "ac050883b75422e0d03bfee760c591b292cbc10cee8ad934480ea5fb2ebc44fe:refs/tags/special" ;;
    *)
      exit 1 ;;
  esac
  baseline "f1:origin/f1"
  baseline "f1:remotes/origin/f1"
  baseline "f1:notes/f1"
  baseline "+refs/heads/*:refs/remotes/origin/*"
  baseline "refs/heads/*1:refs/remotes/origin/*1"
  baseline "refs/heads/f*:refs/remotes/origin/a*"
  baseline "refs/heads/*/f6:refs/remotes/origin/*/f6"
  baseline "main" "f1"
  baseline "heads/main" "heads/f1"
  baseline "refs/heads/main" "refs/heads/f1"
  baseline "refs/heads/*:refs/remotes/origin/*" "^main"
  baseline "heads/f1" "f2" "refs/heads/f3" "heads/main"
  baseline "f*:a*" "refs/heads/main"
  baseline "refs/heads/f*:refs/remotes/origin/a*" "^f1"
  baseline "refs/heads/f*:refs/remotes/origin/a*" "^refs/heads/f1"
  baseline "^heads/f2" "refs/heads/f*:refs/remotes/origin/a*"
  baseline "heads/f2" "^refs/heads/f*:refs/remotes/origin/a*"
  baseline "^refs/heads/f2" "refs/heads/f*:refs/remotes/origin/a*"
  baseline "^main" "refs/heads/*:refs/remotes/origin/*"
  baseline "^refs/heads/main" "refs/heads/*:refs/remotes/origin/*"
  baseline "refs/heads/*:refs/remotes/origin/*" "^refs/heads/main"
  baseline "refs/heads/*:refs/remotes/origin/*" "refs/heads/main:refs/remotes/new-origin/main"
  baseline "refs/heads/*:refs/remotes/origin/*" "refs/heads/main:refs/remotes/origin/main"
  baseline "refs/heads/f1:refs/remotes/origin/conflict" "refs/heads/f2:refs/remotes/origin/conflict"
  baseline "refs/heads/f1:refs/remotes/origin/conflict2" "refs/heads/f2:refs/remotes/origin/conflict2" "refs/heads/f1:refs/remotes/origin/conflict" "refs/heads/f2:refs/remotes/origin/conflict" "refs/heads/f3:refs/remotes/origin/conflict"
  baseline "refs/heads/f1:refs/remotes/origin/same" "refs/tags/v0.0-f1:refs/remotes/origin/same" # same object, not technically a problem but git flags it anyway
  baseline "refs/tags/*:refs/remotes/origin/*" "refs/heads/*:refs/remotes/origin/*"
  baseline "refs/tags/*:refs/tags/*"
  baseline 'refs/heads/f*:foo/f*' 'f1:f1'
  baseline "+refs/heads/*:refs/remotes/origin/*" "refs/heads/f1:refs/remotes/origin/f2" "refs/heads/f2:refs/remotes/origin/f1"
  baseline ':refs/heads/f1'
  baseline ':f1'
  baseline ':'
  baseline 'HEAD:'
  baseline '@:'
  baseline '@:f1'
  baseline '@:HEAD'
)

