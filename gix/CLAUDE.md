# gix

Interact with git repositories just like git would.

**Tier:** porcelain
**Errors:** gix-error
**Notable:** The porcelain hub — `Repository` is the central type, and most parity-level API methods land here as thin wrappers that delegate to `gix-*` plumbing. Leaf-first order: prove the primitive in plumbing, then port the convenience method here. Cheap Platform/handle types allowed; cloning `Repository` for convenience is fine. Pattern for new features: `impl Repository { fn foo(&self) -> ... }` delegating to `gix_<crate>::foo(...)`.

See root `CLAUDE.md` for branch discipline, agent roster (gix-architect / gix-steward / gix-runner), and the parity-loop structure.
