#![no_main]
#![no_std]
use eerie::{eerie_sys::runtime::iree_status_t, runtime::error::RuntimeError};
use eerie::runtime::{
    hal::{BufferMapping, BufferView},
    vm::List,
};
use eerie::eerie_sys::runtime::{self as sys};
extern crate alloc;
use ariel_os::debug::{exit, log::info, println, ExitCode};
use eerie::runtime;
use eerie::runtime::base;
use eerie::runtime::vm::ToRef;
use alloc::format;
mod static_library_loader;

mod ptr_iree_lib_query_fn;

mod dummy_c_symbols;

// mod iree_threading;

// mod utils;

mod workqueue;
use workqueue::print_ops_latency;
// static MODEL_BYTECODE: &[u8] = include_bytes!("../resnet50.vmfb");

#[cfg(feature = "emitc")]
unsafe extern "C" {
    fn module_create(v1: *const sys::iree_vm_instance_t , v2: sys::iree_allocator_t,
        v3: *mut *mut sys::iree_vm_module_t) -> iree_status_t;
}
#[cfg(feature = "emitc")]
unsafe fn module_create_emitc(instance: &eerie::runtime::api::Instance, 
    allocator: &runtime::base::Allocator) -> Result<*mut sys::iree_vm_module_t, RuntimeError> {
        let mut out_vm_module  = core::ptr::null_mut();
        base::Status::from_raw(unsafe { module_create(sys::iree_runtime_instance_vm_instance(instance.ctx), allocator.ctx, 
                                             &mut out_vm_module as *mut *mut sys::iree_vm_module_t)
                            })
                            .to_result()
                            .map_err(RuntimeError::StatusError)?;
        Ok(out_vm_module)
}

use alloc::vec::Vec;
#[cfg(feature = "resnet50")]
fn run_resnet50(vmfb: &[u8], image_bin: &[f32]) -> Vec<f32> {
    use eerie::runtime;
    use eerie::runtime::vm::ToRef;

    let instance = runtime::api::Instance::new(
        &runtime::api::InstanceOptions::new(&mut runtime::hal::DriverRegistry::new())
            .use_all_available_drivers(),
    )
    .unwrap();
    let device = instance
        .try_create_default_device("local-sync")
        .expect("Failed to create device");
    let session = runtime::api::Session::create_with_device(
        &instance,
        &runtime::api::SessionOptions::default(),
        &device,
    )
    .unwrap();
    // info!("vmfb size: {}", vmfb.len());
    unsafe { session.append_module_from_memory(vmfb) }.unwrap();
    let function = session.lookup_function("module.serving_default").unwrap();
    let input_list =
        runtime::vm::DynamicList::<runtime::vm::Ref<runtime::hal::BufferView<f32>>>::new(
            1, &instance,
        )
        .unwrap();
    let input_buffer = runtime::hal::BufferView::<f32>::new(
        &session,
        &[1, 224, 224, 3],
        runtime::hal::EncodingType::DenseRowMajor,
        image_bin,
    )
    .unwrap();
    let input_buffer_ref = input_buffer.to_ref(&instance).unwrap();
    input_list.push_ref(&input_buffer_ref).unwrap();
    let output_list =
        runtime::vm::DynamicList::<runtime::vm::Ref<runtime::hal::BufferView<f32>>>::new(
            1, &instance,
        )
        .unwrap();
    function.invoke(&input_list, &output_list).unwrap();
    let output_buffer_ref = output_list.get_ref(0).unwrap();
    let output_buffer: BufferView<f32> = output_buffer_ref.to_buffer_view(&session);
    let output_mapping = BufferMapping::new(output_buffer).unwrap();
    let out = output_mapping.data().to_vec();
    out
}

