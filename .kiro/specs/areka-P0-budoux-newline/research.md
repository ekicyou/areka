# Gap Analysis: areka-P0-budoux-newline

> 確定済み requirements.md（R1〜R9）と**現行コードベース（`areka-P0-newline-defer` マージ後）**の差分分析。
> 情報提供が目的であり実装判断は下さない（複数案を対置）。言語 = ja（spec.json）。

## 0. 調査サマリ（3–5 bullet）

- **`writing_mode` が完全な前例**。純 areka 拡張キーの「parser 生文字列転記（`BalloonModel` の `Option<String>` フィールド＋accessor）→ emo-text 語彙解決（`1`/`true`→ON・欠落/`0`/`false`→OFF・未知値→`warn!`+フォールバック）」という R1 が要求する全経路が `writing_mode` に実装済みで、`budoux_newline` はこの型をそのまま複製できる（`crates/areka-parsers/src/balloon/{model.rs,parse.rs}`・`crates/areka-emo-text/src/writing.rs`）。
- **折返しの単一実点は `LayoutEngine::layout`（`layout.rs:169`）**。newline-defer マージ後、折返し判定は「①可視 prefix 打切り→②保留改行フラッシュ→③折返し判定（`layout.rs:228`＝`inline_pos + advance > threshold`）→④配置」の 4 段ゲートに整理済み。budoux は**③の分割点選択だけ**を差し替える（閾値源 `TextRegion::wrap_threshold` は不変・R2.4/R5）。
- **全文 lookahead が構造的に可能**。`layout()` は全 `items`（追記正本の全量）を受け取り `placed == visible_count` で可視を切る。従って「塊先頭配置時に塊全体が収まるかを可視 prefix に依らず先決」（R7.1）は、可視ゲート前の全 `items` を走査すれば成立し、リフロー跳び不発生が自然に担保できる。
- **budouy 0.2.2 の API を実確認済み**（docs.rs 実取得）: `Parser` 構造体＋`budouy::model::load_default_japanese_parser()`（`vendored-models` feature）＋`parser.parse(&str) -> Vec<&str>`。決定論・オフライン。crate は既に workspace 未依存（新規追加）。pasta 上流は別文脈（`pasta.toml` の `budoux=[...]` は行長設定値でありcrate依存ではない）。
- **主要な設計決定は「segment 計算の置き場と layout への受け渡し方」**。state.rs 非改変（W3 choice-render 宛先・干渉回避）を保つには、segment 純関数を state の外（layout 隣接の新規純モジュール）に置く必要がある。layout signature 変更は約 20 の呼び出し点（大半がテスト）へ機械的に波及する。

---

## 1. 要件→資産マッピング（ギャップタグ: 既存流用 / Missing / Constraint）

