# Design Document — areka-P0-ghost-shell-balloon-switch

> 本文の実測は 2026-09-26・本ブランチ（main `13b72893` と同じソース）のもの。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。要件 11 の裁定 1〜12 は確定済みで、本設計はそれを覆さない。研究の経緯と選ばなかった案は `research.md` §9〜§11 にある。
> **2026-09-27 追記（裁定 13・要件 12）**: 単独起動の失敗の記憶の書き換え（`record_halt`・`should_record_halt`・`take_last_halted`・鍵 `areka.last.halted`）を「起動中の印」（鍵 `areka.last.running`）で置き換え、切替の記憶を書く時点を「降ろした時点」と「切替先の定常到達」へ移し、OS のセッションの終了（`WM_ENDSESSION`）の後始末を足した。該当は「SessionMark」「SessionEnd」の節・Flow 5〜7・Error Handling・Testing Strategy。選んだ理由と選ばなかった案は `research.md` §12。

## Overview

**Purpose**: プロセスを生かしたまま別のゴーストへ替わる。台本の `\![change,ghost,名(,--option=raise-event)]` と右クリックメニューの「ゴースト」枠が同じ 1 本の切替要求を出し、kanade が送り出しの握手（`OnGhostChanging` → 204 なら `OnClose` → 別れの台詞の再生完了）を行い、areka がゴーストを降ろして（全窓を閉じ・SHIORI を解放し・終了しない）切替先を起こし、kanade が迎え入れの握手（`OnGhostChanged` → 204 なら `OnBoot`）を行う。切替先が起動できなければ既定ゴースト（emo2）を起こし、その `OnBoot` の Ref6＝`halt`・Ref7＝落ちたゴースト名で伝える。あわせて kanade に外から SHIORI イベントを名前と Reference 列で頼める汎用の入口を 1 本作る。

**Users**: α の利用者（2 体目を入れた第三者）、ゴーストの作者（交代の台詞を書く）、後続 spec の開発者（`shell-balloon-switch`・`ghost-install`・`network-update`・`ghost-change-name-resolution`）。

**Impact**: 今日の「停止通知を受けると必ず終了へ進む一本道」（`crates/areka/src/emo2_boot/frame.rs` の `run_ghost_quit_phase` → `quit_app`）に、切替の目印が立っている間だけ「降ろして起こし直す」分岐が加わる。kanade の停止通知の線は「運行の通知」の線に広がり（停止・定常到達・切替の中止の 3 種を運ぶ）、`GhostSession` は `fn main` のローカル変数から World の資源へ移る。完了 `areka-P0-ghost-restart-unit` が用意した「登録 1 回・載せ替え n 回・全窓を閉じるが終了しない」の形が、本仕様で最初の本番の呼び手を得る。

### Goals

- 切替要求の入口を 1 本（`SwitchRequest`）にし、台本とメニューの両方がそこを通る（要件 1）。
- 送り出しと迎え入れの握手を正典の順序と Reference で行う（要件 2・4）。起動の根は 1 つ（初回起動が最優先）。
- 切替の間にプロセスを終了させない。降ろした側の停止通知を「切替による停止」として捌く（要件 3）。
- `raise-event` 付きの切替はバルーンブレークで中止でき、元の定常へ戻る（要件 5）。
- 切替先が起動できなければ既定ゴーストを起こし、`OnBoot` の Ref6/7 で伝える。既定ゴースト自身の失敗は致命（要件 6）。きれいに終わらなかった次の起動も既定ゴーストへ倒す（起動中の印・要件 12.1〜12.5）。
- 壊れた切替先の名前を `LastGhost` に載せない（降ろした時点で既定へ・切替先は定常到達で記憶する・要件 12.6〜12.8）。
- OS のシャットダウン・ログオフでもきれいに終わる（要件 12.9〜12.11）。
- kanade に汎用の通知の入口を 1 本作る（要件 7）。
- 判断の分岐を偽の SHIORI 2 体の決定論テストで固定し、実機で emo2 ⇄ R_POST_and_KOMAINU を往復する（要件 10）。

### Non-Goals

- 名前解決 `random`／`sequential`／`lastinstalled`・`\+`／`\_+`（`areka-P0-ghost-change-name-resolution`）。本仕様では「該当なし」として無視する。
- シェル切替・バルーン切替（`areka-P0-shell-balloon-switch`）。`(change,shell)`・`(change,balloon)` の消費者は 0 組のまま。
- `\![reload,…]`・`\![call,ghost,…]`・多重ゴースト・消滅・`\![set,scaling,…]`・`OnNotify*Info`・`currentghost.*`。
- 降ろす処理の別スレッド化（要件 3.8 の実測が 1 秒を超えたときの設計討議の議題として残す。`GhostSession` が `Send` であることは確認済み＝`ghost_session_restart_tests.rs` の `shutdown_bounded` が `run_bounded<F: Send>` へ move している）。
- 3 人以上のキャラの窓（`derive_scopes()` の `[0, 1]` 固定は据え置き）。
- 既定ゴースト emo2 の `halt` の台詞（開発者が後で足す）。
- 告知に押されたボタンを返す形（`ghost-install`）。

## Boundary Commitments

### This Spec Owns

- **切替要求の入口**: `crates/areka/src/emo2_boot/ghost_switch.rs` の `SwitchRequest` と `request_ghost_switch`（唯一の入口）。出どころ 2 つ＝台本の受け口 `ChangeCueSink`（`emo2_boot/change_cue.rs`）とメニューの「ゴースト」枠（`menu/ghost_frame.rs`）。
- **切替の目印**: UI 側の予約 `SwitchInFlight`（NonSend 資源・`ghost_switch.rs`）と、kanade の停止通知に載せる切替の中身 `ChangeHandoff`（`crates/areka-kanade/src/change.rs`）。二重要求の判定は UI 側（`SwitchInFlight` の有無）。
- **kanade の切替の相**: `crates/areka-kanade/src/schedule/change.rs`（`ChangePending`／`ChangeTalkWait`／`ChangeClosePending`／`ChangeCloseTalkWait` の 4 相と、中止して定常へ戻る経路・終了要求で切替を取りやめる経路）。
- **運行の通知の線**: `KanadeNotice`（`Steady`／`ChangeCancelled`／`Stopped`）と `Action::Notice`。停止通知の受け口 `KanadeStopRx` は `KanadeNoticeRx` に広がる。
- **起動の根の表**: `crates/areka-kanade/src/schedule/boot.rs` の `boot_root`（初回起動 → `OnFirstBoot`・切替 → `OnGhostChanged`・それ以外 → 根なし＝`OnBoot` だけ）と、`OnBoot` の Ref6/7（`BootOrigin::Halted`）。
- **直前のゴーストの情報を切替先へ渡す器**: `KanadeConfig` の `boot_origin: BootOrigin`（`Plain`／`ChangedFrom`／`Halted`）と `shell_folder`。派生関数 `areka_ghost::boot_with_origin`。
- **降ろして起こし直す経路**: `ghost_switch.rs` の `switch_to`／`switch_to_default`（`GhostSession::shutdown` → `close_windows_for_restart` → バルーンの解決 → 窓の準備 `reopen_ghost_windows` → `boot_ghost_strict` → 成功したときだけ窓の投函 `commit_ghost_windows`）。`GhostSession` の置き場 `GhostSlot`（NonSend）。根とプロファイルの置き場 `BootContext`（Resource）。
- **汎用の通知の入口**: `KanadeMsg::RaiseEvent` → `Input::RaiseEvent` → `change::on_raise_event`（許可表と定常の判定）→ `events::raise`。
- **切替後の記憶**: `GhostRoute::Switched`（記憶を書く経路）と、書く時点（降ろした時点＝`LastGhost` 既定＋印・切替先の定常到達＝`LastUsed`）。
- **起動中の印**（2026-09-27 裁定 13）: `PersistKey::LastRunning`（`areka.last.running`・`[last] running`・App スコープ）と、その読み書き（`boot_resolve.rs` の `read_session_mark`／`write_session_mark`／`write_switch_drop`／`clear_session_mark`／`running_name`）と、きれいな終わりの判定 `session_mark_verdict`（`main.rs`・後始末と OS のセッションの終了が共有する唯一の判定）。
- **OS のセッションの終了**: wintf のゴースト窓の受け口 `OnSessionEnd`（部品）と `WM_ENDSESSION` の配送、areka の `session_end::on_os_session_end` と終了の出所 `ExitOrigin::SessionEnd`。
- **切替先の失敗と致命**: `ExitOrigin::GhostFallbackFailed(ShioriFault)`（既定ゴーストの同期の失敗を今日の告知の場面へ流す）。
- **網羅台帳・§8・決定論テスト・実機サインオフの記録**（`signoff.md`）。

### Out of Boundary

- 名前解決（`random`／`sequential`／`lastinstalled`・`\+`／`\_+`）。`ghost_switch.rs` の `resolve_switch_target` が「該当なし」として `warn!` で捨てる腕を、`ghost-change-name-resolution` が置き換える。`crates/areka-parsers/src/sakura/decode.rs`・`crates/areka-sakura/src/compile.rs` には触らない。
- シェル・バルーンの切替と、メニューの「シェル」「バルーン」枠。`ChangeCueSink` は `("change","ghost")` の 1 組だけを担当し、`(change,shell)`・`(change,balloon)` は担当外として読み飛ばす（台帳へも登記しない）。
- `boot_ghost` の LogSink fallback の契約（初回起動の非致命）。切替では `boot_ghost_strict` を使い、fallback の腕は触らない。
- `close.rs` の終了の握手（定常へ戻る経路は 0 本のまま）。切替の相は `close.rs` を流用せず `change.rs` に自分の `OnClose` の段を持つ。
- 告知の場面（`AlertScene` は 5 場面のまま）・メッセージボックスの新設。
- 3 人以上のキャラの窓・切替中の `status` 値・別スレッドで降ろす形。
- `steady.rs`（935 行）・`actor_tests.rs`（970）・`runtime_tests.rs`（986）・`schedule_tests.rs`（962）・`steady_flow_tests.rs`（931）への行の追加。`steady.rs` は既存の match の腕 1 つの書き換え（行数の増減 0）だけ。
- `OnCloseAll`（OS のセッションの終了でも `OnClose` だけ）・`WM_QUERYENDSESSION` で OS の終了を拒む／遅らせること・OS のセッションの終了での別れの台詞・32bit helper のプロセスが OS から受けるセッションの終了（helper 側の振る舞いは変えない）。

### Allowed Dependencies

- 完了 `areka-P0-ghost-restart-unit`: `ghost_session::register_systems`（1 回）・`boot_ghost`（n 回）・`GhostSession::shutdown`・`app_exit::close_windows_for_restart` → `ghost_session::reopen_ghost_windows`・`Emo2BootInputs`。`#[cfg_attr(not(test), allow(dead_code))]` 2 か所を外す。
- 完了 `areka-P0-shiori-fault-notice`: `KanadeStopped{cause: Fault(ShioriFault)}`・`AlertScene::ShioriFault`・`FirstExit`／`fault_of`・`finish_after_run`。
- 完了 `areka-P0-baseware-root-layout`: `areka_ghost::catalog`（`list_ghosts`・`list_balloons`・`companion_balloon`・`BasewareRoot`）・`boot_resolve`（`resolve_balloon`・`read_last_balloon`・`LastUsed::record`・`DEFAULT_GHOST_FOLDER`）・`boot_config::resolve_root`・`main::on_boot_ok`。
- 完了 `areka-P0-popup-menu-minimal`: `menu::register`・`Frame::Ghost`・`ItemBody::Submenu`・`MenuRegistry`（起こすたびに新品）。
- 完了 `areka-P0-balloon-break`: `Input::UserBreak` → `user_break::on_user_break` → `Action::CancelChoice` → `TalkDone{Interrupted, quit_reserved}`。
- `dola::cue::CueSink`／`CueCommand::as_command_carrier`（受け口の型）・`areka_sylphya::persist`（`load_scope`／`save_scope`／`PersistKey`）・`std::sync::mpsc`。
- `wintf` の窓の手続きの配送表（`ecs/window_proc/mod.rs` の `dispatch_window_message`）と「窓に関数を差す部品」の前例 `OnCloseRequest`（`ecs/window/components.rs`・`WM_CLOSE` の受け手は `ecs/window_proc/lifecycle.rs`）。本仕様は同じ形の部品 `OnSessionEnd` と `WM_ENDSESSION` の腕を 1 つずつ足す（wintf は areka を知らないまま）。
- 依存の向き: `areka-talk` → `areka-kanade`（`change.rs`・`schedule/change.rs`・`events.rs`）→ `areka-ghost`（`runtime.rs`・`catalog.rs`）→ `areka`（`ghost_switch.rs`・`change_cue.rs`・`ghost_session.rs`・`menu/ghost_frame.rs`・`main.rs`）。上流は下流を知らない（kanade は areka の `SwitchInFlight` を知らず、`areka-ghost` は kanade の相を知らない）。

### Revalidation Triggers

- `KanadeNotice` の変種の増減・`KanadeStopped` の欄の増減（`shell-balloon-switch`・`network-update` が受け口の形を前提にする）。
- `SwitchRequest`／`GhostSpec` の形の変更（`ghost-install`・`ghost-change-name-resolution` が入口の形を前提にする）。
- `KanadeMsg::RaiseEvent` の引数の形・`ALLOWED_EVENT_IDS` の照合規則・再生中の応答の政策 `value_replaces_active_talk`（後続 3 spec が使う）。
- `KanadeConfig.boot_origin`（`BootOrigin`）の変種の増減（`OnGhostCalled`・`OnVanished` を足す spec が根の表へ行を足す）。
- `GhostSlot`／`BootContext` の欄の変更（`fn main` の後始末と `shell-balloon-switch` の「起動時に最後のシェルを効かせる口」が読む）。
- `PersistKey::LastRunning` の鍵の綴り（`areka.last.running`・`[last] running`）と「空文字は無し」の読み（2026-09-27 に `LastHalted`／`areka.last.halted` から改名。main に未着地なので移行は持たない）。
- 終了の経路の追加・変更（後続の `shell-balloon-switch`・`network-update` など）: きれいな終わりで印を消すのは `session_mark_verdict` を通る 2 か所（`fn main` の後始末・`session_end::on_os_session_end`）だけ。新しい終了の出所を足すときはこの判定の表に行を足す（`ExitOrigin` の網羅の match が漏れを止める）。
- App スコープの記憶へ UI スレッドが直接書く時点（ゴーストの実行系が 1 つも動いていない間だけ・下の SessionMark の不変条件）。
- `close_windows_for_restart` が `GhostWindows` 資源を外すこと（窓を作り直す経路の前提）。

## Architecture

### Existing Architecture Analysis

- **停止通知の一本道**: `frame.rs` の `run_ghost_quit_phase` は受け口 `KanadeStopRx`（プロセスに 1 つ・`emo2_boot::wire_kanade_stop` が据える）から `try_recv` で全件取り出し、最初の 1 件の原因で `quit_app(world, ExitOrigin::KanadeStopped(cause))` を呼ぶ。`KanadeStopped{cause}` は原因 5 値だけを運ぶ。kanade は `Action::StopSelf` の実行点（`actor.rs` の `execute_actions`）で `notify_stop` を 1 回呼ぶ。`GhostRuntime::shutdown` が送る `ForceQuit` は kanade が既に止まっていれば空振りする（通知は増えない）。
- **kanade の相**: `schedule/mod.rs` の `step` は横断の腕（`ForceQuit`・`ShioriDown`・`TalkDone`・`ShioriReply` の失敗）を先に判定し、残りを `dispatch_phase` で相ごとの `step`（`boot`／`steady`／`close`）へ委譲する。`close.rs` の握手は 3 つの終わり方すべてで `Unloading` へ進む（定常へ戻る経路 0 本）。`State` には `pending_close: Option<CloseReason>` の先例がある。`Phase` の `dispatch_phase`・`phase_label` は wildcard 無し（値を足せばコンパイルが漏れを止める）。
- **起動系列**: `boot.rs` の `on_prefetch_reply` が `config.first_boot` で `OnFirstBoot`（→ `BootType`）と `OnBoot`（→ `BootMain`）に分ける。`BootType` の応答は 204 → `OnBoot`・Value → `OnBoot` を飛ばして `basewareversion`。`BootVersion` + `Notified` で `Steady` へ（外へ知らせる手段は無い）。
- **外からの入力**: `KanadeMsg` は 12 変種。`actor.rs` の `spawn_kanade_with_stop_sink` が 1 対 1 で `Input` へ写す（`Close`・`ResourceQuery` だけ `step` を経ない）。`round_trip_request` は許可表に無い ID を `error!` ＋ `Failed(Internal)` にし、応答待ちの相では `Unloading{Fault}` へ倒す。
- **`GhostSession` の置き場**: `fn main` のローカル変数で、`finish_after_run` の後始末の閉包へ move される。フレームの系からは届かない。
- **窓**: `open_ghost_windows` は配置の準備を同期で行い、窓の生成は作業プール経由で次の `Input` 段に着く（同じ `Input` 段に閉包が 2 つ積まれれば両方の窓が生える＝切替先の起動に失敗したあと既定ゴーストの窓を頼むと、壊れた切替先の窓も孤児として生えてしまう）。`spawn_ghost_windows` が `GhostWindows` 資源を差し替える。`close_windows_for_restart` は窓を消すが `GhostWindows` を外さない（消した窓の `Entity` が残る＝`run_attach_phase` のゲートを通ってしまう）。
- **記憶**: `LastUsed::record` は `GhostRoute::Argv` のとき書かない。記憶ファイルは `areka_sylphya::persist::load_scope`／`save_scope` で実 fs から直接読み書きできる（起動前の解決が使っている）。
- **メニュー**: `Frame::Ghost` の枠・`ItemBody::Submenu`（`plan.rs`／`win32.rs` で表示まで結線済み）・`menu::register` が登記待ち。`MenuWiring` は `boot_ghost` の結線ありの腕で `wire_menu` が起こすたびに新品にする。メニューの動作は `trigger.rs` の `finish` で UI スレッドの `&mut World` を持って走る（系の外）。
- **受け口**: `ReadmeCueSink`（`readme_cue.rs`）＝自己選別＋送り出しの 2 段。UI 側は `wire_readme`（ゴーストごとに NonSend を挿す）と `register_readme_drain`（プロセスに 1 回・`Input` 段）の対。

### Architecture Pattern & Boundary Map

