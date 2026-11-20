# ギャップ分析: input-crate-split

- 日付: 2026-11-20（作業の木 `claude/kiro-start-input-crate-split-16b18a`・main `9bf6e32e` の上）
- 入力: `requirements.md`（承認前の草案）・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md` の「本体の crate 分け」と C23 の行）・同じウェーブの兄弟 3 本の作業の木の `brief.md`／`requirements.md`（読むだけ）
- 測り方: Grep／Glob／Read だけ（ビルド・テストは回していない）。数は 2026-11-20 の作業の木の実測。

## 1. 分析の要約

- **前例どおりに移せる**: `runtime-crate-split`・`placement-crate-split` の形（lib・`publish = false`・`test-support` 機能・本体の dev-dependency・テストの実行ファイルの根に落ちの報告の初期化子を置く）がそのまま使える。input が使う runtime と placement の項目は、input が今も別の crate（本体）から使っている以上、すでに全部 `pub`（構造から確か）。
- **要件の草案に無い穴が 5 つ見つかった**: ⑴ 本番が `windows` クレートを使う（2 ファイル）＝要件 1.8 の一覧に無い ⑵ `cursor_answer.rs` が `include_bytes!("../../cursor/hand.cur")` で本体の `crates/areka/cursor/` を読む ⑶ `UserBreakWiring::flag_rx`・`flag_source_connected` が `#[cfg(test)]`＝同じウェーブで触らない `session/runtime_boot_tests.rs` が使う（要件 3.6 の場面が実在する） ⑷ 出所を名前で読む 4 つ目の所 `cursor_answer_tests.rs` の `TARGET` ⑸ 強制退避のテスト 3 本は本物の `quit_app` の結果（窓が消える・`AppExit` が立つ）を見ている。
- **公開にする項目は見込みより多い**: 測ると、モジュール 9・項目 26・メソッド 7（＋`test-support` で開けるメソッド 2）。brief の見込み「約 17 項目と 3 メソッド」は数え方の違い（再輸出・`new` を数えていない）による。
- **口の形の候補は 2 つ**（A: `areka-input` に口の束を新しく置く／B: `SessionHooks` に 1 つ足す）。どちらも `SessionHooks` の前例にそろう。据える所は本体の `register_systems`（窓を開く前・プロセスに 1 回）で、要件 4.3 を満たす。
- **同じウェーブの兄弟との重なり**: 要件が足したファイルは、兄弟の触るファイルと**行では重ならない**。注意は 3 か所（`hit_region.rs` の冒頭の説明が `hit-id-pass` の変えうる行の近く・`follow.rs` は `balloon-flip-ssp-review` の次のウェーブへ回る案だけで重なる・`frame/wiring.rs` 319 行は `audit-emo2-boot-frame` の要件 5.1 がパスを書かない文へ直す）。

## 2. 今の形（Current State）

### 2.1 置き場と大きさ

- `crates/areka/src/input_events/` は 50 ファイル・16,338 行（`wc -l`・空行を含む）。いちばん長いのは `input_events_tests.rs` 890・`balloon_pointer_handler_tests.rs` 878・`user_break_tests.rs` 859・`shell_box_handler_tests.rs` 725・`mod.rs` 612。1,000 行の番人の例外表に input のファイルは 0 件（要件 1.10 はそのまま守れる）。
- 移さない 2 本: `shortcut.rs` 92 行（＋`shortcut_tests.rs` 262）・`file_drop.rs` 350 行（＋`file_drop_tests.rs` 282・`file_drop_mime_tests.rs` 88・`file_drop_wiring_tests.rs` 490）。どちらも input のほかのファイルを使わない（`use super::…` はそれぞれ自分のテストからだけ）＝独立して動かせる。
- 子のモジュールは平らに並び、`#[path]` で親につながる（`char_watch.rs` → `cursor_answer.rs`、`hover_region.rs` → `cursor_pick.rs`、`balloon.rs` → `balloon_*`）。`#[path]` は宣言したファイルの置き場から解くので、同じ平らな並びのまま `src/` へ移せば解け方は変わらない。

### 2.2 本体への参照（本番）

- コード: `mod.rs:34` の `use crate::app_exit::{ExitOrigin, quit_app};` だけ（説明の文を除く）。
- 説明の中のリンク `[`quit_app`]`（`mod.rs` の `on_char_pointer_pressed` の説明）は、上の `use` を消すと解けなくなる（`cargo check` は警告しないが `cargo doc` の `broken_intra_doc_links` になる）＝要件 3.8 の当たり。`[`crate::…`]` の形のリンクは 0 件。
- `shortcut.rs` は `crate::menu::reset_balloon_pos_frame`・`crate::menu::reset_pos_frame` を、`file_drop.rs` は `crate::ghost_session::GhostSlot`・`crate::install::{self, judge::SEPARATOR, InstallOrder, InstallOrigin}` を引く。

