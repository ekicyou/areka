# ギャップ分析: areka-P0-mcp-reload

> 2026-10-18・本ブランチ（main `e2370df8` の上）で測った。コードは「何の定義か」（関数名・型名＋ファイル）で指す。行数は `wc -l` の実測。
> 本書は分析と選択肢を並べるもので、決めるのは要件ディスカッション・設計の段。SSP には一切触れていない（`reload` を打っていない）。SSP で測るべき点は 7 節の「調べること」に挙げた。

## 1. 分析の要約

- **ゴーストの読み直しは既存の道にほぼ乗る**。`crates/areka/src/update/desk.rs` の `fn reload` が使う「同じフォルダ・`raise_event: false`」の `request_ghost_switch` は、kanade の `begin_change`（`schedule/change.rs`）で `OnGhostChanging`・`OnClose` を送らずに降ろし、`take_down` が SHIORI を unload、`boot_into` がディスクから全部（辞書・descript・シェル・バルーン）を読み直す。足りないのは起こす側の「知らせを 1 つも送らない由来」だけで、kanade で触るのは `src/change.rs`（変種 1 つ）と `schedule/boot.rs`（`on_prefetch_reply` と `boot_root` の 2 か所）で閉じる（`BootOrigin` を網羅する `match` は `boot_root` の 1 か所と `matches!` 1 か所だけ）。
- **シェル・バルーンの読み直しも既存の道に乗るが、分かれ目が 1 つ要る**。`request_skin_switch` は今と同じフォルダを指しても受理し、背景で組み直す。ところが差し替えの相 `frame/switch.rs` の `finish_shell`／`finish_balloon` は**無条件に** `raise_changed`（`OnShellChanged`／`OnBalloonChange`）を送り、記憶（`LastShell`／`LastBalloon`）を書き、バルーンでは選び方を `BalloonRoute::Memory` に書き換える。読み直しにはこれらを止める分かれ目が要る（要件 5.3・6.1・6.3）。
- **MCP の返事を遅らせる仕組みは既にある**。`crates/areka/src/mcp/mod.rs` の `later`（`pub(crate)`）に「覗く関数」を預ければ、`mod.rs` も `crates/areka-mcp` も変えずに「読み直しが終わってから答える」が書ける（`get_status.rs`・`dump_surface.rs` が同じ使い方）。10 秒を超えたら返事の端が `is_abandoned` になり置き場から捨てられるだけで、読み直しは続く（要件 8.5 は読み直し側で記録すれば満たせる）。
- **重い・不確かなのは 3 つ**: ⑴ `descript` の読み直し（動作中の値の持ち主が散らばっていて、ゴーストの名前は `GhostRuntime` のマウントの中＝`runtime.rs`・`ghost_session.rs` に触らないと替えられない。シェルの `name` は kanade の `KanadeConfig.shell_name` に入っていて `msg.rs` に触らずには替えられない）、⑵ `shiori` だけの読み直し（要件 4・窓を残した実行系の組み直しは新しい道）、⑶ argv で起こしたゴースト・根の外のバルーンでは既存の切替の入口（目録で引く）が使えないこと。
- **規模**: 要件 4 を除くと 14〜17 タスク、含めると 18〜22（窓を残した実行系の組み直しで +4〜6）。要件 4 を除いても、pasta のデバッガが台本で `\![reload,shiori]` を出す（pasta docs `debug/dev-actions`）ので、`shiori` を「ゴーストの読み直しの別名」として受ける案（+0〜1）が要件ディスカッションの議題になる。

## 2. 今あるもの（要件ごとの資産の地図）

凡例: ✅ 既存で足りる／🔧 既存を少し直す／❌ 無い（作る）／❓ 調べる／⚠ 制約

