# ギャップ分析: areka-P0-ghost-install

> 測定は **2026-09-28**・ブランチ `claude/areka-p0-ghost-install-bd7d9c`・HEAD `f2c22c65`。`git diff --stat 10a8d724 HEAD -- crates` は空（ソースは完了 `session-mark-residue` の着地から不変）。
> コードは「何の定義か」（関数名・型名・定数名＋ファイル）で指す。行数は `wc -l` の実測。0 件の検索は、同じ検索が当たる語で先に確かめてから書いた（各所に較正を併記）。
> 本書は分析と選択肢を並べるもので、決定は書かない。決めるのは要件ディスカッションと設計。

## 1. 分析の要約

- **部品は揃っているが、繋ぐ線が 1 本も無い。** `areka-nar`（読む・検査する・入れる）、切替の入口、汎用の通知の入口、メニューの枠、告知の部品、`lastinstalled` の受け皿は在る。本体から `areka-nar` への依存は 0、3 つの入口は 0、インストール系のイベントの送り手は 0。
- **要件が挙げた「brief に無い事実」4 件は、実物で 4 件とも確かめられた**（`supplement` の宛先・下敷きの写し・応えの有無が戻らない・書庫の中を読む口が無い）。加えて本分析で **新しく 4 件**見つかった: ⑴ 置換語 `%lastghostname`／`%lastobjectname` は `areka-sylphya` の語彙表に「未対応」として登記されており、brief の「触るファイル」に無いクレートへ手が入る ⑵ UI 側に「ゴーストが今定常か」を持つ状態が無い ⑶ 窓が 0 枚の間は OS の終了の受け手が 1 つも居ない ⑷ 定常に入った直後は起動の挨拶が再生中でありうる（後から送った知らせの返事が挨拶を置き換える）。
- **要件 12 裁定 1 の代償のうち「`ghost_switch_tests.rs` の分割が先に 1 手要る」は実物と合わない。** `switch_to` を試すテストはそのファイルにほぼ無く（該当 1 行）、新しいテストは新しい兄弟ファイルへ置けるので、分割は必須ではない。代わりに、降ろしてから起こすまでの間に展開を挟むには `switch_to` を前半と後半に割る必要があり、その間は窓もゴーストも無いフレームが回る（今日は 1 フレームも回らない）。
- **応えの有無を返す道は、kanade の `msg.rs` と `actor.rs` の 2 ファイルに手を入れる形が最小**（`schedule/mod.rs`・`schedule/change.rs`・`steady.rs` は触らずに済む）。kanade を 1 行も触らない代替は、別の入力の返事と取り違えうるので成り立たない。
- **規模の見立ては 20〜26 タスクで、上限 20 を超える公算が高い。** 書庫でない物の投げ込み（要件 9）だけを切り出しても減るのは 1〜2。減らすなら切り出しの単位を選び直す必要がある（§4 g）。

## 2. 現状の調査

### 2.1 brief の数の引き直し

| 対象 | brief（09-28 節） | 実測 | 判定 |
|---|---|---|---|
| `crates/areka/src/main.rs` | 927 | 927 | 一致 |
| `crates/areka/src/ghost_session.rs` | 691 | 691 | 一致 |
| `crates/areka/src/session_end.rs` | 163 | 163 | 一致 |
| `crates/areka/src/emo2_boot/ghost_switch.rs` | 866 | 866 | 一致 |
| `crates/areka/src/emo2_boot/ghost_switch_tests.rs` | 987 | 987 | 一致（残り 13 行） |
| `crates/areka/src/emo2_boot/spine.rs` | 993 | 993 | 一致（残り 7 行） |
| `crates/areka/src/alert.rs` | 191 | 191 | 一致 |
| `crates/areka/src/emo2_boot/mod.rs` | 805 | 805 | 一致 |
| `crates/areka/src/emo2_boot/consumer_ledger.rs` | 756 | 756 | 一致 |
| `crates/areka/src/menu/mod.rs` | 337 | 337 | 一致 |
| `crates/areka/src/input_events/mod.rs` | 545 | 545 | 一致 |
| `crates/areka/src/placement/spawn.rs` | 769 | 769 | 一致 |
| `crates/areka-ghost/src/catalog.rs` | 324 | 324 | 一致 |
| `crates/areka/Cargo.toml` | 72 | 72 | 一致 |
| `crates/areka/src/app_exit.rs` | 291 | 291 | 一致 |
| kanade `schedule/events.rs` | 535 | 535 | 一致 |
| kanade `schedule/mod.rs` | 830 | 830 | 一致 |
| wintf `ecs/window_proc/mod.rs` | 332 | 332 | 一致 |
| wintf `ecs/window/components.rs` | 348 | 348 | 一致 |
| `dist/README.txt` | 103 | 103 | 一致 |
| `session_end_deadline_tests.rs`／`main_session_mark_tests.rs` | 523／938 | 523／938 | 一致 |

brief に数が無く、本分析で測ったもの: kanade `msg.rs` **880**・`actor.rs` **589**・`actor_tests.rs` **970**・`schedule/change.rs` **426**・`schedule/steady.rs` **935**・`schedule/schedule_tests.rs` **867**・`change.rs`（語彙）**103**／`areka-nar` の `lib.rs` **254**・`install.rs` **551**・`plan.rs` **273**・`error.rs` **328**・`manifest.rs` **420**／切替のテスト `ghost_switch_fallback_tests.rs` **222**・`ghost_switch_notice_tests.rs` **308**・`ghost_switch_test_support.rs` **398**・`frame_ghost_quit_switch_tests.rs` **285**・`session_end_tests.rs` **412**。

行数の上限の見張りは `crates/log-capture-kit/tests/file_length_guard_test.rs`（除外の一覧を持つ）。

### 2.2 0 件の検索（較正つき）

