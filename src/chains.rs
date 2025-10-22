//! Items that can be used both as [`Atom`](crate::atoms::Atom) and as
//! [`ParseTool`](crate::view::ParseTool).
use core::marker::PhantomData;

// Associate trait that abstract both View and MatchHelper
pub(crate) trait Chain<T> : Sized{
    type Error;
    type Data;

    fn chain(self, t : &T) -> Result<(Self::Data, Self), Self::Error>;
    fn chain_nodata(self, t : &T) -> Result<Self, Self::Error>{
        self.chain(t).map(|i| i.1)
    }
    fn chain_append<I : Insert<Self::Data>>(self, t : &T, vec : &mut I) -> Result<Self, Self::Error> {
        self.chain(t).map(|(d, m)| {
            vec.insert(d);
            m
        })
    }
}

/// Trait that generalize a container with an `insert` method.
///
/// This trait is already implemented for [`Vec`](alloc::vec::Vec) and
/// [`String`](alloc::string::String) when the `alloc` feature is used.
pub trait Insert<D> {
    /// Inserts a new element.
    fn insert(&mut self, data : D);
}

/// Implementor of [`Insert`] that only counts the elements without storing them.
#[derive(Debug, Copy, Clone)]
pub struct Count(pub usize);

impl Count {
    /// Creates an empty `Count`.
    pub fn new() -> Self {
        Self(0)
    }
}

impl Default for Count {
    fn default() -> Self {
        Self(0)
    }
}

impl<D> Insert<D> for Count{
    fn insert(&mut self, _data : D){
        self.0 += 1;
    }
}

#[cfg(feature = "alloc")]
impl<D> Insert<D> for alloc::vec::Vec<D>{
    fn insert(&mut self, data : D){
        self.push(data);
    }
}
#[cfg(feature = "alloc")]
impl Insert<char> for alloc::string::String{
    fn insert(&mut self, data : char){
        self.push(data);
    }
}

/// Matches both the provided atoms, trying `first` before `second`.
pub struct Seq<F, S>{
    /// The first atom to be checked
    pub first : F,
    /// The second atom to be checked.
    pub second : S,
}

impl<F, S> Seq<F, S> {
    /// Creates a new `Seq`.
    pub fn new(first : F, second : S) -> Self {
        Self{first, second}
    }
    pub(crate) fn parse_logic<E, M : Chain<F, Error = E> + Chain<S, Error = E> >(&self, m : M) -> Result<((<M as Chain<F>>::Data, <M as Chain<S>>::Data), M), E> {
        match m.chain(&self.first) {
            Err(e) => Err(e),
            Ok((fd, mm)) => mm.chain(&self.second)
                .map(|(sd, r)| ((fd, sd), r)),
        }
    }
}

/// Matches at least one of the provided atoms, trying `first` before `second`.
pub struct Or<F, S>{
    /// The first atom to be checked
    pub first : F,
    /// The second atom to be checked.
    ///
    /// If `first` matches then `second` would not be tested.
    pub second : S,
}

impl<F, S> Or<F, S> {
    /// Creates a new `Or`.
    pub fn new(first : F, second : S) -> Self {
        Self{first, second}
    }
    pub(crate) fn parse_logic<D, M : Clone + Chain<F, Data = D> + Chain<S, Data = D> >(&self, m : M) -> Result<(D, M), <M as Chain<S>>::Error> {
        match m.clone().chain(&self.first) {
            Ok(s) => Ok(s),
            Err(_) => m.chain(&self.second),
        }
    }
}

