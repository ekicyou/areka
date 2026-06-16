# W3a-V: wintf コンポジタ・描画 × 脆弱性レビューと非破壊対策

- status: completed
- commit: fix(W3a): コンポジタ・描画系の unsafe 境界に SAFETY/NOTE コメント・debug_assert 2件・境界値特性化テスト 6件を追加

## findings

### 1. unsafe 境界

#### compositor.rs — DIB/HBITMAP ハンドリング
- `create_dib_section` の早期リターン経路を点検: CreateCompatibleDC 失敗時は `DeleteObject(hbitmap)` で解放済み（健全）。一方「有効 hbitmap + null dib_bits」のガード経路は hbitmap を解放せず Err を返す GDI リーク経路だが、CreateDIBSection の成功契約（有効戻り値 ⇒ ppvBits 非 null）により **API 契約上到達不能** であることを確認。NOTE コメントを付記し、エラー経路の整備（解放追加）は挙動変更のため **P42** に記録。
- `SelectObject` の戻り値未検査（万一失敗すると 1x1 ストックビットマップのまま ULW へ空内容が提示される無音縮退）も同様に NOTE 付記 + P42 に記録。なお Drop の解放順（DeleteDC → DeleteObject）は正しく、ストックビットマップ復元の省略による GDI リークはない（NOTE で根拠明記）。
- `biWidth: width as i32` / `biHeight: -(height as i32)` の変換: 唯一の呼び出し元 create_inner が直前に同寸法の D2D CreateBitmap（最大ビットマップサイズ ≦ 16384）を成功させているため、i32 損失・負化・負号オーバーフローは到達不能。根拠コメント + `debug_assert!(width <= i32::MAX && height <= i32::MAX)` を付加（compositor.rs:113-122 付近）。
- `unsafe impl Send/Sync for WindowD3D11Compositor` に SAFETY コメントを付加: D2D 系は MULTI_THREADED ファクトリ系列で内部同期、GDI ハンドル/dib_bits の使用箇所（composite_render_system / ulw_present_system / Drop）はすべて `&mut` 経由で ECS 借用規則により同時アクセス不能。

#### compositor_systems/render.rs — ULW present・transmute・from_raw_parts
- geometricMask `transmute` の COM リーク（P38）: W3a-S で `geometric_mask_layer_params` ヘルパに単一化済み・P38 記録済みを確認。他に transmute サイトなし（`std::mem::zeroed()` の opacityBrush は ManuallyDrop<Option<...>> の None 表現で健全、P38 の suggestion でカバー済み）。再記録せず。
- DIB ピクセルダンプの `from_raw_parts(dib_bits, total_bytes)`: cached_size と DIB は new()/resize() で常に同時設定されるため total_bytes は確保サイズ（w×h×4、32bpp でパディングなし）と一致し OOB なし。乗算オーバーフローも CreateDIBSection が同じ積の確保に成功している事実から排除。SAFETY コメントを付加。
- `draw_with_opacity` の行列バイト列 `from_raw_parts`: スタック上構造体の `size_of` ちょうどで健全。SAFETY コメントを付加。
- `ulw_present_system` の `w as i32` / `h as i32`: is_valid() ⇒ 生成成功 ⇒ w/h ≦ デバイス最大サイズの不変条件で損失なし。根拠コメント + debug_assert 2件相当（w/h 一括）を付加。

#### components.rs — unsafe impl Send/Sync（3 箇所）
- WindowGraphics / VisualGraphics / SurfaceGraphics の `unsafe impl Send/Sync` に SAFETY 条件コメントを付加: D2D 系は内部同期されるが DComp 系（target/visual/surface）は外部同期前提であり、ECS スケジュール構成（&mut 排他または非並行配置）が安全性条件を担う旨を明文化（W1-V の SendWeak と同系統の「前提の明文化」。スケジュール構成自体の検証は本セル境界外）。

### 2. panic 経路（expect/unwrap）

- `systems/clip_sync.rs:140` の `.cast().expect(...)`: IDCompositionRectangleClip → 基底 IDCompositionClip の QI はデバイス状態に依存しないインプロセス呼び出しで、直前の create_rectangle_clip 成功がオブジェクト生存を保証するため **デバイスロスト時を含め発火不能**。根拠 NOTE を付記。
- `systems/init.rs:243` の `dcomp_resource.unwrap()`: `!dcomp_valid` 分岐が全経路 return するため到達時は必ず Some。到達不能根拠の NOTE を付記。
- **デバイスロスト時の D2D/D3D Err を unwrap する箇所はゼロ**: composite_render_system（BeginDraw は戻り値なし、EndDraw/CopyFromBitmap/transfer_to_hbitmap は error ログ + continue）、render_surface（begin_draw/end_draw とも error ログ + continue）、deferred_surface_creation_system / clip_sync_system（全 COM 失敗が error ログ + continue/スキップ）。GPU リセットによる panic DoS 経路は **本セル境界内に存在しない** ことを確認。

### 3. デバイスロスト時のリソースリーク（分析）

