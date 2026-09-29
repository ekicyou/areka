# ギャップ分析（kiro-validate-gap）: areka-P0-network-update

> 実測日 2026-09-29・本ブランチ `claude/areka-p0-network-update-d76046`（HEAD `a07e76b1`＝main `2ec8df59` の直後）。要件（requirements.md）は確定済みで、本書は要件を変えない。コードは「何の定義か」（型名・関数名・定数名＋ファイルパス）で指し、行番号では指さない。裁定はしない——選択肢と推奨を並べ、要件ディスカッションの材料にする。

## 1. 要約（3〜5 点）

- **足りない部品は「結線」だけで、部品はほぼ揃っている。** エンジン `crates/areka-update`（`run`・`WinHttpFetch`・`Progress`・`UpdateOutcome`・`UpdateError`）、kanade の汎用の通知の入口 `KanadeMsg::RaiseEvent { reply }` と返事 `RaiseOutcome`、リソース照会 `KanadeMsg::ResourceQuery`、インストールの依頼の口 `install::submit`／`InstallOrder`、終了で待つ門 `exit_wait::WorkGate`／`register_gate`、自分自身への切替 `ghost_switch::request_ghost_switch`（`GhostSpec::Folder`）、メニュー枠 `Frame::Update`＋`updatebutton.caption`、消費者台帳 `ConsumerLedger::canonical()`——すべて実在を確認した。本体 `crates/areka/Cargo.toml` の依存に `areka-update` が無いのが最初の 1 行。
- **雛形は `crates/areka/src/install/`（窓口 `desk.rs`・背景スレッド `worker.rs`・純粋な手続き `procedure.rs`＋口 `InstallPorts`）で、そのまま写せる。** 更新の手続きは「World もスレッドも知らない純粋な手続き＋口の trait」に置き、本番の口は背景スレッド、テストは偽の口（`procedure_test_support.rs` の前例）。更新後に**同じゴーストを読み直す＝ゴーストの実行系・`MenuWiring`・cue の受け口が全部新品になる**ので、「走っている」旗と門は `InstallDesk` と同じくプロセスに 1 つの NonSend に置かねばならない（ゴーストごとの受信端 `ChangeRx` の型では、読み直しの瞬間に状態が消える）。
- **要件 5.3「総括の返事の台詞が終わってから読み直す」は既存の仕組みで満たせる。** kanade の `schedule/change.rs` の `on_change_ghost` は `Steady{talk: Some}` で受けた切替要求を `pending_change` に控え、`consume_pending` がトークの完了で消化する。`RaiseEvent` の返事（`actor.rs` の `raise_reply`）は動作の実行（`StartTalk`）の後に返るので、「返事を受けたら切替を頼む」だけで台詞の終わりを待つ形になる（インストールの `overwrite.rs` が同じ前提で書いている＝「再生中の台詞の終わりは kanade の保留が待つ」）。
- **確かめが要る点は 4 つ**: ⑴ メニューの「選べない（灰色）」判定に SHIORI リソース `homeurl` を使うには同期の口が無い（供給関数は `Fn(&World, &MenuContext)`・照会は非同期）／⑵ argv で始めたゴースト（`GhostDecision.folder == None`）は `GhostSpec::Folder` で読み直せない／⑶ 終了の門の「書く段」をどこに置くか（`run` は取得と確定を 1 回の呼び出しで行う・WinHTTP は同期で外から中断できない）／⑷ `updateother --shell=名` の名前引きに `list_shells` を使うと `menu,hidden` のシェルが引けない。いずれも設計の分岐で、要件を動かす必要は無い（③は 7.3 の文言に両案が収まる）。
- **規模 M（13〜16 タスク）・リスク中。** 新規ファイルは `update/`（mod・desk・worker・procedure・refs＋兄弟テスト）と `emo2_boot/update_cue.rs`・`menu/update_frame.rs`・`install/fetch_url.rs`（仮）程度。既存に触るのは 12 ファイル前後で、各 10 行以内が多い。1,000 行の上限に近いのは `main.rs`（943）だが触らない見込み。

## 2. 要件 → 既存資産の対応表

凡例: ✅＝在る／🟡＝在るが形が違う（Constraint）／❌＝無い（Missing）／❓＝設計で確かめる（Unknown）。

