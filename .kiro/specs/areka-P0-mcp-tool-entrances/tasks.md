# Implementation Plan

> 設計の正本は design.md。ファイル・型・関数の名前は design.md の「File Structure Plan」と「Components and Interfaces」に従う。テストは実装の隣の `*_tests.rs`（`#[cfg(test)] #[path = "…"] mod …;`）。ツールごとのテストの接続はそのツールのファイル自身に書く（`mod.rs` に書かない）。1 ファイル 1,000 行を超えない。常時テストはネットへ出ず、ポートは OS に割り当てさせる（9801・9821 その他の固定の番号を束ねるテスト 0 本）。10 秒の上限を実際に待つテストは書かない（上限は引数で短くする）。
> `Cargo.toml` は根・`crates/areka-mcp`・`crates/areka` のどれも 0 行。触るファイルは design.md「File Structure Plan」の範囲に限り、ほかを触る要が出たら止めて報告する。

- [ ] 1. 土台: 両側のモジュールとテストの骨組み
- [x] 1.1 プロトコル側（`areka-mcp`）に検査とツールのモジュールの骨組みを置く
  - `lib.rs` に `mod check;` と `pub mod tools;` の 2 行を足す（既存の `pub use` は変えない）
  - `tools/` の下に表・橋・結果の形・10 本のツールのファイルを空で作り、各テストファイル（`check_tests.rs`・`tools_tests.rs`・`tools_socket_tests.rs`・`bridge_tests.rs`・`outcome_tests.rs`）の接続も置く。以後の並走タスクが `lib.rs`・`tools/mod.rs` の宣言を取り合わないようにする
  - 完了の姿: `cargo build -p areka-mcp` と `cargo test -p areka-mcp --no-run` が緑で、`mcp-server-core` の既存テストが 0 行の変更のまま緑
  - _Requirements: 7.1, 8.2_
- [x] 1.2 アプリ本体側（`crates/areka`）に `mcp` モジュールの骨組みを置く
  - `main.rs` に `mod mcp;` の 1 行だけを足し、`mcp/` の下に `mod.rs`・`resolve.rs`・10 本のツールのファイルとそれぞれの `_tests.rs`（`mcp_tests.rs`・`resolve_tests.rs` を含む）を空で作る
  - 完了の姿: `cargo build -p areka --bin areka` と `cargo test -p areka --bin areka --no-run` が緑（使われない項目の警告は後のタスクで消える）
  - _Requirements: 7.1, 7.4_

- [ ] 2. プロトコル側の部品: 結果の形・検査・10 本の定義
- [x] 2.1 (P) 結果の 4 つの形を作る関数
  - 素の値（`OK:` なし・`isError: false`）、成功（付言が空なら本文 `OK`、あれば `OK:<付言>`）、失敗（`NG:<理由>`・`isError: true`）、本文の後に base64 済みの PNG を 1 枚足す形（`mimeType: "image/png"`）
  - 完了の姿: `outcome_tests.rs` で 4 つの形と `ok("")` が `OK`（コロンなし）になることが緑
  - _Requirements: 7.2_
  - _Depends: 1.1_
  - _Boundary: tools/outcome.rs_
- [x] 2.2 (P) inputSchema に照らした引数の検査
  - 「必須の欄 → 型」の順に調べ、最初の誤りの理由（`missing required argument: <名前>`／`argument <名前> must be <型>`）を返す純粋な関数を作る。理由は `failed to deserialize parameters:` で始めない
  - 必須の欄は「無い・`null`」で拒むが、`ghost_name` は調べない。型は `null` でない値だけを string・boolean・integer・array（`items.type` があれば各要素も）で調べ、`properties` に無い欄と未知の `type` は見ない
  - integer の読み方を 1 つの関数にする（`as_i64` が取れればその値、小数部 0 かつ絶対値 2^53 以下の数は受ける、`1.5`・`i64` に収まらない数・数でない値は拒む）。各ツールの詰め替えも同じ関数を使う
  - 完了の姿: `check_tests.rs` で手書きの schema に対する規則の表（必須の欠落・`null` の必須・`ghost_name` の欠落は通る・余計な欄は通る・各型の違反・`1.0` は通る・配列の要素違反・理由の文の前置き）が緑
  - _Requirements: 2.2, 2.3, 2.4, 2.5, 2.7_
  - _Depends: 1.1_
  - _Boundary: check.rs_
