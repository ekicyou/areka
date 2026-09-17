# Requirements Document

## Project Description (Input)

32bit の脳（`shiori-host32-helper.exe`）と WM_COPYDATA で話す**ホスト窓**は SHIORI アクターのスレッドに在るが、そのスレッドは入力待ち（`crates/areka-kanade/src/shiori/real.rs` の受信ループ `run_shiori_loop` の `rx.recv()`）で止まり、要求の往復と起動時の握手以外ではウィンドウメッセージを取り出さない。Windows は「プロセス開始から 20〜30 秒を過ぎ、かつ 14 秒以上メッセージを汲まない窓」を応答なし（hung）と判定するので（`areka-P0-emo2-conformance-e2e` 記録 §13.2 行 8・2026-09-07 較正）、その窓へ「相手が応答なしなら待たずに打ち切る」送り方（`SMTO_ABORTIFHUNG`）で送る側は即失敗する。同 spec のタスク 6.11（コミット `ce545350`・`SendFlavor::Response`）は helper の**応答方向**から旗を外して実害を消したが、「窓を持つスレッドがメッセージを汲まない」構造は残っている（同記録 §13.2 行 10・Requirement 16.5 の申し送り）。

本仕様は、SHIORI アクターが**手空きの間も周期的に backend へ保守の機会を与える**形へ改め、その周期を kanade のトラフィックから切り離す。現状は backend の面倒を見る頻度が受信した仕事の量に従属しており（`backend.status()` による死活監視が `rx.recv()` の内側にある）、仕事が止まると窓のメッセージ処理も死活監視も同時に止まる。これは host32 に固有の事情ではなく、SHIORI アクターと backend の結合の問題である。本仕様はこの結合を解き、「20 秒以上仕事が無くても応答なしにならない」「その間も死活監視が働く」ことを決定論テスト（較正つき）で固定する。6.11 の応答方向の扱いは維持する（二重の守り）。

## Introduction

本仕様の到達目標は「**SHIORI アクターのスレッド（ホスト窓を持つスレッド）が、要求の無い待機中も OS の応答なし判定に落ちない**」ことである。実害は 6.11 で塞がれており走行では発現しないが、helper → areka 向きの自発通知など**将来の逆向き経路**が同じ構造に当たるため、構造そのものを直す。

現状の構造（2026-09-11 実測）:

- ホスト窓の生成・握手・送信パスは `crates/shiori-host32-host/src/parent_window.rs`。握手（`pump_until_hello_or`）の間だけ、別スレッドから自窓へ定期的に起こしメッセージを撃ってメッセージループを回す仕組み（heartbeat）が在る。握手が終わると pump は止まり、以後は `send_request` の同期送出（上限時間つき）の中で応答が再入配送されるだけである。
- SHIORI アクターの受信ループ（`crates/areka-kanade/src/shiori/real.rs` の `run_shiori_loop`）は inbox の blocking 受信で待つ。要求の往復と握手以外では窓のメッセージを取り出さない。
- 終了挨拶の再生中は kanade が毎秒の往復を止める（`crates/areka-kanade/src/schedule/close.rs` の `ClosePending` は `Tick` で pump しない）ため、ホスト窓のスレッドは 17〜19 秒メッセージを取り出さない（記録 §13.2 行 8）。
- 応答方向（helper → ホスト）の送出は 6.11 で `SMTO_ABORTIFHUNG` を外してあり（`crates/shiori-host32-ipc/src/lib.rs` の `send_flags`）、その檻が `crates/shiori-host32-helper/src/main_response_flavor_hung_cage_tests.rs`（20 秒待ち → 往復① → 20 秒待ち → 往復②・上限 90 秒）である。要求方向（ホスト → helper）は旗を保つ。

本仕様は受信ループの**待ちの形**（手空きを検出する周期の導入）と `ShioriBackend` への**手空き通知の追加**だけを変えるものであり、IPC の方式（WM_COPYDATA 一本化・再入 RESPONSE・生バイト列のみ跨ぐ）、helper の実装、アクター境界の受理規約（envelope・停止・死活報告）は変えない。

