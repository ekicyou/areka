# 設計の確かめ: areka-P0-real-run-mcp-kit

- 日付: 2026-10-18
- 対象: `design.md`（`f829e88b`）・`requirements.md`（末尾の「要件の討議で決まったこと」1〜5 を含む）・`research.md`
- 読んだ steering: `product.md`・`tech.md`・`structure.md`・`roadmap.md`（C9 の行 ⑥・1 本だけの席）
- 判定: **GO**（下の 3 件は設計の討議で直す。どれも窓口のクレートの中の 1 か所で直せ、骨格は変わらない）

## 1. まとめ

設計は、要件 1〜10 の各項目を部品（launch・run・output・mcp・close・evidence・i686・偽の子・見張りの直し）に割り当て、トレーサビリティの表で全部を引けるようにしている。頼っている areka の記録の綴り・公開の部品・rmcp の呼び方は、コードと 1 つずつ突き合わせて食い違いが無かった。直すべき点は 3 件で、どれも「既定の形が、すぐ次に使う spec や手元の使い方で困る」種類のもの。骨格の作り直しは要らない。

## 2. コードとの突き合わせ（確かめた主張）

| 設計の主張 | 確かめた場所 | 結果 |
|---|---|---|
| 閉じる知らせの受け取りの行 `event="os_close_request"`（出所 `areka::app_exit`・`info`） | `crates/areka/src/app_exit.rs` `on_ghost_os_close`（279 行〜） | 合う。wintf 側にも同名の `info` の行（`crates/wintf/src/ecs/window_proc/lifecycle.rs` 121 行）があり、どちらも受け手が呼ばれたときだけ出る |
| 終わり方の行 `event="app_exit"`・`origin=?origin`・最初の出所だけ `info` | `app_exit.rs` `quit_app`（105〜133 行・2 度目は `debug` の `app_exit_again`） | 合う（D-5 の前提どおり） |
| 未結線の起動は `quit_app(OsClose)` で直ちに終わる | `app_exit.rs` 270〜277 行 | 合う（`EndedBy::UserClose` に `OsClose` を含めるのも正しい） |
| 保険の自動終了は `quit_app(Smoke)` を直に呼ぶ | `crates/areka/src/main.rs` 301 行 | 合う（D-19 の smoke ① の `SafetyExit` は成り立つ） |
| `OnCloseRequest` を `FrameFinalize` で差す・受け手が無い窓は消える | `app_exit.rs` `attach_os_close_request`・`ghost_session.rs` 82 行・wintf `WM_CLOSE` | 合う。tick の途中で World が借りられていると `WM_CLOSE` は記録なしで捨てられるが、D-3 の送り直しがこれも吸う |
| `taskkill`（強制なし）で別れの台詞を経て終わる先例 | 完了 `app-lifetime-separation` tasks.md 106 行・`mcp-get-status` signoff 7 行・`ghost-restart-unit` tasks.md 116 行 | 合う（4 枚の窓へ届いても握手が吸う） |
| 待受の 3 行の綴りと段 | `crates/areka-mcp/src/server.rs` 37・60・78・80 行 | 合う（始めた＝`info`・束ねられない＝`error`・待ち受けない＝`info`） |
| 標準出力の層は `RUST_LOG`（未設定・不正は `info`）・履歴の層は絞らない | `crates/areka/src/log_history.rs` `init`（411〜418 行） | 合う |
| `areka_user_data::copy_tree_atomic` が公開 | `crates/areka-user-data/src/lib.rs` 11 行 | 合う。ただし `<work_root>\.copy-work\` の棚を作る（回の置き場に空の棚が残る・害は無い） |
| `AREKA_USER_ROOT` を渡さないと書ける根は実行ファイルの隣 | `areka-user-data/src/lib.rs` `decide`（winget の控えも書けない理由も無ければ `Beside`） | 合う（`target\` の下） |
| `areka_mcp_bridge::scan::scan`・`Scan`・`Link`・`config::Timeouts` が公開 | `crates/areka-mcp-bridge/src/lib.rs`（`pub mod scan`・`pub mod config`）・`scan.rs` 20〜47 行・`config.rs` 27 行 | 合う。中継の `main.rs` も `current_thread` の器で同じものを回している |
| rmcp 3.5.0 の `CallToolRequestParams::new`＋`with_arguments`・`call_tool` → `Result<CallToolResult, ServiceError>`・`is_error: Option<bool>` | `rmcp-3.5.0/src/model.rs` 4268・4279・3973〜3999 行・`src/service/client.rs` 1966 行 | 合う（答えの片の型は `ContentBlock`） |
| 見張り `sample_path_guard_test.rs` の形（794 行・`workspace_manifests`・`production_dependencies_on` を使う） | `crates/log-capture-kit/tests/sample_path_guard_test.rs` 646〜696 行・`workspace_scan/mod.rs` 444・473 行 | 合う（D-2 の直しはこの形に素直に載る） |
| 空の根＋位置の引数でも起動が通る | `crates/areka/src/boot_config.rs` `resolve_boot_from`（argv は列挙も記憶も読まない）・smoke ③ が空の根で「ゴーストが見つかりません」になる | 合う |

決まりごととの照合:

- 待ちは観測で終わり、どの待ちにも締切がある（3.7）。眠るのは読み直しの間隔と送り直しの間隔だけで、決まった時間を眠ってから進む待ち・1 フレーム遅らせる解は無い。
- 止めるのは取っ手を持つ子だけ（`Child::kill`・`taskkill /PID` は取っ手が生きている間だけ）。名前や番号の決めつけで止めない（5.6）。
- 置き場はすべて `target\` の下（`TempPath::under_target`・検体の写しは `sample-ghost-kit` の置き場・仲介は `areka.exe` の隣）。
- 外の新しい依存は 0（rmcp・tokio・serde_json・windows・tracing はどれも `Cargo.lock` に在る）。MCP は公式の SDK（rmcp）で話す。
- `crates/sample-ghost-kit/src/lib.rs` に触らない（C9 の約束）。新しいファイルはどれも 1,000 行の内の見込み。

## 3. 直すべき点（3 件）

### 件 1: `compose_rust_log` が呼び手の記録の段を下げる

- **問題**: D-4 は「呼び手の値が出所の名前を含まないときだけ `,areka_mcp::server=info,areka::app_exit=info` を足す」。だが `EnvFilter` は、より細かい出所を名指した指定が勝つ。呼び手が `trace`・`debug`・`areka=debug`・`areka_mcp=debug`（完了 `mcp-get-status` の実機の記録で実際に使った形）を渡すと、足した `=info` が勝って、この 2 つの出所の `debug`／`trace` の行（例: `app_exit_again`・待受の候補を飛ばした `debug`）が標準出力から消える。設計の約束「呼び手の指定を下げない」に反する。
- **影響**: 実機の確かめで判定の分岐の段まで開けたつもりの行が黙って消え、「空振りしたこと」に気付けない（開発者方針「実機の確かめは判定の分岐の段まで `RUST_LOG` を開ける」）。
- **直し方**: 足すのは、その出所の実効の段が `info` より絞られているときだけにする。具体的には、⑴ 呼び手の値が出所そのものか親（`areka`・`areka_mcp`）を名指していれば足さない ⑵ 全体の段（出所の無い指定）が `info`・`debug`・`trace` なら足さない（未設定の既定 `info` もこれに当たる）⑶ 全体の段が `warn`・`error`・`off` で、出所も親も名指されていないときだけ足す。`launch_tests.rs` に `trace`・`debug`・`areka=debug`・`areka_mcp=debug` を渡すと値が変わらないこと、`warn,areka::boot_config=info` には足すことを加える。
- **要件**: 4.4・5.3・1.6
- **設計の箇所**: launch の「`compose_rust_log(base)` は…」の段落・D-4・Testing Strategy の `launch_tests.rs`

### 件 2: 残した回の置き場が、後の走りに使い回されうる

- **問題**: 回の置き場の名前は `areka-rr-<プロセスの番号>-<連番>` で、一意なのは「同じ時間に生きているプロセスの間」だけ。落ちた回は `forget` で残り続けるので、後の日に同じ番号を振られた試験のプロセスが同じ連番を引くと、`create_dir_all` は既にあるフォルダを黙って返す。そのとき ⑴ 前の回の `sample\` が在るので `copy_tree_atomic` が `Raced` を返し、今回の検体の写しが残らず、前の回の写しが今回の証拠に見える ⑵ 「空の根」の `root\` が空でないことがある ⑶ 今回が成功すると `TempPath` の破棄が前の回の証拠を消す。設計の「残した回は誰も消さない（名前は毎回新しい）」は成り立たない。
- **影響**: 落ちた回の証拠（6.1）が混ざる・消える。Windows はプロセスの番号を使い回すので、証拠が溜まるほど起きやすくなる。
- **直し方**: `temp-path-kit` には触らず、`RunDirs::create` の中で、配られたフォルダが空でなければその `TempPath` を `forget`（前の証拠を消さない）して次を取り直す（上限つきの繰り返し・上限を超えたら panic）。`evidence_tests.rs` に「中身のあるフォルダを配られたら使わず、消しもしない」を足す。手引き（`doc/testing.md`）に、残った回の置き場は自分で消すこと（`target\test-roots\areka-rr-*`）を書く。
- **要件**: 1.4・6.1・9.1
- **設計の箇所**: evidence の Responsibilities と「Persistence & consistency」・D-7

### 件 3: 出力の待ちが毎回全文を読み直す前提が、すぐ次の spec で崩れる

- **問題**: output は 50 ms ごとに 2 つのファイルの全文を読み直して探し、「待つ試験は `trace` を開けない前提（`emo2_real_run` は待たずに終わりだけを待つ）」としている。だが窓口を使う次の spec `emo2-real-run-stability` の件 1 は、`RUST_LOG=info,kanade=trace` のまま `wrap=BudouxWordWrap` が出るのを待ってから閉じる形にし、件 2 ではそれを数十回くり返す（同 brief 233・244 行）。閉じ方の受け取りの行の見張り（close）も同じ全文の読み直しを使う。数 MB の出力を 1 秒に 20 回読み直して探すと、走りの長さの 2 乗で重くなる。
- **影響**: 開発機の負荷（開発者方針「トークンの次に PC の負荷を抑える」）。実機の窓が出る試験の時間の読みも狂う。output の Risks に書いた「超えたら読んだ位置から先だけを読む形へ替える」が、最初の利用者で要る。
- **直し方**: 最初から、ファイルごとに読んだ位置を持ち、増えた分だけを読む形にする（境目をまたぐ一致のために、前回の末尾の「探す文字列の長さ − 1」バイトを重ねて探す）。`Run::stdout`・`output` の読み口（3.6）と締切切れの文の末尾 64 KiB は今のまま。`output_tests.rs` に「境目をまたいで書かれた文字列を拾う」を足す。合わせて、`call_until` の呼び直しの間隔は出力の見回り（50 ms）と分け、今の `mcp_get_log` と同じ 250 ms 程度にする（`get_log` の答えは履歴の全文で大きい）。
- **要件**: 3.2・3.7・4.9
- **設計の箇所**: output の Responsibilities（「読み直しは毎回ファイルの全文…」）と Risks・D-9・Performance & Scalability

## 4. よいところ

1. **閉じ方の作りが堅い**: `taskkill /PID`（強制なし）を、受け取りの行か子の終わりを観測するまで 1 秒ごとに送り直す形（D-3）は、窓がまだ無い時点の失敗も、tick の途中で World が借りられていて `WM_CLOSE` が記録なしに捨てられる場合も、同じ 1 つの仕組みで吸う。止めるのは取っ手を持つ子だけで、`unsafe` も新しい依存も足さない。終わり方の見分け（D-5）も areka の `quit_app` の「最初の出所だけ `info`」という実際の振る舞いに正しく載っている。
2. **偽の子が本物の受け口を使う決定論のテスト**: 偽の子が本物の `areka_mcp::start` を呼ぶので、待受の 3 行の綴りがずれると常時のテストが赤になる。偽の子では檻にならない終わり方の綴りは、移し替えた実機の試験に 2 つの確かめ（D-19）を足して本物で固定している。檻の届く範囲と届かない範囲を正直に書き分けている。

## 5. 判定

- **GO**
- 理由: 既存の境界（areka の本番の振る舞い・`sample-ghost-kit`・`temp-path-kit`・中継）を変えずに、要件 1〜10 の全項目へ道が通っている。頼る綴りと公開の部品はコードで確かめられた。上の 3 件は、どれも窓口のクレートの中の 1 か所（`launch.rs` の `compose_rust_log`・`evidence.rs` の `RunDirs::create`・`output.rs` の読み方）で直せ、骨格・公開の型・依存の向きは変わらない。
- 次の手: 設計の討議（`/kiro-design-discussion areka-P0-real-run-mcp-kit`）で件 1〜3 を議題にし、決まった形を `design.md` に入れてから `/kiro-spec-tasks areka-P0-real-run-mcp-kit` へ進む。

## 6. 議題にしなかった気付き（記録だけ）

- `cargo test -p areka` では、窓口が引く rmcp の `client` と tokio の `macros` の機能が試験用に建てる `areka.exe` にも合わさる（resolver 2）。振る舞いは変わらないが、`cargo build -p areka` と交互に回すと建て直しが起きうる。ワークスペース全体の走り（`tools/test-all.ps1`）は中継が既に同じ機能を合わせているので変わらない。
- `Run` の中の rmcp の `Link` は、器（tokio）より先に捨てる並びにしておくこと（欄の並びで決まる・実装の段で気を付ける）。
- `copy_tree_atomic` は `<回>\.copy-work\` に空の棚を残す。害は無いが、手引きの「残した証拠の読み方」で触れておくと迷わない。
- roadmap の C9 の行 ⑥ の触るファイルの欄への追記（D-2）は、棚卸の側が行う決まりのまま。
