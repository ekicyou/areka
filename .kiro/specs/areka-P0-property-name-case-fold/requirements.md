# Requirements Document

> 本文の実物の確認は **2026-10-22・本ブランチ**（main `6e0ea8d8`）のもの。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md) §7.1・§7.3 の 4（SSP 2.9.07 の実測）。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない。
> **意味は ukadoc から持ち込む方針の例外である。** ukadoc はプロパティの名前の英字の大小について一般の規則を書いていない（大小を区別しないと明記があるのは `developer.log.ログ種別(名前).count` の括弧の中の名前だけ）。本 spec は、ukadoc が黙っている所を SSP の実装のふるまいに合わせる（開発者が `mcp-get-property` の要件ディスカッション 議題 1〔2026-10-04〕で互換を優先して起票を選んだ）。
> 「要件の段で決めた細部」の表は、brief が「要件の段で決める」とした細部に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。

## Project Description (Input)

**誰の何が困っているか**: SSP で動くように書かれたゴーストと、その作者・利用者。SSP はプロパティの名前の英字の大小を区別しない（`BASEWARE.NAME` → `SSP`）が、areka のプロパティシステムの読み手は大小を区別するので、同じ聞き方が「無い名前」になる。ゴーストが大小を混ぜて名前を書いていると、areka では値が取れない。あわせて、書き込みの検査も大小を区別するので、`CURRENTGHOST.NAME` のように大文字で書くと「書けない名前」の検査をすり抜け、自由な名前として書き込まれうる（守りの穴）。

**今の状態**: 点付きの名前の解釈 `parse_dotted`（`crates/areka-sylphya/src/key.rs`）は綴りを保ったまま区切り、読み手 `SylphyaReader`（`crates/areka-sylphya/src/reader.rs`）は正準の綴りを鍵に、ゴーストごとの値 → 全体の値の順で完全一致で引く。書き込みの仕分け `classify_set`（`crates/areka-sylphya/src/actor.rs`）も完全一致で比べる。読み手は MCP の `get_property` と SHIORI の `GetProperty` が共有している。

**何を変えるか**: 名前の英字の大小だけが違う聞き方に、どの読む口でも同じ値が返る（`BASEWARE.NAME` → `areka`）。書き込みの検査が大小で迂回されない。読む口どうしの答えがそろったまま（どれか 1 つだけ畳まない）。

> 起票: 2026-10-04 `/kiro-discovery`（`mcp-get-property` の要件ディスカッション 議題 1 で開発者が「起票して残す」と裁定）。版の段では v0.1.0 の守りの穴として扱う（棚卸㉕〜㉘）。

## Introduction

### 誰が困っているか

- **SSP 向けに書かれたゴーストとその利用者**: ゴーストが SHIORI から `GetProperty` で `CurrentGhost.Name` のように大小を混ぜて聞くと、SSP では値が返り、areka では「無い名前」になる。ゴーストの台詞や分岐が SSP と違ってしまう。
- **AI エージェントで MCP のツールを使う人**: `get_property` に `BASEWARE.NAME` を渡すと、SSP では `SSP`、areka では `NG:Cannot find such property name.` になる。
- **areka の開発者**: 書き込みの検査を大文字で迂回できる穴がある。今は台本から書き込む道（`\![set,property]`）がまだ無いので利用者に見える害は出ていないが、その道が開く前に塞いでおく必要がある。

### SSP 2.9.07 の実測（survey §7.1 の要約）

| 送った名前 | SSP の答え |
|---|---|
| `baseware.name` | `SSP` |
| `BASEWARE.NAME` | `SSP`（大小を区別しない） |
| `ghostlist(えも2DEBUG).path`／`ghostlist(えも2debug).path` | 同じパス（括弧の中の名前も大小を区別しない） |
| ` baseware.name `（前後に空白） | `NG:Cannot find such property name.`（空白は削らない） |

### 実物で引き直して確かめた点（2026-10-22）

