# 設計の検証: areka-P0-startup-options-core

- 日付: 2026-10-18
- 対象: `design.md`（確定版）・`requirements.md`・`research.md`・steering（`product.md`・`tech.md`・`structure.md`・`logging.md`）
- やり方: `kiro-validate-design` の手順（分析 → 大きな問題 → よい所 → GO／NO-GO）。設計の主張はコードで確かめた（下の「コードで確かめたこと」）。

## まとめ

設計は、引数を読む口（`cli`）を純粋に保ち、働かせる所を今ある部品（`resolve_ghost` の段・印つきの置き場の項目・`ShioriWiring::Custom`・`areka_mcp::start`）へ足すだけの形で、要件 1〜11 をすべて部品と口まで追えている。並走の約束（触らないファイル）も守れており、`main.rs` の 1,000 行も先に自動 close の仕掛けを外へ出す段取りで収まる。ただし、一時の置き場のゴーストが動いているときの「入れる先」の決め方に、設計が触ると書いていない所が 1 か所あり、ここだけは設計に書き足してから進むのがよい。

## 大きな問題（3 件）

### 問題 1: 動いているゴーストが一時の置き場にいると、`.nar` の入れ先が置き場の外のフォルダになる

- **中身**: `crates/areka/src/install/destination.rs` の `running_root`（58 行目）と `install_into` の記録（170 行目）は、動いているゴーストの場所から「その 2 つ上（`<根>\ghost\<フォルダ>` の `<根>`）」を根とみなす。今は「`folder` が在る＝どれかの根の `ghost\` の下」という決まり（`boot_resolve.rs` 61 行目「argv の経路だけ `folder` が `None`」）があるので正しい。設計は絶対パスの `--ghost` を `route=Pointed`・`folder=Some` にするが、一時の置き場では場所が `<一時の置き場>\<フォルダ>`（`ghost\` の段が無い）なので、2 つ上は一時の置き場の親になる。
- **起きること**: `--ghost C:\dev\ghosts\emily` で起こした emily に、同じフォルダ名の `.nar`（自分の更新・投げ込み・`\![install]`）を入れると、`running_root` が `C:\dev` を根と答え、`C:\dev\ghost\emily` という並びのどこにも無いフォルダへ展開する。動いている emily は上書きされず、要件 3.5（一時の置き場を入れ先にしない）と 3.9（ほかのゴーストと同じ決まり）の考えから外れる。`CatalogDestinations::new` の控えの根（29 行目 `present().next()`）も、一時の置き場が先頭だとそこを指す。
- **直し方**: `install/destination.rs` を「変えるファイル」に足し、`running_root` は動いているゴーストが一時の項目の下にいるときは答えず（`install_root` の「一時の置き場を飛ばす」答えへ倒す）、`first` と記録の根の引き当ても一時の項目を飛ばす、と書く。テストは `catalog_roots_temporary_tests.rs` か `install/destination_tests.rs` に 1 行ずつ。どちらも並走の約束の外（触ってよい所）。
- **要件**: 3.5・3.9
- **設計の箇所**: 「File Structure Plan / Modified Files」（`install/destination.rs` が無い）・「TemporaryRoot」の `install_root` の決まり

### 問題 2: 値を省略できるオプションと複数を取るオプションが、後ろの位置の引数を飲む

- **中身**: 読み方の決まりは「次の引数が起動オプションの名前でなければ値として読む」（要件 1.8 の定めどおり）。`--ghost` のほかに、`--sstp-listen`・`--windowmode`・`--password`・`--dump-*`（省略できる 1 つ）と `--install`（複数）も後ろの位置の引数を値として読む。
- **起きること**: `areka.exe --sstp-listen 9900 C:\g\emily C:\b\x` は正しいが、`areka.exe --sstp-listen C:\g\emily C:\b\x` のように値を省いて位置の引数を後ろに書くと、ゴーストのパスが待受の値（書式の誤り）になり、バルーンのパスがゴーストとして読まれて「ゴーストなし」の知らせと終了コード 1 で終わる。要件 1.13（書式の誤りで止めない）の考えに触れる、分かりにくい失敗になる。
- **直し方**: 読み方は正典どおりのまま変えず、`dist/README.txt` の節の書き方を「位置の引数は `--ghost` より前に」から「位置の引数は**すべての起動オプションより前に**」へ広げる（要件 10.1 の文言はそのまま満たす）。`cli/parse_tests.rs` にこの形の 1 行（飲まれた結果の notes）を足して、ふるまいを固めておく。
- **要件**: 1.8・1.13・2.1・10.1
- **設計の箇所**: 「CliParse・CliRead / 値の読み方の決まり」・「Supporting References」

### 問題 3: 倒れ先の SHIORI の結線（`ghost_boot_options`）が、`dontloadmodule` のときを決定論のテストで確かめられない

- **中身**: 設計は `ModulePolicy` をプロセスに 1 つの値（`OnceLock`）として `boot_config` に置き、`ghost_boot_options` の中で `module_policy()` を読む。「テストは詰めない」「2 度目は error を残して最初の値を保つ」ので、テストで `Skip` を詰めると同じテストの実行ファイルのほかのテストまで `Skip` になり、詰めなければ `Skip` の道を通せない。一方でテストの計画は「`shiori_wiring_for` と `ghost_boot_options` の表（倒れ先）」と書いており、両立しない。
- **起きること**: 結線ありの起動が成り立たず倒れ先へ落ちたとき（`ghost_session.rs:680`）に、`dontloadmodule` なのに 32bit の補助のプロセスを起こす誤りが入っても、テストは緑のまま通りうる（要件 7.1・11.4）。
- **直し方**: `ghost_boot_options` を「`ghost_boot_options_for(policy, …)`（純粋）を `module_policy()` で呼ぶだけの薄い口」に割り、純粋な方を `Load`・`Skip` の表で確かめる。呼び手（`ghost_session.rs:680`）の形は変わらないので並走の約束は守れる。テストの計画の「`ghost_boot_options` の表」をこの純粋な口の表へ書き直す。
- **要件**: 7.1・7.6・11.4
- **設計の箇所**: 「ModulePolicy」の口・「Testing Strategy / SHIORI を読まない」

## よい所

1. **読む口と働かせる所を分けた形**: `cli::parse` はファイル・環境変数・World に触らない純粋な読み方で、値と記録の材料（`ParseNote`）を返すだけ。働きは今ある 1 本の選び方（`resolve_ghost` に段 1b と無作為の段を足す）・印つきの置き場の項目（`UserRoot` の先例と同じ `RootMark::Temporary`）・`Custom` の差し替え口に入れており、後に続く 3 本が「表の行と欄を足すだけ」で乗れる。`RootMark` の網羅の `match` が `report.rs` の 1 か所だけ、`GhostRoute` の網羅の `match` が無いことも確かめた（腕を足しても並走の約束の外を壊さない）。
2. **並走の約束と今の試験を守る段取りが具体的**: `ghost_session.rs`・`ghost_switch.rs`・`areka-ghost` の `runtime.rs`／`config.rs`・`areka-mcp`・実走の 3 本に 1 行も触らずに、2 か所の SHIORI の結線（作り口の閉包と倒れ先）を `crates/areka` の中だけで差し替える道を見つけている。実走の 3 本は位置の引数の絶対パスだけを渡し、`route=Argv` を照合しているので、設計の「位置の引数と `route=Argv` を保つ」で通る。

## 判断

**GO**（条件つき）

- **理由**: 要件 1〜11 は部品と口まで追えており、境界・依存の向き・記録・1,000 行の上限・並走の約束のどれにも大きな食い違いは無い。上の 3 件はどれも設計の考え方を変えずに、書き足し（変えるファイル 1 本・説明書の 1 文・純粋な口の 1 つ）で直せる。
- **次にすること**:
  1. 設計の討議（`/kiro-design-discussion areka-P0-startup-options-core`）で上の 3 件を議題にし、`design.md` に書き足す（問題 1 は書き足しが要る。問題 2・3 は討議で決めてよい）。
  2. その後 `/kiro-spec-tasks areka-P0-startup-options-core` でタスクを作る。

## コードで確かめたこと

| 設計の主張 | 確かめた結果 | 場所 |
|---|---|---|
| `main.rs` は 1,000 行の上限ちょうど | 1,000 行。自動 close の仕掛けは 275〜304 行・定数と関数は 904〜925 行・テストの宣言は 931〜937 行で、設計の行番号と合う | `crates/areka/src/main.rs` |
| 引数は位置で読むだけ | 171 行 `std::env::args().collect()`・174 行 `resolve_boot(&args)`。ほかに `env::args` を読む所は無い | `main.rs` |
| 倒れ先は `ghost_boot_options` | 414 行で `ShioriWiring::Helper` を組む。呼び手は `ghost_session.rs:680` | `boot_config.rs`・`ghost_session.rs` |
| 作り口の閉包で SHIORI を上書きできる | `install_boot_context`（436 行）が `GhostBootInputs::production` を閉じる。`wiring` の欄は `pub`。呼び手は `main.rs:221` と `main_session_mark_tests.rs:314` の 2 か所 | `main.rs`・`ghost_session.rs:351-386` |
| `ShioriWiring::Custom` は `mount.shiori` を読まない | `Custom(connect) => connect`（667 行）。`shiori` の行が無くてもマウントは通る（`ShioriMount.file` は `Option`） | `areka-ghost/src/runtime.rs:55-74,660-673`・`areka-parsers/src/package/resolve_shell.rs:86-93` |
| `ShioriBackend` の 4 つの口 | `get`・`notify`・`unload`・`status` の形が設計と合う | `areka-kanade/src/shiori/real.rs:61-86` |
| `RootMark` は `catalog.rs` | `Plain`・`UserRoot { from_env }` の 2 腕。網羅の `match` は `report.rs:35` だけ（ほかは `matches!`） | `areka-ghost/src/catalog.rs:31-38` |
| 名前の引き当ては切替と同じ | `resolve_switch_target`（`pub(crate)`）・`GhostSpec::Name` は `name` を先にフォルダ名を後に引く | `emo2_boot/ghost_switch.rs:107,176-202` |
| MCP の待受 | `start(&[u16], ToolRegistry, Option<PathBuf>)`・`read_port_candidates` は公開。空の列は「`AREKA_MCP_PORT` が 0」の文を出すので、設計どおり `Off`・`AllRejected` では `start` を呼ばない形が要る | `areka-mcp/src/server.rs:35-39`・`lib.rs:32-36` |
| 説明書を開く口 | `readme.rs:197` は `mod os_port;`（外から見えない）。`OsPort`・`Verb`・`WindowsShell`・`init_com_for_shell` は `pub(crate)` | `readme.rs`・`readme/os_port.rs` |
| 並走の約束 | 設計の変えるファイルに、触らない約束のファイルは 1 つも無い | design.md「Modified Files」 |
| 実走の 3 本 | どれも位置の引数の絶対パスだけを渡す。煙の試験は `route=Argv` を照合する | `crates/areka/tests/*.rs` |
| 「`folder` が在る＝根の `ghost\` の下」の前提 | `install/destination.rs:58,170` がこの前提で根を 2 つ上と導く（問題 1） | `install/destination.rs` |
