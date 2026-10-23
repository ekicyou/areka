# 生きている文書の洗い出し

完了していない文書から、次のどれかを書いている所を洗い出した記録である（要件 8.1・8.2）。1 か所を 1 行で書く。

- **ア**: リリースの本文の作り方
- **イ**: リリースの添付の数
- **ウ**: 全体テスト（`tools/test-all.ps1`）の段の一覧と、段の並び
- **エ**: 上の 3 つを語るために要る置き場の一覧（`tools/` の道具・見どころのフォルダ・リリース CI の説明）

洗い出しは 2026-10-11、作業の枝の先頭 `6f3dab71` で行った。場所は「何の行か」で指す（行の番号では指さない）。

## 洗い出しの範囲と数え方

- **見た文書**: `.kiro/steering/*.md`・`doc/`（`doc/release-notes/` を除く）・根の `README.md`・`dist/`・`.claude/skills/`（スキルの本文・雛形・規則）・`.claude/agents/`・`CLAUDE.md`・`.kiro/settings/`・`.kiro/MEMO.md`・`docs/`。`AGENTS.md` は無い。
- **見たが書き換えない文書**: ほかの完了していない spec のフォルダ（`.kiro/specs/*/`。ほかの spec の文書なので直さない。今の形と食い違う所が在れば、ここに記録する）。
- **見ていない文書**: `.kiro/specs/completed/`（書き換えない・要件 8.2）・`.kiro/specs/areka-P0-release-cycle/`（タスク 5.1・5.2 が洗い出して直した。記録は `release-cycle-sweep.md`）・本 spec のフォルダ・`vendors/`・`target/`。`tools/test-all.ps1` の冒頭の注記はタスク 2.4 が、`.github/workflows/release.yml` はタスク 4.1・4.2 が、`doc/release-notes/` はタスク 1 と 3 が受け持った。
- **探した語**: `encoding check`・`test-all`・`release notes check`・`-License`・数の付いた「段」／`release.yml`・`GitHub Release`・`gh release`・`--generate-notes`・「リリースノート」・「添付」・`Assets`・`.sha256`・`SHA256`・「4 つ」「5 つ」・「見どころ」・`CHANGELOG`・「変更履歴」。
- **数え方**: 上のア〜エを書いている所と、書いているように見えて決めが要った所を 1 か所と数える。`release.yml` や `tools/test-all.ps1` の名前を引くだけの所は数えない。

## 数

| 文書 | 洗い出した所 | 直す | 直さない |
|---|---|---|---|
| `.kiro/steering/tech.md` | 7 | 2 | 5 |
| `.kiro/steering/structure.md` | 6 | 3 | 3 |
| `.kiro/steering/workflow.md` | 2 | 1 | 1 |
| `.kiro/steering/roadmap.md` | 11 | 0 | 11 |
| `.kiro/steering/roadmap-history.md` | 0 | 0 | 0 |
| steering のほかの 5 つ（`product.md`・`focus.md`・`logging.md`・`kiro-spec-schema.md`・`two-tunnel.md`） | 0 | 0 | 0 |
| `doc/crates-io-publish.md` | 3 | 0 | 3 |
| `doc/` のほかの文書（`ROADMAP.md`・`impl-watch.md`・`COMPAT_ARCHITECTURE.md`・`CONSTITUTION.md`・`PASTA_PROFILE.md`・`anchor-compat.md`・`choice-cascade-compat.md`・`emo2-conformance-scope.md`・`discussion-260702-*`・`archive/`・`shiori/`・`ssp-mcp/`・`ukadoc-coverage/`） | 0 | 0 | 0 |
| 根の `README.md` | 3 | 0 | 3 |
| `dist/README.txt` | 1 | 0 | 1 |
| `dist/winget/` | 0 | 0 | 0 |
| `.claude/skills/kiro-complete/SKILL.md` | 3 | 1 | 2 |
| `.claude/skills/kiro-impl/SKILL.md` | 1 | 0 | 1 |
| `.claude/skills/pasta-check/SKILL.md` | 1 | 0 | 1 |
| スキルのほかの文書（上の 3 つを除く `.claude/skills/` の全部）・`.claude/agents/` | 0 | 0 | 0 |
| `CLAUDE.md` | 0 | 0 | 0 |
| `.kiro/settings/`・`.kiro/MEMO.md`・`docs/` | 0 | 0 | 0 |
| ほかの完了していない spec（`winget-release-automation` の brief） | 1 | 0 | 1 |
| 合わせて | 39 | 7 | 32 |

