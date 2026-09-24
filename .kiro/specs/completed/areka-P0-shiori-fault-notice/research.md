# ギャップ分析: areka-P0-shiori-fault-notice

> 2026-09-24・本ブランチ（main `5db3672a` の内容）で、要件 1〜8 と今のコードを突き合わせた。コードは「何の定義か」（関数名・型名＋ファイルパス）で指し、行番号では指さない。brief と要件に書かれた事実はすべて Grep／Read で引き直した（引き直して食い違った点は §1.4 にまとめた）。
> ここに書くのは分析と選択肢であり、決定ではない。決定は要件ディスカッションで行う。

## 0. 要約

- **土台はすべて揃っている。** 告知の部品（`crates/areka/src/alert.rs` の `AlertScene`／`alert_text`／`raise`／`suppressed`）、終了の統合操作（`crates/areka/src/app_exit.rs` の `quit_app`／`ExitOrigin`）、停止通知の 1 本道（kanade `notify_stop` → `Emo2Wiring::kanade_stop` → `run_ghost_quit_phase`）、決定論的に接続を失敗させる口（`areka_ghost::ShioriWiring::Custom`）、実機用の失敗検体（`crates/shiori-host32-testdll-loadu`）は、いずれも実在を確かめた。新しい仕組みは要らない。
- **欠けているのは 4 つ。** ⑴ 失敗の種類と理由が `KanadeStopCause::Fault`（中身なし）で失われる、⑵ `quit_app` が出所を記録に残すだけで後から読める場所に置かない（`fn main` は終了の理由を知る術がない）、⑶ 告知の部品の題名が定数 `TITLE` 1 本で場面ごとに分けられない、⑷ `fn main` の終了コードは `app.run()` が正常に戻れば必ず 0。
- **推奨は「既存の部品を広げる」（案 A）**: kanade の停止通知に失敗の種類と理由を足し、`quit_app` が最初の出所を World の Resource に置き、`fn main` が `run()` の後にそれを読んで「告知するか・終了コードを何にするか」を 1 つの純関数で決める。wintf は無改変。新規ファイルは smoke ④ の検体を組む道具だけ。
- **最大の注意点は常設 smoke ①② の前提が変わること。** 今日 ①② は `target/debug/shiori-host32-helper.exe`（i686 の成果物）が無くても緑（接続失敗 → Fault → 終了コード 0）。要件 3.1・4.2 を入れると、helper が置かれていない環境では ①② が赤になる。これは盲点を塞ぐ狙いどおりだが、「i686 の成果物を置いてから走らせる」前提を smoke の側にどう表すか（受理か・案内して失敗か・test 側で複製するか）を決める必要がある（議題 5）。
- **規模 S・リスク 低〜中。** 変更は既存の型に欄を足す形が中心で、kanade の公開型 `KanadeStopped`／`KanadeMsg::ShioriDown` を触るため、構築点（テスト含む）の書き換えが 20 か所前後に及ぶ。判断そのものは変えない。

## 1. 現状の調査（2026-09-24 実測）

### 1.1 失敗が終了に至る道（すべて実在を確認）

| 段 | 何の定義か | 今日の振る舞い |
|---|---|---|
| 接続の失敗 | `crates/areka-kanade/src/shiori/real.rs` の `spawn_shiori_actor`（`connect()` が `Err(reason)`） | `error!(event="connect_failed", reason)` → `KanadeMsg::ShioriDown { reason }` を送る。理由の文字列は `crates/areka-ghost/src/shiori_wiring.rs` の `real_connect` が組む（DLL 名未解決・親窓生成失敗・helper の spawn 失敗・HELLO 未受領・LOAD の ack が `[1]` でない、の 5 形） |
| helper の予期しない終了 | 同ファイルの `report_exit_once` | `error!(event="helper_exited")` → `ShioriDown { reason: "helper exited unexpectedly: {kind:?}" }` を 1 度だけ |
| 呼出の失敗 | 同ファイルの `map_error` | `RequestError` の 4 種を `ShioriFailure::{Handshake, Timeout, Ipc, Shiori}` へ 1 対 1 で写す。`ShioriFailure::Internal` は kanade 内部（送出 ID の受理規則違反）だけが作る |
| エラー応答の判定 | `crates/shiori-host32-host/src/client.rs` の `map_get_result` | 400・500・`ErrorLevel` 付きは `RequestError::Shiori`、204 は無応答、それ以外の状態は成功 |
| 送出失敗・応答の切断・期限切れ | `crates/areka-kanade/src/actor.rs` の `round_trip` | 3 腕とも `ShioriOutcome::Failed(ShioriFailure::Ipc(..))` に写す（期限切れの腕は `recv()` が無限待ちのため防御用で到達しない） |
| Fault への入口 | `crates/areka-kanade/src/schedule/mod.rs` の `step`（`Input::ShioriDown` の腕）と `on_shiori_reply`（応答待ちの相の `Failed`） | どちらも `to_unloading_fault(state)` へ。例外は `BootPrefetch`（`username` 照会）と選択肢の往復中（`steady.rs` の `choice_shiori_failed_as_204`） |
| 内部の原因 | 同ファイルの `TermCause`（`Quit`・`Forced`・`CloseSilent`・`DeadlineExceeded`・`Fault`・derive なし・`pub(crate)`） | `Phase::Unloading { cause }` に載る。`Fault` は中身を持たない |
| 公開語彙への写し | `crates/areka-kanade/src/actor.rs` の `stop_cause_of`（wildcard なしの 1 対 1）と `notify_stop` | `KanadeStopped { cause }` を 1 度だけ送る。原因を控えられなかったときは `warn!(event="stop_cause_unknown")` の上で `Fault` として送る |
| 公開型 | `crates/areka-kanade/src/msg.rs` の `KanadeStopCause`（5 値・`Copy`）と `KanadeStopped { cause }`（`Copy`・`PartialEq`） | 中身なし |
| UI 側の受け口 | `crates/areka/src/emo2_boot/mod.rs` の `wire_emo2_boot`（`mpsc::channel::<KanadeStopped>()` を作り `boot_with_kanade_stop(.., Some(tx))`・`wiring.set_kanade_stop(rx)`） | 実 sink 結線のときだけ受け口がある。`fn main` の `else` 腕（`areka_ghost::boot`＝`boot_with_kanade_stop(options, None)`）には無い |
| 窓を閉じて終了を指示 | `crates/areka/src/emo2_boot/frame.rs` の `run_ghost_quit_phase` | 通知を全件取り出し、`info!(event="ghost_quit", cause)` の上で `quit_app(world, ExitOrigin::KanadeStopped(stopped.cause))` を 1 度 |
| 統合操作 | `crates/areka/src/app_exit.rs` の `quit_app`（`ExitOrigin` は `Copy`・4 値） | 全窓を despawn → `info!(event="app_exit", origin, closed)` → `AppExit::request_exit()`。**出所を後から読める場所には置かない** |
| 他の終了の指示 | `crates/areka/src/input_events/mod.rs`（`ExitOrigin::Escape`）・`crates/areka/src/main.rs` の `open_startup_window` 内の smoke タイマー（`ExitOrigin::Smoke`）・`app_exit.rs` の `on_ghost_os_close`（`ExitOrigin::OsClose`） | `quit_app` の呼び手は計 4 か所 |
| 終了コード | `crates/areka/src/main.rs` の `fn main() -> windows::core::Result<()>` | `app.run()?` が正常に戻ると ① loop ticker の Close → ② `GhostRuntime::shutdown(CloseReason::User { scope: 0 })` → ③ seriko の join → ④ 性能報告 → `Ok(())`＝**理由を問わず 0**。0 以外は起動解決の失敗・起動窓・`shutdown` の失敗・join の失敗の `Err` 経路（Rust の `Termination` が 1 にする）だけ |
| 告知の部品 | `crates/areka/src/alert.rs`（`AlertScene` 4 値・定数 `TITLE`・`alert_text` 純関数・`suppressed_from`／`suppressed`・`raise`） | `raise` は `error!(event="alert", scene, title, body, suppressed)` を必ず 1 件残し、抑止でなければ `MessageBoxW(None, .., MB_OK|MB_ICONERROR)`。`alert_tests.rs` は 9 本（`#[test]` を数えた） |
| 常設 smoke | `crates/areka/tests/smoke_boot_loop_exit.rs` | 3 方向・`AREKA_APP_SMOKE_EXIT_MS=500`・`AREKA_NO_ALERT=1`・`NO_COLOR=1`・60 秒の見張り。①② は exit 0＋`WIRED`＋`REAL_WINDOWS` の目印（モニタ 0 台なら `accepted_as_no_monitor` で非 0 を受理）。**SHIORI の失敗が無いことは見ていない** |

