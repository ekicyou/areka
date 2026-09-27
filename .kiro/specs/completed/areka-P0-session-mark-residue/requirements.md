# Requirements Document

> 本文の実測は **2026-09-27・本ブランチ**（main `55a2a1fd`＝棚卸⑱のコミット。ソースは `5a232d2f`〔`ghost-shell-balloon-switch` の完了〕から不変）のもの。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない。
> 要件 8 の裁定は brief の議題 2 件（OS の終了で SHIORI を待つ期限の値・LogSink へ倒れた起動の扱い）と、要件を書く途中で答えが要った 2 点に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。期限の値は要件ディスカッション（2026-09-27）で開発者が **3 秒** に確定した（要件 8.1）。

## Project Description (Input)

**誰の何が困っているか**: α の利用者（第三者）と、裁定 13（2026-09-27・起動中の印）の約束そのもの。裁定 13 で areka は「ゴーストを起こしたら記憶に印を書き、きれいに終わったときだけ消す。次の起動で印が残っていれば前回は落ちた＝既定ゴースト emo2 を `OnBoot` の Reference6＝`halt`・Reference7＝落ちたゴースト名で起こす」形になった。この「きれいに終わった」の判定と Windows のシャットダウン／ログオフの後始末に、3 つの穴が残っている。

**今の状態**: ⑴ SHIORI が固まったまま Windows を終了すると、`WM_ENDSESSION` の後始末が SHIORI を約 100 秒（既に待っている往復があれば約 160 秒・無限待ちの設定なら上限なし）待ち、Windows に打ち切られて印が残る（利用者は何も壊していないのに次の起動が `halt` になる）。⑵ 最初の起動が窓への結線に失敗して LogSink の起動へ倒れた後にきれいに終えると、印が消え、次の起動も同じゴーストで同じ失敗を繰り返して `halt` が付かない（裁定 13 に反する）。⑶ OS の終了の受け手がゴーストを降ろす処理の終わりを待つ間、送られてきたウィンドウメッセージを配らない。別スレッドから UI の窓への同期の送信が重なると止まりうるが、確かめていない。

**何を変えるか**: ⑴ OS のセッションの終了のときだけ SHIORI を待つ期限の合計に上限を設け、超えたら待たずに後始末を終えて印を残す（切替・メニューの終了・強制退避の期限は変えない）。⑵ 最初の起動が LogSink へ倒れたこと自体を印を残す理由にする。⑶ 重なる形を再現して止まるかを確かめ、止まるなら直し、止まらないならその理由を記録する。3 つとも決定論のテストで固定する。

> 起票: 2026-09-27 棚卸⑱（`/kiro-discovery` 再入）。roadmap の「登記だけの行」2 本（「OS のセッションの終了では SHIORI の待ちを短くする」・「起動がまるごと失敗すると起動中の印が消える」）の格上げと、完了 `areka-P0-ghost-shell-balloon-switch` の `signoff.md`「気付いたこと」8（引受先 0）をまとめた。段は **バグ**（潜在）。

## Introduction

### 誰が困っているか

- **SHIORI が固まったゴーストを動かしたまま PC を落とした利用者**: Windows に「このアプリがシャットダウンを妨げています」と出され、待つか強制終了するかを迫られる。どちらでも次の起動は emo2 が「前回落ちた」と言って起きる。利用者は何も壊していない。
- **前に動いたゴーストのファイルが外で壊れた（または同梱の emo2 が壊れた）利用者**: 窓の見えないゴーストが残り、閉じてもう一度起動すると同じ壊れたゴーストがまた起きる。既定ゴーストへ戻る道（裁定 13）が効かない。

α の第三者の一周（zip 展開 → 起動 → `.nar` の投げ込み → 切替 → 更新 → 終了 → 再起動）では 3 つとも踏まない。優先度「バグ → α」によりウェーブの先頭に置く。

### いま何が起きているか（2026-09-27 実測）

