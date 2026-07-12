# 設計バリデーションレポート: areka-P0-emo2-boot

- 日付: 2026-07-12
- 対象: `.kiro/specs/areka-P0-emo2-boot/design.md`（requirements.md 承認済み・research.md §9 DD-1〜DD-11 反映済み）
- 手法: design-review.md の REVIEW プロセス（Analysis → Critical Issues → Strengths → GO/NO-GO）＋実シンボル突合（Grep/Read）

## 実シンボル突合の結果（Analysis）

設計が依拠する load-bearing シンボルを実コードと照合し、**全て実在・形状一致**を確認した。

| 設計の主張 | 実コード | 判定 |
|---|---|---|
| `DisplayCommand::{Show,Hide,ShowBalloon,HideBalloon}`・`ShowBalloon` は binds なし・adapter が `BindSet::default()` を組む旨の doc | `crates/areka-seriko/src/output.rs:28-42`（doc :19/:38） | ✅ 一致 |
| `SurfaceOutput::send(&mut self, DisplayCommand)` infallible | output.rs:49-52 | ✅ |
| `spawn_seriko(resolver, static_binds, out) -> (SerikoSink, ActorHandle)`・`O: SurfaceOutput + Send + 'static` | `crates/areka-seriko/src/actor.rs:133-139` | ✅ |
| `boot`/`GhostBootOptions`/`shutdown(CloseReason)`/`ShioriWiring::Custom`/`TickerMode::Disabled`・sink 境界 `Clone + Send + 'static` | `crates/areka-ghost/src/runtime.rs:53,57-61,65,169,301-304` | ✅ |
| `EmoPresenter::{attach_target,apply,text_slot_view,read_back}`・`text_slot_view` は mount＋chain 両方 Some まで None | `crates/areka-emo-present/src/presenter.rs:149,177,404-408,419` | ✅ |
| `PresentCommand::ShowSurface{..}`/`TargetId(pub u32)`/`build_balloon_target` | command.rs:23,39-43／balloon.rs:120-123 | ✅ |
| `EmoTextSink`/`spawn_emo_text(Rc<RefCell<TextLayerRuntime>>)`/`register_actor_view(actor,&view,&model)`/`present_frame(runtime,world,talk_time)` | `crates/areka-emo-text/src/{sink.rs:41, actor.rs:268,197,302}` | ✅ |
| 現 main.rs は boot(:244) が `WinApp::new()`(:266) より前・両 sink `LogSink`・`is_benign_boot_error`・smoke ゲート | `crates/areka/src/main.rs` | ✅ |
| `GhostWindows`（Resource＋戻り値）・`char_window`/`balloon_window`/`scopes` | `crates/areka/src/placement/spawn.rs:111-130,147` | ✅（scope 引数は `usize`・設計の `u32` とは軽微な型差） |
| DD-2 前提「dispatcher は talk ごと初回 Tick を base に相対秒配信」 | `crates/areka-ghost/src/dispatcher.rs:137-147`（`base_now`・`(now-base)/1000.0`） | ✅ |
| `FrameTime(pub f64)`／`FrameFinalize`／`dola::runtime::clock::now` | wintf core.rs:147／schedule_labels.rs:112／dola clock.rs:14 | ✅ |
| `TalkCue.at: f64`・`SurfaceSink`/`TextSink` | `crates/areka-sakura/src/{contract.rs:46-48, sink.rs:15-25}` | ✅ |

要件トレーサビリティは R1〜R10 の全受入基準（10 要件・46 AC）が設計表に写像済みで欠落なし。DD-1〜DD-11 は棄却案の根拠（例: `CommandSender` の sender 公開口不在＝wintf 改変必要）まで実シンボルで裏付けられている。steering 適合（tokio 不使用・外部依存ゼロ・UI スレッド固定・log-first・GPU readback 定石・非改変境界）も確認した。

## Critical Issues（最大 3・重要度順）

🔴 **Critical Issue 1**: BootAssets の scope 集合ソースと GhostWindows の整合機構が未指定
**Concern**: `open_startup_window` は `prepared.placements` を async クロージャへ move し（main.rs:451-465）、placement の実 scope 集合は `wire_emo2_boot` 時点で同期参照できない。設計の事前条件「scopes は placement 準備結果と同一ソースから得る」は規約どまりで、取得機構（自前再 parse か・GhostWindows 到達後の交差か）と片側欠落時（asset あり窓なし／窓あり asset なし）の挙動が明文化されていない。
**Impact**: attach フェーズの走査主体が曖昧なまま実装に入ると、scope 不一致時に silent skip か panic かが実装者判断になり、log-first 規律と R4.2 の再試行意味論が揺れる。
**Suggestion**: attach フェーズは「`GhostWindows::scopes()` を正とし assets を lookup・欠落は `warn!`＋skip（表示なし縮退）」と tasks で明文化し、単体テスト（scope 不一致ケース）を 1 本檻に入れる。`usize`（GhostWindows）↔`u32`（target_map）の型差もここで吸収を指定する。
**Traceability**: R1.2／R1.4／R4.2
**Evidence**: design.md「構築入力 / assets」事前条件・DD-4（frame attach フェーズ）

