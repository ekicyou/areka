# Brief: doc-tree-tidy

## Problem

- **開発者**（2026-10-28 `/kiro-discovery`・議題 10 の 2）: 「doc と docs があり、また、doc も結構下記散らかされていて困ってる。どこかで整理する spec が必要かも」「このspecだけは0.1.0の直後、doraとかの基盤整備に置きます」。
- **後のセッション**: 文書をどこに置けばよいか迷い、置いた後も案内の表に載らないまま散らかる。

## Current State

起票のときの実測（main `d6054e4d`）:

- **`docs/` は 1 つだけ**: `docs/click_through.md`（wintf のクリック透過の仕組みの説明）。完了済みの spec がおよそ 10 本、この場所を名指す（`wintf-clickthrough-alpha-toggle`・`areka-P0-wintf-drag-state-rest-contract`・`areka-P0-roadmap-overview-html` の `doc-inventory.md` など）。
- **`doc/` は直下に 12 のファイルと 7 つのフォルダ・全部で 120 余りのファイル**（2026-10-28 に `ideas/` を足した）。
  - 名前の書き方が混ざる: 大文字の `CONSTITUTION.md`・`COMPAT_ARCHITECTURE.md`・`PASTA_PROFILE.md` と、小文字のハイフンつなぎの `impl-watch.md`・`pasta-real-run.md` など。
  - 案内は `doc/README.md`。決まりは「歴史は `doc/archive/` の下だけ」「足す・動かすときは案内の表に 1 行足す」だが、「決まりは書くだけで、機械の検査は無い」。
- **前の片付け**: 完了 `areka-P0-roadmap-overview-html` が `doc-inventory.md`（doc の棚卸）を作り、今の案内の決まりができた。`docs/` はそのとき残った。

## Desired Outcome

- 文書の置き場が 1 つ（`doc/`）にまとまり、置き方の決まりが案内に書かれている。
- ファイル名の書き方がそろっている。
- 案内の表と実物が合っていることを、試験が確かめる（表に無いファイル・表にあるのに無いファイルを見つける）。散らかりが戻らない。
- 動かしたパスを名指すコード・spec・スキル・steering が、どれも新しいパスを指す。

## Approach

- 置き場は `doc/` にまとめる。`docs/click_through.md` は `doc/` へ移すか、wintf の利用者向けなら wintf のクレートの中へ移すかを要件の段で決める（wintf は crates.io へ出すクレート・`doc/crates-io-publish.md`）。
- ファイル名の書き方は案内の決まりに 1 つ書き、それに合わせて名前を変える。
- 案内の表と実物の突き合わせは、決定論の試験 1 本で押さえる。
- 動かしたパスは、ソース全域を grep して直す（完了 spec のアーカイブで、コードから spec の文書を読む所が壊れた前例がある）。完了済みの spec の中の古いパスを歴史として残すか直すかは、要件の段で決める。

## Scope

- **In**: `docs/` の始末・`doc/` の直下とフォルダの棚卸・ファイル名の書き方の決まりと改名・案内の表の更新・案内と実物の突き合わせの試験・動かしたパスの参照の直し（コード・テスト・スキル・steering・未着手と着手中の spec）。
- **Out**: 文書の中身の書き直し（置き場と名前だけ）・`.kiro/` の下の整理・`doc/ukadoc-coverage/` の生成物の作り方・根の `README.md` の中身。

## Boundary Candidates

- 案内（`doc/README.md`）と置き方の決まり。
- 改名と移動、その参照の直し。
- 突き合わせの試験（どのクレートのテストに置くか）。

## Out of Boundary

- `doc/archive/` の中の歴史の文書の中身。
- 外のリポジトリの文書。

## Upstream / Downstream

- **Upstream**: 無し。
- **Downstream**: 後から文書を足す spec のすべて（置き場と名前の決まりに従う）。

## Existing Spec Touchpoints

- **Extends**: 完了 `areka-P0-roadmap-overview-html` が作った案内の決まりに、機械の検査を足す。
- **Adjacent**: `coverage-roadmap-refresh`・`ukadoc-catalog-refresh`（`doc/ukadoc-coverage/` を触る）・`shiori4-api`（`doc/shiori/fragments/` の封筒を書き換える）。

