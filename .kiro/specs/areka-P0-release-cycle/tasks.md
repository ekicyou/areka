# Implementation Plan

## 走らせ方（この spec だけ）

- `/kiro-impl areka-P0-release-cycle`（タスク番号なし）で始める。1 回の実行で、リリースを 1 回だけ行う。
- `/kiro-impl` の自走の形は、`design.md` の「`/kiro-impl` の読み替え」の表のとおりに読み替える。
  - サブエージェントは起こさない（0 回）。主文脈がタスクを上から順に自分で行う。
  - `[x]` は作業木に付けるだけで、コミットしない（`git add` に `tasks.md` を入れない）。
  - タスクごとのコミットはしない。「Implementation Notes」へも書かない。
  - `/kiro-validate-impl` と `/kiro-complete` は使わない。
- 各タスクの「判定」は、その場で取り直したコマンドの出力と終了コードで確かめてから `[x]` にする。
- 止まって開発者に聞く点は 2 つだけ。承認 A（マージ）と承認 B（タグの push）。承認の言葉がはっきりしないときは、行わずに聞き直す。
- 始めに `git remote` でリモートの名前だけを読み、`{remote}` とする（`-v` は付けない）。`{版}`・`{旧版}`・`{枝}` の意味と、`git` の `--quiet` と標準エラーを捨てる決まりは、`design.md` の「Components and Interfaces」の前置きに従う。
- 見出しに「（初回だけ）」と付いたタスクが 1 つでも在る回を初回（`v0.0.2`）とする。初回だけのタスクは、初回の記録の段で消して番号を詰める（ここの文は番号で指さない）。
- セッションが切れて打ち直したときは、`design.md` の「再開のときの事実の読み方」を先に読み、マージとタグの push を二度しない。
- 1 回の実行は、main から切った新しいハーネスのワークツリーの枝で始める。前の回の記録の相乗りが main に入った後に切った枝であること（古い枝で続けると、相乗りのコミットが枝の差分に出て止まる）。
- 手順は上から順に回す。並べて回せるタスク（P）は無い。

## Tasks

- [ ] 1. 前提を確かめる
- [ ] 1.1 作業木と枝を確かめ、main の最新を取り込む
  - `git status --porcelain` が 0 行であること（この spec の `tasks.md` の作業木だけのチェックは在ってよい）。
  - `git fetch --quiet {remote} main` の後、`git merge-base --is-ancestor {remote}/main HEAD` が 0 でなければ `git merge {remote}/main` で取り込む。衝突したら止まる。
  - `git diff --name-only {remote}/main...HEAD` を読む。0 件であること。
  - `git rev-parse {remote}/main` を「確かめた main」として覚える（3 の確かめは、この main を取り込んだ作業木で回す）。
  - 判定: `git merge-base --is-ancestor {remote}/main HEAD` が終了コード 0 で、枝の差分が上の条件に当たり、「確かめた main」の SHA を覚えた。欠けたら、どれが欠けたかを報告して止まる。
  - _Requirements: 1.7, 2.1, 2.6, 4.2, 4.4_

- [ ] 1.2 根の 3 つのファイルを触る開いた PR が 0 本であることを確かめる
  - `gh pr list --state open --limit 200 --json number,title,files` を読み、`files` に根の `Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md` を持つ PR を数える。
  - `files` がちょうど 100 件の PR は、`gh pr diff {番号} --name-only` で読み直す。
  - この 3 つを触らない PR が開いていても止まらない。
  - 判定: 当たる PR が 0 本。在れば、その PR の番号と題を示して止まる。
  - _Requirements: 2.2, 2.3, 2.6_

- [ ] 1.3 今の版と次の版、それぞれのタグを確かめる
  - 根の `Cargo.toml` の `[workspace.package]` の `version` と `[workspace.dependencies]` の `dola` の `version` を読み、同じ値なら `{旧版}` とする。違えば止まる。
  - `git ls-remote --quiet --tags {remote} refs/tags/v{旧版}` が 1 行であること。0 行なら前の回が「マージの後・タグの前」で止まっている。版を上げずに止まり、題が `chore(release): v{旧版}` のマージ済みの PR を `gh pr list --state merged --search "chore(release): v{旧版} in:title" --json number,mergeCommit` で示して、その squash のコミットで段 5 から続けるかを開発者に聞く。
  - 次の版を決める。指示が無ければ `{旧版}` の 3 つ目の数字に 1 を足す。指示があればその版。数字 3 つを点でつないだ形でなければ聞き直す。
  - `git ls-remote --quiet --tags {remote} refs/tags/v{版}` が 0 行であること。在れば止まる（同じ版で出し直さない）。
  - roadmap の「リリース」に、リモートのいちばん新しい `v*` のタグの行が在るかを読む。無ければ開発者へ知らせる（止まる理由にはしない）。
  - `cargo set-version --version` が終了コード 0 かを読み、2.1 のやり方（いつものやり方か代わりのやり方）を決めておく。
  - 判定: 2 行の版が一致し、`v{旧版}` が在り、`v{版}` が無く、`{版}` が数字 3 つの形である。
  - _Requirements: 1.1, 1.5, 2.6, 3.1, 3.2, 3.6, 3.7_

