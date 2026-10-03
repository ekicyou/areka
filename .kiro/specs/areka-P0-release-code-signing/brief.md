# Brief: areka-P0-release-code-signing

> 2026-10-02 `/kiro-discovery`（配布と公開＝winget・crates.io）で起票。**任意・後回し**。開発者「無料で済ませたい」への答え＝winget に署名は要らないので、まず未署名で出し、CI ができたら無料の署名を申請する。

## Problem

未署名の exe は、Windows 11 の Smart App Control を有効にしている環境では起動を止められる。ブラウザで zip を落として展開した人には「Windows によって PC が保護されました」が出る（winget で入れた人には出ない見込み＝インターネットから来た印が付かない）。署名があればどちらも和らぐが、有料の証明書（OV・年 150 ドル前後）は「無料」の条件に合わない。

## Current State

- 署名の仕組みは無い（`signtool`・`Set-AuthenticodeSignature` は 0 件）。`dist/README.txt` の既知の制限に「署名なし」。
- 調べた事実（2026-10-02）:
  - winget-pkgs は zip／portable に署名を求めない（MSIX だけ）。
  - **SignPath Foundation** はオープンソース向けに無償で署名する。条件: OSI 承認のライセンス（MIT は可）・活発に保守されリリース済み・**検証できる CI のビルドから作った成果物だけ**署名する・関係者全員が多要素認証・Author／Reviewer／Approver の役割・サイトに署名の方針を載せる。
  - Microsoft の Artifact Signing（月約 10 ドル）は**日本では個人が使えない**（個人は米国・カナダのみ。組織は法人の実体が要る）。
  - EV 証明書も 2024 年以降は SmartScreen を即座に通さない（OV と同じ扱い）。自己署名・Sigstore は Authenticode ではないので効かない。
  - どの署名でも、最初は「認識されないアプリ」の表示が出うる（評判は同じ署名者で積み重ねる）。

## Desired Outcome

1. SignPath Foundation に申請し、受理されたら `release.yml` の中で zip の中の exe（`areka.exe`・`shiori-host32-helper.exe`）に署名してから zip を作る。
2. 署名の方針（誰が承認するか・何に署名するか）がリポジトリの文書（例 `doc/SIGNING.md`）とサイト（README）に載る。
3. `dist/README.txt` の既知の制限から「署名なし」を外し、「署名: SignPath Foundation」を書く。
4. 受理されなかった場合は、理由を roadmap の「覚え書き」に残し、未署名のまま続ける（有料の証明書は開発者の決め）。

## Approach

- 前提は CI のビルド（`release-ci-workflow`）。SignPath は GitHub Actions の成果物を受け取って署名し返す Action を提供する＝`release.yml` のビルドと zip の間に段を挟む。
- 採らない案: 有料の証明書（無料の条件に合わない・必要になったら開発者が決める）／自己署名（効かない）／署名のために MSIX へ変える（MSIX は署名が必須で、無料の道が無い）。

## Scope

- **In**: SignPath Foundation への申請（開発者が行う・手順を書く）・`release.yml` への署名の段・署名の方針の文書・説明書の更新。
- **Out**: 有料の証明書・MSIX・ストア配布・SmartScreen の評判の積み上げ（時間が解く）。

## Boundary Candidates

- 申請と方針（文書）
- `release.yml` の段

## Out of Boundary

- 配布スクリプトの中身・マニフェストの形（署名が入っても変わらない）

## Upstream / Downstream

- **Upstream**: `areka-P0-release-ci-workflow`（CI のビルドが前提）・`areka-P0-release-cycle` の数回の実行（「リリース済み・活発」の実績）。
- **Downstream**: なし。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `areka-P0-winget-manifest-submission`（署名が入っても `InstallerSha256` の手順は同じ）。

## Constraints

- 無料の手段だけ。秘密（署名の鍵）はリポジトリに置かない（SignPath が鍵を預かる形）。
- 着手は開発者が望んだときだけ（任意）。

## 想定

- 規模 S（5〜8 タスク）。議題 1 件（受理されなかったときに有料へ行くか）。Opus で足りる。
