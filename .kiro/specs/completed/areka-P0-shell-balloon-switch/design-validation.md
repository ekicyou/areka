# 設計検証レポート — areka-P0-shell-balloon-switch

> 検証日: 2026-09-30・本ブランチ（`1cbd438e` の design.md）。入力は requirements.md・design.md（997 行）・research.md §9〜§12・steering。コードの裏取りは、下の指摘が分かれ目になる箇所だけを「何の定義か」で読んだ。

## 設計レビューの要約

層ごとの責務の切り方（切れ目の判定は kanade、差し替えの順序は seriko の表示の流れ、片付けと登録は present、判断と記憶は UI）は筋が通っており、要件 1〜11 のほぼ全項目に部品と流れが対応している。ただし、**UI に seriko の送り手の複製を持たせると、ゴーストを降ろすときの seriko の join が戻らなくなる**点と、**「台詞の切れ目に達した」の返事が資産づくりの完了より先に来たとき、その返事が古くなる**点の 2 つは、タスク生成の前に design.md で手当てが要る。手当てはどちらも局所的で、構成の組み直しは要らない。

## 確かめたこと（指摘に至らなかったもの）

- **要件 8.1（触らないファイル）は守られている。** `ghost_switch.rs`・`ghost_switch_tests.rs`・`ghost_switch_test_support.rs` は `wire_emo2_boot`・`boot_with_origin`・`build_boot_assets`・`prepare_ghost_windows`・`DisplayCommand`・`PresentBridge`・消費者台帳・メニューの枠のどれも呼ばず・網羅の match も持たない（grep で 0 件）。`boot_with_origin` の署名変更の本番の呼び手は `emo2_boot/mod.rs` の `wire_emo2_boot` だけ。
- **要件 6.4 の運び手の変更は正当。** `Emo2BootInputs { … }` と `StartupDescriptValues { … }` の構造体リテラルは `ghost_switch_test_support.rs` に実在する（それぞれ 1 か所）。欄を足せば触れてはならないファイルの書き換えをコンパイルが要求するので、`BootShellChoice` と引数で運ぶ設計の判断は要件 8.1 と両立させる唯一に近い形。利用者から見える振る舞い（3 か所が同じシェルを見る）は要件どおり。
- **要件 8.2・8.11 を超える kanade の口（`AwaitTalkGap`）は正当。** 印の無い経路でも要件 5.4・1.14・5.7 は「kanade が終了系列へ入ったか」を知らないと判定できず、`KanadeNotice` に変種を足すと `ghost_switch.rs` の網羅の match が壊れる。裁定 8 の根拠がそのまま当てはまる。`on_raise_event`・`steady.rs` は不変で、要件 8.3 も守られる。
- **seriko の inbox の順序づけは成り立つ。** 本番で present の線へ積むのは `PresentBridge` だけで（`attach.rs` の最初の表示と可視性の相の `Hide` は UI スレッドの同期の適用）、ループの ticker も同じ inbox に入る。`Rebased` より前の指令は古い装着へ、後の指令は新しい装着へ当たる。
- **作ってから片付ける順序も成り立つ。** 読み込み・解釈・復号は背景のスレッドで済み、`apply_replace` の中で失敗しうるのは「表に無い target」だけ（そのとき古い装着も無い）。窓が空になる形は作らない（要件 5.6）。
- **失敗の経路はすべて記録つき**（Error Handling の表で `warn!`／`info!`／`error!` が割り当て済み）。新規ファイルはいずれも 500 行未満の見込みで、変更ファイルの行数の見積もり（`msg.rs` 883・`actor.rs` 656・`schedule/mod.rs` 830・`balloon_visibility.rs` 875 など）は実測と一致し、どれも 1,000 行の内側に収まる。

## 重大な指摘（3 件）

🔴 **指摘 1: `Emo2Wiring` に持たせる `SerikoSink` の複製が、ゴーストを降ろすときの seriko の join を止める**

