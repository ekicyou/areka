# ギャップ分析: talk-script-metadata

- 実施日: 2026-11-27（worktree の HEAD `6c2a33ad`・main `af6b9001` の上）
- 入力: `requirements.md`（未承認）・`brief.md`（2026-11-27 の再測定の節を含む）・steering（`structure.md` の crate の層と依存・`logging.md` の段の表を引いた）
- 方法: コードの読み（Grep／Read）と ukadoc MCP（`search_docs`）。外の依存の調べは要らない（新しいクレートを足さない）。

## 1. 分析の要約

- **台本を作る所は全部 kanade の運行表（`schedule/`）の中にある**。`StartTalk` を作るのは `mod.rs` の `start_talk`（呼び手 7 か所）と、直に組む `boot.rs` 2 か所・`change.rs` 2 か所・`close.rs` 1 か所の計 6 か所で、全部 `step` の出口を通る。kanade はゴースト 1 体に 1 つで、`KanadeConfig` は既に `ghost_name` を持ち、それを入れる `resolve_kanade_config` は `mount`（`shiori.dir`＝ghost/master）を手に持っている。**置き場は `config.rs` だけで運べる**（`runtime.rs` に触らない約束と両立する）。
- **受け手へ渡す道（`begin_talk(talk_id, origin)`）は既にある**が、`ScriptOrigin` は `Copy` で、置き場（`PathBuf`）を足すと引数の形を変えることになる。実装は既定（`sink.rs`）・`BalloonLifecycleSink`・`NoUserBreakCueSink`、テストの 2 つ。
- **開く系の受け口は `begin_talk` を受けていない**。`ReadmeCueSink` は `#[derive(Clone)]` で一括の実装に乗っており、要求（`ReadmeRequest::Open`）は UI スレッドの取り出しで `opener::submit` に着き、**そこで `GhostSlot` を引く**（`opener.rs:171-199`）。付帯情報に乗り換えるには、受け口が `BootCueSink` を自分で実装し、要求に付帯情報を載せて運ぶことになる。
- **相対パスを解く共通の決まりはまだ無い**。今あるのは `opener.rs` の `existing()`（絶対はそのまま・相対は `<root>/ghost/master` に繋いで `exists()`）だけ。探す順の 2 通り・置き場が無いとき・空のパス・`warn!` の 1 行は新しく作る。
- **要件と今のコードの食い違いが 6 件ある**（§5）。中でも、選択肢の `script:` は元のトークの出どころ（MCP を含む）を引き継いでいるのに要件 2.2 は送り手を「ゴースト自身」とする点、`\![open,browser|mailer]` は今パスを解かない点、ukadoc の `\![open,file]` の記述例が ghost/master の基準と食い違う点は、要件の議論で決めたい。

## 2. 今のコードの姿

### 2.1 台本の情報の型（`crates/areka-talk`）

- `StartTalk { talk_id, script, epilogue, origin, strict }`（`lib.rs:43-50`）。`StartTalk::new` は `origin = ScriptOrigin::GHOST`・`strict = false`。
- `ScriptOrigin { sender_type, sstp, level }`（`origin.rs:121-126`・`Copy`）。定数 `GHOST`・`MCP_SEND`・`MCP_NOTIFY`・`PROPERTY`。
- crate は `dola::time` と `std` だけに依存する（steering `structure.md`「依存ゼロ（std のみ）・エンジン知識を持ち込まない」）。

### 2.2 kanade で台本を作る所（全部）

`StartTalk` を組む所（本番のコードだけ・テストを除く）:

| # | 場所 | 何のトーク | 出どころの決め方 |
|---|---|---|---|
| 1 | `schedule/mod.rs:132` `start_talk` | 共通の手（採番・枠の差し替え） | 呼び手が渡す |
| 1a | `sstp.rs:62` `on_send` | MCP `sakurascript`（`SstpSend`） | `MCP_SEND`。翻訳へ預ける |
| 1b | `sstp.rs:137` `on_notify_reply` | MCP `raise_event`（`SstpNotify`）への SHIORI の応答 | `MCP_NOTIFY`（出口の `stamp` が元のイベント `EventId::Sstp` の出どころで上書き） |
| 1c | `steady.rs:399` `on_cascade_reply` | 選択肢のイベントの段の応答（`OnChoiceSelect` ほか） | `GHOST`→`stamp` |
| 1d | `steady.rs:609` `on_timeout_reply` | `OnChoiceTimeout` の応答 | `GHOST`→`stamp` |
| 1e | `steady.rs:787` `Steady{talk: None}` | 定常の応答（pump・マウス・アンカー・中断 `OnBalloonBreak`・バルーンのイベント・`\![get,property]` の返し・`OnTranslate` 以外の全部） | `GHOST`→`stamp`（`EventId::Property` なら `PROPERTY`） |
| 1f | `steady.rs:803` `Steady{talk: Some}` 置き換え | 同上（話している最中の置き換え） | `GHOST`→`stamp` |
| 1g | `steady_choice_script.rs:52` `begin` | 選択肢の `script:` | `talk_origin::of_shown`＝**元のトークの最終の出どころを写す** |
| 2 | `boot.rs:262` | 起動の挨拶（`OnBoot`・`OnFirstBoot`・`OnGhostChanged`・読み直し） | `GHOST`→`stamp`。`epilogue` 付き |
| 3 | `boot.rs:282` | **後書きだけのトーク**（初回起動で挨拶が 204 のとき・台本は空） | `GHOST`。応答の台本が無いので `stamp` は効かない |
| 4 | `change.rs:263` | 切替の `OnGhostChanging` の台詞／切替の `OnClose` の台詞（`closing` の枝で共用） | `StartTalk::new`→`stamp` |
| 5 | `change.rs:416` | 取りやめた切替の台本（定常のトークとして流す） | `StartTalk::new`→`stamp` |
| 6 | `close.rs:73` | 終了の `OnClose` の台詞 | `StartTalk::new`→`stamp` |

後から `StartTalk` を書き換える所:

- `talk_origin::stamp`（`step` の出口・`mod.rs:169`）: 応答の一括の全部の `StartTalk` の `origin` を元のイベントの基本の出どころ（応答のレベルで下げた値）で上書き。
- `translate::on_done`（`translate.rs:280-310`）: 預けた一括の `script` と `origin`（レベルを下げるだけ）を書き換える。**他の欄はそのまま**＝付帯情報を `StartTalk` に持たせれば翻訳の後も保たれる（要件 2.4 は形の上で自然に満たせる）。
- `sstp::mark_strict`: `strict` だけ。
- `balloon_events::settle`（`step` の出口）: 最後の `StartTalk` を `ShownTalk { talk_id, script, origin, … }` に控える。選択肢の `script:`（1g）はこの控えの `origin` を写す。

`actor.rs:487-488` の `execute_batch` が `Action::StartTalk` を `TalkCommand::Start` へ写して送るだけで、kanade の外で `StartTalk` を作る本番のコードは無い（テストの `prop_sink.rs:354`・`common_smoke.rs`・`areka-seriko/tests/common` などは除く）。

### 2.3 受け手への道

`actor.rs:488` → relay → `dispatcher.rs:281` `on_start` が登録の受け口を `clone_box` で複製し、複製ごとに `begin_talk(talk_id, origin)` を 1 回呼ぶ（`dispatcher.rs:285-297`）→ `spawn_talk` → `areka-sakura/src/drive.rs:225` が `StartTalk` を分解（`origin: _` で捨てる・分解は網羅形なので欄を足すとここも直す）→ `CueSheet` は時刻と cue だけ。

