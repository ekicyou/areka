# Requirements Document

> 本文の実測は **2026-09-26・本ブランチ**（main `13b72893`＝棚卸⑰の再測定と同じ土台。その後の main はすべて文書のコミットで、ソースは不変）のもの。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。
> 要件 11 の裁定 1〜9 は brief の議題 ⑴〜⑸ と、要件を書く途中で答えが要った 4 点に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。

## Project Description (Input)

**誰の何が困っているか**: 2 体目のゴーストを入れた第三者（α の利用者）。今日の areka は 1 プロセスに 1 体だけを起こし、別のゴーストへ替えるには**プロセスを終了して別の引数で起動し直す**しかない。それは利用者の操作ではない。

**今の状態**: `\![change,ghost,名]` は解析は通るが消費者が居らず、**黙って何も起きない**。`OnGhostChanging`／`OnGhostChanged` は製品コードに 0 件。同じプロセスでゴーストを降ろして起こし直す**形**（`ghost_session.rs`＝登録 1 回・載せ替え n 回・「全窓を閉じるが終了しない」操作）は完了 `areka-P0-ghost-restart-unit` が用意したが、本番の呼び手は 0 で、停止通知を受けると必ず終了へ進む一本道（`run_ghost_quit_phase` → `quit_app`）はそのまま残っている。

**何を変えるか**: 名指しの `\![change,ghost,名(,--option=raise-event)]` と右クリックメニューの「ゴースト」枠から、プロセスを生かしたまま別のゴーストへ替わる。順序は正典どおり（`OnGhostChanging` → 204 なら `OnClose` → 別れの台詞を再生し切る → 降ろす → 起こす → `OnGhostChanged` → 204 なら `OnBoot`）。切替先が起動できなければ既定ゴースト（emo2）へ戻す。切替の間に窓が 0 になっても終了しない。あわせて、kanade に外から SHIORI イベントを送る**汎用の通知の入口を 1 本**作る（後続 `shell-balloon-switch`・`ghost-install`・`network-update` が使う）。名前解決（`random`／`sequential`／`lastinstalled`・`\+`／`\_+`）は `areka-P0-ghost-change-name-resolution`、シェルとバルーンの切替は `areka-P0-shell-balloon-switch` が持つ。

> 起票: 2026-09-18 `/kiro-discovery` 再入（棚卸⑭）。2026-09-20（棚卸⑮）に 3 分割の真ん中として存続、2026-09-24（棚卸⑯）に `ghost-restart-unit` を、2026-09-26（棚卸⑰）に `ghost-change-name-resolution` を前後へ切り出した。brief の各節の file:line は起票時の実測で、要件生成時（2026-09-26）に「何の定義か」で引き直した。

## Introduction

### 誰が困っているか

α の利用者（第三者）。`.nar` を 2 つ入れても、**2 体目に替える操作が無い**。ゴースト側の台本（里々の定石 `\![change,ghost,…]`）も、右クリックメニューの「ゴースト」枠も、今日は何も起こさない。

### いま何が起きているか（2026-09-26 実測）

- **切替の語彙に消費者が居ない。** `crates/areka/src/emo2_boot/consumer_ledger.rs` の `canonical()` が登記する汎用命令は 8 組（`move`・`bind`・`set,zorder`・`reset,zorder`・`open,readme`・`enter,nouserbreakmode`・`leave,nouserbreakmode`・`\f`）で、`(change, …)` は 0 組。`OnGhostChang` は `crates/*.rs` で `crates/ukadoc-survey/src/diff_tests.rs` の 1 件だけ（製品コード 0 件）。網羅台帳 `doc/ukadoc-coverage/ledger/shiori.toml` の `OnGhostChanging`／`OnGhostChanged` と `sakura-script.toml` の `\![change,ghost,…]` はいずれも `absent`・owner 空。
- **降ろして起こし直す形はあるが、呼び手が 0。** `crates/areka/src/ghost_session.rs` の `register_systems`（プロセスに 1 回）・`boot_ghost`（n 回）・`GhostSession::shutdown(self, CloseReason)`（ticker 停止 → ゴースト実行系の終了 → seriko の join）・`reopen_ghost_windows(world, cfg, closed)` と、`crates/areka/src/app_exit.rs` の `close_windows_for_restart(world) -> WindowsClosed` は揃っている。後の 2 つには `#[cfg_attr(not(test), allow(dead_code))]` が付き、コメントが「本番の呼び手はゴースト切替」と書く。
- **停止通知を受けると必ず終了へ進む。** `crates/areka/src/emo2_boot/frame.rs` の `run_ghost_quit_phase` は受け口 `KanadeStopRx`（プロセスに 1 つ・`emo2_boot::wire_kanade_stop` が据える）に届いた停止通知を全件取り出し、原因を問わず `quit_app(world, ExitOrigin::KanadeStopped(cause))` を呼ぶ。原因 `KanadeStopCause`（`crates/areka-kanade/src/msg.rs`）は `Quit`・`Forced`・`CloseSilent`・`DeadlineExceeded`・`Fault(ShioriFault)` の 5 値で、**終了と切替を見分ける値は無い**。`crates/areka-kanade/src/actor.rs` の `stop_cause_of` と `app_exit.rs` の `fault_of` はいずれも網羅の match（値を足せばコンパイルが漏れを止める）。
- **降ろした側の停止通知は必ず 1 件来る。** kanade は終了系列の最後（`Action::StopSelf`）で `notify_stop` を 1 回呼ぶ。`GhostSession::shutdown` が送る `ForceQuit` でも、正典どおり `OnClose` を通した終了でも、原因が違うだけで 1 件届く（例外は `KanadeMsg::Close` と送り手の全喪失＝通知なしに止まる。本仕様はどちらも使わない）。同じプロセスで 2 度起こす完了 `ghost-restart-unit` のテスト `boots_twice_in_one_process_without_double_registration` はフレームを回さずこの 1 件を自分で読み捨てているので、「次のフレームで終了してしまう」症状は本番でしか出ない。
- **`GhostSession` は World の外に居る。** `fn main`（`crates/areka/src/main.rs`）のローカル変数で、`finish_after_run(run, fault, cleanup)` の後始末の閉包へ渡される。フレームの系の中から降ろして起こし直すには、`GhostSession` を World に置くか `main` 側に口を出す必要がある（本仕様に残った唯一の構造の変更）。seriko の join は UI スレッドを塞ぐ。
- **kanade に外からイベントを送る汎用の口は無い。** `KanadeMsg` は 12 変種（`Boot`・`Tick`・`TalkDone`・`CloseRequest`・`ForceQuit`・`ShioriDown`・`Mouse`・`Close`・`Choice`・`ChoiceWaiting`・`UserBreak`・`ResourceQuery`）で、名前と Reference 列で SHIORI イベントを頼む変種は無い。送れるイベントの許可表 `ALLOWED_EVENT_IDS`（`crates/areka-kanade/src/schedule/events.rs`）は 11 語（`OnInitialize`・`OnFirstBoot`・`OnBoot`・`basewareversion`・`OnSecondChange`・`OnClose`・`OnMouseMove`・`OnMouseDoubleClick`・`OnChoiceSelectEx`・`OnChoiceSelect`・`OnChoiceTimeout`）で、`OnGhostChanging`／`OnGhostChanged` は無い。UI 起点のイベントを 1 つ足すには `KanadeMsg` の腕 → `Input` の腕 → `schedule/steady.rs`（935 行）の腕 → `events.rs` の組み立て関数 → 許可表、の 5 段が要る。
- **終了の握手は定常へ戻らない。** `crates/areka-kanade/src/schedule/close.rs` の別れの台詞の待ちは、最後まで流れても中断されても `Unloading{Quit}` へ、204 なら `Unloading{CloseSilent}`、期限切れなら `Unloading{DeadlineExceeded}` へ進む（定常へ戻る経路 0 本＝完了 `balloon-break` 裁定 6）。`schedule/user_break.rs` の `take_user_break_quit` は、利用者の中断で終わった台本が `\-` を予約していたときだけ終了へ結ぶ。
- **起動系列は起動記録で分岐する。** `crates/areka-kanade/src/schedule/boot.rs` は `KanadeConfig.first_boot` が真なら `OnFirstBoot`（204 なら `OnBoot`）、偽なら `OnBoot` へ。`first_boot` は `crates/areka-ghost/src/runtime.rs` の `apply_boot_record_gate` が記憶の鍵 `areka.boot.count` の有無で決める。
- **記憶と目録。** `crates/areka/src/boot_resolve.rs` の `LastUsed::record` は経路が `GhostRoute::Argv` のときゴーストの記憶を書かない（`GhostRoute` は `Argv`・`Memory`・`Only`・`Default`・`Random`）。目録は `crates/areka-ghost/src/catalog.rs`（`list_ghosts`・`GhostEntry`・`Identity`＝`folder`・`name` など 7 項目。`sakura.name` は持たない＝完了 `baseware-root-layout` 要件 2.9）。
- **窓の数。** `crates/areka/src/emo2_boot/mod.rs` の `derive_scopes()` は `[0, 1]` 固定（キャラが 1 人でも 3 人以上でも 0 と 1）。
- **告知とメニュー。** `crates/areka/src/alert.rs` の `AlertScene` は 5 場面（`RootMissing`・`GhostMissing`・`BalloonMissing`・`StartupWindow`・`ShioriFault`）で、`raise` は押されたボタンを返さない。`crates/areka/src/menu/mod.rs` には `Frame::Ghost` と文言 `ghostrootbutton.caption` が在り、登記 0 件（`#[allow(dead_code)]` が `ItemBody::Submenu`・`MenuRegistry::unregister`・`menu::register` の 3 か所）。`MenuRegistry` の説明は「ゴーストを起こすたびに登記をやり直す」契約を書く。
- **行数。** kanade `msg.rs` 852・`schedule/mod.rs` 758・`close.rs` 636・`steady.rs` 935・`actor_tests.rs` 970／areka `emo2_boot/mod.rs` 767・`consumer_ledger.rs` 726・`main.rs` 556・`frame.rs` 474・`ghost_session.rs` 454。