### 2.3 外の crate の使い方（移す分の本番・出現数）

| crate | 本番 | テストだけ |
|---|---|---|
| tracing | 76 | |
| wintf | 31 | |
| areka-runtime | 27 | |
| bevy_ecs | 25 | |
| areka-emo-text | 16 | |
| areka-placement | 13 | |
| areka-kanade | 7 | |
| areka-sakura | 7 | |
| **windows** | **3**（`balloon_wheel.rs:87` の `SystemParametersInfoW`・`cursor_answer.rs:11`／`75` の `IDC_*`・`PCWSTR`） | 5 |
| areka-emo-compose | 2 | |
| areka-ghost・areka-emo-present | 1 ずつ | |
| areka-parsers・areka-sylphya・areka-seriko・areka-emo-atlas | 0 | 5・4・2・1 |
| log-capture-kit | 0 | 13 |
| areka-nar・temp-path-kit・sample-ghost-kit | 0 | 0（`file_drop` の分を除く） |

- brief の「windows は 0」は誤り。`windows` はワークスペースの表から受け（`Win32_UI_WindowsAndMessaging` はワークスペースの機能に入っている＝機能を足さずに済む）、`Cargo.lock` に新しい外のクレートは増えない。areka-runtime・areka-placement も同じ受け方をしている。
- 本番の依存は 12 本（tracing・wintf・bevy_ecs・windows・areka-runtime・areka-placement・areka-emo-text・areka-kanade・areka-sakura・areka-emo-compose・areka-ghost・areka-emo-present）。dev-dependency は log-capture-kit・areka-parsers・areka-sylphya・areka-seriko・areka-emo-atlas と、`test-support` つきの areka-runtime・areka-placement。

### 2.4 同梱のカーソルの絵（要件の草案に無い）

- `cursor_answer.rs:28-29` が `include_bytes!("../../cursor/hand.cur")`・`grip.cur` で `crates/areka/cursor/` を読む（`BUNDLED`）。移した先で同じ相対パスは解けない。
- `crates/areka/cursor/` を名指す所: `tools/cursor-art.ps1`（48 行で `crates/areka/cursor` を組む・自分の検査を持つ）・`crates/areka/cursor/README.md`・網羅台帳 `assets.toml` の `system:grip`・`system:hand`（7782・7799 行・`input_events::cursor_answer の BUNDLED`）・`doc/COMPAT_ARCHITECTURE.md` §8（431 行付近）・完了 spec（歴史・除く）。

### 2.5 外から input への参照（測った一覧）

本体の本番（`ghost_session.rs`・`ghost_session_runtime.rs`・`app_exit.rs`・`menu/{mod,trigger}.rs`・`session/runtime_boot.rs`）と本体のテスト（`runtime_tests/` の 8 本・`menu/*_tests.rs` 4 本・`app_exit_tests.rs`・`ghost_session_{restart,strict,switch}_tests.rs`・`shell_balloon_switch_session_{abort,lap}_tests.rs`・`session/runtime_boot_tests.rs`）が使うもの。複数行の `use crate::input_events::{…}` は 0 件（1 行の形だけ＝grep の取りこぼし無し）。

## 3. 要件ごとの当たり（Requirement-to-Asset Map）

印: **Missing**＝今は無い・作る／**Constraint**＝今の形が縛る／**Unknown**＝設計の段で測る・決める／（無印）＝前例のまま。

