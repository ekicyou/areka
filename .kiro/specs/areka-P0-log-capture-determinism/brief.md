# Brief: areka-P0-log-capture-determinism

> **成立経緯**: areka-P0-input-events の `/kiro-complete`（開発者承認済み）実行中、DoD Test Gate（`cargo test --workspace`）で本欠陥が露呈。開発者判断（2026-07-23）により「このセッションでは実施せず別仕様に分離」となった。**実行プラン策定済み**（本ブリーフ末尾に全文埋め込み・tracing-core ソースレベルで根本原因確定済み）。本仕様の完了が input-events のマージを解除する。

## Problem

`areka-kanade` のテスト専用ログ捕捉基盤 `log_capture.rs`（元 areka-P0-kanade spec・commit 2f867cdd 由来・**32 個の回帰檻が共有**）が、`cargo test --workspace` の並列負荷下で **~1/10〜1/20 の確率でログを取りこぼす**。

- 実際の失敗: `actor::tests::shiori_send_failure_maps_to_ipc_and_logs` —「期待ログ未検出: target="kanade" level=ERROR event="shiori_send_failed"」（log_capture.rs:94 の assert）
- 再現条件: **workspace 並列実行時のみ**。クレート単体（`cargo test -p areka-kanade --lib` 136件）・当該テスト隔離×3 はすべて緑
- これは開発者の決定論テスト方針（[[deterministic-test-coverage-mandate]]）違反であり、**全 spec の kiro-complete DoD Test Gate を確率的に赤くする**プロジェクト横断の毒

### 根本原因（tracing-core 0.1.36 ソースで確定済み・再調査不要）

`capture()` が呼び出しごとに `with_default` で transient dispatcher を登録/破棄する設計が原因:

1. tracing-core の callsite Interest キャッシュは、**live dispatcher が 0 個の瞬間に rebuild が走ると `Interest::never` を焼き付ける**（`callsite.rs:505` `unwrap_or_else(Interest::never)`・sticky・次の `register_dispatch` まで復活しない）
2. 併発して max-level hint が `OFF` に落ちる窓もある（`callsite.rs:408,421` → `event!` マクロの静的 level チェックで捨てられる）
3. dispatcher 破棄は registry から即時削除されない（Weak 参照・`callsite.rs:553` の lazy prune）ため、並列テストで transient dispatcher が全滅した瞬間に別スレッドの callsite 初回 `register()`（`callsite.rs:308-342`）が走ると発症
4. `NoSubscriber::register_callsite` は `Interest::never()`（`subscriber.rs:676-678`）＝既定では何も Interest を支えない

なお log_capture.rs:79-83 には旧 flaky（`Arc::try_unwrap` 競合）の修正注記が既にあり、今回はその**より深層の同族**。

## Current State

- 該当ファイル: `crates/areka-kanade/src/schedule/log_capture.rs`（`capture()` + `assert_logged()`）
- 使用箇所: `actor.rs`（capture×4・うち line 647 は **TRACE レベル檻**）・`schedule/mod.rs`（capture×6・assert_logged 約30）・`schedule/boot.rs`（capture×1・**不在表明檻**＝イベント欠落で偽 PASS する最脆弱檻）
- areka-kanade の lib/integration とも**他の global subscriber 初期化は存在しない**（grep 済み・衝突なし）
- バージョン（Cargo.lock 固定）: tracing 0.1.44 / tracing-core 0.1.36 / tracing-subscriber 0.3.23
- 本欠陥は **main にも存在**する（input-events とは無関係に kanade spec 時代から）。input-events ブランチが並列負荷を上げて露呈させただけ

## Desired Outcome

1. `cargo test --workspace` が並列負荷下でも**決定論的に緑**（log 捕捉檻の取りこぼしゼロ）
2. `capture`/`assert_logged` の **API・意味論は不変**（32 檻・呼出10箇所は無改変で緑のまま）
3. 本仕様のマージ後、**areka-P0-input-events の kiro-complete が再開・完了**できる（下記「Downstream 継続手順」）

## Approach（確定済み・プラン Part A 参照）

**プロセスグローバル interest-keeper**: `capture()` 先頭で `OnceLock` により一度だけ素の `tracing_subscriber::registry()` を `set_global_default` する。

