# Requirements Document

> 本文の実測は **2026-09-24・本ブランチ**（main `5db3672a`＝棚卸⑯着地後・`baseware-root-layout` #12・`app-lifetime-separation` #49・`shiori-loadu` #48 はすべて着地済み）のもの。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。
> 要件 8 の裁定 1〜6 は brief の議題 ⑴〜⑸ と #57 の相乗りに対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。

## Introduction

### 誰が困っているか

α の利用者（第三者）。自分で入れたゴーストの SHIORI が動かないとき、**アプリが一瞬で消え、画面では何が起きたのか何も分からない**。ログには `ERROR` が残るが、利用者向けの表示は 0 件・終了コードは 0（完了 spec `areka-P0-shiori-loadu` の `research.md` §11 で 2026-09-23 に実測: YAYA `konnoyayame` は最初の問い合わせの 500 で **0.47 秒**、pasta `emo2` は初期化が偽で **1.7 秒**で終了・どちらも終了コード 0）。`loadu` でその 2 体は直ったが、第三者が持ち込むゴーストでは 32bit でない DLL・依存 DLL の欠落・壊れた辞書・SHIORI の内部エラー・応答の期限切れのどれでも同じ形が起きる。

### いま何が起きているか（2026-09-24 実測）

- **SHIORI の失敗はすべて kanade の中で 1 本の終了系列（Fault）に集まる。** `crates/areka-kanade/src/schedule/mod.rs` の `on_shiori_reply` は、応答待ちの相で `ShioriOutcome::Failed` を受けると `error!(event="shiori_failed")` を残して `to_unloading_fault` へ倒す（例外は 2 つ＝起動時の `username` 照会と、選択肢の往復中の失敗。後者は `schedule/steady.rs` の `choice_shiori_failed_as_204` が 204 と同じに扱い会話を続ける）。死活報告は同ファイルの `Input::ShioriDown { reason }` の腕（`error!(event="shiori_down")`）。`crates/areka-kanade/src/actor.rs` は送出失敗・応答の切断・期限切れ（`event="shiori_reply_timeout"`）の 3 腕で `ShioriFailure::Ipc` を再投入し、`notify_stop` は原因を控えられなかったときも `stop_cause_unknown` を残して Fault として通知する。
- **失敗の種類は通知の手前で失われる。** `crates/areka-kanade/src/msg.rs` の `KanadeStopCause` は `Quit`・`Forced`・`CloseSilent`・`DeadlineExceeded`・`Fault` の 5 値で中身を持たない。一方、内側には `ShioriFailure`（`Handshake`・`Timeout`・`Ipc`・`Shiori`・`Internal` の 5 種・それぞれ理由の文字列を持つ）と `ShioriDown { reason: String }` が残っている（確立の失敗と helper の予期しない終了は `crates/areka-kanade/src/shiori/real.rs` の `spawn_shiori_actor`／`report_exit_once` が `ShioriDown { reason }` として、呼出の失敗は同ファイルの `map_error` が `ShioriFailure` へ写す。エラー応答の状態コードは `ShioriFailure::Shiori` の理由の文字列「SHIORI error status 500」の中にだけある。GET の 400・500・`ErrorLevel` 付き応答が失敗、204 は無応答、311・312 その他は成功として通る＝`crates/shiori-host32-host/src/client.rs` の `map_get_result`）。
- **アプリの終了は 1 か所を通る。** `crates/areka/src/emo2_boot/frame.rs` の `run_ghost_quit_phase` は停止通知を受けると原因を問わず `quit_app(world, ExitOrigin::KanadeStopped(cause))`（`crates/areka/src/app_exit.rs`）を呼ぶ。`quit_app` は原因を `info!(event="app_exit")` に残して終了を指示するだけで、**原因を後から読める場所に置かない**。`ExitOrigin` は `KanadeStopped(KanadeStopCause)`・`Escape`・`Smoke`・`OsClose` の 4 値。
- **終了コードは理由を問わず 0。** `crates/areka/src/main.rs` の `fn main` が 0 以外で終わるのは、起動解決の失敗・起動窓を開けない・`app.run()` の失敗・終了統括（`GhostRuntime::shutdown`）の失敗・SERIKO の合流の失敗だけ。`app.run()` が正常に戻れば、その後の後始末（loop ticker の停止 → 終了統括 → SERIKO の合流 → 性能報告）を通って 0 で戻る。Fault もここ。
- **利用者向けの告知の部品は起動専用。** `crates/areka/src/alert.rs` の `AlertScene` は `RootMissing`・`GhostMissing`・`BalloonMissing`・`StartupWindow` の 4 場面、題名は定数 `TITLE`（「areka を起動できません」）で共通、`raise` は親窓なしの `MessageBoxW(MB_OK|MB_ICONERROR)` で必ず `error!(event="alert")` を 1 件残す。抑止は環境変数 `AREKA_NO_ALERT`（`suppressed_from`: 未設定・空・空白だけ・`0` なら出す、それ以外は抑える）。文面は `alert_text` が純関数で組み、`alert_tests.rs` の 9 本が固定している。
- **常設の smoke テスト `crates/areka/tests/smoke_boot_loop_exit.rs`** は 3 方向（① argv・② 根・③ 空の根）で、`AREKA_APP_SMOKE_EXIT_MS=500`・`AREKA_NO_ALERT=1`・`NO_COLOR=1` を子へ渡し、60 秒の見張りの内側で終了コードとログの目印を判定する。①② は終了コード 0 と窓の目印を見るだけで、**SHIORI の失敗が起きていないことは見ていない**（helper が無くても Fault で 0 終了＝緑になりうる盲点）。検体は `emo2`（32bit の pasta）で、`shiori-host32-helper.exe` の i686 成果物を要する（記憶 workspace-test-needs-i686-host32-artifacts）。
- **決定論的に失敗させる部品は揃っている。** x64 では `areka_ghost::ShioriWiring::Custom`（接続の閉包が `Err(String)` を返すと `connect_failed` → `ShioriDown`）、kanade の中では `Input::ShioriDown`／`ShioriOutcome::Failed(..)` の直接投入、`frame_ghost_quit_tests.rs` の `wiring_with_stop`（今は `Quit`・`Forced` だけ）。実機用の失敗検体は `crates/shiori-host32-testdll-loadu`（`shiori_loadu.dll`・`HOST32_TESTDLL_LOADU_FAIL=1` で `loadu` が偽を返し、それが無くても `request` は常に `400 Bad Request` を返す＝この 1 本で「接続できなかった」と「エラーを返した」の 2 種類を決定論的に作れる）。会話中の期限切れは本番の口 `AREKA_SHIORI_REQUEST_TIMEOUT_MS`（既定 60 秒＝`crates/shiori-host32-host/src/process_host.rs` の `REQUEST_TIMEOUT`）を極端に短くすれば実機で起こせる（`ShioriFailure::Timeout` として届く）。
- **起動時と会話中は終了の場所からは見分けがつかない。** 接続の失敗はアクターのスレッド上で非同期に起き、`areka_ghost::boot_with_kanade_stop` 自体は成功を返す（窓が出てから消える形）。
- **実 sink 結線（`wire_emo2_boot`）が不成立のときの LogSink 側の起動**（`fn main` の `else` 腕＝`areka_ghost::boot`）は停止通知の受け口を持たない。ここで Fault が起きると窓が残ったまま kanade だけが止まる（静的に確認・実走はしていない）。
- **完了 `areka-P0-app-lifetime-separation` 要件 3.8** は「終了操作 7 種のどれでも、終了の指示のあとの後始末の順序と終了コード（0）を今日と同じに保つ。」と書き、3.1「kanade の終了系列の完了通知」は原因を区別しないので、**Fault の終了コード 0 はこの 3.8 に含まれている**。#12 の 0 以外の終了コードは起動前の失敗だけで Fault に触れていない。
- **`doc/COMPAT_ARCHITECTURE.md` §8** に終了コード・SHIORI の失敗の見せ方の行は無い。

