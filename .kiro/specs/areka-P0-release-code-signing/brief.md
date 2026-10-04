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


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: S（6〜9 タスク）。切る: なし。段は据え置き（任意）のまま。
- 前提の状態: CI のビルド（`release-ci-workflow`）は着地した。「リリース済み・活発」の実績（`release-cycle` の数回の実行）はまだ 0 回＝SignPath Foundation への申請の条件が満たせない。着手は早くても `release-cycle` の数回の後。
- 崩れた前提／古くなった位置:
  - **「ビルドと zip の間に段を挟む」は `release.yml` だけでは書けない**。`release.yml` の段「zip を作る」は `tools/package.ps1 -Arch all` を 1 回呼び、その中で組み立て・同梱物の配置・zip・SHA256 までを一続きに行う。exe に署名してから zip を作るには、`tools/package.ps1` に「組み立てた後・zip の前」に外から手を入れられる口（組み立てと詰める段を分ける、または署名の段を呼ぶ口）が要る＝brief の Out of Boundary「配布スクリプトの中身」に触る。設計の段で境界を改める。
  - 署名する exe は `areka.exe`・`shiori-host32-helper.exe` の 2 本（`tools/package.ps1` の zip の中身の一覧）。C4 の候補 `mcp-stdio-bridge` が Desktop 用の中継 exe を zip に足すと 3 本になる。
  - SHA256（`.sha256`）は署名後の zip から作られる＝`winget-manifest-submission` の `InstallerSha256` の手順は変わらない（brief の見込みどおり）。
  - `dist/README.txt` の「■ 既知の制限」の先頭が「areka.exe には署名がありません」。`winget-manifest-submission` が Smart App Control の 1 行をここへ足す予定＝本 spec はその両方を直す。
- 触るファイル（並走の照合用）:
  - `.github/workflows/release.yml`（署名の段・SignPath の Action）
  - `tools/package.ps1`（組み立てと詰める段の間の口・新しく境界に入れる）
  - `doc/SIGNING.md`（新規）
  - `README.md`・`dist/README.txt`（署名の方針と既知の制限）
- 議題（答えで作業が変わるものだけ）: ⑴（既存）受理されなかったときに有料へ行くか。⑵ `tools/package.ps1` を本 spec の境界に入れてよいか（入れないなら、zip を一度作ってから展開・署名・詰め直す形を `release.yml` に書くことになり、`.sha256` と `BUILD-INFO.txt` の作り方を二重に持つ）。
- 見つけた穴: なし。
