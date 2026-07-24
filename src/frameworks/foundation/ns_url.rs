/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSURL`.

use super::ns_string::{from_rust_string, get_static_str, to_rust_string};
use super::NSUInteger;
use crate::fs::{GuestPath, GuestPathBuf};
use crate::mem::MutPtr;
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};
use crate::Environment;
use std::borrow::Cow;

/// It seems like there's two kinds of NSURLs: ones for file paths, and others.
/// So far only the former is implemented (TODO).
enum NSURLHostObject {
    /// This is a file URL. The NSString is a system path (no `file:///`).
    ///
    /// This is a wrapper around NSString so that conversions between NSURL
    /// and NSString, which happen often, can be simple and efficient.
    FileURL {
        ns_string: id,
        // Relative file URL save the working directory at the time of creation
        // At the moment, used in the description selector.
        working_directory: GuestPathBuf,
    },
    /// Non-file URL.
    OtherURL { ns_string: id },
}
impl HostObject for NSURLHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSURL: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = NSURLHostObject::FileURL { ns_string: nil, working_directory: env.fs.working_directory().into() };
    env.objc.alloc_object(this, Box::new(host_object), &mut env.mem)
}

+ (id)URLWithString:(id)url { // NSString*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initWithString:url];
    autorelease(env, new)
}

+ (id)fileURLWithPath:(id)path { // NSString*
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initFileURLWithPath:path];
    autorelease(env, new)
}

+ (id)fileURLWithPath:(id)path // NSString*
          isDirectory:(bool)is_dir {
    let new: id = msg![env; this alloc];
    let new: id = msg![env; new initFileURLWithPath:path isDirectory:is_dir];
    autorelease(env, new)
}

- (())dealloc {
    match *env.objc.borrow(this) {
        NSURLHostObject::FileURL { ns_string, .. } => release(env, ns_string),
        NSURLHostObject::OtherURL { ns_string } => release(env, ns_string),
    }
    env.objc.dealloc_object(this, &mut env.mem)
}

// NSCopying implementation
- (id)copyWithZone:(NSZonePtr)_zone {
    retain(env, this)
}

- (id)initFileURLWithPath:(id)path { // NSString*
    // FIXME: this should guess whether the path is a directory
    msg![env; this initFileURLWithPath:path isDirectory:false]
}

- (id)initFileURLWithPath:(id)path // NSString*
              isDirectory:(bool)_is_dir {
    // FIXME: this does not resolve relative paths to be absolute!
    // TODO: this does not strip the file:/// prefix!
    assert!(!to_rust_string(env, path).starts_with("file:"));
    let path = msg![env; path stringByExpandingTildeInPath];
    let path: id = msg![env; path copy];
    *env.objc.borrow_mut(this) = NSURLHostObject::FileURL { ns_string: path, working_directory: env.fs.working_directory().into() };
    this
}

- (id)initWithString:(id)url { // NSString*
    if url == nil {
        return nil;
    }

    // FIXME: this should parse the URL
    assert!(!to_rust_string(env, url).starts_with("file:")); // TODO
    let url: id = msg![env; url copy];
    *env.objc.borrow_mut(this) = NSURLHostObject::OtherURL { ns_string: url };
    this
}

- (id)initWithString:(id)url // NSString*
relativeToURL:(id)base_url { // NSURL*
    if url == nil {
        return nil;
    }
    if base_url == nil || url_scheme_component(&to_rust_string(env, url)).is_some() {
        return msg![env; this initWithString:url];
    }

    let base_string: id = msg![env; base_url absoluteString];
    let resolved = resolve_relative_url(
        &to_rust_string(env, base_string),
        &to_rust_string(env, url),
    );
    let resolved = from_rust_string(env, resolved);
    let result = msg![env; this initWithString:resolved];
    release(env, resolved);
    result
}