- [x] 2.3 10 本の定義の逐語・型の付いた引数・要求の種類の列挙
  - 各ツールのファイルに、保存した JSON（`doc/ssp-mcp/tools-list-ssp-2.9.05.json`）のそのツールの 1 個ぶんを生文字列で逐語に貼り、design.md の表どおりの `Args` と、検査済みの引数を詰め替える関数を置く（省略と `null` は `None`・`references` の省略は空の列・`ghost_name` の空の文字列はそのまま渡す）
  - 表（`tools/mod.rs`）に 10 本を SSP の並び（先頭 `get_active_ghost_list`）で並べ、10 変種の `ToolCall` とツール名を返す関数、`REPLY_WAIT`（10 秒）を置く。表の行（定義の文字列と詰め替え）はクレートの中から読めるようにする（`check_tests.rs` が使う）
  - 本番のビルドはクレートの外のファイルを読まない（保存した JSON を読むのはテストだけ）
  - 完了の姿: `tools_tests.rs` で「表が 10 行・先頭が `get_active_ghost_list`・10 本の定義が読めて `ToolSpec` になる・10 本の詰め替えが型の付いた値になる（省略・`null`・`references` の省略を含む）」が緑
  - _Requirements: 1.1, 1.2, 1.3, 5.3, 7.1, 7.6_
  - _Depends: 2.2_
- [x] 2.4 10 本の定義に対する検査のテスト
  - 10 本の定義は 2.3 の表から読む（保存した JSON は読まない）
  - 10 本それぞれについて「欄が全部ある・必須の欄が無い・`null` の必須の欄・欄ごとの型違い（1 つ以上）・`null` の任意の欄・余計な欄」の組を、各ツールの定義を読んで検査に通す
  - `get_expression_table` の `ghost_name` が無いときは通ることを入れる
  - 完了の姿: `check_tests.rs` の 10 本ぶんの表が緑で、どのツールでも理由の文が `failed to deserialize parameters:` で始まらない
  - _Requirements: 2.2, 2.3, 2.4, 2.5, 2.8_

- [ ] 3. 橋: 届けて塞がずに待つ
- [x] 3.1 要求と返事の対（`ToolRequest`・`ReplyTo`・`Pending`・`Answer`）
  - `ToolRequest::new` が要求と返事を受ける側の対を作る（本番の橋もテストもこの関数で作る）。`ReplyTo` は `Send` で、記録用のゴーストの名前を添えられ、1 回だけ送れる。受け手がもう居なければ送りは黙って捨てられる
  - `ReplyTo` の `Drop` は送り手を先に落としてから合図を立てる（合図を立てるのはここ 1 か所）。`Pending` の `Drop` は逆向きの合図を立て、`ReplyTo::is_abandoned` がそれを読む
  - `Pending::try_answer` で待たずに覗ける。公開面に rmcp・tokio・tokio-util の型を出さない
  - 完了の姿: `bridge_tests.rs` で「`is_abandoned` は `Pending` が生きている間 false・落とすと true」「送らずに落とすと `try_answer` が `Dropped`」「`Pending` を落とした後の `send` は何も起こさない」が緑
  - _Requirements: 6.2, 6.4_
  - _Depends: 2.1, 2.3_
- [x] 3.2 送って待つ処理と上限・終了の途中・記録
  - 送り口へ要求を送り、上限のうちに合図が立つのを `.await` で待ってから、返事あり／上限（`NG:` ＋上限の文言・`warn!` 1 件）／手放し（`NG:areka is shutting down`・`warn!` 1 件）に分ける。送りが失敗したら待たずに手放しと同じ
  - どの枝でも最後に `debug!` を 1 件（ツール名・解決したゴーストの名前・`isError`・失敗の本文）。待つ間は合図の写しだけを持ち、フューチャを `Send` に保つ
  - 完了の姿: `bridge_tests.rs` でテストのスレッドの tokio を回し、⑴ 返事をしない受け手と上限 50 ms で上限の `NG:`・`warn!` 1 件・`debug!` 1 件、⑵ 受け口を落としてから呼ぶと長い上限を待たずに `NG:areka is shutting down`・`warn!` 1 件、⑶ 受けた要求を答えずに落としても同じ、⑷ 返事ありで `warn!` 0 件・`debug!` にゴーストの名前、⑸ 上限の後の送りは何も起こさない、⑹ `REPLY_WAIT` の文言が `areka did not respond within 10 seconds`、が緑
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.7, 6.8_

