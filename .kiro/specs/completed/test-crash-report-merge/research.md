# ギャップ分析: test-crash-report-merge

- 測った日: 2026-11-21・ワークツリー `claude/kiro-start-test-crash-report-merge-876b91`（`main` `f6405e09` の上に spec の初期化のコミット 1 つ）
- 入力: `requirements.md`（未承認）・`brief.md`・steering（`product.md`・`tech.md`・`structure.md`）
- やり方: Grep／Read で今のコードを測った。外の資料は引いていない（cargo の機能の統合と `[dev-dependencies]` の見え方は今の `Cargo.toml` と見張りのテストから読める範囲で書いた）

## 1. 要約

- 3 本の写しは、説明の数行を除けば同じ本文である。wintf と areka-placement の差は説明の 8 行だけ（`diff`）。areka-runtime の 1 本はそれに、許可の持ち手の表（169〜229 行）・報告の中の持ち手の行（329〜352 行）・`pub fn installed`（139 行）・`pub extern "C" fn install`（147 行）・較正の 2 行（578〜579 行）を足し、`.CRT$XCU` の静的変数と `RaiseException` の宣言を `#[cfg(test)]` で囲った形である。
- 使い手の入口の形はもうある。`areka-input`（`crash_report_tests.rs` 5・10 行）と本体（`runtime_tests/mod.rs` 12・17 行）は、自分の根の crate に `.CRT$XCU` の静的変数を置き、中身は `areka_runtime::crash_report_test_support::install` を指す。wintf と areka-placement もこの形に寄せられる。
- **一番大きい壁は、要件の仮定 A1（置き場 ⑴＝`log-capture-kit`）と A5（`areka-input` に触らない）を両方取ると、ワークスペースの見張りが赤になることである。** `areka-input` と本体は `areka-runtime` の `test-support` の機能を通って入口へ届く（`#[cfg(test)]` ではない）。そのため `areka-runtime` は本体の crate を `[dependencies]`（任意・`test-support` のときだけ）に持たなければならない。その crate が `log-capture-kit` なら、`crates/log-capture-kit/tests/with_default_guard_test.rs:433` の `the_shared_crate_never_appears_in_a_production_dependency_table` が赤になる（任意の依存も製品側の表に数える・例外表は無い）。⑵（新しい leaf の `*-kit`）ならこの見張りに掛からず、前例（`areka-runtime/Cargo.toml:14` の `shiori-host32-host` を `test-support` のときだけ付ける形）にそのまま乗れる。
- 逆に ⑵ では、要件 1.4 の「`THIRD-PARTY-NOTICES.md` が変わらない」と要件 7.3 の「`Cargo.lock` の差分は置き場の crate の行だけ」が成り立たない（2.4 節）。要件 1 は「⑴ と ⑵ のどちらでも満たせる形」と書いてあるが、実際には ⑴ は見張りと、⑵ は 1.4・7.3 と食い違う。
- 申し送りのカ（`window_move_lines` を `areka-placement` の `test-support` へ移す）にも同じ形の壁がある。関数は `log_capture_kit::CapturedEvent` を受けるが、`log-capture-kit` は `areka-placement` の dev-dependency なので、`test-support` の機能だけで組んだ placement からは見えない。

## 2. 今の姿（根拠つき）

### 2.1 3 本の写し

| ファイル | 行数 | 宣言 | 持ち手の表 | `.CRT$XCU` |
|---|---|---|---|---|
| `crates/wintf/src/test_crash_report.rs` | 487 | `lib.rs:6-7` `#[cfg(test)] mod test_crash_report;` | 無い | mod ごと `cfg(test)` |
| `crates/areka-placement/src/test_crash_report.rs` | 490 | `lib.rs:85-86` `#[cfg(test)] mod test_crash_report;` | 無い | mod ごと `cfg(test)` |
| `crates/areka-runtime/src/crash_report_test_support.rs` | 587 | `lib.rs:69-70` `#[cfg(any(test, feature = "test-support"))] pub mod` | 在る（169〜229） | 静的変数だけ `#[cfg(test)]`（127〜130） |

- wintf と placement の差（`diff`）は 1・9・13・39・140・259 行あたりの説明だけ。
- runtime の 14 行目「`//! 走行で候補 B と決めた）。`」は前の文が消えて残った切れ端（開き括弧が無い）。まとめた本体へ写さないこと。
- 3 本とも `windows` の `Win32::Foundation`・`System::Com`（`CoIncrementMTAUsage`・`CoGetApartmentType`）・`LibraryLoader`・`Threading` と、`kernel32` を手で宣言した 6 つの関数（`SetUnhandledExceptionFilter`・`AddVectoredExceptionHandler`・`RaiseException`・`CreateToolhelp32Snapshot`・`Thread32First/Next`・`VirtualQuery`）を使う。ワークスペースの `windows` の機能（`Cargo.toml:49-78`）で足りる（placement は `windows = { workspace = true }` のままでこの写しを組めている）。

### 2.2 使い手

| 使い手 | 何を使うか | 根拠 |
|---|---|---|
| `areka-runtime` 自身のテスト | 本文と `#[cfg(test)]` の静的変数 | `crash_report_test_support.rs:127-130` |
| `areka-runtime` の `spine_wait.rs` | `hold`・`release` | `spine_wait.rs:24`（`use`）・683（説明）・692・704 |
| `areka-runtime` の `wire_tests.rs:22`・`assets_tests.rs:10`・`assets_shell_tests.rs:3`・`assets_markers_tests.rs:8` | 説明の中の名 `crash_report_test_support` だけ | grep |
| `areka-input` | `install`・`installed` | `crash_report_tests.rs:5・10`・`lib.rs:609-610`・dev-dep `areka-runtime = { features = ["test-support"] }`（`Cargo.toml:47`） |
| 本体 `areka` | `install`・`installed` | `runtime_tests/mod.rs:12・17`・dev-dep `areka-runtime = { features = ["test-support"] }`（`Cargo.toml:105`） |
| windows-rs の数え | 名が `test_*`・`*_test_support`、crate 名が `-kit` ならテストの道具 | `q9-api-count/src/count.rs:68-81`（`is_test_path`）・`85-86`（`is_test_tool`）。`count_tests.rs:62` は文字列 `crates/wintf/src/test_crash_report.rs` を判定の例に使うだけ（ファイルが無くなっても赤にならない） |
| `tools/load-flake.ps1` | `AREKA-CRASH` を含む行を字面で拾う | 193 行・474・484 行 |

