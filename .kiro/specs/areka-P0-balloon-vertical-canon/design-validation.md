# 設計バリデーション報告: areka-P0-balloon-vertical-canon

- 実施日: 2026-08-27
- 実施方式: 非対話（`/kiro-design` からのサブエージェント実行・開発者への質問なし）
- 入力: `spec.json`（language=ja・phase=design-generated）／`requirements.md`（承認済）／`design.md`／`research.md`／`.kiro/steering/`（product・tech・structure・logging・roadmap）
- 検証方法: 設計が主張する file:line をコードで逐一引き直した（末尾の突合表）

---

## 総評

設計は要件討議で確定した 7 つの裁定（拡張キー優先・`\_l` 一括所有・プロパティ非実装・DirectWrite 準拠・切替非実装・クランプ撤去・フィクスチャ正典化）を境界に忠実へ写しており、**単一決定点への集約**（`WritingDirectionDecision`）と **additive ビルダー**の 2 つで、本番波及を `actor.rs:153` の 1 箇所・呼出改変 0 に抑えている。設計が挙げた既存構造の主張は 20 件超をコードで引き直して**すべて一致**した（行数・呼出箇所数・§8 のデータ行 48 件まで逐語で合う）。

ただし **origin クランプ撤去の棚卸しが実出荷資産を 1 件取りこぼしており**、そのままでは実ゴースト emo2 のバルーン文字が画像左上へ抜け、しかも**ワークスペースは緑のまま**になる。撤去そのものは正しいが、棚卸しの網羅方法と境界の定義を直さないと着地できない。

---

## 重大な問題（3 件）

### 🔴 重大 1: クランプ撤去の棚卸しが実出荷ゴーストを取りこぼし、発見方法では原理的に見つからない

**懸念**: 設計 C9 の棚卸し表（7 行）に、宣言 origin が validrect 外である次の 2 箇所が入っていない。

| 場所 | 実測（2026-08-27・本レビューで再測） | 撤去後の帰結 |
|---|---|---|
| `crates/pilot/examples/shiori-host-32/fixtures/emo2/emo2-kakukaku/descript.txt:13-14`（`origin.x,0`／`origin.y,0`） | 面別上書き層 `balloons0s.txt:6-9` が `validrect` を top,46／left,36／right,-44／bottom,-56 へ上書き。`balloonk0s.txt:4-7` は top,40／left,24 | 2 層マージ後の origin (0,0) は validrect の**外**。現在はクランプで開始点 (36,46)（kero は (24,40)）。撤去後は **(0,0)＝バルーン枠の外側左上**へ移り、sakura／kero **両スコープで文字が枠外へ出る** |
| `crates/areka-emo-text/tests/fixtures/emo2-choice/descript-plain.txt:17-18` | `validrect.left,5`／`top,5`。`choice_fixture_test.rs:67` が読む | 現在 (5,5)。撤去後 (0,0)（`descript-cursor.txt` と同型だが表に無い） |

この経路は実在する——`emo2_boot/assets.rs` → `areka-emo-present/src/balloon.rs:499-513 load_scope_balloon_model` が `descript.txt` と `balloons0s.txt` を `parse_str` で 2 層マージし、`BalloonScopeAssets.model` として起動時に確定する。したがって**実機サインオフと emo-present の実描画に効く**。

さらに悪いのは**発見方法**である。設計 C9 Risks は棚卸しを「grep（`クランプ`／`clamp_origin`／`書字開始角`）で網羅する」と定めるが、上記 2 ファイルはいずれもその語を 1 つも含まない（本レビューで `クランプ正準` の repo 全域 grep を実施＝10 ファイル・両者とも非該当）。**設計が定めた網羅手段では、設計が取りこぼした当の 2 件を見つけられない。** 加えて `emo2-kakukaku` の開始点を逐語固定するテストは 1 本も無い（`areka-emo-present`／`emo2_boot`／`emo2_e2e` を走査・0 件）ため、**ワークスペースは全緑のまま静かに壊れる**。

**影響**: 要件 3.10 は撤去を命じているが、設計 Overview の「実行時の挙動変化は『`vertical` の受理』と『宣言 origin の字義解決』の 2 点だけに限局する」という主張は、実出荷ゴーストにおいて**目に見える表示退行**として現れる。要件の Adjacent expectations「下流の適合検証（`emo2-conformance-e2e`・W7）へは非干渉」も、この資産を直さない限り成立しない。本リポジトリが 2 度踏んだ「全緑は十分性の証拠にならない」と、`test-cage-determinism` で踏んだ「対象選定がファイル単位で 1 件取りこぼす」の同型再発である。