- [ ] 4. 登録表と handler（プロトコル側の結線）
- [x] 4.1 10 本の登録表と受け口を組む
  - 上限を引数に、表の各行の定義を `ToolSpec` にして「詰め替え → 送って待つ」を処理として登録し、登録表とアプリ本体が汲む受け口を返す。定義が読めない行は `error!` 1 件で登録しない
  - 受け口は呼び出し側が持ち、アプリ本体へ置くまでの要求は溜まる
  - 完了の姿: `tools_tests.rs` で、組んだ登録表が 10 本を SSP の並びで持ち、その処理を呼ぶと受け口に型の付いた `ToolCall` が届くことが緑
  - _Requirements: 1.1, 1.3, 6.5_
- [x] 4.2 handler に検査・登録順の一覧・`instructions` を入れる
  - 写しの中で、`arguments`（無ければ空）を検査に通し、失敗は `debug!` 1 件を残して `-32602`（`invalid_params`）にする。通れば今までどおり登録した処理を呼ぶ
  - 登録順の定義の列を持ち、`tools/list` はそれを返す（無状態版の `ttlMs`・`cacheScope` の分岐はそのまま）。4 つの欄以外を付けない既存の写し方を保つ
  - `instructions` を design.md の英文 3 文に改める
  - 完了の姿: `mcp-server-core` の既存テスト（`server_*_tests.rs`・`registry_tests.rs` ほか）が 0 行の変更のまま緑。検査の `-32602` と登録順の一覧の振る舞いは直後の 4.3 の実ソケットのテストで固定する
  - _Requirements: 1.3, 1.4, 1.5, 2.1, 2.2, 2.6, 2.7, 8.2, 8.4_
- [x] 4.3 実ソケットのテスト（一覧の一致・`-32602`・9 本の到達・待ちの間の `ping`）
  - 保存した JSON を `CARGO_MANIFEST_DIR` から読み、旧式と無状態版の両方の `tools/list` の `tools` と配列ごと一致させる（並び・欄の過不足を 1 度に見る）
  - 未知の名前・必須の欄の欠落・型違いの 3 本で `-32602`・`result` 無し・受け手への要求 0 件を確かめ、旧式と無状態版の HTTP の状態を測って固定する
  - 9 本それぞれを呼び、偽の受け手が受けた `ToolCall` が期待の型の付いた値と等しく、受け手が返した `NG:not implemented yet` がそのまま応答になる
  - 返事を止めた `tools/call` の待ちの間に `ping` と `tools/list` が答える
  - 完了の姿: `tools_socket_tests.rs` の上の 4 群が緑（ループバックの空きポートだけ）
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 2.1, 2.2, 2.4, 2.7, 2.8, 5.1, 5.3, 5.4, 6.3, 6.8, 8.1_

- [ ] 5. アプリ本体側: 解決・ツールの処理・汲む系
- [x] 5.1 (P) 起動中のゴーストの読み取りと `ghost_name` の解決
  - World から起動中のゴースト 1 体（descript の `name`・絶対化したルートフォルダ）を読む。置き場が空・実行系が無ければ無し（LogSink へ倒れた単位も実行系があれば数える）
  - 解決の判断を純粋な関数にする: 省略・空は `Reject` なら `NOT_ACTIVE`、`UseActive` なら起動中の 1 体（0 体なら `NOT_ACTIVE`）。空でなければ `name` の完全一致か、ルートフォルダとの比較（大文字小文字・区切り・末尾の区切りの差を同じとみなす）で一致、どちらでもなければ `CANNOT_FIND`
  - 一覧に出す値（`name`、無ければルートフォルダのフルパス・末尾の区切りなし）を作る関数を置く
  - 完了の姿: `resolve_tests.rs` で要件 3.9 の全場合（相対パスを含む）と、一覧の値（名前あり・`name` 無し）が同じゴーストへ解決されることが緑。同じファイルで「置き場が空」「実行系の無い単位（`GhostSession::for_test`）」の読み取りが無しになることが緑
  - _Requirements: 3.2, 3.3, 3.4, 3.5, 3.6, 3.8, 3.9, 4.2, 4.4, 4.5_
  - _Depends: 1.2_
  - _Boundary: mcp/resolve.rs_
