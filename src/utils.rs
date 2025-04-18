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
use crate::view::{View, NoMatch};
use crate::tools::{ParseTool, RepeatTool, PredicateTool};

/// Tool that matches the newline characters sequences `\n` and `\r\n`.
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub struct NewlineTool;

impl<'a, F : Clone> ParseTool<'a, F> for NewlineTool {
    fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, NoMatch<F>>{
        st.match_tool(RepeatTool::new_optional('\r'))?.match_tool('\n')
    }
}

/// Tool that matches any sequence of Unicode whitespaces.
#[derive(Debug, Copy, Clone)]
pub struct WhiteTool;

impl<'a, F : Clone> ParseTool<'a, F> for WhiteTool{
    fn parse(&self, st : View<'a, F>) -> Result<View<'a, F>, NoMatch<F>> {
        st.match_tool(RepeatTool::new_unbounded(PredicateTool::new(char::is_whitespace)))
    }
}
