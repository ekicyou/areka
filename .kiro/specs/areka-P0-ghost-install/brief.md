# Brief: areka-P0-ghost-install

> 2026-09-18 `/kiro-discovery` 再入（棚卸⑭＝α ゴールへの組み直し）で起票。`doc/ukadoc-coverage/roadmap-draft.md` 段階 B 順位 1 の束「インストール」の**製品側**（利用者が `.nar` を渡す体験）と、順位 4 の束「投げ込み」（`OnFileDrop2` 等）のうち窓へ落とす経路を引き受ける。エンジン（コンテナ読取・`install.txt` 解釈・安全な展開）は `areka-P0-nar-install` が持つ。
> 本文の file:line は**起票時の実測値**（2026-09-18）。着手時に必ず引き直すこと。

## 2026-09-28 棚卸⑲の再測定（main `10a8d724`）

前回（下の「2026-09-27 棚卸⑱の再測定」節・main `5a232d2f`）の後に main へ 2 本が入った: `ghost-change-name-resolution`（PR#194・`0f50921e`・受け皿の話は直下の節）と `session-mark-residue`（PR#195・`10a8d724`・完了 spec は `.kiro/specs/completed/areka-P0-session-mark-residue/`）。本節は両方の着地を実物で引き直した結果で、前節の数と「触る／触らない」を上書きする。

**変わった数と場所**

1. **行数**: `main.rs` 873 → **927**・`ghost_session.rs` 638 → **691**・`session_end.rs` 112 → **163**（以上 `session-mark-residue`）／`emo2_boot/ghost_switch.rs` 699 → **866**・`ghost_switch_tests.rs` 545 → **987**（`ghost-change-name-resolution`・上限目前は直下の節 3 のまま）／`emo2_boot/spine.rs` 975 → **993**（`session-mark-residue` が偽の SHIORI `ScriptedShioriBackend` に「解かれるまで固まる」台本を足した＝上限目前・本仕様はここに足さない）／`dist/README.txt` 102 → 103。
   **不変**: `alert.rs` 191・`emo2_boot/mod.rs` 805・`emo2_boot/consumer_ledger.rs` 756・`menu/mod.rs` 337・`input_events/mod.rs` 545・`placement/spawn.rs` 769（`ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW` の 1 行も同じ）・`areka-ghost/src/catalog.rs` 324・`crates/areka/Cargo.toml` 72・`app_exit.rs` 291・kanade `schedule/events.rs` 535（`ALLOWED_EVENT_IDS` は 13 件のまま）と `events_change_tests.rs`（`assert_eq!(ALLOWED_EVENT_IDS.len(), 13)` のまま）・kanade `schedule/mod.rs` 830・wintf `window_proc/mod.rs` 332（`dispatch_window_message` の表に `WM_DROPFILES` は 0 行）・wintf `window/components.rs` 348。`WM_DROPFILES`／`DragAcceptFiles`／`DragQueryFile`／`GetOpenFileName`／`IDropTarget` は `crates/` で今も 0 件（`crates/pilot/` を除く）。台帳 3 本の該当行（`descript_install` 11 行・`OnInstall*` など・`\![execute,install,path,…]`）・`roadmap-draft.md` の `[briefs].count = 36`・`crates/areka-nar/src/` の `// ukadoc:` 0 行も前節（と直下の節 4）のまま。
   なお `session-mark-residue` が触る見込みとされていた kanade `schedule/mod.rs`・`app_exit.rs`・host32 `client.rs` は実際には変わっていない（変わったのは kanade `lib.rs`・`shiori/{mod,probe,real}.rs`、host32 `lib.rs`・`lifecycle.rs`・`process_host.rs`・`terminator.rs`、`areka-ghost/src/runtime.rs`）。
2. **`ghost_switch.rs` は「触らない」から外れる。** 前節は「触らない: `ghost_switch.rs`（議題 ⑴ の答えが (a) でなければ）」としたが、受け皿の書く口 `record_last_installed` には `#[allow(dead_code)]` と「本番の呼び手は後続 areka-P0-ghost-install」の注釈が付いており、呼んだコミットで外す（直下の節 1）。したがって議題 ⑴ の答えによらず `ghost_switch.rs` に 2 行の削除が入る。テストは `ghost_switch_tests.rs`（987 行）に足さず本仕様の新しいテストファイルへ置けば、テストファイルの分割は要らない。議題 ⑴ を (a) で答えると `switch_to` に口を足すので本体 866 行とテスト 987 行の両方に手が入り、先にテストの分割が 1 手要る（(a) の手間は前節より増えた）。
3. **起動中の印の判定 `session_mark_verdict`（`main.rs`）の形が変わった。** 引数は `(first, argv_session, logsink_fallback, end: Teardown { run_ok, down_ok, shiori_cut })`。印を残す理由は時系列で最初のものを採り、順は `logsink_fallback` → `no_exit_origin` → 出所の失敗（SHIORI の失敗・既定へ戻せない致命）→ `run_failed` → `session_end_deadline`（OS の終了で SHIORI の待ちを上限で打ち切った）→ `down_failed`。本仕様との関係:
   - 議題 ⑴ を (b)／(c) で答えれば、本仕様は終了や起こし直しの経路を新設しない（(b) は既存の入口 `request_ghost_switch` を呼ぶだけ＝切替の経路の印の扱いは `session-mark-residue` 要件 4.8 で不変と固定済み）＝判定には触らない。
   - 最初の起動が LogSink へ倒れた回（`GhostSession::logsink_fallback()` が真）は、窓も台本の受け口（`emo2_boot/mod.rs` の `wire_emo2_boot` の中で組む）も無いので、本仕様の入口 3 つ（投げ込み・メニュー・台本）はどれも届かない＝ぶつからない。
   - 判定の署名を変えるのは `session-mark-residue` の Revalidation Trigger（呼び手 5 か所を直す）。下の議題 ⑷ の答えによらず、印の材料は足さずに済む（下の 5）。
4. **OS の終了の後始末に期限が入った**（`session_end.rs`）。`SESSION_END_SHIORI_LIMIT`（3 秒・`pub(crate) const`）を、`end_session_within` が後始末に入った時点 `started` から数え、`GhostSession::shutdown_within(reason, WaitBudget { started, limit })` で降ろす。見張るのは SHIORI の待ちだけ（`crates/areka-kanade/src/shiori/probe.rs` の `ShioriProbe`）。ふつうの `GhostSession::shutdown` は署名も振る舞いも変わっていないので、切替の経路（`ghost_switch.rs` の `take_down`）には期限は掛からない＝議題 ⑴ (b) の「既定へ切り替えてから入れる」も今日どおり。
5. **汎用の通知の入口を最初に使うのは本仕様になった。** `KanadeMsg::RaiseEvent` を送る本番コードは今も 0 件（kanade の `msg.rs`・`actor.rs`・`schedule/{mod,change}.rs` の受け手だけ）。前節の帰結 2 は「入口専用の許可表に分けるかは `shell-balloon-switch` が決める」としたが、ウェーブの順が `ghost-install` → `shell-balloon-switch` に替わったので**本仕様が決める**。勝者は明白＝分けない（共用の `ALLOWED_EVENT_IDS` に 10 行足し、`events_change_tests.rs` の数を 13 → 23 にする。表を分けると `events.rs` の判定と `schedule/change.rs` の `on_raise_event` に手が入る）。設計で結果だけ書く。

**install の途中でアプリが終わるとき（新しく見えた穴）**

6. 手続きは背景スレッドで走る（前節・09-24 節 10）。ところが `fn main` の後始末（`after_run` → 降ろす → `settle_session_mark`）も OS の終了の受け手（`end_session_within`）も**そのスレッドを知らない**ので、後始末が済んでプロセスが終われば手続きは途中で断たれる。`areka-nar` の確定は宛先ごとに「宛先 → 作業フォルダの `old-<k>` へ `rename`」「組み上げた木 → 宛先へ `rename`」の 2 手（`crates/areka-nar/src/install.rs` の `commit_one`）なので、2 手の間で断たれると**元のゴーストのフォルダが消え**、元の木は `.nar-work/…/old-<k>/` に残る（7 日の保持の後に片付けの対象）。展開の途中で断たれるのは無害（宛先は無傷・作業フォルダは次の展開の開始時に片付く）。`NarArchive::install` に入る前（利用条件の箱・ファイル選択・`manifest()` の照合）で断たれるのも無害（まだ何も書いていない）。
   - **乗れる点**: 期限の数え方 `WaitBudget { started, limit }` と 3 秒の定数がそのまま使える（OS の終了では手続きの待ちも同じ `started` から数え、SHIORI の待ちと足し算にしない）。印の判定は変えずに済む——断たれても次の起動は、前回のゴーストが根に見つからなければ `warn!(last_ghost_not_found)` を残して次の候補へ進む既存の経路（`boot_resolve.rs` の起動の解決）で起きる。断ったことは `warn!` で必ず残す。
   - **ぶつかる点**: 待つなら `main.rs`（後始末）と `session_end.rs`（期限の中）にそれぞれ数行の呼び出しが入る＝前節の「触らない: `main.rs`」が外れる。待ちは必ず有界にする（`NarArchive::install` の中にいる間だけ待ち、箱やファイル選択で止まっている間は待たずに断つ）。
   - 同じ形の穴は `network-update` の背景の更新（一時フォルダからの確定）にもある見込み（未確認）。本仕様が「背景の手続きを終了で待つ」口を作れば `network-update` はそれに乗れる。→ 議題 ⑷。
