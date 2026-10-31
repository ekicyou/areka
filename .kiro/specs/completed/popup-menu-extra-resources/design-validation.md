# 設計の検証: popup-menu-extra-resources

- 日付: 2026-11-01
- 対象: `design.md`（2026-10-31 生成）・`requirements.md`（要件討議で確定）・`research.md`
- 読んだ steering: `product.md`・`tech.md`・`structure.md`・`roadmap.md`（C15 と「windows-reactor 待ち」の段）
- 突き合わせたコード: `crates/areka/src/menu/captions.rs`・`captions_tests.rs`、`crates/areka-kanade/src/schedule/resources.rs`、`crates/areka-kanade/src/shiori/real.rs`・`actor.rs`、`doc/ukadoc-coverage/`（`ledger/shiori.toml`・`roadmap-draft.md`・`briefing.md`・`linkage.md`・`README.md`）、`crates/ukadoc-survey/`（証拠の取り出し `evidence/extract.rs`・判定の種類 `check/finding.rs`・整合の検査 `tests/consistency/{spec_checks,briefing_arms,linkage_checks}.rs`）

## 設計の検証の要約

設計は、今のメニューの照会の流れ（名前の列を作る `query_ids` → kanade の往復 → 返り値を写す `interpret` → `plan::caption_for`）のうち 2 か所にだけ「新しい名前 → 古い名前」の 1 組を差し込む形で、要件 1〜6 をすべて部品とテストに結び付けている。コードとの突き合わせで、設計の前提（許可表 14 名・`interpret` の 1 件ずつの写しと記録 2 行・2.x への印が同じ許可表から決まること・台帳の数 14／145・`[briefs].count = 66`・束「メニュー」が `updatebuttoncaption` を持つこと）はどれも今のコードと文書に合っていた。残る懸念は実機の見比べ（C5）の手順の 2 点で、設計の骨組みを変えるものではない。

## 重大な懸念（2 件・どちらも実機の見比べの手順）

🔴 **懸念 1: SSP の側で「．さくら」のメニューを開く手順が決まっていない**
**Concern**: C5 は「SSP（`C:\wintools\ssp` に入っている「．さくら」）で同じゴーストのメニューを開いて並べる」とだけ書く。要件討議の前の測り（requirements.md「測った結果」）では、SSP は今「窓際のぱすたさんDEBUG」を動かしており、別のセッション（ぱすた開発）が使っている見込みが高い。「．さくら」を SSP に出すにはゴーストの切り替えか呼び出しが要り、共有の SSP の状態（動いているゴースト・前回のゴーストの記録）を変える。誰に断るか・どう出すか（切り替えか、別のゴーストとして呼ぶか）・済んだ後どう戻すか・画面の操作を排他テストの机で順を取るかが書かれていない。
**Impact**: 実装のタスクの最後で、SSP の持ち主のセッションの作業を止めるか、見比べができずに要件 5.6 が終わらないかのどちらかになる。手順をその場で決めると、外の実物を動かす判断が開発者の確認なしに進むおそれがある。
**Suggestion**: C5 に次を足す。⑴ SSP に触る前に、SSP を使っているセッションへ「急がなくてよい・切れ目で」と断る。⑵ 「．さくら」は切り替えでなく別のゴーストとして呼んで出し、見比べが済んだら閉じる（戻す手間が無い）。切り替えるしかないなら、元のゴーストへ戻すことを手順に書く。⑶ SSP のメニューを開く画面の操作も areka と同じく排他テストの机を取ってから行い、済んだらすぐ返す。⑷ 見比べの前に「．さくら」の `ghost\master\ssp_shiori_log.txt` のバイト位置を控え、差分から SSP が「．さくら」に問うた順も記録に残す（測 1 を 2.x のゴーストでも確かめられる）。⑸ SSP 側が使えないときの代わり（「．さくら」の辞書の `updatebuttoncaption` の言葉と見比べる）を決めておく。
**Traceability**: 要件 5.6・5.8
**Evidence**: design.md「C5 実機の見比べの記録」の Responsibilities & Constraints

