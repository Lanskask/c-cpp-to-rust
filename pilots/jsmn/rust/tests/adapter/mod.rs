//! Judge adapter (judge.md §5): the only test-side file that calls the `jsmn` crate.
//!
//! Stage 1 version: calls the C-ABI functions of the Faithful port inside `unsafe` blocks (`tests/` is outside
//! the zero-`unsafe` scope of `src/`). Stage 2 replaces the *body* of this file with calls to the RULEBOOK §6
//! API; the types and method signatures below must not change (decision 32). Sentinels are the C values:
//! `None` count mode = `parse(js, None)`; unset position = `-1`; errors = `-1`, `-2`, `-3`.
#![allow(dead_code)]

use jsmn::{jsmn_init, jsmn_parse, jsmn_parser, jsmntok_t};

pub const OBJECT: i32 = 1;
pub const ARRAY: i32 = 2;
pub const STRING: i32 = 4;
pub const PRIMITIVE: i32 = 8;

pub const ERROR_NOMEM: i32 = -1;
pub const ERROR_INVAL: i32 = -2;
pub const ERROR_PART: i32 = -3;

/// Size of the persistent token buffer every `P` owns (the C driver's `N`).
pub const CAPACITY: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tok {
    pub kind: i32,
    pub start: i64,
    pub end: i64,
    pub size: i64,
    /// `-1` when the `parent-links` feature is off (the field does not exist then).
    pub parent: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State {
    pub pos: u64,
    pub toknext: u64,
    pub toksuper: i64,
}

pub struct P {
    parser: jsmn_parser,
    buf: Vec<jsmntok_t>,
}

impl P {
    pub fn new() -> P {
        // SAFETY: both are plain C structs of integers; all-zero is a valid value. `jsmn_init` sets the parser.
        let mut parser: jsmn_parser = unsafe { core::mem::zeroed() };
        unsafe { jsmn_init(&mut parser) };
        let buf = vec![unsafe { core::mem::zeroed::<jsmntok_t>() }; CAPACITY];
        P { parser, buf }
    }

    /// `jsmn_init` on the existing parser; the token buffer keeps its contents.
    pub fn reset(&mut self) {
        unsafe { jsmn_init(&mut self.parser) };
    }

    /// Zero the whole token buffer (the C driver's `memset`).
    pub fn clear_tokens(&mut self) {
        self.buf.fill(unsafe { core::mem::zeroed::<jsmntok_t>() });
    }

    /// `None` = count mode (`tokens == NULL`); `Some(n)` = use the first `n` slots of the persistent buffer.
    pub fn parse(&mut self, js: &[u8], tokens: Option<usize>) -> i32 {
        let (ptr, n) = match tokens {
            None => (core::ptr::null_mut(), 0),
            Some(n) => {
                assert!(n <= CAPACITY, "adapter buffer holds {CAPACITY} tokens");
                (self.buf.as_mut_ptr(), n)
            }
        };
        unsafe {
            jsmn_parse(
                &mut self.parser,
                js.as_ptr().cast(),
                js.len(),
                ptr,
                n as u32,
            )
        }
    }

    /// The first `n` slots of the buffer, as plain values.
    pub fn tokens(&self, n: usize) -> Vec<Tok> {
        self.buf[..n]
            .iter()
            .map(|t| Tok {
                kind: t.type_0 as i32,
                start: t.start as i64,
                end: t.end as i64,
                size: t.size as i64,
                #[cfg(feature = "parent-links")]
                parent: t.parent as i64,
                #[cfg(not(feature = "parent-links"))]
                parent: -1,
            })
            .collect()
    }

    pub fn state(&self) -> State {
        State {
            pos: self.parser.pos as u64,
            toknext: self.parser.toknext as u64,
            toksuper: self.parser.toksuper as i64,
        }
    }
}

impl Default for P {
    fn default() -> Self {
        P::new()
    }
}
