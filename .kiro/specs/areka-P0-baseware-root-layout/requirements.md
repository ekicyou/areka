# Requirements Document

> 本文の実測は **2026-09-23・本ブランチ**（main `92f5f448`＝`areka-P0-app-lifetime-separation` 着地後）のもの。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。
> 要件 9 の裁定 1〜5 は **2026-09-23 の要件ディスカッションで確定済み**（1・2 は brief の推奨どおり、3〜5 は開発者裁定）。

## Introduction

### 誰が困っているか

areka を初めて手にする第三者（ゴーストの利用者）と、その人に配る開発者。

### いま何が起きているか（2026-09-23 実測）

- **`areka.exe` は「起動するゴーストを argv で 1 体指定する開発用の実行体」であって、アプリではない。** `crates/areka/src/boot_config.rs` の `fn resolve_config_inputs` は `args[1]`＝ゴーストのフォルダ・`args[2]`＝バルーンのフォルダを受け取り、無ければ `fn default_ghost_root`／`fn default_balloon_root`（`CARGO_MANIFEST_DIR` 相対の `ghost/master`・`balloon/master`＝プレースホルダ）へ落ちる。この既定は**配布した exe では実在しない**。
- **実在しなくても起動を続ける。** `crates/areka/src/main.rs` の `fn main` は根が無いと `warn!`（「{label} が存在しません（決定のみでマウントはしない・継続します）」）を出して進み、`fn open_startup_window` が配置準備に失敗すると `fn spawn_dummy_window` で**検証用ダミー窓**を開いて完走する。第三者から見ると「exe を起動しても小さな四角が出るだけで、何も始まらない」。ロードマップはこの経路を「ログ無し失敗経路の禁止」に反するものとして本仕様が置き換えると定めている（先に直さない＝二度触らない）。
- **ベースウェア直下の `ghost/`・`balloon/` を知っているコードは 1 行も無い。** `ghostpathlist`／`installedghostname`／`balloonpathlist`・`AREKA_ROOT`・`BasewareRoot`・`menu,hidden` はいずれも `crates/**/*.rs` で 0 件。`crates/areka-parsers/src/package/resolve.rs` の `fn resolve` が知るのは **1 体のゴーストの内側**（`ghost/master/descript.txt` と `shell/<名>/`）だけで、読む鍵は `name`・`sakura.name`・`sakura.name2`・`kero.name`・`shiori`・`shiori.encoding`・`shiori.forceencoding`・`seriko.defaultsurfacedirectoryname`・`readme` の 9 つ。`craftman`・`craftmanw`・`id`・`type`・`menu` は読まない。`fn resolve` は `shell/<名>/` の実在を要求して失敗する（`MountError::ShellDirMissing`）ので走査には流用できず、本番の呼び手は 4 か所（`emo2_boot/assets.rs`・`placement/persist.rs`・`placement/source.rs`・`areka-ghost/src/runtime.rs`）が独立に呼ぶ。流用できる部品は `charset::decode`（バイト列→文字列）と `kv::parse_kv`（文字列→鍵と値の表）の 2 つ。
- **バルーンの `descript.txt` の読み手（`crates/areka-parsers/src/balloon/parse.rs` の `fn parse`）は幾何とフォントの鍵だけを読む。** `type`・`id`・`name`・`craftman` は読まない。
- **「前回どのゴーストとバルーンを使ったか」を覚える場所が無い。** `crates/areka-sylphya/src/persist/mod.rs` の永続化は `App | Ghost | Shell | Balloon` の 4 スコープ（`PersistScope`）を持ち、`App` スコープの保存先は `boot_config.rs` の `fn default_app_profile_dir`（環境変数 `AREKA_PROFILE_DIR`、無ければ `<exe のフォルダ>/profile/areka/`）から `GhostBootOptions.app_profile_dir` を経て配線済み。鍵の族（`PersistKey`）は 4 つ（窓位置 `areka.window.scope(ID).x|y`・バルーン offset `areka.balloon.offset.scope(ID).x|y`・起動回数 `areka.boot.count`・消滅回数 `areka.vanish.count`）で**全て `Ghost` スコープに書かれ、`App` スコープに載る鍵は今日 0 個**。ファイルはどのスコープも `<スコープの根>/sylphya.toml`（TOML の表 `[window."ID"]`・`[balloon-offset."ID"]`・`[boot]`・`[vanish]`・全値は文字列）。書き込みは一時ファイルへ全書き→`rename` の原子的確定（`persist/io.rs` の `FsPersistIo::commit`）。
- **順序の罠**: sylphya のアクターはゴーストごとに `areka-ghost` の `fn boot_with_kanade_stop` の中で起きる。ゴーストを選ぶ**前**に `App` スコープの記憶を読む口、およびバルーンを選ぶ**前**（＝起動前）にそのゴーストの `Ghost` スコープの記憶（`<ゴースト>/ghost/master/profile/areka/sylphya.toml`）を読む口は、公開関数 `areka_sylphya::persist::load_scope` を直接呼ぶ形しか無い。書く口はゴーストの起動後に `runtime.sylphya_publisher()`（`persist_put(scope, entries)`）で取れる。
- **検体の根はすでに「根の形」をしている。** `crates/sample-ghost-kit` の `SampleRoot::acquire(名)` は `vendors/sample_ghost/<名>.nar` を展開した使い捨ての複製を配り、`root()` が**ベースウェアの根**（直下に `ghost/`・`balloon/`）、`folder()` が `<根>/ghost/<名>/`、`balloon(名)` が `<根>/balloon/<名>/` を返す。登記済みの検体は `emo2`（ゴースト・同梱バルーン `emo2-kakukaku`・`install.txt` に `balloon.directory,emo2-kakukaku`）・`R_POST_and_KOMAINU`（ゴースト・バルーン同梱なし）・`konnoyayame`（ゴースト・バルーン同梱なし）・`emo2-kakukaku-offsetdpi`／`emo2-kakukaku-wplimit`（バルーン）の 5 体。展開先の形（`<根>/ghost/<directory>/`・`<根>/balloon/<balloon.directory>/`・最上位の `install.txt` は `supplement` 以外で残る）は `.nar` インストールのエンジンが決めたもので、**これがそのまま本仕様の根の形になる**。
- **既定バルーン `StayseeBalloon`（CC0）は `vendors/sample_ghost/StayseeBalloon/` に展開フォルダのまま在り、検体の登記表 `SAMPLES` には載っていない。** `descript.txt` は `type,balloon`・`id,StayseeBalloon`・`name,Balloon for Staysee Syncfield`・`craftman,SSP BUGTRAQ`・`craftmanw,…`・`charset,Shift_JIS`、サムネイルは `thumbnail.pnr`（`.png` は無い）。`.nar` へ畳んで登記するのは並走 spec `areka-P0-default-balloon-nar-fold` の仕事。**本番コードに既定バルーン id の定数を置き、解決順の最後へ渡すのは本仕様の仕事**（完了 spec `areka-P0-default-balloon-bundle` 要件 7.2 の申し送り）。
- **常設の smoke テスト `crates/areka/tests/smoke_boot_loop_exit.rs`** は環境変数 `AREKA_APP_SMOKE_EXIT_MS=500` で areka を起こし、60 秒の見張りの中で終了コードと起動ログの目印を確かめる 2 本＝**フォールバック方向**（引数なし・目印「窓配置の準備起点が見つかりません」「検証用ダミー窓を開きました（placement フォールバック）」・終了コード 0）と**本物方向**（`emo2` の `folder()` と `balloon("emo2-kakukaku")` を argv で渡す・目印「emo2-boot: 実 sink 結線が成立しました（wire 成立）」「本物のゴースト窓を開きました」・終了コード 0。モニタ 0 台の環境ではダミー窓へのフォールバックを受理する）。完了 spec `areka-P0-app-lifetime-separation` からの申し送り: **このテストの 60 秒の見張りと終了コード 0 の判定は残す**（「窓が無いのにプロセスが残る」壊れ方を赤にする唯一の常設検査）。目印を変えるのは構わない。
- **ダミー窓に依存している部品**: `main.rs` の `fn spawn_dummy_window`・`fn on_dummy_pressed`・`DummyWindowMarker`、`crates/areka/src/app_exit.rs` の `ExitOrigin::DummyWindow`・`fn on_dummy_os_close`、`fn quit_app` の私有 `despawn_app_windows`（ダミー窓＋ゴースト窓を閉じる）、テスト `crates/areka/src/main_startup_window_tests.rs`（5 本）・`app_exit_tests.rs` の `despawn_app_windows_hits_dummy_and_ghost_only`。
- **行数**（1 ファイル 1,000 行の目安）: `main.rs` 909・`boot_config.rs` 159・`app_exit.rs` 167・`persist/mod.rs` 934・`persist/format.rs` 464・`areka-ghost/src/lib.rs` 49。`persist/mod.rs` はテストを同居させたまま鍵の族を足すと目安を超えるので、テストの別ファイル化が要る。
- **`windows` crate の機能 `Win32_UI_WindowsAndMessaging` はワークスペース依存に含まれ、`main.rs` が既に読み込んでいる**＝利用者向けのメッセージボックス（`MessageBoxW`）に新規依存は要らない。`MessageBoxW` の呼び出しは今日 0 件。
- **環境変数の命名**: 本番コードが読む env は `AREKA_` 名前空間（記憶 areka-runtime-env-naming）。今日の本番 knob は `AREKA_PROFILE_DIR`・`AREKA_APP_SMOKE_EXIT_MS`・`AREKA_TICK_GATE`・`AREKA_SHIORI_REQUEST_TIMEOUT_MS` 等。`AREKA_ROOT` は 0 件で衝突しない。

