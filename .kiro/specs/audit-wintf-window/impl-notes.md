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

## 2.3

- 直したもの（説明だけ・3 か所）: `window/transition_diag.rs` の `KIND_WINDPI` の説明（「wintf の 5 語（…＝[`KIND_ALL`]）」→「[`KIND_ALL`] から `windpi` を除いた 5 語」。既存の 11 語＝5＋1＋5 は `windpi` を選んだ時点の勘定なので数は残した）・`KIND_ALL` の説明の上位の語の列に `offset` を足した（`PLACEMENT_KIND_ALL` は snapshot／hold／ground／chain／offset の 5 語）・`window/transition_diag_tests.rs` の頭「5 種のレコード」→「6 種」（`KIND_ALL` は 6 語）。
- 直す前の食い違い: `KIND_ALL` は monitor／windpi／write／flush／msg／enqueue の 6 語なのに説明は 5 語でこれを `KIND_ALL` と呼んでいた。上位の列は `offset` が抜けていた。
- `MonitorRecord` の差分 0 行（`git diff | grep -c MonitorRecord` = 0）。説明だけの直しなので較正と消す前の `git grep` は当たらない。

## 3.1

- 変えたこと: `window_proc/window_pos_decision_tests.rs` に 42 番の 3 本を足した（本番は 0 行）。entity は `WindowHandle`・`WindowPos`・`BoxStyle`（最初の大きさ 7x7）・`DPI` 192 を持ち、置いた後に `clear_trackers` で印を消してから道具窓を `SetWindowPos`／`guarded_set_window_pos` で動かす。
  - ⑴ `external_move_marks_window_pos_changed_and_writes_logical_box_size`: 素の `SetWindowPos` で印が立ち、位置 (30,40)、`BoxStyle.size` が 200x100 ÷ 2 = 100x50
  - ⑵ `self_echo_writes_window_pos_without_marking_it_changed`: `guarded_set_window_pos` の照り返しで値は書くが印は立たず、`BoxStyle` は 7x7 のまま
  - ⑶ `after_a_dpi_change_window_pos_is_marked_and_box_style_is_left_alone`: `DpiChangeContext::set` の後、照り返し・外からの両方で印が立ち `BoxStyle` は 7x7 のまま（照り返しの枝で較正 1 を、外からの枝で較正 2 を捕まえる）
- 較正:
  - `window_pos.rs` の `use_bypass` を `is_echo` だけにする → ⑶ が赤（echo=true の枝で印が立たない）
  - `skip_box_style` から `dpi_context.is_some()` を落とす → ⑶ が赤（「DPI の直後に BoxStyle を書いた（echo=false）」）
  - どちらも戻して本番の差分 0 を `cmp` で確かめた
- 消したもの: なし（git grep 不要）
- テストの数: `cargo test -p wintf --lib window_proc::window_pos` 31 → 34（全部緑）

## 3.2

- 変えたもの（テストと説明だけ・本番は変えない）
  - 49: `window_proc/drop_files_tests.rs` に `dispatch_real_hdrop_delivers_the_same_two_paths` を足した。テストの中の `#[link(name = "kernel32")] unsafe extern "system"` で `GlobalAlloc` を宣言し（`GPTR`）、research.md 12.1 の形（`DROPFILES` の頭 20 バイト・`pFiles=20`・`fWide=1`・NUL 区切りの UTF-16 のパス 2 本・終わりの NUL）の HDROP を組んで `dispatch_window_message` へ `WM_DROPFILES` で配る。パスは `C:\投げ込み\ゴースト.nar`・`D:\伺か\シェル フォルダ`（存在しない・ファイルは作らない）。届いた一覧を UTF-16 のまま比べ、警告 0 も確かめる。片付けは受け手の `DragFinish`
  - 43: `window_proc/dpi_helpers_tests.rs` の説明の `window_proc/mod.rs` を `window_proc/window_pos_tests.rs` に 1 語直した（`git grep "fn s1_"` が `window_pos_tests.rs:396` ほかを指す）
- 49 の続き（レビュー 1 回目の差し戻し）: `read_error_variants_are_distinguishable_in_the_record` の説明「`Query` の腕は実物の `HDROP` でしか踏めない」を、通常の道は新しいテストが固定し、`Query` の腕は形の正しい `HDROP` でも踏めない（空のパスで一覧が終わる）ので記録の語だけを固定する、に書き直した（説明だけ）
- 較正: `drop_files.rs` の `vec![0u16; len as usize + 1]` の `+ 1` を落とす → `dispatch_real_hdrop_delivers_the_same_two_paths` が赤（`.nar` が `.na`・`フォルダ` が `フォル`）。戻して `drop_files.rs` の差分 0 を確かめた
- 消したもの: なし（git grep 不要）
- `Cargo.toml`・`Cargo.lock` の差分 0
- テストの数: `cargo test -p wintf --lib window_proc::drop_files` 6 → 7（全部緑）・`window_proc::dpi_helpers` 22（緑のまま）

## 4.1

- 頭の行数: `window/command.rs` 940 行。直した後の見込み（約 975 行）が 1,000 行を越えないので、3 項目は `command.rs` に置いた（`command_ui_thread.rs` は作らない）。直した後 976 行
- 変えたもの
  - 1: `command.rs` に `UI_THREAD`（`OnceLock<ThreadId>`・プロセスで 1 つ）・`record_ui_thread()`（`get_or_init`・`pub(crate)`）・`admits_enqueue(hwnd, ui_thread)`（控え無し・同じスレッド → true、違う → `error!` 1 行（`hwnd`・`ui_thread`・`current_thread`）で false）を足した。`error!` に窓を載せるため、設計の形に `hwnd` の引数を 1 つ足した
  - `SetWindowPosCommand::enqueue` の頭で判断を呼び、false なら `tick_wake::mark`・合流・記録の行より前に戻る。説明に「`WinApp::run` を回すスレッドから呼ぶ。ほかのスレッドから積んだ指令は `error!` を残して捨てる」を書いた
  - 範囲の外の最小の追従: `runtime/mod.rs` の `WinApp::run` の手順 1 の前に `crate::ecs::window::record_ui_thread();`（手順 0 の説明 1 行つき）
  - テスト: `command_threadlocal_tests.rs` に 3 本（`enqueue_is_admitted_when_no_ui_thread_is_recorded`・`enqueue_is_admitted_on_the_recorded_ui_thread`・`enqueue_off_the_recorded_ui_thread_is_dropped_with_one_error`）。控えは引数で渡し、プロセス全体の控えは立てない
- 旗の手順の赤: 判断を常に true にする旗で `enqueue_off_the_recorded_ui_thread_is_dropped_with_one_error` が赤
- 較正:
  - 比べを外す（`Some(ui) if ui != current` → `Some(ui)`）→ `enqueue_is_admitted_on_the_recorded_ui_thread` が赤
  - `error!` を消す → `enqueue_off_the_recorded_ui_thread_is_dropped_with_one_error` が赤
  - どちらも戻して `command.rs` の差分が意図した直しだけであることを確かめた
- 積み込みの呼び手 4 か所（`zorder_pair_maintain.rs`・`graphics/systems/window_pos.rs`・`areka-placement` の `follow/window_move.rs`・`areka/examples/mock-shell.rs`）は直さずビルドが通る
- 消したもの: なし（git grep 不要）
- テストの数: `cargo test -p wintf --lib command_threadlocal_tests` 3 → 6（全部緑）

## 4.2

- 頭の行数: `window/command.rs` 976 行。直した後 983 行（1,000 行以下・`command_ui_thread.rs` は作らない）
- 門（7 の手順 1）: `command_batch_tests.rs` に `a_queue_with_the_same_window_twice_lands_in_the_same_order_as_one_write_per_command` を足した。自分の不可視の窓 4 枚（A〜D）を毎回 `normalize_z` で `0,1,2,3` に並べ直し、`[A を B の後へ][C を A の後へ][A を D の後へ]`（Z だけ・`SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE`）を、⑴ `guarded_set_window_pos` を順に呼ぶ、⑵ キューに積んで `flush_window_pos_commands` で流す、の 2 通りで当てて 4 枚の相対の順を比べる。⑵ がバッチの道を通ったこと（書込レコード 3 本とも `in_batch=true`）も確かめる
- 門の結果: **同じ**の枝。2 回とも 1 本ずつ＝`[1, 2, 3, 0]`（B,C,D,A）、キューで流す＝`[1, 2, 3, 0]`。Wine・ReactOS の読み（2 本目を畳む）なら `[1, 3, 0, 2]` になるはずで、実際の Windows には当たらない
- 7 の手順 2（説明だけ）: `command.rs` の `apply_as_batch` の doc に「同じ窓が 2 度入った列もそのまま積み、実際の Windows では 1 本ずつと同じ Z の順に着く（門のテストの名）・Wine・ReactOS の読みは当たらない」を足し、`flush_window_pos_commands` の doc の「適用の並びも引数もキューのまま」を「積む並びも引数もキューのまま・同じ窓が 2 度入った列でも Z の結果は 1 本ずつと同じ」に直した。本番のコードは変えない（`apply_sequentially` への回しも `has_repeated_window` も足さない）
- 較正: `apply_as_batch` の積み上げの繰り返しを `.rev()` にする → 門のテストが赤（キュー側 `[1, 0, 3, 2]`）。戻して `command.rs` の差分が説明の行だけであることを確かめた
- 消したもの: なし（git grep 不要）
- テストの数: `cargo test -p wintf --lib command_batch_tests` 8 → 9（全部緑）
- レビュー 1 回目の差し戻し: 説明が 1 本の列の測りを一般の決まりにしていたので、`apply_as_batch` の doc を「列 `[A を B の後へ][C を A の後へ][A を D の後へ]` では同じ順に着く・Wine・ReactOS の読みはこの列の実際の Windows の結果に合わない・ほかの列は確かめていない」に絞り、`flush_window_pos_commands` の doc は確かめた範囲を `apply_as_batch` の doc へ指すだけにした（説明だけ・較正のやり直しは不要）

