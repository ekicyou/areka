# 設計バリデーションレポート — areka-P0-balloon-face-cue

> 実施日: 2026-07-12 ／ 実施形態: 非対話（kiro-validate-design subagent・レポートはディスクへ永続化）
> 入力: spec.json（language=ja・phase=design-generated）／requirements.md（確定）／design.md（確定）／research.md（ギャップ分析＋設計決定記録）／brief.md／steering（product・tech・structure・roadmap・logging）
> 検証方法: design-review プロセス（Analysis → Critical Issues → Strengths → GO/NO-GO）＋**設計が主張する既存シンボルの実ツリー突合**（Grep/Read によるスポットチェック）

---

## 1. レビューサマリ

本設計は、`\b`（ブラケット形・裸形・`-1` 非表示）を `\s` と完全対称の第一級 cue 語彙として 5 エンジン（parsers→dola→sakura→seriko→観測）へ additive に貫通させるもの。**設計が根拠とする既存シンボルの主張（行番号レベル）を実ツリーで全数スポットチェックした結果、齟齬ゼロ**であり、要件 R1–R7 の全 AC が Traceability 表で具体コンポーネント・契約・テストへ写像されている。ブロッキングとなる Critical Issue は検出されず、実装準備完了と判断する。

## 2. 実シンボル突合（スポットチェック結果 — 全項目一致）

| 設計の主張 | 実ツリー確認結果 | 判定 |
|---|---|---|
| lexer.rs `SHORTHAND_WORDS = &['w']`・`Token::WaitShorthand(u8)`（`pub(crate)` 内部型） | `lexer.rs:47`・`:37` で確認。shorthand 判定規則（1 桁数字・直後 `[` なら正準タグ優先）は `:132-138` に実在し、`'b'` 追加の一般化がそのまま乗る | 一致 |
| contract.rs `cue_target_of`（catch-all 無し・`Custom`→`None`） | `contract.rs:63-73` で確認。全 variant 分類テスト `cue_target_of_classifies_every_variant`（`:87`）も実在＝設計の「分類テスト拡張」の土台あり | 一致 |
| dola `CueCommand` 7 variant・`Emote{key:String}`・`cue_command_seven_variants` テスト | `command.rs:121`・`:127`・`:230` で確認。8 番目 variant 追加＋テスト改名の設計記述は正確 | 一致 |
| seriko `SurfaceTarget{Show(u32),Hide,Unresolved}`（`resolve_balloon_key` の戻り型として再利用） | `resolve.rs:13` で確認。`-1`→Hide／数値→Show／他→Unresolved の判定ロジック先例も `resolve.rs:44-70` に実在 | 一致 |
| seriko `ScopeStates.apply` 冪等ガード（未知 scope への Hide 一度発行含む） | `state.rs:90-118`＋テスト `:343`（未知 scope Hide→Changed）で確認。`apply_balloon` の「鏡映実装」が成立する | 一致 |
| seriko actor 内側 match の catch-all（`:205`）＝コンパイラ非強制 → テスト檻で補償 | `actor.rs:205-211` で確認。設計のリスク認識（E2E＋`handle_message` 同期単体＋`capture_logs`）は正確。`capture_logs`／`capture_logs_flow` は `actor.rs` テストに実在 | 一致 |
| `emit_display` 単一発行点・`DisplayCommand{Show,Hide}`（`#[non_exhaustive]` 無し）・`MockSurfaceOutput` | `actor.rs:227`・`output.rs:21-30`・`:49` で確認 | 一致 |
| drive.rs `spawn_talk(start, done, surface_sink, text_sink)`・Balloon→`text_sink`・`None`→error!+skip | `drive.rs:56-60`・`:218`・`:219-222` 相当で確認。E2E の同期チェーン（done.recv→SerikoSink drop→disconnect→join）は既存 2 停止経路のみで成立 | 一致 |
| ghost sink.rs `command_kind`（7 arm・catch-all 無し）＝強制点 2 | `sink.rs:56-64` で確認 | 一致 |
| emo-text state.rs `apply_cue` 非消費 arm `Emote|EntityRef|Custom`＝強制点 3 | `state.rs:165`・`:195` で確認 | 一致 |
| emo-present `TextSlotView`・`text_slot_view()`・`build_target_assets`・`hide_then_reshow_recovers_display_from_cache` | `presenter.rs:81`・`:404`・`:531`・`:954` で確認（設計の presenter.rs:954 参照は正確） | 一致 |
| emo-present balloon.rs `build_balloon_target`・TempDir＋MemoryDecoder テスト流儀 | `balloon.rs:120`・`:172`（MemoryDecoder）・`:204`（TempDir 流儀）で確認 | 一致 |
| sakura compile.rs catch-all `other`（Raw 破棄）の前に明示 arm 追加 | `compile.rs:79-83`（catch-all）・`:50`（`Instruction::Surface` 先例 arm）で確認 | 一致 |
| テスト補助 `BindSet::from_ids` | `areka-emo-compose/src/bind.rs:22` で確認 | 一致 |

