# Brief: talk-fast-forward

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

## 2026-10-10 `areka-P0-seriko-trigger-intervals` の完了時の申し送り

出どころは `completed/areka-P0-seriko-trigger-intervals/tasks.md` の Implementation Notes（9.2 の ②・検証の ⑷）と同 spec の設計書（Revalidation Triggers）。

- **実機で見たこと（2026-10-10）**: 台本の `\x` は、待ちもせず消しもしない（`まつ。\xあと。` は、間を置かずに続けて出た）。上の Problem・Current State に書いてあるとおりの今の姿で、新しい事実は無い。同 spec の Implementation Notes はこの件の引受先を `balloon-canon-residue` と書いたが、`\x`／`\x[noclear]` は本 spec が引き取っている（上の Existing Spec Touchpoints）。
- **口パクが、文字の現れる時刻の式を写して持つようになった**: `seriko-trigger-intervals` で、seriko は interval `talk,数値` のために「i 文字目が現れる時刻」を自分でも計算する（`crates/areka-seriko/src/talk.rs` の `TalkFeed`＝文字の層の `RevealSchedule::extend_chunk` と同じ式・`TalkEpoch`＝`TalkClock::observe_cue` と同じ式）。**共有ではなく写し**で、文字の層の式を変えても赤になるテストは無い。
- **本 spec がすること**: 早送り（残りの字を一気に出す）や `\x`（時計を止めて、クリックで再開する）で、文字が現れる時刻・起点・cue の届き方を変えるなら、`talk.rs` の写しも同じに変える。変えないと、口だけが元の時刻で動く（早送りで字が全部出た後に、口が遅れて動き続ける形になりうる）。要件の段で、早送りの間の口の動かし方（区切りを 1 回の刻みで何個も越えたときは、最新の 1 つだけ鳴る決まりが今ある）を 1 行確かめる。
- **見張りの持ち主**: 写しと文字の層の式の一致を見張る仕組みは、新しい spec `seriko-talk-clock-fidelity` が持つ。そちらが先に着地していれば、本 spec の変更で見張りが赤になる。本 spec が先なら、`talk.rs` を手で合わせ、同 spec の brief に合わせた所を 1 行残す。

## 2026-10-23 棚卸㉙の追記
- **印の画像の系列（`clickwait*`）を分け合う相手は `balloon-markers` から `balloon-scroll-arrows`（v0.1.0）に替わった**。同 spec が `balloon-markers` の前半を切り出したとき、装飾の系列の土台（emo-present の `balloon.rs` の `SeriesFamily` の装飾の行・旧名の段・「`clickwait*` も同じ仕組みで解けるようにする」）もあちらへ移った（同 brief の Approach と Downstream）。`balloon-markers` に残った後半（オンラインの印・数字・SSTP の印）は `clickwait*` に触らない。
- **`clickwaitmarker.*` の鍵と、印を出す振る舞いは今も本 spec の物**（`balloon-scroll-arrows` の Scope の Out に「`clickwaitmarker` の振る舞い（`talk-fast-forward`）」）。
- 本文の 46・75・84・93・98・123・141 行目の `balloon-markers` は `balloon-scroll-arrows` と読み替える。列の順は「`balloon-scroll-arrows` が先に系列の土台を作り、本 spec はそれを使う」見込みで、「先に着地した方が作る」の取り合いは無くなる。印を文字の面の中で描くか emo-present で重ねるか（議題 3）も、あちらの答えに合わせる。
- 並走: `balloon-scroll-arrows` とは emo-text の `actor*.rs`・`viewbox*.rs`・`input_events/shell_box.rs`・`user_break.rs`・emo-present の `balloon.rs` が重なる＝同じウェーブに置かない。

## 2026-10-31 棚卸㉛の追記（完了 `balloon-scroll-arrows` の残り物）

- **件 A（区分 B・小さい）**: 矢印の押下が中断の判定の「前の押下で選択したか」を上書きしない。`crates/areka/src/input_events/user_break.rs` の `prev_press_selected` は、バルーンの押下の道（269〜270 行目・371〜372 行目）でだけ書き替わり、矢印が押下を使い切ったときは書き替わらない。このため「選択肢をクリック → 矢印をクリック → 矢印の外をダブルクリック」で、ダブルクリックの 1 打目が前の選択の続きと読まれ、中断を取らない（完了 `balloon-scroll-arrows` の `tasks.md` の 6.2 の記録）。早送りも同じ押下の道を触るので、この spec の要件の段で一緒に直すか、決まりとして記録して閉じるかを決める。規模は 1 タスク（直しと端の場合のテスト）。

