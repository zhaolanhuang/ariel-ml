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

mod static_library_loader;

mod ptr_iree_lib_query_fn;

mod dummy_c_symbols;

mod iree_threading;

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
    let device = static_library_loader::create_local_task_device_with_static_loader(libraries.as_slice()).unwrap();
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
    function.invoke(&input_list, &output_list).unwrap();
    info!("run_lenet5_float, function invoke successful!");
    let output_buffer_ref = output_list.get_ref(0).unwrap();
    let output_buffer: BufferView<f32> = output_buffer_ref.to_buffer_view(&session);
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

#[ariel_os::task(autostart)]
async fn main() {
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
            static MODEL_BYTECODE: &[u8] = include_aligned!(16, "../lenet5_quantized.vmfb");
//        }

//        #[cfg(not(feature = "quantized"))]
//        {
//            static MODEL_BYTECODE: &[u8] = include_aligned!(16, "../lenet5_float.vmfb");
//        }
//        
        // info!("main, vmfb pointer: {:p}", MODEL_BYTECODE.as_ptr());
        let output = run_lenet5_float(&MODEL_BYTECODE, &image_bin);
        output.iter().for_each(|x| info!("output:{}", x));
    }
   
    exit(ExitCode::SUCCESS);
}



