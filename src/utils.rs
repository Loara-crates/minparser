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
//! Other useful parsing tools.
use crate::atoms::{Atom, Match};
use crate::atomlist::{TrueAtom, PredicateAtom};
use crate::chains::*;

/// Tool that matches the newline characters sequences `\n` and `\r\n`.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct Newline;

impl Atom for Newline {
    fn parse(&self, st : &str) -> Option<Match>{
        Seq{
            first : RepeatAny::new_bounds('\r', TrueAtom, 1),
            second :'\n'
        }.parse(st)
    }
}

/// Tool that matches any sequence of Unicode whitespaces.
#[derive(Debug, Copy, Clone)]
pub struct WhiteSP;

impl Atom for WhiteSP{
    fn parse(&self, st : &str) -> Option<Match> {
        RepeatAny::new_unbounded(PredicateAtom::new(char::is_whitespace), TrueAtom).parse(st)
    }
}

/// Tool to retrieve a sequence of consecutive (Unicode) letters and numbers, which the first
/// character is not a number
#[derive(Debug, Copy, Clone)]
pub struct Ident;

impl Atom for Ident{
    fn parse(&self, st : &str) -> Option<Match> {
        Seq{
            first : PredicateAtom::new(char::is_alphabetic),
            second : PredicateAtom::new(char::is_alphanumeric)
        }.parse(st)
    }
}

