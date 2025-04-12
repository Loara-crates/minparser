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
    /// The error type is a match is not found
    type Error;

    /// Additional data the match provides.
    type Data : Sized;

    /// The main parsing algorithm.
    ///
    /// # Errors
    /// If no prefix of `st` satisfies this parsing strategy then an error is issued.
    fn parse(&self, st : View<'a, (), F>) -> Result<View<'a, Self::Data, F>, Self::Error>;

    /// Assigns a name to the parsing tool, providing more useful error information.
    fn assign_name<N : Clone + core::fmt::Display + core::fmt::Debug>(self, name : N) -> impl ParseTool<'a, F> where Self : Sized, F : core::fmt::Debug + core::fmt::Display + Clone {
        Named{
            name,
            p : self,
        }
    }
}

impl<'a, T, F> ParseTool<'a, F> for &T where T : ParseTool<'a, F> + ?Sized {
    type Error = T::Error;
    type Data = T::Data;
    fn parse(&self, st : View<'a, (), F>) -> Result<View<'a, Self::Data, F>, Self::Error>{
        (*self).parse(st)
    }
}

impl<'a, F> ParseTool<'a, F> for str {
    type Error = NoMatch<F>;
    type Data = &'a Self;
    fn parse(&self, st : View<'a, (), F>) -> Result<View<'a, Self::Data, F>, Self::Error>{
        st.match_str(self)
    }
}
impl<'a, F> ParseTool<'a, F> for [&str] {
    type Error = NoMatch<F>;
    type Data = &'a str;
    fn parse(&self, st : View<'a, (), F>) -> Result<View<'a, Self::Data, F>, Self::Error>{
        st.match_strs(self)
    }
}
impl<'a, F, const N : usize> ParseTool<'a, F> for [&str; N] {
    type Error = NoMatch<F>;
    type Data = &'a str;
    fn parse(&self, st : View<'a, (), F>) -> Result<View<'a, Self::Data, F>, Self::Error>{
        st.match_strs(self.as_slice())
    }
}
impl<'a, F> ParseTool<'a, F> for char {
    type Error = NoMatch<F>;
    type Data = Self;
    fn parse(&self, st : View<'a, (), F>) -> Result<View<'a, Self::Data, F>, Self::Error>{
        st.match_char(*self)
    }
}
impl<'a, F> ParseTool<'a, F> for [char] {
    type Error = NoMatch<F>;
    type Data = char;
    fn parse(&self, st : View<'a, (), F>) -> Result<View<'a, Self::Data, F>, Self::Error>{
        st.match_chars(self)
    }
}
impl<'a, F, const N : usize> ParseTool<'a, F> for [char; N] {
    type Error = NoMatch<F>;
    type Data = char;
    fn parse(&self, st : View<'a, (), F>) -> Result<View<'a, Self::Data, F>, Self::Error>{
        st.match_chars(self.as_slice())
    }
}

/// Parses only the end of the input
#[derive(Debug, Clone, Copy)]
pub struct EOFTool;

impl<'a, F> ParseTool<'a, F> for EOFTool{
    type Error = ();
    type Data = ();

    fn parse(&self, st : View<'a, (), F>) -> Result<View<'a, Self::Data, F>, Self::Error> {
        if st.is_empty() {
            Ok(st)
        }
        else{
            Err(())
        }
    }
}

/// Matches the empty string, therefore it always matches.
#[derive(Debug, Clone, Copy)]
pub struct TrueParser;

impl<'a, F> ParseTool<'a, F> for TrueParser{
    type Error = core::convert::Infallible;
    type Data = ();

    fn parse(&self, st : View<'a, (), F>) -> Result<View<'a, Self::Data, F>, Self::Error> {
        Ok(st)
    }
}

/// Error type for [`Named`].
#[derive(Debug)]
pub struct NamedError<N, E, F>{
    name : N,
    err : E,
    pos : crate::pos::Position<F>,
}

impl<N : core::fmt::Display, E : core::fmt::Display, F : core::fmt::Display> core::fmt::Display for NamedError<N, E, F> {
    fn fmt(&self, fd : &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fd, "Named rule {} not satisfied at {}: {}", self.name, self.pos, self.err)
    }
}

impl<N : core::fmt::Display + core::fmt::Debug, E : core::error::Error, F : core::fmt::Display + core::fmt::Debug> core::error::Error for NamedError<N, E, F>{}

/// A named parsing tool.
///
/// It assignes a "name" to an already existing parsing tool, in order to provide more information
/// in the error type. Otherwise, it is equivalent to using the underlying parser.
#[derive(Debug, Clone)]
pub struct Named<N, P>{
    name : N,
    p : P,
}

impl<'a, N, P, F> ParseTool<'a, F> for Named<N, P> where P : ParseTool<'a, F>, N : Clone + core::fmt::Display + core::fmt::Debug, F : core::fmt::Debug + core::fmt::Display + Clone {
    type Error = NamedError<N, P::Error, F>;
    type Data = P::Data;

