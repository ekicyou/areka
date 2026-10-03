# Brief: areka-P0-crates-io-publish

> 2026-10-02 `/kiro-discovery`（配布と公開＝winget・crates.io）で起票。開発者「可能なら crates.io へのリリースも組み込んでほしい」。file:line は起票時値（main `76e17654`）。

## Problem

リリースのたびに、areka のクレート群を同じ版で crates.io へ出したい。今は `areka`・`dola`・`wintf` の 3 つが 0.0.1 で名前を押さえてあるだけで、`areka` が使う部品のクレートは `publish = false` のまま。crates.io は手元のパスだけの依存を受け付けないので、**`areka` を新しい版で出すには部品もすべて出ている必要がある**。

## Current State

- `Cargo.toml` の `[workspace.package]` に `publish = false`・`license = "MIT"`・`repository`・`edition = "2024"`。クレートごとの `publish` は `areka`・`dola`・`wintf` が `true`、残り 26 が `false`。
- crates.io に在るのは `areka`・`dola`・`wintf` の 0.0.1（所有者は開発者・完了 `crate-name-reservation`）。
- 2026-10-02 に名前を確かめた: `areka-actor`・`areka-emo-atlas`・`areka-emo-compose`・`areka-emo-present`・`areka-emo-text`・`areka-ghost`・`areka-kanade`・`areka-nar`・`areka-parsers`・`areka-sakura`・`areka-seriko`・`areka-sylphya`・`areka-talk`・`areka-update`・`shiori-abi`・`shiori-host32-helper`・`shiori-host32-host`・`shiori-host32-ipc`・`log-capture-kit`・`temp-path-kit` は**すべて空いている**。
- `description`・`readme`・`keywords`・`categories` の欄は多くのクレートに無い（起票時は未確認＝最初のタスクで数える）。
- 版は workspace で 1 つ（`release-package-versioned`・`release-cycle` の決めごと）。cargo は `cargo publish --workspace` で依存の順番に出せる（手元の rustc 1.99）。
- 認証は crates.io の **Trusted Publishing**（GitHub Actions から、長生きするトークン無しで出す仕組み）が使える。ただし**クレートが crates.io に無いと設定できない**＝新しいクレートの初回は手元でトークンを使って出す。

## Desired Outcome

1. `areka` が依存する部品のクレートに `publish = true` と `description`（と要るなら `readme`）が付き、`cargo publish --workspace --dry-run` が手元で通る。
2. 出さないクレートは明示的に `publish = false` のまま: 試験用の DLL（`shiori-host32-testdll`・`shiori-host32-testdll-loadu`・`shiori4-testdll`）・`pilot`・`sample-ghost-kit`・`ukadoc-survey`。理由をそれぞれの `Cargo.toml` のコメントに 1 行。
3. 初回: **`release-cycle` の初回（`v0.0.2`）の中で**、開発者が手元で `cargo publish --workspace` を実行して全部を同じ版で出す（新しいクレートは crates.io に無いと Trusted Publishing を設定できない。`areka`・`dola`・`wintf` の 0.0.1 は既に在るので、初回は 0.0.2 で出す）。その後、各クレートに Trusted Publishing（リポジトリ `ekicyou/areka`・workflow `crates-io.yml`）を設定する。本 spec の完了は「`--dry-run` が緑・workflow と手順が揃った」まで。
4. 以後: 自分の workflow `.github/workflows/crates-io.yml`（`release: published` で動く）が、Release の公開の後に `cargo publish --workspace` を Trusted Publishing で実行する。`release.yml`（`release-ci-workflow`）には触らない＝同じウェーブ C2 で並走できる。失敗しても Release と winget の段は巻き戻さない（crates.io は一度出した版を差し替えられない＝**出す前の `--dry-run` を手元の手順と CI の両方に置く**）。
5. `dist/README.txt` と根の `README.md` に「crates.io は部品と本体の公開・名前の確保のため。`cargo install areka` では 32bit の補助 exe が付かないので、利用者は winget か zip で入れる」と書く。
6. `tools/test-all.ps1` に `--dry-run` を足すかは議題（毎回 1〜2 分増える見込み）。

## Approach

- `publish` と `description` は各クレートの `Cargo.toml` に書く（workspace の `publish = false` は外し、出さないクレートに `publish = false` を明示）。
- 出す順番は cargo に任せる（`--workspace`）。手で順番を書かない。
- 採らない案: `areka` だけを出すために部品を 1 つの大きなクレートに畳む（設計の分け方を壊す）／git の依存で出す（crates.io が受け付けない）／長生きする API トークンを Actions の secret に置く（Trusted Publishing で要らない）。

## Scope

- **In**: `publish`・`description` ほかの欄の整備・出さないクレートの明示・`--dry-run` の手順・初回の手元からの公開の手順（実行は `release-cycle` の初回）・Trusted Publishing の設定の手順（開発者が crates.io の画面で行う）・`.github/workflows/crates-io.yml`・README の説明（`dist/README.txt`・`README.md` に「crates.io は部品の公開・利用者は winget か zip」の 1 行）。
- **Out**: 版の決め方（workspace で 1 つ・`release-cycle`）／crates.io 以外の置き場／`cargo install` で動く形にすること（補助 exe の同梱は無理）／docs.rs の見た目の整備（出た後に必要なら）。

## Boundary Candidates

- クレートの欄の整備（`Cargo.toml` 群）
- 公開の段（`crates-io.yml`）

## Out of Boundary

- `release.yml`（ビルド・zip・Release＝`release-ci-workflow`）

## Upstream / Downstream

- **Upstream**: 完了 `crate-name-reservation`・`areka-P0-mcp-server-core`（C1・新しいクレート `areka-mcp` の `publish` を決める＝その後）。`release-ci-workflow` とは同じウェーブ C2（ファイルを共有しない・つながりは「Release の公開」というきっかけだけ）。
- **Downstream**: `areka-P0-release-cycle` の初回（手元からの初回の公開）・`areka-P0-winget-manifest-submission`（README の同じ節に winget の行を足す）。

## Existing Spec Touchpoints

- **Extends**: 完了 `crate-name-reservation`（3 クレートの名前の確保を、部品まで広げる）。
- **Adjacent**: 依存を足す spec（`mcp-server-core`・`animated-image-decode`・`mcp-stdio-bridge`）は `Cargo.toml` を触る＝同じウェーブに置かない。

## Constraints

- 触るのは各 `Cargo.toml`（約 23 本・欄だけ＝依存は変えない）・`.github/workflows/crates-io.yml`（新規）・`dist/README.txt`・`README.md`・`.kiro/steering/tech.md`。`crates/*/src/`・`.github/workflows/release.yml`・`Cargo.lock` には触らない（欄の変更では `Cargo.lock` は動かない＝動いたら止めて報告）。**同じウェーブ C2 で `Cargo.toml` を触る spec を置かない**。
- 一度出した版は消せない。手元の `--dry-run` と全体テストの緑を、出す前の必達にする。
- 秘密をリポジトリに置かない（Trusted Publishing・初回のトークンは手元の `cargo login` だけ）。

## 想定

- 規模 S〜M（8〜12 タスク）。議題 2 件（`--dry-run` を全体テストに入れるか／`shiori-host32-helper` を出すか＝bin だけのクレートで `areka` の依存ではない）。Opus で足りる。
