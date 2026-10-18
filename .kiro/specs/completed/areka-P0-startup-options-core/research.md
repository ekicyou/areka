# ギャップ分析: areka-P0-startup-options-core

- 調べた日: 2026-10-18（ワークツリー `claude/areka-p0-startup-options-d56a01`・`f50a6ebc`）
- 調べたもの: `requirements.md`（確定版）・`brief.md`・steering（`roadmap.md` のウェーブ C9 の行と起動オプションの列。`product.md`・`tech.md`・`structure.md` は 1,000 行の上限・兄弟のテスト・依存を足さない、の決まりだけを当てた）・正典 `ssphelp:option`（ukadoc MCP で全文を読み直した）・今のコード
- この文書は判断の材料を並べるだけで、決めるのは要件の議論と設計の段。

## 1. まとめ

- 引数を読む所は `main.rs` の 1 行（`std::env::args()`）と `boot_config::resolve_boot_from` の `args.get(1)`・`args.get(2)` だけ。起動オプションを読む口は 1 つも無い。ほかの部品は、ほぼそのまま使い回せる: 名前の引き当て（`ghost_switch::resolve_switch_target`）・ゴーストの選び方の段（`boot_resolve::resolve_ghost`）・置き場の印（`RootMark`）・MCP の待受（`areka_mcp::start(&[u16])`）・SHIORI の差し替え口（`ShioriWiring::Custom`）・OS で開く口（`readme::os_port::WindowsShell`）。
- 足りないもの: 引数の読み方そのもの（新しい `cli/`）・一時の置き場の印と、それが効く 5 か所・`--random` と `nobootcheck` を選び方へ通す入力・何を聞かれても中身なしで答える SHIORI・`--sstp-listen` の読み方・`--help` で終わる道。
- 案は 3 つ（下の 4 節）。推しは案 C（新しい `cli/` に読み方を置き、起動の解決・置き場・MCP・SHIORI の結線は今ある所へ値を渡す形で足す）。
- 大きい論点は 2 つ。⑴ SSP の「親のフォルダ」は**ゴーストのフォルダを並べたフォルダ**（SSP の読み込みフォルダ）で、areka の「置き場」（直下に `ghost\` と `balloon\` を持つフォルダ）とは 1 段ずれる。要件 3.2・3.3 の「親を置き場に足す」をそのまま実装すると `親\ghost\` を探して見つからない。⑵ 末尾に足した一時の置き場のゴーストは、上の置き場に同じフォルダ名のゴーストがあると目録で隠れる（要件 3.8 は起動だけなら満たせるが、その後のメニュー・記憶・切替が別のゴーストを指す）。
- `main.rs` はちょうど 1,000 行。`mod cli;` の 1 行も足せないので、先に塊を外へ出すタスクが要る。並走の約束（実走の 3 本・`runtime.rs`・`config.rs`）は、案 C なら触らずに済む。

## 2. 今の姿

### 2.1 起動の道（引数まわり）

| 所 | 今していること |
|---|---|
| `crates/areka/src/main.rs:171` | `std::env::args().collect()` → `resolve_boot(&args)`。UTF-8 でない引数は `args()` が panic する（今も同じ） |
| `crates/areka/src/boot_config.rs:220` `resolve_boot_from` | `args.get(1)` をゴースト、`args.get(2)` をバルーンとして読む。ゴーストが渡されたら目録・起動中の印・最後のゴーストの記憶を読まず、写しもしない（`route == Argv`） |
| `crates/areka/src/boot_resolve.rs:157` `resolve_ghost` | 段 1 argv → 段 2 記憶 → 0 体 → 段 3 唯一 → 段 4 既定 → 段 5 無作為。純粋・`pick` を注入（`pick` は「2 以上 → 添字」が前提） |
| `boot_resolve.rs:415` `read_session_mark` | 起動中の印を読む。在れば記憶を読まずに段を進め、値は `OnBoot` の由来（`BootOrigin::Halted`）にもなる（`main.rs:460`） |
| `main.rs:229` `argv_session` | `route == Argv` のときだけ真。真なら印を読まず・書かず・消さない。最後のゴーストの記憶も書かない（`boot_resolve.rs:576`） |
| 実走の試験 | `smoke_boot_loop_exit.rs:311` が「起動するゴーストを決めました」の行の `route=Argv` を照合している。位置の引数の経路の名前は変えられない |

### 2.2 使い回せる部品

| 部品 | 場所 | 使い道 |
|---|---|---|
| 名前の引き当て | `emo2_boot/ghost_switch.rs:176` `resolve_switch_target(entries, &GhostSpec::Name(..))` | 要件 4.2（`name` を先・無ければフォルダ名）。そのまま呼べる |
| 目録 | `areka_ghost::catalog_roots::list_ghosts`・`catalog::is_ghost_dir` | 要件 3.1・3.10 の「`ghost\master\descript.txt` を持つか」・要件 4.1 |
| 置き場の印 | `areka-ghost/src/catalog.rs:33` `RootMark { Plain, UserRoot }` | 一時の置き場の印の先例。`UserRoot` はメニューの「外す」を選べなくしている（`menu/roots_frame.rs:71`） |
| 置き場の編集 | `roots/edit.rs`（`decide_add`・`decide_remove`・`decide_front`・`apply`→`encode_memory`） | 要件 3.4・3.6・3.7 が効く所 |
| 入れ先 | `catalog_roots.rs:149` `install_root` | 要件 3.5 が効く所 |
| MCP の待受 | `areka-mcp/src/server.rs:35` `start(&[u16], ..)`（先頭から試し、最初に束ねた 1 つ・`127.0.0.1` だけ） | 要件 8.3 はこのまま満たす。候補の列だけ作ればよい |
| SHIORI の差し替え | `areka-ghost/src/runtime.rs:63` `ShioriWiring::Custom(Box<dyn FnOnce() -> Result<Box<dyn ShioriBackend>, String>>)` | 中身なしの SHIORI を **`runtime.rs` に触らずに**差せる |
| 起動入力の作り口 | `main.rs:436` `install_boot_context` が据える `GhostBootInputsSource`（切替もここを通る） | 要件 7.6（切り替えた先も読まない）を 1 か所で効かせられる |
| 倒れ先の起動 | `ghost_session.rs:680` `ghost_boot_options(.., helper_exe, ..)`（`Helper` 固定） | ここも差し替えが要る |
| OS で開く | `readme/os_port.rs` `WindowsShell::shell_execute`・`init_com_for_shell` | `--help`（要件 9.1）。今の開く口（`Opener`）は World と専用のスレッドに載っていて、起動前には使えない |
| 起動の 204 | `areka-kanade/src/schedule/boot.rs`（204 は台本なしで次へ） | 要件 7.2・7.3 の「中身なしで答えたとき」の振る舞いは今ある |
| MAKOTO | `areka-ghost/src/runtime.rs:707` `TranslateSeams::passthrough().makoto` | areka は MAKOTO を読み込まない。要件 7.1 の MAKOTO の部分は今のままで満たす |
| 自前の引数の読み | `crates/areka-impl-watch/src/cli.rs`（表 1 本から読む） | 先例。部品（クレート）は足さない |

### 2.3 決まりごと（steering・記憶）

- 1 ファイル 1,000 行（`file_length_guard`）・テストは兄弟の `*_tests.rs`・失敗の道は `error!`＋`Err`・メッセージボックスは出さない・本番の環境変数は `AREKA_`・依存を足さない（C9 で依存の席は ⑥ が使う）。
- 試験の根と検体はワークツリーの `target\` の下だけ。

## 3. 要件ごとの対応と穴

記号: **在る**＝今のコードで足りる／**足す**＝新しく書く／**不明**＝設計で調べる／**制約**＝縛りがある。

| 要件 | 対応する所 | 判定 | メモ |
|---|---|---|---|
| 1.1〜1.13 読み方 | 新 `cli/parse.rs` | 足す | 純粋に「引数の列 → 中身＋記録の材料の列」を返し、記録は後で出す形にすると、要件 11.1 の「環境変数を使わずに」を満たせる（記録は `log_history::init` の後）。正典の値の取り方の表（値なし・1 つ・省略できる 1 つ・空白区切りで複数）を 1 本持つ |
| 1.6 正典にあるが扱わない名前 | 同上 | 足す | 値の取り方は正典から: `--install`＝複数・`--offline-dump`／`--target-dir`／`--output`／`--fmo-name`／`--ukagaka-link`／`--ssf`／`--guifont`＝1 つ・`--dump-*`／`--password`／`--windowmode`／`--help`＝省略できる 1 つ・`--offline-tool`＝1 つ。短い形は `/f`・`/m`・`/i`・`/s` |
| 1.13 終了コードを変えない | `main.rs` | 制約 | 読み方の結果で `Err` を返さない |
| 2.1〜2.3・2.5 位置の引数 | `resolve_boot_from` | 在る＋足す | 入力を `&[String]` から「起動オプションの中身」へ替える。位置の 1 つ目・2 つ目は今の `Argv` の道を通す（経路の名前 `Argv` を保つ） |
| 2.4 位置と `--ghost` の両方 | 同上 | 足す | `--ghost` が勝つ。位置のバルーンは残す（2.5） |
| 2.6 実走の 3 本 | `crates/areka/tests/*` | 制約 | ファイルに触らない。引数は絶対パスだけで、`-`・`/` で始まらない（読み方が位置の引数として読む）。`route=Argv` の照合が残る |
| 3.1・3.10 絶対パスの判定 | `catalog::is_ghost_dir` | 在る | |
| 3.2・3.3 一時の置き場を足す | `resolve_boot_from`・`ResolvedRoots` | **不明** | SSP の「親」はゴーストのフォルダを並べたフォルダ（正典の例: `--ghost "C:\Users\user\Desktop\myGhost"` → `C:\Users\user\Desktop\` を「ゴーストの読み込みフォルダ」へ）。areka の置き場は直下に `ghost\`・`balloon\` を持つ。親をそのまま置き場にすると `親\ghost\myGhost` を探す。決め方は 6 節の議題 1 |
| 3.4 記憶に書かない | `roots/resolve.rs:275` `encode_memory` | 足す | 印の付いた項目を飛ばす |
| 3.5 入れ先にしない | `catalog_roots.rs:149` `install_root` | 足す | 見えている根・いちばん上の根のどちらからも除く |
| 3.6 先頭へ・外す | `roots/edit.rs` `decide_front`・`decide_remove`・`menu/roots_frame.rs` | 足す | 判断で断る＋メニューで選べない形（`UserRoot` と同じ型） |
| 3.7 追加で普通の置き場へ | `decide_add` | 足す | 今は同じ根なら `AlreadyListed` で変えない。印を外して覚える腕が要る |
| 3.8 同じフォルダ名 | `catalog_roots::union` | **制約** | 起動は `dir` を直に使うので満たせる。ただし和は上の根を残すので、末尾の一時の置き場のゴーストはメニューの一覧・切替・記憶の引き当てで隠れる（6 節の議題 3） |
| 3.9 ほかと同じ決まり | `GhostRoute`・`argv_session`・`LastUsed` | 足す | 新しい経路（例 `Option`）を足せば `argv_session` は偽になり、印も記憶も今の道で書く。`GhostDecision.folder` は `Some(フォルダ名)` を詰める。`match` で全部の腕を並べている所は無い（足しやすい） |
| 3.11 次の起動で無い | `resolve_ghost` 段 2 | 在る | `last_ghost_not_found` の warn で次へ。ほかの置き場に同じフォルダ名があると黙ってそれが選ばれる（議題 3） |
| 4.1〜4.5 名前 | `resolve_switch_target` | 在る | `list_ghosts` の結果へ当てる。`ghost_switch.rs` は ④ `mcp-reload` が先に割る（呼ぶだけでも、関数の場所が動くと取り込みでつなぎ直し） |
| 4.6 空白区切りで複数 | `cli/parse.rs` | 足す | `--ghost` は次の起動オプションまでを全部飲む＝位置のバルーンは `--ghost` の前に置く必要がある（説明書に書くか） |
| 5.1〜5.4 `--random` | `resolve_ghost` | 足す | 入力に「無作為」を足し、記憶・印・既定の段を飛ばす。候補 1 体のとき `pick(1)` は前提の外（議題 7）。0 体は今の `NoGhost` |
| 6.1〜6.7 `--option` の語 | `cli/parse.rs` | 足す | 語の表 1 本（効かせる 2・受けて何もしない 5・まだ扱わない 4） |
| 6.2・6.3 `nobootcheck` | `resolve_boot_from:259` | 足す | 読むのをやめるだけ。書く・消すは今のまま（`argv_session` は偽のまま） |
| 7.1 SHIORI・helper を起こさない | `ShioriWiring::Custom` | 在る＋足す | 中身なしの `ShioriBackend`（`get`→`Ok(None)`・`notify`→`Ok(())`・`unload`→`Ok(ExitKind::Clean)`・`status`→`Running`）を 1 つ書く。テストの偽物は 17 か所あるが本番の型は無い |
| 7.2・7.3・7.5 | kanade の 204 の道 | 在る | 起動の 204 は台本なしで次へ・終了の 204 は終わる（記憶「終了イベントのトークは必ず終了で終わる」） |
| 7.4 SHIORI の行が無い・読めない | `package::resolve`（`shiori.file` は `Option`） | **不明** | マウントが SHIORI の DLL の実在を見ないかを設計で確かめる。`Custom` は `shiori.file` を読まない |
| 7.6 切替の先も読まない | `install_boot_context` の作り口・`ghost_session.rs:680` 倒れ先 | 足す | 旗を作り口の閉包と倒れ先の両方へ運ぶ。`ghost_switch.rs` には触らずに済む |
| 7.8 PLUGIN など | — | 在る | areka に読む働きが無い |
| 8.1〜8.8 `--sstp-listen` | `areka-mcp/src/port.rs` か `cli/` | 足す | 候補の列を作れば `start` がそのまま試す（8.3）。8.7「全部無視されたとき error」: 今の `start` は空の列に `info!("待ち受けない（AREKA_MCP_PORT が 0）")` を出すので文が合わない（議題 9） |
| 9.1〜9.4 `--help` | `main.rs`・`readme/os_port.rs` | 足す | 記録の初期化の後・起動の解決と MCP の前に終わる道。`exe_location()` の隣の `README.txt`。開発のビルド（`target\debug\`）には README が無い＝9.3 の error＋非 0 の道を通る |
| 10.1〜10.3 説明書 | `dist/README.txt` | 足す | 10 行目「引数は要りません」を書き直す＋新しい節。⑤ `sample-url-fetch` と別の行（⑤ の brief は `dist/README.txt` を挙げていない） |
| 11.1〜11.5 テスト | 兄弟の `*_tests.rs` | 足す | `resolve_boot_from` の呼び手は試験に 18 か所（`main_config_input_tests.rs` 10・`main_session_mark_tests.rs` 3・`boot_config_*_tests.rs` 4・`main_session_mark_fallback_tests.rs` 1）。入力の型を替えると全部が書き換わる（議題 12） |

## 4. 実装の案

### 案 A: 今のファイルを伸ばす

- `main.rs` の中で引数を読み、`boot_config.rs` に `--ghost`・`--random` の分岐を足す。
- ✅ 新しいファイルが少ない。
- ❌ `main.rs` は 1,000 行で足せない。`boot_config.rs`（467 行）が読み方と解決の両方を持ち、後に続く 3 本（`offline-surface-dump`・`readonly-mode`・`startup-install-option`）が同じファイルへ足すことになる（roadmap は `cli/` を 4 本で分け合う前提）。
- 判定: 取らない理由が強い。

### 案 B: 全部を新しいモジュールに置く

- `cli/` に読み方だけでなく、起動の解決の別版（`--ghost` 用の解決）・一時の置き場の別の列・独自の MCP の候補・中身なしの SHIORI まで置き、今の部品には触らない。
- ✅ 既存のファイルの差分が最小。並走との衝突が最小。
- ❌ ゴーストの選び方が 2 本になる（要件の「今の段の前に差し込むだけ」と食い違う）。一時の置き場を別の列で持つと、目録・入れ先・メニュー・記憶の書き出しの各所が両方の列を見る必要があり、かえって触る所が増える。
- 判定: 読み方は B の形、解決は B にしない方が素直。

### 案 C: 混ぜる（推し）

- **新しく置く**: `crates/areka/src/cli/`（`mod.rs`＝起動オプションの中身の型、`parse.rs`＝純粋な読み方、`parse_tests.rs`）。`--help` の開いて終わる道、`--sstp-listen` の候補の列、中身なしの SHIORI の型もここか近くに置ける。
- **値を渡して伸ばす**: `resolve_boot_from`（入力の型・`--ghost`・`nobootcheck`）・`resolve_ghost`（無作為の旗・新しい経路）・`RootMark` に一時の印（`areka-ghost/src/catalog.rs`）と、それを見る 5 か所（`encode_memory`・`install_root`・`decide_front`・`decide_remove`・`decide_add`＋`roots_frame.rs`）・`install_boot_context` と倒れ先へ SHIORI の旗。
- **外へ出す**: `main.rs` から 30 行ほど（例: `smoke_exit_ms` の塊 905〜933 行、または `install_boot_context`）を先に出して、引数まわりの差し替えの行を入れる余地を作る。
- ✅ 選び方は 1 本のまま。後に続く 3 本は `cli/` に項目を足すだけ。`runtime.rs`・`config.rs`・実走の 3 本に触らない。
- ❌ 触るファイルが 10 を超える（`boot_config.rs`・`boot_resolve.rs`・`roots/{resolve,edit}.rs`・`menu/roots_frame.rs`・`areka-ghost/src/{catalog,catalog_roots}.rs`・`ghost_session.rs`・`main.rs`・MCP の 1 か所・`dist/README.txt`）。試験の呼び手 18 か所の書き換え。

#### 案 C の中の小さな分かれ道

| 論点 | 案 ⑴ | 案 ⑵ |
|---|---|---|
| 中身なしの SHIORI の置き場 | `areka-ghost/src/shiori_wiring.rs` に `empty_connect()` を置き `lib.rs` で公開（要件の Adjacent の書き方どおり・`lib.rs` は C9 で誰の持ち物でもない） | `crates/areka` の中に置く（`areka_kanade` が `ShioriBackend`・`ExitKind`・`HelperStatus` を再輸出しているので書ける・areka-ghost に触らない） |
| `--sstp-listen` を読む所 | `areka-mcp/src/port.rs` に「待ち受け先の文字列 → 候補」を足す（brief の案・roadmap は ② に `port.rs` を割り当て済み・`lib.rs` の再輸出に 1 行触る） | `cli/` で読み、今の `start(&[u16])` へ渡す（`areka-mcp` に触らない＝④ が「`areka-mcp/**` は ⑧ の持ち物」と書いている所を避けられる） |
| 一時の置き場の表し方 | `RootMark::Temporary` を足す（`UserRoot` と同じ型・`report.rs:37` の写しに 1 腕） | `BootContext` に別の列を持ち、目録を引くたびに足す（触る所が増える） |

## 5. 規模と危うさ

- 規模: **M**（10〜15 タスク）。引数の読み方は先例のある純粋な表、解決は今の段へ値を渡すだけ。一時の置き場の 5 か所と、中身なしの SHIORI と、試験の 18 か所の書き換えで膨らむ。上限 20 の内。
- 危うさ: **中**。新しい技術は無いが、一時の置き場が SSP と areka の「置き場」の意味のずれ（議題 1）と目録の隠れ（議題 3）にかかり、決め方で触る所が変わる。`main.rs` の上限と、並走 ④ が割る `ghost_switch.rs` の関数を呼ぶことも小さな危うさ。

## 6. 議題（要件の議論へ渡すもの）

1. **一時の置き場の「親」の単位**: SSP の親＝ゴーストのフォルダを並べたフォルダ（areka なら `<置き場>\ghost\` に当たる）。areka の置き場は 1 段上。候補: ⒜ 親の名前が `ghost` なら祖父母のフォルダを置き場として足す（`balloon\` も見える）・それ以外は ⒝ へ／⒝ 「ゴーストだけの置き場」（`ghost_store()` が親そのものを返す項目）を作る／⒞ 親をそのまま置き場にする（`親\ghost\` を探して見つからない＝取れない）。要件 3.2・3.3 の「親が置き場の 1 つと同じ」は「親がどれかの置き場の `ghost\` と同じ」と読むのが筋で、文言の直しが要るかもしれない。
2. **一時の置き場の表し方**: `RootMark` に一時の印を足す案（効く所は記憶への書き出し・入れ先・先頭へ・外す・追加で普通の置き場へ戻す、の 5 か所＋メニュー）か、別の列か。
3. **目録の隠れ（要件 3.8 の後）**: 末尾に足すと、上の置き場に同じフォルダ名のゴーストがあれば、メニューの一覧・名前の切替・最後のゴーストの記憶の引き当てがそちらを指す。起動だけ指された物にするのか、一時の置き場を目録でも上に置く（並びの先頭へ入れる＝SSP とも roadmap の「末尾」とも違う）のか、記憶を書かないのか。
4. **写しとの関係**: `--ghost` の絶対パスが areka.exe の隣の置き場の中を指すとき、要件 3.3 で一時の置き場は足さない。そのとき要件 3.9「ほかと同じ決まり」に従って利用者ごとの置き場へ写すと、起きるのは写した先で、要件 3.1「そのフォルダのゴースト」と食い違う。
5. **起動中の印の値と `OnBoot` の由来**: `--ghost`・`--random` は「印に関わらず選ぶ」だが、印の値（前回落ちたゴースト）を `OnBoot` の Ref6=halt・Ref7 に今までどおり渡すか。`nobootcheck` は読まない＝由来も「ふつう」になる。
6. **新しい経路の名前**: 記録の `route=` に新しい値（例 `Option`・`Random`）を足す。位置の引数は `Argv` のまま（実走の試験が照合）。`--random` で選んだときは今の段 5 と同じ `Random` を使うか、別にするか。
7. **`--random` の候補が 1 体のとき**: `pick` は 2 以上が前提。1 体ならその 1 体（要件 5.2 の記録は候補 1）でよいか。
8. **中身なしの SHIORI の置き場と旗の運び方**（4 節の案 ⑴／⑵）。あわせて、並走 ④ `mcp-reload` の `\![reload,shiori]`・MCP の `reload` が SHIORI を読み直すとき、起動入力の作り口（`GhostBootInputsSource`）を通るなら旗は効く。通らない形なら ④ との申し合わせが要る。
9. **`--sstp-listen` を読む所と記録**（4 節の案 ⑴／⑵）。全部無視されたとき（要件 8.7 の error）と `none`（要件 8.4）の区別: 今の `start` に空の列を渡すと「AREKA_MCP_PORT が 0」の info が出て文が合わない。`server.rs` に触らないなら、空のときは `start` を呼ばずに areka の側で記録する形になる。正典の「ポート番号」は IPv4 と IPv6 の両方のループバックだが、要件 8.2 は 127.0.0.1 だけ（`start` も 127.0.0.1 だけ）＝説明書に書くか。
10. **`--help` の開き方**: 起動前で World が無いので、今の開く口（`Opener`）でなく `WindowsShell::shell_execute` を 1 回だけ同期に呼ぶ（COM の初期化 `init_com_for_shell` を先に）。OS が「開いた」と答えても表示までは確かめない。
11. **`main.rs` の 1,000 行**: 先に外へ出す塊をどれにするか（`smoke_exit_ms` の塊・`install_boot_context`・`first_boot_origin` など）。足すのは `mod cli;`・起動オプションを読む 1 行・`--help` で終わる分岐・MCP の候補の選び・SHIORI の旗。
12. **試験の呼び手 18 か所**: `resolve_boot_from` の入力の型を替えると全部書き換わる。`&[String]` を受ける薄い口を残して試験を据え置くか、全部を新しい型へ移すか（実走の 3 本はどちらでも触らない）。
13. **引数の細部**: ⒜ `std::env::args()` は UTF-8 でない引数で panic する（今も同じ）。`args_os` へ替えるか ⒝ `/` で始まる位置の引数（MSYS の `/c/...` の形）は短い形として読まれ、知らない名前で無視される ⒞ `--ghost` は後ろの位置の引数を全部飲むので、位置のバルーンは `--ghost` の前に置く必要がある（説明書に書くか）。
14. **網羅台帳**: `doc/ukadoc-coverage/ledger/` の 4 つ（`assets`・`property`・`sakura-script`・`shiori`）に起動オプションの行は無い。新しい台帳を足すか、この spec では足さないか（brief の見立てでも議題）。
15. **名前の引き当ての関数の場所**: `resolve_switch_target` は `emo2_boot/ghost_switch.rs`（948 行・④ が先に割る）にある。呼ぶだけにするか、純粋な判断を `boot_resolve.rs` 側へ移して両方から呼ぶか（移すと ④ のファイルに触る）。

## 7. 設計で調べること（Research Needed）

- `areka_parsers::package::resolve` が、`shiori` の行の無い・DLL の無いゴーストを通すか（要件 7.4）。`wire_emo2_boot` の側に SHIORI の DLL を読む所が無いか。
- 中身なしの SHIORI で起こしたとき、`OnBoot` の 204 の後に既定のサーフェスが出るか（窓は起動の前に開くので出る見込み・実機の確かめは要件 7.3）。
- `RootMark` に腕を足したとき、`match` で全部の腕を並べている所（今は `roots/report.rs:37` だけ）と、`menu/roots_frame.rs` の選べる・選べないの判断。
- 一時の置き場を末尾に足したときの起動時の記録（`report::log_roots`・`log_hidden`）に印を載せるか。

## 8. 並走の約束との照合（C9-②）

| 約束 | 案 C での見込み |
|---|---|
| 実走の 3 本（`emo2_real_run`・`mcp_get_log_real_run`・`smoke_boot_loop_exit`）に触らない | 触らない。位置の引数と `route=Argv` の記録を保てば通る |
| `areka-ghost/src/runtime.rs`・`config.rs`（③ ④） | 触らない。`ShioriWiring::Custom` で足りる |
| `areka-ghost/src/shiori_wiring.rs` | 案 ⑴ なら触る（許されている）。案 ⑵ なら触らない |
| `areka-ghost/src/catalog.rs`・`catalog_roots.rs` | 一時の印で触る。C9 のほかの 7 本の触るファイルには無い |
| `crates/areka-mcp/**`（④ が「⑧ の持ち物」と書く・⑧ の実際は `handler.rs`・`real_receiver_test.rs`） | roadmap は ② に `port.rs` を割り当て済み。`lib.rs` の再輸出の 1 行も触るなら ⑧ と申し合わせ。案 ⑵ なら触らない |
| `emo2_boot/ghost_switch.rs`（④ が先に割る） | 呼ぶだけ。触らない（議題 15 で移すなら触る） |
| `ghost_session.rs`（885 行） | 倒れ先と作り口の SHIORI の旗で触る。C9 のほかの 7 本の触るファイルには無い（④ の brief では Adjacent）。**2026-10-18 areka棚卸 の申し送り**: ③ ④ とも `shiori_wiring.rs` に触らない（`dontloadmodule` は入れてよい）。ただし ④ は「本物の SHIORI の読み直し」を選ぶと `ghost_session.rs` に触る見込み。設計では `shiori_wiring.rs`・`boot_config.rs` の中で済ませる形を先に探し、`ghost_session.rs`（倒れ先 680 行目）に触るなら着手の前に areka棚卸 へ知らせて ④ と申し合わせる |
| `dist/README.txt`（⑤ と別々の行） | 10 行目の書き直しと新しい節。⑤ の brief の触るファイルに `dist/README.txt` は無い |
| `main.rs` 1,000 行 | 先に外へ出すタスクが要る（議題 11）。全体としては減らす |

## 9. 次の段へ

- 要件の議論で 6 節の議題（とくに 1・3・4・5）を決める。答えによっては要件 3.2・3.3・3.8 の文言が変わる。
- **2026-10-18 要件の討議で決まった**: 議題 1＝親そのものを「ゴーストだけの置き場」として足す（案 ⒝）。議題 3・4＝一時の置き場は並びの先頭に置き、写しがあっても指されたフォルダそのものを起こす。議題 5＝`nobootcheck` なしなら印を読み、`OnBoot` の由来は今までどおり渡す（要件 6.8）。議題 14＝網羅台帳は足さない。残りの議題（2・6〜13・15）は設計の段で決める。
- その後 `/kiro-design areka-P0-startup-options-core` で設計へ。設計では案 C を土台に、4 節の小さな分かれ道と 7 節の調べものを片付ける。

## 10. 設計の段の調べと決めごと（2026-10-18・kiro-spec-design）

- **調べの種類**: 軽い調べ（今ある仕組みの拡張）。新しい部品（クレート）は無いので外の調べはしない。正典は ukadoc MCP の `ssphelp:option` を全文で読み直し、短い形の対応（`/o`・`/r`・`/g`・`/?`・`/f`＝`--fmo-name`・`/m`＝`--ukagaka-link`・`/i`＝`--install`・`/s`＝`--ssf`）と、値の取り方（`--install`・`--ghost`＝空白区切りで複数／`--fmo-name`・`--ukagaka-link`・`--ssf`・`--offline-dump`・`--offline-tool`・`--target-dir`・`--output`・`--guifont`＝1 つ／`--windowmode`・`--dump-*`・`--password`・`--help`・`--sstp-listen`＝省略できる 1 つ）を確かめた。
- **使ったスキルと決まり**: `kiro-spec-design` の規則（境界を先に・型・図・レビューの関門）・steering の `logging.md`（`event=` と `[部品名]` の前置き）・記憶の決まり（1,000 行・兄弟のテスト・依存を足さない・`error!`＋`Err`・メッセージボックスを出さない・試験の根は `target\`）。

### 10.1 コードで確かめたこと

| 問い | 答え | 根拠 |
|---|---|---|
| SHIORI の行が無いゴーストをマウントできるか（7.4） | できる。`ShioriMount.file` は `Option` のまま通り、DLL の名前を読むのは接続の閉包（`real_connect`・`inproc_connect`）だけ。`ShioriWiring::Custom` は `mount.shiori` を読まない | `areka-parsers/src/package/model.rs:201`・`areka-ghost/src/runtime.rs:660-673` |
| 中身なしの SHIORI を `lib.rs` に触らずに外へ出せるか | 出せる。`shiori_wiring` は `pub mod` | `areka-ghost/src/lib.rs` |
| 倒れ先の起動の SHIORI はどこで決まるか | `ghost_session.rs:680` が `boot_config::ghost_boot_options` を呼び、その中で `Helper` を詰める。`ghost_boot_options` の中で決め方を替えれば `ghost_session.rs` に触らずに済む | `boot_config.rs:414` |
| 結線ありの起動と切替の SHIORI はどこで決まるか | `main.rs` の `install_boot_context` が据える作り口の閉包（`GhostBootInputs::production` を呼ぶ）。閉包の中で `wiring.shiori` を上書きできる（欄は `pub`） | `main.rs:436-456`・`ghost_session.rs:364-388` |
| 中身なしの SHIORI の死活 | 実行系は `status()` が `Exited` を返したときだけ落ちたと見る。常に `Running` を返せばよい（`InProc` と同じ） | `areka-kanade/src/shiori/real.rs:228-250` |
| `RootMark` を網羅の `match` で見ている所 | `roots/report.rs:37` だけ。ほかは `matches!` | grep |
| 置き場の項目を探す判断 | `decide_add`・`decide_remove`・`decide_front` は `same_root(r.dir(), …)` で最初の一致を取る。一時の置き場と普通の置き場が同じフォルダを指すと、一時の方を掴む | `roots/edit.rs` |
| 入れ先の決め方 | `install_root`（`install/destination.rs:99` が呼ぶ 1 か所）。見えている項目の根か、見つかるいちばん上の根 | `areka-ghost/src/catalog_roots.rs:149` |
| `--help` で開く口 | `readme/os_port.rs` の `WindowsShell`（`ShellExecuteExW`・`SEE_MASK_FLAG_NO_UI`）と `init_com_for_shell`。`readme.rs` の中で `mod os_port;`（私有）なので、外から使うには `pub(crate) mod` に替える 1 行が要る | `readme.rs:197` |
| MCP の待受の空の列 | `areka_mcp::start(&[])` は「AREKA_MCP_PORT が 0」の info を出す＝要件 8.4・8.7 の文に合わない。`start` を呼ばなければよい（`areka-mcp` に触らない） | `areka-mcp/src/server.rs:35-39` |
| 実走の 3 本の引数 | どれも絶対パス 2 つだけで、`-`・`/` で始まらない。`smoke_boot_loop_exit.rs:311` は `route=Argv` を照合する | `tests/*.rs` |
| `main.rs` の外へ出せる塊 | 自動 close の仕掛け（`smoke_exit_ms` の塊・275〜305 行・904〜937 行の約 60 行）。テストは `main_startup_window_tests.rs` の 1 本（`use super::*` で `smoke_exit_ms_from` を見るだけ）で、ほかの関数に触らない | `main.rs` |

### 10.2 議題の残り（2・6〜13・15）の決め

| 議題 | 決め | 理由 |
|---|---|---|
| 2 一時の置き場の表し方 | `RootMark::Temporary` を足す。項目の `dir()` はゴーストのフォルダを並べたフォルダそのもので、`ghost_store()` は `dir()` を返す。バルーンは探さない（`scan_balloons`・`balloon_stores` が飛ばす） | 印の先例（`UserRoot`）と同じ形。目録・入れ先・メニュー・記憶が並び 1 本だけを見ればよい |
| 6 経路の名前 | `--ghost` で決まったら新しい `GhostRoute::Pointed`。`--random` は今の `Random` を使い、記録の `event` で分ける（`ghost_random_requested`） | 位置の引数の `Argv` はそのまま（実走の照合）。`Random` を特別に扱う所は無い |
| 7 候補 1 体の `--random` | その 1 体（`pick` を呼ばない）。記録の候補の数は 1 | `pick` は 2 以上が前提 |
| 8 中身なしの SHIORI と旗 | `areka-ghost/src/shiori_wiring.rs` に `EmptyShiori` と `empty_connect()`。旗（`ModulePolicy`）は `install_boot_context` の引数で作り口の閉包へ渡し、倒れ先のためにプロセスに 1 つの値として `boot_config` にも置く（`main` が 1 度だけ詰める）。`ghost_session.rs` には触らない | 倒れ先の呼び手（`ghost_session.rs:680`）の形を変えずに済む唯一の道。作り口の側は引数にしたのでテストで確かめられる |
| 9 `--sstp-listen` | `crates/areka/src/cli/listen.rs` で読み、`areka_mcp::start(&[u16])` へ渡す。`none`・全部はねたときは `start` を呼ばずに areka が記録する。`areka-mcp` に触らない | 並走 ⑧ の持ち物を避ける。今の `start` が 8.3（最初に束ねた 1 つ・127.0.0.1）をそのまま満たす |
| 10 `--help` の開き方 | 起動の解決・MCP・窓の前に、`WindowsShell::shell_execute` を 1 回だけ同期に呼ぶ（先に `init_com_for_shell`）。OS が受け付けたら 0、断ったら error＋1 | World も専用のスレッドもまだ無い |
| 11 `main.rs` の 1,000 行 | 自動 close の仕掛けを新しい `smoke_exit.rs` へ出す（テストは `main_startup_window_tests.rs` を `smoke_exit_tests.rs` へ名前替え）。記録の文と環境変数の名前は変えない | ほかの関数とテストを動かさずに約 55 行減る |
| 12 試験の呼び手 18 か所 | 本番は `resolve_boot(&BootRequest)`→`resolve_boot_with`。今の `resolve_boot_from(resolved, &[String], …)` はテストのための口（`#[cfg(test)]`）として残し、中で `cli::parse` を通してから `resolve_boot_with` を呼ぶ | 18 か所を書き換えずに済み、新しいテストも引数の列のまま書ける |
| 13 引数の細部 | ⒜ `std::env::args_os()` で読み、UTF-8 でないものは置き換えて warn（1.14）⒝ `/c/...` は短い形として読まれ、知らない名前で warn＋無視（説明書に「位置の引数は Windows の綴り `C:\…` で」と書く）⒞ 位置の引数は `--ghost` の前に書く（説明書・10.1） | 正典の読み方どおり |
| 15 名前の引き当て | `ghost_switch::resolve_switch_target` を呼ぶだけ（動かさない）。④ `mcp-reload` が `ghost_switch.rs` を割って場所が変わったら、取り込みで `use` の行をつなぎ直す | 並走の約束 |

### 10.3 そのほかの決め（要件の文が決めていない細部）

- `--名前=値` を値を取らない名前（`--random=x`）に付けたとき: 名前は効かせ、値は使わずに warn を 1 件（`option_value_ignored`）。要件 1.13 の「止めない」の内。
- `--ghost=A B` は `--ghost A B` と同じ（要件 1.2「`--名前 値` と同じ意味」）。
- `--ghost` だけ（値なし）と位置の引数のゴーストが両方あるとき: `--ghost` が渡されたとみなし、位置の引数のゴーストは使わない（2.4 の warn）・4.7 の warn・いつもの選び方。
- `--ghost` の指し先が見つからない（3.10・4.4・4.7）ときの「いつもの選び方」は、用語の定めどおり `--random` と `nobootcheck` の効いた選び方。5.3 の「`--random` を使わなかった」の info は、指されたゴースト（または位置の引数のゴースト）を実際に起こしたときだけ出す。
- `--sstp-listen` の値の中の `none` は、値の全体が `none` のときだけ「待ち受けない」。ほかの待ち受け先と並んだ `none` は書式の誤り（8.5 の warn）。`127.0.0.1` 以外の `127.x.x.x` は 8.6 の warn。
- 一時の置き場があるあいだの「置き場を先頭へ」: 先頭は「覚える置き場の先頭」＝一時の置き場の直後へ入れる（一時の置き場は並びの先頭のまま＝3.9）。「置き場を外す」の「最後の 1 つ」は覚える置き場だけで数える。3 つの判断は覚える置き場だけを探す（一時の置き場と普通の置き場が同じフォルダを指しても取り違えない）。
- `--ghost` の名前で決まったゴーストは、記憶の段で決まったのと同じに利用者ごとの置き場へ写す（3.9・4.3）。絶対パスで決まったゴーストは写さない（3.8）。

### 10.4 まとめ方（design-synthesis）

- **一般化**: `--ghost` の指し先・`--random`・記憶・唯一・既定・無作為を 1 本の選び方（`resolve_ghost`）の段として並べる。指し先は段 1 の位置の引数の直後の段で、選び方は 2 本にならない。起動オプションの読み方は表 1 本（名前・短い形・値の取り方・種類）で、後に続く 3 本（`offline-surface-dump`・`readonly-mode`・`startup-install-option`）は表の行を「扱わない」から「効かせる」へ移して `StartupOptions` に欄を足すだけ。
- **作るか借りるか**: 引数を読む部品は借りない（依存を足さない・SSP の書き方〔`/x`・`--x=v`・空白区切りで複数・省略できる値〕に合う部品が無い）。先例は `areka-impl-watch/src/cli.rs` の表。MCP の待受・名前の引き当て・OS で開く口・SHIORI の差し替え口は今あるものを借りる。
- **削ったもの**: 新しいクレート・`areka-mcp` の新しい入口・`cli/` の中の記録専用のファイル・`BootContext` の新しい欄（使う所が 1 つも無いのに 23 のテストを書き換えることになる）・`--help ファイル名` の働き。

### 10.5 危うさ

- **倒れ先のためのプロセスに 1 つの値**: `boot_config` の `OnceLock`。`main` が起動の解決の前に 1 度だけ詰め、テストは詰めない（既定は「読む」）。2 度目は配線の誤りとして error を残して最初の値を保つ。
- **④ `mcp-reload` の SHIORI の読み直し**: 作り口（`GhostBootInputsSource`）か `boot_config::shiori_wiring` を通らずに `real_connect` を直に呼ぶ形になると、`dontloadmodule` が読み直しで破れる。着手の前に areka棚卸 へ知らせ、④ と申し合わせる（design.md の「並走の申し合わせ」）。
- **一時の置き場とバルーン**: 一時の置き場の下の `balloon\` は探さない（要件 3.2）。`BasewareRoot::balloon_store()` 自体は一時の置き場でも綴りを返すので、和と `balloon_stores` が必ず飛ばす。新しい呼び手が `balloon_store()` を直に使うときは印を見る。
- **`--ghost` で exe の隣のゴーストを絶対パスで指した起動**: 写さないので、Program Files などの書けない場所なら今の位置の引数と同じく書けるかの確かめ（`write_failure::check_boot_dirs`）で止まる。今の決まりのまま。

### 10.6 設計のレビューの関門

- 1 回目: 機械の確かめで要件 10.1〜10.3 が対応表の行に無い（範囲の 1 行にまとめていた）→ 3 行に割って直した。境界の 4 節・ファイルの計画・部品とファイルの対応は空きなし。
- 2 回目: 78 の要件がすべて対応表にあり、部品はすべてファイルの計画に載る。要件の穴・食い違いは見つからない（細部の決めは 10.3）。関門を通った。

## 11. 設計の討議（2026-10-18）

- **検証の指摘の取り込み（自明な直し）**: ⑴ 一時の置き場で動いているゴーストへの `.nar` が置き場の外へ入る穴（`install/destination.rs` の `running_root` の「2 つ上が根」の決めつけ・`copied_root`・控えの根）→ 変えるファイルに足した。⑵ 値を省けるオプションが後ろの位置の引数を値として読む → 読み方は正典どおりのまま、説明書の決まりを「位置の引数はすべての起動オプションより前に」へ広げ（要件 10.1 も）、テストで固める。
- **議題 1（開発者が推奨案で承認）**: `dontloadmodule` の倒れ先への伝え方は、10.2 の議題 8・10.5 の「プロセスに 1 つの値（`OnceLock`）」をやめ、起動の入力 `GhostBootInputs` に `modules` を値として持たせる。結線ありの起動（`production`）と倒れ先（`ghost_boot_options`）が同じ値から SHIORI を決める。
  - 理由: 値の流れが追え、決定論のテストで `Skip` を確かめられる（`OnceLock` だと 1 つのテストで詰めると同じ実行ファイルのほかのテストへ漏れる＝検証の大きな問題 3）。開発者の方針「並走の約束を理由に回避の形へ逃げず、膨らむなら他のセッションと調整」。
  - 代わりに `ghost_session.rs` を約 6 行触る（欄・`production` の引数・倒れ先の呼び出し・厳格な入口の分解）。テストの組み立て 4 か所（`ghost_session_restart_tests.rs`・`ghost_session_strict_tests.rs`・`emo2_boot/ghost_switch_test_support.rs`・`main_ghost_wiring_tests.rs`）に `modules: ModulePolicy::Load` を足す。④ `mcp-reload` も触りうるので、タスクの生成の前に areka棚卸 へ知らせて申し合わせる。
  - 10.2 の議題 8 の「`ghost_session.rs` には触らない」と 10.5 の 1 つ目の項目は、この決めで置き換わった。
