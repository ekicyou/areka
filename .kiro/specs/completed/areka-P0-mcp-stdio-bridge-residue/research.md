# ギャップ分析: areka-P0-mcp-stdio-bridge-residue

> 2026-10-16・`/kiro-validate-gap`。調べた元: ブランチ `claude/areka-p0-mcp-stdio-residue-db9925`（main `0bb51d42` の上・spec の初期化の 1 コミット）。行番号はこの時点のもの。

## 1. まとめ

- 件 1〜8 の記述は、どれも今のコードで当たる（中継・`port.rs`・配布スクリプトは中継の着地から動いていない）。
- どの件も今ある形を少し伸ばせば済む。新しいファイルは要らない見込み（件 2 の置き方しだいで 0 本のまま）。新しい外部のクレートも要らない。
- 手が要るのは主に 3 か所: 配布スクリプトの PE の読み（件 1）・中継の 2 つのテストファイル（件 4・5）・差の記録（件 4・5・7・8）。件 2・3 はコードの付け替えだけ。
- 決めることが残るのは、件 1 の判定の番号と出力の形・件 1 の 1 回の確かめのやり方・件 6 を直す場所（共有の文か中継の文か）・件 5 の変異の扱い。どれも設計で決められ、要件を変える必要は無い見込み（件 6 だけ境界の小さな食い違いがある＝5 節）。
- 規模は S（今の形の延長）。危険は低い。ただし件 5 のテストは rmcp の決まった 5 秒を実時間で待つので約 5〜6 秒かかり、件 1 の確かめは release の組み立て一式を走らせるなら時間が掛かる。

## 2. 今のコードの様子

### 2.1 中継 `crates/areka-mcp-bridge/`

| ファイル | 行 | 役目 | この spec との関わり |
|---|---|---|---|
| `src/lib.rs` | 18 | 部品の宣言。17 行目で `#[cfg(test)] #[path = "../tests/common/fake_server.rs"] mod fake_server` | 件 2 |
| `src/main.rs` | 106 | 入口。1 行目 `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`。`EXIT_WAIT`＝6 秒・`serve` が `run` と「閉じてから 6 秒」を競わせる・`runtime.shutdown_background()` で待たずに終わる | 件 1・5・7 |
| `src/scan.rs` | 134 | 探索。98〜105 行目 `open(port, wait)`＝`tokio::time::timeout(wait, TcpStream::connect(..))`、外れたら `io::ErrorKind::TimedOut` | 件 3・8 |
| `src/http.rs` | 446 | 送り手 `OwnHttp`。125〜130 行目 `post` の頭で同じ形の接続（外れたら `HttpError::Connect(TimedOut)`）。268〜277 行目 `classify` は 202 を `StreamableHttpPostResponse::Accepted` にする | 件 3・4 |
| `src/service.rs` | 493 | 判断。`deliver` が `PeerRequestOptions::with_timeout(reply)` で待ち、`classify`（338〜364 行目）が `ServiceError::Timeout` を `No response from server: timed out after <上限>` にする | 件 4 |
| `src/stdio.rs` | 147 | 輸送。壊れた行への答えを書き終えるまで次の行を読まない（82 行目 `settle_reply`） | 件 7 |
| `src/config.rs` | 125 | 設定。103 行目で `areka_mcp::candidates_from_env_value(port_value)`。UTF-8 でない値は 80〜81 行目で `to_string_lossy` してから渡す | 件 6（**触らない約束**） |
| `src/service_tests.rs` | 869 | 判断のテストと道具（`Desk::start`・`start_with`・`ask`・`close` など `pub(super)`） | 足さない（要件 8.5） |
| `src/service_failure_tests.rs` | 229 | 届けた後の失敗のテスト。91〜113 行目に `Reply::Never` で上限切れを確かめるテスト | 件 4 の置き場の候補 |
| `tests/common/fake_server.rs` | 521 | 偽の受け口。`Reply::Accepted`（202・本文なし）・`Reply::Never`・`Reply::Delayed` を既に持つ。519〜521 行目で `#[cfg(test)] #[path = "fake_server_tests.rs"] mod tests;` | 件 2・4・5 |
| `tests/common/fake_server_tests.rs` | 458 | 偽の受け口の自己確認 2 本（`one_request_gets_the_scripted_reply_and_is_recorded`・`the_handshake_defaults_and_every_reply_mode_answer_as_named`）。`use super::*;` | 件 2 |
| `tests/exe_test.rs` | 568 | 建てた exe を起こす通しのテスト。24 行目でも `#[path = "common/fake_server.rs"] mod fake_server;` | 件 2・5 |

