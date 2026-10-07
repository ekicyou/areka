# ギャップ分析: areka-P0-release-cycle

> 2026-10-05・`/kiro-validate-gap`・基準は作業の枝 `claude/areka-p0-release-cycle-9e610d`（main `82607b5f` の上に spec の初期化 `378c1233`）。
> 本書は分析と選択肢だけを示す。決めるのは要件ディスカッションと設計の段。

## 1. まとめ

- **作る物はほぼ無い**。手順が使う道具（全体テスト・配布スクリプトの起動の確かめ・公開前の確かめ・2 本の workflow・手順書）はすべて main に在る。本 spec の中身は「`tasks.md` に書く手順」と「版上げのやり方の選択」になる。
- **版上げの道具は 2 案とも成り立つ**。cargo-edit 0.13.13 の `cargo set-version` は、ソースを読む限り `[workspace.dependencies]` の `dola` の行も動かす。手で 2 行を直して `cargo update -w` を回す案も、`workflow.md` が認めるやり方で済む。
- **いちばん大きい穴は道具ではなく「手順の器」**。`/kiro-impl` の自走の形（タスクごとにサブエージェント・タスクごとに `tasks.md` をコミット・最後に `/kiro-validate-impl`）は、開発者の承認を何度も挟み、版上げの PR に `tasks.md` を混ぜない（要件 4.2）この手順と噛み合わない。`/kiro-complete` の「繰り返し仕様の例外」も、要件 7.4（記録だけの PR を出さない）と食い違う。
- **記録の 1 行の受け渡し先が無い**。要件 7.4 は記録を「次に main へ入る PR」に相乗りさせるが、リリースの枝からその PR へ記録を渡す仕組みは、今のスキルにも steering にも無い。
- 調べ物として残るのは、`cargo set-version` の実物の差分（改行と `Cargo.lock`）、`cargo about` の作り直しが版の行だけで済むか、`git push` の出力に接続先が載らないかの 3 点。

## 2. 今あるもの（要件ごとの対応表）

印: **在る**＝そのまま使える／**欠け**＝無い／**未知**＝確かめが要る／**制約**＝既存の決まりと当たる。

