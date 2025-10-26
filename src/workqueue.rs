#![no_main]
#![no_std]

use ariel_os::debug::log::*;
use ariel_os::thread::sync::Channel;
use ariel_os::thread::sync::Mutex;

static WORK_QUEUE: Channel<Job> = Channel::new();

use portable_atomic::{AtomicUsize, Ordering};

static JOB_REMAINING: AtomicUsize = AtomicUsize::new(0);

#[unsafe(no_mangle)]
pub extern "C" fn print_current_workgroup(x: usize, y: usize, z: usize) {
    info!("Current workgroup ({},{},{})", x, y, z);
}

#[unsafe(no_mangle)]
pub extern "C" fn set_job_num(num: usize) {
    info!("set job num: {}", num);
    JOB_REMAINING.store(num, Ordering::SeqCst);

}

#[unsafe(no_mangle)]
pub extern "C" fn wait_job_done() {
    info!("enter into wait job done func.");
    loop {
       let remaining = JOB_REMAINING.load(Ordering::SeqCst);
        {
        if remaining > 0 {

                ariel_os::thread::yield_same();
        } else {
                return;       
            }
        }
    }
    
}

extern "C" fn some_job(arg: usize) {
    info!("some job, usize is {}", arg);
}

//#[ariel_os::thread(autostart)]
//fn test_thread() {
//    let job = Job {
//        func: some_job,
//        arg: 42,
//    };
//    defer_job(job.func, job.arg);
//}

#[ariel_os::thread(autostart, priority = 1)]
fn thread0() {
    worker();
}

#[ariel_os::thread(autostart, priority = 1)]
fn thread1() {
    worker();
}

#[derive(Copy, Clone)]
#[repr(C)]
struct Job {
    func: extern "C" fn(usize),
    arg: usize,
}

#[unsafe(no_mangle)]
pub extern "C" fn defer_job(func: extern "C" fn(usize), arg: usize) {
    info!("deferring job, func: {:?}, arg: {:?}", func, arg);

    WORK_QUEUE.send(&Job {func, arg});
    
    info!("deferring job done, func: {:?}, arg: {:?}", func, arg);
}

fn worker() {
    let my_id = ariel_os::thread::current_tid().unwrap();
    loop {
        info!("[{:?}] Waiting for job...", my_id);
        let job = WORK_QUEUE.recv();
        info!("[{:?}] Waiting got job, arg {:?}, executing", my_id, job.arg);
       (job.func)(job.arg);
        info!("[{:?}] Job done.", my_id);

        JOB_REMAINING.fetch_sub(1, Ordering::SeqCst);
    }
}