    fn parse(&self, st : View<'a, (), F>) -> Result<View<'a, Self::Data, F>, Self::Error>{
        let pos = st.top_position().clone();
        self.p.parse(st).map_err(|err| NamedError{
            name : self.name.clone(),
            err,
            pos,
        })
    }
    fn assign_name<M : Clone + core::fmt::Display + core::fmt::Debug>(self, name : M) -> impl ParseTool<'a, F> where Self : Sized {
        self.p.assign_name(name)
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

/// Result type for [`OrTool`]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum OrToolData<FD, SD>{
    /// Data returned by first tool.
    First(FD),
    /// Data returned by second tool.
    Second(SD),
}

/// Error type for [`OrTool`]
#[derive(Debug, Clone, Copy)]
pub struct OrToolErr<FE, SE>{
    /// First tool error.
    pub fir : FE,
    /// Second tool error.
    pub sec : SE,
}

impl<'a, F : Clone, FT, ST> ParseTool<'a, F> for OrTool<FT, ST> where FT : ParseTool<'a, F>, ST : ParseTool<'a, F>{
    type Error = OrToolErr<FT::Error, ST::Error>;
    type Data = OrToolData<FT::Data, ST::Data>;
    fn parse(&self, st : View<'a, (), F>) -> Result<View<'a, Self::Data, F>, Self::Error>{
        match self.fir.parse(st.clone()) {
            Ok(d) => Ok(d.map_data(OrToolData::First)),
            Err(e1) => self.sec.parse(st).map(|m| m.map_data(OrToolData::Second)).map_err(|e2| OrToolErr{fir : e1, sec : e2}),
        }
    }
}

#[cfg(any(doc, feature = "alloc"))]
mod allc {
    use super::{ParseTool, TrueParser, View};
    use thiserror::Error;
    /// Tool that matches repetitions with separator
    ///
    /// Separator data is discarded and not saved
    #[derive(Debug, Clone, Copy)]
    pub struct RepeatTool<T, SEP>{
        tool : T,
        sep : SEP,
        min : usize,
        max : Option<usize>,
    }

    /// Error type for [`RepeatTool`]
    #[derive(Debug, Clone, Copy, Error)]
    pub enum RepeatToolErr<TE, SEPE>{
        /// Error of the main tool.
        #[error("{0}")]
        Item(TE),
        /// Error of the tool used for separators.
        #[error("Separator error: {0}")]
        Separator(SEPE),
    }

    impl<T, SEP> RepeatTool<T, SEP>{
        /// Create a new [`RepeatTool`] with specified separator
        pub const fn new_sep(tool : T, sep : SEP, min : usize, max : Option<usize>) -> Self {
            Self{
                tool,
                sep,
                min,
                max,
            }
        }
        /// Create a new [`RepeatTool`] with specified separator and upper bound
        pub const fn new_sep_bounds(tool : T, sep : SEP, min : usize, max : usize) -> Self {
            Self::new_sep(tool, sep, min, Some(max))
        }
        /// Create a new [`RepeatTool`] with specified separator without upper bound
        pub const fn new_sep_unbounded(tool : T, sep : SEP, min : usize) -> Self {
            Self::new_sep(tool, sep, min, None)
        }
        /// Creates a new [`RepeatTool`] which matches an arbitrary number of `tool`.
        pub const fn new_any(tool : T, sep : SEP) -> Self {
            Self::new_sep(tool, sep, 0, None)
        }
        /// Creates a new [`RepeatTool`] which matches at least one occurrence of `tool`.
        pub const fn new_one(tool : T, sep : SEP) -> Self {
            Self::new_sep(tool, sep, 1, None)
        }
    }
    impl<T> RepeatTool<T, TrueParser>{
        /// Create a new [`RepeatTool`] without spaces.
        pub const fn new(tool : T, min : usize, max : Option<usize>) -> Self {
            Self{
                tool,
                sep : TrueParser,
                min,
                max,
            }
        }
        /// Create a new [`RepeatTool`] with specified upper bound
        pub const fn new_bounds(tool : T, min : usize, max : usize) -> Self {
            Self::new(tool, min, Some(max))
        }
        /// Create a new [`RepeatTool`] without upper bound
        pub const fn new_unbounded(tool : T, min : usize) -> Self {
            Self::new(tool, min, None)
        }
        /// Creates a new [`RepeatTool`] which matches zero or one occurrence of `tool`.
        pub const fn new_optional(tool : T) -> Self {
            Self::new(tool, 0, Some(1))
        }
    }

