# Brief: areka-P0-shell-balloon-switch

> 2026-09-20 `/kiro-discovery` 再入（棚卸⑮）で起票。`areka-P0-ghost-shell-balloon-switch`（規模 L）の Approach ②③「バルーン切替・シェル切替」を**単独の spec として切り出した**。名前は `doc/ukadoc-coverage/roadmap-draft.md` 段階 B 順位 2 の束「切替」の候補名そのものである。
> 正典の語彙と Ref の一覧は親 brief（`.kiro/specs/areka-P0-ghost-shell-balloon-switch/brief.md` の Desired Outcome 2・3）が正本。本 brief は**再測定で崩れた前提と、切り出したあとの境界**だけを書く。
> 本文の file:line は**起票時の実測値**（2026-09-20・main `fe157df1`）。着手時に必ず引き直すこと。

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