| 要件 | 必要技術要素 | 現行資産 | ギャップ |
|---|---|---|---|
| **R1** `budoux_newline` 転記＋語彙解決 | parser 生文字列転記フィールド／emo-text 語彙解決（ON/OFF/warn+fallback） | `writing_mode` の完全前例：`model.rs:38/90`（フィールド＋accessor）・`parse.rs:96`（転記）・`writing.rs:63`（`resolve` の match＋`warn!`） | **Missing（低）**: `BalloonModel` へ `budoux_newline: Option<String>` 追加（`new` 引数増＝全 `BalloonModel::new` 呼出更新）・`parse.rs` 転記1行・accessor・新規 `WrapMode`（or bool）解決関数。すべて `writing_mode` の写経 |
| **R2** 分かち書き境界ワードラップ | 全文→セグメント境界列（純関数）＋layout の③分割点差替 | `layout.rs:228` の char 単位折返し・`TextItem`（`state.rs:62`）・`TextRegion::wrap_threshold`（`region.rs:228`） | **Missing（中）**: budouy セグメンテーション純関数（新規 `segment.rs` 想定）＋layout ON 経路（塊 advance 合計 vs 残り行幅の先決）。閾値源は不変流用 |
| **R3** 長大セグメント文字単位縮退 | 「行頭からでも収まらない塊のみ char 折返し」分岐 | `layout.rs:228` の既存 char 折返し＋行頭1グリフ配置（`layout.rs` 無限折返し回避）が縮退先として再利用可能 | **Missing（低）**: 塊 advance 合計 >（閾値−行頭inline開始）判定→当該塊のみ既存 char 経路へ委譲。縮退は塊に閉じ後続塊で budoux 判定再開 |
| **R4** OFF 経路（既定）不変 | OFF 時に既存コードパス完全不変・非回帰檻兼用 | 既存 `layout.rs` テスト檻（横/縦/縮退/deferred 全網羅） | **Constraint**: layout signature 変更時、OFF 経路が旧挙動 byte 等価であることを既存檻で担保する構造にする（新パラメータの OFF 既定値で旧経路へ落ちる分岐設計が必須） |
| **R5** 明示改行・スクロール意味論不変（newline-defer 両立） | 保留改行実体化点でのワードラップ適用・両意味論の無矛盾 | newline-defer の `pending: Option<f32>`＋4段ゲート（`layout.rs:196–256`）・`visible_window`（不変） | **Constraint（中）**: ②保留フラッシュ（inline_pos リセット）と③budoux 分割点判定の順序整合。フラッシュ直後の行頭で塊先決を走らせる weaving が要る（両者テスト檻共有） |
| **R6** 縦書き同一規則 | 軸読み替え正準表上で同一ワードラップ/縮退 | `layout.rs` の単一読み替え式（`inline_start`/`block_start`/`block_dir`）— 分岐なし | **既存流用（低）**: 分割点判定は行内軸の `inline_pos`/`advance` 上の演算ゆえ 3 方向共通式に自然に乗る。新規分岐不要 |
| **R7** typewriter 整合／リフロー跳び不発生 | 塊先頭配置時に全文 lookahead で行送り先決 | `layout()` は全 `items` 受領＋`placed==visible_count` 打切り（`layout.rs:204`）＝全文 lookahead 構造済み | **既存流用（中）**: セグメント境界は全 `items`（可視ゲート前）から計算する原則を守れば成立。可視 prefix でセグメントを切ると跳ぶ＝設計で明文化必須 |
| **R8** 決定論・全網羅検証 | ネット非依存モデル同梱・`FixedMetrics` 注入で純検証 | `FixedMetrics`（`layout.rs:84`）・純粋層構造檻（`lib.rs:104`＝`windows` import 禁止のみ） | **既存流用（低）**: budouy は決定論・オフライン。新規 `segment.rs` は純粋層（`windows` 非依存ゆえ構造檻に抵触せず・budouy import 可）。境界計算/折返し/縮退/OFF 不変を metrics 非依存で檻化 |
| **R9** 実機確認 | fixture balloon descript へ `budoux_newline,1` 追記・有界 auto-exit＋AI vision | fixture = `crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku/`（`descript.txt` 基層＋`balloons0s.txt`/`balloonk0s.txt` 画像別層）・`AREKA_APP_SMOKE_EXIT_MS` 既存 | **Constraint（低）**: fixture 1 行追記のみ（pasta/submodule 非改変・scope 内）。emo2-text byte 等価盲点ゆえ出力画像目視必須（MEMORY: emo-text byte 等価は既定フォントの盲点） |

---

## 2. 実装アプローチ（複数案）

要件は emo-text 内完結（brief 開発者選択）で確定しているため、案の分岐は**crate 選定でなく「segment 計算の配置と layout への配線」**に集約される。

### 主要決定 D1: セグメント境界の計算主体と layout への受け渡し

- **案 A（layout 内で segment 計算）**: `layout()` に `WrapMode` を渡し、ON 時は `layout()` 内部で新規 `segment::segments(items)` を呼んで境界を得る。
  - ✅ 呼び出し側（actor.rs 本番＋テスト）は `WrapMode` 1 引数追加で済む。全文 lookahead が `layout` 内に閉じ整合が自然。
  - ❌ `layout()` が budouy へ依存（純粋層内だが Parser インスタンス生成コストを毎 present_frame で払う懸念→ `OnceCell`/thread_local キャッシュが要る）。`layout` の責務がやや太る。
- **案 B（呼び出し側で precompute し境界列を渡す）**: 新規 `segment.rs` が `&[TextItem] → SegmentPlan`（run 別の glyph-index 境界列）を返し、`actor.rs` 側で present_frame ごとに算出して `layout(..., &plan)` へ渡す。
  - ✅ `layout` は budouy 非依存のまま（境界列という値だけ消費）。Parser キャッシュを wiring 側に持てる。segment を独立に全網羅檻化しやすい。
  - ❌ 全呼び出し点（テスト約20）が plan 引数を組む必要（OFF は空 plan）。配線がやや増える。