`areka-input` と本体は `areka-runtime` を `test-support` の機能つきで引く。`areka-runtime` の `crash_report_test_support` はその機能で開く（`#[cfg(test)]` ではない）。この 2 つの crate の静的変数は、`areka-runtime` を **普通の依存として組んだ形** で入口を呼ぶ。だから入口の後ろに在る本文は、`areka-runtime` の `[dependencies]` から届かなければならない（dev-dependency は依存先として組まれるときには見えない）。

### 2.3 置き場の候補の今

- `log-capture-kit`（`crates/log-capture-kit/Cargo.toml`）: 本番の依存は `tracing` と任意の `tracing-subscriber`（`env-filter`）だけ。dev-dependency に `sample-ghost-kit`（ワークスペースの crate）を持つ。使い手 22 crate はどれも `[dev-dependencies]` から引く（各 `Cargo.toml` の節を awk で確かめた）。
- 見張り（`crates/log-capture-kit/tests/with_default_guard_test.rs`）:
  - ⑶ `the_shared_crate_never_appears_in_a_production_dependency_table`（433 行）＝`log-capture-kit` が `[dependencies]`・`[build-dependencies]`・`[target.*.dependencies]` に 1 行でも現れたら赤。任意の依存も数える。例外表は無い。
  - ⑷ `only_one_crate_declares_the_env_filter_feature`（477 行）＝`env-filter` を宣言してよいのは wintf だけ。新しい機能の名（例 `crash-report`）にはこの縛りは掛からない。
- 文書の縛り: `structure.md:452`「`[dependencies]` へは決して置かない（製品側依存に現れたら `with_default_guard_test.rs` が赤にする）」・`log-capture-kit/src/lib.rs:12`「`[dev-dependencies]` からのみ引く」・使い手の `Cargo.toml` の注（`areka-runtime/Cargo.toml:43` ほか約 10 か所）「依存方向の規律により [dependencies] へは決して置かない」。
- 前例（任意の依存を `test-support` のときだけ付ける）: `areka-runtime/Cargo.toml:14` `test-support = ["dep:shiori-host32-host", "areka-placement/test-support"]`・35 行 `shiori-host32-host = { optional = true }`。同じ crate を dev-dependency にも並べて持つ（49 行あたり）。

### 2.4 `Cargo.lock` と `THIRD-PARTY-NOTICES.md`

- 今の `Cargo.lock` の `log-capture-kit` の項（1505〜1511 行）の `dependencies` は `sample-ghost-kit`・`tracing`・`tracing-subscriber`。`Cargo.lock` は任意の依存も dev-dependency も項に書く。
- wintf・areka-placement・areka-runtime の項は、どれももう `log-capture-kit` を 1 行持つ（dev-dependency の分）。
- `windows` は 0.62.2 の 1 つだけ（`Cargo.lock:2648`）。
- `THIRD-PARTY-NOTICES.md` は `cargo about generate --workspace`（`tools/test-all.ps1:93`・`-License` のとき）で作る。`about.toml` の `ignore-dev-dependencies = true` は外の dev-dependency を外すだけで、**ワークスペースの全メンバーは載る**（`THIRD-PARTY-NOTICES.md:3041-3077` の「MIT License」の対象 crate に `log-capture-kit 0.1.0`・`temp-path-kit 0.1.0`・`real-run-kit 0.1.0` ほか全メンバーが並ぶ）。

| 置き場 | `Cargo.lock` の差分 | `THIRD-PARTY-NOTICES.md` |
|---|---|---|
| ⑴ `log-capture-kit` に `windows = { workspace = true, optional = true }` | `log-capture-kit` の項の `dependencies` に `"windows",` が 1 行増えるだけ（wintf・placement・runtime の項は既に `log-capture-kit` を持つので変わらない） | 変わらない |
| ⑵ 新しい leaf `xxx-kit` | 新しい `[[package]]` の塊 1 つ＋wintf・areka-placement・areka-runtime の項の `dependencies` に 1 行ずつ | 「MIT License」の対象 crate に 1 行増える |

### 2.5 機能の統合（本番のビルドへ `windows` が漏れないか）

- ワークスペースは `resolver = "2"`（`Cargo.toml:5`）。dev-dependency の上で付けた機能は、テストの的を組まないビルド（`cargo build -p areka` など）には統合されない。`log-capture-kit` はどの crate からも dev-dependency で引かれるので、本番のビルドでは組まれもしない。⑴ でも本番のビルドに `windows` が新しく入る道は無い（そもそも本番の crate は全部 `windows` を既に持つ）。
- ただし `cargo test --workspace`（`tools/test-all.ps1:75`）では、wintf ほかが付けた `crash-report` の機能がワークスペースの全部へ統合され、`log-capture-kit` は 1 度だけ機能つきで組まれる。落ちの報告を使わない 19 crate のテストの実行ファイルにも、その本文の rlib がつながる。`.CRT$XCU` の静的変数をそれぞれの根の crate に置く形なら、据える処理は走らない（rlib の中の使われないオブジェクトはリンカが拾わない＝今の本文の 126 行の注「根のクレートの中に置くので、リンクで落ちない」と同じ理屈）。要件 1.3 は crate ごとのビルド（`cargo test -p areka-parsers`）では満たすが、`--workspace` の統合では字面どおりには満たさない（振る舞いは変わらない・`windows` はどの道そのビルドで組まれている）。`env-filter` と同じ性質で、`with_default_guard_test.rs:11-14` の注がその統合を承知している前例がある。
- 逆向きの注意: `cargo test -p log-capture-kit` だけを回すと、機能が付かず、まとめた本体のテスト（要件 5.2〜5.4）が **黙って組まれない**。`--workspace` では統合で付くので走る。dola の `toml` の機能のように別の段を足すか（`tools/test-all.ps1:76`）、`[[test]]` の `required-features` を使うか、機能に頼らない置き場（⑵）にするかは調べが要る（Research Needed R1）。