| 要件 | 今ある資産 | ギャップ |
|---|---|---|
| 1.1・1.5 crate の新設 | `areka-runtime`・`areka-placement` の `Cargo.toml`（`publish = false # 理由`・`version.workspace = true`・`[features] test-support`） | **Missing**: `crates/areka-input/Cargo.toml`・`src/lib.rs` |
| 1.2・1.3 履歴を残す移動・`mod.rs` → `lib.rs` | 前例 `placement-crate-split`（`mod.rs` → `lib.rs`） | 46 ファイルを `git mv`。`lib.rs` 先頭の説明「現状は `throttle` のみ」ほかの古い文は直さず申し送り（8.1） |
| 1.4 本体が path 依存で使う | 本体 `Cargo.toml` の `areka-runtime` の 2 行（本番＋`test-support` の dev） | 同じ形で 2 行。`main.rs:68` の `mod input_events;` を出し直しに替える |
| 1.6 根の `Cargo.toml` を変えない | `members = ["crates/*"]` | 無し（自動で拾われる） |
| 1.7・1.8 依存 | §2.3 | **Constraint**: 1.8 の一覧に `windows` が要る（本番 3 か所）。`areka-nar` は要らない（実測 0） |
| 1.9・5.4 番人の 3 crate | `with_default_guard_test.rs`・`sample_path_guard_test.rs`（例外表に input の行 0） | dev だけに `log-capture-kit`。temp-path-kit・sample-ghost-kit は移す分では使わない |
| 1.10 1,000 行 | 例外表に input 0 件 | 無し |
| 2.1 `shortcut.rs` → `menu/` | `menu/reset_pos_frame`・`reset_balloon_pos_frame`（同じ `menu` の子） | `menu/mod.rs` に `mod shortcut;`。`crate::menu::…` は `super::…` でも解ける |
| 2.2 `file_drop.rs` の置き場 | `log_history` の `RULES`（§5） | **Unknown**: 置き場（§6 の論点 2） |
| 2.3 呼び手 | `ghost_session.rs:332`・`335` | `input_events::file_drop::…`・`input_events::shortcut::…` の 2 行は出し直しで通らない＝書き換える（`ghost_session.rs` は C23 の行の触るファイル） |
| 2.5・7.5 `get_log` の分け方 | `log_history.rs` の `RULES`・`UNKEPT_TARGETS`（`areka::install::install_cue` を外す前例） | §5 |
| 3.1 公開面 | §4 | **Missing**: `pub` への付け替え（モジュール 9・項目 26・メソッド 7） |
| 3.3 自分を指すパス | `crate::input_events::` 20 か所（コード）＋ `pub(in crate::input_events)` 7 か所（`cursor_pick.rs`） | `pub(in crate::input_events)` は根が crate になるので `pub(crate)` と同じ意味になる＝書き方を決める |
| 3.4 出し直し | 無し（ワークスペースに `use <crate> as <名>;` の前例は無い） | **Unknown（軽い）**: 根の `use areka_input as input_events;` を `crate::input_events::…` で引けるか（2018 以降の名前解決で引ける見込み）。設計の初めに `cargo check` 1 回で確かめる |
| 3.5・3.6 触らないファイル | `runtime_tests/` 8 本・`session/runtime_boot*` | **Constraint**: `session/runtime_boot_tests.rs:25` が `UserBreakWiring::flag_rx`（`#[cfg(test)]`）を使う＝3.6 の場面が実在。ほかに `flag_source_connected`（`#[cfg(test)]`）を `ghost_session_{restart,switch}_tests.rs`・`shell_balloon_switch_session_lap_tests.rs`（触ってよいファイル）が使う。`#[cfg(any(test, feature = "test-support"))]` にして本体の dev で `test-support` を付ければ、どのファイルも変えずに通る |
| 3.7 非公開の型 | §4 の署名 | 測った範囲で、公開にする署名に出る input の型は `RegionSource`・`PendingDoubleClick`・`ChoiceSelection`（どれも公開の一覧に入る）だけ。ほかは runtime・emo-text・wintf の公開の型 |
| 3.8 説明のリンク | `[`quit_app`]` 1 か所 | 地の文へ |
| 4.1〜4.7 退避の向き | `areka_runtime::session_hooks::SessionHooks`（`Resource`・`Clone`・`Copy`・関数のポインタの束・`of()` が無ければ `error!(session_hooks_missing)`＋`NO_BODY`）・`session::hooks::install` は `register_systems` の先頭（`main.rs:261`・窓を開く前） | **Missing**: 口の型と据え付け。**Unknown**: 形（§6 の論点 1）と、据わっていないときの終わり方（4.6） |
| 4.5・8.7 記録の順 | `mouse_escape_close`（info・input）→ `log_stay_on_top_discard` → `app_exit`（info・`areka::app_exit`＝real-run-kit の `RELIED_TARGETS` の 1 つ） | 口を同期で呼べば順は変わらない。**Missing**: 実機で Ctrl+Shift+左ダブルクリックを入れる手段（§7） |
| 4.7 退避のテスト | `input_events_tests.rs` の 3 本（`escape_works_without_mouse_wiring`・`handler_ctrl_shift_left_double_click_despawns_all_ghost_windows_without_sending`・`only_escape_terminates_ghost_windows`）が、`GhostWindowMarker` の窓が消えることと `wintf::AppExit::is_requested()` を見る＝本物の `quit_app` の結果 | **Constraint**: 口が無い `areka-input` のテストでは、この 3 本は中身を変えないと通らない（5.1 は偽物への差し替えを禁じる）。ほかの 18 本は退避の枝に入らない（結線済み＋Shift 無し・Ctrl 無し）＝そのまま移せる。本体のほかのテストで退避の枝に入るものは 0 件（`ctrl_down: true` を立てる本体のテスト 0） |
| 5.1・5.2 テストの置き場 | `placement_tests/spawn_assembly_tests.rs`（608 行・`attach_char_pointer_handlers` と placement の `spawn::test_support` だけ） | `spawn_assembly_tests.rs` はそのまま `areka-input` へ移せる（本体を引かない）。退避の 3 本の扱いは §6 の論点 3 |
| 5.3 下の crate の支え | `spine::GpuPermit`（3 ファイル）・`placement::test_support::emily_shaped_scopes`（1 ファイル）・`spawn::test_support`（`spawn_assembly_tests`）・`balloon_test_support` の `RuntimeWiring::new`・`BootAssets` ほか | runtime・placement の `test-support` を dev で付けるだけ（本体と同じ 2 行） |
| 5.5 input の `#[cfg(test)]` の支え | `balloon_test_support`・`drag_test_support` を本体のテストは使っていない（実測 0） | 開ける要があるのはメソッド 2 つ（`flag_rx`・`flag_source_connected`）と、退避の 3 本を本体に残すなら `MouseWiring::with_clock`（`#[cfg(test)]`）と `input_events_tests.rs` の小道具 |
| 5.6 MTA と落ちの報告 | 本体 `runtime_tests/mod.rs` の 3 行の `static INSTALL`（`.CRT$XCU`）＋確かめのテスト 1 本（`runtime-crate-split` 設計 D6） | 同じ形を `areka-input` の `#[cfg(test)]` の根に置く。**Constraint**: 確かめのテストを置けば多重集合が 1 件増える（8.3 で説明） |
| 5.7 数え方と `test-all.ps1` | `test-all.ps1` は crate 名の一覧を持たない（共通の約束） | 無し |
| 6.1〜6.6 パスの書き換え | §8 の一覧 | 書き方の形が 5 通りある（`crates/areka/src/input_events/x.rs` 61・`input_events::x` 36・`input_events/x.rs` 11・素の `input_events` 10・`areka/src/input_events/x.rs` 2）。6.7 の grep が拾うのはパスの形だけで、モジュールの形（`input_events::cursor_answer` ほか・網羅台帳に 21）を拾わない |
| 6.7 0 件の確かめ | — | **Unknown**: モジュールの形・素の `input_events` を当たりに入れるか（§6 の論点 7） |
| 6.9 `frame/wiring.rs` | 295・319・326 行 | **Constraint**: `audit-emo2-boot-frame` の要件 5.1 が 319 行を「パスを書かず本体の入力の層を名前で指す文」に直す。295・326 行は同 spec の要件に無い |
| 7.1・7.2 出所 | モジュールのパス由来だけ（`target:` の直書き 0 件） | 無し |
| 7.3 名前で読む所 | `emo2_mcp_real_run.rs:115`（＋10・40 行の説明）・`window_link_real_run.rs:49`・`doc/pasta-real-run.md` 127・141・214・217 | **Missing**: 4 つ目＝`input_events/cursor_answer_tests.rs:27` の `const TARGET: &str = "areka::input_events::char_watch::cursor_answer";`（88・220 行で出所を照合）。7.6 の grep は拾うが 7.3 の一覧に無い |
| 7.4 `areka_input=debug` | `shortcut.rs`・`file_drop.rs` の記録は info と warn だけ（debug・trace 0） | 2 本が input の出所から外れても、手順書と実機テストの `RUST_LOG` の土台が `info`（`window_link` は `areka::menu=debug` も持つ）なので、見える行は減らない |
| 8.2・8.3 多重集合 | 前例の比べ方 | 置き場が変わるだけなら差は 0。足すテスト（§6 の論点 5）の分だけ差が出る |
| 8.9 `log_history` のテスト | `log_history_tests.rs` に input の出所の行は 0 | **Missing**: 前後で同じことを示す行（既にあるテストの表へ足すか・新しいテストか） |
| 8.10 `THIRD-PARTY-NOTICES.md` | `about.toml`（`ignore-dev-dependencies = true`）・`areka-runtime 0.1.0` の行が載る | `areka-input 0.1.0` の 1 行が増える見込み |

