//! Items that can be used both as [`Atom`](crate::atoms::Atom) and as
//! [`ParseTool`](crate::view::ParseTool).
use core::ops::ControlFlow;
use core::marker::PhantomData;

// Associate trait that abstract both View and MatchHelper
pub(crate) trait Chain<T> : Sized{
    type Error;
    type Data;

    fn chain(self, t : &T) -> ControlFlow<Self::Error, (Self::Data, Self)>;
    fn chain_nodata(self, t : &T) -> ControlFlow<Self::Error, Self>{
        self.chain(t).map_continue(|i| i.1)
    }
    fn chain_append<I : Insert<Self::Data>>(self, t : &T, vec : &mut I) -> ControlFlow<Self::Error, Self> {
        self.chain(t).map_continue(|(d, m)| {
            vec.insert(d);
            m
        })
    }
}

/// Trait that generalize a container with an `insert` method.
pub trait Insert<D> {
    /// Creates an empty container
    fn new() -> Self;

    /// Inserts a new element.
    fn insert(&mut self, data : D);
}

/// Implementor of [`Insert`] that only counts the elements without storing them.
#[derive(Debug, Copy, Clone)]
pub struct Count(pub usize);

impl<D> Insert<D> for Count{
    fn new() -> Self {
        Count(0)
    }

    fn insert(&mut self, _data : D){
        self.0 += 1;
    }
}

#[cfg(feature = "alloc")]
impl<D> Insert<D> for alloc::vec::Vec<D>{
    fn new() -> Self {
        alloc::vec::Vec::new()
    }