### 2.6 許可の持ち手の表と較正（共通にできる所・残る所）

| 部分 | 行（runtime） | 共通の本体へ移せるか | 理由 |
|---|---|---|---|
| 手で宣言した構造体と kernel32 の関数・大きさの assert | 43〜120 | 移せる | どの写しも同じ。`RaiseException` だけは今 `#[cfg(test)]`（110 行）なので、較正を残す crate の側でも宣言が要る（下） |
| `.CRT$XCU` の静的変数 | 127〜130 | **移せない（各テストの実行ファイルの根の crate に 1 つずつ）** | 上流の rlib の `#[used]` は、参照されないとリンクに入らない。今の areka-input と本体の形がこれ |
| `PREVIOUS`・`INSTALLED`・`WRITING`・`install` の本文・`installed` | 133〜160 | 移せる | `pub` にして使い手が呼ぶ |
| 許可の持ち手の表（`HOLDERS`・`MISSED`・`hold`・`release`） | 169〜229 | どちらでもよい | 下の 2 案 |
| 報告の中の持ち手の行 | 329〜352 | 表と同じ側 | 表を読む |
| 受け手・報告の本文・`Line` | 235〜509 | 移せる | 同じ |
| 据え付けの確かめ `crash_filter_is_installed_before_the_tests_start` | 515〜522 | **各テストの実行ファイルに 1 本ずつ**（要件 5.1） | 据わったかは実行ファイルごとの事実 |
| MTA・最初の受け手・`Line` の檻 | 524〜570 | 本体の crate のテストへ 1 回だけ | 本体の crate のテストの実行ファイルにも静的変数が要る（MTA の檻は据わった後でないと緑にならない） |
| 較正（装置なし） | wintf・placement の 475〜486 | 本体の crate か、wintf・placement のどれか 1 か所 | 装置を作らない |
| 較正（装置あり） | 572〜587 | **areka-runtime に残る** | `crate::spine::GpuPermit::take()`（runtime の私有）と `wintf::ecs::GraphicsCore::new()` を使う。`log-capture-kit`／新しい leaf は wintf にも runtime にも依存できない |

持ち手の表の 2 案:

- **案 H1（差し込む口）**: 共通の本体に「据え付けの後、持ち手の行を書く関数を 1 つ登録できる口」を足す（例: 関数ポインタを `OnceLock`／原子変数に置く）。`areka-runtime` の `install` は口に自分の書き手を登録してから本体の `install` を呼ぶ。wintf と placement は本体の `install` を直に呼ぶので、口は空のまま＝`AREKA-CRASH-HOLDER` は 1 行も出ない。表・`hold`・`release` は `areka-runtime` に残る（要件 3.3 を字面どおり満たす）。口は割り当て無しで 1 行の溜め（`Line`）に書ける形が要るので、`Line` か書き込みの小さな口を本体の公開面に出すことになる。
- **案 H2（表も本体へ）**: 表・`hold`・`release` も共通の本体に置き、`areka-runtime` は `pub use` で同じ名を開くだけにする。wintf と placement は `hold` を呼ばないので表は空・錠はいつも取れる・`MISSED` は 0＝`AREKA-CRASH-HOLDER` の行は今と同じく出ない（要件 3.2 は満たす）。ただし要件 3.3「wintf と areka-placement は表を持たない」は字面では満たさない（使わない表がリンクされる）。差し込む口が要らない分、本文は短い。

### 2.7 `areka-input` を触らずに済むか（要件 4.2・7.2・仮定 A5）

- 入口の名（`areka_runtime::crash_report_test_support::install`・`installed`）と、`areka-runtime` の `test-support` の機能でそれが開くことを保てば、`crates/areka-input/` の差分は 0 にできる。`areka-input` の `Cargo.toml` は `areka-runtime` を `test-support` つきで引いているだけなので、`areka-runtime` の側で本体の crate を `test-support` の任意の依存として足せば、`areka-input` の `Cargo.toml` も変わらない。
- そのためには `areka-runtime/Cargo.toml` の `[dependencies]` に本体の crate を任意で置く必要がある（2.2 節の理由）。⑴ ではこれが 2.3 節の見張り ⑶ に当たる。
- 本体の `runtime_tests/mod.rs` も同じ理由で変わらない（仮定 A8 のとおり）。

### 2.8 申し送り（`audit-emo2-boot-frame` から）の測り

