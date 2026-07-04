# 設計バリデーションレポート — areka-P0-actor-foundation

> 実施日: 2026-07-03 / 対象: design.md（FINALIZED）vs requirements.md（R1〜R8・32 基準）+ steering
> 実施形態: kiro-validate-design（非対話・design-review.md プロセス: Analysis → Critical Issues → Strengths → GO/NO-GO）

## レビューサマリ

新設独立クレート `crates/areka-actor`（純粋層 `spawn`/`reply`＝std のみ＋UI ブリッジ `ui`＝executor+event-listener・既存クレート改変ゼロ・新規 crates.io 依存ゼロ）という構成は、要件 32 基準を完全トレースし、steering（tokio 禁止・thiserror/tracing 共通規約・MTA/render 固定・anti-framework）と矛盾なく整合する。設計の核心主張は実コードで裏付けを検証済みであり、実装可能性は高い。残る不確実性は toy(b) の実行機構の一点に局在し、フォールバックも設計内に用意されている。

## 検証結果（コードベース照合）

1. **「UI ブリッジは wintf 本体なしで成立」の主張 — 裏付け確認**。`crates/shiori-host32-host/Cargo.toml` が `wintf-winmsg-executor` + `event-listener` へ wintf 非依存で直接依存する本番前例は実在。設計が使用する API（`spawn_local`・`JoinHandle`・`MessageLoop::run(|msg_loop, _msg| .. FilterResult::Forward)`・`msg_loop.quit()`）は `tick_bridge.rs`／`controller.rs`／`parent_window.rs`／pilot examples の実使用で全て確認できた。別スレッド notify → executor waker → pump 起床の経路も `VsyncEventBridge`→`AsyncTickTask` で本番実証済み。
2. **toy(b) の bounded pump（R8.2/8.3）— 前例確認**。`ParentMessageWindow::pump_until_hello_or`（heartbeat＋deadline 再評価＋`quit()`）は cargo test 内で bounded 実走する実テスト（`window_tests`）を持ち、機械 pass/fail・CI 可能性の主張は成立する。ただし後述 Issue 1 の組合せ未実証が残る。
3. **停止意味論の一貫性 — 確認**。Close＝即時停止（積み残し破棄）・全 Sender drop＝正常終了・reply Sender drop＝要求側 `Err` 観測が、Overview／Boundary Commitments／DD-6／停止経路フロー図／`ui` モジュール（Break→Receiver drop）／toy(a) ケース(iii) まで単一の意味論で矛盾なく貫通している。
4. **要件カバレッジ — 32/32**。Traceability 表は R1(5)+R2(5)+R3(6)+R4(5)+R5(3)+R6(2)+R7(3)+R8(3)＝32 基準を全て設計要素へ対応付けており、欠落なし。文書規約系（2.5/5.1/5.2 等）も lib.rs rustdoc＝規約正本として担い先が明確。

## Critical Issues（最大 3）

🔴 **Critical Issue 1**: toy(b) の三点組合せ（`spawn_local`＋`MessageLoop::run`＋`PostThreadMessageW`）が in-repo 未実証
**Concern**: 各要素は個別に実証済みだが、同時使用の前例がない。host-32 は `MessageLoop::run` を async タスクなしで回し、pilot は `spawn_local`＋`block_on`。「`MessageLoop::run` 単独で spawn_local タスクが poll されるか」「thread message（hwnd なし）が executor filter へ届くか」の 2 点は設計の推論であり、後者のみ設計がリスク＋フォールバック明記、前者は未言及。
**Impact**: toy(b) が実装後半で赤になると R8.2 の観測戦略ごと手戻りし得る（設計全体ではなく試験機構の局所差し替えで収まる見込みだが、順序を誤ると波及）。
**Suggestion**: tasks 生成時に「toy(b) の最小 spike（echo 1 本の組合せ確認）」を実装系タスクの先頭へ置く。不成立時のフォールバックは既に実証済みの組合せ——(a) `block_on`(完了 future)（pilot 前例）または (b) message-only 窓＋`PostMessageW`（parent_window 前例）——へ公開 API 不変で局所差し替え。
**Traceability**: R8.2, R8.3
**Evidence**: design.md「toy tests」節 Risks／DD-5・research.md §7.2, §10

🔴 **Critical Issue 2**: `spawn_ui` の「UI スレッドから呼ぶ」前提が型で強制されず、誤用時挙動が executor 依存のまま
**Concern**: 設計は rustdoc 禁止＋toy(b) 正用法検証のみで担保するが、誤用時（非 pump スレッドからの呼出）に panic するのか静かに死ぬのかを設計が確認していない。
**Impact**: 下流結線（emo-present／ghost-setup）の配線ミスが「メッセージが届かないだけ」の診断困難な形で現れ得る。基盤ユニットの誤用失敗は早期・大声で落ちるべき。
**Suggestion**: 実装時に誤用時の executor 挙動を一度確認し、静かに失敗する場合は `spawn_ui` 冒頭で検出可能な前提違反を明示 panic（アクター名入りメッセージ）へ写像する。設計変更不要・実装ノートで足りる。
**Traceability**: R4.1, R4.4
**Evidence**: design.md「ui」節 Risks・research.md §10

（第 3 の critical issue なし。`UiSendError<M>` の derive 境界等は research §10 で解決策込みで把握済みの軽微事項）

## Design Strengths

1. **主張の全てに写経元がある実証駆動設計**: 名前付き spawn／store→notify／listen-before-work／bounded pump の各機構が既存本番コード（tick_bridge・monitor・controller・parent_window・task_pool）へ行単位でトレースされ、「wintf 不改変で新設クレート内完結」という最重要主張も `shiori-host32-host` の依存前例で裏付けられている。設計リスクが「新規発明」でなく「既知パターンの組合せ確認」に縮退している。
2. **停止意味論の単一化と追加コードゼロの切断シグナル**: Close＝即時停止・積み残し破棄・reply drop＝`Err` 観測を std mpsc の drop 意味論に委ね、要件→DD→フロー図→toy 試験まで一貫。graceful 停止を送信側運用へ押し出す境界設定は R7（anti-framework）と噛み合っており、基盤の最小性が構造的に保たれている。

## Final Assessment

**Decision: GO**

**Rationale**: 要件 32 基準の完全トレース・steering 全制約との整合・核心主張のコードベース裏付けが揃い、残存リスクは toy(b) の試験機構一点に局在して実証済みフォールバックを持つ（設計本体の手戻りリスクではない）。

**Next Steps**:
1. 設計ディスカッション（kiro-design-discussion）で Issue 1/2 の扱い（tasks への spike 先頭配置・誤用時 fail-fast 方針）を確認
2. `/kiro-spec-tasks areka-P0-actor-foundation` でタスク生成（Issue 1 の spike をタスク順序に反映）
