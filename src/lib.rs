/*
 * Minparser Simple parsing functions
 *
 * Copyright (C) 2024-2025 Paolo De Donato
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

//! Simple parsing tools
//!
//! This crate is a collection of objects and algorithms shared among different crates that needs
//! to implement a parser. 
//!
//! A [``Position<F>``](pos::Position) is an object that identifies a (textual) *file* and a position inside it,
//! represented as a *line index* and a *column index*. The main role of a `Position` object is to
//! uniquely identify a single character or a token inside a file in order to allow the
//! user to easily find it.
//!
//! A [``View<'a, F>``](view::View) can be seen as a suffix of a larger string with the position of
//! its first character. The [``match_tool``](view::View::match_tool) and the [``match_atom``](view::View::match_atom)
//! methods can be used to match its prefix with any object implementing the [``ParseTool``](view::ParseTool) and the [``Atom``](atoms::Atom)
//! traits respectively.
//!
//! Many useful parsing tools can be found in [`atomlist`] and [`utils`] modules.
//!
//! # Usage example
//! ```
//! use minparser::prelude::*;
//! let view = ViewFile::new_default("My string   value");
//! let (step, mtc) = view.match_atom_string("My string").unwrap();
//! assert_eq!(mtc, "My string");
//! assert_eq!(step.get_view(), "   value"); 
//! //let step = step.match_tool(minparser::utils::WhiteTool).unwrap();   // Use the WhiteTool tool to
//! //assert_eq!(step.get_view(), "value");                               //match a sequence of whitespaces
//! //assert!(step.match_tool('a').is_err()); // A missing match is an error
//! ```
//!
//! # Main objects
//! ## [`Position<F>`](crate::pos::Position) and [``Pos<T, F>``](crate::pos::Pos).
//! A ``Position<F>`` is an object that holds a location inside a pool of different resources,
//! where each resource can be identified as a string of textual data.
//!
//! These object holds the following information:
//! - a `file` field of type `F` that identify a single resource inside your pool. Its type is
//!   provided by the user. If you work on a single resource then you should use
//!   [`NoFile`](crate::pos::NoFile) as `file`.
//! - a position inside such resource, which is represented as the `line` number and `column`
//!   number, both of type `u32`. Lines here can be separated by either `\n` or `\r\n`, and it is an
//!   error if any `\r` character in the resource is not followed by the `\n` character.
//!
//! The [``Pos<T, F>``](crate::pos::Pos) is just a pairing of an object of type `T` and a ``Position<F>``.
//!
//! ## [``View<'a, F>``](crate::view::View)
//! A `View<'a, F>` holds a reference to a `str` with lifetime `'a` and a ``Position<F>`` that
//! locates the first character inside the resources pool. The most important method is
//! [`match_tool`](crate::view::View::match_tool) that tests if any prefix of the view matches the
//! provided pattern (called here *tool*) and if it matches then it strips away the matched prefix,
//! or an error if no prefix matches it.
//!
//! *Tip*: If you want to evaluate the original view after a missing match then you can clone it (which is
//! possible when `F` implements `Clone`).
//!
//! ## Atoms and Tools
//! An atom is an object that implements the [``Atom``](atoms::Atom) trait. Instead, a tool is an object
//! that implements the [``ParseTool``](view::ParseTool) trait.
//! Both these objects incapsulates patterns that a string prefix may or may not satisfy. Their
//! main difference involes missed matches: an `Atom` that doesn't match a string prefix reports
//! the `Position` of its failure exatly at the beginning of the prefix, whereas a `ParseTool` can
//! be more precise about where the failure happened. You can see an `Atom` just as an atomic element: it
//! can only match or not match. A `ParseTool` instead can be viewed as a compound objects which each
//! element can fail indipendently. 
//!
//! Moreover, a `ParseTool` can optionally return additional data other than the matched prefix.
//! 
//! # Documented features
//! A list of features you can optionally enable. None of these are enabled by default:
//!
//! - `alloc`: Links the library with the [`alloc`] crate and enable associated functions that works
//!   with [`Vec`](alloc::vec::Vec) and other containers in `alloc`;
//! - `nightly-features`: Enable experimental features. **Warning**: it requires nightly builds and
//!   breaking changes may be introduced even at minor releases.
#![deny(missing_docs)]
#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![no_std]
#![cfg_attr(any(feature = "nightly-features", doc, test), feature(doc_cfg), feature(try_trait_v2), feature(try_trait_v2_residual))]

#[cfg(any(feature = "alloc", test, doc))]
extern crate alloc;
#[cfg(any(feature = "std", test, doc))]
extern crate std;

pub mod pos;
pub mod atoms;
// Generalization of views
pub mod chains;
pub mod atomlist;
pub mod view;
pub mod predicates;
pub mod utils;

/// Crate prelude
pub mod prelude {
    pub use crate::pos::*;
    pub use crate::atoms::*;
    pub use crate::chains::*;
    pub use crate::atomlist::*;
    pub use crate::view::*;
}