**提案**: ⑴ 棚卸しの鍵語を「クランプ」から**意味論**へ変える——`origin.x`／`origin.y` を宣言する descript／`balloon(s|k)*s.txt` を repo 全域（`crates/**`・`examples/**`・`tests/fixtures/**`）で列挙し、**各々の 2 層マージ後の validrect と突き合わせて内外を判定する**（本レビューの実測では宣言は 5 ファイル・うち外に出るのは上記 2 件＋設計が既に挙げた 2 件、`emo2-kakukaku-wplimit` は validrect 全 0＝範囲 [0,0] で境界内につき不変）。⑵ C9 の表へ 2 件を追加し、いずれも DD5 の既定（宣言削除）で開始点不変であることを実値で示す。⑶ 実ゴーストについては「削除で (36,46)／(24,40) が不変」を**逐語固定する檻を新設**する（現在は誰も測っていない＝退行が緑をすり抜ける唯一の穴）。

**追跡性**: 3.9／3.10／10.7／10.8、Adjacent expectations（W7 非干渉）
**根拠**: design.md「C9 クランプ撤去の追随」棚卸し表・同 Implementation Notes（Risks）・Overview「Goals」末尾・「Modified Files」表

---

### 🔴 重大 2: 修正に必要なファイルが宣言された境界の外にあり、10.9 の一般化が未裁定

**懸念**: 重大 1 の是正対象 `crates/pilot/examples/shiori-host-32/fixtures/emo2/**` は、設計の **Modified Files にも「触らないファイル」にも無い**——境界の白地である。一方 Out of Boundary は `crates/areka/src/emo2_boot/**`・`placement/**`・`presenter/**` を「W6.95 同居 3 本とのファイル素を保つ」ため非接触と宣言している。要件 10.9 が名指しするのも `emo2-vertical` フィクスチャ 1 件だけであり、**「同じ欠陥形を持つ出荷資産すべてを正典推奨形へ直す」という一般化は要件でも設計でも裁定されていない**（DD5 は「クランプが効いていた箇所は、いずれも未指定に任せれば同じ位置になる形である」と述べるが、その根拠として挙げるのは `emo2-vertical` と `emo2-choice/descript-cursor` の 2 件のみ）。

**影響**: 実ゴースト資産は `emo2-conformance-e2e`（W7）・実機サインオフ・`emo-present` の複数テストが共有する。境界を明示せずに触ると、⑴ 同ウェーブ／下流 spec との所有の衝突が事後に判明する、⑵ 逆に触らずに着地すると重大 1 の退行を出荷する——どちらも避けたい。あわせて設計 C9 が**要件 10.7 の読みを設計側で書き換えている**点（「退行させない＝被覆を失わないこと。期待値の更新は退行ではない」）も、要件本文には無い読みであり、開発者の裁定として記録されていない。本リポジトリの規律「裁定で要件を改訂したら design・境界節・steering まで追随」に照らすと、読みの確定を討議で明示する必要がある。

**提案**: 設計討議で 2 点を裁定として確定し、design.md へ書き戻す——⑴ **正典推奨形への是正は repo 全域の出荷／テスト資産に及ぶ**（対象を列挙して Modified Files と Boundary Commitments に載せる。`crates/pilot/**` の当該 2 ファイルは「バルーン定義の正典適合」として本仕様が所有し、`emo2_boot` のコードには触れない、という切り方が既存の境界と整合する）。⑵ **10.7 の読み**（被覆を失わないこと／期待値更新は退行ではない）を裁定として明記する。いずれも実装量は増えない——増えるのは所有の明示だけである。

**追跡性**: 10.7／10.9、3.10、Boundary Context（Out of scope）
**根拠**: design.md「Boundary Commitments / Out of Boundary」「File Structure Plan / Modified Files」「触らないファイル（境界の裏面）」「C9 Responsibilities & Constraints（10.7 の読み）」「DD5」

---

### 🟡 重大 3: 本仕様の主成果である 13 行の登記に機械的な関門が 1 つも無く、DD8 の根拠が陳腐化している

