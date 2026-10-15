# Brief: areka-P0-roadmap-overview-html

> **版に縛らない（道具と手入れ）・開発者が最初に着手する**（2026-10-15 `/kiro-discovery`「版の段」の続き）。開発者「ちょっとロードマップを視覚的に見たいが、html でロードマップを俯瞰できる資料を作ってもらえないか？ doc フォルダの整理もかねて。これは今作ってくれてもよいし、制作・整理（古い記載の doc を消す）spec を作ってもよい」→ 推し（spec 1 本・作り直せる道具＋古い doc の整理）を開発者が採った（「全て推奨で」「俯瞰図 spec を最初に実施する」）。下の「推す形」と「議題」は、要件の段で覆してよい。

## Problem

- ロードマップの正本 `.kiro/steering/roadmap.md` は 1,000 行を超え、1 行が数千字の表もある。版の段（v0.0.3〜v0.4.0・いつか・windows-reactor 待ち・版に縛らない）・テーマ・依存の順・区分を、開発者が一目で見渡せない。
- `doc/` には古い記載が残り、どれが今も効いているのか分かりにくい（M1 期の議論・旧 ROADMAP のポインタ・旧 ARCHITECTURE など）。

## Current State（main `46123b00` の後・起票時の調べ）

### ロードマップ

- 正本は `.kiro/steering/roadmap.md`（`inclusion: manual`・CRLF）。俯瞰に要る材料は次の 4 か所に在る。
  - 「版の段」節の表（版 → 目標 → spec の名前の並び・`→` が依存の順）。2026-10-15 に置いた。
  - 「spec 台帳」の表（spec ごとの段・区分・何をするか・規模・並び・前提・Fable）。数え方の `awk` は台帳の頭の注に在る。
  - 「テーマ別の決めごと」の各節の `#### Specs (dependency order)`（`- [ ]`／`- [x]` と `Dependencies:`）。
  - 「ウェーブ編成」の表（いま走っているウェーブ C6）。
- 完了した spec は `.kiro/specs/completed/`（263 本）。各 brief は `.kiro/specs/areka-P0-*/brief.md`（台帳と同じ数）。
- HTML を作る道具は無い。`tools/` は PowerShell（`package.ps1`・`release-notes.ps1`・`test-all.ps1` ほか）。`tools/release-notes.ps1` が Markdown から文書を作る前例（`release-notes.strings.json` に文言を外出し）。

### doc フォルダ（全部で 119 ファイル・最終更新日は git の記録）

- 直下: `ROADMAP.md`（06-28・18 行・steering へのポインタだけ。`focus.md` の参照先に「旧パスのポインタ stub」として載る）・`discussion-260702-m1-roadmap.md` と `discussion-260702-concurrency-model.svg`・`discussion-260702-roadmap-tree.svg`（07-03・M1 期の議論）・`CONSTITUTION.md`（10-03）・`PASTA_PROFILE.md`（09-11）・`COMPAT_ARCHITECTURE.md`（10-10・設計判断の正本）・`anchor-compat.md`（10-15）・`choice-cascade-compat.md`（10-07）・`crates-io-publish.md`（10-06）・`emo2-conformance-scope.md`（10-10）・`impl-watch.md`（10-10）。
- フォルダ: `archive/` 36（10-03・旧 `ARCHITECTURE.md`・`spec_backup_20251101_082532` など）・`shiori/` 41（06-28 から更新なし）・`ukadoc-coverage/` 19（10-10・網羅台帳）・`ssp-mcp/` 8（10-11）・`release-notes/` 2（10-15）・`images/` 1。
- **参照の見つかったもの（消すと壊れる）**: `doc/shiori/equivalence_evidence.toml` は `crates/ukadoc-survey/src/evidence/resolve.rs` と網羅台帳 `doc/ukadoc-coverage/ledger/shiori.toml` が読む。`doc/ROADMAP.md`・`doc/shiori/`・`doc/archive/`・`discussion-260702-*` は、steering（`focus.md`・`roadmap.md`・`roadmap-history.md`・`workflow.md`）・スキル `kiro-complete`・`doc/anchor-compat.md`・`.kiro/specs/areka-P0-shiori4-api/brief.md` からも名前が出る。中身が今も効いているかは未確認。

## Desired Outcome