「直す」の 7 か所は全部直した。直し残しは 0 か所である。`.kiro/specs/completed/` の差分は 0 行である。ほかの完了していない spec の文書のうち、古い形（GitHub に作らせる本文・添付 4 つ・新しい段の無い段の一覧）を今の形として書いている所は 0 か所である。

## `.kiro/steering/tech.md` — 7 か所

| # | 何の行か | 従う物 | 決め | 理由・直し |
|---|---|---|---|---|
| T1 | Testing の「フルテスト」の冒頭の段落（`-Format` と `-License` の説明） | ウ | 直さない | 段を並べていない。「`-License` は最後に直列で回す」は今も正しい（新しい段はその前に入った） |
| T2 | Testing の「段 `encoding check`」の項（場所の説明「crates.io の公開前の確認の後、`-License` の段の前」） | ウ | 直さない | 今も正しい。新しい段との前後は T3 の項が書く |
| T3 | Testing の「段 `encoding check`」の項の後ろ（新しい段の項が無かった） | ウ・エ | 直す | 「段 `release notes check`」の項を足した（呼ぶ道具・場所は `encoding check` の後で `-License` の段の前・ネットを使わない・確かめる中身） |
| T4 | Testing の「テストの門は手元のフルテストのまま。ビルドと配布だけを GitHub Actions に乗せる」の段落の、`release.yml` の説明（「zip と SHA256 を作って GitHub Release を公開する」） | ア・イ・エ | 直す | 詳しいリリースノートを作ること・本文はその版の見どころと詳しいリリースノートへのリンクで GitHub に一覧を作らせないこと・添付は 5 つ・見どころが欠けていれば組み立ての前に止まることを足した |
| T5 | Common Commands の「Full test」の行 | ウ | 直さない | コマンドだけで、段を並べていない |
| T6 | 「crates.io への公開」の「全体テストの段はネットを使わない形で包む」 | ウ | 直さない | crates.io の公開前の確認の段の話である。新しい段もネットを使わないので、食い違わない |
| T7 | 「crates.io への公開」の「公開の段は `.github/workflows/crates-io.yml`」（同じタグの `release.yml` の回が緑で終わるのを待つ） | ア・イ | 直さない | 待つ相手として引くだけで、本文にも添付の数にも触れていない |

## `.kiro/steering/structure.md` — 6 か所

| # | 何の行か | 従う物 | 決め | 理由・直し |
|---|---|---|---|---|
| S1 | Workspace Root の「その他の最上位」の `tools/` の道具の一覧 | エ・ウ | 直す | `release-notes.ps1`（`-Check`・`-Notes`・`-Body`・定型文のファイル・見本のフォルダ・`-SelfTest` が全体テストの段 `release notes check`）を、`encoding-check.ps1` の後ろに足した |
| S2 | 同じ行の `.github/` の `workflows/release.yml` の説明（「zip と SHA256 を作って GitHub Release を公開する」） | ア・イ | 直す | 詳しいリリースノートを作ること・本文はその版の見どころ・添付は 5 つを足した |
| S3 | 同じ行（見どころのフォルダの説明が無かった） | エ | 直す | `doc/release-notes/`（`README.md`＝書き方の決まりの正本／`v{版}.md`＝版ごとの見どころ。版上げの PR で 1 つ足し、Release の本文になる）を足した |
| S4 | 同じ行の `tools/` の `test-all.ps1` の説明 | ウ | 直さない | 段を並べず、`tech.md` の Testing を指している |
| S5 | 同じ行の `tools/` の `package.ps1` の説明（zip と SHA256 を組む） | イ | 直さない | 配布スクリプトが手元で作る物の話で、リリースの添付の数ではない |
| S6 | 同じ行の `.github/` の `workflows/crates-io.yml` の説明（release の成功を待って出す） | ア・イ | 直さない | 本文にも添付の数にも触れていない |

## `.kiro/steering/workflow.md` — 2 か所

