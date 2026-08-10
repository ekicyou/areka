---
inclusion: always
updated_at: 2026-07-02
---

# Project Structure

## Organization Philosophy

**責務ごとのクレート分割** - wintfがWindows向けUI基盤、dolaが演出データモデル、arekaがアプリ統合を担う。脳（SHIORI）との内部唯一ABIは独立した最小依存クレート shiori-abi（`IShiori`/`IShioriHost` のカスタムCOM定義＋エルゴノミック変換層）として分離し、UI基盤（wintf）に依存させない（下流32bitホスト/pastaが同ABIを共有するため）。wintf内ではCOMラッパー、ECSサブシステム、Win32メッセージ境界を分け、dolaは定義層から実行時層までを段階的なモジュールで構成する。

## Directory Patterns

### Workspace Root
**Location**: `/`  
**Purpose**: Cargoワークスペース設定、横断ドキュメント、開発ルール  
**Example**: `Cargo.toml`, `README.md`, `doc/`, `.kiro/steering/`

### Library Crate
**Location**: `/crates/wintf/`  
**Purpose**: メインライブラリの実装  
**Structure**:
- `src/` - ライブラリソースコード
- `examples/` - 手動検証用サンプルアプリケーション
- `tests/` - ドメイン別に整理された統合テスト

### COM Wrapper Layer
**Location**: `/crates/wintf/src/com/`  
**Purpose**: Windows COMインターフェイスのRustラッパー  
**Contains**:
- `wuc.rs` - Windows.UI.Composition interop（`Compositor`/`DesktopWindowTarget`・旧 `dcomp.rs` は撤去済み）
- `d3d11.rs` - Direct3D11 API
- `dwrite.rs` - DirectWrite API（縦書き対応）
- `dxgi.rs` - DXGIインターフェイス
- `wic.rs` - Windows Imaging Component
- `animation.rs` - Windows Animation API
- `d2d/` - Direct2D関連

> **合成層の現況（2026-07-05 更新）**: 合成バックエンドは **Windows.UI.Composition（WUC）へ移行完了**（`com/wuc.rs` interop＋`ecs/graphics/wuc_resource.rs`・旧 `com/dcomp.rs`／`dcomp_resource.rs` は撤去済み・`wintf-dcomp-to-wuc-migration` 完了）。WUC はスレッド親和ゆえ WUC を触る graphics schedule は UI スレッド固定。**ULW は `wintf-ulw-removal`（2026-07-05 完了）で撤去済み**: `com/ulw.rs`・`ecs/graphics/compositor.rs`（`WindowD3D11Compositor`）・`compositor_systems/` を削除、`CompositionMode` enum も collapse して **GPU 合成（WUC）単独**へ。別プロセス透過は `WS_EX_TRANSPARENT` 動的トグル方式（`wintf-clickthrough-alpha-toggle` 完了）に一本化。完了 specs: `completed/wintf-ulw-removal`／`completed/wintf-clickthrough-alpha-toggle`。正本は `roadmap.md`／`doc/COMPAT_ARCHITECTURE.md`。

### ECS Component Layer
**Location**: `/crates/wintf/src/ecs/`  
**Purpose**: ECSアーキテクチャのコンポーネント定義  
**Structure**:
- `common/` - 共通インフラ（階層伝播システム）
- `window/` - ウィンドウ管理とWin32状態同期
- `graphics/` - グラフィックスリソースと描画システム
- `layout/` - taffy統合、配置計算、ヒット判定
- `widget/` - UIウィジェット、テキスト、画像、ブラシ
- `pointer/` - ポインター入力のバッファリングと配信
- `drag/` - ドラッグ状態管理とディスパッチ
- `dola/` - DolaRuntimeのECS Component化
- `world/` - schedule labels、vsync、フレーム進行
- `app.rs` - アプリケーション状態管理（ウィンドウカウント、ディスプレイ構成変更）
- `window_proc/` - Win32メッセージ種別ごとのECSブリッジ
- `types.rs` - 共通幾何プリミティブ型（`Point`/`Size`/`Rect` 等）の唯一の定義箇所。`#[repr(C)]` でWin32/D2D1型とゼロコスト相互変換

#### ECS機能グループ詳細

**1. Common Infrastructure** (`common/`)
- 責務: ECS階層システムの汎用的な伝播ロジック
- 代表的な関数: `sync_simple_transforms<L,G,M>()`, `propagate_parent_transforms<L,G,M>()`
- 特徴: 完全ジェネリック化、`Arrangement`/`Transform`両対応