#[cfg(all(feature = "simple_mul", feature = "static"))]
fn run_simple_mul(vmfb: &[u8], a: &[f32], b: &[f32]) -> Vec<f32> {
    info!("run_simple_mul: use static library!");

    let instance = runtime::api::Instance::new(
        &runtime::api::InstanceOptions::new(&mut runtime::hal::DriverRegistry::new())
            .use_all_available_drivers(),
    )
    .unwrap();
    let f : sys::iree_hal_executable_library_query_fn_t = Some(ptr_iree_lib_query_fn::iree_lib_query_fn);
    let libraries  = [f];
    let device = static_library_loader::create_local_sync_device_with_static_loader(libraries.as_slice()).unwrap();
    let session = runtime::api::Session::create_with_device(
        &instance,
        &runtime::api::SessionOptions::default(),
        &device,
    )
    .unwrap();
    
    #[cfg(not(feature = "emitc"))]
    {
        // info!("run_simple_mul, vmfb pointer: {:p}", vmfb.as_ptr());
        info!("run_simple_mul, vmfb[0]: {:x}", vmfb[0]);
        unsafe { session.append_module_from_memory(vmfb) }.unwrap();
        info!("run_simple_mul, vmfb module append successful!");
    }

    #[cfg(feature = "emitc")]
    {
        let module = unsafe { module_create_emitc(&instance, &base::Allocator::get_global())}.unwrap();
        unsafe { session.append_module(module.as_mut().unwrap()) }.unwrap();
        info!("run_simple_mul, emitc module append successful!");
    }

    
    let function = session.lookup_function("module.simple_mul").unwrap();
    info!("run_simple_mul, function lookup successful!");
    
    let input_list =
        runtime::vm::DynamicList::<runtime::vm::Ref<runtime::hal::BufferView<f32>>>::new(
            2, &instance,
        )
        .unwrap();
    let a_buf = runtime::hal::BufferView::<f32>::new(
        &session,
        &[4],
        runtime::hal::EncodingType::DenseRowMajor,
        a,
    )
    .unwrap();
    let a_buf_ref = a_buf.to_ref(&instance).unwrap();
    input_list.push_ref(&a_buf_ref).unwrap();

    let b_buf = runtime::hal::BufferView::<f32>::new(
        &session,
        &[4],
        runtime::hal::EncodingType::DenseRowMajor,
        b,
    )
    .unwrap();
    let b_buf_ref = b_buf.to_ref(&instance).unwrap();
    input_list.push_ref(&b_buf_ref).unwrap();
    info!("run_simple_mul, finished input list!");

    let output_list =
        runtime::vm::DynamicList::<runtime::vm::Ref<runtime::hal::BufferView<f32>>>::new(
            1, &instance,
        )
        .unwrap();
    info!("run_simple_mul, ready for function invoke!");
    function.invoke(&input_list, &output_list).unwrap();
    info!("run_simple_mul, function invoke successful!");
    let output_buffer_ref = output_list.get_ref(0).unwrap();
    let output_buffer: BufferView<f32> = output_buffer_ref.to_buffer_view(&session);
    let output_mapping = BufferMapping::new(output_buffer).unwrap();
    let out = output_mapping.data().to_vec();
    out
}

#[cfg(all(feature = "simple_mul", not(feature = "static"), not(feature = "emitc")))]
fn run_simple_mul(vmfb: &[u8], a: &[f32], b: &[f32]) -> Vec<f32> {
    use eerie::runtime;
    use eerie::runtime::vm::ToRef;

    let instance = runtime::api::Instance::new(
        &runtime::api::InstanceOptions::new(&mut runtime::hal::DriverRegistry::new())
            .use_all_available_drivers(),
    )
    .unwrap();
    let device = instance
        .try_create_default_device("local-sync")
        .expect("Failed to create device");
    let session = runtime::api::Session::create_with_device(
        &instance,
        &runtime::api::SessionOptions::default(),
        &device,
    )
    .unwrap();
    // info!("run_simple_mul, vmfb pointer: {:p}", vmfb.as_ptr());
    info!("run_simple_mul, vmfb[0]: {:x}", vmfb[0]);
    unsafe { session.append_module_from_memory(vmfb) }.unwrap();
    info!("run_simple_mul, module append successful!");
    let function = session.lookup_function("module.simple_mul").unwrap();
    info!("run_simple_mul, function lookup successful!");
    
    let input_list =
        runtime::vm::DynamicList::<runtime::vm::Ref<runtime::hal::BufferView<f32>>>::new(
            2, &instance,
        )
        .unwrap();
    let a_buf = runtime::hal::BufferView::<f32>::new(
        &session,
        &[4],
        runtime::hal::EncodingType::DenseRowMajor,
        a,
    )
    .unwrap();
    let a_buf_ref = a_buf.to_ref(&instance).unwrap();
    input_list.push_ref(&a_buf_ref).unwrap();

    let b_buf = runtime::hal::BufferView::<f32>::new(
        &session,
        &[4],
        runtime::hal::EncodingType::DenseRowMajor,
        b,
    )
    .unwrap();
    let b_buf_ref = b_buf.to_ref(&instance).unwrap();
    input_list.push_ref(&b_buf_ref).unwrap();
    info!("run_simple_mul, finished input list!");

    let output_list =
        runtime::vm::DynamicList::<runtime::vm::Ref<runtime::hal::BufferView<f32>>>::new(
            1, &instance,
        )
        .unwrap();
    info!("run_simple_mul, ready for function invoke!");
    function.invoke(&input_list, &output_list).unwrap();
    info!("run_simple_mul, function invoke successful!");
    let output_buffer_ref = output_list.get_ref(0).unwrap();
    let output_buffer: BufferView<f32> = output_buffer_ref.to_buffer_view(&session);
    let output_mapping = BufferMapping::new(output_buffer).unwrap();
    let out = output_mapping.data().to_vec();
    out
}

