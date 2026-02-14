# Research & Design Decisions

## Summary
- **Feature**: `areka-mock-shell`
- **Discovery Scope**: Extension（既存wintfシステム上にarekaバイナリクレートを新規作成）
- **Key Findings**:
  - wintfの公開APIは十分に成熟しており、arekaクレートからの利用にギャップはない
  - バルーン追従は `OnDrag` / `OnDragEnd` ハンドラで `SetWindowPosCommand` を発行する方式が最も自然
  - ダブルクリック終了は `PointerState.double_click` + `OnPointerPressed` で実現可能

## Research Log

### wintf 公開APIサーフェス
- **Context**: arekaクレートが依存するwintfの公開APIを網羅的に把握する必要がある
- **Sources Consulted**: `crates/wintf/src/lib.rs`, `crates/wintf/src/ecs/mod.rs`, 各サブモジュール
- **Findings**:
  - ECSベースのウィンドウ作成: `Window`, `WindowStyle`, `WindowPos`, `BoxStyle` コンポーネントの組み合わせ
  - 透過ウィンドウ: `WS_POPUP | WS_VISIBLE` + `WS_EX_NOREDIRECTIONBITMAP` で実現（DComp依存）
  - 画像表示: `BitmapSource::new(path)` — `on_add`フックで `Visual`, `BitmapSourceGraphics`, `HitTest::alpha_mask()` を自動挿入
  - 縦書きテキスト: `Typewriter` + `TextDirection::VerticalRightToLeft` + `TypewriterTalk` + `TypewriterToken`
  - ドラッグ: `DragConfig { move_window: true }` で wndproc レベルの高性能ドラッグ
  - 矩形描画: `Rectangle` マーカー + `Brushes::with_foreground(color)` / `Brushes::with_colors(fg, bg)`
  - 非同期タスク: `CommandSender` (`mpsc::Sender<BoxedCommand>`)、`EcsWorld::spawn(|tx| async { ... })`
  - イベントハンドラ: `OnPointerPressed`, `OnDragStart`, `OnDrag`, `OnDragEnd`
  - ダブルクリック検出: `PointerState.double_click == DoubleClick::Left`
  - ウィンドウ終了: `world.despawn(entity)` → `on_window_handle_remove` → `PostMessage(WM_CLOSE)` → 全ウィンドウ破棄で `PostQuitMessage(0)`
- **Implications**: wintfの公開APIのみで全要件を実現可能。内部モジュールへの依存は不要。

### バルーンウィンドウ追従設計
- **Context**: シェルウィンドウのドラッグ移動時にバルーンウィンドウが追従する必要がある（Req 3.2）
- **Sources Consulted**: `drag/dispatch.rs`, `drag/mod.rs`, `drag/context.rs`, `win_message_handler.rs`
- **Findings**:
  - `DragConfig { move_window: true }` の場合、`SetWindowPos` は wndproc レベルで直接呼ばれる
  - `WM_WINDOWPOSCHANGED` は `WindowPos.position` に反映される
  - ドラッグハンドラ `OnDrag` / `OnDragEnd` は ECS スケジュール内で呼ばれる
  - `SetWindowPosCommand` は `EcsWorld` の `flush_window_pos_commands()` で一括適用される
  - **方式A**: シェルに `move_window: true` を設定し、`OnDrag` ハンドラ内でバルーンの `WindowPos` を更新
    - 利点: シェルのドラッグはネイティブ並の性能
    - 課題: `OnDrag` ハンドラは ECS tick タイミングで呼ばれるため、バルーンの追従に1フレーム遅延の可能性
  - **方式B**: シェルに `move_window: false` を設定し、`OnDrag` ハンドラ内で両ウィンドウを同時に移動
    - 利点: 両ウィンドウの同期が完全
    - 欠点: ドラッグ性能がECSスケジュール依存（ネイティブ並ではない）
  - **方式C**: シェルに `move_window: true` を使い、ECSシステムとしてバルーン追従ロジックを組み込む
    - 利点: ドラッグ性能と追従精度の両立
    - 欠点: カスタムECSシステム登録が必要（公開APIで可能かは要確認）
