# Requirements Document

> 本文の実測は **2026-10-04・本ブランチ**（main `e2a373b5`＝棚卸㉑の PR#229 のコミット。`mcp-tool-entrances`〔PR#223〕の着地の後）のもの。コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指し、行番号では指さない。
> 「要件の段での暫定の裁定」の表は、要件を書く途中で答えが要った点に対する**推奨案による暫定の確定**であり、要件ディスカッションで覆せる（覆したら該当要件も改める）。brief は「議題なし」としており、表の項目はどれも答えで作業がほとんど変わらない。

## Project Description (Input)

**誰の何が困っているか**: AI エージェント（Claude Code など）でゴーストを作る人は、プロパティシステム（`baseware.*`・`currentghost.*` など）を読んで状態を確かめたい。SSP では MCP のツール `get_property` がこれを担う。

**今の状態**: areka は SSP と同じ名前・引数で `get_property` を受け付けるが、中身はダミーで、どの名前にも `NG:not implemented yet` を返す（`mcp-tool-entrances` が置いたもの）。

**何を変えるか**: `get_property(property_name, ghost_name?)` が、SHIORI の `GetProperty` と同じ解決で値を素の文字列のまま返し、値の無い名前には `NG:Cannot find such property name.`（`isError: true`）と答える（survey §3: `currentghost.name` → `Emily/Phase4.5`）。本 spec の後にプロパティの値が増えれば、`get_property` 側は何もせずに答えが増える。

> 起票: 2026-09-29 `/kiro-discovery`。SSP MCP の移植の 3 段目（個別のツール）の 1 本で、ウェーブ C3-⑦（段「優先」）。事実の正本は [doc/ssp-mcp/survey.md](../../../../doc/ssp-mcp/survey.md)（SSP 2.9.05 の実測。`get_property` を名前の書き方ごとに当てた詳しい実測は §7＝2026-10-04・SSP 2.9.07）。前提の spec は [completed/areka-P0-mcp-tool-entrances](../areka-P0-mcp-tool-entrances/)（ツールのファイルの約束は同 design の「mcp/<ツール>.rs（10 本）」）。

## Introduction

### 誰が困っているか

- **AI エージェントでゴーストを作る人**: ゴーストに台本を流した後や設定を変えた後に、プロパティシステムで状態を確かめたい（SSP の description は「Get Ukagaka property system value」として ukadoc のプロパティシステムの一覧を指す）。今の areka は何を聞いても `NG:not implemented yet` で、値のある名前（`baseware.name` など）すら読めない。
- **後続のプロパティの spec の実装者**（`property-query-channels`・`property-ipc-transport`・`currentghost-property-tree`・`property-catalog-lists`）: 値を足したことを MCP から確かめる道が無い。roadmap の台帳は `currentghost-property-tree` の読む道に「`property-query-channels` か `mcp-get-property`」と書いている。

### いま何が起きているか（2026-10-04 実測）

