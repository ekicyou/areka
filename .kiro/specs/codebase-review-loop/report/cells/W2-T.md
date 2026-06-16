# W2-T: wintf COM層 × テスト網羅性

- status: completed
- commit: test(W2): COM ラッパー層に com ドメイン統合テスト79件を追加・未完成録画モジュール等の所見を記録

## findings

### モジュール×テスト対応表（crates/wintf/src/com/、改善前 → 改善後）

| モジュール | 対象 | 既存テスト | 追加 | 備考 |
|------------|------|-----------|------|------|
| `d2d/command_types.rs` (584 LOC) | 非 COM コマンド型（SetTags / SetAntialiasMode / SetTextAntialiasMode / SetPrimitiveBlend×3世代 / SetUnitMode / SetTransform / Clear / PushAxisAlignedClip / SetTextRenderingParams(None)）の構築・Clone・Debug | なし | 12件 | 純粋ロジック（引数→フィールド素通し格納）。COM 参照を要する型は `d2d_ext_test.rs` で実デバイス検証（`FillRectangle`/`DrawLine` の dup_com 経路2件を含む） |
| `d2d/command_sink.rs` (432 LOC) | `RecCommandSink` の COM ABI（vtable）経由コールバック: BeginDraw/EndDraw・状態設定群・null/有効ポインタ両経路（SetTransform/Clear/PushAxisAlignedClip/PushLayer）・`commands()` の todo! パニック・push/clear | なし | 9件 | `#[implement]` オブジェクトはデバイス無しで生成可能。COM 引数コールバック（DrawGlyphRun/DrawBitmap/FillGeometry 等）は `d2d_ext_test.rs` の `ID2D1CommandList::Stream` 再生で実データ検証 |
| `d2d/mod.rs` (306 LOC) | `D2D1FactoryExt`（path/rounded rect geometry）・`D2D1DeviceExt`（DC/CommandList 生成）・`D2D1CommandListExt`（open=E_NOTIMPL の特性化・close の1回成功/2回目失敗）・`D2D1DeviceContextExt`（set_transform/clear×2/brush/fill_rectangle/fill_ellipse/fill_geometry/draw_bitmap/draw_text/draw_text_layout/draw_image/create_bitmap_from_wic_bitmap） | なし（ECS 経由の間接実行のみ） | 7件 | 描画ラッパー一式を CommandList へ記録 → `Stream` で `RecCommandSink` へ再生する統合テストを含む。DC と別ファクトリのジオメトリ混在（D2DERR_WRONG_FACTORY）を避けるため `GetFactory()` 経由でジオメトリ生成 |
| `dcomp.rs` (335 LOC) | `DCompositionDeviceExt`（visual/surface/animation/transform×3種/clip/group/frame_statistics/commit）・`DCompositionVisualExt`（offset/opacity/backface/add・remove・remove_all/set_content/set_effect/set_clip_object/clear_clip）・`DCompositionSurfaceExt`（begin_draw None・Some(rect)/end_draw/suspend/resume）・`DCompositionRotateTransform3DExt`・`DCompositionMatrixTransform3DExt` | 一部間接（tests/visual・tests/graphics が create_visual/add_visual 等を経由実行） | 17件 | エラー経路3件を含む: 非子 Visual の remove 失敗・begin_draw 無しの end_draw 失敗・空アニメーションの SetAngle 拒否（所見4）。`create_target_for_hwnd`/`set_root` は実 HWND 必須のため対象外（所見6） |
| `d3d11.rs` (70 LOC) | `d3d11_create_device`（出力パラメータ Option→生ポインタ変換の Some/None 両経路）・`get_device_removed_reason`・`create_texture2d`（成功/初期データ付き/幅0エラー） | 間接のみ（GraphicsCore::new 経由） | 5件 | 幅0 → Err 伝播で `texture2d.unwrap()` が失敗時に到達しないことを確認（`?` が先行するため健全） |
| `dwrite.rs` (206 LOC) | `dwrite_create_factory`・`DWriteFactoryExt`（create_text_format/create_text_layout の手書き wcslen ループ: null PCWSTR・空文字列・ASCII・日本語・サロゲートペア）・`DWriteTextFormatExt`（alignment 設定・E_INVALIDARG 伝播）・`DWriteTextLayoutExt`（get_cluster_count/get_cluster_metrics 整合・hit_test_text_position leading/trailing・範囲外クランプ） | なし | 11件 | DirectWrite は GPU 不要でヘッドレス完結。サロゲートペア（U+20BB7）で「UTF-16 単位 2・クラスタ 1」を固定し wcslen ループの走査単位を特性化 |
| `wic.rs` (135 LOC) | `wic_factory`・`WICImagingFactoryExt`（decoder 生成: 正常/ファイル不在/不正コンテンツ）・`WICBitmapDecoderExt::frame`・`WICFormatConverterExt::init`（PBGRA32 変換）・`WICBitmapSourceExt`（get_size/copy_pixels: 全体・部分矩形・バッファ不足エラー） | 上位 API 経由の間接のみ（tests/widget/bitmap_source_integration_test.rs） | 8件 | com::wic の各 Ext トレイトを直接検証。既存アセット（test_8x8_rgba.png / invalid.bin）を再利用 |
| `animation.rs` (182 LOC) | `create_animation_timer`（非負・単調時刻）・`UIAnimationManagerExt`（variable 初期値/update/storyboard）・`UIAnimationTransitionLibraryExt`（2種遷移生成）・`UIAnimationStoryboardExt`（add_transition/keyframe API/schedule → 最終値到達の決定的検証）・`UIAnimationVariableExt`（get_value/get_curve） | なし（examples/dcomp_demo.rs でのみ使用） | 7件 | UIAnimation は CPU のみの COM コンポーネントでヘッドレス完結。`get_curve` は IDCompositionAnimation を要するため dcomp_test.rs 側に配置（計上は dcomp 側 17件に含む） |
| `ulw.rs` (105 LOC) | `transfer_to_hbitmap` | あり（tests/graphics/compositor_transfer_test.rs: 基本/各サイズ/pitch≠stride 誘発） | 0件 | 既存テストで十分。`present_layered_window` は実 HWND（WS_EX_LAYERED）必須のため解析のみ（所見6） |
| `dxgi.rs` (空) | — | — | 0件 | 改行のみの空モジュール（所見3） |
| `mod.rs` (8 LOC) | モジュール宣言のみ | — | 0件 | テスト対象なし |

