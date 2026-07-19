/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `UINavigationController`.

use crate::frameworks::foundation::ns_string::get_static_str;
use crate::frameworks::foundation::{ns_array, NSUInteger};
use crate::frameworks::uikit::ui_application::UIInterfaceOrientation;
use crate::objc::{
    autorelease, id, impl_HostObject_with_superclass, msg, msg_super, nil, objc_classes, release,
    retain, ClassExports, NSZonePtr, SEL,
};

// TODO: navigation bar and toolbar
// TODO: animations

#[derive(Default)]
struct UINavigationControllerHostObject {
    superclass: super::UIViewControllerHostObject,
    /// something implementing UINavigationControllerDelegate
    delegate: id,
    /// Navigation stack of view controllers, non-retaining
    /// (we explicitly retain/release on push/pop messages)
    navigation_stack: Vec<id>,
    /// Navigation bar restored from a NIB, retained.
    navigation_bar: id,
    navigation_bar_hidden: bool,
}
impl_HostObject_with_superclass!(UINavigationControllerHostObject);

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation UINavigationController: UIViewController

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::<UINavigationControllerHostObject>::default();
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (id)initWithCoder:(id)coder {
    let this: id = msg_super![env; this initWithCoder:coder];

    // Early UIKit archives may contain both keys with equivalent controller
    // arrays. Decode one authoritative representation so child -> parent
    // back-references do not recursively instantiate this controller.
    let view_controllers_key = get_static_str(env, "UIViewControllers");
    let child_view_controllers_key = get_static_str(env, "UIChildViewControllers");
    let controllers: id = if msg![env; coder containsValueForKey:view_controllers_key] {
        msg![env; coder decodeObjectForKey:view_controllers_key]
    } else {
        msg![env; coder decodeObjectForKey:child_view_controllers_key]
    };

    let mut navigation_stack = Vec::new();
    if controllers != nil {
        let count: NSUInteger = msg![env; controllers count];
        navigation_stack.reserve(count as usize);
        for i in 0..count {
            let controller: id = msg![env; controllers objectAtIndex:i];
            retain(env, controller);
            super::set_parent_view_controller(env, controller, this);
            navigation_stack.push(controller);
        }
    }

    let navigation_bar_key = get_static_str(env, "UINavigationBar");
    let navigation_bar: id = msg![env; coder decodeObjectForKey:navigation_bar_key];
    retain(env, navigation_bar);

    let navigation_bar_hidden_key = get_static_str(env, "UINavigationBarHidden");
    let navigation_bar_hidden: bool =
        msg![env; coder decodeBoolForKey:navigation_bar_hidden_key];
    if navigation_bar != nil {
        () = msg![env; navigation_bar setHidden:navigation_bar_hidden];
    }

    let host_object = env.objc.borrow_mut::<UINavigationControllerHostObject>(this);
    assert!(host_object.navigation_stack.is_empty());
    assert!(host_object.navigation_bar == nil);
    host_object.navigation_stack = navigation_stack;
    host_object.navigation_bar = navigation_bar;
    host_object.navigation_bar_hidden = navigation_bar_hidden;

    this
}

- (())dealloc {
    let (navigation_stack, navigation_bar) = {
        let host_object = env.objc.borrow_mut::<UINavigationControllerHostObject>(this);
        (
            std::mem::take(&mut host_object.navigation_stack),
            std::mem::replace(&mut host_object.navigation_bar, nil),
        )
    };

    for controller in navigation_stack {
        let parent = env
            .objc
            .borrow::<super::UIViewControllerHostObject>(controller)
            .parent_view_controller;
        if parent == this {
            super::set_parent_view_controller(env, controller, nil);
        }
        release(env, controller);
    }
    release(env, navigation_bar);

    msg_super![env; this dealloc]
}

- (id)initWithRootViewController:(id)root_vc { // UIViewController *
    () = msg![env; this pushViewController:root_vc animated:false];
    this
}

