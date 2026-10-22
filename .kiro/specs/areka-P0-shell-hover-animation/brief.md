# Brief: areka-P0-shell-hover-animation

> 2026-10-20 `/kiro-discovery` で起票。開発者「『ぱすた開発』と相談して仕様を決めてみて。実は、シェルアニメーションに『マウスホバーが発生した時』だけ発動させたい要望がある。たとえば、リージョン『Head』に乗ってるときにイヤイヤするとか、パスタではクローズボタンに反応させるとか。対応時期はdola関係が落ち着いてから」。コードは「何の定義か」で指す。着手時に引き直す。

## Problem

マウスが当たり判定に乗っている間だけ見た目を変える（頭に乗るとイヤイヤする・× の印に乗ると赤くなる）には、今は SHIORI を通すしかない。正典（ukadoc）の SERIKO の interval（`sometimes`・`rarely`・`random`・`periodic`・`always`・`runonce`・`never`・`yen-e`・`talk`・`bind`）にマウスの語は無く、SSP でシェルだけでマウスに応じられるのはカーソルの形（`mouseup*`・`mousedown*`・`mousehover*`・`mousewheel*`）だけ。

SHIORI を通す形には、トークの最中に動かせないという穴がある。マウス系のイベントへの返事は今のトークを置き換える（正典・生きている決まり 10）ので、ゴーストはトークの最中は返事を控えるしかない。ゴースト「窓際のぱすたさん」は × の印に乗ると `OnMouseEnter` で `\![bind,閉じる,赤,1]`、離れると `OnMouseLeave` で `0` を返して × を赤くするが、`Status` が talking の間は何も返さない（返すとお別れの演出などを上書きしてしまう）。開発者の困りごとは「トーク中に切り替えができない」こと＝トークを止めずに見た目だけ替えたい。

## Current State

- ぱすたのシェル（2026-10-20 「ぱすた開発」のセッションが答えた事実・`project/pasta-in-windows/ghost/pasta-in-windows/shell/master/`）:
  - 当たり判定 `collision20,430,22,476,54,閉じる`（surface1000 と、同じ配置のもう 1 つの surface の 2 か所）。
  - 赤い × は `animation1500.interval,bind`・`animation1500.pattern0,overlay,1510,0,0,0`（surface1510＝キャンバス全面で赤い × だけ不透明な 1 枚絵）。ID 1500 は まばたき 1400 と うで 1600 番台の間（重なりの順の都合）。
  - descript.txt `sakura.bindgroup1500.name,閉じる,赤`（既定は OFF）。
  - したいこと＝乗っている間ずっと赤い ×（ループするアニメではない 1 枚絵）・離れたら元の灰色の × へ。
- interval の読み手: `crates/areka-parsers/src/shell/decode.rs` の `normalize_interval`。自分の形を持つのは `bind`・`random,K`・`bind+random,K`・`runonce`・`talk,N`・`periodic,S` だけで、ほかの語は原文のまま `Interval::Other` に入る（`always` も）。知らない語（例 `hover`）は失敗せず `Other` に残り、seriko の表（`crates/areka-seriko/src/table.rs` の `AnimationTable::from_world_and_films`）が `debug` の記録を 1 行出して表に入れない＝既定の記録の段では見えない。
- seriko の動かし方: `crates/areka-seriko/src/looper.rs` の `LoopRuntime::on_tick`（16 ms の刻み）。`runonce`・`periodic`・`talk` は `trigger.rs` の `Armed`（`hide()`・`show()` を持つ）で、入れ子の部品は `parts_trigger.rs` が同じ `Armed` を使う。面が替わると `on_surface_changed` が武装を捨てる。
- 外からの知らせは seriko の受け箱 `SerikoMsg`（`crates/areka-seriko/src/actor.rs`＝`Cue`・`Tick`・`Replace`・`Stage`・`Close`）だけを通る。ホバーに一番近い前例は `Stage`（UI の側からの点け消し・`SerikoSink::send_stage` → `on_stage` → `LoopRuntime::refresh` → `Armed::hide`／`show`）。seriko は dola の時刻表でアニメを回してはおらず（自前の `timeline.rs`）、dola からは台本の出来事（面の切り替え・`\![bind]`・`talk` の文字）だけを受ける。
- 着せ替えの点け消しは台本の `\![bind]` を seriko の `handle_message` が名前で拾う所だけ（`crates/areka/src/emo2_boot/consumer_ledger.rs` の登記と対）。
- 当たり判定の解決: `crates/areka/src/input_events/mod.rs` の `on_char_pointer_moved` → `MouseWiring::resolve_region` → `emo2_boot/hit_region.rs` の `resolve_hit_region` → `areka_emo_compose::hit_region_in`（画家の順）。**当たり判定の出入りを追う所は今は無い**（間引き `throttle.rs` の `last_sent_region` は送るかどうかを決めるだけ）。間引きは止まっている間の移動を落とすので、出入りは間引きの前で決める必要がある。窓から出たことは wintf の `PointerLeave` の印で分かる（`shell_box_handler.rs` の `clear_box_hover_on_leave`・`balloon_exit.rs` が前例）。
- `mouse-click-wheel-events`（v0.1.0）が窓ごとに「今どの当たり判定の上か」を追う所（案: `crates/areka/src/input_events/hover_region.rs`）を作り、`OnMouseEnter`／`OnMouseLeave` を送る予定。`shell-mouse-cursor`・`shell-tooltip` もそれを使う。
- `\i[ID]`・`\![anim,…]` は未実装（`\i` は `seriko-script-triggers`・`\![anim,…]` はその後の spec）。

