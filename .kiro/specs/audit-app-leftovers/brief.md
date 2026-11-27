# Brief: audit-app-leftovers

> 棚卸㊹（`/kiro-next`・main `ccd790fc`）で起票。出どころ＝本体の見直し 3 本（`audit-areka-app`・`audit-areka-install-update`・`audit-emo2-boot-switch`）が完了のときに「範囲の外」「次の機会に」と書き残した項目のうち、引き受ける spec が無いもの。1 件ずつは小さいが、触る場所が本体と `areka-session` のテストに集まっているので 1 本にまとめる。

## Problem

開発者と、本体のテストを読み書きするセッションが困っている。本体の見直しは「出た指摘は全部直す」方針で進めたが、3 本が着地した後も、直す持ち主のいない小さな取り残しが残っている。テストの失敗の文や記録の文に spec の名や要件の番号が残り（完了した spec の文書は流れていくので、番号を読んでも意味が分からない）、同じ働きの写しが 2 か所にあり、書き込みの結果を捨てているテストがある。

## Current State

数えと場所は棚卸㊹（main `ccd790fc`）のもの。着手のときに数え直す。

- **件 1　失敗の文・記録の文に残る spec の名と要件の番号**（CLAUDE.md の `## Comments` に反する）
  - `crates/areka-placement/src/placement_prepare_tests.rs:154`・`resolver_resolve_free_tests.rs:184`・`resolver_resolve_tests.rs:120,125,174,213,574`
  - `crates/areka/src/runtime_tests/frame_shell_box_align_tests.rs:219,224,253,258,266,288,293`・`crates/areka/src/menu/captions_tests.rs:381`（同じ形が `runtime_tests/` のほかの 2 ファイルにも在る見込み＝数え直す）
  - `crates/log-capture-kit/src/probe_tests.rs:73`・`tests/capture_calibration_test.rs:181`・`tests/file_length_guard_test.rs:65,69,109,115`・`tests/sample_path_guard_test.rs:708,817`（`temp_path_guard_test.rs` も数え直す）
  - 記録の文 `crates/areka-runtime/src/frame/drain_resnap.rs:561`。この文の頭「実表示寸で連鎖を再解決」を、テスト 3 本（`frame_chain_finalize_tests.rs:357`・`frame_reset_hold_tests.rs:31`・`frame_chain_finalize_restart_tests.rs:28`）が字面で引いている。
- **件 2　テストが固定していない事柄**
  - フォールバックの起動の記録 `fallback_boot_wired_off`・`fallback_boot_failed`（`crates/areka-session/src/ghost_session.rs:490,508`）は、テスト（`crates/areka/src/session_tests/ghost_session_strict_tests.rs:68-70`）が名で引くだけで、欄の中身を入力から組んだテストが無い。
  - 記録の控え `log_history::snapshot` が古い順に返すことを固定するテストが無い（`log_history_tests.rs`・`log_history_capture_tests.rs`）。
- **件 3　捨てている結果**: テストの中の `let _ = write_session_mark(…)` が 8 か所（`boot_resolve_last_used_tests.rs:282`・`session_end_deadline_tests.rs:82`・`session_mark_tests.rs:216`・`main_session_mark_fallback_tests.rs:227`・`main_session_mark_tests.rs:547`・`ghost_session_switch_memory_tests.rs:131,279,313`）。
- **件 4　同じ働きの写し**
  - コメントを落としてコードだけを読む関数 `code_only` の写し（`crates/areka/src/placement_tests/zorder_branch_coverage_test_support.rs:16`。共有の `source_text_test_support::code_only` が在る）。
  - ゴーストを降ろす所の時間の測りと記録が 2 か所（`crates/areka-session/src/ghost_session_runtime.rs:120` と `ghost_switch.rs:811`）。
  - 使っていない結線と 3 つ目の写し（`crates/areka/src/runtime_tests/frame_visibility_integration_tests.rs:59` の読まれない `BalloonWiring::new`・`:98` の `visibility_lines`）。
  - 記録を `event` の欄で手書きで濾す所（`crates/areka/src/main_session_mark_fallback_tests.rs:128,196,201` と、`install/`・`update/`・`mcp/`・`roots/`・`menu/`・`runtime_tests/`・`session_tests/shell_balloon_switch*` の同じ形）。共有の `events_named`・`count_event` が在る。