```mermaid
graph TB
    subgraph TalkThread[talk スレッド]
        ChangeCueSink[ChangeCueSink 自己選別と送り出し]
    end
    subgraph UiThread[UI スレッド]
        MenuGhostFrame[menu ghost_frame ゴースト枠]
        RequestSwitch[ghost_switch request_ghost_switch 入口 1 本]
        SwitchInFlight[SwitchInFlight 切替の予約]
        NoticePhase[frame run_ghost_quit_phase 通知の相]
        SwitchTo[ghost_switch switch_to 降ろして起こす]
        GhostSlot[GhostSlot と BootContext]
        Catalog[catalog list_ghosts と sakura_name]
    end
    subgraph Kanade[kanade スレッド]
        ChangeMsg[KanadeMsg ChangeGhost と RaiseEvent]
        ChangePhase[schedule change 切替の相]
        BootRoot[schedule boot boot_root 根の表]
        Notice[KanadeNotice Steady ChangeCancelled Stopped]
    end
    subgraph Ghost[areka-ghost]
        BootOrigin[boot_with_origin BootOrigin]
    end
    ChangeCueSink -->|ChangeRequest mpsc| RequestSwitch
    MenuGhostFrame --> RequestSwitch
    RequestSwitch --> Catalog
    RequestSwitch --> SwitchInFlight
    RequestSwitch -->|ChangeGhost| ChangeMsg
    ChangeMsg --> ChangePhase
    ChangePhase --> Notice
    Notice -->|mpsc| NoticePhase
    NoticePhase --> SwitchInFlight
    NoticePhase --> SwitchTo
    SwitchTo --> GhostSlot
    SwitchTo --> BootOrigin
    BootOrigin --> BootRoot
```

**Architecture Integration**:

- **選んだ形**: 案 C（既存の型と相に腕を足す拡張を基本に、迎え入れの状態だけを UI の小さな資源 `SwitchInFlight` に切り出す）。送り出しの目印は kanade の相（`Phase::Change*`）と `State.change`、迎え入れの目印は UI の `SwitchInFlight`。両者を結ぶのは 1 本の運行の通知の線（`KanadeNotice`）で、順序が保証される（同じ `mpsc` の FIFO）。
- **責務の分割**: 名前の突き合わせ・二重要求の判定・降ろして起こす・既定へ戻す＝UI（`ghost_switch.rs`）。握手の順序・中止・終了要求への譲り・Reference の組み立て＝kanade（`change.rs`・`events.rs`・`boot.rs`）。直前のゴーストの情報の運搬＝`areka-ghost`（`boot_with_origin`）。
- **既存のまま**: `close.rs` の握手・`quit_app`・`ExitPolicy::Explicit`・`register_systems` の 1 回・`boot_ghost` の fallback・`ALLOWED_EVENT_IDS` の 1 か所での照合・`*_cue.rs` の 2 段の型・`wire_*`／`register_*_drain` の対。
- **新しい部品の理由**: `KanadeNotice`＝切替先が定常に入った時点と中止を UI が知る唯一の手段（今日は無い・時間で区切らない）。`SwitchInFlight`＝切替先の失敗（非同期の `Fault`）を「切替の途中」と見分けるため。`BootContext`＝目録とバルーンの解決に要る根がどこにも無いため。`GhostSlot`＝フレームの系から降ろすため。`boot_root`＝ukadoc の「204 なら続けて」の木を 1 か所で持つため。
- **Steering との整合**: 失敗の経路はすべて記録付き（無視 `warn!`・戻す `error!`・中止 `info!`・各段 `info!`）。1 フレーム遅らせる解は無い（切替は 1 つの排他 system の中で降ろして起こし、`GhostWindows` を外して次のフレームの装着を状態で止める）。新しい環境変数 0・新しい依存クレート 0。

### Technology Stack

| Layer | Choice / Version | Role in Feature | Notes |
|-------|------------------|-----------------|-------|
| 運行（kanade） | `areka-kanade`（純粋状態機械 `step` ＋ アクターの殻） | 切替の相・根の表・汎用の入口・運行の通知 | 新規依存なし |
| 結線（ghost） | `areka-ghost` | `boot_with_origin`・`catalog::sakura_name` | `GhostBootOptions` に欄を足さない |
| UI／ECS | `bevy_ecs`（World の NonSend／Resource）・`wintf`（`Input`／`Update` 段） | `SwitchInFlight`・`GhostSlot`・`BootContext`・`Input` 段の取り出し | 既存の段と順序をそのまま使う |
| 記憶 | `areka-sylphya::persist`（TOML・`load_scope`／`save_scope`） | `LastGhost`・`LastRunning`（起動中の印） | 鍵を 1 つ足す（2026-09-27 に `LastHalted` から改名） |
| OS のセッションの終了 | `wintf`（窓の手続きの配送表・窓に関数を差す部品）・Win32 `WM_ENDSESSION` | `OnSessionEnd` → `session_end::on_os_session_end` | 新規依存なし・`WM_QUERYENDSESSION` は既定の手続き |
| 台本の受け口 | `dola::cue::CueSink` | `ChangeCueSink` | `ReadmeCueSink` の型 |
| メニュー | OS ネイティブ（`TrackPopupMenuEx`・既存） | 「ゴースト」枠の子メニュー | `ItemBody::Submenu` は結線済み |
| テスト | 偽の SHIORI（`ScriptedShioriBackend`・x64 偽境界）・`log-capture-kit`・`sample-ghost-kit` | 決定論テスト | 実機は emo2 ⇄ R_POST_and_KOMAINU |

## File Structure Plan

### Directory Structure（新規ファイル）

```
crates/areka-kanade/src/
├── change.rs                        # 切替の語彙（ChangeRequest・ChangeTarget・ChangeOrigin・ChangeHandoff・CancelReason・BootOrigin・ChangedFrom・KanadeNotice・ShioriMethod）。lib.rs から pub 再輸出
└── schedule/
    ├── change.rs                    # 切替の相（4 相の step・受理・中止・終了要求への譲り）と汎用の入口の判断 on_raise_event
    ├── change_tests.rs              # 上の決定論テスト（要件 10.2〜10.4・10.8・2.9・5.x の kanade 側）
    ├── boot_root_tests.rs           # 起動の根の表 boot_root と定常到達の通知の固定（boot.rs の子・非公開の補助を使う）
    └── events_change_tests.rs       # on_ghost_changing／on_ghost_changed／on_boot の Ref6-7／raise の Reference の固定

crates/areka/src/
├── emo2_boot/
│   ├── change_cue.rs                # \![change,ghost,…] の受け口（自己選別＋送り出し・ReadmeCueSink と同型）
│   ├── change_cue_tests.rs
│   ├── ghost_switch.rs              # 入口 request_ghost_switch・SwitchRequest・SwitchInFlight・名前の突き合わせ・取り出しの系・switch_to／switch_to_default・on_ghost_stopped／on_notice
│   ├── ghost_switch_tests.rs        # 突き合わせ・二重要求・該当なし・目印の上げ下げ・Fault の振り分け（要件 10.6・10.7 の UI 側・6.x）
│   ├── ghost_switch_test_support.rs # 2 体の偽ゴーストを持つ根（ghost/A・ghost/B・ghost/emo2・balloon/…）の組み立て
│   └── frame_ghost_quit_switch_tests.rs  # 目印が立っている間の停止通知は quit_app を呼ばない・下りたあとは呼ぶ（要件 10.7）
├── ghost_session_switch_tests.rs    # 偽の SHIORI 2 体で同じ World の 1 周（要件 10.1・10.13・1.8・10.5）
├── main_session_mark_tests.rs       # 2026-09-27: main_halt_record_tests.rs を改名して書き換え。印を書く・消す・残す判定と次の起動（要件 12.1〜12.5・12.12 ⑴〜⑶）
├── ghost_session_switch_memory_tests.rs  # 切替の各時点の記憶と印・戻しの致命（要件 12.6〜12.8・12.12 ⑷⑸）
├── session_end.rs                   # OS のセッションの終了の受け手 on_os_session_end と済みの印 SessionEnded（要件 12.9〜12.11）
├── session_end_tests.rs             # 要件 12.12 ⑹（areka 側）
└── menu/
    ├── ghost_frame.rs               # 「ゴースト」枠の供給関数と子項目の動作
    └── ghost_frame_tests.rs         # 要件 10.9

.kiro/specs/areka-P0-ghost-shell-balloon-switch/signoff.md   # 実機 5 走行の記録（要件 10.12）＋ 2026-09-27 の ⑥〜⑨（要件 12.13）
```

### Modified Files

- `crates/areka-kanade/src/msg.rs` — `KanadeMsg` に `ChangeGhost(ChangeRequest)`・`RaiseEvent{id, references, method}` を足す。`KanadeStopped` に `handoff: Option<ChangeHandoff>` を足す（構築点は本番 1＝`actor::notify_stop`・テスト 7）。`KanadeConfig` に `boot_origin: BootOrigin`・`shell_folder: String` を足す（構造体リテラルは `new` の 1 か所だけ）。
- `crates/areka-kanade/src/lib.rs` — `change` モジュールの再輸出・`events` ファサードへ `on_ghost_changing`／`on_ghost_changed`／`raise`／`value_replaces_active_talk` を足す。
- `crates/areka-kanade/src/actor.rs` — `KanadeMsg` → `Input` の写し 2 変種・`Action::Notice` の実行・`stop_sink: Option<Sender<KanadeNotice>>`・`notify_stop` が `handoff_of(&state)` を載せて `KanadeNotice::Stopped` で送る。
- `crates/areka-kanade/src/actor_stop_notify_tests.rs` — 通知の型と欄の追随・`Steady`／`ChangeCancelled` の送出 2 本。
- `crates/areka-kanade/src/schedule/mod.rs` — `Input` に `ChangeGhost`／`RaiseEvent`、`Phase` に `ChangePending`／`ChangeTalkWait`／`ChangeClosePending`／`ChangeCloseTalkWait`、`Action` に `Notice(KanadeNotice)`、`State` に `change: Option<ChangeState>`・`pending_change: Option<ChangeRequest>`。`step` の横断の腕（`ChangeGhost`・`RaiseEvent` → `change` へ）、`on_talk_done` の 2 か所（切替の相では終了の予約を効かせない／`Steady{Some}` の完了で `pending_change` を消化）、`dispatch_phase`・`phase_label`・`current_talk_id`・`awaits_reply` の腕。
- `crates/areka-kanade/src/schedule/boot.rs` — `boot_root(config) -> Option<ShioriCall>` を足し、`on_prefetch_reply` の分岐をそれに置き換える。`BootVersion + Notified` で `Action::Notice(KanadeNotice::Steady)` を返す。
- `crates/areka-kanade/src/schedule/boot_sequence_tests.rs`・`boot_reply_branch_tests.rs` — `boot_complete` の Action が空でなくなる分の追随（形の固定。判断は `events_change_tests.rs`／`change_tests.rs`）。
- `crates/areka-kanade/src/schedule/events.rs` — `ALLOWED_EVENT_IDS` に `OnGhostChanging`・`OnGhostChanged`（13 語）。`on_ghost_changing`・`on_ghost_changed`・`raise`・`value_replaces_active_talk` を足し、`on_boot` を `boot_origin` で Ref6/7 付きに広げる。
- `crates/areka-kanade/src/schedule/steady.rs` — `on_reply` の `"OnMouseMove" | "OnMouseDoubleClick"` の腕を `origin if events::value_replaces_active_talk(origin)` に書き換える（**行数の増減 0**・`use` は既存の行へ足す）。
- `crates/areka-kanade/src/schedule/schedule_tests.rs`・`schedule_log_firing_tests.rs`・`close.rs` のテスト・`steady_test_support.rs`・`boot_test_support.rs`・`user_break_tests.rs`・`steady_flow_tests.rs` — `State` の構造体リテラル（21 か所）へ 2 欄の追随（値は `None`）。`steady_flow_tests.rs`・`schedule_tests.rs` は行を足さない＝`cargo fmt` が 1 欄 1 行に割るので欄の継ぎ足しでなく、`steady_test_support.rs` の基準の状態を使う構造体更新（`..base_state()`）で既存の欄の行を置き換え、差し引き 0 行以下にする。`schedule_tests.rs` の網羅の match（`Action`・既存の相）は腕が増えるので、その 2 本のテストを兄弟のテストファイルへ移す（消さない）。
- `crates/areka-ghost/src/runtime.rs` — `boot_with_origin(options, kanade_stop: Option<Sender<KanadeNotice>>, origin: BootOrigin)` を足し、`boot_with_kanade_stop` はそれを `BootOrigin::Plain` で呼ぶ。`config.shell_folder` を `mount.shell.dir` の末尾から詰める。
- `crates/areka-ghost/src/catalog.rs` — `sakura_name(ghost_dir) -> Option<String>`（`ghost/master/descript.txt` の `sakura.name` の単独の読み手・`companion_balloon` と同型）。`Identity` は変えない。
- `crates/areka-ghost/src/lib.rs` — `sakura_name` の再輸出。
- `crates/areka/src/emo2_boot/frame.rs` — `KanadeStopRx` → `KanadeNoticeRx(Receiver<KanadeNotice>)`。`run_ghost_quit_phase` は全件取り出したあと 1 件ずつ捌く: `Stopped` は目印が立っていれば `ghost_switch::on_ghost_stopped` へ委譲、立っていなければ今日どおり `quit_app`（最初の 1 件だけ）。`Steady`／`ChangeCancelled` は `ghost_switch::on_notice` へ。可視性を `pub(super)` から `pub(crate)` へ広げる（crate 直下の `ghost_session_switch_tests.rs` が回す）。
- `crates/areka/src/emo2_boot/frame_ghost_quit_tests.rs`・`frame_schedule_tests.rs`・`frame_ghost_quit_logsink_tests.rs` — 通知の型の追随（`KanadeNotice::Stopped(KanadeStopped{cause, handoff: None})`）。
- `crates/areka/src/emo2_boot/mod.rs` — `Emo2BootInputs` に `boot_origin: BootOrigin`。`wire_emo2_boot` で change の線を 1 本作り、`sinks` の 8 本目に `ChangeCueSink`、受信端を `ghost_switch::wire_change_rx` で World へ。`boot_with_origin` を呼ぶ。`wire_kanade_stop` の引数の型。
- `crates/areka/src/emo2_boot/consumer_ledger.rs` — `CommandConsumer::ChangeSink`・`canonical()` に `("change", Some("ghost"))`（9 組）。件数を固定するテスト `canonical_builds_without_duplicate` の 8 → 9。
- `crates/areka/src/ghost_session.rs` — `GhostSlot(Option<GhostSession>)`（NonSend）。`boot_ghost` の結線ありの腕を私有の `boot_wired` に括り出し、`boot_ghost`（fallback へ倒れる・署名不変）と `boot_ghost_strict`（倒れず `Err`）の 2 つの入口にする。`GhostSession::kanade()`・`ghost_dir()`／`names()` の読み口。`register_systems` に `ghost_switch::register_change_drain` を足す。結線ありの腕の `wire_menu` の直後に `menu::ghost_frame::register(world)`。窓を「準備」と「投函」に分ける: `open_ghost_windows` の中身を `prepare_ghost_windows(world, cfg) -> Result<PreparedWindows, OpenWindowsError>`（同期・配置の準備と descript の 1 度の読取・spawn の閉包を持つ）と `commit_ghost_windows(world, prepared)`（閉包を作業プールへ渡す）に括り出し、`open_ghost_windows` は両方を続けて呼ぶ（署名不変・初回起動の `main` の順序は据え置き）。`reopen_ghost_windows` は `PreparedWindows` を返す形に変える（本番の呼び手は本仕様が初めてなので形を変えてよい）。`#[cfg_attr(not(test), allow(dead_code))]` を外す。
- `crates/areka/src/ghost_session_restart_tests.rs` — 通知の型の追随・2 周目のメニューの登記の一覧に `Frame::Ghost` が加わる分の追随。
- `crates/areka/src/app_exit.rs` — `ExitOrigin::GhostFallbackFailed(ShioriFault)`・`fault_of` の腕・`close_windows_for_restart` が `GhostWindows` 資源を外す・`#[cfg_attr(not(test), allow(dead_code))]` を外す。
- `crates/areka/src/main.rs`（既出の行に加えて）— 初回起動の順序（`open_ghost_windows` → `boot_ghost`）は据え置き（そこでは失敗が告知と終了で終わるので孤児の窓は残らない）。
- `crates/areka/src/app_exit_tests.rs` — `fault_of` の新しい腕と `GhostWindows` を外す判断。
- `crates/areka/src/boot_config.rs` — `BootContext`（Resource）と `CurrentGhost`。`resolve_balloon_for_ghost(root, ghost_dir, pick)` を `resolve_boot_from` から括り出す。`BootResolved` の 4 つ目は前回落ちたゴーストの名前（2026-09-27: 起動中の印の値＝`read_session_mark`。印が在れば `LastGhost` を読まない）。
- `crates/areka/src/main_config_input_tests.rs` — `BootResolved` の形の追随。
- `crates/areka/src/boot_resolve.rs` — `GhostRoute::Switched`。起動中の印の読み書き（2026-09-27・下の SessionMark）: `read_session_mark`・`write_session_mark`・`write_switch_drop`・`clear_session_mark`・`running_name`。置き換え前の `read_last_halted`／`take_last_halted`／`record_halt` は消す（呼び手ごと置き換える）。
- `crates/areka/src/boot_resolve_tests.rs` — 置き換え前の往復のテスト（`record_halt_rewrites_to_default_and_take_returns_name_once` ほか 3 本）を印の往復（書く → 読む → 消す・書けないときの `warn!` 1 件・切替の 1 回の書き込みで 2 つの鍵がそろう）へ書き換える。
- `crates/areka/src/main.rs` — `resolve_boot` の 4 つ目の戻り（前回落ちた名前＝印の値）と 5 つ目の戻り（根 `BasewareRoot`＝`BootContext` 用）・`BootContext` と `GhostSlot` の据え付け・`boot_origin` の受け渡し・`run()` の後は `GhostSlot` から取り出して後始末・告知の場面は `BootContext.current` の根と `GhostSession` の名前で組む。2026-09-27: `boot_first_ghost` が argv でなければ起こす前に `write_session_mark`、`should_record_halt` を `session_mark_verdict` に置き換え、後始末は降ろした後にきれいな終わりなら `clear_session_mark`。`on_boot_ok` から記憶を書く部分を `record_last_used` に括り出し、経路が `Switched` のときは書かない（定常到達で `ghost_switch` が呼ぶ）。`SessionEnded` が在れば告知も印も触らない。
- `crates/areka/src/session_end.rs`（新規）・`crates/areka/src/app_exit.rs` — `ExitOrigin::SessionEnd`（`fault_of` は `None`）・`attach_os_close_request` が同じ窓へ `OnSessionEnd(session_end::on_os_session_end)` も差す。
- `crates/areka/src/emo2_boot/ghost_switch.rs` — `switch_to` が降ろした直後に `write_switch_drop(切替先の名前)`、`on_notice(Steady)` の迎え入れの段で `record_steady_memory`（切替先の実行系の記憶の書き手で `LastUsed` と印）。
- `crates/areka/src/ghost_session.rs` — `GhostSession::runtime()`（定常到達で切替先の記憶の書き手を借りる読み口）。
- `crates/wintf/src/ecs/window/components.rs`・`ecs/window_proc/mod.rs`・`ecs/window_proc/lifecycle.rs`・`lifecycle_tests.rs` — 部品 `OnSessionEnd` と `WM_ENDSESSION` の腕と受け手（下の SessionEnd）。
- `crates/areka/src/menu/mod.rs` — `ItemBody::Submenu`・`register` の `#[allow(dead_code)]` を外す。`unregister` のコメントを「呼び手なし（枠の取り消しは今日の spec に無い）」へ改める。`ghost_frame` モジュールの宣言。
- `crates/areka-sylphya/src/persist/mod.rs`・`persist/format.rs`・`persist_tests.rs` — `PersistKey::LastRunning`（`areka.last.running`・`[last] running`・2026-09-27 に `LastHalted`／`areka.last.halted`／`[last] halted` から改名）。空文字は「無し」と読む。
- `doc/ukadoc-coverage/ledger/shiori.toml`・`sakura-script.toml` — 3 行を `implemented`（owner＝本仕様）へ。生成物（`report/*.md`）は生成器（`ukadoc-survey -- report`／`report-summary`）で作り直す。手書きの `briefing-sakura-script.md` の「未対応」の行と、検査が数を照合する `briefing.md`・`roadmap-draft.md` は手で追随させる。
- `doc/COMPAT_ARCHITECTURE.md` — §8 に 9 行（要件 9.2 の (a)〜(i)）。2026-09-27: (i) の行を起動中の印へ書き換え、(j) の行（OS のセッションの終了）を足す。
- `doc/ukadoc-coverage/ledger/shiori.toml` — `OnClose` の備考に「OS のセッションの終了で Ref0＝`system` の通知」、`OnBoot` の備考に「Ref6/7 は切替先の失敗と起動中の印で載る」を足し、生成物を生成器で作り直す（状態の列は変わらない）。
- `.kiro/steering/roadmap.md` — α 後の候補「壊れたゴーストを表示し続ける」を登記だけの行として足す（要件 11.10）。

