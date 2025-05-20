/// 这里定义了每个 CPU 的局部数据，因为非 PIC 的代码在共享库中不能正常工作，
/// 所以 percpu 库的实现方式在这里不能继续使用
use crate::{
    id::TaskId,
    stack_pool::{RunningStack, StackPool},
};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use queue::{AtomicCell, LockFreeQueue};

#[repr(C, align(64))]
pub struct PerCPU {
    /// 就绪队列
    ready_queue: LockFreeQueue<TaskId>,
    /// 正在运行的任务标识
    current_task: AtomicCell<Option<TaskId>>,
    /// 运行栈池，用于线程与协程的兼容
    stack_pool: StackPool,
    /// scheduler_wrapper，记录每个 cpu 使用的调度器的指针，
    /// 当使用非 vdso 调度器时，它指向实际的调度器，调用的方法是在其他模块实现的；
    /// 当使用 vdso 调度器时，它指向外部的 vdso 模块实现的调度器封装器，其实际调用的方法是这个库中提供的 api
    scheduler_ptr: AtomicUsize,
    /// cpu id，在初始化之后保持不变
    cpu_id: AtomicUsize,
    /// 是否是引导处理器
    is_bsp: AtomicBool,
    /// 定时器下次到期时间
    timer_next_deadline: AtomicUsize,
}

#[inline]
pub(crate) fn get_timer_next_deadline() -> usize {
    let percpu = get_percpu();
    percpu.timer_next_deadline.load(Ordering::Relaxed)
}

#[inline]
pub(crate) fn set_timer_next_deadline(deadline: usize) {
    let percpu = get_percpu();
    percpu
        .timer_next_deadline
        .store(deadline, Ordering::Relaxed);
}

#[inline]
pub(crate) fn this_cpu_id() -> usize {
    let percpu = get_percpu();
    percpu.cpu_id.load(Ordering::Relaxed)
}

#[inline]
pub(crate) fn this_cpu_is_bsp() -> bool {
    let percpu = get_percpu();
    percpu.is_bsp.load(Ordering::Relaxed)
}

#[inline]
pub(crate) fn set_scheduler_ptr(scheduler_ptr: usize) {
    get_percpu()
        .scheduler_ptr
        .store(scheduler_ptr, Ordering::Relaxed);
}

#[inline]
pub(crate) fn get_scheduler_ptr() -> usize {
    get_percpu().scheduler_ptr.load(Ordering::Relaxed)
}

#[inline]
pub(crate) fn pick_next_task() -> TaskId {
    get_percpu().ready_queue.pop().unwrap_or(TaskId::NULL)
}

#[inline]
pub(crate) fn put_prev_task(task: TaskId, _front: bool) {
    get_percpu().ready_queue.push(task);
}

/// Add task to processor, now just put it to own processor
/// TODO: support task migrate on differ processor
#[inline]
pub(crate) fn add_task(task: TaskId) {
    get_percpu().ready_queue.push(task);
}

/// First add task to processor
#[inline]
pub(crate) fn first_add_task(task: TaskId) {
    select_least_load_cpu().ready_queue.push(task);
}

#[inline]
fn select_least_load_cpu() -> &'static PerCPU {
    crate::percpu::percpus()
        .iter()
        .min_by_key(|p| p.ready_queue.len())
        .unwrap()
}

#[inline]
pub(crate) fn current_taskid() -> TaskId {
    get_percpu().current_task.load().unwrap_or(TaskId::NULL)
}

#[inline]
pub(crate) fn set_current_task(task: TaskId) {
    get_percpu().current_task.store(Some(task));
}

#[inline]
pub(crate) fn init_running_stack(curr_boot_stack: *mut u8) {
    get_percpu().stack_pool.init(curr_boot_stack);
}

/// 从处理器中取出当前的运行栈
#[inline]
pub(crate) fn pick_current_stack() -> RunningStack {
    get_percpu().stack_pool.pick_current_stack()
}

/// 获取当前运行栈的引用
#[inline]
pub(crate) fn current_stack() -> &'static RunningStack {
    get_percpu().stack_pool.current_stack()
}

/// 设置当前运行栈
#[inline]
pub(crate) fn set_current_stack(stack: RunningStack) {
    get_percpu().stack_pool.set_current_stack(stack);
}

#[inline]
const fn align_up_64(val: usize) -> usize {
    const SIZE_64BIT: usize = 0x40;
    (val + SIZE_64BIT - 1) & !(SIZE_64BIT - 1)
}

static PERCPU_AREA_BASE: spin::once::Once<usize> = spin::once::Once::new();

pub(crate) fn init_percpu_primary(cpu_id: usize) {
    crate::allocator::init();
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
        ready_queue: LockFreeQueue::new(),
        current_task: AtomicCell::new(None),
        stack_pool: StackPool::new(),
        scheduler_ptr: AtomicUsize::new(0),
        cpu_id: AtomicUsize::new(cpu_id),
        is_bsp: AtomicBool::new(true),
        timer_next_deadline: AtomicUsize::new(0),
    };
    setup_percpu(cpu_id);
}

#[inline]
pub(crate) fn init_percpu_secondary(cpu_id: usize) {
    let size = core::mem::size_of::<PerCPU>();
    let base = PERCPU_AREA_BASE.get().unwrap();
    let percpu = unsafe { &mut *((*base + cpu_id * align_up_64(size)) as *mut PerCPU) };
    *percpu = PerCPU {
        ready_queue: LockFreeQueue::new(),
        current_task: AtomicCell::new(None),
        stack_pool: StackPool::new(),
        scheduler_ptr: AtomicUsize::new(0),
        cpu_id: AtomicUsize::new(cpu_id),
        is_bsp: AtomicBool::new(false),
        timer_next_deadline: AtomicUsize::new(0),
    };
    setup_percpu(cpu_id);
}

#[inline]
pub(crate) fn percpus() -> Vec<&'static PerCPU> {
    let size = core::mem::size_of::<PerCPU>();
    let base = PERCPU_AREA_BASE.get().unwrap();
    let mut percpus = Vec::new();
    for i in 0..axconfig::SMP {
        let percpu = unsafe { &*((*base + i * align_up_64(size)) as *const PerCPU) };
        percpus.push(percpu);
    }
    percpus
}

/// Set the architecture-specific thread pointer register to the per-CPU data
/// area base on the current CPU.
///
/// `cpu_id` indicates which per-CPU data area to use.
#[inline]
pub(crate) fn setup_percpu(cpu_id: usize) {
    let tp = PERCPU_AREA_BASE.get().unwrap() + cpu_id * align_up_64(core::mem::size_of::<PerCPU>());
    unsafe {
        core::arch::asm!("mv gp, {}", in(reg) tp);
    }
}

/// Read the architecture-specific thread pointer register on the current CPU.
#[inline]
pub(crate) fn get_percpu() -> &'static PerCPU {
    let tp: usize;
    unsafe { core::arch::asm!("mv {}, gp", out(reg) tp) }
    unsafe { &*(tp as *const PerCPU) }
}
