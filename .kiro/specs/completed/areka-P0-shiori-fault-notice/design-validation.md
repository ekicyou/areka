# 設計レビュー: areka-P0-shiori-fault-notice

> 2026-09-24・本ブランチ（HEAD `59a250ca`）で `design.md`（2026-09-24 生成）を `requirements.md`・`research.md`（§6 の 2026-09-24 討議の反映 2 件と §7 の設計ログ）・steering（`product.md`・`tech.md`・`structure.md`・`logging.md`・`focus.md`・`roadmap.md`）と突き合わせ、設計が書いているコードの事実を Grep／Read で引き直した。コードは「何の定義か」（関数名・型名＋ファイルパス）で指す。非対話で実施（質問はしていない）。

## 要約

設計は「既にある 1 本道（kanade の停止通知 → `quit_app` → `fn main` の後始末）に、失敗の種類と理由・最初の出所・場面ごとの題名・終了コードを足す」形で、新しい仕組み・新しい依存・新しい環境変数を 1 つも増やしていない。要件 1〜8 の全項目（1.1〜8.7）が追跡表に載り、開発者の 3 つの裁定（エラー応答は致命にしない／0 以外の終了コードはプロセスが終わるときだけ／#57 と LogSink 側の穴を本仕様で拾う）はそれぞれ Flow 2・「Data Contracts」・`finish_after_run`・`KanadeStopRx` に落ちている。設計が根拠にしているコードの主張は、下の「引き直した事実」のとおりすべて実物と一致した（1 点、bevy の関数の所在ファイル名だけが違う）。実装に進んでよい。残す論点は 3 つで、いずれも設計ディスカッションで数行の追記か 1 本の小さなテストの追加で閉じる。

## 引き直した事実（設計の主張 → 実物）

