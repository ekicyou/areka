# Requirements Document

## Project Description (Input)

- **誰が困っているか**: areka に MCP でつなぐ AI エージェント。台本に `\s[n]` を書くには「どの番号がどの表情か」の表が要る。
- **今の状況**: SSP の `get_expression_table` はツール説明で「sakurascript の前にこれを使え」と促しており、AI の台本づくりの起点になっている。areka の同名のツールは入口だけがあり、呼ぶと `NG:not implemented yet` を返す。シェルの `surfacetable.txt` を読む仕組みも無い。
- **何を変えるか**: `get_expression_table(ghost_name)` が、今のゴーストの今のシェルから、SSP と同じ形の表（Markdown の表・スコープ・キャラクタ名・説明・`\s[n]`）を返す。SSP が説明の字を化けさせる欠陥は移植せず、正しい字で返す。着せ替え・アニメーションの一覧は出さない（SSP の本ツールも出さない）。シェルの切替・再読み込みは扱わない。

出どころ: `brief.md`（2026-09-29 起票・2026-10-04 再測定）。事実の正本は `doc/ssp-mcp/survey.md`。

## Introduction

`get_expression_table` は、AI エージェントが台本を書く前に「このゴーストはどの番号でどんな表情をするか」を知るためのツールである。本 spec は、入口だけがある今のツールに中身を入れ、今のシェルの `surfacetable.txt` と、areka が持つ既定の名前の表（SSP の日本語の表と同じ 15 件）から、SSP と同じ形・同じ並びの表を組んで返す。

## 根拠（ukadoc と SSP の実測）

### ukadoc（`surfacetable.txt`・https://ssp.shillest.net/ukadoc/manual/descript_shell_surfacetable.html）

brief は説明の出どころの候補に「`surfacetable.txt`・surfaces.txt の `surface.alias`・`descript` の `sakura.surface.alias`」を挙げていたが、ukadoc を引き直した結果、出どころは `surfacetable.txt` だけである。

- `surfacetable.txt` は「SSP専用」「サーフェス内容の定義ファイル」「省略可能」（シェルのページ）。書ける項目は次の 6 つ。
  - `charset,文字コード` — 「記載は必ずファイルの一行目に」。省略時は Shift_JIS。
  - `version,数値` — 「現在のところ常に1でよい」。
  - `option,オプション1,オプション2,...` — 指定できるのは `DisableNoDefineSurfaces` だけ。「surfaces.txt等での定義がなく、surfacetable.txt内でも明示されていないサーフェスについて無視する」。
  - `group,グループ名` — 「サーフェス名定義行をまとめたグループに識別上の名前を与える」。すぐ下に `{` だけの行、`}` だけの行で閉じる。グループ名 `__disabled` は「そのグループ内のサーフェスは使用されていないことを示す」。
  - `scope,数値` — 「そのグループのサーフェスが主に属するスコープの番号。グループ内に一つ記載する」。
  - `サーフェスID,名前` — 「サーフェスIDに対して識別上の名前を与える」。名前 `__parts` は「主にアニメーションや着せ替えなどのパーツとして用いられるもの」。名前を省略すると「名前が未定義であることを明示できる」。
- `surface.alias`ブレス（surfaces.txt の `sakura.surface.alias`・`kero.surface.alias`）と surface*ブレスの `name,定義名` は、`\s[]` で ID の代わりに書ける別名であって、表情の説明ではない。`descript.txt` に `sakura.surface.alias` という項目は無い（brief の「`descript` の `sakura.surface.alias`」は誤り）。

### 開発者の裁定（2026-10-04）

- SSP 本体が持つ既定の名前の表を、areka も持ち、SSP と同じ規則で重ねる。言語は日本語の表の 1 つだけ。15 件は次のとおり（SSP の `data\language\japanese\surfacetable.txt` と同じ）。

  | ID | 名前 | ID | 名前 | ID | 名前 |
  |---|---|---|---|---|---|
  | 0 | 素 | 5 | 微笑み | 10 | \1-素 |
  | 1 | 照れ | 6 | 目閉じ | 11 | \1-刮目 |
  | 2 | 驚き | 7 | 怒り | 19 | \1-歌 |
  | 3 | 不安 | 8 | 冷笑 | 20 | 立て看板 |
  | 4 | 落ち込み | 9 | 照れ怒り | 25 | 歌 |