行数の見込み（1,000 行の目安）: `msg.rs` 852 → 約 900・`schedule/mod.rs` 758 → 約 830・`events.rs` 431 → 約 520・`boot.rs` 323 → 約 350・`actor.rs` 541 → 約 580・`emo2_boot/mod.rs` 767 → 約 800・`frame.rs` 474 → 約 510・`ghost_session.rs` 454 → 約 540・`main.rs` 556 → 約 590・`boot_config.rs` 292 → 約 360・`boot_resolve.rs` 343 → 約 400・`app_exit.rs` 231 → 約 250・`consumer_ledger.rs` 726 → 約 740。新規ファイルはいずれも 500 行未満に収める。2026-09-27 の追加（実装後の実測から）: `main.rs` 742 → 約 760（`should_record_halt` を判定の表に置き換え・差し引きは小さい）・`boot_resolve.rs` 424 → 約 440・`ghost_switch.rs` 622 → 約 670・wintf `lifecycle.rs` 414 → 約 460・`session_end.rs` 約 120。

## System Flows

### Flow 1: `raise-event` 付きの切替（成功）

```mermaid
sequenceDiagram
    participant Talk as talk スレッド
    participant UI as UI スレッド
    participant KA as kanade A
    participant KB as kanade B
    Talk->>UI: ChangeRequest 名 raise_event
    UI->>UI: 目録と突き合わせ SwitchInFlight SendOff を立てる
    UI->>KA: ChangeGhost 切替先 origin raise_event
    KA->>KA: OnGhostChanging GET ChangePending
    alt 台本あり
        KA->>KA: StartTalk ChangeTalkWait
        KA->>KA: TalkDone Ended で Unloading
    else 204
        KA->>KA: OnClose GET ChangeClosePending
        KA->>KA: 台本あり StartTalk ChangeCloseTalkWait なし Unloading CloseSilent
    end
    KA->>UI: Notice Stopped handoff 台本
    UI->>UI: shutdown 窓を閉じる GhostWindows を外す バルーン解決 窓の準備 boot_ghost_strict ChangedFrom 成功なら窓の投函
    UI->>UI: SwitchInFlight Welcoming Target
    KB->>KB: OnInitialize username OnGhostChanged GET 204 なら OnBoot basewareversion
    KB->>UI: Notice Steady
    UI->>UI: SwitchInFlight を下ろす info ghost_switch_done
```

流れの判断: ⑴ 突き合わせは降ろす前・`OnGhostChanging` を送る前に UI で行う（該当なしは `warn!` で終わり kanade へ何も送らない）。⑵ 降ろすのは通知を受けてから（`ForceQuit` は空振りし通知は 1 件）。⑶ 起動の根は `boot_root` が 1 つ選ぶ（B に起動記録が無ければ `OnFirstBoot`・`OnGhostChanged` 0 件）。⑷ 目印が下りるのは `Steady` の通知を受けた時点（切替先の非同期の `Fault` はそれより前に届くので既定へ戻す経路に乗る）。

### Flow 2: 切替の相の状態遷移（kanade）

```mermaid
stateDiagram-v2
    Steady --> ChangePending: ChangeGhost raise_event かつ talk なし
    Steady --> Steady: ChangeGhost talk あり pending_change に控える
    Steady --> Unloading: ChangeGhost raise_event なし talk なし cause CloseSilent
    Steady --> Steady: ChangeGhost pending_close あり 拒否 Notice ChangeCancelled Rejected
    ChangePending --> ChangeTalkWait: Value StartTalk
    ChangePending --> ChangeClosePending: NoContent OnClose GET
    ChangeTalkWait --> Unloading: TalkDone Ended または Quit
    ChangeTalkWait --> Steady: TalkDone Interrupted 中止 Notice ChangeCancelled
    ChangeTalkWait --> Unloading: Tick 期限 DeadlineExceeded
    ChangeClosePending --> ChangeCloseTalkWait: Value StartTalk
    ChangeClosePending --> Unloading: NoContent cause CloseSilent
    ChangeCloseTalkWait --> Unloading: TalkDone Ended または Quit
    ChangeCloseTalkWait --> Steady: TalkDone Interrupted 中止 Notice ChangeCancelled
    ChangeCloseTalkWait --> Unloading: Tick 期限 DeadlineExceeded
    ChangeTalkWait --> Steady: CloseRequest 切替を取りやめ talk を Steady へ戻し pending_close
    ChangeClosePending --> ClosePending: CloseRequest 取りやめ OnClose は送らない
    ChangeCloseTalkWait --> CloseTalkWait: CloseRequest 取りやめ
```

流れの判断: `Unloading` の原因は今日の値（`Quit`・`CloseSilent`・`DeadlineExceeded`・`Fault`）を使い、新しい原因は足さない。「切替で止まった」ことは停止通知の `handoff`（`State.change` から写す）と UI の目印が表す。中止（`Interrupted`）は `quit_reserved` を見ない（衝突表 2）。終了要求は `ChangeCancelled{CloseRequest}` を通知してから今日の終了の握手へ合流する（`OnClose` を既に送っていれば二度送らない）。

### Flow 3: 切替先の失敗と既定ゴースト

```mermaid
flowchart TD
    Stopped[Notice Stopped が届く] --> Marker{SwitchInFlight は}
    Marker -->|なし| Quit[今日どおり quit_app]
    Marker -->|SendOff| Handoff{handoff は}
    Handoff -->|None kanade が受理していない| QuitDrop
    Handoff -->|Some| SwitchTo[switch_to 切替先]
    Marker -->|Welcoming Target| CauseT{原因は}
    CauseT -->|Fault| Fallback[error 切替先を降ろす switch_to_default]
    CauseT -->|それ以外| QuitDrop[目印を下ろし quit_app]
    Marker -->|Welcoming Default| CauseD{原因は}
    CauseD -->|Fault| Fatal[目印を下ろし quit_app KanadeStopped Fault 告知 終了コード 1]
    CauseD -->|それ以外| QuitDrop
    SwitchTo --> SyncFail{同期の失敗 解決 窓の準備 起動}
    SyncFail -->|なし| Commit[窓の投函] --> Welcome[Welcoming Target]
    SyncFail -->|あり| IsDefault{切替先は既定か}
    IsDefault -->|いいえ| Fallback
    IsDefault -->|はい| FatalSync[quit_app GhostFallbackFailed]
    Fallback --> DefaultSync{既定の同期の失敗}
    DefaultSync -->|なし| WelcomeD[Welcoming Default]
    DefaultSync -->|あり| FatalSync
```

流れの判断: `SendOff` で届いた停止通知は `handoff` が `Some` のときだけ切替として捌く（`handoff` は kanade が切替の相を経て止まった証。`None` は利用者の終了と切替要求がほぼ同時で kanade が要求を受理しないまま止まった形＝終了が勝つ。`warn!(event="ghost_switch_not_accepted")` の上で目印を下ろし今日どおり `quit_app(KanadeStopped(cause))`）。窓は切替先の起動（`boot_ghost_strict`）が成功したあとで投函する（失敗したときに壊れた切替先の窓と既定ゴーストの窓が両方生えるのを防ぐ）。戻す試みは 1 回だけ（`Welcoming{Default}` からは戻らない）。既定ゴーストの `OnBoot` は `BootOrigin::Halted{ghost_name}` により Ref6＝`halt`・Ref7＝落ちた名前を載せる。`Welcoming` で届いた `Fault` 以外の停止（例: `OnGhostChanged` の台本が `\-` で終わった）は「切替先が終了を望んだ」として今日どおり終わる。

### Flow 4: 汎用の通知の入口

`KanadeMsg::RaiseEvent{id, references, method}` → `Input::RaiseEvent` → `change::on_raise_event`: ⑴ `events::allowed_static(&id)` が `None`（許可表に無い）→ `warn!(event="raise_event_not_allowed")` で捨てる。⑵ 相が `Steady` でない → `warn!(event="raise_event_not_steady")` で捨てる。⑶ `Steady` → `Action::ShioriRequest(events::raise(id, references, method, snapshot))`。応答は `steady::on_reply` へ流れ、`talk: None` なら再生、`talk: Some` の `Value` は `value_replaces_active_talk(origin)`（`OnSecondChange` 以外は真）で置き換える。

### Flow 5: 起動中の印と次の起動（2026-09-27・要件 12.1〜12.5）

```mermaid
flowchart TD
    Resolve[起動前の解決 argv なし] --> Mark{印は在るか}
    Mark -->|無し| Memory[今日どおり LastGhost の記憶から解く 由来 Plain]
    Mark -->|在る| NoMemory[info session_mark_found LastGhost を読まず 唯一 既定 無作為 で解く 由来 Halted 印の名前]
    Memory --> Write[起こす前に印を書く 起こすゴーストの名前]
    NoMemory --> Write
    Write --> Boot[boot_first_ghost]
    Boot --> Run[run]
    Run --> Verdict{session_mark_verdict}
    Verdict -->|Clear| Clear[降ろした後に印を消す info session_mark_cleared]
    Verdict -->|Keep| Keep[印を残す info session_mark_kept reason]
    Verdict -->|Untouched| None0[argv の起動 何もしない]
    Argv[起動前の解決 argv あり] --> BootArgv[印を読まず書かず起こす]
```

流れの判断: ⑴ 印は「先に書き、きれいな終わりでだけ消す」。強制終了・クラッシュ・電源断は後始末が走らないので自然に「残る」側へ落ちる（`Drop` に頼らない）。⑵ 印が在るとき `LastGhost` を読まないだけで、既定ゴーストを選ぶ段は `resolve_ghost` の既存の段 4（`DEFAULT_GHOST_FOLDER` を参照する唯一の場所）がそのまま担う（新しい経路の値も特別扱いも足さない）。配布物は既定ゴーストを同梱するので既定が選ばれる。根に既定が無ければ唯一・無作為の段が選び、そのゴーストにも Ref6/7 を載せる（前回落ちた事実は変わらない）。⑶ 印は読んだだけでは消さず、起こす前の書き込みが起こすゴーストの名前で上書きする（読んでから書くまでに落ちても、次の起動が同じ名前をもう一度受けるだけ）。⑷ 初回の起動の `LastGhost` は今日どおり起動の呼び出しが返った時点で書く（`on_boot_ok`）。壊れたゴーストがそこで記憶に載っても、落ちれば印が残って次の起動は `LastGhost` を読まないので、結末は切替と同じ（切替だけ時点を遅らせるのは、切替の途中で `LastGhost` が一瞬でも壊れた名前になるのを裁定が禁じたため・研究 §12）。

### Flow 6: 切替の記憶の時点（2026-09-27・要件 12.6〜12.8）

```mermaid
sequenceDiagram
    participant UI as UI スレッド
    participant FS as 記憶 App スコープ
    participant B as 切替先 B の実行系
    participant D as 既定 emo2 の実行系
    Note over FS: LastGhost=A 印=A
    UI->>UI: take_down A 降ろして書き出し済み
    UI->>FS: write_switch_drop LastGhost=emo2 印=B
    UI->>B: boot_into B Switched on_boot_ok は記憶を書かない
    alt B が定常に入る
        B->>UI: Notice Steady
        UI->>B: record_steady_memory
        B->>FS: LastGhost=B LastBalloon LastShell 印=B
    else B の Fault または同期の失敗
        UI->>UI: take_down B
        Note over FS: LastGhost=emo2 印=B のまま
        UI->>D: boot_into emo2 Default Halted B
        D->>FS: on_boot_ok LastGhost=emo2 今日どおり
        D->>UI: Notice Steady
        UI->>D: record_steady_memory
        D->>FS: 印=emo2
    else 既定も失敗 致命
        Note over FS: LastGhost=emo2 印=B のまま 次は emo2 と Ref7=B
    end
```

流れの判断: ⑴ UI スレッドが App スコープへ直接書くのは、ゴーストの実行系が 1 つも動いていない間だけ（初回の起動の前・降ろした直後・後始末で降ろした後）。動いている間の書き込みはそのゴーストの記憶の書き手（sylphya の `persist_put`・書き手の中で直列）を通す。記憶の保存は「読んで重ねて書く」ので、2 つの書き手が同時に書くと片方が消えうるため（SessionMark の不変条件）。⑵ 迎え入れの段を問わず、定常到達で「`LastUsed` と印＝今のゴーストの名前」を同じ手順で書く（切替先の段では印は既に切替先の名前・既定の段では `LastUsed` は既に既定＝どちらも同じ値の上書きで害は無い。段で分けないので分岐が増えない）。⑶ 切替先が定常に入る前に自ら終わった（`Welcoming` での `Fault` 以外の停止＝今日どおりの終了）ときは、きれいな終わりなので印は消え、`LastGhost` は既定のまま（定常に入らなかったゴーストは記憶しない）。

### Flow 7: きれいな終わりと OS のセッションの終了（2026-09-27・要件 12.2・12.9〜12.11）

```mermaid
sequenceDiagram
    participant OS as Windows
    participant W as ゴースト窓の手続き wintf
    participant SE as session_end on_os_session_end
    participant G as ゴーストの実行系
    participant FS as 記憶
    OS->>W: WM_QUERYENDSESSION
    W-->>OS: 既定の手続き TRUE
    OS->>W: WM_ENDSESSION wParam TRUE
    W->>SE: OnSessionEnd の関数 World を借りたまま
    SE->>SE: SessionEnded を立てる 切替の予約を下ろす quit_app SessionEnd
    SE->>G: GhostSession shutdown System
    G->>G: OnClose NOTIFY Ref0 system と unload と join と記憶の書き出し
    SE->>FS: session_mark_verdict が Clear なら印を消す
    SE-->>W: 戻る
    W-->>OS: 0
    Note over OS: この後いつ終了させられてもよい
```

流れの判断: ⑴ `WM_ENDSESSION` から戻った後はいつプロセスが終わってもよいので、`run()` の後の後始末に頼らず窓の手続きの中で同期に済ませる。⑵ OS はトップレベル窓ごとに送る（ゴースト窓は複数）ので、`SessionEnded` で 2 通目以降を読み捨てる。⑶ 別れの台詞は流さない: この処理の間 UI スレッドは塞がって台詞を描けず、戻れば終了させられうる。`GhostSession::shutdown(CloseReason::System)` は kanade へ `ForceQuit` を送り、kanade の既存の `force_quit` が `OnClose` を Ref0＝`system` の NOTIFY で送ってから SHIORI を降ろす（新しい経路を作らない）。⑷ 戻った後もプロセスが続けば、後から届く停止通知は予約が無いので今日どおり `quit_app`（2 度目は `app_exit_again` の `debug!`）、`run()` の後の `fn main` は `SessionEnded` を見て告知も印も触らない。⑸ `WM_QUERYENDSESSION` は既定の手続きが TRUE を返す（拒まない）ので受け手を置かない。`WM_ENDSESSION` の wParam＝FALSE（取りやめ）は何もしない。⑹ 切替の途中で全窓が 0 枚の一瞬に OS が終わると、トップレベル窓が無いので受け手が居ない＝印が残り次の起動は Ref6/7 になる（数百 ms の窓・受け入れる）。

## Requirements Traceability