- **案 C（ハイブリッド）**: `segment.rs` は純関数として境界を計算（案 B の分離）、ただし `layout()` が `WrapMode::Budoux(&SegmentPlan)` の形で受け取り、OFF は `WrapMode::CharByChar`（境界不要）。Parser キャッシュは segment モジュールが `OnceCell` で内部保持。
  - ✅ layout は境界列を消費するだけ（budouy 非依存維持）・segment は純関数で単独檻化・呼び出し側は `WrapMode` enum 1 個で分岐（OFF は境界計算をスキップ）。R4 の「OFF 既定値で旧経路へ」を enum variant で構造化できる。
  - ❌ `WrapMode` enum に境界列参照を載せる型設計が要る（ライフタイム）。

**推奨の方向性（design で確定）**: 案 C 系。理由 = (1) R4 の OFF 不変を「enum variant による経路分離」で構造保証でき既存檻が非回帰檻を兼ねやすい、(2) R8 で segment 純関数を単独全網羅檻化でき layout の budouy 非依存（純粋層規律・Parser キャッシュ局所化）を保てる、(3) R7 の全文 lookahead は「境界は全 items から計算・可視は layout が別途ゲート」と役割分離できる。ただし案 A（layout 内計算）も呼び出し波及が最小で有力＝**design Topic**。

### 主要決定 D2: `WrapMode` 型と `ResolvedBalloonText` への配線

- `WritingMode`（`writing.rs`）と同格で `WrapMode`（or bool）を新設し、`ResolvedBalloonText::resolve`（`actor.rs:106`）で `WritingMode::resolve` と並べて解決する（`mode`/`region`/`font` に `wrap`/`budoux` を追加）。R1 の語彙解決（`1`/`true`→ON・`0`/`false`/欠落→OFF・未知値→`warn!`+OFF）は `WritingMode::resolve` の match を写経。
- 配線点は `actor.rs:482` の唯一の本番 `LayoutEngine::layout` 呼び出し（＋`present_frame`）。`resolved.wrap` を layout へ渡す。

### 主要決定 D3: セグメント境界 ↔ glyph index 対応

- グリフ単位は Rust `char`（`state.rs` 正準・M1 は書記素クラスタ結合なし・emo2 fixture は結合文字不使用）。budouy `parse` は入力文字列のスライス列 `Vec<&str>` を返すため、各チャンクの `chars().count()` を累積すれば glyph-index 境界へ 1:1 で写せる。サロゲート/結合は既存 glyph 化経路（`char` 単位）に一致。
- **run 分割（R2.5）**: `LineBreak` で区切られた各極大グリフ run を独立にセグメント化し、run をまたいで塊を結合しない。`Clear` は state を全消去するため単一 `items[]` 内には現れず（items 自体が消える）、実質 run 境界は `LineBreak` のみ＝design で確認。

### 主要決定 D4: 長大セグメント縮退（R3）と deferred-newline（R5）の織り込み

- 縮退判定 = 塊 advance 合計 >（閾値 − 行頭 inline 開始）なら当該塊だけ既存 char 折返し経路（`layout.rs:228`＋行頭1グリフ配置）へ委譲。縮退は塊に閉じ、後続塊で budoux 判定再開。
- deferred-newline との順序 = ②保留フラッシュ（`inline_pos = inline_start`・current 空）直後の行頭で③塊先決を走らせる。フラッシュで行頭にリセットされた状態が「塊先頭かつ残り行幅最大」になる整合を design で明文化（両者テスト檻共有ゆえ回帰檻を同時に更新）。

---

## 3. 工数・リスク

- **工数: M（3–7 日）**。内訳 = parser 転記（写経・S）＋`WrapMode` 語彙解決（写経・S）＋budouy 依存追加＋`segment.rs` 純関数（新規・S–M）＋`layout()` ON 経路＋signature 波及（中・全呼出更新）＋決定論全網羅檻（境界/折返し/縮退/OFF 不変・M）＋実機（fixture 追記＋目視・S）。
- **リスク: 中**。
  - 新規依存 budouy（Apache-2.0・vendored-models・開発者承認済）: ビルド・feature 適合は低リスク（API 実確認済）だが Parser 生成コスト/キャッシュ設計は要検討。
  - layout signature 変更の約20呼出波及: 機械的だが OFF 経路の byte 等価担保が肝（R4）。
  - **最大リスク = リフロー跳び不発生（R7）× deferred-newline（R5）の相互作用**。可視 prefix でなく全 items からセグメントを計算する原則を破ると跳ぶ。②フラッシュと③塊先決の順序整合も含め、決定論檻で全分岐を固定する必要。

