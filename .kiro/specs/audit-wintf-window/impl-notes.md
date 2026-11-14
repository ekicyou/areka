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
