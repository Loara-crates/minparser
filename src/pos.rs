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
use core::convert::{AsRef, AsMut};
use core::ops::ControlFlow;
use core::fmt::{Formatter, Display, Debug};
use core::error::Error;
use core::write;

/// A placeholder that you can use when you don't want to use the file field in a
/// [`Position`].
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Default)]
pub struct NoFile;

impl Display for NoFile{
    fn fmt(&self, _f : &mut Formatter<'_>) -> core::fmt::Result {
        Ok(())
    }
}

/// A position.
///
/// Characters in a text file can be organizes in a virtual grid in order to easily find characters
/// or parsing errors inside the file. Each file can be divided in multiple _lines_ separated by
/// line feed (U+000A) or carriage return + line feed (U+000D + U+000A).
///
/// To find a character then you need just the index of the line containing it (the first line has index `0`) 
/// and the character index inside thai line (also called the _column_).
///
/// Type `F` is any type that can be used to identify a text file, for example a
/// `String`, a `Path` or a custom type. If you work on a single file and/or you don't want to take
/// in account the used file just put [`NoFile`] as `F` or don't specify `F`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Position<FILE = NoFile> {
    file : FILE,
    r : u32,
    c : u32,
}

impl<F> Position<F>{
    /// Create a new `Position` at specified `file`, `line` and `column` indices
    #[must_use]
    pub const fn new_file(file : F, line : u32, column : u32) -> Self {
        Self{
            file,
            r : line,
            c : column,
        }
    }
    /// Create a new `Position` with `line=0` and `column=0`
    #[must_use]
    pub const fn new_file_zero(file : F) -> Self {
        Self{
            file,
            r : 0,
            c : 0,
        }
    }
    /// Get the file identifier
    #[must_use]
    pub const fn file(&self) -> &F {
        &self.file
    }
    /// Get the line number
    #[must_use]
    pub const fn line(&self) -> u32 {
        self.r
    }
    /// Get the column number
    #[must_use]
    pub const fn column(&self) -> u32 {
        self.c
    }
    /// Converts it to a reference
    #[must_use]
    pub const fn make_ref(&self) -> Position<&F> {
        Position{
            file : &self.file,
            r : self.r,
            c : self.c,
        }
    }
    /// Unpacks the position
    ///
    /// The first integer is the line number, the second one the column number
    #[must_use]
    pub fn unpack(self) -> (u32, u32, F) {
        (self.r, self.c, self.file)
    }
}

impl<F> Position<F> where F : Default{
    /// Like [`Position::new_file`] but with `file` argument set to `F::default()`
    #[must_use]
    pub fn new(line : u32, column : u32) -> Self {
        Self::new_file(F::default(), line, column)
    }
    /// Like [`Position::new_file_zero`] but with `file` argument set to `F::default()`
    #[must_use]
    pub fn new_zero() -> Self {
        Self::new_file_zero(F::default())
    }
}

impl<F> Display for Position<F> where F : Display {
    fn fmt(&self, fmt : &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fmt, "File: {}, line: {}, column: {}", self.file(), self.r, self.c)
    }
}

/// A trait for objects with an associated position
///
/// You can freely implement this trait for your objects or instead use [`Pos`] wrapper.
pub trait Posable<F>{
    /// Get current position
    fn get_pos(&self) -> &Position<F>;

    /// Get current file location
    fn file(&self) -> &F {
        &self.get_pos().file
    }
    /// Get line number
    fn line(&self) -> u32 {
        self.get_pos().r
    }
    /// Get column number
    fn column(&self) -> u32 {
        self.get_pos().c
    }
}

/// A trait for object with mutable position
pub trait PosableMut<F> : Posable<F> {
    /// Get a mutable reference to its position
    fn get_pos_mut(&mut self) -> &mut Position<F>;

    /// Increase or decrease line and column number by an offset
    fn progress(&mut self, nrow : i32, ncol : i32) {
        let pos = self.get_pos_mut();
        pos.r = pos.r.saturating_add_signed(nrow);
        pos.c = pos.c.saturating_add_signed(ncol);
    }
}

