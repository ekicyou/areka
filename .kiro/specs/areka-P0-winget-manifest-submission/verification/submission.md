# 提出の記録: winget-pkgs へ `Areka.Areka.Portable` を初めて出す

- 日付: 2026-10-10（この記録を作った日。時刻はどれも +09:00）
- 機械: 開発機（Windows 11 Pro・x64）・winget v1.29.380
- コミット: `1c1ad468`（この記録を作ったときの枝の先頭。枝は `claude/areka-p0-winget-manifest-1c4fd7`）
- 提出する版: `0.0.2`（Release `v0.0.2`。決め方は手元の確かめの記録 `winget-local-check.md` の「ハッシュの突き合わせ」の 1）
- この記録は、手順と、提出の直前の確かめと、道具の用意までを先に書いてある（タスク 4.1）。提出そのものはまだ行っていない。

## 手順

初回の提出の手順（要件 5.1）。AI にできる操作は AI が行い、開発者の手に残すのは、本人のログイン・同意が要る操作だけ。開発者が打つコマンドは、下にそのまま書いてある。

| 番号 | 何をするか | 誰が |
|---|---|---|
| 1 | 前提を確かめる | AI |
| 2 | 提出の直前の確かめ（読むだけ） | AI |
| 3 | 提出の道具 `wingetcreate` を入れる | AI（止められたら開発者） |
| 4 | 提出のコマンドを打ち、ブラウザで GitHub へログインして許可する | 開発者 |
| 5 | PR の頁で、同意（CLA）と確かめ項目の印を済ませる | 開発者 |
| 6 | 道具が覚えたログインを消す | 開発者 |
| 7 | 審査の人に問われたら、用意した文で答える | 開発者（文は AI が用意済み） |
| 8 | 直しを求める印が付いたら、10 日のうちに応える | 開発者（印は AI が読んで知らせる） |

### 1. 前提（AI が確かめる）

- 提出するマニフェストそのもので入れて、`areka` の 1 語で起動する確かめが合格していること（タスク 2.2）。
- 上げ直しの実測・外し方の実測・機械の全員向けの実測の記録が済んでいること（タスク 3.2・3.3・3.5）。実測で利用者の物が「消える」と分かっていても、提出は 1 回だけ進める。
- winget の設定 `LocalManifestFiles` が元へ戻っていること（提出にこの設定は要らない）。
- 提出する版が決まっていること。今は `0.0.2`。`gh release list --repo ekicyou/areka` を読んで、より新しい Release が出ていたら、どの版で出すかを開発者が決め、雛形のフォルダの名前と中の版をその版にする。
- この記録を書いた時点では、上の 1 つ目と 2 つ目はまだ満たしていない（この手順書と道具の用意を先に済ませた）。満たしたことは、提出の回（タスク 4.2）に AI が確かめてから開発者へ頼む。

### 2. 提出の直前の確かめ（AI・読むだけ）

`gh` の読むだけの問い合わせで、次の 3 つを読む。1 つでも合わなければ、提出を頼まずに開発者へ報告する。

- winget-pkgs（`microsoft/winget-pkgs`）に、同じ名乗りの PR がまだ無いこと・置き場 `manifests/a/Areka/` がまだ無いこと。
- winget-pkgs の PR のひな形（`.github/PULL_REQUEST_TEMPLATE.md`）が勧める書式が、まだ 1.12 であること（雛形の書式は 1.12.0）。
- 提出する版が、着手のときに決めた版（`0.0.2`）のままであること。

今回の結果は、下の「直前の確かめ（読むだけ）」に書いてある。**提出の直前（タスク 4.2）にもう一度読み、同じ節へ書き足す。**

### 3. 道具と入れ方（AI）

- 道具は Microsoft の `wingetcreate`（パッケージの ID は `Microsoft.WingetCreate`）。使うのは「手元の 4 ファイルを PR にする」`submit` だけ。
- 入れ方: AI が普段の権限の端末で次を打つ。winget や OS の設定は変えない。

  ```powershell
  winget install --id Microsoft.WingetCreate --exact --source winget --disable-interactivity
  ```

- 止められたとき（同意を求められた・管理者の権限を求められた、など）は、AI は答えずに出た文を記録し、提出の回に開発者が同じコマンドを自分の端末で打つ。
- 今回の結果は、下の「道具の用意」に書いてある（入った）。

### 4. 打つコマンド（開発者）

