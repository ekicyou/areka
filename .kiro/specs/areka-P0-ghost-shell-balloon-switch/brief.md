# Brief: areka-P0-ghost-shell-balloon-switch

> 2026-09-18 `/kiro-discovery` 再入（棚卸⑭＝α ゴールへの組み直し）で起票。`doc/ukadoc-coverage/roadmap-draft.md` 段階 B 順位 2 の束「切替」（候補名 `areka-P0-shell-balloon-switch`）を、ゴースト切替まで含めて引き受ける。
> 本文の file:line は**起票時の実測値**（2026-09-18）。着手時に必ず引き直すこと。

## 2026-09-20 棚卸⑮の再測定

**本仕様は 3 本に分かれた。** 想定タスクが 30〜40 本で 1 spec の上限（20 本）を大きく超えたためである。本 brief は名前を変えずに**真ん中の 1 本**として残す——完了 spec 7 本とソースのコメント（`crates/areka/src/menu/mod.rs`）と網羅台帳（`doc/ukadoc-coverage/ledger/shiori.toml`）がこの名前を引受先として指しており、完了 spec は書き換えられないからである。

| 段 | spec | 中身 | 本 brief の対応箇所 |
|---|---|---|---|
| 1 | `areka-P0-app-lifetime-separation`（台帳 #49） | アプリの寿命を窓の数から切り離す | Approach ①。**本 brief からは外れた** |
| 2 | **本仕様**（台帳 #13） | 切替要求の入口の型 `SwitchRequest`・台本の `change` の消費者・名前解決（`random`／`sequential`／`lastinstalled`）・**ゴースト切替**（`OnGhostChanging`／`OnGhostChanged`・降ろして起こし直す仕組み）・メニューの「ゴースト」枠への登記 | Approach ④・Desired Outcome 1・4・5・6 |
| 3 | `areka-P0-shell-balloon-switch`（台帳 #50） | シェル切替・バルーン切替・メニューの「シェル」「バルーン」枠への登記・起動時の「最後のシェル」の適用 | Approach ②③・Desired Outcome 2・3。**本 brief からは外れた** |

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