| 検索 | 範囲 | 結果 | 較正 |
|---|---|---|---|
| `WM_DROPFILES`／`DragAcceptFiles`／`DragQueryFile`／`GetOpenFileName`／`IDropTarget`／`WS_EX_ACCEPTFILES` | `crates/**/*.rs` | `crates/pilot/` の 2 ファイルに 17 件、**それ以外は 0 件** | 同じ範囲で `WM_ENDSESSION` は 8 ファイル 29 件 |
| `RaiseEvent` を送る本番コード（`KanadeMsg::RaiseEvent` を組み立てる箇所） | `crates/**/*.rs` | **0 件**。在るのは型の定義（`msg.rs`）・名札の match（`msg.rs`）・`actor.rs` の写し・`schedule/mod.rs`／`change.rs` の受け手・テスト 2 ファイル（`raise_event_tests.rs` は `Input::RaiseEvent` を組み、`schedule_variant_tests.rs` は名札を引く） | 同じ語の総数は 11 行 |
| `// ukadoc:` | `crates/areka-nar/src/` | **0 件** | `crates/**/*.rs` 全体では 35 ファイル 236 件 |
| `OnInstall`／`OnFileDrop`／`OnDirectoryDrop`／`OnGhostTerms` | `crates/**/*.rs` | `crates/areka-nar/src/` の 6 ファイル 7 件だけ（うち 1 件はテスト `install_commit_tests.rs`）。**送る側は 0 件** | 要件の「6 ファイル」と一致 |
| `areka-nar` への依存 | 全 `Cargo.toml` | `crates/sample-ghost-kit/Cargo.toml` の 1 件だけ。**`crates/areka/Cargo.toml` は 0 件** | `areka-nar` 自身の `name =` の行が当たる |
| `Win32_UI_Controls_Dialogs` | 全 `Cargo.toml` | **0 件** | 根の `Cargo.toml` に `Win32_UI_Controls`・`Win32_UI_Shell`・`Win32_System_Com` が当たる |
| `HWND_MESSAGE`（メッセージだけを受ける隠し窓） | `crates/areka/src`・`crates/wintf/src` の本番コード | **0 件**（当たるのは `session_end_sync_send_tests.rs` のテストだけ）。窓を作るのは外部クレート `wintf-winmsg-executor` 越しなので、この 0 は「areka と wintf が自分で隠し窓を作っていない」までを言う | 同テストで 4 件当たる |

### 2.3 数え直した値

- kanade の許可表 `ALLOWED_EVENT_IDS`（`schedule/events.rs`）: **13 語**（`OnInitialize`・`OnFirstBoot`・`OnBoot`・`basewareversion`・`OnSecondChange`・`OnClose`・`OnMouseMove`・`OnMouseDoubleClick`・`OnChoiceSelectEx`・`OnChoiceSelect`・`OnChoiceTimeout`・`OnGhostChanging`・`OnGhostChanged`）。本仕様の 10 語は **0 語**。`events_change_tests.rs` の判定は `assert_eq!(ALLOWED_EVENT_IDS.len(), 13)`。
- 拒否の語彙 `RefuseReason`（`crates/areka-nar/src/error.rs`）: **14 種**（`CorruptArchive`・`IntegrityMismatch`・`UnsupportedEntry`・`NameUndecodable`・`PathTooLong`・`SymlinkEntry`・`UnsafePath`・`CaseCollision`・`MissingInstallTxt`・`UnsupportedType`・`MissingRequiredKey`・`InvalidDirectoryName`・`CompanionSourceMissing`・`TargetGhostMissing`）。1 つの宣言（`refuse_reasons!`）から `kind()` と `ALL_KINDS` を作るので、種類が増えると網羅の match がビルドを止める形にできる（要件 5.2 の「増えたらビルドが止まる」は成り立つ）。要件 5.2 の表は `UnsupportedType` を欄 `found` の有無で 2 語に分ける＝欄を見る match で書ける。
- 消費者台帳 `canonical()`（`consumer_ledger.rs`）の登記: **9 行**。`("execute", Some("install"))` は **0 行**。
- 網羅台帳 `assets.toml` の `descript_install`: 全 15 行のうち `owner = "areka-P0-nar-install"` が **11 行**（全部 `status = "absent"`）。残り 4 行は `network-update` 1・owner 空 3。
- 網羅台帳の本仕様の語: `shiori.toml` の 10 イベント＋`OnInstallReroute` は全部 `absent`・owner 空。`sakura-script.toml` の `\![execute,install,path,…]` は `absent`・owner 空。`assets.toml` の `install.accept` は `absent`・owner 空。`%lastghostname`／`%lastobjectname` は `vocabulary-only`・owner 空。`ghostinstallbutton.caption` は `vocabulary-only`・owner `areka-P0-popup-menu-minimal`。
- `doc/ukadoc-coverage/roadmap-draft.md` の `[briefs].count`: **36**、`[[spec]]` の行も **36**。本仕様の行は **0**。
- `dist/README.txt` の「インストール」: **2 行**（「・シェル・バルーンの切り替え、インストール、ネットワーク更新の項目は、今の版ではメニューに出ません。」と「・α 版の時点では、次のことはできません: シェル・バルーンの切り替え、.nar ファイルからのインストール、ネットワーク更新。」）。

### 2.4 使える既存の部品と型

| 部品 | 場所 | 本仕様での使い道 |
|---|---|---|
| `NarArchive::open`／`manifest()`／`install(&InstallRequest)` | `crates/areka-nar/src/lib.rs` | 手続きの中身。`open` は 1 バイトも書かない。`install` は計画・組み上げ・確定を 1 回の呼び出しで行う |
| `InstallOutcome.installed: Vec<InstalledElement { kind, name, path, target_ghost, existing }>` | `areka-nar` の `install.rs`／`error.rs` | `OnInstallCompleteEx` の Reference0〜2 の材料。並びは本体が先・同梱バルーンが後（`plan.rs` の `build_plan`）＝要件 2.4 の並びと同じ |
| `NarError::Io { phase, rolled_back, survivors, .. }`／`SurvivingTree { destination, path }` | `areka-nar` の `error.rs` | 要件 5.5・5.6 の記録の材料 |
| `request_ghost_switch`・`SwitchInFlight`・`switch_to`・`switch_to_default`・`record_last_installed` | `crates/areka/src/emo2_boot/ghost_switch.rs` | 入れた後の切替・受け皿への記録・起こし直し |
| `KanadeMsg::RaiseEvent { id, references, method }` | `crates/areka-kanade/src/msg.rs` | イベントの送出（応えは戻らない） |
| `KanadeMsg::ResourceQuery { ids, reply }`＋`actor_resources::answer` | kanade `msg.rs`・`actor_resources.rs` | 「返信端を同梱して返事を受ける」前例 |
| `ChangeCueSink`・`ChangeRx`・`register_change_drain`・`drain_change_requests` | `emo2_boot/change_cue.rs`・`ghost_switch.rs` | 台本の受け口と UI 側の取り出しの前例（117 行＋取り出しの系） |
| `menu::register(world, Frame::Install, 供給)`・`menu/ghost_frame.rs` | `crates/areka/src/menu/` | メニューの登記の前例（73 行）。呼び出しは `ghost_session.rs` の `boot_wired` |
| `OnSessionEnd(pub fn(&mut World, Entity))`・`lifecycle::WM_ENDSESSION` | wintf `ecs/window/components.rs`・`ecs/window_proc/lifecycle.rs` | 「窓に関数を差す部品」と振り分けの腕の前例 |
| `alert::raise`・`alert::suppressed`・`NO_ALERT_ENV` | `crates/areka/src/alert.rs` | 利用条件の画面の置き場（`MB_OK` のみ・戻りを見ない） |
| `areka_parsers::charset::decode(bytes, DefaultEncoding::Ansi)` | `crates/areka-parsers/src/charset/` | 利用条件の本文の復号（1 行目の `charset,` の先読みと BOM の読み飛ばしを持つ）。`install.txt` も同じ関数で読んでいる |
| `catalog::sakura_name`・`catalog::companion_balloon` | `crates/areka-ghost/src/catalog.rs` | `accept` の照合相手の半分と、`install.accept` の読み手の型の前例 |
| `boot_resolve::LastUsed { ghost, balloon, shell_folder }.record(publisher)`・`record_last_used` | `crates/areka/src/main.rs` ほか | 要件 6.6（最後に使ったバルーンの記憶の書き換え） |
| `WaitBudget { started, limit }`・`SESSION_END_SHIORI_LIMIT`（3 秒） | kanade `shiori/probe.rs`・`crates/areka/src/session_end.rs` | 終了で待つ上限の数え方 |
| `sample_ghost_kit::{NarBuilder, EntryBuilder, install_txt, fold_tree}` | `crates/sample-ghost-kit/src/nar_writer.rs` | テストの中で書庫を組む（areka は dev 依存に持っている） |
| `CharWindowMarker { scope }`・`BalloonWindowMarker { scope }` | `crates/areka/src/placement/spawn.rs` | 落とされた窓のスコープ番号（`app_exit::on_ghost_os_close` が同じ読み方をしている） |

