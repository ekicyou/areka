# W3a-T: wintf コンポジタ・描画 × テスト網羅性

- status: completed
- commit: test(W3a): コンポジタ・描画系 ECS システムにヘッドレスギャップテスト52件を追加・ClipGuard の COM 参照リーク等を記録

## findings

### ファイル担当の確定（W3a vs W3b）

`crates/wintf/src/ecs/graphics/` の一覧と mod.rs を確認し、以下の通り確定した。

- **W3a（本セル）**: `compositor.rs`、`compositor_systems/{mod,init,render}.rs`、`systems/{init,render,surface,clip_sync}.rs`、`components.rs`
- **W3b（次領域、不変更）**: `visual.rs`、`visual_manager.rs`、`clip.rs`、`core.rs`、`dcomp_resource.rs`、`command_list.rs`、`mod.rs`、`tests.rs`（in-source テスト）、`systems/{brushes,visual_sync,window_pos}.rs`、`systems/mod.rs`

W3b ファイルの型（`ClipShape` / `GraphicsCommandList` / `DCompGraphicsResource` / `GraphicsCore` 等）はテストフィクスチャとして読み取り専用で使用したのみで、W3b 側ソースの変更はない。

### モジュール×テスト対応表（改善前 → 改善後）

| モジュール | 対象 | 既存テスト | 追加 | 備考 |
|------------|------|-----------|------|------|
| `compositor.rs` (265 LOC) | `WindowD3D11Compositor`（new/resize/invalidate/dirty/generation/アクセサ/Debug/Send+Sync） | 15件（lifecycle 8 + integration 4 + transfer 3） | 0件 | ライフサイクル網羅は既存テストで十分。ECS システム経由の駆動は下記で補完 |
| `compositor_systems/init.rs` (137 LOC) | `compositor_init_system` の全分岐: 新規作成 / DComp スキップ / size None・0 スキップ / GraphicsCore 無効の早期リターン / リサイズ（generation+1）/ 同サイズ no-op / デバイスロスト再作成（generation 引き継ぎ） | 0件（integration テストはシステム相当ロジックの手動再現のみで、システム本体は未実行だった） | 8件 | `tests/graphics/compositor_init_system_test.rs`。実 GraphicsCore + Schedule(SingleThreaded) で ECS 駆動。HWND は不要（システムは WindowHandle の存在のみ要求） |
| `compositor_systems/render.rs` (832 LOC) | `composite_render_system`（bounds ベース合成位置 / ULW ウィンドウオフセット補正 / is_visible・opacity==0 スキップ / opacity 減衰 / ClipShape 3 バリアントの ClipGuard 経路 / is_window_dirty の初回・Changed・無変更スキップ / 無効 compositor スキップ）+ `ulw_present_system`（clean・invalid スキップ / 失敗時 dirty 保持リトライ） | 11件（opacity 累積式の再実装テストのみ。システム本体・クリップ・ピクセル出力は未検証） | 14件 | `tests/graphics/compositor_render_system_test.rs`。DIB ピクセル（BGRA）を直接読み出して合成結果を画素単位で検証。private な `ClipGuard`（PushAxisAlignedClip / PushLayer+RoundedRect / PushLayer+PathGeometry 円弧構築）と `draw_with_opacity`（ColorMatrix Effect）はシステム経由でピクセル観測により検証 |
| `systems/render.rs` (236 LOC) | `render_surface`（Changed<SurfaceGraphicsDirty> 駆動の begin_draw→clear→DrawImage→end_draw / invalid スキップ）+ `commit_composition`（リソースなし no-op / 有効 commit / invalidated スキップ） | 0件 | 5件 | `tests/graphics/surface_systems_test.rs`。実 IDCompositionSurface への自己描画を実行。`draw_recursive` は Phase 4 廃止のロールバック用 dead code（下記所見6） |
| `systems/surface.rs` (449 LOC) | `deferred_surface_creation_system`（物理サイズ作成 / 小数切り上げ / 無効サイズスキップ+統計 / 同サイズ no-op / リサイズ+統計 / dirty の +1 トリガー）+ `cleanup_surface_on_commandlist_removed`（クリア+統計 / invalid スキップ） | 3件（mark_dirty_surfaces のみ。deferred/cleanup はシステム実行テストなし） | 7件 | `tests/graphics/surface_systems_test.rs`。実 DComp デバイスで Surface 生成・SetContent まで実行。`sync_surface_from_arrangement` は #[deprecated] + dead code（下記所見6） |
| `systems/init.rs` (337 LOC) | `init_window_graphics`（DComp ウィンドウ検出時の DCompGraphicsResource 遅延初期化 / ULW のみなら未作成 / 無効 HWND の CreateTargetForHwnd 失敗経路 / GraphicsCore 無効の早期リターン） | 12件（format_entity_name 4 / calculate_surface_size 5 / init_graphics_core 3） | 4件 | `tests/graphics/init_window_graphics_test.rs`。WindowGraphics 本体の作成成功経路は実 HWND 必須（所見4）。`init_window_visual` は空実装の deprecated 関数（下記所見6） |
| `systems/clip_sync.rs` (214 LOC) | `clip_sync_system`（3 バリアント適用 / clip None 解除 / サイズ 0 解除 / リソース不在スキップ） | 0件 | 4件 | `tests/graphics/clip_sync_system_test.rs`。IDCompositionRectangleClip は write-only で読み戻し不能のため characterization（完走確認）に留める（所見5）。同等のクリップ意味論は ULW 側でピクセル検証済み |
| `components.rs` (285 LOC) | `VisualGraphics`（Default / new_with_parent / set_parent_visual / invalidate / Debug / **on_remove フックの親デタッチ** / 親なし・invalidate 後 despawn の安全性）+ `SurfaceGraphics`（set_surface 直接更新 / clear） | 12件（src/ecs/graphics/tests.rs の HasGraphicsResources 2 + SurfaceGraphicsDirty 2 + stats 6、reinit_unit_test の invalidate 2） | 10件 | `tests/graphics/components_test.rs`。on_remove フックは「despawn 後に親への remove_visual が失敗する」ことでデタッチ済みを観測的に証明。`WindowGraphics` はヘッドレス構築不能（所見4） |