**2. Window Management** (`window/`, `window_proc/`)
- 責務: Win32ウィンドウのライフサイクル管理とECS統合
- 代表的なコンポーネント: `Window`, `WindowHandle`, `WindowPos`, `WindowStyle`, `ZOrder`
- 特徴: HWNDとEntityの双方向マッピング、マルチスレッド対応

**3. Graphics Resources** (`graphics/`)
- 責務: Direct2D/WUC（Windows.UI.Composition）リソースのライフサイクル管理
- 代表的なコンポーネント: `GraphicsCore`, `WindowGraphics`, `Visual`, `Surface`, `DeviceContext`
- サブモジュール: `wuc_resource.rs`（WUC リソース・UI スレッド固定）, `compositor.rs`（`WindowD3D11Compositor` — ULW 専用・除去予定）, `compositor_systems/`（同・ULW 専用）, `visual_manager.rs`（Visualの挿入・管理API）, `command_list.rs`（D2Dコマンドリスト）
- 特徴: デバイスロスト対応、遅延初期化、階層的描画

**4. Layout System** (`layout/`)
- 責務: taffyレイアウトエンジン統合と配置計算
- サブモジュール: `taffy.rs`, `metrics.rs`, `arrangement.rs`, `rect.rs`, `systems/`, `hit_test/`, `hit_region/`
- 代表的なコンポーネント: `TaffyStyle`, `TaffyComputedLayout`, `Arrangement`, `GlobalArrangement`, `Size`, `Offset`
- 特徴: 軸平行変換最適化、Common Infrastructure活用、Surface生成最適化

**5. Input Systems** (`pointer/`, `drag/`)
- 責務: ポインターイベントの収集、ヒットテスト、ドラッグ状態遷移
- 代表的な型: ポインターバッファー、ドラッグコンテキスト、キャプチャガード
- 特徴: Win32メッセージ境界とECSイベント処理を分離

> **cue の ECS 統合レイヤーは撤去済み（2026-07-17）**: 旧 `ecs/cue/`（`CueQueue`／`dispatch_pending_cue_sheets`／`CueSheetTracker`／`EntityRegistry`）は**旧世代**（生きた App へ未配線）ゆえ `completed/areka-P0-cue-playback-duration` で削除した。cue 再生の制御（変換・状態機械・完了 horizon・バリア・broadcast）は **dola `cue` モジュールが単一の住処**（`CuePlayer` 受動ランタイム＋`CueSink` 単一トレイト＋`to_talk_schedule` 変換1本）。将来 wintf 側で cue を要する場合も**別 cue エンジンを新設せず** `dola::cue::CueSink` を実装すれば足りる。

**6. Dola Animator** (`dola/`)
- 責務: `DolaRuntime` のエンティティごとのECS Component化
- 代表的なコンポーネント: `DolaAnimator`（`DolaRuntime` を内部所有、`unsafe impl Send + Sync`）
- 主要システム: `tick_dola_animators`（`Query<&mut DolaAnimator>` + `Res<FrameTime>` で全エンティティ一括tick）
- 安全性保証: `Query<&mut>` の排他アクセスにより1 tick 1回・単一スレッドでの更新を型レベルで保証
- 消費パターン: 後続システムが `Query<&DolaAnimator>` の `last_result()` で `UpdateResult` を読み取る

**7. World Scheduling** (`world/`)
- 責務: schedule label、vsync、フレーム時間管理
- 特徴: グラフィックス更新、入力処理、アニメーション処理順序を明示する

**8. Application State** (`app.rs`)
- 責務: アプリケーション全体の状態管理
- 代表的な構造体: `App`（ウィンドウカウント、ディスプレイ構成変更検出、メッセージウィンドウ管理）
- 特徴: ECS Resourceとしてワールドに導入、ウィンドウの作成・破棄を追跡

