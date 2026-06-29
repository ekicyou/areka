# Requirements Document

## Project Description (Input)

wintf のメッセージループ・ウィンドウ起動・UI スレッド async は、現状ほぼ自作（`WinThreadMgr::run()` の `PeekMessageW` ポンプ、`create_window` ＋ `GWLP_USERDATA` 手詰め、`async-executor::Executor` ＋ `bevy_tasks::TaskPool` ＋ `mpsc` の手組み）である。Windows 低レベルレイヤーは罠が多く、自作コードは不正確な挙動を抱えるリスクを持つ。wintf のコードベースがこれ以上膨らむ前に、Windows 低レベル知見に基づく洗練版の外部クレート `wintf-winmsg-executor`（`winmsg-executor` フォーク・v0.0.3）ベースへ置き換える。具体的には、(1) メッセージポンプを `MessageLoop`/`block_on` へ、(2) ウィンドウ生成を `util::Window<S>`（`new_ex`/`new_checked_ex`）へ、(3) UI スレッド async を `spawn_local`/`block_on` へ写像し、(4) 60Hz ECS tick を `event_listener` ブリッジ＋ async tick タスクへ移行する。tokio 非依存を維持し、deprecated レガシー（`wndproc`/`win_thread_mgr`/`win_message_handler`）を撤去し、emo2-boot を含む既存 examples が回帰なく動くことをゴールとする。本坑は先進坑 `pilot/wintf-winmsg-executor` の go 判定（開発者承認 2026-06-29 取得済み）を前提依存とする。

## Introduction

本仕様は、wintf の UI スレッド基盤（メッセージループ・ウィンドウ生成・UI スレッド async・60Hz ECS tick 起床）を、Windows 低レベル知見に基づく外部クレート `wintf-winmsg-executor`（v0.0.5・CS_DBLCLKS 内蔵版）ベースへ置き換える横断リファクタである。自作のメッセージポンプ・`GWLP_USERDATA` 手詰め・手組み executor を撤去し、挙動の正しさを高めつつトラブル余地を縮小する。tokio 非依存を維持し、スレッド跨ぎ起床は `event_listener` で実現する。本坑は先進坑 `pilot/wintf-winmsg-executor` の go 判定（開発者承認・2026-06-29 取得済み）を前提依存として持つ。先進坑で検証済みの事実（起床安定性・再入整合・状態アクセス・清掃終了）は先進坑 README を正本とし、本仕様では二重化しない。

## Boundary Context

- **In scope**:
  - メッセージポンプの `MessageLoop`/`block_on` への置換（quit 経路の引き継ぎを含む）。
  - ウィンドウ生成の `util::Window<S>`（`new_ex`/`new_checked_ex`）への移行と、HINSTANCE 取得・クラス登録の責務整理。
  - ウィンドウ手続きにおける Entity 配送の新基盤上での再構築（`GWLP_USERDATA` 手詰めの撤去）。
  - UI スレッド async の `spawn_local`/`block_on` への移行。
  - 60Hz ECS tick の `event_listener` ブリッジ＋ async tick タスクへの移行（ECS 再入ガードとライブラリの nested-message 再入防止の整合）。
  - deprecated レガシー（`wndproc`／`win_thread_mgr`／`win_message_handler`）の撤去。
  - **公開 API 方針（議題①確定・2026-06-29）**: `WinThreadMgr` の公開 API（`new`/`world`/`run` 等）を温存せず、`wintf-winmsg-executor` ベースの新 facade（新公開 API）へ全面置換する。`WinThreadMgr` 自体を撤去対象とし、全 examples ＋ areka 本体を新 API へ追従改修する。
  - 既存 examples（emo2-boot 系を含む）の新 API への追従改修と回帰確認。
- **Out of scope**:
  - 背景重処理用 `bevy_tasks::TaskPool`（`WintfTaskPool` ＝ ECS が `EcsWorld::new()` で Resource として生成・管理する常駐ワーカープール。areka の UI 構築入口 `world.spawn(CommandSender)` を含む）の廃止・再設計（必要なら別仕様。UI スレッドとは無縁な別レイヤで、`spawn_local`（UI スレッド単一）では代替不可）。
  - 透過合成方式（ULW/DComp 切替）のロジックそのものの変更（拡張スタイル受け渡し口を使うのみ）。
  - ECS スケジュール（13 本）の構成・順序の変更。
  - emo2 互換機能の新規実装（M1 emo2-boot ユニット側の領分）。
