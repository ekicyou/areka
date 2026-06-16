# W4a-V: wintf taffy・配置 × 脆弱性レビュー

- status: completed
- commit: （親が確定）

## 観点・基準・範囲

- 観点: V（脆弱性レビュー）。基準: design.md「V 手順」（unsafe 境界・整数オーバーフロー・入力検証・リソースリーク・panic 経路 DoS を点検し、**挙動を変えない対策のみ投入**。挙動変更を要する対策は proposals.md へ。Security Considerations 節 / CellExecutor V 規則）。
- 要件: R2.3（脆弱性レビュー＋挙動非破壊対策）・R2.4（挙動変更対策→提案記録）・R2.5（前後 S2 非破壊）・R2.7（テスト→シンプル化→脆弱性の列順。本セルは W4a-T で整備済みの回帰検知器の上で実行）・R2.8（テスト保護外でも深く解析し、安全適用不能な改善は提案記録）・R4.1（自己レビュー＋検証後コミット）・R5.1（外部観測可能な挙動を変更しない）・R5.2（挙動変更必要時は提案記録）。テスト保護外の unsafe はコメント等の構造的整理に限定（R5.5）。
- 領域: W4a = `crates/wintf/src/ecs/layout/` のうち taffy/arrangement/box_style/dimension 系。点検対象ファイル（W4a-T 確定の担当）: `taffy.rs` / `arrangement.rs` / `box_style.rs` / `dimension.rs` / `mod.rs`(`LayoutRoot`) / `systems/taffy_systems.rs` / `systems/arrangement_systems.rs`。`hit_region/` / `hit_test/` / `metrics.rs` / `rect.rs` / `systems/monitor_systems.rs` / `systems/window_pos_systems.rs` は **W4b のため対象外**（読み取りのみ。型理解に使用）。
- 起点: W4a-S 適用後のクリーンなワークツリー（HEAD = `5280973`、ベースライン 1423 passed / 0 failed）に対する点検。

## 点検手法

境界内 7 ファイルを grep（複数パターン）＋全文精読で走査した。

- panic 経路: `unwrap\(\)` / `expect\(` / `panic!` / `unreachable!` / `unimplemented!` / `todo!` / 配列添字 `\[i\]`/`\[idx\]`/`\[\d+\]`
- 数値境界: `as \w+`（キャスト切り捨て）/ `/`（ゼロ除算）/ `checked_`/`saturating_`/`wrapping_`/`overflow` / `assert` / `debug_assert`
- unsafe 境界: `unsafe` / `Send` / `Sync` / `transmute` / `from_raw` / `as_ptr` / `ManuallyDrop`

加えて `TaffyLayoutResource` の実利用箇所を `grep`（`insert_resource` / `ResMut` / `resource_mut`）で確認し、unsafe Send/Sync の安全根拠（ECS 排他アクセス・生ポインタ非漏出）を実証した。

## 発見した脆弱性候補と判定

### 1. panic 経路 — 現状安全（対策不要）

境界内 7 ファイルすべてで `unwrap()` / `expect()` / `panic!` / `unreachable!` / `todo!` / 配列添字 の出現が **ゼロ**（grep 実証）。検出された panic/cast 候補はすべて `hit_region/` と `hit_test/`（**W4b 領域・対象外**）に存在。

- **taffy 連携の Result はすべて握り潰しで処理**: `taffy_systems.rs` の `create_node` / `set_style` / `set_children` / `remove_child` / `remove_node` / `compute_layout` / `layout` は `let _ = ...` / `if let Ok(...)` / `if result.is_ok()` で受けており、`.unwrap()` で panic する経路は皆無。taffy 操作失敗時はサイレントスキップ（既存挙動・W4a-S で `compute_layout` の available_space 構築まで非破壊実証済み）。**DoS panic 経路なし。**
- **`taffy.rs::verify_mapping_consistency` の `assert_eq!`**: `#[cfg(debug_assertions)]` 付きでリリースビルドには存在せず、かつプロダクションからの呼び出しゼロ（W4a-T 実証・テスト専用デバッグ API）。リリース挙動に影響する panic 経路ではない。**対策不要。**
- 判定: **現状安全。** churn 回避のため変更ゼロ。

### 2. 数値境界 — 現状安全（既知の P49 は据え置き）

