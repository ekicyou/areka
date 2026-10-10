# Brief: areka-P0-winget-release-automation

> 2026-10-10 棚卸㉓で `areka-P0-winget-manifest-submission` から切り出した。理由は大きさでなく人の待ち: winget-pkgs への初回の提出は人の承認を約 2 日待つので、その後でしか動かせない仕事（更新の PR を自動で出す workflow と、説明書の winget の行）を別の spec にした。配布と公開のテーマ（roadmap「配布と公開」）の 1 本。出どころは開発者の依頼「winget 対応インストーラーを作成したい」。

## Problem

- **利用者**: 説明書に winget での入れ方が書かれていない。初回の提出が取り込まれても、`winget install Areka.Areka.Portable` で入れられることを知る手段が無い。
- **開発者**: リリースのたびに、winget-pkgs へ更新の PR を手で出すことになる。

## Current State

今の木（main `ee3af616`）で確かめたこと。

- `.github/workflows/` は `release.yml`・`crates-io.yml` の 2 本。`winget.yml` は無い。`release.yml` は、タグ `v*` の push と、手での起動（main での乾いた走り）の両方で動く。
- `README.md` の「## 入手と起動」の最初の箇条は「GitHub Releases から配布物の zip を入手して展開する」。winget の行は無い。
- `dist/README.txt` の「■ 入手のしかた」は箇条 1 つ（配布の zip）。「■ 既知の制限」の先頭は「areka.exe には署名がありません」。どちらにも winget の行は無い。
- `release-cycle` の決まり（`.kiro/specs/areka-P0-release-cycle/requirements.md` の「見守り」の受け入れ基準と、`design.md` の「緑の判定」の表）: **タグのコミットに `winget.yml` が在るときだけ**、winget-pkgs へその版の PR が出たことを確かめる。`winget.yml` が main に無い間は、winget を見守りの相手に入れない。Release の公開より後の段（winget など）が赤なら、同じ版で出し直さずに、原因を直す spec を起票する。＝`winget.yml` が main に入った次の回から、手順を直さずに見守りの相手が 1 つ増える。
- 更新の PR を出す道具（`winget-releaser`＝GitHub Action）は、winget-pkgs に**既に在る**名乗りの更新しかできない。前提は「既に 1 版が在ること」「開発者のアカウントに winget-pkgs のフォークが在ること」（切り出し元の brief の Current State。出典は roadmap「配布と公開」の節）。
- `GITHUB_TOKEN` が作った Release では `release: published` が飛ばないので、`winget.yml` は `workflow_run` で `release.yml` の終わりを受ける（2026-10-03 `release-ci-workflow` の裁定）。

この spec に ukadoc の正典は関わらない。winget と `winget-releaser` の決まりは、この brief では外の文書を引き直していない（切り出し元の brief と roadmap の記述のまま）。要件の段で引き直す。

## Desired Outcome

- タグを打って `release.yml` が緑で終わると、`winget.yml` が winget-pkgs へその版の更新の PR を出す。main での乾いた走りでは PR を出さない。
- `README.md` と `dist/README.txt` に、winget での入れ方の 1 行と、既知の制限が入る。
  - スタートメニューにアイコンは出ない（コマンド名 `areka` か、インストール先のフォルダから起動する）。
  - Smart App Control を有効にしている環境では、未署名の exe が止まる。
  - 上げ直しのときの利用者のゴーストと記憶の扱い（`winget-manifest-submission` の実測の結果を受けて書く）。
- 次の `release-cycle` の回で、winget-pkgs への PR が見守りの相手に入り、緑になる。

## Approach

- `.github/workflows/winget.yml`（新規）: `workflow_run` で `release.yml` の終わりを受け、「元の走りが push で、タグ（`head_branch` が `v*`）で、緑のとき」だけ進む。中身は `winget-releaser`。zip を拾うために `installers-regex: '\.zip$'` を指定し、版は `workflow_run` の `head_branch` から取る。`max-versions-to-keep` を決める。
- トークン（classic PAT・`public_repo` と `workflow`）は開発者が secret に置く。リポジトリには書かない。
- 説明書の 2 つのファイルに行を足す。足す場所は、`crates-io-publish` が用意した箇条書きの形に合わせる。

## Scope

