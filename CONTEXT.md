# Settlers to Rust

A reusable skill that translates a C/C++ project into Rust, proven on small libraries first. The long-term goal is a fully maintained Rust client for The Settlers II (s25client, later possibly widelands), but that is set aside until the skill works on Pilots.

## Language

**Source project**:
The C/C++ codebase being translated.
_Avoid_: legacy code, original repo, upstream

**Pilot**:
A small library used only to prove the skill works. The C Pilots are jsmn, cJSON, json-parser and miniz; one small C++ Pilot is added after them. A Pilot is never the end goal.
_Avoid_: test project, sample, benchmark

**Goal project**:
The large Source project the skill exists for: s25client first, widelands possibly later. Not worked on until the skill passes its Pilots.
_Avoid_: final target, big project

**Part**:
One logical unit of a Source project (a class, or the file that holds one) that is translated, verified, committed and resumed as a whole. A single function is too small to be a Part.
_Avoid_: unit, chunk, task, module

**Faithful port**:
Stage 1. A Rust version of the Source project that behaves identically to it and keeps its structure, even if it is C-shaped or `unsafe`. May keep the C ABI.
_Avoid_: transpile, raw port, C2Rust output

**Idiomatic pass**:
Stage 2. Turns a Faithful port into idiomatic Rust (safe types, Rust error handling, iterators). Drops the C ABI. Observable behaviour stays the same.
_Avoid_: refactor, cleanup, rewrite, redesign

**Same behaviour**:
For the same input the Rust version gives the same output, including malformed-input cases and error positions. API shape may differ after the Idiomatic pass.
_Avoid_: same functionality, parity, equivalence

**Judge**:
The checks a Part must pass before it is accepted: the original tests where they can be linked to the Rust code, a differential harness, and tests translated into Rust.
_Avoid_: oracle, referee, validator

**Differential harness**:
A small driver built once against the original library and once against the Rust crate. It feeds both the same inputs and diffs their outputs.
_Avoid_: fuzzer, comparison script

**Skill**:
The deliverable: instructions, scripts, and bundled libraries that a Claude Code session follows to translate a Source project, written generic with a per-project adapter.
_Avoid_: tool, pipeline, script
