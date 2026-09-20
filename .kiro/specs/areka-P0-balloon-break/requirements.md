# Requirements Document

> 本文の実測は **2026-09-20・本ブランチ**のもの。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。
> **本文に「仮の裁定」と記した箇所は、この後の要件ディスカッションで開発者が確認する既定である。** 覆されない限り、この既定のまま設計・実装へ進む。

## Introduction

### 誰が困っているか

areka の利用者。**喋っている途中のゴーストを黙らせる手段を持たない。**

### いま何が起きているか（2026-09-20 実測）

- 長い台詞が始まると、台本が終わるまで、さらに表示が終わってから既定 30 秒（`crates/areka/src/emo2_boot/balloon_visibility.rs` の `DEFAULT_BALLOON_TIMEOUT_SECS`）が過ぎるまで、バルーンが画面に居座る。
- **バルーン窓のダブルクリックは捨てられている。** バルーン窓に付くポインタのハンドラは `crates/areka/src/input_events/balloon.rs` の `fn on_balloon_pointer_moved` と `fn on_balloon_pointer_pressed` の 2 本だけで（付けるのは同ファイルの `fn attach_balloon_pointer_handlers`）、後者は左の単押しだけを選択肢の確定へ回し、ダブルクリックを示す欄を読まない（同関数の説明「`double_click` フィールドは一切参照しない」）。ダブルクリックを SHIORI へ届ける経路はキャラクター窓専用である（`crates/areka/src/input_events/mod.rs` の `fn attach_char_pointer_handlers` はキャラクター窓にだけハンドラを付け、ダブルクリックの判定はその押下ハンドラ `fn on_char_pointer_pressed` の中の 1 本の枝として `OnMouseDoubleClick` を送る。ダブルクリック専用のハンドラという別立てのものは無い）。
- **どのバルーンで起きたかは取れる。** バルーン窓はスコープ番号を持っている（`crates/areka/src/placement/spawn.rs` の `struct BalloonWindowMarker`）。
- **再生を止める口は既にある。** `crates/areka-sakura/src/drive.rs` の `fn on_close` が閉じ指示を受けて再生を止め、「中断で終わった」という終わり方を返す。製品コードでこの閉じ指示を送るのは `crates/areka-ghost/src/dispatcher.rs` の `fn on_cancel_choice` と `fn close_active_if_any` の 2 か所だけである。
- **会話の運行側（kanade）から「今のトークを止めろ」と言える口は 1 本だけ**で、その発行点は選択肢の待ち時間が尽きて SHIORI が応答しなかったときの 1 か所（`crates/areka-kanade/src/schedule/steady.rs` の `fn on_timeout_reply`）である。利用者の操作から到達する経路は **0 本**。
- **受け側は既に出来ている。** kanade は「中断で終わった」を「最後まで終わった」と同じ「終了ではない終わり」として扱い、定常へ戻る（`crates/areka-kanade/src/schedule/steady.rs` の `fn on_talk_done`）。
- **表示側はトークの終わりを知らない。** 表示側が受け取る合図は「トークが始まった」と「表示の終わる時刻」の 2 つだけ（`crates/areka/src/emo2_boot/talk_lifecycle.rs` の `enum TalkLifecycleSignal`）。中断されると続きの指示が来なくなるだけで、バルーンは既定 30 秒を待ってから消える。バルーンを隠す理由の語彙（同 `balloon_visibility.rs` の `enum VisibilityTrigger`）に「中断された」は **無い**。
- **中断を禁じる語彙は状態の名前だけ。** `crates/areka-kanade/src/status.rs` の `ExecutionState::NoUserBreak` は一度も立たない（同ファイルの `fn derive` が組み立てるのは `Talking` と `Choosing` の 2 つだけ）。`\![enter,nouserbreakmode]` は汎用の `\!` 運搬（`Instruction::GenericCommand` → `CueCommand::command_carrier`）に名前 `enter`・引数 `nouserbreakmode` として載って最後まで運ばれるが、運ばれたものを引き取る消費者の表（`crates/areka/src/emo2_boot/consumer_ledger.rs` の `ConsumerLedger::canonical`）に登記されているのは 6 行（`move`・`bind`・`set,zorder`・`reset,zorder`・`open,readme`・フォントタグの運搬）で、名前 `enter`／`leave` の行は **0 行**である。
- **中断位置の源は無い。** 再生の指示（dola の cue）は台本上の位置を持たず、`drive.rs` は台本を組み立てた後に台本の文字列を保持しない。
- **SSTP は未実装。** 再生中のトークは必ず SHIORI の応答を起点として kanade を通っている。