1. **今ある読む口は 2 つ。** MCP の `get_property`（`crates/areka/src/mcp/get_property.rs` の `handle`）と SHIORI の `GetProperty`（`crates/areka/src/shiori_host.rs` の `ShioriHostSink` の `GetProperty`）は、どちらも名前を手直しせず `SylphyaReader::resolve_dotted_str` へ渡す。値が無ければ前者は `NG:Cannot find such property name.`、後者は「プロパティが無い」の失敗で答える。台本の `%property[]` は語彙台帳の構文記録（`crates/areka-sylphya/src/vocab/flat.rs` の `SYNTAX_RECORDS`）に載るだけで、読む口はまだ無い。`\![get,property]` も無い（どちらも `property-query-channels` の持ち物）。
2. **書き込みの仕分けは 3 通り。** `classify_set` は ⑴ 書ける名前の表 `SET_EFFECTIVE` に完全一致 → 運行の値への書き込み（今の段では実際には書かず記録だけ）、⑵ 根が `DOTTED_ROOTS` に入るか葉が `GENERIC_PROP_NAMES` に入る（`contains` の完全一致）→ 書けない名前（書かずに記録・呼び出しは失敗にしない）、⑶ それ以外 → 自由な名前としてゴーストごとの区画へ保存、の順。`CURRENTGHOST.NAME` は ⑵ をすり抜けて ⑶ へ、`MENU` は ⑴ を外れて ⑶ へ落ちる。今の書き込みの入口は SHIORI の `SetProperty`（環境変数で開くデモ）と `ShioriHostSink::set_property_value` だけ。カウンタの名前の族を書く `PropSetCueSink` は型の決まった名前で保存へ書き、仕分けを通らない。
3. **語彙の表はすべて小文字。** `DOTTED_ROOTS`・`GENERIC_PROP_NAMES`・`SET_EFFECTIVE`（`crates/areka-sylphya/src/vocab/dotted.rs`）の英字はすべて小文字。
4. **保存された名前は小文字だけ。** 保存の名前 `PersistKey::to_canonical_key`（`crates/areka-sylphya/src/persist/mod.rs`）が作る名前（`areka.window.scope(0).x`・`areka.boot.count`・`areka.last.ghost` など）はどれも英字が小文字で、保存ファイルは型の決まった項目として読み戻される（自由な名前は保存ファイルに入らない）。＝読む側で畳めば、保存ファイルの中身と読み戻しを変えずに大小違いの名前で引ける。
5. **大小の扱いを固定しているテストは 0 本。** `crates/areka-sylphya/src/` と `crates/areka/src/mcp/get_property_tests.rs` のテストが聞く名前の英字はどれも小文字。`get_property_tests.rs` の ` baseware.name`（前に空白 1 つ）は「無い名前」を期待する＝前後の空白を削らないことを固定している。

### 要件の段で決めた細部（要件ディスカッションで覆せる）

| # | 細部 | 決めたこと | 根拠 | 載せた要件 |
|---|---|---|---|---|
| 1 | 畳む文字 | **半角の英字（A〜Z と a〜z）の大小だけを同じとみなす**。全角の英字の大小・ひらがなとカタカナ・全角と半角は同じとみなさない | `mcp-ghost-name-match`（宛先の名前の照合）と同じ決まりにそろえる。実測は半角の英字の大小違いだけ。広げると SSP で外れる名前が areka で当たる恐れがある | 1.1・1.3 |
| 2 | 前後の空白 | **削らない**（前後に空白のある名前は「無い名前」のまま） | SSP も削らない（survey §7.1）。`get_property_tests.rs` が固定している。宛先の名前の照合（`mcp-ghost-name-match`）は前後の空白を除くが、プロパティの名前は除かない＝2 つの決まりは空白の扱いだけ違う | 1.4 |
| 3 | 畳む範囲（名前のどの部分か） | **点付きの名前の全部**＝点で区切った各部分の名前と、括弧の中の名前（`ghostlist(名前)` の `名前`）の両方 | SSP は括弧の中の名前も大小を区別しない（survey §7.1）。ukadoc が唯一書く大小の規則（`developer.log.ログ種別(名前).count`）も括弧の中の名前を区別しない。括弧の中だけ別の決まりにすると、値を足す後の spec が名前ごとに決め直すことになる | 1.1・1.2 |
| 4 | 自由な名前（語彙に無い名前） | **含める**。大小だけ違う綴りで書いた自由な名前は同じ名前への書き込みになり、どの綴りで読んでも同じ値が返る | 読みだけ畳んで書きを畳まないと、書いた綴りで読めない値や、大小違いで 2 つ並び片方しか読めない値ができる | 3.3・3.4 |
| 5 | 保存された名前と既存の保存の読み戻し | **保存ファイルの中身と読み戻しの形は変えない**。読む側で畳むので、保存された値も大小違いの名前で引ける | 保存の名前はもともと小文字だけ（実物 4）。保存の側に触らなければ、同じ版の `readonly-mode` などと触るファイルが重ならない（棚卸㉕〜㉘の推し） | 4.1〜4.3 |
| 6 | 値 | **畳まない**（値は書かれた綴りのまま返す） | 大小を区別しないのは名前だけ（SSP の `currentghost.name` は `えも2DEBUG` を綴りのまま返す） | 1.5 |
| 7 | `shiori.変数名`（SHIORI の変数名を名前に含む形）の変数名の部分 | **決めない**。値がまだ無い | SHIORI の変数名は SHIORI によって大小を区別しうる。値を足す spec が、SHIORI へ渡す綴りと一緒に決める | Boundary Context |

