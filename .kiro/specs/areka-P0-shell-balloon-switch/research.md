# ギャップ分析: areka-P0-shell-balloon-switch

> 実測: 2026-09-30・本ブランチ（`74b9de72`＝main `d79ca8dc` の上に spec の初期化だけ）。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。
> 対象: 確定した `requirements.md`（要件 1〜12）と今のコードの差。決めることは決めず、選択肢と材料だけを書く。

## 1. 要約

- **土台はおおむね在る。** 汎用の通知の入口 `KanadeMsg::RaiseEvent` は返信 `RaiseOutcome`（`Script`／`NoReply`／`NotSteady` ほか）を返せる。メニューの枠の登記（`menu/ghost_frame.rs` が手本）、目録（`catalog::list_shells`／`list_balloons`）、記憶の書き手（`record_last_used`）、present の装着（`EmoPresenter::attach_target`）、文字の層の毎フレームの結び直し（`run_text_scale_phase`）もそろっている。
- **足りないものは次の 5 つ。**
  - ⑴ present の「古い装着を片付ける口」と「登録を消す口」
  - ⑵ seriko の定義（面の別名表・着せ替え・アニメ表）を差し替える語。今は起動時に値で渡したきりで、UI 側は seriko への送出端すら持っていない
  - ⑶ 「今のシェル」の置き場。`GhostRuntime::mount()` は読むだけで書き換えられない
  - ⑷ シェル名を 4 か所の `resolve()` へ運ぶ口と `read_last_shell`
  - ⑸ `OnShellChanging` の台詞の終わりと中断を UI が知る手段
- **要件どうしの衝突が 2 つある（議題にする）。**
  - 要件 5.2（`OnShellChanging` の台詞を中断しても、`\-` があるだけでは終了しない）は、kanade を許可表の外で直さないと成り立たない。要件 8.2（kanade で触るのは許可表だけ）とぶつかる。
  - 要件 1.12 の後半（シェル・バルーンを切り替えている間はゴースト切替を断る）は、`ghost_switch.rs` に触らない約束（要件 8.1）のもとでは、台本から来るゴースト切替を止められない。
- **要件が書いていない、シェル固有の起動時の値が多い。** 配置は起動時にシェルの `descript.txt` から、揃え方・既定位置・バルーンのずらし・`seriko.zorder`・作者の DPI・kero の有無と窓の寸法を読む。これを差し替えのときに読み直すかどうかを決める必要がある。
- **規模は L 寄り、危うさは中〜高。** brief の見立ては 15〜18 タスク。上の穴を足すと 18〜22 になり、上限 20 に触れる。先進坑が確かめたのはバルーン窓の差し替えだけ（拡大率 100%）で、キャラ窓の差し替え（SERIKO のアニメ・当たり判定・着せ替え）と拡大率 200% はまだ誰も見ていない。

## 2. 今のコードの姿（要件に関わるところだけ）

### 2.1 起動の組み立て（一発の構築）

- `crates/areka/src/emo2_boot/mod.rs` の `wire_emo2_boot` が起動を一度に組み立てる。順序は次のとおり。
  1. `build_boot_assets_for` → `build_boot_assets` で、シェルとバルーンの資産を同時に作る。
  2. `spawn_seriko` に `resolver`・`static_binds`・`bind_resolver`・`SerikoLoopConfig { shell_table, balloon_tables, rng }` を値で渡す。
  3. `GhostBootOptions.sinks` に 10 本の受け口を渡して `areka_ghost::boot_with_origin` を呼ぶ。10 本の順は surface・clocked_text・move・lifecycle・zorder・readme・no_user_break・change・install・update。
  4. `Emo2Wiring` を World に据える。
  5. `seed_zorder_descript_base` で `seriko.zorder` を 1 度だけ据える。
- `build_boot_assets`（`emo2_boot/assets.rs`）は `resolve()` を自分でも呼ぶ。WIC の復号器を内部で作るので COM を初期化したスレッドが要る。本番の呼び手は UI スレッドだけ。
- 戻り値 `BootAssets` は `shells`・`balloons`・`resolver`・`static_binds`・`bind_resolver`・`loop_tables`・作者の DPI 2 つ。片側だけを作る関数は無い。
  - `ShellTarget::build_world`（`areka-emo-present/src/shell_target.rs`）は、読み込み済みの `ShellTarget` から入出力なしで `EmoWorld` を作り直せる。ただし `build_boot_assets` は `ShellTarget` を手元に残していない。
- `EmoWorld`（`areka-emo-compose/src/world.rs`）は `Clone` でない。`attach_target` は値で受けて消費する。`AtlasTable` は `Clone`＋`Send + Sync`。

### 2.2 毎フレームの相と装着

- `emo2_frame_system`（`emo2_boot/frame.rs`）は `Update` の段で走る。主な順は次のとおり。
  1. `run_attach_phase`
  2. `run_dpi_phase`
  3. `run_drain_phase`（`PresentCommand` を適用する）
  4. `run_balloon_visibility_phase`
  5. `reconcile_reported_sizes`（窓寸の要求を窓へ当てる）
  6. `run_move_drain_phase`
  7. `run_zorder_drain_phase`
  8. `resnap_shell_targets`
  9. `run_text_scale_phase`
  10. `run_text_phase`
- `run_attach_phase`（`frame/attach.rs`）は `wiring.attached` の旗で 1 度だけ走る。
  - シェル: `attach_target` だけ行う。最初の `\s` が来るまで面は出さない。
  - バルーン: `attach_target` → `set_visibility_ownership(External)` → `ShowSurface{0}` → `connect_balloon_text` → `wiring.balloon_models.insert` の順。
  - 対象の番号は `2*scope`（シェル）と `2*scope+1`（バルーン）。
- present（`areka-emo-present/src/presenter/`）の状態は次のとおり。
  - `EmoPresenter.targets: HashMap<TargetId, PresentTarget>`。
  - `PresentTarget`（`presenter/target.rs`）は、可視性の持ち主 `ownership`・今の面 `current_surface_id`・拡大の方針 `policy`（作者の DPI・装着時に固定）・`applied`・`native_size`・`last_show`・`pending_resize` を持つ。
  - 公開の口は `attach_target`・`apply`・`set_visibility_ownership`・`show_target`・`refresh_scale`・`take_pending_resize`・当たり判定の口・読み出しの口だけ。**片付ける口と登録を消す口は 0 件**。`VisualMount`（`mount.rs`）は `Drop` を持たないので、表から外しても子の実体は窓に残る（先進坑の学び 1 と同じ）。
- 当たり判定は 2 系統あり、どちらも装着の側に在る。
  - 名前付きの当たり判定: `EmoWorld` の面の定義にある `collisions` を、`EmoPresenter::hit_region_client` が今の面で引く。
  - 透過のマスク: 子 `emo-surface` の `AlphaMaskResource` を、`apply_show` が面ごとに差し替える。
  - したがって古い子を消して新しく装着すれば、当たり判定も一緒に替わる。