    impl<'a, F : Clone, T, SEP> ParseTool<'a, F> for RepeatTool<T, SEP> where T : ParseTool<'a, F>, SEP : ParseTool<'a, F> {
        type Error = RepeatToolErr<T::Error, SEP::Error>;
        type Data = alloc::vec::Vec<T::Data>;
        fn parse(&self, st : View<'a, (), F>) -> Result<View<'a, Self::Data, F>, Self::Error>{
            let mut ret = alloc::vec::Vec::new();
            if Some(0) == self.max {
                return Ok(st.set_data(ret));
            }
            let mut nst = match self.tool.parse(st.clone()) {
                Ok(d) => d.push(&mut ret),
                Err(e) => if self.min == 0 {
                    return Ok(st.set_data(ret))
                } 
                else {
                    return Err(RepeatToolErr::Item(e))
                },
            };
            for i in 1.. {
                if let Some(m) = self.max {
                    if i >= m {
                        break;
                    }
                }
                nst = match self.sep.parse(nst.clone()) {
                    Ok(d) => d.drop(),
                    Err(e) => if self.min <= i {
                        break
                    }
                    else {
                        return Err(RepeatToolErr::Separator(e))
                    }
                };
                nst = match self.tool.parse(nst.clone()) {
                    Ok(d) => d.push(&mut ret),
                    Err(e) => if self.min <= i {
                        break
                    }
                    else {
                        return Err(RepeatToolErr::Item(e))
                    }
                };
            }
            Ok(nst.set_data(ret))
        }
    }
}
#[cfg(any(doc, feature = "alloc"))]
pub use self::allc::*;

/// Tool that matches characters which satisfies the provided predicate.
///
/// A predicate is a function that accepts `char` and returns an `Option`: the `None` variand will
/// be interpreted as a nonmatch, whereas the `Some(d)` variant will be interpreted as a match and the
/// provided data `d` is stored in the returned [`View`].
#[derive(Debug, Clone, Copy)]
pub struct PredicateTool<P>{
    predicate : P,
}

impl<D, P : Fn(char) -> Option<D>> PredicateTool<P> {
    /// Create a new [`PredicateTool`] from a predicate.
    pub const fn new_map(predicate : P) -> Self {
        Self{
            predicate,
        }
    }
}

#[cfg(any(doc, feature = "alloc"))]
impl<'a> PredicateTool<alloc::boxed::Box<dyn Fn(char) -> Option<char> + 'a>> {
    /// Create a new [`PredicateTool`] from a function returning a `bool`.
    ///
    /// A `false` result is interpreted as a `None` and a `true` result is converted to
    /// `Some(c)` with `c` is the matched character in the view.
    pub fn new_predicate<F : 'a + Fn(char) -> bool>(f : F) -> Self {
        let predicate = alloc::boxed::Box::new(move |i| if f(i) {Some(i)} else {None});
        Self{predicate}
    }
}

impl<'a, F, D, P : Fn(char) -> Option<D>> ParseTool<'a, F> for PredicateTool<P>{
    type Error = ();
    type Data = D;
    fn parse(&self, st : View<'a, (), F>) -> Result<View<'a, Self::Data, F>, Self::Error>{
        let mut chr = '€';
        let nv = st.pop_char().ok_or(())?.save(&mut chr);
        (self.predicate)(chr).map_or(Err(()), |d| Ok(nv.set_data(d)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tools() {
        let st = "€à/a req sey";
        let vw = View::<'_, (), crate::pos::NoFile>::new_default(st);
        vw.clone().match_tool(TrueParser).unwrap();
        assert!(vw.clone().match_tool(EOFTool).is_err());
        assert_eq!(vw.clone().match_tool(&["Zx", "€à/a re"]).unwrap().consume(), "€à/a re");
        assert!(matches!(vw.match_tool::<OrTool<_, &'static str>>(OrTool{fir : EOFTool, sec : "€à"}).unwrap().consume(), OrToolData::Second("€à")));
    }
    #[cfg(feature = "alloc")]
    #[test]
    fn repeat() {
        use alloc::vec;
        let st = "A A A A";
        let vw = View::<'_, (), crate::pos::NoFile>::new_default(st);
        assert_eq!(vw.match_tool(RepeatTool::new_sep_unbounded('A', ' ', 0)).unwrap().consume(), vec!['A','A','A','A']);
    }
}
