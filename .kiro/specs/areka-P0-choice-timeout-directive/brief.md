# Brief: areka-P0-choice-timeout-directive

> 2026-10-02 棚卸⑳で `areka-P0-sakura-time-directives`（L・全部入れると 20 タスクを超える）の B 群から切り出した。file:line は起票時値（main `03e8d7d6`）。行番号は目安で、正本は「何の定義行か」の方。

## Problem

ゴーストが台本に `\![set,choicetimeout,時間]` を書いても、areka はそれを読まない。選択肢は必ず既定の 30 秒で時間切れになり、`OnChoiceTimeout` が飛ぶ。

利用者から見える害:

- 「時間切れなし」（`0` か `-1`）を指定したメニューが、30 秒で勝手に閉じる。里々・YAYA のゴーストのメニューや長考の選択肢でよく使う書き方である。
- 早押しのような短い指定（例 `500`）も効かず、30 秒待つ。

正典（ukadoc `\![set,choicetimeout,時間]`）: 「選択肢のタイムアウトの時間を指定する。単位はミリ秒。時間のカウントはトークの表示が全て終わってから開始される。そのスクリプト中のみ有効。選択肢より後ろに書いても有効。選択肢がタイムアウトした場合、OnChoiceTimeoutイベントが発生する。時間指定を省略：デフォルト値に戻す／時間指定が0か-1：タイムアウトしない」。

## Current State

- 台本を cue へ写す `crates/areka-sakura/src/compile.rs` は、選択肢があれば選択待ちの barrier を**常に** `BarrierKind::WaitForChoice { timeout: None }` で書く（`timeout: None` の行）。`\![set,choicetimeout,…]` は汎用の入れ物のまま流れ、消費する者が居ない。
- 受ける側はもう在る。`crates/areka-kanade/src/schedule/steady.rs` の `on_choice_waiting` は `timeout_directive_secs: Option<f64>` を受け、`choice_deadline`（`schedule/choice.rs`）が `None`＝既定（`KanadeConfig::choice_timeout_default_ms`＝30,000）・`v <= 0.0`＝無期限・`v > 0.0`＝明示、と写す。時間切れの発火（`fire_choice_timeout_if_due`）と `OnChoiceTimeout` も完了 `choice-select-events` で動いている。
- つまり欠けているのは「台本の指定を barrier の `timeout` へ焼く」ことだけである。
- 単位の食い違いに注意: 正典はミリ秒、受ける側の引数は秒（`timeout_directive_secs`）。途中の型（dola の `WaitForChoice.timeout` と、それを kanade へ運ぶ通知）がどちらの単位で持っているかを要件の段で確かめ、変換を 1 か所に置く。

## Desired Outcome

1. `\![set,choicetimeout,N]` を書いた台本の選択肢は、表示が全部終わってから N ミリ秒で時間切れになる。
2. `0` と `-1` は時間切れなし（選択待ちは無期限に続く）。
3. 時間を省いた `\![set,choicetimeout]` は既定へ戻す。
4. 選択肢より後ろに書いても効く（台本全体を先に見て決める）。同じ台本に複数回書いたときの扱い（最後のものが勝つ、を推す）を要件で決めて固定する。
5. 効くのはその台本の中だけ。次の台本は既定から始まる。
6. 決定論テスト: 台本の文字列を直に入れて、barrier の `timeout` が期待の値になること（指定なし・正の値・`0`・`-1`・省略・選択肢の後ろ・複数回・数でない値）。数でない値は記録を残して既定のまま。
7. 網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` の該当の行を実装済みへ。

## Approach

`compile.rs` が台本を cue へ写す前に、台本全体から `\![set,choicetimeout,…]` を探して値を決め、選択待ちの barrier を書くときにその値を入れる。完了 `sakura-dialogue-tags` の R4.3（compile が中身を読んでよいコマンドの許可の一覧）に、このコマンドは最初から載っている＝原則の変更は無い。

- 汎用の入れ物としての転記は残す（消費する者が居ないので何も起きない・今日と同じ）。
- 受ける側（kanade）の判定は変えない。変えるのは焼く値だけ。

## Scope

- **In**: `\![set,choicetimeout,時間]` の compile での解釈と barrier への焼き込み・単位の変換・決定論テスト・台帳の 1 行・`doc/COMPAT_ARCHITECTURE.md` §8 に正典が黙っている点（複数回書いたとき）の 1 行。
- **Out**: `\![set,balloontimeout,時間]`（受ける側が未実装＝`balloon-lifecycle-events` の項目 7 と対。`sakura-time-directives` に残す）／`quicksection`・`balloonwait`・`embed`・`sound,wait`・`syncobject`・`move` ほかの時間の引数（`sakura-time-directives` に残す）／既定の 30 秒そのものの見直し／時間切れの発火と `OnChoiceTimeout`（完了済み）。

## Boundary Candidates

- 台本全体からの値の決定（純関数）
- barrier への焼き込みと単位の変換

## Out of Boundary

- kanade の選択待ちの帳簿と期限の判定（読むだけ・変えない）
- compile が中身を読んでよいコマンドの一覧の拡張（このコマンド以外は読まない）

## Upstream / Downstream

- **Upstream**: 完了 `areka-P0-sakura-dialogue-tags`（R4.3）・完了 `areka-P0-choice-select-events`（時間切れの受け側）・完了 `cue-playback-duration`（barrier）。
- **Downstream**: `areka-P0-sakura-time-directives`（残りの A・C・D 群と `balloontimeout`。同じ `compile.rs` を触る＝本 spec の後）・`areka-P0-anchor-tag-canon`（同じ `compile.rs`）。

## Existing Spec Touchpoints

- **Extends**: なし（`sakura-time-directives` の brief から B 群の `choicetimeout` を引き取る＝同 brief に注記済み）。
- **Adjacent**: `areka-P0-talk-fast-forward`（早送りは「表示が全部終わってから数え始める」の起点を早めるだけで、値には触らない）。

## Constraints

- 触るソースは `crates/areka-sakura/src/compile.rs` とその兄弟テスト（新しいファイル）、必要なら単位の変換を置く 1 か所だけ。`crates/areka-kanade/src/schedule/` の判定・`crates/areka-emo-text/`・`crates/areka/src/emo2_boot/`・`crates/areka/src/input_events/` には触らない（同じウェーブの他の spec が触る）。
- 正典は ukadoc。SSP の既定の秒数を実測して合わせることはしない。
- 決定論テスト必達。ネットにも実機にも依らない。

## 想定

- 規模 XS〜S（3〜5 タスク）。議題は 1 件（同じ台本に複数回書いたとき）。Opus で足りる。
