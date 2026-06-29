# ギャップ分析: wintf-winmsg-executor

> 本書は確定済み requirements.md と既存コードベースのギャップ分析である。意思決定ではなく
> 情報と選択肢を提供する（要件討議の素材）。先進坑の検証結果は
> `crates/pilot/examples/wintf-winmsg-executor/README.md` を正本とし二重化しない（二坑モデル要件 3.5）。

## 1. 現状調査（既存コードベースの地図）

### 1.1 メッセージループ層（要件 1）
- 正本: `crates/wintf/src/win_thread_mgr.rs`。
- `WinThreadMgrInner::run()`（202行〜）が自作ポンプ。`PeekMessageW(PM_REMOVE)` →
  `WM_QUIT` で break、`WM_VSYNC`（`WM_USER+1`）で `try_tick_on_vsync()`、
  `WM_LAST_WINDOW_DESTROYED`（`WM_USER+2`）で `PostQuitMessage(0)`、その他は
  `TranslateMessage`/`DispatchMessageW`。メッセージが無ければ `try_tick_normal()`
  （`async_executor::Executor::try_tick()`）→ `WaitMessage()`。
- COM 初期化は `WinThreadMgrInner::new()` の `CoInitializeEx(COINIT_MULTITHREADED)`。
  Drop での `CoUninitialize` は意図的に省略（NOTE(W1-V)・P30）。
- 終了経路は二段: ①最後のウィンドウ破棄で `App::on_window_destroyed()`
  （`ecs/app.rs:65`）が message_window へ `WM_LAST_WINDOW_DESTROYED` を PostMessage
  → ②`run()` が受けて `PostQuitMessage(0)`。`winproc.rs:91` 側にも同名処理がある（legacy 経路）。

### 1.2 60Hz ECS tick 駆動（要件 4）
- VSync 専用スレッド（`spawn_vsync_thread`・`win_thread_mgr.rs:331`）が
  `DwmFlush()` 同期 → `VSYNC_TICK_COUNT.fetch_add` → `PostMessageW(WM_VSYNC)`。
  メインスレッドが pop し `Rc<RefCell<EcsWorld>>::try_tick_on_vsync()`（`ecs/world/vsync.rs:51`）。
  = ユーザ指摘の「メッセージ pop 駆動」方式。
- 二重の再入ガード:
  - `IS_TICK_FLUSH_IN_PROGRESS`（`ecs/world/vsync.rs:17`・thread_local Cell + RAII guard）。
    `flush_window_pos_commands()` → `guarded_set_window_pos()` → 同期 `WM_WINDOWPOSCHANGED`
    → 再 `try_tick_on_vsync()` の再帰ループ（フリーズ）を阻止。
  - `RefCell::try_borrow_mut()` の借用失敗で安全スキップ。
- 実 tick は `EcsWorld::try_tick_world()`（`ecs/world/mod.rs:436`）が13本のスケジュールを
  固定順で `try_run_schedule`（Input→Update→PreLayout→Layout→PostLayout→UISetup→
  GraphicsSetup→Draw→PreRenderSurface→RenderSurface→Composition→CommitComposition→FrameFinalize）。
- `try_tick_on_vsync()` は VSYNC_TICK_COUNT と LAST_VSYNC_TICK の差分検知（`ecs/world/mod.rs:505`）。
  カウンタは Relaxed（単一生産者・単一消費者の前提を文書化済み）。
- リフレッシュレート: DwmFlush は実 vblank 追従ゆえ可変周期（先進坑実測 120Hz 機で ≈8.3ms）。
  現行は固定16.67ms前提を持たない（要件 4.4 と整合）。

### 1.3 ウィンドウ生成・ウィンドウ手続き層（要件 2）
- クラス登録: `WinProcessSingleton::get_or_init()`（`process_singleton.rs:51`）が
  HINSTANCE を `GetModuleHandleW(None)` で取得し、**2クラス**を `RegisterClassExW`:
  - `wintf_window_class`（legacy `wndproc`）
  - `wintf_ecs_window_class`（現役 `ecs_wndproc`・`CS_DBLCLKS` 付き）
  - 加えて `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)`。
- **現役のウィンドウ生成は `ecs/window/window_system.rs:create_windows()`**（排他システム）。
  ここが直接 `CreateWindowExW` を呼び、`ecs_window_class_name` を使い、`entity.to_bits()` を
  `lpCreateParams` に渡す（`WM_NCCREATE` で `GWLP_USERDATA` へ手詰め）。
  `WinThreadMgrInner::create_window()`（`win_thread_mgr.rs:149`・Arc ハンドラ + GWLP_USERDATA）は
  **legacy 経路で、現在は `examples/dcomp_demo.rs` のみが使用**。