1. **OS の終了で SHIORI を待ちすぎる**。`WM_ENDSESSION`（wParam＝TRUE）を受けた後始末は `crates/areka/src/session_end.rs` の `on_os_session_end` が窓の手続きの中で同期に行い、`GhostSession::shutdown(CloseReason::System)` → `crates/areka-ghost/src/runtime.rs` の `GhostRuntime::shutdown`（kanade へ強制終了を送り、kanade・dispatcher・ticker・shiori・relay の順に join）→ kanade の強制終了（`crates/areka-kanade/src/schedule/mod.rs` の `force_quit`）が `OnClose`（Ref0＝`system`）を通知（NOTIFY）で送ってから SHIORI を降ろす。SHIORI 側の待ちは次の 3 段の和で**約 100 秒**（下の「並んで待つ」分を足すと約 160 秒）:
   - `OnClose` の NOTIFY の往復: `crates/shiori-host32-host/src/client.rs` の実効の期限（`effective_timeout`）＝既定 60 秒（`crates/shiori-host32-host/src/process_host.rs` の `REQUEST_TIMEOUT`。環境変数 `AREKA_SHIORI_REQUEST_TIMEOUT_MS` で変えられる）
   - UNLOAD の応答: `crates/shiori-host32-host/src/lifecycle.rs` の `UNLOAD_ACK_TIMEOUT`＝30 秒
   - 補助プロセスの終了の観測: 同 `EXIT_OBSERVE_TIMEOUT`＝10 秒

   さらに、kanade は SHIORI への要求を 1 件ずつ送って答えを待つ（`round_trip_request`）ので、OS の終了が届いた時点で GET などの往復が既に待っていれば、強制終了はその後ろに並び、最大でもう 60 秒（計約 160 秒）延びる。`AREKA_SHIORI_REQUEST_TIMEOUT_MS=0`（無限待ち）が設定されていれば上限は無い。ゴーストの実行系の join（`GhostRuntime::shutdown` の各段・`GhostSession::shutdown` の seriko）もすべて期限なしで待つ。期限を呼び手から渡す経路は無い（すべて定数か環境変数）。Windows の猶予（Vista 以降、`WM_ENDSESSION` の処理が 5 秒を超えると利用者に続けるか取りやめるかを選ばせる画面が出る＝`research.md` §5）を超えると打ち切られ、印が残る。
2. **起動が LogSink へ倒れると印が消える**。最初の起動は `crates/areka/src/ghost_session.rs` の `boot_ghost` が、窓への結線（`boot_wired`）に失敗すると LogSink の起動（`areka_ghost::boot_with_kanade_stop`）へ倒れる（完了 `emo2-boot` 以来の非致命の契約）。倒れた先が成功すれば `info!`（「LogSink フォールバックで起動しました」）、失敗すれば `warn!`／`error!` を残して続行する。どちらの場合も終了の出所には何も載らないので、その後 `WM_CLOSE`・smoke の自動終了・OS のセッションの終了などできれいに終えると、`crates/areka/src/main.rs` の `session_mark_verdict`（終了の出所 `ExitOrigin` の網羅の match）が「きれいな終わり」と判定し、`settle_session_mark` が印を消す。切替は倒れ込みを持たない `boot_ghost_strict` を通るので、この穴は踏まない。
3. **OS の終了の受け手が join の間メッセージを配らない（未確認）**。`on_os_session_end` の説明に「降ろす処理の join は UI スレッドを塞ぐ（送られてきたメッセージは配らない）」とある。完了 spec の実機確認（`signoff.md` の ⑪・所要 22 ms）は各窓へ 1 通ずつ返りを待って送ったので、別スレッドからの同期の送信が join と重なる形が起きておらず、止まるかどうかを確かめていない。静的な見立て（2026-09-27）: 本番コードの同期の送信は `crates/shiori-host32-ipc/src/lib.rs` の `SendMessageTimeoutW`（ゴーストの実行系の SHIORI の係のスレッドが持つ親窓〔`crates/shiori-host32-host/src/parent_window.rs`〕と 32bit の補助プロセスの間・`SMTO_ABORTIFHUNG` と期限つき）だけで、UI スレッドの窓への送りは `PostMessageW`（非同期）だけに見える。つまり止まらない見込みだが、再現で確かめていない。なお 32bit の補助プロセスは `WM_ENDSESSION` を扱わず、OS の終了の待ちの間に OS に終わらされうる。

