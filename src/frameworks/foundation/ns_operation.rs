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

use super::{NSInteger, NSUInteger};
use crate::objc::{
    id, msg, msg_class, msg_send_no_type_checking, nil, objc_classes, release, retain,
    ClassExports, HostObject, NSZonePtr, SEL,
};

struct NSOperationHostObject {
    cancelled: bool,
    executing: bool,
    finished: bool,
    invocation: Option<OperationInvocation>,
}
impl HostObject for NSOperationHostObject {}

#[derive(Clone, Copy)]
enum OperationInvocation {
    TargetSelectorObject(id, SEL, id),
    NSInvocation(id),
}

struct NSOperationQueueHostObject {
    max_concurrent_operation_count: NSInteger,
}
impl HostObject for NSOperationQueueHostObject {}

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
    match invocation {
        Some(OperationInvocation::TargetSelectorObject(target, selector, object)) => {
            () = msg_send_no_type_checking(env, (target, selector, object));
        }
        Some(OperationInvocation::NSInvocation(invocation)) => {
            () = msg![env; invocation invoke];
        }
        None => {}
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
    if let Some(invocation) = env.objc.borrow::<NSOperationHostObject>(this).invocation {
        match invocation {
            OperationInvocation::TargetSelectorObject(target, _selector, object) => {
                release(env, target);
                release(env, object);
            }
            OperationInvocation::NSInvocation(invocation) => release(env, invocation),
        }
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
        Some(OperationInvocation::TargetSelectorObject(target, selector, object));
    this
}

- (id)initWithInvocation:(id)invocation { // NSInvocation *
    retain(env, invocation);
    env.objc.borrow_mut::<NSOperationHostObject>(this).invocation =
        Some(OperationInvocation::NSInvocation(invocation));
    this
}

- (id)result {
    // Return values are not captured yet. Void invocation operations, which are
    // the common loading use case on early iPhone OS, are fully supported.
    nil
}

@end


@implementation NSOperationQueue: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = NSOperationQueueHostObject {
        // NSOperationQueueDefaultMaxConcurrentOperationCount
        max_concurrent_operation_count: -1,
    };
    env.objc.alloc_object(this, Box::new(host_object), &mut env.mem)
}

- (())setMaxConcurrentOperationCount:(NSInteger)count {
    env.objc
        .borrow_mut::<NSOperationQueueHostObject>(this)
        .max_concurrent_operation_count = count;
}

- (NSInteger)maxConcurrentOperationCount {
    env.objc
        .borrow::<NSOperationQueueHostObject>(this)
        .max_concurrent_operation_count
}

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
