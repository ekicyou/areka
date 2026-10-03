# 証跡: リリース workflow の確かめ

対象は `.github/workflows/release.yml`。手元の確かめ（タスク 1.1〜1.6）・静的な確かめ（タスク 3）・実行環境での 1 回の走り（タスク 4）の結果を、節を足して記録する。

## 1.1 取り出しの action の固定

`actions/checkout` を、その時点の最新の版が指すコミットの 40 桁の SHA で固定した。

| 項目 | 値 |
|---|---|
| 引いた日 | 2026-10-03 |
| 最新の版 | `v7.0.1` |
| 指すコミット | `3d3c42e5aac5ba805825da76410c181273ba90b1` |
| タグの種類 | 軽量タグ（`git/ref/tags/v7.0.1` の `object.type` が `commit`）＝タグの SHA とコミットの SHA が同じ |
| `release.yml` の行 | `uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1` |

引いた手順:

```
gh api repos/actions/checkout/releases/latest --jq .tag_name
gh api repos/actions/checkout/git/ref/tags/v7.0.1 --jq '.object.type+" "+.object.sha'
gh api repos/actions/checkout/commits/v7.0.1 --jq .sha
```

## 1.2 版の検査の段（S4）を手元で 5 通り回す

回した日: 2026-10-03。今の版（`Cargo.toml` の `[workspace.package] version`）は `0.0.1`。

