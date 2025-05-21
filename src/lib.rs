//!
#![cfg_attr(not(test), no_std)]
#![no_main]
#![feature(lang_items)]
#![allow(internal_features, unused, non_snake_case)]
mod allocator;
mod api;
mod id;
mod percpu;
mod stack_pool;

use allocator::DATA_OFFSET;
pub use api::*;
use percpu::PERCPU_AREA_SIZE;

extern crate alloc;

#[no_mangle]
pub(crate) fn get_data_base() -> usize {
    // 暂时使用这个量来对齐，只要代码的大小不超过 1G 就可以找到数据段的基址
    const DATA_ALIGN: usize = 0xFFFF_FFFF_C000_0000;
    let mut pc = 0usize;
    unsafe {
        core::arch::asm!(
            "auipc {pc}, 0",
            pc = out(reg) pc,
        );
    }
    (pc & DATA_ALIGN)
}

#[cfg(feature = "no_std")]
mod lang_item {

    #[lang = "eh_personality"]
    #[no_mangle]
    fn rust_eh_personality() {}

    #[panic_handler]
    fn panic(_info: &core::panic::PanicInfo) -> ! {
        unsafe {
            core::arch::asm!("ebreak");
        }
        unreachable!()
    }

    #[no_mangle]
    fn _Unwind_Resume() {
        unreachable!()
    }
}