**裁定（2026-09-13・開発者承認）**: 待ちの形は「有限時間の受信待ち＋手空き時の backend 通知」を採る（brief の案 1＝起こし手段を手組みする同時待ち・案 2＝窓の専用スレッド分離は、いずれも不採用）。理由は 3 点——⑴ Win32 の規約上、窓のメッセージを取り出せるのはその窓を所有するスレッド自身だけであり、寝ている受信ループの外からは救えない。⑵ 案 2（窓の引越し）は `ParentShared`／`ResponseSlot` の跨スレッド化と `Shiori3Client` の受け型変更を伴い、`crates/shiori-host32-host/src/client.rs` へ波及して併走 spec `areka-P0-charset-canon` と同一ファイルを取り合う。⑶ 採る形は host32 のための特別扱いではなく、**backend の保守周期をトラフィックから独立させる**一般的な疎結合化であり、`ShioriBackend` の既存 10 実装（`InProcBackend` を含む）のうち host32 の実装だけが実体を持つ。

## Boundary Context

- **In scope（本仕様が担う観測可能な振る舞い）**:
  - **[待機中の応答性]** SHIORI アクターが要求の無い待機中でも、ホスト窓宛のメッセージが取り出され続け、OS の応答なし判定に落ちない。「相手が応答なしなら待たずに打ち切る」送り方の通知がホスト窓へ送られても、長い待機の後で打ち切られない。
  - **[既存契約の不変]** アクター境界の受理規約（要求へちょうど 1 回応答・Close 即時停止・全送信端 drop で正常終了・死活報告の sticky）、握手の期限意味論、要求の往復（single-in-flight・再入受領・上限時間で復帰）、正規 clean shutdown の系列が、待ちの形を変えた後も同じ観測結果を返す。
  - **[二重の守り]** 6.11 の応答方向の旗の扱い（`SMTO_ABORTIFHUNG` を付けない・上限時間のみ）とその檻を無改変で維持する。
  - **[決定論の檻（較正つき）]** x64 の偽境界（実 32bit helper・実 DLL を使わない）で、⑴ 待機中に窓宛メッセージが処理される速い檻と、⑵ 既存の hung 檻の形（20 秒待ち → 往復 → 20 秒待ち → 往復）で OS の応答なし判定に落ちないことを示す遅い檻を置き、いずれも**直す前の構造では赤**になることを較正として記録する。
  - **[実機の非退行]** e2e 手順書 §5.7 の読み方（終了挨拶の後に `unload_clean` 1 行・`unload_failed` 0 行）で非退行を確認する。
  - **[文書の追随]** 「待機中は pump しない」と述べている既存の説明文（本番コードの rustdoc・既存の檻の見出し）を、変更後の姿に合わせて書き換える。
- **Out of scope（本仕様が所有しないもの）**:
  - IPC の方式（WM_COPYDATA 一本化・wire／framing／`MsgTag`／`ResponseSlot`・両方向の上限時間）の変更。`crates/shiori-host32-ipc/` は触れない。
  - helper 側の実装（`crates/shiori-host32-helper/`）。応答方向の旗は 6.11 のまま。
  - kanade の起動系列（`areka-P0-kanade-boot-talkdone-drop`・`crates/areka-kanade/src/schedule/{boot,mod}.rs`）・終了の握手の配線（`schedule/close.rs`）。終了挨拶中に毎秒の往復を止める運行は本仕様の前提であり変えない。
  - helper → areka 向きの自発通知そのもの（将来の spec）。本仕様はその経路が当たる構造を除くだけで、通知の語彙・配送は定義しない。
  - `areka-P0-zorder-chain-residue` の A 系（wintf／host32 の常設の飢餓・間欠赤）の決着。本仕様はそこへ**間欠赤を足さない**責務のみ負う。
  - `areka-P0-charset-canon` が持つ `crates/shiori-host32-host/src/{shiori3.rs,client.rs}`（W13 同 crate 別ファイル・触れない）。