## 3. 要件と資産の対応

凡例: **在る**＝そのまま使える／**足りない**＝新しく作る／**制約**＝既存の形が縛る／**未確認**＝設計で確かめる。

| 要件 | 使える資産 | ギャップ |
|---|---|---|
| 1.1・1.9・1.11 手続き 1 本・待ち行列・背景 | `std::thread::spawn` の前例（`emo2_boot/spine.rs`）・`mpsc` | **足りない**: 手続き・待ち行列・背景スレッド・UI への戻りの受け口（新規 `install.rs`） |
| 1.2 窓への投げ込み | 先進坑の結果（`WS_EX_ACCEPTFILES` だけで届く）・`window_style()` | **足りない**: wintf の `WM_DROPFILES` の腕・受け手の関数・窓に差す部品／`window_style()` に 1 ビット／窓への装着（`app_exit::attach_os_close_request` と同じ形） |
| 1.3・1.4 メニューとファイル選択 | `Frame::Install`・`ghostinstallbutton.caption`・`menu::register` | **足りない**: `menu/install_frame.rs`・ファイル選択。**制約**: `MenuAction` は `Fn(&mut World, &MenuContext)`＝World を借りたまま呼ばれるので、選ぶ画面は別スレッドで出す |
| 1.5〜1.7 台本 | `ChangeCueSink` と同じ骨格・消費者台帳 | **足りない**: `install_cue.rs`・台帳の 1 行・`CommandConsumer` の変種 1 つ・取り出しの系 |
| 1.8 起こすたびに登記 | `boot_wired` の `menu::ghost_frame::register(world)` の隣 | 1 行 |
| 1.10・2.10 定常まで待つ | `KanadeNotice::Steady`（kanade `schedule/boot.rs` が起動の完了で 1 回出す） | **足りない**: UI 側に「今定常か」を持つ状態が無い（`ghost_switch::on_notice` は迎え入れの予約を下ろすだけ）。**制約**: `on_raise_event` は定常以外を捨てる |
| 2.1〜2.5・2.8・2.9・2.11〜2.13 イベント | `RaiseEvent`・`events::raise`・許可表 | 許可表に 10 行・数の判定を 13 → 23。Reference の組み立ては新規 |
| 2.6・2.7・6.2・6.3 応えの有無 | なし | **足りない**: 応えを送り手へ返す道（§4 c） |
| 3.1〜3.7 `accept` | `manifest().accept`・`catalog::sakura_name` | **足りない**: `install.accept` の読み手（`catalog.rs` に 1 関数・`Identity` は変えない） |
| 4.1〜4.11 利用条件 | `charset::decode`・`alert.rs` | **足りない**: 書庫の中を読む口（§4 d）・はい／いいえを返す口・上限で切る処理 |
| 5.1〜5.8 失敗 | `RefuseReason`・`NarError::Io`・`survivors` | 写しの表は新規（材料は全部在る）。`areka-nar` は `open`／`install` の失敗で `error!` を 1 件出す（`log_failure`）＝要件 5.5 の `error!` は areka 側でもう 1 件足す形になる（設計で 2 件の役割を書く） |
| 6.1 受け皿 | `record_last_installed` | 呼ぶだけ。`#[allow(dead_code)]` と注釈の 2 行を消す |
| 6.2 入れた後の切替 | `request_ghost_switch(SwitchRequest { ghost: GhostSpec::Folder(..), raise_event: true, origin: ChangeOrigin::Automatic })` | 呼ぶだけ（背景スレッドから UI へ頼む線は要る）。入口は切替の途中なら要求を無視する（`SwitchVerdict::Busy`）ので、無視されたときの扱いを設計で決める |
| 6.6 バルーンの記憶 | `record_last_used`／`LastUsed::record` | 呼ぶだけ（`BalloonDecision` を組む） |
| 6.8・6.9 置換語 | `areka_sakura::sysvar::resolve_system_var`（凍結した表に値があれば置き換える）・`SylphyaPublisher::publish_static` | **足りない**／**未確認**: §4' の 1 |
| 7.1〜7.7 降ろして入れて起こす | `switch_to`・`take_down`・`boot_into`・kanade の「保留の切替」 | **足りない**: §4 e |
| 7.8 シェル・バルーンは降ろさない | `commit_one`（使用中なら宛先は無傷で失敗） | **未確認**: 表示中のシェル・使用中のバルーンのフォルダを areka 自身が掴んでいるか |
| 7.9・8.7 印の判定を変えない | `session_mark_verdict(first, argv_session, logsink_fallback, end: Teardown)` | 変えずに済む（§4 f） |
| 8.1〜8.10 終了で待つ | `WaitBudget`・`end_session_within`・`fn main` の後始末 | **足りない**: §4 f |
| 9.1〜9.9 書庫でない物 | 窓のスコープ番号の印 | **足りない**: 振り分け・MIME の表・`OnFileDrop2`／`OnDirectoryDrop` の Reference |
| 10.2〜10.5 台帳・文書 | 生成器（`cargo run -p ukadoc-survey -- report`／`report-summary`） | 台帳 3 本・`roadmap-draft.md`（36 → 37）・README 2 行・`doc/COMPAT_ARCHITECTURE.md` の「8. 沈黙ルール対応表」 |
| 10.6 外部クレート 0・環境変数 0 | — | 成り立つ（足すのは `areka-nar` の依存と `windows` の機能 1 つ） |
| 10.7 1,000 行 | — | **制約**: §2.1 の残りの行数 |
| 11 テスト | `NarBuilder`・偽の SHIORI（`ghost_switch_test_support.rs`） | `spine.rs`（993）・`actor_tests.rs`（970）・`ghost_switch_tests.rs`（987）・`main_session_mark_tests.rs`（938）には足さず、新しい兄弟ファイルへ置く |

