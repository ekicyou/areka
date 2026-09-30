# 設計レビュー: areka-P0-network-update

> 2026-09-29 実施（`/kiro-validate-design`・非対話・サブエージェント）。対象は `design.md`（2026-09-29 生成）・`requirements.md`（要件 10 の裁定は確定済み）・`research.md` §1〜§8・`brief.md`・steering。設計が根拠にしている既存コードの記述は、下の「照合した事実」のとおり Grep／Read で実物と突き合わせた。行番号ではなく「何の定義か」で指す。

## レビュー要約

設計は `install/` と同型の `update/`（背景スレッド 1 本＋UI 側の窓口）を採り、既存の部品（汎用の通知の入口と返事・リソースの照会・切替の入口の保留・終了で待つ門・作業場所の後片付け）を組み合わせるだけで要件 1〜9 を満たす道筋を、関数名・型名の単位で示している。設計が挙げた既存コードの事実は照合した範囲ですべて実物と一致し、存在しない関数や型を前提にした箇所は見つからなかった。残る懸念は、`OnUpdateProcessExec` に台本で応えたゴーストが自分で `\![updatebymyself]` を出す正典どおりの使い方が「実行中」として断られ得る点（設計の局所的な直しで済む）と、終了で待つ「書く段」を `run` 全体にした代償の確認である。

## 照合した事実（設計の主張 → 実物）

