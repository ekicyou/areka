# ギャップ分析: areka-P0-mcp-expression-table

作成: 2026-10-04（ブランチ `claude/areka-p0-mcp-expression-table-440874`・起点 main `e2a373b5`）。
対象: 確定済みの `requirements.md`（Requirement 1〜7）と、今のコードの差。
方針: 事実と選択肢を並べる。どれを採るかは設計で決める。

## 1. 分析の要約

- **入口と出口は揃っている**。ツールの定義・引数の型・`ghost_name` の解決・答えの 4 つの形（`OK:` を付けない素の値を含む）は既にあり、本 spec が書くのは `crates/areka/src/mcp/get_expression_table.rs` の `handle` の中身だけで足りる。
- **無いものは 3 つ**。`surfacetable.txt` の読み手・既定の 15 件・表の組み立て（重ね合わせ・並び・Markdown の文字列）。ソース全域で `surfacetable` を読むコードは 0 件（`areka-emo-present` のテストが「面の画像ではない名前」の例として挙げるだけ）。
- **同じウェーブの約束（触らないファイル）は守れる**。`handle` は `&mut World` を受け取るので、今のシェルのフォルダは `mcp/mod.rs`・`mcp/resolve.rs` を変えずに引ける。ただし守るための条件が 4 つある（§5）。
- **要件だけでは決まらない点が残る**。行の終わりに `}` が付いた行の扱い・同じ ID が 2 度書かれたときの勝ち負け・`DisableNoDefineSurfaces` の「定義がある」の範囲と調べ方・「記録されること」をテストで何で確かめるか（§7）。
- 規模は S〜M、リスクは低〜中（§6）。

## 2. 今あるもの（要件ごとに使える資産）

### 2.1 MCP の側（アプリ本体 `crates/areka/src/mcp/`）

| 資産 | 何か | 本 spec との関係 |
|---|---|---|
| `get_expression_table.rs` の `handle(world, ghost, args, reply)` | 中身の無い処理。`NG:not implemented yet` を返す | ここへ中身を書く。引数の並びは変えない |
| `get_expression_table_tests.rs` | 空の World で `NG:not implemented yet` を期待するテスト 1 本 | 書き換える（期待が変わる） |
| `mod.rs` の `dispatch` | `get_expression_table` を `Omitted::Reject` で解決してから `handle` を呼ぶ。省略・空は `NG:Specified ghost is not active`、名前違いは `NG:Cannot find active ghost from specified name` | Requirement 6.3 はこのままで満たす。触らない |
| `mod.rs` の `drain` | 溜まった要求を 1 件ずつ順に振り分ける（UI スレッド・Input の段） | Requirement 6.4（同時に呼ばれてもそれぞれに返す）は、`handle` がその場で答えれば構造上満たす |
| `mod.rs` の `later` | その場で答えられない処理が返事を預ける口（今は呼び手なし） | 読み取りを別スレッドへ出す案を採るときだけ使う（§4 案 C） |
| `resolve.rs` の `ActiveGhost` | 起動中のゴーストの `name` と `root`（ルートフォルダ）だけを持つ値 | **今のシェルのフォルダは持っていない**（§5-1） |
| `mcp_tests.rs` | 振り分けのテスト。`get_expression_table` については解決の失敗の文言だけを見ており、`not implemented yet` は期待していない | 触らずに済む |

プロトコルの側（`crates/areka-mcp/src/`）:

- `tools/get_expression_table.rs` の `DEFINITION`・`Args { ghost_name: Option<String> }`・`parse` は完成している。触らない。
- `tools/outcome.rs` の `value`（`OK:` を付けない・`isError: false`）が Requirement 1.7 の答えの形にそのまま使える。`ng` は Requirement 5.8・3.5 により本ツールの正常系では使わない。
- `handler.rs` の `INSTRUCTIONS` は「未実装のツールは `NG:not implemented yet` で始まる結果を返す」と一般論で書いている。本ツールに中身が入っても、ほかのツールが未実装の間は文として正しいので変更は要らない。
- `tools/tools_socket_tests.rs` は偽の受け手が `NG:not implemented yet` を返すテストで、アプリ本体の `handle` を通らない。影響なし。

### 2.2 今のシェルのフォルダへの道

