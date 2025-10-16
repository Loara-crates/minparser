//! Some usefult parsing atoms;
use core::ops::ControlFlow;
use crate::atoms::{Atom, AlwaysAtom, MatchHelper, Match};
use crate::chains::*;

macro_rules! always_parse {
    ($i:ident) => {
        impl Atom for $i {
            fn parse(&self, st : &str) -> Option<Match> { 
                Some(self.parse_always(st))
            }
        }
    }
}

/// Parses only the end of the input
///
/// ```rust
/// use minparser::prelude::*;
/// let st = MatchHelper::from("My data ");
/// st.match_atom("My data ").unwrap().match_atom(EOFTool).unwrap();
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct EOFTool;

impl Atom for EOFTool{
    fn parse(&self, st : &str) -> Option<Match> {
        if st.is_empty() {
            Some(Match{len : 0})
        }
        else{
            None
        }
    }
}

/// Only checks the provided atom, without progressing.
///
/// ```rust
/// use minparser::prelude::*;
///
/// MatchHelper::from("a").match_atom(CheckTool('a')).unwrap()
///     .match_atom(CheckTool::<fn(&char) -> bool>(char::is_ascii)).unwrap();
/// ```
#[derive(Debug, Copy, Clone, Default)]
pub struct CheckTool<T>(pub T);

impl<T> Atom for CheckTool<T> where T : Atom {
    fn parse(&self, st : &str) -> Option<Match> {
        self.0.parse(st).map(|_| Match{len : 0})
    }
}
impl<T> AlwaysAtom for CheckTool<T> where T : AlwaysAtom {
    fn parse_always(&self, _ : &str) -> Match {
        Match{len : 0}
    }
}

impl<F, S> Atom for And<F, S> where F : Atom, S : Atom {
    fn parse(&self, st : &str) -> Option<Match> {
        match self.parse_logic(MatchHelper::from(st)) {
            ControlFlow::Break(()) => None,
            ControlFlow::Continue(v) => Some(v.finalize().1)
        }
    }
}
impl<F, S> AlwaysAtom for And<F, S> where F : AlwaysAtom, S : AlwaysAtom {
    fn parse_always(&self, st : &str) -> Match {
        MatchHelper::from(st)
            .match_always(&self.first)
            .match_always(&self.second)
            .finalize().1
    }
}

impl<F, S> Atom for Or<F, S> where F : Atom, S : Atom {
    fn parse(&self, st : &str) -> Option<Match> {
        match self.parse_logic(MatchHelper::from(st)) {
            ControlFlow::Break(()) => None,
            ControlFlow::Continue(v) => Some(v.finalize().1)
        }
    }
}
impl<F, S> AlwaysAtom for Or<F, S> where F : AlwaysAtom, S : Atom {
    fn parse_always(&self, st : &str) -> Match {
        self.first.parse_always(st)
    }
}


/// Matches only if the provided atom doesn't match.
///
/// ```rust
/// use minparser::prelude::*;
///
/// MatchHelper::from("ab").match_atom(CheckInvTool('c')).unwrap()
///     .match_atom(CheckInvTool('b')).unwrap();
/// ```
#[derive(Debug, Copy, Clone, Default)]
pub struct CheckInvTool<T>(pub T);

impl<T> Atom for CheckInvTool<T> where T : Atom {
    fn parse(&self, st : &str) -> Option<Match>{
        match self.0.parse(st) {
            Some(_) => None,
            None => Some(Match{len : 0}),
        }
    }
}

/// Matches any single character.
///
/// It is exactly the opposite of [`EOFTool`].
#[derive(Debug, Copy, Clone, Default)]
pub struct AnyTool;

impl Atom for AnyTool {
    fn parse(&self, st : &str) -> Option<Match>{
        st.chars().next().map(|c| Match{len : c.len_utf8()})
    }
}