| 要件 | 使う既存の物 | 状態 | 根拠 |
|---|---|---|---|
| 1.1 実行 1 回＝リリース 1 回 | `/kiro-impl`（自走の形） | **制約** | `.claude/skills/kiro-impl/SKILL.md` の「Autonomous Mode」はタスクごとにサブエージェントを起こし、レビューの後にコミットする。サブエージェントは開発者に聞けない |
| 1.3 `tasks.md` を戻す・`completed/` へ移さない | `kiro-complete` の「例外: 繰り返し仕様」・`workflow.md` の同じ注記 | 在る（ただし 7.4 と食い違う＝下の 4 節） | `.claude/skills/kiro-complete/SKILL.md` の「## 例外: 繰り返し仕様」（戻してコミット→全体テスト→PR）・`.kiro/steering/workflow.md` の「繰り返し仕様の例外」の注記 |
| 1.4 `spec.json` を実装中の段に置く | `kiro-spec-schema.md` の phase の値 | 在る | `.kiro/steering/kiro-spec-schema.md` の phase の表の `implementation`（「実装フェーズ中」）。brief の `implementation-in-progress` は表に無い語 |
| 1.5 版とタグの唯一の経路 | — | 在る（ほかの spec・`/kiro-complete` は版に触らない） | `kiro-complete` に版・タグを触る手順は無い（「版上げ」「タグ」で検索して 0 件） |
| 1.7 接続先を印字しない | 手順書の「してはいけない操作」 | 在る | `doc/crates-io-publish.md` 7 節（`git remote -v` ほか） |
| 2.1 main の最新を取り込む | `workflow.md` の取り込みの決まり | 在る | `.kiro/steering/workflow.md` の「`Cargo.lock` の扱い」 |
| 2.2 3 ファイルを触る開いた PR が 0 本 | `gh pr list --json files` | 欠け（コマンドの形だけ決めればよい） | 既存のスクリプトに無い |
| 2.4 全体テスト | `tools/test-all.ps1` | 在る | `tools/test-all.ps1` の `param([switch]$Format, [switch]$License)` の行と、`-License` で `cargo about generate … -o THIRD-PARTY-NOTICES.md` を回す段 |
| 2.5 起動の確かめ | `tools/package.ps1 -Check` | 在る | `tools/package.ps1` の冒頭の説明（`-Check` で x64 の zip を `target\package\check-*` へ展開して有界で起動） |
| 3.1〜3.3 版を上げる | cargo-edit 0.13.13（開発者の手元）／手で 2 行＋`cargo update -w` | 在る（道具の選択は設計） | `Cargo.toml` の `[workspace.package]` の `version = "0.0.1"` と `[workspace.dependencies]` の `dola = { version = "0.0.1", path = "crates/dola" }`。30 クレートすべて `version.workspace = true`（各クレートの `Cargo.toml` で数えて例外 0）。各クレートの `Cargo.toml` に「`path` と `version` を両方持つ行」は 0 件 |
| 3.3 `Cargo.lock` の版の行 | — | 在る | `Cargo.lock` の `version = "0.0.1"` は 30 行で、どれも `source` を持たない（ワークスペースの 30 クレートだけ）。`crates-io-publish` の実測で「版の行だけが動く（60 行の差＝30 行の入れ替え）」 |
| 3.3 `THIRD-PARTY-NOTICES.md` | `test-all.ps1 -License` | 在る（**未知**あり） | MIT の「対象 crate」の一覧に `- areka 0.0.1` 〜 の 30 行 |
| 3.3 `dist/README.txt` の「時点」の行 | — | 在る | `dist/README.txt` 冒頭の「この説明書は 2026-10-01 時点の内容です。」の行（BOM 付き・作業木は CRLF） |
| 3.5 差分が範囲の外なら止まる | — | 欠け（判定の形を決める） | 既存の道具に無い |
| 3.7 タグが既に在るなら止まる | — | 欠け（コマンドの形だけ） | リモートのタグは今 `v0.0.1` だけ |
| 3.8 公開前の確かめ | `tools/crates-io.ps1 -Verify -Version {版}` | 在る | `tools/crates-io.ps1` の `-Verify` の説明（`cargo publish --dry-run --allow-dirty --locked -p dola -p wintf`）。`crates-io-publish` の `research.md` の実測で「両方を 0.0.2 にすると、まだ crates.io に無い 0.0.2 でも乾いた走りは終了コード 0（`dola` 0.0.2 を仮の置き場で補う）」 |
| 4.1〜4.3 PR と squash マージ | `kiro-complete` の 8 段目の `gh pr create`／`gh pr merge --squash` | 在る（形を借りる） | `.claude/skills/kiro-complete/SKILL.md` の「PR 可: push → PR 作成 → squash マージ」 |
| 5.1〜5.4 タグ | — | 欠け（コマンドの形だけ） | squash のコミットは `gh pr view {番号} --json mergeCommit` で取れる |
| 6.1 Release の走りと Release | `release.yml` | 在る | `.github/workflows/release.yml` の `on: push: tags: ['v*']`・`PUBLISH` の行・「Release を公開」の段（`gh release create $tag @files … --verify-tag --generate-notes`）・`RUST_TOOLCHAIN: 1.99.0`・`timeout-minutes: 150` |
| 6.2 公開の走り | `crates-io.yml`・`tools/crates-io.ps1 -Pending` | 在る | `.github/workflows/crates-io.yml` の `$LIMIT_MINUTES = 120`・`rustup update stable`・`-Pending -Version` の段 |
| 6.3 winget | — | 在る（今は対象外） | `.github/workflows/` は `release.yml`・`crates-io.yml` の 2 本だけ |
| 6.4〜6.8 赤のときの決まり | 手順書 5 節 | 在る | `doc/crates-io-publish.md` 5 節（Re-run・Run workflow・タグのコミットの物が使われる件） |
| 7.1〜7.2 記録の 1 行 | roadmap の「完了サマリ」 | **欠け** | `.kiro/steering/roadmap.md` の「## 完了サマリ」は表と箇条書きだけで、「リリース」の小見出しは無い |
| 7.4 記録を次の PR に相乗り | — | **欠け** | 受け渡しの仕組みが無い（4 節） |
| 8.1〜8.2 Trusted Publishing | 手順書 2 節 | 在る（開発者の手作業） | `doc/crates-io-publish.md` 2 節の表 |
| 8.4 乾いた走り | `release.yml` の `workflow_dispatch` | 在る | 同 workflow の `workflow_dispatch:` と「乾いた走りなので止めずに続ける」の行 |
| 8.5 初回だけの 6 項目 | 申し送り | 在る | `.kiro/specs/completed/areka-P0-release-ci-workflow/verification/runner-trial.md` の「release-cycle への申し送り」1〜6 |
| 8.8 `README.md` の 1 行 | — | 在る | `README.md` の「## 入手と起動」直下の箇条「まだ GitHub Releases での配布はしていないので、…」 |

