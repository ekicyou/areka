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

## 2.2

- 41: `window_proc/window_pos.rs` の `WM_WINDOWPOSCHANGED` から、錠を取らない 2 回目の流し（手順③の `crate::ecs::window::flush_window_pos_commands()` の呼び出しと説明）を消した。手順の説明を「① 借用 → ② 錠を取って流す」の 2 手順にした。行番号で ③ を指す説明（`command.rs:22` の `window_pos.rs:290`・`command_transition_tests.rs` の `window_pos.rs:268`＝今は 390 行目）は「`WM_WINDOWPOSCHANGED` の手順②」と名前で指す形に、入れ子の理由の説明 4 か所（`command.rs` の `flush_window_pos_commands` の「時刻基準は入れ子になる」・`transition_diag.rs` の「区間は入れ子になる」・`command_transition_tests.rs` の `a_nested_flush_epoch_restores_the_outer_one_instead_of_clearing_it`・`window_pos_transition_tests.rs` の `windowposchanged_received_inside_a_window_write_carries_in_swp_and_flush_elapsed`）を「入れ子は錠を取らない直の流しの内側の ② で起きる」に直した。`FlushEpoch` は残した（直の流しの内側の ② で入れ子はまだ起きる）。
- 59: `WM_WINDOWPOSCHANGED`（旧 83-84／300-301）と `WM_DPICHANGED`（旧 387-388／430-431）の中身の無い 2 段のブロックを外して字下げを戻した（振る舞いは変わらない。`DpiChangeContext` に `Drop` は無く、落ちる時点の違いは効かない）。
- 新しいテストのファイル `window_proc/window_pos_decision_tests.rs`（`window_pos.rs` に `#[cfg(test)] #[path] mod decision_tests;`）。道具窓は `window_pos_transition_tests.rs` の `DISPATCH_TARGET`・`test_wndproc`・`TEST_CLASS`・`register_test_class`・`create_test_window` を `pub(super)` にして借りた（写さない）。
- 41 のテスト `windowposchanged_does_not_flush_the_queue_while_the_lock_is_held`: `engage_tick_flush_guard()` を取ったまま 1 本積み、道具窓へ `SendMessageW(WM_WINDOWPOSCHANGED)` を配ると `drain_window_pos_commands()` が 1 本返す。直す前のコードで赤（left: 0, right: 1）。
- 較正: ② の直後に `crate::ecs::window::flush_window_pos_commands();` を 1 度戻すと上のテストが赤（left: 0, right: 1）。戻して緑。
- 消す前の `git grep`（`手順③`・`3ステップ`・`window_pos.rs:2xx`）: 前＝`crates/wintf` の中で `window_pos.rs:41・44・296`・`command.rs:22・712`・`command_transition_tests.rs:390`・`transition_diag.rs:808`・`window_pos_transition_tests.rs:595`。後＝0 件（`②③` の WM_DPICHANGED の手順は別物で残る）。消した呼び出しは関数ではないので、ほかの使い手は関係しない。
- テストの数（`cargo test -p wintf --lib window_proc::window_pos`）: 前＝30・後＝31。`window_proc` 全体 109 通過。`window::command` 52・`transition_diag` 27 通過。
- clippy の `collapsible_if`（`WM_DPICHANGED` の `if let` の入れ子）は直す前からある（HEAD でも出る）。範囲の外なので触っていない。