- `crates/areka/src/ghost_session.rs` の `GhostSlot`（World の NonSend）→ `GhostSession::runtime()` → `areka_ghost::GhostRuntime::mount()` → `MountModel.shell.dir`（`ShellMount` の `dir`・フォルダのパス）。
- `GhostSession::current_shell_folder()` はフォルダ名（末尾）だけを返す。パスが要る本 spec では `mount().shell.dir` を直に読むほうが素直（既存の読み手の例: `emo2_boot/assets.rs` がシェルを読むときの `model.shell.dir`）。
- シェルの切替の後は `GhostRuntime::set_shell_dir` が `mount().shell.dir` を書き換える（`emo2_boot/frame/switch.rs` の後始末が `GhostSession::set_shell_dir` を呼ぶ）。**呼ばれるたびに `mount().shell.dir` から読めば Requirement 6.2 を満たす**。読んだ結果を溜め込むと切替に追従する仕組みが別に要る。

### 2.3 パーサの側（`crates/areka-parsers/`）

| 資産 | 何か | 本 spec との関係 |
|---|---|---|
| `charset::decode(bytes, DefaultEncoding)` | 冒頭の ASCII の部分から `charset,名前` を探し（綴りの大小無視・UTF-8 の BOM を読み飛ばす）、無ければ渡された既定で全体を読む。`DefaultEncoding::Ansi` は Shift_JIS | Requirement 5.1・5.2 をそのまま満たす。`surfaces.txt` の読み取り（`areka-emo-present` の `load_shell_target`）と同じ使い方 |
| `shell/boxes.rs` の `parse_boxes` と `ShellBoxes` | `Shell` 型とは別に、同じ `shell/` の下で**自前の型を返す 2 つ目の読み手**。「転記だけ・検証や記録はしない・失敗しない」 | `surfacetable.rs` の置き方の前例。`shell/mod.rs` へは `mod`・テストの `mod`・`pub use` の行を足している |
| `shell/lexer.rs` の `lex` と `Token` | `surfaces.txt` の構文の切り分け（見出し行＋`{`…`}`・`//` の注釈・空行・字下げの除去・閉じていない `{` の吸収） | 形は近いが、`group,名前` の見出しや `}` が行末に付いた行の扱いが `surfaces.txt` 向け。流用するか自前で行を読むかは設計で決める（§4） |
| `shell::parse` と `Shell.surfaces`（`Surface.id`・`Surface.targets`） | `surfaces.txt` の surface*ブレスの転記。見出しが `surface0-9` のような複数指定のときは `targets` に範囲のまま残る（展開は下流） | Requirement 3.8 の「定義がある」の判定の材料（読むだけ。型に欄は足さない） |
| クレートの規律（`lib.rs` 冒頭） | `package` 以外は I/O を持たない純粋な関数 | `surfacetable.rs` はファイルを開かない。バイト列または読み終えた文字列を受け取る形になる |
| 依存 | `encoding_rs`・`tracing`。テスト用は `temp-path-kit`・`sample-ghost-kit` | **`log-capture-kit` はテスト用の依存に無い**（§5-3） |

### 2.4 「シェルに定義がある」を知る材料（Requirement 3.8）

- `areka-emo-present` の `shell_target::select_surface_images`（公開・ファイル名の一覧から「番号 → `surface*.png`」を決める純粋な関数。先頭の 0 を無視・大小無視）。
- `areka_parsers::shell::parse` の `Shell.surfaces`。範囲の展開は `areka-emo-compose` の `fold::expand_targets` がするが、これはクレートの中だけの関数で外から呼べない。
- 読み込み済みの面の番号の一覧（`areka-emo-compose` の `EmoWorld::surface_ids`）は、`emo2_boot/assets.rs` が `ShellBoxAssets.surface_ids` に写して文字の層の解決へ渡すが、**World から後で引ける置き場には残っていない**。
- areka が読む定義ファイルは `surfaces.txt` の 1 本だけ（`load_shell_target` の定数 `SURFACES_TXT`）。

### 2.5 ログとテストの道具

- 記録の水準の規約は `.kiro/steering/logging.md`（回復できる失敗・後退は `warn!`）。
- テストでログを捕まえるのは `log-capture-kit` の `capture`（`crates/areka` のテスト用の依存にある）。`tracing::subscriber::with_default` を直に呼ぶと番人のテストが赤になる。
- 実行系つきの本物の単位を起こすテストの道具: `emo2_boot::ghost_switch_test_support` の `SwitchRig`（`mcp_tests.rs` の `real_unit_answers_get_active_ghost_list_in_one_frame` が使用例）。
- `GhostSession::for_test` は実行系を持たない（`runtime()` が `None`）ので、これで作った置き場からはシェルのフォルダが引けない。