---

## 4. Research Needed（design フェーズへ持ち越す不確定事項）

1. **budouy Parser のライフサイクル/キャッシュ**: `load_default_japanese_parser()` のモデルロードコスト。present_frame 毎生成は不可＝`OnceCell`/thread_local か wiring 保持か（案 A/B/C で置き場が変わる）。
2. **budouy の std/no_std とビルド適合**: `vendored-models` の同梱モデルサイズ・`html` feature 不要（`vendored-models` のみ）・workspace ビルド（x64/arm64 本体・emo-text は host-32 非関与ゆえ i686 懸念なし）で問題ないかの実ビルド確認。
3. **`parse` の返却ライフタイム**: `Vec<&str>`（入力借用）を境界 index 化する際の所有権・アロケーション最小化。
4. **`WrapMode` の型形（enum vs bool＋境界列）**とライフタイム（案 C の境界参照）。
5. **run 分割の厳密規則（R2.5）**: `LineBreak` のみで run を割る前提の確認（`Clear` は items 消去ゆえ非該当）。空 run・記号のみ run・ASCII 混在 run の budouy 挙動。
6. **実機 fixture の追記先**: `emo2-kakukaku/descript.txt` 基層 vs `balloons0s.txt`/`balloonk0s.txt` 画像別層のどちらへ `budoux_newline,1` を置くか（後勝ちマージ・sakura/kero どちらのバルーンで可読性差を見るか）。

---

## 5. 設計判断アイテム（要件ディスカッションへ供給・番号付き）

1. **segment 計算の配置と layout 配線**（D1）: 案 A（layout 内計算・呼出波及最小）／案 B（wiring precompute・layout は budouy 非依存）／案 C（segment 純関数＋`WrapMode` enum・推奨方向）のいずれを採るか。state.rs 非改変（W3 干渉回避）は全案で満たすが、Parser キャッシュ置き場が変わる。
2. **`WrapMode` の型形**（D2）: `writing_mode` 同格の enum（`CharByChar`/`BudouxWordWrap`）か bool か。境界列を型に載せるか別引数か。**討議 #1 決定（2026-07-18）**: descript の受理値は bool（ON＝`1`/`true`・OFF＝`0`/`false`・双方受理）に留めるが、内部の値解決型は将来のワードラップ戦略名を第一級化しうる **`WrapMode` enum のシームで確保**する（実導出は bool・案③）。よって D2 は「enum で型シームを確保しつつ本 spec の受理・実導出は bool 2 値に閉じる」形で設計する。
3. **`BalloonModel::new` signature 拡張**: `budoux_newline: Option<String>` 追加で全 `BalloonModel::new` 呼出（parser/emo-text テスト多数）が更新される。`#[non_exhaustive]` 前提でも `new` は位置引数ゆえ波及。builder 化はスコープ外か。
4. **OFF 不変の構造保証手段**（R4）: 新パラメータの OFF 既定で旧 char 経路へ落ちる分岐を、既存 layout 檻が非回帰檻を兼ねる形にどう構造化するか（enum variant 分離 vs フラグ分岐）。
5. **リフロー跳び先決の明文規則**（R7×R5）: 「セグメント境界は全 items から計算・可視は別ゲート」「②保留フラッシュ直後の行頭で③塊先決」を design 不変条件として固定し、決定論檻で全分岐を檻化する範囲。
6. **長大セグメント縮退の境界式**（R3）: 「行頭からでも収まらない」の判定式（塊 advance 合計 >（閾値 − 行頭 inline 開始））と、縮退中の行頭1グリフ配置（既存無限折返し回避）との一致確認。
7. **budouy Parser キャッシュ戦略**（Research 1）: モデルロードを1回に抑える機構（`OnceCell`/thread_local）と純粋層規律（決定論檻）の両立。
8. **実機 fixture 追記先**（Research 6）: 基層 descript か画像別層か・sakura/kero どちらのバルーンで確認するか。
