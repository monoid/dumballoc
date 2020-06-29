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
    unsafe { libc::write(libc::STDOUT_FILENO, (&(if val < 10 { b'0' + val } else { b'a' + val - 10 }) as *const u8).cast(), 1); }
        
}
fn print(v: &str, val: usize) {
    unsafe {
        libc::write(libc::STDOUT_FILENO, v.as_bytes().as_ptr().cast(), v.as_bytes().len());
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
        libc::write(libc::STDOUT_FILENO, (&b'\n' as *const u8).cast(), 1);
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
            unsafe {
                // Glibc expects malloc to set errno on any failure
                // (i.e. when null pointer is returned).  Reference:
                // https://linux.die.net/man/3/malloc, Notes section.
                *libc::__errno_location() = libc::ENOMEM;
            }
            ptr::null_mut()
        }
    }
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
unsafe fn get_region_size_by_ptr(ptr: *mut ffi::c_void) -> libc::size_t {
    let base_ptr = ptr.sub(HEADER_SIZE);
    // TODO: use libc::size_t for header.
    let size: usize = *base_ptr.cast();
    return size as libc::size_t;
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

    let orig_size = unsafe { get_region_size_by_ptr(ptr) };

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

// Overriden by panic in vmap or its dependency :(
// #[panic_handler]
// fn dont_panic(_info: &core::panic::PanicInfo) -> ! {
//     unsafe { libc::exit(42) }
// }