テストの置き方は「実装の隣の `*_tests.rs` を `#[path]` で読む」で揃っている。exe ごしのテストは、番号（偽の受け口の番号か `0`）と `AREKA_PROFILE_DIR`（`target` の下）を必ず渡す作法が `exe_test.rs` の頭（1〜6 行目）に書いてある。

### 2.2 共有の番号の読み `crates/areka-mcp/src/port.rs`（76 行）

- 46 行目: 読めない値の `warn!`＝`"[mcp] ポートとして読めない値なので既定の候補で待ち受ける（1〜65535 か、待ち受けないなら 0）"`（`value` の欄に値）。
- 67 行目: UTF-8 でない値の `warn!`＝`"[mcp] ポートの指定が UTF-8 でないので既定の候補で待ち受ける"`。
- **中継は 67 行目を通らない**。中継の `config.rs` が UTF-8 でない値を `to_string_lossy` で置き換えてから共有の関数へ渡すので、中継では UTF-8 でない値も 46 行目の文（値の欄は置き換えた字）になる。67 行目を出すのは areka 本体（`crates/areka/src/main.rs` 191 行目の `read_port_candidates`）だけ。
- この 2 つの文を確かめるテストは `crates/areka-mcp/src/port_tests.rs` の 82・97・113 行目（`contains("既定の候補で待ち受ける")`）。中継の側には、この記録の文を確かめるテストは無い（`config_tests.rs` は候補の列だけを見る）。

### 2.3 配布スクリプト `tools/package.ps1`（746 行・`SCRIPT_VERSION = '2.1.0'`）

- 351〜380 行目 `Read-PeInfo`: PE のバイト列から機種（`Machine`）と取り込みの DLL 名（`Imports`）を読む。PE32 と PE32+ の両方。オプションヘッダーの位置（`$opt = $pe + 24`）は既に求めている。
- 485〜551 行目 `Test-ZipContent`: zip の中身の判定 1〜8。5・6 は 517〜528 行目で `areka.exe`・`areka-mcp-bridge.exe`・`shiori-host32-helper.exe`・`pasta.dll` の 4 つに当てる。各項目の行は端末へ `'{名前}: machine 0x....., imports ...'` の形で 1 行ずつ出る（522 行目）。否の文は `"5 wrong machine: ..."` のように番号で始まる。
- 589〜594 行目 段「`{arch} content check`」: 否が無ければ `content checks 1-8 all passed`、あれば `FAIL ...` を並べて throw（終了コード 1）。
- 405〜421 行目 段「check static linking」も `Read-PeInfo` を使う（ビルドの出力の exe に当てる。zip の中身の判定ではない）。
- CPU 種別ごとの繰り返しは 553 行目から。`-Arch all`（CI の `release.yml` 260 行目）では x64 と arm64 の両方の zip に `Test-ZipContent` が当たる。
- **窓の作り（Subsystem）を読む所はどこにも無い**。`package.ps1` を確かめる自動のテストも無い（確かめは実際に走らせることだけ）。
- 前例: 中継を足したとき（PR#290）に `SCRIPT_VERSION` を 2.1.0 へ上げている（`BUILD-INFO.txt` の `script=` に出る）。

### 2.4 helper の窓の作り

`crates/shiori-host32-helper/src/main.rs` には `windows_subsystem` が無い＝コンソールの作り。`crates/shiori-host32-host/src/process_host.rs` 284〜293 行目が `CREATE_NO_WINDOW` を付けて起こす（「GUI の areka.exe から窓の指定なしで起こすと新しいコンソール窓が開き、利用者が閉じると補助プロセスが殺される」とコメントにある）。**わざとコンソールの作りで、それで正しい**。要件 1.5 のとおり判定にかけない。

### 2.5 差の記録 `doc/ssp-mcp/bridge-diff-areka.md`（110 行）

