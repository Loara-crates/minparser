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
//! Parsing tools with custom errors.
//!
//! This module provides some of the objects in [`tools`](crate::tools) module but with custom error types.

use ToolResult::{Match, NoMatch};

/// Return type of a [`ParseTool`].
pub enum ToolResult<E>{
    /// A successful match.
    Match{
        /// The length in bytes of the matching prefix.
        len : usize,
    },
    /// A failed match.
    NoMatch(E)
}
/// Return type of a [`ParseToolData`].
pub enum ToolResultData<D, E>{
    /// A successful match.
    Match{
        /// The length in bytes of the matching prefix.
        len : usize,
        /// The carried data.
        data : D,
    },
    /// A failed match.
    NoMatch(E)
}

impl<E> ToolResult<E>{
    /// Convert to a compatible error type
    #[must_use]
    pub fn into_tr<F>(self) -> ToolResult<F> where E : Into<F> {
        match self{
            Self::Match{len} => Match{len},
            Self::NoMatch(e) => NoMatch(e.into()),
        }
    }
    /// Apply `f` to a successful match.
    #[must_use]
    pub fn map<F : FnOnce(usize) -> usize>(self, f : F) -> Self {
        match self {
            Self::Match{len} => Self::Match{ len : f(len)},
            Self::NoMatch(e) => Self::NoMatch(e),
        }
    }
    /// Apply `f` to a missed match.
    #[must_use]
    pub fn map_err<G, F : FnOnce(E) -> G>(self, f : F) -> ToolResult<G> {
        match self {
            Self::Match{len} => Match{ len},
            Self::NoMatch(e) => NoMatch(f(e)),
        }
    }
    /// Apply `f` to a successful match and generate data.
    #[must_use]
    pub fn map_data<D, F : FnOnce(usize) -> (usize, D)>(self, f : F) -> ToolResultData<D, E> {
        match self {
            Self::Match{len: i} => {
                let (len, data) = f(i);
                ToolResultData::Match{ len, data}
            }
            Self::NoMatch(e) => ToolResultData::NoMatch(e),
        }
    }
    /// Call `f` if the match is successful, otherwise return `NoMatch`.
    #[must_use]
    pub fn and_then<F : FnOnce(usize) -> Self>(self, f : F) -> Self {
        match self {
            Self::Match{len} => f(len),
            Self::NoMatch(e) => Self::NoMatch(e),
        }
    }
    /// Call `f` if the match is not successful, otherwise return the `Match` variant of `self`.
    #[must_use]
    pub fn or_else<EE, F : FnOnce(E) -> ToolResult<EE>>(self, f : F) -> ToolResult<EE> {
        match self {
            Self::Match{len} => Match{len},
            Self::NoMatch(e) => f(e),
        }
    }
}

impl ToolResult<core::convert::Infallible> {
    /// Convert to any type
    #[must_use]
    pub const fn into_any<F>(self) -> ToolResult<F> {
        match self{
            Self::Match{len} => Match{len},
            Self::NoMatch(_) => unreachable!(),
        }
    }
}

impl<D, E> ToolResultData<D, E> {
    /// Convert to a compatible data and error types.
    #[must_use]
    pub fn into_tr<S, F>(self) -> ToolResultData<S, F> where D : Into<S>, E : Into<F> {
        match self{
            Self::Match{len, data} => ToolResultData::Match{len, data : data.into()},
            Self::NoMatch(e) => ToolResultData::NoMatch(e.into()),
        }
    }
    /// Drops the data field.
    #[must_use]
    pub fn drop_data(self) -> ToolResult<E> {
        match self {
            Self::Match{len, ..} => Match{len},
            Self::NoMatch(e) => NoMatch(e),
        }
    }
}

/// The main parsing trait
///
/// Every object which incapsulate a parsing strategy should implement this trait in order to be
/// composed with other tools.
pub trait ParseTool {
    /// Error type.
    type Error;
    /// The main parsing algorithm.
    ///
    /// If there is a match, a `Result::Ok` is returned with the length *in bytes* of the matched
    /// prefix. Notice that a zero may return if the empty string `""` is a valid match for this
    /// tool.
    ///
    /// # Errors
    /// If no prefix of `st` satisfies this parsing strategy then an error is issued.
    fn parse(&self, st : &str) -> ToolResult<Self::Error>;

    /// Coerce to dyn trait
    fn as_dyn(&self) -> &dyn ParseTool<Error = Self::Error> where Self : Sized {
        self
    }

