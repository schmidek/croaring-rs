use std::fmt;
use std::ops::{BitAnd, SubAssign};
use crate::Bitmap;

pub struct LazyBitmap<'a> {
    bitmap: &'a mut Bitmap,
}

impl<'a> LazyBitmap<'a> {
    /// Modifies the bitmap this lazy bitmap is associated with to be the union of the two bitmaps.
    ///
    /// # Arguments
    /// * `other` - The other bitmap to union with.
    /// * `force_bitsets` - Whether to force conversions to bitsets when modifying containers
    #[inline]
    pub fn or_inplace(&mut self, other: &Bitmap, force_bitsets: bool) -> &mut Self {
        unsafe {
            // Because we have a mutable borrow of the bitmap, `other` cannot be == our bitmap,
            // so this is always safe
            ffi::roaring_bitmap_lazy_or_inplace(
                &mut self.bitmap.bitmap,
                &other.bitmap,
                force_bitsets,
            );
        }
        self
    }

    /// Modifies the bitmap this lazy bitmap is associated with to be the xor of the two bitmaps.
    #[inline]
    pub fn xor_inplace(&mut self, other: &Bitmap) -> &mut Self {
        unsafe {
            // Because we have a mutable borrow of the bitmap, `other` cannot be == our bitmap,
            // so this is always safe
            ffi::roaring_bitmap_lazy_xor_inplace(&mut self.bitmap.bitmap, &other.bitmap);
        }
        self
    }
}

impl<'a> std::ops::BitOrAssign<&Bitmap> for LazyBitmap<'a> {
    #[inline]
    fn bitor_assign(&mut self, other: &Bitmap) {
        self.or_inplace(other, false);
    }
}

impl<'a> std::ops::BitXorAssign<&Bitmap> for LazyBitmap<'a> {
    #[inline]
    fn bitxor_assign(&mut self, other: &Bitmap) {
        self.xor_inplace(other);
    }
}

impl Bitmap {
    /// Perform multiple bitwise operations on a bitmap.
    ///
    /// The passed closure will be passed a handle which can be used to perform bitwise operations on the bitmap lazily.
    ///
    /// The result will be equivalent to doing the same operations on this bitmap directly, but because of reduced
    /// bookkeeping in between operations, it should be faster
    ///
    /// # Examples
    ///
    /// ```
    /// use croaring::Bitmap;
    ///
    /// // Perform a series of bitwise operations on a bitmap:
    /// let mut bitmap = Bitmap::of(&[99]);
    /// let bitmaps_to_or = [Bitmap::of(&[1, 2, 5, 10]), Bitmap::of(&[1, 30, 100])];
    /// let bitmaps_to_xor = [Bitmap::of(&[5]), Bitmap::of(&[1, 1000, 1001])];
    ///
    /// bitmap.lazy_batch(|lazy| {
    ///     for b in &bitmaps_to_or {
    ///         *lazy |= b;
    ///     }
    ///     for b in &bitmaps_to_xor {
    ///         *lazy ^= b;
    ///     }
    /// });
    /// let mut bitmap2 = Bitmap::of(&[99]);
    /// for b in &bitmaps_to_or {
    ///     bitmap2 |= b;
    /// }
    /// for b in &bitmaps_to_xor {
    ///     bitmap2 ^= b;
    /// }
    /// assert_eq!(bitmap, bitmap2);
    /// assert_eq!(bitmap.to_vec(), [2, 10, 30, 99, 100, 1000, 1001]);
    /// ```
    ///
    /// The result the passed closure is returned from `lazy_batch`
    ///
    /// ```
    /// use croaring::Bitmap;
    ///
    /// let mut bitmap = Bitmap::create();
    /// let bitmaps_to_or = [Bitmap::of(&[1, 2, 5, 10]), Bitmap::of(&[1, 30, 100])];
    /// let total_added = bitmap.lazy_batch(|lazy| {
    ///     let mut total = 0;
    ///     for b in &bitmaps_to_or {
    ///         lazy.or_inplace(b, true);
    ///         total += b.cardinality();
    ///     }
    ///     total
    /// });
    /// assert_eq!(total_added, 7);
    pub fn lazy_batch<F, O>(&mut self, f: F) -> O
    where
        F: FnOnce(&mut LazyBitmap<'_>) -> O,
    {
        let mut lazy_bitmap = LazyBitmap { bitmap: self };
        let result = f(&mut lazy_bitmap);
        unsafe {
            ffi::roaring_bitmap_repair_after_lazy(&mut self.bitmap);
        }
        result
    }

    /// ```
    /// use croaring::Bitmap;
    ///
    /// // Perform a series of bitwise operations on a bitmap:
    /// let mut bitmap = Bitmap::of(&[99]);
    /// let bitmaps_to_or = [Bitmap::of(&[1, 2, 5, 10]), Bitmap::of(&[1, 30, 100])];
    /// let bitmaps_to_sub = [Bitmap::of(&[5]), Bitmap::of(&[1, 1000, 1001])];
    ///
    /// let mut lazy = bitmap.into_lazy();
    /// for b in &bitmaps_to_or {
    ///     lazy |= b;
    /// }
    /// for b in &bitmaps_to_sub {
    ///     lazy -= b;
    /// }
    /// let bitmap = lazy.into_inner();
    ///
    /// let mut bitmap2 = Bitmap::of(&[99]);
    /// for b in &bitmaps_to_or {
    ///     bitmap2 |= b;
    /// }
    /// for b in &bitmaps_to_sub {
    ///     bitmap2 -= b;
    /// }
    /// assert_eq!(bitmap, bitmap2);
    /// assert_eq!(bitmap.to_vec(), [2, 10, 30, 99, 100]);
    /// ```
    pub fn into_lazy(mut self) -> LazyOwnedBitmap {
        unsafe {
            ffi::roaring_bitmap_convert_to_lazy(&mut self.bitmap);
        }
        LazyOwnedBitmap { bitmap: self }
    }
}

#[derive(Clone)]
pub struct LazyOwnedBitmap {
    bitmap: Bitmap,
}

impl LazyOwnedBitmap {

