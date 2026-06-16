# W3b-V: wintf グラフィックス資源 × 脆弱性レビューと非破壊対策

- status: completed
- commit: fix(W3b): グラフィックス資源の unsafe 境界に SAFETY/NOTE 注記・再ペアレント未検出とデバイスロスト stale の特性化テスト3件を追加

## findings

### 1. unsafe 境界

- **core.rs — `unsafe impl Send/Sync for GraphicsCore`（SAFETY コメント不在 → 付加）**: 全フィールドが内部同期を持つことを確認し根拠を明文化した。d2d_factory は `D2D1_FACTORY_TYPE_MULTI_THREADED` 生成（core.rs:100）で同一ファクトリ系列（d2d / d2d_device_context）への呼び出しは D2D が直列化、d3d は `D3D11_CREATE_DEVICE_SINGLETHREADED` 非指定でスレッドセーフ、dwrite は SHARED ファクトリ。DComp 系（外部同期前提）と異なり ECS スケジュール構成への追加前提を要しない点も注記。
- **dcomp_resource.rs — `unsafe impl Send/Sync for DCompGraphicsResource`（SAFETY コメント不在 → 付加）**: DComp デバイス（desktop / dcomp）は内部同期を持たない外部同期前提 API であり、ECS の Res/ResMut 借用規則とシステム配置が安全性条件を担う旨を明文化（W3a-V が components.rs の DComp 系 3 コンポーネントに付加したものと同一条件）。
- **command_list.rs — 誤解を招く既存コメントを是正**: 旧コメント「windows-rsのスマートポインタはSend+Sync」は事実に反する（自動で Send/Sync なら unsafe impl 自体が不要）。正しい根拠（ID2D1CommandList は MULTI_THREADED ファクトリ系列で D2D が内部直列化）へ SAFETY コメントを書き換え。コメントのみの変更で挙動非破壊。
- **visual_manager.rs:132 — `unsafe { target.SetRoot(visual) }`**: 境界内で唯一の生 unsafe ブロック。target / visual は直前の二重 Some ガードで有効性確認済み・失敗は HRESULT 報告で UB 経路なしを確認し SAFETY コメントを付加。`let _ =` での失敗黙殺は意図的（デバイスロスト時のみ失敗し復旧は invalidate 系統の責務）だが、検出経路の不在（P40）と DComp 再初期化の不完全性（所見3）に接続する旨を NOTE で明記。
- dcomp_resource.rs に「generation 管理」は存在しない（generation は components.rs の WindowGraphics 側 = W3a 境界）。invalidate 後の use-after-invalidate は全アクセサが Option 返却のため構造的に防護されている（W3b-T 追加の core_accessor_test で Some→None 遷移固定済みを確認）。

### 2. 生成/破棄対称性 — 再ペアレント未検出による孤立 Visual（→ P47）

`visual_hierarchy_sync_system`（systems/visual_sync.rs）の未同期検出は `parent_visual().is_none()` のみで、ドキュメントの「ChildOf変更...を検出」という主張と実装が乖離している:

- **再ペアレント（ChildOf A→B）は検出されない**: 子の parent_visual キャッシュは Some(A) のままで affected_parents に入らず、DComp Visual は旧親 A に接続されたまま ECS 階層と乖離する。
- **旧親の再同期で孤立 Visual が発生**: その後 A が別の未同期子により再同期されると `remove_all_visuals` で切り離されるが、A.Children に居ないため再追加されず、B へも接続されない（画面から消失）。parent_visual キャッシュは実体と乖離した旧親参照を保持し続ける。
- **既存テストがギャップを隠蔽**: `test_childof_change_moves_visual_to_new_parent`（hierarchy_sync_test.rs）はテスト名に反して `parent_visual().is_some()` しか検証せず、stale キャッシュでも通過する。
- 本セルで `tests/visual/hierarchy_reparent_gap_test.rs`（2件、`Interface::as_raw` のポインタ同一性検証）により現行挙動を固定。初回実行でソース解析からの導出どおり全件一致（RED 代替の独立導出）。修正は挙動変更のため **P47** に記録。なおプロダクションに実行時再ペアレント経路は現状存在せず（grep 確認）潜在バグの段階。
- despawn 側の対称性は健全: VisualGraphics の on_remove フック（components.rs、W3a 境界）が親から remove_visual し、COM スマートポインタ Drop が Release を担う（W3a-T 実証済み）。`remove_all_visuals` → Children 順再追加方式自体も Z-order 一本化の設計意図どおり（W3b-T 所見6）。

