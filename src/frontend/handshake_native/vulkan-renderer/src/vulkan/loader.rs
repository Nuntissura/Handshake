//! The Vulkan loader library. On Windows it comes from the system directory and from nowhere else, so a library of
//! that name beside the executable, in the working directory or on the path is never run.

use crate::outcome::Unavailable;

/// Keeps the loader library loaded while something of it is in use.
#[cfg(windows)]
pub(super) type LoaderLibrary = std::sync::Arc<libloading::os::windows::Library>;
/// Elsewhere the entry keeps it.
#[cfg(not(windows))]
pub(super) type LoaderLibrary = std::sync::Arc<ash::Entry>;

#[cfg(windows)]
pub(super) fn load() -> Result<(ash::Entry, LoaderLibrary), Unavailable> {
    use ash::vk::PFN_vkGetInstanceProcAddr;
    use libloading::os::windows::{Library, LOAD_LIBRARY_SEARCH_SYSTEM32};

    // SAFETY: loads the system's Vulkan loader, whose load and unload routines have no precondition on the caller.
    // The flag limits the search, also for its dependencies, to the system directory. A missing or unloadable
    // library is an `Err`.
    let library = unsafe { Library::load_with_flags("vulkan-1.dll", LOAD_LIBRARY_SEARCH_SYSTEM32) }
        .map_err(|_| Unavailable::LoaderMissing)?;
    // SAFETY: the Vulkan specification defines the type of this exported function.
    let get_instance_proc_addr: PFN_vkGetInstanceProcAddr =
        match unsafe { library.get::<PFN_vkGetInstanceProcAddr>(b"vkGetInstanceProcAddr\0") } {
            Ok(symbol) => *symbol,
            Err(_) => return Err(Unavailable::LoaderMissing),
        };
    let static_fn = ash::StaticFn {
        get_instance_proc_addr,
    };
    // SAFETY: the function pointer is the loader's own and stays valid while `library` is kept, which the caller
    // does for as long as it keeps the entry.
    let entry = unsafe { ash::Entry::from_static_fn(static_fn) };
    Ok((entry, LoaderLibrary::new(library)))
}

#[cfg(not(windows))]
pub(super) fn load() -> Result<(ash::Entry, LoaderLibrary), Unavailable> {
    // SAFETY: loads the platform's Vulkan loader through libloading (ash feature `loaded`). A missing or unloadable
    // library is an `Err`; the returned entry keeps it loaded.
    let entry = unsafe { ash::Entry::load() }.map_err(|_| Unavailable::LoaderMissing)?;
    let library = LoaderLibrary::new(entry.clone());
    Ok((entry, library))
}