- **In**: `.github/workflows/winget.yml`・`README.md` の「## 入手と起動」の箇条・`dist/README.txt` の「■ 入手のしかた」の 1 行と「■ 既知の制限」の行・次のリリースの回で PR が出たことの確かめ。
- **Out**: マニフェストの形・手元の `--manifest` での確かめ・初回の手提出（`winget-manifest-submission`）。Release を作る側（`release.yml`）。リリースの手順（`release-cycle`。この spec は手順の文書に触らない）。署名（`release-code-signing`）。インストーラー版（名乗り `Areka.Areka`）。

## Boundary Candidates

- 提出の自動化（`winget.yml`）。
- 説明書の行（`README.md`・`dist/README.txt`）。

## Out of Boundary

- `release.yml`・`crates-io.yml` の中身。
- `dist/winget/**`（マニフェストの雛形。`winget-manifest-submission` の持ち物。更新の PR は winget-pkgs の側の今のマニフェストを元に作られる見込みなので、雛形を版ごとに書き換える仕事は持たない。要件の段で確かめる）。

## Upstream / Downstream

- **Upstream**: `winget-manifest-submission`（完了していること）と、**winget-pkgs が初回の提出を取り込んでいること**（`winget search Areka.Areka.Portable` で引けること）。`release-ci-workflow`・`release-package-versioned`（完了）。
- **Downstream**: `release-cycle` の次の回（見守りの相手が増える）。`release-code-signing`（「既知の制限」の同じ節）。

## Existing Spec Touchpoints

- **Extends**: `winget-manifest-submission` の範囲から、`winget.yml` と説明書の行を引き取る。
- **Adjacent**: `dist/README.txt` を触る `mcp-stdio-bridge`・`install-live-target-hazards`・`release-code-signing`（「■ 既知の制限」）と、別の節を触る見込みの `extra-character-windows`・`property-name-case-fold`・`update-check-options`＝同じウェーブに置くなら節が重ならないことを照らす。`release-cycle`（`README.md` の同じ箇条を初回に直した。今はもう触らない）。

## Constraints

- 触るのは `.github/workflows/winget.yml`（新規）・`README.md`・`dist/README.txt` だけ。`crates/`・`tools/`・`dist/winget/**` には触らない。
- 秘密をリポジトリに置かない。
- **取り込みの前に着手しない**。取り込みの前に `winget.yml` が main に入ると、その間に打ったタグで `winget.yml` が赤になり、`release-cycle` が直す spec を起票することになる。取り込みの前に `winget install` の行を説明書に載せると、利用者が試して失敗する。
- 一度出した名乗り `Areka.Areka.Portable` は変えない。
- 道具が端末へ出す文は ASCII だけにする（workflow の段の中で文字コードを書き替えない）。

## 2026-10-10 棚卸㉓の測定（main `ee3af616`）

- **触るファイル**:
  - `.github/workflows/winget.yml`（新規）
  - `README.md`（「## 入手と起動」の箇条に 1 行）
  - `dist/README.txt`（「■ 入手のしかた」に 1 行・「■ 既知の制限」に 2〜3 行）
  - リポジトリの外（開発者の手が要るもの）: secret に置く classic PAT（`public_repo`・`workflow`）。winget-pkgs のフォークは `winget-manifest-submission` の初回の提出で作ったものを使う。
- **規模**: 3〜4 タスク。
- **先に要るもの**: `winget-manifest-submission` の完了と、winget-pkgs による初回の提出の取り込み（人の承認・実例で約 2 日）。開発者の PAT の登録。
- **優先度の区分**: A（開発者「winget 対応インストーラーを作成したい」）。
- **要件定義のモデル**: Opus。
- **議題**（答えで作業が変わるものだけ）:
  1. `max-versions-to-keep` の数。
  2. 初回の提出の後、この spec が着地するまでに出た版を、手で winget-pkgs へ出すか、次の版まで待つか。
  3. 上げ直しで利用者のゴーストと記憶が消えると `winget-manifest-submission` の実測で分かったとき、説明書にどう書くか（消える前に写しておく手順を書くか）。置き場を変える仕事は別の spec になる。
- **書き換えが要る相手（この brief では触っていない）**: `release-cycle` の要件・設計が `winget.yml` の持ち主を `winget-manifest-submission` と書いている。持ち主の名前がこの spec に替わるだけで、手順は変わらない。

## `areka-P0-winget-manifest-submission` からの申し送り（2026-10-10）

