# Brief: areka-P0-ghost-shell-balloon-switch

> 2026-09-18 `/kiro-discovery` 再入（棚卸⑭＝α ゴールへの組み直し）で起票。`doc/ukadoc-coverage/roadmap-draft.md` 段階 B 順位 2 の束「切替」（候補名 `areka-P0-shell-balloon-switch`）を、ゴースト切替まで含めて引き受ける。
> 本文の file:line は**起票時の実測値**（2026-09-18）。着手時に必ず引き直すこと。

## 2026-09-26 棚卸⑰の再測定（main `13b72893`）

**`ghost-restart-unit` と `shiori-fault-notice` が着地し、本仕様の前提のうち 4 つが「済み」になった。** 残るのは kanade の握手と、起こし直しを呼ぶ側の組み替えである。想定タスクは 18〜21 本で上限（20 本）に張り付くので、**棚卸⑰で名前解決を S の別仕様へ切り出した＝`areka-P0-ghost-change-name-resolution`**（`\+`／`\_+`・`random`／`sequential`・`lastinstalled` の受け皿）。本仕様は**名指しの `\![change,ghost,名(,--option=raise-event)]` だけ**を受け、`random`／`sequential`／`lastinstalled` など解決できない名前は「該当なし → 正典どおり無視＋`warn!`」で受ける（`ghost-change-name-resolution` がその腕を置き換える）。分割後の本仕様は **L（14〜16 タスク）**。Scope の In から `\+`／`\_+` と名前解決の 2 行、Boundary Candidates の「名前解決」は `ghost-change-name-resolution` へ移った（下の旧本文は起票時のまま残す）。

**済んだ項目（`ghost-restart-unit`＝`crates/areka/src/ghost_session.rs`）**
1. 終了順序は `GhostSession::shutdown(self, CloseReason)` の 1 関数になった（ticker 停止 → ゴースト実行系の終了 → seriko の join）。perf の最終報告は `fn main` に残る。（09-24 節の項目 2）
2. 「全窓を閉じるが終了しない」は `app_exit::close_windows_for_restart(world) -> WindowsClosed`。閉じた証の唯一の消費先は `ghost_session::reopen_ghost_windows(world, cfg, closed)`。**どちらも本番の呼び手が 0 で `allow(dead_code)` が付いている＝本仕様が最初の呼び手**（allow を外す）。（項目 4）
3. 系の登録は `ghost_session::register_systems`（プロセスに 1 回）、ゴーストごとの状態は `ghost_session::boot_ghost(world, GhostBootInputs, &StartupDescriptValues, &GhostDecision, &BalloonDecision) -> GhostSession`（n 回）。`MenuWiring` は起こすたびに新品になるので「ゴースト」枠の登記は起こすたびにやり直す（`menu/mod.rs` の `MenuRegistry` の型の説明に契約あり）。項目 5 が挙げた `main.rs` の `open_startup_window` はもう無い。（項目 5）
4. `wire_emo2_boot` は `&mut World` と `Emo2BootInputs`（根 2 つ・SHIORI の結線・時計・記憶の置き場）で呼べる。偽の SHIORI は `ShioriWiring::Custom` で入る。窓を作る側は `WintfTaskPool` へ積むだけなので、フレームの系の中から呼んでも借用は衝突しない（窓は次の tick で出る）。項目 6 の「系の外で実行する」工夫は要らなくなった。（項目 6）
5. `shiori-fault-notice`: 告知の部品 `alert.rs` は場面ごとの題名を持てる（`AlertScene` に切替失敗の場面を 1 つ足せば足りる。押されたボタンを返す形は無い＝本仕様には不要）。`run()` の後始末は `finish_after_run`（`main.rs`）で 1 回通る。`run_ghost_quit_phase`（`emo2_boot/frame.rs`）は停止通知を全件取り出して必ず終了へ進む——切替の分岐を足す場所は本仕様のまま（`shiori-fault-notice` 要件 6.5 が明記）。

**確定した前提・新たに分かったこと**
6. **項目 3 の残り物の通知は必ず 1 件来る。** `GhostSession::shutdown` は kanade へ `ForceQuit` を送り、kanade は終了系列の最後で停止通知（原因 `Forced`）を必ず 1 件送る（`crates/areka-kanade/src/actor.rs` の `notify_stop`）。正典どおり `OnClose` と別れの台詞を通してから降ろしても、原因が `Quit`／`CloseSilent` になるだけで 1 件来る。受け口はプロセスに 1 つ（`KanadeStopRx`）。`ghost-restart-unit` の 2 周テスト `boots_twice_in_one_process_without_double_registration` は判定の中でこの 1 件を自分で読み捨てているので、本番の「次のフレームで終了してしまう」症状は見ていない。**原因の値だけでは終了と切替を見分けられない**（`KanadeStopCause` は 5 値のまま・`stop_cause_of` と `app_exit::fault_of` は網羅 match＝値を足せばコンパイルが漏れを止める）。
7. **`GhostSession` は World の外に居る。** `fn main` のローカル変数を `finish_after_run` のクロージャが持っていく。切替をフレームの系（`run_ghost_quit_phase`）から行うには、`GhostSession` を World の NonSend 資源に置くか、`main` 側に切替の口を出す。これが本仕様に残った唯一の構造の変更である。seriko の join は UI スレッドを塞ぐので、降ろして起こし直す間に画面が止まる時間は要件段階で上限を決め、実機で測る。
8. **記憶は `GhostRoute::Argv` だと書かれない**（`boot_resolve::LastUsed::record`）。切替後に `on_boot_ok` へ渡す `GhostDecision` の route は Argv 以外にする（`GhostRoute` に切替の値を足すかは設計で決める）。
9. `\+`／`\_+` は字句解析で `Bare("+")`／`Bare("_+")` に切れ、`decode_bare` の既定の腕で `Raw` になり compile が捨てる（項目 7 ⓐ の成立を確認）→ `ghost-change-name-resolution` が持つ。
10. `GhostBootOptions {` の構造体リテラルは 28 か所・17 ファイル（欄は足さず派生関数で渡す前提は不変・前例 `areka_ghost::boot_with_kanade_stop`）。
11. **`derive_scopes()` は `[0, 1]` 固定**（`emo2_boot/mod.rs`・`ghost-restart-unit` の research が「`ghost-shell-balloon-switch` の課題」として申し送り）。キャラが 1 人や 3 人以上のゴーストでも常に 0 と 1 を返す。実機の相手 R_POST_and_KOMAINU は 2 スコープなので顕在化しない。本仕様では「2 体目が 1 スコープでも落ちない」までを確かめ、3 人以上の窓は α 後（`alpha-release-signoff` の「既知の制限」の候補）。
12. 行数（着手時の実測）: `msg.rs` 852・`emo2_boot/mod.rs` 767・`schedule/mod.rs` 758・`consumer_ledger.rs` 726・`close.rs` 636・`main.rs` 556・`frame.rs` 474・`ghost_session.rs` 454。上限に近いのは `steady.rs` 935・`runtime_tests.rs` 986・`spine.rs` 971・`schedule_tests.rs` 962・`steady_flow_tests.rs` 931＝いずれも行を足さない（切替の相は `schedule/change.rs`、テストは新しい兄弟ファイル）。

