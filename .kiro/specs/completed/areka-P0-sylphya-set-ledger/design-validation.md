# 設計検証レポート: areka-P0-sylphya-set-ledger

> 実施 2026-09-13（`/kiro-validate-design`・非対話・サブエージェント）。入力＝`requirements.md`（要件 10 本・受入基準 52 項）・`design.md`・`research.md` §8〜§9・`brief.md`・steering（`product.md`／`tech.md`／`structure.md`／`roadmap.md`）。
> 設計が引用する file:line・定数の形・テスト名・数はすべて実ファイルを読んで突き合わせた（下記「裏取りの記録」）。`cargo build`／`cargo test` は実行していない（ワークツリーに submodule が無い）。

## 1. レビュー要約

設計は「配列 1 本の末尾追加＋記録用配列 1 本の新設＋件数検査の追随＋文書 2 つ」に閉じており、既存の構造（`classify_set` が読む表と読まない表を**参照の有無**で分ける）と噛み合っている。設計が引用する数（21 の写し 7 箇所・URL 19 本・名簿 5→6・公開 const 8→9・台帳 26 行・担当欄の空き 2・証拠 2→21）は**全件が実ファイルと一致**し、先送りテスト 3 本（t_zpd10／30／40）を赤にする経路も正しく閉じている。残る問題は「要件どうしの矛盾を設計が黙って解いている箇所」と「数の写しの全数確認が設計自身の記述と食い違う箇所」の 2 点で、どちらも実装前に 1 段落で直せる。

## 2. 重大な指摘（最大 3 件）

### 🔴 指摘 1: 要件 2.4「テストが属するファイルを変更しない」と要件 3.6／9.1／9.2 の矛盾を、設計が読み替えの記録なしに解いている

- **問題**: 要件 2.4 は「要件 2.3 のテストが属するファイル（`crates/areka/src/placement/zorder_property_deferral_tests.rs`）を**変更しない**」と定める。一方、要件 3.6／9.1／9.2 は同じファイルの走査対象名簿へ新表を**登記する**ことを求める。両立しない。設計は要件 9.2 と 7.5 については「要件 9.2 との整合」節で読み替え（判定 3 本の本文は不変・名簿とその写しだけを触る）を明記しているが、要件 2.4 については対応表の 1 行（「名簿の 3 箇所＋較正 1 箇所のみ」）で済ませ、**2.4 の文言どおりには満たせないこと**を書いていない。
- **影響**: 完了時の DoD 検証で「2.4 未達」と判定され得る。また後続 spec（`zorder-property`）が同じ要件文を読み、名簿登記を「違反」と誤解する。
- **提案**: 「要件 9.2 との整合」節に 2.4 を加え、「2.4 の『ファイル』は要件 9.2 が定める『先送りを守る判定そのもの 3 本の本文』と読む。ファイル自体は要件 3.6 の指示に従い名簿・その写し・較正行のみを変更する」と 1 段落で明記する。設計ディスカッションで要件 2.4 の文言を 9.2 に揃える改訂（「…のファイルのうち判定 3 本の本文を変更しない」）を裁定に載せる。
- **Traceability**: 要件 2.4・3.6・9.1・9.2
- **Evidence**: design.md「Requirements Traceability」2.4 行・「走査対象名簿」節「要件 9.2 との整合」

### 🔴 指摘 2: 「件数の写し」の全数確認が、設計自身が書く本文と食い違う（要件 4.7 型＝全項目に○○）

- **問題**: 設計は「実装時に `grep -n '21'` で残りが 0 であることを確かめる」と定めるが、設計自身が `dotted.rs` の説明文に「既存 21 ＋ 本機能の 4 ＝ 25」を書き、兄弟テストに `set_effective_first_21_entries_are_unchanged_in_order`／`SET_EFFECTIVE[..21]` を置く。この検査は**設計どおりに実装すると必ず赤**になり、実装者が黙って検査を捨てる誘因になる。同じ型の取りこぼしがもう 1 つある——`zorder_property_deferral_tests.rs` の「語彙表 5 本」は現物 **8 箇所**（module doc L19・`vocabulary_tables` doc L111／113／116／127・`vocabulary_entries_containing` doc L150・t_zpd10 doc L252・t_zpd11 doc L281）にあるが、設計は `vocabulary_tables()` の doc（L111〜127）だけを更新対象に挙げ、残る **4 箇所**（L19・L150・L252・L281）が古いまま残る。
- **影響**: 要件 4.7 の「どれか 1 箇所だけが古いまま残る状態を作らない」に、設計の側で穴が開く。数を印字するだけの記述が古びるのは本プロジェクトで繰り返し赤になった型。
- **提案**: ⑴ 全数確認の探し語を「固定の数」の形に絞る（例: `len(), 21`・`; 21]`・`_21_`・「件数 21」の 4 形で 0 件、かつ「既存 21」は許容と明記）。⑵ 「5 本」「8 本」の全 9 箇所（5 本×8＋8 本×1）を表に列挙し、doc コメントの写しは全部 6／9 へ更新すると決める（t_zpd10／t_zpd11 の `///` は判定の本文ではないので要件 9.2 には触れない旨を添える）。
- **Traceability**: 要件 4.7・9.2
- **Evidence**: design.md「件数の写しの一覧」末尾の grep 指示・「走査対象名簿」節の変更箇所 5