### 正典（ukadoc）の位置づけ

- 正典 [`OnClose`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnClose:1) の Reference0「※SSPのみ　終了理由。ユーザーが終了した場合、userシャットダウンした場合、systemが返される。」＝OS の終了で `OnClose` を Ref0＝`system` で送ることは完了 spec が実装済みで、本仕様は変えない。
- 正典は、ベースウェアが SHIORI の応答をどれだけ待つか・OS の終了のときに待ちを縮めるかに**沈黙**している。本仕様の期限の上限は areka 裁量で、`doc/COMPAT_ARCHITECTURE.md` §8（沈黙ルール対応表）の既存の行「OS のシャットダウン・再起動・ログオフで終わるとき」（今日は期限を書いていない）に追記する。
- 正典 [`OnBoot`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnBoot:1) の Reference6「前回の処理中で落ちた時にhalt。」・Reference7「前回の処理中で落ちたゴースト名 。」は落ち方を問わない語（完了 spec 裁定 13 の根拠）。窓への結線に失敗して窓の見えないまま動いたゴーストを「落ちた」に含めるのは、この語の読みの延長で、§8 の「きれいに終わらなかった次の起動（起動中の印）」の行に追記する。

### 何を変えるか

1. OS のセッションの終了のときだけ、SHIORI を待つ期限の合計を Windows の猶予より十分短い上限に収める。上限を超えたら SHIORI を待たずに後始末を終え、印を残す（次の起動は emo2＋`halt`）。
2. 最初の起動が LogSink へ倒れたら、その後どう終わっても印を残す（倒れた先の成否を問わない）。
3. OS の終了の受け手の join 中に別スレッドから同期の送信が重なる形を再現し、止まるなら直し、止まらないなら理由を記録する。
4. 1〜3 を決定論のテストで固定する。

## Boundary Context

- **In scope**:
  - OS のセッションの終了のときだけ効く、SHIORI を待つ期限の上限と、それを後始末の出発点から SHIORI の通信まで運ぶこと
  - 上限を超えたときの後始末の終え方と印の扱い
  - 最初の起動が LogSink へ倒れたときの印の扱い
  - OS の終了の受け手の join と同期の送信の重なりの再現と、止まるならその是正
  - それぞれの決定論テスト・記録（ログ）・`doc/COMPAT_ARCHITECTURE.md` §8 への追記
- **Out of scope**:
  - 切替・メニューの終了・強制退避（Ctrl+Shift+左ダブルクリック）・smoke の自動終了・OS の閉鎖要求の期限の変更
  - 起動中の印の方式そのもの（裁定 13 は変えない。印の鍵・書く時点・読む規則・argv の上書き）
  - 登記だけの行「壊れたゴーストを表示し続ける」（SSP 忠実の切替失敗の形）
  - emo2 の `halt` の台詞（開発者が辞書に足す・`alpha-release-signoff` の前提）
  - main スレッドの panic で後始末を通らない件（方針「panic は致命の場合だけ」と整合・棚卸⑰で捨てた）
  - 環境変数 `AREKA_SHIORI_REQUEST_TIMEOUT_MS` の意味の変更（OS の終了以外では今日どおり効く）