### 正典（ukadoc）の位置づけ

| 正典 | 逐語引用 | 本仕様への含意 |
|---|---|---|
| [SHIORI/3.0 ステータスコード](https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html#_30b9_30c6_30fc_30bf_30b9_30b3_30fc_30c9:1) | 「HTTPと同じく、200番台が成功、ほかはなにかしらの失敗（エラー）を示す。」「400 Bad Request リクエスト文字列が解釈不能だった。」「500 Internal Server Error SHIORI内部でなにかしらのエラーが起き、レスポンスを返せなかった。」 | エラー応答は「失敗」だと定めるが、**ベースウェアがどう振る舞うべきかは書いていない**（沈黙）。見せ方は areka の裁量＝§8 に記す。 |
| [DLL共通仕様 `loadu`／`load`](https://ssp.shillest.net/ukadoc/manual/spec_dll.html) | 「初期化に成功した場合は TRUE を、失敗した場合は FALSE を返却する」 | 初期化の失敗は DLL が明示的に返す。失敗したときにベースウェアが何を見せるかは沈黙。 |

SSP の挙動を実測して合わせることはしない（記憶 no-ssp-measurement-import-semantics-from-ukadoc）。

### 何を変えるか

**SHIORI が動かないとき、何が起きたかを利用者に告げてから終わる。** 起動時でも会話中でも、SHIORI の失敗で kanade が止まったら、全ゴースト窓を閉じたあと・プロセスが終わる前に、どのゴーストで・どんな種類の失敗が起きたかをメッセージボックスで告げる（既存の告知の部品を場面ごとの題名を持てる形に広げる）。失敗の種類と理由は kanade の停止通知に載せて終了の場所まで運ぶ。SHIORI の失敗で終わったことは終了コード（0 以外）でも分かるようにし、後始末の順序は今日のまま保つ。常設の smoke テストと有界の自動終了の無人走行は告知で止まらず、smoke には「SHIORI の失敗で終わる方向」を 1 本足す。判断の分岐は決定論テストで固定し、告知の窓は実機で一度見る。

## Boundary Context

- **In scope**:
  - SHIORI の失敗（起動時＝接続できない・初期化が偽／会話中＝応答の期限切れ・通信の切断）で kanade が止まったときの利用者向けの告知と、告知の文面（どのゴーストか・失敗の種類・理由の一行）。
  - エラー応答（400・500 など）を致命の失敗から外すこと＝kanade の腕の判断の変更 1 点だけ（裁定 3・2026-09-24 開発者裁定「500 はそのシーケンスにおける内部エラーでしかなく、常に復旧可能性がある。SHIORI プロトコルが流れているなら致命エラーではない」）。
  - 失敗の種類と理由を kanade の停止通知に載せて終了の場所まで運ぶこと（**運ぶだけ**。どの失敗を終了にするかの判断は変えない）。
  - 告知の部品の一般化＝場面ごとに題名を持てる形（後続 `ghost-install` #15 が乗る）。
  - SHIORI の失敗で終わったときの終了コード（0 以外）と、記録の目印。
  - 常設の smoke テストの追随（①② の盲点を塞ぐ・④ Fault 方向を足す）と、有界の自動終了との両立。
  - 判断の分岐の決定論テストと、実機での一度の目視。
  - `doc/COMPAT_ARCHITECTURE.md` §8 への記録（完了 spec 要件 3.8 の Fault に限った上書き・エラー応答を致命としない裁量）。
- **Out of scope**:
  - SHIORI の失敗そのものを減らすこと（原因ごとの修正は各 spec）。
  - kanade の各腕の判断のうち、エラー応答以外（接続の失敗・期限切れ・通信の切断・内部の失敗・死活報告）を Fault にする判断を変えること。
  - `shiori-host32-helper`／`shiori-host32-host` の失敗の種類と記録。
  - 切替先のゴーストが起動できない場合（`ghost-shell-balloon-switch` #13）・ゴーストの根が無い場合（完了 #12）・照会の往復の失敗の記録の文言（`popup-menu-residue` #44 の 6 番）。
  - 失敗したゴーストを別のゴーストで起こし直すこと（α 後）。
  - 告知に押されたボタンを返すこと（`terms.txt` の受諾／拒否＝#15 が足す）。
  - #57「起動後の途中終了で記憶の確定が飛ぶ」の修正（裁定 6＝`ghost-restart-unit` #58 へ）。
  - 実 sink 結線が不成立のときの LogSink 側の起動で Fault が起きる穴（裁定 5＝別に登記）。
- **Adjacent expectations**:
  - `ghost-install`（#15）は本仕様が一般化した告知の部品（場面ごとの題名）に乗り、押されたボタンを返す形は自分で足す。
  - `ghost-restart-unit`（#58）は `crates/areka/src/main.rs`・`app_exit.rs`・`emo2_boot/frame.rs` を共有するので本仕様の後に着地する。#58 が `fn main` の終了順序を関数へ括り出すとき、本仕様が足す「終了コードを後始末の後に決める」形をそのまま関数の中へ運ぶ。
  - `ghost-shell-balloon-switch`（#13）は `run_ghost_quit_phase` に「切替のときは終了しない」分岐を足す。本仕様は同じ関数に Fault の告知を足すので並走しない（本仕様 → #58 → #13 の直列）。
  - `alpha-release-signoff`（#17）は既知の制限「SHIORI が動かないとアプリが黙って消える」を一覧から外せる。
  - 完了 `areka-P0-app-lifetime-separation` 要件 3.8 は Fault に限って上書きする（完了 spec の文書は変えず §8 に記す）。それ以外の 6 種の終了操作の終了コード 0 と後始末の順序は変えない。

## Requirements

### Requirement 1: SHIORI の失敗で終わるときは黙って消えない

**Objective:** As a α の利用者, I want SHIORI が動かなくてアプリが終わるとき、何が起きたかを画面で知りたい, so that 自分で入れたゴーストのどこが悪いのか（DLL が読めない・初期化に失敗した・応答が返らない・通信が切れた）を見当付けられる

#### Acceptance Criteria

1. When kanade が SHIORI の失敗（停止原因 Fault）で止まった通知を受けて終了する, the areka shall 全ゴースト窓を閉じたあと、プロセスを終える前に、利用者向けの告知（メッセージボックス）を 1 回だけ出す。
2. The areka shall 起動時の失敗（接続できない・初期化が偽）でも会話中の失敗（応答の期限切れ・通信の切断）でも、同じ 1 か所の告知に落とす（失敗の入口ごとに告知を作らない）。
3. When 告知を出す, the areka shall 題名を起動失敗の告知（「areka を起動できません」）とは別の、会話中の失敗にも合う文言にし、本文に (a) どのゴーストか（`descript.txt` の `name` があれば名前と置き場所の絶対パス、名前が取れなければ置き場所だけ）、(b) 失敗の種類（要件 1.4 の語彙）、(c) 記録に残るのと同じ理由の一行、を載せる。
4. The areka shall 失敗の種類を次の 5 語のいずれかの平易な言葉で載せる: 「SHIORI に接続できなかった」（DLL が読めない・入口が無い・初期化が偽・確立の期限切れ）／「SHIORI の応答が期限内に返らなかった」／「SHIORI との通信が切れた」（helper の異常終了・切断）／「areka 側の内部の失敗」／「原因不明」（原因を控えられずに止まったとき）。エラー応答（400・500 など）は要件 6.1 により致命の失敗ではなくなるので、告知の語彙に持たない（将来エラー応答が致命になる経路ができたら、そのときに語を足す）。
5. When 告知を出す, the areka shall 同じ内容（題名・本文・失敗の種類・理由）を `error!` にも残す（記録だけで告知しない・告知だけで記録しない、のどちらも作らない＝完了 #12 要件 6.2 と同じ規律）。
6. While 告知を抑える環境変数 `AREKA_NO_ALERT` が設定されている（完了 #12 要件 6.5 と同じ判定規則）, when 告知を出す場面になる, the areka shall メッセージボックスを出さず、`error!` と終了コード（要件 3）だけを同じにする。
7. If kanade の停止原因が Fault 以外（正規の終了・強制終了・無言終了・別れの台詞の期限切れ）, then the areka shall 告知を出さない（0 件）。
8. If 終了の指示が SHIORI の失敗以外の経路（強制退避・smoke の自動終了・OS の閉鎖要求）から出た, then the areka shall 告知を出さない（0 件）。
9. The areka shall 告知の文面で起動時か会話中かを言い分けない（終了の場所からは見分けがつかず、誤った段を告げるくらいなら載せない）。
10. When 利用者が告知を閉じる, the areka shall そのままプロセスを終える（告知に「再試行」「別のゴーストで起動」などの新しい操作は置かない）。
11. When 告知を出す, the areka shall 告知の背後にゴーストの窓を残さない（窓は告知より先に閉じる）。
12. If 終了の指示を出したあとに別の停止通知や終了の指示が届く, then the areka shall 告知を重ねて出さない（告知は 1 プロセスに最大 1 回）。

### Requirement 2: 失敗の種類と理由が終了の場所まで届く

**Objective:** As a 開発者, I want kanade の中で分かっている失敗の種類と理由を、終了を指示する場所まで運びたい, so that 告知に失敗の種類を載せられ、ログを開かなくても種類が分かる

#### Acceptance Criteria

1. When kanade が SHIORI の失敗で終了系列に入る, the areka shall 停止通知に失敗の種類（要件 1.4 の 6 語に写せる区分）と理由の一行（対応する `error!` の `error=`／`reason=` と同じ文言）を載せて運ぶ（今日の「Fault」だけの通知は中身を持たない）。
2. The areka shall 失敗が Fault へ入る入口のどれからでも種類と理由を運ぶ: SHIORI 呼出の失敗のうち Fault へ入る 4 種（接続・期限切れ・通信・内部。エラー応答は要件 6.1 により Fault へ入らない）・死活報告 2 種（確立の失敗＝「接続できなかった」・helper の予期しない終了＝「通信が切れた」。理由の文字列の綴りで種類を判別する形にはしない）・送出失敗／応答の切断／応答の期限切れ（「通信が切れた」）・原因を控えられずに止まった場合（「原因不明」として運ぶ）。
3. The areka shall 失敗を Fault へ入れるかどうかの判断（どの腕が終了に倒すか）を、エラー応答（要件 6.1）を除いて変えない（運ぶ内容が増えるだけ）。
4. When 接続に失敗する SHIORI（x64 の決定論的な失敗）でゴーストを起動する, the areka shall 停止通知に「接続できなかった」の区分と接続失敗の理由が載る（実機でしか確かめられない形にしない）。
5. The areka shall 停止通知に載せた種類と理由を、終了を指示した場所（`quit_app`）の記録（`info!(event="app_exit")`）にも残す。

### Requirement 3: SHIORI の失敗で終わったことが外から分かる

**Objective:** As a 開発者（無人の走行を回す人・第三者の報告を受ける人）, I want SHIORI の失敗で終わったことをログを読まなくても区別したい, so that 自動の走行やバッチで「正常に終わった」と「SHIORI が動かなかった」を取り違えない

#### Acceptance Criteria

1. When SHIORI の失敗（停止原因 Fault）で終わる, the areka shall 0 以外の終了コードでプロセスを終える（完了 `app-lifetime-separation` 要件 3.8 の Fault に限った上書き。値は 1 つに定め、起動前の失敗と同じ値でも構わない）。0 以外にするのは「SHIORI の失敗が原因でプロセスそのものが終わる」ときに限る（今日はゴースト 1 体なので Fault は必ずプロセスの終了になる）。伺かは複数ゴーストを同時に起動するものなので、将来 1 体の失敗がアプリ全体の終了にならない形になったら、その 1 体の失敗は終了コードに映さない（1 体の失敗はアプリの失敗ではない＝2026-09-24 開発者裁定）。本仕様は「Fault なら必ずアプリが終わる」を契約にしない。
2. When SHIORI の失敗で終わる, the areka shall 終了の指示のあとの後始末（再生ループの停止 → ゴースト実行環境の終了統括 → SERIKO の合流 → 性能報告）を今日と同じ順序で最後まで通してから終了コードを決める（後始末を飛ばして早期に抜けない＝#57 の壊れ方を広げない）。
3. The areka shall Fault 以外の終了（正規の終了・強制終了・無言終了・別れの台詞の期限切れ・強制退避・smoke の自動終了・OS の閉鎖要求）の終了コード 0 と後始末の順序を今日と同じに保つ。
4. While 告知を抑える環境変数が設定されている, when SHIORI の失敗で終わる, the areka shall 終了コードを抑止していないときと同じにする（抑止は表示だけを変える）。
5. When SHIORI の失敗で終わる, the areka shall 「SHIORI の失敗で終わった」ことを示す `error!` を 1 件残し、その記録から失敗の種類と理由が読める（要件 1.5 の告知の記録と同じ 1 件で足りる）。
6. The 本仕様 shall `doc/COMPAT_ARCHITECTURE.md` §8 に、Fault の終了コードを 0 以外にしたこと（完了 spec 要件 3.8 の Fault に限った上書き）と、エラー応答（400・500 など）を致命の失敗とせず会話を続ける裁量（要件 6.1・裁定 3）を、正典の沈黙の根拠とともに記す。0 以外の終了コードは「SHIORI の失敗が原因でプロセスが終わるとき」に限る限定（要件 3.1）も同じ行に記す。

### Requirement 4: 無人の走行を告知で止めない

**Objective:** As a 開発者, I want 常設の smoke テストと有界の自動終了の無人走行が告知の窓で止まらず、SHIORI の失敗が起きたら赤になってほしい, so that 告知を足しても検査が固まらず、しかも SHIORI が動かない壊れ方を検査が見逃さない

#### Acceptance Criteria

1. The 本仕様 shall 常設の smoke テスト `crates/areka/tests/smoke_boot_loop_exit.rs` の 3 方向（① argv・② 根・③ 空の根）と 60 秒の見張り・終了コードの判定を残し、3 方向とも緑のままにする。ただし要件 3.1・4.2 により ①② は「`shiori-host32-helper.exe` の i686 成果物が `areka.exe` の隣にあること」を前提とする（今日は無くても Fault → 終了コード 0 で緑になっている盲点そのもの）。テストはこの前提を自分で満たす（無ければ `target/i686-pc-windows-msvc/{debug,release}` から隣へ複製し、それも無ければ建て方を案内して失敗する。本番コードには手を入れない）。
2. The 本仕様 shall ①② の判定に「SHIORI の失敗で終わっていない」ことを足す（要件 3.5 の記録の目印が 0 件・終了コード 0）。今日は SHIORI が動かなくても終了コード 0 で緑になりうるので、その盲点を塞ぐ。
3. The 本仕様 shall smoke に ④ **失敗方向**を 1 本足す: SHIORI が確実に失敗する検体で起動し、告知を抑止し、見張りの内側でプロセスが終わり、終了コードが 0 以外で、要件 3.5 の記録の目印が 1 件ある。
4. When ④ の失敗する検体を用意する, the 本仕様 shall 本番コードに検証用の口を足さず、既存の検証用 DLL（`shiori-host32-testdll-loadu`＝`HOST32_TESTDLL_LOADU_FAIL=1` で `loadu` が偽を返す口）を SHIORI に持つゴーストのフォルダを組み立てて作る。同 DLL の `request` が常に返す 400 は要件 6.1 により致命ではなくなるので、④ の失敗は `loadu` の偽（＝「接続できなかった」）で作る。その環境変数が areka から helper の子プロセスへ届かないと分かったら、本番コードに口を足さず、検証用 DLL の側（test crate）を「環境変数なしで `loadu` が偽を返す」形に直してよい。
5. While 告知を抑える環境変数が設定されている and 有界の自動終了（`AREKA_APP_SMOKE_EXIT_MS`）が設定されている, when SHIORI の失敗が自動終了より先に起きる, the areka shall 告知を出さずに 0 以外の終了コードで終える（自動終了が先なら 0＝要件 3.3）。
6. The areka shall 告知の抑止と有界の自動終了を別の関心のまま保つ（自動終了が設定されているだけでは告知を抑えない＝完了 #12 裁定 2 と同じ）。

### Requirement 5: 告知の部品は場面ごとの題名を持てる

**Objective:** As a 開発者, I want 既存の告知の部品を「起動できません」以外の場面にも使える形に広げたい, so that 本仕様の告知と後続 `ghost-install` の告知が 2 系統にならない

#### Acceptance Criteria

1. The areka shall 告知の場面に「SHIORI が動かなくなった」を 1 つ足し、場面ごとに題名を持てる形にする（既存 4 場面の題名「areka を起動できません」は変えない）。
2. The areka shall 既存 4 場面（根なし・ゴーストなし・バルーンなし・起動窓を開けない）の文面・記録・抑止の振る舞いを変えない（`alert_tests.rs` の既存テストは無改変で緑）。
3. The areka shall 告知の文面を純関数で組み、テストで固定できる形を保つ（要件 1.3・1.4 の文面）。
4. The 本仕様 shall 押されたボタンを返す形（OK／キャンセルの受諾・拒否）を足さない（`ghost-install` #15 が自分の要件で足す）。

### Requirement 6: 既存の振る舞いと境界を守る

**Objective:** As a 開発者, I want 本仕様が kanade の判断・他の終了操作・並走 spec の持ち場を変えないこと, so that 直列で続く #58・#13 の前提が崩れない

#### Acceptance Criteria

1. When SHIORI がエラー応答（400・500・`ErrorLevel` 付き＝`ShioriFailure::Shiori`）を返す, the areka shall 起動時でも会話中でも、それを致命の失敗（Fault）にせず、204（返事なし）と同じ扱いでその照会の台詞を無しにして会話を続ける（今日は選択肢の往復中だけに限られている `choice_shiori_failed_as_204` の扱いを、応答待ちの全部の相へ広げる）。そのときも記録は必ず 1 件残す（今日の `error!(event="shiori_failed")` を残すか `warn!` に下げるかは設計で決める。記録なしで黙らせない）。エラー応答が続いても回数で終了に倒す閾値は置かない（SHIORI プロトコルが流れている限り致命ではない）。
2. The areka shall 起動時の `username` 照会の失敗で起動を続ける今日の判断を変えない。
3. The 本仕様 shall #57「起動後の途中終了で記憶の確定が飛ぶ」（`app.run()` が失敗を返すと後始末を通らない）を直さず、`ghost-restart-unit` #58 へ送る（裁定 6）。ただし要件 3.2 により本仕様が新しく「後始末を通らない終わり方」を足すことはない。
4. The 本仕様 shall 実 sink 結線が不成立のときの LogSink 側の起動で Fault が起きると窓が残って止まる穴を直さず、ロードマップの台帳に登記だけの行として起票する（裁定 5）。
5. The 本仕様 shall `run_ghost_quit_phase` に「停止原因によって終了しない」分岐を足さない（切替のときに終了しない分岐は #13 の持ち場）。
6. The 本仕様 shall 本番コードが読む環境変数を新しく足さない（既存の `AREKA_NO_ALERT`・`AREKA_APP_SMOKE_EXIT_MS`・`AREKA_SHIORI_REQUEST_TIMEOUT_MS` で足りる）。

### Requirement 7: 決定論テストと実機確認

**Objective:** As a 開発者, I want 判断の分岐が決定論テストで固定され、告知の窓を実機で一度見ていること, so that 後の変更で「黙って消える」形に戻ったら赤になる

#### Acceptance Criteria

1. The 本仕様 shall 告知の文面（要件 1.3・1.4）を、失敗の種類 6 語 × ゴースト名の有無で純関数のテストにより固定する。
2. The 本仕様 shall 「停止原因（5 値）と終了の経路（強制退避・smoke・OS の閉鎖要求）のそれぞれで、告知を出すか・終了コードを何にするか」を 1 つの判定で決め、その表を決定論テストで固定する（Fault だけが告知あり・0 以外、他はすべて告知なし・0）。
3. The 本仕様 shall kanade の失敗の入口ごと（呼出失敗のうち Fault へ入る 4 種・死活報告・送出失敗／応答の切断／期限切れ・原因不明）に、停止通知へ載る種類と理由を決定論テストで固定し、エラー応答（`ShioriFailure::Shiori`）は起動時・会話中のどの相でも Fault にならず会話が続き記録が 1 件残ることを同じ型のテストで固定する。
4. The 本仕様 shall `run_ghost_quit_phase` のテスト（`frame_ghost_quit_tests.rs`）に Fault の場合を足し、種類と理由が終了の指示まで届くことを固定する。
5. The 本仕様 shall x64 で接続に失敗する SHIORI（`ShioriWiring::Custom`）を使い、起動 → 接続失敗 → 停止通知（接続できなかった・理由あり）までを決定論テストで固定する（要件 2.4）。
6. The 本仕様 shall 足すテストを判断の分岐に限り、既に確かめられている配線（マウント→SHIORI→sink・窓の配置・終了の指示・後始末の順序）を再テストしない。
7. When 実機で確認する, the 開発者 shall ① 失敗する検体（`loadu` が偽を返す検証用 DLL を SHIORI に持つゴースト）で起動し、告知の窓が出て（題名・ゴースト名・種類「接続できなかった」・理由）、OK で閉じると終了コードが 0 以外でプロセスが残らないこと、② 動く検体で `AREKA_SHIORI_REQUEST_TIMEOUT_MS` を極端に短くして起動し、会話中の期限切れで告知（種類「応答が期限内に返らなかった」）が出て同様に終わること、③ ① と同じ検体に `AREKA_NO_ALERT=1` と有界の自動終了（失敗より十分長い時間）を付けて起動し、窓が出ずに 0 以外で終わりプロセスが残らないこと、を見る。`RUST_LOG` は判定の分岐（kanade の `shiori_failed`／`shiori_down`・areka の `ghost_quit`／`app_exit`／`alert`）の水準まで開ける（記憶 real-machine-signoff-needs-trace-level-for-the-deciding-branch）。
8. The 本仕様 shall 実機の 3 走行の結果（コマンド・終了コード・目印の件数）を spec の文書に残す。

### Requirement 8: 裁定（暫定・要件ディスカッションで確定）

**Objective:** As a 開発者, I want brief が挙げた議題 ⑴〜⑸ と #57 の相乗りを要件の段階で一旦決めておくこと, so that ギャップ分析と設計がこの形で進み、覆すなら要件で覆す

#### Acceptance Criteria

1. The 本仕様 shall **裁定 1（Fault の終了コード・brief 議題 ⑴）**として、SHIORI の失敗で終わるときの終了コードを 0 以外にし、完了 `app-lifetime-separation` 要件 3.8 を Fault に限って上書きする（要件 3.1・3.6）。根拠: 出どころの実測が「終了コードでは壊れ方を判定できない」と記し、無人の走行で区別する手段が他に無い。smoke の競合（自動終了より先に Fault が起きる場合）は要件 4.2・4.5 で判定を明文にして塞ぐ。**別案**（0 のまま・記録だけで区別）は採らない。**2026-09-24 要件ディスカッションで確定**: 開発者「main まで Result が Err で落ちる状況なら 0 以外が妥当。ただし伺かは複数ゴーストが同時に起動し、1 体が落ちても全体のエラーには波及しない」＝0 以外はプロセスが終わるときだけ・複数ゴーストの形（α 後）では 1 体の失敗を終了コードに映さない（要件 3.1 に転記・§8 にも同じ限定を記す）。
2. The 本仕様 shall **裁定 2（失敗の種類を告知に載せる・brief 議題 ⑵）**として、失敗の種類と理由を kanade の停止通知に載せて運ぶ（要件 2）。根拠: brief の Desired Outcome 1 が「失敗の種類が平易な言葉で載る」を完了条件に置く。kanade の 4 ファイル（`msg.rs`・`actor.rs`・`schedule/mod.rs`・要件 2.2 により `shiori/real.rs`）に手が入り #13 との共有が増えるが、直列（本仕様 → #58 → #13）なので並走の問題は生じない。**別案**（種類を載せず「SHIORI が動かなくなった」とだけ告げる・kanade 無改変）は採らない。**2026-09-24 要件ディスカッションで確定**: 開発者「推奨で。少なくともログには理由が分かる状態にしておかなければならない」＝種類と理由を運ぶ。理由の一行は告知に載る／載らないに関わらず、終了の指示の記録（要件 2.5 の `app_exit`）と失敗の記録（要件 3.5 の `error!`）の両方から必ず読める（要件 2.1・2.5・3.5 は必達）。
3. The 本仕様 shall **裁定 3（エラー応答・brief 議題 ⑶）**として、エラー応答（400・500 など）を致命の失敗から外し、起動時でも会話中でも 204 と同じ扱いで会話を続ける（要件 6.1・3.6・7.3）。**2026-09-24 要件ディスカッションで暫定裁定（告知して終える）を覆して確定**: 開発者「500 は『そのシーケンスにおける内部エラー』でしかなく、常に復旧可能性がある。SHIORI プロトコルが流れているなら致命エラーではない」。根拠: 正典はエラー応答を「失敗」とだけ定めベースウェアの振る舞いに沈黙する（areka の裁量）ので、どの照会が失敗したかの単位で扱う。利用者から見える差: 辞書の 1 か所が壊れたゴーストは、その台詞だけ黙り、アプリは生きる（起動時の最初の問い合わせが 500 なら、ゴーストは出るが起動の台詞が無い）。「黙って壊れる」への戻りは記録 1 件（要件 6.1）で防ぐ。この 1 点だけ kanade の腕の判断を変える（brief の Out of Boundary を開発者裁定で上書き）。
4. The 本仕様 shall **裁定 4（告知を出す場所・brief 議題 ⑷）**として、告知を「全ゴースト窓を閉じたあと・プロセスが終わる前」に出す（要件 1.1・1.11）。利用者から見える差は「告知の背後に固まった窓が残らない」こと。フレームの相の中（ECS の system の中でモーダルを回す）は入れ子のメッセージループを作るので採らない。
5. The 本仕様 shall **裁定 5（LogSink 側の起動の穴・brief 議題 ⑸）**として、実 sink 結線が不成立のときの起動で Fault が起きると窓が残って止まる穴を本仕様で拾わず、ロードマップの台帳へ登記だけの行として起票する（要件 6.4）。根拠: 静的に読めただけで実走はしておらず、その起動はもともとゴーストが喋らない縮退の形（実 sink が無い）で、α の第三者が踏む経路ではない。規模 S を守る。引受先（#58 か新規）は要件ディスカッションで確定する。
6. The 本仕様 shall **裁定 6（#57 の相乗り）**として、#57「起動後の途中終了で記憶の確定が飛ぶ」を本仕様で採らず `ghost-restart-unit` #58 へ送る（要件 6.3）。根拠: #58 が `fn main` の終了順序を関数に括り出すので、`Err` の腕でも同じ関数を通す直しはその関数の中に置くのが筋（brief と #58 の brief が一致）。本仕様は要件 3.2 で「後始末を通らない終わり方」を新しく足さないことだけを守る。
7. Where 設計・実装の途中で裁定 1〜6 のいずれかを覆す必要が判明する, the 本仕様 shall 開発者へ議題として上げ、確定を待ってから要件・設計の該当箇所を改める。
