# 提出の記録: winget-pkgs へ `Areka.Areka.Portable` を初めて出す

- 日付: 2026-10-10（この記録を作った日。時刻はどれも +09:00）
- 機械: 開発機（Windows 11 Pro・x64）・winget v1.29.380
- コミット: `1c1ad468`（この記録を作ったときの枝の先頭。枝は `claude/areka-p0-winget-manifest-1c4fd7`）
- 提出する版: `0.0.2`（Release `v0.0.2`。決め方は手元の確かめの記録 `winget-local-check.md` の「ハッシュの突き合わせ」の 1）
- この記録は、手順と、提出の直前の確かめと、道具の用意までを先に書いた（タスク 4.1）。提出は 2026-10-10 17:29〜17:31 に行い、PR の中身を雛形へ写し戻して見比べた（タスク 4.2。下の「提出した版と日時」「PR の URL」「写し戻しの見比べ」）。**提出のコマンドを打ったのは、手順の表の 4 と 6 の書き方と違って AI だった**（開発者の指示による。事情は「提出した版と日時」の「手順と違ったこと」）。自動の検査は 19:03 までに 10 個とも通り、赤は 0 で、完了の線（自動の検査が通り、人の承認待ちになった）を満たした（タスク 4.3。下の「印と検査の移り変わり」「赤と、その扱い」「完了の判定」）。名前の直し（タスク 5.2）と、範囲の確かめ（タスク 5.3）も済んだ（下の同じ名前の節）。

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

#### 提出の直前の読み直し（2026-10-10 17:15:07・タスク 4.2）

結果: **どれも合った**（提出の妨げは 0）。読んだのは、この仕事を進めている AI（進行役。以下「進行役の AI」）。**この読み直しの出力はファイルに取っていない**ので、下の表は進行役の AI の読みをそのまま書いた物（後から突き合わせられる写しは無い）。どれも読むだけの問い合わせで、何も書き込んでいない。

| 確かめたこと | 結果 |
|---|---|
| winget-pkgs に同じ名乗りの PR が無い | **0 件**。`Areka.Areka` の検索と、題に `Areka` を持つ PR の検索が、どちらも `[]`。検索が当たりを出せることは、同じ形の `rhysd.actionlint` の検索で見た（当たりが返った） |
| winget-pkgs に置き場 `manifests/a/Areka/` が無い | **無い**。`gh api repos/microsoft/winget-pkgs/contents/manifests/a/Areka` は HTTP 404。同じ形の `manifests/r/rhysd` は `actionlint` を返した |
| PR のひな形が勧める書式が 1.12 | **1.12 のまま**。行は `- [ ] Manifest conforms to the [1.12 schema](…/schema/1.12.0)` |
| 提出する版が `0.0.2` のまま | **`0.0.2` のまま**。`gh release list` は `v0.0.2` の 1 件だけ（最新の印あり・公開は 2026-10-06T12:49:21Z） |
| 雛形が検査を通る | `winget validate --manifest dist\winget\0.0.2` は終了コード 0 |
| 環境変数 `WINGET_CREATE_GITHUB_TOKEN` | **無い**（今のプロセス・利用者の側の登録・機械の側の登録の 3 か所とも。在るか無いかだけを見た） |
| winget の設定 `LocalManifestFiles` | `False`（元へ戻っている） |
| 提出の道具 | `wingetcreate` v1.12.13.0（タスク 4.1 で入れた版のまま） |
| 雛形が、確かめたときのまま | `dist/` は枝の先頭（`138027f2`）と違いが 0 |

前提（手順の 1）は、この記録を作ったとき（タスク 4.1）には満たしていなかった 2 つも含めて、提出の前にそろっていた。

- 入れて起動する確かめ（タスク 2.2）は合格（6 項目のうち合格 6・不合格 0。`winget-local-check.md` の「入れて起動する」の 4）。
- 上げ直しの実測（タスク 3.2）・外し方の実測（タスク 3.3）・機械の全員向けの実測（タスク 3.5）の記録は済んでいる（`winget-local-check.md` の同じ名前の節。`tasks.md` でも 3 つとも済みの印）。
- 見つかった件の起票（タスク 3.6）は済んでいる（コミット `138027f2`）。

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

結果: **`Areka.Areka.Portable` の版 `0.0.2` を、2026-10-10 17:29:19〜17:31:16（+09:00）に提出した**（道具の終了コードは 0。PR は下の「PR の URL」）。

| 項目 | 値 |
|---|---|
| 提出した版 | `0.0.2`（Release `v0.0.2` の zip を指す。x64 と arm64 の 2 項目） |
| 提出した物 | `dist/winget/0.0.2/` の 4 ファイル（枝の先頭 `138027f2` のままの物） |
| 道具 | `wingetcreate` v1.12.13.0 |
| コマンド | `wingetcreate submit --prtitle "New package: Areka.Areka.Portable version 0.0.2" <ワークツリーの根>\dist\winget\0.0.2`（`--token` は付けていない・普段の権限） |
| 始めた時刻 | 2026-10-10 17:29:19.109 |
| 終わった時刻 | 2026-10-10 17:31:16.930（始めてから 117.8 秒。ブラウザでのログインを待った時間を含む） |
| 終了コード | 0（標準エラーは 0 バイト） |

### 手順と違ったこと（開発者の指示による）

タスク 4.2 の 1 つ目の箇条と要件 5.2 は、「提出は開発者が自分の手で行い、AI は提出の操作をしない」と決めている。今回はここから外れた。

- 決めたのは開発者。進行役の AI が手順のとおりに提出を頼んだところ、開発者は「だしてよいが、そちらでできない？」と答えた。
- それを受けて、**提出のコマンドは進行役の AI が打った**。開発者が行ったのは、AI にはできない部分だけ＝ブラウザでの GitHub へのログインと、道具が出した 1 回限りのコードを入れて道具に許可を与えること。
- **AI はトークンを見ても扱ってもいない**。トークンは、道具が GitHub から自分で受け取って手元に覚えた。コマンドに `--token` は付けていない。覚えたログインは、PR が出た後に AI が消した（下の「覚えたログインを消した」。手順の 6 では開発者が消す段だった）。
- PR の本文の確かめ項目の印も、開発者に任されて進行役の AI が付けた（手順の 5 では開発者が行う段だった。下の「印と検査の移り変わり」）。
- 手順書（上の「手順」）は、要件 5.1 が求めた「決めた手順」としてそのまま残してある。実際に誰が何をしたかは、次の表のとおり。

| 行ったこと | 手順書での担当 | 実際に行った者 |
|---|---|---|
| 提出のコマンドを打つ | 開発者 | 進行役の AI（開発者の指示） |
| GitHub へのログインと、道具への許可（1 回限りのコードを入れる） | 開発者 | 開発者 |
| 覚えたログインを消す（`wingetcreate token --clear`） | 開発者 | 進行役の AI |
| PR の本文の確かめ項目に印を付ける | 開発者 | 進行役の AI（開発者に任された） |
| 同意（CLA）の文を PR に送る | 開発者 | 進行役の AI（開発者に任された。事情は、この表の下の「同意（CLA）の行について」） |
| PR のファイルを読み、雛形へ写し戻して見比べる | AI | AI（このタスク。読むだけ） |

同意（CLA）の行について（**進行役の AI から聞いた話**。このタスクの AI は、開発者の言葉を直に見ていない）:

- 進行役の AI が「同意のコメントを、開発者の名前で AI が書いてよいか」と尋ね、開発者は「任せる」と答えた（17:32 ごろ）。
- 進行役の AI は、コメントの 2 つの形（個人として＝権利を持つのは自分だけで、雇い主の仕事として出す物ではない／雇い主の代わりに＝`company="…"` を付ける）と、同意の中身の短いまとめを開発者に見せて、どちらに当たるかを尋ねた。開発者は「個人として」と答えた。
- 進行役の AI が、17:35:01 に `@microsoft-github-policy-service agree` を PR に送り、17:35:42 に本文の項目「Signed the Contributor License Agreement」へ印を付けた。
- 印の動き（GitHub の出来事の記録による。読んだのはレビュー係）: `New-Package` が付いたのは 17:34:01・`Needs-CLA` が外れたのは 17:35:21。印の移り変わりの全部は、タスク 4.3 が書く。

同意（CLA）について、GitHub で確かめられたこと（タスク 4.3 の AI が 2026-10-10 19:14:39〜19:15:36 に読んだ。読むだけ。写しは `target\winget-check\logs\4.3-issue-comments.json`・`4.3-issue-events.json`・`4.3-check-runs-all.json`）:

- 同意の文: PR の 3 つ目のコメント。書いたアカウントは `ekicyou`（人のアカウント）、時刻は 2026-10-10T08:35:01Z（17:35:01）、本文は `@microsoft-github-policy-service agree` の 1 行だけ（38 字。`company=` は付いていない＝個人としての形）。書いた後に直されていない（作った時刻と最後に直した時刻が同じ）。
- 印 `Needs-CLA`: 17:31:44 に付き、17:35:21 に外れた（同意の文の 20 秒後。付いていたのは 3 分 37 秒）。付けたのも外したのも、自動の仕組み `microsoft-github-policy-service[bot]`。その後、付き直していない。
- 検査 `license/cla`: 最初に成功したのは 17:35:08（同意の文の 7 秒後）。PR の枝の先頭のコミットに結ばれたこの検査の回は 15 回で、成功が 10 回（17:35:08〜17:35:50 に 7 回・19:03:36〜19:04:13 に 3 回）、「順番待ち」のまま残っている回が 5 回（どれも同意の前の 17:31:23〜17:34:08 に始まった物）、失敗は 0 回。PR の頁に出るのは最後の回（19:04:13・成功）。
- 上の 3 つは、進行役の AI の話（17:35:01 に同意の文を送った・個人としての形）と食い違わない。
- GitHub からは確かめられないこと: 同意の文を誰の手で送ったか（どちらの手でも、アカウントは `ekicyou` になる）と、開発者の言葉（「任せる」「個人として」）。この 2 つは、上のとおり進行役の AI から聞いた話のまま。

### 道具が出した文

標準出力の全部（1 回限りのコードの行だけ、コードを伏せてある）:

```text
このコマンドを続行するには、GitHub アカウントまたはパーソナルアクセストークンがリンクされている必要があります。
GitHub ログインが完了するまで、コマンドの実行は一時停止されます。

GitHub ログインを開始しています...

Web ブラウザーを起動しています... 既定の web ブラウザーが起動しない場合は、次にアクセスしてください: https://github.com/login/device
メッセージが表示されたら、次のユーザー確認コードを入力してください: （1 回限りのコード。ここには写さない）

トークンがキャッシュに正常に格納されました。
コマンド実行を再開しています...
マニフェストの検証に成功しました: True

マニフェストのプル リクエストを送信しています...

プル リクエストはここにあります: https://github.com/microsoft/winget-pkgs/pull/450070
```

- 出た文と時刻は `target\winget-check\logs\4.2-submit.stdout.txt`・`4.2-submit.stderr.txt`（0 バイト）・`4.2-submit.times.txt` に取ってある。上の時刻と終了コードは `4.2-submit.times.txt` の 2 行と同じ。
- `4.2-submit.stdout.txt` は、今は道具が出した文そのままではない。1 回限りのコードの行（7 行目）の 1 行だけ、2026-10-10 17:42:54 に進行役の AI（開発者ではない）が、コードを `<1 回限りのコード・伏せた>` に書き替えた。ほかの行の字は、道具が出したまま（このタスクの AI が、伏せる前の 17:34 に読んで上へ写した文と 1 行ずつ比べて、違うのは 7 行目だけ。行末は、今は全部 LF）。
- コードは、もうどこにも残っていない（17:49 に見た。このファイルの中に、コードの形の文字列は 0 件・伏せた字は 1 件。この記録にも 0 件）。コードは、ログインのための 1 回限りの物で、使い終わって期限も切れている。トークンではない。
- 道具は、出す前に自分でもマニフェストを検査した（`マニフェストの検証に成功しました: True`）。
- 開発者のアカウントに winget-pkgs のフォークは無かったので、道具が作った（`gh api repos/ekicyou/winget-pkgs` が返した作成の時刻は 2026-10-10T08:31:05Z＝17:31:05。PR ができる 10 秒前）。