## 3. 手順の各段で見つかった細かい点

### 3.1 版上げの道具

- **cargo-edit 0.13.13**（`c:\rust\cargo\bin\cargo-set-version.exe` が在る）のソース `src/bin/set-version/set_version.rs` を読んだ。
  - `--workspace` で除外が無いとき `[workspace.package]` の `version` を書き換え（`set_workspace_version`）、続けて `update_dependents` を呼ぶ。この関数は根の `Cargo.toml` も見る（関数の中の注記「`get_dependency_tables_mut` returns workspace dependencies」）。brief の読みどおり、`dola` の行も動く見込み。
  - 最後に `resolve_ws` をもう一度呼び（`cargo metadata`）、`Cargo.lock` を書き直す。**このとき `--locked` を付けると、lock が変わるので失敗するはず**＝付けない。
  - `--dry-run` があり、書き換えずに「何をどう変えるか」を表示できる。
- **手で 2 行＋`cargo update -w`**: `workflow.md` の「`Cargo.lock` の扱い」が「`cargo update -w`（ワークスペースの `Cargo.toml` の差分だけを lock に反映し、他の版は動かさない）」と決めている。外の道具に頼らない。
- どちらの案でも、`dola` の行の動かし忘れは `cargo check` が「failed to select a version for the requirement `dola = "^0.0.1"`」で落ちる（`completed/areka-P0-crates-io-publish/research.md` の実測）。

### 3.2 改行の扱い（未知）

- 作業木の `Cargo.toml`・`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`dist/README.txt`・`README.md` はどれも CRLF で、`core.autocrlf=true`（Git の全体の設定）。
- 記憶に「生成器と about の改行だけの M」が残る＝`cargo about` や cargo の書き直しが LF で書くと、`git status` には「変更あり」と出るが、中身の差分は無い。
- **要件 3.5 の「範囲の外の変更が現れたら止まる」の判定は、`git status` でなく中身の差分（`git diff --numstat`）で行う必要がある**。`git status` で判定すると改行だけの M で誤って止まる。

### 3.3 全体テストの形

- `tools/test-all.ps1 -Format` は先に `cargo fmt --all` で**ソースを書き換える**。要件 1.6（ソースを書き換えない）とぶつかる。`-Format` を付けなくても `cargo fmt --check` は回る（同スクリプト冒頭の説明の 3 番）。
- 要件の「全体テスト（整形とライセンスの確かめを含む形）」は、`-License` だけを付ければ満たせる（整形は確かめるだけ）。
- `-License` の `cargo about generate` が版の 30 行以外も動かすことがあるか（`cargo about` の版・crates.io の情報の変化）は**未知**。動いたら要件 3.5 で止まる。

### 3.4 タグ

- 手元には `pre-rebase-5-1`・`pre-rebase-5-1b` というリモートに無いタグがある（`git tag -l`）。**`git push --tags` を使うと、これらも押し出してしまう**＝タグは名前を指して 1 本だけ押す形に限る。
- 要件 3.7 のタグの在否は、`git ls-remote --tags {remote} refs/tags/v{版}` か `gh api` で確かめられる。どちらも接続先を印字しない見込み。`git push` は出力の「To …」に接続先を出すが、git は認証の部分を伏せて出すはず（**未知**・要件 1.7）。

### 3.5 見守り

- Release の走りは最長 150 分、公開の走りは Release の緑を最長 120 分待つ。見守りはバックグラウンドの待ち（`gh run watch` など）になる。
- 要件 6.2 の `tools/crates-io.ps1 -Pending -Version {版}` は、ワークスペースの版がその版である作業木で回す必要がある（版の判定が先に走る）。タグを打った後の main を取り込んだ作業木で回す。
- 初回の 6 項目のうち、2（後始末の段の消す経路）と 4（Re-run）は、赤の走りが起きたときにしか観察できない。緑で終われば「起きなかった」と記録するだけになる。5 の後半（`release.yml` を含むタグからの乾いた走り）は、タグの後にもう 1 回手で起動する走りが要る（Release の検査は既に在る Release を見て「乾いた走りなので止めずに続ける」と出す）。

### 3.6 範囲の外で見つけたこと

- `crates/areka/README.md` の 7 行目「Early Development - Version 0.0.1」は、版を上げると古くなる。要件 1.6・3.4 の範囲の外なので本 spec では直せない（`areka` は crates.io へ出さないので、crates.io の頁には載らない）。
- `crates/areka/Cargo.toml` の `publish = false # crates.io の 0.0.1 は名前の確保だけ…` と、`README.md` の「crates.io の `areka` は名前を確保するための 0.0.1 だけ」は、版を上げても事実のまま（crates.io の `areka` は 0.0.1 のまま）。

