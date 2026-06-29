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

1. **公開 API の互換方針** → **【確定済み・要件討議①・2026-06-29: Option B】**: `WinThreadMgr` を
   温存せず新 facade（新公開 API）へ全面置換し、`WinThreadMgr` 自体を撤去、全 examples ＋ areka 本体を
   新 API へ追従改修する。要件 5（撤去）・要件 6.1/6.3（新 API 追従＋公開 IF 提供）へ反映済み。
   facade の具体 API 形状（new/world/run 相当の代替・構築フロー）は design 期で確定。
2. **`run()`/`block_on` の終了セマンティクス**: 現行「最後の窓→WM_LAST_WINDOW_DESTROYED→
   PostQuitMessage→WM_QUIT で break」を、`block_on` の future 完了駆動へどう写像するか。
   README 学び（block_on は loop 先行 quit で panic）を踏まえ、終了 future の設計（tick タスク・
   全窓 close の検知）をどこに置くか。
3. **ウィンドウクラス登録方式 → 確定（設計討議②・上流修正で解消）**: ライブラリの共有クラスは
   `style=0`（CS_DBLCLKS なし・0.0.3）だったため当初は wintf 側 `SetClassLongPtrW` 補填案を検討したが、
   **フォーク上流 0.0.5 でクラス登録に `CS_DBLCLKS` ＋既定カーソル（`LoadCursorW(IDC_ARROW)`）を内蔵**
   （`src/util/window.rs` の `CLASS_REGISTRATION.call_once`）。最初に生成される `EXECUTOR_WINDOW` が
   共有クラスを CS_DBLCLKS 込みで産むため全実窓へ自動波及。**wintf 側 `DblClkClassFixup` は撤去**し、
   要件 7.1 の pin を `=0.0.5` へ更新。CS_HREDRAW/VREDRAW は合成窓（DComp/ULW）に無影響ゆえ非採用。
4. **共有状態 S の粒度**: `util::Window<S>` の S を `Rc<RefCell<EcsWorld>>`（単一 World 共有）と
   するか、per-window state を別に持つか。GWLP_USERDATA 全廃後の Entity↔HWND 対応の保持場所。
5. **UI 構築入口（`world.spawn` / WintfTaskPool）の扱い** → **【確定済み・要件討議②・2026-06-29: Option 1（温存）】**:
   `WintfTaskPool` は `bevy_tasks::TaskPool` を `EcsWorld::new()` で Resource 生成する ECS 管理の常駐
   背景ワーカープールであり、UI スレッド（`WinThreadMgr`/`wintf-winmsg-executor`）とは無縁な別レイヤ。
   要件 3 の移行対象は UI スレッド async（`executor_normal`/`spawn_normal`）のみとし、`WintfTaskPool` ＋
   areka の `world.spawn(CommandSender)` 経路は触らず温存する（`spawn_local` は UI スレッド単一ゆえ
   背景プールの代替にならない）。要件 3.4 ＋ Boundary Context Out of scope へ反映済み。再設計が要れば別 spec。
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

---

# 設計フェーズ追記（design generation・2026-06-29）

> ギャップ分析（上記）は要件討議の素材。本節以降は design 期の調査結果・合成判断・残リスクを記録する。
> 設計の自己完結記述は design.md 側にあり、本節はその根拠・代替比較・調査ログを保持する。

## 採用クレート 確定 API（§6 Research Needed の解決・ソース確認は 0.0.3、pin は 0.0.5）

> 注: 下記 API は 0.0.3 ソースで確認したが 0.0.3〜0.0.5 で互換（先進坑を 0.0.5 で再ビルド確認済み）。差分はクラス登録の `CS_DBLCLKS`＋既定カーソル内蔵（設計判断 #3）のみ。pin は `=0.0.5`。