#[cfg(all(feature = "mnist", feature = "static"))]
fn run_mnist(vmfb: &[u8], a: &[f32]) -> Vec<f32> {
    info!("run_mnist: use static library!");

    let instance = runtime::api::Instance::new(
        &runtime::api::InstanceOptions::new(&mut runtime::hal::DriverRegistry::new())
            .use_all_available_drivers(),
    )
    .unwrap();
    let f : sys::iree_hal_executable_library_query_fn_t = Some(ptr_iree_lib_query_fn::iree_lib_query_fn);
    let libraries  = [f];
    let device = static_library_loader::create_local_sync_device_with_static_loader(libraries.as_slice()).unwrap();
    let session = runtime::api::Session::create_with_device(
        &instance,
        &runtime::api::SessionOptions::default(),
        &device,
    )
    .unwrap();
    info!("run_mnist, session creation successful!");
    
    #[cfg(not(feature = "emitc"))]
    {
        // info!("run_simple_mul, vmfb pointer: {:p}", vmfb.as_ptr());
        info!("run_mnist, vmfb[0]: {:x}", vmfb[0]);
        unsafe { session.append_module_from_memory(vmfb) }.unwrap();
        info!("run_mnist, vmfb module append successful!");
    }

    #[cfg(feature = "emitc")]
    {
        let module = unsafe { module_create_emitc(&instance, &base::Allocator::get_global())}.unwrap();
        unsafe { session.append_module(module.as_mut().unwrap()) }.unwrap();
        info!("run_mnist, emitc module append successful!");
    }

    
    let function = session.lookup_function("module.predict").unwrap();
    info!("run_mnist, function lookup successful!");
    
    let input_list =
        runtime::vm::DynamicList::<runtime::vm::Ref<runtime::hal::BufferView<f32>>>::new(
            1, &instance,
        )
        .unwrap();
    let a_buf = runtime::hal::BufferView::<f32>::new(
        &session,
        &[1,28,28,1],
        runtime::hal::EncodingType::DenseRowMajor,
        a,
    )
    .unwrap();
    let a_buf_ref = a_buf.to_ref(&instance).unwrap();
    input_list.push_ref(&a_buf_ref).unwrap();

    info!("run_mnist, finished input list!");

    let output_list =
        runtime::vm::DynamicList::<runtime::vm::Ref<runtime::hal::BufferView<f32>>>::new(
            1, &instance,
        )
        .unwrap();
    info!("run_mnist, ready for function invoke!");
    function.invoke(&input_list, &output_list).unwrap();
    info!("run_mnist, function invoke successful!");
    let output_buffer_ref = output_list.get_ref(0).unwrap();
    let output_buffer: BufferView<f32> = output_buffer_ref.to_buffer_view(&session);
    let output_mapping = BufferMapping::new(output_buffer).unwrap();
    let out = output_mapping.data().to_vec();
    out
}

