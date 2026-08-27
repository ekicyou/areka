---
inclusion: always
updated_at: 2026-08-27
---

# Technology Stack

## Architecture

Rust 2024を前提にしたマルチクレート構成です。wintfはbevy_ecsベースのWindows UIフレームワーク、dolaはプラットフォーム非依存の演出定義ライブラリ、arekaは両者を統合する実アプリ層です。ECSによる状態管理とWindowsネイティブAPIの直接利用を組み合わせ、論理ツリーとビジュアルツリーを分離します。

## Core Technologies

- **Language**: Rust 2024 Edition
- **Graphics**: Direct2D, Direct3D11 ＋ 合成層（**Windows.UI.Composition＝WUC・✅2026-07-02 DComp から移行完了**・下記 Key Technical Decisions 参照）
- **Text**: DirectWrite（縦書き・横書き対応）
- **Imaging**: WIC（Windows Imaging Component）
- **Window System**: Win32 API

## Key Libraries

- **bevy_ecs** (0.19): ECSアーキテクチャの実装基盤。**2026-08-19 に 0.18→0.19 へ更新（spec 外直接コミット `bf2d7950`）**——`ExecutorKind` API が撤去され実行器が改稿された（檻の追随は `dpi-transition-atomicity` マージ時に実施済み・**更新前の性能実測は傾向も持ち越せない**＝roadmap 追記(80)）
- **bevy_app** (0.19): ECSスケジュールとアプリライフサイクル統合
- **thiserror** (2): エラー型定義（全クレート共通）
- **windows** (0.62.2): Windows APIバインディング
- **windows-core** (0.62.2): Windows Core API
- **taffy** (0.13): レイアウトエンジン（bevy 0.19 更新と同時に 0.9→0.13 へ）
- **wintf-winmsg-executor** (=0.0.5): Windows メッセージループ・ウィンドウ生成・UI スレッド async の基盤クレ（`winmsg-executor` フォーク）。極初期版ゆえ完全 pin（`=`）。共有ウィンドウクラスに `CS_DBLCLKS` ＋既定カーソルを内蔵
- **event-listener** (5): スレッド跨ぎの起床通知（VSync スレッド→UI スレッド async tick）。tokio 非依存
- ~~**async-executor**~~: 撤去済み（spec `wintf-winmsg-executor` 完了時）。UI スレッド async は `wintf-winmsg-executor` の `spawn_local` へ移行。背景プール `WintfTaskPool` は `bevy_tasks` ベースで async-executor 非依存
- **bevy_tasks** (0.19): 背景ワーカープール `WintfTaskPool` の実行基盤（`world.spawn` 経路・UI スレッドとは別レイヤ）
- **tracing / tracing-subscriber**: 構造化ロギング
- **windows-numerics** (0.3.1): Windows数値型サポート
- **ambassador** (0.5.0): トレイト委譲（delegation）マクロ。COM/状態ラッパーのボイラープレート削減に使用
- **nonmax** (0.5.5): ニッチ最適化された非最大整数型
- **encoding_rs** (0.8): 伺か資産の charset デコード（`areka-parsers` の `charset` module・意図的依存追加＝2026-07-02 承認済）
- **pasta_core** (0.1.6): 里々インスパイアの会話DSLエンジン。`[patch.crates-io]` で `vendors/pasta/` のサブモジュールへ差し替え（後述の Key Technical Decisions 参照）

### dola クレート依存
- **serde** (1): シリアライズ/デシリアライズ基盤
- **serde_json** (1, feature: `json`): JSON対応（デフォルト有効）
- **toml** (0.8, feature: `toml`): TOML対応
- **serde_yaml** (0.9, feature: `yaml`): YAML対応
- **interpolation** (0.3.0): イージング・補間計算の基盤（`easing.rs`, `runtime/interpolator/`）
- **rand** (0.10.0): アニメーション系の乱数生成
- **pasta_core**: DSL連携のための直接依存（wintf経由ではなくdolaが直接取り込む）

## Development Standards

### Type Safety
Rust言語の型システムを最大限に活用。`unsafe`ブロックはWindows API呼び出し時のみに限定し、安全性を文書化。

### Code Quality
- モジュール単位で責務を明確に分離（`com/`, `ecs/`, `api.rs`など）
- Windows COMオブジェクトのライフタイム管理を厳密に実施
- エラーハンドリングは `thiserror` を使用して構造化enumを定義する（全クレート共通規約）
- Windows API境界では `windows::core::Result` を使用し、内部エラーへの変換は `#[from]` で行う