## 3. 要件と資産の対応（差の一覧）

| 要件 | 今ある資産 | 差 |
|---|---|---|
| 1.1〜1.5・1.7 表の形 | `outcome::value` | **Missing**: 見出し行・区切り行・各行を組む関数 |
| 1.6 正しい字 | `charset::decode` | 差なし（読み手が正しく読めば、答えは Rust の文字列のまま JSON になる） |
| 2.1〜2.9 シェルの行 | なし | **Missing**: `surfacetable.txt` の読み手（`group`・`scope`・`__disabled`・`__parts`・名前の省略・`group` の外の行） |
| 2.10〜2.12 | — | 読み手が `surfaces.txt` を見なければ自然に満たす |
| 3.1 既定の 15 件 | なし | **Missing**: 15 件の持ち方（埋め込みかファイルか）は設計で決める |
| 3.2〜3.6 重ね合わせ | なし | **Missing**: 「書かれている ID の集合」（載せない行の ID も含む）を読み手が返す必要がある |
| 3.7〜3.9 `DisableNoDefineSurfaces` | `shell::parse`・`select_surface_images` | **Missing**＋**Unknown**: 定義の有無の調べ方と範囲（§7-3） |
| 4.1〜4.4 並び | なし | **Missing**: （スコープ, ID）での並べ替え。同じ ID が 2 度あるときの順は未定（§7-2） |
| 5.1〜5.2 文字コード | `charset::decode` | 差なし |
| 5.3〜5.5 注釈・空行・字下げ・閉じていない `{` | `lexer::lex` に同じ寛容さの前例 | **Missing**: `surfacetable.txt` 用の実装 |
| 5.6 読めない行の記録 | `tracing` | **Missing**＋**Unknown**: 何を「読めない行」とするかの細目（§7-1）と、記録を出す場所（§5-3） |
| 5.7 開けない・文字コード不明 | `charset::decode` は知らない名前を `debug!` で記録して既定へ後退する | **Constraint**: 後退は呼び手から見えない。記録の水準を上げるなら読み手か `handle` が自分で名前を確かめる（§7-4） |
| 5.8〜5.9 | — | 読み手が失敗を返さない形にすれば満たす |
| 6.1 副作用なし | `handle` は World を読むだけで済む | 差なし |
| 6.2 切替の後 | `mount().shell.dir` | 毎回読めば差なし |
| 6.3 入口の答え | `dispatch` | 差なし（触らない） |
| 6.4 同時の呼び出し | `drain` | 差なし（その場で答える場合） |
| 7.1〜7.7 常時テスト | `log-capture-kit`・`SwitchRig`・`temp-path-kit` | **Missing**: 検体の `surfacetable.txt` と期待値の文字列。SSP の実測 14 の検体の全文は `requirements.md` に行の列挙として在るので、そこから起こす |

## 4. 実装の選択肢

どの案でも共通: 読み手は新規 `crates/areka-parsers/src/shell/surfacetable.rs`、ツールの中身は `crates/areka/src/mcp/get_expression_table.rs`。違いは「どこまでをパーサに置くか」と「定義の有無をどこから取るか」。

### 案 A: 読み手は転記だけ・組み立てはツールの側

- `surfacetable.rs`: 読み終えた文字列（またはバイト列）を受け取り、自前の型を返す。中身は「書かれた行の列（ID・名前・属する `group` の名前とスコープ・`__disabled` かどうか）」「オプションの列」「読めなかった行の列」。載せる・載せないの判断はしない（`boxes.rs` と同じ流儀）。
- `get_expression_table.rs`: 既定の 15 件（定数）・重ね合わせ・`__disabled`／`__parts` の除外・並べ替え・文字列の組み立て・ファイルの読み取り・記録。組み立ては World を取らない純粋な関数に分け、`handle` は「フォルダを引く → 読む → 組む → 答える」の薄い配線にする。
- 利点: パーサは「転記層」のまま。記録は `log-capture-kit` のある `crates/areka` の側で出るので Requirement 7.6 を既存の道具で確かめられる。Requirement 7.1 の「ツールの返す文字列」を純粋な関数のテストで押さえられる。
- 欠点: `get_expression_table.rs` が 1 ファイルで大きくなりやすい。分けたくても `mcp/mod.rs` へ `mod` の行を足せない（§5-2）。