- **整数オーバーフロー: 該当なし**。境界内に整数算術は存在しない（`as` キャストもゼロ）。レイアウト値はすべて `f32` / `Matrix3x2` の浮動小数演算（`arrangement.rs` の `Mul`/`From<Arrangement>`、`offset * scale`、`size * result_scale`）。f32 演算は NaN/inf を panic せずサイレント伝播するのみで、オーバーフロー panic は発生しない。
- **ゼロ除算: 該当 1 箇所のみ**＝`dimension.rs:104` の `v / 100.0`（`Dimension::Percent` 正規化）。除数はリテラル `100.0` で**ゼロになり得ない**。安全。
- **P49（LengthPercentageAuto/LengthPercentage の Percent ÷100 欠落, dimension.rs:161/207）**: 点検で再発見したが、**既知の挙動変更案件のため一切触れていない**（厳守事項）。修正はレイアウト結果という外部観測可能な挙動を変えるため proposals.md P49 に記録済み（W4a-T 記録、W4a-S 据え置き）。現状挙動は `tests/layout/dimension_conversion_test.rs` の特性化テスト 2 件（`test_length_percentage_auto_percent_to_taffy_not_normalized` / `test_length_percentage_to_taffy`）で固定済み。
- **NaN/inf の数値境界**: Taffy が算出した layout 値（`location` / `size`）や `BoxStyle` 由来の f32 を arrangement 変換へ流すが、非有限値が混入しても panic せず NaN/inf がサイレント伝播する。これは「panic 経路（DoS）」ではなく**入力検証の欠如（縮退）**に分類される。ただし (a) 当該数値の供給元は taffy 計算結果と内部 ECS 値であり外部入力（ファイル・TOML 等）ではないこと、(b) 検証を厳格化して NaN/inf を弾く対策は `Arrangement` 構築のエラー応答や入力拒否という**外部観測可能な挙動変更**を伴い R2.4/R5.2 で本ループ禁止であること、(c) dola 側の同型所見（P14/P25 等）と異なり layout 境界には外部指示書経由の流入経路が存在しないこと、から**現状安全（対策不要）と判定**。debug_assert による非有限チェック追加も検討したが、現状どこも `is_finite()` 不変条件に依存しておらず、0.0 サイズ等の正当な値や端値テストで誤発火し得る**投機的ハードニング**（karpathy「壊れていないものに防御を足さない」）となるため見送り。新規の挙動変更提案に値する流入経路・実害も検出されなかったため proposals 追記なし。
- 判定: **現状安全（P49 は記録済み・据え置き）。**

### 3. unsafe 境界 — 挙動非破壊対策 1 件適用

境界内の `unsafe` は 2 箇所の `unsafe impl Send/Sync`（いずれも `taffy.rs`）のみ。`transmute`/`from_raw`/`ManuallyDrop` 等の生ポインタ操作はゼロ。

- **`unsafe impl Send/Sync for TaffyStyle`（taffy.rs:14-15）**: W4a-S で CompactLength（calc 用 `*const ()`）の自動 Send/Sync 不成立事情と wintf が calc 未使用・ECS アクセス制御下である安全根拠を明記する **SAFETY 注記が既に追加済み**。今回追加対策なし。
- **`unsafe impl Send/Sync for TaffyLayoutResource`（taffy.rs:52-53）→ SAFETY 注記強化（適用）**: 同ファイルの `TaffyStyle` 兄弟 unsafe impl が W4a-S で厳密な SAFETY 注記を得た一方、この impl の元コメントは「TaffyTree は内部的に `*const ()` を持つが、ECS のリソース管理により所有権とライフタイムは保証されるため安全」という 2 行の概略に留まり、**なぜ `*const ()` がスレッド跨ぎで安全か**（TaffyTree がノードに taffy::Style を保持し同根の CompactLength 機構であること、生ポインタが ECS スケジューラ制御外へ漏れないこと）の根拠が兄弟注記より薄い**非対称**な状態だった。これは R5.5 が明示授権する「テスト保護外 unsafe のコメント等構造的整理」に該当し、W4a-S の兄弟注記強化と整合する。

### 適用した挙動非破壊対策（1 件）

| ファイル | 箇所 | 対策 | 種別 | 根拠 |
|----------|------|------|------|------|
| `taffy.rs` | `unsafe impl Send/Sync for TaffyLayoutResource`（52行付近） | SAFETY 注記を兄弟 `TaffyStyle` と同等の厳密さへ強化（CompactLength の自動 Send/Sync 不成立 → wintf は calc 未使用 → リソース排他所有 → アクセスは常に `ResMut`/`resource_mut` 経由で生ポインタが ECS スケジューラ制御外へ非漏出、ゆえに Send/Sync 安全）。コメントのみ追加・コード挙動不変。 | SAFETY/不変条件コメント | R5.5（テスト保護外 unsafe はコメント等の構造的整理に限定）。`TaffyLayoutResource` の実利用が `world/mod.rs:40` の `insert_resource` と systems の `ResMut<TaffyLayoutResource>` / `monitor_systems.rs` の `world.resource_mut::<>()` のみであることを grep 実証し、生ポインタ非漏出の根拠を裏付けた。 |

