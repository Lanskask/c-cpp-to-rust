# Review prompt (adversarial, one Part, after the gate passes)

Runs after `gate.py` passes and before the commit. The reviewer is a separate agent (Sonnet; Opus for
the first Part of each Pilot and for every `judgment`-tier Part in Stage 2). It has read access to the
repository and may run the gate and `cargo` with `--manifest-path`; it edits nothing. The gate says the
Part passed the Judge; the reviewer's job is to find what the Judge cannot see.

Slots: `{part_id}`, `{stage}`, `{diff}` (`git diff <snapshot HEAD> -- <target> <adapter>`),
`{rulebook}` (whole `RULEBOOK.md`), `{idiom_rows}`, `{source}` (original C), `{judge}` (as in
`stage2-prompt.md`), `{gate_output}`.

---

```text
You are an adversarial reviewer. A Part of a C → Rust port just passed its automated Judge.
Assume it is still wrong somewhere and find where. You edit nothing; you report.

Part: {part_id}   Stage: {stage}

Check, in order, and report every finding:
1. Behaviour the Judge cannot see. For each branch of the original C, name the harness input or
   translated test that exercises it, or report it as UNCOVERED with a concrete input (bytes) that would.
   Byte-class checks and boundaries (< vs <=, off-by-one, empty input, NUL, max length) first.
2. Rulebook conformance: §6 API exactly (Stage 2), §2 overrides, §3 feature guards at the same places
   as the C #ifdefs, §7 guard kinds, §9 quirks kept and marked, §8 deviations implemented as written and
   marked. A deviation or quirk that is in the code but not in the rulebook is a finding.
3. Idiom rows: each construct uses its row's mapping for this stage; a second mapping for a listed
   construct is a finding; an unlisted construct is a finding with a proposed new row.
4. Stage rules. Stage 1: C ABI kept, wrapping arithmetic where C wraps, SAFETY comments on non-trivial
   unsafe. Stage 2: zero unsafe, no unwrap/expect on input-driven paths, invariant comments present,
   no #[allow(...)] beyond what the rulebook lists, adapter diff limited to API calls and conversions
   (its public types and signatures unchanged).
5. Arithmetic: every subtraction, cast and index that could underflow, truncate or panic in a debug
   build, with the invariant that prevents it or the input that triggers it.
6. Markers and trailer: TODO(port)/BUG(port)/QUIRK(port)/DEVIATION(port) spelled exactly; PORT STATUS
   trailer present and its counts true.

Output: a TSV block, one finding per line, then one verdict line.
severity<TAB>check#<TAB>location<TAB>finding<TAB>evidence or input
severity: BLOCK (wrong behaviour or broken rule; must be fixed before commit)
          GAP (Judge cannot see it; propose the corpus input or test)
          RULE (rulebook or idiom table needs an amendment; state the exact proposed text)
          NIT (style; never blocks)
VERDICT: accept | fix | escalate
```

---

## What the orchestrator does with the report

- `BLOCK` → back to the implementer as `{previous}` (counts as an attempt).
- `GAP` → add the proposed inputs to the corpus generator or the translated tests (a Judge change, so
  it is its own commit, and the Part is re-gated against the stronger Judge).
- `RULE` → append to rulebook §12 as a proposal; applied by Opus or the user between batches.
- `accept` with no `BLOCK` → commit the Part.
- Two reviewers disagreeing on a `BLOCK` → escalate to Opus with both reports.
