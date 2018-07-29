#![feature(align_offset)]
#![feature(collections_bound)]
#![feature(collections_range)]

extern crate memmap;

use memmap::{MmapOptions, Mmap};

use std::any::{Any};
use std::cell::{RefCell, Ref, RefMut};
use std::collections::{Bound};
use std::fs::{File};
use std::marker::{PhantomData};
use std::mem::{align_of, size_of};
use std::ops::{Deref, DerefMut, RangeBounds};
use std::rc::{Rc};
use std::slice::{from_raw_parts, from_raw_parts_mut};
use std::sync::{Arc};

pub mod sync;

pub struct MemoryMap<T> where T: Copy {
  _fd:  Option<File>,
  mmap: Mmap,
  _mrk: PhantomData<*const T>,
}

unsafe impl<T> Send for MemoryMap<T> where T: Copy {}
unsafe impl<T> Sync for MemoryMap<T> where T: Copy {}

impl MemoryMap<u8> {
  pub fn open_with_offset(file: File, offset: usize, length: usize) -> Result<MemoryMap<u8>, ()> {
    let mmap = match unsafe { MmapOptions::new().offset(offset).len(length).map(&file) } {
      Ok(mmap) => mmap,
      Err(e) => panic!("failed to mmap buffer: {:?}", e),
    };
    Ok(MemoryMap{
      _fd:  Some(file),
      mmap: mmap,
      _mrk: PhantomData,
    })
  }
}

impl<T> AsRef<[T]> for MemoryMap<T> where T: Copy {
  fn as_ref(&self) -> &[T] {
    let raw_s: &[u8] = self.mmap.as_ref();
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
pub struct SharedMem<T> where T: Copy {
  ptr:  *const T,
  len:  usize,
  buf:  Arc<Any + Send + Sync>,
}

// XXX(20161129): Following is necessary because of the `*const T` field.
unsafe impl<T> Send for SharedMem<T> where T: Copy {}
unsafe impl<T> Sync for SharedMem<T> where T: Copy {}

impl<T> AsRef<[T]> for SharedMem<T> where T: Copy {
  fn as_ref(&self) -> &[T] {
    unsafe { from_raw_parts(self.ptr, self.len) }
  }
}

impl<T> Deref for SharedMem<T> where T: Copy {
  type Target = [T];

  fn deref(&self) -> &[T] {
    self.as_ref()
  }
}

impl SharedMem<u8> {
  pub fn as_typed_slice<T: Copy>(&self) -> SharedMem<T> {
    let num_elems = self.len / size_of::<T>();
    assert_eq!(0, self.len % size_of::<T>());
    assert_eq!(0, self.ptr.align_offset(align_of::<T>()));
    SharedMem{
      ptr: self.ptr as *const T,
      len: num_elems,
      buf: self.buf.clone(),
    }
  }
}

impl<T> SharedMem<T> where T: Copy {
  pub fn from<Buf>(buf: Buf) -> SharedMem<T> where Buf: 'static + Deref<Target=[T]> + Send + Sync {
    let (ptr, len) = {
      let slice: &[T] = &*buf;
      (slice.as_ptr(), slice.len())
    };
    let buf = Arc::new(buf);
    unsafe { SharedMem::from_raw(ptr, len, buf) }
  }

  pub unsafe fn from_raw(ptr: *const T, len: usize, buf: Arc<Any + Send + Sync>) -> SharedMem<T> {
    SharedMem{ptr, len, buf}
  }

  pub fn shared_slice<R>(&self, range: R) -> SharedMem<T> where R: RangeBounds<usize> {
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
    SharedMem{
      ptr:  unsafe { self.ptr.offset(start as isize) },
      len:  end - start,
      buf:  self.buf.clone(),
    }
  }

  pub fn as_slice(&self) -> &[T] {
    self.as_ref()
  }
}
