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

## 2026-11-13 棚卸㊱で拾った申し送り（audit-dola から）
- `audit-dola` は 11-13 に着地した（PR#440）＝上の 89 行目の「`audit-dola` は今も重なる」は済み。
- **潜んでいる穴**: `crates/dola/src/runtime/instance_manager/mod.rs` の `transition(gid, Playing)` は、止めた（Paused）状態から戻すときに `pause_start` を残したまま。本番の呼び手は `facade.rs` 180 行目の開始だけ（再開ではない）なので今は表に出ない。時刻を割り込みの刻みへ移すこの spec で、止めた時間の数え方と一緒に直す。区分 B（潜在）。

## 申し送り（2026-11-20 ノベルの作り直しから・開発者の承認済み）

ノベル（演劇）の暫定の設計を作り直した所（`doc/ideas/novel-redesign-2026-11-20/00-architecture.md` の 4.3）、高速再生基盤（字を一度に出す・待ちを飛ばす・クリック待ちを通る）と並走が、時刻の土台に次の 5 つを求める事が分かった。この brief は、どれも約束していない（コードの裏取りは同じフォルダの `92-validation-code.md` の 3・4・5）。**開発者の承認（2026-11-20）: v0.2.0 の要件の段で、この 5 つを入れる前提で確かめる**（dola を、後でもう一度作り直さないため。規模は増える。どれだけ増えるかは、まだ見立てていない）。

1. 起点が、もう配った合図にも効く事（文字の層の時計・口パク・バルーンの寿命の時計が、置き直した起点に付いて来る。今の見積もりは最大を取る形で、過去へ動かせない＝`crates/areka-emo-text/src/talk_clock.rs:38-57`）。
2. 起点が、台本の始まりより前へ動ける事（早送りは、起点を前へ動かして表す）。
3. 合図が、絶対の発火時刻を持つ事。時刻の表が、項目を取り出した時の起点で決める（同じ刻みの中で起点が動いても、後ろの変化の長さが狂わない）。
4. トークのスレッドへ送る刻みが、絶対の時刻である事（今は相対の秒＝`crates/areka-ghost/src/dispatcher.rs:392-399`。並走の流れと主の流れを、同じ時刻で進めるため）。
5. 連続値の側（storyboard の層）を、後ろへ切らない事（値を時間で変える仕組みは dola が持つ、と開発者が決めた＝「イージングはdolaの仕事」）。

## 申し送り（dola-cue-test-builder より）

- `crates/dola/src/cue/test_support.rs` の `carrier_cue` が、テストで使う正準のキャリアの合図の唯一の組み立てになった（`test-support` 機能の下・使い手は areka-ghost・areka-seriko・areka-runtime の dev の依存）。`TalkCue` のフィールドや `CueCommand::command_carrier` の署名を変えるときは、この本文 1 か所を直せば使い手の 22 ファイルへは波及しない（名の層が吸う）。参照: `.kiro/specs/completed/dola-cue-test-builder/design.md` の Revalidation Triggers。

## 2026-11-22 棚卸㊸の再測定（main `312a2f18`）
- **前提の変化**: dola の時刻は今も `f64` の秒で、時計は `dola::runtime::clock::now`（`crates/dola/src/runtime/clock.rs`・74 行）の 1 つ。`dola-cue-test-builder`（PR#498）が正準のキャリアの合図の組み立てを `crates/dola/src/cue/test_support.rs` の 1 か所にした＝その合図の分は、型を替えても使い手へ波及しない。本体の crate 分けで、使い手の crate は 6 から 9 になった（sakura・seriko・emo-text・wintf・ghost・`areka-runtime`・`areka-session`・`areka-input`・本体）。
- **「今」を読む本番の所**: wintf の `ecs/world/mod.rs:114`・`:563`（`FrameTime` を置く・670 行）と `areka-runtime/src/lib.rs:465`（`TalkClock` へつなぐ）。型の持ち主は wintf の `graphics/core.rs` と `areka-runtime/src/talk_clock.rs`。`tech.md` の dola の行（55 行）は `windows` を書いてある＝機能の名前を足すだけ。
- **触るファイル**: `crates/dola/` の全体（112 ファイル・`Cargo.toml` を含む）、wintf の `world/mod.rs`・`graphics/core.rs`・`widget/text/typewriter_draw.rs`・`ecs/dola/mod.rs`、使い手（`dola::` か合図を組むファイル＝`areka-runtime` 38・emo-text 32・本体 22・seriko 16・sakura 12・ghost 5・`areka-session` 4・`areka-input` 2）。
- **規模**: 18〜26 の見込み（起票の L 14〜20 に、ノベルの作り直しの 5 つと、増えた使い手の crate が乗る）。
- **先に要るもの**: `dola-cue-test-builder` は着地。C26 の見直し 3 本とは使い手のテストで重なる（`audit-areka-app`＝`areka-session/src/runtime_boot_tests.rs`・本体の `runtime_tests/spine.rs`・`session_tests/` の 4 本、`audit-areka-install-update`＝`install_cue*`・`update_cue*`・`readme_cue*`・`mcp/dump_*_gpu_*`、`audit-emo2-boot-switch`＝`move_cue*`・`change_cue*`・`balloon_visibility*`・`shell_balloon_switch_session_lap_test_support.rs`）＝C26 の全部の着地の後に 1 本だけで回す（見込み C27 のまま・確かめた）。同じ時に走らせない相手は `element-animation-easing`（`easing.rs`）・`seriko-talk-clock-fidelity`・`talk-fast-forward`・`sakura-embed-directive`（`talk_clock.rs`）・`tick-gate-adoption`（`world/mod.rs`）。
- **優先度の区分**: D（開発者「低優先度」・据え置き。版の段は v0.2.0＝ノベルの作り直しの承認で時刻の土台を v0.2.0 の要件の段に入れた）。
- **要件定義のモデル**: Fable（前例の無い公開の口＝ワークスペースに時刻と長さの型は無く〔wintf の `api.rs` の CPU 時間の素の `u64` の欄と、`areka-session` のテストの `Ticks` だけ〕、dola の時刻の型に 9 つの crate と `Cue` の直列化の形が乗る。ノベルの 5 つ〔起点を過去へも前へも動かす・絶対の発火時刻〕は時刻の順の保証を新しく決める）。
- **分割の案**: 20 を越えたら、型と時計と直列化の付け替え（振る舞いを変えない・使い手の追従まで）と、ノベルの 5 つのうち起点と発火時刻の働き（1〜4）の 2 本に分ける。起票の「storyboard の層を後ろへ切る」は、5 つ目の「後ろへ切らない」と食い違うので使えない。
- **見つけた穴・古くなった記述**: 2026-11-13 の潜んでいる穴（`instance_manager/mod.rs` の `transition(gid, Playing)` が `pause_start` を残す）は今もある（本番の呼び手は `facade.rs:167` の開始だけ）。ノベルの申し送りの 1 の `crates/areka-emo-text/src/talk_clock.rs:38-57` は無いファイル＝今は `crates/areka-runtime/src/talk_clock.rs:30〜40`、4 の `dispatcher.rs:392-399` は今は `:365〜373`。Adjacent の `dola-yaml-backend` は着地済み。Current State の「`dola::` の参照」の数と「今を読む本番の 2 か所」の場所（`emo2_boot`）は起票のときのもの。