- **Adjacent expectations**:
  - 利用側の窓生成系 spec（`areka-P0-window-placement` / `areka-P0-surface-engine`）は旧 API（`WinThreadMgr`/`create_window`）に依存しているため、移行後 API への追従を要する。本仕様は追従に必要な公開インターフェースを提供する責務を負うが、利用側の追従実装そのものは負わない。
  - host-32 は別プロセスであり本件 UI スレッドとは別系統。32bit 可搬性は崩さない。

## Requirements

### Requirement 1: メッセージループ層の置換

**Objective:** wintf 開発者として、自作メッセージポンプを `wintf-winmsg-executor` のメッセージループへ置き換えたい。それにより、Windows 低レベルの罠を抱えた自作コードを撤去し、洗練された基盤の上で正しいポンプ挙動を得るため。

#### Acceptance Criteria

1. When wintf の UI スレッドが起動する, the wintf メッセージループ層 shall 自作の `PeekMessageW`/`TranslateMessage`/`DispatchMessageW` ポンプではなく `wintf-winmsg-executor` の `MessageLoop`/`block_on` を用いてメッセージを処理する。
2. While メッセージループが実行中である, the wintf メッセージループ層 shall OS から届くウィンドウメッセージを取りこぼしなくウィンドウ手続きへ配送する。
3. When アプリケーションの終了が要求される, the wintf メッセージループ層 shall 旧来の終了経路（`PostQuitMessage` 相当）と同等の終了動作を提供し、未完了の UI スレッド async タスクを完了させてからループを抜けてプロセスを清掃終了する。
4. If メッセージループが未完了の async タスクより先に終了しようとする, then the wintf メッセージループ層 shall パニックせずに清掃終了する。
5. The wintf メッセージループ層 shall 終了時にハングまたはパニックを起こさない。

### Requirement 2: ウィンドウ生成・ウィンドウ手続き層の移行

**Objective:** wintf 開発者として、自作のウィンドウ生成と `GWLP_USERDATA` 手詰めを `util::Window<S>` ベースへ移行したい。それにより、状態保持の手詰めを撤去し、ライブラリが束ねる安全な状態アクセス機構へ統一するため。

#### Acceptance Criteria

1. When wintf がウィンドウを生成する, the wintf ウィンドウ生成層 shall 自作 `create_window` ＋ `GWLP_USERDATA` 手詰めではなく `util::Window<S>`（`new_ex`/`new_checked_ex`）を用いてウィンドウと共有状態を束ねる。
2. Where 透過合成のために拡張ウィンドウスタイルの指定が必要である, the wintf ウィンドウ生成層 shall `new_ex`/`new_checked_ex` の拡張スタイル受け渡し口を通じて `WS_EX_NOREDIRECTIONBITMAP` を指定できる。
3. When ウィンドウメッセージが届く, the wintf ウィンドウ手続き層 shall `GWLP_USERDATA` への手詰めなしに、ライブラリが提供する状態アクセス機構を通じて共有状態（ECS world 相当）へ安全にアクセスする。
4. When ウィンドウメッセージが ECS world への配送を要する, the wintf ウィンドウ手続き層 shall 旧 `ecs_wndproc` の Entity 単位の配送と同等の配送結果を新基盤のウィンドウ手続き上で提供する。
5. The wintf ウィンドウ生成層 shall HINSTANCE 取得とウィンドウクラス登録の責務を新基盤に整合する形で扱い、重複登録や未登録によるウィンドウ生成失敗を起こさない。

### Requirement 3: UI スレッド async 実行層の移行

**Objective:** wintf 開発者として、手組みの UI スレッド async 実行（`async-executor` ＋ `mpsc` drain）を `wintf-winmsg-executor` の async 実行へ移行したい。それにより、手組み executor のトラブル余地を縮小し、メッセージループと統合された UI スレッド async を得るため。

#### Acceptance Criteria

1. When UI スレッド上で async タスクを実行する必要がある, the wintf UI スレッド async 層 shall 手組みの UI スレッド async 実行器（`async-executor::Executor`＝`executor_normal` ／ `spawn_normal` 経路）ではなく `wintf-winmsg-executor` の `spawn_local`/`block_on` を用いてタスクを実行する。
2. While UI スレッド async タスクが待機状態にある, the wintf UI スレッド async 層 shall メッセージループの進行を妨げずに当該タスクを起床可能な状態で保持する。
3. The wintf UI スレッド async 層 shall tokio に依存せず、`Send`/`Sync` を要求しない future を UI スレッド上で実行できる。
4. The wintf UI スレッド async 層 shall 背景ワーカープール `WintfTaskPool`（`bevy_tasks::TaskPool` ＋ `world.spawn(CommandSender)` ＋ `CommandSender` mpsc drain・ECS が Resource として管理）を移行対象に含めず、現行構成のまま温存する（議題②確定・2026-06-29。当該プールは UI スレッドとは別の常駐ワーカー群で走り、`spawn_local` では代替できないため別レイヤとする）。