- 拡大率: 係数 `k` は毎回の `ShowSurface` で窓の `DPI` と装着時の `policy` から導く。窓を残せば DPI も引き継がれる。ただし**作者の DPI（`policy`）は装着のたびに渡し直す**ので、新しいシェル／バルーンの `seriko.dpi`／`dpi` を読み直すかどうかを決める必要がある。
- 文字の層: 幾何（折り返し・有効矩形・フォント）は `TextMsg` では運ばない。UI スレッド側の `TextLayerRuntime` の `register_actor_view`（装着時）と、毎フレームの `refresh_actor_binding`（`run_text_scale_phase` 経由）で `BalloonModel` から結ぶ。
  - `wiring.balloon_models[scope]` を差し替え、新しい子 `emo-text-layer-slot` を装着すれば、同じ tick の `run_text_scale_phase` が結び直して古い描画の持ち物を捨てる。
  - `register_actor_view` を呼び直すと古い描画が残るので、使わない。
  - 背景色は `set_balloon_background` で渡す。
  - **`areka-emo-text` の `sink.rs`（`TextMsg`）には触らずに済む見込み**（brief は「触るのは本仕様だけ」と書くが、触る必要は見えていない）。

### 2.3 seriko（面・着せ替え・アニメ）

- `SerikoMsg`（`areka-seriko/src/actor.rs`）は `Cue`・`Tick`・`Close` の 3 腕。
- シェルに依る持ち物は、別名表 `SurfaceResolver`・静的な着せ替え `static_binds`・着せ替えの名前表 `BindResolver`・シェルのアニメ表 `shell_table`。バルーンに依る持ち物は、スコープ別のバルーンのアニメ表 `balloon_tables`。どれも spawn のときに値で渡し、`LoopRuntime.config` は「以後不変」と書かれている。
- 着せ替え（MAYUNA）の状態は `ScopeStates`（`areka-seriko/src/state.rs`）が持つ（`static_binds`・`dynamic_binds`・`pattern_states`）。今の面は `ScopeStates.scopes`（シェル）と `.balloon`（バルーン）。
- UI 側（`Emo2Wiring`）は `SerikoSink` を持たない。`tick_sink` はループの ticker の閉包へ移したきりで、`Emo2BootOutcome` が返すのは `ActorHandle` だけ。
- **バルーンの切替も seriko に触る**（`balloon_tables` の差し替え）。要件 3.5 の「SERIKO のアニメ…は不変」は、シェル側のアニメの意味に読む必要がある。

### 2.4 台本の受け口と台帳

- `ChangeCueSink::emit`（`emo2_boot/change_cue.rs`）は `("change","ghost")` だけを受ける。他は `debug!(change_cue_skip)` で読み飛ばす。`--option=raise-event` 以外の選択肢は `warn!(change_cue_unknown_option)` を残して送る。
- 送る型 `ChangeRequestRaw` と受信端 `ChangeRx`、取り出しの系 `drain_change_requests` は、**どれも `ghost_switch.rs` の中**に在る。
- `ConsumerLedger::canonical()`（`emo2_boot/consumer_ledger.rs`）は 13 行。テストは 2 本ある。
  - `canonical_builds_without_duplicate`: 13 を直書きしている。
  - `canonical_registers_only_change_ghost_for_the_change_sink`: `(change,shell)`・`(change,balloon)`・裸の `change` が登記されていないことを固定している。
  - `SelectorConflict` の規則により、裸の `("change", None)` は登記できない。

### 2.5 汎用の通知の入口（kanade）

- `KanadeMsg::RaiseEvent { id, references, method, reply }`（`areka-kanade/src/msg.rs`）。
- 返信 `RaiseOutcome`（`areka-kanade/src/change.rs`）は `NotAllowed`／`NotSteady`／`Script`／`NoReply`／`Failed`。
- 返信は最初の往復とその後の動作（トークの開始を含む）が済んだ直後に 1 回だけ来る（`actor.rs` の `spawn_kanade_with_stop_sink`）。**`Script` は「台詞を始めた」までで、台詞の終わりは知らせない。**
- `on_raise_event`（`schedule/change.rs`）の判定: 許可表（`events::allowed_static`）→ 定常（`Phase::Steady`・再生中を含む）→ `events::raise`。`pending_close` は見ない。
- 応答の台本は定常の `on_reply`（`schedule/steady.rs`）が始めるか、`value_replaces_active_talk`（`OnSecondChange` 以外）なら今のトークを置き換える。
- 許可表 `ALLOWED_EVENT_IDS`（`schedule/events.rs`）は 42 語。直書きは次の 2 か所（要件 8.2 のとおり）。
  - `events_change_tests.rs` の `assert_eq!(ALLOWED_EVENT_IDS.len(), 42)`（テスト `allowed_static_returns_the_table_spelling_for_the_two_change_events` の中）
  - `events_tests.rs` の `allowed_event_ids_are_exactly_the_forty_two_and_exclude_ontalk_onhour`（42 語を順に完全一致）
  - 足すときは、表の doc コメントとテスト名も追随させる。
- `KanadeNotice::Steady` を出すのは起動の完了のとき（`schedule/boot.rs`）だけ。UI から kanade の相を読む資源は無い。

### 2.6 ゴースト切替との関係

- `request_ghost_switch_with`（`ghost_switch.rs`）が「重ね」を判定するのは `SwitchInFlight` の有無だけ。更新・インストール・シェル切替の状態は見ない。
- 本番の呼び手は 4 か所: `drain_change_requests`（`ghost_switch.rs`）・`ghost_frame_item`（`menu/ghost_frame.rs`）・`install/overwrite.rs` の `request`・`update/desk.rs` の `reload`。
- `SwitchInFlight` は `pub(crate)` なので、本仕様の側から読める。`update::submit` や `install` もそうしている。
- kanade 側の `on_change_ghost` は、定常でトークを再生中なら切替を `pending_change` に積む。つまり `OnShellChanging` の台詞の最中に来たゴースト切替は、台詞の後に始まる。
- `OnGhostChanging` の「台詞を待つ・中断で中止」は kanade の相（`ChangeTalkWait`・`cancel_by_user_break`）に在る。中断による中止は `KanadeNotice::ChangeCancelled{UserBreak}` で UI に届く。汎用の入口の台詞には、これに当たる通知が無い。

### 2.7 中断と終了

- UI の中断（`input_events/user_break.rs` の `press_with_visibility`）は、`TalkLifecycleSignal::UserBreak` を lifecycle の線へ、`KanadeMsg::UserBreak` を kanade へ送る。UI は「自分が中断を送った」ことは知っているが、それが何を意味したかは知らない。
- kanade の `on_talk_done`（`schedule/mod.rs`）では、`break_quit` は `take_user_break_quit(..) && !change::is_change_phase(..)` で決まる。**定常のトーク（＝汎用の入口で送った `OnShellChanging` の台詞）を中断し、それが `\-` を予約していたら、終了系列へ進む。**
- lifecycle の線（`TalkStarted`／`DisplayEndAt(f64)`／`UserBreak`）は mpsc で、本番で読むのは `run_balloon_visibility_phase` の `drain_lifecycle` だけ（毎 tick 全件取り出す）。他の相が同じ信号を読むには、写しを出してもらうか、可視性の状態を読む口が要る。
  - 可視性の状態 `BalloonVisibilityState` の `display_end`・`deadline`・`break_latch` は非公開。
- 終了要求は 2 つの経路で来る。
  - メニューの「終了」と `WM_CLOSE` は kanade への `CloseRequest` になる。
  - OS のセッションの終了は `end_session_from`（`session_end.rs`）が扱い、`SwitchInFlight` を直接外す。

### 2.8 シェル名の運搬と記憶

