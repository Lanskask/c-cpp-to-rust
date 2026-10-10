//! Idiomatic Rust port of jsmn (`jsmn.h` @ the commit in `UPSTREAM`).
//!
//! `strict` = `JSMN_STRICT`, `parent-links` = `JSMN_PARENT_LINKS`. Input is `&[u8]`; a NUL byte ends it.

use core::fmt;

/// Token kind; the discriminants are the C `jsmntype_t` values.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TokenKind {
    #[default]
    Undefined = 0,
    Object = 1,
    Array = 2,
    String = 4,
    Primitive = 8,
}

/// One token. `start`/`end`/`parent` are `None` where C holds `-1`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub start: Option<usize>,
    pub end: Option<usize>,
    pub size: usize,
    #[cfg(feature = "parent-links")]
    pub parent: Option<usize>,
}

impl Token {
    /// An opened container that is not closed yet.
    fn is_open(&self) -> bool {
        self.start.is_some() && self.end.is_none()
    }
}

/// Parse error; `code()` is the C `jsmnerr` value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    NoMem,
    Inval,
    Part,
}

impl Error {
    pub fn code(self) -> i32 {
        match self {
            Error::NoMem => -1,
            Error::Inval => -2,
            Error::Part => -3,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoMem => f.write_str("not enough tokens were provided"),
            Error::Inval => f.write_str("invalid character inside JSON string"),
            Error::Part => f.write_str("the string is not a full JSON packet, more bytes expected"),
        }
    }
}

impl std::error::Error for Error {}

/// Parser state (`jsmn_parser`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Parser {
    pos: usize,
    toknext: usize,
    toksuper: Option<usize>,
}

impl Parser {
    /// `jsmn_init`.
    pub fn new() -> Self {
        Parser {
            pos: 0,
            toknext: 0,
            toksuper: None,
        }
    }

    /// `jsmn_parse` with a token buffer.
    pub fn parse(&mut self, js: &[u8], tokens: &mut [Token]) -> Result<usize, Error> {
        self.run(js, Some(tokens))
    }

    /// `jsmn_parse` with `tokens == NULL`: count only.
    pub fn count(&mut self, js: &[u8]) -> Result<usize, Error> {
        self.run(js, None)
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn toknext(&self) -> usize {
        self.toknext
    }

    pub fn toksuper(&self) -> Option<usize> {
        self.toksuper
    }

    /// `jsmn_alloc_token`: index of a fresh token, or `None` when the slice is full.
    fn alloc_token(&mut self, tokens: &mut [Token]) -> Option<usize> {
        let idx = self.toknext;
        // `get_mut` fails exactly when `toknext >= tokens.len()`, the C guard.
        let tok = tokens.get_mut(idx)?;
        self.toknext += 1;
        // QUIRK(port): Q2
        tok.end = None;
        tok.start = None;
        tok.size = 0;
        #[cfg(feature = "parent-links")]
        {
            tok.parent = None;
        }
        Some(idx)
    }

    /// `jsmn_parse_primitive`.
    fn parse_primitive(&mut self, js: &[u8], tokens: Option<&mut [Token]>) -> Result<(), Error> {
        let start = self.pos;
        'found: {
            // QUIRK(port): Q1
            while self.pos < js.len() && js[self.pos] != 0 {
                let c = js[self.pos];
                match c {
                    // QUIRK(port): Q5
                    #[cfg(not(feature = "strict"))]
                    b':' => break 'found,
                    b'\t' | b'\r' | b'\n' | b' ' | b',' | b']' | b'}' => break 'found,
                    _ => {}
                }
                // DEVIATION(port): D3
                if !(32..127).contains(&c) {
                    self.pos = start;
                    return Err(Error::Inval);
                }
                self.pos += 1;
            }
            #[cfg(feature = "strict")]
            {
                // QUIRK(port): Q6
                self.pos = start;
                return Err(Error::Part);
            }
        }

        // QUIRK(port): Q4
        let Some(tokens) = tokens else {
            // `pos > start` here: the first byte passed the per-byte check, so the loop advanced.
            self.pos -= 1;
            return Ok(());
        };
        let Some(idx) = self.alloc_token(tokens) else {
            // QUIRK(port): Q3
            self.pos = start;
            return Err(Error::NoMem);
        };
        // `idx < tokens.len()`: `alloc_token` just indexed it.
        let token = &mut tokens[idx];
        token.kind = TokenKind::Primitive;
        token.start = Some(start);
        token.end = Some(self.pos);
        token.size = 0;
        #[cfg(feature = "parent-links")]
        {
            token.parent = self.toksuper;
        }
        // `pos > start` here, see above.
        self.pos -= 1;
        Ok(())
    }

