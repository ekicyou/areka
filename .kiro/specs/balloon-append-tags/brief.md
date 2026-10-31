# Brief: balloon-append-tags

> 2026-10-31 `balloon-scroll-arrows` の完了の棚卸で起票（`/kiro-discovery`）。同 spec のタスク 8.2 の実機の走りで台本に `\C` を書いたところ、areka の `check_script` が `unknown_tag` と答えた。開発者の方針「実機で areka の未対応のためにうまくいかなかった件は、範囲外でもすべて起票」に従う。網羅台帳の同じ群「バルーンの追記と選択肢のタイムアウト抑止」の残り 1 件 `\*` も、持ち主が居ないので同じ spec に入れる。
> 起票の理由（`kiro-complete` 冒頭ステップ手順 3 の条件）: 規模が spec 1 本に相当する（バルーンを消さずに前のトークへ書き足す新しい道が要る）。

## Problem

- **ゴーストの作者**: 前のトークへ書き足すつもりで `\C` を書いても、areka ではバルーンがいったん空になってから文が出る。里々・YAYA の作例にも出てくるタグで、SSP で書き足しを前提に組んだ会話が areka では途切れて見える。
- **ゴーストの作者**: 選択肢の待ち時間を止める `\*` が効かず、選択肢が待ち時間で消える。作者の意図と逆に見える。

## Current State

