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
- `crates/areka/src/readme/destination.rs` の `link_destinations` に付けた `#[cfg_attr(not(test), allow(dead_code))]` は、本 spec が本番の呼び手になったら外す（`link-context-copy` が先に外していれば何もしない・`open-external-tags` の完了時の申し送り・2026-10-06）。
- 段: 優先（バルーン関係）。規模の見込み M（8〜12）。

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- **前提の変化**:
  - `wintf-tooltip` が着地した。`crates/wintf/src/ecs/tooltip/` に、窓ごとに範囲を登録する口（`register`・`update`・`unregister`）、マウスが止まったときの知らせ（窓に付ける `OnTooltip` と `TooltipNotice::TurnStarted`）、後から文字を渡す口（`supply_text`）がある＝本 brief の「動的な表示の口」はこれ。areka の本体はまだどこからも使っていない。
  - 範囲は窓の中の論理の単位で渡す。選択肢の当たりの行（`crates/areka-emo-text/src/choice.rs` の `ChoiceHitRow`）は窓の物理ピクセル＝換算が要る。当たりの行は描き直しのたびに作り直されるので、範囲の差し替えをどこで行うかが設計の要になる。
  - `wintf-tooltip` の決まり: 重なった範囲は後から登録した方が勝つ。出ている間も別の範囲へ入れば切り替わる。待ち時間は OS の設定。
  - kanade は `schedule/events.rs` 793 行・`msg.rs` 926 行・`actor.rs` 900 行・`schedule/mod.rs` 955 行。SHIORI リソースを起動の後に尋ねる道は `actor_resources.rs`（メニュー用）にあるが、Reference を付けて尋ねられるかは設計で確かめる。
- **触るファイル**:
  - `crates/areka/src/input_events/` の新しいファイルと `mod.rs`、範囲の差し替えの置き場所（`crates/areka/src/emo2_boot/frame/scale_text.rs` の提示の後の見込み）。
  - `crates/areka/src/readme/destination.rs`（`link-context-copy` が印を外していれば触らない）。
  - `crates/areka-kanade/src/{schedule/events.rs, schedule/resources.rs, actor_resources.rs, msg.rs, actor.rs, schedule/mod.rs, lib.rs}`（2 つのイベントと `balloon_tooltip` の問い合わせ）。新しい入力は新しいファイルへ置く。
  - 網羅台帳 `shiori.toml` の 3 行（`OnChoiceHover`・`OnAnchorHover`・`balloon_tooltip`。どれも持ち主が空）・`doc/COMPAT_ARCHITECTURE.md` §8。
  - emo-text には触らない見込み＝emo-text にファイルを足さない。wintf も触らない見込み。
- **規模**: M（10〜13 タスク）。
- **先に要るもの**: `link-context-copy`（その前に `range-choice-tag`・`anchor-tag-canon`）。`wintf-tooltip` は着地済み。
- **優先度の区分**: A（開発者の 10-05 の問い「マウスホバーって出せる？ URL とか」）。
- **要件定義のモデル**: Fable（答えが後から届く・止まっている間に範囲が作り直される・ホバーのイベントに返ったトークの扱いは開発者に聞く分かれ目）。
- **分割の案**: 切らない。20 に近づいたら「ツールチップ（`balloon_tooltip` と既定の行き先）」と「2 つのイベント」に分けられる。
- **見つけた穴・古くなった記述**: 穴は無い。`mcp-kanade-tools`・`farewell-talk-status`・`mcp-shiori-query`（kanade の同じファイル）とは同じウェーブに置けない。

## 2026-10-10 棚卸㉓の申し送り

- **実機の確かめに 1 項目足す: 主画面より上に置いた画面でツールチップを出す**。完了 `wintf-tooltip` は、ツールチップの位置を OS へ渡すときに 2 つの座標を 16 ビットずつに詰める（`crates/wintf/src/ecs/tooltip/os.rs` の `make_lparam`・負の座標は 2 の補数のまま）。OS が符号付きで読むかは実機でしか決まらない。同 spec の `tasks.md` の Implementation Notes は、3.1 で「負の座標の画面で出す確かめを入れる」と書き、6.1 で「y が負の画面が無いので負の y の詰め方は未確認」と残した。
- areka からこの口を最初に使う spec（本 spec か `shell-tooltip`）の実機の確かめで、主画面の上に画面を置き、そこでバルーンのツールチップの位置が合うかを見る。ずれたら wintf の側の不具合として起票する（本 spec の中では直さない）。