### 正典（ukadoc）の位置づけ

**この操作そのものを定義する項は ukadoc に無い。** ただし操作が存在することは 4 か所が前提にしている（逐語引用）:

| 正典 | 逐語引用 |
|---|---|
| [`\![enter,nouserbreakmode]`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5benter_2cnouserbreakmode_5d:1) | 「スクリプト実行中断(操作はSSPの設定次第。通常、バルーンダブルクリック)の無効化モード開始。通常のSSTPでは使用不可(Auth.SSTP(= Owned SSTP)は可)。」 |
| [`\![leave,nouserbreakmode]`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bleave_2cnouserbreakmode_5d:1) | 「上記を解除する。」 |
| [`\t`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5ct:1) | 「スクリプトブレーク(例:選択肢を選ぶ、バルーンダブルクリックによる中断)か\eまでの間、マウス系などのイベント通知を行わない。」 |
| [`OnBalloonBreak`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnBalloonBreak:1) | 「SSTP以外でブレイクされた際に発生。」Reference0＝中断の操作が起きたスクリプト／Reference1＝バルーンのスコープ番号／Reference2＝中断位置（タグ込みの文字数）。 |
| [`\![change,ghost,…]`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bchange_2cghost_5d:1) | 「バルーンブレーク（通常ダブルクリック）による中止操作も可能な点に注意。」 |

よって本仕様は「正典が名前だけ出して中身を書いていない操作」を areka の規則として定める。**SSP の実測は取らない**（開発者方針）。規則は上の 5 項と矛盾しないことだけを条件にする。正典が「操作は設定次第」と書いている部分について、areka は**左ダブルクリック固定**と定め、割り当てを替える仕組みは持たない。

### 何を変えるか

バルーン窓を**左ダブルクリック**すると、再生中のさくらスクリプトが**その場で止まり**（以降のタグ・文字・待ちは一切実行されない）、**全スコープのバルーンが即座に消える**。30 秒のタイムアウトを待たない。中断した後、ゴーストは定常へ戻り、次のトーク（ランダムトーク・イベント応答）を普通に始める。`\![enter,nouserbreakmode]`〜`\![leave,nouserbreakmode]` の区間では、ダブルクリックしても中断されない。

**表示だけ消して裏で再生が続く形は採らない**（開発者の留意点）。裏で再生が続くと `\![raise]` や面替えが実行され続け、利用者から見て「黙らせた」ことにならないためである。

## Boundary Context

- **In scope**（利用者・ゴースト作者から見える範囲）:
  - バルーン窓の左ダブルクリックの検出と、それが「どのスコープのバルーンで起きたか」を会話の運行側まで届けること。
  - 受理の規則: 再生中なら止める／再生中でなければ何も止めない／無効化の区間では止めない／選択肢の行の上では止めない。
  - 止まった後にゴーストが定常へ戻り、次のトークを普通に始めること。
  - 全スコープのバルーンの即時非表示（タイムアウトを待たない）。
  - `\![enter,nouserbreakmode]`／`\![leave,nouserbreakmode]` による中断の無効化。
  - 失敗経路の記録と、判断分岐の決定論テスト。
  - 網羅台帳の担当欄への登記（2 項目）。