    /// Convert to a tool that drops its error on missing matches.
    #[allow(clippy::type_complexity)]
    fn into_drop(self) -> ErrorTool<Self, (), fn(Self::Error, &str) -> ()> where Self : Sized {
        ErrorTool(self, |_, _| (), core::marker::PhantomData)
    }
    /// Convert to a tool with a convertible error.
    #[allow(clippy::type_complexity)]
    fn into_conv<E>(self) -> ErrorTool<Self, E, fn(Self::Error, &str) -> E> where Self : Sized, Self::Error : Into<E> {
        ErrorTool(self, |e, _| e.into(), core::marker::PhantomData)
    }
    /// Convert to a tool that only checks the match, without progression.
    fn only_check(self) -> CheckTool<Self> where Self : Sized {
        CheckTool(self)
    }
    /// Convert to a tool that doesn't match when the matching result would have zero length.
    fn force_nonempty(self) -> NonEmpty<Self> where Self : Sized {
        NonEmpty(self)
    }
}

/// A tool that always matches.
///
/// Note: even if the tool always matches the match length may be equal to 0, which sometimes can
/// be interpreted as a failed match.
///
/// If An object implements this trait, then [`ParseTool`] should be implemented as follows:
///
/// ```
/// use minparser::prelude_new::*;
/// struct A;
///
/// impl AlwaysParseTool for A {
///     fn parse_always(&self, st : &str) -> usize { todo!(); }
/// }
/// 
/// impl ParseTool for A {
///     type Error = core::convert::Infallible;
///     fn parse(&self, st : &str) -> ToolResult<Self::Error> { 
///         ToolResult::Match{len : self.parse_always(st)}
///     }
/// }
/// ```
pub trait AlwaysParseTool : ParseTool<Error = core::convert::Infallible> {
    /// Returns the length of the match
    fn parse_always(&self, st : &str) -> usize;
}

macro_rules! always_parse {
    ($i:ident) => {
        impl ParseTool for $i {
            type Error = core::convert::Infallible;
            fn parse(&self, st : &str) -> ToolResult<Self::Error> { 
                Match{len : self.parse_always(st)}
            }
        }
    }
}


/// A tool that can generate additional `Data` after a successful match.
///
/// Type `P` can be used not only to pass additional information to generate the `Data`, but can be
/// used to provide multiple implementation of this trait for a single object and to disambiguate
/// between them.
pub trait ParseToolData<'a, P> {
    /// The data that is generate at a successful match.
    type Data;
    /// Error type
    type Error;

    /// The main parsing algorithm.
    fn parse(&self, st : &'a str, par : P) -> ToolResultData<Self::Data, Self::Error>;
}

/// A tool wrapper that changes the error types on missed matches.
#[derive(Debug)]
pub struct ErrorTool<T, E, F>(T, F, core::marker::PhantomData<fn(E) -> F>) where T : ParseTool, F : Fn(T::Error, &str) -> E;

impl<T : ParseTool, E, F : Fn(T::Error, &str) -> E> ErrorTool<T, E, F> {
    /// Creates a new [`ErrorTool`].
    pub fn new(tool : T, fun : F) -> Self {
        Self(tool, fun, core::marker::PhantomData)
    }
}

impl<T, E, F> Clone for ErrorTool<T, E, F> where T : ParseTool + Clone, F : Fn(T::Error, &str) -> E + Clone {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone(), core::marker::PhantomData)
    }
}
impl<T, E, F> Copy for ErrorTool<T, E, F> where T : ParseTool + Copy, F : Fn(T::Error, &str) -> E + Copy {}

impl<T, E, F> ParseTool for ErrorTool<T, E, F> where T : ParseTool, F : Fn(T::Error, &str) -> E {
    type Error = E;

    fn parse(&self, st : &str) -> ToolResult<Self::Error> {
        self.0.parse(st).map_err(|e| (self.1)(e, st))
    }
}

impl<T> ParseTool for &T where T : ParseTool + ?Sized {
    type Error = T::Error;
    fn parse(&self, st : &str) -> ToolResult<Self::Error>{
        (*self).parse(st)
    }
}

#[cfg(any(feature = "alloc", doc, test))]
impl<T> ParseTool for alloc::boxed::Box<T> where T : ParseTool + ?Sized {
    type Error = T::Error;

