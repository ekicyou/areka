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