### 1.2 「起動時」と「会話中」の見分けが付かないこと

`crates/areka-ghost/src/runtime.rs` の `boot_with_kanade_stop` は `spawn_shiori_actor(connect, down_tx)` を呼んで戻る＝接続はアクターのスレッド上で後から起き、boot は成功を返す。`wire_emo2_boot` も `main` も接続の成否を待たない。停止通知に載るのは失敗の種類だけで、「起動の何段目か」は載らない（要件 1.9 の根拠）。

### 1.3 ゴーストの名前と置き場所がどこにあるか（要件 1.3 の (a)）

- 置き場所: `fn main` が持つ `cfg.ghost_root`（`resolve_boot` の結果）。
- 名前: `GhostRuntime::mount().names.name`（`crates/areka-parsers/src/package/model.rs` の `GhostNames.name: Option<String>`＝descript の `name`）。`ghost_runtime` は `Option<GhostRuntime>` として `main` が持つ。`shutdown(self)` は値を消費するので、告知を後始末の後に出すなら名前を先に控える必要がある。
- kanade・frame の側には名前を運ばせなくてよい（告知の文面を組むのは `main` の場所）。

### 1.4 brief／要件の記述と引き直しの差

- 一致: 上の表の全項目。`alert_tests.rs` 9 本。`REQUEST_TIMEOUT` 60 秒と `AREKA_SHIORI_REQUEST_TIMEOUT_MS`（`crates/shiori-host32-host/src/process_host.rs`）。`shiori_loadu.dll` は `HOST32_TESTDLL_LOADU_FAIL=1` で `loadu` が 0・`request` は常に `400 Bad Request`（`crates/shiori-host32-testdll-loadu/src/lib.rs`）。
- 補足 1: `KanadeMsg::ShioriDown { reason: String }` の構築・照合は本番 4 か所（`actor.rs` の写し・`real.rs` の 2 送出・`schedule/mod.rs` の腕）＋テスト 9 ファイル＝計 21 行。`KanadeStopped { .. }` の構築は本番 1（`notify_stop`）＋テスト 3 ファイル。公開型に欄を足すとこれらが動く。
- 補足 2: brief の「相乗り 1 件」（`GhostBootError` の doc が退役済みの「ダミー窓」を今の挙動として書く）は `crates/areka-ghost/src/runtime.rs` の `GhostBootError` の doc に実在する（1 行の直し・本仕様で拾うかは自由）。
- 補足 3: 本ワークツリーには `target/debug/shiori-host32-helper.exe`（areka.exe の隣）と `target/i686-pc-windows-msvc/debug/shiori-host32-helper.exe`・`shiori.dll` が置かれている。`shiori_loadu.dll` の i686 成果物は無い（smoke ④ で要る）。（実装時注記 2026-09-24: `target/debug/` の隣の helper は `cargo build --workspace` が置く **x64** だった＝i686 ではない。smoke は PE の種別で見分けて置き換える。）
- 補足 4: emo2 検体の `ghost/master/descript.txt` は `shiori,pasta.dll`（32bit）を宣言する＝smoke ①② は helper が隣に無いと `real_connect` の spawn で失敗し、今日は「Fault → 終了コード 0」で緑になっている（要件 4.2 が塞ぐ盲点そのもの）。

## 2. 要件ごとの実現性と欠け

