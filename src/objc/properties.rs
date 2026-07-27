/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! Handling of Objective-C properties.
//!
//! Note that these are not the same as instance variables (ivars), though
//! they're closely related, so maybe this file will end up being used for those
//! too.
//!
//! Resources:
//! - `objc_setProperty` and friends are not documented, so [reading the source code](https://opensource.apple.com/source/objc4/objc4-551.1/runtime/Accessors.subproj/objc-accessors.mm.auto.html) is useful.
//!
//! See also: [crate::frameworks::foundation::ns_object].

use super::{id, msg, nil, release, retain, Class, ClassHostObject, ObjC, SEL};
use crate::dyld::{export_c_func, FunctionExports};
use crate::mem::{
    guest_size_of, ConstPtr, ConstVoidPtr, GuestISize, GuestUSize, Mem, MutPtr, MutVoidPtr, Ptr,
    SafeRead,
};
use crate::Environment;

/// The layout of a property list in an app binary.
///
/// The name, field names and field layout are based on what Ghidra outputs.
#[repr(C, packed)]
pub(super) struct ivar_list_t {
    entsize: GuestUSize,
    count: GuestUSize,
    // entries follow the struct
}
unsafe impl SafeRead for ivar_list_t {}

/// The layout of a declared-property list in an app binary. Same shape as the
/// ivar and method lists.
#[repr(C, packed)]
pub(super) struct property_list_t {
    entsize: GuestUSize,
    count: GuestUSize,
    // entries follow the struct
}
unsafe impl SafeRead for property_list_t {}

/// The layout of a property in an app binary.
///
/// This *is* `objc_property_t`'s referent: the runtime's opaque property handle
/// is a pointer straight into this table, and `property_getName` and
/// `property_getAttributes` just read these two fields. So tapHLE hands out the
/// binary's own pointers rather than synthesising anything, and both accessors
/// return exactly the strings the compiler emitted.
#[repr(C, packed)]
pub struct objc_property {
    pub(super) name: ConstPtr<u8>,
    pub(super) attributes: ConstPtr<u8>,
}
unsafe impl SafeRead for objc_property {}

/// An opaque type that represents an Objective-C declared property.
#[allow(non_camel_case_types)]
pub(super) type objc_property_t = ConstPtr<objc_property>;

/// The layout of a property in an app binary.
///
/// The name, field names and field layout are based on what Ghidra outputs.
#[repr(C, packed)]
struct ivar_t {
    offset: ConstPtr<GuestUSize>,
    name: ConstPtr<u8>,
    type_: ConstPtr<u8>,
    alignment: u32,
    size: u32,
}
unsafe impl SafeRead for ivar_t {}

impl ClassHostObject {
    pub(super) fn add_properties_from_bin(
        &mut self,
        property_list_ptr: ConstPtr<property_list_t>,
        mem: &Mem,
    ) {
        let property_list_t { entsize, count } = mem.read(property_list_ptr);
        assert!(entsize >= guest_size_of::<objc_property>());

        let properties_base_ptr: objc_property_t = (property_list_ptr + 1).cast();

        for i in 0..count {
            let property_ptr: objc_property_t =
                Ptr::from_bits(properties_base_ptr.to_bits() + i * entsize);
            let objc_property { name, .. } = mem.read(property_ptr);
            // Declaration order is kept: -class_copyPropertyList reports it,
            // and a linear scan over a handful of properties is not worth a
            // second index.
            let name = mem.cstr_at_utf8(name).unwrap().to_string();
            self.properties.push((name, property_ptr));
        }
    }

    pub(super) fn add_ivars_from_bin(&mut self, ivar_list_ptr: ConstPtr<ivar_list_t>, mem: &Mem) {
        let ivar_list_t { entsize, count } = mem.read(ivar_list_ptr);
        assert!(entsize >= guest_size_of::<ivar_t>());

        let ivars_base_ptr: ConstPtr<ivar_t> = (ivar_list_ptr + 1).cast();

        for i in 0..count {
            let ivar_ptr: ConstPtr<ivar_t> = Ptr::from_bits(ivars_base_ptr.to_bits() + i * entsize);

            // TODO: support type strings
            let ivar_t {
                offset,
                name,
                alignment,
                ..
            } = mem.read(ivar_ptr);

            let name_string = mem.cstr_at_utf8(name).unwrap().into();
            self.ivars.insert(name_string, (offset, alignment));
        }
    }
}