- **触らないファイル（並走の都合）**: `crates/areka/src/emo2_boot/ghost_switch.rs`（切替の経路）・`crates/areka/src/emo2_boot/change_cue.rs`・メニュー（`crates/areka/src/menu/`）・`crates/areka-parsers/`＝`ghost-change-name-resolution` と同じウェーブで並走する。kanade の `crates/areka-kanade/src/schedule/events.rs`（許可表）と `msg.rs` の新しい変種＝`shell-balloon-switch`・`ghost-install`・`network-update` が動かす。
- **Adjacent expectations**:
  - 完了 `ghost-shell-balloon-switch` の起動中の印（`session_mark_verdict`・`settle_session_mark`・`on_os_session_end`）・完了 `shiori-fault-notice`（`finish_after_run`・`Fault` の終了コード 1）・完了 `ghost-restart-unit`（`GhostSession::shutdown`）・完了 `host32-window-thread-pump`（親窓の専用スレッド）の振る舞いに依る。本仕様はそれらの完了 spec の文書を書き換えない。
  - 後続 `shell-balloon-switch`・`ghost-install`・`network-update` は `ghost_session.rs`・`main.rs` を触るので本仕様の後に建つ。後続が終了や再起動の経路を足すときも、印を消すか残すかの判定は `session_mark_verdict` の 1 か所を通す（完了 spec の申し送りを継ぐ）。
  - `alpha-release-signoff` の既知の制限の候補「SHIORI が固まったまま Windows を終了した場合」は本仕様の完了で消える。

## Requirements

### Requirement 1: OS のセッションの終了のときだけ、SHIORI を待つ期限の合計に上限がある

**Objective:** As a SHIORI が固まったゴーストを動かしたまま PC を落とす利用者, I want areka が Windows の猶予のうちに後始末を終えること, so that 「このアプリがシャットダウンを妨げています」が出ず、Windows に打ち切られない

#### Acceptance Criteria

1. When OS のセッションの終了（`WM_ENDSESSION` の wParam＝TRUE）の後始末が今のゴーストを降ろす, the areka shall その時点で既に待っている SHIORI への往復（GET・NOTIFY）の残り・`OnClose`（Ref0＝`system`）の通知の往復・SHIORI を降ろす要求（UNLOAD）の応答・32bit の補助プロセスの終了の観測で SHIORI を待つ時間の**合計**を、後始末に入った時点から数えて上限 T 以下に収める。
2. The areka shall 上限 T を、Windows がセッションの終了の後始末に与える猶予（Vista 以降は 5 秒。超えると利用者に選ばせる画面が出る＝`research.md` §5）より十分短い固定の値とし、T の後に続く後始末（要件 1.6）を含めても 5 秒の内に収まるようにする。**T＝3 秒**（2026-09-27 開発者の確定＝要件 8.1）。
3. When OS のセッションの終了の後始末で SHIORI を待っている, the areka shall どの段で待っていても（後始末に入る前から待っていた往復を含む）、後始末に入った時点から T が過ぎた時点でその待ちを打ち切る。今日の既定の期限が T より早く切れる段は今日どおりに切れる（打ち切りの手＝外から待ちを解くか、段ごとに期限を運ぶか、は設計で決める＝`research.md` §4・§8 D2）。
4. Where 環境変数 `AREKA_SHIORI_REQUEST_TIMEOUT_MS` が設定されている, the areka shall OS のセッションの終了の後始末でも上限 T を優先する（環境変数が T より長い値・無限を指しても、待ちは T を超えない。短い値ならその値で今日どおり切れる）。
5. When SHIORI が上限 T より前に応答し補助プロセスが終わる, the areka shall 今日どおり（完了 `ghost-shell-balloon-switch` 要件 12.9）後始末を終える（`OnClose` の通知 1 件・SHIORI を降ろす・記憶の書き出し・きれいな終わりの判定で印を消す）。
6. The areka shall 上限 T の対象を SHIORI の待ちに限り、ゴーストの実行系の他の段（kanade・dispatcher・ticker・relay・位置や最後の選択の記憶の書き出し）の後始末は上限 T の後でも省かない。

### Requirement 2: 上限を超えたら、SHIORI を待たずに後始末を終え、印を残す

**Objective:** As a 利用者と裁定 13, I want 固まった SHIORI を待ち切れずに終えた回は「きれいに終わらなかった」として扱われること, so that 次の起動が emo2 の `halt` で始まり、壊れたゴーストの名前が伝わる

#### Acceptance Criteria

