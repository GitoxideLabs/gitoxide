# The stable `gix` API

The default API is the baseline we intend to stabilize. The additive `unstable`
Cargo feature exposes APIs which may change incompatibly. It is never enabled by
default or by another public `gix` feature.

- Audit the entire reachable interface: arguments, results, fields, callbacks,
  iterator items, generic bounds, associated types, conversions, trait impls and
  re-exports. Hiding a dependency's module does not hide its types in signatures.
- Gate APIs exposing unstable plumbing types unless the consumer's manifest
  declares them in `package.metadata.api-check.allowed-unstable-types` or
  `allowed-unstable-crates`. Do not gate an API solely for a declared exception.
  `recursive = true` checks supported members of reachable dependency types;
  an exception does not admit another type exposed through those members.
- Use `#[gix_macros::unstable(since = "YYYY-MM-DD")]` for items, methods, and file-backed
  module declarations. Rust 1.99 supports these module attributes directly. It
  retains crate-private access without the feature. For selected fields, use
  `unstable_fields` on the struct and `#[unstable(since = "YYYY-MM-DD")]` on each field.
  Use the item's introduction date from Git history, following renames. Add an
  optional `reason = "..."` only for a rationale beyond the item's signature or
  the fact that it is experimental.
- Trait implementations cannot be crate-private. Gate unstable trait impls and
  use a private adapter where internal code still needs the trait.
- Decide explicitly for each public struct and enum whether callers may construct
  it or match it exhaustively. Private fields protect construction; otherwise use
  `#[non_exhaustive]` when extension is intended. Existing public fields and variant
  payloads remain contracts. Enum-level `non_exhaustive` does not extend fields of
  its existing variants.
- Feature unification can enable `unstable` in any consumer's dependency graph.
  Keep stable signatures, type identities, auto-traits and behavior available in
  both configurations. Add gates only to additional APIs, not alternate meanings
  of existing stable APIs.
- Stable error recovery uses classifications and metadata. Documented downcasts
  to unstable plumbing errors belong to gated APIs or gated documentation. An
  underlying error being inspectable does not promise its concrete type.
- Keep new APIs gated until their complete reachable interface has been reviewed.
  Stabilization removes the gate, adds downstream coverage, and records the first
  stable version in documentation. Never move an already stable API behind a gate
  in a compatible release.
- Check stable and unstable configurations separately. Workspace clients opting
  into unstable can mask missing gates, so use isolated downstream compile tests
  and run the manifest-driven `check-api boundary gix/Cargo.toml` check. Preserve
  supported hash, threading and networking configurations.

This baseline is not a declaration that `gix` or its foundational dependencies
are already ready for a 1.0 release.
