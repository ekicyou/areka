# ギャップ分析: areka-P0-baseware-root-list

> 実測は 2026-10-16・ブランチ `claude/areka-P0-baseware-root-list-455a14`（`8b9b8d15`＝main `1e6590d8` に spec の初期化を足しただけ）。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。行数は `wc -l` の実数。
> この文書は分析と案を並べるだけで、決めない。決めることは末尾の「設計の判断の項目」に番号で挙げる。

## 1. 分析のまとめ

- **土台はほぼ揃っている。** 一覧の項目（`GhostEntry`・`BalloonEntry`）は今でも自分のフォルダの絶対パス（`dir`）を持つ。切替・メニュー・`\![open,explorer,…]`・`updateother` の多くは「一覧を読んで項目の `dir` を使う」形なので、一覧を「並び全体の和」に替えれば、呼び手の多くは型の置き換えで済む。
- **「根＋フォルダ名」でパスを組み直している所が残る。** `boot_resolve.rs` の `resolve_ghost`・`resolve_balloon`（決めた場所を `root.ghost_dir(folder)` で組む）、`main.rs` の既定ゴーストの場所（`ctx.root.ghost_dir(DEFAULT_GHOST_FOLDER)`）、`areka-nar` の宛先（`request.root.join("ghost"|"balloon")`）。ここは「項目が持つ場所」へ寄せる作り替えが要る。
- **新しく要る物**: 根の並びの値型と和の列挙・隠れの記録、並びの決め方（環境変数の `;` 区切り・アプリの記憶・既定）、アプリの記憶に「並び」を持つ入れ物（今の鍵はどれも 1 つの値）、メニューの 2 項目（今の枠は 7 つ固定）、フォルダ選択の画面（今あるのはファイル選択だけ）、告知の文面の可変の行数。
- **いちばん大きい落とし穴は `areka-nar` の確定の仕方。** 作業フォルダ `.nar-work` を根に 1 つ掘り、`rename` で入れ替える（同じボリューム前提）。要件 5.5 の「ゴーストと同梱のバルーンが別の根へ入る」は、根が別のドライブだと `rename` が失敗する。`areka-nar` の公開面（`InstallRequest`）に手を入れる必要がある。
- **規模と危険度**: L（16〜20 タスク・見込みどおり）／中。新しい技術は IFileOpenDialog だけで、ほかは今ある型の延長。ただし呼び手 12 ファイル＋`areka-nar`＋`areka-sylphya` の記憶、テストの組み立て約 40 ファイルに波及する。

## 2. 今のコードの調べ

### 2.1 根と一覧（`crates/areka-ghost/src/catalog.rs`・414 行）

- `BasewareRoot { dir }`: `ghost_store`・`balloon_store`・`ghost_dir(folder)`・`balloon_dir(folder)`。実在の検査はしない。
- `list_ghosts(&BasewareRoot)`・`list_balloons(&BasewareRoot)`: 格納フォルダの直下 1 段を `subdirs` で走査し、`ghost/master/descript.txt`（ゴースト）・`descript.txt`（バルーン、`type` が `balloon` 以外は除く）を持つものだけを返す。並びは `(フォルダ名, パス)` の `sort()`＝フォルダ名のバイト順。格納フォルダが無ければ 0 件（`NotFound` は黙って空・ほかの読めない失敗は `warn!`）。
- 項目は `dir: PathBuf`（絶対パス）と `identity: Identity`（`folder`・`name` ほか 7 項目）を持つ。**「どの根か」の欄は無いが、`dir` の親の親が根になる。**
- `list_shells(ghost_dir)` はゴーストのフォルダ基準で根に依らない（要件 3.6 はそのまま満たす）。
- 公開: `crates/areka-ghost/src/lib.rs` が `BalloonEntry, BasewareRoot, GhostEntry, Identity, ShellEntry` を再輸出。

### 2.2 根の決め方（`crates/areka/src/boot_config.rs`・503 行）