- **ウ（要件 6.1）**: `follow_window_move_hold_watch_tests.rs:31` の `INVARIANT_WARNING` は手写しで、使う 3 か所（135・212・241 行）はどれも「0 件」を言うだけ。本番の文は `follow/window_move.rs:649`。文が変わっても 3 本は緑のまま（空虚）。在ることを数える形の前例が `crates/areka-runtime/src/frame_work_area_resnap_hold_tests.rs:372-410` にある（`#[should_panic(expected = "DpiSyncHold")]`・`#[cfg_attr(not(debug_assertions), ignore)]`・`catch_unwind` の中で数えてから panic を戻す）。placement の側にも同じ形の 1 本を足せば満たせる。
- **エ（要件 6.2・仮定 A6）**: 古い道 `runtime::…`（`areka_runtime::` でないもの）は `areka-placement` に **44 行・21 ファイル**。うち `runtime::frame` は **25 行・14 ファイル**。名指された 3 ファイル 6 行（`config.rs:107・109`・`follow/work_area.rs:27・231`・`follow_visibility_balloon_wiring_tests.rs:812・816`）の外の `runtime::frame` は 19 行・11 ファイル:
  - `balloon_offset_persist_roundtrip_tests.rs:187`
  - `chain_finalize.rs:31・38・69・511`
  - `chain_realign.rs:160・167`（167 は要件 6.6 の範囲と重なる）
  - `diag.rs:187・211`
  - `follow/move_shift.rs:6`
  - `follow/window_move.rs:34・1105`
  - `follow_transition_diag_tests.rs:13`
  - `lib.rs:114・138・187`
  - `spawn.rs:350`
  - `transition_diag.rs:34`
  - `zorder_chain_compose.rs:193`
  - `runtime::frame` でない古い道（19 行）: `examples/window-placement.rs:332`・`config.rs:106`（`runtime::wire_runtime`）・`diag.rs:186`・`follow/offset_space.rs:47`・`follow/window_move.rs:33・916`・`follow/work_area_clamp.rs:17`・`follow.rs:84・101・105・121・131・133・138・154`・`follow_balloon_limit_tests.rs:362`・`lib.rs:181`・`measure.rs:360`・`measure_template_tests.rs:103`。
  - 「本体の」は `areka-placement` に 65 行・38 ファイルあり、本当に本体（`crates/areka/`）を指すものも混じる。古い道と並ぶ「本体の `runtime::…`」は `diag.rs:186・187`・`follow_transition_diag_tests.rs:13`・`follow/window_move.rs:33`・`zorder_chain_compose.rs:193` と名指しの 812・816。
  - 細かい点: `config.rs:107` の `runtime::frame::wiring::seed_zorder_descript_base` は、今は `RuntimeWiring` のメソッド（`areka-runtime/src/frame/wiring.rs:133`・191 `pub(crate) fn`）。道をそのまま `areka_runtime::frame::wiring::seed_zorder_descript_base` に置き換えると、在る項目を指さない。106 行の `ghost_session.rs`・`runtime::wire_runtime` も同じ段落の古い道（`ghost_session.rs` は本体の `crates/areka/src/`・`wire_runtime` は `areka_runtime::wire_runtime`）。
- **オ（要件 6.3）**: `areka-placement/src/test_support.rs:423` の `zorder_pair_deferred::code_only` と `areka-runtime/src/source_text_test_support.rs:4` の `code_only` は同じ本文（`//` で始まる行を落とす）。依存の向きは runtime → placement なので、定義を placement に 1 つ置き、runtime の `source_text_test_support` が `pub use areka_placement::test_support::zorder_pair_deferred::code_only;` で開く形が取れる。runtime は `test-support` でも自分のテストでも placement の `test-support` を付けている（`areka-runtime/Cargo.toml:14`・dev-dep の `areka-placement = { features = ["test-support"] }`）。今の呼び手の道（runtime 7 ファイル・本体の `runtime_tests/zorder_wiring_tests.rs:17` ほか・`audit-areka-app` の寄せ先 `areka_runtime::source_text_test_support::code_only`）はどれも変わらない。`test_support.rs:281` の `measure_template::code_only` は別の働き（ブロックコメントと文字列も落とす）で範囲の外（要件の Boundary のとおり）。
- **カ（要件 6.4・仮定 A4）**: `follow_window_move_diag_tests.rs:170` と `areka-runtime/src/frame_test_support.rs:885-892`（`#[cfg(test)]`）は同じ本文。ただし引数が `&[log_capture_kit::CapturedEvent]` で、`log-capture-kit` は placement の dev-dependency。placement の `test_support` は `#[cfg(any(test, feature = "test-support"))]`（`lib.rs:89-90`）で、runtime から `test-support` で組まれるときは `log_capture_kit` が見えない。今の形の署名のまま `test_support` に `pub` で置くと、runtime のテストのビルドで組めない。`#[cfg(test)]` を付ければ組めるが、runtime からは引けない。引数を文字列の並び（`CapturedEvent::message()` の結果）にする形なら依存は要らない。いずれにしても runtime の写しは要件 7.1 で付け替えないので、この spec の中で「runtime からも引ける」を確かめる使い手は無い。
- **移したテストの置き場（要件 6.5）**: `chain_rule_tests.rs:5` は「本体の `frame_chain_rule_tests.rs`」。実物は `crates/areka-runtime/src/frame_chain_rule_tests.rs`（在る）。
- **武装の説明（要件 6.6）**: `chain_realign.rs:167-169` の「寸変化を伴って書込を起こしたときに呼ぶ。判断（寸が変わったか・書込が起きたか）は呼び手が持つ」。今の呼び手は `areka-runtime/src/frame/dpi.rs:408-415`＝「キャラ窓で、前の寸が在り、前後の寸が違うとき」だけを見る（書込の有無は見ない）。同じ段落の 160 行にも古い道 `runtime::frame::drain_resnap::collect_chain_states` がある（実物は `areka-runtime/src/frame/drain_resnap.rs:617` の私有の関数）。

### 2.9 `carrier_cue` の残り（範囲の外の確かめ）

