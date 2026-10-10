# Brief: areka-P0-winget-manifest-submission

> 2026-10-02 `/kiro-discovery`（配布と公開＝winget・crates.io）で起票。テーマの決めごとは `.kiro/steering/roadmap.md`「配布と公開」節。調べた事実の出典は同節。

## Problem

利用者が `winget install Areka.Areka.Portable` で areka（ポータブル版＝zip をそのまま置く形）を入れられるようにしたい。winget のコミュニティの置き場（microsoft/winget-pkgs）にマニフェストを出し、以後のリリースでは自動で更新の PR が出る形にする。無料・未署名で。

## Current State

- マニフェストは無い。GitHub Releases も無い（`release-ci-workflow` が作る）。
- 決めたこと（開発者・2026-10-02）: **名乗りは `Areka.Areka.Portable`**（zip 版＝「areka ポータブル」。表示名は「areka (portable)」）。**`Areka.Areka` は後で作るインストーラー版のために空けておく**（入れ物の種類ごとに別の名乗りにするのが通例＝portable で入れた人の環境へ次の版でインストーラーを被せる乗り換えを winget は綺麗に扱えない）。発行者は「areka プロジェクト」＝プロジェクト自身。素性は `Publisher: ekicyou`・`PublisherUrl` で示す。**入れ物は zip のまま（`InstallerType: zip`・`NestedInstallerType: portable`）**。**x64 と arm64 を最初から並べる**。
- 調べた事実: zip＋portable に署名の義務は無い（MSIX だけ要る）。portable は**スタートメニューのショートカットを作らない**。PATH へ出すリンク経由の起動で隣の DLL を見失う既知の問題があり、`ArchiveBinariesDependOnPath: true` で避けられる（リンクを作らずインストール先を PATH に足す）。インストール先は利用者単位で `%LOCALAPPDATA%\Microsoft\WinGet\Packages\<Id>_…\`。アンインストールで消えるのは winget が置いたファイルだけ（利用者のゴーストは残る）。初回の提出は手で（1 PR に 1 版・自動検査の後に人が承認・実例で 2 日）。更新は `winget-releaser`（GitHub Action・中身は Komac）が Release の公開をきっかけに PR を出す。前提は「既に 1 版が在ること」「開発者のアカウントに winget-pkgs のフォークが在ること」。`installers-regex` の既定は zip を含まないので `'\.zip$'` を指定する。
- `InstallerUrl` は版ごとに固定の https（GitHub Releases は可）。`release-package-versioned` が `areka-{版}-{arch}.zip` と `.sha256` を出す。

## Desired Outcome

1. マニフェスト 3 ファイル（version・defaultLocale・installer・ManifestVersion 1.12.0）の形が決まり、リポジトリの中（例 `dist/winget/`）に雛形として置かれる。x64 と arm64 の 2 項目・`ArchiveBinariesDependOnPath: true`・`NestedInstallerFiles` に `areka.exe`（`PortableCommandAlias: areka`）・License MIT・`ReleaseDate`・Tags（`ukagaka`・`desktop-mascot` など）。
2. 手元で `winget install --manifest dist/winget/<版>/` を通し、`areka` と打って起動し、ゴーストが出ること。アンインストールで利用者のゴーストが残ること。これを実機の確認とする。
   - 2026-10-03 の実測（`release-package-versioned` の実機の確かめ・`verification/winget-local-check.md`）: `--manifest` は管理者の `winget settings --enable LocalManifestFiles` が要る（終わったら戻す）。手元のマニフェストで入れた物は ID が `ARP\User\X64\<Id>__DefaultSource` になり、`winget uninstall --id <Id>` では見つからない（`winget list` で引いた ID に `--exact --purge`）。リンクを作る形（`ArchiveBinariesDependOnPath` なし）を確かめるときは、開発者モードがオフだと winget がリンクの代わりに PATH を足す。
3. 初回の提出: 開発者が `komac new` か `wingetcreate new` で `v0.0.2`（最初の Release）を提出する手順が書かれ、実行される。自動検査（ウイルス対策・無人のインストールとアンインストール）で何か言われたら直す。
4. 以後: `.github/workflows/winget.yml`（`workflow_run` で動く＝`release.yml` がタグの push で緑に終わったときだけ。`GITHUB_TOKEN` が作った Release では `release: published` が飛ばないため・10-03 `release-ci-workflow` の裁定。中身は `winget-releaser`・`installers-regex: '\.zip$'`・`max-versions-to-keep` を決める）が PR を出す。要るトークン（classic PAT・`public_repo` と `workflow`）は開発者が secret に置く＝リポジトリには書かない。
5. `dist/README.txt` の「入れ方」に winget の 1 行と、「スタートメニューにアイコンは出ない（コマンド名 `areka` か、インストール先のフォルダから）」「Smart App Control を有効にしている環境では未署名の exe が止まる」の既知の制限が入る。

## Approach

- マニフェストは Komac か wingetcreate に作らせ、手で `ArchiveBinariesDependOnPath` とロケールの欄を整える。雛形をリポジトリに置くのは、提出する内容を PR で見られるようにするため（提出そのものは winget-pkgs 側）。
- 採らない案: Inno などの本物のインストーラー（アイコンが欲しくなったら後から別 spec `Areka.Areka` として足す＝開発者の決め）／`ekicyou.areka` の名乗り（開発者が却下）／zip 版とインストーラー版を同じ名乗りにする（乗り換えが混ざる）／リンク経由の起動に頼る（既知の問題）。

## Scope

- **In**: マニフェストの雛形・手元の `--manifest` での実機確認・初回の手提出の手順と実行・`winget.yml`・説明書の追記。
- **Out**: zip を作ること（`release-package-versioned`）・Release を作ること（`release-ci-workflow`）・署名（`release-code-signing`）・インストーラー・ショートカットの作成・winget 以外の置き場（Scoop・Chocolatey）。

## Boundary Candidates

- マニフェストの形（静的）
- 提出の自動化（`winget.yml`）

## Out of Boundary

- `release.yml`（Release を作る側）

## Upstream / Downstream

- **Upstream**: `areka-P0-release-package-versioned`・`areka-P0-release-ci-workflow`・`areka-P0-release-cycle` の初回（`v0.0.2` の Release が実在すること＝初回の提出に要る）。
- **Downstream**: `areka-P0-release-code-signing`（署名が入っても形は変わらない）・将来のインストーラー spec（名乗り `Areka.Areka`・データは `%APPDATA%` へ＝ポータブル版と住み分ける）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `areka-P0-crates-io-publish`（`dist/README.txt`・`README.md` に crates.io の 1 行を先に足す＝本 spec の前のウェーブ）。

## Constraints

- 触るのは `dist/winget/**`（新規）・`.github/workflows/winget.yml`（新規）・`dist/README.txt`・`README.md`。`crates/`・`tools/` には触らない。
- 秘密をリポジトリに置かない。
- 一度出した名乗り `Areka.Areka.Portable` は変えられない。`Areka.Areka` を zip 版に使わない。

## 想定

- 規模 S（6〜9 タスク）。議題 2 件（`max-versions-to-keep` の数／Tags の語）。Opus で足りる。初回の提出の待ち（人の承認）は日単位で、spec の完了はマージを待たず「PR を出して検査が通った」まで。

## 2026-10-03 ウェーブ C3-①（予定・10-03 の組み直し（開発者「1 バグ・2 リリース関係・バルーン関係・アニメーション画像関係・3 その他」））

- 段は「優先」。C3 は C2 の着地で brief が動くので、着手の前に同じウェーブの他の spec と触るファイルを照合し直す（`roadmap.md`「ウェーブ編成」の C3 の行）。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: S（7〜10 タスク・下の議題 2 で上げ直しの実測を足すと +1〜2）。切る: なし。
- 前提の状態: `release-package-versioned`・`release-ci-workflow`・`crates-io-publish` は着地。**`release-cycle` の初回（`v0.0.2`）はまだ**＝Release が無いので初回の手提出（Desired 3）はできない。マニフェストの雛形・手元の `--manifest` の確かめ・`winget.yml` は Release が無くても作れる（手元の確かめは `release-package-versioned` の `verification/winget-local-check.md` と同じく、手元の http サーバーから zip を配る形で通る）。
- 崩れた前提／古くなった位置:
  - **リンク経由の起動の問題は areka 側で解いた**。`release-package-versioned`（PR#218）が `crates/areka/src/boot_config.rs` に、起動した exe のリンクを解く `follow_exe_links` と、それを 1 度だけ求めて覚える `exe_location` を足し、`resolve_root`・`default_helper_exe_path`・`default_app_profile_dir` がそれを通る。実機で `winget install --manifest` → `areka` の 1 語で起動 → リンク経由で根を引けたことを確かめ済み。brief の「`ArchiveBinariesDependOnPath: true` で避ける」は、もう唯一の手ではない（下の議題 1）。
  - 同 spec の設計の範囲外に「portable を外す・上げるときに、入れ先の中の記憶（`profile\areka`・ゴーストの `profile`）が消えるかの利用者向けの扱いは `winget-manifest-submission` へ申し送り」とある（`completed/areka-P0-release-package-versioned/design.md` の「Out of Boundary」）。brief は外すとき（アンインストール）しか書いていない。areka の根は exe の隣なので、利用者が入れたゴースト・記憶・`.nar-work` もすべて winget の入れ先の中に住む（下の議題 2）。
  - 手元の確かめの手順の罠は `release-package-versioned` の `verification/winget-local-check.md` が正本（`LocalManifestFiles` は管理者で入れて戻す・ID は `ARP\User\X64\<Id>__DefaultSource` になり `winget list` で引いて `--exact --purge`・開発者モードがオフだとリンクの代わりに PATH が足される）。
  - `release.yml` は `workflow_dispatch` でも動く（main での乾いた走り）。`winget.yml` を `workflow_run` で受けるなら、元の走りが `push` で、タグ（`head_branch` が `v*`）で、緑のときだけ進む絞りが要る（乾いた走りで PR を出さない）。`winget-releaser` に渡す版も `workflow_run` の `head_branch` から取る。
  - 書き足す場所: `dist/README.txt` の「■ 入手のしかた」は箇条書き 1 行（zip）だけで、winget の行を同じ箇条書きに足せる形になっている（`crates-io-publish` が用意）。「■ 既知の制限」の先頭に「areka.exe には署名がありません」が既に在る。`README.md` は PR#222 で書き直され、「## 入手と起動」の箇条書きが 1 行（「まだ GitHub Releases での配布はしていない」）。
- 触るファイル（並走の照合用）:
  - `dist/winget/**`（新規・3 ファイル×版の雛形）
  - `.github/workflows/winget.yml`（新規）
  - `dist/README.txt`（「■ 入手のしかた」に 1 行・「■ 既知の制限」に 2 行）
  - `README.md`（「## 入手と起動」の箇条書き）
  - 同じウェーブ C3 の他の 8 本はどれもこの 4 つに触らない。後で `dist/README.txt` を触る `install-live-target-hazards`・`mcp-stdio-bridge`（どちらも C4 の候補）・`release-code-signing` とは順に並ぶ。
- 議題（答えで作業が変わるものだけ）:
  1. **`ArchiveBinariesDependOnPath` を付けるか**。⒜ 付ける（brief どおり）＝リンクを作らず入れ先を PATH に足す。areka の外の都合（DLL を探す順など）にも強いが、PATH に areka のフォルダが丸ごと載る ⒝ 付けない＝winget の既定（リンクを作る。ただし管理者でない利用者は開発者モードがオンのときだけリンクになり、オフなら winget が入れ先を PATH に足す＝実測）。areka は今リンクを解けるのでどちらでも動く。どちらにするかでマニフェストの欄と手元の確かめの見るもの（`(Get-Command areka).Source` がリンクかどうか）が変わる。
  2. **上げ直し（`winget upgrade`）で利用者のゴースト・記憶が残るか**。portable の上げ直しは古い版を外してから新しい版を入れる。areka は利用者のゴースト（`ghost\`）・areka の記憶（`profile\areka\`）・ゴーストの記憶を入れ先の中に置く。winget が「自分が置いたファイルだけ」を消すなら残るが、同梱の `ghost\emo2` は新しい版で上書きされる。実測していない。⒜ 本 spec で手元の 2 版のマニフェスト（例 0.0.2 → 0.0.3 の名乗りで同じ zip）を使って上げ直しを 1 回確かめ、結果を説明書に書く ⒝ 確かめずに既知の制限として「上げ直しの前に ghost と profile を写しておく」と書く。消えると分かったら置き場を変える別の spec が要る（インストーラー版の `%APPDATA%` と同じ話）。
  3. （既存）`max-versions-to-keep` の数・Tags の語。
- 見つけた穴: なし（議題 2 は実測していないので穴とは書かない）。


## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: S（7〜10 タスク。上げ直しの実測を足すと +1〜2）。切る: なし。
- 前提の状態: **待ち＝`release-cycle` の初回（`v0.0.2` の Release）**。Release の実在が要るのは確かに初回の手提出（Desired 3）だけ。`InstallerUrl` は Release の zip の URL、`InstallerSha256` はその `.sha256` の値で、どちらも Release が無いと書けない。雛形と手元の `--manifest` の確かめは Release なしで作れる（`release-package-versioned` の `verification/winget-local-check.md` は手元の http から zip を配った）。タグは今 `v0.0.1` だけ。
- 新しく分かった順の制約:
  - `winget.yml`（`release` の緑を受けて `winget-releaser` を回す）は、winget-pkgs に既に在る名乗りの更新しかできない（brief の「前提は既に 1 版が在ること」）。初回の提出が人の承認で取り込まれるまで（実例で 2 日）に次のタグを打つと、その走りの `winget.yml` が赤になる。
  - 本 spec の PR（`winget.yml` を含む）が main に入ってから、winget-pkgs への初回の提出が取り込まれるまでの間は、リリースを打たない。または `winget.yml` だけを取り込みの後の別の PR に分ける。
- 崩れた前提／古くなった位置:
  - C3 の 11 本は `.github/`・`README.md` に触らなかった。`dist/README.txt` には `animated-image-decode` が「■ 動く絵の上限」の節を「■ 入手のしかた」と「■ 既知の制限」の間に足したが、本 spec が足す 2 つの節は変わっていない（「■ 入手のしかた」は箇条書きの zip の 1 行・「■ 既知の制限」の先頭は「areka.exe には署名がありません」）。
  - `README.md` の「## 入手と起動」の最初の箇条は今も「まだ GitHub Releases での配布はしていないので…」。`release-cycle` がこの行を直さなければ本 spec が直す。
  - zip の中身（`tools/package.ps1` の一覧）は変わっていない＝`areka.exe` は zip の根。`NestedInstallerFiles` の位置もそのまま。
- 触るファイル（並走の照合用）:
  - `dist/winget/**`（新規）
  - `.github/workflows/winget.yml`（新規）
  - `dist/README.txt`（「■ 入手のしかた」に 1 行・「■ 既知の制限」に 2 行）
  - `README.md`（「## 入手と起動」の箇条）
  - リポジトリの外: 開発者が secret に置く classic PAT（`public_repo`・`workflow`）と、開発者のアカウントの winget-pkgs のフォーク
- 共有しうる相手:
  - `release-cycle`（`README.md` の同じ行・`dist/README.txt` の冒頭）＝直列。
  - `mcp-stdio-bridge`（`dist/README.txt`・`NestedInstallerFiles` に中継を入れるか）・`install-live-target-hazards`・`release-code-signing`（「■ 既知の制限」）。
  - ほかに `dist/README.txt` の別の節を触る見込みの brief は `animated-image-playback`・`extra-character-windows`・`property-name-case-fold`・`self-alpha-declaration`・`update-check-options`。
- 議題（答えで作業が変わるものだけ）: 棚卸㉑の 3 つのまま（`ArchiveBinariesDependOnPath` を付けるか・上げ直しで記憶が残るか・`max-versions-to-keep` と Tags）。加えて、`winget.yml` を同じ PR に入れるか、初回の提出の取り込みの後の PR に分けるか（上の順の制約）。
- 見つけた穴: 上の順の制約が brief に書かれていない（Desired 4 は `winget.yml` を同じ spec の中で足す前提）。


## 2026-10-10 棚卸㉓の再測定（main `ee3af616`・C4 の着地の後）

- 前提の変化: **待っていた前提が満たされた。** `v0.0.2` が 2026-10-06 に公開された（https://github.com/ekicyou/areka/releases/tag/v0.0.2 ）。
  - 置いてある物は `areka-0.0.2-x64.zip`・`areka-0.0.2-arm64.zip` と、それぞれの `.sha256` の 4 つ＝マニフェストの `InstallerUrl`・`InstallerSha256` が書ける。
  - `README.md` の「## 入手と起動」は、もう「GitHub Releases から配布物の zip を入手して展開する」に直っている（本 spec は winget の 1 行を足すだけ）。
  - `dist/README.txt` の「■ 入手のしかた」は zip の 1 行のまま・「■ 既知の制限」の先頭は「署名がありません」のまま。
  - `.github/workflows/` は `release.yml`・`crates-io.yml` の 2 本。`release.yml` は今も、タグ `v*` の push と手での起動の両方で動く。`dist/winget/` はまだ無い。
  - `release-cycle` の手順は、タグのコミットに `winget.yml` が在るときだけ winget-pkgs への PR を見守る作りになっている＝`winget.yml` が後から入っても、手順を直さずに済む。
- 触るファイル: `dist/winget/**`（新規）・`.github/workflows/winget.yml`（新規）・`dist/README.txt`（「入手のしかた」に 1 行・「既知の制限」に 2 行）・`README.md`（1 行）。`crates/`・`tools/` には触らない＝ほかのどの spec ともソースの重なり 0。
- 規模: 7〜10 タスク（分けるなら 5〜7 と 3〜4）。
- 先に要るもの: なし（今すぐ着手できる）。開発者の手が要るもの＝自分のアカウントの winget-pkgs のフォーク・初回の提出（`komac` か `wingetcreate`）・`winget.yml` が使うトークンの登録。
- 優先度の区分: A（開発者「winget 対応インストーラーを作成したい」）。
- 要件定義のモデル: Opus。
- 分割の案: **2 本に分けるのを推す**（規模のためではなく、人の承認待ち約 2 日を 1 本の PR の中に抱えないため）。
  - 前の 1 本（この spec のまま）: マニフェストの雛形 `dist/winget/**`・手元の `--manifest` での確かめ（上げ直しで利用者のゴーストが残るかも測る）・初回の手提出。「PR を出して自動の検査が通った」で完了する。説明書には触らない。
  - 後の 1 本（新しく起票・例 `winget-release-automation`・3〜4 タスク）: `.github/workflows/winget.yml`・`dist/README.txt` と `README.md` の winget の行。**初回の提出が winget-pkgs に取り込まれてから**着手する（取り込みの前に `winget install` の行を説明書に載せない。取り込みの前にタグを打っても `winget.yml` が赤にならない）。
- 見つけた穴・古くなった記述:
  - 本文「Current State」の「GitHub Releases も無い」と、棚卸㉒の「`README.md` の最初の箇条は今も『まだ配布していない』」は古い。
  - 分けない場合は、本 spec の PR が main に入ってから取り込みまでの間、`release-cycle` を回せない。


## 2026-10-10 棚卸㉓の分割

- **分けた理由**: 規模（7〜10 タスク）のためではない。winget-pkgs への初回の提出は、人の承認を約 2 日待つ。その待ちを 1 本の PR の中に抱えないために 2 本に分ける。あわせて、`winget.yml` が取り込みの前に main に入ると、その間 `release-cycle` を回せなくなる（同 spec は、タグのコミットに `winget.yml` が在れば winget-pkgs への PR を見守り、そこが赤なら直す spec を起票する決まり）ので、`winget.yml` を後ろの 1 本へ出す。
- **残す範囲（In）**:
  - マニフェストの雛形 `dist/winget/**`（新規・3 ファイル。本文の Desired Outcome の 1）。
  - 手元の `winget install --manifest` での確かめ（同 2）。**上げ直し（`winget upgrade`）で利用者のゴースト・記憶が残るかも、ここで測る**（棚卸㉑の議題 2 の ⒜ を採った形）。
  - 初回の手提出（同 3）。
  - **完了の線**: winget-pkgs への PR を出し、その自動の検査が緑になった所まで。取り込み（人の承認）は待たない。
- **出した範囲（Out・どの spec へ）**: → `areka-P0-winget-release-automation`（新しく起票）
  - `.github/workflows/winget.yml`（本文の Desired Outcome の 4）。
  - `README.md` と `dist/README.txt` の winget の行（同 5。「入手のしかた」の 1 行と「既知の制限」の 2 行）。取り込みの前に `winget install` の行を説明書に載せないため。
  - `max-versions-to-keep` の数（議題の一部）と、トークン（PAT）の登録も、そちらへ移る。
- **順番**: この spec → winget-pkgs が初回の提出を取り込む（人の承認・実例で約 2 日）→ `winget-release-automation`。`release-cycle` は、この並びのどこでも回せる（`winget.yml` が main に無い間は winget を見守らない）。
- **残した側の規模**: 5〜7 タスク。
- **残した側が触るファイル**:
  - `dist/winget/**`（新規）
  - この spec のフォルダの中の確かめの記録
  - リポジトリの外（開発者の手が要るもの）: 開発者のアカウントの winget-pkgs のフォーク・初回の提出（`komac` か `wingetcreate`）。
- **同じウェーブで触らない約束**（破るなら止めて報告）:
  - `README.md` に触らない。`dist/README.txt` に触らない。`.github/workflows/` に触らない。
  - `crates/`・`tools/` に触らない（本文の Constraints のとおり）。
  - 本文の Constraints の「触るのは `dist/winget/**`・`.github/workflows/winget.yml`・`dist/README.txt`・`README.md`」は、この分割で「`dist/winget/**` だけ」に読み替える。
  - ほかのどの spec ともファイルの重なりは 0。
- **議題**（この分割の後に残るもの）:
  1. `ArchiveBinariesDependOnPath` を付けるか（棚卸㉑の議題 1 のまま）。
  2. Tags の語。
  3. 提出する版。winget-pkgs は 1 つの PR に 1 つの版。今 Release が在るのは `v0.0.2` だけだが、着手までに次の版が出ていたら、どの版で初回を出すかを着手のときに決める。
- **後ろの spec への申し送り**: 上げ直しの実測の結果（利用者のゴーストと記憶が残ったか・同梱のゴーストが上書きされたか）を、この spec の完了のときに `winget-release-automation` の brief へ書き足す（説明書の「既知の制限」の文の材料になる）。消えると分かったら、説明書に書くのは後ろの spec、置き場を変える仕事は別の spec として起票する。
- **書き換えが要る相手（この brief では触っていない）**: `release-cycle` の要件・設計は、`winget.yml` の持ち主を `winget-manifest-submission` と書いている。持ち主の名前が `winget-release-automation` に替わるだけで、手順（タグのコミットにファイルが在るかで見る）は変わらない。
