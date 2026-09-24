# Brief: areka-P0-shell-balloon-switch

> 2026-09-20 `/kiro-discovery` 再入（棚卸⑮）で起票。`areka-P0-ghost-shell-balloon-switch`（台帳 #13・規模 L）の Approach ②③「バルーン切替・シェル切替」を**単独の spec として切り出した**（台帳 #50）。名前は `doc/ukadoc-coverage/roadmap-draft.md` 段階 B 順位 2 の束「切替」の候補名そのものである。
> 正典の語彙と Ref の一覧は親 brief（`.kiro/specs/areka-P0-ghost-shell-balloon-switch/brief.md` の Desired Outcome 2・3）が正本。本 brief は**再測定で崩れた前提と、切り出したあとの境界**だけを書く。
> 本文の file:line は**起票時の実測値**（2026-09-20・main `fe157df1`）。着手時に必ず引き直すこと。

## 2026-09-24 棚卸⑯の再測定（main `0b01f654`）

**先進坑は別 spec に切り出した＝台帳 #59 `pilot-balloon-asset-swap`。** `two-tunnel.md` の規約（go 判定を本坑の前提依存とし、go 前の本坑着手は規律違反・1 仕様＝1 フォルダ・`pilot-` 接頭辞）により、同じ spec の先頭タスクにはできない。触るのは `crates/pilot/examples/pilot-balloon-asset-swap/` と `crates/pilot/Cargo.toml`（dev-dependencies に areka-emo-present・-compose・-atlas・-text・areka-parsers・wintf を足す＝pilot 側が依存するだけで葉ノードの隔離は崩れない）だけ＝**#55・#58・#13 と共有 0 で並走できる**。本仕様は roadmap に `_Depends(confirmed): pilot-balloon-asset-swap` を持つ。roadmap A3 行の旧記 `crates/pilot/examples/areka-P0-shell-balloon-switch/` は命名規約違反だったので直した。

**崩れた／変わった前提**

