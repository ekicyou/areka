# Brief: areka-P0-font-size-keywords

> 2026-10-10 棚卸㉓で、roadmap の覚え書き「正典語彙の孤児 1 件」（スタイルシートのキーワードの持ち主）から起票した。コードは「何の定義か」で指す。着手時に引き直す。

## Problem

台本の `\f[height,larger]` のように、文字の大きさをスタイルシートの語（`larger`・`small` など）で書くと、areka は語として受け取るだけで大きさを変えない。記録が 1 行残るだけで、画面の文字はそのまま。作者が大きくしたつもりの文字が、areka では大きくならない。

## Current State

- 正典（ukadoc「`\f[height,数値]`」）は、数・`+`／`-` の相対・百分率（記述例）・`default` に加えて、スタイルシートの大きさの指定も使えると 1 文で書くだけ。どの語が使えるか、どの語が何ピクセルになるかは書いていない（ukadoc MCP で「スタイルシート」を引いて当たるのはこの 1 件だけ）。
- 大きさの値の読み `parse_height`（`crates/areka-emo-text/src/look.rs`・802 行）は、9 語（`STYLESHEET_SIZE_KEYWORDS`＝`xx-small`〜`xx-large` の 7 語と `larger`・`smaller`）を `HeightSpec::Keyword` に読む。適用する `apply_height` は、この腕だけ大きさを変えずに印 `Note::StylesheetKeyword` を返す。
- 印を受けた側（`crates/areka-emo-text/src/state_decoration.rs` の `\f` の適用）が、「スタイルシートの大きさの語は語彙のみ——大きさを変えない」の記録を 1 台詞に 1 度出す。
- `check_script` も同じ印を見て `ignored`（受け取るが何もしない）を返す（`crates/areka/src/mcp/check_script_judge.rs` の `judge_font`）。
- この形は完了 `text-decoration-canon` の要件 7.7 が決めた（換算は areka の裁量で見送り）。`doc/COMPAT_ARCHITECTURE.md` §8 の行「スタイルシートの大きさキーワード」に「語彙のみ」と登記してある。
- 網羅台帳の `\f[height,数値]` の行（`doc/ukadoc-coverage/ledger/sakura-script.toml`）は、持ち主が `areka-P0-text-decoration-canon`・状態が `implemented` で、注記に「スタイルシートの大きさの語は語彙のみ」と書いてある。
- 相対の `+N`／`-N` は「そのとき効いている大きさ」、百分率は「既定の見た目の大きさ」を基準にする（同じ `apply_height`）。

## Desired Outcome

- 9 語が文字の大きさを変える。絶対の 7 語は決まった大きさ、`larger`・`smaller` はそのとき効いている大きさからの拡大・縮小。
- 換算の表が 1 か所にあり、`doc/COMPAT_ARCHITECTURE.md` §8 に areka の決まりとして載っている。
- 「語彙のみ」の記録と、`check_script` の `ignored` が出なくなる。

## Approach

`apply_height` の `HeightSpec::Keyword` の腕を、換算の表を引いて大きさを決める形に替える。結果が正の有限値にならないときの止め方は、同じ関数の出口の 1 か所の検査にそのまま乗る。印 `Note::StylesheetKeyword` は要らなくなるので外し、それを見ている 2 か所（文字の層の記録の腕・`check_script` の腕）を合わせる。

## Scope

- **In**: 9 語の換算の表・`apply_height` の腕・印を見ている 2 か所の追従・`COMPAT_ARCHITECTURE.md` §8 の行の書き直し・網羅台帳の `\f[height]` の行の注記の書き直し・決定論テスト（9 語それぞれ・重ねて効くか・0 以下にならないこと）。
- **Out**: 9 語のほかの書き方（`12pt`・`1.5em`・`large` の大文字など）・バルーンの descript の `font.height` に語を書くこと・`\f` のほかのキー。

## Boundary Candidates

- 大きさの読みと適用（`crates/areka-emo-text/src/look.rs`）。
- 印を見ている所（`state_decoration.rs` の記録の腕・`check_script_judge.rs` の `judge_font`）。

## Out of Boundary

- 文字を並べる処理・描く処理（大きさが変わるだけで、今の `\f[height,数]` と同じ道を通る）。
- 相対・百分率・`default`・`disable` の意味（変えない）。

## Upstream / Downstream

- **Upstream**: なし。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし（`text-decoration-canon` は完了済みで消化できない＝新しい spec）。
- **Adjacent**: 文字とバルーンの列で `look.rs` に触る `choice-marker-styling`・`anchor-style-canon`・`anchor-tag-canon`・`text-align-shadow-canon`・`balloon-font-file`。`check_script_judge.rs` に触る `mcp-strict-errors`・`script-impact-tiers`・`check-script-arg-checks`。

## Constraints

- 換算は ukadoc が黙っている所を areka が決めるもの。要件の冒頭にそう書き、`COMPAT_ARCHITECTURE.md` §8 に登記する（SSP の実測は取らない方針）。
- 語は今と同じく小文字の完全一致だけ。
- 印を固定している既存のテスト（`look_font_tag_value_tests.rs` の印の期待・`state_decoration_reset_tests.rs` が腕の字面を見ている所）は、同じ変更で書き直す。

## 2026-10-10 棚卸㉓の測定（main `ee3af616`）

- **触るファイル**: `crates/areka-emo-text/src/look.rs`（802）・`look_font_tag_value_tests.rs`（431）・`state_decoration.rs`（記録の腕）・`state_decoration_reset_tests.rs`（腕の字面を見ている 1 か所）・`crates/areka/src/mcp/check_script_judge.rs`（196・`judge_font` の印の並び）・`doc/COMPAT_ARCHITECTURE.md` §8 の 1 行・`doc/ukadoc-coverage/ledger/sakura-script.toml` の `\f[height]` の行の注記。
- **規模**: S（3〜5 タスク）。
- **先に要るもの**: なし。同じウェーブに置けない相手は Adjacent のとおり（`look.rs`・`state_decoration.rs`・`check_script_judge.rs`）。
- **優先度の区分**: C（ukadoc の拾い残し。今も記録が出て `check_script` も知らせるので、黙って壊れてはいない）。
- **要件定義のモデル**: Fable（正典が黙っている換算を areka が決める＝開発者の判断）。
- **議題**:
  1. 換算の表。案 A: CSS の決まりに合わせる（`medium` を既定の見た目の大きさとし、絶対の 7 語は決まった比で上下・`larger`／`smaller` はそのとき効いている大きさの 1.2 倍と 1/1.2）。案 B: `medium`＝既定の大きさだけ決め、ほかは 1 段ごとに同じ比。どちらも areka の独自の決まりになる。
  2. 絶対の 7 語の基準を「既定の見た目の大きさ」（百分率と同じ基準）に置くか、決まったピクセル数に置くか。バルーンごとに既定の大きさが違うので、前者が素直。
- **網羅台帳の行の扱い**: `\f[height,数値]` の行は持ち主 `areka-P0-text-decoration-canon`・状態 `implemented` のまま変えない（完了した持ち主を書き換えない）。注記の「語彙のみ」の 1 文だけを本 spec が実物に合わせて書き直す。
- **覚え書きとの違い**: 覚え書きは「`look.rs` だけ」と書くが、印を見ている所が文字の層と `check_script` に 1 か所ずつあり、文書も 2 つ直す。規模は S のまま。
- **ukadoc の照合**: ukadoc「`\f[height,数値]`」（さくらスクリプトの一覧）を ukadoc MCP で確かめた。