- **Out of scope**:
  - `OnBalloonBreak`／`OnBalloonClose`／`OnBalloonTimeout` の SHIORI 発火と、そのための受け渡し口の構築（**仮の裁定・裁定 1**）。`areka-P0-balloon-lifecycle-events`（台帳 #35）の項目 8・10 が持つ。本仕様は「中断が起きた・どのスコープで起きた」を運行側まで届けるところまでで、SHIORI へは **1 件も送らない**。
  - **中断位置（`OnBalloonBreak` の Reference2）の源を作ること。** 台本上の位置を、台本を組み立てるところから再生まで通す工事は #35 に残る（#35 の brief に相互登記済み）。
  - `status` の実行状態トークン（`nouserbreak`・`timecritical`）を立てて SHIORI へ見せること＝`areka-P0-status-execution-states`。本仕様は無効化の旗を持つだけで、読み口を塞がない。
  - `\t`（タイムクリティカル）の実装。正典上 `\t` は中断を禁じる側ではなく、中断で**解ける**側なので、本仕様の規則に影響しない。担当は `areka-P0-status-execution-states`（台帳に登記済み）。
  - 中断操作の割り当ての変更（正典の「SSPの設定次第」に当たる部分）。左ダブルクリック固定で、設定は持たない。
  - SSTP 経由の中断禁止（正典の「通常のSSTPでは使用不可」）。SSTP が未実装なので、**判定も記録も 1 行も置かない**。
  - キャラクター窓のダブルクリック（従来どおり `OnMouseDoubleClick` を送る。**変更なし**）。
  - バルーン窓の右ボタンの振る舞い（**変更なし**）。
  - バルーンの可視性の既存規則（完了済み `balloon-visibility`）の変更。足すのは「中断された」という隠す理由 1 つだけ。
  - 再生を止める意味論そのもの（`crates/areka-sakura/src/drive.rs` の `fn on_close`。完成済み・不変）。
- **Adjacent expectations**:
  - **前提（いずれも完了済み・先行必須の未完 spec は無い）**: `areka-P0-input-events`（バルーン窓へのポインタ結線）・`choice-interact`（選択肢の確定）・`sakura-engine`（閉じ指示で再生が止まり「中断で終わった」を返すこと）・`balloon-visibility`（隠す理由の語彙とタイムアウト）。
  - **下流**: `areka-P0-balloon-lifecycle-events`（#35・本仕様が届けた通知を `OnBalloonBreak` の起点に使う。項目 10「中断で終わった会話のタイムアウト起点」は、利用者による中断では即時非表示になるため、対象が選択肢タイムアウトなど残りの中断だけになる）・`areka-P0-status-execution-states`（無効化の旗の読み口）。
  - **並走させない**（同じ `crates/areka-kanade/src/schedule/` を触る）: `areka-P0-balloon-lifecycle-events`・`areka-P0-translate-pipeline`・`areka-P0-sakura-time-directives`。
  - **隣接**: `areka-P0-popup-menu-residue`（入力ハンドラの隣）。

## Requirements

### Requirement 1: バルーンの左ダブルクリックが中断の合図になる

**Objective:** 利用者として、喋っている途中のゴーストをバルーンのダブルクリックで黙らせたい。そうすれば、長い台詞を最後まで聞かされずに済み、SSP で身についた操作がそのまま通じる。

#### Acceptance Criteria

