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

#[cfg(any(doc, feature = "alloc"))]
mod allc{
    use crate::view::{View, NoMatch};
    use crate::parser::{ParseTool, RepeatTool};

    /// Tool that matches any newline character
    #[derive(Clone, Copy, Eq, PartialEq, Debug)]
    pub struct NewlineTool;

    impl<'a, F : Clone + core::fmt::Debug> ParseTool<'a, F> for NewlineTool {
        type Error = NoMatch<F>;
        type Data = ();
        fn parse(&self, st : View<'a, (), F>) -> Result<View<'a, Self::Data, F>, Self::Error>{
            st.match_tool(RepeatTool::new_optional('\r')).expect("BUG: unreachable").drop().match_tool('\n').map(View::drop)
        }
    }
}

#[cfg(any(doc, feature = "alloc"))]
pub use self::allc::*;
