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


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 規模 M〜L（13〜18 タスク）。文字まわりの直列の列（`emo-text-file-split` → `shell-balloon` の後）。**`talk-fast-forward` とは並走できない**（棚卸⑳で訂正＝`areka-parsers/src/balloon/{model,parse}.rs` と `state.rs` を共有）。
- 合っていた点: 予約キーは `writing.rs` に在る・ぶら下げの「未実装」の注記は `layout.rs` と `region.rs` に在る。`layout.rs` の分割は `emo-text-file-split` が先に済ませる。
- **抜け**: ⑴ 描画の側も触る＝字の向きは `DirectionRecipe`（`draw.rs`）か行の TextLayout（`draw_line_store.rs`）、計測用は `draw_metrics.rs`。DirectWrite に縦中横の機能は無いので、塊を自前で描く（`viewbox_draw` 系が太る）。⑵ 縦中横の塊を「一度に現れる 1 単位」にすると `state.rs` の現れる時刻の列を触る。
- **議題**: 禁則の表を手で書くかクレート（UAX#14）か（クレートなら `Cargo` の類を触る・手書きを推す）／縦中横を自前で描く方式。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: M〜L（14〜18 タスク）。今は切らない。20 を超えそうなら、要件の段で「禁則とぶら下げ（折り返しの規則・`line_break`／`hanging_punctuation`）」と「縦中横と字の向き（配置の単位と描画・`text_combine_upright`／`\![text,combine-upright,…]`／`text_orientation`・縦書きの字形の観測点）」に切る。前者が先（触るのが配置の層だけで小さい）。
- 前提の状態: `layout.rs` の分割（`emo-text-file-split`・PR#217）と `shell-balloon`（PR#227）は着地済み＝満たす。列の上では `balloon-font-file` と `anchor-tag-canon`（働き）の後。
- 崩れた前提／古くなった位置:
  - 「`layout.rs` 977 行＝先に分割が要る」は済んだ（Scope の「`layout.rs` の分割」は外してよい）。折り返しの判定と文字の配置は **`layout_scan_glyph.rs` の `Scan::glyph`**（可視の打ち切り→保留の実体化→折り返し判定→遠辺の判定→配置）、行を閉じる仕上げは `layout_scan.rs` の `finish_line` ほか、入口と型は `layout.rs`。ぶら下げ「未実装」の注記は `layout.rs` のモジュール doc・`layout_scan.rs` の `layout_inner` の中・`region.rs`（折り返し基準と絶対上限の 2 値の説明）に在る。
  - 縦中横の塊を描く所は `viewbox_draw_render.rs` の `render_styled`／`line_layout_for`（分割前は `viewbox_draw.rs`）。字の向きは `draw.rs` の `DirectionRecipe` と `draw_line_store.rs` の行 TextLayout、計測は `draw_metrics.rs`。
  - **シェル内バルーンにも効くか**: 効く。箱の定義（`balloon.名前`ブレス）は `areka-emo-compose/src/boxes.rs` で普通のバルーンと同じ `balloon::parse` を通って `BalloonModel` になり、文字の層でも同じ `ResolvedBalloonText::resolve_with_background` を通る（`actor_box.rs` の `register_box`）。新しいキーは**バルーン定義ごとの値**（`ResolvedBalloonText` か、そこから引ける型）に載せること。`state.rs` の `TextLayerConfig`（ランタイム共通・今は `line_gap` だけ）に載せると、普通のバルーンと箱で別々の値を持てない。
  - 予約キー（`writing.rs` の `RESERVED_KEY_TEXT_ORIENTATION`・`RESERVED_KEY_TEXT_COMBINE_UPRIGHT`）は変わらず在る。
- 触るファイル（並走の照合用）:
  - `crates/areka-emo-text/src/{layout_scan_glyph.rs, layout_scan.rs, layout.rs, wrap.rs, segment.rs}`（禁則・ぶら下げ・縦中横の単位）
  - `crates/areka-emo-text/src/region.rs`（ぶら下げの注記と上限）
  - 新規（禁則の表＝新しいファイル）＋`crates/areka-emo-text/src/lib.rs`（新しいファイルの登録）
  - `crates/areka-emo-text/src/{writing.rs, actor.rs（ResolvedBalloonText）, state.rs（`CueCommand::Custom` の腕で `\![text,combine-upright,…]`・現れる時刻の列）}`
  - `crates/areka-emo-text/src/{draw.rs, draw_line_store.rs, draw_metrics.rs, viewbox_draw_render.rs}`（字の向き・縦中横の塊の描画）
  - `crates/areka-parsers/src/balloon/{model.rs, parse.rs}`（4 つのキー・`model.rs` は 774 行）
  - `doc/COMPAT_ARCHITECTURE.md` §8
- 議題（答えで作業が変わるものだけ）: 棚卸⑳の 2 件のまま（禁則の表を手で書くか UAX#14 のクレートか／縦中横を自前で描く方式）。
- 見つけた穴: なし。
