# Brief: areka-P0-actor-foundation

> **種別**: 本坑（main）。⓪ ghost 帰属の**横断基盤**（`areka-P0-parser-foundation` の並行モデル版）。M-boot ユニット・kanade/sakura/seriko/emo-present/ghost-setup の先行依存。
> **正本**: 並行モデル＝roadmap「横断データ構造」節・記憶 areka-concurrency-model（各エンジン＝チャンネル通信のアクター・エンジンインスタンスごと独立スレッド・render/window は UI スレッド固定）。

## Problem

M1 の並行モデルは「各エンジン＝独立スレッドのアクター・相互通信はチャンネル・**I/O 契約＝チャンネルのメッセージ型**」と確定しているが、**共通の通信基盤（プリミティブ）が存在しない**。放置すると kanade/sakura/seriko/emo が各自の channel 流儀（型・生存期間・停止手順・エラー伝搬）を発明し、M-boot 統合で噛み合わない＝典型的な「手前で考えないと詰む」要素。特に **UI スレッド（emo/render・窓）への配送**は自明でない——message pump 中のスレッドは `recv()` でブロックできず、**配送ブリッジ**（queue＋wakeup）という実装物が要る。

## 責務の三分（本ユニットの位置づけ・2026-07-03 開発者確認）

- **機構（本ユニット）**: アクター原語——envelope 規約・spawn/join・停止手順・UI 配送ブリッジ。**特定エンジンの知識を持たない**。
- **経路（kanade）**: 実行時の全体調整＝誰が誰に何を流すかの運行表（SHIORI イベント循環のトポロジー）。kanade は本基盤の**最大の消費者**であって所有者ではない（kanade 自身が基盤上のアクター・seriko→emo 等 kanade 非経由の通信も存在）。
- **結線（ghost / ghost-setup）**: エンジンスレッドを起こし channel を繋ぎ、終了時に落とす構築・lifecycle。

## Current State

- **原語は未整備**: 共有のアクター/チャンネル基盤なし。
- **wintf 既存資産（接続候補）**: `event_listener`＋`std::thread`（pilot clickthrough 実証パターン・tokio 不使用）／`VsyncEventBridge`（`runtime/tick_bridge.rs`）／`wintf-winmsg-executor`（message pump 統合・i686 実証済み）。UI スレッドは MTA（`WinApp::new`＝COINIT_MULTITHREADED・記憶 areka-wuc-runs-on-mta-thread）。
- **host-32 は天然のアクター境界**: 別プロセス＝IPC（WM_COPYDATA）が channel。x64 親窓スレッドも pump スレッド（本基盤の pump 統合と同型の課題を既に解いている＝参照実装）。
- **sakura は per-talk transient**（talk ごとに生成・破棄されるアクター）＝spawn/破棄の軽量性が要件。

## Desired Outcome

全エンジンが同じ原語で会話できる**最小のアクター基盤**: ①アクター spawn/join（名前付きスレッド）②inbox 規約（アクターごと単一 Receiver・メッセージ＝enum）③reply 規約（返信 Sender 同梱＝request/reply）④停止手順（Close メッセージ→drain→join）⑤ **UI スレッド配送ブリッジ**（queue＋wakeup で pump スレッドへ届ける）。

**✔ 観測（単一 pass/fail）**: toy アクター試験——(a) worker⇄worker の request/reply と Close→join が決定的に完走（b) **worker→UI スレッド（message pump 実走）への配送ブリッジ**が echo を返す（wintf の pump 上で実測）。既存エンジンの改修はしない（消費は下流ユニット）。

## Approach

1. **チャンネル実装は `std::sync::mpsc` 起点**（依存ゼロ）。アクター＝inbox 1本（単一 Receiver）ゆえ select 不要・tick は `recv_timeout` で賄う。**`crossbeam-channel` は select/MPMC が実需になった時のみ**の fallback（新規依存＝開発者承認要・現時点で申請しない）。
2. **envelope 規約**: アクターごとにメッセージ enum（命名 `XxxMsg`）・返信は `Sender<Reply>` をメッセージに同梱（oneshot 相当）・横断制御（Close 等）は各 enum に含める（共通トレイトの過剰抽象はしない）。
3. **spawn/join**: `std::thread::Builder` 名前付き・JoinHandle は結線層（ghost）が保持・panic は join で検出し上位へ伝搬（監督ツリーは作らない＝最小）。
4. **UI 配送ブリッジ**: queue＋wakeup（`PostMessage` 系 or 既存 `VsyncEventBridge`/winmsg-executor への相乗り——**design で選定**）。UI 側は pump 内で drain。**emo-present の指令 API・窓移動指令の将来の搬送路**。
5. **backpressure 方針**: 制御メッセージは unbounded＋流量は低レート前提（毎フレーム大量データは channel に流さない＝共有バッファ渡し等・design で規約明文化）。