- `resolve(ghost_root, default_encoding)`（`areka-parsers/src/package/resolve.rs`・952 行）はシェル名を引数に取らない。返す `MountModel`・`ShellMount` は `#[non_exhaustive]` で、他の crate からは組めない。本番の呼び手は 4 か所。
  - `boot_with_origin`（`areka-ghost/src/runtime.rs`）
  - `build_boot_assets`（`emo2_boot/assets.rs`）
  - `load_descript_source`（`placement/source.rs`）
  - `load_restored_state`（`placement/persist.rs`・`shiori.dir` しか使わない）
- `GhostRuntime.mount` は非公開の欄で、口は読むだけの `mount()`。起動後に替える手段が無い。
- `mount().shell.dir` を読む本番の場所は 4 つ。
  - `on_boot_ok`（`main.rs`）→ `record_last_used`
  - `record_steady_memory`（`ghost_switch.rs`）
  - `here`（`update/desk.rs`）
  - `boot_with_origin` の sylphya の Shell スコープの根（`<shell.dir>/profile/areka/`）
- kanade の `shell_folder`／`shell_name` は起動時にだけ使う（`OnBoot` の Ref0・`OnGhostChanged` の Ref7）。
- 記憶まわりの今の形。
  - `boot_resolve.rs` の読み手は `read_last_ghost`・`read_last_balloon`・`read_session_mark` の 3 つ。`read_last_shell` は無い。
  - `LastUsed::record` は `LastGhost`（App・argv 以外）・`LastBalloon`（Ghost・argv 以外）・`LastShell`（Ghost・常に）を 1 度に投函する。
  - `record_last_used(publisher, shell_dir, &GhostDecision, &BalloonDecision)` は **3 つを必ず一緒に書く**。
- `BootContext.current: CurrentGhost { cfg, ghost, balloon }`（`boot_config.rs`）。書き換えるのは `main.rs`（据え付け）と `ghost_switch.rs` の `boot_into` だけ。`BalloonRoute` に「切替で決まった」の腕は無い。
- インストールの完了（`install/desk.rs` の `record_installed`）の扱いは種類で分かれる。
  - ゴースト: `record_last_installed` を呼ぶ。
  - バルーン: `remember_balloon`（今のゴーストの `LastBalloon` を書き換えるだけ）を呼ぶ。
  - シェル: 何もしない。
  - 「最後に入れたバルーン」をプロセスの中に控える資源は無い。

### 2.9 配置（シェルの `descript.txt` から起動時に読む値）

- `build_placement_config`（`placement/config.rs`）は、ゴーストとシェルの kv（シェルが優先）から次の値を読む。
  - `seriko.alignmenttodesktop`／`alignmentondesktop`
  - 既定位置 `defaultx`・`defaulty` ほか
  - `balloon.alignment`・`balloon.offsetx`／`offsety`
  - `seriko.zorder`・`seriko.sticky-window`・`seriko.dpi`
  - `kero.*`（スコープの検出）
- 窓の寸法は `measure_scope_sizes(&src.shell_dir, …)` で決まる。
- 位置の記憶（`WindowPos`・`BalloonOffset`）はゴーストの Ghost スコープにあり、シェルでは分けていない。
- 切替の経路（`boot_into` → `reopen_ghost_windows` → `prepare_ghost_windows`）はこれを全部やり直す。本仕様はキャラ窓を作り直さないので、**どれを読み直すかを自分で決める**必要がある。

### 2.10 メニュー・目録・検体・台帳

- `Frame::Shell`／`Frame::Balloon` は `Frame::ORDER` と文言（`captions.rs` の `FRAME_CAPTIONS`）に在る。登記は 0 件。
- `boot_wired`（`ghost_session.rs`）は `wire_menu` の直後に `ghost_frame`・`install_frame`・`update_frame` の `register` を呼ぶ。
- 手本の `ghost_frame_item` の作り。
  - 子はフォルダ名で指す（`GhostSpec::Folder`）。
  - 今のものに印を付ける。
  - 子が 0 なら選べない見出しにする。
- `catalog::list_shells`（`areka-ghost/src/catalog.rs`）は `menu,hidden` を除く。`update/desk.rs` の `updateother` もこの除外に依っている。
- `Identity` は 7 項目。
- 検体（`sample-ghost-kit`）。
  - `SAMPLES` は 7 件。シェルを 2 つ持つ検体は 0（`.nar` の中を見て確かめた）。
  - `SampleRoot::acquire` は使い捨ての複製を返す。展開後の検体を書き換える部品は無い。
  - バルーンは `claudia` が 2 つ、`emo2` が 1 つ同梱している。
- 台帳。
  - `shiori.toml` の `OnShellChanging`・`OnShellChanged`・`OnBalloonChange` は `absent`・owner 空。
  - `shellrootbutton.caption`／`balloonrootbutton.caption` は `vocabulary-only`。備考の引受先が古く、「登記は説明書と終了の 2 枠だけ」とも書いてあるが、今はゴースト・インストール・更新も在る。
  - `sakura-script.toml` の `\![change,shell,…]`・`\![change,balloon,…]` は `absent`。
  - 生成器は `cargo run -p ukadoc-survey -- report`／`report-summary`。
  - 付記: `ghostrootbutton.caption` もゴースト枠の登記後なのに `vocabulary-only` のまま。本仕様の外の陳腐化の疑い。

### 2.11 行数（要件の「行数」節から変わったもの）

| ファイル | 要件の値 | 今 |
|---|--:|--:|
| `emo2_boot/mod.rs` | 805 | **825** |
| `emo2_boot/frame.rs` | 509 | **533** |
| `emo2_boot/consumer_ledger.rs` | 756 | **831** |
| `ghost_session.rs` | 691 | **705** |
| `main.rs` | 927 | **946**（上限まで 54） |
| kanade `schedule/events.rs` | 535 | **627** |
| `areka-ghost/src/catalog.rs` | 324 | **362** |
| `ghost_switch_tests.rs` | 987 | **988** |

変わっていないもの: `resolve.rs` 952・`presenter/hub.rs` 176・`command.rs` 231・`boot_resolve.rs` 471・`boot_config.rs` 357・`runtime.rs` 739・`steady.rs` 935・`assets.rs` 409・`frame/attach.rs` 441・`frame/wiring.rs` 324・`update/desk.rs` 657・`runtime_tests.rs` 986・`assets_tests.rs` 976・kanade `actor_tests.rs` 970。`GhostBootOptions {` は 31 か所・20 ファイル（うち本当の構造体リテラルは 29）。

## 3. 要件と資産の対応（欠け: 無＝Missing／未知＝Unknown／制約＝Constraint）

