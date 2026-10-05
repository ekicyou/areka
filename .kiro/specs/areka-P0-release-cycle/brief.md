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
5. 見守る: `release.yml` の緑・Release の公開・crates.io の各クレートの版・winget-pkgs への PR（`winget.yml`）。赤のときの決まり（10-03 開発者）: `release.yml` が赤で Release が残っておらず、タグを動かさずに済む原因（通信の失敗・時間切れ・取り消し）なら、同じ走りを「Re-run」でやり直してよい（まだ何も配られていない）。コミットの直しが要る赤と、Release の公開より後（crates.io・winget）の赤は、原因を直す spec を起票し、**同じ版で出し直さない**（zip の URL は版ごとに固定・crates.io は差し替え不可）＝タグを動かさず、直したら次の版で出す。
   - 初回の実走で初めて動くもの（10-03 `release-ci-workflow` の完了時に申し送り・詳細は `.kiro/specs/completed/areka-P0-release-ci-workflow/verification/runner-trial.md` の「release-cycle への申し送り」）: Release の公開の段（4 つのファイル・公開の状態・自動のノートが一つ前のタグからの範囲）・後始末の段の消す経路（タグは残る）・下書きの Release が一時のトークンから見えるか・赤の走りの Re-run・マージ後の乾いた走り（main と `release.yml` を含むタグで始められるか。`v0.0.1` からは始められない）。どれも完了の条件ではなく、見守って赤なら直す spec を起票する。
6. 記録: `roadmap.md` の「完了サマリ」の下に「リリース」の 1 行（版・日付・Release の URL・winget の PR）。
7. **初回（`v0.0.2`）だけの手順**: タグを打つ前に、`wintf`・`dola` に Trusted Publishing を設定する（`crates-io-publish` の手順書。10-03 同 spec の要件討議の議題 3 で crates.io へ出すのは `wintf`・`dola` だけになり、どちらも crates.io に在るので手元からの初回の公開は要らない）。版を上げるときに書き換えるのは、根の `Cargo.toml` の 2 行（`[workspace.package]` の `version`・`[workspace.dependencies]` の `dola` の `version`）と `Cargo.lock` だけで、各クレートの `Cargo.toml` は動かさない（`crates-io-publish` の設計で、出さない `areka` ほかが持っていた `wintf` の版の指定は外した）。上げた後に `tools/crates-io.ps1 -Verify` を通す。手順 2 の `cargo set-version --workspace` で 2 行目（`dola` の `version`）も動くかは、この spec で確かめる。winget の初回の手提出は `winget-manifest-submission`（Release の実在が要る＝初回の後のウェーブ）。

## Approach

- タグは人が打つ（workflow が打たない）。版の正本は `Cargo.toml`、タグはそれを写す。`release.yml` は一致を検査して違えば止める。
- 繰り返し spec の形: `spec.json` の phase は `implementation-in-progress` のまま置き、`tasks.md` を毎回リセットする。`/kiro-impl areka-P0-release-cycle` で 1 回分を回す。
- 採らない案: main への push で自動リリース（PR のたびに版が上がる）／`cargo release` で main へ直接 commit・push（PR 経由の決まりに反する）／feature の PR の中で版を上げる（並走する枝が同じ行を取り合う・リリースの時期を開発者が選べない）。

## Scope

- **In**: 繰り返しの手順（tasks.md の形）・版上げの道具（`cargo set-version` か手元の 1 行）・版とタグの決まり・見守りと赤のときの決まり・記録の形・初回の実行（`v0.0.2`）。
- **Out**: workflow の中身（`release-ci-workflow` ほか）・マニフェストの形（`winget-manifest-submission`）・配布スクリプト（`release-package-versioned`）・大きい版の決め方（0.1.0 や 1.0.0 は開発者がその場で指示する＝本 spec は指示が無ければ patch・指示があればその版にするだけ）。

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
- 1 回の実行で上げる版は、開発者の指示が無ければ patch（+0.0.1）。指示があればその版（10-03 開発者）。

## 想定

- 規模 XS（手順 6 本・毎回同じ）。議題 1 件（版上げを cargo-edit に頼るか、`Cargo.toml` の 1 行を書き換える手元の小さなスクリプトにするか＝道具を足さない方を推す）。Opus で足りる。


---

## 2026-10-04 棚卸㉑の再測定（main `634032f6`・C2 の着地の後）

