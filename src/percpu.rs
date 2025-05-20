use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use alloc::vec::Vec;

/// 这里定义了每个 CPU 的局部数据，因为非 PIC 的代码在共享库中不能正常工作，所以 percpu 库的实现方式在这里不能继续使用
///
use crate::{id::TaskId, processor::Processor};

#[repr(C, align(64))]
pub struct PerCPU {
    processor: Processor,
    cpu_id: AtomicUsize,
    is_bsp: AtomicBool,
}

pub fn set_scheduler_ptr(scheduler_ptr: usize) {
    let current_processor = current_processor();
    current_processor.set_scheduler_ptr(scheduler_ptr);
}

pub fn get_scheduler_ptr() -> usize {
    let current_processor = current_processor();
    current_processor.get_scheduler_ptr()
}

pub fn put_prev_task(task: TaskId, front: bool) {
    let current_processor = current_processor();
    current_processor.put_prev_task(task, front);
}

pub fn set_current_task(task: TaskId) {
    let current_processor = current_processor();
    current_processor.set_current_task(task);
}

pub fn current_taskid() -> TaskId {
    let current_processor = current_processor();
    current_processor.current_task().unwrap_or(TaskId::NULL)
}

pub fn current_processor() -> &'static Processor {
    &get_percpu().processor
}

const fn align_up_64(val: usize) -> usize {
    const SIZE_64BIT: usize = 0x40;
    (val + SIZE_64BIT - 1) & !(SIZE_64BIT - 1)
}

static PERCPU_AREA_BASE: spin::once::Once<usize> = spin::once::Once::new();

pub(crate) fn init_percpu_primary(cpu_id: usize) {
    let size = core::mem::size_of::<PerCPU>();
    let align = core::mem::align_of::<PerCPU>();
    let total_size = align_up_64(size) * axconfig::SMP;
    let layout = core::alloc::Layout::from_size_align(total_size, align).unwrap();
    PERCPU_AREA_BASE.call_once(|| unsafe { alloc::alloc::alloc(layout) as usize });
    let base = PERCPU_AREA_BASE.get().unwrap();
    unsafe {
        core::slice::from_raw_parts_mut(*base as *mut u8, total_size).fill(0);
    };
    let percpu = unsafe { &mut *(*base as *mut PerCPU) };
    *percpu = PerCPU {
        processor: Processor::new(),
        cpu_id: AtomicUsize::new(cpu_id),
        is_bsp: AtomicBool::new(true),
    };
    setup_percpu(cpu_id);
}

pub(crate) fn init_percpu_secondary(cpu_id: usize) {
    let size = core::mem::size_of::<PerCPU>();
    let base = PERCPU_AREA_BASE.get().unwrap();
    let percpu = unsafe { &mut *((*base + cpu_id * align_up_64(size)) as *mut PerCPU) };
    *percpu = PerCPU {
        processor: Processor::new(),
        cpu_id: AtomicUsize::new(cpu_id),
        is_bsp: AtomicBool::new(false),
    };
    setup_percpu(cpu_id);
}

pub(crate) fn percpus() -> Vec<&'static Processor> {
    let size = core::mem::size_of::<PerCPU>();
    let base = PERCPU_AREA_BASE.get().unwrap();
    let mut processors = Vec::new();
    for i in 0..axconfig::SMP {
        let percpu = unsafe { &mut *((*base + i * align_up_64(size)) as *mut PerCPU) };
        processors.push(&percpu.processor);
    }
    processors
}

/// Set the architecture-specific thread pointer register to the per-CPU data
/// area base on the current CPU.
///
/// `cpu_id` indicates which per-CPU data area to use.
pub fn setup_percpu(cpu_id: usize) {
    let tp = PERCPU_AREA_BASE.get().unwrap() + cpu_id * align_up_64(core::mem::size_of::<PerCPU>());
    unsafe {
        core::arch::asm!("mv gp, {}", in(reg) tp);
    }
}

/// Read the architecture-specific thread pointer register on the current CPU.
pub fn get_percpu() -> &'static PerCPU {
    let tp: usize;
    unsafe { core::arch::asm!("mv {}, gp", out(reg) tp) }
    unsafe { &*(tp as *const PerCPU) }
}
