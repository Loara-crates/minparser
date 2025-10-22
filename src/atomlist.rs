//! Some usefult parsing atoms;
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
/// st.match_atom("My data ").unwrap().match_atom(EOFChar).unwrap();
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct EOFChar;

impl Atom for EOFChar{
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

impl<F, S> Atom for Seq<F, S> where F : Atom, S : Atom {
    fn parse(&self, st : &str) -> Option<Match> {
        match self.parse_logic(MatchHelper::from(st)) {
            Err(()) => None,
            Ok(v) => Some(v.1.finalize().1)
        }
    }
}
impl<F, S> AlwaysAtom for Seq<F, S> where F : AlwaysAtom, S : AlwaysAtom {
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
            Err(()) => None,
            Ok(v) => Some(v.1.finalize().1)
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
/// It is exactly the opposite of [`EOFChar`].
#[derive(Debug, Copy, Clone, Default)]
pub struct AnyChar;

impl Atom for AnyChar {
    fn parse(&self, st : &str) -> Option<Match>{
        st.chars().next().map(|c| Match{len : c.len_utf8()})
    }
}

/// Discards empty strings from a match.
///
/// Some atoms that needs to match an undefined number of other atoms (like [`RepeatAtom`] or
/// [`LazyRepeatAtom`]) may enter in an infinite loop if the inner atom matches an empty string `""`. 
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

impl<'a, T> Chain<T> for MatchHelper<'a> where T : Atom {
    type Error = (); // We do not want to send Self as error
    type Data = (); // We do not want to send Data
    
