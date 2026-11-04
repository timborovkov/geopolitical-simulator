"""Run repository checks with Python 3.11+ and the pinned Rust toolchain."""

import argparse
import csv
import os
from pathlib import Path
import re
import subprocess
import sys
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]


def read_register(name, columns):
    with (ROOT / "docs" / name).open(encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle)
        if reader.fieldnames != columns:
            raise ValueError(f"{name}: unexpected columns")
        rows = list(reader)
    if not rows or any(None in row or any(value is None for value in row.values()) for row in rows):
        raise ValueError(f"{name}: empty register or inconsistent columns")
    identifiers = [row[columns[0]] for row in rows]
    if not all(identifiers) or len(identifiers) != len(set(identifiers)):
        raise ValueError(f"{name}: missing or duplicate IDs")
    return rows


def check_documents():
    requirements = read_register(
        "implementation-register.csv",
        ["requirement_id", "priority", "subsystem", "requirement", "status"],
    )
    sources = read_register(
        "source-register.csv",
        ["source_id", "title", "product", "edition", "url", "evidence_kind", "coverage_status", "notes"],
    )
    spec = (ROOT / "docs/implementation-order.md").read_text(encoding="utf-8")
    expected = []
    priorities = []
    for section in re.split(r"^## ", spec, flags=re.MULTILINE):
        heading = re.match(r"Priority (\d+) — (.+)\n", section)
        if not heading:
            continue
        priority, subsystem = heading.groups()
        priorities.append(int(priority))
        for number, requirement in enumerate(re.findall(r"^- (.+)$", section, re.MULTILINE), 1):
            expected.append((f"P{int(priority):02}-{number:02}", str(int(priority)), subsystem, requirement))
    actual = [tuple(row[key] for key in ["requirement_id", "priority", "subsystem", "requirement"]) for row in requirements]
    if expected != actual:
        raise ValueError("Markdown requirements and CSV IDs, order or text differ")
    if priorities != list(range(1, len(priorities) + 1)):
        raise ValueError("Priority headings must be sequential")
    summary = f"{len(requirements)} proposed requirements across {len(priorities)} ordered systems"
    for name in ["README.md", "docs/implementation-order.md"]:
        if summary not in (ROOT / name).read_text(encoding="utf-8"):
            raise ValueError(f"{name}: requirement summary is out of date")
    if f"records {len(sources)} references" not in spec:
        raise ValueError("Implementation order: source count is out of date")

    paths = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z", "--", "*.md"],
        cwd=ROOT,
    ).decode().split("\0")
    for name in filter(None, paths):
        path = ROOT / name
        for target in re.findall(r"\[[^\]]*\]\(([^\s)]+)\)", path.read_text(encoding="utf-8")):
            url = urlsplit(target)
            if not url.scheme and not url.netloc and url.path:
                if not (path.parent / unquote(url.path)).exists():
                    raise ValueError(f"{name}: broken local link {target}")
    print(f"Documents OK: {len(requirements)} requirements, {len(priorities)} systems, {len(sources)} sources.", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--docs-only", action="store_true", help="skip Rust checks")
    args = parser.parse_args()
    check_documents()
    commands = [["git", "diff", "--check"]]
    if not args.docs_only:
        commands += [
            [sys.executable, "scripts/test_reference_data.py"],
            [sys.executable, "scripts/build_synthetic_world.py", "--check"],
            [sys.executable, "scripts/import_country_geometry.py", "--validate"],
            ["cargo", "fmt", "--all", "--", "--check"],
            ["cargo", "clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings"],
            ["cargo", "test", "--workspace", "--locked"],
            ["cargo", "doc", "--workspace", "--no-deps", "--locked"],
            ["cargo", "build", "--workspace", "--release", "--locked"],
            ["cargo", "run", "-p", "headless", "--release", "--locked", "--", "--verify"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--contested-transition"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--content", "data/initial-2000.json"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--content", "data/initial-2000-markets.json"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--content", "data/initial-2000-finance.json"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--content", "data/initial-2000-constitutions.json"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--content", "data/initial-2000-commitments.json"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--content", "data/initial-2000-cabinet.json"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--content", "data/initial-2000-cabinet.json", "--contested-transition"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--content", "data/initial-2000-elections.json"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--content", "data/initial-2000-elections.json", "--contested-transition"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--content", "data/synthetic-test.json"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--content", "data/synthetic-dated-test.json"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--content", "data/synthetic-market-test.json"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--content", "data/synthetic-finance-test.json"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--content", "data/synthetic-constitution-test.json"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--content", "data/synthetic-constitution-test.json", "--contested-transition"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--content", "data/synthetic-commitment-test.json"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--content", "data/synthetic-cabinet-test.json"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--content", "data/synthetic-election-test.json"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--content", "data/synthetic-subnational-test.json"],
            ["cargo", "run", "-p", "headless", "--example", "recovery", "--release", "--locked", "--", "--content", "data/synthetic-subnational-test.json", "--contested-transition"],
        ]
    env = dict(os.environ, RUSTDOCFLAGS="-D warnings")
    for command in commands:
        print(f"+ {' '.join(command)}", flush=True)
        subprocess.run(command, cwd=ROOT, env=env, check=True)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"Validation failed: {error}", file=sys.stderr)
        sys.exit(1)