| 設計の主張 | 実物 | 判定 |
|---|---|---|
| エラー応答を「返事なし」に写す場所は `round_trip_request` 1 か所で足りる | `crates/areka-kanade/src/actor.rs` の `round_trip_request` が GET／NOTIFY の唯一の送出点（`round_trip_unload` は別・Unload の失敗は `real.rs` の `run_shiori_loop` が `Ipc` で返す）。`crates/areka-kanade/src/schedule/boot.rs` の `on_reply` は `BootInit` で `Notified` だけを次へ進め、`NoContent` は `unexpected_reply` で相を維持する＝運行表側で写すと起動が止まる | 一致 |
| `run()` の後で World の資源（`FirstExit`）を読める | `crates/wintf/src/runtime/mod.rs` の `WinApp::run(&self)` は `&self` で、戻った後も `app.world()`（`Rc` の複製）が生きている。手順 4.5 で登録表に残った窓を壊してから戻る（`exit requested while windows remained open — destroying them before returning`） | 一致 |
| `AppExit` は最初が勝つ | `crates/wintf/src/runtime/message_loop.rs` の `AppExit::request_exit` が `requested.replace(true)` で 2 度目を `debug!` で流す | 一致 |
| `HOST32_TESTDLL_LOADU_FAIL` は helper の子へ届く | `crates/shiori-host32-host/src/process_host.rs` の `spawn` は `Command::new(helper_exe)` に `.env` 3 つを足すだけで `env_clear` を呼ばない | 一致 |
| bevy 0.19.1 は `.before(system)` の相手が schedule に無くても失敗しない | `bevy_ecs-0.19.1/src/schedule/node.rs` の `SystemSets::check_type_set_ambiguity` は `instances > 1 && relations > 0` のときだけ `SystemTypeSetAmbiguityError` を返す（`Cargo.lock` の版は 0.19.1）。設計は所在を `schedule.rs` と書いているが正しくは `node.rs`（文書の綴りだけ） | 一致（所在の綴りのみ要訂正） |
| 受け口は今 `Emo2Wiring` の中にだけあり、LogSink 側の起動には無い | `crates/areka/src/emo2_boot/frame/wiring.rs` の `Emo2Wiring.kanade_stop: Option<Receiver<..>>`・`set_kanade_stop`。`crates/areka/src/main.rs` の `fn main` の `else` 腕は `areka_ghost::boot(..)`＝`boot_with_kanade_stop(options, None)` | 一致 |
| `run_ghost_quit_phase` は停止原因で分岐せず `quit_app(world, ExitOrigin::KanadeStopped(cause))` を 1 度呼ぶ | `crates/areka/src/emo2_boot/frame.rs` の `run_ghost_quit_phase` | 一致 |
| `quit_app` の呼び手は 4 か所 | `frame.rs`（`KanadeStopped`）・`input_events/mod.rs`（`Escape`）・`main.rs`（`Smoke`）・`app_exit.rs` の `on_ghost_os_close`（`OsClose`） | 一致 |
| `KanadeStopCause`・`KanadeStopped`・`ExitOrigin` は `Copy` | `crates/areka-kanade/src/msg.rs`・`crates/areka/src/app_exit.rs` の derive | 一致（`String` を載せるので `Clone` へ） |
| `ShioriDown` は理由の文字列だけを運ぶ・送出点は `real.rs` の 2 か所 | `crates/areka-kanade/src/shiori/real.rs` の `spawn_shiori_actor`（接続失敗）と `report_exit_once`（helper の終了） | 一致 |
| `username` 照会のエラー応答が `NoContent` になっても sink の値は同じ | `crates/areka-ghost/src/sylphya_wiring.rs` の `make_username_resource_sink` は `NoContent | Failed(_) => None` | 一致 |
| 選択肢の往復中の失敗は既に 204 と同じ扱い | `crates/areka-kanade/src/schedule/steady.rs` の `choice_shiori_failed_as_204`（2 か所） | 一致 |
| `OnClose` GET がエラー応答なら（写した後は）204＝無言終了へ | `crates/areka-kanade/src/schedule/close.rs` の `ClosePending` の `NoContent` の腕が `CloseSilent`。裁定 3 の帰結として整合（告知なし・終了コード 0） | 一致・設計の帰結 |
| 失敗注入の mock は会話中（GET）の失敗も作れる | `crates/areka-kanade/tests/kanade/common/common_mock_shiori.rs` の `FailOn { id: &'static str, kind }` は任意のイベント id を取る | 一致（`OnSecondChange` の注入が組める） |
| smoke ④ の検体に要る i686 の `shiori_loadu.dll` | 本ワークツリーに `target/i686-pc-windows-msvc/debug/shiori-host32-helper.exe` は在り、`shiori_loadu.dll` は無い | 設計の Open Questions と一致（④ は最初の走行で「建て方の案内」で落ちる） |

steering との整合: 本番コードが読む環境変数の追加は 0（`HOST32_TESTDLL_LOADU_FAIL` の読み手は test crate＝`structure.md` の記述どおり）。記録なしの失敗経路は無い（`shiori_error_response` は `warn!`・`app_exit_again` は `debug!`・告知は既存の `error!(event="alert")`）。1 フレーム遅らせる解は含まれない。決定論テストは判断の分岐（写し・判定表・`finish_after_run`・文面）に限られている。

## 重要な論点（3 件）

### 論点 1: 常設 smoke の i686 成果物の複製が 3 本の並走で競合する

- **問題**: `ensure_i686_artifact` は `areka.exe` の隣（`target/debug/`）へ `shiori-host32-helper.exe` を複製する。smoke ①②④ は同じテストバイナリの中で並走する（cargo test は 1 バイナリ内のテストをスレッドで並走させる）ので、3 本が同時に「無い → 複製する」に入り、書きかけのファイルを別のテストの子プロセスが起動しにいく形が起こりうる（起動できず「接続できなかった」→ ①② が偽の赤）。設計の Risks は「複製は使い捨てで共有しない」と書くが、それは検体フォルダの話で、helper の置き先は 3 本で共有される。
- **影響**: 要件 4.1「3 方向とも緑のまま」が環境によって時々破れる。常設の検査が不安定になると赤が信用されなくなる。
- **提案**: 複製を同じプロセス内で 1 度に絞る（`std::sync::OnceLock` か `Once` で囲む。隣に既にあれば複製しない）。書く側は一時名へ書いてから改名する。④ の `shiori_loadu.dll` の置き先は検体の複製の中（テストごとに別）なので共有の問題は無い＝設計の「同じ関数」は「探索が同じ・置き先は別」と分けて書く。あわせて「建て方の案内」に `cargo build -p shiori-host32-testdll-loadu --target i686-pc-windows-msvc` も載せる（本ワークツリーは helper だけが建っており DLL は無い＝④ は最初の走行で必ずこの案内に当たる）。
- **要件**: 4.1・4.2・4.3。**根拠**: design「検査 / smoke ①②④・`ensure_i686_artifact`」の Batch 契約と Risks。

