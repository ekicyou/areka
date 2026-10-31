# ギャップ分析: actor-thread-log-capture

> 2026-10-31・本ブランチ（main `da020c10` の上・`c0165576`）での読み。要件は `requirements.md`（確定済み）、背景は `brief.md`。
> 依存の読みは `Cargo.lock` の版（`tracing` 0.1.44・`tracing-core` 0.1.36）のソース（`C:\rust\cargo\registry\src\index.crates.io-…\tracing-core-0.1.36\src\dispatcher.rs`）を直に読んだ。

## 1. 分析の要約

- **足りないのは「起こす時点の受け手を子へ渡す」数行だけ**。`spawn_actor`（`crates/areka-actor/src/spawn.rs` 86〜117 行）は素のスレッドを起こし、子の中で `actor` の区間に入って本体を呼ぶだけで、受け手を渡す手は無い。`tracing::dispatcher::get_default(|d| d.clone())` で取り、子の中で `set_default` すれば要件 1〜2 の働きはそろう。窓を開いた側の受け手は `with_default` が作った `Dispatch`（中身は `Arc`）なので、子へ写しを渡すと窓が閉じた後も子の中で生き続ける。
- **避けられない追加のファイルが 3 つある**: ⑴ 番人 `crates/log-capture-kit/tests/with_default_guard_test.rs` の例外表と件数（`spawn.rs` が走査語 `set_default(`／`with_default(` に当たる・4→5 件）⑵ `crates/areka-actor/Cargo.toml` の `[dev-dependencies]` に `log-capture-kit`（要件 6.3 の「共有の窓口を通す」に要る。brief の「新しい依存は要らない」は誤り）⑶ それに伴う `Cargo.lock` の 1 行（`areka-actor` の依存の並びに `"log-capture-kit"`）。
- **本番で変わりうる所が 1 つある**: 本番の子へ「プロセスの全体の受け手」を写して差すと、`tracing-core` の「スレッドごとの差し替えが 1 つも無ければ全体の受け手へ直行する速い道」（`SCOPED_COUNT == 0`）がプロセス全体で使われなくなる。出る行・水準・出先・文言は同じだが、スレッドの後始末の最中に出す記録が落ちる経路が新しく生まれる（下の 4.3）。差すのを「窓（スレッドごとの差し替え）があるときだけ」に絞れるかが設計の要の判断になる。
- **赤に変わる既存のテストは、静的な読みで確実に 1 本・疑いが 4〜6 本**。確実なのは `crates/areka/src/update/worker_path_tests.rs` 201 行（`update_reload_requested` を 1 件と数えるが、窓の中で起きる背景のスレッド `update` の `procedure.rs` も同じ名前を出す＝2 件になる）。262 本ある捕捉を使うファイルの大半は、アクターを窓の前に起こすか、素のスレッドで回すので変わらない。
- **規模は S（4〜6 タスク）・危うさは中**。仕組みは小さいが、本番の速い道の扱い・窓を閉じた後も生きるアクターの記録の溜まり方・赤の手当てが `areka` クレートのテストへ及ぶこと（境界の外）の 3 点が設計の段で決めること。

## 2. 今の形（使える資産と決まり）

