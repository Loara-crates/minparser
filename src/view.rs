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
use crate::pos::{Position, NoFile, Posable};
use crate::tools::ParseTool;

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

/// [`View`] alias if you don't want to specify a file.
pub type ViewFile<'a> = View<'a, NoFile>;

impl<F> AsRef<str> for View<'_, F>{
    fn as_ref(&self) -> &str {
        self.view
    }
}

/// Termination condition for tool repetition
#[derive(Debug, Clone, Copy)]
pub enum RepeatTerm<I, S>{
    /// Termination due to main tool missing match.
    Item(I),
    /// Termination due to separator tool missing match.
    Separator(S),
}

impl<F : core::fmt::Display> core::fmt::Display for RepeatTerm<NoMatch<F>, NoMatch<F>>{
    fn fmt(&self, fd : &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Item(i) => i.fmt(fd),
            Self::Separator(s) => write!(fd, "Separator error: {s}"),
        }
    }
}

impl<F : core::fmt::Debug + core::fmt::Display> core::error::Error for RepeatTerm<NoMatch<F>, NoMatch<F>> {}

/// The standard error type for a missing match at specified position.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct NoMatch<F = NoFile>{
    /// Error position.
    pub pos : Position<F>,
}

impl<F> From<core::convert::Infallible> for NoMatch<F> {
    fn from(i : core::convert::Infallible) -> Self {
        match i {}
    }
}
impl<F> From<RepeatTerm<Self, Self>> for NoMatch<F> {
    fn from(i : RepeatTerm<Self, Self>) -> Self {
        match i {
            RepeatTerm::Item(j) | RepeatTerm::Separator(j) => j,
        }
    }
}

impl<F : core::fmt::Display> core::fmt::Display for NoMatch<F>{
    fn fmt(&self, fd : &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fd, "{}: No match", self.pos)
    }
}
impl<F : core::fmt::Debug + core::fmt::Display> core::error::Error for NoMatch<F> {}

