/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */

use crate::dyld::{export_c_func, FunctionExports};
use crate::libc::errno::{set_errno, EINVAL, ENOMEM};
use crate::mem::{ConstPtr, ConstVoidPtr, GuestUSize, MutPtr, MutVoidPtr, Ptr, SafeRead};
use crate::{Environment, ThreadId};
use std::collections::HashMap;

// Darwin's 32-bit stack_t layout and flags come from sys/signal.h:
// https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/signal.h
const SS_ONSTACK: i32 = 0x0001;
const SS_DISABLE: i32 = 0x0004;
// Darwin advertises a larger MINSIGSTKSZ, but its sigaltstack syscall retains
// this older minimum for binary compatibility:
// https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_sig.c
const ENFORCED_MINSIGSTKSZ: GuestUSize = 8 * 1024;

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
struct stack_t {
    ss_sp: MutVoidPtr,
    ss_size: GuestUSize,
    ss_flags: i32,
}
unsafe impl SafeRead for stack_t {}

#[derive(Default)]
pub struct State {
    /// Alternate signal stacks are configured independently for each guest
    /// thread. Signal delivery itself is not implemented yet, so no entry can
    /// currently acquire [SS_ONSTACK].
    alternate_stacks: HashMap<ThreadId, stack_t>,
}

impl State {
    fn disabled_stack() -> stack_t {
        stack_t {
            ss_sp: Ptr::null(),
            ss_size: 0,
            ss_flags: SS_DISABLE,
        }
    }

    fn stack_for_thread(&self, thread: ThreadId) -> stack_t {
        self.alternate_stacks
            .get(&thread)
            .copied()
            .unwrap_or_else(Self::disabled_stack)
    }
}

fn validate_alternate_stack(stack: stack_t) -> Result<(), i32> {
    if stack.ss_flags & SS_ONSTACK != 0 || stack.ss_flags & !SS_DISABLE != 0 {
        return Err(EINVAL);
    }
    if stack.ss_flags == 0 && stack.ss_size < ENFORCED_MINSIGSTKSZ {
        return Err(ENOMEM);
    }
    Ok(())
}

fn sigaction(env: &mut Environment, signum: i32, act: ConstVoidPtr, old_act: MutVoidPtr) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    log!("TODO: sigaction({:?}, {:?}, {:?})", signum, act, old_act);
    0
}

fn signal(env: &mut Environment, signum: i32, handler: MutVoidPtr) -> MutVoidPtr {
    // TODO: handle errno properly
    set_errno(env, 0);

    log!("TODO: signal({:?}, {:?})", signum, handler);
    Ptr::null()
}

fn sigprocmask(env: &mut Environment, how: i32, set: ConstVoidPtr, old_set: MutVoidPtr) -> i32 {
    // TODO: handle errno properly
    set_errno(env, 0);

    log!("TODO: sigprocmask({}, {:?}, {:?})", how, set, old_set);
    0
}

fn sigaltstack(env: &mut Environment, stack: ConstPtr<stack_t>, old_stack: MutPtr<stack_t>) -> i32 {
    set_errno(env, 0);

    let thread = env.current_thread;
    let previous = env.libc_state.signal.stack_for_thread(thread);
    if !old_stack.is_null() {
        env.mem.write(old_stack, previous);
    }

    if stack.is_null() {
        return 0;
    }

    let requested = env.mem.read(stack);
    if let Err(errno) = validate_alternate_stack(requested) {
        set_errno(env, errno);
        return -1;
    }

    if requested.ss_flags == SS_DISABLE {
        env.libc_state.signal.alternate_stacks.remove(&thread);
    } else {
        env.libc_state
            .signal
            .alternate_stacks
            .insert(thread, requested);
    }
    0
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(sigaction(_, _, _)),
    export_c_func!(signal(_, _)),
    export_c_func!(sigprocmask(_, _, _)),
    export_c_func!(sigaltstack(_, _)),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn enabled_stack(size: GuestUSize) -> stack_t {
        stack_t {
            ss_sp: Ptr::from_bits(0x1000),
            ss_size: size,
            ss_flags: 0,
        }
    }

    #[test]
    fn alternate_signal_stack_uses_the_32_bit_darwin_layout() {
        assert_eq!(std::mem::size_of::<stack_t>(), 12);
    }

    #[test]
    fn alternate_signal_stack_validation_matches_darwin_flags_and_minimum() {
        assert_eq!(
            validate_alternate_stack(enabled_stack(ENFORCED_MINSIGSTKSZ)),
            Ok(())
        );
        assert_eq!(
            validate_alternate_stack(enabled_stack(ENFORCED_MINSIGSTKSZ - 1)),
            Err(ENOMEM)
        );

        let mut disabled = enabled_stack(0);
        disabled.ss_flags = SS_DISABLE;
        assert_eq!(validate_alternate_stack(disabled), Ok(()));

        let mut on_stack = enabled_stack(ENFORCED_MINSIGSTKSZ);
        on_stack.ss_flags = SS_ONSTACK;
        assert_eq!(validate_alternate_stack(on_stack), Err(EINVAL));
    }

    #[test]
    fn alternate_signal_stacks_are_disabled_by_default_and_per_thread() {
        let mut state = State::default();
        let initial = state.stack_for_thread(3);
        let initial_size = initial.ss_size;
        let initial_flags = initial.ss_flags;
        assert!(initial.ss_sp.is_null());
        assert_eq!(initial_size, 0);
        assert_eq!(initial_flags, SS_DISABLE);

        state
            .alternate_stacks
            .insert(3, enabled_stack(ENFORCED_MINSIGSTKSZ));
        let thread_3_flags = state.stack_for_thread(3).ss_flags;
        let thread_4_flags = state.stack_for_thread(4).ss_flags;
        assert_eq!(thread_3_flags, 0);
        assert_eq!(thread_4_flags, SS_DISABLE);
    }
}
