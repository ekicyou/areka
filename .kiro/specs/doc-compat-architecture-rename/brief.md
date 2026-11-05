# Brief: doc-compat-architecture-rename

## Problem
`doc/` の中の名前の書き方は `doc-tree-tidy`（2026-11-05）で「小文字の英字・数字・ハイフン・点」にそろえた。ただ 1 つ、`doc/COMPAT_ARCHITECTURE.md` だけは大文字と下線のまま残っている。同じウェーブ C17 で並んで走る見直しの spec との約束で、`doc-tree-tidy` では名前を変えなかったためである。困っているのは、開発者と、後から文書を足したり名指したりするセッションで、決まりに合わない名前が 1 つだけ例外として残り続ける。

## Current State
- `doc/README.md` の「名前の決まりの例外」の表に `doc/COMPAT_ARCHITECTURE.md` の行があり、行方は「後で改名する（`doc-compat-architecture-rename`）」。照合テスト `crates/log-capture-kit/tests/doc_tree_guard_test.rs` は、この行が決まりの破れを覆っているかぎり緑。
- `doc/COMPAT_ARCHITECTURE.md` を名指すのは多い（2026-11-05 の数え・歴史の場所と `doc-tree-tidy` の文書を除く）: クレートの 18 ファイル（`areka-emo-text` 9・`crates/areka` 7〔`src/placement/persist.rs`・`resolver.rs`・`install/judge.rs`・`emo2_boot/move_cue_geometry.rs`・`ghost_session_scope_scale.rs`・`reference_brain.rs` など〕・`areka-ghost`・`areka-sakura`・`areka-sylphya` 各 1〜2）、steering 5 ファイル（`product.md`・`tech.md`・`structure.md`・`roadmap.md`・`focus.md`）、根の `README.md`、スキル `kiro-next`、`doc/` の中の文書（`anchor-compat.md`・`choice-cascade-compat.md`・`click-through.md`・`constitution.md`・`ukadoc-coverage/` の briefing と ledger の `*.toml`〔根拠の場所の欄〕）、未完了の spec の brief 60 本あまり。どれもコメントや文書の中のパスで、ファイルを開くコードは無い（`doc-tree-tidy` の再測定）。
- `doc/ukadoc-coverage/ledger/*.toml` の根拠の場所の欄は `ukadoc-survey` の道具が読む。値の書き替えで道具の判定が変わらないかは要件の段で確かめる。

## Desired Outcome
- `doc/COMPAT_ARCHITECTURE.md` が決まりに合う名前（推す名前は `doc/compat-architecture.md`）になり、中身は変わらない。
- `doc/README.md` の例外の表からその行が消え、直下の表の行が新しい名前になり、「動かした名前の対応」の表に 1 行足される。照合テストはそのまま緑。
- 歴史の場所（`.kiro/specs/completed/`・`doc/archive/`・`.kiro/steering/roadmap-history.md`）と対応の表のほかに、古い名前 `COMPAT_ARCHITECTURE.md` の名指しが残らない（大文字と小文字を区別する検索で確かめる）。

## Approach
`doc-tree-tidy` と同じやり方を取る。`git mv` で 1 つ改名し、同じコミットで案内の表と名指しを直す。名指しは語 `COMPAT_ARCHITECTURE.md` で探して直し、最後に歴史の場所だけで当たること（空振りしていないこと）と、除いた後に対応の表の行だけが残ることを確かめる。

## Scope
- **In**: `doc/COMPAT_ARCHITECTURE.md` の改名・案内の 3 つの表の直し・コード（コメント）・steering・スキル・根の `README.md`・`doc/` の中の文書・ukadoc-coverage の台帳・未完了の spec の文書の名指しの直し・最後の検索。
- **Out**: `doc/COMPAT_ARCHITECTURE.md` の中身の書き替え・ほかの文書の改名や移動・照合テストの作り替え・歴史の場所の書き替え。

## Boundary Candidates
- 改名と案内の表（`doc/README.md`）
- 名指しの直し（クレートのコメント・steering・スキル・`doc/` の中の文書・台帳・spec の文書）

## Out of Boundary
- `doc/COMPAT_ARCHITECTURE.md` に書かれた設計判断の中身
- `doc-tree-tidy` が作った照合テストの判定の仕組み（表の行を直すだけで足りる）

## Upstream / Downstream
- **Upstream**: `doc-tree-tidy`（名前の決まり・例外の表・対応の表・照合テストを作った）。
- **Downstream**: 無し。名指しの多い spec は、着地の後に新しい名前で grep し直す。

## Existing Spec Touchpoints
- **Extends**: 無し。
- **Adjacent**: `doc/COMPAT_ARCHITECTURE.md` を名指すファイルを受け持つ見直しの spec（2026-10-28 の数えで 9 本＝`audit-emo-text-layout-draw`・`audit-emo-text-state-actor`・`audit-parsers-sakura`・`audit-sylphya-actor`・`audit-ghost-catalog`・`audit-areka-app`・`audit-areka-install-update`・`audit-placement-core`・`audit-emo2-boot-switch`）と、本体のクレート分けの 6 本（`placement-crate-split` から始まる直列）。

## Constraints
- **同じウェーブに置かない**: `doc/COMPAT_ARCHITECTURE.md` を名指すファイルを受け持つ見直しの spec（上の 9 本・着手のときに grep で数え直す）と、本体のクレート分けの spec（`placement-crate-split` から始まる 6 本・どれも `doc/COMPAT_ARCHITECTURE.md`・網羅台帳の根拠の場所・`structure.md` を書き換える）とは、同じウェーブに置かない。どちらが先でも、後の方が新しいパスで grep し直す。
- 中身を変えない（変えてよいのは改名で食い違う自分の名前とパスの書き方だけ）。
- 依存を足さない。`Cargo.toml`・`Cargo.lock` に触らない。
- 名前の決まりと表の書き方の正本は `doc/README.md`。
- **規模**: S（3〜5）。直す所は多いが、どれも語で探して置き換える機械の直し。