### UI スレッド基盤 / Message Handling
**Location**: `/crates/wintf/src/runtime/`（新 facade）＋ `/crates/wintf/src/`（ルート補助）  
**Purpose**: UI スレッド基盤（メッセージループ・ウィンドウ生成・UI スレッド async・60Hz tick・終了規律）  
**Contains（runtime/ = 新 facade `WinApp`・spec `wintf-winmsg-executor` で外部クレ化完了）**:
- `runtime/mod.rs` - 公開 facade `WinApp`（`new`/`world`/`run`/`spawn_ui_local`）。COM/DPI 初期化・World 所有・全結線
- `runtime/message_loop.rs` - `MessageLoopDriver`（`block_on`/`MessageLoop::run` 委譲）＋ `ShutdownPolicy`（`event_listener::Event` 終了規律）
- `runtime/tick_bridge.rs` - `VsyncEventBridge`（DwmFlush→event_listener notify）＋ `AsyncTickTask`（13 schedule tick）
- `runtime/wndproc_bridge.rs` - `WndState`/`make_wndproc`（ライブラリ `Window::new_ex` クロージャ→`dispatch_window_message` 配送・GWLP 不使用）
- `runtime/window_registry.rs` - `WindowRegistry`（NonSend・`Window<S>` 所有・reconcile で寿命/終了管理）
- `runtime/window_factory.rs` - `EcsWindowFactory`（`util::Window::new_ex` 生成・style/pos/title 反映）

**ルート補助モジュール**:
- `win_state.rs` - ウィンドウ状態管理
- `win_style.rs` - ウィンドウスタイル定義
- `api.rs` - Windows API safeラッパー

> **注意**: 旧 deprecated モジュール（`winproc.rs` / `win_message_handler.rs` / `win_thread_mgr.rs` / `process_singleton.rs`）は spec `wintf-winmsg-executor` の完了に伴い**撤去済み**。メッセージ配送は `ecs/window_proc/` 配下の `dispatch_window_message` ＋ 種別別ハンドラ、UI スレッド基盤は `runtime/` の `WinApp` facade を用いること。クラス登録・HINSTANCE はライブラリ（`wintf-winmsg-executor`）が担う。

## Naming Conventions

- **Files**: `snake_case.rs`（Rust標準）
- **Modules**: `snake_case`
- **Types**: `PascalCase` (structs, enums, traits)
- **Functions**: `snake_case`
- **Constants**: `SCREAMING_SNAKE_CASE`

### Test Naming Conventions

#### Integration Tests (`tests/` directory)
- **File name**: `{feature}_{type}_test.rs` or `{feature}_test.rs`
- **Entry point**: `tests/{domain}.rs` — `#[path]` による `mod` 宣言のみ、テストロジックは含まない
- **Common helpers**: `tests/{domain}/common/mod.rs`
- **Domain prefix removal**: ドメインサブディレクトリに配置する際、ドメイン名と重複するプレフィックスを除去する
  - 例: `compile_error_test.rs` → `compile/error_test.rs`
  - 例: `visual_child_order_test.rs` → `visual/child_order_test.rs`
  - ただし `taffy_` のようなサブドメインプレフィックスは維持する
- **パス属性で読み込まれたファイルの子モジュール解決**: `#[path]` で読み込まれたファイルの子モジュールは、**そのファイル自身のディレクトリ**を基準に解決される（`<stem>/` サブディレクトリは探さない）。したがって `tests/dom/a.rs` の中の素の `mod sub;` は `tests/dom/sub.rs` を探し、`tests/dom/a/sub.rs` は探さない——歴史的形式の `{module}/tests.rs` を素の `mod tests;` で引こうとすると **E0583** になるのはこのためである。**下のテスト分離の命名規約はファイル名を `<stem>_<モジュール名>.rs` と定めるので、素の `mod` では決して届かず、`#[path]` の明示が必須になる。**
- **テスト分離の接続規約は `src/` と同一**（下の Unit Tests を参照）。統合テストのファイルが肥大化した場合も、同じ `<stem>_<モジュール名>.rs` ＋ `#[cfg(test)] #[path = "…"] mod <モジュール名>;` の形でテーマ分割する。`tests/` ツリーは既に `#[cfg(test)]` 相当の文脈だが、**接続宣言の `#[cfg(test)]` は冗長でも付ける**（`src/` と同一形にして規約を 1 本に保つため）。

#### Unit Tests (in-source `#[cfg(test)]`)

**新規のテストモジュールは本番ファイルの中に本体を書かない。** 同一ディレクトリの兄弟ファイルへ置き、本番ファイル側にはパス属性つきの接続宣言だけを残す。

```rust
// 本番ファイル側（末尾に置く）
#[cfg(test)]
#[path = "<stem>_<モジュール名>.rs"]
mod <モジュール名>;
```