## 4. 既存の決まりと当たるところ

### 4.1 `/kiro-impl` の器と、この手順の形

`/kiro-impl` の自走の形は、コードを書く spec のために作られている。

- タスクごとに新しいサブエージェントを起こす。サブエージェントは開発者に聞けないので、承認の点（版上げの PR のマージ・タグの push・Trusted Publishing を済ませたかの確認・赤のときの Re-run の判断）で止まる仕組みが要る。
- タスクごとに `tasks.md` の `[x]` をコミットする（「e) Commit」）。この枝から版上げの PR を出すと、**2 回目以降の版上げの PR に `tasks.md` が混ざり、要件 4.2 に反する**。
- テストを先に書く流れ（RED → GREEN）・機能の旗・最後の `/kiro-validate-impl` は、この手順に当てはまらない。
- 1 回の実行の途中で版上げの PR がマージされると、ワークツリーの枝は「既にマージされた枝」になり、後半のタスク（タグ・見守り）はその上で回ることになる。

### 4.2 `/kiro-complete` の「繰り返し仕様の例外」と要件 7.4

- `kiro-complete` は、繰り返し仕様では「`tasks.md` を戻してコミット → 全体テスト → PR → squash マージ」を行うと書く。
- 要件 7.4 は「記録の 1 行と `tasks.md` の戻しは次の spec か棚卸の PR に相乗りさせ、記録だけの PR は出さない」。`/kiro-complete` を毎回使うと、記録だけの PR が出てしまう。

### 4.3 `workflow.md` の「main への統合は `/kiro-complete` だけ」

- `.kiro/steering/workflow.md` は「出口: `/kiro-complete` … main への統合はここだけ」と書く。要件 4.1 の版上げの PR は `/kiro-complete` を通らずにマージされる。繰り返し仕様の例外として読むか、steering に一言足すか（足すなら要件 1.6 の書き換えてよい物の外）を決める必要がある。

### 4.4 記録の受け渡し

- 記録の 1 行は、見守りが終わった後（タグから数時間後）にでき、次に main へ入るほかの spec か棚卸の PR に載る。その PR の枝へ、この 1 行をどう渡すかの決まりが無い。`kiro-complete` の冒頭の棚卸・`kiro-discovery` の棚卸のどちらにも、「出たリリースを記録する」段は無い。

## 5. 実装の選択肢

### 案 A: 既存の道具だけで、手順を `tasks.md` に書く（新しいファイルを作らない）

- 版上げ: `cargo set-version --bump patch --workspace`（指示があれば `cargo set-version {版} --workspace`）。
- 確かめ: `tools/test-all.ps1 -License`・`tools/package.ps1 -Check`・`tools/crates-io.ps1 -Verify -Version {版}`・`gh pr list`・`git diff --numstat` を、`tasks.md` の各タスクにコマンドの形で書く。
- 良い点: 足す物が無い。2 回目以降も同じ文書を読むだけ。
- 悪い点: 差分の範囲の判定（要件 3.5）を毎回人か AI が読んで決める＝判定が揺れうる。cargo-edit が開発者の手元に在ることが前提になる（リポジトリの道具の一覧には無い）。

### 案 B: 版上げは手で 2 行を直し、`cargo update -w` で lock を揃える

- 版上げ: 根の `Cargo.toml` の 2 行を直接書き換え、`cargo update -w` を回す。ほかは案 A と同じ。
- 良い点: 外の道具に頼らない。`workflow.md` が既に認めたやり方。
- 悪い点: 2 行の片方を直し忘れる余地がある（ただし `cargo check` で必ず落ちる）。要件 8.3 の「道具の振る舞いの確かめ」は、この案では「手で直した 2 行の確かめ」に置き換わる。

### 案 C: 案 A か B に、差分の範囲の判定を行う小さなスクリプトを足す

