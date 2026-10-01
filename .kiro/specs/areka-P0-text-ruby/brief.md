# Brief: areka-P0-text-ruby

> 2026-10-01 `/kiro-discovery`（シェル内バルーン）で起票。roadmap「シェル内バルーン」節。**正典に無い areka 独自の機能**だが、普通のバルーンにも効く。本文の file:line は起票時（main `35209987`）の実測＝着手時に引き直すこと。

## Problem

- **台詞にふりがなを振りたい作者**（参考ゴースト「窓際のぱすたさん」の「命（ひととき）」「永久（とわ）」のような当て読み）: 伺かには書き方も描き方も無い（ukadoc で「ルビ」「ふりがな」とも 0 件）。
- 参考資料の列の間隔（本文 27px・列ピッチ 48px＝本文＋ルビ＋字間、字送り約 29px＝字間 0.08em）を指定する手段も無い。

## Current State

- ルビの実装は 0。行（縦書きなら列）の間隔は「文字の高さ＋`line_gap`（既定 2px）」で、バルーンのキーで変えられない（`crates/areka-emo-text/src/state.rs` の行送り）。字間のキーも無い。台本の `\n[比率]` と `\_l` の `lh` 単位はある。

## Desired Outcome（2026-10-01 開発者確定）

- **台本 `\![text,ruby,親文字,ルビ]`**（例 `\![text,ruby,命,ひととき]`）。`\![text,…]` は areka の文字組みの指示の入り口（縦中横の `\![text,combine-upright,…]` と同じ枠）。親文字やルビに `,` を含むときは正典の決まりどおり `"…"` で囲む。青空文庫式（`｜命《ひととき》`）と新しいタグは採らない。
- **バルーンのキー `line_height`**（CSS と同じ: 数字だけなら文字の大きさの倍率〔参考資料は `1.9`〕・`px` 付きなら絶対値）と **`letter_spacing`**（CSS と同じ・参考資料は `0.08em`）。
- **ルビは行の間隔の内側に置く**＝列の幅は一定で、ルビのある列だけ広がることはない。収まらないほど狭いときははみ出させてログに 1 行残す。
- 縦書きではルビは列の右側、横書きでは上側（CSS `ruby-position` の既定と同じ）。
- 1 字ずつの表示では、ルビは親文字と同時に現れる（参考資料 §4）。

## Approach

- 親文字の並びを 1 つの配置単位とし、ルビを付属の小さな文字列として layout に持たせる（`TextItem` に注釈の項目を足す）。親文字はルビの単位の中で折り返さない（禁則の単位をまたがない）。
- ルビの文字の大きさは本文の半分（CSS の既定に合わせる）。キーで変えるかは要件で決める。

## Scope

- **In**: `\![text,ruby,…]` の転記と配置・描画、`line_height`・`letter_spacing`、縦書きと横書きの両方、1 字ずつの表示との合わせ、選択肢（`\q`）の中のルビ、決定論テスト、`doc/COMPAT_ARCHITECTURE.md` §8 への登記。
- **Out**: 圏点（将来の `\![text,emphasis,…]`）・ルビの肩付き／中付きの細かい揃え（要件で要否）・熟語ルビの自動分割。

## Boundary Candidates

- 台本の転記（`areka-sakura` の `\!` の受け取り＝汎用の入れ物から `text` を名前で選ぶ）と、配置（`layout`）と、描画（`draw`）。

## Out of Boundary

- 禁則と縦中横の規則（`text-typesetting`）。本 spec はそれを前提に、ルビの単位を禁則の判定へ渡すだけ。

## Upstream / Downstream

- **Upstream**: `text-typesetting`（同じ `layout.rs` を触る＝直列）。ルビのフォントは `balloon-font-file` の読み手を通る（機能の前提ではない）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `text-align-shadow-canon`（行の幅の判定）・`emo-text-canon-residue`。

## Constraints

- `line_height` の既定は今の行送り（文字の高さ＋2px）と同じ結果になること＝既存の決定論テストが無改変で緑。
- 1 ファイル 1,000 行。決定論テスト網羅は必達。