### Testing

- `cargo test` を基準に、クレートごとの統合テストとin-sourceテストを併用する
- wintfは`tests/`配下をドメイン別に分割し、トップレベルのエントリポイントから束ねる
- examplesは手動検証とグラフィックス挙動確認の補助であり、テストの代替ではない

## Development Environment

### Required Tools
- Rust 2024 Edition
- Windows 10/11（Windows.UI.Composition 対応）
- Visual Studio Build Tools（Windows SDKが必要）

### マルチアーキテクチャ・ターゲット（host-32 トラック）
- 最終成果物は **x64＋arm64 ネイティブ両対応**、**i686 は 32bit SHIORI 駆動の helper のみ**（ターゲットは crate 境界で分離・`cfg` 分岐回避）
- rustup targets: `i686-pc-windows-msvc`（helper）／`aarch64-pc-windows-msvc`（arm64。VS2022 の `Microsoft.VisualStudio.Component.VC.Tools.ARM64` が必須＝無いと最終リンクのみ落ちる）
- **クロスターゲットのビルドは必ず PowerShell で**実行（Git Bash は GNU coreutils の `link.exe` が MSVC link を遮蔽しリンクエラー）
- **32bit 可搬性制約の適用範囲＝host-32 系（`shiori-host32-*`／`shiori-abi`）のみ**。wintf/areka 本体（x64+arm64）の spec に i686 ビルド検証を課さない（`api.rs` の isize 契約で元々 i686 非対象）

### Common Commands
```bash
# Dev (サンプル実行): cargo run -p wintf --example taffy_flex_demo_old
# Build: cargo build
# Build (Release最適化): cargo build --release
# Test: cargo test
```

## Key Technical Decisions

- **ECS採用**: 複雑なGUI要素の管理とヒットテストロジックをコンポーネントベースで実装
- **合成層（Windows.UI.Composition＝WUC・✅2026-07-02 移行完了）**: ハードウェアアクセラレーション合成の基盤。DComp（旧 `com/dcomp.rs`）から **Windows.UI.Composition（`Compositor`/`DesktopWindowTarget`/`CompositionDrawingSurface`＋`CompositionSurfaceBrush`）へ純粋等価移行完了**（interop は `com/wuc.rs`・resource は `ecs/graphics/wuc_resource.rs`・DispatcherQueue は既存 message pump に相乗り＝`DQTAT_COM_NONE`・`Commit()` 廃止→vsync tick の暗黙反映）。旧 `com/dcomp.rs`／`dcomp_resource.rs` は撤去済み。**WUC はスレッド親和ゆえ WUC を触る graphics schedule は UI スレッド固定**（`ExecutorKind::SingleThreaded`）。完了 spec: `.kiro/specs/completed/wintf-dcomp-to-wuc-migration/`。
- **透過の合成方式（旧「ULW 一択」結論を撤回・新方針確定・✅2026-07-05 撤去完了で GPU 合成単独へ）**: 伺か型マスコットは「別プロセスのウィンドウ上に乗り、透明ピクセル上のクリックをその別プロセスへ透過させる」のが中核要件。**現状の実装**は **GPU 合成（WUC）単独**＋クリックスルー機構（`WS_EX_TRANSPARENT` 動的トグル＋`WS_EX_LAYERED` 同伴フラグ＋αマスク `AlphaMask::is_hit`）で成立（全窓 `WS_EX_NOREDIRECTIONBITMAP` の GPU 合成窓固定）。旧 `CompositionMode` enum 切替基盤・COM ラッパー `com/ulw.rs` は `wintf-ulw-removal`（2026-07-05）で撤去済み。
  - **~~実質 ULW 一択~~ ← 撤回**: 先進坑 `pilot-clickthrough-alpha-toggle`（**✅ go 済み 2026-07-01・開発者承認**）が、**`WS_EX_TRANSPARENT` 動的トグル方式**（**表示層＝合成 visual/content と 当たり判定層＝HWND スタイルの二層分離**・別スレッドのカーソル監視＋αマスク連動）で **DComp/GPU 合成を維持したまま別プロセスクリック透過が成立**することを実証（他社 3D マスコット実績のある手段）。ULW（CPU ビットマップ）は GPU 合成と併用不可で「3D 描画を諦める踏み絵」だったが、その制約は解けた。
  - **決定済み方針**: ① 表示合成を **Windows.UI.Composition** へ（**✅ `wintf-dcomp-to-wuc-migration` 2026-07-02 完了**）、② 別プロセス透過は **`WS_EX_TRANSPARENT` 動的トグル**を wintf 本体へ（**✅ `wintf-clickthrough-alpha-toggle` 2026-07-02 完了**）、③ **ULW ルートは除去**（**✅ `wintf-ulw-removal` 2026-07-05 完了**・②完了でゲート解除済み。`compositor.rs`/`compositor_systems/`/`com/ulw.rs` 削除・`CompositionMode` collapse で GPU 合成単独へ）。
  - **不変の却下事項**: `WM_NCHITTEST`→`HTTRANSPARENT` はプロセス境界を越えず別プロセス透過に使えない。`SetWindowRgn` 方式は DComp 描画をクリップするため却下済み（`_rejected/wintf-P0-click-through-rgn`）。正本は `doc/COMPAT_ARCHITECTURE.md`／`.kiro/steering/roadmap.md`。
