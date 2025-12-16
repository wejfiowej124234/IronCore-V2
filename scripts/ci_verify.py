#!/usr/bin/env python3
"""CI closed-loop verifier for IronCore-V2.

This script verifies that the latest GitHub Actions run for a branch (or an explicit run id)
completed successfully AND that required jobs concluded success.

It is designed to work without log downloads and without authentication, but supports
higher rate limits via a token.

Usage examples:
  python scripts/ci_verify.py --owner wejfiowej124234 --repo IronCore-V2 --branch main
  GITHUB_TOKEN=... python scripts/ci_verify.py --owner wejfiowej124234 --repo IronCore-V2 --branch main
  python scripts/ci_verify.py --owner wejfiowej124234 --repo IronCore-V2 --run-id 20259754260
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
import urllib.parse
import urllib.request
from typing import Any, Dict, Iterable, List, Optional, Tuple

DEFAULT_REQUIRED_JOBS = [
    "Gates (ubuntu-latest)",
    "Gates (windows-latest)",
    "Security Audit",
    "Clippy (annotated)",
]


def _headers(token: Optional[str]) -> Dict[str, str]:
    h = {
        "Accept": "application/vnd.github+json",
        "User-Agent": "IronCore-CI-Verify",
    }
    if token:
        h["Authorization"] = f"Bearer {token}"
    return h


def _get_json(url: str, token: Optional[str]) -> Any:
    req = urllib.request.Request(url, headers=_headers(token))
    with urllib.request.urlopen(req) as resp:
        raw = resp.read().decode("utf-8")
    return json.loads(raw)


def _latest_run_id(owner: str, repo: str, branch: str, token: Optional[str]) -> Tuple[int, str]:
    url = (
        f"https://api.github.com/repos/{owner}/{repo}/actions/runs"
        f"?branch={urllib.parse.quote(branch)}&per_page=1"
    )
    data = _get_json(url, token)
    runs = data.get("workflow_runs", [])
    if not runs:
        raise RuntimeError(f"No workflow runs found for branch '{branch}'.")
    run = runs[0]
    return int(run["id"]), str(run.get("html_url", ""))


def _run(owner: str, repo: str, run_id: int, token: Optional[str]) -> Dict[str, Any]:
    return _get_json(f"https://api.github.com/repos/{owner}/{repo}/actions/runs/{run_id}", token)


def _jobs(owner: str, repo: str, run_id: int, token: Optional[str]) -> List[Dict[str, Any]]:
    url = f"https://api.github.com/repos/{owner}/{repo}/actions/runs/{run_id}/jobs?per_page=100"
    data = _get_json(url, token)
    return list(data.get("jobs", []))


def _format_job_summary(jobs: Iterable[Dict[str, Any]]) -> str:
    lines: List[str] = []
    for job in jobs:
        lines.append(f"- {job.get('name')} | {job.get('status')} | {job.get('conclusion')}")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify IronCore-V2 GitHub Actions CI is green.")
    parser.add_argument("--owner", required=True)
    parser.add_argument("--repo", required=True)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument("--branch", help="Branch name to check (uses latest run).")
    group.add_argument("--run-id", type=int, help="Explicit workflow run id.")
    parser.add_argument(
        "--required-job",
        action="append",
        dest="required_jobs",
        help="Job name that must conclude success (can be repeated).",
    )
    parser.add_argument(
        "--timeout-secs",
        type=int,
        default=1800,
        help="Max seconds to wait for a run to complete (default: 1800).",
    )
    parser.add_argument(
        "--poll-secs",
        type=int,
        default=20,
        help="Polling interval in seconds (default: 20).",
    )

    args = parser.parse_args()

    token = os.environ.get("GITHUB_TOKEN") or os.environ.get("GH_TOKEN")

    required = args.required_jobs or DEFAULT_REQUIRED_JOBS

    if args.run_id is not None:
        run_id = args.run_id
        run_url_hint = ""
    else:
        run_id, run_url_hint = _latest_run_id(args.owner, args.repo, args.branch, token)

    start = time.time()

    while True:
        run = _run(args.owner, args.repo, run_id, token)
        status = run.get("status")
        conclusion = run.get("conclusion")
        html_url = run.get("html_url") or run_url_hint
        head_sha = (run.get("head_sha") or "")[:7]

        if status == "completed":
            jobs = _jobs(args.owner, args.repo, run_id, token)
            jobs_by_name = {j.get("name"): j for j in jobs}

            missing = [name for name in required if name not in jobs_by_name]
            bad = [
                name
                for name in required
                if name in jobs_by_name and jobs_by_name[name].get("conclusion") != "success"
            ]

            print(f"run_id={run_id} sha={head_sha} status={status} conclusion={conclusion}")
            if html_url:
                print(f"url={html_url}")

            if missing:
                print("ERROR: missing required jobs:")
                for name in missing:
                    print(f"  - {name}")
                print("\nJobs seen:\n" + _format_job_summary(jobs))
                return 2

            if conclusion != "success" or bad:
                print("ERROR: CI not green.")
                if bad:
                    print("Required jobs not successful:")
                    for name in bad:
                        job = jobs_by_name[name]
                        print(f"  - {name}: {job.get('conclusion')}")
                print("\nJobs:\n" + _format_job_summary(jobs))
                return 1

            print("OK: CI is green and all required jobs succeeded.")
            return 0

        elapsed = time.time() - start
        if elapsed > args.timeout_secs:
            print(
                f"ERROR: timed out waiting for run {run_id} to complete "
                f"(waited {int(elapsed)}s)."
            )
            return 3

        print(
            f"waiting: run_id={run_id} status={status} conclusion={conclusion} "
            f"elapsed={int(elapsed)}s"
        )
        time.sleep(args.poll_secs)


if __name__ == "__main__":
    raise SystemExit(main())
