# Requirements Document

> 本文の実測は **2026-09-29・本ブランチ**（main `2ec8df59`＝`file-drop` の完了 PR#201 のコミット）のもの。brief の最終の再測定（09-28・`10a8d724`）の後に `ghost-install`（PR#198）と `file-drop`（PR#201）が着地したので、許可表の数・汎用の通知の入口の形・依頼の型・終了で待つ口を引き直した。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない。
> 要件 10 の裁定は、brief の議題 6 件と、要件を書く途中で答えが要った点に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。開発者の確定は本文中に 0 件（brief の 09-29 節の「着手順の入れ替え」と「現シェル・現バルーンの読み」は指示として取り込んだ）。

## Project Description (Input)

**誰の何が困っているか**: ゴーストを入れた第三者。作者が辞書を直して配布サイトを更新しても、areka では受け取れない。ゴーストの作者が辞書に書いた `OnUpdateBegin`〜`OnUpdateComplete` の台詞は areka では 1 度も動かない。

**今の状態**: 更新のエンジン（`crates/areka-update`・完了 `areka-P0-update-engine`）は在る——定義ファイル `updates2.dau`／`updates.txt` の読み取り・MD5 の差分・WinHTTP による取得・一時フォルダからの全か無かの確定・`delete.txt` まで済んでいる。しかし本体 `areka` からは辿れず（`crates/areka/Cargo.toml` の依存に無い）、`homeurl` を読む所も、更新系のイベントを送る所も、メニューの「ネットワーク更新」の登記も、台本の入口（`\![updatebymyself]`・`\![update,…]`・`\![updateother,…]`・`\![execute,install,url,…]`）も 0 である。

**何を変えるか**: メニュー「ネットワーク更新」と台本の入口から、今のゴースト・今のシェル・今のバルーンを配布サイトから更新できるようにする。手順は正典のイベント列（`OnUpdateBegin` → `OnUpdateReady` → 各ファイルの `OnUpdate.OnDownloadBegin`／MD5 の照合 → `OnUpdateComplete` または `OnUpdateFailure` → 総括 `OnUpdateResult`）で、ゴースト以外は `OnUpdateOther*` の名で送る。更新の間もゴーストは消えずに進捗を話し、終わってから同じゴーストを読み直す。失敗はメッセージボックスではなくゴーストの台詞（イベント）と記録で伝える。`\![execute,install,url,URL,nar]` は URL から落として `ghost-install` の手続きへ渡す。

> 起票: 2026-09-18 `/kiro-discovery` 再入（棚卸⑭）。2026-09-20 にエンジン `update-engine` を切り出し（09-24 完了）、09-24・09-26・09-27・09-28 の棚卸で再測定した。2026-09-29 の開発者指示で `shell-balloon-switch` より先（B7）になった。

## Introduction

### 誰が困っているか

- **ゴーストを入れた第三者**: 作者が配布サイトを更新しても、手元のゴーストは古いまま。`.nar` を落とし直して入れ直すしかない。
- **ゴーストの作者**: 辞書に書いた更新の台詞（emo2 なら `ghost/master/dic/update.pasta` の `OnUpdateBegin`・`OnUpdateReady`・`OnUpdateFailure`・`OnUpdateComplete`）が areka では 1 度も呼ばれない。台本から `\![updatebymyself]` を出しても何も起きない。

### いま何が起きているか（2026-09-29 実測）

- **エンジンは在るが本体から辿れない。** `crates/areka-update/src/lib.rs` の `run(&UpdateRequest { homeurl, target }, &dyn Fetch, &mut dyn FnMut(&Progress)) -> Result<UpdateOutcome, UpdateError>` が唯一の入口で、同期に回りスレッドを起こさない。`crates/areka/Cargo.toml` の `[dependencies]` に `areka-update` は無く、ワークスペースの他のどのクレートも使っていない。
- **エンジンが知らせる進捗は 6 種**（`crates/areka-update/src/outcome.rs` の `Progress`）: `ManifestFetched { name }`・`DiffDecided { files }`・`DownloadBegin { file, index, total }`（`index` は 0 始まり）・`Md5Compared { file, expected, actual, matched }`・`Committed { placed }`・`Deleted { removed }`。成功は `UpdateOutcome::Unchanged { manifest }`／`Updated { manifest, placed, removed, undeletable, leftovers }`。失敗は `UpdateError { homeurl, target, stage, reason, leftovers, work }` で、原因のファイル名は `UpdateError::file()`（対象フォルダからの相対名・フォルダ単位の失敗では `None`）、短い語は `FailReason::kind()`（11 種: `TargetMissing`・`InvalidHomeurl`・`ManifestMissing`・`ManifestFetch`・`LocalUnreadable`・`WorkArea`・`FileFetch`・`Md5Mismatch`・`EscapesTarget`・`CommitWrite`・`RollbackFailed`）。取得の失敗は `FetchError`（`NotFound`・`Status { code }`・`NameResolution`・`Connect`・`Timeout`・`Tls`・`TooLarge { limit }`・`Other { code }`）。
- **取得の境界は `Fetch::get(&str) -> Result<Vec<u8>, FetchError>`**（`fetch.rs`）。本番の実装 `WinHttpFetch::new()`（`winhttp.rs`）は `Send`／`Sync` ではなく、作ったスレッドで使う。**失敗しても記録を出さない**（`winhttp.rs` に `tracing` の呼び出しは 0 件）。3xx の追随・404・500・日本語パスの符号化はエンジンの実機で確認済み、**https は未確認**（`winhttp_real_tests.rs` は `#[ignore]`・ローカルの http）。
- **定義ファイルの規則はエンジンが持つ。** `updates2.dau` を先に、無ければ（404）`updates.txt`（`file,` の行だけ・`charset,` の行は別に読む）。文字コードの既定は Shift_JIS（`manifest.rs` の `DEFAULT_CHARSET`・BOM が勝つ・知らない名前は警告して既定へ）。無効な行（MD5 なし・32 桁の 16 進でない・絶対パス・末尾 `/`・`..`・定義ファイル自身・作業場所の下）は捨てて `UpdateWarning` に出す。パーセント符号化は「全部が符号化済みなら復号、そうでなければ URL 側だけ符号化」。`delete.txt` → `delete1.txt`… の順で、絶対パス・`..`・対象の外へ出る行・作業場所の下は無視し、走行を失敗にはしない（`delete.rs`・`// ukadoc:` の行 1 つ）。`delete.rs` は `charset,` の行を特別扱いしない（emo2 の `delete.txt` の 1 行目 `charset,UTF-8` は「無い相対パス」として黙って読み飛ばされる＝害は無い）。
- **確定は 1 ファイルずつ 2 回の改名**（`commit.rs` の `commit`: 宛先 → `old/<rel>`、`fresh/<rel>` → 宛先）で、写像中の DLL でも通る（`commit_tests.rs` の較正）。**詰まるのは後片付けだけ**: `work.rs` の `WorkArea::cleanup` が `old/<dll>` を消せないと走行フォルダごと残り、次の走行の `sweep` は「`old/` に中身が残る＝戻せなかった走行」と読んで消さずに残骸に挙げ、`lib.rs` の `warn_leftover` が毎周 `warn!` を出す。印のファイルなどの区別の仕組みは 0。
- **kanade の汎用の通知の入口は在り、応えの有無も分かる。** `KanadeMsg::RaiseEvent { id, references, method, reply }`（`crates/areka-kanade/src/msg.rs`・`ghost-install` で `reply` が増えた）→ `schedule/change.rs` の `on_raise_event`。返事は `RaiseOutcome`（`NotAllowed`・`NotSteady`・`Script`・`NoReply`・`Failed`）で、台本そのものは返さず kanade が再生する（再生中の台詞は置き換わる）。許可表 `ALLOWED_EVENT_IDS`（`schedule/events.rs`）は **23 語**（元の 13＋インストール系 8＋投げ込み 2）で、`events_change_tests.rs` が `assert_eq!(ALLOWED_EVENT_IDS.len(), 23)` と直書きで判定している。`OnUpdate*` は 0 語。許可表に無い名前は `warn!(raise_event_not_allowed)`、定常以外は `warn!(raise_event_not_steady)` で捨てる（積まない）。
- **リソースの許可表は 10 語**（`schedule/resources.rs` の `ALLOWED_RESOURCE_IDS`＝`username`＋枠の項目名 7＋表示可否 2）。`homeurl`・`useorigin1`・`other_homeurl_override` は無い。`KanadeMsg::ResourceQuery { ids, reply }` に Reference を付ける欄は無い。
- **`homeurl` の読み手は 0。** `crates/areka-ghost/src/catalog.rs` に無い（`sakura_name`・`companion_balloon` が前例）。語彙としては `crates/areka-sylphya/src/vocab/shiori_resource.rs` に在るだけ。
- **メニューの枠は在るが登記が無い。** `crates/areka/src/menu/mod.rs` の `Frame::Update`、`captions.rs` の `("updatebutton.caption", Frame::Update, "ネットワーク更新")`。登記の前例は `menu/ghost_frame.rs`・`menu/install_frame.rs`（`super::register(world, Frame::…, …)`）で、呼び手は `crates/areka/src/ghost_session.rs` の `boot_wired`（起こすたびにやり直す）。`update_frame.rs` は無い。
- **台本の受け口。** `crates/areka/src/emo2_boot/consumer_ledger.rs` の `canonical()` に `updatebymyself`・`update`・`updateother` は無い。`("execute", Some("install"))` は `install_cue.rs`（`ghost-install`）が持ち、2 番目の引数が `path` でなければ `warn!(install_cue_unsupported)` で何もしない（`url` を含む）。
- **インストールの依頼の口は在る。** `crates/areka/src/install/mod.rs` の `InstallOrder { archives: Vec<PathBuf>, origin: InstallOrigin }`（`InstallOrigin` は `Menu`・`Script`・`WindowDrop`）と `submit(world, order) -> SubmitVerdict`。
- **終了で背景の仕事を待つ口は在る。** `crates/areka/src/exit_wait.rs` の `WorkGate`（`begin`・`enter_write`・`leave_write`・`end`）と `register_gate`。上限 `EXIT_WAIT_LIMIT`＝3 秒。この部品は `install` を知らず、「後続 `network-update` の背景の更新も同じ口で待てる」と書いてある。
- **読み直しの部品は在る。** `crates/areka/src/emo2_boot/ghost_switch.rs` の `request_ghost_switch(world, SwitchRequest { ghost: GhostSpec::Folder(今のフォルダ), raise_event: false, origin })`。自分自身への切替は除外されず（`ghost_session_switch_tests.rs` の `switch_to_self_takes_down_and_reboots_a`）、降ろして窓を閉じ、バルーンを解き直し（`resolve_balloon_for_ghost`）、窓を作り直して（`placement::prepare_ghost_windows`）起こす。2 度目の起動の根は `OnGhostChanged`（自分→自分・`schedule/boot.rs` の `boot_root`）で、`OnBoot` は送らない。
- **同期送信の見張り。** `crates/areka/src/session_end_sync_send_tests.rs` の `ALLOWED_SYNC_SENDS` は本番の `SendMessageW(`／`SendMessageTimeoutW(` を 2 件の例外表で固定している（背景スレッドから UI の窓へ同期で送る形は赤）。
- **配布物の文書。** `dist/README.txt` の 2 行（「シェル・バルーンの切り替え、ネットワーク更新の項目は、今の版ではメニューに出ません。」「α 版の時点では、次のことはできません: シェル・バルーンの切り替え、ネットワーク更新。」）が「ネットワーク更新」を「できないこと」に数えている。
- **網羅台帳。** `doc/ukadoc-coverage/ledger/shiori.toml` の `OnUpdate*` 26 行は全部 `status = "absent"`・`owner = ""`。`homeurl`・`useorigin1`・`other_homeurl_override` は `vocabulary-only`・owner 空。`updatebutton.caption` は `vocabulary-only`・`owner = "areka-P0-popup-menu-minimal"`。`sakura-script.toml` の `\![updatebymyself…]`・`\![update,更新対象…]`・`\![updateother,…]`・`\![execute,install,url,…]` は `absent`・owner 空。`assets.toml` の `descript_install` の「相対パス」（`delete.txt` の行の書式）は `absent`・`owner = "areka-P0-network-update"`、`descript_ghost`／`descript_shell`／`descript_balloon` の `homeurl` は `absent`・owner 空。
- **検体 emo2**（`vendors/sample_ghost/emo2.nar`）: `ghost/master/descript.txt` に `homeurl,https://ekicyou.github.io/ghost_dev/emo2/emo2/`。書庫の根と `ghost/master/` に `updates.txt`（`charset,UTF-8`・`file,<パス>\x01<MD5>\x01size=…\x01date=…`）、根に `delete.txt`。`shell/master/descript.txt` に `homeurl` は**無い**。同梱バルーン `emo2-kakukaku/descript.txt` の `homeurl` は別のサイト。辞書 `update.pasta` は `OnUpdateComplete` の Reference0 が `changed` かどうかで「更新成功」「更新無し」を分ける。

