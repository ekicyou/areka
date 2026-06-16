# W4a-S: wintf taffy・配置 × シンプル化

- status: completed
- commit: （親が確定）

## 観点・基準・範囲

- 観点: S（シンプル化）。基準: S6 = `karpathy-guidelines`（Simplicity First / Surgical Changes / churn 回避）。
- 要件: R2.2（シンプル化の実行）・R2.5（前後の S2 非破壊確認）・R2.7（テスト→シンプル化→脆弱性の列順。本セルは W4a-T で整備済みの回帰検知器の上で実行）・R2.8（テスト保護外でも解析し、ロジック変更を要する候補は提案記録）・R4.1（自己レビュー＋検証後にコミット）・R5.1（外部観測可能な挙動を変更しない）・R5.3（新機能・意図的挙動変更・大規模再設計を行わない）。unsafe の構造的整理限定は R5.5、不使用コード削除は R2.9 / R2.10 と Boundary Context の「publish=false ゆえ利用ゼロ実証で後方互換を考慮せず削除可」前提に拠る。
- 領域: W4a = `crates/wintf/src/ecs/layout/` のうち taffy/arrangement/box_style/dimension 系（design 領域表 W4a, 約1,290 LOC）。担当ファイルは W4a-T で確定済み（`taffy.rs`/`arrangement.rs`/`box_style.rs`/`dimension.rs`/`systems/taffy_systems.rs`/`systems/arrangement_systems.rs`/`mod.rs`(`LayoutRoot`)/`systems/mod.rs`）。`hit_region/`・`window_pos_systems.rs`・`metrics.rs`・`rect.rs` は W4b のため対象外（本セルでは未変更）。

## 起点（前セッションの残存差分）の妥当性検証

前セッション中断時にワークツリーへ残っていた W4a-S 差分 6 ファイル（box_style/dimension/mod/taffy/taffy_systems + tests/dimension_conversion）を `git diff` で現物確認し、各変更が挙動非破壊であることを 1 件ずつ検証した。

1. **box_style.rs — `apply_box_size` ヘルパー統合**: `From<&BoxStyle> for taffy::Style` 内の size / min_size / max_size の 3 箇所に重複していた「width あれば `.into()` 代入 / height あれば `.into()` 代入」を 1 関数へ抽出。元の各ブロックは `if let Some(w)=size.width { taffy_style.size.width = w.into(); }` … という構造で、未指定軸には何も代入しない（= taffy デフォルト維持）。新ヘルパー `apply_box_size(target,&src)` は同じく `if let Some(w)=src.width { target.width = w.into(); }`・height 同様で、3 呼び出しが元の 3 ブロックと **トークン等価**。未指定軸が taffy::Style::default 値を保つ挙動も不変。box_style_consolidation_test（min/max 片側 None・Percent 正規化・全指定）と component_conversion_test が全通過で裏付け。

2. **dimension.rs — P50 スタブ削除**: `From<taffy::Dimension> for Dimension`（入力に関わらず常に `Dimension::Auto` を返す TODO 付きスタブ）を削除。proposals.md P50 の suggestion (b)「未使用のため trait 実装ごと削除」を適用したもの。**不使用実証は本セルで再実施（下節）**。逆方向の正規変換 `From<Dimension> for taffy::Dimension`（dimension.rs:98）は保持され、Dimension→taffy 変換テスト群は影響を受けない。

3. **tests/layout/dimension_conversion_test.rs — 特性化テスト追随削除**: 上記スタブの現状固定テスト `test_dimension_from_taffy_is_stub_returning_auto` を削除。被テスト対象（スタブ実装）が削除されたことに伴う機械的追随であり、テスト弱体化ではない（固定対象がソースから消滅したため当該特性化テストは存在意義を失う）。R5.1 の趣旨に整合。

4. **taffy_systems.rs — 未使用クエリ項削減 + available_space クロージャ化**:
   - `sync_taffy_tree_system` の `new_entities` を `Query<(Entity, Option<&ChildOf>), Added<TaffyStyle>>` → `Query<Entity, Added<TaffyStyle>>` に縮約。元ループ `for (entity, _) in ...` は第2要素 `Option<&ChildOf>` を `_` で捨てており完全な死値。挙動同一。
   - `changed_hierarchy` を `Query<(Entity, Option<&ChildOf>), Changed<ChildOf>>` → `Query<&ChildOf, Changed<ChildOf>>` に縮約。元ループは `if let Some(parent_ref)=child_of { affected_parents.insert(parent_ref.parent()); }`。`Changed<ChildOf>` フィルタは ChildOf を必ず持つエンティティのみを返すため、元の `Option<&ChildOf>` は常に `Some`。新形では `&ChildOf` を直接フェッチし `affected_parents.insert(child_of.parent())`。`Some` 分岐のみが実行されていた事実から挙動等価。`Entity` も未使用だったため除去。taffy_child_order_test（兄弟順序）・taffy_advanced_test が裏付け。
   - `compute_taffy_layout_system` の available_space 構築を 3 重ネスト `if let` から クロージャ `to_available(Option<Dimension>)` + `and_then` 連鎖へ簡素化。元の真理値表は「box_style なし→MaxContent / size なし→MaxContent / 軸 None→MaxContent / 軸=Px→Definite(px) / 軸=Percent|Auto→MaxContent」。新形は `to_available = |d| match d { Some(Dimension::Px(px))=>Definite(px), _=>MaxContent }` と `root_size = box_style.and_then(|s| s.size)`・`root_size.and_then(|s| s.width|height)` の合成で、各 None 段（box_style なし／size なし／軸 None）が `None`→MaxContent に畳まれ、`Some(Px)`→Definite、`Some(Percent|Auto)`→MaxContent。**元の 5 ケースを厳密保存**。`BoxStyle`/`BoxSize`/`Dimension` はいずれも `Copy` のため `and_then` での値取り出しは借用問題なし（ビルド成功で実証）。Px ルート（taffy_advanced_test 等）・Percent ルート（taffy_flex_layout_pure_test の `Percent(100.0)` ルート）で end-to-end カバー済み。

