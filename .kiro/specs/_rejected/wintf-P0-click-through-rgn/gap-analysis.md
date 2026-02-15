# ギャップ分析レポート: wintf-P0-click-through-rgn

## 分析サマリー

- **最大リスク**: SetWindowRgn と WS_EX_NOREDIRECTIONBITMAP (DirectComposition) の互換性が完全に未検証。コードベースにも外部ドキュメントにも確証がない
- **要件用語の不一致**: 要件書の `HitTest::Opaque` / `HitTest::Client` は既存コードの `HitTestMode::Bounds` / `AlphaMask` / `NamedRegions` に対応。要件修正が必要
- **GDI リージョン API は未使用**: SetWindowRgn, CreateRectRgn, CombineRgn いずれもプロジェクト内にゼロ使用。完全新規実装
- **基盤は十分**: `GlobalArrangement.bounds`（物理ピクセル座標）、`HitTestMode` enum、ドラッグ状態管理、ECS World アクセスパターンが既に揃っている
- **リジェクション容易性は実現可能**: 独立モジュール + タイマー呼び出しパターンで既存コードへの影響を最小化できる

---

## 1. 現状アセットマッピング

### 1.1 HitTest システム

| 既存アセット | ファイル | 状態 |
|---|---|---|
| `HitTestMode` enum (`None`, `Bounds`, `AlphaMask`, `NamedRegions`) | `ecs/layout/hit_test.rs` L66-L82 | ✅ 利用可能 |
| `HitTest` Component | `ecs/layout/hit_test.rs` L100-L115 | ✅ 利用可能 |
| `hit_test()` / `hit_test_in_window()` 関数 | `ecs/layout/hit_test.rs` L455-L565 | ⚠️ ポイント単位。ビットマップ書き込みには使わない |
| WM_NCHITTEST → HTTRANSPARENT パイプライン | `ecs/window_proc/handlers.rs` + `nchittest_cache.rs` | ⚠️ 既存パイプラインは維持（SetWindowRgn と共存） |

**🔴 GAP: 要件用語の不一致**
- 要件書: `HitTest::Opaque`, `HitTest::Client` → **コードに存在しない**
- 正しいマッピング: `HitTestMode::None` = クリックスルー、それ以外（`Bounds` / `AlphaMask` / `NamedRegions`） = 不透明
- → **要件 Req 3 のアクセプタンス基準を修正必要**

### 1.2 ウィンドウ管理

| 既存アセット | ファイル | 状態 |
|---|---|---|
| `WindowStyle` コンポーネント (デフォルト: `WS_EX_NOREDIRECTIONBITMAP`) | `ecs/window.rs` L697-L714 | ✅ 利用可能 |
| `WindowPos` コンポーネント (物理ピクセルサイズ) | `ecs/window.rs` | ✅ ビットマップサイズ決定に使用 |
| `WindowHandle` コンポーネント (HWND) | `ecs/window.rs` | ✅ SetWindowRgn 対象として使用 |
| `WinStyle` ビルダー | `win_style.rs` | ⚠️ SetWindowRgn はスタイル変更ではないので使わない |

**🔴 GAP: GDI リージョン API 完全未使用**
- `SetWindowRgn`, `CreateRectRgn`, `CombineRgn`, `DeleteObject` いずれもゼロ使用
- `windows` crate 経由のバインディングは利用可能（`Win32::Graphics::Gdi` モジュール）
- 完全新規実装が必要

### 1.3 レイアウトシステム

| 既存アセット | ファイル | 状態 |
|---|---|---|
| `GlobalArrangement` Component (`bounds: D2DRect`, 物理px) | `ecs/layout/arrangement.rs` | ✅ 不透明領域の物理座標取得に最適 |
| `Arrangement` Component (ローカル配置) | `ecs/layout/arrangement.rs` | ✅ 参照可能 |
| `ArrangementTreeChanged` ダーティマーカー | `ecs/layout/arrangement.rs` | ✅ Req 5 のレイアウト変更検知に利用可能 |
| `Changed<Arrangement>` bevy_ecs 変更検知 | `ecs/layout/systems.rs` | ✅ 利用可能 |

**✅ GAP なし**: リージョン構築に必要なデータ（`GlobalArrangement.bounds` + `HitTestMode`）はすべて揃っている。

### 1.4 ドラッグ処理

| 既存アセット | ファイル | 状態 |
|---|---|---|
| `DragState` (thread_local!, 状態遷移) | `ecs/drag/state.rs` | ✅ `read_drag_state()` で読み取り可能 |
| `WindowDragging` マーカーコンポーネント | `ecs/drag/systems.rs` | ✅ ECS クエリでドラッグ中検知可能 |
| NCHITTEST ドラッグガード | `nchittest_cache.rs` L141-L155 | ✅ 既存（ドラッグ中は HTCLIENT 強制） |

