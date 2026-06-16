# W8-V: wintf Cue・Dola統合（ecs/cue/ + ecs/dola/） × 脆弱性レビューと非破壊対策

- status: completed
- commit: （親が S10 準拠でコミット）

## 観点・基準・範囲

- セルID: W8-V（領域 W8「wintf Cue・Dola統合」 × 観点 V「脆弱性レビュー」）。本機能の**事前分割最終セル（最後のセルタスク）**。性質: **非挙動変更**（脆弱性点検＋挙動非破壊な対策のみ）。Feature Flag Protocol 不要。
- requirements（source 番号）: 2.3（脆弱性レビュー＋挙動非破壊対策）・2.4（挙動変更を伴う対策→提案記録）・2.5（前後 S2 非破壊）・2.7（列順 T→S→V。W8-T1/T2/S 完了済みの回帰検知器上で実行）・2.8（テスト保護外でも深く解析・安全適用不能は提案記録）・4.1（自己レビュー＋検証）・5.1（外部観測可能挙動を変更しない）・5.2（挙動変更必要時は提案記録）。
- design: Security Considerations（L514-516: unsafe 境界・Send/Sync 妥当性・整数変換の切り捨て/オーバーフロー・panic 経路 DoS を点検し、挙動を変えない範囲＝内部チェック・debug_assert・安全な型置換・SAFETY 注記のみ投入。API/エラー応答を変える対策は proposals へ）、CellExecutor 観点別規則 V（L338）、提案記録様式（L453-460）、セル断片様式（L440-451）、W8 領域定義（L167）。
- 領域（boundary = `crates/wintf/src/ecs/{cue,dola}/`、tests/ の該当ドメイン含む）: cue/（mod.rs・command.rs・component.rs・error.rs・queue.rs・registry.rs・dispatch.rs・systems.rs・tracker.rs の9ファイル）＋ dola/（mod.rs の1ファイル）。境界外には一切触れていない（`crates/dola/` 本体 D1〜D3・`ecs/world/` 等は**読み取りのみ・変更なし**）。
- 起点: W8-S 適用後のクリーンなワークツリー（親検証済みベースライン 1712 passed / 0 failed、HEAD = W8-S コミット `37ee729`）。
- **本領域の最重要点検対象**: W8-S からの主要申し送り＝`dola/mod.rs` の `unsafe impl Send + Sync for DolaAnimator`（Rc 内包・W8-S が SAFETY 注記を格上げ済み）の健全性を、**実スケジュール構成で裏取り**すること（実コードでの単一スレッド/並列実行の確証）。

## 点検手法

境界内10ファイルを grep（`unsafe`/`unwrap(`/`expect(`/`panic!`/`unreachable!`/`todo!`/`par_iter`/`from_bits`/`to_bits`/`as iXX|uXX|fXX`/添字 `[`/算術 `+`/`*`/`checked_`/`saturating_`/`wrapping_`）＋全文精読で走査。**DolaAnimator アクセスシステムの単一スレッド性を World/Schedule 構成（`ecs/world/mod.rs`・`schedule_labels.rs`、読み取りのみ）と bevy_ecs-0.18.0 / ルート Cargo.toml の feature 構成で裏取り**した。Entity ビット変換・スケジュール時刻整数境界・panic 経路を列挙し各々を判定した。

unsafe 健全性の実コード裏取りに用いた一次情報（すべて実コード grep / probe 実測。推測なし）:
- `c:\rust\cargo\registry\src\index.crates.io-1949cf8c6b5b557f\bevy_ecs-0.18.0`（本 crate 使用版。`CARGO_HOME=c:\rust\cargo`）のエグゼキュータ・Entity 実装を直接精読。
- ルート `Cargo.toml [workspace.dependencies.bevy_ecs] features` を直接確認（`"multi_threaded"` 有効）。
- `Entity::from_bits` の panic 条件を一時 probe テスト（境界内 cue/ に一時 mod を追加→検証後**完全撤去**、git status クリーン）で**実測**。

## 発見した脆弱性候補と判定

