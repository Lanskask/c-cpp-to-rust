//! Rust side of the Differential harness (judge.md §5): a line-for-line port of
//! `migration/harness/dump.c`, going through the adapter only. Reads the file named by the last argument and
//! prints the canonical dump of four parses. Output must match the C driver byte for byte.
use std::io::{BufWriter, Write};

#[path = "../tests/adapter/mod.rs"]
mod adapter;

use adapter::{Tok, CAPACITY, ERROR_PART, P};

fn dump(out: &mut impl Write, label: &str, r: i32, toks: &[Tok]) -> std::io::Result<()> {
    writeln!(out, "{label} r={r}")?;
    for (i, t) in toks.iter().enumerate() {
        if cfg!(feature = "parent-links") {
            writeln!(
                out,
                "  {i} type={} start={} end={} size={} parent={}",
                t.kind, t.start, t.end, t.size, t.parent
            )?;
        } else {
            writeln!(
                out,
                "  {i} type={} start={} end={} size={}",
                t.kind, t.start, t.end, t.size
            )?;
        }
    }
    Ok(())
}

/// `r > 0 ? r : (int)p.toknext` from the C driver.
fn shown(r: i32, toknext: u64) -> usize {
    if r > 0 {
        r as usize
    } else {
        toknext as usize
    }
}

fn run(js: &[u8], out: &mut impl Write) -> std::io::Result<()> {
    let mut p = P::new();

    // 1. count only
    let r = p.parse(js, None);
    let s = p.state();
    writeln!(
        out,
        "count r={r} pos={} toknext={} toksuper={}",
        s.pos, s.toknext, s.toksuper
    )?;

    // 2. full parse into a roomy buffer
    p.reset();
    let r = p.parse(js, Some(CAPACITY));
    let s = p.state();
    writeln!(
        out,
        "full pos={} toknext={} toksuper={}",
        s.pos, s.toknext, s.toksuper
    )?;
    dump(out, "full", r, &p.tokens(shown(r, s.toknext)))?;

    // 3. deliberately small buffer (NOMEM paths)
    p.clear_tokens();
    p.reset();
    let r = p.parse(js, Some(3));
    let s = p.state();
    writeln!(
        out,
        "small pos={} toknext={} toksuper={}",
        s.pos, s.toknext, s.toksuper
    )?;
    dump(out, "small", r, &p.tokens(s.toknext as usize))?;

    // 4. byte-at-a-time feeding (resumable parser; PART path)
    p.clear_tokens();
    p.reset();
    let mut r = r; // C keeps `r` from phase 3 when the input is empty
    for cut in 1..=js.len() {
        r = p.parse(&js[..cut], Some(CAPACITY));
        if r != ERROR_PART {
            break;
        }
    }
    let s = p.state();
    writeln!(
        out,
        "chunked pos={} toknext={} toksuper={}",
        s.pos, s.toknext, s.toksuper
    )?;
    dump(out, "chunked", r, &p.tokens(shown(r, s.toknext)))?;

    // 5. dirty buffer: allocation does not reset the type (Q2)
    p.poison_kinds();
    p.reset();
    let r = p.parse(js, Some(CAPACITY));
    let s = p.state();
    writeln!(out, "dirty pos={} toknext={} toksuper={}", s.pos, s.toknext, s.toksuper)?;
    dump(out, "dirty", r, &p.tokens(shown(r, s.toknext)))?;

    // 6. reused parser, num_tokens below toknext (Q9, RP14)
    p.clear_tokens();
    p.reset();
    let half = js.len() / 2;
    let r1 = p.parse(&js[..half], Some(CAPACITY));
    let s = p.state();
    let k = (s.toknext / 2) as usize;
    writeln!(
        out,
        "shrink1 r={r1} pos={} toknext={} toksuper={} k={k}",
        s.pos, s.toknext, s.toksuper
    )?;
    let r = p.parse(js, Some(k));
    let s = p.state();
    writeln!(out, "shrink pos={} toknext={} toksuper={}", s.pos, s.toknext, s.toksuper)?;
    dump(out, "shrink", r, &p.tokens(shown(r, s.toknext)))?;
    Ok(())
}

fn main() {
    let Some(path) = std::env::args().skip(1).last() else {
        std::process::exit(2);
    };
    let Ok(mut data) = std::fs::read(&path) else {
        std::process::exit(2);
    };
    data.truncate((1 << 20) - 1); // the C driver reads at most 1 MiB - 1
    let stdout = std::io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    run(&data, &mut out).expect("write to stdout");
    out.flush().expect("flush stdout");
}