1. When バルーン窓の上で左ボタンのダブルクリックが検出された, the areka shall そのバルーンのスコープ番号を添えて「利用者が中断を求めた」ことを会話の運行側へ 1 件だけ伝える。
2. The areka shall 中断の合図を**左ボタンのダブルクリックだけ**と定める。右ボタン・中ボタン・単押し・長押し・3 打目以降からは中断の合図を作らない（正典が「操作はSSPの設定次第」と書いている部分に対する本仕様の裁定であり、割り当てを替える設定は持たない）。
3. The areka shall キャラクター窓のダブルクリックの扱いを変えない（従来どおり `OnMouseDoubleClick` を SHIORI へ送り、中断の合図にはしない）。
4. The areka shall バルーン窓の右ボタンの扱いを変えない。
5. When 1 打目の左押下が選択肢の確定として消費された, the areka shall その続きの 2 打目から中断の合図を作らない（**仮の裁定・裁定 3**）。選んだ直後に応答のバルーンが消えることを避けるためである。
6. While 選択肢を待っている状態でも、ダブルクリックが選択肢の行の上でない（バルーンの余白の上である）, the areka shall 中断の合図を作る（**仮の裁定・裁定 4**）。
7. If 中断の合図を運行側へ渡せなかった（受け手が既に居ない等）, then the areka shall 失敗を `error!` で記録し、再生を止めずに続ける（記憶 areka-log-first-no-silent-failure）。
8. While ゴーストの起動が完了して入力の結線が済む前, the areka shall 中断の合図を作らず、`trace!` で記録して何もしない（既存の自己抑止と同じ扱い）。
9. While バルーンが画面に出ていない（既に隠れている・クリックが素通りする状態にある）, the areka shall 中断の合図を作らない。利用者はそこに何も見えていないので、ダブルクリックはバルーンへの操作ではない。

### Requirement 2: 中断を受け入れるかどうかの規則

**Objective:** 利用者として、ダブルクリックしたときに「止まる」「止まらない」がいつも同じ理屈で決まってほしい。ゴースト作者として、「ここは止めさせない」と書いた区間が守られてほしい。

#### Acceptance Criteria

1. When 中断の合図が届き、さくらスクリプトを再生中である, the areka shall その再生を止める（要件 3）。
2. When 中断の合図が届き、再生中のさくらスクリプトが無い（表示が終わって居残っているだけのバルーンである）, the areka shall **何も止めず**、バルーンを隠すだけにする（要件 4.2・**仮の裁定・裁定 5**）。利用者から見れば「ダブルクリックでバルーンが消える」は同じ操作である。
3. While 中断の無効化の区間にある, the areka shall 中断の合図を受けても再生を止めず、バルーンも隠さず、`debug!` で 1 行記録して何もしない（要件 5）。表示だけ隠すと裏で再生が続くことになり、本仕様の目的に反するためである。
4. When 中断の合図が既に受け入れられた直後に、同じ操作の合図がもう 1 件届いた, the areka shall 止める対象が無いものとして扱い（要件 2.2 と同じ）、二重に止めない。
5. The areka shall 中断を受け入れるかどうかを**再生中かどうか**だけで決め、どのスコープのバルーンがダブルクリックされたかでは変えない。スコープ番号は「どこで起きたか」を伝えるためだけに運ばれる（下流 #35 が `OnBalloonBreak` の Reference1 に使う）。
6. When 選択肢を待っている最中に中断が受け入れられた, the areka shall 待っていた選択肢を破棄して定常へ戻し、選択肢の時間切れの扱い（`OnChoiceTimeout` の発火）を**行わない**（**仮の裁定・裁定 4**）。利用者は選ばずに会話を終わらせたのであって、時間切れになったのではない。
7. The areka shall 中断で止めるものを**いま再生している台本だけ**とする。中断の時点で既に SHIORI へ出してある要求（選択の確定・選択肢の時間切れの通知）の応答が後から届いたときは、既存の規則のまま扱い（応答に台本があれば次のトークとして始まる）、中断を理由にその応答を捨てる仕組みを **1 つも足さない**。利用者が自分で選んだ選択肢の応答は、利用者が求めたものだからである。

### Requirement 3: 再生がその場で止まる

**Objective:** 利用者として、ダブルクリックした瞬間に本当に喋るのをやめてほしい。表示だけ消えて裏で面が替わり続けるのでは、黙らせたことにならない。

#### Acceptance Criteria

