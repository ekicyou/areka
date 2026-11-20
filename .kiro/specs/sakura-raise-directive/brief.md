# Brief: sakura-raise-directive

> 2026-11-20 `/kiro-discovery` で起票。出どころは、ノベル（演劇）の暫定の設計と検証（2026-11-19）で見つかった、正典の拾い残し。開発者は 2026-11-20 に、pasta の依頼書を待たずに起票する事を承認した（議題 10「OK」）。

## Problem

- **ゴーストの作者**: 台本の途中や末尾で、自分の SHIORI にイベントを起こす正典のタグ `\![raise,イベント名,r0,r1,…]` が、areka では動かない。タグは読み飛ばされ、その先の台本が始まらない（黙って壊れる）。
- 話をつないでいく書き方（1 つの台詞の後に、次のイベントの台詞を続ける）に、広く使われている。網羅台帳の調べでは、wiki の作例の文書に、里々で 8 件、YAYA で 2 件（弱い代理の数）。
- pasta のノベル（演劇）は、SSP 向けには、1 本の末尾を `\x\![raise,OnPlayNext]` の形で出す。areka 向けは、2026-11-20 に「続きを示すキューコマンド」に替わったので、このタグはノベルの芯ではなくなった。それでも、正典のタグとして要る。

## Current State

起票のときの実測（main `49b6f3ab5`）:

- `\![raise,…]` は未対応（網羅台帳 `doc/ukadoc-coverage/ledger/sakura-script.toml` の `\![raise,イベント名,r0,r1,r2...]` の行・`absent`・持ち主なし）。名前は運ぶ入れ物に載って最後まで運ばれるが、拾う受け口が無い。
- **正典**（ukadoc を 2026-11-19 に引いた）: 「イベントを実行する。第3引数以降はReference0以降に順番に格納される。」（`https://ukatech.shillest.net/ukadoc-spec/list_sakura_script.html#bang-raise-r0-r1-r2`）
- **近い道が 2 つ在る**:
  - `\![get,property,…]` は、台本のスレッドの受け口から運行の係（kanade）へ頼み、SHIORI へ GET を送る（`crates/areka/src/session/` の `prop_get_sink.rs`）。
  - MCP の `raise_event` は、イベント名と Reference の列を SHIORI へ送る（`crates/areka/src/mcp/raise_event.rs`・`KanadeMsg::SstpNotify`）。こちらは、送り手が外（MCP）で、起動の挨拶の間は断る、という別の決まりを持つ。
- SHIORI へ送るイベントの名前は、許可した名前の表で縛っている（`crates/areka-kanade/src/schedule/events.rs` の `ALLOWED_EVENT_IDS`・「ukadoc の固定名だけ」）。`\![raise]` の相手は、作者が決める任意の名前である。任意の名前を、出どころごとの決まりで通す前例は在る（SSTP・プロパティの照会）。
- 暫定の設計の記録: `doc/ideas/novel-play-flow-provisional/provisional-design.md` と `provisional-design-validation.md`（`\![raise]` の節）。**正式な設計ではない。** 検証は「受け口から kanade へ送る道は成り立つ。順が入れ替わる道は見つからなかった」と判定し、台本の出どころ（`SecurityLevel`）を運ぶ型が足りない、と指摘している。

## Desired Outcome

- `\![raise,イベント名,r0,r1,…]` が、正典どおりに動く。台本がそのタグに着いた時に、SHIORI にそのイベントが届き、引数は Reference0 から順に載る。
- 返った台本は、ふつうの返事と同じに再生される（今の台本を置き換える）。何も返らなければ、何も起きない。
- 置き換えられた台本の、まだ着いていない `\![raise]` は、実行されない。
- 外から来た台本（SSTP・MCP）の `\![raise]` は、台本の出どころの決まり（`SecurityLevel`）に従う。
- 読めない形（イベント名が空・制御文字）は、トークを止めずに記録する。

## Approach

- `\![get,property]` と同じ形の道にする（台本のスレッドの受け口 → kanade → その名前の GET）。MCP の `raise_event` の道は、送り手と断る決まりが合わないので使わない（暫定の設計の見立て）。
- 許可した名前の表（`ALLOWED_EVENT_IDS`）は触らない。任意の名前を、名前と Reference の検査だけで通す出どころを 1 つ足す（前例と同じ形）。
- **areka の立場**（開発者 2026-11-20）: 「イベントを投げて、さくらスクリプトが返ってきたら、再生する」という仕組みを壊さない。このタグは、その輪を台本の側から回す正典の道具である。正典のタグの意味は変えない（「返事が届くまで話が続いていると扱う」などの areka だけの意味を、このタグに足さない。それは、ノベルの「続きを示すキューコマンド」の持ち物）。

## Scope

- **In**: `\![raise,イベント名,r0,…]` の受け口・kanade への道・任意の名前を通す出どころ・台本の出どころを運ぶ所・読めない形の記録・`check_script` の表（`consumer_ledger.rs`）への登記・網羅台帳の行と `doc/COMPAT_ARCHITECTURE.md` の行・決定論のテスト。
- **Out**: `\![notify,…]`・`\![raiseother,…]`・`\![notifyother,…]`・`\![raiseplugin,…]`・`\![notifyplugin,…]`・`\![timerraise,…]` の系（どれも持ち主なし。`\![notify]` は、返事を使わないだけの兄弟なので、要件の段で一緒に入れるかを決める）・`\![embed,…]`（`sakura-embed-directive`）・ノベルの「続きを示すキューコマンド」と目印とバックログ（pasta の取り決めを待つ受け口）。

## Boundary Candidates

- 台本の受け口（`crates/areka/src/session/` の受け口の並び）。
- kanade の伝言と予定（`crates/areka-kanade/src/msg.rs`・`schedule/`）。
- イベントの名前の出どころ（`crates/areka-kanade/src/schedule/events.rs`）。

## Out of Boundary

- 許可した名前の表（`ALLOWED_EVENT_IDS`）の決まりは変えない。
- 返った台本の再生の仕方（置き換え）は変えない。

## Upstream / Downstream

- **Upstream**: 待つ物は無い。同じファイル（kanade の `msg.rs`・`schedule/`・`consumer_ledger.rs`）を触る spec と、同じウェーブに置かない。
- **Downstream**: `sakura-embed-directive`（同じ「台本からイベントを起こす」群）・SSP 向けの pasta のノベルの台本（`\x\![raise,…]`）を areka で再生した時の動き。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `sakura-embed-directive`・`ghost-call`・`script-impact-tiers`（`\![raise]` を挙げている）・`check-script-arg-checks`・`talk-fast-forward`（`\x` の待ち。末尾の `\x` の後ろの `\![raise]` へ続く）。

## Constraints

- 区分 A（開発者が起票を承認した）。正典のタグで、`play` の命令ではない（「`play` に関わる物は v0.3.0 より後」の対象外）。置く波は、棚卸で決める。
- 規模の見立て: S〜M（5〜7 タスク）。Opus。
- 意味は ukadoc から輸入する。ukadoc で読めない点（台本の途中の `\![raise]` の後ろが続くか・返事が届くまでの見え方・外から来た台本の `\![raise]`）は、議題にする前に SSP の MCP で測る。
- 失敗を黙って捨てない。メッセージボックスは出さない。