1. **起動時に最後のシェルを効かせる口は今も無く、しかも起動のたびに記憶が上書きされる。** `LastUsed::record`（`crates/areka/src/boot_resolve.rs`・`main.rs` の `on_boot_ok` が呼ぶ）は起動に成功するたびに `mount().shell.dir` の末尾を `LastShell` へ書く。`read_last_shell` は無い。**切替だけ実装すると次の起動で既定シェルが書き戻され、切り替えた記憶が消える**＝起動時の適用は省けない必須作業。開ける場所は ⓐ `boot_resolve.rs` に `read_last_balloon` を写した `read_last_shell(ghost_dir)`（数行）→ ⓑ `boot_config::resolve_boot_from` で `shell/<名>/` の実在を確かめる（無ければ `warn!` して既定へ）→ ⓒ `ConfigInputs` に `shell` の欄 → ⓓ `wire_emo2_boot` の引数と `ghost_boot_options` へ流す。
2. **`resolve()` の本番の呼び出しは今も 4 か所だが、シェル名が効くのは 3 か所**（`boot_with_kanade_stop`・`build_boot_assets`・`load_descript_source`）。`load_restored_state`（`placement/persist.rs`）は `model.shiori.dir` しか使わない。引数は `(ghost_root, default_encoding)` のまま＝#12 はシェル名を足していない。**`resolve.rs` は 952 行**で関数を 1 つ足すと 1,000 行を超える→ `resolve_with_shell(root, enc, Option<&str>)` を**隣の新ファイル**に作り `resolve` はそれに `None` で委ねる（テストの呼び出し約 30 本を直さずに済む）。
3. **`areka-ghost` へ渡すのは引数が安い。** `GhostBootOptions {` は 27 か所・16 ファイル（不変）。`boot_with_kanade_stop` の引数にすれば呼び出しは 3 か所。
4. **`catalog::list_shells` は `menu,hidden` を除外する**（`crates/areka-ghost/src/catalog.rs`）。正典では hidden も名指しなら切り替えられるので、名指しの検証と起動時の復元には `shell/<名>/` の実在を別に確かめる。
5. **正典の見落とし 2 件**（ukadoc `\![change,shell,…]`／`\![change,balloon,…]`）: シェル名・バルーン名として `random`／`lastinstalled` も受ける／`raise-event` のときはバルーンブレークで切替を中止できる。どちらも In に無い（議題 4）。
6. 差し替えの語彙 0 件は不変（較正の `pub enum` 4 種は 4 件。seriko の場所は `crates/areka-seriko`・`SerikoMsg` は `src/actor.rs`＝brief の `areka-emo-seriko` は誤記）。現状: `SerikoMsg` Cue／Tick／Close・`TextMsg` Cue／Close・`PresentCommand` ShowSurface／Hide／InvalidateCache・`DispatcherMsg` 7 腕。**`EmoPresenter::attach_target`（`areka-emo-present/src/presenter/hub.rs`）は同じ id を再登録すると表示コンテキストごと置き換える**＝差し替えの最小の部品になり得る（先進坑が確かめる対象）。
7. **`OnShellChang|OnBalloonChange` は crates/ 全体で 0 件**（ukadoc-survey も含む。`OnGhostChang` は `diff_tests.rs` で 7 件＝検索は効いている）。
8. **検体**: `R_POST_and_KOMAINU.nar` のシェルは `master` 1 つ（43 ファイル）。`SAMPLES` は 7 件（2026-09-24 に `claudia` が入った）・`lib_tests.rs` に `SAMPLES.len(), 7` が 2 か所とコメント「7 つ」3 か所・`vendors/sample_ghost/README.md` の表にファイル数と大きさ。**実機で応答を観測できる検体がある**: R_POST の `dic02_Event.txt` は `OnShellChanging`／`OnShellChanged` に、emo2 の `update.pasta` は `OnBalloonChange` に、konnoyayame は 3 つすべてに応答する。
9. メニュー: `Frame::Shell`／`Frame::Balloon` と文言（`shellrootbutton.caption`／`balloonrootbutton.caption`・`captions.rs`）は在り、登記は `wire_menu_with` の Readme・Close の 2 つだけ。`menu::register` の外からの呼び手は 0。
10. kanade 5 ファイル: `msg.rs` 771／`actor.rs` 507／`schedule/mod.rs` 751／`schedule/events.rs` 431／`schedule/steady.rs` **935**。`emo2_boot/` の関わるファイル: `assets.rs` 409・`frame/attach.rs` 441・`frame.rs` 459・`consumer_ledger.rs` 726・`placement/spawn.rs` 765・`source.rs` 287・`persist.rs` 512。`wire_emo2_boot` は今も一発の構築（本番の呼び出しは `main.rs` の 1 か所）。
11. 陳腐化 1 件（本仕様が直す）: `doc/ukadoc-coverage/ledger/shiori.toml` の `shellrootbutton.caption` の備考が引受先を `areka-P0-ghost-shell-balloon-switch` と書いている＝分割後は本仕様の担当。

**タスク数**: #13（と #58）が汎用の通知の入口と `SwitchRequest` を用意してから着手すれば **13〜16 本**（brief の 12〜15 とほぼ同じ）。待たずに着手すると通知の入口の自作＋kanade 3 ファイルで +3〜4、`random`／`lastinstalled` で +1〜2＝17〜20 で M を超える。**分割は不要**（「シェル名の運搬と起動時の適用」4〜5 本は独立しているが `boot_config.rs`／`main.rs` を #13 と共有するので直列のまま）。

**要件段階の議題（Fable）**: ⑴ 2 つ目のシェルの検体をどう用意するか——`R_POST_and_KOMAINU.nar` を畳み直す（README の表と `lib_tests` の数が変わる・改変してよいかのライセンス確認が要る）か、テストのときに展開先で `shell/master` を複製する（`.nar` は変わらないが実機の往復は手作業）か。⑵ `\![change,shell,名]` の「名」をフォルダ名と descript の `name` のどちらで引くか（記憶の鍵はフォルダ名）。⑶ 会話の途中で切り替えたとき、表示中のバルーンの文字と残りの台本を引き継ぐか消すか（「1 フレームも崩さない」保証の形がこれで決まる）。⑷ `random`／`lastinstalled`（シェルとバルーン）と `raise-event` 時のバルーンブレークによる中止を In に入れるか。案 A／B の選択は**先進坑 #59 の go 判定で決める**（要件の前）。

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
