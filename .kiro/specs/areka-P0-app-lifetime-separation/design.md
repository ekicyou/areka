# Design Document: areka-P0-app-lifetime-separation

> 本文のコードの事実は **2026-09-23・本ブランチ**（`claude/areka-p0-app-lifetime-c96473`・main `637799d7` 相当）で実ファイルを読んで確かめたもの。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。
> 要件 6 の裁定 1・2（2026-09-23 開発者確定）と、要件ディスカッションで縛った要件 1.3 は確定事項として扱い、ここでは開き直さない。`research.md` §6 の設計判断項目 1・2・3・5・6・7・8・10・11 は本文書で決めた（§8 に一覧）。

## Overview

**Purpose**: areka のプロセスの寿命を「窓の数」から「明示の終了の指示」へ切り離す。窓が 0 枚の瞬間があってもプロセスは生き続け、終了の指示 1 本で今日と同じ後始末を経て終了コード 0 で終わる。これは後続のゴースト切替（古い窓を全部閉じてから新しい窓を開く）が乗る土台である。

**Users**: 切替を実装する開発者（後続 spec `areka-P0-ghost-shell-balloon-switch`）。areka の利用者から見える終わり方は今までと区別が付かない。

**Impact**: 変わるのは 3 点。
1. **wintf**: アプリの構築時に「窓 0 で終了する／しない」を選べる口（`ExitPolicy`）と、明示の終了を指示する口（`AppExit::request_exit`）が増える。既定は従来どおり「窓 0 で終了」で、`WinApp::new()` の意味は変わらない（example 14 本は 1 文字も触らない）。
2. **areka**: 終了操作 6 種の合流点 4 か所が、それぞれ「全窓を閉じて終了を指示する」1 つの操作 `quit_app` を 1 行で呼ぶ形になる。全窓を閉じるだけの操作は外から呼べなくなる。
3. **`run()` の戻り方**: 明示の終了の指示を受けたとき、まだ画面に残っている窓を `run()` が戻る前に壊す。後始末①〜④の間に利用者の画面へ窓が残らない（要件 1.3）。

### Goals
- 終了の指示が無ければ、窓が 0 枚でもメッセージループから戻らない（1.1）。
- 終了の指示で必ずメッセージループから戻り、今日と同じ順序の後始末と終了コード 0 で終わる（1.2・3.8）。
- 「終了のための全窓破棄」と「終了の指示」を片方だけ呼べない形にする（3.6・裁定 2）。
- 「窓 0 でも生きている」「終了の指示で終わる」をそれぞれ決定論テストで固定する（5.1・5.2）。
- wintf の既定を変えず、example と既存の決定論テストを 1 本も落とさない（2.2・4.1・4.4）。

### Non-Goals
- 切替そのもの・「窓を全部閉じるが終了しない」操作（後続 spec の仕事。本仕様はその操作を**作らない**）。
- `KanadeStopCause` への値の追加、ダミー窓の経路の撤去、トレイアイコン等の「窓が無いときの入口」、複数ゴースト。
- 時間で自動終了する安全網（切替中の「窓 0 で生きている」を壊すため持たない・裁定 2）。
- `ecs/app.rs` の `"[App] Last window closed."` の文言（§8 判断 8）。

## Boundary Commitments

### This Spec Owns
- wintf の終了規律の**入口**: `ExitPolicy`（構築時の選択・実行中に変えられない）、`AppExit`（World に据える NonSend の受け口・「指示済み」の記憶・終了シグナル）、`WinApp::with_exit_policy`、`WinApp::run` が戻る直前の残存窓の破棄。
- areka の**終了の統合操作**: `crates/areka/src/app_exit.rs` の `quit_app(world, origin)` と出所の語彙 `ExitOrigin`。4 か所の呼び手をこの 1 行に書き換えること。
- 全窓を閉じるだけの既存 2 関数（`crates/areka/src/placement/spawn.rs` の `despawn_ghost_windows`・`crates/areka/src/main.rs` の `despawn_smoke_targets`）を削除し、`quit_app` の私有部品へ吸収すること（片方だけ呼べない形＝裁定 2）。
- 決定論テスト 4 本（wintf）と、既存テストの追随（受け口の挿入・置き場の移動）。
- 常設 smoke テストが緑のまま「窓が無いのにプロセスが残る」を赤にする役を保つこと（5.4）。
- 実機確認の手順（5.5・5.6）。

### Out of Boundary
- 登録表の空遷移の検知（`crates/wintf/src/runtime/window_registry.rs` の `reconcile_window_registry`・`WindowRegistry`）。**変更 0 行**——「空遷移でフックを鳴らす」仕掛けはそのまま、フックを仕込むか否かだけを構築時に選ぶ。
- 窓が消える経路（`world.despawn` → `on_window_handle_remove` の `WM_CLOSE` 投函 → `reconcile_window_registry` の drop → `DestroyWindow`）。変更 0 行。
- kanade の終了の握手（`crates/areka-kanade/src/schedule/close.rs`・`user_break.rs`）とその決定論テスト。変更 0 行（4.2）。
- `crates/areka-ghost/src/runtime.rs` の `fn shutdown`。変更 0 行（4.3）。
- `fn main` の後始末①〜④（`crates/areka/src/main.rs`）。順序・終了コードは不変（1.2・3.8）。
- `crates/wintf/src/ecs/app.rs` の `App::on_window_destroyed`（旧経路の窓数カウンタ・終了を駆動しない）。変更 0 行。
- example 14 本（wintf 7・areka 5・areka-emo-text 2）。変更 0 行（4.1）。

### Allowed Dependencies
- wintf 内: `runtime/message_loop.rs`（`AppExit`・`ShutdownPolicy`）← `runtime/mod.rs`（`WinApp`・`ExitPolicy`）。`ecs/` から `runtime/` への上向き依存は作らない（既存の方針）。
- areka: `app_exit.rs` → `wintf::AppExit`・`areka_kanade::KanadeStopCause`・`crate::placement::spawn::GhostWindowMarker`・`crate::placement::diag::DESPAWNED_SKIP_TAG`・`crate::DummyWindowMarker`。呼び手（`emo2_boot/frame.rs`・`input_events/mod.rs`・`main.rs`）→ `app_exit.rs`。**`placement` は `app_exit` に依存しない**（`placement` は `crate::` パスを持てない・example の `#[path]` include のため）。
- 新しい crate: 0。新しい外部依存: 0。`Cargo.toml` の変更: 0（`event-listener` 5.4.2・`wintf-winmsg-executor` =0.0.5・`bevy_ecs` 0.19 はいずれも既存）。

