/// Color stores R, G, B values in that order.
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone)]
pub struct Color(pub(crate) u8, pub(crate) u8, pub(crate) u8);

impl Color {
    #[inline(always)]
    pub fn r(&self) -> u8 {
        self.0
    }

    #[inline(always)]
    pub fn g(&self) -> u8 {
        self.1
    }

    #[inline(always)]
    pub fn b(&self) -> u8 {
        self.2
    }
}
