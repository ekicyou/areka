# Research & Design Decisions: wintf-dpi-aware-layout

## Summary
- **Feature**: `wintf-dpi-aware-layout`
- **Discovery Scope**: Extension（既存レイアウトシステムの DPI スケーリング修正）
- **Key Findings**:
  - 修正対象は3関数＋1デモファイルに限定され、新規コンポーネント追加は不要
  - Echo/bypass ガードと DpiChangeContext の相互作用は安全（RAII ガード＋TLS で同期的に制御）
  - HitRegionMap（ColorMap）は正規化座標経由で自動スケーリングし、ボックスサイズ変更に追従する

## Research Log

### 座標系の正しい設計
- **Context**: requirements.md の REQ-1〜4 が求める「論理 px」座標系は、以前の正常動作状態と同じ設計であることの確認
- **Sources Consulted**: `update_arrangements_system`、`GlobalArrangement * Arrangement` 実装、gap-analysis.md §2, §6
- **Findings**:
  - GA.transform.M11 = 祖先 scale の累積積。Window.scale = DPI_scale → 子の GA.bounds = logical × DPI = physical ✓
  - LayoutRoot.scale = (1.0, 1.0) → Window.offset（物理 px）はそのまま物理座標として正しく機能
  - ラウンドトリップ検証（125%↔200%）で論理サイズ 600×420 が完全保存されることを数学的に確認済み
- **Implications**: 変換連鎖は既存の `GlobalArrangement * Arrangement` 実装で正しく動作する。新たな変換ロジック不要。

### Echo/Bypass とガードフラグの安全性
- **Context**: SWP_NOSIZE 除去後、WM_DPICHANGED → SetWindowPos → WM_WINDOWPOSCHANGED のフローで無限ループが発生しないことの確認
- **Sources Consulted**: `guarded_set_window_pos`（window.rs L140-166）、IS_SELF_INITIATED TLS、`SetWindowPosGuard` RAII、WM_WINDOWPOSCHANGED ハンドラ L134, L215, L275
- **Findings**:
  - `guarded_set_window_pos` は RAII ガードで `IS_SELF_INITIATED = true` を SetWindowPos スコープ内で維持
  - SetWindowPos は同期呼び出し → WM_WINDOWPOSCHANGED が同一スレッドで即座に発火 → `is_echo = true`
  - `DpiChangeContext` が `Some` の場合: `use_bypass = is_echo && dpi_context.is_none() = false` → bypass しない
  - `skip_box_style = is_echo && dpi_context.is_none() = false` → BoxStyle.size を更新する
  - WindowPos 更新時に値が変化した場合のみ `Changed<WindowPos>` が発火 → `sync_window_arrangement_from_window_pos` が新位置を Arrangement.offset に反映
  - **SWP_NOSIZE 除去後**: WM_WINDOWPOSCHANGED は新しいサイズ（suggested_rect由来）を受け取り、DPI除算で論理 px に変換して BoxStyle.size に設定 → Taffy が再計算 → 新 GA が物理座標と一致 → echo 不発 → 安定
- **Implications**: 既存のガードメカニズムにより、SWP_NOSIZE 除去は安全。追加の保護ロジック不要。

### HitRegionMap の座標マッピング
- **Context**: RegionTest 子要素のボックスサイズ縮小時に HitRegionMap 座標の手動更新が必要かの確認
- **Sources Consulted**: `hit_test_entity_ex`（hit_test.rs L400-430）、`hit_test_region`（hit_region.rs L371-383）、Shape 定義（hit_region.rs L83-84）
- **Findings**:
  - **ColorMap**: スクリーン座標 → 正規化座標（0.0〜1.0） → 画像ピクセル座標。ボックスサイズに自動追従。**更新不要**。
  - **Rect/Polygon**: スクリーン座標 → 正規化座標 → `rel * entity_size` で DIP ローカル座標に変換。Shape 定義は DIP 単位の絶対座標（例: 70.0, 75.0）。ボックスサイズが変わる場合、**Shape 座標の手動更新が必須**。
  - テスト `test_color_map_hit_test_region_non_square_entity` で 4×2 画像 × 200×50 エンティティの動作が検証済み。
- **Implications**: Rect/Polygon の HitRegionMap 座標はボックスサイズと連動して更新する必要がある。ColorMap は不要。