- `ecs_wndproc`（`ecs/window_proc/mod.rs:52`）は30種超のメッセージを各ハンドラ関数へ振り分け、
  `get_entity_from_hwnd()`（`GWLP_USERDATA` から `Entity::try_from_bits`）で World へ dispatch。
  World 参照は `static ECS_WORLD: OnceLock<SendWeak>`（`Weak<RefCell<EcsWorld>>` の手動 Send/Sync）。
- `WM_NCCREATE`/`WM_NCDESTROY`（`ecs/window_proc/lifecycle.rs`）が GWLP_USERDATA の格納・クリアと
  Entity despawn を担う。
- ex_style 経路: `create_windows` が `CompositionMode` に応じ ULW=WS_EX_LAYERED /
  DComp=WS_EX_NOREDIRECTIONBITMAP を選択（`window_system.rs:79`）。要件 2.2 の受け渡し口に直結。

### 1.4 UI スレッド async 実行層（要件 3）
- 手組み executor: `WinThreadMgrInner::executor_normal: async_executor::Executor<'static>`。
  `spawn_normal()`（`Send + 'static` 制約付き）でタスク投入、`run()` の idle 時 `try_tick()` で駆動。
  使用例は `examples/dcomp_demo.rs:53` のみ。
- 別レイヤの背景タスク: `WintfTaskPool`（`ecs/widget/bitmap_source/task_pool.rs`）は
  `bevy_tasks::TaskPool` + `mpsc`。`EcsWorld::spawn(FnOnce(CommandSender)->Fut)`
  （`ecs/world/mod.rs:394`）が areka 本体の UI 構築に使われている（`areka/src/main.rs:128`）。
  `CommandSender = mpsc::Sender<BoxedCommand>` を Input スケジュールの
  `drain_task_pool_commands`（`bitmap_source/systems.rs:320`・`ecs/world/mod.rs:103`）で drain。
  → **これは「背景重処理用 TaskPool」であり要件で Out of scope。だが UI 構築の現行入口が
  この `world.spawn()` 経由である点に注意**（後述・設計判断 5）。

### 1.5 利用側（公開 API 消費者）
- 全 examples（`crates/wintf/examples/*`）と `areka/src/main.rs` が
  `WinThreadMgr::new()` → `mgr.world()` → `mgr.run()` の3点を共通利用。
- `mgr.spawn_normal()` と `mgr.create_window()` の利用は `dcomp_demo.rs` のみ。
- areka 本体は `create_window` を直接呼ばず、ECS の `Window`/`WindowStyle`/`WindowPos`
  コンポーネントを spawn し、`create_windows` システムが生成する宣言的経路。

### 1.6 レガシー資産（要件 5・撤去対象）
- `win_message_handler.rs`（約1400行・`#[deprecated]`・巨大 per-message トレイト）。
- `winproc.rs`（legacy `wndproc` + 既知の健全性違反 get_boxed_ptr の UB 経路・P27/P28）。
- `win_thread_mgr.rs` 自体（`#![allow(deprecated)]`）。tech.md にも非推奨と明記済み。
- 撤去には `process_singleton.rs` の legacy クラス登録（`wintf_window_class` + `wndproc` 参照）の
  整理が連動する。

## 2. 要件 → 資産マップ（ギャップタグ: Missing / Unknown / Constraint）

| 要件 | 既存資産 | ギャップ |
|---|---|---|
| 1 メッセージループ置換 | `WinThreadMgr::run()` 自作ポンプ | **Missing**: `MessageLoop`/`block_on` への写像。**Constraint**: `WM_LAST_WINDOW_DESTROYED`/`PostQuitMessage` 終了経路の引き継ぎ。**Unknown**: `block_on` は loop 先行 quit で panic（README 学び）→ 終了規律の再設計 |
| 2 窓生成・wndproc | `create_windows` + `ecs_wndproc` + `GWLP_USERDATA` + 2クラス登録 | **Missing**: `util::Window<S>` への移行と Entity dispatch の閉包再構築。**Constraint**: `CS_DBLCLKS` と30種超メッセージ振り分けをライブラリ内部クラス上で再現。**Unknown**: ライブラリ内部のクラス登録とカスタムクラス style の両立可否 |
| 2.2 ex-style 受け渡し | `create_windows` の `CompositionMode`→ex_style 選択 | **Constraint**: `new_ex`/`new_checked_ex` の ex-style 口へ移植（先進坑で実証済み） |
| 3 UI async | `executor_normal` (`async_executor`) + `spawn_normal` | **Missing**: `spawn_local`/`block_on` への置換。**Constraint**: tokio 非依存・`!Send` future 許容（先進坑で実証済み） |
| 4 60Hz tick ブリッジ | VSync スレッド→PostMessage(WM_VSYNC)→pop | **Missing**: `event_listener::Event` notify → `spawn_local` async tick タスク。**Constraint**: 13本スケジュール構成・順序の不変（要件 4.5）、再入ガードの整合（要件 4.3） |
| 5 レガシー撤去 | `win_message_handler`/`winproc`/`win_thread_mgr` | **Constraint**: legacy クラス登録・WM_LAST_WINDOW_DESTROYED の重複処理（winproc.rs と run() の両方）を漏れなく撤去 |
| 6 examples 回帰防止 | 全 examples + areka が `new/world/run` 利用 | **Constraint**: 公開3点 API（`new`/`world`/`run`）の互換維持 or 追従。**Constraint**: 32bit 可搬性（host-32 は別プロセスゆえ非影響） |
| 7 バージョン pin/前提依存 | tech.md に async-executor/bevy_tasks 記載 | **Missing**: `wintf-winmsg-executor = "=0.0.3"`・`event-listener = "5"` を依存と tech.md に追加。go 判定取得済み（前提充足） |