- **ダミーの所在。** アプリ本体側 `crates/areka/src/mcp/get_property.rs` の `handle(_world, _ghost, _args, reply)` は、本体が `reply.send(outcome::ng("not implemented yet"))` の 1 文。テスト `crates/areka/src/mcp/get_property_tests.rs` は空の World で `NG:not implemented yet` を期待する（中身を入れたら書き換える）。
- **プロトコル側は完成済み。** `crates/areka-mcp/src/tools/get_property.rs` が定義の逐語（`DEFINITION`）と引数の型 `Args { property_name: String, ghost_name: Option<String> }` を持つ。`property_name` の欠落・型違いは手前の検査が `-32602` で拒む＝処理まで来るときは必ず文字列がある。
- **宛先の解決は手前で済む。** `crates/areka/src/mcp/mod.rs` の `dispatch` が `GetProperty` を「省略なら起動中の 1 体」（`Omitted::UseActive`）で解決してから `handle` を呼ぶ。0 体・名前違いは `handle` まで来ずに `NG:Specified ghost is not active`／`NG:Cannot find active ghost from specified name` で終わる。起動中に数えるのは「置き場（`GhostSlot`）に単位があり、その単位に実行系（`GhostSession::runtime()`）がある」とき（`crates/areka/src/mcp/resolve.rs` の `active`）。`handle` に来る `ActiveGhost` は名前とルートフォルダだけを持ち、実行系は持たない。
- **読み手は同期・待ち無し。** `crates/areka-sylphya/src/reader.rs` の `SylphyaReader::resolve_dotted_str` は、どのスレッドからも待たずに呼べ、結果は `DottedResolution::{Value(String), NotFound}` の 2 つ（`crates/areka-sylphya/src/value.rs`）。名前の書式が読めないときは `warn!` を残して `NotFound` に倒れる。引く順は「問い手ごとの値 → 全体の値」＝**誰として聞くか（問い手）で答えが変わりうる**。
- **SHIORI の `GetProperty` の区別。** `crates/areka/src/shiori_host.rs` の `ShioriHostSink::GetProperty` は、`Value(v)` なら `v` を返し（空の文字列も成功）、`NotFound` なら「プロパティが見つからない」の失敗を返す。値の無い名前を暗黙の空値で続けない。
- **実行系の読み手は外へ出ていない。** `crates/areka-ghost/src/runtime.rs` の `GhostRuntime` の欄 `sylphya_reader` は私有で、外へ出る道は `into_parts`（実行系を分解する）の `GhostParts.sylphya_reader` だけ。マウント（`GhostRuntime::mount()`）は公開済み。
- **そのゴーストの問い手。** 実行系の起動（`crates/areka-ghost/src/runtime.rs` の起動の手順）は、ゴースト自身の問い手を `areka_ghost::sylphya_wiring::ghost_asker_id(&mount.shiori.dir)` で組み、その問い手へ静的な値（`baseware.name`＝`areka`・`baseware.version`）を載せる。インストールの置換語の載せ直し（`crates/areka/src/install/names.rs` の `publish_to`）も同じ組み方を使う。
- **今ある値は少ない。** 本番で載る点付きの名前は `baseware.name`・`baseware.version`（と前回までに保存された値）だけで、`currentghost.name` などはまだ値が無い（値を足すのは後続の spec）。
- **答えの記録は済んでいる。** `crates/areka-mcp/src/tools/bridge.rs` の橋が、`tools/call` ごとに答えを `debug!` 1 行（`tool`・`ghost`・`is_error`・`text`）で残す。
- **共有のテストの縛り。** `crates/areka/src/mcp/mcp_tests.rs` の `get_log_and_seven_omitted_do_not_answer_with_a_resolve_failure` は、空の World・作った `ActiveGhost` で `get_property` を振り分け、答えが名前の解決の失敗の 2 つの文言（`NG:Specified ghost is not active`・`NG:Cannot find active ghost from specified name`）の**どちらでもない**ことを確かめる。このテストは共有ファイルで、本 spec は触らない。

### brief の記述を実物で引き直して改めた点

1. brief の「SHIORI の `GetProperty` と同じ解決」は、**解決の仕方と区別**（`resolve_dotted_str` の 2 つの結果を「値」と「見つからない」へ写す）を写す意味で読む。`ShioriHostSink` 自体は、今は実演とテストの経路でアプリ単位の記憶とともに組まれており、本番のゴーストの記憶とは別物。`get_property` が読む記憶は**宛先のゴーストの実行系の記憶**で、問い手は**そのゴースト自身の問い手**（起動の手順が静的な値を載せるときと同じ組み方）とする（要件 1.3）。
2. brief の注意「World に置き場が無いときも panic せず `NG:` で答える」の文言は、名前の解決の失敗の 2 つの文言を使えない（上の「共有のテストの縛り」に反する）。areka 独自の文言を 1 つ決める（暫定の裁定 1・要件 3.1）。

### 要件の段での暫定の裁定（要件ディスカッションで覆せる）

