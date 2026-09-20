# 設計検証レポート: areka-P0-balloon-break

> 検証日 2026-09-20・ブランチ `claude/kiro-balloon-double-click-stop-454210`。対話なしで実施した。
> 対象は `design.md`（636 行）・`requirements.md`（220 行）・`research.md`（465 行）と steering。
> 設計が拠り所にしている既存コードの事実は、設計の文ではなく**実ファイルを読んで**確かめた。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。
> 「0 件」と書いた検索は、先に同じ検索語が既知の場所に当たることを確かめてある（各項に較正を併記）。

## 1. 総評

設計は要件 70 項目（1.1〜9.9）をすべて対応表に載せており（機械で突き合わせて欠け 0・余り 0）、拠り所にしている既存コードの事実（設計の「Existing Architecture Analysis」の 10 点）はいずれも実ファイルと一致した。スレッドをまたぐ順序の議論（旗を `Input` の段で、表示の合図を `Update` の段で取り出すので、旗の線に「トークが始まった」も流す）は正しく、`research.md` §11 の覚え書きを採らなかった判断は妥当である。
見つかった問題は 2 件で、どちらも設計の骨格を変えずに直せる（1 件は運行側の 1 行、1 件は上書きする完了済み要件の書き漏れ）。判定は **GO**。

## 2. 重要な指摘（2 件・上限 3 件）

### 指摘 1: 中断を受理してから完了通知が届くまでの間に、`OnChoiceTimeout` が発火しうる（要件 2.6 の破れ）

- **懸念**: 設計は「止めた後の `TalkDone{Interrupted}` で既存の `clear_choice_ledger(state, "steady_talk_done")` が帳簿を消し、`fn fire_choice_timeout_if_due` は二度と発火しない。足す仕組みは 0」と書いている。実際には、帳簿が消えるのは完了通知が届いた**後**である。`fn on_user_break` が `Action::CancelChoice` を返してから `TalkDone` が戻るまでの間、選択の帳簿は `Waiting` のまま残る。`fn fire_choice_timeout_if_due`（`crates/areka-kanade/src/schedule/steady.rs`）が見るのは「帳簿が `Waiting`・期限到達・帳簿の対象が現行トーク」の 3 つだけで、「中断を出した後か」は見ない。この間に期限を過ぎた `Tick` が届けば `OnChoiceTimeout` が SHIORI へ出る。応答に台本があれば、黙らせた直後に別のトークが始まる。
- **影響**: 要件 2.6（選択肢の時間切れの扱いを行わない）と要件 3.9（中断を理由に SHIORI へ送るものは 0 件）が、到達できる入力の並び（`UserBreak` → 期限後の `Tick` → `TalkDone`）で破れる。実機で起きる窓は数ミリ秒と狭いが、運行の状態機械は純関数なので、この並びはテストで決定論的に再現できる。設計の分岐 ⑶ のテスト（受理 → `TalkDone` → 期限後の `Tick`）はこの並びを通らないので、緑のまま見逃す。
- **提案**: `fn on_user_break` が受理した時点で `clear_choice_ledger(&mut state, "user_break")` を呼ぶ（1 行）。利用者は選ばずに終わらせると決めたのだから、帳簿はその時点で役目を終えている。後から届く `TalkDone` の側の掃除は空振りになるだけで害は無い。分岐 ⑶ のテストに「受理 → 期限後の `Tick` → `OnChoiceTimeout` が出ない」の並びを足す。設計の「足す仕組みは 0」は「掃除点を 1 つ足す」へ直す。
- **対応する要件**: 2.6・3.9・7.1 ⑶
- **設計の該当箇所**: 「kanade user_break」の節の「選択待ちの最中（要件 2.6・2.7・研究項目 R-4）」、Testing Strategy の分岐 ⑶

### 指摘 2: 上書きされる完了済み要件がもう 1 つある（`areka-P0-popup-menu-minimal` の要件 5.4）