- `resolve_root_from(env, exe) -> Result<(PathBuf, RootSource), RootError>`: 純粋な判断。`AREKA_ROOT` があればそれ（空も「設定あり」）、無ければ `exe_location()` の親。`std::path::absolute` で絶対化し `is_dir()` を検査。`canonicalize` は意図して使わない（`\\?\` 接頭辞を持ち込まないため・完了 `release-package-versioned`）。
- `RootSource { EnvVar, ExeDir }`・`RootError { ExeLocationUnavailable, NotADirectory { dir, source } }`。
- `resolve_boot_from(root, args, app_profile_dir, pick)`: 根 → ゴースト → バルーンを決め、`BootResolved`（5 つ組の最後が `BasewareRoot`）を返す。ゴーストの一覧は `list_ghosts(&root)` のフォルダ名の列（`Vec<String>`）に落としてから鎖へ渡す。
- `resolve_balloon_for_ghost(root, ghost_dir, pick)`: 起動と切替の両方が使うバルーンの鎖の入口。`list_balloons(root)` を渡す。
- `BootContext`（bevy の `Resource`）: `root: BasewareRoot`・`app_profile_dir`・`helper_exe`・`argv_session`・`current`。据え付けは `main.rs` の 1 か所（`BootContext { root, … }`）。**今は起動後に書き換える人がいない。**
- `default_app_profile_dir()`: `AREKA_PROFILE_DIR` → `<exe のフォルダ>\profile\areka`。

### 2.3 起動の鎖（`crates/areka/src/boot_resolve.rs`・609 行）

- `GhostInputs { root, argv, memory, listed: &[String] }`・`BalloonInputs { root, …, listed: &[BalloonEntry] }`。
- `resolve_ghost`・`resolve_balloon` の中の `at` 閉包が `inputs.root.ghost_dir(folder)`／`balloon_dir(folder)` で場所を**組み直す**。`warn!` と `NoGhost { ghost_store }`・`NoBalloon { balloon_store }` は 1 つの格納フォルダを載せる。
- フォルダ名の突き合わせ（`find`・`find_balloon`・`find_balloon_by_name`）は `==`＝**大文字小文字を区別する**。
- `running_name(root, ghost)`: `list_ghosts(root)` を引き直して `name` を返す。呼び手は `main.rs` の `boot_first_ghost` と `emo2_boot/ghost_switch.rs` の定常到達（`record_steady_memory` の近く）の 2 つ。
- 既定の定数 `DEFAULT_GHOST_FOLDER`＝`emo2`・`DEFAULT_BALLOON_FOLDER`＝`StayseeBalloon` はこのファイルだけが持つ。
- App スコープへの直接の書き込み（`write_session_mark`・`write_switch_drop`・`clear_session_mark`）には、**「実行系が 1 つも動いていない間だけ UI スレッドから直接書く。動いている間はそのゴーストの記憶の書き手を通す（保存は読んで重ねて書くので、同時に書くと片方が消えうる）」** という約束がコメントで書かれている。

### 2.4 根を 1 つと決め打ちしている本番の呼び手（数え直し）

| ファイル | 定義 | 今の使い方 | 和に替えたときの手当て |
|---|---|---|---|
| `boot_config.rs`（503） | `resolve_root_from`・`resolve_root`・`resolve_boot_from`・`resolve_balloon_for_ghost`・`BootContext.root`・`BootResolved` | 根を 1 つ決めて一覧を引く | 並びの決め方へ作り替え・型の置き換え |
| `boot_resolve.rs`（609） | `GhostInputs.root`・`BalloonInputs.root`・`resolve_ghost`・`resolve_balloon`・`running_name`・`NoGhost`・`NoBalloon` | 根＋フォルダ名で場所を組む | 項目の `dir` を使う形へ（ゴーストの `listed` を `&[String]` から項目の列へ） |
| `main.rs`（**956**） | `boot_first_ghost`（`running_name(&ctx.root, …)`）・後始末の告知（`ctx.root.ghost_dir(DEFAULT_GHOST_FOLDER)`）・`BootContext { root, … }` の据え付け | 3 か所 | 型の置き換えと引き当ての関数の呼び出し（行数はほぼ増えない見込み） |
| `alert.rs`（241） | `AlertScene::{RootMissing, GhostMissing, BalloonMissing}`・`alert_text`（本文は `[String; 3]` の 3 行固定）・定数 `ROOT_BY_ENV` | 根・格納フォルダを 1 つ綴る | 行数を可変に・複数の根を並べる |
| `emo2_boot/ghost_switch.rs`（**902**） | 切替の受理（`list_ghosts(&ctx.root)`）・`switch_to_default`（既定を一覧から引く）・`boot_into`（`c.root.clone()` → `resolve_balloon_for_ghost`）・定常到達の印（`running_name`） | 4 か所 | 型の置き換え（`resolve_switch_target`・`resolve_special_name` は項目の列を受けるのでそのまま） |
| `emo2_boot/shell_balloon_resolve.rs`（217） | `balloon_candidates(root)` | 根 1 つのバルーン | 和を受ける |
| `emo2_boot/shell_balloon_switch.rs`（442） | `SwitchContext.root`・`balloon_candidates(&ctx.root)` | 同上 | 型の置き換え |
| `menu/ghost_frame.rs`（74） | `ghost_frame_item`（`list_ghosts(&ctx.root)`） | 一覧と印 | 和を受ける |
| `menu/balloon_frame.rs`（45） | `balloon_frame_item`（`list_balloons(&ctx.root)`） | 同上 | 同上 |
| `install/desk.rs`（534） | `record_installed`（`list_ghosts(root)`・大小を区別せず突き合わせ）・`ghost_facts`（`GhostFacts.root = ctx.root.dir()`） | 入れる先の根＝1 つ | 背景のスレッドへ並びの写しを渡す・宛先の決め方 |
| `install/overwrite.rs`（277） | `run_between`（`InstallRequest { root: ctx.root.dir() }`） | 動いているゴーストへの上書き | 見えている方の根を宛先に |
| `install/procedure.rs`（477） | `installed_items`（`list_balloons(&BasewareRoot::new(facts.root))`） | 入れた後の名前引き | 和を受ける |
| `install/worker.rs`（259） | `InstallWorker.root`・`install_elsewhere`（`InstallRequest { root }`） | 背景のスレッドが根を 1 つ持つ | 宛先の決め方（下の 2.6） |
| `update/desk.rs`（703） | `Here.root`・`resolve`（`updateother` の `--balloon=` だけ `list_balloons(&here.root)`） | 1 か所 | 和を受ける |
| `readme/opener.rs`（401） | `OpenContext.baseware`・`named_folder`（ゴーストは `resolve_switch_target(list_ghosts(root))`・バルーンは `balloon_candidates(root)`） | 1 か所 | 和を受ける |
| `crates/areka-nar/src/{install.rs, plan.rs}` | `InstallRequest { root, target_ghost }`・`body_placement`・`companion_placement`・`existing_target_ghost`・`WorkArea::create(root)` | 宛先を 1 つの根の下に組む | **公開面の変更が要る**（下の 2.6） |

- 要件の Introduction は「12 ファイル」と数える。上の表のとおり、`install/worker.rs`（根を背景のスレッドへ運ぶ所）と `areka-nar` の 2 ファイルも宛先の決め方で手が入る。
- `mcp/resolve.rs` の `g.root` は動いているゴーストの一覧の項目の場所で、目録の根ではない（触らない）。
- 告知の文面を除く `AREKA_ROOT` の読み口は `boot_config.rs::resolve_root` の 1 か所だけ。結合テスト `crates/areka/tests/smoke_boot_loop_exit.rs` が `AREKA_ROOT` に根を 1 つ渡す（要件 1.6 でそのまま動く）。

### 2.5 アプリの記憶（`crates/areka-sylphya/src/persist/`）

- `PersistKey` は 8 種（`WindowPos{scope,axis}`・`BalloonOffset{scope,axis}`・`BootCount`・`VanishCount`・`LastGhost`・`LastBalloon`・`LastShell`・`LastRunning`）。どれも値は `String` 1 つ。`Copy` の列挙型。**並びを持つ鍵は無い。**
- TOML の形（`format.rs` の `FormatDoc`）: `format-version = 1`・`[window."ID"]`・`[balloon-offset."ID"]`・`[boot]`・`[vanish]`・`[last]`。値はすべて TOML の文字列。配列は使っていない。
- `save_scope` は「読んで重ねて書く」。`apply_entry` は値を置くだけで、**鍵を消す口は無い**（`LastRunning` は空文字を書いて「無し」と読む）。番号付きの族（`[window."0"]` の型）で並びを持つと、並びを短くしたときに残りを消せない。
- `PersistKey::to_canonical_key` は鏡像の dotted 区画への投影で、`parse_dotted` との往復を檻（`canonical_key_round_trips_with_to_canonical_string`）が固定する。新しい鍵を足すとこの投影も決める必要がある（`index(n)`・`scope(n)` の書式は `key.rs` にある）。
- 動いているゴーストの記憶の書き手は `SylphyaPublisher::persist_put(scope, Vec<(PersistKey, String)>)`（`actor.rs`）。App スコープの根は `GhostBootOptions.app_profile_dir` で渡っている。
- 後続 `dev-folder-alias` も「エイリアスの表を根の並びと同じ App の記憶に置く」と brief に書く。入れ物の形はその表も載る形にしておくと後続が楽。

### 2.6 インストール（`areka-nar` を含む）

- `areka-nar::InstallRequest { root: &Path, target_ghost: Option<&str> }`。`plan.rs` の `body_placement` はゴースト・シェル・サプリメントを `root/ghost/…`、バルーンを `root/balloon/…` に組む。同梱のバルーン（`companion_placement`）も同じ `root/balloon/…`。シェル・サプリメントの宛先ゴーストの実在は `root/ghost/<target>` で確かめる。
- 「すでにある」の判定は `install.rs` の `placement.destination.is_dir()`（宛先そのものの在否）。
- 確定は `WorkArea::create(request.root)` で `<根>\.nar-work\` に作業フォルダを 1 つ掘り、配置ごとに `rename` で入れ替える（`commit_all`・`commit_one`・失敗したら `roll_back` で逆順に戻す）。コメントに「入れ替えが `rename` で済むよう根と同じボリュームに置く」とある。
- **ギャップ**: 要件 5.1〜5.5 は配置ごとに宛先の根が変わりうる（ゴーストは見えている方か先頭、同梱のバルーンは見えている方か先頭、シェル・サプリメントは宛先ゴーストのある根）。今の公開面は根を 1 つしか受けず、作業フォルダも 1 つなので、根が別のドライブだと 2 つ目の配置の `rename` が失敗する（Windows の「別のディスクドライブに移動できません」）。巻き戻しもボリュームごとに閉じている必要がある。
- `install/desk.rs::record_installed`・`install/judge.rs` などインストールの側の突き合わせは `eq_ignore_ascii_case`＝**大小を区別しない**。起動・切替（`==`）と食い違っている（今は根が 1 つで、同じ根に大小違いのフォルダは並べないので表に出ない）。
- 背景のスレッド（`InstallWorker`）は窓口に `DeskAsk::Facts` で尋ねて `GhostFacts.root` の写しを受け、書庫 1 つの間それを使う。並びに替えても「尋ねたときの写し」で動く形は保てる。

### 2.7 メニュー（`crates/areka/src/menu/`）

- 枠は `Frame` の 7 種で、`Frame::ORDER: [Frame; 7]`・`MenuRegistry.slots: [Option<Supplier>; 7]`・`captions::FRAME_CAPTIONS`（枠ごとに SHIORI のリソース名・既定名）が 7 に固定。並びを決めるのは `Frame::ORDER` だけ。テスト（`mod_registry_tests.rs`・`captions_tests.rs`・`plan_tests.rs`）も 7 を前提にしている。
- 1 つの枠の供給関数は `MenuItem` を 1 つ返す。葉（`ItemBody::Action`）かサブメニュー（`ItemBody::Submenu(Vec<MenuItem>)`）。選べない形は `enabled: false`（灰色）。供給関数はメニューを出すたびに呼ばれる（`MenuRegistry::snapshot`）＝並びを変えた直後のメニューから新しい一覧が出る（要件 8.2 は今の作りで満たせる）。
- 「インストール…」（`install_frame.rs`）: 葉 1 つ。選べるかは `desk::can_pick`（窓口が在る・終了が始まっていない・選ぶ画面が出ていない）。
- 新しい 2 項目に当たる SSP の SHIORI リソース名（`*.caption`）は見当たらない（ukadoc のリソース一覧に「根」の概念が無い）。`caption_resource: None` で既定名だけにする形は、ゴースト枠の子項目が既にそうしている。

### 2.8 選ぶ画面と UI スレッド

- `install/pick.rs::pick_archive`: `CoInitializeEx(COINIT_APARTMENTTHREADED)` → `GetOpenFileNameW`（持ち主の窓なし）→ `CoUninitialize`。呼んだスレッドを塞ぐので、`desk::start_pick` が短命のスレッド `install-pick` を起こし、結果を窓口の通り道（`raw_tx`）で UI スレッドへ返す。2 つ目を出さない旗（`picking`）・`AREKA_NO_ALERT` のときは出さずに取り消しと同じ扱い。**UI スレッドは塞がない**（要件 6.3 の手本）。
- フォルダ選択（`IFileOpenDialog`＋`FOS_PICKFOLDERS`）の使用例はリポジトリに無い。`CoCreateInstance` の使用例は `wintf/src/com/` などにある。
- 根の `Cargo.toml` の `[workspace.dependencies.windows]`（版 0.62）の機能に `Win32_UI_Shell` と `Win32_System_Com` は**有効**。`crates/areka/Cargo.toml` は `Win32_UI_Controls_Dialogs` を足している。→ 依存を足さずに済む見込み（要件 11.5）。

### 2.9 大文字小文字（フォルダ名の突き合わせ）

- 一覧の並び: バイト順（大小を区別）。
- 起動の鎖・切替・`\![open,explorer,…]`・`updateother`: `==`（区別する）。
- インストールの記録・判定・上書きの宛先の突き合わせ: `eq_ignore_ascii_case`（区別しない）。
- Windows のファイルシステムは大小を区別しないので、根が 1 つなら同じ格納フォルダに `Emo2` と `emo2` は並ばない。**根が 2 つになると、上の根の `Emo2` と下の根の `emo2` が並びうる。** 隠れの判定を大小の区別ありで行うと和に 2 つ並び、記憶の `emo2` は下を引く一方、インストールの「すでにある」は上を指す、という食い違いが起きる。

### 2.10 1,000 行の目安

| ファイル | 行数 | 見込み |
|---|---|---|
| `crates/areka/src/main.rs` | 956 | 手が入るのは 3 か所の置き換え。並びの解決・記録・告知は兄弟のファイルへ（要件 11.6） |
| `crates/areka/src/emo2_boot/ghost_switch.rs` | 902 | 4 か所の置き換え。足す分があれば兄弟へ |
| `crates/areka/src/install/desk.rs` | 534 | 宛先の決め方を足す余地あり |
| `crates/areka/src/update/desk.rs` | 703 | 1 か所の置き換え |
| `crates/areka-ghost/src/catalog.rs` | 414 | 並びの型・和・隠れ・引き当てで 150〜250 行増の見込み。兄弟（`catalog_roots.rs` など）に出す案もある |
| `crates/areka-sylphya/src/persist/format.rs` | 533 | 並びの入れ物の形しだい |
| `crates/areka-nar/src/install.rs` | 573 | 作業フォルダを根ごとにするなら増える |

### 2.11 テストの置き方と波及

- テストは実装の兄弟ファイル `*_tests.rs`（`#[path = …]`）と `*_test_support.rs`。根は一時フォルダに組む（`catalog_test_support.rs`・`ghost_switch_test_support.rs`・`opener_test_support.rs` ほか）。要件 12.7 のとおり `target\` の下に置く決まりは既存のテストの支えに合わせて確かめる。
- `BootContext { root, … }` を組むファイル: 17（本番 2・テスト 15）。`BasewareRoot::new`・`GhostInputs {`・`BalloonInputs {` を組むファイル: 23。型を替えると約 40 ファイルが動く。「根 1 つから並びを作る口」を用意すれば、テストの直しは 1 行の置き換えで済む。
- 環境変数の読み口は今も純粋な関数（`resolve_root_from(env, exe)`）に値を注入する形（要件 12.7 の型がすでにある）。

