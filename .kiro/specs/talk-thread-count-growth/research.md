# ギャップ分析: talk-thread-count-growth

> 対象: `requirements.md`（2026-11-27 生成・未承認）と、今の main（`af6b9001` の上の `d0d9bec6`）のコード。読んだ所は「何の定義か」で指す。行番号は読んだ時点のもの。

## 1. 分析の要約

- **読む限り、トークのスレッドは全部の終わり方で終わる**。`spawn_talk`（`crates/areka-sakura/src/drive.rs`）の body は `run_inbox` 1 本で、終わり方は `settle_after_tick` の自然な終わり・空の台本（`on_start`）・再生機を組めない（`on_tick` の `Armed`）・`on_close` の 3 つの腕・受け口の切断のどれでも `Break`／`return` で抜ける。抜けた後に落ちる受け口（`EmoTextSink`・`BalloonLifecycleSink`・`NoUserBreakCueSink`）の `Drop` は塞がらない送り（unbounded）だけ。dispatcher の `on_done` が `TalkHandle` を join せずに落とすのは、std の `JoinHandle` の drop＝切り離し（スレッドの終わりを止めない）。よって**増えているのは名簿の項目と複製したハンドルだけ**という見立てが強い（確かめは要件 1 の実機の数え）。要件 6 は「行わない」側に倒れる見込み。
- **名簿は終わったスレッドを外す道を持たない**。`register_current_thread`（`thread_registry.rs` 136〜173 行）が外すのは同じ TID の項目だけ。`snapshot` は全部を複写して返す。報告器 `take_and_emit` は読めた項目の数を `threads=` に出す。名簿の項目に「終わったか」を知る手段も、役割の系統ごとの合計を持つ場所も無い（**Missing**）。
- **終わったかを知る手段は、今の `Cargo.toml` の機能の中にある**。`Win32_System_Threading` に `WaitForSingleObject`・`THREAD_SYNCHRONIZE`・`GetExitCodeThread` があり（`windows` 0.62.2 で確かめた）、`GetThreadTimes` は今も終了時刻の `FILETIME` を受けて捨てている。機能も依存も足さずに済む。どれを使うかで、複製の権限（`api.rs` の `duplicate_current_thread_handle`）に触るかが分かれる。
- **順位表の道具の区間の差は TID で照らしており、畳んだ行がそのままでは区間より前の CPU を足してしまう**（区間の始めに生きていて区間の中で終わったスレッドの、始めまでの累積）。役割の系統ごとに「生きた行＋畳んだ行」の合計の差を取る形か、消えた TID の前回の値を差し引く形かで、古いログの順位表がそのまま残るかが変わる（要件 5.3）。
- **研究が要る点**: ⑴ 複製したハンドルを名簿が持つ間は OS がその TID を使い回さないか（使い回さないなら要件 3.4・7.3 の「名簿の中での TID の使い回し」は本物のスレッドでは起こせない）⑵ `GetThreadTimes` の終了時刻は生きたスレッドで「未定義」と書かれている扱い ⑶ スレッドの終わりのフック（スレッドローカルの `Drop`）で記録や CPU の読みが安全か。

## 2. 今の様子（調べたもの）

### 2.1 スレッド名簿 `crates/wintf/src/ecs/world/thread_registry.rs`（185 行）

- 置き場は `static REGISTRY: Mutex<Vec<ThreadEntry>>`。錠の毒化は中身を受け取って続ける（`with_registry`）。
- `ThreadEntry { role, name, tid, handle: Arc<OwnedHandle> }`。`cpu_times()` は `api::get_thread_times`。
- `register_current_thread(role)`: 語彙に無い役割名は `warn!` の上で登録、複製の失敗は `error!`＋`Err`。`entries.retain(|e| e.tid != tid); entries.push(entry)`＝**同じ TID の項目だけを外す**。
- `snapshot()`: 錠の中で `clone` して返す（錠は返る前に手放し、呼び手は錠の外で `GetThreadTimes` を呼ぶ）。
- モジュールの説明に「そのスレッドが終了した後も CPU 時間を読み出せる」「登録は 1 本につき 1 度きり（複製 1 回と push 1 回）」とある。直した後の振る舞いと食い違う所＝要追随。
- 語彙 `ALL_ROLES`（8 語）と `is_known_role`。畳んだ系統の名（例 `actor:sakura-talk-*`）は登録の口を通らないので語彙の検査には掛からないが、報告の行に出る役割名の扱い（`ALL_ROLES` の説明「報告行に出る役割名はここに在るものだけ」）と合わなくなる＝要判断。

