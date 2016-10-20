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
    let prev_epoch = self.epoch.load(Ordering::SeqCst);
    let tid = self.counter.fetch_add(1, Ordering::SeqCst);
    if tid == self.num_thrs - 1 {
      self.counter.store(0, Ordering::SeqCst);
      self.epoch.fetch_add(1, Ordering::SeqCst);
      true
    } else {
      while self.epoch.load(Ordering::SeqCst) == prev_epoch {
      }
      false
    }
  }
}