## 4. 要件の段階で挙がった 7 点の検証

### a. `supplement` は必ず起動中のゴーストのフォルダを宛先にする — **確認**

- `crates/areka-nar/src/plan.rs` の `body_placement` は `InstallKind::Supplement` の宛先を `ghost_store.join(&target)`（＝`<根>/ghost/<宛先ゴースト>/`）にする。`directory` は使わない。`InstallKind::Shell` は `<根>/ghost/<宛先ゴースト>/shell/<directory>/`。
- 宛先ゴーストは `existing_target_ghost` が `InstallRequest.target_ghost` から取り、渡されない・1 階層の名前でない・根に実在しない、のどれでも `RefuseReason::TargetGhostMissing` を返す。
- areka は 1 度に 1 体なので、`accept` が一致する相手は起動中のゴーストだけ（要件 3.5）。したがって **「起動中のゴーストと同じ宛先は断る」を採ると、入れられる `supplement` は 0 本**になる。要件の読みは正しい。
- 補足: `supplement` は `refresh,1` が書かれていても必ず重ね置き（`manifest.rs` の `body_existing` が読み飛ばし、`ManifestWarning::RefreshIgnoredForSupplement` を残す）。重ね置きの下敷きはゴーストのフォルダ丸ごと（次の b）。

### b. 組み上げは宛先の今の中身を下敷きに写す — **確認**（ただし呼び手に見える切れ目は 1 つ）

- `crates/areka-nar/src/install.rs` の `stage_placement` は、宛先がフォルダとして在れば `copy_existing` で作業フォルダへ写してから（`ExistingPolicy::Overlay` は丸ごと・`Replace { keep }` は除外マスクの名前だけ）書庫の中身を重ねる。確定は `commit_one` の「宛先 → `old-<k>`」「組み上げた木 → 宛先」の 2 手。
- 写した後に宛先へ書かれた物（ゴーストが降りるときの保存）は、入れ替えで古い写しに置き換わる。要件 7.3 の心配は実物どおり。
- **ただし公開面に「組み上げだけ」「確定だけ」の口は無い。** `NarArchive::install` が `build_plan` → `WorkArea::create` → `stage_placement` → `commit_all` を 1 回の呼び出しで通す（`lib.rs` の `place`）。呼び手が守る規則は 1 つで足りる: **`install` を呼ぶのは、ゴーストを降ろし終えた後**。`open`・`manifest()`・`accept` の照合・利用条件は降ろす前に済ませられる（どれも宛先を読まない）。
- `areka-nar` に手を入れる必要は、この点については **0**。
- 副作用 1 件: `shell`／`supplement` の宛先の実在の検査（`existing_target_ghost`）は `install` の中で走るので、areka が先に自分で確かめない限り、`TargetGhostMissing` が分かるのはゴーストを降ろした後になる（宛先は起動中のゴースト自身なので、実在しないのは起動中に消された場合だけ）。

### c. 汎用の通知の入口は応えを送り手へ返さない — **確認**

**実物**

- `KanadeMsg::RaiseEvent`（`msg.rs`）の欄は `id`・`references`・`method` の 3 つ。返信端は無い。
- `actor.rs` の受信ループは `Input::RaiseEvent` へ写して `drive` を回す。`drive` は `step` が返した `Action::ShioriRequest` を実行し、応答を `Input::ShioriReply { outcome, origin }` として運行表へ入れ直す。応答が台本なら定常の応答の腕が `Action::StartTalk` を出す。**外へ出るのは再生の指示だけで、送り手へ戻る値は 0**。
- 運行の通知 `KanadeNotice`（`crates/areka-kanade/src/change.rs`）は `Steady`・`ChangeCancelled { reason }`・`Stopped(KanadeStopped)` の **3 種**。応えの有無を運ぶ変種は 0。
- 捨てられた場合（許可表に無い・定常でない）も送り手には何も戻らない（`on_raise_event` が `warn!` を残すだけ）。
- 「返信端を同梱して返事を受ける」前例は在る: `KanadeMsg::ResourceQuery { ids, reply: ReplySender<..> }`。運行表を通さず `actor.rs` がその場で答える。

**選択肢**

| 案 | 触る kanade のファイル | 規模 | 長所 | 短所 |
|---|---|---|---|---|
| **c-1** `RaiseEvent` に返信端の欄（`Option<ReplySender<結果>>`）を足し、`actor.rs` の `drive` が最初の往復の結果（送らなかった／台本あり／返事なし／失敗）を控えて返す | `msg.rs`（欄 1 つ＋結果の型）・`actor.rs`（写しの腕と `drive` の控え）・`lib.rs`（再公開）。**`schedule/mod.rs`・`schedule/change.rs`・`steady.rs` は 0 行**（`Input::RaiseEvent` は今の 3 欄のまま） | 本番 40〜60 行＋新しいテストファイル 1 本 | 送った本人だけに返る（取り違えが起きない）。捨てられたことも分かる。`KanadeMsg::RaiseEvent` を組み立てる既存コードは本番 0・テスト 0 なので、欄を足しても書き換える呼び手は `actor.rs` の 1 か所だけ | brief が「触らない」に挙げた 2 ファイルに手が入る。`actor_tests.rs` は 970 行なのでテストは新しいファイルへ |
| **c-2** `KanadeNotice` に変種を足し、運行表が応答を受けた所で通知する | `change.rs`（語彙）・`schedule/steady.rs`（935 行）か `schedule/mod.rs`・`actor.rs` は不変 | 本番 30〜50 行＋areka 側の受け手（`ghost_switch::on_notice` は網羅の match）＋既存テストの追随 | 既存の 1 本の線に乗る | 上限が近い `steady.rs` に手が入る。通知は名前でしか引けず、同じ名前を続けて送ると区別できない。areka 側の網羅の match（`ghost_switch::on_notice`）に腕が要る |
| **c-3** kanade を触らず、areka が再生の開始を見て推し量る | 0 | — | kanade 不変 | **成り立たない**: 再生の指示（`StartTalk`）は出どころを運ばないので、同じ間に届いた `OnSecondChange`・マウスの返事と区別できない。「返事なし」は何も起きないことなので、待つ時間を決めるしかなくなる |
| **c-4** 応えの有無を使わない（要件を変える） | 0 | — | 作業 0 | 正典の「このイベントが無かった場合 OnInstallComplete が発生」と裁定 5 を満たせない |