| 要件 | 使える既存の資産 | 状態 | ギャップ |
|---|---|---|---|
| 1.1〜1.3 MCP の振り分け | `mcp/mod.rs` の `dispatch`（`Reload` を `Omitted::UseActive` で解決済み）・`mcp/reload.rs` の `handle`（ダミー） | 🔧 | `target` の 6 語の照合（ASCII 大小無視・空白を削らない）は新しい純粋関数。`Kind::from_log_type`（`log_history.rs`）が同じ照らし方の先例 |
| 1.4・1.5 台本の受け口 | `change_cue.rs` の `ChangeCueSink`（自己選別の型）・`switch_cue.rs` | ❌ | `reload` と `reloadsurface` の受け口（新しい `CueSink`）。`emo2_boot/mod.rs` の `sinks` の vec に 12 本目を足し、受信端を据え、取り出しの系を登録する |
| 1.6 始めの `info!` | — | ❌ | 新しい入口で 1 件 |
| 2.1 重なり | `SwitchInFlight`（ゴースト切替）・`SkinSwitchInFlight`（シェル・バルーン）・`update::desk::is_busy` | 🔧 | ゴースト切替の入口 `request_ghost_switch_with` はシェル・バルーンの切替中・更新中を見ない＝読み直しの入口で 4 つとも見てから既存の入口を呼ぶ。「別の読み直し」はゴースト系なら `SwitchInFlight`、シェル系なら `SkinSwitchInFlight` が代わりに立つが、descript の読み直しを同期で行うなら自前の印は要らず、背景で組むなら要る |
| 2.2 終了の途中 | `exit_wait::WorkGate::is_closing`・`exit_wait::register_gate`（`shell_balloon_switch.rs` の `register_switch_drain` が先例）・`mcp::close`（置き場を外す＝"shutting down"） | 🔧 | 読み直しの窓口が門を 1 つ登記すれば判定できる |
| 2.4 ghost／shiori はトークの後 | kanade `on_change_ghost` の保留（`change_pending`） | ✅ | 既存の切替の入口を通せば自動で満たす |
| 2.5 shell／balloon／descript は台詞の切れ目 | `KanadeMsg::AwaitTalkGap`（既存の変種・使うだけなら `msg.rs` は変えない） | ✅ | descript も同じ口で待てる |
| 3.1 同じフォルダで降ろして起こす | `request_ghost_switch` → `switch_to` → `take_down` → `close_windows_for_restart` → `boot_into` | ✅/⚠ | argv で起こしたゴーストは目録に無く `NotFound`（`update/desk.rs` の `reload_folder` も `"argv"` で断る）＝7 節 |
| 3.2 `OnGhostChanging`・`OnClose` を送らない | `ChangeRequest.raise_event = false` → `begin_change` が黙って降ろす | ✅ | — |
| 3.3 起動の知らせ 0 | `BootOrigin`（4 種）・`boot_root`・`on_prefetch_reply` | ❌ | 変種を 1 つ足す。根が無いとき今は `OnBoot` へ直行する＝「根なし」では足りず、`on_prefetch_reply` で `to_baseware_version(state, None, config)` へ直行させる腕が要る。`first_boot` が最優先で `OnFirstBoot` を選ぶので、新しい由来はその判定より前に見る |
| 3.4 情報の通知は送る | boot 系列の `OnInitialize` NOTIFY・`username` の照会・`basewareversion` NOTIFY | ✅ | 上の直行の腕でも 3 つとも通る。初回起動の記録の epilogue（`first_boot_epilogue`）を起動するかは議題 |
| 3.5 窓の位置 | `close_windows_for_restart`＋`reopen_ghost_windows`（切替と同じ） | ✅❓ | ネットワーク更新の読み直しと同じ道なので同じ見え方のはず。実機で確かめる |
| 3.6 既存の知らせを保つ | `switch_to` の `boot_event` の分岐（`Updated`／`ChangedFrom`） | ✅ | 既存の由来の腕に触らない形で足せる（`boot_root_tests.rs` 等は変えない） |
| 3.7 起きなければ既定へ | `switch_to_default`・`on_ghost_stopped` の `ghost_switch_target_fault` | ✅ | — |
| 4 shiori だけ | `spawn_shiori_actor`（接続は 1 回きり）・`GhostSession::shutdown`・`close_windows_for_restart`／`reopen_ghost_windows`／`commit_ghost_windows` | ❌⚠ | 窓を残して実行系だけ組み直す道は無い。kanade に「繋ぎ直し」を足すのは `msg.rs`・`actor.rs`（約束で禁止） |
| 5 シェル | `request_skin_switch`（同じフォルダでも受理）・`spawn_switch_build`（毎回ディスクから）・`frame/switch.rs` | 🔧 | `OnShellChanged` を送らない分かれ目。`LastShell` を書くか（同じ値なので害は無いが「読み直しは選ばない」）は議題 |
| 6 バルーン | 同上・`finish_balloon` | 🔧⚠ | `OnBalloonChange` を送らない分かれ目・`BalloonRoute::Memory` へ書き換えない分かれ目。今のバルーンが根の `balloon/` の外（argv・ゴーストの descript の `default.balloon.path`・同梱）だと `balloon_candidates(&roots)` に無く `NotFound` になる＝読み直しは候補の目録で引かず `BootContext.current.balloon.dir` をそのまま使う形が要る |
| 7 descript | 下の 4 節の一覧 | ❌⚠ | 一番大きい穴（4 節） |
| 8 MCP の答え | `mcp::later`・`ReplyTo::is_abandoned` | 🔧 | 読み直しの終わり（成功・失敗）を知る口が要る（5 節 E） |
| 9 エラーログ | `log_history.rs` の `classify`（`warn` 以上→error 種別）・`TARGET_ERROR`＋`ghost` 欄（先例 `readme/opener.rs`） | 🔧 | 既存の失敗の記録（`ghost_switch_boot_failed`・`skin_switch_failed` など）はどれも `[SYSTEM]`。既存の行を書き換えると既存の振る舞いが変わるので、読み直しの側で「まとめの 1 件」を `TARGET_ERROR`＋`ghost` で残す形が素直 |
| 10 makoto | — | ❌ | `reload.rs` の 1 分岐 |
| 11 台帳 | `ConsumerLedger::canonical`・`consumer_ledger_agreement_tests.rs`（355 行） | 🔧 | `("reload", Some(各語))` 5 行＋`("reloadsurface", None)`。選別子つきで登記すれば後から `("reload", Some("makoto"))` を足せる（`SelectorConflict` は「同じ名前で選別子の有無が混ざる」ときだけ）＝11.2 は形の上で満たす |
| 12 テスト | `boot_root_tests.rs`・`ghost_switch_*_tests.rs`・`frame/switch_*tests.rs`・`mcp/reload_tests.rs` | 🔧 | `ghost_switch_tests.rs` は 990 行＝足すなら新しい兄弟 |
| 13 範囲 | — | ⚠ | 3 節の食い違い |

