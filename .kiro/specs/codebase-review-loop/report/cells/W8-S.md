# W8-S: wintf Cue・Dola統合（ecs/cue/ + ecs/dola/） × シンプル化（unsafe 保守則適用）

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点
- セルID: W8-S（領域 W8「wintf Cue・Dola統合」 × 観点 S「シンプル化」）。調査範囲は **`ecs/cue/` + `ecs/dola/` 全体**（領域 W8 は T セルで `W8-T1`=cue / `W8-T2`=dola に事前分割されたが、S 観点は領域全体を単一セルで担当）。
- 性質: 非挙動変更（リファクタリング／簡素化）。Feature Flag Protocol 不要。
- requirements: 2.2, 2.5, 2.7, 2.8, 4.1, 5.1, 5.3, 5.5（特に **R5.5: テスト保護外の unsafe（`unsafe impl Send + Sync` を含む）は構造的整理に限定**）
- design: S6（karpathy 基準・L136）、S2 検証、S10 コミット規約、W8 領域定義（L167）、観点列 S（L177）、unsafe 保守則（L177, L337, L515）、セル断片様式（L440）、提案記録様式（L453）
- 回帰検知器: 直前 19.1 W8-T1（`ecs/cue/` に in-source 32件＋`cue_performance_test` 決定論化）+ 19.2 W8-T2（`ecs/dola/` に in-source 12件、計44件）の特性化テスト。本セルの簡素化後に S2 全量がベースライン（**1712 passed / 0 failed**）と一致することが挙動非破壊の証拠。
- 参考: `report/cells/W8-T1.md`・`W8-T2.md`（テスト済み範囲・回帰検知器）、`W7a-S.md`・`W5a-S.md`・`W5b-S.md`（unsafe 保守則 S セルの判断基準）、`W5b-V.md`（COM/Rc 系 `unsafe impl` の crate 標準 SAFETY 注記様式）。

## 調査範囲（boundary = `crates/wintf/src/ecs/cue/` + `ecs/dola/`）

cue/ 9ファイル（2,075 LOC・うち in-source `mod tests` は queue/tracker/registry で W8-T1 追加分）＋ dola/ 1ファイル（mod.rs 423 LOC・うち in-source `mod tests` は W8-T2 追加分）を精読し、`cargo clippy -p wintf --lib --tests` の simplification 系 lint を各候補の起点とした。各候補を「テスト保護下か」「テスト保護外の unsafe 域か」で分類した。

| ファイル | 性質 | 簡素化候補の所在 |
|----------|------|------------------|
| cue/`mod.rs`（314 LOC） | re-export + 大規模 doc コメントのみ | なし（ロジックゼロ） |
| cue/`command.rs`（9 LOC） | `dola::cue` からの pure re-export | なし（境界外 D3 領域の型・触れない） |
| cue/`component.rs`（16 LOC） | `PendingCueSheet` 定義のみ | なし（データ定義） |
| cue/`error.rs`（32 LOC） | `CueSystemError`/`CueSheetResult` enum（thiserror） | なし（既に最小・Display は導出） |
| cue/`queue.rs`（761 LOC、うち test ~300） | **CueQueue** = テスト保護下の純粋ロジック（W8-T1 で16件） | collapsible_if ×3（R5.5 外だが churn 回避で見送り）・バリア解決末尾の三重複（churn 回避で見送り） |
| cue/`registry.rs`（168 LOC、うち test ~83） | **EntityRegistry** = テスト保護下の純粋ロジック（W8-T1 で5件） | collapsible_if ×1（churn 回避で見送り） |
| cue/`dispatch.rs`（210 LOC） | **dispatch_cue_sheet_internal**（in-source テストなし・E2E が一部のみ駆動） | RouteAdd≡RouteSwitch 同一本体・Command/Barrier 近重複（**ロジック構造変更につき P67 へ**） |
| cue/`systems.rs`（67 LOC） | **update_cue_sheet_trackers** System | collapsible_if ×1（churn 回避で見送り） |
| cue/`tracker.rs`（498 LOC、うち test ~290） | **CueSheetTracker** = テスト保護下の純粋ロジック（W8-T1 で11件） | resolve_barrier の (Click,Click)/(Skipped,Click) アーム or-統合候補（churn 回避で見送り） |
| dola/`mod.rs`（423 LOC、うち test ~300） | `DolaAnimator` ラッパー（W8-T2 で12件保護下）＋ **`unsafe impl Send + Sync`（テスト保護外）** | unsafe impl の **SAFETY 注記構造整理（R5.5 で適用）** |

