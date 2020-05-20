#![no_std]
use core::ffi;
use core::ptr;
use libc;

// A minimal malloc implementation that simply returns null pointer.
#[no_mangle]
pub extern  fn malloc(_size: libc::size_t) -> *mut ffi::c_void {
    unsafe {
        // Glibc expects malloc to set errno on any failure
        // (i.e. when null pointer is returned).  Reference:
        // https://linux.die.net/man/3/malloc, Notes section.
        *libc::__errno_location() = libc::ENOMEM;
    }
    ptr::null_mut()
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