- **ファイル名の導出規則**: `<stem>_<モジュール名>.rs`。`<stem>` は次のとおり読み替える。
  | 本番ファイル | `<stem>` | 例 |
  |---|---|---|
  | 通常のファイル `foo.rs` | `foo`（basename） | `foo_tests.rs` |
  | `bar/mod.rs` | `bar`（**親ディレクトリ名**） | `bar/bar_tests.rs` |
  | `main.rs` / `lib.rs` | そのまま | `main_tests.rs` / `lib_tests.rs` |

  **逆向き（テストファイル → 本番ファイル）の曖昧さを断つ 2 つの規則**——同一ディレクトリに `foo.rs` と `foo_bar.rs` の両方が在ると、`foo_bar_baz_tests.rs` は「`foo` ＋ `bar_baz_tests`」とも「`foo_bar` ＋ `baz_tests`」とも読めてしまう（実在例: `areka-emo-text/src/` の `viewbox.rs` と `viewbox_draw.rs`）。

  1. **最長 stem 優先**: 候補が複数あるときは、同一ディレクトリに実在する**接続宣言を持ちうるファイル**のうち **stem が最も長いもの**を採る。上の例では `viewbox_draw.rs` が勝ち、`viewbox_draw_test_support.rs` は `viewbox_draw` の `test_support` と読む。

     **「接続宣言を持ちうるファイル」を「本番ファイル」と読み替えないこと。** 候補集合は次のとおりで、`src/` でも本番ファイルには限らない。
     - `src/`: 本番ファイル、**および歴史的形式で既に分離されているテストファイル**（親が `#[cfg(test)] mod X;` で宣言しているもの。例 `areka-parsers/src/shell/decode_tests.rs`・`areka-emo-compose/src/golden_tests.rs`・`areka/src/emo2_boot/spine.rs`）。これらもテーマ分割の親になりうるので候補である
     - `tests/`: 統合テストのファイル
     - `examples/`: サンプルのルートファイル

     候補を本番ファイルだけに絞ると `decode_tests_alias_tests.rs` が「`decode.rs` ＋ `tests_alias_tests`」と誤解決する（`decode_tests.rs` が候補から落ちるため）。正しくは最長 stem の `decode_tests` ＋ `alias_tests` である。
  2. **前向きの衝突禁止**: テーマ名を選ぶとき、`<stem>_<テーマ名>.rs` が**同一ディレクトリの別の本番ファイルから導出しうる名前と衝突してはならない**。上の例なら `viewbox.rs` にテーマ名 `draw_test_support` を付けてはいけない。

  この 2 つを守る限り導出は双方向に一意である。
- **1 モジュール 1 ファイル**: テストモジュール 1 つにつきテストファイル 1 本。1 つのファイルに複数のテストモジュールを詰めない。
- **1 ファイル 1,000 行以下の目安は本番ファイル・テストファイルの双方に適用する。** 超える場合はテーマ単位の複数モジュールへ分割し、それぞれを別の接続宣言で繋ぐ。テーマ間で共有するヘルパは `<stem>_test_support.rs` へ集約する（項目が 1 件でも集約する——複製は本文の同一性を壊す）。
- **テストモジュール以外に付いた `#[cfg(test)]`**（テスト専用のメソッド・定数・`impl` 内の項目など）は**本番ファイルに残す**。移送対象は `#[cfg(test)] mod X { … }` の本体だけである。
- **`mod` 宣言に付いた `///` doc コメントは接続宣言側に残す**（`mod` 項目への doc 属性であり、移動には `///` → `//!` の書き換えが要るため）。
- **共有ヘルパ名の衝突**: 親モジュールに既存の `test_support`（歴史的形式の分離済みファイル等）がある場合、同じモジュール名を二度宣言すると **E0428** になる。モジュール名を変えて回避すること（例: `shared_test_support` → ファイル名は規則どおり `<stem>_shared_test_support.rs`）。導出規則の一意性は保たれる。
- **共有ヘルパが本番項目と同名のときは明示 import で受ける。** `use super::*;` と `use super::test_support::*;` の両方が同じ名前を供給すると **E0659（曖昧）** になる。明示 import はグロブより優先されるので、影付け（shadow）の解決順が移設前と一致する。
- **`include_str!` で本番ファイル本文を読む構造テスト**（層規律の検査など）は、**兄弟テストファイル `<stem>_*.rs` も走査対象に列挙する**。列挙しないと、テストを外へ出した分だけ被覆が黙って縮む。
- **Separated (historical)**: `{module}/tests.rs` — ディレクトリモジュール化パターン（`bitmap_source/` を参照）。**既存のものは歴史的形式としてそのまま維持するが、新規には使わない。**
- **本ワークスペースの `crates/*` に `benches/` と `build.rs` は現状 0 件**（`vendors/` は対象外）だが、追加するときも同じ配置規律を適用する。