| 要件 | 使える資産 | 欠け |
|---|---|---|
| 1.1 入口 1 本 | `ghost_switch.rs` の形（要求の型＋判定の関数＋取り出しの系） | 無: シェル・バルーン用の要求の型と入口 |
| 1.2〜1.4・1.15 台本 | `ChangeCueSink`・`ConsumerLedger` | 無: `(change,shell|balloon)` の受理と、送り先の線（`ChangeRequestRaw`／`ChangeRx` は `ghost_switch.rs` の中なので流用できない）。制約: 台帳のテスト 2 本を書き換える（13 → 15・「3 組が登記され、裸は無い」へ） |
| 1.5・7 メニュー | `ghost_frame.rs` の型・`menu::register`・`boot_wired` | 無: `shell_frame.rs`・`balloon_frame.rs`。要件 7.1「1 つでも枠を出す」は手本（子 0 なら選べない）と同じ作りで足りる |
| 1.6 名前解決（隠しを含む） | `ghost_switch.rs` の `resolve_switch_target`（name → フォルダ名の規則） | 無: 隠しシェルも列挙する目録の口（`list_shells` は除外する・`updateother` がその除外に依る）。制約: `Identity` に欄を足さない |
| 1.9 `random` | `boot_resolve::pick_index` | 無: シェル・バルーン用の解き手（`resolve_special_name` は `&[GhostEntry]` 専用） |
| 1.10 シェルの `lastinstalled` | `request_ghost_switch(GhostSpec::Name("lastinstalled"))` | なし（呼ぶだけ） |
| 1.11 バルーンの `lastinstalled` | `LastInstalledGhost` と同じ形・`record_installed` のバルーンの腕 | 無: プロセスの中の控え（資源 1 つ＋書く 1 行） |
| 1.12 重ね禁止 | `SwitchInFlight`（読める） | 無: シェル切替の進行中の印。**制約: ゴースト切替の入口がその印を見ない**（議題 D） |
| 1.13 更新中 | `UpdateDesk.stage` | 無: crate から読める問いの口（`stage` は `pub(super)`） |
| 1.14 定常でない | `RaiseOutcome::NotSteady` | 未知: `OnShellChanging` を送らない切替では、差し替えの前に kanade の相を知る手段が無い（議題 G） |
| 2.1・2.4・3.2 通知と Ref | `KanadeMsg::RaiseEvent`・許可表 | 許可表に 3 語（42 → 45）。Ref の名前は `catalog::identity` の `name` → フォルダ名 |
| 2.2・2.3・3.1 台詞の切れ目 | `RaiseOutcome::Script`／`NoReply`・lifecycle の線 | **無: 台詞の終わりと中断を UI が知る口**（`Script` は開始まで・線を読むのは 1 か所だけ）（議題 B・C） |
| 2.5 SHIORI を降ろさない | 汎用の入口 | なし |
| 2.6 同じ面番号・着せ替えを既定へ | `EmoPresenter::current_surface_id`・seriko の `ScopeStates` | 無: seriko の定義の差し替えの語（別名表・`static_binds`・`BindResolver`・`shell_table`、動的な着せ替えは消す）と、UI から seriko への送出端（議題 E） |
| 2.7 窓を残して寸法とバルーンの位置 | `pending_resize`→`reconcile_reported_sizes`・`resnap_shell_targets` | 未知: シェル固有の配置の値（ずらし・揃え方・zorder・作者の DPI・kero）を読み直す範囲（議題 F） |
| 2.8 SERIKO の始め直し | `SerikoLoopConfig` | 無: 同上の語（ループの実行時の状態の扱いを含む） |
| 3.3 バルーンの組み直し | `connect_balloon_text`・`run_text_scale_phase`・`wiring.balloon_models` | 無: 片付けの口。`balloon_tables`（seriko）と作者の DPI の差し替え |
| 3.4・6.5 今のバルーン・今のシェル | `BootContext.current.balloon`（書ける） | **無: 今のシェルの書ける置き場**（`mount()` は読むだけ）（議題 A） |
| 4.1〜4.3・4.6 崩れない差し替え | 先進坑の本命の版・`attach_target` | 無: `EmoPresenter` の片付けの口（子の despawn を含む）と登録を消す口、可視性の持ち主と窓寸の要求の引き継ぎ |
| 4.4 復号を tick の外へ | `build_boot_assets`（UI スレッド・COM 要） | 無: 片側だけ作る分割と、作る場所（議題 H） |
| 4.5 拡大率 1 以外 | 窓の `DPI` を毎回読む作り | 未知: 200% での実測（先進坑は 100% だけ） |
| 5.1・5.3 中断で中止 | UI の `press_with_visibility` | 無: 「`OnShellChanging` の台詞を中断した」ことの判定（議題 B） |
| 5.2 `\-` でも終了しない | — | **制約: kanade の `on_talk_done` は定常のトークの中断＋`\-` を終了へ結ぶ**（議題 C） |
| 5.4 命令の後の中断・`\-` は終了が勝つ | kanade の定常の規則 | 未知: UI 側で「終了系列に入った」ことを知る時点 |
| 5.5・5.6 失敗は元のまま | 資産づくりを先に・装着は後に | 制約: 片付けた後に失敗しうるのは装着の登録だけ。`attach_target` は今は必ず `Ok` を返す |
| 5.7 終了が勝つ | `CloseRequest`・`end_session_from` | 無: 切替の保留を終了の経路で捨てる 1 か所（`session_end` と `quit` の経路に掛かる） |
| 6.1 記憶を書く | `record_last_used` | **制約: 3 つを一度に書く**（議題 I） |
| 6.2〜6.4・6.6 起動の復元 | `read_last_balloon` の写し・`Emo2BootInputs`・`boot_with_origin` | 無: `read_last_shell`・`Emo2BootInputs` の欄・`boot_with_origin` の引数・`resolve` の兄弟（`resolve.rs` 952 行なので新ファイル）。**配置の 2 か所（`load_descript_source`・`build_boot_assets`）にも渡す口**（`prepare_ghost_windows` は `boot_wired` より前に走る） |
| 6.7 argv のバルーン | `BalloonRoute::Argv` | 未知: argv のバルーンから切り替えた後に `LastBalloon` を書くか（議題 I） |
| 8.6 同期の送信 0 | `ALLOWED_SYNC_SENDS` | なし |
| 9.1 検体の部品 | `SampleRoot` | 無: `shell/master` を写して `name` を変える部品 |
| 10 台帳と §8 | 生成器 `ukadoc-survey` | なし（書くだけ） |
| 11 テスト | 偽 SHIORI・`sample-ghost-kit`・present 単体 | 制約: 上限の近いテスト 5 本には足さない |

## 4. 実装の方向（案）

### 案 A: 既存の部品を広げる（受け口・入口・差し替えを既存ファイルへ）

- `ChangeCueSink` に 2 本目の送出端を持たせる。
- 要求の型と入口は `ghost_session.rs` か `emo2_boot/mod.rs` に置く。
- 差し替えは `run_attach_phase` の中に「2 度目の装着」として足す。
- ✅ ファイルが増えない。
- ❌ `consumer_ledger.rs` 831・`mod.rs` 825・`main.rs` 946 が上限に近い。`run_attach_phase` は「1 度だけ」の旗に依る作りなので、2 度目を混ぜると判断の分岐が読みにくくなる。

### 案 B: 新しい部品を立てる（brief の「接触ファイル」の形）

- 新しいファイル。
  - `emo2_boot/switch_cue.rs`: `\![change,shell|balloon]` の受け口＋線（11 本目の受け口）
  - `emo2_boot/shell_balloon_switch.rs`（仮）: 要求の型・入口・名前解決・`random`・`lastinstalled`・進行の印・待ちの相
  - `emo2_boot/frame/switch.rs`: 差し替えの相
  - `menu/shell_frame.rs`・`menu/balloon_frame.rs`
  - `areka-parsers/src/package/resolve_shell.rs`
- 既存の側には口だけを足す。
  - present: 片付けの口・登録を消す口
  - seriko: 定義の差し替えの語
  - `catalog`: 隠しを含む列挙
  - `boot_resolve`: `read_last_shell`
  - `update/desk.rs`: 実行中の問い
  - `GhostRuntime`: 今のシェルの口（議題 A）