    /// `jsmn_parse_string`.
    fn parse_string(&mut self, js: &[u8], tokens: Option<&mut [Token]>) -> Result<(), Error> {
        let start = self.pos;
        // Skip the starting quote.
        self.pos += 1;

        // QUIRK(port): Q1
        while self.pos < js.len() && js[self.pos] != 0 {
            let c = js[self.pos];

            // Quote: end of string.
            if c == b'"' {
                // QUIRK(port): Q4
                let Some(tokens) = tokens else {
                    return Ok(());
                };
                let Some(idx) = self.alloc_token(tokens) else {
                    // QUIRK(port): Q3
                    self.pos = start;
                    return Err(Error::NoMem);
                };
                // `idx < tokens.len()`: `alloc_token` just indexed it.
                let token = &mut tokens[idx];
                token.kind = TokenKind::String;
                token.start = Some(start + 1);
                token.end = Some(self.pos);
                token.size = 0;
                #[cfg(feature = "parent-links")]
                {
                    token.parent = self.toksuper;
                }
                return Ok(());
            }

            // Backslash: a quoted symbol is expected.
            if c == b'\\' && self.pos + 1 < js.len() {
                self.pos += 1;
                match js[self.pos] {
                    b'"' | b'/' | b'\\' | b'b' | b'f' | b'r' | b'n' | b't' => {}
                    b'u' => {
                        self.pos += 1;
                        let mut i = 0;
                        // QUIRK(port): Q1
                        while i < 4 && self.pos < js.len() && js[self.pos] != 0 {
                            // DEVIATION(port): D3
                            if !js[self.pos].is_ascii_hexdigit() {
                                self.pos = start;
                                return Err(Error::Inval);
                            }
                            self.pos += 1;
                            i += 1;
                        }
                        // `pos >= 2` here (the quote and the backslash precede it).
                        self.pos -= 1;
                    }
                    _ => {
                        self.pos = start;
                        return Err(Error::Inval);
                    }
                }
            }
            self.pos += 1;
        }
        self.pos = start;
        Err(Error::Part)
    }