## 5.1

- 8: `window/window_handle.rs` の `on_window_handle_remove` から `PostMessageW(hwnd, WM_CLOSE)` の 3 行と「ウィンドウクローズを非同期で要求」の説明を消した。debug の文を `"Entity being removed, window handle released"` に、通知の説明を「ウィンドウカウント更新。OS の窓の破棄は登録簿の掃除（`reconcile_window_registry`）が `RemovedComponents<Window>` を見て行う」に直した。`forget_last_shown` と `on_window_destroyed` は残した。登録簿の掃除による破棄の道は変えていない
- `window_proc/lifecycle.rs`: 陳腐化した id の段落を「despawn から登録簿の掃除までの間に利用者や OS が投げた `WM_CLOSE` が届くと来る」に直した。同じファイルの「`WM_CLOSE` の投函契約と窓破棄の正準シグナルは変更していない」から「投函契約と」を外し、テスト `wm_close_skips_stale_entity_as_normal_teardown` の説明の「経路」と本体アームの 1 行の注も同じ形に直した。`DESPAWNED_SKIP_TAG` とその説明（`:34`・`:38`）の差分は 0
- 消す前の確かめ（直す前の木）: `git grep -n "WM_CLOSE"` で、ハンドルの外れから投げる行は `window_handle.rs:280` の 1 件だけ（ほかの `PostMessageW(.., WM_CLOSE, ..)` は `tooltip/os_tests.rs:446`・`shiori-host32-ipc/src/send_timeout_tests.rs:101` の自前の窓を閉じるテストだけ）。`git grep -n "on_window_handle_remove"` の使い手は `window_handle.rs:21` のフックの登録だけ。`"sending WM_CLOSE"` の文字列を読むテスト・道具は無い（`.kiro/specs/completed/` の古い記録とログにだけ出る）。直した後の木で `git grep -n "PostMessageW" -- crates/wintf/src/ecs/window` は 0 件
- 較正: 足したテスト・確かめを直したテストは無い（消すだけ）ので当たらない
- テストの数: `cargo test -p wintf --lib window_proc::lifecycle` 10 緑・`window::window_handle` 1 緑・`cursor::answer` 12 緑・`zorder_pair_establish` 9 緑・`zorder_pair_maintain` 33 緑（3 ignored）。増減なし
- 消した投函に合わせて直した説明（コードは変えない・行は増やさない）: `clickthrough/controller.rs:91`（窓の close の起点を「登録簿の掃除 `reconcile_window_registry` が `DestroyWindow` する」に）・`window/zorder_pair_maintain_detach_tests.rs:298-299`（「除去フックが `WM_CLOSE` を投函する」を「破棄は登録簿の掃除が行う」に）・`areka/src/menu/trigger_flow_tests.rs:805-807`（「投函するので空のハンドル」を「何も投函しない・実在の窓は要らないので空のハンドル」に）・`areka/examples/mock-shell.rs:423,437`・`areka/examples/emo-present/input.rs:18`（流れを「despawn → `reconcile_window_registry` が `DestroyWindow` → `WindowRegistry` 空遷移」に）
- 直した後に `git grep -n -i "WM_CLOSE" -- crates` を引き直した: 残りはウィンドウ手続きの受け手（`window_proc/lifecycle.rs`・`mod.rs:52`・`world/tick_wake.rs`）・OS の閉鎖要求の説明（`window/components.rs:129`・`runtime/mod.rs:258` の「WM_CLOSE の despawn」＝受け手の despawn）・自前の窓のテストと道具（`tooltip/os_tests.rs`・`shiori-host32-ipc`・`pilot`・`real-run-kit`）だけで、ハンドルの外れから投げると書く説明は 0 件
- 確かめ: `cargo build -p wintf -p areka --tests --examples` 緑・`cargo fmt --all -- --check` 緑

## 5.2

- 9: `crates/wintf/src/ecs/window/window_handle.rs` の `mod tests` に `framed_window_client_to_window_round_trips` を 1 本足した。`crate::api::with_test_window` で `WS_OVERLAPPEDWINDOW` の見えない実窓を作り、クライアント (100,200) 300×150 を `client_to_window_coords` で外枠へ、`window_to_client_coords` でクライアントへ戻すと元の値に戻ることを見る。外枠がクライアントより広い（差が 0 でない）ことも先に確かめる。`#[ignore]` なし。
- 9 の較正: `window_to_client_rect` の `left: window_rect.left - left_diff` を `+ left_diff` に変えると `framed_window_client_to_window_round_trips` が赤（位置 `(84, 200)` ≠ `(100, 200)`）。明示の編集で戻し、`git diff` で戻ったことを確かめた。
- 10: 消す前の `git grep -n "from_hwnd"`: コード（crates・examples・tools・テスト・fixture・設定）では `crates/wintf/src/ecs/window/components.rs:218` の定義の 1 か所だけ。ほかの当たりは `.kiro/specs/completed/` の過去の文書（`WindowStyle::from_hwnd` の記述と、別物の `get_entity_from_hwnd`・`WindowCoordinateConverter::from_hwnd`）と、この spec の文書だけ。`impl WindowStyle { from_hwnd }` を消し、それだけが使っていた `use crate::api::*;`（components.rs）も消した。消した後の `git grep "from_hwnd"` はコードで 0 件。
- テストの数: `cargo test -p wintf --lib window_handle` 10 → 11（1 本足した）。`cargo test -p wintf --lib ecs::window` 474 passed・6 ignored。

## 5.3

- 52: `crates/wintf/src/ecs/window/window_system.rs` の関数の中の `type Pending` を関数の外の `type PendingWindow`（`With<Window>`・`Without<WindowHandle>`・`Without<WindowCreationFailed>` の 3 条件）へ出し、`create_windows(world: &mut World, pending: &mut QueryState<Entity, PendingWindow>)` にした。本体の `SystemState::new` と `.get(world).expect(..)` を `pending.iter(world).collect()` に替え、`use bevy_ecs::system::SystemState;` を消した。self-ref の確かめは今どおり先。`UISetup` への登録（`ecs/world/mod.rs:220`）は変えていない。
- 直前の姿: `window_system.rs:41` に `let mut system_state: SystemState<Query<Entity, Pending>> = SystemState::new(world);`。直した後の `git grep -n "SystemState" -- crates/wintf/src/ecs/window/window_system.rs` は 0 件。
- 直に呼ぶテストを `run_system_once(create_windows).unwrap()` へ替えた: `window_system.rs` の 1 か所、`runtime/window_factory_creation_tests.rs` の 2 か所（範囲の外・完了した `audit-wintf-runtime` の持ち物）。tasks.md に載っていない `runtime/window_factory.rs` の `create_windows_new_path_spawns_registers_and_handles_without_class_error` にも直に呼ぶ所が 2 か所あり、引数が増えてビルドが通らなくなるので同じ形へ替えた（範囲の外）。
- 較正: 振る舞いを変えない直し（design.md「直したテストの較正」で 52 は既にあるテストが緑のままであることで確かめる組）なので、壊し方は入れていない。
- 消したもの: 関数の中の型の別名と `SystemState` の import だけ（公開の口ではない）。`git grep -n "create_windows(&mut"` は直した後 crates で 0 件。
- テストの数: `cargo test -p wintf --lib window_system` 1 → 1、`--lib window_factory` 12 → 12（どちらも緑）。`cargo build -p wintf -p areka -p areka-placement --tests` 通る。

## 6.1

- 直したもの（2 番）: `window/window_pos/mod.rs` の `WindowPos::to_window_coords_for_creation`。位置も大きさも既定（`x == CW_USEDEFAULT`・`width == CW_USEDEFAULT`）ならそのまま返し API を呼ばない。それ以外は空の `RECT` を `AdjustWindowRectExForDpi` に渡して枠の 4 辺を得て、与えた側にだけ足す（既定の側は `CW_USEDEFAULT` のまま）。API の失敗は今どおり `warn!` を残して元の値を返す。口の形は同じ。古い説明（「CW_USEDEFAULTが含まれる場合は変換せずそのまま返す」・「dpi=96により」）は消した。呼び手（`runtime/window_factory.rs:290`）と `initial_pos_flags` は触っていない。`window_factory.rs:14-16` の「位置だけ・大きさだけの窓も、出た後の刻みで跳ばない」は、この直しで枠つきの窓でも成り立つようになったので直していない。
- テスト（`window/window_pos/tests.rs`）: 「position.x だけ CW_USEDEFAULT でも素通し」の 1 本を消し、`WS_OVERLAPPEDWINDOW`・96 DPI の 2 本に改めた。`test_to_window_coords_for_creation_position_only_adds_frame_to_position`（位置だけ → 両方を与えたときの (x, y)・大きさは既定）と `test_to_window_coords_for_creation_size_only_adds_frame_to_size`（大きさだけ → 位置は既定・両方を与えたときの (w, h)）。期待値は道具 `overlapped_both_given`（両方を与えた値を求め、枠が 0 でないことを確かめる）から出す。
- 較正: ⑴ 直す前のコード（素通し）で 2 本とも赤（位置だけ: 得た (500, 50, 既定, 既定)・期待 (492, 19, 既定, 既定)／大きさだけ: 得た (既定, 既定, 300, 200)・期待 (既定, 既定, 316, 239)）。⑵ 直した後に早戻りの条件を `!position_given || !size_given`（片方でも既定なら素通し）へ壊して 2 本とも赤、戻して緑。赤になったテストは上の 2 本。
- 両方を与えたときの値が今と同じこと: 直す前のコードの両方を与えた値は (492, 19, 316, 239)（⑴ の赤の期待値）。直した後に一時の `assert_eq!(result, (492, 19, 316, 239))` を入れて緑を確かめ、外した（この値は OS の枠の寸法によるので残さない）。
- 消す前の確かめ: 消したのはテスト 1 本と説明だけ（公開の口ではない）。`git grep -n "to_window_coords_for_creation"` の使い手は `runtime/window_factory.rs:290` と本ファイルのテストだけ。
- テストの数: `cargo test -p wintf --lib window_pos` 57 → 58（緑）。fmt・clippy（このファイルの警告 0）通る。