### 案 B: 読み手が表の行まで作る・ツールは文字列にするだけ

- `surfacetable.rs` が既定の 15 件と重ね合わせ・並びまで持ち、「（スコープ, キャラクタ名, 説明, ID）の列」を返す。定義の有無は呼び手が関数か集合で渡す。
- 利点: 規則が 1 か所にまとまり、パーサのクレートだけで規則のテストが閉じる。ツールの側が小さい。
- 欠点: 既定の 15 件と重ね合わせは「`surfacetable.txt` の転記」ではなく SSP 本体の振る舞いの写しなので、純粋な転記層に意味の判断が入る。記録をパーサの中で出すと、テストで確かめる道具がそのクレートに無い（§5-3）。

### 案 C: 案 A＋読み取りを別スレッドへ出す

- `handle` はフォルダのパスだけを取り、ファイルの読み取りと組み立てを別スレッドで行い、`mod.rs` の `later` で次のフレーム以降に答える。
- 利点: UI スレッドでファイルを開かない。
- 欠点: `surfacetable.txt` は小さく、読むのは 1 本（オプションがあるときだけ `surfaces.txt` とフォルダの一覧が加わる）。スレッドと受け渡しのぶんテストが重くなる。Requirement 6.4 は満たすが、答えの順が前後しうる。

### 定義の有無（Requirement 3.8）の取り方（案 A〜C と独立に選ぶ）

- **その場で読み直す**: オプションがあるときだけ、シェルのフォルダの一覧を `select_surface_images` にかけ、`surfaces.txt` を `charset::decode` → `shell::parse` して `Surface.targets` を自前で展開する。約束のファイルの外を触らない。欠点は、範囲の展開（除外の指定を含む）を自前で持つことと、`surface.append` だけで現れる番号を定義に数えるかを決める必要があること。
- **読み込み済みの一覧を使う**: 起動・切替のときに組んだ面の番号の一覧（`EmoWorld::surface_ids` 相当）を World から引けるようにして使う。表示と同じ判定になる利点があるが、置き場を作るために `emo2_boot/` のファイル（起動と切替の両方）を触る＝約束のファイルの外へ出る。
- **調べる範囲を絞る**: 調べる必要があるのは「既定の 15 件のうち `surfacetable.txt` に書かれていない ID」だけ（最大 15 個）。全部の面を列挙しなくても、その ID について画像と surface*ブレスの有無を見れば足りる。

## 5. 同じウェーブの約束は守れるか

約束（`brief.md`「2026-10-04 ウェーブ C3-⑧」と `roadmap.md` の C3 の行）: `crates/areka/src/mcp/mod.rs`・`resolve.rs`・`crates/areka-mcp/src/**`・`Cargo.toml` を触らない。読み手は新規 `shell/surfacetable.rs`＋`shell/mod.rs` の行で、`Shell` 型（`model.rs`・`decode.rs`）に欄を足さない。

結論: **守れる**。ただし次の 4 点が条件になる。

1. **今のシェルのフォルダは `ActiveGhost` からは引けない**。`handle` が受け取る `ActiveGhost` は `name` と `root` だけ。`handle` の中で `world.get_non_send::<GhostSlot>()` → `runtime()` → `mount().shell.dir` と引けば、`mod.rs`・`resolve.rs` を触らずに済む（`GhostSlot` は `pub(crate)` で `mcp` の下から見える）。
   - 副作用: 今のテスト（空の World で呼ぶ）は置き場が無いので、新しい `handle` では「シェルのフォルダが引けない」場合に当たる。このとき何を返すかは要件に無い（§7-5）。
   - 表の中身のテストは World を通さない純粋な関数で行い、World を通す配線のテストは `SwitchRig` で実行系つきの単位を起こす形になる（`GhostSession::for_test` は実行系を持たない）。