### 3. デバイスロスト再初期化経路 — REINIT 側の不完全性（P40 の深掘り）

P40（検出の不在）の先、「仮に `GraphicsCore::invalidate()` が呼ばれたら再初期化は正しく機能するか」を全経路追跡した:

- **スケジュール順序は健全**: Update（invalidate_dependent_components）→ PreLayout（init_graphics_core）→ GraphicsSetup（init_window_graphics）の順（world/mod.rs:159/171/213）。ロスト検出がフレーム後半（Composition 等）で起きれば、次フレームの Update で依存無効化 → PreLayout で GraphicsCore 再作成 + `HasGraphicsResources.set_changed()` 一括発火、という順序自体は正しい。
- **ULW 側は復旧する**: WindowD3D11Compositor / BitmapSourceGraphics は invalidate_dependent_components が無効化し、compositor_init_system が generation 引き継ぎで再作成（W3a の既存テストで固定済み）。
- **DComp 側は復旧しない（不完全）**: Phase 2 で invalidate_dependent_components から WindowGraphics / VisualGraphics / SurfaceGraphics が除外された一方、再初期化側は `!is_valid()` を再作成の前提条件とする（init_window_graphics:287 の `if !wg.is_valid()`、visual_resource_management_system:95 の `if !vg.is_valid()`）。誰もこれらを invalidate しないため、`Changed<HasGraphicsResources>` でクエリにマッチしても旧デバイス由来の COM ポインタを保持したまま「有効」と判定され続け、再作成は永久に走らない。components.rs の HasGraphicsResources ドキュメント（「set_changed() で VisualGraphics, WindowGraphics 等の再初期化をトリガー」）は DComp 側では実態と乖離している。結果: DCompGraphicsResource だけが新デバイスへ再作成され、旧デバイスの target/visual/surface と新デバイスが混在（SetRoot/AddVisual は HRESULT エラーまたは無音で死んだ合成ツリーに作用）し、DComp ウィンドウは固まったままになる。
- 現行挙動は `tests/graphics/window_pos_systems_test.rs::invalidate_leaves_dcomp_graphics_components_stale` で固定（DCompGraphicsResource は無効化されるが VisualGraphics / SurfaceGraphics は stale-valid のまま）。window_pos.rs に NOTE を付加。無効化対象の再追加は挙動変更のため新提案とせず **P40 の設計判断（suggestion 末尾）に接続する分析として本断片に記録**（P40 の suggestion が既に「DComp 系を無効化対象へ再追加するか否かの設計判断を含める」と明記しており、再記録は重複のため回避）。

### 4. panic 経路・整数変換

