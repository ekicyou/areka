# Brief: balloon-inline-image

> 起票: 2026-10-23（棚卸㉙・覚え書きの「バルーンの文中に画像を置く `\_b[…,inline]`」から格上げ）。
> 版の段は「いつか」・区分 C（今はタグが消えて画像が出ないだけで、ほかの文字は崩れない）。

## Problem

- **作者**: バルーンの文の中に小さな画像（顔のアイコン・絵文字の代わり・バナー）を 1 文字のように置けない。SSP では置ける。
- **ほかの spec**: `range-choice-tag` の brief の例 1（`\__q[script:\j[…]]\_b[banner.png,inline]\__q`＝バナーを押すとサイトが開く）は、これが無いと動かない。同 spec は画像を範囲の外に置いた。`element-clipping-option` も `\_b[...]` の `--clipping` を範囲の外に置いた。

## Current State

- **正典（ukadoc・2026-10-23 に MCP で引いた）**:
  - 「\_b[ファイルパス,inline]」: 「バルーンのテキスト行内に1文字分扱いとして画像を貼り付ける。[実行したゴーストのフォルダ]/ghost/masterが基準のフォルダとなる。」記述例は「左上のドットを透過色として」表示する。ファイル名の代わりに base64 や exe／dll のリソースも指定できる。
  - 「\_b[ファイルパス,inline,opaque]」: 「左上の色を透過しないで表示する。」
  - 「\_b[ファイルパス,inline,オプション,オプション...]」: 例 `--option=use_self_alpha`（ファイルの透過情報を使う）・`--clipping=100 30 150 90`（画像の一部だけを出す）。
  - 座標の形「\_b[ファイルパス,x,y]」（「画像は文字の下に表示される」）・「\_b[ファイルパス,x,y,opaque]」・「\_b[ファイルパス,x,y,オプション,オプション...]」（`--option=foreground` で文字の上）。URL とプラグインのフォルダの探し方は SSP だけ。
  - 「\c[char,数値]」: 「\_b[ファイルパス,inline]で貼り付けた画像は1文字扱いとなりクリアされる」（areka は `\c[char]` を作らないと決めている＝影響なし）。