- 規模: XS〜S（初回 8〜11 タスク・2 回目からは手順 6 本）。切る: なし。初回だけ重いのは、要件〜タスクを作る段と、`release-ci-workflow` が申し送った初回だけの見守り 6 項目（`completed/areka-P0-release-ci-workflow/verification/runner-trial.md` の「release-cycle への申し送り」1〜6）が乗るため。
- 前提の状態: 上流 3 本（`release-package-versioned`・`release-ci-workflow`・`crates-io-publish`）はすべて着地した。`.github/workflows/release.yml`（タグ `v*` の push と手での起動）・`.github/workflows/crates-io.yml`（タグ `v*` の push を自分で受け、同じタグの `release` の走りの緑を最長 120 分待つ）・`tools/package.ps1`・`tools/crates-io.ps1`・手順書 `doc/crates-io-publish.md` が main に在る。根の `Cargo.toml` は `[workspace.package]` の `version = "0.0.1"` と `[workspace.dependencies]` の `dola = { version = "0.0.1", path = … }` の 2 か所で、各クレートの `Cargo.toml` に `wintf`・`dola` の版の指定は無い（brief の手順 7 のとおり）。
  - ただし手順 1 の「open PR 0 本」は、C2 の残り 2 本（`install-companion-reading`・`mouse-drag-events`）が着地するまで満たせない。
- 崩れた前提／古くなった位置:
  - `crates-io-publish` の裁定（案 B）で、公開の段は `release.yml` から呼ばれず、タグの push を自分で受ける。手順 5 の「見守る」相手は `release`（Release を作る走り）と `crates-io`（公開の段）の 2 本が同時に始まる形になった。brief の手順 5 は「Release の公開より後（crates.io・winget）の赤は原因を直す spec を起票し、同じ版で出し直さない」と書くが、手順書（`doc/crates-io-publish.md` 5 節の末尾）は「その決めごとは `release.yml` の話で、公開の段は同じ版で何度起動し直してもよい（既に出たクレートは飛ばす）」と決めた。tasks.md は手順書に合わせる（`crates-io` の赤は Re-run／Run workflow で同じ版のまま残りを出す。版を上げ直すのはタグのコミットのコードやスクリプトの誤りのときだけ）。
  - `winget.yml` はまだ無い（`winget-manifest-submission`＝C3-①）。初回の見守りの相手に winget は入らない。
  - 開発者の手が要るもの（初回）: ⑴ crates.io で `wintf`・`dola` それぞれに Trusted Publishing を足す（持ち主 `ekicyou`・リポジトリ `areka`・workflow `crates-io.yml`・environment は空。`doc/crates-io-publish.md` 2 節）。済んでいないと公開の段は段「鍵」で止まる（何も上がらない）。⑵ 版上げの PR の squash マージとタグの push（公開に当たる操作）。⑶ 手順 1 の `tools/package.ps1 -Check`（窓が出る起動確認）。⑷ 赤のときの Re-run の判断。シークレットの登録は初回には要らない（`release.yml` は `GITHUB_TOKEN` の `contents: write` だけ・`crates-io.yml` は Trusted Publishing だけ）。
  - `release.yml` は Rust を `1.99.0` に固定し、`crates-io.yml` は `rustup update stable` で最新を使う。公開前の確認（`-Verify`）が Release の組み立てと別の版の Rust で走る＝赤なら版の差を先に疑う（見守りの覚え）。
- 触るファイル（並走の照合用）:
  - `.kiro/specs/areka-P0-release-cycle/`（要件〜tasks.md・新規）
  - `Cargo.toml`（根の 2 行だけ）・`Cargo.lock`
  - `dist/README.txt`（冒頭の「2026-10-01 時点」の行）
  - `README.md`（下の議題 2 で取るなら「## 入手と起動」の 1 行）
  - `.kiro/steering/roadmap.md`（リリースの記録 1 行）
- 議題（答えで作業が変わるものだけ）:
  1. **初回の PR の分け方**。初回は `/kiro-start` で要件〜タスクを作るので、spec の文書のコミットが要る。一方 brief の手順 3 は「版上げの PR に他の変更を混ぜない」。⒜ spec の文書だけの PR を先に入れ、版上げの PR を別に出す ⒝ 初回だけ同じ PR に入れる。さらに手順 6（roadmap への記録）はタグの後なので、版上げの PR には入らない＝⒜ 次の spec の PR か棚卸に相乗り ⒝ 記録だけの PR を出す、のどちらか。
  2. **`README.md` の「まだ GitHub Releases での配布はしていない」の行**（「## 入手と起動」の最初の箇条・PR#222 で書いた）。`v0.0.2` の Release が出た時点で偽になる。本 spec の触る範囲（Constraints）に `README.md` が無く、次に触る `winget-manifest-submission` は C3＝間に偽の期間ができる。初回だけ本 spec が直すか、偽の期間を許して winget に任せるか。
  3. （既存）版上げを cargo-edit の `cargo set-version` に頼るか、根の 2 行を書き換える小さなスクリプトにするか。`set-version --workspace` が `[workspace.dependencies]` の `dola` の `version` も動かすかは未確認のまま。
