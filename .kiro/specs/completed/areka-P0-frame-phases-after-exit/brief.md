# Brief: areka-P0-frame-phases-after-exit

> 2026-09-28 棚卸⑲で `.kiro/steering/roadmap.md` の「登記だけの行」（メニューの終了の直後に表示の適用が窓の無い所へ届き ERROR が 1 件残る・2026-09-27 登記）から格上げ。file:line は起票時値（main `10a8d724`）。行番号は目安で、正本は「何の定義行か」の方。

## Problem

メニューの「終了」で areka を終えると、ログに ERROR が 1 件と WARN が 7 件残ることがある。停止そのものは正常（記憶の書き出し・`session_mark_cleared`・終了コード 0）で、利用者の画面には何も起きない。しかし「正常に終えたのに ERROR が出る」ため、実機確認で ERROR の件数を判定に使うと濁り、本物の異常と見分けにくい。

実物（`C:\tmp\areka-signoff-gcnr\logs\manual2.out.log` の 1420〜1440 行・`areka-P0-ghost-change-name-resolution` の `signoff.md`「2026-09-27 追記」の manual2）:

- 12:26:20.472 SERIKO のループ（animation_id=1400）が発火し、.481 に 1 コマ目の `ShowSurface`（TargetId(0)・surface_id=1000）が普通に適用される
- .631 `ghost_quit cause=Quit` → 全窓を despawn → .635 `app_exit origin=KanadeStopped(Quit)` → `[AppExit] exit requested`
- .636 **同じフレームの続き**で、キューに残っていた次のコマの `ShowSurface` が適用され、`derive_scale: 窓 DPI を取得できない（DPI component 不在）` の `error!` 1 件＋「装着が未完了」の WARN 7 件（surface entity 44v0・text-layer-slot 43v0）
- .683 以降の後始末（loop ticker の Close・記憶の書き出し・印の消去）は正常

## Current State

根本原因（静的に確定）:

1. 終了相と毎フレームの相は**同じ `Update` の同じ巡**で続けて走る。`wire_kanade_stop`（`crates/areka/src/emo2_boot/mod.rs` の `add_systems(Update, ghost_quit_system.before(emo2_frame_system))`・719 行付近）。
2. 終了相 `run_ghost_quit_phase`（`crates/areka/src/emo2_boot/frame.rs` 198 行付近）→ `quit_as_today`（同 233 行付近）→ `quit_app`（`crates/areka/src/app_exit.rs` 86 行付近）→ 私有部品 `despawn_app_windows`（同 199 行付近）が全ゴースト窓を**その場で** despawn し、`AppExit::request_exit` を呼ぶ。窓の子である surface entity・text-layer-slot も連鎖で消える。
3. ところが毎フレームの結線状態 `Emo2Wiring`（presenter と表示指令の受信端）は World に残る。直後の `emo2_frame_system`（`frame.rs` 272 行付近）は終了が指示済みかを見ずに全相を回し、`run_drain_phase`（`crates/areka/src/emo2_boot/frame/drain_resnap.rs` 50 行付近）がキューに残った `ShowSurface` を `EmoPresenter::apply` → `apply_show`（`crates/areka-emo-present/src/presenter/show.rs` 48 行付近）へ渡す。
4. `apply_show` は消えた窓の `DPI` を読み（`world.get::<DPI>(window)`・show.rs 77 行付近）`None` を `derive_scale`（`crates/areka-emo-present/src/scale.rs` 187 行付近）へ渡す → `dpi_missing` の `error!`（同 197 行付近）。続く `mount.set_display`／`set_layout`／`AlphaMaskResource`／`set_visible` が消えた entity を叩いて WARN 7 件。

設計の意図は既にあった: `run_ghost_quit_phase` の doc は「毎フレームの相より前に走る。終了が決まったフレームで他の相を走らせても、これから閉じる窓のために描き直すだけだからである」と書く。**順序は決めたが、終了が決まったフレームで他の相を止める判定がどこにも無い**のが穴である。

ERROR の出方は時刻に依る: 終了の巡で受信端に表示の指令（SERIKO のループのコマ・会話の面の切替）が 1 件でも残っていれば出る。emo2 は animation_id=1400 のループを常に回すので、メニューの終了のたびに起こり得る。

