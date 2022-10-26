use std::iter::{FromIterator, IntoIterator};
use std::marker::PhantomData;
use std::pin::Pin;

use super::Bitmap;

#[derive(Clone)]
pub struct BitmapIterator<'a> {
    iterator: ffi::roaring_uint32_iterator_s,
    rev_iterator: ffi::roaring_uint32_iterator_s,
    phantom: PhantomData<&'a Bitmap>,
}

unsafe impl Send for BitmapIterator<'_> {}
unsafe impl Sync for BitmapIterator<'_> {}

impl<'a> BitmapIterator<'a> {
    fn new(bitmap: &'a Bitmap) -> Self {
        let mut iterator = std::mem::MaybeUninit::uninit();
        unsafe {
            ffi::roaring_init_iterator(&bitmap.bitmap, iterator.as_mut_ptr());
        }
        let mut rev_iterator = std::mem::MaybeUninit::uninit();
        unsafe {
            ffi::roaring_init_iterator_last(&bitmap.bitmap, rev_iterator.as_mut_ptr());
        }
        BitmapIterator {
            iterator: unsafe { iterator.assume_init() },
            rev_iterator: unsafe { rev_iterator.assume_init() },
            phantom: PhantomData,
        }
    }

    #[inline]
    fn current_value(&self) -> Option<u32> {
        if self.has_value() {
            Some(self.iterator.current_value)
        } else {
            None
        }
    }

    #[inline]
    fn has_value(&self) -> bool {
        self.iterator.has_value
    }

    #[inline]
    fn advance(&mut self) -> bool {
        unsafe { ffi::roaring_advance_uint32_iterator(&mut self.iterator) }
    }

    #[inline]
    fn current_value_back(&self) -> Option<u32> {
        if self.has_value_back() {
            Some(self.rev_iterator.current_value)
        } else {
            None
        }
    }

    #[inline]
    fn has_value_back(&self) -> bool {
        self.rev_iterator.has_value
    }

    #[inline]
    fn advance_back(&mut self) -> bool {
        unsafe { ffi::roaring_previous_uint32_iterator(&mut self.rev_iterator) }
    }

    /// Attempt to read many values from the iterator into `dst`
    ///
    /// Returns the number of items read from the iterator, may be `< dst.len()` iff
    /// the iterator is exhausted.
    ///
    /// This can be much more efficient than repeated iteration.
    ///
    /// # Examples
    ///
    /// ```
    /// use croaring::Bitmap;
    ///
    /// let mut bitmap: Bitmap = Bitmap::create();
    /// bitmap.add_range(0..100);
    /// bitmap.add(222);
    /// bitmap.add(555);
    ///
    /// let mut buf = [0; 100];
    /// let mut iter = bitmap.iter();
    /// assert_eq!(iter.next_many(&mut buf), 100);
    /// // Get the first 100 items, from the original range added
    /// for (i, item) in buf.iter().enumerate() {
    ///     assert_eq!(*item, i as u32);
    /// }
    /// // Calls to next_many() can be interleaved with calls to next()
    /// assert_eq!(iter.next(), Some(222));
    /// assert_eq!(iter.next_many(&mut buf), 1);
    /// assert_eq!(buf[0], 555);
    ///
    /// assert_eq!(iter.next(), None);
    /// assert_eq!(iter.next_many(&mut buf), 0);
    /// ```
    ///
    /// ```
    /// use croaring::Bitmap;
    ///
    /// fn print_by_chunks(bitmap: &Bitmap) {
    ///     let mut buf = [0; 1024];
    ///     let mut iter = bitmap.iter();
    ///     loop {
    ///         let n = iter.next_many(&mut buf);
    ///         if n == 0 {
    ///             break;
    ///         }
    ///         println!("{:?}", &buf[..n]);
    ///     }
    /// }
    ///
    /// # print_by_chunks(&Bitmap::of(&[1, 2, 8, 20, 1000]));
    /// ```
    #[inline]
    pub fn next_many(&mut self, dst: &mut [u32]) -> usize {
        let count: u32 = u32::try_from(dst.len()).unwrap_or(u32::MAX);
        let result = unsafe {
            ffi::roaring_read_uint32_iterator(&mut self.iterator, dst.as_mut_ptr(), count)
        };
        debug_assert!(result <= count);
        result as usize
    }
}

impl<'a> Iterator for BitmapIterator<'a> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        match self.current_value() {
            Some(value) => {
                self.advance();

                Some(value)
            }
            None => None,
        }
    }
}

impl<'a> DoubleEndedIterator for BitmapIterator<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self.current_value_back() {
            Some(value) => {
                self.advance_back();

                Some(value)
            }
            None => None,
        }
    }
}

impl Bitmap {
    /// Returns an iterator over each value stored in the bitmap.
    /// Returned values are ordered in ascending order.
    ///
    /// # Examples
    ///
    /// ```
    /// use croaring::Bitmap;
    ///
    /// let mut bitmap = Bitmap::create();
    /// bitmap.add(4);
    /// bitmap.add(3);
    /// bitmap.add(2);
    /// let mut iterator = bitmap.iter();
    ///
    /// assert_eq!(iterator.next(), Some(2));
    /// assert_eq!(iterator.next(), Some(3));
    /// assert_eq!(iterator.next(), Some(4));
    /// assert_eq!(iterator.next(), None);
    /// ```
    pub fn iter(&self) -> BitmapIterator {
        BitmapIterator::new(self)
    }
}

