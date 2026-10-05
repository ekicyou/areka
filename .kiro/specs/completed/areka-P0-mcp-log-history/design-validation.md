# 設計の検証: areka-P0-mcp-log-history

> 検証日: 2026-10-04・本ブランチ（`claude/areka-p0-mcp-log-history-f3750e`）。対象は `design.md`（確定版）・`requirements.md`・`research.md`。
> 設計が既存のコードについて述べていることは、実物のファイルで確かめた（末尾「確かめたこと」）。
> 重大な指摘 1 は、`tracing-subscriber 0.3.23` のソースを読んだうえで、ワークツリーの `target\` の下に作った使い捨ての小さなクレートで再現した（記録した後に消した。リポジトリには残していない）。

## 総評

設計の骨組み（純粋な判断＋薄い継ぎ目・規則の表 1 つを 3 つのテストが読む・出す側 0 行）は要件に合っており、要件 1.1〜8.5 の全項目が設計の部品に対応している。境界の約束（出す側 0 行・`mcp/mod.rs` と `areka-mcp` 0 行・`Cargo.toml` 0 行）も守られている。
ただし、設計が選んだログの出口の形（履歴の層に `filter_fn` の層ごとのフィルタを掛ける）には、**既存の `tracing::enabled!` の前置ガードと組み合わさると履歴が記録を取りこぼす欠陥**がある（実験で再現）。直し方は本 spec の自分のファイルの中で 15 行ほどで済み、境界は動かない。これを設計ディスカッションで直す前提で **GO** とする。

## 重大な指摘（3 件）

### 指摘 1: 診断の target を `RUST_LOG` で点けると、履歴が warn・error・status の行を取りこぼす（実験で再現）

**何が起きるか。** 設計の出口は `registry().with(fmt::layer().with_filter(EnvFilter)).with(履歴の層.with_filter(filter_fn(…)))` で、areka に初めて「層ごとのフィルタ」を 2 つ持ち込む。`tracing-subscriber 0.3.23` の層ごとのフィルタは、`enabled` を尋ねられるたびに「このフィルタは断った」という印をスレッドごとの置き場に書き、その印は直後の `on_event` で消す作りである（`filter/layer_filters/mod.rs` の `Filtered::enabled`・`FilterState::did_enable`）。ところが `tracing::enabled!` は `enabled` を尋ねるだけで出来事を出さないので、印が残る。次にそのスレッドで出た出来事が「どの層も必ず欲しい」と覚えられた呼び出し口（＝`enabled` を尋ね直さない）だと、残った印のせいで履歴の層だけが 1 件飛ばす。

**いつ起きるか。** `tracing::enabled!` の前置ガードは今のコードに 3 か所ある。

- `crates/wintf/src/ecs/world/tick_diag.rs` の `is_enabled`（`try_tick_world` の冒頭で**毎回**呼ぶ）
- `crates/wintf/src/ecs/window/transition_diag.rs` の `is_enabled`
- `crates/areka/src/perf_thread_report.rs` の `is_enabled`（起動時に 1 度）

`RUST_LOG` でこれらの target を debug にして点けると（性能計測や tick の診断でふつうに使う設定）、標準出力の層は「欲しい」、履歴の層は「要らない」と答えが割れ、ガードのたびに履歴の側の印が残る。その直後に同じスレッドで出た warn・error（error 種別）や、標準出力にも出る status の info が、**標準出力には出るのに履歴には残らない**。点けていないときは起きない（呼び出し口が「関心なし」と覚えられ、`enabled` が尋ねられない）。

**実験の結果。** 設計と同じ形の出口で、ガード→`warn!` を繰り返した。

| `RUST_LOG` 相当 | ガードの答え | 標準出力 | 履歴の層 |
|---|---|---|---|
| `info` | false | 全部出る | 全部届く |
| `info,<診断の target>=debug` | true | 全部出る | **ガードの直後の 1 件が毎回届かない**（4 件中 4 件） |

**どの要件に反するか。** 1.7（`RUST_LOG` と独立に残す）・1.9（取りこぼし 0 件）・6.3（出した直後の問い合わせで取りこぼさない）。research.md §11 の「`perf_thread_report` は見立てではなく環境変数を読んでいるので影響しない」は事実と違う（実物は `tracing::enabled!(target: "areka::perf", DEBUG)`）。なお、この欠陥は最大レベルの見立て（`TRACE` か `INFO` か）とは無関係に起きる。

**直し方（実験で確かめた）。** 履歴の層のフィルタを `filter_fn` でなく、`tracing_subscriber::layer::Filter` を自分で実装した型にする。

- `callsite_enabled`: 出来事で、かつ `classify` が当たる → 必ず欲しい。それ以外 → 関心なし（設計と同じ答え）。
- `enabled`: **出来事でないもの（`enabled!` の問い合わせ・スパン）には true を返す**。出来事には `classify` の結果を返す。
- `max_level_hint`: `TRACE`。

これで印が残らない。同じ実験で、取りこぼしが 0 件になり、ガードの答え（点灯の判定）・標準出力の行・取り決めの target の debug／trace が履歴に届くことは変わらなかった（`RUST_LOG` 相当が `info`・`info,<診断>=debug`・`warn` の 3 通り）。変わるのは `crates/areka/src/log_history.rs` の中だけで、境界も依存も動かない。

**あわせて決めること。** この欠陥は、設計の実プロセスの試験（`RUST_LOG=warn,areka::boot_config=info`）では赤にならない。判定の純粋な部分（`callsite_enabled` と `enabled` が何を返すか）を関数に切り出して決定論テストで固定し、Revalidation Triggers に「履歴の層のフィルタは、出来事でない問い合わせを断ってはならない」を足すのがよい。

### 指摘 2: 「本物の受け口を通って届く」ことが固定されない経路が 2 つあり、持ち越し先が登記されていない

設計は「取り決めの target を debug・trace で出した行が、本物の受け口を通って履歴へ届く」ことを常時テストで固定しないと明示している。これ自体は受け入れてよいと考える（理由は下）。ただし次の 2 点が足りない。

1. **固定されないのは debug・trace だけではない。** `log` クレートから橋渡しされた warn 以上の行が履歴に届くこと（要件 2.10）も、常時テストでは `log.` で始まる欄を除く純粋な部分しか固定されず、橋を通ること自体の証跡は research.md §9.3 の実験だけである。「テストで固定されないこと（明示）」の節にこれも書くべきである。
2. **持ち越し先が「`mcp-kanade-tools` か `mcp-strict-errors`」で、どちらの文書にも約束が書かれない。** どちらも info で出すことにしたら、誰も固定しないまま残る。持ち越すなら、2 本のうち 1 本を名指しし、その brief（または roadmap の該当の行）に「debug で出した取り決めの行が実プロセスで履歴に届くことを固定する」を書く作業を、本 spec のタスクに入れる必要がある。

**取りうる道。**

- **(a) 今の設計のまま受け入れ、上の 2 点を足す（推奨）。** 振り分けの判断（debug の `areka::log::script` が script になる）は純粋なテストで固定されており、固定されないのは `tracing-subscriber` の仕組みと「見立てを `TRACE` と答える 1 行」だけである。指摘 1 の直しでフィルタを自分の型にすれば、その答えも純粋な関数として決定論テストに入る。
- **(b) 取り決めを「info 以上で出す」に狭める。** 見立てを `INFO` に戻せるが、要件 2.1・2.2 の「レベルを問わない」を改めることになり、「標準出力に出さずに履歴へだけ残す」（再生した台本を毎回 info で標準出力に流さない）という出す側の選択肢を失う。費用の実測（関心のない `debug!` が 1 回 1〜2 ns 増・`log` の `debug!` が 1 件 120〜140 ns・毎フレームの `log::debug!` は 0 件）から見て、狭める理由は弱い。
- **(c) `log-capture-kit` に「呼び手の層を載せて捕まえる」口を足して本 spec で固定する。** 見張りが止めている「自前の受け口を差す」ことの抜け道を共有の道具に作ることになり、設計判断 b が避けた方向である。本 spec の規模に対して重い。

### 指摘 3: 実プロセスの試験の、止め方と早い終了の扱いが決まっていない

試験の置き方（`CARGO_BIN_EXE_areka`・`AREKA_MCP_PORT` に空きの番号を明示・i686 の helper を自前で揃える・モニタ 0 台は告知と非 0 で受理）は `smoke_boot_loop_exit.rs` と同じ前提で、`tools/test-all.ps1` の「i686 成果物ビルド」→「x64 ワークスペース全テスト」の順でそのまま走る。番号を明示するので、SSP と取り合う 9801・9821 には触れない。ここまでは妥当である。次の 3 点は設計に書かれておらず、実装の段で揺れる。

1. **「答えを得たら子を止める」。** `areka.exe` を外から止めると、隣で起きている i686 の `shiori-host32-helper.exe` と検体の複製（一時フォルダ）の後始末が areka の終了処理を通らない。helper が残れば複製のフォルダが消せず、掃除漏れになる。既存の smoke は、締切を超えたときだけ止め、ふだんは `AREKA_APP_SMOKE_EXIT_MS` の自動終了を待つ。本試験も「答えを得た後は自動終了を待つ（待ち時間が惜しければ `AREKA_APP_SMOKE_EXIT_MS` を短くする）」か、「止めた後に helper が残らないことを確かめた」のどちらかを設計に書くべきである。
2. **子が先に終わったとき。** モニタ 0 台や起動の失敗では、子は待受を開く前後に非 0 で終わる。問い合わせの繰り返しは、子の終了（`try_wait`）も毎回見て、終わっていたら 60 秒を待たずに「受理」か「失敗」を決める必要がある。設計は「締切を超えたら失敗」しか書いていない。
3. **指摘 1 の再発を見張れる位置にある。** この試験は本物の受け口を通る唯一の常時テストである。`RUST_LOG` に前置ガードのある診断の target を 1 つ足し、その状態でも判定 1・2 の行が履歴にあることを見れば、指摘 1 の形の退行を実プロセスで捕まえられる。ただし、ガードの直後にどの行が来るかに依存するので、純粋なテスト（指摘 1 の「あわせて決めること」）を主にし、こちらは足せるなら足す程度でよい。

## 設計の強み

1. **規則の表 `RULES` を 1 つだけ持ち、振り分け・実在の判定・文書との一致の 3 つが同じ表を読む。** 要件 2.12 と 7.3 のテストは「表示するだけ」ではなく判定になっている。実在は target からファイルの場所を導き、導けない target は赤、文書の表は 0 行なら赤、集合が食い違えば赤である。名指しした 13 行のファイルと、`crates/areka-ghost/src/runtime.rs` の `target: "ghost-boot"`／`"ghost-shutdown"` の字面が今のコードにあることも確かめた。
2. **判断を全部、受け口にも World にも触れない関数に寄せている。** `classify`・`draft`・`History`・`answer`・`render` は値を渡すだけでテストでき、見張り（捕捉先を直接差す呼び出しの新設を赤にする）に触れずに要件 1〜5 を固定できる。`resolve` が空の `ghost_name` を「省略」と読むことを実物で確かめたうえで、空を自分で `CANNOT_FIND` にする扱いも正しい。

## 判定

**GO（条件つき）。** 指摘 1 を設計ディスカッションで直す（履歴の層のフィルタを自分の型にする）ことが条件である。直しは `log_history.rs` の中に閉じ、境界・依存・ファイルの構成は動かない。指摘 2・3 はディスカッションで道を決めれば足りる。

**次の一歩**: `/kiro-design-discussion areka-P0-mcp-log-history` で上の 3 件を扱い、その後 `/kiro-spec-tasks areka-P0-mcp-log-history`。

## 開発者の判断なしで直せる小さな点

- **図と本文の食い違い。** System Flows の図と箇条書きは「絞り込みと書式は排他の外」と書くが、`handle` の説明と Performance の節は `log_history::read(|h| answer(h, …))`＝排他の中で絞って整形する、と書く。どちらかに揃える。排他の中で回すなら「`answer` の中でログを出さない」は約束でしか守られない（出すと同じスレッドで履歴の層が排他を取りに行って固まる）ので、絞った記録を写して排他を放してから整形する形の方が、固まる道が構造として無くなる。
- **research.md §11 の誤記。** 「`perf_thread_report` は環境変数を読んでいる」は誤りで、実物は `tracing::enabled!` である（指摘 1 の根）。
- **`log` を使う依存。** `Cargo.lock` で `log` に依存するのは `bevy_app`・`bevy_ecs`・`iana-time-zone`・`wgpu-types`・`tracing-log` の 5 つである。research.md は後ろの 2 つが areka の木に出ないことを書いているが、design.md の設計判断 a は 2 つだけを挙げている。根拠の 1 行（`cargo tree -p areka -i log`）を添えるとよい。
- **`main.rs` の行数。** 今は 957 行で、5 行の初期化が 1 行になり `use` が 1 行減り `mod` が 1 行増えるので、設計の「増えない」は正しい。要件 8.1 の「出口の組み替えと `mod` の宣言だけ」にも合う（出口を据える場所は `fn main()` の同じ位置のまま、中身が `log_history::init()` へ移るだけである）。
- **`dyn Error` の欄。** 標準出力の層は `record_error` で原因の連なり（`<欄>.sources=[…]`）を足すが、設計の履歴の層は「それ以外は `{:?}` の形」としか書いていない。実プロセスの試験の判定 2（本文が標準出力と 1 字も違わない）は `ghost_resolved` の行だけを見るので赤にはならないが、「本文の作り方は標準出力と同じ」と文書に書くなら、この欄だけ見え方が違いうることを 1 行足す。

## 確かめたこと（設計の主張と実物）

| 設計の主張 | 実物 | 結果 |
|---|---|---|
| ログの出口は `fn main()` の先頭の `tracing_subscriber::fmt().with_env_filter(…).init()` の 1 か所・`human_panic::setup_panic!()` の直後・`thread_roles::install()` の前 | `crates/areka/src/main.rs` | 一致 |
| `main.rs` は 957 行 | 同上 | 一致 |
| `get_log::handle(world, args, reply)` はダミーで、`dispatch` は `ghost_name` を解決せずに渡す | `crates/areka/src/mcp/get_log.rs`・`mcp/mod.rs` | 一致 |
| `resolve` は空の `ghost_name` を省略として扱い `NOT_ACTIVE` を返す。空でない値の失敗は 0 体でも `CANNOT_FIND` | `crates/areka/src/mcp/resolve.rs` の `resolve` | 一致 |
| `active`・`resolve`・`listed_value`・`CANNOT_FIND`・`Omitted` は `pub(crate)` | 同上 | 一致 |
| `get_log_and_seven_omitted_do_not_answer_with_a_resolve_failure` は 4 引数とも `None`・0 体で呼び、名前の解決の `NG:` でないことだけを見る | `crates/areka/src/mcp/mcp_tests.rs` | 一致（緑のまま） |
| 見張りの走査語は 3 語で、`init()` は当たらない。例外表は 4 件 | `crates/log-capture-kit/tests/with_default_guard_test.rs` | 一致 |
| `CapturedEvent` は `level`・`target`・訪問順の `fields`（`debug` と `str_raw`）を出す | `crates/log-capture-kit/src/event.rs` | 一致 |
| `target: "ghost-boot"`／`"ghost-shutdown"` の字面がある | `crates/areka-ghost/src/runtime.rs` | 一致 |
| `RULES` が名指しするモジュールのファイルが実在する | `crates/areka/src/` の 7 つ・`crates/areka-update/src/` の `lib.rs`・`winhttp.rs`・`fetch.rs` | 一致 |
| `event = "ghost_resolved"` の行は `areka::boot_config`、「本物のゴースト窓を開きました」は `areka::ghost_session` | `boot_config.rs`・`ghost_session.rs` | 一致 |
| `AREKA_MCP_PORT`・`POST /api/mcp/v1` | `crates/areka-mcp/src/port.rs`・`dispatch.rs` | 一致 |
| `Win32_System_SystemInformation` は根の `Cargo.toml` に宣言済み | 根の `Cargo.toml` | 一致 |
| 実プロセスの試験の前提（i686 の helper・モニタ 0 台の受理）は既存の smoke と同じで、全体テストで走る | `crates/areka/tests/smoke_boot_loop_exit.rs`・`tools/test-all.ps1` | 一致 |
| 出口を組み替えても、関心のない呼び出し口は「関心なし」と覚えられる・`log` の拒まれた行は印を残さない | 使い捨ての実験 | 一致 |
| 「`perf_thread_report` は環境変数を読むので影響しない」（research.md §11） | `crates/areka/src/perf_thread_report.rs` の `is_enabled` | **不一致**（指摘 1） |
