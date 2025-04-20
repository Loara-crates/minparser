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
//! Parsing tools
//!
//! This module provides the [`ParseTool`] trait and some parsing tools which incapsulates some
//! basic algorithms which you can use to define more sofisticated ones.
use crate::view::{NoMatch, View};

/// The main parsing trait
///
/// Every object which incapsulate a parsing strategy should implement this trait in order to be
/// composed with other tools.
pub trait ParseTool<'a, F> {
    /// The main parsing algorithm.
    ///
    /// # Errors
    /// If no prefix of `st` satisfies this parsing strategy then an error is issued.
    fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, NoMatch<F>>;

    /// Coerce to dyn trait
    fn as_dyn(&self) -> &dyn ParseTool<'a, F> where Self : Sized {
        self
    }
}

impl<'a, T, F> ParseTool<'a, F> for &T where T : ParseTool<'a, F> + ?Sized {
    fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, NoMatch<F>>{
        (*self).parse(st)
    }
}

/// Matches a string exactly
impl<'a, F> ParseTool<'a, F> for str {
    fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, NoMatch<F>>{
        if st.view.starts_with(self) {
            Ok(st.progress(self.len()).0)
        }
        else{
            Err(NoMatch{pos : st.pos})
        }
    }
}

/// Matches the first character exactly
impl<'a, F> ParseTool<'a, F> for char {
    fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, NoMatch<F>>{
        match st.view.chars().next() {
            Some(chr) => {
                if *self == chr {
                    Ok(st.progress(chr.len_utf8()).0)
                }
                else {
                    Err(NoMatch{pos : st.pos})
                }
            }
            _ => Err(NoMatch{pos : st.pos}),
        }
    }
}

/// Matches any of the tools in the slice.
impl<'a, F : Clone, R> ParseTool<'a, F> for [R] where R : ParseTool<'a, F> {
    fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, NoMatch<F>>{
        st.match_any_tool(self).map(|i| i.0)
    }
}

/// Matches any of the tools in the array.
impl<'a, F : Clone, R, const N : usize> ParseTool<'a, F> for [R; N] where R : ParseTool<'a, F> {
    fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, NoMatch<F>>{
        st.match_any_tool(self).map(|i| i.0)
    }
}

/// Parses only the end of the input
#[derive(Debug, Clone, Copy)]
pub struct EOFTool;

impl<'a, F> ParseTool<'a, F> for EOFTool{
    fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, NoMatch<F>> {
        if st.is_empty() {
            Ok(st)
        }
        else{
            Err(st.no_match())
        }
    }
}

/// Matches the empty string, therefore it always matches.
#[derive(Debug, Clone, Copy)]
pub struct TrueParser;

impl<'a, F> ParseTool<'a, F> for TrueParser{
    fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, NoMatch<F>> {
        Ok(st)
    }
}

/// Tool that matches at least one of two subtools
///
/// Ordering matters: if the first one matches then the second one is not tried.
#[derive(Debug, Clone, Copy)]
pub struct OrTool<F, S>{
    /// First tool to be tested.
    pub fir : F,
    /// Second tool to be tested.
    pub sec : S,
}

impl<'a, F : Clone, FT, ST> ParseTool<'a, F> for OrTool<FT, ST> where FT : ParseTool<'a, F>, ST : ParseTool<'a, F>{
    fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, NoMatch<F>>{
        self.fir.parse(st.clone()).or_else(|_| self.sec.parse(st))
    }
}

/// Tool that matches at least one of many subtools
///
/// Ordering matters: if one matches then the followings are not tried.
#[derive(Debug, Clone, Copy)]
pub struct Or<Tup>(
    /// Tools to be tested.
    pub Tup
);
/// Tool that matches a sequence of subtools
#[derive(Debug, Clone, Copy)]
pub struct SeqTool<Tup>(
    /// Tools to be tested.
    pub Tup
);

