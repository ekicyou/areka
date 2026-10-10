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


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 文字まわりの直列の列（`text-typesetting` の後・同じ `layout` 系）。棚卸⑳では個別の再測定をしていない＝着手のときに照合する。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: M〜L（12〜16 タスク）。切らない。
- 前提の状態: `text-typesetting`（同じ配置の層・禁則の単位）が**未着手**＝未。`emo-text-file-split`・`shell-balloon` は着地済み。
- 崩れた前提／古くなった位置:
  - `TextItem` の定義は `state.rs`（`pub enum TextItem`）。配置の本体は分割で `layout_scan.rs`（`layout_inner` の駆動・`Scan`・`finish_line`）と `layout_scan_glyph.rs`（`Scan::glyph`＝親文字の単位の折り返し判定はここ）へ移った。描画は `viewbox_draw_render.rs`。
  - **`line_gap` は今ランタイム共通**: 行送りの調整値は `state.rs` の `TextLayerConfig`（欄は `line_gap` だけ・既定 2.0）にあり、行送りは `TextLayerConfig::line_pitch`、計測側は `draw_metrics.rs` の `line_pitch`、選択肢の強調の帯は `choice.rs` が `metrics.line_pitch` を読む。バルーンのキー `line_height`／`letter_spacing` は**バルーン定義ごと**の値なので、ランタイム共通の `TextLayerConfig` ではなく `ResolvedBalloonText`（`actor.rs`）の側に置く必要がある。そうしないと普通のバルーンとシェル内バルーンの箱（同じ `ResolvedBalloonText::resolve_with_background` を通る・`actor_box.rs` の `register_box`）で別々の値を持てない。行送りの式の 1 点（`line_pitch`）の持ち主が変わる＝`draw_metrics.rs`・`choice.rs` の帯まで追随する。
  - 箱の定義は `areka-emo-compose/src/boxes.rs` で同じ `balloon::parse` を通る＝キーを足せば `balloon.名前`ブレスにも書ける（参考ゴーストの `1.9`・`0.08em` は箱に書く想定）。
- 触るファイル（並走の照合用）:
  - `crates/areka-emo-text/src/{state.rs（`TextItem`・`TextLayerConfig`・`CueCommand::Custom` の腕で `\![text,ruby,…]`）, layout.rs, layout_scan.rs, layout_scan_glyph.rs}`
  - `crates/areka-emo-text/src/{actor.rs（ResolvedBalloonText）, draw_metrics.rs, choice.rs, viewbox_draw_render.rs, viewbox_diff.rs}`（行送りの持ち主・ルビの描画と描き直しの範囲）
  - `crates/areka-parsers/src/balloon/{model.rs, parse.rs}`（`line_height`・`letter_spacing`）
  - 新規（ルビの配置）＋`crates/areka-emo-text/src/lib.rs`
  - `doc/COMPAT_ARCHITECTURE.md` §8
- 議題（答えで作業が変わるものだけ）: `line_height` を入れたとき、選択肢の強調の帯（`choice.rs` の `highlight_band_extent`）を本文の高さに合わせるか行送り全体に合わせるか（ルビが行の間隔の内側に入るので、帯がルビに重なるかが変わる）。
- 見つけた穴: なし。


---

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: M〜L（12〜16 タスク）のまま。切らない。
- 前提の状態: `text-typesetting`（同じ配置の層・禁則の単位）は今も未着手＝未。列の上では `balloon-markers` の後。C3 で文字の層に入ったのは `shell-balloon-frame-align`（`actor_box.rs`・`actor.rs` の注記）だけで、配置・描画・台本の受け取りの側（`layout*.rs`・`state.rs`・`draw_metrics.rs`・`choice.rs`・`viewbox_*.rs`・`areka-parsers/src/balloon/`・`areka-sakura`）は 1 行も動いていない。
- 崩れた前提／古くなった位置:
  - 棚卸㉑の位置は全部そのまま当たる（`state.rs` の `pub enum TextItem`・`TextLayerConfig` と `line_pitch`、`draw_metrics.rs` の `line_pitch`、`choice.rs` の `highlight_band_extent`、`actor.rs` の `ResolvedBalloonText`、`actor_box.rs` の `register_box` が `ResolvedBalloonText::resolve_with_background` を呼ぶ所）。
  - **棚卸㉑の一覧に抜けていた行送りの 1 点**: 配置の側の行送りは `layout_styled.rs` の `line_pitch_of`（`metrics.line_pitch` を呼ぶ）を通り、`layout_scan.rs`・`layout_scan_glyph.rs` がそれを読む（要件 7.8「行送りの式は 1 点だけ」）。`line_height` はこの 1 点と計測側の `line_pitch` を一緒に替える。
  - 箱の置き場所は `shell-balloon-frame-align` で「シェルの窓がいま表示している絵の番号」から決まるようになった（`sync_boxes` の引数が絵の番号の組へ）。箱の定義の読み（`register_box` が `ResolvedBalloonText` を解く所）は変わらない＝本 spec のキーの載せ場所には響かない。
