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
//! [`View`] and other associated utilities.
use crate::pos::{Position, NoFile};
use crate::atoms::{Atom, Match};
use crate::atomlist::{LazyRepeatAtom};
use core::ops::ControlFlow;
use crate::chains::*;

/// A view on a `str` to be parsed.
///
/// This view carries a [`Position<F>`] with respect of the initial string.
/// Each time a pattern is matched against a `View` object a new `View` object is returned,
/// returning the *remaining* part of the view which follows the matched prefix.
#[derive(Debug, Clone, Copy)]
pub struct View<'a, F = NoFile>{
    pub(crate) view : &'a str,
    pub(crate) pos : Position<F>,
}

/// The standard error type for a missing match at specified position.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub struct PosNoMatch<F = NoFile>{
    /// Error position.
    pub pos : Position<F>,
}

/// [`View`] alias if you don't want to specify a file.
pub type ViewFile<'a> = View<'a, NoFile>;

impl<'a, F : Default> View<'a, F> {
    /// Creates a new [`View`] object.
    #[must_use]
    pub fn new_default(view : &'a str) -> Self {
        Self::new(view, F::default())
    }
}

impl<'a, F> View<'a, F> {
    /// Creates a new [`View`] object with provided *data* and *file*.
    pub const fn new(view : &'a str, file : F) -> Self {
        Self{
            view,
            pos : Position::new_file_zero(file),
        }
    }
    /// Returns the underlying string.
    pub const fn get_view(&self) -> &'a str{
        self.view
    }
    /// Tests if the underlying string is empty.
    pub const fn is_empty(&self) -> bool {
        self.view.is_empty()
    }
    /// Returns the position of the first character with respect to the main file.
    pub const fn top_position(&self) -> &Position<F> {
        &self.pos
    }
    /// Consumes the view and returns the actual [`Position`].
    pub fn into_pos(self) -> Position<F> {
        self.pos
    }
    /// Progress the view and its position.
    ///
    /// # Panics
    /// Panics if `inc` doesn't lie on UTF-8 code point boundaries.
    pub fn progress(self, inc : usize) -> (Self, &'a str) {
        if inc == 0 {
            (self, "")
        }
        else{
            let (pfx, sfx) = self.view.split_at(inc);
            let mut fit = pfx.split('\n');
            let mut elem = fit.next().unwrap();// at least one element
            let mut nls = 0;
            for el in fit {
                elem = el;
                nls += 1;
            }
            let (mut r, mut c, file) = self.pos.unpack();
            if nls > 0 {
                c = 0;
                r += nls;
            }
            c += u32::try_from(elem.len()).unwrap();
            (Self{
                pos : Position::new_file(file, r, c),
                view : sfx,
            }, pfx)
        }
    }
    /// Matches an atom with the view
    ///
    /// # Errors
    /// If a match doesn't happen then the calling view is returned as `Err` unchanged. You can
    /// then use the [`into_pos`](crate::view::View::into_pos) method to get the actual position of
    /// the missing match.
    pub fn match_atom<R : Atom>(self, t : R) -> Result<Self, Self> {
        match t.parse(self.view) {
            Some(Match{len}) => Ok(self.progress(len).0),
            None => Err(self),
        }
    }
    /// Applies the matching atom and returns the prefix matching such tool
    #[allow(clippy::missing_errors_doc)]
    pub fn match_atom_string<R : Atom>(self, t : R) -> Result<(Self, &'a str), PosNoMatch<F>> {
        match t.parse(self.view) {
            Some(Match{len}) => Ok(self.progress(len)),
            None => Err(PosNoMatch{pos : self.pos}),
        }
    }
    /// Matches an atom with the view
    ///
    /// # Errors
    /// If a match doesn't happen then a new error is created from the string that doesn't match
    /// the specified tool and the `Position` at which this happened.
    pub fn match_atom_err<E, R : Atom, FF : FnOnce(&'a str, Position<F>) -> E>(self, t : R, f : FF) -> Result<Self, E> {
        match t.parse(self.view) {
            Some(Match{len}) => Ok(self.progress(len).0),
            None => Err(f(self.view, self.pos)),
        }
    }
    /// Matches a parsing tool
    #[allow(clippy::missing_errors_doc)]
    pub fn match_tool<T : ParseTool<'a, F>>(self, t : &T) -> Result<Self, T::Error> {
        t.parse(self)
    }
    /// Matches a tool only if another tool matches.
    #[allow(clippy::missing_errors_doc)]
    pub fn match_if_matches<PRE : ParseTool<'a, F>, R : ParseTool<'a, F>>(self, pre : PRE, t : R) -> Result<Self, R::Error> where F : Clone {
        match self.clone().match_tool(&pre) {
            Ok(next) => next.match_tool(&t),
            Err(_) => Ok(self),
        }
    }
}

