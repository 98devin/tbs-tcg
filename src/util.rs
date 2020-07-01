
pub mod bytes;



#[derive(Copy, Clone, Debug)]
pub enum Borrow<'r, R> {
    Owned(R),
    Borrowed(&'r R),
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