| # | 何の行か | 従う物 | 決め | 理由・直し |
|---|---|---|---|---|
| W1 | 「Test Gate はアーカイブの後の 1 回」の項の、かっこの中の段の並び | ウ | 直す（設計の Modified Files に無い） | 段の一覧の写しが、新しい段を落としていた。「道具の文字コードの判定」と「deny」の間に「リリースノートの道具の自分の検査」を足した。手順は変えていない |
| W2 | 完了チェックリストの「全テストがパス」 | ウ | 直さない | コマンドと終了コードだけで、段を並べていない |

要件の Adjacent expectations の「`workflow.md` や `kiro-complete` は直さない」は、見どころを書く段をどこに書くかの決まりである（リリースの手順の文書に書く）。W1 と K1 は全体テストの段の名前の追随だけで、その決まりに触れない。

## `.kiro/steering/roadmap.md` — 11 か所

roadmap は、日付の付いた記録と起票の台帳である。完了の記録と起票のときの実測は、その日の事実なので書き換えない。本 spec の行は、完了のときに `kiro-complete` の文書の更新が直す。

| # | 何の行か | 従う物 | 決め | 理由・直し |
|---|---|---|---|---|
| M1 | 完了サマリの表の C2 の行（「`release.yml` を置いた（zip と SHA256 → 4 つ揃いの Release を公開）」「公開前の確認を全体テストの段に」） | イ・ウ | 直さない | `areka-P0-release-ci-workflow` と `areka-P0-crates-io-publish` の完了の記録で、その時点の事実である |
| M2 | 完了サマリの表の C4 の行（「全体テストの段 `encoding check` に足した」） | ウ | 直さない | `areka-P0-tools-utf8-child-output` の完了の記録である |
| M3 | 完了サマリの「リリース」の表（`v0.0.2` の行） | ア・イ | 直さない | 版・日付・URL の登記で、本文にも添付の数にも触れていない |
| M4 | spec 台帳の `release-notes-highlights` の行 | ア・イ | 直さない | 起票の記録である。もう新しい形（見どころ・Assets に 1 つのファイル）を書いている。完了のときに行ごと外れる |
| M5 | テーマ「配布と公開」の「決めたこと」の 4（「GitHub Actions がビルド・zip・SHA256・Release」） | ア・イ | 直さない | 2026-10-02 の決めの記録で、本文の作り方にも添付の数にも触れていない |
| M6 | テーマ「リリースノート」の「起票時の実測」（`--generate-notes`・「Assets は zip と `.sha256` の 4 つだけ」） | ア・イ | 直さない | 起票のときの main `09f4e33f` の実測と明記してある。直す前の姿の記録である |
| M7 | テーマ「リリースノート」の「推す形」（「5 つ目の Assets として添える」） | ア・イ | 直さない | もう新しい形を書いている |
| M8 | テーマ「リリースノート」の「pasta と違う所」（「添付の数（4 → 5）」） | イ | 直さない | もう新しい数を書いている |
| M9 | テーマ「リリースノート」の Existing Spec Updates の `areka-P0-release-cycle` の行（「公開の後の確かめを 5 つのファイルにする」） | イ | 直さない | もう新しい数を書いている。印は完了のときに付く |
| M10 | テーマ「リリースノート」の Existing Spec Updates の `winget-release-automation` の行（「リリースのファイルが 5 つになる」） | イ | 直さない | もう新しい数を書いている |
| M11 | テーマ「リリースノート」の Specs の `areka-P0-release-notes-highlights` の行 | ア・イ | 直さない | もう新しい形を書いている。印は完了のときに付く |

## `doc/crates-io-publish.md` — 3 か所

| # | 何の行か | 従う物 | 決め | 理由・直し |
|---|---|---|---|---|
| C1 | 「3. いつもの流れ」の 3（「`release.yml` が、組み立て・zip・GitHub Release の公開を行う」） | ア・イ | 直さない | Release を公開する事実だけで、本文の作り方にも添付の数にも触れていない（設計の「文書の追随と全体テストの段」の決めと同じ） |
| C2 | 「3. いつもの流れ」の段の表の「Release の確認」 | ア・イ | 直さない | Release が在って下書きでないことだけを見る段で、本文も添付も見ない |
| C3 | 「6. 予備の手順」の「必達」の 2（全体テストが緑） | ウ | 直さない | コマンドだけで、段を並べていない |

## 根の `README.md` — 3 か所