| 要件 | 既存の資産 | 欠け（種別） |
|---|---|---|
| 1.1・1.11・1.12 告知は窓を閉じたあと 1 回 | `quit_app` が窓を閉じてから終了を指示する。`AppExit::request_exit` は 2 回目以降を流す | **Missing**: 「終了の理由」を `fn main` が読む場所。**Constraint**: `quit_app` は 4 か所から呼ばれ 2 度呼ばれうる（強制退避の後に握手が完了する順序＝`frame_ghost_quit_tests.rs` の 4 本目）＝「最初の出所」と「後から来た出所」のどちらを採るかを決める（議題 3） |
| 1.2 入口ごとに告知を作らない | 停止通知の 1 本道 | なし（構造で満たす） |
| 1.3・1.4・5.1〜5.3 題名と本文・6 語 | `alert_text` は純関数、`TITLE` は定数 | **Missing**: 場面ごとの題名・新しい場面 `AlertScene::ShioriFault { .. }`。文面の語は要件が定める |
| 1.5・3.5 記録と告知を同じ 1 件で | `raise` が `error!(event="alert")` を必ず残す | なし。ただし要件 3.5 の「SHIORI の失敗で終わった目印」を `event="alert"` の 1 件で兼ねるか、別の `event` を立てるかは決める（議題 6・smoke の照合語に効く） |
| 1.6・3.4・4.6 抑止は表示だけ | `suppressed()`／`AREKA_NO_ALERT` | なし |
| 1.7・1.8・3.3・7.2 Fault 以外は告知なし・0 | `ExitOrigin` 4 値＋`KanadeStopCause` 5 値 | **Missing**: 判定表の純関数（`ExitOrigin` → 告知するか／終了コード） |
| 1.10 告知を閉じたらそのまま終える | `MB_OK` のみ | なし |
| 2.1〜2.5 種類と理由を運ぶ | `ShioriFailure` 5 種・`ShioriDown { reason }`・`TermCause::Fault`・`KanadeStopCause::Fault` | **Missing**: `TermCause::Fault` と `KanadeStopped` に種類と理由を載せる欄。**Constraint**: 要件 2.2「綴りで判別しない」ゆえ `KanadeMsg::ShioriDown` に種類（確立の失敗／helper の終了）を型で持たせる必要がある（`real.rs` の 2 送出点は互いに別の関数なので型で分けられる） |
| 3.1・3.2 終了コード 0 以外・後始末の後に決める | `fn main` の順序 ①〜④ | **Missing**: 終了コードの決め方。**Constraint**: `fn main` は `Result<()>`；`Err` で 1 |
| 3.6 §8 に記す | `doc/COMPAT_ARCHITECTURE.md` §8 の表（`| 項目 | 裁量 | 根拠 | 出典 spec |`） | なし（1〜2 行を足す） |
| 4.1・4.2 ①② に「失敗していない」を足す | `assert_line_has`／`accepted_as_no_monitor` の判定の型 | **Constraint**: helper が無い環境で ①② が赤になる（議題 5） |
| 4.3・4.4 smoke ④ | `run_smoke` 共通ドライバ・`SampleRoot::acquire`・`TempPath`・i686 の探索の型（`crates/shiori-host32-helper/src/main_loopback_tests.rs` の `resolve_testdll`・`crates/areka-kanade/tests/kanade/real_helper_test.rs` の `resolve_helper_exe`） | **Missing**: 失敗する検体のフォルダを組む道具（`descript.txt`＋`shiori_loadu.dll` の複製）。**Unknown**: `shiori_loadu.dll` の i686 成果物が無いときに ④ をどう扱うか（議題 5 と同根） |
| 4.5 自動終了との競合 | `quit_app(Smoke)` と `quit_app(KanadeStopped)` は同じ `AppExit` を叩く | 判定の純関数で明文にする（議題 3） |
| 6.1〜6.6 判断・境界を変えない | — | なし（運ぶだけ）。6.4 は roadmap の「登記だけの行」に 1 行足す（議題 4） |
| 7.1〜7.6 決定論テスト | `alert_tests.rs`・`app_exit_tests.rs`・`frame_ghost_quit_tests.rs`・`actor_stop_notify_tests.rs`・`schedule_tests.rs`・`runtime_tests.rs`（`ShioriWiring::Custom` の型） | 置き場は既存の兄弟ファイル。7.5 の起動→接続失敗→通知は `boot_with_kanade_stop(.., Some(tx))`＋`Custom(Err)`＋`rx.recv_timeout` で組める（`runtime_tests.rs` の `boot_returns_mount_error_and_short_circuits_before_touching_shiori_wiring` と同じ道具立て） |
| 7.7・7.8 実機 | 完了 `areka-P0-shiori-loadu` の `research.md` §12.3〜12.4（i686 の建て方・helper の複製・`RUST_LOG` の値・コマンド）がそのまま使える | ① の検体は smoke ④ と同じ道具で組める。② は `AREKA_SHIORI_REQUEST_TIMEOUT_MS` を極端に短く（例 1）して動く検体を起動 |

## 3. 実装の選択肢

### 案 A: 既存の部品を広げる（推奨）

**kanade（運ぶ）**

- `msg.rs`: 失敗の種類の公開型を 1 つ足す（例: `ShioriFaultKind { ConnectFailed, Timeout, Disconnected, ErrorResponse, Internal, Unknown }`＝要件 1.4 の 6 語に 1 対 1）と、種類＋理由の組（例: `ShioriFault { kind, reason: String }`・`Clone`・`PartialEq`）。`KanadeStopped` に `fault: Option<ShioriFault>` を足す（`Copy` は外れる・`cause` は 5 値のまま＝`Debug` 出力 `KanadeStopped(Fault)` の検索語を変えない）。`KanadeMsg::ShioriDown` は理由の綴りで判別しないため、種類を型で持つ（`ShioriDown { kind: ShioriDownKind, reason }` か、`reason` を 2 値の enum にする）。
- `schedule/mod.rs`: `TermCause::Fault` に `ShioriFault` を載せる。`to_unloading_fault` は種類と理由を受け取る。`Input::ShioriDown` の腕は `kind` から、`on_shiori_reply` の `shiori_failed` の腕は `ShioriFailure` の 5 種から写す（写しは 1 か所の純関数に閉じる）。
- `actor.rs`: `stop_cause_of` は `Option<(KanadeStopCause, Option<ShioriFault>)>` 相当を返す（`TermCause` は `pub(crate)` なので `Clone` を足して控える）。`notify_stop` は原因を控えられなかったとき `kind: Unknown` を載せる。
- 判断（どの腕が Fault に倒すか）は無改変。