## 4. 公開にする項目（測った一覧）

本体の本番とテスト（移さない分）が使うものだけ。`spawn_assembly_tests.rs` は `areka-input` へ移すので数えない。

**モジュール（`pub(crate) mod` → `pub mod`）9**: `balloon`・`balloon_wheel`・`char_watch`・`char_watch::cursor_answer`・`choice_drain`・`click`・`shell_box`・`shell_box_handler`・`user_break`（`throttle` は外から使われない＝`pub(crate)` のまま）。

**根の項目 8**: `MouseWiring`・`PendingDoubleClick`・`RegionSource`・`wire_mouse_input`・`attach_char_pointer_handlers`・`char_scope`・`on_char_pointer_pressed`（本体のテストと `runtime_tests/frame_shell_box_arrow_tests.rs`）・（`on_char_pointer_moved` は外から使われない）。

**子の項目 18**: `click::on_char_pointer_released`／`user_break::{UserBreakWiring, register_user_break_drain, wire_user_break, drain_no_user_break_signals, on_left_press}`／`balloon::{BalloonWiring, ChoiceSelection, register_balloon_leave_system, attach_balloon_pointer_handlers, wire_balloon_choice}`／`shell_box::ShellBoxHover`（`pub(crate) use areka_runtime::…` の再輸出 → `pub use`）／`shell_box_handler::register_box_leave_system`／`choice_drain::{register_choice_drain, wire_choice_drain}`／`char_watch::{register_char_watch, char_watch_with}`／`char_watch::cursor_answer::register_char_cursor`／`balloon_wheel::register_balloon_wheel`。

