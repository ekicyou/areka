# Brief: session-crate-split

> 2026-10-28 `/kiro-discovery` で起票。開発者「この際、areka crateのコードベースが大きすぎる問題をサルベージすべきかもしれない」→ 測った後に 6 本で「一括承認」。6 本のうちの 6 本目（最後）。分け方と共通の約束は roadmap の「テーマ別の決めごと」の「本体の crate 分け」が正本。数は main `d6054e4d` の実測（非空行）＝着手のときに測り直す。

## Problem

`session-module-gather` で、起動・終了・切替の層は本体の `src/session/` に集まる。それでも、集めた層は本体だけのモジュール（input・menu・install・update・mcp・cli）を呼んでいる。たとえば窓を開いた後に入力やメニューの受け手を差し込む所や、切替の節目に install と update の机へ知らせる所がそうで、crate に出せない。この spec で呼ぶ向きを逆にし、`areka-session` へ移す。これで本体は「UI スレッドからエンジンへつなぐ」部分だけになる。

## Current State

- **集めた層の大きさ**（見込み）: 本番 約 8,600 行・テスト 約 25,000 行。
- **外への参照を 0 にする手当て**（全件測った・どうしても切れないもの 0 件）。
  - **項目を移すだけで済む約 40 件**と**型を下ろす 2 種**は、`session-module-gather` で済む。
  - **向きを逆にする約 30 件**（仕組み 6 つ）がこの spec の中心になる。

  | 仕組み | 件数 | 中身 |
  |---|---|---|
  | C1 窓を開いた直後の差し込み | 5 | input 4（`attach_char_pointer_handlers`・`attach_balloon_pointer_handlers`・`attach_file_drop_receivers`・`attach_shortcut_receivers`）・menu 1（`attach_release_handlers`） |
  | C2 実行系が立った後の結線 | 13 | menu 9（`wire_menu` と各枠の `register` 8 本）・input 3（`wire_mouse_input`・`wire_choice_drain` ほか）・`install::names::reseed` 1 |
  | C3 切替の節目の知らせ | 5 | `ghost_switch` から install・update の机（`on_steady`・`run_overwrite_between`・`on_switch_end`） |
  | C4 机が忙しいかの問い | 3 | `reload`・`shell_balloon_switch` から `install::desk::is_busy`・`update::desk::is_busy` |
  | C5 受け口 | 3〜5 | `runtime_boot` が作る install・update の `raw_sender` と `InstallCueSink`・`UpdateCueSink` |
  | C6 OS が窓を閉じる求め | 1 | `app_exit.rs` の `MouseWiring` |

  - C1・C2・C5 は、本体が渡す trait 1 つ（メソッド 3 つ）にまとめられる見込み（推測）。
  - C6 は、`GhostSlot` の実行系の `kanade()` から送れば消せる見込み（推測）。
- **テスト**: 集めた層のテストのうち 35 ファイルが、本体だけのモジュールを参照している（roots の残り 47 件・menu 11・cli 10・input 7・install 7・update 7）。
- **外から集めた層への参照**（本体から session への向き・許される）: main.rs 約 68・install 15・update 11・menu 11・mcp 10・roots 7・readme（集めた後は中）・offline_dump 4・input 2・cli 1。
- **`multi-ghost-runtime`**: brief が作り替える中心（`GhostSlot` を識別子から `GhostSession` への集まりにする・止まる流れを「このゴーストが止まった」と「アプリを終える」に分ける・App の範囲の記憶を 1 か所に集める）は、ほぼ全部この層の中にある。

## Desired Outcome

- `crates/areka-session/`（lib・`publish = false`）が集めた層の本番とテストを持つ。`areka-input`・`areka-runtime`・`areka-placement` に依存し、本体からは依存される。
- `areka-session` は本体を参照しない。本体だけのモジュールへの呼び出しは、本体が渡す口（trait や知らせ）を通る。
- 本体（`crates/areka`）の本番は約 15,000 行になる。残るのは main と起動オプションのほかは、MCP・メニュー・インストール・更新・投げ込み・オフラインの面の書き出し・SHIORI のデモだけで、どれも UI スレッドからエンジンへつなぐ部分。
- 振る舞いは何も変わらない（「検証」）。

## Approach

- 向きの逆転を先に本体の中で済ませ、`session/` の外への参照が 0 になったのを grep で示してから、crate に移す（`runtime-module-untangle` → `runtime-crate-split` と同じ順を、1 本の中で行う）。
- 仕組みは要件の段で決める。推奨は次のとおり。
  - C1・C2・C5 は本体が実装する 1 つの trait（窓を開いた直後・実行系が立った後・受け口を作る）。
  - C3 は切替の節目の知らせ。install と update が受け手になる。
  - C4 は install と update が立てる「忙しい」の資源を session が読む形。
  - C6 は kanade へ直に送る形。