🔴 **懸念 2: 往復が 1 回増えた後の所要に、合否の線と止まる条件が無い**
**Concern**: 設計は「照会全体の時間がその 1 往復の分だけ延びる」「『．さくら』（i686 helper 経由）での所要は C5 で記録の時刻から読む」とし、読むだけで終わっている。華和梨は SHIORI/2.x で i686 helper を通るので往復の 1 回が一番重いゴーストであり、しかも完了した `shiori2-protocol` の実機の記録でも右クリックメニューは開いていない（`.kiro/specs/completed/shiori2-protocol/verification.md` の残件表 1 行目）。メニューの照会がこのゴーストで実際にどれだけかかるかは、まだ誰も見ていない。
**Impact**: もし 1,000 ms の上限に近ければ、名前を 1 つ足したことで時間切れが起き、項目名が全部既定名に落ちる。直したいゴーストで今より悪くなる退行で、要件 1.6 は上限を延ばすことを禁じているので、その場で逃げ道が無い。記録を読むだけの手順だと、上限すれすれでも「収まった」と書いて通ってしまう。
**Suggestion**: C5 の手順に合否を書く。⑴ メニューを 3 回以上開き、照会を送ってから写すまでの時間を毎回記録する。⑵ 1 回でも時間切れ（`menu_resource_query_unanswered`）が出たら、既知の差として書かずに実装を止めて報告する。⑶ 上限との余裕（例: 一番遅い回が上限の半分以下か）を記録に書き、余裕が小さいときは開発者に知らせる。これは記録の取り方を決めるだけで、コードは変わらない。
**Traceability**: 要件 1.6・3.1・5.6
**Evidence**: design.md「Performance & Scalability」・「C5 実機の見比べの記録」の 4 つ目の箇条

## 設計の良いところ

- **落とし先を `interpret` の最後で解決し、`CaptionMap` と `plan.rs` を変えない決め**。`CaptionMap` には非空の値しか入らないので「鍵が有る＝引けた」がそのまま成り立ち、空・値なし・失敗の 3 通りが分岐を足さずに同じ扱いになる。記録も今のループが古い名前を他の名前と同じに数えるので、要件 2 の「行の数・水準・種類を増やさない」がコードの追加なしに満たされる。windows-reactor の後に作り直すメニューの組み立て（`plan`・`mod`・`win32`）に乗らない置き場で、開発者の決め（メニューは後で大改造）とも合っている。
- **網羅台帳の検査の仕組みを読んだうえでの置き方**。`LEGACY_CAPTION_RESOURCES` の説明に `ukadoc:` で始まる行を置かない決めは、検査の道具（`evidence/extract.rs`）が `ukadoc:` の後に URL 1 語の行だけを証拠に拾い、ページの URL の行の直後の表は最初の文字列を証拠にする、という決まりに合っている。証拠は許可表の 1 行に限られ、`ImplementedWithoutEvidence` を避けられる。`alias` でなく `implemented` にする決めも、束の検査（`linkage_checks.rs`）が `alias` の項目を束から外すことを求めるので、束「メニュー」の書き直しを生まずに済む。

## 最終の判定

**判定: GO**

**理由**: 今のコードと台帳の数との食い違いは無く、要件 1〜6 の受け入れ基準がすべて部品・テスト・確かめの手順に結び付いている。2 つの懸念はどちらも実機の見比べ（C5）の手順を足せば済むもので、設計の骨組みとコードの変更は変わらない。

**次の手順**:
- 設計の討議（`/kiro-design-discussion popup-menu-extra-resources`）で懸念 1・2 を扱い、C5 の手順に足す文を決める。
- その後 `/kiro-spec-tasks popup-menu-extra-resources` でタスクを作る。C5 の手順（断り・机・合否の線）はタスクの説明にも写す。

## 確かめたこと（参考）

- 許可表 `ALLOWED_RESOURCE_IDS` は今 14 名で、凍結のテストの名前は `allowed_resource_ids_are_exactly_the_fourteen_names`。末尾は `resetballoonpositionbutton.caption`（設計の「末尾に足す」と合う）。
- 2.x への印は `shiori/real.rs` の 2 か所が `is_allowed_resource_id` で決め、送出の関門は `actor.rs` が同じ判定を使う。`shiori2-protocol` の実機の記録で、華和梨は `GET String` の `username` に値を返している（同じ道で資源が引けることは確かめ済み）。
- 台帳の `list_shiori_resource` は実装済み 14・語彙のみ 145 で、`briefing.md` の `[[barrier]]` と合う（設計の 15／144 は 1 件移した後の数）。`[[after]]` は優先度の段の数で、状態を変えても動かない。
- `roadmap-draft.md` の `[briefs].count = 66` は `[[spec]]` の 66 行と合う。束「メニュー」は `updatebuttoncaption` を持ち、段階は A（先例の `areka-P0-popup-menu-minimal` の行と同じ）。
- 同じウェーブの `shell-scale-linkage` と数を分け合う文書（`roadmap-draft.md`・`briefing.md`・報告）は、設計の Risks が「後から着地する側が数え直す・報告は作り直す」と手当てしている。