/// A positioned object, functionally equivalent to a struct containing an object of type `T` (the
/// wrapped object) and its position as a `Position<FILE>` object.
///
/// A parser uses characters inside a text file to create more complex objects. When the parsing
/// fails due to a syntax error you must inform the user about it, in particular you need to tell
/// the user where is the error.
///
/// This type associates to a type `T` (usually a token) a [Position] that allows the user to locate it inside
/// the file
#[derive(Clone, Copy, Debug)]
pub struct Pos<T, FILE = NoFile> {
    el : T,
    pos : Position<FILE>,
}

impl<T, F> AsRef<T> for Pos<T, F> {
    fn as_ref(&self) -> &T {
        &self.el
    }
}
impl<T, F> AsMut<T> for Pos<T, F> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.el
    }
}

impl<T, F> Display for Pos<T, F> where T : Display, F : Display {
    fn fmt(&self, fmt : &mut Formatter<'_>) -> core::fmt::Result {
        write!(fmt, "{}: {}", self.pos, self.el)
    }
}

impl<T, F> Error for Pos<T, F> where T : Error, F : Display + Debug {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.el.source()
    }
}

impl<T, F> Posable<F> for Pos<T, F>{
    fn get_pos(&self) -> &Position<F> {
        &self.pos
    }
}

impl<T, F> PosableMut<F> for Pos<T, F>{
    fn get_pos_mut(&mut self) -> &mut Position<F> {
        &mut self.pos
    }
}

impl<T, F> Pos<T, F> {
    /// Creates a new positioned object.
    pub const fn new(el : T, pos : Position<F>) -> Self {
        Self{el, pos}
    }
    /// Creates a new positioned object.
    pub const fn new_pos(el : T, file : F, line : u32, column : u32) -> Self {
        Self{
            el,
            pos : Position{
                file,
                r : line,
                c : column,
            }
        }
    }
    ///Consumes the objects and returns the wrapped element
    pub fn take(self) -> T {
        self.el
    }
    ///Consumes the objects and returns the position
    pub fn take_pos(self) -> Position<F> {
        self.pos
    }
    ///Consumes the objects and returns both the wrapped element and the position
    pub fn take_all(self) -> (T, Position<F>) {
        (self.el, self.pos)
    }

    /// Tests the wrapped object
    pub fn test<TF : FnOnce(&T) -> bool>(&self, f : TF) -> bool {
        f(&self.el)
    }
    /// Tests both the wrapped object and its position
    pub fn test_pos<TF : FnOnce(&T, &Position<F>) -> bool>(&self, f : TF) -> bool {
        f(&self.el, &self.pos)
    }
    /// Apply `f` at the wrapped object
    pub fn map<U, MF : FnOnce(T) -> U>(self, f : MF) -> Pos<U, F> {
        Pos{
            el : f(self.el),
            pos : self.pos,
        }
    }
    /// Apply `f` at the wrapped object and its position
    pub fn map_pos<U, G, MF : FnOnce(T, Position<F>) -> (U, Position<G>)>(self, f : MF) -> Pos<U, G> {
        let (nel, npos) = f(self.el, self.pos);
        Pos{
            el : nel,
            pos : npos,
        }
    }

    /// Converts it to a reference
    pub fn make_ref(&self) -> Pos<&T, F> where F : Clone {
        Pos{
            el : &(self.el),
            pos : self.pos.clone(),
        }
    }
    /// Converts it to a mutable reference
    pub fn make_mut(&mut self) -> Pos<&mut T, F> where F : Clone {
        Pos{
            el : &mut(self.el),
            pos : self.pos.clone(),
        }
    }
}

