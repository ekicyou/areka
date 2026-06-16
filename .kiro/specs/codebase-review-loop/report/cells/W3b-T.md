# W3b-T: wintf グラフィックス資源 × テスト網羅性

- status: completed
- commit: test(W3b): グラフィックス資源・残システムにギャップテスト32件を追加・SetWindowPos キューの観測不能性等を記録

## findings

### ファイル担当の確定（W3a 断片の確定表の補集合）

W3a-T 断片の担当ファイル確定表に従い、W3b の担当を以下の通り確定した。

- **W3b（本セル）**: `visual.rs`、`visual_manager.rs`、`clip.rs`、`core.rs`、`dcomp_resource.rs`、`command_list.rs`、`mod.rs`、`tests.rs`（in-source テスト）、`systems/{brushes,visual_sync,window_pos}.rs`、`systems/mod.rs`
- **W3a（不変更）**: `compositor.rs`、`compositor_systems/`、`systems/{init,render,surface,clip_sync}.rs`、`components.rs` — 本セルでは一切変更していない（フィクスチャとして `WindowD3D11Compositor` を読み取り使用のみ）

### モジュール×テスト対応表（改善前 → 改善後）

| モジュール | 対象 | 既存テスト | 追加 | 備考 |
|------------|------|-----------|------|------|
| `clip.rs` (188 LOC) | `ClipShape` 3 バリアント・負値クランプ・Clone/Debug/PartialEq | 13件（in-source） | 0件 | 純ロジックは既存 in-source テストで網羅済み。重複追加せず |
| `visual.rs` (162 LOC) | `Visual` セッター / `find_owner_window_composition_mode` / `on_visual_add`（Arrangement 連鎖・DComp 条件分岐・BrushInherit） | 13件（component_test）+ 6件（find_owner_composition_mode_test）+ 3件（dcomp_integration_test の on_add 条件分岐） | 0件 | セッター・祖先探索・DComp/ULW/orphan の on_add 条件分岐すべて既存テストで固定済み |
| `visual_manager.rs` (141 LOC) | `insert_visual`/`insert_visual_with` / `visual_resource_management_system` / `window_visual_integration_system` | 5件（insert_visual_test）+ 5件（graphics_auto_creation_test: 正常系・再作成抑止・GraphicsCore 無効） | 2件 | `tests/visual/resource_management_gap_test.rs`。DCompGraphicsResource **不在**・**空**の早期リターン 2 経路を追加固定。`window_visual_integration_system` は実 HWND 必須（所見1） |
| `core.rs` (139 LOC) | `GraphicsCore`（new/invalidate/is_valid/全6アクセサ）+ `FrameTime` | 約24件（core_test 6 + core_ecs_test 10 + reinit_unit_test 2 + in-source 6）+ FrameTime 3件 | 2件 | `tests/graphics/core_accessor_test.rs`。全 6 アクセサの Some→None 遷移（デバイスロスト時の安全性）を一括固定 |
| `dcomp_resource.rs` (94 LOC) | `DCompGraphicsResource`（new/new_empty/invalidate/アクセサ/Debug） | 6件（dcomp_resource_test） | 1件 | 手書き `Debug` 実装（is_valid 報告）のみ未固定だったため core_accessor_test.rs に追加 |
| `command_list.rs` (31 LOC) | `GraphicsCommandList`（new/empty/アクセサ/Clone/PartialEq/Debug/Send+Sync） | 0件（他テストのフィクスチャ使用のみ） | 5件 | `tests/graphics/command_list_test.rs`。PartialEq は COM ポインタ同一性比較であることをテストで明文化 |
| `systems/brushes.rs` (109 LOC) | `resolve_inherited_brushes`（継承解決の全分岐） | 0件（Brush/Brushes 型自体の in-source 13件のみ。システム本体は未検証） | 10件 | `tests/graphics/brushes_system_test.rs`。**完全デバイス非依存の純粋 ECS ロジック**。デフォルト適用・親複製・フィールド別解決・部分解決親スキップ（所見3）・マーカー除去・複数一括解決を固定 |
| `systems/visual_sync.rs` (268 LOC) | `visual_hierarchy_sync_system` / `visual_property_sync_system` | 7件（hierarchy_sync_test 4 + child_order_test 3。**property_sync は 0件**） | 5件 | `tests/visual/property_sync_test.rs`。offset+opacity 設定・無効 VisualGraphics スキップ・Window の offset スキップ・is_visible=false の opacity 0・範囲外クランプの各分岐を実デバイス Visual で完走固定（読み戻し不能 — 所見2） |
| `systems/window_pos.rs` (141 LOC) | `apply_window_pos_changes` / `invalidate_dependent_components` | 1件（dcomp_integration_test: DCompGraphicsResource 無効化のみ） | 7件 | `tests/graphics/window_pos_systems_test.rs`。invalidate の compositor/bitmap_source 無効化ループ・GraphicsCore 有効 no-op・リソース不在 no-op の 4 件 + apply_window_pos の CW_USEDEFAULT スキップ・無効 HWND フォールバック・非 Window 除外の characterization 3 件（所見4） |
| `systems/mod.rs` / `mod.rs` / `tests.rs` | 宣言・再エクスポートのみ | — | 0件 | テスト対象ロジックなし |

追加テスト合計 **32 件**（2+2+1+5+10+5+7）。新規テストファイル 6 件 + 束ね役 `tests/graphics.rs` / `tests/visual.rs` への mod 追記のみ。プロダクションコードの変更なし（R5.1 充足）。

### テスト不能箇所・深掘り所見（R2.8）