5. **taffy.rs — `unsafe impl Send/Sync for TaffyStyle` に SAFETY 注記追加**: コメントのみの追加。taffy 0.9 の Style が CompactLength（calc 式用の生ポインタ）を含むため自動 Send/Sync にならない事情と、wintf が calc を使わず（Px/Percent/Auto のみ）Style を ECS のアクセス制御下でのみ扱う安全根拠を明記。R5.5（テスト保護外 unsafe はコメント等の構造的整理に限定）に合致。コード挙動は不変。

6. **mod.rs — doc コメント迷い込み行 1 行削除**: `LayoutRoot` の rustdoc 内に紛れていた孤立行 `/// LayoutRootコンポーネント` を除去。コメントのみ。残る rustdoc は整合（doc-test `ecs::layout::LayoutRoot` は通過）。

残存差分は `git diff --stat` 上すべて W4a 境界内（src/ecs/layout 配下 + tests/layout の当該ドメイン）に収まることを確認。境界外への漏れなし。

### 厳守事項の遵守

- **P49（LengthPercentageAuto/LengthPercentage の Percent ÷100 欠落, dimension.rs:161/207）**: 挙動変更（レイアウト結果が変わる）のため **一切触れていない**。proposals.md P49 に記録済みのまま据え置き。
- 適用した簡素化はすべて挙動非破壊。ロジック変更・挙動変更を要する候補は適用せず（下記「proposals へ回した候補」参照）。

## P50 不使用実証（grep + build 再実証）

proposals.md P50 の削除適用に伴い、`From<taffy::Dimension> for Dimension`（逆変換）のプロダクション呼び出し元がゼロであることを `git diff` 後に再実証した。

- `From<taffy::Dimension>` の出現: spec/report 文書内のみ（tasks.md / proposals.md / W4a-T.md）。**ソースコードでのヒットは削除された定義跡のみ**でプロダクション利用なし。
- `Dimension::from`: **ワークスペース全体で 0 件**。
- `taffy::Dimension` の全出現を `crates/` で精査: すべて (a) 順方向 `From<Dimension> for taffy::Dimension`（dimension.rs:98、**保持**）の利用、(b) `Dimension → taffy::Dimension` を検証するテストの `let x: taffy::Dimension = Dimension::_.into()` アサーション、(c) 新 `apply_box_size` の型引数 — のいずれか。**逆方向（`let d: Dimension = <taffy 値>.into()`）の呼び出しは 0 件**で、削除された特性化テストが当該逆変換の唯一の消費者だった。
- ビルド実証: 削除後に `cargo build --workspace` 成功（後述 AFTER S2）。逆変換 impl を参照するコードが残存すれば未解決シンボルでコンパイル失敗するため、ビルド成功が不使用を二重に裏付ける。

結論: P50 の削除は挙動非破壊。R2.9（利用ゼロ実証での不要コード削除）および Boundary Context（publish=false ゆえ後方互換不要）に合致。

## ギャップスイープ（S6 基準・本セルで追加適用）

W4a 境界内全ファイル（taffy.rs / arrangement.rs / box_style.rs / dimension.rs / mod.rs / systems/taffy_systems.rs / systems/arrangement_systems.rs / systems/mod.rs）を S6 視点で再走査し、残存差分が未着手の挙動非破壊簡素化を探索した。適用 1 件・churn 回避で見送り多数。

### 適用（1件）

- **arrangement_systems.rs `mark_dirty_arrangement_trees` の死コード除去**: 元コードは
  ```
  let changed_count = changed.iter().count();
  if changed_count > 0 { /* tracing::info! は全行コメントアウト */ }
  ```
  で、`changed_count` は本体が完全にコメントアウトされた `if` を gate するためだけに計算される死値だった。`changed.iter().count()` は読み取り専用パス（`Changed`/`RemovedComponents` の状態を消費せず、`changed` は直後に `mark_dirty_trees(...)` へ値渡しで委譲される）であり、ブロック除去は委譲先が受け取る入力を変えない。削除し、ログ抑制の意図のみ `Note:` コメントで保存（net −7 行）。これは「自分の変更が生んだ orphan の除去」ではなく死コードの構造的整理だが、本セルの S 観点（R2.2 / design CellExecutor S ステップの「デバッグ残骸・自明な重複除去」）が明示的に授権する範囲であり、同レビューループの先行セル（W3b の「デバッグ残骸除去」）と整合する。挙動非破壊は `--test layout` 156/156（配置伝播テスト群を含む）と AFTER S2 全量で実証。

