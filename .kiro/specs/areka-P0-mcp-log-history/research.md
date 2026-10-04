# ギャップ分析: areka-P0-mcp-log-history

> 実測は **2026-10-04・本ブランチ**（`claude/areka-p0-mcp-log-history-f3750e`・先頭 `b4cebdc4`）。コードは「何の定義か」で指す。
> 本書は選択肢と材料を並べるもので、決定はしない。決めるのは要件ディスカッションと設計である。
> **要件を書いた時点では起動していなかった SSP が、本分析の時点では起動していた**（SSP の MCP に届いた）。§2 に `get_log` の実測を載せる。要件の暫定の裁定のうち 5 件（7・8・10・11・12）が実測と食い違う。

## 1. 要約

- **土台はある。** `get_log` の入口（定義・引数の型・橋・振り分け）は完成していて、本 spec が書くのはアプリ本体側のダミー 1 関数の中身と、履歴そのもの（新規）と、ログの出口の組み替え（`main.rs` の 1 か所）である。依存は足さずに組める見込みが高い。
- **SSP の実測が要件の暫定の裁定と 5 か所で食い違う。** `max_count=0` は 0 件、空の `log_type` は `NG:`、`log_type` は大文字小文字を区別しない、当たらない `ghost_name` は `NG:`、`[<種別>]` と `<名>` の欄は種別の語でも `areka` 相当でもない（§2）。
- **`ghost` の欄は今のコードに既にあり、意味がそろっていない。** フォルダ名・`-`・`Some("…")` の形の値が、status と error に入る行に載っている。要件 2.6 のままだと、これらが `<名>` に出る（§3.4）。
- **network 種別に入る行は、今は 3 行だけ。** 要件 2.3 が名指しする `areka_update::winhttp`・`areka_update::fetch` はファイルとしてはあるが、ログを 1 行も出していない（§3.3）。
- **要件の「標準エラー」は実物と違う。** 今の出口は既定の書き手のままで、既定は標準出力である（§3.1）。
- **テストの置き方に制約がある。** 履歴の層へ本物の出来事を流す単体テストは、見張り（捕捉先を直接差す呼び出しの禁止）に当たる。純粋な判断を切り出す形か、実プロセスの試験に寄せる形になる（§5）。
- 規模は **M**、リスクは **中**（§7）。

## 2. SSP の `get_log` の実測（2026-10-04・SSP 2.9.07・ゴースト「えも2DEBUG」）

SSP の MCP（ツール名 `mcp__ssp__get_log` など）へ読み取りだけを当てた。台本を流す・イベントを起こすなどの副作用のある呼び出しはしていない。[doc/ssp-mcp/survey.md](../../../doc/ssp-mcp/survey.md) は 2.9.05 の記録で、今回の SSP は status の 1 行目が「Starting up SSP mainsystem version 2.9.07」と名乗った。

### 2.1 測れたこと

| 呼び出し | 結果 | 要件との関係 |
|---|---|---|
| 引数なし（error 種別） | `(no log entries)` | 0 件の文言は要件 4.5 と一致 |
| `log_type=status` | `#4 2026/10/04 13:43 [STAT] STAT : Starting up SSP mainsystem version 2.9.07` など 10 行 | 書式は要件 4.2 と一致。**`[<種別>]` は `[STAT]`、`<名>` は `STAT`**（裁定 7・8 と違う） |
| `log_type=network` | `#1 2026/10/04 13:43 [Info] [SYSTEM] : Root certificates loaded. …`、`#18 … [Info] [SYSTEM] : [HTTP] Result=200 … Method=GET URL=https://…` など 11 行 | **`[<種別>]` は `[Info]`、`<名>` は `[SYSTEM]`**（裁定 7・8 と違う）。HTTP の取得 1 件ごと・回線の状態の変化が入る |
| `log_type=script`＋`ghost_name=えも2DEBUG` | `#14 2026/10/04 13:44 [Ghost:OnBoot] えも2DEBUG : \p[0]\s[1000]…\e` など | ゴースト自身のトークは **`[Ghost:<イベント名>]`**。`<名>` はゴーストの名前。survey の `[SSTP(Local,Auth)]` と合わせ、`[<種別>]` に種別の語以外が入る（裁定 8 の `label` の欄の必要を裏づける） |
| `log_type=update` | `(no log entries)` | 実例は**未測のまま**（SSP 本体の更新確認の HTTP は network へ入っていた） |
| 通し番号 | 1〜31 が status・network・script に混ざって振られている | 要件 1.2（全種別で 1 本）と一致 |
| `log_type=STATUS`・`Status` | status の行が返る | **大文字小文字を区別しない**（裁定 10・要件 3.2・3.6 の「大文字の `ERROR` は未知」と違う） |
| `log_type=""`（空の文字列） | `NG:Unknown log_type (error / script / network / update / status)` | **空は `NG:`**（裁定 10・要件 3.1 の「空は error」と違う。省略は error で一致） |
| `log_type=bogus` | 同じ `NG:` | 要件 3.3 と一致 |
| `max_count=0`（status） | `(no log entries)` | **0 は 0 件**（裁定 12・要件 5.7 の「0 は絞らない」と違う） |
| `max_count=-1`（network） | 全 11 行 | 負は絞らない（要件 5.7 の負の側と一致） |
| `since_id=-5`（network） | 全 11 行 | 要件 5.5 と一致 |
| `since_id=23`（network・#23 と #28 がある） | `#28` だけ | 「より大きい」で一致（要件 5.5） |
| `since_id=99999` | `(no log entries)` | 要件 5.8 と一致 |
| `since_id=5`＋`max_count=2`（status） | `#13`・`#15`（新しい方から 2 件・古い順で返る） | 要件 5.6・5.9 と一致 |
| `ghost_name=NoSuchGhost_xyz` | `NG:Cannot find active ghost from specified name` | **当たらない名前は `NG:`**（裁定 11・要件 5.4 の「0 件で答える」と違う） |
| `ghost_name=""`（空の文字列） | 同じ `NG:` | 要件 5.1（空は絞らない）と違う。ただし areka の入口は「空のまま届ける」と決めてあり、扱いは本 spec に任されている |
| `ghost_name=えも2DEBUG`＋`log_type=status`／`network` | `(no log entries)` | `<名>` が `STAT`・`[SYSTEM]` の行は、ゴーストの名前で絞ると出ない（要件 5.2 と同じ考え方） |
| `ghost_name=<ルートフォルダのフルパス・末尾に区切りあり>`＋`log_type=script` | そのゴーストの行が返る | フルパスの読み替えは要件 5.3 と一致 |

### 2.2 測れなかったこと（未測のまま残る）

