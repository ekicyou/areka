# 設計バリデーションレポート — areka-P0-shiori4-test-ghost

> 実施日: 2026-07-18 ／ レビュー対象: design.md（design-generated・requirements 承認済み）
> 手順: kiro-validate-design（Analysis → Critical Issues → Strengths → GO/NO-GO）
> 検証方法: design.md の全コードレベル主張を実コードと突合（下記「実コード突合結果」）

## Review Summary

設計は「既存シームへの変種追加＋自給 cdylib」という最小侵襲パターンで要件 1〜7 を完全トレースしており、コードレベルの主張（`ShioriWiring` 2 変種・connect closure 契約・`ShioriBackend` の Send 境界なし・IShiori ABI・codec・spine 駆動技法）はすべて実コードと一致することを確認した。D-1〜D-8 の設計決定は research.md に根拠ログ付きで決着済みで、実装可能性は高い。残る懸念は「cargo test --workspace が cdylib を locate 位置へ確実に生成するか」という単一の未実証仮定と、スナップショット実採取の実機依存プロセスの 2 点に集約され、いずれも設計内の緩和策（明示 panic・PROVISIONAL 先行）で管理可能な水準にある。

## 実コード突合結果（設計主張の裏取り）

| 設計主張 | 実コード | 判定 |
|---|---|---|
| `ShioriWiring` は `Helper`／`Custom` の 2 変種・connect closure は `FnOnce() -> Result<Box<dyn ShioriBackend>, String> + Send` | `crates/areka-ghost/src/runtime.rs:45-53,328-334` | 一致（match へ 1 arm 追加で済む形状） |
| `real_connect` が並置先例・`pub`・`file: None` は推測せず即失敗 | `crates/areka-ghost/src/shiori_wiring.rs:39-45`（`pub mod shiori_wiring` も lib.rs:29 で公開） | 一致（採取ハーネスの `Custom(Recorder(real_connect(..)))` 合成も可能） |
| `ShioriBackend` は Send 境界なし・`&mut self`・get/notify/unload/status | `crates/areka-kanade/src/shiori/real.rs:47-72` | 一致（`Recorder<B>`・`InProcBackend` の `!Send` 座が成立） |
| ABI: `CreateInstance`（生成＋load 融合）・`Get`（S_OK／`SHIORI_S_PENDING` の HRESULT 生返し）・`Notify`・host 4 メソッド・`SHIORI_E_UNKNOWN_TOKEN`／`SHIORI_E_PROPERTY_NOT_FOUND` | `crates/shiori-abi/src/interface.rs:66-159` | 一致（InProcHost の写像に必要な定数も実在） |
| `shiori_factory` export の正解見本 | `crates/areka/src/reference_brain.rs:269-282` | 一致（パターン雛形として踏襲可能） |
| codec `build_request`／`parse_response`・`ShioriRequest`・`Charset::Utf8`・`RequestError::Shiori(ShioriError::Parse)` 語彙 | `crates/shiori-host32-host/src/shiori3.rs:39-186`・`client.rs:106-123` | 一致（写像表 D-7 の語彙不変が成立） |
| `resolve()` は balloon に触れない（fixture balloon 非同梱の根拠） | `crates/areka-parsers/src/package/resolve.rs:8`（「balloon 系には触れない」明記） | 一致（要件 4.2「過不足なく」の判断は正当） |
| spine の駆動技法（`TickerMode::Disabled`・`RecordingSink` pub・`unique_temp_dir`） | `crates/areka-ghost/tests/ghost/spine_e2e_test.rs:123,235,592` | 一致 |
| emo2 shell 実物の流用元 | `crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/`（surfaces.txt 実在） | 一致 |
| areka-ghost の既存依存（`windows` workspace dep・`shiori-host32-host`） | `crates/areka-ghost/Cargo.toml:18,20` | 一致（feature 追加＋`shiori-abi` 追加のみで済む） |

## Critical Issues（最大 3）

### 🔴 Critical Issue 1: `cargo test --workspace` による cdylib の生成位置（uplift）が未実証

**Concern**: D-1 は「`cargo test --workspace` が `shiori4_testdll.dll` を自動ビルドし、`target/<profile>/` に置かれたものを locate する」と仮定するが、cargo の artifact uplift（`target/<profile>/deps/` → `target/<profile>/` へのコピー）は `cargo build` のルートユニットに対する挙動であり、`cargo test` 経由で dev-dependency としてビルドされた cdylib が**非ハッシュ名で `target/<profile>/` 直下に確実に現れるか**は cargo バージョン依存の未実証仮定である（deps/ 直下にのみ生成される可能性がある）。
**Impact**: 仮定が外れると、fresh checkout での `cargo test --workspace` が常設 e2e で明示 panic し、要件 1.2／5.4 の「手動プリビルド段を要さない」が構造的に破れる（常設ゲートが恒常赤）。
**Suggestion**: 実装の最初のタスクで uplift 挙動を実証する spike を置く（tasks.md の先頭）。併せて `locate_built_test_dll` を「`target/<profile>/` → 不在なら `target/<profile>/deps/` を `shiori4_testdll*.dll` glob で探索（最新 mtime 優先）」の 2 段フォールバックに設計拡張しておけば、cargo の uplift 挙動差に対して構造的に免疫になる（明示 panic は両段不在時の最終防衛線として維持）。
**Traceability**: 要件 1.2・5.4
**Evidence**: design.md「主要設計決定 D-1」「fixture 組立と DLL locate」／research.md §7.1 D-1・§7.3（`-p` 単独時のみをリスク登録し `--workspace` 時は無条件成功と仮定している）