## 3. 要件と今のコードの対応

| 要件 | 今ある物 | ギャップ |
|---|---|---|
| 1 根の並びと決め方 | `resolve_root_from`（環境変数 → exe の隣・注入できる純粋な形）・`RootSource`・`exe_location` | **Missing**: `;` 区切り・空の項目・重複・アプリの記憶の並び・記憶の読めない項目・0 項目・「実行ファイルの隣」の印。**Constraint**: 記憶の入れ物に並びの鍵が無い（2.5）。**Unknown**: 同じフォルダの判定の仕方（大小・末尾の区切り・`..`・短い名前・ジャンクション） |
| 2 上から探す | 項目が `dir` を持つ・`subdirs` の走査 | **Missing**: 和と隠れ・隠れの記録・引き当ての関数 1 つ。**Unknown**: 隠れの単位（フォルダが在ること／ゴーストとして列挙されること）・大小の扱い（2.9） |
| 3 一覧は和 | 一覧の呼び手の多くは項目の `dir` を使う | **Missing**: 和の列挙。`boot_resolve` の `at` 閉包・`main.rs` の既定ゴーストの場所は組み直しをやめる作り替え。項目に「どの根か」の欄（要件 3.4・後続の欄）は無い |
| 4 記憶の鍵はフォルダ名 | 鍵はもともとフォルダ名だけ | ほぼ 0。`last_ghost_not_found` などの `warn!` に載る格納フォルダが 1 つ（記録の中身を並びに合わせるか） |
| 5 インストール先 | `areka-nar` の宛先の組み方・`is_dir` による「すでにある」 | **Missing**: 配置ごとの宛先の根。**Constraint**: 作業フォルダ 1 つ＋`rename`（同じボリューム前提・2.6）。`GhostFacts.root` が 1 つ |
| 6 根を追加… | `install/pick.rs` の別スレッドで出す型・`can_pick` の旗 | **Missing**: フォルダ選択の画面・結果を UI スレッドへ返す受け口・並びへの追加と記憶への書き込み。**Research Needed**: windows 0.62 で `IFileOpenDialog` が今の機能だけで組めるか |
| 7 根を外す ▸ | サブメニュー・`enabled: false` | **Missing**: 枠（今は 7 固定）・並びからの削除 |
| 8 効き方 | 供給関数はメニューを出すたびに呼ばれる | **Missing**: `BootContext` の並びを書き換える口・記憶へ書く経路（2.3 の「動いている間は書き手を通す」約束）・`AREKA_ROOT` のときの灰色 |
| 9 告知 | `AlertScene`・`alert_text`（3 行固定）・`error!` の対 | **Missing**: 複数の根・複数の格納フォルダを並べる可変の行数。`RootError`・`NoGhost`・`NoBalloon` の中身を並びに |
| 10 説明書 | `dist/README.txt`（264 行・根の語は使っていない）・`doc/COMPAT_ARCHITECTURE.md` | 書き足し |
| 11 今を守る | 決定論テストの檻が既にある（起動の鎖・切替・メニュー・インストール） | 並び 1 つのときは今の檻がそのまま通ることを確かめる |
| 12 テスト | 一時フォルダの根・純粋な判断の注入口 | 新しい判断（並び・隠れ・和・宛先・メニューの判断・告知）の檻を足す |