- **何が問題か**: `GhostSession::shutdown_impl`（`crates/areka/src/ghost_session.rs`）の ③ は「seriko の inbox の送り手が全部落ちるまで」join で待つ。同関数の doc は「呼び手は自前の `SerikoSink` クローンを保持しない」を前提に明記している（`spawn_seriko` の終わり方は `SerikoSink::close` か全 drop の 2 つで、`shutdown` は drop に依る）。設計は `Emo2Wiring` に `seriko: Option<SerikoSink>` を足すが、`Emo2Wiring` は降ろした後も World に残り、次の `wire_emo2_boot` で差し替わるまで生きる（`app_exit.rs` の `close_windows_for_restart` の doc「ゴーストごとの結線（`Emo2Wiring`…）: 起こすたびに `insert_non_send` で差し替わる」）。ゴースト切替の `take_down`（`ghost_switch.rs`）は `Emo2Wiring` が居るまま `session.shutdown` を呼ぶ。
- **影響**: シェル・バルーンの切替を 1 度も使わなくても、ゴースト切替・ネットワーク更新の読み直し・OS のセッションの終了で UI スレッドが止まる（アプリが固まる）。要件 8.5（ゴースト切替・終了・更新の読み直しを変えない）に反する。
- **提案**: 複製の持ち主を `GhostSession` にする（`shutdown_impl` の ② で `ghost` と一緒に落とし、③ の join の前に送り手が消える）。差し替えの相は `GhostSlot` 経由で送り手を借りる。あるいは `shutdown_impl` の ③ の前に `SerikoSink::close` を送る形でもよい（ただし `ghost_session.rs` の doc の前提も書き換える）。決定論テストに「シェル切替を 1 度した後のゴースト切替・終了で降ろしが戻る」を 1 本足す。
- **対応する要件**: 8.5・5.8・2.5
- **根拠の箇所**: design.md「Modified Files」の `emo2_boot/mod.rs`（`SerikoSink` の複製を `Emo2Wiring` へ）と `frame/wiring.rs`（`seriko: Option<SerikoSink>`）。Boundary Commitments・Revalidation Triggers には、この寿命の取り決めが書かれていない。

🔴 **指摘 2: 「切れ目に達した（`Reached`）」が資産づくりの完了より先に来ると、差し替えの時点ではもう切れ目でない**

- **何が問題か**: Flow 1 ⑵ は「資産づくりと切れ目の待ちを並行して進め、両方がそろった最初のフレームで seriko に頼む」。kanade は `Reached` を返した時点で見張りを終えるので、その後で資産づくり（画像の復号・アトラス）が数百 ms かかる間に、新しいトーク（`OnSecondChange` の独り言・クリックの応答）が始まっても、終了の保留（メニューの「終了」・`\-` の台本）が起きても、UI はそれを知らずに差し替え・記憶・`OnShellChanged` へ進む。とくに `raise-event` 無しの台本の末尾の命令とメニューのバルーンは、切れ目がすぐ来て復号が律速になるので起きやすい。research §10.1 はこの窓を「1〜2 フレーム」と見積もるが、実際には資産づくりの時間ぶん広がる。
- **影響**: 裁定 2（差し替えは台詞の切れ目で行い、表示中の文字に触れない）が崩れ、バルーンの切替が台詞の途中で起きうる。終了の保留の最中に差し替えて `LastShell`／`LastBalloon` を書く形も出る（要件 5.7 の「進行中」は要件 1.12 の定義で `OnShellChanged` を送り終えるまでを含む）。
- **提案**: 資産が `Reached` より後にそろったときは、そろったフレームで `AwaitTalkGap { raise: None }` をもう 1 度送り、その返事（今が切れ目ならすぐ `Reached` が返る）で差し替える。口は既にあるので追加の部品は要らない。あるいは印の無い経路では最初から「資産がそろってから `AwaitTalkGap` を送る」順にする。どちらでも残る窓は返事から drain までの 1〜2 フレームだけになり、research §10.1 の見積もりと一致する。`frame/switch_tests.rs` に「資産が遅れて届き、その間に始まったトークの終わりまで差し替えを待つ」「その間の終了の保留で取りやめる」を足す。
- **対応する要件**: 2.2・2.3・3.1・5.7・1.12・12.2（裁定 2）
- **根拠の箇所**: design.md「System Flows」Flow 1 の判断 ⑵・Flow 2（`Reached` の後は見張らない）・SwitchPhase の `Waiting` の見極め。research.md §10.1。

🔴 **指摘 3: 印の台詞の中断の判定（`is_marked_break`）の評価順が design.md の中で食い違っている**