1. If OS のセッションの終了の後始末で SHIORI の待ちが上限 T に達する, then the areka shall その時点で SHIORI の応答を待つのをやめ、残りの後始末（ゴーストの実行系の他の段の停止・記憶の書き出し・印の判定・所要の記録）を続けて、窓の手続きから戻る。
2. If 上限 T に達した, then the areka shall `warn!` 以上で、どの段で達したか（少なくとも「後始末に入る前から待っていた往復」「`OnClose` の通知」「SHIORI を降ろす〔UNLOAD の応答と終了の観測〕」の 3 つが見分けられる語。これより細かく分けるかは設計で決める＝`research.md` §8 D4）と上限 T の値と経過時間を 1 件残す（ログの無い失敗の経路を作らない）。
3. If 上限 T に達した, then the areka shall きれいな終わりの判定で印を**残し**、`info!(event="session_mark_kept", reason)` の理由に上限切れと分かる語を載せる（次の起動は既定ゴースト＋`OnBoot` の Ref6＝`halt`・Ref7＝そのゴーストの名前）。
4. If 上限 T に達した, then the areka shall 32bit の補助プロセスを残さない（応答しないまま終わらせる。終わらせる口の形は設計で決める。終わらせるのに失敗したら `error!` で残す）。
5. If 上限 T に達した, then the areka shall 告知（メッセージボックス）を出さない（OS の終了を塞がない・完了 spec 要件 12.11 を継ぐ）。終了コードは今日どおり最初の終了の出所で決める。
6. The areka shall 上限 T に達しなかったときは印の判定を今日どおりにする（本要件の変更で、SHIORI が期限内に応答した OS のセッションの終了の印が残ることはない）。

### Requirement 3: 切替・メニューの終了・強制退避の期限は 1 つも変わらない

**Objective:** As a 開発者, I want 期限を縮めるのが OS のセッションの終了に限られること, so that 遅い SHIORI を持つゴーストが切替や普段の終了で途中で切られない

#### Acceptance Criteria

1. The areka shall ゴーストの切替・メニューの「終了」・強制退避（Ctrl+Shift+左ダブルクリック）・OS の閉鎖要求（`WM_CLOSE`）・smoke の自動終了・kanade の終了系列の完了（別れの台詞のあと・中断のあと）で SHIORI を待つ期限と、別れの台詞を待つ期限（`KanadeConfig` の `close_talk_deadline_ms`＝30 秒）を、今日の値のまま（`OnClose` などの往復＝既定 60 秒または `AREKA_SHIORI_REQUEST_TIMEOUT_MS`・UNLOAD の応答 30 秒・終了の観測 10 秒）**変えない**。
2. The areka shall 最初の起動と切替先の起動の LOAD の応答の期限（`LOAD_ACK_TIMEOUT`＝30 秒）と、起動後の GET／NOTIFY の往復の期限を変えない。
3. The areka shall 上限 T の経路が無い呼び手（OS のセッションの終了以外のすべて）では、期限の決め方の結果が今日と 1 ミリ秒も違わないようにする。
4. The areka shall OS のセッションの終了以外の経路で SHIORI が今日の期限に達したときの印の判定を変えない（本仕様は上限 T に達したときの印だけを決める＝要件 8.4）。
5. The areka shall 本仕様の変更で、本番コードが読む環境変数と依存クレートを足さない（既に依存している `windows` クレートの feature を足すことは、依存クレートの追加に数えない）。

### Requirement 4: 最初の起動が LogSink へ倒れたら、その後どう終わっても印が残る

**Objective:** As a 前に動いたゴーストのファイルが外で壊れた利用者, I want 窓の見えないまま動いた回の次の起動が既定ゴーストで始まること, so that 同じ壊れたゴーストが起動のたびに起きず、emo2 が何が落ちたかを伝えられる

#### Acceptance Criteria