**要件カバレッジ**: Traceability 表は R1.1–R7.3 の全 AC を網羅し、各行の Components/Interfaces が上記実シンボルへ正しく着地する。D1（ukadoc `\b` 正典）は research.md §6.1 で原典ページ直接取得により解決済み（`-1` 非表示・裸形 0–9 単桁・奇数予約＝作法・`\b` に alias 記述なし＝数値解決のみは正典安全側）。

## 3. Critical Issues

ブロッキング（NO-GO 相当）の issue は**なし**。実装フェーズへの申し送りとして低重要度の注意点を 2 件登記する（GO を妨げない）。

### Issue 1（重要度: 低・非ブロッキング）— seriko actor 内側 match への arm 挿入は軽微な再構成を要する
- **内容**: 既存 `handle_message` の内側 match は「key 抽出 match」（`let key = match &cue.command { Emote{key} => key, ... }`・`actor.rs:192-212`）の形であり、設計スニペットの `BalloonSurface` arm（resolve→apply_balloon→emit を arm 内で完結）は値を返さず早期 `return ControlFlow::Continue(())` する形になる。設計の意図（明示 arm・catch-all 非新設）は明確だが、arm の配置は key 抽出 match の**前段分岐**または match 全体の再構成が必要。
- **影響**: task-level の実装形の話であり設計判断に影響しない。タスク生成時に「既存 Emote 経路のコード形状を変えない」制約（R4.6）と両立する挿入位置を明記すれば足りる。

### Issue 2（重要度: 低・非ブロッキング）— 破損数値入力のログ水準がシェル経路と非対称
- **内容**: シェル経路は `Unresolved`（未知 alias・`-2`・u32 超過）を **error!**（`actor.rs:216-222`）とする一方、設計のバルーン経路は名前形（M-boot 未対応）も破損数値（`-2`・範囲外）も **warn!** 一括（Error Categories 表「Unresolved 一括」）。EntityRef の warn! 先例に基づく意図的裁定として登記済みだが、「M-boot 未対応（warn!）」と「破損入力（シェルでは error!）」の類別がバルーン側で潰れている。
- **影響**: ログ檻アサートの水準が実装時に揺れうる程度。実装前にどちらかへ一言確定（warn! 一括のままでも可・その場合はテストが warn! を檻に固定）すればよい。

## 4. Strengths

1. **実シンボル精度が極めて高い**: 設計の既存コード主張（行番号・型形状・テスト名まで）が実ツリーと全数一致し、「書かれたとおりにビルド可能」。特に強制コンパイル点 3 箇所（contract.rs／ghost sink.rs／emo-text state.rs）の同定と、seriko 内側 catch-all の「コンパイラ非強制」リスクをテスト檻（E2E＋同期単体＋capture_logs 実在確認済み）で補償する設計は、catch-all 禁止文化・決定論必達の steering/MEMORY 方針に正確に整合する。
2. **理想形との対比が明示的**: D3（分類先）で理想形（`CueTarget` の `Surface`/`Text` リネーム）の solve/not-solve を対比した上で Option A を採り、名前負債を doc 更新＋将来 spec 申し送りとして「黙って風化させない」処置を登記。D1 の ukadoc 原典直接取得（MCP 空振りの回避）・E2E 同期チェーンの新 API ゼロ成立の実証も、設計フェーズの研究義務を正しく完遂している。

## 5. 最終判定

### GO

**根拠**: (1) 設計が依拠する既存シンボルの主張は実ツリーで全数確認でき、additive 増分として「書かれたとおりに」実装可能。(2) 要件 R1–R7 の全 AC が Traceability 表で具体的な契約・テストへ写像され、ブロッカー研究（D1 ukadoc 正典）は解決済み。(3) 検出された注意点 2 件はいずれも task-level の低重要度であり、設計の再生成を要しない。

**次のステップ**:
- 設計ディスカッション（`/kiro-design-discussion areka-P0-balloon-face-cue`）で Issue 1/2 の裁定を一言確定（arm 挿入位置の制約明記・バルーン経路のログ水準）
- 承認後 `/kiro-spec-tasks areka-P0-balloon-face-cue` でタスク生成へ
