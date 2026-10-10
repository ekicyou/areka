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