2. **`mcp/` の下に新しいファイルを足すなら、`mod` の行は `get_expression_table.rs` の中に書く**。`mod.rs` に足せないため。今のテストのファイルが `#[path = "get_expression_table_tests.rs"]` で自分から繋いでいるのと同じ形にする。
3. **読めない行の記録をパーサの中で出すと、テストで確かめる道具が無い**。`crates/areka-parsers` のテスト用の依存に `log-capture-kit` は無く、足すには `crates/areka-parsers/Cargo.toml` を触る。約束を守るには、読み手は「読めなかった行の列」を値で返し、記録は `crates/areka` の側（`log-capture-kit` がある）で出す。Requirement 7.6 のテストもそちらに置く。
4. **`shell/mod.rs` は厳密には 1 行では済まない**。今の流儀（`boxes`）は `mod boxes;`・`#[cfg(test)] mod boxes_tests;`・`pub use boxes::{…};` の 3 か所。テストを `surfacetable.rs` の中から `#[path]` で繋げば 2 か所。どちらでも、同じウェーブで `shell/` を触る `surface-element-nesting` の持ち分は `model.rs`・`decode.rs` で、`mod.rs` は重ならない。

そのほかの確認:

- `brief.md` が挙げる `crates/areka/src/mcp/handler.rs` は**実在しない**（`handler.rs` は `crates/areka-mcp/src/` にある）。どちらにしても触らない。
- `Shell` 型に欄を足す必要は無い。Requirement 3.8 のために `Shell.surfaces` を**読む**だけ。`surface-element-nesting` が `Surface` の `id`・`targets` の名前や意味を変えなければ影響を受けない（変えるなら合流のときに追随が要る）。
- Shift_JIS の検体をテストに置くとき、`crates/areka` には `encoding_rs` の直接の依存が無い。バイト列をそのまま書くか、Shift_JIS のテストを `crates/areka-parsers` の側に置く（Requirement 7.4 は「ツールの返す文字列」とは言っていないので、読み手の単位でも満たせるかは設計で確かめる）。
- 定義の有無を「読み込み済みの一覧」から取る案（§4）だけは約束の外（`emo2_boot/`）を触る。

## 6. 規模とリスク

- **規模: S〜M**。新規の読み手 1 本とそのテスト、ツールの中身（組み立て・配線）とそのテスト。既存の型・振り分け・プロトコルは変えない。期待値の文字列を検体ごとに起こす手間が規模の大半。
- **リスク: 低〜中**。技術は既存の流儀の範囲。中に寄る理由は、SSP で実測していない点（`DisableNoDefineSurfaces`・同じ ID の重複・行末の `}`）の決め方が設計に残ることと、`surface-element-nesting` と同じ `shell/` の下で並走すること。

## 7. 設計へ持ち越す点（要件の討議で確かめたいもの）

1. **行の終わりに `}` が付いた行（`100,黒塗り}`）の扱い**。Requirement 5.6 は「読み取れない行」の例に挙げている。決めることは 3 つ: ⑴ その行を表に載せるか（載せない／`}` を外した名前で載せる）、⑵ その ID を「書かれている」に数えて既定の名前を消すか、⑶ そこで `group` を閉じたとみなすか（閉じないと、後ろの行が前の `group` のスコープとキャラクタ名を引き継ぐ）。SSP の実測は無い。
2. **同じサーフェス ID が 2 度書かれたとき**。Requirement 5.9 は「失敗させない」まで。両方載せる（実測 4 の「同じスコープの `group` が 2 つ」と同じ扱い）・先勝ち・後勝ちのどれか、両方載せるなら同じ（スコープ, ID）の中の順（書かれた順か）を決める。別のスコープに 1 度ずつ書かれた場合は両方載るのが自然だが、明文は無い。
3. **`DisableNoDefineSurfaces` の「定義がある」の範囲と調べ方**（Requirement 3.8）。⑴ `surface0-9` のような複数指定の見出し・`surface.append` だけで現れる番号を定義に数えるか、⑵ 判定をその場の読み直しで行うか・読み込み済みの一覧から取るか（後者は約束の外を触る）、⑶ `surfaces.txt` が読めないときは「画像だけで判定」でよいか。SSP では未実測なので、areka の表示が実際に出せる面と揃えるのが筋かどうかも含めて決める。
4. **「ログに記録する」の水準と、テストでの確かめ方**（Requirement 5.6・5.7・7.6）。⑴ 読めない行は 1 行ごとに記録するか・1 回の呼び出しでまとめて 1 本か（AI が下調べで何度も呼ぶので、毎回同じ記録が出る）、⑵ 水準は `warn!` か `debug!` か、⑶ `charset` の名前が分からないとき、`charset::decode` は `debug!` を出して既定へ後退するだけなので、それで Requirement 5.7 を満たすとみなすか、別に記録を足すか、⑷ Requirement 7.6 の「記録されること」を、ログの捕捉で確かめるか・読み手の返す「読めなかった行の列」で確かめるか。
5. **シェルのフォルダが引けないとき（置き場が空・実行系が無い）の答え**。入口の解決を通った後なので本番ではほぼ起きないが、切替の途中の 1 フレームやテストでは起きる。既定の 15 件だけを返す（Requirement 3.5 に寄せる）か、`NG:` を返すか。今のテスト `answers_not_implemented_yet_with_an_empty_world` の書き換え先がこれで決まる。
6. **組み立てをどちらのクレートに置くか**（§4 の案 A／案 B）と、既定の 15 件の置き場（`get_expression_table.rs` の定数か・`surfacetable.rs` の定数か）。
7. **UI スレッドでファイルを読むか**（§4 の案 C）。その場で読めば単純で Requirement 6.4 も構造で満たす。別スレッドへ出すなら `later` の最初の呼び手になる。
8. **読み手が受け取るもの**。バイト列（読み手の中で `charset::decode` を呼ぶ）か、読み終えた文字列（呼び手が `charset::decode` を呼ぶ・`shell::parse` や `parse_boxes` と同じ形）か。前者は Shift_JIS・BOM のテストを読み手だけで閉じられる。
9. **細目**: ID の書き方（先頭の 0・負の数・`u32` に収まらない数）・`scope` が数値でないとき・`group` の見出しの後に `{` が来ないとき・`group` の入れ子・`option` の綴りの大小と前後の空白・`{`／`}` と同じ行に別の字があるとき。どれも「読めた分で返す」の範囲だが、期待値の文字列を起こす前に決めておく。