### 🔴 Critical Issue 2: スナップショット実採取（実機 env-gate）が DoD の隠れブロッカー・NOTIFY 系イベントの採取不能が要件 2.2 と暗黙に緊張

**Concern**: (a) 常設ゲートは PROVISIONAL 手書き応答で先行 green になる設計だが、要件 2.2 の充足（実 pasta 採取のゴールデンスナップショット）は実機＋i686 成果物＋`HOST32_PASTA_DLL` の env-gate セッションに依存し、差し替え時には I1 の期待 cue 列定数の再導出も必要になる。(b) 採取点は `ShioriBackend` seam（Recorder）ゆえ **NOTIFY 応答は観測できない**（`notify()` は応答値を破棄する片道契約）。要件 2.2 の正典 6 イベントのうち NOTIFY 発火のもの（`OnInitialize` 等）はスナップショット化されず 204 replay になるが、この narrowing は design.md 内で「kanade が GET する正典 ID に一致させる」と 1 行示唆されるのみで、要件 2.2 との対応関係が明文化されていない。
**Impact**: (a) 採取タスクを独立化しないと「PROVISIONAL のまま spec 完了」の逸脱リスク（要件 2.2 未充足での DoD 通過）。(b) 実装者が正典 6 イベント全部のスナップショットファイルを期待して混乱する／レビュー時に要件逸脱と誤認される。
**Suggestion**: tasks.md で「実採取＋凍結コミット＋期待 cue 列更新」を DoD 直前の独立タスクとして明記し（実機サインオフの既存流儀 [[areka-real-machine-signoff-bounded-auto-exit]] を踏襲）、design.md の narrowing（NOTIFY 応答は wire 上破棄されるため snapshot 対象は GET イベントのみ＝要件 2.2 の「固定の決定論応答」は NOTIFY には 204 受領として充足）を採取タスクの受け入れ条件に転記する。設計本文の修正は不要（タスク分解での明文化で足りる）。
**Traceability**: 要件 2.2・2.6
**Evidence**: design.md「snapshot 採取ハーネス」「SnapshotTable」Risks 欄／research.md §7.3「スナップショット実採取前の開発期間」

（第 3 の critical issue なし——上記 2 点以外に成功を有意に脅かす欠陥は検出されなかった）

## Design Strengths

1. **実コードと完全に噛み合う最小侵襲設計**: 本レビューで突合した設計主張 10 項目がすべて実コードと一致し、変更面は「新規葉 crate＋areka-ghost の 1 モジュール＋1 variant＋テスト支援」に厳密に局在する。特に D-2（Option B・bin carve-out 回避）は「正規シームは既に areka-ghost に居る」という観察に基づく決定で、要件 3.6／7.1 の「正規実装・M2 前方整合」を新規抽象なしで満たしている（steering の投機的抽象禁止・影響半径最小化に合致）。
2. **`Recorder` の単一装置化と構造的安全性**: 要件 1.4（交信列 assert）と要件 2.6（スナップショット採取）を backend 非依存の単一デコレータで賄う一般化は、fake／InProc／実 pasta の 3 者を同一手口で観測でき将来の観測需要にも開いている。また FreeLibrary UB（DLL 実装 COM 参照の解放順序）をフィールド宣言順で構造的に固定する手当・log-first 写像表・明示 panic の単一経路化は、プロジェクトのテスト規律（判断分岐のみ檻・決定論必達）に忠実である。

## Final Assessment

**Decision: GO**

**Rationale**: 要件 1〜7 の全 AC が設計要素へトレースされ、コードレベルの前提はすべて実コードで裏取りできた。残る 2 つの critical issue はいずれも設計の骨格を変えず、タスク分解（spike の前置・採取タスクの独立化・locate のフォールバック拡張）で吸収可能な実装フェーズの管理事項である。

**Next Steps**:
1. 設計ディスカッションで Issue 1（locate 2 段フォールバック）と Issue 2（採取タスクの DoD 位置づけ・NOTIFY narrowing の明文化）の扱いを確認
2. `/kiro-spec-tasks areka-P0-shiori4-test-ghost` でタスク生成——その際 Issue 1 の uplift 実証 spike を先頭タスク、Issue 2 の実採取タスクを DoD 直前タスクとして編成する