| # | 何の行か | 従う物 | 決め | 理由・直し |
|---|---|---|---|---|
| R1 | 「入手と起動」の「GitHub Releases から配布物の zip を入手して展開する」 | ア・イ | 直さない | zip の入手だけで、本文にも添付の数にも触れていない。今も正しい |
| R2 | 「コマンド」の全体テストの行と、その下の「`tools/test-all.ps1` は準備から判定までを 1 本で行います」 | ウ | 直さない | 段を並べていない |
| R3 | 「コマンド」の配布物の zip の注記（「zip と .sha256」） | イ | 直さない | 配布スクリプトが手元で作る物の話で、リリースの添付の数ではない |

## `dist/README.txt` — 1 か所

| # | 何の行か | 従う物 | 決め | 理由・直し |
|---|---|---|---|---|
| D1 | 「■ 入手のしかた」の「配布の zip（GitHub のリリースの頁の URL）を展開します」の行 | ア・イ | 直さない | zip の入手だけで、本文にも添付の数にも触れていない。今も正しい |

## スキルの文書 — 5 か所

| # | 何の行か | 従う物 | 決め | 理由・直し |
|---|---|---|---|---|
| K1 | `kiro-complete` の「5-2. 全体テストを裏で起動」の「段」の項（段の名前の並び） | ウ | 直す | `encoding check` と `cargo deny check` の間に `release notes check`（リリースノートの道具の自分の検査・`tools/release-notes.ps1 -SelfTest`）を足した。段の名前の追随だけで、手順は変えていない |
| K2 | `kiro-complete` の「哲学」の「全テストは全体テストのスクリプト `tools/test-all.ps1` で回す」 | ウ | 直さない | 段を並べていない |
| K3 | `kiro-complete` の「完了チェックリスト」の「全体テスト 1 回」（Format / Test / License / 移動後の各ゲートを兼ねる） | ウ | 直さない | 門の名前で、段の一覧ではない。新しい段は Test の門の中に入る |
| I1 | `kiro-impl` の Preflight の「`tools/test-all.ps1` runs `cargo fmt --all -- --check`」 | ウ | 直さない | 整形の段を 1 つ例に引くだけで、段を並べていない |
| P1 | `pasta-check` の「pasta リポジトリ（hello-pasta）の場合」（「リリースノートはコミット履歴から生成」） | ア | 直さない | pasta のリポジトリのリリース CI の話で、areka のリリースではない（要件の Out of scope「pasta のリポジトリの文書」） |

## ほかの完了していない spec の文書 — 1 か所

| # | 何の行か | 従う物 | 決め | 理由・直し |
|---|---|---|---|---|
| X1 | `winget-release-automation` の brief の、ほかの spec からの申し送り（「リリースに添えるファイルが 4 つから 5 つになる」） | イ | 直さない | ほかの spec の文書である。もう新しい数を書いている |

## 段の一覧の写しと `tools/test-all.ps1` の突き合わせ

`tools/test-all.ps1` の段（末尾の一覧に出る名前・回る順）は、`add i686 target` → `build i686 artifacts` → `cargo fmt (format)`（`-Format`）→ `fmt --check` → `x64 workspace tests` → `i686 tests (host-32)` → `crates.io pre-publish check (package only)` → `encoding check` → `release notes check` → `cargo deny check` → `cargo about generate`（終わりの 2 つは `-License`）である。

| 写し | 直した後の並び | 判定 |
|---|---|---|
| `kiro-complete` の「段」の項（K1） | 上と同じ 11 の名前が、同じ順 | 一致 |
| `workflow.md` の Test Gate のかっこ（W1・日本語の言い換え） | 整形・i686 の導入と成果物のビルド・fmt --check・x64 の全テスト・i686 でしか走らないテスト・crates.io の公開前の確認・道具の文字コードの判定・リリースノートの道具の自分の検査・deny・about | 名前は全部一致。先頭の「整形」はこの文書の前からの言い換えで、スクリプトは i686 の導入と成果物のビルドを先に回す（前から在る違いで、この直しが作ったものではない。`workflow.md` の並びは動かしていない） |
| `tech.md` の段の項（T2・T3） | 段を全部は並べない。`encoding check` は crates.io の公開前の確認の後、`release notes check` は `encoding check` の後で `-License` の段の前 | 食い違いなし |
| `structure.md` の `tools/` の一覧（S1） | 段を並べない。道具ごとに、どの段かを書く（`encoding check`・`release notes check`） | 食い違いなし |