## 6.2

- 14: `WindowPos::set_window_pos`（公開の口・`SetWindowPos` を直に呼ぶ）を消した。私有の `build_flags` の本体を `build_flags_for_system` へ移し、私有の方を消した。`build_flags_for_system` の呼び手 3 つ（`graphics/systems/window_pos.rs:76`・`zorder_chain.rs:479`・`zorder_pair_maintain.rs:207`）とテスト（`window/window_pos/tests.rs`）は変えていない。
- 消す前の確かめ: `git grep -nE "(^|[^_])set_window_pos\(|build_flags\("`（ツリー全体）で、`crates/` の使い手は定義の `window/window_pos/mod.rs` の 2 か所（`set_window_pos` の定義と、その中と `build_flags_for_system` の中の `build_flags()`）だけ。ほかの当たりは `raise_reset_window_pos(`（別の関数）と `.kiro/specs/completed/` の古い記録だけ。examples・tests・fixture・設定に使い手 0。消した後は同じ grep で `crates/` の定義・呼び出し 0（`raise_reset_window_pos` を除く）。
- 16: `window/dpi.rs` の `DpiSuggestedRectPolicy::ApplyPosition` の説明に 2 行足した（直後の `WM_WINDOWPOSCHANGED` で中心を保つ補正が掛かり左上は大きさの差の半分ずれる・左上を保ちたい窓は `ExternalAuthority`）。振る舞いは変えない。
- 17: `window/window_pos/mod.rs` の `SetWindowParentToLayoutRoot::apply` の、ありもしない `ensure_window_parent_system`（`git grep` で `crates/` に 0）を約束する説明を消し、`LayoutRoot` が無い枝に「親を付けない（本番は `EcsWorld::new` が先に作る前提）」の説明と `debug!(entity, "No LayoutRoot; Window entity left without ChildOf")` を足した。
- 21: `window/dpi.rs` のテスト `test_to_physical_rounds_half_away_from_zero` の説明を「1.5 は 0 から遠い側へ（`f32::round`。偶数丸めの `round_ties_even` ではない）」に直した。
- 較正: テストは足しも直しもしていない（説明と削除だけ）ので較正なし。
- テストの数（変化なし）: `cargo test -p wintf --lib ecs::window::window_pos` 17・`ecs::window::dpi` 10・`ecs::window_proc::dpi_helpers` 22、すべて緑。`cargo build -p wintf -p areka -p areka-placement --tests` 警告 0・fmt 通る・clippy で 2 ファイルの警告 0。

## 6.3

- 15: `window/monitor.rs` の `enumerate_monitors` の 2 か所の `warn!` を `error!` にした（1 台ぶんは `hmonitor = ?hmonitor, error = %e`＝`MonitorError` の値を残す・`EnumDisplayMonitors` の失敗は、API が拡張の誤りを返さないので文だけ）。口の形と返す値（失敗で空の `Vec`）は変えない。`Monitor` の定義（欄・メソッド）の差分 0。
- 範囲の外の追従（`layout/systems/monitor_systems.rs`）: `apply_monitor_snapshot` の戻り値を `usize` から `Option<usize>` にし、頭で `new_monitors.is_empty()` なら表を変えずに `None` を返す。`detect_display_change_system` は `None` なら旗を下ろさずに戻り（次の刻みで読み直す）、`Local<bool>`（`zero_streak`）で 0 台の続きを覚えて `warn!`（No monitors enumerated…）と頭の `info!`（Display configuration changed…）を続きの最初の 1 回だけ出す。`LayoutRoot` が無い枝（旗を下ろす）では続きを解く。引数が 8 個になったので、同じ crate の型（`clip_sync.rs`・`zorder_chain_apply.rs`）に倣い `#[allow(clippy::too_many_arguments)]` を付けた。
- テスト: `monitor_systems_tests.rs` に `empty_snapshot_keeps_monitor_table_and_returns_none` を 1 本足した（空の一覧で `None`・`Monitor` の entity が残り値も変わらない）。受け皿 `RedriveCount` を `Option<usize>` にし、既存の 3 か所の判定を `Some(1)`／`Some(0)` に直した。
- 較正: 0 台の判定を `if false && new_monitors.is_empty()` に壊すと `empty_snapshot_keeps_monitor_table_and_returns_none` が赤（`left: Some(0)`・`right: None`）。明示の編集で戻し、差分が意図どおりに戻ったのを確かめた。
- 18: `monitor.rs` の冒頭の使用例を `wintf::ecs::window::monitor::{Monitor, enumerate_monitors}` に直した（`rust,ignore` のまま）。
- 消したもの: なし（git grep 不要）。古い綴り `ecs::monitor::` は `crates/` に 0（完了した spec の文書にだけ残る）。
- テストの数: `cargo test -p wintf --lib monitor` 38 → 39（足した 1 本）・`monitor_systems` 21 → 22、すべて緑。`cargo test -p wintf --test window` 20 緑。`cargo build -p wintf -p areka -p areka-placement --tests` 警告 0（列挙の呼び手は直さずに通る）・fmt 通る・clippy で触った 2 ファイルの警告 0。

## 7.1

- 3: `window/zorder_chain_apply.rs` の `apply_zorder_chain` で、持ち越しの判断を「1 本も書けなかったとき」の枝の外へ出した。鎖の付与に窓ハンドル待ちが 1 本でも残る巡は、書けた本数に依らず `plan.dirty = true`・`carry_announced = true`、残らない巡は両方 false。書けた分は次の巡には帳簿に載るので差分に現れず、待つ間は窓口を呼ばない（見送りの記録は `carry_announced` で待ち始めの 1 回だけ・`execute_ops` は変えていない）。後押しは「鎖か組のどちらかが窓口を呼んだ巡」で今のまま（`acted || pair.acted`）。
- 53: 呼び出し元で `ExecOutcome { acted, handle_missing, missed: pair.missed }` を組むのをやめ、ローカルの `acted`・`handle_missing` の 2 つにした。`ExecOutcome` の型そのもの（`execute_ops` の戻り値・`detached.missed` は読まれる）は残した。長い説明を新しい規則に書き直し、`NoChange` の枝の「出口は 3 つ」を 2 つに、`execute_ops` の説明の「まだ一度も書けていない」の参照を直した。
- 説明の追従（コメントだけ）: `zorder_chain_apply_nudge_tests.rs` の冒頭の主題 2・節の見出し・`a_wait_ended_by_a_non_carrying_pass_also_re_arms_the_record` の説明の ⑴ を新しい規則に直した。`zorder_chain.rs:115` の `dirty` の説明は 7.2（23 番）の持ち場なので触っていない。
- テスト: `zorder_chain_apply_tests.rs` の `an_edge_without_handles_is_skipped_with_a_reason_and_the_rest_are_linked` に「印が残る」の主張を 1 つ足し、`zorder_chain_apply_nudge_tests.rs` に `the_remaining_link_is_made_once_the_late_handle_attaches_and_the_mark_drops` を 1 本足した（偽の OS・1 つの schedule で 3 巡: a←b を書いて b←c は待つ → 待つだけの巡 → c にハンドルが付いた巡で b←c。`SetOwner` は (0x10,0x20)・(0x20,0x30) の 2 回だけ・見送りの記録 1 本・張った記録 2 本・最後に印が落ちる）。`zorder_chain_order_tests.rs`（977）と `zorder_pair_maintain_tests.rs`（978）は触っていない。
- 赤（直す前）: 足した 2 つが赤（52 緑・2 赤）。
- 較正: 持ち越しを「書けなかったとき」の中へ戻す（`plan.dirty = handle_missing && !acted;`・`*carry_announced = handle_missing && !acted;`）と、`an_edge_without_handles_is_skipped_with_a_reason_and_the_rest_are_linked` と `the_remaining_link_is_made_once_the_late_handle_attaches_and_the_mark_drops` の 2 本が赤。明示の編集で戻し、差分が意図どおりに戻ったのを確かめた。
- 消したもの: 関数・型・口は消していない（呼び出し元の構造体の組み立てだけ）ので git grep 不要。
- 印を立てる `areka` の側（`crates/areka/src/runtime/frame/zorder_drain.rs`）は直していない。
- テストの数: `cargo test -p wintf --lib zorder_chain_apply` 53 → 54（足した 1 本）すべて緑・`cargo test -p areka zorder_drain` 34 緑。`cargo build -p wintf -p areka -p areka-placement --tests` 警告 0・fmt 通る・clippy の触ったファイルの警告は前からある `field_reassign_with_default`（`zorder_chain_apply_tests.rs:387-388`・`zorder_chain_apply_nudge_tests.rs` の末尾の 1 本）だけで、足したものは 0。

## 7.2

