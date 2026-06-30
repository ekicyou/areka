# Implementation Plan

> 本坑 wintf-winmsg-executor の実装タスク。設計（design.md）の Migration Strategy（8 段階）と
> 8 コンポーネント（WinApp / MessageLoopDriver / ShutdownPolicy / EcsWindowFactory /
> EntityWndprocBridge / WindowRegistry / VsyncEventBridge / AsyncTickTask）に基づく。
> 先進坑コードはコピー流用せず README 知見を参照して一から実装する（要件 7.4）。
> 各 Phase はワークツリーブランチ上で論理単位ごとに随時コミット可（completed 時 squash）。

## 1. Foundation: 依存とランタイム基盤の足場

- [x] 1.1 採用クレートの依存追加とバージョン pin
  - `wintf-winmsg-executor` を `=0.0.5`（共有クラスに CS_DBLCLKS ＋既定カーソル内蔵版）に pin し、`event-listener` を依存に追加する
  - tech.md に採用クレート・event_listener・tokio 非依存方針を反映する
  - 先進坑 `pilot/wintf-winmsg-executor` の go 判定（取得済み）を前提依存として満たすことを確認する
  - 完了状態: 新依存を含む wintf がクレート解決し、`cargo build` が依存エラーなく通る
  - _Requirements: 7.1, 7.2, 7.3_

- [x] 1.2 runtime モジュール新設と WinApp facade の骨格
  - 新しい runtime レイヤを設け、`WinApp` を UI スレッド基盤の owner として導入する（旧 `WinThreadMgr` 置換の新公開 facade）
  - COM 初期化（`COINIT_MULTITHREADED`）・DPI awareness 設定・`EcsWorld`（`Rc<RefCell<EcsWorld>>`）生成を WinApp に集約する（旧 process_singleton / WinThreadMgr から移設）
  - 公開 API（new / world / UI async 投入口 / run）のシグネチャを確定し、run は最小スタブとする
  - 先進坑コードを流用せず README 知見から実装する
  - 完了状態: `WinApp::new()` が COM/DPI を初期化し world ハンドルを返す（既存レガシーは未撤去のまま並存）
  - _Requirements: 6.3, 5.2, 7.4_
  - _Boundary: WinApp_

## 2. Core: メッセージループと tick 駆動層

- [x] 2.1 (P) メッセージループ層の委譲
  - 自作 `PeekMessageW` ポンプを撤去し、ライブラリの `block_on` / `MessageLoop::run(filter)` へ委譲する
  - filter は原則 Forward とし、自前 `WM_VSYNC` pop 分岐を持たない（wake メッセージはライブラリ保護に委ねる）
  - 完了状態: WinApp が OS メッセージをライブラリのループ経由で取りこぼしなくウィンドウ手続きへ配送する最小経路が動作する
  - _Requirements: 1.1, 1.2_
  - _Boundary: MessageLoopDriver_

- [x] 2.2 (P) VSync 起床ブリッジ
  - 専用 VSync スレッドが `DwmFlush` で vblank を検出し、共有 `event_listener::Event` を全リスナ起床で notify する
  - Event は WinApp が所有し、スレッド生存期間を stop→join の順序規律で管理する
  - 周期はモニターのリフレッシュレートに追従し、固定 16.67ms を前提にしない
  - 完了状態: vblank ごとに Event が notify され、UI スレッド側の待機タスクを起床できる
  - _Requirements: 4.1, 4.4_
  - _Boundary: VsyncEventBridge_

- [x] 2.3 60Hz async tick タスク
  - `spawn_local` した UI スレッド async タスクが、起床通知を待って 1 フレーム分の ECS tick（13 本スケジュール）を実行し再待機するループを実装する
  - 13 本の構成・実行順序は不変とし、既存の tick 実行経路をそのまま呼ぶ
  - ECS 再入ガード（tick フラッシュ進行中フラグ）を安全側に残置し、ライブラリの wndproc 再入防止と二重防御させる
  - 完了状態: 起床ごとに 13 本 tick が 1 周実行され、再入時は借用失敗で安全スキップして二重 tick が起きない
  - _Requirements: 4.2, 4.3, 4.5_
  - _Boundary: AsyncTickTask_
  - _Depends: 2.2_

