# 設計バリデーションレポート: areka-P0-ghost-setup

- **日付**: 2026-07-06
- **対象**: `.kiro/specs/areka-P0-ghost-setup/design.md`（requirements.md R1〜R8 確定済み・research.md §7 設計決定 DD-A〜DD-K）
- **検証方法**: design-review.md 基準（アーキテクチャ整合・一貫性・保守性・型/インターフェース）に加え、設計が触れる実クレート（areka-kanade / areka-sakura / areka-actor / shiori-host32-host / areka バイナリ）の実コードと設計の事実主張を突合した。

## レビューサマリ

設計は実コード接地の精度が非常に高い。主要な事実主張（kanade `talk.rs` の DD-1 切り出し前提・`ShioriMsg::Unload` スタブの 1 アーム差し替え点・契約フォークの型形状・`spawn_kanade`/`spawn_shiori_actor`/`spawn_talk` シグネチャ・`HelperLifecycle::status()` の sticky/`&mut`・ForceQuit の OnClose NOTIFY→Unload→Close 実証順序・`open_startup_window` の private シームと smoke ゲート）を全て実コードで照合し、**全一致**を確認した。要件トレーサビリティは R1.1〜R8.4 の全 AC がコンポーネント・インターフェースへ写像済み（R1.7 物理単一定義＝変換アダプタ不在、R7.6 純 x64＝プロセス spawn ゼロの scripted backend、R8.4 opt-in 限定＝既存 env gate 慣行、いずれも設計で担保）。凍結面（`TalkCue`/sink trait/`cue_target_of`/dola cue・`Shiori3Client`/`RequestError`/`LifecycleReport`・areka-actor 公開面）は不改変が明記され、relay 機構により `spawn_kanade`/`spawn_shiori_actor` のシグネチャ不変のまま循環結線を解いている。正常系 shutdown（ForceQuit→kanade join→dispatcher Close/join→ticker→shiori→relay）はメッセージ順序・チャンネル寿命を追跡した結果**デッドロックフリーで健全**。ただし S6（全断線）が依拠する「全 Sender drop 停止経路」が結線トポロジ上で構造的に成立しない欠陥を 1 件検出した。

## Critical Issues

### 🔴 Critical Issue 1: S6（全断線）シナリオの構造的不成立——切断停止経路が結線トポロジで喪失する

- **Concern**: 2 つの独立した機構が「全 Sender drop で正常終了」経路を塞ぐ。
  (i) **dispatcher の self-sender**: per-talk done ポート用に body が自 inbox の `Sender<DispatcherMsg>` を恒久保持する（design「Delivery guarantees」）ため、dispatcher の inbox は決して切断されず、dispatcher は `Close` 以外で停止不能。dispatcher は `Sender<KanadeMsg>` も保持するため kanade の inbox も切断に到達しない。
  (ii) **on_down 保持化による Sender 循環**: 死活報告のため shiori actor が `down_tx` をループ中保持する変更（DD-D）により、kanade —(shiori_tx 保持)→ shiori —(down_tx 保持)→ down-relay —(kanade_tx 保持)→ kanade の循環が生じ、純粋な sender drop はこの環を貫通できない。現行 `real.rs` は**正にこの理由で** on_down を接続成否確定後に即 drop している（rustdoc: 「保持すると kanade の『全 Sender drop で正常終了』（Req 4.9）を妨げる」）。
  よって S6 の「`into_parts` で senders を drop→kanade／dispatcher／shiori／relay が全て有界時間内に正常終了」は**ハングする**。DD-C の「kanade の『全 Sender drop で正常終了』は down-relay が仲介するため影響しない」という主張は誤り。
- **Impact**: R7.5 の必須シナリオ（全断線）が実装段階で不成立と判明し手戻りが確定する。また actor-foundation の停止 2 経路規約（Close／全 Sender drop）がどのアクターで放棄されるのかが文書化されないまま実装に入ると、shutdown ハング系の欠陥検出（独立レビューの複数回実行）の前提が崩れる。
- **Suggestion**: S6 を「段階的分解」に再定義する——例: `DispatcherMsg::Close`＋`ShioriMsg::Close` を先行送出（GhostParts に shiori sender を含める）→残る senders を全 drop→全 handle の有界 join（dispatcher 停止で kanade_tx が減り、shiori 停止で down_tx が落ち down-relay が終端、kanade は inbox 切断で終端、kanade drop で start_tx が落ち start-relay が終端——この順なら貫通する）。併せて design に**アクター別の有効停止経路マトリクス**（dispatcher＝Close のみ／shiori＝Close・inbox 切断／kanade＝Close・StopSelf・（結線解除後の）切断／relay＝上流 drop）を明記し、DD-C の当該主張と kanade rustdoc（Req 4.9 注記）の更新文言を訂正する。
- **Traceability**: R7.5（全断線の実行テスト網羅）・R6.4（全スレッド join）
- **Evidence**: design.md「System Flows 終了シーケンス」「結線トポロジの要点」「ghost::dispatcher Delivery guarantees」「spine e2e S6」／research.md DD-C・DD-D／実コード `crates/areka-kanade/src/shiori/real.rs`（on_down 即 drop の根拠 rustdoc）・`crates/areka-kanade/src/actor.rs`（「Sender を握ったまま join するとデッドロック」警告）