design 期に crate registry ソースを直接確認（`c:\rust\cargo\registry\src\index.crates.io-*\wintf-winmsg-executor-0.0.3\src\`）。

### `util::Window<S>`（`util/window.rs`）
- API: `new`/`new_ex(window_type, ex_style, state, wndproc)`、`new_checked`/`new_checked_ex(...)`。
  - `new_*`: `wndproc: Fn(Pin<&S>, WindowMessage) -> Option<LRESULT> + 'static`。
  - `new_checked_*`: `FnMut(...)`。内部で `RefCell<F>` を持ち、`try_borrow_mut().ok()?` で**ネスト/モーダル再入を阻止**（再入時は default wndproc へフォールバック）。
- `WindowType` は `TopLevel` / `MessageOnly` の 2 値のみ（カスタムクラスタイプ指定不可）。
- `hwnd()`/`state() -> Pin<&S>`。Drop で `DestroyWindow`。`WindowMessage{hwnd,msg,wparam,lparam}` は windows 0.62 newtype。
- **クラス登録**: `w!("wintf-winmsg-executor")` を `static Once` で 1 回登録。`WNDCLASSW` は `std::mem::zeroed()`＝**`style=0`（CS_DBLCLKS なし・HREDRAW/VREDRAW なし）**、カーソル未設定、名前固定。**スタイル/名前/複数クラスはパラメタライズ不可**（§6・設計判断3 の Unknown を解決＝「不可」）。
- **HINSTANCE**: ライブラリ内部 `get_instance_handle()` が **`__ImageBase` 方式**（`devblogs` oldnewthing）。0.0.3 で**既に DLL 安全**（`GetModuleHandle(NULL)` ではない）。私有関数ゆえ wintf からは呼べないが `new_ex` 経由で恩恵を受ける。→ **研究 note #7／README の「0.0.4 で `__ImageBase` 化」記述は不正確**。0.0.3 で既に `__ImageBase`。0.0.4 差分は同関数の `pub` 公開のみ。要件 2.5 の HINSTANCE 取得はライブラリ内部で充足され、wintf 側の `GetModuleHandleW(None)` は不要化。
- **状態保持**: `UserData<S,F>{state,wndproc}` を `Box` 化し `GWLP_USERDATA` へ格納（`wndproc_setup`→`WM_NCCREATE` で `GWLP_WNDPROC` を型付き wndproc へ差し替え）。**ライブラリが GWLP_USERDATA を占有**＝wintf は Entity を GWLP_USERDATA へ手詰めできない（手詰め全廃が必然）。`WM_CLOSE` はライブラリが握り潰し（`DestroyWindow` を呼ばず Window<S> drop で破棄）、`WM_NCDESTROY` で `UserData` 解放。

### `lib.rs`（`spawn_local`/`block_on`/`MessageLoop`）
- `spawn_local<T:'static>(fut) -> JoinHandle<T>`: `!Send` future 可（同一スレッド runnable）。wake は内部 `EXECUTOR_WINDOW`（MessageOnly 窓）への `PostMessageW(WM_USER=MSG_ID_WAKE, runnable*)`。`JoinHandle` drop で detach。
- `block_on(fut) -> T`: 内部で `MessageLoop::new()` → future 完了時 `quit()` → `run_loop(Forward)` → `poll_ready(task).expect("received unexpected quit message")`。**ループが future より先に quit すると panic**（§6・終了規律を解決）。
- `MessageLoop::run(filter: Fn(&MessageLoop,&MSG)->FilterResult)`: `GetMessageW` ループ。`WH_MSGFILTER` フック（`msg_filter_hook.rs`）でモーダル内部ループ中も filter 駆動。`quit()`（即時フラグ）／`quit_when_idle()`（`PostQuitMessage(0)`）。**filter 内から `MessageLoop::run` 再帰呼び出しは panic**（nested_message_loop テスト）。wake メッセージは filter で drop 不可（ライブラリ保護）。