### 正典（ukadoc）の位置づけ

| 正典 | 逐語引用 | 本仕様への含意 |
|---|---|---|
| [`\![change,ghost,ゴースト名(,--option=raise-event)]`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bchange_2cghost_2c_30b4_30fc_30b9_30c8_540d_28_2c--option_3draise-event_29_5d:1) | 「そのゴーストへの切り替えを行う。」「実行後、該当ゴーストがいなかった場合は無視される。」「ゴースト名の後に--option=raise-eventとすると、メニューから切り替え操作をした場合と同じくOnGhostChangingが通知される。この場合、バルーンブレーク（通常ダブルクリック）による中止操作も可能な点に注意。指定しない場合は通知されない。」 | 名指しの切替・該当なしは無視・`raise-event` の有無で `OnGhostChanging` を送るか否か・`raise-event` のときは中断で中止できる。`random`／`sequential`／`lastinstalled` の行は `ghost-change-name-resolution` が引き受ける。 |
| [`OnGhostChanging`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnGhostChanging:1) | 「他のゴーストへの切り替え指示があった際に発生。SSPでは、このイベントにスクリプトが返されなかった（204）場合、続けてOnCloseが発生する。」Reference0「切り替わるゴーストの本体側の名前。」Reference1「手動で切り替えた場合、manual システムにより切り替えられた場合、automaticが返される。」Reference2「切り替わるゴーストの名前。[SSPのみ]」Reference3「切り替わるゴーストのパス。[SSPのみ]」 | 送り出しの握手の形と Ref0〜3。SSP のみの Ref も送る（記憶 no-ssp-measurement-import-semantics-from-ukadoc）。 |
| [`OnGhostChanged`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnGhostChanged:1) | 「他のゴーストから自ゴーストに切り替えられた際に発生。SSPでは、このイベントにスクリプトが返されなかった（204）場合、続けてOnBootが発生する。」Reference0「直前のゴーストの本体側の名前。」Reference1「直前のゴーストの切り替え時のスクリプト。」Reference2「直前のゴーストの名前。[SSPのみ]」Reference3「直前のゴーストのパス（ファイルの場所）。[SSPのみ]」Reference7「切り替わったゴーストのシェル名。[SSPのみ]」 | 迎え入れの握手の形と Ref0〜3・7。`OnFirstBoot` との優先順は書いていない（下の「起動の根の木」と要件 11 裁定 3）。 |
| [`OnBoot`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnBoot:1) | 「起動した際に発生。SSPでは、OnGhostChanged、OnGhostCalled、OnFirstBoot、OnVanishedに対してスクリプトが返されなかった（204）場合、続けてこのイベントが発生する。」 | 起動系列は「理由の根 1 つ → 204 → `OnBoot`」の 1 形（下の木）。根どうしの間に落ちる辺は無い＝切替で起きたゴーストに送る根は 1 つ。Reference6「※MATERIA、SSPのみ　前回の処理中で落ちた時にhalt。」Reference7「※MATERIA、SSPのみ　前回の処理中で落ちたゴースト名 。」＝切替先が起動の途中で落ちて既定ゴーストへ戻したことを既定ゴーストへ伝える器（要件 6.2・裁定 10）。 |

### 起動と終了の根の木（ukadoc の「204 なら続けて」の逐語から）

```
起動の根（起動の理由で 1 つだけ選ぶ）                       204 →
  OnFirstBoot   「初回起動した際に発生」                       → OnBoot   本仕様: 起動記録が無ければ最優先（裁定 3）
  OnGhostChanged「他のゴーストから自ゴーストに切り替えられた」 → OnBoot   本仕様: 起動記録があるときの切替（要件 4.1）
  OnGhostCalled 「他のゴーストから呼び出された」               → OnBoot   範囲外（多重ゴースト）
  OnVanished    「直前のゴーストの消滅により切り替わった」     → OnBoot   範囲外（消滅）
  OnBoot        共通の葉（根が無いときはこれだけ）
終了の根                                                     204 →
  OnGhostChanging「他のゴーストへの切り替え指示」              → OnClose  本仕様（要件 2）
  OnCloseAll     「SSP 自体が終了する際」                      → OnClose  範囲外（areka は今日 OnClose だけ）
  OnClose        共通の葉
```

根どうし（`OnFirstBoot` と `OnGhostChanged` など）の間に 204 で落ちる辺は無い。したがって切替で起きたゴーストにも根は 1 つだけ送り、複数の根の条件が同時に真のとき（起動記録が無いゴーストへ切り替えた）は**初回起動を最優先**にする（2026-09-26 開発者裁定・里々 wiki「初回起動時はそれが交替であれ呼出であれ OnGhostCalled/OnGhostChanged は発生しない」と一致）。設計はこの木を「根の表＋共通の葉」の 1 か所で持ち、範囲外の根は行を足すだけで入る形にする。 |

正典が沈黙している点（`raise-event` 無しのときに `OnClose` を送るか・切替先が起動できないときの振る舞い・初めて起動するゴーストへの切替で `OnFirstBoot` と `OnGhostChanged` のどちらの根を選ぶか〔ukadoc 本体は沈黙・里々 wiki が初回優先と明記〕・`OnGhostChanged` の Ref1 に何を載せるか・名前をどの項目で引くか・自分自身への切替）は areka の裁量として要件 11 で決め、`doc/COMPAT_ARCHITECTURE.md` §8 に記す。SSP の挙動を実測して合わせることはしない。

### 既存の裁定との衝突表（どちらが先に効くか）

本仕様が通る経路には既存の裁定が少なくとも 4 本かかっている。**切替の途中であることを示す目印が立っている間**は、下の表の右の列が効く。完了 spec の文書は書き換えられないので、上書きは本仕様の要件と `doc/COMPAT_ARCHITECTURE.md` §8 に書く。