- **懸念**: 要件と設計は、終了の規則の改訂が上書きする相手として完了 spec `areka-P0-kanade` の要件 4.5 だけを挙げている。実際にはもう 1 つ、完了 spec `areka-P0-popup-menu-minimal` の要件 5.4「終了の握手が SHIORI に拒まれた（握手が `Steady` へ戻る既存の経路）なら、メニューからの終了でも同じく終了せず、既存の記録をそのまま出す」が同じ規則に乗っている。その spec の tasks.md の「非回帰 6 項目」の 3 行目は、本設計が書き換える 3 本のテスト（`close.rs` の 2 本＋`close_refused_resumes_pump_then_terminates_via_resumed_talk`）を名指しで「変えないと決めたもの」の根拠にしている。
- **影響**: 本設計の後、終了の握手が定常へ戻る経路は **0 本**になる（`fn on_close_talk_wait` の `Input::TalkDone` の腕が唯一の戻り口で、それが終了へ進む腕に変わる）。右クリックメニューの「終了」を選んだ利用者から見ると、「ゴーストに引き止められる」ことが無くなる。これは 2026-09-20 の裁定（議題 2）の帰結そのものだが、前日（2026-09-19）に完了した spec の要件を上書きすることが、どの文書にも書かれていない。プロジェクトの規律（裁定で要件を改訂したら設計・境界の節まで追随する）に照らすと書き漏れである。
- **提案**: 設計の「要件本文との差」に 3 つ目として載せ、設計ディスカッションで要件の Boundary Context（上書きする完了済み要件の列挙）へ `areka-P0-popup-menu-minimal` 要件 5.4 を足す。互換対応表の 3 行目（別れの台詞は必ず終了で終わる）にも「メニューの『終了』も引き止められない」を 1 句足す。コードの作業量は変わらない。
- **対応する要件**: 3.6・9.6・9.9（Boundary Context の In scope「終了の予約」）
- **設計の該当箇所**: Overview の Impact 4、「Modified Files」、「close の規則改訂」、「要件本文との差」

## 3. 設計の強み

1. **順序の議論が実コードに根を持っている。** 旗の線は `Input` の段で `dispatch_pointer_events` の前に、表示の合図の線は `Update` の段（`fn run_balloon_visibility_phase` の中の `fn drain_lifecycle`）で取り出される。同じ巡に `Enter` と `TalkStarted` が届くと逆順に処理されるという問題を設計が自分で見つけ、7 本目の受け口に `BalloonLifecycleSink` と同じ「複製で状態を初期化し、複製後の最初の指示で 1 回だけ送る」手口を持たせて 1 本の線へまとめた。dispatcher の `fn on_start` は `fn close_active_if_any` で古いトークのスレッドを合流させてから新しいトークを起こすので、トークをまたぐ順序も保たれる（実ファイルで確認）。
2. **変更の面が小さく、触らないものを数で言い切っている。** `steady.rs`（935 行）・dispatcher の製品コード・配線層・文字層・表示層はいずれも変更 0 行。停止は既存の `Action::CancelChoice` → `fn on_cancel_choice` → `fn on_close` をそのまま使い、第 2 の停止経路を作らない（要件 3.2）。終わり方の種類は 3 値のまま（要件 3.3）。

## 4. 実コードに当てて確かめた事実

