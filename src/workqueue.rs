#![no_main]
#![no_std]

use ariel_os::debug::log::*;
use ariel_os::thread::sync::Channel;
use ariel_os::thread::sync::Mutex;
use ariel_os::time;

static WORK_QUEUE: Channel<Job> = Channel::new();

use portable_atomic::{AtomicUsize, Ordering};

static JOB_REMAINING: AtomicUsize = AtomicUsize::new(0);

static OP_LATENCY_SUM: AtomicUsize = AtomicUsize::new(0);

pub static mut ops_latency : [u64; 16] = [0;16];

static mut op_idx: usize = 0;

#[unsafe(no_mangle)]
pub extern "C" fn begin_record_op_latency() {
unsafe {
    ops_latency[op_idx] = time::Instant::now().as_micros();
}
    
}

#[unsafe(no_mangle)]
pub extern "C" fn end_record_op_latency() {

unsafe {
    ops_latency[op_idx] = time::Instant::now().as_micros() - ops_latency[op_idx];
    op_idx += 1;
}
}

pub fn print_ops_latency() {

let mut sum_latency: u64 = 0;
unsafe {
    for item in ops_latency.into_iter().enumerate() {
        let (i, x): (usize, u64) = item;
        info!("op latency [{:?}] = {:?}", i, x);
        sum_latency += x;
    }
}
info!("op latency sum [{:?}]", sum_latency);
//    unsafe {ops_latency.iter().for_each(|x| info!("op latency:{}", x));}
}

#[unsafe(no_mangle)]
pub extern "C" fn print_current_workgroup(x: usize, y: usize, z: usize) {
//    info!("Current workgroup ({},{},{})", x, y, z);
}

#[unsafe(no_mangle)]
pub extern "C" fn set_job_num(num: usize) {
//    info!("set job num: {}", num);
    JOB_REMAINING.store(num, Ordering::Relaxed);

}

#[unsafe(no_mangle)]
pub extern "C" fn wait_job_done() {
//    info!("enter into wait job done func.");
    loop {
       let remaining = JOB_REMAINING.load(Ordering::Relaxed);
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

//#[ariel_os::thread(autostart, priority = 1)]
//fn thread1() {
//    worker();
//}

#[derive(Copy, Clone)]
#[repr(C)]
struct Job {
    func: extern "C" fn(usize),
    arg: usize,
}

#[unsafe(no_mangle)]
pub extern "C" fn defer_job(func: extern "C" fn(usize), arg: usize) {
//    info!("deferring job, func: {:?}, arg: {:?}", func, arg);

    WORK_QUEUE.send(&Job {func, arg});
    
//    info!("deferring job done, func: {:?}, arg: {:?}", func, arg);
}

fn worker() {
    let my_id = ariel_os::thread::current_tid().unwrap();
    let core = ariel_os::thread::core_id();
    info!("[{:?}] Runining at [{:?}] ...", my_id, core);
    loop {
//        info!("[{:?}] [{:?}] Waiting for job...", core, my_id);
        let job = WORK_QUEUE.recv();
//        info!("[{:?}] [{:?}] Waiting got job, arg {:?}, executing", core, my_id, job.arg);
       (job.func)(job.arg);
//        info!("[{:?}] [{:?}] Job done.", core, my_id);

        JOB_REMAINING.fetch_sub(1, Ordering::Relaxed);
    }
}
