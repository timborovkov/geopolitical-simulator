"""Behavioral checks for bounded validation, output attribution and failure propagation."""
import subprocess
import threading
import io
from types import SimpleNamespace
from unittest.mock import patch
import validate
import unittest
from parallel_recovery import Result, run_recovery_batch

BASE = ['cargo', 'run', '-p', 'headless', '--example', 'recovery', '--release', '--locked', '--']

class RecoveryBatchTests(unittest.TestCase):
    def test_every_command_runs_once_with_bounded_concurrency_and_ordered_output(self):
        commands = [BASE + [str(i)] for i in range(8)]
        seen, emitted, finished = [], [], []
        active, peak = 0, 0
        lock, started = threading.Lock(), threading.Barrier(3)
        later_finished = threading.Event()
        def execute(command):
            nonlocal active, peak
            with lock:
                active += 1
                peak = max(peak, active)
                seen.append(tuple(command))
            if int(command[-1]) < 3:
                started.wait(timeout=10)
            if command[-1] == '0':
                self.assertTrue(later_finished.wait(timeout=10))
            with lock:
                finished.append(command[-1])
                active -= 1
            if command[-1] == '2':
                later_finished.set()
            return Result(tuple(command), 0, command[-1])
        run_recovery_batch(commands, 3, execute, emitted.append)
        self.assertEqual(sorted(seen), sorted(map(tuple, commands)))
        self.assertEqual(len(seen), len(commands))
        self.assertEqual(peak, 3)
        self.assertNotEqual(finished[0], "0")
        self.assertEqual([r.command for r in emitted], list(map(tuple, commands)))

    def test_nonzero_exit_keeps_all_results_and_fails_validation(self):
        commands, emitted = [BASE + [str(i)] for i in range(5)], []
        def execute(command):
            return Result(tuple(command), 7 if command[-1] == '2' else 0, command[-1])
        with self.assertRaises(subprocess.CalledProcessError) as error:
            run_recovery_batch(commands, 2, execute, emitted.append)
        self.assertEqual(error.exception.returncode, 7)
        self.assertEqual(len(emitted), len(commands))

    def test_mutating_or_dependent_commands_are_refused_before_execution(self):
        commands = [
            ['cargo', 'build', '--release'],
            ['headless', '--save', 'shared.json'],
            ['cargo', 'run', '--example', 'other', '--release', 'recovery'],
            ['cargo', 'run', '--example'],
        ]
        for command in commands:
            with self.assertRaises(ValueError):
                run_recovery_batch([command], 4, lambda _: self.fail('invalid command executed'), lambda _: None)

    def test_invalid_worker_counts_and_mismatched_results_are_refused(self):
        for workers in [0, 5]:
            with self.assertRaises(ValueError):
                run_recovery_batch([BASE], workers, lambda _: None, lambda _: None)
        with self.assertRaises(ValueError):
            run_recovery_batch([BASE], 1, lambda _: Result(('wrong',), 0, ''), lambda _: None)

    def test_full_validator_keeps_builds_serial_before_parallel_replays(self):
        calls = []
        lock = threading.Lock()
        def execute(command, **kwargs):
            with lock:
                calls.append((list(command), kwargs))
            return SimpleNamespace(returncode=0, stdout="Replay output → preserved\n")
        with patch.object(validate, "check_documents") as documents, patch.object(validate.subprocess, "run", side_effect=execute), patch.object(validate.sys, "argv", ["validate.py", "--recovery-workers", "2"]), patch.object(validate.sys, "stdout", io.StringIO()) as output:
            validate.main()
        documents.assert_called_once()
        replay = [index for index, (command, _) in enumerate(calls) if "--example" in command]
        self.assertTrue(replay)
        first = min(replay)
        self.assertTrue(all("--example" not in command for command, _ in calls[:first]))
        self.assertTrue(all("--example" in command for command, _ in calls[first:]))
        self.assertIn("--verify", calls[first - 1][0])
        self.assertTrue(all(options.get("check") for _, options in calls[:first]))
        self.assertTrue(all(options.get("encoding") == "utf-8" for _, options in calls[first:]))
        self.assertEqual(output.getvalue().count("Replay output → preserved"), len(replay))

    def test_empty_batch_has_no_effect(self):
        run_recovery_batch([], 4, lambda _: self.fail('empty batch executed'), lambda _: self.fail('empty batch emitted'))

if __name__ == '__main__':
    unittest.main()
