# Brief: areka-P0-shell-balloon-switch

> 2026-09-20 `/kiro-discovery` 再入（棚卸⑮）で起票。`areka-P0-ghost-shell-balloon-switch`（規模 L）の Approach ②③「バルーン切替・シェル切替」を**単独の spec として切り出した**。名前は `doc/ukadoc-coverage/roadmap-draft.md` 段階 B 順位 2 の束「切替」の候補名そのものである。
> 正典の語彙と Ref の一覧は親 brief（`.kiro/specs/completed/areka-P0-ghost-shell-balloon-switch/brief.md` の Desired Outcome 2・3）が正本。本 brief は**再測定で崩れた前提と、切り出したあとの境界**だけを書く。
> 本文の file:line は**起票時の実測値**（2026-09-20・main `fe157df1`）。着手時に必ず引き直すこと。

## 2026-09-30 `network-update` からの申し送り（要件 8.8）

1. **許可表の件数**: `schedule/events.rs` の `ALLOWED_EVENT_IDS` は **42 件**（`file-drop` の 23 に、`network-update` が更新の 19 語を足した）。下の `file-drop` の申し送り 1 の「23 件・3 件足して 26」は古い＝本仕様が 3 件足すなら **45**。直書きは `events_change_tests.rs` の `assert_eq!(ALLOWED_EVENT_IDS.len(), 42)` と `events_tests.rs` の `allowed_event_ids_are_exactly_the_forty_two_and_exclude_ontalk_onhour`（42 語の完全一致）の 2 か所。リソースの許可表 `schedule/resources.rs` の `ALLOWED_RESOURCE_IDS` は **12 語**（`homeurl`・`useorigin1` を足した）で、同じファイルの `allowed_resource_ids_are_exactly_the_twelve_names` が固定する
2. **「今のシェル・今のバルーン」を切替後の物へ読み替える**（要件 1.4 の 3 つ）: 更新の対象の解決 `crates/areka/src/update/desk.rs` の `resolve_targets`（中身は同じファイルの `here`＝World から写すだけ・`resolve`＝フォルダを読む）は、今のシェルを `GhostSession::runtime().mount().shell.dir`（起動時に解いたシェル）、今のバルーンを `BootContext.current.balloon.dir`（起動時に解いたバルーン）から取っている。メニューの「ネットワーク更新」を選べるかの判定 `can_update` も同じ `here` を通る。本仕様が実行中にシェル・バルーンを差し替えるなら、この 2 つが差し替えた後の物を指すようにする（差し替えの相が `mount()` と `BootContext` を書き換えるなら手当て 0、別の場所に持つなら `here` を読み替える）。判定は `crates/areka/src/update/desk_resolve_tests.rs` の `current_three_resolve_dirs_names_and_fallback_homeurls`・`resolve_targets_reads_the_slot_and_the_boot_context`
3. **共有 5 本の今の行数と中身**: 消費者台帳 `consumer_ledger.rs` の `canonical()` は **13 行**（`updatebymyself`・`update`・`updateother` を選別子なしで `CommandConsumer::UpdateSink` に登記。`update` は第 1 引数によらず更新の受け口が担当するので、本仕様が `\![update,…]` を別の担当へ回すことは無い）。`wire_emo2_boot` の受け口の列は **10 本**（10 本目が `UpdateCueSink`）。メニューは `Frame::Update` に `menu::update_frame::register` を `ghost_session.rs` の `boot_wired` から登記する（起こすたびにやり直す）
4. **読み直しは既存の切替の入口を「同じフォルダ・知らせなし」で呼ぶ**: 更新で何か変わると `desk.rs` の `reload` が `request_ghost_switch(world, SwitchRequest { ghost: GhostSpec::Folder(今のフォルダ), raise_event: false, origin: ChangeOrigin::Automatic })` を 1 回呼び、ゴースト・シェル・バルーンを起動時と同じ解き方で全部読み直す。本仕様が切替の入口や `boot_into` の解き方を変えるなら、更新の後の読み直しで差し替えたシェル・バルーンが保たれるかを確かめる（判定は `desk_reload_tests.rs` の `a_reload_of_the_running_ghost_switches_to_itself_silently_and_ignores_a_midway_switch`）

## 2026-09-29 `file-drop` からの申し送り（完了時）

1. **許可表の件数**: `schedule/events.rs` の `ALLOWED_EVENT_IDS` は **23 件**（`ghost-install` が 13 → 21、`file-drop` が `OnFileDrop2`・`OnDirectoryDrop` を足して 23）。下の棚卸⑲ 1 の「13 件・3 件足して 16」は古い＝本仕様が 3 件足すなら 26。直書きは `events_change_tests.rs` の `assert_eq!(ALLOWED_EVENT_IDS.len(), 23)` と `events_tests.rs` の 23 語の完全一致の 2 か所
2. **`ghost_session.rs` の窓を作る閉包に 1 行増えた**: `attach_balloon_pointer_handlers(world)` の直後に `input_events::file_drop::attach_file_drop_receivers(world)`（ゴースト窓に投げ込みの受け手を差す）。起こし直しも同じ閉包を通る。ゴースト窓の様式 `placement/spawn.rs` の `window_style` は `WS_EX_ACCEPTFILES` を含む 3 ビットになった（`spawn_assembly_tests.rs` の `t_i2_no_window_has_ws_ex_topmost` が固定）

## 2026-09-28 棚卸⑲の再測定（main `10a8d724`）

> `ghost-change-name-resolution`（PR#194）・`session-mark-residue`（PR#195）の着地後に、末尾の「2026-09-27 棚卸⑱の再測定」節の file:line・数と突き合わせた。**ここに書いたものだけが変わった**。書いていない数は実物どおり（`emo2_boot/mod.rs` 805・`frame.rs` 509・`consumer_ledger.rs` 756・`boot_resolve.rs` 471・`boot_config.rs` 357・kanade `msg.rs` 880・`actor.rs` 589・`schedule/mod.rs` 830・`schedule/events.rs` 535・`steady.rs` 935・`presenter/hub.rs` 176・`catalog.rs` 324・`menu/ghost_frame.rs` 73。kanade の `schedule/` と `msg.rs`、`menu/`、`change_cue.rs`、`consumer_ledger.rs`、`sample-ghost-kit` は 2 本とも 0 行）。