## 3. Core: ウィンドウ生成・配送・所有

- [x] 3.1 (P) ウィンドウ手続きブリッジと配送純関数
  - ウィンドウ手続きクロージャを構築し、共有状態 S に World への弱参照と当該 Entity を保持させる（生成時に確定・以後不変）
  - 旧 `ecs_wndproc` の 30 種超メッセージ振り分け表を、World と Entity を引数で受け取る純関数（dispatch_window_message 相当）へ移設する
  - クロージャは弱参照を upgrade し、借用失敗・破棄中は安全スキップする
  - 完了状態: 代表メッセージがクロージャ経由で配送純関数へ橋渡しされ、None 時はライブラリの既定手続きへ委譲される
  - _Requirements: 2.3, 2.4_
  - _Boundary: EntityWndprocBridge_

- [x] 3.2 ウィンドウ手続きハンドラの統一シグネチャ移行
  - 既存ハンドラ群（lifecycle / mouse / keyboard / window_pos / dpi 等・計 31 箇所の自己解決）を、World と Entity を引数で受け取る統一シグネチャへ機械的に移行する
  - グローバル World 参照（`OnceLock` 保持の弱参照）と HWND→Entity 解決（GWLP_USERDATA 依存）を撤去する
  - 各ハンドラ内部の業務ロジックは不変に保つ
  - 完了状態: 全ハンドラが引数経由で World/Entity を受け取り、旧グローバル参照・GWLP_USERDATA 解決への参照が残らずビルドが通る
  - _Requirements: 2.3, 2.4, 5.2_
  - _Boundary: window_proc handlers_
  - _Depends: 3.1_

- [x] 3.3 (P) WindowRegistry（NonSend 所有とリコンサイル）
  - 生成済みウィンドウハンドルを Entity キーで保持する NonSend リソースを実装する（!Send を保持＝UI スレッド束縛・Send 偽装はしない）
  - `Window` コンポーネント破棄を検知するリコンサイルで該当要素を drop し、ハンドル破棄（DestroyWindow）を Entity ライフサイクルに一致させる
  - 除去後に空になったら終了シグナルを発火できる接点を用意する
  - 完了状態: Entity 破棄でレジストリ要素が drop されて窓が破棄され、空判定で終了通知をトリガできる
  - _Requirements: 1.3, 2.1, 5.2_
  - _Boundary: WindowRegistry_

- [x] 3.4 ECS ウィンドウ生成の移行
  - 宣言的ウィンドウ生成を、自作 `CreateWindowExW` 直呼びからライブラリの再入安全なウィンドウ生成（new_checked_ex 相当）へ置換する
  - 透過合成モードに応じた拡張スタイル（ULW=LAYERED / DComp=NOREDIRECTIONBITMAP）の受け渡し口を用い、生成後にスタイル・座標・タイトルを反映する初期化を行う
  - 生成したハンドルを WindowRegistry へ格納し、CS_DBLCLKS はライブラリ内蔵に委ねて wintf 側補填を設けない
  - 完了状態: 宣言的にスポーンした窓がライブラリ経由で生成され、レジストリ保持・スタイル反映済みで表示される
  - _Requirements: 2.1, 2.2, 2.5_
  - _Boundary: EcsWindowFactory_
  - _Depends: 3.1, 3.3_

## 4. Integration: 結線・終了規律・利用側追従・撤去

- [x] 4.1 窓の畳み方の反転
  - クローズ要求ハンドラを「ウィンドウ破棄の直叩き」から「対象 Entity の除去要求（ECS コマンド enqueue）」へ反転する
  - 破棄完了手続きでは ECS 後始末（despawn / 借用）を持たせず、レジストリ要素 drop 駆動の破棄と整合させる（同期再入時の二重借用回避）
  - 非クライアント生成手続きの GWLP_USERDATA 手詰めを撤去する
  - 完了状態: クローズ操作が除去要求として処理され、レジストリ drop により窓が破棄されてパニック・二重借用が起きない
  - _Requirements: 2.3, 1.3_
  - _Boundary: window_proc lifecycle, WindowRegistry_
  - _Depends: 3.2, 3.3, 3.4_

