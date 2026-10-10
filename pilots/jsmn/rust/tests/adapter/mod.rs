//! Judge adapter (judge.md §5): the only test-side file that calls the `jsmn` crate.
//!
//! Stage 2 version: calls the RULEBOOK §6 API; the types and method signatures below must not change (decision 32). Sentinels are the C values:
//! `None` count mode = `parse(js, None)`; unset position = `-1`; errors = `-1`, `-2`, `-3`.
#![allow(dead_code)]

use jsmn::{Parser, Token};

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
    parser: Parser,
    buf: Vec<Token>,
}

fn sentinel(v: Option<usize>) -> i64 {
    v.map_or(-1, |v| v as i64)
}

impl P {
    pub fn new() -> P {
        P {
            parser: Parser::new(),
            buf: vec![Token::default(); CAPACITY],
        }
    }

    /// `jsmn_init` on the existing parser; the token buffer keeps its contents.
    pub fn reset(&mut self) {
        self.parser = Parser::new();
    }

    /// Zero the whole token buffer (the C driver's `memset`).
    pub fn clear_tokens(&mut self) {
        self.buf.fill(Token::default());
    }

    /// `None` = count mode (`tokens == NULL`); `Some(n)` = use the first `n` slots of the persistent buffer.
    pub fn parse(&mut self, js: &[u8], tokens: Option<usize>) -> i32 {
        let result = match tokens {
            None => self.parser.count(js),
            Some(n) => {
                assert!(n <= CAPACITY, "adapter buffer holds {CAPACITY} tokens");
                self.parser.parse(js, &mut self.buf[..n])
            }
        };
        match result {
            Ok(count) => count as i32,
            Err(e) => e.code(),
        }
    }

    /// The first `n` slots of the buffer, as plain values.
    pub fn tokens(&self, n: usize) -> Vec<Tok> {
        self.buf[..n]
            .iter()
            .map(|t| Tok {
                kind: t.kind as i32,
                start: sentinel(t.start),
                end: sentinel(t.end),
                size: t.size as i64,
                #[cfg(feature = "parent-links")]
                parent: sentinel(t.parent),
                #[cfg(not(feature = "parent-links"))]
                parent: -1,
            })
            .collect()
    }

    pub fn state(&self) -> State {
        State {
            pos: self.parser.pos() as u64,
            toknext: self.parser.toknext() as u64,
            toksuper: sentinel(self.parser.toksuper()),
        }
    }
}

impl Default for P {
    fn default() -> Self {
        P::new()
    }
}
