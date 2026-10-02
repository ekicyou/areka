# Brief: areka-P0-release-ci-workflow

> 2026-10-02 `/kiro-discovery`（配布と公開＝winget・crates.io）で起票。テーマの決めごとは `.kiro/steering/roadmap.md`「配布と公開」節。file:line は起票時値（main `76e17654`）。

## Problem

リリースのたびに、x64 と arm64 の zip を作り、SHA256 を添えて GitHub Release を作る作業を、人の手でなく機械に任せたい。ただし**きっかけは開発者が決める**＝普通の PR のマージでは何も起きず、版を上げるコミットに付けたタグ（`v0.0.2` の形）が押されたときだけ動く。無料で済ませる（GitHub Actions の公開リポジトリ向けの無料枠・Windows ランナー）。

## Current State

- `.github/` は無い。GitHub Releases は 0 件。`.kiro/steering/tech.md` は「外部 CI は持たない（GUI・WUC・GPU のテストがホストのランナーで再現できない）」と書く＝**テストの門は手元のまま**で、ビルドと配布だけを CI に乗せる。この区別を `tech.md` に書き足す。
- zip を作るのは `tools/package-alpha.ps1`（`release-package-versioned` が版入りの名前・SHA256・arm64・CI 向けの引数を足す）。
- タグは `v0.0.1` が 1 つ（完了 `crate-name-reservation` が crates.io の名前を押さえたとき）。
- 署名の仕組みは無い（未署名で出す＝テーマの決めごと）。

## Desired Outcome

1. `.github/workflows/release.yml`: `push: tags: ['v*']` で動く。Windows ランナーで、タグの指すコミットを取り出し、`Cargo.toml` の版とタグの版が一致することを確かめ（違えば止める）、配布スクリプトで x64 と arm64 の zip と `.sha256` を作り、`gh release create v{版} --generate-notes` に添えて Release を公開する。
2. 失敗したら Release を作らない（zip が 1 つでも作れなければ止める）。途中で止まった跡（下書きの Release）を残さない。
3. 普通の PR・main への push では動かない。手で `workflow_dispatch` から同じ手順を乾いた走り（Release を作らない）で試せる。
4. 配布スクリプトの `-Check`（窓を出す自己検査）は CI では省く。その代わり、開発者が手元で `-Check` を通してから版を上げる、を `release-cycle` の手順に置く。
5. 後段の口を用意する: Release が公開された後に crates.io へ出す段（`crates-io-publish`）と、winget へ PR を出す段（`winget-manifest-submission`）が、この workflow の後ろに足せる形（job の分け方・`needs:`）。本 spec では空のまま。
6. `tech.md` の「外部 CI は持たない」を「テストの門は手元・ビルドと配布は Actions」に改める。

## Approach

- ランナーは `windows-latest`。Rust は `rustup` で安定版・`x86_64-pc-windows-msvc`・`i686-pc-windows-msvc`・`aarch64-pc-windows-msvc` の 3 target。arm64 のリンクに要る MSVC の arm64 の道具がランナーに在るかは**最初のタスクで実測**し、無ければ VS の部品を足す段を入れる。
- `cargo about`・`cargo deny` は配布スクリプトが要る＝`cargo install` を毎回走らせず、キャッシュ（`Swatinem/rust-cache` か `actions/cache`）を使う。
- 権限は `contents: write` だけ（`GITHUB_TOKEN`・長生きするトークンを置かない）。
- 採らない案: main への push で動かす（PR のたびに版が上がる・開発者の意図と違う）／タグを workflow が打つ（タグは `release-cycle` が開発者の手で打つ＝きっかけは人）。

## Scope

- **In**: `release.yml`・版とタグの一致の検査・x64／arm64 の zip と SHA256・Release の作成・乾いた走り・後段の口・`tech.md` の改め・初回の実走（`v0.0.2`）。
- **Out**: テストの実行（門は手元）／crates.io への公開（`crates-io-publish`）／winget への提出（`winget-manifest-submission`）／署名（`release-code-signing`）／版を上げる手順（`release-cycle`）。

## Boundary Candidates

- ビルドと zip（配布スクリプトの呼び出し）
- Release の作成（`gh`）

## Out of Boundary

- 配布スクリプトの中身（`release-package-versioned`）

## Upstream / Downstream

- **Upstream**: `areka-P0-release-package-versioned`。
- **Downstream**: `areka-P0-release-cycle`（タグを打つ）・`areka-P0-crates-io-publish`・`areka-P0-winget-manifest-submission`・`areka-P0-release-code-signing`。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `tools/test-all.ps1`（触らない・テストは手元）。

## Constraints

- 触るのは `.github/workflows/release.yml`・`.kiro/steering/tech.md`・`structure.md`（`.github/` の説明）。`crates/`・`tools/` には触らない（スクリプトの直しが要れば `release-package-versioned` へ戻す）。
- 秘密（トークン）をリポジトリにも workflow のログにも出さない。`git remote -v` の印字を手順に入れない。
- 無料枠の中で済ます（公開リポジトリ・1 回のリリースで Windows ランナー 1 本・30 分程度の見込み）。

## 想定

- 規模 S（6〜9 タスク）。議題 1 件（arm64 のリンクの道具がランナーに在るか＝実測で決まる）。Opus で足りる。