1. When argv でゴーストを指定しない最初の起動が窓への結線に失敗して LogSink の起動へ倒れる, the areka shall そのプロセスがどの出所で終わっても（メニューの「終了」・OS の閉鎖要求・強制退避・smoke の自動終了・kanade の終了系列の完了・OS のセッションの終了）、きれいな終わりの判定で印を**残す**。
2. The areka shall 4.1 を、倒れた先の LogSink の起動が**成功した**場合（SHIORI は動くが窓は無い）と**失敗した**場合（起動の起点が見つからない `warn!` の場合・それ以外の `error!` の場合）の両方で同じにする。
3. When 4.1 で印を残す, the areka shall `info!(event="session_mark_kept", reason)` の理由に LogSink へ倒れたと分かる語を載せる。倒れた時点では、今日の記録（LogSink の起動の成否）に加えて「このプロセスは終わり方によらず印を残す」と帰結を書いた `warn!` を 1 件残す。
4. When 4.1 で印が残った次の起動が行われる, the areka shall 完了 `ghost-shell-balloon-switch` 要件 12.4 どおり、最後に使ったゴーストの記憶を読まずに既定ゴースト（配布物では emo2）を起こし、`OnBoot` に Ref6＝`halt`・Ref7＝倒れたゴーストの名前（起動前に印へ書いた名前）を載せる。
5. If 倒れたゴーストが既定ゴースト自身である, then the areka shall それでも印を残す（次の起動も既定ゴースト＋Ref6/7 になる。既定ゴーストが壊れ続けていれば毎回同じ知らせになり、裁定 13 の「既定ゴースト自身が落ちたときも印が残る」と同じ）。
6. The areka shall argv でゴーストを指定して始まったプロセスでは、LogSink へ倒れても印を読まず・書かず・消さない（完了 spec 要件 12.5 の argv の上書きが先に効く）。
7. The areka shall LogSink へ倒れたことで、終了コード・告知（メッセージボックス）・LogSink の起動そのもの（非致命の契約・骨格の起動を続けること）を変えない。
8. The areka shall 窓への結線が成功した最初の起動と、切替（倒れ込みを持たない経路）の印の扱いを変えない。

### Requirement 5: OS の終了の受け手の join と同期の送信の重なりを確かめる

**Objective:** As a 開発者, I want OS の終了の後始末がゴーストを降ろすのを待つ間に、別スレッドから UI の窓への同期の送信が重なっても止まらないと分かっていること, so that シャットダウンのたびに固まる形が潜んでいない

#### Acceptance Criteria

1. The 本仕様 shall 実装の前に、OS のセッションの終了の後始末がゴーストを降ろす処理の終わりを待つ間に、UI スレッドの窓へ別スレッドから同期の送信をしうる箇所（ゴーストの実行系のスレッド・32bit の補助プロセスとの通信・wintf の描画や VSync のスレッドなど）を洗い出し、各箇所が待ちと重なりうるかを記録する（2026-09-27 に `research.md` §6 で実施済み: **areka の中で重なる箇所は 0 件**。areka の外のコードが作りうる唯一の輪＝32bit の補助プロセスの中の SHIORI・SAORI が `OnClose` の処理中などに areka の窓へ同期で送る形は、今日は SHIORI の往復の期限でしか切れず、本仕様の上限 T で切れるようになる）。
2. The 本仕様 shall 5.1 で重なりうるとした形を、決定論の再現のテストで起こす（待ちの最中に別スレッドから UI の窓へ同期の送信をし、後始末が上限の時間内に戻るかと、送信が返るかを観測する）。重なりうる箇所が 0 件なら、そう明示して、代表の形（待ちの最中に別スレッドから UI の窓へ同期の送信を 1 通）を再現のテストで起こす。
3. If 5.2 の再現で後始末または送信が止まる（上限の時間内に戻らない）, then the areka shall 止まらない形に直す（待つ間に送られてきたメッセージを配るか、同期の送信を非同期に替えるかは設計で決める）。直した後、同じ再現のテストが緑になる。
4. If 5.2 の再現で止まらない, then the 本仕様 shall コードを変えず、止まらない理由（誰がどのスレッドの窓へ送るか・なぜ待ちと循環しないか）を `on_os_session_end` の説明と本仕様の記録に残し、再現のテストを常設して理由が崩れたら赤になるようにする。
5. The areka shall 5.3 で直した場合も、OS のセッションの終了の後始末が 1 回だけ走ること（完了 spec 要件 12.10）と、`OnClose` が Ref0＝`system` の通知で 1 件だけ送られることを変えない。