### 要件の討議の結果（2026-10-04・SSP の追加実測 2 体の後）

- 番号のずれ: この討議で Requirement 5 に基準 6 を足したため、本書のこれより上の「Requirement 5.6〜5.9」は今の 5.7〜5.10 を指す。Requirement 3.9 は無くなった。
- **1 は解決**: 行末の `}` は名前の一部としてそのまま載る。`group` を閉じるのは `}` だけの行（Requirement 5.6）。
- **3 は消えた**: `option,DisableNoDefineSurfaces` は表を変えない（Requirement 3.8）。定義の有無を調べる必要が無くなり、§2.4 と §4「定義の有無の取り方」は不要。`shell::parse`・`surfaces.txt`・フォルダの一覧は読まない。
- **2・4・5 は設計で決める**。机の 13 本に同じ ID の重複は 0 件。推す案: 2＝両方載せて書かれた順／4＝`.kiro/steering/logging.md` の規約どおり（回復できる後退は `warn!`）・確かめ方は読み手の返す値／5＝既定の 15 件だけを返す（Requirement 3.5・5.9 の「`NG:` にしない」に寄せる）。

## 8. 調べ残し（設計で確かめる）

- `SwitchRig` が起こすゴーストのシェルのフォルダに、テストから `surfacetable.txt` を置けるか（Requirement 6.2 の切替の後のテストを本物の単位で書けるか）。置けなければ、切替の後のテストは「`mount().shell.dir` を毎回読む」ことを別の形で押さえる。
- `lexer::lex` を `surfacetable.txt` に流用したとき、`group,名前` の見出し・`group` の外の行・行末の `}` がどのトークンになるか。流用が実測 8・9 の振る舞いと合わなければ、行を自前で読む。
- 実物の 13 本（`requirements.md`「実物の `surfacetable.txt` に見られた書き方」）を読み手に通したとき、読めない行として記録される行がどれだけ出るか（記録の量の見積もり・上の 4 の材料）。
- `surface-element-nesting` の設計が `Surface.id`・`Surface.targets` を変えるかどうか（変えるなら Requirement 3.8 の判定の材料が動く）。

## 9. 次の一歩

要件の討議で §7 の 1〜5（要件の文言に関わるもの）を確かめ、6〜9 は設計で決める。その後 `/kiro-design areka-P0-mcp-expression-table`。

## 10. 設計の調べと決め（2026-10-04・design.md の生成）

### 要約

