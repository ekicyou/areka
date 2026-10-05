# ギャップ分析: areka-P0-mcp-author-tools

> 実測は 2026-10-05・本ブランチ（main `ec072853` の上）。静的な読み取りだけで、ビルドとテストは回していない。
> コードは「何の定義か」（関数名・型名・定数名＋ファイルパス）で指す。
> この文書は分析と選択肢を示すもので、どの案を採るかは要件ディスカッションと設計で決める。

## 1. まとめ

- **登録口と橋は、足す場所がはっきりしている。** `ToolRegistry::register`（`crates/areka-mcp/src/registry.rs`）は SSP の表かどうかを見ない。10 本の固定は `TABLE`（`crates/areka-mcp/src/tools/mod.rs`）と、それを読むテストにだけ在る。`TABLE` と `entrances` に触らず、別の表と「両方を 1 本の受け口へ登録する関数」を足せば、既存のテストは書き換えずに済む。
- **`areka_list_capabilities` の最大の穴は、台帳がバイナリに入っていないこと。** 網羅台帳とカタログは `doc/ukadoc-coverage/` にだけ在り、本番コードはどこからも読んでいない。配布の zip にも `doc/` は入らない。埋め込む形（3 案）を決める必要がある。
- **`areka_check_script` の最大の穴は、再生の経路に「知らない」を決める 1 か所が無いこと。** 知らないタグは解釈器が 1 か所で決めるが、知らない `\!` コマンドは「誰も拾わなかった」というだけで、判定する関数が無い。台本の中の位置も、解釈の途中で捨てている。`\i`（アニメーション）と `\&`（実体参照）は、いまの areka では「知らないタグ」と同じ扱いになる。
- **`areka_validate_ghost` の最大の穴は、設定の読み取りが黙って捨てる作りであること。** `areka-parsers` の読み取りはどれも「失敗しない・何も報告しない」約束で、読めない行・読まなかったキー・行番号を返さない（例外は `surfacetable.txt` の読み取りだけ）。要件 5.9（読み込みと検査で別々の読み取りを持たない）を満たすには、読み取りの側に手を入れる必要がある。ここが規模を押し上げる。
- **規模の見立ては L（上限の 20 タスクに届く見込み）。** 登録口＋一覧＋台本の検査で M〜L、設定の検査を足すと L を超えうる。brief の「超えたら設定の検査を切る」の判断材料を 5 章に置いた。

## 2. いまの作り（調べた結果）

### 2.1 MCP のツールの登録と橋

| 何 | どこ | 分かったこと |
|---|---|---|
| 登録口 | `ToolRegistry::register`（`crates/areka-mcp/src/registry.rs`） | 定義（`ToolSpec`）と実装（`ToolHandler`）の対を登録順で持つ。同じ名前は後勝ちで `warn!`。表の種類は見ない |
| 10 本の表 | `TABLE`（`crates/areka-mcp/src/tools/mod.rs`） | 要素数 10 の配列。各行は「定義の JSON 文字列」と「引数を `ToolCall` へ詰め替える関数」 |
| 表を登録する関数 | `entrances`・`register_rows`（同上） | `register_rows` が送り口と受け口の対を**自分で作る**。別の表を同じ受け口へ足す口は、いまは無い |
| 呼び出しの型 | `ToolCall`（同上） | 10 変種の enum。`name()` が記録用のツール名を返す |
| 橋 | `bridge::call`・`ToolRequest`・`ReplyTo`・`Pending`（`crates/areka-mcp/src/tools/bridge.rs`） | 送って待つ・上限（10 秒）・終了の途中・記録（`MCP: ツールに答えた`）をここ 1 か所で持つ。`ToolCall` の中身には依らない |
| 一覧の並び | `ArekaHandler::new`・`list_tools`（`crates/areka-mcp/src/handler.rs`） | 登録順のまま返す（rmcp の名前順の並べ替えを避けて、登録順の列を自前で持つ）。**登録順＝一覧の並び**なので、SSP の 10 本を先に登録すれば要件 1.3 は満たせる |
| 引数の検査 | `check_arguments`（`crates/areka-mcp/src/check.rs`） | 必須の欄と型（`string`・`boolean`・`integer`・`array`）を見て `-32602`。`ghost_name` は必須でも調べない。`enum` は見ない |
| 未登録の名前 | `ToolRouter::call`（rmcp。`handler.rs` の冒頭の説明） | `-32602`。要件 1.7 は手を入れずに満たす |
| アプリ側の振り分け | `dispatch`（`crates/areka/src/mcp/mod.rs`） | `ToolCall` の `match`。`ghost_name` の扱い（省略を許すか）をここで決める。変種を足すとここに腕が要る（網羅の `match` なので足し忘れはコンパイルで分かる） |
| 宛先の解決 | `resolve`・`Omitted`・`NOT_ACTIVE`・`CANNOT_FIND`（`crates/areka/src/mcp/resolve.rs`） | 起動中の 1 体に名前かフルパスで当てる純粋な関数。当たらなければ `CANNOT_FIND`。**フォルダとして検査へ回す枝は無い** |
| 後から答える | `later`・`McpLater`（`crates/areka/src/mcp/mod.rs`） | その場で答えられない処理が、返事と「覗く関数」を預ける。`dump_surface` は別スレッドで符号化して後から答える（`crates/areka/src/mcp/dump_surface.rs` の `std::thread::Builder` の呼び出し） |
| 結果の形 | `ToolOutcome`・`ToolContent`（`registry.rs`）、`outcome::value`・`ok`・`ng`・`with_image`（`crates/areka-mcp/src/tools/outcome.rs`） | 文字列と画像だけ。構造化した結果（`structuredContent`）を運ぶ欄は無い |
| 本番の結線 | `fn main()`（`crates/areka/src/main.rs` の `areka_mcp::tools::entrances` の呼び出し） | `entrances` が返す登録表を `areka_mcp::start` へ、受け口を `mcp::install` へ渡す |