### Requirement 4: 60Hz ECS tick 起床ブリッジの移行

**Objective:** wintf 開発者として、VSync スレッドからのメッセージ駆動による ECS tick を、`event_listener` ブリッジ＋ async tick タスクへ移行したい。それにより、メッセージ pop 方式の再入経路を構造的に減らし、スレッド跨ぎ起床を tokio 非依存で実現するため。

#### Acceptance Criteria

1. When VSync 同期スレッドがフレーム境界（`DwmFlush` の vblank）を検出する, the wintf tick 起床ブリッジ層 shall `event_listener` を通じて UI スレッドの async tick タスクへ起床通知を送る。
2. When 起床通知を受け取る, the wintf tick 起床ブリッジ層 shall UI スレッド上で 1 フレーム分の ECS tick（13 本のスケジュール）を実行し、その後再び次の起床を待機する。
3. While ECS tick の実行中にウィンドウメッセージが発生する, the wintf tick 起床ブリッジ層 shall ライブラリの nested-message 再入防止機構と ECS の再入ガードが衝突しないようにし、デッドロックまたは二重 tick を起こさない。
4. The wintf tick 起床ブリッジ層 shall フレーム周期をモニターのリフレッシュレートに追従させ、固定の 16.67ms 周期を前提とした実装に依存しない。
5. The wintf tick 起床ブリッジ層 shall 起床機構の置き換えにあたり ECS スケジュールの構成（13 本）および実行順序を変更しない。

### Requirement 5: レガシーコードの撤去

**Objective:** wintf 開発者として、置換対象の deprecated レガシー実装を撤去したい。それにより、負の遺産の累積を防ぎ、新基盤への移行を完了させるため。

#### Acceptance Criteria

1. When 新基盤への移行が完了する, the wintf shall deprecated レガシー実装（`wndproc`／`win_thread_mgr`／`win_message_handler` 相当）および公開 facade `WinThreadMgr` を撤去する。
2. The wintf shall 撤去後にレガシー実装および旧 `WinThreadMgr` API への参照を残さず、ビルドおよび既存テストが成功する。

### Requirement 6: 既存 examples の回帰防止

**Objective:** wintf 開発者として、基盤置換後も既存 examples（emo2-boot 系を含む）が同等に動作することを保証したい。それにより、置換が利用側へ回帰を持ち込まないことを確認するため。

#### Acceptance Criteria

1. When 既存 examples（emo2-boot 系を含む）および areka 本体を新 facade（新公開 API）へ追従改修した上で実行する, the wintf shall 置換前と同等の動作を回帰なく提供する。
2. The wintf shall 置換後も 32bit 可搬性を崩さず、host-32 を別プロセスとする現行構成を変更しない。
3. Where 利用側（全 examples ＋ areka 本体、および `areka-P0-window-placement` 等の窓生成系 spec）が旧 `WinThreadMgr`/`create_window` API に依存していた, the wintf shall 旧 API を新 facade（新公開 API）へ置き換え、利用側が追従するための公開インターフェースを提供する。

### Requirement 7: 採用クレートのバージョン固定と前提依存

**Objective:** wintf 開発者として、極めて初期の採用クレートを安定的に扱い、二坑モデルの前提依存を遵守したい。それにより、API 不安定リスクを抑え、確定済みの方向に沿って本坑をクリーンに掘るため。

#### Acceptance Criteria

1. The wintf shall 採用クレート `wintf-winmsg-executor` をバージョン v0.0.5（共有ウィンドウクラスに `CS_DBLCLKS` ＋既定カーソルを内蔵した版・フォーク上流で修正済み）に固定（pin）して取り込む。これによりダブルクリック有効化を wintf 側の後付け（`DblClkClassFixup`）なしでライブラリ側が提供する。
2. The wintf shall スレッド跨ぎ起床のために `event_listener` クレートを依存に追加する。
3. The wintf 開発プロセス shall 先進坑 `pilot/wintf-winmsg-executor` の go 判定（開発者承認・取得済み）を本坑着手の前提依存として満たす。
4. The wintf 実装 shall 先進坑コードをコピー流用せず、先進坑 README の検証結果（知見）を参照して一から実装する。
