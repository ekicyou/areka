# ギャップ分析: readonly-mode

> 2026-10-23・本ブランチ（main `15d8b0f3` の上・要件の段のコミット `cd3b450e`）のコードを Grep/Read で読んだ結果。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。行数は今の値。
> 決めるのは設計の段と要件討議。ここは事実と選択肢と、答えで作業が変わる点だけを書く。

## 1. 分析のまとめ

- **記憶の 13 種は 1 つの口を通るが、IO の作り手は 1 つではない**: 13 種はどれも `save_scope`（`crates/areka-sylphya/src/persist/mod.rs`）を通る。ただし `PersistIo` の本物 `FsPersistIo`（単位の構造体）を**その場で作る所が本番に 7 か所・5 ファイル**ある（`areka-ghost/src/sylphya_wiring.rs` の `spawn_ghost_sylphya`・`areka/src/boot_resolve.rs` の `read_last`／`save_app`・`roots/mod.rs` の `read_memory`・`roots/edit.rs` の `apply`・`placement/persist.rs` の読み口・`shiori_host.rs` の試しの道）。brief の「`PersistIo` を 1 か所で差し替える」は、そのままでは 1 か所にならない。
- **「走る間は読み返せる」（要件 2.3）は、書かないだけでは満たせない**: 保存は「読んで重ねて書く」（`save_scope` の `read_existing`）で、切替で起こし直したゴーストの書き手・`boot_resolve` の直読み・窓の位置の読み口は、毎回ファイルから読み直す。書かない形は「メモリの上に重ねる層（重ね書きの控え）」にして、読む側もその層を見る必要がある。
- **書かないで済む所は見つかったより少ない**: 書けるかの確かめ `check_boot_dirs`（`write_failure.rs`）は既に「何も作らない」（`areka_user_data::open_dir_for_add`＝物を足す権限でフォルダを開いて閉じるだけ）＝要件 5.1 は今のコードで満たされている。直すのは 5.2（書けなくても止めない）だけ。二重起動の任せ先（SSP の bootunlock・standalone）も areka には無い＝要件 1.3 は今のままで満たされる。
- **ウェーブ C11 の持ち物に当たるファイルがある**: 要件 2.5 は `areka-ghost/src/runtime.rs` の `boot_with_origin` の中の `create_dir_all`（④ の持ち物）に手を入れないと満たせない。IO を値で配る形にすると、さらに `sylphya_wiring.rs`（④）と `placement/persist.rs`（⑥ `dpi-realign-remembered-chain` の持ち物）にも当たる。IO の層の中で読み取りだけを判断する形なら、当たるのは `runtime.rs` の 1 か所だけになる。
- **調べ残し 2 件は閉じた**: install の `failure_word`（`install/judge.rs`）は拒否の 13 種を残らず写し、ワイルドカードの腕が無い（後半の 6 種は `Unsupported`）。wintf の吹き出しの `SystemParametersInfo` は 2 か所とも読むだけ（`SPI_GETMOUSEHOVERTIME`・`SPI_GETNONCLIENTMETRICS`・書き戻しの指定 0）。新しく見つかったものが 2 件ある（§3.7 の `install/pick.rs` と、§3.6 の正典の失敗理由の語 `readonly`）。

## 2. 今の作り（関わる所）

### 2.1 起動オプションと印の渡し方

