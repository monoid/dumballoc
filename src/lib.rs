#![no_std]
use core::ffi;
use core::ptr;
use libc;

// A minimal malloc implementation that simply returns null pointer.
#[no_mangle]
pub extern fn malloc(_size: libc::size_t) -> *mut ffi::c_void {
    unsafe {
        // Glibc expects malloc to set errno to ENOMEM on any failure
        // (i.e. when null pointer is returned).  Reference:
        // https://linux.die.net/man/3/malloc, Notes section.
        *libc::__errno_location() = libc::ENOMEM;
    }
    ptr::null_mut()
}

// calloc -- allocates num records of size size and fills with zero.
// This is more or less working funciton that calls malloc.
#[no_mangle]
pub extern fn calloc(num: libc::size_t, size: libc::size_t) -> *mut ffi::c_void {
    let mem_size = match num.checked_mul(size) {
        Some(n) => n,
        None => {
            // "calloc returns a error"
            unsafe {
                // Glibc expects malloc/calloc to set errno on any
                // failure.  Reference: Notes section of
                // https://linux.die.net/man/3/malloc,
                *libc::__errno_location() = libc::ENOMEM;
            }
            return ptr::null_mut();
        }
    };

    let mem = malloc(mem_size);
    if mem != ptr::null_mut() {
        unsafe { ptr::write_bytes(mem, 0, mem_size) };
    }
    return mem;
}

// Helper function for realloc that gets allocation size from it
// header.  This noop implementation is never called in proper code
// because malloc never returns a valid pointer.  In real
// implementation, a function that unpacks memory header is to be
// used.
#[inline]
fn get_region_size_by_ptr(_ptr: *mut ffi::c_void) -> libc::size_t {
    unimplemented!("Cannot be implemented for noop malloc");
}

// Primitive version of realloc that always reallocate
#[no_mangle]
pub extern fn realloc(ptr: *mut ffi::c_void, size: libc::size_t) -> *mut ffi::c_void {
    if size == 0 {
        // Linux man page is somewhat contradictory for calloc(..., 0):
        //
        // ... if size is equal to zero, and ptr is not NULL, then the
        // call is equivalent to free(ptr) ... If size was equal to 0,
        // either NULL or a pointer suitable to be passed to free() is
        // returned.
        free(ptr);
        unsafe {
            *libc::__errno_location() = libc::ENOMEM;
        }
        return ptr::null_mut();
    }
    if ptr == ptr::null_mut() {
        // Again, special case in man malloc.
        return malloc(size);
    }

    let orig_size = get_region_size_by_ptr(ptr);

    if orig_size == size {
        // The Easiest part.
        return ptr;
    }
    // else

    // We cannot really realloc in generic implementation, so just
    // alloc new memory.
    let new_mem = malloc(size);
    if new_mem == ptr::null_mut() {
        // errno is set by malloc
        return new_mem;
    }
    // else

    unsafe {
        ptr::copy_nonoverlapping(ptr, new_mem, core::cmp::min(orig_size, size))
    };
    free(ptr);
    return new_mem;
}

// A minimal free implementation that does nothing.
#[no_mangle]
pub extern fn free(p: *mut ffi::c_void) {
    // free has to return immediately if p in null.
    if p == ptr::null_mut() {
        return;
    }
}

// An obligatory no_std panic_handler.
#[panic_handler]
fn dont_panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { libc::exit(42) }
}
