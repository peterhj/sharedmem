use std::sync::atomic::{AtomicUsize, Ordering};

pub struct SpinBarrier {
  counter:  AtomicUsize,
  epoch:    AtomicUsize,
  num_thrs: usize,
}

impl SpinBarrier {
  pub fn new(num_thrs: usize) -> SpinBarrier {
    SpinBarrier{
      counter:  AtomicUsize::new(0),
      epoch:    AtomicUsize::new(0),
      num_thrs: num_thrs,
    }
  }

  pub fn wait(&self) -> bool {
    let prev_epoch = self.epoch.load(Ordering::Acquire);
    let tid = self.counter.fetch_add(1, Ordering::AcqRel);
    if tid == self.num_thrs - 1 {
      self.counter.store(0, Ordering::Release);
      self.epoch.fetch_add(1, Ordering::AcqRel);
      true
    } else {
      while self.epoch.load(Ordering::Acquire) == prev_epoch {
      }
      false
    }
  }
}
