use crate::{
    id::TaskId,
    stack_pool::{RunningStack, StackPool},
};
use alloc::{boxed::Box, collections::VecDeque};
use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicUsize, Ordering},
};
use queue::AtomicCell;
use queue::LockFreeQueue;
use spin::Lazy;

/// 这个数据结构只能使用无锁的数据结构，因为在内核和用户态使用的锁不一样
/// 此外，还需要额外的结构来存放每个 CPU 上使用的数据，因为内核有自己重新定义的数据
/// 可以将 percpu 的初始化放在这里进行，其他的包中不需要使用 percpu 数据
///
/// 使用线程接口进行上下文切换时，需要保证换栈的过程中不会被中断，还需要进一步的思考设计，需要进一步写出文档
#[repr(C, align(64))]
pub struct Processor {
    /// Processor ready_queue
    ready_queue: LockFreeQueue<TaskId>,
    /// 记录的当前任务标识
    current_task: AtomicCell<Option<TaskId>>,
    /// 运行栈池
    stack_pool: StackPool,
    /// scheduler_wrapper，这个是用于记录在内核中的调度器的指针，是为了兼容性，当使用非 vdso 调度器时，这个记录实际的调度器的指针，调用的方法是在其他模块实现的；当使用 vdso 调度器时，它记录的调度器封装器的指针，实际调用的方法是 vdso 提供的 api
    scheduler_ptr: AtomicUsize,
}

unsafe impl Sync for Processor {}
unsafe impl Send for Processor {}

impl Processor {
    pub fn new() -> Self {
        let queue = LockFreeQueue::new();
        Processor {
            ready_queue: queue,
            current_task: AtomicCell::new(None),
            stack_pool: StackPool::new(),
            scheduler_ptr: AtomicUsize::new(0),
        }
    }

    #[inline]
    /// Get the scheduler pointer
    pub fn get_scheduler_ptr(&self) -> usize {
        self.scheduler_ptr.load(Ordering::Relaxed)
    }

    #[inline]
    /// Get the scheduler pointer
    pub fn set_scheduler_ptr(&self, scheduler_ptr: usize) {
        self.scheduler_ptr.store(scheduler_ptr, Ordering::Relaxed);
    }

    #[inline]
    /// Pick one task from processor
    pub(crate) fn pick_next_task(&self) -> Option<TaskId> {
        self.ready_queue.pop()
    }

    #[inline]
    /// Add curr task to Processor, it ususally add to back
    pub(crate) fn put_prev_task(&self, task: TaskId, _front: bool) {
        self.ready_queue.push(task);
    }

    #[inline]
    /// Add task to processor, now just put it to own processor
    /// TODO: support task migrate on differ processor
    pub(crate) fn add_task(&self, task: TaskId) {
        self.ready_queue.push(task);
    }

    #[inline]
    /// First add task to processor
    pub(crate) fn first_add_task(task: TaskId) {
        let p = Processor::select_processor();
        p.ready_queue.push(task);
    }

    #[inline]
    fn select_processor() -> &'static Processor {
        crate::percpu::percpus()
            .iter()
            .min_by_key(|p| p.ready_queue.len())
            .unwrap()
    }
}

/// 与当前任务相关的操作
impl Processor {
    pub fn current_task(&self) -> Option<TaskId> {
        self.current_task.load()
    }
    pub fn set_current_task(&self, task: TaskId) {
        self.current_task.store(Some(task));
    }
}

/// 与运行栈相关的操作
impl Processor {
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
