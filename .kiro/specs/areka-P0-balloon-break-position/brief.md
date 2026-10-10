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


---

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化:
  - `balloon-lifecycle-events`（10-08 着地）: `OnBalloonBreak` は Reference 3 個で送られ、Reference2 は空（`crates/areka-kanade/src/schedule/events.rs` の `on_balloon_break`・注記に本 spec の名）。送るかどうかの判断と「止めたトークの台本の控え」は新しい `schedule/balloon_events.rs` が持つ。網羅台帳 `shiori.toml` の `OnBalloonBreak` は `degraded`・持ち主は `balloon-lifecycle-events`＝本 spec が `implemented` に直す。
  - **位置の源は `mcp-author-tools`（10-08 着地）が作った**: 台本を読む入口 `areka_parsers::sakura::parse_noted`（`crates/areka-parsers/src/sakura/parse.rs`）が、命令ごとに台本の中のバイト範囲（`Read` の `span`）を返す。Current State の「`lex` が捨てている」は古い。残るのは運ぶ道だけ。
  - 運び方の見立て: 台本のコンパイル（`crates/areka-sakura/src/compile.rs`）が「合図の時刻 → 台本の位置」の表を作り、再生（`drive.rs`）が止めた時刻でその表を引けば、dola の合図の型に欄を足さずに済む（合図を組む多数の箇所に触れない）。
  - Constraints の未知 2（どの台本の中の位置か）は実物で答えが出ている: 環境変数の置き換えと `OnTranslate` は再生へ渡す前に済む（`crates/areka-ghost/src/translate_wiring.rs`）。再生が読む台本と、kanade が Reference0 に入れる控えは同じ文字列＝位置はその文字列の中で数える。
  - 完了の知らせ `TalkDone` を組んでいる所は 28 ファイル・62 か所（kanade 20・sakura 3・ghost 2・talk 2・areka 1 ファイル）。
- 触るファイル: `crates/areka-sakura/src/{compile.rs, drive.rs}`・`crates/areka-talk/src/lib.rs`・kanade の `schedule/{events.rs, balloon_events.rs}`・`TalkDone` を組む 28 ファイル（欄を足すなら）・台帳 `shiori.toml` の 1 行・`doc/COMPAT_ARCHITECTURE.md` §8。台本を読む段（`sakura/lexer.rs`・`decode.rs`）と emo-text には触らない。
- 規模: 9〜13 タスク（brief の 12〜18 から、位置の源の分が減った）。
- 分割の案: 切らない。
- 先に要るもの: 満たした（`balloon-lifecycle-events`）。ファイルの重なり: `compile.rs`・`drive.rs` は台本のコンパイルの列（`anchor-tag-canon`・`range-choice-tag`・`talk-fast-forward`・`sakura-time-directives`・`seriko-trigger-intervals`）、kanade の `events.rs` と `TalkDone` を組む検査のファイルは kanade の進行の列のほぼ全部と当たる。
- 優先度の区分: C（正典の Reference の持ち越し。起票は開発者の裁定「中断位置は今は作らない。起票を行え」）。
- 要件定義のモデル: Fable（数え方の単位・待機の途中で止めたときの位置＝正典のあいまいさが残る）。
- 議題: Constraints の未知 1・3 と、`TalkDone` の 62 か所の手直しの減らし方。未知 2 は上のとおり。
- 見つけた穴・古くなった記述: Current State の「27 ファイル・60 か所」「`lex` が捨てている」。Adjacent の `open-external-tags` は着地済み。