### Requirement 6: 印の判定は 1 か所を通り、記録と文書が揃う

**Objective:** As a 後続の spec の開発者, I want 印を消すか残すかの理由が 1 か所の判定に集まり、ログと文書から追えること, so that 後から終了や起動の経路を足しても判定が散らばらない

#### Acceptance Criteria

1. The areka shall 要件 2.3（上限切れ）と要件 4.1（LogSink へ倒れた）の印を残す判断を、`fn main` の後始末と OS のセッションの終了が共有する既存のきれいな終わりの判定（`session_mark_verdict`）で行い、別の場所で印を書き戻したり消すのを止めたりしない。
2. The areka shall 印を残す理由の語を、既存の理由（SHIORI の失敗・切替の致命・メッセージループの失敗・降ろす処理の失敗・終了の出所が無い）と見分けられる別の語にする。
3. The areka shall 上限 T に達した・LogSink へ倒れた・再現の対象の形が起きた、のどれも、ログの無い経路を作らない（上限切れは `warn!` 以上・印を残した理由は `info!`・後始末の所要は今日の `os_session_end_done` の `ms`）。
4. The 本仕様 shall `doc/COMPAT_ARCHITECTURE.md` §8 に、⑴ 既存の「OS のシャットダウン・再起動・ログオフで終わるとき」の行へ、OS のセッションの終了のときだけ SHIORI を待つ期限の合計を上限 T に収め、超えたら印を残す（areka 裁量・正典は待ちの長さに沈黙）ことを追記し、⑵ 既存の「きれいに終わらなかった次の起動（起動中の印）」の行に「最初の起動が LogSink へ倒れた回もきれいな終わりに含めない」を 1〜2 行で追記する。完了 spec の文書は書き換えない。

### Requirement 7: 決定論テストと実機確認

**Objective:** As a 開発者, I want 3 つの振る舞いがテストで固定されていること, so that 後続の変更で黙って崩れない

#### Acceptance Criteria

1. The 本仕様 shall 要件 1〜2 を、偽の時計または偽の SHIORI（x64 の偽境界・実時間を待たない）で固定する: ⑴ 上限 T の中で SHIORI が応答すれば今日どおり印が消える ⑵ 後始末に入った時点で既に待っている往復の段・`OnClose` の通知の段・UNLOAD の応答の段・終了の観測の段のそれぞれで固まったとき、合計が上限 T を超えずに戻り、`warn!` が 1 件出て、印が残る ⑶ 環境変数が T より長い・無限・T より短い、の 3 通りで、長い・無限なら合計が T を超えず、短いならその値で今日どおり切れる⑷ 上限 T を渡さない呼び手の期限が今日と同じ。
2. The 本仕様 shall 要件 3 の「変わらない」を、切替・メニューの終了・強制退避の経路で SHIORI を待つ期限が今日の値のままであることを判定するテストで固定する（期限の決め方が上限を受け取らない経路で今日の定数と等しいことを判定させ、値を印字するだけにしない）。
3. The 本仕様 shall 要件 4 を、きれいな終わりの判定の表（`session_mark_verdict_table`）に行を足して固定する: LogSink へ倒れた × 終了の出所（`KanadeStopped` の各原因・`Escape`・`Smoke`・`OsClose`・`SessionEnd`）× メッセージループと降ろす処理の成否で印が残り、argv では触らない。あわせて、倒れた先が成功した場合と失敗した場合の両方で、その後きれいに終えても印が残り、次の起動が既定ゴースト＋Ref6＝`halt`・Ref7＝倒れたゴーストの名前になることを 1 周で固定する。
4. The 本仕様 shall 上限切れの印の行（`SessionEnd` × 上限切れ）も同じ表に足す。
5. The 本仕様 shall 要件 5 の再現のテストを常設する（止まる形なら是正の後に緑、止まらない形なら理由が崩れたら赤）。
6. The 本仕様 shall 既存のテスト（完了 spec の `session_end_tests.rs`・`main_session_mark_tests.rs`・`shiori-host32-host` の期限の定数の判定）を置き換え無しに消さない。振る舞いが変わる行は新しい振る舞いを固定する形へ書き換える。
7. When 実機で確認する, the 開発者 shall ⑴ 定常のゴーストの areka のトップレベル窓すべてへ `WM_QUERYENDSESSION` → `WM_ENDSESSION`（wParam＝TRUE）を送り（完了 spec の ⑧ と同じ形・エージェントが自分で起こしたプロセスに限る）、後始末が今日どおり 1 回走って印が消え所要が上限 T より十分短いこと、⑵ 最初の起動を LogSink へ倒した回（窓への結線を不成立にする手が取れれば）をきれいに終え、次の起動で emo2 が Ref6＝`halt`・Ref7 付きで起きること、を有界の自動終了つきで見て `signoff.md` に記録する（`RUST_LOG` は印・OS のセッションの終了・上限切れの事象の水準まで開ける）。固まった SHIORI の実機の再現は、固まる SHIORI を用意できる場合に限る（用意できなければ決定論テストで足り、その旨を記録する）。