- **Adjacent expectations（隣接仕様への期待）**:
  - **上流 `areka-P0-emo2-conformance-e2e`（完了）**: 症状 D の機序と較正値（プロセス生存 20〜30 秒超・14 秒以上取り出さない）、6.11 の応答方向の旗と檻、Requirement 16.5 の申し送り（記録 §13.2 行 10）を提供済み。本仕様はその申し送りの引受先であり、完了 spec の文書は書き換えない。
  - **上流 `areka-host32-ipc-and-i686-build`（記憶）／完了 spec `shiori-host32-*`**: WM_COPYDATA 一本化・再入 RESPONSE・親窓の握手／送信パス／正規 clean shutdown を提供済み。本仕様はこれらを不透明に利用する。
  - **下流（将来）helper → areka 向きの自発通知を要する spec**: 本仕様の後は、ホスト窓宛に「相手が応答なしなら待たずに打ち切る」送り方で送っても、待機の長さに関わらず届くことを前提にできる。
  - **隣接 `areka-P0-zorder-chain-residue`（W14）**: 本仕様の檻は壁時計期限の飢餓に依存しない形で置き、A-2 の族（`SPIN_WAIT` 型の壁時計期限）を増やさない。
  - **併走 W13（`kanade-boot-talkdone-drop`・`charset-canon`）**: 共有ファイル 0 を保つ（上の Out of scope のファイルに触れない）。
  - **未確定で design が埋める項目**: 手空きを検出する周期の値（OS の応答なし判定の閾値に対する余裕を根拠に決め、定数の説明文に残す）・手空き通知の名前と置き場・握手時の heartbeat の扱い（常設の保守に統合するか無改変で残すか）・決定論テストの置き場（crate・ファイル）と駆動方法。

## Requirements

### Requirement 1: 待機中の応答性（応答なし判定に落ちない）

**Objective:** As 開発者, I want SHIORI アクターのスレッドが要求の無い待機中も窓のメッセージを取り出し続けること, so that 長い待機の後でホスト窓宛に送られた通知が OS の応答なし判定で打ち切られない

#### Acceptance Criteria

1. While SHIORI アクターが要求の無い待機中である, the SHIORI アクター shall ホスト窓宛のメッセージを継続的に取り出し、待機の長さに関わらず OS の応答なし判定に落ちない。
2. When 待機中に別スレッドからホスト窓へ同期送出（送出側が処理完了まで待つ送り方）のメッセージが送られる, the SHIORI アクター shall そのメッセージを処理し、送出側を上限時間の内に復帰させる。
3. When プロセス開始から 30 秒を超えた後に 20 秒以上要求が無い状態で、「相手が応答なしなら待たずに打ち切る」送り方の通知がホスト窓へ送られる, the SHIORI アクター shall その通知を受け取る（送出側で打ち切られない）。
4. When 待機中に inbox へ要求が届く, the SHIORI アクター shall 窓のメッセージ処理を挟んでも要求を取り落とさず、到着順に処理する。
5. The SHIORI アクター shall 待ちの形の変更によって要求の往復の所要時間に利用者が見える差（毎秒の往復・終了挨拶の後の解放）を生まない（非退行の確認は Requirement 2.8 の既存テストと Requirement 5 の実機走行で行う）。
6. While 要求の往復中（同期送出が上限時間の内でブロックしている間）である, the SHIORI アクター shall 既存の上限時間（要求方向・解放の ack・終了観測）で必ず復帰し、無限待機を作らない（既存どおり）。
7. While 要求の無い待機中である, the SHIORI アクター shall 死活監視（`backend.status()` の sticky 確認）を手空きの周期で継続する。**現状は死活監視が受信ループの内側にあり、仕事が止まると監視も止まる**（終了挨拶中の 17〜19 秒は一度も確認されない）——本受入基準はその欠陥の是正であり、手空き中に backend の異常終了が生じた場合も `ShioriDown` が一度だけ送られる。

### Requirement 2: アクター境界と握手・往復・停止の契約の不変

**Objective:** As kanade（呼び手）, I want 待ちの形が変わっても SHIORI アクターの受理規約と host32 の握手・往復・停止の観測結果が同じであること, so that 既存の運行表・決定論テスト・実機の系列がそのまま成立する

#### Acceptance Criteria