- **error 種別の行の形**（`[<種別>]` と `<名>` に何が入るか）。記録が 0 件だった。出すには台本を strict で流すなどの副作用が要る。
- **update 種別の行の形**。0 件だった。
- **本文が複数行のときの継続行**（タブ始まり）。今回の記録に複数行の本文は無かった。
- **もう起動していないゴーストの名前で絞ったとき**（`NG:` か、残っている記録が返るか）。起動中のゴーストが 1 体だけだった。
- **応答の生の JSON での逆斜線の扱い**（survey §4-2 の欠陥が 2.9.07 で直ったか）。MCP クライアント越しには台本を正しく読めたが、生の応答は見ていない。
- **履歴の上限**（SSP が古い記録を何件で捨てるか）。

### 2.3 追加の実測（同日・開発者が SSP を起こした後・台本を流して測った）

§2.2 の未測のうち、台本を流せば測れるものを測った。流したのは `sakurascript`（`strict: true`）2 回と、応答の無い `raise_event` 1 回。生の JSON は `POST http://127.0.0.1:9801/api/mcp/v1` へ直に打って読んだ。

| 測ったこと | 結果 | 要件との関係 |
|---|---|---|
| ツール定義（`tools/list`） | 10 本とも [tools-list-ssp-2.9.05.json](../../../doc/ssp-mcp/tools-list-ssp-2.9.05.json) と 1 字も違わない | `mcp-tool-entrances` の逐語の定義は 2.9.07 でも正しい |
| **error 種別の行の形** | `#45 2026/10/04 13:54 [Error] えも2DEBUG : [GHOST/Script] surface not found (99999) at position 2 : \s[99999]テスト\![nosuchcommand,1]\` | **`[<種別>]` は `[Error]`、`<名>` はゴーストの名前**。本文は `[GHOST/Script] <誤りの種類> (<値>) at position <位置> : <その位置からの台本の 32 字ほど>`（`mcp-strict-errors` の材料） |
| strict で拾う誤り | `surface not found`・`unknown command`・`balloon not found` の 3 件が、1 つの台本から 1 件ずつ別の記録になる。`\i[88888]`（無いアニメーション）は記録されなかった | `mcp-strict-errors` の材料 |
| 誤りが出たときの status | error の記録 1 群につき、status に `[STAT] STAT : An error/warning occured. Open error log to show details.` が 1 件増える | areka は写さなくてよい（ログ窓への案内） |
| `sakurascript`（strict）の返事の `since_id` | error が 0 件のとき `since_id=0`、error の最後が #47・全体の最後が #49 のとき `since_id=47` | **「error 種別の最後の番号」**であり、全体の最後の番号ではない。要件 6.2 の「いま最後に振った通し番号」でも読む側の結果は同じ（どちらも、その後の error だけが返る） |
| script 種別の行（MCP の `sakurascript` で流した台本） | `#44 … [SSTP(Local,Auth)] えも2DEBUG : \0\s[99999]テスト…\e` | `[<種別>]` は送り手の経路。ゴースト自身のトークは `[Ghost:<イベント名>]`（§2.1） |
| 台本の中の生の改行 | `一行目⏎二行目⏎…` を送ると、記録は `一行目二行目…`（改行は消える） | SSP は台本の改行を捨ててから残す。**継続行（タブ始まり）は今回も実例なし** |
| 応答の生の JSON | `"text":"#50 … : \\0\\s[0]一行目二行目\\s[99998]三行目 \"quote\" \\\\ back\r\n#53 …"`（逆斜線・二重引用符は正しくエスケープ・記録の区切りは `\r\n`・末尾に区切りなし・`isError:false`） | **survey §4-2 の欠陥は 2.9.07 で直っている**。要件 4.1・4.6 と一致 |
| `log_type=" error "`（前後に空白） | `NG:Unknown log_type …` | 空白は削らない |
| `ghost_name=えも2DEBUG`＋`log_type=error` | そのゴーストの error の行が返る | 要件 5.2 と一致 |
| `raise_event`（ゴーストが答えないイベント） | `OK:the ghost returned no script.`・ログは増えない | 何も再生しなければ script に残らない |

**まだ未測**: update 種別の行の形、複数行の本文の継続行、もう起動していないゴーストの名前での絞り込み、履歴の上限。どれも SSP の更新・再読み込み・ゴーストの切替という重い操作が要るので、今回は行っていない。

### 2.4 SSP の `get_log` から読み取れる機能の一覧（areka が持つべきもの）

| # | 機能 | SSP の実測 | 今の要件 |
|---|---|---|---|
| 1 | 5 種別・全種別で 1 本の通し番号（起動ごと） | 一致 | 要件 1.2 |
| 2 | 1 行の書式 `#id 時刻 [語] 名 : 本文`・`\r\n` 区切り・古い順・0 件は `(no log entries)` | 一致 | 要件 4 |
| 3 | `[語]` と `名` は種別ではなく**記録ごと**に決まる（`[STAT] STAT`・`[Info] [SYSTEM]`・`[Error] <ゴースト名>`・`[Ghost:<イベント>] <ゴースト名>`・`[SSTP(Local,Auth)] <ゴースト名>`） | — | 裁定 7・8 と違う（議題） |
| 4 | `log_type`: 省略＝error・大文字小文字を区別しない・空と未知は `NG:`・空白は削らない | — | 要件 3.1・3.2 と違う（議題） |
| 5 | `ghost_name`: 名前かフルパス・起動中のゴーストに当たらなければ（空も）`NG:Cannot find active ghost from specified name` | — | 要件 5.1・5.4 と違う（議題） |
| 6 | `since_id`: より大きい・負は絞らない | 一致 | 要件 5.5 |
| 7 | `max_count`: 新しい方から N 件を古い順で・負は絞らない・**0 は 0 件** | 0 だけ違う | 要件 5.7（議題） |
| 8 | 応答は正しい JSON（逆斜線・引用符） | 一致（2.9.07） | 要件 4.6 |
| 9 | strict の誤りは error 種別へ、ゴーストの名前付きで 1 件ずつ | — | `mcp-strict-errors` の仕事・本 spec は受け皿（要件 6.1） |
| 10 | 再生した台本は script 種別へ、送り手の経路の語とゴーストの名前付きで | — | `mcp-kanade-tools` の仕事・本 spec は受け皿（要件 6.1） |
| 11 | strict の返事に「error の最後の番号」 | — | 要件 6.2（全体の最後の番号でも結果は同じ） |

## 3. 今のコード

### 3.1 ログの出口

