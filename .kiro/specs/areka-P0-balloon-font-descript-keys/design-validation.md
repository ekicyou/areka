# 設計検証レポート: areka-P0-balloon-font-descript-keys

> 実施 2026-09-13（`kiro-validate-design`・非対話）。対象ブランチ `claude/areka-p0-balloon-font-keys-b2e272`。
> 本書は design.md の主張を実ソースで突き合わせた結果である。design.md・requirements.md・research.md・spec.json は無改変。

---

## 検証の要約

設計は「転記層に生文字列転記型 2 つ＋完全一致引き 9 本を additive に足す」だけの最小形で、既存の 3 先例（`with_cursor`／`with_windowposition_raw`／`with_vertical_raw`）の写しになっている。判断分岐・既定値・語彙判定は 1 つも転記層に入らず、steering「parser は転記層」と開発者裁定 3 件（`font.name`＝`degraded`・正典追随の仕組みを買わない・ブリーフィング段 ⑵ を同じコミットで書き換える）のいずれとも矛盾しない。設計本文の実測値は下表のとおりほぼ全数が実ソースと一致し、要件 52 基準はすべて設計要素へ写像されている。**判定は GO**。着手前の前提 1 件（ワークツリーの submodule 未取得）と、本文の小さな不正確 3 点を下に挙げる。

---

## 主張の突き合わせ（実ソースで確認）

