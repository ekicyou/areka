# ギャップ分析（audit の結果）: audit-shiori-host32

> 2026-11-13・`/kiro-validate-gap`。この spec では、ギャップ分析が audit そのものである（2026-10-19 開発者「原則、ギャップ分析でauditを実施して、設計以降で修正」・要件 1）。物差しは `/ponytail:ponytail-audit`（プラグイン 5.3.3）。audit の作法のうち「最大 20 件」と「読まなかった所」は外した（要件 1.3・1.4）。この文書は分析と案だけを書き、決めは要件の討議に残す。

## 1. 前提と測り直し（要件 1.2）

- ブランチ `claude/kiro-start-audit-shiori-host32-9dbea3` の基点は、今の `main` の先頭 `4863cb66`（棚卸㊱・PR#441）と同じ＝`main` は取り込み済み。棚卸㊱の測り（main `069f1a48`）から、範囲の 5 クレートは変わっていない。
- 範囲は要件の 5 クレートだけ（`shiori-abi`・`shiori4-testdll` は外した＝要件の Introduction。外すことの確かめは議題 1）。
- 数え方は brief と同じ（ファイル名が `_tests.rs`・`tests/` の下・`_test_support.rs` のものをテストに数える）。ファイルの中の `#[cfg(test)] mod tests` は本番の側に数えたうえで、下の「中のテスト」の欄に別に書いた。

| クレート | 本番のコード（ファイル数／行） | テスト（ファイル数／行） | .rs でないファイル | 中のテスト（`#[cfg(test)]` 以降の行のおよそ） |
|---|---|---|---|---|
| `shiori-host32-host` | 12／4,792 | 13／4,030 | `Cargo.toml`・`README.md` | `client.rs` 515・`shiori3.rs` 522・`process_host.rs` 303・`lifecycle.rs` 296・`parent_window.rs` 276・`error.rs` 62・`job.rs` 42 |
| `shiori-host32-helper` | 2／1,408 | 6／1,201 | `Cargo.toml` | `shiori_proxy.rs` 376・`main.rs` 20 |
| `shiori-host32-ipc` | 1／651 | 1／128 | `Cargo.toml` | `lib.rs` 207 |
| `shiori-host32-testdll` | 1／405 | 0／0 | `Cargo.toml` | `lib.rs` 129 |
| `shiori-host32-testdll-loadu` | 1／221 | 0／0 | `Cargo.toml` | `lib.rs` 82 |
| **計** | **17／7,477** | **20／5,359** | 6 | — |

- .rs は 37 本・12,836 行（棚卸㊱の「外すと 37 本・12,836 行」と一致）。
- 1,000 行に近いのは `client.rs` 957 行・`shiori3.rs` 879 行・`shiori_proxy.rs` 819 行。どれも半分ほどが中のテストなので、足すときは中のテストを兄弟のファイルへ出せば余裕ができる（要件 5.1）。

## 2. 読み方（要件 1.1・1.4）

範囲を 3 つに分け、3 つのサブエージェントが受け持ちのファイルを 1 行残らず読んだ。範囲の外のファイルは、指摘の確かめと「使われていないか」の grep のためだけに見た。

| 受け持ち | 読んだファイル | 行 |
|---|---|---|
| 1 | host の `client.rs`・`shiori3.rs`・`shiori2.rs`・`charset.rs`・`request_headers.rs`・`error.rs`・`lib.rs`・`terminator.rs`・`job.rs` と、`charset_tests.rs`・`client_shiori2_tests.rs`・`request_headers_tests.rs`・`shiori2_tests.rs`・`shiori3_charset_tests.rs`・`terminator_tests.rs` | 4,694 |
| 2 | host の `Cargo.toml`・`README.md`・`lifecycle.rs`・`parent_window.rs`・`process_host.rs`・`process_host_console_tests.rs`・`process_host_image_tests.rs`・`tests/` の 5 本 | 4,128 |
| 3 | helper の全 9 本・ipc の全 3 本・試験用の DLL 2 つの全 4 本 | 4,105 |

範囲の 43 ファイル（.rs 37 本＋`Cargo.toml` 5 本＋`README.md`）は全部読んだ。**読まなかった所は無い。** 主な指摘は、まとめ役（このギャップ分析の書き手）がその場所のコードを開いて確かめた（件 1・件 2・応答の長さ・再入・job の設定・未使用の依存・公開の型の使い手）。

## 3. この範囲がしていること・想定した負荷

- x64／arm64 の areka は 32bit の SHIORI の DLL を自分では読めない。そこで i686 の補助プロセス（helper）を起こし、helper が DLL を読み、両者は窓のメッセージ `WM_COPYDATA` で要求と応答をやり取りする。host は helper の起動・job への割り当て・親の窓・SHIORI/2.x と 3.0 の要求の組み立てと応答の読み取り・文字コードの取り決め・正常終了と強制終了を持つ。ipc はメッセージの形、試験用の DLL 2 つはテストの相手役。
- 想定した負荷: 1 台の PC の 1 人の利用者。32bit のゴーストは同時に数体まで。1 体あたり要求はおおむね毎秒 1 回（`OnSecondChange` など）。

## 4. 指摘の一覧（通し番号・全 48 件）

各指摘の「分類」は次のどれか。
- **(a) 今直す**
- **⑴ 候補**＝開発者の裁定（完了した spec の要件を含む）に反するかもしれない直し
- **⑵ 候補**＝わざと置いた先送りの印
- **⑶ 候補**＝SSP との互換や ukadoc の正典に関わる振る舞いの変更
- **(b) 申し合わせ**＝同じウェーブの spec と申し合わせが要る（`shiori-host32-host` の公開の形・`Cargo.lock` の席）

「公開の形」は、範囲の外から呼べる型・関数・欄の名前と形のこと（要件 1.7・6.4）。

### 必ず直す（3 件）

**1. helper が SHIORI の DLL を、依存の探し先を絞らずに読む**（`crates/shiori-host32-helper/src/shiori_proxy.rs:97`・取り込み `:59`・説明 `:10-11`・`:79-84`・`:110`・`:177-181`）＝**件 1**
- 何が起きるか: 素の `LoadLibraryW` で読むので、DLL が頼る DLL は「helper の exe のフォルダ（areka の置き場）→ System32 など → 作業フォルダ（ゴーストのフォルダ）→ `PATH`」の順に探される。`PATH` のどこかや areka の置き場に同じ名前の DLL があると、それを読んでしまう。同じプロセス内の読み込み（`crates/areka-ghost/src/shiori_inproc.rs:102-108`）は `LoadLibraryExW` と `LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS` に直してあり、2 つの道で探し方が違う。
- 直し方: `:97` を同じ形の `LoadLibraryExW` に替え、説明を直す。渡す道は本番では絶対の道（`main.rs:329` の `load_dir.join(shiori_name)`・`load_dir` は `areka-ghost/src/shiori_wiring.rs:278-284` が絶対にする）なので、この旗の前提を満たす。テストは同じプロセス内の `a_relative_dll_path_is_refused`（`shiori_inproc_tests.rs:175-208`）を helper の側へ写す（相対の道は読めない・絶対の道は読める＝`LoadLibraryW` に戻すと赤）。
- 公開の形: 変わらない。分類: (a)。詳しくは 5 節。

**2. helper が親の `PASTA_DEBUG` をそのまま受け継ぎ、pasta が 127.0.0.1:9276 を取り合う**（`crates/shiori-host32-host/src/process_host.rs:262-275`・起こすのは `:293`）＝**件 2**
- 何が起きるか: `spawn` は `.arg` 3 つ・`.env` 3 つ・`.current_dir` を足すだけで、環境を消しも上書きもしない。開発者の環境に `PASTA_DEBUG=1` があり、SSP などで pasta のゴーストが 9276 を握っていると、helper の下の pasta が LOAD に失敗する（ack [0]）。
- 直し方: `spawn` の中で `.env("PASTA_DEBUG", "0")` を足す。`spawn` の `Command` の組み立てを内側の関数（例 `helper_command(..) -> Command`）へ出し、`get_envs()` で確かめるテストを置く（手本は `crates/real-run-kit/src/launch_tests.rs:8-30`・`:54-55`）。
- 公開の形: 変わらない（内側の関数は外へ出さない）。分類: (a)。開ける口を設けるかは議題 2。

**3. テストの一時フォルダがワークツリーの `target\` の下に無い**（host: `process_host.rs:553`・`:657`・`:663-664`、`tests/error_paths.rs:122-124`、`tests/lifecycle_cyclic_e2e.rs:179-189`、`tests/lifecycle_kill_e2e.rs:169-177`、`tests/shiori_load_e2e.rs:149-157`、`tests/shiori_request_e2e.rs:170-180`／helper: `shiori_proxy.rs:625-636`・`:764-775`、`main_loopback_tests.rs:88-98`、`shiori_proxy_loadu_tests.rs:205-213`／`shiori-host32-testdll-loadu/src/lib.rs:160-167`）
- 何が起きるか: どれも `std::env::temp_dir()`（`%TEMP%`）の下に `shiori.dll` の写しや `.bat` を作る。決まり「実機の根・一時フォルダはワークツリーの `target\` の下だけ」（要件 4.3）に反する。後始末は成功したときだけで、さらに `shiori_load_e2e.rs:219-220`・`:251`・`:268` と `shiori_request_e2e.rs:260-264` は `terminate()` の直後に `remove_dir_all` を呼ぶ。プロセスの終わりは非同期なので DLL がまだ読み込まれていて消せず、そのエラーは捨てられる＝回すたびに `%TEMP%` に残りがたまる。
- 直し方: 置き場を `target\` の下（結合テストは `CARGO_TARGET_TMPDIR`・単体テストは `CARGO_MANIFEST_DIR` から辿る `target\tmp`）にまとめる。helper にはもう `copy_testdll_into_unique_target_dir`（`shiori_proxy.rs:678-695`）がある＝これに寄せる。消す前に helper の終わりを待つ（`status()` が `Exited` になるまで）。`temp-path-kit` を dev-dependency に足すと `Cargo.lock` が変わるので取らない（要件 5.6）。
- 公開の形: 変わらない。分類: (a)。指摘 21（テストの道具の重複）と一緒に直すのが小さい。

### 直したほうがよい（18 件）

**4. helper の失敗の理由が、本番では areka の記録に残らない**（helper `main.rs:280`・`:288`・`:339`・`:395`・`:400`、`shiori_proxy.rs:273`・`:468`／host `process_host.rs:284-293`）
- 何が起きるか: helper は失敗を `eprintln!` で書くだけ。host は helper の標準入出力に手を付けずに起こし、`CREATE_NO_WINDOW` なので、本番の `areka.exe`（窓のアプリ）では行が捨てられる。areka の記録に残るのは「LOAD が成功 ack [1] を返さなかった: [0]」（`shiori_wiring.rs:183-186`）だけで、`EntryNotFound("request")`・`LoadLibraryFailed(0x800700C1)`・`LoadReturnedFalse` の区別がつかない。MCP の `get_log` にも出ない。areka が `real-run-kit` の下で標準入出力をパイプにされると、helper はそのパイプを受け継いで握り続ける（`job.rs:11-12` の説明の詰まり）。
- 直し方: `spawn` で helper の標準エラーをパイプにし、helper 1 つにつき読み手のスレッド 1 本で行を `tracing` の記録へ回す（読み手が無いとパイプが詰まって helper が止まるので必須）。行の文は変えない＝本体の実機の試験が `[helper] LOAD 失敗（観測・ack[0]）: LoadReturnedFalse` を探している（`crates/areka/tests/emo2_real_run/common.rs:22`）。
- 公開の形: 変わらない。分類: (a)（調べること 12 節の R2）。

**5. 32bit の道で応答が読めないとき、`error!` の記録が無く、ふつうの SHIORI のエラーに見える**（`client.rs:218-221`・`shiori3.rs:299-301`）
- 何が起きるか: 空・ごみ・状態の行が無い応答は `ShioriError::Parse` になるが、どこにも `error!` が出ない。受け手（`areka-kanade` の `actor.rs:622-635`）はこれを「SHIORI がエラー応答」として `warn` で書くので、約束破りの応答がふつうの 400 と見分けられない。同じプロセス内の道（`shiori_inproc.rs:380-387`）は `error!` を書いてから返す＝2 つの道で扱いが違う。
- 直し方: `parse_and_note`／`parse2_and_note` で読めなかったときに、応答の長さと 1 行目（短く切って）を `error!` で書く。
- 公開の形: 変わらない。分類: (a)。

**6. SHIORI のエラーの中身と、運び道の失敗の種類が記録の文に出ない**（`error.rs:59`・`:94-95`）
- 何が起きるか: 受け手は誤りを文字列にして記録する（`areka-kanade/src/shiori/real.rs:246-252`）。`SHIORI/3.0 500` と `ErrorDescription: 辞書の 12 行目に誤り` が返っても、記録は「SHIORI error status 500」だけ。`IpcError::CorruptFrame(..)` も `SendFailed` も同じ「ipc transport failure」になる。`Ipc(IpcError)` に `#[source]` が無く、誤りの鎖も切れる。
- 直し方: 表示の文に `ErrorLevel`・`ErrorDescription` と中の `IpcError` を足し、`#[source]` を付ける。文字列で比べている使い手は、この文を固定する 2 本のテスト（`error.rs:121`・`:139`）のほかに無い（grep 済み）。
- 公開の形: 型と欄は変わらない（表示の文だけ）。分類: (a)。

