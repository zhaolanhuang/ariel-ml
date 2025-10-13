use cc;

fn main() {
    // Rerun if target changes
    println!("cargo:rerun-if-env-changed=TARGET");

    // Read target triple
    let target = std::env::var("TARGET").unwrap();
    println!("cargo:warning=Target triple = {}", target);


    // Split info
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    let env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_else(|_| "none".into());

    println!("cargo:warning=arch={} os={} env={}", arch, os, env);

    // println!("cargo:rustc-env=IREE_LLD_BINARY=/usr/bin/lld");
    // let IREE_LLD_BINARY = "/usr/bin/lld";  

    let context = std::env::var("CARGO_CFG_CONTEXT").unwrap();

    let mut target_cpu = "";

    //TODO: add support for more cpus, and libm support for generic.
    if context.contains("cortex-m4f") || context.contains("cortex-m4") {
        target_cpu = "cortex-m4";
    } else {
        target_cpu = "generic";
    } 
    
    println!("cargo:warning=CARGO_CFG_CONTEXT {}",context);
    println!("cargo:warning=target_cpu {}",target_cpu);

    
    let mut iree_compile_flags: Vec<String> = vec![
                "--iree-hal-target-device=local".into(),
                "--iree-hal-local-target-device-backends=llvm-cpu".into(),
                format!("--iree-llvmcpu-target-triple={}", target),
                format!("--iree-llvmcpu-target-cpu={}", target_cpu),
                // "--iree-llvmcpu-target-triple=armv7a-pc-linux-elf",
                // "--iree-llvmcpu-target-cpu=cortex-m4",
                "--iree-vm-bytecode-module-strip-source-map=true".into(),
                "--iree-vm-emit-polyglot-zip=false".into(),
                // "--iree-llvmcpu-keep-linker-artifacts",

            ];

    
    
    
    let mut model_name = "no_model_selected";
    let mut query_fn_name = "no_fn_func";
    #[cfg(feature = "simple_mul")]
    {
        model_name = "simple_mul";
        query_fn_name = "simple_mul_dispatch_0_library_query";
    }    

    #[cfg(feature = "resnet50")]
    {
        model_name = "resnet50";
    }

    #[cfg(feature = "mnist")]
    {
        model_name = "mnist";
        query_fn_name = "mnist_linked_library_query";
        
    }

    #[cfg(feature = "lenet5")]
    {
        #[cfg(feature = "quantized")]
        {
            model_name = "lenet5_quantized";
        }

        #[cfg(not(feature = "quantized"))]
        {
            model_name = "lenet5_float";
        }
        
        query_fn_name = "module_linked_library_query";
        
    }
    println!("cargo:rustc-env=IREE_LIB_QUERY_FN_NAME={}", query_fn_name);
    

    println!("cargo::rerun-if-changed={}.mlir", model_name);
    println!("cargo::rerun-if-changed={}.vmfb", model_name);
    println!("cargo::rerun-if-changed={}.o", model_name);


    #[cfg(feature = "static")] 
    {
        iree_compile_flags.push("--iree-llvmcpu-link-embedded=false".into());
        iree_compile_flags.push("--iree-llvmcpu-link-static".into());
        iree_compile_flags.push(format!("--iree-llvmcpu-static-library-output-path={}", model_name.to_string() + ".o"));
        println!("cargo:rustc-link-arg={}", model_name.to_string() + ".o");
    }

    #[cfg(feature = "emitc")]
    {
        iree_compile_flags.push("--output-format=vm-c".into());
    }
    
    iree_compile_flags.push("--dump-compilation-phases-to=build/iree".into());

    #[cfg(not(feature= "emitc"))]
    iree_compile_flags.extend(vec![
        model_name.to_string()  + ".mlir", 
        "-o".into(), model_name.to_string()  + ".vmfb",
    ]);

    #[cfg(feature= "emitc")]
    iree_compile_flags.extend(vec![
        model_name.to_string()  + ".mlir", 
        "-o".into(), model_name.to_string()  + "_emitc.c",
    ]);
    

    std::process::Command::new("iree-compile")
        .args(iree_compile_flags)
        .status()
        .map_err(|e| format!("[IREE Model Compile] Failed to compile {}, {}", model_name, e))
        .unwrap();

    #[cfg(feature= "emitc")]
    {
        let mut c_build = cc::Build::new();
        let include_dir = "/home/zhaolan/eerie/eerie-sys/iree/runtime/src"; //should avoid hardcode
        c_build.file(model_name.to_string()  + "_emitc.c")
               .include(include_dir)
               .define("EMITC_IMPLEMENTATION", None)
               .flags(vec![              
                "-DIREE_PLATFORM_GENERIC=1",
                ]);
               
        let obj_files = c_build.compile_intermediates();
        obj_files.iter().for_each(|x|  println!("cargo:rustc-link-arg={}", x.display()) );

    }

    
}