**メソッド 7**: `MouseWiring::{new, send_close_request, defer_right_double_click, take_pending_right_double_click, send_pending_right_double_click}`・`UserBreakWiring::new`・`BalloonWiring::new`。

**`test-support` で開けるメソッド 2**（今は `#[cfg(test)]`）: `UserBreakWiring::{flag_rx, flag_source_connected}`。

- 合わせて 26 項目＋メソッド 7（＋2）。brief の「約 17 項目と 3 メソッド」との差は、再輸出（`ShellBoxHover`）・構築子（`new` 3 つ）・テストだけの使い手（`defer_right_double_click`・`on_char_pointer_pressed`）を数えるかの違い。
- 外から使われない `pub(crate)`（`throttle`・`ChoiceSelectionInbox`・`ChoiceForwarder`・`hover`／`set_hover`・`send_selection` ほか）は `pub(crate)` のまま残す（3.2）。`ChoiceSelectionInbox`・`ChoiceForwarder` の名は本体の説明の文に出るだけ。
- 公開した後も、`areka-input` の非テストのビルドで使われなくなる `pub(crate)` は出ない見込み（今の本体の警告 0 から。項目を `pub` にすれば `dead_code` は鳴らない）。`RegionSource::Mock` の `#[allow(dead_code)]` はそのまま（8.1）。

## 5. ログの出所と `get_log` の分け方

- 決まり（`log_history.rs:115`）の当て方: debug・trace は残さない → warn 以上は error → info は `UNKEPT_TARGETS`（`areka::install::install_cue`）を除いて `RULES` を上から当てる。`areka` は**完全一致**だけ。
- 今の input の info の行（`areka::input_events::…`）はどの行にも当たらない（残さない）。移した後の `areka_input::…` も、`areka` の完全一致・`areka_update` の前方一致のどちらにも当たらない＝同じ（要件 7.5 は書き換え無しで満たされる。示すのはテスト 8.9）。
- 置き直す 2 本の info の行（`file_drop.rs` 3 か所・`shortcut.rs` 1 か所）:

| 置き場 | 出所 | 当たる行 | 前と同じか |
|---|---|---|---|
| `crates/areka/src/file_drop.rs`（src 直下） | `areka::file_drop` | 無し | 同じ |
| `crates/areka/src/install/file_drop.rs` | `areka::install::file_drop` | `areka::install`（Update） | **変わる**（`UNKEPT_TARGETS` に足せば同じに保てる＝`install_cue` の前例） |
| `crates/areka/src/ghost_session/…` ほか | `areka::ghost_session::…` | `areka::ghost_session`（Status） | **変わる** |
| `crates/areka/src/menu/shortcut.rs` | `areka::menu::shortcut` | 無し | 同じ |

- `session-module-gather`（次の 1 本）は src 直下の `ghost_session*` ほかを `session/` へ集める。`file_drop.rs` を src 直下に置くと、その spec の「集める対象」に入るかの判断が後に残る（同 brief は `file_drop.rs` を対象に挙げている）。

## 6. 設計の段で決める論点と候補

### 論点 1: 強制退避の口の形（要件 4.1）

| 案 | 中身 | 良い所 | 気になる所 |
|---|---|---|---|
| **A** `areka-input` に口の束を置く | `#[derive(Resource, Clone, Copy)] pub struct InputHooks { pub escape: fn(&mut World) }`（名は仮）と、無ければ `error!` を出す `of()`。本体 `app_exit.rs` が `fn escape(world) { quit_app(world, ExitOrigin::Escape); }` を据え、`register_systems`（`session::hooks::install` の隣）から 1 回呼ぶ | 型が使い手の crate に在る（`SessionHooks` が使い手の runtime に在るのと同じ向き）。`areka-runtime` を変えない | 口の型が 1 つ増える（ただし仕組みは `SessionHooks` と同じ形） |
| **B** `SessionHooks` に 1 つ足す | `SessionHooks` に `escape: fn(&mut World)` を足し、`NO_BODY` に何もしない答え、`session/hooks.rs` に中身。`SessionHooks::of` を `pub` にする | 口の束が 1 つのまま | `areka-runtime/src/session_hooks.rs` を変える（`frame*` でないので約束の外だが、runtime の公開の面が変わる）。runtime の口に input の都合が入る。`of()` の `#[cfg(not(test))]` の記録の扱いが input の側で効かない |
| C `quit_app` と `ExitOrigin` を下へ降ろす | `app_exit` の一部を runtime か input へ | 呼び出しが直のまま | `ExitOrigin` は `ShioriFault`・`PersistFailure` ほかを持つ・`session/ghost_quit*`（触らない約束）が `quit_app` を使う＝範囲が広がる。要件 4.1「新しい仕組みを作らず」とも合わない。**推さない** |