1. 開発者がブラウザで 1 枚の HTML を開くと、版の段ごとに spec が並び、依存の順・区分・テーマ・着手中（ウェーブ）・完了が見分けられる。
2. その HTML は roadmap.md から道具で作り直せる（手で直さない）。roadmap が動いたあとに作り直すのを忘れても、どこかで気付ける（議題 2）。
3. `doc/` の古い記載が片づき、残ったファイルはどれも「今も効いている」か「歴史として置く」と分かる。消したファイルを参照していた所（steering・スキル・コード・台帳）は 1 つも壊れない。

## Approach

**推す形**（要件の段で覆してよい）:

- **道具**: `tools/roadmap-overview.ps1`（PowerShell・依存を足さない）が roadmap.md の 4 か所を読み、自己完結の HTML（CSS と JS を中に入れる・外の CDN を読まない）を書く。文言は `release-notes.ps1` の前例どおり外へ出してよい。
- **見せ方**: 版の段を横の帯（左から v0.0.3 → … → いつか・待ち・版に縛らない）にし、帯の中で依存の順に spec のカードを並べる。カードは名前・一言・区分・規模・前提。テーマで色分け・絞り込み。着手中と完了の印。
- **置き場**: 作った HTML を `doc/roadmap/index.html` として入れるか、作るたびに手元だけに出すか（議題 1）。
- **doc の整理**: 1 ファイル（フォルダ）ずつ「参照されているか（grep）」「中身が今の正本と食い違うか」を表にし、要件の段で開発者が「消す・`archive/` へ寄せる・残す」を選ぶ。消すときは参照している側を同じ PR で直す。

**採らない方向（起票時）**:

- 手書きの HTML を 1 枚作って置く（roadmap が動くたびに古くなる＝「古い記載の doc を消す」と逆向き）。
- roadmap.md の書き方を HTML のために大きく変える（棚卸と `/kiro-discovery` が書く形はそのまま読む。読みにくい所は道具の側で吸収する）。

## Scope

- **In**: 俯瞰の HTML を作る道具とその決定論のテスト／HTML の置き場と作り直しの手順（`doc/` の説明・必要なら steering の 1 行）／`doc/` の古い記載の棚卸と、選ばれたものの削除・移動と参照側の直し。
- **Out**: roadmap.md の中身（版の段・台帳）の見直し（棚卸と `/kiro-discovery` の持ち物）／網羅台帳 `doc/ukadoc-coverage/` の数の直し（`coverage-roadmap-refresh`）／リリースノート（`/release`）／Web に公開すること（GitHub Pages など）。

## Boundary Candidates

- roadmap.md を読む所（4 か所の表と節を拾う・純粋な関数で検査できる形）。
- HTML を書く所（見せ方）。
- doc の棚卸の表と、消す・寄せる作業。

## Out of Boundary

- spec の版や順の決め直し。
- `doc/` の正本文書（`COMPAT_ARCHITECTURE.md` など）の中身の書き直し。

## Upstream / Downstream

- **Upstream**: 2026-10-15 の「版の段」節（俯瞰の背骨）。
- **Downstream**: 以後の棚卸と `/kiro-discovery`（roadmap を直したら HTML を作り直す手順が加わる）。

## Existing Spec Touchpoints

- **Extends**: なし。
- **Adjacent**: `coverage-roadmap-refresh`（`doc/ukadoc-coverage/` と roadmap の数）・`shiori4-api`（brief が `doc/shiori/` を指す）・スキル `kiro-complete`（`doc/` への参照を持つ）。

## Constraints

- 新しい依存を足さない（PowerShell と、ブラウザだけで開ける自己完結の HTML）。
- 道具が端末へ出す文は ASCII だけ（文字コードを変えて逃げない）。
- 作ったものを手で直さない（生成器があるなら手で直さない）。

## 議題（要件の段で決める）

1. 作った HTML をリポジトリに入れるか（入れるなら `doc/roadmap/index.html`・PR ごとの差分が大きくなる）、手元に出すだけにするか。
2. 作り直し忘れの気付き方（テストで「入れた HTML が今の roadmap から作ったものと同じ」を見る／棚卸の手順に書くだけ）。
3. `doc/` の整理の線引き（`archive/` を丸ごと消すか・`shiori/` の 41 本のうち参照の無いもの・M1 期の議論 3 本・`ROADMAP.md` のポインタ）。
