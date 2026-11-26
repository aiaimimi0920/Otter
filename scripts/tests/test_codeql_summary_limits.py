"""Budget diagnostics must not disclose source text, filenames or credentials."""

import json
from pathlib import Path

import pytest

from scripts import summarize_codeql_results as summary


def test_file_limit_reports_only_safe_budget_metadata(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture
) -> None:
    source = tmp_path / "PRIVATE.sarif"
    source.write_text('{"PRIVATE": "oversized untrusted input"}', encoding="utf-8")
    monkeypatch.setattr(summary, "MAX_FILE_BYTES", 16)
    assert summary.main([str(tmp_path)]) == 2
    captured = capsys.readouterr()
    report = json.loads(captured.out)
    assert report == {
        "error": "input_limit_exceeded",
        "error_kind": "inventory_error",
        "analysis_complete": False,
        "inventory_complete": False,
        "limit": "file_bytes",
        "observed": source.stat().st_size,
        "maximum": 16,
    }
    assert not captured.err
    assert "PRIVATE" not in captured.out
    assert str(tmp_path) not in captured.out
