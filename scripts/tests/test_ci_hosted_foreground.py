"""The CI environment fixture cannot silently admit a failed test or change a developer desktop."""
from pathlib import Path
import sys
import unittest
from unittest.mock import Mock, call

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from ci_hosted_foreground import run

HOSTED = dict(GITHUB_ACTIONS="true", OTTER_CI_RUNNER_ENVIRONMENT="github-hosted", RUNNER_OS="Windows")
COMMAND = ["cargo", "nextest", "run", "--locked", "--workspace"]


class HostedForegroundTests(unittest.TestCase):
    def test_nonhosted_and_other_commands_rejected_before_opening_user32(self):
        factory = Mock()
        for env, command in (({}, COMMAND), (dict(HOSTED, OTTER_CI_RUNNER_ENVIRONMENT="self-hosted"), COMMAND),
                             (dict(HOSTED, RUNNER_OS="Linux"), COMMAND), (HOSTED, ["cargo", "build"])):
            with self.assertRaises(ValueError):
                run(command, env, factory)
        factory.assert_not_called()

    def test_same_selection_and_exit_code_after_verified_restoration(self):
        policy = Mock()
        policy.read.side_effect = [2147483647, 0, 2147483647]
        execute = Mock(return_value=100)
        self.assertEqual(run(COMMAND, HOSTED, lambda: policy, execute), 100)
        execute.assert_called_once_with(COMMAND)
        self.assertEqual(policy.set.call_args_list, [call(0), call(2147483647)])

    def test_rollback_on_launch_or_readback_error(self):
        for observed in (0, 1):
            policy = Mock()
            policy.read.side_effect = [50, observed, 50]
            execute = Mock(side_effect=OSError("launch"))
            with self.assertRaises((OSError, RuntimeError)):
                run(COMMAND, HOSTED, lambda: policy, execute)
            self.assertEqual(policy.set.call_args_list, [call(0), call(50)])
            self.assertEqual(execute.call_count, int(observed == 0))

    def test_failed_restore_does_not_report_passing_tests(self):
        policy = Mock()
        policy.read.side_effect = [50, 0, 1]
        with self.assertRaisesRegex(RuntimeError, "not restored"):
            run(COMMAND, HOSTED, lambda: policy, Mock(return_value=0))