`areka-P0-winget-manifest-submission` の手元の確かめと初回の提出で分かったことを、この spec へ渡す（同 spec の要件 6.2・設計「文書の直し」の 6 項目）。値の出どころは、同 spec の `verification/winget-local-check.md`（以下「確かめの記録」）と `verification/submission.md`（以下「提出の記録」）で、かっこの中はその記録の節の名前。測ったのは開発機の 1 台（Windows 11 Pro・x64・winget v1.29.380）と、手元のマニフェストから入れた areka 0.0.2。この節より上の本文は変えていない。

### 1. 上げ直しと外し方の実測の結果

項目ごとの結果（確かめの記録「上げ直しの実測」の「上げ直し」の 7・「外し方の実測」の 5）。上げ直しは `winget upgrade --manifest`、外し方は `--purge` も `--preserve` も付けない `winget uninstall`（利用者の設定 `uninstallBehavior.purgePortablePackage` は既定のオフ）。どちらも止まらず、何も聞かれず、終了コード 0 で終わった。

| 項目 | 上げ直し | 外し方 |
|---|---|---|
| 後から入れたゴースト（`ghost\claudia\`） | 消えた | 消えた |
| 後から入れたバルーン 3 つ（`balloon\emo2-kakukaku-wplimit\`・`balloon\claudia\`・`balloon\claudia_vertical\`） | 消えた（3 つとも） | 消えた（3 つとも） |
| areka の記憶（`profile\areka\`） | 残った | 残った |
| ゴーストの記憶 2 つ（えも？？ の分・クローディアの分） | 消えた（2 つとも） | 消えた（2 つとも） |
| シェルの記憶 2 か所 | 測れなかった | 測れなかった |
| 同梱のファイル（目印を付けた `ghost\emo2\readme.txt` と、足した目印のファイル） | 置き換わった | -（表に無い。zip の中の物は、外したので全部無くなった） |
| 上げ直しの後の起動 | 別のゴーストで立った（前はクローディア・立ったのは同梱の えも？？。終了コード 0） | - |
| 入れ先のフォルダ | - | 残った（中は areka の記憶の 1 ファイルと、空の `.nar-work\`） |

- **消えた物は、上げ直しで 6・外し方で 6**（同じ 6 つ＝後から入れたゴースト 1・後から入れたバルーン 3・ゴーストの記憶 2）。残った物は 1（areka の記憶）。測れなかった物は 2（シェルの記憶。0.0.2 にはシェルの記憶へ書く操作が無い。シェルのフォルダは `ghost\` の下に在る）。ファイルの数では、上げ直しで 208、外し方で 357（うち利用者の物は 207）。
- 見えた形: winget は、zip の中に在る直下のフォルダ `ghost\`・`balloon\` を、後から置かれた物ごと丸ごと消す（上げ直しでは、その後に zip の中身だけを置き直す）。zip の中に無い直下の物（`profile\`・`.nar-work\`）には触らない。
- **winget の文は、どちらのときも、利用者の物を消したことを告げない**。上げ直しの文には、古い版を外した・ファイルを消したと告げる行が 1 行も無い。外し方の文は「ファイルはインストール ディレクトリに残ります」とは告げる。
- 上げ直しの後、areka は、覚えていたゴーストが無いことを起動の記録の `WARN` 1 行（`last_ghost_not_found`）に残して、同梱のゴーストで立った。そのとき利用者へ台詞で伝えたかどうかは、読んでいない。

### 2. 機械の全員向け（`--scope machine`）の実測の結果

5 項目（確かめの記録「機械の全員向けの実測」の 6）。入れたのは開発者（管理者の端末）。起こしたのは普段の権限で 2 回。

| 項目 | 結果 |
|---|---|
| 入れ先のフォルダの場所 | `C:\Program Files\WinGet\Packages\Areka.Areka.Portable__DefaultSource` |
| ゴーストが立ったか | 立った（2 回とも同梱の えも？？・終了コード 0） |
| areka の記憶が書けたか | 書けなかった（`profile\areka\sylphya.toml` への書き込みが「アクセスが拒否されました」。1 回の起動につき 3 回。`%LOCALAPPDATA%\VirtualStore\` にも無い） |
| ゴーストを後から入れられたか | 入れられなかった（入れ先の直下に作業フォルダ `.nar-work\` を作れずに止まった。試したのは `claudia.nar` の 1 回だけ） |
| うまくいかなかったときに areka が利用者へ伝えたこと | 入れられなかったこと: ゴーストの台詞で伝えたが、口にした理由は「ファイルが壊れてるのかもね。」で、本当の理由（入れ先に書けない）は起動の記録にしか出ていない。記憶が書けなかったこと: 何も伝えていない |

- 表の外で分かったこと（同じ節の 7）: ゴーストごとの記憶（`ghost\` の下）は、この機械では書けた。起動中の印は書くことも消すこともできず、前回がきれいに終わらなかったことに気付く仕組みが、黙って働かない。
- **注意**（確かめの記録「既知の制限」）: 1 台の機械での結果で、その機械は、入れた管理者と普段の利用者が同じアカウント。入れた管理者が別のアカウントの機械では、`ghost\` の下の記憶も書けない見込みで、起こすこともできないおそれが在る。**別のアカウントでは測っていない**（権限の一覧からの読み）。機械の全員向けでの上げ直しも測っていない。入れたときに winget が出した文と、画面で見た台詞の写しは、ファイルに残っていない。
- 外したとき（開発者が管理者の端末で `--purge` を付けて）: 入れ先のフォルダは消えたが、機械の PATH の項目 1 件が残った（下の 3）。空のフォルダ `C:\Program Files\WinGet\` と、その下の `Links\`・`Packages\` も残る（確かめの記録「後片付けの確かめ」の「機械の側」）。

### 3. `ArchiveBinariesDependOnPath: true` を付けたことと、`areka` の解決のされ方

- 雛形は `ArchiveBinariesDependOnPath: true` を持つ（同 spec の要件 1.6）。
- 解決のされ方（確かめの記録「入れて起動する」の 2〜4）: winget は、入れ先のフォルダを利用者の PATH の登録の末尾へ 1 件足す（12 項目 → 13 項目）。新しい端末の `areka` は、入れ先のフォルダの中の `areka.exe` そのものに解決する（リンクではない。`%LOCALAPPDATA%\Microsoft\WinGet\Links\areka.exe` は作られない）。Windows の開発者モードはオフのままで通った。機械の全員向けでは、機械の PATH の登録の末尾へ 1 件足す（18 項目 → 19 項目）。
- 受け入れた代償（同 spec の設計「Security Considerations」の「PATH」）: 入れ先のフォルダが丸ごと PATH に載るので、`shiori-host32-helper.exe` も名前で呼べるようになる。
- **実測で分かった代償: winget で外しても、PATH の項目が残る**（確かめの記録「後片付けの確かめ」の「利用者の側」の 6・7 と「機械の側」／「見つかった件と起票」の 3 行目）。
  - 利用者の側: `--purge` を付けない回も付けた回も、もう無い `areka.exe` の在ったフォルダを指す項目 1 件が、利用者の PATH に残った。機械の側: `--purge` を付けた回に、機械の PATH の項目 1 件が残った。どちらも、開発者が手で消した。
  - 入れ先のフォルダは、areka の記憶（`profile\areka\`）が中に在ると残った（`--purge` を付けない回。`--purge` を付けた回も、残りの上へ入れ直した物だったので残った）。機械の側の `--purge` では消えた。
  - winget の文は、PATH の項目が残ることを告げない（フォルダについては、`--purge` を付けない回だけ「ファイルが入れ先に残る」と告げる）。
  - 原因（winget v1.29.380 の `src/AppInstallerCLICore/PortableInstaller.cpp` の読み）: PATH の項目を消す関数 `RemoveFromPathVariable` は、入れ先のフォルダが在って空でないと、記録に `Install directory is not empty` と書くだけで何もしない。呼ばれる時点のフォルダには、少なくとも winget 自身の控えが在る。だから、`--purge` の有無にも areka の記憶の有無にもよらず、残る見込み。
  - これは winget の側の動きで、areka からは直せない。**PATH の項目が残る分は、この spec が、説明書（`README.md`・`dist/README.txt`）の既知の制限の材料として受け取る**（「見つかった件と起票」の 3 行目の割り付け）。入れ先のフォルダが残る分は、`areka-P0-user-data-root` の持ち物。
  - 測った形の限り: 利用者の側で見たのは 2 回（`--purge` を付けない回と付けた回）で、何も残っていない機械へ入れて外す形は、利用者向けでは別には測っていない。その形を見たのは、機械の全員向けの 1 回だけ（フォルダは消えた・PATH の項目は残った）。

### 4. winget-pkgs への PR と、書いた時点の状態

- PR: https://github.com/microsoft/winget-pkgs/pull/450070 （題は `New package: Areka.Areka.Portable version 0.0.2`）。できたのは 2026-10-10 17:31:15（+09:00）、出し手は `ekicyou`。変わったファイルは `manifests/a/Areka/Areka/Portable/0.0.2/` の下の 4 つだけ（提出の記録「提出した版と日時」「PR の URL」）。
- **この節を書いた時点の状態**（2026-10-10 17:53:13〜17:53:15 +09:00 に、`gh pr view 450070 --repo microsoft/winget-pkgs` と、出来事の一覧 `gh api repos/microsoft/winget-pkgs/issues/450070/events` で読んだ。読むだけ）: 開いている（下書きではない・取り込まれていない）。付いている印は `New-Package` の 1 つだけ。**`Validation-Completed` はまだ付いていない＝自動の検査の途中**。検査は、`01. Pull Request Validation` から `07. Installers Scan` までの 7 つと `license/cla` が成功、`08. Installation Validation` が進行中、`09. Installer Metadata Validation` と `10. Validation Completed` は順番待ち。人のレビューは 0 件。
- 印の出来事は 3 つだけ: `Needs-CLA` が付いた（17:31:44）→ `New-Package` が付いた（17:34:01）→ `Needs-CLA` が外れた（17:35:21）。`Needs-Author-Feedback` と失敗の印は、1 度も付いていない。
- 完了の判定（`Validation-Completed` が付き、`Needs-CLA`・`Needs-Author-Feedback`・失敗の印が無い＝人の承認を待つ状態）は、同 spec のタスク 4.3 が、提出の記録の「印と検査の移り変わり」「完了の判定」に書く。上の 2 行は書いた時点の読みなので、その後の状態は、提出の記録と PR の頁で読む。
- **人の承認待ちになった時刻（タスク 4.3 が完了を判定した後に足した 1 行）**: 2026-10-10 19:03:29 に `Azure-Pipeline-Passed`、19:03:30 に `Validation-Completed` が付いた（+09:00。自動の検査は 10 個とも成功・`Needs-Author-Feedback` と失敗の印は 1 度も付いていない）。2026-10-10 19:14:39〜19:14:43 の読みでは、印は `Azure-Pipeline-Passed`・`Validation-Completed`・`New-Package` の 3 つで、PR は開いたまま（取り込まれていない・人のレビューは 0 件）＝**モデレーターの承認を待っている**（承認が要るという案内のコメントは 19:06:01）。判定の中身は、提出の記録（`areka-P0-winget-manifest-submission` の `verification/submission.md`）の「完了の判定」。

### 5. 直しを求める印の期限

PR に直しを求める印 `Needs-Author-Feedback` が付いたら、**10 日のうちに応える**。応えないと、PR は自動で閉じられる（提出の記録「手順」の 8）。

### 6. 消える物が在ったので: 起票した spec と、それが着地するまでの制限

- 起票した spec は **`areka-P0-user-data-root`**（利用者の物が winget の上げ直しと外し方で消えないようにする仕事。機械の全員向けで書けない件と、外した後に入れ先のフォルダが残る件も同じ spec。確かめの記録「見つかった件と起票」）。
- **`areka-P0-user-data-root` が着地するまで**（同 spec の要件 6.2）:
  - `README.md` と `dist/README.txt` に、winget の行を載せない。
  - winget-pkgs へ次の版を出さない（上げ直しのたびに、利用者の物が消えるため）。
  - 初回の 1 版（0.0.2）は、名乗り `Areka.Areka.Portable` を押さえるために、winget-pkgs に載せたままにする。
- もう 1 本の起票は `areka-P0-write-failure-notice`（書けなかったときに areka が利用者へ何も伝えない件と、入れ先に書けずに `.nar` を入れられなかったのに「ファイルが壊れてるのかもね。」と伝わる件）。MCP の `sakurascript` が中身の無い仮の受け口だった件は、今ある `areka-P0-mcp-kanade-tools` へ割り付けた（新しい起票は無し）。

### この spec を始めるときに効く事実

- winget-pkgs へ出したマニフェストは、2 つのロケール（en-US・ja-JP）に、`winget upgrade` と `winget uninstall` で `ghost`・`balloon` フォルダが後から入れた物ごと消えることを知らせる注意書き `InstallationNotes` を持つ（確かめの記録「後片付けの確かめ」の「利用者の側」の 2・4。開発機では、入れた最後の行に日本語の側が `メモ: …` として出た）。更新の PR が winget-pkgs の側の今のマニフェストを元に作られる（この brief の「Out of Boundary」の見込み。要件の段で確かめる）なら、この注意書きは、誰かが外すまで後の版へ引き継がれる。`areka-P0-user-data-root` が着地すると、この注意書きと上の 6 の制限は古くなる（同 spec の設計「Revalidation Triggers」）。
- 提出の道具 `wingetcreate` は、出すときにファイルを書き直した: 4 ファイルとも 1 行目に `# Created using wingetcreate 1.12.13.0` が足され、installer のファイルの `ReleaseDate` がファイルの最後へ動いた。欄と値は 1 つも変わっていない。このリポジトリの `dist/winget/0.0.2/` は PR の 4 ファイルで置き換えてあり、提出した中身との違いは行末だけ（提出の記録「写し戻しの見比べ」の 3〜5・7）。
- 開発者のフォーク `ekicyou/winget-pkgs` は、もう在る（提出のときに道具が作った。作成は 2026-10-10 17:31:05。提出の記録「提出した版と日時」の「道具が出した文」）。Microsoft の同意（CLA）は、2026-10-10 17:35:01 に PR へ同意の文が送られ、17:35:21 に `Needs-CLA` が外れた（GitHub の出来事の記録。上の 4 の読みでも `license/cla` は成功）。文を送ったのは開発者に任された AI で、開発者が「個人として」と答えたことは、進行役の AI から聞いた話として記録に在る（提出の記録「提出した版と日時」の「手順と違ったこと」）。同意は 1 回済ませれば Microsoft のどのリポジトリにも効く、と手順は書いている（同「手順」の 5）。
- 上の 1〜3 は、**手元のマニフェストで入れた形**での結果。入れ先のフォルダの名前（`Areka.Areka.Portable__DefaultSource`）と `winget list` の ID（`ARP\User\X64\Areka.Areka.Portable__DefaultSource`）は、winget-pkgs から入れた形（`Areka.Areka.Portable_Microsoft.Winget.Source_8wekyb3d8bbwe`）と違い、その形では測っていない。arm64 は実機で入れておらず、確かめたのは `winget validate` の成功とハッシュの一致まで。シェルの記憶は測れなかった（確かめの記録「既知の制限」）。
- 初回の提出は、開発者の指示で AI が提出のコマンドを打ち、開発者が行ったのは、ブラウザでの GitHub へのログインと道具への許可だけだった（同 spec の要件 5.2 の「開発者が自分の手で」から外れたこととして記録済み。AI はトークンを見ても扱ってもいない。提出の記録「提出した版と日時」の「手順と違ったこと」）。この spec で「誰が何をするか」を決めるときの材料になる。

### 上の本文のうち、実測と合わなくなった所

本文は変えていない。次の所は、要件の段で読み替える。

- 「Upstream」・「Constraints」の「取り込みの前に着手しない」・「棚卸㉓の測定」の「先に要るもの」: 先に要るものが、初回の提出の取り込みまでになっている。上の 6 のとおり、説明書の行と次の版の提出には、`areka-P0-user-data-root` の着地も要る。
- 「Desired Outcome」の「上げ直しのときの利用者のゴーストと記憶の扱い（実測の結果を受けて書く）」と、議題 3 の「消えると…分かったとき」: 実測は済み、消えると分かった（上の 1）。置き場を変える別の spec は `areka-P0-user-data-root`。
- 議題 2（着地するまでに出た版を、手で出すか、次の版まで待つか）: 上の 6 の制限（次の版を出さない）が先に掛かる。
- 「書き換えが要る相手」（`release-cycle` の要件・設計にある `winget.yml` の持ち主の名前）: `areka-P0-winget-manifest-submission` のタスク 5.2 が直した（直した行は 3・直さなかった行は 2。提出の記録「名前の直しの数」）。この書き換えは、もう要らない。
