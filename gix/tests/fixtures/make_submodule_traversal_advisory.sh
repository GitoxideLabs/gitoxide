#!/usr/bin/env bash
set -eu -o pipefail

malicious_name='../../../escaped-target.git'

git init -q victim-repo
(cd victim-repo
  echo f002 > README.md
  git add README.md
  git commit -q -m initial
  mkdir -p .git/modules
)

git init -q attacker-src
(cd attacker-src
  echo attacker-controlled-submodule-repo > OWNED.txt
  git add OWNED.txt
  git commit -q -m "attacker repo seed"
)

git clone --bare -q attacker-src escaped-target.git

mkdir -p victim-repo/deps/demo
cat >victim-repo/.gitmodules <<EOF
[submodule "${malicious_name}"]
	path = deps/demo
	url = ./ignored.git
EOF