- [ ] 2. 版を上げる
- [ ] 2.1 版上げの道具で根の `Cargo.toml` と `Cargo.lock` の版を上げる
  - 版とタグを確かめるタスクで道具が在ると分かったら、指示が無い回は `cargo set-version --bump patch --workspace`、指示がある回は `cargo set-version {版} --workspace` を回す。`--locked` は付けない。
  - 道具が無いか、根の `Cargo.toml` の 2 行目（`dola` の行）が動かなかったときは、代わりのやり方にする。2 行を Edit で直し、`cargo update -w` で `Cargo.lock` を揃える。
  - どちらのやり方を使ったかを覚え、最後の報告に書く。
  - 判定: 根の `Cargo.toml` の 2 行がどちらも `{版}` を読む。正しく上がったかの判定は 3.2 が持つ。
  - _Requirements: 3.1, 3.2, 3.3_

- [ ] 2.2 `dist/README.txt` の「時点」の行の日付を直す
  - 冒頭の「この説明書は …… 時点の内容です。」の日付を、その日の日付に Edit で直す。
  - ファイルは BOM 付き・CRLF。その 1 行の置き換えだけにして、ほかの行と改行を動かさない。
  - 判定: `git diff -U0 -- dist/README.txt` が、足した行 1・消した行 1 の「時点」の行だけを返す（同じ日の 2 回目の版上げなら 0 行）。
  - _Requirements: 3.3, 3.4_

- [ ] 3. 手元で確かめる
- [ ] 3.1 lock の整合と全体テストを回す
  - `cargo metadata --locked --no-deps --format-version 1` が終了コード 0 であること。出力の `packages` の数を `N` として覚える（今は 30）。
  - `pwsh -NoProfile -File tools/test-all.ps1 -License` を裏で回し、終わりの知らせを待つ。`-Format` は付けない（ソースを書き換えるため）。
  - 判定: 2 つとも終了コード 0。欠けたら、版上げの PR を出さずに止まり、欠けた確かめの名前を報告する。作業木はそのまま残す。
  - _Requirements: 1.6, 2.4, 2.6, 3.6_

- [ ] 3.2 版上げの差分の範囲を判定する
  - `design.md` の段 3 の「差分の範囲の判定（c）」の表を、主文脈が `git diff --numstat HEAD` と、ファイルごとの `git diff -U0 HEAD -- {ファイル}` を読んで当てはめる。スクリプトは作らない。
  - 動いたファイルは `Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md` の 3 つが必ず在り、ほかは `dist/README.txt` とこの spec の `tasks.md` だけ。`git status --porcelain` の `??` の行は 0 行。
  - `Cargo.toml` は足した行 2・消した行 2 で、`{旧版}` が `{版}` になっただけ。`Cargo.lock` は消した行・足した行がどちらも `N` 行で、版の行だけ。`THIRD-PARTY-NOTICES.md` はワークスペースのクレートの版の行だけで、消した行と足した行の数が同じ。
  - 変更かどうかは `git diff` の中身で見て、改行だけの違いは数えない。
  - 判定: 表の全部の行が当たる。1 つでも当たらなければ、その変更を取り込まずに止まり、当たらなかった行を開発者へ示す。
  - _Requirements: 1.6, 2.6, 3.3, 3.4, 3.5, 3.6, 8.3_

- [ ] 3.3 起動の確かめ（x64）を回す
  - `pwsh -NoProfile -File tools/package.ps1 -Check` を裏で回し、終わりの知らせを待つ（窓を出す）。3.1 と同時に回さない。
  - 判定: 終了コード 0。欠けたら止まって報告する。
  - _Requirements: 2.5, 2.6_

- [ ] 3.4 公開前の確かめを回す
  - `pwsh -NoProfile -File tools/crates-io.ps1 -Verify -Version {版}` を回す。
  - 判定: 終了コード 0。赤なら版上げの PR を出さずに止まり、赤の理由を開発者へ報告する。
  - _Requirements: 3.8, 3.9_