| Requirement | Summary | Components | Interfaces | Flows |
|-------------|---------|------------|------------|-------|
| 1.1 | 入口 1 本 | GhostSwitch | `SwitchRequest`・`request_ghost_switch` | Flow 1 |
| 1.2, 1.3 | 台本からの要求（`raise-event` の有無） | ChangeCueSink → GhostSwitch | `ChangeRequest{name, raise_event}` | Flow 1 |
| 1.4 | メニューからの要求（`manual`） | GhostFrame → GhostSwitch | `SwitchRequest{GhostSpec::Folder, raise_event: true, Manual}` | Flow 1 |
| 1.5, 1.6, 1.7 | 名前の突き合わせ・該当なしは `warn!`・降ろす前に UI で判定 | GhostSwitch | `resolve_switch_target` | Flow 1 |
| 1.8 | 自分自身への切替 | GhostSwitch | `resolve_switch_target` は現在のゴーストを除外しない | Flow 1 |
| 1.9 | 二重要求は `warn!` | GhostSwitch | `SwitchInFlight` の有無 | Flow 1 |
| 1.10 | 汎用命令の名前の選別・型付き新設なし | ChangeCueSink・ConsumerLedger | `("change", Some("ghost"))` | — |
| 1.11 | 「ゴースト」枠の列挙 | GhostFrame・BootContext | `ghost_frame_item` | — |
| 1.12 | 起こすたびに登記をやり直す | GhostSession（`boot_wired`） | `menu::ghost_frame::register` | — |
| 2.1 | `OnGhostChanging` GET・Ref0〜3 | Events・ChangePhase | `on_ghost_changing` | Flow 2 |
| 2.2, 2.3 | 台本あり→再生完了で降ろす／204→`OnClose` | ChangePhase | `ChangePending`・`ChangeClosePending` | Flow 2 |
| 2.4 | 降ろすことで終わる・`quit_app` 0 回 | ChangePhase・NoticePhase | `handoff`・`SwitchInFlight` | Flow 1・3 |
| 2.5 | 30 秒の上限で打ち切って続ける | ChangePhase | `deadline_from`・`DeadlineExceeded` | Flow 2 |
| 2.6 | `raise-event` 無しは何も送らず命令の台本の終わりで降ろす | ChangePhase | `pending_change`・`CloseSilent` | Flow 2 |
| 2.7 | SHIORI の解放を待ってから起こす | GhostSwitch（`switch_to`） | `GhostSession::shutdown` の同期 | Flow 1 |
| 2.8 | 握手中の `Fault` でも切替を続ける | NoticePhase・GhostSwitch | `on_ghost_stopped(SendOff, Fault)` | Flow 3 |
| 2.9 | 終了要求は切替に勝つ | ChangePhase・GhostSwitch | `ChangeCancelled{CloseRequest}` | Flow 2 |
| 3.1 | 完了 spec の操作を本番で呼ぶ | GhostSwitch・AppExit | `close_windows_for_restart`→`reopen_ghost_windows` | Flow 1 |
| 3.2 | 窓 0 でも `AppExit` を出さない | GhostSwitch | `close_windows_for_restart`（`AppExit` に触れない） | Flow 1 |
| 3.3 | 降ろした側の停止通知を切替として捌く | NoticePhase | `run_ghost_quit_phase` の目印の分岐 | Flow 3 |
| 3.4 | どのゴーストの通知か取り違えない | NoticePhase・GhostSwitch | `SwitchStage`（段）と FIFO の順序 | Flow 3 |
| 3.5 | 目印は定常到達で下ろす・その後は今日どおり | Notice・NoticePhase | `KanadeNotice::Steady`・`on_notice` | Flow 1 |
| 3.6 | `GhostSession` の置き場 | GhostSession | `GhostSlot`（NonSend） | — |
| 3.7 | 切替の経路から終了の指示を出さない | GhostSwitch | `quit_app` の呼び出しは致命の 2 か所だけ | Flow 3 |
| 3.8 | 止まる時間 1 秒目標（実機で測る） | GhostSwitch | `info!(event="ghost_switch_down_ms")` | signoff |
| 3.9 | 閉じた事象を `info!` | AppExit | `windows_closed_for_restart`（既存の語彙） | — |
| 4.1 | `OnGhostChanged` GET・Ref0〜3・7 | Events・BootRoot | `on_ghost_changed`・`ChangedFrom` | Flow 1 |
| 4.2, 4.3 | 204 → `OnBoot`／台本 → `OnBoot` なし | BootRoot | `Phase::BootType` の既存の腕 | Flow 1 |
| 4.4 | 起動記録なしは `OnFirstBoot` 最優先・根は 1 つ | BootRoot | `boot_root` | Flow 1 |
| 4.5 | 以降は今日の定常 | BootRoot | `to_baseware_version`（不変） | — |
| 4.6 | `LastGhost` を書く（2026-09-27: 切替先の定常到達で） | GhostSwitch・Main | `GhostRoute::Switched`・`record_last_used`（`on_boot_ok` は `Switched` で書かない）・`record_steady_memory` | Flow 6 |
| 4.7 | 切替先のバルーンの解決・`LastBalloon`／`LastShell`（書くのは定常到達） | BootConfig・GhostSwitch | `resolve_balloon_for_ghost`・`record_steady_memory` | Flow 1・6 |
| 4.8 | 窓ごとの状態の置き換え | GhostSession | `boot_wired`（NonSend の挿し替え） | — |
| 4.9 | 1 スコープのゴースト | Emo2Boot（据え置き） | `derive_scopes` | signoff |
| 4.10 | 初回と同じ手順で窓を作る・位置の復元 | GhostSession | `reopen_ghost_windows` → `open_ghost_windows` | Flow 1 |
| 5.1 | 中断で中止・元の定常へ | ChangePhase | `ChangeTalkWait`／`ChangeCloseTalkWait` + `Interrupted` → `Steady` | Flow 2 |
| 5.2 | 目印を下ろし `info!`・次の要求を受ける | NoticePhase・GhostSwitch | `ChangeCancelled`・`on_notice` | Flow 2 |
| 5.3 | バルーンを隠す規則は不変 | （完了 `balloon-break`） | `TalkLifecycleSignal::UserBreak`（不変） | — |
| 5.4 | `\-` の予約を終了に結ばない | ChangePhase（mod.rs の腕） | `on_talk_done` の `break_quit && !is_change_phase` | Flow 2 |
| 5.5 | `raise-event` 無しの中断は切替を続ける | ChangePhase | `pending_change` の消化（`Ended`／`Interrupted` とも） | Flow 2 |
| 5.6 | 中止後の終了は今日の経路 | ChangePhase | 中止で `State.change` と `pending_change` を空にする | Flow 2 |
| 5.7 | 定常へ戻る経路は切替の相だけ | ChangePhase | `close.rs` 不変 | Flow 2 |
| 6.1 | 失敗は `error!`・切替先を降ろし既定を起こす・fallback へ倒れない | GhostSwitch・GhostSession | `boot_ghost_strict`・`switch_to_default` | Flow 3 |
| 6.2 | 既定の `OnBoot` に Ref6/7・`OnGhostChanged` 0 件・新イベント 0 | Events・BootRoot | `BootOrigin::Halted`・`on_boot` | Flow 3 |
| 6.3 | 告知を出さない | GhostSwitch | `alert::raise` を呼ばない | Flow 3 |
| 6.4 | 既定の失敗は致命・今日の `Fault` の経路 | GhostSwitch・AppExit | `ExitOrigin::GhostFallbackFailed`・`fault_of` | Flow 3 |
| 6.5 | 戻す試みは 1 回・壊れた切替先が既定なら即時終了 | GhostSwitch | `SwitchStage::Welcoming{attempt}` | Flow 3 |
| 6.6 | 切替先の `Fault` を戻す経路へ | NoticePhase | `on_ghost_stopped(Welcoming{Target}, Fault)` | Flow 3 |
| 6.7 | エラー応答は致命でない | （完了 `shiori-fault-notice`） | `round_trip_request` 不変 | — |
| 6.8 | 〔裁定 13 で要件 12.1〜12.5 へ置き換え〕 | SessionMark・Main | `read_session_mark`・`write_session_mark`・`session_mark_verdict`・`BootOrigin::Halted` | Flow 5 |
| 7.1 | 名前＋Reference＋GET/NOTIFY の口 | Msg・RaiseEvent | `KanadeMsg::RaiseEvent`・`ShioriMethod` | Flow 4 |
| 7.2 | 許可表に無ければ `warn!`・行を足すだけで送れる | RaiseEvent・Events | `allowed_static` | Flow 4 |
| 7.3 | 定常なら送り、再生中は置き換える | RaiseEvent・Steady | `value_replaces_active_talk` | Flow 4 |
| 7.4 | 定常以外は `warn!` で捨てる | RaiseEvent | `on_raise_event` | Flow 4 |
| 7.5 | `OnGhostChanging`／`OnGhostChanged` は専用の腕 | ChangePhase・BootRoot | `on_ghost_changing`／`on_ghost_changed` | Flow 1 |
| 7.6 | 入口の判断を決定論テストで固定 | Tests | `change_tests.rs` | — |
| 8.1, 8.2, 8.3 | 終了操作・終了コード・smoke は不変 | AppExit・Main | `quit_app`・`finish_after_run` 不変 | — |
| 8.4 | `run_ghost_quit_phase` の分岐は目印の間だけ | NoticePhase | `SwitchInFlight` の有無 | Flow 3 |
| 8.5 | シェル・バルーン・名前解決を持ち込まない | ChangeCueSink・GhostSwitch | 担当は `("change","ghost")` の 1 組 | — |
| 8.6 | `GhostBootOptions` に欄を足さない | Ghost（`boot_with_origin`） | 派生関数の引数で渡す | — |
| 8.7 | `sakura.name` は単独の読み手・切替先だけ読む | Catalog・GhostSwitch | `catalog::sakura_name` | Flow 1 |
| 8.8 | 1,000 行・上限に近いファイルへ足さない | File Structure Plan | 新規ファイル・`steady.rs` は増減 0 | — |
| 8.9 | `decode.rs`・`compile.rs` に触らない | — | — | — |
| 8.10 | 環境変数を足さない | — | — | — |
| 8.11 | 失敗の経路はすべて記録 | 全 Component | Error Handling の表 | — |
| 9.1 | 台帳 3 行と生成物 | Docs | `cargo run -p ukadoc-survey -- report`／`report-summary` | — |
| 9.2 | §8 に (a)〜(i) | Docs | `doc/COMPAT_ARCHITECTURE.md` | — |
| 9.3 | 登記待ちのコメントと「未対応」の行 | Docs・Menu | `menu/mod.rs`・生成物 | — |
| 10.1, 10.13 | 偽の SHIORI 2 体で 1 周・根は 1 つ | Tests | `ghost_session_switch_tests.rs` | Flow 1 |
| 10.2, 10.3, 10.4 | 204→`OnClose`／`raise-event` 無し／中止 | Tests | `change_tests.rs` | Flow 2 |
| 10.5 | 切替先の `Fault` → 既定・既定の失敗 → 即時終了 | Tests | `ghost_session_switch_tests.rs`・`ghost_switch_tests.rs` | Flow 3 |
| 10.6 | 該当なし・二重要求・自分自身 | Tests | `ghost_switch_tests.rs`・`ghost_session_switch_tests.rs` | — |
| 10.7 | 目印の間の停止通知は `quit_app` を呼ばない | Tests | `frame_ghost_quit_switch_tests.rs` | Flow 3 |
| 10.8 | 汎用の入口の判断 | Tests | `change_tests.rs` | Flow 4 |
| 10.9 | 「ゴースト」枠の登記 | Tests | `ghost_frame_tests.rs` | — |
| 10.10 | 判断の分岐だけ・既存は落とさない | Tests | Testing Strategy | — |
| 10.11, 10.12 | 実機 5 走行と記録 | signoff | `signoff.md` | — |
| 10.14 | 〔裁定 13 で要件 12.12 へ置き換え〕 | Tests | `main_session_mark_tests.rs`・`boot_resolve_tests.rs` | — |
| 11.1 | 裁定 1（中断で中止） | ChangePhase | Flow 2 | — |
| 11.2 | 裁定 2（既定へ戻す・既定の失敗は致命） | GhostSwitch | Flow 3 | — |
| 11.3 | 裁定 3（初回起動が最優先） | BootRoot | `boot_root` | — |
| 11.4 | 裁定 4（優先順・告知なし） | NoticePhase・GhostSwitch | Flow 3 | — |
| 11.5 | 裁定 5（`OnGhostChanged` の Ref1） | ChangePhase・Events | `ChangeHandoff.script` → `ChangedFrom.script` | — |
| 11.6 | 裁定 6（`raise-event` 無しは `OnClose` も送らない） | ChangePhase | `CloseSilent` へ直行 | — |
| 11.7 | 裁定 7（Ref1＝`manual`／`automatic`） | Events | `ChangeOrigin` | — |
| 11.8 | 裁定 8（`name` → フォルダ名・大文字小文字区別） | GhostSwitch | `resolve_switch_target` | — |
| 11.9 | 裁定 9（自分自身への切替） | GhostSwitch | 除外しない | — |
| 11.10 | 裁定 10（Ref6/7 で伝える・roadmap へ登記） | Events・Docs | `BootOrigin::Halted`・roadmap の行 | — |
| 11.11 | 裁定 11〔裁定 13 で置き換え〕 | SessionMark・Main | 要件 12 の行 | Flow 5 |
| 11.12 | 裁定を覆すなら議題へ | — | 設計討議 | — |
| 11.13 | 裁定 13（起動中の印・切替で降ろす時点の記憶・OS のセッションの終了） | SessionMark・GhostSwitch・SessionEnd・Docs | 要件 12 の行・§8 (i)(j) | Flow 5〜7 |
| 12.1 | 起こす前に印を書く（戻しは定常到達で既定の名前へ） | SessionMark・Main・GhostSwitch | `write_session_mark`・`write_switch_drop`・`record_steady_memory` | Flow 5・6 |
| 12.2, 12.3 | きれいな終わりでだけ消す・それ以外は残す | Main・SessionEnd | `session_mark_verdict`・`clear_session_mark` | Flow 5・7 |
| 12.4 | 印が在れば `LastGhost` を読まず既定へ・Ref6/7 | BootConfig・Main | `resolve_boot_from`（`memory: None`）・`first_boot_origin` | Flow 5 |
| 12.5 | argv の起動は印を読まず書かず消さない | BootConfig・Main | `argv_ghost` の分岐・`MarkVerdict::Untouched` | Flow 5 |
| 12.6 | 降ろした時点で `LastGhost` 既定＋印＝切替先・定常到達で `LastUsed` | GhostSwitch | `write_switch_drop`・`record_steady_memory` | Flow 6 |
| 12.7, 12.8 | 戻しの間は印＝切替先・既定の定常で既定の名前・致命でもそのまま | GhostSwitch | `switch_to_default`（印に触れない）・`record_steady_memory` | Flow 6 |
| 12.9 | `WM_ENDSESSION` で窓の手続きの中できれいに終わる | SessionEnd・wintf | `OnSessionEnd`・`on_os_session_end`・`ExitOrigin::SessionEnd` | Flow 7 |
| 12.10 | 1 回だけ・`WM_QUERYENDSESSION` は既定・取りやめは何もしない | SessionEnd・wintf | `SessionEnded`・`WM_ENDSESSION` の wParam | Flow 7 |
| 12.11 | 所要時間の記録・後始末は告知も印も触らない | SessionEnd・Main | `after_run` の `SessionEnded` の分岐 | Flow 7 |
| 12.12 | 判断の分岐の決定論テスト | Tests | `main_session_mark_tests.rs`・`ghost_session_switch_memory_tests.rs`・`session_end_tests.rs`・wintf `lifecycle_tests.rs` | — |
| 12.13 | 実機 ⑥〜⑨ | signoff | `signoff.md` | — |
| 12.14 | 環境変数・依存・1,000 行・記録 | 全 Component | File Structure Plan・Error Handling | — |

## Components and Interfaces

| Component | Domain/Layer | Intent | Req Coverage | Key Dependencies (P0/P1) | Contracts |
|-----------|--------------|--------|--------------|--------------------------|-----------|
| Change 語彙（`areka-kanade/src/change.rs`） | kanade・型 | 切替とその通知の語彙を 1 か所に置く | 2.1, 3.4, 4.1, 7.1 | `areka-talk`（P2） | Event |
| ChangePhase（`schedule/change.rs`） | kanade・相 | 送り出しの握手・中止・終了要求への譲り・汎用の入口の判断 | 2.x, 5.x, 7.x | `events`（P0）・`mod.rs` の横断の腕（P0） | State |
| BootRoot（`schedule/boot.rs` の `boot_root`） | kanade・相 | 起動の根を 1 つ選ぶ・定常到達の通知 | 4.1〜4.5, 6.2, 3.5 | `KanadeConfig.boot_origin`（P0） | State |
| Events（`schedule/events.rs`） | kanade・Reference | 新しい 4 つの組み立てと許可表・置き換えの政策 | 2.1, 4.1, 6.2, 7.2, 7.3 | `ALLOWED_EVENT_IDS`（P0） | Event |
| Actor（`actor.rs`） | kanade・殻 | 写し・通知の送出・`handoff` | 3.3, 3.5, 7.1 | `stop_sink`（P0） | Event |
| Ghost（`areka-ghost/src/runtime.rs`・`catalog.rs`） | 結線 | `boot_with_origin`・`sakura_name` | 4.1, 8.6, 8.7 | `resolve_kanade_config`（P1） | Service |
| ChangeCueSink（`emo2_boot/change_cue.rs`） | areka・talk 側受け口 | `\![change,ghost,…]` の自己選別と送り出し | 1.2, 1.3, 1.10, 8.5 | `dola::cue`（P0） | Event |
| GhostSwitch（`emo2_boot/ghost_switch.rs`） | areka・UI | 入口・突き合わせ・目印・降ろして起こす・既定へ戻す | 1.x, 2.7, 2.8, 3.x, 6.x | GhostSession（P0）・BootContext（P0）・AppExit（P0） | Service, State |
| NoticePhase（`emo2_boot/frame.rs` の `run_ghost_quit_phase`） | areka・UI 相 | 運行の通知を 1 件ずつ捌く | 3.3, 3.5, 6.6, 8.4 | GhostSwitch（P0）・`quit_app`（P0） | State |
| GhostSession（`ghost_session.rs`） | areka・起こし直しの単位 | `GhostSlot`・`boot_ghost_strict`・登記のやり直し | 1.12, 3.1, 3.6, 4.8, 4.10, 6.1 | 完了 `ghost-restart-unit`（P0） | Service |
| BootConfig／BootResolve（`boot_config.rs`・`boot_resolve.rs`） | areka・起動解決 | `BootContext`・バルーンの解決・`Switched`・起動中の印の読み書き（SessionMark） | 4.6, 4.7, 12.1〜12.8 | `catalog`（P0）・`sylphya::persist`（P0） | Service |
| SessionEnd（`session_end.rs`・wintf `OnSessionEnd`） | areka・終了／wintf・窓の手続き | OS のセッションの終了を窓の手続きの中できれいな終わりにする | 12.9〜12.11 | GhostSession（P0）・`quit_app`（P0）・`session_mark_verdict`（P0） | Event |
| GhostFrame（`menu/ghost_frame.rs`） | areka・メニュー | 「ゴースト」枠の子メニュー | 1.4, 1.11, 1.12 | `menu::register`（P0）・BootContext（P0） | — |
| AppExit（`app_exit.rs`） | areka・終了 | `GhostFallbackFailed`・`GhostWindows` を外す | 3.2, 3.9, 6.4 | — | Service |
| Main（`main.rs`） | areka・入口 | 据え付けと後始末・起動中の印を書く／消す判定 | 8.1, 8.2, 12.1〜12.5, 12.11 | GhostSlot・BootContext（P0） | — |
| Docs／Tests | 文書・検査 | 台帳・§8・決定論テスト・実機 | 9.x, 10.x | — | — |