追加テスト合計 **52 件**（8+14+4+12+10+4）。新規テストファイル 6 件 + `tests/graphics.rs`（束ね役エントリ、S9 準拠の `#[path]` 宣言のみ）への mod 追記。プロダクションコードの変更なし（R5.1 充足）。

### デバイス生成方針（前例準拠）

W2-T で確認済みの前例（`GraphicsCore::new()` ベースのヘッドレステスト約20件 + DComp デバイス生成）に従い、(a) ECS システムを `Schedule`（`ExecutorKind::SingleThreaded`、`src/ecs/world/mod.rs` の本番設定と同一）で実駆動し、(b) 合成結果は `WindowD3D11Compositor::dib_bits()` の DIB ピクセル直接読み出しで検証する方式を採用した。compositor 系の既存テストはロジックの「手動再現」だったため、本セルの追加分はクエリフィルタ（`Or<Changed<...>>`・`Without<...>`・`RemovedComponents`）と Commands 適用を含むシステム本体の実行経路を初めて固定する。

### テスト不能箇所・深掘り所見（R2.8）

1. **composite_render_system の赤デバッグ枠の常時描画（→ P37）** — 合成ビットマップ外周 2px に赤枠を無条件描画するブロックが残置（コメントに「DEBUG: 切り分け用」と明記、`cfg(debug_assertions)` ガードなし）。リリースビルドでも全 ULW ウィンドウに赤枠が出るユーザー可視アーティファクト。除去は挙動変更のため characterization テスト（`composite_render_sets_dirty_and_draws_debug_border` が [0,0,255,255] を固定）+ P37 記録に留めた。
2. **composite_render_system の無条件 DIB 全画素スキャン（→ P37）** — ピクセルダンプ用の `first_nonzero` / `nonzero_count` 計算が `trace!` マクロの外側の let 束縛で実行され、トレース無効時も毎合成 O(W×H) のスキャンが走る。性能特性の変更にあたるため P37 に併記。
3. **ClipGuard の geometricMask `transmute` による COM 参照リーク（→ P38）** — `D2D1_LAYER_PARAMETERS1.geometricMask: ManuallyDrop<Option<ID2D1Geometry>>`（windows-rs 0.61 定義で確認）へ owned 値を `std::mem::transmute`（move）しており、Release が永久に走らない。角丸クリップの push 1 回につきジオメトリ COM オブジェクトが 1 個リークし、毎フレーム再合成では無制限増加。修正は unsafe 変更のため P38 として記録し、本セルではクリップの観測挙動（角の透明化・per-corner 半径）をピクセル固定した。
4. **WindowGraphics はヘッドレス構築不能** — `WindowGraphics::new` は `IDCompositionTarget`（`CreateTargetForHwnd` = 実 HWND 必須）を要求し、Default も無いためテスト内で値を作れない。コード解析: 構造は検証済みの `SurfaceGraphics`/`VisualGraphics` と同型の Option ラッパー + generation 管理（`WindowD3D11Compositor` と同一パターンで、そちらは検証済み）でロジックを持たず、リスク残量は小。`init_window_graphics` の作成成功経路・`ulw_present_system` の UpdateLayeredWindow 成功経路も同様に実ウィンドウ必須（W2-T 所見6 と同根）。失敗・スキップ経路は今回テストで固定済み。
5. **clip_sync_system の適用結果は読み戻し不能** — `IDCompositionRectangleClip` は setter のみの write-only COM オブジェクトで、SetClip 適用後の検証 API が存在しない（DComp ツリーの観測は視覚出力のみ）。各分岐の完走 characterization に留め、クリップ形状の意味論（Rectangle / RoundedRectangle / Individual の per-corner 挙動）は同一 `ClipShape` を消費する ULW 側 `ClipGuard` のピクセルテストで間接的に固定した。スケール換算（`radius * scale_x` — Y 半径も X スケールを使う点を含む）はコード解析で確認、検証はデバイス出力目視のみ可。
6. **dead code 3 件は W3a-S へ申し送り** — (a) `systems/render.rs::draw_recursive`（Phase 4 廃止、コメント「ロールバック用に残している」、`#[allow(dead_code)]`）、(b) `systems/surface.rs::sync_surface_from_arrangement`（`#[deprecated]` + `#[allow(dead_code)]`、R2.9 の削除実証対象候補）、(c) `systems/init.rs::init_window_visual`（本体が空の deprecated 相当関数、ただし非推奨属性なし）。いずれも削除は挙動非破壊で W3a-S の整理候補。`compositor.rs::new` の安全な fn への「# Safety」doc 見出し残置も軽微な整理候補。
7. **compositor_init_system の generation 引き継ぎループ** — `while new_compositor.generation() < target_gen { increment }` は `old_generation == u32::MAX` のとき `target_gen == 0` となりループ非実行で 0 のまま（wrapping 意味論として一貫）。境界は実用上到達不能だが挙動は健全と判断。通常経路（0→1）はテストで固定済み。
8. **RED フェーズ代替の検証** — 追加テストは既存挙動の characterization のため RED は N/A。期待値（合成位置のピクセル座標・クリップ円弧の幾何距離・統計カウント・dirty セマンティクス）は実装と独立に Win32 契約とピクセル数学から導出して記述し、初回実行で 52 件全件が導出どおり一致した（導出と実装の相互裏付け）。

