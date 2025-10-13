#[cfg(all(target_os = "none", not(feature = "std")))]
#[unsafe(no_mangle)]
static __preinit_array_start: u8 = 0;

#[cfg(all(target_os = "none", not(feature = "std")))]
#[unsafe(no_mangle)]
static __preinit_array_end: u8 = 0;

#[cfg(all(target_os = "none", not(feature = "std")))]
#[unsafe(no_mangle)]
static __init_array_end: u8 = 0;

#[cfg(all(target_os = "none", not(feature = "std")))]
#[unsafe(no_mangle)]
static __init_array_start: u8 = 0;