| # | 既存の裁定 | 本仕様の経路で重なる場面 | 先に効くもの（本仕様） |
|---|---|---|---|
| 1 | 完了 `balloon-break` 要件 3.6（裁定 6・2026-09-20）「`OnClose` の別れの台詞は `\-` の有無によらず必ず終了で終わる」＝`close.rs` の握手は定常へ戻らず、停止通知（`Quit`）が `run_ghost_quit_phase` → `quit_app` へ流れる | 切替で `OnGhostChanging` が 204 のとき、正典どおり `OnClose` を通す | **「切替で降ろすだけ」が「終了で終わる」に勝つ。** 切替の途中の `OnClose` と、その別れの台詞の終わり（最後まで・中断・期限切れ）は、アプリの終了ではなくゴーストを降ろすことで終わる。`quit_app` は 0 回。要件 3.6 の適用範囲を「切替の途中の `OnClose` を除く」と §8 に記す（要件 2.4・3.3・11.4） |
| 2 | 完了 `balloon-break` 要件 3.8（同裁定）「`\-` を含む台本を利用者が中断したら、ただちに終了する」＝`user_break.rs` の `take_user_break_quit`（`quit_reserved`） | `raise-event` 付きの切替で `OnGhostChanging` の台詞（または 204 のあとの `OnClose` の台詞）を利用者がダブルクリックで止める。正典は「中止操作も可能」 | **「中止して元の定常へ戻る」が「終了で終わる」にも「切替で降ろす」にも勝つ。** 切替の相の中断は切替の中止であり、台詞が `\-` を含んでいても終了へは結ばない。§8 に「3.8 の `\-` の予約は切替の相では終了に結ばない」と記す（要件 5・11.1） |
| 3 | 完了 `shiori-fault-notice` 要件 3.1「SHIORI の失敗（Fault）で終わるときは告知して終了コード 1」（完了 `app-lifetime-separation` 要件 3.8 の Fault に限った上書き）と、同 6.5「`run_ghost_quit_phase` に停止原因によって終了しない分岐を足さない（切替のときに終了しない分岐は本仕様の持ち場）」。同 spec の要件討議の開発者裁定「1 体の失敗はアプリの失敗ではない」（2026-09-24） | 切替先の SHIORI が起動に失敗し、停止通知（`Fault`）が同じ受け口 `KanadeStopRx` に届く | **「既定ゴーストへ戻す」が「告知して終了」に勝つ（切替の目印が立っている間だけ）。** 既定ゴースト自身の失敗は致命＝3.1 のとおり即時終了（終了コード 1）。6.5 は衝突ではなく引き継ぎ（分岐を足すのは本仕様）。§8 に 3.1 の限定を記す（要件 6・11.2） |
| 4 | 完了 `app-lifetime-separation`（`ExitPolicy::Explicit`・`quit_app` だけが終了の指示・要件 3.6「窓を消す部品を単独で呼ぶ形を残さない」）と完了 `ghost-restart-unit` 要件 4（「全窓を閉じるが終了しない」操作は起こし直す呼び手にだけ公開） | 切替の間に全ゴースト窓が 0 になる瞬間 | **衝突なし・そのまま。** 切替は `ghost-restart-unit` が用意した操作の最初で唯一の本番の呼び手になり、`AppExit` を立てない（要件 3.2・3.7）。7 種の終了操作と終了コード 0 も不変（要件 8.1・8.2） |
| 5 | 完了 `ghost-restart-unit` 要件 5.7「`GhostBootOptions` に欄を足さない（起動元の情報を渡す口は派生関数）」・完了 `baseware-root-layout` 要件 2.9「目録の素性に `sakura.name` を足さない」・同裁定 3「列挙の並びは判断に使わない」 | 切替先の本体側の名前（Ref0）と直前のゴーストの情報を起動系列へ渡す・メニューの一覧の並び | **継ぐ。** `sakura.name` は単独の読み手で読む。並びは表示にだけ使う（要件 4.7・8.6・8.7） |

### 何を変えるか

**プロセスを生かしたまま別のゴーストへ替わる。** 台本の `\![change,ghost,名(,--option=raise-event)]` とメニューの「ゴースト」枠が同じ 1 本の切替要求を出し、kanade が送り出しの握手（`OnGhostChanging` → 204 なら `OnClose` → 別れの台詞を再生し切る）を行い、areka がゴーストを降ろして（全窓を閉じ・SHIORI を降ろし・終了しない）新しいゴーストを起こし、kanade が迎え入れの握手（`OnGhostChanged` → 204 なら `OnBoot`）を行う。降ろした側の停止通知は「切替による停止」として捌き、`quit_app` へ流さない。`raise-event` 付きの切替は別れの台詞のダブルクリックで中止でき、元のゴーストの定常へ戻る。切替先が起動できなければ既定ゴースト（emo2）へ戻し、既定ゴースト自身が失敗したら致命として即時終了（終了コード 1）。切替後の選択は記憶に書かれ、次回起動で復元される。kanade には外から SHIORI イベントを名前と Reference 列で頼める汎用の入口を 1 本作る。判断の分岐は偽の SHIORI 2 体で決定論テストに固定し、実機で emo2 ⇄ R_POST_and_KOMAINU を往復する。

## Boundary Context

- **In scope**:
  - 切替要求の入口 1 本（`SwitchRequest` のゴーストの腕）と、その 2 つの出どころ＝台本の `\![change,ghost,名(,--option=raise-event)]`（名指しだけ）・右クリックメニューの「ゴースト」枠（目録のゴーストの一覧）。
  - 送り出しの握手（`OnGhostChanging`・Ref0〜3 → 204 なら `OnClose` → 別れの台詞の再生完了）と、`raise-event` の有無による分岐。
  - 降ろして起こし直すこと（プロセスは生きる・窓 0 の瞬間でも終了しない・降ろした側の停止通知を切替として捌く・`GhostSession` の置き場の構造変更）。
  - 迎え入れの握手（`OnGhostChanged`・Ref0〜3・7 → 204 なら `OnBoot`）と、切替後の記憶・バルーンの決め方・メニューの登記のやり直し。
  - `raise-event` のときのバルーンブレークによる中止（元の定常へ戻る）。
  - 切替先が起動できないときに既定ゴーストへ戻すこと（既定ゴースト自身の失敗は致命＝即時終了・告知の場面は足さない）。既定ゴースト emo2 の `halt` の台詞（`OnBoot` の Ref6/7 を受けて話す辞書）は本仕様の外＝開発者が後で足す。
  - 該当するゴーストが無いときの無視（`warn!`）。解決できない名前（`random`／`sequential`／`lastinstalled`）も同じ腕で受ける（`ghost-change-name-resolution` がその腕を置き換える）。
  - **kanade に外から SHIORI イベントを送る汎用の通知の入口 1 本**（許可表で絞る）。
  - 網羅台帳（`shiori.toml` の `OnGhostChanging`／`OnGhostChanged`・`sakura-script.toml` の `\![change,ghost,…]`）と生成物、`doc/COMPAT_ARCHITECTURE.md` §8 の裁量の記録。
  - 決定論テスト（偽の SHIORI 2 体の往復）と実機サインオフ（emo2 ⇄ R_POST_and_KOMAINU）。
- **Out of scope**:
  - 名前解決 `random`／`sequential`／`lastinstalled` と `\+`／`\_+`（`areka-P0-ghost-change-name-resolution`）。`lastinstalled` の受け皿も同 spec。
  - シェル切替・バルーン切替（`\![change,shell|balloon,…]`・`OnShellChanging`／`OnShellChanged`／`OnBalloonChange`・メニューの「シェル」「バルーン」枠・起動時の最後のシェルの適用＝`areka-P0-shell-balloon-switch`）。本仕様は `(change,shell)`・`(change,balloon)` の消費者を 0 組のまま残す（今日どおり黙って何も起きない）。
  - `\![reload,ghost|shell|balloon]`（α 後。同じゴーストへの切替が代用になる＝要件 1.8）・`\![call,ghost,…]`・多重ゴースト（`OnOtherGhost*`・`OnGhostCall*`）・消滅（`\![vanishbymyself]`・`OnVanish*`）・`\![set,scaling,…]`・`OnNotify*Info`・`currentghost.*` プロパティ。
  - インストール直後の自動切替（`ghost-install` が `lastinstalled` で本仕様の入口を呼ぶ）・ネットワーク更新後の読み直し（`network-update`）。
  - 3 人以上のキャラを持つゴーストの窓（`derive_scopes()` の `[0, 1]` 固定は据え置き。`alpha-release-signoff` の「既知の制限」候補として申し送る）。
  - 切替中の `status` 値（`status-execution-states`・α 後）。
  - 告知に押されたボタンを返す形（`ghost-install`）。
- **Adjacent expectations**:
  - 完了 `ghost-restart-unit` の形（`register_systems` 1 回・`boot_ghost` n 回・`GhostSession::shutdown`・`close_windows_for_restart` → `reopen_ghost_windows`）の上に建ち、本仕様がその最初で唯一の本番の呼び手になる（`#[cfg_attr(not(test), allow(dead_code))]` を外す）。
  - 完了 `shiori-fault-notice` の告知の部品（`AlertScene`・場面ごとの題名）と停止通知の中身（`Fault(ShioriFault)`）を使う。`run_ghost_quit_phase` に切替の分岐を足すのは本仕様（同 spec 要件 6.5 の引き継ぎ）。
  - 完了 `baseware-root-layout` の目録（`catalog::list_ghosts`）・記憶の鍵（`LastGhost`＝App・`LastBalloon`／`LastShell`＝Ghost スコープ）・起動解決（`resolve_balloon`）・`on_boot_ok` を使う。
  - 完了 `popup-menu-minimal` の OS ネイティブメニューと `Frame::Ghost` に登記する（メニューの見た目や操作の作法は変えない）。
  - `ghost-change-name-resolution`（並走・共有ソース 0）は本仕様が作る「解決できない名前」の腕（UI 側の取り出し＝要件 1.7。talk スレッドの受け口ではない）を置き換える。`shell-balloon-switch`・`ghost-install`・`network-update` は本仕様の `SwitchRequest` の形・汎用の通知の入口・`GhostSession` の置き場を見てから設計する（要件までは並走）。本仕様の完了で `.kiro/specs` 直下が減る瞬間、並走側の ukadoc-survey の検査が偶発で赤になりうる（並走側は本仕様の着地後に rebase）。
  - 完了 `areka-P0-nar-install` の申し送り: 起動中のゴーストのフォルダに入れ替えをかける経路は、先に SHIORI を降ろしてから行う（本仕様は降ろす口を提供するだけで、インストールの順序は `ghost-install` が決める）。