    /// Shared implementation of `jsmn_parse` (`tokens == None` is the count-only mode).
    fn run(&mut self, js: &[u8], mut tokens: Option<&mut [Token]>) -> Result<usize, Error> {
        // QUIRK(port): Q7
        let mut count = self.toknext;

        // QUIRK(port): Q1
        while self.pos < js.len() && js[self.pos] != 0 {
            let c = js[self.pos];
            match c {
                b'{' | b'[' => {
                    count += 1;
                    // C: with `tokens == NULL` the `break` leaves the switch and `pos` still advances.
                    if let Some(toks) = tokens.as_deref_mut() {
                        let Some(idx) = self.alloc_token(toks) else {
                            // QUIRK(port): Q3
                            return Err(Error::NoMem);
                        };
                        if let Some(sup) = self.toksuper {
                            // DEVIATION(port): D2
                            let t = toks.get_mut(sup).ok_or(Error::Inval)?;
                            #[cfg(feature = "strict")]
                            if t.kind == TokenKind::Object {
                                // QUIRK(port): Q2
                                return Err(Error::Inval);
                            }
                            t.size += 1;
                            #[cfg(feature = "parent-links")]
                            {
                                toks[idx].parent = self.toksuper;
                            }
                        }
                        // `idx < toks.len()`: `alloc_token` just indexed it.
                        let token = &mut toks[idx];
                        token.kind = if c == b'{' {
                            TokenKind::Object
                        } else {
                            TokenKind::Array
                        };
                        token.start = Some(self.pos);
                        self.toksuper = Some(idx);
                    }
                }
                // QUIRK(port): Q8
                b'}' | b']' => {
                    // QUIRK(port): Q4
                    if let Some(toks) = tokens.as_deref_mut() {
                        let kind = if c == b'}' {
                            TokenKind::Object
                        } else {
                            TokenKind::Array
                        };
                        #[cfg(feature = "parent-links")]
                        {
                            if self.toknext < 1 {
                                return Err(Error::Inval);
                            }
                            // Walk up through `parent` from the newest token.
                            let mut ti = self.toknext - 1;
                            loop {
                                // DEVIATION(port): D2
                                let token = toks.get_mut(ti).ok_or(Error::Inval)?;
                                if token.is_open() {
                                    if token.kind != kind {
                                        return Err(Error::Inval);
                                    }
                                    token.end = Some(self.pos + 1);
                                    self.toksuper = token.parent;
                                    break;
                                }
                                match token.parent {
                                    None => {
                                        if token.kind != kind || self.toksuper.is_none() {
                                            return Err(Error::Inval);
                                        }
                                        break;
                                    }
                                    Some(p) => ti = p,
                                }
                            }
                        }
                        #[cfg(not(feature = "parent-links"))]
                        {
                            // Scan backwards for the innermost open container.
                            // DEVIATION(port): D2
                            let head = toks.get_mut(..self.toknext).ok_or(Error::Inval)?;
                            let i = head.iter().rposition(Token::is_open).ok_or(Error::Inval)?;
                            if head[i].kind != kind {
                                return Err(Error::Inval);
                            }
                            self.toksuper = None;
                            head[i].end = Some(self.pos + 1);
                            // The next open container below it becomes the superior token.
                            if let Some(j) = head[..i].iter().rposition(Token::is_open) {
                                self.toksuper = Some(j);
                            }
                        }
                    }
                }
                b'"' => {
                    self.parse_string(js, tokens.as_deref_mut())?;
                    count += 1;
                    if let (Some(sup), Some(toks)) = (self.toksuper, tokens.as_deref_mut()) {
                        // DEVIATION(port): D2
                        toks.get_mut(sup).ok_or(Error::Inval)?.size += 1;
                    }
                }
                b'\t' | b'\r' | b'\n' | b' ' => {}
                b':' => {
                    // `toknext == 0` gives `None`, as C's `(int)(0u - 1)` gives -1.
                    self.toksuper = self.toknext.checked_sub(1);
                }
                // QUIRK(port): Q8
                b',' => {
                    if let (Some(toks), Some(sup)) = (tokens.as_deref_mut(), self.toksuper) {
                        // DEVIATION(port): D2
                        let kind = toks.get(sup).ok_or(Error::Inval)?.kind;
                        if kind != TokenKind::Array && kind != TokenKind::Object {
                            #[cfg(feature = "parent-links")]
                            {
                                self.toksuper = toks[sup].parent;
                            }
                            #[cfg(not(feature = "parent-links"))]
                            {
                                // DEVIATION(port): D2
                                let head = toks.get(..self.toknext).ok_or(Error::Inval)?;
                                let container = head.iter().rposition(|t| {
                                    matches!(t.kind, TokenKind::Array | TokenKind::Object)
                                        && t.is_open()
                                });
                                // No open container: `toksuper` stays as it is.
                                if let Some(j) = container {
                                    self.toksuper = Some(j);
                                }
                            }
                        }
                    }
                }
                #[cfg(feature = "strict")]
                b'-' | b'0'..=b'9' | b't' | b'f' | b'n' => {
                    // In strict mode primitives must not be keys of the object.
                    if let (Some(toks), Some(sup)) = (tokens.as_deref(), self.toksuper) {
                        // DEVIATION(port): D2
                        let t = toks.get(sup).ok_or(Error::Inval)?;
                        if t.kind == TokenKind::Object
                            || (t.kind == TokenKind::String && t.size != 0)
                        {
                            return Err(Error::Inval);
                        }
                    }
                    self.parse_primitive(js, tokens.as_deref_mut())?;
                    count += 1;
                    if let (Some(sup), Some(toks)) = (self.toksuper, tokens.as_deref_mut()) {
                        // DEVIATION(port): D2
                        toks.get_mut(sup).ok_or(Error::Inval)?.size += 1;
                    }
                }
                // QUIRK(port): Q5
                #[cfg(not(feature = "strict"))]
                _ => {
                    self.parse_primitive(js, tokens.as_deref_mut())?;
                    count += 1;
                    if let (Some(sup), Some(toks)) = (self.toksuper, tokens.as_deref_mut()) {
                        // DEVIATION(port): D2
                        toks.get_mut(sup).ok_or(Error::Inval)?.size += 1;
                    }
                }
                #[cfg(feature = "strict")]
                _ => return Err(Error::Inval),
            }
            self.pos += 1;
        }

        if let Some(toks) = tokens {
            // An opened object or array that was never closed.
            // DEVIATION(port): D2
            let head = toks.get(..self.toknext).ok_or(Error::Inval)?;
            if head.iter().any(Token::is_open) {
                return Err(Error::Part);
            }
        }
        Ok(count)
    }
}

// PORT STATUS: stage=2 confidence=high todos=0 deviations=D1,D2,D3