- (bool)isFileURL {
    match env.objc.borrow(this) {
        NSURLHostObject::FileURL { .. } => true,
        NSURLHostObject::OtherURL { .. } => false,
    }
}

// The following accessors parse the stored URL string. They only apply to
// non-file URLs; a file URL has no scheme/host/query/fragment here and returns
// nil, matching NSURL for a `fileURLWithPath:`-style URL's query/fragment.
- (id)scheme {
    url_component(env, this, url_scheme_component)
}
- (id)host {
    url_component(env, this, url_host_component)
}
- (id)query {
    url_component(env, this, url_query_component)
}
- (id)fragment {
    url_component(env, this, url_fragment_component)
}

- (id)description {
    match env.objc.borrow(this) {
        NSURLHostObject::FileURL { ns_string, working_directory } => {
            let working_directory = working_directory.as_str().to_string();
            let mut description = to_rust_string(env, *ns_string).to_string().clone();
            if !description.starts_with('/') {
                description = format!("{} -- file://localhost{}", description.trim_start_matches("./"), working_directory );
            }
            let desc = from_rust_string(env, description);
            autorelease(env, desc)
        },
        NSURLHostObject::OtherURL { ns_string } => *ns_string,
    }
}

- (id)path {
    match *env.objc.borrow(this) {
        NSURLHostObject::FileURL { ns_string, .. } => ns_string,
        NSURLHostObject::OtherURL { ns_string } => {
            // TODO: Support full URLs, not only ones that are just a path.
            // FIXME: This should do unescaping.
            // TODO: Avoid copy.
            assert!(to_rust_string(env, ns_string).starts_with('/'));
            ns_string
        },
    }
}

- (id)absoluteString {
    match *env.objc.borrow(this) {
        // FIXME: don't assume URL is already absolute
        NSURLHostObject::FileURL { ns_string, .. } => ns_string,
        NSURLHostObject::OtherURL { ns_string } => {
            // TODO: full RFC 1808 resolution
            assert!(to_rust_string(env, ns_string).starts_with("http"));
            ns_string
        },
    }
}

- (id)absoluteURL {
    // FIXME: don't assume URL is already absolute
    let &NSURLHostObject::OtherURL { .. } = env.objc.borrow(this) else {
        unimplemented!(); // TODO
    };
    this
}

- (bool)getFileSystemRepresentation:(MutPtr<u8>)buffer
                          maxLength:(NSUInteger)buffer_size {
    let &NSURLHostObject::FileURL { ns_string, .. } = env.objc.borrow(this) else {
        unimplemented!(); // TODO
    };
    msg![env; ns_string getFileSystemRepresentation:buffer maxLength:buffer_size]
}

- (id)URLByAppendingPathComponent:(id)path_component // NSString *
                      isDirectory:(bool)is_directory {
    let &NSURLHostObject::FileURL { ns_string, .. } = env.objc.borrow(this) else {
        unimplemented!(); // TODO
    };
    let mut path: id = msg![env; ns_string stringByAppendingPathComponent:path_component];
    if is_directory {
        path = msg![env; path stringByAppendingString:(get_static_str(env, "/"))];
    }
    msg_class![env; NSURL fileURLWithPath:path]
}

- (id)URLByDeletingLastPathComponent {
    let &NSURLHostObject::FileURL { ns_string, .. } = env.objc.borrow(this) else {
        unimplemented!(); // TODO
    };
    let path: id = msg![env; ns_string stringByDeletingLastPathComponent];
    msg_class![env; NSURL fileURLWithPath:path]
}

// TODO: more constructors, more accessors

@end

// A caching layer a top of NSURL, it's OK to stub
// as we don't have yet a networking support
@implementation NSURLCache: NSObject
+ (id)sharedURLCache {
    // TODO
    nil
}
@end

};

