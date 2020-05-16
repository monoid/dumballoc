#![no_std]
// An obligatory no_std panic_handler.
use libc;
use core::ffi;
use core::ptr;
use core::sync::atomic;
use vmap::os::{map_anon, unmap};

const HEADER_SIZE: usize = core::mem::size_of::<usize>();

struct AtomicLock {
    atomic: atomic::AtomicBool,
}

struct Guard<'a> {
    parent: &'a AtomicLock,
}

impl AtomicLock {
    const fn new() -> Self {
        AtomicLock { atomic: atomic::AtomicBool::new(false) }
    }

    fn lock(&self) -> Guard<'_> {
        while self.atomic.swap(true, atomic::Ordering::SeqCst) {
            atomic::spin_loop_hint();
        }
        
        Guard { parent: self }
    }
}

impl<'a> Drop for Guard<'a> {
    fn drop(&mut self) {
        self.parent.atomic.store(false, atomic::Ordering::SeqCst);
    }
}

static GLOBAL_LOCK: AtomicLock = AtomicLock::new();

fn print_digit(val: u8) {
    let val = val & 0xF;
    unsafe { libc::write(1, (&(if val < 10 { b'0' + val } else { b'a' + val - 10 }) as *const u8).cast(), 1); }
        
}
fn print(v: &str, val: usize) {
    unsafe {
        libc::write(1, v.as_bytes().as_ptr().cast(), v.as_bytes().len());
    }

    for b in &(val as u64).to_be_bytes() {
        print_digit(*b >> 4);
        print_digit(*b & 0xF);
    }
}

fn print_ptrs(v: &str, size: usize, ptr: usize) {
    let _lock = GLOBAL_LOCK.lock();
    print(v, size);
    print(" ", ptr);
    unsafe {
        libc::write(1, (&b'\n' as *const u8).cast(), 1);
    }
}

#[no_mangle]
pub extern  fn malloc(size: libc::size_t) -> *mut ffi::c_void {
    // TODO check that the sum doesn't overflow isize -- it is
    // non-realistic for modern hardware anyway.
    match unsafe { map_anon(size + HEADER_SIZE) } {
        Ok(base_ptr) => {
            print_ptrs("malloc ", size, base_ptr as usize);
            // Write header
            unsafe {
                (base_ptr as *mut usize).write(size);
            }
            // This offset doesn't overflow because it of the check above.
            unsafe { base_ptr.add(HEADER_SIZE).cast() }
        },
        Err(_) => {
            print_ptrs("malloc null ", size, 0);
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern fn free(ptr: *mut ffi::c_void) {

    print_ptrs("free ", 0, ptr as usize);
    if ptr.is_null() {
        return;
    }

    let ptr: *mut u8 = ptr.cast();
    unsafe {
        let base_ptr = ptr.sub(HEADER_SIZE);
        print_ptrs("free base ", 0, base_ptr as usize);

        let size: usize = *base_ptr.cast();

        print_ptrs("free size ", size, base_ptr as usize);

        unmap(base_ptr, size).unwrap_or(());  // Yep, result is ignored
    }
}

// Overriden by panic in vmap :(
// #[panic_handler]
// fn dont_panic(_info: &core::panic::PanicInfo) -> ! {
//     unsafe { libc::exit(42) }
// }