## 3. 実装アプローチ（複数案）

### 3.1 全体方針: `WinThreadMgr` の内部置換（共通土台）
- **Option A（既存拡張・推奨候補）**: `WinThreadMgr` の公開3点 API（`new`/`world`/`run`）の
  シグネチャを維持しつつ、内部実装を `MessageLoop`/`block_on`/`spawn_local` ベースへ差し替える。
  - ✅ 全 examples・areka が無改修 or 最小改修で回帰確認可能（要件 6.1）。
  - ✅ 移行リスクが `WinThreadMgr` 内に局所化。
  - ❌ `run()` が `block_on` の future 完了で返る新セマンティクスと、現行「最後の窓で quit」を
    どう橋渡しするか設計が要る（block_on panic 規律・README 学び）。
- **Option B（新規 facade）**: `WinThreadMgr` を deprecated 化し、新 facade を導入。
  利用側を新 API へ全面追従。
  - ✅ legacy 由来の制約から解放、クリーンな新設計。
  - ❌ 全 examples + areka の改修が必要（回帰確認の母数増・要件 6 のコスト増）。
- **Option C（ハイブリッド・段階移行）**: まず `WinThreadMgr` 内部を置換（A）し、後続で必要なら
  facade 整理（B）。利用側 spec（areka-P0-window-placement 等）の追従に必要な公開 IF だけ先に確定。
  - ✅ 可逆性最優先（二坑モデル）に整合。段階コミット可能。
  - ❌ 一時的に新旧概念が同居。

### 3.2 ウィンドウ生成・クラス登録の扱い（最大の構造的論点）
- 現行は「自前 `RegisterClassExW`（CS_DBLCLKS + ecs_wndproc）＋ `create_windows` が直接
  `CreateWindowExW`」。一方ライブラリの `util::Window<S>` は内部でクラス登録し wndproc 閉包を束ねる。
- **論点**: ECS は ①ダブルクリック（CS_DBLCLKS）②30種超メッセージ ③Entity 単位 dispatch を要する。
  - **案1**: ライブラリの `Window<S>` をそのまま使い、S=`Rc<RefCell<EcsWorld>>` を預け、
    閉包 wndproc が `msg.msg` で分岐して既存ハンドラ（lifecycle/mouse/keyboard…）へ委譲。
    Entity は HWND→Entity の対応表を World 側で保持（GWLP_USERDATA 全廃）。
    → **Unknown**: `Window<S>` 経由で CS_DBLCLKS 相当を設定できるか（クラス style 指定口の有無）。
    要 design 期調査（クレートソース）。
  - **案2**: クラス登録は自前のまま維持し、状態アクセス機構（`Pin<&S>`）だけライブラリの
    state 機構へ寄せる部分採用。
    → ライブラリの `Window<S>` の前提（自前クラス）と齟齬する可能性。**Unknown**。
- **状態の置き場**: S を `Rc<RefCell<EcsWorld>>` にすると、現行の `OnceLock<SendWeak>` 経由の
  グローバル World 参照を閉包 capture へ置換できる（GWLP_USERDATA + ECS_WORLD 両方を撤去可能）。
  ただし複数ウィンドウで同一 World を共有するため、S を per-window にするか World 共有にするかは
  設計判断（先進坑は per-window の Rc<Shared> = 窓ごと状態だった点に注意・ECS は単一 World 共有）。

### 3.3 tick ブリッジ
- 先進坑実証パターン（README §概要・検証結果）に沿う: vsync スレッドが `Event::notify` →
  `spawn_local` の async tick タスクが `listen().await` 起床 → `try_tick_world()` 1回 → 再 await。
- **再入ガードの委譲度**（要件 4.3）: README 学びでは「tick を message からでなく event_listener
  起床の async タスクで駆動するため WM_WINDOWPOSCHANGED 由来の再入が構造的に発生しにくい」。
  → `IS_TICK_FLUSH_IN_PROGRESS` の一部をライブラリ（`new_checked` の RefCell）＋新モデルへ
  委譲できる「見込み」。**Unknown/設計判断**: どこまで自前ガードを残すか（安全側に二重化するか）。