- `crates/areka/src/cli/parse.rs`（641 行）の `parse` の `Kind::Option` の腕。`"readonly" | "kiosk" | "callghost" | "deletesource"` は `ParseNote::WordPending`（warn）＋まとめの `ignored`（366 行目）。`nobootcheck`・`dontloadmodule` は効く語の腕（350 行目）で `StartupOptions` へ値を立て、`effective` に載る。readonly はこちらの腕へ移すのが自然。
- `--offline-dump` があると、`main` は `opts.offline` を見て書き出しだけ走らせて戻る（`main.rs` 178 行目）。`--option` の語の値は立っても使われない＝要件 1.5 は「`OfflineIgnored` の記録が出る」ことを確かめるだけで足りる（`parse` の `others` に `option` が入る）。
- `StartupOptions`（`cli/mod.rs` 78 行）に欄を足す。先例の `ModulePolicy` は値として `install_boot_context`（`main.rs`）→ `GhostBootInputs::production` → `ghost_boot_options`（`boot_config.rs`）へ流れる。
- `main.rs` は **978 行**（上限 1,000）。印を立てる・記録を出す行は 20 行に収めるか、別のモジュールへ出す必要がある。
- 既にあるプロセスに 1 つの値: `boot_config::exe_location`（`OnceLock`）・`roots::user_root::layout`（`OnceLock<Layout>`）。利用者ごとの置き場の形は `layout()` の最初の呼び出し（`main` の `resolve_boot` → `default_app_profile_dir` の中）で決まって固まる＝読み取りだけの印は `resolve_boot` より前に立っていないと要件 4 に効かない。

### 2.2 記憶の保存の口（sylphya）

- `PersistIo`（`persist/io.rs`）は `read`／`commit` の 2 つだけの trait。`FsPersistIo::commit` は親のフォルダを `create_dir_all` してから「一時ファイル→`fsync`→`rename`」（作業用の `sylphya.toml.tmp` を宛先と同じフォルダに作る）。
- `save_scope` は `read_existing`（今のファイルを読む）→ 重ねる → `commit`。結果は `PersistOutcome::{Saved, Degraded(PersistFailure)}` の 2 つ。
- `PersistOutcome::Saved` を網羅の `match` で読むのは `boot_resolve.rs` の 3 か所（`write_session_mark`・`write_switch_drop`・`clear_session_mark`。どれも成功の info「書きました」「消しました」を出す）。`actor.rs` の書き手と `roots/edit.rs` は `Degraded` だけを `if let` で見る。
- 書き手（`actor.rs` の `run_actor`）は 1 体ずつ別のスレッドで、起動のときに `build_initial_image` で全スコープをファイルから読み直す。
- `FakePersistIo`（メモリの中の置き場）は本番では使わない決まり（rustdoc）。形としては「メモリの上に重ねる層」の先例になる。

### 2.3 13 種の表の外の直接の書き込み（数え直しの結果）

本番の道で `std::fs` の書き込み（`create_dir*`・`write`・`rename`・`remove*`・`copy`・`File::create`・`OpenOptions`）を持つファイルを数え直した（テストのモジュール・試験の道具のクレートを除く）:

| ファイル | 何を書くか | この spec での扱い |
|---|---|---|
| `areka-sylphya/src/persist/io.rs` | 記憶の 13 種 | 要件 2 |
| `areka-ghost/src/runtime.rs` | ゴーストの記憶のフォルダを起動のたびに作る（`boot_with_origin` の 621〜637 行目） | 要件 2.5（議題 5） |
| `areka-user-data/src/establish.rs`・`copy.rs` | 利用者ごとの置き場を作る・exe の隣の記憶を写す | 要件 4.1・4.3 |
| `areka-nar/src/install.rs`・`areka/src/install/` | インストール | 要件 7 |
| `areka-update/src/{commit,delete,work}.rs` | ネットワーク更新 | 要件 7 |
| `areka/src/install/fetch_url.rs` | 一時フォルダ `%TEMP%\areka\download` | 要件 7.2（落とし始めない） |
| `areka/src/offline_dump/` | 書き出し | 範囲外（決まり済み） |

ほかに見えたもの（`areka-ghost/src/config.rs`・`shiori_wiring.rs`・`areka-parsers/src/package/resolve.rs`・`shiori-host32-*`・`areka-emo-atlas/src/emo2_golden.rs`）はどれもテストの中の物。**要件の表に足りない書き込みは見つからなかった**（要件の段の数え直しと一致）。

### 2.4 入口の形