- 22: `zorder_chain_apply.rs` の節の見出し・窓口の節の見出し・替え玉の器の説明の「5」を「6」に、`zorder_chain_apply_tests.rs` の冒頭の窓口の列に「生の 1 つ奥の読み取り」を足して 6 つにした。
- 23: `zorder_chain.rs` で触ったのは `ZOrderChainPlan::dirty` の説明（窓ハンドル待ちの付与が 1 本でも残る巡は書けた本数に依らず true へ戻す＝7.1 の後の規則）と、`log_chain_absent` の説明のパス（`emo2_boot/frame/zorder_drain.rs` → `runtime/frame/zorder_drain.rs`）の 2 か所だけ。`audit-placement-core` の 73・67 の行には触れていない。`zorder_chain_apply.rs` の前例の場所は行番号をやめて `window_proc/window_pos.rs` の `WM_DPICHANGED` に替えた。
- 24: `zorder_chain_diag.rs` の `settled_line` の説明 2 行を「鎖の根」「錨＝根の 1 つ手前。錨の直後が既に根なら先頭」に直し、`zorder_chain_diag_tests.rs` の `the_settled_line_carries_the_declaration_and_the_measurement_together` の例の値を `nudged_hwnd=0xC0 insert_after=0xB0` に揃えた（行の書式は変えていない）。
- 26: `zorder_pair_diag.rs` の `UNKNOWN` と `tristate_field` を `pub(crate)` にし、`zorder_chain_diag.rs` は `hwnd_field` と同じ `use` で借りて自前の 2 つを消した（字面は同じ `-`）。
- 27: `pair_handover_tags` の説明の 1 行を「既存の 7 語の名簿は、隣接の spec が字面を固定したテストが読む（実機の手順は字面を自前で持つ）」に直した。
- 54: `zorder_chain_tests.rs` の `the_nudge_moves_only_the_zorder` で `SWP_NOZORDER` を生の数値で言い直す 6 行を消し、理由の 1 行コメントを上のフラグの全一致の主張の上へ移した。`SET_WINDOW_POS_FLAGS` の import も消した。
- 説明の追従（コメントだけ）: `zorder_chain_apply_nudge_tests.rs` の `a_wait_ended_by_a_non_carrying_pass_also_re_arms_the_record` の「経路は 3 つ」を本番の「出口は 2 つ」（⑴ 待ちの付与が残らない巡＝書けた形と持ち越さない形・⑵ `NoChange`）に揃え、次の 1 本の見出しと「⑵ の 1 本」の参照も直した。
- 較正（24）: `settled_line` の `nudged` と `after` の材料を入れ替えると、`the_settled_line_carries_the_declaration_and_the_measurement_together` と `the_four_signoff_fields_of_the_settled_line_stay_adjacent_in_this_order` の 2 本が赤。明示の編集で戻し、差分が意図どおりに戻ったのを確かめた。
- 消す前の git grep: 鎖の側の `UNKNOWN` の使い手は `zorder_chain_diag.rs` の中と子のテスト `zorder_chain_diag_tests.rs`（`use super::UNKNOWN`・借りた名前で解決する）だけ、`zorder_chain_diag::UNKNOWN` を名指す場所は 0。鎖の側の `tristate_field` の使い手は `settled_line` だけ。`SET_WINDOW_POS_FLAGS` は `zorder_chain_tests.rs` では消した 2 行だけ。消した後、鎖の側に番兵と三値の欄の定義は 0（`git grep "fn tristate_field\|const UNKNOWN"` は `zorder_pair_diag.rs` の 2 行だけ）。
- テストの数: `cargo test -p wintf --lib zorder_chain` 123 緑・`--lib zorder_pair` 94 緑（3 ignored・本数は変わらない）。`cargo build -p wintf --tests` 警告 0・fmt 通る・clippy の触ったファイルの警告 0。`zorder_chain_order_tests.rs`（977）と `zorder_pair_maintain_tests.rs`（978）は触っていない。

## 8.1

- 25: `zorder_chain_order_tests.rs` の `chain_schedule`・`spawn_window`・`publish_full_chain`・`place_after` を `pub(super)` にした（修飾子だけ・977 行のまま）。`zorder_chain_order_lifecycle_tests.rs` の `Scope`（欄 2 つも）・`spawn_scopes`・`scope_chain_schedule`・`balloon_offsets` を `pub(super)` にした。
- outsider: 自前の `chain_schedule`・`spawn_window`・`publish_full_chain` を消し、兄弟から借りた。使わなくなった import（`Schedule`・`SingleThreadedExecutor`・`HINSTANCE`・`apply_zorder_chain`・`ChainPlan` ほか）を消し、doc だけが指す `ZOrderChainPlan` はパスつきのリンクにした。道具立ての見出しを「受け口を作らない用意と記録の絞り込みだけ」に直した。
- inversion: 少し変えた写し 3 つは引数で表せたので写しを残さなかった。`schedule` は `scope_chain_schedule` と中身が同じ。`spawn_scopes` は lifecycle 版を `count = 2` で呼び、`Scope` の列を `[b0, s0, b1, s1]` の entity 列へ 1 行で平らにする。`balloon_offsets` は lifecycle 版に `scope_count = 2` を渡す（添字 `2i`／`2i+1` が `B0,S0,B1,S1` と一致）。`place_after` は基のファイルから借りた。module doc に借りる先を足した。
- 説明の追従（コメントだけ）: lifecycle の `spawn_scopes` の doc「各テストは `arrange_z` で始点を組む」を、inversion が組の確立から始める形も含む書き方に直した。
- 較正: 足したテスト・直したテストは無い（足場を借りる形に変えただけ）ので当たらない。
- 消す前の git grep: 消した 7 つはどれも `pub` の無い private な関数で、そのファイルの外からは名指せない。同じ名前の関数は他の多くのテストファイルにもそれぞれ自前で在る（`window_band_tests.rs`・`zorder_pair_*_tests.rs`・`crates/areka*` ほか）が、別のモジュールの別物で指摘 25 の範囲の外。消した後、outsider と inversion に借りた項目の自前の定義は 0（`fn (chain_schedule|spawn_window|publish_full_chain|place_after|schedule|spawn_scopes|balloon_offsets|scope_chain_schedule)` の grep が 0 件）。
- テストの数: 前（`--list`）＝基 4・lifecycle 3・outsider 2・inversion 2 の 11 本。後＝`cargo test -p wintf --lib zorder_chain_order` 11 緑（同じ 11 本）。行数: inversion 276→209・outsider 427→379・lifecycle 661→665（`spawn_scopes` の署名が fmt で 4 行に折れた分）・基 977→977。build 警告 0・fmt 通る・clippy の触ったファイルの警告 0。

## 9.1