| 要件 | 既存資産（定義と場所） | 状態 | ギャップと注意 |
|---|---|---|---|
| 1.1 入口 4 つ → 1 本の手続き | インストールの `submit(world, InstallOrder) -> SubmitVerdict`（`crates/areka/src/install/mod.rs`）が「唯一の口」の前例 | ❌ | 更新の依頼の型（対象の列・理由 `manual`／`script`・総括の種別 `Result`／`ResultEx`）と口を新設。窓口はプロセスに 1 つの NonSend（`InstallDesk` と同じ理由＝読み直しで World の中身が入れ替わる） |
| 1.2 メニュー枠の登記 | `Frame::Update`・`Frame::ORDER`（`menu/mod.rs`）、`FRAME_CAPTIONS` の `("updatebutton.caption", Frame::Update, "ネットワーク更新")`（`menu/captions.rs`）、登記の前例 `install_frame::register`（`menu/install_frame.rs`・`super::register(world, Frame::Install, …)`）、呼び手 `boot_wired`（`ghost_session.rs`・`menu::install_frame::register(world)` の隣） | ✅ 枠 / ❌ 登記 | `menu/update_frame.rs`（30 行）＋`menu/mod.rs` に `pub(crate) mod update_frame;` 1 行＋`boot_wired` に 1 行。`updatebutton.caption` は `ALLOWED_RESOURCE_IDS` に既に在る（照会は `captions.rs` の `PendingQuery` が既に行う）＝台帳の行を実装済みへ動かすだけ |
| 1.3・1.16 灰色（更新先が無い・走っている間） | `install_item` の `enabled: desk::can_pick(world)`（`menu/install_frame.rs`）が「窓口へ尋ねる」前例 | 🟡 | 「走っている間」は窓口の旗で足りる。「更新先が無い」は **同期に分からない**: 供給関数は `Fn(&World, &MenuContext) -> MenuItem` で、SHIORI リソースの照会は `KanadeMsg::ResourceQuery { ids, reply }` の非同期。`descript.txt` の `homeurl` だけなら同期に読める。→ 議題 A |
| 1.4〜1.7 対象の解き方 | 今のゴースト＝`BootContext.current.ghost`（`GhostDecision { route, dir, folder }`・`boot_config.rs`／`boot_resolve.rs`）、今のバルーン＝`BootContext.current.balloon`（`BalloonDecision { dir, folder }`）、今のシェル＝`GhostSession::runtime().mount().shell.dir`（`ghost_session.rs`・`areka-ghost/src/runtime.rs` の `mount()`）、名前＝`GhostSession::names()`（`name`・`sakura_name`）、シェルの目録 `list_shells(ghost_dir)`・バルーンの目録 `list_balloons(root)`（`areka-ghost/src/catalog.rs`） | ✅ | `updateother` の名前引きは目録の `identity.name` と完全一致（大文字小文字を区別）。**注意**: `list_shells` は `menu,hidden` のシェルを落とす＝隠しシェルは名前で引けない（正典は「起動中でないものも指定できる」と書くだけで隠しシェルに触れない）。→ 議題 D |
| 1.8 更新オプション・知らない語は断る | 受け口の前例 `ChangeCueSink`（`emo2_boot/change_cue.rs`・`--option=raise-event` の判定と知らない option の `warn!`） | ❌ | `emo2_boot/update_cue.rs` を新設（`("updatebymyself", None)`・`("update", None)`・`("updateother", None)` の 3 組を自己選別）。断りは受け口で行い、要求を送らない |
| 1.9 更新先の解決（リソース → descript） | リソース照会 `KanadeMsg::ResourceQuery`（`areka-kanade/src/msg.rs`・殻で答える `actor_resources::answer`・定常でなければ全件 `NoContent`）。`descript.txt` の単独の鍵の読み手の前例 `catalog::sakura_name`・`catalog::install_accept`（`master_descript_keys` 経由）、シェル・バルーンは `read_descript(&dir.join("descript.txt"))`（私有） | 🟡 | `ALLOWED_RESOURCE_IDS`（`schedule/resources.rs`・10 語）に `homeurl`・`useorigin1` を足す（12 語）。`catalog.rs` に `homeurl` の読み手を足す（ゴースト＝`ghost/master/descript.txt`・シェル／バルーン＝各 `descript.txt`）＋`// ukadoc:` の行 3 本（8.2）。**照会は背景スレッドから送って `ReplyReceiver::recv()` で待てる**（`Sender<KanadeMsg>` は `Send`・`ResourceQuery` は運行状態機械を経ない） |
| 1.11 URL をそのまま渡す | `run` の入口の検査（`walk` の `InvalidHomeurl`・末尾 `/` の補い `HomeurlSlashAppended`・`areka-update/src/lib.rs`） | ✅ | 触らない |
| 1.12・1.13 `OnUpdateProcessExec`（メニューだけ） | `RaiseEvent { reply: Some(…) }` → `RaiseOutcome::Script`／`NoReply`（`areka-kanade/src/change.rs`・`actor.rs` の `raise_outcome_of`＝空白だけの台本は `NoReply`） | ✅ | 返事を待つのは UI スレッドでは不可（塞ぐ）。**手続きの背景スレッドの最初の段として送る**（出どころがメニューのときだけ）のが素直＝メニューの動作は依頼を積むだけ |
| 1.14・1.15 1 度に 1 本・二重起動は `executing` | `InstallDesk.busy`（`install/desk.rs`）の前例 | ❌ | 窓口の旗。二重起動の `OnUpdateFailure` は UI スレッドから直接 `RaiseEvent`（`reply: None`）で送れる（`input_events/file_drop.rs` の `send_event` が UI スレッドから直接送る前例） |
| 1.17 定常でなければ断る | `on_raise_event` が定常以外を `warn!(raise_event_not_steady)` で捨てる（`schedule/change.rs`）。UI 側は `SwitchInFlight`（`ghost_switch.rs`）の有無で切替中を知れる。起動の途中は `GhostSlot` の有無と定常到達の通知 `KanadeNotice::Steady`（`ghost_switch::on_notice` → `install::desk::on_steady`） | 🟡 | 「定常か」の正本は kanade。UI 側で先に断るには `SwitchInFlight`／`FirstExit`／`GhostSlot` の 3 つを見るのが今の最大（`desk.rs` の `send_held` と同じ）。起動の途中（`KanadeNotice::Steady` 未着）は数えていないので、`InstallDesk::steady_count` と同じ数を持つか、`OnUpdateProcessExec`／`OnUpdateBegin` の `NotSteady` で断るか。→ 設計 |
| 1.18 UI を止めない・同期送信を増やさない | `install/worker.rs`（`spawn_actor("install", …)`＋`run_inbox`・`areka-actor`）と `mpsc` の頼み `DeskAsk`。`ALLOWED_SYNC_SENDS`（`session_end_sync_send_tests.rs`・2 件）は `SendMessageW(`／`SendMessageTimeoutW(` の字面を見張る | ✅ | 同じ形なら影響 0。`WinHttpFetch` は `Send`／`Sync` でない（`winhttp.rs` の定義の注記）＝背景スレッドで作って同じスレッドで使う |
| 1.19 窓の無い起動では受けない | `boot_wired` の登記は `outcome.ghost` が在るときだけ・cue の受け口は `wire_emo2_boot` でだけ組む（`emo2_boot/mod.rs`） | ✅ | 足す経路 0 |
| 2.1〜2.12 イベント列と Reference | `Progress` 6 変種（`areka-update/src/outcome.rs`）: `ManifestFetched`（写す先なし）・`DiffDecided{files}`→`OnUpdateReady`・`DownloadBegin{file,index,total}`→`OnUpdate.OnDownloadBegin`・`Md5Compared{file,expected,actual,matched}`→`OnMD5CompareBegin`＋`Complete`／`Failure`・`Committed{placed}`→`OnUpdateComplete` の Ref1・`Deleted`（写す先なし）。`UpdateOutcome::Unchanged`→`OnUpdateComplete(none)`。`UpdateError::file()`・`FailReason::kind()`（`error.rs`） | ✅ 材料 / ❌ 写し | 写しの純関数（`update/refs.rs` 仮）を新設。`observe: &mut dyn FnMut(&Progress)` は同期の閉包なので、閉包の中から `raise` して返事を待てる（順序が保たれる）。**`Md5Compared` は照合の後に 1 回だけ来る**＝`Begin` と結果を 1 通知から 2 件送る（裁定 14 の実物どおり） |
| 2.2 `OnUpdateOther*` の名 | `ALLOWED_EVENT_IDS`（`schedule/events.rs`・23 語）と `allowed_static`（逐語）。`events_change_tests.rs` の `assert_eq!(ALLOWED_EVENT_IDS.len(), 23)` | ❌ | 20 語を `// ukadoc:` 付きで足す（43 語）・判定の数を 43 へ。`events.rs` は 566 行→約 610 行で上限の内側 |
| 2.3 GET・送り先は送る時点のゴースト | `GhostSession::kanade() -> Option<&Sender<KanadeMsg>>`（`ghost_session.rs`）。`Sender` は `Send`＝背景スレッドへ複製して渡せる | 🟡 | 2 つの形: ⒜ 依頼を受けた時点の送出端を背景スレッドが持つ（切り替わると送出が `Err`／返事が `Dropped` になる＝5.7 の「居なくなった」の検出がそのまま得られる）／⒝ インストールと同じく毎回 UI の窓口へ頼んで「送る時点の置き場」から取る（`desk.rs` の `send_held`）。5.7 が「居なくなったら捨てる」なので両者の観測できる差は 0。⒜ のほうが短い。→ 設計 |
| 2.13・2.14 `useorigin1` | `ResourceQuery` の返事 `ResourceOutcome::Value(String)`／`NoContent`／`Failed` | ❌ | 要求ごとに 1 回照会し `1` かどうかで 0／1 始まりを決める純関数 |
| 3.1〜3.5 総括 | 正典の形 `種別\x01OK\x01件数` は `install/judge.rs` の `SEPARATOR`（byte 1 で繋ぐ `join`）が前例 | ❌ | `OnUpdateResult`／`OnUpdateResultEx` の Reference を組む純関数。**`SEPARATOR` を `install::judge` から借りるか自前に持つか**（同じ値を 2 か所に持たない） |
| 4.2・4.3 失敗理由の表（漏れ 0） | `FailReason`（11 変種・`fail_reasons!` マクロ・`ALL_KINDS`）と `FetchError`（8 変種）（`areka-update/src/error.rs`）。どちらも `pub` で網羅 `match` が書ける | ✅ | `match` にワイルドカードを置かなければ「種類が増えたらビルドが止まる」。テストは `FailReason::ALL_KINDS` の全数を回す |
| 4.4 `WinHttpFetch::new()` の失敗 | `WinHttpFetch::new() -> Result<WinHttpFetch, FetchError>`（`winhttp.rs`・記録 0） | ✅ | 呼ぶ側で `error!`＋理由 `connect`（要件どおり） |
| 4.5〜4.9 記録と `undeletable`／`leftovers` | `log_failure`（`lib.rs`・エンジンの `error!` 1 件）・`UpdateOutcome::Updated { undeletable, leftovers }` | ✅ | 呼ぶ側の `error!`／`warn!` を足すだけ |
| 5.1 ゴーストを降ろさない | `run` の前提「起動中のゴーストなら SHIORI を先に解放している」（完了 `update-engine` design の Preconditions・「呼び出し側が先に解放する」の Boundary）を外して呼ぶ | 🟡 | 完了 spec の文書は変えない。**実機の未知**: 写像中の DLL の改名は較正済み（`commit_tests.rs`）だが、SHIORI が排他で開いているファイル（記録ファイル・保存データ）が定義ファイルに載っていれば `CommitWrite`→戻し→NG になる。emo2 の `updates.txt` の中身次第。→ Research 2 |
| 5.2 読み直し＝自分自身への切替 | `request_ghost_switch(world, SwitchRequest { ghost: GhostSpec::Folder(…), raise_event: false, origin })`（`ghost_switch.rs`）。`begin_change`（`schedule/change.rs`）は `raise_event: false` なら `OnGhostChanging`／`OnClose` を送らず `Unloading{CloseSilent}`。`switch_to` → `take_down`（`shutdown(CloseReason::System)`）→ `close_windows_for_restart` → `boot_into`（`resolve_balloon_for_ghost` でバルーンを解き直す・`reopen_ghost_windows`・`boot_ghost_strict`）。`ghost_session_switch_tests.rs` の `switch_to_self_takes_down_and_reboots_a` | ✅ | **argv で始めたゴーストは `GhostDecision.folder == None`**（`boot_resolve.rs` の `GhostRoute::Argv` の腕）で、`resolve_switch_target` はフォルダ名で目録を引くので読み直せない（`NotFound`）。→ 議題 B |
| 5.3 台詞が終わってから | `on_change_ghost`（`Steady{talk: Some}` → `pending_change`）と `consume_pending`（`TalkDone` で消化）（`schedule/change.rs`）。`RaiseEvent` の返事は動作の実行後（`actor.rs`・「返事はこの依頼の処理…が済んだ後に 1 回だけ送る」） | ✅ | 総括の返事を受けたら切替を頼む＝待ちは kanade が持つ。UI 側で `TalkLifecycleSignal`（`emo2_boot/talk_lifecycle.rs`）を数える必要は無い |
| 5.5 起動の根 `OnGhostChanged` | `boot_root`（`schedule/boot.rs`・`BootOrigin::ChangedFrom`）と `switch_to` の `BootOrigin::ChangedFrom(ChangedFrom{…})` | ✅ | `OnGhostChanged` の Ref は自分→自分（`prev` は降ろす前の `GhostSession::names()`） |
| 5.6 起こせなければ既定へ | `switch_to_default`・`WelcomeAttempt::Default`（`ghost_switch.rs`） | ✅ | 触らない |
| 5.7・5.8 更新中の切替・読み直し中の切替 | `SwitchInFlight` の有無で `ghost_switch_busy`（`request_ghost_switch_with`） | ✅ | 更新中に別の切替が起きたことは、⒜ 送出端の `Err`／返事の `Dropped`、または ⒝ 窓口が `SwitchInFlight` を見て「頼みを落とす」で分かる |
| 5.9 後片付けの残りを残骸と区別 | `WorkArea::cleanup`（自分のフォルダを `remove_dir_all`・消せなければ `leftovers`）と `sweep`（`old/` に中身が残るフォルダは消さず `residue`）（`areka-update/src/work.rs`）。`lib.rs` の `warn_leftover` | 🟡 | エンジン側の改修 1 点。案: ⒜ `cleanup` が消せなかったとき走行フォルダに印のファイル（例 `committed`）を置き、`sweep` は印の在るフォルダを `warn!` 無しで消す（消せなければまた残す・記録も出さない）／⒝ `cleanup` が `old/` の中身を先に消し切れなかったときは `old/` を `old.done/` へ改名し、`sweep` の `has_content(old)` の判定を通らせる（改名は写像中の DLL でも通る）。どちらも `work.rs`＋`work_tests.rs` の中で閉じる。**戻せなかった走行（`keep`）は印を置かない**ので今日どおり警告に出る |
| 5.10・5.11 §8 と `session_mark_verdict` | `session_mark_verdict(first, argv_session, logsink_fallback, end)`（`main.rs`・`ExitOrigin` の網羅 match）。`doc/COMPAT_ARCHITECTURE.md` §8 の表 | ✅ | `ExitOrigin` を足さない（足すと match が止まる＝要件 5.11 の保証はコンパイルで得られる）。§8 に行を足す |
| 6.1〜6.7 `\![execute,install,url,…]` | `InstallCueSink`（`emo2_boot/install_cue.rs`・`script_request` が `["path", …]` 以外を `NotPath { found }` で返す＝`url` は `install_cue_unsupported` の腕）・`RawInstallRequest { path, origin }`・`InstallOrigin::Script`・`desk::raw_sender` | 🟡 | 受け口に `url` の腕を足し、取得は **別の背景スレッド**（インストールの背景スレッド `install` は依頼を 1 件ずつ受けるので、そこで落とすと取得の間ほかの依頼が止まる。更新の背景スレッドとも独立にする＝更新の「1 度に 1 本」を汚さない）。落とし終えたら `RawInstallRequest { path, origin: Script }` を窓口へ送る（既存の口）。一時フォルダは `std::env::temp_dir().join("areka")` 直下（`C:\` 直下は不可・`temp-path-kit` はテスト専用で本番に使わない）。7 日の掃除は次の取得のとき（要件どおり） |
| 7.1〜7.7 終了で待つ | `WorkGate { begin, enter_write, leave_write, end }`・`register_gate(world, name, gate, on_close)`・`begin_close`／`ClosingWaits::wait(WaitBudget)`（`exit_wait.rs`）。呼び手は `main.rs`（`exit_budget`・`closing.wait(exit_budget)`）と `session_end.rs`（`end_session_from`・同じ `WaitBudget { started, limit }`） | ✅ | `register("update", gate, discard_for_exit)` を 1 回。**「書く段」の置き方**: `run` は取得と確定を 1 回の呼び出しで行い、`WinHttpFetch` は同期で外から中断できない。⒜ `run` の全体を「書く段」にする＝終了は最大 3 秒待ち、取得の途中なら上限で打ち切って進む（プロセスの終了でスレッドは消え、`new/` の残りは次の走行の `sweep` が黙って消す）／⒝ 最後の `Md5Compared{matched: true}` の通知で `enter_write` し、それまでは「書く前」（`exit_wait_abandoned` の記録・待たない）。要件 7.3 の文言（「確定の前に断たれた残りは黙って消え…」）は両案に収まる。→ 議題 C。`exit_wait_timeout` の本文が「`<根>/.nar-work/` の下に残っているかもしれない」とインストール固有の語になっている＝更新も同じ口を使うなら本文を門の名前で分けるか一般化する（小） |
| 8.1 記録 | `install/procedure.rs` の `install_begin`／`install_event`／`install_done`／`install_failed` の型 | ✅ | 同じ型で `update_*` を出す |
| 8.2・8.3 台帳 | `doc/ukadoc-coverage/ledger/shiori.toml`（`OnUpdate*` 26 行 `absent`・`homeurl`／`useorigin1`／`other_homeurl_override` `vocabulary-only`・`updatebutton.caption` `vocabulary-only` で `owner = "areka-P0-popup-menu-minimal"`）、`assets.toml`（`descript_install` の「相対パス」`absent`・`owner = "areka-P0-network-update"`、`descript_ghost`／`descript_shell`／`descript_balloon` の `homeurl` `absent`）、`sakura-script.toml`（4 語 `absent`）。`roadmap-draft.md` の本仕様の行 `owner_count = 1`。証拠の規則は `doc/ukadoc-coverage/README.md`（`implemented` には**ソース側の `// ukadoc:` の URL が要る**・行頭のコメントか `///`） | ✅ 道具 / ❌ 状態 | `cargo run -p ukadoc-survey -- evidence` で証拠の実在を確かめてから状態を動かす。`ImplementedWithoutEvidence` は検査で赤になる |
| 8.4 `dist/README.txt` | 「シェル・バルーンの切り替え、ネットワーク更新の項目は、今の版ではメニューに出ません。」「α 版の時点では、次のことはできません: シェル・バルーンの切り替え、ネットワーク更新。」の 2 行 | ✅ | 「ネットワーク更新」の語を外す（`shell-balloon-switch` の分は残る） |
| 8.6 外部クレート 0 | `areka-update/Cargo.toml` は `encoding_rs`・`thiserror`・`tracing`・`windows` | ✅ | `crates/areka/Cargo.toml` に path 依存 1 行。`Cargo.lock` は `areka` の節だけ動く |
| 8.7 1,000 行 | 実測: `ghost_session.rs` 701・`emo2_boot/mod.rs` 816・`consumer_ledger.rs` 786・`menu/mod.rs` 340・`events.rs` 566・`resources.rs` 346・`catalog.rs` 347・`install_cue.rs` 121・`desk.rs` 530・`main.rs` 943・`ghost_switch.rs` 870 | ✅ | 足す行は各 10 行以内。`main.rs` は触らない |
| 9.1〜9.15 決定論テスト | 偽の取得 `FakeFetch`（`areka-update/src/testkit.rs`・`pub(crate)`＝**クレートの外から使えない**）。偽の口の前例 `install/procedure_test_support.rs`。実時間を待たない門のテスト `exit_wait_tests.rs` | 🟡 | 本仕様側の手続きのテストは「エンジンを呼ばず、`Progress` の列と `Result<UpdateOutcome, UpdateError>` を偽の口から流す」形にすれば `FakeFetch` は要らない（手続きの口に「一周を回す」関数を持たせ、テストは固定の列を返す）。エンジン込みの一周は `areka-update` の `run_tests.rs` が既に固定している |
| 9.16・9.17 実機 | `winhttp_real_tests.rs` の `winhttp_real_two_rounds_update_then_unchanged`（`#[ignore]`・ローカル http）。emo2 の `homeurl` は https | ✅ 道具 | https は本仕様の実機で初めて通す。→ Research 1 |

