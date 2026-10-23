# Requirements Document

> **この文書は完了した spec の記録で、実行には使わない**（2026-10-11）。リリースの手順の正本は `.claude/skills/release/`（`SKILL.md` と `procedure.md`）で、開発者が `/release` を打って回す。本書は `v0.0.2` を出した繰り返し spec の時点の文で、末尾の「手順をスキル `/release` へ移した」の節が、今の形と食い違う行を名指しする。

## Project Description (Input)
リリースの時期は開発者が決めたい。PR のたびに版が上がる形は採らない。「この spec の実装を打ったときだけ」版が +0.0.1 され、タグが押され、GitHub Actions がビルド・Release・crates.io（後に winget）まで運ぶ、という繰り返しの手順が要る（2026-10-02 `/kiro-discovery`「配布と公開」で起票）。今は、版は根の `Cargo.toml` にだけ在り、`/kiro-complete` は版にもタグにも触らず、main へは PR でしか入れない。タグ `v*` の push で動く `release.yml`（`release-ci-workflow`）と `crates-io.yml`（`crates-io-publish`）がこの手順の受け手として main に揃っている。実行 1 回＝リリース 1 回で、手順（タスク）は毎回同じ。終わったらタスクを未完了に戻す。初回は `v0.0.2`。詳細は brief.md（2026-10-05 棚卸㉒の再測定の節を含む）。

## Introduction

本 spec は、開発者が `/release`（入口のスキル。`/kiro-impl areka-P0-release-cycle` と打っても同じ）を打ったときだけ 1 回のリリースを行う、**繰り返し spec** である。`completed/` へは移さず、実行のたびに `tasks.md` のチェックを戻して次の回に備える（`.kiro/steering/workflow.md` と `kiro-complete` の「繰り返し仕様の例外」）。

1 回の実行は次の流れになる。

1. 前提を確かめる。
2. 版を上げ（指示が無ければ +0.0.1）、その版の見どころを書く。
3. 版上げだけの PR を出し、squash マージする（初回だけ、本 spec の文書も同じ PR に載せる）。
4. その squash のコミットに `v{版}` のタグを打って push する。これが GitHub Actions のきっかけになる。
5. Release を作る走りと crates.io へ出す走りを見守り、赤なら決まりに従う。
6. 結果を `roadmap.md` に 1 行で記録する。

初回（`v0.0.2`）だけ、タグの前に開発者が crates.io で Trusted Publishing を設定するなど、追加の手順が乗る。

### 本書で使う言葉

- **版**: ワークスペースで 1 つの版。正本は根の `Cargo.toml` の `[workspace.package]` の `version`。根の `[workspace.dependencies]` の `dola` の `version` も同じ値を持つ（`crates-io-publish` の設計で、版上げで動くのはこの 2 行だけにした）。
- **タグ**: `v` と版を続けた名前（例 `v0.0.2`）。版を写したもので、正本ではない。
- **版上げの枝・版上げの PR**: 版を上げる変更だけを載せた作業の枝と、それを main へ入れる PR。
- **Release の走り**: タグの push で動き、組み立て・zip・SHA256 と、詳しいリリースノート・Release の本文を作って GitHub Release を公開する `release.yml` の走り。
- **公開の走り**: 同じタグの push を自分で受け、Release の走りの緑を待ってから `wintf`・`dola` を crates.io へ出す `crates-io.yml` の走り。
- **乾いた走り**: `release.yml` を手で起動し、Release を作らずに組み立てまでを確かめる走り。
- **全体テスト**: 手元の `tools/test-all.ps1`（整形とライセンスの確かめを含む形。整形は確かめるだけで、ソースを書き換えない）。
- **起動の確かめ**: 配布スクリプト `tools/package.ps1 -Check`（x64）。窓を出すので GitHub Actions では回せず、手元で回す。
- **公開前の確かめ**: `tools/crates-io.ps1 -Verify`（何も上げずに、組み立てまで確かめる形）。
- **見どころのファイル**: その版で特に注目に値する内容を日本語で紹介する Markdown のファイル。版ごとに 1 つ、`doc/release-notes/v{版}.md` に置く。Release の本文のもとになる。
- **詳しいリリースノート**: 一つ前のタグからその版までに main へ入ったすべての変更を、1 変更 1 行で並べた Markdown のファイル（`release-notes-v{版}.md`）。Release の走りが作って Release に添える。同じ形の一覧を、`tools/release-notes.ps1 -Notes` で手元でも出せる。
- **見どころの検査**: `tools/release-notes.ps1 -Check`（その版の見どころのファイルが決まりの形を満たすかを見る。読むだけ）。