**7. DLL が返す応答の長さを、確かめずに信じる**（`shiori_proxy.rs:311-330`）
- 何が起きるか: `from_raw_parts(hres.0, len)` の `len` は DLL が書いた値そのまま。DLL が `len` を書き換え忘れる（要求の長さのまま）・確保より大きい値を書く・動くメモリ（`GMEM_MOVEABLE`）で返す、のどれでも、確保の外を読む・ごみが混じる・helper が落ちる。落ちると `unload` が呼ばれず、ゴーストの保存が失われる。
- 直し方: `GlobalSize(hres)` が 0 か `len` より小さければ、記録して `GlobalFree` し、`RequestFailed` を返す。テストは試験用の DLL に「`len` を大きく書く」ID を足して縛る。
- 公開の形: 変わらない。分類: (a)。

**8. DLL の呼び出しの最中に UNLOAD が届くと、helper が中断して落ちる**（`main.rs:274-293`・`:365`）
- 何が起きるか: REQUEST のあいだ `s.proxy.borrow()` を握ったまま DLL を呼ぶ（`:265` の説明は「DLL はメッセージを回さない」と置いている）。SHIORI がメッセージボックスを出す・SAORI が窓を回すと、host の要求は 60 秒で時間切れになり UNLOAD を送る。helper はメッセージを回しているので「固まった」扱いにならず、UNLOAD が呼び出しの中で処理され、`:365` の `borrow_mut()` が panic する。窓の手続きの中の panic は `extern "system"` を越えてプロセスを止める＝`unload` は呼ばれず保存が失われる。
- 直し方: 「DLL を呼んでいる最中か」の旗を置き、最中に届いた REQUEST は 500 と記録で返し、UNLOAD は「後で終える」印だけ立てて、呼び出しが戻ってから終える。少なくとも `try_borrow_mut` と記録にする。テストは自分の窓へ送る折り返しのテスト（`main_loopback_tests.rs`）で、最中の UNLOAD を作って縛る。
- 公開の形: 変わらない。分類: (a)（調べること R3）。

**9. `unload` が失敗を返しても、`FreeLibrary` が失敗しても、記録が無い**（`shiori_proxy.rs:349-356`・`:233-235`・`:252-254`）
- 何が起きるか: pasta の `unload` が保存の失敗で偽を返しても、何も書かれず、UNLOAD の返事は [1] のまま（`main.rs:375-380`）。
- 直し方: 偽の戻りと `FreeLibrary` の失敗を記録する（流れは変えない）。返事を [0] にするかは議題 9。
- 公開の形: 変わらない。分類: (a)。

**10. 子の様子を問う `try_wait` の失敗を、黙って「動いている」とみなす**（`process_host.rs:323-325`・`:337`）
- 何が起きるか: もし失敗すると、`HelperLifecycle::status()` はずっと「動いている」を返し、`request_clean_shutdown` は終わりの時間切れで誤った記録を残す。
- 直し方: 2 か所とも `error!` を書く。
- 公開の形: 変わらない。分類: (a)。

**11. 親の窓が受けた不正なメッセージに記録が無い**（`parent_window.rs:202-213`）
- 何が起きるか: 知らない種類・長さの合わないメッセージ・形の崩れた HELLO・無視する既知の種類は、数えるだけ（`Cell` の数）。数は本番では読まれない（`hellos`・`bad_frames` はテストだけ・`ignored_known` はどこも読まない）。形の崩れた HELLO は、理由なしの 5 秒のつなぎの時間切れにしか見えない。
- 直し方: そこで `warn!` を書き、数はテストだけのものにするか消す。
- 公開の形: 変わらない。分類: (a)。

**12. helper の中の、捨てられた失敗**（`main.rs:388-390`・`:255-257`）
- 何が起きるか: `let _ = PostMessageW(..)` が失敗すると helper は目を覚まさず終わらない。host は終わりの時間切れまで待って殺し、理由は残らない。`lparam` が空の `WM_COPYDATA` は記録なしで 0 を返す。
- 直し方: 両方を記録する。空の `lparam` は不正なメッセージとして数える。
- 公開の形: 変わらない。分類: (a)。

**13. job の「閉じたら全部殺す」設定が、SHIORI が起こしたプロセスまで殺す**（`crates/shiori-host32-host/src/job.rs:59-60`）
- 何が起きるか: 設定は `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` だけ。SHIORI や SAORI がほかのプログラム（メモ帳など）を起こすと、その子も同じ job に入る。ゴーストを閉じたり切り替えたりして job が閉じると、そのプログラムも殺される。SSP の下ではこうならない見込み。
- 直し方: `JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK` を足し、job に残るのを helper 自身だけにする。
- 公開の形: 変わらない。分類: (a)。外から見た振る舞いが変わるので、⑶ に当たるかを議題 5 で確かめる。

**14. `HelperHandle` に後始末（Drop）が無く、job が作れなかったときに helper が残る**（`process_host.rs:176-184`・`lifecycle.rs:255-264`）
- 何が起きるか: `shiori_wiring.rs:167-187` は HELLO の時間切れや LOAD の失敗で `Err` を返し、包む前の `HelperHandle` を捨てる。job の割り当てが失敗していると（`job.rs:50-78` は記録して先へ進む）、helper は残ってメッセージを回し続ける。
- 直し方: `HelperLifecycle` が持つ「二度呼んでもよい終わらせ方」を `HelperHandle` の Drop へ移す。テストの `HelperGuard` の 3 つの写しも消せる。
- 公開の形: 型・関数・欄は変わらない（Drop の実装を足す）。`crates/areka/` は `HelperHandle` を使っていない（使い手は `areka-ghost/src/shiori_wiring.rs` と `areka-kanade/tests/kanade/real_helper_test.rs`）。分類: (a)。

**15. SHIORI の版で道を分ける所にテストが無い**（`client.rs:199-207`・`:217-222`・`:266-274`）
- 何が起きるか: 2.x と 3.0 の組み立て・読み取りは部品ごとにテストがあるが、`match self.version` でどちらを選ぶかを確かめるテストが無い（`with_version(Shiori2)` のあとに `get` を呼ぶテストはどこにも無い）。2.x の腕を 3.0 の読み手につなぎ違えると、2.x のゴーストの GET が全部「値なし」になって黙るのに、テストは全部緑のまま。
- 直し方: 版で分ける 2 か所を窓の要らない内側の関数（例 `encode_for`・`parse_for`）へ出し、腕ごとにテストする。指摘 46 の重複もこれで消える。
- 公開の形: 変わらない。分類: (a)。

**16. `spawn` そのものにテストが無く、「約束」のテストは写しを試している**（`process_host.rs:651-733`）
- 何が起きるか: このテストは `spawn` と同じ `Command` を手で組み直して `.bat` を走らせる。`spawn` から `.current_dir(load_dir)` や `.env(..)` を 1 つ消しても、単体テストは全部緑。
- 直し方: 指摘 2 の内側の関数を `get_envs()`・`get_args()`・`get_current_dir()` で確かめるテストに置き換え、写しのテストを消す。
- 公開の形: 変わらない。分類: (a)。

**17. 正常終了の失敗の道にテストが無い**（`lifecycle.rs:220-245`）
- 何が起きるか: `Unload(SendError)`・`UnexpectedAck`・`ExitTimeout` の 3 つの腕（どれも `error!`＋`Err`）を通るテストが無い。あるのは「もう終わっている」の近道と、実物の正常な終わりだけ。
- 直し方: 待つ長さを引数にとる内側の関数（`pub(crate)`）を置き、自分の窓へ送る折り返し（`parent_window.rs:670-690` の形）で 3 つの腕を短い待ちで通す。
- 公開の形: 変わらない。分類: (a)。

**18. `error_paths` の 3 本目は時間に頼り、強制終了を確かめていない**（`tests/error_paths.rs:292-351`・`:297`・`:310`）
- 何が起きるか: 親の窓を 0 にしているので、helper は HELLO を送れず自分で終わる（`main.rs:548-553`・終わりの値 2）。「まだ動いている」の確かめ（`:310`）はこの自分での終わりと競り合う。最後の「正常でない終わり」の確かめは、`terminate` が何もしなくても通る（2 でも 1 でも「正常でない」）。
- 直し方: 消す（強制終了の見つけ方は `lifecycle_kill_e2e` が本物のつなぎで確かめている）か、本物の窓を渡す。
- 公開の形: 変わらない。分類: (a)。

**19. `load` に渡す道を ANSI（CP_ACP）で組む所にテストが無い**（`shiori_proxy.rs:465-471`・テストだけの `ansi_encode` `:443-446`）
- 何が起きるか: `init_bytes(&InitEntry::Load(..))` を呼ぶテストが無い（試験用の DLL の `load` は中身を見ない）。腕を入れ替えたり UTF-8 で渡したりしても全部緑のまま。CP932 の PC で日本語のフォルダのゴーストを YAYA や華和梨が読むと、道が化ける。
- 直し方: 日本語の道について `init_bytes(Load, p) == encode_with_codepage(CP_ACP, p)` を確かめるテストを置き、`ansi_encode` を消す。
- 公開の形: 変わらない。分類: (a)。

**20. DLL が空の応答（null）を返す道にテストが無い**（`shiori_proxy.rs:322-324`・`main.rs:279-282`）
- 何が起きるか: null を返す試験用の DLL が無い。`is_invalid` の確かめを消すと null から読む未定義の動きになるのに、テストは緑のまま。
- 直し方: 試験用の DLL に null を返す ID（例 `OnTestNull`）を足し、proxy のテストで `RequestFailed`、折り返しのテストで 500 を確かめる。
- 公開の形: 変わらない。分類: (a)。

**21. テストの道具が 5 本の結合テストと helper のテストに写されている**（`resolve_helper_exe` ×5＝`error_paths.rs:85`・`lifecycle_cyclic_e2e.rs:96`・`lifecycle_kill_e2e.rs:86`・`shiori_load_e2e.rs:66`・`shiori_request_e2e.rs:87`／`resolve_testdll` ×4 と helper の `shiori_proxy.rs:578-604`＝`main_loopback_tests.rs:8-32`／`workspace_i686_target_base` ×4・`make_unique_temp_dir` ×4・`HelperGuard` ×3・定数の写し・`cmd_exit` ×2＝`process_host.rs:445`・`lifecycle.rs:375`／同じ単体テスト `process_host.rs:548-559` と `process_host_image_tests.rs:180-193`）
- 何が起きるか: 成果物の探し方や一時フォルダの置き場を変えるとき（指摘 3）、5〜6 か所を同じに直さないとずれる。
- 直し方: host は `tests/common/mod.rs`、helper はテストだけの共通のモジュールに 1 つずつ置く。重複の単体テストは 1 本にする。
- 公開の形: 変わらない。分類: (a)。

### できれば（27 件）

**22. 応答のヘッダの値の前後の全角の空白（U+3000）まで削る**（`shiori3.rs:314`）
- 何が起きるか: `str::trim` は Unicode の空白を全部削る。`Value: 　こんにちは` は `こんにちは` になり、末尾の全角の空白も消える。2.x の `Sentence`・`String` も同じ。
- 直し方: 半角の空白とタブだけを削る（または `:` の後の 1 字だけ）。
- 公開の形: 変わらない。分類: ⑶ 候補（SSP が値をどう切るかに関わる＝議題 6）。

**23. 同じ名前のヘッダが 2 つあるとき、ヘッダによって先勝ちと後勝ちが混ざる**（`shiori3.rs:213-229` は `Charset` が先勝ち・`:316-328` は `Value` などが後勝ち）
- 何が起きるか: 1 つの応答で決まりが 2 つある。めったに無いが、テストも説明も無い。
- 直し方: どちらかにそろえ、テストを 1 本置く。
- 公開の形: 変わらない。分類: ⑶ 候補（議題 6）。

**24. 正常終了の途中で helper が落ちると、誤った失敗を報告する**（`lifecycle.rs:216-226`）
- 何が起きるか: 状態の確かめと UNLOAD の送りのあいだに helper が落ちると、`Ok(終わり方)` でなく `Err(Unload(Ipc(SendFailed)))` を返し、受け手は「unload_failed」と書く。本当の理由は落ちたこと。
- 直し方: 送りが失敗したら `status()` を問い直し、終わっていれば `Ok(終わり方)` を返す。
- 公開の形: 変わらない。分類: (a)。

**25. `ExitKind::Terminated` は Windows では起きず、説明が誤っている**（`process_host.rs:139-143`・`:155-160`・偽のテスト `:489-503`）
- 何が起きるか: Windows の `ExitStatus::code()` はいつも値を持ち、`TerminateProcess(..,1)` は `Abnormal(1)` になる。説明の「TerminateProcess なら Terminated」は誤り。`classify_terminated_branch_logic` は値なしの場合を通っていない。
- 直し方: 説明を直し、偽のテストを消す。型の腕は残す（C19 は公開の形を変えない＝要件 6.4）。
- 公開の形: 腕を消すと変わる（`areka-kanade/src/shiori/real_tests.rs:444`・`:479`・`real_idle_tests.rs:48` に響く）。分類: (a)（腕は残す）。腕を消すなら (b)＝議題 12。