    fn parse(&self, st : &str) -> ToolResult<Self::Error>{
        use core::ops::Deref;
        self.deref().parse(st)
    }
}
#[cfg(any(feature = "alloc", doc, test))]
impl<T> AlwaysParseTool for alloc::boxed::Box<T> where T : AlwaysParseTool + ?Sized {

    fn parse_always(&self, st : &str) -> usize{
        use core::ops::Deref;
        self.deref().parse_always(st)
    }
}


/// Matches a string prefix.
///
/// ```rust
/// use minparser::prelude_new::*;
/// let st = "My data ";
/// ViewFile::new_default(st).match_tool("My dat").unwrap().match_tool("a ").unwrap();
/// ```
impl ParseTool for str {
    type Error = ();
    fn parse(&self, st : &str) -> ToolResult<Self::Error>{
        if st.starts_with(self) {
            Match{len : self.len()}
        }
        else{
            NoMatch(())
        }
    }
}

/// Matches the first character.
///
/// ```rust
/// use minparser::prelude_new::*;
/// let st = "My data ";
/// ViewFile::new_default(st).match_tool('M').unwrap().match_tool('y').unwrap();
/// ```
impl ParseTool for char {
    type Error = ();

    fn parse(&self, st : &str) -> ToolResult<Self::Error>{
        st.chars().next().map_or(NoMatch(()), |chr| {
            if *self == chr {
                Match{len : self.len_utf8()}
            }
            else {
                NoMatch(())
            }
        })
    }
}

impl ParseTool for fn(char) -> bool {
    type Error = ();

    fn parse(&self, st : &str) -> ToolResult<Self::Error>{
        st.chars().next().map_or(NoMatch(()), |c|  {
            if self(c) {
                Match{len : c.len_utf8()}
            }
            else{
                NoMatch(())
            }
        })
    }
}
impl ParseTool for fn(&char) -> bool {
    type Error = ();

    fn parse(&self, st : &str) -> ToolResult<Self::Error>{
        st.chars().next().map_or(NoMatch(()), |c| {
            if self(&c) {
                Match{len : c.len_utf8()}
            }
            else{
                NoMatch(())
            }
        })
    }
}

/// Matches any of the tools in the slice.
///
/// ```rust
/// use minparser::prelude_new::*;
/// let st = "My data ";
/// ViewFile::new_default(st).match_tool(&['M', 'K', 'O']).unwrap().match_tool(&["ser", "ii", "y d"]).unwrap();
/// ```
impl<R> ParseTool for [R] where R : ParseTool {
    type Error = ();

    fn parse(&self, st : &str) -> ToolResult<Self::Error>{
        for i in self {
            if let Match{len} = i.parse(st) {
                return Match{len};
            }
        }
        NoMatch(())
    }
}

/// Matches any of the tools in the array.
///
/// ```rust
/// use minparser::prelude_new::*;
/// let st = "My data ";
/// ViewFile::new_default(st).match_tool(['M', 'K', 'O']).unwrap().match_tool(["ser", "ii", "y d"]).unwrap();
/// ```
impl<R, const N : usize> ParseTool for [R; N] where R : ParseTool {
    type Error = ();

    fn parse(&self, st : &str) -> ToolResult<Self::Error>{
        self.as_slice().parse(st)
    }
}

/// Parses only the end of the input
///
/// ```rust
/// use minparser::prelude_new::*;
///
/// let st = "My data ";
/// ViewFile::new_default(st).match_tool("My data ").unwrap()
///     .match_tool(EOFTool).unwrap();
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct EOFTool;

impl ParseTool for EOFTool{
    type Error = ();

    fn parse(&self, st : &str) -> ToolResult<Self::Error> {
        if st.is_empty() {
            Match{len : 0}
        }
        else{
            NoMatch(())
        }
    }
}

/// Only checks the provided tool, without progressing.
///
/// ```rust
/// use minparser::prelude_new::*;
///
/// ViewFile::new_default("a").match_tool(CheckTool('a')).unwrap()
///     .match_tool(CheckTool::<fn(&char) -> bool>(char::is_ascii)).unwrap();
/// ```
#[derive(Debug, Copy, Clone, Default)]
pub struct CheckTool<T>(pub T);

impl<T> ParseTool for CheckTool<T> where T : ParseTool {
    type Error = T::Error;
    fn parse(&self, st : &str) -> ToolResult<Self::Error> {
        self.0.parse(st).map(|_| 0)
    }
}