- 置き場: 要件 1.6 と 4.4 により、`tools/` には置けない（`tools/` はソース扱い・初回の PR は spec の文書と版上げだけ）。置けるのは本 spec のフォルダの中だけ（例: `.kiro/specs/areka-P0-release-cycle/` の下）。
- 判定: `git diff --numstat` が「`Cargo.toml` 2/2・`Cargo.lock` 30/30・`THIRD-PARTY-NOTICES.md` 30/30・`dist/README.txt` 1/1」（クレートの数で変わる）と一致するか、行の中身が版と日付の行だけか。
- 良い点: 要件 3.5 の判定が毎回同じになる。
- 悪い点: spec のフォルダにスクリプトを置く前例が無い。クレートが増えると数が変わるので、数でなく「動いた行の形」で判定する作りが要る。規模に対して重い。

## 6. 規模と危険

- **規模: S**（1〜3 日）。作るのは文書（設計・タスク）だけで、初回の重さは見守り（最長で数時間の待ち）と初回だけの確かめ。
- **危険: 中**。コードは書かないが、初めて本物の GitHub Release と crates.io の公開が動く。crates.io の版は差し替えできず、Release の zip の URL は版ごとに固定なので、取り返しのつかない操作（PR のマージ・タグの push）が含まれる。器（`/kiro-impl`）との噛み合わせが未定。

## 7. 設計へ持ち越す調べ物

1. `cargo set-version --bump patch --workspace` の実物の差分: 根の `Cargo.toml` の 2 行が動くか（要件 8.3）、`Cargo.lock` が 30 行の入れ替えだけか、改行が LF に変わるか。`--dry-run` で先に見られる。
2. `test-all.ps1 -License` の `cargo about generate` が、版の 30 行の外を動かさないか。
3. `git push {remote} v{版}` と `git ls-remote` の出力に接続先の認証の部分が出ないか（要件 1.7）。
4. `gh pr list --json files` の `files` が、開いた PR の触るファイルを漏れなく返すか（大きな PR での件数の上限）。

## 8. 設計の段で決めること（議題の候補）

1. **版上げの道具**: 案 A（cargo-edit）か案 B（手で 2 行＋`cargo update -w`）か。brief の読みと棚卸㉒は案 A に傾いている。案 A なら、cargo-edit を開発者の手元の前提として書くかどうか。
2. **`/kiro-impl` との噛み合わせ**: 自走の形のまま回すなら、承認の点でどう止めるか（タスクに「開発者の承認を待つ」印を付けて親が止まる、など）と、タスクごとの `tasks.md` のコミットを版上げの PR に混ぜない方法（`tasks.md` のチェックはコミットしない、など）。要件 1.1 はコマンドを `/kiro-impl areka-P0-release-cycle`（タスク番号なし＝自走の形）と決めている。
3. **`tasks.md` の戻し方**: 版上げの PR に `tasks.md` の `[x]` を載せなければ、main の `tasks.md` は常に未完了のままで、「戻す」は手元の枝だけの話になる。要件 7.4 の「戻しを次の PR に相乗り」が実質的に要らなくなるか。
4. **記録の 1 行の受け渡し**: 見守りの後にできる 1 行を、次の spec か棚卸の PR へどう渡すか。たとえば ⒜ 次の PR を出す側（`/kiro-complete` の冒頭の棚卸か棚卸）が `gh release list` から事実を読んで書く ⒝ リリースの枝に置いたまま、次の PR の枝へ手で写す ⒞ 記憶のファイルに残す。winget の PR の URL と起票した spec の名前（要件 7.2）は Release の一覧からは読めない。
5. **`roadmap.md` の「リリース」の置き場**: 「完了サマリ」の下に「リリース」の小見出しは今無い。初回の記録のときに小見出しと表（版・日付・Release・winget の PR・起票した spec）を作る形でよいか。
6. **`/kiro-complete` と `workflow.md` の扱い**: 本 spec は毎回 `/kiro-complete` を使わない（使うと記録だけの PR が出る）。`kiro-complete` の「繰り返し仕様の例外」と、`workflow.md` の「main への統合は `/kiro-complete` だけ」の文と、どう折り合うか。文書を直すなら、要件 1.6・4.4 の書き換えてよい物の外になる。 → **要件ディスカッション議題 1 で決着**: 本 spec は `/kiro-complete` を使わない。例外は本 spec だけなので、決まりは本 spec の要件・設計・タスクの中に置き、`workflow.md`・`kiro-complete` の文は直さない（要件 1.8）。
7. **全体テストの形**: `-License` だけ（`-Format` を付けない）にするか。`-Format` はソースを書き換えうる。
8. **差分の範囲の判定**: 人（AI）が `git diff --numstat` を読んで判定するか、案 C の小さなスクリプトにするか。改行だけの M を誤って拾わない形（中身の差分で見る）にすること。
9. **タグの押し方**: 名前を指して 1 本だけ押す（`--tags` を使わない＝手元のリモートに無いタグを押さない）。squash のコミットを `gh pr view` から取る。
10. **初回の 6 項目の記録の形と置き場**: 赤が起きないと観察できない項目（後始末の消す経路・Re-run）は「起きなかった」と書く形でよいか。6 項目の 5 の後半（タグからの乾いた走り）をタグの後にもう 1 回起動して確かめるか（最長 150 分）。記録は本 spec のフォルダの `verification/` などに置くか。
11. **見守りの待ち方**: Release の走り（最長 150 分）と公開の走り（最長 120 分待つ）をどう待つか（バックグラウンドの待ちで終わりを知らされる形など）。
12. ~~**`crates/areka/README.md` の「Version 0.0.1」の行**~~ → 要件ディスカッションで**対応不要**とした。この README は「名前の確保のために crates.io へ出した `areka` 0.0.1」の説明（同ファイルの「This crate is published for name reservation purposes」の行）で、`areka` は `publish = false` のまま crates.io では 0.0.1 から動かないので、版を上げても事実のまま。

