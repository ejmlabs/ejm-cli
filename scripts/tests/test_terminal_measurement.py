"""Workload and trace verdicts must retain their distinct evidence."""

from copy import deepcopy
import json
import tempfile
import unittest
from pathlib import Path

from scripts.check_terminal import fresh_stop_acknowledgement, write_measurement_report


def traced_report():
    return {
        "event_trace": True,
        "fixture": {
            "completed": 12,
            "failed": 0,
            "drain_failed": 0,
            "selected_event_receive_to_terminal_write_ns": [123],
            "selected_event_commit_to_terminal_write_ns": [246],
            "paint_trace_omitted": 0,
            "paint_trace_integrity": {
                "enabled": True,
                "subscriptions": 3,
                "selection_changes": 2,
                "generation_resets": 0,
                "lag_signals": 0,
                "reported_lag_total": 0,
                "closed_signals": 0,
                "observer_error_count": 0,
                "observer_error_samples": [],
                "missing_commit_stamps": 0,
                "uncovered_saved_markers": 0,
            },
        },
    }


class MeasurementVerdict(unittest.TestCase):
    def test_stop_acknowledgement_requires_new_text_in_the_selected_session(self):
        before = "ejm-cli agent-0001\nselected session\nwork\n> stop-ready-001\ninput border\nhint"
        after = "ejm-cli agent-0001\nselected session\nStopping\n>\ninput border\nhint"
        self.assertEqual(fresh_stop_acknowledgement(before, after, "agent-0001", "stop-ready-001"), "Stopping")
        for previous, observed in ((before + "\nStopping", after),
                                   (before + "\ncancelled", after),
                                   (before, before + "\nStopping"),
                                   (before, after.replace("agent-0001", "agent-0002")),
                                   (before.replace("agent-0001", "agent-0002"), after),
                                   (before, before.replace("work", "new background work"))):
            with self.subTest(previous=previous, observed=observed):
                self.assertIsNone(fresh_stop_acknowledgement(previous, observed, "agent-0001", "stop-ready-001"))

    def test_stop_proofs_must_match_retired_sessions_and_preserve_useful_counts(self):
        before = "ejm-cli agent-0001\nselected session\nwork\n> stop-ready-001\ninput border\nhint"
        after = "ejm-cli agent-0001\nselected session\ncancelled\n>\ninput border\nhint"
        for failure in (None, "retired", "duplicate", "stale", "no_cancelled"):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                report = traced_report()
                report.update(operator_stop=True, requested_samples=1, trace=[{
                    "action": "stop_acknowledgement", "command": "/stop", "session_id": "session-1",
                    "sent_ns": 10, "observed_ns": 20,
                    "proof": {"before": before, "after": after, "title": "agent-0001", "marker": "stop-ready-001"},
                }])
                report["fixture"].update(operator_stopped_sessions=1, cancelled=1)
                if failure == "retired":
                    report["fixture"]["operator_stopped_sessions"] = 0
                elif failure == "duplicate":
                    report["trace"] *= 2
                elif failure == "stale":
                    report["trace"][0]["proof"]["before"] += "\ncancelled"
                elif failure == "no_cancelled":
                    report["fixture"]["cancelled"] = 0
                raw = deepcopy(report["fixture"])
                path = Path(directory) / "report.json"
                if failure:
                    with self.assertRaisesRegex(AssertionError, "retained report"):
                        write_measurement_report(path, report)
                else:
                    write_measurement_report(path, report)
                retained = json.loads(path.read_text())
                self.assertEqual(retained["operator_stop_validation"]["status"], "failed" if failure else "passed")
                self.assertEqual(retained["fixture"], raw)
                self.assertEqual(retained["trace"], report["trace"])

    def test_sample_latency_budget_does_not_claim_event_population_acceptance(self):
        for samples, expected in (([50_000_000], "within_budget"),
                                  ([50_000_001], "over_budget"),
                                  ([-1], "invalid_samples"), ([], "no_samples")):
            with self.subTest(samples=samples), tempfile.TemporaryDirectory() as directory:
                report = traced_report()
                report["fixture"]["selected_event_receive_to_terminal_write_ns"] = samples
                report["fixture"]["selected_event_commit_to_terminal_write_ns"] = samples
                report["fixture"]["paint_trace_integrity"]["uncovered_saved_markers"] = 2
                path = Path(directory) / "report.json"
                write_measurement_report(path, report)
                retained = json.loads(path.read_text())
                self.assertEqual(retained["latency_sample_verdicts"]["selected_receipt_to_terminal_write"]["status"], expected)
                self.assertEqual(retained["latency_sample_verdicts"]["selected_commit_to_terminal_write"]["status"], expected)
                self.assertEqual(retained["latency_sample_scope"]["event_trace_integrity"], "incomplete")
                self.assertEqual(retained["selected_event_latency_acceptance"], "unverified")

    def test_run_or_drain_failure_keeps_evidence_and_rejects_measurement(self):
        for failed, drain_failed in ((4096, 0), (0, 1)):
            with self.subTest(failed=failed, drain_failed=drain_failed), tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / "evidence" / "report.json"
                report = traced_report()
                report["workload_status"] = "passed"
                report["fixture"].update(completed=8192, failed=failed, drain_failed=drain_failed)
                report["fixture"]["paint_trace_integrity"].update(lag_signals=1, reported_lag_total=7)
                raw = deepcopy(report["fixture"])
                with self.assertRaisesRegex(AssertionError, "retained report"):
                    write_measurement_report(path, report)
                retained = json.loads(path.read_text())
                self.assertEqual(retained["workload_status"], "failed")
                self.assertEqual(retained["event_trace_status"], "incomplete")
                self.assertEqual(retained["selected_event_latency_acceptance"], "unverified")
                self.assertEqual(retained["fixture"], raw)

    def test_trace_failures_do_not_reject_useful_work(self):
        for failure in ("lag", "closed", "observer", "omissions", "legacy", "unavailable",
                        "missing_commit_samples", "partial_commit_samples", "missing_commit_stamp",
                        "missing_commit_accounting", "uncovered_saved_markers",
                        "missing_generation_accounting", "missing_saved_marker_accounting"):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                report = traced_report()
                fixture = report["fixture"]
                integrity = fixture["paint_trace_integrity"]
                if failure == "lag":
                    integrity.update(lag_signals=2, reported_lag_total=8)
                elif failure == "closed":
                    integrity["closed_signals"] = 1
                elif failure == "observer":
                    integrity.update(observer_error_count=1, observer_error_samples=["subscription: unavailable"])
                elif failure == "omissions":
                    fixture["paint_trace_omitted"] = 2
                elif failure == "legacy":
                    del fixture["paint_trace_integrity"]
                elif failure == "unavailable":
                    fixture["paint_trace_integrity"] = None
                elif failure == "missing_commit_samples":
                    del fixture["selected_event_commit_to_terminal_write_ns"]
                elif failure == "partial_commit_samples":
                    fixture["selected_event_receive_to_terminal_write_ns"].append(125)
                elif failure == "missing_commit_stamp":
                    integrity["missing_commit_stamps"] = 1
                elif failure == "missing_commit_accounting":
                    del integrity["missing_commit_stamps"]
                elif failure == "uncovered_saved_markers":
                    integrity["uncovered_saved_markers"] = 1
                elif failure == "missing_generation_accounting":
                    del integrity["generation_resets"]
                else:
                    del integrity["uncovered_saved_markers"]
                raw = deepcopy(fixture)
                path = Path(directory) / "report.json"
                write_measurement_report(path, report)
                retained = json.loads(path.read_text())
                self.assertEqual(retained["workload_status"], "passed")
                self.assertEqual(retained["event_trace_status"], "incomplete")
                self.assertEqual(retained["selected_event_latency_acceptance"], "unverified")
                self.assertTrue(retained["event_trace_reasons"])
                self.assertLessEqual(len(retained["event_trace_reasons"]), 8)
                self.assertEqual(retained["fixture"], raw)

    def test_clean_and_disabled_traces_remain_unverified(self):
        for expected in ("disabled", "no_samples", "diagnostic_only", "saved"):
            with self.subTest(expected=expected), tempfile.TemporaryDirectory() as directory:
                report = traced_report()
                if expected == "disabled":
                    report["event_trace"] = False
                    report["fixture"]["paint_trace_integrity"]["lag_signals"] = 1
                elif expected == "no_samples":
                    report["fixture"]["selected_event_receive_to_terminal_write_ns"] = []
                    report["fixture"]["selected_event_commit_to_terminal_write_ns"] = []
                elif expected == "saved":
                    report = {"saved_sessions": 100}
                path = Path(directory) / "report.json"
                write_measurement_report(path, report)
                retained = json.loads(path.read_text())
                self.assertEqual(retained["event_trace_status"], "disabled" if expected == "saved" else expected)
                self.assertEqual(retained["selected_event_latency_acceptance"], "unverified")
                if "fixture" in report:
                    self.assertEqual(retained["workload_status"], "passed")
                else:
                    self.assertNotIn("workload_status", retained)


if __name__ == "__main__":
    unittest.main()