## 設計指示・注意点

- **フレームワーク化しない**（spec 工場・過剰抽象の禁止に従う）: トレイトだらけの actor framework を作らない。「規約＋薄いヘルパ＋ブリッジ」まで。抽象は2例目の実物が要求してから。
- **メッセージは Send な所有データ**（借用を跨がせない）。大きな画素バッファ等は `Arc` か共有バッファの手渡し規約を明記（コピー禁止の予算意識）。
- **停止の全経路テスト**: Close 前のメッセージは drain して処理 or 破棄——どちらかを規約として固定（曖昧が一番の統合バグ源）。
- **tracing 統合**: スレッド名・アクター名を span に載せる（steering logging.md 準拠・Subscriber 初期化はアプリ側）。
- **配置**: 新設モジュール/小クレート（`areka-actor` 等・design 判断）＝既存クレートと非衝突・現行フロントと並走安全。
- **ukadoc**: 本ユニットは伺か仕様非依存の内部基盤＝ukadoc 参照不要（クロスエンジン I/O 契約 4 クラスタのメッセージ**型の中身**は各エンジン仕様の領分）。

## クロスユニット契約（後続を詰ませない事前考慮）

- **消費者**: kanade（イベント循環＝最初の本格消費者）・sakura（per-talk transient spawn）・seriko（emo への毎フレーム駆動）・emo-present（指令 API の channel 化＋UI 配送ブリッジの受け手）・ghost-setup（結線・lifecycle）・shiori（親窓 pump スレッドとの統合は host-32 参照実装と整合）。
- **emo-present との契約**: 指令口（`show_surface` 級）は本基盤の envelope 規約に載る前提（M-boot は直接呼出で開始し、kanade/seriko 結線時に channel 化——emo-present brief 記載済みの seam の実体が本ユニット）。
- **I/O 契約 4 クラスタ**（撫で/選択肢/二人立ち/移動・roadmap クロスエンジン I/O 節）のメッセージ型は、本基盤の envelope 上に各クラスタ着手時に定義する（本ユニットは器のみ）。

## Scope

- **In**: envelope/spawn/join/停止の規約文書＋薄いヘルパ／UI 配送ブリッジ実装／toy アクター試験（worker⇄worker・worker→UI pump）／backpressure・大型データ手渡しの規約明文化。
- **Out**: 各エンジンの実アクター化（kanade/sakura/seriko/emo の各ユニット）／I/O 契約 4 クラスタのメッセージ型定義／crossbeam-channel 導入（実需まで凍結）／監督ツリー・再起動戦略（M2 以降・実需駆動）／async runtime（tokio 禁止）。

## Boundary Candidates

- 規約＋ヘルパ（純粋・単体テスト可）／UI 配送ブリッジ（wintf を知る唯一の層）の二層。

## Upstream / Downstream

- **Upstream**: wintf の pump/tick 資産（`VsyncEventBridge`・winmsg-executor・event_listener 実証）✅／並行モデル正本（roadmap・記憶）。
- **Downstream**: `areka-P0-kanade`（最初の本格消費者・**本ユニットが先行依存**）／`sakura-engine`／`seriko-engine`／`emo-present`（channel 化時）／`ghost-setup`（結線）。

## Existing Spec Touchpoints

- **Extends**: なし（新設横断基盤）。**Adjacent**: `completed/areka-P0-host32-ipc`（プロセス跨ぎ actor 境界の参照実装・wire は不改変）／`wintf-winmsg-executor`（pump 統合の既存解）。

## Constraints

- Rust 2024・**tokio 禁止**・新規依存なしが既定（`std::sync::mpsc`＋`std::thread`。crossbeam は実需時に承認申請）。
- 最小実装＋薄い拡張シーム（監督/再起動・select・MPMC は**実需の2例目**まで作らない）。
- render/window の UI スレッド固定・D2D 単一スレッド・MTA 前提を破らない。
