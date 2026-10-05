# Requirements Document

## Project Description (Input)

- **困っている人**:
  - ゴーストの作者。作者が指定したバルーンでゴーストが出てこない。
    - ゴーストの書庫に同梱したバルーンを、番号付き（`balloon0`・`balloon1`）だけで書いた場合。
    - ゴーストの descript.txt に標準のバルーン（`balloon`・`default.balloon.path`）を書いた場合。
  - 利用者。初めて起動したゴーストが、作者の想定と違うバルーン（既定の `StayseeBalloon` か無作為の 1 つ）で話す。α の検体 `claudia`（`balloon0.directory,claudia`・`balloon1.directory,claudia_vertical`）が実例。
- **今の状況**:
  - 起動時のバルーンは「引数 → 前回の記憶 → 同梱 → 唯一の 1 個 → 既定 → 無作為（0 個なら失敗）」の順で決まる。ゴーストの切替も同じ順を使う。
  - 同梱の段は、ゴーストのフォルダの `install.txt` の**無印の `balloon.directory` だけ**を読む。番号付きの鍵は読まない。
  - ゴーストの descript.txt の `balloon`・`default.balloon.path` は、どこからも読まれていない。網羅台帳では `absent`・担当なしで、注記はもう無い関数を指している。
- **変えたいこと**:
  - 同梱の段が「最初の 1 個」を使う。無印があれば無印、無ければ `install-companion-reading` の探索の順で最初に見つかった番号（`balloon0`）。
  - ゴーストの descript.txt の `balloon` と `default.balloon.path` が、起動時にバルーンを決める段として効く。
  - 段の並びと、ukadoc が書いていない点の決めを `doc/COMPAT_ARCHITECTURE.md` §8 に記す。網羅台帳の該当の行を実装に合わせる。
  - 各段が当たらなかったときの記録を残す。
- 詳細は同じフォルダの `brief.md`。前身は `completed/areka-P0-install-companion-canon/`（実施せず 3 本へ分けた 2 本目が本 spec）。brief の「名前はインストールと同じ `_` への置き換えで読む」は、`install-companion-reading` が置き換えを採らないと決めたので取り下げ済み。

## Introduction

ゴーストの作者は、そのゴーストが標準で使うバルーンを 2 つの方法で示せる。本 spec が依る ukadoc の記述は次のとおりである（2026-10-05 に引き直した）。

- **書庫に同梱する**（ukadoc「Install設定」 https://ssp.shillest.net/ukadoc/manual/descript_install.html ）:
  - 「なお、ゴーストやシェルに紐づくバルーンとして設定されるのは最初の1個だけである。」
  - 「2個目以降はインストールされるだけで、そのゴーストの標準バルーンにはならない。」
  - 番号の探索は「無印→0→1→2…の順」で、「見つからない番号が出た時点で打ち切られる」。「無印と0は別のものとして扱われる」。
- **ゴーストの descript.txt に書く**（ukadoc「ゴースト設定」 https://ssp.shillest.net/ukadoc/manual/descript_ghost.html ）:
  - `balloon,バルーン名`:「標準で使用するバルーン名。」省略したときは「不明 | SSPバルーンか、ユーザーが設定した標準バルーン( on SSP)」。
  - `default.balloon.path,パス`:「標準で使用するバルーンの相対パス。」省略したときは同上。

ukadoc が書いていないことは次のとおりで、本 spec が決める。

- 前回の記憶・descript の指定・同梱の最初の 1 個を、どの順で使うか（Requirement 3）。記憶 → descript → 同梱の順。2026-10-05 開発者の決め。
- 最初の 1 個が根に見つからないとき、2 個目へ繰り下げるか（Requirement 1）。繰り下げない。
- 最初の 1 個の値が空のとき・パス区切りを含むときの扱い（Requirement 1）。どちらも「最初の 1 個は当たらなかった」とし、2 個目へ繰り下げず、値を読み替えない。空の値の無印も「見つかった」に数える。2026-10-05 開発者の決め。
- `balloon` の「バルーン名」を何と突き合わせるか（Requirement 2）。バルーンの descript.txt の `name` を先に、当たらなければフォルダ名。実行中の `\![change,balloon,バルーン名]` と同じ決め方。2026-10-05 開発者の決め。
- `default.balloon.path` の「相対パス」の起点（Requirement 2）。根のバルーンの置き場（`<根>/balloon/`）。値はフォルダ名 1 段で、置き場に列挙されるバルーンとだけ突き合わせる。ゴーストのフォルダの中は見ない（ゴーストの中の `balloon/` は `areka-P0-ghost-inner-balloon` が扱う）。2026-10-05 開発者の決め。
- `balloon` と `default.balloon.path` の両方が書かれているときの順（Requirement 2）。`default.balloon.path` を先に、当たらなければ `balloon`。2026-10-05 開発者の決め。

