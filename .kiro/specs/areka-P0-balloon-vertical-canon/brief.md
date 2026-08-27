# Brief: areka-P0-balloon-vertical-canon

## Problem

areka の縦書きは、SSP に縦書きが存在しなかった時期に自前拡張キー `writing_mode`（CSS 語彙・snake_case）で実装した。その後 **SSP 2.8.80（および 2.8.83）が縦書きの公式正典を確立**したため、いま次の互換ギャップがある:

- SSP 向けに書かれたバルーン（`vertical,1` を持つ descript.txt）は areka で**横書きのまま**表示される（`vertical` キーは未解析＝黙って無視）。
- areka の縦書きフィクスチャ（`writing_mode,vertical_rl`）は SSP に持ち込んでも縦書きにならない（未知キー無視＝非破壊だが非互換）。
- プロパティシステム `currentghost.balloon.scope(ID).vertical`（SSP 2.8.80）が無いため、縦書き判定を行うゴーストスクリプトが誤動作しうる。

互換ベースウェア戦略（ukadoc 準拠 SSP 代替先行）に照らし、**SSP 正典キーを第一級で受ける**必要がある。

## Current State

2026-08-27 実測（機械踏査済み・file:line は着手時に要再検証）:

- **バルーン descript 解析**: `crates/areka-parsers/src/balloon/parse.rs:71-141`（`map_merged`・2 層マージ :44-56）。`writing_mode` は :110 で生文字列転記のみ。**`vertical` キーは無い**。`origin.x/.y` :84-87・`wordwrappoint.x/.y` :88-91・`validrect.*` :92-97 は解析済み。`model.rs:20` が `writing_mode` を「areka 拡張キー（SSP キーではない）」と宣言・アクセサ `model.rs:129`。
- **語彙解決**: `crates/areka-emo-text/src/writing.rs:45` `WritingMode { HorizontalTb, VerticalRl, VerticalLr }`・`resolve(&BalloonModel)` :63・受理語彙 `horizontal_tb`/`vertical_rl`/`vertical_lr`（:19-24）・未知値は warn＋横書きフォールバック（:70-74）。
- **レイアウト**: 縦書きは軸リマップ 1 本道（別経路なし）。開始角 `region.rs:212-214`（VerticalRl→右上）・折返し軸選択 `region.rs:232-238`（縦= `wordwrappoint.y`・横= `.x`）・カラム送り `layout.rs:307-310`・`\_l` 軸スワップ `layout.rs:462-465`・DirectWrite 方向 `draw.rs:227-268`（`DirectionRecipe::for_mode`）。
- **プロパティシステム**: sylphya に `currentghost.balloon.scope(ID).*` 系は **1 件も無い**（`vertical`/`validwidth`/`validheight`/`lines` 全て 0 ヒット・2026-08-27 実測）。
- **非モデル化キー**: `sstpmessage.*`・`number.*`・`arrow*` は不採用ディストラクタとしてテストで明示（`validation_tests.rs:177-197`・`parse_tests.rs:157-164`）。
- **テスト資産**: `tests/vertical_fixture_test.rs`＋writing/region/layout/choice/viewbox/canvas の縦書きケース多数・フィクスチャ `examples/fixtures/emo2-vertical/descript.txt:13`（`writing_mode,vertical_rl`）。

## SSP 正典（2026-08-27 採取・ライブ ukadoc ＋ SSP 2.8.83 changelog 2026-08-26）