### 論点 2: `from_failure` の `Shiori → Internal` の腕は、討議の反映（research §6 ⑵）と食い違うまま「覆した裁定 0 件」になっている

- **問題**: research §6 の 2026-09-24 討議の反映 ⑵ は「`ShioriFailure` 5 種から Fault の種類への写しは `Shiori` の腕を持たない（到達しない腕を置かない・wildcard も置かない＝写しの入口を『Fault へ入る 4 種』に絞った型にする）」と書き、設計はこれを「§1〜§5 より優先」の入力にしている。ところが design の決定 2 は `Shiori → Internal` の腕を置き、`msg_fault_tests.rs` で「5 腕（`Shiori` → `Internal` を含む）」を固定する。この腕は設計自身の契約（送出点で写すので届かない）により実行されない。追跡表 8.7 は「設計中に覆した裁定は 0 件」と書くが、この点は討議の結論を意図的に変えている。
- **影響**: 実装者がどちらに従うか迷う。実行されない腕を固定するテストは、後の変更で腕が消えても何も守らない（steering: テストは判断の分岐に限る・到達する経路を踏ませる）。
- **提案**: 型を絞る案は `ShioriOutcome::Failed(ShioriFailure)` が mock と本番の共通の境界型なので、失敗の enum をもう 1 つ増やすことになり割に合わない。設計の形（腕を残す）でよいが、⑴ design 決定 2 と追跡表 8.7 に「§6 ⑵ の『型を絞る』は採らず、腕を契約の見張りとして残す」と明記し、⑵ `msg_fault_tests.rs` は届く 4 腕＋`from_down` 2 腕＋`unknown` を固定し、`Shiori` の腕は「届かない契約」の注記だけにする（固定するなら『契約が破れたときの見え方』として明示的に名付ける）。設計ディスカッションで開発者の確認を取る（要件 8.7 の手続き）。
- **要件**: 1.4・2.2・7.3・8.7。**根拠**: design「決定 2」・「`ShioriFault`／`ShioriFaultKind`」の State Management・Testing Strategy の `msg_fault_tests.rs`。research §6 討議の反映 ⑵。

### 論点 3: LogSink 側の起動で `ghost_quit_system` が実際に登録・実行される形を、どのテストも踏まない

- **問題**: 設計は「`.before(emo2_frame_system)` の相手が無くても bevy は失敗しない」を bevy のソースを読んで裏付けている（実物と一致＝上の表）。しかし本仕様の後、この登録が実行される構成（`Emo2Wiring` 無し＋`ghost_quit_system` 登録）を踏むテストは 1 本も無い: `frame_ghost_quit_logsink_tests.rs` は `run_ghost_quit_phase(world)` を直接呼ぶ設計で、`wire_kanade_stop` の登録も `Update` の実行も通らない。smoke ①② は実 sink 結線の経路、③ は起動前に止まる。つまり bevy の版が変わってこの前提が崩れると、本番の LogSink 側の起動だけが schedule の組み立てで落ちる（利用者が踏むのは実 sink が組めない縮退のときだけだが、要件 6.4 が守ると約束した経路そのもの）。
- **影響**: 要件 6.4・7.6 の「LogSink 側でも停止通知が受け口に届く」の検証が、受け口の関数だけを見て、据え付けの形（system の登録と順序指定）を見ていない。
- **提案**: 既存の `crates/areka/src/emo2_boot/frame_schedule_tests.rs`（`Schedules` の構造を見る置き場）に 1 本足す: 素の World に `AppExit`・`KanadeStopRx` を挿し、`emo2_frame_system` を登録せずに `wire_kanade_stop` 相当の登録（`ghost_quit_system.before(emo2_frame_system)`）を行い、停止通知を 1 件送って `Update` を 1 回走らせ、`AppExit::is_requested()` と `FirstExit` を見る。組み立てで落ちれば赤、届かなければ赤。`frame_ghost_quit_logsink_tests.rs` の起動込みの 1 本はそのままでよい（こちらは接続失敗 → 通知の到達を見る）。
- **要件**: 6.4・7.6。**根拠**: design「`wire_kanade_stop`・`wire_emo2_boot`」の Implementation Notes「据え付けそのものは配線（再テストしない）」・Testing Strategy の `frame_ghost_quit_logsink_tests.rs`。