| # | 設計の主張 | 結果 | 確かめた定義 |
|---|---|---|---|
| 1 | 旗の線を `Input` の段・`dispatch_pointer_events` の前で取り出せる。循環しない | 正しい | `dispatch_pointer_events` は `drain_task_pool_commands` の後に登録されているだけ（`crates/wintf/src/ecs/world/mod.rs` の既定システム登録）。areka 側の既存の登録は 4 本とも「後」（`readme.rs`・`choice_drain.rs`・`balloon.rs`・`menu/mod.rs`）で、「前」は 0 本 |
| 2 | 表示の合図は `Update` の段で、表示指令の適用の後に取り出される | 正しい | `fn emo2_frame_system`（`crates/areka/src/emo2_boot/frame.rs`）は `run_drain_phase` の後に `run_balloon_visibility_phase` を呼ぶ |
| 3 | 1 本の線にすれば「トーク N+1 の `Enter` がトーク N+1 の `TalkStarted` に消されない」 | 正しい | 同じスレッドが同じ線へ送る。先頭の指示は `ClearAll`（ここで `TalkStarted`）、`enter` の運搬はその後の指示 |
| 4 | 「トーク N の旗は、トーク N+1 の最中の押下を判定する前に解ける」 | 正しい（条件つき） | トーク N+1 の文字が見えているなら、その前に `ClearAll` が配られており、`TalkStarted` は線に入っている。取り出しは押下の判定より前。kanade が N+1 を起こしてから最初の指示が配られるまで（再生の刻みの既定 50 ミリ秒＝`crates/areka-ghost/src/ticker.rs` の `base_interval`）は旗が残るが、その間に見えているのはトーク N の居残りバルーンなので要件 5.4 の明示した差の内側 |
| 5 | 内容を持つ台本の最初の指示は必ず `ClearAll`（R-2） | 正しい | `fn compile` の末尾（`crates/areka-sakura/src/compile.rs`）。例外は起動記録だけのトーク（§5 の 3） |
| 6 | 掛け金が次のトークの最初の文字を飲み込まない（R-2） | 正しい | 文字の指示が文字層へ送られる時点で、`TalkStarted` は既に表示の合図の線に入っている（`BalloonLifecycleSink` の `fn emit` は最初の指示で送る）。可視性の相はその巡の文字の適用より後に線を全件取り出す |
| 7 | `fn drive` は SHIORI の往復を同期で回す。メッセージの切れ目で起動の挨拶は `Steady{Some}`、選択の帳簿は `Waiting` か無し（R-6・R-4） | 正しい | `fn drive`・`fn execute_actions`（`crates/areka-kanade/src/actor.rs`）、`fn to_baseware_version`（`schedule/boot.rs`）は `[StartTalk, basewareversion]` を 1 バッチで返し、応答はその場で再投入される |
| 8 | 差し替えで止めたトークの完了通知は kanade へ届かない（R-8 a） | 正しい | `fn close_active_if_any` が枠を先に空け、`fn on_done` が不一致として捨てる（`crates/areka-ghost/src/dispatcher.rs`） |
| 9 | 古い `user_break_talk` が後のトークを終了させない（R-8 b） | 正しい | `State::next_talk_id` は単調に増え再利用しない。設計は完了通知の `talk_id` と突き合わせ、現行トークの完了で必ず空にする |
| 10 | 自然に終わるのと利用者の中断が同時（R-8 c） | 問題なし | 自然終了が先なら `TalkDone{Ended}` で帳簿が空になり、遅れた `CancelChoice` は `fn on_cancel_choice` が `cancel_choice_stale` で捨てる。`Quit` で終わったなら元から終了する |
| 11 | `TalkDone {` の出現は 45 行・17 ファイル、製品コードの構築点は 2 つ（R-8 d） | 正しい | 数え直して 45 行・17 ファイル。製品の構築は `fn send_done`・`fn send_interrupted`（`crates/areka-sakura/src/drive.rs`）の 2 つ。代わりに `SakuraMsg::Close` と `CancelChoice` に欄を足す形は 18 行・9 ファイル＋37 行・14 ファイルで、より大きい上に要件 3.3 の字義（「中断で終わった」として返す）を破る。欄 1 つが最小 |
| 12 | 旧規則を固定するテストは 3 本（R-9） | 正しい。4 本目は **0 本** | `close.rs` の 2 本＋`close_refused_resumes_pump_then_terminates_via_resumed_talk`。`close_test_boot_greeting_tests.rs` の別れの台詞はすべて終了で終わる設定で、影響を受けない。較正: `close_refused` は `close.rs` に 1 件当たる |
| 13 | 旧規則を述べる文書（`doc/`・`.kiro/steering/`） | **0 件** | `doc/COMPAT_ARCHITECTURE.md` の `OnClose` の出現は 0 件。`doc/emo2-conformance-scope.md` の 1 件は「終了挨拶＋`\-`」で新規則と矛盾しない。較正: 同じ検索語が `crates/` では `close.rs` ほかに当たる |
| 14 | `enter`／`leave` を特別扱いする既存コード | **0 件** | 製品コードに `"enter"`・`"leave"` の出現は 0 件（較正: `nouserbreak` は `crates/areka-kanade/src/status.rs` に当たる）。汎用の運搬に載り、`ReadmeCueSink` と同じ「名前＋第 1 引数」の選別で拾える |
| 15 | モジュール名に Rust の予約語 `break` を使っていないか | 使っていない | 新設は `user_break.rs`・`user_break_cue.rs`・`schedule/user_break.rs` |

