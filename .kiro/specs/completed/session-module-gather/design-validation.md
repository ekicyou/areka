# 設計の検証: session-module-gather

- 日付: 2026-11-21
- 対象: `design.md`（`a97dbab3c` の作業の木）・`requirements.md`・`research.md`・`spec.json`（language `ja`・phase `design-generated`）
- steering: `product.md`・`tech.md`・`structure.md`・`roadmap.md`（「本体の crate 分け」と C24 の行）
- 進め方: `kiro-validate-design` の REVIEW（分析 → 重大な指摘 → 良い点 → GO/NO-GO）。対話なしで実行した。

## 設計のまとめ

設計は、要件 1〜10 を全部 13 の部品（Components）に割り付けている。移すファイルの表・`main.rs` の項目とテストの仕分けの表・ソースを読むテストの表・直書きの出所の表・grep の式 5 つが具体的で、着手の順（段 0〜9）まで書いてある。下の「コードとの突き合わせ」で、要の主張はどれも今のコードと合っていた。残る指摘 3 件はどれも検証の網の穴を塞ぐもので、形を作り直す必要はない。

## コードとの突き合わせ（要の主張）

| 主張 | 確かめた所 | 結果 |
|---|---|---|
| `register_systems` は 25 の呼び出し・`menu` が 11 番目・`install`・`update`・`mcp` が 17〜19 番目 | `crates/areka/src/ghost_session.rs:43-80` | 合う（44〜79 行を数えて 25） |
| 本体だけの 4 本はどれも `fn(&mut World)` の形で、関数のポインタで渡せる | `menu/mod.rs:255`・`install/mod.rs:134`・`update/mod.rs:225`・`mcp/mod.rs:49` | 合う |
| `register_systems` を呼ぶのは本番では `main.rs:242` の 1 か所・テストは 5 か所 | grep | 合う（`app_exit_escape_tests.rs:137`・`ghost_session_reload_tests.rs:71`・`:179`・`ghost_session_restart_tests.rs:455`・`session/ghost_switch_test_support.rs:237`） |
| 触れない crate の説明が `ghost_session::register_systems` を名指す | `areka-input` 16 行・`areka-placement` 8 行・`areka-runtime` 2 行 | ほぼ合う（設計は `areka-input` を 15 とする。1 行の差で、結論の「名前を変えないので古くならない」は変わらない） |
| 移すファイルの `#[path]` はどれも同じフォルダの名前で、`../` は 0 件 | 移す 90 ファイルの `#[path]` を全部見た | 合う。入れ子の `#[path]`（`ghost_session_scope_rect_tests.rs:642` ほか）も同じフォルダの中なので、フォルダごと移せば解決は変わらない。ほかの crate・example・`build.rs` が本体の src のファイルを `#[path]` や `include_str!` で読む所は 0 件 |
| 残す `roots/edit`・`desk`・`pick` が芯を引くのは `super::` だけで、芯は残す 3 本を引かない | `roots/edit.rs:15`・`:16`・`:170` ほか・芯の 4 ファイル | 合う（芯から `edit`・`desk`・`pick` への参照は 0 件） |
| `main.rs` から移す項目に `info!` は 0 件 | `main.rs` の `info!` は 185・355・567 行（`fn main` と `after_run`）だけ | 合う（7.5 の手当ては要らない） |
| `RULES` は 14 行・`every_rule_target_names_a_module_or_literal_that_exists` は `src/session/<名>.rs` を探せる | `log_history.rs:115-132`・`log_history_convention_tests.rs:331-364`（`::` で区切って `src` の下を `.rs` か `mod.rs` で探す） | 合う |
| `crate::input_events` の呼び手は 26 ファイル・`runtime_tests/mod.rs` は引かない | grep（`main.rs` を含めて 27） | 合う |
| 名前で出所を読む直書きは `RULES` 4 行・`emo2_mcp_real_run.rs`・real-run-kit だけ | テスト以外の `"…areka::<集める名>…"` の grep | 合う |
| ソースを読むテストの表 | `readonly_tests.rs:82`・`:92`・`:153`・`session_end_deadline_tests.rs:405-533`・`readme_tests.rs:424` | 合う。`session_end_deadline_tests.rs:542`・`:550` の `"areka/main.rs"` は較正で写しに足す偽の行なので、直さなくてよい |
| 古いパスの grep の式 3 | 今の木で 102 件・どれも `areka/src/` を含む | ほかの crate の同じ名前のファイルへの誤った当たりは今は 0 件 |

## 重大な指摘

🔴 **重大な指摘 1**: 較正のテストで「残さない」の期待は、空振りしても緑になる
**懸念**: KindCalibration は `type_name` から最後の名前を落とした文字列で `classify` を引き、`alert`・`app_exit`・`session_end`・`exit_wait`・`write_failure`・`readonly`・`roots::report`・`roots::user_root` は `None` を期待する。`std::any::type_name` の文字列の形は標準ライブラリが約束していない。また、項目を選び違えたとき（出し直しの先や別のモジュールの項目を選んだとき）は、導いた文字列がどの決まりにも当たらない。どちらの場合も `None` になり、検査が空振りしたまま緑になる。
**影響**: 要件 10.8 の「表の文字列とファイルの置き場がずれたら赤」が、status の 6 行にしか効かない。
**提案**: 行ごとに、導いた文字列が期待のモジュールのパスの字面（`"areka::session::alert"` ほか）と等しいことを先に `assert_eq!` で確かめ、その後で種別を比べる。新しいテストは足さず、同じ 1 本の中で行う。
**関わる要件**: 7.4・10.8
**根拠**: design.md「KindCalibration」・OPEN QUESTION 5