### DpiChangeContext のライフサイクル
- **Context**: DpiChangeContext が正しく消費され、残留しないことの確認
- **Sources Consulted**: window.rs L25-79（DpiChangeContext 定義と TLS 操作）、handlers.rs L144（take 消費）
- **Findings**:
  - `DpiChangeContext::set()` → TLS に格納（WM_DPICHANGED ハンドラ内）
  - `DpiChangeContext::take()` → TLS から取り出し＋クリア（WM_WINDOWPOSCHANGED ハンドラ冒頭）
  - SetWindowPos は同期呼び出し → WM_DPICHANGED 内で set → 即座に WM_WINDOWPOSCHANGED が発火 → take で消費
  - 1回の DPI 変更に対して正確に1回 set/take のペアが実行される
- **Implications**: ライフサイクルは既存設計で正しく管理されている。変更不要。

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| **A: 既存コンポーネント修正** | 3関数のパラメータ変更のみ | 最小変更量、既存テスト互換 | Arrangement 内の単位混在残存 | **採用** |
| B: Arrangement リファクタリング | offset/size の単位を型で分離 | 意味論の明確化 | 大規模リファクタリング、スコープ外 | 将来検討 |

## Design Decisions

### Decision: Window エンティティの Arrangement.scale に DPI を設定する
- **Context**: 現在 `LayoutScale::default()` で常に (1.0, 1.0) が設定されている
- **Alternatives Considered**:
  1. Window.scale = DPI（子は 1.0）→ 論理→物理変換が変換伝播で自動化
  2. 全エンティティで DPI 除算 → 各エンティティが個別に DPI 対応
- **Selected Approach**: Option 1 — Window のみ scale = DPI
- **Rationale**: 既存の `GlobalArrangement * Arrangement` 変換連鎖が正しく機能する。子エンティティは Taffy が論理 px で計算するため scale = 1.0 で正しい。
- **Trade-offs**: Arrangement 内で offset = 物理 px / size = 論理 px の混在が残るが、数学的に正しく動作する。
- **Follow-up**: 将来的に Arrangement の単位を型で明示化する可能性はあるが、本スコープ外。

### Decision: WM_DPICHANGED で SWP_NOSIZE を除去
- **Context**: 現在 SWP_NOSIZE でサイズ変更を抑制しているが、DPI 変更時にウィンドウの物理サイズを OS 推奨値に更新する必要がある
- **Alternatives Considered**:
  1. SWP_NOSIZE 除去 + suggested_rect サイズ使用
  2. SWP_NOSIZE 維持 + 別途 BoxStyle.size を手動計算
- **Selected Approach**: Option 1 — SWP_NOSIZE 除去
- **Rationale**: OS が suggested_rect で提供するサイズは `論理サイズ × 新DPI` の物理サイズであり、正確。WM_WINDOWPOSCHANGED が新サイズを受け取り、DPI 除算で元の論理サイズに戻る（ラウンドトリップ保証）。
- **Trade-offs**: なし。既存の echo/bypass ガードが安全性を保証。

### Decision: デモウィンドウサイズの具体値は実装時に決定
- **Context**: REQ-5 は「200% DPI モニターに収まること」が要件の本質
- **Selected Approach**: 設計では Window サイズの上限制約（200% モニター論理サイズ − 配置マージン）のみ定義し、具体的な数値は実装タスク内で決定する。
- **Rationale**: `find_non_primary_monitor_origin` のマージン計算や子要素レイアウトとの兼ね合いが実装時に最適化可能。

## Risks & Mitigations
- **R1: WM_WINDOWPOSCHANGED の発火パターン変更** — SWP_NOSIZE 除去で WM_SIZE も追加発火する可能性があるが、WM_SIZE は当該ハンドラで処理しておらず影響なし。
- **R2: Arrangement 単位混在による将来の混乱** — 本スコープでは受容。steering に設計メモを残すことで将来のリファクタリングの入口を確保。
- **R3: 端数丸め** — 論理→物理→論理の変換で浮動小数点の端数が生じうるが、f32 精度で実用上問題なし。

## References
- Microsoft: [Per-Monitor DPI Aware V2](https://learn.microsoft.com/ja-jp/windows/win32/hidpi/high-dpi-desktop-application-development-on-windows) — DPI 対応アプリケーション開発ガイドライン
- Microsoft: [WM_DPICHANGED](https://learn.microsoft.com/ja-jp/windows/win32/hidpi/wm-dpichanged) — suggested_rect の仕様
- gap-analysis.md §4 — 全エンティティの期待座標値（修正後の数値表）
- gap-analysis.md §6 — 座標ラウンドトリップ検証
