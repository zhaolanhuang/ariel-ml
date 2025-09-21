use eerie::eerie_sys::runtime::{self as sys};
use eerie::runtime::error::RuntimeError;
use eerie::runtime::base;
use eerie::runtime::hal;
use eerie::runtime::base::StringView;
use core::marker::PhantomData;

// pub type iree_hal_executable_library_query_fn_t = extern "C" fn(max_version: iree_hal_executable, output: *mut Foo);

#[cfg(feature = "static")]
pub fn create_device_with_static_loader(libraries: &[sys::iree_hal_executable_library_query_fn_t]) 
                                        -> Result<hal::Device, RuntimeError> {

        let mut params : sys::iree_hal_sync_device_params_t = Default::default();
        let ptr_params: *mut sys::iree_hal_sync_device_params_t = &mut params;
        unsafe {sys::iree_hal_sync_device_params_initialize(ptr_params); };
        
        let null_import_provider: sys::iree_hal_executable_import_provider_t = Default::default();
        let mut out_executable_loader = core::ptr::null_mut();
        let host_allocator = base::Allocator::get_global().ctx;
        base::Status::from_raw(unsafe { sys::iree_hal_static_library_loader_create(libraries.len(), 
                                libraries.as_ptr(), 
                                null_import_provider, host_allocator, &mut out_executable_loader as *mut *mut sys::iree_hal_executable_loader_t)
                            })
                            .to_result()
                            .map_err(RuntimeError::StatusError)?;
        let identifier = "local-sync";

        let mut out_allocator  = core::ptr::null_mut();                            
        base::Status::from_raw(unsafe { sys::iree_hal_allocator_create_heap(StringView::from(identifier).ctx, host_allocator, 
                                            host_allocator, &mut out_allocator as *mut *mut sys::iree_hal_allocator_t)
                            })
                            .to_result()
                            .map_err(RuntimeError::StatusError)?;

        let mut out_device  = core::ptr::null_mut();

        base::Status::from_raw(unsafe { sys::iree_hal_sync_device_create(StringView::from(identifier).ctx, ptr_params, 
            1, &mut out_executable_loader as *mut *mut sys::iree_hal_executable_loader_t, 
            out_allocator, host_allocator, &mut out_device as *mut *mut sys::iree_hal_device_t)
                            })
                            .to_result()
                            .map_err(RuntimeError::StatusError)?;

        unsafe {
            sys::iree_hal_allocator_release(out_allocator);
            sys::iree_hal_executable_loader_release(out_executable_loader);
        };

        Ok(hal::Device {
            ctx: out_device,
            marker: PhantomData,
        })
    }