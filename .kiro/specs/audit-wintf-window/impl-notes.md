# 実装の控え: audit-wintf-window

16.1 で research.md へ書き起こすための控え。

## 1.1

- 指摘 13（Q1）。書き込みのテストの道具 7 項目（`fake_hwnd`・`clean_slate`・`transition_lines`・`lines_of_kind`・`emit_control`・`SIGNOFF_DIRECTIVES`・`CONTROL_LINE`）を新しい兄弟のファイル `crates/wintf/src/ecs/window/command_test_support.rs` へ 1 つずつ置き（`pub(super)`）、`command.rs` に `#[cfg(test)] #[path = "command_test_support.rs"] mod test_support;` を足した。
- 写しを消したファイル: `command_batch_tests.rs`（7 項目）・`command_coalesce_tests.rs`（`fake_hwnd`・`emit_control`・`SIGNOFF_DIRECTIVES`・`CONTROL_LINE`）・`command_transition_tests.rs`（7 項目）。3 本とも `use super::test_support::*;` を使い、要らなくなった `IsWindow`・`RECORD_PREFIX_TAG`・`TRANSITION_TARGET`・`drain_window_pos_commands` の取り込みを外した。テストの中身と名前は変えていない。`command_transition_tests.rs` だけが使う `DEFAULT_DIRECTIVES` と `geometry_command` はその場に残した。
- `fake_hwnd` の正本は `command_transition_tests.rs` の版（左へ 1 桁ずらしてから奇数化する＝0x20 と 0x21 を潰さない・説明つき）。3 本の式はすでに同じだった。
- `CONTROL_LINE` は合流のテストだけ `"[coalesce-probe] alive"` だったが、共有の値 `"[transition-probe] alive"` にそろえた。合流のテストは定数を通してしか読まない（`command_coalesce_tests.rs` の `captured.contains(CONTROL_LINE)`）ので振る舞いは変わらない。
- 消す前の `git grep`（ツリー全体）:
  - `command_(batch|coalesce|transition)_tests::(fake_hwnd|clean_slate|transition_lines|lines_of_kind|emit_control|SIGNOFF_DIRECTIVES|CONTROL_LINE)` は 0 件（どれもテストのファイルの私的な項目で、外からの参照は無い）。
  - 字面 `coalesce-probe` は直した後に 0 件（spec 文書・fixture・設定にも参照は無い）。
- テストの数（`cargo test -p wintf --lib command`）: 前＝63 通過・1 無視（batch 8・coalesce 21・transition 16）。後＝63 通過・1 無視（batch 8・coalesce 21・transition 16）。テストの名と結果の一覧は前後で 1 行も違わない（違うのは所要時間の行だけ）。

## 2.1

- 12: `SetWindowPosCommand::flush` の本体を `impl SetWindowPosCommand` の外へ出し、自由関数 `flush_window_pos_commands` にした（字下げを 1 段外しただけで中身は同じ）。中身の無い包みだった旧 `flush_window_pos_commands` は消えた。`trace!` の文 `"SetWindowPosCommand::flush processing"` は `"flush_window_pos_commands processing"` に、`drain_window_pos_commands` の説明の参照も自由関数へ替えた。
- 構造の検査（`command_transition_tests.rs` の `flush_keeps_the_front_guard_so_the_default_run_pays_no_observation_cost`・対照の assert）を `code.contains("pub fn flush_window_pos_commands()")` に直した。
- 呼び名の置き換え: `command.rs` のテスト 1 か所・`command_transition_tests.rs` 3 か所・組と帯の実窓テスト 8 本 43 か所（band 2・always_on_top 14・multipair 2・release 5・survivor 4・maintain 12・measure 2・sink 2）。8 本は `use crate::ecs::window::{…}` に `flush_window_pos_commands` を足し、`SetWindowPosCommand` を他で使わない 3 本（band・multipair・measure）はその名を置き換えた。`zorder_pair_maintain_tests.rs` は `use` が 1 行増えたので、モジュール説明と `use` の間の空行を 1 つ詰めて 978 行のまま。
- 較正: 本番の `pub fn flush_window_pos_commands() {` を `pub fn flush_window_pos_commands () {` に 1 度変えると `command_transition_tests::flush_keeps_the_front_guard_so_the_default_run_pays_no_observation_cost` が「説明文を落とす処理が本文まで落としている」で赤。戻して緑。
- 消す前の `git grep "SetWindowPosCommand::flush"`: 前＝コード 46 件（command.rs 4・command_transition_tests.rs 3・実窓テスト 8 本 43）＋ spec 文書。後＝ `.kiro` を除くツリーで 0 件。`.kiro/` の中（この spec の文書と `completed/` の過去の spec）には指摘の説明・歴史の記録として残る（直さない）。areka・areka-placement からの参照は前から 0 件。
- テストの数（`cargo test -p wintf --lib ecs::window`）: 後＝464 通過・6 無視。名の変更だけでテストの増減は無い。
- レビュー 1 回目の直し: 同じ呼び出しが 2 つ並んだ `command.rs` の `test_flush_empty_queue_is_noop` の説明「便利関数経由でも同様に no-op」（包みが消えて偽になった）を「続けて流しても空のキューは no-op のまま」に替えた（行数は同じ）。`command_transition_tests.rs` の `flush_of_an_empty_queue_emits_no_records` の 2 つ目の呼び出しの前にも同じ趣旨の説明を 1 行足した（573→574 行）。`cargo test -p wintf --lib ecs::window::command`＝52 通過。