**本仕様（分割後）が触るファイル**: 新規 `emo2_boot/change_cue.rs`（`\![change,ghost]` の受け口・前例 `readme_cue.rs`）・kanade `schedule/change.rs`（切替の相・前例 `user_break.rs`）・それぞれのテスト・`emo2_boot/frame_ghost_switch_tests.rs`／既存 `ghost_session.rs`・`ghost_session_restart_tests.rs`・`main.rs`・`app_exit.rs`・`alert.rs`・`boot_resolve.rs`・`emo2_boot/{mod,frame,consumer_ledger,frame_ghost_quit_tests}.rs`・`frame/wiring.rs`・`menu/mod.rs`・`input_events/mod.rs`（`MouseWiring::send_close_request` と同じ口で切替要求を kanade へ）／kanade `msg.rs`・`actor.rs`・`schedule/{mod,events,close,boot}.rs`（`steady.rs` は呼び出し 1 行）・`schedule/user_break.rs`（議題 ⑴ しだい）・テスト支援 `schedule/{steady_test_support,boot_test_support}.rs`／`areka-ghost/src/runtime.rs`（派生関数）／台帳 `shiori.toml`（`OnGhostChanging`／`OnGhostChanged`）・`sakura-script.toml`（`\![change,ghost…]`）と生成物／`doc/COMPAT_ARCHITECTURE.md` §8（裁定が出れば）。**触らない**: `areka-parsers/src/sakura/decode.rs`・`areka-sakura/src/compile.rs`（`ghost-change-name-resolution` へ移った）。

**汎用の通知の入口（棚卸⑰で要件の必達へ格上げ）**: kanade に外からイベントを送る汎用の口は今も無い（`KanadeMsg` 12 変種）。UI 起点のイベントは「`KanadeMsg` の腕 → `Input` の腕 → `steady.rs` の腕 → `events.rs` の組み立て関数 → `ALLOWED_EVENT_IDS`」の 5 段が全部要る。本仕様がこの入口を 1 本作ると、後続の `shell-balloon-switch`・`ghost-install`・`network-update` がそれぞれ 3 タスクずつ軽くなり、`steady.rs`（935 行）と `actor_tests.rs`（970 行）への接触も避けられる（`shell-balloon-switch`・`ghost-install`・`network-update` の再測定がそろって同じ指摘）。**本仕様の要件に「汎用の通知の入口を 1 本作る」を入れる**。

**並走**: `shell-balloon-switch`・`ghost-install`・`network-update`・`alpha-release-signoff` の文書のみ（`.kiro/specs/<名>/` だけ）はソースの共有 0。ただし両者の設計は本仕様が決める `SwitchRequest` の形・`GhostSession` の置き場・汎用の通知の入口を待つ（＝設計は本仕様の main 着地後）。本仕様の完了で `.kiro/specs` 直下が減る瞬間に並走側の検査が偶発で赤になることに注意（並走側は本仕様の着地後に rebase）。

