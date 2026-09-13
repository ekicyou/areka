# 設計検証レポート: areka-P0-ukadoc-coverage-roadmap

> 検証日: 2026-09-12・対象: `design.md`（2026-09-11 生成・確定版）・`requirements.md`（確定版）・`research.md` §6.1（処分と開発者裁定 2 件）・§10（設計フェーズの調査）・steering（`product.md`・`tech.md`・`structure.md`・`roadmap.md`）。
> 設計の主張のうち安価に確かめられるものは作業ツリーの実物（`crates/ukadoc-survey`・`crates/log-capture-kit`・`doc/ukadoc-coverage/`・`.kiro/specs/`）で裏取りした（末尾「実物での裏取り」）。
> 本レポートは非対話で作成した。設計文書・要件文書・研究文書は変更していない。

## 総評

設計は「3 文書が正本・台帳は導出・テストが突き合わせ」という 1 本の形で要件 3〜11 を貫いており、判定 6 種のそれぞれに「1 か所壊して赤」と「対象 0 件でない下限」が具体的に書かれている。開発者裁定 2 件（帰属の正本は `linkage.md`／テンプレート辞書は静的に読み、URL は実装時に確認し、配布物は入れず実走しない）はいずれも設計判断 D-2・D-7・D-8 と許可依存の節に忠実に写っている。残る問題は 3 つで、いずれも設計の骨格ではなく判定の「主張の範囲」と骨組みの欄の意味の定義に関するものであり、設計ディスカッションで文言を確定すれば実装に進める。

## 重大な問題（最大 3 件）

### 🔴 重大 1: 判定 ⑸ が他 spec の起票・完了に連動して赤になり、W13 の「共有ファイル 0」と完了手続きの門を壊す

- **問題**: 判定 ⑸-a／b は `roadmap-draft.md` の `[briefs].count` と `[[spec]]` の名前集合を `.kiro/specs/` 直下の**生きたディレクトリ一覧**と一致させ、⑸-e は `owner` が `completed/` 配下の spec を指す項目の状態を常時主張する。設計自身が「後続の spec は起票・完了のたびに `roadmap-draft.md` の表を更新する」（Revalidation Triggers 5 番目）と認めているとおり、他 spec が brief を 1 本起こすか `/kiro-complete` で 1 本 `completed/` へ移すだけで `cargo test -p ukadoc-survey`（ワークスペース全体のテスト）が赤になる。W13 は 9 本が並走中で、S 規模の兄弟（`kanade-boot-talkdone-drop` 等）は本 spec より先に完了する見込みが高い。本 spec の枝は rebase しない限りその移動を見ないので、枝の上では 27 で緑のまま squash マージされ、**main 上で初めて赤になる**。逆順でも、兄弟の `/kiro-complete` 手順 7-2（移動後テストゲート）が本 spec の文書を直さなければ通らず、その spec は所有しないファイル `roadmap-draft.md` を編集させられる。⑸-e も同じ性質を持つ——`owner` に 120 件を持つ `property-catalog-lists` が一部を `absent` のまま完了すれば、その完了の門で赤になる。
- **影響**: 要件 Boundary Context「W13 の他 spec と共有するファイルは 0」と roadmap.md「同居は実測で共有ファイル 0 が原則」に反し、`roadmap-draft.md` が事実上すべての後続 spec の共有ファイルになる。判定を緩めずに解消できる形が設計に無いので、実装が「赤は判定を緩めて緑にしない」（要件 11.7）と衝突する。
- **提案**: 主張を「生きた総数との一致」から「表の各行の実在」へ組み替える。⑴ `[[spec]]` の各名前は `spec_dirs ∪ completed_specs` に在る（完了で赤にならない）。⑵ `spec_dirs ⊆ [[spec]]` の名前集合（新しい brief は表に登記しなければ赤。just-in-time 起票の入口が本文書である以上、これは望ましい規律であり、起票する `/kiro-discovery` 再入がこの文書を編集する）。⑶ `[briefs].count` は `[[spec]]` の行数と一致（27）とし、本文の数え方に「着手時の写真」と明記する。⑷ ⑸-e は完了済み spec 宛て 75 件の処分（要件 7.4 ⑵）を、生きた `completed/` の走査ではなく `briefing.md` の骨組みに宛先 spec 名を列挙して主張する形（例: `[[owner_completed]] spec = "…"`、判定は「名前が `completed/` に在る」と「その名前を持つ項目の状態」）に限定する。加えて段 6 に「`/kiro-complete` の前に main へ rebase し `spec_dirs` を数え直す」を明記する（記憶: 並走 brief は陳腐化する・設計前に rebase）。要件 11.1 ⑸ の文言「実数と一致」は、⑴⑵ を合わせれば「表に無い brief が無く、表の brief がすべて実在する」として満たせる。
- **トレーサビリティ**: 要件 1.3・7.4 ⑵⑷・10.2・11.1 ⑸・11.7・12.8・Boundary Context「Adjacent expectations」最終 2 項。
- **根拠**: design.md「Revalidation Triggers」5 番目・「判定の一覧」⑸・D-6・`roadmap.md`「ウェーブ編成」前文と干渉台帳。

