/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UIAlertView`.
//!
//! tapHLE cannot draw an alert, so `-show` logs the alert's text and then
//! immediately reports it as dismissed via the **cancel** button. Doing nothing
//! is not a neutral choice: an app that shows an alert is usually waiting for
//! `alertView:clickedButtonAtIndex:` before it continues, so a silent alert
//! stalls it. Cancel is the conservative answer — it is what a user declining
//! an unexpected prompt would choose, and it never confirms a purchase, a
//! deletion or a network action on their behalf.

use crate::frameworks::foundation::{ns_string, NSInteger};
use crate::objc::{
    id, impl_HostObject_with_superclass, msg, msg_send, msg_super, nil, objc_classes, release,
    retain, ClassExports, NSZonePtr,
};
use crate::Environment;
use std::borrow::Cow;

#[derive(Default)]
struct UIAlertViewHostObject {
    superclass: super::UIViewHostObject,
    /// Weak, as delegates always are.
    delegate: id,
    /// How many buttons have been configured. The cancel button, when there is
    /// one, is always the first.
    button_count: NSInteger,
}
impl_HostObject_with_superclass!(UIAlertViewHostObject);

/// Send an optional `(alertView, buttonIndex)` delegate method, if implemented.
fn send_index_callback(
    env: &mut Environment,
    delegate: id,
    selector: &str,
    alert: id,
    index: NSInteger,
) {
    if delegate == nil {
        return;
    }
    let Some(sel) = env.objc.lookup_selector(selector) else {
        return;
    };
    let responds: bool = msg![env; delegate respondsToSelector:sel];
    if !responds {
        return;
    }
    () = msg_send(env, (delegate, sel, alert, index));
}

/// Report the alert as dismissed by `index`, in the order UIKit does.
fn dismiss(env: &mut Environment, alert: id, index: NSInteger, clicked: bool) {
    let delegate = env.objc.borrow::<UIAlertViewHostObject>(alert).delegate;
    // Keep the alert alive across the callbacks: a delegate commonly releases
    // it in the first one it receives.
    retain(env, alert);
    if clicked {
        send_index_callback(
            env,
            delegate,
            "alertView:clickedButtonAtIndex:",
            alert,
            index,
        );
    }
    send_index_callback(
        env,
        delegate,
        "alertView:willDismissWithButtonIndex:",
        alert,
        index,
    );
    send_index_callback(
        env,
        delegate,
        "alertView:didDismissWithButtonIndex:",
        alert,
        index,
    );
    release(env, alert);
}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UIAlertView: UIView

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UIAlertViewHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithTitle:(id)title
                      message:(id)message
                     delegate:(id)delegate
            cancelButtonTitle:(id)cancelButtonTitle
            otherButtonTitles:(id)otherButtonTitles {
    let message_str = if message == nil { Cow::from("(nil)") } else { ns_string::to_rust_string(env, message) };
    let title_str = if title == nil { Cow::from("(nil)") } else { ns_string::to_rust_string(env, title) };
    log!("UIAlertView: title: {:?}, message: {:?}", title_str, message_str);

    // TODO: otherButtonTitles is a nil-terminated variadic list. Only the
    // cancel button's presence is tracked, which is what dismissal needs.
    let _ = otherButtonTitles;

    let new: id = msg_super![env; this init];
    let host_object = env.objc.borrow_mut::<UIAlertViewHostObject>(new);
    host_object.delegate = delegate;
    host_object.button_count = if cancelButtonTitle == nil { 0 } else { 1 };
    new
}

- (id)delegate {
    env.objc.borrow::<UIAlertViewHostObject>(this).delegate
}
- (())setDelegate:(id)delegate {
    env.objc.borrow_mut::<UIAlertViewHostObject>(this).delegate = delegate;
}

- (NSInteger)numberOfButtons {
    env.objc.borrow::<UIAlertViewHostObject>(this).button_count
}

- (NSInteger)cancelButtonIndex {
    if env.objc.borrow::<UIAlertViewHostObject>(this).button_count > 0 {
        0
    } else {
        -1
    }
}

- (bool)isVisible {
    // Never actually displayed, so never visible.
    false
}

- (())addButtonWithTitle:(id)title {
    log!("UIAlertView: button: {:?}", ns_string::to_rust_string(env, title));
    env.objc.borrow_mut::<UIAlertViewHostObject>(this).button_count += 1;
}

- (())show {
    log!("UIAlertView: cannot be displayed; reporting it as cancelled");
    let cancel_index: NSInteger = msg![env; this cancelButtonIndex];
    dismiss(env, this, cancel_index, /* clicked: */ true);
}

- (())dismissWithClickedButtonIndex:(NSInteger)index animated:(bool)_animated {
    dismiss(env, this, index, /* clicked: */ false);
}

@end

};