### 1. `unsafe impl Send + Sync for DolaAnimator`（dola/mod.rs:86-87）— 健全かつ必須だが、SAFETY 注記の旧根拠が実構成と不整合 → 注記是正（適用）＋ Sync ハザードを P68 記録

W8-S が格上げした SAFETY 注記は「wintf は単一スレッドで動作し、スケジュールが `par_iter_mut` で並列化されない／`tick_dola_animators` の `Query<&mut DolaAnimator>` が**単一スレッドの**排他アクセスを保証する」と主張していた（実スケジュール検証は W8-V へ委任と明記）。**実コード裏取りの結果、この「単一スレッド実行」という根拠は実構成と不整合**であり、注記をコメントのみで是正した（適用1）。裏取り事実:

- **(a) `tick_dola_animators`/`DolaAnimator` はプロダクションのいずれのスケジュールにも未配線**: ワークスペース全域 grep（`crates/` 配下）で `DolaAnimator`/`tick_dola_animators` の出現は **`ecs/dola/mod.rs`（定義＋in-source tests）・`ecs/mod.rs`（`pub use dola::{DolaAnimator, tick_dola_animators}` 再エクスポート、:16）・`tests/ecs/dola_animator_test.rs`（統合テスト）の3ファイルのみ**。`ecs/world/mod.rs` の `schedules.add_systems(...)` 群（:101-320、Input/Update/PreLayout/…/FrameFinalize へ多数のシステムを登録）に **`tick_dola_animators` は不在**。areka・examples にも spawn なし。→ **現状この unsafe impl は到達不能**（reachable な跨スレッド共有経路が存在しない＝現状 UB ゼロ）。
- **(b) ECS スケジュールは単一スレッド実行ではない（既定マルチスレッド）**: ルート `Cargo.toml [workspace.dependencies.bevy_ecs]` は `features = [..., "multi_threaded", ...]`（:43-50）を有効化。bevy_ecs-0.18.0 `schedule/executor/mod.rs:48-66` で `ExecutorKind` の `#[default]` は `std` + 非 wasm + `multi_threaded` のとき **`MultiThreaded`**（:65-66）。`ecs/world/mod.rs:80-85` は **`UISetup` のみ** `sc.set_executor_kind(ExecutorKind::SingleThreaded)` で固定し、`Input`/`Update`/`PreLayout` 等は既定（`Schedule::new(Update)` :75）＝マルチスレッドエグゼキュータで走る。`mod.rs:469` のコメント「マルチスレッドで実行されるシステムでもデータにアクセス可能になる」も裏付け。
- **(c) Send 化により `Query<&mut DolaAnimator>` システムはワーカースレッドで走り得る**: bevy のシステム `is_send()`（`function_system.rs:84`）は `NonSend`/`NonSendMut` 等が `set_non_send()` を呼ばない限り **true**。`Query<&mut DolaAnimator>` は（`DolaAnimator: Send + Sync` を本 unsafe impl が満たすため）`Send` パラメータで `is_send==true`。MultiThreadedExecutor は `!is_send && local_thread_running` のときのみメインスレッドに固定する（`multi_threaded.rs:545`）ため、`is_send==true` の本システムは**メインスレッドに固定されずワーカースレッド上で実行され得る**。よって旧注記の「単一スレッドの排他アクセス」は不正確。
- **(d) それでも健全な理由（write 経路）**: `Query<&mut DolaAnimator>` は**排他（&mut）**アクセスであり、bevy のアクセス競合スケジューリングにより、ある `DolaAnimator` を同時に触れるのは常に高々1スレッド。内部 `Rc` の参照カウント操作・`clone` もその単一システム（`tick_dola_animators` の `iter_mut()`＝**逐次**、`par_iter_mut` ではない）内で完結。複数 `DolaAnimator` 間にデータ共有もない。`DolaRuntime` の `Rc` 内包は実コードで確認（`DolaRuntime.timeline_manager`→`TimelineManager`→`ObjectInternPool.pool: HashMap<DynamicValue, Rc<DynamicValue>>` ＝ dola `runtime/interpolator/mod.rs:19-20`、`UpdateResult.changes: Vec<(i64, EvaluatedValue)>` の `EvaluatedValue::Object(Rc<DynamicValue>)` ＝ dola `runtime/types.rs:24,144-146`）。
- **(e) Sync 側の潜在ハザード（read 経路）→ P68**: `Sync` は複数の読み取り専用システム（`Query<&DolaAnimator>`）が跨スレッドで `&DolaAnimator` を同時共有することを許す。`last_result()` が返す `UpdateResult.changes` は `EvaluatedValue::Object(Rc<DynamicValue>)` を含み得るため、**将来そうした並列消費者が内部 `Rc` を clone すると非アトミック参照カウントにデータ競合（UB）が生じ得る**。現状は消費者システム・spawn が皆無で到達不能。配線する際は対策必須ゆえ **P68** に記録（適用禁止・記録のみ）。
- **判定**: `unsafe impl` は**必須**（撤去すれば `DolaAnimator: Component` がコンパイル不能）かつ**現状健全**（到達不能＋write 経路は排他で安全）。ただし健全性の根拠は旧注記の「単一スレッド実行」ではなく「**排他アクセス＋現状未配線**」である。**挙動非破壊対策として SAFETY 注記をコメントのみで是正**（実構成の裏取り事実＋Sync ハザードを明文化）。配線時の堅牢化（Arc 化 / SingleThreaded スケジュール固定）は挙動変更ゆえ **P68**。

