# Brief: seriko-legacy-pattern-format

> 起票: 2026-10-30（`seriko-pattern-calls` の要件の議論の議題 1・開発者「承認」＝旧 SERIKO 1.x の書式を別の spec に分ける）。根拠は `seriko-pattern-calls` の brief の「2026-10-24 棚卸㉚の再測定」の分割の案と、同 spec の `requirements.md` の Out of scope。
> roadmap.md はこの起票では触らない（版の段と並びは次の棚卸で決める）。

## Problem

シェルを作る人と、そのシェルを使う利用者が困る。旧 SERIKO 1.x の書式（SERIKO/2.0 の `animation*.pattern*` より前の書き方）で書いたシェルのアニメーションが、areka では 1 つも動かない。今の読み手は `animation` で始まらない行を飛ばすので、旧書式の行はどの語も読まれない（記録も出ない）。

## Current State

- 読み手: `crates/areka-parsers/src/shell/decode.rs` の `decode_animations` は `animation` で始まる行だけを読む。旧書式の `*interval`・`*pattern*`・`*option` の行は飛ばす。`model.rs` の注記は旧 SERIKO 1.x の並びと語を挙げている。
- 書式の版の宣言（`surfaces.txt` の `descript` ブレスの `version`）は、シェルの読み手（`crates/areka-parsers/src/shell/` の本番のファイル）では読まれていない（2026-10-30 grep・テストの検体に `version,1` が出るだけ）。アニメーションの行の読み分けにも使われていない（`seriko-pattern-calls` の要件 2.7 は「版の宣言に関わらず `animation*.pattern*` を読む」を保つ）。
- 正典（ukadoc `descript_shell_surfaces`）の旧定義（逐語）:
  - `version,*`: 「0でSERIKO/1.x系書式（旧定義）、1でSERIKO/2.0書式」（https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#version%2C* ）。ukadoc の既定の欄は `0`。
  - `animation*.pattern*`: 「旧定義：*pattern*,サーフェスID,ウェイト,描画メソッド,X座標,Y座標」（https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#animation*.pattern*%2C%E6%8F%8F%E7%94%BB%E3%83%A1%E3%82%BD%E3%83%83%E3%83%89%2C%E3%82%B5%E3%83%BC%E3%83%95%E3%82%A7%E3%82%B9%E7%95%AA%E5%8F%B7%2C%E3%82%A6%E3%82%A7%E3%82%A4%E3%83%88%2CX%E5%BA%A7%E6%A8%99%2CY%E5%BA%A7%E6%A8%99 ）。新しい書式とは欄の順（描画メソッドとサーフェス番号・ウェイト）が違う。
  - `animation*.interval`: 「旧定義：*interval,インターバル」。
  - `animation*.option,exclusive`: 「旧定義：*option,exclusive」。
  - `alternativestart`: 「旧定義：alternativestart,[ID1,ID2...]」。

## Desired Outcome

旧 SERIKO 1.x の書式で書いたシェルのアニメーションが、新しい書式で書いたときと同じに動く。読めない行は記録なしに捨てない。

## Approach

旧書式の行を読み手（`lexer.rs`・`decode.rs`・`model.rs`）で読み、新しい書式と同じ `Pattern` などの型へ写す。下流（語の見分け・seriko の再生・合成）は新しい書式の行と区別しない。旧書式の行がどの版の宣言で効くか（`version,0` だけか・宣言なしも含むか・両方の書式が混ざったファイル）は、要件の段で ukadoc を引き、読めない点は SSP の MCP で測る。

## Scope

- **In**:
  - 旧書式の `*interval`・`*pattern*`・`*option` の行（と ukadoc が挙げる旧定義の全部の語）の読み取り。
  - 書式の版の宣言（`version`）との関係と、新旧が混ざったときの扱い。
  - 決定論のテスト。旧書式の検体が `vendors/sample_ghost` にあれば実機の 1 回。
- **Out**:
  - 描画メソッドの語の働きそのもの（重ね方＝`draw-methods-canon`・`draw-methods-blend`、アニメーションを呼ぶ 7 語と `move`＝`seriko-pattern-calls`）。旧書式で書かれていても、語の働きはそれぞれの spec の作りに乗る。
  - interval の抽選そのもの。