### Revalidation Triggers
- `AppExit`／`ExitPolicy` の形の変更（後続のゴースト切替は `ExitPolicy::Explicit` の上に「窓を全部閉じて開き直す」を組む）。
- `quit_app` の署名や `ExitOrigin` の値の増減（`origin` の語は実機ログの検索語）。
- `WinApp::run` が戻る直前の残存窓の扱いの変更（後始末①〜④の間に窓が残るかどうか）。
- `crates/areka/src/main.rs` の共有（並走 spec `areka-P0-baseware-root-layout`）: 本仕様が `main.rs` から終了関連を出すので、相手の作業面積と行数の圧力は下がる。相手は本仕様の着地後に settled main へ再突合する。

## Architecture

### Existing Architecture Analysis

- **終了の決め手は wintf に 1 か所**。`crates/wintf/src/runtime/mod.rs` の `WinApp::new` が `fn wire_shutdown_hook` で `WindowRegistry` の空遷移フックに `Event::notify` を仕込み、`fn run` は `MessageLoopDriver::block_on(ShutdownPolicy::shutdown_future(...))` の完了を待つだけである。`WinApp` は `world` と `shutdown: Rc<event_listener::Event>` の 2 欄で、構築時の設定値の器は無い。
- **`shutdown_future` は通知を記憶しない**。`crates/wintf/src/runtime/message_loop.rs` の `ShutdownPolicy::shutdown_future` は `listen()` を立ててから `await` する。`event-listener` 5.4.2 はリスナ不在の通知を失う（同ファイル `notify_shutdown` の doc が明記）。ゆえに「指示済み」を覚える 1 ビットが要る（1.4・1.5）。
- **`block_on` は完了済みの future でも正常に戻る**（同ファイルの `block_on_ready_future_returns_value` が実証）。投入済みの `spawn_local` タスクも `block_on` の中で駆動される（`MessageLoopDriver::block_on` の doc）。
- **`WinApp` を持つのは `fn main` と `wire_emo2_boot` だけ**。areka の終了 4 か所はどれも `&mut World` しか持たない。wintf には `fn wire_click_through` が World へ挿す NonSend `ClickThroughRegistryHandle`（`crates/wintf/src/ecs/clickthrough/controller.rs`）という「World 経由で利用側へ渡す」先例がある。
- **ウィンドウ手続きは World を `try_borrow` で借り、借用中なら読み飛ばす**（`crates/wintf/src/runtime/wndproc_bridge.rs` の `make_wndproc` 手順 3）。`reconcile_window_registry` は tick の中（World 借用中）で `Window<WndState>` を drop して `DestroyWindow` しているので、**World を借りたまま窓を壊しても再入で壊れない**ことは今日の経路が既に踏んでいる。
- **全窓破棄は 2 関数に分かれている**: `crates/areka/src/placement/spawn.rs` の `despawn_ghost_windows`（`GhostWindowMarker` のみ・呼び手は `run_ghost_quit_phase` と強制退避）と `crates/areka/src/main.rs` の `despawn_smoke_targets`（`DummyWindowMarker`＋`GhostWindowMarker`・呼び手は smoke クロージャ）。`on_dummy_pressed` は `DummyWindowMarker` を自前で despawn する。

### Architecture Pattern & Boundary Map

```mermaid
graph TB
    subgraph wintf_runtime
        WinApp[WinApp with_exit_policy run]
        AppExit[AppExit requested bit and signal]
        Registry[WindowRegistry empty transition hook]
        Loop[MessageLoopDriver block_on]
    end
    subgraph areka
        Quit[app_exit quit_app]
        Frame[emo2_boot frame run_ghost_quit_phase]
        Escape[input_events on_char_pointer_pressed]
        Dummy[main on_dummy_pressed]
        Smoke[main smoke closure]
        Main[main teardown 1 to 4]
    end
    WinApp -->|inserts NonSend| AppExit
    WinApp -->|OnLastWindowClose only| Registry
    Registry -->|hook request_exit| AppExit
    AppExit -->|shutdown_future completes| Loop
    Loop -->|returns then destroy remaining windows| WinApp
    WinApp -->|run returns| Main
    Frame --> Quit
    Escape --> Quit
    Dummy --> Quit
    Smoke --> Quit
    Quit -->|despawn all app windows then request_exit| AppExit
```

**Architecture Integration**:
- Selected pattern: **既存ファイルの中で拡張（wintf）＋統合操作の新モジュール 1 本（areka）**（`research.md` §4 案 C）。wintf は 3 つの定義（`ExitPolicy`・`AppExit`・`with_exit_policy`）を既存テストの隣に足す。areka は「終了とは何か」を 1 か所へ集める。
- Domain boundaries: wintf は「いつメッセージループを終えるか」だけを決め、areka の語彙（`ExitOrigin`）を知らない。areka は「どの窓を閉じ、どこから終了が来たか」だけを知り、ループの終え方を知らない。
- Existing patterns preserved: NonSend の受け口を `WinApp` が World へ据える（`ClickThroughRegistryHandle` と同型）。`listen → await` の通知取りこぼし防止規律（`AsyncTickTask` と同形）に「arm → 指示済みの確認 → await」を足す。
- New components rationale: `AppExit` は「指示済み」の記憶と終了シグナルを 1 つにまとめ、既定ポリシーのフックも同じ `request_exit` を通す＝**終了の完了機構が 1 本**になる。`quit_app` は裁定 2（片方だけ呼べない形）を構造で守る唯一の置き場。
- Steering compliance: ライフサイクル事象は `info!`、2 回目以降の指示や正常系の打ち切りは `debug!`、配線の誤りは `error!`＋記録（`logging.md`・記録無しの失敗経路を作らない）。1 ファイル 1,000 行の目安（`main.rs` は減る）。

### Technology Stack

| Layer | Choice / Version | Role in Feature | Notes |
|-------|------------------|-----------------|-------|
| Runtime（wintf） | `event-listener` 5.4.2（既存）＋ `std::cell::Cell<bool>` | 終了シグナルと「指示済み」の記憶 | 新規依存 0 |
| Runtime（wintf） | `wintf-winmsg-executor` =0.0.5（既存） | `block_on`／`spawn_local` | 完了済み future で正常に戻る（既存テストが実証） |
| ECS | `bevy_ecs` 0.19（既存） | `AppExit` を NonSend リソースとして World へ据える | `Rc` を含むので自動的に `!Send` |

## File Structure Plan

