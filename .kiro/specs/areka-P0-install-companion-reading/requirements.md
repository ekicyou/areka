# Requirements Document

## Project Description (Input)

- **困っている人**: ukadoc「Install設定」どおりに `install.txt` の同時インストールの指定（`balloon*.directory` など）を書いたゴースト・シェルの作者と、その書庫を落とす利用者。
- **今の状況**: areka の `install.txt` の読み方は、次の 3 点で ukadoc と違う。
  - 番号付きの同梱を欠番で打ち切らず、読まれないはずの番号まで入れる。並びは接頭辞の文字の順で、`balloon10` が `balloon2` より先に来る。
  - `*.source.directory` に、書庫の中の階層を辿る相対パス（`extra\bal1` など）を書けない（インストール全体を断る）。
  - `*.source.directory` に `..` を含む値で、インストール全体を断る。
  - その結果、SSP で入る書庫が areka では `OnInstallFailure` で終わる、または SSP では入らない番号まで areka では入る。
- **変えたいこと**: 3 点を ukadoc どおりに読む。
  - 番号は無印 → 0 → 1 → 2… の順に探し、見つからない番号で打ち切る。入る順と知らせの並びもこの順になる。
  - `*.source.directory` は `\` と `/` の相対パスで書庫の中の階層を辿れる。`..` は取り除く。
  - `*.directory` のパス区切りは、ukadoc の「パス区切りは使えない」のとおり今どおり断る。`_` への置き換えは採らない（2026-10-04 の討議で決めた。brief の ⑶ を覆す）。
  - 読み替えたことは 1 件ずつ記録に残す。安全の検査は、読み替えた後の値に今の強さのまま掛ける。
  - 網羅台帳と `doc/COMPAT_ARCHITECTURE.md` §8 を実装に合わせる。
- 詳細は同じフォルダの `brief.md`。前身は `completed/areka-P0-install-companion-canon/`（実施せず 3 本へ分けた 1 本目が本 spec）。

## Introduction

ゴーストやシェルの書庫には、`install.txt` の同時インストールの指定でバルーンを同梱できる。正典は ukadoc「Install設定」（https://ssp.shillest.net/ukadoc/manual/descript_install.html ）で、本 spec が依る記述は次の 3 か所である（2026-10-04 に引き直した）。

- **「同時インストール」の節（番号の付け方の説明）**: 探索は「無印→0→1→2…の順に行われ」、「見つからない番号が出た時点で打ち切られる」。「無印と0は別のものとして扱われる」。記述例は無印を書かずに `balloon0.directory`・`balloon0.source.directory`・`balloon1.directory`・`balloon1.source.directory`・`plugin0.directory` を書く。同梱できる種別の接頭辞は `balloon`・`headline`・`plugin`・`calendar.skin`・`calendar.plugin`。
- **`*.directory` の項目の定義**（https://ssp.shillest.net/ukadoc/manual/descript_install.html#_2a.directory_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1 ）:「ここに指定できるのは1階層のディレクトリ名だけで、パス区切りは使えない（「_」に置換される）。」
- **`*.source.directory` の項目の定義**（https://ssp.shillest.net/ukadoc/manual/descript_install.html#_2a.source.directory_2c_30c7_30a3_30ec_30af_30c8_30ea_540d:1 ）:「SSP 2.9.00以降は、extra\bal1 のようにアーカイブ内の階層を辿る相対パスも指定できる。」「区切りは「\」「/」のどちらでもよい。「..」による上位階層への参照はできない（取り除かれる）。」「2.9.00未満では区切りが「_」に置換されるため、1階層のディレクトリ名しか指定できない。」省略したときの値は「*.directoryで設定したディレクトリ名」。

ukadoc が書いていないこと（本 spec が決める）は次の 3 点である。

- 番号が「見つかった」と判定する鍵（本 spec は `*.directory` の行とする）。
- 先頭に 0 を付けた番号の綴り（`balloon01`）の扱い（本 spec は読まない）。
- `..` と空の段を取り除いた後に何も残らない取り出し元の扱い（本 spec は断る）。

`*.directory` の項目の定義のうち、本 spec が採るのは「パス区切りは使えない」の方である。括弧の中の「`_` に置換される」は、使えない値を書いたときの SSP の後始末であり、areka は真似ずに今どおり断る（2026-10-04 の討議）。

本 spec は、読み替え（打ち切り・取り除き）を入口だけで行い、書庫の外へ書き出さないための安全の検査は、読み替えた後の値に今の強さのまま掛ける。α の検体 3 体の `install.txt` は 3 点のどれも使っていないので、それらの入り方は変わらない。

## Boundary Context

- **In scope**:
  - `type` が `ghost`・`shell` の書庫の、同時インストールのバルーン（無印の `balloon` と `balloonN`）について:
    - 探索の順・欠番での打ち切り・入る順と知らせの並び。
    - `*.source.directory` の階層付きの値と、`..`・空の段の取り除き。
  - 読み替えの記録と、読み替えた後の値への安全の検査。
  - 取り出し元を使う他の 2 か所（同梱バルーンの中の利用条件を探す所・検体をインストール済みの形へ畳む開発用の道具）の追随。
  - `doc/COMPAT_ARCHITECTURE.md` §8 と網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` の更新。
  - 決定論の自動テスト。