> 要件ディスカッション（2026-10-05）での扱い: 3 は要件 1.3・7.4 を「main の `tasks.md` が未完了の状態に在ること」という結果の形に直し、戻しが要るかどうかは設計に任せた。7 は要件の言葉の定義（全体テスト＝整形は確かめるだけ）に書いた。8 の「改行の違いだけを数えない」は要件 3.5 に書いた。5 の「小見出しが無ければ初回に作る」は要件 7.1 に書いた。残りの 1〜11 の決め方は設計の段で行う。

## 9. 次の段

- 要件ディスカッションで 8 節の議題（とくに 2・3・4・6）を開発者と詰める。要件が変わるのは 4・6（記録の受け渡しと文書の直しの範囲）になりうる。
- その後 `/kiro-design areka-P0-release-cycle` で設計へ進む。

---

# 設計の段の調べと決め（2026-10-05・`/kiro-spec-design`）

## Summary

- **Feature**: `areka-P0-release-cycle`
- **Discovery Scope**: Extension（軽い調べ。既に在る道具と workflow をつなぐ手順の設計で、新しい依存は 0 個）
- **Key Findings**:
  - 承認で止まれるのは `/kiro-impl` の主文脈だけ。タスクごとのサブエージェントは開発者に聞けないので、この spec では主文脈が自分でタスクを行う形に読み替える。
  - `tasks.md` の `[x]` をコミットしなければ、版上げの PR に `tasks.md` が混ざらず、main の `tasks.md` はいつも未完了のままになる。「戻す」作業そのものが要らなくなる。
  - `release.yml` は Release を `gh release create` で直接公開する（下書きを経ない）。下書きは gh が止められたときにだけ残る。申し送りの 6 項目のうち 3 つ（後始末の消す経路・下書きの見え方・Re-run）は、赤か取り消しが起きた回にしか見られない。

## Research Log

### `/kiro-impl` の自走の形と承認の点

- **Context**: ギャップ分析 4.1 節・8 節の 2。
- **Sources Consulted**: `.claude/skills/kiro-impl/SKILL.md` の「Autonomous Mode」「e) Commit」「Step 4: Final Validation」「Feature Flag Protocol」。
- **Findings**:
  - 文脈の読み込みで `design.md` と `tasks.md` の両方を読む。読み替えを両方に書けば、スキルの文書を直さずに伝わる。
  - 機能の旗の手順は、スキル自身が「設定・文書・振る舞いの変わらないタスクでは飛ばす」と書いている。
  - タスクごとのコミットは「変えたファイルと `tasks.md`」を足す決まり。この spec ではここを読み替える。
- **Implications**: 設計の「`/kiro-impl` の読み替え」の表。`tasks.md` の冒頭に、その表を指す短い節を置く。

### `release.yml` の公開と後始末の形