### Directory Structure
```
crates/wintf/src/runtime/
├── message_loop.rs        # 変更: AppExit（新）・ShutdownPolicy::shutdown_future の引数と判断・テスト 3 本（新）
└── mod.rs                 # 変更: ExitPolicy（新）・WinApp::with_exit_policy（新）・new() の委譲・wire_shutdown_hook の分岐・run() の残存窓破棄・テスト 1 本（新）＋既存 3 行の追随

crates/areka/src/
├── app_exit.rs            # 新規: ExitOrigin・quit_app・私有 despawn_app_windows
├── app_exit_tests.rs      # 新規: main_seam_tests.rs から移す 3 本（本文は不変・関数名だけ追随）
├── main.rs                # 変更: mod app_exit・with_exit_policy(Explicit)・on_dummy_pressed と smoke クロージャの 1 行化・despawn_smoke_targets の削除・doc の追随
├── main_seam_tests.rs     # 変更: despawn_smoke_targets_* 3 本を app_exit_tests.rs へ移す
├── main_startup_window_tests.rs  # 変更: ダミー窓ダブルクリックの検査へ受け口を挿す
├── emo2_boot/frame.rs     # 変更: run_ghost_quit_phase が quit_app を呼ぶ・import と doc の追随
├── emo2_boot/frame_ghost_quit_tests.rs  # 変更: World 構築ヘルパへ受け口を挿す・終了の指示の確認 2 行
├── input_events/mod.rs    # 変更: 強制退避の腕が quit_app を呼ぶ・import の追随
├── input_events/input_events_tests.rs   # 変更: World 構築ヘルパと素の World の 2 本へ受け口を挿す
└── placement/spawn.rs     # 変更: despawn_ghost_windows の削除（呼び手 0）・モジュール doc の追随

crates/areka/tests/
└── smoke_boot_loop_exit.rs  # 変更なし（doc コメント 1 行の追随のみ・判定と目印は不変）
```

### Modified Files
- `crates/wintf/src/runtime/message_loop.rs` — `AppExit` を `ShutdownPolicy` の隣に置く（終了規律の状態そのもの）。`shutdown_future(exit: AppExit)` は「arm → 指示済みなら即完了 → await」。
- `crates/wintf/src/runtime/mod.rs` — `pub enum ExitPolicy`、`pub use message_loop::AppExit`、`WinApp { world, exit: AppExit }`（`shutdown: Rc<Event>` を置き換え）、`with_exit_policy`、`new()` の委譲、`wire_shutdown_hook` の分岐、`run()` の残存窓破棄。既存テストの `app.shutdown.listen()` 3 か所を `app.exit.signal().listen()` へ（意味は不変）。
- `crates/areka/src/app_exit.rs`（新規） — 統合操作。
- `crates/areka/src/main.rs` — `WinApp::with_exit_policy(ExitPolicy::Explicit)?`（**段取りの最後に入れる 1 行**）。`on_dummy_pressed`・smoke クロージャは `quit_app` を 1 行で呼ぶ。`despawn_smoke_targets` は削除。行数は 958 から減る（4.5）。
- `crates/areka/src/emo2_boot/frame.rs` — `run_ghost_quit_phase` の `despawn_ghost_windows(world)` を `quit_app(world, ExitOrigin::KanadeStopped(stopped.cause))` へ。`ghost_quit`／`ghost_quit_no_windows` の記録は据え置き。
- `crates/areka/src/input_events/mod.rs` — 強制退避の腕の `despawn_ghost_windows(world)` を `quit_app(world, ExitOrigin::Escape)` へ。
- `crates/areka/src/placement/spawn.rs` — `despawn_ghost_windows` を削除（`app_exit` の私有部品へ吸収・外から呼べる全窓破棄を残さない）。モジュール doc の該当行を「全窓を閉じて終了を指示する操作は `app_exit::quit_app` が持つ」へ。干渉台帳（A1）の登記外だが他 spec との重なりは 0（完了時に台帳へ追記）。
- `crates/areka/src/placement/spawn_cleanup_tests.rs` — doc コメント 1 行（`despawn_smoke_targets` の名を `app_exit::quit_app` へ）。テスト本文は不変。
- 変更しないと明記するもの: `crates/wintf/src/runtime/window_registry.rs`・`crates/wintf/src/ecs/app.rs`・`crates/wintf/src/lib.rs`（`pub use runtime::*` で `ExitPolicy`／`AppExit` は自動で公開される）・kanade の `schedule/*`・`crates/areka-ghost/src/runtime.rs`・example 14 本。

## System Flows

### Flow 1: 終了系列の完了通知からの終了（tick の中で指示が出る）

```mermaid
sequenceDiagram
    participant Tick as AsyncTickTask Update
    participant Phase as run_ghost_quit_phase
    participant Quit as quit_app
    participant Exit as AppExit
    participant Fin as FrameFinalize reconcile
    participant Loop as block_on
    participant Run as WinApp run
    participant Main as main teardown
    Tick->>Phase: KanadeStopped
    Phase->>Quit: quit_app KanadeStopped cause
    Quit->>Quit: despawn all app windows
    Quit->>Exit: request_exit
    Exit->>Exit: requested = true, notify
    Tick->>Fin: same tick
    Fin->>Fin: drop Window handles, DestroyWindow
    Note over Fin: registry empty, Explicit means no hook
    Loop->>Loop: poll shutdown_future, complete
    Loop->>Run: return
    Run->>Run: destroy remaining windows, none left
    Run->>Main: Ok
    Main->>Main: steps 1 to 4, exit 0
```

### Flow 2: smoke の自動終了（tick の外で指示が出る・要件 1.3 の効く経路）

```mermaid
sequenceDiagram
    participant Task as smoke spawn_local task
    participant Quit as quit_app
    participant Exit as AppExit
    participant Loop as block_on
    participant Run as WinApp run
    participant Reg as WindowRegistry
    participant Main as main teardown
    Task->>Quit: quit_app Smoke
    Quit->>Quit: despawn all app windows, WM_CLOSE posted
    Quit->>Exit: request_exit
    Loop->>Loop: poll shutdown_future, complete
    Note over Loop,Reg: reconcile may not have run yet
    Loop->>Run: return
    Run->>Reg: remove_non_send and drop
    Reg->>Reg: DestroyWindow for each remaining window
    Run->>Main: Ok
    Main->>Main: steps 1 to 4, exit 0
```

**Flow-level decisions**:
- 指示が tick の中で出ても外で出ても、`run()` が戻る時点で画面に窓は無い。tick の中なら同じ tick の `FrameFinalize` が壊し、外なら `run()` の残存窓破棄が壊す。どちらの経路も「1 フレーム待つ」解を使わない。
- `run()` の残存窓破棄は World を借りたまま行う（`reconcile_window_registry` と同じ条件）。ウィンドウ手続きは `try_borrow` の失敗で読み飛ばすので再入で壊れない。`DestroyWindow` はその窓宛ての未処理メッセージ（投函済み `WM_CLOSE`）を捨てるので、後から届く経路も無い。
- 既定ポリシー（example）の経路: 空遷移フック → `request_exit` → `shutdown_future` 完了。今日と同じ順序で、記録が `info` 1 行増えるだけ。