cue/ 側に他の `unsafe impl`・`unsafe` ブロックは**ゼロ**（grep 実証: cue/ プロダクションの `unsafe` ヒット 0 件）。境界内の唯一の unsafe は `DolaAnimator` の Send/Sync。

### 2. Entity ビット変換のラウンドトリップ安全性（cue/queue.rs:156-161）— 不正ビットで panic する外部入力到達経路 → 現状挙動を特性化（適用）＋堅牢化を P69 記録

`CueQueue::resolve_entity_ref`（queue.rs:156-161）は `CueCommand::EntityRef(bits)` を `Entity::from_bits(*bits)` で復元する。**実コード裏取りの結果、これは外部入力到達可能な panic 経路**:

- bevy_ecs-0.18.0 `Entity::from_bits`（`entity/mod.rs:576-581`）は**不正ビットで panic する非フォールバック版**（`try_from_bits` が None なら `panic!("Attempted to initialize invalid bits as an entity")`）。非 panic 版 `try_from_bits`（:590-599）が併存するが現実装は未使用。
- panic 条件（**一時 probe で実測確認**、推測排除）: 下位 32bit（index ワード）が **0** のとき。`Entity::try_from_bits` は `raw_index = bits as u32` を `EntityIndex::try_from_bits`（:201-208）に渡し、これは `NonZero::<u32>::new(bits)` で **raw=0 を拒否**（`NonMaxU32` の transmute 表現により raw=0 が無効インデックスに対応）。probe 実測値: `bits=0x0000_0001_0000_0000`（generation=1,index=0）→ **panic**、`bits=0x0000_0000_FFFF_FFFF`（下位=u32::MAX）→ **正常復元**（当初想定と逆。transmute 表現のため）、`bits=0`（all-zero）→ panic。
- **外部入力到達性**: `CueCommand` は `Serialize, Deserialize` 導出（dola `cue/command.rs:117-118`）で `EntityRef(u64)`（:128）は**外部 CueSheet（ファイル/設定由来）の任意 u64 を運び得る**。現状リポジトリでは `push_entity_command`（queue.rs:139-146）が常に有効な `entity.to_bits()` を挿入し、外部 CueSheet から EntityRef を流す利用箇所が未実装のため実害は未発現。ただし `resolve_entity_ref` 自体は任意ビットを検証せず、ドキュメント（queue.rs:148-155）が「無効な Entity が返る可能性があるため消費者は Query で存在確認」と謳う一方、現実装は panic（契約と実装の不一致）。
- **判定**: W8-T1 が往復恒等を有効ビットのみで特性化済みだが本 panic 経路は未保護だった。**挙動非破壊対策として現状の panic 挙動を `#[should_panic]` で固定する特性化テストを追加**（適用2・回帰検知器）。堅牢化（`from_bits`→`try_from_bits` で `None` 縮退）は `resolve_entity_ref` の戻り値が panic→`None` へ変わる**外部観測可能な挙動変更**ゆえ R2.4/R5.2 に従い **P69** 記録（適用禁止）。P25（Cue パイプライン時刻入力 NaN/inf 検証欠如）と同系統の「外部 CueSheet 由来データの検証欠如」。

