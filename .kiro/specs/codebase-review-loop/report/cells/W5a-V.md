# W5a-V: wintf テキスト描画 × 脆弱性レビューと非破壊対策

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点・基準・範囲

- セルID: W5a-V（領域 W5a「wintf テキスト描画」 × 観点 V「脆弱性レビュー」）。性質: **非挙動変更**（脆弱性点検＋挙動非破壊な対策のみ）。Feature Flag Protocol 不要。
- requirements（source 番号）: 2.3（脆弱性レビュー＋挙動非破壊対策）・2.4（挙動変更を伴う対策→提案記録）・2.5（前後 S2 非破壊）・2.7（列順 T→S→V。W5a-T/W5a-S 完了済みの回帰検知器上で実行）・2.8（テスト保護外でも深く解析・安全適用不能は提案記録）・4.1（自己レビュー＋検証）・5.1（外部観測可能挙動を変更しない）・5.2（挙動変更必要時は提案記録）。
- design: Security Considerations（L512-516: unsafe 境界・整数変換・ハンドルのリーク/二重解放・外部入力検証・panic 経路 DoS を点検し、挙動を変えない範囲＝内部チェック・debug_assert・安全な型置換のみ投入。API/エラー応答を変える対策は proposals へ）、CellExecutor 観点別規則 V（L338）、提案記録様式（L453）、セル断片様式（L440）。
- 領域（boundary = `crates/wintf/src/ecs/widget/text/`、tests/ の該当ドメイン含む）: `mod.rs`・`label.rs`・`draw_labels.rs`・`typewriter.rs`・`typewriter_ir.rs`・`typewriter_layout.rs`・`typewriter_draw.rs` の 7 ファイル。境界外には一切触れていない。
- 起点: W5a-S 適用後のクリーンなワークツリー（HEAD `33c729d`、親検証済みベースライン 1467 passed / 0 failed）。
- 直前セルからの申し送り: W5a-T 所見5・W5a-S「W5a-V 申し送り」節 = `TypewriterLayoutCache` の手動 `unsafe impl Send/Sync`（typewriter.rs:291-292）の不変条件点検（本セルの主対象）。

## 点検手法

境界内 7 ファイル（うち in-source tests 2 ファイル: typewriter.rs / typewriter_ir.rs）を grep（`unsafe`/`unwrap(`/`expect(`/`panic!`/`unreachable!`/`todo!`/`as `/添字 `[`）＋全文精読で走査。さらに到達可能性判定のため、境界外の依存（`com/dwrite.rs` の `create_text_format`/`create_text_layout`/`get_cluster_metrics` ラッパ、windows-rs 0.62.2 の COM 型の Send/Sync 生成状況）を**読み取りのみ**で確認し、外部入力が DirectWrite へ渡る経路と unsafe の健全性根拠を実証した。

unsafe の冗長性（後述）は**一時的なコンパイル・プローブで実証**: 手動 impl を `assert_send_sync::<TypewriterLayoutCache>()` の静的表明に差し替え `cargo build -p wintf` が成功することを確認 → 即座にプローブを完全 revert（ワークツリーが HEAD と一致することを `git diff --quiet` で検証）。

## 発見した脆弱性候補と判定

### 1. unsafe 境界 — `TypewriterLayoutCache` の手動 `unsafe impl Send/Sync`（申し送りの決着）→ 健全（かつ冗長）。SAFETY 注記＋静的特性化テストを適用

`typewriter.rs:291-292` で `IDWriteTextLayout`（COM）を保持する `TypewriterLayoutCache` に手動で `unsafe impl Send/Sync` を付与している。点検の結果、**健全**であることを 2 段で実証した:

