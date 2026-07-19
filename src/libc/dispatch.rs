/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Small parts of Grand Central Dispatch that are exported by libSystem.

use crate::abi::{CallFromHost, GuestFunction};
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{ConstPtr, ConstVoidPtr, MutPtr, SafeRead};
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

    let block = env.mem.read(block_ptr);
    let invoke = block.invoke;
    assert!(invoke.addr_without_thumb_bit() != 0);
    () = invoke.call_from_host(env, (block_ptr.cast_void(),));
}

pub const FUNCTIONS: FunctionExports = &[export_c_func!(dispatch_once(_, _))];

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