## 4. 複雑度・リスク

- **メッセージループ層**: 効果 M / リスク Medium。新セマンティクス（block_on 終了規律）への
  橋渡しが要るが先進坑で挙動は実証済み。
- **窓生成・wndproc 層**: 効果 L / リスク High。クラス登録方式の齟齬（CS_DBLCLKS・カスタム class）と
  30種超メッセージ・Entity dispatch の再配線が最大の不確実点。GWLP_USERDATA 全廃の影響範囲も広い。
- **UI async 層**: 効果 S / リスク Low。`spawn_local`/`block_on` の素利用は先進坑で実証・既存利用は薄い。
- **tick ブリッジ層**: 効果 M / リスク Medium。先進坑が並行ストレス含め PASS 済み。13本順序の不変と
  再入整合の検証が要る。
- **レガシー撤去 + examples 回帰**: 効果 M / リスク Medium。撤去自体は機械的だが、公開 API
  互換の取り方次第で全 examples の改修母数が変動。
- **総合**: L〜XL（横断リファクタ・複数層・High リスク層を含む）。

## 5. 設計判断項目（要件討議へ送る）

1. **公開 API の互換方針**: `WinThreadMgr` の `new`/`world`/`run` を内部置換で温存（Option A/C）か、
   新 facade へ移行して利用側を追従（Option B）か。要件 6.1（回帰なし）と要件 6.3（利用側 spec への
   公開 IF 提供）のバランス。
2. **`run()`/`block_on` の終了セマンティクス**: 現行「最後の窓→WM_LAST_WINDOW_DESTROYED→
   PostQuitMessage→WM_QUIT で break」を、`block_on` の future 完了駆動へどう写像するか。
   README 学び（block_on は loop 先行 quit で panic）を踏まえ、終了 future の設計（tick タスク・
   全窓 close の検知）をどこに置くか。
3. **ウィンドウクラス登録方式**: ライブラリ `util::Window<S>` の内部クラス登録を全面採用するか、
   CS_DBLCLKS・カスタム class style・複数クラスの要求とどう両立するか（案1/案2）。
   → ライブラリの class style 指定能力は design 期に要ソース調査（Research Needed）。
4. **共有状態 S の粒度**: `util::Window<S>` の S を `Rc<RefCell<EcsWorld>>`（単一 World 共有）と
   するか、per-window state を別に持つか。GWLP_USERDATA 全廃後の Entity↔HWND 対応の保持場所。
5. **UI 構築入口（`world.spawn` / WintfTaskPool）の扱い**: 背景 TaskPool は Out of scope だが、
   areka の UI 構築は現状 `world.spawn(CommandSender)`（mpsc drain）経由。新 `spawn_local` へ
   移すか、TaskPool 経路を温存して触らないか。要件 3 の対象（UI スレッド async = executor_normal）と
   背景 TaskPool の線引きを討議で確定する。
6. **再入ガードの委譲度**（要件 4.3）: `IS_TICK_FLUSH_IN_PROGRESS` を新モデル + ライブラリ RefCell へ
   どこまで委譲し、どこを安全側に残すか。
7. **HINSTANCE 取得**（要件 2.5）: 現行 `GetModuleHandleW(None)`。0.0.3 では `get_instance_handle`
   非公開のため `new_ex`/`new_checked_ex` 経由で回避（要件 7.1 が 0.0.3 pin を要求）。DLL 文脈で
   HINSTANCE が要る場合の扱い（0.0.4 の `__ImageBase` 方式は本坑 pin 対象外）。
8. **依存追加と tech.md 更新**: `wintf-winmsg-executor = "=0.0.3"`・`event-listener = "5"` の追加、
   置換完了後の `async-executor`/`bevy_tasks` 記載の扱い（TaskPool 残置なら bevy_tasks は残る）。

## 6. Research Needed（design 期へ持ち越し）

- `wintf-winmsg-executor` 0.0.3 の `util::Window<S>` / `MessageLoop` / `block_on` / `spawn_local`
  の正確な API シグネチャとクラス登録の内部仕様（特に window class style / CS_DBLCLKS 指定可否、
  複数ウィンドウ・複数クラスの扱い）。crates.io ソースを design 期に直接確認。
- `block_on` の終了規律（future 完了 vs loop quit の panic 条件）の厳密仕様（src/lib.rs の
  `expect("received unexpected quit message")`）。
- 既存30種超メッセージハンドラ（`ecs/window_proc/*`）を閉包 wndproc 上へ束ねる際の dispatch 形と、
  `Pin<&S>` 経由 World 借用の整合（modal/nested 再入時の RefCell 借用と ECS 借用の二重借用回避）。
