# 設計バリデーションレポート: areka-P0-host32-lifecycle

- **日付**: 2026-07-05
- **フェーズ**: design-validation（kiro-validate-design・非対話実行）
- **入力**: requirements.md／design.md／research.md／spec.json（language: ja）／steering（logging.md ほか）
- **検証方法**: 設計書の全主要主張を実コードと突合（`shiori-host32-ipc`／`-host`／`-helper` ソース実地確認）

## Review Summary

設計は「既存 seam の合成による増分」という性格を正確に捉え、新規機構を `HelperLifecycle`（監視の器）・`classify_failure`＋`LifecycleReport`（統一報告語彙）・helper UNLOAD 結線（正規正常終了経路）の 3 点に絞り込んでいる。設計書の実コード立脚主張は**全件検証済みで正確**であり、全 39 受入基準（1.1〜7.7）が traceability 表に対応する。確定済み決定（tracing 採用・実 helper への正規経路増設）はいずれも設計に忠実に反映されており、実装可能性は高い。

## 実コード突合結果（設計主張の裏取り）

| 設計主張 | 実コード | 判定 |
|---|---|---|
| `MsgTag::Unload`(=5) は凍結 ipc に定義済み・「下流が結線」と明記 | `crates/shiori-host32-ipc/src/lib.rs` L53-54「pasta.dll アンロード要求（本ユニット未処理・下流で結線）」`Unload = 5`・`try_from_u32` L74 | ✅ 一致 |
| `poll_exit`／`poll_exit_kind` は try_wait ベース非ブロッキング | `process_host.rs` L274-292（I/O Err は稼働中扱い＝設計の Implementation Notes 記載どおり） | ✅ 一致 |
| `ExitKind{Clean/Abnormal(i32)/Terminated}`・`classify` 純関数 | `process_host.rs` L117-122 | ✅ 一致 |
| `terminate` は `InvalidInput`→`Ok` の冪等実装 | `process_host.rs` L189-196 | ✅ 一致 |
| `RequestError` の `Ipc` は意図的に `#[from]` 無し（timeout 区別保持） | `error.rs` L78-99（型 doc に明記） | ✅ 一致 |
| `map_send_error` の手動振り分け（load-bearing・テスト済み） | `client.rs` L193 以降＋ CRITICAL テスト L245 | ✅ 一致 |
| `pump_until_hello_or` の「フラグ＋自窓 PostMessageW(WM_NULL) 起こし」実証済みパターン | `parent_window.rs` L258-280（heartbeat 実装） | ✅ 一致 |
| helper は正常終了経路を持たない・`MessageLoop::run(\|_,_\| Forward)` 無停止・Unload は現状 `IgnoreKnown` | `helper/src/main.rs` L458・L109（`Ok((tag,_)) => IgnoreKnown(tag)`） | ✅ 一致 |
| `HelperShared`／`send_copydata`／`REPLY_TIMEOUT`／`HelperMessageWindow` の既存シンボル | `helper/src/main.rs` L131／L43／L55／L328 | ✅ 一致 |
| steering `logging.md` が `shiori-host32-*` を tracing 消費ライブラリと明示列挙 | `logging.md` L14-15（workspace `Cargo.toml` L29 に `tracing = "0.1"` 定義済み） | ✅ 一致 |
| ack `[1]` 再入受領・LOAD ack 同型 | `helper/main.rs` L308（LOAD ack 実装）＋`parent_window.rs` RESPONSE 再入受領 | ✅ 一致 |

**確定済み決定の遵守確認**:
- ✅ ログは `tracing`（steering `logging.md` 準拠）——ホスト `lifecycle.rs` に `error!` 配置表 4 点＋`Err` 戻り値。`eprintln!` 許容案の再燃なし。
- ✅ 正規正常終了経路は**実 helper**（`shiori-host32-helper` main.rs への TriggerUnload 増設・R5.6）——stand-in `exit(0)` 代替の再燃なし。設計は「実運転コードの正規経路」と明記。

## Critical Issues（最大 3・design discussion への供給）