| 所 | 今の形 | 本 spec との関わり |
|---|---|---|
| `crates/areka-actor/src/spawn.rs`（360 行） | `thread::Builder::new().name(..).spawn(move \|\| { info_span!("actor") に入る → フック（`THREAD_START_HOOK.get()`）→ body(rx) })`。起こせないときは `error!` と panic。`ActorHandle::join` は panic を `ActorError::Panicked` と `warn!` へ写す | 引き継ぎを差すのは `spawn` に渡す閉包の先頭（区間とフックより前＝要件 1.3）。起こす側での `get_default` は `Builder::spawn` の前（要件 2.2） |
| `crates/areka-actor/src/spawn_hook_tests.rs`（60 行） | フックの導入（プロセスに 1 度・最初が勝つ）と「生成された側で走る」を 1 本に畳んだ試験。`probe_hook` は記録を出さない | 対照の試験の置き場の候補（要件 6.1）。フックの枠はこの試験が取るので、ほかの新しい試験はフックの有無に頼れない |
| `crates/areka-actor/Cargo.toml` | `[dependencies]` は `tracing`・`thiserror`・`async-channel`・`wintf-winmsg-executor`、`[dev-dependencies]` は `windows` だけ | **`log-capture-kit` を引いていない**＝足す必要がある |
| `crates/log-capture-kit/src/capture.rs`（107 行） | `CaptureSubscriber`（`Arc<Mutex<Vec<CapturedEvent>>>`・`enabled` は常に真・`new_span` は固定の id 1）を `tracing::subscriber::with_default` で呼んだスレッドへ差す → 窓の中で `rebuild_interest_cache` → 番兵 → `f()` → 窓を抜けてから `mem::take` で取り出し → 番兵を除く | 子が受け手の写しを持つと、取り出しの後に子が出した記録も同じ `Vec` へ積まれ続ける（誰も読まない）。止めたければ `capture.rs` に「閉じた」の印が要る（下の案 C） |
| `crates/log-capture-kit/src/probe.rs` | 常駐の仕掛け（受け手 2 個を常駐させ、発行点の判定を「毎回訊く」に固定）。これがあるので発行点の判定は各スレッドの今の受け手へ毎回委ねられる | 子のスレッドの記録も「子の今の受け手（＝窓）」で判定される＝取りこぼしの新しい経路は無い。測定専用の切り替え `AREKA_LOG_CAPTURE_PROBES=off` のときも、`CaptureSubscriber` は `enabled` 真なので「いつも」になり、届く |
| `crates/log-capture-kit/src/lib.rs`・`capture.rs` の説明 | 「呼出スレッド局所で、窓の内側でも他スレッドが発火したイベントは入らない」（lib.rs 223 行・capture.rs 96 行「要件 3.6」） | **本 spec の後は字面が半分偽になる**（窓の中で起こしたアクターのスレッドは入る）。直すなら `lib.rs` も触る＝ウェーブの約束（`capture.rs` だけ）の外 |
| `crates/log-capture-kit/src/global.rs` | 全スレッドの捕捉。例外表 `ALLOWED_GLOBAL_CAPTURE` に 3 件（areka-ghost の `spine_e2e_test_global_log_probe.rs`・seriko の `loop_integration.rs`・kanade の `failure_test.rs`） | 3 件とも同じテストの実行ファイルの中で `capture(` を使っていない（確かめた）＝窓に吸われて全体の溜まりから消える記録は無い |
| `crates/log-capture-kit/tests/with_default_guard_test.rs` | 走査語 `with_default(`・`set_default(`・`set_global_default(` が `crates/log-capture-kit/src/` と例外表 `ALLOWED_DIRECT_CALLS`（4 件・`ALLOWED_DIRECT_CALLS_COUNT = 4`）の外に 1 件も無いこと。コメントは除いて走査。**本番のソースも走査の対象**（`crates/` の下の全 `.rs`） | `spawn.rs` が `tracing::dispatcher::set_default(&d)` か `with_default(&d, ..)` を書くと当たる＝例外表と件数の 2 か所へ理由つきで足す（要件 5.3 の想定どおり）。`KNOWN_DEV_DEPENDENTS`（10 件）は「載っているものが本当に引いている」の陽性の確かめなので、`areka-actor` を足さなくても緑 |
| `crates/areka/src/thread_roles.rs` | 本番の `main` が `log_history::init()`（全体の受け手を据える）の後に `thread_roles::install()`。フックは名簿への登録だけで、失敗のときだけ `error!`／語彙に無い役割名のとき `warn!`（`actor:<名>` は語彙に入る＝ふだんは出ない） | 変更 0（要件 4.4）。引き継ぎはフックの枠を使わない（要件 4.2） |

**決まり**: 1 ファイル 1,000 行（`spawn.rs` 360・`spawn_hook_tests.rs` 60・`capture.rs` 107 で余裕あり）・本番の依存は足さない・`log-capture-kit` は `[dev-dependencies]` だけ・`set_global_default` は窓口だけ・本番の記録の出先と文言は変えない（steering `logging.md`・`structure.md` の log-capture-kit 節）。

## 3. 要件と資産の対応

| 要件 | 要るもの | 今あるもの | 判定 |
|---|---|---|---|
| 1.1 窓の中で起こしたアクターの記録が入る | 起こす時点の受け手を子で既定にする | 無い | **Missing**（`spawn.rs` に数行） |
| 1.2 何段先の子も入る | 子のスレッドの中の `get_default` が窓の受け手を返すこと | 1.1 を入れれば自動で成り立つ（子の中の既定が窓の受け手なので、その子が起こすアクターも同じものを取る）。ゴーストの `ghost-dispatcher` が台詞ごとに `spawn_talk` を起こす形（`areka-ghost/src/dispatcher.rs` 357 行）がまさにこの段 | 1.1 で足りる |
| 1.3 フックの中の記録も入る | 既定を差すのを区間とフックより前に置く | 閉包の中の順を決めるだけ | 1.1 と同じ所。**試験で固めるかは未定**（下の 9 の ⑥） |
| 1.4 同じ形で返る | `CapturedEvent::from_event` は区間の情報を持たない＝どのスレッドでも同じ形 | ある | 変更 0 |
| 1.5 待ってから閉じれば取りこぼさない | 子の `event()` は同じ `Mutex` へ同期に積む・取り出しは窓を抜けた後 | ある | 変更 0（ただし窓を抜けた直後〜取り出しの前に子が出した記録はその窓の結果に入る＝2.4 には反しない） |
| 2.1 窓の外で起こしたアクターは入らない | 起こす側に窓が無ければ写すのは全体の受け手か「受け手なし」 | `get_default` の意味どおり | 1.1 の書き方次第（「受け手なし」を差すと、後から全体の受け手が据えられても子は拾わない＝下の 4.2） |
| 2.2 起こした時点で決める | `get_default` を `Builder::spawn` の前で呼ぶ | — | 1.1 の書き方 |
| 2.3 並ぶテストで混ざらない | 受け手はスレッドごと・窓ごとに別の `CaptureSubscriber` | ある | 変更 0 |
| 2.4 閉じた後の記録はほかの窓に入らない | 子は窓 A の受け手を持ち続ける＝後から開いた窓 B へは行かない | 1.1 の性質 | 成り立つ。**ただし窓 A の受け手へ積まれ続ける**（**Constraint**: 記録の溜まりと手間・下の 4.4） |
| 2.5 長生きのアクターの振る舞いは同じ | 受け手の写しを持つだけ | — | 成り立つ |
| 3.1 本番の記録は同じ | 本番は起こす側の既定＝全体の受け手 | — | 出る行は同じ。**速い道の扱いは Unknown**（4.3） |
| 3.2 テストだけの切り替えを持たない | 同じ道 | — | 1.1 を常に通せば成り立つ |
| 3.3 名前・区間・join・起動失敗は同じ | 既存の 5 本の単体試験（`spawn.rs` 244〜359 行） | ある | 変更 0（起動失敗の `error!` は起こす側で出る＝今と同じ） |
| 3.4 待ち・常駐・新しいスレッドを足さない | — | — | 数行の写しと差し替えだけ |
| 3.5 本番の形の行き先を試験で固める | 窓を持たないスレッドから起こしたアクターの行き先が起こす側と同じ | **Unknown**: このテストの実行ファイルに全体の受け手を据えられない（要件 5.1）ので「全体の受け手へ行く」ことは直には見られない。見られるのは「受け手なし同士で同じ」か「窓に入らない」まで（下の 9 の ④） |
| 4.1〜4.4 フックと両立 | フックの枠を使わない | — | 変更 0 |
| 5.1〜5.2 道具の決まり | 全体の捕捉を使わない・窓の決まりを変えない | — | 案 C を取ると `capture.rs` の振る舞いが「閉じた後は積まない」に変わる＝5.2「今の決まりを変えない」の字面との突き合わせが要る |
| 5.3 番人 | 例外表と件数へ理由つきで足す | 番人あり | **Missing**（`with_default_guard_test.rs` の 2 か所） |
| 5.4 1,000 行 | — | 余裕あり | 変更 0 |
| 5.5 赤の手当て | 下の 6 の表 | — | **Constraint**: 直す所が `crates/areka/src/update/` など境界の外のファイルになる |
| 5.6 全体テスト 1 回（静かな机は要らない） | — | — | 最後に 1 回 |
| 6.1〜6.5 対照の試験 | `areka-actor` の中の試験と `log-capture-kit` の窓口 | `[dev-dependencies]` に無い | **Missing**（`Cargo.toml`・`Cargo.lock`） |