1. When 要求（GET／NOTIFY）が届く, the SHIORI アクター shall 同梱の返信先へちょうど 1 回応答し、成功・204・失敗の区別語彙を既存どおり返す。
2. When 停止指示（Close）が届く, the SHIORI アクター shall 即時に受信ループを抜け、接続資材を解放する（既存どおり）。
3. When 全ての送信端が drop される（inbox 切断）, the SHIORI アクター shall 正常に受信ループを終了する（既存どおり・窓のメッセージ待ちが切断の検出を妨げない）。
4. When メッセージ到達時に helper の異常終了が観測される, the SHIORI アクター shall 死活報告を一度だけ送る（sticky・既存どおり）。窓のメッセージの到達だけで死活報告の規約が変わらない。
5. When 握手（HELLO）が期限内に届かない, the host32 ホスト層 shall 既存どおり期限で復帰し握手失敗として報告する。When HELLO が届く, the host32 ホスト層 shall 既存どおり helper 窓を確定して往復を許可する。
6. While 要求の往復中である, the host32 ホスト層 shall single-in-flight を保ち、応答の再入受領・上限内未応答の Timeout 復帰・未握手の拒否を既存どおり行う。
7. When 正規 clean shutdown（unload → ack → 終了観測）が要求される, the host32 ホスト層 shall 既存の系列と結果（`Clean`／非 Clean／失敗の語彙）を返す。
8. The 本仕様 shall `crates/areka-kanade/src/shiori/real_tests.rs`・`crates/shiori-host32-host/` の既存テスト・`crates/shiori-host32-helper/` の既存テストを期待値を緩めずに緑のまま保つ（説明文の書き換えを除き無改変）。
9. The `ShioriBackend` トレイト shall 手空き通知を**既定実装（何もしない）つき**で追加し、既存の実装 10 個のうち host32 の `ShioriConnection`（`crates/areka-kanade/src/shiori/real.rs`）以外——`InProcBackend`（`crates/areka-ghost/src/shiori_inproc.rs`）およびテスト用の実装 8 個——を無改変で保つ。
10. The 手空き通知 shall backend 非依存の契約として定義される（「アクターが手空きのとき backend に処理の機会を与える」）。SHIORI アクター側のコードに Win32 固有の語彙（窓・メッセージポンプ等）を持ち込まず、`crates/areka-kanade/Cargo.toml` に Win32 API crate への依存を足さない（Requirement 7.6 の再掲）。

### Requirement 3: 応答方向の旗の維持（二重の守り）

**Objective:** As 開発者, I want 6.11 が入れた応答方向の扱いを本仕様が壊さないこと, so that 待ちの形と旗の 2 段で解放の失敗を防ぐ

#### Acceptance Criteria

1. The IPC 層 shall 応答方向（helper → ホスト）の送出に `SMTO_ABORTIFHUNG` を付けず、要求方向（ホスト → helper）には付ける、という 6.11 の区別を無改変で保つ。
2. The 本仕様 shall `crates/shiori-host32-helper/src/main_response_flavor_hung_cage_tests.rs` を無改変で緑のまま保つ（見出しの説明文の追随を除く・Requirement 6.1）。
3. The 本仕様 shall `crates/shiori-host32-ipc/` と helper の**コード**（送出の旗・wire・framing・型・テスト）に触れない。Requirement 7.1 が挙げる陳腐化した説明文（rustdoc コメント）2 か所の書き換えのみを例外とし、それ以外の差分を同 crate に持たない。

### Requirement 4: 決定論の檻（較正つき・x64 偽境界）

**Objective:** As 開発者, I want 「待機中も窓のメッセージを汲む」ことを実 32bit helper なしの決定論テストで固定し、直す前の構造で赤になることを較正として示すこと, so that 将来の変更でこの構造へ戻っても常設テストが検出する

#### Acceptance Criteria