- 写しは 3 つ: `areka-ghost/src/prop_sink.rs:118`（テストの中の私有・引数が `ActorKey` の元の `&str`）・`areka-seriko/src/actor_test_support.rs:124`（`pub(super)`・同じ crate の `cue(...)` を使う）・`areka-runtime/src/consumer_ledger_test_support.rs:126`（引数 2 つ・形が違う）。
- どれも `dola` の `TalkCue`・`CueCommand::command_carrier` を組む。`log-capture-kit` も新しい leaf も `dola` に依存しないので、落ちの報告の置き場へは寄せられない（要件の Boundary・仮定 A3 のとおり）。寄せるなら `dola` 自身にテスト用の組み立てを置く別の話になる（`areka-ghost` は `dola` を dev-dependency で・`areka-seriko` は本番の依存で持つ＝向きは合う）。

## 3. 要件と今の資産の対応

| 要件 | 今ある物 | 足りない物・食い違い | 種別 |
|---|---|---|---|
| 1.1 本体を 1 か所 | 3 本の写し | 置き場の crate と公開の入口（`install`・`installed`） | Missing |
| 1.2 ワークスペースの crate に依存しない | `log-capture-kit` は本番の依存 0 | `log-capture-kit` は dev-dependency に `sample-ghost-kit` を持つ（字面の「どの crate にも」と合わない） | Constraint |
| 1.3 使わない使い手に Win32 の依存を引かせない | — | ⑴ は `--workspace` の統合で全使い手に付く（2.5） | Constraint |
| 1.4 外の crate を増やさない・NOTICES が変わらない | — | ⑵ では NOTICES に 1 行増える（2.4） | Constraint |
| 1.5 数えのテストの道具 | `-kit` の crate・`*_test_support` の名 | runtime に残る薄い殻は `crash_report_test_support.rs` の名のままなら数えの外 | — |
| 1.6 1,000 行 | 本文は 1 本 487〜587 行 | — | — |
| 1.7 本番のビルドに現れない | 前例 `shiori-host32-host` の任意の依存 | ⑴ は見張り ⑶ が任意でも赤 | Constraint |
| 2.x 振る舞い | 3 本の本文 | 移すだけ（本文を変えない） | — |
| 3.x 持ち手の表 | runtime の 169〜229・329〜352 | 置き方（H1／H2）。H2 は 3.3 の字面に合わない | Unknown（設計） |
| 4.1〜4.3 付け替え | areka-input・本体の形 | wintf・placement に静的変数と確かめの小さなファイル | Missing |
| 4.4・4.5 説明 | 4 ファイルの説明の名 | 入口の名を保てば説明は今のまま正しい | — |
| 4.6 steering | `structure.md:446-452` | ⑴ なら Modules・Dependencies・Consumers の行と「`[dependencies]` へは決して置かない」の書き換え。⑵ なら新しい節 | Missing |
| 5.1 各実行ファイルの確かめ | 5 つとも在る | 本体の crate のテストの実行ファイルにも要る（6 つ目） | Missing |
| 5.2〜5.4 本体の檻 | 3 本に同じ檻 | 1 か所へ。⑴ では機能が付かないと黙って組まれない（2.5） | Unknown（R1） |
| 5.5 較正 2 形 | wintf・placement（装置なし）・runtime（装置あり） | runtime の較正は `RaiseException` の宣言を自分で持つか、本体が較正用の口を出すか | Missing |
| 5.6 定義が 1 つ | — | 終わりの grep（`SetUnhandledExceptionFilter` の宣言など） | — |
| 6.1 ウ | 前例 `frame_work_area_resnap_hold_tests.rs:372-410` | placement 側の陽性の 1 本 | Missing |
| 6.2 エ | — | 名指しの外に 19＋19 行（2.8）・`seed_zorder_descript_base` はメソッド | Constraint |
| 6.3 オ | 同じ本文 2 つ | placement に定義・runtime は `pub use` | Missing |
| 6.4 カ | 同じ本文 2 つ | 引数の型（`CapturedEvent`）が `test-support` で見えない | Constraint |
| 6.5・6.6 | 実物の道を確かめた | 説明の書き換えだけ | — |
| 7.3 `Cargo.lock` | — | ⑴ なら正しい。⑵ なら「置き場の crate の行だけ」にならない | Constraint |

## 4. 実装の道（3 案）

### 案 A: ⑴ `log-capture-kit` に入れる（brief の推し）

- 置く物: `crates/log-capture-kit/src/crash_report.rs`（新しい・本文と檻）・`lib.rs` に `#[cfg(feature = "crash-report")] pub mod crash_report;`・`Cargo.toml` に `windows = { workspace = true, optional = true }` と `crash-report = ["dep:windows"]`。
- 使い手: wintf・placement の dev-dependency の行に `features = ["crash-report"]`（wintf は `["env-filter", "crash-report"]`）・`lib.rs` の `mod test_crash_report;` を、静的変数と確かめ 1 本の小さなファイル（areka-input の 11 行の形）に替える。
- `areka-runtime`: `crash_report_test_support.rs` を表（H1 なら）と `install`／`installed` の薄い殻に縮める。**`[dependencies]` に `log-capture-kit = { optional = true, features = ["crash-report"] }` を足し、`test-support` に `dep:log-capture-kit` を足す必要がある。**
- 当たる壁: `with_default_guard_test.rs:433` が赤。通すには ⒜ 見張りに「`test-support` の任意の依存で、機能が `crash-report` の行」だけを許す例外を足す（件数の定数と理由を持つ見張りの流儀に合わせる）、⒝ `structure.md:452`・`log-capture-kit/src/lib.rs:12`・使い手の `Cargo.toml` の注（約 10 か所）を書き換える。⒞ あるいは `areka-input` と本体の静的変数を `log_capture_kit::crash_report::install` へ直に向ける（要件 7.2 に反する＝`audit-areka-input-menu` との調整が要る）。
- 良い点: crate が増えない・`Cargo.lock` は 1 行・NOTICES は変わらない（要件 1.4・7.3 をそのまま満たす）。
- 悪い点: 「ログ捕捉の crate」に Win32 の落ちの報告が入る（`structure.md:450` が見張りの置き場で同じ食い違いを承知している前例はある）。見張りと規律の文を緩める。`cargo test -p log-capture-kit` 単独では本体の檻が組まれない（2.5）。