#[cfg(all(feature = "lenet5", feature = "static"))]
fn run_lenet5_float(vmfb: &[u8], a: &[f32]) -> Vec<f32> {
    info!("run_lenet5_float: use static library!");

    let instance = runtime::api::Instance::new(
        &runtime::api::InstanceOptions::new(&mut runtime::hal::DriverRegistry::new())
            .use_all_available_drivers(),
    )
    .unwrap();
    let f : sys::iree_hal_executable_library_query_fn_t = Some(ptr_iree_lib_query_fn::iree_lib_query_fn);
    let libraries  = [f];
    let device = static_library_loader::create_local_sync_device_with_static_loader(libraries.as_slice()).unwrap();
    let session = runtime::api::Session::create_with_device(
        &instance,
        &runtime::api::SessionOptions::default(),
        &device,
    )
    .unwrap();
    info!("run_lenet5_float, session creation successful!");
    
    #[cfg(not(feature = "emitc"))]
    {
        // info!("run_simple_mul, vmfb pointer: {:p}", vmfb.as_ptr());
        info!("run_lenet5_float, vmfb[0]: {:x}", vmfb[0]);
        unsafe { session.append_module_from_memory(vmfb) }.unwrap();
        info!("run_lenet5_float, vmfb module append successful!");
    }

    #[cfg(feature = "emitc")]
    {
        let module = unsafe { module_create_emitc(&instance, &base::Allocator::get_global())}.unwrap();
        unsafe { session.append_module(module.as_mut().unwrap()) }.unwrap();
        info!("run_lenet5_float, emitc module append successful!");
    }

    
    let function = session.lookup_function("module.main").unwrap();
    info!("run_lenet5_float, function lookup successful!");
    
    let input_list =
        runtime::vm::DynamicList::<runtime::vm::Ref<runtime::hal::BufferView<f32>>>::new(
            1, &instance,
        )
        .unwrap();
    let a_buf = runtime::hal::BufferView::<f32>::new(
        &session,
        &[1,1,28,28],
        runtime::hal::EncodingType::DenseRowMajor,
        a,
    )
    .unwrap();
    let a_buf_ref = a_buf.to_ref(&instance).unwrap();
    input_list.push_ref(&a_buf_ref).unwrap();

    info!("run_lenet5_float, finished input list!");

    let output_list =
        runtime::vm::DynamicList::<runtime::vm::Ref<runtime::hal::BufferView<f32>>>::new(
            1, &instance,
        )
        .unwrap();
    info!("run_lenet5_float, ready for function invoke!");
    let time_begin_us = time::Instant::now().as_micros();
    function.invoke(&input_list, &output_list).unwrap();
    let time_end_us = time::Instant::now().as_micros();
    info!("LeNet 5 func invoke time: {:?} us", time_end_us - time_begin_us);
    info!("run_lenet5_float, function invoke successful!");
    let output_buffer_ref = output_list.get_ref(0).unwrap();
    let output_buffer: BufferView<f32> = output_buffer_ref.to_buffer_view(&session);
    let output_mapping = BufferMapping::new(output_buffer).unwrap();
    let out = output_mapping.data().to_vec();
    out
}

#[cfg(all(feature = "mbv1_vww_96", feature = "static"))]
fn run_mbv1_vww_96(vmfb: &[u8], a: &[u8]) -> Vec<f32> {
    info!("mbv1_vww_96: use static library!");

    let instance = runtime::api::Instance::new(
        &runtime::api::InstanceOptions::new(&mut runtime::hal::DriverRegistry::new())
            .use_all_available_drivers(),
    )
    .unwrap();
    let f : sys::iree_hal_executable_library_query_fn_t = Some(ptr_iree_lib_query_fn::iree_lib_query_fn);
    let libraries  = [f];
    let device = static_library_loader::create_local_sync_device_with_static_loader(libraries.as_slice()).unwrap();
    let session = runtime::api::Session::create_with_device(
        &instance,
        &runtime::api::SessionOptions::default(),
        &device,
    )
    .unwrap();
    info!("mbv1_vww_96: session creation successful!");
    
    #[cfg(not(feature = "emitc"))]
    {
        // info!("run_simple_mul, vmfb pointer: {:p}", vmfb.as_ptr());
        info!("mbv1_vww_96: vmfb[0]: {:x}", vmfb[0]);
        unsafe { session.append_module_from_memory(vmfb) }.unwrap();
        info!("mbv1_vww_96: vmfb module append successful!");
    }

    #[cfg(feature = "emitc")]
    {
        let module = unsafe { module_create_emitc(&instance, &base::Allocator::get_global())}.unwrap();
        unsafe { session.append_module(module.as_mut().unwrap()) }.unwrap();
        info!("emitc module append successful!");
    }

    
    let function = session.lookup_function("module.main").unwrap();
    info!("function lookup successful!");
    
    let input_list =
        runtime::vm::DynamicList::<runtime::vm::Ref<runtime::hal::BufferView<f32>>>::new(
            1, &instance,
        )
        .unwrap();
    let a_buf = runtime::hal::BufferView::<u8>::new(
        &session,
        &[1,96,96,3],
        runtime::hal::EncodingType::DenseRowMajor,
        a,
    )
    .unwrap();
    let a_buf_ref = a_buf.to_ref(&instance).unwrap();
    input_list.push_ref(&a_buf_ref).unwrap();

    info!("finished input list!");

    let output_list =
        runtime::vm::DynamicList::<runtime::vm::Ref<runtime::hal::BufferView<f32>>>::new(
            1, &instance,
        )
        .unwrap();
    info!("ready for function invoke!");
    let time_begin_us = time::Instant::now().as_micros();
    function.invoke(&input_list, &output_list).unwrap();
    let time_end_us = time::Instant::now().as_micros();
    info!("func invoke time: {:?} us", time_end_us - time_begin_us);
    info!("function invoke successful!");
    let output_buffer_ref = output_list.get_ref(0).unwrap();
    let output_buffer: BufferView<f32> = output_buffer_ref.to_buffer_view(&session);
    let output_mapping = BufferMapping::new(output_buffer).unwrap();
    let out = output_mapping.data().to_vec();
    out
}


