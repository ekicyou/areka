# ギャップ分析: areka-test-binary-stability

> 2026-11-01 `/kiro-validate-gap`。枝 `claude/kiro-start-areka-test-binary-stability-dec94e`（main `0cd4577f` の上・`80db9b54`）のコードを読んで書いた。テスト・ビルド・負荷の走行は回していない（この段は読むだけ）。行の番号はこのコミットのもの。

## 分析の要約

- **件 1・件 3 の落ちには、このリポジトリの中に強い前例が 2 つある。** ① 完了 `wintf-gpu-test-crash`（2026-07-24）は「同じプロセスで、別のスレッドに 2 つ目の MTA に縛られた `Compositor` を作ると `0xc0000005`」を突き止め、wintf の graphics の実行ファイルを 1 本のオーナーのスレッド（`on_gpu_owner_thread`）へ寄せて直した。ところが `areka` の実行ファイルは、今もテストのスレッドごとに MTA の `Compositor` を作っている（`WucGraphicsResource::new` を呼ぶファイルが 8 本）。② `wintf` の `WicCore` の説明（`wic_core.rs` 16〜22 行）は「並走のテストが一時的に立てた MTA が解体されると、COM の物の `Release` が解放済みのメモリに触れて `0xC0000005`」を書き、`CoIncrementMTAUsage` でプロセスの寿命の MTA を常に立てて直している。件 3 の組（MTA を張って `CoUninitialize` で解くテストと、ASTA の `Compositor` のテスト）は②の形に近い。
- **落ちたテストを名指す仕掛け（要件 1.1）は今は無い。** 開発機に `cdb`／WinDbg は無い（`Windows Kits\10\Debuggers\x64\` は `dbghelp.dll` などの DLL だけ）。WER のダンプの置き場の設定は管理者の操作でシステムの設定を変える。いちばん軽いのは、テストのプロセスの中で `SetUnhandledExceptionFilter`（または vectored の例外の受け手）を据え、落ちたスレッドの名前（libtest はテストごとにテストの名前のスレッドを起こす）・落ちた番地のモジュール・呼び出しの積み上げを標準エラーへ書く形。今の `windows` の機能の一覧には `Win32_System_Diagnostics_Debug` が無い。
- **件 2 の 5 種類の数は brief のとおり**（目印の無い `spin_wait_until` の直の呼び出し 15 か所・`join_bounded` 3 つ・打ち切りの無い `GpuSlots::take`・目印の無い `manual_balloon_runtime`・UI の側を数えない `SwitchRig::progress_probe`）。どれも待ちの部品（`spine_wait.rs`）の形に載せられる。目印に使える口は、偽の SHIORI の `call_count()`・足場の数え・テストが差し込む閉包の側にほぼ揃っている。口が無いのは `WintfTaskPool`（wintf の本番のファイル）の 1 か所。
- **件 4 と件 5 は同じ足場を通る。** 件 5 の 2 本はどちらも `SwitchRig::new` で検体を組む（`main_session_mark_tests.rs` の `rig_with`・`main_session_mark_write_tests.rs` 199 行）。件 4 の `remove_dir`（`ghost_switch_test_support.rs` 223 行）が同じ所にある。赤の文が残っていないので確かめてはいないが、同じ根を疑う材料になる。
- **規模は L（13〜20 タスク）・危うさは高。** 原因が分かっていない落ちを測ってから直すので、手順の先頭（名指しの仕掛けと対照）に重みがある。上限に近いファイル（`frame_test_support.rs` 974・`spine_conformance_support_tests.rs` 959・`spine_conformance_lap_tests.rs` 914・`main.rs` 986）を先に分ける手間が足される。

---

## 1. 今あるもの

### 1.1 待ちの部品（`crates/areka/src/emo2_boot/spine_wait.rs`・462 行）

| 部品 | 形 | この spec との関わり |
|---|---|---|
| `wait_until_with`／`wait_until` | 時計と休みを差し替えられる芯。`Progress::Count` なら「30 秒進まない＝`［止まった］`」「300 秒で届かない＝`［進んではいた］`」、`Progress::Unknown` なら「30 秒＝`［進みは不明］`」 | 件 2 の移し先。檻は偽の時計で打ち切りを確かめられる（`spine_wait_tests.rs` 356 行） |
| `wait_recv`／`run_bounded_watching` | 受け口・別スレッドの閉包を芯で待つ | `join_bounded` の作り直しの候補の部品 |
| `spin_wait_until`（336 行） | 目印なしの古い呼び名。打ち切ると標準エラーへ 1 行出して `false` | 件 2 の項目 5 の 15 か所の呼び先 |
| `run_bounded`（364 行） | 総時間の古い形（`［進みは不明］` を後ろに足す） | 本 spec の数には入っていないが、同じく目印なし |
| `join_bounded`（385 行・`pub(super)`） | `ActorHandle` を受け、目印は `is_finished` の 0→1 | 件 2 の項目 2。呼び手は `spine.rs` 875 行・`spine_talk_close_tests.rs` 368 行と檻 |
| `GpuSlots`／`GpuPermit`（399〜462 行） | 同時に 4 つ・`Condvar::wait_while` で打ち切り無し・`try_take` は檻 16 用 | 件 2 の項目 4 と要件 1.4 の対照 |

`join_bounded` の写しは `install/desk_pick_tests.rs` 60 行と `install/fetch_url_tests.rs` 80 行（どちらも `JoinHandle<()>` を受け、目印は `is_finished`）。打ち切りの文の形が spine のもの（`'{what}' join did not complete (possible hang) — {failure}`）と写し（`{failure}` だけ）で違う＝要件 4.4 の「同じ形」に揃っていない。

### 1.2 足場

- **`SwitchRig`**（`emo2_boot/ghost_switch_test_support.rs`・722 行）: `new`（206 行〜）がテストのスレッドで `CoInitializeEx(MULTITHREADED)` を呼び（211 行・対の `CoUninitialize` は無い）、`acquire_emo2()` の検体の中で既定のバルーンを根へ移して（`rename`）から空になった `balloon` を `remove_dir` する（223 行＝件 4）。`progress_probe`（479 行）は「作り口の呼ばれた数＋偽の SHIORI の呼び出しの数（状態の問い合わせを除く）」で、台詞の起動・表示・定常の知らせ・取りやめの記録を数えない＝件 2 の項目 1。
- **`LapRig::frames_until`**（`shell_balloon_switch_session_lap_tests.rs` 221 行〜・792 行のファイル）: `SwitchRig::progress_probe` をそのまま芯へ渡す。呼び手は `frames_until(` の 43 か所のうち lap 系の 25 か所（abort 13・lap 9・update 2・balloon 1）。
- **前例 `spine_conformance_lap_tests.rs` 270 行の `progress_probe`**: SHIORI の呼び出し＋台詞の起動の数＋表示指令の数。項目 1 の直しの手本。
- **`StageSink::progress_probe`**（`spine_conformance_support.rs` 438 行）: 既定は `None`＝`［進みは不明］`。上書きしているのは lap の足場だけ。`SpineHarness` で段を回す `spine_conformance_support_tests.rs` の檻は目印なし。
- **`manual_balloon_runtime`**（`frame_test_support.rs` 915 行）: MTA を張って `GraphicsCore::new()`（928 行）と `WucGraphicsResource::new` を許可なしで呼ぶ。返り値は `(World, Rc<RefCell<TextLayerRuntime>>)`。呼び手は `balloon_visibility_phase_box_tests.rs` 355 行・`frame/balloon_face_tests.rs` 66 行・`input_events/balloon_pressed_tests.rs` 68 行・`input_events/balloon_wheel_tests.rs` 57 行の 4 本（1 本目は返り値の頭を `_gpu` と名付けているが、中身は World）。
- **GPU の装置を作るファイル**（`GraphicsCore::new` を含む・areka の src）: `film_playback_e2e_tests.rs`・`frame_attach_tests.rs`・`frame_visibility_integration_tests.rs`・`spine.rs`・`balloon_choice_settle_tests.rs`・`dump_surface_gpu_test_support.rs`・`shell_balloon_switch_session_lap_tests.rs` は `GpuPermit` を持つ。持たないのは `frame_test_support.rs` だけ。

### 1.3 負荷の再現（`tools/load-flake.ps1`・297 行）

- 実行ファイルは `-p areka --bin areka` に固定（164 行の `$testArgs`）。`-Filter`・`-TestThreads`・`-Rounds`・`-Burners`・`-RoundTimeoutMin`・`-NoCapture` を持つ。回ごとの終了コードは `summary.txt` に数で残る（`0xc0000005` は `-1073741819`）。
- 置き場は `target\load-flake\<Label>-<時刻>\` だけ。負荷の子は番号で控えて止める（要件 9.10 は今の形で満たしている）。
- **足りないもの**: `cargo test -p wintf --lib`（件 3）を回す口が無い。

### 1.4 原因の前例（このリポジトリの記録）

| 記録 | 分かったこと | 本 spec への効き |
|---|---|---|
| `completed/wintf-gpu-test-crash/research.md`（根本原因の節） | 落ちる点は 2 つ目の `Compositor::new()`。引き金は「同じプロセスで別のスレッドに 2 つ目の MTA に縛られた `Compositor`」。同じスレッドで作り直すのは安全。直しは graphics の族を 1 本のオーナーのスレッドへ寄せること（`crates/wintf/tests/graphics/common/mod.rs` の `on_gpu_owner_thread`）と、スレッドに既にある `DispatcherQueue` の使い回し（`wuc_resource.rs` 87〜98 行）。areka の実行ファイルは「wintf の src の直しの波及だけで緑」と書かれた | areka の frame の族は今も別々のテストのスレッドで MTA の `Compositor` を作る。7 月の「決まって落ちる」形は消えたが、並びと負荷しだいで落ちる形が残っている、という読みと合う。**ただし areka の側では原因を確かめていない**（同記録の Phase 0 表の 2 行目は「未再実測」） |
| `crates/wintf/src/com/wuc.rs` 266〜274 行（テストの説明） | `--lib` の中で MTA/NONE の `Compositor` を作るテストは `wuc_graphics_resource_lifecycle` 1 本だけにしてあり、ほかは ASTA にして避けている | 件 3 の組（`begin_draw_roundtrip` は ASTA）は①の引き金に当たらない＝別の機序の疑い |
| `crates/wintf/src/ecs/widget/bitmap_source/wic_core.rs` 16〜45 行 | 並走のテストが立てた一時的な MTA が解体されると、COM の物の `Release` が解放済みのメモリに触れて `0xC0000005`。`CoIncrementMTAUsage` でプロセスの寿命の MTA を常に立てて直した | 件 3 の MTA のテスト（`alpha_mask_resource.rs` 197 行・`bitmap_source/tests.rs` 11 行）は `CoInitializeEx(MTA)`→`CoUninitialize` の対で MTA を立てては解く。areka にも同じ対がある（`placement/placement_shared_test_support.rs` 14 行・`placement/measure_tests.rs`・本番の `switch_assets.rs` 146〜158 行の背景のスレッド） |
| `completed/emo-atlas-palette-alpha`（`wic_tests.rs` の 8〜10 行） | MTA で書いた `wic_tests.rs` は件 3 の組で 10 回中 10 回落ち、STA にして避けた | 件 3 の確かな再現の材料（MTA の形はコミットに残っていないので、書き戻して使う） |
| `completed/areka-P0-ghost-session-test-load-flake/load-repro.md` 4.2・4.3・4.7〜4.9 | 負荷の下でおおむね 5 回に 1 回ほど落ちた。落ちる頃は `frame::transition_branch_tests`・`text_scale_tests`・`visibility_integration_tests`。スレッドのスタックを 20 秒ごとに採った回（4.2 の注意）でも落ち、どの道具を使ったかの記録は無い | 要件 1.3 の回数と上限の時間を決める材料 |

### 1.5 道具の有無（開発機）

- `cdb`／WinDbg: 無い（`C:\Program Files (x86)\Windows Kits\10\Debuggers\x64\` は `dbgcore.dll`・`dbghelp.dll`・`srcsrv.dll`・`symsrv.dll` だけ）。`wintf-gpu-test-crash` も同じ理由で切り分けの実験に頼った。
- `procdump`: 見当たらない（入れるには取り寄せが要る）。
- `[profile.dev] debug = "line-tables-only"`（ルートの `Cargo.toml`）なので、プロセスの中で採る `std::backtrace` はファイルと行まで出る。
- `rustc 1.99.1`。libtest はテストごとにテストの名前でスレッドを起こし、std はその名前を `SetThreadDescription` で OS にも渡す（設計の段で一度確かめる）。

---

## 2. 要件と資産の対応

| 要件 | 今ある資産 | ギャップ |
|---|---|---|
| 1.1 落ちたスレッドの名前と積み上げを残す | 無し | **Missing**: 例外の受け手（プロセスの中）・据える場所・`windows` の機能（`Win32_System_Diagnostics_Debug`。生きているスレッドの名前を全部並べるなら `Win32_System_Diagnostics_ToolHelp` も）／**Unknown**: 落ちるのがテストのスレッドでなくドライバや WUC の中のスレッドのとき、テストを名指せるか |
| 1.2 仕掛けを先に確かめる | 無し | **Missing**: わざと落とす較正の走り（ふだんの走行では回らない形が要る） |
| 1.3 条件を先に書き、frame の族に絞る | `load-flake.ps1 -Filter`・`-TestThreads`・`-RoundTimeoutMin` | **Constraint**: `-Filter` は libtest の名前の部分一致 1 つ＝「自分で COM と装置を開く 28 ファイル」を 1 度に絞るには `emo2_boot::frame` などの接頭辞で切るか、引数を並べる口が要る |
| 1.4 許可 1 つの対照 | `GPU_SLOTS` は `static`・`GpuSlots::new(4)` | **Missing**: コードを書き換えずに数を替える口（環境変数など）。無ければ対照の回だけ手元で書き換える |
| 1.5〜1.9 特定・直し・記録 | 前例（1.4 節） | **Unknown**: 原因そのもの |
| 2.1〜2.4 件 3 | `wuc.rs` の ASTA のテスト・MTA のテスト | **Missing**: `load-flake.ps1` に wintf の lib を回す口／**Constraint**: 要件 2.1 の `com::alpha_mask_resource` 系・`com::bitmap_source` 系は、実際は `ecs::layout::hit_test::tests_ex::alpha_mask_resource` と `ecs::widget::bitmap_source::tests`（libtest の部分一致の絞り込み `alpha_mask_resource bitmap_source` ならどちらも当たる） |
| 3.1〜3.3 UI の側を数える目印 | `SwitchRig::progress_probe`・lap の前例 | **Missing**: 台詞の起動・表示・定常の知らせ・取りやめの記録の数え／**Constraint**: `SwitchRig::progress_probe` を広げると、直に使う 5 ファイル（`frame_ghost_quit_switch_tests.rs`・`ghost_switch_rig_wait_tests.rs`・`ghost_session_switch_fallback_tests.rs`・`dump_surface_gpu_test_support.rs`・`shell_balloon_switch_session_lap_tests.rs`）と、`LapRig`・`GpuRig` を通す呼び手の全部へ波及（要件 3.3） |
| 4.1〜4.4 `join_bounded` | 芯・3 つの写し | **Missing**: 相手のスレッドの中の進みの数え（`desk_pick` は差し込む閉包、`fetch_url` は `FakeFetch` と `MakeFetch` の閉包で数えられる・spine の `ActorHandle` はアクターの処理の数を外から読む口が要る）／**Unknown**: 相手のスレッドが始まれない（ローダーの錠）とき、目印があっても「進み 0 回の `［止まった］`」になる＝許す赤の範囲の字面の外に残る（要件 4.2 の諮り） |
| 5.1〜5.4 短い掴みの檻 | 檻（`install_commit_tests.rs` 395 行）・本番 `rename_patiently`（`install.rs` 642 行） | **Missing**: 「移しの試みが掴みに当たった」ことを檻が知る口。本番の関数は試しの結果を外へ出さない |
| 6.1〜6.7 許可の待ち | `GpuSlots::take`・`try_take`・檻 16 | **Missing**: 上限・文言・時計の継ぎ目・2 つ取りの待ち合いの檻・`manual_balloon_runtime` の許可 |
| 7.1〜7.5 目印の無い待ち | 15 か所（下の 3.5）・`StageSink` の既定 `None` | **Missing**: 各所の目印。`ghost_session_restart_tests.rs` は作業プールの数えが無い（wintf の本番） |
| 8.1〜8.7 並べたときだけの赤 | 無し | **Unknown**: 件 4 の握り手・件 5 の赤の文・third scope の止まりの中身 |
| 9.1〜9.10 測り方 | `load-flake.ps1`・`tools/test-all.ps1`・机の道具 | **Constraint**: 全体の負荷 5 回で約 50 分。静かな机の基準は `test-roots-under-target` の着地の後の main で採る |
| 10.4 1,000 行 | — | **Constraint**: `frame_test_support.rs` 974・`spine_conformance_support_tests.rs` 959・`spine_conformance_lap_tests.rs` 914・`main.rs` 986 |

---

## 3. 件ごとの読み

### 3.1 件 1（areka の実行ファイルの落ち）

**原因の候補**（どれも確かめていない。設計の段で測る順を決める）

- **候補 A: 別々のスレッドで MTA の `Compositor` を並べて作る**（前例①）。areka の frame の族はテストのスレッドごとに `CoInitializeEx(MTA)`→`GraphicsCore::new`→`WucGraphicsResource::new`（`DQTAT_COM_NONE`）を行う。許可が 4 つなので、同時に最大 4 つの MTA の `Compositor` が別々のスレッドにある。`manual_balloon_runtime` は許可の外で 5 つ目を作りうる。**許可を 1 つにした対照（要件 1.4）は、この候補を直に試す**（許可の下では `Compositor` を作るテストが 1 つずつしか走らない。ただし前例①は「1 つ目を join した後でも別スレッドの 2 つ目で落ちた」と書くので、1 つずつでも消えるとは限らない）。
- **候補 B: 一時的な MTA の解体**（前例②）。areka のテストのスレッドは MTA に入ったまま `CoUninitialize` せずに終わる（`SwitchRig::new`・`manual_balloon_runtime` ほか）。スレッドが終わるとき COM は後始末をし、MTA の最後の 1 本ならそこで MTA が解体される。ゴーストのアクターのスレッドや WUC の中のスレッドがその後に COM の物を `Release` すると、前例②と同じ形で落ちうる。スレッドの終わりはローダーの錠の中で進むので、負荷で並びが変わるという観測とも合う。本番は UI のスレッドが MTA に居続けるので、この形はテストの土台に固有の見込み（要件 1.6）。
- **候補 C: ドライバや D3D の確かめの層のスレッドの終わりの競り合い**（`load-repro.md` 4.2 のスタックの読みで、ドライバの DLL `igd10umt64xe` のスレッドが多い）。直す手がテストの側に乏しい。
- **候補 D: 装置や WUC の物を作ったスレッドと別のスレッドで捨てる**（World を別のスレッドへ渡して落とすテストがあれば）。全数を当たっていない。

**名指しの仕掛け（要件 1.1・1.2）の案**

| 案 | 中身 | 良い点 | 弱い点 |
|---|---|---|---|
| ア. プロセスの中の最後の受け手 | `SetUnhandledExceptionFilter` で、捕まえられなかった `0xC0000005` だけを受け、落ちたスレッドの名前（`GetThreadDescription`）・落ちた番地とそのモジュール（`GetModuleHandleExW` の番地から）・`std::backtrace` を標準エラーへ書く。プロセスの全スレッドの名前を並べれば、そのとき走っていたテストの名前も分かる | 依存を足さない・管理者の操作が要らない・ふだんの走行でも効く（落ちた回をそのまま読める） | 落ちたスレッドの中でヒープや錠が壊れていると積み上げが採れないことがある（先に割り当ての無い書き方で名前と番地を書く）。デバッガが付いていると呼ばれない |
| イ. vectored の例外の受け手 | `AddVectoredExceptionHandler` で最初の段で見る | ドライバの中で握りつぶされる前に見える | 正しく処理される `0xC0000005`（システムの DLL の中の試し読みなど）や Rust の panic の例外も通る＝絞り込みが要る。ふだんの走行に雑音を出しうる |
| ウ. WER のローカルのダンプ | レジストリ `LocalDumps` に `areka-*.exe` を書く | 完全なダンプ | システムの設定を変える管理者の操作・ダンプを読むデバッガが無い |
| エ. `procdump -e` | 外の道具 | 確実 | 取り寄せと承認が要る・負荷の下で道具自身が揺さぶる（`load-repro.md` 4.2 の注意と同じ懸念） |
| オ. libtest の「始まった」の行 | `--format json`（`-Z unstable-options`）で、始まって終わっていないテストの組を出す | コードを足さない | 安定版では使えない（`RUSTC_BOOTSTRAP` が要る）・落ちたスレッドは分からない |

据える場所の案: (i) `#[used] #[link_section = ".CRT$XCU"]` の初期化子を `#[cfg(test)]` のモジュールに置く（依存なし・`main.rs` は触らない・libtest の `main` より前に走る）、(ii) `GpuPermit::take`・`SwitchRig::new` などの足場の入口で `Once` を通して据える（frame の族と切替の族は必ず通るが、ほかは漏れる）、(iii) wintf の lib と共有するため、葉のクレート（`log-capture-kit` と同じ dev 依存の形）へ置く。(iii) の `#[used]` の静的な物は、参照されない rlib の部品だとリンクで落ちうる＝明示の呼び出しが要る（**Research Needed**）。

較正（要件 1.2）: わざと `0xC0000005` を起こすテストを、ふだんの走行で回らない形（環境変数で開く・`#[ignore]` を `--ignored` で回す・別の実行ファイル）で用意し、出力に自分の名前が出ることを先に見る。要件 10.1 の「無視の印で赤を消さない」とは別の使い方だが、形は設計で決める。

### 3.2 件 3（wintf の lib の落ち）

- 組: `com::wuc::tests::begin_draw_roundtrip`（ASTA の `Compositor`・`GraphicsCore`）と、`with_com_initialized`（MTA を立てて解く）を使う `alpha_mask_resource` と `bitmap_source` のテスト。後者のうち `test_wic_core_creation` などは `WicCore::new` を通るので、そこから先はプロセスの寿命の MTA が立つ＝並び順で MTA が解体されるかが変わる。
- 前例①の引き金（2 つ目の MTA の `Compositor`）には当たらない。前例②（一時的な MTA の解体）か、ASTA の `Compositor` の中の物と MTA の出入りの重なりが疑われる（**件 1 と別の原因かもしれない**＝要件 2.3 の枝）。
- 確かな再現の材料: `emo-atlas-palette-alpha` の最初の `wic_tests.rs`（MTA で書いて 10 回中 10 回落ちた）。今のコミットには STA の形しか無い。
- 回す口: `load-flake.ps1` は areka に固定。件 3 は負荷なしでも 8 回で 2 回落ちたので、負荷の道具を広げずに `cargo test -p wintf --lib -- begin_draw_roundtrip alpha_mask_resource bitmap_source` を回数を決めて回す形でも要件 2.1 の「回数を決めた走行」は満たせる（名指しの仕掛けは wintf にも要る）。

### 3.3 件 2 の項目 1（UI の側を数えない目印）

- 待ちは `shell_balloon_switch_session_abort_tests.rs` 404 行（`frames_until(CREEP, …)`）と 423 行（`capture` の中の `frames_until(UNBOUNDED, …)`）。
- 直しの形は 2 つ: (a) `SwitchRig::progress_probe` そのものに UI の側の数えを足す（使う全部の足場が良くなるが、波及が広い・`SwitchRig` は台詞の起動や表示を見ない＝何を数えるかを足場に足す要がある）。(b) `LapRig` に自分の目印を持たせ、`SwitchRig` の数え＋`frames_until` の中で見たこと（表示の指令・見た目の変化・知らせの汲み出しの数）を足す（lap の前例 `spine_conformance_lap_tests.rs` 270 行と同じ形・波及は lap の 25 か所）。
- 取りやめの記録は `capture` の中の記録＝待ちの最中に数える口が `log-capture-kit` に有るかは確かめていない（**Research Needed**）。

### 3.4 件 2 の項目 2（`join_bounded`）

- spine: `ActorHandle` の body の終わりしか読めない。アクターの処理の数（受けた便りの数など）を外から読む口があれば目印にできる。`areka-actor` は本番のクレート。
- install の 2 本: 差し込む閉包の側（`desk_pick` の選ぶ画面の代わり・`fetch_url` の `MakeFetch` と `FakeFetch`）で「始まった・取得した・送った」を数えられる。テストのファイルだけで済む。
- ただしどの形でも、**相手のスレッドが始まれない間は進み 0 回**。負荷の下の主な赤の形（ローダーの錠でスレッドが始まれない）はここに当たるので、目印を良くしても `desk_pick`・`fetch_url` の素の World のテストの `［止まった］` は「許す赤の範囲の字面の外」に残りうる。要件 4.2 の諮りは、目印を良くした後でも要るかもしれない。
- 3 つを 1 つにまとめるか（`JoinHandle` と `ActorHandle` の両方を受ける形）か、写しのまま文の形だけ揃えるか（要件 4.4）。

### 3.5 件 2 の項目 3（短い掴みの檻）

- 今の檻は、別のスレッドが 200 ms 眠ってから掴みを放す。本番は 50 ms ごとに約 2 秒まで試し直す。負荷で放す側のスレッドの始まりが 2 秒より遅れると赤。
- 「当たったことの観測」に結ぶ案: (a) `install.rs` に `#[cfg(test)]` の継ぎ目（試しが 5・32 で拒まれたときに呼ぶ閉包、またはスレッドに 1 つの数え）を置き、檻が拒まれたのを見てから**同じスレッドの中で**掴みを放す＝実時間に頼らない（本番の振る舞いは変わらない・`#[cfg(test)]` は範囲に入る）。(b) 移しを別のスレッドで走らせ、テストのスレッドが数えを見てから放す（放す側の起動の遅れは消えるが、見る側が 2 秒以内に起きる前提が残る）。(c) OS のファイルの便り（oplock など）で試しを知る＝重い・依存の機能が要る。
- `areka-nar` の実行ファイルには GPU のドライバが載らないので、負荷の下でも赤の見込みは低い（brief の読み）。

### 3.6 件 2 の項目 4（許可の待ち）

- 単に時間で打ち切ると、GPU のテストが長く許可を持つ負荷の下で、新しい種類の赤（許可の待ちの打ち切り）が増えうる。芯の考え方に合わせるなら、**目印は「許可が返された回数」**（待ち合いで止まったときだけ増えない）で、`［止まった］` の形で打ち切れる。要件 6.2 の文言（待った秒数・持たれている数と上限）は芯の文の後ろに足す形か、許可の待ち専用の文か。
- 檻（要件 6.3）: `GpuSlots::new` は `const` で檻が自分の数え（上限 2）を作れる（檻 16 と同じ）。2 つ取るテストどうしの待ち合いは、偽の時計を差し替える継ぎ目（`take_with(now, pause)`）があれば実時間を待たずに確かめられる。
- 許可を 2 つ取るのは `frame_visibility_integration_tests.rs` 542 行の 1 本だけ（`spine_wait.rs` 446〜447 行の決まり）。
- `manual_balloon_runtime` の揃え（要件 6.4）: 返り値に許可を足す（組の先頭に置く決まり）と 4 本の呼び手が変わる。ファイルは 974 行なので先に分ける（要件 10.4）。
- 対照（要件 1.4）の数の切り替え: `GPU_SLOTS` は `static` の `const` 初期化。環境変数で替えるなら `LazyLock` へ替える（テストの土台だけの変更）。名前の付け方（本番の `AREKA_` の名前空間の決まりはテストの土台に当てるか）。

### 3.7 件 2 の項目 5（目印の無い 15 か所）

| ファイル | 行 | 待つもの | 目印の候補 |
|---|---|---|---|
| `emo2_boot/spine_boot_smoke_tests.rs` | 35・131 | 描画と台詞の再生 | 足場の受けた指令の数（third scope の `received` と同じ形）・SHIORI の `call_count()` |
| `emo2_boot/spine_close_wiring_tests.rs` | 91 | 同上 | 同上 |
| `emo2_boot/spine_conformance_support_tests.rs` | 470・635 | `non_status_calls().len() >= 5` | `call_count()`（そのまま数） |
| 同 | 696・816 | 段の始まり | 足場の数え（ファイルは 959 行＝先に分ける） |
| `emo2_boot/spine_hold_tests.rs` | 39 | `holding()` | `call_count()` |
| `emo2_boot/spine_seriko_loop_tests.rs` | 233 | seriko の繰り返し | 受けた指令の数 |
| `session_end_deadline_tests.rs` | 111・139・331 | `holding()` | `call_count()` |
| 同 | 354 | `unblock_calls()` の増え | `unblock_calls()` そのもの（数）か `call_count()` |
| `mcp/dump_surface.rs`（`#[cfg(test)]` の `WaitAnswer::wait_answer`） | 415 | 別のスレッドからの答え | 呼び手の足場の目印を渡す引数（呼び手は `check_script_tests.rs` 2 か所・`dump_surface_gpu_tests.rs` 6 か所） |
| `ghost_session_restart_tests.rs` | 74（`run_input_until`） | 作業プールの閉包 | **口が無い**（`WintfTaskPool` は wintf の本番のファイルで、閉包の数を出さない）。テストが差し込む見張りの閉包（`SentinelArrived`）を数えるか、理由つきの例外（要件 7.3） |

- 要件 7.1 の「目印の既定が無い段の駆動器で段を回す檻の待ち」: `StageSink::progress_probe` の既定 `None` を使う `SpineHarness` の檻（`spine_conformance_support_tests.rs`）。`SpineHarness` に目印を持たせれば既定に頼る所が無くなる。
- **数の外にある、同じく文言の無い待ち**（要件の 15 か所には入らない・参考）: `emo2_boot/frame/switch_tests.rs` 641 行の `Instant::now() + 5 秒` の自前の締切、`recv_timeout` の自前の上限が `crates/areka/src` のテストに十数か所（`film_playback_e2e_tests.rs` 290・`frame/drain_resnap_tests.rs` 111・`frame_scale_notice_integration_tests.rs` 254・`ghost_session_reload_tests.rs` 45・`ghost_session_runtime_tests.rs` 51・`mcp/check_script_tests.rs` 205・`mcp/get_expression_table_tests.rs` 729 など）、`run_bounded` の呼び手。打ち切ると「待ちの文言の無い赤」になり、steering では負荷の下でも許さない。要件 7 の字面（`［進みは不明］` を出しうる待ち）には当たらないので、範囲に入れるかは要件の討議で決める。

### 3.8 件 4・件 5・third scope

- **件 4**: `rename` で中身を移した直後の `balloon` を `remove_dir` する所。握り手の候補は、ウイルス対策の読み・検索のインデクサー・同じプロセスの中で検体の木を数え上げる誰か（`sample-ghost-kit` の掃除の仕組み `devroot_sweep` など）。どのエラー（5・32・145）かが記録に無い。直しの案: (a) 移しを `balloon` のフォルダごとの 1 回の `rename` にして `remove_dir` を無くす（根の `balloon` を先に作らない形・ただし同じ握りで `rename` も拒まれうる）、(b) `sample-ghost-kit` に「同梱のバルーンを根へ出した形」の検体を組ませる、(c) 拒まれたら待ちの部品で目印つきに試し直す（一定の sleep を足すのでなく、要件 10.1 に当たらない形が要る）。要件 8.2 の見立ての根拠は、エラーの番号と、そのときの他のスレッドの名前（3.1 の仕掛けとは別に `remove_dir` の失敗の文に足す）で採れる。
- **件 5**: 2 本とも `SwitchRig::new` を通る（`main_session_mark_tests.rs` 285 行の `rig_with`・`main_session_mark_write_tests.rs` 199 行）。記録の捕まえ（`log-capture-kit` の `capture` は呼んだスレッドだけを捕まえる）と件 4 の足場が重なる。赤の文が残っていないので、要件 8.3 で採り直すまで原因は読めない。
- **third scope**（`spine_third_scope_talk_tests.rs` 115〜141 行）: 目印は SHIORI の呼び出し＋受けた指令と知らせの数（進み 10 回で 30 秒止まった）。足場は `SpineHarness`（`spine.rs` 445 行で許可を取る GPU の足場）なので、文の字面は許す赤に当たる。「相手の処理の取り違えが無いか」は、止まったときにどの指令まで届いていたか（`seen`）を文に足すと読み分けやすい（今は件数だけ）。

---

## 4. 実装の方向

### 案 A: 今の部品と足場を広げる

- 待ちは全部 `spine_wait.rs` の芯へ寄せ、許可の待ちも同じファイルに足す。名指しの仕掛けは areka の `#[cfg(test)]` のモジュールに置き、wintf の lib には同じものを wintf の `#[cfg(test)]` に写す。
- 良い点: 新しいファイル・クレートが少ない。既にある決まり（目印・文言の読み分け）をそのまま使う。
- 弱い点: 名指しの仕掛けが 2 か所に写しになる。`spine_wait.rs`（462 行）が太る。上限に近いファイルを分ける手間は変わらない。

### 案 B: 新しい部品を立てる

- 名指しの仕掛けを葉のクレート（dev 依存だけ・`log-capture-kit` と同じ扱い）に置き、areka と wintf（と areka-nar）の実行ファイルの両方で据える。許可の待ちは `GpuSlots` を独立のファイルへ出す。
- 良い点: 名指しの仕掛けが 1 つ。以後のテストの実行ファイルにも据えられる。
- 弱い点: ワークスペースのクレートが 1 つ増える（「依存の追加」に数えるかは要件の Boundary の読み次第）。rlib の `#[used]` が落ちる問題の手当てが要る。

### 案 C: 組み合わせ（段に分ける）

- 段 1: 名指しの仕掛けを areka の `#[cfg(test)]` に置いて較正し、frame の族と許可 1 つの対照で落ちを測る（件 1 の原因の当たりを付ける）。件 3 は同じ仕掛けを wintf に写すか葉のクレートへ移すかを、段 1 の結果（同じ原因か）で決める。
- 段 2: 原因に応じて直す（候補 A ならオーナーのスレッドへ寄せるか許可の中で WUC を 1 本ずつにする・候補 B ならプロセスの寿命の MTA をテストの土台で立てる・本番の欠陥ならテストを添えて直す）。
- 段 3: 件 2 の待ちを揃える（許可の上限は段 2 の後・要件 6.6）。件 4・件 5 は段 1 の測りの中で見る。
- 段 4: 静かな机の全体と負荷つき 5 回を 1 度だけ。
- 良い点: brief の順（件 1 → 件 2）と要件 6.5・6.6 の前後と合う。仕掛けの置き場を、測ってから決められる。
- 弱い点: 段 1 の結果で段 2 の中身が大きく変わる（タスクの見立てがぶれる）。

---

## 5. 規模と危うさ

- **規模: L（1〜2 週・13〜20 タスク）**。件 2 だけなら M（既にある部品の形へ移すだけ）。件 1 の測り（較正・対照・切り分けの実験）と、上限に近い 3〜4 ファイルの分割が足される。
- **危うさ: 高**。原因が分かっていない落ちで、前例の 2 つのどちらとも確かめていない。候補 A なら areka の GPU の族の土台（オーナーのスレッドへ寄せる）が大きく変わり、所要時間の線（要件 9.9・1.20 倍）に当たりうる（オーナーのスレッド 1 本へ寄せると GPU のテストが 1 本ずつになる＝静かな机で延びる見込み）。候補 B なら直しは小さい。

---

## 6. 設計の段への申し送り

### 決めること（設計の判断）

1. 名指しの仕掛けの形（最後の受け手 `SetUnhandledExceptionFilter`／vectored の受け手／両方）と、書く中身（落ちたスレッドの名前・番地とモジュール・積み上げ・生きているスレッドの名前の一覧）。
2. 仕掛けの置き場（areka の `#[cfg(test)]`／足場の入口の `Once`／葉のクレート）と、wintf の lib への据え方。`windows` の機能（`Win32_System_Diagnostics_Debug`・`Win32_System_Diagnostics_ToolHelp`）を areka と wintf の自分の `Cargo.toml` で足すか（areka は `Win32_UI_Controls_Dialogs` を自分の側で足した前例がある）。これを「依存の追加」に数えるか。
3. 較正（わざと落とすテスト）の回し方（環境変数で開く・`--ignored`・別の実行ファイル）。
4. 許可 1 つの対照の切り替え方（環境変数で `GPU_SLOTS` を替える口を残す／対照の回だけ手元で書き換えてコミットしない）。
5. 件 1 の候補 A〜D を試す順と、1 回の上限の時間・回数（`load-repro.md` の落ちの頻度はおおむね 5 回に 1 回）。
6. 項目 1 の目印を `SwitchRig::progress_probe` に足すか、`LapRig` だけに持たせるか。
7. `join_bounded` の 3 つを 1 つにまとめるか、写しのまま文の形だけ揃えるか。install の 2 本の目印を閉包の側で数える形にするか。目印を良くしても残る「始まれない間の進み 0 回」を、要件 4.2 の諮りへ回すか。
8. 短い掴みの檻の「当たった」の観測の口（`install.rs` の `#[cfg(test)]` の継ぎ目で同じスレッドの中で放す／別のスレッドで見て放す）。
9. 許可の待ちの上限の決め方（総時間／「許可が返された回数」を目印にした `［止まった］`）と文言の形。
10. `ghost_session_restart_tests.rs` の作業プールの待ちを、見張りの閉包の数えで目印つきにするか、理由つきの例外にするか。
11. 件 4 の直し方（`remove_dir` を無くす組み方／`sample-ghost-kit` 側の形／目印つきの試し直し）。
12. 上限に近いファイル（`frame_test_support.rs`・`spine_conformance_support_tests.rs`・`spine_conformance_lap_tests.rs`）の分け方。
13. 要件 7 の 15 か所の外にある文言の無い待ち（自前の `recv_timeout`・`Instant` の締切・`run_bounded`）を範囲に入れるか（要件の字面の外）。

### 調べること（Research Needed）

- libtest がテストごとのスレッドに名前を付け、std がそれを `SetThreadDescription` で渡すこと（`rustc 1.99.1`・`--test-threads=1` のときも）。
- 捕まえられなかった例外の受け手の中で `std::backtrace` を採るときの安全さ（ヒープの壊れ・dbghelp の錠）。先に割り当ての無い書き方で 1 行書く手順。
- rlib に置いた `#[used] #[link_section = ".CRT$XCU"]` が、参照の無いときにリンクで残るか。
- 件 3 の MTA のテストの `CoUninitialize` と ASTA の `Compositor` の重なりで、何が解放されるのか（前例②の機序が ASTA にも当たるか）。
- areka のテストのスレッドが `CoUninitialize` せずに終わるときに MTA が解体される条件（`CoIncrementMTAUsage` で立てた MTA が areka の実行ファイルの中で既にあるか＝`WicCore::new` を通るテストの有無）。
- `log-capture-kit` の `capture` の中で、待ちの最中に記録の件数を数える口があるか（項目 1 の取りやめの記録）。
- 件 4 の `remove_dir` のエラーの番号（5・32・145）と、そのときに同じプロセスで検体の木に触れているスレッド。

### 要件の字面について気づいたこと（要件の討議の材料・要件は書き換えていない）

- 要件 2.1 の `com::alpha_mask_resource` 系・`com::bitmap_source` 系は、実際のモジュールの道が `ecs::layout::hit_test::tests_ex::alpha_mask_resource`・`ecs::widget::bitmap_source::tests`（brief のコマンドの部分一致では当たる）。
- 件 5 の 2 本は件 4 と同じ `SwitchRig::new` を通る＝同じ切り分けの中で一緒に見られる。
- `balloon_visibility_phase_box_tests.rs` 355 行は `manual_balloon_runtime` の返り値の頭を `_gpu` と名付けているが、中身は World（許可ではない）。要件 6.4 の揃えのときに名前も合う。
- 要件 7 の数（15 か所）の外にも、打ち切ると「待ちの文言の無い赤」になる自前の締切が残る（3.7 の末尾）。