- どの案でも呼ぶのは同じハンドラの中で同期（4.2 の「1 フレームも遅らせない」を満たす）。据えるのは `register_systems`＝`main.rs:261` で窓を開くより前（4.3）。`register_systems` を呼ばずに窓を組む本番の道は見つからない（呼び手は `main.rs` だけ・ほかはテスト）。
- 4.6 の「本番の組み立ての道で据わることを決定論のテストで示す」は、`register_systems` を呼ぶ既にあるテスト（`ghost_session_reload_tests.rs`・`ghost_session_restart_tests.rs`・`session/ghost_switch_test_support.rs`）の World で口の有無を見る形が取れる。

### 論点 2: `file_drop.rs` の置き場（要件 2.2・2.5・OPEN QUESTIONS 2）

- 候補: src 直下（`areka::file_drop`・分け方は同じ）／`install/` の下（分け方が変わる＝`UNKEPT_TARGETS` に 1 行足す手当てが要る）。§5 の表のとおり、手当て無しで 2.5 を満たすのは src 直下だけ。

### 論点 3: 強制退避のテスト 3 本の置き場（要件 4.7・5.1）

- 3 本は本物の `quit_app` の結果を見る。候補:
  - ⑴ 本体に残す（境をまたぐテスト）。テストの World に口を据える 1 行が要る（`runtime_tests` が `crate::session::hooks::install(&mut world)` を呼ぶ前例）。うち 2 本は `world_with_wiring`（`MouseWiring::with_clock`＝`#[cfg(test)]`）を使うので、`with_clock` を `test-support` で開けるか、小道具を本体の側に写すかが要る（写しは 5.3 の「写しを作らない」に当たる）。
  - ⑵ `areka-input` に置き、口に記録用の関数を据えて「口が `Escape` の意味で 1 回呼ばれた」を見る形に変える＋本体の側に「口の中身が `quit_app(Escape)` である」テストを足す。5.1 の「偽物に替えて中身を変えない」との関係を要件の討議で確かめる要がある。
- `input_events_tests.rs`（890 行・21 本）は関数の単位で割ることになる（前例 `runtime-crate-split` は関数の単位で割った）。

### 論点 4: 同梱のカーソルの絵（§2.4・要件に無い）

- 候補: ⑴ `crates/areka/cursor/` を `crates/areka-input/cursor/` へ履歴を残して移す（`include_bytes!` の相対パスは今と同じ形で解ける。`tools/cursor-art.ps1` 48 行・README・台帳 2 行・§8 の書き換えが要る。`tools/` に触る＝兄弟は `tools/` に触らない）／⑵ 置き場はそのまま、`include_bytes!` を `../../areka/cursor/…` にする（input の crate が本体の置き場のファイルを読む＝ファイルの上の向きが逆）。

### 論点 5: 多重集合に出る差（要件 8.2・8.3・5.6・8.9）

- 足す見込みのテスト: 落ちの報告の確かめ（`crash_filter_is_installed_before_the_tests_start`・前例は本体・runtime・placement・wintf に各 1 本）・`get_log` の分け方の確かめ（8.9）・口が据わる確かめ（4.6）。足せば差を対応表で説明する。既にあるテストの表へ足す形なら名は増えない。

### 論点 6: 出し直しを残す期間（OPEN QUESTIONS 1）

- `session-module-gather` の brief の Upstream は「`ghost_session.rs` の input の参照が `areka_input::` になってから集める」と書く。この spec が出し直しを残すと、その前提と食い違う＝受け手の brief へ申し送る要がある（書き換えるのはどちらの spec か）。

### 論点 7: 6.7 の grep の当たりの形

- 6.7 はパスの形（`crates/areka/src/input_events/`・`input_events/x.rs`）だけを当たりにする。モジュールの形（`input_events::throttle`・`input_events::cursor_answer` ほか・網羅台帳 `assets.toml` に 21・下の crate の説明に多数）と素の `input_events`（10）を当たりに入れるか。6.2 は下の crate について「パスと名前」を直すと書くが、文書（6.4）は「パス」だけ。

