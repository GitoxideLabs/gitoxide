#!/usr/bin/env bash
# Enumerate git command surface and gix Clap subcommands, produce a
# refreshed `docs/parity/commands.md` matrix.
#
# This is a STUB. Fill in when we've closed enough rows manually to know
# the matrix structure we want to scale to. For now the matrix is
# seeded by hand in docs/parity/commands.md.
#
# Planned implementation:
#   - git commands: `git help -a --no-verbose --no-aliases` → list
#   - git flags per command: parse vendor/git/Documentation/git-<cmd>.txt
#   - gix commands: parse src/plumbing/options/mod.rs for Subcommands variants
#                   (or add a `gix debug enumerate` subcommand that uses
#                    `clap::Command::get_arguments()` — cleaner)
#   - ein commands: same approach, parsed separately, marked "no git target"
#
# Output: docs/parity/commands.md with three tables (porcelain, plumbing, ein).

set -eu
echo "TODO: etc/parity/enumerate.sh not yet implemented" >&2
echo "For now, maintain docs/parity/commands.md by hand." >&2
exit 1
