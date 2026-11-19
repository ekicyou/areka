# 暫定の設計の検証: element-animation-easing

- 日付: 2026-11-19。基にした main: `49b6f3ab5`。検証した物: 同じフォルダの `provisional-design.md`（以下「設計」）。
- 設計を書いた係とは別の目で、コードと上位の文書を自分で読んで確かめた。ビルド・テスト・起動はしていない。設計のファイルは直していない。
- 「口 1」「口 2」は設計の 4.3 の呼び名のまま使う（口 1＝実行時の値から終わりの値へ、長さと形で 1 回で始める。口 2＝走っている変化を全部、終わりの値にする）。

## 1. 判定

**条件つき GO。**

- 設計の芯（値を時間で動かす本体は dola の連続値の側にもう在り、本番の結線が 0 件）は、コードで確かめて当たっていた。口 1・口 2 も、dola の中では小さい足しで済む。
- ただし、土台を作る側（`window-timed-directives`）の brief に、その割り当てが 1 行も書かれていない。wintf の側に、設計が「在る・作らない」とした物の足りない所が 3 つ在る（2 章の X1）。
- 上位の決定を変える差し戻しは無い。取り決めの文面の割れが 1 つ在り（舞台が閉じている間の位置・大きさ・透明度を areka へどのタグで出すか）、答えで areka の単位の割り当てが変わる（X2）。

## 2. 重大な問題

### 2.1 この単位の中で閉じる物（設計の文を直せば済む）

**N1. 口 1 の「走っていなければ、変数の今の値」は、dola に無い。**

- どこで: 設計 4.3 の c・4.6 の末（「`cancel` の後に値を書く」）。
- 事実: dola が持つ値は、購読中の変数の最後の値（`crates/dola/src/runtime/subscription_manager/mod.rs:24` の `last_values`。読む口は無い）と、文書に書いた初期値（`crates/dola/src/compile/resolve.rs:371-376`）だけ。文書を使わない口 1 では、初期値も無い。
- なぜ困るか: 窓の位置は placement の持ち物で、ドラッグ・吸い付き・組にした窓の移動で、dola の外で変わる。透明度も、設計の 4.6 の末のとおり書く係がじかに書くと、dola の側の値が古くなる。次の口 1 が古い値から始めて、値が飛ぶ。
- 直す向き: 口 1 は「走っていないときの出発点」を、呼び手から引数で受ける（位置は placement から、その時に読む）。「すぐ替える」は、書く係がじかに書かず、口 1 の長さ 0 で通す（長さ 0 の区間は今の作りで通る。`crates/dola/src/runtime/timeline_manager/mod.rs:299-305`）。

**N2. R11（同じ終わりの値なら何もしない）を、口 1 の中に入れない。**

- どこで: 設計 2 章の R11・4.3 の c・仮定 A8。
- なぜ困るか: `play` の命令では要る（取り決め「命令の表」: 「表の命令は、同じ値をもう一度受けたら何もしない」）。正典のタグでは要らない見込みが高い（`\![set,alpha,50,--time=1000]` の途中に `\![set,alpha,50]` が来たら、すぐ 50 にするのが正典の字面）。設計 4.6 の末（「転換なしの変更が来たら、その変化を捨てて新しい値を置く」）と R11 は、終わりの値が同じときに、どちらが勝つかが書いていない。
- 直す向き: 口 1 は、いつも言われたとおり始める。「同じ値は何もしない」は、呼び手の側に置く。舞台の側は、`shell-multi-compose` の設計 4.3（状態の差だけを当てる）で、もう出ている。

### 2.2 単位をまたぐ物

**X1. 土台の割り当てが、作る側の brief に無い。wintf の側の足りない所が、設計の表から落ちている。**