/// Matches only if the provided tool doesn't match.
///
/// ```rust
/// use minparser::prelude_new::*;
///
/// ViewFile::new_default("ab").match_tool(CheckInvTool('c')).unwrap()
///     .match_tool(CheckInvTool('b')).unwrap();
/// ```
#[derive(Debug, Copy, Clone, Default)]
pub struct CheckInvTool<T>(pub T);

impl<T> ParseTool for CheckInvTool<T> where T : ParseTool {
    type Error = ();

    fn parse(&self, st : &str) -> ToolResult<Self::Error>{
        match self.0.parse(st) {
            Match{..} => NoMatch(()),
            NoMatch(_) => Match{len : 0},
        }
    }
}

/// Matches any character.
///
/// It is the exact opposite of [`EOFTool`] that is one matches if and only if the other misses.
#[derive(Debug, Copy, Clone, Default)]
pub struct AnyTool;

impl ParseTool for AnyTool {
    type Error = ();

    fn parse(&self, st : &str) -> ToolResult<Self::Error>{
        st.chars().next().map_or(NoMatch(()), |c|  Match{len : c.len_utf8()})
    }
}

/// Discards empty strings from a match.
///
/// Some tools that needs to match an undefined number of other tools (like [`RepeatTool`] or
/// [`LazyRepeatTool`]) may enter in an infinite loop if the inner tool matches an empty string `""`. 
/// In that case indeed there always be a match but the tool does not progress, resulting so in an
/// endless cycle.
///
/// This tool takes another tool and converts any match with an empty string with a missing match,
/// avoiding so the issue.
#[derive(Debug, Clone, Copy, Default)]
pub struct NonEmpty<P>(pub P);

impl<P : ParseTool> ParseTool for NonEmpty<P> {
    type Error = Option<P::Error>;

    fn parse(&self, st: &str) -> ToolResult<Self::Error> {
        self.0.parse(st)
            .map_err(Some)
            .and_then(|len| if len > 0 {Match{len}} else {NoMatch(None)})
    }
}

/// Matches the empty string, therefore it always matches.
///
/// ```rust
/// use minparser::prelude_new::*;
/// let st = "My data ";
/// ViewFile::new_default(st).match_tool(TrueTool).unwrap()
///     .match_tool_final("My data ").unwrap();
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct TrueTool;

impl AlwaysParseTool for TrueTool{
    fn parse_always(&self, _ : &str) -> usize {
        0
    }
}
always_parse!(TrueTool);

/// Tool that matches at least one of many subtools.
///
/// This crate provide implementation for tuples with up to 15 elements, but 
/// thanks to associativity property you can use nested tuples to support 
/// more tools.
///
/// *Ordering matters here*: if one matches then the followings are not tried. 
///
/// ```rust
/// use minparser::prelude_new::*;
/// let st = "My data ";
/// ViewFile::new_default(st).match_tool(Or(('Z', "My", 'M')))
/// .unwrap().match_tool_final(" data ").unwrap();
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct Or<Tup>(
    /// Tuple of tools to be tested.
    pub Tup
);

/// Tool that matches a sequence of subtools.
///
/// This crate provide implementation for tuples with up to 15 elements, but 
/// thanks to associativity property you can use nested tuples to support 
/// more tools.
#[derive(Debug, Clone, Copy, Default)]
pub struct SeqTool<Tup>(
    /// Tuple of tools to be tested.
    pub Tup
);

macro_rules! or_tuple {
    {} => {
        impl ParseTool for Or<()> {
            type Error = ();
            fn parse(&self, _ : &str) -> ToolResult<Self::Error> {
                NoMatch(())
            }
        }
        impl ParseTool for SeqTool<()> {
            type Error = core::convert::Infallible;
            fn parse(&self, _ : &str) -> ToolResult<Self::Error> {
                Match{len : 0}
            }
        }
    };
    {$a:ident} => {
        or_tuple!{}
        impl<$a> ParseTool for Or<($a,)> where $a : ParseTool {
            type Error = ();
            fn parse(&self, st : &str) -> ToolResult<Self::Error> {
                self.0.0.parse(st).map_err(|_| ())
            }
        }
        impl<$a> ParseTool for SeqTool<($a,)> where $a : ParseTool {
            type Error = ();
            fn parse(&self, st : &str) -> ToolResult<Self::Error> {
                self.0.0.parse(st).map_err(|_| ())
            }
        }
    };
    {$a:ident, $($r:ident),+} => {
        or_tuple!{$($r),+}
        #[allow(non_snake_case)]
        impl<$a, $($r),+> ParseTool for Or<($a, $($r),+)> where $a : ParseTool, $($r : ParseTool),+ {
            type Error = ();
            fn parse(&self, st : &str) -> ToolResult<Self::Error> {
                let ($a, $($r),+) = &self.0;
                $a.parse(st).or_else(|_| Or(($($r),+,)).parse(&st) )
            }
        }
        #[allow(non_snake_case)]
        impl<$a, $($r),+> ParseTool for SeqTool<($a, $($r),+)> where $a : ParseTool, $($r : ParseTool),+ {
            type Error = ();
            fn parse(&self, st : &str) -> ToolResult<Self::Error> {
                let ($a, $($r),+) = &self.0;
                $a.parse(st).map_err(|_| ()).and_then(|rlen| SeqTool(($($r),+,)).parse(&st[rlen..]).map(|len| len+rlen) )
            }
        }
    }
}