1. **許可表の件数は不変**: `schedule/events.rs` の `ALLOWED_EVENT_IDS` は 13 件、`events_change_tests.rs` の直書きも `assert_eq!(ALLOWED_EVENT_IDS.len(), 13)` のまま。本仕様が 3 件足して 16 にする見込みも変わらない。`OnShellChang|OnBalloonChange` は製品コードで 0 のまま。
2. **行数**: `ghost_session.rs` 638 → **691**（`GhostSession` に欄 `logsink_fallback` と `shutdown_within` が入った）・`main.rs` 873 → **927**（下の 4）・`areka-ghost/src/runtime.rs` 711 → **739**（`GhostRuntime::shutdown_within` と欄 `shiori_probe`。**`boot_with_origin` の引数は不変**＝シェル名の引数を足す場所は⑱ 項目 4 のまま）・`ghost_switch.rs` 699 → **866**・`ghost_switch_tests.rs` 545 → **987**（上限まで 13 行。本仕様は触らないので影響は無いが、`lastinstalled` の往復のテストを足すなら兄弟の新ファイルへ）。
3. **「重ねないための 3 つの約束」は並走の条件としては役目を終えた**（`ghost-change-name-resolution` は着地済み）。実際に触ったのは `ghost_switch.rs`・`ghost_switch_tests.rs`・`ghost_switch_test_support.rs`・`areka-parsers/src/sakura/decode.rs`・`decode_tests.rs`・`parse_bare_tag_tests.rs`（と台帳・生成物・§8）で、`areka-ghost/src/lib.rs` と `areka-sakura/src/compile.rs` は 0。約束 1・2（`ConfigInputs` ほかに欄を足さない・`SwitchRequest`／`GhostSpec`／`request_ghost_switch` の形を変えない）は設計の方針としてそのまま残す。本番の構造体リテラルは `ConfigInputs` 2・`CurrentGhost` 2・`GhostDecision` **4**（⑱ の「5」は数え違いで、2 本の着地の前後とも 4）。
4. **`session_mark_verdict`（`main.rs`）の形が変わった**: 引数が `(first, argv_session, run_ok, down_ok)` から `(first, argv_session, logsink_fallback, end: Teardown { run_ok, down_ok, shiori_cut })` へ。段は「argv → 最初の起動が LogSink へ倒れた → 出所なし → 出所が失敗 → `run()` の失敗 → OS のセッションの終了で SHIORI の待ちを上限で打ち切った → 降ろす処理の失敗 → 消す」。`ExitOrigin` の腕は不変。本仕様は終了の出所を足さないので触らない（⑱ 項目 7 のまま）。
5. **議題 ⑷ の材料が変わった**: ゴースト名の `random`／`sequential`／`lastinstalled` は `request_ghost_switch` の中で解く（`GhostSpec::Name` のときだけ・完全一致・大文字小文字を区別・`GhostSpec::Folder` は素通し）。シェル名・バルーン名の `lastinstalled` を「最後に入れたゴーストへの切替」と読むなら、本仕様は `request_ghost_switch(world, SwitchRequest { ghost: GhostSpec::Name("lastinstalled".into()), .. })` を**呼ぶだけ**で済む（解けないときの `warn!(ghost_switch_unknown)` の `reason=lastinstalled_none`／`lastinstalled_missing` も入口が出す）。記録 `LastInstalledGhost` はプロセスの中だけで、書き手 `record_last_installed` は本番の呼び手が 0（`#[allow(dead_code)]`・`ghost-install` が付ける＝B5 が先なら本仕様の着手時には在る）。一方、**解き手 `resolve_special_name` は `&[GhostEntry]` を受けるゴースト専用**で、シェル・バルーンの `random` には使えない。入れるなら `ShellEntry`／`BalloonEntry` 用に同じ規則（今のものを除いて 1 つ選ぶ・候補 0 なら今のもの）を別に書く（乱数は `boot_resolve.rs` の `pick_index` を共用できる）。
6. **⑱ の申し送り「`signoff.md` 気付き 8」は確かめ済みになった**: OS のセッションの終了の後始末の join の間に UI の窓へ同期の送信が来ても止まらないことを `session-mark-residue` が確かめた（理由は `session_end.rs` の `on_os_session_end` の説明）。**新しい決まり**: `crates/areka/src/session_end_sync_send_tests.rs` の `production_sync_sends_match_the_allowed_table` が、`crates/*/src` の本番ソースの `SendMessageW(`／`SendMessageTimeoutW(` を許可表 `ALLOWED_SYNC_SENDS`（2 行＝`shiori-host32-ipc/src/lib.rs`・`shiori-host32-host/src/parent_window.rs`）と突き合わせる。本仕様が同期の送信を 1 つでも足すと赤になる（足すなら表に 1 行と理由）。資産の復号を tick の外へ出す結論は変わらない。
7. **`GhostBootOptions {` は 31 か所・20 ファイル**（29・18 から、`ghost_session.rs` の `boot_ghost` と `areka-ghost/src/runtime_within_tests.rs` の 2 つが増えた）。欄を足さない方針は同じ。

**規模**: 15〜18 のまま（5 の `lastinstalled` は呼ぶだけで済むが、`random` の分は残る）。**Fable**: 要る（据え置き。議題 ⑷ で「ゴーストへ回す」読み方を取れば作業は小さくなるが、読み方そのものが開発者の決めごと）。

## 2026-09-26 棚卸⑰の再測定（main `13b72893`＝`shiori-fault-notice`・`ghost-restart-unit`・`pilot-balloon-asset-swap` の着地後）

**並走不可のまま・`ghost-shell-balloon-switch` の直後**。`ghost-shell-balloon-switch` と共有するソースが `emo2_boot/mod.rs`・`consumer_ledger.rs`・`frame.rs`・`frame/wiring.rs`・`boot_config.rs`／`boot_resolve.rs`・`ghost_session.rs`・`menu/mod.rs`・`areka-ghost/src/runtime.rs`・kanade 5 ファイルに及ぶ。「起動時に最後のシェルを効かせる口」も同じファイルを通るので先行できない。共有 0 で先にできるのは present の片付け口（2〜3 タスク・単独 spec にはせず本仕様の先頭タスク）と 2 つ目のシェルの検体づくり（議題 ⑴ の答えが出てから）だけ。**要件（`/kiro-start`）は `ghost-shell-balloon-switch` の実装と並走して今すぐ書ける**。設計は `ghost-shell-balloon-switch` が main へ入ってから（`SwitchRequest`・汎用の通知の入口・`GhostSession` の置き場を見てから）。

**`ghost-restart-unit` で形が変わったところ**
1. 起動の結線は `&mut World` で呼べる（`emo2_boot/mod.rs` の `wire_emo2_boot(world, inputs: Emo2BootInputs, …)`）。ゴーストごとの値は `Emo2BootInputs`（`ghost_root`・`balloon_root`・`shiori`・`ticker`・`app_profile_dir`）に集まった。**シェル名の欄はここに足す**（`GhostBootOptions` には足さない＝`ghost-restart-unit` 要件 2.7 と両立）。組み立てるのは `ghost_session.rs` の `GhostBootInputs::production`＝本仕様が触るファイルに `ghost_session.rs`（454 行）を足す。
2. メニューの登記は「ゴーストを起こすたびにやり直す」契約になった（`menu/mod.rs` の `MenuRegistry` の doc）。シェル・バルーンの枠の登記はその契約に乗る。