1. **window_visual_integration_system は実 HWND 必須でヘッドレステスト不能** — クエリが `&WindowGraphics` を要求するが、`WindowGraphics::new` は `IDCompositionTarget`（`CreateTargetForHwnd` = 実ウィンドウ必須）を引数に取り `Default` も無いため、ヘッドレスではエンティティをクエリにマッチさせること自体が不可能（W3a 所見4 と同根）。コード解析: 本体は `get_target()` と `visual()` の二重 Some ガード → `SetRoot` の 1 呼び出しのみで分岐ロジックを持たず、リスク残量は小。`SetRoot` 失敗は `let _ =` で黙殺されるが、DComp の SetRoot が失敗するのはデバイスロスト時のみで、その場合は別系統（invalidate）が復旧を担う。
2. **visual_property_sync_system の適用結果は読み戻し不能** — `IDCompositionVisual3::SetOffsetX/SetOffsetY/SetOpacity` はいずれも setter のみの write-only COM API で、適用値の検証 API が存在しない。DPI スケール換算（`arrangement.offset × global_arrangement.scale`）の数値検証は不可能なため、全 5 分岐の完走 characterization に留めた。スケール計算自体は `GlobalArrangement::scale_x/scale_y` 側（layout ドメイン）の単体テストで担保される。なお Y オフセットは `scale_y` を正しく使っており、W3a 所見5 の clip_sync（Y 半径に X スケール使用）のような軸取り違えはない。
3. **resolve_inherited_brushes の部分解決親スキップ仕様（→ P44）** — `find_parent_brushes` は「両フィールドとも非 Inherit」の祖先のみを継承元として返すため、片フィールドだけ解決済みの親を持つ子は、親の解決済みフィールドすら継承せずデフォルト（黒/透明）に落ちる。`partially_resolved_parent_without_resolved_ancestor_yields_defaults` で現行挙動を固定したが、「親が前景だけ赤に設定 → 子の前景は赤でなく黒」は直感に反し得る。フィールド単位の継承解決への変更は挙動変更のため P44 として記録。
4. **apply_window_pos_changes の出力は観測 API なし（→ P43）** — 本システムの唯一の出力は `SetWindowPosCommand::enqueue`（thread-local `RefCell<Vec<_>>` キューへの push）だが、キューを覗き見る公開 API が存在せず、取り出し手段は実 `SetWindowPos` を呼ぶ `flush()` のみ。このためテストでは「enqueue された座標・フラグ（`SWP_NOMOVE` 強制を含む）」を検証できず、CW_USEDEFAULT スキップ・座標変換フォールバックの 2 分岐は完走 characterization に留めた。ドラッグ中 `SWP_NOMOVE` 経路はフラグ観測不能のためテスト自体を見送り。テスト用 peek API の追加はプロダクション変更のため P43 として記録。なお無効 HWND テストは enqueue まで行うが flush しないため実 Win32 呼び出しは発生しない（TLS のためテストスレッド終了で破棄、他テストへの汚染なし）。
5. **GraphicsCore::new はハードウェアデバイス直結（フォールバックなし）** — `create_device_3d` は `D3D_DRIVER_TYPE_HARDWARE` 固定で WARP フォールバックが無く、GPU の無い CI 環境では本セル追加分を含むデバイス系テスト全件が作成失敗する。現行の開発環境（実 GPU）では問題ないが、テストインフラの可搬性制約として記録（既存の W2-T 前例踏襲のため新規提案はしない）。
6. **visual_hierarchy_sync_system の全削除→再追加方式** — 未同期の子を 1 つでも持つ親は `remove_all_visuals()` で全子を削除して Children 順に再追加する。兄弟が多い場合 O(N) の COM 呼び出しになるが、Z-order の権威的ソースを Children に一本化する設計意図が明確で、既存テスト（child_order_test 3 件）が固定済みのため改善提案は不要と判断。
7. **RED フェーズ代替の検証** — 追加テストは既存挙動の characterization のため RED は N/A。期待値（ブラシ解決規則・アクセサ Some/None 遷移・無効化伝播・COM ポインタ等価性）は実装と独立にソース仕様から導出して記述し、初回実行で 32 件全件が導出どおり一致した。所見3 の「デフォルト落ち」挙動も導出段階で予見し、テストが現行実装を正確に固定していることを相互確認した。

### 検証（S2）

- BEFORE: 親指示に従いベースライン（クリーン HEAD b1a5bc8 で 1357 passed / 0 failed）を信頼。作業ツリーには並行セル由来の dola/ 配下の未コミット変更が存在するが、本セルの境界外であり触れていない
- AFTER: `cargo build --workspace` 成功（warning/error 0）/ `cargo test --workspace` **1389 passed / 0 failed**（1357 + 32 = 1389 で追加分と完全一致。既存テストの変更・削除なし）
- 変更はテストファイル 6 件の新規追加 + 束ね役 2 ファイルの mod 追記 + report 2 ファイルのみ。プロダクションコードの変更なし＝外部観測可能な挙動の変更なし（R5.1 充足）

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue` を含め、AFTER のフル実行 2 回（いずれも 1389/0）で失敗なし。隔離再実行は不要だった

## proposals

- P43: SetWindowPosCommand キューのテスト観測 API 追加（apply_window_pos_changes の出力検証を可能にする）
- P44: resolve_inherited_brushes のフィールド単位継承解決（部分解決親のデフォルト落ち解消）
- 所見5（GraphicsCore の WARP フォールバック不在による CI 可搬性制約）は既存前例踏襲のため提案化せず記録のみ