- (())loadView {
    // Restore only the model during initWithCoder:. Loading the top child here
    // keeps view creation and appearance callbacks out of recursive NIB
    // decoding while making the archived hierarchy visible on first use.
    () = msg_super![env; this loadView];

    let (self_view, top_view_controller) = {
        let host_object = env.objc.borrow::<UINavigationControllerHostObject>(this);
        (
            host_object.superclass.view,
            host_object.navigation_stack.last().copied(),
        )
    };
    if let Some(top_view_controller) = top_view_controller {
        let view: id = msg![env; top_view_controller view];
        () = msg![env; top_view_controller viewWillAppear:false];
        () = msg![env; self_view addSubview:view];
        () = msg![env; top_view_controller viewDidAppear:false];
    }
}

// weak/non-retaining
- (())setDelegate:(id)delegate { // something implementing UINavigationControllerDelegate
    log_dbg!("[(UINavigationController*){:?} setDelegate:{:?}]", this, delegate);
    let host_object = env.objc.borrow_mut::<UINavigationControllerHostObject>(this);
    host_object.delegate = delegate;
}
- (id)delegate {
    env.objc.borrow::<UINavigationControllerHostObject>(this).delegate
}

- (())pushViewController:(id)view_controller // UIViewController *
                animated:(bool)_animated {
    // Load the container before changing the stack. For an ordinary
    // initWithRootViewController: this prevents loadView from mistaking the
    // newly pushed controller for an archived controller that still needs to
    // be attached.
    let self_view: id = msg![env; this view];

    let stack = &mut env.objc.borrow_mut::<UINavigationControllerHostObject>(this).navigation_stack;
    assert!(!stack.contains(&view_controller));
    stack.push(view_controller);
    retain(env, view_controller);
    super::set_parent_view_controller(env, view_controller, this);

    let delegate = env.objc.borrow::<UINavigationControllerHostObject>(this).delegate;
    let sel: SEL = env
        .objc
        .register_host_selector(
            "navigationController:willShowViewController:animated:".to_string(),
            &mut env.mem
        );
    let responds: bool = msg![env; delegate respondsToSelector:sel];
    if responds {
        () = msg![env; delegate navigationController:this willShowViewController:view_controller animated:false];
    }
    let vc_view: id = msg![env; view_controller view];
    // TODO: animations
    () = msg![env; view_controller viewWillAppear:false];
    () = msg![env; self_view addSubview:vc_view];
    () = msg![env; view_controller viewDidAppear:false];
    let sel: SEL = env
        .objc
        .register_host_selector(
            "navigationController:didShowViewController:animated:".to_string(),
            &mut env.mem
        );
    let responds: bool  = msg![env; delegate respondsToSelector:sel];
    if responds {
        () = msg![env; delegate navigationController:this didShowViewController:view_controller animated:false];
    }
}

- (id)popViewControllerAnimated:(bool)_animated {
    let (popped_view_controller, next_view_controller) = {
        let host_object = env.objc.borrow_mut::<UINavigationControllerHostObject>(this);
        if host_object.navigation_stack.len() <= 1 {
            return nil;
        }
        let popped_view_controller = host_object.navigation_stack.pop().unwrap();
        let next_view_controller = *host_object.navigation_stack.last().unwrap();
        (popped_view_controller, next_view_controller)
    };

    // The stack owns the popped controller. Keep a conventional autoreleased
    // return value alive while releasing that ownership after disappearance.
    retain(env, popped_view_controller);

    let self_view: id = msg![env; this view];
    () = msg![env; popped_view_controller viewWillDisappear:false];
    () = msg![env; next_view_controller viewWillAppear:false];

    let popped_view: id = msg![env; popped_view_controller view];
    () = msg![env; popped_view removeFromSuperview];

    // A normal push leaves the previous view underneath the new one, but
    // restore it defensively for archives/controllers that removed it.
    let next_view: id = msg![env; next_view_controller view];
    let next_superview: id = msg![env; next_view superview];
    if next_superview != self_view {
        () = msg![env; self_view addSubview:next_view];
    }

    () = msg![env; popped_view_controller viewDidDisappear:false];
    () = msg![env; next_view_controller viewDidAppear:false];

    super::set_parent_view_controller(env, popped_view_controller, nil);
    release(env, popped_view_controller); // navigation stack ownership
    autorelease(env, popped_view_controller);
    popped_view_controller
}

