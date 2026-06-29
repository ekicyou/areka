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

- [ ] 1.2 runtime モジュール新設と WinApp facade の骨格
  - 新しい runtime レイヤを設け、`WinApp` を UI スレッド基盤の owner として導入する（旧 `WinThreadMgr` 置換の新公開 facade）
  - COM 初期化（`COINIT_MULTITHREADED`）・DPI awareness 設定・`EcsWorld`（`Rc<RefCell<EcsWorld>>`）生成を WinApp に集約する（旧 process_singleton / WinThreadMgr から移設）
  - 公開 API（new / world / UI async 投入口 / run）のシグネチャを確定し、run は最小スタブとする
  - 先進坑コードを流用せず README 知見から実装する
  - 完了状態: `WinApp::new()` が COM/DPI を初期化し world ハンドルを返す（既存レガシーは未撤去のまま並存）
  - _Requirements: 6.3, 5.2, 7.4_
  - _Boundary: WinApp_

## 2. Core: メッセージループと tick 駆動層

- [ ] 2.1 (P) メッセージループ層の委譲
  - 自作 `PeekMessageW` ポンプを撤去し、ライブラリの `block_on` / `MessageLoop::run(filter)` へ委譲する
  - filter は原則 Forward とし、自前 `WM_VSYNC` pop 分岐を持たない（wake メッセージはライブラリ保護に委ねる）
  - 完了状態: WinApp が OS メッセージをライブラリのループ経由で取りこぼしなくウィンドウ手続きへ配送する最小経路が動作する
  - _Requirements: 1.1, 1.2_
  - _Boundary: MessageLoopDriver_

- [ ] 2.2 (P) VSync 起床ブリッジ
  - 専用 VSync スレッドが `DwmFlush` で vblank を検出し、共有 `event_listener::Event` を全リスナ起床で notify する
  - Event は WinApp が所有し、スレッド生存期間を stop→join の順序規律で管理する
  - 周期はモニターのリフレッシュレートに追従し、固定 16.67ms を前提にしない
  - 完了状態: vblank ごとに Event が notify され、UI スレッド側の待機タスクを起床できる
  - _Requirements: 4.1, 4.4_
  - _Boundary: VsyncEventBridge_

- [ ] 2.3 60Hz async tick タスク
  - `spawn_local` した UI スレッド async タスクが、起床通知を待って 1 フレーム分の ECS tick（13 本スケジュール）を実行し再待機するループを実装する
  - 13 本の構成・実行順序は不変とし、既存の tick 実行経路をそのまま呼ぶ
  - ECS 再入ガード（tick フラッシュ進行中フラグ）を安全側に残置し、ライブラリの wndproc 再入防止と二重防御させる
  - 完了状態: 起床ごとに 13 本 tick が 1 周実行され、再入時は借用失敗で安全スキップして二重 tick が起きない
  - _Requirements: 4.2, 4.3, 4.5_
  - _Boundary: AsyncTickTask_
  - _Depends: 2.2_

## 3. Core: ウィンドウ生成・配送・所有

- [ ] 3.1 (P) ウィンドウ手続きブリッジと配送純関数
  - ウィンドウ手続きクロージャを構築し、共有状態 S に World への弱参照と当該 Entity を保持させる（生成時に確定・以後不変）
  - 旧 `ecs_wndproc` の 30 種超メッセージ振り分け表を、World と Entity を引数で受け取る純関数（dispatch_window_message 相当）へ移設する
  - クロージャは弱参照を upgrade し、借用失敗・破棄中は安全スキップする
  - 完了状態: 代表メッセージがクロージャ経由で配送純関数へ橋渡しされ、None 時はライブラリの既定手続きへ委譲される
  - _Requirements: 2.3, 2.4_
  - _Boundary: EntityWndprocBridge_

- [ ] 3.2 ウィンドウ手続きハンドラの統一シグネチャ移行
  - 既存ハンドラ群（lifecycle / mouse / keyboard / window_pos / dpi 等・計 31 箇所の自己解決）を、World と Entity を引数で受け取る統一シグネチャへ機械的に移行する
  - グローバル World 参照（`OnceLock` 保持の弱参照）と HWND→Entity 解決（GWLP_USERDATA 依存）を撤去する
  - 各ハンドラ内部の業務ロジックは不変に保つ
  - 完了状態: 全ハンドラが引数経由で World/Entity を受け取り、旧グローバル参照・GWLP_USERDATA 解決への参照が残らずビルドが通る
  - _Requirements: 2.3, 2.4, 5.2_
  - _Boundary: window_proc handlers_
  - _Depends: 3.1_