- [ ] 4. 版上げの PR を出し、squash マージする
- [ ] 4.1 版上げをコミットし、PR を出して中身を読み直す
  - `git add Cargo.toml Cargo.lock THIRD-PARTY-NOTICES.md dist/README.txt` と名前を挙げて足す（`git add -A`・`git add .` は使わない。`tasks.md` は足さない）。題 `chore(release): v{版}` でコミットする。
  - `git push --quiet {remote} HEAD` の後、`gh pr create --base main --title "chore(release): v{版}"`。本文に版・4 ファイルの `--numstat`・3.1〜3.4 の結果を書く。
  - `gh pr view {番号} --json files` で PR のファイルを読み直す。4 ファイル（同じ日の 2 回目なら 3 ファイル）だけであること。違えば止まる。
  - 判定: PR が開いていて、ファイルの集まりが上の条件に当たる。
  - _Requirements: 4.1, 4.2, 4.4_

- [ ] 4.2 承認 A を取る
  - PR の URL・差分の数・3.1〜3.4 の結果を示し、「squash マージしてよいか」を開発者に聞いて止まる。
  - 判定: 開発者がマージを明示に承認した。はっきりしなければ聞き直し、マージしない。
  - _Requirements: 4.3_

- [ ] 4.3 main が動いていたら取り込んで確かめ直す
  - `git fetch --quiet {remote} main` の後、先端が 作業木と枝を確かめるタスクで覚えた「確かめた main」の SHA と同じなら何もしない。
  - 違えば `design.md` の段 4 の 6 に従う。`git merge {remote}/main`（衝突したら止まる）→ 3.1 を回し直す（後で作業木が `tasks.md` を除いてきれいでなければ、コミットせずに止まって報告する）→ 3.2 を比べる相手を `{remote}/main HEAD` に読み替えて回し直す → `git push --quiet {remote} HEAD` → 「確かめた main」を覚え直す。
  - 取り込みの前と後で `git diff {remote}/main HEAD -- Cargo.toml Cargo.lock THIRD-PARTY-NOTICES.md dist/README.txt` の出力が 1 字でも違えば、承認 A を取り直す。
  - 判定: `{remote}/main` の先端が「確かめた main」の SHA と同じ。
  - _Requirements: 2.4, 2.7_

- [ ] 4.4 squash マージする
  - マージの直前に main をもう一度読み、動いていたら 4.3 に戻る。
  - `gh pr merge {番号} --squash`（`--delete-branch` は付けない。枝は 7 で使う）。main へ直接 push する操作は 0 回。
  - 判定: `gh pr view {番号} --json state,mergeCommit` が `MERGED` を返す。squash のコミット `{sha}` を覚える。
  - _Requirements: 4.1_

- [ ] 5. タグを打つ
- [ ] 5.1 タグを打つコミットを確かめる
  - `git fetch --quiet {remote} main` の後、`git merge-base --is-ancestor {sha} {remote}/main` が 0 であること。
  - `git show {sha}:Cargo.toml` の 2 行の版が `{版}` と 1 字違わず同じであること。タグの名前は `v{版}`。
  - `git ls-remote --quiet --tags {remote} refs/tags/v{版}` が 0 行であることを、もう一度確かめる。
  - 判定: 3 つとも当たる。外れたら止まって報告する。
  - _Requirements: 3.7, 5.1, 5.3_

- [ ] 5.2 承認 B を取り、タグを打って push する
  - タグの名前と `{sha}` を示し、「このコミットにタグを打って push してよいか」を開発者に聞いて止まる。承認 A の答えをこの承認に使わない。
  - 承認を得たら、`git tag v{版} {sha}`（注釈なし）の後、`git push --quiet {remote} refs/tags/v{版}`。`--tags`・`--follow-tags`・`--force` は使わない（手元の `pre-rebase-5-1`・`pre-rebase-5-1b` を押し出さない）。
  - 手元にだけ同じ名前のタグが在るときは `design.md` の「再開のときの事実の読み方」に従う。
  - 押したタグは、この先どの段でも動かさず、消さない。
  - 判定: `git ls-remote --quiet --tags {remote} refs/tags/v{版}` が `{sha}` を返す。タグを押した日を覚える。
  - _Requirements: 1.5, 1.7, 5.1, 5.2, 5.4_

- [ ] 6. 見守る
- [ ] 6.1 2 本の走りを見つけて裏で待つ
  - `gh run list --workflow release.yml --event push --json databaseId,headSha,headBranch,status,conclusion` から、`headBranch` が `v{版}` で `headSha` が `{sha}` の走りを取る。`crates-io.yml` も同じ。数分待っても見つからなければ報告する。
  - 走りごとに `gh run watch {id} --exit-status --interval 60` を裏で回す。裏の待ちが上限で切れたら掛け直す。
  - 判定: 2 本の走りの番号が取れ、どちらも終わった（`gh run view {id} --json status` が `completed`）。
  - _Requirements: 6.1, 6.2_