### 🔴 Critical Issue 2: spine e2e の駆動口・観測口の過小仕様（S3/S5/S6 の具体駆動が未確定）

- **Concern**: (a) `GhostRuntime::into_parts` が返す `GhostParts` の内容が「senders + handles」としか示されず、S6（および S3 の死活トリガ）が要する分解粒度——shiori actor への投函端を含むか——が未確定。(b) S3 の死活検出は「次のループ周回（次の request 到達時）」とされるが、runtime の公開投函口は `kanade()`/`dispatcher()` のみで、テストが shiori actor へ request を発生させる具体経路（`KanadeMsg::CloseRequest` 経由で OnClose GET を誘発するのか、`recv_timeout(500ms)` の周期 poll を有界待機で拾うのか）が指定されていない——後者は「wall-clock 非依存」の主張と緊張関係にある。(c) ghost::config 節の「`close_talk_deadline_ms` は既定 30_000（spine e2e はテスト側で短縮構成を組む）」に対応する注入点が `GhostBootOptions` に存在しない（S5 は注入 `now` で deadline を跨げるため短縮自体が不要のはず——記述と機構の不整合）。
- **Impact**: 決定論 spine e2e は本仕様の単一 pass/fail 観測（binding constraint・R7）。駆動口が未確定のままタスク化すると、e2e 実装時に公開 API の追加（GhostParts 拡張等）という設計差分が発生し、シナリオの決定論性の水準（有界 poll 許容か・メッセージ駆動限定か）が実装者判断に落ちる。
- **Suggestion**: `GhostParts` のフィールドを列挙し、S3/S5/S6 それぞれの駆動手順（誰がどの inbox へ何を投函し、何を有界観測するか）を 1 行ずつ確定する。deadline 短縮の記述は削除する（注入 now で足りる）か `GhostBootOptions` に KanadeConfig override を追加するか、どちらかへ倒す。
- **Traceability**: R7.4（注入 Tick・sleep 不使用）・R7.5（S3/S5/S6）・R5.4
- **Evidence**: design.md「ghost::runtime Service Interface（`into_parts`／GhostParts）」「観測層 spine e2e S3/S5/S6」「ghost::config Responsibilities」

## Design Strengths

1. **実コード接地と偽装シームの的確さ**: 契約フォークの型形状・スタブ差し替え点・connect 手順の昇格元（`connect_real_helper`）・凍結面の全てが実測に基づき、検証で全一致した。特に `ShioriBackend` の公開化＋`Box<dyn ShioriBackend>` 化は、「偽 ShioriConnection は `Child` 所有ゆえプロセスなしで構築不能」という実装不能点を、要件の注入形（connect closure・`spawn_shiori_actor(connect, on_down)`）を保ったまま解く設計判断であり、R7.1/R7.6（純 x64 決定論）を凍結面違反なしに成立させている。
2. **要件再解釈の明示性と正常系終了の健全性**: R1.1「kanade 正本」＝意味論正本＋物理所在 areka-talk（DD-A・kanade DD-1 の執行）、R6.2「kanade 停止観測→Unload」＝kanade 終了系列への内在化（二重発行なし）——いずれも根拠・代替案却下理由・Revalidation Trigger が記録されている。正常系 shutdown は実コードでテスト済みの ForceQuit 系列（OnClose NOTIFY→Unload→Close の順序保証）に接地しており、ForceQuit 送出→join の系列はデッドロックフリーであることをメッセージ順序・チャンネル寿命の追跡で確認した。

## Final Assessment

- **Decision**: **GO（条件付き）**
- **Rationale**: アーキテクチャ（契約正本 areka-talk・relay による循環解消・ShioriBackend シーム・dispatcher 単一 slot＋stale 棄却・ticker 差し替え・終了統括）は既存規約と整合し、R1〜R8 の全 AC をトレース可能に充足する。Critical Issue 1 は S6 の定義と停止経路文書の**局所修正**で解消可能であり（正常系 shutdown は健全）、アーキテクチャの再設計を要しない。Issue 2 は仕様の精密化であり同じ議論で確定できる。
- **Next Steps**:
  1. 設計ディスカッション（kiro-design-discussion）で Issue 1（S6 の段階的分解への再定義＋停止経路マトリクス＋DD-C 主張の訂正）と Issue 2（GhostParts 列挙・S3/S5 駆動手順・deadline 記述の整理）を確定し design.md へ反映。
  2. 反映後 `/kiro-spec-tasks areka-P0-ghost-setup` でタスク生成へ進む。