**懸念**: 本仕様の要件のうち Requirement 4／5／7／8／11／12（＝要件の過半）は **COMPAT §8 への登記と追跡先への双方向登記**であり、コードを 1 行も伴わない。設計 DD8 はこれに檻を作らないと決め、根拠を「`/kiro-complete` のアーカイブ移動が spec 文書の実ファイル読みを壊す既知の穴（PR#114 で 5 件が main で赤のまま放置）」に置いている。しかし**この穴は 2026-08-22 に skill 側で塞がれている**——`.claude/skills/kiro-complete/SKILL.md` はステップ 5-2 でソース全域 grep、5-3 で仕分け（実ファイル読みは必ず更新）、**7-2 で移動後テストゲート**を必須化し、DoD チェックリスト（:375／:380）にも載せている。DD8 の結論（文書間の登記は文書で検査する）自体は追跡先 4 本が M2 ゲートで brief→requirements へ置き換わることを考えれば妥当だが、**引いた根拠が現在の実物と食い違う**。

**影響**: 根拠が陳腐化していること自体は小さいが、結果として「登記が正しく入ったか」を保証する仕組みが**レビュー時の目視のみ**になる。§8 は `include_str!` 保護もテストも 0 件（設計の実測どおり本レビューでも確認）で、`scope-zorder-pinning` が同ウェーブで同じ表末尾へ追記する。`recompose-budget`／`draw-load-parity` が残した教訓（「未達が spec の内側から見えない」）と同じ形になりやすい。

**提案**: DD8 の**根拠だけを現状へ差し替える**（「アーカイブ移動の穴は 51c5696d で是正済み。それでも檻を作らないのは追跡先 brief が M2 で置き換わり、行の逐語固定が偽の赤を生むためである」）。そのうえで、⑴ §8 の**追記行数と項目名**を tasks.md の完了条件へ逐語で持たせる（13 行・項目名一覧を checklist 化）、⑵ 双方向登記表の 6 行を**着手時と完了時の 2 回**引き直す（設計は既に「着手時に再検証」と書いているが、完了時の再検証は書かれていない——追跡先 brief は同ウェーブ中に動きうる）。

**追跡性**: 4.5／5.5／7.5／8.4／11.1〜11.9／12.3〜12.6
**根拠**: design.md「DD8」「C7 互換台帳の登記 / Implementation Notes」「Data Models 登記台帳（13 行）」

---

## 設計の強み

1. **本番波及をゼロに設計している（DD1／DD3 の組合せ）。** `WritingDirectionDecision` を新設しつつ `WritingMode::resolve` を戻り値型不変の委譲へ縮めることで、本番の唯一の呼出 `actor.rs:153` を非改変に保ち（本レビューで `WritingMode::resolve` の呼出全 16 件を確認・本番は 1 件のみ）、`BalloonModel::new` の 30 呼出箇所も additive ビルダーで無傷にしている。SC14（正典の再改訂）に対する追随点が 2 関数に閉じるという要件の期待を、規約ではなく型で実現している点が特に良い。

2. **「実装しない仕事」を仕様として厳密に定義できている。** 6.4（計測と描画で方向が食い違わない）は `DirectionRecipe::for_mode` の本番呼出が `create_text_format` の内側 1 箇所しか無いことで既に成立しており（本レビューで実測・本番 2 箇所＝定義 :253 と呼出 :329 のみ）、設計はこれを「実装」ではなく「単一性を構造檻で固定する」仕事へ正しく落としている。`draw.rs` 974 行に 1 行も足さない方針（上限まで 26 行）も明示されており、1,000 行の番人と衝突しない。

---

## 最終判定

**NO-GO（条件付き——設計討議で上記 3 点を解消すれば GO）**

**理由**: 設計の骨格・境界・型設計・トレーサビリティはいずれも実装可能な水準にあり、主張した file:line も突合で全一致した。ただし重大 1 は「出荷ゴーストの表示が黙って壊れ、かつ設計が定めた発見手段では見つからない」という確定した欠陥であり、重大 2 はその是正に必要な境界が未定義であるため、このまま tasks へ進むと**タスクに書かれない作業が必ず落ちる**。いずれも設計の作り直しではなく、C9 の棚卸し表・Boundary Commitments・Modified Files・DD5／DD8 の局所改訂で解消できる。

