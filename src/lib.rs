#![no_std]
use core::ffi;
use core::ptr;
use libc;

// A minimal malloc implementation that simply returns null pointer.
#[no_mangle]
pub extern  fn malloc(_size: libc::size_t) -> *mut ffi::c_void {
    ptr::null_mut()
}

// A minimal free implementation that does nothing.
#[no_mangle]
pub extern fn free(_ptr: *mut ffi::c_void) {
}

// An obligatory no_std panic_handler.
#[panic_handler]
fn dont_panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { libc::exit(42) }
}