- `set_global_default` は subscriber Arc を **leak** する（`dispatcher.rs:314-319`）ため registrar が永久生存 → Interest は常に ≥ `Sometimes` に固定され Never 焼き付きが構造的に消滅
- 素の `registry()` は per-layer filter 無しゆえ `Interest::always()`/`enabled=true`/on_event no-op（tracing-subscriber `sharded.rs:222-235`）。**TRACE も通る**（actor.rs:647 の TRACE 檻が安全）
- `with_default` のスレッドローカル捕捉は thread-local dispatch が global を shadow するため**意味論不変**
- 導入直後に `tracing::callsite::rebuild_interest_cache()` を一度呼び、導入前に Never 焼き付き済みの callsite も再評価（保険）
- `set_global_default` 失敗（将来誰かが global subscriber を足した場合）は **expect で大声で落とす**（静かな flake 再発を防ぐ・log-first 規律）

却下済み代替案: ①テスト直列化 mutex（capture 外の step() 呼出テストが NoSubscriber 下で callsite を焼くため不十分）②リトライ/bound 拡大（確率を下げるだけ・恒久解でない）。

## Scope

- **In**:
  - `crates/areka-kanade/src/schedule/log_capture.rs` への interest-keeper 導入（~15行・1ファイル）＋ PITFALL doc 更新
  - RED→GREEN 検証（並列プロセス・ストレス＋workspace 反復。手順はプラン A-2）
  - （推奨・同境界の小差分）`crates/areka-kanade/tests/kanade/steady_test.rs:781` 周辺の同型 500-bound リトライループの park-barrier 化（input-events task 4.3 レビューで潜在 flake と指摘済み。mouse_test.rs cage 10 の修正が手本＝`spawn_harness_gated` の `expected_holds`/`hold_indices` バリア＋`join_bounded`）
- **Out**:
  - `capture`/`assert_logged` の API 変更・檻の書き換え（32 檻は無改変）
  - 本番コード（非テスト）への変更一切
  - input-events の機能追加・変更（マージ解除は本仕様の帰結であって作業対象はテスト基盤のみ）

## Boundary Candidates

- log_capture 基盤（interest-keeper）＝本仕様の唯一のコード境界
- steady_test の 500-bound ループ＝同境界内の二次修正（分離可能だが同 PR が経済的）

## Out of Boundary

- tracing/tracing-subscriber のバージョン更新・差し替え
- 他クレートのテスト基盤（wintf 等）への横展開（必要が観測されたら別仕様）
- kanade 本体の挙動・ログ語彙の変更

## Upstream / Downstream

- **Upstream**: areka-P0-kanade（completed・log_capture の出自）。tracing-core 0.1.36 / tracing-subscriber 0.3.23（Cargo.lock）
- **Downstream**: **areka-P0-input-events のマージがブロック中**（本仕様の完了が解除条件）。以降すべての spec の kiro-complete DoD Test Gate が安定化

## Existing Spec Touchpoints

- **Extends**: areka-P0-kanade（completed）のテスト基盤を修正（spec 自体は再開しない）
- **Adjacent**: areka-P0-input-events（未マージブランチ `claude/kiro-design-input-events-8a5a7a`・worktree `areka-p0-collision-geometry-51b0b4`・19 commits ahead / 0 behind・全タスク [x]・実機サインオフ済み・開発者承認済み）

## Constraints

- ビルド/テストは PowerShell で実行（Git Bash は link.exe 遮蔽等の罠）
- `cargo test --workspace` には i686 成果物が前提: `cargo build --target i686-pc-windows-msvc -p shiori-host32-helper -p shiori-host32-testdll`
- cargo-deny advisories の既存項目（main 由来）に新規 allow を足さない
- 1 feature = 1 branch = 1 PR（main への統合は PR squash のみ）

---

# 埋め込みプラン（策定済み実行手順・2026-07-23）

> 以下はブロック発生セッションで策定・探索済みのプラン全文。tracing-core の行番号根拠・kiro-complete 続行に必要な全事実を含む。**Part A が本仕様の実装、Part B は Downstream（input-events 完了）の継続手順**。