- どこで: 設計 4.2 の表（「毎フレームの係と起床の旗 ｜ 在る ｜ 作らない」）・6 章の 1。`.kiro/specs/window-timed-directives/brief.md` の Boundary Candidates（dola も wintf の `ecs/dola` も挙げていない）・Constraints（M・12〜16）。
- コードで確かめた、土台に要る物:
  1. 口 1: `start_internal` は「名前で文書から引いてコンパイルする」所と一体（`crates/dola/src/runtime/facade.rs:99-108`）。113 行より後ろは、コンパイル済みの値だけを使う。前後に割って、手で組んだ 1 区間（`crates/dola/src/compile/types.rs:16-67`）を渡す入口を足せば済む。出発点は、今の `evaluate`（`timeline_manager/mod.rs:107-145`）で取れる。**小さい。**
  2. 口 2: `conclude_internal`（`facade.rs:519-542`）を、生きているインスタンスの全部に回すだけ。**小さい。** ただし、終わりの値が外へ出るのは、次の `tick` の差分である（`subscription_manager/mod.rs:140-146`）。
  3. **`DolaAnimator` に、書き換えの口が無い。** 在るのは `runtime(&mut self) -> &DolaRuntime`（`crates/wintf/src/ecs/dola/mod.rs:145-147`）だけで、包んだ後は `start` も `conclude` も呼べない。足す。
  4. **`tick_dola_animators` は、どのスケジュールにも載っていない**（`crates/` を名前で引いて、定義・再公開・テストだけ）。載せる順が要る。areka のフレームの係は `Update` の `update_typewriters` の後（`crates/areka-runtime/src/lib.rs:709`）。係の注釈（`mod.rs:166-167`「Update スケジュール先頭に配置」）のまま置くと、口 1・口 2 の結果が出るのが次のフレームになる（1 フレーム遅らせない、の決まりに反する）。「受け渡しを読む → 口 1・口 2 → `tick` → 値を書く係 → `visual_property_sync_system`」を 1 フレームの中に並べる。
  5. **始めた時の起床の旗。** 旗 `ANIM` は `tick` の中でしか立たない（`mod.rs:207-212`）。何も走っていない所へ口 1 で始めたとき、最初のフレームを起こす旗を、始めた側が立てる。
- なぜ困るか: 書かれていないと、`window-timed-directives` は brief の字面（「途中の値は、始まりの時刻と長さから毎フレーム計算する」）どおりに、等速の計算を `areka-runtime` に手書きする。舞台の転換とこの単位が、同じ物をもう 1 つ作る。
- 直す向き: 同 brief に 4 つ足す。⑴ Approach に「dola の連続値の側（`DolaRuntime`・`DolaAnimator`）の結線として作る。口 1・口 2 は、舞台のアクターからも使える置き場に置く」。⑵ Boundary Candidates に `crates/dola/src/runtime/facade.rs`・`crates/wintf/src/ecs/dola/mod.rs`。⑶ Downstream に `shell-multi-compose`・`talk-fast-forward`。⑷ 規模の見直し（設計 10 章の「＋5〜7」を足すと 17〜23 で、M を超える）。

**X2. 舞台が閉じている間の `play` の位置・大きさ・透明度・転換を受ける単位が、areka に無い（設計の P7 の指摘は当たり）。**

- 確かめた事実:
  - `shell-multi-compose` の設計は、捨てる側である（同 4.10: 「舞台が閉じているのに届いた舞台の命令（並び・位置・転換ほか）｜ 何もせず `warn!`」・同 要件 20: 「舞台が閉じている間の動き…は、1 つも変えない」）。
  - `window-timed-directives` の Scope In は、正典のタグだけである。
  - 受け口の設計（`doc/ideas/novel-play-flow-provisional/provisional-design.md`）は、`転換|trans|play,(alpha|pos|scale)|set,alpha` で引いて 0 件。
  - 上位の側: 骨子 2.7「舞台を開かない演劇は、デスクトップのゴーストのまま、1 行ずつ進む」・2.9 のフェーズ 1「アクターの窓の位置と透明度と大きさ（数値）」。pasta の暫定の設計（`novel-areka-contract/provisional-design.md` 6.1 の表）は、大きさと透明度を「フェーズ 1・areka へは `\![play,scale,…]`・`\![play,alpha,…]`、SSP へは `\![set,scaling,…,--time=…]`・`\![set,alpha,…,--time=…]`」と置く。
