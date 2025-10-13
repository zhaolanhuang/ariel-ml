use eerie::eerie_sys::runtime::{self as sys};

unsafe extern "C" {
    #[link_name = env!("IREE_LIB_QUERY_FN_NAME")]
    pub unsafe fn iree_lib_query_fn(
        max_version: sys::iree_hal_executable_library_version_t,
        environment: *const sys::iree_hal_executable_environment_v0_t,
    ) -> *mut *const sys::iree_hal_executable_library_header_t;
}