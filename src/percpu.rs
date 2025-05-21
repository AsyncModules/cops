/// 这里定义了每个 CPU 的局部数据，因为非 PIC 的代码在共享库中不能正常工作，
/// 所以 percpu 库的实现方式在这里不能继续使用
use crate::{
    id::TaskId,
    stack_pool::{RunningStack, StackPool},
};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use queue::{AtomicCell, LockFreeQueue};

/// 由于构建脚本中使用的正则表达式没有写中文的匹配，所以这个数据结构内部不能使用中文注解
#[repr(C, align(64))]
pub struct PerCPU {
    /// Processor ready_queue
    ready_queue: LockFreeQueue<TaskId>,
    /// Running TaskId
    current_task: AtomicCell<Option<TaskId>>,
    /// RunningStack Pool
    stack_pool: StackPool,
}

impl PerCPU {
    pub fn new() -> Self {
        Self {
            ready_queue: LockFreeQueue::new(),
            current_task: AtomicCell::new(None),
            stack_pool: StackPool::new(),
        }
    }

    /// Pick one task from processor
    #[inline]
    pub(crate) fn pick_next_task(&self) -> TaskId {
        self.ready_queue.pop().unwrap_or(TaskId::NULL)
    }

    /// Add curr task to Processor, it ususally add to back
    #[inline]
    pub(crate) fn put_prev_task(&self, task: TaskId, _front: bool) {
        self.ready_queue.push(task);
    }

    /// Add task to processor, now just put it to own processor
    /// TODO: support task migrate on differ processor
    #[inline]
    pub(crate) fn add_task(&self, task: TaskId) {
        self.ready_queue.push(task);
    }

    /// First add task to processor
    #[inline]
    pub(crate) fn first_add_task(task: TaskId) {
        Self::select_cpu().ready_queue.push(task);
    }

    #[inline]
    fn select_cpu() -> &'static PerCPU {
        percpus()
            .iter()
            .min_by_key(|p| p.ready_queue.len())
            .unwrap()
    }

    pub fn current_task(&self) -> TaskId {
        self.current_task.load().unwrap_or(TaskId::NULL)
    }

    pub fn set_current_task(&self, task: TaskId) {
        self.current_task.store(Some(task));
    }

    pub fn init_running_stack(&self, curr_boot_stack: *mut u8) {
        self.stack_pool.init(curr_boot_stack);
    }

    /// 从处理器中取出当前的运行栈
    pub fn pick_current_stack(&self) -> RunningStack {
        self.stack_pool.pick_current_stack()
    }

    /// 获取当前运行栈的引用
    pub fn current_stack(&self) -> &RunningStack {
        self.stack_pool.current_stack()
    }

    /// 设置当前运行栈
    pub fn set_current_stack(&self, stack: RunningStack) {
        self.stack_pool.set_current_stack(stack);
    }
}

pub(crate) static mut PERCPU_AREA_SIZE: usize = 0;

pub(crate) fn percpu_area_base() -> usize {
    crate::get_data_base()
}

pub(crate) fn init_percpu(size: usize) {
    unsafe {
        PERCPU_AREA_SIZE = size;
    }
    let base = crate::get_data_base();
    for i in 0..axconfig::SMP {
        let percpu = unsafe { &mut *((base + i * size) as *mut PerCPU) };
        *percpu = PerCPU::new();
    }
}

pub(crate) fn percpus() -> Vec<&'static PerCPU> {
    let base = crate::get_data_base();
    let size = unsafe { PERCPU_AREA_SIZE };
    let mut percpus = Vec::new();
    for i in 0..axconfig::SMP {
        let percpu = unsafe { &*((base + i * size) as *mut PerCPU) };
        percpus.push(percpu);
    }
    percpus
}

/// 这里获取到 PerCPU 数据需要根据 get_data_base 来获取到数据段基址，并且根据 gp 寄存器可以获得到对应的偏移地址
/// 因为记录了 PERCPU_AREA_SIZE，因此可以根据寄存器中的最后几位来获取到实际的偏移地址
/// TODO: 支持多架构实现
#[inline]
pub(crate) fn get_percpu() -> &'static PerCPU {
    let base = crate::get_data_base();
    let size = unsafe { PERCPU_AREA_SIZE };
    let tp: usize;
    unsafe { core::arch::asm!("mv {}, gp", out(reg) tp) }
    let size_bits = get_bits(size);
    let mask = (1 << size_bits) - 1;
    let tp = (tp & mask) + base;
    unsafe { &*(tp as *const PerCPU) }
}

/// 根据 percpu 数据段的大小来获取对应的指针的最低位宽
const fn get_bits(size: usize) -> usize {
    (usize::BITS - size.leading_zeros()) as usize
}
