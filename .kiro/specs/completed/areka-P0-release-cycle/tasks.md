# Implementation Plan

> **この文書は完了した spec の記録で、実行には使わない**（2026-10-11）。リリースの手順の正本は `.claude/skills/release/`（`SKILL.md` と `procedure.md`）で、開発者が `/release` を打って回す。

## この一覧について

- もとの `tasks.md` は、リリース 1 回分の手順（7 つの段・未完了の項目 30）だった。毎回チェックを戻して使う形で、`v0.0.2`（2026-10-06）を 1 回出した。その中身は `.claude/skills/release/procedure.md` へ移した。もとの文は git の履歴（この置き換えの 1 つ前のコミット）に残る。
- 下は、本 spec の最後の仕事（手順をスキル `/release` へ移し、承認なしで最後まで進む形にして、本 spec を完了にする）の一覧である。経緯と開発者の決定は `requirements.md` の末尾の節と、`brief.md` の「`/release` へ移す仕事の作業計画」に在る。
- 製品のコードは 1 行も変えていない（文書とスキルと、ライセンスの一覧の 2 行だけ）。

## Tasks

- [x] 1. 本 spec の文書を記録に直す
- [x] 1.1 要件と設計の先頭に「完了した記録・実行には使わない」と書き、要件の末尾に、今の形と食い違う行を名指しする節を足す
  - 覆る行: 要件 1.3・1.4・1.8・4.3・5.2・7.1〜7.4、2.7 に足した止まり、初回だけの 8.1〜8.8。完了済みの `release-notes-highlights` の要件 6.4・6.7・6.9 を上書きすることも書く。
  - 要件・設計の本文は書き直さない。
- [x] 1.2 `tasks.md` を、リリース 1 回分の手順から、この一覧に置き換える

- [x] 2. 手順をスキルの中へ写す
- [x] 2.1 `.claude/skills/release/procedure.md` を新しく書く
  - 写す: 置き換える語と `git` の決まり・段 1〜6・差分の範囲の判定の表・緑の判定と赤のときの決まりの表・再開のときの事実の読み方・止まり方の決まり・してはいけない事。
  - 落とす: `/kiro-impl` の読み替え・承認 A と承認 B・記録と受け渡しの段・チェックを戻す話・初回（`v0.0.2`）だけの分。
  - 足す: 作業の枝の読み方（`git branch --show-current`）・次のリリースへの申し送りを読む項目（1.4）・main が動いたときに見どころを自分で足す手順（4.2）・最後の報告に見どころの全文を載せる段（段 7）・公開の後に見どころの直しを頼まれたときの手順・開発者の設定に足す許可ルールの文面。
  - 再開の読み方は、同じ枝から出したほかの PR を数えないよう、題が `chore(release): v` で始まる PR だけを見る形にする。
- [x] 2.2 `.claude/skills/release/SKILL.md` を、入口だけの形から、約束と段の要約の形に書き直す

- [x] 3. 参照を合わせる
- [x] 3.1 `release-cycle` を指す文を、スキル `/release` を指す形に直す
  - `CLAUDE.md` 2・`product.md` 1・`tech.md` 1・`kiro-next` のスキル 1・`doc/anchor-compat.md` 1・roadmap の先の話の 4 か所・ほかの spec の brief 12 本（計 20 か所）。
  - `doc/release-notes/README.md` の末尾の 1 文（開発者が見どころを読む場）を、今の形に直す。
  - `workflow.md` と `kiro-complete` の「繰り返し仕様の例外」は、仕組みを残して、例に挙げた「リリース手順」の語だけを外す。
  - 歴史を語る文（完了の記録・`Dependencies:` の並び・済んだ経緯）は置き換えない。
- [x] 3.2 roadmap に節「次のリリースへの申し送り」を作り、本 spec の brief に宛てていた `dola` の `CueCommand` の件を移す。「リリース」の表には「`v0.0.3` からは行を足さない」と書く
- [x] 3.3 `deny.toml` と `about.toml` の一覧から、今の依存に無い `BSD-2-Clause` を外す
  - 版上げの PR には版上げ以外を混ぜないので、申し送りとして残さず、ここで片づける。