**`pilot-balloon-asset-swap` の判定「直す」で決まったこと（09-24 節の「案 A／B は go 判定で決める」は消化済み＝案 B）**
3. present に「古い装着を消して、同じ呼び出しの中で登録し直す」口を足す。`areka-emo-present/src/presenter/hub.rs`（176 行）の `attach_target` は `targets` の表を置き換えるだけで World に触れない（`_world` 未使用）。片付ける口・登録を消す口は present に 0 件、`PresentCommand`（`command.rs` 231 行）は `ShowSurface`／`Hide`／`InvalidateCache` の 3 腕のまま。差し替えは配置が決まる前の段（`Update`・`run_attach_phase` の呼び手は `frame.rs` の `emo2_frame_system` の 1 か所）に置く。
4. 資産は往復のたびに作る（`EmoWorld` は複製できない）。今日の `assets.rs`（409 行）の `build_boot_assets` はシェルとバルーンを一度に作る 1 本なので、**片側だけ作る関数への分割が 1 タスク増える**。作る場所は差し替えの tick の外（設計で決める）。
5. **先進坑の学びのうち申し送りに無かった 6 点目**（`crates/pilot/examples/pilot-balloon-asset-swap/README.md` の「学び」）: 同じ id で `attach_target` を登録し直すと、**可視性の持ち主が既定の `CommandDriven` に、窓寸の要求（`applied`・`native_size`・`pending_resize`）が空に戻る**。本番のバルーン窓は `External`（`emo2_boot/frame/attach.rs` の装着）なので、差し替えた直後に `ShowSurface` が隠れているはずのバルーンを即座に見せる恐れがある。片付け口と再登録の口は、可視性の持ち主と窓寸の要求を引き継ぐ形にすること（引き継ぎを確かめる決定論テストを 1 本）。

**不変の前提（再確認）**: `read_last_shell` は無い・`LastUsed::record` は毎起動 `LastShell` を書く・`ConfigInputs` は 2 欄のみ／`resolve()` の本番呼び出しは 4 か所（シェル名が効くのは 3 か所）・`resolve.rs` 952 行／`list_shells` は `menu,hidden` を除外／`OnShellChang|OnBalloonChange` は `crates/` で 0 件／`SAMPLES` 7・`R_POST_and_KOMAINU.nar` のシェルは `master` のみ（シェルを 2 つ持つ検体は 7 体とも 0）／`shiori.toml` の `shellrootbutton.caption` の備考は今も引受先を `ghost-shell-balloon-switch` と書く（本仕様が直す）。

**数の更新**: `GhostBootOptions {` 28 か所・17 ファイル／`main.rs` 556／kanade `msg.rs` 852・`actor.rs` 541・`schedule/mod.rs` 758・`steady.rs` 935（不変）／`emo2_boot/mod.rs` 767・`frame.rs` 474・`frame/attach.rs` 441・`frame/wiring.rs` 324・`consumer_ledger.rs` 726／上限が近いテスト: `areka-ghost/src/runtime_tests.rs` 986・`emo2_boot/assets_tests.rs` 976・kanade `actor_tests.rs` 970・`schedule_tests.rs` 962＝足さない（兄弟の新ファイルへ）。

**接触ファイル**: 新規 `areka-parsers/src/package/resolve_shell.rs`（仮・`resolve_with_shell`）・`emo2_boot/switch_cue.rs`（`\![change,shell|balloon]` の受け口・**`ghost-shell-balloon-switch` の `change_cue.rs` と `ghost-change-name-resolution` のファイルには触らない**）・`emo2_boot/frame/switch.rs`（差し替えの相）／既存 `areka-emo-present/src/presenter/hub.rs`・`command.rs`・`areka-seriko/src/actor.rs`（`SerikoMsg`）・`areka-emo-text`（`TextMsg`）・`emo2_boot/{mod,assets,frame,consumer_ledger}.rs`・`frame/{attach,wiring}.rs`・`boot_resolve.rs`・`boot_config.rs`・`ghost_session.rs`・`areka-ghost/src/{runtime,catalog}.rs`・`menu/mod.rs`・`placement/source.rs`・kanade（`ghost-shell-balloon-switch` の汎用の通知の入口があれば `schedule/events.rs` の許可表だけ）・台帳 `shiori.toml`・`sakura-script.toml`＋生成物。

**タスク数**: **14〜17**（`ghost-shell-balloon-switch` が汎用の通知の入口を作る前提。無ければ 17〜20）。分割は不要（分割候補はどれも `ghost-shell-balloon-switch` の後で直列）。

**正典の逐語**: ukadoc `\![change,shell,…]` は「シェル名を lastinstalled にすると最後にインストールした**ゴースト**に切り替え」と書く（原文ママ・`\![change,balloon,…]` は「バルーン」）。「最後に入れたシェル」と読むか、ゴーストの切替（`ghost-shell-balloon-switch`・`ghost-change-name-resolution` の経路）へ回すかは 1 行の裁定が要る（議題 ⑷）。`OnShellChanging` の Ref は Ref0＝切り替わるシェル名・Ref1（SSP）＝現在のシェル名・Ref2（SSP）＝切り替わるシェルのパス（09-24 までの本文に無かった）。`OnShellChanged` Ref0＝現在のシェル名・Ref1（SSP）＝現在のゴースト名・Ref2（SSP）＝パス／`OnBalloonChange` Ref0＝名・Ref1＝パス。

**議題の整理**: 09-24 節の ⑴〜⑶ はそのまま。⑷ のうち「バルーンブレークで切替を中止するか」は **`ghost-shell-balloon-switch` の議題 ⑴ と同じ答えにする**（別に問わない）。残るのは `random`／`lastinstalled` を In に入れるかと、上の `lastinstalled` の読み方。

## 2026-09-24 棚卸⑯の再測定（main `0b01f654`）

**先進坑は別 spec に切り出した＝`pilot-balloon-asset-swap`。** `two-tunnel.md` の規約（go 判定を本坑の前提依存とし、go 前の本坑着手は規律違反・1 仕様＝1 フォルダ・`pilot-` 接頭辞）により、同じ spec の先頭タスクにはできない。触るのは `crates/pilot/examples/pilot-balloon-asset-swap/` と `crates/pilot/Cargo.toml`（dev-dependencies に areka-emo-present・-compose・-atlas・-text・areka-parsers・wintf を足す＝pilot 側が依存するだけで葉ノードの隔離は崩れない）だけ＝**`shiori-fault-notice`・`ghost-restart-unit`・`ghost-shell-balloon-switch` と共有 0 で並走できる**。本仕様は roadmap に `_Depends(confirmed): pilot-balloon-asset-swap` を持つ。roadmap A3 行の旧記 `crates/pilot/examples/areka-P0-shell-balloon-switch/` は命名規約違反だったので直した。

**崩れた／変わった前提**

