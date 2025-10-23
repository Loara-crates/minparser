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
//! Parsing atoms
//!
//! This module provides the [`Atom`] trait and some parsing atoms which incapsulates some
//! basic algorithms which you can use to define more sofisticated ones.

/// Data obtained from a successful match.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub struct Match{
    /// Length *in bytes* of the string prefix satisfying the match.
    pub len : usize
}

impl Match {
    const fn new_zero() -> Self {
        Self {len : 0}
    }
    const fn append(self, m : Self) -> Self {
        Self{ len : self.len + m.len}
    }
}

/// The main parsing trait
///
/// Every object which incapsulate a parsing strategy should implement this trait in order to be
/// composed with other atoms.
pub trait Atom {
    /// The main parsing algorithm.
    ///
    /// # Errors
    /// If no prefix of `st` satisfies this parsing strategy then `None` is returned.
    fn parse(&self, st : &str) -> Option<Match>;

    /// Coerce to dyn trait
    fn as_dyn(&self) -> &dyn Atom where Self : Sized {
        self
    }

    /// Converts this atom into a [`ParseTool`](crate::view::ParseTool).
    ///
    /// The generated tool will always return the matched string as data, and the
    /// [`View`](crate::view::View) pointing at the beginning of the string in case of a failed
    /// match. More information can be found at the documentation of
    /// [`AtomTool`](crate::view::AtomTool);
    fn into_tool(self) -> crate::view::AtomTool<Self> where Self : Sized{
        crate::view::AtomTool(self)
    }
}
/// A atom that always matches.
///
/// Note: even if the atom always matches the match length may be equal to 0, which sometimes can
/// be interpreted as a failed match.
///
/// If an object implements this trait, then [`Atom`] should be implemented as follows:
///
/// ```
/// use minparser::prelude::*;
/// struct A;
///
/// impl AlwaysAtom for A {
///     fn parse_always(&self, st : &str) -> Match { todo!(); }
/// }
/// 
/// impl Atom for A {
///     fn parse(&self, st : &str) -> Option<Match> { 
///         Some(self.parse_always(st))
///     }
/// }
/// ```
pub trait AlwaysAtom : Atom {
    /// Returns the length of the match
    fn parse_always(&self, st : &str) -> Match;
}

/// String view with accessory matching methods.
///
/// Despite [`View`](crate::view::View), this object does not contain the string's `Position` but
/// instead stores the byte length of the matched string prefix.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct MatchHelper<'a>{
    st : &'a str,
    prefix : Match,
}

impl<'a> From<&'a str> for MatchHelper<'a> {
    fn from(st : &'a str) -> Self {
        Self{st, prefix : Match::new_zero()}
    }
}

impl<'a> MatchHelper<'a> {
    /// Progress the view and its position.
    ///
    /// # Panics
    /// Panics if `inc` doesn;t lie on UTF-8 code point boundaries.
    const fn progress(self, inc : Match) -> (&'a str, Self) { 
        let (pfx, sfx) = self.st.split_at(inc.len);
        (pfx, Self{st : sfx, prefix : self.prefix.append(inc)})
    }
    /// Matches an atom with the view.
    ///
    /// If a match happens then the progressed view is returned in the `Ok` variant.
    ///
    /// # Errors
    /// If a match doesn't happen then the calling view is returned as `Err` unchanged 
    pub fn match_atom<R : Atom>(self, t : R) -> Result<Self, Self> {
        t.parse(self.st).map(|inc| self.progress(inc).1)
            .ok_or(self)
    }
    /// Matches an atom with the view and returns the matching prefix.
    ///
    /// # Errors
    /// If a match doesn't happen then the calling view is returned as `Err` unchanged 
    pub fn match_atom_string<R : Atom>(self, t : R) -> Result<(&'a str, Self), Self> {
        t.parse(self.st).map(|inc| self.progress(inc))
            .ok_or(self)
    }
    /// Matches an [`AlwaysAtom`] with the view.
    #[must_use]
    pub fn match_always<R : AlwaysAtom>(self, t : R) -> Self {
        self.progress(t.parse_always(self.st)).1
    }
    /// Matches an [`AlwaysAtom`] and returns the matched prefix.
    pub fn match_always_string<R : AlwaysAtom>(self, t : R) -> (&'a str, Self) {
        self.progress(t.parse_always(self.st))
    }
    /// Finalize the helper, returning the matched prefix length and the remaining unparsed string.
    #[must_use]
    pub const fn finalize(self) -> (&'a str, Match) {
        (self.st, self.prefix)
    }
    /// Enforce that all the string has been parsed before finalizing.
    #[allow(clippy::missing_errors_doc)]
    pub const fn all_finalize(self) -> Result<Match, Self> {
        if self.st.is_empty() {
            Ok(self.prefix)
        }
        else {
            Err(self)
        }
    }
    /// Enforce that the atom matches the entire string before finalizing.
    #[allow(clippy::missing_errors_doc)]
    pub fn match_finalize<R : Atom>(self, t : R) -> Result<Match, Self> {
        self.match_atom(t).and_then(MatchHelper::all_finalize)
    }
    /// Matches an atom only if another atom matches.
    #[allow(clippy::missing_errors_doc)]
    pub fn match_if_matches<PRE : Atom, R : Atom>(self, pre : PRE, t : R) -> Result<Self, Self> {
        match self.match_atom(pre) {
            Ok(next) => next.match_atom(t),
            Err(same) => Ok(same),
        }
    }
    /// Matches an [`AlwaysAtom`] only if another atom matches.
    ///
    /// This always result in a successful match.
    #[must_use]
    #[allow(clippy::missing_errors_doc)]
    pub fn always_if_matches<PRE : Atom, R : AlwaysAtom>(self, pre : PRE, t : R) -> Self {
        match self.match_atom(pre) {
            Ok(next) => next.match_always(t),
            Err(same) => same,
        }
    }
}