## Requirements Traceability

| Requirement | Summary | Components | Interfaces | Flows |
|-------------|---------|------------|------------|-------|
| 1.1 | 指示なしの窓 0 で終わらない | ExitPolicy::Explicit・wire_shutdown_hook | `WinApp::with_exit_policy` | — |
| 1.2 | 指示で戻り後始末は同順・exit 0 | AppExit・WinApp::run・main（不変） | `AppExit::request_exit` | Flow 1・2 |
| 1.3 | 残った窓を閉じてから戻る | WinApp::run の残存窓破棄 | `run()` | Flow 2 |
| 1.4 | 二重の指示で失敗しない | AppExit（指示済みビット） | `request_exit` 2 回目は `debug!` | — |
| 1.5 | ループ開始前の指示を取りこぼさない | ShutdownPolicy::shutdown_future | arm → 指示済み確認 → await | — |
| 2.1 | 構築時の選択口 1 つ | ExitPolicy・with_exit_policy | `WinApp::with_exit_policy(policy)` | — |
| 2.2 | 既定は従来どおり | `WinApp::new()` の委譲 | `new() = with_exit_policy(OnLastWindowClose)` | — |
| 2.3 | 既定で空遷移→終了（example 不変） | wire_shutdown_hook（フックあり） | 既存の `close_to_reconcile_to_shutdown_chain_wakes_listener` | — |
| 2.4 | 選べば空遷移でも続き、後で窓を開ける | wire_shutdown_hook（フックなし） | 新テスト（5.1） | — |
| 2.5 | 明示の指示の口 1 本 | AppExit | `request_exit(&self)` | — |
| 2.6 | 選択にかかわらず終わる | AppExit は両ポリシーで World に在る | `shutdown_future` | Flow 1・2 |
| 2.7 | 受けたことを info で記録 | AppExit | `info!("[AppExit] exit requested")` | — |
| 2.8 | 実行中の切替口を持たない | ExitPolicy は構築時に消費・setter 無し | — | — |
| 3.1 | 終了系列の完了で全窓＋指示 | run_ghost_quit_phase → quit_app | `ExitOrigin::KanadeStopped(cause)` | Flow 1 |
| 3.2 | 窓 0 でも指示 | quit_app（閉じた数に依らず指示） | 既存テストへの確認 1 行 | — |
| 3.3 | 強制退避で全窓＋指示 | on_char_pointer_pressed → quit_app | `ExitOrigin::Escape` | — |
| 3.4 | ダミー窓で閉じて指示 | on_dummy_pressed → quit_app | `ExitOrigin::DummyWindow` | — |
| 3.5 | smoke で閉じて指示 | smoke クロージャ → quit_app | `ExitOrigin::Smoke` | Flow 2 |
| 3.6 | 全窓破棄と指示を 1 操作に | quit_app（`despawn_app_windows` は私有） | `quit_app(world, origin) -> usize` | — |
| 3.7 | 出所を info で記録 | quit_app | `info!(event="app_exit", origin, closed)` | — |
| 3.8 | 後始末の順序と exit 0 は不変 | main（不変） | — | Flow 1・2 |
| 4.1 | example を触らない | `new()` の意味不変 | — | — |
| 4.2 | kanade の握手を触らない | 変更対象外（Out of Boundary） | — | — |
| 4.3 | `fn shutdown` を触らない | 変更対象外（Out of Boundary） | — | — |
| 4.4 | 既存テストを落とさない | 既存テストの追随（§Testing Strategy） | 受け口の挿入・置き場の移動 | — |
| 4.5 | `main.rs` 1,000 行以内 | `despawn_smoke_targets` の削除・1 行化 | — | — |
| 5.1 | 窓 0 で終わらない決定論テスト | 新テスト（`runtime/mod.rs`） | 実 HWND＋実 reconcile・`wait_timeout` | — |
| 5.2 | 指示で終わる決定論テスト | 新テスト（`message_loop.rs`） | `spawn_local` から指示 → `block_on` が戻る | — |
| 5.3 | 判断分岐だけを足す | §Testing Strategy の 4 本に限る | — | — |
| 5.4 | smoke テスト緑・見張りの役を保つ | `smoke_boot_loop_exit.rs`（判定不変） | — | Flow 2 |
| 5.5 | 実機確認の手順 | §Testing Strategy「実機確認」 | `AREKA_APP_SMOKE_EXIT_MS`・`RUST_LOG` | — |
| 5.6 | 自分のプロセスだけ止める | 同上（`Start-Process -PassThru` の PID） | — | — |
| 6.1 | 裁定 1（構築時の設定値＋指示 1 本） | ExitPolicy・AppExit | — | — |
| 6.2 | 裁定 2（1 操作・安全網なし） | quit_app・タイマー無し | — | — |
| 6.3 | 覆す必要が出たら議題へ | 本設計は覆さない（§8） | — | — |

## Components and Interfaces

| Component | Domain/Layer | Intent | Req Coverage | Key Dependencies (P0/P1) | Contracts |
|-----------|--------------|--------|--------------|--------------------------|-----------|
| AppExit | wintf runtime | 終了の指示の受け口・指示済みの記憶・終了シグナル | 1.4, 1.5, 2.5, 2.6, 2.7 | event-listener (P0) | Service, State |
| ExitPolicy / WinApp::with_exit_policy | wintf runtime | 構築時に「窓 0 で終了するか」を選ぶ | 1.1, 2.1, 2.2, 2.3, 2.4, 2.8, 6.1 | WindowRegistry hook (P0) | Service |
| WinApp::run 残存窓破棄 | wintf runtime | 指示で戻る前に残った窓を壊す | 1.2, 1.3 | WindowRegistry (P0), wndproc try_borrow (P1) | Service |
| ShutdownPolicy::shutdown_future | wintf runtime | 指示済みなら即完了・そうでなければ通知を待つ | 1.5, 5.2 | AppExit (P0) | Service |
| quit_app / ExitOrigin | areka | 全窓破棄と終了の指示を 1 操作に | 3.1–3.7, 6.2 | AppExit (P0), markers (P0) | Service |
| 呼び手 4 か所 | areka | 各終了操作を `quit_app` 1 行へ | 3.1, 3.3, 3.4, 3.5 | quit_app (P0) | — |

### wintf runtime

#### AppExit

| Field | Detail |
|-------|--------|
| Intent | 明示の終了の指示を受け、指示済みを覚え、終了シグナルを鳴らす |
| Requirements | 1.4, 1.5, 2.5, 2.6, 2.7 |