/// Tool that matches repetitions with separator requiring a minimum number of repetitions.
///
/// ```rust
/// use minparser::prelude::*;
/// let lt = MatchHelper::from("a a a a b");
/// assert_eq!(lt.match_atom_string(RepeatAtom::new_bounds('a', ' ', 0, 3))
/// .unwrap().0, "a a a");
/// assert_eq!(lt.match_atom_string(RepeatAtom::new_bounds('a', ' ', 2, 3))
/// .unwrap().0, "a a a");
/// assert_eq!(lt.match_atom_string(RepeatAtom::new_unbounded('a', ' ', 0))
/// .unwrap().0, "a a a a");
/// assert_eq!(lt.match_atom_string(RepeatAtom::new_unbounded('a', ' ', 2))
/// .unwrap().0, "a a a a");
/// assert!(lt.match_atom_string(RepeatAtom::new_unbounded('a', ' ', 5))
/// .is_err());
/// ```
#[derive(Debug, Clone, Copy)]
pub struct RepeatAtom<T, SEP>{
    pub(crate) atom : T,
    pub(crate) sep : SEP,
    pub(crate) min : usize,
    pub(crate) max : Option<usize>,
}

impl<T, SEP> RepeatAtom<T, SEP>{
    /// Create a new [`RepeatAtom`] with specified separator and upper bound.
    ///
    /// # Panics
    /// It panic when `max` is strictly less than `min`, because in such case no matches are
    /// possible
    pub fn new_bounds(atom : T, sep : SEP, min : usize, max : usize) -> Self {
        if min > max {
            panic!("Maximum value {max} is strictly less than minimum {min}");
        }
        Self {
            atom,
            sep,
            min,
            max : Some(max)
        }
    }
    /// Create a new [`RepeatAtom`] with specified separator without upper bound
    pub const fn new_unbounded(atom : T, sep : SEP, min : usize) -> Self {
        Self {
            atom,
            sep,
            min,
            max : None
        }
    }
    /// Wraps it in a [`WithCont`].
    pub const fn wrap<I>(self) -> WithCont<Self, I> {
        WithCont(self, PhantomData)
    }
}

impl<T, SEP> RepeatAtom<T, SEP> { 
    pub(crate) fn parse_logic<
            E, 
            M : Clone + Chain<T, Error = E> + Chain<SEP, Error = E>,
            I : Insert<<M as Chain<T>>::Data>
        >(&self, st : M, vec : &mut I) -> Result<M, E>{
        let mut helper = st;
        let mut start = self.min;
        if self.min > 0 {
            helper = helper.chain_append(&self.atom, vec)?;
            for _ in 1..(self.min) {
                helper = helper.chain_nodata(&self.sep)?.chain_append(&self.atom, vec)?;
            }
        }
        else {
            match helper.clone().chain_append(&self.atom, vec) {
                Ok(h) => {
                    helper = h;
                    start += 1;
                }
                Err(_) => return Ok(helper),
            }
        }
        if let Some(max) = self.max {
            for _i in start..max {
                if let Ok(hh) = helper.clone().chain_nodata(&self.sep)
                    && let Ok(h) = hh.chain_append(&self.atom, vec) {
                    helper = h;
                }
                else {
                    return Ok(helper);
                }
            }
            return Ok(helper);
        }
        else {
            loop {
                if let Ok(hh) = helper.clone().chain_nodata(&self.sep)
                    && let Ok(h) = hh.chain_append(&self.atom, vec) {
                    helper = h;
                }
                else {
                    return Ok(helper);
                }
            }
        }
    }
    /// Parse and store data in `vec`.
    pub fn parse_store<'a, F, E, I >(&self, st : View<'a, F>, vec : &mut I) -> Result<View<'a, F>, E> 
        where F : Clone, T : ParseTool<'a, F, Error = E>, SEP : ParseTool<'a, F, Error = E>, I : Insert<T::Data> {
            self.parse_logic(st, vec)
    }
}

/// Tool that matches repetitions with separator.
///
/// Like [`RepeatAtom`] but without specifying a minimum number of repetitions. Therefore. it will
/// always match.
#[derive(Debug, Clone, Copy)]
pub struct RepeatAnyAtom<T, SEP>{
    pub(crate) atom : T,
    pub(crate) sep : SEP,
    pub(crate) max : Option<usize>,
}

