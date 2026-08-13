# Design Validation Report: areka-P0-ghost-window-zorder

> 実施: 2026-08-11（kiro-validate-design・非対話実行）
> 対象: `design.md`（design-generated・finalized）／`requirements.md`（approved）／`research.md` §8-9／steering
> 検証方針: 設計の主張は file:line で現物照合する（steering 規律「doc-claims-need-file-line-verification」）

## Review Summary

設計は既存アーキテクチャの 4 先例（政策コンポーネント・entity 参照宣言・判断の純関数化・`Added<WindowHandle>` 検知）に完全に乗り、案 D 宣言層＋案 A 本線／案 B フォールバックの二層構造で要件 8 領域・全 45 受入基準をトレースしている。コードアンカーは全数照合で一致し、research.md §8 のクローズ済み裁定（案 A 本線・ペア浮上と位置分離・2.6 シーム・5.7/5.8/5.9 破棄意味論）も忠実に反映されている。ただし、フォールバック分岐まわりの契約に未指定・不整合が 3 点あり、設計ディスカッションでの解消を要する。

## アンカー実測照合（現物スポットチェック）

| 設計の主張 | 現物 | 判定 |
|---|---|---|
| `enqueue_window_set_pos` が `SWP_NOZORDER`＋`hwnd_insert_after: None` をハードコード | `crates/areka/src/placement/follow/window_move.rs:452-544`（flags `:484-487`・`None` `:496`・`bypass_change_detection` `:501`） | 一致 |
| `WM_ACTIVATE` は非活性化のみ処理（活性化は早期 return） | `crates/wintf/src/ecs/window_proc/keyboard.rs:119-169`（早期 return `:129-131`） | 一致 |
| `ZOrder` 語彙完備・`Send/Sync` 手動 impl・公開エクスポート | `crates/wintf/src/ecs/window/window_pos/mod.rs:25-39`（`:49-50` impl）・`crates/wintf/src/ecs/mod.rs:49` エクスポート | 一致 |
| `SetWindowPosCommand` が `hwnd_insert_after` を搬送済み・flush は warn 継続 | `crates/wintf/src/ecs/window/command.rs:117-125`（`:124`）・`new()` 第 7 引数 `:141`・warn 継続 `:195-203` | 一致 |
| spawn はバルーン→キャラの順・`OnDragEnd` 後付け・`WS_EX_TOPMOST` 無し | `crates/areka/src/placement/spawn.rs:217-328`（balloon `:234`・char `:262`・後付け `:312-314`・`window_style()` `:332-337`） | 一致 |
| `GhostWindows`／markers／`on_remove` hook のペア同時消滅 | `spawn.rs:163-201`／`:107-109`／`:122-142` | 一致 |
| `LibWindow::new_ex` は owner を受け取らず・`Window.parent`＝`SetParent`（owner と非等価の明記） | `crates/wintf/src/runtime/window_factory.rs:137`・`:149-165`（非等価コメント `:154-157`） | 一致 |
| wintf → areka import 禁止・WM_CLOSE は despawn→registry drop 駆動 | `crates/wintf/src/ecs/window_proc/lifecycle.rs:34-35`・`:97-116` | 一致 |
| `is_self_initiated()` エコー判定既設・`WM_WINDOWPOSCHANGED` は z を見ていない | `crates/wintf/src/ecs/window_proc/window_pos.rs:36-`（`:44`） | 一致 |
| click_through 結線の同居先 | `crates/areka/src/main.rs:687-693`（`FrameFinalize`・`register_ghost_windows_click_through`） | 一致（:690-693・±1 行の微差） |
| 配送テスト群の追加先 | `crates/wintf/src/ecs/window_proc/mod.rs:76-254`（`WM_ACTIVATE` 配送 `:70`） | 一致 |

research.md §8 のクローズ裁定の反映: §8-1（案 A 本線／案 B フォールバック）→ Overview・Migration Strategy に反映済み。§8-6（ペア浮上・位置分離＝要件 1.6）→ `PairFix` 型に座標フィールドが存在しない構造的保証＋`SWP_NOMOVE|SWP_NOSIZE` 固定。§8-9（2.6 シーム）→ D5 `ReassertZOrder` insert 契約として Boundary Commitments に相互登記。§8-4（5.7/5.8/5.9）→「破棄経路」節（owner 切離し＋G8）で消化。要件 8 領域は Traceability 表で全 45 受入基準（1.1〜8.5）が設計要素に対応付いている。

## Critical Issues

### 🔴 Critical Issue 1: 案 A＋raise assist（G7 FAIL）時の `RaisedBelow` トリガに供給者が居ない

**Concern**: 維持系は「G7 FAIL 時のみ `RaisedBelow` を処理する」と規定するが、トリガを供給できる唯一の機構（`WM_WINDOWPOSCHANGED` z 変化検知）は「案 B 発動時のみ」（Modified Files）かつ発火条件 `strategy == ExplicitMaintenance`（B2 節）と明記されている。`OwnerLink { raise_assist: true }` では誰も `RaisedBelow` を挿入できない。さらにトリガ命名も不整合——B2 節は「z が動いた窓がバルーン側なら `RaisedAbove`」とするのに、G7 raise assist は「バルーン浮上」を `RaisedBelow` トリガと呼んでいる（G6/G7 表・System Flows 案 A 確立フロー 3）。
**Impact**: G7 FAIL 分岐（要件 1.3「バルーン活性化でキャラを直後へ」）が実装不能仕様になる。ゲート後に発覚すると分岐設計のやり直しになる。
**Suggestion**: ⑴ z 変化検知の発火条件を「`ExplicitMaintenance` または `OwnerLink { raise_assist: true }`」へ拡張し、raise assist 時は検知系も有効化されることを Modified Files・B2 節に明記する。⑵ `PairTrigger` の `RaisedAbove`／`RaisedBelow` の意味（どちらの窓が動いたか）を 1 箇所で定義し、G7・System Flows の記述を統一する。
**Traceability**: 要件 1.3・2.3（G7 FAIL 時）
**Evidence**: design.md「Plan A 実機可否ゲート」G7 行／「System Flows > 案 A の確立フロー」3／「Modified Files」window_pos.rs 行／「wintf / window_proc > WM_WINDOWPOSCHANGED z 変化検知」

