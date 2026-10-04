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