兄弟の経路（呼び手を grep した結果・`quit_app` の呼び手 10 か所）:

| 経路 | 窓を消す口 | 同じ巡で `emo2_frame_system` が古い presenter を回すか |
|---|---|---|
| メニューの終了・別れの台詞のあと（`quit_as_today`） | `quit_app` | **回す**（今回の症状） |
| 切替の後に迎え入れたゴーストを終える（`ghost_switch.rs` `on_ghost_stopped` の `quit_app` 4 か所） | `quit_app` | **回す**（同じ穴） |
| 既定ゴーストへ戻せない致命（`ghost_switch.rs` の `fatal`） | `quit_app` | **回す**（同じ穴・もともと失敗の終了） |
| 強制退避（`input_events/mod.rs` の `ExitOrigin::Escape`）・smoke の自動終了（`main.rs`）・OS のセッションの終了（`session_end.rs`）・kanade 未結線の OS の閉鎖要求（`app_exit.rs` `on_ghost_os_close`） | `quit_app` | 回し得る（同じ穴。`Update` より前の段で閉じても、同じ巡の `Update` で相が回る） |
| **ゴースト切替**（`ghost_switch.rs` の `switch_to`・`switch_to_default`） | `close_windows_for_restart` | **回さない**。`take_down`（`GhostSession::shutdown` で SERIKO の loop ticker を先に止める）→ `close_windows_for_restart` → `boot_into` → `boot_ghost_strict` → `wire_emo2_boot` の手順 6（`world.insert_non_send(wiring)`・`emo2_boot/mod.rs` 639 行付近）が**同じ system 呼び出しの中で** `Emo2Wiring` を新品へ差し替える。古い presenter と古い受信端（残っていた指令ごと）は捨てられ、新しい presenter は target 0 件・`attached=false` で始まる（`run_drain_phase` は装着前に取り出さない） |

**α への効き方**: ゴースト切替の途中では起きない（上の表・静的）。裏付けとして同じ実行体の manual3（切替あり・自動終了）は ERROR 0。起きるのは **α の利用者の一周の最後＝メニューからの終了**（切替の後に終えた場合も同じ）。利用者から見える害は無いが、α の実機確認のログで「正常な終了なのに ERROR」が混ざる。

## Desired Outcome

1. 終了が指示された後（`AppExit::is_requested()` が真）の巡では、`emo2_frame_system` は相を 1 つも回さず、理由の分かる 1 行（`debug!`・下の Approach）だけを残して戻る。消えた窓への表示の適用・文字層の描画・窓寸の反映・`\![move]` の適用・重なりの鎖の公開はどれも起きない。
2. `Emo2Wiring` は World に残す（取り出さない・壊さない）。後始末（`main.rs` の終了統括）の順序は今日のまま。
3. 決定論テストで固定する: 終了指示済みの World で `emo2_frame_system` を回すと、キューの指令は取り出されず（受信端に残る）・ERROR 0 件・読み飛ばしの行 1 件。対照として終了未指示なら同じ指令が取り出され適用される（未登録 target への `Hide` の ERROR 1 件で観測）。
4. 実機（emo2・debug 版）でメニューから終えて ERROR 0 件、読み飛ばしの行が出ていること（ループの指令が残っていた回）。

## Approach

**止める位置は `emo2_frame_system` の先頭 1 か所**（`crates/areka/src/emo2_boot/frame.rs`）。`Emo2Wiring` を取り出す前に `world.get_non_send::<wintf::AppExit>().is_some_and(|e| e.is_requested())` を見て、真なら `debug!(event = "frame_phases_skipped_after_exit", ...)` を 1 行残して戻る。

