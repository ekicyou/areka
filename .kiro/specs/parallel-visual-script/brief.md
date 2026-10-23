# Brief: parallel-visual-script

> 2026-10-21 `/kiro-discovery` で起票。開発者「マウスホバーの方向性」の 2 つ目:「トークを止めない即時再生モードでさくらスクリプトを割り込ませる。理論上はさくらスクリプトは並走再生いくらでも出来る、が、やれれば良いというものではない気がする。→多分未起票。さくらスクリプトの冒頭に特定のタグがあるとき、これまで再生していたスクリプトを『止めず』、スクリプト再生を並走させる」。1 つ目（シェル定義だけでホバーのアニメ）は起票済みの `shell-hover-animation`。コードは「何の定義か」で指す。着手時に引き直す。

## Problem

ゴーストがマウスの動き（`OnMouseEnter`・`OnMouseMove` など）に見た目で応えたいとき、返事の台本は今のトークを置き換えてしまう（正典・完了 `input-events` の DD-IE-2）。だからゴーストはトークの最中は返事を控えるしかなく、「話しながら頭をなでられて表情だけ変える」「話している最中に × を赤くする」が書けない。ゴースト「窓際のぱすたさん」は `Status` が talking の間は `OnMouseEnter`／`OnMouseLeave` に何も返さない（返すとお別れの演出などを上書きする）。

`shell-hover-animation` はシェルの書き方だけでこれを解くが、シェルに書ける動きに限られる。SHIORI の判断（好感度・時刻・状態）で見た目を選びたいゴーストには、台本で「トークを止めずに見た目だけ替える」手段が要る。

## Current State

- **正典に並走は無い**（ukadoc を引いた・2026-10-21）。近いものは 2 つだけで、どちらも並走ではない:
  - PLUGIN/2.0 の応答ヘッダ `ScriptOption: nobreak`＝「現在喋っているトークを中断せず最後まで喋った後で指定したスクリプトを実行する」（`https://ssp.shillest.net/ukadoc/manual/spec_plugin.html`）＝待ち行列であって並走ではない。
  - `\t`（タイムクリティカルセクション）は逆向き＝区間の間マウス系の通知を止める（`sakura-time-critical`）。
  - よって本 spec は ukadoc に無い **areka 独自の書き方**。SSP は知らない `\![…]` を読み飛ばすので、同じ台本を SSP で流すと普通のトーク（置き換え）になる＝壊れはしない（要件の段で SSP の MCP で測る）。