### clippy（simplification 系）境界内ヒットの分類（BEFORE 実測 = AFTER 実測、不変）

`cargo clippy -p wintf --lib --tests --message-format=short` の cue/+dola/ 境界内ヒットは **正確に5件のみ**、いずれも `collapsible_if`（`this if statement can be collapsed`）であり、全て**プロダクションコード**を指す（W8-T1/T2 が追加した in-source `mod tests` を指す診断はゼロ＝テスト追加が新規 lint を導入していないことの裏付け）:

| lint 種別 | 境界内件数 | 所在（file:line） | クレート全域 | 判定 |
|-----------|-----------|------------------|-------------|------|
| `collapsible_if` | **5** | queue.rs:165・queue.rs:188・queue.rs:344・registry.rs:54・systems.rs:44 | 68（全域で容認） | **見送り**（churn 回避・let-chain 不採用慣習。後述） |

その他の simplification 系 lint（`useless_conversion`/`derivable_impls`/`map_or`/`manual_div_ceil`/`default_constructed_unit_structs` 等）は cue/+dola/ 境界内に**ゼロ**（クレート全域のヒットはすべて com/・graphics/・widget/・window/ に分布し本境界外であることを `--message-format=short` で file:line 単位に確認）。

## 適用した簡素化（1件・dola/mod.rs の unsafe impl SAFETY 注記の構造整理。R5.5 適用域）

### 適用1: `DolaAnimator` の `unsafe impl Send + Sync` の SAFETY 注記を crate 標準様式へ格上げ（dola/mod.rs:47-52 → 47-59）

`dola/mod.rs` の `unsafe impl Send for DolaAnimator {}` / `unsafe impl Sync for DolaAnimator {}`（mod.rs:51-52、不変）直前の SAFETY 注記（旧4行 `// Safety: ...`）を、crate 標準の SAFETY 様式（`graphics/command_list.rs:29-33` 等が採る「`SAFETY 条件:` [手動 impl が必須である理由]（だからこそこの unsafe impl が必要）。健全性は…に依拠する: [根拠]」ブロック）へ格上げした（コメントのみ +11 / −4、コード行は一切不変）。

新注記が明記する内容（**親指示が要求する2点を充足**）:
1. **Rc 内包ゆえ手動 impl が必須であること**: `DolaRuntime` は内部に `Rc` を含む（`timeline_manager` の `ObjectInternPool = HashMap<DynamicValue, Rc<DynamicValue>>` ＝ dola `runtime/interpolator/mod.rs:20`、および `UpdateResult.changes` 経由の `EvaluatedValue::Object(Rc<DynamicValue>)` ＝ dola `runtime/types.rs:24`）。`Rc` が `!Send + !Sync` であることにより `DolaRuntime` も自動では `Send/Sync` を導出できず、一方 `bevy_ecs` の `Component` は `Send + Sync` を要求するため、この手動 `unsafe impl` は**冗長ではなく必須（load-bearing）**である。
2. **単一スレッドアクセス不変条件**: wintf は単一スレッド（Windows UI スレッド）でのみ動作し、内部 `Rc` は当該スレッド内でのみアクセスされる。`tick_dola_animators` の `Query<&mut DolaAnimator>` が 1 tick 1 回・単一スレッドの排他アクセスを型レベルで保証する（スケジュールが `par_iter_mut` で並列化されないこと・他システムが跨スレッドで `&DolaAnimator` を共有しないことが不変条件であり、**その実スケジュール構成の検証は W8-V の担当**）。