1. When 中断が受け入れられた, the areka shall 再生中のさくらスクリプトを止め、**中断した位置より後ろのタグ・文字・待ちを 1 つも実行しない**（面替え・`\![raise]`・`\!` の各コマンド・文字送り・`\w` の待ちを含む）。
2. The areka shall 既存の単一の閉じ口（`crates/areka-ghost/src/dispatcher.rs` が送る閉じ指示 → `crates/areka-sakura/src/drive.rs` の `fn on_close`）を使って止め、それを迂回する第 2 の停止経路を作らない（アーキテクチャ決定「talk 中断は単一 Close funnel」）。
3. When 再生が止まった, the areka shall その終わり方を、既に存在する「中断で終わった」という終わり方として運行側へ返す（新しい終わり方の種類を増やさない）。
4. When 中断で再生が止まった, the areka shall 定常へ戻り、次のトーク（ランダムトーク・イベント応答）をそれまでと同じ規則で始める。
5. The areka shall 中断のときに、そのとき出ていた面（サーフェス）を元へ戻さない。止めた時点の面のまま残す（正典に戻す規定は無く、「止める」以上のことをしないため）。
6. When 終了の挨拶（`OnClose` の応答として再生している別れの台詞）の最中に中断が受け入れられた, the areka shall 既存の規則どおり「中断で終わった」を「最後まで終わった」と同じに扱い、**ゴーストを終了させずに定常へ戻す**（**仮の裁定・裁定 6**・`crates/areka-kanade/src/schedule/close.rs` の既存の終了拒否の経路。この同一視は既に決定論テストで固定されている＝同ファイルのテスト `value_then_interrupted_refuses_close_same_as_ended`）。利用者から見ると「終了しようとしたが、別れの台詞を途中で切ったので終了が取り消される」という結果になる。
7. When 中断が受け入れられた, the areka shall SHIORI へイベントを **1 件も送らない**（`OnBalloonBreak` は裁定 1 により本仕様の範囲外。中断が起きたことは運行側まで届くだけで、そこから先へは進まない）。

### Requirement 4: バルーンが即座に消える

**Objective:** 利用者として、ダブルクリックしたらバルーンがその場で消えてほしい。30 秒待たされるのでは、黙らせた感じがしない。

#### Acceptance Criteria

1. When 中断が受け入れられた, the areka shall **そのとき画面に出ているバルーンをすべて**隠す（本文の「全スコープ」はこの意味で使う）。中断の合図が起きたスコープのバルーンだけを隠すのではない。既に隠れているバルーンへは隠す指示を重ねて出さない。
2. When 中断の合図が届いて止める再生が無かった（要件 2.2）, the areka shall 同じく全スコープのバルーンを隠す。
3. The areka shall 隠すまでに既定 30 秒のタイムアウト（`DEFAULT_BALLOON_TIMEOUT_SECS`）を待たない。
4. The areka shall 「中断された」をバルーンを隠す理由の 1 つとして持ち、既存の理由（内容による表示・消去・時間切れ・明示的な指示）の意味を変えない。
5. The areka shall 隠す時刻の決め方を設計に委ねる。ただし**「1 フレーム遅らせる」形の解は採らない**（開発者方針・記憶 no-frame-delay-fixes-change-the-state-shape）。要件が定めるのは「利用者がダブルクリックしてから、次に画面が描かれるときにはもうバルーンが無い」ことである。
6. When 中断でバルーンを隠した後にタイムアウトの時刻が来た, the areka shall 既に隠れているバルーンをもう一度隠そうとせず、**本仕様が新しく足す記録を 1 行も出さない**。可視性の既存規則が元から出している記録（出ているバルーンが無くなったのでタイムアウトの計測を捨てた、の 1 行）は既存どおりで、本仕様は足しも消しもしない。
7. When 中断の後に次のトークが始まった, the areka shall そのトークのバルーンを通常どおり表示する（中断は次の表示を妨げない）。
8. While 中断でバルーンを隠してから次のトークが始まるまでの間, the areka shall 隠したバルーンを**独りでに出し直さない**。止めた台本の文字のうち既に表示側へ届いていた分は、再生を止めた後も時刻の進行だけで見える文字数が増えうる（文字の出る時刻は塊が届いた時点で先まで決まっている＝`crates/areka-emo-text/src/state.rs` の `RevealSchedule`）。バルーンを出す既存規則は「見える文字数が増えた、かつ今は隠れている」（`crates/areka/src/emo2_boot/balloon_visibility.rs` の `fn decide_content`）なので、塞がなければ**文の途中で止めたときだけ、消えたバルーンが次の描画で戻ってくる**（待ちの最中に止めたときは起きない＝走行ごとに出たり出なかったりする）。塞ぎ方は設計が決める。

