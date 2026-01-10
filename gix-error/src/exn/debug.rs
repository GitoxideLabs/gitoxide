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

use std::fmt;
use std::fmt::Formatter;

use crate::exn::Exn;
use crate::exn::Frame;

impl<E: std::error::Error + Send + Sync + 'static> fmt::Debug for Exn<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write_exn(f, self.as_frame(), 0, "")
    }
}

impl fmt::Debug for Frame {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write_exn(f, self, 0, "")
    }
}

fn write_exn(f: &mut Formatter<'_>, frame: &Frame, level: usize, prefix: &str) -> fmt::Result {
    write!(f, "{}", frame.as_error())?;
    write_location(f, frame)?;

    let children = frame.children();
    let children_len = children.len();

    for (i, child) in children.iter().enumerate() {
        write!(f, "\n{prefix}|")?;
        write!(f, "\n{prefix}|-> ")?;

        let child_child_len = child.children().len();
        if level == 0 && children_len == 1 && child_child_len == 1 {
            write_exn(f, child, 0, prefix)?;
        } else if i < children_len - 1 {
            write_exn(f, child, level + 1, &format!("{prefix}|   "))?;
        } else {
            write_exn(f, child, level + 1, &format!("{prefix}    "))?;
        }
    }

    Ok(())
}

fn write_location(f: &mut Formatter<'_>, exn: &Frame) -> fmt::Result {
    let location = exn.location();
    write!(f, ", at {}:{}:{}", location.file(), location.line(), location.column())
}