**要件段階の議題（答えで作業が変わるもの・Fable）**
- ⑴ 送り出しの台詞をダブルクリックで止めたとき、切替を**中止**する（正典 `\![change,ghost]`「この場合、バルーンブレーク（通常ダブルクリック）による中止操作も可能な点に注意」＝`--option=raise-event` のとき）か、09-20 の終了の裁定に倣って**進める**か。中止を取ると kanade に「元の定常へ戻る」経路が 1 本増える（`balloon-break` で 0 本にした種類・`take_user_break_quit` の規則を切替の相だけ逆にする）。09-20 の裁定は終了の話で切替には及んでいない＝**正典どおり中止を推す**。`shell-balloon-switch` のシェル切替の中止も同じ答えを継ぐ。
- ⑵ 切替先が起動できないとき、**元のゴーストへ戻す**（それも失敗なら告知して終了）か、**告知して終了**か。見える差は「壊れたゴーストを選んでも元の子が戻って話し続ける」vs「告知の窓が出て areka が終わる」。戻す方を推す。
- ⑶ 初めて起動するゴーストへ切り替えたとき、「はじめまして」（`OnFirstBoot`）と「○○から交代」（`OnGhostChanged`）のどちらを言うか。ukadoc は `OnGhostChanged` → 204 なら `OnBoot` としか言わず、`OnFirstBoot` との関係は決めていない。areka の `schedule/boot.rs` は起動記録が無ければ `OnFirstBoot` → 204 → `OnBoot`。**切替では `OnFirstBoot` を送らない形を推す**（areka の裁量として §8 に 1 行）。
- **⑷ 切替の途中で流れる別れの台詞を「終了」で終わらせないか（既存の裁定との衝突・2026-09-26 追記）**。正典 `OnGhostChanging` は「このイベントにスクリプトが返されなかった（204）場合、続けてOnCloseが発生する」（`ukadoc:list_shiori_event:OnGhostChanging:1`）。ところが完了 `balloon-break` の要件 3.6（裁定 6・2026-09-20）は「終了イベント（`OnClose`）の応答として再生する別れの台詞を、末尾に `\-` が在るのと同じ結果になるものとして扱う」と定め、kanade の `schedule/close.rs` は別れの台詞が最後まで流れても中断されても `Unloading{Quit}` へ進む（close 握手が定常へ戻る経路は 0 本）。その結果の停止通知（原因 `Quit`）は `run_ghost_quit_phase` → `quit_app` へ流れる。**切替のために `OnClose` を通すと、今の規則のままではアプリごと終わる。** さらに正典 `\![change,ghost]` の「`--option=raise-event` のときはバルーンブレークで中止できる」（議題 ⑴）と、同じ裁定の「`\-` を含む台本を中断したら、ただちに終了する」（要件 3.8・`schedule/user_break.rs` の `take_user_break_quit`＝`quit_reserved`）も同じ場所で重なる。`OnGhostChanging` が 204 でなく台本を返した場合も、その台本が `\-` を含めば同じ問題になる。要件では **「終了で終わる」「切替で降ろすだけ」「中止して元の定常へ戻る」の 3 つのどれが先に効くかを表で決め**、`balloon-break` 要件 3.6 と同裁定の適用範囲を「切替の途中の `OnClose` を除く」のように言い直すかを決める（完了 spec は書き換えられないので、上書きは本仕様の要件と `doc/COMPAT_ARCHITECTURE.md` §8 に書く）。利用者から見える差は「切り替えたつもりがアプリごと終わる」「止めたのに切り替わる」を出さないこと。
- **⑸ 切替先の SHIORI が起動に失敗したとき、失敗の告知と終了（`shiori-fault-notice` の経路）を通さずに元へ戻すか（既存の裁定との衝突・2026-09-26 追記）**。`shiori-fault-notice` は要件の範囲外の欄で「切替先のゴーストが起動できない場合」を本仕様へ渡し（同 spec の `requirements.md` の範囲外の欄）、要件 6.5 で「`run_ghost_quit_phase` に停止原因によって終了しない分岐を足さない（切替のときに終了しない分岐は本仕様の持ち場）」と書いた。ところが仕組みの上では、切替先の kanade が失敗で止まると停止通知（原因 `Fault`）が同じ受け口 `KanadeStopRx` に届き、`run_ghost_quit_phase` → `quit_app` → 告知 → 終了コード 1 へ進む（`crates/areka/src/app_exit.rs` の `fault_of`）。**議題 ⑵ で推す「元のゴーストへ戻す」は、この経路より先に切替の分岐が効かなければ実現しない。** 同じ spec の要件討議で開発者が決めた「1 体の失敗はアプリの失敗ではない」（2026-09-24）は戻す側を支える。要件では、切替の途中に届いた `Fault` を「元へ戻す」へ回す条件（切替の目印が立っている間だけ・元の起動も失敗したら告知して終了コード 1）と、そのとき告知を出すか（「○○は起動できませんでした」を出してから元へ戻すか、黙って戻すか）を決める。1 周目の停止通知が必ず 1 件残る件（項目 6）と同じ分岐で捌くことになる。
- **要件の書き方（2026-09-26 追記）**: 本仕様が通る経路には、既存の裁定が少なくとも 4 本かかっている——`balloon-break` の終了の裁定（要件 3.6〜3.8）・`shiori-fault-notice` の Fault の終了コード（要件 3.1・完了 `app-lifetime-separation` 要件 3.8 の Fault に限った上書き）・`balloon-break` の中断の規則（`take_user_break_quit`）・`app-lifetime-separation` の `ExitPolicy::Explicit`。**要件の冒頭に「既存の裁定との衝突表」を置き、どちらが先に効くかを 1 行ずつ決める**（⑷⑸ はその表の最初の 2 行）。
- 設計で決めるもの（議題にしない）: 切替の目印の置き場（推し: 停止通知の原因に「切替」を足す＝残り物の 1 件も同じ規則で捌け、直前の切替時の台本＝`OnGhostChanged` の Ref1 も同じ通知に載せられる）。旧議題 ⑷（`lastinstalled` の受け皿）は `ghost-change-name-resolution` へ移った。正典「該当ゴーストがいなかった場合は無視」は降ろす前に判定する。

## 2026-09-24 棚卸⑯の再測定（main `0b01f654`）

**本仕様はもう 1 本を前に切り出した＝`areka-P0-ghost-restart-unit`（振る舞いを変えない括り出し・M・6〜7 タスク）。** 理由は、下の実測で想定タスクが **19〜22 本**に膨らみ、1 spec 20 本の上限を超える恐れが出たため。切る場所は「kanade に触るかどうか」——前半（`ghost-restart-unit`）は `crates/areka` の bin だけ、後半（本仕様）は kanade の握手が中心で、共有は `emo2_boot/mod.rs` と `frame.rs` に絞れる。順序は **`shiori-fault-notice` → `ghost-restart-unit` → 本仕様 → `shell-balloon-switch`**。分割後の本仕様は **L（13〜15 タスク）**。

