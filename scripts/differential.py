#!/usr/bin/env python3
"""Independent, stdlib-only closure oracle against the Rust example process.

No Python CEGAR implementation is treated as proof of Rust correctness. This
script must execute a compiled Rust driver and check every returned witness.
"""
from __future__ import annotations

import argparse
import itertools
import json
import random
import subprocess
from pathlib import Path

SEED = 0x4345474152
Model = tuple[int, int, int, list[tuple[int, int, int]]]


def closure_has_bad(model: Model) -> bool:
    n, initial, bad, edges = model
    rows = [1 << state for state in range(n)]
    for source, _action, target in edges:
        rows[source] |= 1 << target
    for via in range(n):
        for source in range(n):
            if rows[source] & (1 << via):
                rows[source] |= rows[via]
    return any(initial & (1 << s) and rows[s] & bad for s in range(n))


def exhaustive():
    for n in range(1, 4):
        positions = list(itertools.product(range(n), repeat=2))
        for edge_mask in range(1 << (n * n)):
            edges = [(s, 0, t) for bit, (s, t) in enumerate(positions) if edge_mask & (1 << bit)]
            for initial in range(1, 1 << n):
                for bad in range(1 << n):
                    yield n, initial, bad, edges


def generated(count: int):
    rng = random.Random(SEED)
    for _ in range(count):
        n = rng.randrange(2, 13)
        edges = [(s, rng.randrange(4), t) for s in range(n) for t in range(n) if rng.randrange(7) == 0]
        yield n, rng.randrange(1, 1 << n), rng.randrange(1 << n), edges


def encode(model: Model) -> str:
    n, initial, bad, edges = model
    fields = [n, initial, bad, len(edges)]
    fields.extend(value for edge in edges for value in edge)
    return " ".join(map(str, fields))


def check(model: Model, result: dict) -> None:
    n, initial, bad, edges = model
    unsafe = closure_has_bad(model)
    expected = "unsafe" if unsafe else "safe"
    if result.get("status") != expected:
        raise AssertionError({"model": model, "expected": expected, "result": result})
    if unsafe:
        states, actions = result.get("states", []), result.get("actions", [])
        if not states or len(states) != len(actions) + 1 or len(states) > n:
            raise AssertionError("invalid witness shape")
        if any(not isinstance(s, int) or s < 0 or s >= n for s in states):
            raise AssertionError("unknown witness state")
        if not initial & (1 << states[0]) or not bad & (1 << states[-1]):
            raise AssertionError("witness endpoint mismatch")
        edge_set = set(edges)
        if any((s, action, t) not in edge_set for s, action, t in zip(states, actions, states[1:])):
            raise AssertionError("witness does not replay in supplied labeled model")


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--driver", type=Path, required=True)
    ap.add_argument("--random-cases", type=int, default=1_000)
    args = ap.parse_args()
    if not 0 <= args.random_cases <= 10_000:
        ap.error("--random-cases must be in 0..10000")
    models = list(exhaustive())
    assert len(models) == 28_868
    models.extend(generated(args.random_cases))
    malformed = ["", "0 1 0 0", "129 1 0 0", "2 0 2 0", "2 1 4 0", "2 1 2 1 0 1 9", "2 1 2 513", "2 1 2 0 extra"]
    lines = [encode(m) for m in models] + malformed
    run = subprocess.run([str(args.driver.resolve())], input="\n".join(lines) + "\n", capture_output=True, text=True, timeout=120, check=True)
    output = run.stdout.splitlines()
    if len(output) != len(lines):
        raise AssertionError(f"driver returned {len(output)} records for {len(lines)} inputs; stderr={run.stderr!r}")
    unsafe = 0
    for model, record in zip(models, output):
        result = json.loads(record)
        check(model, result)
        unsafe += result["status"] == "unsafe"
    for record in output[len(models):]:
        if json.loads(record).get("status") != "invalid":
            raise AssertionError("malformed transport input was not rejected")
    print(json.dumps({"status": "passed", "exhaustive_models": 28_868, "seeded_models": args.random_cases, "seed": SEED, "witnesses_replayed": unsafe, "invalid_records": len(malformed)}))


if __name__ == "__main__":
    main()