### Component Naming Conventions

COMオブジェクトをラップするECSコンポーネントは、以下の命名規則に従う：

#### GPUリソース (`XxxGraphics`)
- **特性**: Direct3D/Direct2D/WUC（Windows.UI.Composition）デバイスに依存
- **デバイスロスト対応**: `invalidate()`メソッドと`generation`フィールドを実装
- **命名**: `XxxGraphics`サフィックス
- **例**:
  - `WindowGraphics` - ウィンドウレベルGPU資源
  - `VisualGraphics` - ウィジェットレベルGPU資源
  - `SurfaceGraphics` - ウィジェットレベルGPU資源
  - 将来: `BrushGraphics`, `BitmapGraphics`

#### CPUリソース (`XxxResource`)
- **特性**: デバイス非依存、永続的
- **デバイスロスト対応**: 不要（通常の参照カウント管理のみ）
- **命名**: `XxxResource`サフィックス
- **例**:
  - `TextLayoutResource` - テキストレイアウト（Label、TextBlock等で再利用）
  - 将来: `TextFormatResource`, `PathGeometryResource`

#### レベル分類
- **ウィンドウレベル**: Windowエンティティに配置（例: `WindowGraphics`）
- **ウィジェットレベル**: 個別ウィジェットエンティティに配置（例: `VisualGraphics`, `TextLayoutResource`）
- **共有リソース**: 複数ウィジェットで再利用（例: 将来の`BrushGraphics`、`GeometryResource`）

#### 非COMコンポーネント
- **論理コンポーネント**: サフィックスなし（例: `Label`, `Rectangle`, `Button`）
- **マーカーコンポーネント**: 用途に応じた名前（例: `HasGraphicsResources`, `GraphicsNeedsInit`）

#### COMアクセスメソッド命名
COMリソースコンポーネント内部のアクセスメソッドは、COM/WinRT インターフェイス型に対応（例は命名パターンを示す。DComp→WUC 移行済みのため実際の戻り型は WUC 系＝`Compositor`/`SpriteVisual`/`CompositionDrawingSurface` 等）：
- `WindowGraphics::target()` → 合成ターゲット型
- `VisualGraphics::visual()` → visual 型
- `SurfaceGraphics::surface()` → surface 型
- `TextLayoutResource::get()` → `Option<&IDWriteTextLayout>`

### Animation Definition Crate
**Location**: `/crates/dola/`  
**Purpose**: 宣言的アニメーション定義フォーマット（Declarative Orchestration for Live Animation）  
**Structure**:
- `src/` - ライブラリソースコード
  - `document.rs` - ルートドキュメント定義
  - `storyboard.rs` - ストーリーボード定義
  - `transition.rs` - トランジション定義
  - `easing.rs` - イージング関数
  - `variable.rs` - アニメーション変数定義
  - `value.rs` - 動的値
  - `builder.rs` - Builder API
  - `playback.rs` - 再生状態
  - `validate/` - バリデーション（`rules.rs` 等）
  - `error.rs` - エラー型
  - `compile/` - 解決・型変換（`resolve.rs`, `types.rs`）
  - `cue/` - **cue 再生の唯一のエンジン**（`completed/areka-P0-cue-playback-duration` で統合）。`command.rs`（`Cue` envelope＝**一律 `duration: f64`**・`CueCommand` 10 variant・配送エンベロープ `TalkCue`）／`sheet.rs`（`CueSheet`＝`absolute_start_time` 付き**自己完結絶対時刻台本**＋canonical 変換 `to_talk_schedule` **1 本**＋duration の ingress clamp）／`schedule.rs`（`TimedSchedule`＋占有 horizon・`is_completed` は entry 枯渇 **かつ** horizon 到達）／`runtime.rs`（**`CuePlayer`**＝受動的注入時刻ランタイム・状態機械・バリア seam・Choice 先積み・**broadcast fan-out**）／`sink.rs`（**`CueSink`** 単一出力契約＋`cue_target_of` relevance 単一権威）。**受動ライブラリ**（スレッド/channel を持たず、アクター化は上流 sakura の領分）
  - `runtime/` - 実行系。ファサード、インスタンス管理、補間、購読管理、タイムライン、ループ制御をサブディレクトリモジュールに分割（`instance_manager/`, `interpolator/`, `subscription_manager/`, `timeline_manager/`, `loop_controller/` 等）