**10 本を固定しているテスト**（要件 1.4 が「書き換えずに緑」を求めるもの）:

- `crates/areka-mcp/src/tools/tools_tests.rs`: `table_has_ten_rows_in_ssp_order`（`TABLE.len() == 10`）・`definitions_are_verbatim_copies_of_saved_list`（`TABLE` と保存した JSON の照合）・`entrances_registers_ten_in_ssp_order`（`entrances` の登録表が 10 本）。
- `crates/areka-mcp/src/tools/tools_socket_tests.rs`: `tools_list_matches_saved_ssp_json`（`entrances` で立てたサーバの `tools/list` が保存した JSON と配列ごと一致）・`ping_answers_while_a_call_waits`（`tools/list` が 10 件）。

どれも `TABLE` か `entrances` を読んでいる。**`entrances` が 13 本を返すように変えると、この 3 本が赤になる。** `entrances` は SSP の 10 本のまま残し、本番（`main.rs`）が呼ぶ関数を別に足す形なら、既存のテストは 1 行も変えずに済む。`ToolCall` に変種を足しても、これらのテストは `TABLE` の行しか回らないので影響しない。

**前例の形（1 本のツール）**: `crates/areka-mcp/src/tools/get_expression_table.rs`（`DEFINITION`・`Args`・`parse`）と `crates/areka/src/mcp/get_expression_table.rs`（`handle` と純粋な `render`・`load`）、テストは同じフォルダの `_tests.rs`。独自のツールも同じ 2 ファイル＋テストの形に乗る。

### 2.2 指示文と登録案内

- `INSTRUCTIONS`（`crates/areka-mcp/src/handler.rs`）は 3 文の英文の定数。`crates/areka-mcp/src/server_protocol_tests.rs` は定数を参照して照合しているので、文を足してもテストは追随する。
- `help_html`（`crates/areka-mcp/src/help.rs`）は引数がポート番号だけ。呼び出しは `help`（`crates/areka-mcp/src/dispatch.rs`）で、`State` が持つのもポート番号と rmcp のサービスだけ。**登録した定義を help へ渡す道が無い**（要件 6.3）。`help_html` の引数を増やすと `crates/areka-mcp/src/help_tests.rs` の呼び出しは直すことになる（SSP の逐語一致のテストではないので、要件 1.4 には当たらない）。
- `.kiro/steering/roadmap.md` の「MCP の 3 段目の約束」は「`mcp/mod.rs`・`handler.rs` は触らない」「`INSTRUCTIONS` の `not implemented yet` の 1 文は `mcp-strict-errors` が消す」と書く。本 spec は共有ファイルを触る側なので、この約束を要件 1.6 の記述と合わせて書き直すことになる。

### 2.3 台本の解釈と「知らない」の決まる場所

再生の経路は 3 段: 字句（`crates/areka-parsers/src/sakura/lexer.rs`）→ 意味（同 `decode.rs`）→ 台本の組み立て（`compile`・`crates/areka-sakura/src/compile.rs`）→ 各消費者（seriko・文字の層・`emo2_boot` の受け口）。