### 2.2 名簿の呼び手（全部）

| 呼び手 | 使い方 | 直しの影響 |
|---|---|---|
| `crates/areka/src/thread_roles.rs` `on_actor_thread_start` | `spawn_actor` で起きた全スレッドを `actor:<name>`（`ticker`／`loop-ticker` だけ専用名）で登録 | 読むだけ。登録の時に掃除が走るなら、トークの始まりごとに掃除が 1 回走る |
| `crates/wintf/src/runtime/mod.rs` `WinApp::new` | UI スレッドを `ui` で登録 | 無し |
| `crates/wintf/src/runtime/tick_bridge.rs` | vblank スレッドを `vblank` で登録 | 無し |
| `crates/wintf/src/ecs/clickthrough/monitor.rs` | カーソル監視を `cursor_monitor` で登録 | 無し |
| `crates/areka/src/perf_thread_report.rs` `run`／`take_and_emit` | 自分を `perf_report` で登録・`snapshot()` を舐める | 主な変更点 |
| テスト: `monitor.rs`・`tick_bridge.rs`・`runtime/mod.rs`・`thread_roles_tests.rs` | 役割名で `snapshot()` を探す。**どれもスレッドを生かしたまま観測する**（「終了した TID が再利用されて項目が置き換わる余地を残さない」） | 生きた項目だけを返す形に変えても緑のまま |
| テスト: `thread_registry_tests.rs` `registration_from_another_thread_is_visible_from_this_thread` | **join の後**に項目が見え、終わったスレッドの CPU が読めることを確かめている | **要件 2.1 と正面から食い違う＝書き直しが要る** |

### 2.3 報告器 `crates/areka/src/perf_thread_report.rs`（464 行）

- 出す順は「`perf(process)` 1 行 → 名簿のスレッド（TID 昇順）→ `unregistered_rest` 1 行」で固定（`take_and_emit`）。
- `threads=` は `samples.len()`＝**名簿から CPU を読めた項目の数**（読めない項目は除く）。要件 4.1 の「生きている名簿のスレッドの数」とは、読めない生きた項目の扱いで差が出る。
- 読めない項目は 1 枚につき 1 度の `warn!` に TID だけ並べる（役割名は出さない）。
- `compute_unregistered_rest` はプロセス − 名簿の合計（0 で止める）。説明に「終了済みのスレッドも名簿に残り、その最終値はプロセス CPU に含まれているので、引く側からも外さない」とある＝要追随。
- `delta`（本番から呼ばない・`#[allow(dead_code)]`）が「差の規則の権威」で、perf-rank はそれを写す約束。ただし perf-rank は監査で D7（名簿外の残りをプロセスの差から出す）と D8（役割別の表だけ `actor:…-<数字>` を `-*` に畳む）を足しており、Rust の `delta` はもう全部の規則の権威ではない。
- モジュールの説明 21〜24 行「読み手が隣り合うスナップショットの差を取って得る」は古い（perf-rank は最後と 60 秒前を比べる）＝要件 5.5 で直す所。
- 兄弟テスト `perf_thread_report_tests.rs`（379 行）は行の欄の重複・綴りと並び・空白の潰し・`delta`・名簿外の残り・周期・消灯時に何も起こさないことを固定している。

### 2.4 Win32 の口 `crates/wintf/src/api.rs`

- `duplicate_current_thread_handle`: `DuplicateHandle(..., THREAD_QUERY_LIMITED_INFORMATION, ...)`。この権限では `WaitForSingleObject` は使えない（`SYNCHRONIZE` が要る）。
- `get_thread_times`: `GetThreadTimes` の作成時刻・**終了時刻**を受けて捨てている。
- 使える（`windows` 0.62.2・機能 `Win32_System_Threading` の中）: `WaitForSingleObject`・`THREAD_SYNCHRONIZE`・`GetExitCodeThread`・`STILL_ACTIVE`・`WAIT_OBJECT_0`。リポジトリの中では `crates/shiori-host32-host/src/process_host.rs` が `WaitForSingleObject(h, 0)` で終わりを見る先例。
- `api.rs` はウェーブの約束 ⑶ の「名簿・報告・`perf-rank.py`」の字面に入っていない wintf のファイル（後述 6.4）。