1. **起動時に最後のシェルを効かせる口は今も無く、しかも起動のたびに記憶が上書きされる。** `LastUsed::record`（`crates/areka/src/boot_resolve.rs`・`main.rs` の `on_boot_ok` が呼ぶ）は起動に成功するたびに `mount().shell.dir` の末尾を `LastShell` へ書く。`read_last_shell` は無い。**切替だけ実装すると次の起動で既定シェルが書き戻され、切り替えた記憶が消える**＝起動時の適用は省けない必須作業。開ける場所は ⓐ `boot_resolve.rs` に `read_last_balloon` を写した `read_last_shell(ghost_dir)`（数行）→ ⓑ `boot_config::resolve_boot_from` で `shell/<名>/` の実在を確かめる（無ければ `warn!` して既定へ）→ ⓒ `ConfigInputs` に `shell` の欄 → ⓓ `wire_emo2_boot` の引数と `ghost_boot_options` へ流す。
2. **`resolve()` の本番の呼び出しは今も 4 か所だが、シェル名が効くのは 3 か所**（`boot_with_kanade_stop`・`build_boot_assets`・`load_descript_source`）。`load_restored_state`（`placement/persist.rs`）は `model.shiori.dir` しか使わない。引数は `(ghost_root, default_encoding)` のまま＝`baseware-root-layout` はシェル名を足していない。**`resolve.rs` は 952 行**で関数を 1 つ足すと 1,000 行を超える→ `resolve_with_shell(root, enc, Option<&str>)` を**隣の新ファイル**に作り `resolve` はそれに `None` で委ねる（テストの呼び出し約 30 本を直さずに済む）。
3. **`areka-ghost` へ渡すのは引数が安い。** `GhostBootOptions {` は 27 か所・16 ファイル（不変）。`boot_with_kanade_stop` の引数にすれば呼び出しは 3 か所。
4. **`catalog::list_shells` は `menu,hidden` を除外する**（`crates/areka-ghost/src/catalog.rs`）。正典では hidden も名指しなら切り替えられるので、名指しの検証と起動時の復元には `shell/<名>/` の実在を別に確かめる。
5. **正典の見落とし 2 件**（ukadoc `\![change,shell,…]`／`\![change,balloon,…]`）: シェル名・バルーン名として `random`／`lastinstalled` も受ける／`raise-event` のときはバルーンブレークで切替を中止できる。どちらも In に無い（議題 4）。
6. 差し替えの語彙 0 件は不変（較正の `pub enum` 4 種は 4 件。seriko の場所は `crates/areka-seriko`・`SerikoMsg` は `src/actor.rs`＝brief の `areka-emo-seriko` は誤記）。現状: `SerikoMsg` Cue／Tick／Close・`TextMsg` Cue／Close・`PresentCommand` ShowSurface／Hide／InvalidateCache・`DispatcherMsg` 7 腕。**`EmoPresenter::attach_target`（`areka-emo-present/src/presenter/hub.rs`）は同じ id を再登録すると表示コンテキストごと置き換える**＝差し替えの最小の部品になり得る（先進坑が確かめる対象）。
7. **`OnShellChang|OnBalloonChange` は crates/ 全体で 0 件**（ukadoc-survey も含む。`OnGhostChang` は `diff_tests.rs` で 7 件＝検索は効いている）。
8. **検体**: `R_POST_and_KOMAINU.nar` のシェルは `master` 1 つ（43 ファイル）。`SAMPLES` は 7 件（2026-09-24 に `claudia` が入った）・`lib_tests.rs` に `SAMPLES.len(), 7` が 2 か所とコメント「7 つ」3 か所・`vendors/sample_ghost/README.md` の表にファイル数と大きさ。**実機で応答を観測できる検体がある**: R_POST の `dic02_Event.txt` は `OnShellChanging`／`OnShellChanged` に、emo2 の `update.pasta` は `OnBalloonChange` に、konnoyayame は 3 つすべてに応答する。
9. メニュー: `Frame::Shell`／`Frame::Balloon` と文言（`shellrootbutton.caption`／`balloonrootbutton.caption`・`captions.rs`）は在り、登記は `wire_menu_with` の Readme・Close の 2 つだけ。`menu::register` の外からの呼び手は 0。
10. kanade 5 ファイル: `msg.rs` 771／`actor.rs` 507／`schedule/mod.rs` 751／`schedule/events.rs` 431／`schedule/steady.rs` **935**。`emo2_boot/` の関わるファイル: `assets.rs` 409・`frame/attach.rs` 441・`frame.rs` 459・`consumer_ledger.rs` 726・`placement/spawn.rs` 765・`source.rs` 287・`persist.rs` 512。`wire_emo2_boot` は今も一発の構築（本番の呼び出しは `main.rs` の 1 か所）。
11. 陳腐化 1 件（本仕様が直す）: `doc/ukadoc-coverage/ledger/shiori.toml` の `shellrootbutton.caption` の備考が引受先を `areka-P0-ghost-shell-balloon-switch` と書いている＝分割後は本仕様の担当。

**タスク数**: `ghost-shell-balloon-switch`（と `ghost-restart-unit`）が汎用の通知の入口と `SwitchRequest` を用意してから着手すれば **13〜16 本**（brief の 12〜15 とほぼ同じ）。待たずに着手すると通知の入口の自作＋kanade 3 ファイルで +3〜4、`random`／`lastinstalled` で +1〜2＝17〜20 で M を超える。**分割は不要**（「シェル名の運搬と起動時の適用」4〜5 本は独立しているが `boot_config.rs`／`main.rs` を `ghost-shell-balloon-switch` と共有するので直列のまま）。

**要件段階の議題（Fable）**: ⑴ 2 つ目のシェルの検体をどう用意するか——`R_POST_and_KOMAINU.nar` を畳み直す（README の表と `lib_tests` の数が変わる・改変してよいかのライセンス確認が要る）か、テストのときに展開先で `shell/master` を複製する（`.nar` は変わらないが実機の往復は手作業）か。⑵ `\![change,shell,名]` の「名」をフォルダ名と descript の `name` のどちらで引くか（記憶の鍵はフォルダ名）。⑶ 会話の途中で切り替えたとき、表示中のバルーンの文字と残りの台本を引き継ぐか消すか（「1 フレームも崩さない」保証の形がこれで決まる）。⑷ `random`／`lastinstalled`（シェルとバルーン）と `raise-event` 時のバルーンブレークによる中止を In に入れるか。案 A／B の選択は**先進坑 `pilot-balloon-asset-swap` の go 判定で決める**（要件の前）。

## 2026-09-24 先進坑 `pilot-balloon-asset-swap` の判定「直す」と申し送り

**開発者判定＝直す**（`.kiro/specs/completed/pilot-balloon-asset-swap/`・一次記録は `crates/pilot/examples/pilot-balloon-asset-swap/README.md`）。本仕様の `_Depends(confirmed): pilot-balloon-asset-swap` はこれで満たされた。**案 B（present・seriko・text に「資産を差し替えろ」の語を足す）で進める。**

1. **同じ id の `attach_target` 再登録だけ（基準の版）は使えない。** 古い装着の子（`emo-surface`・`emo-text-layer-slot`）が World に残り、9 走行とも古い絵と新しい絵が重なったまま揃わなかった（混在が 1 観測あたり 104〜269 枚）。
2. **古い装着を消してから同じ呼び出しの中で再登録・表示・窓寸合わせ（本命の版・`Update` の段）なら、反映待ち以外の崩れは 9 走行とも 0。** 反映待ち（当たり判定と窓寸が絵より画面更新 1 回ぶん先に新しくなる 1 枚）は静かな走行で 0〜1 枚＝本番の `[ID]` の切り替えと同じ（床 1）。先進坑は古い子を名前で探して外から despawn した——**本仕様で present に「古い装着を片付ける正規の口」を足す**（先進坑の要件 5.7・5.8）。presenter には登録（`TargetId`）を消す口も無いので同じ範囲に入れる。
3. **差し替えは配置が決まる前（`Update`）に置く。** 画面への反映の後（`FrameFinalize`）で消すと、古い visual は即座に外れ新しい visual は次の tick に作られるので、絵が空のフレームが 1〜2 枚出た（隠すだけの版は当たり判定が空＝クリックが素通りするフレームが 1〜2 枚）。
4. **`EmoWorld` は `Clone` でない**（`attach_target` は消費する）。往復のたびに資産が要るなら、どこで作り直すか（`build_balloon_target` の復号を差し替えの tick に入れない）を設計で決める。
5. **未観測**: 画面の拡大率が 1 でない場合（開発機の既定は 200%）。先進坑は 100% でしか測っていない。wintf の兄弟の重なり順が描画と当たり判定で逆（登記だけ）は、2 の形なら兄弟が 2 組にならないので表に出ない。

