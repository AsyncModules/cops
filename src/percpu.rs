use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use alloc::vec::Vec;
use queue::{AtomicCell, LockFreeQueue};

use crate::{
    id::TaskId,
    stack_pool::{RunningStack, StackPool},
};

#[repr(C, align(64))]
pub struct PerCPU {
    /// Processor ready_queue
    ready_queue: LockFreeQueue<TaskId>,
    /// 记录的当前任务标识
    current_task: AtomicCell<Option<TaskId>>,
    /// 运行栈池
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

static PERCPU_AREA_BASE: AtomicUsize = AtomicUsize::new(0);
static PERCPU_AREA_SIZE: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn init_percpu(base: usize, size: usize) {
    PERCPU_AREA_BASE.store(base, Ordering::Relaxed);
    PERCPU_AREA_SIZE.store(size, Ordering::Relaxed);
    for i in 0..axconfig::SMP {
        let percpu = unsafe { &mut *((base + i * size) as *mut PerCPU) };
        *percpu = PerCPU::new();
    }
}

pub(crate) fn percpus() -> Vec<&'static PerCPU> {
    let size = PERCPU_AREA_SIZE.load(Ordering::Relaxed);
    let base = PERCPU_AREA_BASE.load(Ordering::Relaxed);
    let mut processors = Vec::new();
    for i in 0..axconfig::SMP {
        let percpu = unsafe { &*((base + i * size) as *mut PerCPU) };
        processors.push(percpu);
    }
    processors
}

/// Read the architecture-specific thread pointer register on the current CPU.
pub fn get_percpu() -> &'static PerCPU {
    let tp: usize;
    unsafe { core::arch::asm!("mv {}, gp", out(reg) tp) }
    unsafe { &*(tp as *const PerCPU) }
}