### kanade

#### Change 語彙（`crates/areka-kanade/src/change.rs`）

| Field | Detail |
|-------|--------|
| Intent | 切替の要求・手渡し・通知・起動の由来の型を 1 ファイルに置き、`lib.rs` から再輸出する |
| Requirements | 2.1, 3.4, 4.1, 5.2, 7.1 |

**Responsibilities & Constraints**
- 型だけを置く（判断は持たない）。`Debug, Clone, PartialEq, Eq` を付け、決定論テストで丸ごと突き合わせられるようにする。
- `ShioriMethod` は `ShioriCall` の `Get`／`Notify` に写す（`ShioriCall` に第 3 の形を作らない）。

##### Event Contract（型）

```rust
/// 切替の要求（UI → kanade）。名前の突き合わせは UI 側で済んでいる。
pub struct ChangeRequest { pub target: ChangeTarget, pub origin: ChangeOrigin, pub raise_event: bool }
/// 切替先（`OnGhostChanging` の Ref0〜3 の材料）。`sakura_name` は無ければ空。`dir` は絶対パスの文字列。
pub struct ChangeTarget { pub sakura_name: String, pub name: String, pub dir: String }
/// `OnGhostChanging` の Ref1。
pub enum ChangeOrigin { Manual, Automatic }   // as_ref_str: "manual" / "automatic"
/// 停止通知に載せる切替の中身（kanade が切替の相を経て止まったことの証）。
pub struct ChangeHandoff { pub script: Option<String> }   // OnGhostChanging が返した台本（送らなかった・204・Fault は None）
/// 中止の理由（記録の語彙）。
pub enum CancelReason { UserBreak, CloseRequest, Rejected }   // Rejected＝kanade が受理しなかった（終了要求と競合）
/// 運行の通知（kanade → UI の 1 本の線）。
pub enum KanadeNotice { Steady, ChangeCancelled { reason: CancelReason }, Stopped(KanadeStopped) }
/// 起動の由来（`KanadeConfig.boot_origin`）。根の表の入力。
pub enum BootOrigin { Plain, ChangedFrom(ChangedFrom), Halted { ghost_name: String } }
/// 直前のゴーストの情報（`OnGhostChanged` の Ref0〜3）。
pub struct ChangedFrom { pub sakura_name: String, pub script: String, pub name: String, pub dir: String }
/// 汎用の入口の GET／NOTIFY。
pub enum ShioriMethod { Get, Notify }
```

`msg.rs` 側の変更: `KanadeMsg::ChangeGhost(ChangeRequest)`・`KanadeMsg::RaiseEvent { id: String, references: Vec<String>, method: ShioriMethod }`・`KanadeStopped { cause, handoff: Option<ChangeHandoff> }`・`KanadeConfig { …, boot_origin: BootOrigin, shell_folder: String }`（`new` の既定は `Plain` と `shell_name` の写し）。既存の `existing_eight_kanade_msg_variants_are_unchanged_by_additive_growth` の網羅 match へ 2 腕を足す。

#### ChangePhase（`crates/areka-kanade/src/schedule/change.rs`）

| Field | Detail |
|-------|--------|
| Intent | 送り出しの握手（`OnGhostChanging` → 204 なら `OnClose` → 台詞の完了）を切替の 4 相で行い、中止と終了要求への譲りを持つ。汎用の入口の判断も置く |
| Requirements | 2.1〜2.6, 2.9, 5.1, 5.2, 5.4〜5.7, 7.2〜7.4, 11.1, 11.6 |

**Responsibilities & Constraints**
- 相: `Phase::ChangePending`（`OnGhostChanging` の応答待ち）・`ChangeTalkWait { talk_id, deadline }`・`ChangeClosePending`（切替の `OnClose` の応答待ち）・`ChangeCloseTalkWait { talk_id, deadline }`。`close.rs` の `ClosePending`／`CloseTalkWait` の写しに「中断 → 定常」の腕を足した形で、`close.rs` は触らない（定常へ戻る経路は切替の相にだけ在る＝要件 5.7）。
- 帳簿: `State.change: Option<ChangeState { req: ChangeRequest, script: Option<String> }>`（受理で `Some`・中止で `None`・停止通知の `handoff` の源）と `State.pending_change: Option<ChangeRequest>`（`Steady{Some}` で受けた要求の保留・`pending_close` と同型）。
- 受理（横断の腕 `Input::ChangeGhost`）: `pending_close` が無い `Steady{talk: None}` → `begin_change`。`pending_close` が無い `Steady{Some}` → `pending_change` に控える（`info!(change_pending)`）。それ以外（`pending_close` が在る・切替の相・起動系列・終了系列）→ `warn!(event="change_rejected_phase", reason)` の上で `Action::Notice(ChangeCancelled{Rejected})` を返す（受理しなかった要求は必ず UI へ返し、UI の目印を残さない。UI が二重要求を弾くので、ここへ届くのは終了要求と競合したときだけ）。
- `begin_change(req)`: `state.change = Some(..)`・選択の帳簿を消す。`raise_event` なら `OnGhostChanging` GET → `ChangePending`。`raise_event` でなければ `Unloading{CloseSilent}` ＋ `ShioriUnload`（`OnClose` を送らない＝裁定 6）。
- `ChangePending` + `Value(script)` → `state.change.script = Some(script)`・採番して `StartTalk`・`ChangeTalkWait{deadline: deadline_from(last_now)}`。`NoContent` → `OnClose` GET（`events::on_close(CloseReason::System, INACTIVE)`）→ `ChangeClosePending`。`Notified`／その他 → `warn!` で維持。
- `ChangeTalkWait`／`ChangeCloseTalkWait` + `TalkDone{Ended}` → `to_unloading_quit`（`Quit` は横断の腕が同じ終端へ送る）。`TalkDone{Interrupted}` → **中止**: `state.change = None`・`pending_change = None`・`Steady{talk: None}`・`Action::Notice(ChangeCancelled{UserBreak})`・`info!(event="change_cancelled", reason="user_break")`。`Tick` の期限超過 → `error!(change_deadline_exceeded)` → `Unloading{DeadlineExceeded}`。
- `ChangeClosePending` + `Value` → `StartTalk`・`ChangeCloseTalkWait`。`NoContent` → `Unloading{CloseSilent}`。
- 終了要求（`Input::CloseRequest{reason}` が切替の相に届く・要件 2.9）: 取りやめ＝`state.change = None`・`pending_change = None`・`Action::Notice(ChangeCancelled{CloseRequest})`・`info!(event="change_yield_to_close", phase)`。相ごとの着地: `ChangePending` → `pending_close = Some(reason)` に控えて相は維持し、応答が来た時点で `pending_close` を見て取りやめる（`Value(script)` → 台本を `Steady{talk: Some(ActiveTalk{origin: "OnGhostChanging", script})}` として再生し、`steady::on_talk_done` が `pending_close` を消化して `OnClose` の握手を始める／`NoContent` → `pending_close` を取り出して `OnClose` GET（`events::on_close(reason)`）→ `Phase::ClosePending{reason}`）。`ChangeTalkWait{talk_id}` → `Steady{talk: Some(ActiveTalk{talk_id, origin: "OnGhostChanging", script})}` ＋ `pending_close = Some(reason)`（台詞は最後まで流れる）。`ChangeClosePending` → `Phase::ClosePending{reason}`（`OnClose` は既に送ってあるので送らない）。`ChangeCloseTalkWait{talk_id, deadline}` → `Phase::CloseTalkWait{talk_id, deadline}`（その別れの台詞の終わりで終了）。
- `mod.rs` の横断の腕への追加は 3 か所: ⑴ `Input::ChangeGhost` と `Input::RaiseEvent` の委譲、⑵ `on_talk_done` の `break_quit` を `take_user_break_quit(..) && !change::is_change_phase(&state.phase)` にする（要件 5.4）、⑶ `on_talk_done` の `Ended`／`Interrupted` で `Steady{Some}` かつ `pending_change.is_some()` なら `change::consume_pending(state, done)` へ（`steady.rs` に行を足さないための置き場）。`consume_pending` は `pending_close` が在れば取りやめ（`state.change = None`・`pending_change = None`・`Notice(ChangeCancelled{CloseRequest})`）の上で `steady::step` へ委譲する（終了が勝つ＝要件 2.9）。無ければ `pending_change` を取り出して `begin_change` と同じ手順を行う（`raise_event` 無しなら `Unloading{CloseSilent}`＝要件 2.6・5.5）。
- `dispatch_phase`・`phase_label`・`current_talk_id`（`ChangeTalkWait`／`ChangeCloseTalkWait` の `talk_id`）・`awaits_reply`（`ChangePending`／`ChangeClosePending`）へ腕を足す。
- 汎用の入口 `on_raise_event(state, id, references, method)`: `events::allowed_static(&id)` が `None` → `warn!(event="raise_event_not_allowed", id)` で捨てる。相が `Steady` でない → `warn!(event="raise_event_not_steady", id, phase)` で捨てる。`Steady` → `Action::ShioriRequest(events::raise(static_id, references, method, &state.snapshot()))`。待ち行列に積まない（要件 7.4）。

**Contracts**: State [x]

##### State Management
- 状態は `State`（`Phase`・`change`・`pending_change`）だけ。時計は `Tick` の注入時刻（`last_now`）で、実時間を読まない。
- 不変条件: `state.change` は `begin_change` だけが立て、中止・取りやめでだけ消え、`Unloading` まで残る（停止通知の `handoff` の源）。`pending_change` だけを控えている間 `state.change` は `None` のまま（保留を捨てて `\-` で終わる停止の `handoff` は `None`＝UI は終了として捌く）。取りやめ・中止で両方空になる。`Unloading` に至った時点の `state.change` がそのまま停止通知の `handoff` になる。

**Implementation Notes**
- Integration: `close.rs` の `deadline_from` を `pub(super)` にして共有する。`events::on_close` の `CloseReason` は `System`（切替は利用者の窓の操作ではない・Ref0＝`system`）。
- Validation: `change_tests.rs`（`close.rs` の `mod tests` の状態の組み立てを写す）。
- Risks: `pending_close` と `pending_change` の両方が在るときは `pending_close` が勝つ（要件 2.9）。`Steady{Some}` に `pending_change` が在り、その台本（`raise-event` 無しの `\![change,ghost,B]` と `\-` を同じ台本に書いた形）を利用者が中断して `\-` の予約が真のときは、横断の腕 ⑵ が先に効いて今日どおり終了系列へ進み、保留の切替は捨てる（`info!(event="change_dropped_by_quit")`・UI の目印を下ろすため切替の中止（`CloseRequest`）を通知する）。定常での `\-` の予約は終了で終わるという完了 `balloon-break` の規則を切替の保留は覆さない（要件 5.5 の「切替を続ける」は `\-` の予約が無いときの話）。中止で `Steady{None}` へ戻るとき `pending_close` が在れば次の `Tick` が `begin_close` を始める（既存の規則）。

#### BootRoot（`crates/areka-kanade/src/schedule/boot.rs`）

| Field | Detail |
|-------|--------|
| Intent | ukadoc の「204 なら続けて」の木を「根の表＋共通の葉」の 1 関数で持ち、定常到達を通知する |
| Requirements | 3.5, 4.1〜4.5, 6.2, 11.3 |

**Responsibilities & Constraints**
- `fn boot_root(config: &KanadeConfig) -> Option<ShioriCall>`: `config.first_boot` → `Some(on_first_boot(..))`。それ以外で `boot_origin` が `ChangedFrom(f)` → `Some(on_ghost_changed(f, &config.shell_folder, ..))`。`Plain`／`Halted` → `None`。範囲外の根（`OnGhostCalled`・`OnVanished`）は `BootOrigin` に値を足し、この表に行を足すだけで入る。
- `on_prefetch_reply` の分岐: `Some(call)` → `Phase::BootType` ＋ `[sink, call]`。`None` → `Phase::BootMain` ＋ `[sink, on_boot(config, ..)]`（今日の `boot_gate` の枝）。`BootType` の既存の腕（204 → `OnBoot`・Value → 飛ばして `basewareversion`）はそのまま使う（`OnGhostChanged` の 204 → `OnBoot`・台本 → `OnBoot` なし＝要件 4.2・4.3）。
- 起動記録: `first_boot` が真なら今日どおり `first_boot_epilogue` が書く（要件 4.4 の「同じ規則」）。切替で起きた `first_boot` 偽のゴーストは記録済み。
- `BootVersion + Notified`: `Steady` へ遷移し `vec![Action::Notice(KanadeNotice::Steady)]` を返す（要件 3.5 の「定常に入った時点」）。

**Contracts**: State [x]

#### Events（`crates/areka-kanade/src/schedule/events.rs`）

| Field | Detail |
|-------|--------|
| Intent | 新しい Reference の組み立てを単一列挙点に置く |
| Requirements | 2.1, 4.1, 6.2, 7.2, 7.3, 11.5, 11.7 |

##### Event Contract
| 組み立て | Method | References |
|---|---|---|
| `on_ghost_changing(req: &ChangeRequest, snapshot)` | GET | Ref0＝`target.sakura_name`・Ref1＝`origin.as_ref_str()`・Ref2＝`target.name`・Ref3＝`target.dir` |
| `on_ghost_changed(from: &ChangedFrom, shell_folder: &str, snapshot)` | GET | Ref0＝`sakura_name`・Ref1＝`script`・Ref2＝`name`・Ref3＝`dir`・Ref4〜6＝空・Ref7＝`shell_folder` |
| `on_boot(config, snapshot)`（拡張） | GET | Ref0＝`shell_name`。`boot_origin` が `Halted{ghost_name}` なら Ref1〜5＝空・Ref6＝`halt`・Ref7＝`ghost_name`。それ以外は Ref0 だけ（今日のまま） |
| `raise(id: &'static str, references: Vec<String>, method: ShioriMethod, snapshot)` | GET／NOTIFY | 渡された列をそのまま（欠番は呼び手が空で埋める） |

- `ALLOWED_EVENT_IDS` に `"OnGhostChanging"`・`"OnGhostChanged"` を足す（13 語）。`allowed_static(id: &str) -> Option<&'static str>`（許可表の一致した `&'static str` を返す＝`EventId::Static` と `origin` の両方に使える）。
- `value_replaces_active_talk(origin: &str) -> bool`: `origin != "OnSecondChange"`。マウス 2 語と汎用の入口の応答は置き換え、pump は今日どおり捨てる（要件 7.3）。`steady.rs` の腕はこの述語を呼ぶだけ。
- `origin` のラベル: 切替の相の `OnGhostChanging` の台本は `ActiveTalk.origin = "OnGhostChanging"`（取りやめで `Steady{Some}` へ戻すときに使う）。

#### Actor（`crates/areka-kanade/src/actor.rs`）

- `spawn_kanade_with_stop_sink(.., stop_sink: Option<Sender<KanadeNotice>>)`。`KanadeMsg::ChangeGhost` → `Input::ChangeGhost`・`RaiseEvent` → `Input::RaiseEvent`。
- `Action::Notice(n)` の実行: `stop_sink` があれば `send(n)`（失敗は `warn!(notice_send_failed)`・無ければ `debug!`）。`StopSelf` は `notify_stop(sink, term_cause, handoff)` で `KanadeNotice::Stopped(KanadeStopped{cause, handoff})` を送る。`handoff_of(&State) -> Option<ChangeHandoff>` は `drive` が `stop_cause_of` と同じ時点（`step` の直前）で控える。

### areka-ghost

#### Ghost（`crates/areka-ghost/src/runtime.rs`・`catalog.rs`）

##### Service Interface
```rust
pub fn boot_with_origin(options: GhostBootOptions, kanade_stop: Option<Sender<KanadeNotice>>, origin: BootOrigin) -> Result<GhostRuntime, GhostBootError>;
pub fn boot_with_kanade_stop(options, kanade_stop) -> Result<GhostRuntime, GhostBootError>; // = boot_with_origin(.., BootOrigin::Plain)
pub fn catalog::sakura_name(ghost_dir: &Path) -> Option<String>;  // ghost/master/descript.txt の sakura.name（companion_balloon と同型・読めなければ warn で None）
```
- `boot_with_origin` は `apply_boot_record_gate` の後に `config.boot_origin = origin`・`config.shell_folder = mount.shell.dir の末尾（無ければ shell_name）` を詰めてから kanade を起こす。`GhostBootOptions` に欄を足さない（要件 8.6）。

### areka（UI）

#### ChangeCueSink（`crates/areka/src/emo2_boot/change_cue.rs`）

| Field | Detail |
|-------|--------|
| Intent | `\![change,ghost,名(,--option=raise-event)]` を自己選別し、名前を無変形で UI へ運ぶ |
| Requirements | 1.2, 1.3, 1.10, 8.5 |

- `#[derive(Clone)] pub(crate) struct ChangeCueSink { tx: Sender<ChangeRequestRaw> }`。`impl dola::cue::CueSink`: ⑴ `as_command_carrier()` で開封（開けない荷物は `command == "change"` なら `warn!`・他は `debug!`）。⑵ `(name, params[0]) == ("change", "ghost")` だけ受理（`shell`／`balloon`・裸の `change` は `debug!` で担当外）。⑶ `params[1]` が無ければ `warn!(change_ghost_no_name)` で捨てる。⑷ `params[2..]` の `--option=raise-event` で `raise_event=true`。知らない option は `warn!` して無視する（切替は続ける）。⑸ `tx.send(ChangeRequestRaw { name: String, raise_event: bool })`。受信端が閉じていれば `warn!`（台本は殺さない）。
- 消費者台帳: `CommandConsumer::ChangeSink`・`canonical()` に `("change", Some("ghost"))`。`(change,shell)`・`(change,balloon)` は登記しない。
- `wire_emo2_boot` の `sinks` の 8 本目（文字 cue に依存しないので末尾）。受信端は `ghost_switch::wire_change_rx(world, rx)` が NonSend `ChangeRx` として World へ据える（ゴーストごとに新品）。

#### GhostSwitch（`crates/areka/src/emo2_boot/ghost_switch.rs`）

| Field | Detail |
|-------|--------|
| Intent | 切替要求の唯一の入口・名前の突き合わせ・切替の予約・降ろして起こし直す・既定へ戻す |
| Requirements | 1.1, 1.4〜1.9, 2.7, 2.8, 3.1〜3.4, 3.7, 3.8, 4.6, 4.7, 4.10, 6.1〜6.6, 8.4, 8.7, 11.2, 11.4, 11.8, 11.9 |

