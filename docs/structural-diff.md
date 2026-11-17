# Structural diffs with `ein`

`ein tool diff BEFORE AFTER` compares the structure of Rust, TOML, and Markdown
files. It shows declarations in their source order, preserving their indentation
and signature layout. Changes inside a function mark that function as modified;
the function's body is never printed. Closure bodies are hidden too.

Rust outlines include modules, types, fields, implementations, and functions.
TOML outlines show tables and keys without their values. Markdown outlines show
heading hierarchies without prose or code blocks. Changes to hidden content still
mark the containing item as modified.

Lines starting with `+` and `-` show additions and deletions. `~` marks an item
whose hidden content or descendants changed while its displayed declaration stayed the same.
Changed declarations appear as an old `-` line and a new `+` line.

```shell
ein tool diff old.rs new.rs
ein tool diff --language rust old-copy new-copy
ein tool diff --color always old.rs new.rs | less -R
```

The file extension selects the language. `--language rust`, `--language toml`, or
`--language markdown` overrides detection. `--path PATH` supplies the original
filename when the two inputs have temporary names. `/dev/null` can represent a
missing side for an addition or deletion.

`--color auto` is the default: colors are enabled only when stdout is a terminal
and `NO_COLOR` is unset. `--color always` and `--color never` override that choice.
Differences exit successfully, as expected by Git difftool; unsupported languages,
invalid source, and unreadable inputs return an error. There is no fallback to
printing raw source for unsupported or invalid inputs.

## Structural context

`--context 1` (or `-U 1`) shows one unchanged sibling on either side of a change
at every nesting depth. Context counts declarations, keys, or headings, not lines.
Ancestors of changed items are always shown. Unchanged context containers keep
their contents collapsed; `…` marks omitted siblings or children.

Supply comma-separated counts for independent depths. `--context 0,2` shows no
unchanged file-level declarations and two peers around changed members inside a
type, implementation, module, TOML table, or Markdown section. The final count
applies at all deeper levels. `--context 0` shows only changes and their ancestors.

## Rename detection

`--find-renames` (or `-M`) compares unmatched declarations of the same kind within
each container, including across unchanged siblings. A match needs at least 60%
word-token similarity and must be the unique best match in both directions.
Whitespace and occurrences of each item's own name do not count against similarity,
so recursive references and modest body edits can accompany a rename. Annotations
identify inferred renames while the old and new declarations remain visible.

Ambiguous candidates stay as additions and deletions. Matching is heuristic,
does not track changes across files or different parents, and can be expensive
when many siblings are replaced. It is disabled by default.

## Word diffs

`--word-diff` highlights changed words within matched declarations using
`[-removed-]` and `{+added+}` on `~` lines. With color enabled, changed words
also receive bold red or green styling. Unchanged whitespace, Unicode names,
and multiline signature layout are preserved. Body-only changes still show
only a modified declaration; implementation text never enters the word diff.

Combine `--word-diff --find-renames` to highlight inferred name changes inline.
Without rename detection, unmatched names remain separate additions/deletions.

## Use with Git difftool

Configure the tool in the current repository:

```shell
git config difftool.structural.cmd 'ein tool diff --path "$BASE" "$LOCAL" "$REMOTE"'
git difftool --no-prompt --tool=structural
git difftool --no-prompt --tool=structural HEAD~1 HEAD -- '*.rs' '*.toml' '*.md'
```

Git invokes the tool once for each changed file. `--path "$BASE"` retains the
original filename for language detection even when either version lives in a
temporary file. Paths with spaces are supported. This is a display tool: its
output is a structural summary rather than an applyable patch, and it does not
modify either input.

## Build support

The default, `lean`, and `lean-async` builds include this tool. It requires a C
compiler to build tree-sitter and the language parsers, so `small` and `max-pure`
omit it. Add the `gitoxide-core-tools-diff` feature to opt in explicitly:

```shell
cargo build --bin ein --no-default-features --features small,gitoxide-core-tools-diff
```
