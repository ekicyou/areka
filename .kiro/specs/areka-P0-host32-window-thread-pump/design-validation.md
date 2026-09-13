# 設計検証レポート: areka-P0-host32-window-thread-pump

> 実施 2026-09-13（`kiro-validate-design`・非対話）。検証対象は `design.md`（2026-09-13 生成）。引用の行番号はすべて本日の HEAD `fe111dc5` の木を実際に読んで確かめた値である（design.md が引く `9e6095cd` と同じ内容・差分なし）。

## 1. 総評（Design Review Summary）

設計は要件 Introduction の裁定（有限時間の受信待ち＋手空き時の backend 通知）を忠実に写しており、`areka-kanade` の変更は疎結合化の方向のみ・Win32 の語彙は host32 に閉じる・`on_idle` は既定実装つき・ipc はコメントのみ・`client.rs` 非接触、という拘束条件をすべて守っている。design.md が引く file:line（`real.rs`・`parent_window.rs`・`lifecycle.rs`・`close.rs`・ipc `lib.rs`・helper の遅いテスト・`wuc.rs`・`shiori/mod.rs`・`real_tests.rs`・`real_helper_test.rs`）は 1 件を除きすべて現在の木と一致した。一致しなかった 1 件（テストの置き場の根拠）と、テストの判定が恒真になりうる 1 件を直せば実装に進める。

## 2. 重大な指摘（Critical Issues・2 件）

### 🔴 指摘 1: テストの置き場の根拠が実コードと食い違う（要件 7.3 からの唯一の逸脱の唯一の根拠）

- **問題**: design.md は「同一プロセスで親窓の組を 2 つ**同時に持てない**既知制約」を理由に、遅いテスト（Test-F）を別のテストバイナリ `crates/areka-kanade/tests/idle_pump_hung.rs` へ置き、要件 7.3（兄弟テストファイルに限る）から逸脱している。しかし実コードの制約は**同時に「生成」すると 2 組目が失敗する**という生成時の競合である——`crates/shiori-host32-host/src/lifecycle.rs` の `WINDOW_TEST_SERIAL` の説明（現 `:519-524`）は「同時**生成**すると 2 組目が `WindowCreationError`」と書き、`crates/shiori-host32-helper/src/main_response_flavor_hung_cage_tests.rs` はホスト役とヘルパー役の message-only 窓 2 枚を**同一プロセスで約 42 秒共存**させている。`wintf-winmsg-executor` 0.0.5 の `Window::new_ex`（クラス登録は `Once`・状態は `CreateWindowExW` の引数で渡す）にも「1 プロセス 1 組」の制約は無い。
- **影響**: 別バイナリという結論そのものは動く（プロセスが分かれるので窓は干渉せず、i686 helper も要らない）が、逸脱の理由が誤っているため、設計ディスカッションで「兄弟ファイルに戻し、窓の生成だけをロックで直列化する」案（Test-D が 41 秒待たされることはない——ロックが覆うのは生成の瞬間だけでよい）が検討されないまま逸脱が確定してしまう。
- **提案**: どちらかに決めて design.md の「置き場の裁定」と Revalidation Triggers 最終項・research §9.2 の該当節を書き直す。⑴ 別バイナリを保つなら理由を「lib テストバイナリ（開発中に最も頻繁に回す `cargo test -p areka-kanade --lib`）へ 42 秒を足さない・生成競合の直列化ロックを不要にする」へ改める。⑵ 7.3 の字義を守るなら Test-F も `real_idle_tests.rs` へ置き、`ParentMessageWindow::create()` の呼び出しだけを `lifecycle.rs` の `WINDOW_TEST_SERIAL` と同型のロックで囲む（lib テストは並列に走るので Test-F の 42 秒は他のテストを止めない）。いずれでも `WindowOnlyBackend` の 2 重複製（後述の観察 4）が消えるか残るかが変わる。
- **Traceability**: 要件 7.3・4.9・4.11。
- **Evidence**: design.md「File Structure Plan」の「置き場の裁定（要件 7.3 との関係）」・「Revalidation Triggers」最終項・research.md §9.2「テストの置き場と窓の制約」。

### 🔴 指摘 2: 「来ない」を主張する判定が、手空きの腕が回った証拠を伴わず恒真になりうる

