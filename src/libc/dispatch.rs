/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Small parts of Grand Central Dispatch that are exported by libSystem.

use crate::abi::{CallFromHost, GuestFunction};
use crate::dyld::{export_c_func, ConstantExports, FunctionExports, HostConstant};
use crate::mem::{ConstPtr, ConstVoidPtr, MutPtr, MutVoidPtr, SafeRead};
use crate::Environment;

/// The stable prefix of a 32-bit Apple block literal.
///
/// `dispatch_once` only needs the hidden block argument and its invoke
/// function. Captured values, when present, follow this prefix and remain in
/// guest memory for the invoke function to read.
#[repr(C, packed)]
struct BlockLiteral {
    _isa: ConstVoidPtr,
    _flags: u32,
    _reserved: u32,
    invoke: GuestFunction,
}
unsafe impl SafeRead for BlockLiteral {}

const DISPATCH_ONCE_DONE: i32 = -1;

#[derive(Default)]
pub struct State {
    main_queue: Option<MutVoidPtr>,
}

fn invoke_block(env: &mut Environment, block_ptr: ConstPtr<BlockLiteral>) {
    let block = env.mem.read(block_ptr);
    let invoke = block.invoke;
    assert!(invoke.addr_without_thumb_bit() != 0);
    () = invoke.call_from_host(env, (block_ptr.cast_void(),));
}

fn claim_once(predicate: &mut i32) -> bool {
    if *predicate != 0 {
        return false;
    }

    // tapHLE executes this callback synchronously. Mark it before entering
    // guest code so a nested call using the same token cannot run it twice.
    *predicate = DISPATCH_ONCE_DONE;
    true
}

fn dispatch_once(
    env: &mut Environment,
    predicate_ptr: MutPtr<i32>,
    block_ptr: ConstPtr<BlockLiteral>,
) {
    let mut predicate = env.mem.read(predicate_ptr);
    if !claim_once(&mut predicate) {
        return;
    }
    env.mem.write(predicate_ptr, predicate);

    invoke_block(env, block_ptr);
}

fn new_queue(env: &mut Environment) -> MutVoidPtr {
    env.mem.alloc_and_write(0_u32).cast_void()
}

fn main_queue(env: &mut Environment) -> MutVoidPtr {
    if let Some(queue) = env.libc_state.dispatch.main_queue {
        return queue;
    }
    let queue = new_queue(env);
    env.libc_state.dispatch.main_queue = Some(queue);
    queue
}

fn dispatch_queue_create(
    env: &mut Environment,
    _label: ConstPtr<u8>,
    _attr: ConstVoidPtr,
) -> MutVoidPtr {
    new_queue(env)
}

fn dispatch_get_main_queue(env: &mut Environment) -> MutVoidPtr {
    main_queue(env)
}

fn dispatch_async(env: &mut Environment, _queue: ConstVoidPtr, block_ptr: ConstPtr<BlockLiteral>) {
    // Queue scheduling is not modeled yet. Running work immediately preserves
    // ordering and avoids pretending that an unimplemented worker exists.
    invoke_block(env, block_ptr);
}

fn dispatch_sync(env: &mut Environment, _queue: ConstVoidPtr, block_ptr: ConstPtr<BlockLiteral>) {
    invoke_block(env, block_ptr);
}

fn dispatch_release(_env: &mut Environment, _object: ConstVoidPtr) {
    // Queue storage is intentionally process-lifetime for now. Dispatch
    // objects can be shared, and freeing an opaque pointer here would make a
    // later synchronous compatibility call use dangling guest memory.
}

pub const CONSTANTS: ConstantExports = &[(
    "__dispatch_main_q",
    HostConstant::Custom(|env| main_queue(env).cast_const()),
)];

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(dispatch_once(_, _)),
    export_c_func!(dispatch_queue_create(_, _)),
    export_c_func!(dispatch_get_main_queue()),
    export_c_func!(dispatch_async(_, _)),
    export_c_func!(dispatch_sync(_, _)),
    export_c_func!(dispatch_release(_)),
];

#[cfg(test)]
mod tests {
    use super::{claim_once, DISPATCH_ONCE_DONE};

    #[test]
    fn dispatch_once_claims_an_initial_token_once() {
        let mut predicate = 0;

        assert!(claim_once(&mut predicate));
        assert_eq!(predicate, DISPATCH_ONCE_DONE);
        assert!(!claim_once(&mut predicate));
    }

    #[test]
    fn dispatch_once_does_not_change_a_completed_token() {
        let mut predicate = DISPATCH_ONCE_DONE;

        assert!(!claim_once(&mut predicate));
        assert_eq!(predicate, DISPATCH_ONCE_DONE);
    }
}