**Responsibilities & Constraints**
- UI スレッドだけで動く（`&mut World`）。talk スレッドからは `ChangeRx` 経由の要求だけが届く。
- 降ろすのは同期（`GhostSession::shutdown` の join を UI スレッドで待つ）。所要時間を `info!(event="ghost_switch_down_ms", ms)` で残す（要件 3.8 の実測）。

##### Service Interface
```rust
/// 切替要求（要件 1.1 の入口の型）。
pub(crate) struct SwitchRequest { pub ghost: GhostSpec, pub raise_event: bool, pub origin: ChangeOrigin }
/// 切替先の指し方。台本は名前（descript の name → フォルダ名の順・大文字小文字区別）、メニューはフォルダ名。
pub(crate) enum GhostSpec { Name(String), Folder(String) }
/// 唯一の入口。台本の取り出しの系とメニューの動作が呼ぶ。
pub(crate) fn request_ghost_switch(world: &mut World, req: SwitchRequest) -> SwitchVerdict;
pub(crate) enum SwitchVerdict { Accepted, Busy, NotFound, NoContext }   // 記録の語彙・呼び手は分岐しない
/// 純粋: 目録の項目と指し方から切替先を決める（現在のゴーストは除外しない）。
pub(crate) fn resolve_switch_target(entries: &[GhostEntry], spec: &GhostSpec) -> Option<SwitchTarget>;
/// 受信端の据え付け（ゴーストごと）と取り出しの系の登録（プロセスに 1 回・Input 段・dispatch_pointer_events の後）。
pub(crate) fn wire_change_rx(world: &mut World, rx: Receiver<ChangeRequestRaw>);
pub(crate) fn register_change_drain(world: &mut World);
/// 通知の相からの委譲。
pub(crate) fn on_ghost_stopped(world: &mut World, stopped: KanadeStopped);   // SwitchInFlight が在るときだけ呼ばれる
pub(crate) fn on_notice(world: &mut World, notice: KanadeNotice);            // Steady / ChangeCancelled
```
- Preconditions: `request_ghost_switch` は `BootContext` と `GhostSlot` が World に在ること（無ければ `warn!(ghost_switch_no_context)`・`NoContext`）。
- Postconditions: `Accepted` なら `SwitchInFlight{stage: SendOff}` が World に在り、`KanadeMsg::ChangeGhost` が kanade へ送られている（送出に失敗したら `error!` の上で目印を下ろし `NoContext`）。

##### State Management
```rust
/// 切替の予約（NonSend・在れば切替中）。
pub(crate) struct SwitchInFlight { pub target: SwitchTarget, pub prev: PrevGhost, pub stage: SwitchStage }
pub(crate) struct SwitchTarget { pub dir: PathBuf, pub folder: String, pub name: String, pub sakura_name: Option<String> }
pub(crate) struct PrevGhost { pub dir: PathBuf, pub name: Option<String>, pub sakura_name: Option<String> }
pub(crate) enum SwitchStage { SendOff, Welcoming { attempt: WelcomeAttempt } }
pub(crate) enum WelcomeAttempt { Target, Default }
```
- `request_ghost_switch`: ⑴ `SwitchInFlight` が在れば `warn!(event="ghost_switch_busy")`・`Busy`。⑵ `BootContext.root` で `catalog::list_ghosts` → `resolve_switch_target` → `None` なら `warn!(event="ghost_switch_unknown", name)`・`NotFound`（降ろさず・`OnGhostChanging` も送らない）。⑶ `sakura_name` は `catalog::sakura_name(&target.dir)` で切替先だけ読む（要件 8.7）。`prev` は `GhostSlot` の `GhostSession` の `mount().names` と `BootContext.current` から取る。⑷ 目印を立て、`KanadeMsg::ChangeGhost(ChangeRequest{target: ChangeTarget{sakura_name, name, dir: 絶対パス}, origin, raise_event})` を `GhostSession::kanade()` へ送る。`info!(event="ghost_switch_requested", from, to, raise_event, origin)`。
- `drain_change_requests`（`Input` 段の系）: `ChangeRx` が無ければ無操作。届いた要求ごとに `request_ghost_switch(world, SwitchRequest{ghost: Name(name), raise_event, origin: Automatic})`。
- `on_ghost_stopped(stopped)`: Flow 3 のとおり。`SendOff` ＋ `handoff: Some` → `switch_to(world, handoff)`。`SendOff` ＋ `handoff: None`（kanade が要求を受理しないまま止まった＝利用者の終了と競合）→ `warn!(event="ghost_switch_not_accepted", cause)`・目印を下ろし `quit_app(world, ExitOrigin::KanadeStopped(cause))`（今日どおり・`Fault` なら告知と終了コード 1）。`Welcoming{Target}` ＋ `Fault` → `error!(event="ghost_switch_target_fault", ghost, reason)` → `switch_to_default(world, fallen_name)`。`Welcoming{Default}` ＋ `Fault` → 目印を下ろし `quit_app(world, ExitOrigin::KanadeStopped(Fault))`（完了 `shiori-fault-notice` の告知と終了コード 1）。`Welcoming{..}` ＋ `Fault` 以外 → `info!(ghost_switch_target_quit)`・目印を下ろし `quit_app(world, ExitOrigin::KanadeStopped(cause))`。
- `switch_to(handoff)`（切替先を起こす）: ① `GhostSlot` から `GhostSession` を取り出し `shutdown(CloseReason::System)`（`Err` は `error!` の上で続ける＝戻す先は無い）・所要 ms を記録。①' （2026-09-27・要件 12.6）降ろし終えた直後（前のゴーストの記憶の書き手は join 済み＝動いている実行系は 0）に `write_switch_drop(app_profile_dir, &target.name)`＝`LastGhost`＝既定・印＝切替先を 1 回の書き込みで。`take_down` 自身は記憶に触れない（`switch_to_default` の降ろしでは印を切替先のまま残すため、書くのは `switch_to` の側）。② `close_windows_for_restart`。③ `resolve_balloon_for_ghost(root, &target.dir)`（`Err` は `error!`）。④ 窓の準備 `reopen_ghost_windows(world, &cfg, closed) -> PreparedWindows`（同期・descript の読取まで・窓はまだ作らない・`Err` は `error!`）。⑤ `boot_ghost_strict(world, inputs{boot_origin: ChangedFrom{prev.sakura_name, handoff.script, prev.name, prev.dir}}, &prepared.descript, &GhostDecision{route: Switched, dir, folder: Some}, &balloon)`（`Err` は `error!`）。③〜⑤ のどれかが失敗 → 窓は投函していないので何も生えない。切替先が既定（`folder == DEFAULT_GHOST_FOLDER`）なら `fatal(world, reason)`、そうでなければ `switch_to_default(world, target.name)`。⑥ 成功 → 窓の投函 `commit_ghost_windows(world, prepared)`・`GhostSlot` へ戻し、`BootContext.current` を切替先で更新、`stage = Welcoming{Target}`、`info!(event="ghost_switch_booted", ghost)`。
- `switch_to_default(fallen_name)`: 既定ゴーストが目録に無ければ `fatal`。在れば「切替先を降ろして」（`GhostSlot` に切替先が在れば `shutdown`・窓を閉じる。⑤ で失敗したときは `GhostSlot` が空で窓も 0 枚なので閉じるものは無い）から ③〜⑥ を `boot_origin: Halted{ghost_name: fallen_name}`・`GhostDecision{route: Default}` で行う。失敗 → `fatal`。成功 → `stage = Welcoming{Default}`。`OnGhostChanged` は送らない（`Halted` の根は無い）。告知は出さない（要件 6.3）。記憶には触れない（`LastGhost` は `switch_to` の降ろしで既に既定・印は切替先のまま＝戻しの途中や致命で落ちても次の起動は Ref7＝切替先・要件 12.7・12.8）。既定の `on_boot_ok`（経路 `Default`）は今日どおり起動の呼び出しが返った時点で `LastUsed`（既定）を書く。
- `fatal(reason)`: 目印を下ろし `quit_app(world, ExitOrigin::GhostFallbackFailed(ShioriFault{kind: Internal, reason}))`（`main` の後始末が今日の `Fault` の経路で告知し終了コード 1）。
- `on_notice(Steady)`: `stage == Welcoming{..}` → 目印を外し `info!(event="ghost_switch_done", ghost, attempt)`。2026-09-27（要件 12.6・12.7）: 目印を外す前に `record_steady_memory(world)`＝`GhostSlot` の `GhostSession::runtime()` の記憶の書き手で ⑴ `crate::record_last_used(runtime, &ctx.current.ghost, &ctx.current.balloon)`（`LastGhost`＝今のゴースト・Ghost スコープの `LastBalloon`／`LastShell`）と ⑵ `persist_put(App, [(LastRunning, 今のゴーストの名前)])` を投函する（段を問わず同じ手順・書き手の中で直列）。置き場か実行系か文脈が無ければ `warn!(event="steady_memory_not_recorded", reason)` で続ける（次の起動は `LastGhost`＝既定で起きる＝害は既定へ倒れるだけ）。目印が無ければ `debug!`（初回起動・LogSink の起動の定常到達）。`on_notice(ChangeCancelled{reason})`: 目印を外し `info!(event="ghost_switch_cancelled", reason)`。目印が無ければ `warn!`。

**Dependencies**
- Inbound: NoticePhase（`frame.rs`）・GhostFrame・`drain_change_requests`（P0）。
- Outbound: `ghost_session`（`GhostSlot`・`boot_ghost_strict`・`reopen_ghost_windows`・`GhostSession::shutdown`）・`app_exit`（`close_windows_for_restart`・`quit_app`）・`boot_config`（`BootContext`・`resolve_balloon_for_ghost`）・`catalog`（P0）。

**Implementation Notes**
- Integration: `run_ghost_quit_phase` は目印の有無だけを見て委譲する（要件 8.4）。切替の各段は `info!` のライフサイクル事象（`ghost_switch_requested`／`windows_closed_for_restart`（既存）／`ghost_switch_booted`／`ghost_switch_done`）。
- Validation: `ghost_switch_tests.rs`（実ゴースト無しで `SwitchInFlight` と偽の `GhostSlot` を組んで判断だけを見る）と `ghost_session_switch_tests.rs`（偽の SHIORI 2 体の 1 周）。
- Risks: `shutdown` の所要時間（helper の unload を含む）が 1 秒を超えると設計討議へ（別スレッド化の下地は `GhostSession: Send`）。

#### NoticePhase（`crates/areka/src/emo2_boot/frame.rs` の `run_ghost_quit_phase`）

- `KanadeNoticeRx(Receiver<KanadeNotice>)`。`run_ghost_quit_phase` は全件を `Vec` に取り出してから（受け口の借用を切って）順に捌く: `Stopped(s)` → `SwitchInFlight` が在れば `ghost_switch::on_ghost_stopped(world, s)`、無ければ今日どおり（最初の 1 件で `info!(ghost_quit)` → `quit_app(KanadeStopped(cause))`・2 件目以降は `debug!(ghost_quit_extra)`）。`Steady`／`ChangeCancelled` → `ghost_switch::on_notice`。返り値「通知を消化したか」は不変。
- 同じフレームで「切替の停止」と「切替先の起動」が起きるので、次のフレームの `emo2_frame_system` は新しい `Emo2Wiring` を見る。`GhostWindows` は `close_windows_for_restart` が外しているので、装着の相は新しい窓が spawn されるまでゲートで待つ（1 フレーム遅らせる仕掛けではなく、資源の有無という状態で決まる）。

#### GhostSession（`crates/areka/src/ghost_session.rs`）

##### Service Interface
```rust
/// 起こしたゴーストの置き場（NonSend・プロセスに 1 つ）。`main` が据え、切替が入れ替え、`main` が `run()` の後に取り出す。
pub(crate) struct GhostSlot(pub(crate) Option<GhostSession>);
/// 起こす材料の作り口（NonSend・プロセスに 1 つ）。切替が相手の構成入力から `GhostBootInputs` を組む。`main` は本番（helper の結線・停止通知の送り口の写し）を、試験は根のフォルダごとに偽の SHIORI を返すものを据える。
pub(crate) struct GhostBootInputsSource(pub(crate) Box<dyn Fn(&ConfigInputs, BootOrigin) -> GhostBootInputs>);
impl GhostSession { pub(crate) fn kanade(&self) -> Option<&Sender<KanadeMsg>>; pub(crate) fn names(&self) -> Option<&GhostNames>; }
/// 2026-09-27: 定常到達で切替先の記憶の書き手を借りる読み口（fallback の骨格で実行系が無ければ None）。
impl GhostSession { pub(crate) fn runtime(&self) -> Option<&areka_ghost::GhostRuntime>; }
/// 結線ありの腕（私有）。`wire_emo2_boot` が成立しなければ `Err(BootWiringFailed)`。
fn boot_wired(world, inputs, descript, ghost, balloon) -> Result<GhostSession, BootWiringFailed>;
/// 今日どおり（fallback へ倒れる・署名不変）。
pub(crate) fn boot_ghost(world, inputs, descript, ghost, balloon) -> GhostSession;
/// 切替用: fallback へ倒れず失敗を返す（要件 6.1）。
pub(crate) fn boot_ghost_strict(world, inputs, descript, ghost, balloon) -> Result<GhostSession, BootWiringFailed>;
```
- `boot_wired` の `wire_menu` の直後に `menu::ghost_frame::register(world)`（起こすたびに登記をやり直す＝要件 1.12）。`register_systems` に `ghost_switch::register_change_drain(world)` を足す（`Input` 段・説明書の取り出しと同じ位置）。
- `reopen_ghost_windows` の `#[cfg_attr(not(test), allow(dead_code))]` を外す。
- `GhostBootInputs::production(cfg, helper_exe, kanade_stop, boot_origin)` に由来を足す（`Emo2BootInputs.boot_origin`）。
- `switch_to`／`switch_to_default` の `inputs` は `GhostBootInputsSource` から組む（`ShioriWiring::Custom` は写せず、停止通知の送り口は `main` にしか無いため、作り口ごと World に置く）。無ければ `error!(ghost_switch_no_context)` で起動失敗と同じ扱い。

#### BootConfig／BootResolve（`crates/areka/src/boot_config.rs`・`boot_resolve.rs`）

```rust
/// 起動の文脈（Resource・プロセスに 1 つ）。目録・バルーンの解決・記憶の置き場・helper のパス・今のゴースト。
#[derive(Resource)] pub(crate) struct BootContext { pub root: BasewareRoot, pub app_profile_dir: PathBuf, pub helper_exe: PathBuf, pub current: CurrentGhost }
pub(crate) struct CurrentGhost { pub cfg: ConfigInputs, pub ghost: GhostDecision, pub balloon: BalloonDecision }
/// argv 無しの分岐（記憶 → 同梱 → 既定）で 1 ゴースト分のバルーンを解く（`resolve_boot_from` の後半を括り出す）。
pub(crate) fn resolve_balloon_for_ghost(root: &BasewareRoot, ghost_dir: &Path, pick: fn(usize) -> usize) -> Result<BalloonDecision, NoBalloon>;
/// 起動前の解決の戻りに「前回落ちたゴースト名」（＝起動中の印の値・読むだけで消さない）と根を足す。
pub(crate) type BootResolved = (ConfigInputs, GhostDecision, BalloonDecision, Option<String>, BasewareRoot);
/// boot_resolve.rs
pub(crate) enum GhostRoute { Argv, Memory, Only, Default, Random, Switched }   // Switched は記憶を書く経路
```
- 起動中の印の読み書きは下の SessionMark（2026-09-27 に `read_last_halted`／`take_last_halted`／`record_halt` と `PersistKey::LastHalted` を置き換えた）。`resolve_boot_from` は argv でゴーストを指定しない起動でだけ `read_session_mark` を呼び、`Some` なら `info!(event="session_mark_found", ghost)` を残して `resolve_ghost` へ `memory: None` を渡す（`LastGhost` を読まない＝段 3〜5 の唯一 → 既定 → 無作為で決まる）。`main` はその値を `BootOrigin::Halted{ghost_name}` として初回の `GhostBootInputs` へ渡す（`None` なら `Plain`・`first_boot_origin` は不変）。

#### SessionMark（`crates/areka/src/boot_resolve.rs`・2026-09-27・要件 12.1〜12.8）

| Field | Detail |
|-------|--------|
| Intent | 起動中の印（動いているゴーストの名前）を実 fs の App スコープへ同期で読み書きする |
| Requirements | 12.1〜12.8 |

```rust
/// 起動中の印（App `areka.last.running`）。空文字・無しは None。
pub(crate) fn read_session_mark(app_profile_dir: &Path) -> Option<String>;
/// 印を書く（初回の起動の前）。info!(session_mark_written)／書けなければ warn!(session_mark_write_degraded)。
pub(crate) fn write_session_mark(app_profile_dir: &Path, running: &str);
/// 切替で降ろした直後: LastGhost＝DEFAULT_GHOST_FOLDER と 印＝target を 1 回の save_scope で。
/// info!(switch_drop_recorded)／warn!(switch_drop_record_degraded)。
pub(crate) fn write_switch_drop(app_profile_dir: &Path, target: &str);
/// きれいな終わり: 印を空文字にする。info!(session_mark_cleared)／warn!(session_mark_clear_degraded)。
pub(crate) fn clear_session_mark(app_profile_dir: &Path);
/// 印に書く名前: 目録の同じフォルダの descript の name、無ければフォルダ名（argv 以外の決定だけが来る）。
pub(crate) fn running_name(root: &BasewareRoot, ghost: &GhostDecision) -> String;
```
- 実装は置き換え前の `save_app`／`read_last`（`save_scope`／`load_scope` の App 版）をそのまま使う。
- **不変条件（UI スレッドの直接書き込みの時点）**: `write_session_mark`・`write_switch_drop`・`clear_session_mark` を呼ぶのは、ゴーストの実行系が 1 つも動いていない間だけ（初回の起動の前・`switch_to` の降ろした直後・後始末と OS のセッションの終了で降ろした後）。実行系が動いている間の App スコープの書き込み（`on_boot_ok` の `LastUsed`・定常到達の `record_steady_memory`）はそのゴーストの記憶の書き手（sylphya）を通す。記憶の保存は「読んで重ねて書く」ので、UI スレッドと sylphya が同時に同じファイルを書くと片方が消えうるため。`GhostRuntime::shutdown` は sylphya の受信箱を処理し切ってから join するので、降ろした後の直接書き込みは先の投函の後に着く。
- `PersistKey::LastRunning`（`areka.last.running`・`[last] running`）。空文字は「無し」と読む（消去は空文字を書く＝既存の鍵と同じ）。