## 3. 実装の形（選択肢）

### 案 A: 既存の `install/` モジュールへ相乗り（拡張）

- `InstallDesk` に更新の待ち行列と旗を足し、`install` の背景スレッドで `run` も回す。
- 利点: 新規ファイルが少ない。門の登記も 1 つで済む。
- 欠点: `desk.rs` 530 行・`worker.rs`・`procedure.rs` が更新の語彙で膨らむ。インストールの「1 件ずつ」の直列に更新が混ざり、`\![execute,install,url]` の取得の待ちでインストールが止まる。読み直し（切替）の道筋に `overwrite.rs` の段が絡む。**取らない**。

### 案 B: 新しい `update/` モジュール（新設・`install/` と同型）

- `crates/areka/src/update/{mod.rs, desk.rs, worker.rs, procedure.rs, refs.rs}`＋兄弟テスト。`mod.rs`＝依頼の型 `UpdateOrder { targets: Vec<UpdateTarget>, reason: UpdateReason, summary: SummaryKind }` と `submit`・`register`。`desk.rs`＝NonSend の窓口（旗 `busy`・門 `gate`・受信端）。`worker.rs`＝背景スレッド `update`（`spawn_actor`＋`run_inbox`）と本物の口。`procedure.rs`＝純粋な手続き（口の trait `UpdatePorts`: `raise`・`query_resources`・`run_engine`・`request_reload`）。`refs.rs`＝`Progress`／結果 → Reference の純関数と失敗理由の表。
- 入口: `menu/update_frame.rs`（依頼を `submit`）・`emo2_boot/update_cue.rs`（`RawUpdateRequest` を窓口の送出端へ・`desk::raw_sender` の型）・`install_cue.rs` の `url` の腕（別スレッドで落とす `install/fetch_url.rs` 仮）。
- 利点: 責務が分かれる。テストは偽の口で閉じる。`install/` は `install_cue.rs`＋取得の 1 ファイルしか触らない。
- 欠点: ファイルが増える（新規 8〜10 本）。`SEPARATOR` のような小さな共有物の置き場を決める必要。

