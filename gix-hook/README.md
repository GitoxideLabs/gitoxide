# gix-hook

Discover and execute traditional Git hooks and configured hook events, without a
repository handle. Callers supply resolved directories, command context, and the
configuration sections they trust.

Hooks run serially with literal arguments and environment overrides, captured
output, and replayed standard input. Prepared commands can stream separate or
combined output with cancellation and bounded buffering. Both forms use the same
process runner. Unix executable scripts without a shebang use Git's shell
fallback; Windows interpreter lookup uses `gix-command`.

Event dispatch also accepts a caller-supplied executor for event-specific command
adjustments and streaming, while sharing configured and traditional hook ordering.

Enable `sha1` or `sha256` to select the hash support required by `gix-config`.