#[cfg(all(feature = "mcunet_10fps_vww", feature = "static"))]
fn run_mcunet_10fps_vww(vmfb: &[u8], a: &[u8]) -> Vec<u8> {
    info!("mcunet_10fps_vww: use static library!");

    let instance = runtime::api::Instance::new(
        &runtime::api::InstanceOptions::new(&mut runtime::hal::DriverRegistry::new())
            .use_all_available_drivers(),
    )
    .unwrap();
    let f : sys::iree_hal_executable_library_query_fn_t = Some(ptr_iree_lib_query_fn::iree_lib_query_fn);
    let libraries  = [f];
    let device = static_library_loader::create_local_sync_device_with_static_loader(libraries.as_slice()).unwrap();
    let session = runtime::api::Session::create_with_device(
        &instance,
        &runtime::api::SessionOptions::default(),
        &device,
    )
    .unwrap();
    info!("mcunet_10fps_vww: session creation successful!");
    
    #[cfg(not(feature = "emitc"))]
    {
        // info!("run_simple_mul, vmfb pointer: {:p}", vmfb.as_ptr());
        info!("mcunet_10fps_vww: vmfb[0]: {:x}", vmfb[0]);
        unsafe { session.append_module_from_memory(vmfb) }.unwrap();
        info!("mcunet_10fps_vww: vmfb module append successful!");
    }

    #[cfg(feature = "emitc")]
    {
        let module = unsafe { module_create_emitc(&instance, &base::Allocator::get_global())}.unwrap();
        unsafe { session.append_module(module.as_mut().unwrap()) }.unwrap();
        info!("emitc module append successful!");
    }

    
    let function = session.lookup_function("module.main").unwrap();
    info!("function lookup successful!");
    
    let input_list =
        runtime::vm::DynamicList::<runtime::vm::Ref<runtime::hal::BufferView<u8>>>::new(
            1, &instance,
        )
        .unwrap();
    let a_buf = runtime::hal::BufferView::<u8>::new(
        &session,
        &[1,64,64,3],
        runtime::hal::EncodingType::DenseRowMajor,
        a,
    )
    .unwrap();
    let a_buf_ref = a_buf.to_ref(&instance).unwrap();
    input_list.push_ref(&a_buf_ref).unwrap();

    info!("finished input list!");

    let output_list =
        runtime::vm::DynamicList::<runtime::vm::Ref<runtime::hal::BufferView<u8>>>::new(
            1, &instance,
        )
        .unwrap();
    info!("ready for function invoke!");
    let time_begin_us = time::Instant::now().as_micros();
    function.invoke(&input_list, &output_list).unwrap();
    let time_end_us = time::Instant::now().as_micros();
    info!("func invoke time: {:?} us", time_end_us - time_begin_us);
    info!("function invoke successful!");
    let output_buffer_ref = output_list.get_ref(0).unwrap();
    let output_buffer: BufferView<u8> = output_buffer_ref.to_buffer_view(&session);
    let output_mapping = BufferMapping::new(output_buffer).unwrap();
    let out = output_mapping.data().to_vec();
    out
}




