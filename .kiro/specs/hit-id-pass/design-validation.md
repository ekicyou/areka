# 設計の検証: hit-id-pass

- 検証の日: 2026-11-20
- 見たもの: `spec.json`・`requirements.md`・`design.md`・`research.md`（9〜11 節）・`brief.md`・steering（`tech.md`・`roadmap.md` の C23 の行）・関係するコード（読みと検索だけ。ビルド・テスト・アプリの起動はしていない）
- 進め方: 対話なし。問いは立てず、判定をそのまま書く。開発者の裁定（決め 1 ほか）には反論せず、設計が裁定を守っているかだけを見る。

## 検証の要約

設計は、設計の段の試し（WARP と実 GPU）の数字で骨組みを決めており、土台にした事実（署名を変えずに済む・表示の成立点で共有の DC を使って描ける・装置の作り直しの追従の口・CPU の矩形の判定の呼び手）はコードで確かめて全部成り立った。判定は **GO（条件つき）**＝設計の討議へ進めてよい。ただし、承認済みの要件の文と食い違う所のうち 5 つは言い回しの直しではなく決めの変更で、色の絵の α の出どころは裁定の趣旨との間に緊張が残る。この 2 点と `region` の重さの 3 件を開発者が決めるまで、タスクの生成へは進めない。

## 重大な指摘（3 件）

### 指摘 1: 色の絵の α を CPU の α マスクから引く形は、裁定の趣旨との間に緊張が残る

- **懸念**: 決め 1 の字面（ID の絵を GPU に持つ）は守られている。設計が選んだ 3 つを裁定の趣旨（D2D の描画が将来の前提・今安いからと CPU に寄りかからない）に照らした読みは次のとおり。
  - D2D で塗り D2D の 1×1 の写し先で読む（D3D11 は使わない）＝**整合**。決め 1 の添え書きは「候補に入れる」で、設計は比べたうえで退けた（D2D は整数の的へ描けない・読みの差は 0.1 ms ほど）。開発者の言う「1 ピクセルだけ CPU に書き戻す方法」は採った読み戻しそのもの。失うのは「周のすぐそばの画素が装置で分かれない」ことだけで、設計は未決の議題 3 に明記している。
  - 原寸で持ち、点の側を CPU で戻す＝**整合**。点を戻す式は SHIORI へ届けるローカル座標（要件 4.3）のために元から要り、CPU で絵を描くことではない。要件 10.5 は細かさの決めを設計へ預けており、試し（`research.md` 11.2・11.3）は物理ピクセルで塗ると要件 3.1・10.5 と SSP の決め方（画像の画素ごと）を満たせないことを示した。ただし wintf の約束（要件 5）の書き換えを伴う（指摘 2）。
  - 色の絵の α を CPU の `AlphaMask` から引く＝**緊張あり**。当たり判定の答えの半分（当たるか・当たらないか）が CPU の画素に残る。brief の Desired Outcome（22 行）は「当たったかどうかは色の側の α（`wintf-d2d-composite-rules` の読み戻し）で決める」と書き、`draw-methods-canon` の brief（124 行）は開発者の 2026-10-15 の言葉を引いて α を 1 ピクセルの読み戻しへ替える見込みとしている。
- **この検証の読み**: 今は色の合成そのものが CPU で、表示の絵と α マスクは同じ原寸のバイト列から作られる。一次情報源は CPU の画素であり、GPU から読むと拡大と補間を経た写しを読むことになる。原寸の ID と細かさがそろい、クリックの透過と当たりの線が 1 つになる点でも、今の段ではこの形に筋が通る。裁定への違反とは読まない。ただし次の 2 点は直しが要る。
  1. 退けた理由のうち 2 つが弱い。「描画の段が走るまで前の絵の α が残る」は、設計の「Existing Architecture Analysis」の 1 項目め（受け手は指令の適用と描画の後にしか走らない）と食い違う。「World が要るので署名が変わる」は、同じウェーブの約束を理由に形を曲げない決まりに触れる。この 2 つを外しても成り立つ理由（一次情報源・原寸の ID と細かさをそろえる・当たりの線の持ち主を 1 つにする）で書き直す。
  2. 付け替えの持ち主が決まっていない。設計は「`draw-methods-canon` が α マスクの出どころを替えるときに一緒に移る」と Revalidation Triggers に置くが、要件が `draw-methods-canon` へ渡したのはクリックの透過だけで、同 brief は「どちらが受け持つかは両 spec の要件で決める」のままである。