- **Out of scope**（どれも今の動きを変えない＝変更 0）:
  - 起動時にどのバルーンを使うか（`areka-P0-ghost-standard-balloon`）。本 spec は起動時の読み方を 1 行も変えない。
  - シェルに紐づくバルーン（`areka-P0-shell-companion-balloon`）。
  - 同梱の `*.directory` の扱い。区切りを含む値は今どおり断り、`_` への置き換えは採らない。
  - 本体の `directory` の扱い。ukadoc の `directory` の項目は区切りについて書いていないので、今の「区切りや `..` を含む値は断る」を変えない。
  - SSP 2.9.00 より前の形（`*.source.directory` の区切りを `_` に置き換える）は採らない。
  - 同梱の種類を増やすこと。`headline`・`plugin`・`calendar.skin`・`calendar.plugin` は今どおり読み飛ばして記録する。
  - `type` が `balloon`・`supplement` の書庫に書かれた同時インストールの指定。ukadoc は「アーカイブ自身のtypeを問わず機能する」と書くが、今どおり読み飛ばして記録する。
  - 2 つの同梱が同じ宛先を指す場合の扱い（同じ `*.directory` の値を 2 つ書いた場合。本 spec はこの場面を増やさない）。
  - `*.directory` の値が空のときの扱い。
  - `type,package` と `developer_options.txt`。
  - 使用中のフォルダへの上書き（`areka-P0-install-live-target-hazards`）。
  - 書庫の中の名前の安全の規則（シンボリックリンク・絶対パス・`..`・長さの上限 200）と、確定の手順（完了 `areka-P0-nar-install`・`areka-P0-nar-install-hardening`）。
- **Adjacent expectations**:
  - インストールの手続き（完了 `areka-P0-ghost-install`）は、「何をインストールしたか」の列の順をそのまま `OnInstallComplete`・`OnInstallCompleteEx` の Reference に写す。本 spec は列の順を変え、手続きの側の写し方は変えない（変更 0）。
  - `areka-P0-ghost-standard-balloon` は、本 spec の探索の順で最初に見つかった同梱を「最初の 1 個」とする。`*.directory` の値は読み替えないので、起動の側と共有する規則は無い（0 個）。
  - `areka-P0-install-live-target-hazards` と同じインストールの手続きの周りを触る。本 spec が手続きの側で触るのは利用条件を探す所だけである。
  - pasta の側の `pasta-check-bundled-balloon`（本リポジトリの外）は ukadoc に従って書くので、本 spec の着地の後は areka でも ukadoc どおりの結果になる前提で進められる。