- **呼ぶ順は変えない**。登録・差し込み・結線の順は、今の順と同じであることをテストで示す。
- 本体だけのモジュールを参照するテスト 35 ファイルは、2 つに分ける。逆転した口の偽物で session の中で確かめるテストと、本体に残して本物どうしをつなぐテストにする。
- **crate の名前**: `areka-session`。`session_end.rs` は OS のログオンのセッションの終わりの受け手で、ゴーストのセッションとは別の意味。紛らわしければ要件の段で `os_session_end` などへ改める。

## Scope

- **In**: 向きの逆転 6 つ・crate の新設と移動・公開面・テストの分け方と移し替え・`test-support` 機能・ログの出所（`areka::session::…` を `areka_session::…` へ。real-run-kit の名指しの決まり・`log_history` の決まり・`RUST_LOG` の文字列・手順書）・文書のパス・`structure.md`・`audit-areka-app`・`audit-areka-install-update` の brief の「受け持つファイル」の書き換え。
- **Out**: ゴーストの単位を複数にすること（`multi-ghost-runtime`）・menu・install・update・mcp・roots の残りの crate 化（どれも本体がつなぐ役そのもの＝本体に残す）・中身の直し（audit へ申し送る）。

## Boundary Candidates

- 向きの逆転（仕組みごと）。
- crate の骨組みと移動・公開面。
- テストの分け方。
- ログと文書。
- 検証。

## Out of Boundary

- 振る舞いの変更。起動の段取り・切替の段取り・終わり方の順は変えない。

## Upstream / Downstream

- **Upstream**: `session-module-gather`。
- **Downstream**: `audit-areka-app`・`audit-areka-install-update` はこの spec の着地を待つ。`multi-ghost-runtime` は、v0.1.0 の余力の枠で着地していなければこの spec の後に、新しい crate の形で測り直して取る。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**:
  - `multi-ghost-runtime`: ファイルがほぼ全部重なる。同じウェーブに置かない。
  - `shiori4-api`（v0.2.0・`ghost_session` を触る）・`resident-ghost-rotation`（v0.4.0）・`tasktray-icon`（windows-reactor 待ち）: roadmap の新旧のパスの対応表で読み替える。

## Constraints

- **検証（要件に入れる）**: roadmap の「本体の crate 分け」の「共通の約束」の検証の項を、そのまま要件に入れる。この spec は向きを逆にするので、実機で次の道を 1 回ずつ回し、記録の順が前後で同じことを示す。
  - 起動
  - メニューからのゴーストの切替
  - MCP の `reload`
  - メニューの終了
  - 煙の自動終了（`AREKA_APP_SMOKE_EXIT_MS`）
  - 窓への `.nar` の投げ込み

  OS のセッションの終わり（`WM_ENDSESSION`）は、今の決定論テストで足りる（実機では起こせない）。
- **フレームを遅らせない**: 逆転した知らせや口が、切替・終了・差し込みを 1 フレームでも遅らせない。
- **共通の約束**: roadmap の同じ節のとおり。
- **Fable**: Opus で足りる（口は本体と session の間だけの内向きのもの）。