- **windows-rs 側の付与**: windows-rs 0.62.2（本 crate 使用版。Cargo.lock で `windows 0.62.2` 確認）は `IDWriteTextLayout` に対し `unsafe impl Send/Sync` を**無条件で生成済み**（`windows-0.62.2/.../DirectWrite/mod.rs:10807-10808`、feature gate なし）。DirectWrite のレイアウト系オブジェクトは読み取り中心利用に対しスレッドアジャイルとして扱われる。
- **構造体としての自動導出**: 内包フィールドは `text_layout: IDWriteTextLayout`（上記により Send+Sync）と `timeline: TypewriterTimeline`（String/Vec/f64/u32 のみのプレーンデータ＝自動 Send+Sync）の 2 つのみ。したがって本構造体は手動 impl が**無くても**自動で Send+Sync を導出できる。コンパイル・プローブ（手動 impl を除去し `assert_send_sync::<TypewriterLayoutCache>()` を置いて `cargo build -p wintf` 成功）で**冗長性を実証**した。
- **実利用上の排他性**: TextLayout への変更系呼び出し（`SetDrawingEffect` 等、typewriter_draw.rs:250/264）は Bevy の Draw スケジュール内で当該エンティティを排他参照するシステム（`draw_typewriters`）からのみ実行され、跨スレッドの同時アクセスは発生しない。
- **判定**: 手動 impl は**冗長だが健全**（vacuously sound）。撤去はロジック非介入の構造整理であり S 観点の churn 判断事項に属する（crate 全域 25 箇所の COM 保持コンポーネントが同じ明示 impl 慣習を採るため、本ファイルだけ撤去すると不整合 churn）。**挙動非破壊対策として SAFETY 注記で根拠を明文化し、手動 impl は明示で残置**（下記 適用1）。**proposals 不要**（撤去は挙動非破壊だが任意・churn のため記録不要、設計変更でもない）。
- 関連の非ブロッキング観測（境界外・本セルでは不修正）: sibling の `ecs/graphics/command_list.rs:29-33` の SAFETY コメントは「windows-rs の COM スマートポインタは自動では Send/Sync にならない（だからこそこの unsafe impl が必要）」と blanket 主張するが、これは型依存で普遍的に正しくない（`ID2D1CommandList`・`IDWriteTextFormat`・`IDWriteTextLayout` は 0.62.2 で Send+Sync 付与済み＝当該 impl も冗長。`IWICBitmapSource` 系は非 Send で genuine に必要）。本コメントは W5a 境界外のためコメント精度の是正はしない（CONCERNS に記録）。

### 2. テキストリソースのリーク・二重解放 — 現状安全（対策不要）

- 境界内に `ManuallyDrop`/`mem::forget`/手動 `Release`/`transmute` は**ゼロ**（grep 確認）。保持される COM 資源（`IDWriteTextLayout`・`IDWriteTextFormat`・`ID2D1CommandList`・各 brush）はすべて windows-rs バインディングが返す所有インターフェイスで、Rust の `Drop` で Release される。
- **ライフサイクル対称性**: `TypewriterLayoutCache` は `init_typewriter_layout`（typewriter_layout.rs:181-183）で生成・挿入され、`invalidate_typewriter_layout_on_arrangement_change`（同:26）で Arrangement 変更時に `remove::<TypewriterLayoutCache>()` される。remove は `on_layout_cache_remove` フック（typewriter.rs:322、ログのみ）を経て Drop へ至り Release。再生成は次フレームの init が担う。生成⇔破棄は対称で、無効化→再生成サイクルでのリークなし。
- `TextLayoutResource`（label.rs）も `Option<IDWriteTextLayout>` を保持し `on_text_layout_remove`（ログのみ）→ Drop で Release。`draw_typewriters` 内で毎フレーム生成される CommandList/brush はローカル変数で、挿入される `GraphicsCommandList` 以外はスコープ終端で Drop（Release）される。**現状安全。**

### 3. 外部入力の検証（フォント名・テキスト内容）— 現状安全（対策不要）

ユーザ提供文字列が DirectWrite へ渡る経路は (a) `Typewriter.font_family`/`Label.font_family`、(b) トークン由来 `full_text`/`Label.text`。いずれも `windows::core::HSTRING::from(&String)`（typewriter_layout.rs:97/150、draw_labels.rs:74/122）で HSTRING 化される。

- `HSTRING::from` は UTF-8→UTF-16 の**全域変換**（`From` シグネチャ＝不可謬、panic なし）。HSTRING は長さ前置のため**内部 NUL・制御文字・任意 Unicode を許容**し、空文字でも空 HSTRING を生成する。極端長は確保失敗時に windows-rs 側で扱われるが、本 crate 側に検証由来の panic 経路はない。
- DirectWrite 呼び出しは `Result` 返却ラッパ（`create_text_format`/`create_text_layout`、いずれも `?`/`match` で受ける）を経由し、不正フォント名・空文字等で失敗しても境界システムは `warn!` + `continue`（typewriter_layout.rs:110/159、draw_labels.rs:86-93/126-133）で**スキップに縮退**する。`create_text_layout`（com/dwrite.rs:76-88、境界外・読み取り確認）は null PCWSTR を空スライスへ事前分岐し `as_wide` の非 null 前提を満たす。
- **空文字ガード**: `init_typewriter_layout` は `full_text.is_empty()` で `continue`（typewriter_layout.rs:68）。空入力で TextLayout を作らない。**現状安全（panic/未定義動作なし、外部入力由来 DoS 経路なし）。**

### 4. panic 経路 — 現状安全（対策不要）