- ✅ 判断の分岐を新しいファイルに閉じられる。既存の相の作りを崩さない。テストを兄弟ファイルに置きやすい。
- ❌ 接点が多い。present・seriko・text・placement・runtime の 5 層にまたがるので、差し替えの 1 手順の順序（seriko の定義 → UI の装着 → 最初の面）を設計で正確に決める必要がある。

### 案 C: 混ぜる（段階を切る）

- 第 1 段: 基盤。present の片付けの口・seriko の差し替えの語・片側だけの資産づくり・シェル名の運搬と `read_last_shell`。どれも共有の少ない下の層なので、単体のテストで先に固められる。
- 第 2 段: 案 B の入口・待ちの相・差し替えの相・メニュー・記憶。
- 第 3 段: 台帳・§8・統合テスト・実機。
- ✅ 危うさの大きい「キャラ窓の差し替え」を早い段で単体に当てられる。
- ❌ 段の間の約束（seriko の差し替えと UI の装着の順序の決め）を第 1 段で先に決めておく必要がある。

**見立て**: brief と要件が既に案 B の形を前提にしている（`switch_cue.rs`・`frame/switch.rs`・`resolve_shell.rs`）。タスクの並べ方は案 C の段の切り方が合う。

## 5. 規模と危うさ

- **規模: L（18〜22 タスクの見込み）。** brief の 15〜18 に、次の分が加わる。
  - seriko の差し替えの語と、送出端の持ち方: +1
  - 台詞の終わりと中断を UI が知る口: +1
  - 今のシェルの置き場: +0.5
  - 配置の読み直し（範囲しだい）: +1〜2
  - 20 に触れたら、brief の切る順（① `random`／`lastinstalled` を追い spec へ ② メニューの 2 枠を 1 タスクに畳む）が使える。記憶「一度切り出した spec を少しまたぐ理由でさらに削らない」に従えば、議題にはせず仕事のまとめ方で抑える。
- **危うさ: 中〜高。**
  - キャラ窓の差し替えは実測が無い。seriko のスレッドと UI の間で、差し替えの前に送られた古いシェルの `PresentCommand`（`wiring.rx` に溜まっている）が新しい装着に当たる順序の穴がある。
  - kanade との約束（要件 5.2・1.12）が要件の境界とぶつかる。
  - 拡大率 200% は未観測。

## 6. 設計へ持ち越す調べもの（Research Needed）

1. **seriko と UI の差し替えの順序。** 差し替えの語を seriko へ送った時点で `wiring.rx` に溜まっている古いシェルの `ShowSurface` を、どう捨てるか。取り得る形は次の 3 つ。
   - 世代番号を付ける（`PresentCommand` は `#[non_exhaustive]`）
   - seriko の返事を待ってから装着する
   - 差し替えの tick で `wiring.rx` を読み捨てる
   - 1 フレームも遅らせない約束と両立する形を選ぶ。
2. **差し替え後の最初の面をどこから出すか。** UI（`current_surface_id`／`last_show`）から出すか、seriko（`ScopeStates` の今の面）に出し直させるか。アニメの途中のパターンの状態をどう扱うか。
3. **資産づくりを tick の外に出す場所。** COM（MTA）を初期化した背景のスレッドで作って UI へ渡せるか。`EmoWorld` の `Send` はコードで確かめられていない（中は bevy の `World` なので `Send` の見込み）。
4. **lifecycle の信号の写し方。** 可視性の相が取り出した信号を `Emo2Wiring` に写して出すか、可視性の相の後に置いた相が小さな状態の写しを読むか。`RaiseOutcome::Script` の返信と `TalkStarted` は別スレッド・別の線を通るので、届く順が決まらない。対応の取り方を決める必要がある。
5. **記憶の反映の時点。** `persist_put` は投函だけで、反映は `shutdown` の barrier に任せている。切替の直後に更新の読み直し（`boot_into` → `resolve_balloon_for_ghost` → `read_last_balloon`）が走ったとき、書いた記憶が読めることを確かめる（古いランタイムの `shutdown` が barrier を通る順序）。インストールの `record_installed` は `barrier()` を呼んでいるので、これが前例になる。
6. **sylphya の Shell スコープの根。** `boot_with_origin` はシェルの `profile/areka/` を Shell スコープの根にする。差し替えた後も古いシェルの根のままでよいか、張り替える口が sylphya に在るか。
7. **拡大率 200% での差し替え。** 先進坑は 100% でしか測っていない（要件 4.5・11.11 ⑥ で実機に回す）。
8. **`attach_target` の失敗の経路。** 今は必ず `Ok` を返す。要件 5.6 の「片付けた後の失敗は装着の登録だけ」を、どの失敗として定義するか。

## 7. 要件ディスカッションへの議題（答えで作業が変わるもの）

- **A. 「今のシェル」の置き場。** `GhostRuntime::mount()` は書き換えられない。一方 `here`（更新の対象）・`record_steady_memory`・`on_boot_ok` は `mount().shell.dir` を読む。
  - (a) `areka-ghost` の `GhostRuntime` に今のシェルを差し替える口を足す（`runtime.rs` は既に接触ファイル）。
  - (b) areka 側に「今のシェル」の資源を新しく置き、`here` と記憶の書き手を読み替える（`CurrentGhost` には欄を足さない約束を守れる）。
  - 要件 3.4・6.5 は「差し替え後を指す」とだけ書くので、どちらでも満たせる。brief ⑱-5 の「置き場は新しく作らない」は、`mount()` が書けない以上そのままでは成り立たない。
- **B. `OnShellChanging` の台詞の終わりと中断を UI がどう知るか。** `RaiseOutcome::Script` は開始までしか言わない。
  - (a) UI が推し量る。自分が送った `UserBreak`、`DisplayEndAt`、可視性の相の状態を使う。kanade は許可表だけ。別の応答がその台詞を置き換えた場合の扱いが要る。
  - (b) kanade に「この依頼の台詞が終わった／中断された」の通知を足す。`msg.rs`・`actor.rs`・`schedule/` に触るので、要件 8.2 と衝突する。
- **C. 要件 5.2 と要件 8.2 の衝突。** kanade の `on_talk_done` は、定常のトークの中断に `\-` の予約があれば終了へ進む。切替の相だけが例外になっている。汎用の入口で送る `OnShellChanging` の台詞は定常のトークなので、許可表だけを触る限り **5.2 は成り立たない**。
  - (a) kanade の判定に 1 か所だけ手を入れ、要件 8.2 を改める。
  - (b) 要件 5.2 を「定常の規則どおり終了が勝つ」へ改め、§8 に記す。
  - 完了 `ghost-shell-balloon-switch` の衝突表 2（切替の相の中断は終了に結ばない）を継ぐかどうかの問いでもある。
- **D. 要件 1.12 の後半（シェル・バルーンの切替中はゴースト切替を断る）。** `request_ghost_switch_with` は `SwitchInFlight` しか見ない。台本のゴースト切替の取り出し `drain_change_requests` も `ghost_switch.rs` の中にある。したがって要件 8.1 を守ると、台本からのゴースト切替は止められない。
  - (a) `ghost_switch.rs` に 1 行の問いを足し、要件 8.1 を改める。
  - (b) 呼び手（メニュー・インストール・更新）でだけ断り、台本からの分は通す。
  - (c) 逆向きにする。ゴースト切替が勝ち、シェル切替は `SwitchInFlight` を見て自分を取りやめる。kanade はゴースト切替を台詞の後へ積むので、この形は自然に起こる。要件 1.12 の書き方を改める必要がある。