- **インストール**: どの入口も `install::submit`（`install/mod.rs`・「依頼を手続きへ渡す唯一の口」）を通る。窓への投げ込み（`input_events/file_drop.rs`）は直に、メニュー（`menu/install_frame.rs` → `desk::pick_and_submit` → 選ぶ画面のスレッド）と台本の `path`（`emo2_boot/install_cue.rs`）は生の要求を窓口へ送り、窓口の `drain` が `submit` を呼ぶ。台本の `url` は `install_cue.rs` の手順 5 で `fetch_url::spawn_download` を起こし、**落とし終えてから** 生の要求を送る＝`submit` で断っても要件 7.2（落とし始めない）は満たせない。
- `install::desk::is_busy` は `busy || queue 非空 || overwrite`。`submit` が積む前に断れば、窓口は「忙しい」にならない（要件 7.6・7.7 は形で満たされる）。
- **ネットワーク更新**: どの入口も `update::submit` → `judge`（`update/mod.rs`）を通る（メニューの `desk::update_current`・台本の `take_raw`・答え待ちの預かりの掛け直し）。`judge` の段の移し（`AwaitingExec`・`Running`）より前で断れば `update::desk::is_busy` は偽のまま。`update/desk.rs`（706 行）に手を入れずに済む。
- 断りの知らせの先例: 二重起動の断り `update::desk::refuse_executing` は `OnUpdateFailure(executing)` を返事なしで送る。インストールの失敗は `install/procedure.rs` が `OnInstallFailure` を Reference0 の語 1 つで送る。

### 2.5 起動中の印

- 印は App スコープの `areka.last.running`。書く（`write_session_mark`）・降ろした直後（`write_switch_drop`）・消す（`clear_session_mark`）はどれも `boot_resolve.rs` の `save_app` → `save_scope`。読むのは `resolve_boot_with` の 1 回（`read_session_mark`）。
- 先例「印を読まず書かず消さない」は `BootContext.argv_session`（`main.rs`・`ghost_switch.rs`・`ghost_session_runtime.rs`・`session_end.rs` の 4 ファイルで見る）。readonly は「読むが書かず消さない」で、argv の扱いとは違う。

### 2.6 利用者ごとの置き場とゴーストの写し

- `areka_user_data::establish_from` は `decide` → `create`（無いときだけ `create_dir_all`）→ `migrate`（`copy_tree_atomic`）→ 記録。読み取りだけなら「在ればそのまま使い、無ければ作らずに隣の形（`Layout.user_root = None`）」にすれば、`app_profile_dir` は exe の隣の `profile\areka` を指す＝要件 4.2・4.3 の推しの形になる。
- ゴーストの写しは `roots/user_root.rs` の `copy_for_write`（「すべてのフックが通る 1 つの口」）だけが書く。呼び手は起動（`writable_ghost`）・切替（`emo2_boot/ghost_switch.rs` の `writable_copy`・`emo2_boot/switch_copy.rs`）・インストールの宛先（`install/destination.rs`）・更新（`update/worker.rs`）。`copy_for_write` の入口で「写さず `dir` を返す」にすれば全部に効く（判断 `plan_copy` は純粋のまま残せる）。

### 2.7 テストの形