### 🔴 重大 2: `override`／`insufficient` の意味が 3 か所で食い違い、判定 ⑷-e が満たせない行が必ず生じる

- **問題**: (a) `briefing.md` の骨組みの注釈は「`override` は第二段で 4 つの根拠の順序から外す行だけに書く」と定める。(b) D-9 は要件 5.3 の配置（「更新」を B の先頭・`system.*` を C の末尾）が `axis_key` の降順と両立しないときも `override` に理由を書くと定める。(c) 判定 ⑷-e は「`override` の本文が『項目 n』か持ち越し行の見出しを含む」ことを要求する。(b) の理由は適合検証項目でも持ち越し行でもないので、(c) を通すには嘘の番号を書くしかない。さらに `insufficient = true` の行は「段階の末尾に置く」（D-8）が、`assets` は退路でも数（0 を含む）なので鍵は存在し、末尾に置けば降順の主張と衝突する。⑷-e は `insufficient` を順序の主張から外すと書いていない。
- **影響**: 判定 ⑷ は要件 6.1・6.7・9.2 の要（「たぶん重要」を機械が止める）であり、これが矛盾したままだと実装は「判定を緩める」か「根拠欄に虚偽を書く」かの二択に落ちる（要件 11.7・6.6 違反）。
- **提案**: `override` を文字列ではなく小さな表にし、理由の種別ごとに受け付ける参照を定義する——`override = { kind = "second-stage", ref = "項目 12" }`／`{ kind = "second-stage", ref = "<持ち越し行の見出し>" }`／`{ kind = "stage-rule", ref = "要件 5.3" }`。判定 ⑷-e は「`override` も `insufficient` も持たない行だけが降順の主張の対象。`insufficient` の行は同じ段階の対象行より後に並ぶ。`override` の `ref` は種別ごとの受け付け形に合う」と書き直す。⑷-g（B の先頭に「更新」・C の末尾に `system.`）はそのまま残る。骨組みの注釈（Logical Data Model）と D-8・D-9 を同じ文言に揃える。
- **トレーサビリティ**: 要件 5.3・6.6・6.7・6.8・9.2・11.1 ⑷・11.3・11.7。
- **根拠**: design.md「Logical Data Model」`briefing.md` の `[[rank]]` 注釈・「判定の一覧」⑷-e・D-8 最終段落・D-9 最終段落。

### 🔴 重大 3: `[tally]` の 4 つの数が 1,552 件を重複なく分割する定義になっていない

- **問題**: 要件 4.2 は「機械の束に現れる id の数・人手で足した id の数・単独項目の数の 3 つ」の合計が対象の全数（1,552）と一致することを求める。設計の `[tally]` は `from_machine`（`members ∖ hand` の総数）・`by_hand`（`hand` の総数）・`singles`（`single = true` の束の数）を置くが、単独項目は「`hand` は `members` と同じと見なす」と定義されている。この定義のまま `by_hand` を全束で合算すると単独項目が `by_hand` と `singles` に二重に数えられ、`target = from_machine + by_hand + singles` は成り立たない。逆に `by_hand` を名前付き束に限るなら、その旨が骨組みの説明にも判定 ⑶-f にも書かれていない。
- **影響**: 判定 ⑶-f は本 spec の中心主張（要件 4.2「ちょうど一方に属する」）を機械で示す唯一の場所であり、欄の意味が曖昧なまま実装すると、緑になった数式がどちらの意味だったかを読み手が復元できない（記憶: 零は明示的に書かなければ要件を満たさない・検査は判定させよ）。
- **提案**: 骨組みに「`from_machine`・`by_hand` は `single = true` でない束だけを合算する。`singles` は単独項目の数（＝単独項目の id 数）。恒等式 `target = from_machine + by_hand + singles` を判定 ⑶-f が主張する」と 1 行で定義し、`singles_by_domain` の合計が `singles` と一致することも同じ判定に入れる。あわせて `alias_excluded`（27）・`not_applicable_excluded`（170）と `target` の和が台帳 4 本の項目数（1,749）と一致することを主張すれば、要件 4.3 の「除いた件数」も数え直しになる。
- **トレーサビリティ**: 要件 4.2・4.3・11.1 ⑶・11.5。
- **根拠**: design.md「Logical Data Model」`linkage.md` の `[tally]` と単独項目の定義（「`hand` は `members` と同じと見なす」）・「判定の一覧」⑶-f。

