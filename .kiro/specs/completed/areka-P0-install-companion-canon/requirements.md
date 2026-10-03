# Requirements Document

> **2026-10-03・実施せず、3 本へ引き継いで閉じた（書きかけ・未承認のまま残す資料）**。要件の段で、同梱の「最初の 1 個」が areka に無い 2 つの機能（ゴーストの標準バルーン・シェルごとのバルーン）に触れると分かったため、開発者指示で分けた。要件 1〜4・6〜8 は `areka-P0-install-companion-reading`、要件 5 は `areka-P0-ghost-standard-balloon`、シェルに紐づくバルーンは `areka-P0-shell-companion-balloon` が引き継ぐ。引き継ぎの経緯は `.kiro/steering/roadmap.md`「同梱バルーンの正典」節。

## Project Description (Input)
- **困っている人**: ukadoc「Install設定」どおりに `install.txt` の同梱（`balloon*.directory` など）を書いたゴースト・シェルの作者と、その書庫を落とす利用者。
- **今の状況**: areka の `install.txt` の読み手は、欠番で探索を打ち切らず読まれないはずの番号まで入れ、宛先を接頭辞のバイト順に並べる。`*.source.directory` に書庫の中の階層を辿る相対パス（`extra\bal1` など）を書けず、`*.directory` のパス区切りや `*.source.directory` の `..` を含む値ではインストール全体を断る＝SSP で入る書庫が areka では `OnInstallFailure` で終わる。起動時に紐づく同梱バルーンは無印の `balloon.directory` だけを読み、番号付きだけで書いたゴーストでは「最初の 1 個」が紐づかない。
- **変えたいこと**: ⑴ 番号付きの同梱を無印→0→1→2…の順に探し、見つからない番号で打ち切る ⑵ `*.source.directory` は `\` と `/` の相対パスで書庫の中の階層を辿れる ⑶ `*.directory` のパス区切りは `_` に置き換える ⑷ `*.source.directory` の `..` は取り除く——を ukadoc どおりにし、読み替えたことを 1 件ずつ記録に残す。書庫の外へ書き出さない安全の検査は読み替えた後の値に今の強さのまま掛ける。起動時の同梱バルーンの選び方は答えに従って直すか理由を `doc/COMPAT_ARCHITECTURE.md` §8 に記す。網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の `descript_install` の行を実装に合わせる。詳細は同じフォルダの `brief.md`。

## Introduction

ゴーストやシェルの書庫には、`install.txt` の同時インストールの指定でバルーンを同梱できる。正典は ukadoc「Install設定」（https://ssp.shillest.net/ukadoc/manual/descript_install.html ）で、本 spec が依る記述は次の 4 点である。

- 「同時インストール」の節（番号の付け方）:「探索は無印→0→1→2…の順に行われ、見つからない番号が出た時点で打ち切られる。」「無印と0は別のものとして扱われる。」記述例は無印を書かずに `balloon0`・`balloon1` だけを書いている。
- `*.source.directory`（https://ssp.shillest.net/ukadoc/manual/descript_install.html#_2a.source.directory_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1 ）:「SSP 2.9.00以降は、extra\bal1 のようにアーカイブ内の階層を辿る相対パスも指定できる。」「区切りは「\」「/」のどちらでもよい。「..」による上位階層への参照はできない（取り除かれる）。」
- `*.directory`（https://ssp.shillest.net/ukadoc/manual/descript_install.html#_2a.directory_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1 ）:「ここに指定できるのは1階層のディレクトリ名だけで、パス区切りは使えない（「_」に置換される）。」
- 「同時インストール」の節（紐づくバルーン）: ゴーストやシェルに紐づくバルーンとして設定されるのは最初の 1 個だけで、2 個目以降はインストールされるだけでそのゴーストの標準バルーンにはならない。

今の areka（完了 spec `areka-P0-nar-install` の要件 3.9・3.12 の読み方）は、⑴ 欠番で探索を打ち切らず、`balloon` の直後が数字だけの接頭辞をすべて読み、接頭辞のバイト順（`balloon10` が `balloon2` より先）に並べる。⑵⑶⑷ パス区切りや `..` を含む `*.directory`・`*.source.directory` を 1 階層の名前の検査に掛けてインストール全体を断る。その結果、ukadoc どおりに書いた書庫が SSP では入り areka では `OnInstallFailure` で終わる、または SSP では入らない番号まで areka では入る。

本 spec は、この 4 点の読み方を ukadoc（2.9.00 以降の形）に揃える。読み替え（打ち切り・置き換え・取り除き）は入口だけで行い、書庫の外へ書き出さないための安全の検査は、読み替えた後の値に今の強さのまま掛ける。読み替えたことは 1 件ずつ記録に残す。α の検体 3 体の `install.txt` はどれも 4 点のどれも使っていない（`alpha-release-signoff` の受入記録 §8.8）ので、それらの入り方は変わらない。

## Boundary Context

- **In scope**:
  - `type` が `ghost`・`shell` の書庫の同時インストールのバルーン（無印の `balloon` と `balloonN`）について、探索の順・打ち切り・並び、`*.source.directory` の階層付きの値と `..` の取り除き、`*.directory` のパス区切りの置き換え。
  - 読み替えの記録と、読み替えた後の値への安全の検査。
  - 起動時に紐づく同梱バルーンの名前を、インストールが実際に作ったフォルダ名と同じ読み方で引くこと（Requirement 5）。
  - 完了 spec の読み方を上書きすることと、正典が沈黙する点で areka が決めたことの `doc/COMPAT_ARCHITECTURE.md` §8 への記録、網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の `descript_install` の該当の行の更新。
  - 書庫を組む既存の検体の道具を使った決定論の自動テスト。
- **Out of scope**:
  - 本体の `directory` の扱い（ukadoc は区切りについて書いていない＝今どおりパス区切りや `..` を含む値は断る）。
  - 2.9.00 より前の形（`*.source.directory` の区切りを `_` に置き換えて 1 階層の名前として扱う）。本 spec は今の ukadoc が本文に書く 2.9.00 以降の形に揃える。
  - `type,package` と `developer_options.txt`（登記だけの行「配布物を束ねる／作る側の 3 件」）。
  - 同梱の種類を増やすこと（`headline`・`plugin`・`calendar.skin`・`calendar.plugin` は今どおり読み飛ばして記録する）。
  - 2 つの同梱が同じ宛先を指す場合の扱い（今の扱いを変えない）。
  - 使用中のフォルダへの上書き（`install-live-target-hazards`）。
  - 書庫の安全な展開の規則（シンボリックリンク・絶対パス・`..`・長さの上限 200 などのエントリ名の検査）と、確定の手順（完了 `areka-P0-nar-install`・`areka-P0-nar-install-hardening`）。
- **Adjacent expectations**:
  - インストールの手続き（完了 `areka-P0-ghost-install`）は、NAR エンジンが返す「何をインストールしたか」の要素の列の順をそのまま `OnInstallComplete`・`OnInstallCompleteEx` の Reference に写す。本 spec は列の順を ukadoc の探索の順に変え、手続きの側の写し方は変えない。
  - 書庫の中の利用条件（`terms.txt`／`terms.md`）の扱いは完了 `areka-P0-ghost-install` の決めのまま。同梱バルーンの取り出し元の中に在る利用条件を「出さずに記録する」ときの取り出し元は、本 spec の階層付きの値に従う。
  - pasta の側の `pasta-check-bundled-balloon`（本リポジトリの外）は ukadoc に従って書くので、本 spec の着地で areka でも SSP と同じ結果になる前提で進める。

## Requirements

### Requirement 1: 番号付きの同梱を ukadoc の順で探し、欠番で打ち切る

**Objective:** As a ゴースト・シェルの作者, I want 同梱のバルーンが ukadoc どおりの順で探され、欠番の後ろは読まれないこと, so that SSP と areka で同じバルーンが同じ順で入る

#### Acceptance Criteria

1. While 書庫の `type` が `ghost` または `shell` である, the NAR エンジン shall 同時インストールのバルーンを、無印（`balloon`）→ `balloon0` → `balloon1` → `balloon2` … の順に探す。
2. The NAR エンジン shall 番号 N のバルーンを「見つかった」とみなすのは、`balloonN.directory` の行が書かれているときに限る。
3. When 番号 N のバルーンが見つからなかったとき, the NAR エンジン shall そこで探索を打ち切り、N より大きい番号のバルーンを入れない。
4. When 無印のバルーンが見つからなかったとき, the NAR エンジン shall 打ち切らずに `balloon0` から探索を続ける（無印は番号ではない。ukadoc の記述例は無印を書かずに `balloon0`・`balloon1` だけを書く）。
5. When 無印と `balloon0` の両方が書かれているとき, the NAR エンジン shall 両方を別のバルーンとして入れる（ukadoc「無印と0は別のものとして扱われる。」）。
6. The NAR エンジン shall 探索で数える番号を、先頭に 0 を付けない 10 進の綴り（`0`・`1`・`2`…`10`…）だけとし、`balloon01`・`balloon00` のような綴りの鍵は入れない。
7. The NAR エンジン shall 入れた同梱のバルーンを、「何をインストールしたか」の要素の列の中で本体の後ろに探索の順で並べる（例: `balloon0`〜`balloon10` が全て書かれていれば `balloon2` は `balloon10` より前）。これにより `OnInstallComplete` の Reference2（最初の同梱バルーンの名前）は探索で最初に見つかったバルーンになり、`OnInstallCompleteEx` の各 Reference の並びも探索の順になる。
8. If 探索で入れなかった同梱のバルーンの鍵（打ち切りの後ろの番号・先頭に 0 を付けた綴り）が書かれているとき, then the NAR エンジン shall その鍵ごとに 1 件、「探索で読まなかった」ことを、知らない鍵の読み飛ばしと区別できる形で記録し、本体と他の同梱のインストールは続ける。
9. If 無印の `balloon.directory` または番号 N の `balloonN.directory` が書かれていないのに、同じ接頭辞の他の鍵（`balloonN.source.directory` など）が書かれているとき, then the NAR エンジン shall そのバルーンを見つからなかったものとして扱い（番号 N なら 3 項のとおり打ち切り、無印なら 4 項のとおり続ける）、その鍵は今どおり宛先の無い断片として読み飛ばして記録する。
10. The NAR エンジン shall `headline`・`plugin`・`calendar.skin`・`calendar.plugin` の同梱と、`type` が `balloon`・`supplement` の書庫に書かれた同梱の扱い（読み飛ばして記録する）を変えない。

### Requirement 2: `*.source.directory` で書庫の中の階層を辿る

**Objective:** As a ゴースト・シェルの作者, I want `*.source.directory` に `extra\bal1` のような相対パスを書けること, so that 同梱のバルーンを書庫の中の深いフォルダに置いた書庫も、SSP 2.9.00 以降と同じく入る

#### Acceptance Criteria

1. When 同梱のバルーンの `*.source.directory` の値が `\` または `/` を含むとき, the NAR エンジン shall その値を、`\` と `/` のどちらでも区切る書庫の中の相対パスとして読み、インストール全体を断らない。
2. When 取り出し元が相対パスで書かれているとき, the NAR エンジン shall 書庫の中でそのパスの配下に在るファイルとフォルダを、そのパスの全段を剥がした相対の位置のまま `<根>/balloon/<*.directory>/` へ置く。
3. The NAR エンジン shall 取り出し元のパスの各段を、今の 1 階層の取り出し元と同じく ASCII の大小を無視して書庫の中の名前と突き合わせる。
4. The NAR エンジン shall 取り出し元のパスの配下に在るファイルとフォルダを本体の側に複製せず、配下でないもの（例: 取り出し元が `extra/bal1` のときの `extra/readme.txt`）は今どおり本体の側に置く。
5. If 取り出し元のパスの配下に書庫のファイルもフォルダも 1 つも無いとき, then the NAR エンジン shall 今どおりインストール全体を断り、理由に鍵と取り出し元の値を載せる。
6. When インストールの手続きが同梱のバルーンの取り出し元の中の利用条件を調べるとき, the areka shall その利用条件（`terms.txt`／`terms.md`）を、1 階層の取り出し元のときと同じく取り出し元のフォルダの直下で探し、出さずに記録する。
7. The NAR エンジン shall `*.source.directory` が無い（または空の）ときに `*.directory` と同じ名前を取り出し元とする今の扱いを変えない。

### Requirement 3: `*.source.directory` の `..` と空の段を取り除く

**Objective:** As a ゴースト・シェルの作者, I want `*.source.directory` に紛れた `..` で書庫全体が断られないこと, so that ukadoc どおり `..` を取り除いた位置から取り出される

#### Acceptance Criteria

1. When 同梱のバルーンの `*.source.directory` の値に `..` の段が含まれるとき, the NAR エンジン shall その段を取り除き、残った段だけで取り出し元を決める（ukadoc「「..」による上位階層への参照はできない（取り除かれる）。」）。上位の階層へは決して辿らない。
2. When 値の先頭・末尾・連続した区切りによって空の段ができるとき, the NAR エンジン shall 空の段を取り除いて取り出し元を決める。
3. If 取り除いた後に段が 1 つも残らないとき, then the NAR エンジン shall インストール全体を断り、理由に鍵と書かれていた値を載せる（書庫の全体を取り出し元にしない。ukadoc はこの場合に沈黙しており、本 spec の決め）。
4. When 本要件の取り除きで値が変わったとき, the NAR エンジン shall 鍵ごとに 1 件、書かれていた値と取り除いた後の値を記録する。

### Requirement 4: `*.directory` のパス区切りを `_` に置き換える

**Objective:** As a ゴースト・シェルの作者, I want `*.directory` に紛れたパス区切りで書庫全体が断られないこと, so that ukadoc どおり `_` に置き換えた 1 階層のフォルダ名で入る

#### Acceptance Criteria

1. When 同梱のバルーンの `*.directory` の値が `\` または `/` を含むとき, the NAR エンジン shall それぞれの区切りを `_` に置き換えた名前を宛先のフォルダ名（`<根>/balloon/<置き換えた名前>/`）とし、インストール全体を断らない。
2. The NAR エンジン shall 置き換えた名前を、「何をインストールしたか」の要素の列と、`*.source.directory` が無いときの取り出し元の名前にも使う。
3. When 置き換えで値が変わったとき, the NAR エンジン shall 鍵ごとに 1 件、書かれていた値と置き換えた後の値を記録する。
4. The NAR エンジン shall `*.directory` の `..` を取り除く読み替えは行わない（ukadoc が書くのは区切りの置き換えだけ）。

### Requirement 5: 起動時に紐づく同梱バルーンの名前の読み方（一部未決）

**Objective:** As a 利用者, I want インストールで入った同梱のバルーンが、起動時にもそのゴーストのバルーンとして見つかること, so that 区切りを置き換えた名前で入ったバルーンでも、起動時に別のバルーンへ取り違えられない

#### Acceptance Criteria

1. When 起動時にゴーストの `install.txt` から同梱のバルーンの名前を引くとき, the areka shall `*.directory` の値をインストールと同じ規則（Requirement 4 のパス区切りの `_` への置き換え）で読んだ名前で、インストール済みのバルーンを探す。

> **未決（要件ディスカッションで開発者に確かめる）**: ukadoc「同時インストール」は「ゴーストやシェルに紐づくバルーンとして設定されるのは最初の1個だけである。」と書く。今の areka は起動時に無印の `balloon.directory` だけを読み、番号付きだけで書いたゴースト（α の検体 `claudia` は `balloon0`・`balloon1`）では `balloon0` が紐づかずに次の段（唯一・既定・無作為）へ進む。(a) 無印が無ければ `balloon0`（＝Requirement 1 の探索で最初に見つかった 1 個）を紐づけるか、(b) シェルの書庫の `install.txt` が同梱したバルーンをそのシェルに紐づけるか、の答えで本要件の残りの受入基準が決まる。今のまま据え置く場合は理由を `doc/COMPAT_ARCHITECTURE.md` §8 に記す（Requirement 7）。

### Requirement 6: 読み替えた後も安全の検査を今の強さのまま掛ける

**Objective:** As a 利用者, I want 読み替えを足しても、書庫の外へ書き出されたり使えない名前のフォルダが作られたりしないこと, so that 第三者の書庫を落としても根の外が書き換わらない

#### Acceptance Criteria

1. The NAR エンジン shall `*.directory` の置き換えた後の名前に、今の 1 階層の名前の検査（空でない・Windows で使えない文字を含まない・末尾がドットや空白でない・予約名でない・長さが UTF-16 で 200 単位以内）を掛け、満たさなければ今どおりインストール全体を断る（例: 置き換えた結果が `CON` のような予約名・200 単位を超える名前）。
2. The NAR エンジン shall `*.source.directory` の取り除いた後の各段に今の 1 階層の名前の検査を掛け、パス全体の長さにも今の上限（UTF-16 で 200 単位）を掛け、満たさなければ今どおりインストール全体を断る（例: `.` の段・`C:` を含む段）。
3. If 本仕様のいずれかの理由でインストール全体を断るとき, then the NAR エンジン shall 今どおり書き込みの前に止め、宛先に 1 バイトも書かず、理由を記録する。
4. The NAR エンジン shall 読み替えの有無にかかわらず、同梱のバルーンのファイルとフォルダを `<根>/balloon/<宛先のフォルダ名>/` の外へ置かない。
5. The NAR エンジン shall 同じ `install.txt` からは、同じ同梱の列・同じ記録の列（件数と並び）を必ず返す。
6. The NAR エンジン shall 同梱の番号に欠番も先頭の 0 も `balloon10` 以上の番号も無く、`*.directory`・`*.source.directory` に区切りも `..` も含まない `install.txt` の書庫（α の検体 3 体を含む）を、今と同じ宛先・同じファイル・同じ並び・同じ記録で入れる。

### Requirement 7: 正典との対応の記録と網羅台帳

**Objective:** As a areka の開発者, I want 読み方を変えたことと areka が決めたことが文書に残ること, so that 後から完了 spec の要件と実装の違いを辿れる

#### Acceptance Criteria

1. The `doc/COMPAT_ARCHITECTURE.md` §8 shall 完了 `areka-P0-nar-install` の要件 3.9（同梱の `*.directory`・`*.source.directory` を 1 階層の名前に限る）と要件 3.12（`balloonN` の N は 0 以上の整数をすべて読む）の読み方を本 spec が上書きしたことを記す行を持つ。
2. The `doc/COMPAT_ARCHITECTURE.md` §8 shall 正典が沈黙する点で本 spec が決めたこと（取り除いた後に段が残らない取り出し元は断る・先頭に 0 を付けた番号は数えない・`*.source.directory` が無いときの取り出し元は置き換えた後の `*.directory` の名前とする・2.9.00 以降の形を採る）を記す行を持つ。
3. The 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` shall `descript_install` の `*.directory` と `*.source.directory` の行（およびそれらの文面が述べる同梱の読み方）で、本 spec の実装と記録の仕方を述べる。

### Requirement 8: 決定論の自動テスト

**Objective:** As a areka の開発者, I want 4 点の読み替えと安全の検査が自動テストで確かめられること, so that 後の変更で正典との違いが黙って戻らない

#### Acceptance Criteria

1. The 自動テスト shall 書庫を組む既存の検体の道具で組んだ書庫で、次の各場面を決定論的に確かめる: 欠番での打ち切り（例: `balloon0` と `balloon2` だけ）・無印が無い番号付きだけ・無印と `balloon0` の両方・`balloon2` と `balloon10` の並び・先頭に 0 を付けた番号・`\` と `/` の階層付きの取り出し元・取り出し元の配下でない兄弟のファイルが本体に残ること・`..` と空の段の取り除き・取り除いた後に段が残らない場合・`*.directory` の区切りの置き換え・置き換えた結果が予約名や長すぎる名前になる場合・起動時の同梱バルーンの名前の読み方。
2. The 自動テスト shall 各場面で、置かれたファイルとフォルダの全てが根の中の宛先の配下に在ることと、記録の件数と中身を確かめる。
3. The 自動テスト shall 検体と一時フォルダをワークツリーの `target\` の下だけに作る。
