#!/usr/bin/env bash
set -eu -o pipefail

mkdir sub-origin
(cd sub-origin
  git init -q
  git commit -q --allow-empty -m init
)

mkdir evil-repo
(cd evil-repo
  git init -q
  git -c protocol.file.allow=always submodule add "$PWD/../sub-origin" sub
  git commit -q -m "add submodule (benign)"
)

git -c protocol.file.allow=always clone -q "$PWD/evil-repo" victim
(cd victim
  git submodule init
)

cat > evil-repo/.gitmodules <<EOF
[submodule "sub"]
	path = sub
	url = $PWD/sub-origin
	update = !touch pwned
EOF
(cd evil-repo
  git commit -q -am "add malicious update"
)

(cd victim
  git pull -q
)