- `crates/areka/src/main.rs` の `fn main()` の先頭の `tracing_subscriber::fmt().with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))).init()` が唯一の出口である。層を重ねてはいない（フィルタは出口全体に掛かる）。
- **書き手は指定していない。** `tracing_subscriber::fmt()` の既定の書き手は標準出力である。要件 1.8・Project Description・Out of scope の「標準エラー」は実物と違う（言い直すだけで、作業は変わらない）。
- **`log` クレートの行は、本番では今も tracing へ橋渡しされている見込み。** ワークスペースの `tracing-subscriber` は `version = "0.3", features = ["env-filter"]`（既定機能は切っていない）で、`Cargo.lock` の `tracing-subscriber 0.3.23` の依存に `tracing-log` がある。この機能が有効なとき `init()` は `log` の受け口も据える。`Cargo.lock` で `log` を引くのは `bevy_ecs`・`bevy_app`・`wgpu-types` など。`crates/wintf/src/ecs/window_proc/lifecycle.rs` のコメントは、`bevy_ecs` が `log::warn!` で「Could not despawn entity」を出すと書いている。brief の「`log::` は tracing の捕捉に届かない」はテストの捕捉（受け口を据えない）の話で、本番の出口とは別である。
- 橋渡しされた行は、出来事の target が `log` に、本来の target は `log.target` という欄に入る（`tracing-log` の作り）。warn 以上は出どころを問わず error 種別なので振り分けは困らないが、**本文に `log.target`・`log.module_path`・`log.file`・`log.line` の欄が並ぶ**（要件 4.3 のままだと）。`tracing-log` は `areka` の直接の依存ではないので、正規化の関数を呼ぶには `Cargo.toml` に 1 行要る。欄の名前（`log.` で始まる）で除くなら依存は要らない。
- `main.rs` は 957 行。`mod` の宣言 1 行と初期化の組み替え（数行の増）は 1,000 行に収まる。

### 3.2 `get_log` の入口とダミー

- プロトコル側 `crates/areka-mcp/src/tools/get_log.rs` の `Args`（`log_type`・`ghost_name`: `Option<String>`、`since_id`・`max_count`: `Option<i64>`）と `DEFINITION`（SSP の定義の逐語）は完成している。
- アプリ本体側 `crates/areka/src/mcp/get_log.rs` の `handle(_world: &mut World, _args: Args, reply: ReplyTo)` は `outcome::ng("not implemented yet")` を返すだけ。`get_log_tests.rs` の `answers_not_implemented_yet_with_an_empty_world` がその文言を期待している（書き換える対象）。
- `crates/areka/src/mcp/mod.rs` の `dispatch` は `ToolCall::GetLog(args) => get_log::handle(world, args, reply)` で、`ghost_name` を解決せず、起動中のゴースト（`active`）も渡さない。
- **フルパスの読み替え（要件 5.3）は既存の関数で足りる。** `crates/areka/src/mcp/resolve.rs` の `active(world)`・`resolve(active, ghost_name, omitted)`・`listed_value(ghost)` は `pub(crate)` で、`get_log.rs` は `mcp` の子モジュールなので `super::resolve` として呼べる（`mod.rs` を触らずに済む）。パスの照合 `same_path` 自体は非公開だが、`resolve` を通せば同じ照合になる。`resolve` は名前にもフルパスにも当たるので、「当たったら `listed_value` へ読み替え、当たらなければ渡された文字列のまま照合」と書ける。
- `crates/areka/src/mcp/mcp_tests.rs` の `get_log_and_seven_omitted_do_not_answer_with_a_resolve_failure` は、0 体の `get_log` が名前の解決の `NG:` を出さないことを固定している。**SSP に合わせて「当たらない `ghost_name` は `NG:`」へ改めるなら、このテストの前提（`get_log_call()` が渡す `ghost_name`）を確かめる要がある**（要件 8.4 は `get_log_tests.rs` 以外のテストを変えないと定める）。

### 3.3 種別の規則が名指しするモジュールと、出しているレベル

テスト用のファイルを除いて、ログのマクロの呼び出しを数えた。

| 種別（要件） | 名指しされた target | 実在 | info の行 | warn 以上の行（error 種別へ行く） |
|---|---|---|---|---|
| network（2.3） | `areka::install::fetch_url` | `crates/areka/src/install/fetch_url.rs` | 3（`install_fetch_swept`・`install_fetch_begin`・`install_fetch_done`） | 3 |
| network（2.3） | `areka_update::winhttp` | `crates/areka-update/src/winhttp.rs` | **0（ログのマクロが無い）** | 0 |
| network（2.3） | `areka_update::fetch` | `crates/areka-update/src/fetch.rs` | **0（同上）** | 0 |
| update（2.4） | `areka_update` | `crates/areka-update/src/lib.rs` | 2（`update started`・`update finished`） | 4 |
| update（2.4） | `areka::update` | `crates/areka/src/update/`（`mod.rs`・`desk.rs`・`procedure.rs`・`worker.rs`） | 約 20 | 約 30 |
| update（2.4） | `areka::install` | `crates/areka/src/install/`（`fetch_url.rs` を除く） | 約 15 | 約 20 |
| status（2.5） | `areka`（完全一致） | `crates/areka/src/main.rs` | 7 | 6 |
| status（2.5） | `areka::boot_resolve` | `crates/areka/src/boot_resolve.rs` | 9 | 9 |
| status（2.5） | `areka::ghost_session` | `crates/areka/src/ghost_session.rs` | 4 | 10 |
| status（2.5） | `areka::emo2_boot::ghost_switch` | `crates/areka/src/emo2_boot/ghost_switch.rs` | 13 | 約 30 |

- **network は今 3 行だけ**で、うち 1 行は「古い一時ファイルを消した」という掃除の行（`install_fetch_swept`）である。更新エンジンはファイル 1 件ごとの取得を info で出していない。SSP の network は HTTP の取得 1 件ごとの行と回線の状態の行である（§2.1）。
- **status の規則から外れている起動の行がある。** `crates/areka/src/boot_config.rs`（target は `areka::boot_config`）に info が 4 行（`session_mark_found` など）、`crates/areka/src/alert.rs` に info が 2 行ある。要件 2.5 の一覧に無いので履歴に残らない。
- **ゴースト自体の起動の節目は別のクレートにある。** `crates/areka-ghost/src/runtime.rs` は target `ghost-boot`・`ghost-shutdown` を明示して出している。SSP の status（`Loading SHELL elements`・`Loading SHIORI subsystem`・`Materialized.`）に近いのはこちらだが、要件 2.5 の一覧には無い。
- `crates/areka-update/src/lib.rs` の行は `target = %target.display()` という**欄**を持つ（tracing の target の指定ではなく、`target` という名前の欄）。本文に ` target=C:\…` と出る。振り分けには影響しない。
- 取り決めの名前 `areka::log::script`・`areka::log::error` は、今の明示の target（`kanade`・`areka_kanade::resource`・`areka_emo_atlas`・`areka_emo_compose`・`areka_ghost`・`ghost-boot`・`ghost-shutdown`・`ghost-sink`・`ghost-runtime`・`ghost-shiori-inproc`・`shiori-actor`・`areka_sylphya::reader`・`areka::persist::save`・`areka::persist::project`・`areka::persist::restore`・`areka::perf`・`wintf::thread_registry`）と重ならない。`crates/areka/src/` に `log` という名前のモジュールも無い。