## Constraints

- 区分 A（開発者が起票）・版は **v0.2.0**（開発者「このspecだけは0.1.0の直後、doraとかの基盤整備に置きます」）。
- パスを動かすので、ほかの spec の brief や設計とぶつかりやすい。v0.2.0 の中でも、ほかの spec の設計が文書のパスを名指す前の早いウェーブに、1 本だけで走らせる。着手の前に、そのとき動いている spec の触るファイルと突き合わせる。
- 規模の見立て: S〜M（6〜10 タスク）。Opus。

## 2026-11-04 棚卸㉞の再測定（main `932406f9`）

- **前提の変化**: `doc/` の直下のファイルは 13（11-01 に v0.1.0 の版の門の確かめの記録 `doc/v0.1.0-gate-real-run.md` が加わった）・フォルダは 7 のまま・`doc/` と `docs/` を合わせて 129 ファイル。ほかに増えたのは `doc/release-notes/v0.1.0.md`。どれも案内の表 `doc/README.md` に載っている。直下で表に名前のとおりには載らないのは日付つきの俯瞰 `doc/roadmap-overview-2026-10-16.html` だけ（表は `{YYYY-MM-DD}` の型で載せる＝突き合わせの試験はこの型を読む）。`docs/click_through.md` を名指すコードは 0（steering の歴史とこの brief だけ）。
- **触るファイル**: `doc/` の外で `doc/`・`docs/` のパスを書くのは、crates の 91 ファイル（ukadoc-survey 44・areka-mcp 16・crates/areka 12〔うち `src/placement/` 2＝`persist.rs`・`resolver.rs`〕・areka-emo-text 9・areka-impl-watch 5・areka-sakura 2・areka-ghost・areka-sylphya・real-run-kit 各 1）・`tools/release-notes.ps1`・`.github/workflows/` 2・スキル 9・steering 6・未完了の spec 96・根の `README.md` と `CLAUDE.md`。ファイルを実際に開くのは ukadoc-survey（`doc/ukadoc-coverage/`）・areka-mcp と crates/areka の試験（`doc/ssp-mcp/` の 3 ファイル）・リリースの道具と流れ（`doc/release-notes/`）で、ほかはコメントの中のパス。いちばん多く名指されるのは `doc/COMPAT_ARCHITECTURE.md`（crates の 18 ファイル）。
- **規模**: S〜M（6〜10）のまま。大文字の 3 ファイルを改名すると、`COMPAT_ARCHITECTURE.md` の分だけでコード 18 ファイルと steering・spec の多くを直す。
- **先に要るもの（見直しと crate 分けを含む）**: 働きの上の前提は無い。重なる見直しの spec は、要件の段で決める改名の範囲で決まる＝`COMPAT_ARCHITECTURE.md` を改名するなら 9 本（`audit-emo-text-layout-draw`・`audit-emo-text-state-actor`・`audit-parsers-sakura`・`audit-sylphya-actor`・`audit-ghost-catalog`・`audit-areka-app`・`audit-areka-install-update`・`audit-placement-core`・`audit-emo2-boot-switch`）、`doc/ssp-mcp/` を動かすなら 5 本（`audit-mcp-update-crates`・`audit-parsers-sakura`・`audit-areka-install-update`・`audit-emo2-boot-spine`・`audit-areka-app`）、`impl-watch.md` なら `audit-impl-watch`、`testing.md` なら `audit-test-kits`、`release-notes/` なら `audit-tools-scripts`、`docs/` だけなら 0 本。本体の crate 分けの 6 本はどれも `doc/COMPAT_ARCHITECTURE.md`・網羅台帳の根拠の場所・`structure.md` を書き換えるので、`placement-crate-split` も含めて同じウェーブに置かない（`placement-crate-split` の brief にも書いてある）。
- **優先度の区分**: A（開発者の起票・「このspecだけは0.1.0の直後」）。
- **要件定義のモデル**: Opus。
- **分割の案**: 無し。
- **見つけた穴・古くなった記述**: Current State の「直下に 12 のファイル」「全部で 120 余り」は、今は 13 と 129。
