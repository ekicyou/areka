# Requirements Document

> 本文の実物の確認は **2026-10-05・本ブランチ**（main `44fc0a61`）のもの。事実の正本は [doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md) §7.2・§7.4（SSP 2.9.07 の実測）。MCP は ukadoc に索引されていない＝survey が MCP の約束の正本。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない。
> 「要件の段で決めた細部」の表は、brief が「要件の段で決める」とした細部と、要件を書く途中で答えが要った点に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。

## Project Description (Input)

**誰の何が困っているか**: AI エージェントで MCP のツールを使う人。エージェントがツールに渡す `ghost_name`（ゴースト名またはルートフォルダのフルパス）の照合が、SSP と 4 点ずれている。SSP で通る指定（名前の英字の大小違い・本体側名＝`sakura.name`・名前の前後の空白）が areka では `NG:Cannot find active ghost from specified name` になり、SSP で外れる空文字が areka では起動中の 1 体に当たる（`get_expression_table` では `NG:Specified ghost is not active` と別の文言になる）。`ghost_name` を受ける全ツールに共通する。

**今の状態**: 照合は `crates/areka/src/mcp/resolve.rs` の `resolve`（純粋な判断）1 か所で、起動中のゴーストの名前とルートフォルダは同じファイルの `active` が組む（本体側名は持たない）。今の形は完了 spec `mcp-tool-entrances` の要件 3.2〜3.5・3.9 と暫定の裁定 6・7（名前は大文字小文字も区別した完全一致・別名では照合しない・空文字は省略と同じ）で、SSP で確かめる前に決めたもの。テスト `crates/areka/src/mcp/resolve_tests.rs` と `crates/areka/src/mcp/mcp_tests.rs` の 1 本がこの形を固定している。

**何を変えるか**: `ghost_name` の照合が survey §7.4 の表と同じ答えになる（4 点のずれが無くなる）。引数の省略と空文字が区別される。今の形を固定していたテストが、SSP の実測の形の期待へ書き換わる。

> 起票: 2026-10-04 `/kiro-discovery`（`mcp-get-property` の要件ディスカッション 議題 2 で開発者が「起票する」と裁定）。2026-10-05 棚卸㉒でウェーブ C4-⑦ に置かれた。

## Introduction

### 誰が困っているか

- **AI エージェントでゴーストを作る・動かす人**: SSP で通っていた呼び方（`えも2debug` のような大小違い・本体側名 `むらさき`・前後に空白の付いた名前）が areka では外れ、エージェントは「ゴーストが居ない」と受け取る。逆に、空文字を渡したときに SSP なら外れるのに areka では起動中のゴーストに当たり、SSP と違う結果になる。
- **3 段目の各ツールの spec**: 宛先の解決に頼るだけで照合を自分で持たない約束（`mcp-get-property` の要件など）なので、この 1 か所が SSP と食い違うと全ツールが食い違う。

### SSP 2.9.07 の実測（survey §7.4 の要約）

| `ghost_name` | SSP の答え |
|---|---|
| 名前そのもの・名前の英字の大小違い（`えも2DEBUG`・`えも2debug`） | 見つかる |
| 本体側名（`sakura.name`＝`むらさき`） | 見つかる |
| 相方の名前（`kero.name`＝`エモ`）・かなの違い（`ムラサキ`） | `NG:Cannot find active ghost from specified name` |
| 名前の前後に空白（` えも2DEBUG`・`えも2DEBUG   `） | 見つかる |
| 空白だけ・空文字 | `NG:Cannot find active ghost from specified name`（`get_status`・`get_log`・`get_expression_table` のどれも） |
| フォルダ名だけ（`emo2`） | `NG:Cannot find active ghost from specified name` |
| `/` 区切り・末尾の区切りなしのフルパス | 見つかる |
| 前後に空白のあるフルパス | `NG:Cannot find active ghost from specified name` |

### 実物で引き直して確かめた点（2026-10-05）

