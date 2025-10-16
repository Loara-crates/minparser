//! Items that can be used both as [`Atom`](crate::atoms::Atom) and as
//! [`ParseTool`](crate::view::ParseTool).
use core::ops::ControlFlow;

// Associate trait that abstract both View and MatchHelper
pub(crate) trait Chain<T> : Sized{
    type Error;

    fn chain(self, t : &T) -> ControlFlow<Self::Error, Self>;
}

/// Matches both the provided atoms, trying `first` before `second`.
pub struct And<F, S>{
    /// The first atom to be checked
    pub first : F,
    /// The second atom to be checked.
    pub second : S,
}

impl<F, S> And<F, S> {
    /// Creates a new `And`.
    pub fn new(first : F, second : S) -> Self {
        Self{first, second}
    }
    pub(crate) fn parse_logic<E, M : Chain<F, Error = E> + Chain<S, Error = E> >(&self, m : M) -> ControlFlow<E, M> {
        match m.chain(&self.first) {
            ControlFlow::Break(e) => ControlFlow::Break(e),
            ControlFlow::Continue(mm) => mm.chain(&self.second),
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
    pub(crate) fn parse_logic<M : Clone + Chain<F> + Chain<S> >(&self, m : M) -> ControlFlow<<M as Chain<S>>::Error, M> {
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
/// assert_eq!(lt.match_atom_string(RepeatTool::new_sep_bounds('a', ' ', 0, 3))
/// .unwrap().1, "a a a");
/// assert_eq!(lt.match_atom_string(RepeatTool::new_sep_bounds('a', ' ', 2, 3))
/// .unwrap().1, "a a a");
/// assert_eq!(lt.match_atom_string(RepeatTool::new_sep_unbounded('a', ' ', 0))
/// .unwrap().1, "a a a a");
/// assert_eq!(lt.match_atom_string(RepeatTool::new_sep_unbounded('a', ' ', 2))
/// .unwrap().1, "a a a a");
/// assert!(lt.match_atom_string(RepeatTool::new_sep_unbounded('a', ' ', 5))
/// .is_err());
/// assert!(lt.match_atom_string(RepeatTool::new_sep_bounds('a', ' ', 2, 1))
/// .is_err());
/// ```
#[derive(Debug, Clone, Copy)]
pub struct RepeatTool<T, SEP>{
    pub(crate) atom : T,
    pub(crate) sep : SEP,
    pub(crate) min : usize,
    pub(crate) max : Option<usize>,
}

impl<T, SEP> RepeatTool<T, SEP>{
    /// Create a new [`RepeatTool`] with specified separator.
    pub const fn new_sep(atom : T, sep : SEP, min : usize, max : Option<usize>) -> Self {
        Self{
            atom,
            sep,
            min,
            max,
        }
    }
    /// Create a new [`RepeatTool`] with specified separator and upper bound
    pub const fn new_sep_bounds(atom : T, sep : SEP, min : usize, max : usize) -> Self {
        Self::new_sep(atom, sep, min, Some(max))
    }
    /// Create a new [`RepeatTool`] with specified separator without upper bound
    pub const fn new_sep_unbounded(atom : T, sep : SEP, min : usize) -> Self {
        Self::new_sep(atom, sep, min, None)
    }
    /// Create a new [`RepeatTool`] that matches at least one occurrence
    pub const fn new_any_one(atom : T, sep : SEP) -> Self {
        Self::new_sep(atom, sep, 1, None)
    }
}

impl<T, SEP> RepeatTool<T, SEP> { 
    pub(crate) fn parse_logic<E, M : Clone + Chain<T, Error = E> + Chain<SEP, Error = E>, F : FnOnce() -> E>
        (&self, st : M, min_err : F) -> ControlFlow<E, (M, usize)>{
        if let Some(max) = self.max && max < self.min {
            return ControlFlow::Break(min_err());
        }
        let mut helper = st;
        let mut start = self.min;
        if self.min > 0 {
            helper = helper.chain(&self.atom)?;
            for _ in 1..(self.min) {
                helper = helper.chain(&self.sep)?.chain(&self.atom)?;
            }
        }
        else {
            match helper.clone().chain(&self.atom) {
                ControlFlow::Continue(h) => {
                    helper = h;
                    start += 1;
                }
                ControlFlow::Break(_) => return ControlFlow::Continue((helper, 0)),
            }
        }
        if let Some(max) = self.max {
            for i in start..max {
                if let ControlFlow::Continue(hh) = helper.clone().chain(&self.sep)
                    && let ControlFlow::Continue(h) = hh.chain(&self.atom) {
                    helper = h;
                }
                else {
                    return ControlFlow::Continue((helper, i));
                }
            }
            return ControlFlow::Continue((helper, max));
        }
        else {
            let mut i = start;
            loop {
                if let ControlFlow::Continue(hh) = helper.clone().chain(&self.sep)
                    && let ControlFlow::Continue(h) = hh.chain(&self.atom) {
                    helper = h;
                    i += 1;
                }
                else {
                    return ControlFlow::Continue((helper, i));
                }
            }
        }
    }
}

/// Tool that matches repetitions with separator.
///
/// Like [`RepeatTool`] but without specifying a minimum number of repetitions. Therefore. it will
/// always match.
#[derive(Debug, Clone, Copy)]
pub struct RepeatAnyTool<T, SEP>{
    pub(crate) atom : T,
    pub(crate) sep : SEP,
    pub(crate) max : Option<usize>,
}

impl<T, SEP> RepeatAnyTool<T, SEP>{
    /// Create a new [`RepeatAnyTool`] with specified separator.
    pub const fn new_sep(atom : T, sep : SEP, max : Option<usize>) -> Self {
        Self{
            atom,
            sep,
            max,
        }
    }
    /// Create a new [`RepeatAnyTool`] with specified separator and upper bound
    pub const fn new_sep_bounds(atom : T, sep : SEP, max : usize) -> Self {
        Self::new_sep(atom, sep, Some(max))
    }
    /// Create a new [`RepeatAnyTool`] with specified separator without upper bound
    pub const fn new_sep_unbounded(atom : T, sep : SEP) -> Self {
        Self::new_sep(atom, sep, None)
    }
}

impl<T, SEP> RepeatAnyTool<T, SEP> { 
    pub(crate) fn parse_logic<M : Clone + Chain<T> + Chain<SEP>>(&self, st : M) -> (M, usize){
        let mut helper = st;
        match helper.clone().chain(&self.atom) {
            ControlFlow::Continue(h) => {
                helper = h;
            }
            ControlFlow::Break(_) => return (helper, 0),
        }
        if let Some(max) = self.max {
            for i in 1..max {
                if let ControlFlow::Continue(hh) = helper.clone().chain(&self.sep)
                    && let ControlFlow::Continue(h) = hh.chain(&self.atom) {
                    helper = h;
                }
                else {
                    return (helper, i);
                }
            }
            return (helper, max);
        }
        else {
            let mut i = 1;
            loop {
                if let ControlFlow::Continue(hh) = helper.clone().chain(&self.sep)
                    && let ControlFlow::Continue(h) = hh.chain(&self.atom) {
                    helper = h;
                    i += 1;
                }
                else {
                    return (helper, i);
                }
            }
        }
    }
}