### 3.4 `ghost`・`label` の欄は今の行にあるか

**`ghost` の欄は既にあり、意味がそろっていない。** 要件の「ゴーストの名前を載せる共通の欄は無い」は、「共通の意味の欄は無い」と読むのが正しい。同じ名前の欄は次の行にある。

| 行（定義の場所・`event` の値） | レベル | 入る種別 | `ghost` の値 |
|---|---|---|---|
| `boot_resolve.rs`・`session_mark_written` | info | status | フォルダ名（文字列） |
| `boot_resolve.rs`・`session_mark_write_degraded` | warn | error | フォルダ名 |
| `boot_resolve.rs`・`last_used_recorded` | info | status | フォルダ名、または **`-`** |
| `emo2_boot/ghost_switch.rs`・`ghost_switch_done`・`ghost_switch_booted` | info | status | `?` で出した `Option`（**`Some("emo2")` の形**） |
| `emo2_boot/ghost_switch.rs`・`session_mark_steady` | info | status | フォルダ名 |
| `emo2_boot/ghost_switch.rs`・`ghost_switch_target_fault`・`ghost_switch_default_fault`・`ghost_switch_boot_failed` | error | error | フォルダ名 |
| `install/names.rs`・`install_names_updated` | info | update | 直前に入れたゴーストの名前（無ければ欄なし） |
| `boot_config.rs`・`session_mark_found`／`emo2_boot/shell_balloon_resolve.rs`・`last_installed_shell_recorded` | info | （一覧の外＝残らない） | フォルダ名／`?` の形 |
| `crates/areka-mcp/src/tools/bridge.rs` の `call`・「MCP: ツールに答えた」 | debug | （残らない） | 一覧に出す値（空のこともある） |

- 要件 2.6 のままだと、これらの行の `<名>` は `emo2`・`-`・`Some("emo2")` になる。どれも `get_active_ghost_list` が返す値（descript の `name`）ではないので、`ghost_name` で絞っても当たらず、絞らないときは `<名>` の欄に不ぞろいな値が並ぶ。
- 出す側の行は 0 行変える約束（要件 8.2）なので、直すなら履歴の側の規則で避ける（§6 の議題 3）。
- **`label` の欄**は、info の行に 3 か所（`input_events/balloon_pressed.rs`・`input_events/shell_box_handler.rs` の `choice_selected`、`crates/areka-kanade/src/schedule/steady.rs` の `choice_accepted`）と debug の行に 1 か所（`crates/areka-parsers/src/charset/decode.rs`）あるが、どれも要件 2 の規則では残らない行である。今は衝突しない。将来これらの行が warn へ変わるか、status の一覧へ入ると `[<表示の語>]` に選択肢の文言が出る。

### 3.5 時刻

- `crates/areka/Cargo.toml` に日付・時刻のクレートは無い。`Cargo.lock` の `chrono` は `rmcp`・`schemars`（`areka-mcp` の側）が引いているだけで、`areka` の直接の依存ではない。
- 標準ライブラリに現地時刻は無い。ワークスペースの `windows` の機能に `Win32_System_SystemInformation` が宣言済み（根の `Cargo.toml`）で、`GetLocalTime` はこの機能にある。今のコードに `GetLocalTime`・`SYSTEMTIME` の利用は無い（初めての利用になる）。
- 要件 4.7 は「時刻は与えた値で書式だけを見る」と定めるので、時刻を読む口は差し替えられる形が要る。

### 3.6 テストの見張り

- `crates/log-capture-kit/tests/with_default_guard_test.rs` が走査する語は、捕捉先をスレッドへ差す 2 語と、プロセス全体へ据える 1 語（どれも開き括弧まで）。`SubscriberInitExt::init()`・`try_init()` の字面は当たらない。**本番の出口を `registry().with(…).init()` に組み替えても見張りには当たらない。**
- 逆に、**履歴の層へ本物の出来事を流すテスト**（自前の層を載せた受け口を据えて `info!` を打つ）は、この 3 語のどれかを使うことになり、例外表（4 件・件数も固定）に 1 行足さない限り赤になる。要件 8.5 は新設を禁じる。
- `log-capture-kit` の `capture`・`install_global_capture_all` が返す `CapturedEvent` は `level`・`target`・`fields`（`message` を含む・訪問順）を持つ。履歴の「振り分け」と「本文の組み立て」を `(レベル, target, 欄の並び)` を受ける純粋な関数にしておけば、`CapturedEvent` をそのまま食わせて試せる。
- 同じ見張りの 4 つ目（`env-filter` を宣言してよいのは wintf だけ）は `log-capture-kit` の機能の話で、本 spec には関係しない。
- 実ソケットの試験（要件 4.7 の後半）: `crates/areka-mcp/src/tools/tools_socket_tests.rs` はあるが、`crates/areka-mcp/src/**` は触らない約束（要件 8.3）。`crates/areka/` の側に MCP の実ソケットの試験は無い（`crates/areka/tests/` は `smoke_boot_loop_exit.rs`・`emo2_real_run.rs` の 2 本で、MCP には触れていない）。`areka_mcp` の `testkit` は非公開のモジュールである。

## 4. 要件ごとの対応（Missing＝無い／Constraint＝制約／Unknown＝未確認）

| 要件 | 今ある資産 | 足りないもの |
|---|---|---|
| 1 履歴（番号・上限・切り詰め・並行） | なし | **Missing**: 履歴の入れ物（種別ごとの環状の列・全体で 1 本の番号・排他）、層。**Unknown**: 取り決めの target を debug・trace でも拾うと、出口全体の最大レベルの見立てが trace まで開き、`log` の橋の手前の足切りが効かなくなる（§5 の研究項目） |
| 1.7・1.8 `RUST_LOG` と独立・出力は同じ | 出口全体に掛かる `EnvFilter` | **Missing**: フィルタを出力の層だけに掛ける組み替え。**Constraint**: 「標準エラー」は実物では標準出力 |
| 2 振り分け | 名指しのモジュールは全部実在 | **Missing**: 規則の表と照合。**Constraint**: network の 2 つの target は今ログを出していない（§3.3）。`ghost` の欄の衝突（§3.4）。`log` の橋の行の欄（§3.1） |
| 2.12 名指しの実在をテストで判定 | `log-capture-kit` の走査の道具（そのクレートのテスト専用） | **Missing**: 出す側を触らずに実在を判定する方法（ファイルの有無を見る・`mod` の宣言を読む、など） |
| 3 `log_type` | 入口は完成 | **Missing**: 中身。**Constraint**: SSP は空を `NG:`・大文字小文字を区別しない（§2） |
| 4 書式 | `outcome`（`OK:`／`NG:`／素の本文）の作り口 | **Missing**: 1 行の組み立て・改行の置き換え・時刻の口。実ソケットの試験の置き場（§3.6） |
| 5 絞り込み | `resolve.rs` の `active`・`resolve`・`listed_value` | **Missing**: 絞り込み。**Constraint**: SSP は `max_count=0` を 0 件・当たらない名前を `NG:`（§2） |
| 6 後続のための口 | なし | **Missing**: 「最後に振った番号」を読む口（置き場は §5） |
| 7 文書 | `doc/ssp-mcp/` | **Missing**: 取り決めの文書、文書と実装の一致のテスト、`.kiro/steering/logging.md` の例の更新 |
| 8 触る範囲 | — | **Constraint**: テストの見張り（§3.6）。SSP に合わせる向きによっては `mcp_tests.rs` に触れる要が出る（§3.2） |

