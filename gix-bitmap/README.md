# gix-bitmap — README (draft)

## What it is
Library crate for reading and using Git pack bitmaps to accelerate reachability/object enumeration in packed repositories within the gitoxide stack.

## When to use / When not to
- **Use when** building tools/services that need faster reachability checks and must scale with large repositories.
- **Do not use when** you need a CLI or a high‑level package/repository management layer; this is an internal component.

## Stability & MSRV
Stability: Unspecified — see the project‑wide stability policy (https://github.com/GitoxideLabs/gitoxide/blob/main/STABILITY.md).
MSRV: Inherits the workspace’s Minimum Supported Rust Version — see MSRV policy for details (https://github.com/GitoxideLabs/gitoxide/blob/main/.github/workflows/msrv.yml).

## Links
- crates.io: https://crates.io/crates/gix-bitmap
- docs.rs: https://docs.rs/gix-bitmap/latest/gix_bitmap/

## Related crates
- `gix-pack` — pack file access and operations.
- `gix-object` — core Git object types and serialization.

## License
Dual-licensed under MIT OR Apache-2.0.
