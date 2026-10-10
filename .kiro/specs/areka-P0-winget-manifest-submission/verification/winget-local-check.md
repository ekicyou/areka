# 手元の確かめ: 提出するマニフェストで winget から入れて `areka` で起動する

- 日付: 2026-10-10
- 機械: 開発機（Windows 11 Pro 10.0.26300.9550・26H2・x64）・winget v1.29.380
- コミット: `f305cee6`（着手のときの枝の先頭。枝は `claude/areka-p0-winget-manifest-1c4fd7`）
- 判定: **まだ判定していない**（入れて起動する確かめが済んだときに書く。タスク 2.2 で記入）
- 提出する版: `0.0.2`（Release `v0.0.2`。決め方は「雛形の検査」の「ハッシュの突き合わせ」の 1）

## 変えた設定

変える設定は次の 1 つだけ（開発者の手で・要件 3.4）。AI は winget と OS の設定を変えない。オンにするのも戻すのも、開発者が管理者の端末で打つ。時刻はどれも 2026-10-10（+09:00）。

| 設定 | 変える前の値 | 変える前の値を読んだ時刻 | オンを確かめた時刻 | 戻した時刻 | 戻した後の値 |
|---|---|---|---|---|---|
| winget の `LocalManifestFiles`（管理者の端末で `winget settings --enable LocalManifestFiles`） | `false` | 11:44:01 | （この後に記入） | （この後に記入） | （この後に記入） |

- 変える前の値の読み方: `winget settings export`（読むだけ）が返す JSON の `adminSettings.LocalManifestFiles`。同じ JSON の管理者向けの設定は 6 つで、6 つとも `false` だった（`BypassCertificatePinningForMicrosoftStore`・`ConfigurationProcessorPath`・`InstallerHashOverride`・`LocalArchiveMalwareScanOverride`・`LocalManifestFiles`・`ProxyCommandLineOptions`）。
- 開発者に打ってもらうコマンド（管理者の端末）: `winget settings --enable LocalManifestFiles`
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

（タスク 2.2 で記入）

## 上げ直しの実測

（タスク 3.1 で作った状態の一覧を、タスク 3.2 で 6 項目の結果を記入）

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

- 取ってきた後の `git status --porcelain` は 0 行（`target` の下に限って見ても 0 行）。`git ls-files -- target` も 0 件で、取ってきた物は `git status` に出ない。
- `git check-ignore -v target/winget-check/release/areka-0.0.2-x64.zip` は `.gitignore:1:target` を返した（追跡の外になる理由がこの 1 行であることの裏付け）。

（この後のタスクで置く物は、置いたタスクが上の表に行を足す）

## 既知の制限

- arm64 の項目は実機で入れていない。arm64 について確かめたのは、`winget validate` の成功（上の「雛形の検査」の「`winget validate`」）と、`InstallerSha256` が Release の `.sha256` に書かれた値と一致すること（同じく「ハッシュの突き合わせ」）まで（要件 3.7）。

（タスク 3.1・3.3・3.4 で記入）

## 見つかった件と起票

（タスク 3.6 で記入）