or_tuple!{A, B, C, D, E, F, G, H, I, J, K, L, M, N, O}



/// Tool that matches repetitions with separator.
///
/// Like [`RepeatTool`] but without specifying a minimum number of repetitions. Therefore. it will
/// always match.
#[derive(Debug, Clone, Copy)]
pub struct RepeatAnyTool<T, SEP>{
    tool : T,
    sep : SEP,
    max : Option<usize>,
}

impl<T, SEP> RepeatAnyTool<T, SEP>{
    /// Create a new [`RepeatAnyTool`] with specified separator.
    pub const fn new_sep(tool : T, sep : SEP, max : Option<usize>) -> Self {
        Self{
            tool,
            sep,
            max,
        }
    }
    /// Create a new [`RepeatAnyTool`] with specified separator and upper bound
    pub const fn new_sep_bounds(tool : T, sep : SEP, max : usize) -> Self {
        Self::new_sep(tool, sep, Some(max))
    }
    /// Create a new [`RepeatAnyTool`] with specified separator without upper bound
    pub const fn new_sep_unbounded(tool : T, sep : SEP) -> Self {
        Self::new_sep(tool, sep, None)
    }
}
impl<T> RepeatAnyTool<T, TrueTool>{
    /// Create a new [`RepeatAnyTool`] without spaces.
    pub const fn new(tool : T, max : Option<usize>) -> Self {
        Self{
            tool,
            sep : TrueTool,
            max,
        }
    }
    /// Create a new [`RepeatAnyTool`] with specified upper bound
    pub const fn new_bounds(tool : T, max : usize) -> Self {
        Self::new(tool, Some(max))
    }
    /// Create a new [`RepeatAnyTool`] without upper bound
    pub const fn new_unbounded(tool : T) -> Self {
        Self::new(tool, None)
    }
}

impl<T, SEP> AlwaysParseTool for RepeatAnyTool<T, SEP> where T : ParseTool, SEP : ParseTool {
    fn parse_always(&self, mut st : &str) -> usize{
        if self.max == Some(0) {
            return 0;
        }
        let mut len = 0;
        let mut start = 0;
        if let Match{len : ilen} = self.tool.parse(st) {
            st = &st[ilen..];
            len += ilen;
            start += 1;
        }
        else{
            return 0;
        }
        if let Some(max) = self.max {
            for _ in start..max {
                if let Match{len : splen} = self.sep.parse(st) {
                    if let Match{len : tlen} = self.tool.parse(&st[splen..]) {
                        st = &st[(splen + tlen)..];
                        len += splen + tlen;
                    }
                    else{
                        return len;
                    }
                }
                else{
                    return len;
                }
            }
            return len;
        }
        loop {
            if let Match{len : splen} = self.sep.parse(st) {
                if let Match{len : tlen} = self.tool.parse(&st[splen..]) {
                    st = &st[(splen + tlen)..];
                    len += splen + tlen;
                }
                else{
                    return len;
                }
            }
            else{
                return len;
            }
        }
    }
}

impl<T, SEP> ParseTool for RepeatAnyTool<T, SEP> where T : ParseTool, SEP : ParseTool {
    type Error = core::convert::Infallible;

    fn parse(&self, st : &str) -> ToolResult<Self::Error> {
        Match{len : self.parse_always(st)}
    }
}