本文の取り出し方: Python と PyYAML で `release.yml` を読み、`jobs.release.steps` から名前が `版の検査` の段をちょうど 1 件取り、その `run:` の本文をワークツリーの `target\` の下の一時ファイルへ書いた。GitHub の pwsh の包み方を写して、先頭に `$ErrorActionPreference = 'stop'`、末尾に `if ((Test-Path -LiteralPath variable:\LASTEXITCODE)) { exit $LASTEXITCODE }` を足し、ワークツリーの根から `pwsh -NoProfile -NonInteractive -Command ". '<一時ファイル>'"` で回した。環境変数は `GITHUB_REF_TYPE`・`GITHUB_REF_NAME` を差し替え、`GITHUB_OUTPUT` は場合ごとの空の一時ファイルを指した。判定は道具が機械でした（終了コード・印字に期待の字が在るか・`GITHUB_OUTPUT` の中身が `version=0.0.1` か空か）。取り出す前に、段の `id` が `version` であること・本文に `${{` が無いこと・段に `timeout-minutes` が無いことも確かめた。一時ファイルは確かめの後に消した。

| 始めた参照 | 期待 | 終了コード | 印字 | `GITHUB_OUTPUT` | 判定 |
|---|---|---|---|---|---|
| タグ `v0.0.1` | 成功・`version=0.0.1` | 0 | `タグの版と Cargo.toml の版が一致（'0.0.1'）` | `version=0.0.1` | PASS |
| タグ `v0.0.2` | 失敗・`0.0.2` と `0.0.1` の両方を印字 | 1 | `タグの版と Cargo.toml の版が違う（タグの版 '0.0.2'・Cargo.toml の版 '0.0.1'）` | 空 | PASS |
| タグ `v` | 失敗・空の版と `0.0.1` を印字 | 1 | `タグの版と Cargo.toml の版が違う（タグの版 ''・Cargo.toml の版 '0.0.1'）` | 空 | PASS |
| タグ `v0.0.1-x` | 失敗・両方を印字 | 1 | `タグの版と Cargo.toml の版が違う（タグの版 '0.0.1-x'・Cargo.toml の版 '0.0.1'）` | 空 | PASS |
| 枝 `main` | 成功・「タグが無い」と `0.0.1` を印字 | 0 | `タグが無いので比べを飛ばす（Cargo.toml の版 '0.0.1'）` | `version=0.0.1` | PASS |

段を置く前に同じ道具を回すと「`版の検査` の段が 0 件」で失敗した（赤）。道具が判定できることの較正として、不一致のときの `exit 1` を一時的に `exit 0` へ変えて回すと、タグ `v0.0.2`・`v`・`v0.0.1-x` の 3 通りが FAIL になった（`release.yml` は元へ戻した）。

（追記・タスク 1.5 の差し戻し 1 回目）タグの版と `Cargo.toml` の版の比べを `-ceq` から `[string]::Equals(…, [StringComparison]::Ordinal)` に替えた（`-ceq` は文化に従い、BOM や U+200B などの幅のない字を無視する）。替えた後の本文で上の 5 通りを回し直し、5 通りとも同じ結果だった。加えて、タグ `v0.0.1`＋U+200B が失敗（終了コード 1）になることを確かめた。結果は 1.5 の「差し戻し 1 回目の直し」の節。

## 1.3 既存の Release の検査の段（S5）を手元で回す

回した日: 2026-10-03。段の名前は `既存の Release の検査`・`id` は `guard`・`timeout-minutes: 5`。トークン（`GH_TOKEN`）と S4 の版（`VERSION`）は段の `env:` で渡し、本文に `${{` は無い。

本文の取り出し方: Python と PyYAML で `release.yml` を読み、`jobs.release.steps` から名前が `既存の Release の検査` の段をちょうど 1 件取り、`run:` の本文をワークツリーの `target\` の下の一時ファイルへ書いた。取り出すときに、`id` が `guard`・`timeout-minutes` が 5・`env` の `GH_TOKEN` と `VERSION` の 2 つ・この段とほかの全部の段の本文に `${{` が無いことを機械で判定した（6 項目すべて合格）。一つ前のタグを決める関数は、本文の `function Get-PrevTag` の行から、行頭の `}` だけの行までを別の一時ファイルへ切り出した。段の全体は、1.2 と同じく GitHub の pwsh の包み方を写して（先頭に `$ErrorActionPreference = 'stop'`、末尾に `if ((Test-Path -LiteralPath variable:\LASTEXITCODE)) { exit $LASTEXITCODE }`）、`pwsh -NoProfile -NonInteractive` で回した。`GH_TOKEN` は設定せず、手元の `gh` の認証で読むだけの呼び出しをした。`GITHUB_OUTPUT` は場合ごとの空の一時ファイルを指した。一時ファイルは確かめの後に消した。

段を置く前に取り出しの道具を回すと「`既存の Release の検査` の段が 0 件」で失敗した（赤）。

### 一つ前のタグを決める関数の 6 通り

道具が `Get-PrevTag` の返す字と期待を文字どおり比べて判定した。

| タグの並び | 今の版 | 期待 | 返った値 | 判定 |
|---|---|---|---|---|
| `v0.0.1` | `0.0.2` | `v0.0.1` | `v0.0.1` | PASS |
| `v0.0.1`・`v0.0.2` | `0.0.2` | `v0.0.1` | `v0.0.1` | PASS |
| `v0.0.2` | `0.0.2` | 空 | 空 | PASS |
| `v`・`vfoo`・`v0.0.1` | `0.0.2` | `v0.0.1` | `v0.0.1` | PASS |
| `v0.0.1`・`v0.0.2`・`v0.0.3-rc.1` | `0.0.3` | `v0.0.3-rc.1` | `v0.0.3-rc.1` | PASS |
| `v0.0.1`・`v0.0.2` | `0.0.3-rc.1` | `v0.0.2` | `v0.0.2` | PASS |

較正: 切り出した関数の写しで比べの `-lt` を `-le` に変えると、2 行目（`v0.0.2` を返した）と 3 行目（`v0.0.2` を返した）の 2 通りが FAIL になった。`v` を除かずに読む写し（`TryParse($name, ...)`）では 5 通りが FAIL になった。どちらも写しだけを変え、`release.yml` は変えていない。

### 段の全体を手元の `gh` で回す

先に `gh api repos/ekicyou/areka/releases --jq length` で Release が 0 件、タグの一覧が `v0.0.1` の 1 本であることを確かめた。

| 場合 | リポジトリ | 始めた参照 | `VERSION` | `PUBLISH` | 印字 | 終了コード | `GITHUB_OUTPUT` |
|---|---|---|---|---|---|---|---|
| 本番の初回を写す | `ekicyou/areka` | タグ `v0.0.2` | `0.0.2` | `true` | `タグ 'v0.0.2' の Release は無い（下書きを含めて 0 件）`・`一つ前のタグ: 'v0.0.1'` | 0 | `absent=true`・`prev_tag=v0.0.1` |
| 枝で始めた乾いた走り | `ekicyou/areka` | 枝 `main` | `0.0.1` | `false` | `タグ 'v0.0.1' の Release は無い（下書きを含めて 0 件）`・`一つ前のタグ: ''` | 0 | `absent=true`・`prev_tag=`（空） |

1 行目が完了の形の「無い・`v0.0.1`」。

（追記・タスク 1.5 の差し戻し 1 回目）Release を探す行のタグの名前の比べを `-ceq` から `[string]::Equals(…, [StringComparison]::Ordinal)` に替えた（1 行のまま）。替えた後の本文で、1 行目（`ekicyou/areka`・タグ `v0.0.2`・本番）と、下の表の「本番で在る」（`cli/cli`・タグ `v2.101.0`）を回し直し、同じ結果だった。結果は 1.5 の「差し戻し 1 回目の直し」の節。

### 在る経路と取れない経路（ほかの公開リポジトリを読むだけで通した）

Release を作らずに「在る」経路を通すため、Release を持つ公開リポジトリ `cli/cli` を読むだけで回した。`cli/cli` は Release が 205 件・タグが 205 本で、どちらも 3 ページにまたがる（ページ送りが要る）。

| 場合 | リポジトリ | 始めた参照 | `PUBLISH` | 印字 | 終了コード | `GITHUB_OUTPUT` |
|---|---|---|---|---|---|---|
| 本番で在る | `cli/cli` | タグ `v2.101.0` | `true` | `タグ 'v2.101.0' の Release が既に在る（URL …/releases/tag/v2.101.0・下書き false）` | 1 | 空（`absent` を出さない） |
| 乾いた走りで在る | `cli/cli` | タグ `v2.101.0` | `false` | 同じ印字・`乾いた走りなので止めずに続ける`・`一つ前のタグ: 'v2.100.0'` | 0 | `prev_tag=v2.100.0`（`absent` は無い） |
| 本番で在る（3 ページ目にだけ在る最古の Release） | `cli/cli` | タグ `v0.4.0` | `true` | `タグ 'v0.4.0' の Release が既に在る（URL …/releases/tag/v0.4.0・下書き false）` | 1 | 空 |
| 一覧を取れない | 実在しないリポジトリ | タグ `v0.0.2` | `false` | `gh: Not Found (HTTP 404)`・`Release の一覧を取れない（gh api が終了コード 1）` | 1 | 空 |

`v2.100.0` が `v2.101.0` の一つ前であることは、タグの一覧（全ページ）を別に引いて確かめた。乾いた走りの「取れない」も失敗で止まる（設計の S5 の事後条件どおり）。

### 残り

- 下書きの Release が一覧に入ることは、Release を作らないので手元では確かめていない（設計の Risks のとおり `release-cycle` へ申し送る）。
- Release を探す行（`$found = @(gh api --paginate ... | Where-Object { ... -ceq $tag })`。タスク 1.5 の差し戻しで比べを `[string]::Equals(…, [StringComparison]::Ordinal)` に替えた）は後始末の段（タスク 1.6）でそのまま使う 1 行にしてある。2 か所が同じことの判定はタスク 3 で行う。

## 1.4 道具の用意（S6）と環境の記録（S7）の段

回した日: 2026-10-03。2 段は `既存の Release の検査` のすぐ後に、`道具の用意` → `環境の記録` の順で置いた（zip を作る段はこの後ろに来る）。

### 道具の取り込みの action の固定

| 項目 | 値 |
|---|---|
| 引いた日 | 2026-10-03 |
| 最新の版 | `v2.87.22` |
| 指すコミット | `83ac0ad63c0167e6f06796fab0fce28db1bf3db0` |
| タグの種類 | 軽量タグ（`git/ref/tags/v2.87.22` の `object.type` が `commit`）＝タグの SHA とコミットの SHA が同じ。`commits/v2.87.22` の `sha` も同じ値 |
| `release.yml` の行 | `uses: taiki-e/install-action@83ac0ad63c0167e6f06796fab0fce28db1bf3db0 # v2.87.22` |

引いた手順:

```
gh api repos/taiki-e/install-action/releases/latest --jq .tag_name
gh api repos/taiki-e/install-action/git/ref/tags/v2.87.22 --jq '.object.type+" "+.object.sha'
gh api repos/taiki-e/install-action/commits/v2.87.22 --jq .sha
```

### action の入力と道具の対応の確かめ（固定したコミットの中身を読むだけ）

`gh api repos/taiki-e/install-action/contents/<パス>?ref=83ac0ad63c0167e6f06796fab0fce28db1bf3db0` で読んだ。

| 確かめたこと | 読んだ物 | 結果 |
|---|---|---|
| 入力の名前が `tool` と `fallback` | `action.yml` の `inputs` | `tool`（必須・空白かカンマ区切り）・`checksum`（既定 `'true'`）・`fallback`（既定 `'cargo-binstall'`）の 3 つ。`tool` と `fallback` だけを渡し、`checksum` は既定の有効のまま |
| `fallback: none` が使える値 | `action.yml` の `fallback` の説明・`README.md` | 説明は「none, cargo-binstall, or cargo-install」。README に「fallback を使わないことを確かめたいなら `fallback: none`」の趣旨の記述と例が在る |
| `fallback: none` でトークンが渡らない | `action.yml` の `env` | `DEFAULT_GITHUB_TOKEN` は `fallback == 'cargo-binstall'` のときだけトークン、それ以外は空 |
| 2 つとも対応する道具の一覧に在り Windows を含む | `TOOLS.md` | `cargo-about`・`cargo-deny` とも置き場 `$CARGO_HOME/bin`・取り元は作者の GitHub Releases・Linux, macOS, Windows |
| `cargo-about` 0.9.2 の x64 Windows のビルド済み | `manifests/cargo-about.json` | `template.x86_64_windows` の URL は `cargo-about-${version}-x86_64-pc-windows-msvc.tar.gz`。`"0.9.2".x86_64_windows.hash` = `1c03e5890238562497c2d89a3b75b02560af349c1fc3e713d3284f532a5cd748` が在る |
| `cargo-deny` 0.20.2 の x64 Windows のビルド済み | `manifests/cargo-deny.json` | `template.x86_64_windows` の URL は `cargo-deny-${version}-x86_64-pc-windows-msvc.tar.gz`。`"0.20.2".x86_64_windows.hash` = `975a22143262fd27476d19ee00c7af67978426e40e1dee94eed6bbade1cf87dc` が在る |

段の形: `timeout-minutes: 10`・`with` は `tool: cargo-about@0.9.2,cargo-deny@0.20.2` と `fallback: none` の 2 つ・`env`（トークン）なし。キャッシュの action は使っていない。

### 静的な確かめ（機械で判定）

Python と PyYAML で `release.yml` を読む確かめの道具をワークツリーの `target\` の下に置き、段を置く前と後に回した。判定した項目: 2 段がちょうど 1 件ずつ在る・`既存の Release の検査` → `道具の用意` → `環境の記録` の順で隣り合う・S6 が `taiki-e/install-action`・`tool` の値が固定の 1 行・`fallback: none`・S6 の `with` が `tool` と `fallback` だけ・S6 の `timeout-minutes` が 10・S6 と S7 に `env` が無い・S6 の `with` と S7 の本文にトークンが無い・S7 に `timeout-minutes` が無い・S7 が `run` の段で `exit 0` を持つ・S7 が `vswhere` で ARM64 の部品を問う・ファイル全体の `uses:` がちょうど 2 件でどちらも 40 桁の SHA・キャッシュの action が 0 件・`fallback: none` の行が在る・どの段の本文にも `${{` が無い。

- 置く前（赤）: 「2 段がちょうど 1 件ずつ在る」「`uses:` がちょうど 2 件（1 件）」「`fallback: none` が在る」が FAIL、終了コード 1。
- 置いた後（緑）: 19 項目すべて PASS、終了コード 0。

### S7 の本文を手元で回す

本文の取り出し方は 1.2 と同じ（名前が `環境の記録` の段をちょうど 1 件取り、`run:` の本文を `target\` の下の一時ファイルへ書き、先頭に `$ErrorActionPreference = 'stop'`、末尾に `if ((Test-Path -LiteralPath variable:\LASTEXITCODE)) { exit $LASTEXITCODE }` を足して `pwsh -NoProfile -NonInteractive` で回す）。手元には `ImageOS`・`ImageVersion` が無いので、その行は「(無い)」が出るのが期待どおり。

| 場合 | 作り方 | 印字（要点） | 終了コード |
|---|---|---|---|
| (a) そのまま | 手元の環境のまま | イメージ `(無い)・版 (無い)`・`rustc 1.99.0 (b940084d7 2026-09-28)`・`cargo 1.99.0 (5f94df478 2026-08-27)`・`cargo-about 0.9.2`・`cargo-deny 0.20.2`・`gh version 2.102.0 (2026-09-30)`・arm64 のリンクの道具 `在る（C:\Program Files\Microsoft Visual Studio\2022\Professional）`・作業ドライブ `C:\` の空き容量 `187.8 GB` | 0 |
| (b) `cargo about`・`cargo deny` が無い | 子の `PATH` から `c:\rust\cargo\bin` を外してツールチェーンの `bin` を前に足し、`CARGO_HOME` を `target\` の下の空のフォルダへ向けた（cargo は在るが下位コマンドが見つからない） | `cargo about: (無い・終了コード 101)`・`cargo deny: (無い・終了コード 101)`。ほかは (a) と同じ | 0 |
| (c) コマンドそのものが無い | 子の `PATH` を pwsh の置き場と `C:\Windows\System32` だけにした | `rustc`・`cargo`・`cargo about`・`cargo deny`・`gh` がどれも「(無い: 用語 '…' は…認識されません…)」。arm64 と空き容量の行は (a) と同じ | 0 |
| (d) `vswhere.exe` が無い | 本文の写しで `VSWHERE_PATH` の値だけを `target\` の下の在らないパスへ替えた（手元の `PATH` に `vswhere` は無い） | `arm64 のリンクの道具: 無い（vswhere.exe が無い・探した場所: … と PATH）` | 0 |
| (e) ARM64 の部品が無い | 本文の写しで部品の名前だけを在らない名前へ替えた | `arm64 のリンクの道具: 無い（Microsoft.VisualStudio.Component.NoSuch.ARM64・vswhere: C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe・終了コード 0）` | 0 |

どの場合も段は止まらず、印字だけをして終了コード 0 で終わった（合否を付けない）。arm64 の問い方（`VSWHERE_PATH` の固定の置き場を先に見て、無ければ `PATH` の `vswhere`・`-latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.ARM64 -property installationPath`）は `tools/package.ps1` の前提の検査と同じ。一時ファイル（確かめの道具・本文の写し・空のフォルダ・読んだ manifest）は確かめの後に消した。

## 1.5 zip を作る段（S8）と 4 つの確かめの段（S9）

回した日: 2026-10-03。2 段は `環境の記録` のすぐ後に、`zip を作る` → `4 つの確かめ` の順で置いた。

段の形:

- S8 `zip を作る`: `timeout-minutes: 90`・`env` なし（トークン・`CARGO_TARGET_DIR`・`RUSTFLAGS` なし）。本文は、この段の出力の文字コードを UTF-8 にしたうえで、別の PowerShell のプロセスを `pwsh -NoProfile -NonInteractive -Command '[Console]::OutputEncoding = [Text.Encoding]::UTF8; & ./tools/package.ps1 -Arch all; exit $LASTEXITCODE'` で起こす（`-File` では呼ぶ前に文字コードを設定できないので `-Command`）。子の終了コードを `$code` に取り、`$host.SetShouldExit($code)` の後に `exit $code` で終わる。
- S9 `4 つの確かめ`: `timeout-minutes` なし・`env` は `VERSION: ${{ steps.version.outputs.version }}` の 1 つだけ（本文に `${{` は無い）。置き場は今いるフォルダからの `target/package`。4 つの名前を組み立てて在りかを確かめ、無ければ無い名前を全部印字して `exit 1`。各 zip の SHA256（`Get-FileHash` を小文字に）から期待の中身「16 進 64 字・空白 2 つ・zip のファイル名・LF」を組み、`.sha256` のバイト列を `[Text.Encoding]::UTF8.GetString` で字にし（BOM は外されず U+FEFF の字として残る）、`[string]::Equals(実際, 期待, [StringComparison]::Ordinal)` で字のとおりに丸ごと比べる（`-ceq` は文化に従い U+FEFF や U+200B を無視するので使わない）。違えば `{名前}.sha256 が {zip} と合わない（期待 '…'・実際 '…'）` を印字して `exit 1`。印字では空白から `~` までの外の字を、CR は `\r`・LF は `\n`・ほかは `\uXXXX`（BOM は `\uFEFF`）の形で見せる。揃えば 4 つの名前・各 zip の SHA256・作業ドライブの空き容量を印字して `exit 0`。期待の形は `tools/package.ps1` の段「SHA256」の書き方（`"{0}  {1}`n"`・BOM 無し・LF）に合わせた。

### 見つけたこと: GitHub の pwsh の段は本文の `exit 3` を 1 に変える

最初の形は S8 の最後を `exit $LASTEXITCODE` にしていた。GitHub の pwsh の段の包み方（`pwsh -command ". '{0}'"`・先頭に `$ErrorActionPreference = 'stop'`・末尾に `if ((Test-Path -LiteralPath variable:\LASTEXITCODE)) { exit $LASTEXITCODE }`）を写して回すと、子が `exit 3` で終わっても段は終了コード 1 で終わった。PowerShell の `-Command` は、最後に回したスクリプトが 0 と 1 以外の終了コードで終わると、プロセスの終了コードを 1 に変える（手元の pwsh 7.6.6 で、`exit 3` だけのファイルを `-Command ". 'ファイル'"` で回すと 1、`-File` で回すと 3 になることを確かめた）。末尾に足される行は本文の `exit` より後なので届かない。本文の中で `$host.SetShouldExit($code)` を呼んでから `exit $code` にすると、同じ包み方で 3 がそのまま返った。今の S8 はこの形。

### 静的な確かめ（機械で判定）

Python と PyYAML で `release.yml` を読む確かめの道具をワークツリーの `target\` の下に置き、段を置く前と後に回した。判定した項目: S8・S9 がちょうど 1 件ずつ在る・`環境の記録` → `zip を作る` → `4 つの確かめ` の順で隣り合う・S8 の `timeout-minutes` が 90・S9 に `timeout-minutes` が無い・S8 に `env` が無い・S9 の `env` が `VERSION: ${{ steps.version.outputs.version }}` だけ・S8 と S9 にトークンが無い・S8 が別の `pwsh` で UTF-8 にしてから配布スクリプトを呼ぶ・S8 が `$code = $LASTEXITCODE` → `$host.SetShouldExit($code)` → `exit $code` で終わる・S8 に `CARGO_TARGET_DIR`・`RUSTFLAGS` が無い・S9 が `target/package` を読む・ファイル全体で配布スクリプトの呼び出しが 1 件で引数が `-Arch all` だけ・`-Check` を付けた呼び出しが 0 件・`GH_TOKEN` を持つ段が `既存の Release の検査` だけ（この時点）・どの段の本文にも `${{` が無い。

- 置く前（赤）: 「S8 がちょうど 1 件」「S9 がちょうど 1 件」「配布スクリプトの呼び出しが 1 件で引数が `-Arch all` だけ」が FAIL、終了コード 1。
- 置いた後（緑）: 16 項目すべて PASS、終了コード 0。

### S8 の本文を手元で回す（本物の配布スクリプト）

本文の取り出し方は 1.2 と同じ（名前が `zip を作る` の段をちょうど 1 件取り、`run:` の本文を `target\` の下の一時ファイルへ書き、先頭に `$ErrorActionPreference = 'stop'`、末尾に `if ((Test-Path -LiteralPath variable:\LASTEXITCODE)) { exit $LASTEXITCODE }` を足す）。ワークツリーの根から `pwsh -NoProfile -NonInteractive -Command ". '<一時ファイル>'"` で回し、印字はファイルへ流した（実行環境と同じく、端末ではなく流し先へ書く形）。回している間はファイルを書き換えていない（配布スクリプトの段「git status 不変の確認」が緑）。

| 回 | 本文 | 終了コード | 所要時間 | 印字（要点） |
|---|---|---|---|---|
| 1 回目（冷えたビルド） | 最初の形（最後が `exit $LASTEXITCODE`） | 0 | 860 秒 | `OK 前提の確認` から `OK 完成` までの 20 段がすべて緑・`全段 緑`。段の時間は i686 helper ビルド 141 秒・x64 本体ビルド 355 秒・arm64 本体ビルド 294 秒・謝辞の生成 48 秒。`zip:`・`sha256:`・`version: 0.0.1` を x64 と arm64 の 2 組 |
| 2 回目（ビルド済み） | 今の形（`$host.SetShouldExit` 入り） | 0 | 36 秒 | 同じく 20 段がすべて緑・`全段 緑` |

どちらの回も、段の名前（`前提の確認`・`x64 本体ビルド`・`謝辞の生成` など）と `全段 緑` が UTF-8 のまま読めた（化けなし）。手元のビルドには `CARGO_TARGET_DIR`・`RUSTFLAGS` を設定していない。

### S9 の本文を手元で回す

本文の取り出し方は S8 と同じ（名前が `4 つの確かめ` の段）。`VERSION=0.0.1` を環境変数で渡した。写しは `target\rc15\root\target\package\` に作り、その都度 `target\rc15\root` へ移ってから回した（本文は今いるフォルダからの `target/package` を読むので、本文を変えずに写しを読ませられる）。

S8 の 2 回目が作った 4 つに対して:

```
揃った: areka-0.0.1-x64.zip（SHA256 3e4723939056cec63a3293440b88b86d136b492f9a17f0a3c311a9cae24561c9）
揃った: areka-0.0.1-x64.zip.sha256
揃った: areka-0.0.1-arm64.zip（SHA256 e5784caf68a05eedf326c2d4d574fa5e54a4ec0442891511aff496de2cc7d271）
揃った: areka-0.0.1-arm64.zip.sha256
作業ドライブ C:\ の空き容量: 181.8 GB
```

終了コード 0。別の道具でも、`target\package` で `sha256sum -c areka-0.0.1-x64.zip.sha256 areka-0.0.1-arm64.zip.sha256` が 2 つとも `OK` だった。1 回目のビルドの 4 つでも同じく揃った（SHA256 は x64 `6d15185366dd4b61779e307cdf12a8f41135822cf72743e492ba25d3ccb09cfc`・arm64 `a54171e72c55df1ead751d7ed9bfdc9c0afad5878fec3bf6ed979ddaf0296922`。作るたびに zip の中の時刻などが変わるので回ごとに値は違う）。

写しで回した場合（判定は道具が終了コードと印字の字で機械でした）:

| 場合 | 写しの作り方 | 期待 | 終了コード | 印字（要点） | 判定 |
|---|---|---|---|---|---|
| 写しのまま | 4 つを写しただけ | 成功 | 0 | 4 つの `揃った:` と空き容量 | PASS |
| `.sha256` のハッシュの 1 字 | arm64 の `.sha256` の先頭の字 `e` を `0` に | 失敗 | 1 | `areka-0.0.1-arm64.zip.sha256 が areka-0.0.1-arm64.zip と合わない（期待 'e5784caf…d271  areka-0.0.1-arm64.zip\n'・実際 '05784caf…d271  areka-0.0.1-arm64.zip\n'）` | PASS |
| `.sha256` のファイル名の 1 字 | x64 の `.sha256` の `-x64.zip` を `-x65.zip` に（ハッシュは正しいまま） | 失敗 | 1 | `areka-0.0.1-x64.zip.sha256 が areka-0.0.1-x64.zip と合わない（期待 '…  areka-0.0.1-x64.zip\n'・実際 '…  areka-0.0.1-x65.zip\n'）` | PASS |
| 改行が CRLF | x64 の `.sha256` の LF を CRLF に | 失敗 | 1 | 実際の側の末尾が `\r\n` | PASS |
| ファイルの欠け | arm64 の zip を消す | 失敗 | 1 | `無い: areka-0.0.1-arm64.zip（置き場 …\target\rc15\root\target\package）` | PASS |

ハッシュの 1 字の場合が完了の形の「不一致で失敗」、本物の 4 つの場合が「揃った」。

### S8 の終了コードの伝わり方（偽の配布スクリプト）

`target\rc15\stub\tools\package.ps1` に、受けた引数を印字して決まった終了コードで終わる偽物を置き、`target\rc15\stub` へ移って S8 の本文（同じ包み方）を回した。`tools\` の本物には触れていない。

| 偽物の終了コード | 段の終了コード | 印字 | 判定 |
|---|---|---|---|
| 3 | 3 | `引数: 0・Arch=all`（余りの引数 0 個・`-Arch` は `all`）・偽物の 1 行 | PASS |
| 1 | 1 | 同上・`FAIL ビルド（偽）` | PASS |
| 2 | 2 | 同上 | PASS |
| 0 | 0 | 同上 | PASS |

較正: 最初の形（最後が `exit $LASTEXITCODE`）で同じ道具を回すと、偽物が 3 のとき段が 1 で終わり FAIL になった（上の「見つけたこと」）。偽物が 1 の場合は最初の形でも 1 だった。

一時ファイル（確かめの道具・本文の写し・写しの根・偽物・印字の流し先）は確かめの後に消した。配布スクリプトが作った `target\package\` の 4 つは残した（追跡外）。

### 差し戻し 1 回目の直し（字のとおりの比べ）

レビューの指摘: PowerShell の `-ceq` は文化に従って比べ、U+FEFF（BOM）と U+200B（幅のない空白）を無視する（`"x"+U+FEFF -ceq "x"` も `"a"+U+200B+"b" -ceq "ab"` も真）。そのため最初の S9 は、先頭に BOM の付いた `.sha256` や、ファイル名に U+200B の入った `.sha256` を「揃った」と通していた（`sha256sum -c` はどちらも通さない）。上の「段の形」の最初の版にあった「BOM も字として残る…丸ごと比べる」という説明は、字は残っても比べが無視していたので誤りだった。同じ弱さが S4 のタグの版の比べと、S5 の Release を探す行にもあった。

直したこと（`release.yml` だけ）:

- S9: 比べを `[string]::Equals($actual, $expected, [StringComparison]::Ordinal)` に替えた。印字の見せ方を、空白から `~` までの外の字すべてを `\r`・`\n`・`\uXXXX` で見せる形に広げた。
- S4: `$tagVersion -ceq $version` を `[string]::Equals($tagVersion, $version, [StringComparison]::Ordinal)` に替えた。
- S5: Release を探す行の `$_.Split("`t")[0] -ceq $tag` を `[string]::Equals($_.Split("`t")[0], $tag, [StringComparison]::Ordinal)` に替えた（1 行のまま。後始末の段でそのまま写す）。
- S8: 説明の行の「起動確認の -Check は付けない」を「起動確認の引数は付けない」にした（ファイルの中の `-Check` を 0 件にするため）。本文は変えていない。

残した `-ceq` は 4 か所（S4 の `$_.name -ceq 'areka'`・`$env:GITHUB_REF_TYPE -ceq 'tag'`、S5 の `$env:GITHUB_REF_TYPE -ceq 'tag'`・`$env:PUBLISH -ceq 'true'`）。どれも cargo か GitHub が決めた値を固定の語と比べるだけの行で、利用者が付けた名前（タグ）や配布物の中身は比べていない。

回し直し（本文の取り出し方と包み方は 1.2 と同じ。判定は道具が、終了コード・印字の字（Ordinal で探す）・`GITHUB_OUTPUT` の中身で機械でした）。較正のため、S4 と S5 は `git show HEAD:.github/workflows/release.yml` から取った直す前の本文でも回した。S9 は、直した本文の比べの行だけを `($actual -ceq $expected)` に戻した写しでも回した:

| 段 | 場合 | 本文 | 期待 | 終了コード | 印字（要点）・`GITHUB_OUTPUT` | 判定 |
|---|---|---|---|---|---|---|
| S4 | タグ `v0.0.1` | 直した後 | 成功 | 0 | 一致（'0.0.1'）・`version=0.0.1` | PASS |
| S4 | タグ `v0.0.2` | 直した後 | 失敗 | 1 | `0.0.2` と `0.0.1` の両方・空 | PASS |
| S4 | タグ `v` | 直した後 | 失敗 | 1 | 空の版と `0.0.1`・空 | PASS |
| S4 | タグ `v0.0.1-x` | 直した後 | 失敗 | 1 | `0.0.1-x`・空 | PASS |
| S4 | 枝 `main` | 直した後 | 成功 | 0 | タグが無いので比べを飛ばす・`version=0.0.1` | PASS |
| S4 | タグ `v0.0.1`＋U+200B | 直した後 | 失敗 | 1 | `タグの版と Cargo.toml の版が違う（タグの版 '0.0.1'＋U+200B・Cargo.toml の版 '0.0.1'）`・空 | PASS |
| S4 | タグ `v0.0.1`＋U+200B | 直す前（較正） | 誤って成功 | 0 | 一致（'0.0.1'）・`version=0.0.1` | 弱さを再現 |
| S5 | `ekicyou/areka`・タグ `v0.0.2`・本番 | 直した後 | 無い | 0 | `タグ 'v0.0.2' の Release は無い（下書きを含めて 0 件）`・`一つ前のタグ: 'v0.0.1'`・`absent=true`・`prev_tag=v0.0.1` | PASS |
| S5 | `cli/cli`・タグ `v2.101.0`・本番 | 直した後 | 在る | 1 | `タグ 'v2.101.0' の Release が既に在る（URL https://github.com/cli/cli/releases/tag/v2.101.0・下書き false）`・空 | PASS |
| S5 | `cli/cli`・タグ `v2.101.0`＋U+200B・本番 | 直した後 | 無い | 0 | `の Release は無い（下書きを含めて 0 件）`・`一つ前のタグ: 'v2.100.0'`・`absent=true`・`prev_tag=v2.100.0` | PASS |
| S5 | `cli/cli`・タグ `v2.101.0`＋U+200B・本番 | 直す前（較正） | 誤って在る | 1 | `の Release が既に在る（URL …/v2.101.0…）`・空 | 弱さを再現 |
| S9 | 本物の 4 つ（S8 の 2 回目が作った物） | 直した後 | 成功 | 0 | 4 つの `揃った:`（x64 `3e472393…61c9`・arm64 `e5784caf…d271`）・空き容量 | PASS |
| S9 | x64 の `.sha256` の先頭に BOM（EF BB BF・書いた後にバイト列で確かめた） | 直した後 | 失敗 | 1 | `areka-0.0.1-x64.zip.sha256 が areka-0.0.1-x64.zip と合わない（期待 '3e472393…61c9  areka-0.0.1-x64.zip\n'・実際 '\uFEFF3e472393…61c9  areka-0.0.1-x64.zip\n'）` | PASS |
| S9 | 同上 | `-ceq` の写し（較正） | 誤って成功 | 0 | 4 つの `揃った:` | 弱さを再現 |
| S9 | arm64 の `.sha256` のファイル名に U+200B（`-arm64`＋U+200B＋`.zip`） | 直した後 | 失敗 | 1 | `… 実際 'e5784caf…d271  areka-0.0.1-arm64\u200B.zip\n'）` | PASS |
| S9 | 同上 | `-ceq` の写し（較正） | 誤って成功 | 0 | 4 つの `揃った:` | 弱さを再現 |
| S9 | ハッシュの 1 字・ファイル名の 1 字・CRLF・arm64 の zip の欠け・写しのまま | 直した後 | 失敗×4・成功×1 | 1・1・1・1・0 | 上の表と同じ印字 | PASS |

直した後の本文の場合（17 通り）はすべて PASS。直す前の本文と `-ceq` の写しの 4 通りは、どれも U+FEFF か U+200B を無視して通した（弱さの再現＝道具が違いを見分けられることの較正）。S4 の印字の中の U+200B は見えないまま出る（S4 の印字は字をそのまま出す）。

直した後の `release.yml` への静的な確かめ（機械で判定）: ファイルの中の `-Check` が 0 件・配布スクリプトの呼び出しが 1 件で引数が `-Arch all` だけ・S4 と S9 の比べが Ordinal・S5 の探す行が Ordinal の 1 行・S9 に `-ceq` が無い・ファイルの中の U+FEFF と U+200B が 0 件・どの段の本文にも `${{` が無い・S8 の上限が 90 で S9 は上限なし・`GH_TOKEN` を持つ段が `既存の Release の検査` だけ。9 項目すべて PASS。YAML として読める。一時ファイルは確かめの後に消した。

## 1.6 Release の公開の段（S10）と後始末の段（S11）

回した日: 2026-10-03。`4 つの確かめ` の後ろに 2 段を足した。

- `Release を公開`（S10）: 条件 `env.PUBLISH == 'true'`・`timeout-minutes: 10`・`env:` に `GH_TOKEN`・`VERSION`（S4 の版）・`PREV_TAG`（S5 の一つ前のタグ）。本文は `target/package` の 4 つ（x64 と arm64 の zip と `.sha256`）を `gh release create v{版}` に渡し、`--repo $env:GITHUB_REPOSITORY`・`--verify-tag`・`--generate-notes`・`--title v{版}` を付け、`PREV_TAG` が空でないときだけ `--notes-start-tag` を足す。gh の終了コードを `$host.SetShouldExit` を通してそのまま段の終了コードにする（gh は 0・1 のほかに 2・4 を返しうるため）。
- `後始末`（S11）: 条件 `(failure() || cancelled()) && env.PUBLISH == 'true' && steps.guard.outputs.absent == 'true'`・`timeout-minutes: 5`・`env:` に `GH_TOKEN` だけ。タグは `GITHUB_REF_NAME`。Release を探す行は S5 の行を道具で 1 字違わず写した（手で打っていない）。一覧を取れなければ「残っているおそれ」と手で確かめる先（Release の一覧 `https://github.com/{リポジトリ}/releases`）を印字して 1、在れば Release の番号を名指しして `gh api -X DELETE repos/{リポジトリ}/releases/{番号}` で消す（タグは消さない・`gh release delete` は使わない）。消せないものが 1 つでもあれば同じ印字で 1、無ければ `タグ '…' の Release は残っていない（下書きを含めて 0 件）` と印字して 0。

`gh release create --help`（手元の gh 2.102）で `--verify-tag`・`--notes-start-tag`・`--generate-notes`・`--title`・`--repo` の綴りを確かめた。

### 静的な確かめ（機械で判定）

Python と PyYAML で `release.yml` を読み、30 項目を判定させた。段を足す前に回すと 20 項目が FAIL（10/30 PASS・終了コード 1）で、足した後は 30/30 PASS（終了コード 0）。

| 項目 | 足す前 | 足した後 |
|---|---|---|
| YAML として読める・U+FEFF と U+200B が 0 件・改行が LF だけ | PASS | PASS |
| 最後の 2 段が `Release を公開`・`後始末` の順 | FAIL | PASS |
| S10 の条件が `env.PUBLISH` を見る | FAIL | PASS |
| S10 の本文に `gh release create`・`--verify-tag`・`--generate-notes`・`--notes-start-tag`・`--repo`・`--title` | FAIL×6 | PASS×6 |
| ファイルの中の `--draft`・`--target`・`--latest` が 0 件 | PASS×3 | PASS×3 |
| S11 の条件に `failure()`・`cancelled()`・`env.PUBLISH`・`steps.guard.outputs.absent` | FAIL×4 | PASS×4 |
| `$found = @(gh api --paginate` で始まる行が S5 と S11 にちょうど 1 行ずつ | FAIL | PASS |
| その 2 行が `==` で同じ | FAIL | PASS |
| S11 が `gh api -X DELETE` で消し、`gh release delete` を持たない | FAIL | PASS |
| `GH_TOKEN` を持つ段が S5・S10・S11 の 3 つだけ（ファイルの中の `GH_TOKEN:` も 3 件） | FAIL×2 | PASS×2 |
| S10 の上限 10・S11 の上限 5 | FAIL×2 | PASS×2 |
| 段の上限の合計が 130（S3 10・S5 5・S6 10・S8 90・S10 10・S11 5）で job の 150 より小さい | FAIL（115） | PASS（130） |
| どの段の本文にも `${{` が無い | PASS | PASS |
| `git push`・`git tag`・`git remote` が 0 件 | PASS | PASS |
| `repository_dispatch`・`gh workflow run`・`workflow_call` が 0 件 | PASS | PASS |
| `secrets.` が 0 件 | PASS | PASS |

較正: `release.yml` の写しで S11 の探す行の `@tsv` を `@csv` に変えると、「2 行が同じ」だけが FAIL（29/30）になった。

### S11 の本文を手元の `gh` で 1 回回す（読むだけ）

先に `gh api repos/ekicyou/areka/releases --jq length` が `0`（Release が 0 件）であることを確かめ、消す経路が動きようのない状態で回した。本文の取り出し方と包み方は 1.2・1.3 と同じ（Python と PyYAML で `後始末` の段の本文を取り、先頭に `$ErrorActionPreference = 'stop'`、末尾に `if ((Test-Path -LiteralPath variable:\LASTEXITCODE)) { exit $LASTEXITCODE }` を付けて `pwsh -command ". '{一時ファイル}'"` で回す）。環境変数は `GITHUB_REF_NAME=v0.0.2`・`GITHUB_REPOSITORY=ekicyou/areka`。`GH_TOKEN` は設定せず手元の `gh` の認証で読んだ。

| 印字 | 終了コード | 回した後の Release の数 |
|---|---|---|
| `タグ 'v0.0.2' の Release は残っていない（下書きを含めて 0 件）` | 0 | 0 |

完了の形の「残っていない」と印字して終わった。何も消えていない。

### S10・S11 の分かれ道を偽の `gh` で回す（GitHub に触れない）

本文の前に PowerShell の関数 `gh` を置いて本物の gh を隠した（関数はアプリより先に引かれる）。偽物は受けた引数を 1 要素ずつ `target\rc16\` の記録ファイルへ書き、呼び方に応じて決めた行と終了コード（`$global:LASTEXITCODE`）を返す。本文と包み方は上と同じ。道具が終了コード・印字の字・記録した呼び出しを期待と比べて判定した。

| 段 | 場合 | 期待 | 終了コード | 記録した呼び出し・印字（要点） | 判定 |
|---|---|---|---|---|---|
| S11 | 一覧に `v0.0.1`（番号 99）と `v0.0.2`（番号 111・下書き） | 111 だけ消して成功 | 0 | 一覧の 1 回の後に `api -X DELETE repos/ekicyou/areka/releases/111` の 1 回・`Release を消した（番号 111…）。タグ 'v0.0.2' は残す` | PASS |
| S11 | `v0.0.2` が 2 件（111 下書き・112 公開）と `v0.0.1` | 2 件とも消して成功 | 0 | `…/releases/111`・`…/releases/112` の 2 回 | PASS |
| S11 | 一覧に `v0.0.1` だけ | 消さずに成功 | 0 | 一覧の 1 回だけ・`残っていない` | PASS |
| S11 | `v0.0.2`（111）が在り、消す呼び出しが終了コード 1 | 失敗 | 1 | `Release を消せない（番号 111…・gh api が終了コード 1）`・`タグ 'v0.0.2' の Release が残っているおそれがある。手で確かめる先: Release の一覧 https://github.com/ekicyou/areka/releases` | PASS |
| S11 | 一覧の呼び出しが終了コード 1 | 失敗・消さない | 1 | 一覧の 1 回だけ・`Release の一覧を取れない（gh api が終了コード 1）`・同じ「残っているおそれ」の印字 | PASS |
| S10 | `VERSION=0.0.2`・`PREV_TAG=v0.0.1`・gh が 0 | 成功 | 0 | `release create v0.0.2 {置き場}/areka-0.0.2-x64.zip {…}.zip.sha256 {…}/areka-0.0.2-arm64.zip {…}.zip.sha256 --repo ekicyou/areka --verify-tag --generate-notes --title v0.0.2 --notes-start-tag v0.0.1`（要素ごとに完全一致） | PASS |
| S10 | `PREV_TAG` が空・gh が 0 | `--notes-start-tag` を付けない | 0 | 上から `--notes-start-tag v0.0.1` を除いた並びと完全一致 | PASS |
| S10 | gh が 1 | 失敗 | 1 | `Release を公開できない（gh release create が終了コード 1）` | PASS |
| S10 | gh が 4 | gh の値のまま | 4 | `…終了コード 4）` | PASS |

9 通りすべて PASS。置き場は作業ツリーの根からの `target/package`（S9 と同じ）。

較正（本文の写しだけを変え、`release.yml` は変えていない。どれも上の 9 通りの道具で FAIL が出た＝変異を見分けた）:

| 写しへの変え方 | FAIL になった場合 |
|---|---|
| S10 の `if ($env:PREV_TAG)` を `if ($true)` に | `PREV_TAG` が空の場合 |
| S10 の `$host.SetShouldExit($code)` を消す | gh が 4 の場合（段が 1 で終わった） |
| S11 の消す番号を `$f[1]` から `$f[0]`（タグの名前）に | 見つかった 2 つの場合 |
| S11 の一覧を取れないときの `exit 1` を `exit 0` に | 一覧の呼び出しが 1 の場合 |

一時ファイル（判定の道具・本文の写し・偽の gh の記録・変えた `release.yml` の写し）は `target\rc16\` の下にだけ置き、確かめの後に消した。

### 残り

- S10 が実際に Release を公開すること（gh の下書き→4 つの添付→公開・`v0.0.1` からの自動のノート）と、S11 の消す経路（止められた gh の下書きや公開済みの Release を番号で消すこと・下書きがトークンから見えること）は、実行環境での走り（乾いた走り）でも通らない。初めて動くのは `release-cycle` の初回の実走で、そこへ申し送る。
- 偽の `gh` は PowerShell の関数なので、本物の gh へ渡るときの引数の引用（Windows のコマンド行への組み立て）は確かめていない。渡す値はどれも空白や引用符を含まない（作業ツリーの絶対パス・`v0.0.2`・`ekicyou/areka` の形）。

## 3 静的な確かめ

設計の Testing Strategy の「静的な確かめ」1〜7 と 9 に、時間の上限の合計（a）と Release を探す行の一致（b）を足し、検索で数を数えて合否を出す道具（Python 3.13＋PyYAML）で判定した。8（走らせたコミットとの差）はタスク 4.2 で判定する。

| 項目 | 値 |
|---|---|
| 確かめた日 | 2026-10-03 |
| 確かめたコミット（HEAD） | `2f0858d206f8226f24013d6fa8e31d22b96dd749` |
| main との分かれ目（`git merge-base main HEAD`） | `d4f9e93daf9d3d10918d6590f0fd58ff0a1745d7` |
| 対象 | `git show HEAD:.github/workflows/release.yml` の写し（作業ツリーのファイルと `cmp` で一致・作業ツリーは未変更 0） |
| 判定の道具の結果 | 40 件中 PASS 40・FAIL 0・終了コード 0 |

### 判定の道具の要点

- YAML を PyYAML で読み、`on`（PyYAML では真偽値 `True` の鍵になるので両方を見る）・`permissions`・`jobs`・各段の `name`・`env`・`if`・`timeout-minutes`・`with` を構造として比べる。
- 「件数 0」の検索はファイル全体（注記の行を含む）に対して数える＝注記に書いただけでも FAIL になる、厳しい側の数え方。
- 「在る」を確かめる検索（配布スクリプトの呼び出し・S10 の引数）は、`#` で始まる注記の行を除いて数える。注記の行に同じ語が在ると、本文から消えても数が残るため（較正で実際に見つけた。下の「較正で直した数え方」）。
- 9 は `git diff --name-only main...HEAD` の全パスを `^crates/`・`^tools/`・`(^|/)Cargo\.(toml|lock)$`・`^dist/README\.txt$` で数え、加えて `git diff --stat main...HEAD -- crates tools Cargo.toml Cargo.lock ':(glob)**/Cargo.toml' dist/README.txt` の印字が空かを見る。
- 各項目を `PASS`／`FAIL` で印字し、1 つでも FAIL なら終了コード 1。

### 結果

| 確かめ | 判定の中身 | 数えた値 | 合否 |
|---|---|---|---|
| 1a | `on` の鍵が `push` と `workflow_dispatch` の 2 つだけ | `['push', 'workflow_dispatch']` | PASS |
| 1b | `push` の下が `tags: ['v*']` だけ | `{'tags': ['v*']}` | PASS |
| 1c | `branches` の件数（ファイル全体） | 0 | PASS |
| 1d | `workflow_dispatch` に入力が無い | 空（`None`） | PASS |
| 2a | 頭の `permissions` が `contents: write` だけ | `{'contents': 'write'}` | PASS |
| 2b | job の `permissions` の件数 | 0 | PASS |
| 2c | `secrets.` の件数 | 0 | PASS |
| 2d | `git remote` の件数 | 0 | PASS |
| 2e | `git push` の件数 | 0 | PASS |
| 2f | `git tag` の件数 | 0 | PASS |
| 2g | `env` に `GH_TOKEN` を持つ段 | 既存の Release の検査・Release を公開・後始末（この順の 3 つ） | PASS |
| 2h | `GH_TOKEN:` の件数（ファイル全体） | 3 | PASS |
| 2i | 取り出しの段の `persist-credentials` | 1 段・`false` | PASS |
| 3a | `uses:` の件数 | 2 | PASS |
| 3b | `@` の後が 40 桁の小文字 16 進の `uses:` の件数 | 2（2 件中 2 件） | PASS |
| 3c | `cache` を名に含む action の件数 | 0 | PASS |
| 3d | `fallback: none` の件数 | 1 | PASS |
| 4z | 注記でない行のうち `package.ps1` を含む行の件数 | 1 | PASS |
| 4a | 配布スクリプトの呼び出し（`& …package.ps1`）の件数 | 1 | PASS |
| 4b | 呼び出しの引数 | `-Arch all` だけ | PASS |
| 4c | `-Check` の件数（ファイル全体） | 0 | PASS |
| 5a | job の数 | 1 | PASS |
| 5b | `runs-on:` の件数 | 1 | PASS |
| 5c | job の `timeout-minutes` | 150 | PASS |
| 5d | S3・S5・S6・S8・S10・S11 の `timeout-minutes` | Rust の固定 10・既存の Release の検査 5・道具の用意 10・zip を作る 90・Release を公開 10・後始末 5（6 段とも在る） | PASS |
| a | 全段の `timeout-minutes` の合計 < job の値 | 130 < 150 | PASS |
| 6a | S10 の `if` に `PUBLISH` | `env.PUBLISH == 'true'` | PASS |
| 6b | S10 の本文（注記の行を除く）の `--verify-tag` | 1 | PASS |
| 6c | 同じく `--generate-notes` | 1 | PASS |
| 6d | 同じく `--notes-start-tag` | 1 | PASS |
| 6e | 同じく `--repo` | 1 | PASS |
| 6f | `--draft` の件数（ファイル全体） | 0 | PASS |
| 6g | S10 の後ろの段 | 後始末だけ | PASS |
| 6h | S11 の `if` に `failure()`・`cancelled()`・`PUBLISH`・`steps.guard.outputs.absent` | 4 つとも在る | PASS |
| b | S5 と S11 の `$found = @(gh api --paginate` で始まる行 | S5 に 1 行・S11 に 1 行・バイト列が同じ | PASS |
| 7a | `workflow_call` の件数 | 0 | PASS |
| 7b | `repository_dispatch` の件数 | 0 | PASS |
| 7c | `gh workflow run` の件数 | 0 | PASS |
| 9a | `main...HEAD` の変更のうち `crates/`・`tools/`・各 `Cargo.toml`・`Cargo.lock`・`dist/README.txt` の件数 | 0（変更は `.github/workflows/release.yml` と `.kiro/` の下（この spec の文書・`tech.md`・`structure.md`・後段 2 本の brief）の計 12 ファイル） | PASS |
| 9b | `git diff --stat main...HEAD -- crates tools Cargo.toml Cargo.lock ':(glob)**/Cargo.toml' dist/README.txt` の行数 | 0 | PASS |

### 較正（9 の 0 が意味を持つこと）

同じ数え方を、該当のパスに触れたと分かっている main の範囲に向けて、0 でない数が出ることを確かめた。

| 範囲 | 結果 |
|---|---|
| `main~5 main`（9a と同じ正規表現） | 113 件（`crates/` 111・`tools/` 1・`Cargo.lock` 1。`:(glob)**/Cargo.toml` は `crates/areka-mcp/Cargo.toml`・`crates/areka/Cargo.toml` を拾う） |
| `main~5 main`（9b と同じ pathspec の `--stat`） | `113 files changed, 11333 insertions(+), 3945 deletions(-)` |
| `03e8d7d6~1 03e8d7d6`（同じ pathspec） | `dist/README.txt` を含む 6 件 |
| `73a6c70d~1 73a6c70d`（同じ pathspec） | `tools/package.ps1` を含む 4 件 |

`Cargo.toml`・`Cargo.lock`・`dist/README.txt` は `git ls-files` で実在する。

### 較正（判定の道具が変異を見分けること）

HEAD の `release.yml` の写しに 1 か所ずつ変異を入れ、道具が終了コード 1 と、狙った確かめの FAIL を出すことを確かめた（9 は変異と無関係なので外して回した）。24 通りすべてで狙いどおりに FAIL が出た。

| 写しへの変え方 | FAIL になった確かめ |
|---|---|
| `push` に `branches: [main]` を足す | 1b・1c |
| `on` に `pull_request:` を足す | 1a |
| `workflow_dispatch` に入力を足す | 1d |
| `GH_TOKEN` を `secrets.GITHUB_TOKEN` に | 2c |
| job に `permissions` を足す | 2b |
| 改行の設定の段に `git push origin HEAD` を足す | 2e |
| 環境の記録の段に `GH_TOKEN` を足す | 2g・2h |
| `persist-credentials: false` を消す | 2i |
| `actions/cache@{40 桁}` の段を足す | 3a・3b・3c |
| 取り出しを `actions/checkout@v4` に | 3b |
| `fallback: none` を消す | 3d |
| 配布スクリプトの引数に `-Check` を足す | 4b・4c |
| job の `timeout-minutes` を消す | 5c・a |
| S3 の `timeout-minutes` を消す | 5d |
| S8 の上限を 90 から 140 に | a |
| S10 の `if` を `success()` に | 6a |
| S10 の本文から `'--verify-tag', ` を消す | 6b |
| S10 の本文から `--repo` とその値を消す | 6e |
| S10 の本文に `'--draft'` を足す | 6f |
| 後始末の後ろに段を足す | 6g |
| S11 の `if` から `cancelled()` を外す | 6h |
| S11 の探す行の `per_page=100` を `per_page=10` に（1 字） | b |
| `gh workflow run next` を足す | 7c |
| `on` に `repository_dispatch:` を足す | 1a・7b |

### 較正で直した数え方

- 初めの版の道具は、`package.ps1` を含む行をすべて呼び出しと数え、注記の行（`tools/package.ps1 と同じ` など）まで拾って 4a が 5 件の FAIL になった。注記の行を除き、`& …package.ps1` の形だけを呼び出しと数えるよう道具を直した（`release.yml` は変えていない）。
- `runs-on:` を行頭で数える検索に複数行の指定が抜けていて 5b が 0 件の FAIL になった。道具を直した。
- 6b は初め「S10 の本文に 1 件以上」で数えていたため、本文から `--verify-tag` を消しても注記の行（`--verify-tag で gh が止まる`）の 1 件が残って PASS のままだった（変異を見分けなかった）。注記の行を除いてちょうど 1 件と数えるよう 6b〜6e を直し、変異が FAIL になることを確かめた。

一時ファイル（判定の道具・変異の道具・写し・印字）は `target\rc3\` の下にだけ置き、確かめの後に消した。道具はリポジトリに加えていない。