| 件 | 今の行 | 今書いてあること |
|---|---|---|
| 4（202） | 4 節 90 行目「応答の上限を越えたときに受け口へ届くもの」 | 「`id` を持つ要求に受け口が 202 を返したときは、上限まで待って `timed out` のエラーになる」と書いてあるが、テストの欄は「テストなし（rmcp の決まり…）」 |
| 5（5 秒） | 1 節 42 行目「終わり」 | 振る舞いは書いてある。テストの欄の終わりに「（5 秒の上限そのものはテストなし＝rmcp の決まり）」 |
| 5（5 秒） | 「残る縛り」102 行目 | 「テストで固定していない」の並びに「標準入力が閉じた後の 5 秒」が入っている |
| 7 | 4 節 96 行目「読まれない標準出力の見切りが効かない場合」 | 条件（壊れた行への答えの書きが詰まる）・見切りが効かないこと・Desktop は壊れた行を送らないこと、は書いてある。「中継が終わらない」という見え方と「相手が標準出力を読まない」という条件の言い方は要件 7.1 より薄い |
| 8 | どこにも無い | 4 節 78 行目「探索の待ち」と 3 節 65 行目「返事をしない相手をどれだけ待つか」が近い行 |
| 6 | 4 節 95 行目「標準エラーの記録」 | 「読めない `AREKA_MCP_PORT` の 1 件だけは、areka 本体と同じ関数が出す日本語の文のまま」 |

文書の頭（9 行目）に「areka の中継だけにある決まり 22 行（4 節）」と行の数が書いてある。4 節に行を足すとこの数も変わる。

## 3. 件ごとの対応と穴

