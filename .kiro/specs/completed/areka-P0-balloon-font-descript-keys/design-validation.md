# 設計検証レポート: areka-P0-balloon-font-descript-keys（09-17 改訂版の設計に対する再検証）

> 実施 2026-09-17（`kiro-validate-design`・非対話・`main` 取り込み後の改訂設計 `229ee781` を対象）。対象ブランチ `claude/areka-p0-balloon-font-keys-b2e272`。
> 本書は design.md の主張を**実ソースで突き合わせた結果**である。design.md・requirements.md・research.md・spec.json・ソースはいずれも無改変。`cargo` は走らせていない（submodule 未取得のため）。行番号はすべて本ブランチでの `sed`／`grep` の実測。
> 09-13 版のレポートは本書で上書きした（改訂前の設計に対する検証は git 履歴 `fef6667b` 以前にある）。

---

## 検証の要約

改訂後の設計は「転記層に生文字列転記型 2 つ＋無効表示の束 1 つ＋完全一致引き 23 本を additive に足し、`areka-emo-text` の純粋モジュール 1 つで `\f` と同じ形のトークン列にして着地済みの受け口 `LookLayers::from_balloon` へ渡す」という形で、受け口・転記層・道具・台帳の実測はほぼ全数が実ソースと一致し、要件 62 基準はすべて設計要素へ写像されている。転記層に既定値・語彙判定は 1 つも入らず、開発者裁定（正典追随の仕組みを買わない・`font.name`＝`degraded`）とも矛盾しない。**判定は GO**。ただし DD10 の根拠となる「`Err` の条件は値の綴りだけで決まり層の値に依らない」という主張が `height` の相対指定（`+N`／`-N`）について**不成立**であり、事前検証の結果が受け口の結果と食い違う入力が存在する（下の Critical Issue 1・設計ディスカッションで確定させること）。ほかに本文の不正確 2 点（束名「既設」の誤り・テスト接続の規約逸脱）を挙げる。

---

## 主張の突き合わせ（実ソースで確認）