## 4. 記録の受け手の仕組みで分かったこと（tracing-core 0.1.36 の実コード）

### 4.1 取り方と差し方

- `dispatcher::get_default(f)`（`dispatcher.rs` 379〜397 行）: スレッドごとの差し替えがプロセスに 1 つも無いとき（`SCOPED_COUNT == 0`）は全体の受け手（未設定なら「受け手なし」`Dispatch::none()`）を返す。1 つでもあれば、そのスレッドの差し替え（無ければ全体の受け手）を返す。**入れ子で呼ぶと「受け手なし」を返す**（受け手の処理の中から `spawn_actor` を呼ぶ形は無いので実害は無い見込み）。
- `dispatcher::set_default(&d)`（276 行→`State::set_default` 841〜852 行）: スレッドの既定を差し替え、`SCOPED_COUNT` を 1 増やす。戻りの番（`DefaultGuard`）を落とすと元へ戻し 1 減らす。`with_default(&d, f)` も同じ（`f` の間だけ）。
- `Dispatch` の写し（`clone`）は中の `Arc` を共有する。窓が `with_default` で作った `Dispatch` を子が写して持てば、窓が閉じて窓の側の `Dispatch` が落ちても、子が持つ限り `CaptureSubscriber` は生きる。
- `Dispatch::is::<NoSubscriber>()`（717 行）で「受け手なし」を見分けられる（`tracing::subscriber::NoSubscriber` として公開されている）。
- **全体の受け手かスレッドごとの差し替えかを見分ける公開の手は無い**。区別が表に出るのは `Debug` の字面（`Dispatch::Global(..)`／`Dispatch::Scoped(..)`・736〜748 行）だけで、これは安定の約束ではない。

### 4.2 「受け手なし」を写すと本番の記録が落ちうる

起こす側がまだ全体の受け手を据えていない（据える前に起きた）アクターへ「受け手なし」を差すと、その子は後から全体の受け手が据えられても既定が「受け手なし」のまま＝記録が永久に消える。areka の本番は `main` の先頭で `log_history::init()` が全体の受け手を据えるので当たらないが、ほかの実行体（`real-run-kit` の `real-run-fake-areka`・`pilot` の `shiori-host-32`・テストの実行ファイル）では起きうる。**「受け手なし」のときは差さない**（子は今と同じく全体の受け手を遅れて読む）の 1 条件が要る。

### 4.3 本番で全体の受け手を写して差すと変わること

本番のアクターは長生きなので、子が全体の受け手の写しを「差し替え」として持ち続けると `SCOPED_COUNT` が本番の間ずっと 1 以上になる。すると:

- **すべてのスレッド**（素のスレッド・UI のスレッドを含む）の記録 1 件ごとに、全体の受け手へ直行する速い道ではなく、スレッドごとの状態（スレッドローカルの借用）を経る道を通る。出先は同じだが手間は少し増える（数 ns 級の見込み・**測っていない**）。
- スレッドの後始末の最中（スレッドローカルが壊された後）に出した記録は、速い道なら全体の受け手へ届くが、遅い道では「受け手なし」になって**落ちる**（`get_default` の `unwrap_or_else(|_| f(&NONE))`）。受け手の処理の中から出した記録（入れ子）も同様。本番に該当する発行点があるかは**未調査**。

要件 3.1 は「出る行・水準・出先・文言」の変更 0 を求めるので、この 2 つ目は満たさない恐れがある。避ける形は 2 つ:

- **案 ⓐ「受け手なし」以外は常に差す**（短い・公開の手だけ）。速い道の喪失を受け入れ、後始末の最中の記録の有無を調べて「該当なし」を示す。
- **案 ⓑ スレッドごとの差し替えのときだけ差す**（本番は差さない＝本番の道は 1 ビットも変わらない）。ただし見分けに公開の手が無い（4.1）。`Debug` の字面に頼るのは版が上がると黙って壊れる。別の見分け方（例: 本番の全体の受け手を areka-actor へ知らせない限り無理）があるかは**要調査**。要件 3.2（テストだけの切り替えを持たない）とは「同じ道を通り、受け手の種類で分かれるだけ」なので両立する。

### 4.4 窓が閉じた後も生きるアクター

窓の中でゴーストを起こすテスト（`SwitchRig::boot` → `boot_ghost` → kanade・SHIORI・`ghost-dispatcher`・ticker・seriko・sylphya など）では、アクターは窓が閉じた後もテストの終わり（`rig.shutdown()`）まで生き、その間の記録（TRACE を含む全水準）を誰も読まない `Vec` へ積み続ける。`CaptureSubscriber::enabled` が常に真なので、ふだんは捨てられる TRACE の欄の組み立ても毎回走る。ticker（50 ms 周期）や台詞の刻みが TRACE を出すなら、テストの時間と記憶が増える（**量は測っていない**）。止める手は `capture.rs` に「閉じた」の印（`AtomicBool`）を足し、取り出しの時点で立てて `enabled`／`event` で捨てる形（常駐の仕掛けで判定は毎回訊かれるので効く。`AREKA_LOG_CAPTURE_PROBES=off` で判定が「いつも」に焼き付く場合に備えて `event` の側でも捨てる）。

## 5. 実装の案

### 案 A: `spawn.rs` だけを広げる（最小）

- 起こす側: `let dispatch = tracing::dispatcher::get_default(|d| d.clone());`
- 子の閉包の先頭（区間とフックより前）: 「受け手なし」でなければ `let _default = tracing::dispatcher::set_default(&dispatch);`（本体の終わりまで持つ）。
- 番人の例外表へ `crates/areka-actor/src/spawn.rs` を理由つきで 1 件（4→5）。
- 対照の試験 5 つ（要件 6.2）を `spawn_hook_tests.rs` へ足すか、兄弟の新しい 1 本（例 `spawn_log_capture_tests.rs`）を `#[cfg(test)] #[path]` でつなぐ。`Cargo.toml` の `[dev-dependencies]` に `log-capture-kit = { path = "../log-capture-kit" }`。
- 本番の扱いは 4.3 の ⓐ か ⓑ。
- ✅ 触るのは `areka-actor` の中と番人と `Cargo.lock` だけ・数行。❌ 窓を閉じた後の記録が溜まり続ける（4.4）・`log-capture-kit` の説明（「他スレッドは入らない」）が古びる。

### 案 B: 走り始めのフックを状態を持てる形へ広げて使う（brief の第 2 案）

- `ThreadStartHook` を `fn(&str)` から「起こす側で何かを取り、子で使う」2 段の形へ広げ、テストの捕捉がそれを導入する。
- ❌ フックの枠はプロセスに 1 つで、areka のテストの実行ファイルでは `thread_roles_tests.rs` が取る＝並べる仕組みが要る（要件 4.1 のフックの形の変更 0 に反する）。❌ テストのときだけ導入する＝要件 3.2（同じ道）に反する。❌ `log-capture-kit` はワークスペースのどの crate にも依存できない（leaf の決まり）ので、導入する者が居ない。**要件と両立しない**。比べのために挙げる。

### 案 C: 案 A ＋ `capture.rs` に「閉じた」の印（混合）

- 案 A に加え、`CaptureSubscriber` に閉じたの印を持たせ、`run_with_subscriber` が取り出しの直前（または `with_default` を抜けた直後）に立てる。閉じた後の `enabled` は偽・`event` は捨てる。
- ✅ 長生きのアクターの記録が溜まらない・TRACE の組み立ても止まる（4.4）。✅ 「窓が閉じた後の記録」の扱いが受け手の寿命でなく窓の印で決まる＝説明しやすい。❌ `capture.rs` に触る（ウェーブの約束の「要るときだけ」に当たるかの判断が要る）・要件 5.2「今の決まりを変えず、窓の中でアクターを起こさないテストの結果を変えない」との突き合わせ（窓の中でアクターを起こさないテストには見えない変化なので満たす見込み）。❌ 印を立てる時点（窓を抜けた直後か、取り出しの直前か）で、閉じる間際の子の記録がその窓に入るかが変わる（どちらでもほかの窓へは入らない）。

## 6. 既存のテストが赤に変わる見込み

`log_capture_kit` を使うファイルは 262 本（`log-capture-kit` 自身を除く）。そのうち、窓の中でアクターを（直に・または起動や依頼の窓口を通して）起こし、件数か「無いこと」を確かめているものを静的に洗った。**大半は、アクターを窓の前に起こすか（例 `install/worker_tests.rs` 327 行・`update/worker_tests.rs` 510 行・`drive_strict_tests.rs` 297 行・`actor_trigger_fixture_tests.rs` 201 行・`boot_resolve_tests.rs` 746 行）、素のスレッドで回す（`strict_judges_tests.rs` 532 行・ECS の多スレッド実行器）ので変わらない**。各アクターのクレート（kanade・sakura・seriko・sylphya・ghost・mcp）の中の試験に、窓の中でアクターを起こすものは見つからなかった。