**きれいな終わりの判定**（`main.rs`・後始末と SessionEnd が共有）:
```rust
pub(crate) enum MarkVerdict { Clear, Keep(&'static str), Untouched }
/// first: 最初の終了の出所・ghost: 今のゴーストの決定・run_ok: run() の成否・down_ok: 降ろす処理の成否。
pub(crate) fn session_mark_verdict(first: Option<&ExitOrigin>, ghost: &GhostDecision, run_ok: bool, down_ok: bool) -> MarkVerdict;
```
| 条件（上から順に） | 判定 | 記録 |
|---|---|---|
| 今のゴーストの経路が `Argv` | `Untouched` | `debug!(session_mark_untouched_argv)` |
| 出所が無い（`FirstExit` 無し） | `Keep("no_exit_origin")` | `info!(session_mark_kept)` |
| `fault_of(出所)` が `Some`（SHIORI の失敗・既定へ戻せない致命） | `Keep("fault")`／`Keep("switch_fatal")` | 同上 |
| `run_ok` が偽 | `Keep("run_failed")` | 同上 |
| `down_ok` が偽 | `Keep("down_failed")` | 同上 |
| それ以外（`KanadeStopped` の `Fault` 以外・`Escape`・`Smoke`・`OsClose`・`SessionEnd`） | `Clear` | `clear_session_mark` の `info!(session_mark_cleared)` |

`ExitOrigin` の網羅の match で書く（出所を足すとコンパイルが判定の漏れを止める）。置き換え前の `should_record_halt` はこの判定に置き換える。

#### Main（`crates/areka/src/main.rs`）

- 据え付け: `register_systems` の後に `BootContext` と `GhostBootInputsSource`（`GhostBootInputs::production` を helper のパスと停止通知の送り口の写しで閉じたもの）を挿す。`boot_ghost` の戻りを `GhostSlot(Some(session))` として World へ挿す（ローカル変数には持たない）。
- `run()` の後: `GhostSlot` から取り出す（`None` なら降ろすものが無い）。告知の場面の `ghost_name`／`ghost_root` は `GhostSession::names()` と `BootContext.current.cfg.ghost_root` から組む（切替後の今のゴースト）。`fatal`（`GhostFallbackFailed`）で終わったときは起こそうとして失敗したのが既定ゴーストなので、`ghost_name`／`ghost_root` は既定ゴーストのフォルダ名と `root.ghost_dir(DEFAULT_GHOST_FOLDER)` で組む（`BootContext.current` は最後に起きた別のゴーストを指したままなので使わない）。`fault_of` が `Some` なら `AlertScene::ShioriFault`（`GhostFallbackFailed` も同じ場面）。
- 〔2026-09-27 裁定 13 で置き換え〕置き換え前は、単独起動の `Fault` のとき後始末で `record_halt` を呼んでいた（`should_record_halt`）。置き換え後は次のとおり。
- 起こす前の印（要件 12.1・12.5）: `boot_first_ghost` は `ghost.route != Argv` のとき、起こす前に `write_session_mark(ctx.app_profile_dir, &running_name(&ctx.root, ghost))`（ゴーストの実行系はまだ 1 つも無い）。argv の起動は印を書かない。
- `on_boot_ok` は位置永続の導管の挿入と、記憶を書く部分 `pub(crate) fn record_last_used(runtime, ghost, balloon)`（シェルのフォルダ名の取り出し＋`LastUsed::record`）に分け、経路が `Switched` のときは書かず `debug!(event="last_used_deferred")`（定常到達で `ghost_switch::record_steady_memory` が同じ関数を呼ぶ・要件 12.6）。それ以外の経路は今日どおり起動の呼び出しが返った時点で書く。
- きれいな終わりの消去（要件 12.2・12.3）: `after_run` は `session_mark_verdict(first, &ctx.current.ghost, run_ok, down_ok)` の材料（`first` と今のゴーストと記憶の置き場）を組み、後始末の閉包が `session.shutdown` の**後**に判定して `Clear` なら `clear_session_mark(app_profile_dir)`、`Keep(reason)` なら `info!(event="session_mark_kept", reason)`。`run_ok` は `finish_after_run` へ渡す前に `run.is_ok()` で控える。
- `after_run` は `SessionEnded`（下の SessionEnd）が World に在れば、告知の場面を組まず（`scene: None`）印の判定も行わない（OS のセッションの終了の中で済んでいる・`info!(event="session_end_already_handled")`）。終了コードは今日どおり `fault_of` で決める。

#### GhostFrame（`crates/areka/src/menu/ghost_frame.rs`）

- `pub(crate) fn register(world: &mut World)`: `menu::register(world, Frame::Ghost, Rc::new(ghost_frame_item))`。
- `ghost_frame_item(world, ctx) -> MenuItem`: `BootContext.root` で `catalog::list_ghosts` を読み（メニューを出すたびに読む・`sakura.name` は読まない）、`ItemBody::Submenu` に目録の並びのまま子項目（ラベル＝`identity.name` か無ければ `folder`・`checked: Some(現在のゴーストか)`）を並べる。0 体でも枠は出す（子 0 の `Submenu`・`enabled: false`）。`BootContext` が無ければ `enabled: false` で枠だけ出す（`trace!`）。
- 子項目の動作: `request_ghost_switch(world, SwitchRequest{ghost: Folder(folder), raise_event: true, origin: Manual})`。戻り値は記録だけ（`Busy`／`NotFound` は入口が `warn!` 済み）。

#### AppExit（`crates/areka/src/app_exit.rs`）

- `ExitOrigin::GhostFallbackFailed(ShioriFault)`。`fault_of`: `KanadeStopped(Fault(f)) | GhostFallbackFailed(f) => Some(f)`。
- 2026-09-27: `ExitOrigin::SessionEnd`（OS のシャットダウン・再起動・ログオフ・`fault_of` は `None`）。`attach_os_close_request` は同じ `Added<WindowHandle>` の窓へ `OnCloseRequest(on_ghost_os_close)` に並べて `OnSessionEnd(session_end::on_os_session_end)` を差す（系は増やさない）。

#### SessionEnd（`crates/areka/src/session_end.rs`・wintf の受け口・2026-09-27・要件 12.9〜12.11）

| Field | Detail |
|-------|--------|
| Intent | OS のセッションの終了（`WM_ENDSESSION` wParam＝TRUE）を、窓の手続きから戻る前にきれいな終わりとして済ませる |
| Requirements | 12.9, 12.10, 12.11 |

**wintf 側**（areka を知らない・`OnCloseRequest` と同じ形）:
```rust
/// ecs/window/components.rs: OS がセッションの終了を確定したとき（WM_ENDSESSION の wParam＝TRUE）に呼ぶ関数。
/// 関数は World 借用中に呼ばれ、戻った直後にプロセスが終了させられうる（後始末を済ませてから戻ること）。
#[derive(Component, Clone, Copy)] #[component(storage = "SparseSet")]
pub struct OnSessionEnd(pub fn(world: &mut World, entity: Entity));
```
- `ecs/window_proc/mod.rs` の `dispatch_window_message` に `WM_ENDSESSION => lifecycle::WM_ENDSESSION(..)` の腕を 1 つ足す。`WM_QUERYENDSESSION` の腕は足さない（`None`＝既定の手続きが TRUE を返す＝終了を拒まない）。
- `lifecycle::WM_ENDSESSION`: wParam＝0（取りやめ）→ `debug!(event="os_session_end_cancelled")`。`try_borrow_mut` に失敗（World が借用中＝モーダルの中などの再入）→ `warn!(event="os_session_end_world_busy")`（受け手を呼べない＝印が残り次の起動が Ref6/7 になる、と帰結を書く）。entity が破棄済み → `DESPAWNED_SKIP_TAG` の `debug!`。`OnSessionEnd` を持つ → `info!(event="os_session_end", entity, lparam)` の上で関数を呼ぶ。持たない → `debug!`。戻り値はどの腕も `Some(LRESULT(0))`（`WM_ENDSESSION` を処理したら 0 を返す）。

**areka 側**:
```rust
/// OS のセッションの終了を 1 回処理した印（Resource）。2 通目以降と `fn main` の後始末が見る。
#[derive(Resource)] pub(crate) struct SessionEnded;
/// OnSessionEnd に差す関数。
pub(crate) fn on_os_session_end(world: &mut World, _entity: Entity);
```
- 手順: ① `SessionEnded` が在れば `debug!(event="os_session_end_again")` で戻る。無ければ挿して `info!(event="os_session_end_begin")`。② `SwitchInFlight` が在れば外し `info!(event="ghost_switch_cancelled", reason="session_end")`（後から届く停止通知が切替として切替先を起こさないため）。③ `quit_app(world, ExitOrigin::SessionEnd)`（全窓を閉じ・最初の出所を残し・終了を指示。既に出所があれば今日どおり最初が勝つ）。④ `GhostSlot` から取り出して `shutdown(CloseReason::System)`（kanade の既存の `force_quit` が `OnClose` を Ref0＝`system` の NOTIFY で送り、SHIORI を降ろし、各アクターを join し、記憶を書き出す）。置き場が空なら `Ok`。失敗は `error!(event="os_session_end_down_failed")`。⑤ `session_mark_verdict(FirstExit, &ctx.current.ghost, true, down_ok)` → `Clear` なら `clear_session_mark`・`Keep` なら `info!(session_mark_kept)`。⑥ `info!(event="os_session_end_done", ms)`（①〜⑤の所要）。
- 告知（メッセージボックス）は出さない（出所が既に `Fault` でも。記録は `app_exit` と kanade の `error!` に在る）。
- Risks: ⑴ `shutdown` の join は UI スレッドを塞ぐ。切替の `take_down` が同じ処理を系の中で同期に行い実機で 1 ms（`signoff.md`）だったので同じ前提に立つ。helper のプロセスが OS から先に終了させられていれば、SHIORI の呼び出しは既存の送信の期限で失敗し（`Fault` の記録）、降ろす処理は続く。⑵ 既定の手続きの期限（OS が「終了を妨げている」画面を出すまでの数秒）を越える恐れは `os_session_end_done` の ms で実機サインオフで測る。⑶ 全窓が 0 枚の切替の一瞬に OS が終わると受け手が居ない（Flow 7 ⑹）。
- `close_windows_for_restart` は窓を消したあと `world.remove_resource::<GhostWindows>()` を行う（消えた窓の `Entity` を装着のゲートに見せない）。`quit_app` は変えない。`#[cfg_attr(not(test), allow(dead_code))]` を外す。

## Data Models

### Domain Model
- **切替の予約 `SwitchInFlight`**（UI・NonSend・高々 1 つ）: 目印そのもの。段 `SendOff` → `Welcoming{Target}` → （失敗なら）`Welcoming{Default}` → 消える。生成は `request_ghost_switch`、消去は `on_notice(Steady)`・`on_notice(ChangeCancelled)`・致命の直前・`Welcoming` での `Fault` 以外の停止。
- **kanade の帳簿 `State.change`／`pending_change`**: 受理で立ち、中止・取りやめで消える。`Unloading` に至った時点の `change` が停止通知の `handoff` になる。
- **起動の文脈 `BootContext`**（Resource）: 根・記憶の置き場・helper・今のゴースト（切替の成功で更新）。
- **起動の由来 `BootOrigin`**（kanade の設定）: `Plain`／`ChangedFrom`／`Halted`。根の表の入力。`Halted` の出どころは切替先の失敗（`switch_to_default`）と、次の起動で見つけた起動中の印（`first_boot_origin`）の 2 つ。
- **起動中の印**（記憶・App・2026-09-27）: 「きれいに終わっていないプロセスが動かしているゴーストの名前」。先に書き、きれいな終わりでだけ消える。**セッションの終了の済み印 `SessionEnded`**（Resource・プロセスに高々 1 つ）: OS のセッションの終了を処理した後に在る。
- 不変条件: `SwitchInFlight` が在る間、`quit_app` を呼ぶのは `fatal`・「`Welcoming` での `Fault` 以外の停止」・「`SendOff` での `handoff: None` の停止」の 3 つだけ。`SwitchInFlight` が無い間、停止通知は今日どおり終了へ。
- 不変条件（`handoff`）: kanade が `begin_change` を通れば停止通知の `handoff` は必ず `Some`（`raise_event` 無し・204・`Fault` で台本が無くても `Some(ChangeHandoff{script: None})`）。`None` は「切替の相を経ていない停止」だけを意味する。

### Logical Data Model（記憶）
| 鍵 | スコープ | 書く時 | 読む時 |
|---|---|---|---|
| `areka.last.ghost` | App | 初回の起動の `on_boot_ok`（argv 以外・起動の呼び出しが返った時点）・切替の降ろした直後（`write_switch_drop`＝既定）・切替先と戻しの定常到達（`record_steady_memory`）・既定への戻しの `on_boot_ok`（`Default`） | 次回の起動解決（印が在れば読まない） |
| `areka.last.running`（起動中の印・2026-09-27） | App | 初回の起動の前（`write_session_mark`・argv 以外）・切替の降ろした直後（`write_switch_drop`＝切替先）・定常到達（`record_steady_memory`＝今のゴースト）。消すのはきれいな終わり（`clear_session_mark`） | 次回の起動解決（`read_session_mark`・argv 以外） |
| `areka.last.balloon`／`areka.last.shell` | Ghost（切替先） | 初回は `on_boot_ok`・切替先は定常到達（`record_steady_memory`） | 切替先のバルーンの解決 |
| `areka.boot.count` | Ghost（切替先） | 初回起動の `first_boot_epilogue`（今日の規則） | `apply_boot_record_gate` |

### Data Contracts & Integration
- `KanadeMsg::ChangeGhost(ChangeRequest)`（UI → kanade・1 件・二重要求は UI が弾く）。
- `KanadeNotice`（kanade → UI・`mpsc`・FIFO・`Stopped` はプロセスの kanade 1 体につき高々 1 件）。
- `ChangeRequestRaw{name, raise_event}`（talk → UI・`mpsc`・ゴーストごとの線）。

## Error Handling

### Error Strategy
失敗は必ず記録してから着地する（無視は `warn!`・戻すは `error!`・中止は `info!`・各段は `info!`）。着地先は 4 つだけ: ⑴ 何もしない（該当なし・二重要求・許可表に無い）、⑵ 元の定常へ戻る（中止）、⑶ 既定ゴーストを起こす（切替先の失敗）、⑷ 今日の `Fault` の経路で終了（既定ゴーストの失敗）。

### Error Categories and Responses
| 場面 | 記録 | 着地 |
|---|---|---|
| 切替先の名前が目録に無い（`random` 等を含む） | `warn!(ghost_switch_unknown)` | 何もしない（降ろさず・送らず） |
| 切替中に新しい要求 | `warn!(ghost_switch_busy)` | 何もしない |
| `BootContext`／`GhostSlot` が無い・kanade への送出に失敗 | `warn!(ghost_switch_no_context)`／`error!(ghost_switch_send_failed)` | 目印を下ろして何もしない |
| kanade が `Steady` 以外か `pending_close` 中に `ChangeGhost` を受けた | `warn!(change_rejected_phase)` → UI `info!(ghost_switch_cancelled reason=rejected)` | 捨て、UI の目印を下ろす |
| `OnGhostChanging`／`OnClose` の台詞を利用者が中断 | `info!(change_cancelled reason=user_break)` → UI `info!(ghost_switch_cancelled)` | 元の定常へ |
| 切替の相に終了要求 | `info!(change_yield_to_close)` → UI `info!(ghost_switch_cancelled reason=close_request)` | 今日の終了の握手へ |
| 送り出しの台詞が 30 秒を超えた | `error!(change_deadline_exceeded)` | 降ろして切替を続ける |
| 握手中の SHIORI の `Fault` | kanade の既存の `error!`（`shiori_failed`／`shiori_down`） | 目印が立っているので切替を続ける |
| `SendOff` の目印の下に `handoff: None` の停止が届く（kanade が要求を受理しないまま止まった） | `warn!(ghost_switch_not_accepted)` | 目印を下ろし今日どおり終了（`Fault` なら告知・終了コード 1） |
| 保留の切替を持つ台本が `\-` の予約つきで中断された（`\-` に辿り着いたときも同じ） | `info!(change_dropped_by_quit)`＋`Notice(ChangeCancelled{CloseRequest})` → UI `info!(ghost_switch_cancelled reason=close_request)` | 保留を捨て今日の終了系列へ。通知は停止より先に届くので UI の目印は停止の前に下り、後の停止（`handoff: None`）は予約の無い今日の終了として捌く（`ghost_switch_not_accepted` の警告は出ない） |
| `GhostSession::shutdown` が `Err` | `error!`（既存） | 続ける（戻す先が無い） |
| 切替先のバルーンが解けない・窓を作れない・起動の結線が成立しない | `error!(ghost_switch_boot_failed stage=…)` | 既定へ（切替先が既定なら致命） |
| 切替先の SHIORI の `Fault`（非同期） | `error!(ghost_switch_target_fault)` | 既定へ |
| 既定ゴーストが目録に無い・既定の同期の失敗 | `error!(ghost_switch_fatal)` | `quit_app(GhostFallbackFailed)` → 告知・終了コード 1 |
| 既定ゴーストの SHIORI の `Fault` | kanade の既存の `error!` ＋ UI の `error!(ghost_switch_default_fault)`（切替先が既定ゴーストだった場合も） | `quit_app(KanadeStopped(Fault))` → 告知・終了コード 1 |
| 窓の準備を投函した後・窓が生える前に全窓を閉じた（切替先の非同期の失敗など） | `debug!(ghost_windows_stale)` | 古い閉包は窓を作らない（`app_exit::WindowsEpoch` の世代で判定・孤児の窓 0） |
| 切替先を起こす段で起動の文脈・起動入力の作り口が無い | `error!(ghost_switch_no_context)` ＋ `error!(ghost_switch_boot_failed, stage=no_context)` | 起動の失敗と同じ扱い（既定へ／致命） |
| 予約の無いときに切替の中止の通知が届いた | `warn!(ghost_switch_cancelled)` | 読み捨て |
| 迎え入れの段でないときに定常到達の通知が届いた（送り出し中の前のゴーストの遅れた通知など） | `debug!(ghost_switch_done)` | 予約を保つ |
| 汎用の入口: 許可表に無い／定常でない | `warn!(raise_event_not_allowed)`／`warn!(raise_event_not_steady)` | 捨てる |
| 通知の送出に失敗（受け口が消えた） | `warn!(notice_send_failed)` | 続ける |
| 印・切替の降ろした直後の記憶を書けない（`write_session_mark`／`write_switch_drop`／`clear_session_mark` の I/O 失敗・2026-09-27） | `warn!(session_mark_write_degraded／switch_drop_record_degraded／session_mark_clear_degraded)`（帰結を文に書く: 次の起動が Ref6/7 付きになる／ならない） | 続ける |
| 起動前の解決で印が見つかった | `info!(session_mark_found)` | `LastGhost` を読まず既定へ・Ref6/7 |
| きれいに終わらなかった（出所が失敗・`run()` の失敗・降ろす失敗・出所なし） | `info!(session_mark_kept, reason)` | 印を残す |
| 定常到達の記憶を書く相手が無い（置き場・実行系・文脈が無い） | `warn!(steady_memory_not_recorded, reason)` | 続ける（`LastGhost` は既定のまま） |
| `WM_ENDSESSION` の wParam＝FALSE | `debug!(os_session_end_cancelled)`（wintf） | 何もしない |
| `WM_ENDSESSION` が World の借用中に届いた | `warn!(os_session_end_world_busy)`（wintf） | 受け手を呼べない（印が残る） |
| セッションの終了の 2 通目以降 | `debug!(os_session_end_again)` | 読み捨て |
| セッションの終了で降ろす処理が失敗 | `error!(os_session_end_down_failed)` | 印を残して戻る |

