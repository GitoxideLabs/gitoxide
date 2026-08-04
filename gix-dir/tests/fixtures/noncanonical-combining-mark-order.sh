#!/usr/bin/env bash
set -eu -o pipefail

git init tracked
git init untracked

# The Arabic is correct, but some shadda-vowel pairs intentionally use non-canonical code-point order.
# Git's UTF-8-MAC conversion preserves that order; full NFC would change the tracked filename's bytes.
real_world_name="مُنَاقَشَةُ سُبُلِ اِسْتِخْدَامِ اللُّغَةِ فِي النُّظُمِ الْقَائِمَةِ، وَلَا سِيَّمَا فِي التَّطْبِيقَاتِ الْحَاسُوبِيَّةِ"

# The minimal equivalent is ALEF + SHADDA (U+0651) + DAMMA (U+064F).
minimal_name="$(printf '\330\247\331\221\331\217')"

touch "tracked/$real_world_name" "tracked/$minimal_name"
touch "untracked/$real_world_name" "untracked/$minimal_name"
git -C tracked add "$real_world_name" "$minimal_name"
