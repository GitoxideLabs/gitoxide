// Copyright 2025 FastLabs Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Frame inspection and type erasure for [`crate::Exn`].
//!
//! The exception type and its extension traits are exported only at the crate root.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

pub(super) mod ext;

pub(super) mod impls;
pub use impls::{Frame, Untyped};

mod macros;