## Problem

**誰の何が困っているか**: 別のシェルを持つゴーストの利用者と、バルーンを入れ替えたい利用者。α の利用者の一周は「右クリックメニューでゴースト／シェル／バルーンを替える」を含む。

`\![change,shell,名]` と `\![change,balloon,名]` は解析は通るが消費者が居らず、**黙って何も起きない**。`OnShellChanging`／`OnShellChanged`／`OnBalloonChange` は製品コードに 0 件（`git grep -l "OnShellChang\|OnBalloonChange" -- 'crates/*.rs'` は `crates/ukadoc-survey/src/diff_tests.rs` の 1 ファイルだけ）。

## Current State——親 brief の前提が 2 つ崩れている

親 brief は「寿命を先に分離しておけば、バルーンとシェルの切替は資産を作り直すだけの仕事になる」「バルーンが最軽量」と書いた。**2026-09-20 の実測はどちらも支持しない。**

1. **走っているゴーストの中で、絵と文字の出し先を差し替える語彙が 1 つも無い。**
   - 出し先（sink）は起動時に `GhostBootOptions.sinks` として**値で渡され**、配る役（dispatcher）に固定される（`crates/areka-ghost/src/runtime.rs` の sink 配線）。
   - `DispatcherMsg` に差し替えの語は無い（`crates/areka-ghost/src/dispatcher.rs` の `DispatcherMsg` の定義）。
   - seriko・present・text・dispatcher の 4 か所を `reload|ReplaceTarget|Rebuild|swap_sink|replace_sinks` で引くと **0 件**（同じ 4 か所は `pub enum SerikoMsg|PresentCommand|TextMsg|DispatcherMsg` で 4 件当たる＝検索は効いている）。
   - 組み立ての本体 `wire_emo2_boot`（`crates/areka/src/emo2_boot/mod.rs`）は**一発の構築**である。シェルとバルーンの資産を束ねて作り、seriko を起こし、`boot_with_kanade_stop` を呼び、`add_systems` も 1 度きり。
   - つまり SHIORI を生かしたままシェルやバルーンだけを替えるには、**各アクターへの読み直しの語彙か、出し先の付け替えが新たに要る**。これが本仕様の主題である。
2. **シェル切替は「マウントの片側を差し替える」では済まない。** シェルを決める `resolve()`（`crates/areka-parsers/src/package/resolve.rs`）は `seriko.defaultsurfacedirectoryname`、無ければ `master` しか見ず、**4 か所から独立に呼ばれている**: `crates/areka-ghost/src/runtime.rs`・`crates/areka/src/emo2_boot/assets.rs`・`crates/areka/src/placement/source.rs`・`crates/areka/src/placement/persist.rs`。選んだシェル名を 4 か所すべてへ運ぶ必要がある。`GhostBootOptions` の構造体リテラルは 27 か所・16 ファイルに在る（`git grep -c "GhostBootOptions {"`）ので、欄を足す形は波及が大きい。

## Desired Outcome

完了時に次が真になっている（語彙と Ref の正本は親 brief）。

1. `\![change,shell,名]` で同じゴーストの別シェルへ替わる。SHIORI は降ろさない。`--option=raise-event` のときだけ `OnShellChanging`、替わったあと `OnShellChanged`。
2. `\![change,balloon,名]` でバルーンが替わり `OnBalloonChange` が届く。
3. 右クリックメニューの「シェル」「バルーン」の枠に、列挙された候補が並び、選ぶと同じ経路で替わる。
4. 選択は `baseware-root-layout` の「最後に使った」鍵へ書かれ、**次回起動でシェルも復元される**（起動時にシェル名を効かせる口は本仕様が作る——`baseware-root-layout` は鍵の定義と書き込みまで）。
5. 該当が無ければ `warn!` を出して無視する。切替の途中で失敗したら、`error!` を出して**元のシェル／バルーンのまま表示が続く**（ログ無し失敗経路の禁止）。

## Approach（要件で選ぶ・いまは方向だけ）

| 案 | 中身 | 見立て |
|---|---|---|
| A. 作り直しの一般形に乗る | ゴースト切替（親 spec）が作る「降ろして起こし直す」仕組みを、SHIORI だけ残して回す | 親 spec が先に着地していれば、新しい語彙が最少。代わりに SHIORI アクターを跨いで残す寿命の扱いが要る |
| B. 各アクターに読み直しの語を足す | seriko・present・text に「資産を差し替えろ」の語を 1 つずつ足す | 影響が局所だが、4 アクター分の語彙と、差し替えの途中の 1 フレームに古い絵と新しい当たり判定が混ざらない保証が要る（記憶 no-frame-delay-fixes-change-the-state-shape） |

**どちらが安いかはコードを書いてみないと分からない類の問い**なので、要件の前に**先進坑（`crates/pilot/examples/`）を 1 本掘る**ことを推す（`.kiro/steering/two-tunnel.md`）。掘る対象は「SHIORI を生かしたまま present のバルーン資産だけを差し替えて、1 フレームも崩れずに表示が続くか」の 1 点。

## Scope

- **In**: `\![change,shell|balloon,名(,--option=raise-event)]` の消費者／`OnShellChanging`／`OnShellChanged`／`OnBalloonChange` の送出と Ref／シェル名を `resolve()` の 4 呼び出し点へ運ぶ口／起動時の「最後のシェル」の適用／メニューの「シェル」「バルーン」枠への登記／2 つ目のシェルを持つ検体（`R_POST_and_KOMAINU` の `shell/master` を写して名前を変えたもの）。
- **Out**: ゴースト切替と切替要求の型 `SwitchRequest`・名前解決（親 spec）／寿命の分離（`areka-P0-app-lifetime-separation`）／`\![reload,shell|balloon]`／`\![bind,…]` 着せ替え／拡大率（`\![set,scaling,…]`）／`currentghost.shelllist.*`・`balloonlist.*` のプロパティ。

## Boundary Candidates

- 資産の差し替え（emo 側: `emo2_boot/assets.rs`・`frame/attach.rs`・`placement/spawn.rs`・各アクターの語彙）
- シェル名の運搬（`resolve.rs` と 4 呼び出し点）
- 切替の通知（kanade の 5 ファイル: `msg.rs`・`actor.rs`・`schedule/{mod,events,steady}.rs`）

## Out of Boundary

- 列挙と記憶の実体（`baseware-root-layout`）
- 切替要求の入口の型と、台本とメニューが同じ 1 本を通る保証（親 spec が作り、本仕様は種別を 2 つ足すだけ）