## Requirements

### Requirement 1: 切替要求の入口は 1 本で、台本とメニューが同じ経路を通る

**Objective:** As a α の利用者, I want ゴーストの台本からも右クリックメニューからも別のゴーストへ替えられること, so that 2 体目を入れたあと、プロセスを起動し直さずに替えられる

#### Acceptance Criteria

1. The areka shall 切替要求の入口（`SwitchRequest` のゴーストの腕＝切替先の名前と `OnGhostChanging` を送るか否か）を 1 本だけ持ち、台本の `\![change,ghost,…]` とメニューの「ゴースト」枠の両方がその入口を通る（経路を 2 本持たない）。
2. When 再生中の台本が `\![change,ghost,名]` の位置に達する, the areka shall その名前を名指しの切替先として受け、`OnGhostChanging` を送らない切替要求を入口へ出す。
3. When 再生中の台本が `\![change,ghost,名,--option=raise-event]` の位置に達する, the areka shall `OnGhostChanging` を送る切替要求を入口へ出す。
4. When 利用者が右クリックメニューの「ゴースト」枠から 1 体を選ぶ, the areka shall その 1 体を切替先とし、`OnGhostChanging` を送る切替要求（Ref1＝`manual`）を入口へ出す。
5. The areka shall 切替先の名前を、まず目録の各ゴーストの `descript.txt` の `name` と突き合わせ、一致が無ければフォルダ名と突き合わせて決める（要件 11 裁定 8）。名前の比較は大文字小文字を区別する。
6. If 切替先の名前が目録のどのゴーストにも一致しない（`random`／`sequential`／`lastinstalled` など本仕様では解決しない名前を含む）, then the areka shall 正典どおり切替を無視し、`warn!` を 1 件残し、ゴーストを降ろさず `OnGhostChanging` も送らない（利用者から見える変化は 0）。
7. The areka shall 「該当するゴーストが無い」の判定を、ゴーストを降ろす前・`OnGhostChanging` を送る前に行う（降ろしてから該当なしに気付く形にしない）。この突き合わせは台本の受け口（talk スレッドの `CueSink`）ではなく、UI 側（フレームの相）の取り出しで目録を読んで行い、受け口は名前を無変形で運ぶだけにする。`ghost-change-name-resolution` が置き換える「解決できない名前」の腕はこの UI 側の取り出しにある（同 spec の brief が指す `change_cue.rs` は運ぶ側であって腕の所在ではない。要件討議 2026-09-26 で確定・同 brief へ申し送り済み）。
8. When 切替先が現在のゴースト自身である, the areka shall 無視せず、他のゴーストへの切替と同じ経路で降ろして起こし直す（読み直しの代用。要件 11 裁定 9）。
9. While 切替が進行中である（切替の目印が立っている）, when 新しい切替要求が届く, the areka shall 新しい要求を無視して `warn!` を 1 件残す（切替を重ねない）。
10. The areka shall 台本の `\![change,ghost,…]` を汎用命令の名前の選別で受け（`change` の第 1 引数 `ghost` までを鍵にする）、型付きの命令を新設しない。`(change,shell)`・`(change,balloon)` の消費者は登記しない（0 組のまま）。
11. When メニューの「ゴースト」枠を組み立てる, the areka shall 目録（`catalog::list_ghosts`）にあるゴーストを、`descript.txt` の `name`（無ければフォルダ名）で、目録の並びのまま列挙し、現在のゴーストも含める。目録が 1 体だけのときも枠は出す。
12. When ゴーストを起こす（初回の起動でも切替後でも）, the areka shall 「ゴースト」枠の登記をそのゴーストの `MenuWiring` に対してやり直す（完了 `ghost-restart-unit` の契約）。

### Requirement 2: 送り出しの握手は正典の順序で行われる

**Objective:** As a ゴーストの作者, I want 交代の台詞（`OnGhostChanging`）と別れの台詞（`OnClose`）が正典の順序と Reference で届くこと, so that 里々／YAYA の標準テンプレートの交代の台詞がそのまま動く

#### Acceptance Criteria

1. When `OnGhostChanging` を送る切替要求を受ける, the areka shall 現在のゴーストへ `OnGhostChanging` を GET で送り、Reference を Ref0＝切替先の本体側の名前（切替先の `descript.txt` の `sakura.name`。無ければ空）・Ref1＝`manual`（メニュー）または `automatic`（台本・要件 11 裁定 7）・Ref2＝切替先のゴーストの名前（`descript.txt` の `name`。無ければフォルダ名）・Ref3＝切替先のゴーストのフォルダの絶対パス、で載せる。
2. When `OnGhostChanging` が台本を返す, the areka shall その台本を通常のトークとして最後まで再生し、再生が終わってからゴーストを降ろす（`OnClose` は送らない）。
3. When `OnGhostChanging` が 204（返事なし）を返す, the areka shall 正典どおり続けて `OnClose` を送り、その別れの台詞を最後まで再生してからゴーストを降ろす。`OnClose` も 204 なら台詞なしで降ろす。
4. While 切替の目印が立っている, when 送り出しの台詞（`OnGhostChanging` の台詞・`OnClose` の別れの台詞）が最後まで再生される, the areka shall ゴーストを降ろすことで終わり、アプリを終了しない（`quit_app` 0 回・`AppExit` 0 件）。台詞が `\-` を含んでいても同じ。
5. When 送り出しの台詞が完了 `balloon-break` と同じ 30 秒の上限に達する, the areka shall 上限に達した時点で台詞を打ち切ってゴーストを降ろし、切替を続ける（終了しない）。
6. When `OnGhostChanging` を送らない切替要求（`raise-event` 無しの台本）を受ける, the areka shall `OnGhostChanging` も `OnClose` も送らず、切替の命令を運んだ台本の再生が終わってから（利用者の中断で早く終わった場合も含む）ゴーストを降ろす（要件 11 裁定 6）。
7. When 切替のためにゴーストを降ろす, the areka shall SHIORI を降ろす順序（`OnClose` の通知の有無・`unload`）を今日の終了と同じにし、SHIORI の解放を待ってから切替先を起こす（完了 `nar-install` の申し送り＝解放前に同じフォルダへ触らない）。
8. If 送り出しの握手の途中で現在のゴーストの SHIORI が失敗する（`Fault`）, then the areka shall 握手を打ち切ってゴーストを降ろし、切替を続ける（元のゴーストがもう答えられないので戻す先が無い。切替先も失敗したときは要件 6.4 の告知へ）。
9. While 切替の相にある（送り出しの台詞の再生中、または `raise-event` 無しの切替で命令を運んだ台本の終わりを待っている間）, when 終了要求（メニューの「終了」・OS の閉鎖要求・Alt＋F4）が届く, the areka shall 切替を取りやめて（目印を下ろし `info!` を 1 件残す）再生中の台詞が終わってから今日の終了経路で終わる（利用者の終了の意思が切替に勝つ。完了 `app-lifetime-separation` の「終了要求は必ず終了で終わる」を継ぐ）。`OnClose` を既に送っていれば二度送らず、その別れの台詞の終わりで終了する。

### Requirement 3: 降ろして起こし直す間、プロセスは生き続ける

**Objective:** As a α の利用者, I want 切替の間にゴーストの窓が一度消えても areka が終わらないこと, so that 切り替えたつもりがアプリごと終わる、という形を出さない

#### Acceptance Criteria

