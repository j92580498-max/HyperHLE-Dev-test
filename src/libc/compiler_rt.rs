/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Compiler builtins from libgcc / compiler-rt, and the C++ ABI's allocation
//! operators.
//!
//! The ARM cores these apps target have no integer divide instruction, so the
//! compiler turns every `/` and `%` on an integer into a call to one of the
//! helpers below. That makes them unusually load-bearing: an app missing them
//! does not fail at some optional feature, it fails at arithmetic. A survey of
//! 1501 apps found `__udivsi3` alone blocking fourteen.
//!
//! Resources:
//! - The [Run-time ABI for the ARM Architecture](https://github.com/ARM-software/abi-aa/blob/main/rtabi32/rtabi32.rst)
//!   defines the `__aeabi_*` spellings and their register conventions.

use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{GuestUSize, MutVoidPtr};
use crate::Environment;

/// Division by zero is undefined in C, and the real helpers do not check for it
/// either — on these cores it traps or returns garbage depending on the build.
/// Returning zero keeps the emulator alive and is no less defined than what the
/// hardware would have done, which matters because the caller is often a
/// third-party library doing arithmetic on data tapHLE handed it.
fn divide_or_zero<T: Default>(divisor_is_zero: bool, compute: impl FnOnce() -> T) -> T {
    if divisor_is_zero {
        log_once!("Warning: integer division by zero in guest code, returning 0");
        return T::default();
    }
    compute()
}

fn __udivsi3(_env: &mut Environment, numerator: u32, denominator: u32) -> u32 {
    divide_or_zero(denominator == 0, || numerator / denominator)
}

fn __umodsi3(_env: &mut Environment, numerator: u32, denominator: u32) -> u32 {
    divide_or_zero(denominator == 0, || numerator % denominator)
}

/// Signed division truncates towards zero in C, which is what Rust's `/` does,
/// so this is a direct mapping — except for `INT_MIN / -1`, whose true quotient
/// is not representable and which would panic in Rust where C leaves it
/// undefined. Wrapping matches what the hardware produces.
fn __divsi3(_env: &mut Environment, numerator: i32, denominator: i32) -> i32 {
    divide_or_zero(denominator == 0, || numerator.wrapping_div(denominator))
}

fn __modsi3(_env: &mut Environment, numerator: i32, denominator: i32) -> i32 {
    divide_or_zero(denominator == 0, || numerator.wrapping_rem(denominator))
}

fn __udivdi3(_env: &mut Environment, numerator: u64, denominator: u64) -> u64 {
    divide_or_zero(denominator == 0, || numerator / denominator)
}

fn __umoddi3(_env: &mut Environment, numerator: u64, denominator: u64) -> u64 {
    divide_or_zero(denominator == 0, || numerator % denominator)
}

fn __divdi3(_env: &mut Environment, numerator: i64, denominator: i64) -> i64 {
    divide_or_zero(denominator == 0, || numerator.wrapping_div(denominator))
}

fn __moddi3(_env: &mut Environment, numerator: i64, denominator: i64) -> i64 {
    divide_or_zero(denominator == 0, || numerator.wrapping_rem(denominator))
}

// The ARM run-time ABI spellings the compiler emits directly.

fn __aeabi_uidiv(env: &mut Environment, numerator: u32, denominator: u32) -> u32 {
    __udivsi3(env, numerator, denominator)
}

fn __aeabi_idiv(env: &mut Environment, numerator: i32, denominator: i32) -> i32 {
    __divsi3(env, numerator, denominator)
}

/// `__aeabi_uidivmod` returns the quotient in r0 and the remainder in r1. On
/// little-endian AAPCS a 64-bit return occupies exactly that pair, low half
/// first, so packing the remainder into the high half places each where the
/// caller expects it.
fn __aeabi_uidivmod(env: &mut Environment, numerator: u32, denominator: u32) -> u64 {
    let quotient = __udivsi3(env, numerator, denominator);
    let remainder = __umodsi3(env, numerator, denominator);
    ((remainder as u64) << 32) | quotient as u64
}

fn __aeabi_idivmod(env: &mut Environment, numerator: i32, denominator: i32) -> u64 {
    let quotient = __divsi3(env, numerator, denominator) as u32;
    let remainder = __modsi3(env, numerator, denominator) as u32;
    ((remainder as u64) << 32) | quotient as u64
}

// The C++ ABI's allocation operators. C++ code in these apps is usually a game
// engine or a third-party SDK, and it allocates before it does anything else.
//
// The throwing forms are documented to throw std::bad_alloc rather than return
// null, and the nothrow forms to return null. tapHLE's allocator does not fail,
// so the distinction never arises and both simply allocate.

fn _Znwm(env: &mut Environment, size: GuestUSize) -> MutVoidPtr {
    // C++ requires that `new` with a zero size still return a distinct,
    // non-null pointer.
    env.mem.alloc(size.max(1))
}

fn _Znam(env: &mut Environment, size: GuestUSize) -> MutVoidPtr {
    _Znwm(env, size)
}

fn _ZdlPv(env: &mut Environment, ptr: MutVoidPtr) {
    if !ptr.is_null() {
        env.mem.free(ptr);
    }
}

fn _ZdaPv(env: &mut Environment, ptr: MutVoidPtr) {
    _ZdlPv(env, ptr)
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(__udivsi3(_, _)),
    export_c_func!(__umodsi3(_, _)),
    export_c_func!(__divsi3(_, _)),
    export_c_func!(__modsi3(_, _)),
    export_c_func!(__udivdi3(_, _)),
    export_c_func!(__umoddi3(_, _)),
    export_c_func!(__divdi3(_, _)),
    export_c_func!(__moddi3(_, _)),
    export_c_func!(__aeabi_uidiv(_, _)),
    export_c_func!(__aeabi_idiv(_, _)),
    export_c_func!(__aeabi_uidivmod(_, _)),
    export_c_func!(__aeabi_idivmod(_, _)),
    export_c_func!(_Znwm(_)),
    export_c_func!(_Znam(_)),
    export_c_func!(_ZdlPv(_)),
    export_c_func!(_ZdaPv(_)),
];