- **Context**: 初回の 6 項目を、どの事実で見るかを決めるため。
- **Sources Consulted**: `.github/workflows/release.yml` の段の名前（「版の検査」「既存の Release の検査」「Release を公開」「後始末」）と、`PUBLISH` の行・`concurrency` の行・後始末の段の `if` の行。
- **Findings**:
  - 公開は `gh release create $tag @files @opts` の 1 回。下書きを作ってから公開に変える形ではない。
  - 後始末の段は「失敗か取り消し」かつ「タグの push」かつ「始めに Release が無かった」ときだけ動く。手で起動した乾いた走りでは動かない。
  - `concurrency` の組は ref ごと。タグから手で起動する乾いた走りは、タグの push の走りと同じ組になる（先の走りが終わるまで待たされるだけで、取り消されない）。
- **Implications**: タグからの乾いた走り（申し送りの 5 の後半）は、Release の走りと公開の走りが両方終わった後に始める。乾いた走りが Release を消すことは無い。

### タグの種類と push の出力

- **Context**: ギャップ分析 3.4 節・7 節の 3。
- **Sources Consulted**: `git cat-file -t v0.0.1`（`commit`＝注釈なしのタグ）・`git tag -l`（`pre-rebase-5-1`・`pre-rebase-5-1b`・`v0.0.1`）・`git remote`（名前だけ）。
- **Findings**: 既に在る `v0.0.1` は注釈なしのタグ。手元の 3 本のうち 2 本はリモートに出したくないタグ。
- **Implications**: 注釈なしのタグで揃える。push は `refs/tags/v{版}` を名指しする 1 本だけ。`git` の通信のコマンドは `--quiet` を付けて標準エラーを捨て、成否は終了コードと読み直しで決める（出力に接続先が載るかどうかを当てにしない）。実物の確かめは初回の実走で行う。

### roadmap の「完了サマリ」の今の形

- **Context**: 「リリース」の置き場。
- **Sources Consulted**: `.kiro/steering/roadmap.md` の「## 完了サマリ」（ウェーブの表と、その後の箇条書き 3 つ）。
- **Findings**: 表の後の箇条書きには「完了 spec 直下エントリ＝……」のように、spec の完了のたびに書き換わる行が在る。
- **Implications**: 「リリース」は箇条書きの後・次の「## 」の前に置き、行は表の末尾に足す。相乗りの頼みには記録の 1 行の文字も添え、取り込みで衝突したら手で足せるようにする。

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| 主文脈が自分で回す | `/kiro-impl` の読み替えを本 spec の中に置く | 承認で止まれる・文書を足さない | 読み替えを読み落とすと、いつもの自走の形で回ってしまう | 採る。`tasks.md` の冒頭にも書く |
| サブエージェントに回させ、承認の点で親が止まる | タスクに「承認待ち」の印を付ける | `/kiro-impl` の形に近い | サブエージェントに渡す文脈が増えるだけで、得る物が無い（レビューする差分が 4 ファイル） | 採らない |
| 専用のスキルを足す | `/kiro-release` のような新しいスキル | 読み替えが要らない | 要件 1.1 はコマンドを `/kiro-impl` と決めている。分け合う文書が増える | 採らない |

## Design Decisions

### Decision: `tasks.md` の `[x]` をコミットしない

- **Context**: 要件 1.3・4.2・7.4。
- **Alternatives Considered**:
  1. タスクごとに `[x]` をコミットし、最後に戻して次の PR に相乗り
  2. `[x]` を作業木にだけ付け、コミットしない
  3. `tasks.md` に印を付けず、進み具合を別のファイルに書く
- **Selected Approach**: 2。
- **Rationale**: 1 は 2 回目以降の版上げの PR に `tasks.md` が混ざる（要件 4.2 に反する）。3 は再開のときに「どこまで進んだか」を読む相手が増える。2 なら main の `tasks.md` に `[x]` が入る経路が 0 本になる。
- **Trade-offs**: 作業木を作り直すと印が消える。そのときは PR・タグ・Release の在否から続きを決める（設計の「再開のときの事実の読み方」）。
- **Follow-up**: 初回の実走で、作業木に `tasks.md` の変更が在るままで `tools/package.ps1 -Check`（始めと終わりの `git status` が同じかを見る）が通ることを確かめる。

### Decision: 記録は作業の枝のコミットと、相乗りの頼みの 1 文で渡す