| # | 議題 | 暫定の裁定 | 根拠 | 載せた要件 |
|---|---|---|---|---|
| 1 | 宛先の実行系（記憶）が見つからないときの文言 | **`NG:Property system is not available`**（`isError: true`）＋`warn!` 1 件 | 本番の振り分けからは来ない場面（手前の解決が実行系の無い単位を起動中に数えない）だが、ツールのファイルの約束は「置き場が無くても panic せず `NG:`」。名前の解決の失敗の 2 文言と「無い名前」の文言のどれとも違う文にして、原因を取り違えないようにする。SSP の文言の型（`NG:` ＋大文字で始まる英文）に揃える | 3.1・3.2 |
| 2 | 値の前後の空白・改行 | **手を加えない**（足さず削らない） | survey §3 の「素の値」。survey §7.1 の SSP 2.9.07 の実測でも、値は `OK:` なしのそのまま、空の値（何もしていないときの `currentghost.status`）は空の本文・`isError: false`。SHIORI の `GetProperty` も値をそのまま返す | 1.1・1.2 |
| 3 | 名前の手直し（前後の空白の除去・大文字小文字の変換） | **しない**（受け取ったまま解決へ渡す） | SHIORI の `GetProperty` と同じ解決にする。手直しを足すと 2 つの口で答えがずれる。survey §7.1 の SSP 2.9.07 でも前後の空白は削られず「無い名前」になる。英字の大小は SSP は区別しない（`BASEWARE.NAME` → `SSP`・survey §7.3 の 4）が、ukadoc は黙っており、読み手は 3 つの口（`GetProperty`・`%property[]`・`get_property`）で共有なので、`get_property` だけでは畳まない。**要件ディスカッション 議題 1（2026-10-04）で開発者が `/kiro-discovery` での起票を裁定し、`property-name-case-fold` を起票した**（読み手の側で 3 つの口を一度にそろえる）。本 spec はその着地の後、何もせずに追従する | 1.4・4.7・5.6 |
| 4 | 実機確認 | **する**（起動中の emo2 に MCP のクライアントから 2 つの名前を聞く） | 決定論テストは本番の振り分けの経路を通らない部分がある。実機の記録と静的な証跡の二本立て | 4.6 |

## Boundary Context

- **In scope**: `get_property` の中身（宛先のゴーストの記憶から値を読んで答える）・「値がある（空を含む）」と「値が無い」の区別・宛先の記憶が見つからないときの答え・実行系の記憶を読む口を 1 本足すこと・決定論テスト・実機確認。
- **Out of scope**: プロパティの値そのものの追加（`currentghost.*`・一覧系・`status`・`zorder` など）・書き込み（SSP の MCP に `set_property` は無い）・`ghost_name` の解決の仕方の変更・他の 9 本のツール・SHIORI の `GetProperty` の振る舞いの変更・ツールの定義（`DEFINITION`）と引数の検査の変更・名前の括弧の読み方（`ghostlist(0)` を番号と読むか名前と読むか。ukadoc の正典は `ghostlist(ゴースト名/本体側名/パス)` と `ghostlist.index(ID)` で、SSP 2.9.07 は `ghostlist(0).name` を「無い名前」にする〔survey §7.3 の 5〕。一覧系の値を実装する `property-catalog-lists` の持ち物）・名前の英字の大小を区別せずに引くこと（`property-name-case-fold` の持ち物・議題 1）・`ghost_name` の照合を SSP 2.9.07 に合わせること（英字の大小・本体側名・前後の空白・空文字＝survey §7.4。`mcp-ghost-name-match` の持ち物・議題 2）。
- **Adjacent expectations**:
  - `mcp-tool-entrances` の振り分け（`dispatch`）が、`ghost_name` の省略を起動中の 1 体へ解決し、0 体・名前違いを `handle` の手前で `NG:` に終える。本 spec はこの振る舞いに頼り、変えない。照合の細部は SSP と 4 点ずれている（survey §7.4）が、直すのは `mcp-ghost-name-match`（要件ディスカッション 議題 2・2026-10-04 起票）。
  - 同じく橋（`bridge.rs`）が答えを `debug!` 1 行で記録する。本 spec は普通の答え（値・空の値・無い名前）に**独自の記録を足さない**。
  - 統一プロパティシステム sylphya の読み手が、名前の書式の誤りを `warn!` に残して「見つからない」に倒す。本 spec はこの記録を重ねて出さない。
  - 後続のプロパティの spec が値を足すと、`get_property` の答えが変更なしで増える。
  - 読み手が値を返す名前には、どれにも答える。前回までに保存された内部の名前（`areka.*`）も、SHIORI の `GetProperty` と同じく読める。`get_property` だけ名前を隠す仕組みは足さない（足すと 2 つの口で答えがずれる）。

## Requirements

### Requirement 1: 値のある名前に素の値で答える

**Objective:** As a AI エージェントでゴーストを作る人, I want プロパティの名前を渡すとその値がそのまま返ること, so that ゴーストの状態を SSP と同じ手順で確かめられる

#### Acceptance Criteria