## 5. 実装の選択肢

### 5.1 ログの出口の組み替え

- **案 A: 層を重ね、フィルタを出力の層だけに掛ける。** `registry().with(fmt::layer().with_filter(EnvFilter…)).with(履歴の層).init()`。履歴は `RUST_LOG` と独立（要件 1.7）、出力は今と同じ（1.8）。依存は足さない（`registry`・層ごとのフィルタは既定機能）。
  - 良い点: 要件 1.7・1.8 をそのまま満たす。`main.rs` は数行。
  - 気を付ける点: 履歴の層が「どのレベルでも拾う target」を持つと、出口全体の最大レベルの見立てが広がる。履歴の層にも自前のフィルタ（target とレベルで「関心なし」を返す）を付ければ、関心のない呼び出し口は今と同じく切られる見込み。**設計で実物で確かめる**（`RUST_LOG` 未設定で debug の行の費用が増えないこと・`log` の橋の最大レベル）。
- **案 B: 出口全体のフィルタは今のまま、その下に履歴の層を足す。** 組み替えは最小。
  - 悪い点: `RUST_LOG=warn` で status が消え、`RUST_LOG=areka_mcp=debug` で error が消える。要件 1.7 を満たさない（裁定 6 を覆す場合だけの案）。
- **案 C: 取り決めの target は info 以上で出す約束にする（案 A の変形）。** 履歴の層のフィルタを「info 以上」に固定でき、最大レベルの見立ては今（`info`）と変わらない。
  - 悪い点: 要件 2.1・2.2 の「レベルを問わない」を狭める。`RUST_LOG=warn` の出力に script の行（info）は出ないので実害は小さい。

### 5.2 履歴の置き場

- **案 A: プロセスに 1 つの置き場（静的な `OnceLock` と排他）。** 層と `get_log` の `handle` と「最後の番号」の口が同じ置き場を見る。`main.rs` は層を作って載せるだけ。
  - 良い点: `handle` の引数（World）を増やさずに読める。`mcp/mod.rs` を触らない。後続の spec は関数 1 つを呼ぶだけ。
  - 悪い点: テストが置き場を共有する。純粋な入れ物（型）をテストし、静的な置き場は薄い包みにとどめる形になる。
- **案 B: 共有の持ち手（`Arc`）を層と World の両方に持たせる。** `main.rs` で作り、層へ渡し、World へ資源として置く。
  - 良い点: テストごとに別の履歴を作れる。
  - 悪い点: World へ置く 1 行が `main.rs` か `mcp::install` に要る（`mcp/mod.rs` は触らない約束なので `main.rs` 側＝要件 8.1 の「初期化とモジュールの宣言だけ」を少し越える）。World の無い所（kanade のスレッドなど）から「最後の番号」を読めない。
- **案 C（併用）: 入れ物は純粋な型、本番はそれを静的な置き場に 1 つ持つ。** テストは型を直に作る。案 A の読みやすさと案 B の試しやすさを両方取る。

### 5.3 ファイルの置き方

- **案 A: `crates/areka/src/log_history.rs`（＋兄弟のテスト）を新設し、`main.rs` に `mod` を 1 行。** `get_log.rs` は絞り込みと書式を持つ。履歴は MCP から独立した部品になる（後続の spec も `crate::log_history` を呼ぶ）。
- **案 B: `crates/areka/src/mcp/get_log.rs` の子モジュールにする。** `main.rs` の `mod` は不要だが、`main.rs` の初期化から `mcp::get_log::…` の層を呼ぶには `mod.rs` の `mod get_log;` を公開する要があり、`mod.rs` を触らない約束に反する。**実質は案 A だけ**。
- 振り分け・入れ物・層・書式を 1 ファイルに詰めると長くなる。`log_history/`（`mod.rs`・`classify.rs`・`store.rs`・`layer.rs`）のように分ける形もある（規模しだい）。

### 5.4 テストの組み方（見張りとの折り合い）

- **案 A: 判断を純粋な関数に寄せる。** 「レベルと target から種別を決める」「欄の並びから `<名>`・表示の語・本文を作る」「入れ物へ積む・捨てる」「絞って 1 つの文字列にする」を、tracing の型に触れない関数にする。tracing との継ぎ目（出来事から欄を取り出す訪問）は薄くし、`log-capture-kit` の `capture` が返す `CapturedEvent`（レベル・target・欄の並び）を同じ関数へ食わせて「本物のマクロが出す形」で固定する。見張りに当たらない。
  - 残る穴: 「層が本当に受け口に載っていて、出来事が履歴へ届く」こと自体は単体では固定されない。
- **案 B: 実プロセスの試験で継ぎ目を固定する。** `crates/areka/tests/` に、areka を起こして実ソケットで `get_log` を呼ぶ試験を足す（要件 4.7 の実ソケットの 1 本と、要件 6.1 の「取り決めの target で出した行が入る」を兼ねる）。既存の `smoke_boot_loop_exit.rs` が実プロセスの起動の手本。
  - 気を付ける点: ポートの取り合い（SSP が 9801・9821 を握る机では隣へ逃げる）、所要時間。
- **案 C: 見張りの例外表に 1 件足す。** 層を載せた受け口をテストで直に差す。要件 8.5 に反するので、要件を改める場合だけ。
- 案 A と案 B の併用が、要件 1.11・2.11・4.7・6.4 を今の約束の中で満たす組み合わせに見える。

### 5.5 `log` の橋の行（要件 2.10）

- **案 A: 欄の名前が `log.` で始まるものを本文から除く**（依存を足さない）。
- **案 B: `tracing-log` を `areka` の依存に足し、正規化の関数で本来の target・欄へ戻す**（`Cargo.toml` を 1 行変える。要件の Out of scope「`Cargo.toml` の変更」に理由を書く要）。
- どちらでも、warn 以上は error 種別に入る。info の `log` の行は target が `log` なので、どの種別にも当たらない（案 B なら本来の target で照合できるが、外のライブラリの info を残す規則は要件に無い）。