impl<F> Posable<F> for NoMatch<F> {
    fn get_pos(&self) -> &Position<F>{
        &self.pos
    }
}

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
    /// Progress the view and its position.
    ///
    /// # Panics
    /// Panics if `inc` doesn;t lie on UTF-8 code point boundaries.
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
    /// Apply the transformation `f` to the view
    #[allow(clippy::missing_errors_doc)]
    pub fn match_map<E, FF : FnOnce(Self) -> Result<Self, E>>(self, f : FF) -> Result<Self, E> {
        f(self)
    }
    /// Returns a [`NoMatch`] at current position.
    pub fn no_match(self) -> NoMatch<F> {
        NoMatch{pos : self.pos}
    }
    /// Matches a tool with the view
    #[allow(clippy::missing_errors_doc)]
    pub fn match_tool<R : ParseTool<'a, F>>(self, t : R) -> Result<Self, NoMatch<F>> {
        t.parse(self)
    }
    /// Matches any of the provided tools
    ///
    /// Tools are evaluated left-to-right, and if one matches then the followings are not evaluated
    #[allow(clippy::missing_errors_doc)]
    pub fn match_any_tool<R : ParseTool<'a, F>>(self, t : &[R]) -> Result<(Self, usize), NoMatch<F>> where Self : Clone{
        for (i, tool) in t.iter().enumerate() {
            if let Ok(r) = tool.parse(self.clone()) {
                return Ok((r, i));
            }
        }
        Err(NoMatch{pos : self.pos})
    }
    /// Checks if a tool matches the view without discarding the matching prefix.
    #[allow(clippy::missing_errors_doc)]
    pub fn check_tool<R : ParseTool<'a, F>>(self, t : R) -> Result<Self, NoMatch<F>> where Self : Clone{
        t.parse(self.clone()).map(|_vw| self)
    }
    /// Checks if a tool *does not* match the viwe without discarding the matching prefix.
    ///
    /// # Errors
    /// If the tool matches the prefix the parsed data is returned as an error.
    pub fn check_inv_tool<R : ParseTool<'a, F>>(self, t : R) -> Result<Self, NoMatch<F>> where Self : Clone{
        match t.parse(self.clone()) {
            Err(_) => Ok(self),
            Ok(_) => Err(NoMatch{pos : self.pos}),
        }
    }
    pub(crate) fn store_diff(self, base : &'a str) -> (Self, &'a str) {
        let uw = self.view;
        debug_assert_eq!(<str as AsRef<[u8]>>::as_ref(base).as_ptr_range().end, <str as AsRef<[u8]>>::as_ref(uw).as_ptr_range().end);
        (self, &base[0..base.len()-uw.len()])
    }
    /// Applies the matching tool and returns the prefix matching such tool
    ///
    /// The [`View`] object returned by [`parse`](ParseTool::parse) method of `t`
    /// should refer to a suffix of this `View`, which should always be the case whenever `t`
    /// doesn't introduce foreign views. Otherwise a panic would likely happens.
    #[allow(clippy::missing_errors_doc)]
    pub fn match_tool_string<R : ParseTool<'a, F>>(self, t : R) -> Result<(Self, &'a str), NoMatch<F>> {
        let vw = self.view;
        let u = self.match_tool(t)?;
        Ok(u.store_diff(vw))
    }
    /// Tests if a prefix of the view matches the specified string.
    #[allow(clippy::missing_errors_doc)]
    pub fn match_str(self, p : &str) -> Result<Self, NoMatch<F>> {
        self.match_tool(p)
    }
    /// Tests if the first characters coincides with the specified character
    #[allow(clippy::missing_errors_doc)]
    pub fn match_char(self, p : char) -> Result<Self, NoMatch<F>> {
        self.match_tool(p)
    }
    /// Pop the first character
    pub fn pop_char(self) -> (Self, Option<char>) {
        match self.view.chars().next() {
            Some(chr) => {
                (self.progress(chr.len_utf8()).0, Some(chr))
            }
            _ => (self, None),
        }
    }
    /// Repeat tool matching indefinitely with provided separator tool
    ///
    /// If you don't want to provide a separator then use
    /// [`TrueParser`](crate::tools::TrueParser).
    ///
    /// *Warning*: either one of main tool and separator must *not* match the empty string,
    /// otherwise this function will run undefinitely.
    pub fn repeat_match<T : ParseTool<'a, F>, SEP : ParseTool<'a, F>>(self, t : T, sep : SEP) -> RepeatRet<Self, F> where Self : Clone {
        match self.clone().match_tool(&t) {
            Err(e) => (self, 0, RepeatTerm::Item(e)),
            Ok(mut nst) => {
                let mut i = 1;
                loop {
                    match nst.clone().match_tool(&sep) {
                        Err(se) => return (nst, i, RepeatTerm::Separator(se)),
                        Ok(snst) => match snst.match_tool(&t) {
                            Err(e) => return (nst, i, RepeatTerm::Item(e)),
                            Ok(newnst) => nst = newnst,
                        }
                    }
                    i += 1;
                }
            }
        }
    }
    /// Repeat tool matching for a finite number with provided separator tool
    ///
    /// If you don't want to provide a separator then use
    /// [`TrueParser`](crate::tools::TrueParser).
    ///
    /// *Warning*: either one of main tool and separator must *not* match the empty string,
    /// otherwise this function will run undefinitely.
    pub fn repeat_match_up<T : ParseTool<'a, F>, SEP : ParseTool<'a, F>>(self, t : T, sep : SEP, bound : usize) -> RepeatRetOpt<Self, F> where Self : Clone {
        if bound == 0 {
            (self, 0, None)
        }
        else{
            match self.clone().match_tool(&t) {
                Err(e) => (self, 0, Some(RepeatTerm::Item(e))),
                Ok(mut nst) => {
                    for i in 1..bound {
                        match nst.clone().match_tool(&sep) {
                            Err(se) => return (nst, i, Some(RepeatTerm::Separator(se))),
                            Ok(snst) => match snst.match_tool(&t) {
                                Err(e) => return (nst, i, Some(RepeatTerm::Item(e))),
                                Ok(newnst) => nst = newnst,
                            }
                        }
                    }
                    (nst, bound, None)
                }
            }
        }
    }
    /// Parses an object and continue the parsing
    #[allow(clippy::missing_errors_doc)]
    pub fn parse_continue<T : crate::parsable::Parsable<'a, F>>(self) -> Result<(T, Self), T::Error> {
        T::parse(self)
    }
    /// Parses an object
    #[allow(clippy::missing_errors_doc)]
    pub fn parse<T : crate::parsable::Parsable<'a, F>>(self) -> Result<T, T::Error> {
        self.parse_continue().map(|i| i.0)
    }
    /// Parse many objects
    #[cfg(any(doc, feature = "alloc"))]
    #[cfg_attr(feature = "nightly-features", doc(cfg(feature = "alloc")))]
    pub fn parse_repeat<T : crate::parsable::Parsable<'a, F>, SEP : ParseTool<'a, F>>(self, sep : SEP) -> (alloc::vec::Vec<T>, Self) where Self : Clone {
        let mut ret = alloc::vec::Vec::new();
        match self.clone().parse_continue::<T>() {
            Err(_) => (ret, self),
            Ok((v, mut nst)) => {
                ret.push(v);
                loop {
                    match nst.clone().match_tool(&sep) {
                        Err(_) => return (ret, nst),
                        Ok(snst) => match snst.parse_continue::<T>() {
                            Err(_) => return (ret, nst),
                            Ok((nv, newnst)) => {
                                ret.push(nv);
                                nst = newnst;
                            }
                        }
                    }
                }
            }
        }
    }
}

#[allow(missing_docs)]
pub type RepeatRet<S, F> = (S, usize, RepeatTerm<NoMatch<F>, NoMatch<F>>);
#[allow(missing_docs)]
pub type RepeatRetOpt<S, F> = (S, usize, Option<RepeatTerm<NoMatch<F>, NoMatch<F>>>);




#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matching() {
        let st = "Abba req sey";
        let vw = ViewFile::new_default(st);
        assert_eq!(vw.clone().match_tool_string("Abba").unwrap().1, "Abba");
        assert_eq!(vw.clone().match_any_tool(&["Zx", "Abba r"]).unwrap().1, 1);
        assert_eq!(vw.clone().match_any_tool(&['A', 'c']).unwrap().1, 0);
        assert!(vw.match_tool('a').is_err());
    }
}