- 見つけた穴: なし（上の `README.md` の行は議題 2）。


## 2026-10-05 棚卸㉒の再測定（main `f26aa1c1`・C3 の着地の後）

- 規模: XS〜S（初回 8〜11 タスク・2 回目からは手順 6 本）。切る: なし。
- 前提の状態: **今すぐ着手できる**。C3 の 11 本はすべて着地し、GitHub の開いている PR は 0 本（10-05 に `gh pr list` で確かめた）。上流の物は main に揃っている:
  - `.github/workflows/release.yml`（タグ `v*` の push と手での起動・Rust は `1.99.0` に固定・テストは回さず、組み立てと zip と Release だけ）
  - `.github/workflows/crates-io.yml`（タグの push を自分で受け、同じコミットの `release` の走りの緑を最長 120 分待つ・`id-token: write`）
  - `tools/package.ps1`（726 行）・`tools/crates-io.ps1`（出すのは `dola` → `wintf` の順）・手順書 `doc/crates-io-publish.md`
  - 根の `Cargo.toml` の版は 2 行＝`[workspace.package]` の `version = "0.0.1"` と、`[workspace.dependencies]` の `dola = { version = "0.0.1", path = "crates/dola" }`。30 クレートすべてが `version.workspace = true`。`wintf` は `dola = { workspace = true }` でこの 2 行目を使う（各クレートの `wintf`・`dola` の行に版の指定は無い）。
  - タグは `v0.0.1` だけ。`release.yml` が実行環境で走ったのは 10-03 の作業の枝の 1 回（緑）だけで、`crates-io.yml` は一度も走っていない。
- C3 で変わったこと:
  - `animated-image-decode` が根の `Cargo.toml` に `[patch.crates-io]`（`image-webp` を GitHub のコミットに固定）を、`deny.toml` に `allow-git` を足した。`wintf` は開発用の依存で `image`（webp）を引く。同 spec は `tools/crates-io.ps1 -Verify -Version 0.0.1` を取り込みつきで通し、包んだ `wintf` の `Cargo.lock` は crates.io の `image-webp` 0.2.4 のままと記録した（`completed/areka-P0-animated-image-decode/research.md`）＝公開の段への影響は無い見込み。
  - ただし git の取り込みを足した後の main で、`release.yml`（実行環境での git の取得・`cargo deny`・`cargo about`）はまだ一度も走っていない。
  - `dist/README.txt` に「■ 動く絵の上限」の節が足された（10-05）。冒頭の「この説明書は 2026-10-01 時点の内容です。」は据え置きのまま＝日付も中身も古い。
- 版上げが触るファイル（並走の照合用・数えた結果）:
  - `Cargo.toml` の 2 行（上の 2 行）
  - `Cargo.lock` の 30 行（ワークスペースの 30 クレートの `version = "0.0.1"`。`crates-io-publish` の調べで「版の行だけが動く」を確かめ済み）
  - `THIRD-PARTY-NOTICES.md` の 30 行（MIT の「対象 crate」の一覧に `areka 0.0.1` 〜 `wintf 0.0.1` の 30 行がある。zip に入る謝辞は `tools/package.ps1` が毎回作り直すので配布物は正しいが、リポジトリの写しは古くなり、`tools/test-all.ps1 -License` が黄色で知らせる）
  - `dist/README.txt` の冒頭の「時点」の行
  - `README.md` の「## 入手と起動」の最初の箇条（議題で取るなら）
  - `.kiro/steering/roadmap.md`（記録 1 行・タグの後）・本 spec のフォルダ