- 何を変えたか: 新しい兄弟のファイル `window/zorder_pair_test_support.rs`（304 行）に組の実窓テストの下ごしらえを 1 つずつ置き、`window/mod.rs` に `#[cfg(test)] mod zorder_pair_test_support;` の 2 行を足した（項目は `pub(super)`）。置いたのは research 32 番の一覧（`create_window`・`create_test_parent`・`create_test_child`・`create_hidden_test_child`・`insert_after`・`destroy_test_window`・`fake_hwnd`・`spawn_window`・`declare_pair`・`request_reassert`・`maintain_schedule`・`pair_schedule`・`issue_control_command`・`lines_with`・`assert_line_count`・`CAPTURE_DIRECTIVES`・タグの文字列）と、2 本以上のファイルに同じ中身で写されていたもの（`create_test_window`・`owner_of`・`is_alive`・`hwnd_hex`・`establish_schedule`・`SIGNOFF_DIRECTIVES`・`FIX_TAG`・`SKIP_TAG`・`VERIFY_FAILED_TAG`・`SINK_OBSERVED_TAG`・`OWNER_ESTABLISHED_TAG`・`RELEASED_REASON`・`DETACHED_MARK`）。常に手前のテストの `create_test_parent(title, always_on_top)` は `create_banded_test_parent` として置き、`create_test_parent(title)` はその `false` の形にした。
- 書き込みの記録の文言: `SET_WINDOW_POS_TAG` は `super::command::WINDOW_WRITE_LOG_MESSAGE` そのもの（手写しをやめた）。そのため `window/command.rs:178` の定数を `pub(super)` にした（修飾子だけ・行数 984 のまま）。design.md の Z3 の表に無い `command.rs` への手。
- 測りのテストの可視の前提: `create_visible_test_parent` にあった `assert!(is_window_visible(parent), …)` を共有の `create_banded_test_parent` に移した（`create_test_parent` を通る全ファイルにかかる）。
- 写しを消したファイル（11 本）: 維持（`zorder_pair_maintain_tests.rs`・`_multipair`・`_detach`・`_survivor`・`_always_on_top`・`_release`）・沈降（`zorder_pair_sink_tests.rs`）・測り（`zorder_pair_measure_tests.rs`）・確立（`zorder_pair_establish_tests.rs`・`_establish_release_tests.rs`）・鎖の適用（`zorder_chain_apply_tests.rs`）。各ファイルは `use crate::ecs::window::zorder_pair_test_support::{…}` で借りる。テストの本文と名前を変えないため、ファイルごとに名前が違うものは `as` で受けた（測り: `SIGNOFF_DIRECTIVES as CAPTURE_DIRECTIVES`・`CAPTURE_DIRECTIVES as MAINTAIN_CAPTURE_DIRECTIVES`・`create_test_parent as create_visible_test_parent`・`create_test_child as create_visible_child`・`create_hidden_test_child as create_hidden_child`／常に手前: `create_banded_test_parent as create_test_parent`／生き残り: `create_test_window as create_test_toplevel`／沈降: `SINK_OBSERVED_TAG as SINK_TAG`／維持の解き放ち: `CAPTURE_DIRECTIVES as DIRECTIVES`／確立の解き放ち: `SIGNOFF_DIRECTIVES as DIRECTIVES`・`SKIP_TAG as RELEASED_SKIP`）。維持の解き放ちの `Trio::create`／`Drop` と確立の解き放ちの `create_hidden_popup` は、引数の形が違う自前の `create_window` をやめて共有の道具を呼ぶ形にした（窓の題・スタイルは同じ）。
- 残したもの: ファイルごとの役の道具（`Trio`・`Pair`・`TwoPairs`・`establish_pair`・`spawn_handleless_window`・`retrigger_handle`・`count`・`count_lines`・`released_skips` ほか）と、そのファイルだけの判定語。常に手前のテストの門（`REAL_WINDOW_ZORDER_ENV`・`require_explicit_real_window_run`）は 9.2 の担当なので動かしていない。A-1 の 3 本の `#[ignore]` と本文 1 行目の門の呼び出しも動かしていない。
- テストの本文: 11 本のファイルの `#[test]` 76 本を HEAD と今とで本文（属性から閉じ括弧まで）ごと比べ、違いは 0。
- 較正: 足したテスト・直したテストは無い。共有の道具が組のテストに本当に効いていることを 1 度だけ確かめた: 共有の `issue_control_command` から `flush_window_pos_commands()` を外すと、`cargo test -p wintf --lib zorder_pair` で 13 本が赤（`maintain::tests` の `already_adjacent_pair_issues_no_command_at_all`・`request_for_a_missing_peer_issues_no_command`・`pending_verification_without_an_issued_command_is_dropped_with_a_record`・`request_without_handles_issues_no_command`・`request_on_a_window_without_declaration_is_dropped_with_a_record`・`without_a_request_the_maintenance_does_nothing`／`survivor_tests` の `detaching_a_lost_peer_leaves_the_surviving_windows_display_state_untouched`・`peer_loss_leaves_the_surviving_windows_display_state_untouched`／`sink::tests` の `the_sink_observation_happens_only_once_per_mark`・`the_sink_observation_issues_no_command_and_moves_nothing`／`release_tests` の `a_reshow_request_on_a_released_pair_moves_nothing_and_is_dropped`・`a_pending_verification_on_a_released_pair_is_dropped_without_checking`・`a_sink_mark_on_a_released_pair_is_dropped_unmeasured`）。編集で戻し、94 緑に戻ったのを見た。
- 消す前の git grep: 消した定義はどれも `pub` の無い private な `fn`／`const` で、そのファイルの外からは名指せない。ツリー全体の `git grep`（`crates/wintf/src/ecs/window/` と `.kiro` の外）で当たるのは、別のモジュールの同じ名前の自前の定義（`api.rs` の `create_test_window`・`window_proc/*` の `SIGNOFF_DIRECTIVES`／`CAPTURE_DIRECTIVES`・`areka-placement` の配線テストの `create_test_window` ほか）だけで、消したものへの参照は 0。文字列で読むのは `crates/areka/src/placement_tests/zorder_inversion_branch_coverage_tests.rs` のテスト名だけで、テスト名は変えていない（`cargo test -p areka zorder_inversion_branch_coverage` 3 緑）。
- 説明の追従: 常に手前のテストの節の見出し「兄弟の維持系テストと同じレシピ」を、残った帯の窓の道具の見出しに直した。`command.rs:165` の「`zorder_pair_*_tests.rs` の `SET_WINDOW_POS_TAG`」は、定義が共有のファイルへ移っても各ファイルが同じ名前で使っているので偽にならず、`command.rs` は修飾子だけの約束なので触っていない。
- テストの数: `cargo test -p wintf --lib zorder_pair -- --list` は前後とも 97 本（一覧の差分 0）、`zorder_chain_apply` は前後とも 54 本（差分 0）。走行は前後とも `zorder_pair` 94 緑・3 ignored（A-1 の 3 本）、`zorder_chain_apply` 54 緑。行数: 維持 978→805・multipair 472→335・detach 643→546・survivor 466→293・always_on_top 920→811・release 312→226・沈降 816→647・測り 477→321・確立 524→427・確立の解き放ち 160→116・鎖の適用 819→791。`zorder_chain_order_tests.rs` は触っていない（977）。build 警告 0・fmt 通る・clippy の新しい警告 0。

## 9.2

- 何を変えたか（19）: 実窓の門（`REAL_WINDOW_ZORDER_ENV` と `require_explicit_real_window_run`）を `window/zorder_pair_test_support.rs` に `pub(super)` で 1 つだけ置き、`window_band_real_tests.rs`・`window_band_pipeline_real_tests.rs`・`zorder_pair_maintain_always_on_top_tests.rs` の 3 つの写しを消して `use` する形にした。門の環境変数の綴り `AREKA_WINTF_REAL_WINDOW_ZORDER` は変えていない（差分に出るのは消した 3 行と足した 1 行で、綴りは同じ）。門の失敗文は 3 つのうち一番詳しい常に手前の版を正本にした（判定は「環境変数があるか」だけで 3 つとも同じ）。各テストの `#[ignore]` と本体の 1 行目の門の呼び出しは動かしていない。
- 帯の窓の手伝い 4 つ（`create_ghost_shaped`・`destroy`・`in_band`・`relative_order`）は `window_band_real_tests.rs` の側を正本にして `pub(super)` にし、`window_band_pipeline_real_tests.rs` は `use super::real_tests::{…}` で借りる。正本の `create_ghost_shaped` は `create_toplevel` 経由で同じ拡張スタイル（`WS_EX_LAYERED | WS_EX_TOOLWINDOW`）・同じ位置と大きさ、`relative_order` は `desktop_z()` を絞る形で同じ列を返す。pipeline 側で使わなくなった import を外した。
- 説明の追従: 帯の 2 本のファイルの頭の「兄弟の … と同じ実窓の門」を、門の置き場（`zorder_pair_test_support.rs`）と手伝いの貸し借りの説明に直した。`.kiro/specs/zorder-chain-residue/brief.md:9` の「always_on_top のファイルの該当本が `#[ignore]`＋環境変数で明示実行」は今も正しいので触っていない。
- 較正: 足したテスト・直したテストは無い（design.md「直したテストの較正」の 582 行の扱い＝写しをまとめた指摘は専用のテストを足さない）。門の内側のテストは既定の走行では `#[ignore]` で走らないので、既定の走行での較正はできない。写しが残っていないことはコンパイルと下の `git grep` で確かめた。
- 消す前の git grep: 消したのはどれも private な `fn`／`const`。消した後の `git grep "require_explicit_real_window_run\|REAL_WINDOW_ZORDER_ENV"`（このスペックの文書の外）で当たるのは、共有のファイルの定義 1 組と、3 本のファイルの `use` と門の呼び出しだけ。`create_ghost_shaped`・`in_band`・`relative_order`・`destroy(hwnd: HWND)` の定義は `window_band_real_tests.rs` の 1 つずつだけ（同じ名前の別物＝`command_batch_tests.rs` の `destroy(windows: &[HWND])`・各テストの型の `fn destroy(self)`・`window_band_tests.rs` の偽の OS の `in_band` は別の定義で、触っていない）。
- テストの数: `cargo test -p wintf --lib window_band` は前後とも 16 緑・3 ignored、`cargo test -p wintf --lib always_on_top` は前後とも 6 緑・3 ignored。行数: 帯の実窓 335→325・帯の流れの実窓 242→178（前の行数は HEAD）・常に手前 811→774・共有の道具 304→340。build 警告 0・fmt 通る・clippy の新しい警告 0。

## 10.1

- 4: 確立系 `establish_owner_links` の宣言の問い合わせを `Ref<KeepDirectlyAbove>` にし（clippy の type_complexity を避けて `DeclarationQuery` の型の別名に置いた・`zorder_pair_maintain.rs` の `OwnerLinkQuery` と同じ形）、手順へ入る条件に `declaration.is_added() ||` を足した。条件の変数名を `handle_added` から `added` に改めた。
- 説明の追従（コメントだけ）: `zorder_pair_establish.rs` の区切りの見出し（「Added<WindowHandle> 駆動」→「ハンドルか宣言の付与で駆動」）・「何を契機に動くか」の段落・条件の行のコメント、`zorder_pair_establish_tests.rs` の `skips_handle_missing_pair_while_the_complete_pair_establishes` の説明（「ハンドルが付いた巡だけ動き」→「ハンドルか宣言が付いた巡だけ動き」）。`zorder_chain_apply.rs:415`・`zorder_chain_apply_release_tests.rs:198` の「ハンドルが付いた巡に確立系が張る」は今も正しいので触っていない。
- 足したテスト（`zorder_pair_establish_tests.rs`・実窓 2 枚・既定の走行で走る）:
  - `establishes_when_the_declaration_is_added_after_the_handles`（4）: ハンドルの付いた窓 2 枚で 1 巡 → 宣言を付けて 1 巡 → もう 1 巡。`owner-established` がちょうど 1 本・OS 側の owner がキャラ窓・`ReassertZOrder` が入る。
  - `retries_and_establishes_once_the_handles_are_replaced`（29・本番は変えない）: null のハンドルで失敗（`owner-establish-failed` 1 本・印あり）→ 両窓のハンドルを実窓の別のハンドルへ付け直し（除去 → 付与）1 巡 → `owner-established` がちょうど 1 本・失敗 0 本・OS 側の owner がキャラ窓。
