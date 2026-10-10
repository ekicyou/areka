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


---

## 2026-10-02 棚卸⑳の再測定（main `03e8d7d6`・α 完成宣言の後）

- 規模 M（13〜18 タスク）。文字まわりの直列の列（`shell-balloon` の後）。
- **抜け（作業が広がる）**: ⑴ `\x` は今パーサの段で `Instruction::Raw` に落ちる（`sakura/decode.rs` に腕が無い）＝パーサ・`areka-sakura` の `compile.rs` と `drive.rs`・dola の cue（`command.rs`・`runtime.rs`）まで作業が要り、dola に種類を足すと網羅の match（emo-text・`areka-ghost/src/sink.rs`・seriko）が連鎖する。⑵ **dola の `CuePlayer` の文書は「一時停止と再開は持ち込まない」を対象外と明記している**（`runtime.rs` の冒頭）＝`\x` で止めて再開する案は過去の設計判断とぶつかる。使えそうな既存の継ぎ目は選択肢で使っている barrier。⑶ 既定バルーン（Staysee）に `clickwait*.png` は無い＝検体が要る。⑷ `\x` の後の `\f` 系の解除は emo-text の `state_decoration.rs` の仕事。
- 「`OnMouseClick` などは送らない」とあるが、areka は今そもそも `OnMouseClick` を送っていない。
- **議題**: dola の対象外を覆すか barrier で組むか／ダブルクリックの 1 回目が早送りとして食われる順序／印を emo-text と emo-present のどちらで描くか。
- `compile.rs` を触る＝`choice-timeout-directive`（C1）の後。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: M〜L（15〜19 タスク）。今は切らない。20 を超えそうなら「早送り（クリック→台詞の終わりまで一気に）」と「`\x`／`\x[noclear]` と `clickwaitmarker.*`」に切る。早送りが先（印の系列は `balloon-markers` と共有で、後ろに置くほど相手が先に作っている見込みが高い）。
- 前提の状態: `shell-balloon`（PR#227・箱の当たり判定と押下の前段）・`choice-timeout-directive`（PR#215・`compile.rs`）とも着地済み＝満たす。
- 崩れた前提／古くなった位置:
  - **棚卸⑳の議題「dola の対象外を覆すか barrier で組むか」は、barrier で組める見込みが強くなった**: dola に入力待ちの区切り `BarrierKind::WaitForInput`（`dola/src/cue/command.rs`・「旧 WaitForClick を統合」）と、その再開口 `CuePlayer::resolve_click`（`dola/src/cue/runtime.rs`）が既に在り、`tick` も `WaitingForInput` で止まる。`CueCommand` に種類を足す必要は無い（網羅の match の連鎖は起きない）。止める仕組みの対象外（`Paused`/`pause`/`resume`）には触れない。
  - **ただし区切りを台詞の途中に置いた前例が無い**: compile が今置くのは末尾の選択待ち（`compile.rs` の `WaitForChoice`）だけ。`TimedSchedule::notify_barrier_resolved` は区切りを外すだけで、後ろの cue の時刻をずらさない＝`\x` の後ろの文字・`\w` の待ちが、クリックまで待った分だけ「過ぎた」扱いで一度に出る。途中の `\x` には時刻の付け替え（再開した時刻へ後ろを寄せる）が dola に要る。
  - **dola の到達の判定は「開始＋相対」の足し算の形を保つ**（`areka-P0-budoux-reveal-reflow` で `TimedSchedule` の到達・区切りの期限・完了を `current_time >= start_time + offset` へ改めた。`CueSheet::absolute_fire_time` と同じ式。引き算へ戻すと予定時刻ちょうどの `tick` で合図を取りこぼす。時刻を付け替えるときもこの形のまま起点か相対の時刻を動かし、`crates/dola/tests/cue/schedule_test.rs` の境目の検査を緑に保つ）。
  - 再生の本体は ghost のスレッド（`areka-ghost/src/dispatcher.rs`・`ResolveChoice` を受ける `on_resolve_choice` と同じ形で「クリックで再開」の知らせを足す）。UI の `emo2_boot/talk_clock.rs` の `TalkClock` は届いた cue の時刻を `observe_cue` で追うだけ＝早送りは ghost の側の時刻を進め、文字の層の「現れる時刻の列」（`state.rs` の `visible`）がそれに追いつく形になる。
  - 分割でクリックは `input_events/balloon_pressed.rs`（末尾で `user_break` へ渡す）、ダブルクリックの判定は `input_events/user_break.rs`（`on_left_press`・箱は `on_box_press`）。**箱の押下は `input_events/shell_box_handler.rs` → `shell_box.rs` の `judge_box_click`（`BoxPressVerdict` は今「選択で使った／シェルの操作」の 2 値）**＝「話している最中なら早送り」はここに 3 つ目の結論として足す。
  - brief の「`OnMouseClick` などは送らない」は、今 areka が `OnMouseClick` を送っていない（棚卸⑳）うえ、`mouse-drag-events`（C2-⑦・起票済み）がキャラクター窓のマウスのイベントを足しにくる＝どちらが先でも、箱の押下でシェルへ送らない条件を相手に合わせる。
  - 印の系列（`clickwait*`）は `balloon-markers` と共有。印の位置は箱の左上からも読む（箱でも出す）。