impl FromIterator<u32> for Bitmap {
    /// Convenience method for creating bitmap from iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use croaring::Bitmap;
    ///
    /// let bitmap: Bitmap = (1..3).collect();
    ///
    /// assert!(!bitmap.is_empty());
    /// assert!(bitmap.contains(1));
    /// assert!(bitmap.contains(2));
    /// assert_eq!(bitmap.cardinality(), 2);
    /// ```
    fn from_iter<I: IntoIterator<Item = u32>>(iter: I) -> Self {
        Bitmap::of(&Vec::from_iter(iter))
    }
}

impl Extend<u32> for Bitmap {
    fn extend<T: IntoIterator<Item=u32>>(&mut self, iter: T) {
        for item in iter {
            self.add(item);
        }
    }
}

#[derive(Clone)]
pub struct BitmapIntoIterator {
    iterator: ffi::roaring_uint32_iterator_s,
    rev_iterator: ffi::roaring_uint32_iterator_s,
    _bitmap: Pin<Box<Bitmap>>,
}

unsafe impl Sync for BitmapIntoIterator {}

impl<'a> BitmapIntoIterator {
    fn new(bitmap: Bitmap) -> Self {
        let bitmap = Box::pin(bitmap);
        let mut iterator = std::mem::MaybeUninit::uninit();
        unsafe {
            ffi::roaring_init_iterator(&bitmap.bitmap, iterator.as_mut_ptr());
        }
        let mut rev_iterator = std::mem::MaybeUninit::uninit();
        unsafe {
            ffi::roaring_init_iterator_last(&bitmap.bitmap, rev_iterator.as_mut_ptr());
        }
        BitmapIntoIterator {
            iterator: unsafe { iterator.assume_init() },
            rev_iterator: unsafe { rev_iterator.assume_init() },
            _bitmap: bitmap,
        }
    }

    /// Attempt to read many values from the iterator into `dst`
    ///
    /// Returns the number of items read from the iterator, may be `< dst.len()` iff
    /// the iterator is exhausted.
    ///
    /// This can be much more efficient than repeated iteration.
    ///
    /// # Examples
    ///
    /// ```
    /// use croaring::Bitmap;
    ///
    /// let mut bitmap: Bitmap = Bitmap::create();
    /// bitmap.add_range(0..100);
    /// bitmap.add(222);
    /// bitmap.add(555);
    ///
    /// let mut buf = [0; 100];
    /// let mut iter = bitmap.iter();
    /// assert_eq!(iter.next_many(&mut buf), 100);
    /// // Get the first 100 items, from the original range added
    /// for (i, item) in buf.iter().enumerate() {
    ///     assert_eq!(*item, i as u32);
    /// }
    /// // Calls to next_many() can be interleaved with calls to next()
    /// assert_eq!(iter.next(), Some(222));
    /// assert_eq!(iter.next_many(&mut buf), 1);
    /// assert_eq!(buf[0], 555);
    ///
    /// assert_eq!(iter.next(), None);
    /// assert_eq!(iter.next_many(&mut buf), 0);
    /// ```
    ///
    /// ```
    /// use croaring::Bitmap;
    ///
    /// fn print_by_chunks(bitmap: &Bitmap) {
    ///     let mut buf = [0; 1024];
    ///     let mut iter = bitmap.iter();
    ///     loop {
    ///         let n = iter.next_many(&mut buf);
    ///         if n == 0 {
    ///             break;
    ///         }
    ///         println!("{:?}", &buf[..n]);
    ///     }
    /// }
    ///
    /// # print_by_chunks(&Bitmap::of(&[1, 2, 8, 20, 1000]));
    /// ```
    #[inline]
    pub fn next_many(&mut self, dst: &mut [u32]) -> usize {
        let count: u32 = u32::try_from(dst.len()).unwrap_or(u32::MAX);
        let result = unsafe {
            ffi::roaring_read_uint32_iterator(&mut self.iterator, dst.as_mut_ptr(), count)
        };
        debug_assert!(result <= count);
        result as usize
    }
}

impl Iterator for BitmapIntoIterator {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        let ret = if self.iterator.has_value {
            let value = self.iterator.current_value;
            unsafe { ffi::roaring_advance_uint32_iterator(&mut self.iterator) };
            Some(value)
        }else{
            None
        };
        ret
    }
}

impl DoubleEndedIterator for BitmapIntoIterator {
    fn next_back(&mut self) -> Option<Self::Item> {
        let ret = if self.rev_iterator.has_value {
            let value = self.rev_iterator.current_value;
            unsafe { ffi::roaring_previous_uint32_iterator(&mut self.rev_iterator) };
            Some(value)
        }else{
            None
        };
        ret
    }
}

impl IntoIterator for Bitmap {
    type Item = u32;
    type IntoIter = BitmapIntoIterator;

    fn into_iter(self) -> Self::IntoIter {
        BitmapIntoIterator::new(self)
    }
}