- **DirectWrite**: 高品質な日本語テキストレンダリングと縦書き対応
- **Workspace構成**: フレームワーク、演出定義、実アプリを分離したモノレポ構成
- **Release最適化**: サイズ最適化（`opt-level='z'`, `lto=true`）でバイナリサイズを削減
- **UI スレッド基盤の外部クレ化（tokio 非依存・置換完了）**: メッセージループ・ウィンドウ生成・UI スレッド async・60Hz ECS tick 起床を、自作実装から `wintf-winmsg-executor`（=0.0.5）ベースへ**置換完了**（spec `wintf-winmsg-executor`・新 facade `WinApp`）。`MessageLoop`/`block_on`/`spawn_local`/`util::Window<S>`（`new_ex`・wndproc 再入可）へ写像し、スレッド跨ぎ起床は `event_listener` で実現。tokio 非採用（`!Send` future を UI スレッド単一で実行）。背景重処理用 `WintfTaskPool`（`bevy_tasks` ＋ `world.spawn`）は別レイヤとして温存。ドラッグの同期 WM_WINDOWPOSCHANGED 再入に依存するため `new_checked_ex`（RefCell 再入阻止）ではなく `new_ex` を採用し、tick 二重実行は ECS ガード（`IS_TICK_FLUSH_IN_PROGRESS`＋`try_borrow_mut`）で防ぐ
- **レガシー UI 基盤の撤去（完了）**: 旧自作 `win_message_handler` / `win_thread_mgr` / `winproc` / `process_singleton` は spec `wintf-winmsg-executor` 完了に伴い**撤去済み**。メッセージ配送は `ecs/window_proc/` の `dispatch_window_message`、UI スレッド基盤は `runtime/` の `WinApp` facade を使用する
- **構造化ログ**: `tracing` を全体規約とし、subscriber初期化はアプリ層で行う
- **pasta のベンダリング**: 外部依存だった `pasta_core` を git サブモジュール（`vendors/pasta/`）として同梱し、`[patch.crates-io]` でローカルパスへ差し替える。wintf/dola/areka とDSLエンジンを同一ワークスペースで協調開発するための運用。クローン時は `git submodule update --init` が必要
- **ukadoc互換ベースウェア戦略（2026-06-26）**: areka を ukadoc準拠の互換ベースウェア（SSP代替）として確立する。SERIKO/MAYUNA完全マップ＋さくらスクリプト優先度順。SERIKO/さくらスクリプトランナーは「タイミング特化の下位層 dola」の上に建てる上位層。SERIKOを平坦サブセットに内包する**階層サーフェスエンジン**（エレメント→別サーフェス定義参照・wintf visual-tree＋dola nested-storyboard）。SHIORIは内部唯一ABI=`IShiori`(COM, HSTRING/UTF-16)、ネイティブ=in-proc COM、過去互換=32bit Rustホスト（flat-C/HGLOBAL/charset/SAORI同居/自前IPC。**IPC は WM_COPYDATA 一本化＋再入 RESPONSE・x64⟷x86 を跨ぐのは生バイト列のみ**＝`areka-P0-host32-ipc` 2026-07-02 完了・3クレート `shiori-host32-ipc`/`-host`/`-helper`）。詳細の正本は `doc/COMPAT_ARCHITECTURE.md`

---
Document standards and patterns, not every dependency.
