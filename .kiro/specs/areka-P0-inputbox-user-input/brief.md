# Brief: areka-P0-inputbox-user-input

> 2026-10-10 棚卸㉓で、roadmap の覚え書き「持ち主のいない SHIORI のイベント 2 群」の ⑵（入力欄）から起票した。棚卸㉒で `mcp-user-response` の再測定から登記された件。コードは「何の定義か」で指す。着手時に引き直す。

## Problem

ゴーストが利用者に文字を打ってもらう道が areka に無い。台本の `\![open,inputbox,…]` は黙って消え、入力欄は開かず、入力を知らせるイベント `OnUserInput`・`OnUserInputCancel` も SHIORI に届かない。名前を聞く・言葉を教える・数を入れてもらう、といった多くのゴーストの基本の会話が、areka では始まらない。

## Current State

- 正典（ukadoc「さくらスクリプト」と「SHIORI Event」の一覧を ukadoc MCP で確かめた。以下は要旨）:
  - `\![open,inputbox,ID,表示時間,テキスト,オプション,...]`: 入力欄を開く。決定（OK）で `OnUserInput`。時間切れと閉じるボタンで `OnUserInputCancel`。`OnUserInputCancel` に返事が無く、かつ時間切れだったときは、互換のために入力を `timeout` とした `OnUserInput` が起きる。ID が `On` で始まるときは、ID の名前のイベントが起きる。表示時間はミリ秒で、省略か 0 以下なら無制限。テキストは開いた直後に入れておく文字（省略可）。SSP だけ `--timeout=`・`--text=`・`--option=` の形でも書ける。入力の間もほかの操作は止めない（SSP）。
  - `\![close,inputbox,ID]`: その ID の入力欄を閉じる（日付・時間・スライダー・伏せ字の入力欄も含む）。全部閉じるときは ID に `__SYSTEM_ALL_INPUT__`。
  - `OnUserInput`: Reference0＝入力欄の ID・1＝入力された内容・2＝補足（スライダーだけ）・3 以降＝`--reference=` で指定した追加の情報。
  - `OnUserInputCancel`: Reference0＝ID・1＝時間切れは `timeout`／閉じられたら `close`・2 と 3 以降は上と同じ。
  - 同じ族: `passwordinput`（伏せ字）・`dateinput`（年月日）・`timeinput`（時分秒）・`sliderinput`（つまみ）・`ipinput`（IP アドレス）。どれも「ほかの動作は inputbox と同じ」と書く。
  - 別の系統: `teachbox`（`OnTeachStart`・`OnTeach`・`OnTeachInputCancel`）・`communicatebox`・入力の補完 `inputbox.autocomplete`・入力欄の最初の位置（SHIORI リソースの `inputbox.defaultleft`／`defaulttop`）。
- 網羅台帳: `\![open,inputbox,…]`・`\![close,inputbox,ID]`（`doc/ukadoc-coverage/ledger/sakura-script.toml`）と `OnUserInput`・`OnUserInputCancel`（同 `shiori.toml`）は、どれも `absent`・持ち主なし。同じ族の 5 つ・`teachbox`・`communicatebox`・`inputbox.autocomplete` も `absent`・持ち主なし。
- コード: `inputbox` を扱う所は 0。`\!` の対応表 `ConsumerLedger::canonical`（`crates/areka/src/emo2_boot/consumer_ledger.rs`・943 行）の `open` の行は 6 つ（`readme`・`file`・`browser`・`explorer`・`editor`・`mailer`＝完了 `open-external-tags` の受け口 `ReadmeCueSink`）だけで、`(open, inputbox)` と `close` の行は無い。＝`check_script` は `\![open,inputbox,…]` に `unknown_command` を返す。
- 送ってよいイベントの表 `ALLOWED_EVENT_IDS`（`crates/areka-kanade/src/schedule/events.rs`・793 行）に `OnUserInput`・`OnUserInputCancel` は無い。外からイベントを頼む口は `KanadeMsg::RaiseEvent`（`crates/areka-kanade/src/msg.rs`）で、表にある名前だけを通す。
- 「`On` で始まる ID は、その名前のイベントを起こす」の前例は選択肢にある（`crates/areka-kanade/src/schedule/choice.rs` の `CascadePlan::Named` と、`events.rs` の `is_allowed_choice_event`）。「返事が無ければ次のイベントを送る」の前例も同じ所（`CascadePlan::Canonical`）。
- areka が今使っている OS 標準の部品は、右クリックのメニュー（`crates/areka/src/menu/win32.rs`）・起動できないときの告知と、はい／いいえを尋ねる口（`crates/areka/src/alert.rs`）・ツールチップ（wintf の `ecs/tooltip/`）。文字を打つ部品は wintf にも areka にも無い。メッセージの汲み出し（`crates/wintf/src/com/wuc.rs`）はキーの変換（`TranslateMessage`）を呼んでいるが、ダイアログ用のキーの扱い（Tab・Enter）は入っていない。
- `mcp-user-response` の brief は「入力欄は areka に無い・持ち主の spec が無い」と書き、入力欄を先に別の spec で作るか自分の範囲を絞るかを議題にしている。