- `tests/` - テスト

> **モジュール分割パターン**: 600行リファクタ（`oversized-file-refactor`）以降、肥大化したファイルは `{module}/mod.rs` + サブモジュールのディレクトリ形式へ分割する方針。dola `runtime/` がその代表例。**なお「600行」は当時の spec 名であって現行の閾値ではない——今の目安は上の Unit Tests に記した 1 ファイル 1,000 行以下であり、本番ファイル・テストファイルの双方に適用する。**
>
> **ファサード形式も可**（`areka-P0-file-slimming` で追加）: 元ファイル `foo.rs` をファサードとして残し、本番項目を `foo/` 配下のサブモジュールへ純移動して `pub use` で再輸出する形でもよい。**呼び出し側を 1 箇所も変えずに済み、公開 API が完全に不変**なので、外部参照の多いファイルにはこちらが向く。代表例は `crates/areka/src/placement/follow.rs`（**2,032 → 122 行**）と `crates/areka/src/emo2_boot/frame.rs`（**1,532 → 201 行**）——いずれもファサード分割そのものによる減少である。`follow.rs` はこれとは別に、先行するテスト分離（同 spec のテーマ分割）で 8,472 → 2,032 行まで減っている。**2 つの機構の効果を足し合わせて 1 つの数字として語らないこと。**
>
> ファサード分割で踏む固有の注意点:
> - **サブモジュールは私有 `mod` で宣言する**。`pub mod` にすると `foo::bar::X` という新しい公開パスが生えて公開面が変わる。
> - **サブモジュールから見た `super` はファサード自身**であって親モジュールではない。兄弟モジュールへの `use super::sibling::…` は壊れる。ファサード側にその `use` 束縛を残し、子は `super::sibling` で辿るのが素直（`super::super::` でも通るが読みにくい）。
> - **テストモジュールが参照していた私有項目はファサードで再束縛する**。可視性キーワード無しの素の `use` で足りる（`pub` へ格上げしないこと）。子孫モジュールからは従来どおり見える。
> - **ファサードの `pub use` 再輸出は `unused_imports` を出しうる**。消費者が `#[cfg(test)]` のテストモジュールや examples しか無い項目は、非 test ビルド単位で未使用と判定されるため。**lib ターゲットの有無で決まるのではない**——決めるのは「その再輸出に非 test ビルドの消費者が居るか」だけである。区別すべき 2 種類がある:
>   - **内向き**（子モジュール同士が親の束縛を引く）——**子が `use super::{…}` でファサード経由に統一すれば 0 件に抑えられる**。この形は lib ターゲットを持たないサンプルバイナリでも 3 度成功している。
>   - **外向き**（テスト・examples など、クレート外あるいは非 test ビルドに現れない消費者のための再輸出）——**内向きの形では消えない。** `#[allow(unused_imports)]` で抑えてよいが、**死んだ再輸出を握り潰していないことを `cargo rustc -p <crate> --bin <bin> -- --force-warn unused_imports`（lib があれば `--lib --profile test` も）で確認し、属性のコメントには実際に未使用な名前だけを書く**こと。実例は `crates/areka/src/placement/follow.rs:60,62,68` の 3 文で、子はすべて `use super::{…}` を使っているにもかかわらず残っている。
> - **`super::`／相対の intra-doc リンクは指す先が変わる**。doc コメントは項目本文の内側にあるため、直すと「本文がバイト単位で不変」という純移動の証明が崩れる。`cargo doc` はゲートではないので**一律未修正が正解**。
> - **純移動であることは機械的に証明できる**——移設前ファイルを `git show <base>:<path>` で取り出し、属性と先行コメント塊つきの最上位項目へ括弧の釣り合いで分解して、移設後の連結と 1 対 1 突合する。許容してよい差分は「クレート内可視性キーワードの付与」と「意味の変わらない整形の折り返し」だけである。

**Dependencies**: `serde` + feature flags (`json`, `toml`, `yaml`) ＋ `interpolation`, `rand`, `pasta_core`

