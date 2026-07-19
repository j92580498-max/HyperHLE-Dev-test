/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSOperation`, `NSInvocationOperation`, and `NSOperationQueue`.
//!
//! The queue currently executes work synchronously. This is less concurrent
//! than iPhone OS, but it preserves operation ordering and lifecycle semantics
//! while making the common operation-based loading pattern functional.

use super::NSUInteger;
use crate::objc::{
    id, msg, msg_class, msg_send_no_type_checking, nil, objc_classes, release, retain,
    ClassExports, HostObject, NSZonePtr, SEL,
};

struct NSOperationHostObject {
    cancelled: bool,
    executing: bool,
    finished: bool,
    invocation: Option<(id, SEL, id)>,
}
impl HostObject for NSOperationHostObject {}

fn alloc_operation(env: &mut crate::Environment, class: crate::objc::Class) -> id {
    let host_object = NSOperationHostObject {
        cancelled: false,
        executing: false,
        finished: false,
        invocation: None,
    };
    env.objc
        .alloc_object(class, Box::new(host_object), &mut env.mem)
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSOperation: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    alloc_operation(env, this)
}

- (())start {
    let cancelled = env.objc.borrow::<NSOperationHostObject>(this).cancelled;
    if cancelled {
        env.objc.borrow_mut::<NSOperationHostObject>(this).finished = true;
        return;
    }

    env.objc.borrow_mut::<NSOperationHostObject>(this).executing = true;
    () = msg![env; this main];
    let host = env.objc.borrow_mut::<NSOperationHostObject>(this);
    host.executing = false;
    host.finished = true;
}

- (())main {
    let invocation = env.objc.borrow::<NSOperationHostObject>(this).invocation;
    if let Some((target, selector, object)) = invocation {
        () = msg_send_no_type_checking(env, (target, selector, object));
    }
}

- (())cancel {
    env.objc.borrow_mut::<NSOperationHostObject>(this).cancelled = true;
}

- (bool)isCancelled {
    env.objc.borrow::<NSOperationHostObject>(this).cancelled
}

- (bool)isExecuting {
    env.objc.borrow::<NSOperationHostObject>(this).executing
}

- (bool)isFinished {
    env.objc.borrow::<NSOperationHostObject>(this).finished
}

- (bool)isConcurrent {
    false
}

- (bool)isReady {
    !env.objc.borrow::<NSOperationHostObject>(this).executing
}

- (())waitUntilFinished {
    // Operations are currently run synchronously, so there is nothing to wait for.
}

- (())dealloc {
    if let Some((target, _selector, object)) =
        env.objc.borrow::<NSOperationHostObject>(this).invocation
    {
        release(env, target);
        release(env, object);
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

@end

@implementation NSInvocationOperation: NSOperation

+ (id)allocWithZone:(NSZonePtr)_zone {
    alloc_operation(env, this)
}

- (id)initWithTarget:(id)target selector:(SEL)selector object:(id)object {
    retain(env, target);
    retain(env, object);
    env.objc.borrow_mut::<NSOperationHostObject>(this).invocation =
        Some((target, selector, object));
    this
}

- (id)result {
    // Return values are not captured yet. Void invocation operations, which are
    // the common loading use case on early iPhone OS, are fully supported.
    nil
}

@end


@implementation NSOperationQueue: NSObject

- (())addOperation:(id)operation {
    retain(env, operation);
    () = msg![env; operation start];
    release(env, operation);
}

- (id)operations {
    // Completed synchronous operations leave the queue immediately.
    msg_class![env; NSArray array]
}

- (NSUInteger)operationCount {
    0
}

- (())cancelAllOperations {}

- (())waitUntilAllOperationsAreFinished {}

@end


};