impl<T, SEP> RepeatAnyAtom<T, SEP>{
    /// Create a new [`RepeatAnyAtom`] with specified separator.
    pub const fn new(atom : T, sep : SEP, max : Option<usize>) -> Self {
        Self{
            atom,
            sep,
            max,
        }
    }
    /// Create a new [`RepeatAnyAtom`] with specified separator and upper bound.
    pub const fn new_bounds(atom : T, sep : SEP, max : usize) -> Self {
        Self::new(atom, sep, Some(max))
    }
    /// Create a new [`RepeatAnyAtom`] with specified separator without upper bound
    pub const fn new_unbounded(atom : T, sep : SEP) -> Self {
        Self::new(atom, sep, None)
    }
    /// Wraps it in a [`WithCont`].
    pub const fn wrap<I>(self) -> WithCont<Self, I> {
        WithCont(self, PhantomData)
    }
}

impl<T, SEP> RepeatAnyAtom<T, SEP> { 
    pub(crate) fn parse_logic<M : Clone + Chain<T> + Chain<SEP>, I : Insert<<M as Chain<T>>::Data>>(&self, st : M, vec : &mut I) -> M{
        let mut helper = st;
        match helper.clone().chain_append(&self.atom, vec) {
            Ok(h) => {
                helper = h;
            }
            Err(_) => return helper,
        }
        if let Some(max) = self.max {
            for _i in 1..max {
                if let Ok(hh) = helper.clone().chain_nodata(&self.sep)
                    && let Ok(h) = hh.chain_append(&self.atom, vec) {
                    helper = h;
                }
                else {
                    return helper;
                }
            }
            return helper;
        }
        else {
            loop {
                if let Ok(hh) = helper.clone().chain_nodata(&self.sep)
                    && let Ok(h) = hh.chain_append(&self.atom, vec) {
                    helper = h;
                }
                else {
                    return helper;
                }
            }
        }
    }
    /// Parse and store data in `vec`.
    pub fn parse_store<'a, F, I >(&self, st : View<'a, F>, vec : &mut I) -> View<'a, F> 
        where F : Clone, T : ParseTool<'a, F>, SEP : ParseTool<'a, F>, I : Insert<T::Data> {
            self.parse_logic(st, vec)
    }
}

/// Tool that matches repetitions lazily.
///
/// It matches the least number of `T` atom (sepatared by `SEP`) which are followed by `TERM` atom.
/// The difference with respect to a [`RepeatAtom`] followed by `TERM` is that here repetitions are
/// evaluated lazily: it interrupts at the first match of `TERM`, whereas `RepeatAtom` evaluates
/// repetitions eagerly and so `TERM` is matched only after the repetition ends.
///
/// ```rust
/// use minparser::prelude::*;
/// let mh = MatchHelper::from("\"ABC\" \"defg\" \"hi");
/// let (su, mh) = mh.match_atom_string(Seq{
///     first : '\"',
///     second : LazyRepeatAtom::new_unbounded(AnyChar, TrueAtom, '\"', 0)
///     }).unwrap();
/// assert_eq!(su, "\"ABC\"");
/// let (su, mh) = mh.match_atom_string(Seq{
///     first : " \"",
///     second : LazyRepeatAtom::new_unbounded(AnyChar, TrueAtom, '\"', 0)
///     }).unwrap();
/// assert_eq!(su, " \"defg\"");
/// assert!(mh.match_atom_string(Seq{
///     first : " \"",
///     second : LazyRepeatAtom::new_unbounded(AnyChar, TrueAtom, '\"', 0)
///     }).is_err());
/// ```
#[derive(Copy, Clone, Debug)]
pub struct LazyRepeatAtom<T, SEP, TERM>{
    atom : T,
    sep : SEP,
    term : TERM,
    min : usize,
    max : Option<usize>,
}

