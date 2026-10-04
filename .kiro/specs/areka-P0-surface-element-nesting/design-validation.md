# 設計レビュー: areka-P0-surface-element-nesting

- 対象: `design.md`（2026-10-04 生成）・`requirements.md`・`research.md`
- 進め方: 対話なし（設計ディスカッションの材料として書く）。設計が既存コードについて述べている点は、実物を読んで確かめた（下の「確かめたこと」）。
- 判定: **GO**（重大な指摘 2 件は設計ディスカッションで決める・直す。構造のやり直しは要らない）

## まとめ

設計は既存の骨組み（`plan.rs` の `flatten_surface` の再帰・`state.rs` の `apply` と `commit_pattern`・`shell_target.rs` の「記録は入口で 1 度」）をそのまま延ばしており、既存コードについての主張は確かめた範囲ですべて実物と合っていた。要件 1.1〜8.5 の全 ID が対応表に載っている。残るのは、設計自身が未解決と書いている要件 7.4 の文面と、「今の絵に出ている部品」を 2 か所で求める規則の書き方の食い違いの 2 点である。

## 重大な指摘（2 件）

### 指摘 1: 要件 7.4 の文面が emo2 の端で字義どおりに成り立たない（開発者の裁定が要る）

- **懸念**: emo2 の `surface.append2110`・`surface.append2210` は自分の `animation0.interval,random,4` を持ち、`\1` のまばたき（`surface.append10,2100` と `surface.append2200` の `animation0`）の `pattern1` として発火の 40ms 後から 120ms 後まで絵に出る（実物の `surfaces.txt` で確認）。設計の規則では、この 80ms の間に 1000ms の境界を跨ぐと 2110／2210 の抽選が 1 回走る。`LotteryBoundary`（`timeline.rs`）は 1000 の倍数の絶対グリッドで、刻みの基本間隔は 50ms（`areka-ghost` の `ticker.rs` の `base_interval`）なので、普段は起きない。起きるのは刻みが 880ms 以上止まった直後（スリープからの復帰など）だけ、という設計の計算は正しい。
- **影響**: 要件 7.4「適合の検体（emo2 を含む）の表示を、本 spec の前と同じ結果に保つ」は、この端で乱数の消費が 1 つ増え、1/4 で 2106／2206 が最長 80ms 重なるので、文面どおりには満たせない。要件 7.1〜7.3 は前提（pattern定義が指すサーフェスがアニメーションを持たない）から emo2 を外しているので、emo2 を守る要件は 7.4 だけである。
- **提案**: 要件 5.13 の裁定（どう参照されても部品として動く）を保ち、7.4 を「刻みが普通に届いているあいだ同じ。刻みが止まった直後の端は 5.13 の帰結」と書き直す案を推す（設計はこの読みで書いてある）。端を塞ぐ規則を足す案は、5.13 に例外を作ることになる。あわせて、`looper_parts_emo2_tests.rs` の「部品の経路を外した場合と一致」の比べる相手は、新しいコードに付けた抜け道ではなく、**実装前の HEAD で採った発行列と乱数の呼び出し回数**を期待値として焼き込む形にする（新しいコードどうしを比べても「前と同じ」の証明にならない）。実物の emo2 の表で刻みを回している既存テスト（`crates/areka-seriko/tests/regression.rs`・`cue_sequence.rs` ほか）に、880ms 以上の飛びを含む刻みの列が無いかを実装の最初に確かめる項目もタスクへ入れる。
- **要件**: 7.4・5.13・5.14
- **設計の該当箇所**: 「emo2 についての事実（要件 7.4）」・「解決したこと・残したこと」の「残した」

### 指摘 2: 「今の絵に出ている部品」の規則の書き方が 3 か所で食い違う（`visible_parts`・`NestTable` の不変条件・`peek`）

- **懸念**: 合成（`flatten_surface`）は、着せ替えの種類の animation のコマを `binds` に在るときだけ合流する（今の `is_bind_animation` の番人）。`NestTable::visible_parts` の規則 ③ も同じ番人を `SurfaceParts.bind_ids` で行う、と書いてある。ところが同じ節の不変条件は「参照を 1 つも持たない番号は載せない」「入れ子も着せ替えの pattern0 も無いシェルでは空」で、Data Models の節は「`children` は全部空」と書いている。pattern0 を持たない `bind+random`（emo2 の `\0` のまばたきがこの形）だけを持つサーフェスが表に載るのかどうかが、文面から決まらない。載らなければ規則 ③ の番人が引けない。また `PartClocks::peek` には「`binds` に無い `BindRandom` のコマを出さない」が書かれていない（`advance` の手順 2 にだけ在る）。
- **影響**: 着せ替えを外した瞬間の `refresh_parts`（`peek` → `commit_pattern`）で、外れた animation のコマが部品の欄に残ると、絵には出ない（合成の番人が落とす）のに `PatternState` が変わり、余分な `Show` が 1 件出て、そのコマの先のサーフェスが「見える部品」に数えられる。要件 5.11（見えない子で描き直さない）と、設計が Revalidation Triggers に挙げた「`visible_parts` と `flatten_surface` がずれると見えない部品で描き直しが起きる」に当たる。到達しにくい経路だが、文面の食い違いは実装者が別々に解釈する種になる。
- **提案**: ⑴ `NestTable` に載せる条件を「`children`・`bind_targets`・`bind_ids` のどれかが空でない番号」と 1 か所で定め、不変条件の文を「入れ子の無いシェルでは `children` が全部空」に統一する。⑵ `peek` にも `advance` と同じ番人（`BindRandom` で `binds` に無いものはコマを出さない）を明記する。⑶ `nesting_visible_tests.rs` の一致の檻に「`binds` から外れた着せ替えの種類のコマ（一番上の欄・部品の欄の両方）」の行を足し、`actor_parts_tests.rs` の着せ替えの変化の檻は「外した側」も踏む。
- **要件**: 5.11・5.12・5.6
- **設計の該当箇所**: 「Nesting（`nesting.rs`）」の Service Interface と Invariants・「Parts（`parts.rs`）」の `peek`・「Data Models」の不変条件