- (id)topViewController {
    if let Some(top_vc) = env.objc.borrow::<UINavigationControllerHostObject>(this).navigation_stack.last() {
        *top_vc
    } else {
        nil
    }
}

- (id)viewControllers {
    let vcs = env.objc.borrow::<UINavigationControllerHostObject>(this).navigation_stack.to_vec();
    for vc in &vcs {
        retain(env, *vc);
    }
    let res = ns_array::from_vec(env, vcs);
    autorelease(env, res)
}
- (())setViewControllers:(id)controllers { // NSArray *
    msg![env; this setViewControllers:controllers animated:false]
}

- (())setViewControllers:(id)controllers // NSArray *
                animated:(bool)animated {
    // Clean existing view controllers
    let self_view: id = msg![env; this view];
    let mut stack = std::mem::take(&mut env.objc.borrow_mut::<UINavigationControllerHostObject>(this).navigation_stack);
    for controller in stack.drain(..) {
        let vc_view = env.objc.borrow::<super::UIViewControllerHostObject>(controller).view;
        if vc_view != nil {
            let vc_view_superview: id = msg![env; vc_view superview];
            if self_view == vc_view_superview {
                // TODO: view{Will,Did}Disappear: messages for vc?
                () = msg![env; vc_view removeFromSuperview];
            }
        }

        let parent = env
            .objc
            .borrow::<super::UIViewControllerHostObject>(controller)
            .parent_view_controller;
        if parent == this {
            super::set_parent_view_controller(env, controller, nil);
        }

        release(env, controller);
    }

    let mut tmp_stack: Vec<id> = Vec::new();
    let count: NSUInteger = msg![env; controllers count];
    if count == 0 {
        return;
    }
    for i in 0..(count - 1) {
        let next: id = msg![env; controllers objectAtIndex:i];
        tmp_stack.push(next);
        retain(env, next);
        super::set_parent_view_controller(env, next, this);
    }
    env.objc.borrow_mut::<UINavigationControllerHostObject>(this).navigation_stack = tmp_stack;

    // The n-1 element in the controllers array is special and need to be pushed
    // TODO: double check this behavior
    let last_vc: id = msg![env; controllers objectAtIndex:(count - 1)];
    () = msg![env; this pushViewController:last_vc animated:animated];
}

- (id)navigationBar {
    env.objc
        .borrow::<UINavigationControllerHostObject>(this)
        .navigation_bar
}

- (bool)shouldAutorotateToInterfaceOrientation:(UIInterfaceOrientation)interface_orientation {
    let top_view_controller = env
        .objc
        .borrow::<UINavigationControllerHostObject>(this)
        .navigation_stack
        .last()
        .copied();
    if let Some(top_view_controller) = top_view_controller {
        msg![env; top_view_controller shouldAutorotateToInterfaceOrientation:interface_orientation]
    } else {
        msg_super![env; this shouldAutorotateToInterfaceOrientation:interface_orientation]
    }
}
- (bool)isNavigationBarHidden {
    env.objc
        .borrow::<UINavigationControllerHostObject>(this)
        .navigation_bar_hidden
}
- (())setNavigationBarHidden:(bool)hidden {
    let navigation_bar = {
        let host_object = env.objc.borrow_mut::<UINavigationControllerHostObject>(this);
        host_object.navigation_bar_hidden = hidden;
        host_object.navigation_bar
    };
    if navigation_bar != nil {
        () = msg![env; navigation_bar setHidden:hidden];
    }
}
- (())setNavigationBarHidden:(bool)hidden animated:(bool)_animated {
    () = msg![env; this setNavigationBarHidden:hidden];
}

@end

// Early Interface Builder archives may instantiate these objects even when
// the app keeps its navigation bar hidden. UINavigationBar inherits UIView's
// allocation and keyed-unarchiving behavior. UINavigationItem needs its own
// placeholder initializer because NSObject does not implement initWithCoder:.

@implementation UINavigationBar: UIView
@end

@implementation UINavigationItem: NSObject

- (id)initWithCoder:(id)_coder {
    this
}

@end

};