- SSP の文字化けの欠陥は移植しない。6 本を同時に呼んだときに SSP が返した「No response from server」も SSP の癖であり、移植しない。

### SSP の実測（2026-10-04・SSP 2.9.05・開発者の机・6 体を 1 体ずつ）

1. 表の形: 見出し行 `|scope \0,\1,\p[2]...|character name|description|surface number : \s[]|`、区切り行 `|-----|-----|-----|-----|`、以降は `|スコープ|グループ名|名前|\s[ID]|`。行の終わりはこの経路では見分けられなかったため、`doc/ssp-mcp/survey.md` の `\r\n` に従う。
2. 並び: スコープの昇順、同じスコープの中はサーフェス ID の昇順（ファイルに書かれた順ではない。既定の 10・11・19 が、シェルの 9 と 20 の間に入る）。
3. スコープ 2 以上は `\p[2]` の形。
4. 同じスコープの `group` が 2 つあるとき: 両方が載り、各行は自分のグループ名を持ち、そのスコープの行は ID の順に混ざる。
5. `group,__disabled` の中の行と、名前 `__parts` の行は載らない。
6. `scope` の行が無い `group` はスコープ 0 として扱われる。
7. `group,`（グループ名が空）はキャラクタ名の列が空になる。
8. どの `group` にも入っていない `サーフェスID,名前` の行（`group` が 1 つも無い平たいファイルを含む）は、スコープ `\0`・キャラクタ名は空で載る。
9. 名前を省略した行（`0,`）は**載る**。説明の列は空で、既定の名前より勝つ（`|\0|本体基本||\s[0]|`）。この検体は `{` が閉じないままファイルが終わるが、読めた分は使われた。
10. `surfacetable.txt` が無いシェルは、既定の 15 件だけの表（すべて `|\0||名前|\s[ID]|`）。
11. 既定の名前は、シェルの `surfacetable.txt` が**触れていない ID** にだけ載る（スコープ `\0`・キャラクタ名は空）。「触れている」は ID だけで判定され、スコープは見ない（10・11・19 をスコープ 1 に書いたシェルでは、既定の 10・11・19 は `\0` に出ず、25 は出る）。`group,__disabled` の中に書かれた ID も既定を消す（10 を `__disabled` の中に `__parts` で書いたシェルでは、既定の 10 は出ず、11・19・20・25 は出る）。
12. surfaces.txt にサーフェスの定義があるかどうかは見られていない（定義の無い 11・19・25 も載る）。少なくとも `option,DisableNoDefineSurfaces` が無いとき。
13. `surface.alias`ブレスは表に影響しない。surfaces.txt で定義されていても `surfacetable.txt` に行が無く既定の表にも無いサーフェスは載らない。
14. 検体（`group,0`＝`scope,0` に 0〜9・20・21、`group,1`＝`scope,1` に 2100〜2110・2200〜2210）の結果: `\0` の 0〜9（キャラクタ名 `0`）→ `|\0||\1-素|\s[10]|` → `|\0||\1-刮目|\s[11]|` → `|\0||\1-歌|\s[19]|` → `|\0|0|ぴえん|\s[20]|` → `|\0|0|ジト|\s[21]|` → `|\0||歌|\s[25]|` → `\1` の 2100〜（キャラクタ名 `1`）。

### 実物の `surfacetable.txt` に見られた書き方（開発者の机の 13 本）

`Charset,UTF-8`（先頭が大文字）・`charset` の前の UTF-8 の BOM・`version,1`・`option,DisableNoDefineSurfaces`・`//` で始まる注釈の行・空行・タブで字下げした行・行の終わりに付いた `}`（`100,黒塗り}`）・閉じていない `{`。

### 実測していないもの

- `option,DisableNoDefineSurfaces` の効き方は SSP で実測していない。Requirement 3 の該当の基準は **ukadoc の文言から導いたもので、SSP では未実測**である。
- 同じサーフェス ID が `surfacetable.txt` に 2 度書かれたときの勝ち負けは実測していない。要件は「表全体を失敗させない」までとし、どちらを採るかは設計で決める。

## Boundary Context

