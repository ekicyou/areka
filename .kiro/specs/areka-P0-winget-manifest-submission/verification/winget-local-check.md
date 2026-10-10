# 手元の確かめ: 提出するマニフェストで winget から入れて `areka` で起動する

- 日付: 2026-10-10
- 機械: 開発機（Windows 11 Pro 10.0.26300.9550・26H2・x64）・winget v1.29.380
- コミット: `f305cee6`（着手のときの枝の先頭。枝は `claude/areka-p0-winget-manifest-1c4fd7`）
- 判定: **まだ判定していない**（入れて起動する確かめが済んだときに書く。タスク 2.2 で記入）
- 提出する版: `0.0.2`（Release `v0.0.2`。決め方は「雛形の検査」の「ハッシュの突き合わせ」の 1）

## 変えた設定

（タスク 2.1 で変える前の値とオンを確かめた時刻を、タスク 3.5 で戻した時刻を記入）

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

（タスク 1.3 で記入）

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