**26. 使い手のいない公開の関数・欄**（`poll_exit`＝`process_host.rs:319-326`・`lib.rs:49`／`HelperHandle::helper_hwnd`・`set_helper_hwnd` と欄 `helper_hwnd`＝`process_host.rs:180`・`:193-206`／`classify_failure`・`FailureClass`・`LifecycleReport`・`HelperLifecycle::report_failure`＝`lifecycle.rs:34-56`・`:80-93`・`:180-197`）
- 何が起きるか: ツリー全体を grep して、使うのはこのクレートのテストだけ（`lifecycle_kill_e2e.rs:307-318` など）。`pilot` は自分の写しを持つ。読む人と直す人の手間だけが残る。
- 直し方: 消し、それを試すだけのテストも消す。
- 公開の形: 変わる（消す）。`crates/areka/` を含めて範囲の外の使い手は無い＝響きは 0。分類: (b)（要件 6.4 は「足すほかは変えない」＝議題 7）。

**27. `request_of` は欄を並べ替えるだけの包み**（`request_headers.rs:50-66`・`lib.rs:51`）
- 何が起きるか: 欄が全部公開の `ShioriRequest { .. }` を書くのと同じ。テスト（`request_headers_tests.rs:92`）は「包みと直書きが等しい」を確かめているだけ。
- 直し方: 消して直書きにする（使い手＝`areka-ghost/src/shiori_inproc.rs:409`・`shiori_inproc_headers_tests.rs`・`client.rs` の 2 か所）。
- 公開の形: 変わる。説明に「2 つの道が同じ組み立てを共有するため（要件 2.8）」とあり、完了した spec の決めである。分類: ⑴ 候補かつ (b)（議題 7）。

**28. 使っていない依存が 3 つある**（host `Cargo.toml:16` の `event-listener`・helper `Cargo.toml:15` の `thiserror`・ipc `Cargo.toml:13` の `windows-core`）
- 何が起きるか: どのソースも使っていない（grep 済み）。組むたびに読まれ、版上げのときにも見る手間が残る。
- 直し方: 3 行を消す。ただし消すと `Cargo.lock` の各パッケージの依存の並びが変わる。
- 公開の形: 変わらない。分類: (b)（C19 の約束 ⑷＝`Cargo.lock` の席は `dola-yaml-backend` だけ・要件 5.6＝議題 8）。

**29. 試験用の DLL 2 つが `windows` の版をじかに書いている**（`shiori-host32-testdll/Cargo.toml:15-21`・`shiori-host32-testdll-loadu/Cargo.toml:15-20` の `"0.62.2"`）
- 何が起きるか: ワークスペースの版を上げると、この 2 つだけ古い版が残り、2 つの版を組むことになる（決まり「各クレートはワークスペースのピンだけ」）。
- 直し方: `workspace = true` にする（重い機能を避けたいなら、その理由を 1 行書く）。解決される版は今と同じ 0.62.2 の見込みで、`Cargo.lock` が変わらないことを設計で確かめる（R6）。
- 公開の形: 変わらない。分類: (a)（`Cargo.lock` が変わるなら (b)）。

**30. `CREATE_NO_WINDOW` を手で定義し、理由の説明が古い**（`process_host.rs:40-44`）
- 何が起きるか: 説明は「`Win32_System_Threading` を有効にしていない」と言うが、ワークスペースの `windows` の機能（ルートの `Cargo.toml:71`）で有効になっていて、同じクレートの `terminator.rs:13` はもうそこから取り込んでいる。
- 直し方: `windows` の定数を使い、手の定義を消す。
- 公開の形: 変わらない。分類: (a)。

**31. `terminate` に Unix でしか起きない腕がある**（`process_host.rs:219`）
- 何が起きるか: `InvalidInput` の腕は Unix の std の振る舞い。Windows の std は終わったプロセスにも `Ok` を返すので、この腕は通らない。
- 直し方: 腕を消す。
- 公開の形: 変わらない。分類: (a)。

**32. 古くなった説明**（`README.md:14-15`・`:90-91`／`lifecycle.rs:3-15`／helper `main.rs:31-32`・`:110-111`・`:115-116`／`shiori_proxy.rs:75-77`・`:164-166`・`:172-173`／helper `Cargo.toml:4`／ipc `lib.rs:45-47`・`:53`・`:59`／`main_response_flavor_hung_cage_tests.rs:39-40`／`main_classify_tests.rs:49-50`／testdll `lib.rs:212-213`）
- 何が起きるか: README は「SHIORI/3.0 の組み立て・文字コード・寿命は持たない」と言うが、今は持つ。依存の一覧は `encoding_rs`・`tracing` が無く、使わない `event-listener` がある。「後のタスクでつなぐ」「`request` は呼ばれない」「応答は echo」「Load／Unload は扱わない」など、済んだことを未来の話として書いている。固まりの檻は x64 で回ると書くが、`main.rs:585-589` で i686 だけにしてある。testdll の説明は「`len` が null なら null を返す」と言うが、実際は 400 の応答を返す。
- 直し方: 今の姿に書き直す。
- 公開の形: 変わらない。分類: (a)。

**33. 古くなった `allow(dead_code)`**（helper `main.rs:408-410`）
- 何が起きるか: 欄 `window` の説明は「テストでない組みでは読まれない」と言うが、`quit_requested()`（`main.rs:455-457`）がテストでない組みでも読む。`main.rs:31-32` は無いモジュール単位の allow を言っている。
- 直し方: allow と古い説明を消す。
- 公開の形: 変わらない。分類: (a)（実在の理由のある語彙の口ではないので ⑵ に当たらない）。

**34. 記録の宛名（target）がそろっていない**（`lifecycle.rs:153`・`:223`・`:229`・`:239`・`:261`・`job.rs:53`・`:67`・`:75`）
- 何が起きるか: `process_host` は `target: "shiori-host32"` で書くが、ここはモジュールの道の名前で書く。`RUST_LOG=shiori-host32=debug` にしても、これらの行は出ない。
- 直し方: `target: "shiori-host32"` にそろえる。
- 公開の形: 変わらない。分類: (a)。

**35. つなぎを待つあいだ、helper が死んでも気付かない**（`parent_window.rs:266-303`・呼び手 `areka-ghost/src/shiori_wiring.rs:170`）
- 何が起きるか: すぐ終わる helper（引数の誤りで終わりの値 2・x64 の helper など）でも、つなぎの 5 秒をまるまる待ち、誤りに終わりの値が載らない。
- 直し方: 時間切れのあとに `status()` を問い、終わり方を誤りの文に載せる（呼び手の `areka-ghost` は C19 のほかの spec と重ならない＝要件 6.1・6.2）。
- 公開の形: 呼び手の側で直せば変わらない。分類: (a)。

**36. 終わりを 5 ミリ秒ごとの眠りで見張る**（`lifecycle.rs:233-245`）
- 何が起きるか: 待つあいだ起きたり寝たりを繰り返す。
- 直し方: `WaitForSingleObject(プロセス, 上限)` の 1 回にする。
- 公開の形: 変わらない。分類: (a)。

**37. 起こしてから job に入れるまでに隙間がある**（`process_host.rs:293-296`）
- 何が起きるか: 子は `AssignProcessToJobObject` の前に動き出す。その隙間に areka が落ちると helper が残る。
- 直し方: 本当に閉じるには `CREATE_SUSPENDED` か `PROC_THREAD_ATTRIBUTE_JOB_LIST`（std の `Command` の外の生の `CreateProcess`）が要り、重い。限界を書いた `shortcut:` の注釈を置くのが小さい。
- 公開の形: 変わらない。分類: (a)（注釈を置く）。

**38. 親の窓が HELLO と RESPONSE を、誰から来ても受ける**（`parent_window.rs:189-201`）
- 何が起きるか: 同じ利用者の別のプロセスがメッセージだけの窓を見つけて（`FindWindowEx(HWND_MESSAGE)`）偽の HELLO を送ると `helper_hwnd` が上書きされ、以後の要求が相手へ行く。要求の最中に偽の RESPONSE を送れば応答を差し替えられる。送り手の窓（`wParam`）を見ていない。
- 直し方: HELLO は 1 度だけ、`GetWindowThreadProcessId(wParam)` が子のプロセス番号と同じときだけ受ける。RESPONSE は `wParam == helper_hwnd` のときだけ受ける。
- 公開の形: 変わらない。分類: (a)。

**39. 実物の pasta を使う任意のテストが、DLL が無いと黙って緑になる**（`shiori_load_e2e.rs:294-301`・`shiori_request_e2e.rs:287-293`・`lifecycle_cyclic_e2e.rs:320-327`）
- 何が起きるか: `HOST32_PASTA_DLL` が無いと標準エラーに書いて戻り、通ったことになる。
- 直し方: 黙って飛ばさないなら、無いときは `#[ignore]` で見える形にするか失敗にする。
- 公開の形: 変わらない。分類: ⑴ 候補（完了した spec の要件 R6.3／R7.6 が「任意」と決めた例外＝議題 11）。要件 4.4 が縛るのは helper と試験用の DLL の成果物で、実物の pasta ではない。

**40. 同じテストの実行ファイルの中で、窓を作るテストが並んで走るとぶつかるおそれ**（`shiori_load_e2e.rs:233`・`:292`／`shiori_request_e2e.rs:190`・`:283`）
- 何が起きるか: 説明には「1 プロセスに窓 1 つ」の制約があり、`--test-threads=1` を言うのは `lifecycle_cyclic_e2e.rs:37-50` だけ。`HOST32_PASTA_DLL` を立てて並んで走らせるとぶつかる見込み。もし制約がプロセス全体なら、本番で 32bit のゴーストを 2 体同時に動かすときにも当たる（各 SHIORI のスレッドが自分の窓を作る＝`areka-kanade/src/shiori/real.rs:514-521`）。
- 直し方: 制約の正体を確かめてから決める（R4）。
- 公開の形: 変わらない。分類: (a)（調べてから）。

**41. 警告済みの文字コード名の集まりが、説明と違って際限なく育つ**（`charset.rs:157-160`・`:255-256`）
- 何が起きるか: 鍵は前後の空白を削って小文字にしただけ。返事ごとに `Charset:` が変わる壊れた DLL だと、違う名前の数だけ文字列がたまり続ける。
- 直し方: 上限（例 64）を置き、越えたら `debug` で書くだけにする（または説明を直す）。
- 公開の形: 変わらない。分類: (a)。

**42. 置き換えた字を数えるとき、要求全体を 1 字ずつ符号化し直す**（`charset.rs:79-85`）
- 何が起きるか: 置き換えが起きた稀な時だけ、要求の全部の字（多くは ASCII）を 1 字ずつ符号化し、ASCII でない字ごとに確保する。
- 直し方: ASCII の字は置き換えが起きないので飛ばす。
- 公開の形: 変わらない。分類: (a)。

**43. job のテストが、job の外に `ping` を残すことがある**（`job.rs:100-103`）
- 何が起きるか: `cmd.exe /c ping` を起こしてから job に入れる。cmd が先に ping を起こすと、ping は job の外になり 60 秒ほど残る。テストは cmd だけを見るので通る。
- 直し方: `terminator_tests.rs:18` と同じく `ping.exe` をじかに起こす。
- 公開の形: 変わらない。分類: (a)。

**44. REQUEST ごとに、持っている Vec をもう一度写す**（helper `main.rs:133`・`:238`）
- 何が起きるか: `read_copydata` がもう持っている Vec を `payload.to_vec()` で写し直す。
- 直し方: 写さずに渡す（move）。
- 公開の形: 変わらない。分類: (a)。

**45. 使い手のいない定数 `PAYLOAD_HEADER_LEN`**（ipc `lib.rs:38`・テスト `:597-601`）
- 何が起きるか: 自分のテストと説明のほかに使い手が無い（grep 済み）。
- 直し方: 消す。ipc の公開の定数だが、使い手は host と helper だけで、どちらも使っていない。
- 公開の形: `shiori-host32-host` の公開の形は変わらない。分類: (a)。

**46. 一緒に変えなければならない写し（host の中）**（`client.rs:199-207` と `:266-274`／`encode_and_note`＝`:311` と `encode2_and_note`＝`:336`／`parse_and_note`＝`:324` と `parse2_and_note`＝`:357`／`shiori3.rs:138-161` と `shiori2.rs:150-162`）
- 何が起きるか: 版で分ける組み立ての塊が 2 つ、1 か所だけ違う関数の対が 2 組、`Reference` の並べ方と「抜けた番号」の飛ばし方が 2.x と 3.0 に写されている。抜けた番号の決まりを変えると 2 か所を同じに直す要がある。
- 直し方: client の分は指摘 15 の内側の関数で消える。組み立ての分は `Reference` を並べる共通の関数 1 つにする。
- 公開の形: 変わらない。分類: (a)。

**47. 試験用の DLL 2 つと helper に写された、HGLOBAL の決まりの写し**（`GMEM_FIXED`＝helper `shiori_proxy.rs:65`・testdll `lib.rs:19`・loadu `lib.rs:24`／応答の確保と NUL の付け方＝testdll `:242-273`・loadu `:116-136`／`alloc_input`＝testdll `:282-291`・loadu `:148-155`）
- 何が起きるか: HGLOBAL の受け渡しの決まりを変えると 3 か所を同じに直す要がある。DLL どうしで共有するには新しいクレートが要り、それは大きすぎる。
- 直し方: 互いの場所を指す 1 行の注釈を置く（新しいクレートは作らない）。
- 公開の形: 変わらない。分類: (a)。

