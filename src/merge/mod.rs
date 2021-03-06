//! The merge algorithm. This one can merge unequal slices, allocating an n/2
//! sized temporary slice of the same type. Naturally, it can only merge slices
//! that are themselves already sorted.

#[cfg(test)]
mod tests;

use std::cmp::Ordering;
use std::ptr;
use gallop::{self, gallop_left, gallop_right};

/// Merge implementation switch.
///
/// `c(a, b)` should return std::cmp::Ordering::Greater when `a` is greater than `b`.
pub fn merge<T, C: Fn(&T, &T) -> Ordering>(list: &mut [T], mut first_len: usize, c: C) {
    let mut second_len: usize;
    let mut first_off: usize;
    if first_len == 0 {
        return;
    }
    unsafe {
        let (first, second) = list.split_at_mut(first_len);
        second_len = gallop_left(first.get_unchecked(first_len - 1), second, gallop::Mode::Reverse, &c);
        if second_len == 0 {
            return;
        }
        first_off = gallop_right(second.get_unchecked(0), first, gallop::Mode::Forward, &c);
        first_len -= first_off;
        if first_len == 0 {
            return;
        }
    }
    let nlist = list.split_at_mut(first_off).1.split_at_mut(first_len + second_len).0;
    if first_len > second_len {
        merge_hi(nlist, first_len, second_len, c);
    } else {
        merge_lo(nlist, first_len, c);
    }
}

/// The number of times any one run can win before we try galloping.
/// Change this during testing.
const MIN_GALLOP: usize = 7;

/// Merge implementation used when the first run is smaller than the second.
pub fn merge_lo<T, C: Fn(&T, &T) -> Ordering>(list: &mut [T], first_len: usize, c: C) {
    unsafe {
        let mut state = MergeLo::new(list, first_len, c);
        state.merge();
    }
}