- **影響**: 開発者が「α も GPU から読む」と決めた場合、`hit_region_client` が World を要り、`resolve_hit_region` の署名・`input-crate-split` との取り決め・presenter のテストの足場まで設計が変わる。
- **提案**: 設計の討議で未決の議題 4 を最初に諮る。今の形で通すなら、退けた理由を書き直し、`draw-methods-canon` の brief へ「名前の解決の α（`shown_mask`）も付け替えの対象」を申し送る作業を設計とタスクに入れる。あわせて、将来 α と ID を 1 度の `Map` で読める余地（`research.md` 11.6 の「2 枚を 2×1 の写し先へ」は 1 回ぶんの待ち）を `IdTarget::read` の説明に残す。
- **要件**: 4.1・決め 1・決め 8・Boundary Context の Out of scope（クリックの透過の付け替え）
- **根拠**: `design.md`「色の絵の α の出どころ」「Revalidation Triggers」「未決の議題 4」／`research.md` 9 節・11.9／`brief.md` 22 行・75 行／`.kiro/specs/draw-methods-canon/brief.md` 124 行

### 指摘 2: 要件の文との食い違いのうち 5 つは、言い回しの直しではなく決めの変更である

- **懸念**: 設計係が挙げた 7 つを 1 つずつ見た結果は次のとおり。利用者に見える働きを落とすものは無いが、承認済みの要件（`spec.json` の `requirements.approved` が真）を設計が先に変えている。

| 要件 | 判定 | 中身 |
| --- | --- | --- |
| 5.1 | 決めの変更 | ID の描き先が描画面に付かず、大きさを使い手が渡す単体の入れ物になる |
| 5.6（後半） | 言い回しの追従 | 拡大して置く口が無くなり、後半が空文になる。1 画素対 1 画素で補間しないことは満たす |
| 5.7 | 決めの変更 | 形に拡大と位置を添える口が無い。色の絵は wintf の変換で拡大され、ID は拡大されない |
| 5.8 | 決めの変更 | 座標が「描画面の物理ピクセル」から「描き先の画素」に変わる。同じ呼び出しの中で答え、前のフレームを使い回さないことは満たす |
| 5.13 | 決めの変更 | 作り直すのが wintf でなく使い手になる。presenter は `refresh_after_device_change` で満たす |
| 6.3 | 言い回しの追従 | 拡大率の変化では描き直さなくなる（「〜のときだけ」の条件は満たす） |
| 10.1 | 決めの変更 | 楕円・円の周のそばの画素が SSP と食い違う（規則の当たる所は SSP の当たる所を半画素ずらした形）。決め 5 の承認は「SSP に合わせる」で、要件 1.1・1.2 の「SSP と同じ」は軸の端でしか成り立たない |

- 設計が挙げていない食い違いも在る: 要件 5 の Objective（「拡大ごとの逆変換も書かずに」）・Boundary Context の In scope（「描画面ごとの ID の描き先」）・Adjacent expectations（「ID の描き先を色の絵と同じ拡大で扱うこの仕様の形に乗る」）・要件 4.2 の言い回し・要件 3.8 の「区別できる上限」（設計の上限 65,536 件は区別の限りではなく塗る手間の予算）。
- **影響**: 要件を直さないままタスクへ進むと、タスクの検証と実装の検証が古い文と照らされ、合っている実装が要件に落ちる。10.1 は SSP との食い違いを 1 つ増やす決めで、開発者の裁定が要る。
- **提案**: 未決の議題 1・2 の承認を取ってから、上の全部の文を 1 度に直す（裁定で要件を改めたら設計・境界の節・steering まで追随させる）。10.1 は代案（楕円と円だけ半画素ずらして SSP に合わせる。軸の端の 4 点が周のちょうど上に載り、答えが塗りの丸めに左右される）と並べて諮る。置き場を presenter にした理由のうち「署名が変わる」は外す（指摘 1 と同じ）。表示の成立と同じ呼び出しで描き終えることだけで理由は足りる。
- **要件**: 1.1・1.2・1.5・3.8・4.2・5.1・5.6・5.7・5.8・5.13・6.3・10.1・決め 5
- **根拠**: `design.md`「未決の議題 1・2」「ID の描き先をサーフェスの原寸で持つ理由」「ID の描き先を presenter が持つ理由」「Requirements Traceability」の 5.1・5.7・5.13 の行／`research.md` 11.1・11.11

### 指摘 3: `region` のマスクをサーフェスの切り替えのたびに CPU から GPU へ載せ直す形は、最小の形に寄っている