// // !! Flatcc requires aligned with 16 bytes !!
// /// Include a file as a `static` array with custom alignment
// /// Usage: include_aligned!("path/to/file", 16)  => aligned to 16 bytes
macro_rules! include_aligned {
    ($align:expr, $path:literal) => {{
        // Include the file as a fixed-size array
        const BYTES: &[u8; include_bytes!($path).len()] = include_bytes!($path);

        // Aligned wrapper
        #[repr(C, align($align))]
        struct Aligned<const N: usize>([u8; N]);

        // Define static aligned instance
        static DATA: Aligned<{ BYTES.len() }> = Aligned(*BYTES);

        // Return reference to the inner array as a slice
        &DATA.0
    }};
}

// use align_data::{include_aligned, Align64, Align16};
use ariel_os::time;
use ariel_os::{
    cell::StaticCell,
    reexports::embassy_usb,
    usb::UsbDriver,
    asynch::spawner,
    time::{Instant, Timer},
};
use embassy_usb::{
    class::cdc_acm::{CdcAcmClass, State},
    driver::EndpointError,
};
use portable_atomic::{AtomicU64, Ordering};
static sum_latency : AtomicU64 = AtomicU64::new(0);

use ariel_os::{
    cell::StaticCell,
    reexports::embassy_usb,
    usb::UsbDriver,
    asynch::spawner,
    time::{Instant, Timer},
};
use embassy_usb::{
    class::cdc_acm::{CdcAcmClass, State},
    driver::EndpointError,
};
use portable_atomic::{AtomicU64, Ordering};
static sum_latency : AtomicU64 = AtomicU64::new(0);
#[ariel_os::thread(autostart, stacksize = 28384, priority = 1)]
fn main() {
    info!(
        "Hello from main()! Running on a {} board.",
        ariel_os::buildinfo::BOARD
    );

    #[cfg(feature = "resnet50")]
    {
        info!("Selected model: resnet50.");
        let image_bin: [f32; 224*224*3] = [1.0; 224*224*3];
        static MODEL_BYTECODE: &[u8] = include_aligned!(16, "../resnet50.vmfb");
        // info!("main, vmfb pointer: {:p}", MODEL_BYTECODE.as_ptr());
        let output = run_resnet50(&MODEL_BYTECODE, &image_bin);
        output.iter().for_each(|x| info!("output:{}", x));
    }

    

    #[cfg(feature = "simple_mul")]
    {
        info!("Selected model: simple_mul.");
        static  MODEL_BYTECODE: &[u8] = include_aligned!(16, "../simple_mul.vmfb");
        let a = [1.0, 2.0, 3.0, 4.0];
        let b = [1.0, 2.0, 3.0, 4.0];
        // info!("main, vmfb pointer: {:p}", MODEL_BYTECODE.as_ptr());
        // info!("main, vmfb[0]: {:x}", MODEL_BYTECODE[0]);
        let output = run_simple_mul(&MODEL_BYTECODE, &a, &b);
        output.iter().for_each(|x| info!("output:{}", x));

    }

    #[cfg(feature = "mnist")]
    {
        info!("Selected model: mnist.");
        let image_bin: [f32; 28*28*1] = [1.0; 28*28*1];
        static MODEL_BYTECODE: &[u8] = include_aligned!(16, "../mnist.vmfb");
        // info!("main, vmfb pointer: {:p}", MODEL_BYTECODE.as_ptr());
        let output = run_mnist(&MODEL_BYTECODE, &image_bin);
        output.iter().for_each(|x| info!("output:{}", x));
    }

    #[cfg(feature = "lenet5")]
    {
        info!("Selected model: lenet5.");
        let image_bin: [f32; 28*28*1] = [1.0; 28*28*1];

//        #[cfg(feature = "quantized")]
//        {
//            static MODEL_BYTECODE: &[u8] = include_aligned!(16, "../lenet5_quantized.vmfb");
              static MODEL_BYTECODE: [u8; 1] = [0; 1]; 
//        }

//        #[cfg(not(feature = "quantized"))]
//        {
//            static MODEL_BYTECODE: &[u8] = include_aligned!(16, "../lenet5_float.vmfb");
//        }
//        
        // info!("main, vmfb pointer: {:p}", MODEL_BYTECODE.as_ptr());
        let time_begin_us = time::Instant::now().as_micros();
        let output = run_lenet5_float(&MODEL_BYTECODE, &image_bin);
        let time_end_us = time::Instant::now().as_micros();
        output.iter().for_each(|x| info!("output:{}", x));
        info!("LeNet 5 inference time: {:?} us", time_end_us - time_begin_us);
        sum_latency.store(print_ops_latency(), Ordering::Relaxed); 
    }
  
    #[cfg(feature = "mbv1_vww_96")]
    {
        info!("Selected model: mbv1_vww_96.");
        let image_bin: [u8; 96*96*3] = [1; 96*96*3];

        static MODEL_BYTECODE: [u8 ; 1] = [0 ; 1];

        let time_begin_us = time::Instant::now().as_micros();
        let output = run_mbv1_vww_96(&MODEL_BYTECODE, &image_bin);
        let time_end_us = time::Instant::now().as_micros();
        output.iter().for_each(|x| info!("output:{}", x));
        info!("mbv1_vww_96 inference time: {:?} us", time_end_us - time_begin_us);
    }
 
    #[cfg(feature = "mcunet_10fps_vww")]
    {
        info!("Selected model: mcunet_10fps_vww.");
        let image_bin: [u8; 64*64*3] = [1; 64*64*3];

        static MODEL_BYTECODE: [u8 ; 1] = [0 ; 1];

        let time_begin_us = time::Instant::now().as_micros();
        let output = run_mcunet_10fps_vww(&MODEL_BYTECODE, &image_bin);
        let time_end_us = time::Instant::now().as_micros();
        output.iter().for_each(|x| info!("output:{}", x));
        info!("mbv1_vww_96 inference time: {:?} us", time_end_us - time_begin_us);
        sum_latency.store(print_ops_latency(), Ordering::Relaxed); 
    }
   
    print_ops_latency();
    exit(ExitCode::SUCCESS);
}