1. **本体側名の在りか。** `crates/areka-parsers/src/package/model.rs` の構造体 `GhostNames` が `name`・`sakura_name`・`sakura_name2`・`kero_name` を持ち、`GhostSession::names()` がそれを返す。`resolve.rs` の `active` は今 `name` だけを取っている。
2. **descript の値の前後の空白。** `crates/areka-parsers/src/kv/parse.rs` の読み取りは、キーと値の前後の空白を除いて持つ（全角の空白も除く）。起動中のゴーストの `name`・`sakura.name` は前後に空白を持たない。
3. **`get_log` は自分で空文字を断っている。** `crates/areka/src/mcp/get_log.rs` の純粋な答え `answer` は、`ghost_name` が空なら解決を呼ばずに `Cannot find` と答え、それ以外は解決を呼んで失敗をすべて `Cannot find` に読み替え、解決できたら一覧に出る値（そのゴーストの名前）で記録を絞る。照合を直せば `get_log` にも同じ規則が効き、`get_log.rs` は変えずに済む。
4. **`get_expression_table` の空文字の文言は解決の中で決まる。** `crates/areka/src/mcp/mod.rs` の振り分け `dispatch` は解決の結果をそのまま `NG:` にする。空文字の文言を `Cannot find` へ変えるのに `mod.rs` を触らずに済む。
5. **今の形を固定しているテスト。** `resolve_tests.rs` の `name_differing_only_in_case_does_not_resolve`・`sakura_name_does_not_resolve`・`empty_or_omitted_with_use_active_resolves_to_the_active_one`・`empty_or_omitted_with_reject_is_not_active`・`no_active_ghost_omitted_is_not_active`（空文字の側）と、`mcp_tests.rs` の `get_expression_table_omitted_is_not_active_even_with_one_ghost`（空文字の側）。
6. **起動中のゴーストは 1 体だけ。** `crates/areka/src/ghost_session.rs` の置き場 `GhostSlot` は 1 体分。本体側名と別のゴーストの名前が重なる場面は今の areka では起きない。
7. **SSP の机での追試はできなかった。** 本要件を書いた時点で SSP の MCP（9801）は応答しなかった。下の表の「未実測」の細部は推奨案で決め、要件ディスカッションと実機確認（要件 6）で確かめる。

### 要件の段で決めた細部（要件ディスカッションで覆せる）

| # | 細部 | 決めたこと | 根拠 | 載せた要件 |
|---|---|---|---|---|
| 1 | 英字の大小の畳み方（ASCII だけか・全文字か） | **半角の英字（A〜Z と a〜z）の大小だけを同じとみなす**。全角の英字・ギリシャ文字などの大小は畳まない。ひらがなとカタカナ・全角と半角も同じとみなさない | 実測は半角の英字の大小違い（`えも2debug`）で通り、かなの違い（`ムラサキ`）で外れた。全角の英字の大小は未実測。畳む範囲を広げると、SSP で外れる指定が areka で当たる恐れがある | 1.1・1.3 |
| 2 | 削る空白の種類と、どこで削るか | **名前（`name`・`sakura.name`）との照合でだけ、前後の空白を除いてから比べる**。除く空白は descript の値を読むときに除くのと同じ文字（半角の空白・タブ・全角の空白など）。途中の空白は除かない。**フルパスとの照合では除かない** | 実測は名前の前後の半角の空白で通り、パスの前後の空白で外れた。全角の空白・タブは未実測。descript の `name` は読むときに同じ規則で前後が除かれている＝同じ規則で除けば食い違わない | 2.1〜2.3 |
| 3 | 本体側名と別のゴーストの名前が重なったときの順 | **決めない**。起動中のゴーストは 1 体だけなので、名前・本体側名・フルパスのどれか 1 つに一致すればそのゴーストへ解決する | brief の範囲外「多重起動（2 体以上）の照合の順」。今の areka では起きない | 1.2・Boundary Context |
| 4 | 本体側のもう 1 つの別名（`sakura.name2`） | **照合しない** | 実測は `sakura.name` だけ。`sakura.name2` は未実測。SSP の description は「ghost name」とだけ書く | 1.4 |
| 5 | 起動中のゴーストが 0 体のときの空文字・空白だけ | **`NG:Cannot find active ghost from specified name`**（空文字は「省略」でなく「外れた名前」）。引数そのものが無い（省略）ときは今のまま `NG:Specified ghost is not active` | SSP は常にゴーストが居るので 0 体は未実測。「空文字は外れた名前」という実測の読み方を 0 体にも当てる | 2.4・3.1 |