🟡 **Critical Issue 2**: 決定論 spine は `emo2_frame_system` の schedule 結線自体を檻に入れない
**Concern**: spine テストは `run_attach_phase`/`run_drain_phase`/`run_text_phase` の直接駆動（設計が意図的に用意したテスト駆動口）で全ロジックを通すが、`add_systems(FrameFinalize, emo2_frame_system)` 登録と NonSend remove→insert の取り回しは決定論檻の外（env-gate 実走と smoke のみ）に残る。R8.1「全経路」の解釈次第で観測漏れと読める。
**Impact**: system 登録・借用規律のバグ（例: remove 忘れの二重借用 panic）は実走まで検出されない。ただし当該部は donor（`boot_present_system`）の実績パターン直系で薄く、リスクは限定的。
**Suggestion**: headless World＋schedule 単体実行（bevy_ecs のみ・wintf 実 loop 不要）で `emo2_frame_system` を 1 フレーム回す最小テストを spine に足すか、Testing Strategy に「schedule 結線は smoke／実走が担う」と観測境界を明記する。
**Traceability**: R8.1
**Evidence**: design.md「Testing Strategy › Integration Tests」（frame フェーズ直接駆動）・frame コンポーネントの Implementation Notes

🟡 **Critical Issue 3**: TalkClock の epoch 推定は Clear を伴わない連続 talk で可視数逆行があり得る
**Concern**: DD-2 の単調 max リベースは新 talk 到着で epoch を前方へ跳ばすが、新 talk が `Clear` で始まらない場合、旧 talk の glyph（旧 epoch 基準の r_i）に対し新 epoch 基準の `talk_time` は小さく写り、既リビール文字が一時的に未リビール側へ戻り得る。emo-text 契約（talk 起点相対・Clear が唯一のリセット）の固有性質であり本設計の新造欠陥ではないが、設計はジッタ逆行（≤50ms）のみ言及し talk 跨ぎの逆行は未言及。
**Impact**: 実運用スクリプトは通常 Clear 開始のため実害は低いが、境界挙動が未定義のまま実装されると spine の単調増加述語（R8.5）が talk 跨ぎケースで偽陽性/偽陰性を出す余地がある。
**Suggestion**: リスク節へ既知制約として明記し、spine の述語適用範囲を「単一 talk 内」または「Clear 起点後」と限定する（必要なら S2 に talk 切替ケースを 1 本追加）。
**Traceability**: R2.2／R2.3／R8.5
**Evidence**: design.md DD-2・research.md §9.3（リスク更新は talk 内ジッタのみ）

## Design Strengths

1. **実シンボル全数検証に基づく決定の質**: DD-1〜DD-11 の各決定が実コード上の制約（`CommandSender` の sender 非公開・`spawn_ui` handler の `&mut World` 不在・`text_slot_view` の遅延 None・dispatcher の per-talk 相対秒）で棄却案まで裏付けられ、本レビューの独立突合でも齟齬ゼロ。トレーサビリティ表は全 46 AC を網羅。
2. **装着タイミングの罠の構造的解消と境界遵守**: DD-4「apply 同期＝同一フレーム text_slot_view Some 保証」＋「mpsc が保留バッファを兼ねる（早着指令の無損失）」で最大リスクだった合流問題を機構でなく構造で消し、変更面を areka bin 1 crate に完全封じ込め（R10 非改変・フォールバックで既存 smoke 前提も温存）。

## Final Assessment

**Decision: GO**

**Rationale**: 設計はアーキテクチャ整合（アクター＋UI 配送・NonSend 規律・log-first）・要件充足（全 AC 写像）・実装可能性（donor 実績パターン直系・全依存シンボル実在確認済み）のいずれも満たす。Issue 1〜3 はいずれも tasks フェーズでの明文化・テスト 1〜2 本の追加で吸収可能な仕様精緻化事項であり、設計の再生成を要する構造欠陥ではない。

**Next Steps**:
1. 設計ディスカッション（kiro-design-discussion）で Issue 1（scope 集合の整合機構）を最優先に裁定
2. Issue 2/3 は tasks 生成時にテスト項目・観測境界注記として反映
3. 裁定後 `/kiro-spec-tasks areka-P0-emo2-boot` へ進む