1. The areka shall 切替のためにゴーストを降ろすとき、完了 `ghost-restart-unit` の `GhostSession::shutdown`（ticker 停止 → ゴースト実行系の終了 → seriko の join）と「全窓を閉じるが終了しない」操作（`close_windows_for_restart` → `reopen_ghost_windows`）を使い、本番の呼び手として `#[cfg_attr(not(test), allow(dead_code))]` を外す。
2. While 切替の目印が立っている, when 全ゴースト窓が 0 になる, the areka shall 終了の指示（`AppExit`）を出さず、次の tick 以降で切替先の窓を作る。
3. While 切替の目印が立っている, when 降ろした側のゴーストの停止通知（原因 `Quit`・`CloseSilent`・`DeadlineExceeded`・`Forced` のいずれか）が受け口に届く, the areka shall それを「切替による停止」として捌き、`quit_app` を呼ばない（0 回）。`GhostSession::shutdown` が送る `ForceQuit` に対する 1 件と、正典どおりの握手の末に kanade が送る 1 件のどちらも同じ規則で捌く。
4. The areka shall 降ろした側の停止通知と切替先の停止通知を取り違えない（どのゴーストの通知かを区別できる形で受ける。区別の持ち方＝停止原因に切替の値を足すか・通知に世代を載せるか・UI 側に予約を持つかは設計で決める）。
5. The areka shall 切替の目印を、切替先の kanade が起動系列（`OnGhostChanged` または `OnBoot` の応答まで）を終えて定常に入った時点で下ろす（切替先の SHIORI の失敗は起動の呼び出しが返った数フレーム後に非同期で届くので、「起動系列が始まった時点」では早すぎる＝要件 6.6 が成り立たない）。UI がその時点を知る手段（kanade からの通知の形）は設計で決める。When 切替の目印が下りたあとに停止通知が届く, the areka shall 今日どおり `run_ghost_quit_phase` → `quit_app` へ流す（`Fault` なら完了 `shiori-fault-notice` の告知と終了コード 1）。
6. The areka shall `GhostSession` をフレームの系（`run_ghost_quit_phase` と同じ相）から降ろして起こし直せる場所に置く（World の資源に置くか `main` 側に口を出すかは設計で決める）。`Drop` による自動の終了は採らない（完了 `ghost-restart-unit` 要件 1.7 を継ぐ）。
7. The areka shall 切替の経路から終了の指示を出す場所を 1 つも持たない（要件 6.4 の既定ゴーストの失敗＝致命だけが `quit_app` を呼ぶ）。
8. The areka shall 降ろし始めてから切替先のゴーストの窓が出るまでの間に UI スレッドが止まる時間（seriko の join を含む）を **1 秒以内**を目標とし、実機サインオフで測って記録に残す（超えたら要件ディスカッションまたは設計ディスカッションの議題に上げる。決定論テストでは固定しない）。
9. When 切替のために窓を閉じる, the areka shall 「切替のために窓を閉じた」ことをライフサイクル事象として `info!` に残す（完了 `ghost-restart-unit` 要件 4.3 の語彙をそのまま使う）。

### Requirement 4: 迎え入れの握手と切替後の状態

**Objective:** As a ゴーストの作者と α の利用者, I want 切替先のゴーストが「○○から交代」の台詞（`OnGhostChanged`）で起き、その選択が次回起動でも復元されること, so that 交代の台詞が動き、次に areka を起動したときに最後に選んだゴーストが出る

#### Acceptance Criteria

1. When 切替先のゴーストが起き、起動系列に入り、そのゴーストに起動記録（`areka.boot.count`）がある, the areka shall `OnInitialize` のあと `OnBoot` の代わりに `OnGhostChanged` を GET で送り、Reference を Ref0＝直前のゴーストの本体側の名前（`sakura.name`。無ければ空）・Ref1＝直前のゴーストの切替時の台本（`OnGhostChanging` が返した台本。`OnGhostChanging` を送らなかったときと 204 のときは空＝要件 11 裁定 5）・Ref2＝直前のゴーストの名前・Ref3＝直前のゴーストのフォルダの絶対パス・Ref7＝切替先で使うシェルのフォルダ名（例 `master`）で載せる。Ref4〜6 は空で送る（番号を詰めない）。
2. When `OnGhostChanged` が 204 を返す, the areka shall 正典どおり続けて `OnBoot` を送る（起動記録の有無に関わらず）。
3. When `OnGhostChanged` が台本を返す, the areka shall その台本を起動の台詞として再生し、`OnBoot` を送らない。
4. When 切替先のゴーストに起動記録が無い（初めて起動する）, the areka shall 初回起動を最優先にして今日どおり `OnFirstBoot`（204 なら `OnBoot`）を送り、`OnGhostChanged` は送らない（0 件・要件 11 裁定 3）。起動記録は初回の起動と同じ規則で書く。切替で起きたゴーストに送る起動の根は常に 1 つ（`OnFirstBoot` と `OnGhostChanged` の両方を送る形は作らない）。
5. When 切替先の起動系列が `OnGhostChanged`（または `OnBoot`）を終える, the areka shall 以降を今日の起動と同じ定常（`basewareversion` の照会・`OnSecondChange` など）にする。
6. When 切替先のゴーストが起動に成功する, the areka shall 最後に使ったゴーストの記憶（`LastGhost`・App スコープ）にそのゴーストを書く（経路は `GhostRoute::Argv` 以外＝記憶を書く経路。値を足すかは設計）。次回起動でそのゴーストが復元される。
7. When 切替先のバルーンを決める, the areka shall 完了 `baseware-root-layout` の起動解決（同梱バルーン → そのゴーストの最後のバルーンの記憶 → 既定バルーン）を argv の指定なしの分岐で使う（初回起動の argv の第 2 引数は切替後には効かせない）。切替先の `LastBalloon`／`LastShell`（Ghost スコープ）は今日の `on_boot_ok` と同じ規則で書く。
8. When 切替先のゴーストが起きる, the areka shall 窓ごとの状態（`MouseWiring`・`MenuWiring`・選択肢の送り口・`PersistWiring`・説明書の経路 ほか完了 `ghost-restart-unit` 要件 2.3 の一覧）を切替先のもので置き換え、前のゴーストのものを残さない。
9. The areka shall 切替先がキャラ 1 人（1 スコープ）のゴーストでも落ちず、2 人のゴーストと同じ窓の数（0 と 1）で起こす（`derive_scopes()` の据え置き。3 人以上は範囲外）。
10. When 切替先のゴーストの窓を作る, the areka shall 初回の起動と同じ手順（配置の準備 → 監視の状態 → 復元 → 窓の生成 → 入力の受け口の装着）で作り、切替先のゴーストの前回の窓位置（永続化されていれば）を復元する。

### Requirement 5: `raise-event` の切替はバルーンブレークで中止できる

**Objective:** As a α の利用者, I want 交代の台詞の途中でバルーンをダブルクリックしたら交代がやめになって元のゴーストが残ること, so that 止めたのに切り替わる、という形を出さない（正典「バルーンブレーク（通常ダブルクリック）による中止操作も可能」）

#### Acceptance Criteria

1. While `OnGhostChanging` を送った切替の送り出しの台詞（`OnGhostChanging` の台詞、または 204 のあとの `OnClose` の別れの台詞）が再生中である, when 利用者がバルーンの左ダブルクリックで再生を止める, the areka shall 切替を中止し、ゴーストを降ろさず、元のゴーストの定常へ戻す（要件 11 裁定 1）。
2. When 切替が中止される, the areka shall 切替の目印を下ろし、`warn!` ではなく `info!` で「切替を中止した」ことを記録に残し、直後の新しい切替要求を受け付ける。
3. When 切替が中止される, the areka shall 中断された台詞のバルーンを完了 `balloon-break` と同じ規則で隠す（隠す規則を変えない）。
4. While 切替の相にある, when 中断された台詞が `\-` を含んでいた, the areka shall 終了へ結ばない（`take_user_break_quit` の規則は切替の相では効かない＝衝突表 2）。
5. When `OnGhostChanging` を送らない切替（`raise-event` 無しの台本）の、命令を運んだ台本を利用者が中断する, the areka shall 中断が命令の位置より前なら切替要求が出ていないので何も起きず（0 件）、命令の位置より後なら切替を続ける（中止しない）。
6. When 切替が中止されたあと、次に利用者が終了を選ぶ, the areka shall 今日の終了経路（`OnClose` → 別れの台詞 → 終了）をそのまま通る（中止が終了の握手に影響を残さない）。
7. The areka shall 切替の中止によって定常へ戻る経路を、切替の相にだけ持つ（終了の握手には足さない＝完了 `balloon-break` 裁定 6「終了の握手から定常へ戻る経路は 0 本」を守る）。

### Requirement 6: 切替先が起動できないときは既定ゴーストへ戻す

**Objective:** As a α の利用者, I want 壊れたゴーストを選んでしまっても既定ゴースト（emo2）が起きて「○○は起きてこなかった」と話してくれること, so that 選び間違い 1 回でアプリが消えない（開発者裁定「1 体の失敗はアプリの失敗ではない」）

#### Acceptance Criteria

