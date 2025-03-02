#!/usr/bin/env bash
set -eu -o pipefail

git init;

function baseline() {
  local specs=""
  for arg in "$@"; do
    if [[ $arg == *"+"* ]]; then
      echo "BUG: Argument '$arg' contains a space - we use that for separating pathspecs right now" >&2
      exit 1
    fi
    specs="${specs}+${arg}"
  done

  {
      echo "$specs"
      git ls-files "$@"
      echo -n ';'
  } >> baseline.git
}

: >goo
: >'g[o][o]'
: >bar
echo 'goo a !b c=v -d' > .gitattributes
mkdir sub && :>sub/bar
git add . && git commit -m init
# this is to avoid issues on windows, which might not be able to manifest these files.
git -c core.protectNTFS=false update-index --add --cacheinfo 100644 "$(git rev-parse HEAD:goo)" "g*"
git update-index --add --cacheinfo 100644 "$(git rev-parse HEAD:goo)" "!a"
for p in bar bAr BAR foo/bar foo/bAr foo/BAR fOo/bar fOo/bAr fOo/BAR FOO/bar FOO/bAr FOO/BAR; do
  git -c core.ignoreCase=false update-index --add --cacheinfo 100644 "$(git rev-parse HEAD:goo)" "$p"
done
git -c core.protectNTFS=false update-index --add --cacheinfo 100644 "$(git rev-parse HEAD:goo)" "    " # 4 x space
git -c core.protectNTFS=false update-index --add --cacheinfo 100644 "$(git rev-parse HEAD:goo)" "  hi  " # 2 x space hi 2 x space

git ls-files > paths

baseline ':(attr:a)goo'
baseline ':(attr:a)Goo'
baseline ':(icase,attr:a)Goo'
baseline ':(attr:!b)goo'
baseline ':(attr:c=v)goo'
baseline ':(attr:-d)goo'
baseline ':(attr:a !b c=v -d)goo'
baseline ':(icase,attr:a !b c=v -d)GOO'
baseline ':(attr:a !b c=v -d)g*'
baseline ':(attr:none)goo'
baseline ':(literal)g*'
baseline 'sub/'
baseline 'sub'
baseline 'sub/*'
baseline 'sub*'
baseline ':(literal)g*'
baseline ':(glob)g*'
baseline ':(exclude,literal)g*'
baseline 'g*'
baseline ':(exclude)g*'
baseline ':(literal)?*'
baseline ':(exclude,literal)?*'
baseline '?*'
baseline ':(exclude)?*'
baseline 'g[o][o]'
# ["g[o][o]", "goo"] == ["g[o][o]"]
baseline ':(icase)G[O][o]' git-inconsistency
baseline ':(literal)g[o][o]'
baseline ':(literal,icase)G[o][O]'
baseline ':(glob)g[o][o]'
# ["g[o][o]", "goo"] == ["g[o][o]"]
baseline ':(glob,icase)g[o][O]' git-inconsistency
baseline ':(glob)**/bar'
baseline ':(literal)**/bar'
baseline '**/bar'
baseline '*/bar'
baseline ':(glob)*bar'
baseline ':(glob)**bar'
baseline '*bar'
baseline '*bar*'
baseline 'bar'
baseline 'bar/'
baseline 'sub/bar/'
baseline 'sub/bar'
baseline '!a'
baseline '?a'
baseline 'foo/'
baseline 'foo'
baseline 'foo/*'
baseline 'foo*'
baseline ':(icase)foo/'
baseline ':(icase)foo'
baseline ':(icase)foo/*'
baseline ':(icase)foo*'
baseline ':(icase)foo/bar'
