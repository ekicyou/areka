# ギャップ分析: areka-P0-open-external-tags

> 2026-10-05 `/kiro-validate-gap`（要件確定後・設計前）。要件 `requirements.md` と今のコードの差を、ファイルと行で示す。決めるのは設計と要件ディスカッションで、ここでは選択肢と根拠だけを置く。行番号は本日のワークツリー（`d89979ad`）のもの。

## 1. 要点（5 行）

- **読み込みの入口は 1 腕で足りる**: `\j[ID]` は字句解析で `Tag{word:"j", args:[ID]}` になり、`decode_tag` の既定の腕（`crates/areka-parsers/src/sakura/decode.rs:272`）で `Raw` に落ちて `compile.rs:237` の catch-all が捨てている。`\+` → `\![change,ghost,random]` の転記（`decode.rs:203`）と同じ手で `GenericCommand` へ写せば、`compile.rs`・dola の `CueCommand` には **触らずに済む**（汎用コマンドの腕 `compile.rs:181-187` は時間 0・offset 不変＝要件 1.6 もそのまま満たす）。
- **受け取り手は既存の `ReadmeCueSink` を広げれば `emo2_boot/mod.rs` に触らない**: `mod.rs` が固定しているのは型名 `crate::readme::ReadmeRequest`（`mod.rs:491`）・`ReadmeCueSink::new(Sender<ReadmeRequest>)`（`:492`）・`resolve_path(&Path, Option<&str>) -> PathBuf`／`wire_readme(&mut World, PathBuf, Receiver<ReadmeRequest>)`（`:717-719`）の 4 つの綴りだけ。`ReadmeRequest` を中身つきの型にし、`readme_cue.rs:84` の自己選別を `open` の 6 つの第 1 引数と `\j` の運搬名まで広げる形が成り立つ。
- **開く処理の 1 か所は今 `readme.rs:165-198` の `open()` だけで、World を借りた UI スレッドの中で `ShellExecuteW` を同期に呼んでいる**（`popup-menu-residue` 残件 10）。要件 7.5（固まらない）と 7.8（台本の順）を同時に満たす最短は「開く専用の 1 本のスレッド＋mpsc」。
- **OS の呼び出しは今の `windows` クレートの機能だけで賄える**（`Cargo.toml` 非接触の約束）: 環境変数の展開は `std::env::var`、ファイルを選んだ状態の窓は `explorer.exe /select,…`、実行ファイルのパス探索は `ShellExecuteW` 自身が行う。`ExpandEnvironmentStringsW`・`SHOpenFolderAndSelectItems` は **未有効の機能**（`Win32_System_Environment`・`Win32_UI_Shell_Common`）を要するので使わない。
- **`get_log` に成功の記録を残すには取り決めの表を 1 行動かす必要がある**（`log_history.rs:113-127` の `RULES` に `areka::readme` 系の target は無い）。失敗は `areka::log::error` へ出せばゴースト名つきで `error` 種別に残る。表を動かすなら `doc/ssp-mcp/log-convention.md:40-58`・`RULES`・`log_history_convention_tests.rs` の 3 か所が同時に動く。

## 2. 今のコード（現状の調査）

### 2.1 台本の読み込み（`\j`）