- **Implications**: 方式Aを推奨。モック段階では1フレーム遅延は許容範囲。`OnDrag` ハンドラの中で `SetWindowPosCommand` を発行しバルーンを追従させる。

### ダブルクリック終了メカニズム
- **Context**: シェルウィンドウのダブルクリックでアプリケーション終了（Req 3.3）
- **Sources Consulted**: `pointer/mod.rs`, `ecs/mod.rs`
- **Findings**:
  - `PointerState.double_click: DoubleClick` — WM_LBUTTONDBLCLKから生成、1フレームのみ有効
  - `OnPointerPressed` ハンドラの `Phase::Bubble(state)` で `state.double_click == DoubleClick::Left` を検査
  - ハンドラ内で `world.despawn(window_entity)` を呼べば全ウィンドウ終了が可能
  - **注意**: `WS_POPUP` ウィンドウでダブルクリックを受信するには `CS_DBLCLKS` クラススタイルが必要
  - wintfの `create_windows` システムが `RegisterClassExW` を呼ぶ箇所で `CS_DBLCLKS` が設定されているかは要確認
- **Implications**: `OnPointerPressed` + `DoubleClick::Left` チェック + `world.despawn()` の組み合わせで実装可能。**✅ 確認完了**: wintfの`process_singleton.rs` L74でECS用ウィンドウクラスに`CS_DBLCLKS`が既に設定されており、ダブルクリック終了は問題なく動作する。

### Cargo.toml 構成（crates.io公開）
- **Context**: arekaクレートをcrates.io公開可能な状態にする（Req 5.1, 5.2）
- **Sources Consulted**: `Cargo.toml`（ワークスペースルート）, `crates/dola/Cargo.toml`, `crates/wintf/Cargo.toml`
- **Findings**:
  - ワークスペースデフォルト: `publish = false`, `edition = "2024"`, `license = "MIT OR Apache-2.0"`
  - `members = ["crates/*"]` — `crates/areka/` を作成すれば自動検出
  - wintfは `publish = { workspace = true }` (= false)
  - dolaは `publish.workspace = true` (= false) だが `description` フィールドあり
  - areka では `publish = true` を明示的にオーバーライドし、`description`, `repository` 等を設定する必要がある
  - バイナリクレートは `[[bin]]` セクションか `src/main.rs` が必要
- **Implications**: `src/main.rs` 配置でバイナリクレートとして認識される。`publish = true` は workspace default をオーバーライドで設定。

### シェルアセット配置と参照パス
- **Context**: `shell/` ディレクトリの移動と実行時パス解決（Req 5.5）
- **Sources Consulted**: `shell/` ディレクトリ構造, `BitmapSource` 実装
- **Findings**:
  - `BitmapSource::new(path)` はパス文字列を保持し、非同期タスクで `WIC` 経由で読み込み
  - 現在の既存 examples は相対パスを使用（`cargo run` 実行ディレクトリからの相対）
  - `crates/areka/shell/` に移動後、`cargo run -p areka` はワークスペースルートから実行される
  - よって `BitmapSource::new("crates/areka/shell/base.png")` のような相対パス指定が必要
  - 将来的には `env!("CARGO_MANIFEST_DIR")` を使った絶対パス解決が望ましい
- **Implications**: 初期実装ではワークスペースルートからの相対パスで問題なし。将来の配布時には `CARGO_MANIFEST_DIR` ベースのパス解決に移行。

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| A: 単一main.rs | 全ロジックを `src/main.rs` に集約 | シンプル、モック段階に最適 | 将来の拡張時にリファクタ必要 | 推奨 |
| B: モジュール分割 | `src/shell.rs`, `src/balloon.rs` 等に分割 | 構造的、テスト容易 | モック段階ではオーバーエンジニアリング | 将来検討 |
| C: lib + bin分離 | `src/lib.rs` + `src/main.rs` | ライブラリとしても利用可能 | モック段階では不要な複雑さ | 将来検討 |

