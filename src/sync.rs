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

pub struct SpinSignal {
  counter:  AtomicUsize,
  epoch:    AtomicUsize,
  num_thrs: usize,
}

impl SpinSignal {
  pub fn new(num_thrs: usize) -> SpinSignal {
    SpinSignal{
      counter:  AtomicUsize::new(0),
      epoch:    AtomicUsize::new(0),
      num_thrs: num_thrs,
    }
  }

  pub fn signal(&self) {
    let prev_epoch = self.epoch.load(Ordering::SeqCst);
    self.counter.fetch_add(1, Ordering::SeqCst);
    while self.epoch.load(Ordering::SeqCst) == prev_epoch {
    }
  }

  pub fn maybe_wait(&self) -> bool {
    if self.counter.load(Ordering::SeqCst) == 0 {
      return false;
    }
    let prev_epoch = self.epoch.load(Ordering::SeqCst);
    let tid = self.counter.fetch_add(1, Ordering::SeqCst);
    if tid == self.num_thrs - 1 {
      self.counter.store(0, Ordering::SeqCst);
      self.epoch.fetch_add(1, Ordering::SeqCst);
    } else {
      while self.epoch.load(Ordering::SeqCst) == prev_epoch {
      }
    }
    true
  }
}