### 正典（ukadoc）の位置づけ

| 正典 | 逐語引用 | 本仕様への含意 |
|---|---|---|
| [ネットワーク更新への対応](https://ssp.shillest.net/ukadoc/manual/dev_update.html) | 「ネットワーク更新で行われるのは、サーバにあるファイルでユーザ側にあるファイルを上書きすることであって、同期（ミラーリング）することではない」「一度配布してしまった不要ファイルをネットワーク更新を通じて取り除くためには、delete.txtを用います」「updates.txtで代用する事ができます」 | 上書きと `delete.txt` の削除の 2 つ。両方の定義ファイルを読む（エンジン済み）。 |
| [ネットワーク更新（ファイル構成）](https://ssp.shillest.net/ukadoc/manual/manual_update.html) | 「ゴーストの場合 +-(myghost) … +-updates2.dau +-updates.txt +-delete.txt +-delete1.txt (SSPのみ・delete[数字].txtを認識可)」「バルーンをネットワーク更新に対応させる場合 +-(myballoon) +-updates2.dau …」 | 対象はゴーストのフォルダ・シェルのフォルダ・バルーンのフォルダ。 |
| [`homeurl,URL`（ゴースト）](https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#homeurl_2cURL:1) | 「ネットワーク更新用のURL。ゴースト起動中はSHIORI側の記述が優先され、エクスプローラ上での更新やプロパティシステムによる呼出は(SHIORI読み込みコストを避けるため)こちらが優先される。urlの末尾に/を忘れないように注意。」 | 起動中の更新先は SHIORI リソース `homeurl` → `descript.txt` の順。末尾の `/` はエンジンが補う。 |
| [`homeurl`（SHIORI リソース）](https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#homeurl:1) | 「ネットワーク更新用ファイルの位置。何かSHIORIに重大な不具合が生じた際のバックアップ用にdescript.txtにも書ける。」 | 同上。 |
| [`homeurl,URL`（シェル）](https://ssp.shillest.net/ukadoc/manual/descript_shell.html#homeurl_2cURL:1)／[（バルーン）](https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#homeurl_2cURL:1) | 「ネットワーク更新用のURL。」 | シェル・バルーンは各 `descript.txt` から。 |
| [`useorigin1`](https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#useorigin1:1) | 「ネットワーク更新時のファイル数の開始数値。1でファイル数1から開始、0でMATERIAと同じ0から開始。」 | 番号の読み替えは本仕様。 |
| [`other_homeurl_override`](https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#other_homeurl_override:1) | 「\![updateother]やシェル・バルーン同時更新機能において、更新対象の更新先URL(homeurl)を強制的に置き換える。」Reference0〜4 付き | 照会に Reference が要る。α では送らない（裁定 11）。 |
| [`updatebutton.caption`](https://ssp.shillest.net/ukadoc/manual/list_shiori_resource.html#updatebutton.caption:1) | 「ネットワーク更新の名称。」 | 項目名は今日どおり。 |
| [`\![updatebymyself(,オプション…)]`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bupdatebymyself_28_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._29_5d:1) | 「ネットワーク更新チェックイベントを開始する。右クリックメニューやゴーストエクスプローラからユーザが行うことも出来る。オプション指定はSSPのみ。」 | メニューと同じ手続き。 |
| [`\![update,更新対象(,オプション…)]`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bupdate_2c_66f4_65b0_5bfe_8c61_28_2c_30aa_30d7_30b7_30e7_30f3_2c_30aa_30d7_30b7_30e7_30f3..._29_5d:1) | 「ghost、shell、balloonが指定可能で、同時に更新する場合それぞれを「+」で区切って指定できる。更新対象をallとすると、\![updatebymyself]と同義。」「ghost以外の更新時、更新があればSHIORIイベントOnUpdateOtherReadyなどOnUpdateがOnUpdateOtherに変わったイベンド群が通知され、すべて更新が終わった後に結果の総括としてOnUpdateResultが発生する。」 | 対象の選び方と、ゴースト以外の名の付け替え。 |
| [`\![updateother,更新対象/オプション群,...]`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bupdateother_2c_66f4_65b0_5bfe_8c61_2f_30aa_30d7_30b7_30e7_30f3_7fa4_2c..._5d:1) | 「任意の名前の更新対象のネットワーク更新イベントを開始する。起動中でないものも指定できる。--balloon=バルーン名 --shell=シェル名 --plugin=… --headline=… --language=…」「結果の総括としてOnUpdateResultExが発生する。」 | α は `--shell=`・`--balloon=` だけ。 |
| 更新オプション指定（同ページの注記・2026-09-29 にページ本体から引いた） | `checkonly`「普通の更新イベントの代わりに、SHIORIイベントOnUpdateCheckComplete、失敗した場合はOnUpdateCheckFailureが通知され、更新の有無を判定でき次第処理を中断する。」`testonly`「実際のファイルの置き換えは行わない。」`recovery [SSP 2.5.26～]` | `OnUpdateCheck*` はオプション `checkonly` の系。α ではオプションを受けない（裁定 8）。 |
| [`OnUpdateProcessExec`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateProcessExec:1) | 「メニューなどから更新を実行する指示がなされた場合に発生する。このイベントに対して何かスクリプトを返すと、更新処理をカスタマイズできる。イベントを無視した場合は標準の更新処理に自動的に移行する。」Reference0「manual : メニューから更新の操作をした。auto : …。testonly : …」 | メニューからだけ送る。応えがあれば標準の手続きを行わない。 |
| [`OnUpdateBegin`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateBegin:1) | 「ネットワーク更新開始が指示された際に発生。」Reference0「ゴースト名。」Reference1「フルパス。」Reference3「SSP：※更新対象種別」Reference4「SSP 2.3：※更新実行理由。」 | Reference2 は無い（空）。 |
| [`OnUpdateReady`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateReady:1) | 「更新ファイルが確認された際に発生。」Reference0「更新を行うファイルの総数から1引かれたもの（0から始まる連番）。」Reference1「カンマでセパレートされた更新されたファイル名のリスト。」Reference3「更新対象種別。(shell ghost balloon headline plugin)」 | 差分の一覧。 |
| [`OnUpdate.OnDownloadBegin`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdate.OnDownloadBegin:1) | Reference0「ダウンロードするファイル名。」Reference1「ダウンロード中の更新ファイルの番号（0から始まる連番）。」Reference2「更新を行うファイルの総数から1引かれたもの（0から始まる連番）。」 | 各ファイルの前に 1 回。 |
| [`OnUpdate.OnMD5CompareBegin`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdate.OnMD5CompareBegin:1)／[`Complete`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdate.OnMD5CompareComplete:1)／[`Failure`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdate.OnMD5CompareFailure:1) | Reference0「比較するファイル名。／落としたファイル名。」Reference1「正しいMD5値。」Reference2「落としたファイルのMD5値。」 | 各ファイルの照合。 |
| [`OnUpdateComplete`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateComplete:1) | 「ネットワーク更新が成功し完了した際に発生。」Reference0「※成功理由」Reference1「カンマでセパレートされた更新されたファイル名のリスト。」 | 差分 0 でもこれ（理由 `none`）。 |
| [`OnUpdateFailure`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateFailure:1) | 「ネットワーク更新に失敗した際に発生。」Reference0「※失敗理由」Reference1「※SSPのみ　失敗したファイル名。」 | Reference1 はエンジンの `file()`。 |
| [`OnUpdateOtherBegin`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateOtherBegin:1) ほか `OnUpdateOther*` 9 語 | 「ゴースト以外の……」で Reference は `OnUpdate*` と同じ形 | シェル・バルーンはこの名。 |
| [`OnUpdateResult`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateResult:1) | 「バルーン・シェル一括更新機能も含めてすべての更新結果を一括で通知する。」Reference*「更新を実行した順に … (※1)[\1](※2)[\1](※3)[\1](※4) または (※1)[\1](※2)[\1](※3)」※1 種別・※2「OK」「NG」・※3「成功した場合は更新ファイル数(更新ファイルなし=noneの場合0)、失敗した場合は失敗理由」・※4「失敗して、かつ失敗原因のファイルがわかる場合、そのファイル名」 | 総括。 |
| [`OnUpdateResultEx`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateResultEx:1) | 「(※1)[\1](※2)[\1](※3)[\1](※4)[\1](※5)」※1 名前・※2 種別・※3 OK/NG・※4 件数か理由・※5 ファイル名 | `updateother` の総括。 |
| 同ページの注記（ukadoc MCP の索引には無く、2026-09-29 にページ本体から引いた） | 更新対象種別「ghost / shell / balloon / plugin / headline / baseware ※CROWのみ」／更新実行理由「auto：SSPの設定などによる自動更新。manual：オーナードローメニューからの選択などによる手動実行。script：さくらスクリプトタグ（\![updatebymyself]など）による更新。」／成功理由「none：更新するファイルが無かった。changed：更新された。」／失敗理由「timeout：タイムアウトした。／too slow／md5 miss：MD5が一致しなかった。／artificial：ユーザ操作による更新中断。／404 等：そのステータスコードでの失敗。／fileio：※SSPのみ。容量不足による更新対象への書き込み失敗。／readonly／virusdetect／toomanyredirect／executing：※SSPのみ。すでに更新を実行中のため、二重起動を阻止した。／parse：※SSPのみ。取得はできたが、内容の解析に失敗した。／downloading／paramerror：※SSPのみ。更新実行タグの設定ミスにより更新を実行できなかった。」 | 種別・理由・成功理由・失敗理由は正典の語で送る。 |
| [`OnUpdateCheckComplete`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateCheckComplete:1)／[`OnUpdateCheckFailure`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateCheckFailure:1)／[`OnUpdateCheckResult`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateCheckResult:1)／[`OnUpdateCheckResultEx`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateCheckResultEx:1) | 「更新チェックのみオプションをつけた時に最後に発生。」 | `checkonly` の系＝α 後（送らない）。 |
| [`OnUpdateResultExplorer`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdateResultExplorer:1) | 「ゴーストエクスプローラからネットワーク更新を実行した際に発生。」 | areka にエクスプローラは無い＝送らない。 |
| [`OnUpdatedataCreating`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdatedataCreating:1)／[`OnUpdatedataCreated`](https://ssp.shillest.net/ukadoc/manual/list_shiori_event.html#OnUpdatedataCreated:1)／[`\![execute,createupdatedata,…]`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bexecute_2ccreateupdatedata_2c_51fa_529b_5148_30d5_30a1_30a4_30eb_5d:1) | 更新定義ファイルを**作る**側。 | 範囲外（α 後・束「開発者機能」）。 |
| [`\![update,platform]`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bupdate_2cplatform_5d:1) | 「本体ネットワーク更新イベントを開始する。」 | 範囲外（α 後）。 |
| [`\![execute,install,url,URL,種別]`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5bexecute_2cinstall_2curl_2cURL_2c_7a2e_5225_5d:1) | 「指定したURLのファイルをダウンロードし、種別に応じたインストール／登録処理を行う。ファイルのダウンロードを一時ファイル用フォルダ内に自動的に行い、適宜インストール処理を行う。このタグで実行されたダウンロード処理については、OnURLQueryは発生しない。種別は省略可能。省略した場合はContent-Typeとファイルの中身から自動判別される。nar……narファイルとしてインストールする feed……ヘッドラインセンサ(RSSFeed)として登録する homeurl……そのURLをhomeurlとみなしてネットワーク更新を実行する ical……… ssf……」 | α は `nar`（省略も `nar` と読む）だけ。`feed`・`homeurl`・`ical`・`ssf` は α 後。 |
| [`\![reload,ghost]`](https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_5c_21_5breload_2cghost_5d:1) | 「ゴースト全体をリロードする。」記述例「リロードする。自分自身へのゴースト切り替えと、SHIORIイベントが発生しないことを除けばほぼ同じ。」 | 読み直し＝自分自身への切替（知らせを送らない）。完全形は α 後。 |
| [`updates2.dau` の書式](https://ssp.shillest.net/ukadoc/manual/spec_update_file.html)（[セキュリティチェック](https://ssp.shillest.net/ukadoc/manual/spec_update_file.html#_30bb_30ad_30e5_30ea_30c6_30a3_30c1_30a7_30c3_30af:1)・[行種別](https://ssp.shillest.net/ukadoc/manual/spec_update_file.html#_884c_7a2e_5225:1)・[拡張フィールド](https://ssp.shillest.net/ukadoc/manual/spec_update_file.html#_62e1_5f35_30d5_30a3_30fc_30eb_30c9_20_28_4f4d_7f6e_5b2_5d_4ee5_964d_29:1)） | 「以下の条件に該当するエントリは無効として扱われる。MD5がない。末尾が / または \（ディレクトリエントリ）。..\ や ../ を含む」「file, でも charset, でもない行は無視される。」 | エンジン済み（完了 `update-engine`）。本仕様は触らない。 |
| [`delete.txt` の「相対パス」](https://ssp.shillest.net/ukadoc/manual/descript_install.html#_76f8_5bfe_30d1_30b9:1) | 「ネットワーク更新時に削除するファイルをゴーストのホームフォルダからの相対パスで列挙する。相対パスの階層は「\」区切り。フォルダごとの指定も可能で、その場合は末尾を\で終える。」 | エンジン済み。台帳の状態を動かすのは本仕様。 |

正典が沈黙している点（`homeurl` の無い対象の扱い・二重起動の知らせ方・正典に無い輸送の失敗の語・`OnUpdateOtherBegin` の Reference0・読み直しの時機と読み直した後の起動の根・更新の途中で切替や終了が来たとき・`\![execute,install,url]` の落とし場所と失敗の知らせ）は areka の裁量として要件 10 で決め、`doc/COMPAT_ARCHITECTURE.md` §8 に記す。SSP の挙動を測って合わせることはしない。

### brief の記述を正典と実物で引き直して改めた点

1. brief の「差分 0 なら `OnUpdateCheckComplete`／`OnUpdateCheckFailure`」は正典と合わない。`OnUpdateCheck*` はオプション `checkonly` の系で、標準の更新で差分 0 のときは `OnUpdateComplete`（成功理由 `none`）。emo2 の辞書もそう読んでいる（`changed` 以外を「更新無し」）。`OnUpdateCheck*` 4 語は α では送らない（裁定 8）。
2. brief の「許可表 13 件」「`events_change_tests.rs` の 13」は `ghost-install`（21）と `file-drop`（23）で動いた。本仕様は 23 → **43**。
3. brief の「`OnUpdateProcessExec` を入れるなら返信付きの `KanadeMsg` の変種を足す（+2〜3）」は `ghost-install` が `RaiseEvent` に `reply` を足したので不要。応えの有無は `RaiseOutcome` の `Script`／`NoReply` で分かる。kanade に触るのは許可表 2 本だけのまま（裁定 5）。
4. brief の「`homeurl`／`useorigin1`／`other_homeurl_override` の 3 行を `ALLOWED_RESOURCE_IDS` に足すだけ」のうち `other_homeurl_override` は Reference0〜4 付きの照会で、`ResourceQuery` に Reference の欄が無い。α では送らない（裁定 11）。足すのは 2 行（10 → 12）。
5. brief の「終了で待つ口を `ghost-install` が作る」は `exit_wait.rs` として着地済み。本仕様は門を 1 つ登記するだけ。
6. brief の「`\![execute,install,url]` は `install_cue.rs` を改変」は実物どおり（`url` は `install_cue_unsupported` の腕）。依頼は `InstallOrder`（`origin` は既存の `Script`）で渡す。
7. brief の Approach「MD5 は `md-5` を 1 本足す」「`HttpFetch`」は古い（エンジンで決着済み・依存の追加 0・境界は `Fetch`）。本仕様で足す外部クレートは 0。
8. brief の 09-27 節「(d) は成り立たない」「推す答え＝(b)」を採る（裁定 1）。エンジンの改修は「成功した確定の後片付けで消せなかった残りを、戻せなかった走行の残骸と区別する」1 点だけ。
9. emo2 の `shell/master/descript.txt` に `homeurl` が無い（実測）。「現ゴースト＋現シェル＋現バルーン」の束で、`homeurl` の無い対象を失敗にすると emo2 は毎回 NG を 1 つ抱える。飛ばす（裁定 7）。

### 要件を書く途中で見つかった、brief に無い事実

1. **エンジンの前提条件と (b) が食い違う。** 完了 `update-engine` の設計は `run` の前提に「起動中のゴーストなら SHIORI を先に解放している」と書く。(b) はこの前提を外して呼ぶ。確定そのものは写像中の DLL でも通る（較正済み）ので、外して困るのは後片付けだけ（本節の実測）。完了 spec の文書は書き換えず、本仕様の設計の「Boundary Commitments」と §8 に「前提を外して呼ぶ・後片付けの残りの扱い」を記す。
2. **進捗の GET の応答は再生中の台詞を置き換える**（`events.rs` の `value_replaces_active_talk` は `OnSecondChange` 以外すべて真）。ファイルごとに 3〜4 件のイベントが続くと、前の台詞は切れる。SSP と同じ振る舞いなので、要件では「イベントを落とさない」だけを求め、設計で明記する。
3. **読み直しは台詞を切る。** 自分自身への切替は再生中の台詞を待たずに降ろす。`OnUpdateComplete`／`OnUpdateResult` の台詞（emo2 なら「更新完了で！」）が見えるように、最後の台詞が終わってから読み直す（要件 5.3）。
4. **`\![execute,install,url]` で落としたファイルの片付け。** インストールの手続きは背景で走り、書庫を読み終えたことを依頼の側へ返す口が無い。落としたファイルはその場で消せない（要件 6.6）。

### 何を変えるか

入口 4 つ（メニュー・`\![updatebymyself]`・`\![update,…]`・`\![updateother,…]`）が同じ 1 本の手続きへ流れ、手続きは背景で走って、対象ごとに正典のイベント列を起動中のゴーストへ送り、最後に総括を送る。更新の間もゴーストは生きて話し、何か変わったときだけ、最後の台詞が終わってから同じゴーストを読み直す。失敗は正典の語の理由で知らせ、記録に残す。`\![execute,install,url,URL,nar]` は URL から落として `ghost-install` の手続きへ渡す。終了は走っている更新を上限つきで待つ。

## Boundary Context

- **In scope**:
  - 更新の手続き 1 本（対象を解く → `homeurl` を解く → エンジンの `run` を背景で呼ぶ → 進捗をイベントに写す → 総括 → 読み直し）と、入口 4 つ: メニュー「ネットワーク更新」・`\![updatebymyself]`・`\![update,ghost|shell|balloon(+…)|all]`・`\![updateother,--shell=名,--balloon=名]`。
  - 対象 3 種: 今のゴースト（`<根>/ghost/<フォルダ>/`）・今のシェル（起動時に解いたシェルのフォルダ）・今のバルーン（起動時に解いたバルーンのフォルダ）。`updateother` は名前で引いた、起動中でないシェル・バルーンも対象にできる。
  - `homeurl` の解決（SHIORI リソース → `descript.txt`）・`useorigin1`・`OnUpdateProcessExec`（メニューからだけ）。
  - 送るイベント 20 語: `OnUpdateProcessExec`・`OnUpdateBegin`・`OnUpdateReady`・`OnUpdate.OnDownloadBegin`・`OnUpdate.OnMD5CompareBegin`・`OnUpdate.OnMD5CompareComplete`・`OnUpdate.OnMD5CompareFailure`・`OnUpdateComplete`・`OnUpdateFailure`・`OnUpdateOtherBegin`・`OnUpdateOtherReady`・`OnUpdateOther.OnDownloadBegin`・`OnUpdateOther.OnMD5CompareBegin`・`OnUpdateOther.OnMD5CompareComplete`・`OnUpdateOther.OnMD5CompareFailure`・`OnUpdateOtherComplete`・`OnUpdateOtherFailure`・`OnUpdateResult`・`OnUpdateResultEx`。リソース 2 語: `homeurl`・`useorigin1`。
  - 更新の間ゴーストを生かしたまま進めること（裁定 1）と、そのためのエンジンの後片付けの改修（`crates/areka-update/` の中だけ）。
  - 更新後の読み直し（同じゴーストへの、知らせを送らない切替）。
  - `\![execute,install,url,URL,nar]`（`nar`・省略）。
  - 終了で走っている更新を待つこと（既存の口に門を 1 つ）。
  - `homeurl` の読み手（`descript.txt`）。
  - 網羅台帳・生成物・`dist/README.txt` の 2 行・`doc/COMPAT_ARCHITECTURE.md` §8・正典 URL の行。
  - 決定論テストと実機確認（https を 1 度通す）。
- **Out of scope**:
  - 定義ファイルの読み取り・差分・取得・MD5・確定・`delete.txt` の規則（完了 `update-engine`。本仕様は `run` を呼ぶだけで、後片付けの 1 点を除いて `crates/areka-update/` を変えない）。
  - 更新オプション（`checkonly`・`testonly`・`recovery`）と `OnUpdateCheckComplete`／`OnUpdateCheckFailure`／`OnUpdateCheckResult`／`OnUpdateCheckResultEx`（α 後）。
  - `other_homeurl_override`（照会に Reference が要る・α 後）。
  - `OnUpdateResultExplorer`（エクスプローラは無い）・定期自動更新（理由 `auto`・設定画面が要る・α 後）・`\![update,platform]`・`OnBasewareUpdating`／`OnBasewareUpdated`。
  - 作る側: `\![execute,createupdatedata]`・`OnUpdatedataCreating`／`OnUpdatedataCreated`・`updates2.dau` の書き出し・`developer_options.txt`・`.gitignore` 形式のフィルタ。
  - `\![execute,install,url]` の `feed`・`homeurl`・`ical`・`ssf`、URL の投げ込み（`OnURLQuery`／`OnURLDropping`／`OnURLDropped`／`OnURLDropFailure`）、`x-ukagaka-link:` の URL（α 後）。
  - `updateother` の `--plugin=`・`--headline=`・`--language=`（プラグイン・ヘッドライン・言語パックは無い）。
  - 読み直しの完全形 `\![reload,ghost|shell|balloon]`（α 後）・シェルとバルーンの実行中の切替（`shell-balloon-switch`）。
  - 更新の途中の中断（理由 `artificial`・中断の操作が無い）。
  - emo2 の辞書に台詞を足すこと（辞書はリポジトリの外・開発者の手）。
- **Adjacent expectations**:
  - 完了 `update-engine` の保証（全部入るか 1 つも入らないか・失敗の値が原因のファイルと作業場所を持つ・無効な行は捨てて警告・`delete.txt` は走行を失敗にしない）に依る。本仕様は `run` の前提「SHIORI を先に解放している」を外して呼ぶ（要件 5・裁定 1）。
  - 完了 `ghost-shell-balloon-switch` の切替の入口（自分自身への切替）・汎用の通知の入口と応えの返事、完了 `ghost-install` の依頼の口と終了で待つ口、完了 `popup-menu-minimal` の「ネットワーク更新」枠、完了 `baseware-root-layout` の根と目録、完了 `session-mark-residue` の上限 3 秒と印の判定に依る。完了 spec の文書は書き換えない。
  - 後続 `shell-balloon-switch`（本仕様の後・許可表の数を 43 から動かす・「今のシェル・今のバルーン」を切替後の物に読み替える）・`alpha-release-signoff`（第三者の手順「更新する」・既知の制限）へ申し送る（要件 8.8）。
  - 並走する `balloon-color-emoji`（`areka-emo-text`・`areka-sakura`）とは共有するソースが 0。`Cargo.lock` は触る節が別。

## Requirements

### Requirement 1: 入口 4 つが同じ手続きへ流れ、対象と更新先が決まる

**Objective:** As a ゴーストを入れた第三者とゴーストの作者, I want メニューからでも台本からでも同じように更新が始まること, so that 入れ方を覚え直さずに済み、辞書の `\![updatebymyself]` がそのまま動く

#### Acceptance Criteria

1. The areka shall メニュー「ネットワーク更新」・`\![updatebymyself]`・`\![update,…]`・`\![updateother,…]` から届いた要求を、同じ 1 本の更新の手続きで扱う（入口ごとに別の手順を持たない）。
2. When ゴーストを起こす（最初の起動でも切替や読み直しの後でも）, the areka shall メニューの「ネットワーク更新」枠へ項目を登記し直す（項目名は今日どおり `updatebutton.caption`、答えが無ければ既定名「ネットワーク更新」）。
3. While 今のゴーストの更新先（要件 1.9 で解いた `homeurl`）が無い, the areka shall 「ネットワーク更新」の項目を選べない状態（灰色）で出す。
4. When 利用者が「ネットワーク更新」を選ぶ, the areka shall 今のゴースト・今のシェル・今のバルーンの 3 つを対象にした要求（理由 `manual`）を作る。
5. When 再生中の台本が `\![updatebymyself]` または `\![update,all]` の位置に達する, the areka shall 要件 1.4 と同じ 3 つを対象にした要求（理由 `script`）を作る。
6. When 再生中の台本が `\![update,対象]` の位置に達し、対象が `ghost`・`shell`・`balloon` を `+` で 1 つ以上並べた形である, the areka shall 並べた対象だけ（今のゴースト・今のシェル・今のバルーン）を、並んだ順に更新する要求（理由 `script`）を作る。
7. When 再生中の台本が `\![updateother,…]` の位置に達し、`--shell=名前`・`--balloon=名前` が 1 つ以上並んでいる, the areka shall 名前を今のゴーストのシェルの目録・根のバルーンの目録から `descript.txt` の `name` で引き（大文字小文字を区別・起動中でなくてもよい）、引けた物を並んだ順に更新する要求（理由 `script`）を作る。引けない名前は `warn!` 1 件で飛ばす。`menu,hidden` のシェルはメニューの目録と同じく引かない（引けない名前として飛ばす・裁定 18）。
8. If `\![update,…]`／`\![updatebymyself]`／`\![updateother,…]` に更新オプション（`checkonly`・`testonly`・`recovery`・`--option=…`）が付いている、または `\![update,…]` の対象に `ghost`・`shell`・`balloon`・`all` 以外の語がある、または `\![updateother,…]` に `--shell=`／`--balloon=` 以外の指定しか無い, then the areka shall 要求を作らず、イベントを 1 件も送らず、`warn!` を 1 件残す。
9. The areka shall 対象ごとの更新先を次の順で解く: ゴーストは SHIORI リソース `homeurl` の応答（空・返事なしでなければそれ）→ ゴーストの `descript.txt` の `homeurl`／シェルとバルーンは各 `descript.txt` の `homeurl`。
10. If 対象の更新先がどちらにも無い, then the areka shall その対象を飛ばし（その対象のイベントは 0 件・総括にも載せない）、`warn!` を 1 件残す。要求の対象が全部飛ばされたときは総括も送らない（0 件）。
11. The areka shall 更新先の値をそのままエンジンへ渡す（末尾の `/` の補いと、`http://`／`https://` 以外の拒否はエンジンの規則のまま）。
12. When 利用者がメニューから「ネットワーク更新」を選んだ, the areka shall 手続きを始める前に `OnUpdateProcessExec`（Reference0＝`manual`）を GET で送り、ゴーストが台本を返したら（`Script`）標準の手続きを行わず（要求を捨て・以後のイベント 0 件・`info!` 1 件）、返さなければ（返事なし・空の台本）標準の手続きへ進む。
13. The areka shall 台本の入口（`\![updatebymyself]`・`\![update,…]`・`\![updateother,…]`）からは `OnUpdateProcessExec` を送らない（0 件）。
14. The areka shall 更新の手続きを 1 度に 1 本だけ走らせる。
15. If 手続きが走っている間に新しい要求が届く, then the areka shall その要求を始めず、`OnUpdateFailure`（対象の種別によらずこの名・Reference0＝`executing`・Reference1＝空・Reference3＝要求の先頭の対象の種別・Reference4＝理由）を GET で 1 回送り、総括は送らず、`warn!` を 1 件残す。
16. While 手続きが走っている, the areka shall 「ネットワーク更新」の項目を選べない状態で出す。
17. While ゴーストが定常でない（起動の途中・切替の途中・読み直しの途中）, when 更新の要求が届く, the areka shall 要求を始めず `warn!` を 1 件残す（待たせない・イベント 0 件）。
18. While ゴーストが表示されている, the areka shall 更新先への接続・取得・照合・確定のどの間も、ゴーストの描画・台詞の再生・メニュー・Windows の終了の後始末を止めない（どのスレッドで回すかは設計で決める。背景から UI の窓へ同期で送る形は取らない＝同期送信の見張りの例外表を増やさない）。
19. The areka shall 最初の起動が窓の無い形へ倒れた回（窓も台本の受け口も無い）では、4 つの入口のどれも受けない（今日どおり・本仕様で足す経路 0）。

### Requirement 2: 対象 1 つの更新は正典の順序でイベントを送る

**Objective:** As a ゴーストの作者, I want 更新の始まり・差分・各ファイルの取得と照合・終わりが正典のイベントと Reference で届くこと, so that 辞書に書いた返事がそのまま動く

#### Acceptance Criteria

1. The areka shall 対象 1 つの更新を「始まりの知らせ → 定義ファイルの取得と差分 → 差分の一覧の知らせ → 各ファイルの取得の知らせと照合の知らせ → 確定と削除 → 締めの知らせ」の順に進め、対象が複数のときは 1 つ終えてから次へ進む（並べない）。
2. The areka shall 今のゴーストが対象のときは `OnUpdate*` の名で、シェル・バルーンが対象のときは同じ列を `OnUpdateOther*` の名（`OnUpdateOtherBegin`・`OnUpdateOtherReady`・`OnUpdateOther.OnDownloadBegin`・`OnUpdateOther.OnMD5CompareBegin`／`Complete`／`Failure`・`OnUpdateOtherComplete`・`OnUpdateOtherFailure`）で送る。
3. The areka shall 本仕様のイベントをすべて GET で送り、送り先は送る時点で起動中のゴーストとする。
4. When 対象 1 つの更新を始める, the areka shall `OnUpdateBegin`（Reference0＝対象の名前〔ゴーストなら `descript.txt` の `name`・シェル／バルーンも各 `descript.txt` の `name`〕・Reference1＝対象のフォルダの絶対パス・Reference2＝空・Reference3＝種別〔`ghost`／`shell`／`balloon`〕・Reference4＝理由〔`manual`／`script`〕）を送る。
5. The areka shall Reference3 の種別と Reference4 の理由を、その対象の全イベント（`OnUpdateBegin`〜`OnUpdateFailure`／`OnUpdateComplete`）に同じ値で載せる。
6. When 定義ファイルを取得して差分が決まり、差分が 1 件以上ある, the areka shall `OnUpdateReady`（Reference0＝件数から 1 引いた数・Reference1＝差分のファイル名をカンマで並べた一覧・Reference3・Reference4）を送る。
7. When 差分が 0 件である, the areka shall `OnUpdateReady` も各ファイルのイベントも送らず（0 件）、`OnUpdateComplete`（Reference0＝`none`・Reference1＝空・Reference3・Reference4）を送る。
8. When 差分の各ファイルの取得を始める, the areka shall `OnUpdate.OnDownloadBegin`（Reference0＝ファイル名・Reference1＝そのファイルの番号・Reference2＝件数から 1 引いた数・Reference3・Reference4）を送る。番号は 0 始まり。
9. When 落としたファイルの MD5 を照合した, the areka shall `OnUpdate.OnMD5CompareBegin`（Reference0＝ファイル名・Reference1＝定義ファイルの MD5・Reference2＝落としたファイルの MD5・Reference3・Reference4）を送り、続けて一致なら `OnUpdate.OnMD5CompareComplete`、不一致なら `OnUpdate.OnMD5CompareFailure`（Reference は同じ形）を送る（照合の始まりと結果を 1 回の通知から続けて 2 件送る）。
10. When 全件が一致して確定と削除が済んだ, the areka shall `OnUpdateComplete`（Reference0＝`changed`・Reference1＝入れ替えたファイル名をカンマで並べた一覧・Reference3・Reference4）を送る。定義ファイル自身は一覧に含めない。
11. If 対象の更新が失敗する, then the areka shall `OnUpdateFailure`（Reference0＝要件 4.2 の失敗理由の語・Reference1＝原因のファイル名〔分からなければ空〕・Reference3・Reference4）を送り、その対象の `OnUpdateComplete` は送らない（0 件）。
12. The areka shall 対象 1 つにつき締めの知らせを `OnUpdateComplete` か `OnUpdateFailure` のどちらか 1 件だけ送る。途中でアプリが終わった場合（要件 7）と、対象のゴーストが居なくなった場合（要件 5.7）だけは 0 件。
13. When SHIORI リソース `useorigin1` の応答が `1` である, the areka shall 要件 2.6 の Reference0・2.8 の Reference1・Reference2 を 1 始まりで数える（番号は 1 から・「件数から 1 引いた数」は件数そのもの）。`0`・空・返事なしなら 0 始まり。
14. The areka shall `useorigin1` を要求ごとに 1 回だけ照会し、その要求の全イベントで同じ読みを使う。
15. The areka shall 本仕様で新しいイベント名を作らない（0 個）。送るのは正典に在る 20 語（Boundary Context）だけで、kanade の許可表は 23 語から 43 語になる。

### Requirement 3: 総括を 1 回送る

**Objective:** As a ゴーストの作者, I want 一括で更新した結果が最後に 1 回まとまって届くこと, so that 何が更新され何が失敗したかを 1 つの台詞で言える

#### Acceptance Criteria

1. When 要求の対象を全部（飛ばした対象を除く）終えた, the areka shall メニュー・`\![updatebymyself]`・`\![update,…]` の要求では `OnUpdateResult` を、`\![updateother,…]` の要求では `OnUpdateResultEx` を、GET で 1 回だけ送る。
2. The areka shall `OnUpdateResult` の Reference を「実行した順に対象 1 つにつき Reference 1 つ」とし、その値を `種別\x01OK\x01入れ替えた件数`（成功・差分 0 なら `0`）または `種別\x01NG\x01失敗理由`（原因のファイルが分かれば `\x01ファイル名` を足す）の形にする（`\x01` はバイト値 1）。
3. The areka shall `OnUpdateResultEx` の Reference を同じ並びで `名前\x01種別\x01OK|NG\x01件数または理由(\x01ファイル名)` の形（先頭に対象の名前）にする。
4. The areka shall 総括を、要求の最後の対象の締めの知らせ（`OnUpdateComplete`／`OnUpdateFailure`）の後に送る。
5. The areka shall 要求の対象が全部飛ばされた（要件 1.10）とき、二重起動を断った（要件 1.15）とき、`OnUpdateProcessExec` に応えがあった（要件 1.12）ときは、総括を送らない（0 件）。

### Requirement 4: 失敗はメッセージボックスではなく、正典の語の理由と記録で伝える

**Objective:** As a 利用者, I want 更新できなかったときに、できなかったことと理由がゴーストの台詞と記録に残り、手元のゴーストが壊れないこと, so that 何も黙って消えず、失敗しても今日どおり使い続けられる

#### Acceptance Criteria

1. The areka shall 失敗・二重起動・飛ばした対象のどれでも、メッセージボックスを出さない（0 個）。本仕様が出す画面は 0。
2. The areka shall `OnUpdateFailure`・`OnUpdateOtherFailure`・総括の失敗理由を次の表で決め、エンジンの失敗の種類 11 種と取得の失敗 8 種のすべてが表のどれか 1 つに必ず写るようにする（写らない種類 0・種類が増えたらビルドが止まる形）。

   | 正典の語 | 写す失敗 |
   |---|---|
   | `paramerror` | 更新先の URL が `http://`／`https://` で始まらない（`InvalidHomeurl`） |
   | `404` などのステータスコードの数字 | 定義ファイルが `updates2.dau`・`updates.txt` のどちらも無い（`ManifestMissing`＝`404`）、取得が成功以外の応答で終わった（`NotFound`＝`404`・`Status { code }`＝その数字） |
   | `timeout` | 取得が時間切れになった（`Timeout`） |
   | `md5 miss` | 落としたファイルの MD5 が定義ファイルと一致しない（`Md5Mismatch`） |
   | `fileio` | 手元の読み書きの失敗（`TargetMissing`・`LocalUnreadable`・`WorkArea`・`EscapesTarget`・`CommitWrite`・`RollbackFailed`） |
   | areka の語 `dns`・`connect`・`tls`・`toolarge`・`http` | 正典に語が無い輸送の失敗（`NameResolution`・`Connect`・`Tls`・`TooLarge`・`Other`）。§8 に記す（裁定 10） |
   | `executing` | 走っている間に届いた要求（要件 1.15） |

3. The areka shall 正典の語 `too slow`・`artificial`・`readonly`・`virusdetect`・`toomanyredirect`・`parse`・`downloading` を送らない（0 件。リダイレクトの追随の上限はエンジンが持ち、超えれば `Other`＝`http`）。
4. If 取得の部品（WinHTTP）を用意できない, then the areka shall `error!` を 1 件残し、その要求を失敗（理由 `connect`）として要件 2.11 と要件 3 のとおり知らせる（エンジンの `WinHttpFetch::new()` は記録を出さないので、呼ぶ側で必ず残す）。
5. If 対象の更新が失敗する, then the areka shall `error!` を 1 件残し、更新先の URL・対象のフォルダ・どの段で失敗したか・エンジンの失敗の種類の語（`kind()`）・原因のファイル・宛先が元へ戻ったか・戻せなかったときは作業場所を載せる（エンジン自身の `error!` とは別に、呼ぶ側の 1 件）。
6. The areka shall 失敗した対象の本番のフォルダを、呼ぶ前のままにする（エンジンの全か無かの保証に依る）。戻せなかったときは作業場所の在りかを `error!` に載せ、次の走行の残骸の警告に任せる（本仕様で救い出しはしない）。
7. The areka shall ゴーストが `OnUpdateFailure`・`OnUpdateOtherFailure`・`OnUpdateResult` に応えなかったとき、代わりの画面や台詞を出さない（利用者から見える変化は 0・記録は残る）。
8. When 更新は成功したが、エンジンが消せなかった物（`undeletable`）や片付けられなかった作業場所（`leftovers`）を返した, the areka shall 成功として扱い、残った場所を `warn!` に残す。
9. The areka shall `delete.txt` の無効な行・定義ファイルの無効な行（エンジンの `UpdateWarning`）の記録をエンジンの `warn!` に任せ、本仕様側で重ねて出さない（0 件）。

### Requirement 5: 更新の間もゴーストは生きていて、何か変わったときだけ終わってから読み直す

**Objective:** As a ゴーストを更新する利用者, I want 更新の間ゴーストが消えずに進捗を話し、終わったら新しい中身で戻ってくること, so that 何が起きているか分かり、更新の後に新しい辞書やシェルが効く

#### Acceptance Criteria

1. The areka shall 取得・照合・確定・削除の間、今のゴーストを降ろさない（SHIORI を解放しない・窓を閉じない）。進捗のイベントはこのゴーストへ届く。
2. When 更新が 1 つでも `changed` で終わった（対象がゴースト・シェル・バルーンのどれでも）, the areka shall 総括の後に、同じゴーストを読み直す（完了 `ghost-shell-balloon-switch` の「自分自身への、知らせを送らない切替」＝`OnGhostChanging` を送らない・終了の挨拶を再生しない・SHIORI の解放を待つ・全窓が 0 になっても終了しない）。読み直しでゴースト・シェル・バルーンの 3 つの中身がすべて読み直される（起動時と同じ解き方）。ただしコマンドライン引数でゴーストのフォルダを指して始めたプロセス（根の目録の外のゴースト）では読み直さず、`warn!` を 1 件残す（裁定 17）。
3. The areka shall 読み直しを、総括への返事の台詞が終わってから（返事が無ければ総括を送った直後に）始める。
4. When 要求の対象がすべて `none`（変更なし）または失敗で終わった, the areka shall 読み直さない（切替の要求 0 件）。
5. The areka shall 読み直しの後の起動の根を今日の切替と同じ `OnGhostChanged`（Reference は自分→自分）とする（裁定 6・`OnBoot` は送らない）。
6. If 読み直しでゴーストを起こせない, then the areka shall 既存の切替の失敗と同じく既定ゴーストへ戻す（`OnBoot` の Reference6＝`halt`・Reference7＝そのゴーストの名前）。既定ゴースト自身を起こせなければ今日の致命の経路（告知・終了コード 1）で終わる。
7. If 更新の途中で対象のゴーストが居なくなった（利用者がメニューや台本で別のゴーストへ切り替えた）, then the areka shall 走っている更新をファイルの上では最後まで進め（全か無かの保証のまま）、以後のイベントと総括を送らず（0 件・1 件ごとに `warn!`）、読み直しをしない。
8. While 読み直しの途中, when 切替の要求が届く, the areka shall 完了 `ghost-shell-balloon-switch` 要件 1.9 と同じく無視して `warn!` を 1 件残す。
9. The areka shall 更新の確定でエンジンが消せなかった残り（写像中の DLL の古い写しなど）を、その走行が成功したなら「戻せなかった走行の残骸」と区別し、次の走行または次の起動で黙って消す。毎周 `warn!` が出続ける形にしない（残る警告は「戻せなかった走行」だけ）。区別の仕組みは `crates/areka-update/` の中に置く（形は設計で決める）。
10. The areka shall エンジンの `run` を「SHIORI を解放していない」まま呼ぶことを、本仕様の設計の Boundary Commitments と `doc/COMPAT_ARCHITECTURE.md` §8 に記す（完了 `update-engine` の設計の前提条件を本仕様で外す）。
11. The areka shall きれいな終わりの判定（`session_mark_verdict`）の引数と、印を残す理由の語を変えない（足す理由 0 個）。

### Requirement 6: `\![execute,install,url,URL,nar]` は URL から落として `ghost-install` の手続きへ渡す

**Objective:** As a ゴーストの作者, I want 台本から URL を示して `.nar` を入れてもらえること, so that 配布サイトの新しいゴーストや追加シェルを勧められる

#### Acceptance Criteria

1. When 再生中の台本が `\![execute,install,url,URL,種別]` の位置に達し、URL が `http://`／`https://` で始まり、種別が `nar` または省略である, the areka shall URL のファイルを一時フォルダへ落とし、落とし終えたら完了 `ghost-install` の依頼の口（`InstallOrder`・書庫 1 本・出どころ `Script`）へ渡す。以後（`install.txt` の検査・イベント・失敗の知らせ）は `ghost-install` の手続きのまま。
2. If 種別が `feed`・`homeurl`・`ical`・`ssf` またはそれ以外の語である, then the areka shall 何もせず `warn!` を 1 件残す（イベント 0 件）。
3. If URL が `http://`／`https://` で始まらない、または空である, then the areka shall 何もせず `warn!` を 1 件残す。
4. If 取得が失敗する（応答が成功以外・時間切れ・接続できない・大きすぎる）, then the areka shall `error!` を 1 件残し（URL と理由）、インストールの依頼を作らず、インストール系のイベントを送らない（0 件）。
5. The areka shall 取得を UI スレッドの外で行い、取得の間もゴーストの描画・台詞・メニューを止めない。
6. The areka shall 落としたファイルを areka 専用の一時フォルダ（場所は設計で決める・`C:\` 直下や対象のゴーストのフォルダの中には作らない）に置き、同じ一時フォルダにある 7 日より古いファイルを次の取得のときに消す（その場では消さない＝手続きが読み終える時点を知る口が無い）。
7. The areka shall `\![execute,install,path,…]` の今日の振る舞いを変えない（同じ受け口に `url` の腕を足すだけ）。

### Requirement 7: 更新の途中でアプリや Windows が終わるとき、走っている更新を上限つきで待つ

**Objective:** As a 更新の最中にアプリを終えた利用者, I want ゴーストのフォルダが中途半端なまま終わらないこと, so that 終了の操作でゴーストを失わない

#### Acceptance Criteria

1. When 終了の後始末に入った時点で更新の手続きが走っている, the areka shall 既存の終了で待つ口（`exit_wait.rs` の門）でその終わりを上限 3 秒まで待ってからプロセスを終える。
2. When OS のセッションの終了の後始末に入った時点で更新が走っている, the areka shall 更新の待ちと SHIORI の待ちを、後始末に入った同じ時点から数え、合わせて 3 秒を超えない（足し算にしない・今日の口の規則のまま）。
3. If 上限に達した, then the areka shall 待つのをやめて後始末を続け、`warn!` に更新先・対象・作業場所が対象のフォルダの下に残っているかもしれないことを 1 件残す。途中で断たれた走行の残りはエンジンの次の走行の残骸の規則に任せる（確定の前に断たれた残りは黙って消え、確定の途中に断たれた残りは「戻せなかった走行」として警告に出る）。
4. When 終了の後始末に入る, the areka shall 以後、本仕様のイベントを送らず（0 件）、読み直しを始めない（0 件）。
5. When 終了の後始末に入る, the areka shall `\![execute,install,url]` の取得の途中の物を待たずに終え、落としかけのファイルは要件 6.6 の掃除に任せる。
6. The areka shall 更新の途中で終えたこと・上限に達したことを、起動中の印を残す理由にしない（印の判定は今日どおり）。
7. The areka shall 切替・メニューの終了・OS の終了で SHIORI を待つ期限を、今日の値のまま変えない。

### Requirement 8: 記録・台帳・文書が実物と揃う

**Objective:** As a 後続の spec の開発者と利用者, I want 何が更新され何が失敗したかが記録から追え、台帳と説明書が実物と食い違わないこと, so that 次の作業と利用者の手順が誤った前提に立たない

#### Acceptance Criteria

1. The areka shall 手続きの各段（要求を受けた・対象と更新先を解いた〔飛ばした対象も〕・`OnUpdateProcessExec` の応えの有無・エンジンを呼んだ・進捗の各件・締めの知らせ・総括・読み直しの要求）を記録に残し、記録の無い失敗の経路を作らない（0 本）。
2. The 本仕様 shall 本体から `areka-update` を辿れるようにしたその変更と同じコミットで、網羅台帳 `assets.toml` の `descript_install` の「相対パス」（`delete.txt` の行の書式）を実装済みへ動かし、`descript_ghost`／`descript_shell`／`descript_balloon` の `homeurl` を読み手の定義の場所（正典 URL の行 `// ukadoc:` 付き）とともに実装済みへ動かす。
3. The 本仕様 shall 網羅台帳の、本仕様が送る 20 イベント・リソース 2 語（`homeurl`・`useorigin1`）・`updatebutton.caption`（枠の登記と同じコミットで実装済みへ）・`\![updatebymyself…]`・`\![update,更新対象…]`・`\![updateother,…]`・`\![execute,install,url,…]` の行を実物に合わせて更新し、送らない行（`OnUpdateCheck*` 4 語・`OnUpdateResultExplorer`・`OnUpdatedataCreating`／`Created`・`other_homeurl_override`・`\![update,platform]`・`\![execute,createupdatedata]`）は状態を動かさず備考に理由を書く。報告書は生成器で作り直し、`roadmap-draft.md` の本仕様の行の数（`owner_count`）を宛先の実数に合わせる（手で数を直さない・`cargo test -p ukadoc-survey` が緑）。
4. The 本仕様 shall `dist/README.txt` の「今の版ではできません」を挙げる 2 行から「ネットワーク更新」の語を外す（`shell-balloon-switch` の分が残っていれば残す）。「■ 更新のしかた」の本文は `alpha-release-signoff` が書く。
5. The 本仕様 shall `doc/COMPAT_ARCHITECTURE.md` §8 に、要件 10 の裁定のうち正典が沈黙している点を記す。
6. The 本仕様 shall 本番コードが読む環境変数を足さず（0 個）、外部クレートを足さない（0 個。ワークスペースの中の `areka-update` を `crates/areka` の依存に足すことは数えない）。
7. The 本仕様 shall 本番ファイルとテストファイルのどれも 1,000 行を超えさせない（`ghost_session.rs` 701・`emo2_boot/mod.rs` 816・`consumer_ledger.rs` 786 に足す行は各 10 行以内の見込み。新しいテストは兄弟の新しいファイルへ）。
8. The 本仕様 shall 完了時に次を申し送る: `shell-balloon-switch` へ「今のシェル・今のバルーン」の読み替え（要件 1.4 の 3 つを切替後の物にする）と許可表の数 43／`alpha-release-signoff` へ第三者の手順「更新する」（メニュー → 進捗の台詞 → 引っ込んで戻る）と既知の制限（更新オプション・`other_homeurl_override`・URL の `feed`／`homeurl`・落としたファイルは 7 日残る・戻せなかった残りは `.update-work` の下に残る）。

### Requirement 9: 決定論テストと実機確認

**Objective:** As a 開発者, I want 手続きの判断の分かれ目がテストで固定され、https を 1 度は本物の配布サイトで通していること, so that 後続の変更で黙って崩れず、α の第三者が最初に踏む道が確かめてある

#### Acceptance Criteria

1. The 本仕様 shall 偽の取得（ネットへ出ない）と偽の SHIORI で、対象 1 つの更新の 4 経路（差分 0＝`none`・差分あり成功＝`changed`・取得の失敗・MD5 不一致）について、送られるイベントの名前・順・Reference（種別・理由・番号・一覧・失敗理由・ファイル名）を判定する。ゴースト（`OnUpdate*`）とシェル（`OnUpdateOther*`）の両方の名で判定する。
2. The 本仕様 shall `useorigin1` の `1`・`0`・返事なしの 3 通りで番号と件数の読みを判定する。
3. The 本仕様 shall 総括（`OnUpdateResult`・`OnUpdateResultEx`）の Reference の並びと形（OK・NG・件数・理由・ファイル名の有無）を、対象 3 つの成功と失敗の混ざった要求で判定する。
4. The 本仕様 shall 入口 4 つのそれぞれが同じ要求を作ること（`update,shell+balloon` の順・`all`・`updateother` の名前引き）と、要件 1.8 の断り（オプション・知らない対象・`--plugin=` だけ）を判定する。
5. The 本仕様 shall 更新先の解き方（リソースが勝つ・`descript.txt` へ倒れる・どちらも無ければ飛ばす・全部飛ばせば総括 0 件）と、メニューの項目の選べる／選べない（更新先の有無・走っている間）を判定する。
6. The 本仕様 shall `OnUpdateProcessExec` の 2 通り（応えあり＝標準の手続き 0 件・応えなし＝標準へ）と、台本の入口では送らないことを判定する。
7. The 本仕様 shall 二重起動（`executing`・総括 0 件）と、定常でないときの断りを判定する。
8. The 本仕様 shall 失敗理由の表（要件 4.2）を、エンジンの失敗の種類 11 種と取得の失敗 8 種の全部について判定する（値を印字するだけにしない）。取得の部品を用意できないときの `error!` と `connect` も判定する。
9. The 本仕様 shall 読み直しが `changed` のときだけ要求されること・総括の返事の台詞が終わってから要求されること・対象のゴーストが居なくなれば残りのイベントを捨てて読み直さないことを、偽の SHIORI で判定する。
10. The 本仕様 shall エンジンの後片付けの改修（要件 5.9）を、消せないファイルを `old/` に残した走行の後で、次の走行が警告を出さずに消すこと、戻せなかった走行の残骸は今日どおり警告に出ることの 2 通りで判定する（`crates/areka-update/` の兄弟テスト）。
11. The 本仕様 shall `\![execute,install,url]` の受け方（`nar`・省略・他の種別・URL の形）と、取得の成功で依頼が 1 本作られること・失敗で依頼 0 件と `error!` 1 件を、偽の取得で判定する。
12. The 本仕様 shall 終了で待つ口を、実時間を待たない形で判定する（走っていれば上限の内に待つ・上限に達したら `warn!` を残して進む・走っていなければ待たない・後始末の後はイベント 0 件と読み直し 0 件）。
13. The 本仕様 shall kanade の許可表が 43 語・リソースの許可表が 12 語であることを判定する（既存の判定の数 23 を書き換える）。
14. The 本仕様 shall 既存のテストを置き換え無しに消さない。振る舞いが変わる行は新しい振る舞いを固定する形へ書き換える。
15. The 本仕様 shall ネットへ出るテストを常時テストに入れない（実機の一周は `#[ignore]` か手順書）。
16. When 実機で確認する, the 開発者 shall 次を見て `signoff.md` に記録する（emo2 は根へ入れた物をコマンドライン引数なしで起こす＝裁定 17 の読み直さない経路を踏まない）: ⑴ emo2 を起動し、メニュー「ネットワーク更新」→ 配布サイト（https・`homeurl,https://ekicyou.github.io/ghost_dev/emo2/emo2/`）に置いた差分 1 件が入る → emo2 の台詞（`OnUpdateBegin`・`OnUpdateReady`・`OnUpdateComplete` の「更新成功」）→ 台詞の後に引っ込んで戻る（記録に `ghost_switch_done` 相当と読み直し後の `OnGhostChanged`）→ シェル・バルーンの中身も読み直されていること ⑵ もう 1 度更新 → 差分 0 → `OnUpdateComplete` の Reference0＝`none`（「更新無し」の台詞）→ 読み直さない ⑶ `homeurl` の無いシェルが飛ばされ、総括にゴーストとバルーンだけが載る（emo2 のバルーンの更新先は別のサイト〔emo-gs〕なので、その中身次第でバルーンが更新される・失敗する。予期しない結果は記録して開発者へ） ⑷ 配布サイトに置けない場合はローカルの http で ⑴⑵ を行い、https が未確認のままであることを既知の制限へ申し送る。
17. When 実機で確認する, the 開発者 shall 記録の水準をイベントの送出・更新先の解決・判断の分かれ目が見える所まで開ける（`RUST_LOG` にエンジンと本仕様のモジュールを `debug` 以上で）。

### Requirement 10: 裁定（暫定・要件ディスカッションで確定）

**Objective:** As a 開発者, I want 答えで作業が変わる点が、推奨案と理由つきで並んでいること, so that 要件ディスカッションで覆すか決めるだけで済む

#### Acceptance Criteria

1. The 本仕様 shall **議題 ⑴（暫定）: 進捗の台詞と SHIORI の解放**を「(b) ゴーストを生かしたまま更新し、終わってから読み直す」とする（要件 5）。理由: SSP と同じ振る舞い。確定は写像中の DLL でも通る（較正済み）。詰まるのは後片付けだけで、要るエンジンの改修は「成功した確定の残りを残骸と区別する」1 点（`crates/areka-update/` の中だけ・誰とも共有しない）。(a)「`run` を 2 段に割る」はエンジン改修 2〜3 タスク＋確定の間ゴーストが消える。(c)「台詞を後でまとめて出す」は正典の順序を崩す。(d)「最後の照合の通知の中で降ろす」は背景スレッドから UI の `&mut World` が要り成り立たない。**代償**: 完了 `update-engine` の前提「SHIORI を先に解放している」を外して呼ぶ（要件 5.10）。SHIORI が更新対象のファイルを降ろすときに書き戻す作りなら、その 1 ファイルは更新の後で SHIORI の値に戻る（作者の側の問題・SSP も同じ・既知の制限に書かない）。
2. The 本仕様 shall **議題 ⑵（暫定）: ゴーストが `OnUpdateFailure` に応えないとき areka 自身が告知するか**を「しない」とする（要件 4.1・4.7）。理由: 09-26 の方針「メッセージボックスは無粋・失敗は既定ゴーストの台詞で」。emo2 の辞書は `OnUpdateFailure` を持つ。
3. The 本仕様 shall **議題 ⑶（暫定）: `\![execute,install,url]` を α に入れるか**を「入れる（`nar` と省略だけ）」とする（要件 6）。理由: brief の Scope に In で挙がっている。`ghost-install` の依頼の口が 1 つで済み、触るのは `install_cue.rs` の `url` の腕と取得だけ（+1〜2）。省略を `nar` と読むのは、Content-Type と中身からの自動判別を持たないため（α 後に `feed`／`homeurl` を足すときに判別も足す）。
4. The 本仕様 shall **議題 ⑷（暫定）: 実機の更新先**を「emo2 の配布サイト（`https://ekicyou.github.io/ghost_dev/emo2/emo2/`）に差分 1 件を置いて 1 周」とする（要件 9.16）。サーバ側の中身（`updates.txt` と差分）を置くのは開発者の手。置けなければローカルの http で回し、https は未確認のまま既知の制限へ。**開発者に確かめること**: 差分 1 件を置いてよいか。
5. The 本仕様 shall **議題 ⑸（暫定）: `OnUpdateProcessExec` を α に入れるか**を「入れる（メニューからだけ・Reference0＝`manual`）」とする（要件 1.12・1.13）。理由: `ghost-install` が `RaiseEvent` に応えの返事を足したので kanade の改修 0。台本の入口から送らないのは、正典の Reference0 に `script` が無く、台本は自分で手続きを差し替えられるため。
6. The 本仕様 shall **議題 ⑹（暫定）: 読み直しの後の起動の根**を「`OnGhostChanged`（自分→自分・今の `boot_root` のまま）」とする（要件 5.5）。理由: `BootOrigin` に「読み直し」を足すと kanade の `boot.rs`・`change.rs` と完了 `ghost-shell-balloon-switch` の再検証に触る。正典の `\![reload,ghost]` は「SHIORI イベントが発生しない」と書くが、areka の読み直しはプロセスを跨がず SHIORI を載せ直すので起動の根の知らせが要る。完全形は α 後。
7. The 本仕様 shall **暫定の確定 7: `homeurl` の無い対象**を「飛ばす（イベント 0・総括に載せない・`warn!`）」とし、今のゴーストに無ければメニューの項目を選べなくする（要件 1.3・1.10）。理由: emo2 のシェルに `homeurl` が無く、失敗にすると毎回 NG が 1 つ出る。台本の入口で全部飛ばされたときに知らせが 0 件なのは記録で補う。**採らなかった案**: 名指しの対象（`\![update,shell]`）だけ `paramerror` で失敗にする（分岐が増える割に見える差が小さい）。
8. The 本仕様 shall **暫定の確定 8: 更新オプション**を「受けない（`warn!` 1 件・要求を作らない）」とし、`OnUpdateCheck*` 4 語を α 後にする（要件 1.8）。理由: `checkonly` は差分を決めた時点で止める必要があり、エンジンの `run` に途中で止める口が無い。`testonly`・`recovery` も同じ。黙って普通の更新に読み替えると「チェックだけのつもりが更新された」になるので、要求ごと断る。
9. The 本仕様 shall **暫定の確定 9: 二重起動**を「`OnUpdateFailure`（Reference0＝`executing`）を 1 回・総括 0 件」とする（要件 1.15）。理由: 正典の失敗理由に `executing` が在る。始まりの知らせ無しに失敗の知らせだけを送るのは、始まっていない更新に始まりを送らないため。
10. The 本仕様 shall **暫定の確定 10: 失敗理由の語**を要件 4.2 の表とし、正典に語が無い輸送の失敗（名前解決・接続・TLS・大きすぎ・その他）には areka の短い語（`dns`・`connect`・`tls`・`toolarge`・`http`）を送る。理由: `timeout` に寄せると「タイムアウトした」と嘘の台詞になる。辞書が知らない語は既定の台詞へ落ちるだけ。
11. The 本仕様 shall **暫定の確定 11: `other_homeurl_override`**を α では送らない（`vocabulary-only` のまま・備考に理由）。理由: Reference0〜4 付きの照会で、`ResourceQuery` に Reference の欄が無く kanade の `msg.rs`・`actor.rs`・`schedule/mod.rs` に手が入る。SSP 2.5.33 の拡張で、検体 4 体の辞書に読み手は 0。
12. The 本仕様 shall **暫定の確定 12: `OnUpdateOtherBegin` の Reference0**を「対象の `name`」（シェル・バルーンの `descript.txt` の `name`）とする（要件 2.4）。理由: 正典は「ゴースト名。」と書くが `OnUpdateResultEx` が対象の名前を持つのと揃え、辞書が「何を更新しているか」を言える。
13. The 本仕様 shall **暫定の確定 13: 読み直しの時機**を「総括への返事の台詞が終わってから・`changed` が 1 つでもあるときだけ・終了が始まっていれば読み直さない」とする（要件 5.2〜5.4・7.4）。理由: 読み直しは台詞を切るので、「更新完了」の台詞を利用者に見せてから引っ込める。バルーンだけの変更でも読み直すのは、読み直しの 1 本の道でシェル・バルーンも解き直される（今日の切替の実物）ため。
14. The 本仕様 shall **暫定の確定 14: MD5 の照合の知らせ**を「1 回の通知から `Begin` と `Complete`／`Failure` を続けて 2 件」とする（要件 2.9）。理由: エンジンは照合の結果だけを知らせ、照合は一瞬で終わる。`Begin` を落とすと辞書の返事が 1 つ呼ばれなくなる。
15. The 本仕様 shall **暫定の確定 15: `\![execute,install,url]` の失敗**を「`error!` 1 件・イベント 0 件」とする（要件 6.4）。理由: 正典の `OnURLDropFailure` は投げ込みの系で「他では発生しない」と書き、インストールの失敗理由の語に取得の失敗は無い。落とし場所は OS の一時フォルダの下の areka 専用の場所（`C:\` 直下と対象のフォルダの中は不可）で、7 日の掃除は次の取得のとき。
16. The 本仕様 shall **暫定の確定 16: 更新中の切替**を「更新は最後まで進め、残りのイベントと読み直しを捨てる」とする（要件 5.7）。理由: 途中で止める口がエンジンに無く、全か無かの保証はファイルの上で守られる。別のゴーストへ進捗を送るのは誤り。
17. The 本仕様 shall **暫定の確定 17: コマンドライン引数で始めたゴーストの読み直し**を「しない（`warn!` 1 件）」とする（要件 5.2）。理由: 引数で指したゴーストは根の目録の外（起動の判断のフォルダ名が無い）で、自分自身への切替はフォルダ名で目録を引くため `NotFound` になる。引数の起動は開発者の上書きの形。読み直せるようにするには切替の指定にフォルダの絶対パスを足す改修が要り（`ghost_switch.rs` と完了 `ghost-shell-balloon-switch` の再検証）、益が小さい。実機確認は根へ入れた emo2 で行う（要件 9.16）。
18. The 本仕様 shall **暫定の確定 18: `\![updateother,--shell=名]` と `menu,hidden` のシェル**を「引かない（引けない名前として飛ばす）」とする（要件 1.7）。理由: シェルの目録（`list_shells`）はメニューと同じく隠しシェルを落とす。正典は隠しシェルの更新に触れておらず、SSP の挙動を測って合わせない方針。