- [x] 5.2 `get_active_ghost_list` の本物の処理と 9 本のダミー
  - `get_active_ghost_list` は一覧に出す値を素の値で送る（1 行・末尾の改行なし・0 体なら空の本文）
  - 9 本は design.md の形の処理で、本体は `NG:not implemented yet` を送る 1 文だけ（World・ゴースト・引数を使わない）。文言は各ファイルに直に書く
  - 完了の姿: `get_active_ghost_list_tests.rs` で名前あり・`name` 無し・0 体がどれも `isError: false` で期待の本文、9 つの `_tests.rs` で空の World と作ったゴースト・引数から `try_answer` が `NG:not implemented yet`・`isError: true` を返すことが緑
  - _Requirements: 4.1, 4.2, 4.3, 4.5, 5.1, 5.2, 5.4_
  - _Depends: 2.1, 2.3, 3.1, 5.1_
- [ ] 5.3 受け口・汲む系・振り分け・後から答える置き場・閉じる
  - 受け口と後から答える置き場を NonSend 資源として置く関数、汲む系を Input 段（`dispatch_pointer_events` の後）へ登録する関数、両方を World から外す関数を作る。`ghost_session::register_systems` に汲む系の登録を 1 行足す（`crate::update::register(world);` の次・`ghost_session.rs` で触るのはこの 1 行だけ）
  - 汲む系は溜まった要求を全件取り出し、1 件ごとに起動中のゴーストを読み直して振り分け（design.md の表のとおり `ghost_name` の扱いを分け、失敗は `NG:` で終える。成功はゴーストの名前を添えて処理へ）、その後で預かった組を全件覗く。受け口が無ければ無操作
  - 後から答える置き場: 組を World から取り出して回し、`Some` を返した組は送って外し、待つ側が居ない組は覗かずに捨てる。置き場が無いときに預けた組はその場で落ちる
  - 完了の姿: `mcp_tests.rs` で、振り分け（0 体で 8 本 → `NOT_ACTIVE`・1 体で `get_expression_table` の省略 → `NOT_ACTIVE`・名前違いで 8 本 → `CANNOT_FIND`・0 体の `get_log` と 1 体の省略の 7 本は解決の文言にならない）、後から答える（次のフレームで届く・預けたフレームで届く・預けたまま閉じると `Dropped`・`Pending` を落とした組は覗かれない・閉じた後の預けは即 `Dropped`）、準備の前に送った要求が置いた後に答えられる、閉じると溜まった要求が `Dropped` で以後の送りが失敗する、受け口が無いときの汲みが無操作、が緑
  - _Requirements: 3.1, 3.4, 3.5, 3.6, 3.7, 6.1, 6.4, 6.5, 6.6, 6.8_
- [ ] 5.4 本物の単位で `get_active_ghost_list` を通すテスト
  - テスト用の起こし方（`emo2_boot/ghost_switch_test_support.rs` の `SwitchRig::new`＋`SwitchRig::boot`。`register_systems` を通るので 5.3 の汲む系も登録される）で実行系つきの単位を起こし、受け口を置いて `get_active_ghost_list` の要求を送り、1 フレーム回す（または汲む関数を直に呼ぶ）と答えが descript の `name`・`isError: false` になる
  - LogSink へ倒れた単位（作り方は `ghost_session_strict_tests.rs` の前例）でも起動中のゴーストとして読めること
  - `emo2_boot/` は 0 行（起こし方は `pub(crate)` で見える）。触る要が出たら止めて報告する
  - 完了の姿: `mcp_tests.rs` の 2 本が緑
  - _Requirements: 3.8, 4.1, 6.1_
  - _Depends: 5.3_