## 4. 実装の進め方の案

### 案 A: 今の型を広げる（`BasewareRoot` を並びにする）

- `BasewareRoot` を中身ごと「根の列」に替え、`list_ghosts`・`list_balloons` の引数の型は名前のまま和を返すようにする。`ghost_dir(folder)` は「上から探して最初に在る場所」を返す関数に意味を替える。
- 良い所: 呼び手の 12 ファイルの多くは 1 文字も変わらない。テストの組み立ても `BasewareRoot::new(dir)` のまま（根 1 つの並び）。
- 悪い所: `ghost_store()`・`balloon_store()`（格納フォルダ 1 つ）の意味が並びでは成り立たない（告知・`warn!`・`areka-nar` へ渡す根）。`ghost_dir(folder)` が「無ければ先頭の根の下のパス」を返すのか「無ければ None」なのかで、今の呼び手（主にテスト）の期待が割れる。型の名前と中身が食い違い、後続（`dev-folder-alias`）がエイリアスの段を足す場所も曖昧になる。

### 案 B: 並びの型を新しく足し、呼び手を寄せる

- `catalog.rs`（または兄弟の新しいファイル）に「根の並び」の型（例: 各根の絶対パス・出どころ・見つかったか）と、和の列挙（項目の列＋隠れの列）、引き当ての関数 1 つを足す。`BasewareRoot` は「並びの 1 要素」としてそのまま残す。
- `BootContext.root` を並びの型に替え、呼び手 12 ファイルの `list_ghosts(&ctx.root)` などを和の関数へ置き換える。`boot_resolve` の `at` 閉包は項目の `dir` を使う形へ（ゴーストの `listed` を項目の列へ）。
- 並びの決め方（環境変数・記憶・既定・記録）は `boot_config.rs` の兄弟の新しいファイルへ。メニューの 2 項目は `menu/` の新しい枠のファイルへ。フォルダ選択は `install/pick.rs` に並ぶ新しいファイルへ。
- 良い所: 「格納フォルダ 1 つ」と「並び」の意味が分かれ、後続（エイリアス・`ghost-inner-balloon` の置き場の欄）が乗る境目がはっきりする。`main.rs`・`ghost_switch.rs` には置き換えだけが入る。
- 悪い所: テストの組み立て約 40 ファイルが動く（「根 1 つから並びを作る口」で 1 行の置き換えに抑えられる）。