### 指摘 3: なし（重大とみなす第 3 の論点は見つからなかった）

## 3. 設計の強み

1. **「読まない」を構造で判定する**——記録用の表 `SOUND_PROP_NAMES` を `classify_set` から切り離し、その不在を T5（`actor.rs` 本文の参照 0 回・較正 `GENERIC_PROP_NAMES` ≥ 1）で判定させる。要件 3.3／3.7／7.2 の「0 件変化」が印字でなく判定になっており、後続 spec が表を仕分けへ繋ぐときに意図した赤が出る。
2. **赤になる箇所の事前列挙と、赤にできない綴りの明記**——影響を受ける既存テスト（21 固定 3 本・t_zpd11・t_zpd12）と緑のまま残る 7 本を分けて列挙し、t_zpd40 が小文字部分一致で拾う `zorder` を追跡 spec 名を含めて sylphya の全ソースから排除する制約を明文化している。実ファイルで確かめた限り、この列挙に漏れはない。

## 4. 判定

**GO**（条件付き）。

- **根拠**: 既存アーキテクチャとの齟齬なし・52 受入基準すべてが設計節へ辿れる・実装経路が明確・設計が引用する事実は全件が実ファイルと一致。指摘 1・2 はいずれも設計文書の 1 段落〜1 表の追記で解消でき、コードの構造には影響しない。
- **次の手順**: 設計ディスカッションで指摘 1（要件 2.4 の読み替えの登記と文言改訂の裁定）と指摘 2（全数確認の探し語の絞り込み・「5 本」9 箇所の列挙）を design.md に反映してから `/kiro-spec-tasks areka-P0-sylphya-set-ledger` へ進む。

## 5. 非重大の所見（差し戻し不要・実装時に扱う）

1. **兄弟テストのモジュール名**: `structure.md` の規約はファイル名 `<stem>_<モジュール名>.rs`・宣言 `mod <モジュール名>;`。設計の `mod dotted_set_ledger_tests;`（ファイル `dotted_set_ledger_tests.rs`）は stem `dotted` に対してモジュール名が `set_ledger_tests` であるべき（`mod set_ledger_tests;`）。
2. **`include_str!` の走査対象**: `structure.md` は「本番ファイル本文を読む構造テストは兄弟テストファイル `<stem>_*.rs` も列挙する」と定める。T5 は `../actor.rs` だけを読む。主張（`classify_set` が表を読まない）には `actor.rs` で足りるが、規約に従うなら `actor_tests.rs`／`actor_actor_integration_tests.rs`／`actor_actor_criteria_cage.rs` も列挙して「参照 0」を確かめるか、列挙しない理由を T5 の説明文に 1 行書く。
3. **要件 5.3 の形**: 要件文は `/// ukadoc:` を例示するが、設計は配列要素の位置ゆえ `// ukadoc:` を採る。証拠抽出器（`crates/ukadoc-survey/src/evidence/extract.rs`）は `///`・`//!`・`//` を同一規則で読み、先例（`crates/areka/src/placement/config.rs:138`）もある。妥当だが、要件 5.3 の「同じ形」の注記として design の URL 注記節にある説明を完了報告にも 1 行残すこと。
4. **担当欄の実数えの環境依存**: `grep -c '^owner = ""$'` は CRLF の `property.toml` に対し Git Bash の grep では 2 を返すことを確認した（`\r` を無視する）。他の環境（`Select-String` 等）では `$` が `\r` の前で当たらず 0（母数 0 の恒真）になり得るので、完了報告には使ったシェルを明記するか `'^owner = ""\r\?$'` の形にする。
5. **要件 1.3 と 7.2 の境目**: 末尾形 `pause`／`playing`／`position` そのものへの書き込みは、変更前 `StoreWrite`（自由な名前として保存）→変更後 `RuntimeCommand`（受理して捨てる）に変わる。設計 Overview「Impact」がこれを唯一の挙動変化として開示しているので問題ないが、完了報告の「利用者から見える変化」にも同じ 1 文を載せること（要件 10.3 の記録と対をなす）。