| 場所 | 今のふるまい |
|---|---|
| `crates/areka-parsers/src/sakura/lexer.rs:198-255` | `\` の後ろを 1〜3 文字読み、`[` が続けば `Token::Tag{word, args}`。`\j[http://…]` は `word="j"`・`args=["http://…"]`。引数は `,` 区切り・`"…"` で `,` を守る・`\]` はリテラル（`:306-386`） |
| `decode.rs:215-273 decode_tag` | `"j"` の腕が無く `_ => decode_passthrough_tag` → `Instruction::Raw("\\j[…]")`（`:356-358`）。ファイル冒頭の注記（`:206-207`・`:271`）が `\j` を「パススルー領分」と明記 |
| `decode.rs:203-205` | **転記の先例**: 裸の `\+` を `decode_passthrough_bang(["change","ghost","random"])` で `GenericCommand{name:"change", raw_args:["ghost","random"]}` へ写す（誰が消費するかは決めない） |
| `decode.rs:270` ＋ `compile.rs:223-230` | もう 1 つの先例: `\f` は `Instruction::Font` → 運搬名 `FONT_TAG_CARRIER`（`"\\f"`・`areka-sakura/src/contract.rs`）のキャリアへ。こちらは `compile.rs` に 1 腕要った |
| `compile.rs:181-202` | `GenericCommand` は `command_carrier(name, raw_args)` の cue を時間 0 で出し、offset を進めない。`:237-239` の catch-all が `Raw` を `debug!` で捨てる |
| `compile_arm_tests.rs:134` | `catch_all_ignored_set_is_raw_only`＝除外集合が `Raw` だけであることの檻。`GenericCommand` は卒業済みなので `\j` を `GenericCommand` にしても檻は動かない |
| テスト | `\j` を `Raw` として固定しているテストは **0 本**（`crates` 全域で `\\j` の綴りはコメント以外に無い） |

### 2.2 受け取り手と結線

| 場所 | 今のふるまい |
|---|---|
| `crates/areka/src/emo2_boot/consumer_ledger.rs:310-369` | 正準台帳 15 行。`("open","readme")` → `ReadmeSink` の 1 行だけが `open` の登記（`:324-326`）。enum `CommandConsumer`（`:100-160`）の `ReadmeSink` の doc（`:110-115`）が ukadoc の URL を持つ |
| 同 `:665-675` | 件数 15 を逐語で固定（増減で本檻と doc の 2 か所を編集せよと書いてある） |
| 同 `:745-762` | `("open","browser")` が **担当なし**であることを固定（本 spec が書き替える対象） |
| `emo2_boot/readme_cue.rs:60-107` | `ReadmeCueSink::emit`: ⑴ キャリアを開封 ⑵ `(name, 第 1 引数) == ("open","readme")` だけ受理（`:83-90`） ⑶ 引数付き `\![open,readme,種類,名前]` は `warn!` して捨てる（`:95-101`） ⑷ `ReadmeRequest`（中身なし）を送る（`:104`）。モジュール doc `:17-20` が「`\![open,他]` に別の担当が付いたら宛名の述語を見直せ」と申し送り済み |
| `emo2_boot/mod.rs:491-492` | `channel::<crate::readme::ReadmeRequest>()`・`ReadmeCueSink::new(readme_tx)`。受け口は boot より前に組む（ゴースト名はまだ無い） |
| `emo2_boot/mod.rs:717-719` | `resolve_path(&ghost_root, mount().readme)` → `wire_readme(world, readme_path, readme_rx)`。**`ghost_root` は `readme.rs` に渡らない**（説明書のパスだけ渡る） |
| `ghost_session.rs:61` | `readme::register_readme_drain(world)` を 1 回登録（`Input` の段・`dispatch_pointer_events` の後） |
| `menu/mod.rs:312-314` | メニューの「説明書」も `readme::open_from_world(world)` を呼ぶ（要件 7.1 の「既存の readme も同じ 1 か所」の 2 つ目の入口） |

### 2.3 開く処理（OS の境界）

| 場所 | 今のふるまい |
|---|---|
| `crates/areka/src/readme.rs:165-198 open()` | `ShellExecuteW(None, "open", path, None, None, SW_SHOWNORMAL)`。戻り値 ≤32 を失敗として `error!(readme_open_failed, path, code)`、成功は `info!(readme_opened, path)`。`unsafe` はこの関数だけ |
| `readme.rs:119-136 open_from_world(&World)` | `ReadmeWiring` を借りたまま `open()` を呼ぶ。実在しないときは `warn!` で OS を呼ばない |
| `readme.rs:154-158 drain_readme_requests(&mut World)` | `Input` の system。溜まった件数ぶん `open_from_world` を呼ぶ＝**UI スレッドで World を借りたまま OS を待つ**（`popup-menu-residue` brief 項目 10「設計で受容済み・実機未観測」） |
| `readme_tests.rs:243-257` | 実在しないファイルで**本物の `ShellExecuteW`** を呼び、符号 2 で `Err` を確かめている（アプリは起きないが OS の境界を踏んでいる＝要件 10.1 の精神とはずれる） |
| 他の `ShellExecute*` | `crates` 全域で `readme.rs` 以外に **0 か所** |

### 2.4 記録と `get_log`

| 場所 | 今のふるまい |
|---|---|
| `crates/areka/src/log_history.rs:80-82` | 取り決めの target `areka::log::script`／`areka::log::error`（この 2 つだけ `ghost`・`label` の欄を名と表示の語に読む） |
| 同 `:113-127 RULES`・`:134-145 classify` | info を振り分ける 13 行。`areka`（完全一致）・`areka::boot_config`・`areka::boot_resolve`・`areka::ghost_session`・`areka::emo2_boot::ghost_switch`・`ghost-boot`・`ghost-shutdown` が `status`。**`areka::readme` に当たる行は無い**＝今の `readme_opened` は履歴に残らない（要件の未決 3 の記述どおり） |
| `doc/ssp-mcp/log-convention.md:38` | 表を変えるときは `RULES`・`TARGET_*` も同時に変えよ。表は `log_history_convention_tests.rs` が読む（`:40-58` の `log-rules:begin/end` の間） |
| 同 `:65-75` | `ghost` 欄は `get_active_ghost_list` の値と同じもの（`crates/areka/src/mcp/resolve.rs:67-78 listed_value`＝descript の `name`、無ければ根のフルパス） |
| 本番で `areka::log::script` へ出している行 | **0 か所**（`crates` 全域で一致するのは `log_history*.rs` だけ）。種別の正本はテスト側にしか無い |

### 2.5 ゴースト名・`ghost/master`・目録の引き方（名前解決の素材）

| 場所 | 使えるもの |
|---|---|
| `mcp/resolve.rs:34-44 active(&World)` | `GhostSlot`（`ghost_session.rs:390`）から `ActiveGhost{name: descript の name, root: ghost/<フォルダ> の絶対パス}` を読む。**mod.rs に触らず**ゴースト名と根を得る口 |
| `areka-ghost/src/catalog.rs:261-263` | `ghost/master` の組み立て＝`ghost_dir.join("ghost").join("master")`。同じ組み立てで `ghost/master` 基準の相対パスが解ける |
| `boot_config.rs:246-255 BootContext`（Resource） | `root: BasewareRoot`（`<根>/ghost`・`<根>/balloon`）・`current`。`ghost_switch.rs:339-347` が `world.get_resource::<BootContext>()` → `list_ghosts(&ctx.root)` で目録を読む先例 |
| `emo2_boot/ghost_switch.rs:170-192 resolve_switch_target` | 純粋: `GhostEntry` の列と名前から 1 体（descript の `name` を先に、無ければフォルダ名・大文字小文字を区別）。`\![change,ghost,名前]` と同じ引き方＝要件 4.4 の「同じ」をそのまま再利用できる |
| `emo2_boot/shell_balloon_resolve.rs:109-120 balloon_candidates`・`:158-185 resolve_skin_target`・同ファイルのシェルの候補（`list_all_shells` から・`:92` 付近） | 純粋: バルーン・シェルの名前解決。`SkinSpec::Name` で name → フォルダ名の順 |

### 2.6 偽の境界とスレッドの先例

| 場所 | 先例 |
|---|---|
| `install/procedure.rs:42 trait InstallPorts` | 「本番は背景のスレッド・テストは偽物」の口。失敗の返し方も enum |
| `areka-update/src/fetch.rs:8 trait Fetch` | 取得口の trait。本物は `winhttp.rs`、常時テストは偽の取得口（`structure.md:350`） |
| `install/fetch_url.rs:167`・`mcp/dump_surface.rs:84`・`emo2_boot/switch_assets.rs:141` | `std::thread::Builder::new()` で名前付きの作業スレッドを起こす先例（`thread_roles.rs` の役割名は `spawn_actor` 経由のアクターだけ） |
| `areka-sakura/src/drive.rs:3` | `ReadmeCueSink::emit` が走るのは talk ごとの `sakura-talk-{id}` スレッド＝ここで OS を待つと台本の再生が止まる（要件 7.5 の「台本の再生を止めない」に反する） |
| `readme_tests.rs:12-33`・`readme_cue_tests.rs:34-44` | `log_capture_kit::capture_lines` で**呼んだスレッドの**記録を拾う（別スレッドの記録は拾えない＝メモリ「`log_capture_kit` は呼んだスレッドだけ」） |

### 2.7 ファイル配置の先例

- `emo2_boot/frame.rs` が `mod attach;` … と宣言し、実体は `emo2_boot/frame/*.rs`（`frame.rs:42-54`）。`readme.rs` の子＝`crates/areka/src/readme/<name>.rs` を `readme.rs` から `mod <name>;` で繋ぐ形がそのまま使える（`structure.md:149-184` の兄弟テストファイルの規則も同じ）。
- `readme.rs` は 202 行。開く処理・行き先の分類・偽の境界を子へ出せば 1,000 行の目安は遠い。

### 2.8 網羅台帳の 6 行

`doc/ukadoc-coverage/ledger/sakura-script.toml` の該当行（いずれも `status = "absent"`・`owner = ""`・優先度 E1）:

| タグ | 行 | 今の note の壊れ方 |
|---|---|---|
| `\j[ID]` | `:5173-5185` | decode の既定の腕 → Raw → catch-all が捨てる（debug! のみ） |
| `\![open,browser,…]` | `:1939-1952` | キャリアで最後まで運ばれるが登記が無い（受け口の debug! のみ） |
| `\![open,editor,…]` | `:2129-2142` | 同上 |
| `\![open,explorer,…]` | `:2160-2173` | 同上 |
| `\![open,file,…]` | `:2175-2188` | 同上 |
| `\![open,mailer,…]` | `:2269-2282` | 同上 |

`implemented` にするなら実装側の定義箇所に `/// ukadoc: <URL>`（`doc/ukadoc-coverage/README.md:346-394`。URL 1 語だけ・コードの尻尾の `//` は拾われない）。書き方の手本は `("open","readme")` の行 `:2398-2408`（実装済み・縮退・壊れ方・ログ・根拠の場所）。ukadoc の URL（MCP で確認済み）:

- `\j[ID]`: `https://ssp.shillest.net/ukadoc/manual/list_sakura_script.html#_j_ID_`（原文: 「IDにジャンプする。http://～でURLジャンプする。file:///～でOSに関連付けられた拡張子でファイルを実行する。…相対パスが使用可能。※SSPのみ mailto:～」）
- `\![open,file,ファイル名]`: `…#_!_open%2Cfile%2C%E3%83%95%E3%82%A1%E3%82%A4%E3%83%AB%E5%90%8D_`（「windowsの環境変数(%SYSTEMROOT%,%SYSTEM%,%TEMP%等)を使う事ができる。ファイル名に実行ファイルを指定した場合、自動でパス探索する」）
- `\![open,browser,パラメータ]`: `…#_!_open%2Cbrowser%2C%E3%83%91%E3%83%A9%E3%83%A1%E3%83%BC%E3%82%BF_`
- `\![open,explorer,ファイル]`: `…#_!_open%2Cexplorer%2C%E3%83%91%E3%83%A9%E3%83%A1%E3%83%BC%E3%82%BF_`（「種類(ghost/shell/balloon/headline/plugin),名前」の形・例 `\![open,explorer,ghost,Emily/Phase4]`）
- `\![open,editor,ファイル,表示行]`: `…#_!_open%2Ceditor%2C%E3%83%91%E3%83%A9%E3%83%A1%E3%83%BC%E3%82%BF_`
- `\![open,mailer,パラメータ]`: `…#_!_open%2Cmailer%2C%E3%83%91%E3%83%A9%E3%83%A1%E3%83%BC%E3%82%BF_`

## 3. 要件と資産の対応（ギャップの印: 無い／不明／制約）

| 要件 | 既にあるもの | 足りないもの・制約 |
|---|---|---|
| 1 `\j` を読む | 字句解析は済み。転記の先例 `decode.rs:203` | **無い**: `decode_tag` の `"j"` 腕。**制約**: 腕は 1 本だけ（`anchor-tag-canon` が次に触る） |
| 1.5 `\!` と同じ受け取り手 | `GenericCommand` → `command_carrier`（`compile.rs:181`） | **決める**: 運搬名の綴り（§4.1） |
| 1.7 3 形以外は `warn!` | — | **無い**。かつ「ID にジャンプ」の意味が不明（§6.1） |
| 2 `\![open,file]` | `ShellExecuteW` の呼び出し `readme.rs:170-179` | **無い**: `ghost/master` 基準の解決・環境変数の展開・名前だけの実行ファイル。**制約**: `ExpandEnvironmentStringsW` の機能 `Win32_System_Environment` は未有効（`Cargo.toml:57-89`）→ `std::env::var` で `%NAME%` を置き換える（Windows の環境変数の読みは大文字小文字を区別しない） |
| 2.4 パス探索 | — | `ShellExecuteW` はパス区切りの無い名前を自分で PATH・App Paths から探す（OS の既定の探索）。areka 側は「`ghost/master` の下に無ければそのまま渡す」だけで足りる見込み（**不明**: 実機で `TortoiseProc.exe` 相当の確認が要る） |
| 3 `\![open,browser]` | 同上 | **無い**: 受理と登記。URL でない引数を渡されたときの扱いは設計で決める（そのまま OS へ渡して失敗は記録、が最短） |
| 4.1-4.3 `\![open,explorer,ファイル]` | — | **無い**: フォルダなら `open`、ファイルなら「選んだ状態」。**制約**: `SHOpenFolderAndSelectItems` は `Win32_UI_Shell_Common`（`ITEMIDLIST`）が要る＝未有効 → `ShellExecuteW("open", "explorer.exe", "/select,\"<path>\"")` で代替（機能追加 0） |
| 4.4-4.6 `種類,名前` | 純粋な解決関数が 3 つ揃っている（§2.5） | **無い**: 結線（`BootContext` の root・今のゴーストの dir を読んで候補を作り、純粋関数へ渡す）。規模は小さい（§6.2） |
| 4.7 headline/plugin | — | 仕組みが無い。`warn!` 1 行で縮退（台帳 `degraded`） |
| 5 `\![open,editor]` | — | `ShellExecuteW` の動詞 `edit`。関連付けが無いと `SE_ERR_NOASSOC`（31）→ `error!`。表示行は読まない |
| 6 `\![open,mailer]` | — | `mailto:` を付けて `open`。既に `mailto:` なら付けない |
| 7.1 1 か所 | `readme.rs:165 open()` が唯一の呼び出し | **変える**: 動詞・引数を受ける形に広げ、メニュー（`menu/mod.rs:313`）と台本の両方を通す |
| 7.2 記録の欄 | `readme_opened`（path のみ） | **無い**: 種類・解決した行き先・ゴースト名・元の綴り。ゴースト名は `mcp::resolve::active(world)` から（mod.rs 非接触） |
| 7.3 `get_log` | `RULES`・取り決めの表 | **決める**: 表に行を足すか（§6.3） |
| 7.4 失敗は `error` 種別 | warn 以上は無条件に `error` 種別（`log-convention.md:27`） | 既に満たす。`target: "areka::log::error"` + `ghost = %name` にすればゴースト名が「名」に出る |
| 7.5 固まらない | — | **無い**: 今は UI スレッドで同期（§2.3）。開く専用スレッドが要る（§4.3） |
| 7.8 台本の順 | mpsc は順序を保つ | 開く側が 1 本のスレッドなら順序は保たれる。要求ごとにスレッドを起こす案は順序を保証しない |
| 8 行き先を取り出す純粋な関数 | `areka_parsers::sakura::parse`（公開・`sakura/mod.rs:38`） | **無い**。`parse` を通して `GenericCommand` だけ拾えば、字句（`"…"`・`\]`）の扱いが開く処理と一致する。**不明**: 「元の綴り」を字句どおりに返すには lexer の範囲情報が要る（`lexer.rs:125-170` の `emit(tok, 範囲)` は内部に範囲を持つが、公開面 `parse` は返さない）。`reconstruct_tag` 相当で組み直した綴り（正規化後）で足りるかは設計で決める |
| 8.6 2 つの規則が食い違わない | — | 受け口と取り出し関数が**同じ分類関数**（`(name, args) -> Option<行き先>`）を呼ぶ構造にすれば、テストは「同じ関数を使っている」ことと分類の表だけを固定すればよい |
| 9.1 台帳 6 行 | §2.8 | 書き替え＋`/// ukadoc:` を定義箇所へ |
| 9.2-9.3 受け取り手の表 | §2.2 | 15 → 20 または 21 行（`\j` の運搬名を登記するなら 21）。`:665-675`・`:745-762` を書き替え、`("open","help")` が担当なしの檻を足す |
| 9.4 §8 の登記 | `doc/COMPAT_ARCHITECTURE.md:122-134` の表 | 行を 4 つ足す（設定を写さない・表示行・headline/plugin・`\j` の旧来のジャンプ） |
| 10 決定論テスト | `log_capture_kit`・`temp_path_kit`・偽の口の先例 | **無い**: OS の口の trait と記録する偽物。**制約**: 記録を検査するテストは OS の口を呼ぶ関数を**同じスレッドで**呼ぶ形にする（別スレッドの記録は拾えない）。`readme_tests.rs:243-257` の本物の `ShellExecuteW` を偽物へ寄せる |

## 4. 実装の選択肢

### 4.1 `\j` の写し方（decode.rs の 1 腕）

| 案 | 中身 | 利点 | 難点 |
|---|---|---|---|
| **ア: 運搬名＝タグの綴り** | `"j" => GenericCommand{name: <運搬名>, raw_args: [ID]}`。運搬名は `\f` の `FONT_TAG_CARRIER`（`"\\f"`）と同じ流儀で `"\\j"` などの定数 | `compile.rs` 無改変・`CueCommand` 無改変・元の ID をそのまま運ぶ・既存の `\!` 名と衝突しない | 定数の置き場: `decode.rs`（areka-parsers）から見える所に要る。`FONT_TAG_CARRIER` は areka-sakura の `contract.rs` にあるが parsers からは見えない（依存は parsers ← sakura）→ parsers 側（`sakura/model.rs` など）に置いて sakura の `contract` が再公開する形になる |
| イ: `\![open,j,ID]` 風に写す | `GenericCommand{name:"open", raw_args:["j", ID]}` | 受け口の登記が `open` の第 1 引数だけで揃う | 正典に無い語彙を作る（`\![open,j]` は無い）・台帳と `\![open,…]` の説明が濁る |
| ウ: decode で http/file/mailto に分類して `open,browser|file|mailer` へ写す | — | 受け口が `\j` を知らずに済む | parser の転記原則（意味づけは消費側）に反する・要件 1.7 の `warn!`（元の ID と理由）と 8.2 の「元の綴り」が decode で失われる |

推し: **ア**。腕は `decode_tag` の `"f"` の次・`_` の前に 1 本（`anchor-tag-canon` の `"_a"` も同じ並びに足される）。

### 4.2 受け取り手の広げ方（mod.rs 非接触の確認）

mod.rs が綴る 4 点（§1）を変えずに済む最小形:

1. `readme.rs` の `ReadmeRequest` を「開く要求」の型にする（例: `enum ReadmeRequest { Readme, Open(OpenRequest) }`、または名前を保つ `type ReadmeRequest = OpenRequest;`）。`mod.rs:491` は型名だけを綴っているので変わらない。
2. `readme_cue.rs:84` の自己選別を `("open", readme|file|browser|explorer|editor|mailer)` と `(運搬名 "\\j", _)` へ広げる。開封できない荷物の宛名の述語（`:65`）もモジュール doc `:17-20` の申し送りどおり見直す。`\![open,help]` など範囲外は従来どおり担当外（`debug!`）。
3. `consumer_ledger.rs` の `canonical()` に行を足す。enum の変種は **既存の `ReadmeSink` を広げる**（doc と名前を「開く系の受け口」へ改める。名前を `OpenSink` に変えるなら `consumer_ledger.rs` の中だけで済む）か、**別の変種を足す**か——実行時に自己選別するのは同じ 1 つの sink なので「1 出現に高々 1 担当」の表としては 1 変種が真実に近い。
4. UI 側の取り出し `drain_readme_requests`（`readme.rs:154`）はそのまま `Input` の段に居るので登録の変更は不要（`ghost_session.rs:61`）。要求の中身で分岐して開く処理へ渡す。

→ **`mod.rs` に触らずに広げられる**（止めて報告する事由は見つからない）。

### 4.3 OS を呼ぶ場所とスレッド（要件 7.5・7.8）

| 案 | 中身 | 利点 | 難点 |
|---|---|---|---|
| **A: 開く専用の 1 本のスレッド** | 取り出し（UI）は要求の解決と記録だけを行い、`(動詞, 対象, 引数)` を mpsc で専用スレッドへ渡す。先例は `install/fetch_url.rs:167` の `thread::Builder` | UI も talk も止まらない・1 本なので台本の順が保たれる（7.8）・COM の初期化を 1 回で済ませられる | スレッドの寿命（アプリ終了時の片付け）と、失敗の記録がそのスレッドで出る＝テストで拾うには「解決と記録」を OS の口の手前に寄せる設計が要る |
| B: 要求ごとに `std::thread::spawn` | 最短のコード | 1 行で済む | 複数の要求の順が保証されない（7.8 に反する）・スレッドの数が台本次第 |
| C: talk スレッドで直接呼ぶ | `ReadmeCueSink::emit` の中で OS を呼ぶ | 結線が最小 | 台本の再生が OS の待ちで止まる（7.5 に反する）・World（ゴースト名・root）が読めない |
| D: 今のまま UI で同期 | 変更なし | 変更なし | 要件 7.5 の文言「窓が固まらない」に反する（`popup-menu-residue` が受容していたのは説明書 1 件だけの頃） |

推し: **A**。失敗・成功の記録をどちらのスレッドで出すかは設計の決めごと（記録を UI 側で出すなら、専用スレッドから結果を返す往復が要る。専用スレッドで出すなら、テストは「OS の口を呼ぶ関数」を同じスレッドで偽物と一緒に呼べば拾える）。

**要調査（設計で）**: `ShellExecuteW` の公式の説明は「使う前に COM を初期化しておくのがよい（`CoInitializeEx` の STA）」と書く。専用スレッドなら起動時に 1 回で済む。`Win32_System_Com` は有効（`Cargo.toml:75`）。

### 4.4 偽の境界（要件 10）

`trait`（例: `OsOpen { fn open(&mut self, verb: &str, file: &str, params: Option<&str>) -> Result<(), u32> }`）を 1 つ切り、本物は `ShellExecuteW`、テストは「何を・どの動詞で・どの引数で」を記録し、指定の符号を返す偽物。先例は `InstallPorts`（`install/procedure.rs:42`）と `Fetch`（`areka-update/src/fetch.rs:8`）。判断（行き先の解決・分類・記録の有無と欄）は trait の手前の純粋な関数に寄せ、テストは偽物でそこを踏む。`readme_tests.rs:243` の本物の呼び出しは偽物へ置き換える（常時テストで OS を触る回数 0）。

### 4.5 行き先を取り出す純粋な関数（要件 8）

- 置き場の候補: ⑴ `crates/areka/src/readme/` の子（開く処理の分類関数と同じファイル群＝8.6 の「同じ規則」が構造で保たれる。`link-context-copy`・`balloon-link-hover` も `areka` クレートなので届く）。⑵ `areka-sakura` の新モジュール（純粋クレート・`sysvar` と同じ公開の仕方）。⑴ が最短。
- 中身: `areka_parsers::sakura::parse(script)` → `GenericCommand` を順に見て、`("open", 5 種)` と運搬名 `\j` を `行き先{種類, 書かれた綴り, 元のタグ}` へ。相対パス・環境変数は触らない（8.4）。`\![open,readme]`・`\q`・`\_a`・本文は落ちる（8.3）。
- 「元の綴り」: `parse` は `"…"` の引用符や `\]` を字句の段で解いてしまう。`\![open,file,manual.pdf]` のように組み直した綴りで足りるか、字句どおりが要るか（lexer の範囲情報を公開する改修＝`lexer.rs` を触る）は設計で決める。後続 2 本の brief（`link-context-copy` brief:28「行き先 1 つにつき 1 項目」）は行き先の値しか使わないので、組み直しで足りる見込み。

### 4.6 全体の進め方: A（既存を広げる）／B（新規）／C（混成）

- **A 既存を広げる**: `readme.rs` の `open()` を動詞つきに広げ、`ReadmeCueSink` を広げ、台帳に行を足す。新しいファイルは無し。→ `readme.rs` が「説明書」の名前のまま開く系全部を抱える・1 ファイルが太る。
- **B 新規に作る**: `readme.rs` の子に `open_external.rs`（分類・解決・記録・OS の口）・`destinations.rs`（取り出し関数）・`os_open.rs`（trait と本物）を置き、`readme.rs` は説明書の決め方と結線だけ残す。→ ファイルは増えるがテストの兄弟配置と 1,000 行の目安に沿う。
- **C 混成（推し）**: 結線の綴り（`ReadmeRequest`・`wire_readme`・`drain_readme_requests`・`open_from_world`）は `readme.rs` に残して mod.rs 非接触を保ち、中身の新規部分は B のとおり子へ。`readme_cue.rs` は自己選別の広げだけ。

規模: **M（10〜14 タスク）**＝brief の見込みどおり。危険度: **中**（スレッドと COM・`get_log` の表の変更・名前解決の結線が新しい。読み込みと登記は先例どおりで低）。

## 5. 要調査（設計へ持ち越す）

1. `ShellExecuteW` と COM の初期化（専用スレッドで `CoInitializeEx` を要するか・無くても動くか）。
2. `file:///` の後ろの読み方: `file:///C:/x%20y.txt` のようなパーセント符号化を復号するか（ukadoc は沈黙・`areka-update/src/urlpath.rs` に復号の部品はある）。`file:///descript.txt`（相対）と `file:///C:/…`（絶対）の見分け方（3 本目の `/` の後ろが `X:` か）。
3. `\j[ID]` の旧来の「ID にジャンプ」が SSP で何をするか（§6.1）。SSP ヘルプの MCP で引く価値あり。
4. 名前だけの実行ファイルを `ShellExecuteW` に渡したとき、`ghost/master` を作業フォルダ（第 5 引数）に渡すべきか（相対の資料を読むアプリの都合）。
5. 「元の綴り」を字句どおりに返す必要があるか（§4.5）。

## 6. 未決の 3 項目の選択肢（根拠つき）

### 6.1 `\j[ID]` が URL でも `file:///` でも `mailto:` でもないとき（要件 1.7）

| 案 | 触る所 | 根拠・見立て |
|---|---|---|
| **⒜ 開かず `warn!` 1 行（暫定）** | 分類関数の 1 枝だけ | kanade に触らない。台帳は `degraded`（旧来のジャンプを実装しない）。コードに「ID にジャンプ」を扱う場所は今 0 か所 |
| ⒝ 選択肢を選んだときと同じ扱い（`OnChoiceSelect` 系へ ID を渡す） | 受け口から選択の送り（`input_events/choice_drain.rs`）または kanade へ新しい経路 | 「ジャンプ」の歴史的な意味は確認できていない（要調査 3）。確認できても kanade の列（C4 で `balloon-lifecycle-events`・`choice-script-prefix` が使う）と接触する |
| ⒞ `http://` 省略の URL とみなす | 分類関数の 1 枝 | `ssp.shillest.net/` のような書き方を救えるが、正典の記述に無い推測。ホスト名かパスかの見分けも曖昧 |

答えで変わるもの: 台帳の `status`（⒜⒞＝`degraded`、⒝＝`implemented`）・kanade への接触（⒝のみ）。

### 6.2 `\![open,explorer,種類,名前]` の `headline`・`plugin` と名前解決の範囲（要件 4.4〜4.7）

- `headline`・`plugin`: areka に仕組みが無い → `warn!` 1 行の縮退一択（台帳は `degraded` の理由に書く）。
- `ghost`・`balloon`・`shell` の名前解決を**本 spec で持つ**場合の規模: 純粋な解決関数は 3 つとも既にある（§2.5）。足すのは「`BootContext` の root から候補を作って呼ぶ」結線と、見つからないときの `error!`、特別な名前（`random`・`lastinstalled`・`sequential`）を**使わない**判断（ukadoc の例は `Emily/Phase4` の名指しだけ）。見立て +2 タスク（結線＋テスト）。
- **縮退に留める**場合（`\![open,readme,種類,名前]` と同じく語彙だけ受理して `warn!`）: −2 タスク。ただし要件 4.4〜4.6 を書き替えることになり、台帳は `degraded`。
- 答えで変わるもの: M の中での上下 2 タスク・要件 4 の文言・台帳の語。

### 6.3 成功の記録を `get_log` のどの種別に入れるか（要件 7.3）

| 案 | 触る所 | 見え方 |
|---|---|---|
| **① `status` の規則に `areka::readme`（下も含む）を 1 行足す** | `log-convention.md:40-58` の表・`log_history.rs:113-127 RULES`・`log_history_convention_tests.rs`（表の読み手と `TARGETS` の対照） | `#n 日時 [STAT] STAT : 本文 kind=url target=… ghost="emo2"`。`status` は取り決めの target でないので `ghost` 欄は「名」にならず本文に残る（`log-convention.md:74`） |
| ② 成功も `areka::log::error` へ info で出す | 出す側だけ | `error` 種別に成功が混じる＝名前が嘘になる。却下寄り |
| ③ 成功は履歴に残さない（標準出力の `info!` だけ） | 無し | 要件 7.3 に反する |
| ④ 新しい取り決めの target（例 `areka::log::status`）を作り、`ghost`・`label` を読ませる | `TARGET_*` の追加＝`log_history.rs:182` の判定・表・テスト | ゴースト名が「名」に出る。SSP の 5 種別の枠は保つが取り決めの target が 3 つに増える |

失敗（7.4）は案に依らず `error!(target: "areka::log::error", ghost = %name, …)` でゴースト名つきの `error` 種別になる（`log-convention.md:27`・`:65-75`）。推し: **①**（表の 1 行＋テストの対照の更新・SSP の `status` も「節目」の種別なので意味が近い）。SSP 自身が URL を開いたときにどの `log_type` へ残すかは未確認（SSP ヘルプの MCP で引ける）。

## 7. 設計へ渡す決めごと（番号は要件ディスカッションの議題の候補）

1. `\j` の運搬名の綴りと定数の置き場（§4.1 ア: parsers 側に置いて sakura が再公開するか）。
2. `CommandConsumer` の変種を `ReadmeSink` のまま広げるか、`OpenSink` へ改名するか（`consumer_ledger.rs` の中だけで済む）。登記の件数は 20（`\j` を登記しない）か 21（運搬名を選別子なしで登記）か。
3. 開く専用スレッドの採用（§4.3 A）と、記録を出すスレッド（UI か専用か）。COM の初期化の要否（要調査 1）。
4. 偽の境界の形（trait の引数＝動詞・対象・引数・作業フォルダ）と、`readme_tests.rs:243` の本物の呼び出しの置き換え。
5. 環境変数の展開を `std::env::var` の `%NAME%` 置換で行う（未定義の変数はそのまま残すか空にするか）。
6. 「ファイルを選んだ状態」を `explorer.exe /select,…` で行う（`SHOpenFolderAndSelectItems` は機能未有効のため使わない）。
7. `file:///` の相対・絶対の見分けとパーセント符号化の扱い（要調査 2）。
8. `\![open,browser,X]` の X が URL の形でないときの扱い（そのまま OS へ渡すか・`warn!` で止めるか）。
9. 行き先を取り出す関数の置き場（`readme/` の子か `areka-sakura` か）と「元の綴り」の忠実度（§4.5・要調査 5）。
10. 未決 1〜3（§6）。
11. `doc/COMPAT_ARCHITECTURE.md` §8 へ足す 4 行の文言（設定を写さない・表示行を無視・headline/plugin を開かない・`\j` の旧来のジャンプ）。
12. `\![open,explorer,種類,名前]` の名前に特別な名前（`random`・`lastinstalled`・`sequential`）が来たときに引かない判断と、そのときの記録（§6.2・ukadoc の例は名指しだけ）。

> 要件ディスカッション（2026-10-05）の結果: 未決 1（§6.1）は ⒜（開かず `warn!` 1 行・台帳は `degraded`・開発者「http とかが無ければ処理できない方がよい」）、未決 2（§6.2）は「名前解決を本 spec で持つ・headline/plugin は縮退」で片付いた。未決 3（§6.3）は上の 10 のうち設計で決める項目として残す。

---

# 設計フェーズの記録（2026-10-05 `/kiro-spec-design -y`）

## Summary

- **Feature**: `areka-P0-open-external-tags`
- **Discovery Scope**: Extension（既存の汎用の `\!` の運び手・受け口・説明書の開き方を広げる。新しい外部依存は 0）＝軽い調査（light）。
- **Key Findings**:
  - `emo2_boot/mod.rs` が綴るのは `crate::readme::ReadmeRequest` の型名・`ReadmeCueSink::new`・`resolve_path`・`wire_readme` の署名だけ（`mod.rs:491-492`・`:717-719`）。型の中身と関数の中身を変えれば `mod.rs` に触らずに開く系全体へ広げられる。開く専用のスレッドは `register_readme_drain`（`ghost_session.rs:61` からプロセスに 1 回）で起こせる。
  - `mcp/mod.rs:6` は `mod resolve;`（非公開）なので `mcp::resolve::active` は `readme` から使えない。ゴースト名と根は `GhostSlot`（`ghost_session.rs:390`・`pub(crate)`）から直に読み、名の決め方だけを `get_active_ghost_list` と揃える。
  - `ShellExecuteW` の公式の説明（Microsoft Learn「ShellExecuteW function」Remarks）は「COM を先に初期化するのがよい・`CoInitializeEx(NULL, COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE)`」と書く。フォルダは `open` で開ける・`edit` は文書でなければ失敗・32 以下が失敗の符号（`SE_ERR_NOASSOC` を含む）。`Win32_System_Com` と `Win32_UI_Shell` は既に有効。

## Research Log

### 開く専用のスレッドと COM
- **Context**: 要件 7.5（固まらない）・7.8（順序）。今は UI スレッドで World を借りたまま `ShellExecuteW`（`readme.rs:154-158`・`:165-198`）。
- **Sources**: Microsoft Learn `nf-shellapi-shellexecutew`（Remarks・Return value・lpDirectory）。先例 `install/fetch_url.rs` の `thread::Builder::new().name("install-fetch")`。
- **Findings**: 1 本のスレッド＋mpsc で順序と非停止を両立。COM は起動時に 1 度だけ STA で初期化する。lpDirectory を NULL にすると呼び手のプロセスの作業フォルダになる。
- **Implications**: `Opener::spawn` のスレッドの閉包だけが COM と本物の OS を持つ。`serve`・`execute` はテストから同じスレッドで呼ぶ（`log_capture_kit` は呼んだスレッドだけ）。

### ゴースト名と文脈の読み口
- **Context**: 要件 7.2 の「出どころのゴースト名」・`ghost/master` 基準の解決・`種類,名前` の目録。
- **Findings**: `GhostSlot`（置き場）→ `names().name`・`ghost_dir()`。`BootContext.root`（`BasewareRoot`・`Clone`）。名前から 1 つを引く関数は `resolve_switch_target`（`ghost_switch.rs:170`）・`shell_candidates`（`shell_balloon_resolve.rs:93`）・`balloon_candidates`（`:110`）。`resolve_skin_target` は `random`・`lastinstalled` を先に解くので、`balloon`・`shell` の名前の照合は候補の列に対して「name → フォルダ名」を直に行う。
- **Implications**: UI スレッドは文脈（名・フォルダ・根）を写すだけ。目録の読み取り（fs）は開く専用のスレッドで行う。

### 記録の振り分け
- **Context**: 要件 7.3・7.4。`log_history.rs:113` の `RULES` に `areka::readme` が無い。
- **Findings**: `rule("areka::readme", Kind::Status, false)` を足せば `areka::readme::opener` の `info` が `status` に入る。`log_history_convention_tests.rs` の `locate` は `areka::readme` → `src/readme.rs` で通る。失敗は `TARGET_ERROR`（`areka::log::error`）に `ghost` を付ければ `error` 種別で名がゴースト名になる（本番で初めての利用）。
- **Implications**: 表（`doc/ssp-mcp/log-convention.md`）・`RULES` の 2 か所を同時に 1 行ずつ。「13 行」の数は 14 へ。

### 網羅台帳の証拠の形
- **Context**: 要件 9.1。
- **Findings**: 証拠は定義箇所の `/// ukadoc: <URL>` 1 行（`doc/ukadoc-coverage/README.md` §3）。URL は entry の鍵の後半を `list_sakura_script.html#` に続けた形（既存の `ReadmeSink` の doc と同じ）。
- **Implications**: `destination.rs` の `classify` の腕に 6 行を置く。

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| 開く専用の 1 本のスレッド（採用） | 受け口は分類して送る・UI は文脈を写す・解決と OS はスレッドで | 固まらない・順序が保たれる・COM 1 回 | 長く待つ要求の後ろが待つ・終了時の未処理は捨てる | ギャップ分析 §4.3 案 A |
| 要求ごとにスレッド | 最短 | — | 順序が崩れる（7.8） | 不採用 |
| 台本のスレッドで直接 | 結線が最小 | — | 台本の再生が止まる・World が読めない | 不採用 |
| UI で同期（今） | 変更なし | — | 7.5 に反する | 不採用 |

## Design Decisions

### Decision: `\j` は運搬名 `"\j"` の汎用コマンドへ写す
- **Alternatives**: ⑴ 運搬名（採用）⑵ `\![open,j,ID]` 風（正典に無い語彙）⑶ decode で http/file/mailto に分類（元の綴りが失われ・転記層の規律に反する）
- **Selected**: `decode_tag` に `"j"` の 1 腕。定数 `JUMP_TAG_CARRIER` は `areka-parsers` の `sakura/model.rs` に置き公開。腕の中は完全なパスで書き、`decode.rs` の `use` 行も変えない（`anchor-tag-canon` との接触を腕 1 本に留める）。
- **Follow-up**: `compile.rs` の既存の汎用の腕が時間 0 を与えることは既に固定済み（`compile_arm_tests.rs` の `catch_all_ignored_set_is_raw_only` 等）なので再テストしない。

### Decision: 規則は 1 関数 `classify`、受け口と取り出しの両方が呼ぶ
- **Context**: 要件 8.6。
- **Selected**: `readme/destination.rs` の `classify(name, args) -> Option<Result<Destination, Rejected>>`。受け口は実行時に、`link_destinations` は `parse` の結果に対して呼ぶ。
- **Trade-offs**: 「元の綴り」は名前と引数から組み直した綴り（引用符と `\]` は戻らない）。後続 2 本は値だけを使うので足りる（ギャップ分析 §4.5）。lexer の範囲を公開する改修はしない。
- **解釈**: 要件 8.1 は「`\j[X]` と `\![open,…,X]` の X を返す」、8.2 は「各項目に種類（5 つ）を添える」。種類を持てない断られる入力（3 形以外の `\j`・引数なし・`headline`／`plugin`）は一覧に含めない＝「開く処理が実際に開こうとする行き先」と一致させる（8.6）。

### Decision: 断りは受け口で `warn!`、解決と OS は開く専用のスレッドで
- **Selected**: 断り（要件 1.7・2.6・3.2・4.7・6.3）は台本のスレッドで分類した時点で `warn!` 1 行・送らない。解決（fs）と OS の呼び出しと成功・失敗の記録は `execute` の 1 か所。
- **Rationale**: 断りに World は要らない。OS を呼ばないことが構造で明らか（送らない）。

### Decision: 受け取り手の表は `ReadmeSink` のまま 6 行を足す
- **Context**: ギャップ分析 §7 の 2。
- **Selected**: 受け口の型名 `ReadmeCueSink` は `mod.rs` が綴るので変えない。表の変種もそれに合わせて `ReadmeSink` のまま、doc を「開く系（説明書を含む）」へ。運搬名 `\j` も選別子なしで登記する（`\f` の運搬名と同じく、表が実際の担当を正しく映すため）。本 spec の分は +6。

### Decision: 開く専用のスレッドは `register_readme_drain` で 1 度起こし、持ち物 `Opener` に送信端
- **Alternatives**: `wire_readme`（ゴーストごと＝何度も呼ばれる）・`static`（終わりが無い）
- **Selected**: プロセスに 1 回の登録で起こし、World の持ち物にする。World が落ちれば送信端が落ちてスレッドは自然に終わる。

### Decision: 成功の記録は `status` 種別（`RULES` に `areka::readme` を 1 行）
- **Context**: ギャップ分析 §6.3 の未決 3（設計で決める）。
- **Selected**: 案①。`info` は OS へ渡す時点で 1 行（渡したこと自体を必ず残す）。失敗は `TARGET_ERROR`＋`ghost` で `error` 種別・名がゴースト名。
- **Trade-offs**: `status` は取り決めの target でないので、成功の行の `ghost` は本文に残る（`get_log` の `ghost_name` で絞れるのは失敗の行だけ）。案④（新しい取り決めの target）は取り決めを増やすので取らない。

### Decision: 解決の細部
- `\j[file:///…]` はパーセント符号化を復号しない（ukadoc の例に無い・`%` を含むファイル名を壊さない）。絶対かどうかは `Path::is_absolute`。残りが空なら断る。
- 前置き（`http://`・`https://`・`file:///`・`mailto:`）は大文字小文字を区別しない。
- 環境変数の展開は ukadoc が書く `\![open,file]` だけ。`%名前%` を `OsPort::env_var` で置き換え、未定義はそのまま残す（`ExpandEnvironmentStringsW` と同じふるまい。機能 `Win32_System_Environment` は未有効なので使わない）。
- 名前だけ（`\`・`/`・`:` を含まない）で `ghost/master` に無ければ、名前のまま `open` へ渡して OS のパス探索に任せる（`\![open,file]` だけ）。
- ファイルを選んだ状態は `explorer.exe` に `/select,"<パス>"`（`SHOpenFolderAndSelectItems` は未有効の機能を要する）。
- `\![open,browser,X]` は X の形を判定しない（要件 3.1「URL として渡す」・推測を足さない＝1.7 の裁定と同じ方針）。
- `\![open,file]` で開くファイルの作業フォルダはそのファイルのあるフォルダ（エクスプローラーのダブルクリックと同じ）。
- `\![open,explorer,種類,名前]` の特別な名前（`random` など）は解かない。`ghost` は `resolve_switch_target`（`\![change,ghost,名前]` と同じ引き方）。

## Synthesis

- **一般化**: 6 つの形は「行き先の綴り → 解決の規則 → OS の動詞」の 1 つの問題の変種。`Target` の変種を「解決の規則」ごとに分け（`Url`・`Mail`・`Path`・`Program`・`Folder`・`NamedFolder`・`Edit`）、説明書も `Path` の 1 件として同じ道へ載せた。
- **作るか使うか**: 既定のアプリの選択は OS（関連付け）を使う。名前の引き方は既存の目録と純粋な関数を使う。環境変数の展開だけは `std::env::var` で 10 行ほどを書く（OS の関数は未有効の機能を要し、`Cargo.toml` 非接触の約束がある）。
- **簡素化**: 説明書専用の `open()` を削り、OS を綴るファイルを 1 つにした。`ReadmeSink` の改名・新しい受け口・新しい結線はしない（`mod.rs` 非接触で済む最小）。取り出しの関数は `areka-sakura` へ置かず `readme/` の子に置く（規則と同じ場所＝8.6 が構造で保たれる）。

## Risks & Mitigations

- 開く専用のスレッドが OS で長く待つと後ろの要求が待つ — 画面と台本は止まらない。順序（7.8）を守る代償として受容。
- アプリの終了時に溜まった要求は捨てられる — 開く要求は使い捨てで、終了を待たせる理由が無い。
- `\![open,file]`・`\![open,browser]` は台本から実行ファイルを起こせる — 正典どおり。開くたびに必ず記録し、同意の窓は `script-impact-tiers` が `submit` へ差し込む。
- 受け取り手の表の総数は `balloon-lifecycle-events` と同時に動く — 後から main へ入る側が実物を数え直す（要件 9.3）。
- 本物の `ShellExecuteW` が「関連付けの無い拡張子」で OS の「アプリを選ぶ」窓を出すことがある — OS の受け口のふるまいであり areka のメッセージボックスではない（要件 7.6 は areka が出さないことを約束する）。実機確認で見る。

## References

- [ShellExecuteW function (shellapi.h)](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shellexecutew) — COM の初期化・動詞・戻り値・作業フォルダ。
- ukadoc さくらスクリプトリスト（§2.8 の 6 つの URL）。
- `doc/ssp-mcp/log-convention.md`（記録の種別の取り決め）・`doc/ukadoc-coverage/README.md` §3（証拠の書き方）。