- **In scope**: 今のシェルの `surfacetable.txt` の読み取り・既定の名前（15 件）の重ね合わせ・表の組み立て（スコープ・キャラクタ名・説明・`\s[n]`）と並び・正しい字で返すこと・崩れた書き方への寛容さ・SSP の実測と突き合わせた期待値による常時テスト。
- **Out of scope**:
  - 着せ替え・アニメーションの一覧。
  - シェルの切替・再読み込み（`shell-balloon-switch`・`mcp-reload` の持ち分）。
  - `ghost_name` からゴーストを決めること、および省略・空・稼働していないゴーストの名指しへの答え（ツールの入口＝`mcp-tool-entrances` が既に答えている。本 spec は変えない）。
  - `surface.alias`ブレスや surface*ブレスの `name` を表に載せること。
  - 既定の名前の表を日本語以外で持つこと・利用者が差し替えること。
  - SSP の文字化けと、同時に呼んだときの「No response from server」の再現。
- **Adjacent expectations**: `mcp-tool-entrances` が、稼働中のゴーストを 1 体に決めてから本ツールの中身を呼ぶ。`mcp-dump-images` の「No such surface ID. Check get_expression_table tool」は本ツールを文言で指すだけで、本 spec は何も受け渡さない。既定の 15 件を areka がどう持つか（埋め込みか・ファイルか）は設計で決める。

## Requirements

### Requirement 1: 表の形

**Objective:** As a MCP でつなぐ AI エージェント, I want SSP と同じ形の表を受け取りたい, so that SSP 向けに作った手順のまま areka のゴーストへ台本を書ける

#### Acceptance Criteria

1. When 稼働中のゴーストを対象に `get_expression_table` が呼ばれた, the get_expression_table ツール shall 1 行目に `|scope \0,\1,\p[2]...|character name|description|surface number : \s[]|`、2 行目に `|-----|-----|-----|-----|` を返す
2. When 稼働中のゴーストを対象に `get_expression_table` が呼ばれた, the get_expression_table ツール shall 3 行目以降に、表の 1 件につき `|スコープ|キャラクタ名|説明|\s[サーフェスID]|` の形の 1 行を返す
3. The get_expression_table ツール shall 見出し行・区切り行・各行の終わりを `\r\n` にする
4. The get_expression_table ツール shall スコープの列を、スコープ 0 は `\0`、スコープ 1 は `\1`、スコープ 2 以上の n は `\p[n]` と書く
5. Where キャラクタ名または説明が空である, the get_expression_table ツール shall その列を何も挟まない空の列として書く（例 `|\0||歌|\s[25]|`・`|\0|本体基本||\s[0]|`）
6. The get_expression_table ツール shall `surfacetable.txt` の文字コードが何であっても、キャラクタ名と説明を元の字のとおりに返す（SSP の文字化けは移植しない）
7. The get_expression_table ツール shall 見出し行・区切り行・表の行のほかには何も返さない（前置きの文・`OK:` の接頭辞・着せ替えの一覧・アニメーションの一覧を付けない）

### Requirement 2: シェルの `surfacetable.txt` から載せる行

**Objective:** As a MCP でつなぐ AI エージェント, I want シェルの作者が付けた表情の名前をそのまま受け取りたい, so that 番号と表情の対応を取り違えずに `\s[n]` を選べる

#### Acceptance Criteria

1. The get_expression_table ツール shall 今のゴーストの今のシェルの `surfacetable.txt` にある `サーフェスID,名前` の 1 行を表の 1 行にし、名前を説明の列へ、サーフェス ID を `\s[サーフェスID]` の列へ書く
2. The get_expression_table ツール shall その行が属する `group` のグループ名をキャラクタ名の列へ、その `group` の `scope` の数値をスコープの列へ書く
3. Where `group` に `scope` の行が無い, the get_expression_table ツール shall その `group` の行をスコープ 0 として載せる
4. Where `group` のグループ名が空である, the get_expression_table ツール shall その `group` の行のキャラクタ名の列を空にする
5. Where `サーフェスID,名前` の行がどの `group` にも属していない, the get_expression_table ツール shall その行をスコープ 0・キャラクタ名は空として載せる
6. Where `サーフェスID,名前` の行の名前が省略されている, the get_expression_table ツール shall その行を説明の列を空にして載せる
7. Where 同じスコープの `group` が 2 つ以上ある, the get_expression_table ツール shall どの `group` の行も載せ、各行にその行が属する `group` のグループ名を書く
8. The get_expression_table ツール shall グループ名が `__disabled` の `group` に属する行を表に載せない
9. The get_expression_table ツール shall 名前が `__parts` の行を表に載せない
10. The get_expression_table ツール shall シェルの `surfacetable.txt` に書かれた行を、そのサーフェスに surfaces.txt の定義や画像があるかどうかに関わらず載せる
11. The get_expression_table ツール shall `surfacetable.txt` に行が無く既定の名前の表にも無いサーフェスを、シェルに定義があっても表に載せない
12. The get_expression_table ツール shall `surface.alias`ブレスと surface*ブレスの `name` を、表の行にも説明にも使わない

