# Brief: key-press-event

> 2026-11-20 `/kiro-discovery` で起票。出どころは、ノベル（演劇）の暫定の設計と検証（2026-11-19）で見つかった、正典の拾い残し。開発者は 2026-11-20 に、pasta の依頼書を待たずに起票する事を承認した（議題 10「OK」）。

## Problem

- **ゴーストの作者**: キーを押した時に SHIORI に届く正典のイベント `OnKeyPress` が、areka では届かない。キーで反応するゴースト（キーで話しかける・キーでメニューを開く）が、黙って動かない。
- **pasta のノベル（演劇）**: 設定の画面への入口に、キーボードが必須と決まっている（pasta の `novel-areka-contract` の決定「入口は 3 つ: 右クリック・常に出ているボタン・キーボード。キーボードも必須」）。受け口は、今の条件の属性で分ける（`＊OnKeyPress　＆＄ｒ０＝「…」`）。Alt+Enter も、ベースウェアが自分で窓の形を切り替えずに、`OnKeyPress` で送る、と決まっている。

## Current State

起票のときの実測（main `49b6f3ab5`）:

- `OnKeyPress` は未対応（網羅台帳 `doc/ukadoc-coverage/ledger/shiori.toml` の `OnKeyPress` の行・`absent`・持ち主なし）。運行の係（kanade）に、このイベントを送る所が無い（`crates/areka-kanade/src` を `OnKeyPress` で grep して 0 件）。
- キーの入力を受ける所は、wintf に在る（`crates/wintf/src/ecs/window_proc/keyboard.rs`）。areka の本体は、それを SHIORI へ運んでいない。
- **正典**（ukadoc を 2026-11-20 に引いた）: 「本体がアクティブな状態でキーが入力された際に発生。」（`https://ukatech.shillest.net/ukadoc-spec/list_shiori_event.html#OnKeyPress`）
  - Reference0: 「入力されたキー。(キートップに書かれた文字)」
  - Reference1: 「入力されたキー。(Win32仮想キーコード)」
  - Reference2: 「※SSPのみ　キーの押しっぱなしカウント。」
  - Reference3: 「※SSPのみ　キーイベントが来たキャラクターウインドウID。」
  - Reference4: 「※SSPのみ　修飾キーがカンマ区切りで列挙される。例："ctrl,alt"」
- SHIORI へ送るイベントの名前は、許可した名前の表で縛っている（`crates/areka-kanade/src/schedule/events.rs` の `ALLOWED_EVENT_IDS`・「ukadoc の固定名だけ・各行に ukadoc の URL」）。`OnKeyPress` は ukadoc の固定名なので、表に 1 行足す形になる。
- マウスのイベントは、返事が無ければ次の段へ進む作りで送っている（`crates/areka-kanade/src/schedule/mouse_events.rs`）。キーにも、近い形が使える見込み。
- 暫定の設計の記録: `doc/ideas/novel-play-flow-provisional/provisional-design.md`（`OnKeyPress` の節）。**正式な設計ではない。**

## Desired Outcome

- ゴーストの窓がアクティブな時にキーを押すと、SHIORI に `OnKeyPress` が GET で届く。Reference は正典の並び。
- 返った台本は、ふつうの返事と同じに再生される。
- 文字を打つ所（入力ボックスなど）にキーを取られている間は、送らない。
- areka が自分で使うキー（今は在れば）と、SHIORI へ送るキーの線が、決まっている。Alt+Enter は、areka が自分で使わずに送る。

## Approach

- **areka の立場**（開発者 2026-11-20）: 「イベントを投げて、さくらスクリプトが返ってきたら、再生する」という仕組みを壊さない。キーを押した時に何をするかは、areka が決めない。イベントを投げ、SHIORI が返した台本を再生する。
- wintf のキーの入力を、マウスと同じ道（入力 → kanade → GET）で SHIORI へ運ぶ。値は、押された時点が分かるので、押し出す（「変わった時点が分かるならプッシュ」の決まり）。
- 意味は ukadoc から輸入する。Reference0 の綴り（Enter・矢印・ファンクションキーなど、キートップに字が無いキー）と、押しっぱなしの繰り返しは、ukadoc で読めないので、SSP の MCP で測ってから決める。

## Scope

- **In**: キーの入力を kanade へ運ぶ道・`OnKeyPress` の GET と Reference0〜4・許可した名前の表の 1 行・入力ボックスなどにキーを取られている間の扱い・網羅台帳の行と `doc/COMPAT_ARCHITECTURE.md` の行・決定論のテスト。
- **Out**: 舞台の窓（`shell-multi-compose`）でのキーの入口（舞台ができた時に、同じ道へつなぐ）・キーを押している間だけの早送り（pasta の取り決めの「作らないもの」）・コントローラー・IME の入力の途中の字・`OnKeyPress` の兄弟のイベント（在れば、要件の段で ukadoc を引き直して決める）。

## Boundary Candidates

- キーの入力の受け口（`crates/wintf/src/ecs/window_proc/keyboard.rs` と、areka の入力の係）。
- kanade の伝言と予定（`crates/areka-kanade/src/msg.rs`・`schedule/`）。
- イベントの名前の表（`crates/areka-kanade/src/schedule/events.rs`）。

## Out of Boundary

- 入力ボックス・メニューのキーの扱いは変えない。
- マウスのイベントの段の列は変えない。

## Upstream / Downstream

- **Upstream**: 待つ物は無い。同じファイル（kanade の `msg.rs`・`schedule/`・入力の係）を触る spec と、同じウェーブに置かない。
- **Downstream**: pasta のノベルの設定の画面の入口（キーボード）・`shell-multi-compose`（舞台の窓のキーの入口・窓の形の切り替えを SHIORI へ任せる）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `inputbox-user-input`（文字を打つ所）・`mouse-hover-event`（同じ入力の係）・`sakura-raise-directive`（一緒に起票した正典の拾い残し）・`shell-multi-compose`。

## Constraints

- 区分 A（開発者が起票を承認した）。正典のイベントで、`play` の命令ではない（「`play` に関わる物は v0.3.0 より後」の対象外）。置く波は、棚卸で決める。
- 規模の見立て: S（4〜5 タスク）。Opus。
- 既定の動きを変えない（開発者 2026-11-20「ukadoc既存の動作に影響が出る決定は許容できない」）。`OnKeyPress` に答えないゴーストでは、何も起きない。
- 失敗を黙って捨てない。メッセージボックスは出さない。
