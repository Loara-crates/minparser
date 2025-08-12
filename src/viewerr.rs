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
use crate::toolerr::{ParseTool, ParseToolData, ToolResult, ToolResultData};

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
pub struct PosNoMatch<E, F = NoFile>{
    /// Error position.
    pub pos : Position<F>,
    /// Error data
    pub err : E,
}

/// [`View`] alias if you don't want to specify a file.
pub type ViewFile<'a> = View<'a, NoFile>;

impl<F> AsRef<str> for View<'_, F>{
    fn as_ref(&self) -> &str {
        self.view
    }
}

impl<E, F> From<core::convert::Infallible> for PosNoMatch<E, F> {
    fn from(i : core::convert::Infallible) -> Self {
        match i {}
    }
}
impl<F> From<Position<F>> for PosNoMatch<(), F> {
    fn from(pos : Position<F>) -> Self {
        Self{pos, err : ()}
    }
}

impl<E : core::fmt::Display, F : core::fmt::Display> core::fmt::Display for PosNoMatch<E, F>{
    fn fmt(&self, fd : &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fd, "{}: {}", self.pos, self.err)
    }
}
impl<E : core::error::Error, F : core::fmt::Debug + core::fmt::Display> core::error::Error for PosNoMatch<E, F> {}

impl<E, F> Posable<F> for PosNoMatch<E, F> {
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
    /// Consumes the view and returns the actual [`Position`].
    pub fn into_pos(self) -> Position<F> {
        self.pos
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
    /// Matches a tool with the view.
    #[allow(clippy::missing_errors_doc)]
    pub fn match_tool<R : ParseTool>(self, t : R) -> Result<Self, PosNoMatch<R::Error, F>> {
        match t.parse(self.view) {
            ToolResult::Match{len} => Ok(self.progress(len).0),
            ToolResult::NoMatch(err) => Err(PosNoMatch{err, pos : self.pos}),
        }
    }
    /// Matches a tool with the entire view, not only with a prefix.
    #[allow(clippy::missing_errors_doc)]
    pub fn match_tool_final<R : ParseTool>(self, t : R) -> Result<(), PosNoMatch<Option<R::Error>, F>> {
        use crate::toolerr::{ErrorTool, EOFTool};
        self.match_tool(ErrorTool::new(t, |e, _| Some(e)))?
            .match_tool(ErrorTool::new(EOFTool, |(), _| None)).map(|_| ())
    }
    /// Matches a [`ParseToolData`].
    ///
    /// Parameter `P` can be used to provide additional parameter to the `ParseToolData` object.
    #[allow(clippy::missing_errors_doc, clippy::needless_pass_by_value)]
    pub fn match_tool_data<P, R : ParseToolData<'a, P>>(self, t : R, p : P) -> Result<(Self, R::Data), PosNoMatch<R::Error, F>> {
        match t.parse(self.view, p) {
            ToolResultData::Match{len, data} => Ok((self.progress(len).0, data)),
            ToolResultData::NoMatch(err) => Err(PosNoMatch{err, pos : self.pos}),
        }
    }
    /// Applies the matching tool and returns the prefix matching such tool
    ///
    /// The [`View`] object returned by [`parse`](ParseTool::parse) method of `t`
    /// should refer to a suffix of this `View`, which should always be the case whenever `t`
    /// doesn't introduce foreign views. Otherwise a panic would likely happens.
    #[allow(clippy::missing_errors_doc)]
    pub fn match_tool_string<R : ParseTool>(self, t : R) -> Result<(Self, &'a str), PosNoMatch<R::Error, F>> {
        match t.parse(self.view) {
            ToolResult::Match{len} => Ok(self.progress(len)),
            ToolResult::NoMatch(err) => Err(PosNoMatch{err, pos : self.pos}),
        }
    }
}

/*
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
*/
