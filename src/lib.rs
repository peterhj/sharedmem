extern crate memmap;

use memmap::{Mmap, Protection};

use std::cell::{RefCell, Ref, RefMut};
use std::fs::{File};
use std::marker::{PhantomData};
use std::mem::{size_of};
use std::ops::{Deref, DerefMut};
use std::rc::{Rc};
use std::slice::{from_raw_parts, from_raw_parts_mut};
use std::sync::{Arc};

pub struct MemoryMap<T> where T: Copy {
  file: Option<File>,
  map:  Mmap,
  _mrk: PhantomData<T>,
}

impl MemoryMap<u8> {
  pub fn open_with_offset(file: File, offset: usize, length: usize) -> Result<MemoryMap<u8>, ()> {
    let map = match Mmap::open_with_offset(&file, Protection::Read, offset, length) {
      Ok(map) => map,
      Err(e) => panic!("failed to mmap buffer: {:?}", e),
    };
    Ok(MemoryMap{
      file: Some(file),
      map:  map,
      _mrk: PhantomData,
    })
  }
}

impl<T> Deref for MemoryMap<T> where T: Copy {
  type Target = [T];

  fn deref(&self) -> &[T] {
    let raw_s = unsafe { self.map.as_slice() };
    unsafe { from_raw_parts(raw_s.as_ptr() as *const T, raw_s.len() / size_of::<T>()) }
  }
}

pub struct RwMemoryMap<T> where T: Copy {
  file: Option<File>,
  map:  Mmap,
  _mrk: PhantomData<T>,
}

impl RwMemoryMap<u8> {
  pub fn open_with_offset(file: File, offset: usize, length: usize) -> Result<MemoryMap<u8>, ()> {
    unimplemented!();
  }
}

impl<T> RwMemoryMap<T> where T: Copy {
  pub fn open_anon(length: usize) -> Result<RwMemoryMap<T>, ()> {
    unimplemented!();
  }

  pub fn freeze(self) -> MemoryMap<T> {
    unimplemented!();
  }
}

impl<T> Deref for RwMemoryMap<T> where T: Copy {
  type Target = [T];

  fn deref(&self) -> &[T] {
    let raw_s = unsafe { self.map.as_slice() };
    unsafe { from_raw_parts(raw_s.as_ptr() as *const T, raw_s.len() / size_of::<T>()) }
  }
}

impl<T> DerefMut for MemoryMap<T> where T: Copy {
  fn deref_mut(&mut self) -> &mut [T] {
    let mut raw_s = unsafe { self.map.as_mut_slice() };
    unsafe { from_raw_parts_mut(raw_s.as_mut_ptr() as *mut T, raw_s.len() / size_of::<T>()) }
  }
}

#[derive(Clone)]
pub struct RwMem<T> {
  buf:  Rc<RefCell<DerefMut<Target=[T]>>>,
}

impl<T> RwMem<T> {
  pub fn new<Buf>(buf: Buf) -> RwMem<T> where Buf: 'static + DerefMut<Target=[T]> {
    let buf: Rc<RefCell<DerefMut<Target=[T]>>> = Rc::new(RefCell::new(buf));
    RwMem{buf: buf}
  }

  pub fn borrow(&self) -> Ref<[T]> {
    Ref::map(self.buf.borrow(), |s| &s[ .. ])
  }

  pub fn borrow_mut(&self) -> RefMut<[T]> {
    RefMut::map(self.buf.borrow_mut(), |s| &mut s[ .. ])
  }

  pub fn as_slice(&self) -> RwSlice<T> {
    let s: &[T] = &**self.buf.borrow();
    RwSlice{
      //ptr:  s.as_ptr(),
      //len:  s.len(),
      from: 0,
      to:   s.len(),
      buf:  self.buf.clone(),
    }
  }
}

#[derive(Clone)]
pub struct RwSlice<T> {
  from: usize,
  to:   usize,
  buf:  Rc<RefCell<DerefMut<Target=[T]>>>,
}

impl<T> RwSlice<T> {
  pub fn borrow<'a>(&'a self) -> Ref<'a, [T]> {
    Ref::map(self.buf.borrow(), |s| &s[self.from .. self.to])
  }

  pub fn borrow_mut(&self) -> RefMut<[T]> {
    RefMut::map(self.buf.borrow_mut(), |s| &mut s[self.from .. self.to])
  }
}

#[derive(Clone)]
pub struct SharedMem<T> {
  buf:  Arc<Deref<Target=[T]>>,
}