const MAX_FULL_SPEED_PACKET_SIZE: u8 = 64;

#[ariel_os::config(usb)]
const USB_CONFIG: embassy_usb::Config<'_> = {
    let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some(ariel_os::buildinfo::OS_NAME);
    config.product = Some("USB serial example");
    config.serial_number = Some("12345678");
    config.max_power = 100;
    config.max_packet_size_0 = MAX_FULL_SPEED_PACKET_SIZE;

    // Required for Windows support.
    config.composite_with_iads = true;
    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config
};

#[ariel_os::task(autostart, usb_builder_hook)]
async fn usb_main() {
    info!("Hello USB World!");

    static STATE: StaticCell<State<'_>> = StaticCell::new();

    // Create and inject the USB class on the system USB builder.
    let mut class = USB_BUILDER_HOOK
        .with(|builder| {
            CdcAcmClass::new(
                builder,
                STATE.init_with(State::new),
                MAX_FULL_SPEED_PACKET_SIZE.into(),
            )
        })
        .await;

    // Do stuff with the class!
    loop {
        class.wait_connection().await;
        info!("Connected");
//        let data = "Hallo Connected to Usb ACM!";
//        class.write_packet(data).await?;
        let _ = echo(&mut class).await;
        info!("Disconnected");
    }
}

struct Disconnected {}

impl From<EndpointError> for Disconnected {
    fn from(val: EndpointError) -> Self {
        match val {
            EndpointError::BufferOverflow => panic!("Buffer overflow"),
            EndpointError::Disabled => Disconnected {},
        }
    }
}

async fn echo(class: &mut CdcAcmClass<'static, UsbDriver>) -> Result<(), Disconnected> {
    let mut buf = [0; MAX_FULL_SPEED_PACKET_SIZE as usize];
    loop {
        let n = class.read_packet(&mut buf).await?;
//        let data = &buf[..n];
        let output = format!("sum latency: {}", sum_latency.load(Ordering::Relaxed));  
        let data = output.as_str();
//        info!("data: {}", Hex(data));
        class.write_packet(data.as_bytes()).await?;
       
    }
}
