
pub mod bytes;



#[derive(Copy, Clone, Debug)]
pub enum Borrow<'r, R> {
    Owned(R),
    Borrowed(&'r R),
}


impl<'r, R> Borrow<'r, R> {

    #[inline]
    pub fn borrowed<'me: 'r>(self: &'me Self) -> Self {
        Borrow::Borrowed(&self)
    }

    #[inline]
    pub fn try_as_mut(&mut self) -> Option<&mut R> {
        match self {
            Borrow::Owned(r) => Some(r),
            Borrow::Borrowed(_) => None,
        }
    }

}



impl<R> std::ops::Deref for Borrow<'_, R> {
    type Target = R;
    fn deref(&self) -> &Self::Target {
        match self {
            Borrow::Owned(ref r) => r,
            Borrow::Borrowed(r) => r,
        }
    }
}

impl<R> AsRef<R> for Borrow<'_, R> {
    fn as_ref(&self) -> &R {
        &self
    }
}

impl<R> From<R> for Borrow<'_, R> {
    fn from(r: R) -> Self {
        Borrow::Owned(r)
    }
}

impl<'r, R> From<&'r R> for Borrow<'r, R> {
    fn from(r: &'r R) -> Self {
        Borrow::Borrowed(r)
    }
}

impl<R> std::borrow::Borrow<R> for Borrow<'_, R> {
    fn borrow(&self) -> &R {
        &self
    }
}