## Boundary Context

- **In scope**:
  - 点付きのプロパティの名前を読むときに、半角の英字の大小を区別しないこと（各部分の名前・括弧の中の名前・自由な名前・保存された名前）。
  - 書き込みの仕分け（運行の値・書けない名前・自由な名前）が、名前の大小で変わらないこと。
  - 今ある 2 つの読む口（MCP の `get_property`・SHIORI の `GetProperty`）の答えがそろうこと。
  - 決定論テストと、配布形での実機確認。
  - 沈黙ルール対応表（`doc/COMPAT_ARCHITECTURE.md` §8）への行の追加と、survey §7.3 の 4 への着地の注記。
- **Out of scope**:
  - プロパティの値そのものの追加（一覧の値・`currentghost` の木の値など）。
  - 括弧の中の数字を番号と読むか名前と読むか（`ghostlist(0)`）と、括弧の中の名前で一覧から選ぶ処理そのもの（`property-catalog-lists` の持ち物）。
  - `shiori.変数名` の変数名の部分の大小（細部 7）。
  - MCP の宛先の名前（`ghost_name`）の照合（`mcp-ghost-name-match` で着地済み）。
  - `%` で始まる台本の変数（`%username` など・点の無い名前）の大小。
  - 台本の `%property[]`・`\![get,property]`・`\![set,property]` の口を開くこと（`property-query-channels` の持ち物）。
  - 保存ファイルの形・保存する名前の綴りの変更。
  - 書式として読めない名前（空・連続した点・閉じない括弧）を「無い名前」とする今の扱いの変更。
- **Adjacent expectations**:
  - MCP の `get_property` と SHIORI の `GetProperty` は、名前を手直しせずに読み手へ渡すまま変えない（`mcp-get-property` 要件 1.4「名前を手直ししない」と、その申し送り）。大小を畳むのは読み手の側の 1 か所で、2 つの口は何も変えずに追従する。
  - `property-query-channels` が台本の読む口・書く口を開くときは、同じ読み手と同じ書き込みの仕分けを通る前提で、本 spec の決まりに追加の作業なしで従う。本 spec はその spec より先に着地し、書く道が開く前に穴を塞ぐ。
  - v0.2.0 の `sylphya-sqlite-kvs` は読み書きと保存の口を作り直す。本 spec の決まり（名前の英字の大小を区別しない）を作り直しで落とさないことを、向こうの要件で引き継ぐ。
  - プロパティの動く値の列（`currentghost-property-tree` ほか）は、値を足すときに名前を小文字の正準の綴りで書けば、大小違いの聞き方にも自動で答える。

## Requirements

### Requirement 1: 名前の英字の大小を区別しない読み

**Objective:** As a SSP 向けに書かれたゴーストの作者, I want プロパティの名前の英字の大小を混ぜて書いても SSP と同じ値が取れること, so that SSP で動くゴーストが areka でも同じに動く

#### Acceptance Criteria

