/// Color stores Red, Green, Blue values in that order.
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct Color(pub(crate) u8, pub(crate) u8, pub(crate) u8);

impl Color {
    /// Get the Red component.
    #[inline(always)]
    pub fn r(&self) -> u8 {
        self.0
    }

    /// Get the Green component.
    #[inline(always)]
    pub fn g(&self) -> u8 {
        self.1
    }

    /// Get the Blue component.
    #[inline(always)]
    pub fn b(&self) -> u8 {
        self.2
    }
}