- **境界 9 ファイルのプロダクションコードに unwrap / expect / panic! / 添字アクセス / 縮小整数キャストはゼロ**（grep 一括確認。`unwrap_or` / `unwrap_or_default` の全域版のみ）。デバイスロスト時の COM Err は全箇所 error ログ + continue / `let _ =` で panic DoS 経路なし（W3a-V の点検結果と整合）。
- **Visual offset の f32 演算**（visual_sync.rs:211-212 `offset × scale`）: 整数キャストを伴わず COM へ f32 のまま渡るため、NaN/inf でも panic/UB なし（DComp 側が HRESULT で拒否または受理）。clip.rs / visual.rs の負値・範囲外は warn + クランプで防護済み（既存 in-source テスト 13 件で固定済み）。
- **祖先走査の無限ループ前提（→ P48）**: `find_owner_window_composition_mode`（visual.rs、on_visual_add フックから同期呼出）・visual_sync.rs の深さ計算・`find_parent_brushes`（brushes.rs）の 3 箇所は ChildOf チェーンの終端到達を前提とし巡回ガードを持たない。bevy_ecs 0.18 は自己参照（A→A）を警告付き除去する（relationship/mod.rs:125 で実物確認）が間接巡回（A→B→A）は構築可能で、その場合 UI スレッドが恒久ハングする。通常 API では巡回は生成されないため NOTE 付記に留め、深さ上限ガードの追加（挙動変更）を **P48** に記録。
- generation カウンタ（WindowGraphics、W3a 境界）は `wrapping_add` 採用済みでオーバーフロー panic なし。init.rs:300 の `while wg.generation() < new_generation` は u32::MAX → 0 ラップ時もループ 0 回で正しく収束することを机上検証（W3a ファイルのため変更なし）。

### 変更ファイル（すべて挙動非破壊: コメント・追加テストのみ。W3a ファイル変更ゼロ）

- `crates/wintf/src/ecs/graphics/core.rs` — SAFETY コメント（Send/Sync）
- `crates/wintf/src/ecs/graphics/dcomp_resource.rs` — SAFETY コメント（Send/Sync）
- `crates/wintf/src/ecs/graphics/command_list.rs` — 誤コメントを正しい SAFETY コメントへ是正
- `crates/wintf/src/ecs/graphics/visual_manager.rs` — SetRoot unsafe ブロックに SAFETY + NOTE
- `crates/wintf/src/ecs/graphics/visual.rs` — 祖先走査の巡回前提 NOTE（P48）
- `crates/wintf/src/ecs/graphics/systems/visual_sync.rs` — 再ペアレント未検出 NOTE（P47）+ 深さ走査 NOTE（P48）
- `crates/wintf/src/ecs/graphics/systems/brushes.rs` — 祖先走査 NOTE（P48）
- `crates/wintf/src/ecs/graphics/systems/window_pos.rs` — DComp 再初期化不完全性 NOTE（P40 接続）
- `crates/wintf/tests/visual/hierarchy_reparent_gap_test.rs` — 新規・特性化テスト 2 件（+ tests/visual.rs に mod 追記）
- `crates/wintf/tests/graphics/window_pos_systems_test.rs` — 特性化テスト 1 件追加
- `report/proposals.md` — P47・P48 追記

## 検証（S2）

- BEFORE: 親検証済みベースライン（HEAD b976d4d クリーン・1389 passed / 0 failed）を信頼。作業ツリーの dola/ 配下の並行セル由来未コミット変更は境界外・未接触。
- AFTER: `cargo build --workspace` 成功（warning/error 0）/ `cargo test --workspace` **1392 passed / 0 failed**（1389 + 3 = 追加特性化テストと完全一致。既存テストの変更・削除ゼロ）/ `cargo build --examples -p wintf` 成功。
- 追加テスト 3 件は個別実行でも green を確認（reparent gap 2 件 + dcomp stale 1 件）。期待値はソース解析から実装と独立に導出し、初回実行で全件一致（RED 代替）。

## flaky

- 既知フレーキー cue_performance bench を含め、フル実行 2 回とも 0 failed。隔離再実行不要（pass-through）。

## proposals

- P47: visual_hierarchy_sync_system の再ペアレント未検出（parent_visual キャッシュ方式の盲点 — 旧親再同期時に Visual が孤立・キャッシュが実体と乖離）
- P48: ChildOf 祖先走査の巡回ガード欠如（間接巡回階層での無限ループによる UI スレッドハング。発生確率極小・優先度低）
- P40（デバイスロスト検出の不在）: 再記録せず、REINIT 側の不完全性分析（DComp 系コンポーネントが stale-valid のまま再初期化トリガー不発）を本断片・所見3 + stale 特性化テストとして接続。