## Requirements

### Requirement 1: 番号付きの同梱を ukadoc の順で探し、欠番で打ち切る

**Objective:** As a ゴースト・シェルの作者, I want 同梱のバルーンが ukadoc どおりの順で探され、欠番の後ろは読まれないこと, so that ukadoc どおりに書いた書庫から、同じバルーンが同じ順で入る

#### Acceptance Criteria

1. While 書庫の `type` が `ghost` または `shell` である, the NAR エンジン shall 同時インストールのバルーンを、無印（`balloon`）→ `balloon0` → `balloon1` → `balloon2` … の順に探す。
2. The NAR エンジン shall バルーンを「見つかった」とみなすのを、その接頭辞の `*.directory` の行（無印なら `balloon.directory`、番号 N なら `balloonN.directory`）が書かれているときに限る。値が空でも行が在れば「見つかった」とし、今どおり Requirement 5 の検査でインストール全体を断る（空の値の扱いは変更 0）。
3. When 番号 N のバルーンが見つからなかったとき, the NAR エンジン shall そこで探索を打ち切り、N より大きい番号のバルーンを入れない（例: `balloon0` と `balloon2` だけが書かれていれば、入るのは `balloon0` だけ）。
4. When 無印のバルーンが見つからなかったとき, the NAR エンジン shall 打ち切らずに `balloon0` から探索を続ける。
5. When 無印と `balloon0` の両方が見つかったとき, the NAR エンジン shall 両方を別のバルーンとして入れる。
6. The NAR エンジン shall 探索で数える番号を、先頭に 0 を付けない 10 進の綴り（`0`・`1`・`2`…`10`…）だけとし、`balloon01`・`balloon00` のような綴りの鍵のバルーンは入れない。
7. The NAR エンジン shall 入れた同梱のバルーンを、「何をインストールしたか」の列の中で、本体の後ろに探索の順で並べる（例: `balloon0`〜`balloon10` が全て書かれていれば、`balloon2` は `balloon10` より前）。
8. When インストールが成功したとき, the areka shall `OnInstallComplete` の Reference2（最初の同梱の名前）に探索で最初に見つかったバルーンを、`OnInstallCompleteEx` の各 Reference に探索の順の並びを載せる。
9. If 接頭辞の `*.directory` の行が無いのに、同じ接頭辞の他の鍵（`balloonN.source.directory` など）だけが書かれているとき, then the NAR エンジン shall そのバルーンを見つからなかったものとして扱う（番号 N なら 3 項のとおり打ち切り、無印なら 4 項のとおり続ける）。
10. The NAR エンジン shall `headline`・`plugin`・`calendar.skin`・`calendar.plugin` の同梱と、`type` が `balloon`・`supplement` の書庫に書かれた同梱の扱い（読み飛ばして記録する）を変えない。

### Requirement 2: `*.source.directory` で書庫の中の階層を辿る

**Objective:** As a ゴースト・シェルの作者, I want `*.source.directory` に `extra\bal1` のような相対パスを書けること, so that 同梱のバルーンを書庫の中の深いフォルダに置いた書庫も入る

#### Acceptance Criteria