#[cfg(any(feature = "alloc", doc, test))]
impl<T> Atom for alloc::boxed::Box<T> where T : Atom + ?Sized {
    fn parse(&self, st : &str) -> Option<Match>{
        use core::ops::Deref;
        self.deref().parse(st)
    }
}

#[cfg(any(feature = "alloc", doc, test))]
impl<T> AlwaysAtom for alloc::boxed::Box<T> where T : AlwaysAtom + ?Sized {
    fn parse_always(&self, st : &str) -> Match{
        use core::ops::Deref;
        self.deref().parse_always(st)
    }
}

impl<T> Atom for &T where T : Atom + ?Sized {
    fn parse(&self, st : &str) -> Option<Match>{
        (*self).parse(st)
    }
}
impl<T> AlwaysAtom for &T where T : AlwaysAtom + ?Sized {
    fn parse_always(&self, st : &str) -> Match{
        (*self).parse_always(st)
    }
}

/// Matches a string exactly
///
/// ```rust
/// use minparser::prelude::*;
/// let view = Into::<MatchHelper>::into("My data ");
/// view.match_atom("My dat").unwrap().match_atom("a ").unwrap();
/// ```
impl Atom for str {
    fn parse(&self, st : &str) -> Option<Match>{
        if st.starts_with(self) {
            Some(Match{len : self.len()})
        }
        else{
            None
        }
    }
}

/// Matches the first character exactly.
///
/// ```rust
/// use minparser::prelude::MatchHelper;
/// let view = Into::<MatchHelper>::into("My data");
/// view.match_atom('M').unwrap().match_atom('y').unwrap();
/// ```
impl Atom for char {
    fn parse(&self, st : &str) -> Option<Match>{
        st.chars().next().and_then( |chr| {
            if *self == chr {
                Some(Match{len : self.len_utf8()})
            }
            else {
                None
            }
        })
    }
}

impl Atom for fn(char) -> bool {
    fn parse(&self, st : &str) -> Option<Match>{
        st.chars().next().and_then( |c|  {
            if self(c) {
                Some(Match{len : c.len_utf8()})
            }
            else{
                None
            }
        })
    }
}
impl Atom for fn(&char) -> bool {
    fn parse(&self, st : &str) -> Option<Match>{
        st.chars().next().and_then( |c|  {
            if self(&c) {
                Some(Match{len : c.len_utf8()})
            }
            else{
                None
            }
        })
    }
}

impl Atom for fn(MatchHelper<'_>) -> Result<MatchHelper<'_>, MatchHelper<'_>> {
    fn parse(&self, st : &str) -> Option<Match> {
        self(st.into()).ok().map(|help| help.finalize().1)
    }
}
impl Atom for fn(MatchHelper<'_>) -> MatchHelper<'_> {
    fn parse(&self, st : &str) -> Option<Match> {
        Some(self(st.into()).finalize().1)
    }
}
impl AlwaysAtom for fn(MatchHelper<'_>) -> MatchHelper<'_> {
    fn parse_always(&self, st : &str) -> Match {
        self(st.into()).finalize().1
    }
}

/// Matches any of the atoms in the slice.
///
/// ```rust
/// use minparser::prelude::MatchHelper;
/// let view = Into::<MatchHelper>::into("My data");
/// view.match_atom(&['M', 'K', 'O']).unwrap().match_atom(&["ser", "ii", "y d"]).unwrap();
/// ```
impl<R> Atom for [R] where R : Atom {
    fn parse(&self, st : &str) -> Option<Match>{
        for i in self {
            if let Some(Match{len}) = i.parse(st) {
                return Some(Match{len});
            }
        }
        None
    }
}
/// Matches any of the atoms in the array.
///
/// ```rust
/// use minparser::atoms::MatchHelper;
/// let view = Into::<MatchHelper>::into("My data");
/// view.match_atom(['M', 'K', 'O']).unwrap().match_atom(["ser", "ii", "y d"]).unwrap();
/// ```
impl<R, const N : usize> Atom for [R; N] where R : Atom {
    fn parse(&self, st : &str) -> Option<Match>{
        self.as_slice().parse(st)
    }
}