1. When 点付きの名前が、値のある名前と半角の英字（A〜Z と a〜z）の大小の違いだけで異なる, the areka shall 値のある名前で聞いたときと同じ値を返す（例 `BASEWARE.NAME`・`Baseware.Name` → `areka`）。
2. When 括弧の中の名前が、値のある名前の括弧の中の名前と半角の英字の大小の違いだけで異なる, the areka shall 同じ値を返す（例 `test(ABC).key` に値があるとき `test(abc).key` で同じ値）。
3. If 名前が値のある名前と、半角の英字の大小以外の点（全角の英字の大小・ひらがなとカタカナ・全角と半角）で異なる, then the areka shall 別の名前として扱い、その名前に値が無ければ「無い名前」と答える。
4. If 名前の前後に空白がある, then the areka shall 空白を削らずに比べ、空白を含む名前に値が無ければ「無い名前」と答える（例 ` baseware.name` は「無い名前」のまま）。
5. The areka shall 値の英字の大小を変えず、書かれた綴りのまま返す。
6. If 名前が書式として読めない（空・先頭や末尾や連続した点・閉じない括弧・数字でない `index(…)`）, then the areka shall 英字の大小にかかわらず、小文字で書いた同じ名前と同じく「無い名前」と答える（例 `INDEX(abc)` は `index(abc)` と同じ扱い）。
7. While 同じ名前にゴーストごとの値と全体の値の両方がある, the areka shall 大小違いの綴りで聞いても、小文字で聞いたときと同じくゴーストごとの値を返す。

### Requirement 2: 読む口どうしの答えがそろう

**Objective:** As a ゴーストの作者と AI エージェントの利用者, I want どの口から聞いても大小違いの名前に同じ答えが返ること, so that 口によって SSP との違いが残らない

#### Acceptance Criteria

1. When MCP の `get_property` に、値のある名前と半角の英字の大小だけが違う `property_name` が渡される, the areka shall 小文字の名前を渡したときと同じ答え（値そのもの・`isError: false`）を返す。
2. When ゴーストの SHIORI が `GetProperty` で、値のある名前と半角の英字の大小だけが違う名前を聞く, the areka shall 小文字の名前で聞いたときと同じ値を返す。
3. If MCP の `get_property` または SHIORI の `GetProperty` に、どの綴りでも値の無い名前が渡される, then the areka shall それぞれ今と同じ「無い名前」の答え（`get_property` は `NG:Cannot find such property name.`・`isError: true`、`GetProperty` は「プロパティが無い」の失敗）を返す。
4. The areka shall 同じ問い手が同じ名前を聞いたとき、MCP の `get_property` と SHIORI の `GetProperty` の答え（値があるか・どの値か）を一致させる。

### Requirement 3: 書き込みの検査が大小で迂回されない

**Objective:** As a areka の開発者, I want 書き込みの検査が名前の英字の大小で変わらないこと, so that 大文字で書くことで書けない名前を書き換えたり、本物の値を隠したりできない

#### Acceptance Criteria

1. When 書き込みの名前が、書けない語彙の名前と半角の英字の大小だけが違う（例 `CURRENTGHOST.NAME`・`Baseware.Name`）, the areka shall 小文字で書いたときと同じく書けない名前として扱い、何も書かず、書けない名前への書き込みとして記録に残し、呼び出しは小文字のときと同じ結果で返す。
2. When 書き込みの名前が、書ける名前（運行の値）と半角の英字の大小だけが違う（例 `MENU`・`Surface.Num`）, the areka shall 小文字で書いたときと同じく運行の値への書き込みとして扱う（今の段では実際には書かずに記録に残すことも同じ）。
3. When 語彙に無い自由な名前を大小を混ぜた綴りで書く, the areka shall その名前を小文字で書いたときと同じ名前への書き込みとし、どの大小の綴りで読んでも書いた値を返す。
4. If 半角の英字の大小だけが違う 2 つの綴りの自由な名前に続けて書く, then the areka shall 1 つの名前への 2 回の書き込みとして扱い、どちらの綴りで読んでも後に書いた値を返す。
5. If 書けない語彙の名前を大小違いの綴りで書いた後に、その名前を読む, then the areka shall 書く前と同じ値（本物の値・または「無い名前」）を返す（大小違いの書き込みが本物の値を隠さない）。