## 3. 制約と、要件・roadmap との食い違い（議題の種）

1. **行数**（1,000 行の目安）: `consumer_ledger.rs` 960（ファイルの中のテストが 441 行目〜）・`ghost_switch.rs` 948・`emo2_boot/mod.rs` 916・`ghost_switch_tests.rs` 990・`frame/switch_finish_tests.rs` 818・`frame/switch_tests.rs` 752・`ghost_session.rs` 885・`crates/areka-ghost/src/runtime.rs` 793・`frame/switch.rs` 636・`shell_balloon_switch.rs` 442・kanade `change.rs` 175・`schedule/boot.rs` 345。
   - `consumer_ledger.rs`: テストの塊を `consumer_ledger_tests.rs` へ出せば本体は約 440 行。
   - `ghost_switch.rs`: 入口は新しい兄弟に置いても、`switch_to` の由来の分岐（数行）と定常到達・中止での終わりの通知（`update::desk::on_switch_end` と同じ所に 1 行ずつ）は本体に要る＝約 955〜960 行。余裕が 40 行しかないので、先に `resolve_special_name`・`NameResolution`・`UnresolvedReason`（約 100 行）を兄弟へ出す選択肢がある。
   - `emo2_boot/mod.rs`: 受け口を足すと「`mod` 宣言と系の登録」だけでは済まない（受け口を作る・`sinks` に足す・受信端を据える＝約 8〜15 行と注釈）。要件 13.4 の書き方より少し広い。
