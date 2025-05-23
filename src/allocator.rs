use crate::{get_data_base, PERCPU_AREA_SIZE};
use core::ptr::NonNull;
use heap::LockedHeap;

pub(crate) const PAGE_SIZE: usize = 0x1000;
pub(crate) const DATA_OFFSET: usize = 0x1000;

// TODO: 这里的大小还需要与 axhal 模块中的链接脚本中的定义一致
const COPS_HEAP_SIZE: usize = (PAGE_SIZE << 4) * axconfig::SMP;

#[global_allocator]
static ALLOCATOR: Allocator = Allocator::new();

pub struct Allocator {
    heap: LockedHeap<32>,
}

impl Allocator {
    const fn new() -> Self {
        Allocator {
            heap: LockedHeap::empty(),
        }
    }

    pub fn init(&self, start: usize, size: usize) {
        unsafe {
            self.heap.lock().init(start, size);
        }
    }
}

// 由于这里记录的虚拟地址是固定的，因此导致了如果映射到其他的虚拟地址空间中，
// 分配的位置是变化的，这样会导致出现错误，导致所有的地址空间中，这段代码的虚拟地址必须相同
// 有一个办法是将堆的空间从 0 开始，然后每次分配成功后，增加一个偏移量，使得在各自的空间中正常工作
// 需要手动实现分配回收函数，记录了偏移之后，应该怎么样计算出正确的虚拟地址呢？
// 使用 auipc 指令来计算出当前的代码地址，并且通过偏移量来计算出虚拟地址，在加载 vdso 空间时，保证数据段和代码段的页是连续的，
// 但是处于不同地址空间的 vdso 的虚拟地址可以不同
// 另一个问题是，如果数据结构中存放了固定的位置的指针，则仍然会存在这个问题，
// 这需要保证第三方库中不会这样操作，但是 SegQueue 会使用 AtomicPtr 来记录数据块的位置，不满足这种要求
// 因此，可以保证使用的数据段的虚拟地址在每个地址空间是相同的，代码段和只读数据段则可以变化
// 在目前的实现过程中，需要解决的问题是怎么实现一个无锁的任务队列，并且可以在不同的虚拟地址空间中使用
//
// 在中断处理例程里面，不要使用堆相关的东西，按照这套机制，纯软件的实现会使用到堆，但是可以使用一个额外的原子操作来临时存放唤醒的任务
/// buddy system allocator 不可以分配一段虚拟的内存，因为它会写对应的地址，这会导致出现页错误
/// Arceos 使用的 bitmap 分配器不行，只能在页大小的尺寸进行分配
///
pub fn init(percpu_size: usize) {
    let mut percpu_area_size = axconfig::SMP * percpu_size;
    percpu_area_size = (percpu_area_size + PAGE_SIZE - 1) & PAGE_SIZE;
    unsafe { ALLOCATOR.init(DATA_OFFSET, COPS_HEAP_SIZE - percpu_area_size) };
}

/// 获取 vdso 使用的堆分配器的空间的起始地址
/// 保证为 4K 对齐
fn allocator_base() -> usize {
    let data_base = get_data_base();
    let percpu_area_size = axconfig::SMP * unsafe { PERCPU_AREA_SIZE };
    (data_base - DATA_OFFSET + percpu_area_size + PAGE_SIZE - 1) & (!(PAGE_SIZE - 1))
}

unsafe impl core::alloc::GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let ptr = self.heap.lock().alloc(layout).unwrap();
        let res = (ptr.as_ptr() as usize + allocator_base()) as *mut u8;
        res
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        self.heap.lock().dealloc(
            NonNull::new_unchecked((ptr as usize - allocator_base()) as *mut u8),
            layout,
        );
    }
}