macro_rules! or_tuple {
    {} => {
        impl<'a, F> ParseTool<'a, F> for Or<()> {
            fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, NoMatch<F>> {
                Err(NoMatch{ pos : st.pos})
            }
        }
        impl<'a, F> ParseTool<'a, F> for SeqTool<()> {
            fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, NoMatch<F>> {
                Ok(st)
            }
        }
    };
    {$a:ident} => {
        or_tuple!{}
        impl<'a, F, $a> ParseTool<'a, F> for Or<($a,)> where $a : ParseTool<'a, F> {
            fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, NoMatch<F>> {
                self.0.0.parse(st)
            }
        }
        impl<'a, F, $a> ParseTool<'a, F> for SeqTool<($a,)> where $a : ParseTool<'a, F> {
            fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, NoMatch<F>> {
                self.0.0.parse(st)
            }
        }
    };
    {$a:ident, $($r:ident),+} => {
        or_tuple!{$($r),+}
        #[allow(non_snake_case)]
        impl<'a, FF : Clone, $a, $($r),+> ParseTool<'a, FF> for Or<($a, $($r),+)> where $a : ParseTool<'a, FF>, $($r : ParseTool<'a, FF>),+ {
            fn parse(&self, st : View<'a, FF>) -> Result<View<'a, FF>, NoMatch<FF>> {
                let ($a, $($r),+) = &self.0;
                $a.parse(st.clone()).or_else(|_| {
                    Or(($($r),+,)).parse(st)
                })
            }
        }
        #[allow(non_snake_case)]
        impl<'a, FF, $a, $($r),+> ParseTool<'a, FF> for SeqTool<($a, $($r),+)> where $a : ParseTool<'a, FF>, $($r : ParseTool<'a, FF>),+ {
            fn parse(&self, st : View<'a, FF>) -> Result<View<'a, FF>, NoMatch<FF>> {
                let ($a, $($r),+) = &self.0;
                $a.parse(st).and_then(|rst| {
                    SeqTool(($($r),+,)).parse(rst)
                })
            }
        }
    }
}

or_tuple!{A, B, C, D, E, F, G, H, I, J, K, L, M, N, O}

/// Tool that matches repetitions with separator
///
/// *Disclaimer*: in order to avoid endless recursion the EOF token (matched only by the empty string
/// `""`) is **never** considered a  match for the `T` tool, even if normally ampty strings matches `T`.
#[derive(Debug, Clone, Copy)]
pub struct RepeatTool<T, SEP>{
    tool : T,
    sep : SEP,
    max : Option<usize>,
}

impl<T, SEP> RepeatTool<T, SEP>{
    /// Create a new [`RepeatTool`] with specified separator
    pub const fn new_sep(tool : T, sep : SEP, max : Option<usize>) -> Self {
        Self{
            tool,
            sep,
            max,
        }
    }
    /// Create a new [`RepeatTool`] with specified separator and upper bound
    pub const fn new_sep_bounds(tool : T, sep : SEP, max : usize) -> Self {
        Self::new_sep(tool, sep, Some(max))
    }
    /// Create a new [`RepeatTool`] with specified separator without upper bound
    pub const fn new_sep_unbounded(tool : T, sep : SEP) -> Self {
        Self::new_sep(tool, sep, None)
    }
    /// Creates a new [`RepeatTool`] which matches an arbitrary number of `tool`.
    pub const fn new_any(tool : T, sep : SEP) -> Self {
        Self::new_sep(tool, sep, None)
    }
}
impl<T> RepeatTool<T, TrueParser>{
    /// Create a new [`RepeatTool`] without spaces.
    pub const fn new(tool : T, max : Option<usize>) -> Self {
        Self{
            tool,
            sep : TrueParser,
            max,
        }
    }
    /// Create a new [`RepeatTool`] with specified upper bound
    pub const fn new_bounds(tool : T, max : usize) -> Self {
        Self::new(tool, Some(max))
    }
    /// Create a new [`RepeatTool`] without upper bound
    pub const fn new_unbounded(tool : T) -> Self {
        Self::new(tool, None)
    }
    /// Creates a new [`RepeatTool`] which matches zero or one occurrence of `tool`.
    pub const fn new_optional(tool : T) -> Self {
        Self::new(tool, Some(1))
    }
}

impl<'a, F : Clone, T, SEP> ParseTool<'a, F> for RepeatTool<T, SEP> where T : ParseTool<'a, F>, SEP : ParseTool<'a, F> {
    fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, NoMatch<F>>{
        match self.max {
            None => {
                let (ret, _, _) = st.repeat_match(&self.tool, &self.sep);
                Ok(ret)
            }
            Some(m) => {
                let (ret, _, _) = st.repeat_match_up(&self.tool, &self.sep, m);
                Ok(ret)
            }
        }
    }
}

/// Tool that matches repetitions with separator requiring a minimum number of repetitions
///
/// Separator data is discarded and not saved
#[derive(Debug, Clone, Copy)]
pub struct RepeatToolMin<T, SEP>{
    tool : T,
    sep : SEP,
    min : usize,
    max : Option<usize>,
}

