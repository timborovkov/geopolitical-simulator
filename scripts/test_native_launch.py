"""Refuse false native-readiness claims, timeouts and save side effects."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch
from check_native_launch import check_launch


class NativeLaunchTests(unittest.TestCase):
    def report(self):
        return dict(schema=1, platform={"win32": "windows", "darwin": "macos"}.get(sys.platform, sys.platform),
                    renderer="wgpu", backend="Vulkan", adapter="Test adapter", frames=3,
                    width=1000, height=700, nonuniform_pixels=True, campaign_started=False)

    def invoke(self, data, returncode=0, extra=False):
        with tempfile.TemporaryDirectory() as root:
            binary = Path(root) / "desktop"
            binary.write_bytes(b"test binary")
            saves = Path(root) / "saves"
            saves.mkdir()
            prior = saves / "campaign.json"
            prior.write_bytes(b"original player save")
            def run(command, **kwargs):
                cwd = Path(kwargs["cwd"])
                self.assertNotEqual(cwd, Path(root))
                self.assertEqual(Path(command[0]), binary.resolve())
                self.assertEqual(command[1:3], ["--developer", "--native-launch-check"])
                self.assertEqual(Path(command[3]).parent, cwd)
                if data is not None:
                    Path(command[3]).write_text(json.dumps(data), encoding="utf-8")
                if extra:
                    (cwd / "saves").mkdir()
                return SimpleNamespace(returncode=returncode, stdout="native output")
            with patch("check_native_launch.subprocess.run", side_effect=run):
                try:
                    return check_launch(binary)
                finally:
                    self.assertEqual(prior.read_bytes(), b"original player save")

    def test_clean_rendered_launch_is_attributed_without_touching_saves(self):
        data = self.invoke(self.report())
        self.assertEqual(len(data["binary_sha256"]), 64)

    def test_success_exit_alone_and_invalid_readbacks_cannot_pass(self):
        for field, value in [("nonuniform_pixels", False), ("width", 0), ("frames", 2),
                             ("campaign_started", True), ("renderer", "CPU"), ("platform", "other")]:
            with self.subTest(field=field):
                data = self.report()
                data[field] = value
                with self.assertRaises(RuntimeError):
                    self.invoke(data)
        with self.assertRaisesRegex(RuntimeError, "no completed renderer report"):
            self.invoke(None)

    def test_failed_exit_and_side_effects_cannot_pass_with_a_valid_report(self):
        with self.assertRaisesRegex(RuntimeError, "exited 7"):
            self.invoke(self.report(), returncode=7)
        with self.assertRaisesRegex(RuntimeError, "unexpected files"):
            self.invoke(self.report(), extra=True)

    def test_hung_window_cannot_pass(self):
        with tempfile.NamedTemporaryFile() as binary:
            with patch("check_native_launch.subprocess.run", side_effect=subprocess.TimeoutExpired("desktop", 90)):
                with self.assertRaisesRegex(RuntimeError, "timed out"):
                    check_launch(binary.name)


if __name__ == "__main__":
    unittest.main()