## Part A: log_capture 根治（本仕様の実装内容）

### A-1. 修正（1ファイルのみ）

**ファイル**: `crates/areka-kanade/src/schedule/log_capture.rs`

```rust
use std::sync::OnceLock;

/// プロセスグローバル interest-keeper（一度だけ導入・leak されて永久生存）。
static INTEREST_KEEPER: OnceLock<()> = OnceLock::new();

/// tracing callsite Interest の Never 焼き付き（tracing-core 0.1.36 callsite.rs:505:
/// live dispatcher 0 個で rebuild → Interest::never が sticky・max-level も OFF に落ちる）
/// を根絶する。素の registry() は全 callsite Interest::always / on_event no-op（tracing-
/// subscriber sharded.rs:222-235）ゆえ、Interest は常に ≥ Sometimes に固定され、
/// with_default のスレッドローカル捕捉（thread-local が global を shadow）は不変。
fn install_interest_keeper() {
    INTEREST_KEEPER.get_or_init(|| {
        tracing::subscriber::set_global_default(tracing_subscriber::registry())
            .expect("log_capture の interest-keeper より先に global subscriber を設定しないこと\
                    （フィルタ付き global は Interest を Never に焼き付け直し capture を壊す）");
        // 導入前に Never 焼き付き済みの callsite も再評価させる保険。
        tracing::callsite::rebuild_interest_cache();
    });
}

pub(crate) fn capture<F: FnOnce()>(f: F) -> Vec<CapturedEvent> {
    install_interest_keeper();
    // ...既存実装そのまま（with_default + mem::take）...
}
```

- API 不変 → 32 檻・呼出10箇所（actor.rs ×4 / schedule/mod.rs ×6 / boot.rs ×1）は無改変
- モジュール doc「決定性の要（PITFALL）」節へ根本原因と keeper を追記（line 79-83 の旧注記と統合）

### A-2. 検証（RED → GREEN）

1. **RED（修正前の再現・時間箱付き）**: `cargo test -p areka-kanade --lib --no-run` 後、`target/debug/deps/areka_kanade-<hash>.exe`（**mtime 最新を選ぶ**・stale が複数残存し得る）を **4 プロセス並列 × 25 ラウンド**起動し CPU 競合を再現。≥1 失敗で RED 確定。~100 実行で未再現なら「RED 未再現」と記録し GREEN 判定は workspace 反復に委ねる
2. **修正適用**
3. **GREEN**: 同ストレス 0 失敗 ＋ `cargo test -p areka-kanade` 全緑 ＋ **`cargo test --workspace` を 5 回以上連続**全緑（i686 前提ビルドを先に）

### A-3. 独立レビュー観点（kiro-review）

- tracing-core 根拠の妥当性（callsite.rs:505 / dispatcher.rs:314-319 / sharded.rs:222-235 をレビュアー自身が読む）
- API 不変・32 檻無改変・**boot.rs の不在表明檻**（イベント欠落で偽 PASS＝本修正の最大受益者）の保護
- expect メッセージ妥当性・OnceLock 競合安全・ストレス＋workspace 反復の再実行

### A-4. コミット

```
fix(areka-P0-log-capture-determinism): log_capture の callsite Interest 競合を根治（interest-keeper 導入）
```

---

## Part B: areka-P0-input-events の kiro-complete 継続手順（Downstream・本仕様マージ後）

### 引き継ぎ状態（2026-07-23 時点・探索済み事実）

| 項目 | 値 |
|---|---|
| ブランチ | `claude/kiro-design-input-events-8a5a7a`（19 commits ahead / 0 behind origin/main） |
| worktree | `C:\home\maz\git\areka\.claude\worktrees\areka-p0-collision-geometry-51b0b4`（名前は旧 spec 由来・実体は input-events） |
| tasks.md | 全 18 タスク `[x]`・実機サインオフ記録（Implementation Notes 4.5）込みで**コミット済み** |
| spec.json | まだ `phase: "tasks-generated"`（Step 4 で completed へ） |
| DoD | Spec Gate 済 / **Test Gate が本仕様マージ待ち** / License Gate 未実施（資産完備: deny.toml・about.toml・about.hbs・THIRD-PARTY-NOTICES.md、cargo-deny 0.20.2・cargo-about 0.9.1 導入済み） |
| 実機サインオフ | ①撫で talk（origin=OnMouseMove talk_id=7,11,12）②dblclick ③Ctrl+dblclick→exit 0 ④置換（talk_id=9→8 stale 破棄）⑤間引き、すべて実機ログ確認・開発者承認済み |