- 「開いている PR 0 本」の本当の要件:
  - 版上げとぶつかるのは `Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md` を触る枝だけ。とくに**新しいクレートや依存を足す枝**（例 `mcp-stdio-bridge`）は、版上げの前に作った `Cargo.lock` に新しいクレートが `0.0.1` で載ったまま、字の上では衝突せずにマージされうる＝`--locked` の組み立てが落ちる。そういう枝は版上げの後に取り込み直して `Cargo.lock` を作り直す。
  - `Cargo.*` に触らない枝（`dist/README.txt` の別の節・`README.md` の別の行だけを触る枝を含む）は、版上げと同じウェーブで走らせてよい。各 spec は squash の 1 PR なので、作業中の枝の中身はリリースに入らない。
  - 残る危険は「タグを打つコミットの中身が、手元の全体テストで確かめた物と同じか」だけ。Release の段はテストを回さない。版上げの枝の全体テストの後、マージまでに main へ別の PR が入ったら、取り込んで全体テストを回し直す。タグは版上げの squash のコミットに打つので、その後に入った PR は入らない。
  - 手順 1 の言い換えの案: 「`Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md` を触る開いた PR が 0 本。版上げの PR のマージまで main が動かない（動いたら取り込んで全体テストを回し直す）」。
- `cargo set-version` の確かめ（インストールも実行もせず、ソースを読んだ）:
  - 開発者の手元に cargo-edit 0.13.13 が入っている（`c:\rust\cargo\bin\cargo-set-version.exe`・10-02）。ソースは `c:\rust\cargo\registry\src\` の下の `cargo-edit-0.13.13`。
  - `--workspace` は `[workspace.package]` の `version` を上げ、続けて各クレートについて「依存している側の版の指定を合わせる」処理（`set_version.rs` の `update_dependents`）を呼ぶ。この処理は根の `Cargo.toml` も見る。表を集める関数（`manifest.rs` の `get_dependency_tables_mut`）は `[workspace.dependencies]` も返し、`path` がそのクレートを指して `version` を持つ行を新しい版に書き換える（`version.rs` の `upgrade_requirement`。`"0.0.1"` は `"0.0.2"` になる）。
  - `dola` の行はこれに当たる＝**2 行目も動く見込み**。最後に `cargo metadata` を走らせて `Cargo.lock` も書き直す。`wintf-winmsg-executor = "=0.0.5"` は `path` の無い crates.io の依存なので動かない。
  - 実際に動かしての確かめは実装の段で行う（`git diff --stat` が `Cargo.toml` の 2 行と `Cargo.lock` の 30 行になること）。`dola` の行を動かし忘れても `cargo check` が「failed to select a version for the requirement `dola = "^0.0.1"`」で落ちる（`crates-io-publish` の調べで確かめ済み）＝見落とせない。
- 開発者の手が要るもの（初回）:
  1. crates.io で `wintf`・`dola` のそれぞれに Trusted Publishing を足す（持ち主 `ekicyou`・リポジトリ `areka`・workflow `crates-io.yml`・environment は空。`doc/crates-io-publish.md` の 2 節）。タグの push より前に。
  2. 手元の `tools/test-all.ps1` と `tools/package.ps1 -Check`（窓が出る起動の確かめ・x64）。
  3. 版上げの PR の squash マージと、タグ `v0.0.2` の push。
  4. 赤のときの Re-run の判断。
  5. （勧め）タグの前に、main で `release.yml` を手で起動する乾いた走りを 1 回（Release は作らない）。git の取り込みを足した後の main で、実行環境の組み立てを先に確かめられる。`release-ci-workflow` の申し送りの 5（マージ後の乾いた走り）もここで消せる。
  - シークレットの登録は要らない。winget の PAT と winget-pkgs のフォークは `winget-manifest-submission` の番。
- 議題（答えで作業が変わるものだけ）:
  1. （既存）初回の PR の分け方。
  2. （既存）`README.md` の「まだ GitHub Releases での配布はしていない」の行を本 spec で直すか。
  3. （既存）cargo-edit に頼るか＝上の読みで「cargo-edit で足りる・道具を足さない」に傾いた。
  4. 手順 1 を上の言い換えに緩めるか。緩めれば、`Cargo.*` を触らない spec と同じウェーブに置ける。
  5. 版上げの PR で `THIRD-PARTY-NOTICES.md` を作り直すか（推し: 作り直す。リポジトリの謝辞が出した版と合う）。
- 見つけた穴: brief の Constraints と手順 2 の触るファイルに `THIRD-PARTY-NOTICES.md` が無い（上）。
- すぐ直せる軽微な修正: なし（brief の文の直しは要件の段で足りる）。

### 同じウェーブ C4 の約束（2026-10-05 棚卸㉒・破るなら止めて報告）

- 版上げの PR を出すとき、`Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md` を触る開いた PR は 0 本（C4 のほかの 12 本はどれも触らない約束）。版上げの全体テストの後に main が動いたら、取り込んで全体テストを回し直してからタグを打つ（Release の段はテストを回さない）。
- `dist/README.txt` は「時点」の行だけ（`self-alpha-declaration` が既知の制限の行を直す＝別の行）。