- 13 種の行列は `write_failure_sites_tests.rs`（401 行）の `KINDS`（13 個の名前）と、本物の実行系を 1 体起こす `SwitchRig` など。要件 10.1 は同じ表を「書く／書かない」に読み替える。
- テストの根は `temp_path_kit::TempPath`（`target\` の下・呼び出しごとに別の名前）。ほぼ全部のテストが 1 つのテストのプロセスの中で**並んで走る**。プロセスに 1 つの印をテストが立てると、同じプロセスの別のテストへ漏れる（§4 の選択肢の分かれ目）。
- `cli/parse_tests.rs`（674 行）は `readonly` を「まだ扱わない語」として 3 か所で判定している（418・527・610 行目）＝書き換えが要る。

## 3. 要件ごとの対応表（欠け＝Missing／未知＝Unknown／制約＝Constraint）

| 要件 | 今あるもの | 欠け・未知・制約 |
|---|---|---|
| 1.1 語を受ける | `parse` の語の腕 | **Missing**: 効く語の腕へ移す・`StartupOptions` の欄 |
| 1.2 info 1 件 | `main` の `shiori_skipped` が先例 | **Constraint**: `main.rs` 978 行 |
| 1.3 bootunlock・standalone 相当 | areka に二重起動の任せ先が無い | 足す物なし（判定のテストと説明書の文だけ） |
| 1.4 ほかの語・kiosk | 今の腕 | 足す物なし |
| 1.5 `--offline-dump` と一緒 | `OfflineIgnored` | テストだけ |
| 1.6 途中で変えない | — | 起動の前に 1 度だけ決める形にすれば満たされる |
| 2.1 13 種を書かない（作業用の一時ファイルも） | `save_scope` 1 か所・`FsPersistIo` の作り手 7 か所 | **Missing**: 書かない形。**Constraint**: 作り手が ④ と ⑥ の持ち物に散る |
| 2.2 今の記憶を読む | `load_scope` | 足す物なし |
| 2.3 走る間は読み返せる | 書き手の鏡像（同じ書き手の中だけ） | **Missing**: 書き手をまたぐ読み返し（切替・直読み）に重ね書きの控えが要る |
| 2.4 終わりに捨てる | — | メモリの中だけに持てば自然に満たされる |
| 2.5 ゴーストの記憶のフォルダを作らない | `runtime.rs` の `create_dir_all` | **Missing**＋**Constraint**: ④ の持ち物の `runtime.rs` に手が要る |
| 2.6・6.2・6.3 記録の行 | `boot_resolve.rs` の成功の info 3 本・`save_scope` の error | **Missing**: 「書かなかった」を見分ける道（議題 4） |
| 3.1〜3.3 印 | 印の 3 関数・`argv_session` の先例 | 重ね書きの控えなら印も控えへ行き、ファイルは変わらない。記録の文だけが残る課題 |
| 4.1〜4.5 利用者ごとの置き場 | `establish_from`・`copy_for_write` | **Missing**: 作らない・写さない分かれ目と info。**Unknown**: 置き場は在るが `profile\areka` が無く、隣に記憶がある場合（§6 の 3） |
| 5.1 確かめで作らない | `open_dir_for_add` が何も作らない | **満たされている**（brief の「試しに書いて消す」は古い） |
| 5.2 書けなくても起こす | `check_boot_dirs` → `stop_boot` | **Missing**: 読み取りだけなら確かめを飛ばす（または結果を記録だけにする） |
| 5.3 読めなければ記憶なしで起こす | `load_scope` の寛容な読み | 足す物なし |
| 6.1・6.4 告知に入らない・ふつうは変えない | `write_failure` の机 | 書かない形が `Degraded` を返さなければ満たされる |
| 7.1 どの入口のインストールも断る | `install::submit` 1 か所 | **Missing**: 断りの判定。**Unknown**: 選ぶ画面を出す前に断るか（§3.7） |
| 7.2 URL を落とし始めない | `install_cue.rs` の手順 5 | **Missing**: `submit` より手前の守り |
| 7.3 更新を断る | `update::judge` 1 か所 | **Missing**: 断りの判定（`update/desk.rs` に触らずに入る） |
| 7.4 記録 1 件・メッセージボックスなし | `install_order_refused`・`update_refused` の先例 | 新しい判定の値と記録 |
| 7.5 ゴーストへの知らせ | `OnUpdateFailure(executing)`・`OnInstallFailure` の先例 | **Unknown**: SSP で測る（議題 3）。**Constraint**: §3.6 |
| 7.6・7.7 窓口を忙しいにしない・溜めない | `is_busy` 2 つ | 積む前・段を移す前に断れば満たされる |
| 8.1〜8.4 ふだんどおり | — | 書くか書かないかの外を触らない設計なら満たされる |
| 8.5 置き場の並びの編集 | `roots/edit.rs` の `apply`（文脈を先に書き換える） | 重ね書きの控えなら記憶の読み返しも合う |
| 8.6 OS の「最近使った」 | `roots/pick.rs`（`IFileOpenDialog`）・`readme/os_port.rs`（`ShellExecuteExW`） | **Unknown**（§3.7・研究が要る） |
| 9.1〜9.3 説明書 | `dist/README.txt` 61 行目の文 | **Missing**: 文の書き換えと例 |
| 10.1〜10.5 テスト | 13 種の行列・`TempPath` | **Constraint**: プロセスに 1 つの印はテストの間で漏れる（§4） |

### 3.6 正典の失敗理由の語 `readonly`（新しく見つけた）

ukadoc の「ネットワーク更新の失敗理由」には `readonly` という語がある（意味の説明は無い）。完了 `areka-P0-network-update` の要件 4.3 は「正典の語 `too slow`・`artificial`・`readonly`・`virusdetect`・`toomanyredirect`・`parse`・`downloading` を送らない（0 件）」と決め、`update/refs_tests.rs` の `UNSENT_WORDS` と `unsent_seven_words_never_appear` が檻になっている（`doc/COMPAT_ARCHITECTURE.md` §8 にも同じ行）。

- 議題 3 の測りで SSP の readonly が `OnUpdateFailure(readonly)` を送ると分かったら、この spec が新しい決まり（readonly の断りでは `readonly` を送る）を持ち、檻の 7 語を 6 語に改め、§8 の行を直すことになる（完了 spec の文書は直さない）。
- インストールの失敗理由の正典は 3 語（`extraction`・`invalid type`・`unsupported`）で、areka は `fileio` を借りている（`install/judge.rs` の `FailureWord`）。readonly に当たる語は無い。

### 3.7 OS が残すもの（議題 6 の数え直し）

- `roots/pick.rs`（置き場を足すフォルダ選び・`IFileOpenDialog`）: 今の指定は `FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST`。`FOS_DONTADDTORECENT` を足せる。**Research Needed**: この指定は「最近使った項目」への追加を止めるだけで、画面が自分で覚える最後のフォルダ（利用者の設定の中）は止まらない見込み（`SetClientGuid`・`ClearClientData` で何が変わるかを設計で調べる）。
- **`install/pick.rs`（インストールの書庫選び・`GetOpenFileNameW`）が議題 6 の数え上げに無い**。この画面も最後に開いた場所を利用者の設定に覚えうる。読み取りだけならインストールは断るので、**選ぶ画面を出す前に断れば**この残り物は起きない（メニューの項目を灰色にするか、押されたら断りの記録だけにするか＝§6 の 5）。
- `readme/os_port.rs`（説明書を開く・`ShellExecuteExW`・`SEE_MASK_FLAG_NO_UI`）: **Research Needed**: `ShellExecuteExW` そのものが「最近使った項目」へ足すか（足すのは開いた側のアプリか）。areka が止められるのは渡し方の指定だけで、開いたアプリが残す物は止められない（要件 11.6 の推しのとおり説明書に書く）。

### 3.8 調べ残しを閉じた 2 件

- **install の `failure_word` の後半**（`install/judge.rs`）: `RefuseReason`（`areka-nar/src/error.rs` の `refuse_reasons!`・13 種）を残らず写している。`CorruptArchive`・`IntegrityMismatch`・`NameUndecodable` → `Extraction`／`MissingInstallTxt`・`UnsupportedType{None}`・`MissingRequiredKey`・`InvalidDirectoryName`・`TargetGhostMissing` → `InvalidType`／`UnsupportedEntry`・`UnsupportedType{Some}`・`SymlinkEntry`・`UnsafePath`・`PathTooLong`・`CaseCollision` → `Unsupported`。ワイルドカードの腕は無い＝readonly の断りを `areka-nar` の拒否の種類として足すと、ここでビルドが止まって語を選ばせる。断りを areka の側（`install::submit`）に置くなら、この表には触らない。
- **wintf の吹き出しの `SystemParametersInfo`**（`crates/wintf/src/ecs/tooltip/os.rs`）: `read_hover_time` の `SystemParametersInfoW(SPI_GETMOUSEHOVERTIME, …, 0)` と字体の `SystemParametersInfoForDpi(SPI_GETNONCLIENTMETRICS, …, 0, dpi)` の 2 か所だけで、どちらも読むだけ（`SPIF_*` を渡していない）。crates 全体に `SPI_SET*`・レジストリの書き込み・`SHAddToRecentDocs`・ジャンプリスト・アプリの ID の呼び出しは無い。

## 4. 実装の選択肢

### 案 A: IO の層の中で読み取りだけを判断する（重ね書きの控えを sylphya に置く）

- **形**: `areka-sylphya/src/persist/` に「書き込みを控える範囲」と「重ね書きの控え（パス → 中身・メモリの中）」をプロセスに 1 つ持つ。`FsPersistIo::commit` は範囲の中なら控えへ入れて戻り（親のフォルダも一時ファイルも作らない）、`read` は控えを先に見る。`save_scope` は範囲の中なら「書けたか」の判定の手前で道を分け、読み取りだけの記録を残して新しい結果（例 `PersistOutcome::Withheld`）を返す（議題 4 の推し）。
- **触るファイル**: `persist/io.rs`（301）・`persist/mod.rs`（451）・`boot_resolve.rs` の網羅の `match` 3 か所（成功の info を出さない腕）・`runtime.rs` の 1 か所（下の「2.5 の触り方」）。**`FsPersistIo` の作り手 7 か所には触らない**＝`sylphya_wiring.rs`（④）・`placement/persist.rs`（⑥）に当たらない。`shiori_host.rs` の試しの道も自動で守られる。
- **テストの漏れの手当て**: 範囲を「全部」と「この根の下だけ」の 2 形にする（本番は全部・テストは自分の `TempPath` の根だけを登録する）。テストの根は呼び出しごとに別なので、並んで走る別のテストへ漏れない。
- **良い所**: 13 種が本当に 1 か所で止まる（brief の考え）。読み返し（要件 2.3）・印（要件 3）・置き場の並びの編集（要件 8.5）が控えだけで揃う。触るファイルが一番少ない。
- **悪い所**: 最下層のクレートにプロセスに 1 つの状態を足す（今の sylphya はプロセスに 1 つの状態を持たない・「IO はこのシームだけ」の決まりの内ではある）。範囲の登録を忘れたテストは本物を書く（今と同じ）。
- **2.5 の触り方**: (a) `runtime.rs` の `create_dir_all` の前に「控える範囲か」を 1 行で見る。(b) ブロックごと消す（`FsPersistIo::commit` が書くときに親を作る「二重の安全網」と書いてあり、`real_commit_creates_missing_parent_dirs` が固定している＝ふつうの起動でも、最初に書くときにフォルダができる。違いは「何も保存しないゴーストに空のフォルダが残らなくなる」ことだけ。起動の時の確かめ `check_boot_dirs` は在る親までさかのぼるので影響しない。このフォルダが在ることを前提にするテストは見つからなかった）。どちらも ④ と先に話す。④ は同じファイルの `publish_ghost_statics` のあたり（660 行目前後）を触る見込みで、621〜637 行目とは離れている。

### 案 B: IO を値で配る（作り手ごとに選ぶ）

- **形**: `StartupOptions` → `BootContext`／`GhostBootInputs` → `GhostBootOptions`（新しい欄）→ `SylphyaInit.io` と、`boot_resolve`・`roots`・`placement` の直読み・直書きへ「読み取りだけの IO」を配る。控えは `Arc` で共有する。
- **触るファイル**: 案 A の物に加えて `runtime.rs`（`GhostBootOptions` の欄と `boot_with_origin`）・`sylphya_wiring.rs`（④）・`placement/persist.rs`（⑥）・`boot_config.rs`（`ghost_boot_options`）・`ghost_session.rs`・`main.rs`・`roots/mod.rs`・`roots/edit.rs`・`shiori_host.rs`、`GhostBootOptions` を組むテスト全部。
- **良い所**: プロセスに 1 つの状態が無い＝テストは差すだけで漏れない。`ModulePolicy` の値の渡し方と同じ。
- **悪い所**: ④ と ⑥ の持ち物に当たる（ウェーブの並びを崩すか、先に話して分け合う）。`main.rs` の残り 22 行に響く。作り手が 7 か所に散ったまま＝後から足した作り手が readonly を黙って破りうる（要件 10 の目的に逆らう）。

### 案 C: 合わせ技（記憶は案 A・それ以外の守りは値で持つ）

- **形**: 記憶の 13 種と 2.5 は案 A。インストール・更新・URL の取り込み・置き場の並びの編集・書けるかの確かめ・利用者ごとの置き場・ゴーストの写しは、World の資源か起動の値で「読み取りだけ」を持って入口で見る（`install::submit`・`update::judge`・`install_cue.rs` の手順 5・`check_boot_dirs` の呼び手・`copy_for_write`・`establish_from`）。利用者ごとの置き場の形は `layout()` が `OnceLock` で固めるので、ここだけはプロセスに 1 つの値が先例どおり要る（`resolve_boot` より前に立てる）。
- **良い所**: 入口の守りは World ごとに分かれる＝install・update のテストが並んで走っても漏れない。記憶はファイルの手前 1 か所で止まる。
- **悪い所**: 「読み取りだけ」の持ち方が 2 つ（IO の層の範囲と、World／起動の値）になる。両者を同じ起動の値から立てる 1 つの口（例 `crates/areka/src/readonly.rs`）が要る。

### 入口の守りの置き場（どの案でも同じ）

| 守り | 置き場の候補 | 触るファイル（C11 の持ち物か） |
|---|---|---|
| インストール | `install::submit` に新しい判定の値（例 `SubmitVerdict::ReadOnly`） | `install/mod.rs`（117・持ち主なし）。`install/desk.rs` は触らずに済む |
| メニューの書庫選び | `desk::pick_and_submit` の前で断る／メニューを灰色に | `install/desk.rs` か `menu/install_frame.rs`（§6 の 5） |
| URL の取り込み | `install_cue.rs` の手順 5 か `fetch_url::spawn_download` の頭 | `emo2_boot/install_cue.rs`（持ち主なし）か `install/fetch_url.rs` |
| 更新 | `update::judge` の頭に新しい判定の値 | `update/mod.rs`（229・持ち主なし）。`update/desk.rs` は触らずに済む |
| 書けるかの確かめ | `check_boot_dirs` の呼び手（`main.rs`）か中 | `write_failure.rs` か `main.rs` |
| 利用者ごとの置き場 | `establish_from` に「作らない・写さない」の分かれ目 | `areka-user-data/src/establish.rs`（呼び手 `roots/user_root.rs` の `layout`） |
| ゴーストの写し | `copy_for_write` の頭 | `roots/user_root.rs` |

## 5. 規模と危うさ

- **規模: M〜L（12〜16 タスク）**: 守る入口は 8 か所前後で、どれも 1 つの口に寄せられる。足すのは控えの層・判定の値・記録・説明書・テスト（13 種の行列の読み替え・通しの前後比べ・語の読み手）。brief の見立てと同じ。
- **危うさ: 中**: 技術は既知（先例＝`FakePersistIo`・`RootMark::Temporary` の 5 か所の守り・`ModulePolicy`）。危ういのは ⑴ プロセスに 1 つの状態とテストの漏れ ⑵ ④（`runtime.rs`）との先の話 ⑶ 議題 3 の SSP の測りの結果で、送るイベントと完了 spec の檻（§3.6）が変わること。

## 6. 設計の段と要件討議へ渡すこと

### 決めること（答えで作業が変わる）

1. **書かない形の置き場**: 案 A（IO の層の中・作り手 7 か所に触らない）／案 B（値で配る・④ と ⑥ の持ち物に当たる）／案 C（記憶は A・入口は値）。推しは C（記憶は 1 か所で止まり、入口の守りはテストの間で漏れない）。
2. **要件 2.5 の触り方**: `runtime.rs` の `create_dir_all` を「控える範囲か」で包む（1〜2 行）か、ブロックごと消す（ふつうの起動でも空のフォルダを先に作らなくなる＝最初の保存のときにできる）。どちらも ④ `mcp-scope-rect-property` のセッションと先に話す。
3. **置き場は在るが中の `profile\areka` が無く、exe の隣に記憶がある場合**（ふつうなら写す）: 読み取りだけでは、置き場の空の記憶を読むか、隣の記憶を読むか（要件 4.2 と 4.3 の境目）。
4. **書かなかった保存の結果の形**（議題 4 の推しの具体）: `PersistOutcome` に 3 つ目の値を足す（網羅の `match` は `boot_resolve.rs` の 3 か所）か、`Saved` のまま記録だけを分けるか。足すなら `boot_resolve.rs` の成功の info（「起動中の印を書きました」など）を出さない腕ができる。
5. **メニューの「インストール…」と「ネットワーク更新」**: 読み取りだけでは項目を灰色にするか、押されたら断りの記録だけにするか。書庫選び（`install/pick.rs`）は議題 6 の数え上げに無く、画面を出すと OS が最後の場所を覚えうる＝出す前に断る形が要件 8.6 に合う。
6. **（2026-10-23 要件討議で解決: readonly の SSP は `readonly` の語を送らない＝檻は改めない・インストールは何も送らず、更新は成否の欄が空の `OnUpdateResultEx`／`OnUpdateResult` だけ。requirements.md の Introduction の表と要件 7.5）** **議題 3 の測りの結果と正典の語 `readonly`**（§3.6）: SSP の readonly が `OnUpdateFailure(readonly)` を送るなら、この spec が「readonly の断りでは送る」決まりを持ち、`update/refs_tests.rs` の送らない 7 語の檻と `doc/COMPAT_ARCHITECTURE.md` §8 の行を改める。インストールの断りで `OnInstallFailure` を送るなら、正典に readonly の語が無いので語を決める（SSP の測りに従う）。
7. **印の扱いの実装**（議題 1 の推しの具体）: 案 A／C なら印は控えへ行き、ファイルの印は読んだまま残る（`argv_session` の 4 ファイルに触らない）。印の 3 関数の記録の文だけを分ける。別の旗（「印を読むが書かない」）を `BootContext` に足す形は `main.rs`（残り 22 行）と 4 ファイルに響く。
8. **印を立てる場所と `main.rs` の行数**: `resolve_boot` より前に立てる必要がある（`layout()` が固まる前）。`main.rs` に置く行を数行に収め、記録と立て方は新しいモジュールへ出す。

9. **⑥ `dpi-realign-remembered-chain` との同じファイル**（2026-10-23 に ⑥ のセッションから知らせ）: ⑥ は窓の位置の記憶に新しい項目を足すため `crates/areka-sylphya/src/persist/mod.rs`（`PersistKey` と TOML への写し）と `persist/format.rs`（`FormatDoc` の表）に触る（format-version は上げない）。意味の上では当たらない（足す項目も同じ書く道を通り、readonly の「書かない」に覆われる）。当たるのは同じファイルの文字だけ＝後に着地する側が取り込む。設計の前に main を取り込み直し、⑥ が先に着地していればその形の上に書く。

### Research Needed（設計の段で調べる）

- `IFileOpenDialog` の `FOS_DONTADDTORECENT` が止める範囲と、画面が自分で覚える最後のフォルダ（利用者の設定の中）を止める手があるか（`SetClientGuid`・`ClearClientData`）。
- `ShellExecuteExW` が「最近使った項目」へ足すか（足すのは開いた側のアプリだけか）。
- （済・2026-10-23）議題 3: readonly の SSP へ `.nar` を投げ込む・`\![updatebymyself]` を流す・メニューの更新を選ぶ・`\![execute,install,url,…]` を流したとき、SSP が送るイベントと Reference（`OnInstallFailure`・`OnUpdateFailure(readonly)` か・何も送らないか）。SSP をもう 1 つ起こすので開発者に一言断る。
- `FOS_*` と同じく、`GetOpenFileNameW` に「最近使った」を止める指定（`OFN_DONTADDTORECENT`）があるが、§6 の 5 で画面を出さないと決めれば要らない。

## 7. 次の手

要件討議（`kiro-requirements-discussion`）で §6 の 1〜8 を決め、議題 3 の SSP の測りを済ませてから、`/kiro-design readonly-mode` で設計へ進む。