- **調べの種類**: 軽い調べ（既存の仕組みへの追加）。新しい依存は無く、外部の調べはしていない。
- **分かったこと**:
  - 今のシェルのフォルダは `GhostSlot` → `GhostSession::runtime()` → `GhostRuntime::mount()` → `MountModel.shell.dir`（`ShellMount` の `dir`・`crates/areka-parsers/src/package/model.rs`）で、`mcp/mod.rs`・`resolve.rs` を触らずに `handle` の中から引ける。
  - `charset::decode` は知らない `charset` の名前を `debug!` で記録して既定へ戻るだけで、呼び手からは見えない。名前の判定は `encoding_rs::Encoding::for_label`。
  - `SwitchRig` は見本のゴーストを一時の根へ写して起こすので、テストは `mount().shell.dir` に `surfacetable.txt` を書ける。切替の後は `GhostSession::set_shell_dir` で別のフォルダへ替えれば、本物の切替を回さずに「毎回 `mount().shell.dir` から読む」ことを踏める（§8 の 1 つ目の答え）。
  - 実測 14 の検体の全文（0〜9 と 2100 番台の名前）と SSP の返した全文はリポジトリに無い。設計の時点で机の SSP は応答しなかった（読むだけの問い合わせが `Server not available`）。

### 決めたこと

| 論点（§7 の番号） | 決め | 理由 |
|---|---|---|
| 2 同じ ID の重複 | 両方載せる・同じ（スコープ, ID）は書かれた順（安定な並べ替え） | 実測 4（同じスコープの `group` が 2 つ）と同じ扱いで、捨てる規則を作らずに済む |
| 4 記録 | 水準は `warn!`。読めない行は 1 回の呼び出しで 1 本にまとめる（件数と行番号）。読み手は読めない行を値で返し、記録は `crates/areka` の側 | `logging.md` の「回復できる後退は `warn!`」。AI は何度も呼ぶので行ごとには出さない。`areka-parsers` に `log-capture-kit` が無い |
| 4-⑶ 知らない `charset` の名前 | 読み手が `charset,名前` の行を `for_label` で確かめ、知らなければ読めない行に入れる | `charset::decode` を変えずに、同じ 1 本の `warn!` で Requirement 5.8 を満たす |
| 5 シェルのフォルダが引けない | `warn!` して既定の 15 件だけ | Requirement 3.5・5.9 の「`NG:` にしない」に寄せる |
| 6 どちらのクレートに置くか | 案 A（読み手は転記だけ・組み立てと既定の 15 件はツールの側） | 既定の 15 件は SSP 本体の振る舞いの写しで、転記ではない |
| 7 UI スレッドで読むか | その場で読む（案 C は採らない） | 小さい 1 本。Requirement 6.4 は `drain` の構造で満たす |
| 8 読み手が受け取るもの | 読み終えた文字列（`parse_boxes` と同じ形）。`charset::decode` は `load` が呼ぶ | 読み手を純粋に保つ。Shift_JIS のテストは `crates/areka` の側でバイト列を直に書く（`encoding_rs` の直接の依存は要らない） |
| 9 細目 | design.md「行の読み分け」の表が正本 | — |
| `lexer::lex` の流用（§8 の 2 つ目） | 使わない・行を自前で読む | 行末の `}` を名前に残す扱いと `group` の外の行が `surfaces.txt` 向けの切り分けと合う保証が無く、要る行の種類は数個 |
| 既定の 15 件の持ち方 | `get_expression_table.rs` の定数 | 差し替え・言語の切替は範囲の外 |

### まとめ直し（一般化・既存の利用・削ったもの）

- 一般化: 「書かれている ID」の判定（Requirement 3.4）と「載せない行」（2.8・2.9）は、読み手が全行を転記すれば `render` の 1 か所で済む。読み手に `disabled` の旗などは足さない（グループ名と名前の比較で足りる）。
- 既存の利用: `charset::decode`・`outcome::value`・`boxes.rs` の置き方・`SwitchRig`・`log-capture-kit`。
- 削ったもの: 別スレッドの読み取りと `later`・読んだ結果の溜め置き・`surfaces.txt` とフォルダの一覧の読み取り・既定の名前のファイル・`Shell` 型への欄。

### 残るリスク

- 実測していない細目（`scope` の位置・`{` の無い `group`・`group` の入れ子・名前の中の `|`）は「読めた分で返す」決めで、SSP と違うと分かったら「行の読み分け」の表の該当行を直す。
- 7.1 の期待値は、最初のタスクで開発者の机の検体と SSP の出力から写す（上の「分かったこと」の 4 つ目）。
- §8 の 3 つ目（実物の 13 本で読めない行がどれだけ出るか）は未確認。記録は 1 回の呼び出しで 1 本なので量は増えない。
