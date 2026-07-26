/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! zlib's gzip file API (`gzopen` and friends).
//!
//! Games use this to keep level and save data compressed. Only the *file* API
//! is implemented; the raw `inflate`/`deflate` stream API is a separate thing
//! and is deliberately not provided speculatively.
//!
//! Everything is buffered in host memory rather than streamed: `gzopen` for
//! reading decompresses the whole file up front, and `gzwrite` accumulates
//! plaintext that is compressed and written once at `gzclose`. Game data is
//! small enough that this is simpler and no slower in practice, and it means
//! the guest filesystem is touched through [crate::fs] exactly twice per file,
//! so the sandbox still applies — which streaming through a host `File` would
//! have bypassed.
//!
//! Resources:
//! - [zlib manual](https://www.zlib.net/manual.html#Gzip)

use crate::dyld::{export_c_func, FunctionExports};
use crate::fs::GuestPath;
use crate::mem::{ConstPtr, GuestUSize, MutPtr, MutVoidPtr, Ptr};
use crate::Environment;
use std::collections::HashMap;
use std::io::{Read, Write};

/// `gzFile`. Opaque to the guest; tapHLE hands out a small unique guest
/// allocation and keeps the real state host-side.
type gzFile = MutVoidPtr;

enum GzStream {
    Reading {
        /// The whole decompressed file.
        data: Vec<u8>,
        position: usize,
    },
    Writing {
        /// Plaintext accumulated so far, compressed at close.
        data: Vec<u8>,
        path: String,
    },
}

#[derive(Default)]
pub struct State {
    streams: HashMap<gzFile, GzStream>,
}
impl State {
    fn get(env: &mut Environment) -> &mut Self {
        &mut env.libc_state.zlib
    }
}

fn gzopen(env: &mut Environment, path: ConstPtr<u8>, mode: ConstPtr<u8>) -> gzFile {
    let path_str = env.mem.cstr_at_utf8(path).unwrap().to_string();
    let mode_bytes = env.mem.cstr_at(mode).to_vec();
    // As with fopen(), only the first character selects the mode; 'b' and a
    // compression level digit are decoration here.
    let writing = match mode_bytes.first() {
        Some(b'r') => false,
        Some(b'w') | Some(b'a') => true,
        other => {
            log!(
                "gzopen({:?}) with unsupported mode {:?}, failing",
                path_str,
                other
            );
            return Ptr::null();
        }
    };

    let stream = if writing {
        GzStream::Writing {
            data: Vec::new(),
            path: path_str.clone(),
        }
    } else {
        let Ok(compressed) = env.fs.read(GuestPath::new(&path_str)) else {
            log_dbg!("gzopen({:?}) for reading: no such file", path_str);
            return Ptr::null();
        };
        let mut decoder = flate2::read::MultiGzDecoder::new(&compressed[..]);
        let mut data = Vec::new();
        if let Err(e) = decoder.read_to_end(&mut data) {
            log!(
                "gzopen({:?}): not valid gzip data ({}), failing",
                path_str,
                e
            );
            return Ptr::null();
        }
        GzStream::Reading { data, position: 0 }
    };

    // A unique, non-null token the guest can compare and pass back. The
    // allocation is freed by gzclose().
    let handle: gzFile = env.mem.alloc(1);
    State::get(env).streams.insert(handle, stream);
    handle
}

fn gzread(env: &mut Environment, file: gzFile, buf: MutVoidPtr, len: GuestUSize) -> i32 {
    let Some(GzStream::Reading { data, position }) = State::get(env).streams.get_mut(&file) else {
        return -1;
    };
    let available = data.len().saturating_sub(*position);
    let count = (len as usize).min(available);
    let chunk = data[*position..*position + count].to_vec();
    *position += count;
    env.mem
        .bytes_at_mut(buf.cast(), count as GuestUSize)
        .copy_from_slice(&chunk);
    count as i32
}