/// `Ivar`, the runtime's opaque handle for an instance variable.
///
/// The real runtime hands back a pointer to the class's `ivar_t` descriptor.
/// tapHLE parses those into a [ClassHostObject]'s map and does not keep the
/// descriptor's address, so what is returned here is the address of the ivar
/// *within the object*. Callers use the result as a found/not-found flag, which
/// this serves exactly; it is not usable with `ivar_getOffset` and friends, and
/// none of those exist to receive it.
type Ivar = MutVoidPtr;

/// Read an instance variable by name, the reflective way.
///
/// Compiled ivar access goes through the offset globals instead, so this is
/// reached only by code doing genuine runtime reflection — key-value coding
/// fallbacks and the like.
///
/// The value is read as one guest word. That is what the real function does
/// too: its `void **` out-parameter cannot express anything wider, so Apple
/// documents it as being for object-typed and pointer-sized ivars.
fn object_getInstanceVariable(
    env: &mut Environment,
    object: id,
    name: ConstPtr<u8>,
    out_value: MutPtr<MutVoidPtr>,
) -> Ivar {
    let name = env.mem.cstr_at_utf8(name).unwrap().to_string();
    let Some(ivar_ptr) = env.objc.object_lookup_ivar(&env.mem, object, &name) else {
        if !out_value.is_null() {
            env.mem.write(out_value, Ptr::null());
        }
        return Ptr::null();
    };
    if !out_value.is_null() {
        let value: MutVoidPtr = env.mem.read(ivar_ptr.cast());
        env.mem.write(out_value, value);
    }
    ivar_ptr.cast()
}

/// The setter half. Provided alongside the getter rather than speculatively:
/// the two are one API, and code that reflects over an ivar to read it is the
/// same code that reflects over it to write it back.
///
/// Note that this stores the pointer as given. The real runtime does the same —
/// it performs no retain, unlike `objc_setProperty` — so an object stored here
/// is not owned by the receiver.
fn object_setInstanceVariable(
    env: &mut Environment,
    object: id,
    name: ConstPtr<u8>,
    value: MutVoidPtr,
) -> Ivar {
    let name = env.mem.cstr_at_utf8(name).unwrap().to_string();
    let Some(ivar_ptr) = env.objc.object_lookup_ivar(&env.mem, object, &name) else {
        return Ptr::null();
    };
    env.mem.write(ivar_ptr.cast(), value);
    ivar_ptr.cast()
}

pub(super) const FUNCTIONS: FunctionExports = &[
    export_c_func!(object_getInstanceVariable(_, _, _)),
    export_c_func!(object_setInstanceVariable(_, _, _)),
];

impl ObjC {
    /// Checks if the object's class has an ivar in its class chain with the
    /// provided name and returns the pointer to the object's ivar, if any,
    /// or None if the object's class doesn't have an ivar with that name.
    pub fn object_lookup_ivar(
        &self,
        mem: &Mem,
        obj: id,
        name: &String,
    ) -> Option<MutPtr<GuestUSize>> {
        // nil has no ivars, and in particular has no isa to read. Every caller
        // reaching here from the runtime's reflective entry points can be
        // handed a nil object by ordinary guest code, so this is the specified
        // answer rather than a guard against a bug.
        if obj == nil {
            return None;
        }
        let mut class = ObjC::read_isa(obj, mem);
        loop {
            let &ClassHostObject {
                superclass,
                ref ivars,
                ..
            } = self.borrow(class);
            if let Some((ivar_offset_ptr, _)) = ivars.get(name) {
                let ivar_offset = mem.read(*ivar_offset_ptr);
                let ivar_ptr = MutVoidPtr::from_bits(obj.to_bits() + ivar_offset);
                return Some(ivar_ptr.cast());
            } else if superclass == nil {
                return None;
            } else {
                class = superclass;
            }
        }
    }

    pub fn debug_all_class_ivars_as_strings(&self, class: Class) -> Vec<String> {
        let mut class = class;
        let mut ivars_strings = Vec::new();
        loop {
            let &ClassHostObject {
                superclass,
                ref ivars,
                ..
            } = self.borrow(class);
            let mut class_ivars_strings = ivars.keys().cloned().collect();
            ivars_strings.append(&mut class_ivars_strings);
            if superclass == nil {
                break;
            } else {
                class = superclass;
            }
        }
        ivars_strings
    }
}

