//! Module for the [`Parsable`] trait in order to match and parse custom objects.
use crate::view::View;

/// Trait for parsable objects, which can be built from a string.
pub trait Parsable<'a, F> : Sized{
    /// Error type.
    type Error;

    /// Parses the object from a string.
    ///
    /// # Errors
    /// If match doesn't happen then [``Self::Error``] is returned.
    fn parse(st : View<'a, F>) -> Result<(Self, View<'a, F>), Self::Error>;
}