1. When `get_property` が起動中のゴーストへ解決され、渡された名前に値がある, the areka shall その値を素の文字列のまま 1 つの本文にして返す（`OK:` を付けない・前後の空白や改行を足さず削らない・`isError: false`）。例: `baseware.name` → `areka`。
2. When 渡された名前の値が空の文字列である, the areka shall 空の本文（`isError: false`）で答え、`NG:` で答えない。
3. The areka shall 値を宛先のゴーストの記憶から、そのゴースト自身の問い手として読む（ゴーストごとの値と全体の値の両方があればゴーストごとの値を返し、別の問い手の値を返さない）。
4. The areka shall 渡された名前を受け取ったまま解決へ渡し、前後の空白の除去・大文字小文字の変換などの手直しをしない。
5. The areka shall `get_property` にその場で答える（返事を後から答える置き場に預けない・別のスレッドに問い合わせて待たない）。
6. When ゴーストが切り替わった後に `get_property` が呼ばれる, the areka shall 呼ばれた時点で起動中のゴーストの記憶から答え、切替の前のゴーストの記憶を読まない。

### Requirement 2: 値の無い名前に `NG:` で答える

**Objective:** As a AI エージェントでゴーストを作る人, I want 値の無い名前と値が空の名前を見分けられること, so that 「まだ無い」を「空」と取り違えない

#### Acceptance Criteria

1. If 渡された名前に値が無い, the areka shall 本文 `NG:Cannot find such property name.`（`isError: true`）で答える。
2. If 渡された名前が空の文字列、またはプロパティの名前として読めない書式（`baseware..name`・末尾の点 `baseware.name.` など）である, the areka shall 要件 2.1 と同じ本文で答える（survey §7.1 の SSP 2.9.07 も同じ本文）。
3. While プロパティシステムがその名前の値をまだ持っていない（2026-10-04 時点の `currentghost.name` など）, the areka shall 要件 2.1 と同じ本文で答え、偽の値や空の本文で答えない。

### Requirement 3: 答えられないときも止まらずに `NG:` で答える

**Objective:** As a AI エージェントでゴーストを作る人, I want どんな場面でも答えが返り、areka が落ちないこと, so that ツールの呼び出しでゴーストを止めずに済む

#### Acceptance Criteria

1. If 宛先は解決されたが、その時点で宛先のゴーストの実行系（記憶）が見つからない, the areka shall 本文 `NG:Property system is not available`（`isError: true`）で答え、宛先のゴーストと渡された名前を添えた `warn!` を 1 件残す。
2. The areka shall 要件 3.1 の本文を、名前の解決の失敗の 2 つの本文（`NG:Specified ghost is not active`・`NG:Cannot find active ghost from specified name`）とも、要件 2.1 の本文とも違うものにする。
3. The areka shall `get_property` のどの入力・どの場面でも panic しない。

### Requirement 4: 決定論テストと実機確認

**Objective:** As a areka の開発者, I want 判断の分岐がすべてテストで固定され、実機でも確かめられていること, so that 後続の spec が値を足しても答え方が崩れない

#### Acceptance Criteria

1. The areka shall 次の判断の分岐をそれぞれ決定論テストで固定する: ⑴ 値がある → その値・`isError: false`、⑵ 値が空 → 空の本文・`isError: false`、⑶ 値が無い → `NG:Cannot find such property name.`・`isError: true`、⑷ 宛先の実行系が見つからない（空の World）→ `NG:Property system is not available`・`isError: true`・`warn!` 1 件、⑸ 問い手の選び方 → 宛先のゴーストの問い手の値を返し、別の問い手の値を返さない。
2. The areka shall 各テストで、答えが呼び出しの直後に（後から答える置き場を回さずに）得られることを確かめる。
3. The areka shall `NG:not implemented yet` を期待していた既存のテストを、新しい振る舞いの期待へ書き換える（ダミーの期待を残さない）。
4. The areka shall 共有のテスト（`crates/areka/src/mcp/mcp_tests.rs`・`resolve_tests.rs`、`crates/areka-mcp` のテスト）を 1 行も変えずに緑のまま保つ。
5. The areka shall 「呼ばれた時点の今のゴーストを読む」こと（要件 1.6）と答えの記録の行（橋の `debug!`）には専用のテストを足さない（判断の分岐でなく、手前の振り分けと橋が確かめ済みの配線であるため）。
6. When 実装が済んだ, the areka shall 実機で emo2 を起こし、MCP のクライアントから `get_property` を呼んで、⑴ `baseware.name` → `areka`（`isError: false`）、⑵ 値の無い名前 → `NG:Cannot find such property name.`（`isError: true`）を確かめ、結果を本 spec の `verification/` に残す。
7. The areka shall 英字の大小だけが違う名前（`BASEWARE.NAME` など）の答えを固定するテストを置かない（「無い名前」と期待するテストも、値が返ると期待するテストも置かない）。大小の扱いは `property-name-case-fold` が読み手の側で決めるので、本 spec のテストがその spec の着地を赤で妨げないようにする。
8. The areka shall `ghost_name` の照合（英字の大小・本体側名・前後の空白・空文字）の答えを固定するテストを本 spec に置かない。照合は手前の振り分けの持ち物で、`mcp-ghost-name-match` が SSP 2.9.07 の形（survey §7.4）へ直すので、本 spec のテストは `ghost_name` を省略するか、起動中のゴーストの名前を完全一致で渡す形だけを使う。

