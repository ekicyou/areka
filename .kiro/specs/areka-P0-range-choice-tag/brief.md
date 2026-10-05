# Brief: areka-P0-range-choice-tag

> 2026-10-05 `/kiro-discovery` で起票。「バルーンのリンクと OS の連携」の 7 本の 1 本（roadmap の同名の節が分け方の正本）。開発者裁定（同日・議題 4）＝`anchor-tag-canon` に足さず、別の spec にして `anchor-tag-canon` の後に作る。

## Problem

- **利用者**: 文字の 1 行より大きな範囲（複数行・画像のバナー）を 1 つの選択肢にしたいゴースト作者。
- 正典の `\__q[ID,...]…\__q`（ukadoc）＝「\__qがくるまでの範囲を選択肢とする。ID仕様は、Onやscript:で始まるものが特別扱いされる点や、ID引数の扱いまで含めて\qと同じ」「改行、\_l[50,50]カーソル移動も可能で、画像を貼れば画像が選択肢になる」。
  - 例: `\__q[script:\j[https://example.com/]]\_b[banner.png,inline]\__q`＝バナーを押すとサイトが開く。
  - 例: `\__q[OnTest,ref]今日のおすすめ\n（クリックで詳しく）\__q`＝2 行のどこを押しても同じ選択肢。
- areka では `\__q` を読まず、範囲の中身はただの文字として出て、押しても何も起きない。

## Current State

- `\__q[…]` と閉じの `\__q` は `crates/areka-parsers/src/sakura/decode.rs` の素通しの腕で `Raw` になり、`compile.rs` が捨てる。
- 網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` の `\__q` の行は「無い」・引受先なし。
- `anchor-tag-canon` の brief に「`\__q` の仕組みは実装済み」とあったが誤り（本 discovery で同 brief を直した）。
- 選択肢の当たりの行は `crates/areka/src/input_events/balloon.rs` の `hit_choice_row`（1 行の選択肢）。範囲の当たりは `anchor-tag-canon` が作る。

## Desired Outcome

- `\__q[ID,…]…\__q` の範囲（複数行・カーソル移動・画像を含む）全体が 1 つの選択肢として押せ、`\q` と同じ選択肢の待ち・タイムアウト・`OnChoiceSelect(Ex)`・`On` で始まる ID・`script:` が動く。普通のバルーンとシェルの中の箱の両方で。
- 自動改行が行われる（ukadoc「\q[]タグと異なり、自動改行も行われる」）。
- 選択中の見た目は既定の 1 種類（選択肢の飾り分けは `choice-marker-styling`）。

## Approach

- `anchor-tag-canon` が作る「範囲を押せるようにする」仕組み（範囲の当たりの行の集合）をそのまま使う。違うのは結論が選択肢（待ちの柵・タイムアウト・選択肢のイベント）であること。
- `script:` は `choice-script-prefix` の道を通す。

## Scope

- **In**: `\__q` の読み込み・コンパイル・範囲の当たり・選択とホバーの見た目・選択肢の待ちとイベント・箱の中・決定論テスト・台帳の行。
- **Out**:
  - 選択肢の飾り分け（`choice-marker-styling`）。
  - 右クリックのコピー・ホバーの説明（`link-context-copy`・`balloon-link-hover`。本 spec の後に作るので範囲の選択肢もそこで拾う）。

## Boundary Candidates

- 読み込みとコンパイル（範囲の開きと閉じ）。
- 範囲の当たりと選択肢の結論（input_events）。

## Out of Boundary

- アンカー `\_a`（`anchor-tag-canon`）。

## Upstream / Downstream

- **Upstream**: `anchor-tag-canon`（範囲の仕組み）・`choice-script-prefix`（`script:`）。
- **Downstream**: `link-context-copy`・`balloon-link-hover`。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `anchor-tag-canon`（同じ範囲の仕組み・brief を直した）・`choice-marker-styling`。

## Constraints

- 文字とバルーンの列と台本のコンパイルの列（`anchor-tag-canon` の次）。
- 画像の選択肢は `\_b[…,inline]` の対応の状況に依る（要件の段で確かめる）。
- 段: 優先（バルーン関係）。規模の見込み M（10〜14）。