### 3. スケジュール時刻の整数境界 — 現状安全（対策不要）。深部 i64/u64 時刻計算は境界外 dola（P25 参照）

cue/ プロダクションの `as` キャスト・算術を全数判定（grep `as iXX|uXX|fXX`・`+`・`*` を test 除外で実施）:

- **cue/ プロダクションに整数キャスト（`as iXX/uXX`）は皆無**（grep ヒット 0 件）。`TimedSchedule` のスケジュール時刻計算（i64/u64 キャスト・加減算）は **dola `cue/schedule.rs`（D3 領域）に存在**し、cue/command.rs は `pub use dola::cue::{... TimedSchedule ...}` で再エクスポートするのみ＝**境界外**（読み取りで確認、変更不可）。FrameTime からの時刻変換も `dola::runtime::clock::now()`（`world/mod.rs:50,465`・境界外）由来で cue/ には整数変換なし。
- **`dispatch.rs:94,135` `let absolute_time = start_time + cue.start_time`**: 両者 `f64`。f64 加算はオーバーフローで panic せず inf へ飽和（整数オーバーフローのサプライズなし）。NaN/inf の素通り自体は **既知 P25**（Cue パイプラインの時刻入力検証欠如）に該当し、本 V セルでは新規採番せず**参照に留める**。
- **`queue.rs:189` `if self.schedule.remaining() + entries.len() > cap`**: `TimedSchedule::remaining() -> usize`（dola `cue/schedule.rs:254` で確認）＋ `Vec::len() -> usize` の `usize + usize`。`usize::MAX` への到達は各エントリがメモリを占有する以上**物理的に不可能**（実機到達不能の DoS ではない）。debug_assert は「常に偽にならない＝発火しない」ためチャーンのみで投入見送り（karpathy 3）。`queue.rs:165-169/188-192` の capacity 検査自体は W8-T1 が境界（ちょうど/超過/アトミック拒否）を特性化済み。
- **判定**: cue/ 境界内に整数変換起因の切り捨て/符号反転/オーバーフローはなし。**現状安全（対策不要）。** 深部時刻計算は境界外 dola（P25 が既知）。

### 4. panic 経路（cue/+dola/）— 上記2の `from_bits` 以外はすべて現状安全（対策不要）

境界内プロダクション経路の `unwrap()`/`expect()`/`panic!`/`unreachable!`/`todo!`/生添字 `[i]` を個別判定:

- **cue/+dola/ プロダクションの `unwrap`/`expect`/`panic!` は、上記2の `Entity::from_bits`（間接 panic）を除き、すべて `#[cfg(test)] mod tests` 内**（grep 実証: queue.rs:475+/tracker.rs:277+/dola mod.rs:187+ の各テスト。プロダクション本体には到達可能な `unwrap/expect/panic!` なし）。tracker/queue/registry/dispatch/systems は `Result`/`Option`/`TrackerAction` を返す設計で、バリア解決・完了・キャンセル・エラーは値写像（W8-T1 が全面特性化）。dola `DolaAnimator` は `DolaRuntime` への薄い委譲で panic 経路なし（W8-T2 が委譲契約を特性化）。
- **生添字 `[i]`**: cue/+dola/ プロダクションに危険な生添字なし（grep 確認）。
- **判定**: 上記2（外部入力到達可能な `from_bits` panic、P69 へ）以外に外部由来データでクラッシュにつながる panic 経路は**検出されず。現状安全（対策不要）。**

