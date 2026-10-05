# Requirements Document

## Project Description (Input)
リリースの時期は開発者が決めたい。PR のたびに版が上がる形は採らない。「この spec の実装を打ったときだけ」版が +0.0.1 され、タグが押され、GitHub Actions がビルド・Release・crates.io（後に winget）まで運ぶ、という繰り返しの手順が要る（2026-10-02 `/kiro-discovery`「配布と公開」で起票）。今は、版は根の `Cargo.toml` にだけ在り、`/kiro-complete` は版にもタグにも触らず、main へは PR でしか入れない。タグ `v*` の push で動く `release.yml`（`release-ci-workflow`）と `crates-io.yml`（`crates-io-publish`）がこの手順の受け手として main に揃っている。実行 1 回＝リリース 1 回で、手順（タスク）は毎回同じ。終わったらタスクを未完了に戻す。初回は `v0.0.2`。詳細は brief.md（2026-10-05 棚卸㉒の再測定の節を含む）。

## Introduction

本 spec は、開発者が `/kiro-impl areka-P0-release-cycle` を打ったときだけ 1 回のリリースを行う、**繰り返し spec** である。`completed/` へは移さず、実行のたびに `tasks.md` のチェックを戻して次の回に備える（`.kiro/steering/workflow.md` と `kiro-complete` の「繰り返し仕様の例外」）。

1 回の実行は次の流れになる。

1. 前提を確かめる。
2. 版を上げる（指示が無ければ +0.0.1）。
3. 版上げだけの PR を出し、squash マージする（初回だけ、本 spec の文書も同じ PR に載せる）。
4. その squash のコミットに `v{版}` のタグを打って push する。これが GitHub Actions のきっかけになる。
5. Release を作る走りと crates.io へ出す走りを見守り、赤なら決まりに従う。
6. 結果を `roadmap.md` に 1 行で記録する。

初回（`v0.0.2`）だけ、タグの前に開発者が crates.io で Trusted Publishing を設定するなど、追加の手順が乗る。

### 本書で使う言葉

- **版**: ワークスペースで 1 つの版。正本は根の `Cargo.toml` の `[workspace.package]` の `version`。根の `[workspace.dependencies]` の `dola` の `version` も同じ値を持つ（`crates-io-publish` の設計で、版上げで動くのはこの 2 行だけにした）。
- **タグ**: `v` と版を続けた名前（例 `v0.0.2`）。版を写したもので、正本ではない。
- **版上げの枝・版上げの PR**: 版を上げる変更だけを載せた作業の枝と、それを main へ入れる PR。
- **Release の走り**: タグの push で動き、組み立て・zip・SHA256 を作って GitHub Release を公開する `release.yml` の走り。
- **公開の走り**: 同じタグの push を自分で受け、Release の走りの緑を待ってから `wintf`・`dola` を crates.io へ出す `crates-io.yml` の走り。
- **乾いた走り**: `release.yml` を手で起動し、Release を作らずに組み立てまでを確かめる走り。
- **全体テスト**: 手元の `tools/test-all.ps1`（整形とライセンスの確かめを含む形）。
- **起動の確かめ**: 配布スクリプト `tools/package.ps1 -Check`（x64）。窓を出すので GitHub Actions では回せず、手元で回す。
- **公開前の確かめ**: `tools/crates-io.ps1 -Verify`（何も上げずに、組み立てまで確かめる形）。

## Boundary Context