1. When 同梱のバルーンの `*.source.directory` の値が `\` または `/` を含むとき, the NAR エンジン shall その値を、`\` と `/` のどちらでも区切る書庫の中の相対パスとして読み、そのことを理由にインストール全体を断らない。
2. When 取り出し元が 2 段以上の相対パスであるとき, the NAR エンジン shall 書庫の中でそのパスの配下に在るファイルとフォルダを、そのパスの全段を剥がした相対の位置のまま、バルーンの置き場の宛先のフォルダ（`<根>/balloon/<宛先のフォルダ名>/`）へ置く。
3. The NAR エンジン shall 取り出し元のパスの各段を、今の 1 階層の取り出し元と同じく、ASCII の大小を無視して書庫の中の名前と突き合わせる。
4. The NAR エンジン shall 取り出し元のパスの配下に在るファイルとフォルダを本体の側に置かず、配下でないもの（例: 取り出し元が `extra/bal1` のときの `extra/readme.txt`）を今どおり本体の側に置く。取り出し元の途中のフォルダ（例の `extra/`）は、その配下に本体の側へ置くものが 1 つも無ければ本体の側に作らない（書庫がフォルダのエントリを持つかどうかで結果を変えない）。
5. If 取り出し元のパスの配下に、書庫のファイルもフォルダも 1 つも無いとき, then the NAR エンジン shall 今どおりインストール全体を断り、理由に鍵と取り出し元の値を載せる。
6. When `*.source.directory` の行が無い、または値が空であるとき, the NAR エンジン shall 宛先のフォルダ名（`*.directory` の値そのまま）を取り出し元とする。
7. When インストールの手続きが同梱のバルーンの取り出し元の中の利用条件（`terms.txt`／`terms.md`）を調べるとき, the areka shall 取り出し元が階層付きでも、1 階層のときと同じく取り出し元のフォルダの直下で探し、出さずに記録する。
8. When 検体をインストール済みの形へ畳む開発用の道具が、階層付きの取り出し元を持つ検体を畳むとき, the 開発用の道具 shall インストールと同じ置き場所（2 項・4 項）へ置く。
9. When 2 つの同梱の取り出し元が重なるとき（例: `extra` と `extra/bal1`）, the NAR エンジン shall それぞれの同梱に、自分の取り出し元の配下の全てを置く（今の「同じ取り出し元を 2 つの同梱が指す」場合と同じく、重なった部分は両方へ入る）。

### Requirement 3: `*.source.directory` の `..` と空の段を取り除く

**Objective:** As a ゴースト・シェルの作者, I want `*.source.directory` に紛れた `..` で書庫全体が断られないこと, so that ukadoc どおり `..` を取り除いた位置から取り出される

#### Acceptance Criteria

1. When 同梱のバルーンの `*.source.directory` の値に `..` の段が含まれるとき, the NAR エンジン shall その段を取り除き、残った段だけで取り出し元を決める（例: `../extra/bal1` は `extra/bal1`）。
2. The NAR エンジン shall `..` の段を、手前の段を打ち消すものとしては読まず（例: `extra/../bal1` は `extra/bal1`）、取り出し元を書庫の外や上位の階層へ辿らせない。
3. When 値の先頭・末尾・連続した区切りによって空の段ができるとき, the NAR エンジン shall 空の段を取り除いて取り出し元を決める（例: `/extra//bal1/` は `extra/bal1`）。
4. If 取り除いた後に段が 1 つも残らないとき, then the NAR エンジン shall インストール全体を断り、理由に鍵と書かれていた値を載せる（書庫の全体を取り出し元にしない）。

### Requirement 4: `*.directory` のパス区切りは今どおり断る

**Objective:** As a 利用者, I want ukadoc が「使えない」と書く値を areka が黙って別の名前に読み替えないこと, so that 作者の書き間違いが、理由の分かる失敗として表に出る

#### Acceptance Criteria

