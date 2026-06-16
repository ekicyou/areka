# W5a-S: wintf テキスト描画 × シンプル化（unsafe 保守則適用）

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点
- セルID: W5a-S（領域 W5a「wintf テキスト描画」 × 観点 S「シンプル化」）
- 性質: 非挙動変更（リファクタリング／簡素化）。Feature Flag Protocol 不要。
- requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3, 5.5（特に R5.5: テスト保護外の unsafe/GUI/COM は構造的整理に限定）
- design: S6（karpathy 基準）、S2 検証、S10 コミット規約、W5a 領域定義、S 観点手順、unsafe 保守則（design.md L177, L337）、セル断片様式（L440）、提案記録様式（L453）
- 回帰検知器: 直前 13.1 W5a-T が追加した特性化テスト16件（TypewriterTalk 状態マシン12・typewriter_ir FireEvent 2・Label/TextLayoutResource 既定値2）。本セルの簡素化後に S2 が緑であることが挙動非破壊の証拠。

## 調査範囲（boundary = `crates/wintf/src/ecs/widget/text/`）

7ファイルを精読し、各候補を「テスト保護下か」「unsafe/GUI/COM 域か」で分類した。

| ファイル | 性質 | 簡素化候補の所在 |
|----------|------|------------------|
| `mod.rs`（18 LOC） | re-export のみ | なし |
| `label.rs`（119 LOC） | プレーンなコンポーネント定義 + フック | **構造整理候補あり（適用）**。COM 域は `TextLayoutResource::new`（型のみ）程度 |
| `draw_labels.rs`（233 LOC） | **全面 DirectWrite/D2D 依存システム** | type_complexity（Query）のみ。ロジックは全て COM |
| `typewriter.rs`（352 LOC + tests） | `TypewriterTalk` は**テスト保護下の純粋状態マシン**。`TypewriterLayoutCache` は COM 保持 + 手動 unsafe impl | 純粋ロジックは既に最小（後述）。unsafe impl は W5a-V 担当 |
| `typewriter_ir.rs`（247 LOC 内 tests） | プレーンデータ型 | 既に最小。候補なし |
| `typewriter_layout.rs`（242 LOC） | **DirectWrite 依存システム** + `convert_to_timeline`（COM 密結合） | `full_text` 二重構築は P54 領域（後述・重複記録せず） |
| `typewriter_draw.rs`（378 LOC） | **全面 DirectWrite/D2D 依存システム** | collapsible_if（COM 域内・R5.5 で見送り） |

clippy（simplification 系 lint）の text/ 境界内ヒットは正確に3件のみ:
- `draw_labels.rs:31` — type_complexity（Bevy `Query` 型）
- `typewriter_draw.rs:295` — type_complexity（Bevy `Query` 型）
- `typewriter_draw.rs:161` — collapsible_if（`draw_typewriters` 内、COM 域）

## 適用した簡素化（1件）

### 適用1: `label.rs` のフック引数 `HookContext` の修飾正規化（構造整理・naming）

`label.rs` は `HookContext` を L5 で `use` 済みにもかかわらず、3 フックのうち `on_label_add`（L52）のみ短縮形を使い、`on_label_remove`（L76）と `on_text_layout_remove`（L115）が冗長に `bevy_ecs::lifecycle::HookContext` とフル修飾していた。後者2箇所を短縮形へ揃えた（net 0 LOC・2行の表記変更）。

- **挙動非破壊根拠**: 同一型（`use` 済みのパスエイリアス）への表記揺れの解消にすぎず、生成コードは完全に同一。型・シグネチャ・ロジックは不変。`cargo build --workspace` 成功 + S2 全量がベースラインと一致（1467/0、テスト増減ゼロ）で実証。
- **テスト保護**: フックの観測挙動（`on_label_add` の Visual 自動挿入）は `tests/visual/widget_visual_auto_insert_test.rs` 4件で保護下。本変更は型修飾のみで挙動経路に触れない。
- **R5.5 整合**: GUI/COM 隣接コードだが「命名・表記の構造的整理」に該当しロジック非介入。sibling の `typewriter.rs` は4フック全てで短縮形を使っており（L64/84/270/322）、本変更は確立済みモジュール慣習への整合（karpathy 3「既存スタイルに合わせる」）。

## R5.5 で構造整理に限定／見送った COM・GUI 域の候補

- **`typewriter_draw.rs:161` の collapsible_if（`draw_typewriters` 内）**: clippy は `if let Some(bg_color) = bg_color_opt { if let Ok(bg_brush) = ... }` の let-chain 統合を提案。しかしこれは (a) `dc.FillRectangle` を含む**全面 COM 依存システム内**の分岐（R5.5 で構造整理＝命名/コメント/自明重複に限定、スタイル lint の適用は範囲外）、(b) 当リポジトリは collapsible_if 警告をクレート全域で69件許容し let-chains を一切採用していない（採用すれば text/ 境界だけ不整合な churn・karpathy 3 違反）ため**見送り**。edition 2024 + rustc 1.96 でコンパイルは可能だが挙動・可読性の改善が乏しく churn のみ。
- **`draw_labels.rs:31` / `typewriter_draw.rs:295` の type_complexity（Bevy `Query`）**: クレート全域で30件発生する Bevy 慣用クエリの典型的偽陽性。`type` エイリアス化は text/ だけ局所適用すると不整合で、`Query` 型を読み解く可読性も実質下がる。codebase 全体が容認済みのため見送り（churn 回避）。

