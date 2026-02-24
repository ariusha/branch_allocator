//! Testing for the entire crate. This module is aware of only the crate root module and the target
//! module, and uses only the public interface for testing.
extern crate std;
use crate::{BranchAllocator, target::Atomic};
use std::vec::Vec;
fn create(order: usize) -> BranchAllocator<'static> {
    let required = BranchAllocator::required(order);
    let mut storage = Vec::with_capacity(required);
    for _ in 0..storage.capacity() {
        storage.push(Atomic::new(0));
    }
    BranchAllocator::new(Vec::leak(storage), order).expect("failed to create allocator!")
}
fn allocate_ok(alloc: &BranchAllocator, index: usize, order: usize) {
    assert!(
        alloc.try_allocate(index, order).is_some(),
        "failed to allocate {}-ordered region containing index {}",
        order,
        index
    );
}
fn allocate_fails(alloc: &BranchAllocator, index: usize, order: usize) {
    assert!(
        alloc.try_allocate(index, order).is_none(),
        "unexpectedly allocated {}-ordered region containing index {}",
        order,
        index
    );
}
fn deallocate_ok(alloc: &BranchAllocator, index: usize, order: usize) {
    assert!(
        alloc.deallocate(index, order).is_some(),
        "failed to deallocate {}-ordered region containing index {}",
        order,
        index
    );
}
fn deallocate_fails(alloc: &BranchAllocator, index: usize, order: usize) {
    assert!(
        alloc.deallocate(index, order).is_none(),
        "unexpectedly deallocated {}-ordered region containing index {}",
        order,
        index
    );
}
#[test]
fn create_allocator() {
    let order = 10;
    let alloc = create(order);
    assert_eq!(alloc.order, order);
}
#[test]
fn allocate_and_free_single_block() {
    let alloc = create(4);
    let block = 5;
    allocate_ok(&alloc, block, 0);
    allocate_fails(&alloc, block, 0);
    deallocate_ok(&alloc, block, 0);
    allocate_ok(&alloc, block, 0);
}
#[test]
fn allocate_stem_node() {
    let alloc = create(4);
    let block = 0;
    allocate_ok(&alloc, block, 3);
    let leaf_inside = 3;
    allocate_fails(&alloc, leaf_inside, 0);
    deallocate_ok(&alloc, block, 3);
    allocate_ok(&alloc, leaf_inside, 0);
}
#[test]
fn range_and_order_mismatch() {
    let alloc = create(4);
    allocate_fails(&alloc, 0, 5);
    allocate_ok(&alloc, 0, 0);
}
#[test]
fn out_of_bounds_index() {
    let alloc = create(4);
    allocate_fails(&alloc, 16, 0);
    deallocate_fails(&alloc, 16, 0);
}
#[test]
fn multiple_allocations_and_frees() {
    let alloc = create(5);
    let blocks: Vec<usize> = (0..32).collect();
    for &f in &blocks {
        allocate_ok(&alloc, f, 0);
    }
    for &f in &blocks {
        allocate_fails(&alloc, f, 0);
    }
    for &f in blocks.iter().step_by(2) {
        deallocate_ok(&alloc, f, 0);
    }
    for &f in blocks.iter().step_by(2) {
        allocate_ok(&alloc, f, 0);
    }
}
#[test]
fn coalescing_across_branches() {
    let alloc = create(5); // 32 blocks
    let left = 0; // region of order 3 covering blocks 0..7
    let right = 8; // region of order 3 covering blocks 8..15
    let parent = 0; // region of order 4 covering blocks 0..15
    allocate_ok(&alloc, left, 3);
    allocate_ok(&alloc, right, 3);
    deallocate_ok(&alloc, left, 3);
    deallocate_ok(&alloc, right, 3);
    allocate_ok(&alloc, parent, 4);
    allocate_fails(&alloc, left, 3);
    allocate_fails(&alloc, right, 3);
}
#[test]
fn concurrent_allocation() {
    use std::thread;
    let alloc = create(8); // 256 blocks
    let blocks: Vec<usize> = (0..256).collect();
    let alloc_ref = &alloc;
    thread::scope(|s| {
        for i in 0..8 {
            let block = blocks[i * 32];
            s.spawn(move || {
                for _ in 0..100 {
                    allocate_ok(alloc_ref, block, 0);
                    deallocate_ok(alloc_ref, block, 0);
                }
            });
        }
    });
}
#[test]
fn out_of_range_indices() {
    let alloc = create(4);
    allocate_fails(&alloc, 16, 0);
    deallocate_fails(&alloc, 16, 0);
    allocate_fails(&alloc, 1_000_000, 0);
    deallocate_fails(&alloc, 1_000_000, 0);
}
#[test]
fn invalid_orders() {
    let alloc = create(4);
    allocate_fails(&alloc, 0, 5);
    deallocate_fails(&alloc, 0, 5);
    allocate_fails(&alloc, 0, 10);
    allocate_ok(&alloc, 0, 0);
}
#[test]
fn double_allocation() {
    let alloc = create(4);
    let block = 0;
    allocate_ok(&alloc, block, 0);
    allocate_fails(&alloc, block, 0);
}
#[test]
fn double_deallocation() {
    let alloc = create(4);
    let block = 0;
    deallocate_fails(&alloc, block, 0);
    allocate_ok(&alloc, block, 0);
    deallocate_ok(&alloc, block, 0);
    deallocate_fails(&alloc, block, 0);
}
#[test]
fn allocate_with_index_not_matching_order() {
    let alloc = create(4);
    allocate_ok(&alloc, 0, 2); // region of blocks 0..3
    allocate_fails(&alloc, 0, 2); // same region already allocated
    allocate_fails(&alloc, 1, 2); // still inside that region
}
#[test]
fn deallocate_with_wrong_order() {
    let alloc = create(4);
    let block = 0;
    allocate_ok(&alloc, block, 0);
    deallocate_fails(&alloc, block, 1);
    deallocate_ok(&alloc, block, 0);
}
#[test]
fn allocate_after_partial_free() {
    let alloc = create(5);
    let stem_block = 0;
    allocate_ok(&alloc, stem_block, 3);
    let leaf_inside = 3;
    deallocate_fails(&alloc, leaf_inside, 0);
    deallocate_ok(&alloc, stem_block, 3);
    allocate_ok(&alloc, leaf_inside, 0);
}
#[test]
fn concurrent_mixed_operations() {
    use std::thread;
    let alloc = create(8);
    let alloc_ref = &alloc;
    let blocks: Vec<usize> = (0..256).collect();
    thread::scope(|s| {
        for i in 0..8 {
            let block = blocks[i * 32];
            s.spawn(move || {
                for _ in 0..50 {
                    // valid operations
                    let _ = alloc_ref.try_allocate(block, 0);
                    let _ = alloc_ref.deallocate(block, 0);
                    // invalid index
                    let _ = alloc_ref.try_allocate(9999, 0);
                    let _ = alloc_ref.deallocate(9999, 0);
                    // invalid order
                    let _ = alloc_ref.try_allocate(block, 10);
                    let _ = alloc_ref.deallocate(block, 10);
                }
            });
        }
    });
}
#[test]
fn allocate_all_blocks_then_free_all() {
    let alloc = create(6); // 64 blocks
    for f in 0..64 {
        allocate_ok(&alloc, f, 0);
    }
    for f in 0..64 {
        deallocate_ok(&alloc, f, 0);
    }
    allocate_ok(&alloc, 0, 6);
    deallocate_ok(&alloc, 0, 6);
}
#[test]
fn allocate_large_blocks_interleaved() {
    let alloc = create(6); // 64 blocks
    for &base in &[0, 16, 32, 48] {
        allocate_ok(&alloc, base, 4);
    }
    allocate_fails(&alloc, 0, 5);
    deallocate_ok(&alloc, 0, 4);
    deallocate_ok(&alloc, 16, 4);
    allocate_ok(&alloc, 0, 5);
}
#[test]
fn allocate_max_order() {
    let alloc = create(10); // 1024 blocks
    allocate_ok(&alloc, 0, 10);
    allocate_fails(&alloc, 500, 0);
    deallocate_ok(&alloc, 0, 10);
    allocate_ok(&alloc, 500, 0);
}
#[test]
fn stress_random_ops() {
    use std::thread;
    let alloc = create(8);
    let blocks: Vec<usize> = (0..256).collect();
    let blocks_ref = &blocks;
    let orders = [0, 1, 2, 3, 4, 5, 6, 7, 8];
    fn xorshift(state: &mut u32) -> u32 {
        let mut x = *state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        *state = x;
        x
    }
    thread::scope(|s| {
        for _ in 0..4 {
            let alloc_ref = &alloc;
            s.spawn(move || {
                let mut rng = 123456789;
                for _ in 0..100 {
                    let f_idx = (xorshift(&mut rng) as usize) % blocks_ref.len();
                    let f = blocks_ref[f_idx];
                    let o_idx = (xorshift(&mut rng) as usize) % orders.len();
                    let o = orders[o_idx];
                    if xorshift(&mut rng) % 2 == 0 {
                        let _ = alloc_ref.try_allocate(f, o);
                    } else {
                        let _ = alloc_ref.deallocate(f, o);
                    }
                }
            });
        }
    });
}
