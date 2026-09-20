# Brief: areka-P0-balloon-break

> 起票: 2026-09-20（開発者指示・`/kiro-discovery`）。「バルーンをダブルクリックしたら再生中のトークが中断し、バルーンが消える」を建てる。さくらスクリプトの再生そのものが止まる点に留意（表示だけ消して裏で再生が続く形は不可）。

## Problem

利用者は、喋っている途中のゴーストを黙らせる手段を持たない。長い台詞が始まると、終わるまで（さらに表示終了から既定 30 秒のタイムアウトまで）バルーンが画面に居座る。SSP では「バルーンをダブルクリックすると台本が中断されバルーンが閉じる」が利用者の常識になっており、areka にはこの操作が無い。

## 正典の位置づけ（ukadoc）

操作そのものを定義する項は ukadoc に**無い**（開発者の指摘どおり）。ただし操作の**存在**は 4 か所が前提にしている（逐語引用）:

- `\![enter,nouserbreakmode]` — 「スクリプト実行中断(操作はSSPの設定次第。通常、バルーンダブルクリック)の無効化モード開始。」 <https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5benter_2cnouserbreakmode_5d:1>
- `\t` — 「スクリプトブレーク(例:選択肢を選ぶ、バルーンダブルクリックによる中断)か\eまでの間、マウス系などのイベント通知を行わない。」 <https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5ct:1>
- `OnBalloonBreak` — 「SSTP以外でブレイクされた際に発生。」Reference0＝中断の操作が起きたスクリプト／Reference1＝バルーンのスコープ番号／Reference2＝中断位置（タグ込みの文字数）。 <https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnBalloonBreak:1>
- `\![change,ghost,…,--option=raise-event]` — 「バルーンブレーク（通常ダブルクリック）による中止操作も可能な点に注意。」

よって本 spec は「正典が名前だけ出して中身を書いていない操作」を areka の規則として定める。SSP の実測は取らない（開発者方針）。規則は上の 4 項と矛盾しないことだけを条件にする。

## Current State（2026-09-20 実測・引用は「何の定義か」で指す）

- **バルーン窓のダブルクリックは捨てられている**。バルーン窓に付くハンドラは `crates/areka/src/input_events/balloon.rs` の `fn on_balloon_pointer_moved` と `fn on_balloon_pointer_pressed` の 2 本だけ。後者は左の単押しだけを選択肢の確定へ回し、`double_click` 欄は読まない（同関数の説明「DBLCLK 2 打目も独立 press として扱う」）。`OnMouseDoubleClick` はキャラクター窓専用（`input_events/mod.rs` の `fn attach_char_pointer_handlers`）。
- **どのバルーンか・スコープ番号は取れる**: `crates/areka/src/placement/spawn.rs` の `struct BalloonWindowMarker { scope }`。
- **再生を止める口は既にある**: `crates/areka-sakura/src/drive.rs` の `fn on_close` が `SakuraMsg::Close` を受けて `player.stop()`→`TalkDone{Interrupted}` を返す。製品コードでこれを送るのは `crates/areka-ghost/src/dispatcher.rs` の `fn on_cancel_choice` と `fn close_active_if_any` の 2 か所だけ。
- **kanade から「今のトークを止めろ」と言える口は 1 本だけ**: `Action::CancelChoice`（`crates/areka-kanade/src/schedule/mod.rs`）→ `TalkCommand::CancelChoice`（`crates/areka-talk/src/lib.rs`・`TalkCommand` は `Start`／`ResolveChoice`／`CancelChoice` の 3 値）。発行点は選択肢タイムアウトで SHIORI が無応答だったとき（`schedule/steady.rs` の `fn on_timeout_reply`）の 1 か所。
- **kanade は `Interrupted` を `Ended` と同じ「終了でない終わり」として扱う**（`schedule/mod.rs` のトーク終了の振り分け・`schedule/steady.rs` の `fn on_talk_done`・`schedule/close.rs`）。受け側は既に出来ている。
- **表示側はトークの終わりを知らない**: `crates/areka/src/emo2_boot/talk_lifecycle.rs` の `enum TalkLifecycleSignal` は `TalkStarted`／`DisplayEndAt` の 2 値。中断されると cue が来なくなるだけで、バルーンは既定 30 秒（`emo2_boot/balloon_visibility.rs` の `DEFAULT_BALLOON_TIMEOUT_SECS`）を待って消える。隠す理由の語彙 `enum VisibilityTrigger` は `Content`／`Clear`／`Timeout`／`Explicit`。
- **UI→kanade の送り口は 2 本ある**: マウス（`struct MouseWiring`→`KanadeMsg::Mouse`）と選択肢（`struct ChoiceSelectionInbox`→`fn drain_choice_selections`→`KanadeMsg::Choice`）。
- **中断禁止の語彙は状態トークンだけ**: `crates/areka-kanade/src/status.rs` の `ExecutionState::NoUserBreak` は一度も立たない（`fn derive` に注記だけの行）。`\![enter,nouserbreakmode]` は汎用 `\!` 運搬（`Instruction::GenericCommand`→`CueCommand::command_carrier`）で名前 `enter`・引数 `nouserbreakmode` の cue になるが、選んで読む消費者は 0。
- **中断位置（`OnBalloonBreak` の Reference2）の源は無い**: `dola` の `Cue`／`TalkCue` は台本上の位置を持たず、`drive.rs` は compile 後に台本文字列を保持しない。
- **SSTP は未実装**＝再生中のトークは必ず SHIORI 応答起点で kanade を通っている。