/// Shared implementation of the non-file-URL component accessors
/// ([NSURL scheme], [host], [query], [fragment]): parse the stored URL string
/// with `parser` and return the result as an autoreleased NSString, or nil.
/// File URLs have none of these components here.
fn url_component(env: &mut Environment, this: id, parser: fn(&str) -> Option<&str>) -> id {
    let ns_string = match env.objc.borrow(this) {
        NSURLHostObject::OtherURL { ns_string } => *ns_string,
        NSURLHostObject::FileURL { .. } => return nil,
    };
    let url = to_rust_string(env, ns_string).to_string();
    match parser(&url) {
        Some(component) => {
            let component = component.to_string();
            let new = from_rust_string(env, component);
            autorelease(env, new)
        }
        None => nil,
    }
}

/// Resolve the common relative-URL forms used by iPhone OS apps. This is not a
/// complete RFC 1808 parser, but it preserves the scheme and authority for an
/// absolute base URL and resolves both root-relative and path-relative inputs.
fn resolve_relative_url(base: &str, relative: &str) -> String {
    if relative.starts_with('/') {
        if let Some(scheme_end) = base.find("://") {
            let authority_end = base[scheme_end + 3..]
                .find('/')
                .map(|index| scheme_end + 3 + index)
                .unwrap_or(base.len());
            return format!("{}{}", &base[..authority_end], relative);
        }
        return relative.to_string();
    }

    let base_without_query_or_fragment = base.split(['?', '#']).next().unwrap();
    let prefix = if base_without_query_or_fragment.ends_with('/') {
        base_without_query_or_fragment
    } else {
        base_without_query_or_fragment
            .rsplit_once('/')
            .map(|(directory, _)| &base_without_query_or_fragment[..directory.len() + 1])
            .unwrap_or("")
    };
    format!("{prefix}{relative}")
}

/// Scheme: the text before the first `:`, if it is a valid scheme.
fn url_scheme_component(url: &str) -> Option<&str> {
    let (scheme, _rest) = url.split_once(':')?;
    let valid = !scheme.is_empty()
        && scheme
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'));
    valid.then_some(scheme)
}

/// Host: for a `scheme://host[:port][/...]` URL, the host part of the authority,
/// with any `user@` prefix and `:port` suffix removed.
fn url_host_component(url: &str) -> Option<&str> {
    let after_scheme = url.split_once("://")?.1;
    let authority = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(after_scheme);
    let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = host.split_once(':').map_or(host, |(h, _)| h);
    (!host.is_empty()).then_some(host)
}

/// Query: the text after the first `?`, up to an optional `#`.
fn url_query_component(url: &str) -> Option<&str> {
    let after_question = url.split_once('?')?.1;
    Some(
        after_question
            .split_once('#')
            .map_or(after_question, |(q, _)| q),
    )
}

/// Fragment: the text after the first `#`.
fn url_fragment_component(url: &str) -> Option<&str> {
    Some(url.split_once('#')?.1)
}

#[cfg(test)]
mod tests {
    use super::resolve_relative_url;

    #[test]
    fn resolves_relative_urls_against_http_base_urls() {
        assert_eq!(
            resolve_relative_url("http://example.com/assets/levels/", "one.json"),
            "http://example.com/assets/levels/one.json"
        );
        assert_eq!(
            resolve_relative_url("http://example.com/assets/levels.json", "one.json"),
            "http://example.com/assets/one.json"
        );
        assert_eq!(
            resolve_relative_url("http://example.com/assets/levels.json", "/scores"),
            "http://example.com/scores"
        );
    }
}

/// Shortcut for host code, provides a view of a URL as a path.
/// TODO: Try to avoid allocating a new GuestPathBuf in more cases.
pub fn to_rust_path(env: &mut Environment, url: id) -> Cow<'static, GuestPath> {
    let path_string: id = msg![env; url path];

    match to_rust_string(env, path_string) {
        Cow::Borrowed(path) => Cow::Borrowed(path.as_ref()),
        Cow::Owned(path_buf) => Cow::Owned(path_buf.into()),
    }
}
