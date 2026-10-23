# Brief: dola-interrupt-ticks

> 2026-10-15 `/kiro-discovery` で起票（開発者「低優先度の起票。dola の時刻精度を、f64 の秒から、QueryUnbiasedInterruptTimePrecise を起点とした 100ns の u64 に変更し、プロセス間で完全同期アニメーションを行うためのベースを作成する。関連する時刻情報について、すべて、QueryUnbiasedInterruptTimePrecise から取得することを前提とし、必要な spec を起票せよ」）。「時刻の土台」の 3 本の 1 本目（roadmap「テーマ別の決めごと」の「時刻の土台」）。

## Problem

dola は「同じ台本を、別々の表示役（別のプロセスでもよい）へ配り、同じ絶対時刻に鳴らす」ための層なのに、時刻が素の `f64` の秒で、時計も「このプロセスで QPC を QPF で割った秒」でしかない。

- `f64` の秒は、足し引きのたびに丸めが積もる。0.1 秒のような値は 2 進で表せず、同じ時刻を別の経路で計算すると一致しないことがある（「時刻は正確に扱う」の大原則に対して、表し方の側に誤差の種がある）。
- 時刻の物差しがプロセスの外で通じる形になっていない。別のプロセスへ台本を渡しても、相手が同じ物差しで「今」を読める保証が型にも時計にも無い。

## Current State

（2026-10-15・main `574bf0a6` の実測）

- **時刻の型が無い**: dola の時刻は全部 `f64` の秒（`src` に `f64` が約 240 か所・テストに約 75 か所）。新しい型（`Seconds` など）は無い。
- **時計は関数 1 つ**: `dola::runtime::clock::now() -> f64`（`src/runtime/clock.rs`・QPC÷QPF＝起動からの秒・`cfg(windows)` のときだけ）。dola の中からは呼ばれず、外から `current_time: f64` を渡す受け身の作り。dola はすでに `windows` に依存している（機能は `Win32_System_Performance` だけ）。
- **時刻を運ぶ公開の口**（主なもの）:
  - 台本の再生（cue の層）: `Cue { start_time, duration }`（serde で直列化する形）・`TalkCue { at, duration }`・`BarrierKind` の `timeout`／`duration`・`CueSheet` の `absolute_start_time` と `absolute_fire_time`／`absolute_end_time`・`TimedSchedule` の `Entry` の時刻と `tick(current_time)`／`occupancy_horizon()`・`CuePlayer::tick`。
  - 連続のアニメ（storyboard の層・本番に未結線）: `TransitionDef` の `delay`／`duration`・`Storyboard` の `time_scale` ほか・コンパイル後の `start_time`／`end_time`／`fire_time`・`DolaRuntime` の `start`／`tick`／`pause`／`resume`。終わらないループの終わりは `f64::INFINITY`・`MIN_LOOP_DURATION = 0.1`。
- **dola の外の使い手**（ざっくりの grep）: `dola::` の参照は areka 82・seriko 18・sakura 14・emo-text 13・wintf 13・ghost 9。`TalkCue {`／`Cue {` を組む所（ほぼテスト）は emo-text 91・areka 77・seriko 36・ghost 8・sakura 7。`dola::runtime::clock::now` を呼ぶ所は 13。
- **「今」を読む本番の 2 か所**: wintf の `FrameTime(pub f64)`（`crates/wintf/src/ecs/graphics/core.rs`・刻みごとに `dola::runtime::clock::now()` で置く）と、areka の `TalkClock`（`emo2_boot/talk_clock.rs`・既定の時計が `dola::runtime::clock::now`）。
- `QueryUnbiasedInterruptTimePrecise` は `windows` 0.62.2 の `Win32::System::WindowsProgramming`（機能 `Win32_System_WindowsProgramming`・`api-ms-win-core-realtime-l1-1-1.dll`・Windows 10 から）に在る＝外の crate を足さずに呼べる（起票時にワークスペースの `windows` のソースで確かめた）。

## Desired Outcome

- dola の時刻は「`QueryUnbiasedInterruptTimePrecise` が返す 100ns 単位の `u64`」を物差しにした型で表され、`f64` の秒は公開の口から消えている。
- その型の「今」を読む関数は 1 つで、`QueryUnbiasedInterruptTimePrecise` を呼ぶ。同じ機械の上なら、どのプロセスで読んでも同じ物差しの値になる（割り込み時刻はシステム全体で 1 本）。
- wintf の `FrameTime` と areka の `TalkClock` が、その関数から「今」を読んでいる。
- 台本（`CueSheet`）を直列化した形も同じ物差しで、別のプロセスへ渡したときに読み替えが要らない。
- テストは今までどおり時刻を外から渡して決定論で回る。

## Approach

推す形（要件の段で覆してよい）:

- **型と時計を dola に置く**（`dola::time` のような 1 か所）。dola は時刻を扱う一番下のクレートで、すでに `windows` に依存している。新しいクレートは作らない（ほかの 2 本の spec で kanade と ghost の時計が同じ型を使うとき、kanade が dola に依存を足す。kanade に dola を引かせたくないと設計で分かったときだけ、下に小さいクレートを切る）。
- **時刻（ある瞬間）と長さ（2 つの瞬間の差）を型で分ける**。どちらも 100ns の整数。差が負になりうる計算（今ある「起点＝今−cue の時刻」の見積もりなど）のために、符号つきの差を持つかを設計で決める。
- **作者が書く単位はそのまま**: 台本の `\w` のミリ秒・pattern定義のミリ秒・storyboard の文書の秒は、読み込む縁で 100ns へ 1 回だけ変える。変えるときの丸めの決まりを 1 か所に書く（100ns は人の目と表示の周期より 5 桁細かい＝丸めで見える差は出ない）。
- **倍率（`time_scale`）とイージングの進み具合は実数のまま**。時刻に掛けて整数へ戻す所を 1 か所にまとめる。
- **終わらないループ**は `f64::INFINITY` の代わりに「終わり無し」を型で表す（`Option` か最大値かは設計）。
- `QueryUnbiasedInterruptTimePrecise` は「眠り（スリープ・休止）の間を数えない」時計。復帰した瞬間にアニメが眠っていた分だけ飛ぶことはない。これを決まりとして書く。

## Scope

- **In**: 時刻と長さの型・「今」を読む関数（`QueryUnbiasedInterruptTimePrecise`）・`dola::runtime::clock::now` の置き換え。dola の公開の口（cue の層と storyboard の層の両方）を `f64` の秒から新しい型へ。直列化の形の変更と往復のテスト。dola の外の使い手の追従（sakura・seriko・emo-text・wintf・ghost・areka。型を替えるだけで、振る舞いは変えない）。wintf の `FrameTime` と areka の `TalkClock` の時計の差し替え。`windows` の機能 `Win32_System_WindowsProgramming` を dola に足すこと。dola の README と crates.io の利用者への説明（壊れる変更＝次の版上げの見どころの「必ず載せるもの」）。`tech.md` の dola の依存の行（今 `windows` の依存が書かれていない）。
- **Out**: ghost の刻み・kanade・seriko の時計（`GetTickCount64` のミリ秒）と、台詞の起点を本物の絶対時刻にすること（`talk-anchor-interrupt-ticks`）。`std::time::Instant` で測る締め切りと入力の時刻（`instant-to-interrupt-ticks`）。暦の時刻（ログの日時・`GetLocalTime`・日付のイベント）。別のプロセスで実際に同期して再生すること（使い手が現れたとき）。

## Boundary Candidates

- 時刻の型と時計の置き場（dola の中か、その下の小さいクレートか）。
- cue の層（本番で動いている）と storyboard の層（本番に未結線）を同じ spec で替えるか。storyboard の層は使い手が居ないので、型を替えるだけに留める。
- 直列化の形（`Cue` の serde の形）の変更と、dola の版上げ。

## Out of Boundary

- 台本の再生の振る舞い（いつ何が鳴るか）は変えない。型を替えた前後で、同じ入力から同じ結果が出ることをテストで示す。
- 表示の周期（`DwmFlush` の待ち）と刻みの出どころ。

## Upstream / Downstream

- **Upstream**: なし（dola の中で閉じる。外の使い手は型を替えるだけ）。
- **Downstream**: `talk-anchor-interrupt-ticks`（ghost・kanade・seriko の時計と台詞の起点）・`instant-to-interrupt-ticks`（残りの「今」の読み取りと番人）。どちらもこの spec の型と時計を使う。

## Existing Spec Touchpoints

- **Extends**: なし（新しい境界）。
- **Adjacent**:
  - `dola-yaml-backend`（区分 C）: 同じ `crates/dola/Cargo.toml` と直列化の往復のテストに触る＝同じ時に走らせない。区分が上なので先に着地する見込み。
  - `element-animation-easing`（区分 C）: dola の `easing.rs` と storyboard の層に触る。先に着地していれば、その spec が足した口も型を替える。
  - `seriko-talk-clock-fidelity`（区分 B）: `TalkClock` に触りうる＝同じ時に走らせない。
  - 文字とバルーンの列（emo-text）・`emo2_boot` の結線の列: 使い手の追従で多くのテストのファイルに触る。型の付け替えだけなので、ほかの spec が無い時に一気に通す。

## Constraints

- 段は**据え置き**・区分 D（開発者の指定「低優先度」）。ウェーブへの組み入れは次の棚卸で決める。
- 時刻は正確に扱う（待ち時間を丸めない・遅れたら過ぎた分だけ進める）。3 つ目の時計を作らない＝この spec の後、dola の時計は `QueryUnbiasedInterruptTimePrecise` の 1 本だけ。
- 外の crate は足さない（`windows` の機能フラグだけ）。dola は crates.io に公開しているクレートで、`cfg(windows)` の外でも型はコンパイルできること（時計の関数だけが Windows 限定）。
- 1 ファイル 1,000 行の番人に触れない（足すときは新しいファイルへ）。
- 規模の見込み: L（14〜20 タスク。dola の中の付け替え 6〜8・使い手の追従 5〜8・時計の差し替えと文書 3〜4）。20 を超えるなら、要件の段で storyboard の層を後ろへ切る。
- 要件定義のモデル: Fable（時刻の型の設計・丸めの決まり・公開の形の変更）。