## 既存 wintf 側の確認結果（配送・tick・終了の現行結線）
- `create_windows`（`ecs/window/window_system.rs`）が現役の唯一の窓生成路（排他システム・宣言的）。`entity.to_bits()`→`lpCreateParams`→`WM_NCCREATE`→`GWLP_USERDATA`。`CompositionMode`→ex_style 選択（ULW=`WS_EX_LAYERED`/DComp=`WS_EX_NOREDIRECTIONBITMAP`）。
- `ecs_wndproc`（`ecs/window_proc/mod.rs`）= 30 種超 `match`。World 参照は `static ECS_WORLD: OnceLock<SendWeak>`、Entity は `get_entity_from_hwnd`（GWLP_USERDATA）。
- tick: `WinThreadMgrInner::run` の `WM_VSYNC` pop→`try_tick_on_vsync`→`try_tick_world`（13 本）。再入ガード `IS_TICK_FLUSH_IN_PROGRESS`（`ecs/world/vsync.rs`）。`VSYNC_TICK_COUNT`/`LAST_VSYNC_TICK` は `win_thread_mgr` 所属。
- 終了: `App::on_window_destroyed`（`ecs/app.rs`）が `WM_LAST_WINDOW_DESTROYED` を message_window へ PostMessage→`run` が `PostQuitMessage(0)`。
- consumer パターン: 全 examples ＋ areka が `WinThreadMgr::new()→world()→run()`。`world.spawn`（WintfTaskPool・議題②温存）が areka UI 構築入口。`create_window`/`spawn_normal` は `dcomp_demo.rs` のみ。areka は**ダブルクリック終了**（CS_DBLCLKS 依存・structure.md 確認）。

## Design Decisions

### Decision: 新 facade を `WinApp` として新設（議題①= Option B 確定の具体化）
- Context: `WinThreadMgr` 撤去・新公開 API への全面置換（要件 5.1/6.3 確定）。
- Selected Approach: `crates/wintf/src/runtime/` に `WinApp`（owner）を新設。公開 API を旧 3 点（`new`/`world`/`run`）に意味対応＋ `spawn_ui_local`（旧 `spawn_normal` 相当・`!Send` 可）。
- Rationale: 旧 3 点へ意味対応させることで consumer 追従コストを最小化（要件 6.1）。owner 役（COM/DPI 初期化・VSync スレッド・終了規律の所有）を 1 箇所へ集約。
- Trade-offs: 新規ファイル群だがレガシーと物理分離でき撤去が機械的。

### Decision: ウィンドウ生成は `new_checked_ex` を採用（`new_ex` ではなく）
- Alternatives: (A) `new_ex`（`Fn`・自前再入防御）, (B) `new_checked_ex`（`FnMut`＋ライブラリ `RefCell` 再入防御）。
- Selected: (B)。Rationale: wndproc 再入防止をライブラリ標準機構へ委譲でき（要件 4.3）、先進坑が `new_checked_ex` で nested 719 回 PASS を実証。`FnMut` 許容で実装自由度も高い。

### Decision: Entity 配送は GWLP_USERDATA 全廃＋クロージャ capture
- Context: ライブラリが GWLP_USERDATA を占有するため Entity を従来の場所へ置けない（§設計判断4）。
- Selected: `create_windows` が生成時に entity を知るので、wndproc クロージャへ `(Rc<RefCell<EcsWorld>>, Entity)` を capture。`get_entity_from_hwnd`／`ECS_WORLD: OnceLock<SendWeak>` を撤去。`ecs_wndproc` の `match` 表は純関数 `dispatch_window_message(world, entity, msg)` へ移設。
- Rationale: 単一 World 共有・per-window クロージャという構造で手詰め全廃が成立（要件 2.3/2.4）。先進坑が `Pin<&S>` 経由 state 到達を実証。

### Decision: CS_DBLCLKS は `SetClassLongPtrW(GCL_STYLE)` で補填
- Context: ライブラリクラスは `style=0`＝CS_DBLCLKS 無し。areka のダブルクリック終了・`mouse_dblclick_wheel` ハンドラが機能しない（要件 2.5/6.1 の回帰リスク）。
- Alternatives: (A) クラス style にライブラリ経由で CS_DBLCLKS 指定（**不可**＝API なし）, (B) 初回窓生成後に `SetClassLongPtrW(hwnd, GCL_STYLE, cur|CS_DBLCLKS)` でクラス共有補填, (C) wndproc 内で自前ダブルクリック検出。
- Selected: (B)。Rationale: クラスはプロセス共有ゆえ 1 回の補填で全窓へ波及。単純・確実。(C) はフォールバックとして記録。
- Follow-up: 初回生成タイミングの確実性を実装フェーズで検証。将来版で style パラメタライズ可能化なら不要化。

