extern crate memmap;

use memmap::{Mmap, MmapOptions};

use std::collections::{Bound};
use std::fs::{File};
use std::marker::{PhantomData};
use std::mem::{align_of, size_of};
use std::ops::{Deref, RangeBounds};
use std::slice::{from_raw_parts};
use std::sync::{Arc};

pub mod sync;

pub struct MemoryMap<T> where T: Copy {
  _fd:  Option<File>,
  map:  Mmap,
  _mrk: PhantomData<T>,
}

impl MemoryMap<u8> {
  pub fn open_with_offset(file: File, offset: usize, length: usize) -> Result<MemoryMap<u8>, ()> {
    let map = match unsafe { MmapOptions::new()
      .offset(offset as u64)
      .len(length)
      .map(&file)
    } {
      Err(_) => return Err(()),
      Ok(m) => m,
    };
    Ok(MemoryMap{
      _fd:  Some(file),
      map:  map,
      _mrk: PhantomData,
    })
  }
}

impl<T> AsRef<[T]> for MemoryMap<T> where T: Copy {
  fn as_ref(&self) -> &[T] {
    let raw_s: &[u8] = self.map.as_ref();
    assert_eq!(0, raw_s.as_ptr().align_offset(align_of::<T>()));
    assert_eq!(0, raw_s.len() % size_of::<T>());
    unsafe { from_raw_parts(raw_s.as_ptr() as *const T, raw_s.len() / size_of::<T>()) }
  }
}

impl<T> Deref for MemoryMap<T> where T: Copy {
  type Target = [T];

  fn deref(&self) -> &[T] {
    self.as_ref()
  }
}

#[derive(Clone)]
pub struct SharedSlice<T> {
  ptr:  *const T,
  len:  usize,
  buf:  Arc<dyn Deref<Target=[T]> + Send + Sync>,
}

// XXX(20161129): Following is necessary because of the `*const T` field.
unsafe impl<T> Send for SharedSlice<T> {}
unsafe impl<T> Sync for SharedSlice<T> {}

impl<T> AsRef<[T]> for SharedSlice<T> {
  fn as_ref(&self) -> &[T] {
    unsafe { from_raw_parts(self.ptr, self.len) }
  }
}

impl<T> Deref for SharedSlice<T> {
  type Target = [T];

  fn deref(&self) -> &[T] {
    self.as_ref()
  }
}

impl<T> SharedSlice<T> {
  pub fn new<Buf>(buf: Buf) -> SharedSlice<T> where Buf: 'static + Deref<Target=[T]> + Send + Sync {
    let buf: Arc<dyn Deref<Target=[T]> + Send + Sync> = Arc::new(buf);
    let (ptr, len) = {
      let slice: &[T] = &*buf;
      let ptr = slice.as_ptr();
      let len = slice.len();
      assert_eq!(0, ptr.align_offset(align_of::<T>()));
      (ptr, len)
    };
    SharedSlice{ptr: ptr, len: len, buf: buf}
  }

  pub fn shared_slice<R>(&self, range: R) -> SharedSlice<T> where R: RangeBounds<usize> {
    let start = match range.start_bound() {
      Bound::Included(&idx) => idx,
      Bound::Excluded(&idx) => idx + 1,
      Bound::Unbounded      => 0,
    };
    let end = match range.end_bound() {
      Bound::Included(&idx) => idx + 1,
      Bound::Excluded(&idx) => idx,
      Bound::Unbounded      => self.len,
    };
    assert!(start <= self.len);
    assert!(end <= self.len);
    assert!(start <= end);
    SharedSlice{
      ptr:  unsafe { self.ptr.offset(start as isize) },
      len:  end - start,
      buf:  self.buf.clone(),
    }
  }
}