- c-1 と c-2 のどちらでも、**「空の台本」を返事なしと読む判断**（裁定 6）は areka 側か kanade の殻に 1 か所要る。今日の実物では、SHIORI が値つきで返せば `ShioriOutcome::Value(String)` になり、空文字列でも台本として再生の指示が出る（`shiori/real.rs` の写しは `Ok(Some(value))` → `Value`）。
- c-1 では背景スレッドが返事を待って止まる。kanade が止まれば返信端が落ちて待ちは解ける（`ReplyError::Dropped`）ので、終了で固まらない。

### d. `areka-nar` に書庫の中のファイルを読む口が無い — **確認**

- 公開の口は `NarArchive::open`・`manifest()`・`install()` の 3 つ（`lib.rs`）。ほかに公開されているのは型と `crc32` だけ。
- 伸長済みの中身は `NarArchive` が既に持っている（私有の欄 `names: Vec<EntryName>`・`contents: Vec<Vec<u8>>`）。`EntryName` は `path`（`/` 区切り・正規化済み）と `is_dir` を持つので、最上位の 1 ファイルを名前（ASCII の大小を区別しない）で引いてバイト列を返す口は、**10〜20 行の追加**で足りる（読み直しも伸長のやり直しも無い）。`locate_install_txt`（`manifest.rs`）が同じ引き方をしている。
- 形の候補: ⑴ 名前を受けて最上位のファイルを返す汎用の口 ⑵ 利用条件だけを返す専用の口（`terms.txt` を先に・無ければ `terms.md`）。⑴ は後続（インストール時の `readme` など）にも使えるが、`areka-nar` の公開面が「書庫の中を自由に読める」形へ広がる。⑵ は正典の 2 つの名前を `areka-nar` が知ることになる。
- 利用条件のファイルは、受諾の後は普通のファイルとして宛先へ置かれる（`plan.rs` の本体の配置は最上位のファイルを除かない。除くのは `supplement` の `install.txt` だけ）。

### e. 起動中のゴーストを上書きする — 3 案の比較

**今日の切替の道筋（実物）**

1. `request_ghost_switch` が予約 `SwitchInFlight { target, prev, stage: SendOff }` を立て、kanade へ `KanadeMsg::ChangeGhost` を送る。
2. kanade は、再生中なら要求を保留してトークの完了で始める（`change.rs` の `on_change_ghost`／`consume_pending`）。`raise_event` が偽なら `OnGhostChanging` も `OnClose` も送らず SHIORI を降ろして止まる（`begin_change`）。
3. 停止通知が届くと `on_ghost_stopped` が `switch_to` を呼ぶ。`switch_to` は **1 回の同期の呼び出しの中で** `take_down`（期限なし・UI スレッドで待つ）→ `write_switch_drop` → `close_windows_for_restart` → `boot_into` を通す。新しい窓は次の `Input` 段で生える。
4. 起こしたゴーストの `KanadeNotice::Steady` で予約を下ろす。

要件 7.1 の「再生中の台詞が終わってから降ろす」は 2 の保留で既に満たせる。要件 7.2 の「`OnGhostChanging` も `OnClose` も送らない」は `raise_event: false` の切替と同じ。要件 7.7（途中の切替要求は無視）は予約の有無の判定のまま。

**案 (a) 降ろしてから入れて起こし直す**

`install` は降ろした後・起こす前に走らせる（b）。形は 2 通り。

| | (a-1) 間を空ける（展開は背景スレッド） | (a-2) 間を空けない（展開を UI スレッドで同期に） |
|---|---|---|
| `ghost_switch.rs` の変更 | `switch_to` を「降ろして全窓を閉じる前半」と「起こす後半」に割る。`SwitchStage` に段を 1 つ足し、閉じた証 `WindowsClosed`・切替の中身・仕事を予約に持たせる。背景スレッドの結果を受けて後半を呼ぶ受け口が要る。失敗の枝（元の中身で起こし直す／既定へ戻す／既定自身なら致命）は `switch_to_default`・`fatal` を使える | `switch_to` の `take_down` と `boot_into` の間に、予約が持つ仕事を 1 回呼ぶ口を足す（10〜20 行） |
| 予約の型 | `SwitchInFlight` に欄が増える。組み立てる箇所は本番 1・テスト 3（`session_end_tests.rs`・`frame_ghost_quit_switch_tests.rs`・`ghost_switch_fallback_tests.rs`） | 同左 |
| 入口の型 `SwitchRequest` | 変えない形にできる（入口を通した直後に予約へ仕事を載せる）。組み立てる箇所は 5（本番 2＝`drain_change_requests`・`menu/ghost_frame.rs`／テスト 3）なので、変えても波及は小さい。ただし形を変えると完了 `ghost-shell-balloon-switch` の見直しの引き金 | 同左 |
| 要件 1.11（展開の間も止めない） | 満たす | **満たさない**（展開の間 UI スレッドが止まる。窓は 0 枚なので描画・台詞・メニューは元々無いが、メッセージは配られない） |
| 窓もゴーストも無いフレーム | **回る**（今日は 0 フレーム）。前のゴーストの結線（`Emo2Wiring`・`MouseWiring` など）は次に起こすまで World に残る＝系が止まった相手へ送ろうとしないかを確かめる必要がある | 回らない |
| OS の終了 | 間の最中は OS の終了の受け手が居ない（下の「新しく見えた点」） | 同じ（今日の切替も同じ）。間が長くなるぶん当たりやすくなる |
| 行数 | `ghost_switch.rs` は 866 行。前半・後半・受け口を足すと上限に近づくので、新しい兄弟ファイルへ出す（`take_down`・`boot_into`・`set_stage` は今は私有＝見える範囲を広げる） | 収まる |

**案 (b) 先に既定ゴーストへ切り替えてから入れる**

- `switch_to` は 0 行。既存の入口を 2 回呼ぶ（起動中 → 既定、入れた後に既定 → 入れたゴースト）。
- 起動中のゴーストが既定ゴースト自身（`boot_resolve::DEFAULT_GHOST_FOLDER`）のときは切り替える先が無い＝この場合だけ別の道が要る（要件 12.1 の指摘どおり）。
- 起こしが 2 回・既定ゴーストの挨拶が 1 回挟まる。`supplement`／上書きの締めの知らせを誰へ送るか（既定ゴーストか、戻った後の元のゴーストか）を決める必要がある。
- 「既定の定常を待ってから入れる」状態を持つのは (a-1) と同じ（定常の通知を手続きへ届ける線は、どの案でも要る＝要件 1.10・2.10）。

**案 (c) 断る**

- 切替の道筋は 0 行。入れられる `supplement` は 0 本（a）。起動中のゴーストの新しい版を `.nar` で入れることもできない。シェルとバルーンは宛先がフォルダの中なので入る。

**テストファイルの分割の要否（要件 12 裁定 1 の「代償」の検証）**

