use crate::{
    allocator,
    id::TaskId,
    percpu::{self, get_percpu, PerCPU},
};

#[no_mangle]
pub extern "C" fn current_task() -> TaskId {
    get_percpu().current_task()
}

#[no_mangle]
pub extern "C" fn put_prev_task(task: TaskId, front: bool) {
    get_percpu().put_prev_task(task, front);
}

#[no_mangle]
pub extern "C" fn set_current_task(task: TaskId) {
    get_percpu().set_current_task(task);
}

#[no_mangle]
pub extern "C" fn init(percpu_size: usize) {
    allocator::init();
    percpu::init_percpu(percpu_size);
}

#[no_mangle]
pub extern "C" fn pick_next_task() -> TaskId {
    get_percpu().pick_next_task()
}

#[no_mangle]
pub extern "C" fn add_task(task: TaskId) {
    get_percpu().add_task(task);
}

#[no_mangle]
pub extern "C" fn first_add_task(task: TaskId) {
    PerCPU::first_add_task(task);
}
