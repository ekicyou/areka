# Brief: areka-P0-text-typesetting

> 2026-10-01 `/kiro-discovery`（シェル内バルーン）で起票。roadmap「シェル内バルーン」節。**正典に無い areka 独自の組版**だが、普通のバルーンにも効く。本文の file:line は起票時（main `35209987`）の実測＝着手時に引き直すこと。

## Problem

- **縦書きで台詞を書く作者**（シェル内バルーンの参考ゴースト「窓際のぱすたさん」）: 句読点が列の頭に来る、閉じ括弧が列頭に落ちる、数字が横倒しになる。日本語の組版として読みにくい。
- 正典（ukadoc）には禁則・縦中横の決まりが無い（調査で 0 件）。

## Current State

- 折り返しは 1 字ずつ、または `budoux_newline,1` で文節（`crates/areka-emo-text/src/wrap.rs`・`segment.rs`）。**禁則は無い**。行末のぶら下げは「未実装」（`layout.rs` と `region.rs` の注記）。折り返し基準（`wordwrappoint`＝やわらかい）と絶対上限（`validrect`＝かたい）の二段構えは実装済み＝ぶら下げの継ぎ目はある（裁定: 記憶 areka-wordwrappoint-soft-validrect-hard）。
- **縦中横は無い**。`text_combine_upright`・`text_orientation` は**キーの名前の予約だけ**（`crates/areka-emo-text/src/writing.rs`・「M2 で検討、今は記録のみ」）。
- 縦書きの字の向きは DirectWrite の既定のまま。縦書きの字形の観測点が無い（`emo-text-canon-residue` 項目 11）。
- `layout.rs` 977 行＝**先に分割が要る**。

## Desired Outcome（2026-10-01 開発者確定）

- **禁則**: バルーンのキー `line_break`（CSS `line-break` と同じ値: `anywhere`＝禁則なし／`loose`／`normal`／`strict`）。**既定は `anywhere`**＝SSP と同じ折り返しのまま（互換を崩さない）。シェル内バルーンは `balloon.*`ブレスに 1 行書いて入れる。
- **ぶら下げ**: バルーンのキー `hanging_punctuation,allow-end`（CSS `hanging-punctuation`）。行末の句読点が折り返し基準を超えてぶら下がってよいが、絶対上限（`validrect` の遠辺）は超えない。
- **縦中横**: 台本 `\![text,combine-upright,12]`（指定した文字をまとめて横に並べ 1 字分に収める＝CSS `text-combine-upright: all`）と、バルーンのキー `text_combine_upright,digits 2`（半角数字が 2 桁までなら自動＝CSS Writing Modes Level 4 の `digits`）。既定は `none`。
- **字の向き**: バルーンのキー `text_orientation`（CSS と同じ `mixed`／`upright`／`sideways`）。既定は `mixed`。
- **縦書きの字形の確かめ**: 句読点・括弧・長音符が縦書き用の字形で描かれることを観測する点を作る（`emo-text-canon-residue` 項目 11 を引き取る）。
- areka 独自のキーの名前は既存の `writing_mode` と同じく「CSS のプロパティ名の `-` を `_` にしたもの」で揃える。台本の命令は `\![text,…]` の下に並べる（ルビの `\![text,ruby,…]` と同じ入り口・`\!` は汎用の入れ物 1 本で使う側が名前で選ぶ）。

## Approach

- 禁則は折り返しの判定（`wrap.rs` と layout の折り返し点）に「この字の前／後で切ってよいか」の表を足す。表は JIS X 4051 の行頭禁則・行末禁則の文字類に沿い、`loose`／`normal`／`strict` で範囲を変える（CSS Text Level 3 の定義を参照）。
- 縦中横は、範囲の字を 1 つの配置単位（1 字分の箱）として layout に渡し、描画で横組みの小さな塊として描く。
- 1 字ずつの表示（書記素単位）との関係: 縦中横の塊は 1 つの表示単位として一度に現れる（要件で確かめる）。

## Scope

- **In**: 上の 4 つのキー、`\![text,combine-upright,…]`、縦書きの字形の観測点、`layout.rs` の分割、決定論テスト、`doc/COMPAT_ARCHITECTURE.md` §8 への areka 独自の語の登記、`emo-text-canon-residue` の項目 11・15 の引き取りの記録。
- **Out**: ルビと列の間隔（`text-ruby`）・圏点（将来の `\![text,emphasis,…]`）・左右の寄せと影（`text-align-shadow-canon`）。

## Boundary Candidates

- 折り返しの規則（禁則・ぶら下げ）と、配置単位（縦中横）と、描画（字の向き）の 3 つ。

## Out of Boundary

- 正典の折り返しの意味（`wordwrappoint`・`validrect`）は変えない。既定値のままなら今と同じ結果になること。

## Upstream / Downstream

- **Upstream**: α の完成宣言。
- **Downstream**: `text-ruby`（同じ `layout.rs`・ルビの親文字は禁則の単位をまたがない）。

## Existing Spec Touchpoints

- **Extends**: `emo-text-canon-residue`（項目 11「縦書きの字形の観測点」と項目 15「行末のぶら下げ」を本 spec が引き取る＝同 brief へ追記済み）。
- **Adjacent**: `text-align-shadow-canon`（同じ layout・折り返し）・`balloon-color-emoji`（完了・書記素クラスタの単位）。

## Constraints

- 既定値で今の出力と同じになること（既存の決定論テストが無改変で緑）。
- 1 ファイル 1,000 行。決定論テスト網羅は必達。
