"""Budget diagnostics must not disclose source text, filenames or credentials."""

import json
import os
import subprocess
from copy import deepcopy
from pathlib import Path

import pytest

from scripts import summarize_codeql_results as summary
from scripts.tests.test_ci_build_gates import aggregate_bash
from scripts.tests.test_codeql_summary import _payload, _write


@pytest.mark.parametrize("size", [22_246_573, 32 * 1024 * 1024])
def test_full_sized_rust_inventory_preserves_every_finding(
    tmp_path: Path, size: int
) -> None:
    # Run 45737565530 produced 22,246,573 bytes; padding models its byte budget
    # without retaining source-bearing CI output in the repository.
    source = _write(tmp_path, _payload())
    expected = summary.summarize(tmp_path)
    with source.open("ab") as stream:
        stream.write(b" " * (size - source.stat().st_size))
    assert source.stat().st_size == size
    assert summary.summarize(tmp_path) == expected


def test_real_file_byte_ceiling_remains_enforced(
    tmp_path: Path, capsys: pytest.CaptureFixture
) -> None:
    source = _write(tmp_path, _payload(), "PRIVATE.sarif")
    size = 32 * 1024 * 1024 + 1
    with source.open("ab") as stream:
        stream.write(b" " * (size - source.stat().st_size))
    assert summary.main([str(tmp_path)]) == 2
    captured = capsys.readouterr()
    assert json.loads(captured.out) == {
        "error": "input_limit_exceeded",
        "error_kind": "inventory_error",
        "analysis_complete": False,
        "inventory_complete": False,
        "limit": "file_bytes",
        "observed": size,
        "maximum": 32 * 1024 * 1024,
    }
    assert not captured.err


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


@pytest.mark.parametrize(
    "constant,budget",
    [
        ("MAX_FILES", "files"),
        ("MAX_TOTAL_BYTES", "total_bytes"),
        ("MAX_RUNS", "runs"),
        ("MAX_INPUT_RESULTS", "input_results"),
        ("MAX_RULES", "rules_per_run"),
        ("MAX_DIRECTORY_ENTRIES", "directory_entries"),
        ("MAX_COMPONENTS", "components_per_run"),
        ("MAX_DIAGNOSTICS", "invocations_per_run"),
    ],
)
def test_every_input_budget_reports_observed_and_maximum(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture,
    constant: str,
    budget: str,
) -> None:
    source = _write(tmp_path, _payload(), "PRIVATE.sarif")
    observed = source.stat().st_size if constant == "MAX_TOTAL_BYTES" else 1
    monkeypatch.setattr(summary, constant, 0)
    assert summary.main([str(tmp_path)]) == 2
    captured = capsys.readouterr()
    assert json.loads(captured.out) == {
        "error": "input_limit_exceeded",
        "error_kind": "inventory_error",
        "analysis_complete": False,
        "inventory_complete": False,
        "limit": budget,
        "observed": observed,
        "maximum": 0,
    }
    assert not captured.err
    assert "PRIVATE" not in captured.out
    assert str(tmp_path) not in captured.out


@pytest.mark.parametrize(
    "budget",
    [
        "notifications_per_run",
        "total_notifications",
        "total_invocations",
        "distinct_rules",
        "distinct_severities",
    ],
)
def test_aggregate_and_per_run_budgets_remain_distinguishable(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture,
    budget: str,
) -> None:
    payload = _payload()
    run = payload["runs"][0]
    invocation = run["invocations"][0]
    notification = {"level": "warning", "message": {"text": "PRIVATE"}}
    if budget == "notifications_per_run":
        invocation["toolExecutionNotifications"] = [notification, notification]
    else:
        if budget == "total_notifications":
            invocation["toolExecutionNotifications"] = [notification]
        payload["runs"].append(deepcopy(run))
        other = payload["runs"][1]
        if budget == "distinct_rules":
            other["tool"]["driver"]["rules"][0]["id"] = "rust/other"
            other["results"][0]["ruleId"] = "rust/other"
        elif budget == "distinct_severities":
            other["results"][0]["level"] = "error"
    monkeypatch.setattr(
        summary,
        "MAX_RULES" if budget.startswith("distinct_") else "MAX_DIAGNOSTICS",
        1,
    )
    _write(tmp_path, payload, "PRIVATE.sarif")
    assert summary.main([str(tmp_path)]) == 2
    captured = capsys.readouterr()
    report = json.loads(captured.out)
    assert (report["limit"], report["observed"], report["maximum"]) == (budget, 2, 1)
    assert report["analysis_complete"] is report["inventory_complete"] is False
    assert "result_count" not in report
    assert "PRIVATE" not in captured.out
    assert str(tmp_path) not in captured.out


def test_exact_input_budget_boundaries_are_accepted(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    source = _write(tmp_path, _payload())
    for constant in (
        "MAX_FILES",
        "MAX_RUNS",
        "MAX_INPUT_RESULTS",
        "MAX_RULES",
        "MAX_DIRECTORY_ENTRIES",
        "MAX_COMPONENTS",
        "MAX_DIAGNOSTICS",
    ):
        monkeypatch.setattr(summary, constant, 1)
    for constant in ("MAX_FILE_BYTES", "MAX_TOTAL_BYTES"):
        monkeypatch.setattr(summary, constant, source.stat().st_size)
    report = summary.summarize(tmp_path)
    assert report["analysis_complete"] is report["inventory_complete"] is True
    assert report["result_count"] == 1


@pytest.mark.parametrize("status", [0, 2, 17])
def test_workflow_publishes_failure_details_without_masking_exit(
    tmp_path: Path, status: int
) -> None:
    root = Path(__file__).resolve().parents[2]
    workflow = (root / ".github/workflows/codeql.yml").read_text(encoding="utf-8")
    step = workflow.split("      - name: Report bounded CodeQL finding inventory\n", 1)[1]
    script = step.split("        run: |\n", 1)[1].split("      - name:", 1)[0]
    script = "\n".join(line[10:] for line in script.splitlines())
    report = json.dumps(
        {"error": "input_limit_exceeded", "limit": "runs", "observed": 101, "maximum": 100}
        if status else {"inventory_complete": True, "result_count": 0}
    )
    summary_file = tmp_path / "summary.md"
    env = dict(
        os.environ,
        GITHUB_STEP_SUMMARY=summary_file.as_posix(),
        INVENTORY_FIXTURE=report,
        INVENTORY_EXIT=str(status),
    )
    # Execute the actual Actions shell with only the scanner process replaced.
    stub = 'python3() { printf "%s\\n" "$INVENTORY_FIXTURE"; return "$INVENTORY_EXIT"; }\n'
    result = subprocess.run(
        [aggregate_bash(), "-e", "-o", "pipefail", "-c", stub + script],
        cwd=tmp_path,
        env=env,
        capture_output=True,
        text=True,
        timeout=15,
    )
    assert result.returncode == status, result.stderr
    assert json.loads(result.stdout) == json.loads(report)
    assert json.loads((tmp_path / "codeql-inventory.json").read_text()) == json.loads(report)
    assert summary_file.exists() == bool(status)
    if status:
        text = summary_file.read_text(encoding="utf-8")
        assert report in text
        assert text.startswith("### CodeQL inventory failure\n\n```json\n")
        assert text.endswith("\n```\n")
    assert "continue-on-error" not in step