### Requirement 5: 触る範囲

**Objective:** As a 同じウェーブ（C3）で並走する spec の実装者, I want 本 spec が触るファイルが決まった少数に限られること, so that 互いのファイルを奪い合わずに並走できる

#### Acceptance Criteria

1. The areka shall 本 spec で変更するソースファイルを `crates/areka/src/mcp/get_property.rs`・`crates/areka/src/mcp/get_property_tests.rs`・`crates/areka-ghost/src/runtime.rs` の 3 つに限る（新しいファイルが要るなら `get_property.rs` の子モジュールとして足す。ほかを触る要が出たら止めて報告する）。
2. The areka shall `crates/areka-ghost/src/runtime.rs` への変更を、実行系の記憶を読む口 1 本の追加に限り、既存の欄・既存の読み口・分解の形（`into_parts` と `GhostParts`）・起動と終了の手順を変えない。
3. The areka shall 次のファイルを変えない（変える行 0）: `crates/areka/src/mcp/mod.rs`・`crates/areka/src/mcp/resolve.rs`・`crates/areka-mcp/src/` の下の全ファイル（`handler.rs` を含む）・`crates/areka/src/main.rs`・`crates/areka/src/ghost_session.rs`・`crates/areka/src/shiori_host.rs`・`crates/areka-sylphya/` の下の全ファイル・すべての `Cargo.toml`（新しい依存 0）。
4. The areka shall プロパティの値を 1 つも足さず、既存の値の載せ方を変えない。
5. The areka shall 1 ファイル 1,000 行以下の目安を、変更する 3 つのファイルすべてで守る。
6. When 本 spec の設計でテストの組み方と読み手の呼び方が決まったとき、および本 spec を完了するとき（`/kiro-complete`）, the areka shall `.kiro/specs/areka-P0-property-name-case-fold/brief.md` の末尾に「`mcp-get-property` からの申し送り（日付）」の節を足し（設計のときに書き、完了のときに実装の事実で書き直す）、次の 3 点を書く: ⑴ `get_property` は渡された名前を手直しせず読み手（`SylphyaReader::resolve_dotted_str`）へ渡すので、読み手の側で大小を畳めば `get_property` は何も変えずに追従する、⑵ `get_property` のテストは大小の扱いを固定していない（要件 4.7）、⑶ `get_property` の実機確認の手順（要件 4.6）のどこに大小を混ぜた名前の確認を足せばよいか。この追記は spec 文書の変更で、要件 5.1 の「変更するソースファイル」に数えない。`property-name-case-fold` が本 spec より先に着地していたら、追記の代わりに、その着地の後の振る舞いを本 spec の要件 1.4 と実機確認に反映する。
7. When 本 spec の実装の最終段階（最後のタスクの実機確認が済み、`/kiro-complete` に入る前）, the areka shall `.kiro/specs/areka-P0-mcp-ghost-name-match/brief.md` の末尾に「`mcp-get-property` からの申し送り（日付）」の節を足し、実装の事実で次の 3 点を書く: ⑴ `get_property` の処理が受け取った宛先（`ActiveGhost`）を何に使っているか（記録の欄だけか・実行系を引く鍵にしているか）＝照合を直しても `get_property` 側を変えずに済むかの判断材料、⑵ `get_property` のテストが `ghost_name` をどう渡しているか（要件 4.8＝省略か完全一致だけ）、⑶ 実機確認（要件 4.6）で `ghost_name` を渡した呼び方と、その答え。この追記は spec 文書の変更で、要件 5.1 の「変更するソースファイル」に数えない。`mcp-ghost-name-match` が本 spec より先に着地していたら、追記の代わりに、その着地の後の照合で本 spec のテストと実機確認が通ることを確かめる。