🔴 **重大な指摘 2**: 登録の順（4.2）の守りが、確かめていない主張に頼っている
**懸念**: `BodySystems` の 2 つの欄は同じ型 `fn(&mut World)` なので、入れ替えてもコンパイルは通る。設計は、入れ替えると `ghost_session_restart_tests` ほかが登録の前提で赤になる、と書く。しかし `Input` の段の系は互いの並びの指定を持たない（設計の Existing Architecture Analysis）ので、入れ替えで赤になるテストがあるとは限らない。また、`session/` のテストが根の `crate::BODY_SYSTEMS` を引く所（5 か所と、`SwitchRig` を通る 46 ファイル）は、次の `session-crate-split` で crate のテストから bin の定数が見えなくなるときの、いちばん大きい直しになる。
**影響**: 4.2 の「全体として前と同じ」が、欄の名前を読む人の目だけで守られる。次の spec の見積もりにも、この所が数えられないまま残るおそれがある。
**提案**: 実装の段で、`BODY_SYSTEMS` の 2 つの欄を一時的に入れ替えて全体テストを 1 回回し、赤になるテストの名前を `research.md` に残す。0 件なら、設計の Risks の文を「欄の名前と見直しで守る」に直す。テストを足すかどうかは討議で決める。あわせて、8.2 の別表に「`SwitchRig`（`ghost_switch_test_support.rs:237`）とテスト 5 か所が根の `BODY_SYSTEMS` を引く」を、`session-crate-split` が直す項目として明記する。
**関わる要件**: 4.1・4.2・8.2
**根拠**: design.md「RegistrationInjection」の Implementation Notes（Risks）・OPEN QUESTION 1

🔴 **重大な指摘 3**: `main.rs` の最上位に出し直しが無いことを確かめる式が無い
**懸念**: 設計は「`main.rs` の最上位に `use session::…`・`use areka_input…` を置かない」と決めている（Allowed Dependencies）。理由は、最上位の `use` が私有でも crate のどこからも `crate::<名>` で引けてしまい、古い道を黙って通すからである。ところが VerificationRecord の grep の式 2 が見るのは `input_events` の語だけである。実装者が手間を省くために最上位へ `use session::boot_config::ConfigInputs;` などを置くと、呼び手を書き換えないまま `crate::ConfigInputs` が通り、コンパイルの守りが働かない。
**影響**: 要件 2.3 の「出し直しを残さない」が、証跡の無い約束になる。
**提案**: VerificationRecord に grep の式を 1 つ足す。`main.rs` の最上位の `use`・`pub(crate) use` が、今の外の crate の行（`bevy_ecs`・`windows`・`wintf`）だけであることを確かめる。式の例は `git grep -n -E '^(pub\(crate\) )?use ' -- crates/areka/src/main.rs` で、結果を `research.md` に残す。
**関わる要件**: 2.3・5.3
**根拠**: design.md「Allowed Dependencies」・「VerificationRecord」の grep の式 1〜5

## 設計の良い点

- **ソースの字面を読むテストを壊さない書き方**: `fn main` の本文の先頭に置く `use session::{…};` で字面を保つ。`readonly_tests.rs` の絞り込みには、空でないことの `assert!` を足す。新しく置く `roots.rs` は、`roots/mod.rs` を移動として追えるようにする。字面で読むテスト・git の履歴・ログの出所の名前のどれも黙って崩れないよう、1 つずつ手当てが決めてある。突き合わせでも、表に漏れは見つからなかった。
- **振る舞いを変えない証跡が多重**: テストの名前の末尾の多重集合・警告 0 件・grep の式 5 つ・`git diff -M --stat`・触らない所の `git diff --name-only`・実機 2 つを重ねている。同じウェーブの約束（`runtime_tests/mod.rs`・`areka-input`・`areka-placement`・`log-capture-kit`・§8）も、`git diff` の機械的な確かめに落としてある。

## 最終判定

**判定: GO**

**理由**: 要の主張（登録の順・`#[path]` の解決・roots の分け方・`info!` の行・ログの決まりの表・ソースを読むテスト）は、どれも今のコードと合っていた。3 件の指摘は検証の網を締める直しで、形の作り直しは要らない。

**次の手順**:
1. 設計の討議（`kiro-design-discussion`）で、上の 3 件と設計の OPEN QUESTIONS 1〜7 を扱う。
2. 承認の後に、`/kiro-spec-tasks session-module-gather` でタスクを作る。

## 討議への申し送り（対話の代わり）

- OPEN QUESTION 1（`BodySystems` の形）: 推奨の形で要件 4.1 の字面（「`main.rs` の呼び手へ移す」）を満たすとみてよいか。登録の名前は `main.rs` に移るが、呼ぶ位置は `register_systems` の中に残る。次点の 3 つに分ける形をとると、触れない crate の説明約 26 か所が古くなる。推奨の形が妥当と見る。
- OPEN QUESTION 3（`src/roots.rs`）: `structure.md:257` の「`{module}/mod.rs` の形」とは違うが、同じ crate の `readme.rs` ＋ `readme/` が前例になる。履歴を追えることを優先する仮定が妥当と見る。
