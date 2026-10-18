#!/usr/bin/env python3
"""校验 autotest report.json 的结构和关键指标，作为 E2E 的最终断言。"""

from __future__ import annotations

import json
import pathlib
import sys


def main(report_dir: str) -> int:
    root = pathlib.Path(report_dir)
    report_path = root / "report.json"
    if not report_path.exists():
        print(f"缺少 {report_path}")
        return 1

    report = json.loads(report_path.read_text(encoding="utf-8"))
    errors: list[str] = []

    if report.get("schema") != "hezhou.autotest.v1":
        errors.append("schema 不是 hezhou.autotest.v1")

    steps_total = report.get("steps_total", 0)
    steps_passed = report.get("steps_passed", 0)
    steps_failed = report.get("steps_failed", 0)
    if steps_total <= 0:
        errors.append("steps_total 必须 > 0")
    if steps_failed != 0:
        errors.append(f"存在失败步骤: {steps_failed}")
    if steps_passed + steps_failed != steps_total:
        errors.append(
            f"步骤计数不一致: passed({steps_passed}) + failed({steps_failed}) != total({steps_total})"
        )

    telemetry = report.get("telemetry", {})
    if telemetry.get("errors", 1) != 0:
        errors.append(f"telemetry errors={telemetry.get('errors')}")
    if telemetry.get("fatal", 1) != 0:
        errors.append(f"telemetry fatal={telemetry.get('fatal')}")

    mode = report.get("mode", "")
    metrics = report.get("metrics", {})
    if mode != "Intent Test" and metrics.get("frames", 0) <= 0:
        errors.append("非 intent 模式没有采集到帧数")
    if metrics.get("max_frame_gap_ms", 0) > 3000:
        errors.append(f"出现 >3s 卡顿: {metrics.get('max_frame_gap_ms')}ms")

    if not (root / "ui_tree.txt").exists():
        errors.append("缺少 ui_tree.txt")
    if not (root / "final.png").exists():
        errors.append("缺少 final.png")
    if not (root / "summary.txt").exists():
        errors.append("缺少 summary.txt")

    print(
        f"report: mode={mode} steps={steps_total} passed={steps_passed} "
        f"failed={steps_failed} frames={metrics.get('frames')} "
        f"avg_frame_ms={metrics.get('avg_frame_ms')} errors={telemetry.get('errors')}"
    )

    if errors:
        print("AUTOTEST REPORT FAILED")
        for error in errors:
            print(" - " + error)
        return 1

    print("AUTOTEST REPORT OK")
    return 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("usage: check_autotest_report.py <report_dir>")
        sys.exit(2)
    sys.exit(main(sys.argv[1]))
