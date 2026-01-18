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

//! A context-aware concrete Error type built on `std::error::Error`
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]

mod ext;
pub use ext::{ErrorExt, OptionExt, ResultExt};

mod impls;
pub use impls::{Frame, Something, Untyped};

mod macros;

/// An exception type that can hold an [error tree](Exn::raise_all) and the call site.
///
/// While an error chain, a list, is automatically created when [raise](Exn::raise)
/// and friends are invoked, one can also use [`Exn::raise_all`] to create an error
/// that has multiple causes.
///
/// # Warning: `source()` information is stringified and type-erased
///
/// All `source()` values are turned into frames, but lose their type information completely.
/// This is because they are only seen as reference and thus can't be stored.
///
/// # `Exn` == `Exn<Untyped>`
///
/// `Exn` act's like `Box<dyn std::error::Error + Send + Sync + 'static>`, but with the capability
/// to store a tree of errors along with their *call sites*.
///
/// # Visualisation
///
/// Linearized trees during display make a list of 3 children indistinguishable from
/// 3 errors where each is the child of the other.
///
/// ## Debug
///
/// * locations: ✔️
/// * error display: Display
/// * tree mode: linearized
///
/// ## Debug + Alternate
///
/// * locations: ❌
/// * error display: Display
/// * tree mode: linearized
///
/// ## Display
///
/// * locations: ❌
/// * error display: Debug
/// * tree mode: None
///
/// ## Display + Alternate
///
/// * locations: ❌
/// * error display: Debug
/// * tree mode: verbatim
pub struct Exn<E: std::error::Error + Send + Sync + 'static = Untyped> {
    // trade one more indirection for less stack size
    frame: Box<Frame>,
    phantom: PhantomData<E>,
}

use std::marker::PhantomData;