- **置き換えの決め所**（kanade）: `schedule/events.rs` の `value_replaces_active_talk`（`OnSecondChange` 以外は置き換え）と `schedule/steady.rs` の `on_reply`（返事が来たら新しい `TalkId` を振って枠を上書きし `StartTalk` を出す・ログの語は「単一 slot 置換」）。トークの枠は `schedule/types.rs` の `Phase::Steady{talk: Option<ActiveTalk>}` の 1 つだけ。待ち行列は設計で禁じている（`steady.rs`「StartTalk・キュー・中断を一切行わず破棄」＝`OnSecondChange` の場合）。
- **再生の枠**（ghost）: `crates/areka-ghost/src/dispatcher.rs` の `DispatcherState{active: Option<ActiveTalk>}`＝「同時に 1 本だけ再生する単一 slot」。`on_start` は前のトークを `close_active_if_any` で閉じてから新しいトークの糸を立てる。刻み（`on_tick`）も終わりの知らせ（`on_done`）もこの 1 本だけに配る。
- **台本から再生まで**: `areka-sakura/src/drive.rs` の `spawn_talk` がトークごとに糸を 1 本立て、`areka_parsers::sakura::parse` → `compile::compile`（`CueSheet`）→ `dola::cue::CuePlayer` を刻む。`compile.rs` は空でない台本の頭に必ず `ClearAll`（バルーンを消す）を置く。`CuePlayer` は受け身の作りなので、いくつ同時にあってもよい。
- **見た目の受け手**: 出来事は再生中のトークの `CuePlayer` に登録した受け手（`emo2_boot/mod.rs` の sink の並び）すべてに配られ、受け手が名前で拾う。`\s[n]` は `CueCommand::Emote` を seriko が、`\![bind,…]` は `Custom("bind")` を seriko の `handle_message` が拾う。seriko の受け箱は 1 つの FIFO で、どのトークから来た出来事も同じに当てる。
- **1 本のトークを前提にしている所**: seriko の `looper.rs` の `observe_cue`（出来事の `at` からトークの時間の起点を見積もり、`ClearAll`／`Clear` で talk の流れを戻す）・`emo2_boot/talk_lifecycle.rs` の `BalloonLifecycleSink`（`begin_talk` と sink の破棄を TalkStarted／TalkEnded とみなし「前の TalkEnded の後に次の TalkStarted」を約束）・kanade の `current_talk_id`・`TalkDone` の id の突き合わせ（知らない id は `error!` で捨てる）。
- **まだ受け手の無いタグ**: `\i[n]`（`seriko-script-triggers` が作る）・`\![anim,…]`（その後の spec）・音のタグ（`\_v`・`\8`・`\![sound,…]`＝鳴らす所が無い）。今は読み手で `Raw` か受け手の無い `Custom` になり、`debug` の記録だけで捨てられる。

## Desired Outcome

- 台本の冒頭に areka 独自の印のタグがあるとき、その台本は今のトークを止めずに**並走**して再生される。印の無い台本の振る舞いは 1 つも変えない（置き換えのまま）。
- 並走の台本がしてよいのは**見た目と音だけ**（2026-10-21 開発者の答え）＝面の切り替え（`\s`・`\0`/`\1`/`\p[n]` のスコープ）・`\![bind,…]`・`\i`・`\![anim,…]`・音・時間の待ち（`\w`・`\_w`）。文字・バルーン・選択肢・クリック待ち（`\x`）・`\e` 以外の終わり方など、バルーンやトークの流れに触るタグは並走の中では効かせず、**読み捨てて記録に出す**（黙って捨てない）。これで「同じバルーンを 2 本が書く」「どちらのクリック待ちか」「どちらが閉じるか」を最初から起こさない。
- 並走の本数は**縛らない**（2026-10-21 開発者の答え）。届いた分だけ並べて流す。同じ面や同じ bindgroup を複数の台本が書いたときの決まり（後から当たった方が勝つ、など）は要件の段で決める。
- 並走の台本は主のトークの枠・`Status`・`TalkDone` の突き合わせ・バルーンの生き死にに一切触らない。主のトークが終わっても、置き換わっても、並走は自分の終わりまで流れる（終了・ゴーストの切り替えのときの止め方は要件の段で決める）。
- `\i`・`\![anim,…]`・音は、それぞれの受け手を作る spec が着地していれば並走の中でも効く。本 spec は「並走で流して受け手へ届ける」所までを持ち、受け手そのものは作らない。
- 作者向けの説明と `doc/COMPAT_ARCHITECTURE.md` の「areka が決めた点」への登記。

## Approach

開発者の指示どおり「台本の冒頭の印のタグ」で並走を選ぶ。作りの骨格（要件・設計の段で確かめる）:

- **選り分け**: kanade の `on_reply`（と SSTP・`\![raise]` など `StartTalk` を出すほかの所のうち、並走を許す所）で、台本の冒頭の印を見て「並走」の出口へ分ける。枠（`Phase`）は使わない。印の読み取りは台本の読み手で 1 か所に（`StartTalk` に「並走」の旗を持たせるか、別の命令にするかは設計で）。
- **並走の再生**: dispatcher に、単一の枠とは別の「並走の組」を持たせ、各並走に自分の `CuePlayer` を刻ませる（`CuePlayer` は受け身なので並べられる）。終わったら組から外すだけで kanade へは知らせない（または並走専用の知らせ）。
- **見た目と音だけに絞る**: 台本を読んだ後・`CueSheet` に積む前に、許したタグの出来事だけを残す。並走の `CueSheet` には頭の `ClearAll` を置かず、バルーンの受け手（文字・lifecycle）には配らない（受け手の組を分ける）。
- **seriko の 1 本前提**: 並走の出来事が主のトークの時間の起点の見積もり（`observe_cue`）と `talk` の流れを乱さないようにする（出来事に「並走から」を載せて見積もりから外す、など）。

採らなかった形:
- **待ち行列（`nobreak` 型）**: 主のトークの後で流すので「トークの最中に見た目を替える」を解かない。
- **何でも並走（文字も）**: 開発者の答え（見た目と音だけ）。バルーンの持ち主が 2 本になり、決めることが大きく増える。
- **応答ヘッダで選ぶ（`ScriptOption` 型）**: 開発者は台本の冒頭のタグを指定した。ヘッダは SHIORI ごとに書けるかどうかが分かれる。

## Scope

- **In**: 冒頭の印のタグの読み取り（名前は要件の段で決める）／並走と置き換えの選り分け（kanade）／並走の再生の組（dispatcher・トークの糸）／見た目と音だけに絞る選り分けと、読み捨てたタグの記録／バルーンの受け手に並走を配らないこと／seriko の時間の起点の見積もりを乱さないこと／終了・切り替え・再読み込みのときの並走の止め方／決定論テスト（主のトークを止めない・並走が複数・主が先に終わる・並走が先に終わる・読み捨ての記録・印の無い台本は今どおり）／作者向けの説明と `doc/COMPAT_ARCHITECTURE.md` の登記。
- **Out**: `\i[n]` の受け手（`seriko-script-triggers`）・`\![anim,…]` の受け手（その後の spec）・音を鳴らす所（受け手の無い今は記録だけ）／シェルの書き方だけのホバー（`shell-hover-animation`）／SHIORI の `OnMouseEnter`／`OnMouseLeave`（`mouse-click-wheel-events`）／`\t` の区間（`sakura-time-critical`）／並走の台本に文字・バルーンを許すこと（開発者の答えで外した）／PLUGIN の `ScriptOption: nobreak`（待ち行列）。

## Boundary Candidates

- 印の読み取りと見た目・音だけへの選り分け（`crates/areka-parsers/src/sakura/`・`crates/areka-sakura/src/compile.rs`）。
- 並走と置き換えの選り分け（`crates/areka-kanade/src/schedule/{steady,events,types,mod}.rs`・`crates/areka-talk/src/lib.rs` の約束）。
- 並走の再生の組（`crates/areka-ghost/src/dispatcher.rs`・`crates/areka-sakura/src/drive.rs`・`crates/areka-ghost/src/sink.rs`）。
- 受け手の側の 1 本前提の手当て（`crates/areka-seriko/src/{actor,looper}.rs`・`crates/areka/src/emo2_boot/{mod,talk_lifecycle}.rs`）。

## Out of Boundary

- 受け手の新設（`\i`・`\![anim]`・音）。
- 当たり判定・マウスの出入り（入力の側）。
- dola の時刻の型（v0.2.0 の時刻の土台が持つ）。

## Upstream / Downstream

- **Upstream**: v0.2.0 の時刻の土台（`dola-interrupt-ticks` → `talk-anchor-interrupt-ticks`＝`drive.rs`・`dispatcher.rs` のトークの時計と起点を作り替える＝その後の形に並走を足す）・`talk-thread-count-growth`（トークの糸の数＝並走で糸が増える前に片付けておく）・`mouse-click-wheel-events`（`OnMouseEnter`／`OnMouseLeave` を送る＝ホバーで使う入口）。`seriko-script-triggers`（`\i`）は無くても着手できる（着地していれば並走の中で効く）。
- **Downstream**: ホバーに見た目で応えるゴースト（ぱすたの × を SHIORI 側で書く形）・`\![anim,…]` と音の受け手を作る後の spec。