- **問題**: Test-B の後半（「さらに `2 × IDLE_INTERVAL` 待っても 2 通目が来ない」）と Test-E（「`3 × IDLE_INTERVAL` 待っても `ShioriDown` が来ない」）は、待ちの間に受信ループが**一度も手空きの腕を通らなかった**場合でも緑になる（何も起きなければ「来ない」は自明に真）。これは要件 4.8 が退ける「短い期限で『発火しない』を主張する形」そのものであり、steering の「検査は到達する経路を踏ませよ」にも反する。特に Test-E は「正規終了後は手空きでも報告しない」＝手空きの腕で `unloaded` を見て黙る判断分岐を固定するテストなので、腕が回ったことを示せなければ判断分岐を検査していない。
- **影響**: 将来 `report_exit_once` の `unloaded` ガードが壊れても、あるいは `Timeout` の腕が `continue` せず待ちに戻らなくなっても、この 2 本は赤にならない。
- **提案**: `IdleProbeBackend` は既に `idles` を数えているので、待ちの前後で `idles` の増分 ≥ 1（できれば ≥ 2）を先に assert し、その上で「`ShioriDown` 0 通」を主張する形にする（診断文に `idles_before`／`idles_after` を含める）。Test-C も同じ理由で、GET を送る前に `idles ≥ 1` を assert して「手空きを挟んだ」ことを証拠にする。これで 3 本とも「腕が回った上で沈黙した」ことを主張するテストになる。較正（段階 1 で赤）の物語は変わらない。
- **Traceability**: 要件 4.8・4.10・1.7・4.2。
- **Evidence**: design.md「Test-A〜E」の表（Test-B・Test-C・Test-E の行）・「診断文（4.6）」段落。

## 3. 設計の強み（Design Strengths）

1. **拘束条件の忠実な反映**: `ShioriBackend::on_idle(&mut self) {}` の既定実装で他 9 実装（実数 10 件のうち `ShioriConnection` 以外・grep で確認）に差分を出さず、`pump_pending_messages(&self)` を `!Send` の `ParentMessageWindow` のメソッドにすることで「窓を作ったスレッドでしか呼べない」を型で保証している。`areka-kanade` に `windows` を足さず（`Cargo.toml` 確認済み）、host32 側も既存 feature `Win32_UI_WindowsAndMessaging` の内で済む。
2. **較正の物語が具体的**: 「段階 1＝契約と部品だけ入れて受信ループは `recv()` のまま」「段階 2＝待ちの形」の 2 コミットに分けることで、Test-A・B・D・F が本番の受信ループそのものを踏んだ上で赤→緑になる。速いテストは `send_copydata_response`（旗なし・2 秒）で「上限まで待って失敗」、遅いテストは `send_copydata`（`SMTO_ABORTIFHUNG`・5 秒）で「① は上限まで待って失敗・② は即時失敗」という、直す前の結果の署名まで書いてある。

## 4. 最終判定（Final Assessment）

- **判定: GO（条件つき）**
- **理由**: アーキテクチャの整合・要件の網羅・実装経路の明確さは満たしている。指摘 1 は文言と置き場の確認、指摘 2 はテストの主張に `idles` の増分を 1 行足すだけで、設計の骨格を変えない。設計ディスカッションで 2 件を決着させてから `/kiro-spec-tasks` へ進む。
- **次の一手**: ⑴ 指摘 1 の置き場を決め design.md の 3 か所を書き直す。⑵ 指摘 2 の Test-B／C／E の主張を「腕が回った証拠つき」へ改める。⑶ 下の観察 1（テスト本数）を直す。

## 5. 重大でない観察（設計ディスカッションで触れる価値のあるもの・5 件）

1. **既存テストの本数**: design.md「Testing Strategy」の「`real_tests.rs`（26 本相当）」は実数 **17**（`#[test]` 17 個・grep で確認）。数字を直す。
2. **説明文の書き換え対象の網羅**: 「待機中はメッセージを取り出さない」「pump フェーズ専用」「待機中に pump しない」「`recv()` で待つ」「タイマー poll は持たない」「blocking `recv`」を `crates/`・`doc/`・`.kiro/steering/` で grep した結果、design.md C8 の表が挙げる箇所（`parent_window.rs` `:13`・`:49`・`:255`・`:312`／`real.rs` `:161`・`:167`／ipc `lib.rs` `:328-329`・`:592`／helper `:5-6`・`:32`）以外に本番コードの陳腐化は無い。唯一 `crates/areka-ghost/tests/ghost/spine_e2e_test_s6_full_disconnect.rs:188` のコメント「inbox 受信（blocking recv）」が語として古くなるが、趣旨（全 Sender drop で Err→正常終了）は `recv_timeout` の `Disconnected` でも真のままで、編集集合の外なので触らない旨を C8 に一言添えるとよい。
3. **段階 1 のコミットは赤いテストを含む**: 較正のために「テストが赤のままのコミット」を履歴に残す手順になる。squash マージ前提なので害はないが、`calibration.md` に「段階 1 のコミットは意図して赤」と書き、ハーネスの完了検証が誤って退行と読まないようにする。
4. **`WindowOnlyBackend` の 2 重複製**: structure.md は「テーマ間で共有するヘルパは項目が 1 件でも `<stem>_test_support.rs` へ集約する（複製は本文の同一性を壊す）」と定める。別バイナリを保つ場合（指摘 1 の ⑴）は、1 ファイルに置いて両側から `#[path]` で読む形が取れるか検討する。兄弟ファイルへ戻す場合（⑵）は複製そのものが消える。
5. **`ShioriConnection::on_idle` の 1 行は踏まれない**: 設計はこれを「配線」として実装レビューの目視に委ねており steering と整合する。ただし `lifecycle.rs` のテストには x64 の stand-in（`cmd.exe /c exit N` を `HelperLifecycle::new` に渡す形・現 `:358-388`）が既にあるので、Test-F の backend を fake でなく実 `ShioriConnection { window, helper }` にできる可能性がある（stand-in を作る関数の可視性次第）。可能なら 1 行の委譲も本番経路として踏める。