窓の中でアクターが起きる主な経路は 3 つ: ⑴ `SwitchRig::boot`／切替／上書きの起き直し（`boot_ghost` がゴーストのアクター一式を起こす）⑵ インストールと更新の窓口が**最初の依頼で**背景のスレッド（`install`／`update`）を起こす（窓口は ECS の系＝テストのスレッドで走る。`install/desk.rs` 536 行・`update/desk.rs` 407 行）⑶ `ghost-dispatcher` が台詞ごとに起こす talk のスレッド（⑴ の子）。

| 確度 | ファイル:行 | 窓の中で起きるアクター | 何が変わるか |
|---|---|---|---|
| **確実** | `crates/areka/src/update/worker_path_tests.rs`:201 | `update`（窓の中の `request_ghost_update` → 最初の `hand_over`） | `update_reload_requested` は背景のスレッドの `update/procedure.rs` 187 行（INFO）と UI の `update/desk.rs` 206 行（INFO）の両方で出る。今は後者だけが映って 1 件。引き継ぎの後は 2 件＝`assert_eq!(.., 1)` が赤 |
| 中 | `crates/areka/src/emo2_boot/ghost_switch_talk_clock_tests.rs`:137・181・203 | A を起こし B へ切り替える一周のゴーストのアクター一式（kanade・SHIORI・dispatcher・talk・seriko など） | 窓の中の ERROR を 0 件と確かめる。kanade の `change_deadline_exceeded`（`areka-kanade/src/schedule/change.rs` 293 行）を含め、アクターのスレッドの ERROR が初めて数に入る。正しく動いていれば 0 のままのはずだが、降ろす途中などで出る ERROR があれば赤（**テストが強くなる側の変化**） |
| 中 | `crates/areka/src/install/desk_overwrite_tests.rs`:126・506・551・617・657 | `install`（窓の中の最初の依頼）と、起き直したゴーストのアクター一式 | 数えているのは UI のスレッドで出る名前（`install_overwrite_done`・`ghost_switch_*`）と `areka_nar` の ERROR（上書きの展開は UI のスレッドで同期＝`install/overwrite.rs` の説明）。背景のスレッドが同じ名前や `areka_nar` の ERROR を出さなければ緑のまま。`clues` の手がかりの文が長くなるだけの見込み。**このファイルは同じウェーブの `install-before-boot` が持つ**＝赤なら直す前にそのセッションと合わせる（要件 5.5） |
| 中 | `crates/areka/src/update/worker_path_tests.rs`:224・285・322 | 起き直したゴースト・`update`（285 の窓は最初の依頼） | 名前を 0 件と確かめる所。背景のスレッドが同じ名前を出す経路（`update_tail_sent` も `procedure.rs` 208 行と `desk.rs` 251 行の両方）に当たれば赤 |
| 低 | `crates/areka/src/update/desk_reload_tests.rs`:315（「判定の記録と列を捨てた記録のほかに `warn!` は無い」）・`install/worker_tests.rs`:488・`ghost_session_switch_tests.rs`:280・`emo2_boot/frame_ghost_quit_switch_tests.rs`:141・`emo2_boot/ghost_switch_reload_tests.rs`（`reload_done` などの件数）・`install/names_tests.rs`:143・165・`boot_shell_tests.rs`:146・188・`mcp/dump_surface_tests.rs`:437 | 起き直したゴーストのアクター一式 | 数えている名前はどれも `areka` の UI のスレッドだけで出る（発行点を確かめた: `reload_done`／`reload_failed`＝`emo2_boot/reload.rs`・`ghost_switch_*`＝`emo2_boot/ghost_switch.rs`・`install_names_*`＝`install/names.rs`・`boot_shell_missing`＝`boot_resolve.rs`）。水準だけで数える所（WARN／ERROR の 0 件）は、アクターのスレッドの WARN／ERROR が入ると赤 |

**見積り**: 確実に赤 1 本（1 か所）・疑い 4〜6 本。ほかに、窓の中でゴーストを起こすテストは窓の後も記録を溜め続ける（4.4）ので、テストの時間と記憶が増えうる（量は未測定）。並びを確かめるテスト（記録の順）で、アクターの記録が間に挟まって赤になる形は、洗った範囲では見つからなかった。

**手当ての置き場**: 確実な 1 本は `crates/areka/src/update/` の中で、同じウェーブのほかの spec は持たない。ただしウェーブの約束（本 spec が触るのは `areka-actor` と要るときの `capture.rs`）の外になる。要件 5.5 の「主張の対象（宛先・本文）を絞る」に従えば、`named(&events, "update_reload_requested")` を宛先 `areka::update::desk` に絞る形の 1 行の直しになる見込み。

## 7. 避けられない追加のファイル（ウェーブの約束の外）