1. The 決定論テスト shall x64 の偽境界で走り、実 i686 helper・実 SHIORI DLL・実ゴーストを使わない（steering: 常時テストは x86 を避ける）。
2. The 決定論テスト shall 本番の待ちの経路（SHIORI アクターの受信ループとホスト窓）を駆動し、待ちの形を写した複製で代替しない。
3. The 速い檻 shall 要求の無い待機中に別スレッドからホスト窓へ同期送出したメッセージが上限時間の内に処理されて復帰することを主張し、直す前の構造では上限まで待って赤になる（Requirement 1.2 の固定）。
4. The 遅い檻 shall 既存の hung 檻と同じ形（20 秒待ち → 往復① → 20 秒待ち → 往復②）で、往復②の時点（プロセス生存が猶予期間の外・スレッドが 20 秒要求を受けていない）に「相手が応答なしなら待たずに打ち切る」送り方でホスト窓へ送った通知が届くことを主張し、直す前の構造では赤になる（Requirement 1.3 の固定）。
5. The 遅い檻 shall 全体の上限を 90 秒以内とし、待ちも送出上限も有限で構造的に打ち切られる。
6. The 決定論テスト shall 判定を assert で行い（数値を印字するだけにしない）、赤のときは待ちの長さ・プロセス生存時間・届いた／届かなかった・送出失敗の回数を診断文に含める。
7. The 本仕様 shall 較正の証跡として、直す前の構造で速い檻と遅い檻が赤になった実行結果（コマンド・所要・赤の診断文）を検証記録に残す。
8. The 決定論テスト shall 壁時計期限の飢餓に依存する判定（短い期限で「発火しない」を主張する形）を持たず、OS の応答なし判定の条件（実時間そのもの）と十分な上限以外に時間の仮定を置かない。
9. The 本仕様 shall 既存の常設テスト全体（`cargo test --workspace`）に間欠的な赤を加えない。速い決定論テストは 5 秒以内に終わる。
10. The 決定論テスト shall 「要求の無い待機中に backend が異常終了したとき、次の仕事を待たずに `ShioriDown` が一度だけ送られる」ことを主張し（Requirement 1.7 の固定）、直す前の構造では送られずに赤になることを較正として記録する。
11. The 決定論テスト shall 速いもの・遅いものの双方が `cargo test --workspace` へ**無条件に含まれ**、環境変数ゲート・`#[ignore]`・feature ゲートのいずれによっても除外されない（2026-09-13 裁定）。遅いテストにより全体テストの壁時計が約 42 秒延びることを受容する（cargo はテストバイナリを直列に走らせるため、helper 側の既存の同型テスト 42 秒とは重ならず素直に加算される）。**この欠陥は 19 秒以上の沈黙でしか再現しないため、常設でなければ守れない**（steering: 使い捨ての場所に置いた検査は誰も守らない）。

### Requirement 5: 実機の非退行

**Objective:** As 開発者, I want 変更後も実機で終了挨拶の後の解放が正規に成立すること, so that M1 完成宣言で合格した項目 13 が退行しない

#### Acceptance Criteria

1. When 実 32bit helper と実ゴーストで、有界の自動終了を上限として付けた走行を行い、終了操作（キャラ窓への Ctrl＋左ダブルクリック）で終了を求めて終了挨拶を経て解放する, the 生ログ shall `event="unload_clean"` を 1 行含み `event="unload_failed"` を 0 行とする（e2e 手順書 §5.7 の読み方）。
2. The 実機の走行 shall 絶対パスで脳・ゴースト・バルーンを指定し、有界の自動終了を上限として付ける（走行が必ず終わる保証）。終了そのものは終了操作で求める——自動終了（`AREKA_APP_SMOKE_EXIT_MS`）は窓を直接閉じる強制終了の経路を通り終了挨拶を経ないため（2026-09-17 開発者裁定・e2e 手順書 A20 と同じ形）。
3. The 生ログ shall `event="helper_exited"` と `event="connect_failed"` を 0 行とする（待ちの形の変更が死活報告を誤発火させない）。
4. The 本仕様 shall 実機の走行結果（日時・コミット・コマンド・数えた行数）を検証記録に残す。0 行の結果も明示的に書く。

### Requirement 6: 失敗経路のログと終了（沈黙の失敗の禁止）

**Objective:** As 開発者, I want 待ちの機構そのものが失敗したときに沈黙せず記録して終わること, so that 原因不明の固まり・空回りが生まれない

#### Acceptance Criteria

