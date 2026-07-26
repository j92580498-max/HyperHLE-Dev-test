/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! The ARC runtime entry points.
//!
//! Automatic Reference Counting does not change how objects are counted — it
//! is the same `retain`/`release`/`autorelease` — it just has the compiler
//! emit calls to these functions instead of message sends. So each of these is
//! a thin wrapper over the corresponding helper.
//!
//! Every one of them is nil-tolerant. ARC emits these calls unconditionally
//! around values that may well be nil, so a nil check is not defensive
//! programming here, it is the specified behavior.
//!
//! The autoreleased-return-value pair is where a real runtime is clever: the
//! callee hands back an autoreleased object and the caller immediately retains
//! it, and the runtime elides both using a thread-local handshake. Eliding is
//! an optimisation, not a semantic, so this implements the straightforward
//! version: autorelease on the way out, retain on the way in. The result is the
//! same object graph, with one extra trip through the autorelease pool.
//!
//! Resources:
//! - Clang's [ARC specification](https://clang.llvm.org/docs/AutomaticReferenceCounting.html#runtime-support)

use super::{autorelease, id, nil, release, retain};
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::MutPtr;
use crate::Environment;

fn objc_retain(env: &mut Environment, object: id) -> id {
    retain(env, object)
}

fn objc_release(env: &mut Environment, object: id) {
    release(env, object)
}

fn objc_autorelease(env: &mut Environment, object: id) -> id {
    autorelease(env, object)
}

fn objc_retainAutorelease(env: &mut Environment, object: id) -> id {
    let object = retain(env, object);
    autorelease(env, object)
}

/// Emitted at the end of a function returning an object at +0.
fn objc_autoreleaseReturnValue(env: &mut Environment, object: id) -> id {
    autorelease(env, object)
}

/// Emitted at the end of a function returning an object the caller must not
/// own, when the value came from a +1 expression.
fn objc_retainAutoreleaseReturnValue(env: &mut Environment, object: id) -> id {
    let object = retain(env, object);
    autorelease(env, object)
}

/// Emitted at a call site that receives an autoreleased object and wants to own
/// it.
fn objc_retainAutoreleasedReturnValue(env: &mut Environment, object: id) -> id {
    retain(env, object)
}

/// Emitted for a `__strong` variable or ivar assignment.
///
/// The new value is retained before the old one is released, so assigning a
/// value that is already stored there cannot destroy it.
fn objc_storeStrong(env: &mut Environment, location: MutPtr<id>, object: id) {
    if location.is_null() {
        return;
    }
    let old = env.mem.read(location);
    if old == object {
        return;
    }
    let object = retain(env, object);
    env.mem.write(location, object);
    release(env, old);
}

/// `__unsafe_unretained` and `__weak` loads. tapHLE has no zeroing-weak
/// support, so a weak reference behaves as unsafe-unretained: it is read
/// straight through and is not zeroed when the object dies.
fn objc_loadWeak(env: &mut Environment, location: MutPtr<id>) -> id {
    if location.is_null() {
        return nil;
    }
    let object = env.mem.read(location);
    autorelease(env, object)
}

fn objc_loadWeakRetained(env: &mut Environment, location: MutPtr<id>) -> id {
    if location.is_null() {
        return nil;
    }
    let object = env.mem.read(location);
    retain(env, object)
}

fn objc_storeWeak(env: &mut Environment, location: MutPtr<id>, object: id) -> id {
    if location.is_null() {
        return nil;
    }
    // Not retained: this models __unsafe_unretained, so the caller is
    // responsible for not reading it after the object dies.
    env.mem.write(location, object);
    object
}

fn objc_initWeak(env: &mut Environment, location: MutPtr<id>, object: id) -> id {
    objc_storeWeak(env, location, object)
}

fn objc_destroyWeak(env: &mut Environment, location: MutPtr<id>) {
    if location.is_null() {
        return;
    }
    env.mem.write(location, nil);
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(objc_retain(_)),
    export_c_func!(objc_release(_)),
    export_c_func!(objc_autorelease(_)),
    export_c_func!(objc_retainAutorelease(_)),
    export_c_func!(objc_autoreleaseReturnValue(_)),
    export_c_func!(objc_retainAutoreleaseReturnValue(_)),
    export_c_func!(objc_retainAutoreleasedReturnValue(_)),
    export_c_func!(objc_storeStrong(_, _)),
    export_c_func!(objc_loadWeak(_)),
    export_c_func!(objc_loadWeakRetained(_)),
    export_c_func!(objc_storeWeak(_, _)),
    export_c_func!(objc_initWeak(_, _)),
    export_c_func!(objc_destroyWeak(_)),
];
