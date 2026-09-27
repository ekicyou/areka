# 設計レビュー: areka-P0-ghost-change-name-resolution

> 実施: 2026-09-27・ブランチ `claude/areka-p0-ghost-name-resolution-0d4fff`（`f953da8f`）。非対話で実施。design.md が既存コードについて述べる主張は、すべて実物のソース（関数名・型名＋ファイルパス）で裏取りした。

## レビュー要約

設計は「唯一の入口の中に、特別な名前を目録のフォルダ名へ解く純粋な関数を 1 段足す」形で、切替の握手・`GhostSpec`／`SwitchRequest`／`SwitchVerdict` の形・kanade・`compile.rs`・`change_cue.rs` に触らない。既存コードへの主張は実物と一致し、要件の受入基準 60 件はすべて追跡表に載り、判断の分岐（目録が空・今のゴースト 1 体だけ・他の候補 1 体・2 体以上と乱数の両端・末尾→先頭・今のゴーストが目録に無い・記録なし・記録のゴーストが消えた・同名のゴーストとの優先・切替中の 2 通目・メニューには掛けない）にはそれぞれ決定論テストが割り当てられている。残る懸念は文書側の 2 点（生成物の作り直しのタイミングと、手書き文書の 1 行）で、設計の骨格を変えるものではない。

## 実物との突き合わせ（主な項目）

| design.md の主張 | 実物 | 判定 |
|---|---|---|
| `request_ghost_switch` の順序は 予約 → 文脈 → 目録 → `resolve_switch_target` → 今のフォルダ名 → 置き場 → 送出 | `crates/areka/src/emo2_boot/ghost_switch.rs` の `request_ghost_switch` がその順 | 一致 |
| `resolve_switch_target` は `Name`＝`name` → フォルダ名の順・`Folder`＝フォルダ名だけ・大文字小文字を区別・今のゴーストも除外しない | 同ファイルの `resolve_switch_target` と説明 | 一致 |
| `pick_index` は `ghost_switch.rs` が `use` 済みで `boot_into` が `resolve_balloon_for_ghost(&root, dir, pick_index)` に渡している | `use crate::boot_resolve::{…, pick_index, …}`・`boot_into` の呼び出し | 一致 |
| `pick_index(n)` は `RandomState` 由来で `% n` | `crates/areka/src/boot_resolve.rs` の `pick_index` | 一致（`n = 0` で割り算が落ちるので「候補 1 体以上のときだけ呼ぶ」の約束は必須。設計はそう書いている） |
| argv 起動は `folder: None` | `boot_resolve.rs` の `resolve_ghost` 段 1 | 一致 |
| `decode_bare` の腕は `e`・`c`・`-`・`n`・`0|h`・`1|u`・`f`、既定は `decode_passthrough_bare` → `Raw`。裸 `\f` → `Font { args: [] }` が前例。各腕に `// ukadoc: <URL>` | `crates/areka-parsers/src/sakura/decode.rs` の `decode_bare` | 一致 |
| `\![change,ghost,random]` は `decode_bang` → `decode_passthrough_bang` → `GenericCommand { name: "change", raw_args: ["ghost","random"] }` | 同ファイル `decode_bang`／`decode_passthrough_bang` | 一致 |
| `Instruction` は `#[non_exhaustive]`・`compile` の `GenericCommand` の腕が `command_carrier(name, raw_args)` へ載せる | `model.rs`・`crates/areka-sakura/src/compile.rs` | 一致 |
| `ChangeCueSink` は `("change","ghost")` だけ受理・第 2 引数を無変形・`raise_event` は `params[2..]` の `--option=raise-event` | `crates/areka/src/emo2_boot/change_cue.rs` | 一致 |
| 字句は `\+` → `Bare("+")`・`\_+` → `Bare("_+")`・`\+[x]` → `Tag { word: "+" }`・直後の本文は残る | `lexer.rs` の `scan_tag`／`bare_tag_len`・`lexer_word_boundary_tests.rs` の `ONE_CHAR_WORDS`（`"+"` を含む）・`lexer_bare_tag_tests.rs` | 一致 |
| 書き換えが要る既存テストは `unknown_names_warn_once_and_send_nothing`（`["Nobody","random","lastinstalled"]`）と `each_canonical_bracketless_tag_yields_exactly_one_raw`（`CANONICAL_BRACKETLESS_SPELLINGS: [&str; 12]` に `"_+"`）の 2 本だけ | 両ファイルの実物。ソース全域に `\+` を `Raw` として固定する他のテストは無い（`lexer_word_boundary_tests.rs` の `_+` は字句だけを見る） | 一致・設計に明記済み |
| メニューは `GhostSpec::Folder` で入口を呼ぶ | `crates/areka/src/menu/ghost_frame.rs` | 一致 |
| `Resource` の前例 `SessionEnded`・`#[allow(dead_code)]` に消費側の時期を注釈する前例 | `session_end.rs`・`input_events/throttle.rs` ほか | 一致 |
| 台帳 `\+`（`_5c_2b:1`）・`\_+`（`_5c__2b:1`）は `absent`・owner 空・B2。`\![change,ghost,…]` は `same-feature` で両者を指し、備考に本仕様の名前 | `doc/ukadoc-coverage/ledger/sakura-script.toml` | 一致（本仕様を `owner` に持つ項目は現在 0 件なので `owner_count = 2` は正しい） |
| `roadmap-draft.md` は `[briefs].count = 35`・本仕様の行なし・完了 `ghost-shell-balloon-switch` の「2026-09-27 の追加」が手本 | 同ファイル | 一致 |
| 整合検査の腕 a／c／f（`spec_checks.rs`）と `check_evidence`（`check/content.rs`・実装済みなのに `// ukadoc:` 証拠が無いと赤） | `crates/ukadoc-survey/tests/consistency/spec_checks.rs`・`src/check/content.rs` | 一致 |
| ファイルの大きさ 699／545／391 行 | `wc -l` | 一致 |
| §8「角括弧なし `\_` タグ」の行が `\_+` を所有先未定と書く | `doc/COMPAT_ARCHITECTURE.md` の該当行 | 一致 |