### 正典（ukadoc）の位置づけ

| 正典 | 逐語引用 | 本仕様への含意 |
|---|---|---|
| [全体の構成](https://ssp.shillest.net/ukadoc/manual/manual_directory.html) | 「ゴーストの構成ファイルは、ベースウェア(SSP,materiaなど)のゴースト格納フォルダに配置される。」「全てのゴーストは必ずひとつのmasterゴーストとmasterシェルを持つ必要がある。」 | 根の下の `ghost/` が「ゴースト格納フォルダ」。ゴーストである条件は `ghost/master/descript.txt` を持つこと。 |
| 同上 | 「バルーンはゴーストごとでなく、一つのベースウェアの元に統括的に管理され、全ゴーストで共有される。各バルーン構成ファイルはベースウェアのバルーン格納フォルダ以下に…配置される。」 | 根の下の `balloon/` が「バルーン格納フォルダ」。バルーンは全ゴースト共有＝根に 1 か所。 |
| 同上 | 「(myghost)\shell\(additional_shell)\ 追加シェルのフォルダ。任意名（英数字推奨）。…いくつでも存在できる。存在するとオーナードローメニューなどから切替選択できる。」 | シェルの列挙は `<ゴースト>/shell/` の直下の各フォルダ。 |
| 同上 | 「(myghost)\thumbnail.png メニューツリーでゴーストの選択時に表示されるサムネイル。…省略可能。」「(myghost)\readme.txt または readme.md ゴースト全体のreadme。」 | 素性の `thumbnail.png` の有無と `readme` の所在はフォルダの最上位で見る。 |
| [`menu,hidden`（シェル）](https://ssp.shillest.net/ukadoc/manual/descript_shell.html#menu_2chidden:1) | 「シェルをゴーストのシェル切り替えメニューに表示しなくなる。」 | `menu,hidden` のシェルは列挙に含めない。 |
| [`type,種別`（バルーン）](https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#type_2c_7a2e_5225:1) | 「ファイルセットの種別。バルーンの場合はballoon。」既定「balloon( on SSP)」 | `balloon/` 直下の `descript.txt` は `type` が無ければバルーンとみなし、`balloon` 以外なら除く。 |
| [`type,種別`（ゴースト）](https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#type_2c_7a2e_5225:1) | 「SSPではghost/masterにあるdescript.txtならghostと識別される」 | ゴーストの判定に `type` は使わない（置き場で決まる）。 |
| [`id,ID名`（バルーン）](https://ssp.shillest.net/ukadoc/manual/descript_balloon.html#id_2cID_540d:1) | 「そのバルーンのID名。半角英数のみ。」 | 既定バルーンは id `StayseeBalloon` で指す（フォルダ名と id がバイト一致することは完了 spec で実測済み）。 |
| [`directory`／`*.directory`（install.txt）](https://ssp.shillest.net/ukadoc/manual/descript_install.html#_2a.directory_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1) | 「同時インストールする内容のインストール後のディレクトリ名。…複数の同じ種類のものをインストールしたい時は、*部分を balloon0,balloon1,...のように…複数書けば良い。」 | ゴースト同梱のバルーンは `install.txt` の `balloon.directory` で引く（番号付きの複数指定は α では読まない）。 |
| [`readme,ファイル名`（ゴースト）](https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#readme_2c_30d5_30a1_30a4_30eb_540d:1) | 「インストール時やオーナードローメニューから開かれるゴーストの説明テキストファイル名。」既定 `readme.txt` | 素性の `readme` は descript の `readme` 鍵（無ければ `readme.txt`）が指すファイルの所在。 |
| [`craftmanw,作者名`（ゴースト）](https://ssp.shillest.net/ukadoc/manual/descript_ghost.html#craftmanw_2c_4f5c_8005_540d:1) | 「そのゴーストの作者名。」 | 素性の作者名は `craftman`・`craftmanw` の 2 つ（無ければ「無し」）。 |

### 何を変えるか

**areka.exe は「根」を 1 つ持つアプリになる。** 既定は exe の隣、開発時は環境変数 `AREKA_ROOT` で差し替える。根の直下の `ghost/`・`balloon/` を走査してゴースト・シェル・バルーンを素性付きで列挙し、最後に使ったゴースト（アプリの記憶）とゴーストごとの最後のバルーン・シェル（ゴーストの記憶）を覚え、起動時は **argv の上書き → 記憶 → 根に 1 体だけ → 既定ゴースト `emo2`（配布物に必ず同梱）→ 無作為に 1 体 → 0 体なら「どこに何を置けば良いか」を告げて終了**の順で決める。バルーンは **argv → そのゴーストの記憶 → ゴーストの同梱 → 1 つ → 既定バルーン `StayseeBalloon` → 無作為 → 0 なら告知**。無いときに `warn!` で起動を続ける経路と検証用ダミー窓は退役し、失敗は `error!`＋利用者向けの告知＋0 以外の終了コードで終わる。

## Boundary Context

- **In scope**:
  - 根の解決（既定＝exe の隣・`AREKA_ROOT` で差し替え）と `ghost/`・`balloon/` の走査。
  - ゴースト・シェル・バルーンの列挙と素性（フォルダ名・`name`・`craftman`・`craftmanw`・`id`・`readme` の所在・`thumbnail.png` の有無・`menu,hidden`・`type`）。
  - 「最後に使った」鍵 3 つ（アプリの記憶に `areka.last.ghost`、ゴーストの記憶に `areka.last.balloon`・`areka.last.shell`）の定義と、起動成功時の書き込み。**シェルの記憶を起動時に効かせる口は作らない**（`areka-P0-shell-balloon-switch` の仕事）。
  - 起動解決の置き換え（ゴースト・バルーンそれぞれの解決順）と、argv の上書きの存続。
  - 既定バルーン id `StayseeBalloon` と既定ゴースト `emo2` の定数と、解決順への配線。
  - 「根が無い／ゴーストが無い／バルーンが無い／起動窓を開けない」の利用者向け告知（メッセージボックス）と、告知を抑える口。
  - `warn!` で起動を続ける経路と検証用ダミー窓の退役・常設の smoke テストの追随。
  - 検体の根（`SampleRoot::root()`）をそのまま根として受ける（テストと実機の入口を 1 つにする）。
- **Out of scope**:
  - `.nar` の展開（完了 spec `areka-P0-nar-install`）・投げ込みや対話でのインストール（`areka-P0-ghost-install`）。
  - 切り替えの実行（`areka-P0-ghost-shell-balloon-switch`・`areka-P0-shell-balloon-switch`）とメニューへの登記（本仕様は `crates/areka/src/menu/` に触らない）。
  - `recommended.balloon`／`recommended.ghost` による並び替えや案内（α 後）。
  - プロパティシステムの `ghostlist`／`balloonlist`／`currentghost.shelllist.*`（`areka-P0-property-catalog-lists`・`areka-P0-currentghost-property-tree`・α 後）。本仕様の列挙はその日の供給源になる形で置くだけ。
  - 複数の根（ポータブル運用と `%APPDATA%` の併用）・多重ゴースト（根は 1 つ・同時起動は 1 体）。
  - 設定画面・設定ファイル（`areka.toml` 等）の新設。記憶はプロパティの永続化で足りる。
  - 既定バルーンを `.nar` へ畳んで検体に登記すること（`areka-P0-default-balloon-nar-fold`）・配布 zip の形（`areka-P0-alpha-release-signoff`）。
  - `install.txt` の番号付きバルーン（`balloon0.directory`…）・`thumbnail.pnr`・`readme.md` の解釈（α では読まない＝0）。
  - アプリの記憶の保存先（`AREKA_PROFILE_DIR`・既定 `<exe のフォルダ>/profile/areka/`）の変更。
  - `wire_emo2_boot` が `wired=false` を返したときの `LogSink` フォールバック boot（`fn main` の else アーム）の存廃。本仕様は触らない（`is_benign_boot_error` の doc が既定パスの不在を根拠にしている箇所の書き換えだけ設計で扱う）。
- **Adjacent expectations**:
  - `areka-P0-app-lifetime-separation`（着地済み）: 終了の指示 `quit_app` と終了操作 7 種の形は変えない。ダミー窓に紐づく終了操作（ダブルクリック・ダミー窓への OS の閉鎖要求）は窓ごと退役する＝同 spec 要件 3.4・3.11 は退役、その他は不変。
  - `areka-P0-nar-install`（着地済み）: 展開先の形＝根の形。`SampleRoot` の 3 つの読み口（`root()`・`folder()`・`balloon()`）は変えない。登記表 `SAMPLES` には触らない。
  - `areka-P0-default-balloon-nar-fold`（並走）: 既定バルーンを検体に登記するのは相手。本仕様の決定論テストは相手の着地に依存せず、フォルダ名 `StayseeBalloon` の偽のバルーンを置いた根で既定の採用を確かめる。
  - `areka-P0-shell-balloon-switch`／`areka-P0-ghost-shell-balloon-switch`（後続）: 切替後の選択は本仕様の鍵へ書き、列挙は本仕様の関数から取る。起動時にシェルの記憶を効かせる口は後続が作る。**バルーンの鍵 `areka.last.balloon` はゴーストの記憶（`Ghost` スコープ）にある**（2026-09-23 裁定 4）ので、同 spec の brief が鍵をアプリの記憶に置いて書いていれば改める。
  - `areka-P0-alpha-release-signoff`（並走）: 2026-09-23 裁定「areka の配布物には必ず `emo2` を同梱する」により、同 spec の brief の「配布 zip の `ghost/` は空・emo2 は 2 体目として `.nar` で入れる」前提は改まる（配布物の中身と第三者の手順の項目 1・4 を同 spec の要件段階で整合させる）。本仕様が置くのは定数 `emo2` と解決順だけで、zip に何を入れるかは相手の仕事。
  - `areka-P0-nar-install-hardening`（並走）: 相手が根の下に置く作業フォルダ（`<根>/.nar-work/`）は `ghost/`・`balloon/` の外なので列挙に影響しない。
  - `areka-P0-status-execution-states`（α 後）: `emo2_boot/mod.rs` を共有しうるが α 後なので順序で解決。

## Requirements

### Requirement 1: ベースウェアの根

**Objective:** As a areka を配る開発者, I want areka.exe が「ゴーストとバルーンを置く場所」を 1 つ持つこと, so that zip を展開して起動するだけで、利用者が置いたゴーストが見つかる

#### Acceptance Criteria

1. The areka shall ベースウェアの根を 1 つ持ち、その直下の `ghost/` をゴースト格納フォルダ、`balloon/` をバルーン格納フォルダとする（`<根>/ghost/<フォルダ名>/`・`<根>/balloon/<フォルダ名>/`）。
2. When 環境変数 `AREKA_ROOT` が設定されていない, the areka shall 実行ファイルのあるフォルダを根とする。
3. When 環境変数 `AREKA_ROOT` が設定されている, the areka shall その値のフォルダを根とし、実行ファイルのあるフォルダは見ない。
4. If 根として決まったフォルダが実在しない, or `AREKA_ROOT` が無く実行ファイルの場所（`current_exe()`）が取れない, then the areka shall `error!` と利用者向けの告知（要件 6）を出し、0 以外の終了コードで終える（カレントディレクトリ `"."` へ黙って倒さない）。
5. The areka shall `CARGO_MANIFEST_DIR` 相対の既定パス（`fn default_ghost_root`・`fn default_balloon_root`）を持たない（配布した exe で実在しない既定を残さない）。
6. The areka shall 検体の根（`SampleRoot::root()`）を `AREKA_ROOT` に渡すだけで、テストと実機が同じ根の形で起動できる。
7. The areka shall アプリの記憶の保存先（`AREKA_PROFILE_DIR`・既定 `<exe のフォルダ>/profile/areka/`）を変えない。

### Requirement 2: インストール済みの列挙と素性

**Objective:** As a 切替やメニューを実装する開発者, I want 根の下のゴースト・シェル・バルーンを素性付きで数えられること, so that 後続の切替・メニュー・インストールが同じ 1 つの目録に乗る

#### Acceptance Criteria

1. When ゴーストの列挙を求められる, the areka shall `<根>/ghost/` の直下の各フォルダのうち `ghost/master/descript.txt` を持つものをゴーストとして返し、持たないフォルダとフォルダ以外の項目は返さない。
2. When あるゴーストのシェルの列挙を求められる, the areka shall `<ゴースト>/shell/` の直下の各フォルダのうち `descript.txt` を持つものをシェルとして返し、`descript.txt` に `menu,hidden` があるシェルは返さない。
3. When バルーンの列挙を求められる, the areka shall `<根>/balloon/` の直下の各フォルダのうち `descript.txt` を持ち、`type` が無いか `balloon` であるものをバルーンとして返し、`type` が `balloon` 以外のフォルダは `warn!` を残して返さない。
4. The areka shall 列挙の各項目に次の素性を付ける: フォルダ名・`name`・`craftman`・`craftmanw`・`id`（それぞれ無ければ「無し」）・説明書の所在（descript の `readme` 鍵、無ければ `readme.txt`、が指すファイルがそのフォルダの最上位に実在すればそのパス、無ければ「無し」）・`thumbnail.png` の有無（フォルダの最上位）。
5. The areka shall 列挙の並びをフォルダ名の昇順（バイト順）とし、`recommended.*` は並びに使わない。
6. If `<根>/ghost/` または `<根>/balloon/` が実在しない, then the areka shall その列挙を 0 件として返す（列挙は失敗にしない。無いことの告知は起動解決＝要件 4・5 が行う）。
7. If ある項目の `descript.txt` が読めない（I/O 失敗）, then the areka shall `warn!` を残してその項目を除き、残りの列挙を続ける。
8. The areka shall descript の文字符号化を既存の charset 復号（完了 spec `areka-P0-charset-canon`）と同じ規則で扱い、`Shift_JIS` 宣言の既定バルーンの `name`・`craftmanw` を文字化けなく返す。
9. The areka shall 列挙に `sakura.name`・`kero.name`・`craftmanurl`・`homeurl`・`recommended.*`・`thumbnail.pnr`・`readme.md` を含めない（α に要る素性は 4 の 7 項目だけ。残りは束「配布物の素性」の未実装として残る）。
10. The areka shall 列挙を「1 体のゴーストの解決」（`fn resolve`）の置き換えではなく別の読み手として置き、`fn resolve` の 4 つの本番の呼び手とその決定論テストを変えない。

### Requirement 3: 最後に使ったものの記憶

**Objective:** As a ゴーストの利用者, I want 前回使ったゴースト・バルーン・シェルを areka が覚えていること, so that 再起動しても選び直さずに済む

#### Acceptance Criteria

1. The areka shall アプリの記憶（`App` スコープ）に鍵 `areka.last.ghost`（ゴーストのフォルダ名）を、ゴーストの記憶（`Ghost` スコープ）に鍵 `areka.last.balloon`（バルーンのフォルダ名）と `areka.last.shell`（シェルのフォルダ名）を持つ（既存の 4 族に 1 族を足す・記憶のファイルの表は `[last]`。バルーンの記憶がゴーストごとなのは 2026-09-23 裁定 4）。本要件で「ゴーストの起動が成功する」とは、起動窓の準備（`fn open_startup_window`）が通った後に `areka_ghost::boot` が `Ok` を返した時点を指す（wired／fallback の両経路で 1 か所）。
2. When ゴーストの起動が成功する and ゴーストが argv 以外の経路（要件 4.2〜4.5）で決まった, the areka shall `areka.last.ghost` にそのフォルダ名を書く。
3. When ゴーストの起動が成功する and バルーンが argv 以外の経路（要件 5.2〜5.6）で決まった, the areka shall そのゴーストの記憶の `areka.last.balloon` にそのフォルダ名を書く。
4. When ゴーストの起動が成功する, the areka shall 起動に使ったシェルのフォルダ名（`seriko.defaultsurfacedirectoryname`、無ければ `master`＝`fn resolve` が決めたもの）を、そのゴーストの記憶の `areka.last.shell` に書く。
5. When ゴーストまたはバルーンが argv で決まった（根の内外を問わない）, the areka shall 対応する記憶を書き換えず、その旨を記録に残す（info。パスが根の直下かの比較は行わない＝2026-09-23 裁定 5。シェルの記憶（4）は argv の有無によらず書く）。
6. The areka shall 記憶の書き込みを既存の永続化と同じ経路（原子的確定・既存の鍵と同じファイル）で行い、既存の 4 族の読み書きと決定論テストを 1 本も変えない。
7. The areka shall 鍵を書いて読み戻す往復と、「無い鍵は無し」を決定論テストで確かめる。
8. The areka shall `areka.last.shell` を起動時の解決に使わない（起動時にシェルの記憶を効かせる口は `areka-P0-shell-balloon-switch` が作る）。

### Requirement 4: 起動時のゴーストの解決

**Objective:** As a ゴーストの利用者, I want exe を起動するだけで前回のゴースト（初回なら置いたゴースト）が立ち上がること, so that argv を知らなくても areka を使える

#### Acceptance Criteria

1. When 起動時に argv の第 1 引数（ゴーストのフォルダのパス）が与えられている, the areka shall そのフォルダを起動するゴーストとし、記憶と列挙を見ない（開発者の上書き。実機サインオフの絶対パス起動は今までどおり動く）。
2. When argv の第 1 引数が無い and 記憶 `areka.last.ghost` が根の直下に実在するゴーストを指す, the areka shall そのゴーストを起動する。
3. When argv の第 1 引数が無い and 記憶が無いか、指すゴーストが根に無い, and 根に列挙されるゴーストが 1 体, the areka shall その 1 体を起動する。
4. When argv の第 1 引数が無い and 記憶が無いか、指すゴーストが根に無い, and 根に列挙されるゴーストが 2 体以上 and 既定ゴースト（フォルダ名 `emo2`）が列挙される, the areka shall 既定ゴーストを起動する（areka の配布物には必ず `emo2` を同梱する＝2026-09-23 裁定）。
5. When argv の第 1 引数が無い and 記憶が無いか、指すゴーストが根に無い, and 根に列挙されるゴーストが 2 体以上 and 既定ゴーストが列挙されない, the areka shall 列挙の中から 1 体を無作為に選んで起動し、無作為に選んだことを記録に残す（info。起動が成功すれば要件 3.2 で記憶に書かれ、次回からは記憶で同じゴーストが立つ）。
6. If 記憶 `areka.last.ghost` の指すゴーストが根に無い, then the areka shall その旨を `warn!` に残して 3〜5 へ進む（黙って読み替えない）。
7. If argv の第 1 引数が無い and 根に列挙されるゴーストが 0 体, then the areka shall `error!` と利用者向けの告知（要件 6）を出し、0 以外の終了コードで終える。
8. If argv の第 1 引数の指すフォルダにゴーストが無い（`ghost/master/descript.txt` が無い）, then the areka shall `error!` と利用者向けの告知を出し、0 以外の終了コードで終える（`warn!` で起動を続けない）。
9. The areka shall 上の解決を「argv・記憶の値・列挙の結果」だけから決まる純粋な判断として持ち、argv／記憶／唯一／既定／無作為／0 体の 6 分岐と記憶の指す先が無い場合を決定論テストで踏む（既定の分岐はフォルダ名 `emo2` の偽のゴーストを置いた根で確かめる＝検体の登記に依存しない。無作為の分岐は「選ばれたものが列挙に含まれる」ことを確かめる）。
10. When ゴーストが決まる, the areka shall どの経路で決まったか（argv／記憶／唯一／既定／無作為）と決まったフォルダを記録に残す（info）。
11. The areka shall 既定ゴーストのフォルダ名 `emo2` を本番コードの定数 1 つで持ち、解決順の段 4 だけがそれを使う（既定バルーンの定数＝要件 5.9 と同じ置き方）。

### Requirement 5: 起動時のバルーンの解決

**Objective:** As a ゴーストの利用者, I want バルーンを同梱しないゴーストでも既定バルーンで会話が出ること, so that 第三者のゴーストの多くが「バルーンが無い」で止まらない

#### Acceptance Criteria

1. When 起動時に argv の第 2 引数（バルーンのフォルダのパス）が与えられている, the areka shall そのフォルダをバルーンとし、以下の解決を見ない（開発者の上書き）。
2. When argv の第 2 引数が無い and 起動するゴーストの記憶 `areka.last.balloon` が根に実在するバルーンを指す, the areka shall そのバルーンを使う（利用者の選択はゴーストごと・同梱より優先＝2026-09-23 裁定 4。起動するゴーストの記憶は起動前に `load_scope` で読む）。
3. When 2 が当たらない and 起動するゴーストの最上位の `install.txt` に `balloon.directory` があり、その名のバルーンが根に列挙される, the areka shall そのバルーンを使う（ゴーストの同梱）。
4. When 2・3 が当たらない and 根に列挙されるバルーンが 1 つ, the areka shall その 1 つを使う。
5. When 2〜4 が当たらない and 根に既定バルーン（フォルダ名 `StayseeBalloon`）が列挙される, the areka shall 既定バルーンを使う。
6. When 2〜5 が当たらない and 根に列挙されるバルーンが 2 つ以上, the areka shall 列挙の中から 1 つを無作為に選んで使い、無作為に選んだことを記録に残す（info。ゴースト＝要件 4.5 と同じ規則）。
7. If 記憶 `areka.last.balloon` の指すバルーンが根に無い, or `install.txt` の `balloon.directory` の指すバルーンが根に無い, then the areka shall その旨を `warn!` に残して次の段へ進む。
8. If argv の第 2 引数が無い and 根に列挙されるバルーンが 0, then the areka shall `error!` と利用者向けの告知（要件 6）を出し、0 以外の終了コードで終える。
9. The areka shall 既定バルーンの id `StayseeBalloon`（フォルダ名と id はバイト一致＝完了 spec `areka-P0-default-balloon-bundle` で実測済み・列挙はフォルダ名で照合する）を本番コードの定数 1 つで持ち、解決順の段 5 だけがそれを使う。
10. The areka shall 上の解決を純粋な判断として持ち、argv／記憶／同梱／1 つ／既定／無作為／0 の 7 分岐と、記憶・同梱の指す先が無い場合を決定論テストで踏む（既定の分岐は、フォルダ名 `StayseeBalloon` の偽のバルーンを置いた根で確かめる＝検体の登記に依存しない。無作為の分岐は「選ばれたものが列挙に含まれる」ことを確かめる）。
11. When バルーンが決まる, the areka shall どの経路で決まったか（argv／記憶／同梱／唯一／既定／無作為）と決まったフォルダを記録に残す（info）。

### Requirement 6: 無いときに黙らない

**Objective:** As a ゴーストの利用者, I want 何も出ないときに「どこに何を置けば良いか」を areka が教えてくれること, so that 初回に exe を起動して途方に暮れない

#### Acceptance Criteria

1. When 根・ゴースト・バルーンのいずれかが解決できない（要件 1.4・4.7・4.8・5.8）, the areka shall 利用者向けのメッセージボックス（OS 標準の Win32 `MessageBoxW`・新規依存 0）に、何が無いかと、置くべき場所の絶対パス（`<根>/ghost/` または `<根>/balloon/`）と、そこに置くものの形（ゴーストなら `ghost/master/descript.txt` を持つフォルダ・バルーンなら `descript.txt` を持つフォルダ）を日本語で示す。
2. When 告知を出す, the areka shall 同じ内容を `error!` にも残す（記録だけで告知しない、告知だけで記録しない、のどちらも作らない）。
3. When 告知を閉じる, the areka shall 0 以外の終了コードでプロセスを終える（起動を続けない・窓 0 で居座らない）。
4. If ゴーストとバルーンが決まった後に起動窓を開けない（配置準備の失敗＝モニタ 0 台・読取不能等）, then the areka shall `error!` と失敗の内容を示す告知を出し、0 以外の終了コードで終える（検証用ダミー窓へのフォールバックは退役）。
5. While 告知を抑える環境変数（`AREKA_` 名前空間・名は設計で 1 つ定める）が設定されている, when 告知を出す場面になる, the areka shall メッセージボックスを出さず、`error!` と終了コードだけを同じにする（自動テストでモーダルが止まらない）。
6. The areka shall 「根が無いのに `warn!` で起動を続ける」経路（`fn main` の存在確認ループ）と検証用ダミー窓（`fn spawn_dummy_window`・`fn on_dummy_pressed`・`DummyWindowMarker`・`ExitOrigin::DummyWindow`・`fn on_dummy_os_close`）を退役させ、コードと doc に残さない。
7. The areka shall 告知の 4 場面（根なし／ゴーストなし／バルーンなし／起動窓を開けない）のそれぞれが `error!` を 1 件残すことを決定論テストで確かめる（メッセージボックスは 5 の抑止で出さない）。

### Requirement 7: 既存の振る舞いと検査を守る

**Objective:** As a 開発者, I want 本仕様が触らない範囲の振る舞いと検査がそのまま残ること, so that 根の導入が他の経路を壊していないと言える

#### Acceptance Criteria

1. The 本仕様 shall argv で絶対パスを渡す起動（実機サインオフ・`tests/emo2_real_run.rs`・`examples/`）の見え方を変えない。
2. The 本仕様 shall `crates/areka/src/menu/`・`crates/areka-ghost/src/runtime.rs`・kanade の終了の握手・`fn quit_app` の終了の指示の形を変えない（ダミー窓の消去に伴う私有関数の対象縮小＝ゴースト窓だけ、を除く）。
3. The 本仕様 shall `fn resolve` の 4 つの本番の呼び手と、既存の永続化の 4 族の決定論テストを 1 本も変えない。
4. The 本仕様 shall ダミー窓に依存する既存テスト（`main_startup_window_tests.rs` の 5 本・`app_exit_tests.rs` のダミー窓の検査）と、撤去する既定パス（要件 1.5）を直接参照する `main_config_input_tests.rs` の 4 本を「退役した検証対象」として除き、意味の残る検査（`fn quit_app` がゴースト窓を全て閉じて終了を指示する）は更新して残す（記憶 obsolete-vs-broken-test-policy）。
5. The 本仕様 shall 常設の smoke テスト `crates/areka/tests/smoke_boot_loop_exit.rs` の 60 秒の見張りと終了コードの判定を残し、3 方向で張る: ① **本物方向**（argv・終了コード 0・目印は今までどおり）／② **根方向**（argv なし・`AREKA_ROOT` に検体 `emo2` の根・`AREKA_PROFILE_DIR` を一時フォルダ（`temp-path-kit`）へ向けて記憶なしを保証・唯一のゴーストと同梱バルーンで起動し終了コード 0。同梱バルーンで起動したことはパスを綴らず要件 5.11 の記録の目印で見る）／③ **0 体方向**（argv なし・空の根・告知を抑止・`error!` の目印と 0 以外の終了コード・見張りの内側で終わる）。旧「フォールバック方向」（ダミー窓）は ③ に置き換える。3 方向とも告知を抑止して起動し、モニタ 0 台の環境では ①② について要件 6.4 の `error!` と 0 以外の終了コードを受理する（今日の「ダミー窓へのフォールバックを受理」の置き換え）。
6. The 本仕様 shall 変更後も `crates/areka/src/main.rs`・`boot_config.rs`・`crates/areka-sylphya/src/persist/mod.rs` を 1 ファイル 1,000 行の目安の内側に収める（`persist/mod.rs` はテストを別ファイルへ移す）。
7. The 本仕様 shall 新規の外部依存を足さない。

### Requirement 8: 決定論テストと実機確認

**Objective:** As a 開発者, I want 根・列挙・記憶・起動解決の判断分岐がそれぞれ赤にできること, so that 後ろの α の spec 全部が乗る契約が固定される

#### Acceptance Criteria

1. The 本仕様 shall 列挙を、検体の根（`emo2`＝ゴースト 1・バルーン 1・同梱の `install.txt`）と、一時フォルダに組んだ根（ゴースト 2 体・`menu,hidden` のシェル 1 つを含むシェル 2 つ・`type,balloon` のバルーン・`type` が `balloon` 以外のフォルダ・`descript.txt` の無いフォルダ・フォルダ名 `StayseeBalloon` の偽のバルーン）で、件数と素性の 7 項目を突き合わせる決定論テストで確かめる。
2. The 本仕様 shall 根の解決（`AREKA_ROOT` あり／なし・実在しない根）を決定論テストで確かめる（環境変数の読み口は値を注入できる形にし、プロセスの env を書き換えない）。
3. The 本仕様 shall 起動解決の分岐（要件 4.9・5.10）を、プロセスを起こさず純粋な判断として踏む決定論テストで確かめる。
4. The 本仕様 shall 足すテストを判断分岐（列挙の採否・記憶の往復・解決順・告知の場面）に限り、既に確かめられている配線（マウント→SHIORI→sink・窓の配置・終了の指示）を再テストしない。
5. When 実機で確認する, the 開発者 shall ① `AREKA_ROOT` に検体の根を渡し argv なしで起動して同梱バルーンで会話が出ること、② 記憶が書かれ（アプリの `profile/areka/sylphya.toml` の `[last] ghost`・ゴーストの `ghost/master/profile/areka/sylphya.toml` の `[last] balloon`／`shell`）再起動で同じゴーストと同じバルーンが立つこと、③ 空の根で起動して告知が出て終了コードが 0 以外であること、④ 有界の自動終了（`AREKA_APP_SMOKE_EXIT_MS`）の後にプロセスが残っていないこと、を見る。
6. While 実機で確認する, the 開発者 shall 自分が起こしたと確認できたプロセス以外を止めない。

### Requirement 9: 裁定候補（要件ディスカッションで確定）

**Objective:** As a 開発者, I want brief が挙げた裁定候補と要件化で増えた分かれ目を要件の段階で確定しておくこと, so that 設計がこの形で進む

#### Acceptance Criteria

1. The 本仕様 shall **裁定 1（根の既定・brief の候補 ⑵・2026-09-23 確定）**として、根の既定を「exe の隣」（ポータブル・zip を展開して起動＝SSP と同じ体験・書き込み権限の問題が起きない場所へ利用者が置く）とし、`%APPDATA%\areka\` は α では採らない（要件 1.2）。
2. The 本仕様 shall **裁定 2（smoke テストとダミー窓・brief の崩れた前提 1・2026-09-23 確定）**として、smoke テストを「陳腐化として除く」のではなく 3 方向へ**更新**し（要件 7.5）、検証用ダミー窓を退役させ（要件 6.6）、告知の抑止は `AREKA_` 名前空間の環境変数 1 つで行う（要件 6.5。`AREKA_APP_SMOKE_EXIT_MS` の有無に相乗りしない＝自動終了と告知の抑止は別の関心）。
3. The 本仕様 shall **裁定 3（複数から 1 つを選ぶ規則・2026-09-23 確定）**として、記憶が無く候補が 2 つ以上のとき「告知して終了」ではなく、ゴーストは**既定ゴースト `emo2`（areka の配布物に必ず同梱）を起動し、居なければ無作為に 1 体**（要件 4.4・4.5・4.11）、バルーンは既定バルーンの後に無作為に 1 つ（要件 5.6）とする。第三者が切替メニュー（後続 spec）を持たない段階で止めてしまうと出口が無いため。列挙の並び（要件 2.5）は解決順に使わない。
4. The 本仕様 shall **裁定 4（バルーンの記憶の置き場と順序・2026-09-23 確定）**として、brief の「アプリの記憶・同梱 → 記憶」を覆し、`areka.last.balloon` を**ゴーストの記憶（`Ghost` スコープ）**に置き、解決順を**記憶 → 同梱**とする（要件 3.1・3.3・5.2・5.3）。開発者裁定「ゴーストごとに選択バルーンは異なる」＝利用者が選んだバルーンは同じゴーストの再起動で同梱へ戻らず、別のゴーストへ替えたとき前のゴーストの選択は持ち越されない。後続 `shell-balloon-switch` の brief の鍵の所在（Adjacent expectations）もこれに従う。
5. The 本仕様 shall **裁定 5（記憶を書く条件・2026-09-23 確定）**として、ゴーストとバルーンの記憶は argv 以外の経路で決まったときだけ書き、argv で決まったときは根の内外を問わず書かない（要件 3.2・3.3・3.5）。brief の「根の直下にあるときだけ書く」は Windows のパス比較（大小無視・`\?\` 接頭辞・`canonicalize` の失敗）を要し、外れたとき黙って起きるので採らない。趣旨「開発者の実機起動が第三者向けの記憶を汚さない」は同じ。
6. Where 設計・実装の途中で裁定 1〜5 のいずれかを覆す必要が判明する, the 本仕様 shall 開発者へ議題として上げ、確定を待ってから要件・設計の該当箇所を改める。