/// Parsing tool trait.
///
/// The main difference with the [`Atom`](crate::atoms::Atom) is that `ParseTool` can fail at
/// multiple positions. If an `Atom` failed to match then the position at which the missing match
/// happened is always at the start of the matching. Instead, `ParseTool` is more specific because
/// it can specify where exactly the match failed. Moreover, `ParseTool` has a custom error type
/// that can store both the position and the string prefix.
pub trait ParseTool<'a, F> {
    /// Error type
    type Error : 'a;

    /// The main parsing algorithm.
    ///
    /// # Errors
    /// If no prefix of `st` satisfies this parsing strategy then `Error` is returned.
    fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, Self::Error>;
}

impl<'a, F, T> Chain<T> for View<'a, F> where T : ParseTool<'a, F>  {
    type Error = T::Error;
    
    fn chain(self, t : &T) -> ControlFlow<T::Error, Self> {
        match self.match_tool(t) {
            Ok(s) => ControlFlow::Continue(s),
            Err(e) => ControlFlow::Break(e),
        }
    }
}

impl<'a, FF, F, S, E> ParseTool<'a, FF> for Seq<F, S> where F : ParseTool<'a, FF, Error = E>, S : ParseTool<'a, FF, Error = E>, E : 'a {
    type Error = E;

    fn parse(&self, st : View<'a, FF>) -> Result<View<'a, FF>, Self::Error> {
        match self.parse_logic(st) {
            ControlFlow::Break(e) => Err(e),
            ControlFlow::Continue(v) => Ok(v)
        }
    }
}
impl<'a, FF, F, S> ParseTool<'a, FF> for Or<F, S> where F : ParseTool<'a, FF>, S : ParseTool<'a, FF>, FF : Clone {
    type Error = S::Error;

    fn parse(&self, st : View<'a, FF>) -> Result<View<'a, FF>, Self::Error> {
        match self.parse_logic(st) {
            ControlFlow::Break(e) => Err(e),
            ControlFlow::Continue(v) => Ok(v)
        }
    }
}

impl<'a, F, T, SEP, E> ParseTool<'a, F> for RepeatAtom<T, SEP> where T : ParseTool<'a, F, Error = E>, SEP : ParseTool<'a, F, Error = E>, E : 'a + Default, F : Clone {
    type Error = E;

    fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, Self::Error> {
        match self.parse_logic::<E, _, _>(st, E::default) {
            ControlFlow::Continue(hh) => Ok(hh.0),
            ControlFlow::Break(e) => Err(e),
        }
    }
}

impl<'a, F, T, SEP> ParseTool<'a, F> for RepeatAnyAtom<T, SEP> where T : ParseTool<'a, F>, SEP : ParseTool<'a, F>, F : Clone {
    type Error = core::convert::Infallible;

    fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, Self::Error> {
        Ok(self.parse_logic(st).0)
    }
}

impl<'a, F, T, SEP, TERM, E> ParseTool<'a, F> for LazyRepeatAtom<T, SEP, TERM> where 
    T : ParseTool<'a, F, Error = E>, 
    SEP : ParseTool<'a, F, Error = E>, 
    TERM : ParseTool<'a, F, Error = E>,
    E : 'a + Default,
    F : Clone {
    type Error = E;

    fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, Self::Error> {
        match self.parse_logic::<E, _, _>(st, E::default) {
            ControlFlow::Continue(hh) => Ok(hh.0),
            ControlFlow::Break(e) => Err(e),
        }
    }
}

/// A wrapper for [`Atom`](crate::atoms::Atom) that implements [`ParseTool`]
pub struct AtomTool<A>(pub A);

impl<'a, F, A> ParseTool<'a, F> for AtomTool<A> where A : Atom, F : 'a {
    type Error = View<'a, F>;

    fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, Self::Error> {
        st.match_atom(&self.0)
    }
}