**崩れた／変わった前提（main `0b01f654`・サブエージェント再測定）**

1. **「窓 0 で終了」は `app-lifetime-separation` で解決済み。** `WinApp::with_exit_policy(ExitPolicy::Explicit)` で `run()` は `quit_app` の指示でしか戻らない。Approach の段 ①（寿命の分離・検証 1 本を含む）と Constraints の「α 最大の構造変更＝寿命の分離」は**本仕様から丸ごと外れた**。`main.rs` は 958 行 → 774 行。
2. **終了順序の括り出しは未着手のまま。** `fn main` の `app.run()?` の後ろに、loop ticker の Close → `GhostRuntime::shutdown(CloseReason::User{scope:0})` → seriko の join → perf の最終報告が並んでいる。`app-lifetime-separation` も `baseware-root-layout` も関数にしていない。→ **`ghost-restart-unit` が持つ**。
3. **新しく入った一本道: 停止通知を受けると必ず終了へ進む。** `emo2_boot/frame.rs` の `run_ghost_quit_phase` は `KanadeStopped` を 1 件でも受けると無条件に `quit_app(world, ExitOrigin::KanadeStopped(cause))` を呼ぶ。切替で停止通知を流すとそのままプロセスが終わる。ここへ「切替なら終了しない」分岐を足すのが本仕様の必須作業（`app-lifetime-separation` の design の申し送り「切替は `quit_app` を通らない」と一致）。`ExitOrigin`（`crates/areka/src/app_exit.rs`）は `KanadeStopped(KanadeStopCause)`・`Escape`・`Smoke`・`OsClose` の 4 値（`DummyWindow` は `baseware-root-layout` で退役）。 **`ghost-restart-unit` の実装で具体化（2026-09-26）**: 停止通知の受け口 `KanadeStopRx` はプロセスに 1 つ（`ghost_session::register_systems` が 1 回据える）なので、起こし直しで 1 周目を `GhostSession::shutdown` すると 1 周目の kanade が送る `KanadeStopped` が受け口に読まれず残り、次のフレームで `run_ghost_quit_phase` → `quit_app` へ進んでアプリが終わる（`ghost-restart-unit` の 2 周テストはフレームを回さないので見ていない）。切替の分岐はこの残った 1 件も「切替による停止」として捌く必要がある。
4. **全窓を消す部品 `despawn_app_windows`（`app_exit.rs`）は私有**で、`app-lifetime-separation` の要件 3.6 により単独では呼べない。「窓を閉じるが終了しない」操作は別に作る。→ **`ghost-restart-unit` が持つ**。
5. **`wire_*` の「1 回だけ登録する」前提が 8 か所ある**（`wire_emo2_boot`・`menu::wire_menu_with`・`readme::wire_readme`・`input_events::user_break::wire_user_break`・`input_events::choice_drain::wire_choice_drain`・`input_events::balloon::wire_balloon_choice`・`placement::spawn::wire_zorder_pair`・`main.rs` の `open_startup_window`）。うち 3 か所（readme・user_break・choice_drain）はコメントで「`main` から 1 度しか呼ばれない」を前提と明記。ゴーストごとに差し替えが要る窓ごとの状態は 5 つ（`MouseWiring`・`MenuWiring`・choice_drain の kanade 送り口・`PersistWiring`・readme の経路）。→ **登録と状態の分離は `ghost-restart-unit` が持つ**。
6. **`wire_emo2_boot` は `&WinApp` を取り、中で `app.world().borrow_mut()` する。** フレームの系の中（World を借りている最中）から呼ぶと二重借用で落ちる。切替は系の外（`CommandSender` か `spawn_local`）で実行するか、`&mut World` を取る形へ組み替える。→ **組み替えは `ghost-restart-unit`、系の外での実行は本仕様**。
7. **brief の触るファイル一覧に無かったもの 3 件**: ⓐ `\+`／`\_+` は `crates/areka-parsers/src/sakura/decode.rs` の `decode_bare` の既定の腕で `Raw` になり compile の catch-all で捨てられる＝parsers と areka-sakura の compile に触る。ⓑ `OnGhostChanged` → 204 → `OnBoot` は起動系列の変種＝`crates/areka-kanade/src/schedule/boot.rs` に触る。ⓒ 起動元の情報（直前のゴースト名・切替時の台本）を boot へ渡す口＝`GhostBootOptions` は構造体リテラルが 27 か所なので欄を足さず、`boot_with_kanade_stop` 型の派生関数で渡す。
8. **`KanadeStopped` が運ぶのは `cause` だけ。** `OnGhostChanged` の Ref1（直前のゴーストの切替時の台本）を運ぶ器はどこにも無い（`KanadeStopCause` 5 値・`stop_cause_of` は網羅 match のまま＝前提 4 は不変）。
9. **中断の規則が新たに絡む。** 正典「`--option=raise-event` のときはバルーンブレークで中止も可能」が、`schedule/user_break.rs` の `on_user_break`（09-20 の「中断しても終了で終わる」裁定）と交差する（議題 1）。
10. **Ref0 の本体側の名前（`sakura.name`）は目録の素性に無い。** `catalog::Identity` は 7 項目で、`baseware-root-layout` の要件 2.9 が「足さない」と決めた。切替先の `sakura.name` は `catalog::companion_balloon` と同じ形の単独の読み手で読む。
11. 前提 2（`canonical()` 8 行・`(change,ghost)` 無し）・前提 5・`GhostRuntime`（683 行）・kanade 8 ファイルは `fe157df1` から**不変**。