## 適用した挙動非破壊対策（2 ファイル・2 箇所）

| ファイル | 箇所 | 対策 | 種別 | 根拠 |
|----------|------|------|------|------|
| `dola/mod.rs` | `unsafe impl Send/Sync` 直前の SAFETY 注記（:50-85）＋モジュール doc `# 安全性`（:6-15） | W8-S の旧注記が主張した「単一スレッド実行／par_iter_mut で並列化されない」という**実構成と不整合な根拠を是正**し、実コード裏取り事実（既定マルチスレッドエグゼキュータ・現状未配線で到達不能・健全性は排他 &mut アクセスに依拠）＋Sync 側潜在ハザード（並列読み取りでの Rc clone データ競合・P68 参照）を明文化 | SAFETY/不変条件コメント | **コメントのみ**・コード挙動不変。`unsafe impl Send/Sync` の2行（:86-87）は無印コンテキスト行＝不変。型・シグネチャ・unsafe の意味論・生成コードに一切非介入。`cargo build --workspace` 成功＋S2 全量が +1（テスト追加分のみ）で実証。 |
| `cue/queue.rs` | in-source `mod tests`（`resolve_entity_ref_returns_none_for_non_entity_ref` の直後、:535-557 付近） | `resolve_entity_ref_panics_on_malformed_bits`（`#[should_panic(expected = "invalid bits")]`）＝不正ビット（index ワード=0、`0x0000_0001_0000_0000`）に対する**現状の panic 挙動を固定する特性化テスト** | 特性化/回帰テスト（S9 命名準拠・in-source） | 現状挙動の characterization（GREEN by construction）。リリース/デバッグ挙動を変えず、P69（try_from_bits 化）適用時に RED 化して堅牢化を検知する回帰検知器。 |

### 追加した特性化テスト一覧（in-source 1 件）

- `cue/queue.rs::resolve_entity_ref_panics_on_malformed_bits` — `CueCommand::EntityRef(0x0000_0001_0000_0000)`（generation=1,index=0 の、`to_bits()` 由来でない外部 u64 を模す不正ビット）を `resolve_entity_ref` に渡すと `Entity::from_bits` が panic することを `#[should_panic(expected = "invalid bits")]` で固定。外部 CueSheet 由来データの panic DoS 経路（P69）の回帰検知器。

是正した SAFETY 注記の実コード根拠は本断片「点検手法」「発見1」に列挙のとおり、すべて bevy_ecs-0.18.0 ソース・ルート Cargo.toml・probe 実測で裏取り済み（W7b-V の未確認本番事実主張による REJECTED 教訓を踏まえ、単一/並列スレッド・スケジュール構成・Send/Sync・panic 条件をすべて実コードで確認）。

## proposals.md へ回した候補（P68・P69）

- **P68**: `DolaAnimator` をマルチスレッドスケジュールへ配線する際の Sync ハザード対策（内部 Rc の Arc 化 or SingleThreaded スケジュール固定）。kind: 挙動変更を伴う脆弱性対策。現状未配線ゆえ到達不能だが、配線時に並列読み取り消費者が内部 `Rc` を clone するとデータ競合（UB）。対策は型変更（dola D 領域）またはスケジュール属性変更を伴い挙動/スレッドモデルを変えるため記録のみ。W8-S が SAFETY 注記を格上げ・本 V が実構成裏取りで Sync ハザードを特定。
- **P69**: `resolve_entity_ref` の Entity ビット復元を `try_from_bits` 化（不正ビット・外部 CueSheet 由来の panic 経路の Result/None 縮退）。kind: 挙動変更を伴う脆弱性対策。`from_bits`→`try_from_bits` の1行修正で panic→`None` 縮退（ドキュメント記載の契約に実装を一致）。戻り値挙動の変更ゆえ記録のみ。本 V が `#[should_panic]` 特性化で現状挙動を固定済み。