### 案 C: 段階を分ける（B を 2 つの塊で進める）

- 前の塊: 並びの型・和・隠れ・引き当て・`AREKA_ROOT` の `;` 区切り・呼び手の付け替え・インストール先（`areka-nar` の宛先の変更を含む）・告知。これだけで要件 1（記憶を除く）・2・3・4・5・9・11 が立ち、実機の確認 ①② ができる。
- 後の塊: アプリの記憶への並びの保存・メニューの 2 項目・フォルダ選択・並びを変えたときの効き方（要件 1.4・1.11〜1.13・6・7・8）。
- 良い所: brief の「20 を超えたら後の 1 本に出す」案の切れ目と同じで、超えたときにそのまま切り出せる。リスクの高い `areka-nar` の変更を前の塊で先に確かめられる。
- 悪い所: 1 本の spec の中でタスクの順を工夫するだけなら効果は小さい（切り出すかどうかは要件の討議で決まる）。

## 5. 規模と危険度

- **規模: L（16〜20 タスク）**。呼び手 12 ファイル＋インストールの宛先（`install/worker.rs`・`areka-nar`）＋記憶の入れ物（`areka-sylphya`）＋メニューの枠＋フォルダ選択＋告知＋説明書。どれも今ある型の延長だが、触るファイルが多い。
- **危険度: 中**。
  - `areka-nar` の確定の仕方（作業フォルダ 1 つ＋`rename`）と配置ごとの宛先の根の食い違い。別のドライブの根で失敗する。
  - 記憶の並びの入れ物（今は値 1 つの鍵だけ・消す口が無い・鏡像への投影の檻）と、動いているゴーストの記憶の書き手との書き合い。
  - 大文字小文字の扱いの食い違い（起動・切替は区別・インストールは区別しない）が、根が 2 つになると表に出る。
  - 新しい技術は `IFileOpenDialog` だけ（COM の作法は `wintf` に先例あり）。