| # | design.md の主張 | 実測（file:line） | 判定 |
|---|---|---|---|
| 1 | `LookLayers::from_balloon(name_candidates, height, color, background, cursor_text, font_overrides: &[Vec<String>], disable_overrides: &[Vec<String>])` | `crates/areka-emo-text/src/look.rs:180-188` に同じ 7 引数 | 成立 |
| 2 | `apply_overrides` は `Err` を**黙って飛ばす**・doc が記録を本仕様へ申し送り | `look.rs:245` `let _ = apply_font_tag(layer, &base, &args);`・`look.rs:230-232` の doc に「記録はバルーン定義を読む側（`areka-P0-balloon-font-descript-keys`）が…」 | 成立 |
| 3 | `apply_overrides` の `base` は差し込み前の層の複製 | `look.rs:237-241`（`default: layer.clone(), disable: layer.clone()`） | 成立 |
| 4 | `apply_font_tag`・`FontTagIssue`（3 フィールド）・`TextLook::ukadoc_default`・`LookLayers` の 3 フィールド・`Note` はすべて `pub` | `look.rs:528` `pub fn apply_font_tag`／`:384-392` `pub struct FontTagIssue { pub key, pub value, pub reason }`／`:103` `pub fn ukadoc_default`／`:154-160` `pub default`・`pub disable`・`pub cursor_text`／`:358` `pub enum Note` | 成立 |
| 5 | `LookLayers::default()` が在る | `look.rs:248` `impl Default for LookLayers`（`from_balloon(Vec::new(), 12, 黒, 白, 黒, &[], &[])`） | 成立 |
| 6 | `UNOWNED_KEYS = ["align","valign","shadowcolor","shadowstyle"]`・`cursor*`／`anchor*` も所有外・未知キーは `Err(REASON_UNKNOWN_KEY)` | `look.rs:477`・`:484-486` `is_unowned`・`:561-564` | 成立 |
| 7 | `outline` は `Ok(Some(Note::VocabularyOnly))` | `look.rs:591` | 成立 |
| 8 | **`Err` の条件はいずれも値の綴りだけで決まり、層の値に依らない**（DD10・C8 Risks・research §11.4） | `switch`（`:575-576`）・`name`（`:745-746`）・`color`（`:717`→`color.rs:98-113`）・未知キーは綴りのみで成立。**しかし `height` は不成立**——`look.rs:686-697`: `HeightSpec::Relative(delta) => current.height + delta` と `Percent`／`Default`／`Disable` は `current`／`layers` の大きさを読み、`:696-697` で `next <= 0.0` なら `Err(REASON_HEIGHT_NOT_POSITIVE)`。相対指定では判定が層の値に依る | **不成立**（Critical Issue 1） |
| 9 | `parse_color` は 3 成分か 1 語 | `crates/areka-emo-text/src/color.rs:98-113`（`[single]`・`[r,g,b]`・それ以外は `REASON_COMPONENT_COUNT`） | 成立 |
| 10 | `draw.rs` 737 行・`resolve_with_background` が `from_balloon(..., &[], &[])` を呼ぶ・doc に「読めるようになったときはここで…」 | `wc -l` 737・`draw.rs:284-292`（`&[], &[]` は `:290-291`）・`:226-229` の doc（「まだ読めない **8 キー**」と書いており本仕様が書き換える対象） | 成立 |
| 11 | `lib.rs` `PURE_SOURCES` 54 本・`assert_eq!(PURE_SOURCES.len(), 54)`・`every_source_file_is_either_scanned_or_explicitly_excluded`・`draw.rs` は除外側 | `lib.rs:184-346` の `include_str!` を数えて 54・`:394` の assert・`:418` のテスト・`:363` `"draw.rs"` が `SOURCES_OUTSIDE_THE_PURE_SCAN` | 成立 |
| 12 | `Font` は `Eq` 派生・`Default` 無し・非公開 3 フィールド・`#[non_exhaustive]` | `crates/areka-parsers/src/balloon/model.rs:360-366`（`derive(Clone, Debug, PartialEq, Eq)`） | 成立 |
| 13 | `Font::new(name: Option<String>, height: Option<u32>, color: FontColor)`・`FontColor::new(Option<u8>×3)` | `model.rs:370`・`:405` | 成立 |
| 14 | `BalloonModel::new` は 7 位置引数・additive ビルダ 3 本（`with_cursor`／`with_windowposition_raw`／`with_vertical_raw`）が「既存呼び出し側は無改変」を doc で宣言 | `model.rs:71-79`・`:98`／`:107`／`:118` と各 doc | 成立 |
| 15 | `WindowPositionRaw` が生文字列転記型の先例（`Default`・`Eq`・`#[non_exhaustive]`） | `model.rs:234-243` | 成立 |
| 16 | `parse.rs` 186 行・既設の `ukadoc:` URL 18 本・`font.name` に URL 行が無い・`cursor.pen.color.*` に無い | `wc -l` 186・`grep -c "ukadoc:"` 18・`parse.rs:114-115`（説明コメントのみ）・`:150-152`（URL 無し） | 成立 |
| 17 | 2 層マージはキー非依存（`descript.clone()`＋後勝ち `insert`）・`get_scalar` は完全一致引き・ビルダ連鎖の末尾 | `parse.rs:47-53`・`:184-186`・`:175-177` | 成立 |
| 18 | 既存 distractor テスト `distractor_keys_do_not_leak_into_modeled_scalars` が在る | `crates/areka-parsers/src/balloon/parse_tests.rs:162` | 成立 |
| 19 | `mod.rs` 26 行・`pub use model::{...}` の再輸出 | `crates/areka-parsers/src/balloon/mod.rs`（`WindowPositionRaw` を含む 1 文） | 成立 |
| 20 | 台帳 15 項目とも `priority = "A5"`・`absent` 10 の `owner` は本仕様・`implemented` 4 の `owner` は `text-decoration-canon`・`disable.font.*` は `vocabulary-only`／`text-decoration-canon` | `doc/ukadoc-coverage/ledger/assets.toml:1514-1758` の 15 塊を `awk` で抽出して全数一致 | 成立 |
| 21 | 台帳の束名: `implemented` 4 が「台詞の書体・正典どおりに動く」、`disable.font.*` が「台詞の書体・名前だけ受けて使わない」、`absent` 10 が「読む経路が無い」 | `grep -c` で 4／1／10 | 成立 |
| 22 | **`degraded` 用の束名「台詞の書体・読めるが正典どおりに描かれない」は既設**（DD7「状態ごとに**既設の**束名へ揃える」） | `grep -c '読めるが正典どおりに描かれない' ledger/assets.toml` → **0**（`ledger/*.toml` 全体でも 0）。この束名は本仕様が**新設**するものである | **不成立**（Critical Issue 3） |
| 23 | 「13 キーと書くが」の一文は `implemented` 4 項目にだけ残る | `assets.toml:1567`／`1581`／`1595`／`1609`＝`font.color.b`／`.g`／`.r`／`font.height` の塊内。計 4 | 成立 |
| 24 | C4 の 15 アンカーはカタログと逐語一致 | `doc/ukadoc-coverage/catalog.toml:111`（`disable.font.…`）・`:113-126`（`font.*` 14 行）と C4 表を 1 行ずつ照合 | 成立 |
| 25 | 「13 キー」「残り 8 キー」は生きた 3 文書 7 か所（本仕様 brief 4+1・`text-align-shadow-canon` brief 1・`roadmap.md` 1） | `grep -c`: 本仕様 brief「13 キー」4・「残り 8 キー」1／shadow brief「13 キー」1・「残り 8」0／`roadmap.md`「13 キー」0・「残り 8 キー」1（`:127`） | 成立 |
| 26 | `briefing-assets.md` の段 ⑵ は「6 か所」「引き取るのは `text-decoration-canon`」と書く | `doc/ukadoc-coverage/briefing-assets.md:802-806` | 成立 |
| 27 | `text-align-shadow-canon` は brief のみ・`text-decoration-canon` は `completed/` | `ls .kiro/specs/areka-P0-text-align-shadow-canon/` → `brief.md` のみ・`ls .kiro/specs/completed/` に `areka-P0-text-decoration-canon` | 成立 |
| 28 | 完了済み spec の最終検証が `disable.font.*` の引受先を本仕様と登記 | `.kiro/specs/completed/areka-P0-text-decoration-canon/tasks.md:392`・`:402` | 成立 |
| 29 | フィクスチャに 9 キー・`disable.font.*` の宣言は 0 件 | `grep -rlE '^(disable\.)?font\.(bold|…)'` と `disable\.font` とも 0 | 成立 |
| 30 | `areka-emo-text` は `tracing`・`areka-parsers` に依存し `log-capture-kit` は dev-deps 既出 | `crates/areka-emo-text/Cargo.toml:16`・`:27`・`:66` | 成立 |
| 31 | `lib.rs` に `#[cfg(test)] mod balloon_overrides_tests;` を置く（File Structure Plan・C8） | `lib.rs:43-59` は `pub mod` のみで、テスト接続は本番ファイル側の `#[cfg(test)] #[path = "…"] mod …;`（`look.rs:757-767`・`draw.rs:728-735`）。steering `structure.md:146` も「本番ファイル側にはパス属性つきの接続宣言だけを残す」 | **不成立**（Critical Issue 2） |
| 32 | 要件 62 基準（1.1-1.5, 2.1-2.9, 3.1-3.4, 4.1-4.6, 5.1-5.5, 6.1-6.5, 7.1-7.10, 8.1-8.6, 9.1-9.12）が Traceability 表に全数在る | `requirements.md` の番号付き基準 62・design.md の表 62 行を ID 列で照合して欠落 0・重複 0 | 成立 |
| 33 | 1,000 行番人（`LINE_LIMIT`）と着地後見込み: `parse_tests.rs` 409→~760、`model.rs` 529→~700、`model_tests.rs` 596→~690 | 番人 `crates/log-capture-kit/tests/file_length_guard_test.rs:49`。T1〜T16（16 本×20〜25 行≈350）・型 3＋フィールド 3＋ビルダ 3＋アクセサ 3＋手書き `Default`（≈170）・M1〜M4（≈95）はいずれも妥当。最小余裕 ~240 行 | 成立（見込みとして妥当） |
| 34 | 転記層に語彙・既定値を持ち込まない（steering「parse は忠実な転記層」`structure.md:277`） | 飾り 5・影 4・無効表示の同 9 本は `Option<String>` 生文字列、無効表示の `name`／`height`／`color` は基底と同じ `get_scalar`（既存の縮退規則の継承であり新しい語彙ではない）。既定値の代入 0 | 成立 |
| 35 | 正典の増減に自動で気付く仕組みを設けない（開発者裁定 09-11） | DD5 の T11 はカタログを読まない・`crates/ukadoc-survey/**` 非接触・`catalog.toml` 非改変 | 成立 |
| 36 | ⓓ 備考テンプレートの「正典どおり画像色との混色は指定が無いときだけ」 | ukadoc `disable.font.(フォント定義),(指定)` の本文は「disable.font.colorのみバルーンの画像色とミックスした色、ほかはfont.定義群と同じ」であり、「宣言した色をさらに混ぜる」とも「無指定時の既定が混色」とも読める。要件 9.10（承認済み）は後者で確定しているので設計はそれに従ってよいが、台帳の備考に「正典どおり」と書く根拠は正典本文からは一意に引けない | 未検証（要件は確定済み・備考の文言だけ注意） |