- **E. seriko の定義の差し替えを、バルーンの切替でも行うこと。** バルーンのアニメ表 `balloon_tables` は seriko の中に在る。要件 3.5 の「SERIKO のアニメ…は不変」は、シェル側に限る意味に読み替えてよいか。UI に seriko への送出端（`SerikoSink` の複製）を持たせる置き場も要る（`Emo2Wiring` か `Emo2BootOutcome`）。
- **F. シェルを差し替えるとき、シェルの `descript.txt` から読む配置の値をどこまで読み直すか。** 候補は次のとおり。
  - 窓の寸法
  - 作者の DPI（`seriko.dpi`）
  - `balloon.offsetx/y`・`balloon.alignment`
  - `seriko.zorder`（`seed_zorder_descript_base` は「起動で 1 度」の作り）
  - `seriko.alignmenttodesktop`・既定位置・`sticky-window`・`kero.*`（スコープの数）

  選択肢は 3 つ。
  - (a) 寸法・作者の DPI・バルーンのずらしだけを読み直す。
  - (b) 配置の準備（`prepare_ghost_windows` の窓を作らない部分）をまるごと回し直す。
  - (c) 要件どおり「今日の配置の規則で置き直す」だけにし、descript 由来の値は起動時のまま（§8 に記す）。

  スコープの数が変わるシェル（kero の有無が違う）への切替をどう扱うかも決める（窓を作り直さない前提と衝突しうる）。バルーンを差し替えるときの作者の DPI（`dpi`）も同じ問い。
- **G. 定常でないとき（要件 1.14）の判定。** `raise-event` の無い切替とバルーンの切替は、kanade と往復する前に差し替える。UI は kanade の相を知らない（`Steady` の通知は起動の完了のときだけ）。
  - (a) 差し替えの前に、UI 側で分かる印（`SwitchInFlight`・終了の門 `is_closing`・`SessionEnded`）で近似する。
  - (b) 後で届く `OnShellChanged` の返信が `NotSteady` だったら記録だけ残す（差し替えは済んでいる）。
  - どちらも「定常でなければ変化 0」を厳密には保てない場面がある。
- **H. 新しい資産を作る場所。** `build_boot_assets` は UI スレッドの COM に依る。
  - (a) 背景のスレッドで作る（COM の初期化を足す・`EmoWorld` の `Send` を確かめる）。
  - (b) UI スレッドの `Input` の段で作る。フレームの相の外ではあるが、UI スレッドは止まる。要件 4.4 の「約 16 ms の内側を目標」とは、実機で測ってから比べる。
- **I. 記憶の書き方。** `record_last_used` は `LastGhost`・`LastBalloon`・`LastShell` を一度に書く。シェルだけを替えたときにこれを通すと、インストール直後に `remember_balloon` が書いた「次から使うバルーン」を、今表示しているバルーンで上書きしてしまう。
  - (a) `LastShell` だけ・`LastBalloon` だけを書く狭い口を `LastUsed` の隣に足す（要件 6.1 の「既存の書き手を通す」を改める）。
  - (b) `record_last_used` を通し、上書きを受け入れる。
  - argv でバルーンを指定して起きたプロセスでバルーンを切り替えたとき、`LastBalloon` を書くか（要件 6.7 は「argv の側は書かない」とだけ書く）。書くなら `CurrentGhost.balloon.route` を何にするか。`BalloonRoute` に腕を足すことは「欄を足さない」約束には当たらない。
- **J. 「バルーンの表示が終わった時点」の定義（要件 2.2・2.3・3.1）。** 次の 2 つのどちらかを決める。
  - 文字を出し終えた時点（`DisplayEndAt`）。この時点ではバルーンはまだ見えていて、可視性の相のタイムアウトで後から隠れる。
  - バルーンが隠れた時点。数秒待つことがある。

  バルーンの切替で「差し替えの直後は隠れたまま」（要件 3.3）を守るには、次のどちらかになる。
  - (a) `DisplayEndAt` で差し替え、残っているバルーンをその場で隠す。
  - (b) タイムアウトで隠れるまで待つ。

  シェルの切替は、どちらでも表示は崩れない。

（参考・議題にしない how）

- 台本の受け口は、新しい受け口（11 本目）と `ChangeCueSink` に 2 本目の送出端を持たせる形のどちらでもよい。台帳の担当は新しい `CommandConsumer` の腕か `ChangeSink` の共用か。どちらも作業量はほぼ同じなので設計で決める。
- メニューの子はフォルダ名で指す（手本の `GhostSpec::Folder` と同じ）。名前の重なりを避けられる。
- 隠しシェルは、`list_shells` を変えずに隣へ「隠しも含む列挙」の関数を足せば、`updateother` の振る舞いを変えずに済む。
- 更新の実行中の問いは、`update/desk.rs` に `pub(crate) fn` を 1 つ足せば済む（`stage != Idle` か `after_switch.is_some()`）。

## 7.1 要件ディスカッションでの扱い（2026-09-30）

- **要件で決めた（自明な修正）**
  - F → 要件 2.7: シェルの `descript.txt` の見た目の値（寸法・作者の DPI・バルーンのずらしと揃え方・`seriko.zorder`・デスクトップへの揃え方）は読み直す。窓の位置とスコープの数（`kero.*`）は保つ。§8 (i)。
  - I → 要件 6.1・6.7: シェルの切替は `LastShell` だけを、バルーンの切替は `LastBalloon` だけを書く（1 つだけを書く口を `LastUsed` の隣へ）。argv でバルーンを指定して起きたプロセスでも、切替の選択は書く。
  - J → 要件 2.2・3.3: 「バルーンの表示が終わった時点」は文字を出し終えた時点（`DisplayEndAt`）。バルーンの差し替えでは、残っている古いバルーンをその場で隠す。§8 (j)。
  - E の前半 → 要件 3.5: バルーンの切替では、バルーン自身のアニメの定義（`balloon_tables`）を新しいバルーンのものへ替える。「不変」はシェル側に限る。
- **設計へ持ち越す**: A（今のシェルの置き場）・E の後半（seriko への送出端の置き場）・G（定常でないことの判定の近似）・H（資産を作る場所）と、§6 の調べもの 1〜8。
- **開発者と決める**: B と C（kanade に手を入れる範囲）・D（ゴースト切替とぶつかったとき）。結果は要件ディスカッションの各コミットと要件 12 に残す。

## 8. 次の手順

- 上の議題 A〜J のうち、要件の改訂を伴うもの（C・D・I、場合により B・G・J）を要件ディスカッションで決める。
- 設計は `/kiro-spec-design areka-P0-shell-balloon-switch`（または `/kiro-design`）。設計では §6 の調べものの 1〜4 を最初に解く。

---

# 設計の段（2026-09-30・`/kiro-spec-design -y`）

## Summary

