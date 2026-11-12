# 実装の検証: audit-kanade（2026-11-07）

## Validation Report
- DECISION: GO
- MECHANICAL_RESULTS:
  - Tests: PASS（`pwsh -NoProfile -File tools/test-all.ps1`・commit `ee2ff8af`・開始時の未コミット 0・13 段すべて緑・終了コード 0。その後の `afeb971e` は tasks.md の印だけ）
  - TBD/TODO grep: CLEAN（この spec が足した行に 0 件）
  - Secrets grep: CLEAN（この spec が足した行に 0 件）
  - Smoke boot: PASS（同じ全体テストの中の `crates/areka/tests/smoke_boot_loop_exit.rs` 5 本が緑）
- INTEGRATION:
  - Cross-task contracts: 一致。選択の時間切れの起こしの線（`Action::ArmChoiceWake` → `KanadeConfig.choice_wake` → areka-ghost の中継 → `TickerMsg::WakeKanadeAt` → `KanadeMsg::ChoiceTimeoutDue`）と、中断の番号の線（`Emo2Wiring::shown_talk_id` → `send_break` → `KanadeMsg::UserBreak{talk_id}`）は、作る側と受ける側の形が合い、継ぎ目ごとにテストがある。端から端を 1 本で通すテストは無い（tasks.md の 16.4 の注のとおり）。
  - Shared state consistency: 一致。資源の問い合わせの断り（19.2 で送る前へ移した）は、断ったときの答えが前と同じ `Failed` で、呼び手（areka の menu/captions・update/worker・update/desk）が送るのは許可表にある名前だけ。
  - Boundary audit: 違反なし。範囲の外で変わった 17 ファイルは design.md 「範囲の外で触るファイル」の行のとおり。`vendors/`・`areka-talk`・`areka-actor`・`Cargo.lock` の差分は 0。`crate::placement::` の行は触っていない。kanade の判断が areka／areka-ghost へ漏れた所は無い。
- COVERAGE:
  - Requirements mapped: 全節（1.1〜9.10）がいずれかのタスクに載り、証跡がある
  - Coverage gaps: なし
- DESIGN:
  - Architecture drift: 記録済みのずれ 2 件だけ（10.1 の #35 は逆の側を消した・4.2 の #30 はコードの `change_input_ignored` に合わせた）。19.2 の作法の変え方（3 回目から Nice を本物の欠陥に絞った）は research.md §13 と tasks.md の注に記録した。
  - Dependency direction: 違反なし（`schedule/` の純粋層は殻を引かない・依存の追加なし）
  - File Structure Plan vs actual: 一致（最大は `schedule/events.rs` の 947 行・1,000 行を超えるファイルは 0）
- OWNERSHIP: LOCAL
- UPSTREAM_SPEC: N/A
- BLOCKED_TASKS: なし（未完 0）
- REMEDIATION: なし

## 完了の棚卸へ渡すもの
- `doc/COMPAT_ARCHITECTURE.md` の :308（空白だけを「無かった」と読む・旧いテスト名）・:327・:347（改名前のテスト名）・:354（消した `replace_absent_markers`）と、`crates/areka/src/mcp/raise_event_tests.rs:389` の注。
- 振る舞いが変わったところ（PR の説明に載せる）:
  - `OnInstallCompleteEx` の答えが空白だけなら、続く `OnInstallComplete` を送らない
  - 切替の保留中はマウスの入力を捨てる
  - ログのイベント名 `talk_done_stale_choice` を `talk_done_stale` に改めた
  - `boot_unexpected_reply` に `reply` の欄を足した
  - crate の根の再公開を 6 つ減らした
  - `change_rejected_phase` の理由を `close_pending` にそろえた
  - 資源の問い合わせは、許可表に無い名前を送る前に断る
  - 外からの台本の改行を取り除く
  - 終了・切替の保留中は RaiseEvent と property の返しを捨てる
- 17.1 のコミット単体では `crates/areka` がビルドできない（17.2 で直した・squash で消える）。
- 任意: areka の update のテストで `homeurl`・`useorigin1` が許可表にあることを固定する。
