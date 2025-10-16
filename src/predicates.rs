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
//! Simple predicates for characters.
//! 
//! This module contains some useful functions in order to analize ASCII and Unicode characters. 
//!
//! A lot of predicates already available for `char` type are not included here, even if they are
//! quite useful. 

use crate::atoms::{Atom, Match};

macro_rules! make_predicate {
    ($n:ident, $p:ident) => {
        #[doc = concat!("Tests if [`", stringify!($p), "`](char::", stringify!($p), ") is true.")]
        #[derive(Copy, Clone, Debug, Default)]
        pub struct $n;

        impl Atom for $n {
            fn parse(&self, st : &str) -> Option<Match>{
                match st.chars().next() {
                    None => None,
                    Some(c) => {
                        if c.$p() {
                            Some(Match{len : c.len_utf8()})
                        }
                        else{
                            None
                        }
                    }
                }
            }
        }
    }
}

make_predicate!(AsciiTool, is_ascii);
make_predicate!(AlphabeticTool, is_alphabetic);
make_predicate!(AlphanumericTool, is_alphanumeric);
make_predicate!(AsciiAlphabeticTool, is_ascii_alphabetic);
make_predicate!(AsciiAlphanumericTool, is_ascii_alphanumeric);
make_predicate!(AsciiDigitTool, is_ascii_digit);
make_predicate!(NumericTool, is_numeric);
make_predicate!(WhitespaceTool, is_whitespace);


/// Tests if a character is a newline character (U+000A, `\n`). Carriage return (U+000D, `\r`) is
/// not detected by this function because it is usually followed by the newline character.
#[must_use]
pub const fn is_newline(c : char) -> bool {
    c == '\n'
}

/// Unicode lowercase/uppercase letter
#[must_use]
pub const fn is_lu_letter(c : char) -> bool {
    c.is_uppercase() || c.is_lowercase()
}