- 無効化経路は健全: `invalidate_dependent_components` → `compositor.invalidate()` → inner Drop で GDI（DeleteDC/DeleteObject）・COM（スマートポインタ Release）とも解放。`compositor_init_system` の再作成失敗時も `invalidate()` で旧リソースを確実に解放。`resize()` 失敗時は新リソース未作成のまま旧状態維持（リーク・不整合なし。`resize_failure_keeps_previous_resources_and_state` で特性化）。
- **根本問題はリークではなく検出の不在**: プロダクションコードに `GraphicsCore::invalidate()` の呼び出しが存在せず（grep で確認、テスト/example のみ）、EndDraw の D2DERR_RECREATE_TARGET は error ログのみ。復旧機構が永久に発火せず ULW ウィンドウが最終フレームで固まる可用性縮退を **P40** として記録（EndDraw エラー経路へ NOTE 付記）。DComp 側も Phase 2 以降 invalidate_dependent_components の対象外で同根（P40 の設計判断に含めた）。

### 4. 整数変換

- `compositor_systems/init.rs` の `size.width as u32`（SizeI = i32）: 負値がラップして巨大値となり（-1 → 4294967295）、`w == 0` ガードを素通りして生成試行 → D2D CreateBitmap Err → error ログで完結（panic/UB なし）。NOTE 付記 + 特性化テスト（init システム経由・new 直呼びの両方）で固定し、事前検証の追加（ログ挙動変更）を **P41** に記録。
- `systems/init.rs::calculate_surface_size_from_global_arrangement`: NaN 幅は `<= 0.0` 比較を素通りするが `NaN.ceil() as u32 == 0` の飽和キャストで width_px==0 ガードに収束（None）。+inf は u32::MAX へ飽和して Some を返し下流の create_surface Err で完結。NOTE 付記 + 特性化テスト 2 件で固定（有限性検証の追加は挙動変更のため未実施。D 系 P14 と同型の網羅性ギャップ）。
- ulw.rs の pitch/stride（W2-V で debug_assert 済み）の graphics/ 側呼び出し元 `transfer_to_hbitmap(staging, dib_bits, w, h)`: w/h は staging 作成時と同一の cached_size であり # Safety 契約（dib_bits ≥ w×h×4、width=staging 幅）を構造的に満たすことを確認。`stride = w as usize * 4` 等は拡大変換のみ。
- `dirty.requested_frame = frame_count.0 as u64`（surface.rs）: u32 → u64 拡大変換のみ。問題なし。

### 変更ファイル（すべて挙動非破壊: debug_assert・コメント・追加テストのみ）

- `crates/wintf/src/ecs/graphics/compositor.rs` — SAFETY コメント（Send/Sync）+ debug_assert 1件 + NOTE 2件（GDI エラー経路）
- `crates/wintf/src/ecs/graphics/compositor_systems/init.rs` — NOTE 1件（負サイズラップ）
- `crates/wintf/src/ecs/graphics/compositor_systems/render.rs` — SAFETY コメント 2件 + NOTE 1件（デバイスロスト未検出）+ debug_assert 1件（ulw_present の i32 変換）
- `crates/wintf/src/ecs/graphics/systems/init.rs` — NOTE 2件（unwrap 到達不能根拠・NaN/inf 飽和挙動）
- `crates/wintf/src/ecs/graphics/systems/clip_sync.rs` — NOTE 1件（expect 不可謬根拠）
- `crates/wintf/src/ecs/graphics/components.rs` — SAFETY 条件コメント 3件（Send/Sync）
- `crates/wintf/tests/graphics/compositor_lifecycle_test.rs` — 特性化テスト 3件追加（巨大サイズ Err・負値ラップ Err・resize 失敗時の状態保存）
- `crates/wintf/tests/graphics/compositor_init_system_test.rs` — 特性化テスト 1件追加（負 WindowPos.size の無 panic スキップ）
- `crates/wintf/tests/graphics/surface_optimization_test.rs` — 特性化テスト 2件追加（NaN → None・+inf → u32::MAX 飽和）
- `report/proposals.md` — P40・P41・P42 追記

## 検証（S2）

- BEFORE: 親検証済みベースラインを信頼（HEAD ffe761b・クリーンツリー・1351 passed / 0 failed）。再実行せず（セル指示に従う）。
- AFTER: `cargo build --workspace` 成功（exit 0）/ `cargo test --workspace` **1357 passed / 0 failed**（+6 は追加した特性化テスト。既存テストの変更・削除ゼロ）/ `cargo build --examples -p wintf` 成功（exit 0）。
- 追加した debug_assert 2件は debug プロファイルの全テスト実行（compositor lifecycle / render system / transfer テスト含む）で非発火を実証。

## flaky

- 既知フレーキー cue_performance bench を含め、全体実行 2 回とも 0 failed。隔離再実行不要（pass-through）。

## proposals

- P40: ULW 合成経路のデバイスロスト検出の欠如（`GraphicsCore::invalidate()` のプロダクション発火経路が不在 → 復旧機構が永久に発火せず ULW ウィンドウが固まる可用性縮退）
- P41: compositor_init_system の負サイズ入力の事前検証（i32 → u32 ラップによる巨大サイズ生成試行とエラーログスパムの解消）
- P42: create_dib_section の GDI エラー経路整備（契約上到達不能な hbitmap リーク経路の解放追加・SelectObject 失敗検査）
- 既知 P37（赤デバッグ枠・全画素スキャン）・P38（geometricMask transmute の COM リーク）・P39（render_surface 未使用パラメータ）は本セルの点検範囲と重複するが再記録せず（P38 はヘルパ単一サイト化済みを確認）。