1. If 同梱のバルーンの `*.directory` の値が `\` または `/` を含むとき, then the NAR エンジン shall 今どおりインストール全体を断り、理由に鍵と値を載せる。
2. The NAR エンジン shall `*.directory` の値を読み替えない（区切りの `_` への置き換えも、`..` の取り除きも行わない。読み替えの記録は 0 件）。
3. The areka shall 本 spec では、起動時にどのバルーンを使うかの決め方を変えない（変更 0）。

### Requirement 5: 読み替えた後も安全の検査を今の強さのまま掛ける

**Objective:** As a 利用者, I want 読み替えを足しても、書庫の外へ書き出されたり使えない名前のフォルダが作られたりしないこと, so that 第三者の書庫を落としても根の外が書き換わらない

#### Acceptance Criteria

1. The NAR エンジン shall `*.directory` の値に、今の 1 階層の名前の検査（空でない・`\` と `/` を含まない・Windows で使えない文字を含まない・末尾がドットや空白でない・予約名でない・長さが UTF-16 で 200 単位以内）を掛ける。
2. If `*.directory` の値が 1 項の検査を満たさないとき, then the NAR エンジン shall 今どおりインストール全体を断り、理由に鍵と値を載せる（例: `..`・`../escape`・`extra\bal1`・`CON`・200 単位を超える名前）。
3. The NAR エンジン shall `*.source.directory` の取り除いた後の各段に今の 1 階層の名前の検査を掛け、段をつないだパスの全体の長さに今の上限（UTF-16 で 200 単位）を掛ける。
4. If 取り除いた後の段またはパスの全体が 3 項の検査を満たさないとき, then the NAR エンジン shall 今どおりインストール全体を断り、理由に鍵と値を載せる（例: `.` の段・`C:` を含む段・末尾が空白の段）。
5. If 本 spec のいずれかの理由でインストール全体を断るとき, then the NAR エンジン shall 今どおり書き込みの前に止め、宛先に 1 バイトも書かず、理由を記録する。
6. The NAR エンジン shall 読み替えの有無にかかわらず、同梱のバルーンのファイルとフォルダを、バルーンの置き場の宛先のフォルダの外へ置かない。
7. The NAR エンジン shall 本体の `directory` の検査（区切りや `..` を含む値は断る）を変えない。
8. The NAR エンジン shall 同梱の番号に欠番も先頭の 0 も無く、`*.source.directory` に区切りも `..` も含まない `install.txt` の書庫（α の検体 3 体を含む）について、今と同じ宛先・同じファイル・同じ記録で入れる。並びが今と変わるのは、番号が 10 以上の同梱を持つ書庫だけである。

### Requirement 6: 読み替えたことを 1 件ずつ記録に残す

**Objective:** As a ゴースト・シェルの作者, I want 書いた値が読み替えられたこと・読まれなかったことがログで分かること, so that 思ったとおりに入らなかった理由を自分で辿れる

#### Acceptance Criteria

1. When `*.source.directory` の `..` または空の段の取り除きで、段の並びが書かれていた値と変わったとき, the NAR エンジン shall 鍵ごとに 1 件、鍵・書かれていた値・取り除いた後の値を記録する。
2. The NAR エンジン shall `*.source.directory` の区切りが `\` か `/` かだけの違い（段の並びは同じ）を、読み替えとして記録しない（記録 0 件）。
3. If 探索で入れなかった同梱のバルーンの鍵（打ち切りの後ろの番号の鍵・先頭に 0 を付けた綴りの鍵）が書かれていて、その接頭辞の `*.directory` の行が在るとき, then the NAR エンジン shall その接頭辞の鍵ごとに 1 件、「探索で読まなかった」ことを、知らない鍵の読み飛ばしと区別できる形で記録し、本体と他の同梱のインストールは続ける。
4. If 接頭辞の `*.directory` の行が無い同梱の鍵（Requirement 1 の 9 項）が書かれているとき, then the NAR エンジン shall その鍵を今どおり読み飛ばして、鍵ごとに 1 件記録する（打ち切りの後ろの番号や先頭に 0 を付けた綴りであっても、3 項ではなく本項の記録にする）。
5. The NAR エンジン shall 1 つの鍵から出す記録を最大 1 件とする。
6. The NAR エンジン shall 同じ `install.txt` からは、同じ同梱の列と同じ記録の列（件数と並び）を必ず返す。
7. When インストールが終わったとき, the areka shall 本要件の記録を、今の読み飛ばしの記録と同じくログに 1 件ずつ警告として出す。
8. The NAR エンジン shall 読み替えも読み飛ばしも無い `install.txt` では、本要件の記録を 0 件とする。

### Requirement 7: 正典との対応の記録と網羅台帳

**Objective:** As a areka の開発者, I want 読み方を変えたことと areka が決めたことが文書に残ること, so that 後から完了 spec の要件と実装の違いを辿れる

#### Acceptance Criteria

1. The `doc/COMPAT_ARCHITECTURE.md` §8 shall 完了 `areka-P0-nar-install` の要件 3.9（同梱の `*.directory`・`*.source.directory` を 1 階層の名前に限る）のうち `*.source.directory` の部分と、要件 3.12（`balloonN` の N は 0 以上の整数をすべて読む）の読み方を、本 spec が上書きしたことを記す（`*.directory` を 1 階層の名前に限る部分は上書きしない）。
2. The `doc/COMPAT_ARCHITECTURE.md` §8 shall ukadoc が書いていない点で本 spec が決めたこと（「見つかった」の判定は `*.directory` の行・先頭に 0 を付けた番号は数えない・取り除いた後に段が残らない取り出し元は断る・SSP 2.9.00 以降の形を採る・`*.directory` の区切りは ukadoc の「使えない」を採って断り、`_` への置き換えは採らない・`type` が `ghost`／`shell` 以外の書庫の同梱は読まないまま）を記す。
3. The 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` shall `descript_install` の `*.directory` と `*.source.directory` の 2 行で、本 spec の後の読み方と記録の仕方を述べる。
4. The 網羅台帳 shall 3 項の 2 行のほかの行を、本 spec では変えない（変更 0）。