| 種類（strict の 6 類＋引数） | 再生の経路でどう決まるか | 検査と共有できる形か |
|---|---|---|
| 知らないタグ | `decode_tag`・`decode_bare`（`decode.rs`）が腕の無い綴りを `decode_passthrough_tag`・`decode_passthrough_bare` で `Instruction::Raw` にする。`compile` の最後の腕が `debug!`（`M-boot 外タグを無視`）で捨てる | **できる**。`Instruction::Raw` になったもの＝知らないタグ、で再生と同じ判定になる。未閉じの `[` などの壊れた断片も同じ `Raw` に入る（`decode_passthrough_raw`）ので、種類を分けるなら字句の段の情報が要る |
| 知らない `\!` コマンド | `decode_bang` が `move` 以外を全部 `Instruction::GenericCommand` にし、`compile` が中身を見ずに汎用の cue へ載せる。受け口はそれぞれ自分の名前で拾う（`MoveCueSink`・`ZOrderCueSink` など `crates/areka/src/emo2_boot/*_cue.rs`、seriko の `bind`、文字の層の `\f`）。**誰も拾わない名前は、どこにも記録されずに消える** | **そのままではできない**。「知っている `\!`」を返す関数が本番に無い。宣言の表 `ConsumerLedger::canonical`（`crates/areka/src/emo2_boot/consumer_ledger.rs`・15 行）は在るが、本番コードはこれを引かない（ファイル先頭に `#![allow(dead_code)]`、呼び出しは自分のテストだけ）。表に無いが再生で意味を持つものもある: `move`（表に在る）・`set,choicetimeout`（`compile` が読む・表に無い）・`\![*]`（`fold_choice_marker` が畳む・表に無い） |
| 知らない `\&` 実体参照 | `decode_tag` に `&` の腕が無いので `Raw`。dola の `CueCommand::EntityRef` は「届かない」と書かれた防御の腕だけ（`crates/areka-seriko/src/actor.rs` の `EntityRef` の腕） | いまは**全部の `\&` が「知らないタグ」**になる |
| 無い surface（`\s`） | `SurfaceResolver::resolve`（`crates/areka-seriko/src/resolve.rs`）。数値は範囲だけ見て `Show(id)` にし、**シェルに在るかは見ない**。名前は別名の表に無ければ `Unresolved`（呼び手が `error!`）。数値の ID がシェルに無いことは、表示の層まで行って初めて分かる | 別名は `SurfaceResolver::resolve` を共有できる。数値の ID は、`dump_surface` の判断が使っている `ShellFacts::surface_exists`（`crates/areka/src/mcp/dump_surface_judge.rs`。本番は表示の層の `has_surface`）が前例 |
| 無いアニメーション（`\i`） | `\i` は `decode.rs` に腕が無く `Raw`（`decode_bare` の説明に「`\i` `\j` … はパススルー」と明記） | いまは**全部の `\i` が「知らないタグ」**。「このシェルに無いアニメーション」を判定する再生の経路が無い |
| 無いバルーン（`\b`） | `resolve_balloon_key`（`resolve.rs`）。数値は範囲だけ、名前の形は文字の層へ（`crates/areka-emo-text/src/state_route.rs` の「`\b[名前]` の箱が今のサーフェスに無い」の `warn!`） | 数値の ID が今のバルーンに在るかを答える関数は未確認（調べもの）。名前の形は「今表示しているサーフェスの箱」に依るので、台本の字面だけでは決まらない（要件 3.8 に当たる可能性） |
| 引数が読めない | 散らばっている。意味の段は黙って既定へ落とす（`wait_absolute_ms`・`newline_ratio_from_arg`・`speaker_scope_n`＝`decode.rs`・記録なし）。`compile` の `set,choicetimeout` は `warn!`。`\f` は文字の層（`crates/areka-emo-text/src/state_decoration.rs` の「`\f` の指定を適用できない」）。`\_l` は `cursor_tag.rs`。`move`・`zorder`・`bind` は各受け口 | 判定が消費者のクレートに在り、記録と一体になっている。検査と共有するには、判定を純粋な関数として取り出す必要がある。**全数の表（要件 3.12）を作るには棚卸しが要る** |

**位置について**: `Instruction`（`crates/areka-parsers/src/sakura/model.rs`）は台本の中の位置を持たない。字句の内側の `scan`（`lexer.rs`）は各トークンの範囲（`Range<usize>`・バイト）を出していて、`substitute_system_vars` が使っているが、`lex` はそれを捨てる。要件 3.5 の「台本の中の位置」には、位置を持ったまま解釈する入口を足す必要がある（`parse` の返す型は多くの消費者が使うので、変えずに別の入口を足す形が安全）。

**`sakurascript` ツールはまだダミー**（`crates/areka/src/mcp/sakurascript.rs` の `handle` は `NG:not implemented yet`）。要件 3.11 の「再生したとき」は、SHIORI から来た台本の再生の経路で比べることになる。

### 2.4 網羅台帳とカタログ