- [x] 4.2 終了規律（ShutdownPolicy）の結線
  - 終了シグナルを ECS 層が所有する `event_listener::Event` とし、WinApp 構築時に下向き注入する（上向き依存を作らない）
  - 最後の窓が消えた（レジストリ空）時点でシグナルを notify し、run が待つ shutdown future を完了させて正常復帰させる
  - 旧 `WM_LAST_WINDOW_DESTROYED` / message_window 経路を撤去し、tail race 回避に終了時 notify を補う
  - 完了状態: 全ウィンドウ破棄でシグナルが発火し、ループが先行 quit せず future 完了で panic なく復帰する
  - _Requirements: 1.3, 1.4, 1.5_
  - _Boundary: ShutdownPolicy, WinApp_
  - _Depends: 4.1_

- [x] 4.3 WinApp::run の全結線と UI スレッド async
  - run で async tick タスクを `spawn_local` し、shutdown future を `block_on` し、VSync ブリッジ・レジストリ・Event の生存期間を WinApp が所有する
  - UI スレッド単一の async 投入口（spawn_ui_local 相当）を提供し、手組み executor（async-executor + spawn_normal）経路を置換する（tokio 非依存・!Send future 許容）
  - 完了状態: `WinApp::new → world → run` の最小フローでメッセージループ・tick・終了が一体で動作する
  - _Requirements: 1.1, 1.3, 3.1, 3.2, 3.3, 6.1, 6.3_
  - _Boundary: WinApp_
  - _Depends: 2.1, 2.3, 4.2_

- [x] 4.4 利用側（examples・areka）の新 API 追従
  - 全 examples と areka 本体の `WinThreadMgr::new/world/run` を `WinApp` 新 API へ追従改修する
  - `dcomp_demo` の旧 `create_window` / `spawn_normal` 利用を、宣言的生成と UI async 投入口へ書き換える
  - 背景プール（WintfTaskPool / world.spawn 経路）は温存し触らない
  - 完了状態: 既存 examples と areka が新 facade 上でビルド・起動し、置換前と同等に動作する
  - _Requirements: 6.1, 4.4, 3.4_
  - _Boundary: consumers (examples, areka)_
  - _Depends: 4.3_

- [ ] 4.5 レガシーコードの撤去
  - deprecated レガシー（自作ポンプ／旧 wndproc／巨大メッセージトレイト）と旧 `WinThreadMgr` facade、2 クラス登録（process_singleton）を撤去する
  - モジュール宣言・公開エクスポートを新 facade へ差し替え、UI async からの async-executor 直接依存を撤去する
  - 完了状態: 旧 API・旧モジュールへの参照が残らず、ビルドおよび既存テストが成功する
  - _Requirements: 5.1, 5.2_
  - _Boundary: legacy modules (win_thread_mgr, winproc, win_message_handler, process_singleton)_
  - _Depends: 4.4_

## 5. Validation: テストと回帰確認

- [x] 5.1 単体テスト
  - 配送純関数が代表メッセージ（ダブルクリック・WINDOWPOSCHANGED・DPICHANGED・非クライアント破棄）で旧手続きと同等の結果を返すことを検証する
  - 終了シグナルで shutdown future が完了し、ループ相当が panic せず復帰することを検証する
  - 再入ガードが tick 中に立ち、スコープ離脱で戻ることを検証する
  - 完了状態: 上記単体テストが緑で、配送同等性・終了復帰・再入ガードが自動検証される
  - _Requirements: 2.4, 1.4, 4.3_
  - _Depends: 4.3_

- [x] 5.2 統合テスト
  - 宣言的ウィンドウ生成フロー（コンポーネント spawn→生成→レジストリ保持→表示）がクラス未登録エラーなく成立することを検証する
  - VSync 起床→tick 13 本実行→再待機の 1 周が成立し、実行順序が不変であることを検証する
  - 最後の窓クローズで run が panic・ハングなく復帰し、32bit 可搬性・host-32 別プロセス構成が不変であることを確認する
  - 完了状態: 生成・tick・終了の統合テストが緑で、回帰なく一連が成立する
  - _Requirements: 2.1, 4.1, 4.2, 4.5, 1.3, 1.5, 6.2_
  - _Depends: 4.4_