- [ ] 6. アプリ本体への結線
- [ ] 6.1 `fn main()` と系の登録に橋をつなぐ
  - `areka_mcp::start` の前で 10 本の登録表と受け口を組んで登録表を渡し、`register_systems` の後で受け口を World に置き、`app.run()` の直後・`exit_wait::begin_close` の前で閉じる
  - 完了の姿: `cargo build -p areka --bin areka` が緑、`main.rs` の main からの増分が 1.2 の `mod mcp;` を含めコメント込みで 12 行以内（954 → 966 行以内）、`ghost_session.rs` の main からの増分が 1 行（5.3）、`exit_wait.rs`・`emo2_boot/` が 0 行
  - _Requirements: 6.1, 6.4, 6.5, 7.4_
  - _Depends: 4.1, 5.3_
  - 汲む系の登録（`ghost_session.rs`）は 5.3 で済んでいる

- [ ] 7. 差の一覧・台帳・全体の確認・実機
- [ ] 7.1 差の一覧と roadmap の干渉台帳
  - `doc/ssp-mcp/transport-diff-areka.md` の「未知のツール名・必須引数の欠落」の行を、4.3 で測った値（エラーの番号・`message`・旧式と無状態版の HTTP の状態）と測ったテストの名前で書き換え、判定と判定の数の行を合わせる（空欄 0）
  - `.kiro/steering/roadmap.md` の干渉台帳に design.md「3 段目の spec が触るファイル」の表と C3 の照合の要点を転記する
  - 完了の姿: 2 つの文書の差分に上の内容が載り、差の一覧の該当行に空欄が無い
  - _Requirements: 2.7, 7.3, 8.3_
  - _Depends: 4.3_
- [ ] 7.2 全体の確認
  - `tools/test-all.ps1` を通し、`Cargo.toml`（根・`crates/areka-mcp`・`crates/areka`）の差分 0 行、触ったファイルが design.md の範囲に収まること、`areka-mcp` の本番のコードがクレートの外のファイルを読まないことを確かめる
  - 完了の姿: `tools/test-all.ps1` が緑、`git diff main --stat -- '*Cargo.toml'` が空、触ったファイルの一覧が design.md「File Structure Plan」と一致
  - _Requirements: 7.4, 7.5, 7.6, 8.1, 8.2_
- [ ] 7.3 実機確認
  - 配布形の `areka.exe` を既定ゴースト（emo2）・`RUST_LOG=areka_mcp=debug` で起動し、Claude Code に `claude mcp add --transport http` で登録して要件 8.5 の ⑴〜⑹ を順に確かめる。実機の根・一時フォルダはワークツリーの `target\` の下に置く
  - 必須の欄 `script` を抜いた `sakurascript` を 1 回呼び、エージェントに見えた文をそのまま書き残す（理由の文が見えなければ差の一覧に書き、直すかは別の spec へ）
  - 完了の姿: `verification/signoff.md` に ⑴〜⑹ と追加の 1 項目の結果（記録の行の抜き書きを含む）が残る
  - _Requirements: 6.7, 8.5_

## Implementation Notes

- 2.3: 変異の確認の後に `git checkout` で戻すと未コミットの実装まで巻き戻る（2.3 で一度起きて作り直した）。変異は Edit で戻し、git の巻き戻しは使わない
- 2.2〜2.3: `check.rs`・`tools/` の呼び手は 4.1・4.2 で入るので、それまで非テストのビルドに dead_code 警告が出る（関門は無い）。4.2 の後に警告 0 を確かめる
- 4.3: 測った値（rmcp 3.5.0・7.1 で差の一覧へ）: 3 本とも `{"code":-32602,"message":…}`（`data` 無し・`result` 無し）・旧式 HTTP 200・無状態版 HTTP 400。`message` は未知の名前 `tool not found`／欠落 `missing required argument: script`／型違い `argument strict must be boolean`。テスト名 `unknown_name_is_invalid_params`・`missing_required_is_invalid_params`・`wrong_type_is_invalid_params`（`tools_socket_tests.rs`）。無状態版の `tools/call` は `Mcp-Name` 見出しが要る
