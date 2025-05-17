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

/// Return type of a [`ParseTool`].
pub enum ToolResult{
    /// A successful match.
    Match{
        /// The length in bytes of the matching prefix.
        len : usize,
    },
    /// A failed match.
    NoMatch,
}
/// Return type of a [`ParseToolData`].
pub enum ToolResultData<D>{
    /// A successful match.
    Match{
        /// The length in bytes of the matching prefix.
        len : usize,
        /// The carried data.
        data : D,
    },
    /// A failed match.
    NoMatch,
}

impl ToolResult{
    /// Apply `f` to a successful match.
    #[must_use]
    pub fn map<F : FnOnce(usize) -> usize>(self, f : F) -> Self {
        match self {
            Self::Match{len} => Self::Match{ len : f(len)},
            Self::NoMatch => Self::NoMatch,
        }
    }
    /// Apply `f` to a successful match and generate data.
    #[must_use]
    pub fn map_data<D, F : FnOnce(usize) -> (usize, D)>(self, f : F) -> ToolResultData<D> {
        match self {
            Self::Match{len: i} => {
                let (len, data) = f(i);
                ToolResultData::Match{ len, data}
            }
            Self::NoMatch => ToolResultData::NoMatch,
        }
    }
    /// Call `f` if the match is successful, otherwise return `NoMatch`.
    #[must_use]
    pub fn and_then<F : FnOnce(usize) -> Self>(self, f : F) -> Self {
        match self {
            Self::Match{len} => f(len),
            Self::NoMatch => Self::NoMatch,
        }
    }
    /// Call `f` if the match is not successful, otherwise return the `Match` variant of `self`.
    #[must_use]
    pub fn or_else<F : FnOnce() -> Self>(self, f : F) -> Self {
        match self {
            Self::Match{len} => Self::Match{len},
            Self::NoMatch => f(),
        }
    }
}

impl<D> ToolResultData<D> {
    /// Drops the data field.
    #[must_use]
    pub fn drop_data(self) -> ToolResult {
        match self {
            Self::Match{len, ..} => ToolResult::Match{len},
            Self::NoMatch => ToolResult::NoMatch,
        }
    }
}

#[cfg(any(feature = "nightly-features", doc))]
mod try_mod {
    use core::ops::{Try, FromResidual, ControlFlow, Residual};

    /// Error tyoe for [`ToolResult`](crate::tools::ToolResult).
    pub struct ToolResultErr;

    impl Try for super::ToolResult {
        type Output = usize;
        type Residual = ToolResultErr;

        fn from_output(len: Self::Output) -> Self {
            Self::Match{len}
        }
        fn branch(self) -> ControlFlow<Self::Residual, Self::Output>{
            match self {
                Self::Match{len} => ControlFlow::Continue(len),
                Self::NoMatch => ControlFlow::Break(ToolResultErr),
            }
        }
    }
    impl FromResidual<ToolResultErr> for super::ToolResult {
        fn from_residual(_ : ToolResultErr) -> Self {
            Self::NoMatch
        }
    }
    impl Residual<usize> for ToolResultErr {
        type TryType = super::ToolResult;
    }
}

#[cfg(any(feature = "nightly-features", doc))]
pub use try_mod::*;

/// The main parsing trait
///
/// Every object which incapsulate a parsing strategy should implement this trait in order to be
/// composed with other tools.
pub trait ParseTool {
    /// The main parsing algorithm.
    ///
    /// If there is a match, a `Result::Ok` is returned with the length *in bytes* of the matched
    /// prefix. Notice that a zero may return if the empty string `""` is a valid match for this
    /// tool.
    ///
    /// # Errors
    /// If no prefix of `st` satisfies this parsing strategy then an error is issued.
    fn parse(&self, st : &str) -> ToolResult;