**48. 時間に頼るテスト**（`lifecycle.rs:407`・`:456-476`／`process_host.rs:530`／`error_paths.rs:204`・`:238`・`:276`・`:306-308`・`:326-329`／`parent_window.rs:587`・`:686`／e2e の `HANDSHAKE_TIMEOUT` 5 秒）
- 何が起きるか: 「1 秒より短く終わる」「5 秒のうちに来る」「`ping -n 3` は 2 秒ほど生きている」を前提にしている。全体テストで PC が混むと揺れるおそれがある（要件 4.2）。
- 直し方: テストの的（何を確かめるか）を外さずに、経過時間の確かめが的でない所は外し、的である所は上限を広げる（決まり「揺れるテストの手当てはテストの本質を外さない」）。
- 公開の形: 変わらない。分類: (a)。

## 5. 申し送りの 2 件の確かめ（要件 7・8）と、指摘との対応（要件 1.6）

| 申し送り | 当たる指摘 | 今の場所 |
|---|---|---|
| 件 1（helper の DLL の探し先） | **指摘 1**（同じ直しを 2 回数えない） | `shiori_proxy.rs:97`（素の `LoadLibraryW`）・`:59`（取り込み） |
| 件 2（`PASTA_DEBUG` の受け継ぎ） | **指摘 2**（あわせて指摘 16 が同じ内側の関数でテストを直す） | `process_host.rs:262-275`（環境は足すだけ）・`:293`（起こす） |

### 件 1 の詳しい確かめ

- 手本の場所: `crates/areka-ghost/src/shiori_inproc.rs:63-66`（取り込み）・`:90-114`（`load_library_quiet`）・`:102-108`（`LoadLibraryExW(wide, None, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS)`）。
- エラーの窓を出さない手当ては、2 つの道とも同じ（読むあいだだけ `SetThreadErrorMode(SEM_FAILCRITICALERRORS | SEM_NOOPENFILEERRORBOX)`＝helper `:86-101`）。直しの後もこれを残せば要件 7.2 を満たす。
- 作業フォルダ: helper は `SetCurrentDirectory` を呼ばない。host が `.current_dir(load_dir)` で起こす（`process_host.rs:273-274`）。
- 探し先の変わり方: 今は「exe のフォルダ → System32 など → 作業フォルダ（＝ゴーストのフォルダ）→ `PATH`」。直した後は「読む DLL のフォルダ（`shiori` の名前がフォルダを含まなければゴーストのフォルダ）→ exe のフォルダ → System32 → `AddDllDirectory` のフォルダ」。旗が効くのはこの 1 回の読み込みの静的な依存だけで、SHIORI が後で自分で呼ぶ `LoadLibrary`（SAORI など）は今のまま。
- 読めなくなるおそれのある配置（要件 7.5）: ① 依存が `PATH` にだけある ② `shiori,bin\x.dll` のように DLL がゴーストのフォルダの下の別のフォルダにあり、依存が `ghost\master` にある（今は作業フォルダで見つかる） ③ 依存が Windows のフォルダにだけある（稀）。
- emo2 の i686 の `pasta.dll`（`vendors/pasta/release/hello-pasta/ghost/master`）が頼るのは `user32`・`kernel32`・`bcryptprimitives`・`api-ms-win-core-synch-l1-2-0`・`ws2_32`・`ntdll`・`userenv` だけで、横に置いた DLL は無い＝直しで変わらない。
- 本番の道は絶対の道。サブエージェントの試しでは、`/` と `..` を含む道もこの旗で読めた（`shiori_proxy_loadu_tests.rs:183-184` のテストの道は直さなくてよい見込み）。
- 縛るテストの候補: 同じプロセス内の `a_relative_dll_path_is_refused` を helper の側（i686 だけ）へ写す。直しの後も緑のままのはずの既存のテスト＝`kernel32_yields_entry_not_found`（`shiori_proxy.rs:555`）・試験用の DLL と折り返しのテスト・host の `shiori_load_e2e.rs` の ③。横の DLL に頼る試験用の DLL を作って「横は見つかる」を縛るのは、取り込みの lib が要って重い（任意）。

### 件 2 の詳しい確かめ

- `spawn`（`process_host.rs:255-276`）は `env_remove`・`env_clear`・`.env("PASTA_DEBUG", ..)` のどれも呼ばない。子は親の環境を全部受け継ぐ（`PASTA_DEBUG`・`PATH`・`RUST_LOG`・`AREKA_*`・テストの仕掛け `HOST32_TESTDLL_LOAD_FAIL` も）。
- 手本: `crates/real-run-kit/src/launch.rs:213-214`（`.env("PASTA_DEBUG", "0")`）と、そのテスト `launch_tests.rs:54-55`（`get_envs()` で確かめる）。
- `spawn` の環境を確かめる既存のテストは無い（指摘 16）。
- 当座の錠 `real_pasta_turn`（`crates/areka-ghost/tests/ghost/common.rs:99-106`）の使い手は 3 本＝`tests/ghost/real_pasta_test.rs:85`・`tests/ghost/snapshot_capture_test.rs:117`・`tests/real_pasta_online.rs:118`。3 本とも `resolve_helper_exe` と `real_connect` で **helper の下の** i686 の pasta を使う（同じプロセス内の pasta ではない）＝件 2 を直せば、錠の理由は消える（議題 3）。それぞれの冒頭の説明にも「`PASTA_DEBUG=0` で回す」とある。
- 公開の形を変えずに `spawn` の中で閉じて直せる（要件 8.2）。

## 6. 数のまとめ

| 組 | 件数 | 番号 |
|---|---|---|
| 必ず直す | 3 | 1〜3 |
| 直したほうがよい | 18 | 4〜21 |
| できれば | 27 | 22〜48 |
| **計** | **48** | |

| 分類 | 件数 | 番号 |
|---|---|---|
| (a) 今直す | 42 | 1〜21・24・25（腕は残す）・29〜38・40〜48（13 は ⑶ に当たるかの確かめつき・29 は `Cargo.lock` が変わらない時） |
| ⑴ 候補 | 2 | 27（あわせて (b)）・39 |
| ⑵ 候補 | 0 | （印は 8 節。どれも指摘にしていない） |
| ⑶ 候補 | 2 | 22・23 |
| (b) 申し合わせ | 3 | 26・27・28 |

27 は ⑴ と (b) の両方に当たるので、(a) 以外の指摘は 6 件（22・23・26・27・28・39）。42＋6＝48。

- 申し送りとの重なり: 件 1＝指摘 1・件 2＝指摘 2。
- clippy の指摘と同じ所（要件 3.5）: 今の時点では、audit の指摘のうち clippy の名前のついた指摘と明らかに重なるものは見つけていない。設計とタスクで、直した場所に clippy の指摘があったときに記録する。

## 7. 公開の形に響く指摘（要件 1.7・6.5）

`shiori-host32-host` の公開の形を変えるのは 3 件だけ。どれも `crates/areka/` の使い手は無い。

| 番号 | 変わるもの | 範囲の外の呼び出し元 |
|---|---|---|
| 26 | `poll_exit`・`HelperHandle::helper_hwnd`／`set_helper_hwnd`・`classify_failure`・`FailureClass`・`LifecycleReport`・`HelperLifecycle::report_failure` を消す | 無い（`pilot` は自分の写しを持つ） |
| 27 | `request_of` を消す | `areka-ghost/src/shiori_inproc.rs:409`・`shiori_inproc_headers_tests.rs` |
| 25（腕を消すときだけ） | `ExitKind::Terminated` を消す | `areka-kanade/src/shiori/real_tests.rs:444`・`:479`・`real_idle_tests.rs:48` |

形を変えない直しで済むもの: 指摘 2（内側の関数）・6（表示の文だけ）・14（Drop の実装を足す＝型・関数・欄は同じ）・17（`pub(crate)`）・35（呼び手の側で直す）。`crates/areka/` が使う公開の型（`ExitKind`・`HelperStatus`・`RequestError`・`ShutdownError`）の形は、どの直しでも変えない。

## 8. わざと置いた印（⑵ の材料・指摘にしなかったもの）

| 場所 | 中身 | 見立て |
|---|---|---|
| `process_host.rs:350-352` | `// ponytail: 先頭 4 KiB だけを見る。…当たったら e_lfanew へ seek して読む形へ広げる。` | 限界（PE の見出しが 4 KiB より後ろ）は今の helper（`e_lfanew` は 0x100 前後）に当たらない＝決めとして残す |
| `shiori_proxy.rs:106-107` | `ProxyError` の `#[allow(dead_code)]`（中身は `main.rs` の `{e:?}` だけが読む・派生の Debug は読み手に数えられない） | 実在の理由つき＝残す |
| `shiori_proxy.rs:209-211` | 「天井:」の注釈（下位のバイトが 0 で上位が立った BOOL を失敗と読む） | 今の負荷で当たらない＝残す |
| `process_host_console_tests.rs:42` | 親のテストが子として起こす観測の本体の `#[ignore]` | わざと＝残す |
| helper `main.rs:585-586` | 約 42 秒の固まりの檻を i686 だけにしている | わざと＝残す（指摘 32 で古い説明だけ直す） |

`shortcut:` の注釈は範囲に 1 つも無い。指摘 37 で 1 つ置く案がある。

## 9. 要件との突き合わせ

| 要件 | 今の姿 | 足りないもの |
|---|---|---|
| 1（洗い出し） | この文書 | 無い（全部読んだ・全 48 件） |
| 2（討議で確定） | 議題は 11 節 | 討議での確定（⑴・⑶ の候補と (b) の 3 件・`shiori-abi` を外す確かめ） |
| 3（直し切る） | — | 48 件の直し。消す前の grep は 26・27・28・45 で済ませた（設計の段でもう一度） |
| 4（テストで縛る） | i686 の試験は `tools/test-all.ps1` の i686 の段で回る。成果物が無いと panic で赤（黙って飛ばさない） | 1・2・7・8・14・15〜20・38 の縛り。テストの一時フォルダの置き場（3） |
| 5（決まり） | 1,000 行に近い 3 本は中のテストを外へ出せる | 4・5・9〜12（記録の無い失敗の道）・34（記録の宛名） |
| 6（範囲の外） | 範囲の外へ及ぶのは `areka-ghost`（35・27）と、テストの錠の `areka-ghost/tests`（議題 3）だけ。`crates/areka/` には及ばない | (b) の 3 件の申し合わせ |
| 7（件 1） | 5 節 | 指摘 1 |
| 8（件 2） | 5 節 | 指摘 2・議題 2・3 |
| 9（仕上げ） | — | 直した後の audit のかけ直し・全体テスト |

## 10. 直し方の案・規模・危うさ

- **案 A（今のファイルの中で直す）**: 指摘の多くは 1〜数行の直しか、記録の 1 行を足すもの。足す量が少なく、今の層の分け方を崩さない。ただし `client.rs`（957 行）に内側の関数とテストを足すと 1,000 行を越える。
- **案 B（新しいファイルを足す）**: 結合テストの共通の道具（`tests/common/mod.rs`）・helper のテストの共通のモジュール・標準エラーを読むスレッド（`process_host` の兄弟のファイル）・版で分ける内側の関数のテスト。ファイルは増えるが、写しが消えて行は減る。
- **案 C（混ぜる・推す）**: 本番のコードの直しは今のファイルの中で（案 A）、テストの道具の重複と新しいテストは兄弟のファイルへ（案 B）。`client.rs`・`shiori3.rs`・`shiori_proxy.rs` は中のテストを兄弟の `_tests.rs` へ出して 1,000 行の余裕を作る。
- **規模**: M（3〜7 日）。48 件だが半分は 1〜数行。重いのは 4（標準エラーの読み手）・8（再入の旗）・17（正常終了の失敗の道のテスト）・3＋21（テストの道具の寄せ）。
- **危うさ**: 中。1・4・8・13・38 は本番の振る舞いを変えるので、実機（emo2 の i686 の pasta）での確かめが要る。ほかは型の決まった直し。
- **タスクの数の見込み**: 直す場所のまとまりで 15〜19（例: 件 1／件 2＋`spawn` のテスト／一時フォルダとテストの道具／helper の標準エラー／記録の無い失敗の道 4 か所／応答の長さと null のテスト／再入／job と Drop／版で分けるテストと写し／正常終了の失敗の道のテスト／親の窓の送り手の確かめ／古い説明と宛名／小さい速さと消すもの／(b) の 3 件／かけ直しと全体テスト）。20 を越えない見込み＝分けなくてよい（議題 13）。

## 11. 討議に出す議題（推奨つき・決めは討議で）