### 案 B: ⑵ 新しい leaf の `*-kit` を足す

- 置く物: `crates/<name>-kit/`（`publish = false`・本番の依存は `windows` だけ・名は `-kit` で終える＝数えの道具の判定 `count.rs:86`）。本文と檻はその crate の `src/` に。
- 使い手: wintf・placement は `[dev-dependencies]` に 1 行。`areka-runtime` は `[dependencies]` に任意で 1 行（`test-support` で付ける・`shiori-host32-host` と同じ前例）と、自分のテストのための dev-dependency の 1 行。
- 見張り: `log-capture-kit` の見張りには掛からない。新しい crate が本番に出ないことを見張りたいなら、`sample_path_guard_test.rs` が `real-run-kit` にしているように見張りを足す選択肢がある（要件は常設の見張りを求めていない）。
- 良い点: 機能の切り替えが無い（`cargo test -p <name>-kit` で檻が必ず組まれる）・「ログ捕捉」と「落ちの報告」が分かれる・規律の文を緩めなくてよい。
- 悪い点: `Cargo.lock` に新しい塊と 3 行（要件 7.3 の字面に合わない）・NOTICES に 1 行（要件 1.4 の字面に合わない）・`structure.md` に新しい節・crate が 1 つ増える。

### 案 C: 混ぜる（本文は ⑴、入口だけ別の道）

- 本文を `log-capture-kit` に置き、`areka-runtime` の `[dependencies]` には何も足さず、`areka-input` と本体の静的変数を `log_capture_kit::crash_report::install` へ直に向け、持ち手の行は `areka-runtime` の `#[cfg(test)]` の側と、本体・areka-input の確かめで別に扱う形。
- `crates/areka-input/crash_report_tests.rs` の 1 行を変えるので要件 7.2・仮定 A5 に反する。しかも areka-input と本体の報告から `AREKA-CRASH-HOLDER` の行が消える（表の書き手を誰も登録しない）＝要件 3.1 に反する。持ち手の行を残すには、結局 runtime の何かを据え付けの時に呼ぶ必要があり、`test-support` で runtime が本文に届く必要に戻る。**実質は成り立たない**（記録のために残す）。

## 5. 規模と危うさ

- 規模: **M**（7〜11 タスクの見込み）。本文の移しは機械的だが、置き場の決め（見張りか要件の字面か）・持ち手の口・本体の crate のテストの実行ファイルへの据え付け・申し送り 6 件（うち 6.1 は新しい陽性の檻・6.4 は型の壁）がある。
- 危うさ: **中**。`.CRT$XCU` の静的変数の置き場を誤ると、受け手が黙って据わらない（確かめのテストが赤で教える＝要件 5.1）。機能の統合で檻が黙って組まれない形（案 A）。見張りの例外の足し方を誤ると見張りが空虚になる。

## 6. 設計の段への申し送り

### 決める事（要件の討議の議題になる）

> 2026-11-21 の要件の討議で決めた（`requirements.md` の「討議で決めたこと」）: 1＝案 B（新しい leaf `crash-report-kit`）・2＝H2（表も本体へ）・3＝本体の crate 自身のテストへ（案 B なので機能の切り替えは無い）・5＝古い道 44 行すべて・6＝署名を文字列の並びに替えて `test_support` へ移す。4・7 と R2 は H2 で要らなくなった分を除き、設計の段で決める。

1. 置き場: 案 A（見張り ⑶ に例外を足し規律の文を緩める）／案 B（新しい `*-kit`・要件 1.4 と 7.3 の字面を直す）。どちらも要件のどこかを直す（7 節）。
2. 持ち手の表: H1（差し込む口・表は runtime）／H2（表も本体・要件 3.3 を直す）。
3. 本体の檻（要件 5.2〜5.4）の置き場と、それが走ることの担保（案 A なら `required-features`／test-all の段／統合に頼る、のどれか）。
4. 装置なしの較正を 1 か所にする置き場（本体の crate・wintf・placement のどれか）と、runtime の装置ありの較正が `RaiseException` をどこから取るか。
5. 申し送りエの広さ: 名指し 6 行だけ（仮定 A6）／`runtime::frame` の 25 行／古い道の 44 行全部。`seed_zorder_descript_base` の書き方（`RuntimeWiring::seed_zorder_descript_base`）。
6. 申し送りカ: 署名を文字列の並びに替えて `test_support` へ移す／`#[cfg(test)]` の placement の内側の共有に留める（runtime から引けない）／次の機会へ回す。
7. wintf と placement に残す小さなファイルの名と形（areka-input の `crash_report_tests.rs` の 11 行に倣うか、`test_crash_report.rs` の名を残すか。どちらも数えのテストの名の判定に入る）。

### 調べが要る事（Research Needed）

- R1: `log-capture-kit` 自身のテストで任意の機能を付ける正しい形（自分自身への dev-dependency は cargo が許すか・`[[test]] required-features` は `src/` の中の `#[cfg(test)]` の檻には効かない点）。
- R2: 差し込む口（H1）を割り当て無し・錠無しで書く形（受け手は壊れかけのプロセスで走る）。関数ポインタの原子変数か `OnceLock` か。
- R3: 本体の crate のテストの実行ファイルで MTA を立てることが、同じ実行ファイルのほかのテスト（`log-capture-kit` なら tracing の捕捉の檻）に影響しないか。

## 7. requirements.md の誤り・食い違い