## Upstream / Downstream

- **Upstream**: `areka-P0-baseware-root-layout`（`list_shells`・`last.shell`・`last.balloon`）／`areka-P0-ghost-shell-balloon-switch`（`SwitchRequest`・切替の通知の入口・作り直しの一般形）／`areka-P0-default-balloon-nar-fold`（バルーン 2 つ目の検体は既定バルーンで足りる）。
- **Downstream**: `areka-P0-ghost-install`（シェル・バルーンの `.nar` を入れた直後の切替）・`areka-P0-network-update`・`areka-P0-alpha-release-signoff`・α 後の `areka-P0-property-catalog-lists`。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: 親 spec・`ghost-install`・`network-update` と、`crates/areka/src/emo2_boot/consumer_ledger.rs`・`emo2_boot/mod.rs`・kanade の 5 ファイル・網羅台帳（`doc/ukadoc-coverage/ledger/*.toml` と生成物）を**全員が触る**＝必ず直列（親 → 本仕様 → `ghost-install` → `network-update`）。`crates/areka-kanade/src/schedule/steady.rs` は 935 行で上限が近い。`crates/sample-ghost-kit/src/{lib.rs,lib_tests.rs}` は検体を足すなら直書きの数の数え直しが要る。

## Constraints

- 規模 **M**（タスク 12〜15 本）。**要件と設計は Fable**（主題が「今は存在しない仕組みの設計」で、答えがコードからは出ない）。
- 差し替えの途中の表示が 1 フレームも崩れないこと（古い絵と新しい当たり判定の混在・空の窓の点滅）。1 フレーム遅らせて辻褄を合わせる解は取らない。
- 決定論テスト網羅は必達。資産は偽のシェル 2 つ・偽のバルーン 2 つで往復し、イベントの Ref を突き合わせる。実機は `R_POST_and_KOMAINU` の 2 シェル往復と、既定バルーン ⇄ `emo2` 同梱バルーンの往復を 1 周。
- 1 ファイル 1,000 行。

**`ghost-shell-balloon-switch` からの申し送り（2026-09-27 完了時）**: ⑴ kanade の汎用の通知の入口 `KanadeMsg::RaiseEvent` の許可表は、起動・終了のイベントと共用の `ALLOWED_EVENT_IDS` をそのまま使う。入口専用の表に分けるかは、入口を最初に使う spec で決める。⑵ 終了や起こし直しの経路を足すときは、印を消す判定 `crates/areka/src/main.rs` の `session_mark_verdict` を必ず通す（`ExitOrigin` を足すと網羅の match がコンパイルで止める）。正本は `doc/COMPAT_ARCHITECTURE.md` §8 と完了 spec の design「Boundary Commitments」

## 2026-09-27 棚卸⑱の再測定（main 5a232d2f）

> `ghost-shell-balloon-switch`（PR#192）・`pilot-dropfiles-on-wuc-window`（PR#191）・`alpha-package`（PR#190）の着地後。上の節の file:line は再測定前の値なので、以下を正とする。

**判定: `ghost-change-name-resolution` と完全並走できる（触るソースの重なり 0）——ただし下の「重ねないための 3 つの約束」を守るとき。** 設計（`/kiro-design`）はいま始められる。

### `ghost-shell-balloon-switch` が実際に作ったもの（本仕様の前提の引き直し）