- なぜ困るか: フェーズ 1 の台本（舞台を開かない）を areka で動かすと、位置・大きさ・透明度の命令が、警告だけ残して全部捨てられる。
- 直す向き（どちらかを、取り決めの要件定義で決める）:
  - (a) 舞台が閉じている間は、pasta が areka へも SSP と同じ正典のタグで出す。areka に足す受け手は 0 で、`window-timed-directives` がそのまま受ける。取り決めの原則「正典に在る物は正典のタグで出し、`play` の命令は、正典に無い物だけにする」と合う。**推す。**
  - (b) areka が、デスクトップでも `play` を受ける。名前からスコープを引く表・転換の組・受け手が要り、持ち主の単位を決める（8〜12 タスクの見込み。机の上の数）。
- つながる物: `window-timed-directives` の brief の Out の 1 文「演劇では正典の `--time` を使わない、と pasta の側で決まっている」は、**決定の節にも骨子にも無い**（3 章）。(a) なら、この文は消す。

**X3. 「発火時刻」と「届いた時刻」の差は、1 フレームより大きい（仮定 A7 が違う）。**

- 事実: cue は 50 ms の格子で配られる（`crates/areka-ghost/src/ticker.rs:56-69`）。v0.2.0 の土台の後も、周期は変えない（`talk-anchor-interrupt-ticks` の brief の Approach「刻みの周期（50・1000・16 ms）は変えない」）。差は 0〜50 ms で、約 3 フレームぶん在る。
- なぜ困るか: 走っている変化へ割り込むとき、設計の形（古い側を、新しい側の開始時刻で評価する）だと、画面は、古い変化を 50 ms 先まで出した後で、発火時刻の値へ戻ってから、新しい変化の「50 ms 進んだ所」へ移る。300 ms のフェードを逆向きに割り込むと、最大で幅の約 3 分の 1 が 1 フレームで動く（机の上の計算）。取り決め「転換」の「今、画面に出ている絵（転換の途中なら、途中の絵）が出発点になる」と、字面でずれる。
- 直す向き（土台の設計で決める）: 出発点は「最後に出した値」、終わりの時刻は「発火時刻＋長さ」にする（値は飛ばない。終わりの時刻は正確。途中の速さだけが少し変わる）。N1 の「出発点を呼び手が渡す」形で作れて、dola は変えない。設計の形のまま受け入れる（字の遅れと同じ扱い）なら、A7 の文を「最大 50 ms ぶん」に直す。

**X4. 早送りの知らせと、口 1・口 2 の順と範囲。**

- どこで: 設計 4.5 の末・6 章の 6。`talk-fast-forward` の設計 4.5（順の決まり: 「起点を付け替えて 1 回刻む（合図を配る）→ その後で `time_synced(狙い)`」）。
- 困る所が 3 つ:
  1. 口 1（始める）と口 2（全部終わりへ）が、別々の線で UI スレッドへ届くと、順が入れ替わる。早送りで配られた転換が、終わりへ飛ばずに走る。**同じ 1 本の線で送る。**
  2. 口 1 に渡す「絶対の発火時刻」は、配る側（再生機の起点＋`at`）が渡す。受け手が見積もらない。`talk-fast-forward` は起点を付け替える（同 4.3・4.4）ので、受け手が古い起点で足すと、未来の時刻になる。`talk-anchor-interrupt-ticks`（起点は配る時に押す）・`talk-fast-forward`（起点の付け替え）・土台の 3 つで、この 1 文をそろえる。
  3. 口 2 の「入れ物の全部」は広すぎる。取り決め「並走」の「クリックの早送りは効かせない」・`window-timed-directives` の議題 3（待つ `\![move]` を飛ばすか）・v0.3.0 の `text-reveal-fade`（同じ係を使うなら巻き込まれる）が在る。口 1 に「どの流れが始めたか」の印を最初から持たせ、口 2 は印で選ぶ（設計 8 章の問 5 は、後で決める物でなく、口の形を決める物）。

