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
use crate::pos::{Position, NoFile, Posable};

/// A view on a `str` to be parsed.
///
/// This view carries a data of type `D` and a [`Position<F>`] with respect of the initial string.
/// Each time a pattern is matched against a `View` object a new `View` object is returned,
/// returning the *remaining* part of the view which follows the matched prefix and the data parsed
/// from the matching prefix.
#[derive(Debug, Clone, Copy)]
pub struct View<'a, D, F = crate::pos::NoFile>{
    view : &'a str,
    data : D,
    pos : Position<F>,
}

impl<D, F> AsRef<str> for View<'_, D, F>{
    fn as_ref(&self) -> &str {
        self.view
    }
}


/// The standard error type for a missing match at specified position.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct NoMatch<F = NoFile>{
    /// Error position.
    pub pos : Position<F>,
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

impl<'a, F : Default> View<'a, (), F> {
    /// Creates a new [`View`] object.
    #[must_use]
    pub fn new_default(view : &'a str) -> Self {
        Self::new(view, (), F::default())
    }
}

impl<'a, D, F> View<'a, D, F> {
    /// Creates a new [`View`] object with provided *data* and *file*.
    pub const fn new(view : &'a str, data : D, file : F) -> Self {
        Self{
            view,
            data,
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
    /// Pushes data in the provided vector.
    #[cfg(any(doc, feature = "alloc"))]
    pub fn push(self, d : &mut alloc::vec::Vec<D>) -> View<'a, (), F> {
        d.push(self.data);
        View{
            view : self.view,
            data : (),
            pos : self.pos,
        }
    }
    /// Get a view to inner data.
    pub const fn view_data(&self) -> &D {
        &self.data
    }
    /// Stores data in `d`.
    pub fn save(self, d : &mut D) -> View<'a, (), F> {
        *d = self.data;
        View{
            view : self.view,
            data : (),
            pos : self.pos,
        }
    }
    /// Sets the inner data.
    pub fn set_data<W>(self, data : W) -> View<'a, W, F> {
        View{
            view : self.view,
            data,
            pos : self.pos,
        }
    }
    /// Maps the inner data through `f`.
    pub fn map_data<T, FUN : FnOnce(D) -> T>(self, f : FUN) -> View<'a, T, F> {
        View{
            view : self.view,
            data : f(self.data),
            pos : self.pos,
        }
    }
    /// Drops data.
    pub fn drop(self) -> View<'a, (), F> {
        View{
            view : self.view,
            data : (),
            pos : self.pos,
        }
    }
    /// Returns the data by consuming the view.
    pub fn consume(self) -> D {
        self.data
    }
}