- **`vertical,0/1`**（2.8.80）: 1 で日本語縦書き（上→下へ字送り・右→左へ行送り）。座標指定の意味が変わる: `origin.x`＝1 列目の**右端**（既定 validrect.right）・`origin.y`＝字送り開始位置（既定 validrect.top）・`validrect` 不変・**折返しは `wordwrappoint.y`・`wordwrappoint.x` は無視**。
- **フォント**: 指定フォントの縦書き用異体（頭に `@`）が自動使用。異体が無い場合（バルーン同梱フォント・欧文等）は**環境の標準ゴシックの縦書き用異体へ自動差し替え**。
- **字送り基準の再解釈**: `\f[align,～]`＝列内の字寄せ（left＝上・right＝下）・`\f[valign,～]`＝行の厚み方向（top＝右・bottom＝左）・arrow0/arrow1＝右/左スクロールの意味・下線は列の**右側**。
- **制限**: communicatebox は横書きのまま。`balloon(s/k)*s.txt` でもサーフェス毎に指定可（切替時は前レイアウト崩れ＝SSP 側も許容）。
- **プロパティ**（2.8.80 導入・**2.8.83 で意味論改訂**）: `currentghost.balloon.scope(ID).vertical`＝縦書きなら 1。**2.8.83 現行**: `validwidth`＝列が並ぶ方向（右→左）に使える幅・`validheight`＝1 列の長さ（スクロールで不変）・`lines`＝収まる**列数**——いずれも**画面上の向き**基準。⚠ **ukadoc-mcp スナップショットは 2.8.80 時点の旧意味論**（「文字の送り方向基準」＝validwidth と validheight の役割が現行と逆）を返す。**本 spec の正典参照はライブ ukadoc ＋ changelog 2.8.83 とする。**
- **2.8.83 追加**（changelog 2026-08-26）: `sstpmessage.yb`（縦書き用）・`number.x`/`number.yb`（左上原点系の別名）・`\_l` は縦書きでも**バルーン画像基準の座標系のまま**（列送りは負の X で左へ）。

## Desired Outcome

1. `vertical,1` を持つ SSP 向けバルーンが areka で SSP と同じ縦書き挙動（座標再解釈・折返し・寄せ・下線側）で表示される。
2. `currentghost.balloon.scope(ID).vertical` が 0/1 を返す。
3. 自前拡張 `writing_mode` との共存規則が COMPAT に登記され、フィクスチャ・テストが正典キー側でも通る。

## Approach

**SSP 正典キー `vertical` を第一級で受け、既存の `WritingMode` 軸リマップ機構へ写像する**（`vertical,1` → `VerticalRl`）。レイアウト機構は既存資産を流用し、差分は「解析の受口」「意味論の SSP 突合」「プロパティ導出」に絞る。ゼロから縦書きを作り直さない。

## Scope

- **In**:
  - `vertical,0/1` の解析（2 層マージ込み）と `WritingMode` 解決への統合・`writing_mode` との優先順位裁定。
  - 縦書き時の座標意味論の SSP 突合: `origin.x`＝1 列目右端（既定 validrect.right）・`wordwrappoint.y` 折返し・`wordwrappoint.x` 無視——現行実装との一致検証と差分是正。
  - `\_l` の縦書き座標系: SSP 2.8.83 正典（バルーン画像基準・負 X で列送り）と現行の軸スワップ実装（`layout.rs:462-465`＝(y,x) 入替）の**突合と裁定**（非互換なら是正）。
  - `\f[align]`/`\f[valign]` の縦書き再解釈（left＝上・right＝下／top＝右・bottom＝左）の実装状況確認と是正。
  - 下線の描画側（列の右側）の確認。
  - フォント縦書き異体: DirectWrite は `@` フォント（GDI 機構）ではなくネイティブに縦組みグリフを扱うため、**挙動等価（グリフ直立・約物回転）を定義して COMPAT へ「areka 裁量」登記**。
  - プロパティ `currentghost.balloon.scope(ID).vertical` の sylphya 追加（最小）。
  - フィクスチャ・決定論テスト（`vertical,1` 版フィクスチャ追加・既存 `writing_mode` フィクスチャは共存検証に転用）。
  - COMPAT_ARCHITECTURE.md への登記（正典キー・拡張キー共存規則・DWrite フォント等価）。