**✅ GAP なし**: ドラッグ状態の読み取りは複数手段で可能。

### 1.5 DirectComposition

| 既存アセット | ファイル | 状態 |
|---|---|---|
| `GraphicsCore` (IDCompositionDevice3 等) | `ecs/graphics/core.rs` | ⚠️ 共存可能性が未検証 |
| `WindowGraphics` (IDCompositionTarget) | `ecs/graphics/components.rs` | ⚠️ SetWindowRgn がターゲットに影響するか不明 |
| Commit パイプライン | `ecs/graphics/systems.rs` | ⚠️ SetWindowRgn は独立実行で干渉回避が望ましい |

**🔴 GAP: DirectComposition + SetWindowRgn 互換性（最大リスク）**
- `WS_EX_NOREDIRECTIONBITMAP` ウィンドウに SetWindowRgn を適用した場合の挙動が不明
  - DComp Visual の描画がクリップされるのか？
  - DWM のヒットテスト領域だけが変わるのか？
  - そもそも SetWindowRgn の呼び出し自体が失敗するのか？
- **Research Needed**: 設計フェーズで最優先の実験検証が必要

### 1.6 タイマー/スケジューリング

| 既存アセット | ファイル | 状態 |
|---|---|---|
| VSync スレッド (`DwmFlush` + `PostMessageW`) | `win_thread_mgr.rs` L317-L351 | ⚠️ パターン参考（0.25秒タイマーは別途必要） |
| `WM_TIMER` ハンドラ枠 | `win_message_handler.rs` L1022 | ⚠️ 存在するが **未使用**（`None` 返却のみ） |
| `ecs_wndproc` | `ecs/window_proc/mod.rs` | 🔴 WM_TIMER 未ディスパッチ |
| `WM_USER` 空間 | `win_thread_mgr.rs` L53 | ✅ WM_USER+3 以降が空き |
| `message_window` (HWND) | `ecs/world.rs` L481 | ✅ タイマー設定先として使用可能 |

**🔴 GAP: 0.25秒タイマーの実装方式が未確定**

タイマー実装の選択肢:

| 方式 | メリット | デメリット |
|---|---|---|
| A. `SetTimer(message_window, ...)` + WM_TIMER ディスパッチ追加 | Win32標準、シンプル | `ecs_wndproc` に WM_TIMER 追加が必要 |
| B. 専用スレッド + `PostMessageW(WM_USER+3)` | VSync と同パターン、既存フロー類似 | スレッド管理オーバーヘッド |
| C. ECSシステム + フレームカウント制御 | ECS 統合、World アクセス容易 | フレームレート依存（VSync が不安定だと更新間隔も不安定） |

→ **設計フェーズで決定**

---

## 2. 要件別フィージビリティ

| Req | 要件 | フィージビリティ | 既存基盤 | GAP |
|-----|------|----------------|---------|-----|
| 1 | リージョン定期更新 (0.25s) | ⚠️ | message_window, WM_TIMER枠 | タイマーディスパッチ未実装 |
| 2 | ビットマップベース構築 | ✅ | - | 完全新規（GDI API） |
| 3 | エンティティ不透明領域書き込み | ✅ | GlobalArrangement, HitTestMode | **用語修正必要** |
| 4 | 解像度構成可能性 | ✅ | - | 定数定義のみ |
| 5 | レイアウト変更検知 | ✅ | ArrangementTreeChanged, Changed<> | ダーティフラグ連携 |
| 6 | ドラッグ時拡張 | ✅ | DragState, WindowDragging | 読み取りロジック追加 |
| 7 | DirectComposition互換性検証 | 🔴 | WS_EX_NOREDIRECTIONBITMAP | **Research Needed（最大リスク）** |
| 8 | クロスプロセスクリックスルー | ⚠️ | - | SetWindowRgn自体の動作検証 |
| 9 | パフォーマンス測定 | ✅ | tracing infrastructure | `Instant::elapsed()` + tracing |
| 10 | モジュール化/リジェクション | ✅ | 既存パターン（nchittest_cache等） | 独立モジュール新設 |

---

## 3. 実装アプローチ選択肢

### Option A: 独立モジュール + SetTimer（推奨）

**方針**: `ecs/click_through_rgn.rs` モジュール新設。Win32 `SetTimer` で 250ms タイマー。WM_TIMER ハンドラから独立関数を呼び出し。