## Desired Outcome

- バルーン窓を左ダブルクリックすると、再生中のさくらスクリプトがその場で止まり（以降のタグ・文字・待ちは一切実行されない）、全スコープのバルーンが即座に消える。30 秒のタイムアウトを待たない。
- 中断後、kanade は定常へ戻り、次のトーク（ランダムトーク・イベント応答）を普通に始められる。
- `\![enter,nouserbreakmode]`〜`\![leave,nouserbreakmode]` の区間ではダブルクリックしても中断されない。
- 判断分岐（中断する／しない）が決定論テストで固定されている。

## Approach

**採用: kanade 経由で止める。** バルーン窓のダブルクリックを UI→kanade の新しい通知 1 種として送り、kanade が「トーク再生中か」を見て既存の停止経路（dispatcher の `SakuraMsg::Close`→`TalkDone{Interrupted}`）へ流す。表示側には「中断された」を伝える信号を 1 値足し、全バルーンを即時に隠す。

- なぜ kanade 経由か: トークの有無と識別子を持っているのは kanade だけ。停止は既存の単一の閉じ口へ寄せる（アーキテクチャ決定「talk 中断は単一 Close funnel」）。後続の `OnBalloonBreak` 発火（#35）も kanade が起点になる。
- **却下 B: UI から dispatcher／sakura へ直接 Close を送る** — kanade の台帳（選択肢・`pending_close`）と食い違う第 2 の停止経路が出来る。
- **却下 C: 表示だけ隠す** — 再生が裏で続き、`\![raise]` や面替えが実行され続ける。開発者の留意点に正面から反する。

隠す時刻の決め方（kanade の受理を待つか・UI が先に隠すか）は設計で決める。ただし「1 フレーム遅らせる」形の解は取らない（開発者方針）。

## Scope

- **In**:
  1. バルーン窓の左ダブルクリックの検出（スコープ番号つき）と UI→kanade の通知。
  2. kanade の受理規則（再生中なら止める／再生中でなければ何も止めない）と、既存の停止経路への接続。`TalkCommand::CancelChoice` を一般化するか新値を足すかは設計。
  3. 表示側の即時非表示（全スコープ）。
  4. `\![enter,nouserbreakmode]`／`\![leave,nouserbreakmode]` による中断の無効化（区間の旗 1 本）。
- **Out**:
  - `OnBalloonBreak`／`OnBalloonClose`／`OnBalloonTimeout` の SHIORI 発火と `BalloonLifecycleNotice` の構築＝`areka-P0-balloon-lifecycle-events`（台帳 #35）の項目 8・10。本 spec は「中断が起きた・どのスコープで起きた」を kanade まで届けるところまで。中断位置（Reference2）の源を作るのも #35。
  - `status` の `nouserbreak`／`timecritical` トークンを立てること＝`areka-P0-status-execution-states`。本 spec は旗を持つだけで、読み口を塞がない。
  - `\t`（タイムクリティカル）の実装。正典上 `\t` は中断を禁じない（中断で**解ける**側）ので本 spec の規則には影響しない。
  - 中断操作の割り当て変更（SSP の「設定次第」）。左ダブルクリック固定。
  - キャラクター窓のダブルクリック（従来どおり `OnMouseDoubleClick`）。

