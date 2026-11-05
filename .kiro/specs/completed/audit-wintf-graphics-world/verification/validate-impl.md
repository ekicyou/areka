# 実装の検証（/kiro-validate-impl）: audit-wintf-graphics-world

2026-11-06。1 回目は NO-GO（指摘 40・41）、直し（`7606ad11`）の後の 2 回目で GO。

## Validation Report
- DECISION: GO
- MECHANICAL_RESULTS:
  - Tests: PASS（`pwsh -NoProfile -File tools/test-all.ps1`・commit `7606ad11`・始めの未コミット 0・13 段すべて緑・x64 workspace tests 902 s）
  - TBD/TODO grep: CLEAN（`git diff 88eb0a43..HEAD -- . ':!.kiro'` の足した行に 0 件）
  - Secrets grep: CLEAN（同じ差分の足した行に 0 件）
  - Smoke boot: PASS（同じ全体テストの中の `crates/areka/tests/smoke_boot_loop_exit.rs` 5 本が緑）
- INTEGRATION:
  - Cross-task contracts: 整合。装置の失いの知らせ（3.2）→ 先頭の確かめ → 無効化 → 装置 → WUC → 窓の的（4.1）→ 見た目・描画面を作る係は `run_if(wuc_ready)`（4.3・4.4・4.7）→ 描く係は本当の失いのときだけ `GraphicsCore` の印を立てる（4.5）→ 下流（5.1 emo-present・5.2 emo-text と areka の毎フレームの呼び出し）が `Res<GraphicsCore>` の変化の印で控えを捨てる。
  - Shared state consistency: 整合。`GraphicsCore` の変化の印を立てる所は作り直しと本当の失いだけ（emo-text の `resource_scope` は `with_graphics_core` で時刻を戻す）。単一スレッドの一覧 `world::single_threaded_schedules()` は WUC に触る全部のスケジュールを含む。
  - Boundary audit: 整合。範囲の外で触ったファイルは design の「範囲の外で直すファイル」と記録済みの補正（`crates/wintf/Cargo.toml` の `Win32_Security`・`clip_sync_system_test.rs`・`visual_manager.rs` の説明・`frame.rs` の 1 行）だけ。wintf から areka への依存は 0。触ったファイルはすべて 1,000 行未満。
- COVERAGE:
  - Requirements mapped: 要件 1〜6 の全節（research.md 11・12 節、tasks.md の各タスク）
  - Coverage gaps: なし
- DESIGN:
  - Architecture drift: なし（作り直しの合図は `Res<GraphicsCore>` の変化の印に一本化し、`HasGraphicsResources` の印を立てる繰り返しは 40 番で消した＝ design.md を追随）
  - Dependency direction: 違反なし
  - File Structure Plan vs actual: 一致
- OWNERSHIP: LOCAL
- UPSTREAM_SPEC: N/A
- BLOCKED_TASKS: なし
- REMEDIATION: なし（1 回目の 40・41 は `7606ad11` で直した）