impl<T> SharedMem<T> {
  pub fn new<Buf>(buf: Buf) -> SharedMem<T> where Buf: 'static + Deref<Target=[T]> {
    let buf: Arc<Deref<Target=[T]>> = Arc::new(buf);
    SharedMem{buf: buf}
  }

  pub fn as_slice(&self) -> SharedSlice<T> {
    let s: &[T] = &**self.buf;
    SharedSlice{
      ptr:  s.as_ptr(),
      len:  s.len(),
      buf:  self.buf.clone(),
    }
  }

  pub fn slice(&self, from_idx: usize, to_idx: usize) -> SharedSlice<T> {
    let s: &[T] = &(**self.buf)[from_idx .. to_idx];
    let s_ptr = s.as_ptr();
    let s_len = s.len();
    SharedSlice{
      ptr:  s_ptr,
      len:  s_len,
      buf:  self.buf.clone(),
    }
  }

  pub fn unsafe_as_slice(&self) -> RacingSlice<T> {
    let new_buf = self.buf.clone();
    let s: &[T] = &**self.buf;
    RacingSlice{
      ptr:  s.as_ptr() as *mut T,
      len:  s.len(),
      buf:  new_buf,
    }
  }

  pub fn unsafe_slice(&self, from_idx: usize, to_idx: usize) -> RacingSlice<T> {
    let s: &[T] = &(**self.buf)[from_idx .. to_idx];
    let s_ptr = s.as_ptr() as *mut T;
    let s_len = s.len();
    RacingSlice{
      ptr:  s_ptr,
      len:  s_len,
      buf:  self.buf.clone(),
    }
  }
}

#[derive(Clone)]
pub struct SharedSlice<T> {
  ptr:  *const T,
  len:  usize,
  buf:  Arc<Deref<Target=[T]>>,
}

impl<T> Deref for SharedSlice<T> {
  type Target = [T];

  fn deref(&self) -> &[T] {
    unsafe { from_raw_parts(self.ptr, self.len) }
  }
}

pub struct RacingSlice<T> {
  ptr:  *mut T,
  len:  usize,
  buf:  Arc<Deref<Target=[T]>>,
}

impl<T> RacingSlice<T> {
  pub fn as_ptr(&self) -> *const T {
    self.ptr
  }

  pub fn as_mut_ptr(&mut self) -> *mut T {
    self.ptr
  }

  pub fn len(&self) -> usize {
    self.len
  }
}

/*pub struct RwSharedMem<T> {
  buf:  Rc<RefCell<Box<DerefMut<Target=[T]>>>>,
}

impl<T> RwSharedMem<T> {
  pub fn new<Buf>(buf: Buf) -> RwSharedMem<T> where Buf: 'static + DerefMut<Target=[T]> {
    let buf: Box<DerefMut<Target=[T]>> = Box::new(buf);
    RwSharedMem{buf: Rc::new(RefCell::new(buf))}
  }

  pub fn as_slice(&self) -> RoSharedSlice<T> {
    let new_buf = self.buf.clone();
    let s: &[T] = &**self.buf.borrow();
    RoSharedSlice{
      ptr:  s.as_ptr(),
      len:  s.len(),
      buf:  new_buf,
    }
  }

  pub fn as_mut_slice(&self) -> MutSharedSlice<T> {
    let new_buf = self.buf.clone();
    let s: &mut [T] = &mut **self.buf.borrow_mut();
    MutSharedSlice{
      ptr:  s.as_mut_ptr(),
      len:  s.len(),
      buf:  new_buf,
    }
  }
}

#[derive(Clone)]
pub struct RoSharedSlice<T> {
  ptr:  *const T,
  len:  usize,
  buf:  Rc<RefCell<Box<DerefMut<Target=[T]>>>>,
}

impl<T> RoSharedSlice<T> {
}

#[derive(Clone)]
pub struct MutSharedSlice<T> {
  ptr:  *mut T,
  len:  usize,
  buf:  Rc<RefCell<Box<DerefMut<Target=[T]>>>>,
}

impl<T> MutSharedSlice<T> {
}

impl<T> Deref for MutSharedSlice<T> {
  type Target = [T];

  fn deref(&self) -> &[T] {
    unsafe { from_raw_parts(self.ptr, self.len) }
  }
}

impl<T> DerefMut for MutSharedSlice<T> {
  fn deref_mut(&mut self) -> &mut [T] {
    unsafe { from_raw_parts_mut(self.ptr, self.len) }
  }
}*/

/*pub struct ConcurrentMem<T> {
  buf:  Arc<Box<Deref<Target=[T]>>>,
}

impl<T> ConcurrentMem<T> {
  pub fn new<Buf>(buf: Buf) -> ConcurrentMem<T> where Buf: 'static + Deref<Target=[T]> {
    let buf: Box<Deref<Target=[T]>> = Box::new(buf);
    ConcurrentMem{buf: Arc::new(buf)}
  }

  pub fn as_slice(&self) -> ConcurrentSlice<T> {
    let new_buf = self.buf.clone();
    let s: &[T] = &**self.buf;
    ConcurrentSlice{
      ptr:  s.as_ptr(),
      len:  s.len(),
      buf:  new_buf,
    }
  }

  pub fn unsafe_as_racing_slice(&self) -> RacingSlice<T> {
    let new_buf = self.buf.clone();
    let s: &[T] = &**self.buf;
    RacingSlice{
      ptr:  s.as_ptr() as *mut T,
      len:  s.len(),
      buf:  new_buf,
    }
  }
}

pub struct ConcurrentSlice<T> {
  ptr:  *const T,
  len:  usize,
  buf:  Arc<Box<Deref<Target=[T]>>>,
}*/

pub fn test() -> SharedMem<u8> {
  let mem: SharedMem<u8> = SharedMem::new(vec![]);
  mem
}