| 件・要件 | 今あるもの | 穴・制約 | 印 |
|---|---|---|---|
| 1／要件 1.1〜1.4 | `Read-PeInfo`（オプションヘッダーの位置まで求めている）・`Test-ZipContent` の 5（PE を読む所）・CPU 種別ごとの繰り返し | Subsystem を読んでいない。読むのはオプションヘッダーの先頭から 68 バイト目の 2 バイト（PE32 と PE32+ で同じ位置。2＝GUI、3＝コンソール） | 足りない |
| 1／要件 1.5 | helper と `pasta.dll` は同じ 5 の繰り返しに入っている | 窓の作りの判定は `areka.exe` と中継だけに当てる。同じ繰り返しの中で名前で絞るか、別の短い繰り返しにする | 制約 |
| 1／要件 1.6 | 総括の行 `content checks 1-8 all passed`・端末の行 `machine ..., imports ...`・否の文の番号 | 新しい判定に番号を振ると総括の行の字が変わる。今の 1 行に足すと今の行の形が変わる（設計で決める＝6 節の 1・2） | 制約 |
| 1／要件 1.7 | 否のときの出力の形は決まっている | コンソールの作りの中継が入った zip を作る口が無い（6 節の 4） | 要調査 |
| 2／要件 2 | 偽の受け口 1 ファイルを 2 つの束が `#[path]` で読む | `fake_server.rs` の中の `mod tests` が両方の束で走る。自己確認のテストが使うのは公開の型（`Script`・`Reply`・`FakeServer`）と `MCP_PATH`・`SERVER_NAME`・rmcp の型だけ（非公開の関数は使っていない）ので、片方の束へ移すのは容易 | 足りない |
| 3／要件 3 | `scan.rs` の `open` と `http.rs` の `post` の頭 | 同じ 4 行の形が 2 か所。違いは外れたときの包み方だけ（`io::Error` のまま／`HttpError::Connect` で包む）。`io::Result<TcpStream>` を返す 1 つの関数にして、送り手は `map_err(HttpError::Connect)` で包めば記録・答えは変わらない | 足りない |
| 4／要件 4 | 偽の受け口の `Reply::Accepted`・`Desk::start_with`（短い上限）・`Reply::Never` の同じ形のテスト | rmcp 3.5.0（`transport/streamable_http_client.rs` 1628〜1667 行目）は `id` を持つ要求の 202 を「受け取った」として、その `id` を待ちのまま残す。中継の上限（`PeerRequestOptions::with_timeout`）で `ServiceError::Timeout` になり、`No response from server: timed out after <上限>` で 1 回答える。テストは `Reply::Never` のテストの台本を `Reply::Accepted` に替えた形でほぼ足りる | 足りない（テストだけ） |
| 5／要件 5.1 | exe ごしのテストの道具（`run`・`Finished.waited`・`record`）・`Reply::Never`／`Delayed` | 5 秒は rmcp 3.5.0 の `service.rs` 1762〜1803 行目の固定の値（`QuitReason::Closed` のとき `Duration::from_secs(5)`）で、外から短くできない。中継の部品のテスト（メモリ上の管）では終了コードと `shutdown_background` を確かめられないので、終了コード 0 まで見るなら exe ごし（約 5〜6 秒かかる）。tokio の時間を止める機能（`test-util`）は今どこでも使っていない | 足りない（テストだけ） |
| 5／要件 5.2 | `one_initialize_line_then_eof_gives_one_answer_line_and_exit_code_0`・`several_lines_give_only_answer_lines_on_stdout_and_records_on_stderr`・`two_bridges_at_once_keep_their_answers_apart_and_the_cache_readable` | 最初のテストの `initialize` は受け口へ届けない（つないだときの情報で答える）。ほかの 2 本は `tools/call` を含むが、標準入力が閉じた時点でその要求がまだ途中かどうかは決まっていない。「受け口へ届けた要求が閉じた後に返る」を決まって踏むテストは無い | 不明（6 節の 9） |
| 6／要件 6.1・6.2 | 共有の関数の `warn!` 1 件・値の欄 | 文が「待ち受ける」。中継の文だけを変えるには `config.rs` を触る必要がある（呼ぶのは `config.rs` 103 行目だけ）＝棚卸㉖の約束とぶつかる。共有の文を中立にすれば `port.rs` の 2 行だけで済むが、`port_tests.rs` の 3 か所の期待も直す | 制約 |
| 6／要件 6.3 | 本体の `read_port_candidates` | 本体のしていることは「既定の候補で待ち受けを試す」。中立の文（例:「既定の候補を使う」）でも食い違わない | — |
| 6／要件 6.4 | 件数 1・段 `warn`・`env`／`value` の欄 | 文だけを替えれば保たれる | — |
| 7／要件 7.1 | 4 節 96 行目に近い行がある | 見え方（中継が終わらない）と条件の言い方を足す | 足りない（文書だけ） |
| 8／要件 7.2 | 完了した spec の `tasks.md` 266 行目に既知の限界として書いてある | 差の記録に行が無い。受け口の 30 秒は hyper の頭を読む上限（`areka-mcp` の `server.rs` が `TokioTimer` を渡す）から来る | 足りない（文書だけ） |
| 7.4 | 2.5 の表の 3 か所 | 件 4・5 のテストの名前を載せ、「テストなし」「固定していない」の言い方を直す | 足りない（文書だけ） |
| 8.1 | `exe_test.rs` の作法 | 件 5 の exe ごしのテストは偽の受け口の番号を渡すので満たせる。件 6 を exe ごしに確かめると `AREKA_MCP_PORT` に読めない値を渡すことになり、字面とぶつかる（入力が空なら探索は走らず机の番号には触れないが） | 制約 |
| 8.2・8.3 | 変異の作法（`target\` へ控え・素のコピーで戻す・sha256） | 件 4 は中継の側に当てられる変異がある（`http.rs` の `classify` の 202 の枝・`service.rs` の `classify` の `Timeout` の枝）。件 5.1 の「5 秒で切る」は rmcp の決まりで、中継の側の変異（`main.rs` の `select` や `shutdown_background` を変える）では結果が変わらない見込み（6 節の 10） | 制約 |
| 8.5 | 兄弟のテストファイル・1,000 行 | `service_failure_tests.rs`（229 行）・`exe_test.rs`（568 行）はどちらも余裕がある | — |
| 8.6 | 新しい外部のクレートなし | 件 6 の記録をテストで確かめるなら、ワークスペースの中の `log-capture-kit` を中継の開発用の依存に足すことになる（外部ではない） | 制約 |

## 4. 実装の案

### 案 A: 今あるファイルを伸ばす（新しいファイル 0 本）

| 件 | 触る所 |
|---|---|
| 1 | `Read-PeInfo` が `Subsystem` も返す（`& $u16 ($opt + 68)` の 1 行）。`Test-ZipContent` の 5 の繰り返しの中で `areka.exe`・`areka-mcp-bridge.exe` だけに判定を当て、値を 1 行出し、2 でなければ否の文を足す |
| 2 | `fake_server.rs` の末尾の `mod tests` を外し、片方の束（`lib.rs` か `exe_test.rs`）だけが `#[path = ".../fake_server_tests.rs"]` で読む。テストの頭は `use super::*;` から `use crate::fake_server::*;` と `areka_mcp` の 2 つの定数へ |
| 3 | `http.rs` に `pub(crate) async fn connect(port, wait) -> io::Result<TcpStream>` を置き、`scan.rs` の `open` を消してそれを使う。送り手は `map_err(HttpError::Connect)` |
| 4 | `service_failure_tests.rs` に 1 本（`Reply::Accepted`・短い上限・答えは 1 回・次の `ping` に答える・本当の要求は 1 回） |
| 5 | `tests/exe_test.rs` に 1 本（偽の受け口が `tools/call` に答えない／5 秒より後に答える・1 行流して閉じる・終了コード 0・標準出力は空・待ったのは 5 秒以上 30 秒未満・`[bridge] finished` が出て 6 秒の見切りの行は出ない） |
| 6 | `port.rs` 46・67 行目の文を中立に（例:「既定の候補を使う」）。`port_tests.rs` の期待の 3 か所も直す |
| 7・8・7.4 | 差の記録の 4 節 96 行目を書き直し、件 8 の行を足し、42・90・102 行目を直す |