    #[inline]
    pub fn create() -> Self {
        LazyOwnedBitmap {
            bitmap: Bitmap::create()
        }
    }

    /// Modifies the bitmap this lazy bitmap is associated with to be the union of the two bitmaps.
    ///
    /// # Arguments
    /// * `other` - The other bitmap to union with.
    /// * `force_bitsets` - Whether to force conversions to bitsets when modifying containers
    #[inline]
    pub fn or_inplace(&mut self, other: &Bitmap, force_bitsets: bool) -> &mut Self {
        unsafe {
            // Because we have a mutable borrow of the bitmap, `other` cannot be == our bitmap,
            // so this is always safe
            ffi::roaring_bitmap_lazy_or_inplace(
                &mut self.bitmap.bitmap,
                &other.bitmap,
                force_bitsets,
            );
        }
        self
    }

    #[inline]
    pub fn or_inplace_owned(&mut self, other: &mut Bitmap, force_bitsets: bool) -> &mut Self {
        unsafe {
            // Because we have a mutable borrow of the bitmap, `other` cannot be == our bitmap,
            // so this is always safe
            ffi::roaring_bitmap_lazy_or_inplace_owned(
                &mut self.bitmap.bitmap,
                &mut other.bitmap,
                force_bitsets,
            );
        }
        self
    }

    #[inline]
    pub fn add(&mut self, element: u32) {
        unsafe { ffi::roaring_bitmap_lazy_add(&mut self.bitmap.bitmap, element) }
    }

    pub fn into_inner(self) -> Bitmap {
        let mut bitmap = self.bitmap;
        unsafe {
            ffi::roaring_bitmap_repair_after_lazy(&mut bitmap.bitmap);
        }
        bitmap
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        unsafe { ffi::roaring_bitmap_is_empty(&self.bitmap.bitmap) }
    }
}

impl std::ops::BitOrAssign<&Bitmap> for LazyOwnedBitmap {
    #[inline]
    fn bitor_assign(&mut self, other: &Bitmap) {
        self.or_inplace(other, false);
    }
}

impl std::ops::BitOrAssign<Bitmap> for LazyOwnedBitmap {
    #[inline]
    fn bitor_assign(&mut self, mut other: Bitmap) {
        self.or_inplace_owned(&mut other, false);
    }
}

impl std::ops::BitOrAssign<&LazyOwnedBitmap> for LazyOwnedBitmap {
    #[inline]
    fn bitor_assign(&mut self, other: &LazyOwnedBitmap) {
        self.or_inplace(&other.bitmap, false);
    }
}

impl std::ops::BitOrAssign<LazyOwnedBitmap> for LazyOwnedBitmap {
    #[inline]
    fn bitor_assign(&mut self, mut other: LazyOwnedBitmap) {
        self.or_inplace_owned(&mut other.bitmap, false);
    }
}

impl<'a, 'b> BitAnd<&'a LazyOwnedBitmap> for &'b LazyOwnedBitmap {
    type Output = Bitmap;

