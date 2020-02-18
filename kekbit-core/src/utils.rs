use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) const WATERMARK: u64 = 0xFFFF_FFFF_1111_1111;
pub(crate) const CLOSE: u64 = 0xFFFF_FFFF_FFFF_FFFF;
pub(crate) const U64_SIZE: usize = std::mem::size_of::<u64>(); //8 bytes, size of u64
pub(crate) const REC_HEADER_LEN: u32 = 8; //8 bytes for len or message type
pub(crate) const FOOTER_LEN: u32 = 32; //we need 8 bytes for WATERMARK|CLOSE_MARK the other are for future use

const REC_ALIGNMENT: u32 = U64_SIZE as u32; //8 bytes, size of u64

#[inline]
pub(crate) const fn align(value: u32) -> u32 {
    (value + (REC_ALIGNMENT - 1)) & !(REC_ALIGNMENT - 1)
}

#[inline]
pub(crate) const fn is_aligned(val: u32) -> bool {
    val & (REC_ALIGNMENT - 1) == 0
}

#[inline]
pub(crate) fn store_atomic_u64(pos_ptr: *mut u64, value: u64, order: Ordering) {
    let store_pos = unsafe { &*(pos_ptr as *const AtomicU64) };
    store_pos.store(value, order);
}

#[inline]
pub(crate) fn load_atomic_u64(pos_ptr: *mut u64, order: Ordering) -> u64 {
    let store_pos = unsafe { &*(pos_ptr as *const AtomicU64) };
    store_pos.load(order)
}