impl<T, SEP> RepeatToolMin<T, SEP>{
    /// Create a new [`RepeatToolMin`] with specified separator
    ///
    /// # Panics
    /// Panic if `max` is strictly lesser than `min`.
    pub const fn new_sep(tool : T, sep : SEP, min : usize, max : Option<usize>) -> Self {
        if let Some(m) = max {
            assert!(m >= min, "Max is strictly lesser than min");
        }
        Self{
            tool,
            sep,
            min,
            max,
        }
    }
    /// Create a new [`RepeatToolMin`] with specified separator and upper bound
    pub const fn new_sep_bounds(tool : T, sep : SEP, min : usize, max : usize) -> Self {
        Self::new_sep(tool, sep, min, Some(max))
    }
    /// Create a new [`RepeatToolMin`] with specified separator without upper bound
    pub const fn new_sep_unbounded(tool : T, sep : SEP, min : usize) -> Self {
        Self::new_sep(tool, sep, min, None)
    }
    /// Create a new [`RepeatToolMin`] that matches at least one occurrence
    pub const fn new_any_one(tool : T, sep : SEP) -> Self {
        Self::new_sep(tool, sep, 1, None)
    }
}
impl<T> RepeatToolMin<T, TrueParser>{
    /// Create a new [`RepeatToolMin`] without spaces.
    ///
    /// # Panics
    /// Panic if `max` is strictly lesser than `min`.
    pub const fn new(tool : T, min : usize, max : Option<usize>) -> Self {
        if let Some(m) = max {
            assert!(m >= min, "Max is strictly lesser than min");
        }
        Self{
            tool,
            sep : TrueParser,
            min,
            max,
        }
    }
    /// Create a new [`RepeatToolMin`] with specified upper bound
    pub const fn new_bounds(tool : T, min : usize, max : usize) -> Self {
        Self::new(tool, min, Some(max))
    }
    /// Create a new [`RepeatToolMin`] without upper bound
    pub const fn new_unbounded(tool : T, min : usize) -> Self {
        Self::new(tool, min, None)
    }
}

impl<'a, F : Clone, T, SEP> ParseTool<'a, F> for RepeatToolMin<T, SEP> where T : ParseTool<'a, F>, SEP : ParseTool<'a, F> {
    fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, NoMatch<F>>{
        match self.max {
            None => {
                let (ret, i, e) = st.repeat_match(&self.tool, &self.sep);
                if i < self.min {
                    Err(e.into())
                }
                else {
                    Ok(ret)
                }
            }
            Some(m) => {
                let (ret, i, oe) = st.repeat_match_up(&self.tool, &self.sep, m);
                if let Some(e) = oe {
                    if i < self.min {
                        Err(e.into())
                    }
                    else{
                        Ok(ret)
                    }
                }
                else {
                    Ok(ret)
                }
            }
        }
    }
}
/// Tool that matches characters which satisfies the provided predicate.
#[derive(Debug, Clone, Copy)]
pub struct PredicateTool<P>{
    predicate : P,
}

impl<P : Fn(char) -> bool> PredicateTool<P> {
    /// Create a new [`PredicateTool`] from a predicate.
    pub const fn new(predicate : P) -> Self {
        Self{
            predicate,
        }
    }
}

impl<'a, F : Clone, P : Fn(char) -> bool> ParseTool<'a, F> for PredicateTool<P>{
    fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, NoMatch<F>>{
        let (nv, oc) = st.clone().pop_char();
        match oc {
            None => Err(NoMatch{pos : st.pos}),
            Some(c) => {
                if (self.predicate)(c) {
                    Ok(nv)
                }
                else{
                    Err(NoMatch{pos : st.pos})
                }
            }
        }
    }
}

/// Tool that wraps a function that accepts a [`View`] object.
///
/// It can be used to join different tools and threat them as a unique tool
#[derive(Debug, Copy, Clone)]
pub struct JoinTool<S>(S);

impl<S>  JoinTool<S> {
    /// Creates a new [`JoinTool`]
    #[must_use]
    pub const fn new(s : S) -> Self {
        Self(s)
    }
}
impl<'a, F, S : Fn(View<'a, F>) -> Result<View<'a, F>, NoMatch<F>>> ParseTool<'a, F> for JoinTool<S> {
    fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, NoMatch<F>> {
        (self.0)(st)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tools() {
        let st = "€à/a req sey";
        let vw = View::<'_, crate::pos::NoFile>::new_default(st);
        vw.clone().match_tool(TrueParser).unwrap();
        assert!(vw.clone().match_tool(EOFTool).is_err());
        assert_eq!(vw.clone().match_any_tool(&["Zx", "€à/a re"]).unwrap().1, 1);
        assert_eq!(vw.clone().match_tool_string(Or((EOFTool, "€à", "€"))).unwrap().1, "€à");
        assert!(vw.clone().match_tool(Or(('r', "€àb"))).is_err());
        assert_eq!(vw.clone().match_tool_string(SeqTool(("€à", "/a re", "q se", 'y', EOFTool))).unwrap().1, "€à/a req sey");
    }
}