| # | design.md の主張 | 実測 | 判定 |
|---|---|---|---|
| 1 | `parse.rs` 186 行・`model.rs` 529 行・`mod.rs` 26 行・`parse_tests.rs` 409 行・`model_tests.rs` 596 行 | `wc -l` で 186／529／26／409／596 | 成立 |
| 2 | `map_merged` が引く `font.*` は 5 本（`font.name`・`font.height`・`font.color.{r,g,b}`） | `parse.rs` の `Font::new(...)` と `FontColor::new(...)` の引数行に 5 本のみ。9 キーの `get` は 0 件 | 成立 |
| 3 | `parse.rs` の既設 `ukadoc:` URL は `font.color.r`／`.g`／`.b`・`font.height` の 4 本。`font.name` は説明コメントのみで URL 行が無い | `FontColor::new` の 3 引数行の直前と `get_scalar::<u32>(merged, "font.height")` の直前に 4 本。`font.name` の行の直前は `// font.name は文字列値（数値化しない・R2.5）。` だけ | 成立 |
| 4 | URL コメントの置き場は「キーを引く行の直前」で、`origin`／`wordwrappoint`／`validrect`／`cursor.*` も同じ | `parse.rs` の全 `ukadoc:` 行がその位置（`cursor.pen.color.*` の 3 本には URL 無し＝設計の「すべて」は `cursor.pen` を含まない点で言い過ぎだが本仕様の作業に影響しない） | 概ね成立 |
| 5 | additive ビルダ `with_cursor`／`with_windowposition_raw`／`with_vertical_raw` が既存 | `model.rs` の `impl BalloonModel` に 3 本とも定義あり。`new` は 7 位置引数で `BalloonCursor::default()`／`WindowPositionRaw::default()`／`None` を代入 | 成立 |
| 6 | `WindowPositionRaw` は `Option<String>`×2・`#[non_exhaustive]`・`Default`・`Eq` | `model.rs` の `pub struct WindowPositionRaw { x_raw, limit_raw }`・`#[derive(Clone, Debug, Default, PartialEq, Eq)]` | 成立 |
| 7 | `Font::new` はワークスペース 50 呼出 | `grep -rn "Font::new(" crates` ＝ 50 件（`areka-emo-text/src`・同 `tests`・`areka/src/input_events`・`areka-parsers/src/balloon`） | 成立 |
| 8 | `BalloonModel::new` は 43 呼出 | `grep -rn "BalloonModel::new" crates` ＝ 43 件 | 成立 |
| 9 | `ResolvedFont::resolve` は `model.font()` の `name()`／`height()`／`color()` しか読まない | `areka-emo-text/src/draw.rs` の `pub fn resolve(model: &BalloonModel)` 本体は `font.name()`・`font.height()`・`font.color()` の 3 読みのみ | 成立 |
| 10 | 新 2 型を読む本番コードは 0 か所・名前の衝突無し | `grep -rn "font_decoration_raw\|font_shadow_raw\|FontDecorationRaw\|FontShadowRaw" crates` ＝ 0 件 | 成立 |
| 11 | フィクスチャの `descript.txt` に 9 キーの宣言は 0 件 | `grep -rl` で 0 件 | 成立 |
| 12 | 既存の `distractor_keys_do_not_leak_into_modeled_scalars` が在る | `parse_tests.rs` に定義あり（`fn` 一覧で確認） | 成立 |
| 13 | `Degraded`／`VocabularyOnly` が道具の状態語彙に在る | `ukadoc-survey/src/model.rs` の `enum Status { Implemented, VocabularyOnly, Degraded, Absent, Alias, NotApplicable, Unclassified }`・文字列は `"vocabulary-only"`／`"degraded"` | 成立 |
| 14 | 証拠の要否を見るのは `Status::Implemented` の行だけ | `check/content.rs` の `if entry.status != Status::Implemented { return; }` | 成立 |
| 15 | 証拠行の形（行頭コメント記号・`ukadoc:` の後に空白＋1 語） | `evidence/extract.rs` のモジュール doc と `TOKEN` の扱い | 成立 |
| 16 | `DomainReportStale` は台帳から作り直した本文との一致で判定 | `check/freshness.rs` の `strip_cr(stored) == render_domain(ledger, input.themes)` | 成立 |
| 17 | `report` は `summary.md` を作り直さない | README の一覧で `report`＝ドメイン別 4 本・`report-summary`＝全体。`freshness.rs` も `summary.md` を見ない | 成立 |
| 18 | 束名・優先度は機械が見ない | `report/domain.rs` の束は `links` から作る（備考の「束:」の文は読まない）。`priority` は report／check のどちらにも出ない。`owner` の検査は `check/structure.rs` のドメイン帰属だけで spec 名は見ない | 成立 |
| 19 | 台帳 14 項目が `descript_balloon:font.*`・`implemented` 4／`absent` 10・A11／E66・全 14 項目の備考に「13 キーと書く」の一文 | `ledger/assets.toml` の `[entry."ukadoc:descript_balloon:font.*"]` 14 塊。`grep -c "13 キーと書く"` ＝ 14 | 成立 |
| 20 | 14 本の URL アンカー表はカタログの `url` 欄の写し | `catalog.toml` の 14 行と全アンカー一致（`font.shadowstyle` のみ `versions = ["2.5.27"]`） | 成立 |
| 21 | 「13 キー」の生きた文書は 4 本・`text-decoration-canon` の brief は 7 か所＋「残り 8 キー」1 か所・本仕様 brief 4 か所・`align-shadow` brief `:27`・roadmap `:124` | `grep -rn -E "13 キー|残り 8 キー"` の結果がそのとおり（decoration brief の 19/25/29/33/39/65/89 行に「13 キー」・89 行に「残り 8 キー」） | 成立 |
| 22 | ブリーフィングの是正候補の段 ⑵ は「6 か所」「引き取るのは `text-decoration-canon`」 | `briefing-assets.md` の `**⑵ areka-P0-text-decoration-canon——書体の欄の数が 1 つ足りない**` の段がそのとおり | 成立 |
| 23 | 1,000 行番人は `log-capture-kit/tests/file_length_guard_test.rs` | 実在・`LINE_LIMIT` 1000 | 成立 |
| 24 | DD6「分割の閾値（900 行）」 | steering `structure.md`・`tech.md` に 900 という閾値は無い（あるのは 1,000 の目安のみ） | **不成立**（本文の独自の目安。影響は無い） |
| 25 | `real_repo_data_produces_no_findings`・`a_stale_domain_report_turns_red_and_names_its_domain` が在る | `ukadoc-survey/tests/consistency/checks.rs` に両方定義あり | 成立 |
| 26 | 検査コマンド `cargo run -p ukadoc-survey -- check` が今のブランチで所見 0 | **走らせられない**——`vendors/pasta` submodule 未取得で cargo がワークスペース解決に失敗（下の Issue 1） | 未確認 |