- **挙動非破壊根拠**: 変更は `//` コメント行のみ（`git diff` 実証: 変更ハンク内の `+`/`-` 行はすべて `//` で始まるコメント行で、`unsafe impl Send/Sync` の2行は無印のコンテキスト行＝不変）。型・シグネチャ・unsafe の意味論・生成コードに一切触れない。`cargo build --workspace` 成功 + S2 全量がベースライン（1712/0、テスト増減ゼロ）一致で実証。
- **テスト保護**: `unsafe impl Send/Sync` の健全性そのものは型システム + スケジュール構成の不変条件でありテストで検証する性質のものではない（W8-T2 所見3 と同認識）。`DolaAnimator` の機能契約（tick/last_result/system 配線）は W8-T2 の in-source 12件が保護下。
- **R5.5 整合**: R5.5 は「テスト保護外の unsafe はロジック変更を伴わない構造的整理（命名・コメント・自明な重複除去）に限定」と規定。本変更は **SAFETY 注記（コメント）の構造整理のみ**でロジック非介入。親指示「DolaAnimator の SAFETY 注記を整理する場合は、Rc 内包ゆえ手動 impl が必須であること＋単一スレッドアクセス不変条件を明記（W5a-V/W5b-V/W7a-V の SAFETY 注記様式を参考）」に正対する。前例 **W5b-V 適用1〜3**（WIC/COM 保持型の旧1行コメントを「必須/冗長を区別する SAFETY 根拠ブロック」へ格上げ）と同種・同様式。なお `unsafe impl` の**健全性点検（実スケジュールが単一スレッド実行されることの検証）は 19.4 W8-V の主担当**であり、本 S セルでは意味論に踏み込まず注記の構造整理に限定した（W5a-S → W5a-V、W5b-S → W5b-V の S/V 役割分担に準拠）。

## R5.5 で構造整理に限定／見送った unsafe・System 域の候補

- **`collapsible_if`（境界内5件・queue.rs:165/188/344・registry.rs:54・systems.rs:44）**: clippy は `if let Some(cap) = self.capacity { if self.schedule.remaining() >= cap {`（capacity 検査）等の let-chain 統合を提案。queue/registry の3〜4件は **W8-T1 でテスト保護下の純粋ロジック**（R5.5 の「テスト保護外 unsafe」域ではない）だが、(a) 当リポジトリは collapsible_if をクレート全域で **68件**容認し let-chains を**一切採用していない**（`grep` 実証: wintf src に `if let ... && let` 形式ゼロ。edition 2024 + rustc で構文は可能だが慣習として不採用）。cue/ だけ let-chain 化すれば本境界だけ不整合な churn（karpathy 3 違反）。(b) 直前の S セル群（W7a-S が collapsible_if 29件・W5a-S が typewriter_draw.rs:161・W5b-S が systems.rs:349）がいずれも**同一根拠で見送り済み**で判断の一貫性が必要。**見送り**（churn 回避、可読性改善が乏しい）。systems.rs:44 の1件は System 本体（read/write query 域）。
- **cue/dispatch.rs の RouteAdd≡RouteSwitch 同一本体・Command/Barrier 近重複**: 後述「proposals へ回した候補（P67）」参照。**ロジック構造変更につき proposals 記録**（適用せず）。
- **cue/tracker.rs:180/191 の resolve_barrier アーム or-統合**: `(Some(BarrierResponse::Click), Some(TrackedBarrierKind::Click)) => ResolveAllClicks`（:180-182）と `(Some(BarrierResponse::Skipped), Some(TrackedBarrierKind::Click)) => ResolveAllClicks`（:191-193）は結果アクションが同一で `(Some(Click) | Some(Skipped), Some(Click))` への or-pattern 統合が技術的には可能。**ただし見送り**: (a) clippy が `match_same_arms` を発火していない（出力にゼロ）＝適用簡素化を診断にトレースできない、(b) 両アームは「実 Click 応答」と「ハンドラ不在の Skipped を Click バリアで resolve 扱い」という**意味的に別個のドメイン判断**を表しており、or-統合は「偶然同じ結果になる別ケースの併合」で intent の可読性を**下げる**（W8-T1 が `receive_barrier_skipped_is_not_recorded_then_first_valid_wins`・`resolve_click_barrier_returns_resolve_all_clicks_without_finishing` で両者を別個に特性化）。テスト保護下だが karpathy 3「壊れていないものを refactor しない」。**churn 回避で見送り**。
- **cue/queue.rs:300-373 のバリア解決末尾の三重複**: `resolve_click`（:300-313）・`resolve_choice`（:316-340）・`skip_barrier`（:356-373）が各々「`state = Playing`・`barrier_entered_time = None`・`barrier_timeout = None`・（trace）・`if schedule.is_completed() { state = Completed }`」の末尾を持つ。**ただし見送り**: (a) 3メソッドは本体が非同一（`resolve_click` は `pending_choices.clear()` を**含まない**が他2つは含む、`resolve_choice` は `found` 分岐 + `Option<String>` 返却を持つ、trace メッセージが各々異なる）ため単一ヘルパで綺麗に括れず、`&str` ラベル引数化等の machinery を要する（karpathy 2「単一目的に抽象を作らない」に抵触気味）、(b) テスト保護下（W8-T1 のバリアテスト群）だが各メソッドは状態機械として線形に読める現行形が明瞭で、ヘルパ抽出は state machine 横断の間接化を持ち込む。可読性の明確な向上がなく churn が勝るため**見送り**（karpathy 2/3）。

