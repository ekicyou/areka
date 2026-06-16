# W4a-T: wintf taffy・配置 × テスト網羅性

- status: completed
- commit: test(W4a): taffy・配置・寸法ドメインにギャップテスト32件を追加・パーセント正規化欠落等を記録

## findings

### ファイル担当の確定（W4a / W4b の境界）

`crates/wintf/src/ecs/layout/` のファイル一覧と `mod.rs` を確認し、担当を以下の通り確定した。

- **W4a（本セル）**: `taffy.rs`、`arrangement.rs`、`box_style.rs`、`dimension.rs`、`systems/taffy_systems.rs`、`systems/arrangement_systems.rs`、`mod.rs`（`LayoutRoot` とその on_add フック — taffy/配置ドメインの中核コンポーネントのため本セル担当）、`systems/mod.rs`（宣言のみ・変更なし）
- **W4b（不変更）**: `hit_test/`、`hit_region/`、`metrics.rs`、`rect.rs`、`systems/monitor_systems.rs`、`systems/window_pos_systems.rs` — 本セルでは一切変更していない（テストからは `Offset`/`Size`/`LayoutScale`（metrics.rs）と `D2DRect`（rect.rs）を読み取り使用のみ）

### モジュール×テスト対応表（改善前 → 改善後）

| モジュール | 対象 | 既存テスト | 追加 | 備考 |
|------------|------|-----------|------|------|
| `taffy.rs` (118 LOC) | `TaffyStyle`/`TaffyComputedLayout`/`TaffyLayoutResource`（create/remove/双方向マッピング/整合性検証） | taffy_layout_integration_test 約23件 + taffy_advanced_test のマッピング系 7件 | 0件 | ノード作成・削除・双方向マッピング・`verify_mapping_consistency` まで網羅済み。重複追加せず |
| `arrangement.rs` (234 LOC) | `Arrangement`/`GlobalArrangement`/`ArrangementTreeChanged` + From/Mul 変換 + on_add フック | arrangement_bounds_test 21件 + hierarchical_bounds_test 6件（local_bounds・From<Arrangement>・Mul・スケール階層） | 10件 | アクセサー9種（scale_x/y/scale/offset_x/y/offset/width/height/size）・Default（恒等変換）・`From<Offset>`/`From<LayoutScale>`/`From<Arrangement>` for Matrix3x2 の直接検証・on_add フック（GlobalArrangement/ArrangementTreeChanged 自動挿入）が未カバーだった |
| `box_style.rs` (256 LOC) | `BoxStyle` と `From<&BoxStyle> for taffy::Style` の全分岐 | box_style_consolidation_test 13件 + component_conversion_test 7件 | 7件 | min_size/max_size（全指定・片側 None）・size/flex_basis の Percent 正規化・position: Relative 明示・Flex アイテム系のみで display 自動設定なし・`BoxStyle::new()` が未カバーだった |
| `dimension.rs` (293 LOC) | `Dimension`/`LengthPercentageAuto`/`LengthPercentage`/`Rect<T>` の taffy 変換・定数 | taffy_layout_integration_test の const コンストラクター1件のみ（Percent 変換は全テスト未検証だった） | 13件 | 新規 `tests/layout/dimension_conversion_test.rs`。`Dimension::Percent` の ÷100 正規化、LPA/LP の **÷100 欠落の特性化**（所見1 → P49）、`From<taffy::Dimension>` スタブの特性化（所見2 → P50）、TaffyZero/TaffyAuto 定数、`Rect::zero/auto/default` と taffy 変換を固定 |
| `mod.rs`（`LayoutRoot`, 138 LOC） | on_add フック（Arrangement 自動挿入・既存保持） | feedback_loop_convergence_test 等で間接使用のみ（フック仕様の直接検証なし） | 2件 | 新規 `tests/layout/component_hooks_test.rs`。Arrangement 連鎖挿入と既存 Arrangement の非上書きを直接固定 |
| `systems/taffy_systems.rs` (324 LOC) | `build_taffy_styles_system`/`sync_taffy_tree_system`/`compute_taffy_layout_system`/`update_arrangements_system`/`cleanup_removed_entities_system` | component_conversion_test 7件 + taffy_advanced_test 約15件 + taffy_child_order_test 4件 + boxstyle_coordinate_separation_test の update_arrangements 3件 | 2件 | build_taffy_styles の「LayoutRoot のみ（BoxStyle なし）→ デフォルト TaffyStyle + TaffyComputedLayout + ArrangementTreeChanged 挿入」分岐と「Changed<BoxStyle> → 既存 TaffyStyle 更新」分岐が未カバーだった。compute の available_space 分岐は所見3参照 |
| `systems/arrangement_systems.rs` (94 LOC) | `sync_simple_arrangements`/`mark_dirty_arrangement_trees`/`propagate_global_arrangements` | feedback_loop_convergence_test 8件（3システム連結で DPI 96/192・スキップ系を網羅）+ graphics_sync_test | 0件 | 中身は common/tree_system のジェネリック関数への委譲ラッパー。連結シナリオで十分固定済みのため重複追加せず |
| `systems/mod.rs` | 宣言・再エクスポートのみ | — | 0件 | テスト対象ロジックなし |