- よい所: 新しいファイルが無い。どの件も今の形の延長で、読み手が迷わない。`config.rs` に触れない。
- 気になる所: 件 6 は共有の文を変えるので、areka 本体の記録の文も変わる（意味は同じ）。`port_tests.rs` は境界の外（5 節）。件 1 の出力の形の変え方は要件 1.6 との兼ね合いで決める。

### 案 B: 新しい部品を作る

| 件 | 新しく作るもの |
|---|---|
| 1 | Subsystem を読む関数を `Read-PeInfo` と別に作る（例: `Read-PeSubsystem`）。zip の判定の外に独立の段を足す |
| 3 | 接続の関数だけの小さな部品（例: `src/net.rs`）を作り、探索と送り手の両方から使う |
| 5 | 終わり方のテストを新しい兄弟ファイル（例: `src/service_shutdown_tests.rs`）にメモリ上の管で書く |
| 6 | 共有の関数に「誰が読むか」を渡す引数を足し、中継と本体で文を分ける |

- よい所: 件 6 で中継と本体の文を正確に分けられる。件 3 の置き場がはっきりする。
- 気になる所: 件 6 の引数は `config.rs`（触らない約束）と本体の `main.rs` を触る。件 1 で段を足すと「ほかの段の振る舞い・出力の形を変えない」（要件 1.6・境界）に当たりやすい。件 5 をメモリ上の管で書くと終了コード 0 を確かめられず、5 秒も実時間で待つので得が無い。ファイルが増える。

### 案 C: 組み合わせ

案 A を基本にして、設計で次だけを選び直す:
- 件 6 は、同じウェーブの `areka-P0-user-data-root` と相談がつけば `config.rs` の 103 行目だけを触って中継の文を分ける（中継の文は英語の `[bridge]` の行にそろえられる）。つかなければ案 A（共有の文を中立に）。
- 件 2 で残す束に、自己確認のテストを `#[path]` の兄弟として読むために新しいファイルは作らない（今の `fake_server_tests.rs` をそのまま使う）。

- よい所: 件 6 の答えを「本質の形」で選べる。ほかは案 A と同じく小さい。
- 気になる所: 件 6 を `config.rs` で直すなら、並走の spec との調整が要る（約束の外で触るファイルとして挙げる）。

## 5. 境界の食い違い（要件は直さない・設計で扱う）

1. **件 6 を共有の側で直すと `crates/areka-mcp/src/port_tests.rs` も直す**。要件の境界は `port.rs` の「記録の 2 行」だけを挙げている。期待の文（`既定の候補で待ち受ける`）を確かめる 3 か所があり、文を変えると赤になる。同じクレートの兄弟のテストの期待の付け替えなので、設計で「境界の 2 行に付いて直すもの」として挙げるのが自然。
2. **件 6 を中継の側だけで直す道は `config.rs` を通る**。中継が共有の関数を呼ぶのは `config.rs` 103 行目だけで、`config.rs` は同じウェーブの `user-data-root` が持つ（触らない約束）。
3. **差の記録の頭の行の数（9 行目「22 行（4 節）」）と、4 節 95 行目「標準エラーの記録」の行**は、要件の境界の行（件 4・5・7・8）の外。件 8 を新しい行にすれば 9 行目の数が変わり、件 6 で文を変えれば 95 行目の言い方（「日本語の文のまま」）が古くなりうる（中立の日本語の文にするなら古くならない）。
4. **件 5 の exe ごしのテスト**は「テストする所と同じ場所の兄弟」（要件 8.5）の読み方による。終わり方は `main.rs` と rmcp が持ち、`main.rs` に兄弟のテストファイルは無い。exe ごしのテストの置き場は今も `tests/exe_test.rs`（終わり方の今のテスト `stdin_closed_while_stdout_is_never_read_exits_with_code_0_by_the_bound` もここ）なので、ここへ足すのが今の作法に合う。

## 6. 設計で決めること（要件の議論へ渡す）