## Boundary Context

- **In scope**:
  - `ghost_name` を受ける 9 本（`get_status`・`get_expression_table`・`get_property`・`get_log`・`sakurascript`・`raise_event`・`reload`・`dump_surface`・`dump_balloon`）に共通する照合の規則（名前の英字の大小・本体側名・名前の前後の空白・空文字と空白だけ）。
  - `get_expression_table` に空文字・空白だけを渡したときの文言（`Specified ghost is not active` → `Cannot find active ghost from specified name`）。
  - 今の形を固定していたテストの書き換えと、照合の決定論テスト。
  - 実機での確認（survey §7.4 の表と同じ指定を当てる）。
- **Out of scope**:
  - 起動中の一覧（`get_active_ghost_list`）の値の形。
  - プロパティの名前の英字の大小（`property-name-case-fold` の持ち物）。
  - 多重起動（2 体以上）の照合の順（起動中のゴーストが 2 体以上になる spec の持ち物）。
  - 宛先が解決された後の各ツールの処理（各ツールの答えの中身は変えない）。
  - プロトコル側の引数の検査（型・必須の欄の欠落の `-32602`）。
  - フルパスの照合の規則（大文字小文字・`\` と `/`・末尾の区切りの差を同じとみなし、フォルダ名だけ・相対パス・上位のパスは外れる）は今のまま変えない（SSP と同じ）。
  - SSP の実測の記録（survey）そのものの書き換え。
- **Adjacent expectations**:
  - 完了 spec `mcp-tool-entrances` の要件 3.2〜3.5・3.9 と暫定の裁定 6・7 を本 spec が上書きする。完了した spec の文書は書き換えない（本書が新しい約束の正本）。同 spec の要件 4.4（一覧の値を `ghost_name` に渡すと同じゴーストへ解決する）は保つ。
  - `mcp-get-property` は宛先の解決に頼るだけで照合をテストで固定していない（同 spec の申し送り）。本 spec の着地で `get_property` の答えの中身は変わらない。
  - 同じウェーブ C4 の約束（棚卸㉒）: 本 spec が触るのは `crates/areka/src/mcp/` の `resolve.rs`・`resolve_tests.rs`・`mcp_tests.rs` だけ。`mcp/mod.rs`・各ツールのファイル・`crates/areka-mcp/`・`ghost_session.rs` は触らない。`mcp-author-tools` は `mcp_tests.rs` に触らない約束。この約束を破る必要が出たら止めて報告する。

## Requirements

### Requirement 1: 名前での照合（英字の大小・本体側名）

**Objective:** As a AI エージェントの利用者, I want SSP で通る名前の綴り（英字の大小違い・本体側名）で areka でもゴーストを指せること, so that SSP と同じ呼び方がそのまま areka で通る

#### Acceptance Criteria

1. When `ghost_name` が、起動中のゴーストの descript の `name` と、半角の英字（A〜Z と a〜z）の大小の違いを除いて一致する, the areka shall そのゴーストへ解決する（例 `えも2DEBUG` のゴーストに `えも2debug`）。
2. When `ghost_name` が、起動中のゴーストの descript の `sakura.name`（本体側名）と 1.1 と同じ規則で一致する, the areka shall そのゴーストへ解決する。descript に `name` が無いゴーストでも、`sakura.name` があればこの規則で解決する。
3. If `ghost_name` が `name`・`sakura.name` と半角の英字の大小以外の点で違う（ひらがなとカタカナの違い・全角と半角の違い・全角の英字の大小の違いを含む）, then the areka shall 本文 `NG:Cannot find active ghost from specified name`・`isError: true` の結果で答え、ツールの処理へ届けない。
4. If `ghost_name` が `kero.name`（相方の名前）・`sakura.name2` など、`name` と `sakura.name` 以外の名前にだけ一致する, then the areka shall 本文 `NG:Cannot find active ghost from specified name`・`isError: true` の結果で答え、ツールの処理へ届けない。
5. When `get_log` の `ghost_name` が 1.1・1.2 の規則（と要件 2.1 の空白の扱い）で起動中のゴーストへ解決する, the areka shall そのゴーストの記録（`get_active_ghost_list` に出るのと同じ名前で記録されたもの）に絞って返す（本体側名や大小違いの綴りで指しても、名前そのもので指したときと同じ記録が返る）。

### Requirement 2: 前後の空白と空文字

**Objective:** As a AI エージェントの利用者, I want 名前の前後に紛れ込んだ空白で外れず、空の指定ははっきり外れること, so that SSP と同じ結果になり、空の指定が知らないうちに起動中のゴーストへ当たらない

#### Acceptance Criteria

1. When `ghost_name` の前後に空白がある, the areka shall 名前（`name`・`sakura.name`）との照合では前後の空白を除いてから要件 1 の規則で比べる。除く空白は、descript の値を読むときに前後から除くのと同じ文字（半角の空白・タブ・全角の空白など）とする。
2. The areka shall `ghost_name` の途中の空白は除かずに比べる（途中の空白の数や種類が違う名前は一致しない）。
3. If 前後に空白のある `ghost_name` が、空白を除けばルートフォルダのフルパスに一致する, then the areka shall 本文 `NG:Cannot find active ghost from specified name`・`isError: true` の結果で答え、ツールの処理へ届けない（フルパスとの照合では前後の空白を除かない）。
4. If `ghost_name` が空の文字列、または空白だけである, then the areka shall `ghost_name` を受ける 9 本のどれでも（`get_expression_table` を含み、起動中のゴーストが 0 体のときも）、本文 `NG:Cannot find active ghost from specified name`・`isError: true` の結果で答え、ツールの処理へ届けない。

### Requirement 3: 引数の省略は今のまま

**Objective:** As a AI エージェントの利用者, I want `ghost_name` を書かずに呼んだときの振る舞いが変わらないこと, so that 省略で起動中のゴーストを使う今の呼び方がそのまま通り、空文字とだけ区別される

#### Acceptance Criteria

1. When `get_status`・`get_property`・`sakurascript`・`raise_event`・`reload`・`dump_surface`・`dump_balloon` の 7 本が `ghost_name` の引数そのものを持たずに呼ばれる, the areka shall 起動中のゴースト（1 体）へ解決する。ゴーストが起動していなければ、本文 `NG:Specified ghost is not active`・`isError: true` の結果で答え、ツールの処理へ届けない。
2. If `get_expression_table` が `ghost_name` の引数そのものを持たずに呼ばれる, then the areka shall 本文 `NG:Specified ghost is not active`・`isError: true` の結果で答え、ツールの処理へ届けない。
3. When `get_log` が `ghost_name` の引数そのものを持たずに呼ばれる, the areka shall ゴーストで記録を絞らずに答える。

### Requirement 4: 変えないもの

**Objective:** As a AI エージェントの利用者, I want SSP と既に同じだった照合と答えが変わらないこと, so that 今通っている呼び方が本 spec の着地で壊れない

#### Acceptance Criteria

1. When `ghost_name` が起動中のゴーストのルートフォルダのフルパスと一致する（大文字小文字・区切り〔`\` と `/`〕・末尾の区切りの有無の差は同じとみなす）, the areka shall そのゴーストへ解決する。
2. If `ghost_name` がフォルダ名だけ・相対パス・ゴーストのフォルダでない上位のパスである, then the areka shall 本文 `NG:Cannot find active ghost from specified name`・`isError: true` の結果で答え、ツールの処理へ届けない。
3. The areka shall `get_active_ghost_list` が返す値を `ghost_name` に渡すと、同じゴーストへ解決されるようにする（名前がある場合も、名前が無くフルパスが載る場合も）。
4. When `ghost_name` が解決できた, the areka shall そのツールの答えを、名前そのもので指したときと同じにする（照合の綴りによって答えの中身を変えない）。

### Requirement 5: 決定論テスト

**Objective:** As a areka の開発者, I want 照合の規則が実行のたびに同じ結果になるテストで固定されていること, so that 後の変更で SSP とのずれが戻ったら赤で分かる

#### Acceptance Criteria

1. The areka shall 照合の判断を、ゴーストの名前・本体側名・ルートフォルダの値を受け取る（アプリ本体やゴーストの実行系を要しない）判断として、次の各場合を決定論テストで固定する: 名前の英字の大小違い（一致）・本体側名（一致）・`name` が無いゴーストの本体側名（一致）・相方の名前（不一致）・`sakura.name2`（不一致）・かなの違い（不一致）・全角の英字の大小違い（不一致）・前後に空白の付いた名前（一致・半角の空白・タブ・全角の空白のそれぞれ）・途中の空白の違い（不一致）・空白だけ（`Cannot find`）・空文字（`Cannot find`・省略で起動中のゴーストを使う扱いと省略を断る扱いの両方・ゴーストが 0 体のときも）・省略（起動中の 1 体へ・断る扱いでは `Specified ghost is not active`・0 体では `Specified ghost is not active`）・前後に空白のあるフルパス（不一致）・`/` 区切りで末尾の区切りの無いフルパス（一致）・フォルダ名だけ（不一致）。
2. The areka shall 今の形を固定していたテスト（英字の大小違いの名前で不一致・本体側名で不一致・空文字を省略と同じに扱う・`get_expression_table` の空文字で `Specified ghost is not active`）を、SSP の実測の期待へ書き換える（古い期待と新しい期待を両方残さない）。
3. The areka shall ツールの振り分けを通した決定論テストで、`get_expression_table` に空文字・空白だけを渡すと `NG:Cannot find active ghost from specified name`、省略すると `NG:Specified ghost is not active` になることを固定する。
4. The areka shall 一覧の値（名前あり・名前が無くフルパス）を `ghost_name` に渡すと同じゴーストへ解決することを、決定論テストで固定し続ける。

### Requirement 6: 実機確認

**Objective:** As a areka の開発者, I want 配布形の areka に実際の MCP の呼び出しで survey §7.4 の表と同じ指定を当てて確かめること, so that テストの外の経路（受け口・振り分け・実際のゴーストの descript）でも SSP と同じ答えになることが分かる

#### Acceptance Criteria

1. When 配布形の areka を起動し、MCP の `tools/call` で survey §7.4 の表の各行と同じ形の `ghost_name`（名前・英字の大小違い・本体側名・相方の名前・かなの違い・前後に空白の付いた名前・空白だけ・空文字・フォルダ名だけ・`/` 区切りのフルパス・前後に空白のあるフルパス）を `get_status`・`get_expression_table`・`get_log` に当てる, the areka shall 表と同じ答え（見つかる／`NG:Cannot find active ghost from specified name`）を返し、呼び方と答えを記録に残す。
2. Where 起動中のゴーストの名前に半角の英字が含まれない, the areka の実機確認 shall 英字の大小違いの行を、名前に半角の英字を含むゴースト（survey の実測と同じ `えも2DEBUG` など）で確かめる。
3. Where 同じ机で SSP を動かせる, the areka の実機確認 shall 同じ指定を SSP にも当てて答えを並べ、「要件の段で決めた細部」の表の未実測の細部（全角の英字の大小・全角の空白とタブ・`sakura.name2`）の SSP の答えを記録に残す。