**回帰テストの追加なし（根拠）**: 本対策はコメントのみで新規コード経路・観測挙動が一切なく、特性化すべき新挙動が存在しない。`TaffyLayoutResource: Send + Sync` を assert する compile-time テストは `unsafe impl` により真が確定する自明な恒真式で回帰検知価値がなく、karpathy churn 回避により追加しない。`TaffyLayoutResource` は既存の taffy 統合テスト群（layout 156 件）が bevy スケジューラ経由で Send/Sync 越しに駆動しており、AFTER S2 全量がその非破壊を継続実証している。

## proposals.md へ回した候補

- **なし（P51 以降の新規記録なし）。** 点検で再発見した既知の挙動変更案件は P49（LengthPercentage 系 ÷100 欠落・据え置き）のみで記録済み。NaN/inf の入力検証欠如は layout 境界に外部入力の流入経路がなく実害・新規挙動変更提案に値する所見が検出されなかったため追記せず。新規の挙動変更を要する脆弱性対策は本境界に存在しなかった。

## verification (S2)

- BEFORE: 親検証済みベースライン（W4a-S 直後 = 1423 passed / 0 failed、HEAD `5280973`）を信頼し省略。
- AFTER（必須・全量実施）:
  - `cargo build --workspace` → **成功**（exit 0、wintf/areka 再コンパイル、4.55s）。
  - `cargo test --workspace` → **1423 passed / 0 failed**（全テストバイナリで FAILED 0。passed 合計をスクリプト集計し 1423、failed 合計 0 を確認）。ベースライン 1423 と**完全一致**（コメントのみ変更ゆえテスト件数不変。回帰ゼロ）。
- 反復用に `cargo test -p wintf --test layout` 系は AFTER S2 全量に含まれ 156 passed / 0 failed（配置伝播・taffy 統合を含む）。

## S3 clippy 所見（記録のみ・非ブロッカー）

`cargo clippy -p wintf` の W4a 境界内 span を抽出。**いずれも本セル編集に起因しない既存 lint**（W4a-S 記録と同一）であり、S3 は記録のみのため未適用。本セルのコメントのみ編集（`taffy.rs`）は**新規 clippy 警告を一切導入していない**（taffy.rs に clippy 指摘なし）。

- `this impl can be derived`: arrangement.rs:29 / dimension.rs:91 / dimension.rs:150（手書き `Default`。house スタイルとして W4a-S で意図的見送り）。
- `very complex type used`: arrangement_systems.rs:14/34/54 / taffy_systems.rs:29/189（bevy のシステム引数型。`type` 別名化は広域様式 refactor で churn）。
- `this if statement can be collapsed`: taffy_systems.rs:90/107/155/228（edition 2024 の let-chain 化。残存差分が触れていない既存コード。サージカル性のため見送り）。

## flaky

- `cue_performance_test::bench_pop_ready_empty_queue`（既知・負荷依存、W4a 境界外）: AFTER S2 全量で初回から pass（run 内に存在を確認・FAILED なし）。隔離再実行不要だった（フレーキー発火なし）。

## 自己レビュー

- 実装は本物（モック/スタブ/プレースホルダなし）。本セルの変更は SAFETY 注記の強化 1 件のみで、新たな unsafe・スタブ・TODO を導入していない。
- TODO/FIXME 残存なし（W4a-S で P50 スタブ削除済み。本セル追加分に TODO/FIXME なし）。
- 点検は境界内 7 ファイルを grep＋精読で網羅。panic 経路ゼロ・整数算術ゼロ・ゼロ除算リテラル定数のみ・unsafe 2 箇所（うち 1 件 SAFETY 強化、1 件は既強化済み）を確認。既知 P49 は厳守事項どおり不触で記録参照に留めた。
- テストは意味を持つ: 既存 1423（全量）/ 156（layout）が回帰検知器として機能し、コメントのみ変更の非破壊を件数完全一致（1423=1423）で実証。
- 境界遵守: 変更は `crates/wintf/src/ecs/layout/taffy.rs` 1 ファイルのみ（W4a 境界内・コメント追加）。tasks.md 未更新・コミット未作成・不要な proposals 記録なし。`.kiro/specs/` 機能spec文書・`vendors/`・W4b ファイルへの変更なし。
- 結論: 本境界は脆弱性耐性が高く、warranted な挙動非破壊対策は SAFETY 注記強化 1 件に限られた。残る所見（P49・NaN/inf 縮退）は挙動変更を伴うか実害ある流入経路がないため、それぞれ記録済み参照 / 現状安全と判定し churn を回避した。
