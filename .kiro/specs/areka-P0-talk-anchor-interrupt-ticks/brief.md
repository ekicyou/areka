# Brief: areka-P0-talk-anchor-interrupt-ticks

> 2026-10-15 `/kiro-discovery` で起票（開発者「低優先度の起票。dola の時刻精度を、f64 の秒から、QueryUnbiasedInterruptTimePrecise を起点とした 100ns の u64 に変更し、プロセス間で完全同期アニメーションを行うためのベースを作成する。関連する時刻情報について、すべて、QueryUnbiasedInterruptTimePrecise から取得することを前提とし、必要な spec を起票せよ」）。「時刻の土台」の 3 本の 2 本目（roadmap「テーマ別の決めごと」の「時刻の土台」）。

## Problem

台詞の再生の時刻が、本物の絶対時刻になっていない。そのうえ時計が 2 つある。

- ghost の刻み・kanade・seriko は `GetTickCount64` のミリ秒（分解能 10〜16 ms）で、文字の層は QPC の秒。
- 台本の再生機（sakura）が台本に押す起点（`CueSheet` の `absolute_start_time`）は「その台詞の最初の刻み＝ほぼ 0 秒」で、絶対時刻ではない。文字の層と seriko は、その起点を読まず、自分に cue が届いた時刻から起点をそれぞれ見積もり直している（`seriko-talk-clock-fidelity` が実機で −16.9〜+1.2 ms の差と、cue の届く時刻の揺れ −9〜+50 ms を測った）。
- dola の設計の約束（記憶「dola＝絶対時刻台本の同期ブロードキャスト」＝起点は配る時に押す・届いた先で時刻を導き直さない）が、実装で守られていない。これでは別のプロセスへ台本を渡しても、同じ時刻に鳴らせない。

## Current State

（2026-10-15・main `574bf0a6` の実測）

- **ghost の刻み**: `crates/areka-ghost/src/ticker.rs`。台本の再生機へ 50 ms・kanade へ 1000 ms・SERIKO のループへ 16 ms。時計は差し込み式（`TickerConfig.clock: Box<dyn Fn() -> MonotonicMs>`・既定は `GetTickCount64`）。
- **台詞の刻みの中継**: `crates/areka-ghost/src/dispatcher.rs` が台詞の最初の刻みで `base_now` を決め、`(now - base) / 1000.0` を `SakuraMsg::Tick(f64)` として送る（50 ms の格子）。sakura はその最初の刻み（ほぼ 0.0）を `sheet.with_absolute_start_time(t)` で起点に押す（`crates/areka-sakura/src/drive.rs`）。`areka-ghost` の `sink.rs` は `0.0` を押す。
- **kanade**: `MonotonicMs(u64)`（`crates/areka-kanade/src/msg.rs`）。`OnSecondChange`（OS の連続起動時間の参照）と選択肢の時間切れの締め切りに使う（`schedule/steady.rs`）。kanade は今 dola に依存していない。
- **seriko**: `SerikoClock = Arc<dyn Fn() -> u64>`（`crates/areka-seriko/src/actor.rs`）を `emo2_boot/mod.rs` が `GetTickCount64` へつなぐ。`TalkEpoch`（`talk.rs`）がミリ秒の `u64` と cue の `f64` の秒を混ぜて起点を見積もる。
- **文字の層**: `TalkClock`（`emo2_boot/talk_clock.rs`）が「今−cue の時刻」の最大で起点を見積もる。
- 時刻はまだどれもプロセスの外へ出ていない。host32 の SHIORI の経路に乗る時刻は、`OnSecondChange` の参照の文字列だけ。

## Desired Outcome

- ghost の刻み・kanade・seriko・台詞の刻みの中継が、`dola-interrupt-ticks` の型と時計（`QueryUnbiasedInterruptTimePrecise` の 100ns）で時刻を扱う。`GetTickCount64` を読む所が本番に無い。
- 台本の起点は、配る時に押す本物の絶対時刻（割り込み時刻の値）。文字の層と seriko はその起点を読み、cue の届いた時刻から起点を見積もり直さない（届く時刻の揺れが、鳴る時刻に出ない）。
- その結果、別のプロセスが同じ台本を受け取れば、同じ時計で「今」を読むだけで同じ時刻に鳴らせる＝プロセス間の同期の土台ができている（実際に別のプロセスで鳴らすことは範囲外）。
- 中断・選択肢・待ちで止まって再開するときは、今の約束どおり台本を流す側が起点を押し直す。

## Approach

推す形（要件の段で覆してよい）:

- 刻みの時計（`TickerConfig.clock`・`SerikoClock`・`MonotonicMs`）を新しい型へ替える。差し込み式の形はそのまま使い、テストの偽の時計も同じ型で動かす。
- 台詞の最初の刻みで起点を押す所（`dispatcher.rs` の `base_now`・`drive.rs` の `with_absolute_start_time`・`sink.rs`）を、本物の今（割り込み時刻）で押すように替える。台本の再生機へ送る刻みも、台詞からの相対の秒でなく割り込み時刻そのものにするかを設計で決める。
- 文字の層の `TalkClock` と seriko の `TalkEpoch` は、届いた `CueSheet`／`TalkCue` が運ぶ起点を使う。見積もりのコードは消す。
- 刻みの周期（50・1000・16 ms）は変えない。

**要件の段で決める議題**:

1. **OS の連続起動時間**: `OnSecondChange` の参照（ukadoc では OS の連続起動時間）は、今 `GetTickCount64`（眠りの間も数える）から出ている。`QueryUnbiasedInterruptTimePrecise` は眠りの間を数えないので、そのまま替えると値が短くなる。⑴ この参照だけ眠りを数える同じ系統の時計（`QueryInterruptTimePrecise`）にする ⑵ 眠りを数えない値を正とする ⑶ 開発者の指定「すべて QueryUnbiasedInterruptTimePrecise から」を字面どおり守る、のどれか。
2. **眠りから戻った直後の締め切り**: 選択肢の時間切れなどの締め切りも眠りの間は進まなくなる（10 秒の時間切れが、復帰した瞬間に切れない）。これを正とするか。
3. `seriko-talk-clock-fidelity` が先に着地して起点を 1 か所で見積もる形を入れていたら、その見積もりはこの spec で「押された起点を読む」に置き換わる。

## Scope

- **In**: ghost の刻み・台詞の刻みの中継・kanade・seriko の時計を新しい型へ。台本の起点を配る時に本物の絶対時刻で押すこと。文字の層と seriko の起点の見積もりを、押された起点を読む形へ。`OnSecondChange` の連続起動時間の値の決め（議題 1）。`doc/COMPAT_ARCHITECTURE.md` の dola の節と、記憶にある起点の約束の実装の位置。
- **Out**: dola の型と時計そのもの（`dola-interrupt-ticks`）。`std::time::Instant` の締め切りと入力の時刻（`instant-to-interrupt-ticks`）。別のプロセスへ台本を実際に送って鳴らすこと。暦の時刻（`OnMinuteChange` などの日時・ログの日時）。刻みの周期の変更。

## Boundary Candidates

- 刻みの時計（ghost の `ticker.rs`・kanade の `msg.rs`・seriko の `actor.rs`・`emo2_boot/mod.rs` の結線）。
- 起点を押す所（`dispatcher.rs`・`drive.rs`・`sink.rs`）と、起点を読む所（`talk_clock.rs`・seriko の `talk.rs`）。

## Out of Boundary

- 台詞の中で何がいつ鳴るか（文字の現れる時刻の式・`talk` を鳴らす決まり）。起点の取り方を替えるだけで、同じ入力なら同じ結果。
- kanade の時間切れの権限の持ち主（kanade に一本化した今の決め＝`doc/choice-cascade-compat.md`）は変えない。

## Upstream / Downstream

- **Upstream**: `dola-interrupt-ticks`（型と時計）。
- **Downstream**: 別のプロセスで同期して鳴らす仕事（まだ起票しない＝使い手が現れたとき）。`talk-fast-forward`（1 本の台詞の時計を進める前提＝この spec の後なら起点が 1 つ）。

## Existing Spec Touchpoints

- **Extends**: なし（新しい境界）。
- **Adjacent**:
  - `seriko-talk-clock-fidelity`（区分 B・先に着地する見込み）: 同じ `talk_clock.rs`・seriko の `talk.rs`・`emo2_boot/mod.rs` の時計に触る＝直列。fidelity の brief へ「時計を揃える形を選ぶなら、揃える先は QPC でなく割り込み時刻（この spec の行き先）にする」を申し送った。
  - seriko の列（`seriko-trigger-lottery-gaps`・`seriko-interval-combinations`・`seriko-script-triggers`）と kanade の列（`mcp-kanade-tools`・`script-security-level`）: 同じファイルに触る＝同じ時に走らせない。
  - `instant-to-interrupt-ticks`: 触るファイルは重ならない見込み（あちらは wintf と areka の締め切り）。

## Constraints

- 段は**据え置き**・区分 D（開発者の指定「低優先度」）。ウェーブへの組み入れは次の棚卸で決める。
- 時刻は正確に扱う。1 フレーム遅らせる解は取らない。3 つ目の時計を作らない（この spec の後、本番の時計は割り込み時刻の 1 本＋暦の時刻だけ）。アニメのエンジンは sakura と seriko の 2 つのまま。
- 注入した偽の時間が観測を追い越さない（テストの檻の決まり）。
- 規模の見込み: M〜L（10〜16 タスク）。
- 要件定義のモデル: Fable（スレッドをまたぐ時刻の扱い・起点の持ち主の判断）。