**`app-lifetime-separation`／`baseware-root-layout` が用意済みで仕事が減る部品**: 列挙は `areka_ghost::catalog`（`BasewareRoot`・`list_ghosts`・`list_shells`・`list_balloons`・`companion_balloon`・`GhostEntry`・`Identity`）＝`random`／`sequential` はその上の純関数 1 本。記憶は `PersistKey::{LastGhost(App), LastBalloon(Ghost), LastShell(Ghost)}` と `boot_resolve.rs` の `LastUsed::record`＝新ゴーストの送り口で呼び直すだけ。切替先のバルーン決めは `boot_resolve::resolve_balloon`（7 分岐・純粋）と `read_last_balloon` を流用。起動成功時の後処理 `main.rs` の `on_boot_ok` が繰り返し呼べる単位の芯。告知は `alert.rs` の `AlertScene`・`raise` に場面を 1 つ足すだけ。メニューは `Frame::Ghost` と文言 `ghostrootbutton.caption`＝「ゴースト」が既にあり、登記 0 件（`#[allow(dead_code)]` 3 か所＝`ItemBody::Submenu`・`MenuRegistry::unregister`・`menu::register`）。

**本仕様（分割後）が触るファイル**: `emo2_boot/mod.rs`（`change` の受け口）・`frame.rs`（`run_ghost_quit_phase` の切替分岐）・`frame/wiring.rs`（`Emo2Wiring` の載せ替え）・`consumer_ledger.rs`・`boot_config.rs`／`boot_resolve.rs`（切替先の解決）・`menu/mod.rs`（最初の登記者）・kanade `msg.rs`（771）・`actor.rs`・`schedule/{mod,events,steady（935）,close,boot}.rs`・`schedule/user_break.rs`（議題 1 しだい）・`areka-ghost/src/runtime.rs`（派生関数）・`areka-parsers/src/sakura/decode.rs`＋areka-sakura の compile・`alert.rs`・網羅台帳 `shiori.toml`（`OnGhostChanging`／`OnGhostChanged` は `absent`・owner 空）と `sakura-script.toml`（`\![change,ghost…]`・`\+`・`\_+`）と生成物。**1,000 行の上限**: `steady.rs` 935（相を足すと超える恐れ大＝相は新ファイルへ・前例 `user_break.rs`）・テスト側 `runtime_tests.rs` 986・`schedule_tests.rs` 945・`steady_flow_tests.rs` 931。陳腐化 1 件: `shiori.toml` の `shellrootbutton.caption` の備考が引受先を本仕様と書くが、分割後は `shell-balloon-switch` の担当（`shell-balloon-switch` が直す）。

**要件段階の議題（答えで作業が変わるもの・Fable）**: ⑴ `OnGhostChanging` の台詞をバルーンの中断で止めたら切替を**中止**する（正典）か、09-20 の終了の裁定に倣って**切替へ進む**か——中止を取ると kanade に「元の定常へ戻る」経路が 1 本増える（`balloon-break` で 0 本にした種類）。⑵ 切替の目印をどこに持つか——`KanadeStopCause` に「切替」を足して停止通知で運ぶか、UI 側に切替の予約を持つか。Ref1（台本）の器の置き場所も同時に決まる。⑶ 新ゴーストが起動できないとき、元へ戻す（推し・元の再起動も失敗した場合の着地が要る）か告知して終了か。⑷ `lastinstalled` は `ghost-install` が後着なので本仕様の時点では常に「該当なし → 無視」——受け皿を本仕様で用意するか `ghost-install` へ送るか。**未確認**: `OnGhostChanged` と `OnFirstBoot` の関係（ukadoc の 2 項目からは決まらない・設計で引き直す）／`sequential` の順序は正典に定義が無い（目録の昇順を使う想定・`baseware-root-layout` 裁定 3「列挙の並びは判断に使わない」は起動の解決の話で切替に及ぶかは未確認）／降ろして起こし直す間に UI スレッドが止まる時間は未実測。

## 2026-09-20 棚卸⑮の再測定

**本仕様は 3 本に分かれた。** 想定タスクが 30〜40 本で 1 spec の上限（20 本）を大きく超えたためである。本 brief は名前を変えずに**真ん中の 1 本**として残す——完了 spec 7 本とソースのコメント（`crates/areka/src/menu/mod.rs`）と網羅台帳（`doc/ukadoc-coverage/ledger/shiori.toml`）がこの名前を引受先として指しており、完了 spec は書き換えられないからである。

| 段 | spec | 中身 | 本 brief の対応箇所 |
|---|---|---|---|
| 1 | `areka-P0-app-lifetime-separation` | アプリの寿命を窓の数から切り離す | Approach ①。**本 brief からは外れた** |
| 2 | **本仕様** | 切替要求の入口の型 `SwitchRequest`・台本の `change` の消費者・名前解決（`random`／`sequential`／`lastinstalled`）・**ゴースト切替**（`OnGhostChanging`／`OnGhostChanged`・降ろして起こし直す仕組み）・メニューの「ゴースト」枠への登記 | Approach ④・Desired Outcome 1・4・5・6 |
| 3 | `areka-P0-shell-balloon-switch` | シェル切替・バルーン切替・メニューの「シェル」「バルーン」枠への登記・起動時の「最後のシェル」の適用 | Approach ②③・Desired Outcome 2・3。**本 brief からは外れた** |

順序は 1 → `areka-P0-baseware-root-layout` → 2 → 3。親 brief の「②バルーンが最軽量 → ③ → ④」の段取りは**コードから支持されなかった**（下の「崩れた前提」3）。降ろして起こし直す仕組み（本仕様）が一般形で、シェルとバルーンの切替はその「SHIORI を残す版」になるので、本仕様が先である。分割後の規模は **M〜L（タスク 15〜18 本）**。