### 2.5 トークの側（読むだけ・ウェーブの約束 ⑶）

- `crates/areka-actor/src/spawn.rs` `spawn_actor`: 素の `thread::Builder::spawn`。フック（`THREAD_START_HOOK`）を body の前に 1 度呼ぶ。`ActorHandle` は「非 RAII・drop しても join しない（detach）」。std の `JoinHandle` の drop は OS のハンドルを閉じるだけで、スレッドの走りを止めも延ばしもしない。
- `run_inbox`: 終わり方は `Ok(Break)` と切断の 2 つ（型と実装で強制）。
- `drive.rs` の終わり方（全部 `Break` か `return`）: 空の台本（`on_start`）・再生機を組めない（`on_tick` の `Armed`）・自然な終わり（`settle_after_tick`）・`on_close` の `Driving`／`Armed`／`Idle` の 3 つ・受け口の切断。`TalkDone` は `Break` の直前に送るので、dispatcher が `on_done` を受けた時点ではトークのスレッドはまだ数マイクロ秒生きている（切り離しで問題ない）。
- 選択待ち（`WaitingForChoice`）のトークは終わらないが、それは「今のトーク」1 本で、次の `Start` の `close_active_if_any`（`Close` → join）で必ず終わる＝トークの数に比例しては増えない。
- 受け口の `Drop`（`crates/areka-emo-text/src/sink.rs`・`crates/areka-runtime/src/talk_lifecycle.rs`・`user_break_cue.rs`）は塞がらない送りだけ（`UiSender::send` は unbounded の `try_send`＝`crates/areka-actor/src/ui.rs` の説明）。
- dispatcher（`crates/areka-ghost/src/dispatcher.rs`）: `on_start` は前のトークを `close_active_if_any`（`Close` → join）してから起こす。`on_done` は一致したら `self.active = None`（join しない＝切り離し）。`on_close` も `close_active_if_any`。

**読んだ結論（見立て・未確定）**: 生きたトークのスレッドはどの時点でも高々 1 本で、トークの数に比例して残る道は読める範囲に無い。ゴーストの切り替えで起こし直すアクター（`ghost-dispatcher`・`kanade`・`ticker`・`loop-ticker`・`seriko`・`shiori`・`sylphya` など）も同じ名簿に載るので、切り替えのたびに名簿の項目が増える形は同じ（名簿の直しで一緒に止まる）。番号つきの名を持つアクターは `sakura-talk-<番号>` だけ（本番の `spawn_actor` の呼び手を全部見た）。

### 2.6 順位表の道具 `tools/perf/perf-rank.py`（819 行）と自己テスト

- `scan_run_log`: `perf(thread)` は `snap t_s tid name role cpu_us kernel_us user_us` を読む（欠けたら exit 4・部分の集計を出さない）。`perf(process)` は `snap` と `cpu_us` だけ（`threads=` は読まない＝変えても止まらない）。
- `stage_thread`: 最後のスナップショットと「その 60 秒前」（無ければ最初）を比べる。`thread_delta` は **TID をキーにした辞書**（`{row.tid: row}`）で照らし、今回の行だけを回す（消えた TID は結果に出ない・巻き戻りは 0）。D7＝差の基準では名簿外の残りを `max(プロセスの差 − 名簿の行の差の合計, 0)` で置き換える。D8＝役割別の表だけ `re.sub(r"^(actor:.+)-\d+$", r"\1-*", role)` で畳む。スレッド別の表は元の役割のまま。
- 自己テスト: `fixtures-loop/rank/` の各ケース（`case.txt`＋`run.log`＋`expected_rank.txt`）を逐語で再現する。段②に関わるのは `full_ok`・`talk_threads_folded`（T22）・`tid_reused`（T4）・`thread_missing_cpu_us` など。`check_tree(readme_examples=True)` が README の例も見張る。
- **畳んだ行を足すときの落とし穴**:
  - 名簿外の残りは `tid=0`。畳んだ行も `tid=0` にすると、`thread_delta` の辞書で前回の `tid=0` の行が 1 本に潰れ、差が化ける（Rust の `delta` も `find` で最初の 1 本を拾う）。
  - 区間の始めに生きていて区間の中で終わったスレッドは、最後のスナップショットでは生きた行が消え、畳んだ合計に**最後の累積の全部**が入る。畳んだ行の差（後 − 前）はそのスレッドの区間より前の分も含むので、そのままでは要件 5.1「区間より前の分を足さず」を破る。