## 6. 設計の段へ持ち越す調べもの（Research Needed）

1. windows 0.62 で `IFileOpenDialog`（`FileOpenDialog` の生成・`SetOptions(FOS_PICKFOLDERS | FOS_FORCEFILESYSTEM | FOS_PATHMUSTEXIST)`・`Show(None)`・`GetResult` → `IShellItem::GetDisplayName(SIGDN_FILESYSPATH)` → `CoTaskMemFree`）が、今の機能（`Win32_UI_Shell`・`Win32_System_Com`）だけで組めるか。取り消しは `HRESULT_FROM_WIN32(ERROR_CANCELLED)` で返る。機能の追加（`Win32_UI_Shell_Common` 等）が要るなら `Cargo.toml` を触ることになる（要件 11.5 は「新しい外部依存」を禁じるだけで、機能の追加は別）。
2. 同じフォルダの判定（要件 1.9・6.6・6.8）: `std::path::absolute` の結果（Windows では `GetFullPathNameW`）が末尾の区切り・`.`・`..` をどう畳むか。大小の違い・8.3 の短い名前・`subst`・ジャンクション・シンボリックリンクを同じとみなすか（`canonicalize` は `\\?\` 接頭辞を持ち込むので避けてきた経緯）。
3. `areka-nar` の作業フォルダを宛先の根ごとに掘る形にしたとき、2 つの根にまたがる確定の巻き戻し（1 つ目の根で確定した後に 2 つ目の根で失敗したら 1 つ目を戻す）が今の `roll_back` の形で閉じるか。`.nar-work` の残骸の片付け（今は「入れる根」の棚だけを掃除する）を並びのどの根まで見るか。
4. 記憶の並びの入れ物を TOML の配列にする場合、`format.rs` の寛容読み（型外れは「無し」）と `FormatDoc` の往復の檻への影響。鏡像の dotted 区画へ並びを投影するか（プロパティとして見せるか）、投影しない鍵を許すか。
5. `IFileOpenDialog` を持ち主の窓なしで別スレッドから出したとき、ゴーストの窓より後ろに出ないか（今のファイル選択と同じ扱いでよいか・実機で見る）。

## 7. 設計の判断の項目（要件の討議へ渡す）

1. **並びの型の置き場と `BootContext` の持ち方**: 案 A（`BasewareRoot` の中身を並びに替える）／案 B（並びの型を新しく足し、`BasewareRoot` は 1 要素として残す）。`BootContext.root` の型を替えるか、並びを別の資源に分けるか。メニューから並びを書き換える口（今の `BootContext` は起動後に誰も書き換えない）。
2. **和の項目に「どの根か」の欄を足すか**（要件 3.4）: 今の `dir` の親の親で足りる（足さない）／欄を足す（後続 `dev-folder-alias` のエイリアス・`ghost-inner-balloon` のゴーストの中の置き場では「親の親＝根」が成り立たないので、欄があると後続が素直）。
3. **隠れの単位**: 「格納フォルダに同じ名前のフォルダが在る」で隠すか、「ゴースト（`ghost/master/descript.txt` を持つ）・バルーン（`descript.txt` を持ち `type` が `balloon`）として列挙される」で隠すか。例: 上の根に descript の壊れた `x`、下の根に正常な `x` があるとき、前者なら `x` は使えず、後者なら下の `x` が使われる。インストールの「すでにある」（今は宛先のフォルダの在否）と同じ単位にそろえるかどうかも絡む。
4. **大文字小文字**: 上の根の `Emo2` と下の根の `emo2` を同じ名前として隠すか（Windows のファイルシステムに合わせる・インストールの突き合わせと同じ）、今の起動・切替の突き合わせどおり区別して両方見せるか（2.9 の食い違いが表に出る）。
5. **同じ根の判定**（要件 1.9・6.6・6.8）: 綴りの比較（絶対化の後に大小を区別せず比べる）か、実体の比較（`canonicalize` 等・`\\?\` の扱い）か。
6. **アプリの記憶の並びの入れ物**: (a) 鍵を 1 つ足し、値を 1 本の文字列に詰める（区切りはパスに使えない文字＝`|` か改行）。今の「重ねて書く」で並びごと置き換えられる／(b) TOML の配列を format 層に足す（手で読みやすい・値の型を広げる）／(c) 番号付きの鍵の族（`[window."0"]` の型）。今の保存は消す口が無いので、並びを短くしたときに残りが消えない。後続のエイリアスの表も同じファイルに置く予定。
7. **記憶へ書く経路**: 動いているゴーストの記憶の書き手（`persist_put(App, …)`）を通すか、UI スレッドから直接書くか。今のコードの約束は「実行系が動いている間は書き手を通す」。書き手を通すなら鍵は鏡像への投影も持つ（項目 6 と絡む）。書き手が居ないとき（起動に失敗して既定へ戻る途中など）の扱い。
8. **「実行ファイルの隣」の記憶の綴り**（要件 1.10）: 記号（例 `<exe>`）か、相対パス（`.`＝実行ファイルのフォルダを基準に読む）か。相対パスにすると、`AREKA_ROOT` の相対（カレント基準・要件 1.8）と基準が別になる。
9. **インストールの宛先と `areka-nar` の公開面**（要件 5.1〜5.5）: (a) `InstallRequest` をゴーストの置き場の根・バルーンの置き場の根の 2 つにし、作業フォルダを宛先の根ごとに掘る／(b) 呼び手が配置ごとに宛先の根を渡す口（問い合わせの関数）にする／(c) 同梱のバルーンはゴーストと同じ根へ入れる（要件 5.5 を改める・別ドライブの問題を避ける）。境界の Out「インストールの中身」との境目をどこに引くか。
10. **メニューの 2 項目の置き場と並び**: `Frame` を足す（7 → 8 か 9・`slots`・`ORDER`・`FRAME_CAPTIONS`・テストの 7 の前提が動く）／「インストール…」の枠の中に入れる／「根」のサブメニュー 1 つの下に 2 つ並べる。置く順（「インストール…」の隣など）。SHIORI のリソースで文言を差し替えられるようにするか（SSP に当たる名前は無い・`caption_resource: None` の先例あり）。
11. **フォルダ選択の結果を UI スレッドへ返す受け口**: 今のインストールの窓口（`InstallDesk` の通り道）に相乗りするか、根の並びの小さな受け口を新しく作るか。2 つ目を出さない旗・終了が始まったら結果を捨てる・`AREKA_NO_ALERT` のときは出さずに取り消しと同じ扱い（今のファイル選択に合わせる）。
12. **フォルダ選択の API**: `IFileOpenDialog`＋`FOS_PICKFOLDERS`（今の Windows の標準・brief の案）／`SHBrowseForFolderW`（古い木の画面）。ファイルシステムでない場所（ライブラリ・ネットワークの仮想フォルダ）を選ばせない指定（`FOS_FORCEFILESYSTEM`）。
13. **外した根で動いているゴーストと、外したことで下の根の同じ名前が見えるようになる場合**（要件 8.4）: メニューの印は今のフォルダ名で付くので、見えるようになった下の根の `x` に印が付く。それを選んだときに「今のゴーストと同じ」として何もしないのか、別のフォルダへの切替として扱うのか。
14. **告知の形**（要件 9.4〜9.6）: `alert_text` の本文を 3 行固定から可変にする。`RootError`・`NoGhost`・`NoBalloon`・`AlertScene` の中身を並びに替える。並びが長いとき（10 個など）のメッセージボックスの見え方。
15. **`AREKA_ROOT` の `;` 区切りとフォルダ名の `;`**: `;` を含む名前の根は渡せない（`PATH` と同じ・引用符の扱いは足さない）でよいか。説明書に書くか。
16. **起動のたびの見つからない根**（要件 9.1・9.2）: 起動の後に外付けのドライブをつないだとき、メニューを出したときに見つかれば使うか（一覧はメニューを出すたびに読むので、何もしなければ使われる）。「見つかったかどうか」を起動のときに 1 度だけ決めるか、使うたびに見るか。
17. **1,000 行の目安の守り方**（要件 11.6）: `main.rs`（956）・`ghost_switch.rs`（902）には置き換えだけを入れ、並びの決め方・記録・メニューの動作・フォルダ選択はすべて新しい兄弟のファイルへ出す。`catalog.rs`（414）に足すか兄弟へ出すか。

## 8. 設計の段への勧めの材料（決めない）

- 案 B（並びの型を新しく足し、呼び手を寄せる）は、後続 3 本（`dev-folder-alias`・`user-data-root`・`property-catalog-lists`）が乗る境目がはっきりし、`main.rs`・`ghost_switch.rs` に置き換えだけを入れる形にしやすい。テストの波及は「根 1 つから並びを作る口」で抑えられる。
- 先に確かめたいのは `areka-nar` の宛先（項目 9）と記憶の入れ物（項目 6・7）。どちらも要件の言い回し（5.5・1.4・8.1）に跳ね返りうる。
- 大文字小文字（項目 4）と隠れの単位（項目 3）は、記憶・切替・インストールの 3 経路で「同じフォルダ名から同じ場所を引く」（要件 2.6）を成り立たせる前提なので、要件の段で決めておくと設計が 1 通りに定まる。

## 9. 要件の討議での確定（2026-10-16・全議題を推奨案で承認）

- 項目 3（隠れる単位）→ 要件 2.7: 一覧に出せるものだけが隠す。
- 項目 4（大文字小文字）→ 要件 2.8: 区別しない 1 つの規則を全経路で使う（どこまでの字を同じとみなすかは設計）。
- 項目 9（インストールの宛先）→ 要件 5.5 は (a) のまま・要件 5.9 を追加（根にまたがっても全部か無しか・入れ替えは各根の中）。`areka-nar` の宛先の口と作業フォルダの置き方は In。
- 項目 13（外して見えるようになった同じ名前）→ 要件 8.7: 今のゴーストとして扱い切り替えない。
- 項目 15（`;` を含むフォルダ）→ 要件 1.14・10.2: 渡せない・メニューから足す。
- 項目 16（見つかるかの確かめ）→ 要件 9.3: 使うたびに確かめ、記録は起動と並びの変更のときだけ。
- メニューの文言は「置き場を追加…」「置き場を外す ▸」（要件 6・7・10.1）。
- 残りの項目 1・2・5〜8・10〜12・14・17 と第 6 節の調べものは設計の段で決める。