1. If 手空きの間に backend の保守が失敗を返す, then the SHIORI アクター shall error レベルで記録し、無限待機も busy loop も作らずに既存の終了経路へ合流する。
2. The SHIORI アクター shall 手空きの検出そのものに、生成・準備を要する資材（イベントオブジェクト・追加スレッド等）を用いない。**採用した形では「待ちの機構の生成に失敗する」経路が構造上存在しないため、当初の受入基準 6.2（準備失敗時の ShioriDown 送出）は該当なしとして閉じる**（2026-09-13 裁定）。
3. The SHIORI アクター shall 手空きの保守を、既存のログ規約（`target`・`event` 欄）を保ったまま行い、正常時の手空き 1 回ごとにログを出さない（ログを汚さない）。

### Requirement 7: 文書の追随・編集集合・規模

**Objective:** As 開発者, I want 「待機中は pump しない」と述べる既存の説明文が変更後の姿に追随し、編集集合と規模が W13 の干渉台帳の内に収まること, so that 次にこの経路を触る spec が古い説明を読まず、併走 spec と衝突しない

#### Acceptance Criteria

1. The 本仕様 shall 「待機中は pump しない」「heartbeat は pump フェーズ専用」を述べる本番コードの rustdoc（`crates/shiori-host32-host/src/parent_window.rs` の module doc・`send_request` の説明・`crates/areka-kanade/src/shiori/real.rs` の `run_shiori_loop` の説明）と既存の檻の見出し（`main_response_flavor_hung_cage_tests.rs`「本番では shiori アクターが `recv()` で待つ」）を、変更後の姿に合わせて書き換える。書く前に file:line で裏取りする。 書き換え対象には `crates/shiori-host32-ipc/src/lib.rs` の `send_copydata_response` の rustdoc と `send_flavor_tests` の見出しにある「待機中はメッセージを取り出さない」の 2 か所を含む（コメントのみ・Requirement 3.3 の例外）。
2. The 本仕様 shall 完了 spec のアーカイブ（`.kiro/specs/completed/areka-P0-emo2-conformance-e2e/`）を書き換えない。
3. The 本仕様 shall 編集集合を `crates/areka-kanade/src/shiori/real.rs`・`crates/shiori-host32-host/src/parent_window.rs`・それらの兄弟テストファイル・本仕様の記録に限る。入力の到着でスレッドを起こす薄い包みが要る場合は、その置き場を設計で確定し、`crates/areka-kanade/src/schedule/`・`crates/shiori-host32-host/src/{shiori3.rs,client.rs}`・`crates/shiori-host32-ipc/`・`crates/shiori-host32-helper/` には触れない（W13 の共有ファイル 0）。ただし Requirement 7.1 が挙げる `crates/shiori-host32-ipc/src/lib.rs` の説明文 2 か所については、コメントのみの書き換えを例外として許す。**窓を作る決定論テスト**（速い窓テスト・遅い窓テスト）は kanade の既存の統合テストバイナリ（`crates/areka-kanade/tests/kanade.rs` 配下・`tests/kanade/idle_pump_test.rs`）に置き、その接続宣言 1 行・共有ヘルパ 1 ファイル（`tests/kanade/common/`）・既存 `tests/kanade/real_helper_test.rs` の窓生成へのロック取得 2 行を編集集合に含める（2026-09-13 設計ディスカッション裁定——窓を作るテストは既にそこに居り、lib テストバイナリに 41 秒を足さない）。
4. The 本仕様 shall 1 ファイル 1,000 行以下を本番ファイル・テストファイルの双方で保つ（`parent_window.rs` は 659 行・新しいテストは兄弟テストファイルまたは統合バイナリの新規ファイルへ置く）。
5. The 本仕様 shall 待ちの形の裁定と理由（不採用とした案を含む）を設計文書に残し、`doc/COMPAT_ARCHITECTURE.md` §8 に本仕様の節を追記する場合は自節のみとする。
6. The 本仕様 shall `areka-kanade` に Win32 API crate への直接依存を足さない（`crates/areka-kanade/Cargo.toml` は現状 `windows` 非依存）。窓のメッセージを汲む部品は host32 ホスト層（`crates/shiori-host32-host/`）が提供し、`real.rs` はそれを呼ぶ——「host32 型を import してよい唯一の場所は `real.rs`」という既存の境界を保つ。
