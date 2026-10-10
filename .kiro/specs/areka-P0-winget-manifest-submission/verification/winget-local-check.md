# 手元の確かめ: 提出するマニフェストで winget から入れて `areka` で起動する

- 日付: 2026-10-10
- 機械: 開発機（Windows 11 Pro 10.0.26300.9550・26H2・x64）・winget v1.29.380
- コミット: `f305cee6`（着手のときの枝の先頭。枝は `claude/areka-p0-winget-manifest-1c4fd7`）
- 判定: **入れて起動する確かめは合格**（6 項目とも合格・タスク 2.2・2026-10-10 13:01〜13:03。下の「入れて起動する」）。上げ直し・外し方・機械の全員向けの実測と、後片付けの確かめは、まだ済んでいない（それぞれの節に、そのタスクが書く）
- 提出する版: `0.0.2`（Release `v0.0.2`。決め方は「雛形の検査」の「ハッシュの突き合わせ」の 1）

## 変えた設定

変える設定は次の 1 つだけ（開発者の手で・要件 3.4）。AI は winget と OS の設定を変えない。オンにするのも戻すのも、開発者が管理者の端末で打つ。時刻はどれも 2026-10-10（+09:00）。

| 設定 | 変える前の値 | 変える前の値を読んだ時刻 | オンを確かめた時刻 | 戻した時刻 | 戻した後の値 |
|---|---|---|---|---|---|
| winget の `LocalManifestFiles`（管理者の端末で `winget settings --enable LocalManifestFiles`） | `false` | 11:44:01 | 12:57:16 に `true` を確認 | （この後に記入） | （この後に記入） |

- 変える前の値の読み方: `winget settings export`（読むだけ）が返す JSON の `adminSettings.LocalManifestFiles`。同じ JSON の管理者向けの設定は 6 つで、6 つとも `false` だった（`BypassCertificatePinningForMicrosoftStore`・`ConfigurationProcessorPath`・`InstallerHashOverride`・`LocalArchiveMalwareScanOverride`・`LocalManifestFiles`・`ProxyCommandLineOptions`）。
- 開発者に打ってもらうコマンド（管理者の端末）: `winget settings --enable LocalManifestFiles`
- オンにしたのは開発者（管理者の端末で上のコマンドを打った）。開発者が伝えた winget の答えは 1 行で、`管理者設定 'LocalManifestFiles' を有効にしました。`（この文は AI が自分で見た物ではなく、開発者から聞いた物）。
- オンの確かめ（AI が読んだ・12:57:16）: `winget settings export` の `adminSettings.LocalManifestFiles` が `true`（終了コード 0・標準エラー 0 バイト）。
- **変えた設定はこの 1 つだけ**。同じ回の読みで、管理者向けの設定 6 つのうち `true` は `LocalManifestFiles` の 1 つだけで、残りの 5 つは変える前と同じ `false`。下の「読んで記録するだけの設定」も同じ時刻に読み直して、変える前と同じだった（開発者モードは値 `0`＝オフ・利用者の設定ファイルは無いまま＝フォルダ 4 つ・ファイル 0 件）。
- 組織の決まり（グループ ポリシー）で winget の設定が固定されていないことも読んだ（11:45:25）。`HKLM:\SOFTWARE\Policies\Microsoft\Windows\AppInstaller` と `HKCU:\SOFTWARE\Policies\Microsoft\Windows\AppInstaller` はどちらもキーが無い。

（戻した時刻と戻した後の値は、タスク 3.5 で記入）

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

（注意書きを足したとき・PR から写し戻したときの `winget validate` は、そのタスクがここへ書き足す）

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

（タスク 3.3 で記入）

## 機械の全員向けの実測

（タスク 3.5 で記入）

## 後片付けの確かめ

（利用者の側はタスク 3.4 で、機械の側はタスク 3.5 で記入）

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

（タスク 3.3・3.4 で記入）

## 見つかった件と起票

| 起きたこと | どの段か | 根拠の記録の場所 | 起票した spec |
|---|---|---|---|
| 入れた areka（0.0.2）の MCP の `sakurascript` が、台本を受け取らずに `NG:not implemented yet` と答える。台本で `.nar` を入れる・ゴーストを切り替える・終える、のどれもできず、状態を MCP の口から作れなかった（代わりに、右クリックメニューを AI が画面の操作で動かして作った） | 段 4（タスク 3.1） | `target\winget-check\logs\3.1-mcp.log`（送った文と答え）・上の「上げ直しの実測」の「作る状態」・タグ `v0.0.2` の `crates/areka/src/mcp/sakurascript.rs` | （タスク 3.6 で記入。中身を入れる仕事は、起票済みの `areka-P0-mcp-kanade-tools` の brief に在る） |
| winget の上げ直しで、利用者の物が消えた。areka は利用者の物（後から入れたゴーストとバルーン・ゴーストの記憶）を、入れ先の `ghost\`・`balloon\` の中に置く。winget は上げ直しのときに、この 2 つのフォルダを中身ごと消してから zip の中身を置き直した（消えた物は 6・208 ファイル。areka の記憶 `profile\` は残った）。その後の起動は、覚えていたゴーストが無いので同梱のゴーストで立った | 段 5（タスク 3.2） | 上の「上げ直しの実測」の「上げ直し」・`target\winget-check\logs\3.2-diff-after-upgrade.txt`・`3.2-run.stdout.log` | （タスク 3.6 で記入。要件 4.6 の起票＝利用者の物が上げ直しと外し方で消えないようにする仕事） |

（タスク 3.6 で記入）
