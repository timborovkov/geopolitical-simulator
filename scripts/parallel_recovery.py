"""Bounded, portable orchestration for independent in-memory campaign checks."""
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
import subprocess

@dataclass(frozen=True)
class Result:
    command: tuple[str, ...]
    returncode: int
    output: str

def is_recovery(command):
    try:
        index = command.index("--example")
        return command[index + 1] == "recovery" and "--release" in command
    except (ValueError, IndexError):
        return False

def run_recovery_batch(commands, workers, execute, emit):
    """Keep bounded concurrency and stable command/output blocks; report failures."""
    if not 1 <= workers <= 4:
        raise ValueError('Choose one through four recovery workers')
    if not all(is_recovery(command) for command in commands):
        raise ValueError('Only independent in-memory recovery examples can be batched')
    failed = []
    # All submitted commands complete so failures retain every actual result.
    # No mutation/save commands are eligible for this runner.
    with ThreadPoolExecutor(max_workers=workers) as pool:
        futures = [pool.submit(execute, command) for command in commands]
        for command, future in zip(commands, futures):
            result = future.result()
            if result.command != tuple(command):
                raise ValueError('Recovery result belongs to a different command')
            emit(result)
            if result.returncode:
                failed.append(result)
    if failed:
        raise subprocess.CalledProcessError(failed[0].returncode, list(failed[0].command))