### 案 C: 折衷（案 B＋エンジン側の改修を先頭に）

- 案 B に加え、`crates/areka-update/src/work.rs`（＋`work_tests.rs`・`lib.rs` の呼び出し）の後片付けの改修を**先頭のタスク**として独立に着地させる（誰とも共有しない・要件 5.9・9.10）。
- 利点: エンジンの改修が本体の結線と混ざらず、`cargo test -p areka-update` だけで閉じる。
- 欠点: 特になし（案 B の並べ方の話）。

**推奨: 案 C**（＝案 B のファイル構成・エンジンの改修を先頭）。

## 4. 規模とリスク

- **規模: M（13〜16 タスク）**。内訳の見込み: エンジンの後片付け 1〜2／依存と `homeurl` の読み手と許可表 2 本 2／依頼の型と窓口と門 2／手続きと Reference の写しと失敗の表 3／入口 4 つ（メニュー・cue・`url`）3／読み直しと `OnUpdateProcessExec` 1〜2／台帳・README・§8 1／実機 1。
- **リスク: 中**。根拠: 部品は揃い前例（`install/`）が同型で写せる（低）が、⑴ ゴーストを生かしたまま確定する実機の挙動（排他で開かれたファイル）と https が未確認、⑵ 読み直しの後の窓・バルーンの解き直しが「自分→自分」で実物どおり通るかは `switch_to_self_takes_down_and_reboots_a` が固定しているものの、更新で中身が変わった後の再起動は初めて（中）。