追加テスト合計 79 件。新規ドメイン `tests/com.rs`（束ね役エントリ、S9 準拠）+ `tests/com/` 配下 7 ファイル + 共通ヘルパー `tests/com/common/mod.rs`（CoInitializeEx ガード・アセットパス解決は `tests/widget/bitmap_source_integration_test.rs` の既存パターンを踏襲）。プロダクションコードの変更なし。

### 除外テスト

0 件（com/ 配下に既存ユニットテストが存在しないため除外対象なし）。

### デバイス生成前例の確認（テスト方針の根拠）

既存テストスイートに実デバイス生成のヘッドレス前例が確立している: `GraphicsCore::new()`（D3D11 HARDWARE + D2D + DWrite）を tests/graphics の約20テスト、DComp デバイス生成を tests/visual の4ファイルが使用し、ベースライン（1219 passed / 0 failed）で全合格。本セルはこの前例に従い、(a) 純粋ロジック（command_types・RecCommandSink の vtable 経路・wcslen 境界）を最優先で固定し、(b) デバイス必須のラッパーは既存前例と同一経路（core.rs::create_device_3d と同パラメータ）で実デバイス検証した。

### テスト不能箇所・深掘り所見（R2.8）

1. **d2d 録画モジュールは未完成の利用ゼロコード（→ P33）** — `command_sink.rs`/`command_types.rs`（計 1,016 行 = com/ 全体の 43%）はワークスペース内利用ゼロ。`RecCommandSink::commands()` は `todo!()` で必ずパニック（観測 API が存在しない）、`DrawCommand` の COM 保持バリアントは `#[derive(Clone)]` × `ManuallyDrop` の組み合わせで clone 1 回につき COM 参照を 1 つリーク（AddRef した複製が永久に Release されない）。`dup_com` 自体は SAFETY コメント通り「元オブジェクトが複製より長生き」なら健全だが、不変条件を強制する仕組みがない。非推奨指定なし → R2.9 削除不可 → P33 として記録。テストは現行挙動（記録成功・todo パニック・drop 時の非解放）の特性化に留めた。
2. **DWriteTextLayoutExt のエラー黙殺（→ P34）** — `get_cluster_metrics`/`get_cluster_count` は 1 回目の `GetClusterMetrics` の HRESULT を `let _ =` で破棄し、E_NOT_SUFFICIENT_BUFFER（正常イディオム）と真の失敗を区別しない。失敗時は空成功（`Ok(0)`）へ写像される。修正は戻り値セマンティクス変更のため P34 として記録。
3. **dxgi.rs は空モジュール** — 改行のみの 1 行で実装ゼロ。steering（structure.md:32「dxgi.rs - DXGIインターフェイス」）の記載と乖離。削除（mod 宣言含め2行）は挙動非破壊で W2-S での整理候補として申し送り。
4. **空 IDCompositionAnimation の SetAngle は E_INVALIDARG**・**未描画サーフェイスへの部分更新 begin_draw も E_INVALIDARG** — OS 側の前提条件（セグメント1つ以上／初回は全面描画）であり、ラッパーはエラーをそのまま伝播する。両者をエラー経路テストとして特性化した（`set_angle_animation_with_empty_animation_fails` / `surface_begin_draw_with_update_rect_succeeds_after_full_draw`）。
5. **ulw::transfer_to_hbitmap の pitch < stride 不変条件** — 行単位コピー分岐は `pitch != stride` のみを判定し、理論上 `pitch < stride` なら src の行間を越えて読み出す。D2D の Map 契約上 pitch >= width×4 が保証されるため実害はないが、`debug_assert!(pitch >= stride)` の付加は W2-V（脆弱性点検）での対応候補として申し送り。
6. **実 HWND 依存 API はヘッドレス検証不能** — `dcomp::create_target_for_hwnd`/`set_root`（DComp ターゲット結線）と `ulw::present_layered_window`（WS_EX_LAYERED ウィンドウ + MemoryDC）は実ウィンドウ必須。コード解析の結果、いずれも引数素通しの薄いラッパー＋エラー伝播のみでロジックを持たず（present_layered_window の BLENDFUNCTION/座標系選択は doc コメントに設計根拠記載済み）、リスク残量は小と判断（R2.8）。
7. **wic.rs の `pguidvendor: Option<*const GUID>` は safe API に生ポインタが露出** — `Option<&GUID>` 化が可能（呼び出し箇所2件のみ）。シグネチャ変更は API 変更のため W2-S での検討候補として申し送り。
8. **RED フェーズ代替の検証** — 追加テストは既存挙動の特性化のため RED は N/A。期待値はラッパー実装と Win32 API 契約の読解から導出して記述し、実行で全件一致を確認した。導出が誤っていた2件（所見4の OS 前提条件）は初回実行で fail → OS 仕様を確認のうえエラー経路の特性化テストへ転換した（W1-T と同パターン）。

### 検証（S2）

- BEFORE: HEAD 93c7b92（クリーンツリー）で `cargo build --workspace` + `cargo test --workspace` 成功（exit 0、親指示のベースライン 1219 passed / 0 failed と一致）
- AFTER: `cargo build --workspace` 成功 / `cargo test --workspace` 1298 passed / 0 failed（+79 はすべて追加分。既存テストの変更・削除なし）
- 変更はテストファイル9件の新規追加と report 2ファイルのみ。プロダクションコードの変更なし＝外部観測可能な挙動の変更なし（R5.1 充足）

## flaky

- （AFTER 実行結果を反映: 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue` を含め失敗なし）

## proposals

- P33: d2d 描画コマンド録画モジュール（RecCommandSink / DrawCommand）の完成または削除（利用ゼロ・todo! パニック・Clone の COM 参照リーク）
- P34: DWriteTextLayoutExt のクラスタ数取得におけるエラー黙殺の解消
- 挙動非破壊で対応可能な整理（dxgi.rs 空モジュール削除・wic.rs の生ポインタシグネチャ改善）は W2-S へ、`transfer_to_hbitmap` の debug_assert 付加は W2-V へ申し送り（所見3/5/7）