7. **利用条件の箱とファイル選択は UI スレッドで出さない**（設計で決める how・勝者が明白）。メニューの動作 `menu::MenuAction` は `Fn(&mut World, &MenuContext)`＝World を借りたまま呼ばれる。そこで `GetOpenFileNameW`／`MessageBoxW` を出すと、入れ子のメッセージループの間に届いた窓のメッセージは World を借りられずに捨てられる。とくに `WM_ENDSESSION` は wintf の受け手（`window_proc/lifecycle.rs` の `WM_ENDSESSION`）が `warn!(os_session_end_world_busy)` だけ残して後始末を飛ばす＝印が残り、次の起動が emo2 の `halt` で始まる。したがって箱とファイル選択は手続きの背景スレッドで出す（`alert.rs` の `raise` は既に `MessageBoxW(None, …)`＝持ち主の窓を持たず、どのスレッドからも呼べる）。**持ち主の窓は渡さない**（UI の窓を持ち主にすると、別スレッドの箱が `EnableWindow` などで持ち主へ同期にメッセージを送り、後始末で UI スレッドが待っている間に互いを待つ形になりうる。前に出すには `MB_SETFOREGROUND` などの旗で足りる）。ファイル選択のスレッドを COM の STA で初期化する要否は設計で確かめる（WUC の MTA とは別スレッド＝衝突しない）。09-24 節の未測定「UI スレッドで `MessageBoxW` の入れ子のモーダルが出ている間の ECS の刻み」は、この形なら測らなくてよい。
   - `session-mark-residue` が置いた本番ソースの同期の送信の検査（`session_end_sync_send_tests.rs` の `SYNC_SEND_TOKENS`／`ALLOWED_SYNC_SENDS`）は `SendMessage*` 系だけを数え、`MessageBoxW`・`GetOpenFileNameW`・`DragQueryFileW`・`DragFinish` は数えない＝本仕様で表は動かない（`SendMessage*` を足さない限り）。

**議題（3 件 → 4 件・答えで作業が変わるものだけ）**

- ⑴ **起動中のゴーストを上書きするとき**（前節のまま (a)／(b)／(c)）。(b)／(c) なら `ghost_switch.rs` は上の 2 の 2 行だけ。(a) は上の 2 のとおり手間が増えた。(b) の「既定の定常を待ってから入れる」待ちの状態は、OS の終了で切替の予約 `SwitchInFlight` が下ろされる所（`end_session_within` の冒頭）と同じ扱いで下ろして `warn!` を残す（⑷ と同じ口で扱える）。
- ⑵ **巻き戻せなかったときの伝え方**（前節のまま）。上の 6 の「終了で断たれた」場合も元の木は同じ `old-<k>` に残るが、次の起動では伝える手続きがもう無く記録だけになる——同じ扱いでよいかをここで一緒に決める。
- ⑶ **利用条件の箱の閉じるボタン**（前節のまま）。`AREKA_NO_ALERT` のときの既定もここ。箱を背景スレッドで出すこと（上の 7）は答えに影響しない。
- ⑷ **新規: install の途中でアプリが終わるとき**（上の 6）。(a) 待つ——`NarArchive::install` の中にいる間だけ有界に待つ（メニューの「終了」などの後始末では上限を要件で決める・OS の終了では 3 秒の残りだけ）。`main.rs`・`session_end.rs` に入り +1 タスク、`network-update` が同じ口に乗れる／(b) 待たない——既知の制限として `alpha-release-signoff` の既知の制限の候補へ送る。見える差＝「入れている最中にアプリを終えたり Windows を終了したりしたとき、元のゴーストのフォルダが消えることがあるか」。

**規模**: 14〜17 → **15〜18**（+1＝⑷ を (a) で答えたときの終了時の待ち。(b) なら 14〜17 のまま。⑴ を (a) で答えるとさらに +1＝テストの分割）。`ghost_switch.rs` の 2 行（受け皿への書き込み 0.5 に含む）・箱とファイル選択を背景スレッドで出すこと（利用条件 1〜2・メニューとファイル選択 1〜2 に含む）・許可表を分けないこと（0）は内訳に収まる。上限 20 の内側。投げ込みは切り出さない（前節の判断のまま）。

**要件定義の Fable 要否＝○（据え置き）**: 議題が 4 件に増え、⑴ と ⑷ はどちらも `session-mark-residue` が固めた終了の経路（印の判定・3 秒の期限）と切替の経路を読み合わせて答えを出す必要があり、⑷ の答えは `network-update` が乗る口の形まで決めるため。

**触るファイル（確定形・main `10a8d724` の行数）**

- 新規: `crates/areka/src/install.rs`（`NarArchive::open` → `manifest()` → 照合 → `install` の手続き・背景スレッド・利用条件の箱とファイル選択もこのスレッドで出す・結果を Ref 列へ写す・戻りの受け口・⑷ (a) なら終了で待つ口）・その兄弟テスト（`install_tests.rs` など）・`emo2_boot/install_cue.rs`・`menu/install_frame.rs`・`input_events/drop.rs`・`terms.rs`・wintf `ecs/window_proc/drop.rs`（`WM_DROPFILES` → `DragQueryFileW` → `DragFinish`）
- 既存: `crates/areka/Cargo.toml` 72（`areka-nar`・`Win32_UI_Controls_Dialogs`）・`ghost_session.rs` 691（`boot_wired` に登記 1 行・`register_systems` に受け口の系 1 行）・`emo2_boot/mod.rs` 805（mod 宣言・channel・sink）・`emo2_boot/consumer_ledger.rs` 756（`canonical()` に 1 行＋`CommandConsumer` に変種 1 つ）・`emo2_boot/ghost_switch.rs` 866（`record_last_installed` の `#[allow(dead_code)]` と注釈の 2 行を消す）・`menu/mod.rs` 337（mod 1 行）・`input_events/mod.rs` 545（mod 1 行）・`alert.rs` 191（はい／いいえを返す口）・`placement/spawn.rs` 769（`ex_style` に `WS_EX_ACCEPTFILES`）・`areka-ghost/src/catalog.rs` 324（`install.accept` の読み手）・kanade `schedule/events.rs` 535（許可表に 10 行）＋`events_change_tests.rs`（13 → 23）・wintf `window_proc/mod.rs` 332（1 腕）・wintf `window/components.rs` 348（`OnFileDrop` の部品）・`areka-nar/src/*`（`// ukadoc:` の行）・`dist/README.txt` 103（「インストール」を 2 行から消す）・台帳 `assets.toml`／`shiori.toml`／`sakura-script.toml`＋`roadmap-draft.md`（`[briefs].count` 36 → 37 は生成器で）＋生成物＋`doc/COMPAT_ARCHITECTURE.md` §8
- ⑷ を (a) で答えたとき: `main.rs` 927（後始末で待つ呼び出し）・`session_end.rs` 163（期限の中で待つ呼び出し）・兄弟テストは新しいファイルへ（`session_end_deadline_tests.rs` 523 行・`main_session_mark_tests.rs` 938 行には足さない）
- ⑴ を (a) で答えたとき: `ghost_switch.rs` の `switch_to` に口・`ghost_switch_tests.rs` 987 の分割
- **触らない**: kanade `msg.rs`／`actor.rs`／`schedule/mod.rs`／`schedule/change.rs`／`steady.rs`・`emo2_boot/spine.rs` 993・`ghost_switch_tests.rs` 987（⑴ が (a) でなければ）・`session_mark_verdict` の署名・`areka-parsers/src/sakura/decode.rs`／`areka-sakura/src/compile.rs`

**後続 2 本と共有するファイル**（本仕様 → `shell-balloon-switch` → `network-update` の直列は変わらない）

- `shell-balloon-switch`（B6）と: `ghost_session.rs`・`emo2_boot/mod.rs`・`emo2_boot/consumer_ledger.rs`・`menu/mod.rs`・`areka-ghost/src/catalog.rs`・kanade `schedule/events.rs`＋`events_change_tests.rs`（許可表の数の直書き＝本仕様の 23 から同 spec が +3 で 26）の 7 本。条件付きで `main.rs`（同 spec は `record_last_used` を呼び足すとき・本仕様は ⑷ (a) のとき）。文書は `dist/README.txt` の**同じ 2 行**（「シェル・バルーンの切り替え、インストール、…」＝前節に無かった重なり）・`sakura-script.toml`・`shiori.toml`・生成物・§8。
- `network-update`（B7）と: `crates/areka/Cargo.toml`・`ghost_session.rs`・`emo2_boot/mod.rs`・`emo2_boot/consumer_ledger.rs`・`menu/mod.rs`・`areka-ghost/src/catalog.rs`・kanade `schedule/events.rs`＋`events_change_tests.rs` の 8 本。条件付きで `emo2_boot/install_cue.rs`（`\![execute,install,url]`＝同じ鍵 `("execute", Some("install"))`）・`alert.rs`（同 spec の議題 ⑵ を「箱を出す」で答えたとき）・**`install.rs` の終了で待つ口と `main.rs`・`session_end.rs`**（本仕様の ⑷ を (a) で答え、同 spec が背景の更新をそれに乗せるとき＝新規）。文書は `assets.toml` の `descript_install`・`roadmap-draft.md`・`dist/README.txt` の同じ 2 行・生成物・§8。

**バグ**: 読んだ範囲（`session_end.rs`・`main.rs` の印の判定・`ghost_session.rs` の `shutdown_within`・`ghost_switch.rs` の受け皿・wintf `lifecycle.rs` の `WM_ENDSESSION`・`areka-nar` の `commit_one`）で、今の main に利用者から見える実害は見つからなかった。上の 6・7 は本仕様が作る背景スレッドとモーダルの箱で初めて生じる穴で、設計で塞ぐ。