既知 proposals の再発見（重複記録なし・参照に留めた）:
- **P25**（W4b 系・Cue パイプラインの時刻入力 NaN/inf 検証欠如）: 発見3で `dispatch.rs` の `f64` 時刻加算・NaN/inf 素通りを再確認したが既知ゆえ参照のみ（cue/ 境界内の時刻入力検証は P25 のスコープ）。
- **P67**（W8-S・dispatch 配送アーム重複統合）: ロジック構造変更を要する簡素化候補（S 観点）で V 観点の脆弱性ではないため二重記録せず参照に留める。

## verification (S2)

- BEFORE: 親検証済みベースライン（W8-S 直後 = **1712 passed / 0 failed**、HEAD = `37ee729`・クリーンワークツリー）を信頼し省略（design フェーズ0 規定 + 親指示「BEFORE S2 は省略可」に従う）。
- AFTER（必須・全量実施）:
  - `cargo build --workspace` → **成功**（exit 0、wintf/areka 再コンパイル、20.79s）。
  - `cargo test --workspace` → **1713 passed / 0 failed**（ignored 32。全22本の `test result:` 行を awk 合算で実測 `passed=1713 failed=0 ignored=32`）。`test result: FAILED`/`^error[`/`^error:`/`panicked` 行ゼロ。ベースライン 1712 から **+1 = 追加した特性化テスト1件と一致**（既存1712件の削除・変更ゼロ＝全既存テストが SAFETY 注記是正後コードをそのまま通過＝挙動非破壊の裏付け）。
  - 反復検証: `cargo test -p wintf --lib cue::` で cue in-source **33 passed / 0 failed**（W8-T1 の 32 + 新規1）。`--lib dola` の dola in-source は **12 passed / 0 failed**（W8-T2・本セル追加なし、不変）。`--test ecs` は **102 passed / 0 failed**（cue/dola 統合・`cue_performance_test` 含む全件緑、不変）。
  - 追加1件は `#[should_panic]` 特性化（GREEN by construction）。**初回は不正ビット想定（下位=u32::MAX）が誤りで RED 化**→一時 probe で `from_bits` の真の panic 条件（下位 index ワード=0）を実測し、テストとドキュメント（断片・P69）を真の条件へ修正（プロダクション挙動不変・特性化の正常収束）。
- 変更ファイル（`git diff --numstat` 実測）: `dola/mod.rs`（**+39/−12**、コメントのみ。SAFETY 注記是正＋モジュール doc 是正）・`cue/queue.rs`（**+20/−0**、in-source 特性化テスト1件のみ）・`proposals.md`（**+12/−0**、P68/P69 追記・断片外の記録ファイル）。boundary（cue/+dola/）内のプロダクションコード**ロジック変更ゼロ**・新規テストファイルなし・tests/ 不変。一時 probe（cue/ に追加した一時 mod）は検証後**完全撤去**（git status クリーン＝上記3ファイルのみ）。

## clippy（S3・記録のみ・非ブロッカー）

- `cargo clippy -p wintf --lib --tests --message-format=short` の cue/+dola/ 境界内 simplification 系 lint:
  - **`collapsible_if`: 5**（queue.rs:165/188/344・registry.rs:54・systems.rs:44。**W8-S 記録と完全一致**＝BEFORE=AFTER 不変。すべてプロダクションコードの既存 lint で R5.5/churn 回避により据え置き、let-chain 不採用慣習）。
  - その他の simplification 系 lint は境界内に**ゼロ**。
  - **本セルの編集（SAFETY コメント是正＋`#[should_panic]` テスト1件）は新規 clippy 警告/error を一切導入していない**（AFTER で境界を再 lint し W8-S と同一の5件のみを確認。追加テストの `0x0000_0001_0000_0000` リテラルを指す診断もゼロ）。
- S3 規定によりブロッカーとせず記録に留める。

## flaky