### Application Binary Crate
**Location**: `/crates/areka/`  
**Purpose**: デスクトップマスコット・プラットフォーム本体  
**Status**: 試作実装（シェル+バルーン2ウィンドウ表示、ドラッグ移動、ダブルクリック終了）＋ SHIORI 契約チェーン e2e（`shiori_host`/`shiori_session`/`reference_brain`＝native 脳デモ・`shiori_create` 入口）  
**Dependencies**: wintf, human-panic, thiserror, tracing, tracing-subscriber, async-io, bevy_ecs, windows

### Parser Crate
**Location**: `/crates/areka-parsers/`
**Purpose**: 伺か資産（さくらスクリプト / surfaces.txt / balloon descript / ghost descript.txt）の**純粋パーサ群**（M1 の②parsers トラック・**2026-07-02 全モジュール実装完了**）。UI・COM 非依存。
**Modules**（foundation 2 ＋ parser 4）:
- `charset/` - **共通基盤**: BOM 読飛→冒頭 ASCII プリスキャン→charset 宣言/既定 encoding_rs 再デコード（全パーサー共通の入口）
- `kv/` - **共通基盤**: KV 読み込み（素朴 BTreeMap・後勝ち・trim）
- `sakura/` - さくらスクリプト emo2 subset→token（パターン確立元）
- `shell/` - surfaces.txt→SERIKO/2.0 subset 型付きモデル（四層 model←lexer←decode←parse）
- `balloon/` - balloon descript→幾何＋フォント型付きモデル（descript＋画像別の後勝ち2層マージ）
- `package/` - `ghost/master/descript.txt` 起点の SHIORI/shell 2点マウント解決（`install.txt` は NAR 配置マニフェスト＝起動時不使用でスコープ外）
**Pattern**:
- API: `pub fn parse(&str) -> Model`（**`Result` 無しの寛容パース**・未知は passthrough/`Raw` へ吸収）。**例外**: `package` は致命失敗3種（起点不在/読取不能/shell 不在）を `MountError` で観測可能化
- **parse は忠実な転記層**（範囲非展開・記述子保持）＝展開・実ツリー構築は下流のエンジン構築側
- 値型: NewType＋opaque inner＋read-only accessor、enum は `#[non_exhaustive]`（拡張シームのみ）、未指定は `Option`（`None` と `Some(0)` を区別）
- テスト: in-source `#[cfg(test)]`、ukadoc 準拠自前テスト主軸＋emo2 実 fixture スモーク・**過剰実装禁止**（emo2 使用分のみ）
**Dependencies**: `tracing`＋`encoding_rs`（意図的追加・承認済。外部パーサ非依存）

### Unified Property System Crate（sylphya）
**Location**: `/crates/areka-sylphya/`
**Purpose**: 「名前で引ける値」の**唯一の解決機構**（`areka-P0-sylphya` 2026-07-24 完了）。%フラット名前空間と点付きプロパティ木を**単一名前空間の 2 つの窓**として提供し、%環境変数解決器／専用永続ストア／`ShioriHostSink` プロパティストアの 3 箱分裂を解消した。
**Architecture**: **掲示板（マテリアライズド・ビュー）＋単一同期アクター** — 読みは共有読みハンドル（epoch 交換の不変スナップショット）で**同期・無待機**、供給（publish／SET 中継／永続書込）は古典スレッド 1 本の同期アクターが所有。同期読み経路でのクロスアクター pull 照会は禁止。
**Modules**:
- `key.rs` - 正準 key `PropPath`／セレクタ 5 形パーサ（正準文字列化 `to_canonical_string()` が**唯一の権威**）
- `vocab/` - 語彙台帳（`flat.rs` 26 トークン／`dotted.rs` ルート枝 10＋汎用名 17＋SET 意味論／`shiori_resource.rs` 159 項目）＝**完全語彙を第一級保持**し未導出は縮退シームで管理
- `mirror.rs` / `reader.rs` / `value.rs` / `asker.rs` - 不変鏡像（per-asker/global 区画・epoch 単調増加）と `SylphyaReader`（`resolve_flat`／`resolve_dotted`／`talk_snapshot`）・問い合わせ元コンテキスト第一級
- `actor.rs` - `SylphyaMsg` envelope・**純関数中核 `SylphyaCore::apply`**（判断分岐を集約・受信ループは薄い配線）・`spawn_sylphya`／`SylphyaPublisher`（`barrier()` で反映フェンス）
- `persist/` - 層別スコープ（App/Ghost/Shell/Balloon）×TOML（`format-version`）×原子的書込（temp→rename）×寛容読取 3 段×4 key 族
**Dependencies**: **std・thiserror・tracing・toml・areka-actor のみ**（上流 areka クレートへの依存は**禁止**＝最下層規律。「消費者は backing を知らない」はこの依存方向から自動帰結）
**Consumers**: `areka-ghost`（結線・静的構成 publish・provider `SystemVarWiring::FromSylphya`）／`crates/areka` bin（`ShioriHostSink` 委譲）。`areka-kanade` は **sylphya へ依存しない**（`ResourceSink` クロージャで疎結合）