- [ ] 3.3 (P) WindowRegistry（NonSend 所有とリコンサイル）
  - 生成済みウィンドウハンドルを Entity キーで保持する NonSend リソースを実装する（!Send を保持＝UI スレッド束縛・Send 偽装はしない）
  - `Window` コンポーネント破棄を検知するリコンサイルで該当要素を drop し、ハンドル破棄（DestroyWindow）を Entity ライフサイクルに一致させる
  - 除去後に空になったら終了シグナルを発火できる接点を用意する
  - 完了状態: Entity 破棄でレジストリ要素が drop されて窓が破棄され、空判定で終了通知をトリガできる
  - _Requirements: 1.3, 2.1, 5.2_
  - _Boundary: WindowRegistry_

- [ ] 3.4 ECS ウィンドウ生成の移行
  - 宣言的ウィンドウ生成を、自作 `CreateWindowExW` 直呼びからライブラリの再入安全なウィンドウ生成（new_checked_ex 相当）へ置換する
  - 透過合成モードに応じた拡張スタイル（ULW=LAYERED / DComp=NOREDIRECTIONBITMAP）の受け渡し口を用い、生成後にスタイル・座標・タイトルを反映する初期化を行う
  - 生成したハンドルを WindowRegistry へ格納し、CS_DBLCLKS はライブラリ内蔵に委ねて wintf 側補填を設けない
  - 完了状態: 宣言的にスポーンした窓がライブラリ経由で生成され、レジストリ保持・スタイル反映済みで表示される
  - _Requirements: 2.1, 2.2, 2.5_
  - _Boundary: EcsWindowFactory_
  - _Depends: 3.1, 3.3_

## 4. Integration: 結線・終了規律・利用側追従・撤去

- [ ] 4.1 窓の畳み方の反転
  - クローズ要求ハンドラを「ウィンドウ破棄の直叩き」から「対象 Entity の除去要求（ECS コマンド enqueue）」へ反転する
  - 破棄完了手続きでは ECS 後始末（despawn / 借用）を持たせず、レジストリ要素 drop 駆動の破棄と整合させる（同期再入時の二重借用回避）
  - 非クライアント生成手続きの GWLP_USERDATA 手詰めを撤去する
  - 完了状態: クローズ操作が除去要求として処理され、レジストリ drop により窓が破棄されてパニック・二重借用が起きない
  - _Requirements: 2.3, 1.3_
  - _Boundary: window_proc lifecycle, WindowRegistry_
  - _Depends: 3.2, 3.3, 3.4_

- [ ] 4.2 終了規律（ShutdownPolicy）の結線
  - 終了シグナルを ECS 層が所有する `event_listener::Event` とし、WinApp 構築時に下向き注入する（上向き依存を作らない）
  - 最後の窓が消えた（レジストリ空）時点でシグナルを notify し、run が待つ shutdown future を完了させて正常復帰させる
  - 旧 `WM_LAST_WINDOW_DESTROYED` / message_window 経路を撤去し、tail race 回避に終了時 notify を補う
  - 完了状態: 全ウィンドウ破棄でシグナルが発火し、ループが先行 quit せず future 完了で panic なく復帰する
  - _Requirements: 1.3, 1.4, 1.5_
  - _Boundary: ShutdownPolicy, WinApp_
  - _Depends: 4.1_

- [ ] 4.3 WinApp::run の全結線と UI スレッド async
  - run で async tick タスクを `spawn_local` し、shutdown future を `block_on` し、VSync ブリッジ・レジストリ・Event の生存期間を WinApp が所有する
  - UI スレッド単一の async 投入口（spawn_ui_local 相当）を提供し、手組み executor（async-executor + spawn_normal）経路を置換する（tokio 非依存・!Send future 許容）
  - 完了状態: `WinApp::new → world → run` の最小フローでメッセージループ・tick・終了が一体で動作する
  - _Requirements: 1.1, 1.3, 3.1, 3.2, 3.3, 6.1, 6.3_
  - _Boundary: WinApp_
  - _Depends: 2.1, 2.3, 4.2_

- [ ] 4.4 利用側（examples・areka）の新 API 追従
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

- [ ] 5.1 単体テスト
  - 配送純関数が代表メッセージ（ダブルクリック・WINDOWPOSCHANGED・DPICHANGED・非クライアント破棄）で旧手続きと同等の結果を返すことを検証する
  - 終了シグナルで shutdown future が完了し、ループ相当が panic せず復帰することを検証する
  - 再入ガードが tick 中に立ち、スコープ離脱で戻ることを検証する
  - 完了状態: 上記単体テストが緑で、配送同等性・終了復帰・再入ガードが自動検証される
  - _Requirements: 2.4, 1.4, 4.3_
  - _Depends: 4.3_

- [ ] 5.2 統合テスト
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
