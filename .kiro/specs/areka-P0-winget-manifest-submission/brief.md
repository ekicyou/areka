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
4. 以後: `.github/workflows/winget.yml`（中身は `winget-releaser`・`installers-regex: '\.zip$'`・`max-versions-to-keep` を決める）が PR を出す。`release.yml` は後段を呼ばないので、`winget.yml` は自分できっかけを受ける。候補は、`crates-io.yml` と同じ「タグ `v*` の push を受け、同じタグの `release.yml` の回が成功で終わるのを待つ」形か、`workflow_run`（`release` の完了）で動く形（winget は crates.io の Trusted Publishing を使わないので `workflow_run` も使える）で、どちらにするかは本 spec で決める（10-03 `crates-io-publish` の完了時の開発者の裁定＝案 B）。`GITHUB_TOKEN` で作った Release は `release: published` を起こさないので、そのきっかけは使えない。要るトークン（classic PAT・`public_repo` と `workflow`）は開発者が secret に置く＝リポジトリには書かない。
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
