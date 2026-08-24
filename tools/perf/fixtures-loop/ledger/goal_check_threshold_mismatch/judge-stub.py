#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""自己較正用の偽の判定スクリプト。

perf-ledger.py goal-check は判定スクリプトを **import せず字面だけを読む**（実物は重く、
Windows 専用の前提を持つため）。ここは版と較正値の 2 行だけを本物と同じ書き方で置く。
"""

SCRIPT_VERSION = (
    "0.4.0 (自己較正用の見本 / 実物は tools/perf/judge-perf.py)"
)

IDLE_CPU_MAX_RELEASE_PCT = 3.0