## Existing Spec Touchpoints

- **Extends**: なし（新しい spec）。
- **Adjacent**: `shell-hover-animation`（同じ困りごとのシェルの側）・`sakura-time-critical`（マウス系の通知を止める側・`steady.rs` を触る）・`talk-fast-forward`（`drive.rs`・`dispatcher.rs`）・`talk-anchor-interrupt-ticks`（同）・`seriko-talk-clock-fidelity`（seriko の時間の起点の見積もり）・`seriko-script-triggers`（`\i`）・`script-security-level`・`script-impact-tiers`（台本の出どころ・`StartTalk` の約束）・`multi-ghost-runtime`（dispatcher はゴーストごと）・`audit-kanade`・`audit-seriko`（v0.1.0 の後の隙間の見直し・同じファイル）。

## Constraints

- 時期: v0.4.0（2026-10-21 開発者の答え・`shell-hover-animation` と同じ版）。v0.2.0 の時刻の土台の後。
- ukadoc に無い areka 独自の書き方＝作者に向けた公開の約束。印の名前は正典の `\![…]` と衝突しないこと・SSP が読み飛ばすこと（ukadoc で引き直し、SSP の MCP で測る）。
- 失敗を黙って捨てない（読み捨てたタグ・効かない印の位置は記録に出す）。
- 1 フレーム遅らせる解は取らない（開発者の方針）。
- 印の無い台本の振る舞いと、正典の置き換え（DD-IE-1／DD-IE-2）は変えない。
- 1 ファイル 1,000 行の上限（`steady.rs`・`dispatcher.rs`・`looper.rs` は足す前に行数を確かめる）。

## 要件の段で決めること（議題の候補）

1. **印の名前と形**: 例 `\![enter,parallelmode]`（`nouserbreakmode` の語の型）・`\![set,parallel]`・`\![parallel]` など。冒頭以外に書かれたときの扱い（効かせない＋記録、を推す）。
2. **見た目と音の範囲の線**: 許すタグの一覧（`\s`・スコープ切り替え・`\![bind]`・`\i`・`\![anim]`・音・`\w`/`\_w`）。`\![move]`・`\![set,zorder]` など窓に触る指令、`\![raise]`・`\![embed]` などほかの台本を呼ぶ指令をどちらに入れるか。
3. **書き合いの決まり**: 本数を縛らないので、同じスコープの面・同じ bindgroup を主のトークと並走、または並走どうしが書いたとき（後から当たった方が勝つ、を推す）。主のトークが後で面を替えたら並走の面は上書きされてよいか。
4. **止め方**: ゴーストの終了・切り替え・再読み込み・利用者のクリックでの中断（ユーザーブレーク）のとき並走をどうするか。主のトークが置き換わったときは止めない（推し）。
5. **どの入口で許すか**: SHIORI の返事だけか、SSTP・MCP の `sakurascript`・`\![raise]` の結果も並走できるか。`OnSecondChange` の返事（今はトーク中は捨てる）に印があれば並走させるか。
6. **SSP で測ること**（SSP の MCP）: 印のタグを SSP がどう扱うか（読み飛ばしてほかを生かすか）。

## 2026-10-21 起票時の見立て

- **規模**: M（12〜16 タスク）。20 を超えるなら「受け手の側の 1 本前提の手当て」を切り出す。
- **優先度の区分**: A（開発者が起票を頼んだ）・段＝その他。
- **要件定義のモデル**: Fable（ukadoc に無い書き方をゴーストの作者へ新しく約束する＝先例の無い公開の契約・kanade と dispatcher の「単一の枠」という前提を崩す）。
