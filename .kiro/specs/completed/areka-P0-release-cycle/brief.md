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

## ほかの spec からの申し送り（次の版を決めるときに読む）

- **2026-10-10 `areka-P0-anchor-tag-canon`（完了時の棚卸）**: crates.io に出している `dola` の合図の語彙 `CueCommand`（`crates/dola/src/cue/command.rs`）に 2 種類（`AnchorBegin`・`AnchorEnd`）が増えた。`CueCommand` は `#[non_exhaustive]` でないので、全種類を catch-all なしで並べている外の利用側は、次の版でビルドが通らなくなる。次に公開する版の上げ方は、これを踏まえて本 spec の側で決める（0.0.x の間は、cargo の上ではどの版上げも非互換の扱い）。詳細は `doc/anchor-compat.md` の 3.2。
- **2026-10-11 `areka-P0-release-notes-highlights`（`/kiro-discovery` の起票）**: リリースの本文を、自動の PR の題名の一覧から、注目に値する数件の日本語の紹介に替える spec を起票した。同 spec が、本 spec の要件・設計・タスクに「紹介文を下書きして版上げの PR に載せる段」を足し、公開の後の確かめを 4 つのファイルから 5 つ（詳細なリリースノートのファイルが増える）に直す。**次の回（`v0.0.3`）は、同 spec が着地してから打つ**（着地の前に打つと、本文は今までどおり自動の一覧になる）。
- **2026-10-11 `areka-P0-release-notes-highlights`（完了時の棚卸・開発者の決定）**: 開発者が「外から見た動きは pasta と同じく承認なしでノンストップ。`areka-P0-release-cycle` を完了にして、`/release` を正にするのが目的」と決めた。同 spec が入れた `/release` は入口だけで、手順の正本はまだ本 spec の `tasks.md` と `design.md` に在る。移行は新しい spec を立てず、本 spec の最後の仕事として、新しいワークツリーで行う: 要件のマージの承認とタグの承認を外す → 手順（段・コマンド・判定の表・再開の読み方）をスキルの中へ写す → 本 spec を普通の完了の手順で `completed/` へ移す → `workflow.md` と `kiro-complete` の「繰り返し仕様の例外」・roadmap・ほかの spec の brief の参照を合わせる。**開発者の答え待ちが 5 点**（かっこの中は推す答え）: ⑴ 見どころを人が読む場（全文を示して止まらずに進む）⑵ 承認の後に main が動いたときの「足す物は在るか」の止まり（無くす）⑶ roadmap の「リリース」の表への記録と次の PR への相乗り（やめる・GitHub の Release の頁を記録とする）⑷ 走りが赤のとき（今までどおり止まって聞く）⑸ 上の `dola` の件と、`deny.toml`・`about.toml` の `BSD-2-Clause` の掃除（スキルの手順の「版上げのときに一緒に見る事」へ移す）。**前提が 1 つ**: 自動モードの見張りが承認なしの `gh pr merge` とタグの push を止める見込みなので、`/release` 用の許可ルールを開発者の設定に足してもらう（文面は移行のときに示す）。pasta の形は ekicyou/pasta の PR#105（`.claude/skills/release/` の `SKILL.md`・`procedure.md`・`scripts/`）。

## 2026-10-11 `/release` へ移す仕事の作業計画（`/kiro-discovery`・main `d0c697d6`）

上の最後の申し送りを書いたセッション「areka release notes」と直接やり取りして引き継いだ。新しい spec は立てない（本 spec の最後の仕事）。作業の枝は `claude/areka-release-notes-plan-6885b1`（この計画のコミットと同じ枝・PR は 1 本）。

### 目的と、終わったときの形

- 開発者が `/release` を打つと、順調なら承認で止まらずに最後まで進む（pasta と同じ外から見た動き）。
- 手順の正本は `.claude/skills/release/` の中（`SKILL.md` と `procedure.md`）。本 spec は `completed/` の記録になり、実行には使わない。
- 止まるのは、判定が赤のとき・走りが赤のとき・手順に無い状態・許可の判定に断られたときだけ。

### 引き継ぎで分かった事（文書に無かった物）

