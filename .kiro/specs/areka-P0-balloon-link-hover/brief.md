# Brief: areka-P0-balloon-link-hover

> 2026-10-05 `/kiro-discovery` で起票（開発者「マウスホバーって出せる？ URL とか」）。「バルーンのリンクと OS の連携」の 7 本の 1 本（roadmap の同名の節が分け方の正本）。
> 開発者裁定（同日・議題 3）＝**案 A**: 正典の `balloon_tooltip`・`OnChoiceHover`・`OnAnchorHover` を実装し、ゴーストが何も返さないときだけ areka が行き先を出す。土台は `wintf-tooltip`（開発者指示「ベース実装 → 本議題の実装」）。

## Problem

- **利用者**: リンクを押す前に行き先を確かめたい利用者と、選択肢に説明を添えたいゴースト作者。
- 正典（ukadoc）:
  - SHIORI リソース `balloon_tooltip`＝「バルーン選択肢上のツールチップ内容の取得。選択肢にマウスカーソルが乗った際に通知」。Reference0＝選択肢のテキスト・Reference1＝ID・Reference*＝拡張情報。アンカーの分は書かれていない。
  - イベント `OnChoiceHover`（選択肢上で静止）・`OnAnchorHover`（`\_a` ジャンパ上で静止）。Reference0＝テキスト・Reference1＝ID・Reference*＝拡張情報。
- areka ではどれも未実装で、選択肢にマウスを乗せても行が光るだけ。

## Current State

- `balloon_tooltip` は語の表だけ（`crates/areka-sylphya/src/vocab/shiori_resource.rs`）。網羅台帳 `doc/ukadoc-coverage/ledger/shiori.toml` で「語の表だけ」・引受先なし。
- `OnChoiceHover`・`OnAnchorHover` は同台帳で「無い」・引受先なし。
- バルーンのホバーは選択肢の行の光らせだけ（`crates/areka/src/input_events/balloon.rs` の `hover`）。止まったことの検出は無い（`wintf-tooltip` が作る）。

## Desired Outcome

選択肢・アンカー・範囲の選択肢にマウスが止まったら、次のようにする。

1. 選択肢なら、ゴーストに `balloon_tooltip` を尋ねる。返った文字があればツールチップに出す（ゴーストの言葉が優先）。
2. 返らなかった・アンカーだったら、`link-context-copy` の決め方の 1・2 段目で行き先を読み、あれば出す。表示されている文字の繰り返し（3 段目）は出さない。
3. どちらも無ければ何も出さない。
4. 止まったことを `OnChoiceHover`／`OnAnchorHover` でゴーストへ知らせる。

例:

- `\q[公式サイト,script:\j[https://example.com/]]` に止まり、ゴーストが何も返さない → `https://example.com/`。
- 同じ選択肢で、ゴーストが「ブラウザで公式サイトを開きます」と返す → その文字。
- `\_a[OnURL,https://example.com/]example.com\_a` → `https://example.com/`（アンカーはゴーストに尋ねない）。
- `\q[今日の天気,OnWeather,tokyo]` で何も返らない → 何も出さない。

待ち時間は OS の設定どおり（`wintf-tooltip`）。普通のバルーンとシェルの中の箱の両方。

## Approach

- `wintf-tooltip` の「動的な表示」の口を使う。止まった知らせ → `balloon_tooltip` を SHIORI へ尋ねる（答えは後から届く）→ 届いた答え、または既定の行き先を出す。
- `OnChoiceHover`／`OnAnchorHover` を選択肢の待ちの間に送ったとき、ゴーストが返したトークをどう扱うか（今の選択肢を流すか・待ちを保つか）は要件の議題。SSP の振る舞いは ukadoc で引き直す。

## Scope

- **In**: `balloon_tooltip` の問い合わせ・2 つのホバーのイベント・既定の行き先・選択肢・アンカー・`\__q`・箱の中・決定論テスト・台帳の 3 行。
- **Out**:
  - キャラクター窓のツールチップ（`shell-tooltip`）。
  - ツールチップの土台（`wintf-tooltip`）。

## Boundary Candidates

- 何を出すかの決め方（純粋・ゴーストの答え → 既定の行き先 → 出さない）。
- 配線（止まった知らせ・SHIORI への問い合わせ・イベント）。

## Out of Boundary

- 行き先の読み方の規則そのもの（`link-context-copy` が持つ）。

## Upstream / Downstream

- **Upstream**: `wintf-tooltip`・`link-context-copy`・`anchor-tag-canon`・`range-choice-tag`。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `anchor-tag-canon`（アンカーのホバーの光らせ）・`mcp-shiori-query`（SHIORI へ尋ねる口の近く）。

## Constraints

- 文字とバルーンの列と kanade の列（イベント・SHIORI リソースの問い合わせ）に掛かる。着手の前に両列の先頭と照合する。
- 段: 優先（バルーン関係）。規模の見込み M（8〜12）。