## 設計の強み

1. **正本を 1 か所に置き、写しは全部数え直す形が徹底している**。人が決めるもの（帰属・段階・順位・4 つの根拠の見出し）だけを 3 文書の骨組みに書き、`priority`・段階ごとの件数・テーマの和集合・跨ぐドメイン・資産の広さ・基盤共有度はすべて `derive` が導いて判定が比べる（D-1・D-2・D-9）。`priority-apply` を副手続きにしたこと（D-4）で、第二段の再実行と「台帳が文書どおりか」の検証が同じ導出関数を通り、使い捨てスクリプトの二重管理が消えている。`replace_priority` の行頭判定は実データで安全（4 台帳とも `priority = ` の行頭行が項目数と一致、字下げされた同綴りは 0 件）で、冪等の事後条件も明文である。
2. **既存の道具の 2 層規律を崩さずに延ばしている**。純粋層 `check/` と `FindingKind` に触れず、判定を `tests/consistency/` の兄弟ファイルに置き（D-3）、囲みの読み手を `examples.rs` から `documents::parse` へ移して重複を 1 つにし、判定 ⑹ は `render_summary` を「判定範囲＋証拠の表」に切って接頭辞一致にする（D-5）。`checks.rs`（893 行）へ足さず新ファイルを 2 つに分けた点も 1,000 行の番人の実在（`file_length_guard_test.rs`）と整合する。

## 最終判定

**GO（条件付き）**——重大 1〜3 はいずれも設計の骨格（正本・導出・判定の 3 層と 12 の設計判断）を変えるものではなく、判定 ⑶-f・⑷-e・⑸ の主張の範囲と骨組みの欄の意味を文言で確定すれば解消する。設計ディスカッションでこの 3 件を裁定して design.md に反映したうえで `/kiro-spec-tasks` へ進むこと。特に重大 1 は W13 並走中の実害（兄弟 spec の完了で main が赤）につながるので、タスク生成前に必ず閉じる。

### 次の段

- 設計ディスカッションで重大 1〜3 の文言を確定し、Logical Data Model・判定の一覧・D-6・D-8・D-9・Revalidation Triggers を同時に直す（記憶: 裁定で要件を改訂したら design・境界節まで追随）。
- そのうえで `/kiro-spec-tasks areka-P0-ukadoc-coverage-roadmap`。

## 軽微な指摘（重大には含めない）

1. **README に「常時の検査に入っていない」の記述が設計の編集範囲の外に 3 か所残る**。「4. 報告の扱い」節の「全体報告は…新しさは常時の検査に入っていない」・「統合担当が作り直したときにだけ更新される」、および「⚠ 全体報告は黙って古くなる」節の「常時の検査には入っていないので何も失敗しない」。判定 ⑹ を入れた後は前 2 つが事実でなくなる。設計の Modified Files は表と直下の段落と「一式」の追記に限っており（要件 12.4）、残る箇所は「触らない理由」か「編集範囲に含める根拠」のどちらかを書くべき。
2. **要件 8.1 ⑶ の「段階ごとの順序付き束一覧（4 つの根拠を含む）」は本文の表になるが、壊れ方とテーマは `[[rank]]` に書かず `linkage.md` から読む設計なので、本文の表に書いた壊れ方・テーマは判定されない写しになる**。本文の表は束名・順位・`assets`・`shared` だけを持ち、壊れ方・テーマは「`linkage.md` の `breakage`・`themes`」と欄名で指す（D-1 の規則）と明記するとよい。
3. **順位の「1 から通し」（要件 7.1）と同順位の共存の数え方が未定義**。⑷-e は「等しい鍵は同じ `rank`・異なれば異なる `rank`」までしか主張しない。密な順位（1,2,2,3）か飛ばす順位（1,2,2,4）かを決めて判定に入れないと、「通し」が検査できない。
4. **D-7 ⑵ の補修 1 本の向きが README の種別定義と逆**。README は `configures` を「設定キー → 挙動・タグ・イベント」と定義するが、設計は property 台帳の `currentghost.seriko.zorder`（プロパティ）から `descript_shell:seriko.zorder`（設定キー）へ `configures` を書く。既存 24 本の流儀に合わせた判断は要件 3.2 どおりだが、流儀自体が定義と逆である事実を `linkage.md`「補修した関連」節に 1 行書いておかないと、次の読み手が同じ疑問を持つ。束は向きを持たないので判定への影響は無い。
5. **持ち越し 8 行の出典パスの綴り**。設計は `verification/m1-completion.md` と書くが実在は `.kiro/specs/completed/areka-P0-emo2-conformance-e2e/verification/m1-completion.md`（§6 に 8 行を確認）。読むだけの文書だが、`briefing.md` に写すときは完全な相対パスで書くこと。
6. **判定 ⑵ の下限「報告から読めた束 id が 100 以上」は妥当**（実測 75＋25＋23＝123、`links` 補修 3 本での合流は高々数件）。ただし D-7 ⑵ の補修で `descript_shell:seriko.zorder` が 48 件の束へ入る結果、全体報告の束の行は増減しうるので、段 1 で数え直した値を非空の下限の注釈に残すこと。
7. **`OWN_SPEC_DIR` と `/kiro-complete` 手順 5-2 の関係は設計の言うとおり安全**（置換は `.kiro/specs/{name}/` → `.kiro/specs/completed/{name}/` のパス形で、ディレクトリ名だけの定数は形が一致しない。仮に書き換わっても直下に無い名前を除くだけで 27 のまま）。`examples.rs` に同種の注釈の前例があるので、その文言に揃えると読み手が迷わない。