## 2026-09-27 `ghost-change-name-resolution` の着地＝`lastinstalled` の受け皿は同 spec が持つ

同 spec が先に着地した（完了 spec は `.kiro/specs/completed/areka-P0-ghost-change-name-resolution/`・実機の記録は同じフォルダの `signoff.md`）。下の 09-26 節の「本仕様が先なら `install.rs` に `LastInstalled` を置く」の分岐は消え、本仕様は**書くだけ**になった。本仕様の要件・設計に効く事実:

1. **受け皿の名前は固定**（同 spec の design「設計で決めたこと」）: 記録の型 `LastInstalledGhost`（`crates/areka/src/emo2_boot/ghost_switch.rs`・`#[derive(Resource)]`・フォルダ名 1 つ・プロセスの中だけ・ファイルや記憶へは書かない・`lastinstalled` で使っても消えない）と、書く口 `record_last_installed(world: &mut World, folder: String)`（置き換えて `info!(event="last_installed_recorded", folder)` を 1 件残す）。本仕様はインストールに成功したゴーストのフォルダ名でこの書く口を呼ぶだけ。今は `#[allow(dead_code)]` で呼び手を本仕様と注釈しているので、呼んだら allow を外す。
2. **`\![change,ghost,lastinstalled]` の実機確認は本仕様が行う**（同 spec は本番の呼び手が無いため決定論テストだけで固定した）。一周は「インストール → `lastinstalled` で切替 → `ghost_switch_resolved name=lastinstalled` → `ghost_switch_done`」。記録が無ければ `ghost_switch_unknown reason=lastinstalled_none`、記録のゴーストが目録に無ければ `reason=lastinstalled_missing` で切替を無視する。
3. **`ghost_switch_tests.rs` は 987 行で上限 1,000 行の目前**。本仕様が `ghost_switch.rs` の入口にテストを足すなら、先にテストファイルを分ける。
4. 共有していた `sakura-script.toml`・`roadmap-draft.md`（`[briefs].count` は同 spec で 36）は同 spec の側で更新済み。本仕様が +1 するときは生成器で生成物を作り直す。

## 2026-09-26 先進坑 `pilot-dropfiles-on-wuc-window` の結果＝**go**（開発者判定）

`_Depends(confirmed): pilot-dropfiles-on-wuc-window` は充足。一次記録は `crates/pilot/examples/pilot-dropfiles-on-wuc-window/README.md` の「検証結果」（完了 spec は `.kiro/specs/completed/pilot-dropfiles-on-wuc-window/`）。本仕様の要件・設計に効く事実:

1. **投げ込みは `WM_DROPFILES` で行ける**（棚卸⑭の仮裁定 7 はそのまま・`IDropTarget` への切り替えは不要）。本番と同じ様式＋`WS_EX_ACCEPTFILES` の窓で、絵の上への落とし 5 回がすべて窓手続きまで届き `DragQueryFileW` でパスが取れた。受け口は wintf の `dispatch_window_message` の表に 1 分岐足す形で足りる見込み（先進坑は `SetWindowSubclass` で重ねた手続きで受けた）。
2. **宣言は `WS_EX_ACCEPTFILES` のビットだけで足りる**。`DragAcceptFiles`・生成後の付け直しは要らなかった（生成直後の読み戻しで `accept_files=true`・`raw=0x200090`）。上の「現状」4 の見込みは実測で裏付けられた＝`placement/spawn.rs` の `ex_style` に 1 ビット足すだけ。
3. **絵の外（クリック透過中）に落とすと窓には届かず背後の窓へ抜ける**。落とし先は OS が `WS_EX_TRANSPARENT` のビットで決め、wintf のカーソル監視（12ms 周期）はドラッグ中も付け外しを続ける＝**本仕様で「絵の外なら捨てる」処理は要らない**。透過の付け外しを 36 回繰り返した後も `WS_EX_ACCEPTFILES` は残った。
4. **既知の制限の候補**: 絵の縁ぎりぎりに素早く落とすと、12ms の監視が追い付く前のビットで落とし先が決まる可能性は残る（実測では縁の 7 物理 px 内側でも正しく届いた）。
5. 配送は待ち行列経由（tick の途中の同期配送は観測されず）。重ねた手続きの後片付けで終了時の警告は 0 件。ふつうの権限で測った（管理者として起動したときの手当て `ChangeWindowMessageFilterEx` は対象外のまま）。

## 2026-09-26 棚卸⑰の再測定（main `13b72893`＝`shiori-fault-notice`・`ghost-restart-unit` の着地後）

**棚卸⑰の裁定（本仕様に効くもの）**: ⓐ 投げ込みが WUC 合成の窓に届くかを先に確かめる**先進坑を別 spec に切り出した＝`pilot-dropfiles-on-wuc-window`**（`crates/pilot/` だけ・`ghost-shell-balloon-switch` と並走）。本仕様は `_Depends(confirmed): pilot-dropfiles-on-wuc-window`＝**要件（`/kiro-start`）は先進坑の go 判定の後**。届かなければ設計が `WM_DROPFILES` から `IDropTarget`（OLE・STA が要り WUC の MTA と衝突しうる＝棚卸⑭の仮裁定 7 の見直し）へ変わるからである。ⓑ `lastinstalled` の受け皿は **`areka-P0-ghost-change-name-resolution` が持つ**（本仕様は入れた直後に `ghost-change-name-resolution` の受け皿へ書くだけ）。順序は `ghost-shell-balloon-switch` → `shell-balloon-switch` ∥ `ghost-change-name-resolution` → 本仕様。ⓒ 要件定義は **Fable** に格上げ（議題が 3 件に増え、`ghost-shell-balloon-switch` の起こし直しと利用条件の閉じるボタンの読みが絡む・想定 17〜20 タスクで上限に張り付く）。

**`shiori-fault-notice` の一般化で変わったところ**（09-24 節の前提 5 の更新）
1. `alert.rs`（191 行）は場面ごとに題名を持てる形になった（`alert_text` が `(題名, 本文)` を返す・場面は 5 つ＝`RootMissing`／`GhostMissing`／`BalloonMissing`／`StartupWindow`／`ShioriFault`）。失敗の告知は場面を 1 つ足せば乗れる。**ただし `shiori-fault-notice` は「一般化」まではしていない＝押されたボタンを今も返さない**（`raise(scene, suppressed)` は `MB_OK | MB_ICONERROR` の箱を出して戻りを見ない）。`shiori-fault-notice` の要件 5.4 が「押されたボタンを返す形は `ghost-install` が足す」と明記している。`terms.txt` の受諾／拒否は **`raise` の隣に「はい／いいえを返す」口を 1 つ足す**（1 タスク・既存 5 場面の文面テスト 9 本は不変）。`AREKA_NO_ALERT` で抑えたときに受諾と拒否のどちらにするかは要件で決める。

**`ghost-restart-unit` で変わったところ**
2. ゴーストごとの結線は `ghost_session.rs`（454 行）の `boot_ghost` に集まった（`wire_mouse_input`・`wire_menu`・`wire_balloon_choice`・`wire_choice_drain` をここで挿す）。台本の受け口の受信端と落とし物の送り口はここで挿す＝**触るファイルに `ghost_session.rs` を足す**。窓へのハンドラ装着は同ファイルの `open_ghost_windows`（`attach_balloon_pointer_handlers` と同じ場所）。
3. 起動中のゴーストを上書きして起こし直す順序は `ghost-restart-unit` の `GhostSession::shutdown` → `close_windows_for_restart` → `reopen_ghost_windows` → `boot_ghost`。ただし「1 周目の停止通知が受け口に残って次フレームで終了へ進む」（`ghost-restart-unit` → `ghost-shell-balloon-switch` の申し送り）はこの経路にも掛かるので、`ghost-shell-balloon-switch` の捌き方を待つ（議題 ⑴ は `ghost-shell-balloon-switch` の設計待ち）。

**崩れた前提 1 件**: 09-24 節の前提 9 の「`areka-update` は `areka-nar` に依存」は誤り（`network-update` の再測定）。本番の依存は無く、dev の `sample-ghost-kit` 経由だけ。`areka_nar` の型を改名してもエンジンには影響しない。

**見落としていた近道**
4. 落とし物の受け入れは `DragAcceptFiles` を呼ばず、ゴースト窓の拡張スタイルに `WS_EX_ACCEPTFILES` を足すだけで済む見込み（`placement/spawn.rs` の `WindowStyle { style: WS_POPUP|WS_VISIBLE, ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW }` の 1 行。wintf の `window_factory.rs` は `WS_EX_LAYERED` を外し `WS_EX_NOREDIRECTIONBITMAP` を足す以外の bit を通す）。`WM_DROPFILES` の腕と `DragQueryFileW` は wintf 側に要る（変わらず）。**クリック透過のトグル（`WS_EX_TRANSPARENT`）中の窓に落とし物が届くかは未測定**＝先進坑 `pilot-dropfiles-on-wuc-window` が答える。