## 2026-10-31 棚卸㉜の再測定（main d374f901）
- **前提の変化**（棚卸㉔の後）:
  - `mouse-click-wheel-events`（v0.1.0・着地）で、areka は単クリックの `OnMouseClick` をシェルへ送るようになった（kanade の `schedule/mouse_events.rs`）＝本文の「箱のクリックで `OnMouseClick` などは送らない」が意味を持った。箱の押下の前段 `crates/areka/src/input_events/shell_box_handler.rs` の `on_box_pointer_pressed`（`true` ならシェルへ送らない）に、`balloon-scroll-arrows` が矢印の押下を足した前例が在る（236 行目）＝早送りも同じ所に足す。
  - `balloon-scroll-arrows`（着地）が印の系列の土台を作った＝`clickwait` の系列は `crates/areka-emo-present/src/balloon_marker.rs`（26〜27 行目）に在り、本 spec は使うだけ。鍵 `clickwaitmarker.*` と印を出す振る舞いは今も無い（`areka-parsers/src/balloon/` に 0 件）。
  - `script-small-tags`（10-31）が `\x` を「知らないタグ」の見本に使う検査を増やした＝`\x` に腕を足すと替える見本は 3 ファイル約 20 か所（`crates/areka/src/mcp/check_script_judge_tests.rs` 11・`check_script_tests.rs` 1・`crates/areka-parsers/src/sakura/parse_noted_tests.rs` 8）。`\x` の腕が `sakura/decode.rs` に無いこと、compile が置く区切りは末尾の `WaitForChoice`（`crates/areka-sakura/src/compile.rs` 449〜462 行目）だけで、dola の `WaitForInput` と `CuePlayer::resolve_click`（`crates/dola/src/cue/runtime.rs` 296 行目）は在ることは、棚卸㉑〜㉔の見立てのまま。
- **触るファイル**: 前半（早送り）＝`crates/areka/src/input_events/{balloon_pressed,user_break,shell_box,shell_box_handler}.rs`・kanade の `msg.rs`（`mcp-kanade-tools` が割って 575 行。`schedule/steady.rs` は 968 行で上限の近く）と新しいファイル・`crates/areka-ghost/src/dispatcher.rs`・`crates/areka-sakura/src/drive.rs`・`crates/areka/src/emo2_boot/talk_clock.rs`・`crates/areka-seriko/src/talk.rs`（口パクの写し・上の申し送り）。後半（`\x`）＝`sakura/{decode,model}.rs`・`areka-sakura/src/{compile,drive}.rs`・dola の `cue/{schedule,runtime}.rs`・emo-text の `state.rs`・`state_decoration.rs`・`actor.rs`＋新しいファイル（`lib.rs` の一覧の席）・`balloon/{model,parse}.rs`（`clickwaitmarker.*`）・`balloon_marker.rs` の読み手・検査の見本 3 本・台帳 `ledger/{sakura-script,assets}.toml`。
- **規模**: 19〜24 タスク（棚卸㉔の 18〜23 に件 A の 1）。
- **分割の案**（棚卸㉓の境界・まだ一度も切り出していない）: 要件の段で 20 を超えたら、前半 `talk-fast-forward`（早送り＋件 A・9〜11）と後半 `click-wait-tag`（仮・`\x`／`\x[noclear]` と `clickwaitmarker.*`・11〜13）に切る。前半は `\x` が無くても「台詞の終わりまで」で成り立つ。早送りが区切りで止まる所は後半が足す。
- **先に要るもの**: 働きの前提は満たしている。列の順では、台本のコンパイルの列で `range-choice-tag` の後、文字とバルーンの列で `balloon-font-file`・`text-typesetting` の後＝今は先頭でない。
- **重なり**: `shell-scale-linkage` とは 0（`emo2_boot/` で触るのは `talk_clock.rs` だけ）。同じウェーブに置けない相手＝`balloon-link-hover`・`mcp-shiori-query`（kanade）・`balloon-markers`（`input_events/`）・`balloon-font-file`（emo-text の `actor.rs`・`lib.rs` の席）・`config-parse-diagnostics`（`balloon/parse.rs`・dola の `cue/runtime.rs`）。
- **議題**（棚卸㉑・㉒から持ち越し）: ⑴ 途中の `\x` の後ろの時刻を dola の `TimedSchedule` で付け替えるか（全文の先渡し `CueSink::preview` に手が入らないのでこちらが有力）、compile が台本を前後に分けるか ⑵ ダブルクリックの 1 打目が早送りとして食われる順序 ⑶ 印を文字の面の中で描くか emo-present で重ねるか（`balloon-scroll-arrows` の答えに合わせる・MCP の `dump_balloon` は文字の面だけ写す）⑷ 箱の上で押して動かしたとき、早送りとドラッグの開始を両方起こすか。時刻を付け替えるときも、到達の判定は「開始＋相対」の足し算の形を保つ（`crates/dola/tests/cue/schedule_test.rs` の境目の検査を緑に）。
- **優先度の区分**: A（roadmap「シェル内バルーン」節の開発者の確定 6「早送り」）。
- **要件定義のモデル**: Fable（時刻の新しい約束を作る＝dola の区切りの後ろの時刻の付け替えと、UI から ghost のスレッドへ渡したクリックとダブルクリックの判定の順。どちらも前例が無い）。
- **見つけた穴・古くなった記述**: 本文 Current State の押下の場所は分割で動いた（単クリックは `input_events/balloon_pressed.rs`、箱は `shell_box.rs` の `judge_box_press`〔4 値〕と `user_break.rs` の `on_box_press` 357 行目）。棚卸㉑〜㉔の「areka は `OnMouseClick` を送っていない」は古い（上のとおり今は送る）。
