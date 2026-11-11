"""Launch the real desktop in an isolated directory and require rendered readback."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def check_launch(binary, timeout=90):
    binary = Path(binary).resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="statecraft-native-launch-") as directory:
        report = Path(directory) / "launch.json"
        command = [str(binary), "--developer", "--native-launch-check", str(report)]
        try:
            result = subprocess.run(command, cwd=directory, stdout=subprocess.PIPE,
                                    stderr=subprocess.STDOUT, encoding="utf-8",
                                    errors="replace", timeout=timeout, check=False)
        except subprocess.TimeoutExpired as error:
            raise RuntimeError("Native launch timed out before clean exit and renderer readback") from error
        if result.returncode:
            raise RuntimeError(f"Native launch exited {result.returncode}:\n{result.stdout}")
        if not report.is_file():
            raise RuntimeError(f"Native launch produced no completed renderer report:\n{result.stdout}")
        data = json.loads(report.read_text(encoding="utf-8"))
        platform = {"win32": "windows", "darwin": "macos"}.get(sys.platform, sys.platform)
        if (data.get("schema") != 1 or data.get("platform") != platform
                or data.get("renderer") != "wgpu" or not data.get("backend")
                or not data.get("adapter") or data.get("frames", 0) < 3
                or data.get("width", 0) < 320 or data.get("height", 0) < 240
                or data.get("nonuniform_pixels") is not True
                or data.get("campaign_started") is not False):
            raise RuntimeError(f"Native report did not prove a rendered, unchanged launch: {data}")
        if sorted(p.name for p in Path(directory).iterdir()) != ["launch.json"]:
            raise RuntimeError("Native launch created unexpected files in its isolated directory")
        data["binary_sha256"] = hashlib.sha256(binary.read_bytes()).hexdigest()
        return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "target" / "release" /
                        ("desktop.exe" if sys.platform == "win32" else "desktop"))
    parser.add_argument("--report", type=Path, help="new persistent evidence file; existing files are protected")
    args = parser.parse_args()
    if args.report and args.report.exists():
        parser.error("Report already exists")
    data = check_launch(args.binary)
    text = json.dumps(data, indent=2) + "\n"
    if args.report:
        with args.report.open("x", encoding="utf-8") as output:
            output.write(text)
    print(text, end="")


if __name__ == "__main__":
    main()