**不変（再確認）**: `WM_DROPFILES`／`DragAcceptFiles`／`DragQueryFile`／`IDropTarget`／`GetOpenFileName` は `crates/` で 0 件。振り分け表は `crates/wintf/src/ecs/window_proc/mod.rs`（331 行）の `dispatch_window_message`。`Win32_UI_Controls_Dialogs` は未有効。`crates/areka/Cargo.toml` に `areka-nar` 無し。`Identity` 7 項目・`sakura.name` 無し。`areka_nar::{InstallRequest, InstallOutcome}` の再公開は残る（改名の議題は不変）。`Frame::Install`・「インストール…」は在り、`menu::register` は `#[allow(dead_code)]`。台帳: `descript_install` の 11 行は `absent`／`owner="areka-P0-nar-install"`、`OnInstall*`／`OnFileDrop2`／`OnDirectoryDrop`／`OnGhostTerms*` は `absent`／`owner=""`、`roadmap-draft.md` に本仕様の行は無い。

**接触ファイル**: 新規 `crates/areka/src/install.rs`（手続き・`areka-nar` 3 段の呼び出し・結果型）・`emo2_boot/install_cue.rs`（`network-update` が改変）・`input_events/drop.rs`・wintf `window_proc/drop.rs`（`WM_DROPFILES` の腕＋`DragQueryFileW`）とハンドラ用の ECS 型・`terms.rs`・kanade の相のファイル（前例 `schedule/user_break.rs`）／既存 `crates/areka/Cargo.toml`・`main.rs` 556・`ghost_session.rs` 454・`emo2_boot/mod.rs` 767・`consumer_ledger.rs` 726・`alert.rs` 191・`menu/mod.rs` 339・`placement/spawn.rs` 767（`ex_style` 1 行）・`areka-ghost/src/catalog.rs` 303・wintf `window_proc/mod.rs` 331・kanade（`ghost-shell-balloon-switch` の汎用の通知の入口があれば `schedule/events.rs` の許可表中心）・`areka-nar/src/*`（`// ukadoc:` の 1 行）・台帳 3 本＋`roadmap-draft.md`＋生成物。上限が近い: kanade `steady.rs` 935・`actor_tests.rs` 970・`schedule_tests.rs` 962＝足さない。

**依存の判定**: `ghost-shell-balloon-switch` とは `ghost_session.rs`・`emo2_boot/mod.rs`・`consumer_ledger.rs`・`menu/mod.rs`・`alert.rs`・kanade を共有＝並走不可。**`shell-balloon-switch` への依存は弱い**（バルーン `.nar` は切り替えず記憶だけ・シェルの一覧はメニューを開くたびに列挙し直す）が、`consumer_ledger.rs`・`emo2_boot/mod.rs`・`menu/mod.rs` を共有するので直列。→ 棚卸⑱: `shell-balloon-switch` は機能の前提ではない（共有ファイルの都合だけ・順序は入れ替えられる）。`ghost-change-name-resolution` とは共有 0 で完全並走できる。

**タスク数**: 17〜20（`ghost-shell-balloon-switch` が汎用の通知の入口を作る前提・無ければ +3）。**要件の時点で「投げ込み」の切り出しを決める**——20 を超えるなら wintf の腕・ハンドラ型・`OnFileDrop2`／`OnDirectoryDrop`・振り分け（5〜6 本）を `areka-P0-file-drop` へ（kanade を通るので本仕様とは直列のまま）。

**正典の逐語（要件で写す）**: `OnInstallRefuse` は Ref0（accept の本体側名）に加えて Ref1＝識別子・Ref2＝名前（2.4.85）／`OnInstallCompleteEx`・`OnInstallCompleteAll` の Ref0〜2 は byte 1 区切りの識別子・名前・**インストールした場所**（`OnInstallComplete` の Ref2 だけ「同梱バルーンの名前」）／`OnFileDrop2` Ref2＝MIME（2.7.98）・`OnDirectoryDrop` Ref0＝パス・Ref1＝スコープ／`OnGhostTermsAccept`／`Decline` は「右上の閉じるボタンでは何もイベントは発生しない」／`terms.txt|md` は**バルーンにも**置ける（`manual_directory`「(myballoon)\terms.txt または terms.md」）＝バルーン `.nar` も同じ経路で表示する（追加の手間 0）／`\![execute,install,path]` は「フルパスで行うこと（相対パス指定不可）」／`\![open,terms]`（2.5.33）は Out に明記する。

**議題の追加**: ⑶ 利用条件の箱の右上の閉じるボタン——はい／いいえ（閉じるボタンの無い形）にするか、閉じる＝拒否と読んで `OnGhostTermsDecline` を送るか（正典は「閉じるでは何も発生しない」。`MessageBoxW` の OK／キャンセルでは閉じる＝キャンセルと区別できない）。見える差は「閉じたときにゴーストが何か言うか」。

## 2026-09-24 棚卸⑯の再測定（main `0b01f654`）

**`ghost-install` と `network-update` は直列のまま**（共有するソースファイル 12 本＝`crates/areka/Cargo.toml`・`main.rs`・`emo2_boot/{consumer_ledger,mod}.rs`・`emo2_boot/install_cue.rs`（本仕様が作り `network-update` が改変する＝消費者台帳の登記の単位は「名前＋第 1 引数」なので `\![execute,install,path]` と `\![execute,install,url]` は同じ鍵 `("execute", Some("install"))` になり同じ受け口でしか扱えない＝**構造上確定**）・`alert.rs`・`areka-ghost/src/catalog.rs`・kanade `msg.rs`／`actor.rs`／`schedule/{mod,events}.rs`・`Input` の網羅 match を持つ kanade のテスト支援。文書類＝台帳 3 本・`roadmap-draft.md`・生成物・§8）。**実装は `ghost-shell-balloon-switch`・`shell-balloon-switch` の着地待ち**（両方とも brief だけ）。要件と設計は先行できる。想定 **17〜19 タスク**（手続きの段が縮んだ分と `alert` の拡張・`accept` の読み手が増えた分がほぼ相殺）。20 を超えそうなら「投げ込み」（wintf の `WM_DROPFILES`・`OnFileDrop2`／`OnDirectoryDrop`）を別 spec へ切り出す（wintf と入力系が中心で `network-update` との共有なし）。

**崩れた／変わった前提**

1. **フォルダを落としたときの正典は `OnDirectoryDrop`**（ukadoc で確認・束「投げ込み」の members にも在る）。Desired Outcome 6「ファイル・フォルダを `OnFileDrop2` へ」はずれている＝要件で `OnDirectoryDrop` を足す。
2. **`accept` の照合に要る値が目録に無い。** 正典の照合相手は `sakura.name` か受け手ゴーストの `install.accept`。`areka_ghost::catalog::Identity` は 7 項目で閉じ（`baseware-root-layout` 要件 2.9 が `sakura.name` を明示的に除外）、どちらも持たない。前例 `catalog::companion_balloon`（1 つの鍵だけを読む関数）と同じ形の読み手を足せば `Identity` を変えずに済む。
3. **Approach 段 ① `install(root, path) -> InstallOutcome` は不要。** `NarArchive::open(path)`（全部を検証し、まだ何も書かない）→ `.manifest()`（`accept`・`kind`・`name` が取れる＝ここで照合して `OnInstallBegin` を送れる）→ `.install(&InstallRequest{root, target_ghost})` の 3 段が既に在る。成功時は `InstallOutcome.installed: Vec<InstalledElement{kind,name,path,target_ghost,existing}>` を `OnInstallCompleteEx` の Ref0〜2 へ写せる。`OnInstallFailure` の理由は `RefuseReason::kind()`（`ALL_KINDS` も公開済み）。
4. **09-19 の申し送り 2〜4 は `nar-install-hardening` で完了**（長さ上限 200＝UTF-16 の単位・超えたら `RefuseReason::PathTooLong` で書庫全体を拒否／巻き戻せなかった元の木＝`NarError::Io.survivors: Box<[SurvivingTree{destination,path}]>`・作業フォルダは 7 日保持／確定の段の失敗で `work` の欄を見るテスト）。下の 09-19 節の 2〜4 番は**古い**。残る仕事は在りかを告知に載せるかどうかと文面だけ（議題 2）。
5. **利用者向け告知 `crates/areka/src/alert.rs` はそのままでは使えない**（すべて `pub(crate)`・場面は起動専用の 4 つ・題名「areka を起動できません」固定・`MB_OK|MB_ICONERROR` のみで `raise` は押されたボタンを返さない）。`terms.txt` の受諾／拒否（OK／キャンセル）と失敗告知には拡張が要る。**`shiori-fault-notice`（B1）が先に一般化する**ので本仕様はそれに乗る。
6. **`Win32_UI_Controls_Dialogs` は未有効**（根は `Win32_UI_Controls`・`Win32_UI_Shell` のみ）。前例は「クレート側で機能を上乗せ」（`crates/areka-update/Cargo.toml`・`dola`）＝根ではなく `crates/areka/Cargo.toml` に書く。
7. **`WM_DROPFILES`／`DragAcceptFiles`／`DragQueryFile`／`IDropTarget`／`OnFileDrop` は `crates/` で 0 件**（較正: `WM_ACTIVATE` は 7 ファイル）。振り分けの表は `crates/wintf/src/ecs/window_proc/mod.rs` の `dispatch_window_message` の `match msg.msg`（受け手は `fn(world, entity, hwnd, wparam, lparam) -> Option<LRESULT>`・表に無いものは `_ => None` で既定処理へ）。足し方は「1 腕＋受け手関数」で、置き場所は既存の子 module（`keyboard.rs` など）か新しいファイル。areka の層へ届ける ECS の型は wintf 側に新しく要る。`DragAcceptFiles` は窓を作った直後（`main.rs` の `open_startup_window` 近辺）。
8. **メニューの枠と項目名は済み**＝`menu::Frame::Install` と `captions::FRAME_CAPTIONS` の `ghostinstallbutton.caption`／既定名「インストール…」。本仕様は `menu::register(world, Frame::Install, supplier)` を呼ぶだけ（`register` の最初の呼び手は `ghost-shell-balloon-switch`）。
9. **本体からはまだ辿れない**（`crates/areka/Cargo.toml` に `areka-nar` 無し。依存を持つのは `sample-ghost-kit` と **`areka-update`**＝新事実）。`// ukadoc:` は `crates/areka-nar/src/` に 0 行（較正: リポジトリ全体 322・`crates/` 233）。台帳 `assets.toml` の `descript_install` 11 行は `status="absent"`・`owner="areka-P0-nar-install"` のまま／`roadmap-draft.md` に `areka-P0-ghost-install` の行は無く（`nar-install` の行が `owner_count = 11`）、`shiori.toml` の `OnInstallBegin`・`OnFileDrop2` などは `owner=""`。`[[spec]]` に行を足すなら `[briefs].count` も +1（検査の腕 a／c／f が赤で知らせる）。
10. **背景処理は `WintfTaskPool` でなくてよい。** 同 pool（`crates/wintf/src/ecs/widget/bitmap_source/task_pool.rs`）の戻り道は ECS のコマンドで areka の利用者は 0。areka の前例は `std::thread::spawn`（`emo2_boot/spine.rs`・`shiori_host.rs`）＝イベントは kanade へ `Sender<KanadeMsg>` で送るので専用スレッドが素直。
11. **固定 `.nar` の fixture はファイルとして存在しない**＝`sample_ghost_kit::nar_writer` の組み立て器でテストの中で作る（areka は dev-dependency に持っている）。
12. kanade: `msg.rs` 771・`actor.rs` 507・`schedule/mod.rs` 751・`schedule/events.rs` 431・`steady.rs` 935（相は入らない＝新しい相のファイル・前例 `schedule/user_break.rs` 75 行）。前例 balloon-break では `Input` の変種 1 つでテスト支援・偽の sakura を含む 22 ファイルに波及した。`main.rs` は 774 行。
13. 名前の衝突（`areka_nar::{InstallRequest, InstallOutcome}`・`install.rs`・`lib.rs` から再公開）は**残っている**＝要件で改名。

