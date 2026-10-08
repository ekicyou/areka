# Brief: areka-P0-balloon-break-position

> 起票: 2026-10-05（`areka-P0-balloon-lifecycle-events` の要件の討議 議題 1。開発者裁定「中断位置は今は作らない。起票を行え」）。`balloon-lifecycle-events` は `OnBalloonBreak` の Reference2 を空で送り、互換対応表に縮退として記録する。本 spec はその空を埋める。

## Problem

ゴーストの作者は、利用者がどこで話を止めたかを `OnBalloonBreak` の Reference2（中断位置）で受け取れない。正典（ukadoc `list_shiori_event.html#OnBalloonBreak`）では Reference2 は「スクリプト先頭からの文字数。さくらスクリプトのタグも含めて数える」。止められた箇所から話を続ける・言い直す辞書は、areka では位置を知る手段が無い。

## Current State

2026-10-05 時点（main `44fc0a61`・`balloon-lifecycle-events` のギャップ分析 §4 の A）:

- 台本の中の位置を持ち運ぶ仕組みはどこにも無い。
  - 字句の走査の本体（`crates/areka-parsers/src/sakura/lexer.rs` の `scan`）は字句ごとにバイトの区間を渡すが、`lex` が捨てている。
  - `Instruction`・台本のコンパイル（`crates/areka-sakura/src/compile.rs`）・dola の cue・再生（`crates/areka-sakura/src/drive.rs`）・完了の知らせ `areka_talk::TalkDone`（`crates/areka-talk/src/lib.rs` の定義＝`talk_id`・`reason`・`quit_reserved` の 3 欄）のどれにも位置が無い。
- `TalkDone {` を組んでいる所は 27 ファイル・60 か所（kanade 19・sakura 3・ghost 2・talk 2・areka 1）。欄を足すなら全部に手が入る。
- 送る側は `balloon-lifecycle-events` が作る（kanade の中断の完了の腕で `OnBalloonBreak` を組み立て、Reference2 は空）。本 spec は「位置を完了の知らせまで通す」と「その値を Reference2 に入れる」だけを足す。

## Desired Outcome

利用者の中断で `OnBalloonBreak` が送られるとき、Reference2 に正典どおりの中断位置（台本の先頭からの文字数・タグも数える）が入る。互換対応表と網羅の台帳の縮退の記録が外れ、`OnBalloonBreak` の行が実装済みになる。

## Approach

字句の段で捨てている区間を残し、命令 → cue → 再生 → 完了の知らせへ「いま流している命令が台本のどこから来たか」を通す。中断の時点で最後に発火した（または進行中の）命令の位置を完了の知らせに載せ、kanade が Reference2 に入れる。詳しい方式は要件・設計で決める。

## Scope

- **In**: 台本の中の位置を字句から完了の知らせまで通すこと・`OnBalloonBreak` の Reference2 に入れること・縮退の記録を外すこと・決定論のテスト。
- **Out**: `OnBalloonBreak` を送る時機と Reference0・Reference1（`balloon-lifecycle-events` が持つ）・中断の操作そのもの（完了 `balloon-break`）・ほかのイベントへの位置の利用。

## Boundary Candidates

- 位置の源（字句・命令に区間を残す）／位置の運搬（cue・再生・完了の知らせ）／Reference2 への写し（kanade）。

## Out of Boundary

- 位置の数え方以外の正典の残り（`OnBalloonBreak` の他の Reference・`OnBalloonClose`・`OnBalloonTimeout`）。

## Upstream / Downstream

- **Upstream**: `areka-P0-balloon-lifecycle-events`（**先行必須**＝`OnBalloonBreak` の送り口と Reference2 の空の口）。
- **Downstream**: 中断位置を使う辞書（実ゴーストの適合）。

## Existing Spec Touchpoints

- **Extends**: `areka-P0-balloon-lifecycle-events`（Reference2 の縮退を埋める）。
- **Adjacent**: 台本のコンパイルの列（`open-external-tags`・`anchor-tag-canon`・`range-choice-tag`・`talk-fast-forward`・`sakura-time-directives` が `compile.rs`・`sakura/decode.rs` を触る）＝同時に走らせない。kanade の列（`schedule/`）。

## Constraints

- 要件で決める未知が 3 つある:
  1. 数え方：正典は「文字数」だが、走査の区間はバイト。UTF-16 の単位で数えるか、Unicode の文字で数えるか。
  2. どの台本の中の位置か：`OnTranslate` で書き換えた後の台本（`balloon-lifecycle-events` が Reference0 に入れるもの）か。`%` の環境変数の置き換え（`lexer.rs` の `substitute_system_vars`）の前か後か。
  3. 待機（`\w` など）の途中で止めたとき、位置はその待機タグの前か後か。
- 規模の見込み: M〜L（12〜18 タスク）。`TalkDone` の 60 か所は、欄を `Option` にして既定値の組み立てを足すなどで機械的な手直しを減らせるかを設計で詰める。
- 1 ファイル 1,000 行未満・決定論のテスト必達・時刻ではなく位置なので丸めの問題は無い。