- Feature Flag の手順: 旗 OFF で 4 のテストが赤（`establishes_when_the_declaration_is_added_after_the_handles` が `zorder_pair_test_support.rs:334` の本数の検査で panic）→ 旗 ON で 11 本緑 → 旗を消して 11 本緑。
- 較正: ⑴ `declaration.is_added() ||` を外す → `establishes_when_the_declaration_is_added_after_the_handles` だけが赤（10 緑 1 赤）。⑵ 比べを `failed.is_some()` にする → `retries_and_establishes_once_the_handles_are_replaced` だけが赤（10 緑 1 赤）。どちらも戻して 11 本緑、`git diff` は意図した変更だけ。
- 消したもの: 無い（git grep 不要）。
- テストの数: `cargo test -p wintf --lib zorder_pair_establish` 9 → 11 緑、`cargo test -p wintf --lib zorder` 233 緑・3 ignored（足した 2 本を含む）、`cargo test -p areka-placement` 1008 緑・1 ignored。行数: `zorder_pair_establish.rs` 224 → 237・`zorder_pair_establish_tests.rs` 427 → 512。`zorder_pair_tests.rs` は触っていない。fmt 通る・clippy の新しい警告 0。
- 振る舞いの差（記録だけ）: 宣言を窓の生成前に付ける areka の経路（`spawn.rs:593`）では、宣言の付いた巡にまだハンドルが無ければ `reason=HandleMissing` の見送りが組ごとに 1 本増えうる（実機では確かめていない・窓の振る舞いは変わらない）。

## 10.2

- 28（説明だけ・案の切り替えと `raise_assist` は残す）: `zorder_pair.rs` の `ZOrderPairStrategy` の説明（本体の段落に「z の変化を知らせる仕組みは無いので、案 B と `raise_assist` が効くのは判断の表の上だけ」を足し、`OwnerLink` を「`raise_assist` は判断の表にだけ効く・true も false と同じ動き」、`ExplicitMaintenance` を「owner を張らず、`ReassertZOrder` が挿されたときだけ組を直す・初めの要求も入らない」に）と `decide_pair_fix` の説明（「案 A・raise assist 無効ではそもそも z 変化検知を結線しない」→「z 変化検知はどの案でも結線されていない」）。`zorder_pair_establish.rs` の案 B の判定のコメントに「案 B では初めの `ReassertZOrder` も入れず、z の変化を知らせる仕組みも無いので、組は外から要求が挿されたときだけ直る」を足した。
- 30: `PairTrigger::Establish` を消し、`decide_pair_fix` の腕を `(PairTrigger::Reassert, _)` だけに、判断の表の行を `Reassert` だけにした。`#[allow(dead_code)]` の上の 2 行を「z の変化の 2 種は案 B の語彙で、完了 spec `areka-P0-ghost-window-zorder` の要件 5.6 の案の切り替えを残す 2026-11-14 の討議の決め 2 で残す。挿す側は無い」に。テスト（`zorder_pair_tests.rs`）: `ALL_TRIGGERS` を 3 種に・`..._on_establish_and_reassert` を `..._on_reassert` に改名して `Reassert` だけを回す・帯の表のテストの数を 7 組→4 組（Reassert×3 案＋RaisedBelow×案 B）・ハンドル未付与のテストの基の観測を `Reassert` に。
- 31（コードは変えない）: `zorder_pair.rs` の `ReassertZOrder` の説明（挿入元 ①②③ の並び→本番で挿すのは確立系だけ・②の「`areka-P0-balloon-visibility` の着地後に実機確認」を消し「areka の表示の切り替えはこの要求を挿さない」に・③の見出しを「z の変化を知らせる仕組みは無い」に）、ツリーに無い `compute_pair_z_intent` の行を消した、記録の語彙の節の「結線は後続タスクの担当」→「呼ぶのは確立系・維持系」、`record_decision` の「見送り 5 種」→「6 種」。`zorder_pair_tests.rs:11` の「適用系は後続タスクの担当」→「維持系のテスト（`zorder_pair_maintain_tests.rs` ほか）が持つ」。`zorder_pair_deferred_vocabulary_tests.rs:321` の「（ペア・グループ）」→「（ペア・鎖＝グループの後継）」。
- 追従（コメントだけ）: `zorder_pair_establish.rs` の確立の挿入のコメント「他の挿入元（要件 2.6 の再表示・z 変化検知）」→「外から挿す口（要件 2.6 の再表示・今は挿す側が無い）」。
- 消す前の git grep: `git grep -n "PairTrigger::Establish\|Establish\b"`（ツリー全体・areka／areka-placement 含む）で当たったのは `zorder_pair.rs` の定義・表の行・腕と `zorder_pair_tests.rs` の 3 か所だけ（ほかは `.claude/skills/kiro-impl/SKILL.md` の英語の "Establish repo baseline" で別物）。消した後 `git grep "PairTrigger::Establish" -- crates` は 0 件（spec の文書だけが名を挙げる）。
- 較正: `(PairTrigger::RaisedBelow, ExplicitMaintenance) => place_above` を `Skip(EchoOrIrrelevant)` に変える → `decide_pair_fix_follows_the_band_table_for_the_place_above_arm`（4 組の数の検査）と `decide_pair_fix_handles_raised_below_per_strategy` が赤（10 緑 2 赤）。明示の書き戻しで戻し、退避の写しとバイトで一致を確かめた。
- テストの数: `cargo test -p wintf --lib zorder_pair` 96 緑・3 ignored（足し引き 0・改名 1）。行数: `zorder_pair.rs` 929→922・`zorder_pair_tests.rs` 934→933・`zorder_pair_establish.rs` 237→239。fmt 通る・build 警告 0・clippy の新しい警告 0。

## 11.1

- 直したこと（5）: `zorder_pair_sink.rs` の純関数 `is_behind_foreground` に `foreground_visible: bool` を足し、前面窓が取れない行（`None`）を先に判定したあと、前面窓が不可視なら `None`（判定できない）を返す。説明の表に「前面窓が不可視」の行を足した。呼び手 `observe_marked_pair_sinks` は `foreground.is_some_and(is_window_visible)` を渡す（呼び手はこの 1 か所だけ）。
- 説明の直し 2 か所: `zorder_pair.rs` の `FrontScan::windows`（「判定に使う前面窓は必ず可視」）と `zorder_pair_sink_tests.rs` の走査のテストの説明（「前面窓は必ず可視ゆえ判定は変わらない」）を、不可視の前面窓は判定側が「判定できない」に倒す、に直した。
- テスト: `is_behind_foreground_reports_absence_rather_than_a_false_negative` に「前面窓が不可視 → `None`」の 1 行を足した（既存の呼び出しは `true`・前面窓が無い行は `false` を渡す）。
- 較正: 本番で引数を無視する（`let _ = foreground_visible;` で判定に使わない）形にして走らせ、`is_behind_foreground_reports_absence_rather_than_a_false_negative` が赤（`left: Some(false)` / `right: None`）。判定を入れて緑に戻した。
- 消したもの: なし（git grep 不要）。
- テストの数: `cargo test -p wintf --lib zorder_pair_sink` 13 本緑（本数は変わらない・1 本に行を 1 つ足した）。`zorder_pair` で 96 本緑・3 本 ignored。

## 11.2

- 33: `zorder_pair_sink_tests.rs` の前面窓の相対のテストの説明を「可視の子窓（トップレベル窓ではない）」に直した。
- 34: 出力先の理由の文を「出力先は組のモジュール 1 つに揃える（読み手の絞り込みは前方一致なので漏れはしないが、記録の関数を 1 つのファイルに集めて固定する）」に直した。指摘の 5 か所（`zorder_pair.rs` の頭と組立の節・`zorder_pair_sink.rs` の頭・`zorder_pair_record_tests.rs` の `LOG_TARGET` と出力先の検査）に加え、同じ誤った理由を書いていた `zorder_pair_diag.rs` の頭と `zorder_pair_maintain.rs` の頭も直した（説明だけ）。`zorder_pair_record_tests.rs` の頭の 9 行目と検査のテストの説明も「組のモジュールの出力先」に揃えた。
- 34 の検査: `LOG_TARGET` を区切りの `:` まで含めた `"wintf::ecs::window::zorder_pair:"` にし、`every_record_is_visible_under_the_signoff_directive` で 6 種の判定語の**行ごとに**この出力先を含むことを確かめる形にした（前は出力全体への部分一致で、兄弟の出力先の行も通っていた）。
- 35: 見送りの理由のテストの対照の説明を、数を書かない「理由の数だけの行がすべて相異なる」に直した。
- 55: `measure_nearest_visible` と `measure_windows_in_front` の走査の失敗の `warn!` を、`hwnd=` が始めた窓・`at=` が失敗した窓の 2 欄にした（上限の `warn!` の `hwnd=` と意味が揃う）。`[zorder-pair]` のタグは無いので実機の grep の語は変わらない。この行を見るテストは無い（`git grep` で「走査に失敗」「at=」を wintf に 0 件）ので、既存のテストが緑のままであることで確かめた。
- 56: `zorder_pair_record_tests.rs` の先頭に `const ALL_REASONS: [(SkipReason, &str); 6]` を置き、`skip_line_names_a_distinct_reason_for_every_skip` と `record_decision_reports_every_skip_reason_and_yields_no_fix` の 2 つの一覧をこれに替えた。テストのファイルで `SkipReason::ReleasedByChain` が現れるのは 1 か所だけ（`git grep -c`）。
- 57: `zorder_pair_sink_tests.rs` の自前の `WA_INACTIVE`／`WA_ACTIVE` を消し、`windows::Win32::UI::WindowsAndMessaging::{WA_ACTIVE, WA_INACTIVE}` を使う（`drag/reentry.rs` と同じ・新しい feature は不要）。`dispatch_activate` の引数を `u32` にし、`WPARAM` へ入れるときに 1 度だけ広げる。
- 消す前の確かめ: 自前の `WA_*` 定数はそのファイルの私的な定数で、消す前に同じファイルの 58・60 行で定義・同じファイルの 3 か所だけで使っていた。消した後の `git grep -n "const WA_" -- crates` は別の意味の `WA_BOTTOM`／`WA_LOW`／`WA_HIGH`（areka・areka-placement のテスト）だけで、活性の定数の自前の定義は 0 件。
- 較正（34）: 本番の `log_sink_observed` の `debug!` に `target: "wintf::ecs::window::zorder_pair_sink"` を足して走らせ、`every_record_is_visible_under_the_signoff_directive` が赤（`[zorder-pair] sink-observed` の行が組のモジュールの出力先へ出ていない）。前の検査（出力全体に `wintf::ecs::window::zorder_pair` を含む）は、他の 5 種の行が前方一致の名を含むのでこの壊し方でも緑のままだった形。`target:` の 1 行を消して緑に戻し、`git diff` で本番の差分が意図した直し（説明と 55 の欄）だけであることを確かめた。
- テストの数: `cargo test -p wintf --lib zorder_pair_sink` 13 本緑・`zorder_pair::record_tests` 13 本緑（本数は変わらない）。`cargo build -p wintf --tests`・`cargo fmt --all -- --check` 通過・clippy は自分が足した警告なし。