## Boundary Context

- **In scope**:
  - 繰り返しの手順そのもの（毎回同じ `tasks.md` の形と、戻し方）。
  - 版の上げ方（使う道具は設計で決める）と、版とタグの決まり。
  - 版上げで書き換えるファイル: 根の `Cargo.toml` の 2 行・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`（作り直し）・`dist/README.txt` の「時点」の行・その版の見どころのファイル（新しく足す 1 つ）。
  - 見守りと、赤のときの決まり。
  - 記録の形（`roadmap.md` の 1 行）。
  - 初回だけ、根の `README.md` の「まだ GitHub Releases での配布はしていない」の行の直し（Release の公開を確かめた後・記録の 1 行と同じ PR）。
  - 初回（`v0.0.2`）の実行と、初回だけの手順。
- **Out of scope**:
  - workflow の中身（`release.yml`＝`release-ci-workflow`・`crates-io.yml`＝`crates-io-publish`・`winget.yml`＝`winget-release-automation`）。本 spec は workflow のファイルに触らない。
  - winget のマニフェストの形と初回の手提出（`winget-manifest-submission`）。
  - 配布スクリプトの中身（`release-package-versioned`）と、公開前の確かめのスクリプトの中身（`crates-io-publish`）。
  - 大きい版（0.1.0・1.0.0 など）をいつ上げるかの判断。開発者がその場で指示する。本 spec は指示があればその版にするだけ。
  - ソースコードの変更。赤の原因の修正は、本 spec では行わず、別の spec として起票する。
- **Adjacent expectations**:
  - `release.yml` は、タグの版と `Cargo.toml` の版を比べ、違えば何も作らずに止まる。Rust の版は `1.99.0` に固定されている。
  - `crates-io.yml` は、同じタグの Release の走りが緑で終わるのを最長 120 分待ってから出す。Rust は最新の安定版を使う（Release の走りと Rust の版が違うので、公開の走りだけが赤なら版の差を先に疑う）。同じ版で何度起動し直してもよく、既に出たクレートは飛ばす（`doc/crates-io-publish.md` 5 節）。
  - Trusted Publishing の設定の値と手順は `doc/crates-io-publish.md` 2 節が正本。本 spec は写さず、そこを指す。
  - 見どころの書き方の決まり（形・口調・必ず載せるもの）は `doc/release-notes/README.md` が正本。本 spec は写さず、そこを指す。変更の一覧と見どころの検査には `tools/release-notes.ps1` を呼ぶだけで、その中身は書き換えない。`release.yml` は同じ検査を組み立ての前に回し、その版の見どころのファイルが欠けていれば何も公開せずに止まる。
  - `release-ci-workflow` は、初回の実走で初めて動く 6 項目を本 spec へ申し送った（`.kiro/specs/completed/areka-P0-release-ci-workflow/verification/runner-trial.md` の「release-cycle への申し送り」）。本 spec は初回にそれを見守る。完了の条件にはしない。
  - `winget-release-automation` が `winget.yml` を main へ入れた後の回から、見守る相手に winget-pkgs への PR が加わる。手順の形は変わらない。
  - `roadmap.md` の「完了サマリ」は棚卸（`/kiro-discovery` の再入）と分け合う場所で、本 spec はそこへリリースの 1 行を足すだけ。

## Requirements

### Requirement 1: 繰り返しの形

**Objective:** As a 開発者, I want リリースを打ちたいときに同じ手順を 1 回ずつ回せる, so that リリースの時期を自分で選べ、PR のたびに版が上がることがない

#### Acceptance Criteria

1. When 開発者が本 spec の実装（`/release`、または `/kiro-impl areka-P0-release-cycle`）を打つ, the リリースの手順 shall 1 回の実行で 1 回のリリース（版を 1 つ上げ、タグを 1 つ打つ）だけを行う。
2. The リリースの手順 shall 2 回目以降、毎回同じタスクの並びで回る（初回だけの手順は要件 8 に分け、2 回目以降のタスクには残さない）。
3. When 1 回の実行が終わる（要件 7 の記録の 1 行を書き終える）, the リリースの手順 shall main の `tasks.md` をチェックがすべて未完了の状態（次の回にそのまま使える形）に置き、本 spec を `.kiro/specs/` の直下に置いたままにする（`completed/` へ移さない）。
4. The 本 spec の `spec.json` shall 初回の実行の後も実装中の段のまま置き、完了の段へ進めない。
5. The リリースの手順 shall 版を上げ、タグを打つ唯一の経路である（ほかの spec の PR や `/kiro-complete` は版にもタグにも触らない）。
6. The リリースの手順 shall ソースコード（各クレートのコード・`tools/` のスクリプト・workflow のファイル）を書き換えない。書き換えるのは要件 3.3 のファイルと、本 spec のフォルダと、要件 7 の記録の 1 行と、初回だけ根の `README.md` の 1 行（要件 8.8・記録の 1 行と同じ PR）だけ。
7. The リリースの手順 shall 接続先の URL や認証の情報を画面にもログにも印字しない（`git remote -v` などを手順に入れない。してはいけない操作の一覧は `doc/crates-io-publish.md` 7 節に従う）。
8. The 本 spec shall `/kiro-complete` を使わない。main への入り方（版上げの PR・記録の 1 行の相乗り）は本 spec の要件 4・7 と、それに従う設計・タスクだけで決める（例外は本 spec だけなので、決まりは本 spec の中に置き、ほかの決まりの文書は書き換えない・2026-10-05 開発者の裁定）。

### Requirement 2: 前提の確かめ

**Objective:** As a 開発者, I want 版を上げる前と、タグを打つ前に、出してよい状態かを確かめたい, so that 手元で確かめていない中身や、組み立てられない中身にタグを打たない

#### Acceptance Criteria

1. When 1 回の実行を始める, the リリースの手順 shall 版上げの枝が main の最新を取り込んでいることを確かめる。
2. When 1 回の実行を始める, the リリースの手順 shall GitHub の開いている PR のうち、`Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md` のどれかを触るものが 0 本であることを確かめる（2026-10-05 棚卸㉒で「開いている PR 0 本」から改めた）。
3. While この 3 つのファイルを触らない PR が開いている, the リリースの手順 shall それを理由に止まらない（それらは squash の 1 PR で入るので、タグを打つコミットの中身に途中の物は入らない）。
4. When 版を上げ終えた（要件 3）, the リリースの手順 shall 版上げの枝の中身で全体テストを回し、終了コード 0 を確かめる。
5. When 版を上げ終えた, the リリースの手順 shall 版上げの枝の中身で起動の確かめ（x64）を手元で回し、緑を確かめる。
6. If 要件 2.1・2.2・2.4・2.5 のどれかが欠ける, then the リリースの手順 shall 版上げの PR を出す前に止まり、どの確かめが欠けたかを開発者へ報告する。
7. If 全体テストを回した後、版上げの PR をマージするまでに main へ別の PR が入った, then the リリースの手順 shall main を取り込み、全体テストを回し直して終了コード 0 を確かめてから、マージへ進む。

### Requirement 3: 版の上げ方

**Objective:** As a 開発者, I want 版を決まった数か所だけ、決まった値に上げたい, so that 版が食い違ったまま出たり、関係のない行が動いたりしない

#### Acceptance Criteria

1. While 開発者から版の指示が無い, the リリースの手順 shall 版を patch で 1 つ上げる（例 `0.0.1` → `0.0.2`）。
2. Where 開発者がその回の版を指示した, the リリースの手順 shall 指示された版にする。
3. The リリースの手順 shall 版上げで次だけを書き換える: 根の `Cargo.toml` の 2 行（`[workspace.package]` の `version`・`[workspace.dependencies]` の `dola` の `version`）、`Cargo.lock` のワークスペースのクレートの版の行、`THIRD-PARTY-NOTICES.md`（全体テストで作り直した物）、`dist/README.txt` の冒頭の「この説明書は …… 時点の内容です。」の日付（その日の日付にする）、その版の見どころのファイル `doc/release-notes/v{版}.md`（新しいファイル 1 つ。その版の分だけ）。初回だけ直す根の `README.md` の 1 行は版上げに含めず、記録の 1 行と同じ PR に載せる（要件 8.8）。
4. The リリースの手順 shall 各クレートの `Cargo.toml`・外部のクレートの版・`dist/README.txt` の「時点」の行以外の行を書き換えない。新しく足すファイルは、その版の見どころのファイルの 1 つだけで、ほかの版の見どころのファイルは書き換えない。
5. If 版上げの差分に要件 3.3 の外の変更（`Cargo.lock` の外部のクレートの行を含む）が現れる, then the リリースの手順 shall その変更を取り込まずに止まり、開発者へ報告する（変更かどうかは中身の差分で判断し、改行の違いだけのものは変更と数えない）。
6. If 根の `Cargo.toml` の 2 行の版が一致しない, then the リリースの手順 shall 版上げの PR を出さずに止まる。
7. If 上げた後の版のタグ `v{版}` が既に在る, then the リリースの手順 shall 版上げの PR を出さずに止まり、開発者へ報告する（同じ版で出し直さない）。
8. When 版を上げ終えた, the リリースの手順 shall 版上げの枝の中身で公開前の確かめを通し、緑を確かめる。
9. If 公開前の確かめが赤, then the リリースの手順 shall 版上げの PR を出さずに止まり、赤の理由を開発者へ報告する。
10. When 版を上げ終えた, the リリースの手順 shall 一つ前のタグから版上げの枝の先頭までの変更の一覧を手元で出し（詳しいリリースノートと同じ形）、その中から見どころを選んで、その版の見どころのファイルを `doc/release-notes/README.md` の決まりに従って書く。
11. When 見どころのファイルを書き終えた, the リリースの手順 shall 版上げの PR を出す前に見どころの検査を回し、赤なら版上げの PR を出さずに止まって、欠けたものを開発者へ報告する。

### Requirement 4: 版上げの PR とマージ

**Objective:** As a 開発者, I want 版上げを main へ PR で入れ、マージは自分の承認で行いたい, so that main へ直接書かず、リリースの中身を自分で確かめてから出せる

#### Acceptance Criteria

1. The リリースの手順 shall 版上げを PR で main へ入れ、squash マージする。main へ直接 push しない。
2. The 版上げの PR shall 2 回目以降、版上げの変更（要件 3.3）だけを載せ、ほかの変更を混ぜない。
3. The リリースの手順 shall 版上げの PR の squash マージを、開発者の明示の承認を得てから行う。
4. Where その回が初回（`v0.0.2`）である, the 版上げの PR shall 版上げの変更（要件 3.3）と本 spec の文書（要件〜タスク）を 1 本の PR に載せ、それ以外の変更を混ぜない（2026-10-05 開発者の裁定）。

### Requirement 5: タグ

**Objective:** As a 開発者, I want `Cargo.toml` の版を写したタグを、自分の承認で 1 つだけ打ちたい, so that GitHub Actions が正しい版の中身からリリースを作る

#### Acceptance Criteria

1. When 版上げの PR が squash マージされた, the リリースの手順 shall main のその squash のコミットに `v{版}` のタグを打って push する（それより後に main へ入ったコミットには打たない）。
2. The リリースの手順 shall タグを、開発者の明示の承認を得てから push する。workflow はタグを打たない。
3. The タグの名前 shall `v` と、そのコミットの根の `Cargo.toml` の版を 1 字違わず続けたものにする。
4. The リリースの手順 shall push したタグを動かさず、消さない（赤の後も同じ）。

### Requirement 6: 見守りと、赤のときの決まり

**Objective:** As a 開発者, I want タグの後の走りを見守り、赤のときに何をしてよいかが決まっている, so that 一度配った版を差し替えたり、同じ版で出し直したりしない

#### Acceptance Criteria

1. When タグを push した, the リリースの手順 shall Release の走りが緑で終わること、その版の GitHub Release が下書きでない公開の状態で在ること、5 つのファイル（x64 と arm64 の zip・それぞれの SHA256・詳しいリリースノート）が添わっていること、本文がその版の見どころであることを確かめる。
2. When タグを push した, the リリースの手順 shall 公開の走りが緑で終わること、`wintf`・`dola` がその版で crates.io に在ること（`tools/crates-io.ps1 -Pending` の残りが空）を確かめる。
3. Where タグのコミットに `winget.yml` が在る, the リリースの手順 shall winget-pkgs へその版の PR が出たことも確かめる。
4. If Release の走りが赤で、その版の Release が残っておらず、原因がタグを動かさずに済むもの（通信の失敗・時間切れ・取り消し）である, then the リリースの手順 shall 開発者の判断で、同じ走りを「Re-run」でやり直してよい。
5. If Release の走りが赤で、原因がタグのコミットの直しを要する, then the リリースの手順 shall 同じ版で出し直さず、タグを動かさず、原因を直す spec を `/kiro-discovery` で起票し、直したら次の版で出す。
6. If 公開の走りが赤, then the リリースの手順 shall `doc/crates-io-publish.md` 5 節の切り分けに従う（同じ版のまま「Re-run」か「Run workflow」で残りを出してよい。次の版で出し直すのは、タグのコミットのコードや公開前の確かめのスクリプトの誤りのときだけで、そのときは原因を直す spec を起票する）。
7. If Release の公開より後の段（winget など、公開の走りを除く）が赤, then the リリースの手順 shall 同じ版で出し直さず、原因を直す spec を起票する。
8. The リリースの手順 shall 赤の走りをやり直すかどうかを開発者に決めてもらい、自分では決めない。

### Requirement 7: 記録

**Objective:** As a 開発者, I want 出したリリースを roadmap で一目で辿りたい, so that どの版をいつ出し、どこに置いたかが分かる

#### Acceptance Criteria

1. When 見守りが終わる（すべて緑、または赤の原因を直す spec の起票まで済む）, the リリースの手順 shall `.kiro/steering/roadmap.md` の「完了サマリ」の下の「リリース」に 1 行を足す（「リリース」の小見出しが無ければ初回に作る）。
2. The 記録の 1 行 shall 版・日付・GitHub Release の URL・winget-pkgs への PR の URL（在る回だけ）・赤の原因を直すために起票した spec の名前（在る回だけ）を含む。
3. The リリースの手順 shall 記録を PR で main へ入れる（main へ直接 push しない）。
4. The リリースの手順 shall 記録の 1 行と、main の `tasks.md` に要るなら、そのチェックの戻し（要件 1.3）を、次に main へ入る spec か棚卸の PR に相乗りさせ、記録だけの PR は出さない（2026-10-05 開発者の裁定）。

### Requirement 8: 初回（`v0.0.2`）だけの手順

**Objective:** As a 開発者, I want 初めての自動のリリースで、事前に要る設定と、初めて動く物の確かめを漏らさない, so that 最初の公開が設定の漏れで止まらず、初めて動いた物の赤を見落とさない

#### Acceptance Criteria

1. When 初回のタグを打つ前, the リリースの手順 shall 開発者に、crates.io で `wintf`・`dola` のそれぞれへ Trusted Publishing を設定してもらう（値と手順は `doc/crates-io-publish.md` 2 節）。
2. If 開発者から Trusted Publishing の設定を済ませたと伝えられていない, then the リリースの手順 shall 初回のタグを push しない。
3. When 初回の版を上げる, the リリースの手順 shall 版上げの道具が根の `Cargo.toml` の 2 行の両方を動かしたことを差分で確かめ、その結果を本 spec に記録する（`dola` の行の動かし忘れは組み立ての失敗で必ず気付けるが、道具の振る舞いとして確かめておく）。
4. When 初回のタグを打つ前, the リリースの手順 shall main で `release.yml` を手で起動する乾いた走りを 1 回行い、緑を確かめる（git から取り込む依存を足した後の main では、まだ一度も走っていない。roadmap の「勧め」なので、開発者が省くと決めたときは飛ばす）。
5. When 初回のタグを push した, the リリースの手順 shall `release-ci-workflow` が申し送った初回だけの 6 項目（Release の公開の段・後始末の段の消す経路・下書きの Release の見え方・赤の走りの「Re-run」・マージ後の乾いた走り・「タグで始めた」枝）を見守り、結果を本 spec に記録する。
6. If 初回だけの 6 項目のどれかが赤, then the リリースの手順 shall それを直す spec を `/kiro-discovery` で起票する（どれも 1 回の実行の完了の条件にはしない）。
7. While `winget.yml` が main に無い, the リリースの手順 shall 初回の見守りの相手に winget を入れない（winget の初回の手提出は `winget-manifest-submission` の番）。
8. When 初回の Release が公開の状態で在ることを確かめた（要件 6.1）, the リリースの手順 shall 根の `README.md` の「## 入手と起動」の最初の箇条にある「まだ GitHub Releases での配布はしていない」の行を、GitHub Releases から入手できる事実に合わせて直し、その直しを記録の 1 行と同じ PR に載せる（公開を確かめる前には直さない・2026-10-05 開発者の裁定）。

## 決まったこと（brief の議題の行き先）

- **初回の PR の分け方**（brief の議題 1・2026-10-05 開発者「どちらも推しで」）→ 初回だけ、本 spec の文書と版上げを 1 本の PR に入れる（要件 4.4）。記録の 1 行と `tasks.md` のチェックの戻しは、次の spec か棚卸の PR に相乗りさせ、記録だけの PR は出さない（要件 7.4）。
- **`README.md` の「まだ GitHub Releases での配布はしていない」の行**（brief の議題 2・同じ裁定）→ 初回の本 spec が直す。Release の公開を確かめてから、記録の 1 行と同じ PR で直す（要件 1.6・8.8）。
- **`/kiro-complete` との関係**（要件ディスカッション 議題 1・2026-10-05 開発者「この spec だけが例外なんだから、要件・設計・タスクがそれに従っていればいい。kiro-complete は繰り返し spec では使わない」）→ 本 spec は `/kiro-complete` を使わず、main への入り方は本 spec の要件・設計・タスクで決める。`workflow.md` や `kiro-complete` の文は直さない（要件 1.8）。
- **版上げの道具**（brief の議題 3）→ 設計で決める（要件は変わらない）。brief の棚卸㉒の節は、手元の cargo-edit 0.13.13 のソースを読み、`cargo set-version --bump patch --workspace` が `[workspace.dependencies]` の `dola` の行も動かす見込みとした。実物は要件 8.3 で確かめる。
- **開いている PR の条件**（brief の議題 4）→ 2026-10-05 棚卸㉒の裁定 8 で「`Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md` を触る開いた PR 0 本」に改めた（要件 2.2・2.3）。
- **`THIRD-PARTY-NOTICES.md` の作り直し**（brief の議題 5）→ 作り直す。`.kiro/steering/workflow.md` の「`Cargo.lock` の扱い」が「lock を揃えたら謝辞も作り直してコミットに含める」と決めており、全体テストの `-License` が作り直す（要件 3.3）。
- **見どころを書く段**（2026-10-11 に `areka-P0-release-notes-highlights` が足した）→ 版上げの PR に、その版の見どころのファイルを 1 つ足し、PR を出す前に見どころの検査を通す（要件 3.3・3.4・3.10・3.11）。公開の後の確かめは、5 つのファイルと本文に改めた（要件 6.1）。書き方の決まり（`doc/release-notes/README.md`）と道具（`tools/release-notes.ps1`）は同 spec が main に入れた物で、本 spec はそれを使うだけで書き換えない（要件 1.6）。

## 2026-10-11 手順をスキル `/release` へ移した（最後の仕事）

開発者の決定（2026-10-11）:「外から見た動きは pasta と同じく承認なしでノンストップ。`areka-P0-release-cycle` を完了にして、`/release` を正にするのが目的」。本 spec の最後の仕事として、手順をスキルの中へ写し、本 spec を普通の完了の手順で `completed/` へ移した。上の要件は書き直さず、今の形と食い違う行をここに挙げる。今の決まりは `.claude/skills/release/procedure.md` が正本である。

| 要件 | もとの決まり | 今の形 |
|---|---|---|
| 1.3・1.4 | `tasks.md` を未完了に保ち、`spec.json` を実装中のまま置き、`completed/` へ移さない | 本 spec は完了した。手順はスキルの中に在り、進み具合はどのファイルにも書かない（再開の位置は PR・タグ・Release・走りの事実から決める） |
| 1.8 | `/kiro-complete` を使わず、共通の文書を直さない | リリースの中で `/kiro-complete` を使わないのは同じ。本 spec 自身は `/kiro-complete` で完了にした。`workflow.md` と `kiro-complete` の「繰り返し仕様の例外」は、仕組みを残して、例に挙げた「リリース手順」の語だけを外した |
| 4.3 | 版上げの PR の squash マージは、開発者の明示の承認の後 | 承認を取らない。段 1〜4 の判定が全部当たったときだけ、聞かずにマージする |
| 5.2 | タグの push は、開発者の明示の承認の後 | 承認を取らない。段 5 の判定が全部当たったときだけ、聞かずに押す |
| 2.7 に足した止まり | 承認の後に main が動いたら、新しく入った題名を示して止まる | 止まらない。取り込んだら変更の一覧を出し直し、「必ず載せるもの」に当たる変更を手順を回す側が見どころへ足して、検査を回し直す |
| 7.1〜7.4 | roadmap の「リリース」の表に 1 行を足し、次の PR に相乗りさせる | やめた。記録は GitHub の Release の頁。表の今ある行は歴史として残し、`v0.0.3` からは足さない |
| 6.8 | 走りが赤のとき、やり直すかは開発者が決める | 変わらない（止まって聞く） |
| 8.1〜8.8 | 初回（`v0.0.2`）だけの手順 | 済んだ。スキルには写していない |

開発者が決めた事（2026-10-11・5 点）:

1. **見どころを読む場**: 手順の最後の報告に全文を載せる。開発者はそこで読み、直しが要れば公開の後に頼む（公開の頁の本文と main のファイルを直す。タグのコミットの中のファイルは直せない）。
2. **PR を出した後に main が動いたとき**: 上の表の 2.7 の行のとおり、止まらない。
3. **roadmap への記録**: やめる。
4. **走りが赤のとき**: 今までどおり止まって聞く。
5. **次の版への申し送りの置き場**: roadmap の節「次のリリースへの申し送り」。手順は段 1 でそこを読む。版上げの 5 つのファイルの外を触る仕事は、そこへ書かずに、リリースの前に別の PR で片づける。

完了済みの `areka-P0-release-notes-highlights` の要件 6.4（マージの承認の前に見どころを読める）・6.7（リリースの手順を繰り返しの形のまま保つ）・6.9（入口のスキルは手順を写さない）と、同 spec の設計の「新しく入った題名を示して止まる」は、この仕事で上書きした。同 spec の文書は記録なので書き換えていない。

本 spec に宛てていた申し送りのうち、`dola` の `CueCommand` の件は roadmap の「次のリリースへの申し送り」へ移した。`deny.toml`・`about.toml` の `BSD-2-Clause` の掃除は、この仕事の PR で片づけた。