普段の権限の端末（管理者の端末でなくてよい）を新しく開き、本仕様のワークツリーの根（`dist\winget\0.0.2` が見える場所）で、次の 1 行を打つ。

```powershell
wingetcreate submit --prtitle "New package: Areka.Areka.Portable version 0.0.2" dist\winget\0.0.2
```

- **`--token` は付けない**（付けると、トークンが道具の記録に残りうると、道具の文書に書いてある）。
- 打つとブラウザが開くので、開発者が GitHub へログインして、道具に許可を与える。
- 開発者のアカウントに winget-pkgs のフォークが無ければ、道具が自分で作る。
- 出し終えると、ブラウザで PR の頁が開く。その URL を AI へ伝える（AI は `gh` でも探せる）。
- 出すのは `Areka.Areka.Portable` の 1 つの版（`0.0.2`）の 4 ファイルだけ。ほかの物は足さない。
- PR の題は、winget-pkgs のひな形が示す形（`New package: Publisher.Name version X.Y.Z`）に合わせてある。

### 5. PR の頁で開発者が行うこと（開発者）

- 同意（CLA）: 初めての PR なので、PR に同意を求める案内（`Needs-CLA` の印と、案内のコメント）が出る。案内に従って同意する。1 回済ませれば、Microsoft のどのリポジトリにも効く。
- PR の本文の確かめ項目に印を付ける（`[ ]` を `[x]` にする）。付けるのは次の 6 つ。
  - 同意（CLA）を済ませた
  - 同じマニフェストの、ほかに開いている PR が無い
  - この PR が直すマニフェストは 1 つだけ
  - `winget validate --manifest <path>` で手元で検査した
  - `winget install --manifest <path>` で手元で試した
  - マニフェストは 1.12 の書式に合っている
- 「関わる issue を結んだ（在れば）」の項目は、関わる issue が無いので印を付けない。

### 6. トークンの扱い（開発者と AI の両方）

- トークンと認証の値を、リポジトリにも、この記録にも、端末の写しにも書かない。AI はトークンを扱わず、提出の操作をしない。
- 提出のコマンドに `--token` を付けない（上の 4）。トークンを作る・貼る場面は無い。
- 道具は、環境変数 `WINGET_CREATE_GITHUB_TOKEN` が在ると、それをトークンとして使う（道具の文書の `token` の説明）。提出の前に、この変数が無いことを AI が確かめる（在るか無いかだけを見る。値は読まない）。今回の結果は、下の「道具の用意」に書いてある（無い）。
- 道具が手元に覚えたログインは、PR を出し終えたら、開発者が次の 1 行で消す。消したことを AI がこの記録に書く。

  ```powershell
  wingetcreate token --clear
  ```

- 端末の出力を AI へ貼って見せるときは、トークンや認証の値が写っていないことを見てから貼る。

### 7. 人に問われたときの答えの文（開発者が PR に書く）

名乗りの頭（`Areka`）と発行者（`ekicyou`）が違うことを審査の人に問われたら、次の文で答える。

> Areka is the project name and ekicyou is the GitHub account of its maintainer. The identifier Areka.Areka is kept free for a future installer edition, so this zip edition is Areka.Areka.Portable.

### 8. 直しを求める印の期限（開発者。印は AI が読んで知らせる）

- PR に直しを求める印（`Needs-Author-Feedback`）が付いたら、**10 日のうちに応える**。応えないと、PR が自動で閉じられる。
- AI は区切りごとに PR の印と検査の結果を `gh` で読み、この印が付いたら、付いた日と期限の日を開発者へ伝える。
- 雛形の直しで済むときは、AI が `dist/winget/0.0.2/` を直して `winget validate` を通し、開発者が同じ内容を、自分のフォークのその PR の枝へ載せる（新しい PR は出さない）。

### 提出の後に AI が行うこと（読むだけ）

- PR のファイルを読み、雛形へ写し戻して見比べる（下の「写し戻しの見比べ」）。
- PR の印と検査の結果を読み、移り変わりを記録する（下の「印と検査の移り変わり」）。常に見張る仕組みは作らない。

### 直前の確かめ（読むだけ）

読んだ日は 2026-10-10。どれも `gh` の読むだけの問い合わせで、何も書き込んでいない。**3 つとも合った**（提出の妨げは 0）。