### 覚えたログインを消した

- 時刻: 2026-10-10 17:31:47（PR ができた 32 秒後）
- コマンド: `wingetcreate token --clear`（進行役の AI が打った）
- 道具が出した文: `GitHub トークン キャッシュをクリアしています`
- 終了コード: 0
- このコマンドの出力はファイルに取っていない（進行役の AI の読み）。

## PR の URL

**https://github.com/microsoft/winget-pkgs/pull/450070**

読んだのは 2026-10-10 17:34:20 と 17:37:36（`gh pr view 450070 --repo microsoft/winget-pkgs --json …` と `gh api repos/microsoft/winget-pkgs/pulls/450070/files`。どちらも読むだけ。2 回目の出力は `target\winget-check\logs\4.2-pr-view.json`・`4.2-pr-files.json`）。2 回とも下の値は同じだった。

| 項目 | 値 |
|---|---|
| 番号 | 450070 |
| 題 | `New package: Areka.Areka.Portable version 0.0.2` |
| 状態 | 開いている（下書きではない） |
| できた時刻 | 2026-10-10T08:31:15Z（17:31:15 +09:00） |
| 出し手 | `ekicyou` |
| 出した枝 | `ekicyou/winget-pkgs` の `Areka.Areka.Portable-0.0.2-50d5a35e-36a8-4eed-8e75-e23fd1691038` |
| 枝の先頭のコミット | `077130407a9cc119eaab395db9916fb12ab8bdd4`（2026-10-10T08:31:12Z・コミットは 1 つ・題は PR と同じ） |
| 受ける枝 | `microsoft/winget-pkgs` の `master` |
| 変わったファイル | 4（足した行 69・消した行 0） |

PR のファイルの一覧（全部。4 つとも「足した」ファイル）:

| ファイル | 足した行 | 消した行 | git の中身の番号（blob） |
|---|---|---|---|
| `manifests/a/Areka/Areka/Portable/0.0.2/Areka.Areka.Portable.installer.yaml` | 21 | 0 | `1568ecbcfb45bbe379b90a62f56672d165ad27ff` |
| `manifests/a/Areka/Areka/Portable/0.0.2/Areka.Areka.Portable.locale.en-US.yaml` | 25 | 0 | `bcfe71b3dff8436d63e4eba6b294f74c4260703f` |
| `manifests/a/Areka/Areka/Portable/0.0.2/Areka.Areka.Portable.locale.ja-JP.yaml` | 15 | 0 | `d9eb110138a85f3a4c4f622d91d1215e1133de8f` |
| `manifests/a/Areka/Areka/Portable/0.0.2/Areka.Areka.Portable.yaml` | 8 | 0 | `a665efea5d3a82cc1a1da3e558fdea39e5e783a4` |

判定（要件 5.2・設計の段 11 の 1）: **合った**。

- 変わったファイルは 4 つだけで、4 つとも winget-pkgs の決まった置き場 `manifests/a/Areka/Areka/Portable/0.0.2/` の直下に在る。この置き場の外のファイルは 0。
- 含むのは 1 つのパッケージ（`Areka.Areka.Portable`）の 1 つの版（`0.0.2`）だけ。ファイルの名前は、version・installer・既定のロケール（en-US）・追加のロケール（ja-JP）の 4 つで、雛形の 4 ファイルと同じ名前。

## 印と検査の移り変わり

結果: **自動の検査は 10 個とも成功し、2026-10-10 19:03:30 に `Validation-Completed` の印が付いた**（PR ができてから 92 分 15 秒。失敗の印と `Needs-Author-Feedback` は 1 度も付いていない）。

まず、PR を出した直後の読み（タスク 4.2）。その後の移り変わりと自動の検査の結果（タスク 4.3）は、下の「出来事の記録から読んだ印の移り変わり」から。時刻は、Z と書いた物のほかは、どれも 2026-10-10（+09:00）。

| 時刻 | 読んだ者 | 付いていた印 | あわせて読んだこと |
|---|---|---|---|
| 17:31（進行役の AI が伝えた時刻） | 進行役の AI | `Needs-CLA` | コメントは 2 つ。どちらも `microsoft-github-policy-service`（自動の仕組み）の物で、1 つは検査の進み具合を示す絵、もう 1 つは同意（CLA）を求める案内。同意はまだ |
| 17:34:20 | このタスクの AI | `Needs-CLA`・`New-Package` | 状態は開いている・ファイルは 4 つ |
| 17:37:08・17:37:36 | このタスクの AI | `New-Package`（**`Needs-CLA` は外れていた**） | コメントは 3 つに増えていた。3 つ目は `ekicyou` の `@microsoft-github-policy-service agree`（17:35:01）。PR の本文は、印の付いた項目が 6・付いていない項目が 1 になっていた。PR の最後の更新の時刻は 17:36:30 |

- 1 行目の時刻について: コメントが付いた時刻（17:37:08 にこのタスクの AI が読んだ値）は、絵のコメントが 17:31:43、同意を求める案内が 17:33:23。だから、進行役の AI が 2 つ目のコメントを読んだのは 17:33:23 より後で、「17:31」は印を最初に読んだ時刻と読むのがよい（この読みの出力はファイルに取っていないので、確かめる写しは無い）。

PR の本文に付けた印（進行役の AI・17:32:42・`gh pr edit --body-file`。開発者に任された）:

- 印を付けたのは、「Manifest Checklist」の下の 5 つ（ほかに開いている同じ PR が無い・この PR が直すマニフェストは 1 つだけ・`winget validate` で検査した・`winget install --manifest` で試した・1.12 の書式に合っている）。
- 説明の欄に 1 行足した: `New package: Areka.Areka.Portable version 0.0.2 (areka, portable zip edition; x64 and arm64).`
- このとき印を付けなかったのは 2 つ: 同意（CLA）の項目（まだ同意していなかった）と、「関わる issue を結んだ」の項目（関わる issue が無い）。
- 書き替える前と後の本文は `target\winget-check\logs\4.2-pr-body-before.md`・`4.2-pr-body-after.md` に取ってある。2 つの違いは、上の 5 つの印（`[ ]` → `[x]`）と足した 1 行だけ（行で見比べて確かめた）。

17:37 の読みについて:

- このタスクの AI は、GitHub へ何も書き込んでいない（読んだだけ）。
- 3 つ目のコメント（同意の文）と、本文の印の増え方（5 → 6。増えたのは同意の項目で、「関わる issue」の項目は付いていないまま）は、PR の出し手のアカウント `ekicyou` による物。
- 同じ時刻に、進行役の AI が残した写しが `target\winget-check\logs\` に在る: `4.2-cla-comment.time.txt`（中身は `commented at 2026-10-10 17:35:01 +0900` の 1 行）と、本文の書き替えの前後 `4.2-pr-body-before-cla.md`・`4.2-pr-body-after-cla.md`（ファイルの時刻は 17:35:39。2 つの違いは、同意の項目の `[ ]` → `[x]` の 1 行だけ）。＝同意の文を送ったのも、同意の項目に印を付けたのも、進行役の AI の手（写しからそう読め、進行役の AI の話とも合う）。開発者とのやり取りは、このタスクの AI は直に見ていない。進行役の AI から聞いた中身（開発者は「任せる」「個人として」と答えた）は、上の「手順と違ったこと」の「同意（CLA）の行について」に書いた。
- 17:37:08 の読みは `target\winget-check\logs\4.2-pr-read-2.json` に取ってある。

### 出来事の記録から読んだ印の移り変わり（タスク 4.3）

読んだのは 2026-10-10 19:14:39〜19:14:43（タスク 4.3 の AI。どれも読むだけで、GitHub へは何も書き込んでいない）。

- 読み方: `gh pr view 450070 --repo microsoft/winget-pkgs --json …`（状態・印・検査・コメント・レビュー・枝の先頭）／`gh api repos/microsoft/winget-pkgs/issues/450070/events --paginate`（出来事の記録の全部）／`gh api repos/microsoft/winget-pkgs/issues/450070/comments --paginate`／`gh api repos/microsoft/winget-pkgs/pulls/450070/files`。4 つとも終了コード 0。
- 写し: `target\winget-check\logs\4.3-pr-view.json`・`4.3-issue-events.json`・`4.3-issue-comments.json`・`4.3-pr-files.json`。読んだ時刻は `4.3-read.times.txt`、下の表の元にした書き出しは `4.3-summary.txt`。
- 19:16:54〜19:16:57 にもう一度読んだ（`4.3-pr-view-2.json`・`4.3-issue-events-2.json`）。印の一覧・出来事（18 件で、並びも同じ）・検査の結果・枝の先頭・コメントの数は、1 回目と同じだった。

印の出来事は 5 つ（付いた 4・外れた 1）。GitHub の出来事の記録の全部（18 件）から、印に関わる物を時刻の順に並べた。

| 番号 | 時刻（Z） | 時刻（+09:00） | 動き | 印 | 動かした者 |
|---|---|---|---|---|---|
| 1 | 2026-10-10T08:31:44Z | 17:31:44 | 付いた | `Needs-CLA` | `microsoft-github-policy-service[bot]` |
| 2 | 2026-10-10T08:34:01Z | 17:34:01 | 付いた | `New-Package` | `wingetvalidator-prod[bot]` |
| 3 | 2026-10-10T08:35:21Z | 17:35:21 | 外れた | `Needs-CLA` | `microsoft-github-policy-service[bot]` |
| 4 | 2026-10-10T10:03:29Z | 19:03:29 | 付いた | `Azure-Pipeline-Passed` | `wingetvalidator-prod[bot]` |
| 5 | 2026-10-10T10:03:30Z | 19:03:30 | 付いた | `Validation-Completed` | `wingetvalidator-prod[bot]` |

- 動かした者は、5 つとも自動の仕組み。人が付けた印・外した印は 0。
- 付いたことのある印は 4 種類（`Needs-CLA`・`New-Package`・`Azure-Pipeline-Passed`・`Validation-Completed`）。**`Needs-Author-Feedback` と失敗の印は、1 度も付いていない**（設計の「赤の仕分け」の表の印との照らし合わせは、下の「赤と、その扱い」）。
- 印でない出来事は 13 件: 出し手 `ekicyou` への知らせに関わる物が 12 件（`mentioned` 6・`subscribed` 6。17:31:25〜17:34:13）と、`auto_squash_enabled` が 1 件（19:04:06・`microsoft-github-policy-service[bot]`。出来事の名前からは、取り込みの条件がそろったら自動で取り込む設定がオンになった、と読める）。
- 枝の押し直し・閉じた・開き直した・レビューの出来事は 0 件。

その時刻に付いていた印（上の 5 つの出来事から）:

| 時刻（+09:00） | 付いていた印 |
|---|---|
| 17:31:15（PR ができた）〜17:31:44 | 無し |
| 17:31:44〜17:34:01 | `Needs-CLA` |
| 17:34:01〜17:35:21 | `Needs-CLA`・`New-Package` |
| 17:35:21〜19:03:29 | `New-Package` |
| 19:03:29〜19:03:30 | `New-Package`・`Azure-Pipeline-Passed` |
| 19:03:30〜（19:16:57 の読みまで変わらず） | `New-Package`・`Azure-Pipeline-Passed`・`Validation-Completed` |

PR の中身は、写し戻し（タスク 4.2）の後に変わっていない。

- 枝の先頭のコミットは `077130407a9cc119eaab395db9916fb12ab8bdd4` のまま（上の「PR の URL」の表と同じ。19:14 と 19:16 の 2 回とも）。
- PR のファイルは 4 つのままで、4 つの中身の番号（blob）は、上の「PR のファイルの一覧」の表と同じ。
- リポジトリの雛形の 4 ファイルは、`git hash-object --no-filters` の値が、PR の 4 つの中身の番号と同じ（一致 4・違い 0）。`dist/` は枝の先頭（`2ff18536`）と違いが 0。＝雛形は提出した物と同じ内容のまま（要件 2.3）。

### 自動の検査の結果

結果: **10 個とも成功・失敗は 0**（19:14:39 の読み。19:16:54 の読みでも同じ）。

| 検査 | 結果 | 始まり（+09:00） | 終わり（+09:00） | 掛かった時間 |
|---|---|---|---|---|
| `01. Pull Request Validation` | 成功 | 17:31:39 | 17:32:02 | 23 秒 |
| `02. Manifest Validation` | 成功 | 17:32:11 | 17:32:35 | 24 秒 |
| `03. URLs Validation` | 成功 | 17:32:41 | 17:33:08 | 27 秒 |
| `04. URL Domain Validation` | 成功 | 17:33:09 | 17:33:26 | 17 秒 |
| `05. Manifest Policy Validation` | 成功 | 17:33:28 | 17:33:47 | 19 秒 |
| `06. Catalog Content Verification` | 成功 | 17:33:49 | 17:34:16 | 27 秒 |
| `07. Installers Scan` | 成功 | 17:34:22 | 17:45:27 | 11 分 5 秒 |
| `08. Installation Validation` | 成功 | 17:45:29 | 19:01:57 | **76 分 28 秒** |
| `09. Installer Metadata Validation` | 成功 | 19:02:13 | 19:03:07 | 54 秒 |
| `10. Validation Completed` | 成功 | 19:03:12 | 19:03:37 | 25 秒 |
| `license/cla` | 成功 | 最初の成功は 17:35:08 | PR の頁に出る最後の回は 19:04:13 | （すぐ終わる検査） |

- 10 個は、`wingetvalidator-prod`（winget-pkgs の検査の仕組み）の物。01 の始まりから 10 の終わりまでは 91 分 58 秒。
- `license/cla` の回の数は、上の「同意（CLA）について、GitHub で確かめられたこと」に書いた（成功 10・順番待ちのまま 5・失敗 0）。
- PR の頁に結ばれた検査は、全部で 131 件: 成功が 11 件（上の表）、「飛ばした」が 120 件。失敗・取り消し・時間切れ・進行中は 0 件。
- 「飛ばした」の 120 件は、GitHub Actions の補助の仕組み 5 種類（`Domain Validation Assist`・`Manifest Validation Diagnosis`・`Missing Dependency Assist`・`Transient Security Explanation`・`Wingetbot PR Triage`）の物で、どれも動かずに終わっている。
- 赤のときの文は無い（赤が 0 なので）。

### コメント

コメントは 4 つ（19:14:39 の読み）。人のレビューは 0 件で、人（審査の人）のコメントも 0 件。

| 番号 | 時刻（+09:00） | 書いた者 | 中身 |
|---|---|---|---|
| 1 | 17:31:43 | `microsoft-github-policy-service[bot]` | 検査の仕組みと公開の仕組みの今の調子を示す絵が 2 つ（文は無い） |
| 2 | 17:33:23 | `microsoft-github-policy-service[bot]` | 同意（CLA）を済ませるまで取り込めない、という案内（案内の型の名前は `msftbot/needsCLA`） |
| 3 | 17:35:01 | `ekicyou` | `@microsoft-github-policy-service agree`（同意の文。上の「手順と違ったこと」） |
| 4 | 19:06:01 | `microsoft-github-policy-service[bot]` | 外からの PR はモデレーターの承認が要る・モデレーターは有志なので時間を見てほしい、という案内（案内の型の名前は `msftbot/requiresApproval/moderator`） |

- 4 つ目は、`Validation-Completed` が付いた 2 分 31 秒後に付いた。＝自動の検査が通って、人の承認を待つ段へ移ったことを、winget-pkgs の側の仕組みも告げている。
- 直しを求める文・問い（名乗りの頭と発行者の違い、など）は、どのコメントにも無い。

### 進行役の AI の読み（区切りごと）

次の読みは、進行役の AI から聞いた物（**出力はファイルに取っていない**ので、突き合わせられる写しは無い）。進行役の AI は、読むたびに結果を開発者へ伝えた、とのこと。

- 17:42:54・17:51:20: 印は `New-Package` だけ。
- 17:53:13〜17:53:15: 印は `New-Package` だけ・検査は 01〜07 と `license/cla` が成功・08 が進行中（タスク 5.1 で読んだ物。後ろの spec への申し送りの 4 に書いてある）。
- 17:57:53: 検査の 01〜07 と `license/cla` は成功（07 は 11 分 5 秒）・08 は進行中。
- 18:02:07・18:17:31・18:42:46: 08 は進行中のまま（始まりは 17:45:29）・09 と 10 は順番待ち・印は `New-Package` だけ・失敗の印は無し。
- 18:44 ごろ: 同じ時間帯に出されたほかの 3 つの PR と見比べた（下の「検査の 08 に掛かった時間」）。
- 19:13:28: 印は `Azure-Pipeline-Passed`・`Validation-Completed`・`New-Package` の 3 つ。検査の 08・09・10 は成功。4 つ目のコメントが付いていた。レビューは 0 件・状態は開いている。

タスク 4.3 の AI が照らしたこと:

- この読みは、どれも、上の出来事の記録・検査の始まりと終わりの時刻と食い違わない（17:35:21 から 19:03:29 までの印は `New-Package` だけ・08 が動いていたのは 17:45:29〜19:01:57）。
- **直しを求める印 `Needs-Author-Feedback` は 1 度も付かなかったので、10 日の期限を開発者へ急ぎで伝える場面は 0 回だった**（期限そのものは、上の「手順」の 8 と、後ろの spec への申し送りの 5 に書いてある）。

### 検査の 08 に掛かった時間（見たこと）

`08. Installation Validation` は 76 分 28 秒掛かった。同じ時間帯に出されて、同じく `New-Package` の印が付いたほかの 3 つの PR より長い。**理由は分からない**（検査の中の記録は読んでいない。結果は成功で、赤ではない）。

| PR | できた時刻（+09:00） | `Validation-Completed` が付くまで | 07 に掛かった時間 | 08 に掛かった時間 |
|---|---|---|---|---|
| #450070（この PR） | 17:31:15 | **92 分 15 秒** | 11 分 5 秒 | **76 分 28 秒** |
| #450018 | 16:40:27 | 44 分 12 秒 | 4 分 48 秒 | 32 分 34 秒 |
| #450069 | 17:25:45 | 57 分 22 秒 | 13 分 32 秒 | 35 分 50 秒 |
| #450079 | 17:54:39 | 46 分 30 秒 | 4 分 38 秒 | 35 分 32 秒 |

- ほかの 3 つは、タスク 4.3 の AI が 19:15:27〜19:15:36 に読み直した値（`gh pr view <番号>` と出来事の記録。読むだけ。写しは `target\winget-check\logs\4.3-other-*`）。選んだのは進行役の AI で、18:44 ごろの見立ては「45〜58 分」だった。読み直した値では 44〜57 分。
- 3 つは見比べの目安で、数は少ない。winget-pkgs の検査の普段の所要時間を表す物ではない。

### 見守りの形

- **常に見張る仕組みは作っていない**（設計の段 11 の 3）。読んだのは、タスクの区切りでの読みと、時間を置いて 1 回だけ読む形の読みが 3 回（進行役の AI の話）、それにタスク 4.3 の AI の 2 回（19:14・19:16）。どれも `gh` の読むだけの問い合わせ。
- 繰り返し動く物・知らせを受ける仕掛け・workflow は、リポジトリにも機械にも足していない。
- タスク 4.3 が GitHub へ書き込んだ物は 0（コメント・印・本文の書き替え・レビューのどれもしていない）。winget・`wingetcreate`・areka は動かしていない。

## 写し戻しの見比べ

結果: **PR の 4 ファイルで雛形の 4 ファイルを置き換えた。欄と値は 1 つも変わっていない**（判定①は 4 ファイルとも一致・判定②は x64 と arm64 のどちらも一致・YAML として解いた見比べも違い 0）。置き換えた後の `winget validate` は成功（終了コード 0）。変わったのは、設計が許す 4 種類のうちの 3 種類だけで、それ以外の差は 0。

### 1. PR のファイルを取ってきた（17:34:32〜17:34:38）

- 取り方: PR の枝の先頭のコミットを名指しして、4 ファイルを 1 つずつ、中身をそのまま（1 バイトも変えずに）取ってきた。Git Bash で、`gh api "repos/ekicyou/winget-pkgs/contents/manifests/a/Areka/Areka/Portable/0.0.2/<ファイル名>?ref=077130407a9cc119eaab395db9916fb12ab8bdd4" -H "Accept: application/vnd.github.raw" > target/winget-check/pr-450070/<ファイル名>`（4 回とも終了コード 0）。
- 置き場: `target\winget-check\pr-450070\`。
- 取ってきた物が PR の物と同じバイトであること: 4 ファイルとも、`git hash-object --no-filters` の値が、上の「PR のファイルの一覧」の blob の番号と同じだった（一致 4・違い 0）。

| ファイル | 大きさ（バイト） | 行 | SHA256 | BOM | 行末 |
|---|---|---|---|---|---|
| `Areka.Areka.Portable.installer.yaml` | 864 | 21 | `2dab8c43e7471128e1975afbab1542dae0a10f07ec1547528903a3bec6e0f7ac` | 無い | 全部 CRLF（21 行） |
| `Areka.Areka.Portable.locale.en-US.yaml` | 1165 | 25 | `3fa631718e9bed3e93dac8d1a408f848374066d3b6cb681c98e600d779da9aa3` | 無い | 全部 CRLF（25 行） |
| `Areka.Areka.Portable.locale.ja-JP.yaml` | 955 | 15 | `8b16399ec9126c9b356e884ed5d921bd51545f119662a215417a00adc8928eff` | 無い | 全部 CRLF（15 行） |
| `Areka.Areka.Portable.yaml` | 267 | 8 | `c0397563cd5fd296bbf04bebde90f133448bf031a363b429ef93b661204b2529` | 無い | 全部 CRLF（8 行） |

- 4 ファイルとも、最後の行は改行で終わり、その後ろに空行は無い。行の合計は 69（PR の「足した行 69」と同じ）。

置き換える前の雛形（写しは `target\winget-check\pr-450070\template-before\`。枝の先頭 `138027f2` の物と同じ）:

| ファイル | 大きさ（バイト） | 行 | SHA256 | BOM | 行末 |
|---|---|---|---|---|---|
| `Areka.Areka.Portable.installer.yaml` | 804 | 20 | `f9b991e70f6293289b9bc6578d2e390c3f9a16fb75170b5f09973d6b8c492753` | 無い | 全部 LF |
| `Areka.Areka.Portable.locale.en-US.yaml` | 1101 | 24 | `1b00e2b06ccbc7c939ee4303b80213542c2a01e9162e5a662d092b2e72550b3f` | 無い | 全部 LF |
| `Areka.Areka.Portable.locale.ja-JP.yaml` | 901 | 14 | `261b30f7ddfd03a674cb4d7ecf4df101c8e3946ac3eb9925b17854f4c115da75` | 無い | 全部 LF |
| `Areka.Areka.Portable.yaml` | 220 | 7 | `dd7263bcea1185a9d138e333a9cd81131b79c2424ff8142ea49175bc5c340126` | 無い | 全部 LF |

### 2. 置き換える前に 2 つの判定を行った（17:35:54）

判定は、置き換える前に、雛形の写しと取ってきた PR のファイルを相手に行った（合わなければ雛形に触らずに止めるため）。

- コマンド（ワークツリーの根で）: `python -I target\winget-check\4.2-compare.py target\winget-check\pr-450070\template-before target\winget-check\pr-450070 .kiro\specs\areka-P0-winget-manifest-submission\verification\winget-local-check.md target\winget-check\logs\4.2-release-assets.json`
- 終了コード: **0**（最後の行は `RESULT: PASS`。`PASS` の行は 16・`FAIL` の行は 0）。出た文は `target\winget-check\logs\4.2-compare.txt`。
- スクリプトは、3 つのどれか 1 つでも合わなければ終了コード 1 で終わる。

**判定①（行の集まりの一致）: 4 ファイルとも一致**。ファイルごとに、コメントの行（`#` で始まる行）と空行を除き、行頭と行末の空白を外し、値を丸ごと包んでいる引用符を外して、並べ替えた行の集まりを比べた。