### Requirement 5: 中断の無効化（`nouserbreakmode`）

**Objective:** ゴースト作者として、消滅の演出や初回起動の案内など「ここは止めさせたくない」区間を守りたい。そうすれば、SSP 向けに書いた `\![enter,nouserbreakmode]` が areka でも効く。

#### Acceptance Criteria

1. When 再生中の台本に `\![enter,nouserbreakmode]` が現れた, the areka shall そこから中断の無効化の区間に入る（正典 [\\![enter,nouserbreakmode]](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5benter_2cnouserbreakmode_5d:1)「スクリプト実行中断(操作はSSPの設定次第。通常、バルーンダブルクリック)の無効化モード開始。」）。
2. When 再生中の台本に `\![leave,nouserbreakmode]` が現れた, the areka shall 無効化の区間から出る（正典 [\\![leave,nouserbreakmode]](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bleave_2cnouserbreakmode_5d:1)「上記を解除する。」）。
3. While 無効化の区間にある, the areka shall バルーンのダブルクリックで再生を止めず、バルーンも隠さない（要件 2.3）。
4. When 無効化の区間に入ったまま `\![leave,nouserbreakmode]` が現れずにトークが終わった, the areka shall そのトークの終わりで無効化を解く（**仮の裁定・正典は沈黙**）。解かないと、閉じ忘れた台本 1 本のせいでそのゴーストが以後ずっと止められなくなり、利用者が黙らせる手段を永久に失うためである。
5. When 既に無効化の区間にある最中にもう一度 `\![enter,nouserbreakmode]` が現れた, the areka shall 区間に入ったままとして扱う（入れ子を数えない）。`\![leave,nouserbreakmode]` が 1 回現れれば区間から出る。
6. When 無効化の区間に入っていないのに `\![leave,nouserbreakmode]` が現れた, the areka shall 何もせず `debug!` で 1 行記録する。
7. The areka shall 無効化の状態を SHIORI の `status` へ見せない（`nouserbreak` トークンを立てるのは `areka-P0-status-execution-states` の仕事）。ただし、本仕様が持つ旗をその spec が読めるようにしておき、読み口を塞がない。
8. The areka shall SSTP 経由の実行に対する無効化の例外（正典の「通常のSSTPでは使用不可(Auth.SSTP(= Owned SSTP)は可)」）について、**判定を 1 つも置かない**。SSTP が未実装で、再生中のトークは必ず SHIORI 応答起点だからである。

### Requirement 6: 記録

**Objective:** 開発者として、ダブルクリックしたのに止まらなかった理由をログから読み取りたい。

#### Acceptance Criteria

1. When 中断が受け入れられて再生を止めた, the areka shall `info!` で 1 行（スコープ番号）を記録する。
2. When 中断の合図が届いたが止める再生が無かった, the areka shall `debug!` で 1 行（スコープ番号・理由）を記録する。
3. When 中断の合図が無効化の区間のために退けられた, the areka shall `debug!` で 1 行（スコープ番号・理由）を記録する。
4. The areka shall 失敗の経路（合図を渡せない・停止の指示を送れない・表示側へ隠す指示を渡せない）をすべて `error!` で記録し、**記録の無い失敗経路を 1 つも持たない**（記憶 areka-log-first-no-silent-failure・steering `logging.md`）。
5. The areka shall 記録の書式を steering `logging.md`（構造化フィールド・スコープ接頭辞）に従わせる。
6. The areka shall ダブルクリックそのもの（受理／不受理を判定する前の検出）を `trace!` 以下に留め、通常の運転でログを埋めない。

### Requirement 7: 決定論テストと実機確認

