# Design Validation Report: areka-P0-budoux-newline

- 日付: 2026-07-18
- 対象: `.kiro/specs/areka-P0-budoux-newline/design.md`（requirements.md R1–R9 確定済み）
- 検証方法: design.md の主要主張を現行コードベース（newline-defer マージ後の本ワークツリー）に対して実測照合した上で、design-review.md の観点（既存アーキテクチャ整合・一貫性・拡張性・型/インターフェース設計）でレビュー

## コード実測照合（design 主張の裏取り）

| design の主張 | 実測結果 |
|---|---|
| layout 4 段ゲート（①可視打切り→②保留フラッシュ→③折返し→④配置） | **一致**。`crates/areka-emo-text/src/layout.rs` ①L204・②L211（`pending.take()`）・③L228（`!current.is_empty() && inline_pos + advance > threshold`）・④L241 |
| `layout()` は全 items 受領＋可視は個数打切り＝全文 lookahead 可能 | **一致**。`layout.rs:169–204` |
| 軸読み替え正準表は単一式・分岐なし | **一致**。`layout.rs:182–186`（inline_start/block_start/block_dir） |
| `writing_mode` が転記→語彙解決の完全前例 | **一致**。`parse.rs:96`（転記1行）・`model.rs:38/90`（フィールド/accessor）・`writing.rs:63–76`（match＋未知値 `warn!`+フォールバック） |
| layout 呼出は 10 ファイル・約 53 箇所（本番は actor.rs 1 箇所） | **完全一致**。53 箇所/10 ファイル・本番は `actor.rs:482` のみ |
| `BalloonModel::new` 呼出は 16 ファイル・23 箇所 | **完全一致**。23 箇所/16 ファイル |
| 純粋層構造檻 `PURE_SOURCES`（windows import 禁止） | **一致**。`lib.rs:105`（state/writing/region/layout/canvas/viewbox 登録済み・wrap/segment の追加登録が必要という design 記述と整合） |
| fixture `emo2-kakukaku/descript.txt`（基層・UTF-8・Yu Gothic UI・wordwrappoint.x,-34） | **一致**。実在確認・`budoux_newline` 1 行追記で足りる形 |
| `ResolvedBalloonText::resolve` が mode/region/font の一点解決口 | **一致**。`actor.rs:106–108`（`WritingMode::resolve` と並ぶ形に `wrap` を足すだけの配線） |

事実誤認・鮮度ズレは検出されなかった（research §6.1 の再実測が design 生成直前に行われており、行番号・箇所数まで現物と一致する）。

## Review Summary

設計は現行コードの実態と細部まで一致しており、変更面がゲート③の enum 分岐・新規純粋モジュール 2 個・配線 1 箇所に厳密に閉じている。最大リスクだった R7×R5（リフロー跳び×保留改行）は INV-1/INV-2/INV-3 の不変条件と prefix 安定性檻で決定論檻に落ちており、OFF 非回帰（R4）は `WrapPlan::CharByChar` variant の構造分離で保証される。残る懸念は新規依存 budouy の実ビルド未確認と、plan 不整合時の縮退契約の記述整合という実装順序・明文化レベルの 2 点で、いずれも設計骨格を揺るがさない。

## Critical Issues（最大 3）

### 🔴 Critical Issue 1: budouy 0.2.2 の実ビルド・API 実形が未確認のまま設計が確定している

- **Concern**: budouy の API（`budouy::model::load_default_japanese_parser()`・`Parser::parse(&str) -> Vec<&str>`・`vendored-models` feature 名）は docs.rs 参照のみで、workspace 実ビルド（x64/arm64）・`Parser: Sync` の成否・ロード API が `Result` を返すか否かが未検証（research §4-2 が自ら「実ビルド確認が要る」と挙げたまま §6 で消化されていない）。design は thread_local 退避・`error!`+OFF 縮退写像で吸収する方針を書いているが、`OnceLock` static か `thread_local!` かで segment.rs の形とテスト形が変わる。
- **Impact**: 実装後半でこの分岐を踏むと segment.rs の檻の書き直しが発生する。逆に最初に踏めば影響ゼロ。
- **Suggestion**: tasks 生成時に「Task 1 = budouy 依存追加＋最小 spike（vendored-models で parse 1 回・`Sync` 確認・ロード API の Result 有無確認）」を独立の先頭タスクとして切り、その結果で `OnceLock` / `thread_local!` を確定してから segment.rs 本実装へ進む順序にする。
- **Traceability**: R8.1（決定論・オフライン）・R1.2（ON 解決の前提）
- **Evidence**: design.md「Technology Stack」「Responsibilities & Constraints（segment.rs）」「Risks（actor.rs）」「Error Handling — budouy モデルロード」

