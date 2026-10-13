# The stable `gix` API

The default API is the baseline we intend to stabilize. The additive `unstable`
Cargo feature exposes APIs which may change incompatibly. It is never enabled by
default or by another public `gix` feature.

- Audit the entire reachable interface: arguments, results, fields, callbacks,
  iterator items, generic bounds, associated types, conversions, trait impls and
  re-exports. Hiding a dependency's module does not hide its types in signatures.
- Gate APIs exposing unstable plumbing types, including foundational crates such
  as `gix-hash` and `gix-trace`, until those types have a stability commitment.
  TODO: stabilize `gix-error`; its public errors, metadata, classifications and
  helpers are temporarily exempt from this rule. Do not gate an API just because
  it uses `gix::Result` or `gix::Error`.
- Use `#[gix_macros::unstable(reason = "...")]` for items and methods. It retains
  crate-private access without the feature. Use `unstable_module!` for file-backed
  modules to preserve the MSRV. For selected fields, use `unstable_fields` on the
  struct and `#[unstable(reason = "...")]` on each field. State the concrete
  dependency or unresolved API design in the reason.
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
  and inspect the stable rustdoc API. Preserve supported hash, threading and
  networking configurations.

This baseline is not a declaration that `gix` or its foundational dependencies
are already ready for a 1.0 release.