### Monitoring
実機サインオフは `RUST_LOG` を `kanade=info,areka=info`（判定の分岐の水準）に加えて `GhostRuntime::shutdown` の段ごとの記録（`areka_ghost` の runtime）の水準まで開け（`ghost_switch_down_ms` が 1 秒を超えたとき、kanade・dispatcher・SHIORI（helper の unload）・relay・ticker・sylphya のどの join が支配したかを切り分けるため）、`ghost_switch_requested`／`change_*`／`windows_closed_for_restart`／`ghost_switch_booted`／`ghost_switch_done`／`ghost_switch_cancelled`／`ghost_switch_target_fault`／`app_exit` の件数と順序、`ghost_switch_down_ms` の値を `signoff.md` に残す。2026-09-27 の追加走行（要件 12.13）は `session_mark_*`・`switch_drop_recorded`・`last_used_recorded`・`os_session_end*`・`app_exit origin=SessionEnd` と、kanade の `force_quit`・`OnClose` の送出（`shiori_request` の trace＝Reference0 が `system`）の水準まで開ける。

## Testing Strategy

判断の分岐だけを固定し、既に確かめられている配線（登録と載せ替え・終了の握手・seriko の join・窓の配置・告知の文面）は再テストしない（要件 10.10）。既存テストは 1 本も削除しない（字面で形を固定しているものは新しい字面へ追随させる）。判定は集めてから 1 回。

### Unit Tests（kanade の純粋な `step`・`schedule/change_tests.rs`／`events_change_tests.rs`）
- 受理: `Steady{None}` ＋ `ChangeGhost{raise_event}` → `OnGhostChanging` GET（Ref0〜3 の突き合わせ・`manual`／`automatic`）＋ `ChangePending`。`Steady{Some}` → `pending_change` に控え Action 0。`ChangePending`／`BootMain` 等で受けたら `warn!` 1 件・状態不変（要件 2.1・11.7・7.5）。
- 204 → `OnClose`（`Ref0=system`）→ 台本あり → `TalkDone{Ended}`／`TalkDone{Quit}`／期限超過のいずれも `Unloading{Quit|DeadlineExceeded}`・`handoff` に台本なし（要件 2.3〜2.5・10.2。`Interrupted` は `\-` の予約があっても中止＝要件 5.1・5.4）。`OnGhostChanging` の台本が `\-` で終わる（`TalkDone{Quit}`）でも `Unloading{Quit}` ＋ `handoff.script == Some`。
- `raise_event` 無し: `Steady{None}` → 即 `Unloading{CloseSilent}`・`OnGhostChanging` 0・`OnClose` 0。`Steady{Some}` → `pending_change` → `TalkDone{Ended}` と `TalkDone{Interrupted}` の両方で `Unloading{CloseSilent}`（要件 2.6・5.5・10.3）。
- 中止: `ChangeTalkWait` ＋ `TalkDone{Interrupted, quit_reserved: true}` → `Steady{None}` ＋ `Notice(ChangeCancelled{UserBreak})`・`Unloading` へ進まない・`state.change == None`。`ChangeCloseTalkWait` も同じ。中止のあと `CloseRequest` → `begin_close`（`OnClose` GET）が今日どおり出る（要件 5.1・5.4・5.6・10.4）。
- 終了要求: `ChangeTalkWait` ＋ `CloseRequest` → `Steady{Some}`・`pending_close`・`Notice(ChangeCancelled{CloseRequest})`、続く `TalkDone{Ended}` で `OnClose` GET が 1 件。`ChangeClosePending`／`ChangeCloseTalkWait` ＋ `CloseRequest` → `ClosePending`／`CloseTalkWait` へ・`OnClose` の追加送出 0（要件 2.9）。
- 根の表: `first_boot` 真 ＋ `ChangedFrom` → `OnFirstBoot`・`OnGhostChanged` 0。`first_boot` 偽 ＋ `ChangedFrom` → `OnGhostChanged`（Ref0〜3・Ref4〜6 空・Ref7）→ 204 → `OnBoot`／Value → `OnBoot` 0。`Halted` → `OnBoot` の Ref6＝`halt`・Ref7＝名前・Ref1〜5 空。`Plain` → `OnBoot` は Ref0 だけ（要件 4.1〜4.4・6.2・10.13）。
- 定常到達: `BootVersion` ＋ `Notified` → `Action::Notice(Steady)` 1 件（要件 3.5）。
- 汎用の入口: 許可表に無い → `warn!`・Action 0。`Steady` 以外 → `warn!`・Action 0。`Steady` → GET／NOTIFY が渡した Reference のまま。`value_replaces_active_talk("OnSecondChange") == false`・マウス 2 語と `OnGhostChanging` は真（要件 7.2〜7.4・7.6・10.8）。
- Actor: `Stopped` に `handoff` が載る・`Notice` が `stop_sink` へ流れる（`actor_stop_notify_tests.rs` に 2 本）。

### Unit Tests（areka の判断・実ゴースト無し）
- `ghost_switch_tests.rs`: `resolve_switch_target`（`name` 一致 → フォルダ名一致 → 該当なし・大文字小文字・現在のゴーストを除外しない）。`request_ghost_switch`: 二重要求 `Busy` ＋ `warn!` 1 件・該当なし `NotFound` ＋ `warn!` 1 件 ＋ kanade の受信端は空・受理で `SwitchInFlight{SendOff}` ＋ `ChangeGhost` 1 件。`on_notice(ChangeCancelled)` で目印が消え `info!` 1 件。`on_ghost_stopped(Welcoming{Default}, Fault)` → `AppExit` 要求 ＋ `FirstExit == KanadeStopped(Fault)`（要件 1.5〜1.9・5.2・6.4・10.6）。
- `frame_ghost_quit_switch_tests.rs`: `SwitchInFlight` が在る World に `Stopped{handoff: Some}` を積む → `quit_app` 0 回（`AppExit` 未要求）・委譲先の記録 1 件。目印を外して同じ通知 → `AppExit` 要求。`SwitchInFlight{SendOff}` ＋ `Stopped{handoff: None}` → `AppExit` 要求 ＋ `warn!` 1 件 ＋ 目印が消えている（要件 2.9・3.3・3.5・8.4・10.7）。
- `ghost_session_switch_tests.rs`（同じ土台）: 切替先の起動が同期で失敗（`ShioriWiring::Custom(|| Err)` を結線の失敗に見立てる）→ 次の `Input` 段を 1 回回したあと `GhostWindowMarker` の窓が既定ゴーストの分だけ（孤児 0）・`GhostWindows` が 1 つ（要件 6.1・4.10）。
- `change_tests.rs`: `Steady{Some}` ＋ `pending_change` ＋ `TalkDone{Interrupted, quit_reserved: true}` → 終了系列（`begin_close`）へ進み `pending_change == None`・`OnGhostChanging` 0 件（要件 5.5 の但し書き）。
- `ghost_frame_tests.rs`: 2 体の目録で `Submenu` の子 2 つ・並び・ラベル（`name` 無しはフォルダ名）・現在のゴーストに `checked`・子を選ぶと `SwitchRequest{Folder, raise_event: true, Manual}` が入口へ届く・`boot_wired` の 2 周目で登記が新品（`registered_frames` に `Ghost`）（要件 1.4・1.11・1.12・10.9）。
- `app_exit_tests.rs`: `close_windows_for_restart` の後 `GhostWindows` が無い・`fault_of(GhostFallbackFailed)` が `Some`。
- 〔2026-09-27 裁定 13 で置き換え〕`boot_resolve_tests.rs`／`main_halt_record_tests.rs` の `record_halt`・`take_last_halted`・`should_record_halt` の固定は、下の要件 12 のテストへ書き換える（置き換え無しに消さない）。
- `boot_resolve_tests.rs`（要件 12.1・12.6）: 印の往復（書く → 読む → 消す＝空文字は無し）・`write_switch_drop` の 1 回で `LastGhost == emo2` と印がそろう・書けない置き場（App の置き場が普通のファイル）でそれぞれ `warn!` 1 件で戻る・`running_name` が `name` → フォルダ名の順。
- `main_session_mark_tests.rs`（`main_halt_record_tests.rs` を改名して書き換え・要件 12.2〜12.5・12.12 ⑴〜⑶）: `session_mark_verdict` の表（出所ごと・`run_ok`／`down_ok` の偽・`Argv`・出所なし・`SessionEnd` は `Clear`）。偽の SHIORI の土台で A を初回の起動として起こし（`boot_first_ghost`）→ 印＝A。⑴ 起動系列の途中の `Fault`・⑵ 定常のあとの `Fault`・⑶ 後始末を通さずに捨てた（強制終了に見立てる）の 3 通りのあと、同じ根で `resolve_boot_from` → 既定が選ばれ 4 つ目が A の名前（`LastGhost` は A のままでも読まれない）→ `first_boot_origin` が `Halted{A}` → 起こすと印＝emo2 の名前。既定ゴースト自身の `Fault` → 次も既定で Ref7＝既定の名前。メニューの「終了」相当（`KanadeStopped(Quit)`）の後始末 → 印が消え、次の起動は `LastGhost` どおりで由来 `Plain`。`Argv` の起動 → 印を読まず書かず消さない（前の印が残る）。
- `ghost_session_switch_memory_tests.rs`（`SwitchRig`・要件 12.6〜12.8・12.12 ⑷⑸）: A → B（`raise_event`）で、降ろした直後（`Welcoming{Target}` の間）に `read_last_ghost == emo2`・印＝B・B の Ghost スコープに `LastBalloon` 無し → B の `Steady` の処理のあと `LastGhost == B`・印＝B・B の `LastBalloon`／`LastShell` あり。B が接続に失敗 → 戻しの間は印＝B → emo2 の `Steady` のあと印＝emo2・`LastGhost == emo2`。根に emo2 が無い＋B の失敗（致命）→ `LastGhost == emo2`・印＝B。判定は集めてから 1 回（記憶は sylphya の書き手の投函なので、読む前に置き場の実行系の記憶の書き手へ `barrier` を掛けるか有界の待ちで読む）。
- `session_end_tests.rs`（`SwitchRig`・要件 12.9〜12.11・12.12 ⑹）: A を起こして印を書き定常まで回す → `on_os_session_end(world, 任意の entity)` → A の呼出列の最後の方に `OnClose`（NOTIFY・Ref0＝`system`）が 1 件・`FirstExit == SessionEnd`・終了の指示あり・`GhostSlot` が空・印が消えている・`SessionEnded` が在る。2 回目の呼び出し → 呼出列が増えず `debug!(os_session_end_again)` 1 件。`SwitchInFlight` を立てた World → 予約が消える。先に `quit_app(KanadeStopped(Fault))` を指示した World → 印が残る（`Keep("fault")`）。`after_run` は `SessionEnded` が在れば告知の場面を組まない。
- wintf `ecs/window_proc/lifecycle_tests.rs`（要件 12.10・12.12 ⑹）: `OnSessionEnd` を持つ entity へ `WM_ENDSESSION`（wParam＝1）→ 関数が 1 回呼ばれ `Some(LRESULT(0))`／wParam＝0 → 呼ばれない／部品なし・破棄済みの entity → 呼ばれず panic 無し／World を借用中 → 呼ばれず `Some(LRESULT(0))`。配送表の `WM_QUERYENDSESSION` は `None`（既定の手続き）。
- `app_exit_tests.rs`: `fault_of(SessionEnd) == None`・`attach_os_close_request` の後にゴースト窓が `OnSessionEnd` も持つ。
- `change_cue_tests.rs`: `("change","ghost","B")` → 要求 1 件（`raise_event=false`）・`--option=raise-event` → 真・`("change","shell",…)` → 0 件（担当外）・名前なし → `warn!`（要件 1.2・1.3・1.10・8.5）。
- `consumer_ledger` の件数の固定を 9 へ。

### Integration Tests（偽の SHIORI 2 体・同じ World・`ghost_session_switch_tests.rs`）
- 土台: `ghost_switch_test_support.rs` が一時の根を組む（`ghost/A`・`ghost/B`・`ghost/emo2` ＝ emo2 検体の複製・`balloon/emo2-kakukaku`）。B と emo2 には起動記録（`ghost/master/profile/areka/sylphya.toml` の `[boot] count`）を先に書く。`World::new()` ＋ `Schedules` ＋ `AppExit` ＋ `register_systems` ＋ `BootContext`（`ghost_session_restart_tests.rs` の形）。偽の SHIORI は `SpineHarness::standard_backend` の型（`ScriptedShioriBackend::builder()`）で 1 体ずつ台本を組み、`ShioriWiring::Custom` で渡す（`GhostBootInputsSource` を試験用に据え、`cfg.ghost_root` のフォルダ名で B／emo2 の台本を選ぶ）。フレームは回さず、`run_ghost_quit_phase(world)` と `drain_change_requests(world)` を有界に回す（`spin_wait_until`＋`run_bounded`）。
- 1 周（要件 10.1・10.13）: A（`OnBoot` 台本）→ `request_ghost_switch(Name(B), raise_event: true, Automatic)` → A の `OnGhostChanging`（台本あり）→ 再生完了 → `Stopped` → B（`OnGhostChanged` 204 → `OnBoot`）→ `Steady`。判定（集めて 1 回）: ⑴ A・B の呼出列と Reference（`RecordedCall`）、⑵ `AppExit` 未要求、⑶ 系の数が 1 周目と同じ、⑷ `ReadmeWiring` が B の根の下・`MenuWiring` の登記に `Ghost`・`UserBreakWiring` と `KanadeNoticeRx` が生きた送出端につながっている、⑸ `SwitchInFlight` が消えている。
- 同じ土台で: 起動記録の無い B → `OnFirstBoot`・`OnGhostChanged` 0 件（要件 4.4・10.13）。A → A（自分自身・要件 1.8）。B の SHIORI が接続に失敗（`Custom(|| Err)`）→ emo2 が `OnBoot` の Ref6＝`halt`・Ref7＝B の名前で起き `OnGhostChanged` 0 件・`alert::raise` は呼ばれない（要件 6.1・6.2・10.5）。根に emo2 が無い ＋ B の失敗 → `AppExit` 要求 ＋ `FirstExit == GhostFallbackFailed`（要件 6.4・10.5）。

### E2E（実機・`signoff.md`・要件 10.11・10.12）
① emo2 → メニュー「ゴースト」→ R_POST_and_KOMAINU（交代の台詞 → 「○○から交代」）。② R_POST → emo2（往復）。③ ① の台詞の途中でダブルクリック → 中止・emo2 が残る。④ R_POST → SHIORI が失敗するゴースト（`shiori-host32-testdll-loadu`）→ emo2 が Ref6/7 つきで起きる（ログで判定）。⑤ ① のあと終了 → 再起動で R_POST。各走行のコマンド・終了コード・目印の事象の件数・`ghost_switch_down_ms` を記録する。1 スコープのゴーストへの切替が落ちないこと（要件 4.9）は ④ の失敗するゴーストを 1 人にして兼ねる。
2026-09-27 の追加（要件 12.13・同じ記憶の置き場・argv なしの起動）: ⑥ R_POST を起動 → エージェントが自分で起こした areka の PID に限って `taskkill /F /PID` → 次の起動が emo2 で `OnBoot` の Ref6＝`halt`・Ref7＝R_POST の名前・`session_mark_found` 1 件。⑦ （時間の窓が取れれば）R_POST → fail-one の切替の迎え入れの途中（`ghost_switch_booted` の記録を見てから `ghost_switch_target_fault` より前）で強制終了 → 次の起動の Ref7＝fail-one・記憶の `LastGhost` が fail-one でない（`[last]` の中身を記録）。窓が取れなければ ④ の走行の記憶の時系列（`switch_drop_recorded` → `last_used_recorded` の順と値）で代える。⑧ 定常の R_POST の PID のトップレベル窓を列挙し（`EnumWindows`＋`GetWindowThreadProcessId`）、各窓へ `SendMessageTimeout` で `WM_QUERYENDSESSION`（lParam＝0）→ 全窓 TRUE を確かめてから `WM_ENDSESSION`（wParam＝TRUE・lParam＝0）を送る（OS が各トップレベル窓へ送るのと同じ形）→ `os_session_end_begin` 1 件・`os_session_end_again` が窓の数−1 件・`OnClose` の Reference0＝`system` が 1 件・`session_mark_cleared` 1 件・`os_session_end_done` の ms を記録 → 次の起動で Ref6/7 が付かず記憶どおりのゴーストで起きる。⑨ メニューの「終了」で閉じた次の起動で Ref6/7 が付かない。

## Performance & Scalability
- 目標: 降ろし始めから切替先の窓が出るまで 1 秒以内（要件 3.8）。UI スレッドを塞ぐのは `GhostSession::shutdown`（kanade → dispatcher → shiori（helper の unload）→ relay → ticker → sylphya の join → seriko の join）。`ghost_switch_down_ms` を実機で測る。超えたら設計討議へ（別スレッド化の下地: `GhostSession: Send`・フレームをまたぐ状態は `SwitchStage` に `Descending` を足す形）。
- 目録の読み取り（`list_ghosts`）はメニューを出すたびと要求ごと（fs I/O・数体なら無視できる）。`sakura_name` は切替先だけ 1 回。

## Supporting References
- 選ばなかった案と根拠: `research.md` §9（B-1〜B-8 の決定）・§10（危険と対策）・§11（設計の簡素化）。
- 正典の引用: `requirements.md` の「正典（ukadoc）の位置づけ」と「起動と終了の根の木」。
- 完了 spec の契約: `areka-P0-ghost-restart-unit`（登録 1 回・載せ替え n 回）・`areka-P0-shiori-fault-notice`（`Fault` の告知）・`areka-P0-balloon-break`（中断の受理）・`areka-P0-baseware-root-layout`（目録と記憶）。
