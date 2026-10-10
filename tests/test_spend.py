"""spend.py sums transcript usage once per message id and inside the time window."""

from __future__ import annotations

import json
from datetime import UTC, datetime
from pathlib import Path

import spend


def line(ts: str, mid: str, out: int, model: str = "claude-haiku-5-5") -> str:
    usage = {"input_tokens": 1, "output_tokens": out, "cache_creation_input_tokens": 10, "cache_read_input_tokens": 100}
    return json.dumps({"timestamp": ts, "message": {"id": mid, "model": model, "usage": usage}})


def write(path: Path, lines: list[str]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join([*lines, "not json", json.dumps({"type": "user"})]) + "\n")


def test_dedup_window_and_subagents(tmp_path: Path) -> None:
    write(
        tmp_path / "s1.jsonl",
        [
            line("2026-10-09T10:00:00Z", "a", 5),
            line("2026-10-09T10:00:01Z", "a", 5),  # same message, second content block
            line("2026-10-09T12:00:00Z", "b", 7),  # outside window
        ],
    )
    write(tmp_path / "s1" / "subagents" / "x.jsonl", [line("2026-10-09T10:30:00Z", "c", 3, "claude-opus-5-5")])
    t = spend.sum_usage(tmp_path, datetime(2026, 10, 9, 9, tzinfo=UTC), datetime(2026, 10, 9, 11, tzinfo=UTC))
    assert (t.messages, t.output, t.input, t.cache_read) == (2, 8, 2, 200)
    assert t.by_model == {"claude-haiku-5-5": 5, "claude-opus-5-5": 3}


def test_log_appends_rows(tmp_path: Path) -> None:
    tr = tmp_path / "tr"
    write(tr / "s.jsonl", [line("2026-10-09T10:00:00Z", "a", 5)])
    out = tmp_path / "SPEND.md"
    args = ["log", "--phase", "E0", "--since", "2026-10-09T00:00:00Z", "--transcripts", str(tr), "--out", str(out)]
    assert spend.main(args) == 0
    assert spend.main(args) == 0
    text = out.read_text()
    assert text.count("| date |") == 1
    assert text.count("| E0 |") == 2


def test_bad_inputs(tmp_path: Path) -> None:
    assert spend.main(["sum", "--since", "2026-10-09", "--transcripts", str(tmp_path / "missing")]) == 1
    assert spend.main(["sum", "--since", "yesterday", "--transcripts", str(tmp_path)]) == 1


def test_default_dir_naming() -> None:
    d = spend.default_transcripts_dir(Path("/Users/a1/My_Projects/settlers_to_rust"))
    assert d.name == "-Users-a1-My-Projects-settlers-to-rust"