### 論点 8: `frame/wiring.rs` の古いパスの受け手（要件 6.9・OPEN QUESTIONS 7）

- `audit-emo2-boot-frame` の要件 5.1 は 319 行だけを直す。295 行（`input_events/mod.rs` の `resolve_hit_owned`）と 326 行（`input_events/balloon.rs` の `on_balloon_pointer_moved`）は同 spec の要件に無い。同じウェーブで動いているので、完了の申し送りを待たずに向こうのセッションへ「295・326 行も同じ扱いで」と相談できる。

### 論点 9: `pub(in crate::input_events)` の書き方（要件 3.3）

- `cursor_pick.rs` の 7 か所。根が crate になると `pub(in crate)`＝`pub(crate)`。字面を `pub(crate)` にするか、`pub(in crate)` のままにするか（意味は同じ・振る舞いは変わらない）。

## 7. 同じウェーブ（C23）の兄弟との重なり（調べた事実）

要件が roadmap の C23 の行の外に足したファイルを、兄弟 3 本の触るファイル（roadmap の C23 の行と、各作業の木の `brief.md`／`requirements.md`）と突き合わせた。

| この spec が足したファイル | `hit-id-pass` | `balloon-flip-ssp-review` | `audit-emo2-boot-frame` | 見立て |
|---|---|---|---|---|
| `doc/pasta-real-run.md` | 無し | 無し | ログの語を変えると裁定されたときだけ読み手を直す（同 7.6・条件つき） | 今は 0。向こうが語を変える裁定を出したら同じファイルの別の行 |
| `crates/areka/src/install/mod.rs`・`readme.rs` | 無し | 無し | 無し | 0 |
| `areka-placement` の `spawn.rs`・`follow/drag_follow.rs`・`follow_drag_end_gate_tests.rs`・`placement_char_clamp_tests.rs`・`examples/window-placement.rs` | 無し（約束 ⑶ で placement に触らない） | 無し（案 ⑵⑶ でも `balloon_limit.rs`・`follow/window_move.rs`・`balloon_limit_flip_tests.rs`） | 無し（同 7.3 で placement の全部に触らない） | 0 |
| `areka-placement` の `follow.rs` | 無し | 判定し直しの登録を外す案のときだけ（その案は約束 ⑵ で次のウェーブへ回る） | 無し | このウェーブでは 0 の見込み・こちらは説明の 1 行 |
| `areka-runtime` の `balloon_hover.rs`・`shell_box_hover.rs`・`examples/collision-probe.rs`・`collision-probe/setup.rs` | 無し | 無し | 無し（範囲は `frame*`） | 0 |
| `areka-runtime` の `hit_region.rs` | `hit_region_client` の形を変えるときだけ（唯一の呼び手） | 無し | 無し（同 7.3） | こちらは冒頭の説明の 8・18・20 行。向こうが変えるなら 14・36 行（`hit_region_client` のリンク）と本体が近い＝行が隣り合えば合わせの手間。OPEN QUESTIONS 5 の仮定どおり |
| `areka-emo-text/src/choice.rs` | 無し（emo-compose・emo-present・wintf） | 無し | 無し | 0 |
| `doc/COMPAT_ARCHITECTURE.md` | §8 の末尾への追記＋「形がまだ当たらない」行の書き直し | §8 の 438 行の行の書き直し | 無し（同 7.3 で §8 に触らない） | こちらの 20 行は全部 §8 の中（153・181・182・370〜375・395・399・415・419〜421・426〜428・431・432・447 行）。438 行と `collisionex` の行（277〜279）とは別の行で隣り合わない |
| 網羅台帳 `assets.toml` | `collisionex*`（7031 行付近）・`animation*.collision*`（6111 行付近）と `region` の行 | 無し | 無し | こちらは `don't need onmousemove`（2863）・`mouse*`（7193〜7270）・`system:*`（7731〜7885）の別の項目。`report/` は欄の文を写さない（今の `report/` に input の語 0）＝作り直しても差は出ない見込み |
| `sakura-script.toml`・`shiori.toml`・`doc/anchor-compat.md`・`doc/ideas/`（実測 6 ファイル） | 無し | 無し | 無し | 0 |
| `.kiro/steering/structure.md` | 無し | 無し | 確かめ方を引くだけ | 0。ただし `log-capture-kit` の使い手の数「21 crate」（443 行）に `areka-input` が加わる |
| `tools/load-flake.ps1` | 無し | 無し | 名を引くだけ | 0。`-Binary` は今 `areka-bin`・`wintf-lib`・`areka-placement-lib`・`areka-runtime-lib` の 4 つ（`ValidateSet` 72 行・切り替え 208 行・自分の検査の表 275 行の 3 か所） |
| `crates/areka/cursor/`・`tools/cursor-art.ps1`（論点 4 の案 ⑴ のとき） | 無し | 無し | 無し | 0 |