- [ ] 5.3 E2E・手動検証（examples / areka）
  - 複数ウィンドウ・ULW/DComp 両モードの代表 example が新 facade 上で回帰なく描画することを確認する
  - areka 本体でシェル＋バルーン表示・ドラッグ移動・**ダブルクリック終了**（ライブラリ 0.0.5 内蔵 CS_DBLCLKS）が成立することを確認する
  - 背景プール（world.spawn 経路）による UI 構築が温存され機能することを確認する
  - 完了状態: 代表 example と areka が手動確認で回帰なく動作し、ダブルクリック終了と背景プール温存が実機で確認される
  - _Requirements: 6.1, 2.5, 3.4_
  - _Depends: 4.4_

## Implementation Notes

- **【5.3 実機回帰修正】wndproc は `new_ex`（再入可）で生成する（design の `new_checked_ex` 採用を上書き）**: design.md は要件 4.3 の二重防御として `util::Window::new_checked_ex`（`FnMut`＋内部 `RefCell` で wndproc 再入阻止）を採用したが、**5.3 手動 E2E で areka のドラッグがちらつく回帰**を発見。根本原因: ドラッグは `WM_MOUSEMOVE`→World 借用解放→`guarded_set_window_pos`→`SetWindowPos`→**同期発火する WM_WINDOWPOSCHANGED に wndproc が再入し WindowPos を echo-bypass 更新する**設計（mouse_move.rs:399-414 / window_pos.rs の `is_self_initiated` echo 機構）に依存。`new_checked_ex` の RefCell が同一窓 wndproc の再入を阻止 → 入れ子 WM_WINDOWPOSCHANGED が DefWindowProc 送りになり WindowPos 更新が失われ ECS と OS の位置がズレてちらつく。旧 `ecs_wndproc`（素の extern fn＝再入可）はこの再入で正しく動いていた。**修正**: factory を `new_ex`（`Fn`・RefCell なし＝再入可）へ、`make_wndproc` を `FnMut`→`Fn` へ。tick 二重実行防止は ECS 側ガード（`IS_TICK_FLUSH_IN_PROGRESS`＋World `try_borrow_mut`）＋make_wndproc の `try_borrow` 安全スキップで担保（旧経路と同じ単一防御＝実績あり・要件 4.3 維持）。先進坑はヘッドレスで「再入 blocked」を検証していたが areka ドラッグは「再入 needed」のケースで盲点だった。crate ソース（`util/window.rs`）で `new_checked_ex` が内部 `RefCell::new(wndproc)` で再入検出する一方 `new_ex` は素通しと確認済。実機で areka ドラッグ滑らか・ダブルクリック終了 exit 0 を確認（開発者 2026-06-30）。**design.md の該当記述（new_checked_ex 採用）は kiro-complete の doc 同期で new_ex へ要修正。**