**崩れた前提（main `fe157df1` での実測）**

1. **「窓 0 で終了」の所在は `main.rs` ではなく wintf**（`crates/wintf/src/runtime/mod.rs` の `wire_shutdown_hook`）。詳細は `areka-P0-app-lifetime-separation` の brief。
2. **消費者の登記は 5 行ではなく 8 行**（`crates/areka/src/emo2_boot/consumer_ledger.rs` の `canonical()`）。本文が挙げる 5 つに `(open,readme)`・`(enter,nouserbreakmode)`・`(leave,nouserbreakmode)` が加わった。選別子は第 1 引数までしか見ないので、`(change,ghost)`・`(change,shell)`・`(change,balloon)` は別々の登記になる＝本仕様は `(change,ghost)` だけを足せる。
3. **シェルとバルーンは「資産を作り直すだけ」ではない。** 出し先（sink）は起動時に値で渡されて固定され、走っている最中に差し替える語彙が 4 アクターのどこにも無い。詳細は `areka-P0-shell-balloon-switch` の brief。
4. **`KanadeStopped` は「終了」と「切替」を区別しない。** `KanadeStopCause`（`crates/areka-kanade/src/msg.rs`・5 値）に値を足すか、UI 側で切替の予約を持つかを要件で決める。`stop_cause_of`（`crates/areka-kanade/src/actor.rs`）は wildcard を置いていないので、値を足すとコンパイルがここで止まる＝漏れは構造が止める。
5. **終了の経路は `areka-P0-balloon-break`（PR#164）で形が変わった。** 中断のあとの終了を新設の `crates/areka-kanade/src/schedule/user_break.rs` が引き受け、終了の握手が定常へ戻る経路は 0 本になった。本文の「終了」の記述はその前の形である。

**本仕様が触るファイル（実測・確度の高いもの）**: `crates/areka/src/main.rs`（`app.run()` の後ろの終了順序を、繰り返し入れる単位へ括り出す）・`boot_config.rs`・`emo2_boot/mod.rs`（`add_systems` を 2 度呼んでも壊れない形に・`Emo2Wiring` の載せ替え）・`emo2_boot/frame.rs`・`emo2_boot/consumer_ledger.rs`・入力／メニュー／選択肢／永続の結線のやり直し・kanade の `schedule/close.rs` と 5 ファイル（`msg.rs`・`actor.rs`・`schedule/{mod,events,steady}.rs`）・`crates/areka/src/menu/mod.rs`（`#[allow(dead_code)]` 3 か所を外す最初の登記者になる）・網羅台帳と生成物。

**後ろの 3 本（`shell-balloon-switch`・`ghost-install`・`network-update`）を軽くする設計の提案**（確認済みの事実ではない）: イベントを足す spec は毎回 kanade の 5 ファイルに触る。本仕様が**汎用の通知の入口を 1 本**作れば、後続の接触は `schedule/events.rs` の許可表（`ALLOWED_EVENT_IDS`）の数行に縮む。`schedule/steady.rs` は 935 行で上限が近く、相を足した spec が分割を強いられる。

**規模と担当**: 要件と設計は **Fable**。α で最も大きい構造の変更は、分割したあとも本仕様に残っている（繰り返し起動できる形への括り出し）。

## Problem

**誰の何が困っているか**: 2 体目のゴーストを入れた第三者。着せ替えではなく「別のシェル」を持つゴーストの利用者。バルーンを入れ替えたい利用者。

今日の areka は**構造的に単一ゴースト**である。argv で 1 つの `ghost_root` を受け、`GhostRuntime` を 1 つ起こし、窓が全て閉じたら `app.run()` が返って終了する（`crates/areka/src/main.rs:317`・`main.rs:351` の `runtime.shutdown(CloseReason::User)`）。別のゴーストへ替えるには**プロセスを終了して別の argv で起動し直す**しかなく、それは利用者の操作ではない。

切替を指示する正典の語彙は全て未対応である（`doc/ukadoc-coverage/briefing-sakura-script.md:944-946`）。`\![change,ghost,…]`／`\![change,shell,…]`／`\![change,balloon,…]` は汎用キャリア `GenericCommand` として解析は通るが消費者が居ない（`crates/areka/src/emo2_boot/consumer_ledger.rs:239-247` が登記するのは `move`・`bind`・`set,zorder`・`reset,zorder`・`\f` のみ）＝**黙って何も起きない**。イベント `OnGhostChanging`／`OnGhostChanged`／`OnShellChanging`／`OnShellChanged`／`OnBalloonChange` は `crates/` に 0 件。

## Current State

- **起動**: `areka-ghost` の `boot`（`crates/areka-ghost/src/runtime.rs`）が「マウント → SHIORI 起動 → sink 配線 → sylphya 復元」を 1 回行う。マウントは `crates/areka-parsers/src/package/resolve.rs:41-118`（2 点＝SHIORI 側とシェル側）。
- **終了**: 利用者操作は右クリックメニューの「終了」（`crates/areka/src/menu/mod.rs` の `request_close`。**2026-09-19 追記**: 起票時の入口だった結線済みの Ctrl＋左ダブルクリックは `areka-P0-popup-menu-minimal` のタスク 8.2 で除去された・開発者裁定）→ `CloseRequest{User{scope}}` → ゴーストの `OnClose` 握手 → 終了挨拶 → `\-` → 窓を閉じる（`emo2_boot/frame.rs:168-201`・`event = "ghost_quit"`）。**この握手は切替の前半（`OnGhostChanging` → 204 なら `OnClose`）とほぼ同じ形**である（`ukadoc:list_shiori_event:OnGhostChanging:1`「SSPでは、このイベントにスクリプトが返されなかった（204）場合、続けてOnCloseが発生する」）。
- **窓が 0 になるとアプリが終わる**（`main.rs:317`）。切替の間、窓は一度全て消えるので、この「0 で終了」を「切替中は終了しない」へ変える必要がある。これが本仕様で最も大きい構造の変更である。
- **シェル**: マウント時に `shell/<名>` を 1 つ決める（`resolve.rs`）。切替＝マウントの片側（シェル側）だけを差し替えて emo の資産（アトラス・合成・配置）を作り直す。
- **バルーン**: argv 第 2 引数で決まり、`areka-emo-present` の balloon 側が読む。切替＝バルーン側の資産だけを作り直す。