1. **切替要求の型は `SwitchRequest`（`crates/areka/src/emo2_boot/ghost_switch.rs` の `pub(crate) struct SwitchRequest { ghost: GhostSpec, raise_event: bool, origin: ChangeOrigin }`）で、ゴースト専用**。`GhostSpec` は `Name(String)`／`Folder(String)` の 2 腕。唯一の入口は同ファイルの `request_ghost_switch(world, req) -> SwitchVerdict`。上の「Out of Boundary」の「本仕様は種別を 2 つ足すだけ」は成り立たない——`SwitchRequest` にシェル・バルーンの種別を足す形は取れない（型も入口も kanade への `ChangeGhost` と結びついている）。**本仕様はシェル・バルーン用の要求の型と入口を自前で持つ**（`switch_cue.rs`＝受け口・`frame/switch.rs`＝差し替えの相。`ghost_switch.rs` には触らない）。
2. **台本の受け口 `ChangeCueSink`（`emo2_boot/change_cue.rs`）は `("change","ghost")` の 1 組だけを受理し、`(change,shell)`・`(change,balloon)` は「担当外」として `debug!` で読み飛ばす。** 消費者台帳（`emo2_boot/consumer_ledger.rs`）も `("change", Some("ghost"))` だけを登記し、同ファイルのテスト `canonical_registers_only_change_ghost_for_the_change_sink` が「`(change,shell)`・`(change,balloon)` は登記されていない」を固定している。本仕様は `("change", Some("shell"))`・`("change", Some("balloon"))` の 2 行を足し、**このテストを書き換える**（削除でなく「3 組が登記され、裸の `change` は無い」へ）。
3. **汎用の通知の入口は在る**: `KanadeMsg::RaiseEvent { id: String, references: Vec<String>, method: ShioriMethod }`（`crates/areka-kanade/src/msg.rs`）→ `Input::RaiseEvent` → `schedule/change.rs` の `on_raise_event` → `events::allowed_static`（`ALLOWED_EVENT_IDS` との逐語照合）→ `events::raise`。**許可表は起動・終了と共用の `ALLOWED_EVENT_IDS`（`schedule/events.rs`）で、`OnShellChanging`／`OnShellChanged`／`OnBalloonChange` は載っていない**＝本仕様が 3 行足す。kanade で触るのは `events.rs` の表だけ（`msg.rs`・`actor.rs`・`schedule/mod.rs`・`steady.rs` は 0）。入口の性質で設計に効く 3 点: ⓐ 定常（`Phase::Steady`。再生中の `Steady{talk: Some}` も含む。起動系列は `boot.rs` の `OnBoot` の応答で `Steady{talk}` へ入るので、`OnBoot` の台本の中の `\![change,shell]` は受理される）以外では `warn!` で捨てて積まない＝起動系列の途中に届いた通知は失われる。ⓑ 再生中に GET の応答（台本）が返ると `events::value_replaces_active_talk` により**今のトークを置き換える**（`OnShellChanged` の台詞が会話に割り込む形になる＝議題 ⑶ と同じ問い）。ⓒ 入口は「送る」だけで、台詞の終わりを呼び手に知らせない。**`--option=raise-event` の「`OnShellChanging` の台詞のあとで替える・ダブルクリックで中止」は kanade に相が無いので UI 側で組む**＝`Emo2Wiring.lifecycle_rx`（`frame/wiring.rs`）へ届く `TalkLifecycleSignal`（`DisplayEndAt`／`UserBreak`）を差し替えの相が読んで待つ形（設計で決める・+1〜2 タスク）。kanade に切替の相を足す形（`schedule/change.rs` 同型）は kanade 3 ファイル＋`steady.rs` に触るので取らない。
4. **`GhostSession` の置き場は `crates/areka/src/ghost_session.rs`（638 行・棚卸⑰の 454 から増えた）の `GhostSlot(Option<GhostSession>)`（NonSend）**。起動入力は `GhostBootInputs::production(cfg: &ConfigInputs, helper_exe, kanade_stop, boot_origin)` が `Emo2BootInputs { ghost_root, balloon_root, shiori, ticker, app_profile_dir, boot_origin }`（`emo2_boot/mod.rs`・805 行）を組む。**シェル名の欄は `Emo2BootInputs` に足し、`production` の中で `read_last_shell(&cfg.ghost_root)` を読んで詰める**（棚卸⑰ 項目 1 のとおり）。棚卸⑯ 項目 1 の「ⓒ `ConfigInputs` に `shell` の欄」は**取り下げ**（下の約束 1）。`resolve()` を呼ぶのは `areka_ghost::boot_with_origin`（`crates/areka-ghost/src/runtime.rs`・`boot_with_kanade_stop` はその薄い皮）なので、シェル名の引数は **`boot_with_origin` に足す**（本番の呼び手は `emo2_boot/mod.rs` の結線ありの腕と `boot_with_kanade_stop` の 2 か所）。`boot_with_origin` は `mount.shell.dir` の末尾を `KanadeConfig.shell_folder` へ詰める（`OnGhostChanged` の Ref7・`OnBoot` の Ref0）ので、シェルを替えて起こし直せば Ref は自動で追随する。
5. **今のシェルの置き場は新しく作らない**: `GhostSession::runtime()` → `mount().shell.dir` で読める（`ghost_switch.rs` の `record_steady_memory` がそうしている）。`CurrentGhost`（`boot_config.rs`）に `shell` の欄は足さない（約束 1）。バルーン切替後の `ctx.current.balloon = …` は欄への代入なので許される。
6. **メニューの枠の登記の仕方**: `menu/ghost_frame.rs`（73 行）が手本。`pub(crate) fn register(world)` が `menu::register(world, Frame::Ghost, Rc::new(ghost_frame_item))` を呼び、供給関数はメニューを出すたび `BootContext` の根で目録を読み、子を `ItemBody::Action` で並べる。呼び手は `ghost_session.rs` の `boot_wired` で `menu::wire_menu` の直後（ゴーストを起こすたびに登記をやり直す契約）。本仕様は `menu/shell_frame.rs`・`menu/balloon_frame.rs` を足し、`menu/mod.rs` に `pub(crate) mod` 2 行、`boot_wired` に `register` の呼び出し 2 行。`Frame::Shell`／`Frame::Balloon` は `Frame::ORDER` に在り、文言は `captions.rs` に在る（不変）。
7. **`session_mark_verdict`（`crates/areka/src/main.rs`・きれいな終わりの判定・`ExitOrigin` の網羅の match）**: 本仕様は終了の出所を足さない（差し替えの失敗は元のシェル／バルーンのまま続ける）ので触らない。**印を別の場所で消さない**（§8 (j) の末尾の決まり）。
8. **記憶の書き込み**: `crate::record_last_used(publisher, shell_dir, ghost, balloon)`（`main.rs`）が `LastShell`（常に）・`LastBalloon` を書く既存の 1 か所。切替後はこれを呼ぶだけでよい（実行系が動いている間は UI から App／Ghost スコープへ直接書かず、`runtime.sylphya_publisher()` を通す決まり＝`boot_resolve.rs` の「起動中の印」節のコメント）。`read_last_shell` は今も無い（`boot_resolve.rs` 471 行に `read_last_ghost`・`read_last_balloon`・`read_session_mark` の 3 読み手）。
9. **present は不変**: `areka-emo-present/src/presenter/hub.rs`（176 行）の `attach_target` は `targets` の表を置き換えるだけ（`_world` 未使用・可視性の持ち主は既定 `CommandDriven` で登録）、片付ける口・登録を消す口は 0、`PresentCommand` は 3 腕。バルーンの `External` は `emo2_boot/frame/attach.rs` の装着が `set_visibility_ownership` で明示する。棚卸⑰ 項目 3〜5 はそのまま。差し替えの相は `frame.rs`（509 行）の `emo2_frame_system` の `run_attach_phase` の前に置く。
10. **不変の確認**: `catalog::list_shells` は `menu,hidden` を除外（`crates/areka-ghost/src/catalog.rs` 324 行）／`OnShellChang|OnBalloonChange` は製品コードで 0（`schedule/raise_event_tests.rs` が `raise` に直接 `"OnShellChanged"` を渡す 1 件はテストで、許可表を通らない）／`resolve()` の本番呼び出しは 4 か所（`assets.rs`・`placement/persist.rs`・`placement/source.rs`・`areka-ghost/runtime.rs`。`config.rs`・`shiori_wiring.rs` の 2 件は fixture の組み立て）／`GhostBootOptions {` は 29 か所・18 ファイル（欄は足さない）／`SAMPLES` 7・2 シェルの検体 0。
11. **台帳の陳腐化（本仕様が直す）**: `shiori.toml` の `shellrootbutton.caption` の備考「引受先: areka-P0-ghost-shell-balloon-switch」と `balloonrootbutton.caption` の「引受先: areka-P0-baseware-root-layout」は**どちらも本仕様**。`sakura-script.toml` の `\![change,shell,…]`・`\![change,balloon,…]`、`shiori.toml` の `OnShellChanging`・`OnShellChanged`・`OnBalloonChange` は `absent`・owner 空のまま。
12. **数**: `emo2_boot/mod.rs` 805・`frame.rs` 509・`consumer_ledger.rs` 756・`ghost_session.rs` 638・`main.rs` 873・`boot_resolve.rs` 471・`boot_config.rs` 357／kanade `msg.rs` 880・`actor.rs` 589・`schedule/mod.rs` 830・`events.rs` 535・`steady.rs` 935（触らない）／上限が近いテスト: `runtime_tests.rs` 986・`spine_conformance_lap_tests.rs` 990・`balloon_visibility_tests.rs` 982・`assets_tests.rs` 976・`actor_tests.rs` 970・`schedule_tests.rs` 962（足さない・兄弟の新ファイルへ）。

### 触るソースファイル（本仕様・今の実物）

新規: `crates/areka-parsers/src/package/resolve_shell.rs`（＋`package/mod.rs` の mod 1 行）・`crates/areka/src/emo2_boot/switch_cue.rs`・`emo2_boot/frame/switch.rs`・`crates/areka/src/menu/shell_frame.rs`・`menu/balloon_frame.rs`・各兄弟テスト。
既存: `crates/areka-emo-present/src/presenter/hub.rs`・`command.rs`・（必要なら `presenter/visibility.rs`）／`crates/areka-seriko/src/actor.rs`（`SerikoMsg`）・`crates/areka-emo-text`（`TextMsg`）／`crates/areka/src/emo2_boot/{mod,assets,frame,consumer_ledger}.rs`・`frame/{attach,wiring}.rs`／`crates/areka/src/{boot_resolve,ghost_session,main}.rs`（`main.rs` は `record_last_used` の呼び足しがあれば）／`crates/areka/src/menu/mod.rs`／`crates/areka-ghost/src/{runtime,catalog}.rs`／`crates/areka/src/placement/{source,persist}.rs`／`crates/areka-kanade/src/schedule/events.rs`（`ALLOWED_EVENT_IDS` の 3 行）／検体 `crates/sample-ghost-kit/src/{lib,lib_tests}.rs`・`vendors/sample_ghost/README.md`（議題 ⑴ の答えしだい）。
別枠（重なりに数えない）: `doc/ukadoc-coverage/ledger/{sakura-script,shiori}.toml`・生成物 `report/*.md`・`doc/COMPAT_ARCHITECTURE.md` §8 末尾・`.kiro/steering/roadmap.md`。`THIRD-PARTY-NOTICES.md` は依存を足さないので 0。