**要件段階の議題**: ⑴ 起動中のゴーストへ入れる（同じゴーストを上書きする）とき、`OnInstallBegin` → 降ろす → `install` → 起こし直す → `OnInstallComplete` の順序を `ghost-shell-balloon-switch`／`ghost-restart-unit` の「降ろして起こし直す」仕組みにどこまで任せるか（`ghost-shell-balloon-switch` の設計が固まるまで決まらず、本仕様のタスク数を動かす）。⑵ 巻き戻せなかったときの告知に、在りかのパス・「7 日後に消える」の一文・連番付きの別ゴーストとして救い出す案（採ると 1〜2 タスク増）のどれを載せるか。**未測定**: `DragAcceptFiles` をした窓（WUC 合成・クリック透過のトグルあり）に `WM_DROPFILES` が実際に届くか／UI スレッドで `MessageBoxW` の入れ子のモーダルが出ている間の ECS の刻み。束の件数は `linkage.md` の members で「インストール」40・「投げ込み」10（brief の 41 とずれ・原因未確認）。

## 2026-09-20 棚卸⑮の再測定

**実測の追記（main `fe157df1`）**

- **名前の衝突。** 本文の `InstallRequest { path }`／`InstallOutcome` は、`areka_nar::{InstallRequest, InstallOutcome}`（`crates/areka-nar/src/lib.rs`・`install.rs`）と同名で意味が違う。要件段階で改名する。
- 「`Shell::` の呼び出しは 0 件」は、いま 1 件（`crates/areka/src/readme.rs` の `ShellExecuteW`）。
- ファイル選択に要る `Win32_UI_Controls_Dialogs` は未有効（根の `Cargo.toml` に在るのは `Win32_UI_Controls` のみ）＝機能を 1 行足す。
- `WM_DROPFILES` の受け口は `crates/wintf/src/ecs/window_proc/mod.rs` の振り分けの表に 1 分岐足す形。表に `WM_DROPFILES` は 0 行。
- **申し送り 4 件のうち 2〜4 番（長さの上限・巻き戻せなかったときの元の木の在りか・`work` の欄の検査）は `areka-P0-nar-install-hardening`へ移した。** 本仕様に残るのは 1 番（網羅台帳 `descript_install` の 11 行を実装済みへ動かす）と、`crates/areka-nar/src` への正典 URL のコメント行の追記。
  - **2026-09-23 `nar-install-hardening` の要件ディスカッションからの申し送り**: 部品の側は「巻き戻せなかった宛先ごとの元の木の在りかを失敗の値に載せる」「元の木を含む作業フォルダは失敗から 7 日のあいだ片付けで消さず、期限を過ぎたら次の展開の開始時に消す」まで（同 spec 要件 2）。**利用者にどう見せるかは本仕様で決める**: 在りかを告知に載せるか・「7 日後に自動で消える」と伝えるか・元の木を連番付きの別ゴースト（例 `ghost/<名前>-1`）として救い出す形を採るか（開発者の 09-23 の問いかけ。部品が 7 日残すので後から選べる）。なお同じ 7 日の規則は、**成功した展開の後片付けに失敗して旧木 `old-<k>/` が残った作業フォルダ**にも同じく掛かる（部品は両者を区別しない・同 spec 設計「棚の片付けの判定」）。告知の文面はこの場合も含めて決める。
- 切替の相手は 2 本に分かれた: ゴーストの `.nar` を入れた直後の切替は `areka-P0-ghost-shell-balloon-switch`、シェル・バルーンの `.nar` は `areka-P0-shell-balloon-switch`。**本仕様は両方の着地を待つ。**
- 分割後の想定タスクは 17〜20 本（上限の内側）。台本の入口は `consumer_ledger.rs` と `emo2_boot/mod.rs` の 4 点・kanade の 5 ファイル・網羅台帳を触る＝切替の 2 本と `network-update` とは必ず直列。

## Problem

**誰の何が困っているか**: 配布サイトから `.nar` を落としてきた第三者。

正典（`ukadoc:manual_install`）: 「install.txtが適切に用意されていれば、D&Dなどの手段でインストーラ機能が働き、自動的にインストールできる」。今日の areka には D&D も、ファイル選択も、`\![execute,install,path,…]` も無い。`WM_DROPFILES`／`DragQueryFile`／`IDropTarget`／`RegisterDragDrop` は `crates/` に 0 件（`wintf/src/ecs/drag/` はマウスによる窓の移動であってシェルの D&D ではない）。インストール系イベント（`OnInstallBegin`／`OnInstallComplete(Ex)`／`OnInstallCompleteAll`／`OnInstallFailure`／`OnInstallRefuse`）も 0 件。

`nar-install` が着地しても、それは**開発者が `target/` に検体を展開する道具**であり、利用者が `.nar` を渡す入口は別に要る。

## Current State

- **エンジン**: `areka-P0-nar-install`（単独枠・本仕様の前提）が `areka-nar` クレートを建てる——zip 読取・`install.txt`（`type`／`name`／`directory`／`accept`／`charset`／`refresh`／`refreshundeletemask`／`*.directory`／`*.source.directory`）・ファイル名の文字コード・パス安全性・原子的確定。展開先の形は `<根>/ghost/<directory>/`・`<根>/balloon/<balloon.directory>/`（`baseware-root-layout` の根の形）。
- **根**: `baseware-root-layout` が「インストール先」を与える。
- **窓**: wintf の Win32 窓。`Win32_UI_Shell` は有効（ルート `Cargo.toml`）だが `Shell::` の呼び出しは 0 件。`DragAcceptFiles`＋`WM_DROPFILES` は `Win32_UI_Shell` に居る＝依存追加 0。
- **既存の「投げ込み」**: 無し。`OnFileDrop2`（Ref0＝パス・byte 1 区切り・Ref1＝スコープ・Ref2＝MIME）が現行仕様（`ukadoc:list_shiori_event:OnFileDrop2:1`「このイベントが現時点での最新仕様となる」）。

## Desired Outcome

完了時に次が真になっている。

1. **キャラクター窓へ `.nar`（または `.zip`）を落とすとインストールされる。** 手順: `OnInstallBegin` → `areka-nar` で展開（`accept` が別のゴーストを名指ししていれば `OnInstallRefuse` で止める）→ 成功なら `OnInstallCompleteEx`（無応答なら `OnInstallComplete`）（Ref0＝識別子 `ghost`／`shell`／`balloon`／…・Ref1＝`install.txt` の `name`・Ref2＝同梱バルーンの名前）→ 複数なら最後に `OnInstallCompleteAll`。失敗は `OnInstallFailure`（Ref0＝失敗理由）。
2. **メニュー「インストール…」からファイル選択で同じことができる**（`GetOpenFileNameW`＝`Win32_UI_Controls_Dialogs`・依存追加 0）。
3. **台本 `\![execute,install,path,フルパス]` でも同じことができる**（相対パス不可・正典どおり）。
4. **`terms.txt`（`terms.md`）があればインストール前に表示し、受諾で `OnGhostTermsAccept`・拒否で `OnGhostTermsDecline`。** 表示は `MessageBoxW`（OK＝受諾／キャンセル＝拒否）。Markdown の装飾は解釈しない（本文のまま出す）。
5. **インストール直後にそのゴーストへ切り替える**（`ghost-shell-balloon-switch` の `lastinstalled`）。バルーンだけを入れたときは切り替えない（記憶だけ更新）。シェルだけを入れたときはそのゴーストが起動中ならシェル一覧が増える。
6. **`.nar` 以外のファイルを落としたら `OnFileDrop2`** として SHIORI へ渡す（束「投げ込み」の最小＝ファイルとフォルダ。URL・テキストは α 後）。
7. **何も黙って消えない。** 展開の失敗・`accept` 不一致・`install.txt` 不在は `error!`／`warn!` と利用者向け告知の両方。