### Decision: 終了規律＝shutdown future を `block_on` で待つ（PostQuitMessage を撃たない）
- Context: `block_on` は loop 先行 quit で panic（要件 1.4）。
- Selected: `run()` は `block_on(shutdown_signal.await)`。`App::on_window_destroyed`（最後の窓）が `event_listener::Event`（shutdown_signal）を notify→future 完了→`block_on` 正常復帰。`WM_LAST_WINDOW_DESTROYED` PostMessage を撤去。未完 UI async タスクを残したまま `PostQuitMessage` しない（README 終了規律）。tail race は終了時 notify 補填で回避。
- Trade-offs: message_window の存在意義（VSync PostMessage 宛先）が tick の event_listener 化で消えるため、message_window/`set_message_window` は撤去候補。

### Decision: 再入ガード `IS_TICK_FLUSH_IN_PROGRESS` は安全側に残置（議題6）
- Context: 新モデルで message pop 由来の tick 再起動は構造的に減るが、`flush_window_pos_commands`→`SetWindowPos`→同期 `WM_WINDOWPOSCHANGED` 経路は残る。
- Selected: ECS ガード（`IS_TICK_FLUSH_IN_PROGRESS`）＋ライブラリ `new_checked_ex` の `RefCell`（wndproc 再入防止）の**二重防御を維持**。先進坑が両者の非衝突（`double_tick=false`）を実証。
- Rationale: 安全側委譲。完全委譲はリスクが高く、要件 4.3 を確実に満たすため二重化を選択。

## Synthesis Outcomes（design-synthesis 適用）
- **Generalization**: 4 層（メッセージ/窓/async/tick）は「UI スレッド単一の owner（`WinApp`）が委譲する薄いアダプタ群」に一般化。owner が lifecycle/初期化/終了規律を一括所有（areka concurrency-model のメモリと整合）。
- **Build vs Adopt**: pump・状態保持・wake・モーダル対応・wndproc 再入防止は**全てライブラリ採用**（自作撤去）。スレッド跨ぎ起床は `event_listener` 採用（tokio 不採用＝要件制約）。wintf 固有結線（Entity 配送・13 本 tick・終了規律・CS_DBLCLKS 補填）のみ自作。
- **Simplification**: tick タスクは**単一**（単一 World 共有ゆえ。先進坑の 3 タスクはストレス検証用で本番不要）。`WM_VSYNC`/message_window/`OnceLock<SendWeak>`/`get_entity_from_hwnd`/`process_singleton` 2 クラス登録を**撤去**（重複機構の排除）。

## Risks & Mitigations
- 生成後 style/title/pos 反映の順序ミス → 現行 `to_window_coords_for_creation` を生成後 `SetWindowPos` へ移植・初期化ステップを明示。
- tick が 1 vblank を超過時の積み残し → `VSYNC_TICK_COUNT` 差分検知で複数 notify を 1 tick へ間引き（現行踏襲）。
- `Window<S>` 所有権保持先（`WindowHandle` 併設 vs World マップ）→ Entity ライフサイクルと Drop=`DestroyWindow` 一致を前提に実装フェーズ確定（どちらでも境界は閉じる）。
- ビルド未実施（vendors/pasta submodule 未populate 回避）→ 静的ソース解析で確定。実ビルド/実行検証は実装フェーズ。

## References
- crate source: `c:\rust\cargo\registry\src\index.crates.io-*\wintf-winmsg-executor-0.0.3\src\{lib.rs,util/window.rs,util/msg_filter_hook.rs}` — 0.0.3 確定 API の一次根拠。
- 先進坑 README: `crates/pilot/examples/wintf-winmsg-executor/README.md` — 検証結果（go 判定・起床安定性・再入整合・終了規律）の正本（二重化しない・要件 7.4）。
- steering: `tech.md`（依存・レガシー非推奨方針）, `structure.md`（Message Handling 配置・ダブルクリック終了）。
