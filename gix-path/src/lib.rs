//! This crate contains an assortment of utilities to deal with paths and their conversions.
//!
//! Generally `git` treats paths as bytes, but inherently assumes non-illformed UTF-8 as encoding on windows. Internally, it expects
//! slashes to be used as path separators and paths in files must have slashes, with conversions being performed on windows accordingly.
//!
//! <details>
//!
//! ### Research
//!
//! * **windows**
//! - [`dirent.c`](https://github.com/git/git/blob/main/compat/win32/dirent.c#L31:L31) contains all implementation (seemingly) of opening directories and reading their entries, along with all path conversions (UTF-16 for windows). This is done on the fly so git can work with [in UTF-8](https://github.com/git/git/blob/main/compat/win32/dirent.c#L12:L12).
//! - mingw [is used for the conversion](https://github.com/git/git/blob/main/compat/mingw.h#L579:L579) and it appears they handle surrogates during the conversion, maybe some sort of non-strict UTF-8 converter? Actually it uses [WideCharToMultiByte](https://docs.microsoft.com/en-us/windows/win32/api/stringapiset/nf-stringapiset-widechartomultibyte)
//!   under the hood which by now does fail if the UTF-8 would be invalid unicode, i.e. unicode pairs.
//! - `OsString` on windows already stores strings as WTF-8, which supports [surrogate pairs](https://unicodebook.readthedocs.io/unicode_encodings.html),
//!   something that UTF-8 isn't allowed do it for security reasons, after all it's UTF-16 specific and exists only to extend
//!   the encodable code-points.
//! - informative reading on [WTF-8](https://simonsapin.github.io/wtf-8/#motivation) which is the encoding used by Rust
//!   internally that deals with surrogates and non-wellformed surrogates (those that aren't in pairs).
//! * **unix**
//! - It uses [opendir](https://man7.org/linux/man-pages/man3/opendir.3.html) and [readdir](https://man7.org/linux/man-pages/man3/readdir.3.html)
//!   respectively. There is no encoding specified, except that these paths are null-terminated.
//!
//! ### Learnings
//!
//! Surrogate pairs are a way to extend the encodable value range in UTF-16 encodings, used primarily on windows and in Javascript.
//! For a long time these codepoints used for surrogates, always to be used in pairs, were not assigned, until…they were for rare
//! emojies and the likes. The unicode standard does not require surrogates to happen in pairs, even though by now unpaired surrogates
//! in UTF-16 are considered ill-formed, which aren't supposed to be converted to UTF-8 for example.
//!
//! This is the reason we have to deal with `to_string_lossy()`, it's _just_ for that quirk.
//!
//! This also means the only platform ever eligible to see conversion errors is windows, and there it's only older pre-vista
//! windows versions which incorrectly allow ill-formed UTF-16 strings. Newer versions don't perform such conversions anymore, for
//! example when going from UTF-16 to UTF-8, they will trigger an error.
//!
//! ### Conclusions
//!
//! Since [WideCharToMultiByte](https://docs.microsoft.com/en-us/windows/win32/api/stringapiset/nf-stringapiset-widechartomultibyte) by now is
//! fixed (Vista onward) to produce valid UTF-8, lone surrogate codepoints will cause failure, which `git`
//! [doesn't care about](https://github.com/git/git/blob/main/compat/win32/dirent.c#L12:L12).
//!
//! We will, though, which means from now on we can just convert to UTF-8 on windows and bubble up errors where necessary,
//! preventing potential mismatched surrogate pairs to ever be saved on disk by gitoxide.
//!
//! Even though the error only exists on older windows versions, we will represent it in the type system through fallible function calls.
//! Callers may `.expect()` on the result to indicate they don't wish to handle this special and rare case. Note that servers should not
//! ever get into a code-path which does panic though.
//! </details>
#![deny(missing_docs, rust_2018_idioms)]
#![cfg_attr(not(test), deny(unsafe_code))]

/// A dummy type to represent path specs and help finding all spots that take path specs once it is implemented.
mod convert;
pub use convert::*;

mod util;
pub use util::is_absolute;

///
pub mod realpath;
pub use realpath::function::{realpath, realpath_opts};

/// Information about the environment in terms of locations of resources.
pub mod env;

///
pub mod relative_path;