    /// Coerce to dyn trait
    fn as_dyn(&self) -> &dyn ParseTool where Self : Sized {
        self
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

    /// The main parsing algorithm.
    fn parse(&self, st : &'a str, par : P) -> ToolResultData<Self::Data>;
}

impl<T> ParseTool for &T where T : ParseTool + ?Sized {
    fn parse(&self, st : &str) -> ToolResult{
        (*self).parse(st)
    }
}

#[cfg(any(feature = "alloc", doc, test))]
impl<T> ParseTool for alloc::boxed::Box<T> where T : ParseTool + ?Sized {
    fn parse(&self, st : &str) -> ToolResult{
        use core::ops::Deref;
        self.deref().parse(st)
    }
}

/// Matches a string exactly
///
/// ```rust
/// use minparser::view::ViewFile;
/// let st = "My data ";
/// ViewFile::new_default(st).match_tool("My dat").unwrap().match_tool("a ").unwrap();
/// ```
impl ParseTool for str {
    fn parse(&self, st : &str) -> ToolResult{
        if st.starts_with(self) {
            ToolResult::Match{len : self.len()}
        }
        else{
            ToolResult::NoMatch
        }
    }
}

/// Matches the first character exactly.
///
/// ```rust
/// use minparser::view::ViewFile;
/// let st = "My data ";
/// ViewFile::new_default(st).match_tool('M').unwrap().match_tool('y').unwrap();
/// ```
impl ParseTool for char {
    fn parse(&self, st : &str) -> ToolResult{
        st.chars().next().map_or(ToolResult::NoMatch, |chr| {
            if *self == chr {
                ToolResult::Match{len : self.len_utf8()}
            }
            else {
                ToolResult::NoMatch
            }
        })
    }
}

impl ParseTool for fn(char) -> bool {
    fn parse(&self, st : &str) -> ToolResult{
        st.chars().next().map_or(ToolResult::NoMatch, |c|  {
            if self(c) {
                ToolResult::Match{len : c.len_utf8()}
            }
            else{
                ToolResult::NoMatch
            }
        })
    }
}
impl ParseTool for fn(&char) -> bool {
    fn parse(&self, st : &str) -> ToolResult{
        st.chars().next().map_or(ToolResult::NoMatch, |c| {
            if self(&c) {
                ToolResult::Match{len : c.len_utf8()}
            }
            else{
                ToolResult::NoMatch
            }
        })
    }
}

/// Matches any of the tools in the slice.
///
/// ```rust
/// use minparser::view::ViewFile;
/// let st = "My data ";
/// ViewFile::new_default(st).match_tool(&['M', 'K', 'O']).unwrap().match_tool(&["ser", "ii", "y d"]).unwrap();
/// ```
impl<R> ParseTool for [R] where R : ParseTool {
    fn parse(&self, st : &str) -> ToolResult{
        for i in self {
            if let ToolResult::Match{len} = i.parse(st) {
                return ToolResult::Match{len};
            }
        }
        ToolResult::NoMatch
    }
}

/// Matches any of the tools in the array.
///
/// ```rust
/// use minparser::view::ViewFile;
/// let st = "My data ";
/// ViewFile::new_default(st).match_tool(['M', 'K', 'O']).unwrap().match_tool(["ser", "ii", "y d"]).unwrap();
/// ```
impl<R, const N : usize> ParseTool for [R; N] where R : ParseTool {
    fn parse(&self, st : &str) -> ToolResult{
        self.as_slice().parse(st)
    }
}

/// Parses only the end of the input
///
/// ```rust
/// use minparser::view::ViewFile;
/// let st = "My data ";
/// ViewFile::new_default(st).match_tool("My data ").unwrap().match_tool(minparser::tools::EOFTool).unwrap();
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct EOFTool;

impl ParseTool for EOFTool{
    fn parse(&self, st : &str) -> ToolResult {
        if st.is_empty() {
            ToolResult::Match{len : 0}
        }
        else{
            ToolResult::NoMatch
        }
    }
}

/// Only checks the provided tool, without progressing.
///
/// ```rust
/// use minparser::view::ViewFile;
/// use minparser::tools::CheckTool;
///
/// ViewFile::new_default("a").match_tool(CheckTool('a')).unwrap()
///     .match_tool(CheckTool::<fn(&char) -> bool>(char::is_ascii)).unwrap();
/// ```
#[derive(Debug, Copy, Clone, Default)]
pub struct CheckTool<T>(pub T);

impl<T> ParseTool for CheckTool<T> where T : ParseTool {
    fn parse(&self, st : &str) -> ToolResult {
        self.0.parse(st).map(|_| 0)
    }
}

/// Matches only if the provided tool doesn't match.
///
/// ```rust
/// use minparser::view::ViewFile;
/// use minparser::tools::CheckInvTool;
///
/// ViewFile::new_default("ab").match_tool(CheckInvTool('c')).unwrap()
///     .match_tool(CheckInvTool('b')).unwrap();
/// ```
#[derive(Debug, Copy, Clone, Default)]
pub struct CheckInvTool<T>(pub T);

impl<T> ParseTool for CheckInvTool<T> where T : ParseTool {
    fn parse(&self, st : &str) -> ToolResult{
        match self.0.parse(st) {
            ToolResult::Match{..} => ToolResult::NoMatch,
            ToolResult::NoMatch => ToolResult::Match{len : 0},
        }
    }
}

/// Matches any character.
#[derive(Debug, Copy, Clone, Default)]
pub struct AnyTool;

impl ParseTool for AnyTool {
    fn parse(&self, st : &str) -> ToolResult{
        st.chars().next().map_or(ToolResult::NoMatch, |c|  ToolResult::Match{len : c.len_utf8()})
    }
}

/// Matches the empty string, therefore it always matches.
///
/// ```rust
/// use minparser::view::ViewFile;
/// let st = "My data ";
/// ViewFile::new_default(st).match_tool(minparser::tools::TrueParser).unwrap().match_tool("My data ").unwrap();
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct TrueParser;

impl ParseTool for TrueParser{
    fn parse(&self, _ : &str) -> ToolResult {
        ToolResult::Match{len : 0}
    }
}

/// Tool that matches at least one of many subtools.
///
/// This crate provides
/// implementation for tuples with up to 15 elements, but thanks to associativity property you can
/// use nested tuples to support more tools.
///
/// *Ordering matters here*: if one matches then the followings are not tried. 
///
/// ```rust
/// use minparser::view::ViewFile;
/// let st = "My data ";
/// ViewFile::new_default(st).match_tool(minparser::tools::Or(('Z', "My", 'M')))
/// .unwrap().match_tool(" data ").unwrap();
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct Or<Tup>(
    /// Tools to be tested.
    pub Tup
);
/// Tool that matches a sequence of subtools.
///
/// This crate provides
/// implementation for tuples with up to 15 elements, but thanks to associativity property you can
/// use nested tuples to support more tools.
///
/// ```rust
/// use minparser::view::ViewFile;
/// let st = "My data ";
/// ViewFile::new_default(st).match_tool(minparser::tools::SeqTool(('M', "y da", 't', "a"))).unwrap();
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct SeqTool<Tup>(
    /// Tools to be tested.
    pub Tup
);

macro_rules! or_tuple {
    {} => {
        impl ParseTool for Or<()> {
            fn parse(&self, _ : &str) -> ToolResult {
                ToolResult::NoMatch
            }
        }
        impl ParseTool for SeqTool<()> {
            fn parse(&self, _ : &str) -> ToolResult {
                ToolResult::Match{len : 0}
            }
        }
    };
    {$a:ident} => {
        or_tuple!{}
        impl<$a> ParseTool for Or<($a,)> where $a : ParseTool {
            fn parse(&self, st : &str) -> ToolResult {
                self.0.0.parse(st)
            }
        }
        impl<$a> ParseTool for SeqTool<($a,)> where $a : ParseTool {
            fn parse(&self, st : &str) -> ToolResult {
                self.0.0.parse(st)
            }
        }
    };
    {$a:ident, $($r:ident),+} => {
        or_tuple!{$($r),+}
        #[allow(non_snake_case)]
        impl<$a, $($r),+> ParseTool for Or<($a, $($r),+)> where $a : ParseTool, $($r : ParseTool),+ {
            fn parse(&self, st : &str) -> ToolResult {
                let ($a, $($r),+) = &self.0;
                $a.parse(st).or_else(|| Or(($($r),+,)).parse(st))
            }
        }
        #[allow(non_snake_case)]
        impl<$a, $($r),+> ParseTool for SeqTool<($a, $($r),+)> where $a : ParseTool, $($r : ParseTool),+ {
            fn parse(&self, st : &str) -> ToolResult {
                let ($a, $($r),+) = &self.0;
                $a.parse(st).and_then(|rlen| SeqTool(($($r),+,)).parse(&st[rlen..]).map(|len| len+rlen) )
            }
        }
    }
}

or_tuple!{A, B, C, D, E, F, G, H, I, J, K, L, M, N, O}

/// Tool that matches repetitions with separator requiring a minimum number of repetitions.
///
/// ```rust
/// let lt = minparser::view::ViewFile::new_default("a a a a b");
/// assert_eq!(lt.match_tool_string(minparser::tools::RepeatTool::new_sep_bounds('a', ' ', 0, 3))
/// .unwrap().1, "a a a");
/// assert_eq!(lt.match_tool_string(minparser::tools::RepeatTool::new_sep_bounds('a', ' ', 2, 3))
/// .unwrap().1, "a a a");
/// assert_eq!(lt.match_tool_string(minparser::tools::RepeatTool::new_sep_unbounded('a', ' ', 0))
/// .unwrap().1, "a a a a");
/// assert_eq!(lt.match_tool_string(minparser::tools::RepeatTool::new_sep_unbounded('a', ' ', 2))
/// .unwrap().1, "a a a a");
/// assert!(lt.match_tool_string(minparser::tools::RepeatTool::new_sep_unbounded('a', ' ', 5))
/// .is_err());
/// assert!(lt.match_tool_string(minparser::tools::RepeatTool::new_sep_bounds('a', ' ', 2, 1))
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
impl<T> RepeatTool<T, TrueParser>{
    /// Create a new [`RepeatTool`] without spaces.
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
    fn parse(&self, mut st : &str) -> ToolResult{
        if let Some(max) = self.max {
            if max < self.min {
                return ToolResult::NoMatch;
            }
        }
        let mut len = 0;
        let mut start = self.min;
        if self.min > 0 {
            if let ToolResult::Match{len : ilen} = self.tool.parse(st) {
                st = &st[ilen..];
                len += ilen;
                for _ in 1..(self.min) {
                    if let ToolResult::Match{len : splen} = self.sep.parse(st) {
                        if let ToolResult::Match{len : tlen} = self.tool.parse(&st[splen..]) {
                            st = &st[(splen + tlen)..];
                            len += splen + tlen;
                        }
                        else{
                            return ToolResult::NoMatch;
                        }
                    }
                    else{
                        return ToolResult::NoMatch;
                    }
                }
            }
            else{
                return ToolResult::NoMatch;
            }
        }
        else if let ToolResult::Match{len : ilen} = self.tool.parse(st) {
            st = &st[ilen..];
            len += ilen;
            start += 1;
        }
        else{
            return ToolResult::Match{len : 0};
        }
        if let Some(max) = self.max {
            for _ in start..max {
                if let ToolResult::Match{len : splen} = self.sep.parse(st) {
                    if let ToolResult::Match{len : tlen} = self.tool.parse(&st[splen..]) {
                        st = &st[(splen + tlen)..];
                        len += splen + tlen;
                    }
                    else{
                        return ToolResult::Match{len};
                    }
                }
                else{
                    return ToolResult::Match{len};
                }
            }
            return ToolResult::Match{len};
        }
        loop {
            if let ToolResult::Match{len : splen} = self.sep.parse(st) {
                if let ToolResult::Match{len : tlen} = self.tool.parse(&st[splen..]) {
                    st = &st[(splen + tlen)..];
                    len += splen + tlen;
                }
                else{
                    return ToolResult::Match{len};
                }
            }
            else{
                return ToolResult::Match{len};
            }
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
/// use minparser::tools::{LazyRepeatTool, LazyNoTerm, SeqTool};
/// use minparser::predicates::*;
/// let st = minparser::view::ViewFile::new_default("a a a a 0 0 1 2");
/// assert_eq!(st.match_tool_string(
/// LazyRepeatTool::new_unbounded(
///     AsciiAlphanumericTool,
///     WhitespaceTool, 
///     SeqTool((WhitespaceTool, AsciiDigitTool)),
///     0)).unwrap().1, 
/// "a a a a 0");
/// assert_eq!(st.match_tool_string(
/// LazyRepeatTool::new_unbounded(
///     AsciiAlphanumericTool,
///     WhitespaceTool, 
///     SeqTool((WhitespaceTool, AsciiDigitTool)),
///     6)).unwrap().1, 
/// "a a a a 0 0 1");
/// assert_eq!(st.match_tool_data(
/// LazyRepeatTool::new_unbounded(
///     AsciiAlphanumericTool,
///     WhitespaceTool, 
///     SeqTool((WhitespaceTool, AsciiDigitTool)),
///     6),
///     LazyNoTerm).unwrap().1, 
/// "a a a a 0 0");
/// assert!(st.match_tool_string(
/// LazyRepeatTool::new_bounds(
///     AsciiAlphanumericTool,
///     WhitespaceTool, 
///     SeqTool((WhitespaceTool, '2')),
///     0, 6)).is_err());
/// ```
#[derive(Copy, Clone, Debug)]
pub struct LazyRepeatTool<T, SEP, TERM>{
    tool : T,
    sep : SEP,
    term : TERM,
    min : usize,
    max : Option<usize>,
}

/// Unit struct for [`LazyRepeatTool`] in order to return a matching string without `TERM`.
#[derive(Copy, Clone, Debug, Default)]
pub struct LazyNoTerm;

impl<T, SEP, TERM> LazyRepeatTool<T, SEP, TERM>{
    /// Create a new `LazyRepeatTool`.
    ///
    /// # Panics
    /// Panic if `max` is strictly lesser than `min`.
    pub const fn new(tool : T, sep : SEP, term : TERM, min : usize, max : Option<usize>) -> Self {
        if let Some(m) = max {
            assert!(m >= min, "Max is strictly lesser than min");
        }
        Self{
            tool,
            sep,
            term,
            min,
            max,
        }
    }
    /// Create a new `LazyRepeatTool` with specified upper bound.
    pub const fn new_bounds(tool : T, sep : SEP, term : TERM, min : usize, max : usize) -> Self {
        Self::new(tool, sep, term, min, Some(max))
    }
    /// Create a new `LazyRepeatTool` without upper bound.
    pub const fn new_unbounded(tool : T, sep : SEP, term : TERM, min : usize) -> Self {
        Self::new(tool, sep, term, min, None)
    }
}

impl<T, SEP, TERM> ParseTool for LazyRepeatTool<T, SEP, TERM> where T : ParseTool, SEP : ParseTool, TERM : ParseTool {
    fn parse(&self, st : &str) -> ToolResult {
        ParseToolData::parse(self, st, LazyNoTerm).drop_data()
    }
}

impl<'a, T, SEP, TERM> ParseToolData<'a, LazyNoTerm> for LazyRepeatTool<T, SEP, TERM> where T : ParseTool, SEP : ParseTool, TERM : ParseTool {
    type Data = &'a str;
    fn parse(&self, mut st : &'a str, _ : LazyNoTerm) -> ToolResultData<&'a str>{
        let ini = st;
        let joint = SeqTool((&self.sep, &self.tool));
        if let Some(max) = self.max {
            if max < self.min {
                return ToolResultData::NoMatch;
            }
        }
        let mut len = 0;
        let mut start = self.min;
        if self.min > 0 {
            if let ToolResult::Match{len : ilen} = self.tool.parse(st) {
                st = &st[ilen..];
                len += ilen;
                for _ in 1..(self.min) {
                    if let ToolResult::Match{len : inclen} = joint.parse(st) {
                        st = &st[inclen..];
                        len += inclen;
                    }
                    else{
                        return ToolResultData::NoMatch;
                    }
                }
            }
            else{
                return ToolResultData::NoMatch;
            }
        }
        else{
            if let ToolResult::Match{len : ilen} = self.term.parse(st) {
                return ToolResultData::Match{len : ilen, data : ""};
            }
            if let ToolResult::Match{len : ilen} = self.tool.parse(st) {
                st = &st[ilen..];
                len += ilen;
                start += 1;
            }
            else{
                return ToolResultData::NoMatch;
            }
        }
        if let Some(max) = self.max {
            for _ in start..max {
                if let ToolResult::Match{len : ilen} = self.term.parse(st) {
                    return ToolResultData::Match{len : len + ilen, data : &ini[0..len]};
                }
                if let ToolResult::Match{len : inclen} = joint.parse(st) {
                    st = &st[inclen..];
                    len += inclen;
                }
                else{
                    return ToolResultData::NoMatch;
                }
            }
            return self.term.parse(st).map_data(|ilen| (len + ilen, &ini[0..len]));
        }
        loop {
            if let ToolResult::Match{len : ilen} = self.term.parse(st) {
                return ToolResultData::Match{len : len + ilen, data : &ini[0..len]};
            }
            if let ToolResult::Match{len : inclen} = joint.parse(st) {
                st = &st[inclen..];
                len += inclen;
            }
            else{
                return ToolResultData::NoMatch;
            }
        }
    }
}

/// Get matched character in [`PredicateTool`] and [`PredicateRefTool`].
pub struct GetChar;
/// Transform the matched character in [`PredicateTool`] and [`PredicateRefTool`].
pub struct Transform<F>(pub F);

/// Tool that matches characters which satisfies the provided predicate.
///
/// See also [`PredicateRefTool`].
///
/// ```rust
/// use minparser::tools::PredicateRefTool;
/// use minparser::tools::PredicateTool;
/// let lt = minparser::view::ViewFile::new_default("aB1৬");
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
    pub const fn new_zero_or_more(predicate : P) -> RepeatTool<Self, TrueParser> {
        RepeatTool::new_unbounded(Self::new(predicate), 0)
    }
    /// Creates a tool that matches one or more occurrences of characters that satisfy the
    /// specified predicate.
    pub const fn new_one_or_more(predicate : P) -> RepeatTool<Self, TrueParser> {
        RepeatTool::new_bounds(Self::new(predicate), 0, 1)
    }
}

impl<P : Fn(char) -> bool> ParseTool for PredicateTool<P>{
    fn parse(&self, st : &str) -> ToolResult{
        st.chars().next().map_or(ToolResult::NoMatch, |c| {
            if (self.predicate)(c) {
                ToolResult::Match{len : c.len_utf8()}
            }
            else{
                ToolResult::NoMatch
            }
        })
    }
}
impl<'a, P : Fn(char) -> bool> ParseToolData<'a, GetChar> for PredicateTool<P>{
    type Data = char;
    fn parse(&self, st : &'a str, _ : GetChar) -> ToolResultData<Self::Data>{
        st.chars().next().map_or(ToolResultData::NoMatch, |c| {
            if (self.predicate)(c) {
                ToolResultData::Match{len : c.len_utf8(), data : c}
            }
            else{
                ToolResultData::NoMatch
            }
        })
    }
}

impl<'a, P : Fn(char) -> bool, D, F : Fn(char) -> D> ParseToolData<'a, Transform<F>> for PredicateTool<P>{
    type Data = D;
    fn parse(&self, st : &'a str, f : Transform<F>) -> ToolResultData<Self::Data>{
        st.chars().next().map_or(ToolResultData::NoMatch, |c| {
            if (self.predicate)(c) {
                ToolResultData::Match{len : c.len_utf8(), data : (f.0)(c)}
            }
            else{
                ToolResultData::NoMatch
            }
        })
    }
}

/// Tool that matches characters which satisfies the provided ref predicate.
///
/// See also [`PredicateTool`].
///
/// ```rust
/// use minparser::tools::PredicateRefTool;
/// use minparser::tools::PredicateTool;
/// let lt = minparser::view::ViewFile::new_default("aB1৬");
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
    pub const fn new_zero_or_more(predicate : P) -> RepeatTool<Self, TrueParser> {
        RepeatTool::new_unbounded(Self::new(predicate), 0)
    }
    /// Creates a tool that matches one or more occurrences of characters that satisfy the
    /// specified predicate.
    pub const fn new_one_or_more(predicate : P) -> RepeatTool<Self, TrueParser> {
        RepeatTool::new_bounds(Self::new(predicate), 0, 1)
    }
}

impl<P : Fn(&char) -> bool> ParseTool for PredicateRefTool<P>{
    fn parse(&self, st : &str) -> ToolResult{
        st.chars().next().map_or(ToolResult::NoMatch, |c| {
            if (self.predicate)(&c) {
                ToolResult::Match{len : c.len_utf8()}
            }
            else{
                ToolResult::NoMatch
            }
        })
    }
}
impl<'a, P : Fn(&char) -> bool> ParseToolData<'a, GetChar> for PredicateRefTool<P>{
    type Data = char;
    fn parse(&self, st : &'a str, _ : GetChar) -> ToolResultData<Self::Data>{
        st.chars().next().map_or(ToolResultData::NoMatch, |c| {
            if (self.predicate)(&c) {
                ToolResultData::Match{len : c.len_utf8(), data : c}
            }
            else{
                ToolResultData::NoMatch
            }
        })
    }
}
impl<'a, P : Fn(&char) -> bool, D, F : Fn(char) -> D> ParseToolData<'a, Transform<F>> for PredicateRefTool<P>{
    type Data = D;
    fn parse(&self, st : &'a str, f : Transform<F>) -> ToolResultData<Self::Data>{
        st.chars().next().map_or(ToolResultData::NoMatch, |c| {
            if (self.predicate)(&c) {
                ToolResultData::Match{len : c.len_utf8(), data : (f.0)(c)}
            }
            else{
                ToolResultData::NoMatch
            }
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn tools() {
        let vw = ViewFile::new_default("€à/a req sey");
        vw.clone().match_tool(TrueParser).unwrap();
        assert!(vw.clone().match_tool(EOFTool).is_err());
        assert_eq!(vw.clone().match_tool_string(&["Zx", "€à/a re"]).unwrap().1, "€à/a re");
        assert_eq!(vw.clone().match_tool_string(Or((EOFTool, "€à", "€"))).unwrap().1, "€à");
        assert!(vw.clone().match_tool(Or(('r', "€àb"))).is_err());
        assert_eq!(vw.clone().match_tool_string(SeqTool(("€à", "/a re", "q se", 'y', EOFTool))).unwrap().1, "€à/a req sey");
    }
}