---

## Critical Issues（≤3）

🔴 **Critical Issue 1**: DD10 の事前検証は `height` の相対指定で受け口と食い違う
**Concern**: DD10 は「`Err` の条件は値の綴りだけで決まり層の値に依らないので、`LookLayers::default()`（高さ 12）を相手に事前検証しても受け口と同じ結果になる」を根拠に、配線が `warn!` を出す/出さないを決めている。実ソースでは `apply_height` の `Relative(delta)` が `current.height + delta` を計算し（`look.rs:688`）、正でなければ `Err(REASON_HEIGHT_NOT_POSITIVE)`（`:696-697`）。本番の受け口は `layer`＝バルーンの `font.height` を持つ層で判定する（`from_balloon` → `apply_overrides(&mut default, …)`、`:223`）。例: `font.height,20`＋`disable.font.height,-15` → 事前検証 12−15＜0 で **warn が出る**が受け口は 20−15＝5 で**適用する**。逆に `font.height,8`＋`disable.font.height,-10` → 事前検証は 2 で緑・受け口は −2 で**黙って飛ばす**＝要件 8.5「飛ばした事実を `warn!`」に反する。
**Impact**: 「警告が出たのに効いている」「効いていないのに警告が無い」の両方が起こり得る。正典の `font.height` は数値のみで相対指定は `\f` 側の語彙なので実害は語彙外の入力に限られるが、DD10 の不変式（「判定の実体は受け口 1 か所」）が壊れており、タスク別レビューがこの前提を引き継ぐ。
**Suggestion**: 事前検証の相手を `LookLayers::default()` ではなく**実際に差し込む層**にする。`resolve_with_background` は `from_balloon` の 5 引数を既に持っているので、`let probe = LookLayers::from_balloon(candidates.clone(), height, color, background, cursor_text, &[], &[])` を先に組み、`font_overrides(model, &probe.default)`、続いて `disable_overrides(model, &<font_overrides 適用後の default を複製した disable 層>)` の形で相手を渡す（配線の署名に `&TextLook` を 1 つ足すだけ・純粋層のまま・受け口は非接触）。それが重いなら、代替として「相対指定（`+`／`-` 始まり）と `%` の `height` は事前検証の対象外と明記し、W4／W5 の判定から外す」を設計に書いて食い違いを零として認める。どちらを採るかを設計ディスカッションで決めること。
**Traceability**: 8.5・9.11（警告の正確さ）・8.2（`disable.font.height`）
**Evidence**: design.md「設計判断」DD10・「C8 配線」Implementation Notes の Risks・research.md §11.4 の 2 つ目の箇条