## Desired Outcome

完了時に次が真になっている。

1. **`\![change,ghost,名]` で別のゴーストへ替わる。** プロセスは生き続ける。順序は正典どおり: `OnGhostChanging`（Ref0＝切替先の本体側名・Ref1＝`manual`／`automatic`・Ref2＝ゴースト名・Ref3＝パス）→ 204 なら `OnClose` → 現ゴーストの終了挨拶を再生し切る → SHIORI を降ろす → 新ゴーストを起こす → `OnGhostChanged`（Ref0＝直前のゴーストの本体側名・Ref1＝直前の切替時の台本・Ref2・Ref3・Ref7）→ 204 なら `OnBoot`。名前は `random`／`sequential`／`lastinstalled` を受ける（`lastinstalled` は同一プロセス内でのみ有効）。`--option=raise-event` の有無で `OnGhostChanging` を送るか否かを分ける。
2. **`\![change,shell,名]` で同じゴーストの別シェルへ替わる。** `OnShellChanging`（`--option=raise-event` のときだけ）→ シェル側を差し替え → `OnShellChanged`（Ref0＝現シェル名・Ref1＝ゴースト名・Ref2＝パス）。`menu,hidden` のシェルは名指しでは切り替えられる（メニューに出ないだけ）。
3. **`\![change,balloon,名]` でバルーンが替わり `OnBalloonChange`（Ref0＝名・Ref1＝パス）が届く。**
4. **記憶される。** 切替後の選択は `baseware-root-layout` の「最後に使った」鍵へ書かれ、次回起動で復元される。
5. **該当が無ければ無視される**（正典「該当シェルがなかった場合は無視される」）。ただし `warn!` は出す。
6. **メニューからの切替と台本からの切替は同じ 1 本の経路を通る。** メニュー（`popup-menu-minimal`）は本仕様が公開する `SwitchRequest { Ghost | Shell | Balloon }` を送るだけ。

## Approach

**選んだ形**: 「終了の握手」を再利用して「切替の握手」を作り、アプリの寿命を窓の数から切り離す。

| 段 | 中身 | 検証 |
|---|---|---|
| ① 寿命の分離 | `main.rs:317` の「窓 0 で `app.run()` が返る」を「`AppExit` の指示で返る」へ。終了操作は `AppExit` を送り、切替は送らない。窓が 0 の瞬間があっても落ちない | 既存の終了経路の決定論テストが全て緑のまま（配線の再テストはしない・記憶 test-only-decision-branches-not-proven-wiring）＋「切替中に窓 0 でも生きている」1 本 |
| ② バルーン切替 | `SwitchRequest::Balloon(名)` → 列挙で解決 → present のバルーン資産を作り直し → `OnBalloonChange` → 記憶 | 決定論: 偽のバルーン 2 つで往復し、イベントの Ref を突き合わせる |
| ③ シェル切替 | `SwitchRequest::Shell(名, raise)` → `OnShellChanging`（raise 時）→ マウントのシェル側差し替え → emo 資産の作り直し → `OnShellChanged` → 記憶 | 決定論: `R_POST_and_KOMAINU` に 2 つ目のシェルを持つ fixture（`shell/master` を写して名前を変えたもの）で往復 |
| ④ ゴースト切替 | `SwitchRequest::Ghost(名, raise)` → `OnGhostChanging`（raise 時・204 で `OnClose`）→ 終了挨拶の再生完了を待つ（既存の `ghost_quit` の合図）→ `runtime.shutdown` → 新 `boot` → `OnGhostChanged`（204 で `OnBoot`）→ 記憶 | 決定論: 偽の SHIORI 2 体（`shiori-host32-testdll`／in-proc の偽境界）で往復し、イベント列と Ref を突き合わせる。実機: emo2 ⇄ R_POST_and_KOMAINU |

段 ① を先に置く理由: 切替の 3 種はどれも「窓が一度消える」瞬間を持つ。寿命を先に分離しておけば、②〜④ は「資産を作り直す」だけの仕事になる。

**取らない形**: プロセスの再起動（新しい argv で自分を起動し直す）。`OnGhostChanged` の Ref1（直前のゴーストの切替時の台本）を渡せず、終了挨拶の再生完了を待つ握手も切れる。「終了経路は正規実装・小細工禁止」（記憶 canonical-not-minimal-lifecycle）に反する。

## Scope

- **In**:
  - `\![change,ghost|shell|balloon,名(,--option=raise-event)]` の消費者（汎用キャリアの name 選別・typed 新設はしない）
  - `\+`／`\_+`（ランダム／シーケンシャル切替・`OnGhostChanging` を送らない）
  - `OnGhostChanging`／`OnGhostChanged`／`OnShellChanging`／`OnShellChanged`／`OnBalloonChange` の送出と Ref
  - `random`／`sequential`／`lastinstalled` の名前解決
  - アプリ寿命の分離（`AppExit`）
  - 切替後の記憶（`baseware-root-layout` の鍵へ）
  - `SwitchRequest` の公開（メニューの入口）