- **件 5　取り込みの道の混ざり**: `crates/areka-session/src/runtime_boot.rs:22` は `ChangeRequestRaw` を `ghost_switch.rs:44` の `pub(crate) use` 越しに取り、`:31` は `SkinRequestRaw` を `areka-runtime` から直に取る。
- **確かめていない 3 件**（着手のときに見る）: `crates/areka-session/src/boot_resolve_tests.rs:3,143` の「6 分岐」の数・`boot_config_option_tests.rs` の起動オプションの説明のコメント・`session_tests` の起動し直しのテストから消えた「なぜ」のコメント。
- `crates/areka-session/src/ghost_switch.rs` は 970 行（上限 1,000）。件 4 でここに足さない。

## Desired Outcome

- `crates/` の失敗の文・記録の文に、`.kiro` の spec の名と要件の番号が 0（件 1 の場所。歴史の場所は除く）。
- 件 2 の 2 つをテストが固定する。
- 件 3 の 8 か所が結果を確かめる。
- 件 4・件 5 の写しと混ざりが 1 つにそろう。
- 本番の振る舞いは変わらない（全体テストが緑のまま）。記録の文は、テストが引く頭の字面を残す。

## Approach

件ごとに小さく直す。件 1 は括弧の中の「（…要件 N.N）」「areka-P0-… N.N」だけを消し、文の残りは変えない。`drain_resnap.rs:561` は「…再解決（初期配置の確定）」の形にし、3 本のテストが引く頭を残す。失敗の文を道具（`tools/` の `.ps1`・`.py`）や見張りのテスト（`file_length_guard_test.rs`・`sample_path_guard_test.rs` は自分の文を読む作りかもしれない）が字面で読んでいないかを、直す前に確かめる。件 4 の降ろす所は、`ghost_switch.rs` の外に 1 つの関数を置き、両方から呼ぶ。

## Scope
- **In**: 件 1〜件 5 と、確かめていない 3 件の確かめ（要れば直す）。
- **Out**: 本番の振る舞いを変える直し・テストの置き場やファイル名の変更・`crates/areka-session/src/ghost_switch.rs` の分割（触る spec が先に割る）。

## Boundary Candidates
- 失敗の文・記録の文（件 1）
- `areka-session` と本体のテストの取り残し（件 2・件 3・件 4 のテストの側）
- `areka-session` の本番の側の 2 つ（件 4 の降ろす所・件 5）

## Out of Boundary
- 記録の控えに残さない宛先の表 `UNKEPT_TARGETS`（`crates/areka-session/src/log_history.rs`）へ、これから `update_cue`・`readme_cue` に足す `info!` の宛先を足す件。足す記録がまだ無い（ロードマップの覚え書きに在る）。
- テストの名の作業番号の札（✅ `test-name-work-tags` が済ませた）。

## Upstream / Downstream
- **Upstream**: なし（見直し 3 本は着地した）。
- **Downstream**: なし。

## Existing Spec Touchpoints
- **Extends**: ✅ `audit-areka-app`・✅ `audit-areka-install-update`・✅ `audit-emo2-boot-switch`（取り残しの出どころは各 `completed/<名前>/tasks.md` の Implementation Notes と `research.md`）。
- **Adjacent**（同じファイルを触る＝同じウェーブに置かない）: `ghost-switch-balloon-fallback`・`instant-to-interrupt-ticks`・`balloon-script-offset-align`（`crates/areka-session/src/ghost_switch.rs`）、`sylphya-sqlite-kvs`（`boot_resolve_last_used_tests.rs`）、`areka-test-wait-wording`（本体のテスト）、`balloon-text-visual-tree`（`crates/areka-runtime/src/frame/drain_resnap.rs`）。

## Constraints
- 本番の振る舞いを変えない。記録の文を変えるときは、字面で読む側（テスト・`tools/`）を先に確かめる。
- 規模の上限は 20 タスク。越えたら件で切る。

## 2026-11-27 棚卸㊹の測り（main ccd790fc）
- **触るファイル**: 上の「Current State」の場所（`areka-session` 約 10・本体のテスト 約 15〜・`areka-placement` 3・`areka-runtime` 1・`log-capture-kit` 4〜5）。新しいファイルは件 4 の降ろす所の 1 つ。
- **規模**: S〜M（6〜10 タスク）。
- **先に要るもの**: 働きの依存は無い。同じファイルを触る v0.2.0 の spec（上の Adjacent）が着地してから取る。
- **優先度の区分**: A（見直しの指摘は全部直す方針の取り残し）。
- **要件定義のモデル**: Opus。
- **議題**: 件 1 の失敗の文から番号を消すだけにするか、文そのものを言い直すか。件 4 の降ろす所の関数の置き場。