- **新規ファイル**: `ecs/click_through_rgn.rs`（全ロジック集約）
- **既存ファイル変更 (最小)**: 
  - `ecs/window_proc/mod.rs` — WM_TIMER ディスパッチ追加
  - `ecs/mod.rs` — モジュール宣言追加
  - `ecs/world.rs` — `SetTimer` / `KillTimer` 呼び出し（初期化/終了時）

**Trade-offs**:
- ✅ Req 10 のリジェクション容易性に最も適合（SetTimer を KillTimer に変えるだけ、モジュール削除で完了）
- ✅ ECS レンダリングパイプラインに干渉しない
- ✅ 既存コードへの変更が最小（3ファイルのみ）
- ❌ WM_TIMER ディスパッチ追加が ecs_wndproc に必要
- ❌ WM_TIMER ハンドラ内で ECS World への borrow が必要（既存パターンあり）

### Option B: ECS システム統合

**方針**: ECS スケジュールに `region_update_system` を追加。フレームカウントで 250ms 間隔をエミュレート。

- **新規ファイル**: `ecs/click_through_rgn.rs`（リージョン構築ロジック）
- **既存ファイル変更**: `ecs/world.rs`（スケジュール登録）

**Trade-offs**:
- ✅ bevy_ecs のクエリシステムと自然に統合
- ✅ World アクセスがシステム引数で自動的に解決
- ❌ フレームレート依存（VSync が 60fps でない場合に 250ms が保証されない）
- ❌ リジェクション時にスケジュールからシステム削除が必要（Option A より複雑）

### Option C: 専用スレッド + PostMessage

**方針**: VSync スレッドと同様のパターンで 250ms タイマースレッドを作成。WM_USER+3 をメインスレッドに PostMessage。

- **新規ファイル**: `ecs/click_through_rgn.rs`
- **既存ファイル変更**: `win_thread_mgr.rs`（スレッド起動）、`ecs/window_proc/mod.rs`（WM_USER+3 ハンドラ）

**Trade-offs**:
- ✅ VSync スレッドと一貫したパターン
- ✅ タイマー精度が高い
- ❌ スレッド管理のオーバーヘッド
- ❌ 既存ファイルへの変更が Option A より多い

---

## 4. 複雑性・リスク評価

### 工数: **M（3–7日）**

- 新パターン（GDI リージョン API）の導入
- DirectComposition 互換性の実験検証に不確定要素
- ビットマップ→HRGN変換ロジックの実装
- ただし既存基盤（HitTest, Arrangement, Drag）が充実しており統合は比較的容易

### リスク: **High**

- **DirectComposition + SetWindowRgn 互換性**: 最大のリスク。SetWindowRgn が `WS_EX_NOREDIRECTIONBITMAP` ウィンドウで期待通りに動作するか未検証。描画が壊れる可能性、あるいは SetWindowRgn 自体が無効化される可能性あり
- **パフォーマンス不確実性**: ビットマップ→HRGN変換の負荷が 16ms 以内に収まるか未検証（特に大きなウィンドウサイズで多数の矩形領域を持つ場合）
- **実験的仕様**: 上記リスクが顕在化した場合、アプローチ全体を破棄する可能性あり

---

## 5. 設計フェーズへの推奨事項

### 優先決定事項

1. **DirectComposition 互換性の実験検証**（最優先）
   - SetWindowRgn + WS_EX_NOREDIRECTIONBITMAP の最小再現テストを設計に含める
   - 検証結果によりアプローチ継続/破棄を判断

2. **要件 Req 3 の用語修正**
   - `HitTest::Opaque` / `HitTest::Client` → `HitTestMode::Bounds` / `AlphaMask` / `NamedRegions`（不透明扱い）
   - `HitTest::None` → `HitTestMode::None`（クリックスルー）

3. **タイマー方式の選定**
   - Option A（SetTimer）を推奨。リジェクション容易性と実装シンプルさのバランスが最良

### Research Needed（設計フェーズで調査）

- [ ] SetWindowRgn + WS_EX_NOREDIRECTIONBITMAP の互換性実験
- [ ] ビットマップ→HRGN変換の最適アルゴリズム（スキャンライン vs 矩形合成）
- [ ] 大ウィンドウ（1920x1080等）でのパフォーマンス特性
- [ ] `CombineRgn` の上限（矩形数に対する O(n) 特性の確認）

### 推奨アプローチ

**Option A（独立モジュール + SetTimer）** を推奨。理由:
- Req 10（リジェクション容易性）に最適合
- 既存コードへの変更が最小（3ファイルのみ）
- ECS レンダリングパイプラインに非干渉
- 実験的仕様として「やめやすい」構造

---

_Analysis generated: 2026-02-15_