**areka（読んで決める）**

- `app_exit.rs`: `ExitOrigin::KanadeStopped` に `KanadeStopped`（種類と理由込み）を載せる（`Copy` → `Clone`）。`quit_app` は**最初の**出所を World の `Resource`（例: `LastExit(ExitOrigin)`）に置く（`AppExit::request_exit` が 2 回目を流すのと同じ「最初が勝つ」規則。`is_requested()` で判る）。判定の純関数を 1 つ足す（例: `exit_decision(&ExitOrigin) -> (Option<AlertScene>, u8)`）＝要件 7.2 の表。
- `main.rs`: `app.run()?` の後に `app.world().borrow().world().get_resource::<LastExit>()` を読み、告知（`alert::raise`）→ 後始末 ①〜④ → 終了コード。`fn main` の戻りは `Result<ExitCode>` に変える（`Ok(ExitCode::from(1))`）か、後始末の後に `Err(E_FAIL)` を返す（既存の失敗と同じ 1）。
- `alert.rs`: `AlertScene::ShioriFault { ghost_name: Option<String>, ghost_root: PathBuf, kind, reason }` を足し、`alert_text` が場面ごとの題名を返す（既存 4 場面は `TITLE` のまま）。
- `frame.rs`: `run_ghost_quit_phase` は `ExitOrigin::KanadeStopped(stopped)` を渡すだけ（分岐を足さない＝要件 6.5）。
- smoke: ①② に「`event="alert"`（または 3.5 の目印）が 0 件」を足す。④ を 1 本足し、検体を組む道具（`descript.txt`＝`charset,UTF-8`・`name,..`・`shiori,shiori_loadu.dll`・`shell/master/` の最小構成＋`shiori_loadu.dll` の複製）を test 側に置く。

- 長所: 新しい仕組みなし・wintf 無改変・構造は 1 本道のまま・判断は純関数でテストできる。
- 短所: 公開型の欄が増えるので構築点（主にテスト）を 20 か所前後書き換える。`ExitOrigin` が `Copy` でなくなる。

### 案 B: kanade を触らず、areka 側だけで告知する

- 停止通知は `Fault` のまま。告知は「SHIORI が動かなくなった」とだけ告げ、理由はログへ誘導する。
- 長所: 変更は areka の 4 ファイルに閉じる。#13 との共有は 2 本のまま。
- 短所: 要件 1.4・2.x（種類を載せる）を満たさない＝裁定 2 を覆す必要がある。brief の Desired Outcome 1 に反する。

### 案 C: 種類だけ kanade から運び、理由は運ばない

- `KanadeStopCause::Fault` を `Fault(ShioriFaultKind)`（`Copy` のまま）にし、理由の文字列はログにだけ残す。
- 長所: `Copy` を保てる・欄の追加が小さい。
- 短所: 要件 1.3 (c)「記録と同じ理由の一行」を告知に載せられない。`Debug` 出力が `Fault(ConnectFailed)` になり、既存の検索語 `KanadeStopped(Fault)` が変わる。

### 出さない告知の場所の別案（要件 8.4 で既に退けられている）

- frame の相の中（`run_ghost_quit_phase`）で `MessageBoxW` を回す案は、ECS の system の中で入れ子のメッセージループを作る。`quit_app` が窓を despawn したフレームの中で出すことになり、窓が実際に消える（`FrameFinalize`／WUC の反映）より先にモーダルが立つ可能性がある。採らない。

## 4. 規模とリスク

- **規模: S**（既存の型と関数に欄と腕を足す形が中心。新規ファイルは smoke ④ の検体を組む道具と、テストの兄弟ファイル数本）。
- **リスク: 低〜中**。
  - 低: 判断を変えない・wintf 無改変・告知と終了の場所は既に 1 本道。
  - 中: `KanadeStopped`・`KanadeMsg::ShioriDown` は公開型で、テストの構築点が多い（§1.4 補足 1）。`ExitOrigin` の `Copy` 撤去は 4 呼び手が値渡しなので機械的。
  - 注意: smoke ①② の前提が変わる（§0・議題 5）。**実装者は `target/debug/shiori-host32-helper.exe` を置いてから smoke を走らせないと ①② が赤になる**（本ワークツリーには置かれている）。

## 5. 設計フェーズへの推奨

- **推奨: 案 A。** 種類は kanade の公開型で 1 対 1 に写し、areka は `ExitOrigin` を World の Resource で `main` へ運び、判定は `app_exit.rs` の純関数 1 つに集める。
- 設計で決める事項（下の議題）以外に、次を design に明記する:
  - 種類の写し（`ShioriFailure` 5 種＋`ShioriDown` 2 種＋原因不明 → 6 語）を 1 つの純関数に閉じ、wildcard を置かない（既存 `stop_cause_of` と同じ規律）。
  - `quit_app` は「最初の出所が勝つ」（`AppExit` の規則と同じ）。2 度目は `debug!` で流す。
  - `fn main` の終了コードは後始末 ①〜④ の**後**で決める（要件 3.2）。#58 が終了順序を関数へ括り出すとき、その関数の戻り値に終了コードを載せられる形にしておく。
  - smoke ④ の検体は `SampleRoot` に登記せず（`vendors/sample_ghost` に nar を足さない）、test 側の道具で一時フォルダに組む。`shiori_loadu.dll` の探索は既存の `resolve_testdll` の型（env 優先 → `target/i686-pc-windows-msvc/{debug,release}` 探索）に倣う。