## 確かめたこと（指摘なし）

- **切り替えの `Show` が 1 件で、`state.rs` は無改変のままでよい**: `ScopeStates::apply` は面を `Shown(新しい番号)` にして格納済みの `PatternState` を消し、空の `Show` を返す。直後に公開の `commit_pattern` を呼べば、冪等の番人（今の値と同じなら `Unchanged`）と「今の面・今の着せ替え・新しいコマ」で `Show` を組む処理がそのまま使える。`actor.rs` の `handle_message` が `apply` の `Show` の代わりに `refresh_parts` の `Show` を `emit_display` へ渡せば、発行は 1 件で、部品のコマは切り替えの 1 枚目に載る。次の刻みは同じ値なので `Unchanged` になり、二重には出ない。着せ替えの経路（`commit_bind` が今のコマごと `Show` を返す）と `Hide`（`refresh_parts` が `None`）も同じ組み合わせで閉じる。
- **入れ子の無いシェルの乱数の順と合成の回数**: `on_tick` の抽選は今の順のまま先に全スコープぶん走り、部品の抽選はその後の進行の段でだけ足される。`has_animated_parts` が偽なら部品の経路を通らない。`ComposeKey`（`cache.rs`）は `PatternState` を丸ごと持つので、部品の欄が空なら鍵は今と同じ。
- **当たり判定**: `hit_region` は `RegionPriority::Painter` だけで、列を逆順に見て最初に当たったものを返す（`collision-sort` は見ない）。領域は矩形 4 値なのでずらすだけで済む。「子の分 → 親に直接書いた分」と並べれば要件 4.3〜4.5 の手前奥になる。本番の入口は `EmoPresenter::hit_region_client` の 1 か所（`emo2_boot/hit_region.rs` の `resolve_hit_region` 経由）で、`input_events/` は触らずに済む。
- **読み飛ばしの結論が報告と合成で同じになるか**: 合成が切るのは「先祖に在る番号へ戻る辺」で、先祖は element定義・着せ替えの pattern0・コマの辺でできる。報告の循環は「子から element定義の辺とすべての pattern定義の辺で親へ戻れる」なので、合成が切る element定義の辺は必ず報告に含まれる。領域の持ち込みは element定義の辺だけをたどるので、その経路の先祖は合成の同じ経路の先祖と一致する（要件 4.6）。無い番号は `EmoWorld::surface` の有無で、どちらも同じ表を見る。
- **無い番号の誤報が出ないか**: `build_shell_target_with_boxes` の聞くための面の表は `EmoWorld::build_with_images` で組まれており、ファイル名の慣習だけで建つ面（`apply_base_images` が新設）も「在る」に数えられる（要件 1.8）。`element0` がサーフェスを指す親は、`apply_base_images` の「層 0 が在る」の判定で `surface*.png` を使わない（要件 2.4）。
- **`balloon` の読み分け**: `decode_elements`（parsers）は `overlay` の行だけを採るので、`balloon` の行は `element_kind` に届かない（要件 1.4）。
- **行数**: 実数は `plan.rs` 730・`hit.rs` 714・`actor.rs` 647・`shell_target.rs` 606・`table.rs` 566・`looper.rs` 442 行（設計の「694 行」は空行を除いた数）。どれも増分を足して 1,000 行に届かない。`state.rs`（632）・seriko の `bind.rs` は触らない。
- **同じ波で触らない約束**: parsers の `shell/mod.rs`・`AtlasKey`・`AtlasTable::new`・`areka-emo-text`・`input_events/` に手を入れる箇所は設計に無い。`NormalizedElement` を組み立てているのは `fold.rs`・`base_image.rs` と `normalized.rs` のテストだけなので、欄の追加はクレートの外へ波及しない。
- **`surface.append*`ブレスの画像が焼かれない件**: 本 spec の前からの事実で、roadmap に持ち主（`self-alpha-declaration` の議題 2）が登記済み。新しい起票は要らない。検体（要件 8.2）で `surface.append*`ブレスに画像の element定義を書くと描かれないので、検体では数字だけの element定義だけを書く。

## 良い点

- **継ぎ目が 2 つだけ**: 静的な写し（`NestTable`）と毎回の入力（`PatternState` の部品の欄）だけで compose と seriko をつなぎ、`state.rs`・`cache.rs`・parsers・atlas を無改変に保っている。キャッシュの鍵が自動で部品のコマを含む点も、既存の型の性質をそのまま使っている。
- **自分に不利な事実を実物で確かめて書いている**: emo2 が要件 7.1〜7.3 の前提を満たさないこと、端が起きる条件（880ms）を、推測でなく `surfaces.txt` と境界の計算から示し、未解決として開発者へ渡している。

## 最終判定

- **判定**: GO
- **理由**: 既存の構造との食い違いは無く、全要件に実装の道筋がある。残る 2 件は要件の文面の裁定（指摘 1）と設計の文言の統一（指摘 2）で、どちらも構造を変えない。
- **次の手順**: 設計ディスカッションで指摘 1 を開発者に確かめ、指摘 2 を `design.md` に反映してから `/kiro-spec-tasks areka-P0-surface-element-nesting` へ進む。