- 触るファイル（並走の照合用）:
  - `crates/areka-parsers/src/sakura/{decode.rs, model.rs}`（`x` の腕）
  - `crates/areka-sakura/src/compile.rs`（途中の `WaitForInput`）・`drive.rs`
  - `crates/dola/src/cue/{schedule.rs, runtime.rs}`（区切りの後ろの時刻の付け替え）
  - `crates/areka-ghost/src/dispatcher.rs`（クリックで再開・早送り）と知らせの型
  - `crates/areka/src/emo2_boot/talk_clock.rs`・kanade への届け口（クリックをどの経路で ghost へ渡すか次第で `areka-kanade/src/msg.rs`・`schedule/steady.rs` 系）
  - `crates/areka-emo-text/src/{state.rs, state_decoration.rs（`\x` での `\f` の解除）, actor.rs}`
  - `crates/areka/src/input_events/{balloon_pressed.rs, user_break.rs, shell_box.rs, shell_box_handler.rs}`
  - 印: `crates/areka-emo-present/src/balloon.rs`（`SeriesFamily` の `clickwait` の行・`balloon-markers` と取り合い）・`crates/areka-parsers/src/balloon/{model.rs, parse.rs}`（`clickwaitmarker.*`）・検体
  - `doc/ukadoc-coverage/ledger/{sakura-script,assets}.toml`
- 議題（答えで作業が変わるものだけ）:
  1. 途中の `\x` の後ろの時刻を、dola の `TimedSchedule` で付け替えるか（区切りの再開時刻を新しい起点にする）、compile が `\x` で台本を前後に分けるか。
  2. ダブルクリックの 1 回目が早送りとして食われる順序（棚卸⑳のまま）。
  3. 印を emo-text と emo-present のどちらで描くか（棚卸⑳のまま・`balloon-markers` と同じ答えにする）。
- 見つけた穴: 実害のあるバグは無い。ただし「dola の入力待ちの区切りは解いても後ろの時刻をずらさない」ので、今の部品のまま途中に `WaitForInput` を置くと後ろが一度に出る（未使用の経路なので今は害が無い）。


---

## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: M〜L（15〜19 タスク）。今は切らない（20 を超えるなら㉑の切り方＝早送りが先、`\x` と `clickwaitmarker.*` が後）。
- 前提の状態: 満たす。列の上では `budoux-reveal-reflow`（バグ・10-05 起票）・`balloon-font-file`・`anchor-tag-canon`・`text-typesetting` の後。`budoux-reveal-reflow` の「一度出した字の行を動かさない」約束は、早送りで残りの字が一度に届くときにも効く（向こうの brief の Downstream に本 spec の名がある）＝向こうの着地の後に始める。
- 崩れた前提／古くなった位置:
  - **㉑の「`judge_box_click` の `BoxPressVerdict` は 2 値・3 つ目として足す」は誤り**（C3 の前からこの形）。結論は「シェルの操作／選択で使った／中断を禁じる区間／中断」の 4 値で、順を決めるのは `input_events/shell_box.rs` の `judge_box_press`、呼び手は `input_events/user_break.rs` の `on_box_press`。今、話している最中の箱の単クリックは ⑵「左ダブルクリックでない→シェルの操作」に落ちる。早送りはこの ⑵ の手前に足す。「話している最中か」は同じ `user_break.rs` の `talking()`（`shell_box.rs` の `fold_talking` で畳む）が既に持つ＝作らなくてよい。
  - `mouse-drag-events`（PR#240）が着地: areka がキャラクター窓から送るのは `OnMouseMove`・`OnMouseDoubleClick`・`OnMouseDragStart`・`OnMouseDragEnd` だけで、単クリック（`OnMouseClick`）は今も送らない＝「箱の押下でシェルへ送らない」の相手は確定した。ただしドラッグの受け手（`input_events/drag.rs`）は箱の上かどうかを見ない＝話している最中に箱を押して動かすと「早送り」と「ドラッグの開始」が両方起きうる。要件で扱いを決める。
  - クリックを ghost の再生へ届ける道は、選択と同じなら kanade を通る（`Action::ResolveChoice` の形）。kanade の `msg.rs` は 909 行・`schedule/steady.rs` 947 行・`schedule/mod.rs` 938 行＝足すものは新しいファイルへ。
  - dola の `WaitForInput`（`cue/command.rs`）・`CuePlayer::resolve_click`（`cue/runtime.rs`）・compile が末尾にだけ置く `WaitForChoice`（`compile.rs`）・`decode.rs` に `x` の腕が無いことは㉑のまま（C3 はどれにも触れていない）。
  - 印をどこで描くかの議題に材料が 1 つ増えた: MCP の `dump_balloon`（C3・`mcp-dump-images`）は背景の絵に**文字の面だけ**を重ねて返す（`mcp/dump_balloon_overlay.rs` の `overlay_text`）。印を文字の面の中に描けば写り、emo-present の別の絵で重ねると写らない。
- 触るファイル（並走の照合用）:
  - `crates/areka-parsers/src/sakura/{decode.rs, model.rs}`・`crates/areka-sakura/src/{compile.rs, drive.rs}`・`crates/dola/src/cue/{schedule.rs, runtime.rs}`
  - `crates/areka-ghost/src/dispatcher.rs`・`crates/areka/src/emo2_boot/talk_clock.rs`・kanade の `msg.rs`＋新しいファイル（`schedule/mod.rs` に腕 1 本の見込み）
  - `crates/areka-emo-text/src/{state.rs, state_decoration.rs, actor.rs}`・`lib.rs`（新しいファイル）
  - `crates/areka/src/input_events/{balloon_pressed.rs, user_break.rs, shell_box.rs, shell_box_handler.rs}`
  - 印: `crates/areka-emo-present/src/balloon.rs`（`clickwait` の行）・`crates/areka-parsers/src/balloon/{model.rs, parse.rs}`・検体（印を present の外で描くなら `crates/areka/src/mcp/dump_balloon*.rs` も）
  - `doc/ukadoc-coverage/ledger/{sakura-script,assets}.toml`
- 議題（答えで作業が変わるものだけ）: ㉑の 3 件のまま。⑷（新）箱の上で押して動かしたときに早送りとドラッグの開始の両方を起こすか。
- 見つけた穴: なし。軽微: ㉑の「`judge_box_click`（2 値）に 3 つ目」の書き方を上のとおり読み替える。
- 並走の判定（厳しめ）: `budoux-reveal-reflow`（`state.rs`）・`anchor-tag-canon`・`text-typesetting`（`state.rs`・`balloon/{model,parse}.rs`）・`balloon-markers`（`shell_box.rs`・`user_break.rs`・emo-present の `balloon.rs`）・`balloon-canon-residue`（emo-present の `balloon.rs`）・`balloon-lifecycle-events`（kanade の `schedule/mod.rs` の見込み）とは重なる＝並べない。`balloon-font-file` とは `actor.rs`・`lib.rs` が重なる＝並べない。


---

## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化:
  - `budoux-reveal-reflow`（10-06 着地）: 文字の層は台本の全文を先に受け取って区切る（`crates/areka-emo-text/src/lookahead.rs`）。早送りで残りの字が一度に届いても、出した字の行は動かない＝待っていた前提は満たした。dola の予定表の到達の判定は「開始＋相対」の足し算になった（`crates/dola/src/cue/schedule.rs`）＝区切りの後ろの時刻の付け替えは「開始の時刻を、待った分だけ後ろへずらす」形で書ける見込み。
  - **議題 1 に材料が増えた**: 全文の先渡し（`CueSink::preview`）は、再生機に受け手を登録するときに 1 度だけ、台本の全部を渡す。`\x` で台本を前後に分ける案だと先渡しが 2 回になり、`\x[noclear]`（内容を保つ）で「台詞の頭は必ず全消去」という先読みの前提が崩れる（向こうの design が見直しの引き金に挙げている）。dola の中で時刻を付け替える案なら先読みに手が入らない。
  - `mcp-author-tools`（10-08 着地）: 台本を読む段（`crates/areka-parsers/src/sakura/decode.rs`）は腕ごとに印を返す形になった。`\x` に腕を足すと「知らないタグ」の印が消える。**`\x` を「知らないタグ」の見本に使っている検査が 3 本ある**（`crates/areka/src/mcp/check_script_judge_tests.rs` の 8 か所・`check_script_tests.rs`・`crates/areka-parsers/src/sakura/parse_noted_tests.rs`）＝見本を別の綴りへ替える。
  - `balloon-lifecycle-events`（10-08 着地）: バルーンの時間切れはトークの終わりから数える（`crates/areka/src/emo2_boot/balloon_visibility*.rs`・`talk_lifecycle.rs`）。`\x` で止まっている間はトークが終わっていないので数えない見込み＝要件で一言確かめる。中断の規則（`input_events/user_break.rs`・`shell_box.rs` の `judge_box_press` の 4 値）は変わっていない。
  - 上限に近いファイル: kanade の `msg.rs` 926・`actor.rs` 900・`schedule/steady.rs` 950・`schedule/mod.rs` 955 行＝足すものは新しいファイルへ。
- 触るファイル: 棚卸㉒の一覧に、検査の見本の 3 本（上記）を足す。emo-text には検査の兄弟のファイル（`\x` の後の装飾の戻し）と、印を文字の面で描くなら配置のファイルを足す＝**emo-text にファイルを足す**（`lib.rs` の席を使う）。
- 規模: 18〜23 タスク（棚卸㉒は 15〜19）。
- 分割の案: 20 をまたぐ。要件の段で超えたら、棚卸㉑の境界で次の 2 本に切る（一度も切り出していない spec）。
  - 前半 `talk-fast-forward`（早送り・8〜10）: `crates/areka/src/input_events/{balloon_pressed,user_break,shell_box,shell_box_handler}.rs`・kanade の `msg.rs` と新しいファイル・`crates/areka-ghost/src/dispatcher.rs`・`crates/areka-sakura/src/drive.rs`・`emo2_boot/talk_clock.rs`。`\x` が無くても「台詞の終わりまで」で成り立つ。
  - 後半（仮の名前 `click-wait-tag`・`\x`／`\x[noclear]` と `clickwaitmarker.*`・11〜13）: `sakura/{decode,model}.rs`・`crates/areka-sakura/src/{compile,drive}.rs`・dola の `cue/{schedule,runtime}.rs`・emo-text の `state.rs`・`state_decoration.rs`・emo-present の `balloon.rs`・`balloon/{model,parse}.rs`・検査の見本 3 本・台帳。早送りが区切りで止まる所はこちらが足す。
- 先に要るもの: 働きの前提は満たした。列の順は `anchor-tag-canon` → `range-choice-tag`（同じ `decode.rs`・`compile.rs`）→ … → `text-typesetting` の後。印の系列は `balloon-markers` と共用（先に着地した方が作る）。
- 優先度の区分: A（roadmap「シェル内バルーン」節の開発者の確定 6「早送り」）。
- 要件定義のモデル: Fable（時刻の付け替え・スレッドをまたぐ届け方・ダブルクリックとの順）。
- 議題: 棚卸㉑・㉒の 4 件のまま（1 は上の材料つき）。
- 見つけた穴・古くなった記述: コードの穴は無い。棚卸㉒の行数（`msg.rs` 909 ほか）は上の数に読み替える。