## Desired Outcome

- `\![open,inputbox,ID,表示時間,テキスト]` で入力欄が開き、利用者が文字を打って決定すると `OnUserInput`（ID が `On` で始まるときはその名前のイベント）が正典どおりの Reference で届く。
- 閉じる・時間切れで `OnUserInputCancel` が届き、時間切れで返事が無いときは `timeout` の `OnUserInput` が続く。
- `\![close,inputbox,ID]` と `__SYSTEM_ALL_INPUT__` で閉じられる。
- 入力欄が開いている間も、ゴーストは喋り、ほかの操作も止まらない。
- 日本語の入力（IME）がふつうに使える。
- `check_script` が `\![open,inputbox,…]` を「拾う者がいる」と答える。網羅台帳の 4 行が実物と合う。

## Approach

受け口を 1 つ足す（`open,inputbox` と `close,inputbox` を拾う）。入力欄そのものは 1 つの部品にまとめ、開く・閉じる・決定・取り消し・時間切れを「出来事」として kanade へ渡す。kanade は表に 2 つの名前を足し、時間切れの 2 段（`OnUserInputCancel` → `OnUserInput`）と `On` で始まる ID を、選択肢と同じ形で扱う。入力欄の見た目と作り（OS 標準の部品か、バルーンに合わせた箱か）は議題 1 で開発者が決める。

## Scope

- **In**: ふつうの文字の `inputbox`（ID・表示時間・最初のテキスト）・`\![close,inputbox,ID]`（`__SYSTEM_ALL_INPUT__` を含む）・`OnUserInput`・`OnUserInputCancel`・時間切れの 2 段・`On` で始まる ID・同じ ID を 2 回開いたときの扱い・ゴーストの切替と終了のときに閉じること・対応表の行・網羅台帳の 4 行・決定論テスト。
- **Out**（どれも網羅台帳は `absent`・持ち主なしのまま残る。起票は要望が出てから）:
  - 同じ族の 5 つ（`passwordinput`・`dateinput`・`timeinput`・`sliderinput`・`ipinput`）。本 spec の部品に種類を足す形で後から足せるように作るが、足さない。伏せ字は `script-security-level` の後で（外から来た台本が利用者に秘密を打たせる道になる）。
  - `teachbox`・`communicatebox`（別のイベントの組を持つ別の入力欄）。
  - 入力の補完 `inputbox.autocomplete`・最初の位置のリソース・バルーンの descript の入力欄の見た目の指定。
  - SSP だけの `--option=`（`noclose` など）と `--reference=`。`--timeout=`・`--text=` の書き方を受けるかは要件で決める。
  - 答えをエージェントへ返す道（`mcp-user-response`）。

## Boundary Candidates

- 台本の受け口（`crates/areka/src/emo2_boot/` の新しいファイル）と対応表（`consumer_ledger.rs`）。
- 入力欄の部品（`crates/areka/src/` の新しいフォルダ）。
- kanade のイベント（`schedule/events.rs` の表と組み立て・時間切れの 2 段の調停）。
- 表示時間の計り方（areka の「時刻は正確に扱う」の原則に乗せる）。

## Out of Boundary

- 台本を読む段（`\![open,…]` は今も汎用の運び方で最後まで届く。読む段は変えない）。
- 開く系の受け口 `ReadmeCueSink`（同じ `open` でも第 1 引数が違う。触らない）。
- 文字の層（バルーンに合わせた箱にすると決めたときだけ関わる）。

## Upstream / Downstream

- **Upstream**: 働きの上では無い。外から来た台本（MCP・将来の SSTP）が入力欄を開けるかどうかの線は `script-security-level`・`script-impact-tiers` が決める（本 spec は SHIORI の台本から開く道を作る）。
- **Downstream**: `mcp-user-response`（利用者の入力をエージェントへ返す。入力欄があってはじめて意味を持つ）・同じ族の 5 つ・`teachbox`／`communicatebox`。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `mcp-user-response`（答えの返り道）・`open-external-tags`（完了。`\![open,…]` の第 1 引数で受け口を分ける形の手本）・`script-security-level`・`script-impact-tiers`（入力欄を開くことの段）・`consumer_ledger.rs` に行を足すすべての spec（`mcp-reload`・`makoto-reload-directives`・`sakura-time-critical` ほか）・kanade の `schedule/events.rs` に触る spec（`mcp-kanade-tools`・`anchor-tag-canon`・`mouse-click-wheel-events` ほか）。

