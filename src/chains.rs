//! Items that can be used both as [`Atom`](crate::atoms::Atom) and as
//! [`ParseTool`](crate::view::ParseTool).

#[cfg(feature = "alloc")]
use crate::view::{ParseTool, View};

// Associate trait that abstract both View and MatchHelper
pub(crate) trait Chain<T> : Sized{
    type Error;
    type Data;

    fn chain(self, t : &T) -> Result<(Self::Data, Self), Self::Error>;
    fn chain_nodata(self, t : &T) -> Result<Self, Self::Error>{
        self.chain(t).map(|i| i.1)
    }
}

// Inserter for both T and SEP
pub(crate) trait InsertB<TD, SEPD> {
    fn insert(&mut self, data : TD);
    fn insert_sep(&mut self, data : SEPD);
}

#[derive(Debug, Copy, Clone, Default)]
pub(crate) struct Count(pub(crate) usize);

impl<TD, SEPD> InsertB<TD, SEPD> for Count{
    fn insert(&mut self, _ : TD){
        self.0 += 1;
    }
    fn insert_sep(&mut self, _ : SEPD){}
}
impl Count {
    pub(crate) const fn new() -> Self {
        Self(0)
    }
}

#[cfg(feature = "alloc")]
impl<TD, SEPD> InsertB<TD, SEPD> for alloc::vec::Vec<TD>{
    fn insert(&mut self, data : TD){
        self.push(data);
    }
    fn insert_sep(&mut self, _ : SEPD){}
}

pub(crate) fn ihelp<T, SEP, M, I>(atom : &T, sep : &SEP, st : M, vec : &mut I) -> Option<M>
    where M : Chain<T> + Chain<SEP>,
          I : InsertB<<M as Chain<T>>::Data, <M as Chain<SEP>>::Data>,
    {
        let (si, h) = st.chain(sep).ok()?;
        let (i, ret) = h.chain(atom).ok()?;
        vec.insert_sep(si);
        vec.insert(i);
        Some(ret)
    }
pub(crate) fn ihelpe<T, SEP, M, I, E>(atom : &T, sep : &SEP, st : M, vec : &mut I) -> Result<M, E>
    where M : Chain<T, Error = E> + Chain<SEP, Error = E>,
          I : InsertB<<M as Chain<T>>::Data, <M as Chain<SEP>>::Data>
    {
        let (si, h) = st.chain(sep)?;
        let (i, ret) = h.chain(atom)?;
        vec.insert_sep(si);
        vec.insert(i);
        Ok(ret)
    }

#[derive(Debug, Copy, Clone)]
pub(crate) struct PfxLen(pub(crate) usize);