impl<T, E, F> Pos<Result<T, E>, F> {
    /// Build result from output
    pub fn from_output(out : Pos<T, F>) -> Self {
        Self{
            el : Ok(out.el),
            pos : out.pos,
        }
    }
    /// Build result from output and position
    pub fn from_output_pos(out : T, pos : Position<F>) -> Self {
        Self::from_output(Pos::new(out, pos))
    }
    /// Build result from residual
    #[allow(clippy::missing_panics_doc)]
    pub fn from_residual(out : Pos<Result<core::convert::Infallible, E>, F>) -> Self {
        Self{
            el : match out.el {
                Err(e) => Err(e),
                Ok(_) => panic!("Infallible"),
            },
            pos : out.pos,
        }
    }
    /// Build result from residual and position
    pub fn from_residual_pos(out : Result<core::convert::Infallible, E>, pos : Position<F>) -> Self {
        Self::from_residual(Pos::new(out, pos))
    }
    /// Build result from error and position
    pub const fn from_err_pos(out : E, pos : Position<F>) -> Self {
        Self{
            el : Err(out),
            pos,
        }
    }
    ///Branch
    pub fn branch(self) -> ControlFlow<Pos<Result<core::convert::Infallible, E>, F>, Pos<T, F>>{
        let pos = self.pos;
        match self.el {
            Ok(t) => ControlFlow::Continue(Pos{
                el : t,
                pos,
            }),
            Err(e) => ControlFlow::Break(Pos{
                el : Err(e),
                pos,
            }),
        }
    }


    /// Converts a `Pos<Result<T, E>, F>` into `Result<Pos<T, F>, Pos<E, F>>` that can be used with
    /// the `?` operator 
    #[allow(clippy::missing_errors_doc)]
    pub fn throw(self) -> Result<Pos<T, F>, Pos<E, F>> {
        let pos = self.pos;
        match self.el {
            Ok(t) => Ok(Pos{
                el : t,
                pos,
            }),
            Err(e) => Err(Pos{
                el : e,
                pos,
            }),
        }
    }
    /// Converts a `Pos<Result<T, E>, F>` into `Result<Pos<T, F>, E>` that can be used with
    /// the `?` operator 
    #[allow(clippy::missing_errors_doc)]
    pub fn throw_el(self) -> Result<Pos<T, F>, E> {
        match self.el {
            Ok(el) => Ok(Pos{el, pos : self.pos}),
            Err(e) => Err(e),
        }
    }
    /// Converts a `Pos<Result<T, E>, F>` into `Result<T, Pos<E, F>>` that can be used with
    /// the `?` operator 
    #[allow(clippy::missing_errors_doc)]
    pub fn throw_err(self) -> Result<T, Pos<E, F>> {
        match self.el {
            Ok(t) => Ok(t),
            Err(el) => Err(Pos{el, pos : self.pos}),
        }
    }
}

impl<T, F> Pos<Option<T>, F> {
    /// Applies `Option::ok_or` to the wrapped object and then [`Pos::throw`] the resulting object
    ///
    /// # Errors
    /// If the wrapped value is `None` then trows passed error
    pub fn ok_or<E>(self, err : E) -> Result<Pos<T, F>, Pos<E, F>> {
        self.map(|i| i.ok_or(err)).throw()
    }
    /// Applies `Option::ok_or` to the wrapped object and then [`Pos::throw_err`] the resulting
    /// object
    ///
    /// # Errors
    /// If the wrapped value is `None` then trows passed error
    pub fn ok_or_err<E>(self, err : E) -> Result<T, Pos<E, F>> {
        self.map(|i| i.ok_or(err)).throw_err()
    }
}

impl<B, C, F> Pos<ControlFlow<B, C>, F> {
    /// Converts a `Pos<ControlFlow<B, C>, F>` into a `ControlFlow<Pos<B, F>, C>` by moving the
    /// resulting `Position` to the break case
    pub fn try_break(self) -> ControlFlow<Pos<B, F>, C> {
        let pos = self.pos;
        match self.el {
            ControlFlow::Break(b) => ControlFlow::Break(Pos::new(b, pos)),
            ControlFlow::Continue(c) => ControlFlow::Continue(c),
        }
    }
}