- [x] 4. 確かめる
- [x] 4.1 もとの設計・タスクのコマンドと `procedure.md` のコマンドを機械で突き合わせる
- [x] 4.2 `SKILL.md` と `procedure.md` の中の、節の名前と項目の番号の指し先が全部在ることを確かめる
- [x] 4.3 文字コードの検査を回す
- [x] 4.4 `release-cycle` の語の残りを一覧にして、残す理由を確かめる
- [x] 4.5 レビュー役に、もとの文書と `procedure.md` を読ませ、落ちた手順・足された手順・承認を外したことで開いた穴を見させる

## Implementation Notes

- **4.1 の結果**（2026-10-11）: バッククォートで囲んだ語を両方から集めて比べた。`procedure.md` にだけ在るコマンドは、2.1 の「足す」に挙げた物だけ（`git branch --show-current`・`gh pr list --head {枝} … ,title`・`tools/release-notes.ps1 -Body …`・`gh release edit v{版} --notes-file …`・見どころの直しの `gh pr create`・`gh release view v{版} --json body`・一覧の先を main に指す `-To`・手元のタグを読む `git rev-parse --verify --quiet refs/tags/v{版}^{commit}`）。もとにだけ在るコマンドは、初回だけの物（`gh workflow run release.yml --ref …`・`gh run view {id} --json jobs` ほか）、記録の段の物（`git cherry-pick`・`git ls-remote --quiet --heads {remote} {枝}`・`git show HEAD:…/tasks.md` ほか）、承認 A を取り直すかを決めていた `git diff {remote}/main HEAD -- …` だけ。
- **4.3 の結果**: `tools/encoding-check.ps1` が終了コード 0（`encoding check: all passed`）。
- **4.4 の結果**: 完了済みの spec・`roadmap-history.md`・本 spec のフォルダの外に残る `release-cycle` は、roadmap の完了の記録と `Dependencies:` の並び・台帳の行（完了の手順で外す）・`winget-release-automation` の brief の済んだ経緯の 3 か所・`focus.md` の数の行（完了の手順で数え直す）。
- **3.3 の結果**: 外した後の `cargo deny check licenses` は「出会わなかったライセンス」の警告なしで `licenses ok`。作り直した謝辞は `THIRD-PARTY-NOTICES.md` と 1 字も変わらない。
- **4.5 の結果**: レビュー役（サブエージェント 1 回・読むだけ）の指摘は、重大 2・中 6・軽 6。変えられたコマンドと、指し先の欠けは 0 件。全部を `procedure.md` と `SKILL.md` に反映した。
  - 重大: 再開の表が「マージ済みでタグも在る」に 2 つの答えを出していた（見守りの途中で切れると次の版を上げうる）→ 表を上から順に当てる形に書き直し、済んだ回の後は引数で新しい版を指示したときだけ次の回を始める。再開したセッションは「確かめた main」を覚えていない → 開いた PR からの再開は、全体テストを必ず 1 回回し直してからマージする。
  - 中: 手元に古い同じ名前のタグが在ると違うコミットを押しうる → 5.1 と 5.2 に手元のタグの読みを足した。変更の一覧は本線だけを読むので、main を merge で取り込んだ枝の先頭からは main の変更が出ない → 一覧の先を `-To` で main に指す。PR を出す前に切れた回の行を足した。申し送りは版を決める前に読む（1.4 → 1.5）。起票で `/kiro-discovery` が roadmap に書く分は「roadmap を書き換えない」から除く。見どころの直しの入口を `SKILL.md` に足した。
  - 軽: main を取り込んだ後の確かめの順（見どころの直し → 全体テスト → 差分の範囲 → push）・再開のときの `{sha}` の出どころ・「見直すきっかけ」の節・直した後の本文の比べ相手・起票の PR が入るまで見どころの直しを待つ。
- **試せていない事**: スキルは開発者が打ったときだけ動き、動かせば本物のリリースになる。承認なしのマージとタグの push が自動モードの許可の判定を通るか・見どころを書く段・公開の段は、`v0.0.3` の実走が初めてになる（roadmap の「リリースノート」の節に 1 行残した）。