- `ghost_switch_tests.rs`（987 行・テスト 27 本）の中身は、純粋な突き合わせ・特別な名前の解決・入口・取り出しの系の 4 群。`switch_to`／`on_ghost_stopped`／`SwitchStage`／予約の組み立てに触れる行は **1 行**。
- `switch_to` と停止通知の振り分けを試しているのは `ghost_switch_fallback_tests.rs`（222）・`ghost_switch_notice_tests.rs`（308）・`frame_ghost_quit_switch_tests.rs`（285）・`session_end_tests.rs`（412）・`ghost_session_switch_memory_tests.rs`（339）。どれも余裕がある。
- `ghost_switch.rs` は既に 3 つのテストファイルを `#[path]` で持つ。4 つ目を足すのは 3 行。
- したがって **分割は必須ではない**（0 手にできる）。分割が要るのは、`ghost_switch_tests.rs` の既存のテストを 13 行を超えて書き足す場合だけ。分割するなら、`ghost-change-name-resolution` が足した 2 群（合わせて約 430 行）を新しいファイルへ移し、共用の道具（先頭の約 125 行）を `ghost_switch_test_support.rs`（398 行）へ寄せる機械的な 1 手。

**新しく見えた点（どの案にも効く）**

- `OnSessionEnd` を差すのは `app_exit::attach_os_close_request` で、相手は `GhostWindowMarker` を持つ窓だけ。`close_windows_for_restart` はその窓を全部消す。areka と wintf の本番コードが自分で作る隠し窓は 0（§2.2）。**窓が 0 枚の間に OS の終了が来ると、後始末（`end_session_within`）を呼ぶ受け手が居ない。** 今日の切替では窓が 0 枚なのは同期の呼び出しの中だけだが、展開を挟むと長くなる。
- 起こし直した直後の `KanadeNotice::Steady` は起動の完了の合図で、起動の挨拶が再生中でも出る（kanade `schedule/boot.rs` は `BootVersion { talk }` をそのまま `Steady { talk }` へ移して通知する）。そこへ締めの知らせを送ると、返事の台本が挨拶を置き換える（`events::value_replaces_active_talk` は `OnSecondChange` 以外を置き換える）。同じことは `OnInstallBegin` の返事と `OnInstallCompleteEx` の返事の間でも起きる（展開が速いと、始まりの台詞が終わりの台詞に置き換わる）。要件 7.1 は降ろす前の待ちだけを定めている。

### f. 終了で展開の終わりを待つ — **印の判定の署名を変えずにできる**

**実物**

- `fn main` の後始末は `after_run`（置き場の単位と印の材料を取り出す）→ `finish_after_run` の閉包（`session.shutdown(CloseReason::User { scope: 0 })` → `settle_session_mark(mark, Teardown { run_ok, down_ok, shiori_cut: false })` → 告知 → 性能の最終報告）。`shutdown` に期限は無い。
- `end_session_within(world, limit)` は冒頭で `started = Instant::now()` を取り、予約を下ろし、`run_ghost_quit_phase` → `quit_app(ExitOrigin::SessionEnd)` → `shutdown_within(CloseReason::System, WaitBudget { started, limit })` → `settle_session_mark`。
- `session_mark_verdict` の引数は `(first, argv_session, logsink_fallback, end: Teardown)`、`Teardown` の欄は `run_ok`・`down_ok`・`shiori_cut` の 3 つ。

**乗せ方**

- 手続きの背景スレッドとゴーストの降ろしは別のスレッドなので、**並んで進む**。降ろしが戻った後に、同じ `WaitBudget` の残りだけ手続きを待てば、合計は上限を超えない（要件 8.2 の「足し算にしない」）。`fn main` の側は後始末に入った時点から 3 秒を数える。
- 待つかどうかは「今 `NarArchive::install` の中に居るか」で決まる（要件 8.4）。手続きの側に「段」と「終了が始まった」の 2 つを 1 つの鍵の下で持たせ、終了が始まった後は `install` へ入らない形にする（鍵が無いと、終了の判定と `install` への入りがすれ違う）。
- 待った結果（間に合った・上限に達した）は `warn!` に残すだけで、`Teardown` にも `MarkInputs` にも欄を足さない＝**印を残す理由は 0 個増**（要件 7.9・8.7）。
- 触るのは `main.rs`（`after_run` か閉包に数行・927 行）と `session_end.rs`（数行・163 行）。テストは新しい兄弟ファイルへ。
- 後続 `network-update` が同じ口に乗れる形（要件 8.10）にするなら、口は「背景の仕事 1 本の、今書いている最中かと終わりの合図」を返す小さな型にして `install.rs` の外へ置く選択肢がある。

**裁定 4 の「終了の指示の後は確定の段へ入らない」を採る場合**

- 組み上げと確定の間で止める口が `areka-nar` に要る。`InstallRequest` に欄を足すと、組み立てる箇所 **50**（10 ファイル。テスト 47・`sample-ghost-kit/src/devroot.rs` 1・example `fold-samples.rs` 1・`areka-nar/src/lib.rs` の説明の中の例 1）を書き換える。`install` の隣に「続けてよいかを尋ねる関数を受ける口」を足す形なら、既存の呼び手の書き換えは 0。
- 採らない場合、終了の指示の時点で組み上げの途中なら、そのまま確定まで進んで 3 秒の内に終わるか、上限で断たれる。組み上げの途中で断たれるのは無害（宛先は無傷）、確定の 2 手の間で断たれると宛先のフォルダは無く元の中身は `old-<k>` に残る（7 日）。

### g. 規模の見立てと切り出し

| # | 仕事 | 見立て（タスク） |
|---|---|---|
| 1 | 本体から `areka-nar` へ繋ぐ＋`// ukadoc:` の行＋`descript_install` 11 行（要件 10.2 は同じコミット）＋書庫の中を読む口 | 1〜2 |
| 2 | kanade の許可表 13 → 23 と数の判定 | 1 |
| 3 | 応えの有無を返す道（c-1）とテスト | 1〜2 |
| 4 | `install.accept` の読み手 | 0.5〜1 |
| 5 | 手続きの純粋な部分（依頼と結果の型・`accept` の照合・失敗理由の表・Reference の組み立て） | 2 |
| 6 | 背景スレッド・待ち行列・定常の合図・イベントの送出（Ex → 旧仕様 → All） | 2 |
| 7 | 利用条件（読む・復号・上限で切る・はい／いいえ・抑止） | 1〜2 |
| 8 | 台本の受け口と台帳と取り出し | 1 |
| 9 | メニューの登記とファイル選択 | 1〜2 |
| 10 | 投げ込み: wintf の腕と部品 1〜2／窓の様式・装着・振り分け 1／`OnFileDrop2`・`OnDirectoryDrop`・MIME 1 | 3〜4 |
| 11 | 起動中のゴーストへ入れる一周と失敗の枝 | 2 |
| 12 | 入れた後（受け皿・切替・バルーンの記憶・置換語 2 つ） | 1〜2 |
| 13 | 終了で待つ口と 2 か所の呼び出し | 1〜2 |
| 14 | 台帳 3 本・生成物・README 2 行・`COMPAT_ARCHITECTURE.md` | 1 |
| 15 | 実機確認 | 1 |
| | **合計** | **20〜26** |