### 手順（kiro-complete ステップ 1 残り → 8）

1. **鮮度確認**: `git fetch origin` → `git merge origin/main`（本仕様のマージ分を取り込み）→ `cargo test --workspace` 再実行（今度は決定論的に緑のはず・これが Test Gate 証拠）
2. **License Gate**: `cargo deny check`（licenses/bans/sources 緑で通過。advisories の unmaintained/vulnerability は main 由来の既存事項＝報告のみ・allow を広げない）＋ `cargo about generate --workspace about.hbs -o THIRD-PARTY-NOTICES.md`（差分あればコミットへ）
3. **移動**: `Move-Item ".kiro/specs/areka-P0-input-events" ".kiro/specs/completed/"`（**spec.json 編集は移動後**・VSCode 復活バグ）
4. **spec.json**（新パスで）: `phase:"completed"`・`updated_at`・`approvals.implementation:{completed:true, completed_at}`
5. **参照パス更新（対象は 1 ファイルのみ）**: `.kiro/specs/areka-P0-choice-select-events/brief.md:73` の `.kiro/specs/areka-P0-input-events/brief.md` → `.kiro/specs/completed/areka-P0-input-events/brief.md`（bare-name 参照は多数あるが無改変）
6. **ROADMAP**: **`.kiro/steering/roadmap.md` のみ**（`doc/ROADMAP.md` はポインタ株・編集禁止）。W2 行（~line 185）/ M1 ゴール表（~line 170）を ✅ へ、追記㊱ を末尾様式（追記㉟=line 312 参照）で追加（completed +1・active −1）。⚠️ 長大行ゆえ Read は狭範囲/grep で
7. **完了コミット**: `chore(areka-P0-input-events): spec完了・アーカイブ`
8. **PR → squash マージ**:
   - `gh pr create --base main --head claude/kiro-design-input-events-8a5a7a`
   - `gh pr merge --squash --delete-branch --subject <s> --body <b>`
   - **subject 案**: `feat(areka-P0-input-events): マウス入力イベント（OnMouseMove/OnMouseDoubleClick）の UI→kanade→SHIORI 配信と stand-in 退役`
   - **body 骨子**: kanade（境界型・正典構築子・Steady GET・出所別置換/DD-6 保存政策）／areka UI 層（10Hz 間引き・MouseWiring＋mock シーム・ハンドラ・Ctrl+左 dblclick 暫定退避）／stand-in 退役（balloon 撤去・DD-IE-12）／決定論檻一式／placement 逆依存是正（example #[path] 不変条件回復）／実機サインオフ済み
   - 成否は**マージ API の結果のみ**で判定。`--delete-branch` のローカル削除警告は非致命。マージ後 `git ls-remote origin <branch>` で残存確認 → 残れば `git push origin --delete <branch>`（gh の worktree quirk）
   - ローカル worktree/ブランチの teardown はハーネス委譲

### 実行モデル（推奨）

1. **本仕様**（log-capture-determinism）: 新規ハーネス worktree ブランチで実装 → 自身の PR で main へ squash マージ（1 feature = 1 branch = 1 PR 遵守）
2. **input-events 完了**: 既存 worktree（上表）を再開し、Part B を実行（origin/main マージで本仕様の修正を取得 → Test Gate 緑 → 完了）

---

## 検証（本仕様の受け入れ）

| 段階 | コマンド | 合格基準 |
|---|---|---|
| RED | lib test exe 4 並列 × 25 ラウンド（修正前） | ≥1 失敗で再現確定（未再現なら記録の上先へ） |
| GREEN | 同ストレス（修正後） | 0 失敗 |
| Gate | `cargo test --workspace` × 5+（i686 前提ビルド後） | 全回 failed 0 |
| 回帰 | 32 檻すべて | 無改変で緑（特に boot.rs 不在表明檻） |