- **懸念**: マスクの絵（8 ビットの α）は描き直しのたびに作る（1 件あたり約 1.7 ms・100 件で約 180 ms・UI スレッド）。描き直しはサーフェスが変わるたびに起きるので、台詞の途中の表情の切り替えごとにこの手間がかかる。`hit_shapes` が `RegionMask` を `IdMask` へ直すのも描き直しごとである。設計は「`region` を多く使うシェルが出たら足す」として、GPU の絵を覚えておく形を削った。
- **影響**: 要件 6 の Objective（正典が 100 以上を注意する書き方のシェルでも普段どおり触れる）に、点の判定では応えるが、サーフェスの切り替えでは応えない。「あるべき姿で設計する・最小の形にしない」決まりに触れる。マスクの正本を CPU に置いて毎回載せ直す形は、GPU に持つという裁定の趣旨からも遠い。これまで GPU に触らなかった「控えに当たった表示」の道にも、切り替えのたびに塗りが入る。
- **提案**: マスクの GPU の絵と `IdMask` を表示対象の寿命で覚え、装置の変化（`refresh_after_device_change` で `hit_picture` を捨てる所）で一緒に捨てる形を今の設計に入れる。捨てる口はすでに在るので足すのは控え 1 つで済む。未決の議題 5 では、上限 65,536 件の数とあわせて諮る。表示の成立点の所要の記録（`FrameTiming`）に塗りの段を足すかも決める（設計は「足さない」としている）。
- **要件**: 6（Objective）・6.3・3.8・決め 1
- **根拠**: `design.md`「Performance & Scalability」「未決の議題 5」「hit_shapes」／`research.md` 11.3・11.7・11.8（削る）

## 設計の強み

- **決めが試しの数字に支えられている**: 縁のぼかしなしの塗りと SSP の測りの照合・拡大つきの塗りの食い違い・マスクの補間・WARP と実 GPU の画素の差・読み戻しの待ちを、退けた形も含めて測ってから決めている。要件の文と合わなくなった所を隠さず「未決の議題」に集めた点もよい。
- **今ある形にそのまま乗る**: 表示・α マスク・当たりの絵を同じ呼び出しで入れ替える決まり、装置の変化で控えを捨てて出し直す口、`SurfaceComposite::read_pixel` の読み戻しの形（1×1 の写し先・錠・同じ座標の控え）を使い回す。wintf の型は areka と ukadoc の語を持たず、新しい外部の依存も無い。失敗の道の表はどの行にも記録が在る。

## コードで確かめた事実

| 設計の主張 | 結果 | 根拠 |
| --- | --- | --- |
| `hit_region_client`・`resolve_hit_region` の署名を変えずに済む | 成り立つ | `crates/areka-emo-present/src/presenter/hit.rs` 88 行（`&self`・World なし）・`crates/areka-runtime/src/hit_region.rs` 96 行。本番の呼び手は `crates/areka/src/input_events/mod.rs` 184 行だけ |
| 表示の成立点で共有の DC を使って同期に描ける | 成り立つ | `apply_show` は `&mut World` を持ち、同じ呼び出しの中で `record_display`（`crates/areka-emo-present/src/display.rs` 158〜228 行）が共有の DC に `SetTarget`・`BeginDraw`・`EndDraw` を行う前例が在る。`GraphicsCore::device_context()` は `crates/wintf/src/ecs/graphics/core.rs` 241 行 |
| 装置の失いの追従は `refresh_after_device_change` が正しい口 | 成り立つ | `crates/areka-emo-present/src/presenter/refresh.rs` 163 行。`crates/areka-runtime/src/frame.rs` 255 行が毎フレーム呼ぶので `frame*` の直しは要らない。`is_lost_failure` は `pub(crate)`（wintf の中の `IdTarget` から使える） |
| CPU の矩形の判定を消しても、ほかに呼び手が無い | 成り立つ | `hit_region_in`・`hit_region_scaled_in`・`ScaledHit`・`RegionPriority` は `crates/areka-emo-compose/src/` の 6 ファイルと `crates/areka-emo-present/src/` の 3 ファイルにだけ在る。`crates/areka/`・`crates/areka-runtime/`・`crates/areka-placement/`・examples には無い |
| α マスクの閾値は 128・範囲の外は当たらない | 成り立つ | `crates/wintf/src/ecs/widget/bitmap_source/alpha_mask.rs` 7 行・121 行 |
| シェルの切り替えは対象を作り直す（当たりの欄は空から） | 成り立つ | `crates/areka-emo-present/src/presenter/replace.rs` 88〜95 行（外して付け直す） |

成り立たなかった所と抜け:

1. `crates/areka-runtime/src/hit_region.rs` には、矩形の端や「α を参照しない」を書いた説明が無い。「Modified Files」のこの行は直す相手が無いので外せる。外せば、同じファイルの冒頭の説明に触る `input-crate-split` との重なりが消える。
2. `crates/areka-emo-present/src/presenter_film_tests.rs`（393 行・406 行）がテスト専用の `EmoPresenter::hit_region` を使うが、書き直すテストの一覧に無い。`hit_region_in` を消した後のテスト専用の `hit_region` の扱い（消すか・新しい解決へ寄せるか）も設計に書かれていない。
3. `crates/areka-emo-present/src/shell_target.rs` に `ShapesInactive` 専用の出口は無い（読みの崩れをまとめて出す繰り返しが在るだけで、読み手の腕を消せば足りる）。このファイルは今 985 行で、1,000 行の番（`log-capture-kit/tests/file_length_guard_test.rs`）まで 15 行しか無い。上限越えの表に触ってよいのは C23 の約束 ⑸ で `audit-emo2-boot-frame` だけなので、`region` の画像の読みと記録はこのファイルの外へ置く計画が要る。
4. `shown_mask` は、α マスクの輪番（`presenter/budget.rs` の `Arc::get_mut`・下流が手放して単独所有になる前提）に 2 人目の持ち主を足す。`set_shared` と同じ所で入れ替える限り成り立つが、確保が増えないことを確かめる項目が「Testing Strategy」に無い。
5. 「Existing Architecture Analysis」の「受け手は指令の適用と描画の後にしか走らない」と、退けた形の理由「描画の段まで古い絵が残る」が食い違う（指摘 1）。

## 同じウェーブ（C23）との重なり

`.kiro/steering/roadmap.md` の C23 の行の「触るファイル」と突き合わせた。

| ファイル | 相手 | 中身 |
| --- | --- | --- |
| `doc/COMPAT_ARCHITECTURE.md` §8 | `balloon-flip-ssp-review` | 約束 ⑷ は「`hit-id-pass` は末尾への追記」だが、設計は今ある行（まだ当たらない形の行・矩形の端と逆向きの矩形の行）も書き換える（要件 9.2・9.6）。相手が書き換える行とは別の行だが、約束の文を越える |
| `.kiro/steering/tech.md` | `input-crate-split` | roadmap は C23 の「依存を足す spec」の席（`Cargo.lock`・`THIRD-PARTY-NOTICES.md`・`tech.md`）を `input-crate-split` に置く。設計は Testing の節の WARP の例外の列に 1 文を足す。行は別だが同じファイル |
| `crates/areka-runtime/src/hit_region.rs` | `input-crate-split` | `research.md` 9 節の取り決めでは向こうが冒頭の説明の行に触る。上の 1 のとおり、こちらは触らずに済む |

`audit-emo2-boot-frame`（`crates/areka-runtime/src/frame*` ほか）と重なるファイルは無い。設計が触るほかのファイル（`presenter/{show,refresh,target,hub,replace}.rs`・`shell_target.rs`・emo-compose の `world.rs`・`lib.rs`・`normalized.rs`・wintf の `ecs/graphics/mod.rs`・`layout/hit_region/mod.rs`・`areka-emo-atlas`・`areka-parsers` の 4 ファイル）は roadmap の `hit-id-pass` の欄には無いが、相手 3 本の欄にも無い。roadmap の欄に在る `hit_import.rs`・`presenter/budget.rs`・`cache.rs`・`alpha_mask.rs` には設計は触らない。

## 最終の判定

**GO（条件つき）**: 骨組みに今の構造との衝突は無く、土台の事実は成り立ち、全部の要件に受け持ちが在る。残るのは開発者の決めが要る 3 件で、設計はどれも代案つきで討議へ出している。

タスクの生成の前に済ませること:

1. 設計の討議で、未決の議題 4（α の出どころ）・議題 2（細かさと置き場）・議題 1（楕円と円の周のそばの画素）・議題 5（`region` の重さと上限）を諮る。議題 2 か議題 4 が代案に決まれば、presenter の部分と署名の設計をやり直す。
2. 決まった形に合わせて `requirements.md` の文（指摘 2 の表と、設計が挙げていない 5 か所）を 1 度に直す。
3. 「成り立たなかった所と抜け」の 1〜4 を設計に入れる。
4. `draw-methods-canon` の brief への申し送り（名前の解決の α の付け替え）と、C23 の約束 ⑷ と `tech.md` の重なりの知らせを作業に入れる。

確かめていないこと: ビルドとテストは回していない。受け手が走る段の順（指令の適用と描画の後だけか）は設計の記述を読んだだけで、コードでは追っていない。`research.md` 11 節の試しの数字は、試しのコードが木に残っていないので読み直せていない。