    fn insert(&mut self, data : D){
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
    pub(crate) fn parse_logic<E, M : Chain<F, Error = E> + Chain<S, Error = E> >(&self, m : M) -> ControlFlow<E, ((<M as Chain<F>>::Data, <M as Chain<S>>::Data), M)> {
        match m.chain(&self.first) {
            ControlFlow::Break(e) => ControlFlow::Break(e),
            ControlFlow::Continue((fd, mm)) => mm.chain(&self.second)
                .map_continue(|(sd, r)| ((fd, sd), r)),
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
    pub(crate) fn parse_logic<D, M : Clone + Chain<F, Data = D> + Chain<S, Data = D> >(&self, m : M) -> ControlFlow<<M as Chain<S>>::Error, (D, M)> {
        match m.clone().chain(&self.first) {
            ControlFlow::Continue(s) => ControlFlow::Continue(s),
            ControlFlow::Break(_) => m.chain(&self.second),
        }
    }
}


/// Tool that matches repetitions with separator requiring a minimum number of repetitions.
///
/// ```rust
/// use minparser::prelude::*;
/// let lt = MatchHelper::from("a a a a b");
/// assert_eq!(lt.match_atom_string(RepeatAtom::new_sep_bounds('a', ' ', 0, 3))
/// .unwrap().1, "a a a");
/// assert_eq!(lt.match_atom_string(RepeatAtom::new_sep_bounds('a', ' ', 2, 3))
/// .unwrap().1, "a a a");
/// assert_eq!(lt.match_atom_string(RepeatAtom::new_sep_unbounded('a', ' ', 0))
/// .unwrap().1, "a a a a");
/// assert_eq!(lt.match_atom_string(RepeatAtom::new_sep_unbounded('a', ' ', 2))
/// .unwrap().1, "a a a a");
/// assert!(lt.match_atom_string(RepeatAtom::new_sep_unbounded('a', ' ', 5))
/// .is_err());
/// assert!(lt.match_atom_string(RepeatAtom::new_sep_bounds('a', ' ', 2, 1))
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
    /// Create a new [`RepeatAtom`] with specified separator.
    pub const fn new_sep(atom : T, sep : SEP, min : usize, max : Option<usize>) -> Self {
        Self{
            atom,
            sep,
            min,
            max,
        }
    }
    /// Create a new [`RepeatAtom`] with specified separator and upper bound
    pub const fn new_sep_bounds(atom : T, sep : SEP, min : usize, max : usize) -> Self {
        Self::new_sep(atom, sep, min, Some(max))
    }
    /// Create a new [`RepeatAtom`] with specified separator without upper bound
    pub const fn new_sep_unbounded(atom : T, sep : SEP, min : usize) -> Self {
        Self::new_sep(atom, sep, min, None)
    }
    /// Create a new [`RepeatAtom`] that matches at least one occurrence
    pub const fn new_any_one(atom : T, sep : SEP) -> Self {
        Self::new_sep(atom, sep, 1, None)
    }
    /// Wraps it in a [`WithCont`].
    pub const fn wraps<I>(self) -> WithCont<Self, I> {
        WithCont(self, PhantomData)
    }
}

impl<T, SEP> RepeatAtom<T, SEP> { 
    pub(crate) fn parse_logic<
            E, 
            M : Clone + Chain<T, Error = E> + Chain<SEP, Error = E>, 
            F : FnOnce() -> E, 
            I : Insert<<M as Chain<T>>::Data>
        >(&self, st : M, min_err : F) -> ControlFlow<E, (M, I)>{
        if let Some(max) = self.max && max < self.min {
            return ControlFlow::Break(min_err());
        }
        let mut helper = st;
        let mut start = self.min;
        let mut vec = I::new();
        if self.min > 0 {
            helper = helper.chain_append(&self.atom, &mut vec)?;
            for _ in 1..(self.min) {
                helper = helper.chain_nodata(&self.sep)?.chain_append(&self.atom, &mut vec)?;
            }
        }
        else {
            match helper.clone().chain_append(&self.atom, &mut vec) {
                ControlFlow::Continue(h) => {
                    helper = h;
                    start += 1;
                }
                ControlFlow::Break(_) => return ControlFlow::Continue((helper, vec)),
            }
        }
        if let Some(max) = self.max {
            for _i in start..max {
                if let ControlFlow::Continue(hh) = helper.clone().chain_nodata(&self.sep)
                    && let ControlFlow::Continue(h) = hh.chain_append(&self.atom, &mut vec) {
                    helper = h;
                }
                else {
                    return ControlFlow::Continue((helper, vec));
                }
            }
            return ControlFlow::Continue((helper, vec));
        }
        else {
            loop {
                if let ControlFlow::Continue(hh) = helper.clone().chain_nodata(&self.sep)
                    && let ControlFlow::Continue(h) = hh.chain_append(&self.atom, &mut vec) {
                    helper = h;
                }
                else {
                    return ControlFlow::Continue((helper, vec));
                }
            }
        }
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
    pub const fn new_sep(atom : T, sep : SEP, max : Option<usize>) -> Self {
        Self{
            atom,
            sep,
            max,
        }
    }
    /// Create a new [`RepeatAnyAtom`] with specified separator and upper bound
    pub const fn new_sep_bounds(atom : T, sep : SEP, max : usize) -> Self {
        Self::new_sep(atom, sep, Some(max))
    }
    /// Create a new [`RepeatAnyAtom`] with specified separator without upper bound
    pub const fn new_sep_unbounded(atom : T, sep : SEP) -> Self {
        Self::new_sep(atom, sep, None)
    }
    /// Wraps it in a [`WithCont`].
    pub const fn wraps<I>(self) -> WithCont<Self, I> {
        WithCont(self, PhantomData)
    }
}

impl<T, SEP> RepeatAnyAtom<T, SEP> { 
    pub(crate) fn parse_logic<M : Clone + Chain<T> + Chain<SEP>, I : Insert<<M as Chain<T>>::Data>>(&self, st : M) -> (M, I){
        let mut helper = st;
        let mut vec = I::new();
        match helper.clone().chain_append(&self.atom, &mut vec) {
            ControlFlow::Continue(h) => {
                helper = h;
            }
            ControlFlow::Break(_) => return (helper, vec),
        }
        if let Some(max) = self.max {
            for _i in 1..max {
                if let ControlFlow::Continue(hh) = helper.clone().chain_nodata(&self.sep)
                    && let ControlFlow::Continue(h) = hh.chain_append(&self.atom, &mut vec) {
                    helper = h;
                }
                else {
                    return (helper, vec);
                }
            }
            return (helper, vec);
        }
        else {
            loop {
                if let ControlFlow::Continue(hh) = helper.clone().chain_nodata(&self.sep)
                    && let ControlFlow::Continue(h) = hh.chain_append(&self.atom, &mut vec) {
                    helper = h;
                }
                else {
                    return (helper, vec);
                }
            }
        }
    }
}

/// Tool that matches repetitions lazily.
///
/// It matches the least number of `T` atom (sepatared by `SEP`) which are followed by `TERM` atom.
/// The difference with respect to a [`RepeatAtom`] followed by `TERM` is that here repetitions are
/// evaluated lazily: it interrupts at the first match of `TERM`, whereas `RepeatAtom` evaluates
/// repetitions eagerly and so `TERM` is matched only after the repetition ends.
#[derive(Copy, Clone, Debug)]
pub struct LazyRepeatAtom<T, SEP, TERM>{
    atom : T,
    sep : SEP,
    term : TERM,
    min : usize,
    max : Option<usize>,
}

impl<T, SEP, TERM> LazyRepeatAtom<T, SEP, TERM>{
    /// Create a new `LazyRepeatAtom`.
    ///
    /// # Panics
    /// Panic if `max` is strictly lesser than `min`.
    pub const fn new(atom : T, sep : SEP, term : TERM, min : usize, max : Option<usize>) -> Self {
        if let Some(m) = max {
            assert!(m >= min, "Max is strictly lesser than min");
        }
        Self{
            atom,
            sep,
            term,
            min,
            max,
        }
    }
    /// Create a new `LazyRepeatAtom` with specified upper bound.
    pub const fn new_bounds(atom : T, sep : SEP, term : TERM, min : usize, max : usize) -> Self {
        Self::new(atom, sep, term, min, Some(max))
    }
    /// Create a new `LazyRepeatAtom` without upper bound.
    pub const fn new_unbounded(atom : T, sep : SEP, term : TERM, min : usize) -> Self {
        Self::new(atom, sep, term, min, None)
    }
    /// Wraps it in a [`WithCont`].
    pub const fn wraps<I>(self) -> WithCont<Self, I> {
        WithCont(self, PhantomData)
    }
}

impl<T, SEP, TERM> LazyRepeatAtom<T, SEP, TERM> {
    // Second M is the match without including TERM
    pub(crate) fn parse_logic<E, M : Clone + Chain<T, Error = E> + Chain<SEP, Error = E> + Chain<TERM, Error = E>, F : FnOnce() -> E, I : Insert<<M as Chain<T>>::Data> >(&self, st : M, min_err : F) -> ControlFlow<E, (M, M, I)>{
        let mut helper = st;
        if let Some(max) = self.max && max < self.min {
            return ControlFlow::Break(min_err());
        }
        let mut vec = I::new();
        let mut start = self.min;
        if self.min > 0 {
            helper = helper.chain_append(&self.atom, &mut vec)?;
            for _ in 1..(self.min) {
                helper = helper.chain_nodata(&self.sep)?.chain_append(&self.atom, &mut vec)?;
            }
        }
        else{
            match helper.clone().chain_nodata(&self.term) {
                ControlFlow::Continue(hend) => {
                    return ControlFlow::Continue((hend, helper, vec));
                }
                ControlFlow::Break(e) => {
                    if let ControlFlow::Continue(h) = helper.chain_append(&self.atom, &mut vec) {
                        helper = h;
                        start += 1;
                    }
                    else{
                        return ControlFlow::Break(e);
                    }
                }
            }
        }
        if let Some(max) = self.max {
            for _i in start..max {
                match helper.clone().chain_nodata(&self.term) {
                    ControlFlow::Continue(hend) => {
                        return ControlFlow::Continue((hend, helper, vec));
                    }
                    ControlFlow::Break(e) => {
                        if let ControlFlow::Continue(h1) = helper.chain_nodata(&self.sep)
                        && let ControlFlow::Continue(h) = h1.chain_append(&self.atom, &mut vec) {
                            helper = h;
                        }
                        else{
                            return ControlFlow::Break(e);
                        }
                    }
                }
            }
            let hend = helper.clone().chain_nodata(&self.term)?;
            ControlFlow::Continue((hend, helper, vec))
        }
        else {
            loop {
                loop {
                    match helper.clone().chain_nodata(&self.term) {
                        ControlFlow::Continue(hend) => {
                            return ControlFlow::Continue((hend, helper, vec));
                        }
                        ControlFlow::Break(e) => {
                            if let ControlFlow::Continue(h1) = helper.chain_nodata(&self.sep)
                            && let ControlFlow::Continue(h) = h1.chain_append(&self.atom, &mut vec) {
                                helper = h;
                            }
                            else{
                                return ControlFlow::Break(e);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Wrapper for [`RepeatAtom`], [`RepeatAnyAtom`] and [`LazyRepeatAtom`] that allows you to specify
/// the container in which store retrieved data.
#[derive(Debug, Copy, Clone)]
pub struct WithCont<W, I>(pub W, PhantomData<I>);

impl<W, I> WithCont<W, I> {
    /// Wrapps a [`RepeatAtom`], [`RepeatAnyAtom`] or [`LazyRepeatAtom`].
    pub fn new(wrap : W) -> Self {
        Self(wrap, PhantomData)
    }
}