/// Error type for [`RepeatTool`].
#[derive(Debug, Copy, Clone, thiserror::Error)]
pub enum RepeatToolErr{
    /// The minimum number of repetitions is not reached.
    #[error("Insufficient repetition number, required: {min} found: {fnd}")]
    Min{
        /// Minimum number of repetitions
        min: usize,
        /// Repetitions reached
        fnd: usize
    },
    /// No match is possible because the maximum number of repetitions is strictly lesser than the
    /// minimum one.
    #[error("No match possible because minimum {min} is strictly greater than maximum {max}")]
    MinMax{
        /// Minimum number of repetitions
        min: usize,
        /// Maximum number of repetitions
        max : usize
    }
}

/// Tool that matches repetitions with separator requiring a minimum number of repetitions.
///
/// ```rust
/// use minparser::prelude_new::*;
///
/// let lt = ViewFile::new_default("a a a a b");
/// assert_eq!(lt.match_tool_string(RepeatTool::new_sep_bounds('a', ' ', 0, 3))
/// .unwrap().1, "a a a");
/// assert_eq!(lt.match_tool_string(RepeatTool::new_sep_bounds('a', ' ', 2, 3))
/// .unwrap().1, "a a a");
/// assert_eq!(lt.match_tool_string(RepeatTool::new_sep_unbounded('a', ' ', 0))
/// .unwrap().1, "a a a a");
/// assert_eq!(lt.match_tool_string(RepeatTool::new_sep_unbounded('a', ' ', 2))
/// .unwrap().1, "a a a a");
/// assert!(lt.match_tool_string(RepeatTool::new_sep_unbounded('a', ' ', 5))
/// .is_err());
/// assert!(lt.match_tool_string(RepeatTool::new_sep_bounds('a', ' ', 2, 1))
/// .is_err());
/// ```
#[derive(Debug, Clone, Copy)]
pub struct RepeatTool<T, SEP>{
    tool : T,
    sep : SEP,
    min : usize,
    max : Option<usize>,
}