impl<'a, F> View<'a, (), F> {
    /// Matches a tool with the view
    #[allow(clippy::missing_errors_doc)]
    pub fn match_tool<R : crate::parser::ParseTool<'a, F>>(self, t : R) -> Result<View<'a, R::Data, F>, R::Error> {
        t.parse(self)
    }
    /// Checks if a tool matches the view without discarding the matching prefix.
    #[allow(clippy::missing_errors_doc)]
    pub fn check_tool<R : crate::parser::ParseTool<'a, F>>(self, t : R) -> Result<View<'a, R::Data, F>, R::Error> where Self : Clone{
        t.parse(self.clone()).map(|vw| View {
            view : self.view,
            data : vw.data,
            pos : self.pos,
        })
    }
    /// Checks if a tool *does not* match the viwe without discarding the matching prefix.
    ///
    /// # Errors
    /// If the tool matches the prefix the parsed data is returned as an error.
    pub fn check_inv_tool<R : crate::parser::ParseTool<'a, F>>(self, t : R) -> Result<View<'a, R::Error, F>, R::Data> where Self : Clone{
        match t.parse(self.clone()) {
            Err(e) => Ok(View{
                view : self.view,
                data : e,
                pos : self.pos,
            }),
            Ok(d) => Err(d.consume()),
        }
    }
    /// Tests if a prefix of the view matches the specified string.
    #[allow(clippy::missing_errors_doc)]
    pub fn match_str<'b>(self, p : &'b str) -> Result<View<'a, &'a str, F>, NoMatch<F>> {
        match self.view.strip_prefix(p) {
            Some(sfx) => {
                let pfx = unsafe {self.view.get_unchecked(0..p.len())};
                let npos = progress(self.pos, pfx);
                Ok(View{
                    view : sfx,
                    data : pfx,
                    pos : npos,
                })
            }
            _ => Err(NoMatch{pos : self.pos}),
        }
    }
    /// Tests if a prefix of the view matches any of the specified prefixes.
    ///
    /// If a match happens, then the matched string is returned as data.
    #[allow(clippy::missing_errors_doc)]
    pub fn match_strs<'b>(self, p : &'b [&'b str]) -> Result<View<'a, &'a str, F>, NoMatch<F>> {
        for s in p {
            if let Some(sfx) = self.view.strip_prefix(s) {
                let pfx = unsafe {self.view.get_unchecked(0..s.len())};
                let npos = progress(self.pos, pfx);
                return Ok(View{
                    view : sfx,
                    data : pfx,
                    pos : npos,
                });
            }
        }
        Err(NoMatch{pos : self.pos})
    }
    /// Tests if the first characters coincides with the specified character
    #[allow(clippy::missing_errors_doc)]
    pub fn match_char(self, p : char) -> Result<View<'a, char, F>, NoMatch<F>> {
        match self.view.chars().next() {
            Some(chr) => {
                if p == chr {
                    let pfx = unsafe {self.view.get_unchecked(0..p.len_utf8())};
                    let sfx = unsafe {self.view.get_unchecked(p.len_utf8()..)};
                    let npos = progress(self.pos, pfx);
                    Ok(View{
                        view : sfx,
                        data : p,
                        pos : npos,
                    })
                }
                else {
                    Err(NoMatch{pos : self.pos})
                }
            }
            _ => Err(NoMatch{pos : self.pos}),
        }
    }
    /// Pop the first character
    pub fn pop_char(self) -> Option<View<'a, char, F>> {
        match self.view.chars().next() {
            Some(chr) => {
                let pfx = unsafe {self.view.get_unchecked(0..chr.len_utf8())};
                let sfx = unsafe {self.view.get_unchecked(chr.len_utf8()..)};
                let npos = progress(self.pos, pfx);
                Some(View{
                    view : sfx,
                    data : chr,
                    pos : npos,
                })
            }
            _ => None,
        }
    }
    /// Tests if the first character of the view is equal to any of the provided characters.
    #[allow(clippy::missing_errors_doc)]
    pub fn match_chars<'b>(self, p : &'b [char]) -> Result<View<'a, char, F>, NoMatch<F>> {
        match self.view.chars().next() {
            Some(chr) => {
                if p.contains(&chr) {
                    let pfx = unsafe {self.view.get_unchecked(0..chr.len_utf8())};
                    let sfx = unsafe {self.view.get_unchecked(chr.len_utf8()..)};
                    let npos = progress(self.pos, pfx);
                    Ok(View{
                        view : sfx,
                        data : chr,
                        pos : npos,
                    })
                }
                else {
                    Err(NoMatch{pos : self.pos})
                }
            }
            _ => Err(NoMatch{pos : self.pos}),
        }
    }
}

fn progress<F>(pos : Position<F>, fitstr : &str) -> Position<F> {
    let mut fit = fitstr.split('\n');
    let mut elem = fit.next().unwrap();// at least one element
    let mut nls = 0;
    for el in fit {
        elem = el;
        nls += 1;
    }
    let (mut r, mut c, file) = pos.unpack();
    if nls > 0 {
        c = 0;
        r += nls;
    }
    c += u32::try_from(elem.len()).unwrap();
    Position::new_file(file, r, c)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn matching() {
        let st = "Abba req sey";
        let vw = View::<'_, (), crate::pos::NoFile>::new_default(st);
        assert_eq!(vw.clone().match_str("Abba").unwrap().consume(), "Abba");
        assert_eq!(vw.clone().match_strs(&["Zx", "Abba r"]).unwrap().consume(), "Abba r");
        assert_eq!(vw.clone().match_chars(&['A', 'c']).unwrap().consume(), 'A');
        assert!(vw.match_char('a').is_err());
    }
}
