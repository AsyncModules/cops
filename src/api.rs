use crate::{allocator, id::TaskId, percpu, processor::Processor};

#[no_mangle]
pub extern "C" fn current_task() -> TaskId {
    percpu::current_taskid()
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