| ファイル | 雛形の行 | PR の行 | 片方にだけ在る行 | 判定 |
|---|---|---|---|---|
| `Areka.Areka.Portable.installer.yaml` | 18 | 18 | 0 | 一致 |
| `Areka.Areka.Portable.locale.en-US.yaml` | 22 | 22 | 0 | 一致 |
| `Areka.Areka.Portable.locale.ja-JP.yaml` | 12 | 12 | 0 | 一致 |
| `Areka.Areka.Portable.yaml` | 5 | 5 | 0 | 一致 |

- 別の作りでも同じ判定を 1 回行った（スクリプトを使わない、端末の 1 行。引用符は、包んでいる物に限らず全部の `'` と `"` を両方の側から外した）: `diff <(tr -d '\r' < 雛形 | grep -v '^[[:space:]]*#' | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' | grep -v '^$' | tr -d "'\"" | LC_ALL=C sort) <(PR の側も同じ)`。4 ファイルとも `diff` の終了コードは 0（違い 0）。出た文は `target\winget-check\logs\4.2-judge1-oneliner.txt`。
- 英語の `InstallationNotes` の行は、文の中に二重引用符（`"winget upgrade"` など 4 組）を持つ。道具はこの行を引用符で包み直さず、雛形と 1 字も違わない行で出していた（この行に限った差は 0）。

**判定②（arch ごとの URL とハッシュの組の一致）: x64 と arm64 のどちらも一致**。PR の installer のファイルを YAML として解き（行の並びには頼らない）、`Installers` の項目ごとの（`Architecture`・`InstallerUrl`・`InstallerSha256`）の組を比べた。

| arch | PR の `InstallerUrl` | PR の `InstallerSha256` | タスク 1.1 の突き合わせの値と同じ | Release が示す URL・ハッシュと同じ | 雛形の組と同じ |
|---|---|---|---|---|---|
| x64 | `https://github.com/ekicyou/areka/releases/download/v0.0.2/areka-0.0.2-x64.zip` | `66D3A9A01A611DD36B9C3AF276C0CD071EF87C28BA82B6CF8FC314FA6A751C9C` | 同じ | 同じ | 同じ |
| arm64 | `https://github.com/ekicyou/areka/releases/download/v0.0.2/areka-0.0.2-arm64.zip` | `77C5356A8F77CB121F828CFE761D7C63588C822790FB80BE78234F0F9FDD79FB` | 同じ | 同じ | 同じ |

- 比べた相手は 3 つ: ① `winget-local-check.md` の「ハッシュの突き合わせ」の表の行（スクリプトが表から読む。その行の 3 つの値が 1 種類であることも見る）② 今の Release が示す、その zip の URL と `digest`（17:35 に `gh release view v0.0.2 --repo ekicyou/areka --json assets,tagName,publishedAt` で読んだ。写しは `target\winget-check\logs\4.2-release-assets.json`）③ 置き換える前の雛形の組。ハッシュは大文字と小文字の違いを問わずに比べた。
- PR の `Installers` に在る arch は `x64` と `arm64` の 2 つだけ。

**YAML として解いた見比べ（判定①②より強い確かめ）: 4 ファイルとも違い 0**。雛形と PR のファイルを PyYAML 6.0.3 で解き、欄の名前と値を、入れ子の中まで 1 つずつ比べた（リストは並びも比べる）。解き方は 2 通り＝値を型に直す解き方（`SafeLoader`）と、どの値も書かれた字のまま読む解き方（`BaseLoader`）。4 ファイル × 2 通りの 8 回とも、違いは 0。

判定が赤を出せること（1 回だけ確かめた）:

- PR のファイルの写しで、installer の 2 つのハッシュを入れ替えた物（`target\winget-check\pr-450070\negative-swap\`）: 終了コード 1。**判定①は 4 ファイルとも一致のまま**で、判定②が x64 と arm64 の両方で不一致、解いた見比べも違い 2 を出した（設計が判定②を置いた理由のとおり＝並べ替えた行の集まりでは、組の入れ替わりは見えない）。出た文は `target\winget-check\logs\4.2-compare-negative-swap.txt`。
- PR のファイルの写しで、英語のロケールの `License: MIT` を `License: Apache-2.0` に変えた物（`target\winget-check\pr-450070\negative-value\`）: 終了コード 1。判定①がそのファイルで不一致（片方にだけ在る行を 1 つずつ挙げた）、解いた見比べも違い 1 を出した。出た文は `target\winget-check\logs\4.2-compare-negative-value.txt`。

### 3. 置き換えた（17:36:04）

- 取ってきた 4 ファイルを、`dist/winget/0.0.2/` の同じ名前のファイルへ上書きで写した。写した後の 4 ファイルは、取ってきた物と 1 バイトも違わない（`cmp` で 4 つとも同じ。SHA256 は上の 1 の表の値）。
- `dist/winget/0.0.2/` の中は 4 ファイルのまま（YAML 以外の物は無い）。`dist/` の下で変わったのは、この 4 ファイルだけ（`git status --short` は 4 行）。
- 置き換えた後の雛形を相手に、同じスクリプトをもう 1 回回した（17:37:07。雛形の写しと `dist\winget\0.0.2` を比べた）: 終了コード 0・`RESULT: PASS`・`PASS` の行は 16・`FAIL` の行は 0。出た文は `target\winget-check\logs\4.2-compare-after-replace.txt`。

### 4. 置き換えた後の差分の全文

`git diff -- dist/winget/0.0.2` の全文（49 行。`target\winget-check\logs\4.2-git-diff.txt` と同じ）。`git diff --stat` は「4 files changed, 5 insertions(+), 1 deletion(-)」。

```diff
diff --git a/dist/winget/0.0.2/Areka.Areka.Portable.installer.yaml b/dist/winget/0.0.2/Areka.Areka.Portable.installer.yaml
index 3f1a204c..0981323f 100644
--- a/dist/winget/0.0.2/Areka.Areka.Portable.installer.yaml
+++ b/dist/winget/0.0.2/Areka.Areka.Portable.installer.yaml
@@ -1,3 +1,4 @@
+# Created using wingetcreate 1.12.13.0
 # yaml-language-server: $schema=https://aka.ms/winget-manifest.installer.1.12.0.schema.json
 
 PackageIdentifier: Areka.Areka.Portable
@@ -8,7 +9,6 @@ NestedInstallerFiles:
 - RelativeFilePath: areka.exe
   PortableCommandAlias: areka
 ArchiveBinariesDependOnPath: true
-ReleaseDate: 2026-10-06
 Installers:
 - Architecture: x64
   InstallerUrl: https://github.com/ekicyou/areka/releases/download/v0.0.2/areka-0.0.2-x64.zip
@@ -18,3 +18,4 @@ Installers:
   InstallerSha256: 77C5356A8F77CB121F828CFE761D7C63588C822790FB80BE78234F0F9FDD79FB
 ManifestType: installer
 ManifestVersion: 1.12.0
+ReleaseDate: 2026-10-06
diff --git a/dist/winget/0.0.2/Areka.Areka.Portable.locale.en-US.yaml b/dist/winget/0.0.2/Areka.Areka.Portable.locale.en-US.yaml
index 52675136..66d4057d 100644
--- a/dist/winget/0.0.2/Areka.Areka.Portable.locale.en-US.yaml
+++ b/dist/winget/0.0.2/Areka.Areka.Portable.locale.en-US.yaml
@@ -1,3 +1,4 @@
+# Created using wingetcreate 1.12.13.0
 # yaml-language-server: $schema=https://aka.ms/winget-manifest.defaultLocale.1.12.0.schema.json
 
 PackageIdentifier: Areka.Areka.Portable
diff --git a/dist/winget/0.0.2/Areka.Areka.Portable.locale.ja-JP.yaml b/dist/winget/0.0.2/Areka.Areka.Portable.locale.ja-JP.yaml
index 85285c6e..c2fbaccf 100644
--- a/dist/winget/0.0.2/Areka.Areka.Portable.locale.ja-JP.yaml
+++ b/dist/winget/0.0.2/Areka.Areka.Portable.locale.ja-JP.yaml
@@ -1,3 +1,4 @@
+# Created using wingetcreate 1.12.13.0
 # yaml-language-server: $schema=https://aka.ms/winget-manifest.locale.1.12.0.schema.json
 
 PackageIdentifier: Areka.Areka.Portable
diff --git a/dist/winget/0.0.2/Areka.Areka.Portable.yaml b/dist/winget/0.0.2/Areka.Areka.Portable.yaml
index ef8b3ca5..9a66ebe2 100644
--- a/dist/winget/0.0.2/Areka.Areka.Portable.yaml
+++ b/dist/winget/0.0.2/Areka.Areka.Portable.yaml
@@ -1,3 +1,4 @@
+# Created using wingetcreate 1.12.13.0
 # yaml-language-server: $schema=https://aka.ms/winget-manifest.version.1.12.0.schema.json
 
 PackageIdentifier: Areka.Areka.Portable
```

この差分に行末の違いが出ない理由:

- このリポジトリの手元の設定（`core.autocrlf` が `true`）では、git は、作業の場所のファイルの CRLF を LF に直してから比べる。だから、行末だけが違う行は差分に出ない。`git diff --ignore-cr-at-eol -- dist/winget/0.0.2` の出力は、上の全文と 1 バイトも違わなかった。
- 行末そのものは、どの行も変わっている: 置き換える前は 4 ファイルとも全部 LF（65 行）、置き換えた後は 4 ファイルとも全部 CRLF（69 行）。上の 1 の 2 つの表のとおり。
- コミットに入る中身は LF になる（git が直して入れる）。4 ファイルとも、git がコミットに入れる中身の番号（`git hash-object`）は、PR のファイルの CRLF を LF に直した物の番号と同じだった（`0981323f…`・`66d4057d…`・`c2fbaccf…`・`9a66ebe2…`＝上の差分の `index` の行の右側）。＝リポジトリに入る雛形と PR のファイルの違いは、行末だけ。

### 5. 差の仕分け

差分に出た行（足した行 5・消した行 1 の、合わせて 6 行）と、差分に出ない行末の違いを、設計が許す 4 種類へ仕分けた。

| 種類 | 数 | 中身 |
|---|---|---|
| 冒頭のコメントの行 | **4**（足した行 4。1 ファイルに 1 行ずつ） | 4 ファイルとも、1 行目に `# Created using wingetcreate 1.12.13.0` が足された。書式の場所を示す行（`# yaml-language-server: …`）は 2 行目へ下がっただけで、字は変わっていない |
| 欄の並び | **1**（消した行 1 と足した行 1 の、動いた欄 1 つ） | installer のファイルの `ReleaseDate: 2026-10-06` が、`Installers` の前から、ファイルの最後（`ManifestVersion` の後ろ）へ動いた。値は同じ。ほかの 3 ファイルの並びは変わっていない |
| 引用符の有無 | **0** | 引用符が付いた行・外れた行は無い |
| 行末・BOM・末尾の空行 | **行末は 4 ファイルの全部の行**（LF → CRLF。git の差分には出ない）。BOM は 0・末尾の空行は 0 | 上の 4 のとおり。BOM は前も後も無い。最後の行は前も後も改行 1 つで終わる |
| **4 種類のどれにも入らない差** | **0** | 差分の 6 行は、上の 2 種類（4 行と 2 行）で全部。欄の名前・値が変わった行、足された欄、消えた欄は無い |