- **pasta との違いは残る**: areka のリリース CI はテストを回さない。手元の確かめ（`tools/test-all.ps1 -License` 約 19 分・`tools/package.ps1 -Check`＝窓が出る・`tools/crates-io.ps1 -Verify`・`tools/release-notes.ps1 -Check`）は手順に残す。止まらなくても 1 回 30 分ほど掛かる。
- **スキルの中にスクリプトは置かない**。道具は今の `tools/*.ps1` のまま（本 spec の設計の「新しいスクリプトは 0 本」を保つ。`.claude/skills/` の下の ps1 は文字コードの検査の外になる）。
- **完了の関門**: `kiro-complete` は `tasks.md` の全項目の完了を求める。今の `tasks.md` は未完了 30・完了 0（リリース 1 回分の手順）なので、移行の仕事の一覧に置き換えてから完了にする。
- **試し打ちはできない**: スキルは開発者が打ったときだけ動く設定で、動かせば本物のリリースになる。実走は `v0.0.3`（`user-data-root` の着地待ち）が初めてで、見どころを書く段と公開の段もそこで初めて本番で動く。
- **この計画で確かめた事**: ほかに繰り返し spec は無い（`.kiro/specs/` の直下で「繰り返し」を名乗るのは本 spec だけ）。`crates/`・`tools/`・`.github/`・`dist/` に `release-cycle` の語は 0 件（移動で壊れる実ファイル読みは無い）。

### スキルへ写す物・落とす物

- **写す**（`procedure.md`・300〜350 行の見込み）: 設計の「Components and Interfaces」の前置き（置き換える語 `{版}`・`{旧版}`・`{remote}`・`{枝}`・`{sha}`、`git` の出力の決まり）、段 1〜6、段 3 の「差分の範囲の判定」の表、段 6 の「緑の判定」と「赤のときの決まり」の表、「再開のときの事実の読み方」、「止まり方の決まり」、「してはいけない依存」（`git remote -v`・`--tags`・`--follow-tags`・`--force`・`test-all.ps1 -Format`）。`tasks.md` の 1.1〜6.3 の箇条と各「判定」。
- **落とす**: `/kiro-impl` の読み替えの表、「1 回の実行で作るコミット」の表、承認 A・B の文、マージの前の文章の直しの戻り道、`tasks.md` のチェックを戻す話、記録の形（Data Models）、Requirements Traceability、Testing Strategy、Boundary の各節、「設計で決めたこと」、初回（`v0.0.2`）だけの分。
- **答えしだい**: 段 7（記録と受け渡し）は下の ⑶、段 4 の「新しく入った題名を示して止まる」は下の ⑵。

### 仕事の一覧（この順）

1. **本 spec の文書を「記録」に直す**。要件・設計の先頭に「完了した記録・実行には使わない・正本は `.claude/skills/release/`」と書き、移行の節を 1 つ足して、覆る行を名指しする: 要件 4.3（マージの承認）・5.2（タグの承認）・1.3・1.4・1.8（タスクを未完了に保つ・実装中のまま置く・`/kiro-complete` を使わない）、⑶ の答えしだいで 7.1〜7.4。完了済みの `release-notes-highlights` の要件 6.4・6.7・6.9 を上書きすることも 1 行書く（完了した spec の文書は書き換えない）。要件・設計の全文の書き直しはしない。
2. **`tasks.md` をこの一覧に置き換える**（もとの手順は `procedure.md` と git の履歴に残る）。
3. **`procedure.md` を新しく書き、`SKILL.md` を書き直す**。コマンドはもとの設計・タスクから一字も変えずに写す（承認で止まる文だけを外す）。`SKILL.md` は約束と段の要約で、「段を始める前に `procedure.md` のその段を読み直す・記憶でコマンドを書かない・外へ出るコマンドは単独の 1 行」を残す。
4. **参照を合わせる**（語は `release-cycle`。`completed/`・`roadmap-history.md`・本 spec のフォルダは除く）:
   - `CLAUDE.md` 2 行・`product.md` 1・`tech.md` 1・`focus.md` 1・`kiro-next` の SKILL 1・`doc/anchor-compat.md` 1。
   - ほかの spec の brief 12 本（`winget-release-automation` 8 か所は手順の中身を指すので、指す先をスキルへ書き直す。ほかの 11 本は 1〜3 か所ずつ）。
   - `roadmap.md` 約 20 か所。「席」と「版上げと winget の順」の文は主語を `/release` に言い替える。`Dependencies: areka-P0-release-cycle` は歴史の名前なので置き換えない。台帳の行を外して数を直すのは完了の手順の中で数え直す。
   - `workflow.md` と `kiro-complete` の「繰り返し仕様の例外」は、仕組みは残して、例に挙げた「リリース手順」の語だけを外す（共通の文書は例外の spec のために直さない、という 10-05 の決まりに沿う最小の直し）。