### SHIORI ABI Crate
**Location**: `/crates/shiori-abi/`
**Purpose**: 脳（SHIORI）との**内部唯一 ABI**。`IShiori`/`IShioriHost` のカスタム COM 定義（HSTRING/UTF-16・IID 既定義）＋エルゴノミック変換層。UI 基盤（wintf）に依存させない最小依存クレート（下流 32bit ホスト/pasta が同 ABI を共有）。x64 native 脳は in-proc COM、過去互換は 32bit Rust ホスト（host-32）が IPC 越しに同 ABI を実装。

### Host-32 Crates（32bit SHIORI ブリッジ・3クレート）
**Location**: `/crates/shiori-host32-ipc/`・`/crates/shiori-host32-host/`・`/crates/shiori-host32-helper/`
**Purpose**: x64/arm64 の areka から 32bit SHIORI DLL（emo2 の `pasta.dll` 等）を駆動する過去互換ブリッジ（M1 ①shiori トラック・`areka-P0-host32-ipc` 2026-07-02 完了）。
**Pattern**（ターゲットを **crate 境界で分離**＝`cfg` 分岐回避）:
- `shiori-host32-ipc` - プロトコル定義（bytes-over-wire・両側共有）
- `shiori-host32-host` - x64＋arm64 側ホスト
- `shiori-host32-helper` - **i686 専用** helper 実行体（`wintf-winmsg-executor` の message pump 上）
- トランスポートは **WM_COPYDATA 一本化＋再入 RESPONSE**（named pipe 不要）。x64⟷x86 を跨ぐのは生バイト列のみ
- 下流ユニット（shiori-load／request／lifecycle）はこの seam の上に増分

### Pilot (Two-Tunnel Knowledge) Crate
**Location**: `/crates/pilot/`
**Purpose**: 二坑モデルの**先進坑（pilot・使い捨て）知見クレート**。**空（最小）`lib.rs` ＋ 探索コードは `examples/<spec-name>/` のみ**という構造で葉ノード隔離を担保（出荷グラフから被依存しない＝可逆性の構造的担保）。完了 pilot はここへ隔離保全（例: `examples/pilot-clickthrough-alpha-toggle/`・`examples/shiori-host-32/`）。規律の正本は `.kiro/steering/two-tunnel.md`。

### Vendored: pasta DSL Engine
**Location**: `/vendors/pasta/`（git サブモジュール）  
**Repository**: [https://github.com/ekicyou/pasta](https://github.com/ekicyou/pasta)  
**Purpose**: 里々インスパイアの会話記述DSLスクリプトエンジン  
**Integration**: `[patch.crates-io]` で `pasta_core` をローカルパスへ差し替え、ワークスペース内で協調開発する。dola が直接依存し、areka は wintf/dola 経由で利用する。クローン時は `git submodule update --init` が必要

## Import Organization

```rust
// 標準ライブラリ
use std::sync::Arc;

// 外部クレート（アルファベット順）
use bevy_ecs::prelude::*;
use windows::UI::Composition::*;

// 内部モジュール（相対パス）
use crate::com::wuc::*;
use crate::ecs::window::*;
```

## Code Organization Principles

- **レイヤー分離**: COM→ECS→Message Handlingの依存方向を厳守
- **COMライフタイム**: `windows-rs`提供のスマートポインターで直接管理
- **unsafe隔離**: `unsafe`ブロックはCOMラッパー層に集約し、安全なAPIを上位層に提供
- **モジュール独立性**: 各モジュールは独立してテスト可能な単位として設計
- **テスト入口の固定化**: `tests/{domain}.rs` は束ね役に留め、実テストはドメイン配下へ寄せる

---
Workspace構成により将来的な機能拡張（別クレート追加）が容易。
