# W3b-S: wintf グラフィックス資源 × シンプル化（unsafe 保守則適用）

- status: completed
- commit: refactor(W3b): on_visual_add の commands 三重借用を単一ブロック化・未使用パラメータ/デバッグ残骸を整理（net -36行）

## findings

S6 基準（karpathy-guidelines）で境界 9 ファイル（visual.rs / visual_manager.rs / clip.rs / core.rs / dcomp_resource.rs / command_list.rs / systems/{brushes,visual_sync,window_pos}.rs）を点検し、挙動非破壊の構造的整理 4 件を適用した。clip.rs / core.rs / dcomp_resource.rs / command_list.rs / brushes.rs の 5 ファイルは点検の結果、簡素化の余地なしと判断して無変更（brushes.rs の重複気味な `resolve_brush_fields` は P44 が同関数の再構成を提案済みのため、衝突回避の観点からも不実施）。

### 適用 1: `on_visual_add` の commands 三重借用の単一ブロック化（visual.rs）

`world.commands()` を `cmds` / `cmds2` / `cmds3` と 3 回借用し直す構造（DComp 判定の world 読み取りが command 借用と交互に挟まるため）を、「全チェック（world 読み取り）を先に完了 → commands を一度だけ借用して全 insert を発行」に再構成した（-10 行）。

- deferred command のため読み取り順序の前倒しは等価（command は flush まで world に反映されない）。insert 発行順序（Arrangement → VisualGraphics → SurfaceGraphics → SurfaceGraphicsDirty → BrushInherit）は完全に維持
- 保護テスト: `tests/graphics/dcomp_integration_test.rs::test_dcomp_window_visual_gets_dcomp_components` / `test_ulw_window_visual_does_not_get_dcomp_components` が DComp/ULW 両分岐の挿入結果を直接アサート。Arrangement 自動挿入は `tests/layout/graphics_sync_test.rs` ほか多数が依存（R5.5 のテスト保護下に該当）

### 適用 2: `create_visual_only` の未使用 `_commands` パラメータ削除（visual_manager.rs）

Phase 6 リファクタリング（Surface 作成の Draw スケジュール遅延移行）の名残である私有ヘルパーの `_commands: &mut Commands`（下線付きで未使用が自認されていた）をシグネチャから削除した。これにより呼び出し元 `visual_resource_management_system` の `Commands` も完全未使用となるが、**システムパラメータの削除はスケジューラから見たアクセスセットの変更（外部観測可能な性質）にあたるため P39 の前例判断に従い不実施**とし、`_commands` への改名 + NOTE コメント（P45 参照）に留めた。システムパラメータ削除は P45 として記録。

- 保護: grep で `create_visual_only` の呼び出しは visual_manager.rs 内 1 箇所のみを確認。`tests/visual/component_test.rs` / `graphics_auto_creation_test.rs` / `resource_management_gap_test.rs` がシステム動作を保護

### 適用 3: 読み取り専用クエリの誤った可変性除去・陳腐化コメント削除（window_pos.rs）

`apply_window_pos_changes` のクエリは全項目読み取り専用（`&WindowHandle`, `&WindowPos`, `Option<&Name>`, `Has<WindowDragging>`)にもかかわらず `mut query` + `iter_mut()` で回していたため、`query` + `iter()` に修正した（アクセスセットはクエリ型で決まるため変化なし）。あわせて、実体（position/size の取り出し）と無関係な陳腐化コメント「エコーバックチェック」1 行を削除した（echo 抑制の実機構は関数 doc コメントに記載済みの `bypass_change_detection` / `Changed` 不発火）。

- 保護テスト: `tests/graphics/window_pos_systems_test.rs`（W3b-T 追加の characterization: CW_USEDEFAULT スキップ・座標変換フォールバック）

### 適用 4: p1() 二重借用の統合・コメントアウト済みデバッグ残骸の削除（visual_sync.rs）

R5.5（write-only COM setter 経路は構造的整理に限定）の範囲内で 2 点を整理した。

1. `visual_hierarchy_sync_system`: 親の Visual と名前（ログ用）を取得するために同一エンティティへ `vg_queries.p1().get(parent_entity)` を 2 回実行していた重複借用ブロックを、1 回の get で `(parent_visual, parent_name)` を同時取得する形に統合（-9 行）。COM 呼び出し（remove_all_visuals / add_visual）の順序・条件は不変。保護テスト: `tests/visual/hierarchy_sync_test.rs` / `child_order_test.rs`
2. `visual_property_sync_system`: コメントアウト済みデバッグログ残骸 2 ブロック（info! 比較ログ・eprintln）と、それのみを内容とする空 else 分岐を削除（-17 行）。Window スキップの理由説明は if 直上の既存コメントが引き続き担う。実行コードの変更なし。保護テスト: `tests/visual/property_sync_test.rs`（W3b-T 追加）

## verification (S2)

- BEFORE: 親ベースライン信頼（HEAD 2e1d6f8 clean、1389 passed / 0 failed）
- AFTER: `cargo build --workspace` 成功（警告 0）・`cargo test --workspace` **1389 passed / 0 failed**（ベースライン完全一致）・`cargo build --examples -p wintf` 成功
- 差分: 4 ファイル、+28 / -64（net -36 行）

## flaky

- cue_performance bench を含め flaky な失敗は観測されず（全テスト一発 green、isolate-rerun 不要）

## proposals

- **P45**（新規記録）: `visual_resource_management_system` の未使用 Commands システムパラメータ削除（P39 と同系統のアクセスセット変更のため本ループ不実施）
- **P46**（新規記録）: `apply_window_pos_changes` の重複 debug ログ統合（ログ出力は観測可能挙動のため本ループ不実施。P43 と同一ファイルで統合実施可）
- P43 / P44 は W3b-T 記録済みのため再記録せず（brushes.rs の継承解決規則・TLS キュー観測 API はいずれも該当箇所を確認したが現状維持）