impl InsertB<usize, usize> for PfxLen{
    fn insert(&mut self, data : usize){
        self.0 += data;
    }
    fn insert_sep(&mut self, data : usize){
        self.0 += data;
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
    pub const fn new(first : F, second : S) -> Self {
        Self{first, second}
    }
    #[allow(clippy::type_complexity)]
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
    pub const fn new(first : F, second : S) -> Self {
        Self{first, second}
    }
    pub(crate) fn parse_logic<D, M : Clone + Chain<F, Data = D> + Chain<S, Data = D> >(&self, m : M) -> Result<(D, M), <M as Chain<S>>::Error> {
        m.clone().chain(&self.first).map_or_else(
            |_| m.chain(&self.second),
            Ok
        )
    }
}

/// Only checks the provided atom, without progressing.
///
/// ```rust
/// use minparser::prelude::*;
///
/// MatchHelper::from("a").match_atom(Check('a')).unwrap()
///     .match_atom(Check::<fn(&char) -> bool>(char::is_ascii)).unwrap();
/// ```
#[derive(Debug, Copy, Clone, Default)]
pub struct Check<T>(pub T);

impl<T> Check<T>{
    pub(crate) fn check<M : Chain<T> + Clone>(&self, m : M) -> Result<(M::Data, M), M::Error> {
        let (d, _) = m.clone().chain(&self.0)?;
        Ok((d, m))
    }
}

/// Matches only if the provided atom doesn't match.
///
/// ```rust
/// use minparser::prelude::*;
///
/// MatchHelper::from("ab").match_atom(CheckInv('c')).unwrap()
///     .match_atom(CheckInv('b')).unwrap();
/// ```
#[derive(Debug, Copy, Clone, Default)]
pub struct CheckInv<T>(pub T);
impl<T> CheckInv<T>{
    pub(crate) fn check<M : Chain<T> + Clone>(&self, m : M) -> Result<(M::Error, M), M::Data> {
        match m.clone().chain(&self.0) {
            Err(e) => Ok((e, m)),
            Ok((d, _)) => Err(d),
        }
    }
}

/// Tool that matches repetitions with separator requiring a minimum number of repetitions.
///
/// ```rust
/// use minparser::prelude::*;
/// let lt = MatchHelper::from("a a a a b");
/// assert_eq!(lt.match_atom_string(Repeat::new_bounds('a', ' ', 0, 3))
/// .unwrap().0, "a a a");
/// assert_eq!(lt.match_atom_string(Repeat::new_bounds('a', ' ', 2, 3))
/// .unwrap().0, "a a a");
/// assert_eq!(lt.match_atom_string(Repeat::new_unbounded('a', ' ', 0))
/// .unwrap().0, "a a a a");
/// assert_eq!(lt.match_atom_string(Repeat::new_unbounded('a', ' ', 2))
/// .unwrap().0, "a a a a");
/// assert!(lt.match_atom_string(Repeat::new_unbounded('a', ' ', 5))
/// .is_err());
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Repeat<T, SEP>{
    pub(crate) atom : T,
    pub(crate) sep : SEP,
    pub(crate) min : usize,
    pub(crate) max : Option<usize>,
}

impl<T, SEP> Repeat<T, SEP>{
    /// Create a new [`Repeat`] with specified separator and upper bound.
    ///
    /// # Panics
    /// It panic when `max` is strictly less than `min`, because in such case no matches are
    /// possible
    pub fn new_bounds(atom : T, sep : SEP, min : usize, max : usize) -> Self {
        assert!(min <= max, "Maximum value {max} is strictly less than minimum {min}");
        Self {
            atom,
            sep,
            min,
            max : Some(max)
        }
    }
    /// Create a new [`Repeat`] with specified separator without upper bound
    pub const fn new_unbounded(atom : T, sep : SEP, min : usize) -> Self {
        Self {
            atom,
            sep,
            min,
            max : None
        }
    }
}

impl<T, SEP> Repeat<T, SEP> { 
    pub(crate) fn parse_logic<
            E, 
            M : Clone + Chain<T, Error = E> + Chain<SEP, Error = E>,
            I : InsertB<<M as Chain<T>>::Data, <M as Chain<SEP>>::Data>
        >(&self, st : M, vec : &mut I) -> Result<M, E>{
        let mut helper = st;
        let mut start = self.min;
        if self.min > 0 {
            helper = {
                let (i, h) = helper.chain(&self.atom)?;
                vec.insert(i);
                h
            };
            for _ in 1..(self.min) {
                helper = ihelpe(&self.atom, &self.sep, helper, vec)?;
            }
        }
        else {
            match helper.clone().chain(&self.atom) {
                Ok((ii, h)) => {
                    helper = h;
                    start += 1;
                    vec.insert(ii);
                }
                Err(_) => return Ok(helper),
            }
        }
        if let Some(max) = self.max {
            for _i in start..max {
                if let Some(h) = ihelp(&self.atom, &self.sep, helper.clone(), vec) {
                    helper = h;
                }
                else {
                    return Ok(helper);
                }
            }
            Ok(helper)
        }
        else {
            loop {
                if let Some(h) = ihelp(&self.atom, &self.sep, helper.clone(), vec) {
                    helper = h;
                }
                else {
                    return Ok(helper);
                }
            }
        }
    }
}

/// Tool that matches repetitions with separator.
///
/// Like [`Repeat`] but without specifying a minimum number of repetitions. Therefore. it will
/// always match.
#[derive(Debug, Clone, Copy)]
pub struct RepeatAny<T, SEP>{
    pub(crate) atom : T,
    pub(crate) sep : SEP,
    pub(crate) max : Option<usize>,
}

impl<T, SEP> RepeatAny<T, SEP>{
    /// Create a new [`RepeatAny`] with specified separator.
    pub const fn new(atom : T, sep : SEP, max : Option<usize>) -> Self {
        Self{
            atom,
            sep,
            max,
        }
    }
    /// Create a new [`RepeatAny`] with specified separator and upper bound.
    pub const fn new_bounds(atom : T, sep : SEP, max : usize) -> Self {
        Self::new(atom, sep, Some(max))
    }
    /// Create a new [`RepeatAny`] with specified separator without upper bound
    pub const fn new_unbounded(atom : T, sep : SEP) -> Self {
        Self::new(atom, sep, None)
    }
}

impl<T, SEP> RepeatAny<T, SEP> { 
    pub(crate) fn parse_logic<M : Clone + Chain<T> + Chain<SEP>, I : InsertB<<M as Chain<T>>::Data, <M as Chain<SEP>>::Data>>(&self, st : M, vec : &mut I) -> M {
        let mut helper = st;
        match helper.clone().chain(&self.atom) {
            Ok((ii, h)) => {
                helper = h;
                vec.insert(ii);
            }
            Err(_) => return helper,
        }
        if let Some(max) = self.max {
            for _i in 1..max {
                if let Some(h) = ihelp(&self.atom, &self.sep, helper.clone(), vec) {
                    helper = h;
                }
                else {
                    return helper;
                }
            }
            helper
        }
        else {
            loop {
                if let Some(h) = ihelp(&self.atom, &self.sep, helper.clone(), vec) {
                    helper = h;
                }
                else {
                    return helper;
                }
            }
        }
    }
}

/// Tool that matches repetitions lazily.
///
/// It matches the least number of `T` atom (sepatared by `SEP`) which are followed by `TERM` atom.
/// The difference with respect to a [`Repeat`] followed by `TERM` is that here repetitions are
/// evaluated lazily: it interrupts at the first match of `TERM`, whereas `Repeat` evaluates
/// repetitions eagerly and so `TERM` is matched only after the repetition ends.
///
/// ```rust
/// use minparser::prelude::*;
/// let mh = MatchHelper::from("\"ABC\" \"defg\" \"hi");
/// let (su, mh) = mh.match_atom_string(Seq{
///     first : '\"',
///     second : LazyRepeat::new_unbounded(AnyChar, TrueAtom, '\"', 0)
///     }).unwrap();
/// assert_eq!(su, "\"ABC\"");
/// let (su, mh) = mh.match_atom_string(Seq{
///     first : " \"",
///     second : LazyRepeat::new_unbounded(AnyChar, TrueAtom, '\"', 0)
///     }).unwrap();
/// assert_eq!(su, " \"defg\"");
/// assert!(mh.match_atom_string(Seq{
///     first : " \"",
///     second : LazyRepeat::new_unbounded(AnyChar, TrueAtom, '\"', 0)
///     }).is_err());
/// ```
#[derive(Copy, Clone, Debug)]
pub struct LazyRepeat<T, SEP, TERM>{
    atom : T,
    sep : SEP,
    term : TERM,
    min : usize,
    max : Option<usize>,
}

impl<T, SEP, TERM> LazyRepeat<T, SEP, TERM>{
    /// Create a new `LazyRepeat` with specified upper bound.
    ///
    /// # Panics
    /// Panic if `max` is strictly lesser than `min`.
    pub fn new_bounds(atom : T, sep : SEP, term : TERM, min : usize, max : usize) -> Self {
        assert!(min <= max, "Maximum value {max} is strictly less than minimum {min}");
        Self {
            atom,
            sep,
            term,
            min,
            max : Some(max)
        }
    }
    /// Create a new `LazyRepeat` without upper bound.
    pub const fn new_unbounded(atom : T, sep : SEP, term : TERM, min : usize) -> Self {
        Self {
            atom,
            sep,
            term,
            min,
            max : None
        }
    }
}

impl<T, SEP, TERM> LazyRepeat<T, SEP, TERM> {
    pub(crate) fn parse_logic<E, M, I>(&self, st : M, vec : &mut I) -> Result<M, E>
        where M : Clone + Chain<T, Error = E> + Chain<SEP, Error = E> + Chain<TERM, Error = E>,
              I : InsertB<<M as Chain<T>>::Data, <M as Chain<SEP>>::Data>
    {
        let mut helper = st;
        let mut start = self.min;
        if self.min > 0 {
            helper = {
                let (i, ret) = helper.chain(&self.atom)?;
                vec.insert(i);
                ret
            };
            for _ in 1..(self.min) {
                helper = ihelpe(&self.atom, &self.sep, helper, vec)?;
            }
        }
        else{
            match helper.clone().chain_nodata(&self.term) {
                Ok(hh) => {
                    return Ok(hh);
                }
                Err(e) => {
                    if let Ok((hi, h)) = helper.chain(&self.atom) {
                        helper = h;
                        start += 1;
                        vec.insert(hi);
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
                        if let Some(h) = ihelp(&self.atom, &self.sep, helper, vec) {
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
                match helper.clone().chain_nodata(&self.term) {
                    Ok(hh) => {
                        return Ok(hh);
                    }
                    Err(e) => {
                        if let Some(h) = ihelp(&self.atom, &self.sep, helper, vec) {
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
    #[cfg(feature = "alloc")]
    /// Parse and store data in `vec`.
    #[allow(clippy::missing_errors_doc)]
    pub fn parse_store<'a, F, E >(&self, st : View<'a, F>, vec : &mut alloc::vec::Vec<T::Data>) -> Result<View<'a, F>, E> 
        where F : Clone,
              T : ParseTool<'a, F, Error = E>, 
              SEP : ParseTool<'a, F, Error = E>, 
              TERM : ParseTool<'a, F, Error = E> {
            self.parse_logic(st, vec)
    }
}

/// Like [`LazyRepeat`] but without a minimum number of repetitions
///
/// ```rust
/// use minparser::prelude::*;
/// let mh = MatchHelper::from("\"ABC\" \"defg\" \"hi");
/// let (su, mh) = mh.match_atom_string(Seq{
///     first : '\"',
///     second : LazyRepeatAny::new_unbounded(AnyChar, TrueAtom, '\"')
///     }).unwrap();
/// assert_eq!(su, "\"ABC\"");
/// let (su, mh) = mh.match_atom_string(Seq{
///     first : " \"",
///     second : LazyRepeatAny::new_unbounded(AnyChar, TrueAtom, '\"')
///     }).unwrap();
/// assert_eq!(su, " \"defg\"");
/// assert!(mh.match_atom_string(Seq{
///     first : " \"",
///     second : LazyRepeatAny::new_unbounded(AnyChar, TrueAtom, '\"')
///     }).is_err());
/// ```
#[derive(Copy, Clone, Debug)]
pub struct LazyRepeatAny<T, SEP, TERM>{
    atom : T,
    sep : SEP,
    term : TERM,
    max : Option<usize>,
}

impl<T, SEP, TERM> LazyRepeatAny<T, SEP, TERM>{
    /// Create a new `LazyRepeatAny` with specified upper bound.
    pub const fn new_bounds(atom : T, sep : SEP, term : TERM, max : usize) -> Self {
        Self {
            atom,
            sep,
            term,
            max : Some(max)
        }
    }
    /// Create a new `LazyRepeatAny` without upper bound.
    pub const fn new_unbounded(atom : T, sep : SEP, term : TERM) -> Self {
        Self {
            atom,
            sep,
            term,
            max : None
        }
    }
}

impl<T, SEP, TERM> LazyRepeatAny<T, SEP, TERM> {
    pub(crate) fn parse_logic<M, I>(&self, st : M, vec : &mut I) -> Result<M, <M as Chain<TERM>>::Error >
        where M : Clone + Chain<T> + Chain<SEP> + Chain<TERM>,
              I : InsertB<<M as Chain<T>>::Data, <M as Chain<SEP>>::Data>
    {
        let mut helper = st;
        match helper.clone().chain_nodata(&self.term) {
            Ok(hh) => {
                return Ok(hh);
            }
            Err(e) => {
                if let Ok((hi, h)) = helper.chain(&self.atom) {
                    helper = h;
                    vec.insert(hi);
                }
                else{
                    return Err(e);
                }
            }
        }
        if let Some(max) = self.max {
            for _i in 1..max {
                match helper.clone().chain_nodata(&self.term) {
                    Ok(hh) => {
                        return Ok(hh);
                    }
                    Err(e) => {
                        if let Some(h) = ihelp(&self.atom, &self.sep, helper, vec) {
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
                match helper.clone().chain_nodata(&self.term) {
                    Ok(hh) => {
                        return Ok(hh);
                    }
                    Err(e) => {
                        if let Some(h) = ihelp(&self.atom, &self.sep, helper, vec) {
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
    #[cfg(feature = "alloc")]
    /// Parse and store data in `vec`.
    #[allow(clippy::missing_errors_doc)]
    pub fn parse_store<'a, F, E >(&self, st : View<'a, F>, vec : &mut alloc::vec::Vec<T::Data>) -> Result<View<'a, F>, E> 
        where F : Clone, 
            T : ParseTool<'a, F, Error = E>, 
            SEP : ParseTool<'a, F, Error = E>, 
            TERM : ParseTool<'a, F, Error = E> {
            self.parse_logic(st, vec)
    }
}