**次の手順**:
1. `/kiro-design-discussion areka-P0-balloon-vertical-canon` で重大 1〜3 を議題化（重大 2 は開発者裁定が要る＝境界拡張と 10.7 の読み）。
2. 裁定を design.md へ書き戻す（C9 棚卸し表へ 2 件追加＋新規の開始点固定檻・Boundary/Modified Files へ `crates/pilot/**` の当該 2 ファイル・DD8 の根拠差し替え）。
3. `/kiro-spec-tasks areka-P0-balloon-vertical-canon`。

---

## 付録: 設計の主張とコードの突合（本レビューでの再測・2026-08-27）

| 設計の主張 | 実測 | 一致 |
|---|---|---|
| `region.rs` 721 行・`clamp_origin_component` :302-331・呼出 :216／:223 の 2 箇所・private | 逐語一致 | ✅ |
| 書字開始角 :212-215（HorizontalTb／VerticalLr＝(left,top)・VerticalRl＝(right,top)） | :212-215 | ✅ |
| 折返し軸の網羅 match :232-239（縦書きは `wordwrappoint.y` のみ） | :232-239 | ✅ |
| 負値＝反対端 `resolve_coord` :284-286・`resolve_or` :290-298 | :284-286／:290-298 | ✅ |
| `writing.rs` 224 行・`resolve` 全域 match :63-77・未知値 warn＋横書き縮退 | :63-77 | ✅ |
| `WritingMode::resolve` の本番呼出は `actor.rs:153` の 1 箇所のみ | 全 16 件中 本番 1 件 | ✅ |
| `.writing_mode()` の読み手は `writing.rs:64` のみ（本番） | 1 件 | ✅ |
| `parse.rs` 161 行・`writing_mode` 転記 :110・ビルダー鎖は `new()` 直後 | :110／:151-152 相当 | ✅ |
| `model.rs` 496 行・`new` 7 引数 :60・`with_cursor` :86・`with_windowposition_raw` :95 | 逐語一致 | ✅ |
| `BalloonModel::new` はワークスペース 30 呼出箇所 | 30 件 | ✅ |
| `draw.rs` 974 行・`for_mode` :253・`create_text_format` :302／apply :329 | 逐語一致 | ✅ |
| `for_mode` の本番呼び手は `create_text_format` の 1 箇所（他 5 件はテスト） | 一致 | ✅ |
| `draw_format_metrics_tests.rs` 469 行・`layout_cursor_tests.rs` 670 行・`vertical_fixture_test.rs` 151 行 | 逐語一致 | ✅ |
| COMPAT §8 は :122 開始・データ行 48 | 48 行 | ✅ |
| `lib.rs` の `PURE_SOURCES` は 9 本（:173-183）・兄弟テストは未列挙 | 一致 | ✅（注） |
| `emo2-vertical/descript.txt:15-16` に `origin.x,0`／`origin.y,0` | 一致 | ✅ |
| `emo2-choice/descript-cursor.txt:17-19` 同型＋クランプ言及コメント | 一致 | ✅ |
| 「クランプ撤去は `\_l` の挙動を 1 ビットも変えない」（4.4） | `\_l` は `layout.rs:453-454` で `region.left()`／`region.top()` を基準に解決＝`region.start()` に非依存。**主張は成立** | ✅ |
| 「実行時の挙動変化は 2 点だけに限局」 | **不成立**（重大 1・実ゴースト emo2 の開始点が (36,46)/(24,40)→(0,0) へ動く） | ❌ |
| C9 棚卸し表が撤去の影響範囲を尽くしている | **不成立**（重大 1・2 件欠落） | ❌ |
| DD8 の根拠「アーカイブ移動が spec 文書読みを壊す既知の穴」 | skill 側で是正済み（ステップ 5-2／5-3／7-2・DoD :375／:380） | ⚠ 陳腐化 |

（注）`PURE_SOURCES` は本番 9 本のみで、`layout_cursor_tests.rs`／`layout_wrap_tests.rs`／`choice_tests.rs`／`state_*_tests.rs` 等の既存兄弟テストは列挙されていない。設計は新規 2 本の追加のみを計画しており、structure.md:181 の規律に対して**既存の穴はそのまま残る**。本仕様の責務外として妥当だが、討議で「既存分は別途」と一言添えると後続が同じ規律を引いたときに迷わない。