    fn chain(self, t : &T) -> Result<(Self::Data, Self), Self::Error> {
        match self.match_atom(t) {
            Ok(s) => Ok(((), s)),
            Err(_) => Err(()),
        }
    }
}


/// Repeat atom with the specified limits without a separator
pub fn repeat_bounds<T>(atom : T, min : usize, max : usize) -> RepeatAtom<T, TrueAtom> {
    RepeatAtom::new_bounds(atom, TrueAtom, min, max)
}
/// Repeat atom with the specified limits without a separator
pub const fn repeat_unbounded<T>(atom : T, min : usize) -> RepeatAtom<T, TrueAtom> {
    RepeatAtom::new_unbounded(atom, TrueAtom, min)
}

impl<T, SEP> Atom for RepeatAtom<T, SEP> where T : Atom, SEP : Atom {
    fn parse(&self, st : &str) -> Option<Match> {
        let h = MatchHelper::from(st);
        let mut c = Count::new();
        match self.parse_logic::<(), _, _>(h, &mut c) {
            Ok(hh) => Some(hh.finalize().1),
            Err(_) => None,
        }
    }
}

/// Repeat atom with the specified limits without a separator
pub const fn repeat_any_bounds<T>(atom : T, max : usize) -> RepeatAnyAtom<T, TrueAtom> {
    RepeatAnyAtom::new_bounds(atom, TrueAtom, max)
}
/// Repeat atom with the specified limits without a separator
pub const fn repeat_any_unbounded<T>(atom : T) -> RepeatAnyAtom<T, TrueAtom> {
    RepeatAnyAtom::new_unbounded(atom, TrueAtom)
}


impl<T, SEP> AlwaysAtom for RepeatAnyAtom<T, SEP> where T : Atom, SEP : Atom {
    fn parse_always(&self, st : &str) -> Match {
        let h = MatchHelper::from(st);
        let mut c = Count::new();
        self.parse_logic(h, &mut c).finalize().1
    }
}


impl<T, SEP> Atom for RepeatAnyAtom<T, SEP> where T : Atom, SEP : Atom {
    fn parse(&self, st : &str) -> Option<Match> {
        Some(self.parse_always(st))
    }
}


impl<T, SEP, TERM> Atom for LazyRepeatAtom<T, SEP, TERM> where T : Atom, SEP : Atom, TERM : Atom {
    fn parse(&self, st : &str) -> Option<Match> {
        let h = MatchHelper::from(st);
        let mut c = Count::new();
        match self.parse_logic::<(), _, _>(h, &mut c) {
            Ok(hh) => Some(hh.finalize().1),
            Err(_) => None,
        }
    }
}


/// Tool that matches characters which satisfies the provided predicate.
///
/// See also [`PredicateRefAtom`].
///
/// ```rust
/// use minparser::prelude::*;
/// let lt = MatchHelper::from("aB1৬");
/// lt.match_atom(PredicateRefAtom::new(char::is_ascii_lowercase)).unwrap()
/// .match_atom(PredicateRefAtom::new(char::is_ascii_uppercase)).unwrap()
/// .match_atom(PredicateRefAtom::new(char::is_ascii_digit)).unwrap()
/// .match_atom(PredicateAtom::new(char::is_numeric)).unwrap();
/// ```
#[derive(Debug, Clone, Copy)]
pub struct PredicateAtom<P>{
    pub(crate) predicate : P,
}

impl<P : Fn(char) -> bool> PredicateAtom<P> {
    /// Create a new [`PredicateAtom`] from a predicate.
    pub const fn new(predicate : P) -> Self {
        Self{
            predicate,
        }
    }
    /// Creates an atom that matches zero or more occurrences of characters that satisfy the
    /// specified predicate.
    pub const fn new_zero_or_more(predicate : P) -> RepeatAnyAtom<Self, TrueAtom> {
        repeat_any_unbounded(Self::new(predicate))
    }
    /// Creates an atom that matches one or more occurrences of characters that satisfy the
    /// specified predicate.
    pub const fn new_one_or_more(predicate : P) -> RepeatAtom<Self, TrueAtom> {
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

impl<P : Fn(char) -> bool> Atom for PredicateAtom<P>{
    fn parse(&self, st : &str) -> Option<Match>{
        self.parse_char(st).map(|(_, len)| Match{len})
    }
}


/// Tool that matches characters which satisfies the provided ref predicate.
///
/// See also [`PredicateAtom`].
///
/// ```rust
/// use minparser::prelude::*;
/// let lt = MatchHelper::from("aB1৬");
/// lt.match_atom(PredicateRefAtom::new(char::is_ascii_lowercase)).unwrap()
/// .match_atom(PredicateRefAtom::new(char::is_ascii_uppercase)).unwrap()
/// .match_atom(PredicateRefAtom::new(char::is_ascii_digit)).unwrap()
/// .match_atom(PredicateAtom::new(char::is_numeric)).unwrap();
/// ```
#[derive(Debug, Clone, Copy)]
pub struct PredicateRefAtom<P>{
    pub(crate) predicate : P,
}

impl<P : Fn(&char) -> bool> PredicateRefAtom<P> {
    /// Create a new [`PredicateRefAtom`] from a predicate.
    pub const fn new(predicate : P) -> Self {
        Self{
            predicate,
        }
    }
    /// Creates an atom that matches zero or more occurrences of characters that satisfy the
    /// specified predicate.
    pub const fn new_zero_or_more(predicate : P) -> RepeatAnyAtom<Self, TrueAtom> {
        repeat_any_unbounded(Self::new(predicate))
    }
    /// Creates an atom that matches one or more occurrences of characters that satisfy the
    /// specified predicate.
    pub const fn new_one_or_more(predicate : P) -> RepeatAtom<Self, TrueAtom> {
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

impl<P : Fn(&char) -> bool> Atom for PredicateRefAtom<P>{
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
        assert!(vw.clone().match_atom(EOFChar).is_err());
        assert_eq!(vw.clone().match_atom_string(&["Zx", "€à/a re"]).unwrap().0, "€à/a re");
        assert_eq!(vw.clone().match_atom_string(Or::new(EOFChar, ["€à", "€"])).unwrap().0, "€à");
        assert!(vw.clone().match_atom(Or::new('r', "€àb")).is_err());
    }
}
