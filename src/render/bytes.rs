

pub unsafe trait IntoBytes: 'static {}
pub unsafe trait FromBytes: IntoBytes {}

pub fn of<T: IntoBytes>(t: &T) -> &[u8] {
    let data = t as *const _ as *const u8;
    let size = std::mem::size_of::<T>();
    unsafe {
        std::slice::from_raw_parts(data, size)
    }
}

pub fn of_slice<T: IntoBytes>(ts: &[T]) -> &[u8] {
    let data = ts as *const _ as *const u8;
    let size = std::mem::size_of::<T>() * ts.len();
    unsafe {
        std::slice::from_raw_parts(data, size)
    }
}

pub fn to<T: FromBytes>(b: &[u8]) -> &T {
    assert_eq!(b.len(), std::mem::size_of::<T>());
    unsafe { &*(b.as_ptr() as *const T) }
}

pub fn to_slice<T: FromBytes>(b: &[u8]) -> &[T] {
    assert_eq!(b.len() % std::mem::size_of::<T>(), 0);
    unsafe {
        std::slice::from_raw_parts(
            b.as_ptr() as *const T, b.len() / std::mem::size_of::<T>() 
        )
    }
}


macro_rules! impl_into_bytes {
    () => { };
    ($($t:ty,)+) => {
        $(
            unsafe impl IntoBytes for $t {}
        )+
    }
}

macro_rules! array_impls {
    ($($N:literal)+) => {
        $(
            unsafe impl<T: IntoBytes> IntoBytes for [T; $N] {}
        )+
    }
}

macro_rules! tuple_impls {
    () => { };
    ($t:ident, $($ts:ident,)*) => {
        unsafe impl<$t: IntoBytes, $($ts: IntoBytes,)*> IntoBytes for ($t, $($ts,)*) {}
        tuple_impls!($($ts,)*);
    }
}

array_impls!(
     1  2  3  4  5  6  7  8
     9 10 11 12 13 14 15 16
    17 18 19 20 21 22 23 24
    25 26 27 28 29 30 31 32
);

tuple_impls!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P,);

use nalgebra_glm as glm;

impl_into_bytes!(
    glm::Vec1, glm::Vec2, glm::Vec3, glm::Vec4,
    glm::Mat2, glm::Mat3, glm::Mat4,
    
    imgui::DrawVert,
    
    u8, u16, u32, u64, u128,
    i8, i16, i32, i64, i128,
    f32, f64,
);