- **タスク順序の再整列（4.5 は 5.x の後・開発者 steer から導出）**: 開発者決定「旧コードは移行が確認できるまで残す（撤去は最終）」。移行確認＝5.3 手動 E2E。よって legacy 撤去（4.5）は検証前に安全網を外さぬよう **5.1→5.2→5.3→4.5** の順で実行する（4.5 の依存は 4.4 のみゆえ 5.x 先行は依存的に可）。5.3 は手動 E2E ゆえ開発者確認の gate（STOP の可能性）。5.x で新経路の不具合が出たら legacy を reference に修正可能。
- **dcomp_demo 書換（4.4・記録）**: 旧 dcomp_demo は命令的 WinMessageHandler ＋ create_window による 3D カードめくり記憶ゲーム。ECS draw 層に 3D カードフリップ widget が無く faithful port 不可ゆえ、設計意図（新 API 実演）に沿って **宣言的 DComp デモ**（CompositionMode::DComp 窓＋Rectangle/Label カードグリッド＋spawn_ui_local 実演）へ recast。ゲームロジックは意図的にドロップ（example ＝手動検証補助でテスト代替でない）。module doc に経緯記載。要 5.3 で代表 example の回帰確認。
- **Phase 4 = 逐次 live cutover（開発者決定 2026-06-30）**: 4.1→4.2→4.3→4.4→4.5 を 1 タスクずつ implementer→review→commit。各コミットは「コンパイル緑＋lib テスト緑＋boundary」で検証。ランタイム/example 実行検証は **4.4（build）＋5.3（手動E2E）に集約**（旧経路と新経路の共有 lifecycle ハンドラ＋create 経路が flip するため、4.1〜4.4 の中間コミットは一時的に実行不可・squash-merge で消える）。design Migration Strategy 順序と一致。「旧コードは reference として保持・実撤去は 4.5」の共存 steer は維持。
- **【要対応 gap】factory が Window.parent 未転送（4.3 レビューで発見・3.4 由来）**: 旧 create_windows は `Window.parent` を `CreateWindowExW` の parent 引数へ渡していたが、新 EcsWindowFactory（3.4）は `WindowType::TopLevel` 固定で parent を無視。areka シェル+バルーンが親子窓か独立トップレベルかで影響。**5.3 E2E 前（4.4 追従改修時 or 専用修正）に factory の parent 転送可否を確認・必要なら補修すること**（ライブラリ `new_checked_ex` の parent 受け渡し口を確認）。4.3 範囲外ゆえ 4.3 はブロックしない。
- **4.3 解釈（全結線・cutover 心臓部）**: 「WinApp::run 全結線」は run の tick/block_on 結線だけでなく、新経路を実働させる create_windows 切替＋reconcile 結線を含む（これ無しでは「new→world→run で生成/tick/終了が一体動作」が成立しない＝design「全結線」の含意）。境界 "WinApp" を超えて window_system.rs / ecs/world を最小限触る（boundary 拡張・記録済）。設計判断:
  - **(1) self-weak resource**: create_windows（&mut World・bevy World しか持たない）が factory 用の `Weak<RefCell<EcsWorld>>`（WndState 用）を得る経路として、ECS 層に NonSend リソース（例 `EcsWorldSelfRef(Weak<RefCell<EcsWorld>>)`・ecs/world 定義＝ECS→ECS）を新設し WinApp が new()/run() で注入。create_windows はこれを読み factory へ渡す（唯一の上向きエッジ create_windows→factory は設計公認）。
  - **(2) create_windows 切替**: window_system.rs の旧 create_windows 本体は `create_windows_legacy`（#[allow(dead_code)]・共存温存・撤去 4.5）へ退避し、新 create_windows は EcsWindowFactory 経由（self-weak resource 未注入時は何もしない安全動作＝旧 WinThreadMgr 経路でも panic しない）。schedule 登録名は据置。
  - **(3) reconcile 結線**: WinApp が `world.add_systems(<late schedule>, reconcile_window_registry::<Window<WndState>>)`（runtime→World・ECS schedule 定義に runtime 型を持ち込まない）。RemovedComponents<Window> が tracker クリア前に読める遅め schedule に配置（実装者が try_tick_world の clear_trackers タイミングを確認し決定）。
  - **(4) run() 結線**: VsyncEventBridge::new()（WinApp 所有・drop で stop→join）→ AsyncTickTask::spawn(bridge.event().clone(), Rc::downgrade(&world)) → block_on(ShutdownPolicy::shutdown_future(self.shutdown.clone()))。spawn_ui_local は既存。
  - **(5) dead_code 解除**: 結線された building block（MessageLoopDriver/VsyncEventBridge/AsyncTickTask/ShutdownPolicy/WinApp.shutdown/factory/registry reconcile）の scoped #[allow(dead_code)] を外す（想定 3 件警告も解消）。
  - 検証: 旧経路（WinThreadMgr）と新経路の二重生成を避けるため、新 create_windows は self-weak 未注入時 no-op。full run 復帰・実窓表示の E2E は 4.4（examples 切替後）＋5.3（手動）。4.3 自体はコンパイル緑＋lib テスト緑＋（可能なら）headless な run 部品テスト。