/// Implementation of `merge_lo`. We need to have an object in order to
/// implement panic safety.
struct MergeLo<'a, T: 'a, C: Fn(&T, &T) -> Ordering> {
    list_len: usize,
    first_pos: usize,
    first_len: usize,
    second_pos: usize,
    dest_pos: usize,
    list: &'a mut [T],
    tmp: Vec<T>,
    c: C,
}
impl<'a, T: 'a, C: Fn(&T, &T) -> Ordering> MergeLo<'a, T, C> {
    /// Constructor for a lower merge.
    unsafe fn new(list: &'a mut [T], first_len: usize, c: C) -> Self {
        let mut ret_val = MergeLo{
            list_len:   list.len(),
            first_pos:  0,
            first_len:  first_len,
            second_pos: first_len,
            dest_pos:   0,
            list:       list,
            tmp:        Vec::with_capacity(first_len),
            c:          c,
        };
        // First, move the smallest run into temporary storage, leaving the
        // original contents uninitialized.
        ret_val.tmp.set_len(first_len);
        for i in 0..first_len {
            ptr::copy_nonoverlapping(ret_val.list.get_unchecked(i), ret_val.tmp.get_unchecked_mut(i), 1);
        }
        ret_val
    }
    /// Perform the one-by-one comparison and insertion.
    unsafe fn merge(&mut self) {
        let c = &self.c;
        let mut first_count  = 0;
        let mut second_count = 0;
        while self.second_pos > self.dest_pos && self.second_pos < self.list_len {
            debug_assert!(self.first_pos + (self.second_pos - self.first_len) == self.dest_pos);
            if (second_count | first_count) < MIN_GALLOP {
                // One-at-a-time mode.
                if c(self.tmp.get_unchecked(self.first_pos), self.list.get_unchecked(self.second_pos)) == Ordering::Greater {
                    ptr::copy_nonoverlapping(self.list.get_unchecked(self.second_pos), self.list.get_unchecked_mut(self.dest_pos), 1);
                    self.second_pos += 1;
                    second_count += 1;
                    first_count = 0;
                } else {
                    ptr::copy_nonoverlapping(self.tmp.get_unchecked(self.first_pos), self.list.get_unchecked_mut(self.dest_pos), 1);
                    self.first_pos += 1;
                    first_count += 1;
                    second_count = 0;
                }
                self.dest_pos += 1;
            } else {
                // Galloping mode.
                second_count = gallop_left(self.tmp.get_unchecked(self.first_pos), self.list.split_at(self.second_pos).1, gallop::Mode::Forward, c);
                ptr::copy(self.list.get_unchecked(self.second_pos), self.list.get_unchecked_mut(self.dest_pos), second_count);
                self.dest_pos   += second_count;
                self.second_pos += second_count;
                debug_assert!(self.first_pos + (self.second_pos - self.first_len) == self.dest_pos);
                if self.second_pos > self.dest_pos && self.second_pos < self.list_len {
                    first_count = gallop_right(self.list.get_unchecked(self.second_pos), self.tmp.split_at(self.first_pos).1, gallop::Mode::Forward, c);
                    ptr::copy_nonoverlapping(self.tmp.get_unchecked(self.first_pos), self.list.get_unchecked_mut(self.dest_pos), first_count);
                    self.dest_pos  += first_count;
                    self.first_pos += first_count;
                }
            }
        }
    }
}
impl<'a, T: 'a, C: Fn(&T, &T) -> Ordering> Drop for MergeLo<'a, T, C> {
    /// Copy all remaining items in the temporary storage into the list.
    /// If the comparator panics, the result will not be sorted, but will still
    /// contain no duplicates or uninitialized spots.
    fn drop(&mut self) {
        unsafe {
            // Make sure that the entire tmp storage is consumed. Since there are no uninitialized
            // spaces before dest_pos, and no uninitialized space after first_pos, this will ensure
            // that there are no uninitialized spaces inside the slice after we drop. Thus, the
            // function is safe.
            if self.first_pos < self.first_len {
                ptr::copy_nonoverlapping(self.tmp.get_unchecked(self.first_pos), self.list.get_unchecked_mut(self.dest_pos), self.first_len - self.first_pos);
            }
            // The temporary storage is now full of nothing but uninitialized.
            // We want to deallocate the space, but not call the destructors.
            self.tmp.set_len(0);
        }
    }
}

/// Merge implementation used when the first run is larger than the second.
pub fn merge_hi<T, C: Fn(&T, &T) -> Ordering>(list: &mut [T], first_len: usize, second_len: usize, c: C) {
    unsafe {
        let mut state = MergeHi::new(list, first_len, second_len, c);
        state.merge();
    }
}

/// Implementation of `merge_hi`. We need to have an object in order to
/// implement panic safety.
struct MergeHi<'a, T: 'a, C: Fn(&T, &T) -> Ordering> {
    first_pos: isize,
    second_pos: isize,
    dest_pos: isize,
    list: &'a mut [T],
    tmp: Vec<T>,
    c: C
}