## 6. 要件ディスカッションへ渡す議題（答えで作業が変わるもの）

> 要件ディスカッションでの仕分け（2026-10-04）: 1〜5・7 は開発者と話す。6 は要件 2.3 に事実を書き足して済ませた（受け皿のまま）。8 は要件の文言を直した。**9・10 は設計の段で決める**（あわせて §5.2 履歴の置き場・§5.3 ファイルの置き方・§5.5 `log` の橋の行・§3.5 時刻の口も設計で決める）。

1. **`max_count=0` の意味。** SSP は 0 件を返す。要件 5.7（0 は絞らない）は SSP と逆。SSP に合わせるか、「AI が既定のつもりで渡す 0」を守るか。
2. **`log_type` の空と大文字小文字。** SSP は空を `NG:`、`STATUS`・`Status` を受ける。要件 3.1・3.2・3.6 は逆（空は error・大文字は未知）。
3. **当たらない `ghost_name`・空の `ghost_name`。** SSP はどちらも `NG:Cannot find active ghost from specified name`。要件 5.1・5.4 は「空は絞らない」「当たらなければ 0 件」。SSP に合わせると、もう起動していないゴーストの記録を名前で読めなくなり、`mcp_tests.rs` の前提にも触れうる。合わせないなら、SSP 向けの呼び方が areka では違う答えになる。
4. **`<名>` と `[<種別>]` の既定の語。** SSP は status が `[STAT] STAT`、network が `[Info] [SYSTEM]`、ゴースト自身のトークが `[Ghost:<イベント名>] <ゴースト名>`。要件（裁定 7・8）は `[status] areka`。SSP の語へ寄せるか（種別ごとに既定の語と名を持つ）、areka の語でよいか。error・update は未測。
5. **今ある `ghost` の欄との衝突。** フォルダ名・`-`・`Some("…")` が `<名>` に出る（§3.4）。考えられる向き: (a) 取り決めの欄を別の名前にする（例 `log.ghost`・`ghost_name`。裁定 9 の「名前は設計で変えてよい」の範囲か、要件 2.6 の改訂か）、(b) `ghost` の欄を `<名>` に使うのは取り決めの target（`areka::log::script`・`areka::log::error`）の行だけにする、(c) そのまま受け入れて、行の持ち主の spec が直すのを待つ。
6. **network 種別の中身。** 今入るのは 3 行（うち 1 行は掃除）。SSP の network は HTTP の取得 1 件ごと。出す側を触らない約束の中では増やせない。「受け皿だけ用意し、中身は行の持ち主の spec が増やす」と書くか、掃除の行を update へ回すか。
7. **status に入れる起動の行の範囲。** `areka::boot_config`・`areka::alert` の info と、`areka-ghost` の `ghost-boot`・`ghost-shutdown`（SSP の status に近い）を一覧に足すか。足すと、ゴーストの読み込みの節目が AI から見える。
8. **「標準エラー」の言い直し。** 実物は標準出力。要件の文言だけの話で、作業は変わらない（議題にせず直すだけでもよい）。
9. **取り決めの target を debug・trace でも拾うか（要件 2.1・2.2 の「レベルを問わない」）。** 拾うと出口全体の最大レベルの見立てが広がりうる（§5.1 の案 C は「info 以上で出す約束」に狭める）。設計で費用を実測してから決めてもよい。
10. **層の継ぎ目のテストをどこで固定するか。** 純粋な関数＋実プロセスの試験（§5.4 の案 A＋B）か、見張りの例外表に足すか（要件 8.5 の改訂）。実プロセスの試験を足すなら、要件 4.7 の「実ソケットで 1 本」の置き場もそこになる。

## 7. 規模とリスク

- **規模: M（3〜7 日・8〜12 タスク）。** 新規は履歴の部品（振り分け・入れ物・層）と `get_log` の中身（絞り込み・書式）と文書。既存の変更は `main.rs` の数行と `get_log.rs`・`get_log_tests.rs`・`logging.md`。依存は足さずに済む見込み。
- **リスク: 中。** 仕組み（層を重ねる・層ごとのフィルタ）は `tracing-subscriber` の既定機能で、道筋は見えている。中にしたのは、(1) SSP の実測との食い違いが要件を 5 か所動かしうること、(2) 層ごとのフィルタへ組み替えたときの費用と `log` の橋の振る舞いを実物で確かめる要があること、(3) テストの見張りのために継ぎ目の試験を実プロセスへ寄せる要があること。

## 8. 設計へ持ち越す研究項目

- 層ごとのフィルタへ組み替えた後、`RUST_LOG` 未設定で debug・trace の行の費用が今と変わらないこと（関心なしの呼び出し口が切られること）と、`log` の橋の最大レベルがどうなるかを実物で確かめる。
- `bevy_ecs` の `log::warn!` が本番の出口と履歴の層の両方へ届くこと・届いたときの本文の形（`log.` で始まる欄）を実物で確かめる（要件 2.10）。
- 出力の層を `fmt::layer()` へ替えたとき、行の書式（時刻・レベル・target・欄・色）が `tracing_subscriber::fmt()` の今の出力と 1 字も変わらないこと（要件 1.8）。
- 時刻の口: `GetLocalTime` を使う場合の差し替えの形（テストは与えた値）。
- 名指しのモジュールの実在をテストで判定する方法（要件 2.12）と、文書と実装の一致の判定（要件 7.3。例: 文書の表を読み込んで規則の表と突き合わせる）。
- SSP の未測の点（§2.2）のうち、要件を動かすもの（error 種別の行の形・もう起動していないゴーストの名前での絞り込み・継続行）を、副作用を許せる場で測る。
- 排他の中でログを出さないこと（履歴の層の中で tracing のマクロを呼ぶと自分へ戻る）と、毒された排他でもパニックしないこと（要件 1.10）。

---

# 設計の段の調査と決定（2026-10-04）