### 🔴 Critical Issue 2: `decide_pair_fix` の可視性が案 B（B3）の呼出契約と矛盾

**Concern**: `decide_pair_fix`（および `PairObservation`／`PairFixDecision`）は `pub(crate)`（wintf 内限定）と宣言されているが、B3 節は「areka 側ヘルパ（維持系と同じ観測→`decide_pair_fix` 呼出）で z 意図を求めて渡す」と規定する。areka は別クレートであり `pub(crate)` の関数・型を呼べない。
**Impact**: 案 B へフォールバックした瞬間に B3 の契約が成立せず、実装時に場当たりの可視性変更か判断ロジックの複製（純関数一元化の毀損）を誘発する。
**Suggestion**: どちらかに確定して明文化する——⑴ `decide_pair_fix` と入出力型を `pub` で公開（案 B 発動時のみ公開を広げる注記でも可）、または⑵ 観測組立＋判断を包んだ wintf 側 pub ヘルパ（例 `compute_pair_z_intent(world, entity) -> ZOrder`）を契約点とし、areka はそれだけを呼ぶ。
**Traceability**: 要件 2.1〜2.5（案 B 時）
**Evidence**: design.md「Service Interface（純関数）」の `pub(crate)` 宣言／「areka / placement > funnel z 引数（案 B・B3）」

### 🔴 Critical Issue 3: `sink-observed`（WM_ACTIVATE 非活性化枝）の観測タイミングが S3 判定を偽 FAIL させうる

**Concern**: 要件 4.4/7.5 の証跡を `WM_ACTIVATE(WA_INACTIVE)` 処理中の `GetWindow` 実測で残す設計だが、このメッセージは活性化トランザクションの途中に届き、新前面窓の raise（z 移動）が完了する前でありうる。その瞬間の走査は「ゴースト全窓が前面窓より背面」をまだ満たしておらず、実機サインオフ S3 が実装欠陥なしでも FAIL しうる。
**Impact**: 実機ゲート／サインオフの判定信頼性（要件 7.5）が下がり、「指令は出たが効かない」型と観測タイミング起因の偽陽性を切り分けられなくなる——本設計自身が最重要視する 2 値レコードの実測側が汚れる。
**Suggestion**: `sink-observed` を WM_ACTIVATE 内の即時走査ではなく、非活性化を記録して次フレーム（または `ReassertZOrder` の `pending_verify` と同じ次巡機構）で走査する遅延観測にする。少なくとも S3 の PASS 判定は「非活性化後の最後の観測レコード」で行うことを Testing Strategy に明記する。
**Traceability**: 要件 4.4・7.5
**Evidence**: design.md「WM_ACTIVATE 沈降観測」／「Real-Machine Signoff」S3／research.md §8-10

## Design Strengths

1. **未知を全てゲートに封じ込めた分岐構造**: 案 A の最大リスク（`GWLP_HWNDPARENT` 後付け×WUC 合成の実挙動）を「設計時に解決すべき不確定」ではなく「最初の実装タスク＝G1〜G8 実機ゲート」として構造化し、FAIL 側の帰結（案 B 切替・owner 確立系のコード撤去・空虚な保険を残さない）まで判定表で確定している。要件 5.6 の実装形がゲート表そのものになっている点は、要件と設計の対応として秀逸。
2. **構造的保証への一貫した志向**: 要件 1.6（位置不変）を「`PairFix` 型に座標フィールドが存在しない」というコンパイル時保証に、要件 3.4（2 窓限定）を `InsertAfter` の性質に、要件 4.1（受動沈降）を「維持系はトリガ駆動のみ」に還元しており、檻でなく型と構造で不変条件を守る本リポジトリの規律（判断分岐のみテスト・配線は再テストしない）に合致する。診断ログの「指令＋実測を同一行」も過去の誤診への直接の対策として根拠が明確。

## Final Assessment

**Decision: GO**（条件: 上記 3 件を設計ディスカッションで解消し design.md へ反映すること）

**Rationale**: アーキテクチャ整合（依存方向・既存 4 先例・スレッド制約）に欠陥はなく、全 45 受入基準のトレースとクローズ済み裁定の反映が確認でき、コードアンカーは全数一致した。3 件の critical issue はいずれもフォールバック分岐・観測タイミングの契約明文化レベルであり、本線（案 A・G7 PASS）の実装可能性と全体構造を揺るがさない——再設計ではなくディスカッションでの追記・修正で解消できる。

**Next Steps**:
1. 設計ディスカッション（kiro-design-discussion）で Critical Issues 1〜3 を裁定し design.md を改訂
2. 改訂反映後 `/kiro-spec-tasks areka-P0-ghost-window-zorder` でタスク生成（最初のタスク＝案 A 実機ゲートの編成を維持）