### Requirement 8: 決定論の自動テスト

**Objective:** As a areka の開発者, I want 3 点の読み替えと安全の検査が自動テストで確かめられること, so that 後の変更で正典との違いが黙って戻らない

#### Acceptance Criteria

1. The 自動テスト shall 書庫を組む既存の検体の道具で組んだ書庫で、次の各場面を決定論的に確かめる。
   - 欠番での打ち切り（`balloon0` と `balloon2` だけ）
   - 無印が無い番号付きだけ
   - 無印と `balloon0` の両方
   - `balloon2` と `balloon10` の並び
   - 先頭に 0 を付けた番号
   - `*.directory` の行が無い接頭辞の断片
   - `\` と `/` の階層付きの取り出し元
   - 取り出し元の配下でない兄弟のファイルが本体に残ること
   - `..` と空の段の取り除き
   - 取り除いた後に段が残らない場合
   - 同梱の `*.directory` に区切りがある場合（`extra\bal1`・`../escape`。今どおり断る）
   - 取り出し元が重なる 2 つの同梱と、取り出し元の途中のフォルダ
   - 本体の `directory` に区切りがある場合（今どおり断る）
2. The 自動テスト shall どの同梱をどの値で読むかの場面では、同梱の列の並びと記録の件数と中身を確かめ、読んだ値からどこへ置くかの場面（階層付きの取り出し元・兄弟のファイル・途中のフォルダ・重なる取り出し元）と、実際に入れる場面（3 項と、読み替えを通した 1 件）では、置かれたファイルとフォルダの全てが根の中の宛先の配下に在ることを確かめる。
3. The 自動テスト shall 「何をインストールしたか」の列が探索の順になることを、番号が 10 以上の同梱を持つ書庫で確かめる。この列から知らせ（`OnInstallComplete` の Reference2 と `OnInstallCompleteEx` の各 Reference）への写しは、インストールの手続きの既存のテストが順を保つことを確かめているので、本 spec では足さない（手続きの側は変更 0）。
4. The 自動テスト shall 階層付きの取り出し元の直下の利用条件が見つかること（Requirement 2 の 7 項）を確かめる。
5. The 自動テスト shall 検体と一時フォルダをワークツリーの `target\` の下だけに作る。