1. **仮定 A1 と要件 1 の「⑴ と ⑵ のどちらでも満たせる」は成り立たない。** ⑴ は、要件 4.2／7.2（仮定 A5）を守ると `areka-runtime` の `[dependencies]` に `log-capture-kit` を置くことになり、`crates/log-capture-kit/tests/with_default_guard_test.rs:433` が赤になる（`structure.md:452` の規律とも食い違う）。⑵ は要件 1.4 と 7.3 の字面に合わない（次の 2・3）。
2. **要件 1.4 の括弧「`THIRD-PARTY-NOTICES.md` が変わらない」は ⑵ では偽。** `cargo about generate --workspace` はワークスペースの全メンバーを載せる（`THIRD-PARTY-NOTICES.md:3041-3077`・`about.toml:14` の `ignore-dev-dependencies` は外の dev-dependency だけを外す）。
3. **要件 7.3「`Cargo.lock` の差分を置き場の crate の行だけにする」は ⑴ では正しい（`log-capture-kit` の項に `"windows",` が 1 行）が、⑵ では偽**（新しい `[[package]]` の塊＋wintf・areka-placement・areka-runtime の項に 1 行ずつ）。⑴ に決めるなら「`log-capture-kit` の項の `dependencies` に `windows` が 1 行増えるだけ」と書くほうが確かめやすい。
4. **仮定 A2 の「要件 3.5」は無い**（要件 3 は 1〜4 まで）。指す先は要件 3 の末尾の段落。
5. **要件 3.3 が置き方を先に決めている。** 要件 3 の末尾は「表の置き方は設計の段で決める」と書くが、3.3「wintf と areka-placement は表を持たない」は案 H2（表を本体に置き、wintf・placement は使わない）を除く。H2 でも報告の行は要件 3.1・3.2 のとおりに出る（2.6 節）。
6. **要件 1.2 の「どの crate にも依存しない」は ⑴ の `log-capture-kit` では字面どおりでない**（dev-dependency に `sample-ghost-kit`・`log-capture-kit/Cargo.toml` の `[dev-dependencies]`）。「本番の依存に」と限るのが正確。
7. **要件 6.4 の「`areka-runtime` からも引ける」は今の署名のままでは組めない**（`CapturedEvent` は placement の dev-dependency の型・2.8 節カ）。しかも要件 7.1 で runtime の写しを付け替えないので、この spec では引けることを確かめる使い手が無い。
8. **要件 6.2 の書き換え先「`areka_runtime::frame::…`」は 1 か所で在る項目を指さない**: `config.rs:107` の `seed_zorder_descript_base` は `RuntimeWiring` のメソッド（`areka-runtime/src/frame/wiring.rs:133・191`）。同じ段落の 106 行（`ghost_session.rs`・`runtime::wire_runtime`）も古い道で、要件の「106〜109 行」に入っているが本文は `runtime::frame::…` と「本体の」しか挙げていない。
9. 細かい点: 要件 5.1 の 5 つの実行ファイルに、まとめた本体の crate 自身のテストの実行ファイル（要件 5.2〜5.4 の檻が走る所）が入っていない。そこにも静的変数と確かめが要る。

> 7 節の 1〜9 は 2026-11-21 の要件の討議で要件の側を直して閉じた（`requirements.md` の決定 1〜8・要件 1.3・1.4・3.3・5.1・6.2・6.4・7.3）。

## 8. 設計の段の調べ（2026-11-21・`kiro-spec-design`）

- 種別: 既存の仕組みの寄せ直し（Extension）＝軽い調べ（`design-discovery-light.md`）。外の資料は引いていない（新しい外の crate は 0・`windows` 0.62.2 はワークスペースに在る版）。下請けの調べ係は立てず、Grep／Read で測った。
- 使った手引き: `kiro-spec-design` の `design-principles.md`・`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`、`.kiro/settings/templates/specs/design.md`。ponytail（最小の変更）の流儀で「移すだけ・書き直さない」に寄せた。

### 8.1 測って分かったこと