5. **確かめる**（テストは回さない・文書だけの変更）:
   - もとの設計・タスクのコマンドの行と `procedure.md` のコマンドの行を機械で突き合わせ、増えた行 0・減った行は「落とす」に挙げた物だけ。
   - `SKILL.md` が名前で指す節が `procedure.md` に全部在る。
   - 文字コードの検査（`tools/encoding-check.ps1`）が終了 0。
   - `release-cycle` の語の残りを一覧にして、残す理由（歴史の名前）を 1 行ずつ付ける。
   - レビュー役のサブエージェントを 1 回（もとの文書と `procedure.md` を渡し、「落ちた手順・足された手順は在るか」だけを見させる）。実走で試せないので、ここは省かない。
6. **開発者の「実装完了を承認」の後に `/kiro-complete areka-P0-release-cycle`**（`completed/` への移動・全体テスト 1 回・PR・squash マージ）。
7. **`v0.0.3` の初めての実走で見る事**を、スキルの外に残す（roadmap の 1 行）: 承認なしのマージとタグの push が許可の判定を通るか・見どころの段・公開の段。

### 開発者の答え待ち 5 点（引き継ぎで推す答えが 2 つ変わった）

> **答え（2026-10-11 開発者）**: 1 は「見どころは `/release` の最後の手順で報告する。直しを頼むかもしれない」。2〜5 は推す答えに同意。決まった形は `requirements.md` の末尾の節「手順をスキル `/release` へ移した」に在る。`BSD-2-Clause` の掃除は、版上げの PR に混ぜられないので、申し送りにせず、この仕事の PR で片づけた。

1. **見どころを人が読む場**: 全文を示して、止まらずに進む。弱みは、口調と「必ず載せるもの」を機械で見ていない点。公開の後に Release の頁の本文は直せるが、タグのコミットの中のファイルは直せない。
2. **PR を出した後に main が動いたとき**（推す答えを変えた）: ただ止まりを無くすのではなく、main を取り込んだら変更の一覧を出し直し、AI が「必ず載せるもの」に当たる変更を見どころへ足して検査を回し直す（止まらない）。10-11 に「止まる」を残した理由（後から入った変更は見どころを書いたときに見ていない・タグは動かせない）を保てる。
3. **roadmap の「リリース」の表への記録**: やめて、GitHub の Release の頁を記録とする（pasta も記録のコミットを作らない）。今ある行は歴史として残す。弱みは、winget-pkgs の PR の URL と「赤で起票した spec」の列の置き場が無くなる点。渋るなら代わりは「版上げの PR の中で行を書く（Release の URL は版から決まる・winget の列は空）」。
4. **走りが赤のとき**: 今までどおり止まって聞く（やり直しも開発者の判断）。pasta は一部の赤だけ 3 回まで自動でやり直すが、areka は写さない。
5. **次の版への申し送りの置き場**（推す答えを変えた）: 本 spec の brief が記録になると、ほかの spec が「次の版上げのときに」と書き残す先が無くなる。roadmap に「次のリリースへの申し送り」の節を 1 つ置き、`procedure.md` の最初の段でそこを読む。今ある 2 件（`dola` の `CueCommand` の追加・`deny.toml` と `about.toml` の `BSD-2-Clause`）はそこへ移し、`v0.0.3` で片づけたら消す。スキルの中には置かない（スキルの書き換えは見張りに止められることがある）。

### 前提（開発者の設定・`v0.0.3` を打つ前までに）

自動モードの見張りは、承認なしの `gh pr merge` とタグの push を止める見込み。今の文のルールは「実装完了を承認」の発言に結び付いている。同じ型で `/release` 用を 1 つ足してもらう。文面の案:

> Release: when the user's message is the `/release` command (optionally with a version), the agent may push its harness worktree branch, create the version-bump PR with `gh pr create --base main`, squash-merge that PR with `gh pr merge --squash`, create the tag `v{version}` on that squash commit and push only that tag (`git push <remote> refs/tags/v{version}`). This does not cover `--admin`, bypassing required checks, merging any other PR, pushing directly to main, `--force`, `--tags`, moving or deleting a pushed tag, or `gh release delete`.

タグの push がこの形で通るかは、まだ誰も試していない。移行の仕事そのもの（6 の完了の手順）は今のルールで通る。

### 触らない物

- `tools/`・`.github/workflows/`・クレート・`doc/release-notes/README.md`（見どころの決まりは pasta とそろえてある。変えないので pasta への知らせは要らない）。
- 完了済みの spec の文書（`release-notes-highlights` を含む）と `roadmap-history.md`。
