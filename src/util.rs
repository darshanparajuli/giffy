use std::convert::From;

/// Color stores Red, Green, Blue values in that order.
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct Color(pub(crate) u8, pub(crate) u8, pub(crate) u8);

impl Color {
    /// Get the Red component.
    #[inline(always)]
    pub fn r(self) -> u8 {
        self.0
    }

    /// Get the Green component.
    #[inline(always)]
    pub fn g(self) -> u8 {
        self.1
    }

    /// Get the Blue component.
    #[inline(always)]
    pub fn b(self) -> u8 {
        self.2
    }
}

impl From<Color> for [u8; 3] {
    fn from(c: Color) -> Self {
        [c.r(), c.g(), c.b()]
    }
}

impl From<&Color> for [u8; 3] {
    fn from(c: &Color) -> Self {
        [c.r(), c.g(), c.b()]
    }
}

impl From<[u8; 3]> for Color {
    fn from(array: [u8; 3]) -> Self {
        Color(array[0], array[1], array[2])
    }
}

impl From<&[u8]> for Color {
    fn from(array: &[u8]) -> Self {
        Color(array[0], array[1], array[2])
    }
}