**X5. v0.2.0 の時刻の土台の切り方しだいで、仮定 A2 が崩れる。**

- 事実: 連続値の側の時計は、呼び手が渡す `f64` の秒である（`facade.rs:294`）。本番の出どころになる `FrameTime` は、`dola::runtime::clock::now()`（QPC の秒）をフレームごとに置く（`crates/wintf/src/ecs/world/mod.rs:579`・`crates/dola/src/runtime/clock.rs:14-37`）。`dola-interrupt-ticks` の brief は、両方を同じ新しい型へ替える範囲に入れている。ここまでは、設計の A2 と合う。
- 危ない所: 同 brief は「storyboard の層は使い手が居ないので、型を替えるだけに留める」「20 を超えるなら、要件の段で storyboard の層を後ろへ切る」と書く。切られると、`FrameTime`（新しい型）と `DolaRuntime`（`f64` の秒）の物差しが割れたまま、土台の着手になる。
- 直す向き: `dola-interrupt-ticks` の brief に「storyboard の層に使い手が現れる（`window-timed-directives`）。後ろへ切るなら、切った残りを同 spec の前に置く」を申し送る。

## 3. 上位の決定との突き合わせ

- **転換の 4 つの決まり**（途中の絵が出発点・待たせない・早送りで飛ぶ・形の引数なし）は、設計の形で支えられる。出発点だけ、X3 の直しが要る。
- **`window-timed-directives` の Out の 1 文の根拠**（頼まれた確かめ）: pasta の `novel-areka-contract` のフォルダを `--time` で引いた。
  - 決定の節（`brief.md` の 201〜349 行）で当たるのは 311 行だけで、向きが逆である: 「フェーズ 1 の芯は、SSP 2.9.34 の実機で確かめた…透明度と大きさの `--time`」。
  - 骨子は 0 件。
  - 在るのは経緯（`discovery-2026-11-19.md`）だけ: 1032 行「時間を付ける口は、転換に 1 本化する（正典の `--time` は使わない）」・1389 行「areka と合わせた決まりは『時間を付けるのは `！転換` だけ』」・1409 行「SSP では `\p[0]\![set,alpha,0,--time=300]`、areka では `\![play,trans,移動,300]\![play,alpha,さくら,0]`」。
  - 読み: 「areka へは `play` で出す」は、決定の節の表の最後の行と 312 行（「pasta が…タグへ直す」）から読めるが、文としては書かれていない。原則（210 行「正典を先に使う」）と並べると、割れて読める。設計の P7 の「文面どうしが読み分けられない」は当たり。
