# Brief: areka-P0-sakura-embed-directive

> 2026-10-04 棚卸㉑で `areka-P0-property-query-channels` から切り出し（開発者「負荷が高すぎる仕様は分割を検討せよ」）。`sakura-time-directives` の C 群（実行時に長さが決まる待ち）から `\![embed]` だけをこちらへ移した。

## Problem

ゴーストの作者は `\![embed,イベント名,r0,r1,…]` で、台本の再生の途中にイベントを起こし、その返事の台本をタグの位置へ差し込んで続きを再生させる（ukadoc `list_sakura_script` の `\![embed,…]`）。プロパティを読む `\![get,property,…]` の結果をその場で使う組み合わせの片割れでもある。areka はこのタグを汎用キャリアとして運ぶだけで、誰も消費しない＝書いたゴーストは差し込みが起きず、続きの台詞が欠ける。

## Current State

- `\![embed,…]` は字句・変換で `Instruction::GenericCommand` になり、dola の `CueCommand::Custom` として全部の受け口へ配られ、全員に無視される。網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` の `\![embed,…]` の行の持ち主は `areka-P0-property-query-channels`（着地時に本 spec へ直す）。
- 台本は再生の前にまとめてコンパイルされ、絶対時刻を焼き込んだ cue の列になる（`crates/areka-sakura/src/compile.rs`）。途中で長さの分からない待ちを入れる仕組みは無い（`sakura-time-directives` の C 群の論点）。
- kanade は台詞 1 つにつき SHIORI の往復を 1 回まわし、再生中の往復の口は無い（`translate-pipeline` が足したのは再生の前の `OnTranslate` だけ）。
- 送る側の要素は `property-query-channels` が作る: 許可の表に無い任意の名前のイベントを出所つきで送る口と、`SenderType` のヘッダ（ukadoc の `SenderType` に `embed` がある）。

## Desired Outcome

- 再生が `\![embed]` に達したら、そのイベントを `SenderType: embed` で送り、返事の台本（Result）をタグの位置へ差し込み、差し込んだ先頭から再生を続ける。返事が無ければタグを消して続ける。
- 差し込んだ台本も表示の規則（`\![embed]` の入れ子を含む）どおりに再生される。待ちの間の時間は後ろの cue の絶対時刻へ正しく反映される。
- 決定論のテストで、差し込みあり・なし・失敗・入れ子・中断を固定する。

## Approach

台本を `\![embed]` の位置で分け、前半を再生し終えた点で待ちに入り（dola の待ちの継ぎ目）、kanade が往復を回して返事を後半の前へつないで組み直す。組み直しの範囲と、続きの絶対時刻の付け直しが設計の要。

## Scope

- **In**: `\![embed]` の台本の分割と待ち・kanade の再生中の往復・返事の差し込みと続きの再生・`SenderType: embed`・決定論のテスト・網羅台帳 1 行・COMPAT §8。
- **Out**: `\![get,property]` などの照会の経路（`property-query-channels`）・`\![sound,wait]`・`\![wait,syncobject]` など他の待ち（消費する者が現れるまで roadmap の覚え書き）・`\![raise]`。

## Boundary Candidates

- 台本の側（分割と待ちの継ぎ目）／運行の側（再生中の往復と組み直し）。

## Out of Boundary

- 任意の名前のイベントを送る口・`SenderType` の運搬そのもの（`property-query-channels`）。
- 時間の指令の焼き込み（`sakura-time-directives`）。

## Upstream / Downstream

- **Upstream**: `property-query-channels`（任意の名前のイベント・`SenderType`）・完了 `translate-pipeline`・完了 `cue-playback-duration`（絶対時刻の台本・待ちの継ぎ目）。
- **Downstream**: `\![get,property]` と組み合わせて使う実ゴーストの適合。

## Existing Spec Touchpoints

- **Extends**: なし（新しい経路）。
- **Adjacent**: `property-query-channels`（分割元）・`sakura-time-directives`（同じコンパイルの時間まわり・C 群の他の語）・台本のコンパイルの列（`anchor-tag-canon`・`talk-fast-forward`）・kanade の進行の列。

## Constraints

- 正典は ukadoc。1 フレーム遅らせる解は取らない。決定論のテスト必達。
- `crates/areka-sakura/src/compile.rs` と kanade の `schedule/` を触る＝台本のコンパイルの列と kanade の進行の列の両方で、同じ列の spec と同時に走らせない。`steady.rs`（929 行）・`schedule/mod.rs`（937 行）に足して 1,000 行を超えるなら先頭のタスクで分割する。

## 2026-10-04 棚卸㉑で切り出し

- 元の spec: `property-query-channels`（`\![embed]` の対）。
- 規模: M（10〜14 タスク）。
- 前提: `property-query-channels`（未）。
- 触るファイル: `crates/areka-sakura/src/compile.rs`・`crates/areka-sakura/src/drive.rs`・`crates/dola/src/cue/`・`crates/areka-kanade/src/{msg.rs, actor.rs, schedule/}`・`doc/ukadoc-coverage/ledger/sakura-script.toml`・`doc/COMPAT_ARCHITECTURE.md` §8。
- 共有しうる相手: `anchor-tag-canon`・`talk-fast-forward`・`sakura-time-directives`（`compile.rs`）、`mouse-drag-events`・`balloon-lifecycle-events`・`sakura-time-critical`（kanade の `schedule/`）。
- 議題: 待ちの間に利用者の中断・マウスの返事の置き換えが来たときの扱い（差し込みを捨てるか）。

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: M（10〜14）のまま。切る: なし。
- 前提の状態: `property-query-channels` はまだ＝着手できない。C3 は `areka-sakura`・`dola` に触れていない（`compile.rs` 411・`drive.rs` 565 は前回と同じ）。
- 崩れた前提／古くなった位置:
  - `SenderType: embed` を送る口の持ち主が変わりうる: 10-05 起票の `script-security-level`（優先・前提なし）が出どころ（`\![embed]` を含む）と `SenderType` の運搬を持つ。どちらが運んでも、本 spec は「出どころ `embed` を付けて送る」だけで済む。上流に `script-security-level` を足す。
  - kanade の行数: `schedule/steady.rs` 947・`schedule/mod.rs` 938・`msg.rs` 909。再生中の往復は新しいファイル（例 `schedule/embed.rs`）に置き、上の 2 本へは呼び出しの数行だけにする。
  - 共有の相手から `mouse-drag-events` は着地で外れた（✅ 10-05）。
- 触るファイル: `crates/areka-sakura/src/{compile.rs, drive.rs}`・`crates/dola/src/cue/`・`crates/areka-kanade/src/{msg.rs, actor.rs}`・`schedule/`＋新規ファイル・`doc/ukadoc-coverage/ledger/sakura-script.toml`（`\![embed,…]` の 1 行）・`doc/COMPAT_ARCHITECTURE.md` §8。
- 議題（答えで作業が変わるものだけ）: 前回の 1 つ（待ちの間の中断・置き換え）のまま。
- 見つけた穴: 網羅台帳の `\![embed,…]` の行の持ち主が今も `property-query-channels`（実害なし・すぐ直せる）。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**:
  - `property-query-channels` は未着手のまま＝着手できない（その前に `mcp-kanade-tools`・`script-security-level`・`property-name-case-fold`）。
  - **`budoux-reveal-reflow`（✅ 10-06）で、合図の受け手は登録の時に「これから届く合図の全部」を 1 度だけ先に受け取るようになった**（dola の `CueSink::preview`＝`crates/dola/src/cue/{runtime,sink,schedule}.rs`）。文字の層はこの全文で文節の折り返しを決め、「出した字の行は動かさない」を約束している。`\![embed]` は再生の途中で台本を差し込むので、先に渡した全文が途中で変わる＝差し込みの後の全文の渡し直しと、既に出した行を動かさないことの両立が、設計の要に加わる。
  - `mcp-author-tools`（✅ 10-08）の `check_script` は、今 `\![embed,…]` を「誰も拾わない指令」と答える（事実どおり）。本 spec が着地するとき、受け取り手の表（`crates/areka/src/emo2_boot/consumer_ledger.rs`・今 23 組）に `embed` の行を足し、一致のテスト `consumer_ledger_agreement_tests.rs` の見本と `doc/ssp-mcp/areka-tools.md` の組の数を直す（触るファイルに無かった）。
  - `choice-script-prefix`（✅ 10-07）が、返事の台本を新しいトークとして走らせる口を kanade に足した（`crates/areka-kanade/src/schedule/steady_choice_script.rs`）。`\![embed]` は途中への差し込みで別物だが、台本を受けて走らせる所の手本になる。
  - 行数: kanade の `schedule/steady.rs` **950**・`schedule/mod.rs` **955**・`msg.rs` **926**・`actor.rs` **900**（本文 Constraints の 929・937 は古い）。`crates/areka-sakura/src/compile.rs` 411・`drive.rs` 565。
- **触るファイル**: `crates/areka-sakura/src/{compile.rs, drive.rs}`・`crates/dola/src/cue/{runtime.rs, schedule.rs, sink.rs}`（407・341・87）・`crates/areka-kanade/src/{msg.rs, actor.rs}`・`schedule/` の新しいファイル（`steady.rs`・`mod.rs` へは呼び出しの数行）・全文の先渡しを受ける側（`crates/areka-emo-text/src/{sink.rs, actor.rs}`・`crates/areka/src/emo2_boot/talk_clock.rs` の見込み）・`crates/areka/src/emo2_boot/consumer_ledger.rs`（**943**）と `consumer_ledger_agreement_tests.rs`・`doc/ssp-mcp/areka-tools.md`・台帳 `sakura-script.toml` の 1 行・`doc/COMPAT_ARCHITECTURE.md` §8。
- **規模**: M（12〜16。全文の渡し直しと受け取り手の表の分で 10〜14 から上げた）。**分割の案**: なし（一度切り出した spec）。
- **先に要るもの**: `property-query-channels`（働き）。
- **ファイルの重なり**: 台本のコンパイルの列（`anchor-tag-canon`・`range-choice-tag`・`talk-fast-forward`・`sakura-time-directives`）・kanade の進行の列の全員・文字とバルーンの列（emo-text に触れる場合）・`emo2_boot` の結線の列（`consumer_ledger.rs`＝`mcp-reload`・`property-query-channels`）。
- **優先度の区分**: C（ukadoc の `\![embed]` の拾い残し）。**要件定義のモデル**: Fable（再生の途中の往復・時刻の付け直し・中断）。
- **見つけた穴・古くなった記述**: 網羅台帳 `sakura-script.toml` の `\![embed,…]` の行の持ち主は今も `property-query-channels` のまま（この棚卸では台帳を直していない。本 spec が着地のときに、数の欄と一緒に直す）。
