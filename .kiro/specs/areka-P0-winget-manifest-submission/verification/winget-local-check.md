# 手元の確かめ: 提出するマニフェストで winget から入れて `areka` で起動する

- 日付: 2026-10-10
- 機械: 開発機（Windows 11 Pro 10.0.26300.9550・26H2・x64）・winget v1.29.380
- コミット: `f305cee6`（着手のときの枝の先頭。枝は `claude/areka-p0-winget-manifest-1c4fd7`）
- 判定: **入れて起動する確かめは合格**（6 項目とも合格・タスク 2.2・2026-10-10 13:01〜13:03。下の「入れて起動する」）。上げ直しの実測（タスク 3.2）と外し方の実測（タスク 3.3）は測れて、**どちらも消えた物は 6**（後から入れたゴースト 1・後から入れたバルーン 3・ゴーストの記憶 2。areka の記憶は残った）。消えた物が在るので、雛形の 2 つのロケールへ注意書きを足し、`winget validate` を通して入れ直した（タスク 3.4・14:06〜14:07。インストールの最後に、日本語の注意書きが出た）。**利用者の側の後片付けは済んだ（4 項目とも 0）**。ただし winget だけでは 0 にならなかった: `--purge` を付けて外した後も、4 項目のうち 2 つ（入れ先のフォルダと、利用者の PATH の項目 1 件）が残り、フォルダは中身の一覧を取ってから AI が消し（14:17:20）、PATH の項目は開発者が手で消した（14:18 ごろ。利用者の PATH は、入れる前の文字列と 1 字も違わない形に戻った）（下の「後片付けの確かめ」）。機械の全員向けの実測も測れた（タスク 3.5・14:18〜14:23）: ゴーストは立ったが、areka の記憶は書けず、ゴーストを後から入れることもできなかった（下の「機械の全員向けの実測」）。**機械の側の後片付けも済んだ（4 項目とも 0）**。こちらも winget だけでは 0 にならなかった: 開発者が管理者の端末で `--purge` を付けて外すと（14:41:08）、入れ先のフォルダは消えたが、機械の PATH の項目 1 件が残り、開発者が手で消した（機械の PATH は、入れる前の文字列と 1 字も違わない形に戻った）。**設定 `LocalManifestFiles` は、開発者が 14:43:20 に元の `false` へ戻した**（下の「変えた設定」）。**見つかった件の起票も済んだ**（タスク 3.6・設定が戻っていることを確かめた後の 15:50〜17:20 ごろ。見つかった 6 件を、新しい spec 2 本＝`areka-P0-user-data-root`・`areka-P0-write-failure-notice` と、今ある spec 2 本＝`areka-P0-mcp-kanade-tools`・`areka-P0-winget-release-automation` へ割り付けた。下の「見つかった件と起票」）。まだ済んでいないのは、提出（タスク 4.2・4.3）
- 提出する版: `0.0.2`（Release `v0.0.2`。決め方は「雛形の検査」の「ハッシュの突き合わせ」の 1）

## 変えた設定

変える設定は次の 1 つだけ（開発者の手で・要件 3.4）。AI は winget と OS の設定を変えない。オンにするのも戻すのも、開発者が管理者の端末で打つ。時刻はどれも 2026-10-10（+09:00）。

| 設定 | 変える前の値 | 変える前の値を読んだ時刻 | オンを確かめた時刻 | 戻した時刻 | 戻した後の値 |
|---|---|---|---|---|---|
| winget の `LocalManifestFiles`（管理者の端末で `winget settings --enable LocalManifestFiles`） | `false` | 11:44:01 | 12:57:16 に `true` を確認 | **14:43:20**（開発者が管理者の端末で `winget settings --disable LocalManifestFiles`。時刻は winget 自身の記録から。AI が `false` を読んだのは 14:43:46 と 14:49:44） | **`false`**（変える前と同じ） |

- 変える前の値の読み方: `winget settings export`（読むだけ）が返す JSON の `adminSettings.LocalManifestFiles`。同じ JSON の管理者向けの設定は 6 つで、6 つとも `false` だった（`BypassCertificatePinningForMicrosoftStore`・`ConfigurationProcessorPath`・`InstallerHashOverride`・`LocalArchiveMalwareScanOverride`・`LocalManifestFiles`・`ProxyCommandLineOptions`）。
- 開発者に打ってもらうコマンド（管理者の端末）: `winget settings --enable LocalManifestFiles`
- オンにしたのは開発者（管理者の端末で上のコマンドを打った）。開発者が伝えた winget の答えは 1 行で、`管理者設定 'LocalManifestFiles' を有効にしました。`（この文は AI が自分で見た物ではなく、開発者から聞いた物）。
- オンの確かめ（AI が読んだ・12:57:16）: `winget settings export` の `adminSettings.LocalManifestFiles` が `true`（終了コード 0・標準エラー 0 バイト）。
- **変えた設定はこの 1 つだけ**。同じ回の読みで、管理者向けの設定 6 つのうち `true` は `LocalManifestFiles` の 1 つだけで、残りの 5 つは変える前と同じ `false`。下の「読んで記録するだけの設定」も同じ時刻に読み直して、変える前と同じだった（開発者モードは値 `0`＝オフ・利用者の設定ファイルは無いまま＝フォルダ 4 つ・ファイル 0 件）。
- 組織の決まり（グループ ポリシー）で winget の設定が固定されていないことも読んだ（11:45:25）。`HKLM:\SOFTWARE\Policies\Microsoft\Windows\AppInstaller` と `HKCU:\SOFTWARE\Policies\Microsoft\Windows\AppInstaller` はどちらもキーが無い。
- 利用者の PATH のこと（上の表には入れない。確かめのために決めて変えた設定ではなく、winget が入れるときに足した物の後片付け）: 利用者の PATH の登録（`HKCU:\Environment` の `Path`）は、winget の入れる操作が入れ先のフォルダの項目を 1 件足し（13:01）、winget の外す操作では消えなかったので、開発者が手でその 1 件を消した（14:18 ごろ）。消した後の文字列は、入れる前に取っておいた文字列と 1 字も違わない（12 項目・524 字。作業を進めている AI が 14:19:22 に、この記録を書いた AI が 14:22:20 に読んだ。下の「後片付けの確かめ」の「利用者の側」の 8）。AI は PATH を変えていない。
- 設定を戻したこと（タスク 3.5）: 開発者に打ってもらったコマンド（管理者の端末）は `winget settings --disable LocalManifestFiles`。戻したのは開発者。**開発者の端末に出た文は取れていない**。代わりに、winget 自身の記録（`%LOCALAPPDATA%\Packages\Microsoft.DesktopAppInstaller_8wekyb3d8bbwe\LocalState\DiagOutputDir\` の、その回のファイル。写しは `target\winget-check\logs\3.5-final-winget-diag-settings-disable.txt`）を読んだ: このコマンドが 14:43:20.772 に始まり、14:43:20.805 に `Leaf command succeeded: root:settings` で終わっている（管理者に上げたプロセス＝記録の `Level[1]`）。
- 戻した後の確かめ（AI が読んだ・`winget settings export` の `adminSettings.LocalManifestFiles`）: 作業を進めている AI（親のセッション）の読みでは、14:41:46 はまだ `true`・14:43:46 に `false`（その回の出力のファイルは無い）。この節を書いた AI が 14:49:44 に読み直して **`false`**（終了コード 0・標準エラー 0 バイト）。同じ回の読みで、管理者向けの設定は 6 つとも `false`＝変える前（11:44:01）と同じ。出た文は `target\winget-check\logs\3.5-final-settings-export.stdout.txt` と `3.5-final-machine.txt`。
- **設定がオンだった時間は 12:56:29〜14:43:20（約 1 時間 47 分）**。始まりも終わりも winget 自身の記録の時刻（オンにした回の写しは `logs\3.5-final-winget-diag-settings-enable.txt`。その 19 秒前の 12:56:10 に同じコマンドが 1 度、管理者の端末でなかったために断られている＝`0x8a150019`。`winget error 0x8a150019` が返す名前は `APPINSTALLER_CLI_ERROR_COMMAND_REQUIRES_ADMIN`）。この間に、手元のマニフェストから入れた・上げた操作（`--manifest` を付けた `winget install`・`winget upgrade`）は 4 回で、どれも本仕様の雛形か確かめ専用のマニフェスト（13:01:26・14:07:35 の `dist\winget\0.0.2`／13:42:18 の `target\winget-check\upgrade-0.0.2.1`／14:18:56 の `dist\winget\0.0.2` に `--scope machine`）。ほかのマニフェストを入れた回は 0（winget 自身の記録を、この間の全部＝56 回分数えた。`logs\3.5-final-winget-diag-window.txt`）。この 56 回には、開発者が自分の用事で打った `winget upgrade --all` の 1 回が入っている（12:56:38〜12:59:04・管理者の端末・winget のソースから取り寄せる物で、手元のマニフェストは使っていない。本仕様の操作ではない）。
- 機械の PATH のこと（上の表には入れない。利用者の PATH と同じく、決めて変えた設定ではなく、winget が入れるときに足した物の後片付け）: 機械の PATH の登録（`HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment` の `Path`）は、winget の機械の全員向けの入れる操作が入れ先のフォルダの項目を 1 件足し（14:19:10。18 項目 → 19 項目）、`--purge` を付けた外す操作（14:41:08）では消えなかったので、**開発者が手で**その 1 件を消した（14:43:46 より後・14:46:58 より前。管理者の端末で）。消した後の文字列は、どの入れる操作よりも前に取っておいた文字列（`logs\2.2-pre-machine-path.txt`）と 1 字も違わない（18 項目・646 字・値の種類 `ExpandString`。作業を進めている AI が 14:46:58 に、この節を書いた AI が 14:49:29 に、大文字と小文字を区別して比べた。下の「後片付けの確かめ」の「機械の側」）。AI は PATH を変えていない。
- **確かめのために決めて変えた winget と OS の設定は `LocalManifestFiles` の 1 つだけで、元の値 `false` に戻っている**。下の「読んで記録するだけの設定」も 14:49:44 に読み直して、変える前と同じだった（開発者モードは値 `0`＝オフ・利用者の設定ファイルは無いまま＝フォルダ 4 つ・ファイル 0 件・控えも無い・組織の決まりのキーは `HKLM`・`HKCU` とも無い・winget は v1.29.380）。利用者の PATH と機械の PATH は、どちらも入れる前の文字列と同じ。

### 読んで記録するだけの設定（変えない・要件 4.5）

| 設定 | 読んだ値 | 読み方 | 読んだ時刻 |
|---|---|---|---|
| Windows の開発者モード | **オフ**（値 `0`） | レジストリ `HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock` の `AllowDevelopmentWithoutDevLicense`（キーは在り、値はこの 1 つだけ） | 11:44:01 |
| 利用者の設定の「外すときに入れ先を丸ごと消す」（`uninstallBehavior.purgePortablePackage`） | **欄なし＝既定（オフ）** | `winget settings export` が `userSettingsFile` として示す `%LOCALAPPDATA%\Packages\Microsoft.DesktopAppInstaller_8wekyb3d8bbwe\LocalState\settings.json`。**このファイル自体が無い**（利用者の設定は 1 つも書かれていない）。同じフォルダに在るのはフォルダ 4 つだけで、ファイルは 0 件（控えの `settings.json.backup` も無い） | 11:44:01 |
| ポータブルな物の入れ先の付け替え（`installBehavior.portablePackageUserRoot`・`portablePackageMachineRoot`） | **付け替えなし＝既定の入れ先**（利用者向けは `%LOCALAPPDATA%\Microsoft\WinGet\Packages\`・機械の全員向けは `%ProgramFiles%\WinGet\Packages\`） | 上と同じ。設定ファイルが無いので、`installBehavior` の欄も無い | 11:44:01 |

- 開発者モードは変えない（雛形は `ArchiveBinariesDependOnPath: true` で、リンクを作らないので要らない）。
- winget の版は v1.29.380（`winget --version`・11:44:01）。

### 変える前の機械の状態（読むだけ・後のタスクが見比べる元）

読んだ時刻は 11:44:37〜11:45:03。**5 項目とも、areka の物は 0**（前の確かめの残りは無い）。何も消していないし、何も入れていない。

| 項目 | 見方 | 結果 |
|---|---|---|
| `winget list` の areka の行 | `winget list --name areka` と `winget list --id Areka`（どちらも部分一致で引く） | **0 行**。どちらも `入力条件に一致するインストール済みのパッケージが見つかりませんでした。` と出て、終了コードは -1978335212（16 進で `0x8A150014`。見つからなかったときの値）・標準エラーは 0 バイト |
| 利用者向けの入れ先 `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource` | `Test-Path` | **無い**。`Packages\` の下で名前が `Areka` で始まる物も 0 件（`Packages\` 自体は在り、ほかのパッケージの物が 2 つ在る） |
| `%LOCALAPPDATA%\Microsoft\WinGet\Links\areka.exe` | `Test-Path` | **無い**。`Links\` の下で名前が `areka` で始まる物も 0 件（`Links\` 自体は在る） |
| PATH の登録の中の `WinGet\Packages\Areka` を含む項目 | 利用者の側はレジストリ `HKCU:\Environment` の `Path`、機械の側は `HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment` の `Path` を、環境変数を開かない形で読み、`;` で分けて数えた | 利用者の側は **0 件**（全 12 項目）・機械の側は **0 件**（全 18 項目）。`areka` の字を含む項目も、どちらも 0 件 |
| 新しい端末での `areka` の解決 | 上の 2 つの登録（機械の側・利用者の側の順）から PATH を組み直した新しい `pwsh -NoProfile` のプロセスで `Get-Command areka -All` を打った。同じ PATH で `where.exe areka` も打った | **解決しない**（`Get-Command` は 0 件。`where.exe` は終了コード 1・標準出力 0 バイト） |

あわせて読んだこと:

- 機械の全員向けの入れ先 `%ProgramFiles%\WinGet\Packages\` は、フォルダ自体がまだ無い（`Areka` で始まる物は 0 件）。`%ProgramFiles%\WinGet\Links\areka.exe` も無い。
- 利用者の側の PATH の登録で `WinGet` の字を含む項目は 1 件で、ほかのパッケージ（`rhysd.actionlint`）の入れ先のフォルダ。機械の側は 0 件。`%LOCALAPPDATA%\Microsoft\WinGet\Links` は、どちらの登録にも入っていない。

見方が当たりを出せること（0 と書く前に確かめた）:

- 同じ形の `winget list --name PowerShell` は 2 行を返し、終了コード 0 だった。＝この引き方は、入っている物が在れば行を出す。
- 打った字面のこと: areka を引いた 2 回の `winget list` には `--accept-source-agreements --disable-interactivity` を、`PowerShell` を引いた 1 回には `--disable-interactivity` を付けていた。`--accept-source-agreements` は要らなかった（付けなかった 3 回目も同じ機械で問いを出さずに通った）ので、この後の winget のコマンドには付けない。
- 新しいプロセスでの解決は、同じプロセスの中で `Get-Command pwsh -All` が 2 件・`Get-Command winget -All` が 1 件を返した。＝組み直した PATH は効いていて、在るコマンドは見つかる。
- 出た文は `target\winget-check\logs\pre-2.1-*.txt` に取ってある（winget の文は UTF-8 として読んだ）。

## 雛形の検査

### `winget validate`

結果: **成功**（雛形を置いたとき・タスク 1.2）。

- 対象: `dist/winget/0.0.2/` の 4 ファイル（`Areka.Areka.Portable.yaml`・`Areka.Areka.Portable.installer.yaml`・`Areka.Areka.Portable.locale.en-US.yaml`・`Areka.Areka.Portable.locale.ja-JP.yaml`）。
- コマンド: `winget validate --manifest dist\winget\0.0.2`（ワークツリーの根で打った。設定は何も変えていない）
- 時刻: 2026-10-10 11:34:22（+09:00）
- winget が出した文（標準出力の全部。警告の行は 0）: `マニフェストの検証は成功しました。`
- 終了コード: 0（標準エラーは 0 バイト）
- 出た文は `target\winget-check\logs\validate-1.2.stdout.txt` に取ってある（UTF-8 として読んだ）。
- 同じコマンドは 11:33:38 にも 1 度打っていて、同じ文と終了コード 0 だった。上の時刻は、下の「検査が赤を出せること」を見た後の打ち直しのもの。

検査が赤を出せること（1 回だけ確かめた）: 4 ファイルを `target\winget-check\validate-negative\` へ写し、installer の `InstallerType: zip` を `InstallerType: zap` に変えた物へ同じ `winget validate --manifest` を打つと、`マニフェストの検証に失敗しました。` と `Manifest Error: Invalid field value. [InstallerType]` が出て、終了コードは -1978335191（0 でない）だった。＝この検査は書式に合わない欄を見つけて止まる。

あわせて見たこと（どれも PowerShell で、ワークツリーの根から）:

| 見たこと | 見方 | 結果 |
|---|---|---|
| 4 ファイルとも同じ名乗り・同じ版・同じ書式の版 | `Select-String -Path dist\winget\0.0.2\*.yaml -Pattern '^PackageIdentifier: (.*)$' -CaseSensitive` を `PackageVersion`・`ManifestVersion` でも打ち、行の数と値の種類を数えた | 3 つとも 4 行・値は 1 種類（`Areka.Areka.Portable`・`0.0.2`・`1.12.0`） |
| `ManifestType` | 同じ形で `^ManifestType: ` を引いた | `version`・`installer`・`defaultLocale`・`locale` が 1 つずつ |
| 1 行目が書式の場所を示す行 | 各ファイルをバイトで読み、UTF-8 として解いた 1 行目を見た | 4 ファイルとも `# yaml-language-server: $schema=https://aka.ms/winget-manifest.<種類>.1.12.0.schema.json`（種類は上の `ManifestType` と同じ） |
| UTF-8 の BOM なし | `[System.IO.File]::ReadAllBytes` の先頭 3 バイト | 4 ファイルとも `23 20 79`（`# y`）。`EF BB BF` は 0 件。UTF-8 として厳密に解いて誤りは 0 |
| 置き場に YAML 以外が無い | `Get-ChildItem dist\winget\0.0.2 -Force -Recurse` | 4 件・拡張子が `.yaml` でない物は 0 件 |
| ハッシュは大文字 | installer の `InstallerSha256` の 2 行 | 上の「ハッシュの突き合わせ」の表の値と同じ字面（x64 は `66D3A9A0…6A751C9C`・arm64 は `77C5356A…9FDD79FB`） |
| 説明の文 | 雛形の `ShortDescription` が、設計の「雛形の欄」の初稿の文と字面で同じかを比べた | 英語（105 字）・日本語（49 字）とも同じ |

雛形が持つ欄の全部（欄の名前と、4 ファイルを通した行の数）: `PackageIdentifier` 4・`PackageVersion` 4・`ManifestType` 4・`ManifestVersion` 4・`DefaultLocale` 1・`InstallerType` 1・`NestedInstallerType` 1・`NestedInstallerFiles` 1（`RelativeFilePath` 1・`PortableCommandAlias` 1）・`ArchiveBinariesDependOnPath` 1・`ReleaseDate` 1・`Installers` 1（`Architecture` 2・`InstallerUrl` 2・`InstallerSha256` 2）・`PackageLocale` 2・`PackageName` 2・`ShortDescription` 2・`Tags` 2・`Publisher` 1・`PublisherUrl` 1・`PublisherSupportUrl` 1・`PackageUrl` 1・`License` 1・`LicenseUrl` 1・`ReleaseNotesUrl` 1。`InstallationNotes` はまだ無い（足すのは、実測で消える物が在ると分かったときだけ）。

注意書きを足したとき（タスク 3.4・要件 2.2）: **成功**。

- 対象: 同じ 4 ファイル。2 つのロケールのファイルへ `InstallationNotes` の行を 1 行ずつ足した後の物（足した文は、下の「後片付けの確かめ」の「利用者の側」の 2）。
- コマンド: `winget validate --manifest dist\winget\0.0.2 --disable-interactivity`（ワークツリーの根で打った。設定は何も変えていない）
- 時刻: 2026-10-10 14:06:19（+09:00）
- winget が出した文（標準出力の全部。警告の行は 0）: `マニフェストの検証は成功しました。`
- 終了コード: 0（標準エラーは 0 バイト）
- 出た文は `target\winget-check\logs\3.4-validate.stdout.txt` に取ってある（UTF-8 として読んだ）。
- 雛形が持つ欄の変わり方: `InstallationNotes` が 0 → 2（英語と日本語のファイルに 1 つずつ）。ほかの欄の数は、上の一覧のまま。

PR から写し戻したとき（タスク 4.2・要件 2.2・2.3）: **成功**。

- 対象: 同じ 4 ファイル。winget-pkgs へ出した PR（https://github.com/microsoft/winget-pkgs/pull/450070）の 4 ファイルの中身で、1 バイトも変えずに置き換えた後の物（置き換えは 17:36:04。見比べの全部は `submission.md` の「写し戻しの見比べ」）。
- コマンド: `winget validate --manifest dist\winget\0.0.2 --disable-interactivity`（ワークツリーの根で打った。設定は何も変えていない）
- 時刻: 2026-10-10 17:36:46.910〜17:36:47.777（+09:00）
- winget が出した文（標準出力の全部。警告の行は 0）: `マニフェストの検証は成功しました。`
- 終了コード: 0（標準エラーは 0 バイト）
- 出た文と時刻は `target\winget-check\logs\4.2-validate.stdout.txt`・`4.2-validate.stderr.txt`・`4.2-validate.times.txt` に取ってある（UTF-8 として読んだ）。
- 雛形が持つ欄の変わり方: 欄の名前・値・数は、どれも変わっていない（上の一覧と、注意書きの `InstallationNotes` 2 のまま）。変わったのは字面だけ＝4 ファイルとも 1 行目に道具の名前の行（`# Created using wingetcreate 1.12.13.0`）が足されて、書式の場所を示す行は 2 行目になった／installer のファイルの `ReleaseDate` がファイルの最後へ動いた／作業の場所のファイルの行末が LF から CRLF になった（コミットに入る中身は LF のまま）。BOM は前も後も無い。

### ハッシュの突き合わせ（要件 1.5・5.3）

結果: **x64 と arm64 のどちらも、3 つの値が一致した**（一致 2 / 2・違い 0）。

1. 提出する版を決めた。2026-10-10 11:27:37（+09:00）に `gh release list --repo ekicyou/areka --limit 20 --json tagName,name,isDraft,isPrerelease,isLatest,publishedAt` で公開済みの Release の一覧を読んだ。返ってきたのは次の 1 件だけで、これより新しい Release は 0 件。だから提出する版は `0.0.2` とした（開発者に決めてもらう場合には当たらない）。

   | タグ | 下書き | 先行版 | 最新の印 | 公開の時刻（UTC） |
   |---|---|---|---|---|
   | `v0.0.2` | いいえ | いいえ | はい | 2026-10-06T12:49:21Z |

2. 2026-10-10 11:28:41（+09:00）に、`gh release download v0.0.2 --repo ekicyou/areka --dir target\winget-check\release --clobber` に 4 つのファイル名を `--pattern` で名指しして、zip 2 つと `.sha256` 2 つを取ってきた（終了コード 0）。
3. 3 つの値を次のとおりに読んだ。
   - Release の `.sha256` に書かれた値: 取ってきた `areka-0.0.2-{arch}.zip.sha256` の、行の先頭の 64 桁。ファイルの中身は「小文字の 64 桁・空白 2 つ・zip のファイル名・改行」の 1 行だった。
   - GitHub が示す値: `gh release view v0.0.2 --repo ekicyou/areka --json assets` が返す、zip の `digest`（`sha256:` の後ろの 64 桁）。
   - 手元で計算した値: 取ってきた zip に `Get-FileHash -Algorithm SHA256` を掛けた値。
4. 大文字と小文字の違いを問わずに比べた（3 つとも大文字にそろえて、文字列として同じかを見た）。下の表は大文字で書いてある。雛形にはこの値を大文字で書く。

| arch | zip | 大きさ（バイト） | Release の `.sha256` に書かれた値 | GitHub が示す値 | 手元で計算した値 | 一致 |
|---|---|---|---|---|---|---|
| x64 | `areka-0.0.2-x64.zip` | 8695089 | `66D3A9A01A611DD36B9C3AF276C0CD071EF87C28BA82B6CF8FC314FA6A751C9C` | `66D3A9A01A611DD36B9C3AF276C0CD071EF87C28BA82B6CF8FC314FA6A751C9C` | `66D3A9A01A611DD36B9C3AF276C0CD071EF87C28BA82B6CF8FC314FA6A751C9C` | 一致 |
| arm64 | `areka-0.0.2-arm64.zip` | 8552648 | `77C5356A8F77CB121F828CFE761D7C63588C822790FB80BE78234F0F9FDD79FB` | `77C5356A8F77CB121F828CFE761D7C63588C822790FB80BE78234F0F9FDD79FB` | `77C5356A8F77CB121F828CFE761D7C63588C822790FB80BE78234F0F9FDD79FB` | 一致 |

- 取ってきた zip の大きさは、GitHub が Release の物として示す大きさ（x64 は 8695089・arm64 は 8552648）と同じだった。
- `.sha256` のファイルの大きさは、x64 が 86 バイト・arm64 が 88 バイト。

### 持たない欄

結果: **持たない欄は 0 件**（2026-10-10・タスク 1.2・`dist/winget/0.0.2/` の 4 ファイルの中身を検索した）。

探した語（大文字と小文字の違いは問わない。行のどこに在っても当たる）:

- 名乗りとしての `Areka.Areka`（後ろに `.Portable` が続かない `Areka.Areka`）・`Moniker`
- 署名: `SignatureSha256`
- インストーラー向け: `InstallerSwitches`・`ProductCode`・`AppsAndFeaturesEntries`・`ElevationRequirement`・`InstallModes`
- 依存: `Dependencies`
- `Scope`・`UpgradeBehavior`・`MinimumOSVersion`
- ショートカット: `shortcut` の語

打ったコマンド（PowerShell。`Select-String` は既定で大文字と小文字を区別しない）:

```powershell
$pat = 'Moniker|SignatureSha256|InstallerSwitches|ProductCode|AppsAndFeaturesEntries|ElevationRequirement|InstallModes|Dependencies|Scope|UpgradeBehavior|MinimumOSVersion|shortcut|Areka\.Areka(?!\.Portable)'
(Select-String -Path dist\winget\0.0.2\*.yaml -Pattern $pat).Count
```

- 当たった行: **0 件**。

検索が当たりを出せること（0 と書く前に 1 回確かめた）:

- 同じ `$pat` の後ろに `|PackageIdentifier` を足して打つと 4 行に当たった（4 ファイルに 1 行ずつ）。＝この検索は 4 ファイルを読めていて、語が在れば当たる。
- 名乗りの部分だけを `Areka\.Areka(?!\.Portabl\b)` に変えて打つと 4 行に当たった。＝「後ろに続く語を見て外す」部分は、続く語が違えば当たる。
- `INSTALLERTYPE`（全部大文字）で打つと 2 行に当たった（`InstallerType` と `NestedInstallerType`）。＝大文字と小文字の違いは問わずに当たる。

設計の「設計で決めたこと」5 が入れないと決めたほかの欄（`Description`・`Author`・`Copyright`）も、欄の名前として 0 件（`^\s*-?\s*(Description|Author|Copyright|InstallationNotes):` で 0 行。`ShortDescription` は別の欄なので当たらない）。

### zip が変わらないこと

結果: **zip へ写す物は `dist/README.txt` の 1 つだけ・`dist/winget/**` は入らない**（2026-10-10 11:39（+09:00）・タスク 1.3・要件 2.4。配布スクリプトと workflow を読んで数えた。zip は作り直していない）。