## Approach

**選んだ形**: 3 つの入口（D&D・ファイル選択・台本）を 1 本の `InstallRequest { path }` に畳み、`areka-nar` を呼ぶ 1 つの手続きへ流す。

| 段 | 中身 | 検証 |
|---|---|---|
| ① 手続き | `install(root, path) -> InstallOutcome`（`areka-nar` の展開＋`accept` 判定＋結果の型）。イベント列は kanade に「インストール相」を足して送る | 決定論: 固定 `.nar` 4 種（ghost・ghost with balloon・balloon・accept 付き shell）で `InstallOutcome` とイベントの Ref を突き合わせる |
| ② 入口 A 台本 | `\![execute,install,path,…]` の消費者（汎用キャリアの name 選別） | 決定論: 台本 → `InstallRequest` |
| ③ 入口 B メニュー | `popup-menu-minimal` の `MenuRegistry` に「インストール…」を登記・`GetOpenFileNameW` | 実窓 1 本 |
| ④ 入口 C D&D | `DragAcceptFiles`＋`WM_DROPFILES`（wintf の窓手続きに 1 分岐）→ 拡張子で `.nar`/`.zip` なら `InstallRequest`、それ以外は `OnFileDrop2` | 決定論: `WM_DROPFILES` の偽メッセージ → 分岐。実窓: 実際に落とす |
| ⑤ 利用条件 | `terms.txt` の検出と `MessageBoxW`・受諾／拒否イベント | 決定論: あり／なし／拒否 |

`WM_DROPFILES` を選ぶ理由: `IDropTarget`（OLE）は COM の初期化（`OleInitialize`＝STA）を要求し、wintf の WUC は MTA で動く（記憶 areka-wuc-runs-on-mta-thread）。`DragAcceptFiles` は COM を要求しない。テキストや URL の投げ込み（`OnTextDrop`／`OnURLDropping`）には `IDropTarget` が要るので、それは α 後にする。

## Scope

- **In**:
  - `InstallRequest` と手続き 1 本（`areka-nar` の呼び出し・`accept` 判定・結果）
  - 入口 3 つ（D&D＝`WM_DROPFILES`・ファイル選択・`\![execute,install,path,…]`）
  - イベント 7 種（`OnInstallBegin`／`OnInstallComplete`／`OnInstallCompleteEx`／`OnInstallCompleteAll`／`OnInstallFailure`／`OnInstallRefuse`／`OnInstallReroute` は多重ゴースト前提なので**送らない**＝`OnInstallRefuse` に落とす）
  - `terms.txt`／`terms.md` の表示と `OnGhostTermsAccept`／`OnGhostTermsDecline`
  - インストール直後の切替（`lastinstalled`）
  - `.nar` 以外の `OnFileDrop2`（ファイル・フォルダ）・`OnFileDropping`（ドラッグ中＝`WM_DROPFILES` では取れないので**送らない**・α 後）
  - 環境変数の置換語 `%lastghostname`／`%lastobjectname`（束「インストール」の 2 件・切替の名前解決と同じ値）
- **Out**:
  - `.nar` の読取と展開そのもの（`nar-install`）
  - `\![execute,install,url,…]`・URL の投げ込み（HTTP が要る＝`network-update`）
  - `readme.txt` の表示（インストール時に開く慣行はあるが α はメニュー「説明書」で足りる）
  - `type,package`／`bootghost`／`plugin`／`headline`／`calendar*`／`language`（`areka-nar` が受理する type は `ghost`・`shell`・`supplement`・`balloon` の 4 つ。他は `OnInstallFailure` 理由 `unsupported type`）
  - `OnTextDrop`／`OnURLDrag*`／`OnOtherObjectDrop*`（`IDropTarget` が要る・α 後）
  - `installedghostname` 等のリソース（PLUGIN 側・α 後）

## Boundary Candidates

- **手続き**（`areka-nar` の上の 1 関数と結果型）
- **入口 3 つ**（それぞれ 1 分岐で `InstallRequest` を作る）
- **イベント相**（kanade の schedule に「インストール」の相）
- **投げ込みの振り分け**（拡張子で install か `OnFileDrop2` か）

## Out of Boundary

- 展開の意味論と安全性（`nar-install`）
- 根の場所（`baseware-root-layout`）
- 切替の実行（`ghost-shell-balloon-switch`）

## Upstream / Downstream

- **Upstream**: `areka-P0-nar-install`（`areka-nar`）／`areka-P0-baseware-root-layout`（根）／`areka-P0-ghost-shell-balloon-switch`（`lastinstalled`）／`areka-P0-popup-menu-minimal`（`MenuRegistry`）／完了仕様 `areka-P0-charset-canon`（`terms.txt` の `charset,` 行）。
- **Downstream**: `network-update`（`\![execute,install,url,…]` は本仕様の `InstallRequest` に「落としてきたファイル」を渡すだけ）・`alpha-release-signoff`（第三者の手順「`.nar` を落とす」）。

## Existing Spec Touchpoints

- **Extends**: なし（新規）。
- **Adjacent**:
  - `areka-P0-nar-install` の Out「利用者が投げた `.nar` を受け取る UI／D&D／インストーラ体験。M2 予約群の範囲」——**その範囲が本仕様**。nar-install の brief に追記済み（2026-09-18）。
  - `areka-P0-translate-pipeline`／`sakura-time-directives`（α 後・kanade の schedule）——本仕様が相を 1 つ足す。後着が rebase。

## Constraints

- 新規の外部依存 0（`Win32_UI_Shell`・`Win32_UI_Controls_Dialogs` は `windows` crate の機能フラグ。後者が未有効なら機能フラグを 1 行足す＝crate の追加ではない）。
- `WM_DROPFILES` は落とされた側の窓の手続きで受ける。窓は UI スレッド固定（記憶 areka-concurrency-model）。展開は時間が掛かるので UI スレッドで行わない（背景プール `WintfTaskPool` へ）。
- パスの安全性は `areka-nar` が持つ。本仕様は **`.nar` の中身を信用しない**前提を崩さない（`accept` の判定も展開前に `install.txt` だけ読んで行う）。
- 決定論テスト網羅は必達。固定 `.nar` 4 種は `nar-install` の fixture を再利用し、増やすなら同じ場所へ。
- 1 ファイル 1,000 行。
- 規模 **M**。

- **2026-09-18 `nar-install` 設計からの申し送り（使用中の宛先）**: `areka-nar` の展開は「作業フォルダに組んでから宛先と入れ替える」形で、宛先の中のファイル（起動中のゴーストの `shiori.dll` 等）が開かれていると入れ替えが失敗し、宛先は無傷のまま `NarError::Io { phase: Commit, rolled_back: true }` が返る。**エンジンは SHIORI の解放を試みない**。起動中のゴーストへ入れる・更新する・切り替える経路は、呼び出し側が先に SHIORI をアンロード（`OnClose` 相当の終了経路）してから `install` を呼ぶこと。

- **2026-09-19 `nar-install` 実装完了からの申し送り（4 件・使用中の宛先）**:
  1. **網羅台帳 `descript_install` の 11 行を「実装済み」へ動かすのは本仕様である。** `doc/ukadoc-coverage/ledger/assets.toml` の 11 項目は引受先の欄に `areka-P0-nar-install` を書いたまま、状態は `absent` で止めてある（`*.directory`／`*.refresh`／`*.refreshundeletemask`／`*.source.directory`／`accept`／`charset`／`directory`／`name`／`refresh`／`refreshundeletemask`／`type` の 11 行）。台帳が定める「実装済み」は 2 つを同時に満たすことで、いま満たしているのは 0 である——⑴ 定義箇所に正典 URL の 1 行（`// ukadoc:`）があること: `crates/areka-nar/src/` にこの綴りは **0 行**（同じ綴りは他クレートに 194 行あるので、数え方は当たる）。⑵ areka が正典どおりに動くこと: `areka-nar` を依存に持つのは試験専用の `sample-ghost-kit` 1 つだけで、`crates/areka/Cargo.toml` の依存一覧に `areka-nar` は無い＝**本体から辿れない**。本仕様が本体へ繋ぐ配線を入れた**そのコミットで**、URL の 1 行を置き、11 行を `implemented` へ動かし、`cargo run -p ukadoc-survey -- report` と `-- report-summary` の 2 本を作り直すこと。
  2. **`.nar` の中の 1 要素あたりの名前・パスの長さに上限が無い。** 本 brief の制約「パスの安全性は `areka-nar` が持つ」は正しく、否定しない——`areka-nar` は名前の検査（絶対パス・親への上り・区切りの正規化）を持っている。決まっていないのは**長さ**だけである。総量には上限があり（`areka-nar` の `container` の `MAX_TOTAL_DECLARED_SIZE` ＝ 1 GiB）、**1 要素あたりの長さだけが決まっていない**ので、30 万文字の名前を持つ要素がそのまま受理される。`nar-install` 側は自ら「実装の欠陥ではなく設計の穴」と書いており（同仕様の `tasks.md` 実装メモ 3.2・`validation-report.md` の引受先の節）、直すには「どこまでの長さを受け入れるか」を誰かが決めるしかない。**利用者が投げ込む `.nar` を受けるのは本仕様なので、受け入れる長さを決めるのは受け口の側の仕様である**——値を決めて `areka-nar` に渡す（または入口で撥ねる）形を要件で置くこと。中身を信用しない前提はそのままで、信用しない相手に「どこで線を引くか」を足す話である。
  3. **確定に失敗したときの表示に、作業フォルダの場所を載せるかが未決。** `areka-nar` の失敗の型（`NarError`）は巻き戻せたかどうか（`rolled_back`）と詰まった宛先のパスは持つが、**利用者の元の木が `<根>/.nar-work/<プロセス識別子>-<連番>/old-<k>/` に生き残っていることを知る手段を持たない**（場所は失敗の記録の `work` の欄にだけ出る）。巻き戻しに失敗したとき、利用者は「どこで詰まったか」は分かるが「自分の元のゴーストがどこにあるか」は分からない。失敗の告知（`OnInstallFailure` の理由と利用者向けの表示）を作るのは本仕様なので、載せるか載せないかをここで決めること。
  4. **失敗の記録の `work` の欄は、確定の段の失敗では一度も検査されていない。** `areka-nar` の全数対応のテストは 13 の固定入力を持つが、13 件はすべて作業フォルダを掘る前（`WorkArea::create` の手前）で拒否されるため、そのテストが `work` について主張しているのは「**空であること**」だけである（`lib_vocabulary_tests.rs` の全数対応のテスト）。確定の段で失敗したときに `work` が実際の場所を持つことは、まだどのテストも見ていない。本仕様が「起動中のゴーストへ入れて確定に失敗する」経路を `NarArchive::install` 経由で踏むテストを書くとき、そこで併せて埋めること。

