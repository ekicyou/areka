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