1. While 切替の目印が立っている, when 切替先のゴーストの起動が失敗する（起動解決の失敗・マウントの失敗・SHIORI の失敗＝停止通知の原因 `Fault`・窓を開けない）, the areka shall `error!` を 1 件残し（失敗の種類と理由を含む）、切替先を降ろして、既定ゴースト（`GhostRoute::Default`＝配布物に同梱の emo2）を同じ経路で起こす（要件 11 裁定 2・2026-09-26 開発者裁定＝呼び出し元へは戻さない）。切替では、初回起動の「起動に失敗しても LogSink の代替で骨格だけ起こす」契約（`boot_ghost` の fallback）へ倒れず、失敗として扱う（初回起動の契約そのものは変えない。切替先が居ないまま窓だけ出る形は作らない）。
2. When 既定ゴーストを起こす, the areka shall 通常の起動系列で起こし、`OnGhostChanged` は送らない（交代は成立していない）。その `OnBoot`（起動記録が無ければ `OnFirstBoot` → 204 → `OnBoot` の葉）の Reference に **Ref6＝`halt`・Ref7＝切替先のゴーストの名前**（`descript.txt` の `name`。無ければフォルダ名）を載せ、Ref1〜5 は空で送る（正典 `OnBoot` の「前回の処理中で落ちた時に halt」「落ちたゴースト名」の器を、切替先が起動の途中で落ちた知らせとして使う＝要件 11 裁定 10）。切替失敗を知らせる新しい SHIORI イベントは作らない（0 件）。記憶（`LastGhost`）は既定ゴーストの起動で既定ゴーストに書き換わる（次回起動も既定ゴースト＝SSP の「次回は FIRST へ」と同じ結果）。既定ゴーストの `halt` の台詞（Ref6/7 を受けて「○○は起きてこなかった」と言う辞書）は本仕様の範囲外で、開発者が emo2 に後で足す。
3. When 既定ゴーストへ戻す, the areka shall 利用者向けの告知（メッセージボックス）を出さない（要件 11 裁定 4・2026-09-26 開発者裁定「デスクトップマスコットに箱は無粋」）。記録は 6.1 の `error!` 1 件と、既定ゴーストへ渡す `OnBoot` の Ref6/7 で足りる。
4. If 既定ゴーストの起動も失敗する（既定ゴーストが目録に無い場合を含む）, then the areka shall それを致命的なエラーとして**即時終了**する（2026-09-26 開発者裁定）。終わり方は完了 `shiori-fault-notice` の `Fault` の経路そのまま（告知は既存の場面 `ShioriFault`・終了コード 1・`AREKA_NO_ALERT` の抑止規則も同じ）で、本仕様は告知の場面を新しく足さない（0 場面）。
5. The areka shall 戻す試みを 1 回だけにする（切替先 → 既定 → … と重ねない）。When 壊れた切替先が既定ゴースト自身である, the areka shall 戻す試みを行わず 6.4 の即時終了へ進む（既定ゴーストの失敗は切替の途中でも致命）。
6. While 切替の目印が立っている, when 切替先の SHIORI の失敗が停止通知（`Fault`）として届く, the areka shall 完了 `shiori-fault-notice` の告知と終了コード 1 の経路（`run_ghost_quit_phase` → `quit_app`）へ流さず、6.1 の戻す経路へ回す（衝突表 3）。
7. When 切替先の SHIORI がエラー応答（400・500 など）を返す, the areka shall 完了 `shiori-fault-notice` 要件 6.1 のとおり致命にせず起動を続ける（本仕様は判断を変えない）。

### Requirement 7: kanade に外から SHIORI イベントを送る汎用の入口が 1 本ある

**Objective:** As a 後続の spec を作る開発者, I want UI や台本の消費者から kanade へ「イベント名と Reference 列」で SHIORI イベントを頼める口が 1 本あること, so that `shell-balloon-switch`・`ghost-install`・`network-update` がイベントを 1 つ足すたびに kanade の 5 段（`KanadeMsg`・`Input`・`steady.rs`・`events.rs`・許可表）を触らずに済む

#### Acceptance Criteria

1. The areka shall kanade の外から「イベント名＋Reference 列（Ref0〜Ref n・欠番は空）＋GET か NOTIFY か」を渡して SHIORI イベントを頼める口を 1 本持つ。
2. When その口で頼まれたイベント名が許可表（`ALLOWED_EVENT_IDS`）に無い, the areka shall 送らず（0 件）`warn!` を 1 件残す。許可表に行を足すだけで新しいイベントを送れる形にする。
3. While kanade が定常にある, when その口でイベントを頼まれる, the areka shall そのイベントを SHIORI へ送り、GET の応答の台本を通常のトークとして再生し、204 なら何もしない。応答が届いたときに別のトークが再生中なら、マウス系の応答と同じ規則でそのトークを置き換えて再生する（捨てない・待たない。この口を使うイベントは利用者の操作の結果＝シェル切替・インストール完了・更新の進捗なので、応答を捨てると「操作したのに何も言わない」形になる）。
4. While kanade が定常以外（起動系列・終了系列・切替の相）にある, when その口でイベントを頼まれる, the areka shall 捨てて `warn!` を 1 件残す（待ち行列に積まない。積む形が要る spec が出たらそのときに足す）。
5. The areka shall 本仕様の `OnGhostChanging`／`OnGhostChanged` 自身をその口で送るか、切替の相の専用の腕で送るかを設計で決める（どちらでも要件 2・4 の順序と Reference を満たすこと）。
6. The areka shall その口の判断（許可表にある／無い・定常にある／無い）を決定論テストで固定する。

### Requirement 8: 既存の振る舞いと境界を守る

**Objective:** As a 開発者, I want 本仕様が終了経路・終了コード・並走 spec の持ち場を変えないこと, so that 完了 spec の契約と直後に続く spec の前提が崩れない

#### Acceptance Criteria

1. The areka shall 終了操作 7 種（完了 `app-lifetime-separation` 要件 3）と右クリックメニューの「終了」→ `OnClose` の握手 → `ghost_quit` の経路を変えず、それらの決定論テストを 1 本も落とさない（roadmap の保存義務）。
2. The areka shall 切替を終了操作に数えず、切替の成功でも中止でも終了コードを変えない（プロセスが切替を経て正常に終わったときの終了コードは 0）。
3. The areka shall 常設の smoke テスト（`crates/areka/tests/smoke_boot_loop_exit.rs`・4 方向）を変更後も緑にし、実機サインオフの「絶対パス起動」（argv 上書き）を残す。
4. The 本仕様 shall `run_ghost_quit_phase` に足す分岐を「切替の目印が立っている間の停止通知」に限り、それ以外の停止通知の扱い（`Fault` の告知と終了コード 1 を含む）を変えない。
5. The 本仕様 shall `(change,shell)`・`(change,balloon)`・`OnShellChang*`・`OnBalloonChange`・`\+`／`\_+`・`random`／`sequential`／`lastinstalled` の解決を持ち込まない（それぞれ `shell-balloon-switch`・`ghost-change-name-resolution`）。
6. The 本仕様 shall `GhostBootOptions` に欄を足さない（構造体リテラルが 28 か所・17 ファイル。起動元の情報＝直前のゴーストの名前・パス・切替時の台本は派生関数で渡す）。
7. The 本仕様 shall 目録の素性（`catalog::Identity`）に `sakura.name` を足さず、切替先と直前のゴーストの `sakura.name` は `catalog::companion_balloon` と同じ形の単独の読み手で読む。その読み手を呼ぶのは `OnGhostChanging` の Reference を組むとき（切替先）だけで、直前のゴーストの分は起動済みの `MountModel` の名前から取り、メニューの「ゴースト」枠は `sakura.name` を読まない。
8. The 本仕様 shall 触るファイルすべてを 1 ファイル 1,000 行の目安の内側に収める。上限に近い `schedule/steady.rs`（935）・`actor_tests.rs`（970）・`runtime_tests.rs`（986）・`schedule_tests.rs`・`steady_flow_tests.rs` には行を足さず、切替の相は新しいファイル（前例 `schedule/user_break.rs`）、受け口は新しいファイル（前例 `emo2_boot/readme_cue.rs`）、テストは兄弟の新ファイルへ置く。
9. The 本仕様 shall `crates/areka-parsers/src/sakura/decode.rs`・`crates/areka-sakura/src/compile.rs` に触らない（`ghost-change-name-resolution` の持ち場）。
10. The 本仕様 shall 本番コードが読む環境変数を新しく足さない。
11. The 本仕様 shall 失敗の経路に記録の無いものを作らない（無視は `warn!`・戻すは `error!`・中止は `info!`・切替の各段はライフサイクル事象として `info!`）。

### Requirement 9: 網羅台帳と裁量の記録