- **Research Needed（設計で確かめる）**
  1. `MessageBoxW(None, ..)` を `run()` が戻った直後（WinApp は生存・窓は despawn 済み）に呼んだとき、WUC の窓の実体が消えているか（要件 1.11 の実機確認 ① で見る。wintf の `run()` の doc は「登録表に残っていた窓を壊してから戻る」と書く）。
  2. `AREKA_SHIORI_REQUEST_TIMEOUT_MS` を極端に短くしたとき、期限切れが起動時の最初の問い合わせ（`OnInitialize`）で起きて「会話中」の確認にならない可能性（要件 7.7 ②）。値の選び方（例: 起動が通る程度に長く、`OnSecondChange` で切れる程度に短く）を実機で当たる。
  3. smoke ④ で `loadu` を偽にする env `HOST32_TESTDLL_LOADU_FAIL=1` は helper の子プロセスに渡る必要がある（areka → helper へ env が継承されることを確かめる）。渡らない場合でも `request` が常に 400 を返すので「エラーを返した」の形で Fault になる（種類の期待値が変わる）。

## 6. 設計判断の議題（要件ディスカッションへ）

> **2026-09-24 要件ディスカッションの反映（裁定 3 を覆した）**: エラー応答（`ShioriFailure::Shiori`＝400・500・`ErrorLevel` 付き）は致命の失敗から外し、起動時・会話中のどの相でも 204 と同じ扱いで会話を続ける（要件 6.1）。設計への影響: ⑴ `crates/areka-kanade/src/schedule/mod.rs` の `on_shiori_reply` で `Failed(ShioriFailure::Shiori(..))` を `to_unloading_fault` へ倒さず、`steady.rs` の `choice_shiori_failed_as_204` と同じ「返事なし」の道へ写す（全相＝1 か所の分岐で足りるか、相ごとの腕が要るかは設計で確かめる）。記録は残す（`error!` のままか `warn!` か）。⑵ 失敗の種類の語彙は 5 語（「エラーを返した」は無し）＝`ShioriFailure` 5 種から Fault の種類への写しは `Shiori` の腕を持たない（到達しない腕を置かない・wildcard も置かない＝写しの入口を「Fault へ入る 4 種」に絞った型にする）。⑶ smoke ④ と実機 ① の検体は `HOST32_TESTDLL_LOADU_FAIL=1` の `loadu` 偽（＝接続できなかった）でしか作れない。§5 の Research Needed 3（env が helper の子へ届くか）が**前提条件**になる＝届かないなら検証用 DLL（test crate）を env なしで偽を返す形に直す（要件 4.4）。⑷ 出どころの実測「YAYA が最初の問い合わせの 500 で 0.47 秒で終了」の形は、この spec の後は「ゴーストは出るが起動の台詞が無い・記録 1 件」になる。
>
> **同日の反映（裁定 5・6 を覆した＝2 件とも本仕様で拾う）**: ⑸ #57＝`fn main` の `app.run()?` の `?` を外し、`Ok`／`Err` どちらでも後始末 ①〜④ を通してから終了コードを決める（`Err` は 0 以外）。「`run()` の結果を受けて後始末へ進む」判断を関数に切り出す（#58 が後で括り出す関数の芽になる）。panic は対象外。⑹ LogSink 側の起動（`fn main` の `else` 腕＝`areka_ghost::boot`＝`boot_with_kanade_stop(options, None)`）にも停止通知の受け口を通す。`wire_emo2_boot` は `mpsc::channel::<KanadeStopped>()` を作って `wiring.set_kanade_stop(rx)` に置くので、受け口（`run_ghost_quit_phase` が読む `Receiver`）を結線の成否より前に作り、`else` 腕でも `boot_with_kanade_stop(.., Some(tx))` を渡す形が最小。受け口の置き場（`Emo2Wiring` の中か World の Resource か）は設計で決める。§6 の議題 4 は消滅（登記先の選択は不要）。規模は 7〜9 タスク。


1. **失敗の種類と理由の載せ方（案 A／C）**: `KanadeStopped` に `fault: Option<ShioriFault>` を足して `cause` の 5 値と `Debug` の検索語を保つか、`KanadeStopCause::Fault(kind)` に変えて `Copy` を保つか。推奨は前者（理由の一行を運べる・検索語が変わらない）。
2. **`KanadeMsg::ShioriDown` の型**: 要件 2.2「綴りで判別しない」を満たすため、種類を型で持つ（`{ kind, reason }` か 2 値 enum）。構築点は本番 3・テスト 9 ファイル。
3. **出所が 2 つ来たときの勝ち方**: `quit_app` は「最初の出所」を記録する（`AppExit` と同じ）か、「Fault が 1 つでもあれば Fault」とするか。前者だと「強制退避（Escape）の後に Fault の通知」は告知なし・0、「Fault の後に smoke の自動終了」は告知あり・0 以外。要件 4.5 は「自動終了が先なら 0」と書くので前者と整合する。推奨は前者。
4. **裁定 5 の登記先**: LogSink 側の起動（`fn main` の `else` 腕）で Fault が起きると窓が残る穴を、roadmap の「登記だけの行」に新規の行として足すか、#58 `ghost-restart-unit` の brief へ追記するか。#58 は `fn main` の起動経路を関数へ括り出すので、そこで `else` 腕にも停止通知の受け口を通せる（#58 への追記が自然）。
5. **smoke ①② と i686 の成果物**: 要件 3.1・4.2 により、`target/debug/shiori-host32-helper.exe` が無い環境では ①② が赤になる（今日は緑）。選択肢: (a) 前提として受け入れ、失敗時のメッセージで「i686 helper を建てて隣へ置く」手順を案内する、(b) helper が無いときは `accepted_as_no_monitor` と同じ形で「接続できなかった」の非 0 を受理する（盲点が一部戻る）、(c) test 側で `target/i686-pc-windows-msvc/{debug,release}` から areka.exe の隣へ複製する（本番コードに手を入れない・成果物が無ければ (a) の案内）。smoke ④ の `shiori_loadu.dll` も同じ扱いにする。推奨は (c)＋(a)。**要件ディスカッション（2026-09-24）で (c)＋(a) を要件 4.1 に明記した**＝設計では複製の置き方（探索の順・複製の型 `resolve_helper_exe`／`resolve_testdll` に倣う）だけを決める。
6. **要件 3.5 の目印**: 「SHIORI の失敗で終わった」記録を `raise` の `error!(event="alert")` 1 件で兼ねる（`scene` 欄で場面が判る）か、`quit_app`／`main` に別の `event`（例 `shiori_fault_exit`）を立てるか。smoke ①②④ の照合語が決まる。推奨は `alert` の 1 件で兼ね、照合は本文の冒頭の文言（既存 ③ が「ゴーストが見つかりません」で照合しているのと同型）。
7. **終了コードの出し方**: `fn main` を `Result<ExitCode>` にして `Ok(ExitCode::from(1))` を返すか、後始末の後に `Err(E_FAIL)` を返して既存の 1 に揃えるか。値は 1 でよい（要件 3.1「起動前の失敗と同じ値でも構わない」）。`Err` は stderr に `Error: ..` を 1 行出す（release は `windows_subsystem="windows"` なので見えない）。
8. **告知を出す位置（後始末の前か後か）**: `run()` の直後（後始末の前・actors は手空きで生存）か、後始末 ①〜④ の後か。前者は `shutdown` が `Err` を返す既存の早期 `return`（#57 の形）に巻き込まれず必ず出る。後者は「全部畳んでから」の形で分かりやすいが、名前を `shutdown(self)` の前に控える必要がある。推奨は前者。
9. **題名と文面の語**: 新しい場面の題名（例「SHIORI が動かなくなりました」）と本文 3 行（どのゴーストか／失敗の種類／理由）の定型。既存 4 場面の 3 行構成に揃える。
10. **`GhostBootError` の doc の古い記述（「ダミー窓」）**: 本仕様で 1 行直すか、直接修正候補へ送るか。