🔴 **Critical Issue 2**: テストモジュールの接続先が crate と steering の規約から外れている
**Concern**: File Structure Plan と C8 は `lib.rs` に `#[cfg(test)] mod balloon_overrides_tests;` を置くとしている。`areka-emo-text` の `lib.rs:43-59` は `pub mod` の一覧だけで、兄弟テストはすべて本番ファイル末尾の `#[cfg(test)] #[path = "<stem>_tests.rs"] mod tests;` で接続している（`look.rs:757-767`・`draw.rs:728-735`）。steering `structure.md:146` も「本番ファイル側にはパス属性つきの接続宣言だけを残す」と定める。
**Impact**: 規約逸脱そのものは小さいが、`lib.rs` からの素の `mod balloon_overrides_tests;` は `src/balloon_overrides_tests.rs` を探すので動きはする——動くがゆえに実装者が気付かず、レビューで差し戻される。
**Suggestion**: `balloon_overrides.rs` の末尾に `#[cfg(test)] #[path = "balloon_overrides_tests.rs"] mod tests;` を置く形へ本文を直し、`lib.rs` の変更は `pub mod balloon_overrides;` と `PURE_SOURCES` の 2 本登録（54→56）だけにする。
**Traceability**: 9.8（編集集合）・9.10／9.11（配線テストの置き場）
**Evidence**: design.md「File Structure Plan」`lib.rs` 行・「C8 配線」`lib.rs:` の箇条