- **出し直し（P5）**: `shell-multi-compose` の設計の要件 12 は「走っている転換は、終わりの絵へ飛ばす」と書いていて、設計の P5 と同じ向き。X2 で (a) を取るなら、デスクトップの正典の透明度・倍率は「残る命令の表」の外で、出し直しでは戻らない（pasta が全部書くので困らない）。この 1 行を取り決めに足すと、読み違いが出ない。
- **高速再生（P6）**: 骨子 2.6 は「`\x` と瞬間の命令を出さない」。pasta の暫定の設計は、転換を「瞬間（組）」に置く。高速再生の 1 本は転換を打たないので、同じアクターの値は「すぐ替える」で上書きされる。別のアクターの走っている転換は続く。害は小さい。設計の「決まっていない」は当たりだが、口の形は変わらない。
- **`\i[ID]`（頼まれた確かめ）**:
  - 取り決めの頼みごとに在るのは `\i[ID]` だけである（`brief.md` 296 行）。`\i[ID,wait]` は、同フォルダを `ID,wait` で引いて、経緯の 1423 行にたとえとして出るだけで、頼みごとではない。
  - `\i[ID]` の持ち主は `seriko-script-triggers`（同 brief 52 行・`roadmap.md` 458 行で区分 A）。宙に浮いていない。まだ着地していない。
  - `\i[ID,wait]`・`\__w[animation,ID]`・`\![anim,…]` は、持ち主の brief が無い（設計 3.5 のとおり。`.kiro/specs/` の brief と暫定の設計を `ID,wait|__w[animation` で引いて、当たるのは `seriko-script-triggers`・この単位・完了済みの `areka-P0-seriko-trigger-intervals` だけ）。取り決めは要らないので、ノベルの妨げにはならない。

## 4. ほかの単位の暫定の設計との突き合わせ

| 相手 | 結果 | 直す向き |
| ---- | ---- | -------- |
| `shell-multi-compose`（継ぎ目 5） | **半分だけ乗っている。** 転換の値の決め方（同 4.7: 絶対の始まりの時刻と長さの関数・毎フレーム計算・途中の値が出発点・早送りは知らせで飛ばす）は、この土台と同じ向きで、別の時計も作っていない。前の絵を残して溶かす・アクターごと、も口 1 を 1 枚ごとの透明度に使えば乗る。ただし持ち主の言い方が違う: 同 6 章は「先に着地した側が作る」、同 4.1 は「`dola` は、この設計では触らない」。この設計は「`window-timed-directives` が作る」。dola の連続値の側、という語は向こうに無い | 持ち主を 1 つに決める（X1）。決まれば、向こうの「dola は触らない」は当たったままになる。向こうの 6 章の文を「`window-timed-directives` が作った口 1・口 2 を使う」に直す |
| 同・前提の妥当さ | `roadmap.md:1734` の Dependencies は `element-animation-easing` を挙げるが、向こうの暫定の設計は、この単位に「形の語彙と式」しか求めていない（同 4.7 の末・6 章）。等速で始めるなら、働きの前提ではない。設計 4.2 の読みは当たり | roadmap の Dependencies を `window-timed-directives` に替える。`roadmap.md:1165` のこの単位の「Dependencies: none」にも `window-timed-directives` を足す |
| 同・揺れとフラッシュ | 向こうは「時刻の関数で」動かすと書くだけで、土台を使うかは書いていない。同じ係を使うなら、口 2 の範囲に入る（X4 の 3） | 印で選ぶ形なら、どちらでも困らない |
| `talk-fast-forward`（継ぎ目 5） | 同じ土台を前提にしている（同 4.5: 転換の受け手が `time_synced` を拾って終わりへ飛ばす・同 6 章: 転換は壁の時計で測る）。別の物は作っていない。順と発火時刻は X4 | X4 の 1・2 |
| `parallel-visual-script` | 時間つきの指令は、区間に書ける物から外している（同 仮 4）。口 2 とぶつからない。**ただし** 同 233・263 行は、`\![anim]` の受け手を「`element-animation-easing`・`seriko-script-triggers`」に期待している。この単位は `\![anim]` を持たない（設計 3.5） | 向こうの設計を直す（`\![anim]` を区間に書ける物から外すか、持ち主を別に立てる）。取り決めが求めるのは `\i[ID]` だけ |
| 受け口 | `again` を束ねる所は作らず、各単位が自分のぶんを戻す形（同 140 行）。転換を終わらせるのは舞台の側で、この単位の持ち物は無い | 無い |
| `text-reveal-fade`（v0.3.0） | 設計 6 章の 7 は「dola の `apply_easing` の上に書く」を頼むが、`apply_easing` は今 `pub(super)`（`crates/dola/src/runtime/interpolator/mod.rs:131`）で、公開するのは後から着地するこの単位（設計 4.3 の b）。順が逆 | 公開の 1 行は、先に使う側が入れる |
| 継ぎ目 7（舞台の中を `play` で受けるか正典で受けるか） | `shell-multi-compose` の設計 8 章の問い 1 と、この設計の P7 は、同じ割れを、舞台の中と外から見ている | X2 と 1 つの問いにまとめる |