| 何 | 実測 |
|---|---|
| 台帳 | `doc/ukadoc-coverage/ledger/sakura-script.toml`（342 項目・409 KB）・`shiori.toml`（677 項目・1,163 KB）。1 項目は `status`・`alias_of`・`note` など。**見出しの綴りと URL は台帳に無い** |
| カタログ | `doc/ukadoc-coverage/catalog.toml`（544 KB・ukadoc の 1,749 項目）。項目 id → `page`・`title`（見出しの綴り）・`url`。要件 4.3 の「綴り・URL」はここから取る |
| 状態の内訳（2 台帳の合計 1,019） | `absent` 517・`vocabulary-only` 185・`not-applicable` 170・`implemented` 119・`alias` 23・`degraded` 5・`unclassified` 0 |
| 種類の手がかり | さくらスクリプトの台帳: id が `\![` で始まる 198・その他の `\` で始まる 115・`%` で始まる 28・どれでもない 1（「環境変数の記述例」の見出し）。SHIORI の台帳: `list_shiori_event` 290・`list_shiori_event_ex` 168・`list_shiori_resource` 159・`list_plugin_event` 19・`spec_*` と `memo_*` 計 41 |
| 本番からの到達 | **無い**。本番コードで `doc/` を `include_str!` している所は 0。`crates/areka` に `build.rs` も `toml` の依存も無い。台帳を読むコードは `crates/ukadoc-survey` だけで、その `Cargo.toml` は「areka の実行時コードからは参照されない」と書く |
| 配布形 | `tools/package.ps1` が zip に入れるのは `areka.exe`・補助 exe・`ghost`・`balloon`・文書 4 本だけ。`doc/` は入らない |

要件 4.1 は「タグ・`\!` コマンド・SHIORI イベント」の 3 種類と書くが、台帳には `%` の変数（28）と、イベント以外の SHIORI の項目（リソース 159・プラグイン 19・仕様の節 41）が在る。要件 4.7 は「台帳と過不足なく一致」と書くので、**この 248 項目（`%` 28・見出しだけ 1・SHIORI のイベント以外 219）をどの種類で返すか（または返さないか）** を決める必要がある。

### 2.5 設定ファイルの読み取り

| 何 | どこ | いま出す誤り・注意 |
|---|---|---|
| `key,value` の読み取り | `parse_kv`（`crates/areka-parsers/src/kv/parse.rs`） | 何も出さない。カンマの無い行・キーが空の行は黙って飛ばす。同じキーは後勝ち。行番号も順序も残らない |
| ゴーストの descript・シェルの解決 | `resolve`・`resolve_with_shell`（`crates/areka-parsers/src/package/`） | 致命の 3 種だけ `MountError`（`StartPointMissing`・`StartPointUnreadable`・`ShellDirMissing`）。着せ替えの名前の不完全な行は `warn!`（`resolve.rs`） |
| `surfaces.txt` | `shell::parse`（`crates/areka-parsers/src/shell/`） | 何も出さない。字句の `Token::Raw`（孤立した `{` `}`・閉じていないブレス）は `decode` が黙って捨てる。知らない先頭語の行も黙って飛ばす。知らない語彙は値のまま運ぶ（`Interval::Other`・`DrawMethod`）。下流が `debug!`・`warn!` を出す（`crates/areka-seriko/src/table.rs` の「非採録」など） |
| バルーンの descript | `balloon::parse`・`parse_str`（`crates/areka-parsers/src/balloon/parse.rs`） | 何も出さない。キーは完全一致で引き、無い・数でない・範囲外は `None`。文字の層が後から `warn!`（`crates/areka-emo-text/src/balloon_overrides.rs`） |
| `surfacetable.txt` | `parse_surfacetable`（`crates/areka-parsers/src/shell/surfacetable.rs`） | **読めない行の行番号を `SurfaceTable::unreadable` で返す**。唯一の「診断を値で返す」前例 |
| シェルの列挙・ゴーストのフォルダの判定 | `list_all_shells`・`is_ghost_dir`（`crates/areka-ghost/src/catalog.rs`） | 要件 5.2・5.3 の部品として使える |
| 文字コード | `charset::decode`（`crates/areka-parsers/src/charset/`） | 化けは `debug!` だけ |
| 検体 | `SampleRoot::acquire`（`crates/sample-ghost-kit/src/lib.rs`）。`emo2`・`R_POST_and_KOMAINU`・`konnoyayame`・`StayseeBalloon`・`claudia` を登記済み | 要件 5.10 のテストの土台。`vendors/sample_ghost/<名>.nar` を展開して使う（ネットへ出ない） |

**「areka が読むキー」の一覧は、コードのどこにも無い。** 各読み取りが自分の要るキーを完全一致で引くだけで、引かれなかったキーを集める仕組みが無い。要件 5.5（読まずに捨てるキーを注意として返す）と 5.9（読み込みと同じ読み取り）を両方満たす材料が、いまは存在しない。

## 3. 要件ごとの対応表

凡例: **無い**＝部品が無い ／ **不明**＝調べものが要る ／ **制約**＝既存の作りが縛る。

| 要件 | 使える既存の部品 | 足りないもの |
|---|---|---|
| 1.1〜1.3 登録と並び | `ToolRegistry`・登録順の一覧 | **無い**: 独自の表、2 つの表を 1 本の受け口へ登録する関数。**制約**: `entrances` と `TABLE` は変えない |
| 1.4 10 本を変えない | 既存のテスト 5 本 | なし（上の制約を守れば満たす） |
| 1.5 定義 | `spec_from_definition` | 定義の文面 3 本 |
| 1.6 触るファイルの固定 | roadmap の「3 段目の約束」 | 書き直し（設計の成果物） |
| 1.7 未登録の名前 | rmcp が `-32602` | なし |
| 2.1 引数の検査 | `check_arguments` | **制約**: `enum` は見ないので、要件 4.6 の「決められた語でない」は処理の側で `NG:` にする（要件どおり） |
| 2.2・2.6・2.7 `NG:`・時間切れ・記録 | `outcome::ng`・`bridge::call` | 橋を通せばそのまま満たす。**橋を通さずに答える案では記録を自前で出す必要がある** |
| 2.3 宛先の解決 | `resolve`・`resolved!` | なし（要件 5.2 の枝は別） |
| 2.4 何もさせない | 前例のテスト（`crates/areka/src/mcp/get_expression_table_tests.rs`） | 3 本ぶんのテスト |
| 2.5 止めない | `later`・別スレッドの前例 | **不明**: 全シェルの検査にかかる時間（UI スレッドで読むか、別スレッドへ出すか） |
| 3.1 解釈して返す | `areka_parsers::sakura::parse` | **無い**: 位置を持つ解釈の入口 |
| 3.2 知らないタグ・`\!`・`\&` | `Instruction::Raw` | **無い**: 「知っている `\!`」を返す 1 つの関数。**制約**: `\&` は全部が知らないタグになる |
| 3.3 無い surface・アニメーション・バルーン | `SurfaceResolver::resolve`・`ShellFacts::surface_exists` | **制約**: `\i` は未対応なので全部が知らないタグ。**不明**: バルーンの面の ID の在り無しを答える関数 |
| 3.4 引数が読めない | 各消費者の判定 | **無い**: 判定を純粋な関数として取り出したもの・全数の棚卸し |
| 3.5 種類・位置・綴り・文言・正本の文書 | `doc/ssp-mcp/` の文書の前例（`log-convention.md`） | **無い**: 種類の名前と文言の形（本 spec が決める） |
| 3.6・3.7 結果の形 | `outcome::value` | 文字列の中の形（決めごと） |
| 3.8 字面だけ | — | 判定から外すものの一覧（`\![raise]`・`\![change,shell]` の後・`%` の中身・名前の形の `\b`） |
| 3.11 再生と一致 | — | **無い**: 再生の側と検査の側が同じ関数を通ることを固定するテスト。`\!` は受け口の自己選別と表の一致を固定する必要がある |
| 3.12 全数の表とテスト | — | 表そのもの |
| 4.1〜4.5 一覧 | 台帳・カタログ | **無い**: バイナリの中のデータ・種類の分類 |
| 4.7 台帳と一致 | テストから `doc/` を読む前例（`tools_tests.rs` の `SAVED_LIST`） | 照合のテスト |
| 4.8 `doc/` 無しで動く | — | **無い**: 埋め込み |
| 4.10 対応表に在って `absent` | `ConsumerLedger::canonical` | **無い**: 対応表の（名前, 第 1 引数）と台帳の項目 id を結ぶ対応。**制約**: 対応表は `crates/areka` の中（`emo2_boot`）に在る |
| 5.1 起動中のゴースト | `resolve`・`ActiveGhost::root` | なし |
| 5.2 フォルダのフルパス | `is_ghost_dir` | **無い**: `resolve` の「当たらないがフォルダは在る」の枝・バルーンのフォルダの見分け |
| 5.3 全シェル | `list_all_shells` | 今のバルーンのフォルダを引く道（**不明**） |
| 5.4 読めない行・無いファイル | `MountError`・`SurfaceTable::unreadable` の形 | **無い**: `surfaces.txt`・descript の読めない行を値で返す口・参照先のファイルの解決（画像の探し方の規則は表示の層に在る＝**不明**） |
| 5.5 読まれないキー | — | **無い**: 「読むキー」の一覧か、読んだキーの記録 |
| 5.7 行の位置 | — | **無い**: `parse_kv` は行番号を残さない |
| 5.9 読み込みと同じ読み取り | — | **制約**: 読み取りは「何も報告しない」約束。手を入れると `areka-parsers` を使う全員に波及する |
| 5.10 検体で 0 件 | `sample-ghost-kit` | **不明**: いまの検体が「誤り」0 件になるか（閉じていないブレスなどが実在するかも） |
| 6.1 指示文 | `INSTRUCTIONS` | 文の追加 |
| 6.2・6.3 help | `help_html` | **無い**: 定義を help へ渡す道・日本語の 1 行の置き場（定義の `description` は英文） |
| 7.1〜7.3 テスト | `tools_socket_tests.rs` の `serve`・`post_legacy` | 13 本のテスト |
| 7.4 文書 | `doc/ssp-mcp/` | 新しい文書 1 本 |
| 7.5 実機 | 既存の実機の手順 | — |

## 4. 進め方の選択肢

### 4.1 登録口と橋

| 案 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| A: `ToolCall` に変種を足す | `ToolCall` に 3 変種、独自の表（`TABLE` と同じ形の別の配列）、2 つの表を同じ送り口へ登録する関数を足す。`dispatch` に腕を 3 本足す | 既存の形のまま。橋・時間切れ・記録・`resolved!` がそのまま使える。足し忘れはコンパイルで分かる | 後続の spec がツールを 1 本足すたびに `tools/mod.rs`（変種・表）と `mcp/mod.rs`（腕）を触る＝共有ファイルの衝突が続く |
| B: 独自の呼び出しの型を別に持つ | `ToolCall` に 1 変種（独自の呼び出しを包む）だけ足し、中身は独自の enum。振り分けも独自のモジュール（例: `crates/areka/src/mcp/` の下に独自ツール用のフォルダ）に置く | SSP の 10 本の側のファイルは最初の 1 回しか触らない。後続は独自の側の表と振り分けだけ触る（要件 1.6 が書きやすい） | 型と振り分けが 2 段になる。`resolved!` を独自の側でも使えるようにする手直しが要る |
| C: 橋を通さないツールを許す | 一覧のツールのように UI スレッドの状態が要らないものは、`areka-mcp` の中で直接答える（`ToolRegistry::register` に実装を直接渡す） | UI スレッドを 1 回も通らない＝要件 2.5 を作りで満たす | 記録（要件 2.7）と時間切れ（2.6）を自前で揃える必要がある。一覧のデータと `\!` の対応表は `crates/areka` の側に在るので、`areka-mcp` へ置くと要件 4.10 のテストがクレートをまたぐ |

A と B は橋を共有する点で同じで、違いは「後続が触るファイルをどこまで絞るか」。C は一覧のツールだけの部分的な選択肢で、A・B と組み合わせられる。

### 4.2 台帳をバイナリへ入れる方法（要件 4.8）

| 案 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| A: 元の TOML を `include_str!` | 台帳 2 本とカタログ（計 約 2.1 MB）を埋め、起動後に `toml` で読む | 生成物が無い。台帳を直せば次のビルドで反映 | `crates/areka` に `toml` の依存が増える（「依存を足す spec は 1 ウェーブに 1 本」の席を使う・`THIRD-PARTY-NOTICES.md` が変わる）。備考（開発者向けの文）がバイナリに入る。exe が約 2 MB 太る。読み取りを `ukadoc-survey` から借りるか書き直すか |
| B: 小さな表を生成してリポジトリに置く | 要る 5 欄（綴り・種類・状態・URL・別名の先）だけの表（Rust の定数か行区切りの文字列）を道具（`ukadoc-survey` のサブコマンドなど）で作り、コミットする。テストが台帳と突き合わせる（要件 4.7 のテストがそのまま「作り直し忘れ」の番人になる） | 依存が増えない。バイナリに入るのは要る欄だけ。実行時に解釈が要らない | 台帳を直した人が表も作り直す手順が増える。同じウェーブで台帳を直す spec と衝突する（roadmap の「台帳の行を直す spec と同じウェーブに置かない」に近い制約が付く） |
| C: `build.rs` で生成 | ビルドのたびに台帳から表を作る | 作り直し忘れが起きない | `crates/areka` に `build.rs` が初めて入る。`build-dependencies` に `toml`。`doc/` を変えるたびに `areka` が再ビルドになる |

### 4.3 「知らない `\!`」の決め方（要件 3.2・3.11）

| 案 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| A: `ConsumerLedger::canonical` を正本にする | 検査は表を引く。表に無い 2 つ（`set,choicetimeout`・`\![*]`）を表へ足すか、別の小さな表で補う。受け口の自己選別と表の一致は、いまも一部がテストで固定されている（`consumer_ledger.rs` の `canonical_registers_both_zorder_pairs` など） | 既存の表を使う。要件 4.10 とも同じ表 | 表は「宣言」で、受け口は表を引かない。**全部の受け口**について一致を固定するテストを足さないと、要件 3.11 の「どちらも 0 件」を言えない。内部用の運搬名（`\f` の運搬名・プロパティの書き込みの運搬名）は台本に書けないので除く必要がある |
| B: 受け口に「自分が拾うか」を答えさせる | 各受け口の選別の条件を純粋な関数に取り出し、検査はそれを全部呼ぶ | 再生と検査が同じ関数を通る＝要件 3.11 を作りで満たす | 受け口 8 つ＋seriko＋文字の層に手が入る。`emo2_boot` の列のファイルを広く触る |
| C: 網羅台帳の状態で決める | 台帳で `implemented`・`degraded`・`vocabulary-only` の `\!` を「知っている」とする | 一覧のツールと答えが揃う | 台帳は人が書いたもので、再生の実際と食い違いうる（要件 3.11 を保証できない）。台本の 1 箇所と台帳の 1 項目を結ぶ表が要る（暫定の裁定 6 が避けたもの） |

### 4.4 設定の検査の読み取り（要件 5.4・5.5・5.9）

| 案 | 中身 | 良い点 | 悪い点 |
|---|---|---|---|
| A: 読み取りに「診断つき」の入口を足す | `parse_kv`・`shell::parse`・`balloon::parse` などに、診断の列も返す入口を足し、既存の入口はそれを呼んで診断を捨てる（`SurfaceTable::unreadable` と同じ考え方）。読まれないキーは「読んだキーを覚える入れ物」で記録する | 読み込みと検査が同じ読み取り＝要件 5.9 を作りで満たす | `areka-parsers` の 4 つのモジュールと、キーを引く下流（`package`・文字の層・配置）に手が入る。`areka-parsers` を触る他の spec と衝突する。**これだけで 8〜10 タスクになりうる** |
| B: 検査の側に別の走査を持ち、テストで一致を固定する | 検査専用の行の走査を書き、「検査が誤りと言う行＝読み取りが捨てる行」を検体と壊した設定のテストで突き合わせる | 既存の読み取りに触らない | 要件 5.9 の「別々の読み取りを持たない」に字面で反する（要件を改める必要がある） |
| C: 読まれないキーは網羅台帳で決める | `doc/ukadoc-coverage/ledger/assets.toml`（542 項目）の状態が `absent`・`vocabulary-only` のキーを「読まれない」とする（4.2 の埋め込みに 1 本足す） | キーの一覧を新しく作らない。一覧のツールと同じ出どころ | 台帳に無いキー（作者の書き間違い・独自のキー）と、台帳の誤りを区別できない。要件 5.9 を保証できない |
| D: 設定の検査を別の spec へ切る | 本 spec は登録口・一覧・台本の検査まで。設定の検査は読み取りの手直しと一緒に別の spec にする | 本 spec が 20 タスクに収まる。読み取りの手直しを落ち着いて設計できる | 3 本そろうのが遅れる。指示文・help・文書を 2 回触る |

## 5. 規模とリスク

| まとまり | 規模 | リスク | 理由 |
|---|---|---|---|
| 登録口・橋・指示文・help | S〜M | 低 | 既存の形の延長。既存のテストを変えない道が見えている |
| `areka_list_capabilities` | M | 中 | 処理は単純。埋め込みの形と種類の分類が決めごと。台帳と対応表の食い違い（要件 4.10）が実際に出るかは未確認 |
| `areka_check_script` | M〜L | 中〜高 | 位置を持つ入口・`\!` の正本・引数の棚卸し・再生との一致のテスト。消費者のクレートに手が入る範囲で規模が変わる |
| `areka_validate_ghost` | L | 高 | 読み取りが「何も報告しない」約束のため、要件 5.9 を守ると `areka-parsers` と下流を広く触る。画像の探し方の規則は未調査 |
| 文書・実機 | S | 低 | 前例どおり |
| **全体** | **L（20 タスクに届く見込み・設定の検査を 4.4-A で入れると超える見込み）** | **中〜高** | 上の合計 |

## 6. 要件ディスカッションへ持っていく論点

1. **橋の形**（4.1 の A・B・C）。後続の `mcp-user-response`・`mcp-shiori-query` が触るファイルをどこまで絞るか。`entrances` と `TABLE` を変えないことは、どの案でも前提にできる。
2. **`\i` と `\&` の扱い。** いまの areka は `\i`・`\&` に対応していないので、再生では全部が「知らないタグ」になる。要件 3.3 の「シェルに無いアニメーション」・3.2 の「知らない実体参照」は、種類の名前だけ先に決めて（`mcp-strict-errors` のため）診断は「知らないタグ」で出すのか、種類を分けて出すのか。要件 3.11（再生と一致）と 3.12（各行に「出る台本」「出ない台本」）は、未対応のタグでは「出ない台本」が書けない。
3. **「知らない `\!`」の正本**（4.3 の A・B・C）。再生の経路に判定の 1 か所が無いので、本 spec がどれかを正本に決めることになる。`set,choicetimeout` と `\![*]` は対応表に無いが再生で意味を持つ。
4. **引数が読めない箇所（要件 3.4）の範囲。** 判定は意味の段・台本の組み立て・文字の層・各受け口に散らばり、黙って既定へ落とすもの（`\_w`・`\n`・`\p`）も在る。全数を拾うか、本 spec では意味の段と台本の組み立てで決まるものに絞り、残りを表に「対象外」と書いて起票するか。
5. **数値の surface ID の判定。** 再生の経路（`SurfaceResolver::resolve`）は数値の ID がシェルに在るかを見ない。検査は `ShellFacts::surface_exists` と同じ事実で「無い」と言えるが、これは再生の「知らない」より踏み込んだ判定になる（要件 3.11 はタグと `\!` だけを名指ししているので矛盾はしない）。バルーンの面の ID と、名前の形の `\b` をどこまで診るか。
6. **台本の中の位置の単位。** 字句が持っているのは UTF-8 のバイトの範囲。エージェントに返すのは、バイト・文字数・行と桁のどれか。
7. **結果の形。** `ToolOutcome` は文字列と画像だけ。診断の一覧を JSON の文字列で返すか、SSP 風の表（Markdown）で返すか。「0 件と 1 件以上を機械的に見分ける」（要件 3.6・5.8）の具体的な形。
8. **台帳の埋め込みの形**（4.2 の A・B・C）。A は依存を 1 つ足す。B は台帳を直す人の手順が増える。
9. **一覧の種類の分類。** さくらスクリプトの台帳の `%` の変数 28 件と見出しだけの 1 件、SHIORI の台帳のイベント以外の 219 件（リソース 159・プラグイン 19・仕様の節 41）を、3 種類のどれで返すか・種類を増やすか・返さないか。要件 4.1（3 種類）と 4.7（台帳と過不足なく一致）のどちらかを書き足す必要がある。
10. **対応表と台帳を結ぶ対応（要件 4.10）。** 対応表のキーは（名前, 第 1 引数）、台帳のキーは ukadoc の項目 id。カタログの `title`（例 `\![set,zorder,…]`）を字句に通して結ぶか、手書きの対応を持つか。
11. **設定の検査を本 spec に残すか**（4.4 の D）。残すなら、要件 5.9 を「読み取りに診断つきの入口を足す」（A）で守るか、要件を改めて別の走査（B）や台帳（C）で済ませるか。
12. **「読まれないキー」の定義。** 完全一致で引くキーのほかに、番号つき・前置きで拾うキー（`sakura.bindgroup*.name` など）が在る。SHIORI が自分で読むキー・SSP 専用のキーを「注意」に含めるか。
13. **参照先のファイルの範囲（要件 5.4）。** `surfaces.txt` の `element` の画像だけか、暗黙の `surface*.png`・`.pna`・バルーンの画像・SHIORI の DLL まで見るか。画像の探し方（大文字小文字・拡張子の省略）は表示の層の規則に合わせる必要がある。
14. **フォルダのフルパスの受け方（要件 5.2）。** `resolve` は「当たらなければ `CANNOT_FIND`」の純粋な関数。設定の検査だけ、解決の前にフォルダの枝を置くことになる。ゴーストの根とバルーンのフォルダの見分け方、相対パスを断るか、任意のフォルダを読めることを文書にどう書くか（受け口は `127.0.0.1` だけ・読むだけ・返すのは相対パス）。
15. **UI スレッドで読むか、別スレッドへ出すか（要件 2.5）。** 全シェルの検査はファイルを多く読む。`dump_surface` の「別スレッドで作って後から答える」の形に乗せるか。台本の検査は、シェルの事実だけ UI スレッドで取れば残りは純粋な計算。
16. **help の日本語の 1 行の置き場（要件 6.2・6.3）。** 名前は登録した定義から組めるが、定義の `description` は英文。日本語の 1 行を定義の隣に持つか、help の側に名前をキーにした表を持つか（後者は「名前を 2 か所に書かない」に触れる）。
17. **roadmap の「3 段目の約束」の書き直し（要件 1.6）。** いまは「`mcp/mod.rs`・`handler.rs` は触らない」。独自のツールを足す spec が触るファイルを、4.1 の案に合わせて書く。`INSTRUCTIONS` の `not implemented yet` の 1 文は `mcp-strict-errors` が消す約束なので、本 spec は残す（要件 6.1 と一致）。

## 7. 設計の段へ持ち越す調べもの

- バルーンの面の ID が今のバルーンに在るかを答える関数（表示の層・バルーンの資産の側）。
- 別名の表（`alias_snapshot`・`crates/areka-emo-compose/src/world.rs`）と `SurfaceResolver` を、UI スレッドの `World` からどう読むか（`dump_surface.rs` の `Emo2Wiring` の読み方が前例）。
- 引数が読めない箇所の全数（`move_cue.rs`・`zorder_cue.rs`・`state_decoration.rs`・`cursor_tag.rs`・`change_cue.rs`・`switch_cue.rs`・`update_cue.rs`・`install_cue.rs` の読み捨ての枝）。
- 画像の探し方の規則（`crates/areka-emo-atlas`・`crates/areka-emo-present/src/shell_target.rs`）と、暗黙の `surface*.png` の扱い。
- 起動中のゴーストの「今使っているバルーン」のフォルダを `World` から引く道。
- 検体 5 つを今の読み取りに通したとき、捨てられる行・読まれないキーが何件在るか（要件 5.10 の「誤り 0 件」が成り立つか、「注意」が何件出るか）。
- 対応表に在って台帳で `absent` の `\!` が実際に在るか（要件 4.10 のテストが最初から赤になるか）。
- `crates/areka/src/mcp/` と `crates/areka-mcp/src/tools/` のファイルの行数の余裕（1 ファイル 1,000 行の上限）。
- 外部の依存の調査は不要（rmcp の使い方は変えない）。4.2 の A を採るときだけ、`toml` を `crates/areka` の木へ入れる手続き（`deny.toml`・`THIRD-PARTY-NOTICES.md`）を確かめる。