打ったコマンド（PowerShell。ワークツリーの根から。`Select-String` は既定で大文字と小文字を区別しない。`dist/` と `dist\` のどちらの書き方にも当たる）:

```powershell
$files = @('tools\package.ps1') + @(Get-ChildItem .github\workflows\*.yml | ForEach-Object { Resolve-Path -Relative $_.FullName })
foreach ($f in $files) { "$f : $(@(Select-String -Path $f -Pattern 'dist[/\\]').Count)" }   # dist/ か dist\ を指す行
foreach ($f in $files) { "$f : $(@(Select-String -Path $f -Pattern 'dist').Count)" }          # dist の字が在る行（広げた検索）
foreach ($f in $files) { "$f : $(@(Select-String -Path $f -Pattern 'winget').Count)" }
@(Get-ChildItem tools, .github -Recurse -File | Select-String -Pattern 'dist[/\\]') | Group-Object Path
```

`dist/`（`dist\`）を指す行の数:

| ファイル | `dist/` を指す行 | `dist` の字が在る行 | `winget` の字が在る行 |
|---|---|---|---|
| `tools/package.ps1` | **2 行** | 2 行（左と同じ 2 行） | 0 行 |
| `.github/workflows/release.yml` | **0 行** | 0 行 | 0 行 |
| `.github/workflows/crates-io.yml` | **0 行** | 0 行 | 0 行 |

- workflow は 2 ファイルで、この 2 つが `.github/workflows/*.yml` の全部。workflow は合わせて 0 行。
- `tools/` と `.github/` の下の全部のファイルへ広げて引いても、当たるのは `tools/package.ps1` の同じ 2 行だけ（ほかのファイルは 0 行）。

当たった 2 行が何の行か:

| 行 | 何をする行か | zip へ写すか |
|---|---|---|
| 段「assemble」の `Copy-Item -LiteralPath 'dist/README.txt' -Destination $stage` | `dist/README.txt` の 1 ファイルを、zip にするフォルダ（`target/package/stage-{arch}`）へ写す。`-LiteralPath` でファイルを 1 つ名指ししていて、`-Recurse` も `*` も無い | **写す**（これが 1 つ） |
| 出来た zip の中身を判定する関数の「8. README・ライセンス・謝辞は写す元とバイトが同じ」の `foreach` の行 | zip の中の `README.txt` を、写す元の `dist/README.txt` とバイトで比べるために読む | 写さない（読んで比べるだけ） |

あわせて読んだこと:

- `tools/package.ps1` の `Copy-Item` で始まる行は 8 行で、8 行とも段「assemble」に在る。写す元に `dist` の字を持つのは上の 1 行だけ。残りの 7 行の写す元は、`areka.exe`・補助の exe・`nar-sample-path` が返すフォルダ 3 つ（`-Recurse` が付くのはこの 3 行だけ）・`LICENSE-MIT`・謝辞のファイルで、どの行にも、その写す元の変数を決める行にも `dist` の字は無い（`dist` の字が在る行が全部で 2 行なので）。
- zip を作る行は `[IO.Compression.ZipFile]::CreateFromDirectory` の 1 行だけ（`Compress-Archive` は 0 行）。固める元は `stage-{arch}` のフォルダで、段「assemble」が毎回消して作り直す。＝zip に入るのは、上の 8 行が写した物と、同じ段が書く `BUILD-INFO.txt` だけ。
- `dist/winget` を名指しする行は 0 行。`dist/` をフォルダごと写す行・`dist/*` のように何でも拾う行も 0 行（`dist` の字が在る 2 行は、どちらも `dist/README.txt` と書いてある）。
- workflow の `release.yml` は、zip を作る段で `./tools/package.ps1 -Arch all` を呼ぶだけで、自分では `dist/` の物を写さない（`dist` の字が 0 行）。

検索が当たりを出せること（0 と書く前に確かめた）:

- 同じ `'dist[/\\]'` の検索が、`tools/package.ps1` では中身を知っている `Copy-Item -LiteralPath 'dist/README.txt'` の行に当たっている。＝この検索は語が在れば当たる。workflow の 0 行は、同じコマンドを同じ回に打った結果。
- `'dist'` へ広げた検索を `tools/` の全部へ打つと、`tools/perf/invoke-followup-checks.ps1` の `$distinctDpi` の 3 行にも当たった（`dist/` とは関係ない語）。＝広げた検索は、字が在れば別の語の中でも当たる。それでも workflow は 0 行。

zip を作り直さなかった理由: `tools/**` と `.github/workflows/**` の変更が 0 だから、写す経路は動いていない。

- `git diff --name-only main...HEAD -- tools .github` は 0 件（`main` は `0205adff`）。`git status --porcelain -- tools .github` も 0 行。
- 同じ `git diff --name-only main...HEAD` を `-- dist` で打つと 4 件（雛形の 4 ファイル）を返した。＝このコマンドは変更が在れば出す。
- だから、この枝の `tools/package.ps1` は `dist/winget/` を足す前と 1 バイトも変わらず、上で読んだとおり `dist/` からは `README.txt` の 1 つしか写さない。`dist/winget/0.0.2/` に 4 ファイルが在っても無くても、zip の中身の構成は同じになる。

steering の直し（要件 2.7）: `.kiro/steering/structure.md` の「その他の最上位」の行にある `dist/` の説明の後ろへ、「と、`winget/`＝winget へ提出するマニフェストの雛形（初回に出した版の写し・配布 zip には入らない）」を足した。`git diff --numstat` は 1 行の足しと 1 行の消し（同じ 1 行の書き替え）で、字の単位で見比べる `git diff --word-diff-regex=.` では、足した字がこの 1 か所だけ・消した字は 0。

## 入れて起動する

結果: **合格**（6 項目とも合格・不合格は 0。2026-10-10・タスク 2.2・要件 3.1・3.2・3.3）。時刻はどれも 2026-10-10（+09:00）。パスの `%LOCALAPPDATA%` は、アカウントの名前を書かないための置き換え（出た文の中の実際の字面は `C:\Users\` から始まる、変数を開いた形）。

入れた物はこの後のタスクが続けて使うので、**外していない**（設定 `LocalManifestFiles` もオンのまま）。

### 1. 普段の権限で入れた（要件 3.1）

- 入れる直前（同じ回の、コマンドを打つ前）に読んだこと: 入れ先 `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource` は無い・`%LOCALAPPDATA%\Microsoft\WinGet\Links\areka.exe` は無い・利用者の側の PATH の登録は 12 項目・機械の側は 18 項目・`winget settings export` の `adminSettings.LocalManifestFiles` は `true`（終了コード 0）。打った端末は管理者の権限ではない（管理者の役割を持つかを読んで `False`）。
- コマンド（ワークツリーの根で。同意を先に済ませる `--accept-*`・`--scope`・`--force`・`--ignore-security-hash` は付けていない）:

  ```powershell
  winget install --manifest dist\winget\0.0.2 --disable-interactivity
  ```

- 始めた時刻 13:01:26・終わった時刻 13:01:48（22 秒）・**終了コード 0**・標準エラーは 0 バイト。問い（同意・管理者への切り替え）は 1 つも出なかった。
- winget が出した文（標準出力の全部＝11 行。省いた行は無い。UTF-8 として読んだ）:

  ```text
  見つかりました areka (portable) [Areka.Areka.Portable] バージョン 0.0.2
  このアプリケーションは所有者からライセンス供与されます。
  Microsoft はサードパーティのパッケージに対して責任を負わず、ライセンスも付与しません。
  ダウンロード中 https://github.com/ekicyou/areka/releases/download/v0.0.2/areka-0.0.2-x64.zip
  インストーラーハッシュが正常に検証されました
  アーカイブを展開しています...
  アーカイブが正常に展開されました
  パッケージのインストールを開始しています...
  パス環境変数が変更されました; 新しい値を使用するにはシェルを再起動してください。
  コマンド ライン エイリアスが追加されました: "areka"
  インストールが完了しました
  ```

- 判定: ハッシュの検証が通った行（`インストーラーハッシュが正常に検証されました`）が 1 行・インストール完了の行（`インストールが完了しました`）が 1 行・終了コード 0。**合格**。
- 出力の取り方: PowerShell のパイプを通さず、`Start-Process -NoNewWindow -Wait -PassThru -RedirectStandardOutput … -RedirectStandardError …` でファイルへ向け、UTF-8 として読んだ（端末の文字コードは変えていない）。

### 2. 新しい端末と同じ PATH を組み直した（要件 3.3）

- 読み方: 機械の側はレジストリ `HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment` の `Path`、利用者の側は `HKCU:\Environment` の `Path` を、環境変数を開かない形で読み、それぞれ `%名前%` を開いてから「機械の側;利用者の側」の順に `;` でつないだ。これが、新しく開いた端末が受け取る PATH と同じ文字列。
- 組み直した PATH は **31 項目**（機械の側 18＋利用者の側 13）。areka の項目は **1 件**で、31 番目（末尾）:

  ```text
   1〜18: 機械の側の 18 項目（入れる前と 1 字も違わない。areka の字を含む項目は 0 件）
  19〜30: 利用者の側の、入れる前から在った 12 項目（並びも字面も入れる前と同じ）
  31: %LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource
  ```

- 利用者の側の登録の変わり方: 入れる前 12 項目 → 入れた後 13 項目。足されたのは上の 1 件だけで、消えた項目・書き替わった項目は 0（前後を項目ごとに見比べた）。足された項目は、登録に変数を開いた形（`C:\Users\` から始まる字面）で書かれている。値の種類は `ExpandString` のまま。
- 機械の側の登録は、入れる前と文字列として同じ（18 項目・areka の項目は 0 件）。
- `%LOCALAPPDATA%\Microsoft\WinGet\Links` は、組み直した PATH に入っていない（入れる前と同じ）。

### 3. `areka` の 1 語で起動した（要件 3.2）

- 起こし方: 上の文字列を PATH にした新しい `pwsh -NoProfile -NonInteractive` のプロセスを起こし、その中で次を行った（スクリプトは `target\winget-check\2.2-launch.ps1` と `2.2-inner.ps1`）。

  ```powershell
  # 新しいプロセスの中（PATH は組み直した 31 項目）
  Get-ChildItem Env: | Where-Object { $_.Name -like 'AREKA_*' -or $_.Name -like 'WINTF_*' } | ForEach-Object { Remove-Item "Env:$($_.Name)" }
  $env:AREKA_APP_SMOKE_EXIT_MS = '10000'; $env:AREKA_NO_ALERT = '1'; $env:RUST_LOG = 'info'; $env:NO_COLOR = '1'
  (Get-Command areka).Source; @(Get-Command areka -All).Count
  $p = Start-Process -FilePath areka -WorkingDirectory target\winget-check\run -NoNewWindow -PassThru `
      -RedirectStandardOutput target\winget-check\logs\2.2-run.stdout.log `
      -RedirectStandardError  target\winget-check\logs\2.2-run.stderr.log
  $p.WaitForExit(90000)   # 自分から終わるのを最長 90 秒待つ
  ```

- 外した環境変数は 1 つ（`AREKA_IMPL_WATCH_HOME`。作業中のセッションから引き継いでいた物）。`WINTF_*` は 0。
- `areka` はフルパスでなく 1 語で渡した。起きたプロセスの実体は `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource\areka.exe`（起こした直後にプロセスから読んだ）。
- 始めた時刻 13:02:47.645・終わった時刻 13:02:59.279（11.6 秒）・**自分から終わった**（止めていない）・**終了コード 0**。起こしたのはこの 1 回だけ。
- 新しい `pwsh` は、起動のときに自分の置き場を PATH の先頭へ 1 件足す（`C:\Program Files\WindowsApps\Microsoft.PowerShell_7.6.6.0_x64__8wekyb3d8bbwe`。中のプロセスから見た PATH は 32 項目）。areka の物ではなく、`Get-Command areka -All` は 1 件のまま。
- 起動の記録: 標準出力 `2.2-run.stdout.log` は 102 行（23712 バイト）。標準エラー `2.2-run.stderr.log` は 1 行（`[helper] SHIORI 初期化の入口: loadu`）。

### 4. 判定（設計の段 3 の表の 6 項目）

| 項目 | 合格の条件 | 見た値 | 判定 |
|---|---|---|---|
| `本物のゴースト窓を開きました` の行 | 1 件以上 | **1 件**（`… areka::ghost_session: 本物のゴースト窓を開きました（placement シーム・スコープごとにキャラ窓＋バルーン窓） scopes=[0, 1]`） | 合格 |
| `root_resolved` の行の `root=` | winget の入れ先のフォルダ | `root=%LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource source=ExeDir`（1 行） | 合格 |
| `(Get-Command areka).Source` | 入れ先のフォルダの中の `areka.exe` | `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource\areka.exe`（種類は `Application`・`Get-Command areka -All` は 1 件） | 合格 |
| その `areka.exe` の `LinkType` | 空（リンクではない）。`%LOCALAPPDATA%\Microsoft\WinGet\Links\areka.exe` は無い | `LinkType` は空・`Target` も空・属性は `Archive`・9375744 バイトの実ファイル。`Links\areka.exe` は**無い**（`Links\` の下で名前が `areka` で始まる物は 0 件） | 合格 |
| 利用者の PATH | 入れ先のフォルダが 1 件足されている | 利用者の側の登録 13 項目のうち、入れ先のフォルダと同じ項目は **1 件**（入れる前は 12 項目・0 件）。機械の側の登録は入れる前と同じ | 合格 |
| 終了コード | 0（自分から終わった） | **0**（11.6 秒で自分から終わった） | 合格 |

- 合格 6・不合格 0。
- 判定に使う 3 つ目の行 `last_ghost_not_found` は 0 件。
- 雛形は直していない（やり直しは無い）。

### 5. あわせて読んだこと（この後のタスクが使う。判定には入れない）

`winget list` の areka の行（`winget list --id Areka --disable-interactivity` と `winget list --name areka --disable-interactivity`。どちらも終了コード 0・標準エラー 0 バイトで、同じ 1 行を返した）:

```text
名前             ID                                               バージョン
----------------------------------------------------------------------------
areka (portable) ARP\User\X64\Areka.Areka.Portable__DefaultSource 0.0.2
```

- 名前は `areka (portable)`・ID は `ARP\User\X64\Areka.Areka.Portable__DefaultSource`・版は `0.0.2`。ソースの列は出なかった（列は 名前・ID・バージョン の 3 つ）。

入れ先のフォルダの直下の一覧（入れた直後 13:02 と、1 回目の起動の後 13:03）:

| 名前 | 入れた直後 | 1 回目の起動の後 |
|---|---|---|
| `Areka.Areka.Portable__DefaultSource.db` | 16384 バイト | 同じ |
| `areka.exe` | 9375744 バイト | 同じ |
| `balloon\` | 50 ファイル・106101 バイト | 同じ |
| `BUILD-INFO.txt` | 151 バイト | 同じ |
| `ghost\` | 93 ファイル・6908857 バイト | **143 ファイル・7331639 バイト**（50 ファイル増えた） |
| `LICENSE-MIT` | 1085 バイト | 同じ |
| `profile\` | 無い | **在る**（1 ファイル・55 バイト） |
| `README.txt` | 20425 バイト | 同じ |
| `shiori-host32-helper.exe` | 221696 バイト | 同じ |
| `THIRD-PARTY-NOTICES.md` | 138674 バイト | 同じ |
| ファイルの数（下の階層も数えた合計） | 150 | 201 |

- 直下の `Areka.Areka.Portable__DefaultSource.db` は zip の中の物ではなく、winget が置いた物。
- 1 回目の起動が作った物は 51 ファイルで、2 か所に在る。どちらも入れ先のフォルダの中。
  - `profile\areka\sylphya.toml`（55 バイト）＝areka の記憶。
  - `ghost\emo2\ghost\master\profile\` の下の 50 ファイル＝同梱のゴーストの記憶（`areka\sylphya.toml` 70 バイト・`pasta\save\save.json` 2 バイト・`pasta\logs\pasta.log`・`pasta\cache\lua\…` 17 ファイル・`pasta\pasta_scripts\…` 30 ファイル）。
- 起動の後、入れ先のフォルダから動いているプロセスは 0（読んだだけ）。作業フォルダ `target\winget-check\run\` には何も出来ていない。

起動の記録の `WARN`・`ERROR` の行: **`WARN` 4 行・`ERROR` 0 行**。違う文は 3 つ。

- `areka_emo_text::actor::attach: 折返し基準が描画範囲の外に解決された——実効の折返し位置は描画範囲の辺になる（バルーン定義側の粗さ） balloon="emo2-kakukaku" axis="x" wrap_threshold=254.0 inline_limit=240.0`（1 行）
- `areka_emo_atlas: bake: element が全透明（α=0）でトリム後 0 寸です（ゴースト制作者ミスの可能性） set=0 rel_path="purple/a/null.png" original_w=382 original_h=547`（2 行）
- `kanade: 強制終了指示——終了系列（Forced）へ直行 event="force_quit" reason="user"`（1 行。決めた時間が過ぎて自分から終わるときの行）

同じ記録から読めたこと:

- 起動したゴーストは `ghost_resolved` の行で `route=Only dir=…\ghost\emo2`、バルーンは `balloon_resolved` の行で `route=Companion dir=…\balloon\emo2-kakukaku`（どちらも入れ先のフォルダの中）。
- MCP の口は `http://127.0.0.1:9801/api/mcp/v1` で待ち受けを始め、終わるときに閉じた。
- 終わりの行は `ghost shutdown sequence completed` と `きれいに終わったので起動中の印を消しました event="session_mark_cleared"`。

## 上げ直しの実測

### 作る状態（タスク 3.1・要件 4.1）

結果: **作れた**（2026-10-10・3 回目の起動で）。作り方は設計の見込みと違う。入れた areka（0.0.2）の MCP の `sakurascript` は、台本を受け取らずに `NG:not implemented yet` と答えるので、台本では入れることも・切り替えることも・終えることもできなかった（1 回目）。設計の段 4 の決まりどおり右クリックメニューでの同じ操作に切り替え、3 回目に、検体のゴースト 1 体とバルーン 1 つを入れ、クローディアへ切り替え、「終了」で終えた。シェルの記憶だけは 0.0.2 では作れず、**測れなかった**（理由は下の「作った物の一覧」）。入れ先の写しは `target\winget-check\state-snapshot\` に取った。**この後、上げ直しの実測（タスク 3.2）まで areka を起動しない。** 時刻はどれも 2026-10-10（+09:00）。

右クリックメニューを操作したのは開発者の手ではなく AI（この作業を進めているセッション）。開発者が 13:21 ごろに `areka.exe` の画面の操作を AI に許し、AI が画面の操作でメニューを開いて選んだ（操作した側の控え。起動の記録には、メニューが選ばれた行は出るが、誰の手かは出ない）。開発者の手は要らなかった。

#### 1 回目: MCP の口から台本を送った（13:12〜13:15）

起こし方（タスク 2.2 と同じ。違いは、決めた時間で終わる指定を外したことと、MCP の口の番号を決めたこと）:

- PATH を登録（機械の側;利用者の側）から組み直した新しい `pwsh -NoProfile -NonInteractive` のプロセスの中で、`AREKA_*`・`WINTF_*` を外し（外れたのは `AREKA_IMPL_WATCH_HOME` の 1 つ）、`AREKA_NO_ALERT=1`・`RUST_LOG=info`・`NO_COLOR=1`・`AREKA_MCP_PORT=9871` を置いた。`AREKA_APP_SMOKE_EXIT_MS` は置いていない。
- 番号 9871 は、起こす直前に `Get-NetTCPConnection -State Listen` で待ち受けが 0 件であることを見て決めた（9800〜9899 の待ち受けも 0 件）。
- `areka` の 1 語を渡した（`Get-Command areka -All` は 1 件）。起きたプロセスの実体は `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource\areka.exe`。作業フォルダは `target\winget-check\run\`。スクリプトは `target\winget-check\3.1-launch.ps1` と `3.1-inner.ps1`。
- 始めた時刻 13:12:09.202。起動の記録（`target\winget-check\logs\3.1-run1.stdout.log`）から: `root_resolved` の `root=` は入れ先のフォルダ（`source=ExeDir`）／`ghost_resolved` は `route=Memory dir=…\ghost\emo2`／`balloon_resolved` は `route=Memory dir=…\balloon\emo2-kakukaku`／`MCP: 待受を始めた url=http://127.0.0.1:9871/api/mcp/v1`／`本物のゴースト窓を開きました … scopes=[0, 1]` は 1 件／`last_ghost_not_found` は 0 件。

MCP の口へ送った物と答え（送り方は `tools/call` を 1 本ずつの HTTP。送った文と答えの全文は `target\winget-check\logs\3.1-mcp.log`。スクリプトは `target\winget-check\3.1-mcp.ps1`）:

| 時刻 | 送った物 | 答え | 確かめたこと |
|---|---|---|---|
| 13:12:14 | `get_status` | `talking,balloon(0=0)` | 起動の台詞を話している |
| 13:12:14 | `get_active_ghost_list` | `えも？？` | 今のゴーストは同梱の えも？？ |
| 13:12:29 | `get_status` | `balloon(0=0/1=0)` | 話し終わった（`talking` が無い） |
| 13:12:29 | `check_script`（下と同じ台本） | JSON-RPC のエラー `-32602`・`tool not found` | 0.0.2 にはこのツールが無い（0.0.2 より後に足された物。不具合ではない） |
| 13:12:29 | `sakurascript`（下の台本） | **`NG:not implemented yet`**（`isError: true`） | 台本は受け取られなかった |
| 13:13:13 | `get_status`・`get_active_ghost_list` | `balloon(0=0/1=0)`・`えも？？` | 何も変わっていない |

送った台本（そのまま。`<ワークツリー>` はこのワークツリーの根の絶対パス。検体は `vendors\sample_ghost\claudia.nar` の写しで、SHA256 は元と同じ `93A34C50…D23A790A`）:

```text
\![execute,install,path,<ワークツリー>\target\winget-check\nar\claudia.nar]\e
```

- 送ってから 40 秒待っても、入れ先に `ghost\claudia\` はできなかった。起動の記録にも、インストールの行は 1 行も無い。
- 理由: 0.0.2 の MCP の `sakurascript` は、中身がまだ無い仮の受け口（タグ `v0.0.2` の `crates/areka/src/mcp/sakurascript.rs` の `handle` が、何もせず `NG:not implemented yet` を返す 1 文。この枝の同じファイルも同じ）。同じ口の `tools/list` は 10 本を返すが（答えの全文は `target\winget-check\logs\3.1-tools-list.json`）、`raise_event` と `reload` も同じ仮の受け口（こちらはタグ `v0.0.2` のソースの読みで、実機では呼んでいない）。中身を入れる仕事は、起票済みでまだ着手していない spec `areka-P0-mcp-kanade-tools` の brief に書かれている。
- だから 2 本目の台本（`emo2-kakukaku-wplimit.nar` を入れる）・切り替えの台本（`\![change,ghost,…]`）・終える台本（`\-`）は**送っていない**（同じ答えになるだけなので）。

終え方（台本の `\-` が使えないので、OS の閉じる要求で終えた）:

- 自分が起こしたプロセス（プロセスの番号・実体のパス・始めた時刻が、起こしたときの控えと合うことを確かめた 1 つ）のキャラクター窓 1 枚へ、OS の閉じる要求（`WM_CLOSE`＝Alt＋F4 と同じ物）を 1 回送った（13:15:04.677）。プロセスを止める操作はしていない。
- areka はこれを、右クリックメニューの「終了」と同じ終わり方として扱った。起動の記録の行: `[os_close] OS の閉鎖要求をメニューの「終了」と同じ終了要求として kanade へ送る event="os_close_request" scope=0 kind="ghost"` → `OnClose GET を発行し握手を開始` → 終わりの台詞 → `ghost shutdown sequence completed` → `きれいに終わったので起動中の印を消しました event="session_mark_cleared"`。
- 終わった時刻 13:15:14.219（起こしてから 185.0 秒）・**自分から終わった**・**終了コード 0**。終わった後、入れ先のフォルダから動いているプロセスは 0・9871 の待ち受けは 0。
- 起動の記録は 645 行（160426 バイト）。`ERROR` は 0 行・`WARN` は 5 行（タスク 2.2 と同じ 2 つの文が 3 行＝絵の `null.png` が 2 行と折り返しの基準が 1 行。残り 2 行は、MCP へ無いツール `check_script` を聞いた答えと、試しに引数なしで聞いた `get_property` の答え＝どちらも聞き方の側の物）。標準エラーは 1 行（`[helper] SHIORI 初期化の入口: loadu`）。

1 回目の後の入れ先（隠しファイルも数えた。一覧は `target\winget-check\logs\3.1-installdir-run1-after-end.txt`・相対パスと大きさ）:

| 物（設計の段 4 の表） | 置き場（入れ先のフォルダから見て） | 有無 |
|---|---|---|
| 後から入れたゴースト | `ghost\claudia\` | 無い |
| 後から入れたバルーン | `balloon\emo2-kakukaku-wplimit\` | 無い |
| クローディアと一緒に入るバルーン 2 つ | `balloon\claudia\`・`balloon\claudia_vertical\` | 無い・無い |
| areka の記憶 | `profile\areka\` | 在る（1 ファイル・55 バイト。中の `[last]` は `ghost = "emo2"`・`running = ""`） |
| ゴーストの記憶（えも？？） | `ghost\emo2\ghost\master\profile\areka\` | 在る（1 ファイル・90 バイト） |
| ゴーストの記憶（クローディア） | `ghost\claudia\ghost\master\profile\areka\` | 無い（ゴーストがまだ無い） |
| シェルの記憶（えも？？） | `ghost\emo2\shell\master\profile\areka\` | 無い |
| シェルの記憶（クローディア） | `ghost\claudia\shell\<シェル>\profile\areka\` | 無い（ゴーストがまだ無い） |

- ファイルの数は 201（起こす前と同じ）・合計 17214191 バイト。起こす前（`3.1-installdir-before-launch.txt`・201 ファイル・17211954 バイト）と比べて、大きさが変わったのは 2 ファイルだけ（`ghost\emo2\ghost\master\profile\areka\sylphya.toml` が 70 → 90 バイト・`ghost\emo2\ghost\master\profile\pasta\logs\pasta.log` が 2788 → 5005 バイト）。増えたファイル・消えたファイルは 0。
- この時点では、状態は 1 つも作れていない（下の 2 回目・3 回目へ続く）。

#### 2 回目: 右クリックメニューの「インストール…」が、ファイルを選ぶ画面を出さなかった（13:19〜13:22）

- 起こし方は 1 回目と同じ（`3.1-launch.ps1 -Port 9871 -Tag run2`。`AREKA_NO_ALERT=1` を置いたまま）。始めた時刻 13:19:08.106。今のゴーストは えも？？（`ghost_resolved` は `route=Memory dir=…\ghost\emo2`）・`本物のゴースト窓を開きました` は 1 件。MCP の口へは何も送っていない。
- 13:22:09.133 に右クリックメニューの「インストール…」が選ばれた（`[menu] selected event="menu_selected" scope=0 frame=Install id=6`）。直後の行は `WARN … [install] 告知が抑止されているので、ファイルを選ぶ画面を出さずに取り消しと同じに扱います event="install_pick_suppressed"`。ファイルを選ぶ画面は出ず、何も入らなかった。
- これは不具合ではない。`AREKA_NO_ALERT` は、自動のテストが画面の前で止まらないように、知らせの画面を抑える環境変数（タグ `v0.0.2` の `crates/areka/src/alert.rs` の説明）で、ファイルを選ぶ画面も同じ扱いにし、抑えたことを記録に残す作りになっている。利用者の普段の起動では置かれない。確かめの側が、起こし方からこの変数を外せば済む（3 回目）。
- 13:22:48.691 に「終了」が選ばれ（`frame=Close id=8`）、`ghost shutdown sequence completed` → `session_mark_cleared`（13:22:51.176）。終わった時刻 13:22:51.399（223.3 秒）・自分から終わった・**終了コード 0**。
- 起動の記録 `3.1-run2.stdout.log` は 578 行（144450 バイト）。`ERROR` は 0 行・`WARN` は 4 行（タスク 2.2 と同じ文の 3 行と、上の `install_pick_suppressed` の 1 行）。`last_ghost_not_found` は 0 件。

#### 3 回目: 右クリックメニューで状態を作った（13:23〜13:26）

起こし方: 1 回目との違いは、`AREKA_NO_ALERT` を**置かない**ことだけ（`target\winget-check\3.1-launch-alert.ps1 -Port 9871 -Tag run3`。中で呼ぶ `3.1-inner-alert.ps1` は、`3.1-inner.ps1` から `$env:AREKA_NO_ALERT = '1'` の 1 行を除いただけの物）。置いた環境変数は `AREKA_MCP_PORT=9871`・`RUST_LOG=info`・`NO_COLOR=1`。外したのは `AREKA_IMPL_WATCH_HOME` の 1 つ。`areka` の 1 語で起こし、実体は入れ先のフォルダの `areka.exe`。MCP の口へは何も送っていない（起動の記録に MCP の受け答えの行は 0 行）。

始めた時刻 13:23:26.253。`root_resolved` の `root=` は入れ先のフォルダ・`ghost_resolved` は `route=Memory dir=…\ghost\emo2`・`balloon_resolved` は `route=Memory dir=…\balloon\emo2-kakukaku`・1 件目の `本物のゴースト窓を開きました … scopes=[0, 1]` は 13:23:27.603。

行った操作と、済んだことを確かめた行（起動の記録 `target\winget-check\logs\3.1-run3.stdout.log`。`<入れ先>` は入れ先のフォルダ）:

| 時刻 | 操作（右クリックメニュー） | 済んだことを確かめた行 |
|---|---|---|
| 13:23:48 | 「インストール…」（`menu_selected … frame=Install id=6`）→ ファイルを選ぶ画面で `<ワークツリー>\target\winget-check\nar\claudia.nar` を開く | 13:24:04.929 `[install] 依頼を受けました event="install_order_queued" origin=Menu count=1` → `install_begin archive=…\nar\claudia.nar origin=Menu` → `install_accept … verdict="accepted"` → 13:24:05.274 `[install] 入れました event="install_done" … kind=Ghost places=[<入れ先>\ghost\claudia, <入れ先>\balloon\claudia, <入れ先>\balloon\claudia_vertical]` → `last_installed_recorded folder=claudia` → `install_event id="OnInstallComplete" raised=Script` |
| 13:24:28 | 「インストール…」（`frame=Install id=9`）→ `<ワークツリー>\target\winget-check\nar\emo2-kakukaku-wplimit.nar` を開く | 13:24:40.483 `install_order_queued origin=Menu count=1` → 13:24:40.545 `install_done … kind=Balloon places=[<入れ先>\balloon\emo2-kakukaku-wplimit]` → `last_installed_balloon_recorded folder=emo2-kakukaku-wplimit` → `[install] 今のゴーストの「最後に使ったバルーン」を入れたバルーンへ書き換えました event="install_balloon_remembered" folder=emo2-kakukaku-wplimit` → `install_event id="OnInstallComplete" raised=Script` |
| 13:25:03 | 「ゴースト」→「悪役令嬢クローディア」（`frame=Ghost id=1`） | `切替の要求を kanade へ送った event="ghost_switch_requested" from=Some("えも？？") to=悪役令嬢クローディア … origin="manual"` → 13:25:08.510 `change_accepted` → 13:25:11.491 `switch_drop_recorded last_ghost="emo2" mark="悪役令嬢クローディア"` → 13:25:12.156 `ghost_switch_booted ghost=Some("claudia") attempt=Target` → 13:25:12.337 2 件目の `本物のゴースト窓を開きました` → 13:25:12.899 `last_used_recorded ghost="claudia" balloon="StayseeBalloon" shell="master"` → 13:25:12.904 `ghost_switch_done ghost=Some("claudia")` |
| 13:26:22 | クローディアのキャラクターを左へドラッグ（シェルの記憶を作ろうとして足した操作） | `[DragStartEvent] … x=2530 y=1286` → 13:26:22.814 `[DragEndEvent] … x=2447 y=1286 cancelled=false` → `char DragEnd 保存 scope=0 char_x=2131 char_y=704 saved_x=2131 saved_y=704`（位置はクローディアのゴーストの記憶の `[window.0]` に書かれた。シェルの記憶はできなかった） |
| 13:26:47 | 「終了」（`frame=Close id=12`） | `OnClose GET を発行し握手を開始 event="close_handshake_begin" reason="user"` → 13:26:52.922 `app_exit origin=KanadeStopped(Quit) closed=4` → 13:26:53.086 `ghost shutdown sequence completed` → 13:26:53.112 `きれいに終わったので起動中の印を消しました event="session_mark_cleared"` |

- 画面で読んだ台詞（操作した側の控え。起動の記録には台詞の本文は出ない）: クローディアを入れた後に えも？？ が「来たで！ うちらの出番、減らへんやろな‥‥？」「新しいお友達だね。」／切り替えの後にクローディアの挨拶／ドラッグの後に「ふん、この場所がよろしいの？ まあ、眺めは悪くありませんわ。」。
- **きれいに終えたときの今のゴーストはクローディア**。終わった後の areka の記憶 `profile\areka\sylphya.toml` は `[last]` の `ghost = "claudia"`・`running = ""`（起動中の印は空）。
- 終わった時刻 13:26:59.258（213.0 秒）・**自分から終わった**・**終了コード 0**（`3.1-run3.result.json`）。起動の記録の最後の行は 13:26:53.138 の `MCP: 待受を閉じた` で、プロセスが消えたのはその 6.1 秒後（1 回目は 0.07 秒後・2 回目は 0.2 秒後）。この間の記録は無く、理由は調べていない。終了コードは 0 で、起動中の印も消えている。
- 起動の記録は 595 行（145355 バイト）。`ERROR` は 0 行・`last_ghost_not_found` は 0 件・`WARN` は 9 行（タスク 2.2 と同じ文の 3 行と、切り替えた直後の `balloon: 面がデフォルト定義側（本体側）の系列へ縮退した scope=1 surface_id=2 …`・`… surface_id=3 …` が 3 行ずつ）。標準エラーは 2 行（`[helper] SHIORI 初期化の入口: loadu` が、えも？？ とクローディアの分）。
- 切り替えた先のクローディアは、自分の `.nar` が連れてきたバルーン（`claudia`）でなく `StayseeBalloon` で立った（上の `last_used_recorded` の行。クローディアのゴーストの記憶にも `balloon = "StayseeBalloon"` と書かれた）。これは 0.0.2 の動き。タグ `v0.0.2` より後のコミット `8599cd91`（完了 `areka-P0-ghost-standard-balloon`＝バルーンをゴーストの descript の指定と同梱の最初の 1 個で決める）がこの枝には入っている。上げ直し・外し方の実測が見るのはフォルダの有無なので、結果には響かない。

#### 作った物の一覧（3 回目の後・写しを取る前）

一覧は `target\winget-check\logs\3.1-installdir-run3-after-end.txt`（相対パスと大きさ・隠しファイルも含む・スクリプトは `3.1-list.ps1`）。取ったとき、入れ先のフォルダから動いている `areka.exe`・`shiori-host32-helper.exe` は 0・9871 の待ち受けは 0。

| 物（設計の段 4 の表） | 置き場（入れ先のフォルダから見て） | 有無 |
|---|---|---|
| 後から入れたゴースト | `ghost\claudia\` | **在る**（69 ファイル・4456155 バイト。うち 1 つはゴーストの記憶） |
| 後から入れたバルーン | `balloon\emo2-kakukaku-wplimit\` | **在る**（20 ファイル・31898 バイト） |
| クローディアと一緒に入ったバルーン 2 つ | `balloon\claudia\`・`balloon\claudia_vertical\` | **在る**（37 ファイル・83386 バイト）・**在る**（31 ファイル・54749 バイト） |
| areka の記憶 | `profile\areka\` | **在る**（1 ファイル `sylphya.toml`・58 バイト。`[last]` の `ghost = "claudia"`・`running = ""`） |
| ゴーストの記憶（えも？？） | `ghost\emo2\ghost\master\profile\areka\` | **在る**（1 ファイル `sylphya.toml`・98 バイト。`[boot]` の `count`・`[last]` の `balloon = "emo2-kakukaku-wplimit"`・`shell = "master"`） |
| ゴーストの記憶（クローディア） | `ghost\claudia\ghost\master\profile\areka\` | **在る**（1 ファイル `sylphya.toml`・124 バイト。`[boot]` の `count`・`[last]` の `balloon = "StayseeBalloon"`・`shell = "master"`・`[window.0]` の `x = "2131"`・`y = "704"`） |
| シェルの記憶（えも？？） | `ghost\emo2\shell\master\profile\areka\` | 無い＝**測れなかった** |
| シェルの記憶（クローディア） | `ghost\claudia\shell\master\profile\areka\` | 無い＝**測れなかった** |

- **シェルの記憶が測れなかった理由**: 0.0.2 には、シェルの記憶へ書く操作が 1 つも無い。タグ `v0.0.2` のソース（テストを除く）で記憶へ書く口 `persist_put` を呼んでいる所は 8 か所（`crates/areka/src/boot_resolve.rs` の 4 か所・`emo2_boot/ghost_switch.rs`・`install/desk.rs`・`placement/persist.rs`・`crates/areka-ghost/src/prop_sink.rs`）で、書き先はどれも areka の記憶（`PersistScope::App`）かゴーストの記憶（`PersistScope::Ghost`）。シェルの記憶（`PersistScope::Shell`）は、起動のときに読む所だけに出てくる。`crates/areka-sylphya/src/persist/mod.rs` の説明も「本番の鍵はすべてゴーストの記憶に載る」と書いている。設計が足す操作に挙げた「キャラクターを動かす」は 3 回目で行い、位置はゴーストの記憶（`[window.0]`）に書かれた。「シェルを切り替える」も、書く口は上の 8 か所のどれかなので、書き先は areka の記憶かゴーストの記憶になる（今のシェルの名前は、ゴーストの記憶の `[last]` の `shell` に書かれている）。だから、この種類は作らずに「測れなかった」とする（ファイルを手で置くことはしていない）。
- シェルのフォルダそのもの（`ghost\emo2\shell\master\`・`ghost\claudia\shell\master\`）は `ghost\` の下に在る。上げ直しと外し方で `ghost\` に起きることは、そのままシェルのフォルダにも起きるので、後の実測ではこの 2 つのフォルダの有無を見る。
- えも？？ のゴーストのフォルダ `ghost\emo2\ghost\master\profile\` には、areka の記憶のほかに、ゴースト自身（pasta）が作った物が 49 ファイル在る（`pasta\save\`・`pasta\logs\`・`pasta\cache\`・`pasta\pasta_scripts\`。合わせて 50 ファイル・431218 バイト）。
- 入れ先の直下に、空のフォルダ `.nar-work\` ができている（0 ファイル）。`.nar` を入れるときの作業用フォルダで、説明書（`README.txt`）に載っている物。
- 入れ先のファイルの数は **358**・合計 **21846581 バイト**・下の階層のフォルダは 58。起こす前（201 ファイル）から増えたのは 157 ファイル（`ghost\claudia\` 69・`balloon\claudia\` 37・`balloon\claudia_vertical\` 31・`balloon\emo2-kakukaku-wplimit\` 20）で、消えたファイルは 0。直下の内わけ: `balloon\` 138 ファイル・`ghost\` 212 ファイル・`profile\` 1 ファイル・直下のファイル 7 つ（うち 1 つは winget の控え `Areka.Areka.Portable__DefaultSource.db`＝隠しファイル）。
- 状態は右クリックメニューで作ったので、台本（MCP）の入口は通っていない（「既知の制限」に書いた）。

#### 写し

- 置き場: `target\winget-check\state-snapshot\`（取る前は無かった）。取った時刻 13:29:53〜13:29:54。
- コマンド: `robocopy <入れ先> target\winget-check\state-snapshot /E /COPY:DAT /DCOPY:DAT /R:1 /W:1`。終了コード 1（すべて写した、の意味。8 未満は成功）。robocopy の数えは フォルダ 59（根を含む）・ファイル 358・失敗 0・飛ばした物 0（`target\winget-check\logs\3.1-snapshot-robocopy.log`）。
- 見比べ: 写しの一覧（`3.1-installdir-snapshot.txt`）は **358 ファイル・21846581 バイト**で、入れ先の一覧と 1 行も違わない（一覧のファイルの SHA256 が同じ `21CCE03A…0EDD5A05`）。写した後に取り直した入れ先の一覧（`3.1-installdir-run3-before-snapshot-check.txt`）も同じ。358 ファイルすべてで、入れ先と写しの SHA256 が同じ（違い 0）。下の階層のフォルダは 58 と 58。空の `.nar-work\` も写しに在る。winget の控えは、写しでも隠しファイルのまま。
- 抜き出して書く 3 つ: `areka.exe` は `CC800980…659D237A`・`profile\areka\sylphya.toml` は `3E16E14A…325EDDF6`・`ghost\claudia\ghost\master\profile\areka\sylphya.toml` は `80ED6700…ADE49528`（どれも入れ先と写しで同じ）。
- 写しを取った後、入れ先のフォルダから動いているプロセスは 0・9871 の待ち受けは 0。**上げ直しの実測（タスク 3.2）まで、areka を起動しない。**

### 上げ直し（タスク 3.2・要件 4.2・4.4）

結果: **測れた。消えた物は 6**（後から入れたゴースト 1・後から入れたバルーン 3・ゴーストの記憶 2）。areka の記憶は残った。同梱のファイルは置き換わった。上げ直しの後の起動は、前のゴースト（クローディア）ではなく**別のゴースト（えも？？）で立った**。使ったコマンドは `winget upgrade --manifest` で、代わりの手（`winget install --manifest`）は要らなかった。時刻はどれも 2026-10-10（+09:00）。`<入れ先>` は `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource`。

入れ先は、起動した後の状態のまま**置いてある**（写しから戻していない・外していない）。外し方の実測（タスク 3.3）がここから続ける。

#### 1. 上げ直しの前に確かめたこと（読むだけ・13:40:32）

- `<入れ先>` から動いている `areka.exe`・`shiori-host32-helper.exe` は 0（この名前のプロセス自体が 0）。
- `<入れ先>` は 358 ファイル・21846581 バイト・下の階層のフォルダ 58 で、写し `target\winget-check\state-snapshot\` と同じ数。
- 見比べの元になる値: Release の `areka-0.0.2-x64.zip` の中の `ghost/emo2/readme.txt`（3105 バイト）の SHA256 は `E1B864F9CE41519F1B2DDEF00AD986397BE51D7558A54F7213266B8BC0483B40`（zip を開かずに、中の 1 件をメモリへ読んで計算した）。入れ先の `ghost\emo2\readme.txt` も同じ値だった。zip の中に `winget-check-marker.txt` は無い。zip の最上位のファイルは 6 つ（`areka.exe`・`BUILD-INFO.txt`・`LICENSE-MIT`・`README.txt`・`shiori-host32-helper.exe`・`THIRD-PARTY-NOTICES.md`）。

#### 2. 目印（13:40:48）

入れ先の中で手を入れたのは、この 2 つだけ。zip の最上位の 6 ファイルには触っていない。

| 目印 | 場所（入れ先のフォルダから見て） | 付けた後 |
|---|---|---|
| 末尾へ 1 行足した（73 バイト・ASCII の `winget-check marker line (areka-P0-winget-manifest-submission task 3.2)` と改行） | `ghost\emo2\readme.txt` | 3178 バイト・SHA256 `266B860A544CBB0D77B8B9247D28A4BEFC1DE06D26DB472C9555FAD53088907D`（付ける前は 3105 バイト・`E1B864F9…C0483B40`） |
| 新しく置いたファイル（73 バイト） | `ghost\emo2\winget-check-marker.txt` | SHA256 `61C7E035D2A5DF0B2660D279E5ED97C09E1C8496A34CA9F906873EB03355221A` |

- 付けた後の入れ先は 359 ファイル・21846727 バイト（1 ファイル・146 バイト増えた）。

#### 3. 確かめ専用のマニフェスト

- 置き場: `target\winget-check\upgrade-0.0.2.1\`（4 ファイル。`dist\winget\0.0.2\` の 4 ファイルの写し）。
- 変えた欄は `PackageVersion` の 1 つだけ（`0.0.2` → `0.0.2.1`。4 ファイルに 1 行ずつで、変わった行は合わせて 4 行）。`InstallerUrl`・`InstallerSha256` と、ほかの欄は 0.0.2 のまま（元との見比べで、違う行はこの 4 行だけ）。
- `winget validate --manifest target\winget-check\upgrade-0.0.2.1 --disable-interactivity`: 13:41:04・**終了コード 0**・標準エラー 0 バイト。出た文は `マニフェストの検証は成功しました。` の 1 行。
- `git status --porcelain` は 0 行（`git check-ignore -v` は `.gitignore:1:target` を返した）。リポジトリは追跡していない。

#### 4. 上げ直しの前の一覧と、打つ前の確かめ

- 一覧（相対パス・大きさ・SHA256・書いた時刻・作った時刻。隠しファイルも含む）: `target\winget-check\logs\3.2-before.tsv`（13:41:06・**359 ファイル・21846727 バイト**・隠しファイル 1）。フォルダの一覧（相対パス・NTFS がフォルダに振る番号・作った時刻）: `3.2-before.dirs.tsv`（下の階層のフォルダ 58）。スクリプトは `target\winget-check\3.2-list.ps1`。
- `<入れ先>` から動いている `areka.exe`・`shiori-host32-helper.exe` は、13:41:04・13:41:13・13:42:14・13:42:18 の 4 回とも 0（最後の 2 回は、打つスクリプトが 4 秒あけて見た分）。
- 打つ前の `winget list --id Areka --disable-interactivity`: ID は `ARP\User\X64\Areka.Areka.Portable__DefaultSource`・版は `0.0.2`（終了コード 0）。
- 利用者の側の PATH の登録は 13 項目で、入れ先のフォルダは 1 件。`winget settings export` の `adminSettings.LocalManifestFiles` は `true`。打った端末は管理者の権限ではない（管理者の役割を持つかを読んで `False`）。

#### 5. 上げ直し（要件 4.2）

- コマンド（ワークツリーの根で・普段の権限で。`--accept-*`・`--force`・`--ignore-security-hash`・`--scope`・`--purge` は付けていない。スクリプトは `target\winget-check\3.2-fire.ps1`）:

  ```powershell
  winget upgrade --manifest target\winget-check\upgrade-0.0.2.1 --disable-interactivity
  ```

- 始めた時刻 13:42:18.834・終わった時刻 13:42:28.967（10.1 秒）・**終了コード 0**・標準エラーは 0 バイト。問い（同意・管理者への切り替え）は 1 つも出なかった。**止まらなかったので、代わりの手 `winget install --manifest` は打っていない。**
- winget が出した文（標準出力の全部＝10 行・741 バイト。省いた行・まとめた行は無い。進み具合の描き直しの行は、ファイルへ向けた出力には 1 行も出なかった。UTF-8 として読んだ）:

  ```text
  見つかりました areka (portable) [Areka.Areka.Portable] バージョン 0.0.2.1
  このアプリケーションは所有者からライセンス供与されます。
  Microsoft はサードパーティのパッケージに対して責任を負わず、ライセンスも付与しません。
  ダウンロード中 https://github.com/ekicyou/areka/releases/download/v0.0.2/areka-0.0.2-x64.zip
  インストーラーハッシュが正常に検証されました
  アーカイブを展開しています...
  アーカイブが正常に展開されました
  パッケージのインストールを開始しています...
  コマンド ライン エイリアスが追加されました: "areka"
  インストールが完了しました
  ```

- 初めて入れたとき（上の「入れて起動する」の 11 行）との違いは、`パス環境変数が変更されました; …` の行が無いことだけ。古い版を外した・ファイルを消した、と告げる行は 1 行も無い（利用者は、この文からは物が消えたことを知れない）。
- 出力の取り方は、入れたときと同じ（PowerShell のパイプを通さず、`Start-Process -NoNewWindow -PassThru -RedirectStandardOutput … -RedirectStandardError …` でファイルへ向け、UTF-8 として読んだ）。

#### 6. 上げ直しの後の一覧（起動より先・13:42:30）と見比べ

一覧は `target\winget-check\logs\3.2-after-upgrade.tsv` と `3.2-after-upgrade.dirs.tsv`（**151 ファイル・16789175 バイト**・下の階層のフォルダ 25・隠しファイル 1）。見比べの全部は `3.2-diff-after-upgrade.txt`（スクリプトは `3.2-diff.ps1`）。この時点で areka はまだ起こしていない。

| 見比べ（359 ファイルの 1 つずつ） | 数 |
|---|---|
| 消えた | **208** |
| 増えた | 0 |
| ハッシュが同じ | 149（うち 148 は書いた時刻が新しい＝winget が zip から置き直した物。時刻も変わらなかったのは `profile\areka\sylphya.toml` の 1 つだけ） |
| ハッシュが変わった | 2（`ghost\emo2\readme.txt`＝目印の行が消えて元の値へ・`Areka.Areka.Portable__DefaultSource.db`＝winget の控え） |

直下の名前ごと（前 → 後）:

| 直下の名前 | 前 | 後 | 消えた | ハッシュが変わった |
|---|---|---|---|---|
| 直下のファイル 7 つ | 7 | 7 | 0 | 1（winget の控え） |
| `balloon\` | 138 | 50 | 88（`claudia\` 37・`claudia_vertical\` 31・`emo2-kakukaku-wplimit\` 20） | 0 |
| `ghost\` | 213 | 93 | 120（`claudia\` 69・`emo2\ghost\master\profile\` 50・目印のファイル 1） | 1（`emo2\readme.txt`） |
| `profile\` | 1 | 1 | 0 | 0 |

- フォルダ（下の階層）: 58 → 25。消えたフォルダは 33（`ghost\claudia\` とその下 14・`balloon\claudia\`・`balloon\claudia_vertical\`・`balloon\emo2-kakukaku-wplimit\`・`ghost\emo2\ghost\master\profile\` とその下 14）。残った 25 のうち 22 は、名前は同じでも NTFS の番号が変わっている＝**消してから作り直された**（`balloon\` と `ghost\` と、その下のすべて）。番号が変わらなかったのは 3 つ（`.nar-work\`・`profile\`・`profile\areka\`）。
- 見えた形: zip の中に在る直下のフォルダ（`ghost\`・`balloon\`）は、中に後から置かれた物ごと丸ごと消され、zip の中身だけが置き直された。zip の中に無い直下の物（`profile\`・`.nar-work\`）は触られなかった。
- 入れ先のフォルダ: 名前は同じ `Areka.Areka.Portable__DefaultSource`・**同じフォルダ**（NTFS の番号も作った時刻 13:01:48 も前と同じ。`Packages\` の下で名前に `Areka` を含む物はこの 1 つ）。
- `winget list`（`--id Areka` と `--name areka` の 2 回。どちらも終了コード 0・標準エラー 0 バイトで同じ 1 行）: 名前 `areka (portable)`・ID `ARP\User\X64\Areka.Areka.Portable__DefaultSource`（前と同じ）・版 **`0.0.2.1`**（前は `0.0.2`）。
- 利用者の側の PATH の登録: 文字列として前と同じ（13 項目・入れ先のフォルダは **1 件**・値の種類は `ExpandString`）。機械の側も前と同じ。`%LOCALAPPDATA%\Microsoft\WinGet\Links\` に `areka` で始まる物は 0。
- winget の控え `Areka.Areka.Portable__DefaultSource.db`: 在る・隠しファイルのまま・大きさは同じ 16384 バイト・**中身は変わった**（SHA256 `486FC085…FE16FA4D` → `61748269…EA24CA67`・書いた時刻 13:42:28）。
- `.nar-work\`: **残った**（空のまま・NTFS の番号も前と同じ）。

#### 7. 6 項目（設計の段 5 の表・要件 4.2・4.4）

判定は、上の一覧の見比べ（起動より先）で決めた。6 つ目だけは起動の記録と MCP の答えで決めた。

| 項目 | 見た物（入れ先のフォルダから見て） | 前 → 後 | 結果 |
|---|---|---|---|
| 後から入れたゴースト | `ghost\claudia\` | 69 ファイル → フォルダごと無い | **消えた** |
| 後から入れたバルーン | `balloon\emo2-kakukaku-wplimit\` | 20 ファイル → フォルダごと無い | **消えた** |
| 〃（クローディアと一緒に入った物） | `balloon\claudia\` | 37 ファイル → フォルダごと無い | **消えた** |
| 〃（同じ） | `balloon\claudia_vertical\` | 31 ファイル → フォルダごと無い | **消えた** |
| areka の記憶 | `profile\areka\` | 1 ファイル → 1 ファイル。`sylphya.toml` はハッシュも書いた時刻も前と同じ（`3E16E14A…325EDDF6`・58 バイト。中の「前に使っていたゴースト」は `claudia` のまま＝もう無いゴーストを指している） | **残った** |
| ゴーストの記憶（えも？？） | `ghost\emo2\ghost\master\profile\areka\` | 1 ファイル → フォルダごと無い（上の `ghost\emo2\ghost\master\profile\` が丸ごと消えた＝50 ファイル。ゴースト自身（pasta）の書き残し `pasta\save\save.json` ほか 49 ファイルも一緒） | **消えた** |
| ゴーストの記憶（クローディア） | `ghost\claudia\ghost\master\profile\areka\` | 1 ファイル → 無い（ゴーストのフォルダごと） | **消えた** |
| シェルの記憶（えも？？） | `ghost\emo2\shell\master\profile\areka\` | 前も後も無い | **測れなかった**（0.0.2 にはシェルの記憶へ書く操作が無く、作れなかった）。シェルのフォルダ `ghost\emo2\shell\master\` は**置き換わった**（65 ファイル → 65 ファイルでハッシュは全部同じだが、フォルダは消してから作り直された。中に後から置いた物が在れば、一緒に消える形） |
| シェルの記憶（クローディア） | `ghost\claudia\shell\master\profile\areka\` | 前も後も無い | **測れなかった**（同じ理由）。シェルのフォルダ `ghost\claudia\shell\master\` は**消えた**（16 ファイル → 無い） |
| 同梱のファイル | `ghost\emo2\readme.txt`・`ghost\emo2\winget-check-marker.txt` | `readme.txt` は 3178 バイト `266B860A…` → 3105 バイト `E1B864F9…C0483B40`（zip の中の値に戻った）。目印のファイルは無い | **置き換わった** |
| 上げ直しの後の起動 | 起動の記録と MCP の答え（下の 8） | `本物のゴースト窓を開きました` 1 件・`last_ghost_not_found` 1 件・クローディアのフォルダを指す行 0 件・`get_active_ghost_list` は `えも？？`・終了コード 0 | **別のゴーストで立った**（前はクローディア・立ったのは えも？？） |

**まとめ: 消えた物は 6**（後から入れたゴースト 1＝`ghost\claudia\`／後から入れたバルーン 3＝`balloon\emo2-kakukaku-wplimit\`・`balloon\claudia\`・`balloon\claudia_vertical\`／ゴーストの記憶 2＝えも？？ の分とクローディアの分）。残った物は 1（areka の記憶）。測れなかった物は 2（シェルの記憶 2 か所）。ファイルの数では **208 ファイルが消えた**（後から入れた 4 つのフォルダ 157＝クローディアのゴーストの記憶 1 を含む・えも？？ の `ghost\master\profile\` 50＝ゴーストの記憶 1 とゴースト自身の書き残し 49・目印のファイル 1）。

- 消えた物が在るので、設計の段 7（注意書きを足して入れ直す・起票）を通る（タスク 3.4）。

#### 8. 上げ直しの後の起動（一覧を取った後・13:43:37）

- 起こし方はタスク 2.2 と同じ（PATH を登録から組み直した 31 項目にした新しい `pwsh -NoProfile -NonInteractive` の中で、`AREKA_*`・`WINTF_*` を外し＝外れたのは `AREKA_IMPL_WATCH_HOME` の 1 つ、`AREKA_APP_SMOKE_EXIT_MS=10000`・`AREKA_NO_ALERT=1`・`RUST_LOG=info`・`NO_COLOR=1` を置き、`areka` の 1 語を渡した）。**足したのは `AREKA_MCP_PORT=9871` の 1 つだけ**（動いている間に、MCP の口から今のゴーストの名前を聞くため。番号は、起こす直前に待ち受けが 0 件であることを見て決めた。9800〜9899 の待ち受けも 0 件）。スクリプトは `target\winget-check\3.2-run.ps1`・`3.2-inner.ps1`・`3.2-mcp.ps1`。
- `Get-Command areka -All` は 1 件で、`<入れ先>\areka.exe`。起きたプロセスの実体も同じ。
- 始めた時刻 13:43:37.360・終わった時刻 13:43:47.924（10.6 秒）・**自分から終わった**（止めていない）・**終了コード 0**。起こしたのはこの 1 回だけ。最後の記録の行（13:43:47.871）からプロセスが消えるまでは 0.05 秒。終わった後、`<入れ先>` から動いているプロセスは 0・9871 の待ち受けは 0。
- 起動の記録 `target\winget-check\logs\3.2-run.stdout.log` は 126 行（29704 バイト）。標準エラーは 1 行（`[helper] SHIORI 初期化の入口: loadu`）。判定に使う行（行の頭の時刻は省き、入れ先のパスは `<入れ先>` に置き換えた。記録の中の並びのまま）:

  ```text
  INFO areka::boot_config: ベースウェアの根を決めました event="root_resolved" root=<入れ先> source=ExeDir
  WARN areka::boot_resolve: [boot_resolve] 前回のゴーストが根に見つからないので次の候補へ進みます event="last_ghost_not_found" memory="claudia" ghost_store=<入れ先>\ghost
  INFO areka::boot_config: 起動するゴーストを決めました event="ghost_resolved" route=Only dir=<入れ先>\ghost\emo2
  INFO areka::boot_config: バルーンを決めました event="balloon_resolved" route=Companion dir=<入れ先>\balloon\emo2-kakukaku
  INFO areka::boot_resolve: [boot_resolve] 最後に使ったものを記憶へ書きました（- は argv なので書いていない） event="last_used_recorded" ghost="emo2" balloon="emo2-kakukaku" shell="master"
  INFO actor{actor=emo-text}: areka::ghost_session: 本物のゴースト窓を開きました（placement シーム・スコープごとにキャラ窓＋バルーン窓） scopes=[0, 1]
  ```

- 件数: `本物のゴースト窓を開きました` **1 件**・`last_ghost_not_found` **1 件**（覚えていた `claudia` が `ghost\` に無い）・クローディアのフォルダを指す行 **0 件**（`claudia` の字が出るのは、上の `last_ghost_not_found` の 1 行だけ）・`root_resolved` 1 件。
- MCP の答え（`target\winget-check\logs\3.2-mcp.log`）: `get_active_ghost_list` は 13:43:37.830 と 13:43:42.490 の 2 回とも **`えも？？`**。`get_status` は `talking` と `talking,balloon(0=0/1=0)`。
- `ERROR` は 0 行・`WARN` は 5 行（上の `last_ghost_not_found` 1 行と、タスク 2.2 と同じ文の 4 行＝絵の `null.png` 2 行・折り返しの基準 1 行・決めた時間で終わるときの `force_quit` 1 行）。終わりの行は `ghost shutdown sequence completed` と `session_mark_cleared`。
- areka は、前のゴーストが無いことを起動の記録の `WARN` 1 行に残して、同梱の えも？？ で立った。そのときに利用者へ台詞で伝えたかどうかは、読んでいない（この起動は 10 秒で終わる形で、台詞の中身は取っていない）。
- 起動の後の入れ先（判定には使わない。一覧は `3.2-after-run.tsv`・見比べは `3.2-diff-after-run.txt`）: 201 ファイル・17211954 バイト。起動が作り直した物は 50 ファイル（`ghost\emo2\ghost\master\profile\` の下＝えも？？ のゴーストの記憶 1 とゴースト自身の書き残し 49）。areka の記憶 `profile\areka\sylphya.toml` は書き替わり、「前に使っていたゴースト」が `claudia` から `emo2` になった（58 → 55 バイト）。作り直された えも？？ のゴーストの記憶は、最後に使ったバルーンが `emo2-kakukaku`（上げ直しの前は、後から入れた `emo2-kakukaku-wplimit`）。

## 外し方の実測

（タスク 3.3・要件 4.3・4.4・4.5）

結果: **測れた。消えた物は 6**（後から入れたゴースト 1・後から入れたバルーン 3・ゴーストの記憶 2）。areka の記憶は残った。**入れ先のフォルダは残った**（中に残ったのは areka の記憶の 1 ファイルと、空のフォルダ `.nar-work\` だけ）。打ったのは `--purge` も `--preserve` も付けない `winget uninstall` で、止まらず・何も聞かれず・終了コード 0 で終わった。時刻はどれも 2026-10-10（+09:00）。`<入れ先>` は `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource`。

入れ先のフォルダと、利用者の PATH に足された項目は、外した後の状態のまま**置いてある**（消していない・入れ直していない）。後片付け（タスク 3.4）がここから続ける。この実測の間、areka は 1 回も起こしていない。

### 1. 写しから戻した（13:54:38〜13:54:44）

上げ直し（タスク 3.2）で消えた物と、その後の起動が書き替えた物を、写し `target\winget-check\state-snapshot\` から入れ先へ戻して、タスク 3.1 の状態にそろえた。入れ先の中で手を入れたのは、これだけ。スクリプトは `target\winget-check\3.3-restore.ps1`（先に、何をするかだけを出させて読み（`logs\3.3-restore-plan.log`）、範囲の外の違いが 0 であることを見てから行った。行った記録は `logs\3.3-restore-apply.log`）。

| 戻した物（入れ先のフォルダから見て） | 行ったこと |
|---|---|
| `ghost\claudia\` | フォルダごと置いた（69 ファイル） |
| `balloon\claudia\` | フォルダごと置いた（37 ファイル） |
| `balloon\claudia_vertical\` | フォルダごと置いた（31 ファイル） |
| `balloon\emo2-kakukaku-wplimit\` | フォルダごと置いた（20 ファイル） |
| えも？？ のゴーストの記憶 `ghost\emo2\ghost\master\profile\`（50 ファイル） | 写しと中身の違う 3 ファイルを上書きした: `areka\sylphya.toml`（70 → 98 バイト）・`pasta\cache\lua\pasta\scene_dic.lua`（586 → 586 バイト・中身が違う）・`pasta\logs\pasta.log`（2788 → 11196 バイト）。残りの 47 ファイルは、起動が作り直した物が写しと同じ中身だったので触っていない。足したファイル・除いたファイルは 0 |
| areka の記憶 `profile\areka\sylphya.toml` | 上書きした（55 → 58 バイト。「前に使っていたゴースト」が `emo2` から、写しの `claudia` へ戻った） |

- 合わせて、置いたファイルは 157・上書きしたファイルは 4・除いたファイルは 0・作ったフォルダは 18。上書きする前の 4 ファイルは `target\winget-check\3.3-set-aside\overwritten\` に取ってある。
- **戻していない物**: winget の控え `Areka.Areka.Portable__DefaultSource.db`（写しの物は 0.0.2 のときの控え。入れ先に在る 0.0.2.1 の控えをそのままにした。戻す前と後で SHA256 は同じ `61748269…EA24CA67`）と、zip が置いたファイル（写しと同じ中身なので、触る必要が無い）。
- 戻す直前（13:54:44）に、`<入れ先>` から動いている `areka.exe`・`shiori-host32-helper.exe` は 0（この名前のプロセス自体が 0）。
- 戻した後の確かめ（13:54:44）: 写しの 358 ファイルのうち winget の控えを除く **357 ファイルすべてが、入れ先に同じ SHA256 で在る**（無い物 0・中身の違う物 0）。**入れ先にだけ在る余分なファイルは 0**。入れ先は 358 ファイル・21846581 バイト・下の階層のフォルダ 58（写しも 58。無いフォルダ 0・余分なフォルダ 0）。違うのは winget の控えのハッシュだけ。
- 外す前の一覧（相対パス・大きさ・SHA256・書いた時刻・作った時刻。隠しファイルも含む）: `target\winget-check\logs\3.3-before.tsv`（13:54:47・**358 ファイル・21846581 バイト**・隠しファイル 1）。フォルダの一覧は `3.3-before.dirs.tsv`。スクリプトは `target\winget-check\3.3-list.ps1`（タスク 3.2 の物と同じ作り）。

設計の段 4 の表の物が、すべて在ることを確かめた（上の一覧で数えた）:

| 物（設計の段 4 の表） | 置き場（入れ先のフォルダから見て） | 外す前の有無 |
|---|---|---|
| 後から入れたゴースト | `ghost\claudia\` | **在る**（69 ファイル） |
| 後から入れたバルーン | `balloon\emo2-kakukaku-wplimit\` | **在る**（20 ファイル） |
| クローディアと一緒に入ったバルーン 2 つ | `balloon\claudia\`・`balloon\claudia_vertical\` | **在る**（37 ファイル）・**在る**（31 ファイル） |
| areka の記憶 | `profile\areka\` | **在る**（1 ファイル・58 バイト・`3E16E14A…325EDDF6`＝写しと同じ） |
| ゴーストの記憶（えも？？） | `ghost\emo2\ghost\master\profile\areka\` | **在る**（1 ファイル・98 バイト） |
| ゴーストの記憶（クローディア） | `ghost\claudia\ghost\master\profile\areka\` | **在る**（1 ファイル・124 バイト・`80ED6700…ADE49528`＝写しと同じ） |
| シェルの記憶（2 体分） | `ghost\emo2\shell\master\profile\areka\`・`ghost\claudia\shell\master\profile\areka\` | 無い＝**測れなかった**（タスク 3.1 のときと同じ。0.0.2 にはシェルの記憶へ書く操作が無い）。シェルのフォルダ `ghost\emo2\shell\master\`（65 ファイル）・`ghost\claudia\shell\master\`（16 ファイル）は在る |

### 2. 外す前に読んだこと（読むだけ・13:55:07〜13:55:09）

- 利用者の設定の「外すときに入れ先を丸ごと消す」（`uninstallBehavior.purgePortablePackage`）: **欄なし＝既定（オフ）のまま**（要件 4.5）。`winget settings export`（終了コード 0）が `userSettingsFile` として示す `%LOCALAPPDATA%\Packages\Microsoft.DesktopAppInstaller_8wekyb3d8bbwe\LocalState\settings.json` は、**ファイル自体が無い**（控えの `settings.json.backup` も無い。同じフォルダはフォルダ 4 つ・ファイル 0 件で、「変えた設定」の節で読んだときと同じ）。組織の決まりのキー（`HKLM`・`HKCU` の `SOFTWARE\Policies\Microsoft\Windows\AppInstaller`）も、どちらも無い。管理者向けの設定は `LocalManifestFiles` だけが `true`。
- `winget list --name areka --disable-interactivity`（終了コード 0・標準エラー 0 バイト）: 1 行だけで、名前 `areka (portable)`・**ID `ARP\User\X64\Areka.Areka.Portable__DefaultSource`**・版 `0.0.2.1`。
- 利用者の側の PATH の登録: 13 項目・入れ先のフォルダは 1 件（13 番目）・値の種類は `ExpandString`。
- `<入れ先>` から動いている `areka.exe`・`shiori-host32-helper.exe` は 0（13:55:09）。

### 3. 外した（要件 4.3）

- 打つスクリプト（`target\winget-check\3.3-fire.ps1`）は、打つ直前にもう一度 `winget list --name areka --disable-interactivity` を読み（13:55:29・終了コード 0）、areka の行が 1 行だけで、その行から引いた ID が `ARP\User\X64\Areka.Areka.Portable__DefaultSource`・版が `0.0.2.1` であることを確かめてから、その ID を渡した。
- `<入れ先>` から動いている `areka.exe`・`shiori-host32-helper.exe` は、13:55:30.045 と 13:55:34.061 の 2 回（4 秒あけて）とも 0（この名前のプロセス自体が 0）。
- コマンド（ワークツリーの根で・普段の権限で＝管理者の役割を持つかを読んで `False`。**`--purge` も `--preserve` も付けていない**。`--force`・`--accept-*` も付けていない）:

  ```powershell
  winget uninstall --id "ARP\User\X64\Areka.Areka.Portable__DefaultSource" --exact --disable-interactivity
  ```

- 始めた時刻 13:55:34.072・終わった時刻 13:55:35.754（1.7 秒）・**終了コード 0**・標準エラーは 0 バイト。問い（同意・確かめ・管理者への切り替え）は 1 つも出なかった。打ったのはこの 1 回だけ。
- winget が出した文（標準出力の全部＝5 行・466 バイト。省いた行・まとめた行は無い。進み具合の描き直しの行は、ファイルへ向けた出力には 1 行も出なかった。UTF-8 として読んだ。4 行目のパスは、実際の文ではアカウントの名前を含む絶対パスで、ここでは頭を `%LOCALAPPDATA%` に置き換えてある。置き換えていない文は `target\winget-check\logs\3.3-uninstall.stdout.txt`）:

  ```text
  ソースの検索中にエラーが発生しました;結果は含まれません: msstore
  見つかりました areka (portable) [ARP\User\X64\Areka.Areka.Portable__DefaultSource]
  パッケージのアンインストールを開始しています...
  ファイルはインストール ディレクトリに残ります: %LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource
  正常にアンインストールされました
  ```

- 1 行目は、winget が物を探すときに Microsoft Store の側（`msstore`）へ問い合わせられなかった、という知らせ。外す相手は機械に入っている物なので、そのまま進んだ（止まっていない・聞かれていない）。
- 4 行目が、入れ先のフォルダにファイルが残ることを告げている。**利用者の物（後から入れたゴーストとバルーン・ゴーストの記憶）を消したことを告げる行は 1 行も無い。**
- 出力の取り方は、入れたとき・上げ直したときと同じ（PowerShell のパイプを通さず、`Start-Process -NoNewWindow -PassThru -RedirectStandardOutput … -RedirectStandardError …` でファイルへ向け、UTF-8 として読んだ。スクリプトは `target\winget-check\3.3-wg.ps1`。時刻と終了コードは `logs\3.3-uninstall.result.json`）。

### 4. 外した後の一覧（13:55:35）と見比べ

一覧は `target\winget-check\logs\3.3-after.tsv` と `3.3-after.dirs.tsv`（**1 ファイル・58 バイト**・下の階層のフォルダ 3・隠しファイル 0）。見比べの全部は `3.3-diff-after.txt`（スクリプトは `3.3-diff.ps1`）。

| 見比べ（358 ファイルの 1 つずつ） | 数 |
|---|---|
| 消えた | **357** |
| 増えた | 0 |
| ハッシュが同じ | 1（`profile\areka\sylphya.toml`。書いた時刻も作った時刻も前と同じ） |
| ハッシュが変わった | 0 |

直下の名前ごと（前 → 後）:

| 直下の名前 | 前 | 後 | 消えた |
|---|---|---|---|
| 直下のファイル 7 つ（zip の最上位の 6 つと、winget の控え） | 7 | 0 | 7 |
| `balloon\` | 138 | 0（フォルダごと無い） | 138（`claudia\` 37・`claudia_vertical\` 31・`emo2-kakukaku-wplimit\` 20・同梱の `emo2-kakukaku\` 21・同梱の `StayseeBalloon\` 29） |
| `ghost\` | 212 | 0（フォルダごと無い） | 212（`claudia\` 69・`emo2\` 143＝うち `ghost\master\profile\` が 50） |
| `profile\` | 1 | 1 | 0 |

- フォルダ（下の階層）: 58 → 3。消えたフォルダは 55（`ghost\` と `balloon\` と、その下のすべて）。残った 3 つ（`.nar-work\`・`profile\`・`profile\areka\`）は、NTFS がフォルダに振る番号も前と同じ＝触られていない。
- 見えた形は上げ直しのときと同じ: zip の中に在る直下の物（`ghost\`・`balloon\`・最上位の 6 ファイル）は、フォルダなら中に後から置かれた物ごと丸ごと消された。zip の中に無い直下の物（`profile\`・`.nar-work\`）は触られなかった。

### 5. 項目ごとの結果（設計の段 6・要件 4.3・4.4）

判定は、上の一覧の見比べで決めた。

| 項目 | 見た物（入れ先のフォルダから見て） | 前 → 後 | 結果 |
|---|---|---|---|
| 後から入れたゴースト | `ghost\claudia\` | 69 ファイル → フォルダごと無い | **消えた** |
| 後から入れたバルーン | `balloon\emo2-kakukaku-wplimit\` | 20 ファイル → フォルダごと無い | **消えた** |
| 〃（クローディアと一緒に入った物） | `balloon\claudia\` | 37 ファイル → フォルダごと無い | **消えた** |
| 〃（同じ） | `balloon\claudia_vertical\` | 31 ファイル → フォルダごと無い | **消えた** |
| areka の記憶 | `profile\areka\` | 1 ファイル → 1 ファイル。`sylphya.toml` はハッシュも書いた時刻も前と同じ（`3E16E14A…325EDDF6`・58 バイト） | **残った** |
| ゴーストの記憶（えも？？） | `ghost\emo2\ghost\master\profile\areka\` | 1 ファイル → フォルダごと無い（上の `ghost\emo2\ghost\master\profile\` の 50 ファイルが丸ごと。ゴースト自身（pasta）の書き残し 49 ファイルも一緒） | **消えた** |
| ゴーストの記憶（クローディア） | `ghost\claudia\ghost\master\profile\areka\` | 1 ファイル → 無い（ゴーストのフォルダごと） | **消えた** |
| シェルの記憶（えも？？） | `ghost\emo2\shell\master\profile\areka\` | 前も後も無い | **測れなかった**（0.0.2 にはシェルの記憶へ書く操作が無く、作れなかった）。シェルのフォルダ `ghost\emo2\shell\master\` は**消えた**（65 ファイル → 無い） |
| シェルの記憶（クローディア） | `ghost\claudia\shell\master\profile\areka\` | 前も後も無い | **測れなかった**（同じ理由）。シェルのフォルダ `ghost\claudia\shell\master\` は**消えた**（16 ファイル → 無い） |
| 入れ先のフォルダ | `<入れ先>` | 在る → 在る（名前も、NTFS の番号も、作った時刻 13:01:48 も前と同じ＝同じフォルダ） | **残った** |

数に入れない物（外したのだから無くなって当たり前の物と、winget 自身の物）に起きたこと:

- zip の最上位の 6 ファイル（`areka.exe`・`shiori-host32-helper.exe`・`README.txt`・`BUILD-INFO.txt`・`LICENSE-MIT`・`THIRD-PARTY-NOTICES.md`）: 6 つとも無くなった。
- 同梱のゴースト `ghost\emo2\`（記憶を除いて 93 ファイル）と、同梱のバルーン `balloon\emo2-kakukaku\`（21 ファイル）・`balloon\StayseeBalloon\`（29 ファイル）: フォルダごと無くなった。
- winget の控え `Areka.Areka.Portable__DefaultSource.db`: 無くなった。
- `.nar-work\`: **残った**（空のまま）。

**まとめ: 消えた物は 6**（後から入れたゴースト 1＝`ghost\claudia\`／後から入れたバルーン 3＝`balloon\emo2-kakukaku-wplimit\`・`balloon\claudia\`・`balloon\claudia_vertical\`／ゴーストの記憶 2＝えも？？ の分とクローディアの分）。残った物は 1（areka の記憶）。測れなかった物は 2（シェルの記憶 2 か所）。入れ先のフォルダは残った。ファイルの数では **357 ファイルが消えた**（利用者の物 207＝後から入れた 4 つのフォルダ 157 と、えも？？ の `ghost\master\profile\` 50／同梱の物 149＝zip の中身の全部／winget の控え 1）。

- 消えた物は、上げ直しのとき（タスク 3.2）と同じ 6 つ。＝利用者の物のうち、`--purge` を付けない外し方で残るのは areka の記憶だけ。
- 消えた物が在るので、設計の段 6 の 4（消えた物が 0 のときだけ、ここで後片付けを確かめる）は通らない。後片付けは、段 7 の入れ直しと `--purge` の後（タスク 3.4）に確かめる。

### 6. 残った入れ先のフォルダの中身（消すのはタスク 3.4）

入れ先のフォルダ `<入れ先>` は**残った**。中身は次の 4 つで全部（隠しファイルも含めて見た。**1 ファイル・58 バイト**・下の階層のフォルダ 3）。`Packages\` の下で名前が `Areka` で始まる物は、このフォルダの 1 つだけ。

| 残った物（入れ先のフォルダから見て） | 種類 | 大きさ |
|---|---|---|
| `.nar-work\` | フォルダ（空） | - |
| `profile\` | フォルダ | - |
| `profile\areka\` | フォルダ | - |
| `profile\areka\sylphya.toml` | ファイル（areka の記憶。`[last]` の `ghost = "claudia"`・`running = ""`） | 58 バイト |

- 残った areka の記憶は、「前に使っていたゴースト」として、もう無い `claudia` を指している。
- このフォルダは消していない（タスク 3.4 が、入れ直して `--purge` で外した後に確かめる）。

### 7. 外した後に読んだこと（読むだけ・13:56:13〜13:56:22。後片付けの 4 項目の正式な確かめはタスク 3.4）

| 見た物 | 見方 | 結果 |
|---|---|---|
| `winget list` の areka の行 | `winget list --name areka --disable-interactivity` と `winget list --id Areka --disable-interactivity` | **0 行**。どちらも `入力条件に一致するインストール済みのパッケージが見つかりませんでした。` と出て、終了コードは -1978335212（`0x8A150014`。見つからなかったときの値）・標準エラー 0 バイト |
| 利用者の PATH に足された項目 | レジストリ `HKCU:\Environment` の `Path` を、環境変数を開かない形で読んだ | **残っている**。13 項目のままで、入れ先のフォルダの項目は 1 件（13 番目）。文字列は、外す前とまったく同じ（値の種類も `ExpandString` のまま）。機械の側の PATH は 18 項目で、上げ直しの前と同じ文字列（`areka` の字を含む項目は 0） |
| 新しい端末での `areka` の解決 | 登録（機械の側;利用者の側）から PATH を組み直した 31 項目にした新しい `pwsh -NoProfile -NonInteractive` のプロセスで `Get-Command areka -All` と `where.exe areka` | **解決しない**（`Get-Command` は 0 件。`where.exe` は終了コード 1）。PATH の項目は残っているが、その先の `areka.exe` がもう無い |
| 入れ先のフォルダ | `Test-Path` | **在る**（上の 6） |
| `%LOCALAPPDATA%\Microsoft\WinGet\Links\` の `areka` で始まる物 | 一覧 | 0 件 |

- 見方が当たりを出せること: 同じ形の `winget list --name PowerShell --disable-interactivity` は 2 行を返し、終了コード 0 だった。新しいプロセスの中で `Get-Command winget -All` は 1 件・`Get-Command pwsh -All` は 2 件を返した（組み直した PATH は効いている）。
- ＝`--purge` を付けない外し方の後、利用者の機械には、入れ先のフォルダ（areka の記憶 1 ファイルと空のフォルダ）と、そのフォルダを指す PATH の項目 1 件が残る。winget の一覧からは消える。
- 読んだ後も、`<入れ先>` から動いているプロセスは 0（13:56:22）。

## 機械の全員向けの実測

結果（要件 4.7）: **機械の全員向け（`--scope machine`）に入れた areka は、普段の権限で起こすと立った。しかし、areka の記憶は書けず、ゴーストを後から入れることもできなかった**。入れ先は `C:\Program Files\WinGet\Packages\Areka.Areka.Portable__DefaultSource`（設計の見込みどおり）で、普段の権限の利用者は、このフォルダの直下に物を作れない。areka は、areka の記憶（`profile\areka\`）と、`.nar` を入れるときの作業フォルダ（`.nar-work\`）を入れ先の直下に作ろうとして、どちらも「アクセスが拒否されました」で失敗した。記憶が書けなかったことは、areka は利用者へ**何も伝えなかった**（起動の記録に残しただけ）。入れられなかったことは、ゴーストの台詞で伝えたが、その台詞は「ファイルが壊れてるのかもね。」で、本当の理由（書けない場所に入っている）は起動の記録にしか出ていない。

時刻はどれも 2026-10-10（+09:00。起動の記録の中の時刻は UTC なので、9 時間足して書いた）。この節の中では、`<入れ先>` は `C:\Program Files\WinGet\Packages\Areka.Areka.Portable__DefaultSource`、`<ワークツリー>` はこのワークツリーの根の絶対パス。

### 1. 誰が何をしたか

| したこと | 誰が | 時刻 |
|---|---|---|
| 管理者の端末で、機械の全員向けに入れた | **開発者** | 14:18〜14:19 ごろ |
| 入れた後の読み（`winget list`・入れ先のフォルダ・PATH の登録 2 つ） | 作業を進めている AI（親のセッション） | 14:19:22 |
| 普段の権限で `areka` を起こした（有界で 1 回・有界でなく 1 回） | 作業を進めている AI | 14:20:44〜14:23:00 |
| 2 回目の起動の中で、右クリックメニューの「インストール…」と「終了」を動かした | 作業を進めている AI（開発者に `areka.exe` の画面の操作を許されて行った。実体の場所が変わったので、開発者は許しの画面でもう一度許した） | 14:22:09〜14:22:52 |
| 残っているファイルの突き合わせ（起動の記録・入れ先の一覧・権限の一覧・タグ `v0.0.2` のソース・配布 zip の中の辞書を読んだ） | この記録を書いた AI | 14:30 より後 |

- この記録を書いた AI は、areka を起こしていないし、winget のコマンドも打っていない。下の値は、残っているファイルを自分で読んで確かめた物。ファイルが残っていない物（開発者の端末に出た winget の文・入れた直後の読み・起こす前の一覧・画面で見た台詞）は、作業を進めている AI から聞いた物として、そう書く。
- 台本（MCP の `sakurascript`）ではなくメニューで行ったのは、タスク 3.1 と同じ理由（0.0.2 の `sakurascript` は中身の無い仮の受け口。「見つかった件と起票」の 1 行目）。終わらせ方も、台本の `\-` ではなくメニューの「終了」。

### 2. 入れた（開発者・管理者の端末）

- 開発者に示したコマンド（管理者の端末で打つ。示したときは `<ワークツリー>` を絶対パスで書いた）:

  ```powershell
  winget install --manifest <ワークツリー>\dist\winget\0.0.2 --scope machine
  ```

- **winget が出した文は取れていない**（開発者の端末に出た。写しは無い）。
- 入れた後の読み（14:19:22・作業を進めている AI・その回の出力のファイルは無い）: `winget list` に `areka (portable)`・ID `ARP\Machine\X64\Areka.Areka.Portable__DefaultSource`・版 `0.0.2` の行／入れ先は `<入れ先>`／機械の側の PATH の登録は 19 項目（入れる前は 18）で、`<入れ先>` の項目が 1 件／利用者の側の PATH の登録は、入れる前の文字列のまま（タスク 3.4 で戻した 12 項目）。
- 残っているファイルで確かめられたこと（この記録を書いた AI）:
  - 14:20:44 に登録から組み直した PATH（`logs\3.5-bounded1-fresh-path.txt`）は **31 項目**。1〜18 番目は、入れる前の機械の側の 18 項目（`logs\2.2-pre-machine-path.txt`）と同じ並び・同じ字面。**19 番目が `<入れ先>`**。20〜31 番目は、利用者の側の 12 項目（`logs\3.4-final-user-path.raw.txt` の変数を開いた形）と同じ。`areka` の字を含む項目は、19 番目の 1 件だけ。＝winget は、機械の側の PATH の登録の末尾へ、入れ先のフォルダを 1 件足した。
  - 入れ先の一覧（下の 5）で、zip の中の物と winget の控えの 150 ファイルの時刻は 14:19:04〜14:19:10。開発者の「14:18〜14:19 ごろ」と合う。
- 起こす前の入れ先（作業を進めている AI の読み。**一覧のファイルは残っていない**＝置き場の指定を誤って保存できなかった。残っているのは次の数だけ）: 150 ファイル・16789117 バイト。直下は `balloon\`・`ghost\`・`Areka.Areka.Portable__DefaultSource.db`・`areka.exe`・`BUILD-INFO.txt`・`LICENSE-MIT`・`README.txt`・`shiori-host32-helper.exe`・`THIRD-PARTY-NOTICES.md`。`%LOCALAPPDATA%\VirtualStore\Program Files\WinGet` は無い。
  - この数は、起動の後の一覧と合う: 起動の後の一覧から、areka が作った `ghost\emo2\ghost\master\profile\` の下を除くと、ちょうど 150 ファイル・16789117 バイトで、直下の 9 つも同じ（下の 5）。

### 3. 1 回目の起動（有界・普段の権限・14:20:44〜14:20:56）

- 起こし方: タスク 2.2 と同じ（登録から組み直した PATH にした新しい `pwsh -NoProfile -NonInteractive` のプロセスの中で、`AREKA_*`・`WINTF_*` を外してから、`areka` の 1 語で起こす）。違いは `AREKA_MCP_PORT=9871` を 1 つ足したことだけ。スクリプトは `target\winget-check\3.5-launch.ps1 -Port 9871 -Tag bounded1`（中で `3.5-inner.ps1` を呼ぶ。名前が `bounded` で始まる回だけ、`AREKA_APP_SMOKE_EXIT_MS=10000` と `AREKA_NO_ALERT=1` を置く）。
- 置いた環境変数: `AREKA_APP_SMOKE_EXIT_MS=10000`・`AREKA_MCP_PORT=9871`・`AREKA_NO_ALERT=1`・`NO_COLOR=1`・`RUST_LOG=info`。外したのは `AREKA_IMPL_WATCH_HOME` の 1 つ。
- `Get-Command areka -All` は 1 件で、`<入れ先>\areka.exe`。起きたプロセスの実体も同じ（起こした直後にプロセスから読んだ）。管理者に上げていない。
- 始めた時刻 14:20:44.981・終わった時刻 14:20:56.592（11.6 秒）・**自分から終わった**・**終了コード 0**（`logs\3.5-bounded1.result.json`）。
- 起動の記録 `logs\3.5-bounded1.stdout.log` は 123 行（29883 バイト）。標準エラーは 1 行（`[helper] SHIORI 初期化の入口: loadu`）。`ERROR` は **3 行**・`WARN` は 6 行（タスク 2.2 と同じ文の 4 行と、下の `session_mark_…` の 2 行）。タスク 2.2 の起動では `ERROR` は 0 行だった。
- 要の行（時刻の順）:

  ```text
  14:20:45.418  INFO areka::boot_config: ベースウェアの根を決めました event="root_resolved" root=C:\Program Files\WinGet\Packages\Areka.Areka.Portable__DefaultSource source=ExeDir
  14:20:46.276 ERROR areka_sylphya::persist: persist commit failed; existing file intact (temp→rename), reporting Degraded scope=App path=C:\Program Files\WinGet\Packages\Areka.Areka.Portable__DefaultSource\profile\areka\sylphya.toml error=アクセスが拒否されました。 (os error 5)
  14:20:46.276  WARN areka::boot_resolve: [boot_resolve] 起動中の印を記憶へ書けませんでした（このゴーストが落ちても、次の起動の Ref6/7 にこのゴーストの名前は載りません） event="session_mark_write_degraded" ghost="えも？？" dir=C:\Program Files\WinGet\Packages\Areka.Areka.Portable__DefaultSource\profile\areka
  14:20:46.431  INFO areka::boot_resolve: [boot_resolve] 最後に使ったものを記憶へ書きました（- は argv なので書いていない） event="last_used_recorded" ghost="emo2" balloon="emo2-kakukaku" shell="master"
  14:20:46.432 ERROR actor{actor=sylphya}: areka_sylphya::persist: persist commit failed; … scope=App path=…\profile\areka\sylphya.toml error=アクセスが拒否されました。 (os error 5)
  14:20:46.434  INFO actor{actor=emo-text}: areka::ghost_session: 本物のゴースト窓を開きました（placement シーム・スコープごとにキャラ窓＋バルーン窓） scopes=[0, 1]
  14:20:56.435  INFO actor{actor=emo-text}: areka::app_exit: [quit_app] 全窓を閉じ、終了を指示した event="app_exit" origin=Smoke closed=4
  14:20:56.518 ERROR actor{actor=emo-text}: areka_sylphya::persist: persist commit failed; … scope=App path=…\profile\areka\sylphya.toml error=アクセスが拒否されました。 (os error 5)
  14:20:56.518  WARN actor{actor=emo-text}: areka::boot_resolve: [boot_resolve] 起動中の印を消せませんでした（きれいに終わったのに、次の起動は前回落ちたとして既定のゴーストで Ref6/7 付きになります） event="session_mark_clear_degraded" dir=C:\Program Files\WinGet\Packages\Areka.Areka.Portable__DefaultSource\profile\areka
  ```

- `本物のゴースト窓を開きました` は **1 件**。`ghost_resolved` は `route=Only dir=<入れ先>\ghost\emo2`・`balloon_resolved` は `route=Companion dir=<入れ先>\balloon\emo2-kakukaku`（タスク 2.2 の初めての起動と同じ決まり方）。起動の挨拶は `OnFirstBoot`。
- 記憶への書き込みの失敗は 3 回（起こす前に起動中の印を書くとき・最後に使ったゴーストを書くとき・終わるときに印を消すとき）。3 回とも、相手は areka の記憶 `<入れ先>\profile\areka\sylphya.toml`。`session_mark_written`・`session_mark_cleared` の行は 0 件。

### 4. 2 回目の起動（有界でない・普段の権限・`AREKA_NO_ALERT` なし・14:21:45〜14:23:00）

- 起こし方: `3.5-launch.ps1 -Port 9871 -Tag menu1`。1 回目との違いは、`AREKA_APP_SMOKE_EXIT_MS` と `AREKA_NO_ALERT` を**置かない**ことだけ（`AREKA_NO_ALERT` を置くと、0.0.2 はメニューの「インストール…」でファイルを選ぶ画面を出さない）。置いた環境変数は `AREKA_MCP_PORT=9871`・`NO_COLOR=1`・`RUST_LOG=info`。実体は `<入れ先>\areka.exe`。管理者に上げていない。
- 始めた時刻 14:21:45.692・終わった時刻 14:23:00.863（75.2 秒）・**自分から終わった**（メニューの「終了」）・**終了コード 0**（`logs\3.5-menu1.result.json`）。
- 起動の記録 `logs\3.5-menu1.stdout.log` は 347 行（84540 バイト）。標準エラーは 1 行（1 回目と同じ）。`ERROR` は **5 行**（記憶への書き込みの失敗 3 行と、入れられなかったことの 2 行）・`WARN` は 5 行（タスク 2.2 と同じ文の 3 行と、`session_mark_…` の 2 行）。
- 起きたことと、その行:

  | 時刻 | 起きたこと | 起動の記録の行 |
  |---|---|---|
  | 14:21:45.764 | 根が決まった | `event="root_resolved" root=<入れ先> source=ExeDir` |
  | 14:21:45.766〜.783 | ゴーストとバルーンが決まった | `ghost_resolved route=Only dir=<入れ先>\ghost\emo2`・`balloon_resolved route=Memory dir=<入れ先>\balloon\emo2-kakukaku` |
  | 14:21:46.009〜.010 | 起動中の印が書けなかった | `ERROR … persist commit failed; … scope=App path=<入れ先>\profile\areka\sylphya.toml error=アクセスが拒否されました。 (os error 5)` → `WARN … event="session_mark_write_degraded" ghost="えも？？"` |
  | 14:21:46.363〜.364 | 最後に使ったゴーストが書けなかった | `INFO … event="last_used_recorded" ghost="emo2" balloon="emo2-kakukaku" shell="master"` → 次の行が `ERROR … persist commit failed; … scope=App …` |
  | 14:21:46.376 | ゴーストが立った | `本物のゴースト窓を開きました … scopes=[0, 1]`（**1 件**） |
  | 14:21:47.093 | 起動の挨拶（`OnFirstBoot`）。14:22:04.208 に終わった | `event="boot_talk" talk_id=1`・`source=OnFirstBoot`・`prop_set_cue applied key="areka.boot.count" value="1"`（14:22:04.206） |
  | 14:22:09.849 | 右クリックメニューが出た | `[menu] shown event="menu_shown" scope=0 items=8` |
  | 14:22:12.403 | 「インストール…」を選んだ → ファイルを選ぶ画面で `<ワークツリー>\target\winget-check\nar\claudia.nar` を開いた | `[menu] selected event="menu_selected" scope=0 frame=Install id=6` |
  | 14:22:21.971〜.972 | areka が依頼を受けて、手続きを始めた | `[install] 依頼を受けました event="install_order_queued" origin=Menu count=1` → `[install] 書庫の手続きを始めます event="install_begin" archive=<ワークツリー>\target\winget-check\nar\claudia.nar origin=Menu` |
  | 14:22:21.982〜.984 | ゴーストへ「入れ始めた」を知らせ、ゴーストが台詞で答えた | `event="steady_talk" talk_id=2 origin="OnInstallBegin"` → `[install] イベントを送りました event="install_event" id="OnInstallBegin" raised=Script` |
  | 14:22:22.030 | 書庫の中身は受け取れる物と判定した | `[install] 受け取ります event="install_accept" … verdict="accepted" accept=None target_ghost=None` |
  | 14:22:22.031 | **入れ先の直下に作業フォルダを作れなかった** | `ERROR actor{actor=install}: areka_nar: [areka_nar] refused or failed archive=…\claudia.nar reason=…\claudia.nar: Stage で I/O に失敗: C:\Program Files\WinGet\Packages\Areka.Areka.Portable__DefaultSource\.nar-work\34544-0: アクセスが拒否されました。 (os error 5) committed=0 rolled_back=true work=` |
  | 14:22:22.032 | **入れられなかった** | `ERROR actor{actor=install}: areka::install::procedure: [install] 書庫を入れられませんでした event="install_failed" archive=…\claudia.nar kind="Io" phase="stage" rolled_back=true word="extraction"` |
  | 14:22:22.042〜.044 | ゴーストへ「入れられなかった」を知らせ、ゴーストが台詞で答えた（14:22:26.909 に終わった） | `event="steady_talk_replace" talk_id=3 origin="OnInstallFailure"` → `[install] イベントを送りました event="install_event" id="OnInstallFailure" raised=Script` |
  | 14:22:31.104・14:22:47.114 | 普段のおしゃべりに戻った | `event="steady_talk" talk_id=4 origin="OnSecondChange"`・`talk_id=5 origin="OnSecondChange"` |
  | 14:22:45.178・14:22:45.421 | MCP の口から 2 つ読んだ（`target\winget-check\3.5-mcp.ps1`） | `get_active_ghost_list` の答えは `えも？？`／`get_log` の答えは areka 自身の記録の 8 件（上の `persist commit failed` 2 件・`session_mark_write_degraded`・`[areka_nar] refused or failed`・`install_failed` を含む。台詞は入っていない）。`logs\3.5-mcp.log` |
  | 14:22:52.365 | 右クリックメニューの「終了」を選んだ | `[menu] selected event="menu_selected" scope=0 frame=Close id=8` |
  | 14:22:56.159〜14:23:00.769 | 終わりの挨拶をして、きれいに終わった | `OnClose GET を発行し握手を開始 event="close_handshake_begin" reason="user"` → `event="talk_done_quit" talk_id=6` → `正規 clean shutdown 完了 … event="unload_clean"` → `event="app_exit" origin=KanadeStopped(Quit) closed=4` → `ghost shutdown sequence completed` |
  | 14:23:00.770 | 起動中の印を消せなかった | `ERROR … persist commit failed; … scope=App …` → `WARN … event="session_mark_clear_degraded"` |
  | 14:23:00.779 | 最後の行 | `MCP: 待受を閉じた addr=127.0.0.1:9871`（プロセスが消えたのは 0.08 秒後） |

- 画面で見た台詞（作業を進めている AI が 14:22:26 ごろの画面の写しで見た物。**写しのファイルは残っていない**。起動の記録には台詞の本文は出ない）: 「えー、なんでなん！？」（本体の側）と「ファイルが壊れてるのかもね。」（相方の側）。数秒後には普段のおしゃべりに戻っていた。
- この台詞の出どころ（この記録を書いた AI が、配布 zip `target\winget-check\release\areka-0.0.2-x64.zip` の中の辞書を読んだ。`logs\3.5-zip-grep.txt`）: 同梱のゴースト えも？？ の辞書 `ghost\emo2\ghost\master\dic\install.pasta` には、`OnInstallFailure` の台詞が 3 つ在り、頭の注釈は「`OnInstallFailure`（reference[0] = 失敗理由）は理由を問わず共通トーク」。その 2 つ目（94〜97 行目）が、画面で見た 2 行を含む:

  ```text
  ＊OnInstallFailure
  　　　エモ：＠静観　インストールできなかったみたい。
  　むらさき：＠そんなあ　えー、なんでなん！？
  　　　エモ：＠通常　ファイルが壊れてるのかもね。
  ```

  起動の記録でも、`OnInstallFailure` の台詞（`talk_id=3`）の間に、相方の側のバルーン → 本体の側のバルーンの順に文字が出ている（14:22:22.063 `scope=1 trigger="content" visible=true`・14:22:23.863 `scope=0 trigger="content" visible=true`）。この辞書の並び（エモ → むらさき → エモ）と合う。
- areka がゴーストへ渡した失敗の理由は、正典の語 `extraction` の 1 語だけ（上の `install_failed` の行の `word="extraction"`）。タグ `v0.0.2` の `crates/areka/src/install/judge.rs` の関数 `failure_word` は、書庫が壊れているとき（`CorruptArchive`・`IntegrityMismatch`・`NameUndecodable`）と、ファイルの読み書きに失敗したとき（`NarError::Io`。今回はこちら＝`kind="Io" phase="stage"`）を、同じ `extraction` に写す。＝ゴーストの側からは、「書庫が壊れている」と「入れ先へ書けない」の見分けがつかない。
- 2 回目の起動は、「前回はきれいに終わらなかった」としては**起きていない**: `event="session_mark_found"`（タグ `v0.0.2` の `crates/areka/src/boot_config.rs` が、残っている印を見つけたときに出す行）は 0 件で、起動の挨拶は普段と同じ `OnFirstBoot`。1 回目の終わりの警告は「次の起動は前回落ちたとして…」と言うが、印は起こす前にも書けていなかった（消す物が無かった）ので、そうはならなかった。
- 2 回目も起動の挨拶が `OnFirstBoot` だったのは、書けなかったこととは別の話。利用者向けに入れたとき（タスク 2.2 の有界の起動 → タスク 3.1 の 1 回目）も同じ並びだった。初めての起動かどうかは、ゴーストごとの記憶に起動の回数（`areka.boot.count`）が在るかで決まり（タグ `v0.0.2` の `crates/areka-kanade/src/msg.rs` の説明）、有界の起動は、初めての挨拶が終わる前（この回数を書く前）に終わる。
- タスク 3.1 の 3 回目で見た「最後の行からプロセスが消えるまで約 6 秒」は、今回は出なかった（1 回目は 0.07 秒・2 回目は 0.08 秒）。ただし、今回は `.nar` が入っていない。

### 5. 起動の後の入れ先のフォルダ（一覧 2 つ・14:21:42 と 14:23:20）

一覧は `logs\3.5-installdir-after-bounded1.tsv`（1 回目の後）と `logs\3.5-installdir-after-menu1.tsv`（2 回目の後）。隠しファイルも含む。列は、入れ先から見たパス・大きさ（フォルダは `<dir>`）・時刻。数えたスクリプトは `target\winget-check\3.5-listing-summary.py`、出た文は `logs\3.5-listing-summary.txt`。

| | 1 回目の後 | 2 回目の後 |
|---|---|---|
| ファイル | 200・17211723 バイト | 200・17214486 バイト |
| うち `ghost\emo2\ghost\master\profile\` の下（areka とゴーストが作った物） | 50・422606 バイト | 50・425369 バイト |
| うち、それ以外（zip の中の物と winget の控え） | 150・16789117 バイト | 150・16789117 バイト |
| 下の階層のフォルダ | 37（うち 15 が `ghost\emo2\ghost\master\profile\` と、その下） | 37（同じ） |
| 直下の物 | 9 つ（`balloon\`・`ghost\`・`Areka.Areka.Portable__DefaultSource.db`・`areka.exe`・`BUILD-INFO.txt`・`LICENSE-MIT`・`README.txt`・`shiori-host32-helper.exe`・`THIRD-PARTY-NOTICES.md`） | 同じ 9 つ |
| 直下の `profile\`（areka の記憶） | **無い**（0 件） | **無い**（0 件） |
| 直下の `.nar-work\` | **無い**（0 件） | **無い**（0 件） |
| `ghost\` の直下 | `emo2` の 1 つだけ | `emo2` の 1 つだけ |
| `balloon\` の直下 | `emo2-kakukaku`・`StayseeBalloon` の 2 つ（同梱の物だけ） | 同じ 2 つ |
| 名前に `claudia` を含む物 | 0 件 | 0 件 |

- **2 回の起動が入れ先の中に作った物は、`ghost\emo2\ghost\master\profile\` の下の 50 ファイル・15 フォルダだけ**。直下には何も増えていない。zip の中の物と winget の控えの 150 ファイルは、数も大きさの合計も起こす前の読みと同じ。
- 50 ファイルの内訳: areka がゴーストごとに持つ記憶 `ghost\emo2\ghost\master\profile\areka\sylphya.toml` が 1 つと、ゴースト自身（pasta）の書き残し 49。
- 1 回目の後と 2 回目の後で、パスの並びは同じ（増えた物・消えた物は 0）。変わったのは 4 ファイル（と、フォルダ 2 つの時刻）で、どれも `ghost\emo2\ghost\master\profile\` の下: `areka\sylphya.toml`（70 → 90 バイト・時刻 14:20:46 → 14:22:04。上の `areka.boot.count` を書いた時刻と合う）・`pasta\logs\pasta.log`（2612 → 5355 バイト）・`pasta\cache\lua\pasta\scene_dic.lua`・`pasta\save\save.json`（大きさは同じで、時刻だけ）。
- `%LOCALAPPDATA%\VirtualStore\` の下に、同じ並びはできていない。作業を進めている AI の読み（起こす前と、2 回目の後。出力のファイルは無い）では `%LOCALAPPDATA%\VirtualStore\Program Files\WinGet` は無い。この記録を書いた AI も 14:34:39 に読んだ: `%LOCALAPPDATA%\VirtualStore` は在るが、中身は 0 件（`Program Files` も無い。名前に `Areka`・`sylphya`・`nar-work` を含む物も 0 件。`logs\3.5-virtualstore.txt`）。＝Windows は、書けなかった書き込みを別の場所へ逃がしていない。

### 6. 5 項目（設計の段 8 の表・要件 4.7）

| 項目 | 見た物 | 結果 | 根拠の記録 |
|---|---|---|---|
| 入れ先のフォルダの場所 | `root_resolved` の行の `root=` | **`C:\Program Files\WinGet\Packages\Areka.Areka.Portable__DefaultSource`**（`source=ExeDir`。2 回とも同じ。設計の見込みどおり、`%ProgramFiles%\WinGet\Packages\` の下） | `logs\3.5-bounded1.stdout.log`・`3.5-menu1.stdout.log` の 2 行目／`logs\3.5-bounded1.result.json`・`3.5-menu1.result.json`（`areka` の 1 語の解決先） |
| ゴーストが立ったか | `本物のゴースト窓を開きました` の件数 | **立った**（1 回目 1 件・2 回目 1 件。どちらも同梱の えも？？。終了コードはどちらも 0） | 同じ 2 つの起動の記録／`logs\3.5-mcp.log`（`get_active_ghost_list` の答え `えも？？`） |
| areka の記憶が書けたか | 入れ先の `profile\areka\` の有無・`%LOCALAPPDATA%\VirtualStore\` | **書けなかった**。`<入れ先>\profile\` は、2 回の起動の後も無い。起動の記録に `persist commit failed … scope=App path=<入れ先>\profile\areka\sylphya.toml error=アクセスが拒否されました。 (os error 5)` が 1 回の起動につき 3 行（計 6 行）。`VirtualStore` の下にも無い | 一覧 2 つ（`logs\3.5-installdir-after-*.tsv`・`3.5-listing-summary.txt`）／起動の記録 2 つ／`logs\3.5-virtualstore.txt` |
| ゴーストを後から入れられたか | 入れ先の `ghost\` の下に検体のフォルダができたか | **入れられなかった**。`ghost\` の下は `emo2` だけで、`claudia` を含む物は 0 件。areka は書庫を受け取れる物と判定した後、入れ先の直下に作業フォルダ `.nar-work\34544-0` を作れずに止めた（`Stage で I/O に失敗: …: アクセスが拒否されました。 (os error 5)`・`install_failed kind="Io" phase="stage" rolled_back=true`）。試したのは `claudia.nar` の 1 回だけ | 2 回目の後の一覧／`logs\3.5-menu1.stdout.log`（上の 4 の表の 14:22:21〜14:22:22 の行） |
| うまくいかなかったときに areka が利用者へ伝えたこと | ゴーストの台詞・起動の記録の警告とエラーの行 | **入れられなかったこと**: ゴーストの台詞で伝えた（「えー、なんでなん！？」「ファイルが壊れてるのかもね。」）。理由として口にしたのは「ファイルが壊れているのかも」で、本当の理由（入れ先に書けない）は、起動の記録の `ERROR` 2 行にしか出ていない。**記憶が書けなかったこと**: **何も伝えていない**。起動の記録に `ERROR` 3 行と `WARN` 2 行（1 回の起動につき）が残っただけで、このことを伝える台詞もイベントも 0（2 回の起動で出た台詞の出どころは `OnFirstBoot`・`OnInstallBegin`・`OnInstallFailure`・`OnSecondChange`・`OnClose` だけ。知らせの画面を告げる行も 0。2 回目は `AREKA_NO_ALERT` を置いていない） | 画面で見た台詞は、作業を進めている AI の控え（写しのファイルは無い）／辞書は `logs\3.5-zip-grep.txt`／起動の記録 2 つ／`logs\3.5-mcp.log`（`get_log` の答え）。`dump_balloon` は呼んでいない |

- まとめ: 5 項目のうち、見込みどおりだったのは 2（場所・立った）。**うまくいかなかったのは 2（areka の記憶・後から入れる）**。伝え方は、入れる失敗が「理由の違う台詞」・記憶の失敗が「無言」。

### 7. 表の外で分かったこと

- **ゴーストごとの記憶は、この機械では書けた**。`ghost\emo2\ghost\master\profile\` の下に 50 ファイルができ、2 回目の起動はそれを読んでいる（1 回目のバルーンの決まり方は `route=Companion`、2 回目は `route=Memory`＝1 回目が書いた「最後に使ったバルーン」を読んだ）。一方、areka の記憶に書くはずの「最後に使ったゴースト」は読めていない（2 回目の `ghost_resolved` は `route=Only`。利用者向けに入れたときの 2 回目の起動＝タスク 3.1 の 1 回目は `route=Memory` だった）。
- 書けた理由は、入れ先の権限（`logs\3.5-acl.txt`。作業を進めている AI が 14:23:20 に読んだ 7 か所。取ったコマンドは残っていない）:

  | 場所（入れ先から見て） | 持ち主 | 権限の行 |
  |---|---|---|
  | `<入れ先>` そのもの | `BUILTIN\Administrators` | `BUILTIN\Administrators` に全部の権限／`BUILTIN\Users` に**読むことと実行だけ**／`CREATOR OWNER`（下の階層へ引き継ぐ分だけ）。この端末の利用者の行は**無い** |
  | `ghost\`・`ghost\emo2\`・`ghost\emo2\ghost\master\`・`balloon\` | `BUILTIN\Administrators` | **この端末の利用者に全部の権限**（引き継いだ行）／`BUILTIN\Administrators` に全部の権限。`BUILTIN\Users` の行は無い |
  | `ghost\emo2\ghost\master\profile\` | この端末の利用者（areka が作った） | 上と同じ 2 つ |
  | `areka.exe` | `BUILTIN\Administrators` | この端末の利用者に全部の権限／`BUILTIN\Administrators` に全部の権限。`BUILTIN\Users` の行は無い |

  - ＝普段の権限の areka は、入れ先の**直下**には何も作れない（`profile\`・`.nar-work\` が作れなかった理由）。`ghost\`・`balloon\` の中へは書けた。この端末の利用者の行が下の階層にだけ在るのは、入れた開発者のアカウント（管理者に上げたこの端末の利用者）から来ている見込みだが、仕組みは調べていない。
  - **注意**: これは、入れた管理者と普段の利用者が**同じアカウント**の機械での結果。入れた管理者が別のアカウントの機械では、この行は入れた側のアカウントの物になる見込みで、普段の利用者は `ghost\` の下の記憶も書けないはず。さらに、上の一覧のとおり `areka.exe`・`ghost\`・`balloon\` に `BUILTIN\Users` の行が無いので、別のアカウントの利用者は、読むことも起こすこともできないおそれが在る。どれも**測っていない**（権限の一覧からの読み）。
- **起動中の印は、書くことも消すこともできなかった**（1 回の起動につき `session_mark_write_degraded` 1 行・`session_mark_clear_degraded` 1 行）。印は「今動いているゴーストの名前」を areka の記憶に書いておき、きれいに終わったときだけ消す物で、次の起動で残っていれば「前回はきれいに終わらなかった」と分かる仕組み（タグ `v0.0.2` の `crates/areka/src/boot_resolve.rs` の説明）。ここから出てくること:
  - この入れ方では、areka は**前回がきれいに終わらなかったことに気付けない**（落ちても、次の起動は何事も無かったように立つ）。利用者には、この仕組みが働いていないことは伝わらない。
  - 逆の心配（きれいに終わったのに、次の起動が「前回落ちた」扱いになる）は、今回は**起きなかった**（上の 4。印がそもそも書けていないので、残る物が無い）。終わるときの警告の文は、この場合の実際とは合っていない。
  - 起動の記録の `INFO … 最後に使ったものを記憶へ書きました event="last_used_recorded"` は、すぐ次の行で areka の記憶への書き込みが失敗しているのに出ている（バルーンとシェルの分はゴーストごとの記憶に書けたが、ゴーストの分は書けていない）。
- Windows は、書けなかった書き込みを `%LOCALAPPDATA%\VirtualStore\` へ逃がしていない（上の 5）。

### 8. 外して、設定を戻した（開発者・管理者の端末・14:40〜14:47。要件 4.8・3.4）

この 8 は、上の 1〜7 とは別の AI が書いた（「この節を書いた AI」。areka を起こしていないし、入れる・上げ直す・外す・設定を変える winget のコマンドも打っていない。打ったのは読むだけの `winget list`・`winget settings export`・`winget error`）。**開発者の管理者の端末に出た文は、1 つも取れていない。** 下に書くのは、作業を進めている AI（親のセッション）の読み（聞いた物。その回の出力のファイルは無い）と、winget 自身の記録（この節を書いた AI が読んだ）と、この節を書いた AI が 14:49 に機械を読み直した値。

作業を進めている AI が、開発者へ 1 度に示したコマンド（管理者の端末で打つ。AI は、機械の全員向けの物を外さない・PATH を変えない・winget の設定を変えない）:

```powershell
# 1. 外す（後片付けなので --purge を付ける）
winget uninstall --id "ARP\Machine\X64\Areka.Areka.Portable__DefaultSource" --exact --purge
# 2. 入れ先のフォルダが残っていたときの備え
Remove-Item -LiteralPath "C:\Program Files\WinGet\Packages\Areka.Areka.Portable__DefaultSource" -Recurse -Force -ErrorAction SilentlyContinue
# 3. 機械の PATH から、入れ先のフォルダの項目（と、空の項目）だけを除いて書き戻す
$k='HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment'; $o=(Get-Item $k).GetValue('Path','','DoNotExpandEnvironmentNames'); $n=(($o -split ';') | Where-Object { $_ -and $_ -notlike '*\WinGet\Packages\Areka.Areka.Portable__DefaultSource' }) -join ';'; Set-ItemProperty $k -Name Path -Value $n -Type ExpandString
# 4. 設定を戻す
winget settings --disable LocalManifestFiles
```

どれを打ったか:

| コマンド | 打ったか | いつ | 何で分かるか |
|---|---|---|---|
| 1（`--purge` を付けて外す） | **打った**（開発者） | 14:41:07.483〜14:41:08.643 | winget 自身の記録（下） |
| 2（フォルダの備え） | **打っていない**（要らなかった。1 でフォルダが消えた） | - | 作業を進めている AI から聞いた。winget 自身の記録の `Purged install location directory` の行と、14:41:29 の読み（フォルダは無い）とは合う |
| 3（機械の PATH の項目を消す） | **打った**（開発者） | 効いたのは 14:43:46 より後・14:46:58 より前 | 前後の読み（14:43:46 は 19 項目・14:46:58 は 18 項目）。このコマンドの記録はどこにも残らない（それより前にも打っていたかどうかは、分からない） |
| 4（設定を戻す） | **打った**（開発者） | 14:43:20.772〜14:43:20.805 | winget 自身の記録（上の「変えた設定」） |

- 効いた順は 1 → 4 → 3。3 は、作業を進めている AI とのやり取りをもう 1 往復してから効いた（示し方が短すぎて、開発者から「どのフォルダか」「PATH の行は何をするのか」と聞き返された。下の「既知の制限」）。

winget 自身の記録から読めたこと（`%LOCALAPPDATA%\Packages\Microsoft.DesktopAppInstaller_8wekyb3d8bbwe\LocalState\DiagOutputDir\` の、その回のファイル。写しは `target\winget-check\logs\3.5-final-winget-diag-*.txt`。パスの頭は置き換えてある）:

- **入れた回**（上の 2 の補い。`3.5-final-winget-diag-install-machine.txt`）: `install --manifest <ワークツリー>\dist\winget\0.0.2 --scope machine` が 14:18:56.553 に始まり、14:19:10.530 に `Leaf command succeeded: root:install` で終わった（管理者に上げたプロセス＝`Level[1]`）。14:19:10.265 に `Created target install directory: <入れ先>`（＝winget が自分で作ったフォルダ）、14:19:10.426 に `Appending portable target directory to PATH registry: <入れ先>`。
- **外した回の 1 度目**（`3.5-final-winget-diag-uninstall-machine-refused.txt`）: 同じ `uninstall … --exact --purge` が 14:40:58.947 に始まり、相手を 1 つ見つけた直後の 14:41:00.086 に `Terminating context: 0x8a150019` で止まった。管理者に上げていないプロセス（`Level[0]`）で、`0x8a150019` は「管理者の権限が要る」（`winget error 0x8a150019` の答えは `APPINSTALLER_CLI_ERROR_COMMAND_REQUIRES_ADMIN`）。何も消していない（消したことを告げる行は 0）。
- **外した回の 2 度目**（`3.5-final-winget-diag-uninstall-machine-purge.txt`）: 7 秒後の 14:41:07.485 に、管理者に上げたプロセス（`Level[1]`）で同じコマンドが始まり、14:41:08.643 に `Leaf command succeeded: root:uninstall` で終わった。要の行（記録の中の並びのまま。行の頭の日付は省き、入れ先のパスは `<入れ先>` に置き換え、続く 4 行を 1 行にまとめた所が 1 つ在る）:

  ```text
  14:41:08.399 <I> [CLI ] Found one app. App id: ARP\Machine\X64\Areka.Areka.Portable__DefaultSource App name: areka (portable)
  14:41:08.560 <I> [CLI ] Deleting portable exe at: <入れ先>\areka.exe
  14:41:08.566 <I> [CLI ] Removing directory at <入れ先>\balloon
  14:41:08.578 <I> [CLI ] Deleting portable exe at: <入れ先>\BUILD-INFO.txt
  14:41:08.583 <I> [CLI ] Removing directory at <入れ先>\ghost
  14:41:08.609〜.625  （LICENSE-MIT・README.txt・shiori-host32-helper.exe・THIRD-PARTY-NOTICES.md を 1 つずつ）
  14:41:08.631 <I> [CORE] Install directory is not empty: <入れ先>
  14:41:08.636 <I> [CLI ] Portable index deleted: <入れ先>\areka.areka.portable__defaultsource.db
  14:41:08.636 <I> [CLI ] Purged install location directory. Deleted 1 files or directories
  14:41:08.638 <I> [CLI ] PortableARPEntry deleted.
  ```

  - `Install directory is not empty` の行が、機械の PATH の項目を消さなかった所（利用者の側と同じ行。理由は下の「後片付けの確かめ」の「利用者の側」の 7＝winget は、PATH の項目を消そうとする時点で入れ先が空でないと何もしない。この時点では winget 自身の控えがまだ在る）。
  - `Purged install location directory` の行が、入れ先のフォルダを丸ごと消した所。利用者の側（タスク 3.4）では出なかった行で、違いは、今回は winget が自分で作ったフォルダだったこと（入れた回の `Created target install directory`）。areka が中に作った `ghost\emo2\ghost\master\profile\` の下の 50 ファイルは、`ghost` のフォルダごと消えた。

作業を進めている AI の読み（聞いた物。時刻の順）:

| 時刻 | 読んだ値 |
|---|---|
| 14:30:25（外す前） | `LocalManifestFiles` は `true`／`<入れ先>` は在る／機械の PATH は 19 項目で、areka の項目が 1 件 |
| 14:41:29・14:41:46（開発者が外した後） | `<入れ先>` は**無い**・`C:\Program Files\WinGet\Packages\` の下で名前が `Areka` で始まる物は 0 件・`winget list --name areka` は 0 行／機械の PATH は **19 項目のまま**で、areka の項目が 1 件残っている／`LocalManifestFiles` はまだ `true` |
| 14:43:46 | `LocalManifestFiles` は **`false`**（管理者向けの設定は 6 つとも `false`）／機械の PATH はまだ 19 項目・716 字・areka の項目 1 件で、入れる前の文字列と同じでない／利用者の PATH は 12 項目で、入れる前の文字列と同じ／登録から組み直した PATH の上に `areka.exe` は無い（同じ見方で `winget.exe` は 1 件）／`winget list --name areka` は 0 行で終了コード -1978335212（同じ形の `--name PowerShell` は終了コード 0）／`Links\` に `areka` で始まる物は無い／areka のプロセスは無い |
| 14:46:58（開発者が 3 を打った後） | 機械の PATH は **18 項目・646 字・areka の項目 0 件**・値の種類 `ExpandString`・入れる前に取っておいた文字列（`logs\2.2-pre-machine-path.txt`）と、大文字と小文字を区別して**同じ**／`LocalManifestFiles` は `false`／`<入れ先>` は無い |

- ＝**`--purge` を付けた外す操作は、入れ先のフォルダは消したが、機械の PATH の項目は残した**（利用者の側と同じ動き）。項目は、開発者が手で消すまで残っていた。
- 機械の側の 4 項目の今の値と、この節を書いた AI の読み直し（14:49）は、下の「後片付けの確かめ」の「機械の側」。

## 後片付けの確かめ

### 利用者の側（タスク 3.4・要件 1.10・2.2・3.1・3.6・4.6）

結果: **利用者の側の後片付けの 4 項目は、すべて 0**。注意書きを足す・`winget validate`・入れ直し・注意書きが出たことの記録・`--purge` を付けて外す、まで済んだ。ただし、**winget の外す操作だけでは 0 にならなかった**: `--purge` を付けて外した直後（14:09）は、`winget list` の行と `areka` の解決は 0 だったが、入れ先のフォルダ（areka の記憶の 1 ファイルと空のフォルダ）と、利用者の PATH の項目 1 件が残っていた（下の 6。理由は下の 7）。残った 2 つは、フォルダを AI が（中身の一覧を取ってから）、PATH の項目を開発者が手で、それぞれ片付けた（下の 8）。時刻はどれも 2026-10-10（+09:00）。`<入れ先>` は `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource`。この節の間、この入れ先の areka は 1 回も起こしていない。

4 項目の今の値（どの読みで 0 と言うか・誰が何をしたか）:

| 項目 | 結果 | いつ・どう読んだか | 0 にしたのは誰か |
|---|---|---|---|
| ① 入れ先のフォルダ | **0**（`<入れ先>` は無い。`%LOCALAPPDATA%\Microsoft\WinGet\Packages\` の下で名前が `Areka` で始まる物は 0 件・`…\WinGet\Links\` の下で `areka` で始まる物も 0 件） | 14:22:20 に `Test-Path` と一覧で読んだ（この記録を書いた AI）。同じ `Packages\` には、ほかのパッケージのフォルダが 2 つ在り、同じ `Test-Path` はそのフォルダに `True` を返した（＝在る物は見つかる） | `--purge` を付けた外し方では消えなかった。作業を進めている AI（親のセッション）が、中身の一覧を取ってから 14:17:20 に消した（下の 8） |
| ② 新しい端末での `areka` の解決 | **0**（`Get-Command areka -All` は 0 件・`where.exe areka` は終了コード 1） | **14:09 の読み**（`--purge` で外した直後。下の 6）。14:19 より後には読み直していない（理由は表の下） | winget の外す操作（`areka.exe` が消えた） |
| ③ 利用者の PATH に足された項目 | **0**（`HKCU:\Environment` の `Path` は 12 項目・524 字・値の種類 `ExpandString`。入れ先のフォルダの項目は 0 件・`areka` の字を含む項目も 0 件。入れる前に取っておいた文字列 `logs\2.2-pre-user-path.txt` と、大文字と小文字を区別して比べて**同じ**） | 14:22:20 に、環境変数を開かない形で読んだ（この記録を書いた AI）。同じ数え方を、`--purge` の直後に取っておいた文字列（616 字）に当てると 1 件で、その文字列は入れる前の文字列と同じでない、と出る（＝残っていれば見つかる） | winget の外す操作では消えなかった。**開発者が手で**、14:18 ごろに消した（下の 8。AI は PATH を変えていない） |
| ④ `winget list` の areka の行 | **0**（`--name areka` でも `--id Areka` でも 0 行） | **14:09 の読み**（`--purge` で外した直後。下の 6）。14:19 より後には読み直していない（理由は表の下） | winget の外す操作 |

- **② と ④ を 14:19 より後に読み直していない理由**: 次のタスク（3.5）の、機械の全員向け（`--scope machine`）の入れ方が、14:19:22 の読みのすぐ後に済んでいたため。今は `areka` が機械の側の PATH から解決し、`winget list` には機械の全員向けの行（ID `ARP\Machine\X64\Areka.Areka.Portable__DefaultSource`）が出るので、今読むと、利用者の側の残りと機械の側の物の区別がつかない。14:22:20 に読んだ機械の側の PATH の登録は 19 項目で、`%ProgramFiles%\WinGet\Packages\Areka.Areka.Portable__DefaultSource` の項目が 1 件在った（入れる前は 18 項目・0 件）。だから ② と ④ の根拠は、`--purge` で外した後・機械の全員向けに入れる前の 14:09 の読み（`logs\3.4-after-purge-*`）。
- ② の補い（利用者の側だけを見るので、機械の全員向けの物に左右されない・14:22:20）: 利用者の PATH の 12 項目のうち、フォルダとして在る 11 か所の中に、名前が `areka.` で始まるファイルは **0 件**。同じ見方で `actionlint.` で始まるファイルは 1 件見つかった（＝在る物は見つかる）。14:09 の読みの後に利用者の側で変わったのは、フォルダが消えたことと PATH の項目が 1 件減ったことだけで、どちらも `areka` が解決する向きの変化ではない。
- あわせて読んだこと（14:22:20）: `HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall` の下で名前に `Areka` を含むキーは 0。
- 14:22:20 の読みのスクリプトは `target\winget-check\3.4-final-user.ps1`、出た文は `logs\3.4-final-user.txt`、読んだ PATH の文字列は `logs\3.4-final-user-path.raw.txt`。読むだけで、winget のコマンドは 1 つも打っていない。

#### 1. 開発者への報告（要件 4.6）

上げ直しと外し方の結果（どちらも消えた物は 6＝後から入れたゴースト 1・後から入れたバルーン 3・ゴーストの記憶 2。areka の記憶は残った）は、14:05 ごろ、作業を進めている AI（親のセッション）がチャットで開発者へ報告した。この節を書いた AI が自分で報告したのではない。

#### 2. 足した注意書き（要件 1.10）

実測で消えた物は、設計の見込みどおりだった（`ghost` と `balloon` の 2 つのフォルダが、後から入れた物ごと消えた。上げ直しでも、外すときでも同じ。areka の記憶 `profile\` は残ったので、名指ししない）。だから、設計の「注意書き」の初稿の 2 つの文を、1 字も変えずに使った。

| ファイル（`dist/winget/0.0.2/`） | 足した行（欄は `InstallationNotes`。値は下の文） | 長さ |
|---|---|---|
| `Areka.Areka.Portable.locale.en-US.yaml` | `areka keeps the ghosts and balloons you add, and their saved data, inside its install folder (next to areka.exe). "winget upgrade" and "winget uninstall" delete the "ghost" and "balloon" folders together with everything you added. Copy these two folders to another place before you upgrade or uninstall.` | 303 字（303 バイト） |
| `Areka.Areka.Portable.locale.ja-JP.yaml` | `areka は、後から入れたゴースト・バルーンとその記憶を、インストール先のフォルダ（areka.exe と同じ場所）の中に置きます。winget upgrade と winget uninstall は、ghost フォルダと balloon フォルダを、後から入れた物ごと消します。上げ直す前と外す前に、この 2 つのフォルダを別の場所へ写しておいてください。` | 182 字（UTF-8 で 412 バイト） |

- 置いた所: 英語は `ReleaseNotesUrl` の行の次、日本語は `Tags` の並びの次（どちらも `ManifestType` の行の前）。値は 1 行で書いた（折り返しなし・引用符で囲まない形）。
- 長さの上限: 書式 1.12.0 の `InstallationNotes` は 1 字以上・10000 字以下（winget-cli のリポジトリの `schemas/JSON/manifests/v1.12.0/` に在る既定のロケールと追加のロケールの書式の、`InstallationNotes` の `minLength`・`maxLength` を `gh` で読んだ）。303 字と 182 字は、どちらも内に収まる。
- `git diff --numstat`: 2 ファイルとも、足した行 1・消した行 0。残りの 2 ファイル（version・installer）は変更 0。足した後も BOM なし・行末は LF のまま（CR は 0）。
- YAML として読めること: PyYAML（6.0.3）で 2 ファイルを読み、取り出した `InstallationNotes` の値が、設計書の初稿の文（設計書のファイルから取り出した物）と 1 字も違わないことを比べた（2 つとも一致）。末尾の 1 字だけを変えた文とは一致しなかった（＝この比べ方は、1 字の違いを見分ける）。`InstallationNotes:` で始まる行は、どちらのファイルにも 1 行だけ。スクリプトは `target\winget-check\3.4-notes-check.py`、出た文は `target\winget-check\logs\3.4-notes-check.txt`。

#### 3. `winget validate`（要件 2.2）

`winget validate --manifest dist\winget\0.0.2 --disable-interactivity` を 14:06:19 に打ち、`マニフェストの検証は成功しました。` の 1 行と終了コード 0 が返った（標準エラー 0 バイト。上の「雛形の検査」の「`winget validate`」にも書いた）。

#### 4. 入れ直した（要件 3.1 のやり直し）

- 入れる直前に読んだこと（14:07:32〜14:07:35）: `<入れ先>` は、外し方の実測の後のまま（1 ファイル・58 バイト・下の階層のフォルダ 3）。利用者の側の PATH の登録は 13 項目で、入れ先のフォルダの項目が 1 件（外し方の実測の後に読んだ文字列と同じ）。`winget list --name areka --disable-interactivity` の areka の行は 0。`winget settings export` の `adminSettings.LocalManifestFiles` は `true`。打った端末は管理者の権限ではない（管理者の役割を持つかを読んで `False`）。`<入れ先>` から動いている `areka.exe`・`shiori-host32-helper.exe` は 0。
- コマンド（ワークツリーの根で・普段の権限で。`--accept-*`・`--scope`・`--force`・`--ignore-security-hash`・上書きの指定は付けていない。スクリプトは `target\winget-check\3.4-install.ps1`）:

  ```powershell
  winget install --manifest dist\winget\0.0.2 --disable-interactivity
  ```

- 始めた時刻 14:07:35.244・終わった時刻 14:07:47.273（12.0 秒）・**終了コード 0**・標準エラーは 0 バイト。問い（同意・管理者への切り替え）は 1 つも出なかった。打ったのはこの 1 回だけ。
- winget が出した文（標準出力の全部＝11 行・1157 バイト。省いた行・まとめた行は無い。UTF-8 として読んだ）:

  ```text
  見つかりました areka (portable) [Areka.Areka.Portable] バージョン 0.0.2
  このアプリケーションは所有者からライセンス供与されます。
  Microsoft はサードパーティのパッケージに対して責任を負わず、ライセンスも付与しません。
  ダウンロード中 https://github.com/ekicyou/areka/releases/download/v0.0.2/areka-0.0.2-x64.zip
  インストーラーハッシュが正常に検証されました
  アーカイブを展開しています...
  アーカイブが正常に展開されました
  パッケージのインストールを開始しています...
  コマンド ライン エイリアスが追加されました: "areka"
  インストールが完了しました
  メモ: areka は、後から入れたゴースト・バルーンとその記憶を、インストール先のフォルダ(areka.exe と同じ場所)の中に置きます。winget upgrade と winget uninstall は、ghost フォルダと balloon フォルダを、後から入れた物ごと消します。上げ直す前と外す前に、この 2 つのフォルダを別の場所へ写しておいてください。
  ```

- 判定（要件 3.1）: ハッシュの検証が通った行（`インストーラーハッシュが正常に検証されました`）が 1 行・インストール完了の行（`インストールが完了しました`）が 1 行・終了コード 0。**合格**。
- **注意書きは出た**。インストール完了の行の次＝最後の行に、頭へ `メモ: ` を付けた形で 1 行。出たのは**日本語の側**（開発機の言語に合う側）で、英語の文は出た文の中に 0 件。`--disable-interactivity` を付けていても出た。
- 出た文と雛形の文の違いは **2 字**: 雛形の全角の括弧 `（`・`）` が、半角の `(`・`)` になって出た（上の文の `フォルダ(areka.exe と同じ場所)`）。残りの 180 字は同じ。winget は、マニフェストから読んだ文の字の形をそろえてから扱う（v1.29.380 のソースの `AppInstallerStrings.h` で、文字列の型 `NormalizedString` が NFKC のそろえ方を使う定義。雛形の文を NFKC でそろえた形は、出た文の中に 1 件見つかった）。意味は変わらないので、雛形の文は設計の文のまま（直していない）。比べたスクリプトは `target\winget-check\3.4-notes-shown.py`、出た文は `logs\3.4-notes-shown.txt`。
- 初めて入れたとき（上の「入れて起動する」の 11 行）との違い: `パス環境変数が変更されました; …` の行が無く（PATH の項目が前から在ったため）、最後の `メモ: …` の行が増えた。
- 残っていた物を、入れ直しがどう扱ったか（入れる前の一覧 `logs\3.4-before-install.tsv` と、入れた後の一覧 `logs\3.4-after-install.tsv` を見比べた）:

  | 残っていた物 | 入れ直しの後 |
  |---|---|
  | areka の記憶 `profile\areka\sylphya.toml`（58 バイト） | **そのまま残った**（ハッシュ `3E16E14A…325EDDF6` も、書いた時刻も、作った時刻も前と同じ） |
  | `.nar-work\`（空）・`profile\`・`profile\areka\` | そのまま（NTFS がフォルダに振る番号も前と同じ） |
  | 入れ先のフォルダそのもの | 同じフォルダ（番号も、作った時刻 13:01:48 も前と同じ。作り直されていない） |
  | 利用者の PATH の、入れ先のフォルダの項目 | **1 件のまま**（2 件にならなかった。登録の文字列は、入れる前と 1 字も違わない＝13 項目・616 字） |

- 入れた後の `<入れ先>` は 151 ファイル・16789175 バイト・下の階層のフォルダ 25・隠しファイル 1（直下のファイル 7・`balloon\` 50・`ghost\` 93・`profile\` 1）。上げ直しの直後の一覧（`logs\3.2-after-upgrade.tsv`）と比べると、違うのは winget の控え `Areka.Areka.Portable__DefaultSource.db` のハッシュだけ。

#### 5. `--purge` を付けて外した

- 打つスクリプト（`target\winget-check\3.4-purge.ps1`）は、打つ直前に `winget list --name areka --disable-interactivity` を読み（14:09:07・終了コード 0）、areka の行が 1 行だけで、その行から引いた ID が `ARP\User\X64\Areka.Areka.Portable__DefaultSource`・版が `0.0.2` であることを確かめてから、その ID を渡した。
- `<入れ先>` から動いている `areka.exe`・`shiori-host32-helper.exe` は、14:09:09.060 と 14:09:13.077 の 2 回（4 秒あけて）とも 0（この名前のプロセス自体が 0）。
- コマンド（ワークツリーの根で・普段の権限で＝管理者の役割を持つかを読んで `False`。`--force`・`--accept-*` は付けていない）:

  ```powershell
  winget uninstall --id "ARP\User\X64\Areka.Areka.Portable__DefaultSource" --exact --purge --disable-interactivity
  ```

- 始めた時刻 14:09:13.091・終わった時刻 14:09:14.515（1.4 秒）・**終了コード 0**・標準エラーは 0 バイト。問いは 1 つも出なかった。打ったのはこの 1 回だけ。
- winget が出した文（標準出力の全部＝4 行・305 バイト。省いた行・まとめた行は無い。UTF-8 として読んだ）:

  ```text
  ソースの検索中にエラーが発生しました;結果は含まれません: msstore
  見つかりました areka (portable) [ARP\User\X64\Areka.Areka.Portable__DefaultSource]
  パッケージのアンインストールを開始しています...
  正常にアンインストールされました
  ```

- 外し方の実測のとき（5 行）との違い: `ファイルはインストール ディレクトリに残ります: …` の行が無い。入れ先を丸ごと消した、と告げる行も無い。

#### 6. `--purge` で外した直後の 4 項目（要件 3.6。この時点では 2 つが 0 でなかった。②と④は、この読みが根拠）

読んだ時刻は 14:09:14〜14:09:20（スクリプトは `target\winget-check\3.4-facts.ps1`）。右端の列は、同じスクリプトを、入れた後・外す前（14:08:08〜14:08:16）に回したときの値＝この見方が、在る物を見つけられることの確かめ。

| 項目 | 見方 | 外した後に見た値 | 0 か | 同じ見方で、入れた後・外す前に見た値 |
|---|---|---|---|---|
| ① 入れ先のフォルダ | `Test-Path` で `<入れ先>`。あわせて `%LOCALAPPDATA%\Microsoft\WinGet\Packages\` の下で名前が `Areka` で始まる物と、`…\WinGet\Links\` の下で名前が `areka` で始まる物を数えた | **在る**（中身は 1 ファイル・58 バイト・下の階層のフォルダ 3）。`Areka` で始まる物は 1 件（このフォルダ）。`Links\` の `areka` で始まる物は 0 件 | **0 でない** | 在る・1 件・`Links\` は 0 件 |
| ② 新しい端末での `areka` の解決 | 登録（機械の側;利用者の側）から PATH を組み直した 31 項目にした新しい `pwsh -NoProfile -NonInteractive` のプロセスで、`Get-Command areka -All` と `where.exe areka` | `Get-Command` は **0 件**。`where.exe` は終了コード 1（見つからない） | 0 | `Get-Command` は 1 件（`<入れ先>\areka.exe`）・`where.exe` は終了コード 0 で同じパス。同じプロセスで `Get-Command winget -All` は 1 件・`pwsh` は 2 件（外した後も同じ＝組み直した PATH は効いている） |
| ③ 利用者の PATH に足された項目 | レジストリ `HKCU:\Environment` の `Path` を、環境変数を開かない形で読み、`;` で分けて、入れ先のフォルダと同じ項目を数えた。あわせて、変える前の機械の状態の文字列（入れる前の 12 項目。`logs\2.2-pre-user-path.txt`）と比べた | **1 件残っている**（13 項目のうちの 13 番目。値の種類は `ExpandString`）。変える前の文字列（12 項目・524 字・末尾に `;` なし）とは**同じでない**: 今は 13 項目・616 字で、増えた項目が 1（入れ先のフォルダ）・消えた項目が 0・末尾に `;` が 1 つ。入れる前から在った 12 項目の並びと字面は変わっていない | **0 でない** | 1 件（同じ文字列）。変える前の文字列には 0 件 |
| ④ `winget list` の areka の行 | `winget list --name areka --disable-interactivity` と `winget list --id Areka --disable-interactivity` | どちらも **0 行**（`入力条件に一致するインストール済みのパッケージが見つかりませんでした。`・終了コード -1978335212＝`0x8A150014`・標準エラー 0 バイト） | 0 | どちらも 1 行（`areka (portable)`・`ARP\User\X64\Areka.Areka.Portable__DefaultSource`・`0.0.2`・終了コード 0）。同じ形の `winget list --name PowerShell` は、外す前も後も 2 行・終了コード 0 |

**まとめ（14:09 の時点）: 0 は 2 つ（②・④）。0 でない物は 2 つ（①・③）。winget の外す操作だけでは、利用者の側の後片付けは済まなかった。**（残った 2 つを片付けた後の値は、下の 8 と、この節の頭の表）

この時点で残っていた物（これで全部）:

1. フォルダ `<入れ先>`。中身は、外し方の実測の後・入れ直す前とまったく同じ（一覧のファイルが 1 バイトも違わない。`logs\3.4-after-purge.tsv`・`3.4-after-purge.dirs.tsv`）:

   | 残った物（入れ先のフォルダから見て） | 種類 | 大きさ |
   |---|---|---|
   | `.nar-work\` | フォルダ（空） | - |
   | `profile\` | フォルダ | - |
   | `profile\areka\` | フォルダ | - |
   | `profile\areka\sylphya.toml` | ファイル（areka の記憶。SHA256 `3E16E14A…325EDDF6`） | 58 バイト |

2. 利用者の PATH の登録（`HKCU:\Environment` の `Path`）の 13 番目の項目 1 件＝上のフォルダの絶対パス（変数を開いた形）と、その前後の `;`。

あわせて読んだこと: 機械の側の PATH の登録は 18 項目で、変える前と同じ文字列（`areka` の字を含む項目は 0）。`HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall` の下で名前に `Areka` を含むキーは 0（入れた後・外す前は 1）。`<入れ先>` から動いているプロセスは 0（14:09:20）。

#### 7. `--purge` を付けても残った理由（winget 自身の記録とソースで読んだ）

winget 自身の記録（`%LOCALAPPDATA%\Packages\Microsoft.DesktopAppInstaller_8wekyb3d8bbwe\LocalState\DiagOutputDir\` の、その回のファイル。写しは `target\winget-check\logs\3.4-winget-diag-*.txt`）:

- 入れ直しの回（14:07）には、`Created target install directory`（入れ先のフォルダを作った）の行が**無い**。初めて入れた回（13:01）には在る。＝入れ先のフォルダは前から在ったので、winget は「自分が作ったフォルダ」と覚えなかった。
- 入れ直しの回には `Portable target directory already exists in PATH registry`（PATH の登録にもう在る）の行が在る。
- 外した回（14:09）には `Install directory is not empty`（入れ先のフォルダが空でない）の行が在り、PATH の項目を消していない。`Purged install location directory`（入れ先を丸ごと消した）の行は**無い**。

ソース（winget v1.29.380 の `src/AppInstallerCLICore/PortableInstaller.cpp`）:

- 入れ先のフォルダを片付ける関数 `RemoveInstallDirectory` は、「自分が作った」と覚えているフォルダだけを扱う。`--purge` で丸ごと消すのも、この関数の中。
- PATH の項目を消す関数 `RemoveFromPathVariable` は、そのフォルダが在って空でないときは、`Install directory is not empty` と記録に書くだけで、何もしない。winget がこの関数を呼ぶのは、入れた物を 1 つずつ消している途中（`areka` の名前の登録を片付ける番）で、入れ先のフォルダを片付ける関数より前。外した回（14:09）の記録でも、`Deleting portable exe at: …`・`Removing directory at …` の行が 8 つ続いた後の最後に、この行が出ている。

かみ砕くと、残った 2 つの理由は別々:

- **PATH の項目**: winget が PATH の項目を消そうとする時点で、入れ先のフォルダが空でなかったから。その時点のフォルダには、少なくとも winget 自身の控え（`Areka.Areka.Portable__DefaultSource.db`。入れた物を消し終えた後に消される）が在る。だから、`--purge` を付けても付けなくても、areka が記憶を書いていてもいなくても、この項目は残る見込み（ソースの読み）。実際に、外し方の実測のとき（13:55）の記録にも、上げ直しの回（13:42）の記録にも、同じ `Install directory is not empty` の行が在り、項目は 13:01 に足されてから、winget の操作では 1 度も消えていない。
- **入れ先のフォルダ**: 外し方の実測の残り（入れ先のフォルダ）の上へ入れ直したので、winget が「自分が作ったフォルダ」と覚えておらず、`--purge` の丸ごと消す処理を通らなかったから。`--purge` を付けない外し方（13:55）では、areka の記憶 `profile\areka\` と空の `.nar-work\` が残っていてフォルダが空でなかったので、winget は `Unable to remove install directory as there are remaining files in: …` と記録に書いて、フォルダを残した。

**測っていないこと**: 何も残っていない機械へ入れて `--purge` で外したときに、4 項目がすべて 0 になるか。上のソースの読みでは、フォルダは消える見込みだが、PATH の項目は残る見込み。

#### 8. 残った 2 つの片付けと、片付けた後の読み（14:17〜14:22）

設計の「後片付け」の決まり（外した後に入れ先のフォルダが残ったら、中身の一覧を記録してから AI が消す）と、「winget と OS の設定は AI が変えない」の決まりに沿って、フォルダは AI が、PATH は開発者が片付けた。下の 1〜3 は、作業を進めている AI（親のセッション）が行ったこと・読んだことで、この記録を書いた AI はそれを聞いて書いている（その回の出力のファイルは無い）。4 は、この記録を書いた AI が自分で読んだ。

1. **入れ先のフォルダを消した（14:17:20.330・作業を進めている AI）**。消す直前に取った中身の一覧は次の 4 つで全部（`--purge` の直後の一覧＝上の 6 と同じ）。`<入れ先>` から動いているプロセスは 0・`winget list` の areka の行は 0、を見てから消した。消した後、`<入れ先>` は無く、`%LOCALAPPDATA%\Microsoft\WinGet\Packages\` の下で名前が `Areka` で始まる物は 0 件（同じフォルダに、ほかのパッケージの物が 2 つ在る＝在る物は見つかる）。

   | 消した物（入れ先のフォルダから見て） | 種類 | 大きさ |
   |---|---|---|
   | `.nar-work\` | フォルダ（空） | - |
   | `profile\` | フォルダ | - |
   | `profile\areka\` | フォルダ | - |
   | `profile\areka\sylphya.toml` | ファイル（areka の記憶） | 58 バイト |

2. **PATH の直し方を、変えずに試算した（作業を進めている AI）**。今の文字列から入れ先のフォルダの項目 1 件を除くと、12 項目・524 字になり、入れる前に取っておいた文字列（`logs\2.2-pre-user-path.txt`）と、大文字と小文字を区別して同じになることを確かめてから、開発者へコマンドを示した。
3. **開発者が PATH の項目を消した（14:18 ごろ・開発者の手で・普段の端末で）**。打ったコマンドは次の 1 行（AI が示した物そのまま。入れ先のフォルダの項目と、空の項目＝末尾の `;` の分だけを除いて書き戻す。値の種類は `ExpandString` のまま）:

   ```powershell
   $k='HKCU:\Environment'; $o=(Get-Item $k).GetValue('Path','','DoNotExpandEnvironmentNames'); $n=(($o -split ';') | Where-Object { $_ -and $_ -notlike '*\WinGet\Packages\Areka.Areka.Portable__DefaultSource' }) -join ';'; Set-ItemProperty $k -Name Path -Value $n -Type ExpandString
   ```

   作業を進めている AI が 14:19:22 に読んだ値: 利用者の PATH は 12 項目・524 字・`areka` の項目 0 件・値の種類 `ExpandString`・入れる前の文字列と同じ＝`True`。`<入れ先>` は無い。
4. **片付けた後の読み（14:22:20・この記録を書いた AI・読むだけ）**。① と ③ を読み直して、どちらも 0（値と、見方が当たりを出せることの確かめは、この節の頭の表）。② と ④ は読み直していない（理由も頭の表の下）。出た文は `logs\3.4-final-user.txt`。

＝利用者の側の後片付けの 4 項目は、すべて 0。利用者の PATH は、winget で入れる前と 1 字も違わない。

### 機械の側（タスク 3.5・要件 4.8・3.6）

結果: **機械の側の後片付けの 4 項目は、すべて 0**（この節を書いた AI が 14:49:28〜14:49:45 に読んだ。読むだけ）。ただし、利用者の側と同じく、**winget の外す操作だけでは 0 にならなかった**: 開発者が管理者の端末で `--purge` を付けて外した後（14:41:08）、① 入れ先のフォルダ・② `areka` の解決・④ `winget list` の行は 0 になったが、**③ 機械の PATH の項目 1 件は残っていた**。残った 1 件は、開発者が手で消した（14:43:46 より後・14:46:58 より前）。経緯は上の「機械の全員向けの実測」の 8。時刻はどれも 2026-10-10（+09:00）。`<入れ先>` は `C:\Program Files\WinGet\Packages\Areka.Areka.Portable__DefaultSource`。

4 項目の今の値（どの読みで 0 と言うか・誰が 0 にしたか）:

| 項目 | 結果 | いつ・どう読んだか | 見方が当たりを出せること | 0 にしたのは誰か |
|---|---|---|---|---|
| ① 入れ先のフォルダ | **0**（`<入れ先>` は無い。`C:\Program Files\WinGet\Packages\` の下は 0 件で、名前が `Areka` で始まる物も 0 件。`C:\Program Files\WinGet\Links\` の下で `areka` で始まる物も 0 件） | 14:49:28 に `Test-Path` と一覧（隠しファイルも含む）で読んだ | 同じ `Test-Path` は、親の `C:\Program Files\WinGet` と `C:\Program Files` に `True` を返した。同じ形の名前の絞り込み（`Win*`）は `C:\Program Files` の下で 10 件を返した | winget の外す操作（開発者が管理者の端末で `--purge` を付けて打った。winget 自身の記録に `Purged install location directory` の行） |
| ② 新しい端末での `areka` の解決 | **0**（`Get-Command areka -All` は 0 件・`where.exe areka` は終了コード 1 で標準出力 0 字。`shiori-host32-helper` も 0 件） | 14:49:33 に、登録（機械の側;利用者の側）から PATH を組み直した 30 項目（機械の側 18＋利用者の側 12。`areka` の字を含む項目は 0 件）にした新しい `pwsh -NoProfile -NonInteractive` のプロセスで読んだ。解決を見ただけで、`areka` という名前の物は何も起こしていない | 同じプロセスの中で `Get-Command winget -All` は 1 件・`pwsh` は 2 件・`where.exe winget` は終了コード 0 で 1 行。組み直した PATH のうち、フォルダとして在る 29 か所の中に、名前が `areka.` で始まるファイルは 0 件・`winget.` で始まるファイルは 1 件 | winget の外す操作（`areka.exe` が消えた） |
| ③ 機械の PATH に足された項目 | **0**（`HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager\Environment` の `Path` は **18 項目・646 字**・値の種類 `ExpandString`・末尾の `;` は 0。`<入れ先>` と同じ項目は 0 件・`WinGet\Packages\Areka` を含む項目も 0 件・`areka` の字を含む項目も 0 件。どの入れる操作よりも前に取っておいた文字列 `logs\2.2-pre-machine-path.txt`（18 項目・646 字）と、大文字と小文字を区別して比べて**同じ**） | 14:49:29 に、環境変数を開かない形で読んだ | 同じ数え方を、機械の全員向けに入れてあった間に組み直した PATH（`logs\3.5-bounded1-fresh-path.txt`・31 項目）に当てると、`<入れ先>` と同じ項目が 1 件。今の文字列の末尾の 1 字を変えた物は、取っておいた文字列と同じでない、と出る（＝1 字の違いも見分ける） | winget の外す操作では**消えなかった**。**開発者が手で**消した（14:43:46 より後・14:46:58 より前・管理者の端末で。上の「機械の全員向けの実測」の 8 のコマンド 3。AI は PATH を変えていない） |
| ④ `winget list` の areka の行 | **0**（`winget list --name areka --disable-interactivity` でも `winget list --id Areka --disable-interactivity` でも 0 行。どちらも終了コード -1978335212＝`0x8A150014`（見つからなかったときの値）・標準エラー 0 バイト） | 14:49:35〜14:49:40（`--accept-*` は付けていない。問いは出なかった） | 同じ形の `winget list --name PowerShell --disable-interactivity` は 2 行を返し、終了コード 0 | winget の外す操作（winget 自身の記録に `PortableARPEntry deleted.` の行） |

- 読んだスクリプトは `target\winget-check\3.5-final-machine.ps1`、出た文は `logs\3.5-final-machine.txt`（読んだ PATH の文字列は `logs\3.5-final-machine-path.raw.txt`・`3.5-final-user-path.raw.txt`・`3.5-final-fresh-path.txt`、`winget list` の文は `logs\3.5-final-list-*.stdout.txt`）。打った端末は管理者の権限ではない（管理者の役割を持つかを読んで `False`）。
- 0 になるまでの経緯（起きた順）: 14:41:08 の外す操作の直後は **① ② ④ が 0・③ は 0 でない**（機械の PATH は 19 項目のままで、もう無い `<入れ先>` を指す項目が 1 件。作業を進めている AI の 14:41:46 と 14:43:46 の読み）。③ が 0 になったのは、開発者が手で消した後（作業を進めている AI の 14:46:58 の読みと、この節を書いた AI の 14:49:29 の読み）。
- あわせて読んだこと（14:49:28〜14:49:45）:
  - アンインストールの登録（`HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall`・その `WOW6432Node` の側・`HKCU` の側）で、名前に `Areka` を含むキーは 0（それぞれ 148・311・8 のキーの中で）。
  - 名前が `areka`・`shiori-host32-helper` のプロセスは 0（同じ見方で `pwsh` は 1 つ以上）。
  - `%LOCALAPPDATA%\VirtualStore` は在るが、中身は 0 件のまま。
  - 利用者の側も変わっていない: 利用者向けの入れ先 `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource` は無い（同じ `Packages\` の 2 件の中に `Areka` で始まる物は 0 件・`Links\` に `areka` で始まる物は 0 件）。利用者の PATH は 12 項目・524 字で、入れる前の文字列と同じ。
  - 利用者の側の後片付けで「14:19 より後には読み直していない」と書いた ② と ④（上の「利用者の側」の表）も、機械の全員向けの物が無くなった今の読みでは 0（上の表の ② と ④ は、機械の側と利用者の側を合わせた PATH と、`winget list` の全部を見ている）。
- **4 項目の外に残った物（事実として書く。消していない）**: winget が機械の全員向けの入れる操作のときに作ったフォルダ `C:\Program Files\WinGet\` と、その下の `Links\`・`Packages\`。3 つとも**空のまま残っている**（14:49:28 に、隠しファイルも含めて下の階層まで読んだ: フォルダ 2 つ・ファイル 0 件）。

  | 残っているフォルダ | 作られた時刻 | 最後に書かれた時刻 | 中身 |
  |---|---|---|---|
  | `C:\Program Files\WinGet\` | 14:18:56 | 14:19:09 | 下の 2 つのフォルダだけ |
  | `C:\Program Files\WinGet\Links\` | 14:18:56 | 14:18:56 | 空 |
  | `C:\Program Files\WinGet\Packages\` | 14:19:09 | 14:41:08 | 空 |

  - 作られた時刻は、機械の全員向けに入れた回（14:18:56〜14:19:10）の中。設定を変える前（11:44〜11:45）の読みでも、`%ProgramFiles%\WinGet\Packages\` は「フォルダ自体がまだ無い」だった（上の「変える前の機械の状態」）。＝この 3 つは、今回の入れる操作が作った物。`Packages\` の最後に書かれた時刻は、外した回の終わり（14:41:08）と合う。
  - winget の `--purge` は、パッケージの入れ先（`…\Packages\Areka.Areka.Portable__DefaultSource`）までを消し、その親のフォルダは消さない。PATH にも、`winget list` にも、`areka` の解決にも関わらない空のフォルダで、消すには管理者の権限が要る。作業を進めている AI が開発者へ伝えたのは `Packages\` が空で残っていることで、開発者はそのままにした（親の `WinGet\` と隣の `Links\` も今回できた物であることは、この節を書いた AI が 14:49 に作られた時刻を読んで分かった）。AI は `C:\Program Files\WinGet\` に触っていない。
- 機械の全員向けでは、**何も残っていない所へ入れて `--purge` で外す形**になった（利用者の側では測れなかった形。上の「利用者の側」の 7 の「測っていないこと」）。その形で見えたのは、ソースの読みの見込みどおり「フォルダは消える・PATH の項目は残る」。ただし、これは機械の全員向け（管理者の端末）での 1 回で、利用者向けの入れ方で同じ形を測ったのではない。

＝機械の側の後片付けの 4 項目は、すべて 0。機械の PATH は、winget で入れる前と 1 字も違わない。

## 置き場（要件 3.5）

確かめのために自分が作る物は、すべてワークツリーの `target\winget-check\` の下に置く。`target` はリポジトリの `.gitignore` の 1 行目で追跡の外になっている。

| 置いた物 | 場所 | 置いたタスク |
|---|---|---|
| Release `v0.0.2` から取ってきた zip 2 つと `.sha256` 2 つ | `target\winget-check\release\` | 1.1 |
| `winget validate` が出した文（雛形の分と、赤を出せることを見た分） | `target\winget-check\logs\validate-1.2*.txt` | 1.2 |
| 赤を出せることを見るための、わざと壊した雛形の写し 4 ファイル | `target\winget-check\validate-negative\` | 1.2 |
| 設定を変える前の機械の状態を読んだときの出力（`winget list` 3 回分・新しいプロセスでの `areka` の解決・`where.exe`） | `target\winget-check\logs\pre-2.1-*.txt` | 2.1 |
| オンを確かめたときの `winget settings export` の出力 | `target\winget-check\logs\on-2.1-settings-export.*.txt` | 2.1 |
| 入れる直前の読み（PATH の登録 2 つ・`winget settings export`）・`winget install` が出した文・`winget list` 2 回分・入れた後の PATH の登録 2 つ・組み直した PATH | `target\winget-check\logs\2.2-*.txt`（`2.2-pre-*`・`2.2-install.*`・`2.2-list-*`・`2.2-post-*`・`2.2-fresh-path.txt`・`2.2-fresh-shell.*`） | 2.2 |
| 起動の記録（標準出力・標準エラー）と、起こしたプロセスの結果（解決先・時刻・終了コード） | `target\winget-check\logs\2.2-run.stdout.log`・`2.2-run.stderr.log`・`2.2-run.result.json` | 2.2 |
| 入れ先のフォルダの直下の一覧（入れた直後・1 回目の起動の後） | `target\winget-check\logs\2.2-installdir-after-install.txt`・`2.2-installdir-after-run.txt` | 2.2 |
| 新しいプロセスで `areka` を起こすスクリプト 2 つ | `target\winget-check\2.2-launch.ps1`・`target\winget-check\2.2-inner.ps1` | 2.2 |
| 起動のときの作業フォルダ（中は空） | `target\winget-check\run\` | 2.2 |
| winget が入れた areka（置いたのは winget。winget 自身の入れ先なので `target\` の外。この後のタスクが続けて使い、後片付けで外す） | `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource\` | 2.2 |
| 検体の `.nar` の写し 2 つ（`claudia.nar`・`emo2-kakukaku-wplimit.nar`。元は `vendors\sample_ghost\` で、元には触っていない） | `target\winget-check\nar\` | 3.1 |
| areka を有界でなく起こすスクリプト 2 つ・MCP の口へ 1 本送るスクリプト・入れ先の一覧を取るスクリプト | `target\winget-check\3.1-launch.ps1`・`3.1-inner.ps1`・`3.1-mcp.ps1`・`3.1-list.ps1` | 3.1 |
| 1 回目の起動の記録（標準出力・標準エラー）・起こしたプロセスの結果（解決先・時刻・終了コード）・組み直した PATH | `target\winget-check\logs\3.1-run1.stdout.log`・`3.1-run1.stderr.log`・`3.1-run1.result.json`・`3.1-run1-fresh-path.txt`・`3.1-run1-fresh-shell.*` | 3.1 |
| MCP の口へ送った文と答えの全文・`tools/list` の答え | `target\winget-check\logs\3.1-mcp.log`・`3.1-tools-list.json` | 3.1 |
| 入れ先のフォルダの一覧（起こす前・閉じる要求の直前・終わった後。相対パスと大きさ・隠しファイルも含む） | `target\winget-check\logs\3.1-installdir-before-launch.txt`・`3.1-installdir-run1-before-close.txt`・`3.1-installdir-run1-after-end.txt` | 3.1 |
| `AREKA_NO_ALERT` を置かずに起こすスクリプト 2 つ（3 回目に使った） | `target\winget-check\3.1-launch-alert.ps1`・`3.1-inner-alert.ps1` | 3.1 |
| 2 回目・3 回目の起動の記録（標準出力・標準エラー）・起こしたプロセスの結果・組み直した PATH | `target\winget-check\logs\3.1-run2.*`・`3.1-run2-fresh-*`・`3.1-run3.*`・`3.1-run3-fresh-*` | 3.1 |
| 状態を作った後の入れ先のフォルダの一覧（写しの前・写しの後）と、写しの一覧 | `target\winget-check\logs\3.1-installdir-run3-after-end.txt`・`3.1-installdir-run3-before-snapshot-check.txt`・`3.1-installdir-snapshot.txt` | 3.1 |
| 状態の写し（入れ先のフォルダの丸ごと。358 ファイル・21846581 バイト・隠しファイルと空のフォルダも含む）と、写したときの robocopy の記録 | `target\winget-check\state-snapshot\`・`target\winget-check\logs\3.1-snapshot-robocopy.log` | 3.1 |
| 同梱のファイルへ付けた目印 2 つ（`readme.txt` の末尾の 1 行と、`winget-check-marker.txt`。winget の入れ先の中なので `target\` の外。上げ直しで 2 つとも消えた） | `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource\ghost\emo2\` | 3.2 |
| 版だけを `0.0.2.1` に上げた確かめ専用のマニフェスト 4 ファイルと、`winget validate` が出した文 | `target\winget-check\upgrade-0.0.2.1\`・`target\winget-check\logs\3.2-validate.*.txt` | 3.2 |
| 一覧を取る・動いているプロセスを読む・winget を 1 回打つ・一覧を見比べる、のスクリプト 4 つ | `target\winget-check\3.2-list.ps1`・`3.2-procs.ps1`・`3.2-fire.ps1`・`3.2-diff.ps1` | 3.2 |
| 上げ直しの前の読み（入れ先の一覧・フォルダの一覧・PATH の登録 2 つ・`winget list`・`winget settings export`） | `target\winget-check\logs\3.2-before.tsv`・`3.2-before.dirs.tsv`・`3.2-before-*.txt` | 3.2 |
| `winget upgrade` が出した文（標準出力・標準エラー）と、時刻・終了コード | `target\winget-check\logs\3.2-upgrade.stdout.txt`・`3.2-upgrade.stderr.txt`・`3.2-upgrade.result.json` | 3.2 |
| 上げ直しの後（起動より先）の読み（入れ先の一覧・フォルダの一覧・前との見比べ・PATH の登録・`winget list` 2 回分） | `target\winget-check\logs\3.2-after-upgrade.tsv`・`3.2-after-upgrade.dirs.tsv`・`3.2-diff-after-upgrade.txt`・`3.2-after-user-path.raw.txt`・`3.2-after-list-*.txt` | 3.2 |
| 上げ直しの後に有界で起こすスクリプト 2 つと、MCP の口へ 1 本送るスクリプト | `target\winget-check\3.2-run.ps1`・`3.2-inner.ps1`・`3.2-mcp.ps1` | 3.2 |
| 上げ直しの後の起動の記録（標準出力・標準エラー）・起こしたプロセスの結果・組み直した PATH・MCP の口へ送った文と答え | `target\winget-check\logs\3.2-run.stdout.log`・`3.2-run.stderr.log`・`3.2-run.result.json`・`3.2-run-fresh-*`・`3.2-mcp.log` | 3.2 |
| 起動の後の入れ先の一覧と、上げ直しの直後との見比べ（判定には使わない） | `target\winget-check\logs\3.2-after-run.tsv`・`3.2-after-run.dirs.tsv`・`3.2-diff-after-run.txt` | 3.2 |
| 写しから入れ先へ戻した物（置いた 157 ファイル・上書きした 4 ファイル・作ったフォルダ 18。winget の入れ先の中なので `target\` の外。外したときに、areka の記憶の 1 ファイルを除いて消えた） | `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource\` の `ghost\claudia\`・`balloon\claudia\`・`balloon\claudia_vertical\`・`balloon\emo2-kakukaku-wplimit\`・`ghost\emo2\ghost\master\profile\`・`profile\areka\sylphya.toml` | 3.3 |
| 戻すときに上書きした 4 ファイルの、上書きする前の中身 | `target\winget-check\3.3-set-aside\overwritten\` | 3.3 |
| 写しから戻す・一覧を取る・一覧を見比べる・winget を 1 回打って文を取る・外す・外した後を読む、のスクリプト 6 つ | `target\winget-check\3.3-restore.ps1`・`3.3-list.ps1`・`3.3-diff.ps1`・`3.3-wg.ps1`・`3.3-fire.ps1`・`3.3-facts.ps1` | 3.3 |
| 戻す前に出させた手はずと、戻した記録（確かめの結果を含む） | `target\winget-check\logs\3.3-restore-plan.log`・`3.3-restore-apply.log` | 3.3 |
| 外す前の読み（入れ先の一覧・フォルダの一覧・`winget settings export`・`winget list` 2 回分・利用者の PATH の登録） | `target\winget-check\logs\3.3-before.tsv`・`3.3-before.dirs.tsv`・`3.3-pre-settings-export.*`・`3.3-pre-list-name.*`・`3.3-fire-list-name.*`・`3.3-before-user-path.raw.txt` | 3.3 |
| `winget uninstall` が出した文（標準出力・標準エラー）と、時刻・終了コード | `target\winget-check\logs\3.3-uninstall.stdout.txt`・`3.3-uninstall.stderr.txt`・`3.3-uninstall.result.json` | 3.3 |
| 外した後の読み（入れ先の一覧・フォルダの一覧・前との見比べ・`winget list` 3 回分・PATH の登録 2 つ・組み直した PATH・新しいプロセスでの `areka` の解決） | `target\winget-check\logs\3.3-after.tsv`・`3.3-after.dirs.tsv`・`3.3-diff-after.txt`・`3.3-after-list-*`・`3.3-after-user-path.raw.txt`・`3.3-after-machine-path.raw.txt`・`3.3-after-fresh-path.txt`・`3.3-after-fresh-shell.*` | 3.3 |
| 外した後に残った入れ先のフォルダ（areka の記憶の 1 ファイルと空の `.nar-work\`。winget の入れ先なので `target\` の外。後片付けのタスク 3.4 まで置いておく） | `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource\` | 3.3 |
| 書式 1.12.0 の、既定のロケールと追加のロケールの書式（`InstallationNotes` の長さの上限を読んだ） | `target\winget-check\schema\` | 3.4 |
| 注意書きの行を確かめるスクリプト 2 つ（雛形の値と設計の文の見比べ・winget が出した文と雛形の値の見比べ）と、出た文 | `target\winget-check\3.4-notes-check.py`・`3.4-notes-shown.py`・`target\winget-check\logs\3.4-notes-check.txt`・`3.4-notes-shown.txt` | 3.4 |
| winget を 1 回打って文を取る・一覧を取る・入れ直す・`--purge` で外す・後片付けの 4 項目を読む、のスクリプト 5 つ | `target\winget-check\3.4-wg.ps1`・`3.4-list.ps1`・`3.4-install.ps1`・`3.4-purge.ps1`・`3.4-facts.ps1` | 3.4 |
| 注意書きを足した後の `winget validate` が出した文と、時刻・終了コード | `target\winget-check\logs\3.4-validate.*` | 3.4 |
| 入れ直す前の読み（入れ先の一覧・フォルダの一覧・利用者の PATH の登録・`winget settings export`・`winget list`） | `target\winget-check\logs\3.4-before-install.tsv`・`3.4-before-install.dirs.tsv`・`3.4-before-install-*` | 3.4 |
| `winget install` が出した文（標準出力・標準エラー。注意書きの行を含む）と、時刻・終了コード | `target\winget-check\logs\3.4-install.stdout.txt`・`3.4-install.stderr.txt`・`3.4-install.result.json` | 3.4 |
| 入れた後・外す前の読み（入れ先の一覧・フォルダの一覧・後片付けの 4 項目と同じ見方の結果） | `target\winget-check\logs\3.4-after-install.tsv`・`3.4-after-install.dirs.tsv`・`3.4-after-install-*` | 3.4 |
| `--purge` を付けた `winget uninstall` が出した文（標準出力・標準エラー）と、時刻・終了コード。打つ直前の `winget list` | `target\winget-check\logs\3.4-uninstall-purge.*`・`3.4-fire-list-name.*` | 3.4 |
| 外した後の読み（入れ先の一覧・フォルダの一覧・後片付けの 4 項目＝PATH の登録 2 つ・組み直した PATH・新しいプロセスでの `areka` の解決・`winget list` 3 回分） | `target\winget-check\logs\3.4-after-purge.tsv`・`3.4-after-purge.dirs.tsv`・`3.4-after-purge-*` | 3.4 |
| winget 自身の記録の写し（入れ直しの回・`--purge` で外した回・入れ先のフォルダと PATH の扱いを告げる行の抜き出し。パスの頭は `%LOCALAPPDATA%` に置き換えてある） | `target\winget-check\logs\3.4-winget-diag-install.txt`・`3.4-winget-diag-uninstall-purge.txt`・`3.4-winget-diag-path-lines.txt` | 3.4 |
| 読んだ winget のソースと文書の写し（v1.29.380 の `PortableInstaller.cpp`・`InstallFlow.cpp`・`AppInstallerStrings.h`・`ManifestLocalization.h`・`doc/Settings.md`） | `target\winget-check\review\3.4\` | 3.4 |
| winget が入れ直した areka（置いたのは winget。winget の入れ先なので `target\` の外。`--purge` を付けて外した後に残ったのは、入れ直す前から在った areka の記憶の 1 ファイルと空のフォルダ。**このフォルダは、中身の一覧を取ってから 14:17:20 に AI が消した＝今は無い**） | `%LOCALAPPDATA%\Microsoft\WinGet\Packages\Areka.Areka.Portable__DefaultSource\` | 3.4 |
| 片付けた後の利用者の側を読むスクリプト（入れ先のフォルダと利用者の PATH の登録を読むだけ）・出た文・読んだ利用者の PATH の文字列 | `target\winget-check\3.4-final-user.ps1`・`target\winget-check\logs\3.4-final-user.txt`・`3.4-final-user-path.raw.txt` | 3.4 |
| 機械の全員向けに入れた areka を普段の権限で起こすスクリプト 2 つ（名前が `bounded` で始まる回だけ有界にする）と、MCP の口へ 1 本送るスクリプト | `target\winget-check\3.5-launch.ps1`・`3.5-inner.ps1`・`3.5-mcp.ps1` | 3.5 |
| 1 回目（有界）と 2 回目（有界でない・メニューを動かした回）の起動の記録（標準出力・標準エラー）・起こしたプロセスの結果（解決先・時刻・終了コード）・組み直した PATH | `target\winget-check\logs\3.5-bounded1.*`・`3.5-bounded1-fresh-*`・`3.5-menu1.*`・`3.5-menu1-fresh-*` | 3.5 |
| MCP の口へ送った文と答えの全文（`get_active_ghost_list`・`get_log`） | `target\winget-check\logs\3.5-mcp.log` | 3.5 |
| 機械の全員向けの入れ先の一覧（1 回目の起動の後・2 回目の起動の後。相対パスと大きさと時刻・隠しファイルも含む）と、入れ先の権限の一覧（7 か所。利用者の名前は「この端末の利用者」に置き換えてある） | `target\winget-check\logs\3.5-installdir-after-bounded1.tsv`・`3.5-installdir-after-menu1.tsv`・`3.5-acl.txt` | 3.5 |
| 一覧 2 つを数えて見比べるスクリプトと出た文／配布 zip の中の辞書から `OnInstallFailure` の台詞を探すスクリプトと出た文（zip は開いて読むだけで、展開していない）／`%LOCALAPPDATA%\VirtualStore` の読み | `target\winget-check\3.5-listing-summary.py`・`logs\3.5-listing-summary.txt`／`target\winget-check\3.5-zip-grep.py`・`logs\3.5-zip-grep.txt`／`logs\3.5-virtualstore.txt` | 3.5 |
| winget が機械の全員向けに入れた areka（置いたのは winget で、打ったのは開発者。winget の入れ先なので `target\` の外。2 回の起動が中に作ったのは `ghost\emo2\ghost\master\profile\` の下の 50 ファイルだけ。**開発者が 14:41:08 に、管理者の端末で `--purge` を付けて外した＝今は無い**） | `C:\Program Files\WinGet\Packages\Areka.Areka.Portable__DefaultSource\` | 3.5 |
| 機械の側の後片付けの 4 項目を読むスクリプト（読むだけ。`winget list`・`winget settings export` のほかは winget を打たない）・出た文・読んだ PATH の文字列（機械の側・利用者の側・組み直した物）・新しいプロセスでの `areka` の解決・`winget list` 3 回分・`winget settings export` の出力 | `target\winget-check\3.5-final-machine.ps1`・`target\winget-check\logs\3.5-final-machine.txt`・`3.5-final-machine-path.raw.txt`・`3.5-final-user-path.raw.txt`・`3.5-final-fresh-path.txt`・`3.5-final-fresh-shell.*`・`3.5-final-list-*`・`3.5-final-settings-export.*` | 3.5 |
| winget 自身の記録の写し 6 つ（設定をオンにした回と、その前の断られた回・機械の全員向けに入れた回・外した回と、その前の断られた回・設定を戻した回。パスの頭は置き換えてある）と、設定がオンだった間の winget の全部の回を 1 行ずつにまとめた一覧、`winget error 0x8a150019` の答え | `target\winget-check\logs\3.5-final-winget-diag-settings-enable-refused.txt`・`3.5-final-winget-diag-settings-enable.txt`・`3.5-final-winget-diag-install-machine.txt`・`3.5-final-winget-diag-uninstall-machine-refused.txt`・`3.5-final-winget-diag-uninstall-machine-purge.txt`・`3.5-final-winget-diag-settings-disable.txt`・`3.5-final-winget-diag-window.txt`・`3.5-final-winget-error.*` | 3.5 |
| winget が機械の全員向けの入れる操作のときに作った空のフォルダ 3 つ（置いたのは winget。`target\` の外。**今も残っている**。中身は 0 件。「後片付けの確かめ」の「機械の側」） | `C:\Program Files\WinGet\`・`C:\Program Files\WinGet\Links\`・`C:\Program Files\WinGet\Packages\` | 3.5 |
| 提出の道具が出した文（標準出力・標準エラー）と、時刻・終了コード（標準出力の写しは、ログインのときの 1 回限りのコードの行だけ、17:42:54 に進行役の AI がコードを伏せた字に書き替えてある。ほかの行は道具が出したまま。コードはトークンではなく、どこにも残っていない） | `target\winget-check\logs\4.2-submit.stdout.txt`・`4.2-submit.stderr.txt`・`4.2-submit.times.txt` | 4.2（進行役の AI） |
| PR の本文の写し（確かめ項目に印を付ける前と後・同意の項目に印を付ける前と後）と、同意の文を送った時刻 | `target\winget-check\logs\4.2-pr-body-before.md`・`4.2-pr-body-after.md`・`4.2-pr-body-before-cla.md`・`4.2-pr-body-after-cla.md`・`4.2-cla-comment.time.txt` | 4.2（進行役の AI） |
| PR から取ってきた 4 ファイル（PR の枝の先頭のコミットの物と同じバイト） | `target\winget-check\pr-450070\` | 4.2 |
| 置き換える前の雛形 4 ファイルの写し | `target\winget-check\pr-450070\template-before\` | 4.2 |
| 判定が赤を出せることを見るための、わざと変えた PR のファイルの写し（ハッシュを入れ替えた物・値を 1 つ変えた物） | `target\winget-check\pr-450070\negative-swap\`・`target\winget-check\pr-450070\negative-value\` | 4.2 |
| 写し戻しの判定のスクリプト（行の集まりの一致・arch ごとの URL とハッシュの組の一致・YAML として解いた見比べ）と、出た文（置き換える前・置き換えた後・赤を出せることを見た 2 回・端末の 1 行での判定） | `target\winget-check\4.2-compare.py`・`target\winget-check\logs\4.2-compare.txt`・`4.2-compare-after-replace.txt`・`4.2-compare-negative-swap.txt`・`4.2-compare-negative-value.txt`・`4.2-judge1-oneliner.txt` | 4.2 |
| PR を読んだときの出力（PR の値とファイルの一覧・印とコメントと本文）と、Release `v0.0.2` の物の一覧（URL と `digest`） | `target\winget-check\logs\4.2-pr-view.json`・`4.2-pr-files.json`・`4.2-pr-read-2.json`・`4.2-release-assets.json` | 4.2 |
| 置き換えた後の `git diff -- dist/winget/0.0.2` の全文と、`winget validate` が出した文・時刻・終了コード | `target\winget-check\logs\4.2-git-diff.txt`・`4.2-git-diff.stderr.txt`（0 バイト）・`4.2-validate.stdout.txt`・`4.2-validate.stderr.txt`・`4.2-validate.times.txt` | 4.2 |

- 取ってきた後の `git status --porcelain` は 0 行（`target` の下に限って見ても 0 行）。`git ls-files -- target` も 0 件で、取ってきた物は `git status` に出ない。
- `git check-ignore -v target/winget-check/release/areka-0.0.2-x64.zip` は `.gitignore:1:target` を返した（追跡の外になる理由がこの 1 行であることの裏付け）。

（この後のタスクで置く物は、置いたタスクが上の表に行を足す）

## 既知の制限

- arm64 の項目は実機で入れていない。arm64 について確かめたのは、`winget validate` の成功（上の「雛形の検査」の「`winget validate`」）と、`InstallerSha256` が Release の `.sha256` に書かれた値と一致すること（同じく「ハッシュの突き合わせ」）まで（要件 3.7）。
- 実測の元になる状態（タスク 3.1）は、右クリックメニューで作った。メニューを操作したのは、開発者から `areka.exe` の画面の操作を許された AI。だから、メニューの入口「インストール…」「ゴースト」「終了」は**通っている**。通っていないのは台本（MCP の `sakurascript`）の入口のほうで、これは設計の見込み（状態は MCP の台本で作り、メニューの入口は通らない）と**逆**になった。理由は、入れた 0.0.2 の `sakurascript` が中身の無い仮の受け口で、台本を受け取らないため（「見つかった件と起票」の 1 行目）。入れる手続きと切り替えの手続きそのものは、どちらの入口でも同じ物。
- 状態を作った起動（3 回目）は、`AREKA_NO_ALERT` を置いていない（置くと、0.0.2 はメニューの「インストール…」でファイルを選ぶ画面を出さない）。利用者の普段の起動と同じ側の条件。
- シェルの記憶（`ghost\<ゴースト>\shell\<シェル>\profile\areka\`）は、0.0.2 では作る操作が無く、作れなかった。上げ直しと外し方の実測では、この種類を「測れなかった」と書く（理由は「上げ直しの実測」の「作った物の一覧」）。シェルのフォルダそのものは `ghost\` の下に在るので、フォルダの有無は見られる。
- 上げ直しに使ったコマンドは `winget upgrade --manifest`（タスク 3.2）。止まらなかったので、代わりの手（`winget install --manifest`）は使っていない。
- 上げ直しの確かめ専用のマニフェストは、版の欄だけを `0.0.2.1` にした物で、取り寄せる zip は 0.0.2 と同じ。「置き換わった」は、目印を付けた同梱のファイルが zip の中の元の値に戻ったことで見ている（中身の違う新しい版へ替わるところは見ていない）。
- 上げ直しの後の起動は、タスク 2.2 の起こし方に `AREKA_MCP_PORT` を 1 つ足している（今のゴーストの名前を MCP の口から聞くため）。0.0.2 の `get_status` は名前を返さないので、名前は `get_active_ghost_list` で読んだ。
- 上げ直しと外し方の実測の結果は、**手元のマニフェストで入れた形**でのもの（要件 4.5）。入れ先のフォルダの名前（`Areka.Areka.Portable__DefaultSource`）と、`winget list` の ID（`ARP\User\X64\Areka.Areka.Portable__DefaultSource`）は、winget-pkgs から入れた形（`Areka.Areka.Portable_Microsoft.Winget.Source_8wekyb3d8bbwe`）と違う。winget-pkgs から入れた形では測っていない。
- 上げ直しと外し方の実測は、winget の利用者の設定「外すときに入れ先を丸ごと消す」（`uninstallBehavior.purgePortablePackage`）が**既定のまま（オフ）**で測った（要件 4.5）。利用者の設定ファイル自体が無いことを、設定を変える前（「変えた設定」の節）と、外す直前（「外し方の実測」の 2）に読んでいる。この設定をオンにした機械では、`--purge` を付けなくても入れ先が丸ごと消える見込みだが、測っていない。
- 外し方の実測は、確かめ専用の版 `0.0.2.1` へ上げ直した後の物を外している（提出する版 0.0.2 を入れたままの物を外したのではない）。上げ直しで消えた物と、その後の起動が書き替えた物は、外す前に写しから入れ先へ戻した。戻した物は、winget から見ると、上げ直しの後に利用者が置いた物と同じ（winget の控えに載っていないファイル）。winget の控えは 0.0.2.1 の物のままにした。
- 外した後の「利用者の PATH の項目が残っている」「`areka` が解決しない」は、読んだだけの事実で、後片付けの正式な確かめではない（「後片付けの確かめ」の節に、タスク 3.4 が書く）。
- 入れ直し（タスク 3.4）で目に入った注意書きは、**開発機の言語に合う側（日本語）だけ**。英語の側について確かめたのは、`winget validate` の成功と、YAML として読んだ値が設計の文と 1 字も違わないことまでで、winget が英語の文を出すところは見ていない。
- winget は、注意書きの全角の括弧 `（`・`）` を半角の `(`・`)` にそろえて出す（2 字。「後片付けの確かめ」の「利用者の側」の 4）。雛形の文は全角のまま。
- 注意書きは、winget の利用者の設定で「インストールの後の注意書きを出さない」（`installBehavior.disableInstallNotes`。既定はオフ）をオンにしている機械では出ない（winget v1.29.380 の `doc/Settings.md` と、ソースの `DisplayInstallationNotes` の読み。測っていない）。開発機は利用者の設定ファイルが無く、既定のままで出た。
- 入れ直し（タスク 3.4）は、外し方の実測の残り（入れ先のフォルダと、利用者の PATH の項目）の上へ行った。何も残っていない機械へ入れる形は、注意書きを足す前の雛形で 1 度通っている（タスク 2.2）。注意書きを足した雛形を、何も残っていない機械へ入れる形では測っていない。
- `--purge` を付けた外し方は、残りの上へ入れ直した物を外す形でだけ測った（入れ先のフォルダと PATH の項目が残った。理由は「後片付けの確かめ」の「利用者の側」の 7）。何も残っていない機械へ入れた物を `--purge` で外す形では測っていない。
- **winget で外しただけでは、利用者の PATH に、もう無いフォルダを指す項目が 1 件残る**。開発機では、開発者が手でその項目を消した（「後片付けの確かめ」の「利用者の側」の 8）。areka を winget で外す利用者は、自分で消さない限り、この行き先の無い項目を持ち続ける。見たのは、手元のマニフェストで入れた形で、入れ先のフォルダが前の実測から残っていた回だけ（`--purge` を付けない回と付けた回の 2 回）。何も残っていない機械へ入れて外す形は、別には測っていない（winget のソースの読みでは、その形でも同じく残る見込み。同じ節の 7）。
- 利用者の側の後片付けの 4 項目のうち、② `areka` の解決と ④ `winget list` の行は、`--purge` で外した直後（14:09）の読みが根拠で、残った 2 つを片付けた後（14:19 より後）には読み直していない。そのときには、次のタスクの機械の全員向けの入れ方が済んでいて、今読むと機械の側の物が当たるため（同じ節の頭の表の下）。
- 機械の全員向けの実測（タスク 3.5）は、**1 台の機械**での結果で、その機械は、入れた管理者と普段の利用者が**同じアカウント**（開発者が、自分のアカウントを管理者に上げた端末で入れた）。ゴーストごとの記憶（`ghost\` の下）が書けたのは、入れ先の下の階層に、このアカウントへ全部の権限を与える行が在ったため（「機械の全員向けの実測」の 7）。入れた管理者が別のアカウントの機械では、それも書けない見込みで、`areka.exe`・`ghost\`・`balloon\` に `BUILTIN\Users` の行が無いことから、起こすこともできないおそれが在る。別のアカウントでは測っていない。
- 機械の全員向けに入れたときに winget が出した文は、取れていない（開発者の管理者の端末に出た）。入れた直後の読み（`winget list` の行・PATH の登録の数）も、その回の出力のファイルは無く、作業を進めている AI の読みを聞いて書いた。残っているファイルで確かめられたのは、14:20:44 に登録から組み直した PATH と、入れ先の一覧の時刻まで。（後片付けのときに、winget 自身の記録で、入れた回の始まりと終わりの時刻・入れ先のフォルダを作った行・PATH へ足した行も読んだ。「機械の全員向けの実測」の 8）
- 機械の全員向けに入れた後・起こす前の入れ先の中身は、**数でしか分からない**（150 ファイル・16789117 バイトと、直下の 9 つの名前。一覧のファイルは、置き場の指定を誤って保存できなかった）。起動の後の一覧から areka が作った物を除いた数は、この数と合う。
- 機械の全員向けでの「インストール…」は、**1 回・検体 1 つ（`claudia.nar`）**で試しただけ。バルーンの `.nar` は試していない（同じ作業フォルダ `.nar-work\` を入れ先の直下に作る手続きなので、同じ所で止まる見込み）。ゴーストの切り替えも試していない（ゴーストが同梱の 1 体だけなので、切り替える先が無い）。
- 機械の全員向けで入れられなかったときのゴーストの台詞は、作業を進めている AI が画面の写しで見た 2 行（「えー、なんでなん！？」「ファイルが壊れてるのかもね。」）で、**写しのファイルは残っていない**。MCP の `dump_balloon` は呼んでいない（`get_log` は呼んだが、返るのは areka 自身の記録で、台詞ではない）。台詞の全文（3 行）は、配布 zip の中の辞書から読んだ物で、画面で 1 行目を見たわけではない。
- 入れ先の権限の一覧（`logs\3.5-acl.txt`）は 7 か所だけで、取ったコマンドは残っていない。下の階層にだけ「この端末の利用者」の行が在る仕組みは調べていない。
- 機械の全員向けに入れた形での上げ直しは、測っていない。
- 機械の全員向けを外した回と、設定を戻した回に、**開発者の管理者の端末に出た文は取れていない**。分かっているのは、winget 自身の記録に残った行（始まりと終わりの時刻・打った字面・何を消したか）と、前後に AI が機械を読んだ値まで。機械の PATH を直した 1 行（PowerShell）は、winget を通らないので記録がどこにも無く、効いた時刻は前後の読みの間（14:43:46〜14:46:58）としか言えない。
- 機械の側の後片付けの 4 項目のうち、外した直後から片付くまでの途中の値（14:41:29・14:41:46・14:43:46・14:46:58）は、作業を進めている AI の読みを聞いて書いた（その回の出力のファイルは無い）。この節を書いた AI が自分で読んだのは、片付いた後の 14:49 の 1 回。
- ② `areka` の解決は、これまでと同じく、**登録から組み直した PATH**で見た（新しく開く端末が受け取るはずの PATH）。開いたままのアプリが持っている古い PATH は読んでいない。PATH の登録を手で直した 1 行は、レジストリの値を書き替えるだけなので、サインインし直すまでは、もう無いフォルダを指す項目を持ったままのアプリが在るかもしれない（測っていない。在っても、その先に `areka.exe` は無いので解決はしない）。利用者の PATH を手で直したとき（タスク 3.4）も同じ。
- **開発者に頼んだ回数は、設計の見込み（提出を含めて 4 回＝提出の前までは 3 回）より多かった**。提出の前までで 4 回: ① 設定をオンにする ②′ 利用者の PATH の項目を消す（タスク 3.4。winget が消さなかったため）② 機械の全員向けに入れる ③ 機械の全員向けを外す・機械の PATH の項目を消す・設定を戻す。③ は 1 度で済まず、やり取りが 3 往復になった（作業を進めている AI の示し方が短すぎた: 開発者から「どのフォルダか」と聞かれ、次に PATH の 1 行の説明を求められ、その後に PATH の 1 行をもう 1 度打ってもらった）。winget 自身の記録では、オンにする回と外す回のそれぞれで、管理者の端末でなかったために断られた 1 度目が在る（どちらも、すぐ後に管理者の端末で通っている）。
- **`C:\Program Files\WinGet\` と、その下の空のフォルダ `Links\`・`Packages\` が、開発機に残っている**（機械の全員向けの入れる操作が作った物。`--purge` を付けて外しても消えない。後片付けの 4 項目の外。「後片付けの確かめ」の「機械の側」）。消すには管理者の権限が要る。開発者へは、14:43 に `Packages\` の 1 つを、14:59 ごろの報告で 3 つとも今回できた空のフォルダであることを伝えてあり、消すかどうかは開発者に任せている（この記録を書いた時点では残っている）。

## 見つかった件と起票

| 起きたこと | どの段か | 根拠の記録の場所 | 起票した spec |
|---|---|---|---|
| 入れた areka（0.0.2）の MCP の `sakurascript` が、台本を受け取らずに `NG:not implemented yet` と答える。台本で `.nar` を入れる・ゴーストを切り替える・終える、のどれもできず、状態を MCP の口から作れなかった（代わりに、右クリックメニューを AI が画面の操作で動かして作った） | 段 4（タスク 3.1） | `target\winget-check\logs\3.1-mcp.log`（送った文と答え）・上の「上げ直しの実測」の「作る状態」・タグ `v0.0.2` の `crates/areka/src/mcp/sakurascript.rs` | `areka-P0-mcp-kanade-tools`（今ある spec。`sakurascript` に中身を入れる仕事を、同 spec の brief がすでに持っている。新しい起票は無し。brief は直していない） |
| winget の上げ直しで、利用者の物が消えた。areka は利用者の物（後から入れたゴーストとバルーン・ゴーストの記憶）を、入れ先の `ghost\`・`balloon\` の中に置く。winget は上げ直しのときに、この 2 つのフォルダを中身ごと消してから zip の中身を置き直した（消えた物は 6・208 ファイル。areka の記憶 `profile\` は残った）。その後の起動は、覚えていたゴーストが無いので同梱のゴーストで立った。**外すとき（`--purge` も `--preserve` も付けない `winget uninstall`）も同じ物が消えた**（根は同じ＝利用者の物が、入れ先の `ghost\`・`balloon\` の中に在る）。winget はこの 2 つのフォルダを中身ごと消し、消えた物は同じ 6（利用者の物は 207 ファイル）。残ったのは areka の記憶 `profile\` と空の `.nar-work\` だけで、winget は「ファイルが入れ先に残る」と告げて入れ先のフォルダを残した。そのフォルダを指す利用者の PATH の項目 1 件も残っていた。winget の文は、どちらのときも、利用者の物を消したことを告げない | 段 5（タスク 3.2）・段 6（タスク 3.3） | 上の「上げ直しの実測」の「上げ直し」・`target\winget-check\logs\3.2-diff-after-upgrade.txt`・`3.2-run.stdout.log`／上の「外し方の実測」・`target\winget-check\logs\3.3-diff-after.txt`・`3.3-uninstall.stdout.txt` | `areka-P0-user-data-root`（新しい起票。要件 4.6 の起票＝利用者の物が winget の上げ直しと外し方で消えないようにする仕事。外した後に入れ先のフォルダが残る分も、この spec） |
| winget で外しても、利用者の機械に後が残る。雛形は `ArchiveBinariesDependOnPath: true` で、入れ先のフォルダを利用者の PATH に 1 件足す。`winget uninstall` は、`--purge` を付けても付けなくても、この PATH の項目を消さなかった（もう無い `areka.exe` の在ったフォルダを指す項目が残る）。入れ先のフォルダも、areka の記憶（`profile\areka\`）が中に在ると残った（`--purge` を付けない回。`--purge` を付けた回も、残りの上へ入れ直した物だったので残った）。winget の文は、PATH の項目が残ることを告げない（フォルダについては、`--purge` を付けない回だけ「ファイルが入れ先に残る」と告げる）。開発機では、フォルダを AI が、PATH の項目を開発者が手で片付けた。**機械の全員向け（`--scope machine`）でも同じことが起きた**（タスク 3.5）: 開発者が管理者の端末で `--purge` を付けて外すと、入れ先のフォルダは消えたが、機械の PATH の項目 1 件は残り（winget 自身の記録に、同じ `Install directory is not empty` の行）、開発者が手で消した。これは winget の側の動きだが、areka が利用者の物を入れ先の中に置くことと、雛形が PATH の入れ方を選んだことから出てくる（だから、上の行の「利用者の物が消える」の起票へまとめる手も在る。まとめるか分けるかは、タスク 3.6 で作業を進めている AI が決める） | 段 6（タスク 3.3）・段 7（タスク 3.4）・段 8（タスク 3.5） | 上の「後片付けの確かめ」の「利用者の側」の 6（14:09 の 4 項目の読み）・7（winget 自身の記録の行 `Install directory is not empty`・`Unable to remove install directory as there are remaining files in: …` とソースの読み）・8（片付け）／`target\winget-check\logs\3.4-after-purge-*`・`3.4-winget-diag-path-lines.txt`・`3.4-winget-diag-uninstall-purge.txt`・`target\winget-check\review\3.4\PortableInstaller.cpp`／機械の側は、上の「機械の全員向けの実測」の 8・「後片付けの確かめ」の「機械の側」・`target\winget-check\logs\3.5-final-winget-diag-uninstall-machine-purge.txt`・`3.5-final-machine.txt` | 2 つに分けた。**入れ先のフォルダが残る分** → `areka-P0-user-data-root`（新しい起票。上の行と同じ spec。利用者の物が入れ先の中に無くなれば、winget がフォルダを消せる）／**PATH の項目が残る分** → `areka-P0-winget-release-automation`（今ある spec。winget の側の動きで、areka からは直せない。説明書の既知の制限の材料として渡す。同 spec の brief への申し送りの文は、後のタスク 5.1 が書く） |
| 機械の全員向け（`--scope machine`）に入れると、普段の権限の利用者は、areka の記憶を書けず、ゴーストを後から入れられない。areka は、利用者の物（areka の記憶 `profile\areka\`・後から入れるゴーストとバルーン・入れるときの作業フォルダ `.nar-work\`）を `areka.exe` の隣＝入れ先のフォルダに置く。機械の全員向けの入れ先 `C:\Program Files\WinGet\Packages\…` は、普段の権限では直下に何も作れない。1 回の起動につき、areka の記憶への書き込みが 3 回とも「アクセスが拒否されました」で失敗し、メニューの「インストール…」は作業フォルダを作れずに止まった。ゴーストは立つ。**根は上の 2 行目（上げ直しと外し方で利用者の物が消える）と同じ＝利用者の物が入れ先の中に在る**ので、置き場を直す起票へまとめられる | 段 8（タスク 3.5） | 上の「機械の全員向けの実測」の 3〜7・`target\winget-check\logs\3.5-bounded1.stdout.log`・`3.5-menu1.stdout.log`（`persist commit failed … scope=App`・`Stage で I/O に失敗: …\.nar-work\34544-0`）・`3.5-installdir-after-menu1.tsv`・`3.5-acl.txt` | `areka-P0-user-data-root`（新しい起票。2 行目と根が同じなので、同じ起票にまとめた＝要件 4.8） |
| areka の記憶が書けないとき、areka は利用者へ何も伝えない。起動の記録に `ERROR`・`WARN` が残るだけで、台詞も知らせも出ず、ゴーストは普段どおりに立って喋る。その裏で、最後に使ったゴーストは覚えられず、起動中の印は書くことも消すこともできない＝前回がきれいに終わらなかったことに気付く仕組みが、黙って働かなくなる（落ちても、次の起動は何事も無かったように立つ）。あわせて、記録の文が実際と合わない所が 2 つ在った: `last_used_recorded` の「記憶へ書きました」は、すぐ次の行で書き込みが失敗しているのに出る／終わるときの警告「次の起動は前回落ちたとして…」は、印がそもそも書けていないこの場合には、そうならない（2 回目の起動に `session_mark_found` は 0 件） | 段 8（タスク 3.5） | 上の「機械の全員向けの実測」の 3・4・6・7・`target\winget-check\logs\3.5-bounded1.stdout.log`・`3.5-menu1.stdout.log`（`session_mark_write_degraded`・`session_mark_clear_degraded`・`last_used_recorded`）・タグ `v0.0.2` の `crates/areka/src/boot_resolve.rs`・`crates/areka/src/boot_config.rs` | `areka-P0-write-failure-notice`（新しい起票） |
| 入れ先に書けないせいで `.nar` を入れられなかったのに、利用者には「ファイルが壊れてるのかもね。」と伝わる。areka がゴーストへ渡す失敗の理由は正典の語 `extraction` の 1 語だけで、書庫が壊れているときと、入れ先へのファイルの読み書きに失敗したとき（今回＝`kind="Io" phase="stage"`）が同じ語になる。同梱のゴースト（えも？？）の辞書は、理由を見ずに 3 つの台詞から 1 つを選び、その 1 つがこの台詞。本当の理由（アクセスが拒否された場所）は、起動の記録の `ERROR` にしか出ない。利用者は、壊れていない `.nar` を疑うことになる。上の行（置き場）を直せば今回の形では起きなくなるが、ほかの読み書きの失敗でも同じ伝わり方になるので、別の件として書く | 段 8（タスク 3.5） | 上の「機械の全員向けの実測」の 4（`install_failed … word="extraction"` の行と、台詞の出どころ）・`target\winget-check\logs\3.5-menu1.stdout.log`・`3.5-zip-grep.txt`（辞書 `ghost\emo2\ghost\master\dic\install.pasta` の 89〜101 行目）・タグ `v0.0.2` の `crates/areka/src/install/judge.rs`（関数 `failure_word`） | `areka-P0-write-failure-notice`（新しい起票。上の行と同じ spec） |

起票の結果（タスク 3.6・要件 4.6・4.8・6.3）:

- **起票した数は 2**。名前は `areka-P0-user-data-root`（上の表の 2・4 行目と、3 行目のうち入れ先のフォルダが残る分）と `areka-P0-write-failure-notice`（5・6 行目）。どちらも brief だけを書いた（`.kiro/specs/areka-P0-user-data-root/brief.md`・`.kiro/specs/areka-P0-write-failure-notice/brief.md`）。steering の `roadmap.md` には、spec 台帳の末尾へ 2 行と、「テーマ別の決めごと」の末尾へ節 1 つを足し、数を直した（brief を持つ spec は 96 本 → 98 本）。
- **今ある spec へ割り付けた件は 2**。1 行目 → `areka-P0-mcp-kanade-tools`／3 行目のうち PATH の項目が残る分 → `areka-P0-winget-release-automation`。この時点では、どちらの brief も直していない。
- 上の表の 6 行は、どの行にも spec の名前が入っている（空の行は 0）。
- 起票した日時: 2026-10-10（+09:00）。開発者との起票の対話（`/kiro-discovery`）は 15:50〜16:10 ごろで、作業を進めている AI（親のセッション）が行った。brief と `roadmap.md` への書き足しは 17:00〜17:20 ごろで、この節を書いた AI が、対話の結果を受け取って書いた。
- **設定が戻っていることを確かめてから始めた**: 対話の前の 15:47:38 に、作業を進めている AI が `winget settings export` の `adminSettings.LocalManifestFiles` を読んで `False`（その回の出力のファイルは無い。この節を書いた AI は winget を打っておらず、聞いて書いた）。設定を戻した時刻は 14:43:20（上の「変えた設定」）。
- 分け方は、作業を進めている AI が決めた（開発者は、分け方を AI に任せた）。`areka-P0-user-data-root` の向き（利用者の物を入れ先の外の書ける根に置く）は起票のときの推しで、**開発者の裁定はまだ受けていない**（同 spec の要件の討議で受ける）。
- 考えたが、別には起票しなかった件は 3:
  - 説明書（`dist/README.txt`）の「■ 記憶の置き場」が、0.0.2 では書く操作の無いシェルの記憶の置き場も書いている件 → `areka-P0-user-data-root` の範囲に入れた（置き場を直すときに、同じ節を書き直すため）。
  - タスク 3.1 の 3 回目の起動で 1 度だけ見た、最後の記録の行からプロセスが消えるまでの約 6 秒 → タスク 3.2・3.5 の起動では出なかった（0.05〜0.08 秒）。再現していないので起票しない。
  - `AREKA_NO_ALERT` を置くと、メニューの「インストール…」がファイルを選ぶ画面を出さない件 → 決めたとおりの動き（自動のテストで画面が止まらないようにする環境変数）なので起票しない。