## 5. 設計の係の仮定のうち、危ない物

- **A7**（出発点は開始時刻で評価・差は 1 フレームより小さい）: 違う。X3。
- **A2**（v0.2.0 の土台の後で、物差しが 1 本）: `dola-interrupt-ticks` が storyboard の層を後ろへ切ると崩れる。X5。なお、今も文字の層の `TalkClock` は QPC の秒で起点を見積もっている（`crates/areka-runtime/src/talk_clock.rs:16`）ので、「直せない」は「正確には直せない」である。
- **A8**（同じ終わりの値は、長さや形が違っても何もしない）: `play` には当たるが、正典のタグには当たらない見込み。N2。
- **A4・R2**（見本集の 30 語を、dola の名前つきの式へ写すだけ）: Back と Elastic は、曲線が見本集と違う見込み。dola の式は AHEasing の系統である（`interpolator/mod.rs:185-188` の注釈）。brief の Desired Outcome「Web の見本集…で作った値がそのまま使える」に反しうる。要件の段で、見本集の式と突き合わせる（この検証では、外のクレートの式も見本集の式も読んでいない。机の上の見立て）。
- **A3**（正典の `--time` の既定は等速）: ukadoc は形を書いていない。`window-timed-directives` の Desired Outcome は「台本『席替え』が、SSP と同じ見た目で動く」。SSP の形が等速でなければ、R7（書かなければ等速と同じ値）の「等速」が動く。決める前に SSP の MCP で測る（areka の決まり）。
- 設計 4.4 の「既に在る受け渡しの道（move・zorder と同じ形）」: 道は在る（`crates/areka-runtime/src/lib.rs:507`・`frame/drain_resnap.rs:178`）。口 1 と口 2 を別の道に分けない（X4 の 1）。
- 設計 4.2 の「変数の値を、窓の…倍率へ書く係」: 倍率の席は整数の比である（`crates/areka-emo-compose/src/scale.rs:59-63` の `ScaleRatio{num: u32, den: u32}`）。毎フレームの途中の値をどう入れるか、窓の大きさと字の描き直しが毎フレーム走るかは、`window-timed-directives` の持ち物で、この検証では読んでいない。

## 6. 差し戻しが要りそうな所

- **上位の決定（骨子・取り決め）の中身を変える差し戻しは、無い。**
- 取り決めの要件定義で閉じてほしい文面の割れが 1 つ在る（X2）。答えで areka の単位の割り当てが変わるので、ほかの文面の足しより先に決めたい。
  - 割れている文: `novel-areka-contract/brief.md`「原則」の「正典に在る物は正典のタグで出し、`play` の命令は、正典に無い物だけにする」／同「命令の表」の最後の行「舞台・並び・位置・重ね順・大きさ・透明度・転換・… ｜ `play` の命令」／同「フェーズ」の「フェーズ 1 の芯は…透明度と大きさの `--time`」／骨子 2.7「舞台を開かない演劇は、デスクトップのゴーストのまま、1 行ずつ進む」。
