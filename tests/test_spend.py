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


def test_per_model_usage_has_all_counters_and_goes_to_sidecar(tmp_path: Path) -> None:
    write(
        tmp_path / "s.jsonl",
        [
            line("2026-10-09T10:00:00Z", "a", 5),
            line("2026-10-09T10:01:00Z", "b", 3, "claude-opus-5-5"),
            line("2026-10-09T10:02:00Z", "c", 4),
        ],
    )
    t = spend.sum_usage(tmp_path, datetime(2026, 10, 9, 9, tzinfo=UTC), datetime(2026, 10, 9, 11, tzinfo=UTC))
    assert t.usage_by_model["claude-haiku-5-5"] == {
        "input": 2,
        "output": 9,
        "cache_create": 20,
        "cache_read": 200,
        "messages": 2,
    }
    assert t.usage_by_model["claude-opus-5-5"]["input"] == 1
    out = tmp_path / "SPEND.md"
    spend.append_row(out, "E9", "-", t, "note")
    side = (tmp_path / "SPEND-MODELS.tsv").read_text().splitlines()
    assert side[0].startswith("date\tphase\tmodel\tinput")
    assert any(ln.split("\t")[2:] == ["claude-haiku-5-5", "2", "9", "20", "200", "2"] for ln in side[1:])