境界内プロダクション経路の `unwrap()`/`expect()`/`panic!`/`unreachable!`/`todo!` は**ゼロ**。grep ヒットの内訳:
- `panic!` 7 件・`Entity::from_raw_u32(..).unwrap()` 4 件 … すべて in-source tests（typewriter.rs / typewriter_ir.rs の `mod tests`）。プロダクション非経路。
- `draw_labels.rs:31` / `typewriter_draw.rs:102` の `DEFAULT_FOREGROUND.as_color().unwrap()` … `DEFAULT_FOREGROUND` は `const` Brush（外部入力ではない）で、`as_color()` は当該 const に対し常に `Some` を返す不可謬 fallback。リリースでも到達不能 panic。**現状安全。**
- `label.rs:31` の `.unwrap()` … doc コメント（`/// world.spawn(... .unwrap())`）内の例示。実コードではない。
- 配列・スライス添字: 境界内に生添字 `[i]` は**なし**（`timeline.items[self.next_item_index]`（typewriter.rs:222）はループ条件 `next_item_index < items.len()`（:221）直下で範囲内が保証される全域アクセス。W5a-T の12件が `update` の単調前進・境界を特性化済み）。**現状安全。**

### 5. 整数境界 — 現状安全（対策不要）

- `typewriter.rs:256` `visible_cluster_count as f32 / total_cluster_count as f32` … u32→f32。除算は直上の `if timeline.total_cluster_count > 0`（:255）でゼロ除算を構造的に排除（else 枝は `progress = 1.0`）。W5a-T `test_..._zero_clusters_completes_immediately` で固定済み。
- `typewriter_layout.rs:194` `cluster_metrics.len() as u32` … usize→u32。クラスタ数は DirectWrite が返すテキスト長相当で、現実テキストで u32 を超えない（>40 億クラスタは非現実）。切り捨て非発生。
- `typewriter_draw.rs:125` `visible_count as usize`（u32→usize 拡大・無損失）、`:126` `m.length as u32`（`DWRITE_CLUSTER_METRICS.length` は u16→u32 拡大・無損失）。
- `typewriter_draw.rs:222` `full_text.chars().count() as u32`（usize→u32）と `:247/:261` の減算 `total_text_length - visible_text_length` … 減算はいずれも `if visible_text_length < total_text_length`（:224）ガード内でのみ実行され、**アンダーフロー非発生**（供給元が異なる 2 値でも `<` 成立時のみ減算するため差は常に正）。`as u32` 切り捨ては >40 億文字（>4GB 文字列）でのみ発生し非現実。**現状安全。**

## 適用した挙動非破壊対策（1 ファイル・2 箇所、+23 行）

| ファイル | 箇所 | 対策 | 種別 | 根拠 |
|----------|------|------|------|------|
| `typewriter.rs` | `unsafe impl Send/Sync` 直前（:291 前） | `IDWriteTextLayout`（windows-rs が Send+Sync 付与済み）＋`TypewriterTimeline`（プレーンデータ）により自動 Send+Sync 導出可＝手動 impl は健全かつ冗長である旨と、実利用上の排他参照根拠を記す SAFETY コメント（11 行） | SAFETY/不変条件コメント | コメントのみ・コード挙動不変。申し送りの unsafe 健全性根拠を明文化（design Security Considerations「内部チェック・debug_assert・安全な型置換」枠の注記）。冗長性はコンパイル・プローブで実証済み。 |
| `typewriter.rs` | in-source `mod tests`（`use super::*;` 直後） | `test_typewriter_layout_cache_is_send_sync`（`fn assert_send_sync<T: Send + Sync>()` を `TypewriterLayoutCache` に適用するコンパイル時静的表明、+ 12 行の doc/コメント） | 特性化/回帰テスト（S9 命名準拠） | `TypewriterLayoutCache` が Send+Sync である不変条件をコンパイル時に固定。device 非依存（型のみで実 IDWriteTextLayout 不要）。将来フィールド追加で Send 性が壊れた場合（手動 impl を併せて撤去した場合）に検出する。 |

合計 +23 行（`git diff --stat`: 1 file changed, 23 insertions）。プロダクションロジックの変更は**ゼロ**（SAFETY コメント + コンパイル時表明テストのみ）。境界内に収束、tests/ への新規ファイルなし（既存 in-source `mod tests` への追記1件）。

## proposals.md へ回した候補

- **新規記録なし**（P55 採番なし）。挙動変更を要する脆弱性対策（panic→Result 化・入力検証の厳格化・unsafe 設計変更等）に該当する実在脆弱性は本境界に検出されなかった。手動 unsafe impl の撤去は挙動非破壊だが任意かつ churn のため proposals 化もしない（設計変更ではない）。
- 既知 proposals の再発見（重複記録なし・参照に留めた）: **P54**（`convert_to_timeline` の純粋ロジック分離）— typewriter_layout.rs:188 の DirectWrite 密結合は本セルでも単体不能だが、これはロジック構造変更（S 観点）であり V 観点の脆弱性ではないため二重記録しない。

## verification (S2)

