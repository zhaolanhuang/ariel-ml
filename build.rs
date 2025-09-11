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
    if context.contains("cortex-m4") {
        target_cpu = "cortex-m4";
    } else if context.contains("cortex-m4f") {
        target_cpu = "cortex-m4f";
    } else {
        target_cpu = "generic";
    } 
    
    println!("cargo:warning=CARGO_CFG_CONTEXT {}",context);
    println!("cargo:warning=target_cpu {}",target_cpu);

    #[cfg(feature = "simple_mul")]
    {
        println!("cargo::rerun-if-changed=simple_mul.mlir");
        println!("cargo::rerun-if-changed=simple_mul.vmfb");
        std::process::Command::new("iree-compile")
            .args([
                "--iree-hal-target-device=local",
                "--iree-hal-local-target-device-backends=llvm-cpu",
                format!("--iree-llvmcpu-target-triple={}", target).as_str(),
                format!("--iree-llvmcpu-target-cpu={}", target_cpu).as_str(),
                "--iree-vm-bytecode-module-strip-source-map=true",
                "--iree-vm-emit-polyglot-zip=false",
                "--iree-llvmcpu-debug-symbols=false",
                // format!("--iree-llvmcpu-embedded-linker-path={}", IREE_LLD_BINARY).as_str(),
                "simple_mul.mlir", 
                "-o", "simple_mul.vmfb"
            ])
            .status()
            .map_err(|e| format!("[IREE Model Compile] Failed to compile simple_mul, {}", e))
            .unwrap();
    }


    #[cfg(feature = "resnet50")]
    {
        println!("cargo::rerun-if-changed=resnet50.mlir");
        println!("cargo::rerun-if-changed=resnet50.vmfb");
        std::process::Command::new("iree-compile")
            .args([
                "--iree-input-type=stablehlo",
                "--iree-hal-target-device=local",
                "--iree-hal-local-target-device-backends=llvm-cpu",
                format!("--iree-llvmcpu-target-triple={}", target).as_str(),
                format!("--iree-llvmcpu-target-cpu={}", target_cpu).as_str(),
                "--iree-vm-bytecode-module-strip-source-map=true",
                "--iree-vm-emit-polyglot-zip=false",
                "--iree-llvmcpu-debug-symbols=false",
                // format!("--iree-llvmcpu-embedded-linker-path={}", IREE_LLD_BINARY).as_str(),
                "resnet50.mlir", 
                "-o", "resnet50.vmfb"
            ])
            .status()
            .map_err(|e| format!("[IREE Model Compile] Failed to compile resnet50, {}", e))
            .unwrap();
    }

    
}