- **Feature**: `areka-P0-shell-balloon-switch`
- **Discovery Scope**: Complex Integration（既存の拡張だが、kanade・seriko・present・placement・ghost・parsers の 6 層にまたがる）。発見は light〜full の中間で、外部の調べもの（Web）は無し（新しい依存が 0 のため）。コードの実測で行った。
- **Key Findings**:
  - 印の無い経路（`raise-event` 無し・バルーン）でも、要件 5.4・1.14・5.7 は「kanade が終了系列へ入ったか」を知らないと判定できない。既存の返信（`RaiseOutcome`）と通知（`KanadeNotice`）では足りない（§9.1）。
  - 古い定義の表示の指令と新しい装着の食い違いは、seriko の inbox の FIFO に合図を乗せれば、世代番号も読み捨ても無しで消せる（§9.2）。
  - `Emo2BootInputs` と `StartupDescriptValues` の構造体リテラルが、触れてはならない `ghost_switch_test_support.rs` に在る。シェル名はこの 2 つの欄では運べない（§9.5）。

## 9. 設計判断（Design Decisions）

### 9.1 kanade の口は「台詞の切れ目を待つ」1 本（議題 B・C・G の設計側・§6 の 4）

- **Context**: 要件 8.11 は `OnShellChanging` の印の口を求め、要件 8.2 は kanade の変更をそれに限る。しかし印の無い経路にも、次の判定が要る。
  - 要件 5.4: 台本を運んだ台詞が `\-` の予約つきで中断されたら、切替を捨てる。
  - 要件 1.14: 定常でなければ無視する。
  - 要件 5.7: 終了要求が勝つ。
- **確かめた事実**:
  - `\-` の予約は台本の翻訳時に決まる（`crates/areka-sakura/src/drive.rs` の `on_close` の doc）。cue として流れないので、UI の受け口からは見えない。
  - 終了系列へ入ったことを UI へ知らせる通知は無い。`KanadeNotice` は `Steady`（起動の完了）・`ChangeCancelled`・`Stopped` だけ（`crates/areka-kanade/src/change.rs` の `KanadeNotice` の定義）。
  - `KanadeNotice` に変種を足すと、`ghost_switch.rs` の `on_notice` の網羅の match が壊れる（要件 8.1 で触れない）。
  - 完了 `ghost-shell-balloon-switch` の同じ場面（`raise-event` 無しのゴースト切替）は、kanade の `pending_change` と `quit_dropping_pending`（`schedule/change.rs`）が扱っている。UI は推し量っていない。
  - `RaiseOutcome::NotSteady` は依頼の時点の相しか言わない（`actor.rs` の `spawn_kanade_with_stop_sink` の返信の組み立て）。
- **Alternatives**:
  1. UI が推し量る（自分の `UserBreak`・`DisplayEndAt`・差し替え後の `OnShellChanged` の `NotSteady`）。`\-` 入りの中断や終了の握手の途中でも差し替えてしまい、要件 5.4・1.14 を満たせない。裁定 8 が同じ理由で退けた形。
  2. 印の口を `OnShellChanging` にだけ付け、印の無い経路は 1 のとおり。利用者から見ると、`\-` で終わるゴーストが最後の一瞬だけ新しいシェルになり、次の起動でそのシェルが記憶されうる。
  3. **採用**: 口を `KanadeMsg::AwaitTalkGap { raise: Option<GapRaise>, reply }` の 1 本にし、印は任意の添え物にする（本仕様で付けるのは `OnShellChanging` だけ）。
- **Selected Approach**: 判断は `schedule/talk_gap.rs`（純粋）に置く。`step` の後の見極め `observe` と、`on_talk_done` の `break_quit` への 1 項（`is_marked_break`）の 2 か所で mod.rs に繋ぐ。殻は結果を 1 回送るだけ。
- **Rationale**: 裁定 8 の根拠がそのまま印の無い経路にも当てはまり、開発者の指示「目的の実現を優先せよ」に沿う。`steady.rs` と `RaiseEvent` は 1 行も変えない（要件 8.3）。
- **Trade-offs**: 要件 8.2・8.11 の字面（kanade の範囲と「知らせる時点」）を超える。結果は「印の台詞が終わったとき」ではなく「その後の切れ目」で 1 回返る。印の台詞の終わり方は `Reached{marked}` に載るので、8.11 ⑴ の情報は失われない。
- **Follow-up**: 設計討議で要件 8.2・8.11 の本文の追随を確かめる（§12）。

### 9.2 差し替えの順序は seriko の表示の流れで決める（§6 の 1・2）

- **Alternatives**:
  1. `PresentCommand` に世代番号を付ける。`ShowSurface` の構築点すべてに波及する。
  2. seriko の返事を待ってから装着する。返事と表示の流れは別の線なので、順序が決まらない。
  3. 差し替えの tick で `wiring.rx` を読み捨てる。seriko が `Replace` を処理する前に出した古い指令が、後から届く。
  4. **採用**: seriko が定義を替えた点で `DisplayCommand::Rebased{epoch}` を出す。areka の `PresentBridge` がそれを、UI が先に置いた荷物（`SwapSlot`）と突き合わせて `PresentCommand::ReplaceTarget` に写し、同じ present の線へ流す。
- **Rationale**: seriko の inbox は FIFO なので、`Rebased` より前の指令は古い装着に、後の指令は新しい装着に必ず当たる。最初の面は seriko の `ScopeStates` の今の面から出す。表示の流れの権威は seriko で、UI の `current_surface_id` を使う案（§6 の 2）はアニメの途中の状態を二重に持つので採らない。
- **Trade-offs**: `PresentBridge` に共有の置き場（`Arc<Mutex<Option<SwapPayload>>>`）が 1 つ増える。鍵を持つのは置く・取り出すの一瞬だけ。`handle_message` の署名はテストの呼び出し 45 本のために変えず、`Replace` は受信の閉包で先に捌く。

### 9.3 資産は背景のスレッドで作る（議題 H・§6 の 3）

- 受理ごとに `std::thread` を 1 本起こし、COM を MTA で初期化して WIC で復号する。本番の UI スレッドも MTA で、同じ `WicDecoderArm` を使う。
- `EmoWorld` は bevy の `World` を 1 欄に持つだけ（`areka-emo-compose/src/world.rs`）で、`AtlasTable` は `Arc` と `HashMap` だけ（`areka-emo-atlas/src/table.rs`）。どちらも送れる。コンパイル時の確かめをテストに置く。
- 位置の記憶（`persist::load_restored_state`）と配置の値（`load_descript_source_for_shell`）も背景で読み、UI スレッドではファイルを読まない。

### 9.4 今のシェルの置き場は `GhostRuntime` に書く口（議題 A）

- `GhostRuntime::set_shell_dir` で `mount.shell.dir` を書き換える。`ShellMount.dir` は `pub` 欄。
- `mount().shell.dir` の本番の読み手 3 つ（`update/desk.rs` の `here`・`ghost_switch.rs` の `record_steady_memory`・`main.rs` の `on_boot_ok`）は手当て 0 で差し替え後を指す。
- 新しい資源を置く案（議題 A (b)）は、`here` の書き換えと 2 か所の真実源を生むので採らない。
- sylphya の Shell スコープの根は起動時のシェルのまま。本番で `PersistScope::Shell` へ書く呼び手が 0 なので張り替えない。Revalidation Triggers に登記した。

### 9.5 シェル名は「配置の準備が決めた資源」と `wire_emo2_boot` の引数で運ぶ