## Boundary Candidates

- 読み手の行の読み分け（旧書式の見分けと欄の並べ替え）。
- 版の宣言の解釈。

## Out of Boundary

- 語の意味と再生（下流は新旧を区別しない）。

## Upstream / Downstream

- **Upstream**: `seriko-pattern-calls`（読み手の同じファイル `lexer.rs`・`decode.rs`・`model.rs` に触れる＝直列。候補の並び `(ID1,ID2)` の読み方をそちらが決める）。
- **Downstream**: 旧書式のシェル全般。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `seriko-pattern-calls`（読み手のファイルで直列・同じウェーブに置かない）・`shell-mouse-cursor`（`areka-parsers/src/shell/mod.rs` に新しいファイルを足すとき）・`config-parse-diagnostics`。

## Constraints

- 正典は ukadoc。読めない点は SSP の MCP で測る。
- 決定論のテスト必達。
- 優先度の区分は C（ukadoc の拾い残し）の見込み。版の段は次の棚卸で決める。

## 2026-10-31 棚卸㉛の再測定（main 4068422f）
- **前提の変化**: 上流の ✅ `seriko-pattern-calls`（10-31・main #397）が着地し、読み手の同じファイル（`crates/areka-parsers/src/shell/decode.rs`・`model.rs`）を触り終えた＝直列の相手は済んだ。本文の現状は今も当たる: `decode.rs` 313 行目の `decode_animations` は `animation` で始まらない行を飛ばす（322 行目）・`model.rs` 184〜185 行目の注記は旧形式（SERIKO 1.x）を Non-Goals と書く・`version` を読むのは `surfacetable.rs` 66 行目（読み飛ばすだけ）で、`surfaces.txt` の読み手には無い。
- **検体**: ぱすた（`pasta-in-windows`）のシェルは `version,1` で、新しい書式の `animation*.pattern*` が 32 行・旧書式は 0 行。`vendors/sample_ghost` の書庫 10 本（`dot_sakura` は `version,1`・新しい書式 279 行）も、リポジトリのテストの検体も、旧書式の行は 0＝本文 Scope の「検体があれば実機の 1 回」は手元の検体では回せない（作った検体で回すか、決定論のテストだけにするかは要件の段で）。
- **触るファイル**: `crates/areka-parsers/src/shell/lexer.rs`（222）・`decode.rs`（623）・`model.rs`（303）と兄弟のテスト（新しいファイル）・網羅台帳 `assets.toml` の旧定義の行・`doc/COMPAT_ARCHITECTURE.md` §8。
- **規模**: S（5〜8 タスクの見込み＝旧定義の語 4 種〔`*interval`・`*pattern*`・`*option`・旧 `alternativestart`〕の読みと欄の並べ替え・`version` の解釈と新旧の混ざり・読めない行の記録・テスト）。
- **先に要るもの**: 働きの上では無い。同じウェーブに置かない相手（ファイル）: `script-small-tags`・`seriko-interval-combinations`（`shell/decode.rs`）・`config-parse-diagnostics`（`shell/decode.rs`・`shell/lexer.rs`）。1 本だけの席は取らない。ukadoc で読めない点は SSP の MCP で測る（打つ前に許し）。
- **優先度の区分**: C（ukadoc の拾い残し。起票は `seriko-pattern-calls` の要件の討議で分けることを開発者が承認したもの＝機能の依頼ではない）。版の推奨は**いつか**: v0.1.0 のゴールの検体（ぱすた）も手元の検体も旧書式を使わない＝版の門に当たらない。v0.2.0（wintf・dola の基幹）の中身でもなく、道具や測ることでもない＝版に縛らない列にも当たらない。
- **要件定義のモデル**: Opus（今ある読み手に行の読み分けを足し、新しい書式と同じ型へ写す形・新しい境界や並行の設計に当たらない）。
- **分割の案**: なし。
- **見つけた穴・古くなった記述**: roadmap の spec 台帳にこの spec の行が無い（brief を持つ数 166 と台帳の 165 が合わない件の正体）。
