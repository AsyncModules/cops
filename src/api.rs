use crate::{allocator, id::TaskId, percpu, processor::Processor};

#[no_mangle]
pub extern "C" fn set_scheduler_ptr(scheduler_ptr: usize) {
    percpu::set_scheduler_ptr(scheduler_ptr);
}

#[no_mangle]
pub extern "C" fn get_scheduler_ptr() -> usize {
    percpu::get_scheduler_ptr()
}

#[no_mangle]
pub extern "C" fn current_task() -> TaskId {
    percpu::current_taskid()
}

#[no_mangle]
pub extern "C" fn put_prev_task(task: TaskId, front: bool) {
    percpu::put_prev_task(task, front);
}

#[no_mangle]
pub extern "C" fn set_current_task(task: TaskId) {
    percpu::set_current_task(task);
}

#[no_mangle]
pub extern "C" fn init_primary(cpu_id: usize) {
    allocator::init();
    percpu::init_percpu_primary(cpu_id);
}

#[no_mangle]
pub extern "C" fn init_secondary(cpu_id: usize) {
    percpu::init_percpu_secondary(cpu_id);
}

#[no_mangle]
pub extern "C" fn pick_next_task() -> TaskId {
    percpu::current_processor()
        .pick_next_task()
        .unwrap_or(TaskId::NULL)
}

#[no_mangle]
pub extern "C" fn add_task(task: TaskId) {
    percpu::current_processor().add_task(task);
}

#[no_mangle]
pub extern "C" fn first_add_task(task: TaskId) {
    Processor::first_add_task(task);
}