- [ ] 6.2 緑を判定する
  - `design.md` の段 6 の「緑の判定」の表を当てはめる。Release の走りが `success`、`gh release view v{版} --json isDraft,assets,url` で `isDraft` が `false` かつ添わった物がちょうど 4 つ（x64 と arm64 の zip とそれぞれの `.sha256`）。
  - 公開の走りが `success`、`pwsh -NoProfile -File tools/crates-io.ps1 -Pending -Version {版}` が終了コード 0 で標準出力 0 行（作業木の版は `{版}` のまま）。
  - `git cat-file -e {sha}:.github/workflows/winget.yml` が 0 のときだけ、`gh pr list --repo microsoft/winget-pkgs` でその版の PR を探す。無い回は winget を見守らない。
  - 判定: 表の全部の行が緑。Release の URL（と在れば winget の PR の URL）を覚える。赤の行が在れば 6.3 へ。
  - _Requirements: 6.1, 6.2, 6.3, 8.7_

- [ ] 6.3 赤のときは決まりを当てはめ、開発者に決めてもらう
  - `design.md` の段 6 の「赤のときの決まり」の表に、見えた事実を当てはめて示し、止まる。やり直すかどうかは開発者が決める。自分では決めない。
  - Release の走りの Re-run は、その版の Release が残っておらず（下書きも無い）、原因が通信の失敗・時間切れ・取り消しのときだけ。公開の走りの赤は `doc/crates-io-publish.md` 5 節の切り分けに従う。公開の走りだけが赤なら Rust の版の差を先に疑う。
  - タグのコミットの直しが要る赤や、公開より後の段の赤は、同じ版で出し直さず、タグを動かさない。開発者の判断を得て、主文脈がこの作業の枝で `/kiro-discovery` を回して起票し、そのコミットを別のコミットとして置く（7.1 より前。roadmap の行は、起票のコミットの後に足す）。
  - 判定: 赤が 0 件、または赤のすべてに開発者の判断があり、やり直しの結果が緑か、起票した spec の名前を覚えた。赤が無い回は何もしない。
  - _Requirements: 6.4, 6.5, 6.6, 6.7, 6.8_

- [ ] 7. 記録して受け渡す
- [ ] 7.1 作業の枝に main を取り込む
  - `git fetch --quiet {remote} main` の後、`git merge {remote}/main`。
  - 判定: `git merge-base --is-ancestor {remote}/main HEAD` が 0 で、`git diff {remote}/main HEAD -- Cargo.toml Cargo.lock THIRD-PARTY-NOTICES.md dist/README.txt` が 0 行。
  - _Requirements: 7.3_

- [ ] 7.2 roadmap の「リリース」に 1 行を足す
  - 表の末尾に、版・タグを押した日・GitHub Release の URL・winget-pkgs の PR の URL・赤で起票した spec の名前の 5 列で 1 行を足す。無い物は「—」。Release が公開まで進まなかった回は Release の欄に「公開なし」と書き、起票した spec の名前を必ず書く。途中の行は動かさない。
  - 判定: 表の最後の行が今回の `v{版}` で、5 列すべてが埋まっている。
  - _Requirements: 7.1, 7.2_

- [ ] 7.3 記録をコミットして push し、相乗りの頼みを渡す
  - `git add` の直前に、作業木の `tasks.md` の `[x]` を全部 `[ ]` に戻す（このタスク自身にも、以後 `[x]` を付けない）。
  - 名前を挙げて `git add` する。`.kiro/steering/roadmap.md` だけ。題 `docs(release): v{版} の記録` でコミットし、`git push --quiet {remote} HEAD`。
  - 記録だけの PR は出さず、`/kiro-complete` も使わない。`design.md` の段 7 の 6 の頼みの 1 文を、枝・記録のコミット（起票があればそのコミットも）・記録の 1 行を埋めて開発者へ渡す。
  - この spec のフォルダは `.kiro/specs/` の直下に置いたまま、`spec.json` の `phase` は `implementation` のまま動かさない。
  - 判定: `git status --porcelain` が 0 行、`git show --name-only --format= HEAD` が上の名前だけを返し、`git ls-remote --quiet --heads {remote} {枝}` が記録のコミットを返す。`git show HEAD:.kiro/specs/areka-P0-release-cycle/tasks.md` に `[x]` が 0 個。
  - _Requirements: 1.3, 1.4, 1.8, 7.3, 7.4_