**転記層への語彙・既定値の混入**: 設計要素を全数読んだ限り、`get_scalar` 不使用・`Option<String>` 生文字列・`Default`＝全 `None`・ログ 0 行で、既定値（`ＭＳ ゴシック`／`12`／`0`／`none`／`offset`）と語彙（`none`／`offset`／`outline`）はすべて「下流への引き渡し」表と doc コメントの「判定は下流」の記述にだけ現れる。**混入 0**。

**開発者裁定との整合**: R7.7（`font.name`＝`degraded`）は C5 の変更表で採用。R9.7／9.8（実装側だけを見る・カタログ非参照・`ukadoc-survey` 非接触）は DD5 と Untouched で採用。R1.5（段 ⑵ を同じコミットで）は C6 で採用。**矛盾 0**。

**編集集合**: コード側は `balloon/{parse,model,mod}.rs`＋`parse_tests.rs`／`model_tests.rs`、文書側は R1.3 が列挙する 4 文書＋R1.5 の `briefing-assets.md`＋R7.8 の `ledger/assets.toml`／`report/assets.md`。W13 の他 spec と共有する実ファイルは無い（`balloon/mod.rs` を触る brief は他に無い。`text-decoration-canon` の brief と `roadmap.md` は R1.3 の明示指示）。`brief.md` の列挙（`{parse,model}.rs`＋兄弟テスト＋台帳）に対して `mod.rs` の再輸出 1 行だけがはみ出す（下の Issue 2 ⒜）。

**テストの実効性**: T1〜T12・M1〜M3 は公開入口 `parse()`／`parse_str()` を通し、各写像を 1 本外せば T1／T2／T11 の読み戻しが `None` になって赤になる形。T11 の `assert_eq!(FONT_BASE_KEYS.len(), 14)` 単独は配列型で恒真だが、同テストの「14 本すべてを固有値で読み戻す」判定が本体なので恒真ではない。T10 は接頭辞付き 13 キーだけを書いて基底 14 本が全 `None` を判定するので、接尾一致や接頭辞剥がしの誤実装で赤になる。較正（`font.strike` の引きを一時的に外して T11・T1 が赤になるのを見て戻す）は Testing Strategy に明記されている。

**要件被覆**: 9 要件 52 基準（R1: 5・R2: 7・R3: 4・R4: 5・R5: 4・R6: 5・R7: 9・R8: 4・R9: 9）はトレーサビリティ表に全数あり、コード・台帳・テストのいずれかの要素へ写像されている。散文（手順・不変の宣言）だけで覆われるのは 1.4（食い違い時の照合手順）・5.2（既存写像行の無改変）・5.4（消費側 0 か所）・8.3（着地順の再測定手順）・9.9（出力を切り詰めない）の 5 件で、いずれも「変えない」「手順で確かめる」型の基準ゆえ設計要素を持たないのが正しい。**未被覆 0**。

---

## Critical Issues

### 🔴 Critical Issue 1: このワークツリーでは設計の検証コマンドが 1 本も走らない（submodule 未取得）

- **Concern**: `git submodule status` が `-048d646c… vendors/pasta`（未取得）を示し、`cargo run -p ukadoc-survey -- check` が `failed to load source for dependency pasta_core` でワークスペース解決に落ちる。設計の成功基準（`cargo test -p areka-parsers`・`cargo test -p ukadoc-survey`・`cargo run -p ukadoc-survey -- check/report/evidence`・1,000 行番人）はすべてこの上に乗っている。
- **Impact**: 設計の欠陥ではなく環境の前提だが、実装タスクの検証段が全部「走らせられない」で止まる。過去にも同じ罠が記録されている（`kanade-boot-talkdone-drop` の申し送り）。
- **Suggestion**: タスク生成で最初のタスク（着手前の前提）に `git submodule update --init vendors/pasta` と、`cargo run -p ukadoc-survey -- check` の**着手前ベースライン（所見 0 件の確認）**を置く。ベースラインが赤なら本仕様の責任範囲外の赤を切り分けてから始める。
- **Traceability**: 5.3・6.5・7.9・9.9
- **Evidence**: design.md「Testing Strategy › 道具のテスト・検査」「較正」