**Responsibilities & Constraints**
- `crates/wintf/src/runtime/message_loop.rs` に置く（`ShutdownPolicy` の状態そのもの）。`runtime/mod.rs` が `pub use message_loop::AppExit` で公開し、`lib.rs` の `pub use runtime::*` で `wintf::AppExit` になる。
- `Clone` 可（中身は `Rc` の共有）。`Rc` を含むので `!Send`＝bevy が NonSend として扱う。`WinApp` が 1 つ持ち、その clone を World へ挿す（`ClickThroughRegistryHandle` と同型）。
- **指示は 1 度だけ効く**。2 回目以降は `debug!` で流し、通知も撃たない（撃っても冪等だが、記録で 1 回目と区別する）。
- `pub fn new()` は `WinApp` と、素の `World` で走る areka の既存テストが受け口を据えるために使う（本番の意味論を曲げない headless の器）。

**Dependencies**
- Outbound: `event_listener::Event` — 終了シグナル (P0)
- Inbound: `WinApp`（構築・World 挿入・`run()` の待ち）、`WindowRegistry` の空遷移フック（既定ポリシー）、areka `quit_app` (P0)

**Contracts**: Service [x] / API [ ] / Event [ ] / Batch [ ] / State [x]

##### Service Interface
```rust
/// 明示の終了の指示の受け口（NonSend リソース・Clone は共有）。
#[derive(Clone)]
pub struct AppExit { requested: Rc<Cell<bool>>, signal: Rc<event_listener::Event> }

impl AppExit {
    pub fn new() -> Self;
    /// 終了を指示する。1 回目: 指示済みを立て info! の上で notify(usize::MAX)。2 回目以降: debug! のみ。
    pub fn request_exit(&self);
    /// 指示済みか。
    pub fn is_requested(&self) -> bool;
    /// 終了シグナル（`run()` の防御的 notify と wintf 内テストが使う）。
    pub(crate) fn signal(&self) -> &event_listener::Event;
}
```
- Preconditions: なし（どのスレッドでも作れるが、World へ挿すのは UI スレッド）。
- Postconditions: `request_exit` 後は `is_requested() == true` が恒久に成り立つ。戻す口は無い。
- Invariants: 記録は 1 回目が `info!("[AppExit] exit requested")`、2 回目以降が `debug!("[AppExit] exit already requested — ignored")`。

##### State Management
- State model: `requested: bool`（false → true の一方向）。`Event` は通知の運び手で状態を持たない。
- Persistence & consistency: プロセス内のみ。`Rc` 共有なので `WinApp`・World・呼び手のどこから見ても同じ 1 ビット。
- Concurrency strategy: UI スレッド単独（`!Send`）。

#### ExitPolicy と WinApp::with_exit_policy

| Field | Detail |
|-------|--------|
| Intent | 構築時に「窓の数が 0 になったら終了するか」を選び、実行中は変えられない |
| Requirements | 1.1, 2.1, 2.2, 2.3, 2.4, 2.8, 6.1 |

**Responsibilities & Constraints**
- `crates/wintf/src/runtime/mod.rs`。`ExitPolicy` は `OnLastWindowClose`（既定）と `Explicit` の 2 値。**フィールドに保持しない**——`wire_shutdown_hook` がフックを仕込むか否かを決めるだけで、以後は参照されない（2.8 を「読める場所が無い」形で守る）。
- `WinApp::new()` は `Self::with_exit_policy(ExitPolicy::OnLastWindowClose)` へ委譲する。署名は不変（example 14 本は触らない）。
- `wire_shutdown_hook(world, exit, policy)`: ⑴ `ProdWindowRegistry` が無ければ挿す（従来どおり）、⑵ `exit.clone()` を World の NonSend として挿す（**両ポリシーで**・2.6）、⑶ `OnLastWindowClose` のときだけ `set_shutdown_hook(move || exit.request_exit())` を仕込む。`Explicit` のとき `reconcile_window_registry` は空遷移でフック `None` を見て何もしない（既存の分岐・変更 0 行）。登録表は空でも `insert` を受けるので「後で窓を開ける」は自然に成り立つ（2.4）。
- 既定ポリシーのフックが `Event::notify` を直接撃つ今の形は捨て、`request_exit` を通す＝終了の完了機構が 1 本になる。副作用は「最後の窓が閉じたとき `info` が 1 行増える」だけ。

**Contracts**: Service [x]

##### Service Interface
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitPolicy { OnLastWindowClose, Explicit }

pub struct WinApp { world: Rc<RefCell<EcsWorld>>, exit: AppExit }