/// Undocumented function (see link above) apparently used by auto-generated
/// methods for properties to get an ivar.
pub(super) fn objc_getProperty(
    env: &mut Environment,
    this: id,
    _cmd: SEL,
    offset: GuestISize,
    atomic: bool,
) -> id {
    // We currently aren't touching the ivar layouts contained in the binary, so
    // we are assuming they are already correctly set by the compiler. Since we
    // aren't using ivars at all in our host classes, we shouldn't have any
    // issues with host classes' ivars clobbering guest classes' ivars, but
    // what if the compiler doesn't set the ivar layout at all? This is a simple
    // safeguard: any real ivar offset will be after the isa pointer.
    assert!(offset >= 4);

    if atomic {
        log_once!("TODO: Lock when atomic is set to true in objc_getProperty");
    }

    let ivar: MutPtr<id> = Ptr::from_bits(this.to_bits().checked_add_signed(offset).unwrap());
    env.mem.read(ivar)
}

/// Undocumented function (see link above) apparently used by auto-generated
/// methods for properties to set an ivar and handle reference counting, copying
/// and locking.
pub(super) fn objc_setProperty(
    env: &mut Environment,
    this: id,
    _cmd: SEL,
    offset: GuestISize,
    value: id,
    atomic: bool,
    should_copy: i8,
) {
    // We currently aren't touching the ivar layouts contained in the binary, so
    // we are assuming they are already correctly set by the compiler. Since we
    // aren't using ivars at all in our host classes, we shouldn't have any
    // issues with host classes' ivars clobbering guest classes' ivars, but
    // what if the compiler doesn't set the ivar layout at all? This is a simple
    // safeguard: any real ivar offset will be after the isa pointer.
    assert!(offset >= 4);

    if atomic {
        log_once!("TODO: Lock when atomic is set to true in objc_setProperty");
    }

    let ivar: MutPtr<id> = Ptr::from_bits(this.to_bits().checked_add_signed(offset).unwrap());
    let old = env.mem.read(ivar);

    let void_null: MutVoidPtr = Ptr::null();
    let value: id = if value != nil {
        match should_copy {
            0 => retain(env, value),
            1 => msg![env; value copyWithZone:void_null],
            2 => msg![env; value mutableCopyWithZone:void_null],
            // Apple's source code implies that any non-zero value that isn't 2
            // should mean "copy", but that seems weird, let's be conservative.
            _ => panic!("Unknown \"should copy\" value: {should_copy}"),
        }
    } else {
        nil
    };
    env.mem.write(ivar, value);

    if old != nil {
        release(env, old);
    }
}

// note: https://opensource.apple.com/source/objc4/objc4-723/runtime/objc-accessors.mm.auto.html
//       says that hasStrong is unused.
pub(super) fn objc_copyStruct(
    env: &mut Environment,
    dest: MutVoidPtr,
    src: ConstVoidPtr,
    size: GuestUSize,
    _atomic: bool,
    _hasStrong: bool,
) {
    // It's safe to ignore atomic as we never switch thread unless we call back
    // into guest code and we're not doing that here, just calling memmove.
    // TODO: implement atomic support
    env.mem.memmove(dest, src, size);
}

/// Logs a placeholder message for an unimplemented ObjC setter
///
/// This macro must be used inside [crate::_objc_method],
/// as it relies on constants for the current class and selector
/// set by it and [crate::objc::objc_classes]
#[macro_export]
macro_rules! todo_objc_setter {
    ($this:ident, $($arg:tt)+) => {
        const _: () = {
            let bytes = _OBJC_CURRENT_SELECTOR.as_bytes();
            let starts_with_set =
                bytes.len() > 3 && bytes[0] == b's' && bytes[1] == b'e' && bytes[2] == b't';
            assert!(starts_with_set, "Selector does not start with set.");
        };
        log!(
            "TODO: [({}*) {:?} {}:{:?}]",
            _OBJC_CURRENT_CLASS,
            $this,
            _OBJC_CURRENT_SELECTOR,
            $($arg)+
        );
    };
}
pub use crate::todo_objc_setter;