- 網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml`:
  - `\C`（`ukadoc:list_sakura_script:_5cC:1`）: `status="absent"`・`owner=""`。壊れ方は「黙って壊れる」。読み手 `decode_bare` の既定の腕から `decode_passthrough_bare` へ落ちて `Raw` になり、台本の組み立て（`areka-sakura` の `compile.rs`）の残りの腕が cue を作らずに捨てる。組み立てが台本の先頭へ前置きする `ClearAll` でバルーンが空になってから表示される。記録は `debug!` で出る。
  - `\*`（`ukadoc:list_sakura_script:_5c_2a:1`）: `status="absent"`・`owner=""`。同じく素通しになる。
- 報告 `doc/ukadoc-coverage/briefing-sakura-script.md` の群「バルーンの追記と選択肢のタイムアウト抑止（2 件）」が、成立させる最小の基盤を「バルーンを消さずに書き足す道と、選択肢の待ち時間を止める道」と書いている。
- areka の `check_script`（MCP）は `\C` を `unknown_tag` と答える（2026-10-31・`balloon-scroll-arrows` タスク 8.2 の実機の走りの記録＝同 spec の `research.md`）。

## Desired Outcome

- `\C` を含む台本は、前のトークのバルーンの中身を消さず、その後ろへ書き足して表示する（ukadoc の定義どおり。細部は要件の段で ukadoc から引き直し、ukadoc で読めない点は SSP の MCP で測る）。
- `\*` を含む台本の選択肢は、待ち時間で消えない。
- 台帳の 2 行が `implemented`・持ち主＝本 spec になる。

## Approach

- 読み手（`areka-parsers` の `sakura/decode.rs`）に 2 つのタグの腕を足し、台本の組み立て（`areka-sakura` の `compile.rs`）で先頭の `ClearAll` を `\C` のときに置かない形にする。選択肢の待ち時間は、今の時間切れの判断の側で止める印を読む。直し方の細部は要件・設計の段で決める。

## Scope
- **In**: `\C` の書き足し・`\*` の選択肢の待ち時間の抑止・台帳の 2 行・`doc/COMPAT_ARCHITECTURE.md` §8 の登記。
- **Out**: バルーンの部分消去（`\c[char,…]`・`\c[line,…]`＝実装しない裁定・記憶 `areka-partial-clear-tags-not-implemented-bitmap-vs-glyph`）。手動の送り（`balloon-scroll-arrows` が持つ）。

## Boundary Candidates
- 読み手と組み立て（台本の構造）
- バルーンの中身を保つ側（文字の層の消去の扱い）
- 選択肢の時間切れの判断

## Out of Boundary
- 選択肢のマーカーの描画（`\![*]`＝`script-small-tags`）。
- 時刻の指令（`\_q` など＝`sakura-time-directives`）。

## Upstream / Downstream
- **Upstream**: `areka-parsers`（読み手）・`areka-sakura`（組み立て）・emo-text（文字の層の消去）。
- **Downstream**: 書き足しを前提に組んだ里々・YAYA のゴースト。

## Existing Spec Touchpoints
- **Extends**: なし
- **Adjacent**: `script-small-tags`（同じ読み手の `decode.rs` を触る・`\![*]`）・`sakura-time-directives`・`balloon-canon-residue`（バルーンの正典の残り）。

## Constraints
- 意味論は ukadoc から輸入する。ukadoc で読めない点は議題の前に SSP の MCP で測る（記憶 `no-ssp-measurement-import-semantics-from-ukadoc`）。
- 優先度の区分: C（ukadoc の拾い残し）。版の置き場は次の棚卸で決める。

## 2026-10-31 棚卸㉛の再測定（main 4068422f）
- **前提の変化**: 起票（10-31・`e115e530`）の後に `stay-on-top-directives`（main #399）が同じ読み手 `crates/areka-parsers/src/sakura/decode.rs`（510）に `\v` の腕を足した（素の 1 文字のタグの腕の新しい先例）。本文の現状は今も当たる: 台帳 `sakura-script.toml` の `\C`（545 行目の項）・`\*`（3886 行目の項）はどちらも `status="absent"`・`owner=""`。`decode.rs` 226 行目の `decode_bare` の既定の腕から 486 行目の `decode_passthrough_bare` へ落ちる。組み立て `crates/areka-sakura/src/compile.rs`（625）の 471 行目が、内容のある台本の先頭へ `ClearAll` を置く。
- **検体**: ぱすた（`pasta-in-windows` の辞書と Lua）と `dot_sakura` は `\C`・`\*` を使わない（`dot_sakura` の `\*` の 1 件は文字の逃がしの並びで、タグではない）。`\C` を書くのは YAYA の雛形の系統＝`emily4` 18 か所・`claudia` 12・`konnoyayame` 2（説明の行を含む数）。`\*` を書く検体は無い。
- **触るファイル**: `crates/areka-parsers/src/sakura/decode.rs`・`model.rs`（`\*` の字句が要るなら `lexer.rs`）と兄弟のテスト・`crates/areka-sakura/src/compile.rs`（`\C` のとき `ClearAll` を置かない形）・文字の層の消去の側（`areka-emo-text`・場所は設計で）・選択肢の時間切れ `crates/areka-kanade/src/schedule/choice.rs`（376）・台帳の 2 行・`doc/COMPAT_ARCHITECTURE.md` §8。
- **規模**: S〜M（6〜9 タスクの見込み・見積もりは要件の段）。
- **先に要るもの**: 働きの上では無い。同じウェーブに置かない相手（ファイル）: `script-small-tags`（`sakura/decode.rs`・`compile.rs`・`schedule/choice.rs`）・`seriko-script-triggers`（`sakura/decode.rs`・`compile.rs`）・`balloon-break-position`・`talk-fast-forward`（`compile.rs`）・`ukadoc-catalog-refresh`（`sakura/decode.rs`）・`anchor-style-canon`・`choice-ranges-one-function`・`range-choice-tag`・`text-ruby`・`text-typesetting`・`emo-text-line-gap`（`schedule/choice.rs`）。ukadoc で読めない点は SSP の MCP で測る（打つ前に許し）。
- **優先度の区分**: C のまま（ukadoc の拾い残し）。版の推奨は **v0.3.0**（小さく早く出す版）: v0.1.0 のゴールの検体（ぱすた・dot_sakura）は使わない＝v0.1.0 には入れない。v0.2.0 は wintf・dola の基幹だけ。一方で `\C` は YAYA の雛形の系統（SSP に付く `emily4` を含む）が広く書くので、いつかに置くと、よく使うゴーストの会話が途切れて見えるまま残る。v0.3.0 を軽く保ちたいならいつかのままでもよい（開発者の決め）。
- **要件定義のモデル**: Opus（今ある読み手・組み立て・選択肢の時間切れに腕と印を足す形・新しい境界や並行の設計に当たらない）。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**: 本文に規模の見積もりが無い（上で 6〜9 と置いた）。