### Requirement 3: 既定の名前の重ね合わせ

**Objective:** As a MCP でつなぐ AI エージェント, I want 作者が名前を付けていないシェルでも、よく使われる番号の表情の名前を受け取りたい, so that `surfacetable.txt` の無いゴーストにも SSP と同じ手がかりで台本を書ける

#### Acceptance Criteria

1. The get_expression_table ツール shall 既定の名前として、0 素・1 照れ・2 驚き・3 不安・4 落ち込み・5 微笑み・6 目閉じ・7 怒り・8 冷笑・9 照れ怒り・10 \1-素・11 \1-刮目・19 \1-歌・20 立て看板・25 歌 の 15 件を持つ
2. Where 既定の名前のサーフェス ID が今のシェルの `surfacetable.txt` のどこにも書かれていない, the get_expression_table ツール shall その既定の名前を、スコープ 0・キャラクタ名は空の行として載せる
3. Where 既定の名前のサーフェス ID が今のシェルの `surfacetable.txt` に書かれている, the get_expression_table ツール shall その既定の名前を表に載せない
4. The get_expression_table ツール shall 「`surfacetable.txt` に書かれている」をサーフェス ID だけで判定し、書かれた `group` のスコープが 0 でなくても、`__disabled` の `group` の中でも、名前が `__parts` でも、名前が省略されていても、書かれているものとして扱う
5. If 今のシェルに `surfacetable.txt` が無い, then the get_expression_table ツール shall 既定の 15 件だけの表を返す（`NG:` を返さない）
6. The get_expression_table ツール shall 既定の名前を、OS や areka の言語の設定に関わらず日本語の 15 件のまま返す
7. Where `surfacetable.txt` に `option,DisableNoDefineSurfaces` が無い, the get_expression_table ツール shall 既定の名前を、そのサーフェスがシェルに定義されているかどうかを見ずに載せる
8. Where `surfacetable.txt` に `option,DisableNoDefineSurfaces` がある, the get_expression_table ツール shall 既定の名前のうち、今のシェルに定義（surfaces.txt の surface*ブレス、または surface*.png の画像）の無いサーフェス ID のものを表に載せない（ukadoc の文言から導いた基準。SSP では未実測）
9. Where `surfacetable.txt` に `option,DisableNoDefineSurfaces` がある, the get_expression_table ツール shall シェルの `surfacetable.txt` に書かれた行は、Requirement 2 のとおりに載せる（このオプションで減らさない）

### Requirement 4: 行の並び

**Objective:** As a MCP でつなぐ AI エージェント, I want 表がいつも同じ並びで返ってほしい, so that スコープごと・番号順に表情を探せる

#### Acceptance Criteria

1. The get_expression_table ツール shall 表の行をスコープの昇順に並べる
2. The get_expression_table ツール shall 同じスコープの行を、サーフェス ID の数値の昇順に並べる（`surfacetable.txt` に書かれた順にしない）
3. The get_expression_table ツール shall 既定の名前の行とシェルの行を分けずに、同じ並びの規則で混ぜる（例: シェルの `\s[9]` の後に既定の `\s[10]`・`\s[11]`・`\s[19]`、その後にシェルの `\s[20]`）
4. Where 同じスコープの `group` が 2 つ以上ある, the get_expression_table ツール shall それらの行を `group` ごとにまとめず、サーフェス ID の昇順で混ぜる

### Requirement 5: `surfacetable.txt` の読み取りと崩れた書き方

**Objective:** As a MCP でつなぐ AI エージェント, I want 書き方の少し崩れた `surfacetable.txt` でも表を受け取りたい, so that 実在のゴーストのどれに対しても下調べが止まらない

#### Acceptance Criteria