## Desired Outcome

- シェルの書き方だけで、マウスが名前の付いた当たり判定に乗っている間だけ動くアニメーションを書ける。トークの最中でも、トークを止めずに見た目が替わる。
- 2 つの書き方を持つ（2026-10-20 開発者の答え「両方」）:
  1. **アニメーションの interval に語を足す**（例 `animation1500.interval,hover,閉じる`）。動く絵（イヤイヤ）にも 1 枚絵にも効く。
  2. **着せ替えの組（bindgroup）に「乗っている間だけ ON」を足す**（例 `sakura.bindgroup1500.hover,閉じる`）。ぱすたの今の作り（`interval,bind`＋bindgroup）をそのまま生かせる。
- 乗っている間の動きを書き分けられる（2026-10-20 開発者の答え「両方を書き分け」）＝**乗っている間ループ（離れたらすぐ止めて消す・1 枚絵なら乗っている間ずっと出る）**と、**乗るたびに 1 回だけ再生**。
- ぱすたが × の赤をシェルだけで書け、トークの最中も × が赤くなる。
- SSP は知らない語を読み飛ばす（動かないだけで壊れない）。areka が何を足したかは `doc/COMPAT_ARCHITECTURE.md` の「areka が決めた点」に登記する。

## Approach

開発者は 3 つの案（A シェルに宣言する areka 独自の書き方／B ゴーストが `OnMouseEnter`・`OnMouseLeave` で `\i`・`\![anim,stop]` を返す／C 両方）から A を選んだ（2026-10-20・「ぱすた開発」のセッションで本人が答えた）。B はトークを置き換える穴が残るので、困りごとを解かない。

作りの骨格（要件の段で確かめる）:
- 入力の側: `mouse-click-wheel-events` が作る「窓ごとの今の当たり判定」の追跡を使い、変わったときに seriko の受け箱へ新しい知らせ（`SerikoMsg` に足すか `StageNote` に足す・`SerikoSink::send_*` は `send_stage` に倣う）を送る。窓から出たら「どこにも乗っていない」を送る。SHIORI の `OnMouseEnter`／`OnMouseLeave` と同じ出入りを見る（トークの最中も止めない）。
- 読み手: interval の新しい語を `Interval` に足す（`Other` のままにしない）。bindgroup の新しい鍵は descript の読み手へ。
- seriko: 表に新しい起動の形（`LoopTrigger` の腕）を足し、`Armed` に似た点け消しを当たり判定の名前で持つ。bindgroup の側は「乗っている間 ON」を着せ替えの状態に重ねる。
- 書き分けの語の形（例 `hover` と `hover+runonce`・`hover` と `hoveronce` など）は要件の段で決める。`seriko-interval-combinations` が決める `+` のつなぎ方とそろえる。

## Scope

- **In**: interval の新しい語（ループ・1 回だけの書き分け・当たり判定の名前の引数）の読み取りと動き／bindgroup の「乗っている間 ON」の鍵の読み取りと動き／入力の側から seriko への出入りの知らせ（窓から出たときも）／トークの最中も動くこと／知らない形・名前の無い当たり判定の記録（`warn` など・黙って捨てない）／決定論テスト（出入り・窓の外・面の切り替え・トークの最中・2 つの書き方）／`doc/COMPAT_ARCHITECTURE.md` の登記・作者向けの説明。
- **Out**: SHIORI のイベント `OnMouseEnter`／`OnMouseLeave`（`mouse-click-wheel-events`）・`OnMouseHover`（静止の判定）／カーソルの形（`shell-mouse-cursor`）・ツールチップ（`shell-tooltip`）／台本の `\i[ID]`（`seriko-script-triggers`）・`\![anim,…]`／`collisionex` の形の当たり判定を読むこと（`collisionex-regions`・読めるようになれば同じ名前で効く）／バルーンの上のホバー（`balloon-link-hover`）／乗ってから何ミリ秒で発動、のような待ち（要るなら要件の段で開発者に聞く）／ぱすたのシェルの書き換え（ぱすたの持ち物）。