### 🔴 Critical Issue 2: plan 不整合時の縮退動作と「塊内は追加判定なし」の記述が食い違い得る

- **Concern**: System Flows は「塊先頭でない→配置（塊内は追加判定なし）」と描くが、Error Handling は「塊先頭に該当しないグリフは文字単位規則で配置（優しい縮退）」と規定する。両者は「塊内（先決済み・remaining カウンタで追跡）」と「plan 非被覆（不整合）」を区別して初めて両立する——素朴に `segment_starting_at` の Some/None だけで分岐すると、非被覆グリフが折返し判定を一切通らず無限に行内へ積まれる実装が書けてしまう。
- **Impact**: 本番配線は 1 箇所で同一 items から plan を導出するため実害は通常到達しないが、契約が曖昧なまま実装に入ると「優しい縮退」の檻が書けない（何をアサートすべきか不定）。
- **Suggestion**: 実装時に「塊内 = 先決済み塊の残グリフ数カウンタで追跡（判定なし配置）」「plan 非被覆 = 既存 CharByChar 式で判定」の 2 状態を明示的に分け、非被覆グリフの文字単位縮退を 1 檻固定する（design 本文の変更は不要・tasks の受け入れ条件へ落とせば足りる）。
- **Traceability**: R2.3（塊内は途中分割なし）・Error Handling の呼び手契約
- **Evidence**: design.md「System Flows — 塊内は追加判定なし」vs「Error Handling — plan と items の不整合」・「Service Interface（layout）Preconditions」

### （3 件目なし）

上記 2 件以外に成功を左右する懸念は見当たらない。glyph 通し番号写像（placed カウンタと同一の数え方で 0 起点整合）・INV-3 の空行不生成（行頭で `cap_rem == cap_full` ゆえガード無しで成立）・縦書きの新規分岐ゼロ（行内軸演算のみ）は、いずれも現物コード構造の上で成立することを確認した。

## Design Strengths

1. **実測に裏打ちされた変更面の最小化**: 4 段ゲートの行番号・呼出箇所数（53/23）・fixture 内容まで design 記述が現物と完全一致しており、変更がゲート③ + 新規 2 モジュール + 配線 1 箇所に閉じることが検証可能な形で示されている。実装の曖昧さがほぼない。
2. **リスクの構造的排除**: OFF 不変を enum variant で構造保証（既存檻がそのまま非回帰檻を兼ねる）、リフロー跳びを INV-1/INV-2 の不変条件＋prefix 安定性檻（可視段階増加で全量 prefix 一致）で決定論檻に落とし、空行不生成（INV-3）をガード無しの数式的成立にした点は、[test-only-decision-branches] と決定論テスト網羅の steering 方針の模範的適用である。

## Final Assessment

- **Decision: GO**
- **Rationale**: 既存アーキテクチャとの整合（writing_mode 前例の写経・4 段ゲートの③のみ拡張・純粋層規律・state.rs 非改変による W3 干渉回避）が現物照合で裏付けられ、要件 R1–R9 のトレーサビリティが完備している。残る 2 件はいずれもタスク順序（依存 spike の先頭配置）と受け入れ条件の明文化で吸収でき、設計変更を要しない。
- **Next Steps**: 設計ディスカッション（kiro-design-discussion）で Critical Issue 1・2 の扱いを確認 → `/kiro-spec-tasks areka-P0-budoux-newline` でタスク生成（Issue 1 の spike を先頭タスク化・Issue 2 を該当タスクの受け入れ条件へ）。