2. **要件 13.1 に無いファイルが要る見込み**:
   - descript の読み直しでゴーストの名前を替えるには `GhostRuntime` のマウントの `names`（`runtime.rs`）と、`GhostSession::names()`（`ghost_session.rs`）の側に差し替えの口が要る（`set_shell_dir` と同じ形）。13.2 は要件 4 のときだけ 2 ファイルを足すが、要件 7.3 を満たすには要件 4 と無関係に要る。
   - 13.5（`bind` の定数）は seriko の `crates/areka-seriko/src/actor.rs` に触る。13.1 の一覧に seriko は無い（C9 で seriko に触る spec は見当たらない＝衝突は無いが、一覧に足すか次の spec へ送るかを決める）。
   - `frame/switch.rs` の兄弟のテスト（`switch_tests.rs`・`switch_finish_tests.rs`）は、`SkinSwitchInFlight` に欄を足す形を取れば書き換えが要る（別の印で持つ形なら要らない＝5 節 C）。
3. **roadmap と要件の食い違い**: roadmap の C9 ④ の行は触るファイルに `crates/areka/src/update/desk.rs` と `doc/ssp-mcp/areka-tools.md` を挙げるが、要件 13.1／13.3 は前者を挙げず後者を「変えない」とする。`update/desk.rs` は `SwitchRequest` の形を変えなければ触らずに済む（5 節 A）。roadmap の行を要件に合わせて直すかは完了時の片付け。
4. **`SwitchRequest`・`SwitchInFlight` は構造体リテラルで多数の所から組まれる**（`SwitchRequest {` は本番 4 か所＋テスト 10 か所、`SwitchInFlight {` はテスト 11 か所）。欄を足すと `install/overwrite.rs`・`menu/ghost_frame.rs`・`update/desk.rs` と多数のテストの書き換えになり、13.1 の外へ出る。
5. **シェルの `name` は kanade の中**: `KanadeConfig.shell_name`（`OnBoot` の Ref0 などの材料）は起動時に詰めたきりで、替えるには kanade へ新しい指示が要る（`msg.rs`＝約束で禁止）。既存のシェルの切替も同じく古いままなので、本 spec 固有の穴ではないが、要件 7.3 の「一覧の値は 1 つ残らず替える」とは衝突する。
6. **要件 2.1 の重なりの一覧にインストールが無い**: インストールの窓口（`install/desk.rs`）の実行中に読み直しが来た場合の扱いは要件に無い。ゴースト切替の入口も今はインストールを見ていない。
7. **既存の小さな穴（範囲外・記録だけ）**: `update/desk.rs` の `reload` は後送りの列が空だと `boot_event: None` になり、`ChangedFrom` で起こす＝`OnGhostChanged` を送る。ふだんは列に `OnUpdateComplete` が入るので表に出ない。

## 4. descript の読み直しで替える値（設計で確定する一覧の下書き）

起動時に descript.txt から読んで、動作中に持ち続けている値の持ち主（`grep` で見つけたもの・漏れがありうる）:

| descript | 値 | 持ち主（動作中） | 替える口 |
|---|---|---|---|
| ゴースト | `name`・`sakura.name`・`kero.name` ほか | `GhostRuntime` のマウントの `names`（`GhostSession::names()` → MCP の `resolve`・切替の `PrevGhost`・`install/desk.rs`・`update/desk.rs`・`readme/opener.rs`） | ❌ 差し替えの口が無い（`runtime.rs`・`ghost_session.rs`） |
| ゴースト | 同上の静的プロパティ（`selfname` など） | sylphya の静的構成（`publish_ghost_statics`・`pub`） | 🔧 呼び直せばよい（areka-ghost の公開関数） |
| ゴースト | `char{n}.name` ほか窓のタイトル | `placement::source::GhostTitles` | ❓ 窓のタイトルを書き直す口 |
| ゴースト＋シェル | 配置の値（`seriko.alignmenttodesktop`・`*.defaultx`・`seriko.zorder`・バルーンの offset など） | `DescriptSource`（`ghost_kv`・`shell_kv`）→ `apply_shell_descript`・`DefaultPlacementBasis`・重なりの基底 | ✅ シェルの差し替えの後始末 `finish_shell` が既に入れ直す（`build_shell` が `load_descript_source_for_shell` でゴーストとシェルの両方を読む）＝「シェルの読み直し（イベントなし）」に乗せれば得られる |
| シェル | `name` | `KanadeConfig.shell_name` | ⚠ kanade の中（3 節 5） |
| シェル | `seriko.dpi` | 作者基準 DPI（資産） | ✅ シェルの組み直しで入れ直る見込み |
| バルーン | 文字の模型・色・`dpi`・`windowposition.*`・`wordwrappoint.*` ほか | `BalloonModel`（`wiring.balloon_models`）・背景色 | ✅ バルーンの差し替えの後始末 `finish_balloon` が入れ直す |
| ゴースト | `readme` | 起動時に 1 度だけ決める（`readme::wire_readme`） | ❓ |
| ゴースト | `homeurl` ほか | 更新の窓口が必要なときに読む（`catalog::homeurl`） | ✅ 読み直しは不要の見込み |
| ゴースト | `default.balloon.path`・`balloon` | 切替・起動でバルーンを選ぶとき | ✅ 次に選ぶときに読む（読み直しはバルーンを選び直さない＝裁定 5） |

選択肢:
- **D1 シェル・バルーンの読み直しの道に乗せる**: `descript`（全部）＝「シェルの読み直し＋バルーンの読み直し（どちらもイベントなし・記憶なし）＋ゴーストの名前と静的プロパティの差し替え」。画像も組み直すので重いが、配置の値の入れ直しは既存の後始末で漏れなく済む。`descript,shell` はシェルの読み直しと同じになる（画像も読み直す＝要件より広い）。
- **D2 値だけを読み直す新しい道**: descript.txt だけを読み、配置の値・バルーンの値・名前を直接入れ直す。軽いが、`apply_shell_descript`・`BalloonModel` の組み立てを画像抜きで呼べるように切り出す仕事が要り、入れ直しの漏れが出やすい。
- どちらも、ゴーストの名前の差し替えには `runtime.rs`・`ghost_session.rs` の口が要り、シェルの `name` は kanade に届かない（受け入れて記録に残すか、要件 7.3 の一覧から外すか）。

## 5. 実装の選択肢

### A. ゴーストの読み直し: 「知らせなし」をどこで持つか

- **A1 別の印（推奨候補）**: 新しい兄弟ファイル（例 `emo2_boot/reload.rs`）に読み直しの進行の印（NonSend）を置き、`switch_to` が由来を決める所で「印があれば `BootOrigin::<新しい変種>`」と 1 か所だけ読む。`SwitchRequest`・`SwitchInFlight` の形を変えない＝`update/desk.rs`・`install/overwrite.rs`・`menu/ghost_frame.rs`・十数本のテストに触らない。印の後始末を、定常到達・中止・停止・致命の各所で漏らさない工夫が要る（印の寿命を `SwitchInFlight` に合わせる）。
- **A2 `SwitchRequest.boot_event` の型を列挙に変える**（`ChangedFrom`／`Updated(..)`／`Silent`）: 意味は素直だが、構築点の書き換えが 13.1 の外へ広がる（3 節 4）。
- **A3 `GhostSpec` に「今のゴースト」を足す**: argv で起こしたゴーストも目録を引かずに今の `BootContext.current.ghost.dir` で指せる（7 節）。`resolve_switch_target` の `match` は `ghost_switch.rs` の中だけ。A1 と組み合わせられる。