impl WinApp {
    pub fn new() -> Result<Self>;                              // = with_exit_policy(OnLastWindowClose)
    pub fn with_exit_policy(policy: ExitPolicy) -> Result<Self>;
    fn wire_shutdown_hook(world: &Rc<RefCell<EcsWorld>>, exit: &AppExit, policy: ExitPolicy);
}
```
- Preconditions: `with_exit_policy` は `new()` と同じ（COM/DPI 初期化・UI スレッドの登録）。
- Postconditions: World に `ProdWindowRegistry` と `AppExit` が在る。`OnLastWindowClose` なら登録表にフックが在り、`Explicit` なら無い。
- Invariants: ポリシーを後から変える口は無い。

#### WinApp::run の残存窓破棄

| Field | Detail |
|-------|--------|
| Intent | 明示の指示で戻るとき、まだ登録表に残っている窓を戻る前に壊す |
| Requirements | 1.2, 1.3 |

**Responsibilities & Constraints**
- `run()` の手順 4（`block_on(ShutdownPolicy::shutdown_future(self.exit.clone()))`）の直後に置く。World を `borrow_mut` し、`remove_non_send::<ProdWindowRegistry>()` で登録表ごと取り出して drop する。`Window<WndState>` の drop が `DestroyWindow` を呼ぶ。空でなければ `info!("[WinApp::run] exit requested while windows remained open — destroying them before returning")` を 1 行残す。
- 新しい API は足さない（`WindowRegistry` は変更 0 行）。`run()` を 2 度呼ぶ運用は今も想定外（`wire_new_path` の doc）なので、登録表を取り去って戻って差し支えない。`WinApp` の drop は今までどおり World を drop するだけになる。
- World を借りたまま壊すのは `reconcile_window_registry` と同じ条件。ウィンドウ手続きは `try_borrow` の失敗で読み飛ばす（`wndproc_bridge.rs`）。
- 手順 5 の防御的 notify は `ShutdownPolicy::notify_shutdown(self.exit.signal())` として据え置く（意味は変えない）。

**Contracts**: Service [x]（`run()` の署名は不変 `pub fn run(&self) -> Result<()>`）

#### ShutdownPolicy::shutdown_future

| Field | Detail |
|-------|--------|
| Intent | 指示済みなら即完了、そうでなければ通知を待つ future を返す |
| Requirements | 1.5, 5.2 |

##### Service Interface
```rust
impl ShutdownPolicy {
    /// arm → 指示済みの確認 → await。arm と確認の順で、確認とほぼ同時に届く指示も取りこぼさない。
    pub(crate) fn shutdown_future(exit: AppExit) -> impl Future<Output = ()>;
    pub(crate) fn notify_shutdown(event: &Event);   // 不変
}
```
- Preconditions: なし。
- Postconditions: `exit.is_requested()` が真になった後は必ず完了する（先に真でも、後で真でも）。
- Invariants: `MessageLoopDriver::block_on` は完了済みの future で正常に戻る（既存テスト）ので、ループ開始前の指示は「ループ開始後ただちに終わる」になる。

### areka

#### quit_app と ExitOrigin

| Field | Detail |
|-------|--------|
| Intent | 「終了のために全窓を閉じる」と「終了を指示する」を 1 つの操作にし、片方だけ呼べない形にする |
| Requirements | 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 6.2 |

**Responsibilities & Constraints**
- `crates/areka/src/app_exit.rs`（新規・`main.rs` から `mod app_exit;`）。`quit_app` は ⑴ 私有の `despawn_app_windows`（`Or<(With<DummyWindowMarker>, With<GhostWindowMarker>)>`・今日の `despawn_smoke_targets` の本文そのまま＝連鎖破棄済みの標的を `DESPAWNED_SKIP_TAG` の `debug!` で打ち切り残りを処理し切る）を呼び、⑵ World から `AppExit` を取り、`info!(event = "app_exit", origin = ?origin, closed, "[quit_app] 全窓を閉じ、終了を指示した")` の上で `request_exit()` を呼ぶ。閉じた数が 0 でも指示する（3.2・分岐を置かない）。
- 受け口が World に無いとき（本番では `WinApp` が必ず挿すので配線の誤り）: `error!(event = "app_exit_unwired", origin = ?origin, closed, "[quit_app] 終了の受け口が World に無い——窓は閉じたが終了を指示できない")` を残して戻る。記録無しの失敗経路を作らない。素の `World` で走る既存テストはこの経路を踏まないよう受け口を挿す（§Testing Strategy）。
- `despawn_ghost_windows`（`placement/spawn.rs`）と `despawn_smoke_targets`（`main.rs`）は削除し、この私有部品へ吸収する。**全窓を閉じるだけの操作はクレート内のどこからも呼べない**（3.6 を構造で守る）。後続のゴースト切替が要る「閉じるが終了しない」操作は本仕様で作らない（Out of scope）。後続は `ExitPolicy::Explicit` の上に自分の操作を足す。
- `ExitOrigin::KanadeStopped(cause)` は既存の `KanadeStopCause` をそのまま包む。`Debug` 出力は `KanadeStopped(Quit)` のように `ghost_quit` の `cause` と同じ語になる（記録の語彙を揃える・3.7）。

**Dependencies**
- Outbound: `wintf::AppExit` (P0)、`crate::placement::spawn::GhostWindowMarker`・`crate::DummyWindowMarker`・`crate::placement::diag::DESPAWNED_SKIP_TAG` (P0)、`areka_kanade::KanadeStopCause` (P0)
- Inbound: `emo2_boot::frame::run_ghost_quit_phase`・`input_events::on_char_pointer_pressed`・`main::on_dummy_pressed`・`main::open_startup_window` の smoke クロージャ (P0)

**Contracts**: Service [x]

##### Service Interface
```rust
/// どの終了操作から来たか（記録の語彙・受け手は分岐しない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExitOrigin {
    KanadeStopped(areka_kanade::KanadeStopCause),  // メニューの終了・別れの台詞のあと・中断のあと
    Escape,                                        // Ctrl+Shift+左ダブルクリック
    DummyWindow,                                   // ダミー窓の左ダブルクリック
    Smoke,                                         // AREKA_APP_SMOKE_EXIT_MS
}

/// 全窓（ダミー窓＋ゴースト窓）を閉じ、終了を指示する。戻り値は標的として拾った窓の数。
pub(crate) fn quit_app(world: &mut World, origin: ExitOrigin) -> usize;