## 6. 裏取りの記録（設計の引用と実ファイルの突き合わせ）

| 設計の主張 | 実ファイルでの確認 | 結果 |
|---|---|---|
| `SET_EFFECTIVE` 21 項・`GENERIC_PROP_NAMES` 17・`dotted.rs` 289 行 | `crates/areka-sylphya/src/vocab/dotted.rs`（`wc -l` 289・配列要素を数えた） | 一致 |
| 21 の写しは 7 箇所（dotted.rs 5＋ledger 2） | `grep -n '21'` → dotted.rs L57／188／190／191／209・ledger_key_determinism_tests.rs L210／211 | 一致（要件 4.7 の 6 に doc 1 箇所を加えた設計の数え直しが正しい） |
| `classify_set` 3 段・`is_canonical_vocab` は根 ∨ 葉・語彙表の読み手は `actor.rs` のみ | `actor.rs:136-166`・`grep -rn` で非テスト参照は `actor.rs` のみ（`ukadoc-survey/lib_test_support.rs:301` は同名の見本で無関係） | 一致 |
| `SCANNED_VOCAB_TABLES: [&str; 5]`・`vocabulary_tables()` 5 本・t_zpd11 `len()==5`＋較正 5 行・t_zpd12 は `pub const ` 行を抜き出す・t_zpd40 は小文字部分一致 `zorder` | `zorder_property_deferral_tests.rs` の該当箇所を全文で確認 | 一致（ただし「5 本」の doc 写しは 8 箇所＝指摘 2） |
| `vocab/` の公開 const は 8 本 | `grep '^\s*pub const '` → dotted 5・flat 2・shiori_resource 1 | 一致（新表で 9） |
| 台帳「正典 SET有効」26 行・担当欄の空き 2・`areka-P0-` 担当 ≥180 | `grep -c` → 26／2／186 | 一致 |
| 台帳の 2 行の note が「裁定待ち: …」で終わる・「転記元: doc/COMPAT_ARCHITECTURE.md:207」 | `property.toml` L949-964・L1065-1081 | 一致 |
| COMPAT §8 の `currentghost.seriko.zorder` 行の直後に追加（既存行は L207） | `doc/COMPAT_ARCHITECTURE.md:207`・CRLF | 一致（直後への追加は L207 の参照をずらさない） |
| URL 19 本はカタログ `:623`・`:640-689` の `url` と一致（`name:2`・`path:2`・`meta.*:1`） | `catalog.toml` の 19 行を id で引いた | 全件一致 |
| 証拠抽出器は `///`・`//!`・`//` を同一規則・`ukadoc:`＋空白＋URL 1 語・ページ URL 単独行はスライス突合を起動・走査は兄弟テストも含む | `evidence/extract.rs:35-40`・`evidence/resolve.rs:20-30, 224`・`io/sources.rs:42-46` | 一致（T6 の探し語を行頭に置かない注意は正しい） |
| property の証拠は 2 → 21 | 現行の property 証拠は `baseware.name`／`baseware.version` の 2 件のみで 19 項目と重ならない | 一致 |
| `sound(bgm.mp3)` は解釈不能で StoreWrite | `key.rs:104-121`（`.` で分割してから `(` を読む） | 一致 |
| 追跡 spec の実在 | `.kiro/specs/areka-P0-property-query-channels/brief.md`（追記(88) が本 spec への分割を登記）・`.kiro/specs/areka-P0-zorder-property/brief.md` | 実在 |
| roadmap 仮裁定 1 の改訂（21→25・26 ではない） | `.kiro/steering/roadmap.md:105` | 一致 |
| 受入基準 52 項の対応 | 対応表 52 行（1.1〜1.6・2.1〜2.5・3.1〜3.7・4.1〜4.10・5.1〜5.4・6.1〜6.5・7.1〜7.5・8.1〜8.4・9.1〜9.3・10.1〜10.3） | 全件あり |