**`ghost-shell-balloon-switch` からの申し送り（2026-09-27 完了時）**: ⑴ kanade の汎用の通知の入口 `KanadeMsg::RaiseEvent` の許可表は、起動・終了のイベントと共用の `ALLOWED_EVENT_IDS` をそのまま使う。入口専用の表に分けるかは、入口を最初に使う spec で決める。⑵ 終了や起こし直しの経路を足すときは、印を消す判定 `crates/areka/src/main.rs` の `session_mark_verdict` を必ず通す（`ExitOrigin` を足すと網羅の match がコンパイルで止める）。正本は `doc/COMPAT_ARCHITECTURE.md` §8 と完了 spec の design「Boundary Commitments」

## 2026-09-27 棚卸⑱の再測定（main `5a232d2f`＝`ghost-shell-balloon-switch`・`pilot-dropfiles-on-wuc-window`・`alpha-package` の着地後）

**着地した 3 本で変わったところ**

1. **汎用の通知の入口は在る**: `KanadeMsg::RaiseEvent { id, references, method }`（`crates/areka-kanade/src/msg.rs` の `KanadeMsg` の変種）→ `schedule/change.rs` の `on_raise_event` → `events::raise`。許可表 `ALLOWED_EVENT_IDS`（`schedule/events.rs`・13 件）に無い名前と**定常以外**は `warn!` で捨てる（待ち行列に積まない）。したがって本仕様は kanade に「インストール相」を**足さない**＝`msg.rs`・`actor.rs`・`schedule/mod.rs`・新しい相のファイル・`Input` の網羅 match を持つテスト支援（09-24 節 12 の 22 ファイルの波及）は触らない。触る kanade は `schedule/events.rs`（許可表に `OnInstallBegin`／`OnInstallComplete`／`OnInstallCompleteEx`／`OnInstallCompleteAll`／`OnInstallFailure`／`OnInstallRefuse` の 6 件＋`OnGhostTermsAccept`／`Decline` の 2 件＋`OnFileDrop2`／`OnDirectoryDrop` の 2 件）と `events_change_tests.rs`（`ALLOWED_EVENT_IDS.len() == 13` を直書き＝足すたびに数を動かす）だけ。想定タスクから −3。
   - 帰結 1: 「定常以外は捨てる」ので、起動中のゴーストを上書きして起こし直した後の `OnInstallComplete` は、起こし直した先の定常到達の通知 `KanadeNotice::Steady`（`emo2_boot/ghost_switch.rs` の `on_notice` が受ける）を待って送る形になる。起動系列の途中に送ると `warn!(raise_event_not_steady)` だけ残して落ちる。設計で明記する。
   - 帰結 2: 入口専用の許可表に分けるかは「入口を最初に使う spec」（ウェーブ B4 の `shell-balloon-switch`）が決める。本仕様はその答えに乗る。
2. **切替の入口の形が確定した**: `emo2_boot/ghost_switch.rs` の `SwitchRequest { ghost: GhostSpec::{Name, Folder}, raise_event, origin: ChangeOrigin }` と唯一の入口 `request_ghost_switch(world, req) -> SwitchVerdict`。自分自身への切替も同じ経路で降ろして起こし直す（完了 spec の裁定 9）。降ろす→全窓を閉じる→起こす（`GhostSession::shutdown` → `close_windows_for_restart` → `boot_ghost_strict`）と印の扱いは `switch_to` の中に閉じているので、本仕様が終了・起こし直しの経路を**新設しなければ** `session_mark_verdict`・`ExitOrigin` には触らない。09-26 節 3 の「`ghost-shell-balloon-switch` の捌き方を待つ」は解消。
3. **`accept` の照合相手の半分が用意された**: `areka_ghost::catalog::sakura_name(ghost_dir)`（`crates/areka-ghost/src/catalog.rs`・`companion_balloon` と同型の読み手）。正典の照合相手はもう 1 つ `install.accept`（ukadoc `descript_install`「accept,本体側名」＝「\0名,sakura名。もしくはゴースト側descript.txtのinstall.acceptに設定してある名前でも可」、`descript_ghost`「install.accept,名前1,名前2,名前3...」）＝同型の読み手を 1 つ足す（`catalog.rs` 324 行・`Identity` は変えない）。09-24 節 2 は半分解消。
4. **メニュー登記の前例が在る**: `crates/areka/src/menu/ghost_frame.rs`（`super::register(world, Frame::Ghost, Rc::new(供給関数))`・73 行）と、その呼び出し `ghost_session.rs` の `boot_wired`（`menu::wire_menu` の直後・ゴーストを起こすたびにやり直す）。本仕様は `menu/install_frame.rs`（仮）を同型で作り、`menu/mod.rs` に mod 宣言 1 行・`ghost_session.rs` の `boot_wired` に登記 1 行を足す。`GetOpenFileNameW` の機能 `Win32_UI_Controls_Dialogs` は `crates/areka/Cargo.toml` の `windows` に上乗せ（根は今も `Win32_UI_Controls`・`Win32_UI_Shell` のみ・前例 `crates/areka-update/Cargo.toml`）。
5. **台本の受け口の前例**: `emo2_boot/change_cue.rs`（`("change","ghost")` の自己選別＋`Sender` へ送り出すだけ・117 行）。`install_cue.rs` は同型で、`consumer_ledger.rs` の `canonical()`（現行 9 行）に `("execute", Some("install"))` を 1 行登記し `CommandConsumer` に変種 1 つ。UI 側の取り出しの前例は `ghost_switch.rs` の `ChangeRx`／`register_change_drain`（プロセスに 1 回・`ghost_session::register_systems` から）／`drain_change_requests` の対。
6. **`dist/README.txt`（`alpha-package` の成果物）が本仕様を待つ行を持つ**: 「ゴースト・シェル・バルーンの切り替え、インストール、ネットワーク更新の項目は、今の版ではメニューに出ません」「α 版の時点では、次のことはできません: …… .nar ファイルからのインストール、ネットワーク更新」の 2 行と「■ .nar の入れ方（未記入: alpha-release-signoff が仕上げます）」。本仕様の完了時に 2 行の「インストール」を消す（触るファイルに足す・「入れ方」の本文は `alpha-release-signoff`）。brief に無かった申し送り。
7. **落とし物の宣言と受け口の前例**: `placement/spawn.rs` の全ゴースト窓共通の `WindowStyle { ex_style: WS_EX_LAYERED | WS_EX_TOOLWINDOW }` に `WS_EX_ACCEPTFILES` を足す 1 行（先進坑 go・09-26 節）。窓に関数を差す部品の前例は wintf `ecs/window/components.rs` の `OnSessionEnd(pub fn(&mut World, Entity))`（348 行）と `window_proc/lifecycle.rs` の `WM_ENDSESSION` の受け手（`ghost-shell-balloon-switch` が足した）＝同型で `OnFileDrop` の部品と `window_proc/drop.rs`（`WM_DROPFILES` → `DragQueryFileW` → `DragFinish`）を足し、`window_proc/mod.rs` の振り分け表（332 行）に 1 腕。

**告知の方針の変更（2026-09-26 開発者方針「メッセージボックスは無粋・失敗は既定ゴーストの台詞で伝える」）**: Desired Outcome 7 の「利用者向け告知」は、展開の失敗・`accept` 不一致・`install.txt` 不在を**メッセージボックスでは出さない**——`OnInstallFailure`／`OnInstallRefuse` を送ってゴーストが台詞で伝え、`error!`／`warn!` を必ず残す。`alert.rs`（191 行・`raise(scene, suppressed)` は `MB_OK | MB_ICONERROR` で戻りを見ない）に足すのは `terms.txt` の「はい／いいえを返す」口 1 つだけ。09-24 節 5・09-26 節 1 の「失敗の告知の場面を足す」は取り下げ。ゴーストが `OnInstallFailure` に応えないときに何も見えないのは、既定ゴースト emo2 の辞書で拾う（`network-update` 議題 ⑵ と同じ答え）。