/// Discards empty strings from a match.
///
/// Some atoms that needs to match an undefined number of other atoms (like [`RepeatTool`] or
/// [`LazyRepeatTool`]) may enter in an infinite loop if the inner atom matches an empty string `""`. 
/// In that case indeed there always be a match but the atom does not progress, resulting so in an
/// endless cycle.
///
/// This atom takes another atom and converts any match with an empty string with a missing match,
/// avoiding so the issue.
#[derive(Debug, Clone, Copy, Default)]
pub struct NonEmpty<P>(pub P);

impl<P : Atom> Atom for NonEmpty<P> {
    fn parse(&self, st: &str) -> Option<Match> {
        self.0.parse(st).and_then(|mtc| if mtc.len > 0 {Some(mtc)} else {None})
    }
}

/// Matches the empty string, therefore it always matches.
///
/// ```rust
/// use minparser::prelude::*;
/// MatchHelper::from("My data ").match_atom(TrueAtom).unwrap().match_atom("My data ").unwrap();
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct TrueAtom;

impl AlwaysAtom for TrueAtom{
    fn parse_always(&self, _ : &str) -> Match {
        Match{len : 0}
    }
}
always_parse!(TrueAtom);


// impl<T> RepeatTool<T, TrueAtom>{
//     /// Create a new [`RepeatTool`] without spaces.
//     ///
//     /// # Panics
//     /// Panic if `max` is strictly lesser than `min`.
//     pub const fn new(atom : T, min : usize, max : Option<usize>) -> Self {
//         if let Some(m) = max {
//             assert!(m >= min, "Max is strictly lesser than min");
//         }
//         Self{
//             atom,
//             sep : TrueAtom,
//             min,
//             max,
//         }
//     }
//     /// Create a new [`RepeatTool`] with specified upper bound
//     pub const fn new_bounds(atom : T, min : usize, max : usize) -> Self {
//         Self::new(atom, min, Some(max))
//     }
//     /// Create a new [`RepeatTool`] without upper bound
//     pub const fn new_unbounded(atom : T, min : usize) -> Self {
//         Self::new(atom, min, None)
//     }
// }

impl<'a, T> Chain<T> for MatchHelper<'a> where T : Atom {
    type Error = (); // We do not want to send Self as error
    
    fn chain(self, t : &T) -> ControlFlow<Self::Error, Self> {
        match self.match_atom(t) {
            Ok(s) => ControlFlow::Continue(s),
            Err(_) => ControlFlow::Break(()),
        }
    }
}


/// Repeat atom with the specified limits without a separator
pub const fn repeat_bounds<T>(atom : T, min : usize, max : usize) -> RepeatTool<T, TrueAtom> {
    RepeatTool::new_sep(atom, TrueAtom, min, Some(max))
}
/// Repeat atom with the specified limits without a separator
pub const fn repeat_unbounded<T>(atom : T, min : usize) -> RepeatTool<T, TrueAtom> {
    RepeatTool::new_sep(atom, TrueAtom, min, None)
}

impl<T, SEP> Atom for RepeatTool<T, SEP> where T : Atom, SEP : Atom {
    fn parse(&self, st : &str) -> Option<Match> {
        let h = MatchHelper::from(st);
        match self.parse_logic::<(), _, _>(h, || ()) {
            ControlFlow::Continue(hh) => Some(hh.0.finalize().1),
            ControlFlow::Break(()) => None,
        }
    }
}

/// Repeat atom with the specified limits without a separator
pub const fn repeat_any_bounds<T>(atom : T, max : usize) -> RepeatAnyTool<T, TrueAtom> {
    RepeatAnyTool::new_sep(atom, TrueAtom, Some(max))
}
/// Repeat atom with the specified limits without a separator
pub const fn repeat_any_unbounded<T>(atom : T) -> RepeatAnyTool<T, TrueAtom> {
    RepeatAnyTool::new_sep(atom, TrueAtom, None)
}


impl<T, SEP> AlwaysAtom for RepeatAnyTool<T, SEP> where T : Atom, SEP : Atom {
    fn parse_always(&self, st : &str) -> Match {
        let h = MatchHelper::from(st);
        self.parse_logic(h).0.finalize().1
    }
}
impl<T, SEP> Atom for RepeatAnyTool<T, SEP> where T : Atom, SEP : Atom {
    fn parse(&self, st : &str) -> Option<Match> {
        Some(self.parse_always(st))
    }
}