## 6. 検証した事項の台帳（依頼された観点への回答）

| 観点 | 結果 |
|---|---|
| (a) `std::sync::mpsc::Receiver::recv_timeout` が既存の処理を飢餓させたり順序を変えたりしないか | **問題なし**。`Ok` の腕は到達で即起きる（送信で起床・遅延 0）。`Timeout` の腕の `on_idle` はキューが空になれば終わる（親 WndProc は自窓へ post しない）ので有界。`Disconnected` は inbox が空になってから返るため積み残しの取り落としも無い。FIFO は std mpsc の契約。 |
| (b) 別バイナリ `tests/idle_pump_hung.rs` が窓の制約を避けられるか・i686 helper なしで走るか | **走る**。別プロセスなので窓は干渉しない。`spawn_shiori_actor`・`ShioriBackend`・`ParentMessageWindow`・`send_copydata`／`MsgTag`／`hwnd_from_u32` はすべて公開 API か既存 dev-dep（`lib.rs` `:41`・host32 `lib.rs` `:34`・kanade `Cargo.toml` の `shiori-host32-ipc`）。helper プロセスは起こさない。ただし逸脱の**理由**が誤っている（指摘 1）。 |
| (c) 手空き時の死活監視が `ShioriDown` を二重に報告しうるか | **しない**。`report_exit_once` は `down_reported` で sticky に閉じ、到達時・手空き時とも同じ関数を通る（現行 `real.rs` `:181-194` の純移送）。Test-B がそれを見張るが、後半の主張は指摘 2 の形に直す。 |
| (d) 500 ms の根拠が OS の応答なし判定に対して妥当か | **妥当**。判定は「所有スレッドが `GetMessage`／`PeekMessage` 系を 5 秒呼ばない」（`IsHungAppWindow` の文書値）。`PM_REMOVE` の取り出しは確実に「呼んだ」扱いになる。500 ms は 10 倍、6.11 の実測 14 秒に対しては 28 倍の余裕。速いテストの上限 2 秒＝4 倍を const assert で固定する設計も整合。 |
| (e) 説明文の書き換え一覧が完全か | **完全**（観察 2 のとおり。本番コード側に漏れなし・`areka-ghost` のテストコメント 1 件は趣旨が真のまま）。 |
| design.md の file:line 主張 | `real.rs` `:47-72`・`:74-100`・`:159-171`・`:161`・`:167`・`:172-245`・`:179`・`:181-194`・`:184-192`・`:241`・291 行／`parent_window.rs` `:12-13`・`:47-50`・`:254-256`・`:258-295`・`:270-282`・`:284-289`・`:309-312`・`:320-341`・659 行／`lifecycle.rs` `:127-140`・`:519-524`／`close.rs` `:55`／ipc `lib.rs` `:284-289`・`:328-329`・`:398-431`・`:591-593`／helper `:5-6`・`:32`／`wuc.rs` `:107-117`／`shiori/mod.rs` `:44-46`／`real_tests.rs` `:603-623`／`real_helper_test.rs` `:22-24`——**すべて一致**。`ShioriBackend` 実装 10 個・kanade の lib テストバイナリに親窓を作るテスト 0 本・`Window<S>` が HWND（生ポインタ）を持ち `!Send`——**一致**。不一致は指摘 1（制約の中身）と観察 1（テスト本数）のみ。 |
| 拘束条件（裁定）の遵守 | kanade は疎結合化の方向のみ・Win32 語彙なし・`on_idle` 既定空・ipc コメントのみ・`client.rs` 非接触——**遵守**。 |
| テストの常設性 | env ゲート・`#[ignore]`・feature ゲートなし・x64 偽境界・実 helper 不使用——**遵守**。 |
