use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

pub struct ThreadPool<F>
where
    F: FnOnce() + Send + 'static,
{
    queue: Arc<Mutex<VecDeque<F>>>,
    manager: JoinHandle<()>,
    quit: Arc<Mutex<bool>>,
}

impl<F> ThreadPool<F>
where
    F: FnOnce() + Send + 'static,
{
    pub fn new(count: usize) -> Self {
        let count = Arc::new(Mutex::new(count));
        let quit =  Arc::new(Mutex::new(false));
        let queue = Arc::new(Mutex::new(VecDeque::<F>::new()));
        let workers = Arc::new(Mutex::new(Vec::<JoinHandle<()>>::new())); 
        let quit_clone = quit.clone();
        let queue_clone = queue.clone();

        let handle = thread::spawn(move || {
            loop {
                {
                    let mut w = workers.lock().unwrap();
                    let mut q = queue.lock().unwrap();
                    let c = count.lock().unwrap();
                    let mut i = 0usize;
                    while i < w.len() {
                        if w[i].is_finished() {
                            w.swap_remove(i);
                            continue
                        }
                        i += 1;
                    }
                    if q.len() > 0 && w.len() < *c {
                        let free_count = (*c - w.len()).min(q.len());
                        for _ in 0..free_count {
                            let job = q.pop_front().unwrap();
                            let handle = thread::spawn(job);
                            w.push(handle);
                        }
                    }
                    let end = quit.lock().map(|q| *q).unwrap_or(true);
                    if end {
                        loop {
                            while w.len() > 0 {
                                let h = w.pop().unwrap();
                                let _ = h.join();
                            }
                            if q.len() == 0 {
                                break;
                            }
                            for _ in 0..q.len().min(*c) {
                                let job = q.pop_front().unwrap();
                                let handle = thread::spawn(job);
                                w.push(handle);
                            }
                        }
                        break;
                    }
                }
                thread::sleep(std::time::Duration::from_micros(10_000));
            }
        });
        let this = Self {
            queue: queue_clone,
            manager: handle,
            quit: quit_clone,
        };
        return this;
    }

    pub fn join(self) {
        {
            let mut quit = self.quit.lock().unwrap();
            *quit = true;
        }
        let _ = self.manager.join();
    }

    pub fn execute(&mut self, f: F) {
        let guard = self.queue.lock();
        guard
            .map(|mut q| q.push_back(f))
            .expect("something went wrong while accessing ThreadPool queue");
        return;
    }
}