1. **`shiori-abi`・`shiori4-testdll` を受け持ちから外す**（要件 2.7）。推＝外す（`shiori4-api` が消す・残すと `crates/areka/` に及ぶ）。
2. **helper の下の pasta のデバッグの待ち受けを開ける口を設けるか**（要件 8.3・指摘 2）。推＝設けない。helper にはいつも `PASTA_DEBUG=0` を渡す（使う人がいない口は作らない・要るとき 1 行で足せる）。別案＝`AREKA_` の名前の環境変数（例 `AREKA_HOST32_PASTA_DEBUG=1`）のときだけ親の値を通す。
3. **当座の錠 `real_pasta_turn` を残すか外すか**（要件 8.4）。推＝外す。3 本とも helper の下の pasta なので、件 2 の直しで理由が消える。錠と 3 本の説明の「`PASTA_DEBUG=0` で回す」を一緒に消す（`areka-ghost` は C19 のほかの spec と重ならない）。
4. **件 1 の探し方と、読めなくなるおそれのある配置**（要件 7.1・7.5）。推＝同じプロセス内と同じ旗（`LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS`）。5 節の ①〜③ の配置は受け入れる（同じプロセス内の 64bit の道もすでに同じ）。別案＝`LOAD_WITH_ALTERED_SEARCH_PATH`（今の探し方に近いが、`PATH` と作業フォルダが残り、2 つの道がそろわない）。
5. **job が SHIORI の起こしたプロセスまで殺す件**（指摘 13）。推＝直す（`SILENT_BREAKAWAY_OK`）。SHIORI が起こしたプログラムがゴーストを閉じても残るのは SSP と同じ向き。⑶ に当たるかは、SSP で測ってから決める（ukadoc はプロセスの寿命を書いていない見込み）。
6. **応答のヘッダの値の切り方と、同じ名前のヘッダの決まり**（指摘 22・23・⑶ 候補）。推＝ukadoc で確かめ、書いていなければ SSP の MCP で測り、SSP と同じに直す（要件 2.3 で出どころと引用を添える）。測れなければ、22 は「半角の空白とタブだけ削る」、23 は「先勝ちにそろえる」。
7. **使い手のいない公開の関数と `request_of`**（指摘 26・27・(b)）。推＝26 は `runtime-module-untangle` のセッションと申し合わせて C19 のうちに消す（`crates/areka/` は使っていない＝響きは 0）。27 は完了した spec の決め（要件 2.8「2 つの道が同じ組み立てを共有する」）なので残す（⑴）。
8. **使っていない依存 3 つ**（指摘 28・(b)・`Cargo.lock` の席）。推＝`dola-yaml-backend` のセッションと申し合わせ、その着地の後に本 spec が `main` を取り込んでから消す（本 spec の最後の方のタスク）。本 spec の完了の方が先なら C20 へ回す。
9. **`unload` が偽を返したときの UNLOAD の返事**（指摘 9）。推＝記録だけ足して返事は [1] のまま（host は [1] でないと `UnexpectedAck` で終わりを失敗にする・保存の失敗で終了を止めない）。
10. **helper の標準エラーを areka の記録へ回す**（指摘 4）。推＝回す（パイプ＋読み手のスレッド・文はそのまま・`warn` の段）。本番の記録に helper の行が出るようになる＝外から見える変化なので確かめる。
11. **実物の pasta の任意のテストが、DLL が無いと黙って緑になる件**（指摘 39・⑴ 候補）。推＝残す（完了した spec が「任意」と決めた例外・要件 4.4 の成果物は helper と試験用の DLL で、実物の pasta は外の物）。直すなら `#[ignore]` で見える形に。
12. **`ExitKind::Terminated`**（指摘 25）。推＝腕は残して説明と偽のテストだけ直す（C19 は公開の形を変えない）。腕を消すのは C20 以降の別の話。
13. **タスクの数**（要件 2.6）。推＝分けない（見込み 15〜19）。

## 12. 設計で調べること（Research Needed）

