#![no_std]
use core::mem;

use aya_ebpf::programs::XdpContext;

/// Safely get a pointer to a structure of type T at the given offset within the XDP packet data.
///
/// # Arguments
/// * `ctx` - The XdpContext containing packet data pointers.
/// * `offset` - The offset within the packet data to read from.
///
/// # Returns
/// * `Ok(*const T)` - A pointer to the structure of type T if successful.
/// * `Err(())` - An error if the offset is out of bounds.
#[inline(always)]
pub fn ptr_at<T>(ctx: &XdpContext, offset: usize) -> Result<*const T, ()> {
    let start = ctx.data();
    let end = ctx.data_end();
    let len = mem::size_of::<T>();

    if start + offset + len > end {
        return Err(());
    }

    Ok((start + offset) as *const T)
}