### 🔴 Critical Issue 2: 設計本文の小さな不正確 3 点（実装前に本文を直すか、承知の上で進めるかを決める）

- **Concern**: ⒜ 編集集合の列挙に `crates/areka-parsers/src/balloon/mod.rs`（再輸出 1 行）が含まれる一方、R9.8 と brief の約束は `balloon/{parse,model}.rs`＋兄弟テスト＋台帳。`WindowPositionRaw` の先例と同じ流儀で必要な 1 行であり共有ファイルでもないが、R9.8 の「brief の約束どおり」との字面の食い違いは残る。⒝ C6 のブリーフィング段 ⑵ の書き換え文が「説明書 4 本が 13 と書いていた」と言うが、実測では 13 と書くのは brief 3 本（12 か所）で、roadmap は「残り 8 キー」と書くだけ（R1.5 は「件数は是正後に数え直した実測を書き、引き算で導かない」と定める）。⒞ DD6 の「分割の閾値（900 行）」は steering に無い（目安は 1,000 行のみ）。
- **Impact**: ⒜ はレビュアーが「編集集合の逸脱」と読む余地、⒝ は案内文書に不正確な件数を書く恐れ、⒞ は無害。いずれも設計の骨格に影響しない。
- **Suggestion**: ⒜ R9.8 の編集集合に `mod.rs` の再輸出を含める旨を設計の Boundary Commitments に 1 行足す（要件本文は変えない・brief 側は完了時に実測で追随）。⒝ 段 ⑵ の書き換え文は「brief 3 本が 12 か所で 13 と書き、roadmap が 1 か所で残り 8 と書いていた。是正後は 0」のように**実測の件数**で書く。⒞ 「900」を消して「1,000 行に近づいたら」に改める。
- **Traceability**: 1.5・9.8
- **Evidence**: design.md「File Structure Plan」「C6 文書是正」「DD6」

### （3 件目なし）

台帳の担当値 `areka-P0-text-align-shadow-canon` は 4 台帳のどこにもまだ現れない新しい綴りで、道具は spec 名を検査しない。誤字がそのまま残るので、実装時に spec ディレクトリ名（`.kiro/specs/areka-P0-text-align-shadow-canon`）から写すこと——これは注意点であり設計の欠陥ではない。

---

## Design Strengths

1. **「判断しない層」を構造で守っている**。`get_scalar` 不使用・`Option<String>`・`Default`＝全 `None`・ログ 0 行・`Result` 無し、という 5 つの選択がすべて「宣言の事実を潰さない」に向いており、`font.bold,yes` や `font.shadowcolor.r,300` が下流まで届く。既存の `vertical`／`windowposition_raw` と同型なので、後から読む人が新しい規律を覚える必要がない。
2. **零と数を明示し、見張りの恒真を避けている**。Untouched の節で 0 行のファイルを列挙し、T11 を「要素数」でなく「14 本すべての読み戻し」で判定し、較正手順（1 本外して赤を見る）まで書いてある。台帳の機械が見ない項目（束名・備考・優先度）を「最終検証で全数読み直す」と明記し、steering「全項目に○○型はタスク別レビューに映らない」の教訓を設計に取り込んでいる。

---

## Final Assessment

**判定: GO**

**根拠**: 既存アーキテクチャとの整合（additive ビルダ・完全一致引き・生文字列転記）・要件 52 基準の全数被覆・開発者裁定 3 件との無矛盾・転記層への語彙混入 0・テストが実際に赤になる形、の 5 点が実ソースで確認できた。Issue 1 は環境の前提、Issue 2 は本文の字面であり、どちらも設計の作り直しを要しない。

**次の段**: `/kiro-spec-tasks areka-P0-balloon-font-descript-keys`。タスク生成時に Issue 1 の前提タスク（submodule 取得＋`check` の着手前ベースライン）を先頭に置き、Issue 2 ⒜⒝⒞ は設計ディスカッションで採否を決めて design.md に反映する（採る場合は 3 行の修正で済む）。
