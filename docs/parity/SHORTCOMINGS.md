# Parity Shortcomings Ledger

Auto-generated from `tests/journey/parity/*.sh` by `etc/parity/shortcomings.sh`.
**Do not edit by hand** — re-run the generator.

Two row classes:
- **deferred** — `shortcoming "<reason>"`: row closed as a legitimate deferral; reason describes the gap.
- **compat** — `compat_effect "<reason>"`: row green at effect mode (exit-code parity); byte-output parity is a known follow-up.

## branch

| Class | Section | Reason | Source |
|---|---|---|---|
| compat | `gix branch --verbose` | branch -v column-aligned sha+subject rendering deferred | [branch.sh:175](../../tests/journey/parity/branch.sh#L175) |
| compat | `gix branch --verbose` | branch -vv upstream tracking rendering deferred | [branch.sh:178](../../tests/journey/parity/branch.sh#L178) |
| compat | `gix branch --abbrev` | branch -v --abbrev=<n> bytes parity follows -v renderer | [branch.sh:210](../../tests/journey/parity/branch.sh#L210) |
| compat | `gix branch --abbrev` | branch -v --no-abbrev bytes parity follows -v renderer | [branch.sh:213](../../tests/journey/parity/branch.sh#L213) |
| compat | `gix branch --column` | branch --column=always packing deferred | [branch.sh:309](../../tests/journey/parity/branch.sh#L309) |

## clone

| Class | Section | Reason | Source |
|---|---|---|---|
| deferred | `gix clone -b / --branch=<name>` | deferred: gix::clone::Fetch ref-not-found error maps to exit 1; git exits 128 | [clone.sh:312](../../tests/journey/parity/clone.sh#L312) |
| deferred | `gix clone --revision=<rev>` | deferred: --revision is vendor-only; system git 2.47 rejects it | [clone.sh:327](../../tests/journey/parity/clone.sh#L327) |
| deferred | `gix clone --revision=<rev> --branch=<name> (conflict)` | deferred: --revision is vendor-only; conflict row depends on it | [clone.sh:334](../../tests/journey/parity/clone.sh#L334) |
| deferred | `gix clone --revision=<rev> --mirror (conflict)` | deferred: --revision is vendor-only; conflict row depends on it | [clone.sh:341](../../tests/journey/parity/clone.sh#L341) |
| deferred | `gix clone --reference=<repo>` | deferred: gix doesn't write objects/info/alternates; --reference is parse-only | [clone.sh:457](../../tests/journey/parity/clone.sh#L457) |
| deferred | `gix clone --reference-if-able=<repo>` | deferred: parent --reference is not yet wired; --reference-if-able inherits the deferral | [clone.sh:470](../../tests/journey/parity/clone.sh#L470) |
| deferred | `gix clone --dissociate --reference=<repo>` | deferred: depends on --reference alternates wiring | [clone.sh:482](../../tests/journey/parity/clone.sh#L482) |
| deferred | `gix clone --depth=0 (non-positive)` | deferred: Clap's NonZeroU32 parser exits 2; git exits 128 with a fatal | [clone.sh:520](../../tests/journey/parity/clone.sh#L520) |
| deferred | `gix clone --shallow-since=<time>` | deferred: gix-protocol shallow-since decoder returns 'Could not decode server reply' | [clone.sh:533](../../tests/journey/parity/clone.sh#L533) |
| deferred | `gix clone --shallow-exclude=<ref>` | deferred: gix-protocol deepen-not opcode alignment (same gap as fetch.sh --shallow-exclude) | [clone.sh:545](../../tests/journey/parity/clone.sh#L545) |

## fetch

| Class | Section | Reason | Source |
|---|---|---|---|
| deferred | `gix fetch --shallow-exclude=<ref>` | --shallow-exclude semantic parity needs gix-protocol deepen-not alignment | [fetch.sh:497](../../tests/journey/parity/fetch.sh#L497) |
| deferred | `gix fetch --unshallow` | happy-path parity needs expect_parity to reset fixtures between the git and gix invocations for stateful ops | [fetch.sh:512](../../tests/journey/parity/fetch.sh#L512) |
| deferred | `gix fetch --negotiate-only` | --negotiate-only needs ack-only-capability enforcement in gix-protocol | [fetch.sh:546](../../tests/journey/parity/fetch.sh#L546) |
| deferred | `gix fetch --multiple` | --multiple needs Clap-level or dispatch-level remapping of positionals to remote-names | [fetch.sh:641](../../tests/journey/parity/fetch.sh#L641) |

## log

| Class | Section | Reason | Source |
|---|---|---|---|
| compat | `gix log --not <ref>` | gix log --not multi-revspec state-flipper deferred — first revspec honored, --not flag accepted | [log.sh:226](../../tests/journey/parity/log.sh#L226) |
| compat | `gix log -- <path>` | gix log -- <path> pathspec filtering deferred — flag accepted, full traversal emitted | [log.sh:239](../../tests/journey/parity/log.sh#L239) |
| compat | `gix log <ref> -- <path>` | gix log <ref> -- <path> pathspec filter on revspec deferred — clap accepted | [log.sh:249](../../tests/journey/parity/log.sh#L249) |
| compat | `gix log -- <missing-path>` | gix log -- <missing-path> pathspec filter deferred — empty match exit-0 parity holds | [log.sh:260](../../tests/journey/parity/log.sh#L260) |
| compat | `gix log --since=<time>` | gix log --since filter deferred — flag accepted, no date predicate applied | [log.sh:313](../../tests/journey/parity/log.sh#L313) |
| compat | `gix log --until=<time>` | gix log --until filter deferred — flag accepted, no date predicate applied | [log.sh:322](../../tests/journey/parity/log.sh#L322) |
| compat | `gix log --author=<pattern>` | gix log --author filter deferred — flag accepted, no regex applied to authors | [log.sh:331](../../tests/journey/parity/log.sh#L331) |
| compat | `gix log --committer=<pattern>` | gix log --committer filter deferred — flag accepted, no regex applied to committers | [log.sh:340](../../tests/journey/parity/log.sh#L340) |
| compat | `gix log --grep=<pattern>` | gix log --grep filter deferred — flag accepted, no regex applied to messages | [log.sh:349](../../tests/journey/parity/log.sh#L349) |
| compat | `gix log -i --grep=<pattern>` | gix log -i --grep case-insensitive match deferred — flag accepted | [log.sh:358](../../tests/journey/parity/log.sh#L358) |
| compat | `gix log --invert-grep --grep=<pattern>` | gix log --invert-grep filter deferred — flag accepted | [log.sh:367](../../tests/journey/parity/log.sh#L367) |
| compat | `gix log --all-match --grep=<p1> --grep=<p2>` | gix log --all-match multi-grep AND-semantics deferred — flag accepted | [log.sh:376](../../tests/journey/parity/log.sh#L376) |
| compat | `gix log -E --grep=<regex>` | gix log -E POSIX extended regex deferred — flag accepted | [log.sh:385](../../tests/journey/parity/log.sh#L385) |
| compat | `gix log -F --grep=<literal>` | gix log -F literal-string match deferred — flag accepted | [log.sh:394](../../tests/journey/parity/log.sh#L394) |
| compat | `gix log --oneline` | gix log --oneline pretty-format divergence deferred — clap accepted | [log.sh:428](../../tests/journey/parity/log.sh#L428) |
| compat | `gix log --pretty=oneline` | gix log --pretty=oneline format emission deferred — clap accepted | [log.sh:439](../../tests/journey/parity/log.sh#L439) |
| compat | `gix log --pretty=short` | gix log --pretty=short format emission deferred — clap accepted | [log.sh:448](../../tests/journey/parity/log.sh#L448) |
| compat | `gix log --pretty=medium` | gix log --pretty=medium format emission deferred — clap accepted | [log.sh:457](../../tests/journey/parity/log.sh#L457) |
| compat | `gix log --pretty=full` | gix log --pretty=full format emission deferred — clap accepted | [log.sh:466](../../tests/journey/parity/log.sh#L466) |
| compat | `gix log --pretty=fuller` | gix log --pretty=fuller format emission deferred — clap accepted | [log.sh:475](../../tests/journey/parity/log.sh#L475) |
| compat | `gix log --pretty=raw` | gix log --pretty=raw format emission deferred — clap accepted | [log.sh:485](../../tests/journey/parity/log.sh#L485) |
| compat | `gix log --pretty=reference` | gix log --pretty=reference format emission deferred — clap accepted | [log.sh:494](../../tests/journey/parity/log.sh#L494) |
| compat | `gix log --format=%H` | gix log --format=<fmt> custom formatter deferred — clap accepted | [log.sh:505](../../tests/journey/parity/log.sh#L505) |
| compat | `gix log --format='%h %s'` | gix log --format=<fmt> custom formatter deferred — clap accepted | [log.sh:515](../../tests/journey/parity/log.sh#L515) |
| compat | `gix log --abbrev-commit` | gix log --abbrev-commit hash-width control deferred — clap accepted | [log.sh:525](../../tests/journey/parity/log.sh#L525) |
| compat | `gix log --no-abbrev-commit` | gix log --no-abbrev-commit hash-width control deferred — clap accepted | [log.sh:535](../../tests/journey/parity/log.sh#L535) |
| compat | `gix log --abbrev=<n>` | gix log --abbrev=<n> hash-width control deferred — clap accepted | [log.sh:545](../../tests/journey/parity/log.sh#L545) |
| compat | `gix log --reverse` | gix log --reverse output-order reversal deferred — flag accepted | [log.sh:556](../../tests/journey/parity/log.sh#L556) |
| compat | `gix log --topo-order` | gix log --topo-order already-default for gix's topo walker — flag accepted | [log.sh:565](../../tests/journey/parity/log.sh#L565) |
| compat | `gix log --date-order` | gix log --date-order commit-date ordering deferred — flag accepted | [log.sh:574](../../tests/journey/parity/log.sh#L574) |
| compat | `gix log --author-date-order` | gix log --author-date-order author-date ordering deferred — flag accepted | [log.sh:583](../../tests/journey/parity/log.sh#L583) |
| compat | `gix log --first-parent` | gix log --first-parent merge-parent selection deferred — flag accepted | [log.sh:592](../../tests/journey/parity/log.sh#L592) |
| compat | `gix log --decorate` | gix log --decorate ref-name emission deferred — clap accepted | [log.sh:626](../../tests/journey/parity/log.sh#L626) |
| compat | `gix log --decorate=short` | gix log --decorate=short ref-name emission deferred — clap accepted | [log.sh:635](../../tests/journey/parity/log.sh#L635) |
| compat | `gix log --decorate=full` | gix log --decorate=full ref-name emission deferred — clap accepted | [log.sh:644](../../tests/journey/parity/log.sh#L644) |
| compat | `gix log --decorate=no` | gix log --decorate=no suppressed (never emitted decorations anyway) — clap accepted | [log.sh:653](../../tests/journey/parity/log.sh#L653) |
| compat | `gix log --no-decorate` | gix log --no-decorate suppressed (never emitted decorations anyway) — clap accepted | [log.sh:662](../../tests/journey/parity/log.sh#L662) |
| compat | `gix log --decorate-refs=<pattern>` | gix log --decorate-refs filter deferred — clap accepted | [log.sh:672](../../tests/journey/parity/log.sh#L672) |
| compat | `gix log --decorate-refs-exclude=<pattern>` | gix log --decorate-refs-exclude filter deferred — clap accepted | [log.sh:682](../../tests/journey/parity/log.sh#L682) |
| compat | `gix log --clear-decorations` | gix log --clear-decorations decoration-filter reset deferred — clap accepted | [log.sh:692](../../tests/journey/parity/log.sh#L692) |
| compat | `gix log --source` | gix log --source ref-prefix emission deferred — clap accepted | [log.sh:702](../../tests/journey/parity/log.sh#L702) |
| compat | `gix log --graph` | gix log --graph ASCII-art commit graph deferred — clap accepted | [log.sh:713](../../tests/journey/parity/log.sh#L713) |
| compat | `gix log -p` | gix log -p per-commit diff emission deferred — clap accepted | [log.sh:724](../../tests/journey/parity/log.sh#L724) |
| compat | `gix log -s` | gix log -s --no-patch diff suppression (vs -p) deferred — clap accepted | [log.sh:733](../../tests/journey/parity/log.sh#L733) |
| compat | `gix log --stat` | gix log --stat diffstat emission deferred — clap accepted | [log.sh:742](../../tests/journey/parity/log.sh#L742) |
| compat | `gix log --shortstat` | gix log --shortstat summary emission deferred — clap accepted | [log.sh:751](../../tests/journey/parity/log.sh#L751) |
| compat | `gix log --numstat` | gix log --numstat machine-readable diffstat deferred — clap accepted | [log.sh:760](../../tests/journey/parity/log.sh#L760) |
| compat | `gix log --name-only` | gix log --name-only path listing deferred — clap accepted | [log.sh:769](../../tests/journey/parity/log.sh#L769) |
| compat | `gix log --name-status` | gix log --name-status status-letter path listing deferred — clap accepted | [log.sh:778](../../tests/journey/parity/log.sh#L778) |
| compat | `gix log --raw` | gix log --raw diff emission deferred — clap accepted | [log.sh:787](../../tests/journey/parity/log.sh#L787) |
| compat | `gix log -M` | gix log -M/--find-renames rename detection deferred — clap accepted | [log.sh:796](../../tests/journey/parity/log.sh#L796) |
| compat | `gix log --follow <file>` | gix log --follow rename-following deferred — flag accepted, positional treated as pathspec hint | [log.sh:811](../../tests/journey/parity/log.sh#L811) |
| compat | `gix log --full-diff -- <path>` | gix log --full-diff diff-scope override deferred — clap accepted | [log.sh:821](../../tests/journey/parity/log.sh#L821) |
| compat | `gix log -L <start>,<end>:<file>` | gix log -L line-range traversal deferred — flag accepted, range parser and file-lookup unwired | [log.sh:834](../../tests/journey/parity/log.sh#L834) |
| compat | `gix log --date=relative` | gix log --date=relative date-format emission deferred — clap accepted | [log.sh:845](../../tests/journey/parity/log.sh#L845) |
| compat | `gix log --date=iso` | gix log --date=iso date-format emission deferred — clap accepted | [log.sh:854](../../tests/journey/parity/log.sh#L854) |
| compat | `gix log --date=short` | gix log --date=short date-format emission deferred — clap accepted | [log.sh:863](../../tests/journey/parity/log.sh#L863) |
| compat | `gix log --date=raw` | gix log --date=raw date-format emission deferred — clap accepted | [log.sh:872](../../tests/journey/parity/log.sh#L872) |
| compat | `gix log --date=unix` | gix log --date=unix date-format emission deferred — clap accepted | [log.sh:881](../../tests/journey/parity/log.sh#L881) |
| compat | `gix log --date=format:<strftime>` | gix log --date=format:<strftime> strftime emission deferred — clap accepted | [log.sh:890](../../tests/journey/parity/log.sh#L890) |
| compat | `gix log -m` | gix log -m per-parent merge diff deferred — clap accepted | [log.sh:901](../../tests/journey/parity/log.sh#L901) |
| compat | `gix log -c` | gix log -c combined merge diff deferred — clap accepted | [log.sh:910](../../tests/journey/parity/log.sh#L910) |
| compat | `gix log --cc` | gix log --cc dense combined merge diff deferred — clap accepted | [log.sh:919](../../tests/journey/parity/log.sh#L919) |
| compat | `gix log --diff-merges=off` | gix log --diff-merges=off merge-diff control deferred — clap accepted | [log.sh:928](../../tests/journey/parity/log.sh#L928) |
| compat | `gix log --diff-merges=first-parent` | gix log --diff-merges=first-parent merge-diff control deferred — clap accepted | [log.sh:937](../../tests/journey/parity/log.sh#L937) |
| compat | `gix log --mailmap` | gix log --mailmap author/committer rewriting deferred — clap accepted | [log.sh:948](../../tests/journey/parity/log.sh#L948) |
| compat | `gix log --no-mailmap` | gix log --no-mailmap .mailmap bypass deferred — clap accepted | [log.sh:957](../../tests/journey/parity/log.sh#L957) |
| compat | `gix log --log-size` | gix log --log-size message-length header deferred — clap accepted | [log.sh:966](../../tests/journey/parity/log.sh#L966) |
| compat | `gix log --notes` | gix log --notes refs/notes emission deferred — clap accepted | [log.sh:975](../../tests/journey/parity/log.sh#L975) |
| compat | `gix log --no-notes` | gix log --no-notes refs/notes suppression deferred — clap accepted | [log.sh:985](../../tests/journey/parity/log.sh#L985) |
| compat | `gix log --show-signature` | gix log --show-signature GPG verification deferred — clap accepted | [log.sh:994](../../tests/journey/parity/log.sh#L994) |
| compat | `gix log --color=always` | gix log --color=always ANSI color emission deferred — clap accepted | [log.sh:1005](../../tests/journey/parity/log.sh#L1005) |
| compat | `gix log --no-color` | gix log --no-color color-suppression deferred (never emitted colors anyway) — clap accepted | [log.sh:1014](../../tests/journey/parity/log.sh#L1014) |
| compat | `gix log --boundary` | gix log --boundary + --not boundary-marker emission deferred — clap accepted | [log.sh:1025](../../tests/journey/parity/log.sh#L1025) |
| compat | `gix log --ancestry-path` | gix log --ancestry-path ancestry filter deferred — clap accepted | [log.sh:1034](../../tests/journey/parity/log.sh#L1034) |
| compat | `gix log -G<regex>` | gix log -G pickaxe line-add/remove regex deferred — flag accepted | [log.sh:1045](../../tests/journey/parity/log.sh#L1045) |
| compat | `gix log -S<string>` | gix log -S pickaxe occurrence-count deferred — flag accepted | [log.sh:1054](../../tests/journey/parity/log.sh#L1054) |
| compat | `gix log --pickaxe-regex` | gix log --pickaxe-regex -S-as-regex mode deferred — flag accepted | [log.sh:1063](../../tests/journey/parity/log.sh#L1063) |
| compat | `gix log --pickaxe-all` | gix log --pickaxe-all merge-inclusion in pickaxe deferred — flag accepted | [log.sh:1072](../../tests/journey/parity/log.sh#L1072) |
| compat | `gix log --cherry` | gix log --cherry patch-equivalence detection deferred — flag accepted | [log.sh:1083](../../tests/journey/parity/log.sh#L1083) |
| compat | `gix log --cherry-mark` | gix log --cherry-mark equivalence-class annotation deferred — flag accepted | [log.sh:1092](../../tests/journey/parity/log.sh#L1092) |
| compat | `gix log --cherry-pick` | gix log --cherry-pick equivalence-class filter deferred — flag accepted | [log.sh:1101](../../tests/journey/parity/log.sh#L1101) |
| compat | `gix log --left-only` | gix log --left-only symmetric-diff side filter deferred — flag accepted | [log.sh:1110](../../tests/journey/parity/log.sh#L1110) |
| compat | `gix log --right-only` | gix log --right-only symmetric-diff side filter deferred — flag accepted | [log.sh:1119](../../tests/journey/parity/log.sh#L1119) |
| compat | `gix log --left-right` | gix log --left-right symmetric-diff side annotation deferred — flag accepted | [log.sh:1128](../../tests/journey/parity/log.sh#L1128) |
| compat | `gix log -g / --walk-reflogs` | gix log --walk-reflogs reflog traversal mode deferred — flag accepted | [log.sh:1139](../../tests/journey/parity/log.sh#L1139) |
| compat | `gix log --grep-reflog=<pattern>` | gix log --grep-reflog reflog message filter deferred — flag accepted | [log.sh:1148](../../tests/journey/parity/log.sh#L1148) |
| compat | `gix log --simplify-by-decoration` | gix log --simplify-by-decoration history simplification deferred — flag accepted | [log.sh:1159](../../tests/journey/parity/log.sh#L1159) |
| compat | `gix log --simplify-merges` | gix log --simplify-merges merge-simplification deferred — flag accepted | [log.sh:1168](../../tests/journey/parity/log.sh#L1168) |
| compat | `gix log --full-history` | gix log --full-history disables history simplification — flag accepted, simplification never applied in gix | [log.sh:1177](../../tests/journey/parity/log.sh#L1177) |
| compat | `gix log --dense` | gix log --dense alias for --full-history — flag accepted | [log.sh:1186](../../tests/journey/parity/log.sh#L1186) |
| compat | `gix log --sparse` | gix log --sparse sparse-history mode deferred — flag accepted | [log.sh:1195](../../tests/journey/parity/log.sh#L1195) |
| compat | `gix log --no-walk` | gix log --no-walk traversal suppression deferred — flag accepted | [log.sh:1204](../../tests/journey/parity/log.sh#L1204) |
| compat | `gix log --do-walk` | gix log --do-walk --no-walk override deferred — flag accepted | [log.sh:1213](../../tests/journey/parity/log.sh#L1213) |
| compat | `gix log --in-commit-order` | gix log --in-commit-order emission-order override deferred — flag accepted | [log.sh:1222](../../tests/journey/parity/log.sh#L1222) |
| compat | `gix log --exclude=<pattern>` | gix log --exclude ref-category exclusion deferred — flag accepted | [log.sh:1233](../../tests/journey/parity/log.sh#L1233) |
| compat | `gix log --glob=<pattern>` | gix log --glob glob ref-selection deferred — flag accepted | [log.sh:1242](../../tests/journey/parity/log.sh#L1242) |
| compat | `gix log --alternate-refs` | gix log --alternate-refs alternates traversal deferred — flag accepted | [log.sh:1251](../../tests/journey/parity/log.sh#L1251) |
| compat | `gix log --parents` | gix log --parents parent-id emission deferred — flag accepted | [log.sh:1262](../../tests/journey/parity/log.sh#L1262) |
| compat | `gix log --children` | gix log --children child-id emission deferred — flag accepted | [log.sh:1271](../../tests/journey/parity/log.sh#L1271) |
| compat | `gix log --show-pulls` | gix log --show-pulls merge rejoin detection deferred — flag accepted | [log.sh:1280](../../tests/journey/parity/log.sh#L1280) |
| compat | `gix log --show-linear-break` | gix log --show-linear-break linear-break marker deferred — flag accepted | [log.sh:1289](../../tests/journey/parity/log.sh#L1289) |
| compat | `gix log -z` | gix log -z NUL-terminator emission deferred — flag accepted | [log.sh:1298](../../tests/journey/parity/log.sh#L1298) |
| compat | `gix log --count` | gix log --count count-only suppression deferred — flag accepted | [log.sh:1307](../../tests/journey/parity/log.sh#L1307) |
| compat | `gix log --submodule=<mode>` | gix log --submodule diff rendering mode deferred — flag accepted | [log.sh:1318](../../tests/journey/parity/log.sh#L1318) |
| compat | `gix log --unified=<n>` | gix log --unified=<n> unified-context width control deferred — flag accepted | [log.sh:1334](../../tests/journey/parity/log.sh#L1334) |
| compat | `gix log --summary` | gix log --summary extended-header summary emission deferred — flag accepted | [log.sh:1344](../../tests/journey/parity/log.sh#L1344) |
| compat | `gix log --compact-summary` | gix log --compact-summary compact-summary emission deferred — flag accepted | [log.sh:1354](../../tests/journey/parity/log.sh#L1354) |
| compat | `gix log --minimal` | gix log --minimal Myers minimal-diff variant deferred — flag accepted | [log.sh:1364](../../tests/journey/parity/log.sh#L1364) |
| compat | `gix log --patience` | gix log --patience patience diff algorithm deferred — flag accepted | [log.sh:1374](../../tests/journey/parity/log.sh#L1374) |
| compat | `gix log --histogram` | gix log --histogram histogram diff algorithm deferred — flag accepted | [log.sh:1384](../../tests/journey/parity/log.sh#L1384) |
| compat | `gix log --diff-filter=<filter>` | gix log --diff-filter=<filter> status-letter filter deferred — flag accepted | [log.sh:1394](../../tests/journey/parity/log.sh#L1394) |
| compat | `gix log --find-object=<oid>` | gix log --find-object=<oid> object-touched filter deferred — flag accepted | [log.sh:1404](../../tests/journey/parity/log.sh#L1404) |
| compat | `gix log --find-copies-harder` | gix log --find-copies-harder aggressive copy detection deferred — flag accepted | [log.sh:1414](../../tests/journey/parity/log.sh#L1414) |
| compat | `gix log --exit-code` | gix log --exit-code diff-presence exit-code bit deferred — empty range used for exit-0 parity — flag accepted | [log.sh:1424](../../tests/journey/parity/log.sh#L1424) |
| compat | `gix log --check` | gix log --check whitespace/conflict-marker check deferred — flag accepted | [log.sh:1434](../../tests/journey/parity/log.sh#L1434) |
| compat | `gix log --binary` | gix log --binary binary patch emission deferred — flag accepted | [log.sh:1444](../../tests/journey/parity/log.sh#L1444) |
| compat | `gix log --full-index` | gix log --full-index full-index hash emission deferred — flag accepted | [log.sh:1454](../../tests/journey/parity/log.sh#L1454) |
| compat | `gix log --remerge-diff` | gix log --remerge-diff remerge-diff rendering deferred — flag accepted | [log.sh:1464](../../tests/journey/parity/log.sh#L1464) |
| compat | `gix log --dirstat` | gix log --dirstat dirstat emission deferred — flag accepted | [log.sh:1474](../../tests/journey/parity/log.sh#L1474) |
| compat | `gix log --ext-diff` | gix log --ext-diff external-diff program dispatch deferred — flag accepted | [log.sh:1484](../../tests/journey/parity/log.sh#L1484) |
| compat | `gix log --no-ext-diff` | gix log --no-ext-diff external-diff suppression deferred — flag accepted | [log.sh:1494](../../tests/journey/parity/log.sh#L1494) |
| compat | `gix log --textconv` | gix log --textconv textconv filter application deferred — flag accepted | [log.sh:1504](../../tests/journey/parity/log.sh#L1504) |
| compat | `gix log --no-textconv` | gix log --no-textconv textconv filter suppression deferred — flag accepted | [log.sh:1514](../../tests/journey/parity/log.sh#L1514) |
| compat | `gix log --text` | gix log --text treat-all-as-text mode deferred — flag accepted | [log.sh:1524](../../tests/journey/parity/log.sh#L1524) |
| compat | `gix log --patch-with-raw` | gix log --patch-with-raw patch+raw composition deferred — flag accepted | [log.sh:1534](../../tests/journey/parity/log.sh#L1534) |
| compat | `gix log --patch-with-stat` | gix log --patch-with-stat patch+stat composition deferred — flag accepted | [log.sh:1544](../../tests/journey/parity/log.sh#L1544) |
| compat | `gix log --color-moved` | gix log --color-moved moved-line highlighting deferred — flag accepted | [log.sh:1554](../../tests/journey/parity/log.sh#L1554) |
| compat | `gix log --word-diff` | gix log --word-diff word-level diff rendering deferred — flag accepted | [log.sh:1564](../../tests/journey/parity/log.sh#L1564) |
| compat | `gix log --word-diff-regex=<regex>` | gix log --word-diff-regex=<regex> word-boundary regex deferred — flag accepted | [log.sh:1574](../../tests/journey/parity/log.sh#L1574) |
| compat | `gix log --ws-error-highlight=<kind>` | gix log --ws-error-highlight=<kind> whitespace-error highlighting deferred — flag accepted | [log.sh:1584](../../tests/journey/parity/log.sh#L1584) |
| compat | `gix log --function-context` | gix log --function-context function-context hunk expansion deferred — flag accepted | [log.sh:1594](../../tests/journey/parity/log.sh#L1594) |
| compat | `gix log --inter-hunk-context=<lines>` | gix log --inter-hunk-context=<lines> inter-hunk merge-context control deferred — flag accepted | [log.sh:1604](../../tests/journey/parity/log.sh#L1604) |
| compat | `gix log --indent-heuristic` | gix log --indent-heuristic indent-heuristic hunk tuning deferred — flag accepted | [log.sh:1614](../../tests/journey/parity/log.sh#L1614) |
| compat | `gix log --no-indent-heuristic` | gix log --no-indent-heuristic indent-heuristic suppression deferred — flag accepted | [log.sh:1624](../../tests/journey/parity/log.sh#L1624) |
| compat | `gix log --irreversible-delete` | gix log --irreversible-delete deletion-only hunk reordering deferred — flag accepted | [log.sh:1634](../../tests/journey/parity/log.sh#L1634) |
| compat | `gix log --no-renames` | gix log --no-renames rename-detection suppression deferred — flag accepted | [log.sh:1644](../../tests/journey/parity/log.sh#L1644) |
| compat | `gix log --rename-empty` | gix log --rename-empty empty-file rename semantics deferred — flag accepted | [log.sh:1654](../../tests/journey/parity/log.sh#L1654) |
| compat | `gix log --no-rename-empty` | gix log --no-rename-empty --rename-empty negation deferred — flag accepted | [log.sh:1664](../../tests/journey/parity/log.sh#L1664) |
| compat | `gix log --ignore-all-space` | gix log --ignore-all-space whitespace-ignoring diff mode deferred — flag accepted | [log.sh:1674](../../tests/journey/parity/log.sh#L1674) |
| compat | `gix log --ignore-blank-lines` | gix log --ignore-blank-lines blank-line ignore mode deferred — flag accepted | [log.sh:1684](../../tests/journey/parity/log.sh#L1684) |
| compat | `gix log --ignore-cr-at-eol` | gix log --ignore-cr-at-eol CR-at-EOL ignore deferred — flag accepted | [log.sh:1694](../../tests/journey/parity/log.sh#L1694) |
| compat | `gix log --ignore-matching-lines=<regex>` | gix log --ignore-matching-lines=<regex> regex-match line-ignore deferred — flag accepted | [log.sh:1704](../../tests/journey/parity/log.sh#L1704) |
| compat | `gix log --ignore-space-at-eol` | gix log --ignore-space-at-eol EOL-whitespace ignore deferred — flag accepted | [log.sh:1714](../../tests/journey/parity/log.sh#L1714) |
| compat | `gix log --ignore-space-change` | gix log --ignore-space-change whitespace-amount ignore deferred — flag accepted | [log.sh:1724](../../tests/journey/parity/log.sh#L1724) |
| compat | `gix log --src-prefix=<prefix>` | gix log --src-prefix=<prefix> source-prefix override deferred — flag accepted | [log.sh:1734](../../tests/journey/parity/log.sh#L1734) |
| compat | `gix log --dst-prefix=<prefix>` | gix log --dst-prefix=<prefix> destination-prefix override deferred — flag accepted | [log.sh:1744](../../tests/journey/parity/log.sh#L1744) |
| compat | `gix log --no-prefix` | gix log --no-prefix a/ b/ prefix suppression deferred — flag accepted | [log.sh:1754](../../tests/journey/parity/log.sh#L1754) |
| compat | `gix log --relative` | gix log --relative pathname-relativization deferred — flag accepted | [log.sh:1764](../../tests/journey/parity/log.sh#L1764) |
| compat | `gix log --no-relative` | gix log --no-relative --relative negation deferred — flag accepted | [log.sh:1774](../../tests/journey/parity/log.sh#L1774) |
| compat | `gix log --output=<file>` | gix log --output=<file> diff-output redirection deferred — flag accepted | [log.sh:1784](../../tests/journey/parity/log.sh#L1784) |
| compat | `gix log --reflog` | gix log --reflog HEAD reflog inclusion as tips deferred — flag accepted | [log.sh:1794](../../tests/journey/parity/log.sh#L1794) |
| compat | `gix log --stdin` | gix log --stdin stdin rev-argument reader deferred — empty stdin keeps exit 0 — flag accepted | [log.sh:1804](../../tests/journey/parity/log.sh#L1804) |
| compat | `gix log --ignore-missing` | gix log --ignore-missing missing-object tolerance deferred — flag accepted | [log.sh:1814](../../tests/journey/parity/log.sh#L1814) |
| deferred | `gix log --merge` | --merge precondition check not yet implemented — git exits 128 when no merge pseudoref exists, gix walks HEAD | [log.sh:1827](../../tests/journey/parity/log.sh#L1827) |
| compat | `gix log --since-as-filter=<time>` | gix log --since-as-filter=<time> --since-as-filter predicate deferred — flag accepted | [log.sh:1836](../../tests/journey/parity/log.sh#L1836) |
| compat | `gix log --exclude-first-parent-only` | gix log --exclude-first-parent-only first-parent exclude semantics deferred — flag accepted | [log.sh:1846](../../tests/journey/parity/log.sh#L1846) |
| compat | `gix log --remove-empty` | gix log --remove-empty empty-commit removal deferred — flag accepted | [log.sh:1856](../../tests/journey/parity/log.sh#L1856) |
| compat | `gix log --single-worktree` | gix log --single-worktree single-worktree ref scoping deferred — flag accepted | [log.sh:1866](../../tests/journey/parity/log.sh#L1866) |
| compat | `gix log --encoding=<enc>` | gix log --encoding=<enc> commit-message encoding conversion deferred — flag accepted | [log.sh:1876](../../tests/journey/parity/log.sh#L1876) |
| compat | `gix log --expand-tabs[=<n>]` | gix log --expand-tabs[=<n>] tab-expansion deferred — flag accepted | [log.sh:1886](../../tests/journey/parity/log.sh#L1886) |
| compat | `gix log --no-expand-tabs` | gix log --no-expand-tabs --expand-tabs negation deferred — flag accepted | [log.sh:1896](../../tests/journey/parity/log.sh#L1896) |
| compat | `gix log --basic-regexp` | gix log --basic-regexp POSIX basic-regex mode for --grep deferred — flag accepted | [log.sh:1913](../../tests/journey/parity/log.sh#L1913) |
| compat | `gix log -P / --perl-regexp` | gix log -P / --perl-regexp Perl-regex mode for --grep deferred — flag accepted | [log.sh:1923](../../tests/journey/parity/log.sh#L1923) |
| compat | `gix log --exclude-hidden=<section>` | gix log --exclude-hidden=<section> hidden-refs filter deferred — flag accepted | [log.sh:1933](../../tests/journey/parity/log.sh#L1933) |
| compat | `gix log --bisect` | gix log --bisect bisect-output emission deferred — flag accepted | [log.sh:1943](../../tests/journey/parity/log.sh#L1943) |
| compat | `gix log --relative-date` | gix log --relative-date relative-date shorthand deferred (equivalent to --date=relative) — flag accepted | [log.sh:1953](../../tests/journey/parity/log.sh#L1953) |
| compat | `gix log --dd` | gix log --dd --diff-merges=dd alias deferred — flag accepted | [log.sh:1963](../../tests/journey/parity/log.sh#L1963) |
| compat | `gix log --no-diff-merges` | gix log --no-diff-merges --diff-merges suppression deferred — flag accepted | [log.sh:1973](../../tests/journey/parity/log.sh#L1973) |
| compat | `gix log --combined-all-paths` | gix log --combined-all-paths combined-diff per-parent path emission deferred — flag accepted | [log.sh:1983](../../tests/journey/parity/log.sh#L1983) |
| compat | `gix log --output-indicator-new=<char>` | gix log --output-indicator-new=<char> custom add-indicator character deferred — flag accepted | [log.sh:1993](../../tests/journey/parity/log.sh#L1993) |
| compat | `gix log --output-indicator-old=<char>` | gix log --output-indicator-old=<char> custom remove-indicator character deferred — flag accepted | [log.sh:2003](../../tests/journey/parity/log.sh#L2003) |
| compat | `gix log --output-indicator-context=<char>` | gix log --output-indicator-context=<char> custom context-indicator character deferred — flag accepted | [log.sh:2013](../../tests/journey/parity/log.sh#L2013) |
| compat | `gix log -t (show tree objects)` | gix log -t (show tree objects) tree-object diff emission deferred — flag accepted | [log.sh:2023](../../tests/journey/parity/log.sh#L2023) |
| compat | `gix log --anchored=<text>` | gix log --anchored=<text> anchored-diff algorithm deferred — flag accepted | [log.sh:2033](../../tests/journey/parity/log.sh#L2033) |
| compat | `gix log --cumulative` | gix log --cumulative cumulative-dirstat emission deferred — flag accepted | [log.sh:2043](../../tests/journey/parity/log.sh#L2043) |
| compat | `gix log --dirstat-by-file` | gix log --dirstat-by-file dirstat per-file counting mode deferred — flag accepted | [log.sh:2053](../../tests/journey/parity/log.sh#L2053) |
| compat | `gix log --no-color-moved` | gix log --no-color-moved --color-moved negation deferred — flag accepted | [log.sh:2063](../../tests/journey/parity/log.sh#L2063) |
| compat | `gix log --color-moved-ws=<mode>` | gix log --color-moved-ws=<mode> moved-line whitespace mode deferred — flag accepted | [log.sh:2073](../../tests/journey/parity/log.sh#L2073) |
| compat | `gix log --no-color-moved-ws` | gix log --no-color-moved-ws --color-moved-ws negation deferred — flag accepted | [log.sh:2083](../../tests/journey/parity/log.sh#L2083) |
| compat | `gix log --color-words` | gix log --color-words word-level coloring deferred — flag accepted | [log.sh:2093](../../tests/journey/parity/log.sh#L2093) |
| compat | `gix log -B / --break-rewrites` | gix log -B / --break-rewrites break-rewrites detection deferred — flag accepted | [log.sh:2103](../../tests/journey/parity/log.sh#L2103) |
| compat | `gix log -C / --find-copies` | gix log -C / --find-copies copy detection deferred — flag accepted | [log.sh:2113](../../tests/journey/parity/log.sh#L2113) |
| compat | `gix log -l<num>` | gix log -l<num> rename-detection scan cap deferred — flag accepted | [log.sh:2123](../../tests/journey/parity/log.sh#L2123) |
| compat | `gix log -O / --orderfile=<file>` | gix log -O / --orderfile=<file> path-ordering file deferred — flag accepted | [log.sh:2133](../../tests/journey/parity/log.sh#L2133) |
| compat | `gix log --skip-to=<path>` | gix log --skip-to=<path> diff skip-to-path deferred — flag accepted | [log.sh:2143](../../tests/journey/parity/log.sh#L2143) |
| compat | `gix log --rotate-to=<path>` | gix log --rotate-to=<path> diff rotate-to-path deferred — flag accepted | [log.sh:2153](../../tests/journey/parity/log.sh#L2153) |
| compat | `gix log -R (reverse diff inputs)` | gix log -R (reverse diff inputs) old/new side swap deferred — flag accepted | [log.sh:2163](../../tests/journey/parity/log.sh#L2163) |
| compat | `gix log --ignore-submodules[=<when>]` | gix log --ignore-submodules[=<when>] submodule-diff ignore mode deferred — flag accepted | [log.sh:2173](../../tests/journey/parity/log.sh#L2173) |
| compat | `gix log --default-prefix` | gix log --default-prefix default-prefix restore deferred — flag accepted | [log.sh:2183](../../tests/journey/parity/log.sh#L2183) |
| compat | `gix log --line-prefix=<prefix>` | gix log --line-prefix=<prefix> per-line prefix emission deferred — flag accepted | [log.sh:2193](../../tests/journey/parity/log.sh#L2193) |
| compat | `gix log --ita-invisible-in-index` | gix log --ita-invisible-in-index intent-to-add invisibility deferred — flag accepted | [log.sh:2203](../../tests/journey/parity/log.sh#L2203) |
| compat | `gix log --show-notes-by-default` | gix log --show-notes-by-default notes-by-default emission deferred — flag accepted | [log.sh:2219](../../tests/journey/parity/log.sh#L2219) |
| compat | `gix log --show-notes` | gix log --show-notes deprecated --show-notes alias deferred — flag accepted | [log.sh:2229](../../tests/journey/parity/log.sh#L2229) |
| compat | `gix log --standard-notes` | gix log --standard-notes deprecated --standard-notes alias deferred — flag accepted | [log.sh:2239](../../tests/journey/parity/log.sh#L2239) |
| compat | `gix log --no-standard-notes` | gix log --no-standard-notes deprecated --no-standard-notes alias deferred — flag accepted | [log.sh:2249](../../tests/journey/parity/log.sh#L2249) |
| compat | `gix log --no-use-mailmap` | gix log --no-use-mailmap --no-use-mailmap (alias of --no-mailmap) deferred — flag accepted | [log.sh:2259](../../tests/journey/parity/log.sh#L2259) |

## tag

| Class | Section | Reason | Source |
|---|---|---|---|
| compat | `gix tag --sort` | tag --sort=<key> interpreter deferred (key-based sort, descending/version) | [tag.sh:270](../../tests/journey/parity/tag.sh#L270) |
| compat | `gix tag --sort` | tag --sort=<key> interpreter deferred (key-based sort, descending/version) | [tag.sh:273](../../tests/journey/parity/tag.sh#L273) |
| compat | `gix tag --sort` | tag --sort=<key> interpreter deferred (key-based sort, descending/version) | [tag.sh:276](../../tests/journey/parity/tag.sh#L276) |
| compat | `gix tag --column / --no-column` | tag --column packing deferred; Clap accepts, one-per-line output | [tag.sh:306](../../tests/journey/parity/tag.sh#L306) |
| compat | `gix tag --column / --no-column` | tag --column=always packing deferred; Clap accepts, one-per-line output | [tag.sh:309](../../tests/journey/parity/tag.sh#L309) |
| deferred | `gix tag -F / --file` | tag -F - stdin row blocked on expect_parity_reset PARITY_STDIN plumbing | [tag.sh:662](../../tests/journey/parity/tag.sh#L662) |