### B. kanade の「知らせなし」の由来

- **B1 変種を 1 つ足す（例 `BootOrigin::Reloaded`）**: `change.rs` に 1 変種、`schedule/boot.rs` の `on_prefetch_reply` に「根も `OnBoot` も飛ばして `to_baseware_version` へ」の腕、`boot_root` の `match` に 1 腕、`first_boot` より前に見る。テストは `boot_root_tests.rs` に足すか新しい兄弟。既存の 4 種の腕は 1 行も変えない（要件 12.3）。
- **B2 kanade に触らず UI 側だけで済ませる**: 起動の根を選ぶのは kanade の中だけなので、UI 側では送られる GET を止められない＝成り立たない（`ShioriWiring` で包んで GET を握りつぶす案は、本物の SHIORI に届かない＝ゴーストの状態を壊す）。

### C. シェル・バルーンの読み直し: イベントと記憶を止める分かれ目

- **C1 別の印**: 読み直しの印（A1 と同じ NonSend か別の NonSend）を `finish_shell`／`finish_balloon` が見て、`raise_changed`・`record_memory`・`BalloonRoute::Memory` の書き換えを飛ばす。`SkinSwitchInFlight` の形は変えない＝兄弟のテストの構築点に触らない。
- **C2 `SkinSwitchInFlight` に欄（例 `reload: bool`）**: 意味は明快（印と進行の寿命が一致）だが、`frame/switch.rs` の分解・組み直し 3 か所と、テスト 3 ファイルの構築点を書き換える。
- バルーンは候補の目録を引かずに今のバルーンのフォルダを指す口が要る（`request_skin_switch_with` の中か、読み直し専用の入口から `SkinCandidate` を直接組む）。

### D. descript（4 節の D1／D2）

### E. MCP の返事を読み直しの終わりまで待つ

- **E1 終わりの通知を受ける受け手を渡す**: 読み直しの入口が `Receiver<結果>` を返し、`reload.rs` が `super::later(world, reply, move |_| rx.try_recv()…)` で覗く（`get_status.rs` と同じ形）。終わりの通知はゴースト系なら `ghost_switch.rs` の定常到達・中止（`update::desk::on_switch_end` を呼んでいる 2 か所）、シェル系なら `frame/switch.rs` の後始末の成功と失敗の所から送る。
- **E2 進行の印を覗く**: 覗く関数が毎フレーム「`SwitchInFlight`／`SkinSwitchInFlight` が消えたか」と「成功の印が立ったか」を見る。通知の線は要らないが、成功と失敗（既定へ戻った・中止）の見分けに成功側の印立てが結局要る。
- どちらも `mcp/mod.rs`・`crates/areka-mcp` は不変。10 秒を超えると返事の端が捨てられるだけなので、終わりの記録（成功の `info!`・失敗の `error!`）は返事の有無に依らず読み直し側で残す（要件 8.5）。
- 台本から始めた読み直しは返事を持たない（同じ入口で「返事の受け手なし」）。

### F. `\![reload,shiori]`（要件 4）