- **何が問題か**: 「Modified Files」は `on_talk_done` の `break_quit` に「`&& !talk_gap::is_marked_break(&state, &done)` を掛ける」と書き、TalkGap の State Management は「`take_user_break_quit` の**前**に評価する」と書く。今の `on_talk_done`（`crates/areka-kanade/src/schedule/mod.rs`）は `user_break::take_user_break_quit(&mut state, &done) && !change::is_change_phase(..)` の順で、`take_user_break_quit` が `state.user_break_talk.take()` で帳簿を空にする。前者の字面どおり後ろに掛けると、`is_marked_break` は空の帳簿を読んで常に偽になり、`\-` 入りの `OnShellChanging` の台詞の中断でアプリが終わる（裁定 8 が退けた形そのもの）。また、`step` の後の `observe` が `BrokenByUser` を知るには、帳簿が空になる前の判定結果を `State.talk_gap` に残しておく必要があるが、その書き込み点が明記されていない。
- **影響**: 要件 5.2・8.11 ⑵ の核心。`talk_gap_tests.rs` の中断のテストで赤にはなるが、タスクの記述が「Modified Files」の字面から作られると、実装者が誤った順で書いて手戻りになる。
- **提案**: design.md の「Modified Files」の `schedule/mod.rs` の行を「`take_user_break_quit` の前に `let marked_break = talk_gap::is_marked_break(&state, &done);` を取り、真なら `State.talk_gap` の `Marked` を `BrokenByUser` にしてから `break_quit` に `&& !marked_break` を掛ける」に揃える。帳簿を空にする単一の履行点は `take_user_break_quit` のままにできる。
- **対応する要件**: 5.2・8.11・11.4
- **根拠の箇所**: design.md「File Structure Plan › Modified Files」の `schedule/mod.rs` の行と、「Components › TalkGap › State Management」の `is_marked_break` の項。

## 設計の強み

1. **差し替えの順序を seriko の表示の流れに乗せた判断。** `DisplayCommand::Rebased` → `PresentCommand::ReplaceTarget` を同じ FIFO に並べることで、世代番号も読み捨ても 1 フレームの遅れも要らずに「古い指令は古い装着へ・新しい指令は新しい装着へ」が構造で決まる。present は seriko を、seriko は present を知らないまま（写しは areka の `PresentBridge`）で、依存の向きも崩していない。
2. **触れてはならないファイルと既存の判断を守るための迂回が、どれも実測に基づいている。** `Emo2BootInputs` のリテラルの所在、`KanadeNotice` の網羅の match、`handle_message` のテストの呼び出し 45 本、`resolve.rs` の 952 行と既存の呼び出し約 30 本を実際に数え、その上で欄を足さない・署名を変えない・本体を隣へ移す形を選んでいる。要件の字面から外れる 2 点（8.2・8.11 と 6.4）も冒頭と research §12 で自ら申告している。

## 最終判定

**判定: GO（条件つき）**

**理由**: 構成と責務の切り方に根本の食い違いは無く、要件 1〜12 の全項目に実装の道筋がある。上の 3 件はいずれも局所の手当て（送り手の持ち主を移す・資産がそろった後に切れ目を 1 度確かめ直す・評価順の字面を揃える）で閉じられ、構成の組み直しは要らない。ただし指摘 1 を残したままタスクへ進むと、ゴースト切替と終了が固まる欠陥を作り込むので、タスク生成の前に design.md へ反映すること。

**次の手順**:
1. 設計ディスカッション（`/kiro-design-discussion areka-P0-shell-balloon-switch`）で指摘 1〜3 を design.md に反映する。
2. 同じ場で、design.md 冒頭の「要件の本文の追随が要る 2 点」（要件 8.2・8.11 の口の広さ・要件 6.4 の運び手）を開発者が確かめ、要件 12.10 に従って requirements.md の本文を追随させる（振る舞いは変わらないので裁定は覆らない）。
3. その後 `/kiro-spec-tasks areka-P0-shell-balloon-switch` でタスクを作る。

### 補足（重大ではないが、タスク生成で拾うと手戻りが減るもの）

- `build_boot_assets` に `shell` の引数を足すと、`assets_tests.rs`（976 行・行を足さない約束）の呼び出し 12 か所と `frame_attach_tests.rs`・`frame_visibility_integration_tests.rs`・`spine.rs` の呼び出しも直す必要があり、複数行の呼び出し（`assets_tests.rs` の `build_boot_assets(` が改行されている 1 か所）は整形で行が増えうる。「Modified Files」に載っていない。`build_boot_assets` の署名は据え置き、シェル名つきの兄弟関数を足す形にすると呼び出しの追随が 0 になる。
- `\![change,shell,B]\-` のように命令の直後で台本が自ら終わると、`AwaitTalkGap` が届いた時点で kanade は終了系列にあり、UI は `warn!(skin_switch_not_steady)` を残す。Error Handling の表の「命令の台本が終了で終わった」は `info!(skin_switch_dropped, reason=closing)` なので、同じ場面が届く時機しだいで `warn!` と `info!` に分かれる。実害は無いが、実機サインオフの目印の件数を数えるときに注意がいる。