## 7. 設計フェーズの調査と決定（2026-09-24・`design.md` の根拠）

> 設計の入力は要件 1〜8 と §6 の「2026-09-24 討議の反映」（§1〜§5 の古い記述より優先）。ここは調査の記録と決定の理由で、結論は `design.md` に書き直してある（`design.md` だけで読める）。

### 7.1 要約

- **Discovery Scope**: Extension（既存の 1 本道に中身と読む場所を足す）。外部依存・新規 crate・新しい環境変数は 0。
- **主な発見**:
  1. エラー応答を「返事なし」に写す場所は運行表（`schedule/*.rs`）には置けない。`BootInit`（`OnInitialize`＝NOTIFY）は `Notified` だけを次へ進め、`NoContent` は「想定外」で相を維持する（`crates/areka-kanade/src/schedule/boot.rs` の `BootInit` の腕）。応答が GET か NOTIFY かを知っているのは送出点 `round_trip_request`（`actor.rs`）だけ＝写しはそこに置く（design Flow 2）。
  2. `HOST32_TESTDLL_LOADU_FAIL` は helper の子へ届く: `crates/shiori-host32-host/src/process_host.rs` の `spawn` は `Command::new(helper_exe)` に `.env(..)` 3 つを足すだけで `env_clear` を掛けない＝親の環境を継ぐ（§5 の Research Needed 3 は静的に解けた・④ が実証する）。
  3. LogSink 側の起動には `Emo2Wiring` も無い（`emo2_frame_system` は `remove_non_send::<Emo2Wiring>()` が `None` で即戻る）ので、受け口を `Emo2Wiring` に置いたままでは 2 経路で共有できない。受け口は World の NonSend 資源 `KanadeStopRx` に移し、読む system を `emo2_frame_system` から独立させる。
  4. bevy 0.19.1 は `.before(system)` の相手が schedule に無くても失敗しない: `SystemSets::check_type_set_ambiguity`（`bevy_ecs-0.19.1/src/schedule/node.rs`・`schedule.rs` の `build_schedule` が呼ぶ）が拒むのは同型 system が**複数**ある set への辺だけで、空の set は辺を持たないだけ。LogSink 側の起動（`emo2_frame_system` 未登録）で `ghost_quit_system.before(emo2_frame_system)` を登録しても安全。
  5. `WinApp::run` は「exit requested while windows remained open — destroying them before returning」（`crates/wintf/src/runtime/mod.rs` の `run` の手順「残存窓の破棄」）＝`run()` の後に出す告知の背後にゴースト窓は残らない（§5 の Research Needed 1 の静的な裏取り・実機 ① で目視）。

### 7.2 調査の記録

#### 停止通知に種類と理由を載せる形

- **Context**: §6 議題 1（`fault: Option<ShioriFault>` を足すか・`Fault(kind)` にするか）。
- **Sources**: `crates/areka-kanade/src/msg.rs`（`KanadeStopCause`・`KanadeStopped`・`ShioriFailure`）、`actor.rs`（`stop_cause_of`・`notify_stop`）、`crates/areka/src/app_exit.rs`（`ExitOrigin`・`quit_app` の `origin = ?origin`）、構築点の数え（`KanadeStopped {` は本番 1＋テスト 2 ファイル・`KanadeStopCause` は 7 ファイル・`TermCause` は 10 ファイル）。
- **Findings**: 理由の一行を運ぶなら `Copy` はどのみち失う（`String`）。`fault: Option<..>` を別欄にすると「`cause: Quit` で `fault: Some`」という不正な組み合わせが型で作れる。`Debug` の検索語は `KanadeStopped(Fault(ShioriFault { .. }))` になり `KanadeStopped(Fault` で今までどおり引ける。
- **Implications**: `KanadeStopCause::Fault(ShioriFault)`（決定 1）。

#### エラー応答を致命から外す場所