**Objective:** As a 開発者, I want 実装した語彙が台帳に映り、areka の裁量が §8 に残ること, so that 「書いてあるのに何も起きない」の一覧から切替が外れ、裁定の根拠が後から追える

#### Acceptance Criteria

1. The 本仕様 shall `doc/ukadoc-coverage/ledger/shiori.toml` の `OnGhostChanging`／`OnGhostChanged` と `sakura-script.toml` の `\![change,ghost,ゴースト名(,--option=raise-event)]` を実装済みへ更新し（owner＝本仕様・備考に `random`／`sequential`／`lastinstalled` は `ghost-change-name-resolution` と記す）、生成物を生成器で作り直す（手で直さない）。
2. The 本仕様 shall `doc/COMPAT_ARCHITECTURE.md` §8 に次を 1 行ずつ記す: (a) 切替の途中の `OnClose` と別れの台詞は終了で終わらない（完了 `balloon-break` 要件 3.6 の適用範囲の限定）、(b) 切替の相の中断は中止であり `\-` の予約を終了に結ばない（同 3.8 の限定）、(c) 切替先の `Fault` は既定ゴーストへ戻す（完了 `shiori-fault-notice` 要件 3.1 の限定・既定ゴースト自身の失敗は致命＝3.1 のまま即時終了）、(d) 起動の根は 1 つ・起動記録が無いゴーストへの切替は初回起動（`OnFirstBoot`）を最優先し `OnGhostChanged` を送らない、(e) `raise-event` 無しの切替では `OnClose` も送らない、(f) `OnGhostChanged` の Ref1 の中身、(g) 名前の突き合わせの順序と自分自身への切替、(h) 切替先の失敗は既定ゴーストへ戻し、その `OnBoot` の Ref6＝`halt`・Ref7＝切替先の名前で伝える（SSP の「切替は成功扱い・壊れたまま表示・次回起動で FIRST へ」は採らない・切替失敗の新イベントは作らない）。各行に正典の沈黙の根拠と開発者裁定の日付を付ける。
3. The 本仕様 shall `crates/areka/src/menu/mod.rs` の登記待ちのコメント（`#[allow(dead_code)]` 3 か所の理由）と `doc/ukadoc-coverage/briefing-sakura-script.md` の「未対応」の行を、実装後の形に合わせて改める。

### Requirement 10: 決定論テストと実機確認

**Objective:** As a 開発者, I want 切替の判断の分岐が偽の SHIORI 2 体で赤にでき、実機で 1 周していること, so that 後の変更で「切り替えたつもりがアプリごと終わる」「止めたのに切り替わる」に戻ったら赤になる

#### Acceptance Criteria

1. The 本仕様 shall 偽の SHIORI 2 体（x64 の偽境界＝`ShioriWiring::Custom`・偽の資産）で「A を起こす → `\![change,ghost,B,--option=raise-event]` → `OnGhostChanging`（台本あり）→ 再生完了 → 降ろす → B（起動記録あり）を起こす → `OnGhostChanged`（Ref0〜3・7 を突き合わせる）→ 204 → `OnBoot`」を同じプロセス・同じ World で 1 周する決定論テストを 1 本持ち、⑴ イベント列と Reference、⑵ 降ろした側の停止通知で `quit_app` が呼ばれていない（終了の指示 0 件）、⑶ 系の登録数が 1 周目と同じ、⑷ 窓ごとの状態が B のもので置き換わっている（決定論テストでは実窓を作れないので、完了 `ghost-restart-unit` と同じく配線の資源＝説明書の経路・メニューの登記・中断の旗・停止通知の受け口で判定する）、を集めてから 1 回で判定する。
2. The 本仕様 shall `OnGhostChanging` が 204 のとき `OnClose` が続き、その別れの台詞（`\-` 入りを含む）が最後まで流れても・中断されても・期限切れでも、終了ではなく降ろすことで終わることを決定論テストで固定する（要件 2.3〜2.5・衝突表 1）。
3. The 本仕様 shall `raise-event` 無しの切替で `OnGhostChanging` も `OnClose` も送られず（0 件）、命令を運んだ台本の終了後に降ろされることを決定論テストで固定する（要件 2.6）。
4. The 本仕様 shall `raise-event` 付きの切替の送り出しの台詞をバルーンブレークで止めると切替が中止され、元のゴーストの定常へ戻り、`\-` の予約が終了に結ばれないことを決定論テストで固定する（要件 5・衝突表 2）。
5. The 本仕様 shall 切替先の起動が失敗（接続に失敗する偽 SHIORI＝`Fault`）したとき既定ゴーストが `OnBoot`（Ref6＝`halt`・Ref7＝切替先の名前・`OnGhostChanged` 0 件）で起き告知が出ないこと、既定ゴーストが目録に無いか失敗したとき完了 `shiori-fault-notice` と同じ経路で即時終了（終了コード 1・新しい告知の場面 0）になることを決定論テストで固定する（要件 6・衝突表 3）。
6. The 本仕様 shall 該当なし（未知の名前・`random`・`lastinstalled`）で `warn!` 1 件・降ろさず・`OnGhostChanging` 0 件、切替中の二重要求で `warn!` 1 件、自分自身への切替で降ろして起こし直される、をそれぞれ決定論テストで固定する（要件 1.6〜1.9）。
7. The 本仕様 shall `run_ghost_quit_phase` のテスト（`frame_ghost_quit_tests.rs` の兄弟）に「切替の目印が立っている間の停止通知は `quit_app` を呼ばない・下りたあとは呼ぶ」を足す（要件 3.3・3.5）。
8. The 本仕様 shall 汎用の通知の入口の判断（許可表・定常か否か）を決定論テストで固定する（要件 7.6）。
9. The 本仕様 shall メニューの「ゴースト」枠の登記（目録の並び・現在のゴーストを含む・選ぶと `manual` の切替要求が出る・起こすたびにやり直す）を決定論テストで固定する。
10. The 本仕様 shall 足すテストを上の判断の分岐に限り、既に確かめられている配線（`ghost-restart-unit` の登録と載せ替え・終了の握手・seriko の join・窓の配置・告知の文面）を再テストしない。既存の決定論テストは 1 本も落とさない（本番ソースの字面で形を固定しているテストは新しい字面へ追随させ、削除しない）。
11. When 実機で確認する, the 開発者 shall ① emo2 で起動し、メニューの「ゴースト」枠から R_POST_and_KOMAINU を選び、emo2 の交代の台詞のあと R_POST が「○○から交代」の台詞で起きること、② R_POST から emo2 へ戻れること（往復 1 周）、③ ①の交代の台詞の途中でバルーンをダブルクリックすると交代がやめになって emo2 が残ること、④ R_POST から SHIORI が失敗するゴースト（`shiori-host32-testdll-loadu` を SHIORI に持つフォルダ）へ切り替えると既定ゴースト emo2 が `OnBoot` の Ref6＝`halt`・Ref7 つきで起きること（R_POST から行うと「呼び出し元」と「既定」の差が見える。emo2 の `halt` の台詞は無くてよく、ログの Reference で判定する）、⑤ ① のあとに終了して再起動すると R_POST が出ること（記憶）、を有界の自動終了つきで見る。`RUST_LOG` は判定の分岐（kanade の切替の相・areka の `ghost_quit`／切替の事象／`app_exit`）の水準まで開ける。降ろし始めから切替先の窓が出るまでの時間を記録する（要件 3.8）。
12. The 本仕様 shall 実機の 5 走行の結果（コマンド・終了コード・目印の件数・止まった時間）を spec の文書に残す。
13. The 本仕様 shall 起動記録の無い B へ切り替えたとき `OnFirstBoot`（204 なら `OnBoot`）が送られ `OnGhostChanged` が 0 件であること、起動記録のある B では `OnGhostChanged` が送られ `OnFirstBoot` が 0 件であることを、同じ決定論テストの形で固定する（要件 4.1・4.4・裁定 3。起動の根は常に 1 つ）。

### Requirement 11: 裁定（暫定・要件ディスカッションで確定）

**Objective:** As a 開発者, I want brief が挙げた議題 ⑴〜⑸ と、要件を書く途中で答えが要った 4 点を要件の段階で一旦決めておくこと, so that ギャップ分析と設計がこの形で進み、覆すなら要件で覆す

#### Acceptance Criteria

