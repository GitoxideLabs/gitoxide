# gix-hook

Discover and execute traditional Git hooks and configured hook events, without a
repository handle. Callers supply resolved directories, command context, and the
configuration sections they trust.

Hooks run serially with literal arguments and environment overrides, captured
output, and replayed standard input. Unix executable scripts without a shebang
use Git's shell fallback; Windows interpreter lookup uses `gix-command`.

Enable `sha1` or `sha256` to select the hash support required by `gix-config`.