### Requirement 8: 裁定（暫定・要件ディスカッションで確定）

**Objective:** As a 開発者, I want 要件を書く途中で答えが要った点が、推奨案と未定の項目に分けて並んでいること, so that 要件ディスカッションで覆すか決めるだけで済む

#### Acceptance Criteria

1. The 本仕様 shall **議題 1（確定・2026-09-27 開発者）: 上限 T＝3 秒**とする。条件は「Windows の猶予（Vista 以降は 5 秒＝`research.md` §5）より、T の後の後始末の分も含めて十分短い」「正典 `OnClose` の Ref0＝`system` の通知を送る余地を残す」。理由: SHIORI が健全なときの後始末は数十 ms（完了 spec の実機で 22 ms）なので 5 秒までの余白は 2 秒で足り、遅いだけの SHIORI が降りるときの保存（ゴーストの変数など）にできるだけ長く時間を渡す方が利用者の記憶を守れる。**採らなかった値**: 1 秒（重い辞書の保存が切られうる）・2 秒（余白 3 秒は健全な後始末には過剰）。値は要件 1.2 と `doc/COMPAT_ARCHITECTURE.md` §8 の行に書く。本番では定数、テストでは引数で渡す（要件 3.5 により環境変数は足さない）。
2. The 本仕様 shall **議題 2（暫定の確定）: LogSink へ倒れた起動の扱い**を「LogSink へ倒れたこと自体を印を残す理由にする」とする（要件 4）。**採らなかった案**: 「LogSink の起動の失敗を終了の出所に載せる」——倒れた先の起動が成功した場合（SHIORI は動くが窓が無い）を塞げない（brief・roadmap 棚卸⑱の再測定）。
3. The 本仕様 shall **暫定の確定 3: LogSink へ倒れたプロセスでその後に切替が定常まで届いた場合**、倒れた理由は切替先へ引き継がない（印は完了 spec 要件 12.6・12.7 どおり切替先の名前へ書き換わり、切替先がきれいに終われば印は消える）。理由: 印は「いま動いているゴースト」を表し（裁定 13）、定常に入った切替先は窓への結線を通った健全なゴーストであるため。引き継ぐと、健全な切替先の名前が Ref7 に載る。なお今日のコードではこの場合は起きない（切替の受信端 `ChangeRx` は窓への結線が成立した後にだけ据えられ、右クリックメニューは `boot_wired` の中でだけ結線される＝`research.md` §3.1）。将来そうなったときの約束として置く。
4. The 本仕様 shall **暫定の確定 4: OS のセッションの終了以外の経路で SHIORI が今日の期限に達したときの印の扱い**は本仕様で変えない（要件 3.4）。brief の Out（期限の変更と印の方式そのもの）に従う。変えるべきと判断されたら別に起票する。
