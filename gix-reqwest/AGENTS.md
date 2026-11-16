# Maintaining `gix-reqwest`

This directory is a fork of [reqwest](https://github.com/seanmonstar/reqwest),
carrying HTTP fixes required by Gitoxide until they are available upstream.
Preserve the original authors, copyright notices, and dual licenses.

Use [the reintegration skill](../.agents/skills/reintegrate-reqwest/SKILL.md)
when updating from upstream or importing its patches. The latest integrated
trunk commit is recorded in reachable `Reqwest-Upstream` commit fields;
`Reqwest-Patch` identifies separate ports and `Reqwest-PR` their known source PRs.
Keep all local notes and provenance before the verbatim original message, with
nothing appended. Read fields from the full message, as Git's trailer parser
only recognizes the final block. Do not infer the trunk
baseline from the newest local patch or maintain a second revision file.

Keep imported source conventions and the local 100-column formatter to make
upstream merges reviewable. The manifest lists explicit existing Clippy style
exceptions and Gitoxide QA configurations. Apply the root development rules
to new local code; avoid unrelated cleanup of copied upstream code.

The package and Rust crate names are `gix-reqwest` and `gix_reqwest`. These
types are distinct from upstream `reqwest` types. Keep docs, examples, and
`gix_transport::gix_reqwest` consistent. Feature names and `reqwest_unstable`
retain their upstream spelling.

Implement generally correct HTTP behavior here. Keep Git-specific URL suffix
validation, config precedence, and credential-helper routing in Gitoxide's
transport and repository layers. Retain regression coverage for outstanding
patches, including proxy credentials and headers at every redirect hop.