### `ghost-change-name-resolution` と重ねないための 3 つの約束

`ghost-change-name-resolution` が触るのは `crates/areka/src/emo2_boot/ghost_switch.rs`（と `ghost_switch_tests.rs`・その兄弟の新テスト）・`crates/areka-parsers/src/sakura/decode.rs`（と `decode_tests.rs`）だけ（必要なら `crates/areka-sakura/src/compile.rs`）。本仕様はこれらに触らない。加えて:
1. **`ConfigInputs`・`CurrentGhost`・`GhostDecision`・`BalloonDecision` に欄を足さない。** `ghost_switch.rs` の `boot_into` が `ConfigInputs { … }`・`CurrentGhost { … }`・`GhostDecision { … }` を構造体リテラルで組んでいる（本番のリテラルは `ConfigInputs` 2・`CurrentGhost` 2・`GhostDecision` 5 か所）。欄を足すとコンパイルが `ghost_switch.rs` の書き換えを要求し、重なりが生まれる。シェル名は `Emo2BootInputs`（`mod.rs`）へ、今のシェルは `mount().shell.dir` から。
2. **`SwitchRequest`／`GhostSpec`／`request_ghost_switch` の形を変えない・呼ばない。** シェル・バルーンの `lastinstalled` を「ゴーストの切替へ回す」と裁定した場合（議題 ⑷）も、呼ぶのは `request_ghost_switch` を**呼ぶだけ**（形は変えない）。
3. **`crates/areka-ghost/src/lib.rs` と `crates/areka-parsers/src/sakura/` に触らない。**（名前解決が純関数を areka-ghost の新ファイルに置く場合は `lib.rs` に `pub mod` 1 行が入る。）

### 想定タスク数と切る場所

**15〜18**（内訳: present の片付け口と再登録の引き継ぎ 2〜3・資産の片側だけ作る分割 1・受け口 `switch_cue.rs`＋台帳 2 行＋テスト書き換え 1・差し替えの相（シェル）1〜2・（バルーン）1・シェル名の運搬（`resolve_shell.rs`・`boot_with_origin` の引数・`assets`／`source`／`persist`）2・`read_last_shell`＋`Emo2BootInputs.shell`＋`production` 1・通知 3 種と `ALLOWED_EVENT_IDS`＋`OnShellChanging` の台詞待ちと中止 2・メニュー 2 枠 1〜2・切替後の記憶 1・検体 1・台帳と §8 1・決定論の統合テストと実機 1〜2）。20 に触れたら切る順: ① シェル・バルーンの `random`／`lastinstalled`（議題 ⑷）を Out へ（S の追い spec・+1〜2 ぶん）② メニューの「バルーン」枠と「シェル」枠を 1 タスクに畳む。「起動時に最後のシェルを効かせる口」を先に切り出す案は `mod.rs`・`ghost_session.rs` を共有するので直列にしかならず、本数の節約にならない（採らない）。

### 要件定義で Fable が要るか

**要る。** 議題 ⑵（名をフォルダ名と descript の `name` のどちらで引くか）は**取り下げ**——`ghost_switch.rs` の `resolve_switch_target` が「`name` → フォルダ名の順・大文字小文字は区別」と決めており、シェル・バルーンも同じ規則に乗せる（勝者が明白）。残る議題は 3 つ: ⑴ 2 つ目のシェルの検体（`R_POST_and_KOMAINU.nar` を畳み直す＝改変の可否をライセンスで確かめる／テスト時に `shell/master` を複製）、⑶ 会話の途中で切り替えたとき表示中の文字と残りの台本を引き継ぐか消すか（上の 3-ⓑ のとおり `OnShellChanged` の台詞が今のトークを置き換える入口の性質と噛み合わせて 1 つの答えにする）、⑷ `random`／`lastinstalled` を In に入れるかと、正典が「シェル名を lastinstalled にすると最後にインストールした**ゴースト**に切り替え」と書く読み方（ukadoc `\![change,shell,…]` 原文ママ・`\![change,balloon,…]` は「バルーン」）。ダブルクリックでの中止は `ghost-shell-balloon-switch` の裁定 4（中止して元へ戻る）に揃える＝問わない。

### `ghost-shell-balloon-switch` からの申し送り（完了 spec の design「Revalidation Triggers」・`signoff.md`・上の末尾の 2 点に加えて）

- `KanadeNotice`（`Steady`／`ChangeCancelled`／`Stopped`）の変種と `KanadeStopped` の欄の形を、`frame.rs` の `run_ghost_quit_phase` が前提にしている。本仕様が受け口 `KanadeNoticeRx` を読むなら形を変えない。
- `RaiseEvent` の応答は再生中の台詞を置き換える（`value_replaces_active_talk`＝`OnSecondChange` 以外はすべて置き換え）。`OnShellChanged`／`OnBalloonChange` を再生中に送ると今の台詞が消える。
- `GhostSlot`／`BootContext`（`root`・`app_profile_dir`・`argv_session`・`current`）は読むだけ。`BootContext.current.balloon` の書き換えは本仕様の持ち場（doc コメントに明記済み）。
- `close_windows_for_restart`（`app_exit.rs`）は窓の一式ごとに 1 度だけ立つ資源（`ChainFinalized` ほか 5 つ）を外す。**本仕様はキャラ窓を作り直さない前提**（シェルの差し替えは窓を残して資産だけ替える）。窓を作り直す設計に倒れるなら、この経路の前提（初期配置の確定が走り直す）を再検証する。
- `signoff.md` 気付き 8: `GhostSession::shutdown` の join の間は UI の窓へ届く同期の送信が止まりうる（未確認）。本仕様は降ろさないので直接は関係しないが、差し替えの相で UI スレッドを長く塞ぐ処理（資産の復号）は tick の外へ出す（棚卸⑰ 項目 4 と同じ結論）。

## 2026-09-29 着手順の入れ替え（開発者指示）

- **本仕様は `network-update` の後（B8）になった。** Adjacent 節の「親 → 本仕様 → `ghost-install` → `network-update`」の直列は、向きが `ghost-install` → `file-drop` → `network-update` → **本仕様** に変わる（共有 5 本の直列はそのまま）。
- 着手時には、`network-update` が足した許可表の件数・メニューの枠（`Frame::Update` の登記）・`consumer_ledger.rs` の行を読み直してから載せる。
- `areka-emo-text` の `sink.rs`（`TextMsg`）を触るのは本仕様だけ。前倒しの `balloon-color-emoji` が先に着地していれば、書記素クラスタ単位になった `areka-emo-text` の上に載せる。