🔴 **Critical Issue 1**: `lifecycle_cyclic_e2e.rs` 内の windowed テスト 2 本と 1 窓制約の並行実行衝突
**Concern**: 設計は「窓が要る試験は 1 バイナリ 1 windowed-test（error_paths.rs 踏襲）」と宣言しつつ、`lifecycle_cyclic_e2e.rs` に windowed テストを 2 本置く（項目 8 cyclic＋項目 10 pasta 追験）。cargo test は同一バイナリ内の `#[test]` を並列スレッドで走らせるため、`HOST32_PASTA_DLL` 設定時に親 message-only 窓が同時 2 組生成され得る（既知制約: 2 組目失敗）。
**Impact**: CI（env 未設定＝pasta は早期 return・窓 0）では顕在化しないが、開発者の実 pasta 追験 run が非決定的に失敗し得る（R6.1 の confidence 検証が flaky 化）。
**Suggestion**: 実 pasta 追験の実行手順（Testing Strategy の PowerShell 手順）に `--test-threads=1` を明記する、または pasta 追験を別バイナリへ分離する。既存 `shiori_request_e2e.rs` も同構造ゆえ、精度としては「先行ユニットから引き継いだ既知の綾」であり設計自体の欠陥ではない——tasks 生成時に手順へ 1 行足せば閉じる。
**Traceability**: R6.1／R3.1（＋1 窓制約は R7 系の検証再現性）
**Evidence**: design.md「File Structure Plan」「Testing Strategy 項目 8／10」「Existing Architecture Analysis: 1 窓制約」

🟡 **Critical Issue 2**（低重大度・記録目的）: helper 側新設失敗経路（UNLOAD ack 送出失敗）の観測が `eprintln!` のみ
**Concern**: R7.6 は「本仕様の失敗経路」に `error!`＋`Err` を求めるが、helper UNLOAD アーム手順 4 の ack 送出失敗は `eprintln!` 観測のみ（設計は「helper ログ機構刷新は Out of Boundary・既存流儀踏襲」と明示スコープ外化）。steering `logging.md` L17 は helper 実行体を tracing-subscriber アプリとして列挙している。
**Impact**: 実害は限定的——親は ack timeout（`ShutdownError::Unload`＋ホスト側 `error!`）で必ず検出するため silent failure にはならない。ただし steering との乖離が暗黙のまま残る。
**Suggestion**: 設計変更は不要。design discussion で「helper の tracing 化は将来ユニットへ送る意図的逸脱」として 1 行明文化（または backlog 化）すれば足りる。確定済み決定（host 側 tracing 採用）の再議論ではない。
**Traceability**: R7.6（helper 側適用範囲の解釈）
**Evidence**: design.md「helper UNLOAD 結線: Responsibilities 手順 4」「Out of Boundary: helper 側ログ機構の刷新」／steering logging.md L17

（3 件目なし——上記以外に成功を有意に脅かす問題は検出されなかった）

## Design Strengths

1. **実コード 1:1 立脚の増分設計**: 全主要主張（凍結 `MsgTag::Unload` の消費・seam の非ブロッキング/冪等性・`map_send_error` の区別保持・`pump_until_hello_or` の posted-wake パターン）が実コードと一致し、新規 I/O 経路をほぼ持たない。RefCell 再入規律（borrow 非保持 drop）・再入 ack・1 byte ack 契約など、先行ユニットで実証済みのパターンのみで正規正常終了経路を組み立てており、実装リスクが構造的に低い。
2. **凍結遵守と区別保持の両立**: `classify_failure`（純関数・突合表全行単体テスト可）＋`LifecycleReport`（凍結 `RequestError` を所有内包・二軸保持）の B＋C 折衷は、R7.2（消費のみ）と R2.4（不潰し）を同時に満たす明快な形。「終了検出が他のすべてに優先」の不変条件と `Send` 静的 assert の明文化も下流 kanade の消費契約として堅い。

## Final Assessment

**Decision: GO**

**Rationale**: 設計の実コード立脚主張は全件検証で正確、全 39 受入基準が traceability 済み、確定済み決定（tracing／実 helper 正規経路）を忠実に反映しており、残る 2 件はいずれも設計変更を要しない低重大度（手順 1 行追記・逸脱の明文化）である。

**Next Steps**:
1. design discussion で Issue 1（pasta 追験の直列実行手順）と Issue 2（helper eprintln! 逸脱の明文化）を確認・記録
2. `/kiro-spec-tasks areka-P0-host32-lifecycle` で実装タスク生成へ進む