## 実物での裏取り（設計の主張のうち確かめたもの）

| 設計の主張 | 確かめ方 | 結果 |
|---|---|---|
| `checks.rs` は 893 行で足せない | `wc -l` | 893 行。`non_vacuity.rs` 442・`examples.rs` 378・`perturb.rs` 242 |
| 1,000 行の番人が実在 | `crates/log-capture-kit/tests/file_length_guard_test.rs` の `LINE_LIMIT` と例外表の定義 | 実在。上限 1000・例外表は逐語で件数を持つ |
| `blocks::split` が塊ごとの `start`／`end` を返す | `ledger/blocks.rs` の `Block` の定義と `split` の署名 | バイト位置で返す。設計の 1 行置換の前提どおり |
| `priority = ` 行は各塊にちょうど 1 行・字下げの同綴りは無い | `grep -c '^priority = '`／`grep -c '^[[:space:]]\+priority = '` | 542／188／342／677（項目数と一致）・字下げは 4 台帳とも 0 |
| `render_summary` の 5 節のうち証拠の表だけがソース木由来 | `report/summary.rs` の `render_summary` 本文 | ⑴〜⑷ はカタログと台帳のみ、⑸「ドメインごとの証拠あり件数」だけが `EvidenceIndex` を使う。冒頭 2 行目の文言は設計の引用どおり |
| `examples.rs` に `toml_blocks`／`quoted_ids` の前例 | 関数定義と較正テスト `the_id_scan_reports_an_id_that_the_catalog_does_not_have` | 実在。`.kiro/specs/completed/…/requirements.md` を読む前例もあり、テストが `.kiro/` を読むこと自体は既存の形 |
| 全体報告の束の表の行は 1 列目が束 id | `report/summary.md` の「束 id｜跨ぐドメイン｜構成 id」の表 | 75 行。ドメイン別報告の「ドメイン内で関連が閉じている束」は「束 id｜構成 id」の 2 列。いずれも 1 列目で束 id、末尾列で構成 id が読める（判定 ⑵・⑶-c の入力に足りる） |
| brief 済み未完了 spec は本 spec を除いて 27 | `.kiro/specs/` 直下で `brief.md` を持つディレクトリを数えた | 28（本 spec 含む）→ 27。`completed/` は 175 |
| 状態の分布 | `grep -c '^status = "…"'` | 88／440／22／1,002／27／170（対象 1,552・要件の値と一致） |
| 「更新」はテーマ 8 つの 1 つ | `model.rs` の `THEMES` と `values.md` の節 | 8 番目に実在（判定 ⑷-g が成立しうる） |
| 適合検証項目表は 20 項目・持ち越しは 8 行 | e2e 完了 spec の design.md「適合検証項目表（D4…）」と `verification/m1-completion.md` §6 | 20 行・8 行を確認 |
| 副手続きは 8 つ固定 | `cli/mod.rs` の `SUBCOMMANDS: [Subcommand; 8]` と `cli_tests.rs` | 9 つ目の追加で表・使い方・テストの「8」を同時に直す必要がある（設計の Modified Files どおり） |
| `configures` の向きの定義 | README「関連の種別は 6 つ」の表 | 「設定キー → 挙動・タグ・イベント」（軽微 4 の根拠） |