    #[inline]
    fn bitand(self, other: &'a LazyOwnedBitmap) -> Bitmap {
        unsafe { Bitmap::take_heap(ffi::roaring_bitmap_lazy_and(&self.bitmap.bitmap, &other.bitmap.bitmap)) }
    }
}

impl SubAssign<&Bitmap> for LazyOwnedBitmap {
    #[inline]
    fn sub_assign(&mut self, other: &Bitmap) {
        unsafe { ffi::roaring_bitmap_lazy_andnot_inplace(&mut self.bitmap.bitmap, &other.bitmap) }
    }
}

impl fmt::Debug for LazyOwnedBitmap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let bitmap = self.clone().into_inner();
        bitmap.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use crate::Bitmap;
    use crate::bitmap::LazyOwnedBitmap;

    #[test]
    fn test_lazy() {
        // Perform a series of bitwise operations on a bitmap:
        let bitmap = Bitmap::create();
        let bitmaps_to_or = [Bitmap::of(&[99]), Bitmap::of(&[1, 2, 5, 10]), Bitmap::create(), Bitmap::of(&[1, 30, 100]), Bitmap::of(&[10001, 10002, 10005, 10010]), Bitmap::of(&[10001, 10030, 10100]), Bitmap::from_range(200000..300000)];
        let bitmaps_to_sub = [Bitmap::of(&[5]), Bitmap::of(&[1, 1000, 1001]), Bitmap::of(&[10005]), Bitmap::of(&[10001, 11000, 11001]), Bitmap::from_range(210000..290000)];

        let mut lazy = bitmap.into_lazy();
        for b in &bitmaps_to_or {
             lazy |= b;
        }
        for b in &bitmaps_to_sub {
             lazy -= b;
        }
        let bitmap = lazy.into_inner();

        let mut bitmap2 = Bitmap::of(&[99]);
        for b in &bitmaps_to_or {
             bitmap2 |= b;
        }
        for b in &bitmaps_to_sub {
             bitmap2 -= b;
        }
        assert_eq!(bitmap, bitmap2);
    }

    #[test]
    fn test_lazy_owned() {
        // Perform a series of bitwise operations on a bitmap:
        let bitmap = Bitmap::create();
        let bitmaps_to_or = [Bitmap::of(&[99]), Bitmap::of(&[1, 2, 5, 10]), Bitmap::create(), Bitmap::of(&[1, 30, 100]), Bitmap::of(&[10001, 10002, 10005, 10010]), Bitmap::of(&[10001, 10030, 10100]), Bitmap::from_range(200000..300000)];
        let bitmaps_to_sub = [Bitmap::of(&[5]), Bitmap::of(&[1, 1000, 1001]), Bitmap::of(&[10005]), Bitmap::of(&[10001, 11000, 11001]), Bitmap::from_range(210000..290000)];

        let mut lazy = bitmap.into_lazy();
        for b in bitmaps_to_or.clone().into_iter() {
            lazy |= b;
        }
        for b in &bitmaps_to_sub {
            lazy -= b;
        }
        let bitmap = lazy.into_inner();

        let mut bitmap2 = Bitmap::of(&[99]);
        for b in &bitmaps_to_or {
            bitmap2 |= b;
        }
        for b in &bitmaps_to_sub {
            bitmap2 -= b;
        }
        assert_eq!(bitmap, bitmap2);
    }

    #[test]
    fn test_lazy_and() {
        // Perform a series of bitwise operations on a bitmap:
        let bitmaps1 = [Bitmap::of(&[1, 2, 5, 10]), Bitmap::of(&[1, 30, 100]), Bitmap::of(&[10001, 10002, 10005, 10010]), Bitmap::of(&[10001, 10030, 10100]), Bitmap::from_range(200000..300000)];
        let bitmaps2 = [Bitmap::of(&[5]), Bitmap::of(&[1, 1000, 1001]), Bitmap::of(&[10005]), Bitmap::of(&[10001, 11000, 11001]), Bitmap::from_range(210000..290000)];

        let mut bitmap1l = LazyOwnedBitmap::create();
        for b in &bitmaps1 {
            bitmap1l |= b;
        }
        let mut bitmap2l = LazyOwnedBitmap::create();
        for b in &bitmaps2 {
            bitmap2l |= b;
        }

        let lazy_result = &bitmap1l & &bitmap2l;

        let mut bitmap1 = Bitmap::create();
        for b in &bitmaps1 {
            bitmap1 |= b;
        }
        let mut bitmap2 = Bitmap::create();
        for b in &bitmaps2 {
            bitmap2 |= b;
        }

        let result = &bitmap1 & &bitmap2;
        assert_eq!(lazy_result, result);
    }
}