- **F1 外す・`shiori` はゴーストの読み直しの別名**: pasta のデバッガの「SHIORIリロード」はエンジンが台本で `\![reload,shiori]` を出す作り（pasta docs `debug/dev-actions`）。外して「拾わない」にすると、このデバッガの導線が areka で黙って動かない。別名にすれば辞書は読み直され、デバッグのポートは `DebugHandle` の Drop で放されてから新しい SHIORI が開き直す（pasta docs `internals/debug` の不変条件）。違いは窓が閉じて開くこととシェル・バルーンも読み直すこと。+0〜1 タスク。
- **F2 外す・拾わない／MCP は縮退の `NG:`**: 要件 4 の注記のとおり。pasta のデバッガの導線が切れる。
- **F3 含める・窓を残して実行系だけ組み直す**: `GhostSession::shutdown` の一部（実行系だけ）＋窓を残したまま新しい `GhostRuntime` を `wire_emo2_boot` に近い形で結線し直す。seriko・受け口・アトラスは窓と実行系の両方にまたがるので、どこまで作り直すかの切り分けが新しい設計になる。`runtime.rs`・`ghost_session.rs`・`emo2_boot/mod.rs` の結線に触る。+4〜6 タスク・リスク高。
- **F4 含める・kanade に繋ぎ直しの指示**: `msg.rs`・`actor.rs`＝約束で禁止。`mcp-kanade-tools` の着地の後の別 spec なら可。

### 推奨の方向（決定ではない）

ハイブリッド: 新しい兄弟ファイルに「読み直しの入口・進行の印・窓口（終了の門）」を置き（B 型）、既存の切替の入口と差し替えの相へは「由来の上書き」「イベントと記憶を飛ばす」「終わりの通知」の数行の口だけを開ける（A 型）。具体的には A1(+A3)・B1・C1・E1。要件 4 は F1 か F3 を要件ディスカッションで選ぶ。

## 6. 規模とリスク

- **規模: L**（要件 4 を除き 14〜17 タスク・F3 で含めると 18〜22）。タスクの見込み（要件 4 を除く）:
  1. `consumer_ledger.rs` のテストを兄弟へ出す（＋`bind` の定数を決めるなら）
  2. kanade の由来の変種と起動の腕（`change.rs`・`schedule/boot.rs`＋テスト）
  3. 純粋な振り分け（`target` の 6 語・台本の綴り・`descript` のパラメータの語）＋テスト
  4. 読み直しの入口と進行の印・重なりの判定・終了の門・始めの `info!`
  5. ゴーストの読み直し（由来の上書き・終わりの通知・argv の扱い）＋テスト
  6. シェルの読み直し（`frame/switch.rs` の分かれ目）＋テスト
  7. バルーンの読み直し（今のフォルダを直接指す・記憶と選び方を書き換えない）＋テスト
  8〜9. descript の読み直し（値の一覧の確定・名前の差し替えの口・語ごとの振り分けと記録）
  10. 台本の受け口・台帳の行・一致のテスト・`mod.rs` の結線
  11. MCP の `reload.rs`（返事の遅延・makoto の縮退・知らない語）＋`reload_tests.rs` の書き換え
  12. エラーログの取り決め（`TARGET_ERROR`＋`ghost`）のまとめの記録
  13. 網羅台帳 `sakura-script.toml` の行
  14. 既存のテストの緑の確かめ（切替・終了・更新）
  15. 実機の確かめ（`verification/`）
  （＋`ghost_switch.rs` を先に削る 1 タスク・F1 の別名 0〜1）
- **リスク: 中**。ゴースト・シェル・バルーンは既存の道の延長で、kanade の変更も 2 ファイルで閉じる。descript（値の持ち主が散らばる・kanade の中の値）と、要件 4 を F3 で含める場合は高。

## 7. 調べること（設計の段へ持ち越す）

### SSP で測ること（開発者の SSP には触れていない・測る手順は要件ディスカッションの前に決める）

1. `reload` の成功の本文（6 対象ごと）と、返事を返すのが読み直しの前か後か。
2. `reload` の `ghost`・`shiori`・`shell`・`balloon`・`descript` で送られる SHIORI イベント（`OnBoot`・`OnGhostChanged`・`OnShellChanged`・`OnBalloonChange` など）と NOTIFY（`basewareversion` のほか `installedghostname` 等）。
3. `shiori` の読み直しでシェルの窓が閉じるか・`ghost` の読み直しで窓の位置が保たれるか（棚卸㉗の問い）。
4. `target` の大小・前後の空白の扱い。
5. 台詞の再生中に `reload` が来たときに待つか（`ghost`・`shell`）。
6. `\![reload,descript,ghost,shell]`（カンマ区切り）・知らない語・空のパラメータの扱い。
7. `ghost` の読み直しでバルーンを選び直すか（argv や descript 指定のバルーンのとき）。