| 設計の主張 | 実物 | 判定 |
|---|---|---|
| `exit_wait.rs` の `WorkGate { begin, enter_write, leave_write, end, is_closing }`・`register_gate(world, name, gate, on_close: fn(&mut World))`・`begin_close`・`ClosingWaits::wait(WaitBudget)`・`EXIT_WAIT_LIMIT`＝3 秒・本文に `<根>/.nar-work/` | `crates/areka/src/exit_wait.rs` に同名で在る。本文は `exit_wait_timeout` の 1 か所 | 一致 |
| `ghost_switch::on_notice` の `KanadeNotice::Steady` の腕が末尾で `install::desk::on_steady(world)` を呼ぶ | `crates/areka/src/emo2_boot/ghost_switch.rs` の `on_notice` | 一致 |
| `KanadeNotice::Steady` は起動の完了で 1 回だけ出る（定常到達ごとの `homeurl` 照会＝起動直後の GET 1 件） | 送出は `crates/areka-kanade/src/schedule/boot.rs` の 1 か所だけ | 一致 |
| `request_ghost_switch(world, SwitchRequest { ghost: GhostSpec::Folder, raise_event, origin })`・`SwitchVerdict { Accepted, Busy, NotFound, NoContext }`・`SwitchInFlight`・`switch_to` が `take_down` → `close_windows_for_restart` → `install::desk::run_overwrite_between` → `boot_into` | 同ファイルに同名で在る。`ChangeOrigin::Automatic` は `crates/areka-kanade/src/change.rs` | 一致 |
| `KanadeMsg::RaiseEvent { id, references, method, reply: Option<ReplySender<RaiseOutcome>> }`・`RaiseOutcome` は `NotAllowed`・`NotSteady`・`Script`・`NoReply`・`Failed` の 5 値・返事は `drive` の後に 1 回 | `crates/areka-kanade/src/msg.rs`・`change.rs`・`actor.rs`（`drive` の戻りの後で `send_raise_reply`） | 一致 |
| `KanadeMsg::ResourceQuery { ids: Vec<&'static str>, reply }`・`actor_resources::answer` が運行表を経ずに答え、定常でなければ全件 `NoContent` | `msg.rs`・`actor_resources.rs` | 一致 |
| `on_change_ghost` は `Steady { talk: Some }` で `pending_change` に控え `consume_pending` で始める・`raise_event: false` は `CloseSilent` | `crates/areka-kanade/src/schedule/change.rs` | 一致 |
| `ReplyReceiver::recv()`・`try_recv(&self)`・`ReplyError::Dropped` | `crates/areka-actor/src/reply.rs` | 一致 |
| `Sender<KanadeMsg>` は `std::sync::mpsc`・`GhostSession::kanade()` が返す | `crates/areka/src/ghost_session.rs` | 一致 |
| `work.rs`: `WorkArea::create` が `sweep` → `old/` に中身が残るフォルダは `residue`・`cleanup` は `remove_dir_all` できなければ `leftovers`・`keep`・印の仕組み 0・記録 0 | `crates/areka-update/src/work.rs`（132 行） | 一致 |
| `lib.rs` の `run` は `RollbackFailed` のときだけ `keep`、それ以外の失敗と成功で `cleanup` | `crates/areka-update/src/lib.rs` の `run` と `walk` の末尾 | 一致（印を `cleanup` の失敗点に置く判断は成り立つ） |
| `FailReason` 11 変種（`fail_reasons!`・`ALL_KINDS`）・`FetchError` 8 変種・`UpdateError { homeurl, target, stage, reason, leftovers, work }`・`file()`・`rolled_back()` | `crates/areka-update/src/error.rs` | 一致 |
| `Progress` 6 変種・`UpdateOutcome::Updated { placed, undeletable, leftovers, .. }` | `crates/areka-update/src/outcome.rs` | 一致 |
| `WinHttpFetch::new() -> Result<_, FetchError>`・記録 0・`MAX_BODY_BYTES`・`Fetch::get(&self, url)` | `winhttp.rs`・`fetch.rs` | 一致 |
| 「写像中の DLL でも確定の改名は通る」の較正 | `commit_tests.rs` の `calibrate_pin_lets_rename_through_and_refuses_remove`（`testkit::pin` は `LoadLibraryExW(LOAD_LIBRARY_AS_IMAGE_RESOURCE)`＝本当に写像する） | 一致 |
| `install::judge::script_request(&[&str]) -> Result<PathBuf, ScriptRefusal>`・`ScriptRefusal { NotPath, Empty, Relative }`・`SEPARATOR`＝byte 値 1・`file_drop.rs` が同じ借り方 | `judge.rs`・`input_events/file_drop.rs`（`send_event` は `reply: None`） | 一致 |
| `InstallCueSink { tx: Sender<RawInstallRequest> }`・`InstallOrigin::Script`・`install_cue_unsupported`・`install_cue_extra_ignored` | `emo2_boot/install_cue.rs` | 一致 |
| `install/procedure.rs` の口は `&mut dyn InstallPorts`（更新の口を `&self` にする理由の対照） | `run_order(&InstallOrder, &mut dyn InstallPorts)` | 一致 |
| 消費者台帳は 10 行・`try_register`・`canonical_builds_without_duplicate` | `consumer_ledger.rs` | 一致 |
| `ALLOWED_EVENT_IDS` 23 語（テストが `len() == 23` を直書き）・`ALLOWED_RESOURCE_IDS` 10 語（テストが 10 語の固定表）・`updatebutton.caption` は既に在る | `schedule/events.rs`・`events_change_tests.rs`・`schedule/resources.rs` | 一致 |
| `Frame::ORDER` の 4 番目が `Update`・`MenuItem { label, caption_resource, enabled, .. }`・`captions::default_label`・`captions::send_query`・`install_frame::register` は `boot_wired` から | `menu/mod.rs`・`captions.rs`・`install_frame.rs`・`ghost_session.rs` | 一致 |
| `BootContext.current.ghost.folder: Option<String>`・`current.balloon.dir`・`runtime().mount().shell.dir`・`catalog::list_shells` が `menu,hidden` を落とす・`Identity` に `homeurl` 無し・`read_descript` は私有 | `boot_config.rs`・`boot_resolve.rs`・`areka-ghost/src/runtime.rs`・`catalog.rs` | 一致 |
| `main.rs` と `session_end.rs` が `begin_close` → `closing.wait(WaitBudget)` を同じ出発点で呼ぶ | 両ファイル | 一致 |

## 重要な問題（最大 3 件）

### 問題 1: `OnUpdateProcessExec` に台本で応えたゴーストが自分で `\![updatebymyself]` を出すと、「実行中」として断られ得る