impl<'a, T: 'a, C: Fn(&T, &T) -> Ordering> MergeHi<'a, T, C> {
    /// Constructor for a higher merge.
    unsafe fn new(list: &'a mut [T], first_len: usize, second_len: usize, c: C) -> Self {
        let mut ret_val = MergeHi{
            first_pos:  first_len as isize - 1,
            second_pos: second_len as isize - 1,
            dest_pos:   list.len() as isize - 1,
            list:       list,
            tmp:        Vec::with_capacity(second_len),
            c:          c
        };
        // First, move the smallest run into temporary storage, leaving the
        // original contents uninitialized.
        ret_val.tmp.set_len(second_len);
        for i in 0..second_len {
            ptr::copy_nonoverlapping(ret_val.list.get_unchecked(i + first_len), ret_val.tmp.get_unchecked_mut(i), 1);
        }
        ret_val
    }
    /// Perform the one-by-one comparison and insertion.
    unsafe fn merge(&mut self) {
        let c = &self.c;
        let mut first_count: usize  = 0;
        let mut second_count: usize = 0;
        while self.first_pos < self.dest_pos && self.first_pos >= 0 {
            debug_assert!(self.first_pos + self.second_pos + 1 == self.dest_pos);
            if (second_count | first_count) < MIN_GALLOP {
                // One-at-a-time mode.
                if c(self.tmp.get_unchecked(self.second_pos as usize), self.list.get_unchecked(self.first_pos as usize)) != Ordering::Less {
                    ptr::copy_nonoverlapping(self.tmp.get_unchecked(self.second_pos as usize), self.list.get_unchecked_mut(self.dest_pos as usize), 1);
                    self.second_pos -= 1;
                } else {
                    ptr::copy_nonoverlapping(self.list.get_unchecked(self.first_pos as usize), self.list.get_unchecked_mut(self.dest_pos as usize), 1);
                    self.first_pos -= 1;
                }
                self.dest_pos -= 1;
            } else {
                // Galloping mode.
                first_count = self.first_pos as usize + 1 - gallop_right(self.tmp.get_unchecked(self.second_pos as usize), self.list.split_at(self.first_pos as usize + 1).0, gallop::Mode::Reverse, c);
                copy_backwards(self.list.get_unchecked(self.first_pos as usize), self.list.get_unchecked_mut(self.dest_pos as usize), first_count);
                self.dest_pos  -= first_count as isize;
                self.first_pos -= first_count as isize;
                debug_assert!(self.first_pos + self.second_pos + 1 == self.dest_pos);
                if self.first_pos < self.dest_pos && self.first_pos >= 0 {
                    second_count = self.second_pos as usize + 1 - gallop_left(self.list.get_unchecked(self.first_pos as usize), self.tmp.split_at(self.second_pos as usize + 1).0, gallop::Mode::Reverse, c);
                    copy_nonoverlapping_backwards(self.tmp.get_unchecked(self.second_pos as usize), self.list.get_unchecked_mut(self.dest_pos as usize), second_count);
                    self.dest_pos   -= second_count as isize;
                    self.second_pos -= second_count as isize;
                }
            }
        }
    }
}

/// Perform a backwards `ptr::copy_nonoverlapping`. Behave identically when size = 1, but behave
/// differently all other times
unsafe fn copy_backwards<T>(src: *const T, dest: *mut T, size: usize) {
    ptr::copy(src.offset(-(size as isize - 1)), dest.offset(-(size as isize - 1)), size)
}

/// Perform a backwards `ptr::copy_nonoverlapping`. Behave identically when size = 1, but behave
/// differently all other times
unsafe fn copy_nonoverlapping_backwards<T>(src: *const T, dest: *mut T, size: usize) {
    ptr::copy_nonoverlapping(src.offset(-(size as isize - 1)), dest.offset(-(size as isize - 1)), size)
}

impl<'a, T: 'a, C: Fn(&T, &T) -> Ordering> Drop for MergeHi<'a, T, C> {
    /// Copy all remaining items in the temporary storage into the list.
    /// If the comparator panics, the result will not be sorted, but will still
    /// contain no duplicates or uninitialized spots.
    fn drop(&mut self) {
        unsafe {
            // Make sure that the entire tmp storage is consumed. Since there are no uninitialized
            // spaces before dest_pos, and no uninitialized space after first_pos, this will ensure
            // that there are no uninitialized spaces inside the slice after we drop. Thus, the
            // function is safe.
            if self.second_pos >= 0 {
                copy_nonoverlapping_backwards(self.tmp.get_unchecked(self.second_pos as usize), self.list.get_unchecked_mut(self.dest_pos as usize), self.second_pos as usize + 1);
            }

            // The temporary storage is now full of nothing but uninitialized.
            // We want to deallocate the space, but not call the destructors.
            self.tmp.set_len(0);
        }
    }
}