- 他の読み手: `judge-perf.py` は `perf(thread)`／`perf(process)` の行を**数えるだけ**（`thread_report_lines`・`process_report_lines`）で、欄は語彙の見本（`J_LINE_VOCABULARY_SAMPLES`）の重複検査だけ。`perf-compare.py`・`perf-ledger.py`・`perf_lines.py`・`*.ps1` は `perf(thread)`／`threads=` を読まない。`tools/perf/fixtures/generate.py` は判定の道具の検体に `threads=14` と `unregistered_rest` の行を書く（判定は数えるだけなので変えなくても止まらない）。

### 2.7 説明 `tools/perf/README.md` の段②（930〜946 行の辺り）

- 「名簿は終了したスレッドの登録を消しません（同じ TID が登録し直されたときだけ…）。終了したスレッドも最後の CPU 値つきで 1 行残ります」
- 「TID が使い回されると、新しいスレッドはその窓では 0 と読まれ…」
- 「値は累積です。読み手が 2 つのスナップショットの差を取ります」
- 直した後の振る舞いに合わせて書き直す所（要件 5.5）。README の例は自己テストの `check_tree` が見張る。

## 3. 要件と今の資産の対応

| 要件 | 今ある資産 | 差（Missing／Unknown／Constraint） |
|---|---|---|
| 1 実機の切り分け | 報告器（`AREKA_PERF_THREAD_REPORT_SEC=10`・`RUST_LOG` に `areka::perf=debug`）・OS の数え（`(Get-Process -Id <pid>).Threads.Count`）・トークの起こし方（areka の MCP の `raise_event`／`sakurascript`） | **Constraint**: emo2 の実走は絶対パスで短いパス・実機の根は `target\` の下・pasta の `9276` の握り・机と画面の操作はタスクの区切りの中。**Unknown**: 報告の 1 枚とトークの終わりの対の取り方（周期 10 秒に合わせて待つ手順）。 |
| 2.1 終わったスレッドを残さない | `retain` が同じ TID だけ外す | **Missing**: 終わったかを知る手段・掃除の時点（登録時／読み出し時）。 |
| 2.2 消灯でも増えない | 登録は点灯によらず常に走る | **Constraint**: 消灯時は `snapshot` が呼ばれないので、**登録時の掃除が要る**（読み出し時だけでは満たせない）。 |
| 2.3 生きた項目は今と同じ | `ThreadEntry` | 無し（形を保てる）。 |
| 2.4 項目の数の頭打ち | — | **Missing**: 系統ごとの合計の置き場。**Constraint**: 掃除は登録／読み出しの時点でしか走らないので、頭打ちは「その時点で」の約束（要件 2.1 と同じ読み）。 |
| 2.5 刻みの経路に入れない | 呼び手は起動と報告の周期だけ | 無し（登録時の掃除はトークの始まりのスレッドの頭で走るが、刻みの経路ではない）。 |
| 3.1 畳む前に最後の CPU を読む | `cpu_times()` | **Missing**: 系統への足し込み。 |
| 3.2 系統＝末尾の `-<数字>` を畳む | perf-rank の D8（`actor:` だけ） | **Constraint**: 名簿の側と perf-rank の側で同じ規則を 2 か所に持つ（Rust と Python）。 |
| 3.3 読めなかったら記録 | 報告器の `warn!`（TID だけ） | **Missing**: 名簿の側の記録（TID と役割名）。 |
| 3.4 TID の使い回し | `retain` の置き換え | **Unknown**（研究 R1）: 名簿がハンドルを持つ間は使い回されない見込み。 |
| 3.5 同じスレッドの登録し直し | `retain` の置き換え | 無し（今の振る舞いのまま・ただし「生きているか」を見て 3.4 と分ける必要）。 |
| 3.6 累積で減らない | — | **Missing**。読み出しの一貫性（下の 5.4）。 |
| 4.1 `threads=` は生きた数 | `samples.len()` | **Constraint**: 読めない生きた項目を数えるかの決め。 |
| 4.2 畳んだ行を毎回出す | 行の組み立て `format_thread_line` | **Missing**: 見分けの形（`tid`・`name`・`role`・欄の追加のどれで）。 |
| 4.3 名簿外の残り | `compute_unregistered_rest` | 畳んだ合計も引く形へ。 |
| 4.4 差を広げない | — | 実機の確かめ（要件 7.8）で見る。 |
| 4.5・4.6 行の語彙・順 | 兄弟テスト | 畳んだ行の置き場所（生きた行の後・残りの前）を決める。 |
| 4.7 消灯で起こさない | `start` の前置ガード | 無し。 |
| 5.1・5.2 区間の差 | `thread_delta`・D7・D8 | **Missing**: 畳んだ行の区間の差の規則（2.6 の落とし穴）。 |
| 5.3 古いログ | `full_ok`・`tid_reused`・`talk_threads_folded` の逐語 | **Constraint**: 規則の選び方で、古いログの出力が端の場合に変わりうる（4 章の P1）。 |
| 5.4 判定・比べ・台帳 | `judge-perf.py` は数えるだけ・他は読まない | 無し（止まらない）。語彙の見本に畳んだ行を足すかは任意。 |
| 5.5 説明 | 報告器のモジュールの説明・README の段② | 書き直し。 |
| 6 トークの後始末 | drive.rs・dispatcher.rs | 読む限り不要（2.5）。実機の数えで確かめる。 |
| 7.1 100 本以上・頭打ち | 兄弟テストの流儀「全体の件数は決して当てにしない」 | **Constraint**: 名簿はプロセス共有で並列のテストが同時に登録する。全体の数でなく**自分の系統だけ**で数えるか、プロセス共有でない芯（純データの名簿）を作って数える必要。 |
| 7.3 TID の使い回し | — | **Unknown**（研究 R1）: 本物のスレッドで起こせないなら、芯への差し込み（生死の判定を差し替えられる形）で固定する。 |
| 7.5 perf-rank の自己テスト | `fixtures-loop/rank/` | 新しいケース（区間の途中で畳まれた・古い形のログ）を足す。 |
| 7.6 消灯でも積み上がらない | — | 登録だけを繰り返すテスト（報告器を起こさない）。 |
| 7.7 読めない失敗の記録 | `log-capture-kit`（wintf の dev 依存にある） | 本物のハンドルで読めない場面を作れない＝芯への差し込みが要る見込み。 |
| 7.8 実機の数え直し | 要件 1 と同じ | 無し。 |

## 4. 実装の選択肢

### 4.1 掃除の時点（名簿）

| 案 | 中身 | 要件との当たり | 得失 |
|---|---|---|---|
| **A1 登録時と読み出し時の両方で掃除**（既存の拡張） | `register_current_thread` と `snapshot` が錠の中で全項目の生死を見て、終わった項目の最後の CPU を系統の合計へ足して外す | 2.1・2.2・2.4 を満たす | 既存の 1 ファイルの中で済む。登録のたびに n 回（今は 20 前後）の Win32 呼び出しが錠の中で走る（費用はトーク 1 つにつき数十 µs の見込み・刻みの外）。モジュールの説明の「登録は複製 1 回と push 1 回」は改める。 |
| A2 読み出し時だけ | `snapshot` だけが掃除 | **2.2 を満たさない**（消灯時は読まれない） | 不採用の候補（A1 の片方としてだけ使う）。 |
| A3 スレッドの終わりのフック | 登録した時にスレッドローカルの見張りを置き、その `Drop`（スレッドの後始末）で項目を畳む／「終わった」の印を立てる | 2.1・2.2 を満たす。生死の Win32 の呼び出しは要らない | 時点が正確。ただし ⑴ スレッドローカルの後始末はスレッドの本当の終わりより少し前で、そこで読む CPU は最後の値より少し小さい（差は名簿外の残りへ） ⑵ 後始末の中で `tracing` を呼ぶ安全さ（研究 R3）⑶ UI スレッドなど本体のスレッドは後始末が走らないことがある ⑷ `ThreadStartHook` の型は変えずに名簿の中だけで済む。**印だけ立てて CPU は A1 の掃除で後から読む**混ぜ方（A1＋A3）もある。 |
| A4 上限つきの輪 | 項目の数に上限を置いて古いものから捨てる | 3.1（黙って落とさない）を破る | 不採用。 |

### 4.2 終わったかの見分け方（A1 のとき）

| 案 | 呼び出し | 触る所 | 得失 |
|---|---|---|---|
| **L1 `WaitForSingleObject(h, 0) == WAIT_OBJECT_0`** | 文書に書かれた正の方法 | 複製の権限に `THREAD_SYNCHRONIZE` を足す（`api.rs` の `duplicate_current_thread_handle`）・新しい口 1 つ | 確実。権限を広げる（同期待ちだけ）。`shiori-host32-host` に先例。 |
| L2 `GetThreadTimes` の終了時刻が 0 でない | 今の呼び出しの捨てている出力を使う | `api.rs` の `get_thread_times` の戻りに終了時刻を足す（権限は今のまま） | 1 回の呼び出しで「最後の CPU」と「終わったか」が同時に取れる。**文書は生きたスレッドの終了時刻を「未定義」と書く**（研究 R2）。 |
| L3 `GetExitCodeThread == STILL_ACTIVE` | 今の権限（LIMITED）で呼べる | `api.rs` に新しい口 1 つ | 終了コードが 259 のスレッドを生きていると誤る（Rust のスレッドは 0 で終わるので実害は薄い・文書の注意書きどおり）。 |
| L4 A3 の印 | Win32 を呼ばない | 名簿の中だけ | 4.1 の A3 の得失。 |

どの案でも依存と `Cargo.toml` の機能は足さない（全部 `Win32_System_Threading` の中・`windows` 0.62.2 で確かめた）。ただし L1〜L3 は **`crates/wintf/src/api.rs` に触る**（6.4）。v0.2.0 の windows-rs の移行の本体（`windows` クレートをやめて束縛へ移る）では、増えた Win32 の口も束縛に載せる対象になる（L2 は新しい関数を増やさない）。

### 4.3 畳んだ合計の持ち方と読み出しの一貫性（名簿・報告器）

- 置き場: 名簿の中に「系統名 → カーネル／ユーザーの累積（100ns）と畳んだ本数」を持つ。項目の数は「生きた項目＋系統の数」。
- **一貫性の穴**: 今の `snapshot` は錠の中で複写し、報告器は錠の外で CPU を読む。掃除を足すと「報告器が生きた項目として複写した直後に別のスレッドの登録が同じ項目を畳む」→ 1 枚の中で同じスレッドを生きた行と畳んだ合計の両方に数える、が起こりうる（要件 3.6・4.3 の算出が崩れる）。選択肢:
  - C1 生きた項目の CPU の読みも錠の中で行い、項目と合計を 1 つの時点の組として返す（錠の中で n 回の `GetThreadTimes`・数十 µs）。
  - C2 名簿に世代の番号を持ち、報告器が読み終えた後に世代が動いていたら読み直す。
  - C3 掃除で外した項目を「最後の値の読み済み」として 1 回だけ報告器へ渡す（行の作りが複雑になる）。
- 公開の口: `snapshot()` は生きた項目だけを返す今の型のまま残し（呼び手 8 か所が変わらない）、畳んだ合計は別の口か、生きた項目と合計を束ねた新しい読み出しで返す、のどちらか。

### 4.4 報告の行の形（報告器）

| 案 | 畳んだ行の形 | 得失 |
|---|---|---|
| R1 今の `perf(thread)` の欄のまま見分けを値で付ける | 例: `tid=0 name=<印> role=actor:sakura-talk-*`。生きた行の後・残りの前に系統ごと 1 行 | 欄の綴りと並びが今のまま（4.6）。`tid=0` は名簿外の残りと重なるので、perf-rank と Rust の `delta` の照らし方を TID だけから外す必要。`role` に `-*` が付くのはトークだけで、`actor:kanade` のような番号の無い系統は生きた行と同じ役割名になる（見分けは `name`／`tid` に頼る）。 |
| R2 畳んだ行にだけ欄を 1 つ足す | 例: `… user_us=… exited=<本数>` | 見分けが字面で確か。欄の集合が行の種類で変わる（判定の道具の語彙の見本に足すか）。 |
| R3 別の行の種類 | 例: `perf(thread_exited): …` | perf-rank が新しい種類を読む。`judge-perf.py` の数えには入らない。 |

`perf(process)` の `threads=` は「生きた名簿の数」へ。終わった本数を別に出すか（例 `exited_threads=`）は要件 4.1 の前提（未確定）のとおり設計で決める。

### 4.5 区間の差（perf-rank と Rust の `delta`）

| 案 | 規則 | 要件 5.1〜5.3 |
|---|---|---|
| P1 系統の合計の差 | 系統ごとに「生きた行＋畳んだ行」の合計を前後で取り、差を 0 で止める。役割別の表と名簿の差の合計（D7 の引き算）はこれで出す。スレッド別の表は今の TID の差のまま | 5.1・5.2 を満たし、TID の使い回しに強い。古いログでも `full_ok`・`tid_reused`・`talk_threads_folded` の出力は変わらない（手で追った）が、**同じ系統の中で TID ごとの 0 止めと系統の 0 止めがずれる端の場合**に古いログの出力が変わりうる（5.3「今と同じ順位表」に当たるかの判断）。 |
| P2 TID の差＋消えた TID の差し引き | 生きた行は今の TID の差。畳んだ行は（後 − 前）から「前回にあって今回に無い TID の、同じ系統の前回の値」を引く | 古いログ（畳んだ行が無い）では今と逐語で同じ。TID の使い回し（名簿が手放した後に起こりうる）では、使い回した新しいスレッドの差が 0 止めのまま、古いスレッドの前の値を引けず畳んだ行が膨らむ端の場合が残る。 |
| P3 終わったスレッドの最後の行を 1 度だけ出す | 報告器が、畳んだ直後の 1 枚だけ終わったスレッドの TID つきの最後の行を出し、以後は系統の合計に入れる | perf-rank は TID で照らせる。行が増え、報告器に「1 度だけ」の状態が要る。 |

どの案でも、Rust の `delta`（規則の権威）と兄弟テストを同じ規則へ直し、perf-rank の `thread_delta` の説明と合わせる。

### 4.6 総合の組み合わせ（例）

- **案 甲（最小・既存の拡張）**: A1＋L2（終了時刻）＋C1＋R1＋P1。`api.rs` は `get_thread_times` の戻りを広げるだけ。権限は今のまま。研究 R2 が通ることが前提。
- **案 乙（文書に忠実）**: A1＋L1（`WaitForSingleObject`）＋C1＋R2＋P1 か P2。`api.rs` の複製の権限を広げ、口を 1 つ足す。
- **案 丙（終わりのフック）**: A3（印だけ）＋A1 の掃除で CPU を読む＋C1＋R1／R2＋P1。Win32 の新しい口は要らないが、研究 R3 が要る。

## 5. 研究が要る点（設計の段へ持ち越す）

- **R1 名簿がハンドルを持つ間、OS はその TID を使い回すか**。Windows ではスレッドの番号はスレッドのオブジェクトが消える（全部のハンドルが閉じる）まで使い回されない、という理解が一般的。そうなら、名簿の中で「終わったスレッドの TID を新しいスレッドが使い回す」（要件 3.4）は起こらず、使い回しはログの側（名簿が手放した後のスナップショットの間）でだけ起こる。要件 7.3 の固定を本物のスレッドで作れないので、生死の判定を差し替えられる芯での固定になる。Microsoft の文書で確かめる。
- **R2 `GetThreadTimes` の終了時刻**: 文書は「スレッドが終わっていなければ中身は未定義」。0 が返る実際の振る舞いに頼ってよいか。頼れないなら L1 か L3。
- **R3 スレッドローカルの後始末（A3）**: Windows の std のスレッドで `thread_local!` の `Drop` が走る順、その中で `tracing` の記録・`Mutex` の錠・`GetThreadTimes` を呼んでよいか、UI スレッド（本体）で走らない場合の扱い。
- **R4 1 枚の一貫性**: C1〜C3 のどれで「同じスレッドを 1 枚の中で 2 度数えない」を守るか、錠の中で CPU を読む費用。
- **R5 実機の手順**: 10 秒の周期で「各トークの終わりの後に 1 枚」を確実に対にする待ち方。トークを MCP で流すときの、起動の挨拶と MCP の呼び出しの重なり。

## 6. 規模・危うさ・ウェーブの約束

### 6.1 規模
**S〜M（5〜10 タスクの見込み・brief のまま）**。実機の切り分け 1・名簿（掃除・畳み・記録・芯）2〜3・報告器（行・`threads=`・残り・`delta`）1〜2・perf-rank（区間の規則・自己テストのケース）1〜2・説明 1・実機の数え直し 1。トークの側は読む限り 0。

### 6.2 危うさ
**中〜低**。既存の型と流儀の拡張で、新しい並行の設計は要らない。危うさは ⑴ 1 枚の一貫性（4.3）⑵ Win32 の振る舞いの細部（R1・R2）⑶ 古いログの順位表を変えないこと（5.3）と区間の正しさ（5.1）の両立 ⑷ 実機の確かめの段取り（机・画面・emo2 の実走の決まり）。

### 6.3 トークの側（要件 6）
読む限り、トークのスレッドは全部の終わり方で終わり、切り離しも終わりを妨げない（2.5）。実機の数えで「生きたスレッドは増えない」と出れば、`drive.rs`・`dispatcher.rs` には触らない（要件 6.4）。出なかったときだけ、`talk-script-metadata` の着地の後へ回す（ウェーブの約束 ⑶）。

### 6.4 ウェーブの約束 ⑶ との当たり
約束の字面は「名簿・報告・`perf-rank.py` だけ」。直しで触る見込みのファイルは次のとおりで、字面の外が混じる:

- 字面の中: `crates/wintf/src/ecs/world/thread_registry.rs`・`crates/areka/src/perf_thread_report.rs`・`tools/perf/perf-rank.py`
- 兄弟（要件の範囲の中・同じウェーブの他の spec の触るファイルと重ならない）: `thread_registry_tests.rs`・`perf_thread_report_tests.rs`・`tools/perf/fixtures-loop/rank/` の新しいケース・`tools/perf/README.md`
- **字面の外**: `crates/wintf/src/api.rs`（L1〜L3 のとき。L4／A3 なら触らない）。C28 の他の 5 本の触るファイルの一覧に `api.rs` は無い（ウェーブの表で確かめた）ので重なりは 0 だが、約束の字面を広げるかの扱いが要る。
- 読むだけ: `crates/areka/src/thread_roles.rs`・`drive.rs`・`dispatcher.rs`・`crates/areka-actor/src/spawn.rs`。

## 7. 設計の段への申し送り（決めること）

1. 掃除の時点（A1＝登録時＋読み出し時／A3 のフックとの混ぜ方）。
2. 終わったかの見分け方（L1 `WaitForSingleObject`＋権限の追加／L2 終了時刻／L3 終了コード／L4 印）と、`api.rs` に触ることの扱い。
3. 1 枚の一貫性の守り方（C1 錠の中で読む／C2 世代／C3 1 度だけ渡す）。
4. 畳んだ行の見分けの形（R1 値で／R2 欄を足す／R3 別の種類）と、`tid` の値（名簿外の残りの `0` と重ねない）。
5. `threads=` の意味と、終わった本数を別に出すか（`perf(process)` に欄を足すか）。読めない生きた項目を数えるか。
6. 系統の名の規則（`-<数字>` を `-*` に・全部の役割か `actor:` だけか）と、系統の名を報告の行に出すときの語彙（`ALL_ROLES` の説明との合わせ方）。
7. 区間の差の規則（P1 系統の合計／P2 消えた TID の差し引き／P3 最後の行を 1 度）と、Rust の `delta`（権威）と perf-rank の写しの合わせ方。古いログの順位表を「逐語で同じ」に縛るか。
8. 決定論テストの立て方: プロセス共有の名簿で「自分の系統だけを数える」本物のスレッドのテスト（要件 7.1・7.2・7.6）と、生死の判定と CPU の読みを差し替えられる芯のテスト（要件 7.3 の TID の使い回し・7.7 の読めない失敗）の分け方。
9. 掃除で外すときの記録の形（読めなかった TID と役割名・`error!` か `warn!` か・名簿外の残りへ回ることの書き方）。
10. 実機の手順（R5）。
11. 要件 3.4・7.3 の TID の使い回しの扱い: R1 のとおり名簿がハンドルを持つ間は使い回しが起こらないなら、3.4 は守りとして残し、7.3 の固定は生死の判定を差し替えられる芯で行うか（本物のスレッドでは起こせない）。

要件の議論で決まったこと（設計はこれに従う）: `api.rs` は名簿の側の一部として触ってよい（2）・`threads=` は生きている名簿のスレッドの数で、CPU を読めなかった生きたスレッドも数え、畳んだ行に畳んだ本数（累積）を添える（5）・系統は役割によらず末尾の `-<数字>` を `-*` に畳み、番号の無い役割名はそのまま、畳んだ行は役割名でなく行の形で見分ける（4・6）・古いログは止まらずに読み役割別の CPU を正しく出せばよく、今の道具と逐語で同じには縛らない（7）・実機は MCP でトーク 5 回以上・数え直しにゴーストの切り替えを 1 回足す（10）。