## proposals へ回した候補

- **P67（新規）**: `dispatch_cue_sheet_internal`（cue/dispatch.rs:27-179）の配送アーム重複統合。(1) `RoutingCommand::RouteAdd`（dispatch.rs:48-62）と `RouteSwitch`（:63-77）のアーム本体が tracing ログ文字列以外**バイト同一**（両者とも `registry.resolve(to)` → 成功で `register_actor` 後勝ち上書き → debug/warn）。(2) `CuePayload::Command`（:93-131）と `CuePayload::Barrier`（:134-172）のアームが `Entry::Payload` vs `Entry::Barrier` の構築種別と warn 文言以外ほぼ同一（約38行の `routes_for_actor`→空スキップ→`insert`→`set_cue_sheet`→`seen_targets`/`all_targets` 集約手続きが複製）。**ロジック構造変更につき本ループ非実施・記録のみ**: dispatch.rs は **in-source テストなし**（W8-T1 が確認）、配送検証を担う `tests/ecs/cue_dispatch_e2e_test.rs`（6件）は **`CuePayload::Command`（Text/Clear）と `RouteSwitch` のみ駆動**し、**`CuePayload::Barrier` 配送アーム・`RouteAdd`・`RouteRemove` は一切駆動しない**（grep 実証: E2E に Barrier 配送・RouteAdd 構築なし。RouteAdd の wintf 側構築は `cue_data_model_test.rs:114` の型検査のみで dispatch 駆動なし）。統合対象アームの一部が**回帰検知器なしの未保護分岐**ゆえ、構造変更は実起動 S7 でしか挙動非破壊を担保できず、R5.2/R2.8 に従い proposals 記録。W6a-T の P58（buffers down/up 転送 match 重複の DRY 整理候補）と同系統。`proposals.md` 末尾 P66 の次として **P67** を採番・追記済み。
- 上記以外に proposals 新規記録なし（collapsible_if・resolve_barrier or-統合・バリア末尾三重複はいずれも「ロジック変更を要する」候補ではなく churn 回避の見送りゆえ、新規仕様化を要さず proposals 化しない）。

## 適用しなかった候補と理由（churn 回避等）

- 上記「R5.5 で構造整理に限定／見送った候補」セクションの全候補（collapsible_if 5・resolve_barrier or-統合・バリア末尾三重複）。
- **cue/mod.rs・command.rs・component.rs・error.rs・dola/mod.rs のラッパー本体**: simplification 系 clippy ヒットゼロ。精読の結果、いずれも S6「最小コード」を既に満たす。mod.rs は re-export + doc のみ、command.rs は dola pure re-export（境界外 D3 型ゆえ触れない）、error.rs の enum は thiserror 導出で最小、dola/mod.rs の `DolaAnimator`（new/with_runtime/tick/last_result/runtime/Default/Debug）は `DolaRuntime` への薄い委譲で踏み込んだ簡素化の余地なし。変更なし。
- **テスト保護下の純粋ロジック（CueQueue・CueSheetTracker・EntityRegistry・DolaAnimator 委譲）**: W8-T1/T2 の回帰検知器の保護下でより踏み込んだ簡素化が許される領域だが、精読 + clippy 併用の結果、**挙動非破壊で可読性が明確に上がる適用候補が存在しなかった**（CueQueue の capacity 検査・tick の状態ガード・EntityRegistry の filter_map・CueSheetTracker の update 優先順位はいずれも既に最小で、統合可能な冗長分岐・デッドコード・自明ヘルパ抽出箇所を持たない。dispatch のみ DRY 候補があるが未保護分岐ゆえ P67 へ）。

## S6（karpathy-guidelines）適合確認