- **4.2 解釈（共存遵守＋building block）**: 新終了シグナル＝`event_listener::Event` を **WinApp（runtime）所有**（design:387 は「ECS 層所有」だが、その根拠は reconcile が ECS の場合の上向き依存回避。本坑は WindowRegistry/reconcile を runtime 配置（3.3 決定）ゆえ notify が runtime→runtime で完結し上向き依存なし＝WinApp 所有が一貫・最小）。WinApp::new() で Event 生成→WindowRegistry を World へ確保（既存なら流用）→`set_shutdown_hook(notify Event)` を注入。run の `block_on(shutdown_future)` 結線は **4.3**。旧 `WM_LAST_WINDOW_DESTROYED`/`message_window`/`App::on_window_destroyed` PostMessage 経路は**温存**（旧 WinThreadMgr 専用・撤去は 4.5）＝app.rs に触れない。4.2 検証は「Event notify→shutdown future/listener 完了」「registry 空 hook→Event 発火」をヘッドレス単体で（full run 復帰は 4.3/5.3）。tail race は終了時 notify(usize::MAX)＋run 側 listen 先行 arm で回避（4.3 で本結線）。
- **4.1 解釈（共存遵守）**: 新経路に必須なのは **WM_CLOSE 反転（DestroyWindow 直叩き→対象 Entity の despawn 要求）** のみ。WM_NCCREATE の GWLP 手詰めと WM_NCDESTROY の ECS 後始末は **旧経路専用コード**（新経路はライブラリが NCCREATE/NCDESTROY を所有し wintf の当該ハンドラを呼ばない・dispatch 表からも除外済 3.2）。ゆえに 4.1 ではこれらを**撤去せず温存**し、実撤去は 4.5 へ（task 4.1 bullet 2/3 の「撤去」は 4.5 へ繰延・開発者の keep-old-code steer 優先）。WM_CLOSE の despawn→`RemovedComponents<Window>`→reconcile→registry drop→DestroyWindow の完結は reconcile 結線（4.3）後ゆえ、4.1 単体検証は「WM_CLOSE ハンドラが entity を despawn する」まで（full close→destroy は 4.3/5.3 で検証）。同期再入の二重借用回避は make_wndproc の try_borrow safe-skip（3.1）＋reconcile が破棄手続きで ECS 後始末を持たないことで担保。