## 5. 要件ディスカッションへ出す設計の分かれ目（番号つき）

議題は「答えで作業が変わる」ものだけに絞る。A〜D は要件の文言に触れ得る。E〜H は設計で決められるが、要件の含意を確かめるために挙げる。

1. **議題 A: メニューの「更新先が無い→灰色」（要件 1.3）の判定に何を使うか。** 供給関数は同期（`Fn(&World, &MenuContext) -> MenuItem`）で、SHIORI リソース `homeurl` は非同期の照会。選択肢: ⒜ `descript.txt` の `homeurl` だけで灰色を決める（SHIORI にだけ書いたゴーストは灰色になる＝要件 1.9 の順と食い違う）／⒝ 定常到達（`KanadeNotice::Steady`）のたびに UI から `homeurl` を 1 回照会して World に控え、供給関数はその写しと `descript.txt` を見る（照会の返事待ちの取り出しは `Input` の段の系で・`menu/trigger.rs` の `PendingQuery` と同型）／⒞ 灰色にせず、選ばれてから解いて無ければ飛ばす（要件 1.10）＝要件 1.3 を「走っている間だけ灰色」に縮める。推奨は ⒝（要件を変えない・kanade に触らない）。⒞ なら要件 1.3 を改める。
2. **議題 B: argv で始めたゴースト（`GhostDecision.folder == None`）の読み直し。** `request_ghost_switch` はフォルダ名で目録を引くので、根の外のゴーストは読み直せない（`NotFound`・降ろさない）。選択肢: ⒜ argv のゴーストは読み直さず `warn!` 1 件（開発者向けの起動形＝既知の制限に書く）／⒝ `GhostSpec` に `Dir(PathBuf)` を足し `resolve_switch_target` を広げる（`ghost_switch.rs` 870 行に触る・完了 `ghost-shell-balloon-switch` の再検証）。推奨は ⒜。要件 5.2 に「argv で始めたゴーストを除く」の一言を足すかどうかだけ決める。
3. **議題 C: 終了で待つ「書く段」の置き方（要件 7.1〜7.3）。** ⒜ `run` の全体を書く段にする（取得の途中でも上限 3 秒まで待つ・上限で打ち切って進む）／⒝ 最後の照合の通知（`Md5Compared{matched: true}` の最終件）で書く段へ入る（取得の途中は待たずに進む＝`exit_wait_abandoned`）。⒝ は「確定に入る」の通知をエンジンに足さなくても `Progress` の並び（`Md5Compared` の後が確定）に頼れば書けるが、並びに頼る弱さは brief の (d) と同じ。⒜ は単純で、待ちの上限は今日の 3 秒のまま。推奨は ⒜。どちらでも要件の文言は動かないが、7.3 の `warn!` の本文（「作業場所が残っているかもしれない」）は ⒜ のとき「取得の途中で断った」も含む。
4. **議題 D: `\![updateother,--shell=名]` で `menu,hidden` のシェルを引けるか。** `list_shells` は隠しシェルを落とす（`catalog.rs`・`// ukadoc: …descript_shell.html#menu_2chidden:1`）。⒜ 隠しシェルは引けない（メニューに出ないものは更新もできない・`warn!` で飛ばす）／⒝ 名前引き専用に隠しも含めて走査する読み手を足す。推奨は ⒜（正典は隠しシェルの更新に触れておらず、SSP の挙動を測らない方針）。要件 1.7 に「隠しシェルは引かない」を足すか、§8 に記すか。
5. **E: 送り先の取り方（要件 2.3・5.7）。** ⒜ 依頼を受けた時点の `Sender<KanadeMsg>` を背景スレッドが持ち、切り替わったら送出の `Err`／返事の `Dropped` で「居なくなった」と分かる／⒝ インストールと同じく毎回 UI の窓口へ頼み、窓口が `SwitchInFlight`／`FirstExit` を見て落とす。観測できる差は 0。⒜ は短いが、「走っている間に切替が始まって、まだ古い kanade が止まっていない」数百 ms の間は古いゴーストへ届く（`raise_event_not_steady` で kanade が捨てる＝送られない）。設計で決める。
6. **F: `OnUpdateProcessExec` をどこから送るか（要件 1.12）。** メニューの動作（UI スレッド）は返事を待てないので、背景スレッドの手続きの最初の段として「出どころがメニューのときだけ」送るのが素直。`Script` なら手続きを捨てて `info!`（総括 0 件）。設計で決める。
7. **G: エンジンの後片付けの区別の形（要件 5.9）。** 印のファイル方式か `old/` の改名方式か（§2 の 5.9 の行）。`work.rs` の中で閉じるので設計で決める。テストは `work_tests.rs` の兄弟へ。
8. **H: `\![execute,install,url]` の取得のスレッドと一時フォルダ（要件 6.5・6.6）。** 取得は更新の背景スレッドにもインストールの背景スレッドにも乗せず、取得のたびに短命のスレッドを起こす（`install-pick` と同じ形）か、更新の背景スレッドに乗せる（1 度に 1 本の直列に入る）か。一時フォルダは `%TEMP%\areka\` の下。設計で決める。

## 6. Research Needed（設計へ持ち越す確かめ）

1. **https の実走**（要件 9.16・裁定 4）: `WinHttpFetch` は `WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY`・同期。emo2 の `homeurl,https://ekicyou.github.io/ghost_dev/emo2/emo2/` に `updates.txt` と差分 1 件が置けるかは開発者の手。置けないときはローカル http で `winhttp_real_tests.rs` と同じ手順。
2. **ゴーストを生かしたまま確定する実機の挙動**（要件 5.1・裁定 1）: 写像中の DLL の改名は較正済みだが、SHIORI が排他で開いているファイルが定義ファイルに載っているときの `CommitWrite`→戻しの経路は実機で 1 度見る（emo2 の `updates.txt` の中身と、pasta が開きっぱなしにするファイルの有無）。
3. **読み直しの後のシェル・バルーンの解き直し**（要件 5.2 の「3 つの中身がすべて読み直される」）: `boot_into` は `resolve_balloon_for_ghost` でバルーンを解き直し、`reopen_ghost_windows`＋`boot_ghost_strict` でシェルを読み直す。実物で `ghost_switch_done` の後に新しい中身が効いていることを見る（実機の項目 ⑴）。
4. **`updates.txt` の根（`homeurl` 直下）と対象フォルダの対応**: emo2 は書庫の根と `ghost/master/` の両方に `updates.txt` を持つ。ゴーストの対象は `<根>/ghost/<folder>/`（`updates.txt` の行は `ghost/master/…`・`shell/master/…`）で、シェルの対象は `<ghost>/shell/master/`（`homeurl` 無し＝飛ばす）。ゴーストの更新がシェルの中身も入れ替える（SSP と同じ）ことを実機で確かめる。
5. **`exit_wait_timeout` の本文**: 「`<根>/.nar-work/` の下に残っているかもしれない」はインストール固有。更新の門を足すとき本文を門の名前で分けるか一般化するか（`exit_wait.rs`・小）。

## 7. 設計へ渡す推奨

- 案 C（`update/` 新設・`install/` と同型・エンジンの後片付けを先頭のタスクへ）。
- 窓口はプロセスに 1 つの NonSend（読み直しで World の中身が入れ替わるため）。門は `register_gate(world, "update", …)` を `ghost_session::register_systems` から 1 回。
- 手続きは純粋（口の trait）＋背景スレッド 1 本。イベントは背景スレッドから `RaiseEvent { reply: Some }` で送って返事を待つ（`Progress` の同期の閉包の中で順序が保たれる）。総括の返事を受けたら `request_ghost_switch(GhostSpec::Folder(今のフォルダ), raise_event: false, origin: Automatic)` を UI へ頼む（待ちは kanade の `pending_change`）。
- `useorigin1`・`homeurl` は背景スレッドから `ResourceQuery` で 1 回ずつ（`ReplyReceiver::recv`）。
- 失敗理由の表は `FailReason`／`FetchError` の網羅 `match`（ワイルドカード無し）。
- 台帳の状態は「本体から `areka-update` を辿れるようにした変更」と同じコミットで動かし、`cargo run -p ukadoc-survey -- evidence` と `cargo test -p ukadoc-survey` で証拠と数を確かめる。
