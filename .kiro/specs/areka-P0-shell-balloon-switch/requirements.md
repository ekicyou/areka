# Requirements Document

> 本文の実測は **2026-09-30・本ブランチ**（main `d79ca8dc`＝`network-update` の着地直後）のもの。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。
> 要件 12 の裁定 1〜9 は 2026-09-30 の要件ディスカッションで開発者が確定した（裁定 1・2・4〜7 は要件生成時の推奨案を一括で確認・裁定 3 は推奨案を覆して確定・裁定 8・9 は議題 1・2 で新たに確定）。覆すときは要件 12.10 に従う。

## Project Description (Input)

**誰の何が困っているか**: 別のシェルを持つゴーストの利用者と、バルーンを入れ替えたい利用者（α の利用者＝第三者）。α の利用者の一周は「右クリックメニューでゴースト／シェル／バルーンを替える」を含むが、今日はゴーストしか替えられない。

**今の状態**: `\![change,shell,名]` と `\![change,balloon,名]` は解析は通るが消費者が居らず、**黙って何も起きない**（台本の受け口 `ChangeCueSink` は `(change,ghost)` だけを受理し、`(change,shell)`・`(change,balloon)` は担当外として読み飛ばす）。`OnShellChanging`／`OnShellChanged`／`OnBalloonChange` は製品コードに 0 件で、送出の許可表にも無い。右クリックメニューの「シェル」「バルーン」の枠は在るが登記が 0 件で項目が出ない。走っているゴーストの中で絵と文字の出し先を差し替える語彙は 1 つも無く、シェルを決める解決は 4 か所から独立に呼ばれて既定のシェルしか見ない。起動のたびに「最後のシェル」の記憶が既定シェルで上書きされ、読み手も無い。

**何を変えるか**: SHIORI を降ろさずに、同じゴーストの別のシェルへ（`\![change,shell,名(,--option=raise-event)]`・メニューの「シェル」枠）、別のバルーンへ（`\![change,balloon,名]`・メニューの「バルーン」枠）替わる。正典の順序と Reference で `OnShellChanging`（`raise-event` のときとメニュー）→ 差し替え → `OnShellChanged`、`OnBalloonChange` を送る。差し替えは台詞の切れ目で行い、表示が 1 フレームも崩れない。選択は記憶に書かれ、次回起動でシェルも復元される（起動時にシェル名を効かせる口は本仕様が作る）。該当が無ければ `warn!` して無視、途中で失敗したら `error!` して元のシェル／バルーンのまま表示が続く。ゴースト切替の入口（`SwitchRequest`）・名前解決の腕・読み直し（`\![reload,…]`）・着せ替え・拡大率は本仕様の外。

> 起票: 2026-09-20 `/kiro-discovery` 再入（棚卸⑮）で `areka-P0-ghost-shell-balloon-switch` から切り出し。先進坑 `pilot-balloon-asset-swap`（2026-09-24 判定「直す」＝案 B）が前提依存を満たす。着手順は 2026-09-29 の開発者指示で `network-update` の後（B8）。brief の各節の file:line は起票時の実測で、要件生成時（2026-09-30）に「何の定義か」で引き直した。

## Introduction

### 誰が困っているか

α の利用者（第三者）と、シェルを複数持つゴーストの作者。ゴースト側の台本（`\![change,shell,…]`・`\![change,balloon,…]`）も、右クリックメニューの「シェル」「バルーン」枠も、今日は何も起こさない。バルーンを入れ替えるには `areka.exe` の第 2 引数で起動し直すしかなく、それは利用者の操作ではない。シェルは起動し直しても替えられない。

### いま何が起きているか（2026-09-30 実測）

- **切替の語彙に消費者が居ない。** `crates/areka/src/emo2_boot/consumer_ledger.rs` の `canonical()` は 13 組を登記し、`("change", Some("ghost"))` だけを `ChangeSink` へ結ぶ。同ファイルのテスト `canonical_registers_only_change_ghost_for_the_change_sink` が「`(change,shell)`・`(change,balloon)` は登記されていない」を固定している。`crates/areka/src/emo2_boot/change_cue.rs` の `ChangeCueSink::emit` は `("change","ghost")` だけを受理し、他は `debug!(event="change_cue_skip")` で読み飛ばす。
- **通知は送れる形があるが、許可表に無い。** `crates/areka-kanade/src/msg.rs` の `KanadeMsg::RaiseEvent { id, references, method, reply }` → `crates/areka-kanade/src/schedule/change.rs` の `on_raise_event` が、`events::allowed_static`（許可表 `ALLOWED_EVENT_IDS`＝**42 語**・`crates/areka-kanade/src/schedule/events.rs`）との逐語照合のあと、定常（`Phase::Steady`・再生中を含む）でだけ送る。定常以外では `warn!(raise_event_not_steady)` で捨てる（積まない）。応答の台本は `events::value_replaces_active_talk`（`OnSecondChange` 以外はすべて）により**再生中のトークを置き換える**。`OnShellChanging`／`OnShellChanged`／`OnBalloonChange` は許可表に無い。許可表の件数の直書きは `events_change_tests.rs` の `assert_eq!(ALLOWED_EVENT_IDS.len(), 42)` と `events_tests.rs` の `allowed_event_ids_are_exactly_the_forty_two_and_exclude_ontalk_onhour`（42 語の完全一致）の 2 か所。
- **ゴースト切替の入口はゴースト専用。** `crates/areka/src/emo2_boot/ghost_switch.rs` の `SwitchRequest { ghost: GhostSpec, raise_event, origin: ChangeOrigin, boot_event: Option<(&'static str, Vec<String>)> }`・`GhostSpec::{Name, Folder}`・`request_ghost_switch(world, req) -> SwitchVerdict`（`Accepted`／`Busy`／`NotFound`／`NoContext`）。名前は `resolve_switch_target` が「descript の `name` → フォルダ名」の順・大文字小文字を区別して引き、`random`／`sequential`／`lastinstalled` は `resolve_special_name(name, &[GhostEntry], current, last_installed, pick)` が解く（**ゴーストの目録専用**でシェル・バルーンには使えない）。`lastinstalled` の記録 `LastInstalledGhost` はプロセスの中だけで、書き手 `record_last_installed` は `crates/areka/src/install/desk.rs`（ゴーストを入れたとき）。バルーンを入れたときは同ファイルの `remember_balloon` が Ghost スコープの `LastBalloon` を書くだけで、「最後に入れたバルーン」のプロセス内の記録は無い。
- **出し先は起動時に固定され、差し替える語彙が無い。** `crates/areka-emo-present/src/presenter/hub.rs` の `attach_target` は `targets` の表を置き換えるだけ（同じ id の再登録は表示の状態ごと置き換え、可視性の持ち主が既定へ戻る）。片付ける口・登録を消す口は 0 件、`PresentCommand`（`crates/areka-emo-present/src/command.rs`）は `ShowSurface`／`Hide`／`InvalidateCache` の 3 腕。先進坑 `pilot-balloon-asset-swap` は「同じ id の再登録だけでは古い絵と新しい絵が重なる。古い装着を消してから同じ呼び出しの中で再登録・表示・窓寸合わせ（配置が決まる前の段）なら反映待ち以外の崩れは 0」を確かめた（`crates/pilot/examples/pilot-balloon-asset-swap/README.md`）。
- **シェルの解決は 4 か所・既定しか見ない。** `crates/areka-parsers/src/package/resolve.rs` の `resolve(ghost_root, default_encoding)` は `seriko.defaultsurfacedirectoryname`、無ければ `master` を採る（シェル名の引数は無い・952 行）。本番の呼び手は `crates/areka-ghost/src/runtime.rs` の `boot_with_origin`・`crates/areka/src/emo2_boot/assets.rs`・`crates/areka/src/placement/source.rs`・`crates/areka/src/placement/persist.rs` の 4 か所（シェル名が効くのは前 3 つ）。起動入力 `Emo2BootInputs`（`crates/areka/src/emo2_boot/mod.rs`）は `ghost_root`・`balloon_root`・`shiori`・`ticker`・`app_profile_dir`・`boot_origin` の 6 欄で、シェルを選ぶ欄は無い。
- **最後のシェルは書かれるだけで読まれない。** `crates/areka/src/boot_resolve.rs` の読み手は `read_last_ghost`・`read_last_balloon`・`read_session_mark` の 3 つで、`read_last_shell` は無い。`LastUsed::record` は起動が成功するたびに `mount().shell.dir` の末尾を Ghost スコープの `LastShell` へ書く（`main.rs` の `record_last_used`）。**切替だけ作ると次の起動で既定シェルが書き戻され、切り替えた記憶が消える**。
- **目録。** `crates/areka-ghost/src/catalog.rs` の `list_shells(ghost_dir)` は `menu,hidden` のシェルを除外する（正典では隠しシェルも名指しなら切り替えられる）。`list_balloons(root)` はベースウェアの根の `balloon/` を列挙する（同梱バルーンは `.nar` のインストールでそこへ置かれるので、同じ目録に載る）。`ShellEntry`／`BalloonEntry` は `{ dir, identity }`（`Identity` は `folder`・`name` など 7 項目）。
- **メニュー。** `crates/areka/src/menu/` に `ghost_frame.rs`・`install_frame.rs`・`update_frame.rs` が在り、`Frame::Shell`／`Frame::Balloon` は `Frame::ORDER` と文言（`captions.rs` の `shellrootbutton.caption`＝「シェル」・`balloonrootbutton.caption`＝「バルーン」）だけで登記 0 件。登記は `crates/areka/src/ghost_session.rs` の `boot_wired` が `menu::wire_menu` の直後に枠ごとの `register` を呼ぶ形で、ゴーストを起こすたびにやり直す契約。
- **ネットワーク更新が「今のシェル・今のバルーン」を読む。** `crates/areka/src/update/desk.rs` の `here` は今のシェルを `GhostSession::runtime().mount().shell.dir`、今のバルーンを `BootContext.current.balloon.dir` から取る。更新で何か変わると `reload` が `request_ghost_switch(world, SwitchRequest { ghost: GhostSpec::Folder(今のフォルダ), raise_event: false, origin: Automatic, boot_event: Some(…) })` を 1 回呼び、起動時と同じ解き方で全部読み直す。
- **台詞の終わりの合図。** `crates/areka/src/emo2_boot/talk_lifecycle.rs` の `TalkLifecycleSignal`（`TalkStarted`／`DisplayEndAt(f64)`／`UserBreak`）が `Emo2Wiring.lifecycle_rx`（`crates/areka/src/emo2_boot/frame/wiring.rs`）へ届く。kanade の入口は「送る」だけで台詞の終わりを呼び手に知らせない。
- **検体。** `crates/sample-ghost-kit/src/lib.rs` の `SAMPLES` は 7 件。シェルを 2 つ持つ検体は 0（`emo2`・`R_POST_and_KOMAINU`・`konnoyayame`・`claudia` はいずれも `shell/master/` だけ）。`claudia` は同梱バルーンを 2 つ持つ唯一の検体。実機で応答を観測できる辞書: `R_POST_and_KOMAINU` の `dic02_Event.txt` は `OnShellChanging`／`OnShellChanged` に、`konnoyayame` は 3 つすべてに、emo2 は `OnBalloonChange` に応答する。
- **台帳の陳腐化。** `doc/ukadoc-coverage/ledger/shiori.toml` の `shellrootbutton.caption` の備考は引受先を `areka-P0-ghost-shell-balloon-switch`、`balloonrootbutton.caption` は `areka-P0-baseware-root-layout` と書く（どちらも本仕様が引き受ける）。`OnShellChanging`／`OnShellChanged`／`OnBalloonChange`、`sakura-script.toml` の `\![change,shell,…]`・`\![change,balloon,…]` は `absent`・owner 空。
- **行数（ギャップ分析の実測で改めた）。** `emo2_boot/mod.rs` 825・`frame.rs` 533・`consumer_ledger.rs` 831・`ghost_session.rs` 705・`main.rs` 946（上限まで 54）・`boot_resolve.rs` 471・`resolve.rs` 952・`presenter/hub.rs` 176・`catalog.rs` 362／kanade `events.rs` 627・`steady.rs` 935（触らない）／上限が近いテスト: `runtime_tests.rs` 986・`assets_tests.rs` 976・`actor_tests.rs` 970・`schedule_tests.rs` 962・`ghost_switch_tests.rs` 988（足さない・兄弟の新ファイルへ）。