### churn 回避で見送り（karpathy「壊れていないものを refactor しない」）

- **arrangement.rs `Mul<Arrangement> for GlobalArrangement` の `result_transform.M11/M22` 再導出**: 数学的に `parent_scale × child_scale` と一致するが、座標変換（Visual の `SetOffsetX(offset*scale)` との整合）の根拠コメントが付いた稼働コード。畳み込みは挙動リスク + churn のため見送り（arrangement_bounds_test / hierarchical_bounds_test が現挙動を固定）。
- **clippy `this impl can be derived`（後述）**: 手書き `Default` は `Dimension`/`LengthPercentageAuto`/`Arrangement` で意図的・一貫したハウススタイル（`const AUTO`/`const ZERO` と併存）。`#[derive(Default)]`+`#[default]` 化は型定義への非自明な構造変更で、本残存差分・本タスクの目的と無関係な既存 lint。R5.1 と karpathy「既存スタイルに合わせる」に従い見送り。

## proposals.md へ回した候補

- なし（P51 以降の新規記録なし）。本セルのギャップスイープで発見した未適用候補は、いずれも「ロジック変更を要する簡素化」ではなく「稼働中の正しいコードを churn しないために見送った構造」であり、提案様式（kind: ロジック変更を要する簡素化／その他）に値する新規ロジック改変候補は検出されなかった。既知の挙動変更候補は P49（据え置き）・P50（削除適用済み）で網羅済み。

## verification (S2)

- BEFORE: 親検証済みベースライン（W4a-T 後 = 1424 passed / 0 failed）を信頼し省略。
- AFTER（必須・全量実施）:
  - `cargo build --workspace` → **成功**（exit 0、wintf/areka 再コンパイル）。
  - `cargo test --workspace` → **1423 passed / 0 failed**（FAILED 結果行 0）。
  - 件数照合: ベースライン 1424 − 1（P50 特性化テスト `test_dimension_from_taffy_is_stub_returning_auto` の正当な追随削除）= 1423。**既存テストの回帰ゼロ**、差分は意図した 1 件の削除のみ。R5.1 の「削除対象が消滅した特性化テストの除去はテスト弱体化に当たらない」に整合。
  - 反復用 `cargo test -p wintf --test layout` → 156 passed / 0 failed（前セッション 156 と一致。配置伝播テストを含み、本セルの arrangement_systems 編集の非破壊を直接実証）。

## S3 clippy 所見（記録のみ・非ブロッカー）

`cargo clippy -p wintf`（JSON）で W4a 境界内の primary span を抽出（W4b の hit_region/window_pos は除外）。**いずれも本残存差分・本セル編集に起因しない既存 lint** であり、S3 は記録のみのため未適用。

- `this impl can be derived`: arrangement.rs:29 / dimension.rs:91 / dimension.rs:150（手書き `Default`。上記理由で意図的見送り）。
- `very complex type used`: arrangement_systems.rs:14/34/54・taffy_systems.rs:29/189（bevy のシステム引数型。`type` 別名化は広域な様式 refactor で churn）。
- `this if statement can be collapsed`: taffy_systems.rs:90/107/155/228（edition 2024 の let-chain 化。全箇所が残存差分の触れていない既存コード。サージカル性のため見送り）。

本セル編集（arrangement_systems.rs の死コード除去）が **新規 clippy 警告を一切導入していない**ことを確認（編集箇所 40–56 行に残るのは編集前から存在する signature の `very complex type`(:54) のみで、unused 系の新警告なし）。

## flaky

- `cue_performance_test::bench_pop_ready_empty_queue`（既知・負荷依存、W4a 境界外）: AFTER S2 全量で **初回から `... ok`**。隔離再実行不要だった（フレーキー発火なし）。

## 自己レビュー

- 実装は本物（モック/スタブ/プレースホルダなし）。残存差分の P50 はスタブの「削除」であり、新たなスタブ導入ではない。
- TODO/FIXME 残存なし: 削除された P50 スタブが唯一の TODO 箇所だった。本セル追加・編集分に TODO/FIXME なし。
- テストは意味を持つ: 既存 156（layout）/ 1423（全量）が回帰検知器として機能し、適用した 2 系統の簡素化（残存差分 + 死コード除去）の非破壊を実証。削除した 1 件は被テスト対象消滅に伴う正当な追随。
- 境界遵守: 変更 7 ファイルすべて `crates/wintf/src/ecs/layout/`（+ in-boundary `tests/layout/`）。tasks.md 未更新・コミット未作成・proposals 不要記録なし。