### 検証（S2）

- BEFORE: 親指示に従いベースライン（クリーン HEAD 6b7576f で 1299 passed / 0 failed）を信頼。なお作業ツリーには並行セル由来の dola/ 配下の未コミット変更が存在するが、本セルの境界外であり触れていない
- AFTER: `cargo build --workspace` 成功（warning/error 0）/ `cargo test --workspace` **1351 passed / 0 failed**（+52 はすべて追加分。既存テストの変更・削除なし）
- 変更はテストファイル5件の新規追加 + `tests/graphics.rs` の mod 追記 + report 2ファイルのみ。プロダクションコードの変更なし＝外部観測可能な挙動の変更なし（R5.1 充足）

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue` を含め、AFTER 2 回のフル実行（いずれも 1351/0）で失敗なし。隔離再実行は不要だった

## proposals

- P37: composite_render_system のデバッグ残置コード除去（赤デバッグ枠の常時描画・トレース無効時も走る DIB 全画素スキャン）
- P38: ClipGuard の geometricMask `transmute` による COM 参照リーク修正（角丸クリップの毎フレームリーク）
- 挙動非破壊の dead code 整理 3 件（draw_recursive / sync_surface_from_arrangement / init_window_visual）と doc 見出し整理は W3a-S へ申し送り（所見6）