fn despawn_app_windows(world: &mut World) -> usize;   // 私有
```
- Preconditions: `&mut World`。受け口の有無は問わない（無ければ `error!`）。
- Postconditions: `DummyWindowMarker`／`GhostWindowMarker` を持つ entity は 0。受け口が在れば `is_requested() == true`。
- Invariants: 記録は areka が `app_exit` 1 行、wintf が `[AppExit]` 1 行（層ごとに 1 行・wintf は areka の語彙を知らない）。

#### 呼び手 4 か所（summary-only）

| 場所 | 変更 |
|---|---|
| `crates/areka/src/emo2_boot/frame.rs` の `run_ghost_quit_phase` | `let closed = quit_app(world, ExitOrigin::KanadeStopped(stopped.cause));`。`ghost_quit`（info）と `ghost_quit_no_windows`（debug）は据え置き。doc の「窓が 0 になると wintf の `run()` が戻り」を「終了の指示で `run()` が戻り」へ |
| `crates/areka/src/input_events/mod.rs` の `on_char_pointer_pressed` 強制退避の腕 | `quit_app(world, ExitOrigin::Escape);`。`mouse_escape_close`（info）は据え置き |
| `crates/areka/src/main.rs` の `on_dummy_pressed` | 自前の query＋despawn ループを `app_exit::quit_app(world, ExitOrigin::DummyWindow);` へ。ダミー窓検出の info は据え置き |
| `crates/areka/src/main.rs` の `open_startup_window` 内 smoke クロージャ | `let count = app_exit::quit_app(w, ExitOrigin::Smoke);`。`smoke 自動 close` の info は据え置き（smoke テストの目印ではない） |
| `crates/areka/src/main.rs` の `fn main` | `let app = WinApp::with_exit_policy(ExitPolicy::Explicit)?;`。`app.run()?` の注記を「終了の指示で戻る」へ。後始末①〜④は変更 0 行 |

**Implementation Notes**
- Integration: 段取りは ⑴ wintf（`AppExit`・`ExitPolicy`・`with_exit_policy`・`run()` の残存窓破棄・決定論テスト 4 本）→ ⑵ areka `app_exit.rs`＋4 か所の書き換え＋既存テストの追随 → ⑶ **`main` の `with_exit_policy(Explicit)` の 1 行を最後に入れる**（それまでは既定ポリシーのまま。`quit_app` の指示が先に立ち、空遷移フックの `request_exit` は 2 回目として `debug!` で流れる＝各段で smoke テストが従来どおり緑であることを確かめられる）→ ⑷ smoke テスト緑の確認と実機確認。
- Validation: 各段で `cargo test -p wintf` と `cargo test -p areka`（`tests/smoke_boot_loop_exit.rs` を含む）。
- Risks: `run()` の残存窓破棄は `DestroyWindow` を World 借用中に呼ぶ。今日の `reconcile_window_registry` と同じ条件なので新しい再入は無い。実機確認の smoke 経路（Flow 2）で「windows remained open」の行が出ることを見る（出なければ reconcile が先に走っただけで、どちらも正常）。

## Data Models

### Domain Model
- **終了の指示（`AppExit`）**: 一方向の 1 ビット `requested` と終了シグナル。集約の根は `WinApp`。World の NonSend は同じ実体の別名（`Rc` 共有）。
- **終了の出所（`ExitOrigin`）**: 記録のための値。受け手は分岐しない。
- 不変条件: `requested` は false → true のみ。`ExitPolicy` は構築時に消費され、状態として残らない。

## Error Handling

### Error Strategy
- **配線の誤り（受け口不在）**: `quit_app` は `error!(event = "app_exit_unwired")` を残し、窓は閉じたまま戻る。プロセスは残る（既定ポリシーなら空遷移フックで終わる）。本番では `WinApp::with_exit_policy` が必ず挿すので起き得ず、起きたら配線の欠陥として赤くする。
- **二重の指示**: `AppExit::request_exit` の 2 回目は `debug!` で流す（1.4）。失敗にしない。
- **閉じる窓が無い**: 正常系。`quit_app` は指示を出し、`run_ghost_quit_phase` は従来どおり `ghost_quit_no_windows` を `debug!` で残す（3.2）。
- **連鎖破棄済みの標的**: 従来どおり `DESPAWNED_SKIP_TAG` の `debug!` で打ち切り、残りを処理し切る。
- **`run()` が戻る時点で窓が残っている**: 失敗ではなく想定内（Flow 2）。`info` 1 行の上で壊す。

### Monitoring
- 実機ログの検索語: `event="app_exit"`（areka・`origin`・`closed` 付き）、`[AppExit] exit requested`（wintf）、`windows remained open`（wintf・残存窓破棄）、`event="app_exit_unwired"`（出てはならない）。

## Testing Strategy

足すのは判断分岐 2 つ（5.1・5.2）と縁の条件 2 つ（1.4・1.5）の計 4 本に限る（5.3）。既に確かめられている配線（閉じる → 登録表から外れる → 合図が鳴る）は再テストしない。

### 新設の決定論テスト（4 本）

| # | 置き場 | 名前（案） | 何を固定するか | 壊れると赤になる判断 |
|---|---|---|---|---|
| 1 | `crates/wintf/src/runtime/mod.rs` `tests` | `explicit_policy_keeps_the_loop_alive_when_the_last_window_closes` | `with_exit_policy(Explicit)` で実 HWND を 1 枚作り（既存 `close_to_reconcile_to_shutdown_chain_wakes_listener` と同じ構築）、`Window` を外して実 `reconcile_window_registry` を回す → 登録表は空・`exit.signal().listen().wait_timeout(20ms)` は `None`・`is_requested()` は偽。続けてもう 1 枚を factory で作り `WindowHandle` が付くこと（2.4「後で窓を開ける」） | `wire_shutdown_hook` が `Explicit` でもフックを仕込む（5.1・1.1・2.4） |
| 2 | `crates/wintf/src/runtime/message_loop.rs` `tests` | `exit_requested_from_a_ui_task_ends_the_loop` | `AppExit::new()` の clone を `spawn_local(async move { exit.request_exit() })` で投入し、`MessageLoopDriver::block_on(ShutdownPolicy::shutdown_future(exit))` が戻る | 指示が future を完了させない（5.2・2.6） |
| 3 | 同上 | `exit_requested_before_the_loop_starts_completes_immediately` | `request_exit()` を先に呼んでから `block_on(shutdown_future(exit))` が戻る | arm → 確認の順が無く、リスナ不在の通知を失う（1.5） |
| 4 | 同上 | `second_request_is_ignored_and_the_future_still_completes` | `request_exit()` を 2 回呼んでも panic せず `is_requested()` は真のまま・`block_on(shutdown_future(exit))` が戻る | 2 回目で失敗や二重処理を起こす（1.4） |

- 1 の対照は既存の `close_to_reconcile_to_shutdown_chain_wakes_listener`（既定ポリシーで同じ構築のまま listener が起きる）。同じ構造で結論だけが逆なので、両方が緑なら分岐が効いている。
- 2・3 は `block_on` を使うが実窓は作らない。既存の `block_on_ready_future_returns_value` が同じ道具で通っている。
- `run()` そのものを headless で回すテストは**作らない**（VSync スレッドとクリック透過ワーカを起こす `run()` の実績が無く、判断分岐は 1〜4 で固定できる。`run()` の貫通は smoke テストが実プロセスで踏む）。実装中に `run()` の headless 実行が容易だと分かっても、5.3 に従い足さない。

### 既存テストの追随（本文の判断は変えない）

| 置き場 | 追随 |
|---|---|
| `crates/wintf/src/runtime/mod.rs` `tests`（`new_wires_registry_shutdown_hook_to_notify_event`・`close_to_reconcile_to_shutdown_chain_wakes_listener`・`new_owns_unfired_shutdown_signal`） | `app.shutdown.listen()` → `app.exit.signal().listen()`（欄の置き換えのみ） |
| `crates/areka/src/emo2_boot/frame_ghost_quit_tests.rs`（4 本） | `world_with_ghost_windows` が `wintf::AppExit::new()` を `insert_non_send` する。テスト 1（通知 1 件）とテスト 4（窓 0）に「受け口が `is_requested()` になっている」の確認を 1 行ずつ足す（3.1・3.2 の契約の追随。テスト 4 の「`ERROR` 0 行」はそのまま） |
| `crates/areka/src/input_events/input_events_tests.rs`（強制退避 3 本） | `world_with_wiring` と素の `World` を組む 2 本（`escape_works_without_mouse_wiring`・`handler_ctrl_shift_left_double_click_despawns_all_ghost_windows_without_sending`）へ受け口を挿す。後者に `is_requested()` の確認を 1 行（3.3） |
| `crates/areka/src/main_startup_window_tests.rs`（`double_click_left_despawns_all_dummy_windows`） | 受け口を挿し、`is_requested()` の確認を 1 行（3.4）。無関係 entity が残る確認はそのまま |
| `crates/areka/src/main_seam_tests.rs` の `despawn_smoke_targets_*` 3 本 | `crates/areka/src/app_exit_tests.rs` へ移し、対象を私有の `despawn_app_windows` にする。本文（標的の種類・空 World の no-op・連鎖破棄済み標的の打ち切りと対照アーム）は不変 |
| `crates/wintf/src/runtime/window_registry.rs`（3 本）・`message_loop.rs`（4 本）・`crates/wintf/tests/win_app.rs`（2 本） | 変更なし |
| `crates/areka/tests/smoke_boot_loop_exit.rs`（2 本） | 変更なし（doc コメント 1 行の追随のみ）。`AREKA_APP_SMOKE_EXIT_MS=500`・60 秒の見張り・終了コード 0・経路の目印はそのまま。**「窓が無いのにプロセスが残る」壊れ方（受け口不在・指示の送り忘れ・`shutdown_future` の取りこぼし）はこの見張りが赤にする**（5.4） |

### 実機確認（5.5・5.6）

前提: `cargo build -p areka` 済み。検体は emo2（`sample-ghost-kit` の登記）。**引数は絶対パス・短いパス**で渡す（相対だと `pasta.dll` の読み込みが `0x8007007E` で失敗する既知の罠）。ログは `RUST_LOG=info,wintf::runtime=debug`——判定の分岐（2 回目の指示の `debug!`・`WinApp initialized`）まで開けておく。

走行 1（smoke の自動終了・Flow 2・`ExitOrigin::Smoke`）:
```powershell
$env:RUST_LOG = "info,wintf::runtime=debug"
$env:AREKA_APP_SMOKE_EXIT_MS = "3000"
$p = Start-Process -FilePath .\target\debug\areka.exe -ArgumentList "<絶対 ghost root>","<絶対 balloon root>" -PassThru -NoNewWindow -RedirectStandardError .\areka-smoke.log
$p.WaitForExit(60000)
$p.HasExited          # true が期待値。false なら Stop-Process -Id $p.Id（この PID 以外は止めない）
$p.ExitCode           # 0
Select-String -Path .\areka-smoke.log -Pattern 'event="app_exit"|\[AppExit\] exit requested|windows remained open|app_exit_unwired'
```
期待: `event="app_exit" origin=Smoke closed=4`（2 スコープ×キャラ窓＋バルーン窓）→ `[AppExit] exit requested` → （reconcile より先にループが戻れば）`windows remained open` → 終了順序①〜④の既存の info 行 → `HasExited` 真・終了コード 0。`app_exit_unwired` は 0 行。

走行 2（メニューの「終了」・Flow 1・`ExitOrigin::KanadeStopped(Quit)`）: smoke の環境変数を外して同じく `Start-Process -PassThru` で起動し、キャラ窓を右クリック → 「終了」。別れの台詞が流れ終わったあとに `event="ghost_quit" cause=Quit` → `event="app_exit" origin=KanadeStopped(Quit)` → `[AppExit] exit requested` の順で出て、窓が消え、`HasExited` が真になる。`windows remained open` は出ない（同じ tick の reconcile が壊す）。

走行 3（任意・強制退避・`ExitOrigin::Escape`）: 走行 2 と同じ起動で Ctrl＋Shift＋左ダブルクリック。`event="mouse_escape_close"` → `event="app_exit" origin=Escape` → `[AppExit] exit requested`。

ダミー窓の経路（`ExitOrigin::DummyWindow`）は引数なし smoke テスト（フォールバック方向）が実プロセスで踏むので、実機の手動走行は要らない。
止めてよいのは `$p.Id` の 1 つだけ。`Get-Process areka` で見つけた他のプロセスは止めない（5.6）。

## 並走 spec への申し送り

- `areka-P0-baseware-root-layout`: 本仕様が `main.rs` から `despawn_smoke_targets` と `on_dummy_pressed` の本体を出すので、相手が触る `main()` の根の解決との距離が広がる。`tests/smoke_boot_loop_exit.rs` の目印（マーカー）は変えてよいが、**60 秒の見張りと終了コード 0 の判定は残す**（要件の Adjacent expectations に転記済み）。相手の brief への追記は本仕様の完了時（roadmap の干渉台帳の更新と同時）。
- `areka-P0-ghost-shell-balloon-switch`: 本仕様は「窓を全部閉じるが終了しない」操作を作らない。後続は `ExitPolicy::Explicit` の上に自分の操作（例: ゴースト窓だけを閉じて開き直す）を足し、その操作は `quit_app` を通らない。

## §8 設計判断の一覧（`research.md` §6 の項目番号）

| 項目 | 選んだ案 | 一言 |
|---|---|---|
| 1 口の形 | ⑴ `pub enum ExitPolicy { OnLastWindowClose, Explicit }`＋`WinApp::with_exit_policy(policy)`・`new()` は既定で委譲 | 選択肢が 2 つしか無い。設定値の構造体・ビルダーは要らない |
| 2 指示の届け方 | ⑴ wintf が構築時に World へ挿す NonSend `AppExit` | `ClickThroughRegistryHandle` と同型・配線 0 |
| 3 指示済みビット | `AppExit { requested: Rc<Cell<bool>>, signal: Rc<Event> }`（`message_loop.rs`）。`shutdown_future` は arm → 確認 → await。既定ポリシーのフックも `request_exit` を通す（完了機構 1 本）。2 回目は `debug!` | `WinApp` の `shutdown: Rc<Event>` を `exit: AppExit` へ置き換え |
| 5 統合操作の置き場 | 新モジュール `crates/areka/src/app_exit.rs` の `quit_app(world, origin) -> usize`。`despawn_ghost_windows`・`despawn_smoke_targets` は削除して私有部品へ吸収 | 全窓破棄だけの操作を外から呼べなくする |
| 6 受け口不在 | ⑴ テスト側が headless の `AppExit::new()` を挿す。本番の不在は `error!`＋指示なし | 本番の意味論を曲げない |
| 7 出所の記録 | ⑴ 層ごとに 1 行（areka `event="app_exit"`・wintf `[AppExit] exit requested`） | wintf は areka の語彙を知らない |
| 8 `[App] Last window closed.` | ⑴ 触らない | 文は事実のまま（最後の窓は閉じた）。終了を含意していない |
| 10 要件 1.5 のテストの単位 | ⑵ `shutdown_future`＋ビットの単位（`block_on` で駆動）。`run()` 直接のテストは作らない | 判断分岐はこの単位で固定でき、`run()` の貫通は smoke テストが踏む |
| 11 実機確認の手順 | smoke の自動終了＋メニューの「終了」の 2 走行（強制退避は任意）。`RUST_LOG=info,wintf::runtime=debug`・`Start-Process -PassThru` の PID だけを見る | ダミー窓は引数なし smoke テストが踏む |

裁定 1・2 を覆す必要は設計の途中で見つからなかった（6.3）。