- **Out**:
  - `\![reload,ghost|shell|balloon]`（読み直し・α 後。切替経路の上に S で乗る）
  - `\![bind,…]` 着せ替え（完了仕様 `mayuna-compose`／`bindoption-exclusivity` の所有）
  - `\![set,scaling,…]`／`OnShellScaling`／`OnBalloonScaling`（束「切替」に居るが拡大率の話・α 後）
  - `OnNotifySelfInfo`／`OnNotifyShellInfo`／`OnNotifyBalloonInfo`／`OnNotifyDressupInfo`（通知系・α 後の `property-*` 群と同時に）
  - `currentghost.shelllist.*` プロパティ（`currentghost-property-tree`・α 後）
  - 多重ゴースト（`OnOtherGhost*`）・ゴースト呼び出し（`OnGhostCall*`）
  - 消滅（`\![vanishbymyself]`・`OnVanish*`・α 後の裁定候補）

## Boundary Candidates

- **アプリ寿命**（`main.rs` の `app.run()` の終了条件）
- **切替の握手**（kanade の schedule に `OnGhostChanging → OnClose` の相を足す。既存の close 握手の隣）
- **資産の作り直し**（emo 側: シェルとバルーンの再ロード。`present-gpu-transform-scale` 完了後の atlas/compose/present の直列 3 分割の上に乗る）
- **名前解決**（`random`／`sequential`／`lastinstalled`＝`baseware-root-layout` の列挙の上の純関数）

## Out of Boundary

- 列挙と記憶の実体（`baseware-root-layout`）
- メニュー項目の表示（`popup-menu-minimal`）
- インストール直後の自動切替（`ghost-install` が `lastinstalled` を使って本仕様を呼ぶ）

## Upstream / Downstream

- **Upstream**: `areka-P0-baseware-root-layout`（列挙・記憶）／`areka-P0-nar-install`（2 体目の検体 `R_POST_and_KOMAINU` の根）／完了仕様 `areka-P0-host32-window-thread-pump`（SHIORI アクターの寿命と死活監視＝降ろして起こし直す前提）／完了仕様 `areka-P0-kanade-boot-talkdone-drop`（起動系列の途中の通知の扱い＝新ゴースト起動時に再利用）。
- **Downstream**: `popup-menu-minimal`（メニュー項目 → `SwitchRequest`）・`ghost-install`（インストール後に `lastinstalled` へ切替）・`network-update`（更新後の `\![reload,…]` は α 後だが、更新後の再起動相当に本仕様の切替を使える）・`alpha-release-signoff`。

## Existing Spec Touchpoints

- **Extends**: なし（新規）。
- **Adjacent**:
  - `areka-P0-status-execution-states`（α 後・`emo2_boot/mod.rs`）——切替中の `status` 値（`balloon`／`minimizing` 等）。本仕様は値を出さない。
  - `areka-P0-property-query-channels`／`property-ipc-transport`（α 後・`areka-ghost` の `prop_sink.rs`／`runtime.rs`）——**`runtime.rs` を本仕様も触る**。α 後なので順序で解決するが、本仕様の変更は `boot`／`shutdown` の寿命に限り、sink の形は変えない。
  - `areka-P0-translate-pipeline`（α 後・`kanade/{actor,msg,schedule}`）——kanade の schedule に本仕様が相を足す。後着が rebase。
  - `areka-P0-zorder-chain-residue`（W14・`spine_*_tests.rs`）——切替で窓を作り直すと z 順の鎖も作り直る。A 群の間欠赤が本仕様のテストに混ざらないよう、A 群を先に着地させるか、本仕様のテストは鎖を数えない。

## Constraints

- **α の最大の構造変更**＝アプリ寿命の分離。`main.rs` と `areka-ghost/src/runtime.rs` に触る。単独か、共有ファイル 0 を実測した相手とだけ並走する。
- 正典の Ref 番号を落とさない（`OnGhostChanged` の Ref7＝切替先のシェル名 等）。SSP のみの Ref も送る（記憶 no-ssp-measurement-import-semantics-from-ukadoc＝意味論は ukadoc から）。
- 切替中の失敗（新ゴーストが起動できない）は `error!`＋**元のゴーストへ戻す**か**「ゴーストが無い」告知で終了**のどちらかへ必ず着地する（ログ無し失敗経路の禁止）。戻す方を推す。
- 決定論テスト網羅は必達。偽の SHIORI 2 体はテスト DLL ではなく偽境界（記憶 prefer-x64-fake-boundary-tests-not-x86）。実機は emo2 ⇄ R_POST_and_KOMAINU の往復を 1 周（記憶 areka-real-machine-signoff-bounded-auto-exit）。
- 1 ファイル 1,000 行。`runtime.rs`・`main.rs`・`kanade/schedule/*` の現在行数を着手時に測る。
- 規模 **L**。要件段階で ①②（寿命＋バルーン）を先行スライスにできる。

- **2026-09-18 `nar-install` 設計からの申し送り（使用中の宛先）**: `areka-nar` の展開は「作業フォルダに組んでから宛先と入れ替える」形で、宛先の中のファイル（起動中のゴーストの `shiori.dll` 等）が開かれていると入れ替えが失敗し、宛先は無傷のまま `NarError::Io { phase: Commit, rolled_back: true }` が返る。**エンジンは SHIORI の解放を試みない**。起動中のゴーストへ入れる・更新する・切り替える経路は、呼び出し側が先に SHIORI をアンロード（`OnClose` 相当の終了経路）してから `install` を呼ぶこと。