- 要件 12 裁定 16 の見立て（17〜20）との差は、3（+1〜2・要件は織り込み済み）・11（割る形は 1 ではなく 2）・12 の置換語（brief の一覧に無いクレート）・1 の読む口。
- **要件 9 だけを切り出しても減るのは 1〜2**（10 の 3 行目と振り分けの一部）。wintf の腕・窓の様式・装着は書庫の投げ込み（要件 1.2）に要るので残る。
- 切り出しの単位の候補と、減る見立て:

| 候補 | 減る | 本仕様に残るもの・失うもの |
|---|---|---|
| ㋐ 書庫でない物の投げ込み（要件 9） | 1〜2 | 書庫の投げ込みは残る。切った先は wintf の部品・`ghost_session.rs`・許可表・台帳を共有するので直列 |
| ㋑ 投げ込みの全部（要件 1.2・9・wintf） | 3〜4 | 入口はメニューと台本の 2 つになる。α の一周「`.nar` を窓へ落とす」は切った先が着地するまで成り立たない |
| ㋒ 起動中のゴーストへ入れる一周（要件 7） | 2 | 切った先が着地するまで `supplement` と起動中のゴーストの上書きは断る（理由の語は要件で決める） |
| ㋓ 終了で待つ（要件 8） | 1〜2 | 着地までは「展開の最中に終えると元のフォルダが消えうる」が既知の制限になる。`network-update` と同じ口なので、そちらと束ねる余地がある |
| ㋔ 置換語 2 つ（要件 6.8・6.9） | 0.5〜1 | 今日どおり置き換えない（`%lastghostname` が台詞にそのまま出る） |

## 4'. 本分析で新しく見つかった点

1. **置換語は `areka-sylphya` に「未対応」として登記されている。** `crates/areka-sylphya/src/vocab/flat.rs` の `FLAT_VOCAB` は `lastghostname`・`lastobjectname` を `BackingLayer::RuntimeState`・`M1Status::Degraded`・`DegradePolicy::PassThroughRaw` で持ち、同ファイルのテストが `lastghostname` の状態を `Degraded` と判定している。置き換えそのものは `areka_sakura::sysvar::resolve_system_var` が、トークの開始で凍結した表に値があれば行う（表に無ければ `%名前` のまま）。値を表へ載せる道は `SylphyaPublisher::publish_static` が在るが、これは「静的な構成」の層の口で、語彙表が宣言している層（運行の状態）の口は無い。さらに値はプロセスで 1 つなのに、載せる先（`GhostRuntime::sylphya_publisher()`）はゴーストの実行系ごとに在り、起こすたびに新しくなるので、**ゴーストを起こすたびに載せ直す**必要がある（入れた直後に切り替えた先のゴーストで引けるように）。brief の「触るファイル」に `areka-sylphya`・`areka-ghost` は無い。
2. **UI 側に「今定常か」の状態が無い。** 定常の合図は `KanadeNotice::Steady` の 1 回きりで、`ghost_switch::on_notice` は迎え入れの予約を下ろすのに使うだけ。要件 1.10・2.10・7.1・7.4・7.5 は、どれも手続きが「定常に入った」を知る必要がある。合図を受ける所（`on_notice`、または `emo2_boot/frame.rs` の `run_ghost_quit_phase`）に数行と、降ろす・起こすで下ろす処理が要る。
3. **窓が 0 枚の間は OS の終了の受け手が居ない**（§4 e）。
4. **定常の合図の時点では起動の挨拶が再生中でありうる**（§4 e）。

## 5. 実装方針の選択肢

### 案 A: 既存の部品を広げる

- 手続きを切替の道筋の中へ織り込む（`ghost_switch.rs` に入れる前後の処理を書き、`alert.rs` に利用条件の画面を足し、`input_events/mod.rs` に振り分けを書く）。
- ✅ 新しいファイルが少ない。
- ❌ `ghost_switch.rs`（866）・`main.rs`（927）・`emo2_boot/mod.rs`（805）が上限に近づく。切替の道筋とインストールの判断が 1 つのファイルに混ざり、後続 `shell-balloon-switch`・`network-update` と同じ行を触る。

### 案 B: 新しい部品として建てる

- 手続き・待ち行列・背景スレッド・終了で待つ口・利用条件・振り分けを新しいファイル（`install.rs`・`terms.rs`・`emo2_boot/install_cue.rs`・`menu/install_frame.rs`・`input_events/drop.rs`・wintf `ecs/window_proc/drop.rs`）に置き、既存のファイルには呼び出しの 1〜数行だけを足す。起動中のゴーストへ入れる一周は切替の道筋に頼らず、自前で降ろして起こす。
- ✅ 判断の分かれ目が純粋な関数に集まり、テストが書きやすい。
- ❌ 降ろす・起こすを自前で持つと、印の扱い・既定へ戻す・致命の経路を二重に持つことになる（完了 `ghost-shell-balloon-switch` の申し送り「終了や起こし直しの経路を足すときは印の判定を必ず通す」に当たる）。

### 案 C: 組み合わせ

- 新しい判断は新しいファイルへ（案 B）。降ろす・起こす・既定へ戻す・致命は切替の道筋の関数を使い（案 A）、`switch_to` には前半と後半に割るための最小の口だけを足す。kanade は c-1 の 2 ファイル、`areka-nar` は読む口 1 つ（と、採るなら確定の前で止める口 1 つ）。
- ✅ 既存の保証（印・既定へ戻す・致命）を 1 か所のまま使える。上限の近いファイルへの追加が小さい。
- ❌ 切替の道筋に「展開の待ち」の段が入り、切替の状態が 1 つ増える。完了 spec の見直しの引き金に当たるかの判断が要る。

## 6. 規模とリスク

- **規模: L**（1〜2 週間）。理由: 触るクレートが 6（`areka`・`areka-nar`・`areka-kanade`・`areka-ghost`・`wintf`・置換語を含めるなら `areka-sylphya`）、新しいスレッドとモーダルの画面、切替・終了の 2 つの既存の道筋への合流。brief の規模 M より大きい。
- **リスク: 中〜高**。理由: 既知の型の組み合わせが多い一方、⑴ 窓もゴーストも無いフレーム ⑵ 窓が 0 枚の間の OS の終了 ⑶ 背景スレッドと終了のすれ違い、の 3 つは今日の実物に前例が無い。利用者のフォルダを書き換えるので、失敗したときに失う物が大きい。
- 並走する `frame-phases-after-exit` が触るのは `crates/areka/src/emo2_boot/frame.rs` とその兄弟テスト。本仕様が定常の合図を `frame.rs` の `run_ghost_quit_phase` で受ける形を採ると、共有するソースが 0 ではなくなる（`ghost_switch::on_notice` で受ける形なら 0 のまま）。