**触るファイル（今の実物で引き直し）**
- 新規: `crates/areka/src/install.rs`（`NarArchive::open` → `manifest()` → 照合 → `install(&InstallRequest { root, target_ghost })` の手続き・結果を Ref 列へ写す・背景スレッドと戻りの受け口）・`emo2_boot/install_cue.rs`・`menu/install_frame.rs`・`input_events/drop.rs`（拡張子で install か `OnFileDrop2`／`OnDirectoryDrop` か）・`terms.rs`（`terms.txt|md`・ゴーストとバルーン）・wintf `window_proc/drop.rs`
- 既存: `crates/areka/Cargo.toml`（`areka-nar`・`Win32_UI_Controls_Dialogs`）・`ghost_session.rs` 638（登記 1 行＋受け口の系の登録 1 行）・`emo2_boot/mod.rs` 805（mod 宣言・channel・sink）・`consumer_ledger.rs` 756（1 行＋変種 1 つ）・`menu/mod.rs` 337（mod 1 行）・`input_events/mod.rs`（mod 1 行）・`alert.rs` 191（はい／いいえ）・`placement/spawn.rs` 769（1 行）・`areka-ghost/src/catalog.rs` 324（`install.accept` の読み手）・kanade `schedule/events.rs` 535＋`events_change_tests.rs`・wintf `window_proc/mod.rs` 332・`window/components.rs` 348・`areka-nar/src/*`（`// ukadoc:` は今も 0 行）・`dist/README.txt`・台帳 3 本（`assets.toml` の `descript_install` 11 行は今も `absent`／`owner="areka-P0-nar-install"`、`shiori.toml` の `OnInstall*`／`OnFileDrop2`／`OnDirectoryDrop`／`OnGhostTerms*` は `absent`／`owner=""`、`sakura-script.toml` の `\![execute,install,path,…]` は `absent`／`owner=""`）・`roadmap-draft.md`（本仕様の行なし・`[briefs].count = 35`）・生成物・§8
- **触らない**: `main.rs`（873 行・結線は `boot_wired` へ）・kanade `msg.rs`／`actor.rs`／`schedule/mod.rs`・`ghost_switch.rs`（議題 ⑴ の答えが (a) でなければ）・`areka-parsers/src/sakura/decode.rs`／`areka-sakura/src/compile.rs`

**並走の判定**
- **`shell-balloon-switch` は機能の前提ではない**（バルーンの `.nar` は記憶だけ・シェルの `.nar` は一覧の再列挙で見える・`\![change,shell,lastinstalled]` の読みは同 spec の議題 ⑷ で本仕様は使わない）。ただし `consumer_ledger.rs`・`emo2_boot/mod.rs`・`menu/mod.rs`・`ghost_session.rs`・kanade `schedule/events.rs`（許可表）の 5 本を共有＝**実装は直列**。順序はどちらが先でも動く（先に着地した方が入口専用の許可表の要否を決める）。要件・設計は並走できる。09-26 節の「順序は … `shell-balloon-switch` → 本仕様」は依存ではなく共有ファイルの都合。
- **`ghost-change-name-resolution` とは要件から完了まで並走できる**。機能の前提は `lastinstalled` の受け皿だけ。同 spec の触るファイル（`decode.rs`・`compile.rs`・`areka-ghost` の新ファイル・「該当なし」の腕＝09-26 申し送りで UI 側へ移った実物は `ghost_switch.rs` の `resolve_switch_target`／`request_ghost_switch`）と本仕様の触るファイルの重なりは **0**（本仕様が `ghost_switch.rs` に触らない限り＝議題 ⑴）。共有は `sakura-script.toml` の別の行と `roadmap-draft.md` の `[briefs].count`（両方が 35 から +1 すると 36 で黙って合流する＝合流後に `cargo test -p ukadoc-survey` が赤で知らせ、生成物は生成器で作り直す）。受け皿の所有は**先に着地する方**: 同 spec が B4 で先なら同 spec のまま（本仕様は書くだけ）。本仕様が先なら `install.rs` に `LastInstalled`（プロセス内・NonSend・永続化しない）を置き、同 spec は読むだけ。どちらでも触るファイルは増えない。
- `pilot-dropfiles-on-wuc-window`: 充足（go）。

**`network-update` との共有（数え直し）**: 確実 7 本＝`crates/areka/Cargo.toml`・`consumer_ledger.rs`・`emo2_boot/mod.rs`・`ghost_session.rs`（登記 1 行）・`menu/mod.rs`・kanade `schedule/events.rs`＋`events_change_tests.rs`・`areka-ghost/src/catalog.rs`（`install.accept` と `homeurl` の読み手）。条件付き 2 本＝`install_cue.rs`（`\![execute,install,url]` を α に入れるとき。同じ鍵 `("execute", Some("install"))` は構造上不変）・`alert.rs`（`network-update` 議題 ⑵ を「箱を出す」で答えたとき＝方針上は出さない）。09-24 の 12 本 → **7〜9 本**（両方とも kanade の相を作らなくなった分が減った）。文書は `assets.toml` の同じ表 `descript_install`（11 行と「相対パス」1 行）・`roadmap-draft.md` の数・`dist/README.txt` の同じ 2 行・§8・生成物。**直列のまま**（消費者台帳と sink の組み立ては 1 か所＝切り方で 0 にはできない）。

**想定タスク数**: **14〜17**（09-26 の 17〜20 から −3＝kanade の相・失敗告知の場面・起こし直しの経路が要らなくなった分）。内訳の見立て: 手続きと Ref 列 2〜3・受け口と台帳 1・メニューとファイル選択 1〜2・利用条件 1〜2・上書きの順序 1〜2・受け皿への書き込み 0.5・投げ込み（wintf の部品と腕・宣言・振り分けと `OnFileDrop2`／`OnDirectoryDrop`）4〜5・網羅台帳と `// ukadoc:` 1・README と §8 0.5・実機 1。

**投げ込みの切り出し（仮 `areka-P0-file-drop`）の判断＝切らない**。⑴ 上限の圧が消えた（投げ込み込みで 14〜17）。⑵ 切ると「`.nar` を落としたら install へ渡す」が spec 間の界面になり、しかも `ghost_session.rs`（または `placement/spawn.rs`）・kanade `schedule/events.rs`・台帳を本仕様と共有するので並走の益が無い（`shell-balloon-switch` とも同じファイルで重なる）。⑶ 先進坑 go で設計の分岐（`IDropTarget`）は消え、残る作業は wintf 側 1 部品＋1 腕・宣言 1 行・振り分け 1 ファイル＝4〜5 タスクの直線。切るのは要件で 20 を超えたときだけ（そのときの重なりは上の ⑵ の 3 本）。

**要件定義の Fable 要否＝○（据え置き）**。議題は 3 件（答えで作業が変わるものだけ）:
- ⑴ **起動中のゴーストを上書きするとき**（`areka-nar` は開かれている `shiori.dll` の入れ替えに失敗し `rolled_back: true` で返す・SHIORI の解放は呼び手の責務）: (a) 切替の経路 `switch_to` の「降ろした後・起こす前」に手続きを挟む口を足す（`ghost_switch.rs` に触る＝`ghost-change-name-resolution` と重なる。`SwitchRequest` の形の変更は完了 spec の Revalidation Trigger）／(b) 先に既定ゴーストへ切り替えてから入れる（`request_ghost_switch(GhostSpec::Folder(既定))` → 定常到達 `KanadeNotice::Steady` を待って `install` → 受け皿へ書く。本仕様は既存の入口を呼ぶだけ。見える差＝一瞬 emo2 が出て挨拶する）／(c) 起動中のゴーストと同じ宛先は `OnInstallFailure`（理由 in use）で断る（α の制限として README に書く・見える差＝自分の更新は `network-update` に任せる）。
- ⑵ 巻き戻せなかったときの伝え方（09-24 節 ⑵ のまま。台詞で伝える方針なので、`OnInstallFailure` の Ref に作業フォルダの在りかを載せるか・「7 日で消える」をどこに書くか）。
- ⑶ `terms.txt` の箱の閉じるボタン（09-26 節 ⑶ のまま）。`AREKA_NO_ALERT` のときの既定（受諾か拒否か）もここで決める。
- 議題にしないもの: `OnFileDrop2` の Ref2（MIME）の求め方＝拡張子の表で足りる（設計）。

**申し送りの取り込み**: `ghost-shell-balloon-switch`（許可表の共用・`session_mark_verdict`）は上の段落に在り、本節 1・2 で実物へ写した。`pilot-dropfiles-on-wuc-window` の go は 09-26 節に在る。`alpha-package` からは `dist/README.txt` の 2 行（本節 6・brief に無かった）。

**バグ**: 読んだ範囲（`ghost_switch.rs`・`change_cue.rs`・`menu/ghost_frame.rs`・kanade `schedule/change.rs`・`events.rs`・`alert.rs`・wintf の振り分け表）で利用者から見える実害は見つからなかった。注意 1 件: `on_raise_event` は定常以外を**捨てる**（積まない）ので、汎用の入口を使う後続 spec は起動系列・終了系列の間に頼んだイベントが（`warn!` は出るが）届かないことを前提に置く。
