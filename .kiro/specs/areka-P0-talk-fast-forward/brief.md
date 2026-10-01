# Brief: areka-P0-talk-fast-forward

> 2026-10-01 `/kiro-discovery`（シェル内バルーン）で起票。roadmap「シェル内バルーン」節。普通のバルーンにも効く。本文の file:line は起票時（main `35209987`）の実測＝着手時に引き直すこと。

## Problem

- **利用者**: 1 字ずつの表示を待たずに読み終えたいとき、クリックで先へ進める手段が無い。今あるのはダブルクリックでの中断だけ（台本を止めてバルーンを消す）。
- **作者**: クリック待ち `\x`／`\x[noclear]` が使えない（転記層で素通し）。あふれた台詞を区切る正典の手段が `\x` と `\c` なので、シェル内バルーンの小さな箱では特に要る。クリック待ちの印 `clickwaitmarker.*` も無い。
- 正典に「話している最中のクリックで早送り」は無い（ukadoc で「早送り」「スキップ」0 件・SSP 本体の利用者設定の側と見られる）。

## Current State

- 1 字ずつの表示は台詞の時計で決まる（`crates/areka-emo-text/src/state.rs` の表示時刻・`crates/areka/src/emo2_boot/talk_clock.rs`）。全部を即時に出す口は無い。
- バルーンの左ダブルクリック＝中断（`crates/areka/src/input_events/user_break.rs`・`nouserbreakmode` で抑止）。1 回のクリックは選択肢の選択だけ（`input_events/balloon.rs`）。
- `\x` は網羅台帳で `alias`、`\x[noclear]` は `vocabulary-only`（`doc/ukadoc-coverage/ledger/sakura-script.toml`）。`clickwaitmarker.*` は `absent`（`assets.toml`）。いずれも引受先は `balloon-canon-residue` → 分割後は `balloon-lifecycle-events`（項目 9）。
- wintf に使われていない typewriter の `skip()` がある（`crates/wintf/src/ecs/widget/text/typewriter/mod.rs`）。

## Desired Outcome（2026-10-01 開発者確定）

- **早送り**: 話している途中にバルーン（普通のバルーンもシェル内バルーンも）を 1 回クリックすると、**次のクリック待ち（`\x`）か台詞の終わりまで一気に進む**。文字の表示も `\w` の待ちも飛ばす。途中のサーフェスの切り替え（`\s`）などは飛ばさず順番どおり一瞬で適用する。仕組みは「台詞の時計を `\x` の地点まで早回しする」（台詞は絶対時刻の台本＝dola で動くので、早回しで壊れない）。
- **シェル内バルーンの箱のクリック**: 話している最中なら早送りだけを行い、シェルの当たり判定へのクリックのイベント（`OnMouseClick` など）は送らない。話していなければ従来どおりシェルのクリック。
- ダブルクリックでの中断は今のまま。
- **`\x`／`\x[noclear]`**: 正典どおり（`\x` はクリック後にスコープを `\0` へ戻し `\f` 系を解除・`\x[noclear]` は内容とスコープと `\f` 系を保つ）。クリック待ちの間は `clickwaitmarker.*` の印（`clickwait*.png`）を出す。
- **原則（開発者裁定）**: クリック待ちは台本に明示した `\x` だけ。areka が自動でクリックを求める仕組み（自動の改ページなど）は作らない——デスクトップマスコットは利用者の状況にお構いなしに喋るもの。

## Approach

- 早送りは表示の側の特別扱いではなく、台詞の時計（talk clock）の早回しで実現する（文字の層・SERIKO・音などが同じ時計を見ているので一貫する）。
- `\x` は台詞の時計を止める点として台本に置き、クリックで再開する。

## Scope

- **In**: 早送り（普通のバルーン・シェル内バルーン）、箱のクリックの振り分け、`\x`／`\x[noclear]`、`clickwaitmarker.*`（印の位置のキーと画像の系列の `clickwait*`）、網羅台帳の更新、決定論テスト。
- **Out**: `\_q`・`\![quicksection]`・`\![set,balloonwait]`（`sakura-time-directives` の担当のまま。早送りは利用者の操作、`\_q` は台本の指示で別物）・`OnBalloonClose` 等のイベント（`balloon-lifecycle-events`）・あふれのフェード（`balloon-scroll-fade`）。

## Boundary Candidates

- 入力（クリックの振り分け）と、台詞の時計（早回し・`\x` での停止と再開）と、印の表示。

## Out of Boundary

- 中断（ダブルクリック）の規則・`nouserbreakmode`。

## Upstream / Downstream

- **Upstream**: `shell-balloon`（箱のクリックの振り分けはシェル内バルーンの当たり判定の上に乗る）。印の画像の系列の解決は `balloon-markers` と同じ仕組み（先に着地した方が作り、後の方が使う）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: `balloon-lifecycle-events`（項目 9「`\x`／`\x[noclear]`」を本 spec が引き取る＝同 brief へ追記済み）。
- **Adjacent**: `sakura-time-directives`（`\_q`・`balloonwait`）・`balloon-lifecycle-events`（中断で終わった会話のタイムアウト起点＝項目 10 は向こうのまま）。

## Constraints

- 決定論テスト網羅は必達（時計の早回しは注入した模擬時刻で確かめる。模擬時刻は観測を追い越さない）。
- ログ無しの失敗の経路を作らない。