- 触るファイル（並走の照合用）:
  - `crates/areka-emo-text/src/{state.rs, layout.rs, layout_styled.rs, layout_scan.rs, layout_scan_glyph.rs, draw_metrics.rs, choice.rs, actor.rs, viewbox_draw_render.rs, viewbox_diff.rs, lib.rs}`＋新規（ルビの配置）
  - `crates/areka-parsers/src/balloon/{model.rs, parse.rs}`・`doc/COMPAT_ARCHITECTURE.md` §8
- 議題: 棚卸㉑のまま（強調の帯を本文の高さに合わせるか行送り全体に合わせるか）。
- 見つけた穴: なし。


---

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化:
  - `budoux-reveal-reflow`（10-06 着地）が文字の層に「先読み」を足した（`crates/areka-emo-text/src/lookahead.rs`）。台本の全文を先に求め、届いた内容がその先頭と一致するときだけ全文で配置する。新しい項目（ルビつきの親文字）を `state.rs` の `TextItem` に足すと、先読みも同じ項目を扱う: 空回しは同じ `apply_cue` を通るので内容は自動で入るが、「その区間に何かを足した合図か」の見分け（`lookahead.rs` の中の合図の種類の並び）と、「届いた内容が全文の先頭と一致するか」の比べにルビを入れる。入れ忘れると、黙って修正前の動き（届いた字だけで区切る）へ落ちる。
  - 全文が先に分かるので、「親文字の単位の中で折り返さない」は全文で先に決められる（`text-typesetting` が 1 字ずつの折り返しでも全文を使う形に広げていれば、それに乗る）。
  - `mcp-author-tools`（10-08 着地）で、台本の検査が `\!` の受け取り手の表に無い名前を「知らない命令」と答える。`\![text,ruby,…]` は表（`crates/areka/src/emo2_boot/consumer_ledger.rs`・943 行）に 1 行と、一致の検査（`consumer_ledger_agreement_tests.rs`）に見本が要る（`text-typesetting` が `text` の行を先に作っていれば、選び分けの語を 1 つ足すだけ）。
  - 配置・計測・描画・読み手の側（`layout_styled.rs` の `line_pitch_of`・`draw_metrics.rs`・`choice.rs`・`viewbox_*.rs`・`crates/areka-parsers/src/balloon/`）は C4 で無変更。`state.rs`・`actor.rs` には先読みの欄と「空回し」の印が足されただけで、`TextItem`・`TextLayerConfig`・`ResolvedBalloonText` の位置は棚卸㉒のまま当たる。
- 触るファイル: 棚卸㉒の一覧に、emo-text の `lookahead.rs` と受け取り手の表の 2 本を足す。ルビの配置は新しいファイル＝**emo-text にファイルを足す**（`lib.rs` の席を使う。純粋の数は今 73）。
- 規模: 13〜17 タスク（棚卸㉒は 12〜16。先読みの扱いで 1 増えた）。
- 分割の案: 切らない。
- 先に要るもの: `text-typesetting`（禁則の単位・全文の使い分け）＝未着手。列の順は `balloon-markers` の後。
- 優先度の区分: A（roadmap「シェル内バルーン」節の開発者の確定 7「ルビ」）。
- 要件定義のモデル: Fable（選択肢の強調の帯は開発者に決めてもらう分かれ目・行送りの持ち主が替わる・先読みとの噛み合わせ）。
- 議題: 棚卸㉑のまま（強調の帯を本文の高さに合わせるか行送り全体に合わせるか）。
- 見つけた穴・古くなった記述: なし。
