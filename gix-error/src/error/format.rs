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

use crate::error::{ErrorNode, frame_error};
use crate::{Frame, write_location};

pub(crate) fn debug_frame(frame: &Frame, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write_frame_recursive(f, frame, "", ErrorMode::Display, TreeMode::Linearize)
}

pub(crate) fn display_frame(frame: &Frame, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if f.alternate() {
        write_frame_recursive(f, frame, "", ErrorMode::Debug, TreeMode::Verbatim)
    } else {
        fmt::Display::fmt(frame_error(frame), f)
    }
}

#[derive(Copy, Clone)]
enum ErrorMode {
    Display,
    Debug,
}

#[derive(Copy, Clone)]
enum TreeMode {
    Linearize,
    Verbatim,
}

fn write_frame_recursive(
    f: &mut fmt::Formatter<'_>,
    frame: &Frame,
    prefix: &str,
    err_mode: ErrorMode,
    tree_mode: TreeMode,
) -> fmt::Result {
    write_error_node_recursive(f, ErrorNode::Frame(frame), prefix, err_mode, tree_mode)
}

fn write_error_node_recursive(
    f: &mut fmt::Formatter<'_>,
    node: ErrorNode<'_>,
    prefix: &str,
    err_mode: ErrorMode,
    tree_mode: TreeMode,
) -> fmt::Result {
    // Nested boundaries are expanded below; formatting their whole graph here would print the same causes twice.
    let mut error = node.error();
    while let Some(nested) = error.downcast_ref::<crate::Error>() {
        error = nested.error();
    }
    match err_mode {
        ErrorMode::Display => fmt::Display::fmt(error, f),
        ErrorMode::Debug => write!(f, "{error:?}"),
    }?;
    if !f.alternate() {
        write_location(f, node.location())?;
    }

    let children = node.children();
    let children_len = children.len();

    for (child_index, child) in children.into_iter().enumerate() {
        write!(f, "\n{prefix}|")?;
        write!(f, "\n{prefix}└─ ")?;

        let child_child_len = if child
            .error()
            .downcast_ref::<crate::Error>()
            .is_some_and(|err| err.iter_errors().filter(|source| !source.is::<crate::Error>()).count() > 1)
        {
            1
        } else {
            child.children().len()
        };
        let may_linearize_chain = matches!(tree_mode, TreeMode::Linearize) && children_len == 1 && child_child_len == 1;
        if may_linearize_chain {
            write_error_node_recursive(f, child, prefix, err_mode, tree_mode)?;
        } else if child_index < children_len - 1 {
            write_error_node_recursive(f, child, &format!("{prefix}|   "), err_mode, tree_mode)?;
        } else {
            write_error_node_recursive(f, child, &format!("{prefix}    "), err_mode, tree_mode)?;
        }
    }

    if let Some(err) = node.error().downcast_ref::<crate::Error>() {
        for source in err.iter_errors().filter(|source| !source.is::<crate::Error>()).skip(1) {
            write!(f, "\n{prefix}|\n{prefix}└─ ")?;
            match err_mode {
                ErrorMode::Display => fmt::Display::fmt(source, f),
                ErrorMode::Debug => write!(f, "{source:?}"),
            }?;
        }
    }

    Ok(())
}