| 項目 | 測り | 設計への効き |
|---|---|---|
| 本文の差 | wintf と runtime の写しをコメント行を除いて `diff`＝runtime は wintf の上位集合（足すのは持ち手の表・`pub`・`installed`・`#[cfg(test)]` の 2 か所だけ） | runtime の写しを本体の元にし、`#[cfg(test)]` の 2 か所を外せば 3 本の働きを 1 本で持てる |
| ワークスペースの members | `Cargo.toml:2-4` が `"crates/*"` | 根の `Cargo.toml` は変えない（新しい crate は置けば members に入る） |
| `windows` の機能 | 本文が使う `Win32_Foundation`（`Win32` の既定）・`Win32_System_Com`・`Win32_System_LibraryLoader`・`Win32_System_Threading` はワークスペースの表（`Cargo.toml:49-78`）に在る | 新しい crate は `windows = { workspace = true }` の 1 行で組める・`Cargo.lock` の `windows` の項は増えない |
| 入口の名の使い手 | `areka-input/src/crash_report_tests.rs:5・10`・`areka/src/runtime_tests/mod.rs:12・17`・`spine_wait.rs:24・683・692・704`・説明の名 4 か所（`wire_tests.rs:22`・`assets*_tests.rs`） | runtime の `crash_report_test_support` を `pub use` の薄い入口にすれば、これらはどれも差分 0（説明の名も今の入口を正しく指したまま） |
| `lib.rs` の宣言 | wintf `lib.rs:6-7`・placement `lib.rs:85-86`・runtime `lib.rs:67-70` | ファイルの名を保てば 3 つの `lib.rs` は差分 0 |
| 数えの道具の判定 | `q9-api-count/src/count.rs` の `is_test_path`（`test_*`・`*_test_support`・`*_tests`）と `is_test_tool`（`-kit` で終わる） | `crash-report-kit` も、残す `test_crash_report.rs`・`crash_report_test_support.rs` もテストの道具に数えられる＝本番の数は変わらない |
| `structure.md` | kit の節は 446〜478 行に並ぶ（log-capture-kit・temp-path-kit・sample-ghost-kit・real-run-kit）。runtime の節 312 行は `shiori-host32-host` を「`test-support` のときだけ付く optional の依存」と書く | temp-path-kit の節の後に新しい節を足し、runtime の節の Dependencies に 1 句足す |
| 見張り | `log-capture-kit/tests/` の見張りに、ワークスペースの crate の一覧や `.CRT$XCU` を数えるものは無い（grep） | 新しい crate を足しても既存の見張りは変わらない |
| `THIRD-PARTY-NOTICES.md` | `tools/test-all.ps1:91-93`（`-License` で `cargo about generate --workspace`）・106〜108 行が差分を黄色で知らせる。今の MIT の対象 crate の並び（3065 行 `log-capture-kit 0.1.0`・3077 行 `temp-path-kit 0.1.0`） | `-License` の 1 回で `crash-report-kit 0.1.0` の 1 行が足され、それをコミットに含める |
| `window_move_lines` | placement の定義（`follow_window_move_diag_tests.rs:170`）は呼び手 7 か所・`WINDOW_MOVE_RECORD_TAG` は `pub`（`diag.rs:76`）・`CapturedEvent::message` は `&str` を返す（`log-capture-kit/src/event.rs:82`）・`test_support.rs` は 552 行 | 署名を `&str` の並びにすれば `test_support` に置ける・行数の余裕がある |
| `code_only` | runtime の `source_text_test_support.rs:3-10` と placement の `test_support.rs:422-429` は同じ本文。runtime の呼び手は 6 ファイル（`use crate::source_text_test_support::code_only` ほか） | runtime の定義を `pub use areka_placement::test_support::zorder_pair_deferred::code_only;` に替えれば呼び手は差分 0 |
| 申し送りウの前例 | placement の `a_system_write_that_bypasses_its_deferral_still_trips_the_watch`（148〜153 行）は鳴る側の `#[should_panic]` で、警告の文を数えていない。runtime の前例 `frame_work_area_resnap_hold_tests.rs:385-410` は `capture` の中で `catch_unwind` し、数えてから `resume_unwind` する | placement の鳴る側の 1 本を前例の形に替える（新しいテストは足さない） |
| 古い道 44 行 | 2.8 節の一覧のとおり。今の置き場は `areka-runtime/src/` の下に在る（`frame/dpi.rs:546` の `run_dpi_phase`・`frame/drain_resnap.rs:419・617・720`・`frame/work_area_sync.rs:207`・`frame/balloon_offset_follow.rs:123`・`frame/scale_text.rs:122`・`assets.rs:314`・`lib.rs:416` の `wire_runtime`・`frame/wiring.rs:191` の `seed_zorder_descript_base`）。`runtime::move_cue_place` は今その名の関数が無い（`areka-runtime/src/move_cue.rs` の中を実装の段で確かめる） | 書き換えの決まり（設計の表）と、行ごとに今の定義を grep で確かめる手順にする |

### 8.2 まとめ（synthesis）

- **一般化**: 3 本の差は「持ち手の表を使うか」だけで、表を使わない実行ファイルでは表が空＝行が出ない（要件 3.2）。差し込む口（案 H1）は要らない＝決定 2 のとおり、表ごと本体に置く。据え付けの静的変数だけは各テストの実行ファイルの根に要る（2.6 節）ので、「本体 1 つ＋各根に 3 行の据え付け」の形に揃える（`areka-input` と本体が既に取っている形）。
- **作るか借りるか**: 落ちの報告に使える既製の crate（`minidumper`・`crash-handler` など）は、外の crate を増やさない（要件 1.4）・報告の行の形を変えない（要件 2.5）と合わないので取らない。今の本文をそのまま移す。
- **削り**:
  - wintf と placement の装置なしの較正 2 本は、本体の crate の 1 本にまとめる（本文が 1 つになったので、較正が見るもの＝報告の行の形も 1 つ）。
  - `RaiseException` の宣言を runtime に残さないため、本体に較正の口 `crash_on_an_unnamed_thread` を 1 つ出し、本体の較正と runtime の装置ありの較正が共に使う。
  - runtime の説明の名（4 か所）・`spine_wait.rs`・3 つの `lib.rs`・`areka-input`・本体の `runtime_tests/mod.rs` は、入口の名を保つことで差分 0 にする。
  - 写しが戻ることの見張りは足さない（決定 7）。新しい crate が本番の依存に現れないことの見張りも足さない（要件に無い・`real-run-kit` の見張りのような常設は決定 7 と同じ理由で見送る）。

### 8.3 研究の残り（R1〜R3）の決着

- R1（`log-capture-kit` の機能の付け方）: 案 B に決まったので要らない。`crash-report-kit` は機能を持たず、`cargo test -p crash-report-kit` で檻が必ず組まれる。
- R2（差し込む口）: H2 に決まったので要らない。
- R3（本体の crate のテストの実行ファイルの MTA）: 本体の crate のテストは落ちの報告の檻だけ（据え付けの確かめ・MTA・最初の受け手・1 行の溜め・較正）で、ほかのテストが無い＝MTA の影響を受ける相手が無い。MTA の檻は据え付けの後でないと緑にならないので、本体の crate のテストの実行ファイルにも据え付けの静的変数を置く。

### 8.4 危うさ

- 静的変数の置き場を誤ると受け手が黙って据わらない → 各実行ファイルの据え付けの確かめ（要件 5.1）が赤で知らせる。
- 装置ありの較正は GPU を使い、プロセスを落とす → 走らせるのは仕上げの確かめ（要件 8.3）の 1 回ずつだけ・排他の机の決まりに従う。