| 確かめたこと | 結果 | 読んだ時刻 |
|---|---|---|
| winget-pkgs に同じ名乗りの PR が無い | **0 件**（開いている物・閉じた物・取り込まれた物のどれも） | 11:47:36〜11:47:40 |
| winget-pkgs に置き場 `manifests/a/Areka/` が無い | **無い**（HTTP 404） | 11:47:48〜11:47:50 |
| PR のひな形が勧める書式が 1.12 | **1.12 のまま** | 11:47:48〜11:47:50 |
| 提出する版が `0.0.2` のまま | **`0.0.2` のまま**（より新しい Release は 0 件） | 11:47:59〜11:48:01 |

同じ名乗りの PR:

- `gh search prs --repo microsoft/winget-pkgs "Areka.Areka" --json number,title,state,url --limit 30` → `[]`（0 件・終了コード 0）
- `gh search prs --repo microsoft/winget-pkgs "Areka" --match title --json number,title,state,url --limit 30` → `[]`（題に `Areka` を持つ PR は 0 件・終了コード 0）
- `gh search prs --repo microsoft/winget-pkgs "Areka" --json number,title,state,url --limit 30` → `[]`（題・本文・コメントのどこかに `Areka` を持つ PR は 0 件・終了コード 0）
- 検索が当たりを出せること（0 と書く前に確かめた）: 同じ形の `gh search prs --repo microsoft/winget-pkgs "rhysd.actionlint" --match title --json number,title,state --limit 3` は 3 件を返した（どれも取り込まれた PR で、題は `New version: rhysd.actionlint version 1.7.12` など）。＝この検索は、PR が在れば、取り込まれた物も含めて出す。

置き場:

- `gh api repos/microsoft/winget-pkgs/contents/manifests/a/Areka -i` → 1 行目は `HTTP/2.0 404 Not Found`（`gh: Not Found (HTTP 404)`）。
- 問い合わせが当たりを出せること: 同じ形の `gh api repos/microsoft/winget-pkgs/contents/manifests/r/rhysd` は、中のフォルダの名前 `actionlint` を返した（終了コード 0）。＝在る置き場には答えが返る。
- あわせて、`manifests/a` の直下の一覧を `gh api "repos/microsoft/winget-pkgs/git/trees/master:manifests/a"` で読んだ。719 件（途中で切れていない）のうち、名前が `Are` で始まる物（大文字と小文字の違いは問わない）は `Arelle`・`Ares`・`AresValley`・`ares-emulator` の 4 つで、`Areka` は無い。

PR のひな形:

- `gh api repos/microsoft/winget-pkgs/contents/.github/PULL_REQUEST_TEMPLATE.md -H "Accept: application/vnd.github.raw"` で全文を読んだ（写しは `target\winget-check\logs\pre-4.1-pr-template.md`）。
- 書式の版を名指ししている行はこの 1 行だけ: `- [ ] Manifest conforms to the [1.12 schema](https://github.com/microsoft/winget-pkgs/tree/master/doc/manifest/schema/1.12.0)`
- ひな形の確かめ項目は 7 つ（同意・関わる issue・ほかに同じ PR が無い・直すマニフェストは 1 つ・`winget validate`・`winget install --manifest`・1.12 の書式）。上の「5. PR の頁で開発者が行うこと」は、この並びに合わせてある。
- ひな形の 1 行目は、題の形を `New package: Publisher.Name version X.Y.Z` と示している。提出のコマンドの題と同じ形。

提出する版:

- `gh release list --repo ekicyou/areka --limit 20 --json tagName,isDraft,isPrerelease,isLatest,publishedAt` → 1 件だけ（`v0.0.2`・下書きでない・先行版でない・最新の印あり・公開は 2026-10-06T12:49:21Z）。着手のときに読んだ一覧と同じ。
- 雛形の置き場 `dist/winget/` の下のフォルダは `0.0.2` の 1 つだけで、中の 4 ファイルの `PackageVersion` は 4 行とも `0.0.2`。

（提出の直前にもう一度読んだ結果は、タスク 4.2 がここへ書き足す）

### 道具の用意

結果: **入った**（`wingetcreate` v1.12.13.0。普段の権限で・同意を求める問いも、管理者の権限を求める問いも出なかった）。