- **Context**: §6 の反映 ⑴「全相＝1 か所の分岐で足りるか、相ごとの腕が要るか」。
- **Sources**: `schedule/mod.rs` の `on_shiori_reply`（横断の腕は `origin: &'static str` しか持たない）、`schedule/boot.rs`（`BootInit`＋`Notified` だけが `username` 照会へ進む・`BootVersion`＋`Notified`）、`schedule/close.rs`（`ClosePending` は GET・`Notified` は想定外）、`shiori/real.rs` の `handle_call`（GET・NOTIFY とも `Err` → `Failed(map_error(..))`）、`actor.rs` の `round_trip_request`（`method` を既に取り出している・本番と mock の唯一の実行点＝DD-IT-7）、`tests/kanade/failure_test.rs`（`FailOn::on_initialize(FailKind::Shiori)` で NOTIFY の失敗を注入できる）。
- **Findings**: 運行表に置くと GET／NOTIFY の見分けが要り相ごとの腕になる（`BootInit`・`BootVersion` は NOTIFY、`BootPrefetch`・`BootType`・`ClosePending` は GET、`Steady` は両方）。送出点なら 1 か所で全相に効く。
- **Implications**: 決定 2。`BootPrefetch` の `username` 照会だけ写し先が `ResourceOutcome::Failed` → `NoContent` に変わる（sink は「204／失敗は不在記録」で同じ・`crates/areka-ghost/src/sylphya_wiring.rs` の `make_username_resource_sink`）。

#### 受け口の共有と終了相の置き場

- **Context**: §6 の反映 ⑹（受け口の置き場＝`Emo2Wiring` の中か World の資源か）。
- **Sources**: `crates/areka/src/emo2_boot/mod.rs` の `wire_emo2_boot`（channel を作り `set_kanade_stop`・`add_systems(Update, emo2_frame_system.after(update_typewriters))`）、`frame.rs` の `emo2_frame_system`（`Emo2Wiring` 無しは no-op・終了相を先頭で呼ぶ）、`frame/wiring.rs`（`kanade_stop: Option<Receiver<..>>`）、`main.rs` の `else` 腕、`spine.rs` のハーネス（`set_kanade_stop`）、`frame_ghost_quit_tests.rs`（`wiring_with_stop`）。
- **Findings**: `Receiver` は `Sync` でないので NonSend 資源。終了相を `emo2_frame_system` から外しても、終了が決まったフレームで他の相が走ることは強制退避・smoke の自動終了で既に毎回起きている（`quit_app` は `Input` の系や UI スレッドの closure から呼ばれ、その巡の `Update` は普通に走る）。
- **Implications**: 決定 3。`Emo2Wiring::kanade_stop`／`set_kanade_stop` は退役。

#### 終了コードと #57

- **Context**: §6 議題 7・8 と反映 ⑸。
- **Sources**: `main.rs` の `fn main`（`app.run()?` → ①〜④ → `Ok(())`・②③ の失敗は `error!`＋早期 `return Err(E_FAIL)`）、`GhostRuntime::shutdown(self, ..)`（値を消費）、`GhostRuntime::mount().names.name`（`crates/areka-parsers/src/package/model.rs` の `GhostNames.name: Option<String>`）。
- **Findings**: 告知を後始末の前に出せば名前を控える必要がない。`Result<()>` のまま `Err(E_FAIL)` を返せば既存の失敗と同じ 1 で、戻り型を変えずに済む。
- **Implications**: 決定 4・5。

#### smoke ④ の検体

- **Context**: §6 議題 5 と反映 ⑶。
- **Sources**: `crates/areka/tests/smoke_boot_loop_exit.rs`（`run_smoke` は `.env("AREKA_APP_SMOKE_EXIT_MS", ..)` の後に `.envs(envs)`＝後勝ちで上書きできる）、`crates/sample-ghost-kit/src/lib.rs`（`SampleRoot::acquire` は使い捨ての複製）、`crates/areka-ghost/tests/ghost/spine_e2e_test_s2_connect_failure.rs` の `write_ghost_fixture`（最小の `descript.txt` で mount が通る 4 キー）、`crates/shiori-host32-testdll-loadu/src/lib.rs`（`loadu` は env `"1"` で 0・`request` は常に 400）、`real_helper_test.rs` の `resolve_helper_exe`／`main_loopback_tests.rs` の `resolve_testdll`（探索の型）。
- **Findings**: emo2 の複製の `ghost/master/descript.txt` を最小の UTF-8 内容で上書きし DLL を置けば、シェルは本物のまま「接続できなかった」だけを起こせる。失敗は helper の spawn＋`loadu` で約 2 秒（`shiori-loadu` の実測 1.7 秒）なので、自動終了を 20 秒にすれば失敗が先に来る。
- **Implications**: design「smoke ①②④」。

### 7.3 アーキテクチャの選択肢

| Option | Description | Strengths | Risks | Notes |
|---|---|---|---|---|
| 案 A'（採用） | 既存の部品を広げる。`Fault(ShioriFault)`・受け口は World・判定は `fault_of` 1 つ・写しは送出点 | 新しい仕組み 0・wintf 無改変・1 本道のまま | 公開型の欄が動き構築点の追随が 20 か所前後（機械的） | §3 案 A の載せ方だけ改めた |
| 案 A（§3） | 同上だが `KanadeStopped { cause, fault: Option<..> }` | `cause` 5 値と `Debug` の綴りを保つ | 不正な組み合わせを型で作れる | 採らない |
| 案 B／C（§3） | kanade 無改変／種類だけ | 変更が小さい | 要件 1.3(c)・2.x・裁定 2 に反する | 採らない |
| 運行表でエラー応答を写す | `on_shiori_reply` に腕 | 送出点に触らない | GET／NOTIFY を知らず相ごとの腕になる（`BootInit` が止まる） | 採らない |

### 7.4 設計の決定

#### 決定 1: `KanadeStopCause::Fault(ShioriFault)`
- **Alternatives**: `fault: Option<ShioriFault>` の別欄（§6 議題 1 の推奨）／`Fault(kind)` だけ（案 C）。
- **Selected**: `Fault` にだけ中身を持たせる。`KanadeStopCause`・`KanadeStopped`・`ExitOrigin` は `Copy` → `Clone`。`TermCause::Fault(ShioriFault)`＋`Clone`。
- **Rationale**: 要件 2.1「Fault だけの通知は中身を持たない」への最短の答え。不正な組み合わせが無い。検索語 `KanadeStopped(Fault` は保たれる。
- **Trade-offs**: 構築点の追随（テスト中心・判断なし）。
- **Follow-up**: `matches!(.., TermCause::Fault)` を `Fault(_)` へ。