## Design Decisions

### Decision: 単一 main.rs 構成
- **Context**: モック段階のareka クレートの内部構造
- **Alternatives Considered**:
  1. Option A — `src/main.rs` に全ロジック集約
  2. Option B — `src/shell.rs`, `src/balloon.rs` にモジュール分割
- **Selected Approach**: Option A — 単一 `src/main.rs`
- **Rationale**: モック段階は機能が限定的（シェル+バルーン+ドラッグ+終了）であり、分割の利点が薄い。wintf の examples パターンと整合。将来のクレート成長時にリファクタリングする計画。
- **Trade-offs**: 将来リファクタが必要だが、今は開発速度と把握しやすさを優先
- **Follow-up**: 機能追加時にモジュール分割を検討

### Decision: バルーン追従方式
- **Context**: シェルウィンドウのドラッグ時にバルーンが追従する必要がある
- **Alternatives Considered**:
  1. 方式A — `move_window: true` + `OnDrag` ハンドラでバルーン `WindowPos` 更新
  2. 方式B — `move_window: false` + 両ウィンドウ同時カスタム移動
  3. 方式C — カスタムECSシステムでフレーム単位同期
- **Selected Approach**: 方式A
- **Rationale**: シェルのドラッグ性能はネイティブ並を維持しつつ、バルーン追従はECSハンドラで行う。1フレーム遅延はモック段階では許容。wintfの既存パターン（`OnDrag` ハンドラ）に準拠。
- **Trade-offs**: バルーンの追従に微細な遅延が発生する可能性があるが、視覚的にはほぼ気にならない
- **Follow-up**: 遅延が問題になればカスタムECSシステム方式に移行

### Decision: ダブルクリック終了
- **Context**: タスクバー非表示のデスクトップマスコットに終了手段が必要
- **Selected Approach**: `OnPointerPressed` ハンドラで `PointerState.double_click == DoubleClick::Left` を検知し、全ウィンドウの `Entity` を `despawn`
- **Rationale**: wintf の既存メカニズム（PointerState, DoubleClick, despawn → WM_CLOSE → PostQuitMessage）を完全に活用。追加のウィンドウメッセージ処理不要。
- **Trade-offs**: `CS_DBLCLKS` クラススタイルが wintf のウィンドウクラス登録に含まれていない場合は動作しない
- **Follow-up**: `CS_DBLCLKS` の有無を実装時に確認。未設定なら wintf への修正要求を検討。

## Risks & Mitigations
- **CS_DBLCLKS 未設定リスク** — ✅ **Resolved**: wintfの`process_singleton.rs` L74でECS用ウィンドウクラスに`CS_DBLCLKS`が設定済み。ダブルクリック終了は実装可能。
- **ChildOf コンポーネント利用可能性** — ✅ **Resolved**: wintfの`ecs/mod.rs` L23で`pub use bevy_ecs::hierarchy::{ChildOf, Children};`として公開されている。`wintf::ecs::ChildOf`として利用可能。
- **アセットパス依存** — `BitmapSource` のパスがワークスペースルートからの相対パスに依存。`cargo install` でのバイナリ配布時にはパス解決が破綻する。モック段階では許容し、将来 `include_bytes!` や `CARGO_MANIFEST_DIR` ベースのパス解決に移行。
- **crates.io 名前予約** — `areka` クレート名が既に取得されている可能性。公開前に `crates.io` で確認が必要。

## References
- wintf 公開API: `crates/wintf/src/ecs/mod.rs` — 全exportの定義
- 既存exampleパターン: `crates/wintf/examples/typewriter_demo.rs` — 初期化・非同期タスク・Typewriter
- 既存exampleパターン: `crates/wintf/examples/taffy_flex_demo.rs` — マルチエンティティ・ドラッグ・BitmapSource
- steering/structure.md: areka クレートの配置計画
- steering/logging.md: tracing 初期化パターン