- **R1** 件 1 の直しで、実物のゴースト（emo2 の i686 の pasta のほか、手元の YAYA・華和梨・里々などがあれば）が読めるかを実機で確かめる。
- **R2** 指摘 4: 標準エラーをパイプにしたとき、本体の実機の試験（`crates/areka/tests/emo2_real_run/common.rs:22` が探す `[helper] LOAD 失敗…`）が、areka の記録の中にその文を見つけ続けられるか（`tracing` の出力先と、行の前に付く飾り）。`real-run-kit` の下でパイプを握る詰まり（`job.rs:11-12`）が消えるか。
- **R3** 指摘 8: `wintf-winmsg-executor` 0.0.5 の窓の手続きに panic を受け止める所が無いことの確かめ（`util/window.rs:263-303`）と、最中の UNLOAD を折り返しのテストで作れるか。
- **R4** 指摘 40: 「1 プロセスに窓 1 つ」の制約が本当にプロセス全体のものか（`wintf-winmsg-executor` のソース）。全体なら、本番で 32bit のゴーストを 2 体同時に動かす場合にも当たる＝範囲の外へ持ち出す話かもしれない。
- **R5** 指摘 13・22・23: SSP の振る舞い（SHIORI が起こしたプロセスの寿命・ヘッダの値の空白の切り方・同じ名前のヘッダ）を ukadoc と SSP の MCP で確かめる。
- **R6** 指摘 29: 試験用の DLL 2 つを `windows = { workspace = true }` にしたとき `Cargo.lock` が変わらないか。
- **R7** 指摘 3: `CARGO_TARGET_TMPDIR` はクレートの結合テストだけで使える＝単体テストと helper の i686 のテストで `target\` の下を指す一番小さい形（`CARGO_MANIFEST_DIR` から辿る）を決める。
- **R8** 指摘 38: `GetWindowThreadProcessId` を使うのに要る `windows` の機能が、ワークスペースの機能でもう有効か（足すと `Cargo.toml` の機能の行が増えるだけで `Cargo.lock` は変わらない見込み）。

## 13. 次の段

要件の討議（`/kiro-requirements-discussion audit-shiori-host32`）で、11 節の議題と 48 件を直す対象として確定する。その後 `/kiro-design audit-shiori-host32`。

## 14. 設計の段（2026-11-13・`/kiro-spec-design audit-shiori-host32 -y`）

### 14.1 発見の型と読んだもの

- 発見の型は light（今ある 5 クレートの直し・新しい依存なし・外の資料の調べは要らない）。サブエージェントは使わず、進め役が直接コードを読んだ。
- 使った手引き: `kiro-spec-design` の `rules/`（design-principles・design-discovery-light・design-synthesis・design-review-gate）と `.kiro/settings/templates/specs/design.md`。直し方は ponytail の「いちばん小さな完全な変更」に従った。
- 読んだコード（範囲の外を含む）: host の `process_host.rs`・`lifecycle.rs`・`parent_window.rs`・`client.rs`・`shiori3.rs`・`shiori2.rs`・`job.rs`・`lib.rs`・`error.rs`・`charset.rs`、helper の `main.rs`・`shiori_proxy.rs`、ipc の `lib.rs`、`wintf-winmsg-executor` 0.0.5 の `src/lib.rs`・`src/util/window.rs`・`src/util/msg_filter_hook.rs`（cargo の置き場の写し）、std の `sys/process/windows.rs`（`Child::kill`）、`areka-ghost/src/shiori_wiring.rs`・`tests/ghost/common.rs`、`crates/areka/tests/emo2_real_run/common.rs`、`real-run-kit/src/run.rs`・`launch.rs`、`log-capture-kit/tests/temp_path_guard_test.rs`・`with_default_guard_test.rs`・`src/global.rs`、`temp-path-kit/src/lib.rs`、ルートの `Cargo.toml`・`Cargo.lock`、`tools/test-all.ps1`。

### 14.2 調べること（12 節）の結果

- **R1（件 1 で実物のゴーストが読めるか）**: 静かな読みの結論は 5 節のまま（emo2 の i686 の `pasta.dll` が頼るのは OS の DLL だけ）。実機の確かめは SSP も動いている areka も動かさないこの段ではせず、実装の U2 の最後の手順にした（`areka-ghost` の `real_pasta_test` を `HOST32_PASTA_DLL` で回す）。読めなければ要件 7.5 で議題。
- **R2（標準エラーをパイプにしても本体の実機の試験が文を探せるか）**: 探せる。本体の試験は areka の標準出力と標準エラーをつないだ文字列を `contains` で探す（`emo2_real_run/common.rs:143`・`real-run-kit/src/run.rs:110-111`）。areka の記録の行は同じ出力に出ていて（同じ試験が `WIRE`・`ATTACH` の記録の行を見つけている）、`warn!` の本文に行をそのまま入れれば一致する。試験の `RUST_LOG`（`info,kanade=trace,..`）は `warn` を通す。`job.rs:11-12` の詰まりのうち、標準エラーの受け継ぎの分は消える（標準出力の受け継ぎは今のまま＝指摘に無い）。
- **R3（窓の手続きの panic）**: `wintf-winmsg-executor` 0.0.5 の `util/window.rs` の `wndproc_typed`（`unsafe extern "system"`）は利用者の手続きを `catch_unwind` なしで呼ぶ。panic は `extern "system"` を越えられずプロセスが止まる＝指摘 8 は本物。`catch_unwind` があるのは executor の起こし用の窓（`lib.rs:35`）と `MessageLoop::run` のフィルタ（`lib.rs:238`）だけ。再入の折り返しのテストは、試験用の DLL が `request` の中から helper の窓へ `SendMessageW` を送れば、同じスレッドの中で決定的に作れる（同じスレッドの窓への `SendMessage` は手続きを直接呼ぶ）。
- **R4（「1 プロセスに窓 1 組」の制約）**: ソースに当たる制限は無い。窓のクラスはプロセスに 1 度だけ登録され（`new_ex` の中の `static CLASS_REGISTRATION: Once`）、`CreateWindowExW` は呼ぶたびに新しい窓を作る。スレッドごとの決まりは `MsgFilterHook::register` の「同じスレッドで `MessageLoop::run` を入れ子にしない」（`msg_filter_hook.rs` の `assert!`）だけ。設計は、2 つのスレッドで窓を同時に持つテストを先に置き、緑なら直列の錠と説明を消す、赤なら原因を絞って逃げ道へ、とした（design.md「窓を作るテストの並び」）。
- **R5（SSP の振る舞い）**: 13 は要件 10.7 で決まったので測らない。22・23 は実装の U11 の始めに、開発者の承認を得てから SSP で測る手順にした。測れなければ要件 10.8 の既定（22＝半角の空白とタブだけ・23＝先勝ち）。
- **R6（試験用の DLL を `workspace = true` にしたとき `Cargo.lock` が変わるか）**: 変わらない。ワークスペースの `windows` は `"0.62"` で、`Cargo.lock` の `windows` は 0.62.2 の 1 つだけ。`Cargo.lock` の 2 つの DLL の依存の並びは `"windows"` の名前だけで、版も機能も載らない。重い機能はワークスペースの機能に入っているが、i686 の段では helper が同じ機能で `windows` をもう組んでいる（機能は同じ組みの中でまとめられる）ので、組みの重さは増えない。実装では `git diff --exit-code Cargo.lock` で確かめる。
- **R7（テストの根を `target\` の下に置く一番小さい形）**: `temp-path-kit` と同じ場所（`CARGO_MANIFEST_DIR` の 2 つ上の `target\test-roots\`）にする。`CARGO_TARGET_TMPDIR` は結合テストにしか無いので使わない。このワークツリーには `.cargo/config.toml` も `CARGO_TARGET_DIR` も無い（`target` は移されていない）。`temp-path-kit` を dev-dependency に足すと `Cargo.lock` が変わるので足さない。今の範囲の 10 ファイルは `log-capture-kit/tests/temp_path_guard_test.rs` の例外表に載っている＝直した後は表から外す（外さないと `every_exception_still_has_a_real_hit` が赤）。
- **R8（`windows` の機能）**: `GetWindowThreadProcessId` は `Win32_UI_WindowsAndMessaging`（host が有効にしている）。`WaitForSingleObject` と `CREATE_NO_WINDOW` は `Win32_System_Threading`（ワークスペースの機能にある＝host は `workspace = true` で受け継ぐ）。`JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK`・`QueryInformationJobObject` は `Win32_System_JobObjects`（host が有効）。`GlobalSize`・`GlobalLock` は `Win32_System_Memory`（helper が有効）。足すのは host の `Win32_Globalization`（`MultiByteToWideChar`）と testdll の `Win32_System_DataExchange`（`COPYDATASTRUCT`）だけで、機能の行は `Cargo.lock` に載らない。
- **ついでに確かめたこと**: 今の std（1.100.0）の Windows の `Child::kill` は、終わったプロセスへの `TerminateProcess` の `ERROR_ACCESS_DENIED` を `Ok` にする（`sys/process/windows.rs:725-737`）＝指摘 31 の腕は通らない。`ExitStatus::code()` は Windows でいつも値を持つ＝指摘 25 のとおり。`log-capture-kit` の番人は、捕捉先をスレッドへ差す呼出（`with_default(`・`set_default(`）と全体の捕捉の窓口を例外表の外で禁じている＝標準エラーの読み手は記録の行き先をスレッドへ受け渡さず、行を渡す口（sink）で試す形にした。

### 14.3 指摘 26 の申し合わせ（要件 10.9）

- 2026-11-13、進め役を通して `runtime-module-untangle` のセッションから返事を得た: `poll_exit`・`HelperHandle::helper_hwnd`／`set_helper_hwnd`（と欄）・`classify_failure`・`FailureClass`・`LifecycleReport`・`HelperLifecycle::report_failure` を使っておらず、使う予定も無い（そのブランチの `crates/areka` の grep で 0 件）。その書き換えは `shiori-host32-host` の公開の形に触らない。C19 のうちに消すことに異議は無い。
- C19 の約束 ⑴「③ のセッションと申し合わせる」を満たすので、26 は待ちのない普通のまとまり（U16）にした。
- 一覧に無い `EXIT_POLL_INTERVAL`（U9 で使い手がいなくなる公開の定数・`crates/areka` の使い手 0）は、U16 の始めに同じセッションへ一言知らせてから消す。異議があれば残して開発者へ報告する。

### 14.4 総合（design-synthesis）

- **まとめられること**: テストの一時の置き場の決まりを 1 つ（`target\test-roots\`）にし、host・helper・loadu で同じにした。helper の失敗の行の出口を `helper_log` 1 つにした（行の文は変えない・テストで読める）。版で分ける 4 つの関数を `encode_for`／`parse_for` の 2 つに、`Reference` の並べ方を `push_references` 1 つにした。正常終了の 3 つの失敗の腕と 24 の問い直しを、待つ長さを引数にとる 1 つの内側の関数で試せる形にした。
- **作るか、あるものを使うか**: テストの置き場は `temp-path-kit` を使いたいが `Cargo.lock` が変わるので、同じ場所の決まりを小さく写した（互いの場所を注釈で指す）。終わりの見張りは眠りの繰り返しをやめて OS の `WaitForSingleObject` を使う。job の抜け道は OS の `SILENT_BREAKAWAY_OK`。応答の長さは OS の `GlobalSize`／`GlobalLock`。標準エラーの別のスレッドの記録を試すのに全体の捕捉は使わず（番人の例外表を増やさない）、行を渡す口を内側の関数の引数にした。
- **削ったもの**: 親の窓の本番で読まれない数（`hellos`・`bad_frames`・`ignored_known`）を記録に替えて消す。結合テストの `HelperGuard` の 3 つの写しは `HelperHandle` の Drop で要らなくなる。写しのテスト（`.bat`）と偽のテスト（`classify_terminated_branch_logic`）と時間に頼るテスト（`error_paths` の 3 本目）を消す。helper の下の pasta のデバッグの待ち受けを開ける口は作らない（要件 10.4）。

### 14.5 設計で決めたこと（見直しの材料）

1. **再入（8）の UNLOAD は ack [1] を先に返し、後始末は DLL の呼び出しが戻ってから**: host の取り決め（UNLOAD の返事が受け皿に入らないと送りの失敗）を変えずに、`unload` を呼ぶ道を残すため。DLL が `EXIT_OBSERVE_TIMEOUT`（10 秒）より長く戻らなければ host が終わらせる（今は必ず panic で落ちる）。
2. **送り手の確かめ（38）は足す口 `expect_helper_pid` で本番だけ子のプロセス番号に縛る**: `pump_until_hello_or` の形を変えられない（要件 6.4）ため。呼ばない使い手も「HELLO は 1 度だけ・送り手と中身が同じ」で守られる。
3. **`HelperHandle` の Drop は終わりを 5 秒まで、標準エラーの読み手を 1 秒まで待つ**: テストの根を消すときに DLL がまだ読まれている事故（指摘 3）と、最後の行が接続の失敗より後に記録に入る順の乱れを、同じ所で無くすため。`kill` の後の終わりはすぐ来るので、本番の終わりを遅らせない。
4. **標準エラーの UTF-8 でない行は CP_ACP で読む**: helper 自身の行は UTF-8 だが、SHIORI の DLL が C の実行時の関数で書く行は ANSI のことがある。文字化けを読む側で直す決まりに従う。
5. **`EXIT_POLL_INTERVAL` も消す**（申し合わせの一覧の外・知らせてから）。
6. **22・23 は既定（半角の空白とタブだけ・先勝ち）の形で設計し、SSP で測れて違えば合わせる**: この段で SSP を動かさないため、測りは実装の段で開発者の承認を得てから。
7. **`error_paths` の 3 本目は直さずに消す**（指摘 18 の 2 案のうち小さい方）。helper が自分で終わる道の記録は、実の helper の標準エラーのテストが受ける。
8. **28 は U17 として `dola-yaml-backend` の着地を待つ**。先に完了するなら C20 へ回す（要件 10.10）。

### 14.6 危うさ

- R4 の見立てが外れて窓を 2 組持てなければ、U9 の正常終了の試験の作り方を替える（逃げ道は design.md に書いた）。
- 標準エラーを areka の記録へ回すので、壊れた SHIORI が大量に書くと記録が増える（行ごとに 1 行。上限は置かない＝今の負荷で当たらない）。
- 件 1 の直しで、依存を `PATH` にだけ置くゴーストや、DLL を下のフォルダに置き依存を `ghost\master` に置くゴーストは読めなくなる（要件 10.6 で受け入れ済み）。

### 14.7 clippy の基準（要件 3.5）

- U1 の始めに、範囲の 5 クレートの `cargo clippy`（x64 と i686）の警告の一覧（ファイル・行・lint の名前）をここに控え、U18 で消えた分を書く（`clippy-199-lints` が直し直さないため）。

### 14.8 設計の討議（2026-11-13・開発者「承認」＝3 議題を推奨案で確定）

- 検証（`design-validation.md`）は GO。重大な指摘 3 件のうち 2 件は自明な直しで設計に入れた（areka-ghost の当座の錠を外すのを U8 の窓 2 組のテストが緑の後へ移す・helper の標準の取っ手を受け継がせない `disinherit_std_handles()`）。
- 議題 1（指摘 38 の RESPONSE）: 確かめは残し、限界を書いた `shortcut:` の注を置く。要件 2.2 ⑵ の「わざと置いた先送りの印」として開発者が裁定した。理由＝`wParam` は送り手の自己申告で、同じ利用者のプロセスは DLL の差し込みなど、もっと強いことができる。本当に防ぐには起こすたびの合言葉を枠に載せる要があり、取り決めの形が変わる。
- 議題 2（指摘 8 の再入）: 設計のまま。最中の UNLOAD には ack [1] を先に返し、DLL が戻ってから `close` する。最中の REQUEST には 500。DLL が 10 秒以内に戻らなければ host が終わらせる（そのとき `unload` は呼ばれない）。
- 議題 3（指摘 22・23 の SSP での測り）: 設計のまま。U11 の始めに進行役が開発者に改めて許しを伺い、許しがあれば測る。無ければ既定（半角の空白とタブだけ削る・先勝ち）で直す。

### 14.9 main の取り込み（2026-11-13・origin/main `343ec0350`）

- 棚卸の知らせで取り込んだ。audit 系でない 4 本が着地した: `dola-yaml-backend`（PR#443）・`wintf-d2d-composite-rules`（PR#444）・`emo-text-reveal-incremental`（PR#445）・`runtime-module-untangle`（PR#447。本体の `emo2_boot/` は `runtime/` へ、本体に頼る所は `session/` へ移った）。
- 本 spec が触る予定のファイル（範囲の 5 クレート・`areka-ghost` の `shiori_wiring.rs` と錠の 4 ファイル・`temp_path_guard_test.rs`・`areka-kanade` のテスト）は、どれも取り込みで変わっていない。変わったのはワークスペースの `Cargo.toml`（`async-executor` の行が消えた）と `Cargo.lock` だけで、本 spec の直しには響かない。設計の前提は崩れていない。
- 指摘 26 の 6 項目と `EXIT_POLL_INTERVAL` を、取り込んだ後のツリーで grep し直した（`crates`・`tools`・`doc`。範囲のクレートと `pilot` を除く）。使い手は 0 件。`helper_hwnd` に当たった `areka-ghost/src/shiori_wiring.rs:171` と `areka-kanade/tests/kanade/real_helper_test.rs:121` は、同じ名前の手元の変数で、消す口ではない。
- 指摘 28: `dola-yaml-backend` が着地したので、`Cargo.lock` の席は空いた。タスク 13 は待たずに消す道を取る。取り込んだ後のソースでも、host の `event-listener`・helper の `thiserror`・ipc の `windows-core` を使う所は 0 件。host と ipc の `thiserror` は使われているので残す。

### 14.10 clippy の基準（タスク 1.1・直しの前・`33400f539`）

- 測り方: `cargo clippy -p shiori-host32-host -p shiori-host32-ipc --all-targets`（x64）と `cargo clippy -p shiori-host32-helper -p shiori-host32-testdll -p shiori-host32-testdll-loadu -p shiori-host32-ipc --all-targets --target i686-pc-windows-msvc`（i686）。どちらも直しを 1 行も入れる前に回した。
- 警告は全部で 5 件（x64 が 4 件・i686 が 1 件）。helper・ipc・loadu は 0 件。

| 段 | 場所 | lint | 中身 |
|---|---|---|---|
| x64 | `crates/shiori-host32-host/tests/lifecycle_cyclic_e2e.rs:275` | `clippy::drop_non_drop` | Drop を持たない `client` を `drop` している |
| x64 | `crates/shiori-host32-host/tests/lifecycle_cyclic_e2e.rs:408` | `clippy::drop_non_drop` | 同上（実物の pasta のテスト） |
| x64 | `crates/shiori-host32-host/tests/lifecycle_kill_e2e.rs:332` | `clippy::drop_non_drop` | 同上 |
| x64 | `crates/shiori-host32-host/Cargo.toml`（manifest） | `unused dependency`（cargo の警告） | `event-listener` を使っていない（指摘 28・U17 で消す） |
| i686 | `crates/shiori-host32-testdll/src/lib.rs:63` | `clippy::needless_bool` | `if` の守りが真偽の字をそのまま返す |

- U18 では、この表の 5 件のうち消えた分と、audit の指摘と重なった所（`event-listener` は指摘 28）を書く。

### 14.11 試験用の DLL の依存の形（タスク 2・指摘 29）

- testdll と loadu の `Cargo.toml` を `[dependencies.windows] workspace = true` と機能の行にした（版の直書き `"0.62.2"` を消した）。testdll は `OnTestReenter` のために `Win32_System_DataExchange`（と、ワークスペースの機能に元からある `Win32_UI_WindowsAndMessaging`）を足した。
- `git diff --exit-code Cargo.lock` は 0（差分なし）。x64 と i686（`--target i686-pc-windows-msvc`）の両方で `cargo build -p shiori-host32-testdll -p shiori-host32-testdll-loadu` が通った後に確かめた。解決される `windows` の版は今までと同じ 0.62.2。
- 14.10 の基準のうち、i686 の `crates/shiori-host32-testdll/src/lib.rs:63` の `clippy::needless_bool` はこのタスクで消えた（`load` の戻りを比べの式にした）。

### 14.12 実物のゴーストが読めるか（タスク 3・R1・要件 7.5）

- 直し: helper の `load_library_quiet`（`crates/shiori-host32-helper/src/shiori_proxy.rs`）を `LoadLibraryExW(wide, None, LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS)` にした。エラーの窓を止める手当ては残した。テスト `shiori_proxy_tests.rs::a_relative_dll_path_is_refused`（i686 だけ）は、素の `LoadLibraryW` に戻すと相対パスで読めて赤になることを確かめた。
- 静かな読み（`dumpbin /dependents`・どれも `14C machine (x86)`）: 静的な依存はどれも OS の DLL だけで、探し先を絞っても見つかる。
  - pasta（`vendors/pasta/release/hello-pasta/ghost/master/pasta.dll`）: user32・kernel32・bcryptprimitives・api-ms-win-core-synch-l1-2-0・ws2_32・ntdll・userenv
  - YAYA（`konnoyayame` の `yaya.dll`）: KERNEL32・USER32・SHELL32
  - 里々（`R_POST_and_KOMAINU`・手元の `emo-gs` の里々・`Kunigunde2` の `satori.dll`）: USER32・KERNEL32
  - 華和梨のゴーストは手元に無かった（ゴーストの `ghost\master` を探して見つからなかった）。
- 実機（排他のテストの机を取って 2 回。どれも `PASTA_DEBUG=0`）。

| 回 | 走らせたもの | helper | SHIORI の DLL | 結果 |
|---|---|---|---|---|
| 1 | `areka-ghost` の `real_pasta_boot_observes_a_cue_then_shuts_down_cleanly` | 新 | hello-pasta | 赤。`[helper] SHIORI 初期化の入口: loadu` の後に `LoadReturnedFalse { last_error: 203 }` |
| 1 | `areka-ghost` の `real_pasta_online` | 新 | hello-pasta | 7 本緑・1 本赤（`emo2_does_not_start_chatting_while_online`）。赤は上と同じ `LoadReturnedFalse` |
| 1 | `areka-kanade` の `real_helper_boot_pump_close_completes` | 新 | hello-pasta | 緑（入口 `loadu`） |
| 2 | 1a: `real_pasta_boot_observes_a_cue_then_shuts_down_cleanly` | 新 | emo2 の検体に入っている `pasta.dll` | 緑（入口 `loadu`・1.06 秒） |
| 2 | 1b: 同じテスト | 旧（素の `LoadLibraryW`） | hello-pasta | 赤。1 回目と同じ `LoadReturnedFalse { last_error: 203 }` |
| 2 | areka.exe を有界に自動終了で起こす（`konnoyayame`・YAYA） | 新 | `yaya.dll` | 終了コード 0・入口 `loadu`・`LoadLibraryFailed` と ERROR は 0 行 |
| 2 | 同（`R_POST_and_KOMAINU`・里々） | 新 | `satori.dll` | 終了コード 0・入口 `load`・`LoadLibraryFailed` と ERROR は 0 行 |

- 入口の行は DLL を読んで入口を解いた後にしか出ないので、どの走りでも読む段は通っている。
- `areka-ghost` の 2 本の赤は、hello-pasta の `pasta.dll`（3,832,832 B）を emo2 の辞書の中へ差し込む組み合わせのせい。emo2 の検体の `pasta.dll`（3,681,280 B）は別物で、それなら新しいコードで緑になり、旧の `LoadLibraryW` でも hello-pasta なら同じ赤になる。この直しとは関係がない。
- 結論: 読めないゴーストは見つからなかった。要件 7.5 の議題は要らない。

### 14.13 応答のヘッダの値の空白と重なり（タスク 9.2・R5・指摘 22・23・要件 10.8）

- 開発者の答え（2026-11-13・進行役が伝えた）: 「SSPで何か調べるならどうぞ。今起動しているSSPは終了しておきました。」＝SSP で測ってよい。
- ukadoc: ukadoc の MCP が応答せず（検索の呼び出しが 2 回とも失敗）、値の前後の空白と同じ名前のヘッダの扱いが ukadoc で決まっているかは確かめられなかった。
- 測り方: 測るだけのゴースト `arekaR5probe` を作り（`target\test-roots\shiori-host32-r5probe\arekaR5probe`・SHIORI は SSP の `emily4` の `yaya.dll` の写し・辞書 `probe.dic` の `request` が応答を丸ごと返す・シェルは 64×64 の単色 2 枚）、排他のテストの机を取って `C:\wintools\ssp\ghost\arekaR5probe` へ写した。`C:\wintools\ssp\ssp.exe` を起こし、起動したゴースト（窓際のぱすたさんDEBUG）から `\![call,ghost,arekaR5probe]` で呼び、SSP の MCP の `raise_event` で 5 つのイベントを送った。SSP の答えの台本と `get_log`（script・#27〜#31）は同じ。終わった後、両方のゴーストを `\-` で閉じて SSP を正常に終わらせ、写したゴーストを消した（SSP の `data\profile` に残った名前には触っていない）。
- 測った値:

| イベント | ゴーストが返した行 | SSP が採った台本 | 読み |
|---|---|---|---|
| `OnR5FullWidth` | `Value: 　fw　`（前後が全角の空白） | `　fw　` | 全角の空白は削らない |
| `OnR5HalfWidth` | `Value:`・タブ・半角の空白・`hw`・半角の空白・タブ | `hw`・半角の空白・タブ | 頭の半角の空白とタブだけ削る |
| `OnR5DupValue` | `Value: first`・`Value: second` | `second` | 後勝ち |
| `OnR5DupCharsetUtf8First` | `Charset: UTF-8`・`Charset: Shift_JIS`・`Value: こんにちは`（本文は UTF-8） | `縺薙ｓ縺ｫ縺｡縺ｯ`（Shift_JIS で読んで化けた） | 後勝ち |
| `OnR5DupCharsetSjisFirst` | 上の 2 行の順を入れ替えたもの | `こんにちは`（UTF-8 で読んだ） | 後勝ち |

- 後ろの半角の空白とタブが残ることは、SSP の MCP の答えと `get_log` の表示で読んだ。バイトでは確かめていない（このゴーストには `ssp_shiori_log.txt` が作られなかった）。
- 直し（`crates/shiori-host32-host/src/shiori3.rs`・削るのは半角の空白とタブだけ＝`HEADER_BLANKS`）: 設計の「SSP で測れて結果が違えば SSP に合わせる」に従い、既定（前後を削る・先勝ち）から次の形にした。
  - 測った値に合わせる: 値の見出し（3.0 の `Value`・2.x の `Sentence`／`String`）の値と `Charset` の値は頭だけ削る。後ろの空白と全角の空白は残す。`Charset` の値の後ろの空白は `encoding_rs` のラベルの読みが落とすので、残しても文字コードは解決できる（テストで確かめた）。
  - 測っていない点は今の要件の書きぶりを保つ: ヘッダ名と `ErrorLevel`・`ErrorDescription`・`SecurityLevel` の値は前後を削る。`SecurityLevel` は areka-P0-script-security-level 要件 3.5 が「前後の空白を落とした生の値」と書いており、これを縛る既存のテスト（`request_headers_tests.rs::response_security_level_is_the_trimmed_raw_value_case_insensitive`・`shiori3_charset_tests.rs::charset_header_name_matches_case_insensitively_and_tolerates_spacing`・`areka-ghost` の `shiori_inproc_headers_tests.rs::map_get_outcome_carries_the_raw_level`）を変えずに緑のまま保つため（開発者側の進行役の裁定・案 B）。
  - 重なり: 同じ名前のヘッダはすべて後勝ち（`Value`・2.x の値の見出し・`Charset`・`ErrorLevel`・`ErrorDescription`・`SecurityLevel`）。測ったのは `Value` と `Charset` だけで、ほかは同じ決まりにそろえた。
- テスト（新しい決まりで書き、直す前に 3 本とも赤を確かめた）: `shiori3_tests.rs::a_value_keeps_full_width_spaces`（頭のタブと空白・後ろの空白とタブ・全角の空白・後ろに空白の残る `Charset` の値が UTF-8 に解決される）・`the_last_of_duplicated_headers_wins`（5 つの見出しが 2 つずつで、どれも後ろ）、`shiori2_tests.rs::a_sentence_keeps_full_width_spaces_and_the_last_wins`。tasks.md の名前 `the_first_of_duplicated_headers_wins` は、測りで決まりが変わったので `the_last_…` にした。

### 14.14 `EXIT_POLL_INTERVAL` を消すことの知らせ（タスク 12・指摘 26・要件 10.9）

- 2026-11-13、進め役が確かめた: `runtime-module-untangle` は完了して `.kiro/specs/completed/runtime-module-untangle` に退避済みで、`main` に `343ec0350`（PR #447）として着地し、このブランチにも取り込み済み。そのセッションは動いていない（同日、走っている下請けの一覧で確かめた）。
- 結論: 知らせる相手なし・異議なしとして消す。`EXIT_POLL_INTERVAL` は 14.3 の一覧の項目と一緒にタスク 12 で消した（消す前にツリー全体を grep し直し、`crates/shiori-host32-host` の外の使い手は 0。`crates/pilot/examples/shiori-host-32/` の同名の項目は pilot 自身の写しで、設計どおり響かない）。

### 14.15 使っていない依存を消した（タスク 13・指摘 28・要件 10.10）

- 2026-11-13、進め役が確かめた: `dola-yaml-backend` は `main` に着地して `.kiro/specs/completed/dola-yaml-backend` に退避済みで、このブランチにも取り込み済み（`e7fd67119`）。C20 へは回さず、host の `event-listener`・helper の `thiserror`・ipc の `windows-core` を消した（消す前に 3 クレートを grep し直し、使う所は 0 件）。host の README の依存の一覧も直した。

### 14.16 かけ直し 1 回目（タスク 14）

- 結果: 新しい指摘が 4 件出た。4 件とも直した。
  1. 危うさ（本番）: helper の `helper_log` が `eprintln!` で書いており、標準エラーの管の読み手（host の `helper_stderr_read_failed`・`helper_stderr_reader_spawn_failed` の後）が居なくなると書けずに panic した。窓の手続き（`extern "system"`）の中なのでプロセスが落ち、unload が走らない。直し: `let _ = writeln!(std::io::stderr().lock(), ..)` で書き、書けなければ捨てる（記録の行き先がもう無いため）。縛るテスト: `crates/shiori-host32-helper/src/main_helper_log_tests.rs::helper_log_does_not_panic_when_stderr_reader_is_gone`（標準エラーはプロセスに 1 つで、libtest は `eprintln!` を自分で捕まえるため、子のプロセスの中で OS の取っ手を読み口の閉じた管へ差し替えて呼ぶ。管は `std::io::pipe`・差し替えは `SetStdHandle`・後で戻す）。直す前に赤を確かめた。host の 2 つの行の文（「ここから先の行は記録へ回らない」「helper の行は記録へ回らない」）は直した後の振る舞いと合うので変えていない。
  2. 黙った失敗: host の親窓が `lparam` の空の WM_COPYDATA を記録せずに捨てていた。直し: `parent_frame_ignored` の WARN（`reason = "null_lparam"`）を出す。縛るテスト: `parent_window_tests.rs::ignored_frames_are_logged` に 1 件足した（直す前に赤を確かめた）。
  3. 重なり: `read_copydata` が helper の `main.rs` と host の `parent_window.rs` に同じ形で 2 つあった。直し: `crates/shiori-host32-ipc` に `pub unsafe fn read_copydata`（`# Safety` 付き）を 1 つ置き、両方から呼ぶ。`Cargo.toml`・`Cargo.lock` は変えていない。振る舞いは既存のテストが縛る。
  4. 説明の古さ: `ParsedResponse::charset_header`（頭の半角の空白とタブだけ落とす）・`job.rs`（helper の標準入力・標準出力は NUL、標準エラーは host の管・helper が受け継ぎの印を自分で落とす）・`windows` 依存を「隔離する」と書いた所（`job.rs`・`terminator.rs`・`lib.rs` の 2 か所・`process_host.rs` も `windows` を使う）・host の `resolve_param`（helper の関数）の名・`shiori_proxy.rs` の出す行の一覧・`error.rs` の「本タスク（4.2）では定義のみ」・`parent_window.rs` の `ParentShared` の持ち物・testdll の `TESTDLL_SERIAL` の置き場・`shiori_proxy_tests.rs` の DLL を読むテストの一覧・`main_loopback_tests.rs` の UNLOAD（今は `close`）・`tests/shiori_load_e2e.rs` の SAFETY（2 つのテストは並行に走る）を今の姿に直した。`shiori_proxy.rs` の応答を `GlobalLock` で開けないときは、長さの行と別の行（`GlobalLock で開けない`）を出すようにした。返す Err は `ResponseLengthMismatch` のまま（呼び手は 500 を返すだけで、種類を分けても振る舞いが変わらない）。
- 見直して指摘にしなかったもの:
  - (a) helper の LOAD の腕が DLL を呼んでいる最中に次の LOAD が来る道: 届く道が無い。本番で LOAD を送るのは `crates/areka-ghost/src/shiori_wiring.rs:188` の 1 か所で、helper 1 つにつき 1 度だけ送る。同じ利用者の別のプロセスが LOAD を偽って送るのは、14.8 の議題 1 のとおり脅威の見立ての外。よって欠陥ではない。
  - (b) x64 の `clippy::drop_non_drop` の 3 件: 振る舞いに響かず、spec `clippy-199-lints` の持ち場。

タスク 14 で、1.1 と同じ 2 つの命令（x64 の `cargo clippy -p shiori-host32-host -p shiori-host32-ipc --all-targets` と、i686 の `cargo clippy -p shiori-host32-helper -p shiori-host32-ipc -p shiori-host32-testdll -p shiori-host32-testdll-loadu --all-targets --target i686-pc-windows-msvc`）を回し直した。14.10 の基準 5 件のうち 2 件が消え、3 件が残った。消えたのは、i686 の `crates/shiori-host32-testdll/src/lib.rs:63` の `clippy::needless_bool`（タスク 2 で `load` の戻りを比べの式にした・指摘 29 の直しと同じ所）と、x64 の `crates/shiori-host32-host/Cargo.toml` の使っていない依存 `event-listener` の警告（タスク 13・指摘 28 で依存を消した）である。残ったのは x64 の `clippy::drop_non_drop` の 3 件で、場所は直しで行が動き、今は `crates/shiori-host32-host/tests/lifecycle_cyclic_e2e.rs:162`・`:287` と `crates/shiori-host32-host/tests/lifecycle_kill_e2e.rs:212` にある。この 3 件は Drop を持たない `Shiori3Client` を `drop` しているだけで振る舞いに響かず、audit の指摘とも重ならないので、本 spec では直さず `clippy-199-lints` に残す。i686 の段の警告は 0 件になった。

### 14.17 かけ直し 2 回目（タスク 14）

- 結果: 新しい指摘が 10 件出た。10 件ともテスト・説明・片付けで、本番の振る舞いを変えるものは 0 件。10 件とも直した。
  1. テストの弱さ: `crates/shiori-host32-helper/src/main_helper_log_tests.rs` の子のテストを手書きの名前＋`--exact` で起こし、`.status().success()` だけを見ていた。名前がずれると子は 0 件走って成功し、親は緑のまま残る。直し: 子の名前を `module_path!()` から組み（`crates/shiori-host32-host/src/terminator_tests.rs` の `hold_child_entry` と同じ形）、`.output()` に替えて子の標準出力に `1 passed` があることを確かめる。名前をわざとずらして親が赤になる（子は `0 passed` で成功）のを確かめてから戻した。
  2. 説明の古さ: `crates/shiori-host32-host/tests/error_paths.rs` の 2 か所が `read_copydata` の置き場を `src/parent_window.rs` と書いていた。今は crate `shiori-host32-ipc` の `src/lib.rs`（14.16 の 3）。直した。
  3. 説明の古さ: `tests/shiori_load_e2e.rs`・`tests/shiori_request_e2e.rs`・`tests/lifecycle_cyclic_e2e.rs`・`tests/lifecycle_kill_e2e.rs` の 4 か所が「helper の cwd＝DLL 探索起点」と書いていた。タスク 3 から DLL は `load_dir\shiori.dll` の絶対パスを `LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_DEFAULT_DIRS` で読み、作業フォルダは探す場所に入らない。「`load_dir\shiori.dll` を絶対パスで読む（作業フォルダも load_dir）」に直した。
  4. 説明の古さ: `tests/shiori_load_e2e.rs` の `remove_var` の SAFETY が「上記 set と対（単一スレッド）」のままだった（14.16 の 4 で set の側だけ直した）。「上記 set と同じ根拠」に直した。
  5. 説明の古さ（「courtesy unload」）: helper の `main.rs` のメッセージループの前の注、`main_loopback_tests.rs` の断言の文（take→drop）と窓を消す前の注、`shiori_proxy.rs` の `request` の「Drop courtesy のみ」の 4 か所。今は UNLOAD の道が `close` を呼び、Drop が後始末するのは UNLOAD を経ない道だけ。その姿に直した。
  6. 説明の古さ: `crates/areka-ghost/src/shiori_wiring.rs` の `HANDSHAKE_TIMEOUT` が「既存 E2E の `HANDSHAKE_TIMEOUT` と同値」と書いていた。E2E の方は今 30 秒、本番は 5 秒。「同値」の句を消した。値は変えていない。
  7. 説明の古さ: `tests/lifecycle_kill_e2e.rs` の流れ図と kill 後の注が、kill の結果に `Terminated`（終了コードなし）が起こりうるように書いていた。Windows ではいつも `Abnormal(1)`（`process_host.rs` の `ExitKind`・指摘 25）。説明だけ直し、断言（`Abnormal(_) | Terminated` を通し `Clean` を拒む）は誤りではないので変えていない。
  8. 説明の古さ: `crates/shiori-host32-host/README.md` の手順 ③ が `cargo test -p shiori-host32-ipc --target i686-pc-windows-msvc` だけだった。`tools/test-all.ps1` と同じ `cargo test -p shiori-host32-helper -p shiori-host32-ipc --target i686-pc-windows-msvc` に直した。
  9. 説明の形: `crates/shiori-host32-ipc/src/lib.rs` の unsafe でない `send_copydata`・`send_copydata_response`・`send_copydata_with`・`send_request` が `# Safety` の節を持っていた。節の名を「# 前提」に替え、`# Safety` は本当に unsafe な `read_copydata` だけにした。
  10. 片付け: `crates/shiori-host32-host/src/parent_window_tests.rs` の `ignored_frames_are_logged` が、同じファイルの `ignored_reasons` と同じ絞り込みを手で書いていた。`ignored_reasons(&events)` を呼ぶ形にした。
- 見直して指摘にしなかったもの:
  - (a) helper の `main.rs` の REQUEST の腕（いま約 378 行目）で、UNLOAD を先送りした後に DLL の呼び出しの最中へ入れ子の REQUEST が来ると、`finish_deferred_unload` → `close` が外側の呼び出しの中で走りうる道: 届く道が無い。host は UNLOAD の後に何も送らない（`crates/shiori-host32-host/src/lifecycle.rs` の `request_clean_shutdown`・`crates/areka-kanade/src/shiori/real.rs:205`）。偽って送るのは 14.8 の議題 1 のとおり脅威の見立ての外。14.16 の (a) と同じ型で、欠陥ではない。
- 範囲の外の申し送り（`/kiro-complete` の棚卸へ）: 本番のハンドシェイクの上限は 5 秒（`shiori_wiring.rs`）、テストは 30 秒。ひどく混んだ初回起動では本番が上限を越えうる。変えるかは要件・設計の裁定なので、本 spec では変えない。
- clippy の回し直し: x64 は 14.16 と同じ `clippy::drop_non_drop` の 3 件だけ（7 の注が 1 行増えたので `tests/lifecycle_kill_e2e.rs` の 1 件は今 `:213`）。i686 は 0 件。

### 14.18 かけ直し 3 回目（タスク 14）

- 結果: 新しい指摘が 14 件出た。12 件は古い・誤った説明、2 件は言い分を証していないテストで、本番の振る舞いを変えるものは 0 件。14 件とも直した。
  1. テストの弱さ: `crates/shiori-host32-host/src/client_shiori2_tests.rs` の `probe_with` が `negotiator.current()` の前後の一致を断言していたが、交渉の状態は変わらない値で `probe_version` にも渡らず、いつも真だった。直し: その断言を消し、`client_tests.rs` の `the_one_way_event_body_contains_no_adoption_path` と同じ形の `the_probe_body_contains_no_adoption_path` を足した（`client.rs` の `probe_version` の本文に `parse_for`・`note_response`・`parse_response` が無いこと、`verdict_of_probe(` を通ることを見る）。本文に `// parse_response` の 1 行を一時に足して赤になる（`client_shiori2_tests.rs:151` の断言で落ちる）のを確かめてから戻した（`client.rs` は手を付けていない）。
  2. テストの弱さ: `crates/shiori-host32-helper/src/main_loopback_tests.rs` の UNLOAD の後の MessageLoop の段が、main のフィルタの写しを自分で回して「quit の配線の回帰の守り」と名乗っていた。写しは main の配線を通らないので守りにならない。直し: その段を消し、本当の守りは main の本物のフィルタを通す `tests/lifecycle_cyclic_e2e.rs` の `ExitKind::Clean` の検査だと 1 行の注で示した。
  3. 説明の古さ: `main_loopback_tests.rs` の `unload_during_a_dll_call_defers_the_unload` の説明の「（今は `proxy` の借りの衝突で panic して落ちる）」を消した（指摘 8 の直しの後は落ちない）。
  4. 説明の古さ: `crates/shiori-host32-host/src/shiori3_charset_tests.rs` の出所の `git show 4db516fe:…` は今の履歴から引けない（squash で消えた）。そう書いた。`areka/src/emo2_boot/spine_conformance_script.rs` は今 `crates/areka/src/runtime/spine_conformance_script.rs`（在ること・`SHELL_NAME` があることを確かめた）に直した。
  5. 説明の誤り: `shiori3_tests.rs` の `build_compound_status_full_request_is_verbatim_in_place` の説明が `Status` の位置を「`Sender` の直後」と書いていた。逐語の期待どおり「`SenderType` の直後」に直した。
  6. 説明の誤り: `tests/lifecycle_kill_e2e.rs` の 2 か所が、実効の timeout を client を作るときに解決すると書いていた。`client.rs` の `get_with_headers`・`notify_with_headers` は往復のたびに `effective_timeout()` で env を読む。「往復ごとに解決」に直した。`set_var` に SAFETY の注を足した（このバイナリのテストは 1 本で、helper の spawn やスレッドの前に設定する）。
  7. 説明の誤り: `tests/error_paths.rs` の単体テストの道を `parent_window::tests::classify_tests::length_mismatch_is_ignored_as_bad` に直した（在ることを確かめた）。「`LengthMismatch` は実受信では原理的に発火しない」を和らげた: `read_copydata` は `cbData > 0` で `lpData` が空のとき空の payload と `len = cbData` を返すので、同じプロセスの送り手からは届き得る。冒頭の注と `send_raw` の注の 2 か所。
  8. 説明の誤り: `src/terminator_tests.rs` の `GetQueueStatus` の注が下位語を「今キューにある種類」と書いていた。上位語が今キューにある種類、下位語は前回の GetQueueStatus／GetMessage／PeekMessage の後に加わり今もキューにある種類。そう直した。コードは変えていない。
  9. 説明の古さ: `src/job.rs` のテストの注が KILL_ON_JOB_CLOSE で「child とその子孫」を kill すると書いていた。SILENT_BREAKAWAY_OK で子孫は job に入らない。「helper 役の child 自身・子孫は抜けている」に直した。
  10. 説明の古さ: `src/process_host.rs` の `HelperHandle::terminate` が「検証用 additive・統合 5.2 の観測点」のままだった。今は本番の kill の道（`HelperHandle` の `Drop` と `HelperLifecycle::terminate` が使う）。そう直した。
  11. 説明の古さ: helper の `src/main.rs` の 3 か所。冒頭の「不正フレームはカウンタに記録するのみ」に「`[helper]` の行に残す」を足した。`TriggerUnload` の説明の流れを今の形（proxy の `close`＝unload → FreeLibrary → `quit_requested` → ack `[1]` → `PostMessageW`、DLL の呼び出しの最中は ack を先に返し `close` は戻ってから）に直した。`HelperShared` の「観測カウンタのみ」に終了・再入の印（`quit_requested`・`in_dll_call`・`unload_pending`）を足した。
  12. 説明の古さ: `src/shiori_proxy_loadu_tests.rs` の「既存 `mod tests`（`shiori_proxy.rs` 末尾）」を `shiori_proxy_tests.rs` に直した。
  13. 限界の注が無かった: `process_host.rs` の `forward_helper_stderr` の `read_until(b'\n')` に行の長さの上限が無い。`// shortcut: 1 行の長さに上限なし（改行の無い大きな書き込みは丸ごと貯まる）、実害が出たら上限で切る` の注を置いた（下の決め 1）。
  14. 説明の古さ: `crates/shiori-host32-host/README.md` の手順 ③ の見出しが「proto の 32bit 可搬性テスト」のままだった。今は helper の proxy・loopback・再入・固まった窓のテストも回る（約 42 秒）。そう書き直した。
- 数えなかった小さな直し: `crates/shiori-host32-ipc/src/lib.rs` の冒頭が同居する項目を並べているので `read_copydata`（受信の取り出し）を足した。
- 決め 1（標準エラーの行の上限）: 上限は設けず、`shortcut:` の注で残す。design-validation.md（64 行目）の見立て「直すなら `shortcut:` の注釈で足りる」と、Implementation Note 4.3（14 で見る）に従った。今の負荷（helper の `[helper]` の行と SHIORI の DLL の診断の行）では当たらない。
- 決め 2（`wait_handle` の WAIT_FAILED）: 今のまま、時間切れと同じに扱う（Implementation Note 6）。呼び手は記録を出す（`ExitTimeout` の error、または読み手を切り離した旨の debug の行）ので黙った失敗ではなく、正しいプロセスの取っ手では WAIT_FAILED は起きない。
- 見直して指摘にしなかったもの:
  - (a) `crates/shiori-host32-helper/src/shiori_proxy_tests.rs` と `crates/shiori-host32-testdll/src/lib.rs` の「courtesy unload」の語: 正しい。UNLOAD を経ない道（no-UNLOAD）では `Drop` → 後始末 → `unload` が本当に走り、`testdll_drop_invokes_courtesy_unload` が testdll の書く印のファイルでそれを証す。14.17 の 5 で直したのは UNLOAD の道（今は `close`）の説明だけ。
- clippy の回し直し: x64 は 14.16 と同じ `clippy::drop_non_drop` の 3 件だけ。6 の SAFETY の注で 2 行増えたので `tests/lifecycle_kill_e2e.rs` の 1 件は今 `:215`（`lifecycle_cyclic_e2e.rs:162`・`:287` は動いていない）。i686 は 0 件。

### 14.19 かけ直し 4 回目（タスク 14）

- 結果: 新しい指摘が 3 件出た。どれも説明とテストの文言で、本番の振る舞いを変えるものは 0 件。3 件とも直した（コードは変えていない）。
  1. 言い分がテストの証すより大きい: `crates/shiori-host32-host/tests/error_paths.rs` の経路 2 と `src/parent_window_tests.rs` の (e) が「無応答の相手への `send_request` は `SMTO_ABORTIFHUNG` ＋上限時間で `Timeout` を返す（要件 5.2/5.3）」と名乗っていた。宛先は同じスレッドが作った窓（親自身）なので `SendMessageTimeoutW` は WndProc を直に呼び、上限時間も `SMTO_ABORTIFHUNG` も効かない。`Timeout` は `shiori-host32-ipc` の `send_request` の `slot.take().ok_or(IpcError::Timeout)`（slot が空）から来る。直し: 冒頭の注・関数の説明・段の注・断言の文を「応答なし → slot が空 → `Timeout` に写る（要件 5.2）」に直し、上限時間そのものは別スレッドの窓を使う `crates/shiori-host32-ipc/src/send_timeout_tests.rs` が確かめると書いた。冒頭の「wedge」の語も外した。テストの関数名も `window_error_paths_handshake_timeout_wedge_and_corrupt_frame_isolation` から `window_error_paths_handshake_timeout_no_response_and_corrupt_frame_isolation` に改めた（コードとテストにほかの参照は無い。残るのは `.kiro/specs/completed/` の過去の検証記録だけで、記録なので触らない）。
  2. 説明の古さ: `crates/shiori-host32-host/Cargo.toml` の `log-capture-kit` の dev-dependency の注が「`CharsetNegotiator` のログを観測するためだけ」のままだった。今は lifecycle・`parent_window_tests`・`process_host_tests`・`process_host_stderr_tests`・`process_host_image_tests`・`client_tests`・`client_shiori2_tests` も使う。「host のテストが記録（件数・レベル・`event` の欄）を観測するための」に直した。注だけで、`Cargo.lock` は変わらない。
  3. 説明の古さ: `crates/shiori-host32-host/README.md:68` の `--test error_paths` の注が「要件 1.4/2.5/3.4/5.x」だった。1.4 のテストは消した（14 の決め 7）。今 `error_paths.rs` に残る経路は 3.4・2.5・5.2 なので「（要件 2.5/3.4/5.x）」に直した。
- 見直して数えなかったもの（4 回目の監査役の申し送り）:
  - (a) testdll の「二重解放の検知」の語は言い過ぎ: `GlobalFree` の戻り値を捨てているので検知はしていない。テスト用 DLL の説明の語で、本番の振る舞いに関わらない。
  - (b) helper の `std::env::args().nth(n)` は対にならないサロゲートを含む引数で panic する: panic は記録に残り、実際のフォルダ名では起きない。
  - (c) `response_not_from_helper` の理由は、ハンドシェイク前に届いた応答にも付く: 捨てて記録する振る舞いは正しく、理由の語の幅の話。
  - (d) 入れ子の LOAD: 14.16 の (a) で決着済み。
- clippy の回し直し: x64（`-p shiori-host32-host -p shiori-host32-ipc --all-targets`）は 14.16 と同じ `clippy::drop_non_drop` の 3 件だけで、行は動いていない（`tests/lifecycle_kill_e2e.rs:215`・`lifecycle_cyclic_e2e.rs:162`・`:287`）。