- **areka**:
  - 網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml`: `inline` の 3 行（4144 行目〔オプションつき〕・4159 行目〔`opaque`〕は absent、4174 行目〔素の `inline`〕は alias）と座標の 3 行（4190・4205 行目は absent、4220 行目は alias）。6 行とも持ち主なし。4144 行目の注記: 「黙って壊れる（タグが消える）— decode_tag の既定の腕から decode_passthrough_tag へ落ちて Raw になり、compile の catch-all が cue を作らずに捨てる」（記録は debug! だけ）。
  - 台本の読み手は `crates/areka-parsers/src/sakura/decode.rs`（502 行）、台本から再生の指示を作る所は `crates/areka-sakura/src/compile.rs`（481 行）、再生の指示の種類は `crates/dola/src/cue/command.rs`。文字の並べと描画は emo-text（`crates/areka-emo-text/src/layout.rs` 502・`draw.rs` 761・`state.rs` 751・`lib.rs` 545）。文字の層が画像を読む道は今は無い（バルーンの絵を読むのは emo-present の `balloon.rs`）＝着手のときに確かめる。

## Desired Outcome

- `\_b[ファイルパス,inline]`・`\_b[…,inline,opaque]`・`\_b[…,inline,--option=use_self_alpha]`・`--clipping` で、画像が文の今の位置に 1 文字分として入り、後ろの文字はその右（縦書きなら下）へ続く。折り返し・行の高さ・選択肢の中（`\q`・`\__q`）でも崩れない。
- 基準のフォルダは「ゴーストの `ghost/master`」。読めない画像は記録を残して飛ばす（黙って消さない）。
- MCP の `dump_balloon` の絵に、文中の画像も写る（文字の面の中で描く）。
- 決定論のテストで「1 行の中の画像」「画像で折り返す」「行より背の高い画像」「選択肢の中の画像」「読めない画像」を固定する。

## Approach

- 画像は、文字の並べの中で「幅と高さを持つ 1 文字」として扱い、描くときに字の代わりに画像を置く。読み手は `\_b` の腕を足して形（`inline`／座標）とオプションを転記し、`compile.rs` が再生の指示を 1 種類足す。
- 行の高さと画像の縦の位置（下端を字の並びの線にそろえるか）は ukadoc が黙っているので、SSP で測ってから決める。
- 透過の扱いは areka の決まり（α 付きの画像を正しく使うのが原則・宣言の無いときも α を使う・抜き色は譲歩）に合わせ、正典の「左上の色を透過」との差は `doc/COMPAT_ARCHITECTURE.md` §8 に残す。

## Scope

- **In**: `inline` の 3 つの形（素・`opaque`・オプション）・`--option=use_self_alpha`・`--clipping`・`ghost/master` を基準にした読み・普通のバルーンとシェルの中の箱・選択肢の中・網羅台帳の 3 行（`inline`）・決定論のテスト・実機の確かめ。
- **Out**: base64・exe／dll のリソース・URL・プラグインのフォルダ（議題で入れる候補は base64 だけ）・動く画像の再生（議題）・`\c[char]`（作らない決まり）。
- **議題で決める**: 座標の形（`\_b[ファイルパス,x,y]` の 3 行）も入れるか。同じ「画像を読んでバルーンに置く」道で、違うのは置き場所（文字の下／`--option=foreground` で上）だけ。

## Boundary Candidates

- 台本の読み手と再生の指示（`sakura/decode.rs`・`areka-sakura/src/compile.rs`・dola の `cue/command.rs`）
- 文字の並べ（1 文字分の画像の幅と高さ）と描画（emo-text）
- 画像の読み込みと透過の扱い（どのクレートで読むかは要件の段で決める）

## Out of Boundary

- 文字の字形の並べの決まり（`text-typesetting`）と、選択肢の当たりの決まり（`range-choice-tag`・`choice-ranges-one-function`）は変えない。画像はその上に 1 文字として乗る。

## Upstream / Downstream

- **Upstream**: 働きの上の前提は無い。
- **基準のフォルダ**（2026-10-25 起票の `talk-script-metadata`・v0.2.0）: ファイルを指すタグの相対パスは、台本の付帯情報（実行したゴーストの置き場）と、その spec の解く関数 1 つで読む。この spec が先に着地するなら、受け手が自分で `GhostSlot` を引く形にしない（`talk-script-metadata` へ乗り換えやすい口に留める）。
- **Downstream**: `range-choice-tag`（例 1 のバナーの選択肢）。

## Existing Spec Touchpoints

- **Extends**: なし（新しい台本の指示）。
- **Adjacent**: `range-choice-tag`・`element-clipping-option`（`--clipping` の書き方の読みを共有できるか）・`talk-fast-forward`・`balloon-scroll-arrows`・`vertical-choice-hover-band`・`choice-ranges-one-function`（emo-text のファイルを分け合う）・`script-small-tags`・`stay-on-top-directives`（`sakura/decode.rs`・`compile.rs`）。

## Constraints

- 1 ファイル 1,000 行（emo-text の `draw.rs` 761・`state.rs` 751・`surface.rs` 995＝足す分は新しいファイルへ）。emo-text に新しいファイルを足すと `lib.rs` の一覧に載せる＝emo-text の「1 本だけの席」を使う。
- 透過は areka の決まりに従う（2026-10-05 の裁定「α 付きの画像を正しく使うのが原則・宣言の無いときは α を使う」）。
- 決定論のテストは必達。

## 2026-10-23 棚卸㉙の測り

- **触るファイル**: `crates/areka-parsers/src/sakura/{decode.rs 502, model.rs 172}`・`crates/areka-sakura/src/compile.rs`（481）・`crates/dola/src/cue/command.rs`（種類を足すと、網羅の match を持つ emo-text・`crates/areka-ghost/src/sink.rs`・seriko へ連鎖する）・emo-text の `layout.rs`・`draw.rs`・`state.rs`・`lib.rs`＋新しいファイル（画像の 1 文字の並べと描画）・画像を読む所・`crates/areka/src/mcp/check_script*`（台本の検査に `\_b` を知らせる）・台帳 `sakura-script.toml` の 3〜6 行。
- **規模**: M（10〜14 タスク。座標の形を入れると 13〜17）。
- **先に要るもの**: 働きの上では無い。同じウェーブに置けない相手（v0.1.0）: emo-text を触る `balloon-scroll-arrows`・`vertical-choice-hover-band`・`choice-ranges-one-function`・`default-balloon-selfmade`・`real-run-log-noise`、`sakura/decode.rs`・`compile.rs` を触る `script-small-tags`・`stay-on-top-directives`。`consumer_ledger.rs` の列（`\!` の受け手の表）・kanade の `schedule/events.rs`・`input_events/`・パーサの `balloon/` には触らない（`\_b` は `\!` ではない）。**1 本だけの席**: emo-text に新しいファイルを足す＝使う。SSP での測り（行の高さと縦の位置）は開発者の機械の SSP。
- **優先度の区分**: C。
- **要件定義のモデル**: Opus（再生の指示を 1 種類足して文字の層で描く形は先例が多い）。
- **議題の候補**: ⑴ 座標の形も入れるか。⑵ 行の高さと縦の位置（SSP で測る）。⑶ 正典の既定「左上の色を透過」と areka の「宣言の無いときは α」の差をどう扱うか（決まりどおり α・差を §8 に書く、を推奨）。⑷ 画像をどのクレートで読むか（emo-text の中／emo-present から渡す）。⑸ base64 を入れるか。⑹ 動く画像（APNG・WebP）を文中で動かすか。