## 2026-11-22 棚卸㊷の再測定（main `d4db9eb0`）
- **前提の変化**: `session-module-gather`（PR#494）が着地し、起動・終了・切替の層は本体の `crates/areka/src/session/`（122 ファイル）に集まった。本体だけのモジュールの系の登録は、`main.rs` の `BODY_SYSTEMS`（`session::ghost_session::BodySystems` の関数ポインタの欄 2 つ＝`menu_poll`・`after_input`）を `register_systems` へ渡す形になった＝C1・C2・C5 の口は、この欄を足す形が一番近い前例（新しい trait は要らない見込み）。入力の 5 件は `areka-input` の `pub` を直に呼ぶので逆にしない・C6＝0（前の節のまま）。
- **逆にする件の数え直し**（向こうの research.md 14.3＝本番 28 行）: C1＝`ghost_session.rs:298・304・307`、C2＝`ghost_session_runtime.rs:34〜51`、C3＝`ghost_switch.rs:504・513・515・525・675`、C4＝`reload.rs:309・312`・`shell_balloon_switch.rs:131`、C5＝`runtime_boot.rs:24・26・89・90`。新しく 1 件＝`alert.rs:286` の `crate::smoke_exit::smoke_exit_ms()`（`smoke_exit.rs` 72 行は本体のまま・`install/startup.rs:235`・`main.rs:267` も使う）。
- **テストの側**: 本体だけの項目を引く session のテストは 19 ファイル。うち `crate::BODY_SYSTEMS` を渡す 5 か所（`app_exit_escape_tests.rs:140`・`ghost_session_reload_tests.rs:71・179`・`ghost_session_restart_tests.rs:455`・`ghost_switch_test_support.rs:237`）は、`SwitchRig` を通る 47 ファイルの支え。根の `after_run`・`finish_after_run`・`boot_first_ghost`・`install_boot_context` を引く 7 ファイル（`session_end_tests.rs`・`ghost_session_switch_fallback_tests.rs`・`ghost_session_switch_memory_tests.rs`・`readonly_run_tests.rs`・`readonly_sites_tests.rs`・`write_failure_sites_tests.rs`・`boot_config_module_policy_tests.rs`）は結線の守りとして本体に残す候補。テストの支えの置き場も要る＝`crate_source_test_support`（7 か所）・`sample_test_support::acquire_emo2`（4）・`runtime_tests::spine::SpineHarness`（2）。扱いの 3 つは向こうの research.md 14.4 の申し送りのとおり。
- **触るファイル**: `session/` の 122 ファイルを `crates/areka-session/`（新しい crate・`Cargo.toml`・`src/lib.rs`）へ移す。本体で `session::` を引く 117 ファイル（直下 9・`install/` 20・`mcp/` 20・`menu/` 19・`update/` 11・`runtime_tests/` 24・`roots/` 6・`cli/` 4・`offline_dump/` 4）と `tests/emo2_mcp_real_run.rs`・`tests/smoke_boot_loop_exit.rs`。`runtime_tests/zorder_wiring_tests.rs` は `include_str!` で `../session/runtime_boot.rs`・`../session/ghost_session.rs` を読む（33・68 行）。本体とルートの `Cargo.toml`・`Cargo.lock`。real-run-kit の `launch.rs:372`（`RELIED_TARGETS`）・`launch_tests.rs`・`close_tests.rs`・`bin/real-run-fake-areka.rs`。パスを名指すコメント＝`crates/areka-runtime/src/talk_clock.rs:229`・`user_break_cue.rs:79`・`placement_tests/transition_judge_offset_signoff_tests.rs:57`・`crates/areka-kanade/src/schedule/script_log.rs:16`・`crates/areka/tests/emo2_real_run.rs:58`。文書＝`doc/testing.md`・`doc/ssp-mcp/log-convention.md`・`doc/pasta-real-run.md`・§8 のパスの行・`structure.md`。前の節の見張り 4 本（`crate_path` で歩く）・`file_drop.rs:17` の `GhostSlot`・`app_exit.rs:139` の `install_input_hooks` はそのまま当たる。上限に近いのは `ghost_switch_tests.rs` 982・`update/worker_tests.rs` 967・`main_session_mark_tests.rs` 957・`ghost_switch.rs` 928・`ghost_session.rs` 897（`main.rs` は 623 に減った）。
- **規模**: 13〜16 タスク（前の 12〜14 から上の端を上げた＝テストの支えの置き場と `BODY_SYSTEMS` を通る 47 ファイルの扱いが、向こうの申し送りのとおりこの spec でいちばん大きい直し）。20 は越えない見込み。
- **先に要るもの**: 無い（`session-module-gather` は着地した）。1 本だけの席は `Cargo.lock`（クレート `areka-session` を足す）＝C25。
- **同じウェーブに置けない相手**: `audit-areka-app`・`audit-areka-install-update`・`audit-emo2-boot-switch`（待つ側）・`multi-ghost-runtime`・`doc-compat-architecture-rename`。`test-name-work-tags` とは本体の `runtime_tests/zorder_wiring_tests.rs` で重なる（向こうは 2 本の名を変える）。`dola-cue-test-builder` とは、向こうが本体の呼び手まで付け替えるときだけ `session/readme/readme_cue_tests.rs`・`session/runtime_boot_tests.rs`・`update/update_cue_tests.rs`・`runtime_tests/consumer_ledger_agreement_tests.rs` で重なる（runtime の `carrier_cue` の名を残せば 0）。
- **優先度の区分**: A（開発者「この際、areka crateのコードベースが大きすぎる問題をサルベージすべき」→ 一括承認）。
- **要件定義のモデル**: Opus。口は前例 `BodySystems`（`session/ghost_session.rs`）・`SessionHooks`（`crates/areka-runtime/src/session_hooks.rs`）・`InputHooks`（`crates/areka-input/src/hooks.rs`）に倣う内向きのもの。
- **分割の案**: 無し。
- **見つけた穴・古くなった記述**: Current State の「テスト 35 ファイル（roots の残り 47 件…）」は古い（roots の芯は session の中へ入った＝今は上の 19 ファイル）。C1〜C5 の行は前の節から動いた（上のとおり）。