### 要件 2.8（隠れたのに再生が続く、を作らない）の全分岐

| 場面 | UI | kanade | 対が破れるか |
|---|---|---|---|
| 無効化の区間 | 隠さない・送らない | 何も届かない | 破れない |
| 選択を確定した続きの 2 打目 | 隠さない・送らない | 何も届かない | 破れない |
| バルーンが出ていない／結線前／`Emo2Wiring` 不在の早期復帰 | 隠さない・送らない | 何も届かない | 破れない |
| 再生中で受理 | 隠す | `fn current_talk_id` が `Some` の 3 場面を一律に止める。dispatcher は kanade と同じ線の順で枠を見るので不一致にならない | 破れない |
| 再生中でない | 隠す | 止める対象なし | 破れない（止めるものが無い） |
| kanade への送出に失敗 | 隠す | 届かない | **破れる**。`error!` あり（要件 1.7・2.8 が明示する唯一の破れ） |
| 表示側への送出に失敗 | 隠れない | 止める | 逆向き（止まったが残る）。`error!` あり |
| 停止の指示の送出に失敗 | 隠す | 送れない | 破れる。既存の `talk_command_send_failed` が `error!` |

記録の無い破れは **0 本**。

### 1 ファイル 1,000 行（実測 `wc -l`）

| ファイル | 設計の記載 | 実測 | 増分の見立て後 |
|---|---:|---:|---:|
| `crates/areka/src/input_events/balloon.rs` | 917 | 917 | 927 以内 |
| `crates/areka-kanade/src/schedule/steady.rs` | 935 | 935 | 935（触らない） |
| `crates/areka/src/emo2_boot/balloon_visibility.rs` | 770 | 770 | 840 前後 |
| `crates/areka-kanade/src/schedule/mod.rs` | 713 | 713 | 763 前後 |
| `crates/areka-kanade/src/schedule/close.rs` | 609 | 609 | 609 前後 |
| `crates/areka-kanade/src/schedule/schedule_tests.rs` | 記載なし | 935 | 最大 944（§5 の 1） |
| `crates/areka-kanade/src/schedule/steady_flow_tests.rs` | 記載なし | 924 | 最大 931（§5 の 1） |

設計の 5 つの数字は実測と一致。1,000 行を越えるファイルは **0 本**。

## 5. 開発者の判断を待たずに直せる細かい点