impl<T, SEP> RepeatTool<T, SEP>{
    /// Create a new [`RepeatTool`] with specified separator.
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
    /// Create a new [`RepeatTool`] that matches at least one occurrence
    pub const fn new_any_one(tool : T, sep : SEP) -> Self {
        Self::new_sep(tool, sep, 1, None)
    }
}
impl<T> RepeatTool<T, TrueTool>{
    /// Create a new [`RepeatTool`] without spaces.
    pub const fn new(tool : T, min : usize, max : Option<usize>) -> Self {
        Self{
            tool,
            sep : TrueTool,
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
}

impl<T, SEP> ParseTool for RepeatTool<T, SEP> where T : ParseTool, SEP : ParseTool {
    type Error = RepeatToolErr;

    fn parse(&self, mut st : &str) -> ToolResult<Self::Error>{
        if self.min == 0 {
            RepeatAnyTool{
                tool : &self.tool,
                sep : &self.sep,
                max : self.max
            }.parse(st).into_any()
        }
        else{
            if let Some(max) = self.max && max < self.min {
                return NoMatch(RepeatToolErr::MinMax{min : self.min, max});
            }
            let mut rlen = 0;
            match self.tool.parse(st) {
                Match{len} => {
                    rlen += len;
                    st = &st[len..];
                }
                NoMatch(_) => return NoMatch(
                    RepeatToolErr::Min{min : self.min, fnd : 0}
                ),
            }
            let seq = SeqTool( (&self.sep, &self.tool) );
            for i in 1..(self.min) {
                match seq.parse(st) {
                    Match{len} => {
                        rlen += len;
                        st = &st[len..];
                    }
                    NoMatch(()) => return NoMatch(
                        RepeatToolErr::Min{min : self.min, fnd : i}
                    ),
                }
            }
            let new_max = self.max.map(|i| i - self.min);
            let inc = RepeatAnyTool{
                tool : &seq,
                sep : TrueTool,
                max : new_max
            }.parse_always(st);
            Match{len : rlen + inc}
        }
    }
}



/// Tool that matches repetitions lazily.
///
/// It matches the least number of `T` tool (sepatared by `SEP`) which are followed by `TERM` tool.
/// The difference with respect to a [`RepeatTool`] followed by `TERM` is that here repetitions are
/// evaluated lazily: it interrupts at the first match of `TERM`, whereas `RepeatTool` evaluates
/// repetitions eagerly and so `TERM` is matched only after the repetition ends.
///
/// ```rust
/// use minparser::prelude_new::*;
///
/// let st = ViewFile::new_default("a a\ta a 0 0 1 2");
/// assert_eq!(st.match_tool_string(
/// LazyRepeatAnyTool::new_unbounded(
///     PredicateRefTool::new(char::is_ascii),
///     PredicateTool::new(char::is_whitespace), 
///     SeqTool(
///     (PredicateTool::new(char::is_whitespace), PredicateRefTool::new(char::is_ascii_digit))
///     ))).unwrap().1, 
/// "a a\ta a 0");
/// assert_eq!(st.match_tool_data(  // Without termination matching string
/// LazyRepeatAnyTool::new_unbounded(
///     PredicateRefTool::new(char::is_ascii),
///     PredicateTool::new(char::is_whitespace), 
///     SeqTool(
///     (PredicateTool::new(char::is_whitespace), PredicateRefTool::new(char::is_ascii_digit))
///     )), LazyNoTerm).unwrap().1, 
/// "a a\ta a");
/// assert!(st.match_tool_string(
/// LazyRepeatAnyTool::new_bounds(
///     PredicateRefTool::new(char::is_ascii),
///     PredicateTool::new(char::is_whitespace), 
///     SeqTool(
///     (PredicateTool::new(char::is_whitespace), '2')
///     ), 6)).is_err());
/// ```
#[derive(Copy, Clone, Debug)]
pub struct LazyRepeatAnyTool<T, SEP, TERM>{
    tool : T,
    sep : SEP,
    term : TERM,
    max : Option<usize>,
}

/// Unit struct for [`LazyRepeatTool`] in order to return a matching string without `TERM`.
#[derive(Copy, Clone, Debug, Default)]
pub struct LazyNoTerm;

impl<T, SEP, TERM> LazyRepeatAnyTool<T, SEP, TERM>{
    /// Create a new `LazyRepeatTool`.
    ///
    /// # Panics
    /// Panic if `max` is strictly lesser than `min`.
    pub const fn new(tool : T, sep : SEP, term : TERM, max : Option<usize>) -> Self {
        Self{
            tool,
            sep,
            term,
            max,
        }
    }
    /// Create a new `LazyRepeatTool` with specified upper bound.
    pub const fn new_bounds(tool : T, sep : SEP, term : TERM, max : usize) -> Self {
        Self::new(tool, sep, term, Some(max))
    }
    /// Create a new `LazyRepeatTool` without upper bound.
    pub const fn new_unbounded(tool : T, sep : SEP, term : TERM) -> Self {
        Self::new(tool, sep, term, None)
    }
}

impl<T, SEP, TERM> ParseTool for LazyRepeatAnyTool<T, SEP, TERM> where T : ParseTool, SEP : ParseTool, TERM : ParseTool {
    type Error = TERM::Error;

    fn parse(&self, st : &str) -> ToolResult<Self::Error> {
        ParseToolData::parse(self, st, LazyNoTerm).drop_data()
    }
}

impl<'a, T, SEP, TERM> ParseToolData<'a, LazyNoTerm> for LazyRepeatAnyTool<T, SEP, TERM> where T : ParseTool, SEP : ParseTool, TERM : ParseTool {
    type Data = &'a str;
    type Error = TERM::Error;

    fn parse(&self, mut st : &'a str, _ : LazyNoTerm) -> ToolResultData<Self::Data, Self::Error>{
        let init_st = st;
        if self.max == Some(0) {
            return self.term.parse(st).map_data(|i| (i, ""));
        }
        let mut rlen = 0;
        match self.term.parse(st) {
            Match{len} => return ToolResultData::Match{len, data : ""},
            NoMatch(te) => {
                match self.tool.parse(st) {
                    Match{len} => {
                        rlen += len;
                        st = &st[len..];
                    }
                    NoMatch(_) => return ToolResultData::NoMatch(te),
                }
            }
        }
        let seq = SeqTool((&self.sep, &self.tool));
        if let Some(max) = self.max {
            for _ in 1..max {
                match self.term.parse(st) {
                    Match{len} => return ToolResultData::Match{len : rlen + len, data : &init_st[0..rlen]},
                    NoMatch(te) => {
                        match seq.parse(st)  {
                            Match{len} => {
                                rlen += len;
                                st = &st[len..];
                            }
                            NoMatch(()) => return ToolResultData::NoMatch(te),
                        }
                    }
                }
            }
            return self.term.parse(st).map_data(|len| (len + rlen, &init_st[0..rlen]));
        }
        loop {
            match self.term.parse(st) {
                Match{len} => return ToolResultData::Match{len : rlen + len, data : &init_st[0..rlen]},
                NoMatch(te) => {
                    match seq.parse(st)  {
                        Match{len} => {
                            rlen += len;
                            st = &st[len..];
                        }
                        NoMatch(()) => return ToolResultData::NoMatch(te),
                    }
                }
            }
        }
    }
}

/// Tool that matches characters which satisfies the provided predicate.
///
/// See also [`PredicateRefTool`].
///
/// ```rust
/// use minparser::prelude_new::*;
///
/// let lt = ViewFile::new_default("aB1৬");
/// lt.match_tool(PredicateRefTool::new(char::is_ascii_lowercase)).unwrap()
/// .match_tool(PredicateRefTool::new(char::is_ascii_uppercase)).unwrap()
/// .match_tool(PredicateRefTool::new(char::is_ascii_digit)).unwrap()
/// .match_tool(PredicateTool::new(char::is_numeric)).unwrap();
/// ```
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
    /// Creates a tool that matches zero or more occurrences of characters that satisfy the
    /// specified predicate.
    pub const fn new_zero_or_more(predicate : P) -> RepeatTool<Self, TrueTool> {
        RepeatTool::new_unbounded(Self::new(predicate), 0)
    }
    /// Creates a tool that matches one or more occurrences of characters that satisfy the
    /// specified predicate.
    pub const fn new_one_or_more(predicate : P) -> RepeatTool<Self, TrueTool> {
        RepeatTool::new_bounds(Self::new(predicate), 0, 1)
    }
}

impl<P : Fn(char) -> bool> ParseTool for PredicateTool<P>{
    type Error = Option<char>;

