# Brief: areka-P0-release-cycle

> 2026-10-02 `/kiro-discovery`（配布と公開＝winget・crates.io）で起票。**繰り返し spec**（kiro の「リリース手順の spec は `completed/` へ移さず、実行のたびにタスクを戻す」決まり＝`.kiro/steering/workflow.md`・`kiro-complete` の例外）。

## Problem

リリースの時期は開発者が決めたい。PR のたびに版が上がる形は採らない。「この spec の実装を打ったときだけ」版が +0.0.1 され、タグが押され、GitHub Actions がビルド・Release・crates.io・winget まで運ぶ、という繰り返しの手順が要る。

## Current State

- 版は `Cargo.toml` の `[workspace.package] version` 1 か所。`/kiro-complete` は版にもタグにも触らない。main へは PR でしか入れない。
- タグ `v*` が押されたときだけ動く workflow（`release-ci-workflow`）と、その後段（`crates-io-publish`・`winget-manifest-submission`）がこの手順の受け手。
- 配布 zip の起動確認（`-Check`）は窓を出すので CI では回せない＝手元の手順に残る。

## Desired Outcome

実行 1 回＝リリース 1 回。手順（タスク）は毎回同じで、終わったら `[ ]` に戻す。

1. 前提の確認: main が最新・open PR 0 本（並走の spec が無い）・手元の全体テスト（`tools/test-all.ps1`）が緑・配布スクリプトの `-Check` が緑（x64）。1 つでも欠ければ止める。
2. 版を上げる: `cargo set-version --bump patch --workspace`（cargo-edit）か同じことをする手元の 1 行。`Cargo.lock` も更新。`dist/README.txt` の「時点」の行を今日に。`crates-io-publish` の公開前の確認（組み立てまでの形）を通す。
3. 版上げの PR を出し、squash マージする（他の変更を混ぜない）。
4. main の squash コミットに `v{版}` のタグを打って push する。これが Actions のきっかけ。
5. 見守る: `release.yml` の緑・Release の公開・crates.io の各クレートの版・winget-pkgs への PR（`winget.yml`）。赤なら原因を直す spec を起票し、**同じ版で再実行しない**（zip の URL は版ごとに固定・crates.io は差し替え不可）＝直したら次の版で出す。
6. 記録: `roadmap.md` の「完了サマリ」の下に「リリース」の 1 行（版・日付・Release の URL・winget の PR）。
7. **初回（`v0.0.2`）だけの手順**: タグを打つ前に、`wintf`・`dola` に Trusted Publishing を設定する（`crates-io-publish` の手順書。10-03 同 spec の要件討議の議題 3 で crates.io へ出すのは `wintf`・`dola` だけになり、どちらも crates.io に在るので手元からの初回の公開は要らない）。版を上げるとき、出さない `areka` ほかが持つ `wintf = { version = "0.0.1", path = ... }` の版の指定も合わせる必要があるかは `crates-io-publish` の設計で決まる。winget の初回の手提出は `winget-manifest-submission`（Release の実在が要る＝初回の後のウェーブ）。

## Approach

- タグは人が打つ（workflow が打たない）。版の正本は `Cargo.toml`、タグはそれを写す。`release.yml` は一致を検査して違えば止める。
- 繰り返し spec の形: `spec.json` の phase は `implementation-in-progress` のまま置き、`tasks.md` を毎回リセットする。`/kiro-impl areka-P0-release-cycle` で 1 回分を回す。
- 採らない案: main への push で自動リリース（PR のたびに版が上がる）／`cargo release` で main へ直接 commit・push（PR 経由の決まりに反する）／feature の PR の中で版を上げる（並走する枝が同じ行を取り合う・リリースの時期を開発者が選べない）。

## Scope

- **In**: 繰り返しの手順（tasks.md の形）・版上げの道具（`cargo set-version` か手元の 1 行）・版とタグの決まり・見守りと赤のときの決まり・記録の形・初回の実行（`v0.0.2`）。
- **Out**: workflow の中身（`release-ci-workflow` ほか）・マニフェストの形（`winget-manifest-submission`）・配布スクリプト（`release-package-versioned`）・大きい版の上げ方（0.1.0 や 1.0.0 は開発者がその場で決める＝本 spec は patch だけ）。

## Boundary Candidates

- 手順そのもの（文書＝tasks.md）
- 版上げの小さな道具（在れば）

## Out of Boundary

- CI の中の手順

## Upstream / Downstream

- **Upstream**: `areka-P0-release-package-versioned`・`areka-P0-release-ci-workflow`（初回の実行に要る）。`crates-io-publish`・`winget-manifest-submission` は後から足されても手順は変わらない（見守る相手が増えるだけ）。
- **Downstream**: 毎回のリリース。

## Existing Spec Touchpoints

- **Extends**: `kiro-complete` の繰り返し spec の例外（既にある）。
- **Adjacent**: 棚卸（`/kiro-discovery` 再入）＝リリースの記録を roadmap に足す場所を共有。

## Constraints

- 触るのは本 spec の `tasks.md`・`Cargo.toml`（版の 1 行）・`Cargo.lock`・`dist/README.txt`（時点）・`roadmap.md`（記録 1 行）。コードには触らない。
- 秘密を印字しない（`git remote -v` を手順に入れない）。
- 1 回の実行で上げる版は patch（+0.0.1）だけ。

## 想定

- 規模 XS（手順 6 本・毎回同じ）。議題 1 件（版上げを cargo-edit に頼るか、`Cargo.toml` の 1 行を書き換える手元の小さなスクリプトにするか＝道具を足さない方を推す）。Opus で足りる。