- **懸念**: 決めたこと 3 と 5 の組み合わせ。メニューから始めた要求は `submit` の時点で `busy` を立て、背景スレッドが `OnUpdateProcessExec` を送る。ゴーストが台本を返す（`Script`）と背景スレッドは要求を捨てて `OrderDone` を送るが、その台本の中の `\![updatebymyself]`（正典の「更新処理をカスタマイズできる」の典型＝確認の台詞や前処理のあとに自分で更新を始める）は台本の再生が始まった瞬間に受け口から `raw_tx` へ届き得る。窓口の `drain` は（`install/desk.rs` と同じく）生の要求を先に、背景スレッドの頼みを後に捌くので、同じ tick に両方が届くと `busy` がまだ真のまま `submit` が `Executing` を返し、ゴーストへ `OnUpdateFailure(executing)` が飛ぶ。台詞を挟む台本なら `OrderDone` が先に着くので実害は出にくいが、時機に依る（決定論でない）。
- **影響**: 正典が示す `OnUpdateProcessExec` の使い方（応えて自分で更新を始める）が、たまに「二重起動」と誤って断られる。利用者から見ると「メニューを選んだのに更新が始まらず、失敗の台詞が出る」。要件 1.12・1.15 のどちらにも反しない形で起きるので、テストで固定しなければ実機でも気付きにくい。
- **提案**（どれも窓口の中の小さな直し・アーキテクチャは変えない）: ⒜ `busy` を「標準の手続きが実際に始まった」時点で立てる——背景スレッドが `OnUpdateProcessExec` の答えを見た後（`NoReply` なら `DeskAsk::Started`、`Script` なら `OrderDone`）に窓口へ知らせ、それまでの要求は断らず背景スレッドの待ち行列（`Sender<UpdateJob>` の mpsc）へ渡す。1 度に 1 本は背景スレッドが直列に回すことで保たれる。⒝ 最低限として `drain` の順を「頼み（`OrderDone`）→ 生の要求」に入れ替える（同じ tick の競合だけ解消・別 tick の競合は残る）。⒜ を推奨。あわせて `desk_tests.rs` に「`Script` の答えの直後に台本から届いた要求が断られない」判定を 1 つ足す（要件 9.6・9.7 の境目）。
- **Traceability**: 要件 1.12・1.14・1.15・10.5・10.9、要件 9.6・9.7
- **Evidence**: design.md「設計で決めたこと」3・5、「依頼と受付（`update/mod.rs`）」の `submit` の判定の順、「窓口（`update/desk.rs`）」の `drain` の説明、「手続き（`update/procedure.rs`）」の `run_order` の順 ⑵

### 問題 2: 終了で待つ「書く段」が `run` 全体なので、取得の途中で終了すると必ず上限 3 秒まで止まる

- **懸念**: 決めたこと 13 は `enter_write` を `run` の直前に置く。`run` は定義ファイルの取得・各ファイルの取得・照合・確定を 1 回で行うので、取得（ネットの待ち・最長は受信の時間切れ 60 秒）の間に終了が始まると、まだ何も本番のフォルダへ書いていないのに `ClosingWaits::wait` が上限 3 秒まで待つ。OS の終了では SHIORI を降ろす待ちと同じ出発点なので残りの時間だけだが、メニューの終了では毎回 3 秒の待ちになる。設計はこれを承知で採っている（`Progress` の並びに頼って段を分けるのは弱い）。正しさは崩れない（取得の途中の残りは `.update-work/new/` の下で次の `sweep` が黙って消す）。
- **影響**: 利用者から見える差は「更新の取得中に終了すると 3 秒遅れる」だけ。ただし要件 7.3 の記録 `exit_wait_timeout` が「書きかけの物が残っているかもしれない」と言うのに、実際は確定に入っていない場面が大半になり、記録が読み手を誤らせる。
- **提案**: 設計討議で「受け入れる」か「エンジンに `Progress::CommitBegin`（確定に入る）を 1 変種足し、その観測で `enter_write` する」かを決める。後者はエンジンの境界を `outcome.rs`・`lib.rs` の各 1 行ぶん広げるが、写しの `progress_events` は網羅の `match` なので腕が 1 つ増えるだけで、要件 9.1 の 4 経路のテストにも自然に乗る。前者なら `exit_wait_timeout` の本文（決めたこと 14）を「取得か確定の途中だった」と読める語にし、`label` に段（取得中／確定中）を載せる。
- **Traceability**: 要件 7.1・7.2・7.3、9.12
- **Evidence**: design.md「設計で決めたこと」13・14、「終了で待つ（要件 7）」の流れ図と本文、research.md §8.2「終了の門の『書く段』（議題 C）」

