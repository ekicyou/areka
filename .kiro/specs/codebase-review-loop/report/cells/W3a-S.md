# W3a-S: wintf コンポジタ・描画 × シンプル化（unsafe 保守則適用）

- status: completed
- commit: refactor(W3a): 廃止済み dead code 3件を削除実証付きで除去・ClipGuard レイヤパラメータ構築を共通化・doc 見出し整理

## findings

### 適用 1: 申し送り dead code 3 件の削除（grep + build 実証、W1-S/W2-S 前例準拠）

W3a-T 所見 6 の 3 件すべてについて、リポジトリ全体 grep で参照ゼロ（プロダクション・テスト・examples とも）を実証したうえで削除した。

| 対象 | 場所 | 実証 | 同時整理した孤児 |
|------|------|------|------------------|
| `draw_recursive` | `systems/render.rs`（59 LOC、`#[allow(dead_code)]`、Phase 4 廃止のロールバック残置） | 唯一の参照は自己再帰呼び出しのみ。widget 側（rectangle.rs / label.rs）の言及は「draw_recursive 方式」という歴史的 NOTE コメントのみで API 参照なし | import の `bevy_ecs::hierarchy::Children` と `tracing::trace`（render.rs 内で他に使用なし） |
| `sync_surface_from_arrangement` | `systems/surface.rs`（128 LOC、`#[deprecated]` + `#[allow(dead_code)]`、surface-allocation-optimization spec の Req 2.1 で廃止決定済み） | スケジュール登録なし・呼び出しゼロ | 唯一の呼び出し先だった private `create_surface_for_visual`（27 LOC）も孤児化のため同時削除。import の `Arrangement` / `DCompositionVisualExt` / `windows::Win32::Foundation::*`（E_FAIL）を除去 |
| `init_window_visual` | `systems/init.rs`（20 LOC、本体が空コメントのみの deprecated 相当関数） | スケジュール登録なし・呼び出しゼロ（現責務は visual_resource_management_system） | import の `VisualGraphics` を除去 |

いずれも `pub` だが利用ゼロの廃止済み関数であり、削除は外部観測可能な挙動に影響しない（R2.9 / R5.3。ビルド・全テストで確認）。

### 適用 2: ClipGuard の `D2D1_LAYER_PARAMETERS1` 構築の共通化（テスト保護下の挙動非破壊簡素化）

`compositor_systems/render.rs` の `ClipGuard::push` で、`RoundedRectangle` / `RoundedRectangleIndividual` 両バリアントに完全同一の 16 行の構造体構築ブロックが重複していたため、私有ヘルパ `geometric_mask_layer_params(geo_mask, width, height)` へ抽出した。

- 当該経路は W3a-T のピクセル固定テスト（`rounded_rectangle_clip_clears_corners` / `individual_corner_clip_applies_per_corner_radii`）で保護されており、R5.5 の「テストで保護された経路の保守的な挙動非破壊簡素化」に該当する
- **P38 のリーク挙動（`transmute` による owned move）は意図的にそのまま保持**。ヘルパの doc コメントに P38 参照を明記した。修正時の変更箇所が 2 → 1 に減るため P38 実施の下準備にもなる

### 適用 3: `compositor.rs::new` の「# Safety」doc 見出し整理（doc のみ）

safe fn である `WindowD3D11Compositor::new` に `# Safety` 見出しが残置されていた（W3a-T 所見 6 末尾の申し送り）。safe fn への `# Safety` セクションは「呼び出し側に安全条件がある」という誤解を招くため、通常の前提条件文へ変更した。なお「dc は有効であること」は `&ID2D1DeviceContext` 型自体が保証するため記述から外し、サイズ前提（無効サイズは Err 返却）のみ残した。コード変更なし。

### S6 検証のうえ見送った候補（churn 回避・挙動変更回避）

1. **P37（赤デバッグ枠・全画素スキャン）**: characterization テストが赤枠 [0,0,255,255] を固定しており除去は挙動変更。不変のまま（提案済み、変更禁止の親指示も遵守）
2. **P38（ClipGuard COM リーク）**: unsafe ロジック変更のため不変のまま（提案済み）
3. **`render_surface` の未使用システムパラメータ 2 件**: 削除はリソースアクセス集合の変更（FrameCount 必須要求の消失等）にあたるため **P39 として記録**
4. **`mark_dirty_surfaces` の `count` 集計と空 if ブロック**: 挙動非破壊だが、リポジトリ慣習として残置されているコメントアウト済みデバッグログの除去を伴うため見送り
5. **`commit_composition` の match → let-else 等のスタイル変更**: 価値のない churn のため見送り（karpathy 3「壊れていないものをリファクタしない」）

### 差分サマリ

プロダクション 5 ファイル、+32 / −294（net −262 行）。テストの変更・削除なし（追加された 52 検出器を含む全テストが無変更で通過 = 挙動非破壊の裏付け）。W3b 担当ファイル（visual / visual_manager / clip / core / dcomp_resource / command_list / systems/{brushes,visual_sync,window_pos}）への変更なし。

### 検証（S2）

- BEFORE: 親指示に従いベースライン（クリーン HEAD d9844bd で 1351 passed / 0 failed）を信頼
- AFTER:
  - `cargo build --workspace` 成功（warning/error 0）
  - `cargo test --workspace` **1351 passed / 0 failed**（テスト数増減なし）
  - `cargo build --examples -p wintf` 成功

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue` が初回フル実行（並列実行で高負荷時）と隔離再実行 2 回で失敗（1.34〜1.80ms vs 1ms 閾値）。**本セルの変更を git stash で退避したクリーン HEAD でも失敗（1.338ms）することを確認**し、変更と無関係の環境負荷起因と実証した。負荷沈静後の最終フル実行（上記 AFTER）では当該テストを含む 1351 件全件が通過

## proposals

- P39: `render_surface` の未使用システムパラメータ（`_graphics_core` / `_frame_count`）削除（リソースアクセス集合の変更を伴うため記録に留めた）
- 適用済みのため新規提案なしの項目: dead code 3 件（削除完了）、`# Safety` 見出し（整理完了）、ClipGuard 重複構築（共通化完了。P38 修正時は単一箇所の変更で済む）