### 6. 置き換えた後の `winget validate`（要件 2.2）

結果: **成功**。

- コマンド: `winget validate --manifest dist\winget\0.0.2 --disable-interactivity`（ワークツリーの根で打った。設定は何も変えていない）
- 時刻: 2026-10-10 17:36:46.910〜17:36:47.777
- winget が出した文（標準出力の全部。警告の行は 0）: `マニフェストの検証は成功しました。`
- 終了コード: 0（標準エラーは 0 バイト）
- 出た文と時刻は `target\winget-check\logs\4.2-validate.stdout.txt`・`4.2-validate.stderr.txt`・`4.2-validate.times.txt` に取ってある（`winget-local-check.md` の「雛形の検査」の「`winget validate`」にも同じ結果を書いた）。

### 7. 判定（要件 2.3・設計の段 11 の 2）

**リポジトリの雛形は、winget-pkgs へ提出したマニフェストと同じ内容**。判定①は 4 / 4 が一致、判定②は 2 / 2 が一致、YAML として解いた見比べは 8 / 8 が違い 0、4 種類のどれにも入らない差は 0。止めて開発者へ報告する場合（欄か値が変わっていた）には当たらない。

- このタスクが行ったのは、読むだけの問い合わせ（`gh`）と、`dist/winget/0.0.2/` の 4 ファイルの置き換えと、`winget validate` だけ。GitHub へは何も書き込んでいない。`wingetcreate` は打っていない。winget の入れる・上げる・外す・設定の操作もしていない。
- 取ってきた物・雛形の写し・スクリプト・出た文（`target\winget-check\pr-450070\` と `target\winget-check\logs\4.2-*`）に、GitHub のトークンの形の文字列（設計の「範囲の確かめ」が挙げる 3 つの頭で始まる物）は 0 件。雛形の 4 ファイルと、この記録にも 0 件。

## 赤と、その扱い

結果: **赤は 0**（自動の検査の赤は 1 つも無い。提出の後の雛形の直しは 0 回・起票は 0 件）。

根拠（2026-10-10 19:14:39〜19:14:43 の読み。19:16:54〜19:16:57 の読み直しでも同じ）:

- 自動の検査の 10 個は、どれも成功。失敗は 0（上の「自動の検査の結果」）。
- 設計の「赤の仕分け」の表に在る失敗の印（名前で見る物 18・頭の字で見る物 2＝`Policy-Test-2.` と `Internal-Error`）は、**今の印の一覧に 0 件・出来事の記録の全部（付いた 4・外れた 1）にも 0 件**。1 度も付いていない。
- 直しを求める印 `Needs-Author-Feedback` も、今の一覧に 0 件・出来事の記録に 0 件。
- 表に載っている印のうち、付いたことがあるのは **`Needs-CLA` の 1 つだけ**（17:31:44〜17:35:21 の 3 分 37 秒）。これは検査の赤ではなく、初めての PR に付く同意待ちの印で、表の扱い（同意する・雛形は変わらない）のとおりに済んで外れた（上の「手順と違ったこと」の「同意（CLA）について、GitHub で確かめられたこと」）。
- 赤のときの文（検査が出す失敗の文・直しを求めるコメント）は 0 件。

照らし合わせの仕方:

- 印の名前の照らし合わせは、目で見るだけでなく、判定を返すスクリプトで行った: `python -I target\winget-check\4.3-judge.py target\winget-check\logs\4.3-pr-view.json target\winget-check\logs\4.3-issue-events.json` → 終了コード **0**（最後の行は `RESULT: MET`。出た文は `target\winget-check\logs\4.3-judge.txt`。読み直しの分は `4.3-judge-2.txt` で、同じ結果）。
- この判定が「満たしていない」を出せること（0 と書く前に確かめた）: 同じスクリプトを、実在のほかの PR #450018 の読み（`Validation-Completed` は付いているが、`Needs-CLA` が付いたまま）に掛けると、終了コード **1**（`RESULT: NOT MET`。`4.3-judge-control-450018.txt`）。出来事の記録を見る側が当たりを出せることは、この PR 自身の記録から `Needs-CLA` を「付いたことがある」と拾ったことで見た。
- 失敗の印を拾えること（同じく、0 と書く前に確かめた。19:19:27〜19:19:37 に、失敗の印が付いている実在の PR を印ごとに 1 つずつ読んで、同じスクリプトに掛けた。読むだけ）: 4 つとも終了コード **1** で、失敗の印を「今の一覧」と「付いたことがある」の両方に挙げた。#449897 は `Validation-Installation-Error`・`Validation-Defender-Error`、#450105 は `Manifest-Validation-Error`、#450038 は `Internal-Error-Dynamic-Scan`（頭の字で見る物）・`Needs-Attention`、#449546 は `Policy-Test-2.3`・`Policy-Test-2.8`（頭の字で見る物）。出た文は `target\winget-check\logs\4.3-judge-control-*.txt`。

赤が 0 なので、行わなかったこと:

- 提出の後の雛形の直しは **0 回**（要件 5.5 に当たる赤は無い）。PR の枝への載せ直しは 0 回・2 回目の写し戻しと見比べは 0 回・やり直しの `winget validate` は 0 回。
- 入れ方に関わる欄の直しは 0 なので、winget の設定の 2 回目のオンと戻しは **0 回**・手元で入れて起動する確かめの 2 回目は **0 回**（要件 3.4・3.6 に書き足す物は無い。手元の確かめの記録 `winget-local-check.md` は、このタスクでは変えていない）。
- 雛形の直しでは消えない赤は **0 件**（要件 5.6 に当たる物は無い）。開発者へ報告する原因は 0・`/kiro-discovery` での起票は 0 本。
- この段から、要件 6.3 の起票へ足す件は **0 件**（自動の検査で、areka の側の未対応や不具合が見つかったことは無い）。

## 完了の判定

結果: **完了の線を満たした**（自動の検査が通り、人の承認待ちになった。要件 5.7）。

| 項目 | 値 |
|---|---|
| 判定した時刻 | 2026-10-10 19:14:39〜19:14:43（+09:00。この時刻に読んだ PR の印で判定した）。19:16:54〜19:16:57 に読み直して、同じ判定 |
| そのときの印の一覧 | `Azure-Pipeline-Passed`・`Validation-Completed`・`New-Package`（3 つ。ほかの印は無い） |
| PR の状態 | 開いている（下書きではない・取り込まれていない・閉じていない）。人のレビューは 0 件で、レビューの欄は「承認が要る」（`REVIEW_REQUIRED`） |
| 判定した相手 | PR #450070 の枝の先頭 `077130407a9cc119eaab395db9916fb12ab8bdd4`（提出したときのまま） |
| 人の承認待ちになった時刻 | 2026-10-10 19:03:30（`Validation-Completed` が付いた時刻。PR ができてから 92 分 15 秒）。承認が要るという案内のコメントが付いたのは 19:06:01 |
| 判定した者 | タスク 4.3 の AI（読むだけ。写しは `target\winget-check\logs\4.3-pr-view.json`・`4.3-issue-events.json`・`4.3-judge.txt`） |

当てた決まり（設計の「設計で決めたこと」の 7）: PR に `Validation-Completed` の印が付き、`Needs-CLA`・`Needs-Author-Feedback`・失敗の印（設計の「赤の仕分け」の表）がどれも付いていないこと。

| 条件 | 判定のときの値 | 合否 |
|---|---|---|
| `Validation-Completed` が付いている | 付いている（19:03:30 に付いた。`Azure-Pipeline-Passed` も 19:03:29 に一緒に付いた＝設計が見込んだ並び） | 合 |
| `Needs-CLA` が付いていない | 付いていない（17:35:21 に外れて、そのまま） | 合 |
| `Needs-Author-Feedback` が付いていない | 付いていない（1 度も付いていない） | 合 |
| 失敗の印が付いていない | 0 件（1 度も付いていない） | 合 |

- 「読みに行ったときに、もう取り込みまで済んでいた」場合には当たらない（まだ取り込まれていない）。
- **人の承認による取り込みは待たない**（要件 5.7）。本仕様の完了の線は「自動の検査が通り、人の承認待ちになった」ことで、それは上のとおり満たした。取り込まれたかどうかは、本仕様の完了の条件に入らない。
- この判定は、上の枝の先頭のコミットに対する物。PR の枝へ新しいコミットが載ると、自動の検査はやり直しになる（本仕様では載せない）。

本仕様の外に残ること:

- モデレーター（有志）の承認と、その後の取り込み。19:06:01 の案内は、承認が要ることと、時間を見てほしいことを告げている。19:04:06 に、条件がそろったら自動で取り込む設定がオンになった、と出来事の記録から読める（上の「出来事の記録から読んだ印の移り変わり」）。いつ承認されるかは分からない。
- 取り込みの後の仕事（版を出すたびの更新の PR・説明書の winget の行）は、`areka-P0-winget-release-automation` の持ち物。その前に `areka-P0-user-data-root` の着地が要る（後ろの spec への申し送りの 6）。
- 本仕様が完了した後は、AI はこの PR を読みに行かない。PR の出し手 `ekicyou` は、この PR の知らせを受ける側に入っている（出来事の記録に `subscribed` が 6 件）。

この後に開発者が行うこと（起きたときだけ）:

- **直しを求める印 `Needs-Author-Feedback` が付いたら、10 日のうちに応える**。応えないと、PR は自動で閉じられる（上の「手順」の 8）。雛形の直しが要るときは、同じ PR の枝で直し、雛形へ写し戻す（要件 5.5）。
- 審査の人に、名乗りの頭（`Areka`）と発行者（`ekicyou`）の違いを問われたら、上の「手順」の 7 に用意した文で答える。

## 名前の直しの数

結果: **直した行は 3・直さなかった行は 2**（要件 6.4。2026-10-10・タスク 5.2）。相手は `areka-P0-release-cycle` の `requirements.md` と `design.md` の 2 ファイルだけ。

直す前の数（本仕様の名前が出る行。どの行も 1 行に 1 回なので、行の数と出る回数は同じ）:

| ファイル | 短い名前 `winget-manifest-submission` | 長い名前 `areka-P0-winget-manifest-submission` |
|---|---|---|
| `requirements.md` | 4 行 | 0 行 |
| `design.md` | 1 行 | 0 行 |

相手の文書は本仕様を、頭の `areka-P0-` が無い短い名前で書いている。長い名前で数えると 0 件になる。直す前の 2 ファイルに `areka-P0-winget-release-automation` は 0 回。

直した行（3 行。`winget.yml` の持ち主として本仕様の名前を挙げていた行）:

- `requirements.md` の「Out of scope」の、workflow の中身を並べた行（`winget.yml`＝ の右の名前）。
- `requirements.md` の「Adjacent expectations」の、「`winget.yml` を main へ入れた後の回から、見守る相手に winget-pkgs への PR が加わる」の行（行の頭の名前）。
- `design.md` の「winget の見守りの細部」の行（かっこの中の頭の名前）。

直さなかった行（2 行。どちらも初回の手提出のことを言っていて、それは本仕様の持ち物のまま）:

- `requirements.md` の「Out of scope」の、「winget のマニフェストの形と初回の手提出」の行。
- `requirements.md` の受け入れ条件の、「`winget.yml` が main に無い間は、初回の見守りの相手に winget を入れない（winget の初回の手提出は … の番）」の行。

直した後の数:

| ファイル | 短い名前（残り） | 長い名前 | `areka-P0-winget-release-automation` |
|---|---|---|---|
| `requirements.md` | 2 行 | 0 行 | 2 行 |
| `design.md` | 0 行 | 0 行 | 1 行 |

確かめたこと:

- 差分は名前の置き換えだけ。`git diff --numstat` は `requirements.md` が 2 行足して 2 行消し、`design.md` が 1 行足して 1 行消し。語の単位の差分で、消えた語は `winget-manifest-submission` が 3 回・足した語は `areka-P0-winget-release-automation` が 3 回で、ほかは 0。直した後の 2 ファイルで新しい名前を古い名前へ戻すと、直す前（HEAD）と 1 バイトも違わない。
- 手順の文は 1 字も変えていない（要件の「タグのコミットに `winget.yml` が在る」ときだけ winget-pkgs への PR を確かめる条件、「`winget.yml` が main に無い」間の条件、設計の見守りの表の winget の行と、その下の「`winget.yml` がタグのコミットに無い回」の文。どれも差分に出ない）。
- `brief.md` は相手にしていない（差分 0）。`brief.md` には短い名前が 7 行あり、そのまま残る。`areka-P0-release-cycle` のほかのファイル（`design-validation.md`・`research.md`・`spec.json`・`tasks.md`・`verification/first-run.md`）は、短い名前も長い名前も 0 行で、触っていない。
- 置き換えた名前は、頭の `areka-P0-` が付いた長い形。相手の文書は、隣の名前（`release-ci-workflow`・`crates-io-publish` など）を短い形で書いているので、ここだけ形がそろわない。要件 6.4 が直し先を `areka-P0-winget-release-automation` と名指ししているので、そのとおりにした。

## 範囲の確かめ

結果: **範囲に収まっている**（2026-10-10 19:20 ごろ〜19:35。レビューを受けた書き直しは 19:40 ごろ・タスク 5.3。要件 2.5・2.6・2.8・5.8・6.1）。枝の差分は 19 ファイルで、設計の「File Structure Plan」の 6 つの組のどれかに全部入る（外れた物は 0）。**変更 0 の対象は 0 件**。workflow と説明書の winget の行は作っていない。**トークンの形の文字列は 0 件**。2 つの記録は、設計の「記録の見出し」の節を全部持つ。**起票した数は 2**（今ある spec への割り付けは 2）。タスク 5.3 の AI が行ったのは、読むだけの確かめと、この節を書くことと、この記録の冒頭の箇条へ 1 文（「名前の直し（タスク 5.2）と、範囲の確かめ（タスク 5.3）も済んだ」）を足すこと（GitHub へは書き込んでいない。`gh`・winget・`wingetcreate`・areka は動かしていない）。あわせて、このタスクの終わりに、進行役の AI が 3 つを行った: `winget-local-check.md` の冒頭の「判定」の古い 1 文を書き替えた（下の 7 の最後）／この節の 10 の、レビューの差し戻しの行に補いを足した／`winget list` を 1 回打って、提出の道具が入ったままであることを読んだ（読むだけ。下の 8 の「開発機に残っている物」）。

### 1. 比べた元と、打ったコマンド

- 比べた元（枝が main から分かれた点）: `414d43ebefde99476c42e98fb87b1bb6c3c6e2d1`（PR #281 のコミット）。`git merge-base HEAD main` と `git merge-base HEAD origin/main` が同じ値を返した。
- 枝の先頭: `165ab9ef`（タスク 4.3 のコミット）。分かれた点からのコミットは 36。
- main は、その後 `05ee7902` まで進んでいる。この枝へは取り込んでいない。下の差分は、分かれた点から枝の先頭までの物（main の側の進みは入らない）。
- コマンド（ワークツリーの根・Git Bash）:

  ```bash
  B=$(git merge-base HEAD main)
  git diff --name-status "$B"...HEAD     # 19 行（足した 14・変えた 5・消した 0）
  git diff --numstat "$B"...HEAD
  git status --porcelain                 # 0 行（この節を書き始める前）
  ```

- この節を書いた後の、コミットしていない変更は 2 つ: この記録と、`winget-local-check.md`（どちらも、下の一覧にすでに在るファイル。`git status --porcelain` は、この 2 行だけ）。`winget-local-check.md` の変更は 1 行で、進行役の AI が行った（下の 7 の最後）。

### 2. 差分のファイル名の一覧と、入る組

組は、設計の「File Structure Plan」と要件 2.6 が挙げる 6 つ: ① 雛形（`dist/winget/**`）② 本仕様のフォルダの中の文書 ③ steering の `structure.md` ④ 後ろの spec の brief ⑤ `areka-P0-release-cycle` の要件と設計 ⑥ 起票（`/kiro-discovery`）が作った文書。

| 番号 | ファイル | 足した・変えた | 足した行 | 消した行 | 組 |
|---|---|---|---|---|---|
| 1 | `dist/winget/0.0.2/Areka.Areka.Portable.installer.yaml` | 足した | 21 | 0 | ① |
| 2 | `dist/winget/0.0.2/Areka.Areka.Portable.locale.en-US.yaml` | 足した | 25 | 0 | ① |
| 3 | `dist/winget/0.0.2/Areka.Areka.Portable.locale.ja-JP.yaml` | 足した | 15 | 0 | ① |
| 4 | `dist/winget/0.0.2/Areka.Areka.Portable.yaml` | 足した | 8 | 0 | ① |
| 5 | `.kiro/specs/areka-P0-winget-manifest-submission/requirements.md` | 足した | 157 | 0 | ② |
| 6 | `.kiro/specs/areka-P0-winget-manifest-submission/research.md` | 足した | 398 | 0 | ② |
| 7 | `.kiro/specs/areka-P0-winget-manifest-submission/design.md` | 足した | 571 | 0 | ② |
| 8 | `.kiro/specs/areka-P0-winget-manifest-submission/design-validation.md` | 足した | 60 | 0 | ② |
| 9 | `.kiro/specs/areka-P0-winget-manifest-submission/tasks.md` | 足した | 173 | 0 | ② |
| 10 | `.kiro/specs/areka-P0-winget-manifest-submission/spec.json` | 足した | 22 | 0 | ② |
| 11 | `.kiro/specs/areka-P0-winget-manifest-submission/verification/winget-local-check.md` | 足した | 1330 | 0 | ② |
| 12 | `.kiro/specs/areka-P0-winget-manifest-submission/verification/submission.md` | 足した | 726 | 0 | ② |
| 13 | `.kiro/steering/structure.md` | 変えた | 1 | 1 | ③ |
| 14 | `.kiro/specs/areka-P0-winget-release-automation/brief.md` | 変えた | 91 | 0 | ④ |
| 15 | `.kiro/specs/areka-P0-release-cycle/requirements.md` | 変えた | 2 | 2 | ⑤ |
| 16 | `.kiro/specs/areka-P0-release-cycle/design.md` | 変えた | 1 | 1 | ⑤ |
| 17 | `.kiro/specs/areka-P0-user-data-root/brief.md` | 足した | 149 | 0 | ⑥ |
| 18 | `.kiro/specs/areka-P0-write-failure-notice/brief.md` | 足した | 116 | 0 | ⑥ |
| 19 | `.kiro/steering/roadmap.md` | 変えた | 29 | 3 | ⑥ |

- 組ごとの数: ① 4・② 8・③ 1・④ 1・⑤ 2・⑥ 3 ＝ 19。**どの組にも入らないファイルは 0**。
- 行の数は、枝の先頭のコミットまでの物（12 番は、この節を足す前の数）。
- ひと言要る物:
  - 5〜10 番（要件・調べ・設計・設計の検査・タスク・`spec.json`）は、設計の「Directory Structure」の図には載っていない（図に在るのは記録の 2 つだけ）。本仕様そのものの文書で、要件 2.6 の「本仕様のフォルダの中の文書」に入る。
  - 本仕様の `brief.md` は、分かれた点にすでに在り、この枝では変えていない（差分に出ない）。
  - 14 番の brief は、**この枝が作った物ではない**。分かれた点のコミット（PR #281）が main の側で作った物で、この枝は末尾に書き足しただけ（`git log --diff-filter=A` が返すのは `414d43eb` の 1 つ）。
  - 19 番の `roadmap.md` は、要件 2.6 の「steering の roadmap まわりへの書き足し」（起票したときだけ）。
  - `areka-P0-mcp-kanade-tools` の brief は、件を 1 つ割り付けた相手だが、直していない（差分 0）。

### 3. 変えたファイル（本仕様のフォルダの外の 5 つ）の、変わり方

| ファイル | 設計が許す形 | 実際 | 判定 |
|---|---|---|---|
| `.kiro/steering/structure.md` | `dist/` の説明の 1 か所 | 1 行（「その他の最上位」の行）。行の末尾の `dist/` の説明に「と、`winget/`＝winget へ提出するマニフェストの雛形（初回に出した版の写し・配布 zip には入らない）」を足しただけ。その前の字は変わっていない | 合う |
| `.kiro/specs/areka-P0-winget-release-automation/brief.md` | 末尾への書き足しだけ（既にある文は変えない） | 消した行は 0。分かれた点の中身（9132 バイト）は、今の中身の頭の 9132 バイトと 1 バイトも違わない（`cmp` で同じ）。足したのは、日付の付いた節 1 つ（「`areka-P0-winget-manifest-submission` からの申し送り（2026-10-10）」）で、中の小見出しは 8 つ＝設計の 6 項目と、「この spec を始めるときに効く事実」「上の本文のうち、実測と合わなくなった所」 | 合う |
| `.kiro/specs/areka-P0-release-cycle/requirements.md` | 名前の 2 行だけ | 2 行。語の単位の差分で、消えた語は `winget-manifest-submission` が 2 回・足した語は `areka-P0-winget-release-automation` が 2 回。ほかは 0 | 合う |
| `.kiro/specs/areka-P0-release-cycle/design.md` | 名前の 1 行だけ | 1 行。同じ置き換えが 1 回。ほかは 0（数は上の「名前の直しの数」と同じ＝直した行 3） | 合う |
| `.kiro/steering/roadmap.md` | 起票の書き足し | 足した行 29・消した行 3。消した 3 行は、どれも数を書いた行の書き直し（spec 台帳の見出し＝brief を持つ spec の数 96 → 98／区分ごとの数の行／段ごとの数の行）で、古い数は「その前の数」として同じ行に残してある。足した 29 行は、その 3 行の新しい形・台帳の表の 2 行（`user-data-root`・`write-failure-notice`）・「テーマ別の決めごと」の末尾の節 1 つ（24 行。空行を含む）。台帳の表の行は、数え直して 98（分かれた点では 96） | 合う |

### 4. 変更 0 の対象

結果: **変更 0 の対象は 0 件**（要件 2.6。触らないと先へ進めない場面は無かった＝要件 2.8 で止める場合には当たらなかった）。

- コマンド: `git diff --name-only "$B"...HEAD -- README.md dist/README.txt .github/workflows crates tools` → 0 行（`git diff --stat` の同じ形も、出力は空）。対象ごとに分けて打っても、5 つとも 0 行。`.github` の全部でも 0 行。
- 各 `Cargo.toml` と `Cargo.lock`: `git diff --name-only "$B"...HEAD | grep -ic cargo` → 0。
- 配布 zip の中身（設計の「Out of Boundary」が挙げる 7 つ目）: zip を組む道具（`tools/**`）と、zip へ入る説明書（`dist/README.txt`）の差分が 0。zip が `dist/winget/` を足す前と変わらないことは、`winget-local-check.md` の「雛形の検査」の「zip が変わらないこと」で確かめてある。
- **0 と書く前に確かめたこと**（この形のコマンドが、変わった物を出せること・相手が実在すること）:
  - 同じ形に `dist/winget` を足すと 4 行を返した（`git diff --stat "$B"...HEAD -- dist/winget` の最後の行は「4 files changed, 69 insertions(+)」）。
  - 同じ `grep -ic` で `yaml` を数えると 4。
  - 対象は、どれも実在して追跡されている: `README.md` 1・`dist/README.txt` 1・`.github/workflows` 2・`crates` 2048・`tools` 297 ファイル。`Cargo.toml` は 31・`Cargo.lock` は 1。
- `dist/` の下で変わったのは、雛形の 4 ファイルだけ。

### 5. workflow と説明書の winget の行（要件 5.8）

- `.github/workflows/` に在るのは `crates-io.yml` と `release.yml` の 2 つで、分かれた点と同じ。**`winget.yml` は無い**（作っていない）。
- `README.md` と `dist/README.txt` は、差分が 0。今の 2 つのファイルに `winget` の字は 0 行（大文字と小文字の違いは問わない）。＝**説明書に winget の行は足していない**。
- 更新の PR を自動で出す仕組みと、説明書の winget の行は、`areka-P0-winget-release-automation` の持ち物のまま。

### 6. トークンの形の文字列（要件 2.5）

結果: **0 件**。

- 探した相手: 雛形の 4 ファイルと 2 つの記録（6 ファイル）。念のため、この枝が足した・変えたファイルの全部（上の一覧の 19 ファイル。この節を書いた後の、この記録も含む）。
- 探した形（正規表現は `grep -E`。この記録そのものが当たらないよう、頭の字はつなげて書かない）:

| 記号 | 何の形か | 正規表現 | 大文字と小文字 |
|---|---|---|---|
| ア | GitHub のトークン: `gh`＋1 字（`p`・`o`・`u`・`s`・`r` のどれか）＋下線で始まり、後ろに英数字が続く物 | `gh[pousr]_[A-Za-z0-9]` | 区別する（区別しない形でも見た） |
| イ | GitHub の細かい権限のトークン: `github`＋下線＋`pat`＋下線で始まり、後ろに字が続く物 | `git(hub)_pat_[A-Za-z0-9_]` | 同じ |
| ウ | 認証のヘッダーの値: `Bearer` の後ろに空白と 8 字以上の値 | `Bearer[[:space:]]+[A-Za-z0-9._~+/-]{8,}` | 区別しない |
| エ | パスワードや秘密の値を入れる書き方（名前の後ろに `:` か `=` と値） | `(password\|passwd\|pwd\|secret\|api[_-]?key\|access[_-]?token\|client[_-]?secret)[[:space:]]*[:=][[:space:]]*[^[:space:]]` | 区別しない |
| オ | トークンを入れる書き方（`token` の後ろに `=` と値） | `(TOKEN\|token)[[:space:]]*=[[:space:]]*[^[:space:]]` | 区別しない |
| カ | GitHub へのログインの 1 回限りのコードの形（大文字の英数字 4 字・`-`・4 字） | `(^\|[^A-Za-z0-9-])[A-Z0-9]{4}-[A-Z0-9]{4}($\|[^A-Za-z0-9-])` | 区別する |

（表の中の `\|` は、表の区切りと紛れないようにした書き方。打った正規表現では `|` の 1 字。）

- コマンドの形: `git diff --name-only "$B"...HEAD` の 19 ファイルをつなげて `grep -Ec -- '<正規表現>'`（区別しない形は `-Eic`）。6 ファイルは、1 つずつ `grep -Ec`。
- **0 と書く前に確かめたこと**: 形ごとに、当たる見本を 1 つずつ作って同じ `grep` に掛け、当たることを見た（見本は 10 個＝アが 5・イ・ウ・エ・オ・カが 1 つずつ。10 個とも、その形に当たった）。見本は端末の変数の中だけで作り、ファイルには書いていない。逆に、頭の字だけを引用の印で囲んだ説明の書き方は、アとイに当たらないことも見た（0）。

| 相手 | ア | イ | ウ | エ | オ | カ |
|---|---|---|---|---|---|---|
| 雛形の 4 ファイルと 2 つの記録（6 ファイル） | 0 | 0 | 0 | 0 | 0 | 0 |
| 枝が足した・変えた 19 ファイル | 0 | 0 | 0 | 0 | 0 | 0 |

- 頭の字だけ（後ろに何も続かない物）が出てくる行は、19 ファイルで 2 行（本仕様の `design.md` の「範囲の確かめ」の行と、`tasks.md` のタスク 5.3 の行）。どちらも「この頭で始まる物を探す」という説明で、トークンではない。雛形と 2 つの記録には 0 行。
- 「コード」の字と、カの形が同じ行に在る所は 0 行。提出のときに道具が出した 1 回限りのコードは、この記録に写していない（上の「道具が出した文」）。
- カを大文字と小文字を区別せずに探すと、`kiro-impl`・`Copy-Item`・`Test-Path` のような、ふつうの語が当たる（コードではない）。区別する形では 0。
- 追跡していない確かめの置き場（`target\winget-check\`。776 ファイル）も、ついでにアとイで探した: 当たったファイルは 0。

### 7. 記録の見出し（要件 6.1）

結果: **2 つの記録は、設計の「記録の見出し」が挙げる節を全部持つ**（欠けは 0）。

`winget-local-check.md`（設計の 11 項目）:

| 設計の項目 | 記録の見出し |
|---|---|
| 日付・機械と winget の版・コミット・判定 | 冒頭の箇条（日付・機械・コミット・判定・提出する版） |
| 変えた設定（表） | 「変えた設定」（表 1 つ） |
| 雛形の検査（`winget validate`・ハッシュの突き合わせ・持たない欄・zip が変わらないこと） | 「雛形の検査」と、その下の同じ名前の 4 つの節 |
| 入れて起動する（手順と結果） | 「入れて起動する」（1〜5） |
| 上げ直しの実測 | 「上げ直しの実測」（「作る状態」「上げ直し」） |
| 外し方の実測 | 「外し方の実測」（1〜7） |
| 機械の全員向けの実測 | 「機械の全員向けの実測」（1〜8） |
| 後片付けの確かめ | 「後片付けの確かめ」（「利用者の側」「機械の側」） |
| 置き場 | 「置き場（要件 3.5）」 |
| 既知の制限 | 「既知の制限」 |
| 見つかった件と起票 | 「見つかった件と起票」 |

`submission.md`（設計の 9 項目）: 「手順」「提出した版と日時」「PR の URL」「印と検査の移り変わり」「写し戻しの見比べ」「赤と、その扱い」「完了の判定」「名前の直しの数」「範囲の確かめ」。9 つとも、設計と同じ名前の見出しで在る。

要件 6.1 が求める中身:

| 求める物 | 在る所 |
|---|---|
| 機械と winget の版 | 2 つの記録の冒頭（開発機・Windows 11 Pro・x64・winget v1.29.380） |
| コミット | `winget-local-check.md` の冒頭は `f305cee6`（着手のときの枝の先頭＝タスクを作ったコミットで、タスク 1.1 のコミットの 1 つ前）。この記録の冒頭は `1c1ad468`（この記録を作ったときの枝の先頭＝タスク 4.1 のコミットの 1 つ前）。どちらも、この枝の履歴に在ることを見た |
| 変えた設定と戻した時刻 | `winget-local-check.md` の「変えた設定」（`LocalManifestFiles` の 1 つだけ。オンは 12:56:29〜14:43:20・戻した時刻は 14:43:20・戻した後の値は `false`） |
| 手順と結果 | 2 つの記録の各節 |
| PR の URL | この記録の「PR の URL」（https://github.com/microsoft/winget-pkgs/pull/450070 ） |

- どちらの記録にも、後から埋める印（「タスク … で記入」の形）は残っていない。
- `winget-local-check.md` の冒頭の「判定」の箇条は、このタスクの終わりまで、最後の文が「まだ済んでいないのは、提出（タスク 4.2・4.3）」のままだった（その箇条を最後に書いた時点＝タスク 3.6 の文。提出と自動の検査は、その後に済んでいた）。このタスクの確かめが済んだ後（19:30 ごろ）に、**進行役の AI がその 1 文を書き替えた**: 今の文は「提出も済んだ」で始まり、PR の URL・自動の検査が 10 段とも通ったこと・19:03:30 に `Validation-Completed` が付いて人の承認待ちになったこと・この記録の名前を書いている。`git diff -- <その記録>` で見ると、変わったのはその 1 行だけ（足した行 1・消した行 1。語の単位で見ても、替わったのは最後の 1 文だけで、その前の字は同じ）。タスク 5.3 の AI は、`winget-local-check.md` を直していない。

### 8. 「未確認」「測れなかった」の項目の一覧

2 つの記録から、測れなかった物・測っていない物・写しが残っていない物・理由が分からない物を集めた（開発者へ報告する一覧。33 件＝測れなかった 1・測っていない 14・写しが残っていない、聞いて書いた 13・理由が分からない、調べていない 5）。「未確認」の字で書いた項目は 0。「測れなかった」は 1 種類（1 番）。

測れなかった物:

| 番号 | 何が | 理由 | 書いてある所 |
|---|---|---|---|
| 1 | シェルの記憶（えも？？ の分とクローディアの分の 2 か所）が、上げ直しと外し方で残るか | 0.0.2 には、シェルの記憶へ書く操作が無く、作れなかった。シェルのフォルダの行方は見た（`ghost\` と一緒に消える・置き換わる） | `winget-local-check.md` の「上げ直しの実測」の「作る状態」と「上げ直し」・「外し方の実測」の 5・「既知の制限」 |

測っていない物（読みや見込みだけ）:

| 番号 | 何が | 理由 | 書いてある所 |
|---|---|---|---|
| 2 | arm64 の項目を実機で入れること | 要件 3.7 で含めないと決めた。確かめたのは `winget validate` とハッシュの一致まで | 同「既知の制限」 |
| 3 | winget-pkgs から入れた形での上げ直しと外し方 | 取り込みの前なので、その形では入れられない。測ったのは手元のマニフェストで入れた形（入れ先の名前と ID が違う） | 同上（要件 4.5） |
| 4 | winget の設定「外すときに入れ先を丸ごと消す」をオンにした機械での外し方 | 既定のまま（オフ）で測った。オンなら丸ごと消える見込み | 同上（要件 4.5） |
| 5 | 中身の違う新しい版へ替わるところ | 確かめ専用のマニフェストは、版の欄だけを変えた物で、zip は 0.0.2 と同じ | 同上 |
| 6 | 提出する版 0.0.2 を入れたままの物を外すこと | 外したのは、確かめ専用の版へ上げ直した後の物 | 同上 |
| 7 | winget が英語の注意書きを出すところ | 開発機の言語は日本語で、出たのは日本語の側だけ | 同上 |
| 8 | 注意書きを出さない設定をオンにした機械で、注意書きが出ないこと | winget の文書とソースの読みだけ | 同上 |
| 9 | 注意書きを足した雛形を、何も残っていない機械へ入れること | 入れ直しは、外し方の実測の残りの上へ行った | 同上 |
| 10 | 利用者向けに、何も残っていない機械へ入れて `--purge` で外したとき、後片付けの 4 項目が 0 になるか | 同じ理由。ソースの読みでは、フォルダは消えて PATH の項目は残る見込み（機械の全員向けでは、その形を 1 回見た） | 同「後片付けの確かめ」の「利用者の側」の 7・「機械の側」・「既知の制限」 |
| 11 | 利用者の側の後片付けの 4 項目のうち、`areka` の解決と `winget list` の行を、片付けた後に読み直すこと | そのときには機械の全員向けの入れ方が済んでいて、読むと機械の側の物が当たる。根拠は `--purge` の直後（14:09）の読み | 同「既知の制限」 |
| 12 | 機械の全員向け: 入れた管理者と普段の利用者が別のアカウントの機械 | 開発機 1 台で、同じアカウント。別のアカウントでは、記憶が書けず、起こせないおそれも在る（権限の一覧からの読み） | 同「機械の全員向けの実測」の 7・「既知の制限」 |
| 13 | 機械の全員向け: バルーンの `.nar` を入れること・ゴーストを切り替えること | 試したのは 1 回・検体 1 つ（ゴースト）。切り替える先のゴーストが無い | 同「既知の制限」 |
| 14 | 機械の全員向けに入れた形での上げ直し | 測ると決めていない | 同上 |
| 15 | 開いたままのアプリが持っている古い PATH | 見たのは、登録から組み直した PATH（新しい端末が受け取る物） | 同上 |

写しが残っていない物・聞いて書いた物:

| 番号 | 何が | 理由 | 書いてある所 |
|---|---|---|---|
| 16 | 機械の全員向けに入れた回・外した回・設定を戻した回に、winget が出した文 | 開発者の管理者の端末に出た。代わりに winget 自身の記録を読んだ | 同「変えた設定」・「機械の全員向けの実測」の 2 と 8・「既知の制限」 |
| 17 | 設定をオンにしたときの winget の答えの 1 行 | 開発者から聞いた物 | 同「変えた設定」 |
| 18 | 機械の全員向けに入れた直後の読み（14:19:22。`winget list` の行と、機械の PATH の登録の数＝19 項目） | 進行役の AI の読みを聞いて書いた（その回の出力のファイルは無い）。残っているファイルで確かめられたのは、14:20:44 に登録から組み直した PATH と、入れ先の一覧の時刻まで | 同「機械の全員向けの実測」の 2・「既知の制限」 |
| 19 | 機械の全員向けに入れた後・起こす前の、入れ先の一覧 | 置き場の指定を誤って保存できなかった。数（150 ファイル）と直下の名前だけ | 同「既知の制限」 |
| 20 | 入れられなかったときのゴーストの台詞の、画面の写し | 写しのファイルが残っていない（進行役の AI が画面で見た 2 行）。`dump_balloon` は呼んでいない | 同上 |
| 21 | 入れ先の権限の一覧を取ったコマンド | 残っていない（一覧は 7 か所だけ） | 同上 |
| 22 | 機械の PATH を手で直した時刻 | winget を通らないので記録が無い。前後の読みの間（14:43:46〜14:46:58）としか言えない | 同上・「機械の全員向けの実測」の 8 |
| 23 | 機械の側の後片付けの途中の読み（14:41〜14:46）と、起票の前の設定の読み（15:47:38） | 進行役の AI の読みを聞いて書いた（出力のファイルは無い） | 同「既知の制限」・「見つかった件と起票」 |
| 24 | 提出の直前の読み直し（17:15:07）の出力 | ファイルに取っていない（進行役の AI の読み） | この記録の「直前の確かめ」の「提出の直前の読み直し」 |
| 25 | `wingetcreate token --clear` の出力 | 同じ | この記録の「覚えたログインを消した」 |
| 26 | 進行役の AI の区切りごとの読み（17:31〜19:13） | 同じ。出来事の記録とは食い違わない | この記録の「進行役の AI の読み（区切りごと）」 |
| 27 | 同意（CLA）の文を誰の手で送ったかと、開発者の言葉（「任せる」「個人として」） | GitHub からは確かめられない。進行役の AI から聞いた話 | この記録の「手順と違ったこと」 |
| 28 | 提出のときに道具が出した文の写しの 7 行目 | 1 回限りのコードを伏せた物で、道具が出したままではない | この記録の「道具が出した文」 |

理由が分からない物・調べていない物:

| 番号 | 何が | 理由 | 書いてある所 |
|---|---|---|---|
| 29 | 上げ直しの後の起動で、前のゴーストが無いことを、areka が台詞で伝えたか | 10 秒で終わる起動で、台詞の中身は取っていない | `winget-local-check.md` の「上げ直しの実測」の「上げ直し」 |
| 30 | 状態を作った 3 回目の起動だけ、最後の記録の行からプロセスが消えるまで 6.1 秒掛かった理由 | 調べていない。その後の起動では出ていない（起票していない） | 同「作る状態」・「見つかった件と起票」 |
| 31 | 機械の全員向けの入れ先で、下の階層にだけ利用者の権限の行が在る仕組み | 調べていない | 同「機械の全員向けの実測」の 7・「既知の制限」 |
| 32 | 自動の検査の 08 が 76 分 28 秒掛かった理由 | 検査の中の記録は読んでいない。結果は成功 | この記録の「検査の 08 に掛かった時間」 |
| 33 | モデレーターの承認がいつになるか | 分からない（本仕様の完了の線の外） | この記録の「完了の判定」 |

開発機に残っている物（測り残しではないが、開発者へ伝える物）:

- `C:\Program Files\WinGet\` と、その下の空のフォルダ `Links\`・`Packages\`（`winget-local-check.md` の「既知の制限」の最後。消すかどうかは開発者に任せてある）。
- 提出の道具 `wingetcreate`（`Microsoft.WingetCreate` v1.12.13.0）。**入ったまま**。読んだのは進行役の AI（2026-10-10 19:38:44 +09:00。タスク 5.3 の AI は winget を動かしていないので、聞いて書いた）: `winget list --id Microsoft.WingetCreate --exact --disable-interactivity` は、`Windows Package Manager Manifest Creator`・ID `Microsoft.WingetCreate`・版 `1.12.13.0`・ソース `winget` の 1 行を返し（終了コード 0）、`wingetcreate` の解決先は `%LOCALAPPDATA%\Microsoft\WindowsApps\wingetcreate.exe`。入れたのはタスク 4.1（この記録の「道具の用意」）。道具が覚えたログインは、17:31:47 に消してある（この記録の「覚えたログインを消した」）。外すかどうかは、開発者に任せる。

### 9. 起票した数

- **起票した数は 2**: `areka-P0-user-data-root`（利用者の物を、入れ先の外の書ける根へ出す。区分 A）と `areka-P0-write-failure-notice`（書けなかったときの伝え方を直す。区分 B）。どちらも brief だけ（上の一覧の 17・18 番）。
- **今ある spec への割り付けは 2**: 0.0.2 の MCP の `sakurascript` が仮の受け口である件 → `areka-P0-mcp-kanade-tools`（brief は直していない）／winget で外した後に PATH の項目が残る件 → `areka-P0-winget-release-automation`（申し送りの節に書いた。上の一覧の 14 番）。
- 照らした元は `winget-local-check.md` の「見つかった件と起票」（6 行。どの行にも spec の名前が入っている）。4 つの brief は、4 つとも在る。
- 提出の後の自動の検査からの起票は 0（上の「赤と、その扱い」）。
- `areka-P0-user-data-root` の向きは起票のときの推しで、開発者の裁定はまだ受けていない（その spec の要件の討議で受ける）。

### 10. 設計とタスクの書き方と違ったこと

詳しくは、指した先に書いてある。ここは一覧だけ。

- **実測の元になる状態は、台本でなく右クリックメニューで作った**。メニューを動かしたのは、開発者に `areka.exe` の画面の操作を許された AI。0.0.2 の MCP の `sakurascript` が仮の受け口だったため → `winget-local-check.md` の「上げ直しの実測」の「作る状態」・「既知の制限」。
- **シェルの記憶は測れなかった** → 上の 8 の 1 番。
- 今のゴーストの名前は、`get_status` でなく `get_active_ghost_list` で読んだ（0.0.2 の `get_status` は名前を返さない）→ 同「既知の制限」。
- **winget で外すだけでは後片付けが済まなかった**。入れ先のフォルダは AI が消し、利用者の PATH と機械の PATH の項目は開発者が手で消した → 同「後片付けの確かめ」。
- **開発者に頼んだ回数は、設計の見込みより多かった**。設計の見込みは、提出を含めて 4 回（＝提出の前までは 3 回）。実際は、提出の前までで 4 回だった: ① 設定をオンにする ②′ 利用者の PATH の項目を消す（winget が消さなかったため）② 機械の全員向けに入れる ③ 機械の全員向けを外す・機械の PATH の項目を消す・設定を戻す（この ③ は 1 度で済まず、やり取りが 3 往復になった）。そのうえで、提出（④）は、開発者の指示で AI がコマンドを打ち、開発者が行ったのはブラウザでのログインと許可だけだった（次の行）→ 同「既知の制限」の終わりから 2 つ目・この記録の「手順と違ったこと」。
- **提出のコマンドは、開発者でなく AI が打った**（開発者の指示。要件 5.2 と設計の「誰の手で行うか」から外れた）。覚えたログインを消す・PR の本文の印・同意（CLA）の文も、開発者に任されて AI が行った。AI はトークンを見ていない → この記録の「手順と違ったこと」。
- **`gh` を書き込みに 3 回使った**（開発者の指示。設計の「Technology Stack」は、`gh` を「書き込みには使わない」と書いている）: PR の本文の書き替えが 2 回（17:32:42 に確かめ項目の印 5 つと説明の 1 行・17:35:42 に同意の項目の印）と、同意（CLA）のコメントが 1 回（17:35:01）。行ったのは進行役の AI → この記録の「手順と違ったこと」・「印と検査の移り変わり」。
- タスクの順: 4.1（手順書と道具の用意）と 5.2（名前の直し）は、設定の変更が要らないので、開発者の 1 回目の操作を待つ間に先に済ませた。5.1（申し送り）は、自動の検査を待つ間に先に書き、4.3 の後に「人の承認待ちになった時刻」を足す形にした → `tasks.md` の「Implementation Notes」。
- タスク 3.5 の実機の部分は、設定がオンの時間を縮めるため、進行役の AI がタスク 3.4 の記録の仕上げと並べて行った → 同上。
- タスク 3.4 で、後片付けが 0 にならなかったときのデバッグ係の起動を 1 回省いた（原因が、ソースと winget 自身の記録で特定できていたため）→ 同上・`winget-local-check.md` の「後片付けの確かめ」の「利用者の側」の 7。
- タスク 2.1 の読み取りで、`winget list` に同意を先回りして与える旗を付けて 2 回打っていた（入れる操作ではない。以後は付けていない）→ `tasks.md` の「Implementation Notes」。
- タスク 4.2 で、レビューの差し戻しが 1 回在り、直してから通った（進行役の AI から聞いた。差し戻しの中身は、このタスクの AI は直に見ていない）。進行役の AI による補い: 差し戻しの相手は記録の 3 行だけで、雛形と見比べは 1 回目から合格だった。①② 提出の道具の出力の写し（`4.2-submit.stdout.txt`）について、記録は「1 回限りのコードの行が残っている」と書いていたが、記録を書いた後の 17:42:54 に進行役の AI がその 1 行を伏せていたので、事実と合わなくなっていた（この記録と `winget-local-check.md` の「置き場」の 2 か所）③ 同意（CLA）の行に、開発者の言葉でない引用が、出どころの印なしで書かれていた。3 行を直して、レビュー係が見直し、通った。
- 提出の直前の読み直しと、進行役の AI の区切りごとの読みは、出力をファイルに取っていない → 上の 8 の 24・26 番。
- `areka-P0-release-cycle` へ入れた名前は、頭の `areka-P0-` が付いた長い形（相手の文書の隣の名前は短い形）→ 上の「名前の直しの数」。