1. The get_expression_table ツール shall `surfacetable.txt` を 1 行目の `charset` の文字コードで読み、`charset` が無ければ Shift_JIS で読む
2. The get_expression_table ツール shall `charset` の綴りの大文字と小文字の違い（`Charset`）と、`charset` の前に置かれた UTF-8 の BOM を受け入れる
3. The get_expression_table ツール shall `version` の行・`//` で始まる注釈の行・空行を、表の行にせずに読み飛ばす
4. The get_expression_table ツール shall 行の先頭のタブや空白による字下げを無視して読む
5. If `{` が閉じられないまま `surfacetable.txt` が終わっている, then the get_expression_table ツール shall そこまでに読めた行を使って表を返す
6. If `surfacetable.txt` に読み取れない行がある（行の終わりに `}` が付いた行・サーフェス ID が数値でない行・知らないオプションなど）, then the get_expression_table ツール shall 読み取れた分で表を返し、読み取れなかったことを areka のログに記録する
7. If `surfacetable.txt` があるのに開けない、または文字コードが分からず読めない, then the get_expression_table ツール shall そのことを areka のログに記録し、読めた分と既定の名前で表を返す
8. The get_expression_table ツール shall `surfacetable.txt` の書き方の崩れを理由に `NG:` を返すことも、表全体を空にすることもしない
9. If 同じサーフェス ID が `surfacetable.txt` に 2 度以上書かれている, then the get_expression_table ツール shall 表全体を失敗させずに表を返す

### Requirement 6: 呼び出しの副作用と対象

**Objective:** As a MCP でつなぐ AI エージェント, I want 表を取るだけでゴーストに何も起こさないでほしい, so that 台本を書く前の下調べを何度でも安全に行える

#### Acceptance Criteria

1. When `get_expression_table` が呼ばれた, the get_expression_table ツール shall ゴーストに台詞を喋らせず、サーフェスも変えず、SHIORI へイベントも送らず、シェルのファイルも書き換えない
2. When 稼働中のゴーストがシェルを切り替えた後に `get_expression_table` が呼ばれた, the get_expression_table ツール shall 切り替えた後のシェルの `surfacetable.txt` から表を返す
3. The get_expression_table ツール shall `ghost_name` からゴーストを決める規則と、省略・空・稼働していないゴーストの名指しに対する答えを、ツールの入口が返す今の答えから変えない
4. When 複数の `get_expression_table` が同時に呼ばれた, the get_expression_table ツール shall それぞれに表を返す（SSP の「No response from server」は移植しない）

### Requirement 7: 常時テスト

**Objective:** As a areka の開発者, I want SSP と同じ表が返ることを机の SSP 無しで確かめたい, so that 後の変更で表の形や並びが崩れたら常時テストが赤になる

#### Acceptance Criteria

1. The 常時テスト shall SSP の実測 14 の検体と同じ中身の `surfacetable.txt` に対して、SSP の実測と突き合わせた期待値の文字列（見出し行・区切り行・全行・行の終わりまで）と、ツールの返す文字列が一致することを確かめる
2. The 常時テスト shall 次の実測のそれぞれを、期待値の行で確かめる: スコープ 2 以上の `\p[n]`・同じスコープの `group` が 2 つ・`__disabled` と `__parts` が載らないこと・`scope` の無い `group`・グループ名が空の `group`・`group` の外の行・名前を省略した行・閉じていない `{`・`surfacetable.txt` の無いシェル（既定の 15 件だけ）
3. The 常時テスト shall 既定の名前が消える 3 つの場合（スコープ 0 以外の `group` に書かれた ID・`__disabled` の中に書かれた ID・名前を省略した ID）と、消えない場合（どこにも書かれていない ID）を確かめる
4. The 常時テスト shall Shift_JIS の `surfacetable.txt` と UTF-8 の `surfacetable.txt`（BOM の有無・`Charset` の綴りを含む）のそれぞれで、キャラクタ名と説明が元の字のとおりに返ることを確かめる
5. The 常時テスト shall `option,DisableNoDefineSurfaces` がある場合と無い場合で、既定の名前の載り方が Requirement 3 のとおりに変わることを確かめる
6. The 常時テスト shall 読み取れない行のある `surfacetable.txt` で、表が返ることと、読み取れなかったことが記録されることを確かめる
7. The 常時テスト shall 稼働中の SSP にも、ネットワークにも頼らずに走る