## 要件ディスカッションで裁定する分かれ目

1. **`OnBalloonBreak` を本 spec に入れるか**（推奨: 入れない）。Reference2 の源が無く、台本位置を compile→cue→再生へ通す工事が要る。#35 は 8・10 を一体で持っており、そこへ「中断の起点が出来た」を相互登記するのが最小。入れる場合は規模 S→M。
2. **`nouserbreakmode` を入れるか**（推奨: 入れる）。中断操作だけ建てて無効化を建てないと、「ここは止めさせない」と宣言しているゴースト（消滅・初回起動の演出）を areka だけが止めてしまう。旗 1 本で済む。
3. **選択肢の行の上でのダブルクリック**（推奨: 中断しない）。1 打目が既に選択の確定として消費されるため、2 打目を中断に数えると「選んだ直後に応答が消える」。
4. **選択待ち中のバルーン余白のダブルクリック**（推奨: 中断する・`OnChoiceTimeout` は出さない）。
5. **再生が終わって居残っているだけのバルーンのダブルクリック**（推奨: 隠す。止める対象は無いので kanade は何もしない）。利用者から見れば「ダブルクリックでバルーンが消える」は同じ操作。
6. **`OnClose` の別れの台詞を中断したとき**: 既存規則では `Interrupted` は `Ended` と同じく終了拒否へ落ちる（`schedule/close.rs`）。`\-` に届かず終了しない、で据え置くか確認。

## Boundary Candidates

- 入力の検出（`input_events/balloon.rs`）／kanade の受理と停止（`areka-kanade` `schedule/`・`areka-talk`・`areka-ghost` dispatcher）／表示の即時非表示（`emo2_boot/`）／無効化の旗（`\!` 運搬 cue の読み口）の 4 片。

## Out of Boundary

- 可視性の既存規則（`balloon-visibility` で完成）の変更。足すのは「中断」という隠す理由 1 つだけ。
- `areka-sakura` の `fn on_close` の停止意味論（完成済み・不変）。

## Upstream / Downstream

- **Upstream**: 完了 `input-events`・`choice-interact`（バルーン窓のハンドラ）・`sakura-engine`（Close→`Interrupted`）・`balloon-visibility`（隠す理由の語彙）。先行必須の未完 spec は無い。
- **Downstream**: `areka-P0-balloon-lifecycle-events`（#35・`OnBalloonBreak` の起点として本 spec の通知を使う。項目 10「中断で終わった会話のタイムアウト起点」は、利用者による中断では即時非表示になるため対象が先取り・選択肢タイムアウト等の残りだけになる）・`areka-P0-status-execution-states`（旗の読み口）。

## Existing Spec Touchpoints

- **Extends**: なし（新しい境界）。
- **Adjacent**: `balloon-lifecycle-events`・`translate-pipeline`・`sakura-time-directives`（いずれも `areka-kanade/src/schedule/` を触る＝並走させない）／`status-execution-states`（`emo2_boot/mod.rs`・`status.rs`）／`popup-menu-residue`（入力ハンドラの隣）。

## Constraints

- 編集集合の見込み: `crates/areka/src/input_events/balloon.rs`・`crates/areka-kanade/src/{msg.rs,actor.rs,schedule/{mod,steady}.rs}`・`crates/areka-talk/src/lib.rs`・`crates/areka-ghost/src/dispatcher.rs`・`crates/areka/src/emo2_boot/{talk_lifecycle,balloon_visibility}.rs`。規模 S〜M。
- 決定論テスト必達。固定するのは判断分岐だけ: 再生中／非再生／選択待ち／無効化区間／選択肢行の上、の 5 分岐と「中断後に次のトークが始まる」。既に証明済みの配線（Close→`Interrupted`）は再テストしない。
- 失敗経路は必ずログ（送り先が落ちている等は `error!`）。
- 実機サインオフ 1 件: emo2 で長台詞の途中をダブルクリック→文字送りが止まりバルーンが即時に消える・次のランダムトークが出る。
- 要件定義は Opus で足りる（裁定は上の 6 件・いずれも利用者から見える結果で語れる）。