本 spec の後、記憶を持たないゴースト（初めて起動したゴースト・記憶の先のバルーンが消えたゴースト）は、作者の指定したバルーンで出てくる。利用者が一度でもバルーンを選んだゴーストは、今どおり選んだバルーンで出てくる。

## Boundary Context

- **In scope**:
  - 起動時にバルーンを決めるときの、同梱の最初の 1 個の読み方（無印が無ければ `balloon0`）。
  - ゴーストの descript.txt の `balloon`・`default.balloon.path` を、バルーンを決める段にすること。
  - 段の並び。
  - ゴーストの切替で切替先のバルーンを決めるときも同じ並びになること。
  - 各段が当たらなかったときの記録。
  - `doc/COMPAT_ARCHITECTURE.md` §8 と網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml`（範囲外の `recommended.balloon`・`recommended.balloon.path` の 2 行は、古い注記の文だけを直す）。
  - 決定論の自動テスト。
- **Out of scope**（どれも今の動きを変えない＝変更 0）:
  - `recommended.balloon`／`recommended.balloon.path`（これ以外へ切り替えると「強い内容の警告」を出す項目。roadmap の覚え書き。網羅台帳の注記の文だけは Requirement 6 で直す）。
  - ゴーストのフォルダの中の `balloon/` に置いたバルーン（`areka-P0-ghost-inner-balloon`）。本 spec は、バルーンをゴーストのフォルダの中に探さない（変更 0）。
  - シェルに紐づくバルーン（`areka-P0-shell-companion-balloon`）。シェルの切替では、本 spec の段を見ない。
  - インストールの読み方（完了 `areka-P0-install-companion-reading`）。本 spec は書庫の入れ方を 1 行も変えない。
  - `char*.balloon.*` などバルーンの見た目の鍵。
  - 引数でバルーンを渡したときの扱い、唯一・既定・無作為の段、バルーンが 0 個のときの失敗。
  - 実行中のバルーンの切替（メニュー・`\![change,balloon,バルーン名]`）の切替先の決め方。
  - フォルダ名の突き合わせの規則（大文字と小文字を区別する完全一致）。
  - 記憶の持ち方（ゴーストごとの `areka.last.balloon`）。
- **Adjacent expectations**:
  - 完了 `areka-P0-install-companion-reading` から受け取るのは探索の順だけである（無印 → 0 → 1…・「見つかった」は `*.directory` の行が在ること・先頭に 0 を付けた綴りは数えない）。`*.directory` の値は読み替えないので、共有する読み替えの規則は 0 個。
  - 完了 `areka-P0-baseware-root-layout` の要件 5.3（同梱は無印の `balloon.directory`）を本 spec が上書きする。同要件 5.2 の「利用者の選択は同梱より優先」は崩さない。
  - `areka-P0-shell-companion-balloon` は、本 spec の並びへ「シェルに紐づくバルーン」の段を足す。本 spec はその段の置き場所を決めない。
  - `areka-P0-ghost-inner-balloon` は、ゴーストの中の `balloon/` に置いたバルーンを使えるようにする。`default.balloon.path` の起点を広げるかどうかは、その spec が決める。本 spec の後は、根のバルーンの置き場だけが起点である。

## Requirements

### Requirement 1: 同梱の最初の 1 個を標準のバルーンにする

**Objective:** As a ゴーストの作者, I want 番号付きだけで書いた同梱でも、最初の 1 個がそのゴーストの標準のバルーンになること, so that ukadoc の記述例どおりに書いたゴーストが、同梱したバルーンで出てくる

#### Acceptance Criteria

1. When 同梱の段でバルーンを決めるとき, the areka shall ゴーストのフォルダの最上位の `install.txt` から、同梱のバルーンを無印（`balloon`）→ `balloon0` → `balloon1` … の順に探し、最初に見つかった 1 個だけを「同梱の最初の 1 個」とする。
2. The areka shall 同梱のバルーンを「見つかった」とみなすのを、その接頭辞の `*.directory` の行（無印なら `balloon.directory`、番号 N なら `balloonN.directory`）が書かれているときに限る（完了 `areka-P0-install-companion-reading` と同じ判定）。
3. When 無印と `balloon0` の両方が見つかるとき, the areka shall 無印を最初の 1 個とする。
4. When 無印が見つからず `balloon0` が見つかるとき, the areka shall `balloon0` を最初の 1 個とする（例: `claudia` は `balloon0.directory,claudia` の `claudia`）。
5. When 無印も `balloon0` も見つからないとき, the areka shall 同梱の指定は無いものとして扱い、次の段へ進む（`balloon1` より後ろだけが書かれていても読まない。記録は 0 件）。
6. The areka shall 番号として数える綴りを、先頭に 0 を付けない 10 進（`0`・`1`…）だけとする（`balloon00.directory` は最初の 1 個にしない）。
7. The areka shall 最初の 1 個の `*.directory` の値を、根のバルーンの置き場のフォルダ名と、今どおり大文字と小文字を区別する完全一致で突き合わせる。
8. When 最初の 1 個の名前のバルーンが根に列挙されるとき, the areka shall 同梱の段でそのバルーンを使う。
9. If 最初の 1 個の名前のバルーンが根に列挙されないとき, then the areka shall そのことを記録して次の段へ進み、2 個目以降の同梱へは繰り下げない（例: `claudia` が消されていても `claudia_vertical` を同梱の段で選ばない）。
10. The areka shall 2 個目以降の同梱を、同梱の段の候補にしない。
11. When ゴーストのフォルダに `install.txt` が無いとき, the areka shall 同梱の指定は無いものとして扱い、次の段へ進む（記録は 0 件）。
12. If `install.txt` が在るのに読めないとき, then the areka shall 今どおり記録を 1 件残し、同梱の指定は無いものとして次の段へ進む。
13. The areka shall 無印の `balloon.directory` を書いたゴースト（検体 `emo2` を含む）について、同梱の段の結果を今と変えない。
14. When 最初の 1 個の `*.directory` の行が在り、値が空のとき, the areka shall その行を「見つかった」に数えて探索をそこで終え（空の値の無印が在れば `balloon0` へ進まない）、最初の 1 個は当たらなかったものとして次の段へ進む（2 個目以降へ繰り下げない。今の「空の値は無しとして扱う」に合わせて記録は 0 件。2026-10-05 開発者の決め）。
15. If 最初の 1 個の `*.directory` の値がパス区切り（`\`・`/`）を含むとき, then the areka shall 値を読み替えず（`_` への置き換えは 0 か所）、9 項のとおり「根に列挙されない」として扱う（警告は今と同じ形で 1 件・2 個目以降へ繰り下げない。2026-10-05 開発者の決め）。

> 空の値とパス区切りを含む値は、どちらもインストールで断られる（完了 `areka-P0-install-companion-reading` の要件 1 の 2 項・要件 5）。起動の側が出会うのは、手で置かれたゴーストだけである。「行が在れば、値が空でも見つかった」はインストールと同じ判定である。

### Requirement 2: ゴーストの descript.txt の標準のバルーンの指定を使う

**Objective:** As a ゴーストの作者, I want descript.txt に書いた標準のバルーンが起動時に効くこと, so that バルーンを同梱しなくても、使ってほしいバルーンを名指しできる

#### Acceptance Criteria

1. When descript の段でバルーンを決めるとき, the areka shall 起動するゴーストの `ghost/master/descript.txt` の `default.balloon.path` と `balloon` を、標準のバルーンの指定として読む。
2. The areka shall 鍵が書かれていない、または値が空の指定を、今の「空の値は無しとして扱う」と同じく、書かれていないものとして扱う。
3. The areka shall `default.balloon.path` の値を、根のバルーンの置き場（`<根>/balloon/`）の直下のフォルダ名 1 段として読み、置き場に列挙されるバルーンのフォルダ名と、大文字と小文字を区別する完全一致で突き合わせる（2026-10-05 開発者の決め）。
4. If `default.balloon.path` の値が `..`・絶対パス・パス区切り（`\`・`/`）を含むとき, then the areka shall 値を読み替えず（区切りの取り除き・`_` への置き換えは 0 か所）、当たらなかったものとして扱う（例: `../balloon/claudia`・`balloon/claudia`・`claudia/`・`C:\balloon\claudia` はどれも当たらない）。
5. The areka shall `default.balloon.path` の指す先を、ゴーストのフォルダの中（`<ゴースト>/balloon/` を含む）に探さない（変更 0。ゴーストの中の `balloon/` は `areka-P0-ghost-inner-balloon` が扱う）。
6. The areka shall `balloon` の値を、実行中の `\![change,balloon,バルーン名]` が、ふつうの名前（`random`・`lastinstalled` 以外）から切替先を決めるときと同じ次の 4 点で、根のバルーンの置き場に列挙されるバルーンと突き合わせる（2026-10-05 開発者の決め。この 2 語の扱いは 8 項）。
   - まず、バルーンの descript.txt の `name` が値と一致するバルーンを探す。
   - `name` が一致するバルーンが 1 つも無いときに限り、フォルダ名が値と一致するバルーンを探す。
   - どちらの一致も、大文字と小文字を区別する完全一致とする。
   - `name` が一致するバルーンが複数在るときは、列挙の並び（フォルダ名の順）で最初のものとする。
7. When あるバルーンの `name` と別のバルーンのフォルダ名がどちらも `balloon` の値と一致するとき, the areka shall `name` が一致したバルーンを使う。
8. The areka shall `balloon` の値の `random`・`lastinstalled` を、特別な指し方として解かず、他の値と同じくバルーンの `name`・フォルダ名と突き合わせる（この 2 語はさくらスクリプトの `\![change,balloon,…]` の指し方で、ukadoc の `balloon,バルーン名` には書かれていない）。
9. When `default.balloon.path` と `balloon` の両方が書かれているとき, the areka shall `default.balloon.path` を先に突き合わせ、当たらなかったときに限り `balloon` を突き合わせる（2026-10-05 開発者の決め）。
10. When `default.balloon.path` が当たったとき, the areka shall そのバルーンを descript の段で使い、`balloon` を突き合わせない（`balloon` についての記録は 0 件）。
11. When `default.balloon.path` が当たらず（書かれていない場合を含む）、`balloon` が当たったとき, the areka shall `balloon` の指すバルーンを descript の段で使う。
12. If 書かれている指定がどれも当たらなかったとき, then the areka shall 当たらなかった鍵ごとに記録を残して次の段へ進む（起動は止めない）。
13. When `default.balloon.path` も `balloon` も書かれていないとき, the areka shall descript の指定は無いものとして扱い、次の段へ進む（記録は 0 件）。
14. The areka shall `recommended.balloon`・`recommended.balloon.path` を、バルーンを決める段に使わない（変更 0）。

### Requirement 3: バルーンを決める段の並び

**Objective:** As a 利用者, I want 自分で選んだバルーンが作者の指定で上書きされないこと, so that 一度選んだバルーンが、再起動のたびに作者の標準へ戻らない

brief の推し「記憶 → descript → 同梱 → 唯一 → 既定 → 無作為」を採り、2026-10-05 に開発者がこの並びを確かめた。利用者が選んだものを最優先にし、作者が文字で書いた指定（descript）を、書庫の詰め方から導いた指定（同梱）より上に置く。

#### Acceptance Criteria

1. The areka shall 起動時のバルーンを、引数 → 前回の記憶 → descript の指定 → 同梱の最初の 1 個 → 唯一の 1 個 → 既定（`StayseeBalloon`）→ 無作為 の順で、最初に当たった段のバルーンに決める。
2. When 引数でバルーンが渡されたとき, the areka shall 今どおりそのバルーンを使い、記憶・descript・同梱を読まない。
3. When 前回の記憶が根に列挙されるバルーンを指すとき, the areka shall descript の指定と同梱の最初の 1 個が何であっても、記憶のバルーンを使う。
4. When 記憶が当たらず、descript の指定が当たるとき, the areka shall 同梱の最初の 1 個が何であっても、descript の指定のバルーンを使う。
5. When 記憶も descript の指定も当たらず、同梱の最初の 1 個が当たるとき, the areka shall 同梱の最初の 1 個を使う。
6. When 記憶・descript の指定・同梱の最初の 1 個のどれも当たらないとき, the areka shall 今どおり 唯一の 1 個 → 既定 → 無作為 の順で決める。
7. If 根に列挙されるバルーンが 0 個のとき, then the areka shall 今どおり失敗として扱う（告知の出し方は変更 0）。
8. The areka shall 記憶を持つゴーストについて、記憶のバルーンが根に在るかぎり、起動時のバルーンを今と変えない。

### Requirement 4: ゴーストの切替でも同じ並びで決める

**Objective:** As a 利用者, I want ゴーストを切り替えた先でも、作者の指定したバルーンで出てくること, so that 起動したときと切り替えたときでバルーンが食い違わない

#### Acceptance Criteria

1. When 実行中にゴーストを切り替えるとき, the areka shall 切替先のゴーストのバルーンを、Requirement 3 の並びのうち引数の段を除いたもの（切替先の記憶 → 切替先の descript の指定 → 切替先の同梱の最初の 1 個 → 唯一 → 既定 → 無作為）で決める。
2. The areka shall 実行中のバルーンの切替（メニュー・`\![change,balloon,バルーン名]`）の切替先の決め方を変えない（変更 0）。
3. The areka shall シェルの切替で、本 spec の段を見ない（変更 0）。

### Requirement 5: 当たらなかった段を記録に残す

**Objective:** As a ゴーストの作者, I want 指定したバルーンが使われなかった理由がログで分かること, so that 書き間違いや入れ忘れを自分で辿れる

#### Acceptance Criteria

1. When バルーンが決まったとき, the areka shall どの段で決まったかを、今の記録と同じ形で残し、descript の段で決まったことを他の段と区別できるようにする。
2. If descript の指定の鍵（`default.balloon.path`・`balloon`）が突き合わせて当たらなかったとき, then the areka shall その鍵ごとに、鍵・書かれていた値・バルーンの置き場を載せた警告を 1 件残す（`default.balloon.path` の値が `..`・絶対パス・パス区切りを含むときも同じ 1 件）。
3. If 同梱の最初の 1 個が当たらなかったとき, then the areka shall 今と同じ形（書かれていた値・バルーンの置き場）の警告を 1 件残す。
4. If ゴーストの `ghost/master/descript.txt` が読めないとき, then the areka shall 記録を 1 件残し、descript の指定は無いものとして次の段へ進む。
5. The areka shall 1 回の解決で出す「当たらなかった」の記録を、descript の段からは鍵ごとに最大 1 件（2 つの鍵で最大 2 件）、同梱の段からは最大 1 件とする。
6. The areka shall 指定が書かれていない段からは、記録を出さない（0 件）。
7. The areka shall 本 spec が足す分岐に、記録の無い失敗の経路を作らない（0 本）。

### Requirement 6: 正典との対応の記録と網羅台帳

**Objective:** As a areka の開発者, I want 段の並びと areka が決めたことが文書に残ること, so that 後から完了 spec の要件と実装の違いを辿れる

#### Acceptance Criteria

1. The `doc/COMPAT_ARCHITECTURE.md` §8 shall バルーンを決める段の並び（Requirement 3）と、その理由（利用者の選択を最優先・文字で書いた指定を書庫の詰め方から導いた指定より上）を記す。
2. The `doc/COMPAT_ARCHITECTURE.md` §8 shall ukadoc が書いていない点で本 spec が決めたことを記す。
   - 最初の 1 個が根に無いとき、2 個目へ繰り下げない。
   - 最初の 1 個の値が空のとき・パス区切りを含むときは当たらなかったものとし、値を読み替えない。空の値の無印も「見つかった」に数える。
   - `balloon` の値は、バルーンの descript.txt の `name` を先に、当たらなければフォルダ名と突き合わせる（実行中の `\![change,balloon,バルーン名]` と同じ）。`random`・`lastinstalled` は特別に解かない。
   - `default.balloon.path` の起点は根のバルーンの置き場で、値はフォルダ名 1 段。`..`・絶対パス・パス区切りを含む値は当たらないものとし、読み替えない。
   - 両方が書かれているときは `default.balloon.path` を先に、当たらなければ `balloon`。
3. The `doc/COMPAT_ARCHITECTURE.md` §8 shall 完了 `areka-P0-baseware-root-layout` の要件 5.3（同梱は無印の `balloon.directory`）を本 spec が上書きしたことを記す。
4. The 網羅台帳 `doc/ukadoc-coverage/ledger/assets.toml` shall `descript_ghost` の `balloon` と `default.balloon.path` の 2 行で、本 spec の後の読み方と記録の仕方を述べ、もう無い関数（`boot_config::default_balloon_root`・`boot_config::resolve_config_inputs`）を指す注記を残さない。
5. The 網羅台帳 shall `descript_install` の `*.directory` の行で、起動時に同梱の最初の 1 個が標準のバルーンになることを述べる。
6. The `doc/COMPAT_ARCHITECTURE.md` §8 shall 「ゴーストの中の `balloon/` は見ない・`ghost-inner-balloon` が扱う」ことを記す。
7. The 網羅台帳 shall 範囲外の `descript_ghost` の `recommended.balloon` と `recommended.balloon.path` の 2 行で、status を `absent` のまま変えず、注記の文だけを直して、もう無い関数（`boot_config::default_balloon_root`・`boot_config::resolve_config_inputs`）を指す記述を残さない（残り 0 か所。2026-10-05 開発者の決め）。

### Requirement 7: 決定論の自動テスト

**Objective:** As a areka の開発者, I want 段ごとの当たり・外れ・並びが自動テストで確かめられること, so that 後の変更で作者の指定が黙って効かなくなることがない

#### Acceptance Criteria

1. The 自動テスト shall 同梱の最初の 1 個の読み方を、次の各場面で決定論的に確かめる。
   - 無印だけ
   - 番号付きだけ（`balloon0`・`balloon1`）
   - 無印と `balloon0` の両方
   - `balloon1` だけ（`balloon0` が欠番）
   - 先頭に 0 を付けた綴りだけ
   - `*.directory` の行が無く、同じ接頭辞の他の鍵だけ
   - `install.txt` が無い・読めない
2. The 自動テスト shall 段の並びを、次の各場面で決定論的に確かめる。
   - 記憶が descript の指定と同梱に勝つ
   - descript の指定が同梱に勝つ
   - 同梱が唯一・既定・無作為に勝つ
   - 記憶の先が根に無く、descript の指定へ進む
   - descript の指定の先が根に無く、同梱へ進む
   - 同梱の最初の 1 個が根に無く、2 個目へ繰り下げずに既定へ進む
   - 引数が在るとき、他の段を読まない
3. The 自動テスト shall 2 項の各場面で、決まったバルーン・決まった段・記録の件数と中身を確かめる。
4. The 自動テスト shall descript の指定の突き合わせを、次の各場面で決定論的に確かめる。
   - `default.balloon.path` だけが書かれ、当たる・当たらない
   - `default.balloon.path` の値が `..` を含む・絶対パス・パス区切りを含む（どれも当たらず、警告 1 件で次の段へ進む）
   - `default.balloon.path` の値と同じ名前のフォルダがゴーストの中の `balloon/` にだけ在る（当たらない）
   - `balloon` だけが書かれ、バルーンの `name` で当たる・フォルダ名で当たる・どちらにも当たらない
   - `balloon` の値が、あるバルーンの `name` と別のバルーンのフォルダ名の両方に一致する（`name` の側が当たる）
   - `balloon` の値と一致する `name` のバルーンが複数在る（列挙の並びで最初のものが当たる）
   - `balloon` の値が大文字と小文字だけ違う（当たらない）
   - `balloon` の値が `random`（特別に解かれない）
   - 両方が書かれ、`default.balloon.path` が当たる（`balloon` についての記録は 0 件）
   - 両方が書かれ、`default.balloon.path` が当たらず `balloon` が当たる（警告 1 件）
   - 両方が書かれ、どちらも当たらない（警告 2 件で同梱の段へ進む）
   - 鍵が書かれていない・値が空（記録は 0 件）
   - `ghost/master/descript.txt` が読めない
5. The 自動テスト shall 同梱の最初の 1 個の値を、次の各場面で決定論的に確かめる。
   - 無印の値が空で `balloon0` が在る（`balloon0` へ進まず、記録 0 件で次の段へ進む）
   - `balloon0` の値が空で `balloon1` が在る（`balloon1` へ繰り下げず、記録 0 件で次の段へ進む）
   - 値がパス区切りを含む（読み替えず、警告 1 件で次の段へ進む）
6. The 自動テスト shall ゴーストの切替で切替先のバルーンが同じ並びで決まることを、切替先が descript の指定または番号付きだけの同梱を持つ場面で確かめる。
7. The 自動テスト shall 検体と一時フォルダをワークツリーの `target\` の下だけに作る。