追加テスト合計 **32件**（10+7+13+2+2... 内訳: dimension_conversion 13・component_hooks 3・arrangement_bounds 追記 7・box_style_consolidation 追記 7・component_conversion 追記 2）。新規テストファイル 2件 + 既存 3 ファイルへの追記 + 束ね役 `tests/layout.rs` への mod 2 行追記のみ。プロダクションコードの変更なし（R5.1 充足）。

### テスト不能箇所・深掘り所見（R2.8）

1. **LengthPercentageAuto / LengthPercentage の Percent 変換に ÷100 正規化がない（→ P49）** — `Dimension::Percent(v)` は `taffy::percent(v/100)` に正規化するのに、`LengthPercentageAuto::Percent(v)`（dimension.rs:172）と `LengthPercentage::Percent(v)`（dimension.rs:218）は値をそのまま渡す。3 型ともドキュメントは「0.0～100.0 で指定・変換は自動」と謳うため、margin/padding/inset に `Percent(50.0)` を指定すると taffy 解釈で 5000% になる潜在バグ。リポジトリ内の利用は Px のみで未発現。修正は挙動変更のため特性化テスト 2 件で現状を固定し P49 に記録。
2. **`From<taffy::Dimension> for Dimension` は常に Auto を返すスタブ（→ P50）** — ソース内 TODO 明記。プロダクションからの呼び出しは grep で 0 件（dead-ish な公開 trait 実装）。`test_dimension_from_taffy_is_stub_returning_auto` で特性化し P50 に記録（正確な逆変換の実装 or 削除）。
3. **compute_taffy_layout_system の available_space 分岐は単体では観測困難** — ルート BoxStyle の size が Px なら `AvailableSpace::Definite`、Percent/Auto/なしなら `MaxContent` を構築する分岐（taffy_systems.rs:139-167）は、結果が `compute_layout` 内部に吸収され、固定 Px サイズの典型シナリオ（taffy_advanced_test・taffy_flex_layout_pure_test で網羅）では出力差が現れにくい。Percent ルートの end-to-end は taffy_flex_layout_pure_test が `Percent(100.0)` ルートで実質カバー済みのため、重複となる追加は見送った。
4. **update_arrangements_system の DPI スケール適用は既存テストで十分** — Window + DPI → scale 設定・非 Window → (1.0,1.0)・Window offset 維持の 3 分岐は boxstyle_coordinate_separation_test（3件）と feedback_loop_convergence_test（DPI 96/192）で固定済み。Window かつ DPI コンポーネントなしのフォールバック分岐のみ未カバーだが、実アプリでは Window 生成時に DPI が必ず付与される構成のため優先度低と判断し見送り（過剰な網羅より過不足整理を優先）。
5. **taffy::Style のジェネリック化（taffy 0.9）** — `taffy::Style::default()` は型パラメータ `S: CheapCloneStr` の推論が単独では効かず `let s: taffy::Style = Default::default()` 形式の注釈が必要。テスト記述時の注意点として記録（既存テストは `::taffy::Style::default()` を比較対象側の型推論が効く文脈でのみ使用していた）。
6. **RED フェーズ代替の検証** — 追加テストは既存挙動の characterization のため RED は N/A。期待値（÷100 正規化の有無・行列合成順序 M31=offset×scale・フック連鎖挿入・display 自動設定の発火条件）はすべて実装と独立にソース仕様・taffy 0.9 のセマンティクスから導出して記述し、初回実行で 32 件全件が導出どおり一致した。所見1 の正規化欠落も導出段階で予見し、特性化テストが現行実装を正確に固定していることを相互確認した。

## flaky

- `cue_performance_test::bench_pop_ready_empty_queue`（既知・負荷依存）: 本セルの全ワークスペース実行では初回から pass。再実行不要だった。

## verification (S2)

- BEFORE: 親のベースライン（clean HEAD 0a112ff で 1392 passed / 0 failed）を信頼して流用。
- AFTER: `cargo build --workspace` 成功、`cargo test --workspace` **1424 passed / 0 failed**（+32 = 追加テスト数と一致、削除なし）。`--test layout` 単体は 157 passed / 0 failed。

## proposals

- P49: LengthPercentageAuto / LengthPercentage の taffy 変換でパーセント正規化（÷100）が欠落（挙動変更を伴う脆弱性対策）
- P50: From<taffy::Dimension> for Dimension のスタブ実装の実装または削除（その他）