🔴 **Critical Issue 3**: `degraded` の束名は「既設」ではなく新設
**Concern**: DD7 は「状態ごとに**既設の**束名へ揃える」とし、`degraded`＝「台詞の書体・読めるが正典どおりに描かれない」を挙げるが、台帳（`ledger/*.toml` 全体）にこの束名は **0 件**。既設は「正典どおりに動く」（4）・「名前だけ受けて使わない」（1）・「読む経路が無い」（10）の 3 つで、`degraded` 用の束名は本仕様が新しく作るものである。
**Impact**: 実装者が既設の名前を探して見つからず止まるか、別の綴りを作って `font.name` と `disable.font.*` の 2 項目で束名が割れる。機械（`ukadoc-survey`）は束名を見ないので赤にならず、最終検証の全数読み直しでしか捕まらない。
**Suggestion**: DD7・C5 の記述を「`degraded` の 2 項目には束名『台詞の書体・読めるが正典どおりに描かれない』を**新設**して揃える（既設 0 件・09-17 実測）」に改め、「同じ状態の項目に別の束名を残さない」の確認手順（`grep -o '束: 台詞の書体[^（]*' | sort | uniq -c` で 3 種＋新設 1 種）を最終検証に書く。
**Traceability**: 7.5・7.7・7.10
**Evidence**: design.md「設計判断」DD7・「C5 網羅台帳の是正」項目別の変更表

---

## Design Strengths

- **受け口の形にそのまま合わせた薄い配線**——`from_balloon` が「項目を列挙しないトークン列」で待っているのを実測で確かめ（`look.rs:180-188`）、配線は「どのキーがどのトークン名になるか」だけを持ち語彙表を複製しない。受け口（完了済み spec の面）・`Font`・`BalloonModel::new`・KV 化層・2 層マージがすべて 0 行で、影 4 キーを「渡さない」と零として判定に固定する（W6）のも正しい。
- **実測の裏取りが厚い**——台帳 15 項目の状態・担当・優先度、カタログの 15 アンカー、3 文書 7 か所の件数、`PURE_SOURCES` の母数 54、フィクスチャ 0 件、`apply_overrides` の黙殺と doc の申し送りまで、本文の数値は上表のとおり 36 件中 33 件が実ソースと一致した。DD5 の「読み戻せることを判定にする」と較正 3 通りは steering「檻は到達する経路を踏ませよ」「報告では実在が判断できない」に沿う。

---

## Final Assessment

**Decision: GO**（条件付き）

**Rationale**: 既存アーキテクチャとの整合（転記層は生文字列・受け口は非接触・純粋層と `windows` 層の分離）と要件 62 基準の写像は成立しており、実装経路は明確で規模も S。不成立 3 件はいずれも設計の骨格を変えず、Issue 1 は配線関数へ引数を 1 つ足す（または対象外を明記する）だけ、Issue 2・3 は本文の訂正で閉じる。

**Next Steps**:
1. 設計ディスカッションで Issue 1 の採り方（実層を相手に事前検証する／相対・百分率の `height` を対象外と明記する）を決め、DD10・C8・research §11.4 の「層の値に依らない」の文を実測に合わせて書き直す。
2. Issue 2（`balloon_overrides.rs` 末尾の `#[path]` 接続）と Issue 3（束名は新設）を design.md に反映する。
3. 台帳 ⓓ の「正典どおり画像色との混色は指定が無いときだけ」は、正典本文が一意に読めないので「本仕様（要件 9.10）はこう解釈した」の形にする（表 #36）。
4. その後 `/kiro-spec-tasks areka-P0-balloon-font-descript-keys` へ。

---

## 着手前の前提（09-13 版から据え置き・再確認済み）

- ワークツリーは `vendors/pasta` submodule が未取得のため `cargo` がワークスペース解決に失敗する。実装の最初のタスクで `git submodule update --init vendors/pasta` を行い、`cargo run -p ukadoc-survey -- check` と `cargo test -p areka-parsers`・`cargo test -p areka-emo-text` の着手前の緑を確かめてから始める（design.md「着手前の前提」に既に書かれている）。
- `areka-emo-text` の新規 `.rs` は `lib.rs` の `PURE_SOURCES` へ登録しないと `every_source_file_is_either_scanned_or_explicitly_excluded`（`lib.rs:418`）が赤になる。