| 見たこと | 結果 | 時刻 |
|---|---|---|
| 入れる前 | 入っていない。`winget list --id Microsoft.WingetCreate --exact --source winget --disable-interactivity` は `入力条件に一致するインストール済みのパッケージが見つかりませんでした。`（終了コード -1978335212）。`Get-Command wingetcreate -All` は 0 件 | 11:48:01 |
| 入れる | `winget install --id Microsoft.WingetCreate --exact --source winget --disable-interactivity`（同意を先回りして与える旗は付けていない）。終了コード 0 | 11:48:07〜11:49:01 |
| 入れた後の `winget list` | `Windows Package Manager Manifest Creator`・ID `Microsoft.WingetCreate`・版 `1.12.13.0`・ソース `winget` の 1 行（終了コード 0） | 11:49:12 |
| 解決先 | `Get-Command wingetcreate -All` は 1 件: `%LOCALAPPDATA%\Microsoft\WindowsApps\wingetcreate.exe`（アプリの呼び出し名。本体は `%ProgramFiles%\WindowsApps\Microsoft.WindowsPackageManagerManifestCreator_1.12.13.0_x64__8wekyb3d8bbwe`） | 11:49:12 |
| 道具の版 | `wingetcreate --version` の 1 行目は `WingetCreateCLI 1.12.13.0+1aa9b6637ca872911b84639d89526f5fe67a06c3`。`wingetcreate info` は `Windows パッケージ マネージャー マニフェスト作成者 v1.12.13.0` | 11:49:12・11:49:30 |

入れるときに winget が出した文（標準出力の全部。進み具合の印の行は除く）:

```text
見つかりました Windows Package Manager Manifest Creator [Microsoft.WingetCreate] バージョン 1.12.13.0
このアプリケーションは所有者からライセンス供与されます。
Microsoft はサードパーティのパッケージに対して責任を負わず、ライセンスも付与しません。
インストーラーハッシュが正常に検証されました
パッケージのインストールを開始しています...
インストールが完了しました
```

あわせて読んだこと:

- 入った版 1.12.13.0 は、設計が見込んだ版（2026-10-10 時点の最新）と同じ。
- `wingetcreate --version` は、版の行の後ろに使えるコマンドの一覧も出し、終了コードは 1 だった（版を出す正しい書き方は `wingetcreate version`。版の値は上の 1 行目と `wingetcreate info` の 2 つで読めている）。
- 提出のコマンドの書き方は、道具の文書（`microsoft/winget-create` の `doc/submit.md`・`doc/token.md`。11:49:40 に `gh` で読んだ）と照らした: `submit` は `--prtitle`（PR の題）とマニフェストの場所を受ける／`--token` を付けなければ GitHub へのログインを求める／`--token` を付けるとトークンが記録に残りうる／覚えたログインを消すのは `token --clear`。手順の 4 と 6 は、この書き方のとおり。
- 環境変数 `WINGET_CREATE_GITHUB_TOKEN` は**無い**（11:49:52。今のプロセス・利用者の側の登録・機械の側の登録の 3 か所とも無い。在るか無いかだけを見て、値は読んでいない。同じ見方で `Path` は「在る」と出る）。
- `wingetcreate info` が示した、道具が自分で決める置き場（本仕様が選べない例外）: 道具の記録は `%LOCALAPPDATA%\Packages\Microsoft.WindowsPackageManagerManifestCreator_8wekyb3d8bbwe\LocalState\DiagOutputDir`・利用者の設定は同じ `LocalState` の `settings.json`・取ってきたインストーラーの一時の置き場は `%TEMP%\wingetcreate`。
- 道具は、既定で使い方の統計（匿名）を Microsoft へ送る（`wingetcreate info` の文による）。この設定は変えていない。
- AI が打ったのは、`winget install`（上の 1 回）・`winget list`・`wingetcreate --version`・`wingetcreate help submit`・`wingetcreate help token`・`wingetcreate info` だけ。`wingetcreate submit` と `wingetcreate token` は打っていない（ログインも提出もしていない）。`wingetcreate help <コマンド名>` は、コマンドごとの説明でなく全体の一覧を返した（終了コード 1）ので、書き方は上のとおり道具の文書で読んだ。
- winget と OS の設定は何も変えていない。入れた物は `Microsoft.WingetCreate` の 1 つだけ。
- 出た文は `target\winget-check\logs\` の `pre-4.1-*`・`4.1-*` に取ってある（winget と道具の文は UTF-8 として読んだ）。

## 提出した版と日時

（タスク 4.2 で記入）

## PR の URL

（タスク 4.2 で記入）

## 印と検査の移り変わり

（タスク 4.3 で記入）

## 写し戻しの見比べ

（タスク 4.2 で記入）

## 赤と、その扱い

（タスク 4.3 で記入）

## 完了の判定

（タスク 4.3 で記入）

## 名前の直しの数

（タスク 5.2 で記入）

## 範囲の確かめ

（タスク 5.3 で記入）