## 小さな指摘（ディスカッションで拾えば足りる）

- design「Technology Stack」と research §7.1 の 4・§7.6: bevy の検査関数の所在は `bevy_ecs-0.19.1/src/schedule/node.rs`（`SystemSets::check_type_set_ambiguity`）。`schedule.rs` はそれを呼ぶ側（`build_schedule`）。
- `crates/areka/src/emo2_boot/spine_close_wiring_tests.rs` が `run_ghost_quit_phase(&mut harness.wiring, &mut harness.world)` を直接呼んでいる。設計の File Structure Plan には無いが、署名が `run_ghost_quit_phase(world)` に変わるので追随が要る（判断なし・機械的）。
- 告知を後始末 ① より前に出すため、利用者が告知を閉じるまでのあいだ SERIKO の loop ticker（16 ms）が seriko を回し続け、表示指令が誰も取り出さないチャネルに溜まる。① は `GhostRuntime` を消費しないので、告知を ① の直後・② の前に出しても名前は読めるし順序 ①→②→③→④ も変わらない。害は小さい（溜まるのは面の切替のたびの小さな指令）が、費用 0 で避けられる。
- 要件 2.1・7.1 の「6 語」は設計の Open Questions のとおり 5 語が確定。要件文の綴りの直しはディスカッションで。

## 設計の強み

1. **写す場所の選び方が実物に裏付けられている。** エラー応答を「返事なし」にする場所を運行表ではなく送出点 `round_trip_request` に置いた判断は、`BootInit` が `Notified` しか受けないという実物の制約から導かれており、本番（`real.rs`）と mock の両方が必ず通る 1 点で全相に効く。裁定 3 を最小の変更（1 か所・記録 1 件）で実現している。
2. **不正な状態を型で作れない・判定が 1 つ。** `KanadeStopCause::Fault(ShioriFault)` は「Fault 以外に中身がある」組み合わせを排し、`FirstExit` は `AppExit` と同じ「最初が勝つ」規則を World の資源で再現し（要件 4.5 の競合が自然に決まる）、`fault_of` 1 つで「告知するか」と「終了コード」を同時に決める。wintf 無改変・新しい依存 0・環境変数 0 のまま、後続 #58（`finish_after_run` を括り出す）と #15（`alert_text` の `(title, body)`）が乗る形になっている。

## 判定

**GO。** 既存の 1 本道を壊さず、要件 1〜8 と 3 つの裁定を過不足なく写している。設計が根拠にしたコードの主張はすべて実物と一致した。3 つの論点は、⑴ smoke の複製の直列化と案内の 1 行、⑵ 討議の反映 ⑵ を覆す旨の明記（開発者の確認）、⑶ LogSink 側の登録を踏む小さなテスト 1 本——のいずれも設計の骨を変えず、ディスカッションで追記して閉じられる。

### 次の手順

1. `/kiro-design-discussion areka-P0-shiori-fault-notice` で論点 1〜3 と小さな指摘を扱い、`design.md` に追記する（`requirements.md` は 2.1・7.1 の「6 語」の綴りだけ）。
2. 追記後に `/kiro-spec-tasks areka-P0-shiori-fault-notice` へ進む。実装の前に i686 の 2 成果物（`shiori-host32-helper`・`shiori-host32-testdll-loadu`）を建てておく（本ワークツリーは helper だけが在る）。