/// Tool that matches repetitions lazily.
///
/// It matches the least number of `T` atom (sepatared by `SEP`) which are followed by `TERM` atom.
/// The difference with respect to a [`RepeatTool`] followed by `TERM` is that here repetitions are
/// evaluated lazily: it interrupts at the first match of `TERM`, whereas `RepeatTool` evaluates
/// repetitions eagerly and so `TERM` is matched only after the repetition ends.
#[derive(Copy, Clone, Debug)]
pub struct LazyRepeatTool<T, SEP, TERM>{
    atom : T,
    sep : SEP,
    term : TERM,
    min : usize,
    max : Option<usize>,
}

impl<T, SEP, TERM> LazyRepeatTool<T, SEP, TERM>{
    /// Create a new `LazyRepeatTool`.
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
    /// Create a new `LazyRepeatTool` with specified upper bound.
    pub const fn new_bounds(atom : T, sep : SEP, term : TERM, min : usize, max : usize) -> Self {
        Self::new(atom, sep, term, min, Some(max))
    }
    /// Create a new `LazyRepeatTool` without upper bound.
    pub const fn new_unbounded(atom : T, sep : SEP, term : TERM, min : usize) -> Self {
        Self::new(atom, sep, term, min, None)
    }
}

impl<T, SEP, TERM> LazyRepeatTool<T, SEP, TERM> {
    // Second M is the matchwithout including TERM
    pub(crate) fn parse_logic<E, M : Clone + Chain<T, Error = E> + Chain<SEP, Error = E> + Chain<TERM, Error = E>, F : FnOnce() -> E>(&self, st : M, min_err : F) -> ControlFlow<E, (M, M, usize)>{
        let mut helper = st;
        if let Some(max) = self.max && max < self.min {
            return ControlFlow::Break(min_err());
        }
        let mut start = self.min;
        if self.min > 0 {
            helper = helper.chain(&self.atom)?;
            for _ in 1..(self.min) {
                helper = helper.chain(&self.sep)?.chain(&self.atom)?;
            }
        }
        else{
            match helper.clone().chain(&self.term) {
                ControlFlow::Continue(hend) => {
                    return ControlFlow::Continue((hend, helper, 0));
                }
                ControlFlow::Break(e) => {
                    if let ControlFlow::Continue(h) = helper.chain(&self.atom) {
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
            for i in start..max {
                match helper.clone().chain(&self.term) {
                    ControlFlow::Continue(hend) => {
                        return ControlFlow::Continue((hend, helper, i));
                    }
                    ControlFlow::Break(e) => {
                        if let ControlFlow::Continue(h1) = helper.chain(&self.sep)
                        && let ControlFlow::Continue(h) = h1.chain(&self.atom) {
                            helper = h;
                        }
                        else{
                            return ControlFlow::Break(e);
                        }
                    }
                }
            }
            let hend = helper.clone().chain(&self.term)?;
            ControlFlow::Continue((hend, helper, max))
        }
        else {
            loop {
                let mut i = start;
                loop {
                    match helper.clone().chain(&self.term) {
                        ControlFlow::Continue(hend) => {
                            return ControlFlow::Continue((hend, helper, i));
                        }
                        ControlFlow::Break(e) => {
                            if let ControlFlow::Continue(h1) = helper.chain(&self.sep)
                            && let ControlFlow::Continue(h) = h1.chain(&self.atom) {
                                helper = h;
                                i += 1;
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
impl<T, SEP, TERM> Atom for LazyRepeatTool<T, SEP, TERM> where T : Atom, SEP : Atom, TERM : Atom {
    fn parse(&self, st : &str) -> Option<Match> {
        let h = MatchHelper::from(st);
        match self.parse_logic::<(), _, _>(h, || ()) {
            ControlFlow::Continue(hh) => Some(hh.0.finalize().1),
            ControlFlow::Break(()) => None,
        }
    }
}


/// Tool that matches characters which satisfies the provided predicate.
///
/// See also [`PredicateRefTool`].
///
/// ```rust
/// use minparser::prelude::*;
/// let lt = MatchHelper::from("aB1৬");
/// lt.match_atom(PredicateRefTool::new(char::is_ascii_lowercase)).unwrap()
/// .match_atom(PredicateRefTool::new(char::is_ascii_uppercase)).unwrap()
/// .match_atom(PredicateRefTool::new(char::is_ascii_digit)).unwrap()
/// .match_atom(PredicateTool::new(char::is_numeric)).unwrap();
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
    /// Creates an atom that matches zero or more occurrences of characters that satisfy the
    /// specified predicate.
    pub const fn new_zero_or_more(predicate : P) -> RepeatAnyTool<Self, TrueAtom> {
        repeat_any_unbounded(Self::new(predicate))
    }
    /// Creates an atom that matches one or more occurrences of characters that satisfy the
    /// specified predicate.
    pub const fn new_one_or_more(predicate : P) -> RepeatTool<Self, TrueAtom> {
        repeat_unbounded(Self::new(predicate), 1)
    }
    /// Tests if the first character satisfy the predicate, and in the affirmative case the matched
    /// character and its length are returned.
    pub fn parse_char(&self, st : &str) -> Option<(char, usize)> {
        st.chars().next().and_then(|c| {
            if (self.predicate)(c) {
                Some((c, c.len_utf8()))
            }
            else {
                None
            }
        })
    }
}

impl<P : Fn(char) -> bool> Atom for PredicateTool<P>{
    fn parse(&self, st : &str) -> Option<Match>{
        self.parse_char(st).map(|(_, len)| Match{len})
    }
}


/// Tool that matches characters which satisfies the provided ref predicate.
///
/// See also [`PredicateTool`].
///
/// ```rust
/// use minparser::prelude::*;
/// let lt = MatchHelper::from("aB1৬");
/// lt.match_atom(PredicateRefTool::new(char::is_ascii_lowercase)).unwrap()
/// .match_atom(PredicateRefTool::new(char::is_ascii_uppercase)).unwrap()
/// .match_atom(PredicateRefTool::new(char::is_ascii_digit)).unwrap()
/// .match_atom(PredicateTool::new(char::is_numeric)).unwrap();
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
    /// Creates an atom that matches zero or more occurrences of characters that satisfy the
    /// specified predicate.
    pub const fn new_zero_or_more(predicate : P) -> RepeatAnyTool<Self, TrueAtom> {
        repeat_any_unbounded(Self::new(predicate))
    }
    /// Creates an atom that matches one or more occurrences of characters that satisfy the
    /// specified predicate.
    pub const fn new_one_or_more(predicate : P) -> RepeatTool<Self, TrueAtom> {
        repeat_unbounded(Self::new(predicate), 1)
    }
    /// Tests if the first character satisfy the predicate, and in the affirmative case the matched
    /// character and its length are returned.
    pub fn parse_char(&self, st : &str) -> Option<(char, usize)> {
        st.chars().next().and_then(|c| {
            if (self.predicate)(&c) {
                Some((c, c.len_utf8()))
            }
            else {
                None
            }
        })
    }
}

impl<P : Fn(&char) -> bool> Atom for PredicateRefTool<P>{
    fn parse(&self, st : &str) -> Option<Match> {
        self.parse_char(st).map(|(_, len)| Match{len})
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;
    #[test]
    fn atoms() {
        let vw = MatchHelper::from("€à/a req sey");
        vw.clone().match_atom(TrueAtom).unwrap();
        assert!(vw.clone().match_atom(EOFTool).is_err());
        assert_eq!(vw.clone().match_atom_string(&["Zx", "€à/a re"]).unwrap().1, "€à/a re");
        assert_eq!(vw.clone().match_atom_string(Or::new(EOFTool, ["€à", "€"])).unwrap().1, "€à");
        assert!(vw.clone().match_atom(Or::new('r', "€àb")).is_err());
    }
}