impl<T, SEP, TERM> LazyRepeatAtom<T, SEP, TERM>{
    /// Create a new `LazyRepeatAtom` with specified upper bound.
    ///
    /// # Panics
    /// Panic if `max` is strictly lesser than `min`.
    pub fn new_bounds(atom : T, sep : SEP, term : TERM, min : usize, max : usize) -> Self {
        if min > max {
            panic!("Maximum value {max} is strictly less than minimum {min}");
        }
        Self {
            atom,
            sep,
            term,
            min,
            max : Some(max)
        }
    }
    /// Create a new `LazyRepeatAtom` without upper bound.
    pub const fn new_unbounded(atom : T, sep : SEP, term : TERM, min : usize) -> Self {
        Self {
            atom,
            sep,
            term,
            min,
            max : None
        }
    }
    /// Wraps it in a [`WithCont`].
    pub const fn wrap<I>(self) -> WithCont<Self, I> {
        WithCont(self, PhantomData)
    }
}

use crate::view::{View, ParseTool};
impl<T, SEP, TERM> LazyRepeatAtom<T, SEP, TERM> {
    pub(crate) fn parse_logic<E, M : Clone + Chain<T, Error = E> + Chain<SEP, Error = E> + Chain<TERM, Error = E>, I : Insert<<M as Chain<T>>::Data> >(&self, st : M, vec : &mut I) -> Result<M, E>{
        let mut helper = st;
        let mut start = self.min;
        if self.min > 0 {
            helper = helper.chain_append(&self.atom, vec)?;
            for _ in 1..(self.min) {
                helper = helper.chain_nodata(&self.sep)?.chain_append(&self.atom, vec)?;
            }
        }
        else{
            match helper.clone().chain_nodata(&self.term) {
                Ok(hh) => {
                    return Ok(hh);
                }
                Err(e) => {
                    if let Ok(h) = helper.chain_append(&self.atom, vec) {
                        helper = h;
                        start += 1;
                    }
                    else{
                        return Err(e);
                    }
                }
            }
        }
        if let Some(max) = self.max {
            for _i in start..max {
                match helper.clone().chain_nodata(&self.term) {
                    Ok(hh) => {
                        return Ok(hh);
                    }
                    Err(e) => {
                        if let Ok(h1) = helper.chain_nodata(&self.sep)
                        && let Ok(h) = h1.chain_append(&self.atom, vec) {
                            helper = h;
                        }
                        else{
                            return Err(e);
                        }
                    }
                }
            }
            helper.chain_nodata(&self.term)
        }
        else {
            loop {
                loop {
                    match helper.clone().chain_nodata(&self.term) {
                        Ok(hh) => {
                            return Ok(hh);
                        }
                        Err(e) => {
                            if let Ok(h1) = helper.chain_nodata(&self.sep)
                            && let Ok(h) = h1.chain_append(&self.atom, vec) {
                                helper = h;
                            }
                            else{
                                return Err(e);
                            }
                        }
                    }
                }
            }
        }
    }
    /// Parse and store data in `vec`.
    pub fn parse_store<'a, F, E, I >(&self, st : View<'a, F>, vec : &mut I) -> Result<View<'a, F>, E> 
        where F : Clone, T : ParseTool<'a, F, Error = E>, SEP : ParseTool<'a, F, Error = E>, TERM : ParseTool<'a, F, Error = E>, I : Insert<T::Data> {
            self.parse_logic(st, vec)
    }
}

/// Wrapper for [`RepeatAtom`], [`RepeatAnyAtom`] and [`LazyRepeatAtom`] that allows you to specify
/// the container in which store retrieved data.
#[derive(Debug, Copy, Clone)]
pub struct WithCont<W, I>(pub(crate) W, PhantomData<I>);

impl<W, I> AsRef<W> for WithCont<W, I> {
    fn as_ref(&self) -> &W {
        &self.0
    }
}