- 要件 6.4 は運び手を `Emo2BootInputs` の欄と名指しする。しかし `Emo2BootInputs` と `StartupDescriptValues` の完全な構造体リテラルが `ghost_switch_test_support.rs` に在る（それぞれ `GhostBootInputsSource` の閉包と切替の土台の中）。欄を足すと、触れてはならないファイルの書き換えをコンパイルが要求する（要件 8.1）。
- 採用した形:
  - `ghost_session::prepare_ghost_windows` が `decide_boot_shell` を 1 度呼び、配置の準備へ渡す。同時に資源 `BootShellChoice { ghost_root, shell }` に置く。
  - `boot_wired` がそれを取り出して `wire_emo2_boot(.., shell)` へ渡す。
  - `prepare` を経ない起動（テスト）では `boot_wired` が自分で決める。
  - 決定は 1 回・`warn!` も 1 回。
- 要件 6.4 の本旨（欄を足さない 5 つの型・3 か所が同じシェル）は満たす。要件の本文の追随は §12。

### 9.6 記憶は 1 つずつ書く（議題 I の設計側）

- `record_last_shell`／`record_last_balloon` を `LastUsed` の隣に置く（投函だけ・`info!`）。
- 更新の読み直しでは、`GhostSession::shutdown` が記憶の書き手を処理し切ってから、起動前の解決が記憶を直読みする。書いた値は読める（§6 の 5）。

### 9.7 シェルの差し替えで読み直す配置の値（要件 2.7・議題 F の設計側）

- 起動時と同じ純関数を同じ順で通す: `build_placement_config` → `resolve_placement` → `apply_restored_placements`。そこから `anchor`・`balloon_offset_base`・`balloon_keyword_base` だけを取る（`char_pos` は捨てる＝窓の位置を保つ）。
- 利用者のドラッグの記憶は、起動時と同じ規則で重なる。scope の集合は起動時の `GhostWindows` のまま。

### 9.8 重なりの基底の置き直しでタグ由来のグループも落ちる

- 台帳 `ZOrderGroupLedger::set_descript_base` は、基底を置くとタグ由来のグループも落とす（同関数の doc「終状態は `reset_to_descript` と一致させる」）。
- 基底だけを入れ替える口を新設すると、台帳の不変条件（基底は先頭・高々 1 つ・再指定の拒否）を二重に持つことになる。そこで既存の関数を使い、シェルの差し替えの直後は「新しいシェルで起きた直後」と同じ重なりの状態にする。
- 正典は沈黙している。台本が `\![set,zorder,…]` を掛け直したければ、`OnShellChanged` の台本で掛け直せる。

### 9.9 下位の判断（要件と裁定で答えが決まるもの・議題にしない）

- **選択肢の扱い**: 選択肢の選択の応答（カスケード）が印の台詞を置き換えたら `Replaced`（要件 5.2「置き換えは台詞の終わり」）。時間切れの解除は利用者の中断ではないので `Completed`。
- **印の台詞が自ら `\-` に達したとき**: 今日どおり終了系列へ進み、切替は捨てる。要件 5.2 が例外にするのは中断だけ。台本が自ら終了を求めた場合は、要件 5.7 と同じ向き（終了の意思が勝つ）。
- **新しいシェルに今の面が無いとき**: 要件 2.6・2.7 の「今日の『無い面を指定された』ときの扱い」に従い、`error!` を残してその scope は表示なし。次の `\s` で出る。
  - 今日の扱いは「前の面を出し続ける」だが、前の面は古いシェルのもので持ち越せない。
  - 要件 4.2 の「キャラ窓の絵が空になる」は差し替えの途中のフレームの崩れを指す。面が無いという定常の状態には、個別の規則（2.6）が先に効くと読む。
- **定常でない時点で届いた要求**: `warn!`（要件 1.14）。待っている間に終了・ゴースト切替で取りやめになったものは `info!`（要件 1.12・5.7）。

## 10. 危険と対策

1. **切れ目と差し替えの間に新しいトークが始まる**: `Reached` を受けてから seriko の `Rebased` が drain に届くまでは、1〜2 フレーム。この間に `OnSecondChange` 等が始まると、差し替えはそのトークの最初の方と重なる。
   - シェルなら面が新しいシェルで出るだけ。
   - バルーンなら、文字の層は `refresh_actor_scale` が表示の進み具合を保って新しいスロットへ結び直す。可視性の制御は、新しい装着に可視コンテンツが置かれた時点で今日どおり見せる。崩れ（重なり・空・素通し）は出ない。
   - 実機サインオフで目視する。
2. **`Rebased` に荷物が無い**（起きない想定）: UI は荷物を置いてから `Replace` を送り、置き場は 1 つで UI だけが置く。起きたときは `error!` と切替の失敗とし、seriko の定義だけが新しくなる。構造上の防御として記録だけ残す。
3. **拡大率 200% は未観測**（先進坑は 100% だけ）: 新しい target の `policy` は新しい作者の DPI、k は窓の `DPI` から毎回導く。仕組みは今日の表示と同じ。実機 ⑥ で確かめる。
4. **`\-` で自ら終わる印の台詞**: §9.9 のとおり終了を優先する。
5. **無い面**: §9.9。
6. **記憶の反映**: §9.6。
7. **`build_boot_assets` の括り出し**: 振る舞いは不変。既存の `assets_tests.rs`（976 行・足さない）が緑のままであることで確かめる。

## 11. 簡素化（Synthesis）

- **一般化**:
  - 台詞の切れ目の口は、`raise` の有無だけで印の有無を表す 1 本にした（印の経路と印の無い経路を別の口にしない）。α 後の `network-update-canon-order` の「台詞が終わってから次へ」にも同じ口が使える見込み（実装は本仕様の範囲だけ）。
  - `PresentCommand::ReplaceTarget` は、シェルとバルーンで同じ 1 つの語。
- **作るか借りるか**: 依存は足さない。
  - 背景の復号は `std::thread`＋既存の WIC。
  - 名前の照合は完了 spec の規則を写す（`resolve_switch_target` は `ghost_switch.rs` の私有の関数で呼べない）。
  - 乱数は `pick_index`、配置は起動時の純関数、重なりは台帳の既存の関数を使う。
- **削ったもの**:
  - `PresentCommand` の世代番号・seriko の返事の線・UI 側の台詞の終わりの推し量り・`here` の書き換え・`Emo2BootInputs` の欄。
  - Shell スコープの張り替え・タグ由来の重なりの保存・切替の告知。
  - `detach_target` は本番の呼び手が `apply_replace` だけ（要件 4.6 が口として求めるので置く。登録を消したまま戻さない経路は作らない）。

## 12. 要件の本文の追随が要る点（設計討議で確かめる）

利用者から見える振る舞いは要件どおり。変わるのは要件が名指しした部品だけである。

1. **要件 8.2・8.11**:
   - kanade に足す口が「`OnShellChanging` の印」だけでなく「台詞の切れ目を待つ口（印は任意）」になる。
   - 結果を返す時点は、印の台詞の終わりではなく「その後の切れ目」になる（印の台詞の終わり方は結果に載る）。
   - 理由は §9.1。追随しない場合（口を印だけに限る）は、印の無い経路で要件 5.4・1.14・5.7 を近似でしか満たせない。利用者から見える差は「`\-` で終わるゴーストの台本が最後の一瞬だけ新しいシェル／バルーンを見せ、次の起動でそれを記憶しうる」。
2. **要件 6.4**:
   - 運び手が「`Emo2BootInputs` の欄と `boot_with_origin` の引数」ではなく、「配置の準備が決めた資源 `BootShellChoice`・`wire_emo2_boot` の引数・`boot_with_origin` の引数」になる。
   - 理由は §9.5（要件 8.1 と両立させるため）。