## Boundary Candidates

- 入力の側の出入りの知らせ（`crates/areka/src/input_events/` の追跡を使う所・`PointerLeave` の後始末）。
- 読み手（`crates/areka-parsers/src/shell/{model,decode}.rs` の interval・descript の bindgroup の鍵）。
- seriko の表と動き（`crates/areka-seriko/src/{table,looper,looper_trigger,trigger,parts_trigger,actor,state}.rs`）。

## Out of Boundary

- kanade と SHIORI のイベント（出入りの SHIORI への送り出しは `mouse-click-wheel-events`）。
- 当たり判定の解決そのもの（`hit_region_in`・今の解決をそのまま使う）。
- wintf（`PointerLeave` は既にある）。

## Upstream / Downstream

- **Upstream**: `mouse-click-wheel-events`（v0.1.0・窓ごとの今の当たり判定の追跡）・`seriko-interval-combinations`（v0.1.0・`+` のつなぎ方と seriko の `table.rs`・`looper.rs`）・`seriko-script-triggers`（v0.1.0・同じファイル）・v0.2.0 の時刻の土台（`dola-interrupt-ticks` → `talk-anchor-interrupt-ticks`＝seriko の時計を替える）・`collisionex-regions`（v0.2.0・「Head」を `collisionex` で書くシェルに効くため）。
- **Downstream**: ぱすた（× の赤をシェルだけで書ける）・`\![anim,…]` を持つ後の spec（ホバーで点いたアニメを台本から止めるときの順）。

## Existing Spec Touchpoints

- **Extends**: なし（新しい spec）。
- **Adjacent**: `mouse-click-wheel-events`・`shell-mouse-cursor`・`shell-tooltip`（同じ追跡を使う）・`seriko-interval-combinations`・`seriko-script-triggers`・`seriko-trigger-lottery-gaps`・`seriko-talk-clock-fidelity`（seriko の同じファイル）・`balloon-link-hover`（バルーンのホバー＝別物）・`audit-seriko`（v0.1.0 の後の隙間の見直し・同じファイル）。

## Constraints

- 時期: v0.4.0（dola の整備の後・開発者「対応時期はdola関係が落ち着いてから」）。
- 1 フレーム遅らせて出入りを決めない（開発者の方針）。出入りは間引きの前で決める（間引きは止まっている間の移動を落とす）。
- 失敗を黙って捨てない（知らない形・書いた名前の当たり判定がどの面にも無い・は記録に出す）。
- ukadoc に無い areka 独自の書き方＝作者に向けた公開の約束。名前の付け方は ukadoc の用語の型でそろえる（「interval」「bindgroup」）。
- 1 ファイル 1,000 行の上限（seriko の `looper.rs` などは足す前に行数を確かめる）。
- 実機の確かめは画面のロックを外した机で（マウスを動かすテスト）。

## 要件の段で決めること（議題の候補）

1. **語の形**: interval の語（`hover,名前` を推す）と、1 回だけの書き分けの形（`seriko-interval-combinations` の `+` とそろえるか・別の語か）。bindgroup の鍵の名前（`sakura.bindgroupN.hover,名前` を推す）。当たり判定の名前を複数書けるか・名前を省いたら窓のどこでも、を許すか。
2. **面が替わったとき**: マウスが止まったまま面が替わり、下の当たり判定が変わったら出入りとみなすか（移動が無いので今の入力の側は気付かない）。
3. **台本の `\![bind]` と bindgroup の hover が食い違うとき**どちらが勝つか（例: 乗っている間に台本が OFF にした）。
4. **ループの形の止め方**: 離れたとき途中のコマで即消すか、1 周を終えてから消すか（推しは即消す＝ぱすたの × に合う）。
5. **SSP で測ること**（SSP の MCP）: SSP が知らない interval の語・知らない bindgroup の鍵をどう扱うか（読み飛ばしてほかの行を生かすか）。同じシェルを SSP で使ったときに壊れないことの裏付け。

## 2026-10-20 起票時の見立て

- **規模**: M（12〜16 タスク）。20 を超えるなら bindgroup の側を切り出す。
- **優先度の区分**: A（開発者が起票を頼んだ）・段＝その他。
- **要件定義のモデル**: Fable（ukadoc に無い書き方をシェルの作者へ新しく約束する＝先例の無い公開の契約を引く）。