- areka の中で直す物（差し戻しではない）:
  - `window-timed-directives` の brief: Approach の「値は台本の時計で動かす」（字面どおり台詞からの相対の時計で動かすと、`\x` で止まり、同 brief の Desired Outcome「`\![moveasync]` で、台本を止めずに窓が移る」と合わない。設計 6 章の 2 の直しは当たり）・土台の割り当て（X1）・Out の 1 文（3 章）。
  - `roadmap.md:1734`・`:1165` の Dependencies（4 章）。`roadmap.md:1980` の `window-timed-directives` の Dependencies は `talk-anchor-interrupt-ticks` だけで、設計と合う。
  - `dola-interrupt-ticks` の brief への申し送り（X5）。
  - `parallel-visual-script` の暫定の設計の `\![anim]` の持ち主（4 章）。

## 7. 確かめた物と机の上だけの物

- **実行して確かめた物: 0。**
- **コードを読んで確かめた物**:
  - dola の連続値の側の全体: `facade.rs`（全部）・`timeline_manager/mod.rs`（全部）・`conflict_resolver.rs`（全部）・`subscription_manager/mod.rs`（全部）・`clock.rs`（全部）・`compile/types.rs`（全部）・`compile/resolve.rs:330-399`・`interpolator/mod.rs:1-215`。
    - 値は、コンパイル済みの区間と時刻だけで決まる。`start` の後で出発点を実行時の値へ差し替える所は無い（設計が「読んでいない」とした所。**無い、と確かめた**）。だから口 1 は要る。
    - `Cancel`／`Trim` は、古い側を新しい側の開始時刻で評価する（`conflict_resolver.rs:71-74`）。割り込みの方針の既定は `Conclude`（`crates/dola/src/storyboard.rs:51`）なので、口 1 が作るインスタンスは方針を明示する。
    - 値域（min／max）へ収める働きは、もう在る（`timeline_manager/mod.rs:34-56`）。透明度を 0〜100 に収めるのに使える。
  - wintf: `ecs/dola/mod.rs`（全部）・`world/mod.rs:555-620`（`FrameTime` の置き方と 13 本のスケジュールの順）・`world/tick_wake.rs` の `mark`・`ANIM`。
  - 結線が 0 件であること: `crates/` を `tick_dola_animators|DolaAnimator|DolaRuntime` で引いた。当たるのは dola・`wintf/src/ecs/dola/mod.rs`・`wintf/src/ecs/mod.rs`・`wintf/tests/` だけ。
  - areka: `areka-runtime/src/talk_clock.rs:1-80`・`lib.rs:709`（フレームの係の位置）・`areka-ghost/src/ticker.rs` の周期・`dola/src/cue/sink.rs`（全部）・`cue/runtime.rs:140-219`・`cue/command.rs` の `TalkCue`。
  - 文書: この単位・`window-timed-directives`・`dola-interrupt-ticks`・`talk-anchor-interrupt-ticks` の brief（全部）。`shell-multi-compose`・`talk-fast-forward` の暫定の設計（全部）。`parallel-visual-script`・受け口の暫定の設計と `seriko-script-triggers` の brief は grep だけ。pasta の骨子の 140〜214 行・取り決めの brief の 199〜349 行・pasta の暫定の設計の 216〜255 行・`discovery-2026-11-19.md` は `--time` の行だけ。`roadmap.md` はこの単位・`window-timed-directives`・`seriko-script-triggers` の名前の行だけ。
- **机の上だけの物**:
  - 口 1・口 2 の「小さい」は、コードの形からの見立てで、書いて確かめてはいない。
  - X3 の「幅の約 3 分の 1」は、式からの計算である。
  - X2 の (b) の 8〜12 タスクは、見立てである。
  - 見本集の 30 語と dola の式の差（外のクレート `interpolation` 0.3.0 の式は読めなかった。手元の cargo の置き場に見つからなかった）。
  - 窓の位置・倍率を毎フレーム書く係が、`areka-placement`・`areka-emo-present` の今の作りに乗るか。
  - SSP の `--time` の速さの形と、SSP が `--easing` をどう扱うか（設計の R17 のまま、確かめていない）。
  - 段 3・段 4（pattern定義のコマ間・element の名指し）は、検証していない。