### 問題 3: 変更表に載っていない既存テストが 2 本あり、`script_request` の戻りの型変更で壊れる

- **懸念**: `judge::script_request` の戻りを `Result<PathBuf, _>` から `Result<ScriptRequest, _>` に広げる（決めたこと 17）と、`install/desk_pick_tests.rs`（`script_request(&["path", ABSOLUTE]).expect(..)` を `PathBuf` として使う）と `emo2_boot/install_cue_tests.rs` が追随を要する。また `exit_wait_tests.rs` は `exit_wait_timeout` の本文に `.nar-work` が含まれることを判定しており、決めたこと 14 で必ず赤になる（設計は「あれば追随」と書くが、実在する）。どれもビルドかテストで即座に見つかる種類で、設計の骨格には影響しない。
- **影響**: タスク生成の見積もりから漏れ、実装時に「既存テストを消さない・新しい振る舞いへ書き換える」（要件 9.14）の判断が場当たりになる。
- **提案**: design.md の「変更」の表に `install/desk_pick_tests.rs`・`emo2_boot/install_cue_tests.rs`・`exit_wait_tests.rs` の 3 行（各「戻りの型／本文に追随」）を足し、タスク生成で `judge` と `exit_wait` のタスクに含める。
- **Traceability**: 要件 6.7・7.3・9.14
- **Evidence**: design.md「File Structure Plan」の「変更」の表、「`exit_wait.rs` の本文」の節

## 設計の強み

1. **既存の仕組みだけで「台詞が終わってから読み直す」を得ている**（決めたこと 7）。返事が `drive` の後に返ることと、切替の要求が `Steady { talk: Some }` で保留されることを実物（`actor.rs`・`change.rs`）で確かめ、UI 側に台詞の終わりを数える線を足していない。`install/overwrite.rs` と同じ前提で、後続 `shell-balloon-switch` にも同じ形で渡せる。
2. **失敗の経路が値と記録で閉じている**。失敗理由の表は `FailReason` 11 種・`FetchError` 8 種を網羅の `match` で写し（増えればビルドが止まる）、`Raised::Closed`・`EngineRun`・`TargetEnd` の 5 値で「締めは 1 件だけ」「居なくなれば以後 0 件」「終了後は 0 件」を型の上で表している。テスト戦略は偽の口と `Progress` の台本でエンジンを呼ばずに 4 経路×2 種別を固定し、ネットへ出るテストが 0 である。

## 最終判定

**GO**（問題 1 を設計討議で解決してからタスク生成へ）

**理由**: 既存アーキテクチャとの整合（`install/` 同型・依存の向き・境界の宣言・外部クレート 0・メッセージボックス 0・1,000 行）に食い違いは無く、設計が根拠にした既存コードの記述は照合した全項目で実物と一致した。問題 1 は窓口の `busy` の立て方という局所の直しで、問題 2 は要件を満たした上での代償の確認、問題 3 は表の補いであり、どれも設計の骨格を変えない。

**次の一手**:
1. 設計討議で問題 1 の ⒜／⒝、問題 2 の「受け入れる／`Progress::CommitBegin` を足す」を決め、design.md の該当する「決めたこと」と Testing Strategy を改める。
2. 問題 3 の 3 行を「変更」の表へ足す。
3. `/kiro-spec-tasks areka-P0-network-update` でタスクを生成する（設計の申し送りどおり、`work.rs` の印を先頭の独立タスクにする）。