- ここが根本である理由: 窓を消す口（`quit_app`）は出所が 7 種類あり、どれも「窓を消してから終了を指示する」（`quit_app` の不変条件・`AppExit` の型の doc の契約）。したがって「終了指示済み」は「ゴースト窓はもう無い」と同値で、判定を 1 つ置けば全出所・全相（drain・バルーン可視性・窓寸の反映・`\![move]`・重なりの鎖・文字層）が同時に塞がる。
- level は `debug!`: 終了処理の正常系の読み飛ばしで、前例（完了 `areka-P0-dpi-window-vanish` の「破棄済みの窓に対して警告以上を出さない」・`quit_as_today` の `ghost_quit_no_windows`・`despawn_app_windows` の `DESPAWNED_SKIP_TAG`）と揃える。行は読み飛ばした巡ごとに 1 行（終了指示の後に走る巡は通常 1 つ）。
- 採らない案:
  - **presenter（`apply_show`）で「窓 entity が無ければ読み飛ばす」**: 表示の適用しか塞がらず、他の相（文字層・バルーン可視性・窓寸の反映）は消えた窓を叩き続ける。emo-present の層が「アプリの終了」を知ることにもなる。
  - **`derive_scale` の `dpi None` を `error!` から下げる**: 完了 `areka-P0-emo-dpi-scaling` の design（Error Handling の表「窓 DPI 取得不能（component 不在）→ error!」・要件 1.4）は、生きた窓に DPI が無いことを本物の異常として `error!` にしている。症状の側を黙らせると本物の異常まで消える。
  - **終了相で `Emo2Wiring` を取り除く**: `quit_app` の他の出所（強制退避・smoke・OS のセッションの終了）が塞がらない。presenter（GPU 資源）の drop の時点も動く。

## Scope

- **In**: `emo2_frame_system` の先頭の終了指示の判定 1 つと `debug!` 1 行。`run_ghost_quit_phase` の doc の「他の相を走らせても……」の一文を、判定が `emo2_frame_system` 側にあることへ書き直す。兄弟のテストファイル 1 本（新規）。実機の確認 1 回（emo2・メニューの終了）。
- **Out**: `derive_scale` の level（据え置き）。presenter・emo-present の変更。`quit_app`／`close_windows_for_restart` の変更。ゴースト切替の経路（穴が無い）。

## Boundary Candidates

- 毎フレームの相の入口 `emo2_frame_system`（areka・`emo2_boot/frame.rs`）だけ。
- 判定に使う `wintf::AppExit::is_requested`（既存の公開 API・`crates/wintf/src/runtime/message_loop.rs`）は読むだけ。

## Out of Boundary

- `crates/areka-emo-present/**`（`scale.rs` の `error!`・`presenter/show.rs` の適用）: 変えない。
- `quit_app` が外さない資源（`ZOrderChainPlan` など）が終了の巡の確定段で消えた窓を読むかどうか: 今回のログに症状が無い。見つかれば別件で登記する。
- ゴースト切替の経路（`ghost_switch.rs`・`ghost_session.rs`・`app_exit.rs` の `close_windows_for_restart`）: 同じ system 呼び出しの中で結線ごと差し替わるので穴が無い（Current State の表）。

## Upstream / Downstream

- Upstream: 完了 `areka-P0-app-lifetime-separation`（`quit_app` の「窓を閉じてから終了を指示する」不変条件）・完了 `areka-P0-ghost-shell-balloon-switch`（終了相 `run_ghost_quit_phase` と `ghost_quit_system.before(emo2_frame_system)` の順序）・完了 `areka-P0-emo-dpi-scaling`（`derive_scale` の `error!` の意味）。いずれも前提として既に main に在る。
- Downstream: `areka-P0-alpha-release-signoff`（α の実機確認のログで「正常な終了の ERROR 0 件」を判定に使える）。

## Existing Spec Touchpoints

- 完了 `areka-P0-dpi-window-vanish`: 「終了処理でゴースト窓が破棄された後の処理は警告以上を出さない」（requirements の終了時ログの節）。本件は同じ方針を毎フレームの相へ広げる。
- 完了 `areka-P0-ghost-shell-balloon-switch`: `run_ghost_quit_phase` の doc の一文を書き直す（挙動は変えない）。
- 完了 `areka-P0-ghost-change-name-resolution` の `signoff.md`「2026-09-27 追記」: 症状の初出。
- `.kiro/steering/roadmap.md` の「登記だけの行」の該当行: 本 spec の完了時に消す。

## Constraints

**触るファイルの確定一覧**（行数は起票時の実測）:

| ファイル | 現行行数 | 変更 |
|---|---|---|
| `crates/areka/src/emo2_boot/frame.rs` | 509 | `emo2_frame_system` 先頭の判定＋`debug!`（約 10 行）・`run_ghost_quit_phase` の doc の一文・新テストの `#[cfg(test)] #[path = "frame_exit_gate_tests.rs"] mod exit_gate_tests;`（4 行）。`use wintf::AppExit` か完全パス |
| `crates/areka/src/emo2_boot/frame_exit_gate_tests.rs` | 新規（見込み 100〜150） | 決定論テスト（下） |
| `.kiro/specs/completed/areka-P0-frame-phases-after-exit/*` | — | spec 文書・実機の記録 |
| `.kiro/steering/roadmap.md` | — | 完了時に登記の行を消す（`/kiro-complete`） |