- `BootCueSink::begin_talk`（`sink.rs:39`）の既定は何もしない。`impl<T: CueSink + Clone + Send + 'static> BootCueSink for T` の一括の実装に乗る受け口は上書きできない。
- `begin_talk` を自分で実装しているのは `BalloonLifecycleSink`（`areka-runtime/src/talk_lifecycle.rs:278`・番号だけ使う）と `NoUserBreakCueSink`（`user_break_cue.rs:128`・出どころで `\![enter,nouserbreakmode]` の許しを判定）の 2 つ。テストに `dispatcher_begin_talk_tests.rs:44,175`。
- 受け口の並び（`areka-session/src/runtime_boot.rs:79-93`）は 13 本。ゴーストの起動ごとに組み直し、readme の線（`readme_tx`/`readme_rx`）も起動ごとに新しく張る。

### 2.4 開く系の受け口と opener

- `ReadmeCueSink`（`areka-session/src/readme/readme_cue.rs`・`#[derive(Clone)]`）は自分宛の cue を `destination::classify` で分類し、`ReadmeRequest::Open(Destination)` を UI スレッドへ送るだけ。`begin_talk` は受けない。
- UI スレッドの `drain_readme_requests`（`readme.rs:203-210`）→ `opener::submit(world, dest)`（`opener.rs:171`）が **`GhostSlot` を引き**、`ghost_dir`（ゴーストの最上位・`std::path::absolute` 済み）と名（`listed_name`）と `BootContext` の根の並びから `OpenContext` を組み、開く専用のスレッドへ送る。ゴーストが居なければ `open_external_dropped`（`reason = "no_ghost"`）で捨てる。
- 開く専用のスレッドの `resolve`（`opener.rs:211-268`）:
  - `Target::Url`（`\![open,browser]`・`\j[http(s)://]`）・`Target::Mail`（`\![open,mailer]`・`\j[mailto:]`）: **パスを解かずそのまま OS へ**。
  - `Target::Path`（`\j[file:///…]`・説明書）・`Target::Folder`（`\![open,explorer,X]`）・`Target::Edit`（`\![open,editor,X]`）: `existing()`。
  - `Target::Program`（`\![open,file,X]`）: `%名前%` を展開 → `existing()` → 無くて区切り（`\`・`/`・`:`）を含まない名前だけなら OS のパス探索へ。
  - `Target::NamedFolder`（`\![open,explorer,ghost|shell|balloon,名前]`）: ghost は根の並びの目録（`list_ghosts`）、shell は `shell_candidates(ghost_dir)`、balloon は `balloon_candidates(根, ghost_dir)`（今のゴーストの中が先）。
  - `existing()`（`opener.rs:337-349`）: 絶対はそのまま、相対は `ghost_dir/ghost/master/<書かれたパス>`。`exists()`（ファイルもフォルダも）で判定、無ければ `NotFound(試したパス)`。`..` は断らない。
- 解けないときの記録は `error!`（`target: TARGET_ERROR`・`event = "open_external_failed"`・`reason = "not_found"` ほか・`ghost` の欄付き）。
- 空の書かれたパスは **`classify` が先に断る**（`\![open,file,]`・`\j[file:///]` は `MissingArgument`）ので opener には届かない。例外は `%X%` が空の値の環境変数に展開された `\![open,file]` だけ（今は `ghost/master` そのものが「在る」と解けて開かれる）。
- 説明書（メニューと引数なしの `\![open,readme]`）は `ReadmeWiring.path`（起動時に決めた絶対パス）を `Target::Path` で `submit` へ渡す＝基準のフォルダは使わない（名と根の並びだけ `GhostSlot`・`BootContext` から）。

### 2.5 MCP の宛先

- `areka/src/mcp/resolve.rs:36` `active()` と `ask_kanade()`（`resolve.rs:74-130`）が **1 つだけの `GhostSlot`** のゴーストの kanade へ `KanadeMsg::SstpSend`（`sakurascript.rs:62`）／`SstpNotify`（`raise_event.rs:58`）を送る。`SstpSend`・`SstpNotify` を作るのはこの 2 つだけ（`msg.rs:302-315` の欄は `script|event, references, strict, reply`＝送り手の名は無い）。
- kanade はゴースト 1 体に 1 つなので、**受けた kanade のゴースト＝喋るゴースト**。要件 2.3 の「実行するゴースト＝送り先」は、kanade の側で付ければ形の上で満たせる。

### 2.6 置き場を kanade へ運ぶ所

- `resolve_kanade_config(mount, default_encoding)`（`areka-ghost/src/config.rs:24-33`）は `mount: &MountModel` を持つ。`mount.shiori.dir` は「ghost_root/ghost/master（存在確定済み）」（`areka-parsers/src/package/model.rs:106-108`）、最上位は既存の `listed_ghost_name` と同じく `shiori.dir.ancestors().nth(2)`。**ghost/master と最上位の両方がここで取れる**。絶対かどうかは起動の根の渡し方次第なので、`std::path::absolute` を当てる（`listed_name`・`opener::submit`・MCP の `active()` と同じ手）。
- 本番で `KanadeConfig` を作るのは `runtime.rs:578` の `resolve_kanade_config` 1 か所で、その後の `apply_boot_record_gate`・`choice_wake`・`boot_origin`・`shell_folder` の上書きは他の欄だけ（`runtime.rs:578-630` を読んだだけ・触らない）。**`KanadeConfig` に欄を足し `config.rs` で埋めれば `runtime.rs` に触らずに済む**。
- `KanadeConfig::new` は置き場を知らないので、テストの支え（`KanadeConfig::new("master", "1.0.0")` が 40 か所以上）は「置き場が無い」になる＝要件 1.6／仮定 A1 と合う。
- `KanadeConfig.ghost_name` は既に `listed_name` で作られ、script の記録（`script_log.rs:59`）と strict の記録が使う＝要件 1.4 の名はそのまま使える。

### 2.7 ukadoc の基準の語（MCP `search_docs` で引いた逐語）

- `\j[ID]`（https://ukatech.shillest.net/ukadoc-spec/list_sakura_script.html#j-ID ）: 「file:///～でOSに関連付けられた拡張子でファイルを実行する。ファイル名には、絶対パスの他、[実行したゴーストのフォルダ]/ghost/masterからの相対パスが使用可能。」
- `\![open,file,ファイル名]`（https://ukatech.shillest.net/ukadoc-spec/list_sakura_script.html#bang-open-file ）: 「[実行したゴーストのフォルダ]/ghost/master/からの相対パスか、絶対パスで指定。」ただし記述例は「\![open,file,shell/master/surface0.png]shell/masterフォルダのsurface0.pngが開かれる。」＝shell/master はゴーストの最上位の下にあるので、**本文（ghost/master 基準）と記述例（最上位基準に見える）が食い違う**。
- `\![open,browser,パラメータ]`（#bang-open-browser）・`\![open,mailer,パラメータ]`（#bang-open-mailer）・`\![open,editor,ファイル,表示行]`（#bang-open-editor）・`\![open,explorer,ファイル]`（#bang-open-explorer）: **相対パスの基準の記述は無い**（browser・mailer の例は URL とメールアドレスだけ）。

## 3. 要件と今ある部品の対応表

| 要件 | 要るもの | 今あるもの | ギャップ |
|---|---|---|---|
| 1.1・1.2 付帯情報の中身・実行するゴーストを別の欄 | 名・ghost/master・最上位・送り手を持つ型と、`StartTalk` の欄 | `StartTalk.origin`（種類と信用度だけ） | **Missing**: 型と欄 |
| 1.3 絶対パス | `std::path::absolute` | `listed_name`・opener・MCP が同じ手を使う | **Constraint**: `config.rs` で 1 回当てる |
| 1.4 名の決め方 | `listed_name` | `KanadeConfig.ghost_name` | 揃っている |
| 1.5 今の出どころを変えない | `ScriptOrigin` の値 | `stamp`・`translate::on_done`・`of_shown` | **Constraint**: 送り手を `ScriptOrigin` に入れるなら `Copy` と全部の比較（`PartialEq` の網羅テスト）が影響を受ける |
| 1.6 置き場が無い | `Option` の置き場 | `KanadeConfig::new` が置き場を知らない | **Missing**: 無いことの表し方 |
| 2.1 送り手の名 | ゴースト名 or 道具名 | `RequestOrigin.sender`（`SenderName::{Areka, Mcp}`＝SHIORI の `Sender` 頭書き用・`request_origin.rs`） | **Missing**＋**Unknown**: 既存の `SenderName` との関係（§4 の決め 3） |
| 2.2 ゴースト由来のトーク | 送り手＝実行するゴースト | 表の 1c〜1f・2〜6 | 1g（選択肢の `script:`）は今、元のトークの出どころを写す（§5 の 1） |
| 2.3 MCP | 道具の名・送り先＝実行するゴースト | `SstpSend`↔`sakurascript`・`SstpNotify`↔`raise_event` が 1 対 1 | **Unknown**: 道具の名を `msg.rs` で運ぶか kanade の中の定数にするか |
| 2.4 翻訳 | 送り手を保つ | `translate::on_done` は他の欄を保つ | 満たせる（欄を `StartTalk` に置く限り） |
| 2.5 別のゴーストを表せる | 送り手のゴーストと実行するゴーストが別の値を持てる型 | ― | **Missing**（型だけ） |
| 3.1〜3.3 始まりで決め、変えない | 作る所で埋め、受け手は値で持つ | `start_talk`・`step` の出口 | **Missing**: 埋める所（§4 の決め 1） |
| 3.4 途中の切替で基準が変わらない | 受け手が `GhostSlot` を引かない | opener が引く | **Missing**: §3 の 6.2 |
| 4.1・4.2 受け手へ届ける・持ち越さない | `begin_talk` に付帯情報 | `begin_talk(talk_id, origin)`・複製＝トークの境界 | **Missing**: 引数の形（§4 の決め 2） |
| 4.3 使わない受け手を変えない | 既定の実装が何もしない | 一括の実装の既定 | 満たせる |
| 4.4 `CueSheet` に持たせない | ― | `drive.rs` が `origin: _` で捨てる | 満たせる（`drive.rs` の分解に 1 欄足すだけ） |
| 5.1〜5.8 共通の決まり | 純粋な関数＋`exists()` | `opener::existing()`（ghost/master だけ・絶対・`..`） | **Missing**: 探す順「ghost/master → 最上位」・置き場が無い・空・最後の候補を返す |
| 5.9 `warn!` 1 行 | 記録 | opener は `error!`（`open_external_failed`） | **Constraint**: 記録の段と重なり（§5 の 4） |
| 5.10 環境変数は opener に残す | `expand_env` | opener にある | 満たせる |
| 5.11 本体のフォルダを入れない | ― | ― | 満たせる |
| 6.1〜6.4 開く系の乗り換え | 受け口が付帯情報を受け、要求に載せて運ぶ | `ReadmeCueSink` は `Clone`・`ReadmeRequest::Open(dest)`・`submit` が `GhostSlot` | **Missing**: 受け口の `BootCueSink` の自前の実装・要求への載せ方・`OpenContext` の作り方 |
| 6.5 1 体のとき前と同じ | 振る舞いの固定 | `opener_tests.rs`・`opener_submit_tests.rs`・`opener_execute_tests.rs` | **Constraint**: 空の展開の端（§5 の 3）と browser/mailer（§5 の 2） |
| 6.6 メニューの説明書は今のまま | `open_from_world` | ある | 満たせる（台本の要求と道を分ける） |
| 7.2 `target\` の下 | `temp-path-kit` の `TempPath::under_target` | ある | 満たせる |
| 7.3 `warn!` を数える | `log-capture-kit` | ある（`areka-session` の dev 依存） | 満たせる |

## 4. 実装の案

### 決め 1: 付帯情報を埋める所

- **案 1-A（`step` の出口で一括に押す）**: `talk_origin::stamp` と同じく、`step` の出口で返す一括の全部の `StartTalk` に `config` の実行するゴースト（名・置き場）を押す。送り手は出どころと入力の種類（`SstpSend`／`SstpNotify` の応答）から決める。作る所 6 か所は触らずに済み、作る所が増えても漏れない。翻訳の結果の入力（`TranslateDone`）の `step` でも押し直すが、`config` はその kanade で変わらないので同じ値になる。
  - ✅ 漏れが構造で起きない・差分が小さい／❌ 「出口で上書き」の規則が 2 つ（出どころと付帯情報）になり、腕で付けた値が出口で消える形を増やす。
- **案 1-B（`start_talk` と直に組む 5 か所で埋める）**: `start_talk` に `config` を渡し、`boot.rs`・`change.rs`・`close.rs` の直の組み立てを `start_talk` 相当の手に寄せる。
  - ✅ 作る所で決まる（要件 3.1 の字面に近い）／❌ `start_talk` の呼び手 7 か所と直の 5 か所を全部触る。`StartTalk::new` の既定（置き場が無い）を残すと、埋め忘れが試験でしか見つからない。
- **案 1-C（`actor.rs` の `execute_batch` で送る直前に押す）**: 運行表を純粋なまま残し、殻が `config` から押す。
  - ✅ 運行表のテストを変えない／❌ 運行表のテストで付帯情報を確かめられない（要件 7.1 の「作る所ごと」が殻のテストになる）。選択肢の `script:` の送り手を控えから写すには運行表の側に値が要る。

### 決め 2: 受け手へ渡す形

- **案 2-A（`begin_talk` の引数を付帯情報 1 つにする）**: `begin_talk(&mut self, talk_id, meta: &TalkMeta)`（出どころは `meta` の中）か、`begin_talk(talk_id, origin, meta)`。付帯情報は `Arc` で包み、13 本の複製に配っても複写は参照の数だけ。
  - ✅ 道は今のまま・受け手が値で持てる（要件 3.3）／❌ 実装 3 つとテストの 2 つ（`dispatcher_begin_talk_tests.rs`）の引数を変える。
- **案 2-B（付帯情報を `ScriptOrigin` に入れる）**: brief が採らなかった案。`Copy` を失い、`ScriptOrigin` を値で比べる所（`translate_level_tests.rs` ほか多数）と `of_shown` の写しの意味が変わる。要件 1.5 の「値を変えない」を守るのが難しい。

### 決め 3: 送り手の持ち方

- **案 3-A（付帯情報の別の欄）**: `sender: Sender::Ghost(名) | Sender::Tool(名)` を付帯情報に持つ。`ScriptOrigin` は今のまま。
- **案 3-B（`RequestOrigin` を使い回す）**: 既に `RequestOrigin { sender: SenderName, origin }` があるが、`SenderName` は SHIORI の `Sender` 頭書きの値（ゴースト由来のイベントは `"areka"`＝ベースウェア）で、要件 2.2 の「送り手＝実行するゴースト自身」とは意味が違う。使い回すと頭書きの値（SSP 互換）を巻き込む。
- 道具の名の運び方: ⑴ kanade の中で `SstpSend`→`"sakurascript"`・`SstpNotify`→`"raise_event"` の定数（今は 1 対 1）、⑵ `KanadeMsg::SstpSend`／`SstpNotify` に送り手の欄を足して MCP の側が入れる（`msg.rs`・`sakurascript.rs`・`raise_event.rs`・`actor.rs:202-215`）。⑵ は後の `sstp-receive` が同じ口を使うときに曲がらないが、`shiori4-api`（後のウェーブ）と `msg.rs` を取り合う。

### 決め 4: 共通の決まりの置き場

- **案 4-A（`areka-talk`）**: 付帯情報の型の隣に純粋な関数（`exists()` は引数の述語で受ける形にもできる）。どの受け手の crate からも `areka_sakura::contract` 経由で届く。❌ 「エンジン知識を持ち込まない」葉の crate に ukadoc のタグの決まり（探す順）を置く。
- **案 4-B（`areka-runtime`）**: 受け手（`*_cue.rs`）の層。`areka-session`・本体・下流の `balloon-inline-image`（文字の層）・`tasktray-ghost-icon`（本体）から届くか要確認（`areka-emo-text` は `areka-runtime` に依存しない）。
- **案 4-C（`areka-ghost` の `config.rs` の隣）**: 置き場を作る所と同じ crate。`areka-ghost` に依存しない受け手が後で困る。
- どの案でも fs を読むのは開く専用のスレッドなど UI スレッドの外で呼ぶ（opener の今の形を保つ）。

### 決め 5: opener の乗り換え方

- **案 5-A（要求に付帯情報を載せる）**: `ReadmeCueSink` の `#[derive(Clone)]` を外し `BootCueSink` を自分で実装（`NoUserBreakCueSink` と同じ形）、`begin_talk` で受けた付帯情報を `ReadmeRequest::Open { dest, meta }` に載せる。`submit` は台本の要求では `GhostSlot` を引かず `meta` から `OpenContext` を組み（根の並びは今のまま `BootContext`）、`resolve` の `existing()` を共通の決まりに置き換える。メニューと説明書の道（`open_from_world`）は今のまま。
- **案 5-B（`OpenContext` を付帯情報そのものに替える）**: `OpenContext.ghost_dir` を付帯情報で置き換え、説明書の道も付帯情報（表示中のゴーストから組んだもの）を使う。道が 1 つにまとまるが、メニューの道で「表示中のゴーストから付帯情報を組む」手が要る。

### 総じて

- **ハイブリッド（推す形の素案・決めは設計の段）**: 型は `areka-talk`（決め 2-A・3-A）、埋めるのは `step` の出口か `start_talk`（決め 1）、置き場は `config.rs`、opener は 5-A。新しいファイルは型と共通の決まりのテスト程度。

## 5. 要件と今のコードの食い違い

1. **選択肢の `script:` の送り手**（要件 2.2）: `steady_choice_script.rs:51` は `talk_origin::of_shown` で**元のトークの最終の出どころをそのまま写す**（元が MCP の `sakurascript` なら `MCP_SEND`・Owned）。要件 2.2 は選択肢の台本の送り手を「実行するゴースト自身」とするが、要件 1.5 で出どころの値は変えないので、出どころは MCP・送り手はゴーストという食い違った組になる。送り手も元のトークから写す（`ShownTalk` に控える）のが今の作りと揃う。`balloon_events.rs` の `ShownTalk` も触る所に入る。
2. **`\![open,browser|mailer]` は今パスを解かない**（要件 6.1・6.5）: `Target::Url`・`Target::Mail` は書かれた綴りのまま OS へ渡す。ukadoc にも基準の記述が無い。要件 6.1 が browser・mailer を「相対パスを持つ」ものに挙げているのは、6.5（1 体のとき前と同じ）と矛盾しうる。乗り換えの対象は `\![open,file|explorer|editor]` と `\j[file:///]`（と `\![open,explorer,shell|balloon,名前]`）に絞るのが今の形に合う。
3. **空のパス**（要件 5.8 と 6.5）: 開く系の空の引数は `classify` が先に断るので、共通の決まりの「空なら解けない」は opener では `%X%` が空の値に展開された `\![open,file]` の端でだけ効く。今はその端で ghost/master のフォルダが開く。5.8 に従うと、区切りを含まない空の名前として OS へ渡る枝（`opener.rs:236-238`）に落ちるか、`NotFound` になる。6.5 の「前と同じ」の例外として明示するかを決めたい。
4. **解けないときの記録**（要件 5.9 と 6.5）: opener は今 `error!`（`target: TARGET_ERROR`・`open_external_failed`・`reason = "not_found"`）を残し、`get_log` の error 種別に載る。共通の決まりが `warn!` を 1 行残すと、opener の道では `warn!` と `error!` の 2 行になる（重なり）か、opener の `error!` を `warn!` に下げる（6.5 の「断り」と `get_log` の見え方が変わる）。記録を共通の決まりの呼び手に任せる形（決まりは結果だけ返す）も取れる。
5. **ukadoc の `\![open,file]` の記述例**（仮定 A4）: 本文は「ghost/master/からの相対パス」、記述例は `shell/master/surface0.png` を「shell/masterフォルダの…が開かれる」とする（§2.7）。今の opener は本文どおり ghost/master だけ。SSP が実際に最上位も探すなら探す順は「ghost/master → 最上位」になる。memory の決まり（ukadoc で読めない点は議題の前に SSP の MCP で測る）に当たる。
6. **要件 2 の作る所の挙げ方**: 区分の挙げ方としては漏れは無いが、次の 3 つは字面に出ていない。⑴ **後書きだけのトーク**（`boot.rs:282`・初回起動で挨拶が 204・台本は空で応答の出どころの `stamp` が効かない）、⑵ **切替の `OnClose` の台詞**（`change.rs:263` の `closing` の枝）、⑶ **取りやめた切替の台本**（`change.rs:416`）。⑴ は送り手がゴースト自身で良いはずだが、要件 3.2「付帯情報を持たないトークを始めない」の確かめ（7.1）の表に載せたい。

（参考）brief の「触るファイル」に無いが触ることになる所: `crates/areka-sakura/src/drive.rs:225`（`StartTalk` の網羅の分解）・`crates/areka-session/src/readme.rs`（`ReadmeRequest`）・`readme_cue.rs`（`Clone` を外す）・`crates/areka-session/src/runtime_boot.rs`（並びは変えないが受け口の作り方）・`StartTalk { … }` を直に組むテスト（`areka-kanade` の 9 ファイル・`areka-ghost` 3・`areka-sakura` 2・`areka-runtime` 1・`areka` 1・`areka-seriko/tests/common`・`areka-kanade/tests/kanade/common`）。`StartTalk::new` を使う所は既定を持たせれば触らずに済む。

## 6. 規模と危うさ

- **規模: M**（7〜11 タスクの見立てどおり）。型 1・埋める所 1〜2・受け手の引数の形 1・共通の決まり 1・opener の乗り換え 1〜2・テスト 2〜3。既存の道（`origin` の通り道・`begin_talk`・`TempPath`・`log-capture-kit`）に乗るので新しい型の作りは少ない。
- **危うさ: Low〜Medium**。道は既にある。Medium の芽は ⑴ `begin_talk` の形の変更が受け口の一括の実装に触れること、⑵ `StartTalk` の直の組み立てのテストが多いこと（既定値で吸える）、⑶ §5 の 4（記録の段）と 5（ukadoc の食い違い）が振る舞いを変えうること。

## 7. 設計の段へ渡すこと

- 決めること: §4 の決め 1〜5、§5 の 1〜5 の扱い。
- Research Needed:
  - `\![open,file,相対]` が SSP で最上位も探すか（§5 の 5）。SSP の MCP（`sakurascript`）で ghost/master に無く最上位に在るファイルを開かせて確かめる（外の SSP を動かすので開発者の許しが要る）。
  - 共通の決まりを置く crate から、下流の受け手（文字の層・トレイ・音）が届くか（依存の向き・`structure.md` の各 crate の Dependencies）。
  - `begin_talk` の形を変えたとき、一括の実装に乗る受け口（seriko・文字・`MoveCueSink` ほか）が無改変のままか（既定の実装の引数だけ替わるので満たせる見込み）。