**Objective:** 開発者として、判断の分かれ目を毎回のテストで固定し、画面と OS に触れる部分だけを実機で 1 度確かめたい。

#### Acceptance Criteria

1. The areka shall 次の **6 つの判断分岐**を決定論テストで固定する（記憶 test-only-decision-branches-not-proven-wiring・既に証明済みの配線は再テストしない）:
   ⑴ 再生中 → 止まる（要件 2.1・3.1）
   ⑵ 再生中でない → 止める対象が無く、隠すだけ（要件 2.2）
   ⑶ 選択肢を待っている最中の余白 → 止まり、`OnChoiceTimeout` を送らない（要件 2.6）
   ⑷ 無効化の区間 → 止まらず隠れない（要件 2.3・5.3）
   ⑸ 選択肢の行の上の 2 打目 → 合図を作らない（要件 1.5）
   ⑹ 中断で隠した後、次のトークが始まる前に見える文字数が増えた → 出し直さない／次のトークが始まった後に増えた → 出す（要件 4.8・4.7）
2. The areka shall 「中断の後に次のトークが普通に始まる」ことを決定論テストで確かめる（要件 3.4）。
3. The areka shall 無効化の区間の出入り（入る・出る・閉じ忘れたままトークが終わる・区間外の `leave`）を決定論テストで確かめる（要件 5.1・5.2・5.4・5.6）。
4. The areka shall 「全スコープのバルーンが隠れる」こと（要件 4.1）を、画面を描かずに確かめられる形（隠す指示の観測）で固定する。
5. The areka shall 上のテストが判断分岐を壊すと赤になることを、少なくとも 1 つの分岐で摂動して示す（記憶 cage-must-walk-the-reachable-path・checks-must-judge-not-just-print・mutate-by-replacing-not-translating＝平行移動ではなく経路から外す形で摂動する）。
6. The areka shall テストを実装と同じディレクトリの兄弟ファイルへ置く（記憶 areka-bin-crate-internal-tests-in-crate。既存の `crates/areka/src/input_events/balloon_pointer_handler_tests.rs`・`crates/areka-kanade/src/schedule/steady_choice_tests.rs` 等と同じ並び）。
7. The areka shall 既に決定論テストで固定されている配線を**もう一度テストしない**（記憶 test-only-decision-branches-not-proven-wiring）。零も明示する＝再テストしないのは次の 2 つである: ⑴ 閉じ指示 → 再生停止 →「中断で終わった」を返す経路（`crates/areka-sakura/src/drive_lifecycle_tests.rs`）⑵ 別れの台詞が「中断で終わった」ときに終了を拒む同一視（`crates/areka-kanade/src/schedule/close.rs` のテスト `value_then_interrupted_refuses_close_same_as_ended`）。
8. The areka shall 新設・改変したファイルを 1 ファイル 1,000 行以内に収める（番人は `crates/log-capture-kit/tests/file_length_guard_test.rs`）。**2026-09-20 時点で `crates/areka/src/input_events/balloon.rs` は 917 行・`crates/areka-kanade/src/schedule/steady.rs` は 935 行**であり、いずれも残りが 100 行を切っている。行を足せない場合の分割は設計が決める。
9. The areka shall 実機サインオフを **1 件**行い、確認項目と結果を tasks の完了記録に残す: emo2（絶対パス起動・記憶 areka-emo2-signoff-needs-absolute-paths）で長い台詞の途中にバルーンを左ダブルクリックし、⑴ 文字送りがその場で止まる ⑵ バルーンが即座に消える（30 秒待たない）⑶ その後に次のランダムトークが普通に出る、の 3 点を確かめる。

### Requirement 8: 網羅台帳への登記

**Objective:** 網羅調査の読み手として、本仕様が正典のどの項目を引き受けたかが台帳で分かってほしい。

#### Acceptance Criteria