**触らないファイル**: `crates/areka-emo-present/src/scale.rs`（230）・`crates/areka-emo-present/src/presenter/show.rs`（426）・`crates/areka/src/app_exit.rs`（291）・`crates/areka/src/emo2_boot/ghost_switch.rs`（866）・`crates/areka/src/ghost_session.rs`（691）・`crates/areka/src/emo2_boot/mod.rs`（805）・`crates/areka/src/emo2_boot/frame/drain_resnap.rs`（481）・`crates/areka/src/emo2_boot/frame_test_support.rs`（900・上限 1,000 の目前＝助けの関数を足さない。`headless_wiring_with`・`capture_logs`・`count_level`・`zero_clock` は既存のまま使う）・`crates/wintf/**`・`crates/areka/src/main.rs`。

**テストの形**（`frame_drain_text_tests.rs` の `run_drain_phase_gates_on_attach_then_drains_all_in_fifo_order` と同型・GPU 不要）:
- `headless_wiring_with(rx, zero_clock())` に `attached = true` を立て、`PresentCommand::Hide { target: TargetId(0), reply: None }` を 1 件送る（未登録 target ゆえ適用されれば `apply(Hide): 未装着ターゲット` の ERROR 1 件＝適用の観測点）。`World::new()` に `wintf::AppExit::new()` と `Emo2Wiring` を挿す。
- 終了指示済み（`request_exit()`）: `capture_logs(|| emo2_frame_system(&mut world))` の ERROR 0 件・`frame_phases_skipped_after_exit` 1 件・`Emo2Wiring` が World に残り受信端に指令が 1 件残る。
- 対照（未指示）: 同じ組み立てで ERROR 1 件・読み飛ばしの行 0 件・受信端は空（判定が常に真の実装を赤にする）。
- 変異の確認: 判定を外すと 1 本目が赤になることを実装時に一度確かめる。
- テストは実装と同じディレクトリの兄弟ファイル・1 ファイル 1,000 行上限（`crates/log-capture-kit/tests/file_length_guard_test.rs`）。crate 単位のテストに加えて `-p log-capture-kit` も回す。

**実機の確認**: emo2・debug 版・`RUST_LOG` は `areka=debug` を含める（読み飛ばしの行が `debug!` のため）。起動 → SERIKO のループが回るのを待つ → メニューの「終了」。判定語: `level=ERROR`／` ERROR ` が 0 件、`event="app_exit" origin=KanadeStopped(Quit)` 1 件、`session_mark_cleared` 1 件、終了コード 0。`frame_phases_skipped_after_exit` は指令が残っていた回だけ出る（出なくても不合格ではない）ことを起動前に言っておく。

**規模**: XS。タスク見込み 2〜3（判定＋テスト 1・doc の一文とログ語の整理 1・実機の確認 1）。

**Fable の要否: −**。止める位置は静的に 1 か所へ決まり（採らない案 3 つの理由も実物で確定）、level は前例に揃えるだけで、答えで作業が変わる議題が無い。要件・設計・実装とも Opus で足りる。

**並走**: `areka-P0-ghost-install` の brief の触るファイル一覧（`ghost_session.rs`・`emo2_boot/mod.rs`・`consumer_ledger.rs`・`menu/mod.rs`・`input_events/mod.rs`・`alert.rs`・`placement/spawn.rs`・`areka-ghost/src/catalog.rs`・kanade `schedule/events.rs`＋`events_change_tests.rs`・wintf `window_proc/mod.rs`／`window/components.rs`・新規 `install.rs`／`install_cue.rs`／`menu/install_frame.rs`／`input_events/drop.rs`／`terms.rs`／wintf `window_proc/drop.rs` ほか）と本件の触るソースの重なりは **0 本**（本件は `emo2_boot/frame.rs` と新しい兄弟テストだけ）。文書は `.kiro/steering/roadmap.md` の別の行だけ。完全並走できる。