### 正典（ukadoc）の位置づけ

| 正典 | 逐語引用 | 本仕様への含意 |
|---|---|---|
| [`\![change,shell,シェル名(,--option=raise-event)]`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bchange_2cshell_2c_30b7_30a7_30eb_540d_28_2c--option_3draise-event_29_5d:1) | 「そのシェルへの切り替えを行う。該当シェルがなかった場合は無視される。変更後SHIORIイベントOnShellChangedが通知される。」「シェル名をlastinstalledにすると最後にインストールしたゴーストに切り替え。ただしSSPを一度終了した場合は無効。」「シェル名をrandomにするとランダムに切り替え。」「SSP 2.4.74以降で、シェル名の後に--option=raise-eventとすると、メニューから切り替え操作をした場合と同じくOnShellChangingが通知される。この場合、バルーンブレーク（通常ダブルクリック）による中止操作も可能な点に注意。指定しない場合は通知されない。」 | 名指しの切替・該当なしは無視・`OnShellChanged` は必ず・`OnShellChanging` は `raise-event` とメニューだけ・`raise-event` のときは中断で中止できる。`lastinstalled` は原文が「**ゴースト**に切り替え」だが、ukadoc の他の `lastinstalled`（バルーン・カレンダースキン・ヘッドライン・プラグインほか）はどれも命令と同じ種類を指すので誤記と読み、**最後にインストールしたシェル**・プロセスの中だけで有効とする（要件 12 裁定 3）。`random` を受ける。 |
| [`\![change,balloon,バルーン名]`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bchange_2cballoon_2c_30d0_30eb_30fc_30f3_540d_5d:1) | 「そのバルーンへの切り替えを行う。該当バルーンがなかった場合は無視される。変更後SHIORIイベントOnBalloonChangeが通知される。」「バルーン名をlastinstalledにすると最後にインストールしたバルーンに切り替え。ただしSSPを一度終了した場合は無効。」「バルーン名をrandomにするとランダムに切り替え。」 | バルーンには `raise-event` も「切り替え前」のイベントも無い。`lastinstalled` は**バルーン**・プロセスの中だけで有効。`random` を受ける。 |
| [`OnShellChanging`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnShellChanging:1) | 「他のシェルへ切り替えた際に発生。」Reference0「切り替わるシェル名。」Reference1「（SSPのみ）切り替わる前（現在）のシェル名。」Reference2「（SSPのみ）切り替わるシェルのパス。」 | 送り出しの Ref0〜2。SSP のみの Ref も送る。 |
| [`OnShellChanged`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnShellChanged:1) | 「他のシェルから切り替わった際に発生。」Reference0「現在のシェル名。」Reference1「（CROWのみ）Reference0と同じ。（SSPのみ）現在のゴースト名。」Reference2「（SSPのみ）現在のシェルのパス。」 | 差し替え後の Ref0〜2。Ref1 は SSP の意味（現在のゴースト名）を採る。 |
| [`OnBalloonChange`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnBalloonChange:1) | 「他のバルーンから切り替わった際に発生。」Reference0「切り替わったバルーン名。」Reference1「切り替わったバルーンのパス。」 | 差し替え後の Ref0〜1。 |
| [`menu,hidden`（シェルの descript）](https://ssp.shillest.net/ukadoc/manual/descript_shell.html#menu_2chidden:1) | （`catalog::list_shells` が参照する項目） | メニューには出さないが、名指しなら切り替えられる（親 brief Desired Outcome 2）。 |

正典が沈黙している点（会話の途中に切替の命令が来たとき表示中の文字と残りの台本をどうするか・切替の途中で失敗したときの振る舞い・自分自身への切替・`random` の候補の取り方・切替後にどの面を出すか・切替後の着せ替えの状態）は areka の裁量として要件 12 で決め、`doc/COMPAT_ARCHITECTURE.md` §8 に記す。SSP の挙動を実測して合わせることはしない。

### 既存の裁定との関係（どちらが先に効くか）

| # | 既存の裁定 | 本仕様で重なる場面 | 先に効くもの |
|---|---|---|---|
| 1 | 完了 `ghost-shell-balloon-switch` 裁定 1「`raise-event` 付きの切替の送り出しの台詞をバルーンブレークで止めたら切替を中止し、元の定常へ戻す」（同 brief が「`shell-balloon-switch` のシェル切替の中止も同じ答えを継ぐ」と明記） | `raise-event` 付き（またはメニュー）のシェル切替で `OnShellChanging` の台詞を利用者がダブルクリックで止める | **中止して元のシェルのまま**（要件 5.1）。問わない。 |
| 2 | 完了 `balloon-break` 要件 3.8「`\-` を含む台本を利用者が中断したら、ただちに終了する」 | `raise-event` 無しの台本が `\![change,shell|balloon,…]` を運び、その台本を利用者が中断した | **終了が勝ち、保留の切替は捨てる**（完了 `ghost-shell-balloon-switch` 要件 5.5 と同じ・要件 5.4）。`OnShellChanging` の台詞の中断は 1 行目のとおり切替の中止で、`\-` を含んでいても終了には結ばない（同 spec 衝突表 2 を継ぐ）。 |
| 3 | 完了 `ghost-shell-balloon-switch` 要件 7.3「汎用の入口の応答は再生中のトークを置き換える」 | 差し替えの直後に送る `OnShellChanged`／`OnBalloonChange` の応答が届いたとき別のトーク（`OnSecondChange` の独り言など）が始まっていた | **置き換える**（変えない）。差し替え自体は台詞の切れ目で行うので、差し替えが台詞を断ち切ることは無い（要件 2.3・裁定 2）。 |
| 4 | 完了 `ghost-shell-balloon-switch`「`SwitchRequest`／`GhostSpec`／`request_ghost_switch` の形を変えない・`ghost_switch.rs` に触らない」（brief の約束 2）と完了 `network-update` の読み直し（同じフォルダのゴースト切替で全部読み直す） | 更新後の読み直しで差し替えたシェル・バルーンを保つ | **形は変えない**（シェル・バルーンの入口は自前・裁定 9 によりゴースト切替の入口も変えない）。読み直しは記憶（`LastShell`／`LastBalloon`）から起動時と同じ解き方で復元するので、切替の成功時に記憶を書けば保たれる（要件 6.5）。 |
| 5 | 完了 `ghost-install` 裁定「インストールと切り替えは別イベント・areka は入れた後の切替を主導しない」 | シェル・バルーンの `.nar` を入れた直後 | **変えない**。本仕様は入口を用意するだけで、入れた後に自動で切り替えない（0 回）。 |
| 6 | 記憶 no-frame-delay-fixes-change-the-state-shape（1 フレーム遅らせる解は取らない）・先進坑の学び 3「差し替えは配置が決まる前の段に置く」 | 差し替えの相の置き場 | **配置が決まる前の段で、古い装着を消して同じ呼び出しの中で再登録する**（要件 4）。 |

### 何を変えるか

**SHIORI を生かしたまま、絵と文字の出し先だけを替える。** 台本の `\![change,shell,名(,--option=raise-event)]`／`\![change,balloon,名]` とメニューの「シェル」「バルーン」枠が同じ 1 本の要求を出し、areka は台詞の切れ目を待って（`raise-event` とメニューのシェル切替は `OnShellChanging` の台詞を再生し切ってから）古い装着を片付け、新しいシェル／バルーンの資産を同じ呼び出しの中で登録し直し、直後に `OnShellChanged`／`OnBalloonChange` を送る。表示は 1 フレームも崩れず、可視性の持ち主と窓寸の要求は引き継がれる。切替の成功で記憶（`LastShell`／`LastBalloon`）を書き、起動時に「最後のシェル」を効かせる口を作って次回起動でシェルも復元する。該当が無ければ `warn!` して無視、途中の失敗は `error!` して元のまま。判断の分岐は偽のシェル 2 つ・偽のバルーン 2 つで決定論テストに固定し、実機で `R_POST_and_KOMAINU` の 2 シェル往復と既定バルーン ⇄ emo2 同梱バルーンの往復を 1 周する。

## Boundary Context

- **In scope**:
  - シェル・バルーン切替の要求の入口 1 本（ゴースト切替の `SwitchRequest` とは別の、本仕様が自前で持つ形）と、その出どころ＝台本の `\![change,shell,名(,--option=raise-event)]`・`\![change,balloon,名]`・メニューの「シェル」「バルーン」枠。
  - 名前の解決（descript の `name` → フォルダ名・大文字小文字を区別・隠しシェルも名指しなら可）と、`random`（シェル・バルーン）・`lastinstalled`（シェル・バルーンとも＝プロセスの中の記録。シェルの原文「ゴーストに切り替え」は誤記と読む）。
  - `OnShellChanging`（`raise-event` とメニューだけ・Ref0〜2）→ 台詞の再生完了 → 差し替え → `OnShellChanged`（Ref0〜2）、`OnBalloonChange`（Ref0〜1）の送出（既存の汎用の入口を使う・許可表に 3 語）。
  - 走っているゴーストの中での資産の差し替え（シェル＝キャラ窓を残して絵と当たり判定を替える・バルーン＝バルーンの絵と文字の層を替える）と、1 フレームも崩れない保証（可視性の持ち主・窓寸の要求の引き継ぎ・拡大率 1 以外でも同じ）。
  - `raise-event` の切替のバルーンブレークによる中止・失敗時に元のまま続けること・切替の重ね禁止・終了要求が勝つこと。
  - 切替後の記憶（`LastShell`／`LastBalloon`）と、起動時に「最後のシェル」を効かせる口（読み手・実在の確認・シェル名を解決の各呼び出し点へ運ぶ口）。
  - メニューの「シェル」「バルーン」枠への登記（起こすたびにやり直す）。
  - 2 つ目のシェルを持つ検体（テストと実機の作り方は要件 9・裁定 1）。
  - 網羅台帳（`shiori.toml` の 3 イベントと 2 つの `*rootbutton.caption` の備考・`sakura-script.toml` の 2 命令）と生成物・`doc/COMPAT_ARCHITECTURE.md` §8 の裁量の記録。
  - 決定論テストと実機サインオフ。
- **Out of scope**:
  - ゴースト切替そのもの・`SwitchRequest`／`GhostSpec`／`request_ghost_switch` の形・ゴースト名の解決（完了 `ghost-shell-balloon-switch`・`ghost-change-name-resolution`）。`ghost_switch.rs` には触らない。
  - `\![reload,ghost|shell|balloon]`（α 後 `mcp-reload`。同じシェル・バルーンへの切替が代用になる＝要件 1.8）・`\![bind,…]` 着せ替えの操作（完了 `mayuna-compose`／`bindoption-exclusivity`）・`\![set,scaling,…]`／`OnShellScaling`／`OnBalloonScaling`・`OnNotifyShellInfo`／`OnNotifyBalloonInfo`／`OnNotifyDressupInfo`・`currentghost.shelllist.*`／`balloonlist.*` のプロパティ（α 後）。
  - `sequential`（正典はシェル・バルーンに `sequential` を書いていない）。
  - インストール直後の自動切替（`ghost-install` の裁定「別イベント」）・ネットワーク更新の読み直しの形（`network-update`）・更新の対象の解決の変更（`here` が今のシェル・バルーンを指す限り触らない）。
  - 3 人以上のキャラの窓（`derive_scopes()` の据え置き）・`.pna`／`full`／`0` の透過（未実装のシーム）・メッセージボックスでの告知（0 場面）。
  - 検体 `.nar` の畳み直し（裁定 1）・`konnoyayame` のシェルの改変（CC BY-NC-ND）。
- **Adjacent expectations**:
  - 完了 `ghost-shell-balloon-switch` の汎用の通知の入口（`KanadeMsg::RaiseEvent`・許可表は共用）と `GhostSession` の置き場（`GhostSlot`）・`BootContext.current.balloon` の書き換えは本仕様の持ち場（doc コメントに明記済み）を使う。kanade で触るのは許可表の 3 行と「切替の台詞」の口（要件 8.11・裁定 8）。
  - 完了 `ghost-restart-unit` の「ゴーストを起こすたびにメニューの登記をやり直す」契約と `Emo2BootInputs` に乗る（シェル名の欄はそこへ・`GhostBootOptions` には足さない）。
  - 完了 `baseware-root-layout` の目録（`list_shells`・`list_balloons`）・記憶の鍵（`LastShell`／`LastBalloon`＝Ghost スコープ）・書き手（`record_last_used`）を使い、目録の素性（`Identity`）に欄を足さない。
  - 完了 `pilot-balloon-asset-swap` の学び（同じ id の再登録だけでは重なる・古い装着を消してから同じ呼び出しの中で再登録・配置が決まる前の段・`EmoWorld` は複製できない・可視性の持ち主と窓寸の要求が既定へ戻る）の上に建つ。present に「古い装着を片付ける正規の口」と「登録を消す口」を足すのは本仕様。
  - 完了 `network-update`: 更新の対象の解決 `here` が「今のシェル」を `mount().shell.dir`、「今のバルーン」を `BootContext.current.balloon.dir` から取るので、差し替えの相がその 2 つを書き換えれば手当て 0（書き換えない置き場を選ぶなら `here` を読み替える）。更新後の読み直しで差し替えたシェル・バルーンが保たれること（要件 6.5）。更新の実行中に届いた切替要求は受けない（要件 1.13）。
  - 完了 `balloon-color-emoji`（09-30 着地済み）: `areka-emo-text` は書記素クラスタ単位になっている。文字の層の幾何は毎フレームの結び直し（`run_text_scale_phase`）で `BalloonModel` から結ばれるので、ギャップ分析の見立てでは `sink.rs`（`TextMsg`）に触る必要は無い（触るなら本仕様だけ）。
  - 完了 `session-mark-residue` の同期の送信の許可表 `ALLOWED_SYNC_SENDS`（2 行）: 本仕様は同期の送信を足さない（足せば赤）。
  - 既定ゴースト emo2 の辞書は開発者が持つ。`OnBalloonChange` への応答は emo2 の `update.pasta` に在る。

## Requirements

### Requirement 1: 切替要求の入口は 1 本で、台本とメニューが同じ経路を通る

**Objective:** As a α の利用者とゴーストの作者, I want 台本からも右クリックメニューからも同じゴーストの別シェル・別バルーンへ替えられること, so that プロセスを起動し直さずに見た目を替えられ、里々／YAYA の標準テンプレートの `\![change,shell,…]` がそのまま動く

#### Acceptance Criteria

1. The areka shall シェル・バルーンの切替要求の入口（種別＝シェルかバルーンか・切替先の名前・`OnShellChanging` を送るか否か）を 1 本だけ持ち、台本の `\![change,shell,…]`・`\![change,balloon,…]` とメニューの「シェル」「バルーン」枠のすべてがその入口を通る（経路を 2 本持たない）。この入口はゴースト切替の `SwitchRequest` とは別に本仕様が持ち、`SwitchRequest`／`GhostSpec`／`request_ghost_switch` の形は変えない。
2. When 再生中の台本が `\![change,shell,名]` の位置に達する, the areka shall その名前をシェルの切替先として受け、`OnShellChanging` を送らない切替要求を入口へ出す。
3. When 再生中の台本が `\![change,shell,名,--option=raise-event]` の位置に達する, the areka shall `OnShellChanging` を送る切替要求を入口へ出す。
4. When 再生中の台本が `\![change,balloon,名]` の位置に達する, the areka shall その名前をバルーンの切替先として受け、切替要求を入口へ出す（バルーンに「切り替え前」のイベントは無い。第 3 引数以降は読み飛ばし、未知の選択肢は `warn!` を 1 件残して切替は続ける＝`ChangeCueSink` の今日の規則と同じ）。
5. When 利用者がメニューの「シェル」枠から 1 つを選ぶ, the areka shall それをシェルの切替先とし、`OnShellChanging` を送る切替要求を入口へ出す（正典「メニューから切り替え操作をした場合と同じく OnShellChanging が通知される」）。When 利用者がメニューの「バルーン」枠から 1 つを選ぶ, the areka shall それをバルーンの切替先とする切替要求を入口へ出す。
6. The areka shall 切替先の名前を、まず候補の各シェル／バルーンの `descript.txt` の `name` と突き合わせ、一致が無ければフォルダ名と突き合わせて決める（完了 `ghost-shell-balloon-switch` 裁定 8 と同じ規則）。名前の比較は大文字小文字を区別する。シェルの候補は現在のゴーストの `shell/` 直下で `descript.txt` を持つフォルダ全部（`menu,hidden` のシェルを**含む**＝名指しなら隠しシェルにも切り替えられる）、バルーンの候補はベースウェアの根の `balloon/` の目録（`list_balloons`）とする。
7. If 切替先の名前が候補のどれにも一致しない, then the areka shall 正典どおり切替を無視し、`warn!` を 1 件残し、`OnShellChanging` も `OnShellChanged`／`OnBalloonChange` も送らない（利用者から見える変化は 0）。
8. When 切替先が現在のシェル／バルーン自身である, the areka shall 無視せず、他への切替と同じ経路で資産を作り直して差し替え、`OnShellChanged`／`OnBalloonChange` を送る（読み直しの代用・要件 12 裁定 5）。
9. When 切替先の名前が `random` である, the areka shall シェルなら現在のゴーストの隠しでないシェルから、バルーンなら目録のバルーンから、現在のものを除いて 1 つを無作為に選び（候補が 0 なら現在のもの＝1.8 の経路）、選んだ名前を `info!` に残す（要件 12 裁定 4）。
10. When シェルの切替先の名前が `lastinstalled` である, the areka shall このプロセスの中で最後に入れたシェル（シェルのインストールの完了時に、入れた先のゴーストとシェルのフォルダ名をプロセスの中に控えたもの）を切替先とし、控えが無いか、控えが今のゴーストのシェルでないか、今のゴーストの `shell/` に無ければ 1.7 の該当なしとして扱う（プロセスを終えると無効・ゴースト切替は起こさない＝要件 12 裁定 3）。
11. When バルーンの切替先の名前が `lastinstalled` である, the areka shall このプロセスの中で最後に入れたバルーン（インストールの完了時にプロセスの中に控えたフォルダ名）を切替先とし、控えが無いか目録に無ければ 1.7 の該当なしとして扱う（プロセスを終えると無効＝正典どおり）。
12. While シェルまたはバルーンの切替が進行中である（要求を受けてから `OnShellChanged`／`OnBalloonChange` を送り終えるか中止・失敗で終わるまで）, when 新しいシェル・バルーンの切替要求が届く, the areka shall 新しい要求を無視して `warn!` を 1 件残す（切替を重ねない）。While シェルまたはバルーンの切替が進行中である, when ゴースト切替の要求が届く, the areka shall ゴースト切替を今日どおり受け（台詞の再生中なら台詞の後に始まる）、進行中のシェル・バルーンの切替は差し替えの前にゴースト切替の進行を見つけた時点で取りやめて `info!` を 1 件残す（差し替えも `OnShellChanged`／`OnBalloonChange` も記憶の書き込みも無い。新しいゴーストはそのゴーストの記憶のシェルで起きる＝ゴースト切替が勝つ・要件 12 裁定 9）。While ゴースト切替が進行中である, when シェル・バルーンの切替要求が届く, the areka shall 無視して `warn!` を 1 件残す。
13. While ネットワーク更新の実行中である（更新の窓口が要求を預かっている間）, when シェル・バルーンの切替要求が届く, the areka shall 無視して `warn!` を 1 件残す（更新が置き換えている最中のファイルを読まない）。
14. While kanade が定常にない（起動系列・終了系列の途中）, when シェル・バルーンの切替要求が届く, the areka shall 無視して `warn!` を 1 件残す（待ち行列に積まない。汎用の通知の入口の規則と同じ）。
15. The areka shall 台本の `\![change,shell,…]`・`\![change,balloon,…]` を汎用命令の名前の選別で受け（`change` の第 1 引数 `shell`／`balloon` までを鍵にする）、型付きの命令を新設しない。消費者台帳に `("change", Some("shell"))`・`("change", Some("balloon"))` の 2 行を足し、「`(change,ghost)` だけ」を固定していたテストは「3 組が登記され、裸の `change` は無い」へ書き換える（削除しない）。
16. The areka shall 切替要求を受けたこと・切替先の決定・差し替えの完了・中止・無視をそれぞれ記録に残す（受理と完了は `info!`・無視は `warn!`・中止は `info!`・失敗は `error!`）。

### Requirement 2: シェル切替の握手は正典の順序と Reference で行われる

**Objective:** As a ゴーストの作者, I want `OnShellChanging`（切り替え前）と `OnShellChanged`（切り替え後）が正典の順序と Reference で届くこと, so that 里々の標準テンプレート（`dic02_Event.txt`）の切り替えの台詞がそのまま動く

#### Acceptance Criteria

1. When `OnShellChanging` を送るシェルの切替要求（`raise-event` 付きの台本・メニュー）を受け、切替先が決まる, the areka shall 現在のゴーストへ `OnShellChanging` を GET で送り、Reference を Ref0＝切り替わるシェルの名前（切替先の `descript.txt` の `name`。無ければフォルダ名）・Ref1＝現在のシェルの名前（同じ規則）・Ref2＝切替先のシェルのフォルダの絶対パス、で載せる。
2. When `OnShellChanging` が台本を返す, the areka shall その台本を通常のトークとして最後まで再生し、再生が終わってから（バルーンの表示が終わった時点で）差し替えを行う。本仕様で「バルーンの表示が終わった時点」は、台詞の文字を出し終えた時点を指す（バルーンが時間切れで隠れるまでは待たない）。When `OnShellChanging` が 204（返事なし）を返す, the areka shall 台詞なしで差し替えを行う。
3. When `OnShellChanging` を送らないシェルの切替要求（`raise-event` 無しの台本）を受ける, the areka shall `OnShellChanging` を送らず、切替の命令を運んだ台本の再生が終わってから（利用者の中断で早く終わった場合も含む・バルーンの表示が終わった時点で）差し替えを行う（要件 12 裁定 2＝差し替えは台詞の切れ目で行い、表示中の文字と残りの台本には触れない）。
4. When 差し替えが済む, the areka shall 現在のゴーストへ `OnShellChanged` を GET で送り、Reference を Ref0＝現在の（切り替わった）シェルの名前・Ref1＝現在のゴーストの名前（`descript.txt` の `name`。無ければフォルダ名）・Ref2＝現在のシェルのフォルダの絶対パス、で載せる。応答の台本は通常のトークとして再生し、204 なら何もしない。
5. The areka shall シェルの切替の間、SHIORI を降ろさず・起こし直さず、kanade の起動系列も終了系列も走らせない（`OnClose`・`OnBoot`・`OnGhostChanged` は 0 件）。会話の状態（`OnSecondChange` の計時・選択肢の待ち・中断の旗）は差し替えをまたいで保たれる。
6. When 差し替えが済む, the areka shall 各キャラ窓に、差し替えの直前と同じ面番号の面を新しいシェルで表示する（新しいシェルにその面が無ければ、今日の「無い面を指定された」ときの扱いと同じ）。着せ替え（MAYUNA）の状態は新しいシェルの `descript.txt` の既定に戻す。
7. When 差し替えが済む, the areka shall キャラ窓を作り直さず（窓の位置・重なり順・ドラッグ中の状態を保つ）、窓の大きさだけを新しい面の大きさに合わせ、バルーンの位置を新しい面に対して今日の配置の規則で置き直す。シェルの `descript.txt` から読む見た目の値（窓の寸法・作者の DPI〔`seriko.dpi`〕・バルーンのずらしと揃え方〔`balloon.offsetx`／`offsety`・`balloon.alignment`〕・`seriko.zorder`・デスクトップへの揃え方）は新しいシェルから読み直す。窓の位置（ゴーストごとの記憶）とスコープの数（起動時に決めた数・`kero.*` は読み直さない）は保つ。新しいシェルに在るスコープの面が無いときは、今日の「無い面を指定された」ときの扱いと同じにする。
8. When 差し替えが済む, the areka shall 走っている SERIKO のアニメ（まばたきなど）を新しいシェルの定義で始め直し、古いシェルの定義を残さない。

### Requirement 3: バルーン切替は差し替えのあと `OnBalloonChange` を送る

**Objective:** As a α の利用者, I want バルーンを替えると次の台詞から新しいバルーンで話し、ゴーストにもそれが伝わること, so that 既定バルーンと同梱バルーンを行き来できる

#### Acceptance Criteria

1. When バルーンの切替要求を受け、切替先が決まる, the areka shall 再生中の台詞があればその再生が終わってから（バルーンの表示が終わった時点で）、無ければ直ちに差し替えを行う（要件 12 裁定 2）。
2. When 差し替えが済む, the areka shall 現在のゴーストへ `OnBalloonChange` を GET で送り、Reference を Ref0＝切り替わったバルーンの名前（`descript.txt` の `name`。無ければフォルダ名）・Ref1＝切り替わったバルーンのフォルダの絶対パス、で載せる。応答の台本は通常のトークとして新しいバルーンで再生し、204 なら何もしない。
3. When 差し替えが済む, the areka shall 各スコープのバルーン窓を新しいバルーンの絵と文字の層で組み直し、次の台詞から新しいバルーンの幾何（折り返しの位置・有効矩形・フォントの既定・作者の DPI）で描く。差し替えの時点でまだ見えている古いバルーン（文字を出し終えて時間切れを待っているもの）はその場で隠し、新しいバルーンは差し替えの直後は隠れたままで、次の台詞で今日どおり現れる。
4. When 差し替えが済む, the areka shall 「今のバルーン」（ネットワーク更新の対象の解決が読む場所）を新しいバルーンにする。
5. The areka shall バルーンの切替で SHIORI もシェルも触らない（シェル側の SERIKO のアニメ・面・着せ替え・キャラ窓は不変）。バルーン自身のアニメの定義は新しいバルーンのものへ替える。

### Requirement 4: 差し替えの途中で表示が崩れない

**Objective:** As a α の利用者, I want 切替の瞬間に古い絵と新しい絵が重なったり、窓が空になって点滅したり、クリックが素通りしたりしないこと, so that 切替が「一瞬で替わった」ように見える

#### Acceptance Criteria

1. The areka shall 差し替えを、古い装着を片付けて同じ呼び出しの中で新しい装着を登録し、表示し、窓寸を合わせる 1 手順で行い、その手順を画面への反映の後ではなく配置が決まる前の段に置く（先進坑の学び 2・3）。1 フレーム遅らせて辻褄を合わせる解は取らない。
2. The areka shall 差し替えのどのフレームでも、古い絵と新しい絵が同時に見える・古い絵に新しい当たり判定が乗る・キャラ窓の絵が空になる・透明でない画素のクリックが素通りする、のいずれも起こさない（許すのは先進坑が「床 1」とした反映待ち＝当たり判定と窓寸が絵より画面更新 1 回ぶん先に新しくなる 1 枚だけ）。
3. When 同じ窓に新しい装着を登録し直す, the areka shall 可視性の持ち主（バルーン窓は外から制御する形）と窓寸の要求（適用済み・素の大きさ・保留中の大きさ変更）を古い装着から引き継ぎ、隠れているはずのバルーンを差し替えの直後に見せない（先進坑の学び 5）。
4. The areka shall 新しい資産の読み込みと復号（画像のデコード・アトラスの作成）を、画面を進める処理の中で行わず、その外で済ませてから差し替えの手順へ渡す（UI スレッドを長く塞がない・完了 `ghost-shell-balloon-switch` の申し送り）。差し替えの手順そのものが UI スレッドを止める時間は 1 フレーム（約 16 ms）の内側を目標とし、実機サインオフで測って記録に残す（決定論テストでは固定しない）。
5. Where 画面の拡大率が 1 でない（開発機の既定は 200%）, the areka shall 差し替え後の絵・当たり判定・窓寸を差し替え前と同じ拡大率で出す（拡大率 1 のときと同じ手順で、拡大率だけを引き継ぐ）。
6. The areka shall present に「古い装着を片付ける正規の口」と「登録を消す口」を足し、差し替えがその口を通る（先進坑のように名前で子を探して外から消す形は本番に残さない）。

### Requirement 5: 中止・失敗・終了要求

**Objective:** As a α の利用者, I want 切り替えの台詞の途中でダブルクリックしたら切替がやめになり、途中で失敗しても元の見た目のまま話し続け、終了を選んだら必ず終わること, so that 「止めたのに替わる」「壊れて何も出ない」「切替中に終われない」を出さない

#### Acceptance Criteria

1. While `OnShellChanging` の台詞が再生中である, when 利用者がバルーンの左ダブルクリックで再生を止める, the areka shall シェル切替を中止し、差し替えを行わず、`OnShellChanged` を送らず、`info!` で「切替を中止した」ことを記録に残し、直後の新しい切替要求を受け付ける（完了 `ghost-shell-balloon-switch` 裁定 1 を継ぐ・記憶は書かない）。
2. While `OnShellChanging` の台詞が再生中である, when 中断された台詞が `\-` を含んでいた, the areka shall 終了へ結ばない（切替の相の中断は中止であって終了ではない）。kanade はその台詞を印の付いた依頼の台詞として扱い、中断を切替の中止として知らせる（要件 8.11）。When `OnShellChanging` の台詞が中断されずに別のトークに置き換えられる, the areka shall 台詞の終わりとして扱い、切替を続ける。
3. When 切替が中止される, the areka shall 中断された台詞のバルーンを完了 `balloon-break` と同じ規則で隠す（隠す規則を変えない）。
4. When `OnShellChanging` を送らない切替（`raise-event` 無しの台本・バルーンの切替）の、命令を運んだ台本を利用者が中断する, the areka shall 中断が命令の位置より前なら切替要求が出ていないので何も起きず（0 件）、命令の位置より後なら台詞の終わりとして切替を続ける。ただし中断された台本に `\-` の予約が在れば定常の規則どおり終了が勝ち、保留の切替は捨てる（`info!`）。
5. If 差し替えの途中で失敗する（切替先のシェル／バルーンのフォルダが読めない・`descript.txt` や `surfaces.txt` が解釈できない・画像が復号できない・装着の登録が失敗する）, then the areka shall `error!` を 1 件残し（失敗の種類と理由を含む）、古い装着を残したまま（元のシェル／バルーンのまま）表示と会話を続け、`OnShellChanged`／`OnBalloonChange` を送らず、記憶を書かない。`OnShellChanging` を既に送っていても同じ（切り替え前のイベントだけが届いた形になる）。
6. The areka shall 失敗の判定をできる限り差し替えの手順の**前**（新しい資産の読み込みと復号の段）で行い、古い装着を片付けた後に失敗して窓が空になる形を作らない（片付けた後の失敗は装着の登録だけに限る）。
7. While 切替が進行中である（台詞の切れ目を待っている・`OnShellChanging` の台詞の再生中）, when 終了要求（メニューの「終了」・OS の閉鎖要求・Alt＋F4・OS のセッションの終了）が届く, the areka shall 切替を取りやめて（`info!`）今日の終了経路で終わる（利用者の終了の意思が切替に勝つ）。
8. The areka shall 切替の経路から終了の指示を出す場所を 1 つも持たず、切替の成功・中止・失敗で終了コードを変えない。

### Requirement 6: 選択は記憶され、次回起動でシェルも復元される

**Objective:** As a α の利用者, I want 選んだシェルとバルーンが次に areka を起動したときも出ること, so that 起動のたびに選び直さなくてよい

#### Acceptance Criteria

1. When シェルの差し替えが済む, the areka shall 現在のゴーストの「最後のシェル」の記憶（`LastShell`・Ghost スコープ・フォルダ名）を新しいシェルに書く。When バルーンの差し替えが済む, the areka shall 現在のゴーストの「最後のバルーン」の記憶（`LastBalloon`・Ghost スコープ・フォルダ名）を新しいバルーンに書く。シェルの差し替えは `LastShell` だけを、バルーンの差し替えは `LastBalloon` だけを書き、他の記憶に触らない（3 つを一度に書く `record_last_used` を通すと、バルーンを入れた直後に `remember_balloon` が書いた「次から使うバルーン」を今表示しているバルーンで上書きしてしまうため、1 つだけを書く口を既存の書き手の隣に置く）。実行系が動いている間の記憶の書き込みの決まり（`sylphya_publisher` を通す）を守る。書けないときは `warn!` を残して切替は成功として扱う（記憶の縮退は今日と同じ）。
2. When ゴーストを起こす（初回の起動・ゴースト切替の切替先・更新後の読み直し）, the areka shall そのゴーストの「最後のシェル」の記憶を読み、`shell/<記憶の名>/` に `descript.txt` が実在すればそのシェルで起こす（`menu,hidden` のシェルでも可）。記憶が無ければ今日どおり既定のシェル（`seriko.defaultsurfacedirectoryname`、無ければ `master`）。
3. If 「最後のシェル」の記憶が指すフォルダが実在しないか `descript.txt` を持たない, then the areka shall `warn!` を 1 件残して既定のシェルで起こし、その起動の成功で記憶を既定のシェルへ書き直す（次回から警告が出ない）。
4. The areka shall 起動時に選んだシェル名を、シェルを決める解決のすべての本番の呼び出し点（実行系の起動・資産の組み立て・配置の情報源）へ運び、どの呼び出し点も同じシェルを見る（片方だけ既定のシェルを見る形を作らない）。運ぶ口は `Emo2BootInputs` の欄と `boot_with_origin` の引数で、`GhostBootOptions` と `ConfigInputs`・`CurrentGhost`・`GhostDecision`・`BalloonDecision` には欄を足さない（完了 `ghost-shell-balloon-switch` の約束 1）。
5. When 切替のあとにネットワーク更新の読み直し（同じフォルダのゴースト切替）が走る, the areka shall 差し替えたシェルとバルーンで起こし直す（記憶から復元される）。When 切替のあとにメニューの「ネットワーク更新」の可否と対象を判定する, the areka shall 差し替えた後のシェル・バルーンを対象にする（更新の窓口が読む「今のシェル」「今のバルーン」が差し替え後を指す）。
6. The areka shall `OnBoot`／`OnGhostChanged` に載せるシェル名（`OnBoot` の Ref0・`OnGhostChanged` の Ref7）を起動時に選んだシェルにする（起動の解決がそのシェルでマウントすれば自動で追随する）。
7. The areka shall 起動時の argv（`areka.exe <ゴーストの根> <バルーンの根>`）の第 2 引数の意味を変えない（argv でバルーンを指定した起動では今日どおりそのバルーンで起き、起動の成功で argv のバルーンを記憶に書かない、のまま）。argv でバルーンを指定して起きたプロセスでも、利用者がバルーンを切り替えたら、その選択は 6.1 のとおり `LastBalloon` に書く（切替は利用者の明示の選択であって argv ではない）。

### Requirement 7: メニューの「シェル」「バルーン」枠

**Objective:** As a α の利用者, I want 右クリックメニューの「シェル」「バルーン」に候補が並び、選ぶと替わること, so that 台本を書かなくても見た目を替えられる

#### Acceptance Criteria

1. When メニューの「シェル」枠を組み立てる, the areka shall 現在のゴーストの隠しでないシェル（`list_shells`）を `descript.txt` の `name`（無ければフォルダ名）で目録の並びのまま列挙し、現在のシェルも含める。候補が 1 つだけのときも枠は出す。
2. When メニューの「バルーン」枠を組み立てる, the areka shall 目録のバルーン（`list_balloons`）を `descript.txt` の `name`（無ければフォルダ名）で目録の並びのまま列挙し、現在のバルーンも含める。候補が 1 つだけのときも枠は出す。
3. When 利用者が枠の 1 つを選ぶ, the areka shall 要件 1.5 の切替要求を入口へ出す（メニューは要求を出すだけで、切替の可否や順序を自分で判断しない）。
4. When ゴーストを起こす（初回の起動でも切替後でも）, the areka shall 「シェル」「バルーン」枠の登記をそのゴーストのメニューの登記に対してやり直す（完了 `ghost-restart-unit` の契約・`ghost_frame`／`install_frame`／`update_frame` と同じ形）。
5. The areka shall 枠の項目名の SHIORI リソース照会（`shellrootbutton.caption`・`balloonrootbutton.caption`）と既定名（「シェル」「バルーン」）・メニューの見た目や操作の作法（OS ネイティブ）を変えない。

### Requirement 8: 既存の振る舞いと境界を守る

**Objective:** As a 開発者, I want 本仕様がゴースト切替・終了経路・更新の結線の形を変えず、上限に近いファイルを増やさないこと, so that 直前に着地した spec の契約と後続 spec の前提が崩れない

#### Acceptance Criteria

1. The 本仕様 shall `crates/areka/src/emo2_boot/ghost_switch.rs`（と `ghost_switch_tests.rs`・`ghost_switch_test_support.rs`）・`crates/areka-parsers/src/sakura/`・`crates/areka-sakura/src/compile.rs`・`crates/areka-ghost/src/lib.rs`（`pub mod` 1 行を除く）に触らず、`SwitchRequest`／`GhostSpec`／`request_ghost_switch` の形を変えない（呼ぶだけ・`boot_event: None`）。
2. The 本仕様 shall 送出の許可表 `ALLOWED_EVENT_IDS` に `OnShellChanging`・`OnShellChanged`・`OnBalloonChange` の 3 語を足し（42 → 45）、件数の直書き 2 か所（`events_change_tests.rs` の `assert_eq!` と `events_tests.rs` の完全一致のテスト名・配列）を追随させる。kanade で触るのは許可表とそのテスト、および要件 8.11 の「切替の台詞」の口だけとする（要件 12 裁定 8）。
3. The 本仕様 shall 汎用の通知の入口の判断（許可表・定常か否か・応答の置き換え）を、印の無い依頼については変えない（印の無い依頼の振る舞いと既存の決定論テストは不変）。
4. The 本仕様 shall `GhostBootOptions` に欄を足さず（構造体リテラル 31 か所・20 ファイル）、目録の素性（`catalog::Identity`）に欄を足さず、`ConfigInputs`・`CurrentGhost`・`GhostDecision`・`BalloonDecision` に欄を足さない。
5. The 本仕様 shall ゴースト切替・終了操作・終了コード・ネットワーク更新の読み直し・インストール後の振る舞い（自動で切り替えない）を変えず、それらの決定論テストを 1 本も落とさない（本番ソースの字面で形を固定しているテストは新しい字面へ追随させ、削除しない）。
6. The 本仕様 shall 同期の送信（`SendMessageW`／`SendMessageTimeoutW`）を本番ソースに足さない（`ALLOWED_SYNC_SENDS` は 2 行のまま）。
7. The 本仕様 shall 本番コードが読む環境変数と依存クレートを新しく足さない。
8. The 本仕様 shall 触るファイルすべてを 1 ファイル 1,000 行の目安の内側に収める。`resolve.rs`（952）には関数を足さず、シェル名つきの解決は隣の新ファイルに置いて `resolve` はそれに委ねる（既存の呼び出し約 30 本を直さない）。上限に近いテスト（`runtime_tests.rs`・`assets_tests.rs`・`actor_tests.rs`・`schedule_tests.rs`・`ghost_switch_tests.rs`）には行を足さず、兄弟の新ファイルへ置く。`steady.rs`（935）は触らない。
9. The 本仕様 shall 失敗の経路に記録の無いものを作らない（無視は `warn!`・失敗は `error!`・中止は `info!`・受理と完了は `info!`）。
10. The 本仕様 shall `session_mark_verdict`（きれいな終わりの判定）と終了の出所（`ExitOrigin`）を触らず、起動中の印を別の場所で消さない（切替の失敗は元のまま続けるので終了の出所を足す理由が無い）。
11. The 本仕様 shall kanade の汎用の通知の入口に「この依頼の台詞は切替の台詞である」という印を付けられる口を足し、印の付いた依頼について次を満たす（要件 12 裁定 8）: ⑴ その依頼が始めた台詞が終わったとき、最後まで再生されたか・利用者の中断で止まったか・別のトークに置き換えられたかを、依頼した側へ 1 回だけ知らせる（台詞が無い〔204〕ときは今日の返信のとおり）、⑵ その台詞が利用者の中断で止まったときは、台本に `\-` の予約が在っても終了系列へ進まない（ゴースト切替の相の中断と同じ扱い＝完了 `ghost-shell-balloon-switch` 衝突表 2 を継ぐ）。本仕様で印を付けるのは `OnShellChanging` だけとする。

### Requirement 9: 2 つ目のシェルを持つ検体

**Objective:** As a 開発者, I want テストと実機で「同じゴーストの別シェル」へ往復できる検体が在ること, so that シェル切替を赤にできる

#### Acceptance Criteria

1. The 本仕様 shall 検体 `.nar` を畳み直さず（`vendors/sample_ghost/*.nar` と `SAMPLES` の 7 件・README の表は不変）、テストでは展開先の複製に対して `shell/master/` を写して別の名前（`descript.txt` の `name` も変える）のシェルを足す部品を `sample-ghost-kit`（テスト専用）に 1 つ持ち、決定論の統合テストはそれで 2 シェルの検体を組む（要件 12 裁定 1）。
2. The 本仕様 shall 実機サインオフでは `nar-sample-path` が作る `manual/R_POST_and_KOMAINU/` の下に同じ手順で 2 つ目のシェルを置き（置き場はワークツリーの `target\` の下・`C:\` 直下には作らない）、その手順を `signoff.md` に残す。
3. The 本仕様 shall `konnoyayame` のシェル（CC BY-NC-ND）を写さず・改変せず、配布物にも入れない。

### Requirement 10: 網羅台帳と裁量の記録

**Objective:** As a 開発者, I want 実装した語彙が台帳に映り、areka の裁量が §8 に残ること, so that 「書いてあるのに何も起きない」の一覧から切替が外れ、裁定の根拠が後から追える

#### Acceptance Criteria

1. The 本仕様 shall `doc/ukadoc-coverage/ledger/shiori.toml` の `OnShellChanging`／`OnShellChanged`／`OnBalloonChange` と `sakura-script.toml` の `\![change,shell,シェル名(,--option=raise-event)]`・`\![change,balloon,バルーン名]` を実装済みへ更新し（owner＝本仕様・備考にシェルの `lastinstalled` は原文の「ゴースト」を誤記と読んで最後に入れたシェルへ切り替える旨と `sequential` は正典に無い旨を記す）、`shellrootbutton.caption`・`balloonrootbutton.caption` の備考の引受先を本仕様に改めて登記済みの枠の一覧も今の形（ゴースト・インストール・更新・シェル・バルーン・説明書・終了）へ直し、生成物を生成器で作り直す（手で直さない）。
2. The 本仕様 shall `doc/COMPAT_ARCHITECTURE.md` §8 に次を 1 行ずつ記す: (a) シェル名の `lastinstalled` は原文の「最後にインストールしたゴーストに切り替え」を誤記と読み、プロセスの中で最後に入れたシェル（今のゴーストのもの）へ切り替える、(b) 差し替えは台詞の切れ目で行い、表示中の文字と残りの台本には触れない（`OnShellChanging` の台詞・命令を運んだ台本の終わりを待つ）、(c) 自分自身への切替は無視せず作り直す（読み直しの代用）、(d) `random` は現在のものを除いて選び、候補 0 なら現在のもの・隠しシェルは `random` の候補に入れない、(e) 隠しシェルは名指しなら切り替えられる、(f) `OnShellChanged` の Ref1 は SSP の意味（現在のゴースト名）、(g) 差し替え後は同じ面番号を新しいシェルで出し、着せ替えは新しいシェルの既定へ戻す、(h) 途中の失敗は元のまま続け、切替失敗のイベントは作らない、(i) シェルの差し替えではシェルの `descript.txt` の見た目の値を読み直し、窓の位置とスコープの数は保つ（要件 2.7）、(j) 差し替えの時点は台詞の文字を出し終えた時点で、バルーンの差し替えでは残っている古いバルーンをその場で隠す（要件 2.2・3.3）。各行に正典の沈黙の根拠と裁定の日付を付ける。
3. The 本仕様 shall `crates/areka/src/menu/mod.rs`・`consumer_ledger.rs`・`change_cue.rs` の「シェル・バルーンは別の spec」と書く doc コメントを実装後の形に合わせて改める。

### Requirement 11: 決定論テストと実機確認

**Objective:** As a 開発者, I want 切替の判断の分岐が偽のシェル 2 つ・偽のバルーン 2 つで赤にでき、実機で 1 周していること, so that 後の変更で「止めたのに替わる」「重なる」「次の起動で戻る」に戻ったら赤になる

#### Acceptance Criteria

1. The 本仕様 shall 偽の SHIORI（x64 の偽境界）と偽のシェル 2 つで「A で起きる → `\![change,shell,B,--option=raise-event]` → `OnShellChanging`（Ref0〜2 を突き合わせる・台本あり）→ 再生完了 → 差し替え → `OnShellChanged`（Ref0〜2）→ 記憶 `LastShell`＝B」を同じプロセス・同じ World で往復（B → A も）する決定論テストを 1 本持ち、⑴ イベント列と Reference、⑵ SHIORI を降ろしていない（`OnClose`・`OnBoot` 0 件）、⑶ 装着の子が古い分だけ消えて新しい分だけ在る、⑷ 可視性の持ち主と窓寸の要求が引き継がれている、を集めてから 1 回で判定する。
2. The 本仕様 shall 偽のバルーン 2 つで「`\![change,balloon,Y]` → 台詞の終わり → 差し替え → `OnBalloonChange`（Ref0〜1）→ 記憶 `LastBalloon`＝Y → 次の台詞が新しいバルーンの幾何で組まれる」の往復を同じ形で固定する。
3. The 本仕様 shall `raise-event` 無しのシェル切替で `OnShellChanging` が 0 件・命令を運んだ台本の終わりの後に差し替わること、メニューからのシェル切替で `OnShellChanging` が送られること、バルーンの切替で「切り替え前」のイベントが 0 件であることを固定する（要件 1.2〜1.5・2.3）。
4. The 本仕様 shall `OnShellChanging` の台詞をバルーンブレークで止めると中止され、差し替えも `OnShellChanged` も記憶も無く、`\-` が終了に結ばれないことを固定する（要件 5.1〜5.3）。kanade 単体でも、印の付いた依頼の台詞の終わり方 3 通り（再生し切った・中断・置き換え）の知らせと、中断＋`\-` で終了系列へ進まないこと、印の無い依頼では今日どおり終了系列へ進むことを固定する（要件 8.11・兄弟の新しいテストファイルへ）。
5. The 本仕様 shall 該当なし（未知の名前）で `warn!` 1 件・イベント 0 件、隠しシェルへの名指しの切替が通ること、`random` が現在のものを除いて選ぶこと（候補 0 なら現在のもの）、バルーンの `lastinstalled`（控えあり・控えなし）、シェルの `lastinstalled`（控えあり・控えなし・控えが別のゴーストのシェル＝該当なし、いずれもゴースト切替は 0 件）、自分自身への切替で作り直されること、進行中の二重要求・ゴースト切替中・更新中・定常以外での `warn!` 1 件、シェル切替の進行中に来たゴースト切替が通りシェル切替が差し替えの前に取りやめられること（`info!` 1 件・差し替えとイベントと記憶が 0）、をそれぞれ固定する（要件 1.6〜1.14）。
6. The 本仕様 shall 差し替えの途中の失敗（読めないフォルダ・復号できない画像・登録の失敗）で `error!` 1 件・元の装着が残る・イベントと記憶が 0 であること、切替の進行中に終了要求が届くと切替が捨てられて今日の終了経路で終わることを固定する（要件 5.5〜5.7）。
7. The 本仕様 shall 起動時の復元＝記憶の名のシェルで起きる（隠しシェルでも）・記憶が無ければ既定・記憶の先が実在しなければ `warn!` 1 件と既定と記憶の書き直し・4 か所の解決が同じシェルを見る（資産・配置の情報源・実行系の `mount().shell.dir`・`OnBoot` の Ref0）を固定する（要件 6.2〜6.4・6.6）。
8. The 本仕様 shall 切替後の更新の対象の解決が差し替え後のシェル・バルーンを指すこと（完了 `network-update` の `current_three_resolve_dirs_names_and_fallback_homeurls`・`resolve_targets_reads_the_slot_and_the_boot_context` の兄弟で）と、更新後の読み直しで差し替えたシェル・バルーンが保たれること（同 `a_reload_of_the_running_ghost_boots_with_the_tail_head_and_sends_the_rest_after_the_switch` が緑のまま、かつ記憶からの復元の分岐を 1 本）を固定する（要件 6.5）。
9. The 本仕様 shall present の片付けの口と再登録の引き継ぎ（可視性の持ち主・窓寸の要求）を present 単体の決定論テストで固定し（要件 4.3・4.6）、メニューの 2 枠の登記（並び・現在を含む・1 つでも出す・選ぶと要求が出る・起こすたびにやり直す）を固定する（要件 7）。
10. The 本仕様 shall 足すテストを上の判断の分岐に限り、既に確かめられている配線（起動・終了の握手・ゴースト切替・窓の配置・汎用の入口）を再テストしない。既存の決定論テストは 1 本も落とさない。
11. When 実機で確認する, the 開発者 shall ① `R_POST_and_KOMAINU` を起こし、メニューの「シェル」枠から 2 つ目のシェルを選び、`dic02_Event.txt` の `OnShellChanging` の台詞のあとに絵が替わり `OnShellChanged` の台詞が出ること、② 元のシェルへ戻れること（往復 1 周）、③ ①の台詞の途中でバルーンをダブルクリックすると切替がやめになり元の絵が残ること、④ メニューの「バルーン」枠から emo2 同梱バルーン（`emo2-kakukaku`）を選び、次の台詞が新しいバルーンで出て既定バルーン（`StayseeBalloon`）へ戻れること、⑤ ①④のあとに終了して再起動すると 2 つ目のシェルと選んだバルーンで起きること（記憶）、⑥ 画面の拡大率 200%（開発機の既定）で①④の絵・当たり判定・窓寸が崩れないこと、⑦ emo2 で `\![change,balloon,…]` の台本から替わり `update.pasta` の `OnBalloonChange` の応答が出ること、を有界の自動終了つきで見る。`RUST_LOG` は判定の分岐（切替の受理・切替先の決定・差し替え・中止・記憶）の水準まで開ける。差し替えの手順が UI スレッドを止めた時間と、目視で崩れたフレームの有無を記録する（要件 4.4）。
12. The 本仕様 shall 実機の走行の結果（コマンド・終了コード・目印の件数・止まった時間・2 つ目のシェルの作り方）を spec の文書（`signoff.md`）に残す。

### Requirement 12: 裁定（2026-09-30 要件ディスカッションで確定）

**Objective:** As a 開発者, I want brief が挙げた議題 ⑴・⑶・⑷ と、要件を書く途中で答えが要った 4 点を要件の段階で一旦決めておくこと, so that ギャップ分析と設計がこの形で進み、覆すなら要件で覆す

#### Acceptance Criteria

1. The 本仕様 shall **裁定 1（2 つ目のシェルの検体は畳み直さず、テストのときに写す・brief 議題 ⑴）**として、検体 `.nar` は不変のまま、展開先の複製に `shell/master/` を写して別名のシェルを足す部品を `sample-ghost-kit` に置き、実機は同じ手順で `target\` の下に置く（要件 9）。根拠: `R_POST_and_KOMAINU.nar` はリポジトリで畳んだ 4 本の 1 つだが README にライセンスの登記が無く、改変してよいかを確かめる作業が要る。`konnoyayame` は CC BY-NC-ND で不可。`claudia` は Unlicense で畳み直せるが、`.nar`・README の表・`SAMPLES` の数・`lib_tests` の直書きが動き、揃える利得が無い（README「揃える利得が無い」）。写す形なら `.nar` に触らず、どの検体にも使え、`SampleRoot` の複製は札ファイルを閉じれば消える。**別案**（`.nar` を畳み直す）は要望が出てから。
2. The 本仕様 shall **裁定 2（差し替えは台詞の切れ目で行い、表示中の文字と残りの台本には触れない・brief 議題 ⑶）**として、`raise-event` の切替は `OnShellChanging` の台詞の再生が終わってから、それ以外の切替は命令を運んだ台本の再生（中断で早く終わった場合を含む）が終わってから、メニューからの切替も再生中の台詞があればその終わりを待ってから差し替える（要件 2.2・2.3・3.1）。根拠: 完了 `ghost-shell-balloon-switch` 要件 2.6「命令を運んだ台本の再生が終わってから降ろす」と同じ形で、汎用の入口の性質（応答が再生中のトークを置き換える）と噛み合う＝差し替えの時点でバルーンは隠れており「文字を引き継ぐか消すか」の問いが立たない。バルーンの文字の層を差し替える最中に文字を描く形（引き継ぐ）は 1 フレームも崩れない保証を難しくし、途中で消す形（捨てる）は「言いかけたことが消える」を出す。正典は切替の時点に沈黙する。**別案**（命令の位置で即座に差し替え、残りの台本を新しいバルーンで続ける）は、シェル切替なら残りの台本を新しい面で続けられるが、バルーン切替では文字の層の作り直しの最中に残りの文字を置く仕事が増えるので採らない。
3. The 本仕様 shall **裁定 3（シェル名の `lastinstalled` は最後に入れたシェル・原文の「ゴースト」は誤記と読む・brief 議題 ⑷・2026-09-30 要件ディスカッション議題 3・開発者確定）**として、`\![change,shell,lastinstalled]` はこのプロセスの中で最後に入れたシェル（今のゴーストのもの）へ切り替え、`\![change,balloon,lastinstalled]` は最後に入れたバルーンへ切り替える（要件 1.10・1.11）。どちらもゴースト切替は起こさない。根拠: ukadoc の `\![change,shell,…]` だけが「最後にインストールした**ゴースト**に切り替え」と書くが、ukadoc で `lastinstalled` を受ける他の項目（`\![change,balloon,…]`・`\![change,calendarskin,…]`・`\![execute,calendarplugin,…]`・`\![execute,headline,…]`・`\![raiseplugin,…]`・`\![call,ghost,…]`・`\![change,ghost,…]`）はどれも命令と同じ種類の「最後に入れたもの」を指すので、ゴースト切替の説明を写したときの誤記と読む（開発者確定）。原文どおりに読むと、着替えのつもりの台本がゴーストを入れ替えてしまう。控えは `LastInstalledGhost` と同じ形の資源で、書き手はインストールの完了時（`install/desk.rs` の `record_installed` のシェルとバルーンの腕に 1 行ずつ）。
4. The 本仕様 shall **裁定 4（`random` の規則）**として、現在のものを除いて 1 つを無作為に選び、候補が 0 なら現在のもの（＝作り直し）にし、隠しシェルは候補に入れない（要件 1.9）。根拠: ゴースト名の `random` と同じ規則（完了 `ghost-change-name-resolution`＝「今のものを除いて 1 つ選ぶ・候補 0 なら今のもの」）に揃え、乱数の取り方（`boot_resolve.rs` の `pick_index`）も共用する。隠しシェルは「メニューに出さない」意思の表れなので無作為の候補からも外す（名指しは可）。
5. The 本仕様 shall **裁定 5（自分自身への切替は作り直す）**として、切替先が現在のシェル／バルーン自身でも無視せず同じ経路で作り直し、`OnShellChanged`／`OnBalloonChange` を送る（要件 1.8）。根拠: 完了 `ghost-shell-balloon-switch` 裁定 9（自分自身へのゴースト切替は降ろして起こし直す）と同じで、α 後の `\![reload,shell|balloon]`（`mcp-reload`）の代用になり、経路に例外を作らない。
6. The 本仕様 shall **裁定 6（差し替え後の面と着せ替え）**として、差し替えの直前と同じ面番号を新しいシェルで表示し（無ければ今日の「無い面」の扱い）、着せ替えの状態は新しいシェルの `descript.txt` の既定に戻す（要件 2.6）。根拠: 正典は沈黙する。面番号はゴーストの台本が持つ意味（表情）なので保ち、着せ替えの区分はシェルごとの定義なので持ち越せない。多くのゴーストは `OnShellChanged` の台本で面を出し直すので、どちらでも作者の意図が勝つ。
7. The 本仕様 shall **裁定 7（途中の失敗は元のまま・告知なし・失敗のイベントなし）**として、差し替えの途中で失敗したら `error!` を残して元のシェル／バルーンのまま続け、メッセージボックスも新しい SHIORI イベントも出さない（要件 5.5）。根拠: 開発者裁定「メッセージボックスは無粋・失敗はゴーストの台詞で伝える」（2026-09-26）と「1 体の失敗はアプリの失敗ではない」（2026-09-24）。ukadoc に切替失敗のイベントは無く、areka 独自のイベントを作っても受ける辞書がどのゴーストにも無い。
8. The 本仕様 shall **裁定 8（kanade に「切替の台詞」の口を足す・2026-09-30 要件ディスカッション議題 1・開発者確定）**として、汎用の通知の入口に印を付けられる口を足し、印の付いた台詞の終わり方を依頼した側へ知らせ、その中断を終了に結ばない（要件 5.2・8.11）。根拠: 汎用の入口の返信（`RaiseOutcome::Script`）は台詞の開始までしか知らせず、kanade の `on_talk_done` は定常のトークの中断に `\-` の予約があれば終了系列へ進む（例外はゴースト切替の相だけ）。kanade に触らず UI が推し量る案は、`\-` 入りの台詞の中断でアプリが終わり、別のイベントの応答が台詞を置き換えたときに判断を誤るので採らない。開発者の指示「並走 spec が無いのでスコープを広げる弊害は無い・目的の実現を優先せよ」。「台詞が終わってから次へ進む」口は α 後の `network-update-canon-order` でも要る（同根）。
9. The 本仕様 shall **裁定 9（シェル・バルーンの切替の最中に来たゴースト切替が勝つ・2026-09-30 要件ディスカッション議題 2・開発者確定）**として、ゴースト切替を断らず、進行中のシェル・バルーンの切替が差し替えの前に自分を取りやめる（要件 1.12）。根拠: ゴースト切替を断ると、利用者がメニューで選んだ切替が黙って消える。ゴースト切替は全部を作り直すので、途中のシェル切替を続ける意味が無い。kanade は定常で再生中のゴースト切替を台詞の後へ積むので、`\![change,shell,B]\![change,ghost,X]` のような並びでも答えが 1 通りに決まる。ゴースト切替の入口（`ghost_switch.rs`）は変えずに済む。
10. Where 設計・実装の途中で裁定 1〜9 のいずれかを覆す必要が判明する, the 本仕様 shall 開発者へ議題として上げ、確定を待ってから要件・設計・`doc/COMPAT_ARCHITECTURE.md` §8 の該当箇所を改める。