    fn parse(&self, st : &str) -> ToolResult<Self::Error>{
        st.chars().next().map_or(NoMatch(None), |c| {
            if (self.predicate)(c) {
                Match{len : c.len_utf8()}
            }
            else{
                NoMatch(Some(c))
            }
        })
    }
}

/// Tool that matches characters which satisfies the provided ref predicate.
///
/// See also [`PredicateTool`].
///
/// ```rust
/// use minparser::prelude_new::*;
///
/// let lt = ViewFile::new_default("aB1৬");
/// lt.match_tool(PredicateRefTool::new(char::is_ascii_lowercase)).unwrap()
/// .match_tool(PredicateRefTool::new(char::is_ascii_uppercase)).unwrap()
/// .match_tool(PredicateRefTool::new(char::is_ascii_digit)).unwrap()
/// .match_tool(PredicateTool::new(char::is_numeric)).unwrap();
/// ```
#[derive(Debug, Clone, Copy)]
pub struct PredicateRefTool<P>{
    predicate : P,
}

impl<P : Fn(&char) -> bool> PredicateRefTool<P> {
    /// Create a new [`PredicateRefTool`] from a predicate.
    pub const fn new(predicate : P) -> Self {
        Self{
            predicate,
        }
    }
    /// Creates a tool that matches zero or more occurrences of characters that satisfy the
    /// specified predicate.
    pub const fn new_zero_or_more(predicate : P) -> RepeatTool<Self, TrueTool> {
        RepeatTool::new_unbounded(Self::new(predicate), 0)
    }
    /// Creates a tool that matches one or more occurrences of characters that satisfy the
    /// specified predicate.
    pub const fn new_one_or_more(predicate : P) -> RepeatTool<Self, TrueTool> {
        RepeatTool::new_bounds(Self::new(predicate), 0, 1)
    }
}

impl<P : Fn(&char) -> bool> ParseTool for PredicateRefTool<P>{
    type Error = Option<char>;

    fn parse(&self, st : &str) -> ToolResult<Self::Error>{
        st.chars().next().map_or(NoMatch(None), |c| {
            if (self.predicate)(&c) {
                Match{len : c.len_utf8()}
            }
            else{
                NoMatch(Some(c))
            }
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude_new::*;
    #[test]
    fn tools() {
        let vw = ViewFile::new_default("€à/a req sey");
        vw.clone().match_tool(TrueTool).unwrap();
        assert!(vw.clone().match_tool(EOFTool).is_err());
        assert_eq!(vw.clone().match_tool_string(&["Zx", "€à/a re"]).unwrap().1, "€à/a re");
        assert_eq!(vw.clone().match_tool_string(Or((EOFTool, "€à", "€"))).unwrap().1, "€à");
        assert!(vw.clone().match_tool(Or(('r', "€àb"))).is_err());
        assert_eq!(vw.clone().match_tool_string(SeqTool(("€à", "/a re", "q se", 'y', EOFTool))).unwrap().1, "€à/a req sey");
    }
}