| ファイル | 何を | なぜ避けられないか |
|---|---|---|
| `crates/log-capture-kit/tests/with_default_guard_test.rs` | `ALLOWED_DIRECT_CALLS` に `crates/areka-actor/src/spawn.rs` を理由つきで 1 件・`ALLOWED_DIRECT_CALLS_COUNT` を 4→5 | 子のスレッドの既定を差す公開の手は `set_default`／`with_default` だけで、どちらも走査語に当たる（別名で読み込んで逃れるのは番人の趣旨に反する）。要件 5.3 が想定している |
| `crates/areka-actor/Cargo.toml` | `[dev-dependencies]` に `log-capture-kit = { path = "../log-capture-kit" }` | 要件 6.3（共有の窓口を通して捕捉する）。`log-capture-kit` の通常の依存は `tracing` だけ＝循環は生まれない |
| `Cargo.lock` | `areka-actor` の依存の並びに `"log-capture-kit"` の 1 行 | 上の dev 依存で自動で変わる（dev 依存も `Cargo.lock` に載る）。外の依存ではないので `THIRD-PARTY-NOTICES.md` と `tech.md` は変わらない見込み。ウェーブの「1 本だけの席」⑴（依存を足す spec）に当たるかの確認が要る（中の crate の path 依存で、外の crate は増えない） |
| （赤が出たら）`crates/areka/src/update/worker_path_tests.rs` ほか 6 の表 | 主張の対象を宛先で絞る | 要件 5.5 |
| （案 C を取るなら）`crates/log-capture-kit/src/capture.rs` | 閉じたの印 | 4.4 |
| （説明を直すなら）`crates/log-capture-kit/src/lib.rs` の「全スレッド捕捉の使いどころ」の節・`capture.rs` 92〜98 行の説明・steering `structure.md` の log-capture-kit 節の「Consumers」（15 crate→16）と areka-actor 節 | 「呼出スレッド局所・他スレッドは入らない」が「窓の中で起こしたアクターは入る」に変わる | 説明が古びると次の人が全体の捕捉へ逃げる。steering は `/kiro-complete` の文書の同期で直せる |

## 8. 規模と危うさ

- **規模: S（4〜6 タスク）**。本体は `spawn.rs` の数行と番人の 2 か所と dev 依存。対照の試験 5 つ（＋要るなら 1.3 の 1 つ）で 1〜2 タスク。赤の手当て 1〜2 か所と全体テスト 1 回。案 C なら `capture.rs` の印と自己試験で +1。
- **危うさ: 中**。仕組みは `tracing` の決まった使い方で新しい並行の保証は要らないが、⑴ 本番の速い道の扱い（4.3）を決めないと要件 3.1 を満たすかが言えない ⑵ 赤の手当てが境界の外（`areka` クレートのテスト）へ及び、1 本は同じウェーブの `install-before-boot` の持ち物 ⑶ 長生きのアクターの記録の溜まり（4.4）がテストの時間を延ばしうる、の 3 つ。

## 9. 設計への申し送り

### 決めること（要件の討議・設計の段）

1. **本番で何を子へ差すか**: ⓐ「受け手なし」以外は常に差す（全体の受け手を写して差す＝本番の速い道を失う・後始末の最中の記録の扱いを調べて示す）か、ⓑ スレッドごとの差し替え（＝窓）のときだけ差す（本番の道は今と同じ・見分けに公開の手が無い）か。どちらでも「受け手なし」のときは差さない（4.2）。
2. **窓が閉じた後の子の記録をどうするか**: 案 A（窓 A の受け手へ積まれ続ける・誰も読まない）か、案 C（`capture.rs` に閉じたの印を足して捨てる）か。案 C の印を立てる時点（`with_default` を抜けた直後／取り出しの直前）も決める。
3. **対照の試験の置き場**: `spawn_hook_tests.rs` へ足す（フックの試験と同じファイル・60 行）か、兄弟の新しい 1 本か。新しい 1 本なら名前（例 `spawn_log_capture_tests.rs`）。
4. **要件 3.5（本番の形の行き先）を試験でどこまで固めるか**: このテストの実行ファイルには全体の受け手を据えられない（要件 5.1）。見られるのは「窓を持たないスレッドから起こした子の既定が、起こす側と同じく『受け手なし』である」（`get_default(|d| d.is::<NoSubscriber>())` を両方で比べる）と「その記録は並行の窓に入らない」まで。全体の受け手へ行くことを直に見るには、別の実行ファイルで全体の捕捉を使う形になり要件 5.1 と衝突する。
5. **赤の手当ての範囲と合わせ方**: `update/worker_path_tests.rs` 201 行（確実）は境界の外。宛先で絞る 1 行の直しを本 spec で行うか（要件 5.5 のとおり）。`install/desk_overwrite_tests.rs` が赤になったら `install-before-boot` のセッションへ先に相談する。
6. **要件 1.3（フックの中の記録）を試験で固めるか**: フックの枠はテストの実行ファイルに 1 つで、`spawn_hook_tests.rs` の 1 本が取る。固めるならその 1 本の中で `probe_hook` に記録を出させ、窓の中で起こしたアクターのフックの記録が入ることを足す形になる（要件 6.2 の 5 つには入っていない）。
7. **`log-capture-kit` の説明（`lib.rs`・`capture.rs`）と steering の追随**: 「他スレッドは入らない」の字面を本 spec の中で直すか（`lib.rs` はウェーブの約束の外・ほかの C14 の spec は持たない）、`/kiro-complete` の文書の同期へ回すか。
8. **`Cargo.lock` の 1 行**がウェーブの「1 本だけの席」⑴ に当たるか（中の crate の path 依存だけ・外の crate は増えない）。

### 調べること（設計の段で）

- **Research Needed**: 本番の areka で、スレッドの後始末の最中（スレッドローカルが壊された後）や受け手の処理の中から出る記録があるか（4.3・案 ⓐ を取る場合の根拠）。`log_history` の層（`HistoryLayer`・`CaptureLayer`）が記録の処理の中で記録を出すか。
- **Research Needed**: 案 ⓑ の見分け方。`Debug` の字面以外に、全体の受け手とスレッドごとの差し替えを見分ける安定の手があるか（`tracing-core` の公開面にはない＝0.1.36 で確認）。
- **Research Needed**: 窓の中でゴーストを起こすテストで、窓の後に溜まる記録の量（ticker・台詞の刻み・seriko の TRACE）。案 C を取るかの判断の材料。
- **Research Needed**: 6 の表の「中」の 3 つ（talk の時計の檻・上書きの檻・更新の道の檻）が実際に赤になるか。静的には決めきれない＝実装の後の全体テスト 1 回（要件 5.6）で洗う。