> ここから下は設計の段で足した。調べ方は「既存の仕組みへの足し込み」向けの軽い調査（統合点・依存・危険の確認）で、外部の新しい依存は 0 件である。
> 実験は、ワークツリーの `target\` の下に作った使い捨ての小さなクレート（`tracing 0.1.44`・`tracing-subscriber 0.3.23`〔`env-filter`〕・`tracing-log 0.2.0`・`log 0.4`＝本体の `Cargo.lock` と同じ版・release ビルド）で行い、記録した後に消した。リポジトリには残していない。

## 9. 設計の段の実測

### 9.1 出口を組み替えても標準出力は 1 字も変わらない（要件 1.8・設計判断 h）

- **比べたもの**: 今の形 `tracing_subscriber::fmt().with_env_filter(EnvFilter…).init()` と、組み替えた形 `tracing_subscriber::registry().with(tracing_subscriber::fmt::layer().with_filter(EnvFilter…)).with(履歴の層.with_filter(filter_fn(…))).init()`。
- **流した出来事**: 欄つきの info（文字列の欄・`?` の欄・`%` の欄・数の欄・逆斜線と二重引用符と日本語を含むメッセージ）、warn、error、debug、取り決めの target の debug／trace／info、`log::warn!`・`log::info!`・`log::debug!`。
- **結果**: `RUST_LOG` が未設定・`warn`・`<モジュール>=debug` の 3 通りで、標準出力は行数も中身も、行頭の時刻を除いて完全に一致した（7 行・3 行・3 行）。`NO_COLOR=1`・ファイルへの出力で比べた。
- **理由（ソース）**: `tracing_subscriber::fmt()` が作る受け口は「`EnvFilter` の下に `fmt::Layer` の既定値、その下に `Registry`」で、`fmt::layer()` は同じ `fmt::Layer` の既定値を返す。違いはフィルタが全体に掛かるか層に掛かるかだけである。`EnvFilter` は層ごとのフィルタとしても使える（`tracing-subscriber 0.3.23` の `filter/env/mod.rs` に `layer::Filter` の実装がある）。
- **`init()` は見張りに当たらない**: `SubscriberInitExt::init()` の字面は、`crates/log-capture-kit/tests/with_default_guard_test.rs` の走査語（開き括弧までの 3 語）のどれでもない。

### 9.2 取り決めの target をどのレベルでも拾うときの費用（設計判断 a）

`tracing-subscriber 0.3.23` の `SubscriberInitExt::try_init`（`util.rs`）は、受け口を据えた後に `LogTracer` の最大レベルを `LevelFilter::current()`（受け口全体の最大レベルの見立て）に合わせる。履歴の層のフィルタの見立てを変えて測った。

| 構成 | `RUST_LOG` | tracing の最大レベル | `log` の最大レベル | 関心のない `debug!` | 関心のない `trace!` | どの層も要らない info | `log::debug!` |
|---|---|---|---|---|---|---|---|
| 今の形 | 未設定相当 | INFO | Info | 0.74 ns | 0.92 ns | 1.72 ns | 1.39 ns |
| 組み替え・見立て `TRACE` | 未設定相当 | **TRACE** | **Trace** | 2.41 ns | 2.13 ns | 2.01 ns | **137 ns** |
| 組み替え・見立て `INFO` | 未設定相当 | INFO | Info | 0.70 ns | 0.74 ns | 1.33 ns | 0.68 ns |
| 今の形 | `warn` | WARN | Warn | 0.73 ns | 0.72 ns | 0.64 ns | 0.73 ns |
| 組み替え・見立て `TRACE` | `warn` | **TRACE** | **Trace** | 1.65 ns | 1.51 ns | 1.89 ns | **116 ns** |
| 組み替え・見立て `INFO` | `warn` | INFO | Info | 0.62 ns | 0.67 ns | 1.32 ns | 0.57 ns |

（1 回あたり。tracing は 5,000 万回、`log` は 500 万回の平均。「未設定相当」は `info,<測定用の target>=warn`。）

- **tracing のマクロ**: 見立てを `TRACE` にしても、関心のない呼び出し口は「関心なし」が覚えられ、増えるのは 1 回あたり 1〜1.7 ns。
- **`log` クレート**: 見立てが `TRACE` だと `log::debug!`／`log::trace!` が受け口まで届き、1 件 約 120〜140 ns で捨てられる。
- **`log` を使う依存**: `cargo tree -p areka -i log` で `bevy_ecs 0.19.1`・`bevy_app 0.19.1`・`tracing-log`（`wgpu-types`・`iana-time-zone` は `Cargo.lock` にはあるが areka の木には出ない）。`bevy_ecs` の `debug!` は 6 件（段階実行 `schedule/stepping.rs` の 4 件と、差し替えの 2 件）、`bevy_app` は `debug!` 2 件（プラグインを足したとき）と `trace!` 4 件（スレッドプールを作るとき）で、**毎フレーム通る `log` の `debug!`／`trace!` は 0 件**。
- **届くこと**: 見立てが `TRACE` のとき、`RUST_LOG` が未設定でも `warn` でも、`debug!(target: "areka::log::script", …)` と `trace!(target: "areka::log::error", …)` は履歴の層へ届いた（標準出力には出ない）。見立てが `INFO` のときは届かない。

### 9.3 `log` クレートの行の届き方（要件 2.10・設計判断 f）

- `log::warn!("log crate warn {}", 1)` は、標準出力には ` WARN <クレート名>: log crate warn 1`（本来の target・`log.` の欄なし）と出る。履歴の層の `on_event` には、**target が `log`**、欄が `message`・`log.target="<クレート名>"`・`log.module_path="…"`・`log.file="src\\main.rs"`・`log.line=94` の形で届いた。
- フィルタ（`enabled`）には本来の target とレベルで問い合わせが来る（`tracing-log 0.2.0` の `LogTracer::enabled` が `metadata.as_trace()` で尋ねる）。よって、フィルタは本来の target で、`on_event` は target `log` で判定することになる。warn 以上はどちらでも error 種別なので食い違わない。
- 見立てが `INFO` のとき、`log::info!` は履歴の層に届かなかった（フィルタが「要らない」と答える）。

### 9.4 本文の形は標準出力の行と同じになる

- 履歴の層で「`message` を先頭、残りの欄を ` 名前={:?}`（文字列の欄は引用符つき、`%`・`?` の欄は整形済みの文）」で組んだ本文は、標準出力の行の `<target>: ` より後ろと一致した。例: `起動 \0\s[0] "q" event="boot" folder="C:\\a\\b" name=x y n=3`。
- 台本をメッセージとして出すと、逆斜線は 1 つのまま残る（`\0\s[0]こんにちは\n\![raise,OnTest]`）。欄として文字列で出すと引用符つき・逆斜線は 2 つになる。**台本はメッセージで出す**のが取り決めになる。

### 9.5 既存のコードで確かめたこと

- `crates/areka/src/mcp/resolve.rs` の `resolve` は、`ghost_name` が空の文字列だと「省略」と同じ扱いで `NOT_ACTIVE` を返す。SSP は空にも `Cannot find active ghost from specified name` を返すので、`get_log` は空を `resolve` へ渡さずに自分で `CANNOT_FIND` を返す。空でない値なら、`resolve` の失敗は起動中が 0 体のときも含めて `CANNOT_FIND` になる。
- `crates/areka/src/mcp/mcp_tests.rs` の `get_log_call` は 4 つの引数とも `None` で、`get_log_and_seven_omitted_do_not_answer_with_a_resolve_failure` は 0 体でそれを呼ぶ。`ghost_name` が `None` のとき名前の解決をしない設計なので、このテストは変えずに緑のままである（§3.2 の懸念は当たらない）。
- 「起動するゴーストを決めました」の行は `crates/areka/src/boot_config.rs`（`event = "ghost_resolved"`・`route = ?…`・`dir = %….display()`）にあり、target は `areka::boot_config`＝status。逆斜線入りのパスと二重引用符を含むので、実プロセスの試験の「JSON を通しても字が変わらない」の材料になる。「本物のゴースト窓を開きました」は `crates/areka/src/ghost_session.rs`（target `areka::ghost_session`＝status）。
- `crates/areka-mcp/src/server.rs` の `start` は待受の番地を `info!` で出し、`AREKA_MCP_PORT` に番号を 1 つ指定すればその番号だけを使う。`tools/call` は初期化の往復なしの `POST` 1 本で通る（`crates/areka-mcp/src/tools/tools_socket_tests.rs` の旧式の送り方）。`areka-mcp` の `testkit` は非公開なので、実プロセスの試験は手書きの HTTP/1.1 を自前で持つ。
- `GetLocalTime` は `windows 0.62.2` の `Win32::System::SystemInformation` にあり、この機能は根の `Cargo.toml` に宣言済み。`crates/areka` には `unsafe` を禁じる属性は無く、既に `alert.rs`・`readme.rs` などが `unsafe` を持つ。
- `crates/areka-ghost/src/runtime.rs` には `target: "ghost-boot"`／`target: "ghost-shutdown"` の字面が 18 か所ある。`RULES` が名指しするモジュールのファイル（`crates/areka/src/` の 7 つと `crates/areka-update/src/` の 3 つ）は全部実在する。

## 10. 設計判断の記録

### 判断 a: 取り決めの target はどのレベルでも拾う
- **選択肢**: (1) どのレベルでも拾う（見立て `TRACE`） (2)「info 以上で出す」約束に狭める（見立て `INFO`）
- **決定**: (1)。
- **理由**: 要件 2.1・2.2 の「レベルを問わない」を字のとおりに満たせる。出す側が「標準出力には出さずに履歴へだけ残す」（`debug!` で出す）を選べる。費用は §9.2 のとおり小さい。
- **引き換え**: `log` クレートの `debug!`／`trace!` が受け口まで届く（今の木では毎フレームの行が 0 件）。debug・trace で出した行が本物の受け口を通ることは、本 spec の常時テストでは固定されず、§9.2 の実験が証跡になる。
- **見直すとき**: `log` で大量に出す依存を足すとき。(2) へ狭めれば見立ては `INFO` に戻る（要件 2.1・2.2 と取り決めの文書を改める）。

### 判断 b: 継ぎ目のテストは「純粋な関数＋実プロセス 1 本」
- **選択肢**: (1) 純粋な関数に `CapturedEvent` を食わせる＋実プロセス・実ソケットの試験 (2) 見張りの例外表に 1 件足して、層を載せた受け口をテストで差す
- **決定**: (1)。例外表は 4 件のまま。
- **理由**: (2) は要件 8.5 に反する。実プロセスの試験は要件 4.7 の「実ソケットで 1 本」と兼ねられ、`RUST_LOG` と独立であること（要件 1.7）も同じ 1 本で固定できる（`RUST_LOG=warn,areka::boot_config=info` で起こし、標準出力に無い info が履歴にあることを見る）。
- **引き換え**: 実プロセスの試験は i686 の helper とモニタを前提にする（`smoke_boot_loop_exit.rs` と同じ）。helper を揃える手順の写しが 60 行ほど増える（既存のテストを触らない約束のため）。

### 判断 c: 置き場は「純粋な型＋静的な 1 つ」
- **決定**: `History`（純粋）と `static` の `Mutex<History>`。`Mutex::new`・`VecDeque::new` は `const` なので `OnceLock` は要らない。
- **理由**: `handle` の引数も `mcp/mod.rs` も変えずに読める。World の無いスレッドからも `last_id` を読める。テストは `History` を手元で作る。

### 判断 d: 1 ファイル
- **決定**: `crates/areka/src/log_history.rs`（450 行前後の見込み）＋兄弟のテスト 2 本。出口の据え付け `init` もここに置く（`main.rs` は 1 行の呼び出しになり、行数は 957 から減る）。
- **見直すとき**: 900 行を超えたら、ファサード形式で `log_history/` へ分ける。

### 判断 e: 時刻は `GetLocalTime`・差し替えは引数
- **決定**: `History::push(at, draft)` が時刻を引数で受ける。本番は層が `local_now()` を渡し、テストは値を渡す。時計の trait は作らない（実装が 1 つしか無い）。

### 判断 f: `log.` で始まる欄は本文から除く
- **決定**: `tracing-log` を依存に足さず、欄の名前で除く。標準出力の側も同じ 4 欄を出さないので、見え方がそろう。

### 判断 g: 規則の表 1 つを 3 つのテストが読む
- **決定**: `RULES`（13 行）を `classify`・実在のテスト（要件 2.12）・文書との一致のテスト（要件 7.3）が読む。実在は target からソースの場所を導いて判定し、導けない target は赤にする。

### 判断 h: 出口の形
- **決定**: §9.1 の組み替えた形。証跡は §9.1。

### まとめ直し（一般化・自作か採用か・削ったもの）
- **一般化**: 「どう出せばどの種別になるか」は `RULES` と取り決めの target の 1 つの規則に寄せた。network・update・status・後続の script・error は同じ道を通る（後続の spec は規則を足さない）。
- **採用**: 層を重ねる仕組み・層ごとのフィルタ・`log` の橋は `tracing-subscriber` の既定機能をそのまま使う。自作は環状の列と振り分けの表だけ。
- **削ったもの**: 時計の trait、World に置く資源、`log_history/` のディレクトリ分け、`tracing-log`・`serde_json`（テスト用）の依存の追加、SSP の「誤りが出たら status に案内を足す」振る舞い。

## 11. 危険と手当て

- **`RUST_LOG` の最大レベルの見立てが常に `TRACE` になる** — 費用は §9.2。`perf_thread_report` のように「`RUST_LOG` に特定の指定があるか」を自分で読む処理には影響しない（見立てではなく環境変数を読んでいる）。
- **排他の中でログを出すと固まる** — 排他を握る箇所を 3 か所に限り、その中で tracing のマクロを呼ばない。`answer` も呼ばない。
- **実プロセスの試験のポートの取り合い** — 空きの番号を OS から得てから渡す。既定の 9801・9821 は使わない。
- **SSP の未測の点**（update 種別の行の形・継続行・もう起動していないゴーストの名前・上限）— 要件の暫定の裁定のまま進む。設計では動かしていない。