1. **`State` に欄を足す追随が計画に無い。** `user_break_talk` を足すと、`State { … }` を直に組み立てている箇所が追随する（`choice_prev_talk:` を手がかりに数えて 22 行・8 ファイル。うち型定義 1・`fn initial` 1・残りはテスト）。`schedule_tests.rs`（935 行）は `TalkDone {` 8 か所＋`State` 1 か所で最大 +9、`steady_flow_tests.rs`（924 行）は 4＋3 で最大 +7。どちらも 1,000 行に収まるが、File Structure Plan と行数の見立てに載せる。コンパイラが漏れを止める点は `TalkDone` と同じ。
2. **旧規則や旧テスト名に触れているコメントの追随漏れ。** 設計が挙げた 3 か所のほかに、`crates/areka-kanade/tests/kanade/close_test_boot_greeting_tests.rs`（`close_refused_...` の名を引く説明）、`crates/areka-kanade/tests/kanade/steady_test.rs` の 2 か所（同）、`crates/areka/src/emo2_boot/balloon_visibility_tests.rs`（「中断のみを理由とする即時非表示は無い」の説明）、`enum VisibilityTrigger` の説明の「4 種」（5 種になる）。テストの緑・赤には影響しない。
3. **「内容を 1 つも持たない台本は指示を 1 件も配らない」は 1 つだけ例外がある。** 起動記録だけのトーク（`fn to_baseware_version` の「応答なし・かつ epilogue 非空」の腕）は、`fn append_epilogue` が空の台本へ運搬の指示を 1 件足すので、指示を 1 件配り、先頭は `ClearAll` ではない。文字を 1 つも持たないので、掛け金と旗の結論は変わらない。NoUserBreakCueSink の節の記述だけ直す。
4. **UI が表示の合図の線の送出端を持ち続ける副作用。** `UserBreakWiring` が `lifecycle_tx` の複製を持つので、`fn drain_lifecycle` の切断検出（`error!` 1 回）は本番で到達しなくなる（送出端が 1 本残るため）。dispatcher が落ちたときは他の記録（`talk_command_send_failed`）が出るので実害は小さい。受け入れる旨を Boundary か Risks に 1 行書く。
5. **受理の直後に別のトークへ差し替わったとき。** 受理から完了通知までの間に kanade がマウス応答などで差し替えると、古いトークの `TalkDone{Interrupted}` が先に dispatcher を通れば kanade で `unknown_talk_done` の `error!` になり（1 世代の照合 `choice_prev_talk` は選択起因の差し替えだけが書く）、終了の予約も失われる。選択肢の時間切れの解除が既に持っている競合と同じ種類で、窓は数ミリ秒。Risks に 1 行足す。
6. **Risks 1 行目の拡張。** 先頭に `\![enter,nouserbreakmode]` を置いたトークが、最初の指示を配る前（最大で再生の刻み 50 ミリ秒＋画面更新 1 回）に、**前のトークの居残りバルーン**へのダブルクリックで 1 文字も出さずに止められる窓がある。消滅の演出のように選択肢から始まる流れは `prev_press_selected` が守るので、実際に踏むのは「居残りバルーンを消そうとした瞬間に守られたトークが始まった」場合だけ。裁定 8（隠す判断と止める判断を分ける）の帰結として Risks に書く。
7. **要件 5.4 の読み替えが「要件本文との差」に載っていない。** 設計は「次のトーク」を「内容を持つ次のトーク」と読んでいる。閉じ忘れが無ければ差は 0 だが、読み替えなので指摘 2 とあわせて同じ節に載せる。

## 6. 最終判定

**GO**

- **理由**: 既存の構造（アクター＋単一の閉じ口＋純関数の判断中核）に沿い、§4 で当てた 15 項目（設計の「Existing Architecture Analysis」の 10 点を含む）が実コードと一致し、要件 70 項目の対応に欠けが無い。指摘 1 は運行側の 1 行とテスト 1 本、指摘 2 は文書の追記で、どちらも設計の骨格・規模（S〜M）・ファイル構成を変えない。
- **次の一歩**: 設計ディスカッションで指摘 1・2 を設計へ反映し、§5 の 7 点を取り込む。その後 `/kiro-spec-tasks areka-P0-balloon-break` へ進む。指摘 1 の掃除点とテストの並びは、タスクの「kanade user_break」に含める。