## 10. 要件討議の決着（2026-10-31・一括承認）

- **議題 1（本番で子に受け手を据えるか）**: 案 ⓑ。起こした側のスレッドがスレッドごとの受け手（捕捉の窓）を持つときだけ引き継ぐ。本番の道は今のまま・「受け手なし」も渡さない（要件 3.6 を足し、3.2・3.5 を合わせた）。見分け方は設計の段で決める（9 の「調べること」の 2 つ目）。安定の手が見つからなければ、設計の段で開発者へ戻す。
- **議題 2（ウェーブの表の外のファイル）**: 7 の表のファイルはすべて本 spec の中で触ってよい（Boundary の Adjacent に列挙）。`Cargo.lock` の 1 行は中の path 依存だけで、依存を足す spec の 1 本だけの席は取らない。`install/desk_overwrite_tests.rs` が赤なら、直す前に `install-before-boot` のセッションと合わせる。

## 11. 設計の段の調べと決め（2026-10-31・`/kiro-spec-design -y`）

### 要約

- **Feature**: `actor-thread-log-capture`
- **Discovery Scope**: Extension（既存の `spawn_actor` と捕捉の窓を広げる・light の調べ）。調べの部下は立てず、`tracing-core` 0.1.36 と `tracing-subscriber` 0.3.23 のソースを直に読んだ。
- **使った手引き**: `kiro-spec-design` の rules（`design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`）と、テンプレート `design.md`・`research.md`。
- **Key Findings**:
  - 全体の受け手かスレッドごとの差し替えかを見分ける公開の手は無いが、**受け手そのものに尋ねる**公開の口 `Subscriber::downcast_raw`（`Dispatch::is::<T>()` が通る）がある。窓の受け手だけが答える目印の型を決めれば、議題 1 の ⓑ は安定の API だけで書ける。
  - 目印の型を std の型だけで書けば（`[(); 0x6172_656B]`）、`areka-actor` と `log-capture-kit` が互いに依存せずに同じ `TypeId` を名指せる。
  - 全スレッドの捕捉（`global.rs`）も同じ `CaptureSubscriber` を据えるので、目印を `CaptureSubscriber` に答えさせると、全スレッドの捕捉を使う 3 本の実行ファイルで子に差し替えが据わり要件 3.6 の字面に反する。＝目印に答えるのは `capture` だけが差す窓専用の受け手にする。

### 調べの記録

#### 見分け方（9 の「調べること」の 2 つ目）

- **Context**: 議題 1 で ⓑ（窓のときだけ引き継ぐ）に決まり、見分け方が設計の段に残った。
- **Sources Consulted**: `tracing-core-0.1.36/src/dispatcher.rs`（`get_default` 379〜398 行・`Dispatch::is`／`downcast_ref` 717〜726 行）・`src/subscriber.rs`（既定の `register_callsite` 175〜181 行・`downcast_raw` 467〜498 行・`dyn Subscriber::is`／`downcast_ref` 500〜517 行）・`tracing-subscriber-0.3.23/src/filter/layer_filters/mod.rs`（`Filtered::downcast_raw` 873〜883 行・`MagicPlfDowncastMarker` 1273〜1281 行）・`src/layer/layered.rs`（`Layered::downcast_raw` 214・344 行）。
- **Findings**:
  - `Dispatch::is::<T>()` は `downcast_raw(TypeId::of::<T>())` が null でない印を返すかで決まる。`downcast_raw` の約束は「`Some` なら、その型の正しい値を指す」。大きさ 0・整列 1 の型なら、null でない任意の番地が正しい値になる。
  - `tracing-subscriber` は自分の目印の型（`MagicPlfDowncastMarker`）を同じ口で答えさせ、層の性質を問い合わせている＝この口を目印に使うのはライブラリの作り手の形。
  - 本番の受け手（`log_history` が組む `Layered`）は、自分の型・層の型・`fmt` 層の部品の型にしか答えない。目印の型に答える所は無い。「受け手なし」（`NoSubscriber`）も既定の答え（自分の型だけ）。
  - `CaptureSubscriber` は `register_callsite` を持たない＝既定が `enabled` を通る。閉じた後に `enabled` を偽にすると、発行点の interest に `never` を焼きうる。
- **Implications**: 判定は `get_default(|d| d.is::<目印>().then(|| d.clone()))` の形。本番は `None` で子に何も据えず、`SCOPED_COUNT == 0` の速い道は保たれる。閉じた後は `event` の側で捨て、`enabled` は真のまま。

#### 捨てた案