- **Out**:
  - `sstpmessage.*`・`number.*`（2.8.83 の `.yb` 系含む）——バルーン機能ごと非モデル化（M2 で機能自体を検討する際に一括）。
  - arrow0/arrow1 のスクロール画像・SSTP マーカー送信元表示・ネットワーク更新進捗表示・communicatebox——機能自体が M1 非実装。
  - `validwidth`/`validheight`/`lines` プロパティ族の実装（sylphya に族ごと不在。**要件段階で「.vertical のみ＋族は縮退シーム登記」か「族ごと実装」かを裁定**——既定推奨は前者＝defer-canon 4 点セット）。
  - `vertical_lr`（SSP に対応物なし・areka 拡張のまま維持）。
  - budoux 改行との相互作用の拡張（現行挙動維持のみ）。

## Boundary Candidates

- 解析の受口（parsers）と語彙解決（emo-text writing.rs）——同一 spec 内の直列 2 段。
- レイアウト意味論の SSP 突合（region/layout）——差分が出た箇所だけ是正する検証駆動。
- プロパティ導出（sylphya）——独立に切れる最小追加。

## Out of Boundary

- バルーン画像・矢印・マーカー等の SSP バルーン UI 部品群（M1 非実装のまま）。
- 縦書きの品質向上（縦中横・ルビ等）——M2 の emo テキスト進化予約。

## Upstream / Downstream

- **Upstream**: `emo-text-layer`（completed・軸リマップ機構）・`areka-parsers` バルーン 2 層マージ・sylphya（統一プロパティ機構）。
- **Downstream**: `emo2-conformance-e2e`（W7）——ただし emo2 は縦書き痕跡なし（`doc/emo2-conformance-scope.md:61`＝M1 適合には不要）なので適合 14 項目へは非干渉。M2 のテキスト進化・バルーン美観系。

## Existing Spec Touchpoints

- **Extends**: なし（新規境界）。
- **Adjacent**: W6.95 同居 3 本——`present-write-coherence`（presenter/show.rs＝素）・`balloon-offset-dpi`（placement/follow 系＝素）・`scope-zorder-pinning`（zorder＋ghost config＝素。**要ウォッチ**: 両者とも sylphya の語彙表へ行を足す可能性＝行レベル隣接のみ・機械衝突は軽微）。

## Constraints

- 正典参照は**ライブ ukadoc ＋ SSP 2.8.83 changelog（2026-08-26）**。ukadoc-mcp スナップショットはプロパティ意味論が 2.8.80 時点で旧い（validwidth/validheight の役割が現行と逆）ことが確認済み——縦書き関連は必ずライブ側で裏取りする。
- `writing_mode` は areka 拡張として**維持**（`vertical_lr` は SSP に対応物が無い）。優先順位（両キー併記時）は要件段階の裁定事項——候補: 拡張キー明示指定が正典キーに勝つ（より特定的な指定を尊重）。
- SSP は未知キー無視のため、`vertical` と `writing_mode` の併記フィクスチャは SSP でも areka でも同じ見た目になる書き方が可能（移行推奨形として COMPAT へ記す）。
- 決定論テスト必達（deterministic-test-coverage-mandate）・実装ファースト・1,000 行目安。

## 要件段階の裁定事項（裁定密度の一覧）

1. `vertical` × `writing_mode` 併記時の優先順位（＋COMPAT 登記文）。
2. `\_l` 縦書き座標系: SSP 2.8.83 形へ揃えるか（現行軸スワップは自前拡張時代の解釈＝**非互換の疑い**・要実測突合）。
3. プロパティの範囲: `.vertical` 単独＋族は縮退シーム登記（推奨）か、`validwidth`/`validheight`/`lines` 族込みか（族は 2.8.83 の画面向き基準意味論で）。
4. フォント縦書き異体の挙動等価定義（DWrite ネイティブ縦組み vs GDI `@` フォント）。
5. `balloon(s/k)*s.txt` サーフェス毎切替の扱い（SSP は「崩れる」と明記＝同水準の許容で良いか）。