- BEFORE: 親検証済みベースライン（W5a-S 直後 = 1467 passed / 0 failed、HEAD `33c729d`、クリーンワークツリー）を信頼し省略（design フェーズ0 規定 + 親指示に従う）。
- AFTER（必須・全量実施）:
  - `cargo build --workspace` → **成功**（exit 0、wintf/areka 再コンパイル、4.52s）。
  - `cargo test --workspace` → **1468 passed / 0 failed**（per-binary 集計: passed 合計 1468・failed 合計 0・ignored 32。20 本の result 行すべて failed=0）。ベースライン 1467 から **+1 = 追加した特性化テスト `test_typewriter_layout_cache_is_send_sync` と一致**（既存テストの削除・変更ゼロ）。
  - 触れたバイナリの内訳確認: `wintf` lib（in-source）が **262 → 263（+1）**、他バイナリは増減なし＝追加1件が lib に着地し既存は不変。
  - 反復検証: `cargo test -p wintf --lib text::` で **25 passed / 0 failed**（W5a-S の 24 + 新規1。text モジュール全件が緑）。
- 件数整合（1468 = 1467 + 1）でコメント/テスト追加の挙動非破壊を実証（既存テストはコメント追加に影響されずそのまま通過）。

## RED フェーズ代替の検証

追加1件（`test_typewriter_layout_cache_is_send_sync`）は既存型の Send+Sync 性の characterization のため RED は N/A（GREEN by construction = コンパイルが通れば成立）。期待値は実装と独立に「内包フィールドがすべて Send+Sync ⇒ 構造体は自動 Send+Sync」という型システム規則から導出し、さらに**逆方向の実証**として一時プローブで手動 impl を除去しても `assert_send_sync` がコンパイル成功することを確認（＝手動 impl が冗長であり、テストの主張が現行型を正確に固定していることを相互確認）。初回コンパイルで成立、矛盾なし。

## clippy（S3・記録のみ・非ブロッカー）

- `cargo clippy --workspace` はワークスペース全域で 156 警告（W5a-S と同数）。text/ 境界内は正確に**3 件のみ**で、いずれも W5a-S 時点と同一の既存 lint:
  - `draw_labels.rs:31` — type_complexity（Bevy `Query` 型）
  - `typewriter_draw.rs:295` — type_complexity（Bevy `Query` 型）
  - `typewriter_draw.rs:161` — collapsible_if（`draw_typewriters` 内、COM 域）
- 本セルの編集（typewriter.rs への SAFETY コメント・コンパイル時表明テスト）は**新規 clippy 警告を一切導入していない**（typewriter.rs に clippy ヒットなし）。3 件はいずれも COM 域内/Bevy 慣用クエリの既知見送り対象（W5a-S 参照）。S3 規定によりブロッカーとせず記録に留める。

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue`（W5a 境界外 `tests/ecs`）は本セルの `cargo test --workspace` 全量実行で合格（FAILED 行ゼロ・隔離再実行不要）。本セルの変更とは無関係。

## 自己レビュー

- 実装は本物（モック/スタブ/プレースホルダ/TODO なし）。本セルの変更は SAFETY コメント1・コンパイル時静的表明テスト1のみで、新たな unsafe・スタブを導入していない。一時プローブは完全 revert 済み（最終 diff は +23 行のみ）。
- 点検は境界内 7 ファイルを grep＋精読で網羅し、申し送りの主対象（手動 unsafe impl Send/Sync）の健全性をコンパイル・プローブと windows-rs ソース確認で二重実証。unsafe 境界・リソースリーク・外部入力経路・panic 経路・整数境界の 5 観点すべてを判定し、warranted な挙動非破壊対策は SAFETY 注記＋静的特性化テストの 2 箇所に限られた（その他はすべて現状安全と判定し churn 回避）。挙動変更を要する実在脆弱性は不検出のため proposals 新規ゼロ。
- 件数の実測整合: S2 全量 1468 = 1467 + 1（追加テスト1）。lib 262→263。clippy 156（不変）・text/ 3 件（不変）。すべて git diff・cargo test 実測と一致（推測なし）。
- 境界遵守: 変更は `crates/wintf/src/ecs/widget/text/typewriter.rs`（W5a 境界内）のみ。tasks.md 未更新・コミット未作成・境界外/`vendors/`/機能spec文書/proposals.md への変更なし。
- 結論: 本境界は脆弱性耐性が高い。申し送りの手動 `unsafe impl Send/Sync` は健全（windows-rs が `IDWriteTextLayout` に Send+Sync を付与済みのため冗長でもある）で、SAFETY 注記＋静的特性化テストの挙動非破壊対策で根拠を固定した。リソースリーク・外部入力・panic・整数境界はいずれも現状安全と判定し、挙動変更を要する対策は不要のため proposals 新規記録なし。