## 重要な指摘（最大 3 件）

### 🔴 指摘 1: 台帳を変えた同じコミットで `report/*.md` を作り直さないと `cargo test -p ukadoc-survey` が赤になる——設計はそれを「後に main へ入る側」の作業のように読める

- **懸念**: `ukadoc-survey` の常時検査には、⑴ ドメイン別報告（`report/sakura-script.md` 等）が台帳より古いと赤にする `DomainReportStale`（`crates/ukadoc-survey/tests/consistency/checks.rs`）と、⑵ `report/summary.md` の本文がカタログと台帳 4 本から作り直した本文と全文一致することを求める判定 ⑹（`documents_checks.rs` の `the_summary_report_is_as_fresh_as_the_ledgers`）がある。`\+`・`\_+` を `absent` → `implemented` に改めた瞬間、両方の件数が動く。設計の Modified Files には `report/*.md` が無く、要件 7.3 と File Structure Plan の文「並走する spec と同時に変わる数は、後に main へ入る側が作り直す」は、本ブランチでの作り直しを省いてよいように読める。Goals の「整合検査が緑のまま」はこの作り直しを前提にしている。
- **影響**: 台帳と生成物のコミットが分かれると、その間の全体テストが赤になる（`/kiro-complete` は全体テスト 1 回を通す）。完了 `ghost-shell-balloon-switch`（PR#192・`5a232d2f`）は `ledger/*.toml`・`report/sakura-script.md`・`report/shiori.md`・`report/summary.md`・`roadmap-draft.md` を同じ PR で更新している。
- **提案**: Modified Files に `doc/ukadoc-coverage/report/sakura-script.md`・`report/summary.md`（`shiori.toml` は備考だけの変更なので `report/shiori.md` は動かない見込みだが、生成器の出力をそのまま採る）を足し、「台帳の状態を変えるタスクは同じタスクで `cargo run -p ukadoc-survey -- report` と `report-summary` を走らせ、`cargo test -p ukadoc-survey` が緑であることを完了条件にする」と書く。「後に main へ入る側が作り直す」は「合流で衝突したときの解き方」として残す。
- **Traceability**: 要件 7.1・7.3・7.6
- **Evidence**: design.md「File Structure Plan」「Out of Boundary」（生成物の行）・「Goals」5 点目・追跡表 7.3

### 🔴 指摘 2: 手書き `briefing-sakura-script.md` の `\![change,ghost,…]` の行が「特別な名前の解決は本仕様の持ち場」と書いたまま残る

- **懸念**: `doc/ukadoc-coverage/briefing-sakura-script.md` の `\![change,ghost,ゴースト名(,--option=raise-event)]` の行は「特別な名前 `random`／`sequential`／`lastinstalled` の解決は `areka-P0-ghost-change-name-resolution` の持ち場」と未来形で書いている。設計が同ファイルで改めるのは `\+`・`\_+` の「未対応」の行と `\_+` の「無所有一覧で裁定」の行だけで、この 1 行が漏れる。台帳（7.2）と `shiori.toml`（7.8）では同じ文を「解決済み」へ改めるのに、手書きの側だけ古い文が残る。
- **影響**: 完了後に台帳だけを読む開発者と手書き文書を読む開発者で、`random` 等の状態の読みが食い違う（要件 7 の目的「台帳と裁量の記録が実装と一致する」に反する）。機械の検査には掛からないので、見落とすとそのまま残る。
- **提案**: Modified Files の `briefing-sakura-script.md` の項に「`\![change,ghost,…]` の行の『解決は本仕様の持ち場』を『解決済み（本仕様・`resolve_special_name`）』へ改める」を 1 文足す（7.2 と同じ文面）。
- **Traceability**: 要件 7.2・7.9
- **Evidence**: design.md「File Structure Plan」の `briefing-sakura-script.md` の項・追跡表 7.9