### Requirement 4: 保存された値

**Objective:** As a areka の利用者, I want 前の版で保存された設定や記録が、本 spec の着地の後もそのまま読めること, so that 版を上げても窓の位置や前回のゴーストの記憶が失われない

#### Acceptance Criteria

1. When 本 spec の着地より前に保存された値（窓の位置・バルーンの位置・起動の回数・前回のゴースト・シェル・バルーンなど）を起動時に読み戻す, the areka shall 保存ファイルを書き換えずに読み戻し、これまでと同じ名前で同じ値を返す。
2. When 保存された値の名前を半角の英字の大小だけ違う綴りで聞く（例 `AREKA.BOOT.COUNT`）, the areka shall 小文字で聞いたときと同じ値を返す。
3. The areka shall 保存ファイルの形と、保存ファイルに書き出す名前と値を、本 spec の着地の前後で同じにする。

### Requirement 5: 決定論テスト

**Objective:** As a areka の開発者, I want 大小を区別しない決まりが実行のたびに同じ結果になるテストで固定されていること, so that 後の変更で SSP とのずれや書き込みの穴が戻ったら赤で分かる

#### Acceptance Criteria

1. The areka shall アプリ本体やゴーストの実行系を要しない決定論テストで、読みの次の各場合を固定する: 各部分の名前の大小違い（同じ値）・括弧の中の名前の大小違い（同じ値）・全角の英字の大小違い（別の名前）・かなの違い（別の名前）・前後に空白のある名前（「無い名前」）・書式として読めない大文字の名前（「無い名前」）・値の綴りが変わらないこと・ゴーストごとの値が全体の値に先立つこと（大小違いの綴りでも）・保存された値の名前の大小違い（同じ値）。
2. The areka shall アプリ本体やゴーストの実行系を要しない決定論テストで、書き込みの仕分けの次の各場合を固定する: 書けない語彙の名前の大小違い（書けない名前）・書ける名前の大小違い（運行の値）・自由な名前の大小違いの書き込みと読み（同じ名前）・大小だけ違う 2 つの綴りへの続けての書き込み（後の値）・書けない語彙の名前を大文字で書いた後も本物の値が読めること。
3. The areka shall 実行系を通した決定論テストで、MCP の `get_property` に `BASEWARE.NAME` を渡すと `baseware.name` と同じ値が返り、` baseware.name`（前に空白）は `NG:Cannot find such property name.` のままであることを固定する。
4. The areka shall 決定論テストで、SHIORI の `GetProperty` に大小違いの名前を渡すと小文字の名前と同じ値が返ることを固定する。
5. The areka shall 既存のテストの期待（小文字の名前での読み書き・前後の空白を削らないこと・書式として読めない名前の扱い）を変えずに通す。

### Requirement 6: 実機確認と記録

**Objective:** As a areka の開発者, I want 配布形の areka で実際の呼び出しを当てて確かめ、決まりを対応表に残すこと, so that テストの外の経路でも SSP と同じ答えになることが分かり、ukadoc の例外であることが後から辿れる

#### Acceptance Criteria

1. When 配布形の areka を起動し、MCP の `tools/call` で `get_property` に `baseware.name`・`BASEWARE.NAME`・` baseware.name`（前に空白）を順に渡す, the areka shall それぞれ `areka`・`areka`・`NG:Cannot find such property name.` を返し、呼び方と答えを記録に残す。
2. The areka の記録 shall 沈黙ルール対応表（`doc/COMPAT_ARCHITECTURE.md` §8）に「プロパティの名前の英字の大小」の行を足し、決めたこと（半角の英字の大小だけを同じとみなす・前後の空白は削らない・値は畳まない・書き込みの仕分けも同じ決まり）と、ukadoc が黙っている所を SSP の実測（survey §7.1）に合わせた例外であることを書く。
3. The areka の記録 shall survey §7.3 の 4（「areka の読み手は大小を区別する」）に、本 spec の着地で違いが無くなったことの注記を 1 行足す。