- **In scope**:
  - 繰り返しの手順そのもの（毎回同じ `tasks.md` の形と、戻し方）。
  - 版の上げ方（使う道具は設計で決める）と、版とタグの決まり。
  - 版上げで書き換えるファイル: 根の `Cargo.toml` の 2 行・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`（作り直し）・`dist/README.txt` の「時点」の行。
  - 見守りと、赤のときの決まり。
  - 記録の形（`roadmap.md` の 1 行）。
  - 初回だけ、根の `README.md` の「まだ GitHub Releases での配布はしていない」の行の直し（Release の公開を確かめた後・記録の 1 行と同じ PR）。
  - 初回（`v0.0.2`）の実行と、初回だけの手順。
- **Out of scope**:
  - workflow の中身（`release.yml`＝`release-ci-workflow`・`crates-io.yml`＝`crates-io-publish`・`winget.yml`＝`winget-manifest-submission`）。本 spec は workflow のファイルに触らない。
  - winget のマニフェストの形と初回の手提出（`winget-manifest-submission`）。
  - 配布スクリプトの中身（`release-package-versioned`）と、公開前の確かめのスクリプトの中身（`crates-io-publish`）。
  - 大きい版（0.1.0・1.0.0 など）をいつ上げるかの判断。開発者がその場で指示する。本 spec は指示があればその版にするだけ。
  - ソースコードの変更。赤の原因の修正は、本 spec では行わず、別の spec として起票する。
- **Adjacent expectations**:
  - `release.yml` は、タグの版と `Cargo.toml` の版を比べ、違えば何も作らずに止まる。Rust の版は `1.99.0` に固定されている。
  - `crates-io.yml` は、同じタグの Release の走りが緑で終わるのを最長 120 分待ってから出す。Rust は最新の安定版を使う（Release の走りと Rust の版が違うので、公開の走りだけが赤なら版の差を先に疑う）。同じ版で何度起動し直してもよく、既に出たクレートは飛ばす（`doc/crates-io-publish.md` 5 節）。
  - Trusted Publishing の設定の値と手順は `doc/crates-io-publish.md` 2 節が正本。本 spec は写さず、そこを指す。
  - `release-ci-workflow` は、初回の実走で初めて動く 6 項目を本 spec へ申し送った（`.kiro/specs/completed/areka-P0-release-ci-workflow/verification/runner-trial.md` の「release-cycle への申し送り」）。本 spec は初回にそれを見守る。完了の条件にはしない。
  - `winget-manifest-submission` が `winget.yml` を main へ入れた後の回から、見守る相手に winget-pkgs への PR が加わる。手順の形は変わらない。
  - `roadmap.md` の「完了サマリ」は棚卸（`/kiro-discovery` の再入）と分け合う場所で、本 spec はそこへリリースの 1 行を足すだけ。

## Requirements

### Requirement 1: 繰り返しの形

**Objective:** As a 開発者, I want リリースを打ちたいときに同じ手順を 1 回ずつ回せる, so that リリースの時期を自分で選べ、PR のたびに版が上がることがない

#### Acceptance Criteria

1. When 開発者が本 spec の実装（`/kiro-impl areka-P0-release-cycle`）を打つ, the リリースの手順 shall 1 回の実行で 1 回のリリース（版を 1 つ上げ、タグを 1 つ打つ）だけを行う。
2. The リリースの手順 shall 2 回目以降、毎回同じタスクの並びで回る（初回だけの手順は要件 8 に分け、2 回目以降のタスクには残さない）。
3. When 1 回の実行が終わる（要件 7 の記録の 1 行を書き終える）, the リリースの手順 shall `tasks.md` のチェックをすべて未完了に戻し、本 spec を `.kiro/specs/` の直下に置いたままにする（`completed/` へ移さない）。
4. The 本 spec の `spec.json` shall 初回の実行の後も実装中の段のまま置き、完了の段へ進めない。
5. The リリースの手順 shall 版を上げ、タグを打つ唯一の経路である（ほかの spec の PR や `/kiro-complete` は版にもタグにも触らない）。
6. The リリースの手順 shall ソースコード（各クレートのコード・`tools/` のスクリプト・workflow のファイル）を書き換えない。書き換えるのは要件 3.3 のファイルと、本 spec のフォルダと、要件 7 の記録の 1 行と、初回だけ根の `README.md` の 1 行（要件 8.8・記録の 1 行と同じ PR）だけ。
7. The リリースの手順 shall 接続先の URL や認証の情報を画面にもログにも印字しない（`git remote -v` などを手順に入れない。してはいけない操作の一覧は `doc/crates-io-publish.md` 7 節に従う）。

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
3. The リリースの手順 shall 版上げで次だけを書き換える: 根の `Cargo.toml` の 2 行（`[workspace.package]` の `version`・`[workspace.dependencies]` の `dola` の `version`）、`Cargo.lock` のワークスペースのクレートの版の行、`THIRD-PARTY-NOTICES.md`（全体テストで作り直した物）、`dist/README.txt` の冒頭の「この説明書は …… 時点の内容です。」の日付（その日の日付にする）。初回だけ直す根の `README.md` の 1 行は版上げに含めず、記録の 1 行と同じ PR に載せる（要件 8.8）。
4. The リリースの手順 shall 各クレートの `Cargo.toml`・外部のクレートの版・`dist/README.txt` の「時点」の行以外の行を書き換えない。
5. If 版上げの差分に要件 3.3 の外の変更（`Cargo.lock` の外部のクレートの行を含む）が現れる, then the リリースの手順 shall その変更を取り込まずに止まり、開発者へ報告する。
6. If 根の `Cargo.toml` の 2 行の版が一致しない, then the リリースの手順 shall 版上げの PR を出さずに止まる。
7. If 上げた後の版のタグ `v{版}` が既に在る, then the リリースの手順 shall 版上げの PR を出さずに止まり、開発者へ報告する（同じ版で出し直さない）。
8. When 版を上げ終えた, the リリースの手順 shall 版上げの枝の中身で公開前の確かめを通し、緑を確かめる。
9. If 公開前の確かめが赤, then the リリースの手順 shall 版上げの PR を出さずに止まり、赤の理由を開発者へ報告する。

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

1. When タグを push した, the リリースの手順 shall Release の走りが緑で終わること、その版の GitHub Release が下書きでない公開の状態で在ること、4 つのファイル（x64 と arm64 の zip とそれぞれの SHA256）が添わっていることを確かめる。
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

1. When 見守りが終わる（すべて緑、または赤の原因を直す spec の起票まで済む）, the リリースの手順 shall `.kiro/steering/roadmap.md` の「完了サマリ」の下の「リリース」に 1 行を足す。
2. The 記録の 1 行 shall 版・日付・GitHub Release の URL・winget-pkgs への PR の URL（在る回だけ）・赤の原因を直すために起票した spec の名前（在る回だけ）を含む。
3. The リリースの手順 shall 記録を PR で main へ入れる（main へ直接 push しない）。
4. The リリースの手順 shall 記録の 1 行と `tasks.md` のチェックの戻し（要件 1.3）を、次に main へ入る spec か棚卸の PR に相乗りさせ、記録だけの PR は出さない（2026-10-05 開発者の裁定）。

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
- **版上げの道具**（brief の議題 3）→ 設計で決める（要件は変わらない）。brief の棚卸㉒の節は、手元の cargo-edit 0.13.13 のソースを読み、`cargo set-version --bump patch --workspace` が `[workspace.dependencies]` の `dola` の行も動かす見込みとした。実物は要件 8.3 で確かめる。
- **開いている PR の条件**（brief の議題 4）→ 2026-10-05 棚卸㉒の裁定 8 で「`Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md` を触る開いた PR 0 本」に改めた（要件 2.2・2.3）。
- **`THIRD-PARTY-NOTICES.md` の作り直し**（brief の議題 5）→ 作り直す。`.kiro/steering/workflow.md` の「`Cargo.lock` の扱い」が「lock を揃えたら謝辞も作り直してコミットに含める」と決めており、全体テストの `-License` が作り直す（要件 3.3）。