## 12.1

- 36: `zorder_pair_maintain.rs` の旗の立て直しの説明から `spawn.rs:721-728` と 2 本の chain の字面を外し、「`areka-placement` の組の結線では確立系が本 system より前に走る」に直した（B-6 を閉じる・台帳は 16.2）。
- 37: 同じファイルの頭の見出し「本モジュールが担わないもの（後続タスクの担当）」を「G8 PASS により足さないと決めたもの」に直し、完了 spec `areka-P0-ghost-window-zorder` の判定表が G1〜G8 全 PASS で tasks.md 5.2〜5.7 は実施しないことを書いた。切離しを早める強化は「G8 で PASS と判定済み（tasks.md 5.2 は不実施）」、重なりの変化の検知は「z の変化を知らせる仕組みは無い（`ReassertZOrder` の説明＝10.2 で直した `zorder_pair.rs`）」に。切離しの関数の説明の「G8 の判定対象である」も「G8 で PASS と判定済み（tasks.md 5.2 は不実施）」に直した。11.2 が直した頭の記録の段落は触っていない。
- 38: 「2 本」と書いていた門の説明は 9.2 で `zorder_pair_test_support.rs` へ移り、本数を言わない形になっていた。常に手前のテストのファイルの頭に「明示実行へ隔離した 3 本」の節を足してテストの名前 3 つを並べた。`pair_fix_commands_keep_a_pair_inside_the_band_it_already_shares` の `#[ignore]` の理由の文を「帯から外す指令が効かない回がある（AREKA_WINTF_REAL_WINDOW_ZORDER を与えて明示実行する）」に替えた（環境変数の案内は兄弟の 2 本と同じく残した）。属性と本体 1 行目の門の呼び出しは 3 本とも変えていない。
- 40: 挿入位置に帯の窓を渡す対照の「すぐ手前」を「すぐ背後」に直した。research.md の 5 か所は今の行で 3 か所（テストの説明の対照②・対照②の失敗文・対照の行のコメント）。残りの 2 か所（元の `:459`・`:622`）は「すぐ手前」の語を含んでいなかった。ほかの「すぐ手前」はバルーンがキャラの上に来る話で正しく、触っていない。
- 較正: 説明だけの直しで、足したテスト・直したテストは無い。
- 消したもの: なし（git grep 不要）。古い字面が他に残っていないことを `git grep "spawn.rs:72\|apply_zorder_pair_maintenance).chain\|後続タスクの担当\|G8 の判定対象\|2 本のみ"`（crates）で確かめた。当たるのは本物のコード 2 行（`examples/tooltip_demo.rs:223`・`zorder_pair_test_support.rs:298` のスケジュールの組み立て）だけ。
- テストの数: `cargo test -p wintf --lib zorder_pair` は 96 緑・3 ignored（10.2 の後と同じ）。`-- --list --ignored` は A-1 の 3 本だけ。行数: `zorder_pair_maintain.rs` 563→565・常に手前 774→783・`zorder_pair_maintain_tests.rs` は触っていない（805）。build 通る・fmt 通る。
- 40 と同じ向きの誤り（取りまとめの指示で足した）: 常に手前のテストの腕①のコメント「バルーンをキャラのすぐ手前へ」は、組む指令が `InsertSpec::After(character)` なのでバルーンをキャラのすぐ背後へ置く（`InsertSpec::After` は `ZOrder::InsertAfter`＝指した窓のすぐ背後・`zorder_pair.rs` の `InsertSpec` の説明）。本番の腕①（`PlaceAboveOverBelow`）はキャラの最も近い可視の手前の窓を渡して結果としてすぐ手前に入る。コメントを「腕①の形の指令・挿入位置にキャラそのものを渡すのでバルーンはキャラのすぐ背後へ入る・見るのは帯の印だけ」に直した（コメントだけ・1 行増え 784 行）。

## 12.2

- 39: `zorder_pair_maintain_detach_tests.rs` の `establish_pair` で、外すのを `remove::<(ReassertZOrder, IssuedPairFix)>()` にし、外した直後に `flush_window_pos_commands();` を 1 行足した。説明は「確立系と維持系は chain でつながるので、同じ巡の維持系が要求をもう使い、非表示の窓どうしは隣と測れないので是正の指令が積まれ `IssuedPairFix` も付く。要求と印を外し、積まれた指令も捕捉窓の外で流し切る」に直した。確かめの形として、`character_first_despawn_detaches_the_owner_and_spares_the_balloon` の捕捉を `CAPTURE_DIRECTIVES` にし、捕捉の中に対照の 1 本（`issue_control_command`）を置いて `SET_WINDOW_POS_TAG` がちょうど 1 本、の主張を足した（survivor と同じ形）。
- 較正: `establish_pair` の流しの 1 行をコメントにすると `character_first_despawn_detaches_the_owner_and_spares_the_balloon` が赤（`SET_WINDOW_POS_TAG` が left: 2 / right: 1）。戻して緑（detach_tests 7 緑）。戻した後の `git diff --stat` は戻す前と同じ（24 行足し・9 行消し）。
- 58: `zorder_pair_maintain_tests.rs` の `measure_window_below_maps_failure_to_absence_and_records_it`（主張は「ログが空でない」だけ）と、その 1 本だけの節の見出し「検証で使う実測（記録の付随フィールド）」を消し、使われなくなった `measure_window_below` の use を外した。同じことは `zorder_pair_measure_tests.rs` の `measurement_falls_back_to_absence_when_the_scan_hits_its_limit` が、同じ捕捉の中の `measure_window_below(HWND::default())` が `None` を返すことと `FAILURE_MARK` がちょうど 1 本であることで厳しく押さえている（research.md が書く `:325-336` は今の行では `:203-215`）。緑を確かめた。
- 消す前の git grep: `git grep "measure_window_below_maps_failure_to_absence_and_records_it"` は消したテストの定義 1 か所と research.md の指摘、completed の spec の検証記録（テスト名の一覧・実行の記録）だけ。コード・fixture・設定からの参照は無い。
- テストの数: `cargo test -p wintf --lib zorder_pair_maintain` は 33→32 緑・3 ignored（1 本消した分）。行数: `zorder_pair_maintain_tests.rs` 805→784・`zorder_pair_maintain_detach_tests.rs` 546→561。build・fmt 通る・clippy に足した警告は無い。

## 13.2

- 直したこと（6 番）: `window_proc/mouse_move.rs` に `track_mouse_leave(hwnd)` を置き、`WM_MOUSEMOVE` の頭（World の借用の外）で毎回呼ぶ。成功は `trace!`「TrackMouseEvent(TME_LEAVE) set」、失敗は `debug!`「TrackMouseEvent(TME_LEAVE) failed」を 1 行ずつ。理由の 1 行（返す相手が無い・次の移動でまた呼ぶ・`warn!` だと溢れる・`cursor/answer.rs` と同じ）を本番の説明に書いた。旗の読み書きと `WM_MOUSELEAVE` の旗の戻しを消し、`WM_MOUSEMOVE` の説明の「初回移動時に」を「移動のたびに」に直した。
- 消したもの: 型 `WindowPointerTracking`（`ecs/pointer/types/mod.rs`）・そのテスト `test_window_pointer_tracking_default`（`ecs/pointer/types/tests.rs`）・再公開（`ecs/mod.rs`）。
- 消す前の `git grep "WindowPointerTracking"`: crates の中は `ecs/mod.rs:35`・`pointer/types/mod.rs:198,206`・`pointer/types/tests.rs:122,125`・`window_proc/mouse_move.rs:131,159,171,172,180,493,523,525` だけ（areka・areka-placement・examples・`crates/*/tests`・fixture・設定は 0）。ほかは `.kiro/specs/` の文書（この spec と completed の記録）だけ。消した後は crates の中で 0 件。
- 新しいテスト: `window_proc/mouse_move_dispatch_tests.rs`（`mouse_move.rs` から `#[cfg(test)] #[path] mod dispatch_tests;`）の `every_mouse_move_arms_the_leave_watch`。道具窓は `window_pos_transition_tests.rs` の `DISPATCH_TARGET`・`create_test_window` を借りる（写さない）。借りるために `window_pos.rs` の `mod window_pos_transition_tests` を `pub(super)` に、2 項目を `pub(in crate::ecs::window_proc)` に広げた（テストの時だけの可視性で、本番は変わらない）。
- 赤（直す前）: 今のコードのままで `left: 1, right: 2`（旗が立った 2 度目は掛けない）。
- 較正: `track_mouse_leave` の頭に thread_local の「1 度だけ」の門を入れる → `every_mouse_move_arms_the_leave_watch` が赤（`left: 1, right: 2`）。門を外して緑に戻した。
- テストの数: `window_proc::mouse_move` 8→9（足した 1 本）・`pointer::types` 22→21（消した 1 本）。