fn gzwrite(env: &mut Environment, file: gzFile, buf: ConstPtr<u8>, len: GuestUSize) -> i32 {
    let chunk = env.mem.bytes_at(buf, len).to_vec();
    let Some(GzStream::Writing { data, .. }) = State::get(env).streams.get_mut(&file) else {
        return 0;
    };
    data.extend_from_slice(&chunk);
    len as i32
}

fn gzclose(env: &mut Environment, file: gzFile) -> i32 {
    let Some(stream) = State::get(env).streams.remove(&file) else {
        // Z_STREAM_ERROR
        return -2;
    };
    let result = match stream {
        GzStream::Reading { .. } => 0,
        GzStream::Writing { data, path } => {
            let mut encoder =
                flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            let compressed = encoder
                .write_all(&data)
                .and_then(|()| encoder.finish())
                .ok();
            match compressed {
                Some(compressed) if env.fs.write(GuestPath::new(&path), &compressed).is_ok() => 0,
                _ => {
                    log!("gzclose(): failed to write {:?}", path);
                    // Z_ERRNO
                    -1
                }
            }
        }
    };
    env.mem.free(file);
    result
}

fn gzeof(env: &mut Environment, file: gzFile) -> i32 {
    match State::get(env).streams.get(&file) {
        Some(GzStream::Reading { data, position }) => (*position >= data.len()).into(),
        // A write stream is never at end-of-file.
        Some(GzStream::Writing { .. }) => 0,
        None => 1,
    }
}

fn gztell(env: &mut Environment, file: gzFile) -> i32 {
    match State::get(env).streams.get(&file) {
        Some(GzStream::Reading { position, .. }) => *position as i32,
        Some(GzStream::Writing { data, .. }) => data.len() as i32,
        None => -1,
    }
}

fn gzrewind(env: &mut Environment, file: gzFile) -> i32 {
    match State::get(env).streams.get_mut(&file) {
        Some(GzStream::Reading { position, .. }) => {
            *position = 0;
            0
        }
        _ => -1,
    }
}

/// Only `SEEK_SET` and `SEEK_CUR` are meaningful; zlib does not support
/// `SEEK_END` on a gzip stream either.
fn gzseek(env: &mut Environment, file: gzFile, offset: i32, whence: i32) -> i32 {
    const SEEK_SET: i32 = 0;
    const SEEK_CUR: i32 = 1;
    let Some(GzStream::Reading { data, position }) = State::get(env).streams.get_mut(&file) else {
        return -1;
    };
    let target = match whence {
        SEEK_SET => offset as i64,
        SEEK_CUR => *position as i64 + offset as i64,
        _ => return -1,
    };
    if target < 0 {
        return -1;
    }
    *position = (target as usize).min(data.len());
    *position as i32
}

fn gzgetc(env: &mut Environment, file: gzFile) -> i32 {
    let Some(GzStream::Reading { data, position }) = State::get(env).streams.get_mut(&file) else {
        return -1;
    };
    if *position >= data.len() {
        return -1;
    }
    let byte = data[*position];
    *position += 1;
    byte as i32
}

/// The error string for a stream. Nothing here records a recoverable error, so
/// this always reports success; a failed open returns NULL instead, and a
/// failed read or write returns its own error value.
fn gzerror(env: &mut Environment, _file: gzFile, errnum: MutPtr<i32>) -> ConstPtr<u8> {
    if !errnum.is_null() {
        env.mem.write(errnum, 0);
    }
    // An empty string rather than NULL: callers print this without checking.
    env.mem.alloc_and_write_cstr(b"").cast_const()
}

pub const FUNCTIONS: FunctionExports = &[
    export_c_func!(gzopen(_, _)),
    export_c_func!(gzread(_, _, _)),
    export_c_func!(gzwrite(_, _, _)),
    export_c_func!(gzclose(_)),
    export_c_func!(gzeof(_)),
    export_c_func!(gztell(_)),
    export_c_func!(gzrewind(_)),
    export_c_func!(gzseek(_, _, _)),
    export_c_func!(gzgetc(_)),
    export_c_func!(gzerror(_, _)),
];