- 補足: `audit-emo2-boot-frame` の要件 7.3 は、この spec の持ち物（本体の `input_events/`・`main.rs`・`ghost_session*.rs`・`app_exit.rs`・`menu/`・`spawn_assembly_tests.rs`・`Cargo.toml`・`Cargo.lock`・`log-capture-kit/src/event.rs` とその試験）に触らないと書いている＝向こうからの重なりも 0。

## 8. 書き換えの数（参考値・設計の段で測り直す）

- `doc/COMPAT_ARCHITECTURE.md` 20 行／網羅台帳 `assets.toml` 16 行（モジュールの形 21 か所を含む）・`sakura-script.toml` 2・`shiori.toml` 15（計 33 行）／`doc/anchor-compat.md` 6／`doc/ideas/` 6 ファイル 14 行（要件の「7 ファイル」は実測 6）／`doc/pasta-real-run.md` 4／`structure.md` 2（281・288 行）＋使い手の数／`log-capture-kit` 4（`event.rs` 1・`event_tests.rs` 3）／`areka-placement` 12（`spawn.rs` 7 ほか）／`areka-runtime` 12（`frame/wiring.rs` の 3 を除けば 9）／`areka-emo-text` 1／本体の説明（`install/mod.rs` 1・`readme.rs` 1・`ghost_session.rs` ほかの説明の中の `input_events` の語）。

## 9. 実装の候補の組み方

- **Option A（本体の中を先に動かしてから crate へ移す・推す形に近い）**: 段 1 で `shortcut.rs`・`file_drop.rs` を本体の中で置き直し（`ghost_session.rs` の 2 行）、段 2 で退避の口を据えて `mod.rs` から `crate::app_exit` を消し（まだ本体の中で動く＝テストで確かめられる）、段 3 で crate を作って `git mv`・公開面・出し直し、段 4 でテスト・支え・出所・文書。brief の Boundary Candidates と同じ順。各段で全部が組める。
- **Option B（一度に移してから直す）**: 先に crate を作って全部を移し、組めない所を順に直す。差分は少し小さいが、途中の段で組めない時間が長く、退避の口の確かめが crate の分けと混ざる。
- **Option C（出し直しを使わず呼び手を全部 `areka_input::…` へ書き換える）**: C23 の約束 ⑴ で `runtime_tests/`・`session/runtime_boot*` に触れないので、このウェーブでは取れない（取るなら次のウェーブへ）。

## 10. 規模と危うさ

- **規模: M**（12〜15 タスクの見込みは妥当）。前例の手順が揃っていて、新しい仕組みは口 1 つだけ。テストの割り（`input_events_tests.rs`）と書き換えの数が手間の主。
- **危うさ: Medium**。理由は、`#[cfg(test)]` の項目が crate の境で消えること（§3 の 3.6・本体のテストが組めなくなる）・退避のテストの置き場・同梱の絵のパス・出し直しの名前解決の 4 つが「組んでみて初めて分かる」形で、どれも設計の初めの `cargo check` で露わにできる。振る舞いの変わる余地は退避の口の 1 か所だけ。

## 11. 設計の段へ持ち越す調べ（Research Needed）

1. 根の `use areka_input as input_events;` を本体の中から `crate::input_events::…` で引けるか（`cargo check` 1 回）。
2. 公開面の一覧（§4）を `cargo check --workspace --all-targets` で確かめ直す（取りこぼしは組めない誤りで出る）。
3. 実機の退避（8.7）の手段: `tools/mouse-input.ps1` はダブルクリックも修飾キーも持たない。結線済みの本番の起動では Ctrl+**Shift**+左ダブルクリックが要る（結線前の枝は起動の失敗が要る）。候補は ⑴ `mouse-input.ps1` に修飾キーつきのダブルクリックを足す（`tools/` に触る） ⑵ `window_link_real_run.rs` と同じく試験の中で `SendInput` を組む（`VK_CONTROL`・`VK_SHIFT` と 2 回の押し放し） ⑶ 手で操作して記録だけ取る。どれでも排他テストの机と開発者の許しが要る。
4. `log_history` の確かめ（8.9）を既にあるテストの表へ足せるか（`log_history_tests.rs` の形）。
5. 落ちの報告の初期化子を `areka-input` のテストの実行ファイルに置いたとき、`areka-runtime` の `crash_report_test_support` の `#[cfg(test)]` の `static` と二重に据わらないこと（前例 D6 の形なら `areka-runtime` 側は自分のテストでだけ立つ）。