| 案 | 中身 | 捨てた理由 |
|---|---|---|
| `Debug` の字面（`Dispatch::Global`／`Scoped`） | 種類を文字で見分ける | 安定の約束が無い（要件討議で却下済み） |
| 目印に `NoSubscriber` など `tracing` の既存の型を使う | 共有の型を借りる | `Dispatch::is::<NoSubscriber>()` の意味を壊す（窓が「受け手なし」に見える） |
| `enabled` に決まった宛先を尋ねる（`tracing::enabled!` の問い） | 窓は常に真 | 本番で `RUST_LOG=trace` だと全体の受け手も真＝本番で据わる |
| 全体の受け手の写しと比べる | 本物の全体を取り出す | 全体の受け手を取り出す公開の手が無い（窓の無いスレッドで尋ねるには新しいスレッドが要る＝要件 3.4 に反する） |
| `CaptureSubscriber` 自身に目印を答えさせる | 1 か所の上書きで済む | 全スレッドの捕捉も同じ型＝その 3 本の実行ファイルで子に差し替えが据わる（要件 3.6 の字面に反する） |
| `CaptureSubscriber` に印の欄を足す | 包む型を作らない | `capture_tests.rs` の `CaptureSubscriber(..)` の組み立てが壊れ、承認された触るファイルの表の外へ出る |

### 設計の決め

#### 決め: 見分け方は「窓専用の受け手が目印の型に答える」

- **Context**: 要件 3.6・議題 1。
- **Alternatives Considered**: 上の捨てた案の表。
- **Selected Approach**: `capture` が差す `WindowSubscriber` が、窓が開いているあいだだけ `[(); 0x6172_656B]` の `TypeId` に `self` の番地を返す。`spawn_actor` は起こす前に `get_default` で今の受け手を尋ね、答えたときだけ写しを子へ渡して `set_default` する。
- **Rationale**: 使うのは `tracing` の公開・文書化された API だけ。依存の向きは変わらない（両側が std の型の形を 1 つ写すだけ）。本番の道は 1 ビットも変わらない。
- **Trade-offs**: 目印の型が両側の写しで、片方だけ変えると黙って引き継ぎが止まる。対照の試験 ⑴ が赤にする（両側の綴りのコメントで互いを指す）。
- **Follow-up**: `tracing` の版上げのとき、`Dispatch::is` が `downcast_raw` を通る形を読み直す（Revalidation Triggers）。

#### 決め: 窓が閉じた後は目印に答えず、記録を捨てる（9 の 2 つ目＝案 C の形）

- **Selected Approach**: `WindowSubscriber` の `open`（`AtomicBool`）を、`capture` の中の `CloseOnDrop` が関数を抜けるとき（panic を含む）に偽にする。閉じる時点は `run_with_subscriber` の取り出しの後。`enabled` は真のまま、`event` で捨てる。
- **Rationale**: 窓の中でゴーストを起こすテストで、テストの終わりまで記録が溜まり続ける形（4.4）を、量を測らずに閉じられる。同じ印が目印の答えも切り替えるので、閉じた後に窓生まれのアクターが起こす孫は引き継がない（要件 2.1 の「閉じた後に起こされた」）。
- **Trade-offs**: 捨てる 1 行は公開の口から観測できない（檻に入れない・記憶の性質）。同じ印が目印の答えを切り替えることは試験 ⑹ が固める。

#### 決め: そのほかの B の項目（9 の 3〜7）

- **対照の試験の置き場**: 兄弟の新しい 1 本 `crates/areka-actor/src/spawn_log_capture_tests.rs`（`mod log_capture_tests`）。`spawn_hook_tests.rs` はフックの枠を取る 1 本の試験のまま、1.3 だけをそこへ足す。
- **要件 3.5 をどこまで固めるか**: 判定の関数の両側（受け手なし・目印に答えない受け手で `None`）と、窓の無いスレッドから起こした子の既定が起こした側と同じ（「受け手なし」同士）まで。全体の受け手へ届くことそのものは要件 5.1 のため直には見ない。
- **要件 1.3 を試験で固めるか**: 固める。`spawn_hook_tests.rs` の `probe_hook` に `trace!` を 1 件出させ、同じ 1 本の中で窓の中で起こしたアクターのフックの記録が入ることを確かめる（閉包の中の順＝据えるのが区間とフックより前、を檻に入れる）。
- **説明の直し**: `log-capture-kit` の `lib.rs`（116〜118 行）と `capture.rs`（冒頭・96〜97 行）は本 spec の中で直す。steering `structure.md` は `/kiro-complete` の文書の同期で直す。
- **赤の手当て**: `crates/areka/src/update/worker_path_tests.rs` 201 行を宛先 `areka::update::desk` に絞る。ほかは全体テスト 1 回で洗い、出たものを同じ形で直す。

### 合成（design-synthesis）

- **一般化**: 「窓の受け手を子へ渡す」は、素のスレッドへ広げるときも同じ判定の関数で足りる形（`inheritable` は `Dispatch` を受けて `Option<Dispatch>` を返すだけ）。本 spec では `spawn_actor` の 1 か所だけに使う（素のスレッドは範囲の外）。
- **作るか借りるか**: `tracing` の `downcast_raw` を借りる。新しい外のクレートは要らない（`tracing-subscriber` の層や `tracing-core` の差し替えの仕組みで代わりになるものは無い）。
- **削ったもの**: 目印の型を `log-capture-kit` から公開して両側の `TypeId` を比べる専用の試験（⑴ がずれを赤にするので要らない）・引き継ぎの有無を知らせる記録・`capture_tests.rs` の自己試験の追加（承認された表の外）。

### 危うさと手当て

- 目印の型の両側のずれ — ⑴ が赤にする・両側のコメントで互いを指す。
- 赤に変わる既存のテスト（確実 1・疑い 4〜6）— 201 行は設計で直す・ほかは全体テスト 1 回で洗う・`install/desk_overwrite_tests.rs` は `install-before-boot` と合わせてから。
- `areka-test-binary-stability` が測っているあいだの着地 — 着地させない（要件の Adjacent のとおり）。