1. The 本仕様 shall 網羅台帳の担当欄（`owner`）に本仕様の名前を登記する項目を次の **2 項目**と定める（いずれも `doc/ukadoc-coverage/ledger/sakura-script.toml`・2026-09-20 時点で `status = "absent"`・`owner = ""`）: `\![enter,nouserbreakmode]`・`\![leave,nouserbreakmode]`。実装が着地したらこの 2 項目を実装済みへ改める。
2. The 本仕様 shall 次のものを**担当にしない**（零も明示する。理由付き）:
   - `OnBalloonBreak`（`doc/ukadoc-coverage/ledger/shiori.toml`・裁定 1 で本仕様の範囲外。担当は既に登記されており、#35 が引き受ける）。
   - `\t`（担当は既に `areka-P0-status-execution-states`）。
   - `\![enter,onlinemode]`／`\![leave,onlinemode]` など、`\!` 運搬の他の名前（本仕様は `nouserbreakmode` の 2 つだけを読む）。
   - 中断の操作そのもの（**ukadoc に項が無いので、登記できる台帳の行が 0 行である**）。
3. When 担当欄を登記する, the 本仕様 shall 同じコミットで `doc/ukadoc-coverage/roadmap-draft.md` の `[[spec]]` に本仕様の行（`owner_count`＝台帳の数え直し）を足し、`[briefs].count` と本文の手書きの数を**実際に数え直した値**で直し（記憶 zeros-must-be-stated-not-implied・引き算で合わせない）、報告を作り直して `cargo test -p ukadoc-survey` が緑であることを確かめる。
4. The 本仕様 shall 登記した 2 項目の定義箇所に正典 URL のコメントを 1 行ずつ置く（既存の慣行・`doc/ukadoc-coverage/README.md`）。

### Requirement 9: 裁定候補と既定

**Objective:** 開発者として、答えで作業が変わる分かれ目だけを渡され、それぞれに既定が置かれていてほしい。そうすれば、覆さない限り実装は止まらない。

#### Acceptance Criteria

1. The 本仕様 shall 裁定 1「`OnBalloonBreak` の発火を本仕様に入れるか」を**入れない**で進める。中断位置（Reference2）の源が無く、台本上の位置を「組み立て → 再生の指示 → 再生」へ通す工事が要る。#35 は項目 8 と 10 を一体で持っており、そこへ「中断の起点が出来た」を相互登記する（#35 の brief に登記済み）のが最小である。入れる場合は規模が S から M へ上がる。
2. The 本仕様 shall 裁定 2「`nouserbreakmode` を入れるか」を**入れる**で進める（要件 5）。中断の操作だけ建てて無効化を建てないと、「ここは止めさせない」と宣言しているゴースト（消滅・初回起動の演出）を areka だけが止めてしまう。旗 1 本で済む。
3. The 本仕様 shall 裁定 3「選択肢の行の上でのダブルクリック」を**中断しない**で進める（要件 1.5）。1 打目が既に選択の確定として消費されるため、2 打目を中断に数えると「選んだ直後に応答が消える」ことになる。
4. The 本仕様 shall 裁定 4「選択待ち中のバルーン余白のダブルクリック」を**中断する・`OnChoiceTimeout` は送らない**で進める（要件 1.6・2.6）。
5. The 本仕様 shall 裁定 5「再生が終わって居残っているだけのバルーンのダブルクリック」を**隠す（止める対象は無いので何も止めない）**で進める（要件 2.2・4.2）。利用者から見れば「ダブルクリックでバルーンが消える」は同じ操作である。
6. The 本仕様 shall 裁定 6「`OnClose` の別れの台詞を中断したとき」を**既存規則のまま据え置く**で進める（要件 3.6）。中断で終わった別れの台詞は終了拒否へ落ち、ゴーストは終了しない。
7. The 本仕様 shall 裁定 7「無効化の区間を閉じずにトークが終わったとき」を**トークの終わりで解く**で進める（要件 5.4）。正典は沈黙しており、解かない側を採ると閉じ忘れ 1 本でそのゴーストが永久に止められなくなる。
8. Where 開発者が上の既定を覆した, the 本仕様 shall 該当する要件と設計・境界の節を同時に改訂する（記憶 revise-design-not-just-requirements）。
