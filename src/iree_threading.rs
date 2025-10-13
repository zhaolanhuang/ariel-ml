use eerie::eerie_sys::runtime::{self as sys};
use eerie::runtime::base::StringView;

use core::ffi::c_void;

use ariel_os::debug::{exit, log::info, println, ExitCode};

#[unsafe(no_mangle)]
pub extern "C" fn iree_thread_yield() {
    info!("iree_thread_yield, get phone call!");
    
}

#[unsafe(no_mangle)]
pub extern "C" fn iree_thread_release(thread: *mut sys::iree_thread_t){

    info!("iree_thread_release, get phone call!");
    
}

#[unsafe(no_mangle)]
pub extern "C" fn iree_thread_resume(thread: *mut sys::iree_thread_t) {
    info!("iree_thread_resume, get phone call!");

    
}

#[unsafe(no_mangle)]
pub extern "C" fn iree_thread_create(entry: sys::iree_thread_entry_t, entry_arg: *mut c_void,
                                  params: sys::iree_thread_create_params_t,
                                 allocator: sys::iree_allocator_t,
                                 out_thread: *mut *mut sys::iree_thread_t) -> sys::iree_status_t {
    info!("iree_thread_create, get phone call!");
    return unsafe { sys::iree_status_allocate(sys::iree_status_code_e_IREE_STATUS_UNIMPLEMENTED,
                                    "iree_threading.rs".as_ptr(),
                                    27, StringView::from("iree_thread_create not implemented!").ctx)};
    
}

#[unsafe(no_mangle)]
pub extern "C" fn iree_thread_request_affinity(thread: *mut sys::iree_thread_t, affinity: sys::iree_thread_affinity_t) {
        info!("iree_thread_request_affinity, get phone call!");

}