- 適用1件（dola/mod.rs の SAFETY 注記格上げ）は「既存コメントの crate 標準様式への整合 + 親指示が要求する根拠（Rc 内包ゆえ手動 impl 必須・単一スレッド不変条件）の明文化」であり、新規抽象・投機的柔軟性・不要なエラー処理の追加はゼロ（rule 2 Simplicity First / Surgical Changes）。変更はコメントのみで、自分の変更で孤児化したものなし（rule 3）。
- 成功基準（rule 4 Goal-Driven）: 「S2 全量がベースライン 1712/0 と一致＝挙動非破壊」を満たした。挙動を変える/ロジック構造変更を要する簡素化候補（P67）は proposals へ退避し本ループでは実装しない（R5.2/R5.5）。テスト保護外 unsafe（`unsafe impl Send + Sync`）はロジックに踏み込まず注記整理に限定した（R5.5 厳守）。

## verification (S2)

- BEFORE: 親検証済みベースライン（HEAD = W8-T2 コミット・クリーンツリー・**1712 passed / 0 failed**）を信頼し省略（親指示「BEFORE S2 は省略可」・design フェーズ0 規定に従う）。
- AFTER: `cargo build --workspace` 成功（exit 0）/ `cargo test --workspace` **1712 passed / 0 failed**（ignored 32、全22本の `test result:` 行を awk で合算した実測 `passed=1712 failed=0 ignored=32`）。`test result: FAILED`/`^error[`/`^error:`/`panicked` 行ゼロ（grep 実証 `bad_markers=0`）。
  - **ベースラインと完全一致（1712/0）= テストの追加・変更・削除ゼロで全1712既存テストが簡素化後コードをそのまま通過 = 挙動非破壊の裏付け**。グローバル件数変動なし（±0）。
  - 反復検証: `cargo test -p wintf --lib cue::` で **32 passed / 0 failed**（W8-T1 in-source 回帰検知器全件緑）、`--lib dola` で **12 passed / 0 failed**（W8-T2 in-source 回帰検知器全件緑）、`--test ecs` で **102 passed / 0 failed**（cue/dola 統合テスト・`cue_performance_test` 含む全件緑）。
- 変更ファイル: `crates/wintf/src/ecs/dola/mod.rs`（**+11 / −4**、`git diff --numstat` 実測。コメントのみ・コード行不変）の1ファイル + `report/proposals.md`（P67 追記・断片外の記録ファイル）。boundary（cue/+dola/）内のプロダクションコード**ロジック変更ゼロ**・新規テストファイルなし・tests/ 不変。

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue` は W8-T1 で**決定論化により解消済み**（実時間しきい値除去）。本 AFTER S2 全量実行で `tests/ecs` バイナリは **102 passed / 0 failed**、反復の `--test ecs` 単独実行でも 102/0 で安定合格。本セルの変更（SAFETY コメント整理のみ）は cue キュー timing と無関係。隔離再実行不要。flaky 判定によりゲート通過。
- 本セルは新規テスト追加ゼロ・プロダクションロジック変更ゼロ（コメントのみ）のため、フレーキー新規導入なし。

## clippy（S3・記録のみ・非ブロッカー）

- `cargo clippy -p wintf --lib --tests` の cue/+dola/ 境界内 simplification 系 lint:
  - **`collapsible_if`: 5 → 5**（BEFORE = AFTER、不変。queue.rs:165/188/344・registry.rs:54・systems.rs:44。R5.5/churn 回避で意図的に未適用）。
  - その他の simplification 系 lint（useless_conversion・derivable_impls・map_or・manual_div_ceil・default_constructed_unit_structs 等）は境界内に**ゼロ**（BEFORE/AFTER とも。クレート全域のヒットは com/・graphics/・widget/・window/ に分布し本境界外であることを file:line 単位に確認）。
  - **新規 clippy 警告/error の導入はゼロ**（適用1はコメントのみのため lint 出力に影響せず。AFTER で境界を再 lint し BEFORE と同一の5件のみを確認）。W8-T1/T2 が追加した in-source `mod tests` を指す診断もゼロ（テスト追加が新規 lint を導入していないことの裏付け）。
  - 解消した lint: なし（本セルは挙動非破壊で適用可能な lint 解消候補を持たなかった。cue/ の collapsible_if は churn 回避で据え置き）。
- S3 規定によりブロッカーとせず記録に留める。