- runtime の各 building block（MessageLoopDriver / VsyncEventBridge / AsyncTickTask）は WinApp::run へ未結線の間、scoped `#[allow(dead_code)]` を帯びる。task 4.3 の結線後に allow を外す（dead_code 警告 3 件は想定内・新規警告ではない）。
- tick 再入ガード `IS_TICK_FLUSH_IN_PROGRESS` は `ecs::world::engage_tick_flush_guard()`（RAII・進行中なら `None`）/ `is_tick_flush_in_progress()` で再利用可能（task 2.3 で追加）。legacy `try_tick_on_vsync` は不変のまま並存。
- workspace cargo を回す前に `git submodule update --init`（vendors/pasta）が必要（worktree では未populate のことがある）。ビルド/テストは PowerShell で実行（Git Bash の coreutils `link.exe` が MSVC link を遮蔽する）。
- **task 3.1 配置是正（3.2 で実施）**: 3.1 は `dispatch_window_message` を `runtime/wndproc_bridge.rs` に置いたが、design.md:188/443 は ECS 層（`ecs/window_proc/mod.rs`）配置を指定。旧 `ecs_wndproc`（ECS層）が同関数を共有呼びするには ECS 層配置が必須（ecs→runtime の上向き依存禁止・design:54）。よって 3.2 で `dispatch_window_message` を `ecs/window_proc/mod.rs` へ移設し、`runtime/wndproc_bridge.rs::make_wndproc` はそれを呼ぶよう繋ぎ替える（WndState/make_wndproc は維持・3.1 テストは緑のまま）。
- **旧経路の共存維持（開発者決定 2026-06-30）**: 「最終的には完全撤去。ただし移行が確認できるまで旧コードを残す（知見転記漏れの保険）」。よって 3.2 では旧グローバル/GWLP 解決（`ECS_WORLD`/`SendWeak`/`set_ecs_world`/`try_get_ecs_world`/`get_entity_from_hwnd`）と `ecs_wndproc` を**撤去せず存続**させる。`ecs_wndproc` は entity/world を自己解決して新設 `dispatch_window_message` へ委譲する薄いシムへ縮約（業務ロジックは移行済みハンドラ側に集約）。実際の撤去は 4.5（legacy teardown）へ寄せる。各フェーズで lib ビルド＋既存テスト＋example 緑を維持（design の Phase チェックポイント遵守）。
- **3.4 EcsWindowFactory = building block 方式（開発者決定 2026-06-30）**: factory（`Window<WndState>` 生成・CompositionMode→ex_style・生成後 style/pos/title 反映・WindowRegistry 格納・graphics 用 WindowHandle 連携）を実装＋ヘッドレステストのみ。**旧 create_windows は live のまま温存**（`ecs/window/window_system.rs` に触れない・examples 緑維持）。live cutover（schedule 切替・reconcile 結線・WM_CLOSE→despawn 反転）は 4.1/4.3 でまとめて行う。factory は #[allow(dead_code)]（2.x/3.1/3.3 と同じ dead-building-block パターン）。理由: 即 cutover すると WM_CLOSE 握り潰し＋reconcile 未結線で window close/exit が 4.1/4.3 まで不能になり examples が壊れるため。生成時の同期メッセージ（WM_CREATE 等）は schedule 借用中ゆえ closure の try_borrow 失敗で安全スキップ（panic なし）。4.3 結線用シグネチャ: `EcsWindowFactory::create_window(world: &mut World, entity: Entity, ecs_world: Weak<RefCell<EcsWorld>>)`（排他システム `&mut World` 想定・World 読取りは new_checked_ex 前に解放・NonSend `WindowRegistry` 未挿入時は遅延 init の安全網あり）。CW_USEDEFAULT 含む座標は SetWindowPos スキップ（ライブラリが CW_USEDEFAULT 生成済・design:409）。
- **WindowRegistry 配置（3.3）= runtime 層**: `Window<WndState>`（!Send・ライブラリ型）を保持し WndState（3.1・runtime）に依存するため runtime に置く（設計 File Structure も window_factory 等 Window<S> building block を runtime/ へ集約）。NonSend リソースとして World へ挿入予定。reconcile（`RemovedComponents<Window>` 駆動）も runtime 定義とし、schedule への結線は WinApp（runtime→World）が 4.3 で行う（ECS→runtime の上向き依存を作らない）。唯一の上向きエッジ create_windows(ECS)→factory/registry(runtime) は設計公認で 3.4 の領分。3.3 では旧 create_windows / WindowHandle / process_singleton に触れない（共存維持）。`reconcile_window_registry<W: 'static>` は generic（ヘッドレステスト seam）ゆえ、4.3 の schedule 登録時は具体型 `reconcile_window_registry::<wintf_winmsg_executor::util::Window<WndState>>`（turbofish）で monomorphize して登録する（in-code 文書化済 window_registry.rs:105-107）。`shutdown_hook: Option<Box<dyn Fn()>>` の実 Event（event_listener）注入は 4.2 領分。
- **WM_NCCREATE/WM_NCDESTROY のみ 3.2 で非移行**: この 2 つは entity 確立/解体の lifecycle 特例で「窓の畳み方の反転」（task 4.1）の領分。現行シグネチャ `(hwnd,message,wparam,lparam)` のまま `ecs_wndproc` から直呼びし、`dispatch_window_message` 表には含めない。NCCREATE は lpCreateParams から entity を確立する message ゆえ引数化不可。3.2 の (world,entity) 引数一様移行の対象は entity 解決後メッセージ（WM_CLOSE/ERASEBKGND/PAINT/DISPLAYCHANGE/WINDOWPOSCHANGED/DPICHANGED/mouse/keyboard）。**WM_CLOSE は移行対象**（dispatch 表経由・本体は world/entity を `_` で無視するが統一シグネチャに従う・entity 確立解体を伴わないため特例ではない）。