### コードで確かめること

8. pasta が壊れた辞書で load をどう返すか（load が偽→areka の起動失敗→既定へ戻る、なのか、load は成功して pasta 自身のログだけ、なのか）。後者なら要件 12.7 ⑶「`get_log` に失敗が載る」は areka が観測できる失敗が無く、満たし方を考え直す。
9. argv で起こしたゴースト（実機のテストは `argv[1] = ghost_root` で起こす・`crates/areka/tests/emo2_real_run.rs`）と、ぱすた開発の起こし方（argv か `AREKA_ROOT` か）。argv なら既存の切替の入口は使えない（A3 が要る）。argv のバルーン（`BalloonRoute::Argv`）は `boot_into` の `resolve_balloon_for_ghost` で選び直されてしまう。
10. 動く WebP の多いシェル（ぱすた: 48・40・40 コマ）でゴーストの読み直しが 10 秒の上限に収まるか（release の焼き込みは約 3.2 秒・debug は未測）。
11. 窓のタイトル・`readme` の行き先を動作中に替える口があるか。
12. `first_boot_epilogue`（初回起動の記録の SET）を読み直しの起動で流すか（流しても台詞は出ない・記録が書かれる）。

## 8. 要件ディスカッションへ渡す議題

1. **`\![reload,shiori]` の扱い**（F1 別名／F2 拾わない／F3 窓を残して作る）。pasta のデバッガが台本で `\![reload,shiori]` を出す点が新しい材料。
2. **descript の読み直しの作り方**（D1 シェル・バルーンの読み直しに乗せる／D2 値だけ）と、`descript,shell` が画像まで読み直してよいか。
3. **要件 7.3 の「一覧の値は 1 つ残らず替える」と kanade の中のシェルの `name`**（`KanadeConfig.shell_name`・`msg.rs` 禁止）。一覧から外して記録に残すか、後の spec へ送るか。
4. **要件 13.1 に足すファイル**: descript の名前の差し替えのための `runtime.rs`・`ghost_session.rs`（要件 4 と無関係に）・13.5 の seriko `actor.rs`・`frame/switch.rs` の兄弟のテスト・`emo2_boot/mod.rs` の受け口の結線（13.4 の「宣言と系の登録だけ」を広げる）。
5. **argv で起こしたゴースト・根の外のバルーンの読み直し**（A3 で今のフォルダを直接指すか、argv では断るか）。
6. **シェル・バルーンの読み直しで記憶（`LastShell`／`LastBalloon`）とバルーンの選び方を書き換えるか**（書かない案が「選び直さない」に沿う）。
7. **「知らせなし」の持ち方**（A1 別の印／A2 型を変える＝`update/desk.rs` ほかに波及）と、`ghost_switch.rs` を先に削るか。
8. **読み直しの起動で `first_boot` を無視するか・初回起動の記録の epilogue を流すか**。
9. **MCP の終わりの知らせ方**（E1 受け手を渡す／E2 印を覗く）と、中止（`ChangeCancelled`）や既定へ戻ったときの答え（`NG:Failed…` に寄せるか）。
10. **重なりの一覧にインストールの実行中を足すか**（要件 2.1）。
11. **壊れた辞書の失敗を areka が観測できるか**（調べること 8 の結果次第で要件 12.7 ⑶ の書き方を改める）。
12. **roadmap の C9 ④ の行の触るファイル**（`update/desk.rs`・`areka-tools.md`）を要件に合わせて直すか。