1. The 本仕様 shall **裁定 1（中断で切替を中止する・brief 議題 ⑴）**として、`raise-event` 付きの切替の送り出しの台詞をバルーンブレークで止めたら切替を中止し、元のゴーストの定常へ戻す（要件 5）。根拠: 正典が「中止操作も可能」と明記する。完了 `balloon-break` 裁定 6（2026-09-20）は終了の話で切替には及んでいない。kanade に「切替の相から定常へ戻る」経路が 1 本増えるが、終了の握手には足さない。`shell-balloon-switch` のシェル切替の中止も同じ答えを継ぐ。**別案**（09-20 の裁定に倣って中断しても切替へ進む）は「止めたのに切り替わる」を出すので採らない。
2. The 本仕様 shall **裁定 2（起動できなければ既定ゴーストへ戻す・brief 議題 ⑵・2026-09-26 開発者裁定で確定）**として、切替先が起動できないとき既定ゴースト（emo2）を起こし、既定ゴースト自身が失敗したら致命として即時終了する（要件 6・終わり方は完了 `shiori-fault-notice` のまま）。根拠: 開発者裁定「1 体の失敗はアプリの失敗ではない」（2026-09-24）と「確実に `halt` に応えるゴースト（既定）へ戻す」（2026-09-26）。利用者から見える差は「壊れたゴーストを選んでも emo2 が起きて『○○は起きてこなかった』と話す」。要件生成時の暫定（呼び出し元へ戻す）は、第三者のゴーストが `OnBoot` の Ref6/7 を扱う保証が無く黙って戻るだけになるので覆した。**別案**（告知して終了）は採らない。
3. The 本仕様 shall **裁定 3（初回起動が最優先・brief 議題 ⑶・2026-09-26 開発者裁定で確定）**として、初めて起動するゴースト（起動記録なし）へ切り替えたときは `OnFirstBoot`（204 なら `OnBoot`）を送り `OnGhostChanged` を送らず、起動記録があるときだけ `OnGhostChanged`（204 なら `OnBoot`）を送る（要件 4.1・4.4）。根拠: ukadoc の「204 なら続けて」の木で `OnFirstBoot`・`OnGhostChanged`・`OnGhostCalled`・`OnVanished` は `OnBoot` へ落ちる兄弟の根で、根どうしに辺は無い＝起動の根は 1 つ。優先順は ukadoc 本体が沈黙し、里々 wiki「起動・終了関連」が「初回起動時はそれが交替であれ呼出であれ OnGhostCalled/OnGhostChanged は発生しない」「初めての起動ということで、他のイベントより優先」と明記する。初回の条件は永続の状態（一度も起きていない）で、切替は出来事なので状態の根が先。**要件生成時の暫定（常に `OnGhostChanged`）は覆した**＝里々の標準テンプレートの「＊初回」が切替経由でも動く。
4. The 本仕様 shall **裁定 4（衝突表の優先順・brief 議題 ⑷⑸）**として、切替の目印が立っている間は「中止して元の定常へ戻る」＞「切替で降ろすだけ」＞「終了で終わる」の順で効かせ、切替先の `Fault` は「既定ゴーストへ戻す」へ回す（衝突表 1〜3・要件 2.4・5.4・6.6）。既定へ戻すときは告知を出さない（要件 6.3）。根拠: 2026-09-26 開発者裁定「デスクトップマスコットにメッセージボックスは無粋」＝失敗は既定ゴーストの台詞（`OnBoot` の Ref6/7）で伝える。告知の部品はモーダルで、完了 `shiori-fault-notice` 裁定 4 が「フレームの相の中でモーダルを回さない」と定めてもいる。**別案**（「○○は起動できませんでした」を出してから戻す）は、告知を出す場所（全窓を閉じたあと・起こし直す前）を新しく作る必要があり、規模 L を守るため採らない。要件ディスカッション（2026-09-26）で告知なしを確定した。
5. The 本仕様 shall **裁定 5（`OnGhostChanged` の Ref1）**として、Ref1 に `OnGhostChanging` が返した台本をそのまま載せ、`OnGhostChanging` を送らなかったとき（`raise-event` 無し）と 204 のときは空にする（要件 4.1）。根拠: 正典「直前のゴーストの切り替え時のスクリプト」は `OnGhostChanging` の応答を指すと読むのが自然で、`raise-event` 無しの台本は「切り替え時の」台本ではなく命令を運んだ通常の台本である。**別案**（命令を運んだ台本の全文を載せる）は消費者の位置から元の台本の全文を取り戻す口が要るので採らない。
6. The 本仕様 shall **裁定 6（`raise-event` 無しは `OnClose` も送らない）**として、`OnGhostChanging` を送らない切替では `OnClose` も送らず、命令を運んだ台本の終了後に黙って降ろす（要件 2.6）。根拠: 正典は `OnClose` を「`OnGhostChanging` が 204 のとき続けて発生」としか書かず、`OnGhostChanging` を通さない切替での `OnClose` に沈黙する。`raise-event` 無しの切替は台本の作者が「別れの台詞はこの台本に書いた」と決めた形（`\+` の説明「このスクリプトによる切り替えでは `OnGhostChanging` は通知されない」と同じ意図）。**別案**（`OnClose` を送る）は、台本の作者が別れを言ったあとに別れの台詞がもう 1 度流れる。
7. The 本仕様 shall **裁定 7（`OnGhostChanging` の Ref1）**として、メニューからの切替を `manual`、台本（`raise-event` 付き）からの切替を `automatic` にする（要件 2.1）。根拠: 正典「手動で切り替えた場合 manual、システムにより切り替えられた場合 automatic」。台本の命令は利用者の手ではなくゴースト（システム側）の判断で出る。**別案**（`raise-event` は「メニューから切り替え操作をした場合と同じく」なので `manual`）は、正典の文が `OnGhostChanging` の有無について言っており Ref1 の値には触れていないので採らない。
8. The 本仕様 shall **裁定 8（名前の突き合わせ）**として、切替先の名前を `descript.txt` の `name` → フォルダ名の順で引き、大文字小文字を区別する（要件 1.5）。根拠: 正典の記述例「`\![change,ghost,Emily/Phase4.5]`」はゴーストの `name` であり、里々 wiki の tip は `name` と `sakura.name` の不一致で失敗すると書く（＝SSP は `name` で引く）。フォルダ名を 2 番目に置くのは記憶の鍵がフォルダ名なので利用者がフォルダ名で書く場面があるため。**別案**（`sakura.name` でも引く）は本体側の名前が複数のゴーストで重なりうるので採らない。
9. The 本仕様 shall **裁定 9（自分自身への切替）**として、切替先が現在のゴースト自身でも無視せず同じ経路で降ろして起こし直す（要件 1.8）。根拠: `\![reload,ghost]` を α 後に送った代用（`network-update` の brief「読み直しは同じゴーストへの切替で代用」）が成り立ち、経路に例外を作らない。正典は自分自身への切替に沈黙する（里々 wiki は「自分自身に交替」を起こりうる形として書く）。
10. The 本仕様 shall **裁定 10（切替失敗の形＝既定ゴーストへ戻し、`OnBoot` の Ref6/7 で伝える・2026-09-26 開発者裁定）**として、切替先の起動失敗は既定ゴーストを起こす（裁定 2）と同時に、失敗の知らせを新しい SHIORI イベントではなく、既定ゴーストの `OnBoot` の Ref6＝`halt`・Ref7＝切替先の名前で伝える（要件 6.2）。根拠: 正典 `OnBoot` の Reference6「前回の処理中で落ちた時に halt」・Reference7「前回の処理中で落ちたゴースト名」が「試したゴーストが処理の途中で落ちた」知らせの器として在り、ukadoc に「切替失敗」のイベントは無い（`Failure` 系は `\![execute,…]` の `--event` の流儀だけ）。areka 独自のイベントを作っても受ける辞書がどのゴーストにも無い。**検討して採らなかった形**（開発者の説明による SSP の挙動）: ⑴ 切替を成功扱いにして SHIORI が死んだまま表示を続け、失敗を記憶して次回起動で FIRST（既定）ゴーストへ強制切替——kanade に「SHIORI が死んでいるが生きている」状態が要り、完了 `shiori-fault-notice` の「`Fault` は告知して終了」を切替後に限って覆す規模になるため α では採らず、α 後の候補として `roadmap.md` に「壊れたゴーストを表示し続ける」で登記する。⑵ 呼び出し元へ戻す——第三者のゴーストが Ref6/7 を扱う保証が無く、黙って戻るだけになりうる（要件生成時の暫定。開発者裁定で「確実に応える既定ゴースト」固定へ改めた。既定ゴースト自身が壊れた切替先なら致命＝即時終了）。⑶ 成功扱い＋今日どおり告知して終了——選び間違い 1 回でアプリが消える。
11. Where 設計・実装の途中で裁定 1〜10 のいずれかを覆す必要が判明する, the 本仕様 shall 開発者へ議題として上げ、確定を待ってから要件・設計・`doc/COMPAT_ARCHITECTURE.md` §8 の該当箇所を改める。