- **Context**: 要件 7.3・7.4・1.8。ギャップ分析 4.4 節。
- **Alternatives Considered**:
  1. 次の PR を出す側のスキルが、Release の一覧から事実を読んで書く
  2. 作業の枝に記録のコミットを置いて push し、開発者へ頼みの 1 文を渡す
  3. 記憶のファイルに残す
- **Selected Approach**: 2。
- **Rationale**: 1 は `/kiro-complete` か `/kiro-discovery` の文書を直すことになり、要件 1.8 に反する。winget の PR と起票した spec の名前は Release の一覧から読めない。3 は main に入る保証が無い。
- **Trade-offs**: 開発者が次のセッションへ 1 文を渡す手間が残る。忘れに備えて、次の回の段 1 が「前の回の行が main に在るか」を読んで知らせる。
- **Follow-up**: roadmap の取り込みで衝突が多いようなら、置き場を見直す。

### Decision: 初回の乾いた走りは段 1 で始め、承認 A の前に緑を見る

- **Context**: 要件 8.4（タグを打つ前・main で）。
- **Alternatives Considered**:
  1. 版上げの PR をマージした後、タグの直前に回す
  2. 段 1 で始め、マージの承認の前に緑を見る
- **Selected Approach**: 2。
- **Rationale**: 1 で赤が分かると、main の版だけが上がってタグを打てない版が残る。2 なら何も出ていないうちに止まれる。「main で」「タグを打つ前」の両方を満たす。
- **Trade-offs**: 乾いた走りが見る main は版上げの前の物（版上げの 4 ファイルの差は入っていない）。版上げの中身は手元の全体テスト・起動の確かめ・公開前の確かめが見る。

### Decision: 起票の書き込みは、この手順の「書き換える物」に数えない

- **Context**: 要件 1.6 は書き換えてよい物を限り、要件 6.5・6.7・8.6 は `/kiro-discovery` での起票を求める。起票は brief と roadmap の行を書く。
- **Selected Approach**: 起票の書き込みは `/kiro-discovery` の成果物として別のコミットに置き、記録のコミットと一緒に相乗りで渡す。要件 1.6 は「リリースの手順そのものが書き換える物」の決まりとして読む。
- **Rationale**: そう読まないと 1.6 と 6.5 が両立しない。起票が要るのは赤の回だけ。
- **Follow-up**: 設計ディスカッションで、この読みでよいかを開発者に確かめる。

### 統合（`design-synthesis` の 3 つの見方）

- **まとめられる物**: 「範囲の外の変更で止まる」（3.5）・「2 行の版が合わなければ止まる」（3.6）・「道具が 2 行とも動かしたか」（8.3）は、同じ 1 つの差分の読み（段 3 の c）で済む。別々の確かめにしない。
- **作るか・在る物を使うか**: 全部、在る物を使う。足すスクリプトは 0 本。差分の範囲の判定のスクリプト（ギャップ分析の案 C）は、見る相手が 4 ファイル・行の形が 2 種類で、承認 A で開発者も同じ差分を見るので作らない。
- **削った物**: 進み具合を書く別のファイル・記録だけの PR・`tasks.md` を戻すコミット・`/kiro-validate-impl`・レビュー役のサブエージェント。

## Risks & Mitigations

- 読み替えを読み落として、いつもの自走の形で回る — `tasks.md` の冒頭に「走らせ方」を置き、最初のタスクを「読み替えを読む」にする（タスクの段で決める）。
- `cargo about` の作り直しが版の行の外を動かす — 段 3 の c で止まる。開発者に示し、謝辞だけを先に別の PR で揃えるかを決めてもらう。
- 相乗りを忘れる — 次の回の段 1 が知らせる。
- 開いた PR の `files` が 100 件で切れる — 100 件ちょうどの PR は `gh pr diff --name-only` で読み直す。
- マージの後にタグを打てない — 待つ。再開しても、タグを打つ相手は同じ squash のコミット。

## References

- `.claude/skills/kiro-impl/SKILL.md` — 自走の形と、読み替える段。
- `doc/crates-io-publish.md` — 2 節（Trusted Publishing）・4 節（出たことを確かめる）・5 節（やり直し）・7 節（してはいけない操作）。
- `.kiro/specs/completed/areka-P0-release-ci-workflow/verification/runner-trial.md` — 「release-cycle への申し送り」の 6 項目。
- `.kiro/steering/workflow.md` — 「`Cargo.lock` の扱い」（`cargo update -w`）。