### 🔴 指摘 3: 既存 `ghost_switch_unknown` の本文（「切替先が目録のどのゴーストにも一致しない」）が新しい 4 つの理由に合わない

- **懸念**: 設計は既存の `warn!(event = "ghost_switch_unknown")` に `reason` 欄を足す（`name`／`random_empty`／`sequential_empty`／`lastinstalled_none`／`lastinstalled_missing`）。既存の腕の本文は「目録のどのゴーストにも一致しない」で、`random_empty`（目録が空）や `lastinstalled_none`（記録が無い）には当てはまらない。設計は `reason` の語彙は決めているが、本文（人が読む 1 文）を理由ごとに分けるのか共通にするのかを書いていない。
- **影響**: 実機サインオフで `ghost_switch_unknown` を grep したとき、本文が実態と食い違う行が出る（「一致しない」と読めるのに実際は「記録が無い」）。小さいが、要件 6.1 の「それぞれを見分けられる」を本文でも満たすのが自然。
- **提案**: SwitchEntry の Event Contract に「本文は理由ごとに 1 文（例: `random` の候補が無い／目録が空・`lastinstalled` の記録が無い／記録のゴーストが目録に無い）」と書き、`UnresolvedReason` に `as_ref_str` と並べて短い説明文を返す口を 1 つ足す（`match` 1 つで済む）。
- **Traceability**: 要件 6.1
- **Evidence**: design.md「SwitchEntry」Responsibilities 4 点目・「Event Contract（記録）」

## 設計の強み

1. **形を変えずに 1 段足すだけ**。`GhostSpec`・`SwitchRequest`・`SwitchVerdict`・`resolve_switch_target`・字句解析・`compile.rs`・`change_cue.rs` を触らず、解けた名前を `GhostSpec::Folder` に読み替えて今日の経路へ流す。並走する `session-mark-residue`・後続の `ghost-install`／`shell-balloon-switch` への申し送りが要らない（研究 §2.8 の約束をそのまま守る）。転記の 2 腕は裸 `\f` の前例と同型で、`\+[…]` を作らないことも字句の性質から自然に成り立つ。
2. **判断と記録の語彙が揃って決まっている**。純粋な関数の入力（目録・今のフォルダ名・記録・乱数）と出力 3 通り、`pick` を呼ぶ条件（候補 1 体以上のときだけ 1 回）、解けない 4 理由の `reason` の語、解けたときの `ghost_switch_resolved` が一体で定義され、要件 6.3 の分岐すべてに単体／統合テストが番号付きで割り当てられている。既存の道具立て（`entry`・`fixture_root`・`world_with_slot`・`assert_one_event`）を使い回す判断も妥当（新しい兄弟ファイルにすると `_test_support.rs` への括り出しが増える）。

## 最終判定

**GO**

- **理由**: 既存コードへの主張はすべて実物と一致し、境界・依存の向き・ログ無し失敗経路の禁止・決定論テスト網羅の規律に沿う。3 件の指摘はいずれも文書の追記と本文 1 文の追加で済み、設計の骨格を変えない。
- **次の段**: 設計ディスカッションで指摘 1〜3 を design.md に反映（Modified Files に `report/*.md` の作り直しと `briefing-sakura-script.md` の 1 行を足す・Event Contract に本文の分け方を足す）してから `/kiro-spec-tasks areka-P0-ghost-change-name-resolution` へ進む。タスク生成では、設計が番号で示したテストに関数名を付け、台帳を変えるタスクと生成物を作り直すタスクを同じタスクにまとめること。

## 補足（判定を変えない観察）

- `pick_index(0)` は `% 0` で落ちる。設計の不変条件「`pick` は候補 1 体以上のときだけ呼ぶ」がそれを防いでいるので、単体テスト 3・5・6・7 の「`pick` は呼ばれない（呼ばれたら panic する閉包）」は必ず実装すること。
- 入口の統合テストは `fixture_root`（`A`・`B` の 2 体）で組むため、「目録 1 体だけ」「今のゴーストが目録に無い」は純粋な関数の単体テストだけで固定される。入口が `current` を正しく渡すことは統合テスト 2（`sequential` の A→B と B→A）が証明するので、判断の分岐だけを固定する方針（steering）に照らして足りる。
- `roadmap-draft.md` の `wave = "B4-②"` は `.kiro/steering/roadmap.md` の B4 の行（① `session-mark-residue` ∥ ② 本仕様）と一致する。`wave` は整合検査の対象外。