#### 決定 2: エラー応答は送出点 `round_trip_request` で「返事なし」に写す
- **Selected**: GET → `NoContent`・NOTIFY → `Notified`・`warn!(event="shiori_error_response")` 1 件。運行表の腕は無改変。`ShioriFault::from_failure` の `Shiori` の腕は `Internal`（届かない契約が破れたことを示す・wildcard も到達しない腕の省略も置かない）。
- **Rationale**: 7.2 の調査のとおり種別を知る場所が 1 つしかない。本番・mock の両方が通る。
- **Trade-offs**: `failure_test.rs` のケース 1 から `Shiori` が外れる（要件 6.1 の帰結）。`username` 照会のエラー応答は `NoContent` として sink へ（値は同じ）。
- **Follow-up**: `warn!` が毎秒続く可能性は記録で見える（閾値を置かない裁定）。

#### 決定 3: 受け口は World の NonSend 資源 `KanadeStopRx`・終了相は独立の `ghost_quit_system`
- **Selected**: `main` が channel を 1 本作り送出端を 2 経路へ配る。`wire_kanade_stop`（`emo2_boot/mod.rs`）が資源の据え付けと `ghost_quit_system.before(emo2_frame_system)` の登録を 1 回だけ行う。`run_ghost_quit_phase(world)` の判断は不変。
- **Rationale**: 要件 6.4「受け口は 1 本を共有」を構造で満たす。`Emo2Wiring` の有無に依らない。
- **Trade-offs**: `emo2_frame_system` の「終了相を消化したフレームは他の相を飛ばす」早期 return が無くなる（強制退避と同じ形になるだけ）。
- **Follow-up**: `spine.rs`・`frame_ghost_quit_tests.rs` の据え付けの綴りを追随。

#### 決定 4: 最初の出所は `FirstExit`（Resource）・判定は `fault_of` 1 つ
- **Selected**: `quit_app` が無ければ挿す（2 度目は `debug!(app_exit_again)`）。`fault_of(&ExitOrigin) -> Option<&ShioriFault>`＝`Some` なら告知あり・終了コード 1。
- **Rationale**: `AppExit::request_exit` の「最初が勝つ」と同じ規則＝要件 4.5「自動終了が先なら 0」・議題 3 の推奨と整合。
- **Trade-offs**: なし。

#### 決定 5: 告知は後始末 ① の直後（② の前）・終了コードは後始末の後・`fn main` は `Result<()>` のまま
- **Selected**: `let run = app.run();`（`?` を外す）→ `FirstExit` → `finish_after_run(run, fault, cleanup)`（`cleanup` の中で ① → 告知 → ②③④。設計討議 2026-09-24 で「後始末の前」から ① の直後へ 1 段ずらした＝loop ticker が告知の間に指令を溜めない・費用 0）→ `Err(E_FAIL)`＝1。後始末 ①〜④ の順序と「②③ の失敗は記録して止める」は今日どおり。
- **Rationale**: 名前を控えずに済む・後始末の失敗に告知が巻き込まれない（議題 8 の推奨）。値は既存の失敗と同じ 1（議題 7・要件 3.1 が許す）。
- **Trade-offs**: debug ビルドで `Error: ..` が stderr に 1 行（既存と同じ）。
- **Follow-up**: #58 が終了順序を括り出すとき `finish_after_run` ごと運ぶ。

#### 決定 6: 題名と文面
- **Selected**: 題名 `SHIORI が動かなくなりました`（既存 4 場面は `areka を起動できません` のまま）。本文 3 行＝`ゴースト: 名前（置き場所）`／`失敗の種類: ..`／`理由: ..`。種類の 5 語は `alert.rs` の `fault_kind_text` が持つ（kanade は日本語を持たない）。
- **Rationale**: 既存 4 場面の 3 行構成に揃える（議題 9）。smoke の目印は題名の綴り。

#### 決定 7: 相乗り
- `GhostBootError` の doc の「ダミー窓」1 行は本仕様で直す（議題 10・同じ crate のテストを触るついで）。台帳 #57 の行は完了時に「#55 で修正」へ（要件 8.6）。

### 7.5 リスクと手当

- `KanadeMsg::ShioriDown` の形が変わる（本番 4・テスト 9 ファイル）— コンパイルが全点を指す・判断なし。
- smoke ①② が i686 helper 無しの環境で赤になる — テストが自分で複製し、無ければ建て方を案内して失敗（要件 4.1）。
- ④ で env が helper へ届かない — `Command` の既定で届く（7.1 の 2）。届かなければ検証用 DLL を env なしで偽に（要件 4.4）。
- 実機 ② の期限切れの値 — 起動時に切れても告知は同じ（要件 1.9）。値は `signoff.md` に記す。
- エラー応答が毎秒続く — `warn!` が毎秒 1 件（閾値を置かない裁定の帰結・記録で見える）。

### 7.6 参照

- [SHIORI/3.0 ステータスコード](https://ssp.shillest.net/ukadoc/manual/spec_shiori3.html) — エラー応答は「失敗」と定めるだけでベースウェアの振る舞いに沈黙（§8 の 2 行目の根拠）。
- [DLL 共通仕様 `loadu`／`load`](https://ssp.shillest.net/ukadoc/manual/spec_dll.html) — 初期化の失敗は DLL が返す。
- `bevy_ecs-0.19.1/src/schedule/node.rs` の `SystemSets::check_type_set_ambiguity`（`schedule.rs` の `build_schedule` が呼ぶ） — 空の system-type set への辺は許される。
- 完了 spec `areka-P0-shiori-loadu` の `research.md` §11〜§12 — 実機の手順・i686 の建て方・失敗の実測（0.47 秒／1.7 秒）。