## 13.3

- 45: `WM_MOUSEMOVE` の当たった枝と当たらない枝を `let target_entity = hit_entity.unwrap_or(window_entity);` の 1 本にまとめた。`get_entity_mut` の失敗はどちらでも今の当たった枝の `warn!`「[WM_MOUSEMOVE] Failed to get entity_mut」。挿入の `debug!`「PointerState inserted (Enter)」に `hit`（当たったか）の欄を足し、当たらない枝だけの字面「PointerState inserted on Window (no hit)」「PointerState moved to Window, Leave marker inserted」は無くなった。`PointerState` の構築（`local_point` の欄を含む）は中身を変えず、`WM_MOUSEMOVE` の中の 1 か所になった（申し合わせ ⒞）。
- 離脱の付け外しを `leave_pointer(world, window_entity, keep: Option<Entity>)` に出した（`debug!`「PointerState moved, Leave marker inserted」に `old_entity`・`new_entity`＝keep・`window`）。`WM_MOUSEMOVE` は `Some(target_entity)`、`WM_MOUSELEAVE` は `None` で呼ぶ。`WM_MOUSELEAVE` の自前の集め直しは消え、字面「PointerLeave marker inserted (scoped to window)」も無くなった（使っていた `hwnd` は `_hwnd` に）。
- `collect_entities_to_leave` の `exclude` を `Option<Entity>` にし、既存のテスト 4 本の引数を `Some(..)` に直した（主張は変えない）。
- 消した字面の git grep: `git grep "PointerState inserted on Window\|PointerState moved to Window\|PointerLeave marker inserted"` は `mouse_move.rs` の本番の行だけ（テスト・fixture・設定・areka 側からの参照は 0）。完了 spec の文書・記録に出てくるのは残す字面（「PointerState moved, Leave marker inserted」「PointerState inserted (Enter)」）だけ。
- 48: `mouse_move_dispatch_tests.rs` に `mouse_leave_leaves_only_the_window_it_reached` を足した。窓 A・B（`Window` 付きの素の entity）に `PointerState` の子を 1 つずつ置き、`dispatch_window_message` で A へ `WM_MOUSELEAVE` → A の子は `PointerState` 無し・`PointerLeave` 付き、B の子はそのまま。続けて B へ配ると B の子も離脱になる。窓は作らない（受け手は Win32 を呼ばない）。
- 較正: `collect_entities_to_leave` の窓の絞り込み（`find_owner_window(world, e) == Some(window_entity)`）を外す → `mouse_leave_leaves_only_the_window_it_reached` と `test_collect_protects_other_windows_pointer_state` が赤。戻して緑。戻した後の行は元の形（`git diff` で確かめた）。
- テストの数: `cargo test -p wintf --lib window_proc::mouse_move` 9→10 緑。行数: `mouse_move.rs` 612→530・`mouse_move_dispatch_tests.rs` 56→111。build・fmt 通る・clippy に足した警告は無い（`drop(entity_ref)` の `drop_non_drop` は元の 2 か所から 1 か所に減っただけ）。

## 13.4

- 44: `window_proc/mouse_move.rs` の借用の外の `guarded_set_window_pos` の `let _ =` を `if let Err(e)` にし、`error!(event = "drag_move_window_failed", hwnd, x, y, error)` で残す（`tracing` の取り込みに `error` を足した）。
- 44 のテスト: `mouse_move_dispatch_tests.rs` に `failed_drag_window_move_is_logged_as_error` を足した。`start_preparing`→`start_dragging`→`update_dragging`（窓の文脈は `move_window: true`・取っ手なし）で `Dragging`（取っ手が空）にし、`dispatch_window_message` で `WM_MOUSEMOVE` を配ると、`mouse_move=error` の絞りで `ERROR` かつ `drag_move_window_failed` の行がちょうど 1。
- 較正: 本番が `let _ =` のまま（直す前）にテストを回し、`failed_drag_window_move_is_logged_as_error` が赤（`left == right` 失敗・0 本）。直した後は緑。
- 62 ⒜: `Preparing` の腕の手書きの判定（`dx`・`dy`・`distance_sq`・`threshold_sq`・`>=`）5 行を消し、`crate::ecs::drag::check_threshold(current_pos, drag_config.threshold)` の呼び出しに替えた。中身は同じ（二乗の距離の `>=`・始めの点はスレッドの持ち物の `Preparing` の `start_pos`＝直前の写しと同じ値）。腕の `start_pos` の束縛は `DragTransition::Started` の組み立てでまだ使うので残した。`ecs/drag/` は触っていない。
- 消す前の `git grep`: 消したのはローカルの束縛だけ（`dx`・`dy`・`distance_sq`・`threshold_sq`）で、外から引く名は無い。
- 古くなった説明: `ecs/drag/state/mod.rs:618-621` の `check_threshold` の説明（「本番呼び出しがゼロ」「本番の閾値判定は `mouse_move.rs` 内にインライン複製」）はこの直しで事実と合わなくなったが、`ecs/drag/` はあちら（`audit-wintf-input`）の持ち物で、申し合わせ（12.2 門 GI ⒝）で説明の直しはあちらが持つので触っていない。
- テストの数: `cargo test -p wintf --lib window_proc::mouse_move` 10→11 本緑。`window_proc` 全体 116 本緑。`drag::state` 32 本緑（`check_threshold` の 5 本を含む）。

## 14.1

- 何を変えたか（46・61）:
  - `window_proc/mouse_click.rs`: `handle_button_message` に `double_click: bool` を足し `pub(super)` にした（引数が 8 個になるので `#[allow(clippy::too_many_arguments)]`・クレートに前例あり）。true のとき、付ける `PointerState` と既にある `PointerState` の `double_click` に `double_click_of(button)` を入れる。`double_click_of` を同じファイルに置いた。ふつうの押下・離しの 8 か所の呼び出しは `false`。
  - 61: 付ける `PointerState` の `left_down`・`right_down`・`middle_down` の初めの値を消して `..Default::default()` に任せた。`client_point`・`local_point` の欄は変えていない（申し合わせ ⒞）。
  - `window_proc/mouse_dblclick_wheel.rs`: `handle_double_click_message`（約 120 行）を消し、4 つの `WM_*BUTTONDBLCLK` は `handle_button_message(.., is_down = true, double_click = true)` を呼ぶだけにした。使われなくなった `use tracing::debug` を外した。これで当たらないときの予備の枝（窓の entity へ `record_button_down`）がダブルクリックにも効く。
  - 振る舞いの差（テストの外）: 消した受け手は既にある `PointerState` の `shift_down`/`ctrl_down` も直に書いていた。新しい道は直に書かず、ふつうの押下と同じく `set_modifier_state` の溜め（次の tick の転送）に任せる。ログの `[handle_double_click_message] Double-click detected` の行は無くなり、ふつうの押下の `[handle_button_message] hit_test result` の行が出る（`git grep` で crates・tools・doc にこの文字列を読む所は 0 件。完了 spec の記録の中にだけ残る）。
  - 文書: `doc/COMPAT_ARCHITECTURE.md:368` の定義点を `mouse_click.rs` の `handle_button_message`（`WM_LBUTTONDBLCLK` が印つきで呼ぶ）に直した。`mouse_click.rs` のファイル頭と共通処理の説明に、ダブルクリックもここを通ることを足した。
- テストの試し方: 既にあるダブルクリックのテスト 4 本（W1×2・W2×2）は `dispatch_window_message` で配っているので、試し方・主張とも変える所が無く、新しい道で緑のまま。
- 足したテスト: `mouse_dblclick_wheel_tests.rs` の `w3_left_dblclick_without_hit_records_press_on_window`（素の `EcsWorld`・窓に `PointerState` を付け、`WM_LBUTTONDBLCLK` を配って `transfer_buffers_to_world` の後に `left_down` が true）。
  - RED: 直す前に回して赤（`cargo test -p wintf --lib mouse_dblclick_wheel`: 4 passed; 1 failed・`w3_left_dblclick_without_hit_records_press_on_window`）。
  - 較正: 予備の枝の `record_button_down(window_entity, button)` を外す → `w3_left_dblclick_without_hit_records_press_on_window` が赤（116 passed; 1 failed）。明示の編集で戻し、`git diff` が意図した差分だけに戻ったことを確かめた。
- 消す前の `git grep`（`handle_double_click_message`）: crates の中は `mouse_dblclick_wheel.rs` の定義と 4 つの呼び出しだけ。tools・テスト・fixture・設定は 0 件。`doc/COMPAT_ARCHITECTURE.md:368` に 1 件（上で直した）。ほかは `.kiro/specs/completed/` の過去の記録（直さない）と、この spec の research.md・design.md。`DOUBLE_CLICK_BUFFERS` は触っていない。
- テストの数: `cargo test -p wintf --lib window_proc` 116 → 117 passed（足した 1 本）。