## proposals へ回した候補

- **新規記録なし**。
- ロジック構造変更に当たる候補は既存 **P54**（`convert_to_timeline` の純粋ロジック分離）で記録済みのため重複記録しない。本セルで検出した **`full_text` の二重構築**（`init_typewriter_layout` L59 が HSTRING 用に Text トークンを連結し、`convert_to_timeline` L196 が `timeline.full_text` として同じ連結を再生成＝同一論理文字列の二重 String アロケーション）は、消去するには COM 依存 `init_typewriter_layout` システムと `convert_to_timeline` のシグネチャ／データフロー変更を要し、まさに P54 のスコープ（convert_to_timeline 周辺のロジック構造変更）に内包される。タスク指示に従い P54 を参照し**重複記録しない**。

## 適用しなかった候補と理由（churn 回避等）

- **`TRANSPARENT_COLOR`（`D2D1_COLOR_F`）の `draw_labels.rs:16` ↔ `typewriter_draw.rs:25` 重複**: バイト同一の 6 行 private const が COM draw 2ファイルに重複。集約は共有 const の置き場所選定（例: `text/mod.rs`）と 2 つの COM ファイルへの import 追加を伴い、R5.5 の「自明な重複除去」を超えて COM draw モジュール構造の再編に踏み込む。自己文書的かつ低害な重複であり、可読性向上に乏しく churn が勝るため見送り（R5.5 GUI/COM 域の保守的判断、karpathy 2/3）。※ `shapes/rectangle.rs:68` の `TRANSPARENT_COLOR` は別型（`Color`）・別境界（shapes/）のため対象外。
- **`MIN_LAYOUT_SIZE`（=10.0）の `typewriter_layout.rs:78` ↔ `typewriter_draw.rs:325` 重複**: いずれも別ファイルの関数ローカル const。上と同様に共有化は import 結合の追加を要し、関数ローカルで完結する自明定数のため churn 回避で見送り。
- **`TypewriterTalk`（テスト保護下の純粋状態マシン）の `update`/`pause`/`resume`/`skip`**: W5a-T の12件で保護されており踏み込んだ簡素化が許される領域だが、精読の結果**既に S6「最小コード」を満たす**。`update` の progress 二分岐（`total_cluster_count > 0` で除算、else 1.0）はゼロ除算回避の必須分岐で統合不能、while+3アーム match は冗長な中間 Vec・デッドコード・統合可能分岐を持たない。挙動非破壊で可読性が明確に上がる変更が存在しないため不適用（churn 回避）。
- **`Label::default` の `direction: Default::default()`（L69）vs `typewriter.rs` の `TextDirection::default()`**: フィールド型既知の文脈で `Default::default()` は十分慣用的・明瞭。整合のための変更は無価値な churn のため不適用（karpathy 3）。

## S6（karpathy-guidelines）適合確認

- 適用1件は「既存の表記揺れの除去」のみで、新規抽象・投機的柔軟性・不要なエラー処理の追加はゼロ（rule 2）。変更2行は sibling 慣習への整合にトレースでき、自分の変更で孤児化したものなし（rule 3）。
- 成功基準: 「S2 全量がベースライン 1467/0 と一致＝挙動非破壊」を満たした（rule 4）。

## verification (S2)

- BEFORE: 親検証済みベースライン（クリーンツリー・1467 passed / 0 failed）を信頼し省略（親指示・design フェーズ0 規定に従う）。
- AFTER: `cargo build --workspace` 成功（exit 0）/ `cargo test --workspace` **1467 passed / 0 failed**（ignored 32、result 行20本すべて failed=0）。
  - ベースラインと完全一致＝テストの追加・変更・削除ゼロで全既存テストが新表記をそのまま通過＝挙動非破壊の裏付け。
  - 反復検証: `cargo test -p wintf --lib text::` で **24 passed / 0 failed**（W5a-T の回帰検知器を含む text モジュール全件が緑）。
- 変更ファイル: `crates/wintf/src/ecs/widget/text/label.rs`（+2/−2、net 0 LOC）のみ。boundary 内に収束。tests/ 不変。

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue`（W5a 境界外 `tests/ecs`）は `cargo test --workspace` 全量実行で合格（隔離再実行不要）。本セルの変更と無関係。

## clippy（S3・記録のみ・非ブロッカー）

- `cargo clippy --workspace` はワークスペース全域で 156 警告。text/ 境界内は正確に3件（`draw_labels.rs:31` type_complexity・`typewriter_draw.rs:295` type_complexity・`typewriter_draw.rs:161` collapsible_if）で、いずれも**本セルの変更前から存在**（修飾正規化は新規警告を導入せず・解消もしない）。3件とも上記「R5.5 で限定／見送り」の判断対象。S3 規定によりブロッカーとせず記録に留める。

## proposals

- 新規記録なし（P54 を参照。重複記録回避）。
- W5a-V への申し送り（W5a-T 所見5から継続）: `TypewriterLayoutCache` の手動 `unsafe impl Send/Sync`（typewriter.rs:291-292）の不変条件点検は 13.3 W5a-V 担当。本 S セルでは unsafe の意味論に踏み込まず（R5.5）。