1. **件 1 の判定の番号**: 新しい「9」として足す（総括の行が `content checks 1-9 all passed` に変わる）か、今の「5（PE を読む所）」の中に入れる（総括の行は変わらず、否の文は `5 wrong subsystem: ...` の形）か。要件 1.6「ほかの項目・出力の形を変えない」との兼ね合い。
2. **件 1 の端末の行**: 今の `{名前}: machine 0x...., imports ...` の行の後ろに足す（2 つの exe だけ行の形が変わる）か、別の 1 行（例: `areka-mcp-bridge.exe: subsystem 2 (windows gui)`）を出すか。
3. **件 1 で `SCRIPT_VERSION` を上げるか**（前例は中継を足したときの 2.1.0。`BUILD-INFO.txt` の `script=` が変わる）。要件 1.6・境界の「版の扱い」との兼ね合い。
4. **件 1 の 1 回の確かめ（要件 1.7）のやり方**:
   - (a) `main.rs` の 1 行目を一時的に消して（控えを `target\` に取る）`tools/package.ps1 -Arch x64` を通しで走らせ、段「x64 content check」の否を記録して戻す。i686 の helper と x64 の本体・中継の release の組み立て・ライセンスの検査・謝辞の生成・検体の展開まで走るので時間が掛かる（このワークツリーの `target\package` は空なので最初から組む）。検体の展開は `target/nar-samples/manual/<検体>/` を消して作り直す（実機の走りと重ねない）。開発者へ所要時間を先に伝える。
   - (b) 出来た zip の写しの中の中継の Subsystem の 2 バイトを 3 に書き換え、判定の関数だけを当てる。ただし `package.ps1` には zip を 1 つ渡して判定だけを走らせる口が無く、関数を外から呼ぶには工夫が要る（口を足すのは境界の外）。
   - (c) debug で建てた中継（`debug_assertions` が立つのでコンソールの作り）を写しの zip へ差し込んで (b) と同じに当てる。(b) と同じ制約。
5. **件 2 で残す束**: 部品のテストの束（`lib.rs`）か、通しのテストの束（`exe_test.rs`）か。どちらでも同じに走る。テストの道筋の名前は `fake_server::tests::<関数名>` から `<残す側の部品名>::<関数名>` に変わる（関数の名前は変わらない）。要件 2.3 の「テストの名前」を関数の名前と読むか、道筋の名前と読むか。
6. **件 3 の関数の置き場**: `http.rs`（探索が既に `http.rs` の `verify`・`OwnHttp` を使っている＝下の層）に置くか、`scan.rs` の `open` を残して送り手から使うか。
7. **件 6 の直し方**: (a) 共有の 2 行を中立の文にする（例:「既定の候補を使う」。括弧の「待ち受けないなら 0」は中継では「探さないなら 0」なので、両方に合う言い方にする）＋ `port_tests.rs` の期待を直す／(b) 中継の文を分ける（`config.rs` を触る＝`user-data-root` との調整が要る）。中継の記録は英語の `[bridge]` の行で、この 1 件だけが日本語の `[mcp]` の行である点もあわせて決める。
8. **件 6 をテストで確かめるか**: 要件 6 はテストを求めていない。確かめるなら (a) `port_tests.rs` で共有の文に「待ち受ける」が無いことを見る、(b) 中継の側で `Config::from_values` の記録を拾う（`log-capture-kit` を開発用の依存に足す。ワークスペースの中のクレートで外部ではない）、(c) exe ごし（要件 8.1 の字面とぶつかる）。
9. **件 5 の 2（5 秒以内に返る）を今のテストで「判定されている」と見るか**: 今のテストは「受け口へ届けた要求が、閉じた時点でまだ途中」を決まって踏んでいない（3 節の表）。新しく 1 本足すなら、偽の受け口が `tools/call` に 1〜2 秒遅れて答える形で、件 5.1 のテストと並べて書ける。
10. **件 5.1 の変異（要件 8.2）**: 5 秒で切るのは rmcp の決まりで、中継の側で当てられる変異は見当たらない（`main.rs` の `select` を外す・`shutdown_background` を `drop` に替える、などでは終わり方が変わらない見込み）。「当てられる変異なし」と理由を記録するか、設計の段で `main.rs` の終わり方を読み直して当てられる変異を探すか。件 4 は `http.rs` の `classify` の 202 の枝を変えれば赤になる見込み。
11. **件 7 の行**: 4 節 96 行目を書き直すか、行を足すか（足すと 9 行目の数が変わる）。
12. **件 8 の行の置き場**: 4 節「探索の待ち」（78 行目）に足すか、新しい行にするか、3 節 65 行目「返事をしない相手をどれだけ待つか」に書くか。新しい行なら 9 行目の数（境界の外）も直す。

## 7. 規模と危険

- **規模: S**（1〜3 日）。どの件も今の形の延長で、新しい依存も新しい境目も無い。タスクは 6〜10（brief の見立てのまま）。
- **危険: 低**。中継の振る舞いは変えない（件 2・3 は付け替えだけ、件 4・5 はテストだけ、件 7・8 は文書だけ）。気を付ける所は 3 つ:
  - 件 5 のテストは実時間で約 5〜6 秒待つ。全体テストの負荷で時間の幅が外れないよう、今の `stdin_closed_while_stdout_is_never_read_exits_with_code_0_by_the_bound` と同じく幅を広めに取る（5〜15 秒など。30 秒は確実に下回る）。
  - 件 4 のテストは上限の後に rmcp が受け口へ `notifications/cancelled` を送る（4 節 90 行目）。「本当の要求は 1 回」を数えるときは、方法の名前（`tools/call`）で数える（今の `Reply::Never` のテストと同じ）。
  - 件 1 の確かめを (a) で行うなら、release の組み立て一式で時間が掛かり、検体の展開が実機の走りの木を消す。

## 8. 調べること（設計へ持ち越す）

- **PE の Subsystem の位置と値**: オプションヘッダーの先頭から 68 バイト目（PE32・PE32+ 共通）・2＝Windows GUI・3＝Windows コンソール。Microsoft の PE の書式の文書で確かめてから書く。
- **rmcp 3.5.0 の 202 と上限**: `id` を持つ要求の 202 で待ちが残り、上限で `Timeout` になること（`transport/streamable_http_client.rs` 1628〜1667 行目）。上限の後に送る `notifications/cancelled` が、偽の受け口の記録にどう載るか（テストの数え方に関わる）。
- **rmcp 3.5.0 の 5 秒**: `service.rs` 1762〜1803 行目。応答が 5 秒を越えたときに `timed out draining in-flight responses` を `warn` で出す（中継の既定の絞り `rmcp=error` では出ない）。
- **件 5.1 で中継の側に当てられる変異の有無**（6 節の 10）。
- **件 1 の確かめの所要時間**（6 節の 4 の (a) を選ぶとき）。

## 9. 設計の段（2026-10-16・`/kiro-spec-design -y`）

### 9.1 調べ方と確かめたこと

- **調べ方**: 軽い調べ（今ある仕組みの延長）。外の調べは要らなかった（新しい依存なし）。読んだもの: 中継の `lib.rs`・`main.rs`・`scan.rs`・`http.rs`・`service_tests.rs`（道具）・`service_failure_tests.rs`・`tests/common/fake_server.rs`・`fake_server_tests.rs`・`tests/exe_test.rs`、`crates/areka-mcp/src/port.rs`・`port_tests.rs`、`tools/package.ps1` の `Read-PeInfo`・`Test-ZipContent`・説明と較正値、`doc/ssp-mcp/bridge-diff-areka.md`。
- **rmcp 3.5.0 の 202**（`c:\rust\cargo\registry\src\index.crates.io-*\rmcp-3.5.0\src\transport\streamable_http_client.rs` 1628〜1667 行目）: `Accepted` の応答は `awaits_stream_response` になり、その `id` は待ちの表から外されない。中継の上限で `Timeout` になる（8 節の見立てどおり）。
- **rmcp 3.5.0 の 5 秒**（同 `src/service.rs` 1762〜1803 行目）: `QuitReason::Closed` のとき `Duration::from_secs(5)` で途中の応答を待つ（`Cancelled` は 2 秒）。外から変えられない。
- **PE の Subsystem**: オプションヘッダーの先頭から 68 バイト目の 2 バイト。PE32 は BaseOfData（4）と 4 バイトの ImageBase、PE32+ は BaseOfData なしで 8 バイトの ImageBase なので、どちらも 68 で同じ。2＝`IMAGE_SUBSYSTEM_WINDOWS_GUI`、3＝`IMAGE_SUBSYSTEM_WINDOWS_CUI`（Microsoft の PE の書式の文書の定義）。
- **偽の受け口の自己確認**が使う項目は公開の型（`FakeServer`・`Script`・`Reply`・`Conn`。欄も公開）と `MCP_PATH`・`SERVER_NAME` だけ。非公開の定数（`WAIT` など）は使っていない。
- **偽の受け口の止め方**: `stop` は自分の tokio を落とすので、`Reply::Delayed` で眠っている仕事は待たずに消える（件 5 のテストで 12 秒の遅れを使っても、テストの終わりが延びない）。
- **記録の文を読むもの**: 「既定の候補で待ち受ける」を引くのは `port.rs`・`port_tests.rs` と、この spec の文書だけ（完了した spec の文書を除く全体を grep）。文を変えても壊れる読み手は無い。
- **確かめの記録の前例**: 完了した `areka-P0-mcp-stdio-bridge` の `verification/package-red-runs.md`。

### 9.2 6 節の決めごとの答え

| 6 節 | 決めたこと | 理由 |
|---|---|---|
| 1 判定の番号 | 項目 5 の中（否の文は `5 wrong subsystem: ...`） | 9 にすると総括の行 `content checks 1-8 all passed` が変わり、要件 1.6 に当たる。項目 5 は「PE を読む所」 |
| 2 端末の行 | 今の `machine ..., imports ...` の行は変えず、直後に `{名前}: subsystem {値}` を 1 行 | 今の行の形を変えない（要件 1.6） |
| 3 `SCRIPT_VERSION` | 上げない | 版の扱いは境界の外 |
| 4 1 回の確かめ | (a) `main.rs` の 1 行目を一時的に消して `package.ps1 -Arch x64` を 1 回。同じ走りに `areka.exe: subsystem 2`（合格の道筋）も出るので、緑の走りはもう回さない | (b)(c) は判定だけを走らせる口が無く、口を足すのは境界の外。負荷を 1 回に抑える |
| 5 残す束 | 部品のテストの束（`src/lib.rs` に兄弟の宣言 `fake_server_tests`） | 自己確認は部品のテストで exe を起こさない。要件 2.3 は関数の名前で数える |
| 6 接続の関数の置き場 | `http.rs` の `pub(crate) async fn connect` | 探索は既に `http.rs` を使う（下の層）。向きが今と同じ |
| 7 件 6 の直し方 | (a) 共有の 2 行を中立の文「areka の MCP の口の番号に既定の候補を使う」に＋`port_tests.rs` の期待 | (b) は `config.rs`（触らない約束）を通る。中立の文は本体（待ち受けを試す）にも中継（探す）にも合う |
| 8 件 6 のテスト | (a) `port_tests.rs` で「既定の候補を使う」があり「待ち受ける」が無いことを見る | 開発用の依存を足さない・exe ごしにしない（要件 8.1） |
| 9 件 5.2 | 今のテストは判定していないと見て、1 本足す（`Reply::Delayed(1 秒)`） | 今のテストは「届けた要求が閉じた時点で途中」を決まって踏まない |
| 10 件 5 の変異 | `main.rs` の `EXIT_WAIT` を 0 にすると 5.1（5 秒より前に終わる）と 5.2（応答の前に終わる）が赤。5.1 の「30 秒を待たない」の側は rmcp の決まりで、中継の側に変異が無い旨を記録する。件 4 は `http.rs` の `classify` の 202 の枝を `NoResponse` に替える | 要件 8.2 |
| 11 件 7 の行 | 4 節 96 行目を書き直す（行は足さない） | 今の行が同じ件を書いている |
| 12 件 8 の行 | 4 節に新しい行（96 行目の後）。頭の数を 22 → 23 | 限界は 7 と並べて読める所へ。「探索の待ち」は待ちの長さの行で、限界とは別 |

### 9.3 まとめる・借りる・削る

- **まとめる**: 件 2・3 はどちらも「同じものを 1 つにする」だけで、新しい抽象は作らない（`connect` は 1 つの関数・テストの宣言は 1 行の移し替え）。
- **借りる**: 件 1 は今の `Read-PeInfo` と項目 5 の繰り返し、件 4 は `Reply::Accepted` と `Reply::Never` のテストの形、件 5 は `run_bridge`・`Reply::Delayed`・`Finished::waited`、件 6 は `port_tests.rs` の `capture`。新しい道具は 0。
- **削る**: 較正値への名前の表の追加（境界の外・値は変わらない）・`SCRIPT_VERSION` の上げ・中継の側の記録の文のテスト・配布スクリプトの判定だけを走らせる口・緑の確かめの走り、はどれも取らない。
- **使ったスキルの手引き**: `kiro-spec-design` の `rules/`（設計の原則・軽い調べ・まとめ方・審査の門）。