- 既知フレーキー `cue_performance_test::bench_pop_ready_empty_queue` は W8-T1 で**決定論化により解消済み**。本 AFTER S2 全量で `tests/ecs` バイナリは **102 passed / 0 failed**、反復の `--test ecs` 単独でも安定合格。本セルの変更（SAFETY コメント是正＋`#[should_panic]` テスト）は cue キュー timing と無関係。
- 本セル追加の `resolve_entity_ref_panics_on_malformed_bits` は純粋な決定論的 panic 特性化（タイミング依存なし）。フレーキー新規導入なし。

## 自己レビュー

- 実装は本物（モック/スタブ/プレースホルダ/TODO なし）。本セルの変更は SAFETY 注記是正（コメント）＋ `#[should_panic]` 特性化テスト1件のみで、新たな unsafe・スタブを導入していない。プロダクションロジック変更ゼロ。
- 点検は境界内10ファイルを grep＋精読で網羅。**最重要の `unsafe impl Send + Sync for DolaAnimator` の健全性を実スケジュール構成で裏取り**: (a) tick_dola_animators/DolaAnimator がプロダクション未配線（ワークスペース全域 grep）、(b) bevy_ecs `multi_threaded` feature 有効で既定エグゼキュータが MultiThreaded（bevy_ecs ソース + ルート Cargo.toml）、(c) UISetup のみ SingleThreaded 固定で Update 等は並列（world/mod.rs）、(d) Send 型クエリは is_send==true でワーカースレッド実行可（bevy_ecs ソース）、(e) write は排他で健全だが Sync の並列読み取り Rc clone がハザード（P68）。Entity ビット変換・スケジュール時刻整数境界・panic 経路も判定。**unsafe 健全性・panic 条件・スケジュール構成の本番挙動主張はすべて bevy_ecs-0.18.0 ソース直接精読・ルート Cargo.toml・一時 probe 実測で裏取り**（W7b-V の未確認本番事実主張による REJECTED を回避）。
- warranted な挙動非破壊対策は (a) DolaAnimator SAFETY 注記の実構成是正（旧「単一スレッド」根拠の訂正＋Sync ハザード明文化）と (b) Entity 不正ビット panic の `#[should_panic]` 特性化に限られた。挙動変更を要する対策（Sync ハザード堅牢化・from_bits→try_from_bits）は P68/P69 へ記録、既知 P25/P67 は参照に留めた。
- 件数の実測整合: S2 全量 1713 = 1712 + 1（追加テスト1）。cue lib 32→33、dola lib 12（不変）、tests/ecs 102（不変）。追加 `#[test]` git diff 実測 = 1（`#[should_panic]`）、削除 0、プロダクションロジック変更 0。clippy boundary 5（すべて既存・W8-S 一致・新規ゼロ）。すべて git diff・cargo test 実測と一致（推測なし）。
- 境界遵守: 変更は `dola/mod.rs`・`cue/queue.rs`（W8 境界内）＋ `proposals.md`（提案台帳）のみ。`crates/dola/` 本体・`ecs/world/` 等は読み取りのみ・変更なし。tasks.md 未更新・コミット未作成・境界外/`vendors/`/機能spec文書への変更なし。シェル出力を OS 一時パスへリダイレクトせず、リポジトリルートにスクラッチを残していない（一時 probe は撤去済み・git status で確認）。
- 結論: W8 領域は脆弱性耐性が高い。最重要の `unsafe impl Send + Sync` は**現状健全かつ必須**（未配線で到達不能＋write 経路は排他で安全）だが、W8-S の SAFETY 注記が主張した「単一スレッド実行」という根拠は実構成（既定マルチスレッド）と不整合だったため**実コード裏取り事実へ是正**し、配線時に顕在化する Sync ハザードを P68 へ警告記録した。Entity ビット変換は不正ビットで panic する外部入力到達経路を発見し現状挙動を `#[should_panic]` で固定・堅牢化を P69 へ。スケジュール時刻整数境界は cue/ 境界内に変換がなく現状安全（深部は境界外 dola・P25 既知）、panic 経路は from_bits 以外すべて現状安全。挙動非破壊で適用可能な対策のみを投入し churn を回避した。