## 7. 設計へ持ち越す調査

1. **窓が 0 枚の間の OS の終了**: 受け手の居ない間に `WM_ENDSESSION` が来たときに何が起きるか（プロセスがそのまま終わらされるか）と、手当ての形（展開の間は窓を閉じない／受け手だけの窓を持つ／間を空けない）。
2. **窓もゴーストも無いフレーム**: 降ろした後に残る前のゴーストの結線（`Emo2Wiring`・`MouseWiring`・`BalloonWiring`・メニューの結線など）の下で、毎フレームの系が `error!`／`warn!` を出さないか。降ろす時点で結線を外す必要があるか。
3. **空の台本の届き方**: 32bit の補助プロセスの経路で、値が空の 200 応答が `Value("")` と `NoContent` のどちらで届くか。
4. **使用中のフォルダ**: 表示中のシェル・使用中のバルーンのフォルダの中のファイルを、areka 自身（絵の読み込み）か SHIORI が開いたままにしているか（要件 7.8 の成否）。
5. **置換語の載せ方**: `publish_static` で載せた値が、語彙表の層の宣言（運行の状態）と食い違っても読めるか。読めるなら語彙表の 2 行の状態をどう書くか。
6. **ファイルを選ぶ画面**: `GetOpenFileNameW`（`crates/areka/Cargo.toml` に `Win32_UI_Controls_Dialogs` を 1 行）と、`IFileOpenDialog`（機能 `Win32_UI_Shell`・`Win32_System_Com` は根で有効済み・別スレッドを STA で初期化する）のどちらにするか。背景スレッドで出したとき前面に出るか。
7. **利用条件の画面**: `MessageBoxW` の `MB_YESNO` は閉じるボタンが効かない形になる（要件 4.4 と合う）。本文の長さの上限（画面に収まる行数）と、持ち主の窓なしで前面に出す旗。
8. **記録の 2 件**: `areka-nar` が出す `error!`（`log_failure`）と、要件 5.5 の areka 側の `error!` の役割の分け方（同じ失敗が 2 回に見えないように）。
9. **MIME の表**: 拡張子から決める表の出どころと広さ（要件 9.4）。

## 8. 要件ディスカッションへ出す判断項目

1. **応えの有無を返す道の形**（§4 c）: c-1（返信端を足す・kanade の `msg.rs` と `actor.rs` に手が入る）か c-2（通知の変種を足す）か。どちらも brief の「触らない」を外す。
2. **起動中のゴーストへ入れる形**（§4 e）: (a-1) 間を空ける／(a-2) 間を空けない（要件 1.11 を「ゴーストが降りている間は除く」と読み替える）／(b) 既定へ切り替えてから／(c) 断る。(a-1) を採るなら、窓が 0 枚の間の OS の終了をどう扱うかを併せて決める。
3. **要件 12 裁定 1 の「代償」の書き直し**: テストファイルの分割は必須ではない（§4 e）。代償は「`switch_to` を割る・切替の段が 1 つ増える・窓もゴーストも無いフレームが回る」へ改める。
4. **規模が上限を超える見立てへの対応**（§4 g）: 切り出さずに 20 を超えることを認めるか、㋐〜㋔ のどれを切るか。要件 9 だけを切っても 1〜2 しか減らない。
5. **置換語 2 つを本仕様に含めるか**（§4' 1）: 含めるなら触るクレートに `areka-sylphya`（と載せ直しの場所）が増える。
6. **台詞の置き換え**（§4 e）: 始まりの知らせの返事が終わりの知らせの返事に置き換えられること、起こし直した直後の挨拶が締めの知らせの返事に置き換えられることを、そのままにするか、再生の終わりを待つか。待つなら「再生が終わった」を UI へ知らせる線が要る（今日は 0）。
7. **空の台本の判定の置き場**（§4 c）: kanade の殻で「返事なし」に写すか、areka の手続きで読むか。
8. **確定の前で止める口を `areka-nar` に足すか**（§4 f・裁定 4 の末尾）。
9. **書庫の中を読む口の形**（§4 d）: 名前を受ける汎用の口か、利用条件だけの専用の口か。
10. **締めの知らせの送り先が替わる場合**（案 (b) を採るとき・要件 2.13）: 既定ゴーストへ送るか、戻った後の元のゴーストへ送るか。
11. **終了で待つ口の置き場**（§4 f・要件 8.10）: `install.rs` の中か、`network-update` も使える独立の小さな型か。

## 9. requirements.md・brief.md の記述と実物の食い違い

| 記述 | 実物 | 影響 |
|---|---|---|
| 要件 12 裁定 1「テストファイル `ghost_switch_tests.rs`（987 行）の分割が先に 1 手要る」 | `switch_to` に触れるテストはそのファイルに 1 行。新しいテストは新しい兄弟ファイルへ置ける | 分割は 0 手にできる（§4 e） |
| brief 09-28 節「触らない: kanade `msg.rs`／`actor.rs`／…」 | 応えの有無を返すには c-1 で 2 ファイル、c-2 で 2〜3 ファイル | 要件の「brief に無い事実 3」のとおり。`schedule/mod.rs` は c-1 なら 0 行 |
| brief「`areka-nar/src/*`（`// ukadoc:` の行）」 | 書庫の中を読む口が要る | 要件の「brief に無い事実 4」のとおり |
| brief の「触るファイル」 | 置換語は `areka-sylphya`（語彙表とそのテスト）に掛かる。定常の合図の受け手（`ghost_switch.rs` の `on_notice` か `frame.rs`）にも数行 | 一覧に 2〜3 ファイル増える |
| brief 09-26 節「`menu::register` は `#[allow(dead_code)]`」 | 今は `menu/ghost_frame.rs` が呼んでいる。`#[allow(dead_code)]` が残るのは `unregister` だけ | 影響なし（古い記述） |
| brief 09-27 節の README の引用「ゴースト・シェル・バルーンの切り替え、インストール、…」 | 今の行は「シェル・バルーンの切り替え、インストール、…」 | 消す語は同じ。行は 2 行のまま |
| 要件 Introduction「`OnInstall`…の綴りは `crates/areka-nar/src/` の 6 ファイル（説明と失敗の語彙）」 | 6 ファイルのうち 1 つはテスト（`install_commit_tests.rs`） | 影響なし |
| brief Constraints「規模 M」 | 見立ては L（§6） | 見積もりの前提が変わる |