## Constraints

- メッセージボックスは出さない（開発者の方針）。入力欄は利用者の操作を止めない窓にする（正典も、SSP は入力の間ほかの操作を止めないと書く）。
- OS の持つ仕組みを最大限使う（開発者の方針）。メニューは OS 標準。
- 入力された文字は台本として解釈しない。SHIORI へ渡す Reference に入れるだけ（改行や区切りの文字の扱いを要件で決める）。
- 表示時間は丸めない（時刻は正確に扱う）。
- `consumer_ledger.rs` は 943 行で上限（1,000 行）に近い。行を足す前に、ファイルの中のテストの塊を兄弟のファイルへ出す（ほかの brief と同じ段取り）。対応表と受け口の一致のテスト `consumer_ledger_agreement_tests.rs` にも行を足す。
- kanade の `msg.rs` は 926 行。出来事を足すなら新しいファイルへ置く。
- 実機の確かめ（文字を打つ・IME・時間切れ）は開発者の机で。

## 2026-10-10 棚卸㉓の測定（main `ee3af616`）

- **触るファイル**: 新しい受け口 `crates/areka/src/emo2_boot/inputbox_cue.rs`＋兄弟のテスト・入力欄の部品（新しいフォルダ）・`emo2_boot/consumer_ledger.rs`（943）・`consumer_ledger_agreement_tests.rs`（334）・`emo2_boot/mod.rs`（912・受け口の登録）・`crates/areka-kanade/src/schedule/events.rs`（793）・kanade の新しいファイル（入力の出来事と 2 段の調停）・`doc/ukadoc-coverage/ledger/{sakura-script,shiori}.toml` の 4 行・`doc/ssp-mcp/areka-tools.md`（受け取り手の数の 1 行）・`doc/COMPAT_ARCHITECTURE.md` §8。OS 標準の部品にするなら wintf には触らない見込み（キーの扱いを汲み出しに足す必要が出たときだけ `crates/wintf/src/com/wuc.rs`）。バルーンに合わせた箱にするなら `crates/areka-emo-text/` に広く触る。
- **規模**: M〜L（OS 標準の部品なら 12〜16 タスク・バルーンに合わせた箱なら 20 を超える見込み＝要件の段で切る）。
- **先に要るもの**: 働きの上では無い。同じウェーブに置けない相手: `consumer_ledger.rs` に行を足す spec と、kanade の `schedule/events.rs` に触る spec（Adjacent のとおり）。
- **優先度の区分**: C（ukadoc の拾い残し。台帳が「黙って壊れる」と書く穴だが、作っていない機能で、バグではない）。網羅台帳の優先度は `A7`。
- **要件定義のモデル**: Fable（入力欄の作りは開発者の判断・時間切れの 2 段と `On` で始まる ID は正典の読み方が要る）。
- **議題**:
  1. **入力欄の作り（開発者の判断）**。案 A: OS 標準の入力の部品を載せた小さな窓（IME・コピーと貼り付け・読み上げが OS のまま使える・メニューと同じ「OS 標準」の線・規模は小さい・見た目はゴーストに合わない）。案 B: バルーンに合わせた箱を areka が描く（見た目は合う・文字の入力と IME の変換の表示を自前で作ることになり重い）。推しは A（「OS の仕組みを最大限使う」「メニューは OS 標準」の決まりと同じ向き）。
  2. 入力欄を出す場所。正典の最初の位置のリソース（`inputbox.defaultleft`／`defaulttop`）は範囲の外にしたので、areka の決まり（キャラクターの近く・画面の中央など）を 1 つ決める。
  3. 時間切れの 2 段を kanade のどこで扱うか（選択肢の 2 段と同じ調停に乗せるか）。`On` で始まる ID のイベントを通す決まり（選択肢の `is_allowed_choice_event` と同じ「`On` で始まること」だけにするか）。
  4. 話の最中に `OnUserInput` の返事が届いたとき、今の話を置き換えるか（マウスの返事と同じ扱いにするか）。
  5. 外から来た台本（MCP の `sakurascript`）が入力欄を開いてよいか。`script-security-level` が着地するまでの扱い。
- **ukadoc の照合**: ukadoc「さくらスクリプト」の `\![open,inputbox,ID,表示時間,テキスト,オプション,...]`・`\![close,inputbox,ID]`・`\![open,passwordinput,…]`・`\![open,dateinput,…]`・`\![open,timeinput,…]`・`\![open,sliderinput,…]`・`\![open,ipinput,…]`・`\![open,teachbox]`、「SHIORI Event」の `OnUserInput`・`OnUserInputCancel`・`inputbox.autocomplete` を ukadoc MCP で確かめた。`--option=` に書ける語の一覧は確かめていない（例の `noclose` だけ）。
