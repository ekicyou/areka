# ギャップ分析: audit-wintf-layout-widget

- 実施日: 2026-11-06
- 測った時点: 作業枝 `claude/kiro-start-audit-wintf-layout-widget-19f903` の `dc8a439f`（main `73c5b25d` の上に spec の初期化を 1 つ足しただけ。範囲のファイルは main と同じ。範囲を最後に変えたのは `3f413059`＝`audit-wintf-runtime`）
- 物差し: ponytail 5.3.2 の `/ponytail:ponytail-audit`（`~/.claude/plugins/cache/ponytail/ponytail/5.3.2/skills/ponytail-audit/SKILL.md`。5.3.3 の同じファイルと中身は同じ）。作法のうち「最大 20 件」と「読まなかった所（Not checked）」は外した（要件 2.2・2.3）。範囲のファイルは全部、頭から終わりまで読んだ。
- この文書は分析と案だけを書く。直す・直さないの決めは要件の討議で行う。

---

## 1. 受け持つ範囲（要件 1）

`crates/wintf/src/ecs/` のうち、名前が `layout` か `widget` で始まるもの全部＝`layout/` と `widget/` の 2 つのフォルダ。着手のときより後に足されたファイルは無い（要件 1.2 の該当なし）。`.rs` でないファイルは無い。

| 区分 | ファイル数 | 行数（空行を含む） |
|---|---|---|
| 本番 | 32 | 7,217 |
| テスト（`_tests.rs`・`tests.rs`・`tests/`・`tests_ex/`） | 18 | 4,251 |
| 合計 | 50 | 11,468 |

（`alpha_mask.rs`・`brushes.rs`・`rectangle.rs`・`typewriter_ir.rs` はファイルの中に `mod tests` を持つが、ファイル名の決まりどおり本番に数えた。）

読んだファイルの一覧（行数）。読まなかったファイルは無い。表の中のパスは `crates/wintf/src/ecs/` から見たもの。以下も同じ。

| ファイル | 行 | ファイル | 行 |
|---|---|---|---|
| `layout/arrangement.rs` | 235 | `layout/taffy.rs` | 129 |
| `layout/box_style.rs` | 207 | `widget/bitmap_source/alpha_mask.rs` | 448 |
| `layout/dimension.rs` | 282 | `widget/bitmap_source/alpha_mask_regenerate_tests.rs`（テスト） | 300 |
| `layout/hit_region/mod.rs` | 513 | `widget/bitmap_source/bitmap_source.rs` | 147 |
| `layout/hit_region/tests/color_map.rs`（テスト） | 300 | `widget/bitmap_source/mod.rs` | 21 |
| `layout/hit_region/tests/mod.rs`（テスト） | 10 | `widget/bitmap_source/resource.rs` | 115 |
| `layout/hit_region/tests/polygon.rs`（テスト） | 180 | `widget/bitmap_source/systems.rs` | 389 |
| `layout/hit_region/tests/shapes.rs`（テスト） | 258 | `widget/bitmap_source/task_pool.rs` | 124 |
| `layout/hit_test/hit_test_shared_mask_tests.rs`（テスト） | 199 | `widget/bitmap_source/task_pool_tests.rs`（テスト） | 9 |
| `layout/hit_test/mod.rs` | 632 | `widget/bitmap_source/tests.rs`（テスト） | 227 |
| `layout/hit_test/tests.rs`（テスト） | 333 | `widget/bitmap_source/wic_core.rs` | 85 |
| `layout/hit_test/tests_ex/alpha_mask.rs`（テスト） | 79 | `widget/brushes.rs` | 402 |
| `layout/hit_test/tests_ex/alpha_mask_resource.rs`（テスト） | 300 | `widget/mod.rs` | 16 |
| `layout/hit_test/tests_ex/entity_ex.rs`（テスト） | 208 | `widget/shapes/mod.rs` | 2 |
| `layout/hit_test/tests_ex/mod.rs`（テスト） | 22 | `widget/shapes/rectangle.rs` | 362 |
| `layout/hit_test/tests_ex/opacity.rs`（テスト） | 200 | `widget/text/draw_labels.rs` | 232 |
| `layout/hit_test/tests_ex/tree_ex.rs`（テスト） | 193 | `widget/text/label.rs` | 118 |
| `layout/metrics.rs` | 40 | `widget/text/mod.rs` | 17 |
| `layout/mod.rs` | 135 | `widget/text/typewriter/mod.rs` | 338 |
| `layout/rect.rs` | 208 | `widget/text/typewriter/tests.rs`（テスト） | 263 |
| `layout/systems/arrangement_systems.rs` | 87 | `widget/text/typewriter_draw.rs` | 377 |
| `layout/systems/mod.rs` | 16 | `widget/text/typewriter_ir.rs` | 247 |
| `layout/systems/monitor_systems.rs` | 576 | `widget/text/typewriter_layout.rs` | 241 |
| `layout/systems/monitor_systems_tests.rs`（テスト） | 567 | | |
| `layout/systems/monitor_systems_transition_tests.rs`（テスト） | 603 | | |
| `layout/systems/taffy_systems.rs` | 305 | | |
| `layout/systems/window_pos_systems.rs` | 171 | | |

1,000 行に近いファイルは無い（最大は `layout/hit_test/mod.rs` の 632 行）。

---

## 2. audit の結果（要件 2）

### このコードがしていること

wintf の「置き場所」と「部品」である。`layout/` は、taffy（flexbox の配置の計算機）と ECS の木を結び（`BoxStyle` → `TaffyStyle` → taffy の木 → `Arrangement`）、親から子へ位置と倍率を積んで画面の上の矩形（`GlobalArrangement`）を出し、窓の位置（`WindowPos`）と行き来させる。モニタの一覧（`Monitor`）と仮想デスクトップの根（`LayoutRoot`）を持ち、表示の構成が変わったら一覧を直して窓の DPI を引き直す。当たり判定（`hit_test`）は、画面の点から最前面の部品を探す（矩形・α のマスク・名前付きの領域）。`widget/` は、描く部品（四角 `Rectangle`・文字 `Label`・打ち出し表示 `Typewriter`・画像 `BitmapSource`）と色（`Brushes`）を持ち、それぞれ D2D の命令の列（`GraphicsCommandList`）を作る。

想定した負荷: デスクトップマスコット 1 プロセス、UI スレッド 1 本、窓は数枚、部品は数十、画面更新は 60〜120 Hz、マウスの移動ごとに当たり判定。areka の本番が使うのは、`LayoutRoot`・モニタ・配置の伝播・窓の位置の行き来・`hit_test_in_window`・`AlphaMask`／`AlphaMaskResource`・`Brushes`（`areka-emo-present`・`areka-emo-text` が使う）。`Typewriter`・`Rectangle`・`Label`・`BitmapSource`・名前付きの領域・taffy の flexbox は、areka の本番からは使われていない（使うのは `crates/wintf/examples/`・`crates/areka/examples/` だけ。`git grep` で確かめた）。wintf は crates.io に出すライブラリなので、areka の外の使い手が公開の型を普通に使う場合も「起こりうる」と数えた。

### 数

| 組 | 件数 | うち「直さずに残してよい 3 種類」の候補 | うち範囲の外へ直しが及ぶもの |
|---|---|---|---|
| 必ず直す | 8 | 0 | 0（2 番は直し方次第で `ecs/world/mod.rs`＝避ける） |
| 直したほうがよい | 17 | 1（16 番＝⑶） | 6（13・14・22・23・24・25 番＝`crates/wintf/tests/`・`examples/`。23 番の serde は `Cargo.lock`） |
| できれば | 12 | 2（33 番＝`#[deprecated]` の扱い・35 番＝⑴） | 3（29・33・35 番＝`crates/wintf/tests/`） |
| 合計 | 37 | 3 | 9 |

「直さずに残してよい 3 種類」は ⑴ 開発者の裁定に反する直し ⑵ わざと置いた先送りの印（実在の理由の doc 付きの `#[allow(dead_code)]`・限界が今の負荷を越えていない `shortcut:`・`ponytail:`） ⑶ SSP との互換や ukadoc の正典に関わる振る舞いの変更。範囲の中に `#[allow(dead_code)]`・`shortcut:`・`ponytail:`・`TODO` は 0（`grep` で確かめた）。⑵ に近い印は `rectangle.rs:19` の `#[deprecated]` だけ（23 番）。

各件の書き方: 番号・場所・何が悪いか・こうなると困る（具体の場面）・いちばん小さな直し方の案・範囲の外（木全体を探した結果）・残してよい種類。

### `audit-wintf-runtime` が直し済みの件（数え直さない・要件 2.5）

次の 5 件は今のコードで直っていることを確かめ、指摘に入れていない。

- 5: `dimension.rs:161`・`:207` の `Percent` を 100 で割る（直っている）。
- 94: `box_style.rs` の `FlexContainer`・`FlexItem`（無い）。
- 101: `metrics.rs` の `Opacity`（無い）。
- 146: `arrangement.rs:190-194` の `From<Arrangement> for GlobalArrangement` は `GlobalArrangement::default() * arrangement`（直っている）。
- 144: `taffy.rs:82-89` の `remove_node`（木から先に消す）と `taffy_systems.rs:296-305` の `cleanup_removed_entities_system`（`error!`）（直っている）。9 番は同じファイルの別の行（`sync_taffy_tree_system` と `compute_taffy_layout_system`）だけを指す。

### `audit-wintf-graphics-world` が直した 3 ファイル（要件 2.7）

`widget/bitmap_source/systems.rs`・`widget/shapes/rectangle.rs`・`layout/systems/window_pos_systems.rs` は、立ち直り（命令の列が外されたら描き直す `Without<GraphicsCommandList>`・装置の文脈を 1 回だけ取る）を入れた後の今の形で読んだ。10 番はその `Without<GraphicsCommandList>` が、部品ごとの恒久の失敗でも毎回やり直す形になっている件（立ち直りの作りそのものは否定しない）。

### `Arrangement` の on_add フック（要件 2.6）の判断

**指摘にする（13 番・直したほうがよい）。** 理由: ⒜ 本番にその使い方は無く、今の振る舞いへの害は 0 だが、⒝ 範囲の中のテスト 2 本が、この上書きを避けるためにわざと形を曲げている（`layout/hit_test/tests_ex/entity_ex.rs:19-20` は `Arrangement` を入れずに `GlobalArrangement` だけを置く・`layout/hit_test/tests_ex/opacity.rs:32-37` は一度 `flush` してから入れ直す）。範囲の外の `crates/wintf/tests/drag/window_dragging_filter_test.rs:113`・`:194` も上書きを前提に説明を書いている。つまり「束で両方を入れると渡した値が黙って消える」罠に、木の中で 3 度引っかかっている。⒞ 直しは 1 行（同じ層の `LayoutRoot` のフック `layout/mod.rs:123-135` がすでに「無ければ入れる」形）で、本番の振る舞いは変わらない。

---

### 必ず直す（8 件）

**1. 木に入った後で `BoxStyle` を付けた部品は、taffy の木につながらず、ずっと配置されない**
- 場所: `layout/systems/taffy_systems.rs:57`・`:61`・`:83-103`（`sync_taffy_tree_system`）
- 何が悪いか: taffy の親子をつなぐのは「`ChildOf` が変わった（`Changed<ChildOf>`）」ときだけ。その時点で子の taffy の節がまだ無ければ（`BoxStyle`→`TaffyStyle` がまだ無い）、`filter_map` で飛ばされる。後の回に `BoxStyle` を付けると節は作られる（`:68-72`）が、`ChildOf` はもう変わらないので親につながらない。親の側に後から `BoxStyle` を付けた場合も同じ（子の節は親の無い根のまま）。`BoxStyle` を外したときは `TaffyStyle` が残り、古い寸法のまま配置される（`build_taffy_styles_system` `:25-51` は外れたことを見ない）。
- こうなると困る: `spawn((部品, ChildOf(親)))` の後で `insert(BoxStyle{..})` すると、その部品の配置（`TaffyComputedLayout`）は既定の 0 のまま、`Arrangement` の大きさも 0 で、画面に出ない。エラーも出ない。モニタの節を `initialize_layout_root`（`monitor_systems.rs:107-114`・`:152-159`）と `apply_monitor_snapshot`（`:392-397`）がわざわざ先に作っているのは、この穴を避けるためと読める。
- 直し方の案: 新しく節を作った部品について、その親（`ChildOf`）と自分（`Children` を持つなら自分を親として）を「影響を受けた親」に足す（`:83-86` に 2 行）。`BoxStyle` が外れたら `TaffyStyle` も外す（`RemovedComponents<BoxStyle>`）。元に戻すと落ちるテスト 2 本（子に後から付ける・親に後から付ける）。
- 範囲の外: なし（係の中だけ。登録の順は変えない）。
- 残してよい種類: 当たらない。

**2. 子を消しても、残った兄弟の配置を計算し直さない**
- 場所: `layout/systems/taffy_systems.rs:128-130`（`compute_taffy_layout_system` の `has_changes`）
- 何が悪いか: 計算し直すきっかけは `TaffyStyle`・`BoxStyle`・`ChildOf` の「変わった」だけで、「外れた・消えた」を見ない。子を `despawn` すると、`sync_taffy_tree_system`（`:106-112`）が taffy の親から外し、`cleanup_removed_entities_system` が節を消すが、計算はその回も次の回も走らない（掃除の係は計算の後に並ぶ＝`ecs/world/mod.rs:246-253`）。
- こうなると困る: 横に 3 つ並べた flex の真ん中を消すと、3 つ目は元の位置に残ったまま。次に何かの寸法が変わるまで詰まらない。
- 直し方の案: 計算の係が `RemovedComponents<ChildOf>`・`RemovedComponents<TaffyStyle>` も見て、空でなければ計算する（係の引数を 2 つ足すだけ）。元に戻すと落ちるテスト 1 本（真ん中を消すと 3 つ目が詰まる）。**掃除の係を計算より前へ並べ替える直し方は `ecs/world/mod.rs` に触るので、このウェーブでは取らない**（要件 5.9）。
- 範囲の外: なし（上の直し方なら）。
- 残してよい種類: 当たらない。

**3. 打ち出し表示（`Typewriter`）のトークを入れ替えても、前のトークの文と時間表が使われる**
- 場所: `widget/text/typewriter_layout.rs:34-39`（`init_typewriter_layout` は `Without<TypewriterLayoutCache>` だけを見る）・`:20-28`（作り直しのきっかけは `Changed<Arrangement>` だけ）・`widget/text/typewriter/mod.rs:270-272`（`TypewriterTalk` が外れたときのフックはログだけ）
- 何が悪いか: 描画の控え（`TypewriterLayoutCache`＝文の配置と時間表）は、一度作ると大きさが変わるまで作り直さない。新しい `TypewriterTalk` を入れても控えは古いまま。トークを外しても、控えと最後の命令の列（`GraphicsCommandList`）が残る。型の説明（`typewriter/mod.rs:101-106`・`:278-281`）は「トーク追加時に生成」「完了・クリア時に remove」と書くが、そうする所が無い。
- こうなると困る: 「こんにちは」を打ち終えた後で `insert(TypewriterTalk::new([Text("さようなら")], t))` すると、画面には「こんにちは」が出続け、新しいトークの進み具合で古い文が打ち出される。トークを外して消したつもりでも最後の絵が残る。
- 直し方の案: `TypewriterTalk` の付け替え（`Changed<TypewriterTalk>` のうち `tokens` が変わったとき＝入れ直し。`on_insert`／`on_remove` のフックでもよい）で控えを外し、外れたら命令の列も空にする（`Label` の `on_label_remove`・`label.rs:76-82` と同じ形）。元に戻すと落ちるテスト 1 本（入れ替えで控えの文が新しくなる）。
- 範囲の外: `crates/areka/examples/mock-shell.rs`・`crates/wintf/examples/typewriter_demo.rs` は 1 度入れるだけの使い方で、形は変わらない（要件 5.11・5.12 に当たらない）。
- 残してよい種類: 当たらない。

**4. 最後の文字の後ろに置いた「待ち」と「知らせ」が捨てられる**
- 場所: `widget/text/typewriter/mod.rs:221-264`（`TypewriterTalk::update`）
- 何が悪いか: 全部の文字を出した時点で `Completed` にする（`:262-264`）。その時点で後ろにまだ `Wait`・`FireEvent` が残っていても、次の回からは `Playing` でないので早く戻り（`:213-215`）、二度と進まない。
- こうなると困る: 台本 `[Text("おしまい"), Wait(1.0), FireEvent{target, Complete}]` は、最後の文字が出た瞬間に完了になり、1 秒の待ちも、完了の知らせも起きない。知らせを待つ側は永久に待つ。今のテスト（`typewriter/tests.rs:241-263`）は「文字 0・知らせだけ」の退化の形しか見ておらず、この穴を踏まない。
- 直し方の案: 完了の判定を「時間表の項目を最後まで進めた（`next_item_index == items.len()`）」に変える（1 行）。元に戻すと落ちるテスト 1 本（最後の文字の後の待ちと知らせ）。
- 範囲の外: なし。
- 残してよい種類: 当たらない。

**5. 打ち出し表示の「知らせ」の宛先が消えていると、UI スレッドが落ちる**
- 場所: `widget/text/typewriter_draw.rs:52-56`（`commands.entity(target).insert(...)`）
- 何が悪いか: bevy_ecs 0.19 では、命令（`Commands`）の `insert` が消えた部品に当たると失敗になり、既定の重さは `Panic`（`bevy_ecs-0.19.1/src/error/bevy_error.rs:20`「重さの無い失敗は Panic」・`error/handler.rs:117-121` の既定 `match_severity`）。wintf は既定の扱いを変えていない（`FallbackErrorHandler` を設定する所は木に 0）。
- こうなると困る: 台本が知らせの宛先にしたバルーンの部品を、トークの途中で閉じて消すと、知らせの時刻に UI スレッドが panic で落ちる。
- 直し方の案: `try_insert` にする（消えていたら黙って捨てる・bevy の説明書の推す形）か、`get_entity` で確かめて `debug!` を残す。元に戻すと落ちるテスト 1 本（宛先を消してから進める）。
- 範囲の外: なし。
- 残してよい種類: 当たらない。

**6. 打ち出し表示が文字を 3 通りに数え、絵文字や 𠮷 で隠すはずの文字が見える・待ちの位置がずれる**
- 場所: ⒜ `widget/text/typewriter_draw.rs:122-127`（見せる長さ＝クラスタの長さの和＝UTF-16 の数）と `:222`（全体の長さ＝`chars().count()`＝Unicode の文字数）を `:224-248` で比べ、`DWRITE_TEXT_RANGE`（UTF-16 の位置）に渡す ⒝ `widget/text/typewriter_layout.rs:206-216`（トークンごとに `chars().count()` だけ時間表の項目を作るが、項目はクラスタ番号）
- 何が悪いか: DirectWrite の位置とクラスタの長さは UTF-16 の単位。サロゲートの対（𠮷・多くの絵文字）は 1 文字で 2 単位、異体字セレクタや結合文字は 2 文字で 1 クラスタ。3 つの数え方を混ぜている。
- こうなると困る: ⒜「𠮷野家です」（UTF-16 で 6・文字数 5）を 1 文字だけ出した時点で、隠す範囲は位置 2 から長さ 3＝位置 5 の「す」が隠れず、最初から見えている。⒝「❤️」（2 文字・1 クラスタ）を含む `[Text("❤️ok"), Wait(1.0), Text("!")]` は、最初のトークンが 4 項目を作って「!」のクラスタまで食べ、「!」が 1 秒の待ちの前に出る。
- 直し方の案: 全体の長さは `encode_utf16().count()`（または配置の文字数）に、時間表はトークンごとの UTF-16 の範囲とクラスタの長さの和で進める（クラスタの長さを足して、トークンの UTF-16 の終わりに届くまでの項目を作る）。元に戻すと落ちるテスト 2 本（サロゲートの対・異体字セレクタ）。⒝ の時間表は DirectWrite 非依存の関数に切り出せばテストは純 CPU で書ける（`get_cluster_metrics` の結果だけを渡す）。
- 範囲の外: なし。
- 残してよい種類: 当たらない（`Typewriter` は wintf 独自の部品で、伺かの `\w`・`\_w` の正典は areka-sakura の側）。

**7. `BitmapSource` を外す・付け直すと、古い画像と古い当たりのマスクが残る**
- 場所: `widget/bitmap_source/systems.rs:144`（D2D の画像を作り直すのは「無効なとき」だけ）・`:337`（α のマスクを作るのは `Added<BitmapSourceResource>` のときだけ）・`widget/bitmap_source/bitmap_source.rs:137-147`（外すときは命令の列を空にするだけで、`BitmapSourceResource`・`BitmapSourceGraphics`・`HitTest::alpha_mask()` を残す）
- 何が悪いか: 付け直しで読み込みが走ると、新しい `BitmapSourceResource` が上書きで入る（`bitmap_source.rs:116-121`）。上書きは `Added` にならないのでマスクは作られず、`BitmapSourceGraphics` は古い D2D の画像を「有効」と持ち続けるので、描き直しで古い画像を描く。外したときは、見えない部品が古いマスクのまま当たり続ける。
- こうなると困る: ⒜ `remove::<BitmapSource>()` の後 `insert(BitmapSource::new("b.png"))` すると、画面には a.png が出続け、当たりはマスク無し（矩形全体）になる。⒝ `BitmapSource` を外しただけの部品は、絵が消えたのに a.png の形でクリックを吸う。
- 直し方の案: 描画で `resource.is_changed()` のときも D2D の画像を作り直す。マスクの係を `Changed<BitmapSourceResource>` にし、まだマスクの無いものだけ作る（今の `:351-353` の確かめはそのまま効く）。外すフックで `BitmapSourceResource` と、フックが入れた `HitTest` を外す（利用者が入れた `HitTest` を外さないよう、入れた印を持つか、`HitTest` はそのまま残して `BitmapSourceResource` だけ外す＝マスクが無ければ矩形になる。どちらにするかは設計）。元に戻すと落ちるテスト 2 本（付け直し・外し）。GPU を使うテストは実 GPU で回す（要件 5.6）。
- 範囲の外: `crates/wintf/tests/widget/bitmap_source_integration_test.rs`（範囲の外の wintf のテスト・持ち主の `audit-wintf-runtime` は完了）に足すか、範囲の中の `bitmap_source/tests.rs` に足す。
- 残してよい種類: 当たらない。

**8. 空のトークの背景は、一度描いたら大きさや色が変わっても描き直さない**
- 場所: `widget/text/typewriter_draw.rs:293-298`（`draw_typewriter_backgrounds` の問い合わせが `Without<GraphicsCommandList>`）
- 何が悪いか: 命令の列が 1 度入ると、二度とこの係の対象にならない。`Arrangement`・`Brushes` が変わっても描き直さない。
- こうなると困る: 背景色つきの空の `Typewriter` の窓を広げると、背景は最初の大きさの矩形のまま（はみ出た所は透明）。色を変えても変わらない。
- 直し方の案: 問い合わせを `Or<(Changed<Arrangement>, Changed<Brushes>, Changed<TypewriterTalk>, Without<GraphicsCommandList>)>` にする。12 番・21 番と一緒に直すのが自然。元に戻すと落ちるテスト 1 本（大きさを変えると命令の列が入れ替わる）。
- 範囲の外: なし。
- 残してよい種類: 当たらない。

---

### 直したほうがよい（17 件）

**9. 失敗しても記録が残らない所が多い**（プロジェクトの決まり「記録の無い失敗の道を作らない」・要件 5.3）
- 場所:
  - `layout/systems/taffy_systems.rs:70`（`create_node`）・`:77`（`set_style`）・`:98-100`（`set_children`）・`:109`（`remove_child`）・`:153`（`compute_layout` の失敗を `is_ok()` で黙って飛ばす）・`:156`（`layout()` の失敗）
  - `widget/bitmap_source/systems.rs:208`（`EndDraw`。装置が失われたときの失敗はここに出る）
  - `widget/text/draw_labels.rs:100-113`（向きの設定 8 つ）・`:136`（`SetWordWrapping`）・`:142`（`GetMetrics`）
  - `widget/text/typewriter_layout.rs:119-144`（向きと揃えの設定 16 個）
  - `widget/text/typewriter_draw.rs:132`（`GetMetrics`）・`:162`（背景のブラシ）・`:229-241`（透明のブラシが作れないとき黙って全文を描く）・`:250`・`:264`（`SetDrawingEffect`）・`:333`・`:347`・`:362`・`:367`（背景の係は失敗を全部黙って飛ばす）
- 何が悪いか: `let _ =` や `Err(_) => continue` で失敗を捨てている。
- こうなると困る: 例えば縦書きの向きの組み合わせが DirectWrite に断られても、横書きのまま描かれて何も残らない（19 番）。`GetMetrics` が失敗すると原点が 0 になり、文字の位置がずれても追えない。
- 直し方の案: 失敗を `warn!`／`error!` で残す（ECS の係は `Err` を返す先が無いので記録だけにし、理由を 1 行書く・`audit-wintf-graphics-world` の 10 番と同じ扱い）。21 番で描く骨組みを 1 つにまとめれば、そこで一度に揃う。
- 指摘にしなかった所: `bitmap_source.rs:122`・`systems.rs:386` の `let _ = tx.send(cmd)`（受け手の `WintfTaskPool` が World ごと消えたときだけ失敗＝終了の途中で無害）、`rectangle.rs:233`・`draw_labels.rs:183`・`typewriter_draw.rs:183` の `EndDraw`（直前の失敗を `warn!` で残した後の後始末）。
- 範囲の外: なし。
- 残してよい種類: 当たらない。

**10. 部品ごとに必ず失敗する描画を、毎回やり直して毎回警告を出す**
- 場所: `widget/bitmap_source/systems.rs:113-117`・`:144-157`、`widget/shapes/rectangle.rs:138-143`・`:176-186`、`widget/text/draw_labels.rs:33-37`・`:76-94`
- 何が悪いか: 描画の係は「命令の列が無い（`Without<GraphicsCommandList>`）」も対象にする（`audit-wintf-graphics-world` が立ち直りのために入れた）。失敗すると命令の列を入れずに `continue` するので、次の回もまた対象になる。装置の不調のような一時の失敗ならよいが、部品そのものが原因の失敗も同じく毎回やり直す。
- こうなると困る: D2D の最大の大きさ（多くの装置で 16,384）を越える PNG を `BitmapSource` にすると、`CreateBitmapFromWicBitmap` が毎回断られ、毎秒 60 回の `warn!` と、毎回の画像の解読（11 番）が UI スレッドで続く。
- 直し方の案: 失敗した部品には空の命令の列（`GraphicsCommandList::empty()`）を入れて、次の変化（`Changed<…>`）か立ち直り（命令の列が外される）までやり直さない。立ち直りの作りはそのまま効く。元に戻すと落ちるテスト 1 本（失敗した部品が 2 回目の係で対象にならない）。
- 範囲の外: なし（`ecs/graphics/` の立ち直りの係は変えない）。
- 残してよい種類: 当たらない（`audit-wintf-graphics-world` の要件は「外されたら描き直す」で、恒久の失敗をやり直すことは求めていない。設計で突き合わせる）。

**11. 画像の「裏での読み込み」が、実は UI スレッドで 2 回解読している**
- 場所: `widget/bitmap_source/bitmap_source.rs:98-133`（裏の作業）・`systems.rs:355-375`（α のマスクのために UI スレッドで `copy_pixels`）・`systems.rs:145`（描画で `CreateBitmapFromWicBitmap`）・説明 `systems.rs:355`
- 何が悪いか: 裏の作業が返すのは、`crate::com::wic::load_bitmap_source`（`com/wic.rs:30-71`）の「形式の変換器」で、まだ解読していない（WIC は読むときに初めて解読する）。だから PNG の解読は、UI スレッドのマスク作り（`copy_pixels`）と描画（`CreateBitmapFromWicBitmap`）で 1 回ずつ起きる。`:355` の説明「IWICBitmapSource は Send でないため同期で取る」は、`BitmapSourceResource` に手で `Send`／`Sync` を付けている（`resource.rs:31-32`）ことと食い違う。
- こうなると困る: 大きな画像を出すと、そのたびに UI スレッドが 2 度止まる（測ってはいない）。「非同期で応答性を保つ」（`mod.rs:3-4`）が成り立っていない。
- 直し方の案: 裏の作業で `CreateBitmapFromSource(…, WICBitmapCacheOnLoad)` を 1 回呼んで中身を持たせる（範囲の中の `bitmap_source.rs` で済む）。α のマスクも裏で作り、命令で戻す（今の `SetAlphaMaskCommand` を使い回す）。説明を直す。
- 範囲の外: `com/wic.rs` は変えなくて済む（範囲の中で包む）。
- 残してよい種類: 当たらない。

**12. 打ち出し表示は、変化が無くても毎回すべてを描き直す**
- 場所: `widget/text/typewriter_draw.rs:71-80`（`draw_typewriters` の問い合わせに変化の絞りが無い）・`:36-49`（`update_typewriters` が毎回 `&mut TypewriterTalk` を触るので、何も進まなくても `Changed` が立つ）
- 何が悪いか: 打ち終えて止まっている表示も、毎回新しい命令の列を作って入れ直す。入れ直すと描画面の描き直し（`Changed<GraphicsCommandList>`）まで毎回走る。
- こうなると困る: `Typewriter` を 1 つ置くだけで、毎秒 60 回、文の描画と描画面の更新が続く。
- 直し方の案: `update` で見える数・状態が変わったときだけ書く（変わらなければ `bypass_change_detection`）。描画の係を `Or<(Changed<TypewriterTalk>, Changed<TypewriterLayoutCache>, Changed<Arrangement>, Changed<Brushes>, Without<GraphicsCommandList>)>` に絞る。元に戻すと落ちるテスト 1 本（止まった表示で 2 回目の係が命令の列を入れ替えない）。
- 範囲の外: なし。
- 残してよい種類: 当たらない。

**13. `Arrangement` と `GlobalArrangement` を一緒に入れると、渡した `GlobalArrangement` が黙って既定値に変わる**（要件 2.6 の判断＝指摘にする）
- 場所: `layout/arrangement.rs:39-47`（`on_arrangement_add`）
- 何が悪いか: フックが無条件に `GlobalArrangement::default()` を入れ直す。命令は生成の束の後に効くので、束で渡した値を上書きする。
- こうなると困る: 本番には無い使い方だが、範囲の中のテストが 2 度この罠を避けている（`hit_test/tests_ex/entity_ex.rs:19-20`・`tests_ex/opacity.rs:32-37`）。範囲の外の `crates/wintf/tests/drag/window_dragging_filter_test.rs:113`・`:194` も上書きを前提に書いている。
- 直し方の案: `insert_if_new`（無ければ入れる）にする。`LayoutRoot` のフック（`layout/mod.rs:128-134`）と同じ考え方。テスト 1 本（束で渡した値が残る）。上の 2 本の回り道は素直な形に戻せる。
- 範囲の外: `crates/wintf/tests/drag/window_dragging_filter_test.rs`（説明だけ・どちらも既定値を渡しているので答えは変わらない）・`crates/wintf/tests/ecs/tree_propagation_test.rs:37` の説明。本番の呼び手で `GlobalArrangement` を一緒に入れる所は 0（`git grep`）。
- 残してよい種類: 当たらない。

**14. 当たり判定の規則が 2 か所に写してあり、名前付きの領域では答えが食い違う**
- 場所: `layout/hit_test/mod.rs:285-340`（`hit_test_entity`）と `:386-487`（`hit_test_entity_ex`）。矩形のときの α の判定（`:308-335` と `:411-437`）がまるごと同じ。
- 何が悪いか: `hit_test_entity` は `None`・`Bounds` 以外を全部 α のマスクとして扱う（`:337-339`）ので、`NamedRegions` の部品も α のマスクで当否を決める。`hit_test_entity_ex` は `NamedRegions` を常に「当たり」とする。本番のクリック・カーソル・ツールチップは前者（`hit_test_in_window`・`clickthrough/controller.rs:202` など 6 か所）、領域の名前は後者を使う。さらに `_ex` 側の α のマスクの分岐はテストが 0：`tests_ex/alpha_mask_resource.rs:5`・`:127-131`・`:263-264` は「`hit_test_in_window` は `hit_test_entity_ex` 経由」と書くが、実際は `hit_test`→`hit_test_entity`（`mod.rs:551`）で、同じ側を 2 度試している。
- こうなると困る: `NamedRegions` の部品に `AlphaMaskResource` を付けると、クリックは透明な所を素通しするのに、領域の問い合わせは「当たり」と答える。片方だけ直すと、もう片方が古いまま残る。
- 直し方の案: `hit_test_entity` を `!matches!(hit_test_entity_ex(..), RegionHit::Miss)` の 1 行にし、写しを消す（約 40 行減）。`NamedRegions` をどちらに揃えるか（マスクも見る／見ない）は設計で決める。テストの説明を直し、`_ex` 側の α の分岐に 1 本足す。
- 範囲の外: 呼び手（`clickthrough`・`cursor`・`tooltip`・`window_proc`・areka の `pilot` の例・`areka-emo-present`・`areka-emo-text` のテスト）は関数の形が変わらないので触らない。`NamedRegions` を本番で使う所は 0（`crates/wintf/examples/taffy_flex_demo/` だけ）。
- 残してよい種類: 当たらない。

**15. 色分け画像の領域が 256 色を越えると、番号が回って別の名前を返す**
- 場所: `layout/hit_region/mod.rs:148`（`(region_names.len() + 1) as u8`）
- 何が悪いか: 領域の番号を 1 バイトに詰めている。256 個目は 0（＝名前なし）、257 個目は 1（＝1 個目の名前）になる。
- こうなると困る: 色の対応を 257 個渡すと、257 個目の色の画素で 1 個目の領域の名前が返る。エラーは出ない。（今の使い手は `taffy_flex_demo` の数色だけで、起きる条件は遠い。）
- 直し方の案: 255 個を越えたら `Err` を返す（`HitRegionError` に 1 つ足す）か、番号を `u16` にする。テスト 1 本。
- 範囲の外: なし。
- 残してよい種類: 当たらない。ただし要件 4.4（`hit-id-pass` が使い回す見込み）の申し送りに載せる。

**16. 色分け画像の透明な画素が「黒」の領域として当たり、半透明の画素は色がずれる**
- 場所: `layout/hit_region/mod.rs:165-174`（乗算済みの `32bppPBGRA` へ変換）・`:190-201`（RGB だけを見て α を捨てる）
- 何が悪いか: 乗算済みへ変換すると、透明な画素の RGB は (0,0,0) になり、半透明の画素は色が α 倍に変わる。そのうえで α を見ずに色だけで引く。
- こうなると困る: 背景が透明な PNG に、黒 (0,0,0) を「髪」として対応させると、透明な背景の全体が「髪」として当たる。縁を少しぼかした領域は、ぼけた画素が対応から外れる。
- 直し方の案: 乗算しない `32bppBGRA` へ変換し、α = 0 の画素は対応外にする（2〜3 行）。テスト 1 本（18 番の画像のテストと一緒に・`target/` の下に置く）。
- 範囲の外: なし。
- 残してよい種類: **⑶ の候補**。この読み込みは `hit-id-pass` が SSP の `collisionex` の `region`（指定した画像の指定色の領域）に使い回す見込み（`.kiro/steering/roadmap.md:470`）。ukadoc（https://ukatech.shillest.net/ukadoc-spec/descript/shell-surfaces/#collisionex-star）は「region: [SSP 2.5.19～]指定した画像ファイルの指定色の領域を当たり判定として扱う」「指定フォーマット：ファイル名,R,G,B,領域反転(falseかtrue)」とだけ書き、**透明な画素の扱いを書いていない**。決める前に SSP の MCP で測るか（記憶の決まり「ukadoc で読めない点は議題の前に SSP の MCP で測る」）、`hit-id-pass` へ申し送るかを議題にする（要件 4.3・4.4）。

**17. 色分け画像のパスを `to_string_lossy` で化かす**
- 場所: `layout/hit_region/mod.rs:155-156`
- 何が悪いか: 名を UTF-8 に直してから渡すので、対のないサロゲートを含む名が U+FFFD に化け、在るファイルが開けない。`audit-wintf-runtime` の 125 番が `com/wic.rs`（`:34-35`）と `areka-emo-atlas` で直したのと同じ形が、ここに残っている。
- 直し方の案: `HSTRING::from(image_path)`（1 行・`com/wic.rs:35` と同じ）。テスト 1 本（`com/wic_tests.rs` の `load_bitmap_source_opens_name_with_lone_surrogate` と同じ形・置き場は `target/` の下）。
- 範囲の外: なし。
- 残してよい種類: 当たらない。要件 4.4 の申し送りに載せる。

**18. 色分け画像の読み込み（`ColorMapData::from_image`）にテストが 1 本も無い**
- 場所: `layout/hit_region/mod.rs:135-217`。テストは手で組んだ `ColorMapData` だけ（`hit_region/tests/color_map.rs`）。`HitRegionMap::from_color_map` を呼ぶのは `crates/wintf/examples/taffy_flex_demo/setup.rs:387` だけ。
- 何が悪いか: 画素の並び（B・G・R・A の順）、色から番号への対応、画像の寸法の写しを試すものが無い。`b`・`r` を入れ替えても緑のまま。
- 直し方の案: 2×2 の PNG を `target/` の下に作って読み、各画素の名前を見るテスト 1 本（15・16・17 番の確かめもここに載る）。
- 範囲の外: なし。
- 残してよい種類: 当たらない。要件 4.4 の申し送りに載せる（消す案ではない）。

**19. 縦書き・横書きの設定が 2 か所に写してあり、失敗は捨て、テストが無い**
- 場所: `widget/text/draw_labels.rs:97-116` と `widget/text/typewriter_layout.rs:116-147`（揃えの 2 行が違うだけ）
- 何が悪いか: 4 つの向きの組み合わせ（読む向き・行の進む向き）を 2 か所で別々に書き、どちらも `let _ =` で失敗を捨てる（9 番）。向きごとの答えを試すテストは 0（`crates/wintf/tests/widget/label_defaults_test.rs` は既定値だけ。`audit-wintf-runtime` の 129 番がそれに合わせて名前を変えた）。
- こうなると困る: 片方だけ直すと `Label` と `Typewriter` で縦書きの出方が変わる。DirectWrite が組み合わせを断っても、黙って横書きで描かれる。
- 直し方の案: `fn apply_direction(format, direction) -> windows::core::Result<()>` を 1 つ作り、2 か所から呼ぶ。失敗は `warn!`。4 つの向きで `ReadingDirection`・`FlowDirection` を読み返すテスト 1 本（DirectWrite の工場だけで書ける・GPU は要らない）。
- 範囲の外: なし。
- 残してよい種類: 当たらない。

**20. モニタの `BoxStyle` を 3 か所で同じ形に組み立てている**
- 場所: `layout/systems/monitor_systems.rs:135-148`（起動）・`:179-188`（値の更新）・`:375-388`（新しいモニタ）
- 何が悪いか: 物理の大きさと左上から `BoxStyle`（大きさ・絶対配置・inset）を作る 13 行を 3 回写している。
- こうなると困る: 片方だけ直すと、起動時のモニタと後から挿したモニタで配置が変わる。
- 直し方の案: `fn monitor_box_style(&Monitor) -> BoxStyle` を 1 つにする。注意: `monitor_systems_transition_tests.rs:577-602` は `include_str!("monitor_systems.rs")` で本文の字面（`transition_diag::is_enabled()` が 2 回など）を数えている。数を変えない直しにする。
- 範囲の外: なし。
- 残してよい種類: 当たらない。

**21. 描画の骨組み（命令の列を作って閉じるまで）が 5 か所に写してあり、失敗の扱いがばらばら**
- 場所: `widget/shapes/rectangle.rs:175-258`・`widget/text/draw_labels.rs:145-219`・`widget/text/typewriter_draw.rs:135-279`・`:330-369`・`widget/bitmap_source/systems.rs:160-219`
- 何が悪いか: 「`CreateCommandList` → `SetTarget` → 変換を戻す → `BeginDraw` → 透明で消す → 描く → `EndDraw` → `close`」の同じ並びを 5 回書き、失敗は `warn!` のもの・黙るもの（`typewriter_draw.rs:330-369`）・`EndDraw` を捨てるもの（`systems.rs:208`）が混ざる。変換を戻す呼び方も `SetTransform`（3 か所）と `set_transform`（1 か所）で揃っていない。透明の色の定数も 3 回（`rectangle.rs:67-73`・`draw_labels.rs:15-21`・`typewriter_draw.rs:24-30`）。
- 直し方の案: `fn record(dc, draw: impl FnOnce(&ID2D1DeviceContext) -> Result<()>) -> Result<ID2D1CommandList>` を 1 つ作り、失敗は呼び手が 1 回 `warn!` する。9・10・12 番の直しがここで揃う。
- 範囲の外: なし。
- 残してよい種類: 当たらない。

**22. 要らない `unsafe impl Send/Sync` が、型の検査を黙らせている**
- 場所: `widget/text/typewriter/mod.rs:302-303`（`TypewriterLayoutCache`）・`widget/bitmap_source/resource.rs:79-80`（`BitmapSourceGraphics`）・テスト `typewriter/tests.rs:9-13`
- 何が悪いか: 中身の `IDWriteTextLayout`・`ID2D1Bitmap1` は windows-rs 0.62.2 が `Send`／`Sync` を付けている（`windows-0.62.2/src/Windows/Win32/Graphics/DirectWrite/mod.rs:10807-10808`・`Direct2D/mod.rs:2560-2561` で確かめた）。説明自身が「冗長」と書く。手で付けているので、後で `Send` でない欄を足してもコンパイラが止めない。テスト `test_typewriter_layout_cache_is_send_sync` は手の impl があるかぎり絶対に落ちない（説明自身が「検出できない」と書く）。`resource.rs:76-78` の説明は、もう無い `dcomp_resource.rs` を名指す。
- 直し方の案: 2 組の `unsafe impl` を消し、自動の導出に任せる（テストはそのとき初めて意味を持つ＝残す）。説明を消す。
- 範囲の外: なし（`GraphicsCommandList` などほかの COM の入れ物の慣習は `ecs/graphics/` の持ち物で変えない）。
- 残してよい種類: 当たらない。

**23. 使われていないコード**（消す前に木全体を `git grep` で確かめた・要件 3.3）
- `widget/bitmap_source/systems.rs:70-91` の `InsertBitmapSourceResource`（公開・参照 0。読み込みは閉包で入れている）
- `widget/bitmap_source/systems.rs:47-55` の `test_asset_path`（`cfg(test)`・参照 0。外のテストは `crates/wintf/tests/com/common/mod.rs:8` の別物を使う）
- `widget/shapes/rectangle.rs:15-65` の `Color` 別名と非推奨の `colors` の色（参照は自分のテスト `:337-361` だけ）。`Brush::*` と同じ値の写し。
- `layout/metrics.rs:21-34` の `LayoutScale::validate`（参照 0）
- `widget/bitmap_source/task_pool.rs:78-85` の `drain_and_apply`（範囲の中のテストだけ。本番は `drain_commands`）・`:100-112` の `send_command`・`is_empty`。**`is_empty` は常に `true` を返す**（説明自身が「確認できない」と書く）ので、それを確かめるテスト `bitmap_source/tests.rs:72-81` は何も試していない。
- `layout/hit_region/mod.rs:259-277` の `ColorMapDef`・`ColorMapping`（参照 0。serde の定義の入れ物で、読み込む所が無い）
- `widget/text/typewriter_ir.rs:112-122` の `TypewriterTimeline::empty`・欄 `total_duration`（書くだけで本番の読み手 0・テストだけ）
- 直し方の案: 消す。テストも消す。
- 範囲の外: なし（参照 0）。ただし公開の型・関数を消すのは crates.io の使い手への約束の変更（議題 D-6）。`ColorMapDef`・`ColorMapping` を消すと wintf の `serde` 機能を使う所が `Shape`・`ShapeRegion` だけになる。機能ごと外すと `crates/wintf/Cargo.toml` と `Cargo.lock`（wintf の依存に `serde` が載っている）が変わり、`Cargo.lock` の席は同じウェーブの `placement-crate-split` のもの（要件 5.8・C18 ⑹）。**機能は外さず型だけ消すか、後へ回すか**（議題 D-7）。`hit_region/` の中なので要件 4.4 の確かめにも当たる。
- 残してよい種類: `colors` は `#[deprecated(since = "0.1.0")]` を付けた「移行の期間」の印で、⑵ に近い（議題 D-6）。

**24. 書くだけで誰も読まない部品 `TextLayoutResource`・`TextLayoutMetrics`**
- 場所: `widget/text/draw_labels.rs:221-230`（毎回の描画で入れる）・`widget/text/label.rs:84-118`（型とフック）・`layout/metrics.rs:7-12`
- 何が悪いか: 読む所が本番に 0（`get()`・`empty()` を呼ぶのは `crates/wintf/tests/widget/label_defaults_test.rs` だけ・`TextLayoutMetrics` は `crates/wintf/tests/layout/metrics_test.rs` の既定値の確かめだけ）。文字の寸法を配置へ渡す仕組みも無い（`Label` の大きさは `BoxStyle` で別に決める）。説明の「metrics は物理サイズ」（`draw_labels.rs:222`）も誤り（DirectWrite の寸法は DIP）。
- こうなると困る: 描くたびに要らない部品を 2 つ入れ、読む人は「寸法が配置に効く」と誤る。
- 直し方の案: 2 つの型と入れる行を消す。
- 範囲の外: `crates/wintf/tests/widget/label_defaults_test.rs`・`crates/wintf/tests/layout/metrics_test.rs`（範囲の外の wintf のテスト・該当の確かめを消す）。areka の参照は 0。公開の型の削除（D-6）。
- 残してよい種類: 当たらない。

**25. `PhysicalPoint` という名前が、別々の 2 つの型を指す**
- 場所: `layout/hit_test/mod.rs:49-50`（`PhysicalPoint = PointF`・小数）。一方 `ecs/mod.rs:33-38`（`:35`）は `pointer::PhysicalPoint`（整数の `Point`）を名指しで再公開し、`pub use layout::*` の方を隠す。
- 何が悪いか: `wintf::ecs::PhysicalPoint` と `wintf::ecs::layout::PhysicalPoint` が違う型になる。当たり判定の関数は後者を取る。
- こうなると困る: `use wintf::ecs::*;` で `hit_test_in_window(&w, win, PhysicalPoint::new(..))` と書くと型が合わずに通らない。`crates/areka-emo-present/src/mount_tests.rs:22` がこの罠を注記で避けている。
- 直し方の案: `layout` の別名を消し、当たり判定の関数は `PointF` を取ると書く（型は同じなので呼び手の答えは変わらない）。
- 範囲の外: `crates/wintf/examples/taffy_flex_demo/main.rs:88`（`layout::PhysicalPoint` を取り込んでいる・`PointF` に）。`ecs/mod.rs` は触らない（要件 5.9）。areka は `pointer` の方を使っていて影響 0。
- 残してよい種類: 当たらない。同じ形の名前の重なりに `layout::Rect<T>`（`dimension.rs:216`）と `ecs::Rect`（`types::Rect`）があるが、こちらは罠に掛かった跡が木に無いので指摘にしていない。

---

### できれば（12 件）

**26. taffy の逆引きの表がテストのためだけにある**
- 場所: `layout/taffy.rs:46-47`・`:75`・`:86`（`node_to_entity`）・`:96-99`（`get_entity`）・`:111-128`（`verify_mapping_consistency`）
- 何が悪いか: 逆引きを読むのは `crates/wintf/tests/layout/taffy_advanced_test/`・`taffy_child_order_test.rs` だけ。本番は毎回両方の表を書く。
- 直し方の案: 逆引きの表と 2 つの関数を消し、テストは `get_node` で確かめる。範囲の外のテスト 3 ファイルを直す。

**27. `BitmapSource` のフックが同じ部品を 2 度入れる**
- 場所: `widget/bitmap_source/bitmap_source.rs:48-80`
- 何が悪いか: 4 つの枝の 1 つ目で `HitTest::alpha_mask()` を入れた後、`:75-80` でもう一度入れる。枝は「無ければ入れる」を 3 回書けば足りる。
- 直し方の案: `if !has_visual {..} if !has_graphics {..} if !has_hit_test {..}` の 3 行に。

**28. `BitmapSource.path` を書き換えても何も起きない**
- 場所: `widget/bitmap_source/bitmap_source.rs:21-24`（欄が `pub`）・読み込みは `on_add` だけ（`:91-133`）
- こうなると困る: `bs.path = "b.png".into()` は黙って無視される。
- 直し方の案: ㋐ 欄を読み取り専用にする（`path()` を足し、欄を非公開に）＝小さい。㋑ `Changed<BitmapSource>` で読み直す＝新しい機能に当たりうる（要件 3.6）。範囲の外: `crates/areka/examples/`・`crates/wintf/examples/` は `BitmapSource::new` だけで欄を書かない（`git grep`）。

**29. 古い説明・中身と違う説明**
- `layout/mod.rs:57-58`・`layout/rect.rs:179`（「将来 DirectComposition の Visual 層で」＝今の合成は WUC）
- `layout/mod.rs:104-112`（`LayoutRoot` の例が `BoxSize` を部品として入れる＝今は `BoxStyle` の値の型）・`widget/bitmap_source/bitmap_source.rs:13-18`（無い `BoxSize::fixed` を使う例）・`widget/shapes/rectangle.rs:78`（「`BoxSize` → Taffy」）
- `widget/shapes/rectangle.rs:89-90`・`widget/text/label.rs:41-42`（無い「draw_recursive 方式」と「Phase 4 完了後」）
- `widget/text/label.rs:38`（「pt 単位・範囲 8.0〜72.0」＝DirectWrite の大きさは DIP で、範囲の確かめも無い）
- `widget/text/typewriter_draw.rs:197-201`（「origin.X = text_metrics.width - layoutWidth」と書くが、コードは `(0,0)`）
- `layout/taffy.rs:11-13`・`:50-56`（「taffy 0.9」・根拠「wintf は calc を使わない」。今は 0.13 で、`TaffyStyle::new` は公開なので calc の値も入りうる。安全の本当の根拠は、`TaffyTree<()>` が calc のポインタを一度も読まないこと＝`taffy-0.13.0/src/tree/taffy_tree.rs:395-397` は 0 を返すだけ）
- `widget/bitmap_source/systems.rs:355`（11 番）・`resource.rs:76-78`（22 番）・`hit_test/tests_ex/alpha_mask_resource.rs:5`・`:127-131`（14 番）
- 直し方の案: 中身に合わせて直すか消す。

**30. 毎回の小さな無駄（文字列とログ）**
- 場所: `layout/systems/taffy_systems.rs:267`・`layout/systems/window_pos_systems.rs:35`・`widget/bitmap_source/systems.rs:135`・`widget/shapes/rectangle.rs:161`（`format_entity_name` で毎回 `String` を作る。使うのは `trace!`／`debug!` と失敗のときだけ）・`widget/text/typewriter_layout.rs:42-45`（毎回の `debug!` で問い合わせを数える）・`layout/systems/arrangement_systems.rs:61-82`（伝播の前の古い値を `trace!` に出す）
- 直し方の案: 名前は失敗のときだけ作る（`audit-wintf-graphics-world` の 17 番と同じ）。数える `debug!` と伝播前の `trace!` は消す。

**31. 何もしない・常に同じ答えのコード**
- `layout/box_style.rs:178-183`（`display = Flex` は taffy 0.13 の既定値そのもの＝`taffy-0.13.0/src/style/mod.rs:208`）・`:195-196`（`flex_grow`／`flex_shrink` の既定値の入れ直し）
- `layout/arrangement.rs:29-37`（手で書いた `Default` が `derive` と同じ）
- `layout/systems/window_pos_systems.rs:140`（ファイルの頭と同じ `use` をもう一度）
- `layout/systems/monitor_systems.rs:212`（`Query<(Entity, &mut Monitor), With<Monitor>>` の `With` が要らない）
- `widget/bitmap_source/systems.rs:223-231`・`widget/shapes/rectangle.rs:262-283`（新しく作った命令の列は COM の番地が必ず違うので `*existing != new_cmd_list` は常に真。`_ =>` の枝に来ない）
- 直し方の案: 消す・短くする。

**32. 前景色を既定へ落とす式が 5 か所、それを写したテストが 1 本**
- 場所: `layout/hit_test/mod.rs:318-327`・`:420-429`、`widget/shapes/rectangle.rs:170-173`、`widget/text/draw_labels.rs:68-71`、`widget/text/typewriter_draw.rs:99-102`。テスト `widget/brushes.rs:373-401` は式を写した閉包を試していて、本物の `draw_rectangles` を通らない。
- 直し方の案: `Brushes` に `fn foreground_color(&self) -> D2D1_COLOR_F` を 1 つ足して 5 か所から呼び、テストはそれを試す。（14 番で hit_test の 2 か所は 1 か所になる。）

**33. 値の型に `Component` を付けている**
- 場所: `layout/dimension.rs:39`・`:114`・`:171`・`:215`（`Dimension`・`LengthPercentageAuto`・`LengthPercentage`・`Rect<T>`）・`layout/metrics.rs:15`（`LayoutScale`）
- 何が悪いか: どれも欄としてだけ使い、部品として問い合わせる所は 0（`git grep` で `Query`／`With`／`spawn` の中に 0）。`box_style.rs:10-11` は「Component としての利用は廃止」と書く。
- 直し方の案: `Component` の derive を外す。公開の形の変更（D-6）。areka-emo-present・areka-emo-text は `LayoutScale` を欄としてだけ使うので影響 0。

**34. ログを 1 行出すだけの外しのフック**
- 場所: `widget/text/label.rs:113-118`（`on_text_layout_remove`・24 番で型ごと消える）・`widget/text/typewriter/mod.rs:83-86`・`:270-272`・`:333-335`
- 直し方の案: フックと属性を消す（3 番の直しで `TypewriterTalk` の外しのフックは中身を持つ）。

**35. 型の作りや定数を確かめるだけのテスト**
- 場所: `widget/text/typewriter_ir.rs:128-246`（列挙を作って `match` で取り出すだけの 9 本）・`widget/text/typewriter/tests.rs:15-35`（既定値・`PartialEq` の derive）・`widget/shapes/rectangle.rs:298-335`（単位の構造体・定数・型の別名）・`widget/brushes.rs:195-231`・`:285-371`（色の定数・derive）・`widget/bitmap_source/tests.rs:123-136`・`:153-170`（作り口が値をそのまま持つ）・`layout/hit_test/tests.rs:25-46`・`tests_ex/alpha_mask.rs:9-13`・`:75-79`（作り口）・`tests_ex/alpha_mask.rs:33-52`（すぐ上の本の写し・説明自身が「同じ」と書く）
- 何が悪いか: 壊れても困らない所を試していて、本物の誤りを捕まえない。
- 直し方の案: 消す。
- 残してよい種類: **⑴ の候補**。開発者の決め「決定論のテストの網羅は必達」との関係を確かめる（議題 D-8・`audit-wintf-graphics-world` の 34 番と同じ問い）。

**36. 名前付きの領域の当たり判定で、領域の表が無いとマウスが動くたびに警告**
- 場所: `layout/hit_test/mod.rs:449-456`
- こうなると困る: `taffy_flex_demo` の「フォールバックテスト」の部品の上でマウスを動かすと、動くたびに `warn!` が出る。
- 直し方の案: 1 度だけ出す（部品に印を付ける）か `debug!` に下げる。

**37. テストの COM の初期化が、失敗を捨て、入っていない部屋にも後始末を呼ぶ**
- 場所: `widget/bitmap_source/tests.rs:11-21`・`layout/hit_test/tests_ex/alpha_mask_resource.rs:197-209`
- 何が悪いか: `CoInitializeEx` の失敗を捨て、`CoUninitialize` を無条件に呼ぶ。`audit-wintf-runtime` の 130 番が `crates/wintf/tests/com/common/mod.rs` と `areka-emo-atlas` で直したのと同じ形。
- 直し方の案: 入れたときだけ後始末し、`RPC_E_CHANGED_MODE` のほかの失敗は落とす（130 番と同じ形）。

---

### 問題にしなかった所（確かめて、指摘にならなかったもの）

- `AlphaMask`（`widget/bitmap_source/alpha_mask.rs`）: 詰め方・再生成・範囲外の扱いが揃い、等価と再利用のテストもある。`areka-emo-present` が `regenerate_from_pbgra32`・`packed_capacity` を本番で使う。要件 4.4 の「消せる」とは指していない。
- `hit_region/` の矩形・多角形（`point_in_polygon` の除数 0 の守り・定義順の先勝ち）: テストが揃う。負や NaN の正規化座標を 0 の画素へ寄せるテスト（`tests/color_map.rs:268-300`）は今の飽和の動きの固定で、本番の呼び手は矩形の中の点しか渡さない（`hit_test/mod.rs:403-406` の早期の戻り）ので場面が無い。
- モニタの値の変化の更新と DPI の引き直し（`monitor_systems.rs:274-568`）: 値で比べる判定・半開区間・`CW_USEDEFAULT` の守り・刻印の出所まで、決定論のテストが非空虚の対照つきで揃う。`window_center`（`:448-458`）の `CW_USEDEFAULT` の守りは x と幅だけを見るが、y と高さだけが番兵の窓を作る所は木に無く、y の側の足し算は桁溢れしない。
- `sync_taffy_tree_system` の付け替え: taffy 0.13 の `set_children`（`taffy-0.13.0/src/tree/taffy_tree.rs:709-733`）は前の親から外すので、付け替えで古い親に残ることは無い。
- `window_pos_sync_system`（`window_pos_systems.rs:21-110`）: 位置の切り捨て・寸法の切り上げは、窓の bounds が整数の位置から来るので食い違わない。
- `WicCore`（`wic_core.rs`）: 暗黙の MTA を自ら保つ作りと理由の説明が揃う。
- `TaffyStyle`・`TaffyLayoutResource` の `unsafe impl Send/Sync`（`taffy.rs:14-15`・`:57-58`）: 本当に要る（calc のタグつきポインタで自動では付かない）。安全は保たれている（29 番の説明の直しだけ）。
- 当たりの α の閾値 128/255（`hit_test/mod.rs:311`・`alpha_mask.rs:7`）: 開発者の決め「当たりは α ≥ 0.5 の画素だけ」と合う。
- `ecs/mod.rs`・`ecs/world/mod.rs` を直す必要のある指摘は 0（2 番は係の中で直す案にした）。`monitor_systems_transition_tests.rs:32` は `include_str!("../../world/mod.rs")` で `schedules.insert(Schedule::new(Update));` の字面を見張る。同じウェーブの `wintf-visual-tree` が `ecs/world/mod.rs` を書き換えてその字面が消えると、このテストが赤になる（こちらの直しの問題ではないが、相手に知らせる）。

---

## 3. 要件との対応表

| 要件 | 今の状態 | 足りない所 |
|---|---|---|
| 1（範囲を確定して全部読む） | 50 ファイル（本番 32・7,217 行／テスト 18・4,251 行）を全部読んだ・一覧は 1 節 | なし |
| 2.1〜2.4（全件を記録） | 37 件・上限なし・番号と組つき | なし |
| 2.5（直し済みを数え直さない） | 5・94・101・146・144 は 2 節で確かめて除いた | なし |
| 2.6（`Arrangement` の on_add） | 指摘にする（13 番）・理由は 2 節 | なし |
| 2.7（3 ファイルは直した後の形） | 今の形で読んだ（10 番は立ち直りの作りを前提に書いた） | なし |
| 3.2（バグと危ない所に元に戻すと落ちるテスト） | 1〜8 番はどれもテストが無い | 1〜8・10〜19 番に新しいテスト（Missing） |
| 3.3（消す前に木全体で確かめる） | 23・24・26・33 番は確かめ済み（テスト・例・説明の参照まで） | 設計で消す範囲を確定するときにもう一度 |
| 3.4（形が変わる関数の呼び出し元） | 各件の「範囲の外」に列挙。areka の本番・`crates/areka/examples/` は形が変わらない | `crates/wintf/tests/`・`crates/wintf/examples/` の書き換え（Constraint） |
| 4.1〜4.3（残す指摘を裁定へ） | 候補は 16 番（⑶）・23 番の `colors`（⑵ に近い）・35 番（⑴） | 要件の討議で裁定（下の D-1〜D-9）。16 番は ukadoc に書いてない点を SSP で測るか（Research Needed） |
| 4.4（`hit_region/`・`alpha_mask.rs`） | audit は両方を「消せる」と指していない。`hit_region/` の中の 15〜18 番（直す）・23 番の `ColorMapDef`・`ColorMapping`（消す案）が当たる | `hit-id-pass` への申し送りを議題に（D-4） |
| 4.5（触らない約束に当たる指摘） | `ecs/mod.rs`・`ecs/world/mod.rs` 0 件・`crates/areka/` 0 件・`Cargo.lock` は 23 番の serde の機能を外す場合だけ | D-7 |
| 5.1（1,000 行） | 最大 632 行 | なし |
| 5.3（記録の無い失敗の道） | 9 番が該当 | 直す |
| 5.6（GPU のテストは実 GPU） | 7・10・12・21 番のテストは実の装置で回す | なし |
| 5.8（依存を足さない） | どの案も依存を足さない | なし |
| 5.9〜5.12（並走の約束） | 2 節「範囲の外」のとおり | 2 番の直し方を係の中に限る |
| 6（かけ直しの audit で 0 件） | 未実施 | 実装の最後のタスク |

---

## 4. 進め方の案

タスクの見積もり（小物はまとめる）: 必ず直す 8 件で 6〜7（taffy の 2 件・打ち出し表示の 3〜4・画像の 1）・直したほうがよい 17 件で 9〜11（失敗の記録と描く骨組みと毎回のやり直しを 1〜2 にまとめる・当たり判定 1・色分け画像 1〜2・向き 1・モニタ 1・使われないコード 1〜2）・できれば 12 件で 3〜4・最後のかけ直しの audit と全体テスト 1〜2。合計 **19〜24**。20 を越えうる（要件 6.7）。

### 案 A: この spec 1 本で全部直す
- 良い点: 1 回で終わる。描く骨組み（21 番）を先にまとめると、9・10・12 番が一度に片づく。
- 悪い点: 20 を越えうる。

### 案 B: 組で 2 本に分ける（要件 6.7）
- 1 本目＝必ず直す 1〜8 番と、それに絡む 9〜14・19・21 番（振る舞いが変わるもの・バグの根）。見積もり 13〜16。
- 2 本目＝残りの直したほうがよい（15〜18・20・22〜25 番）とできれば（26〜37 番）。見積もり 8〜10。消す・まとめる・説明が大半で、公開の形の議題（D-6）の答えをそのまま持っていける。
- 良い点: どちらも 20 以下。1 本目だけを先に着地させれば、`wintf-d2d-composite-rules`・`hit-id-pass` を待たせる時間が短い（どちらも描画と当たりの振る舞いの直しを待つ）。
- 悪い点: 同じ範囲で 2 本走らせる手間。要件 6 の「同じ範囲で 0 件」は 2 本目の最後で見る形にする必要がある。

### 案 C: 場所で 2 本に分ける（`layout/` と `widget/`）
- 見積もり: `layout/` 10〜12・`widget/` 11〜13。
- 悪い点: 14 番（当たり判定）と 32 番（前景色）・7 番（画像の当たりのマスク）が両方にまたがる。21 番の骨組みは `widget/` だけで済む。

### 規模と危うさ
- 規模: M（既存の形のまま直す件が大半。新しい作りは無い）。
- 危うさ: 中。1・2 番は taffy の配置の結果が変わる（正しくなる方へ）ので、`crates/wintf/tests/layout/` の配置のテストで前後を確かめる。14 番は本番のクリックの判定の関数に触る（`NamedRegions` を本番で使う所は 0・矩形と α のマスクの答えは変えない）。6 番は時間表の作り方を変える。7・10・11・12 番は描画の流れに触るので、`audit-wintf-graphics-world` の立ち直りのテスト（`crates/wintf/tests/graphics/device_lost_recovery_test.rs` など）を前後で回す。

---

## 5. 設計へ持ち越す調べもの（Research Needed）

- 16 番: SSP の `collisionex` の `region` で、透明な画素（α = 0）が指定色に当たるか。ukadoc に書いてない＝SSP の MCP で測るなら、測る前に開発者の許可を取る（記憶の決まり）。`hit-id-pass` へ申し送るなら測りもあちらへ。
- 19 番: DirectWrite の `IDWriteTextFormat` は `SetReadingDirection(TOP_TO_BOTTOM)` と `SetFlowDirection` を呼ぶ順で断るか（今の順は「読む向き → 行の向き」）。断るなら今の縦書きは黙って横書きになっている。テストを書けば分かる。
- 2 番: 計算の係が `RemovedComponents` を読んでも、掃除の係（後に並ぶ）が同じ印を読めること（bevy の `RemovedComponents` は係ごとに読み位置を持つ＝読めるはず）。
- 6 番: クラスタの長さ（UTF-16）からトークンの境目を引く形で、改行（`\n` は 1 クラスタ）と結合文字が正しく数わるかの確かめ。
- 7 番: 外すフックで `HitTest` を外すか残すか（利用者が入れた `HitTest` との見分け方）。
- 11 番: `CreateBitmapFromSource(WICBitmapCacheOnLoad)` で持たせた中身を、`CreateBitmapFromWicBitmap` がそのまま使うか（解読が 1 回になるか）。

---

## 6. 要件の討議へ持っていく議題（答えで作業が変わるもの）

- **D-1（進め方）**: 4 節の案 A・B・C のどれか。推す案は B（20 を越えうるため。1 本目を先に着地させ、待っている `wintf-d2d-composite-rules`・`hit-id-pass` を早く解く）。
- **D-2（14 番）名前付きの領域の当たり**: `NamedRegions` の部品で α のマスクも見るか（今の `hit_test_entity`）、見ないか（今の `hit_test_entity_ex`）。推す案は「見ない」（名前付きの領域はその表で形を決める・本番の使い手 0）。
- **D-3（7 番）`BitmapSource` を外したときの `HitTest`**: フックが入れた `HitTest::alpha_mask()` を外すか残すか。推す案は「`BitmapSourceResource` だけ外す」（`HitTest` は利用者が入れたものかもしれない・マスクが無ければ矩形に戻る）。
- **D-4（要件 4.4）`hit_region/` の色分け画像の読み込み**: 15・16・17・18 番は「消す」でなく「直す・試す」。`hit-id-pass` が使い回す見込みなので、⒜ この spec で直してから渡す ⒝ 16 番（透明の扱い＝SSP の `region` の意味）だけは `hit-id-pass` へ申し送る ⒞ 4 件とも申し送る。推す案は ⒝（15・17・18 番は SSP に依らない直し。16 番は SSP の振る舞いを測ってから決める）。あわせて 23 番の `ColorMapDef`・`ColorMapping`（使い手 0 の定義の入れ物）を消してよいか。`wintf-d2d-composite-rules` の議題（`alpha_mask.rs` を置き換えるか）に響く指摘は無い（`alpha_mask.rs` への指摘は 0）。
- **D-5（16 番）⑶ の候補**: SSP の MCP で `region` の透明な画素の扱いを測るか（測るなら reload の許可を取る）。
- **D-6（23・24・33 番）公開の型・関数を消すか**: wintf は crates.io に出すライブラリ。`InsertBitmapSourceResource`・`colors`（`#[deprecated]` の移行の期間＝⑵ に近い）・`LayoutScale::validate`・`TextLayoutResource`・`TextLayoutMetrics`・値の型の `Component` などを消すのは公開の形の変更。推す案は「消す」（版はまだ 0.x・使い手は木の中だけ・areka の参照 0・`crates/areka/examples/` の参照も 0）。
- **D-7（23 番の serde・要件 4.5）**: `ColorMapDef`・`ColorMapping` を消した後、wintf の `serde` 機能を外すと `Cargo.lock` が変わる（席は `placement-crate-split`）。推す案は「機能は残して型だけ消す」（`Shape`・`ShapeRegion` にまだ付いている・機能を外すのは後のウェーブの依存の棚卸へ）。
- **D-8（35 番）⑴ の候補**: 型の作り・定数・derive を確かめるだけのテストを消してよいか。「決定論のテストの網羅は必達」の決めとの関係。推す案は「消す」（網羅は振る舞いの網羅・`audit-wintf-graphics-world` の D-8 と同じ答えに揃える）。
- **D-9（28 番）`BitmapSource.path`**: ㋐ 読み取り専用にする ㋑ 書き換えで読み直す（新しい機能に当たりうる・要件 3.6）。推す案は ㋐。
- **申し合わせ（議題ではなく手順）**: `monitor_systems_transition_tests.rs:32` が `ecs/world/mod.rs` の字面を見張っていることを、同じウェーブの `wintf-visual-tree` のセッションに知らせる。`crates/wintf/tests/`・`crates/wintf/examples/` の書き換え（13・14・24・25・26・33 番）は、持ち主の `audit-wintf-runtime` が完了しているので申し合わせは要らない。

---

## 7. まとめ

- 範囲は 50 ファイル・11,468 行（本番 7,217・テスト 4,251）。全部読んだ。指摘は 37 件（必ず直す 8・直したほうがよい 17・できれば 12）。`audit-wintf-runtime` の直し済み 5 件は数えていない。`Arrangement` の on_add は指摘にした（13 番）。
- 必ず直すの芯は、taffy の木のつなぎ（後から付けた `BoxStyle`・消した子）と、`Typewriter`（トークの入れ替え・最後の待ちと知らせ・宛先が消えると落ちる・文字の数え方）と、`BitmapSource` の付け直し。どれも areka の本番の画面には出ないが、wintf の公開の部品として普通の使い方で壊れる。
- 直したほうがよいの多くは、描く骨組みの 5 重の写しと、捨てた失敗と、毎回のやり直し・毎回の描き直し・UI スレッドでの 2 度の解読で、骨組みを 1 つにすると一度に片づく。
- 範囲の外へ及ぶのは `crates/wintf/tests/`・`crates/wintf/examples/` だけで、`ecs/mod.rs`・`ecs/world/mod.rs`・`ecs/visual/`・`ecs/graphics/`・`crates/areka/` には触らずに直せる。`Cargo.lock` に触るのは serde の機能を外す場合だけ（D-7）。
- タスクは 19〜24 で 20 を越えうる。組で 2 本に分ける案 B を推す。

---

## 8. 設計の段の調べ（2026-11-06・`/kiro-spec-design -y`）

### 8.1 調べの種類と使った手引き

- 種類: 軽い調べ（在るコードの直し・依存の追加なし）。`kiro-spec-design` の手引き（`design-discovery-light.md`・`design-synthesis.md`・`design-review-gate.md`）と、ponytail の作法（いちばん小さな完全な変更・消せるものは消す）に従った。
- 調べた所: 範囲のファイル（直す所の周り）、bevy_ecs 0.19.1・windows 0.62.2・taffy 0.13.0 の手元の写し（`C:\rust\cargo\registry\src\index.crates.io-1949cf8c6b5b557f\` の下）、Microsoft の DirectWrite の説明。
- 5 節の調べものは 16 番を除いて全部この節で答えた（16 番は `hit-id-pass` へ申し送り済み）。

### 8.2 2 番: 消えたことを計算の係が読んでも、掃除の係は読めるか

- `RemovedComponents<T>` は係ごとに読み位置を持つ（`bevy_ecs-0.19.1/src/lifecycle.rs:511-514` の `reader: Local<'s, RemovedComponentReader<T>>`）。`read()`（`:578`）は自分の読み位置だけを進め、`is_empty()`（`:603`）は読み位置を進めない。
- 消えた記録の入れ物を古い順に捨てるのは `World::clear_trackers`（`world/mod.rs:1735-1736`）だけで、wintf はどこでも呼ばない（`crates/wintf/src/runtime/mod.rs:255-257` に同じ確かめがある）。だから、計算の係・掃除の係・木をつなぐ係がそれぞれ同じ消えた記録を 1 度ずつ読める。
- 設計への含み: 計算の係は、読んだかどうかを `is_empty()` で見るだけだと読み位置が進まず、次の回もまた「消えた」と見て計算し直す。2 つの読み口を毎回両方とも読み切る（`read().count()` を両方取ってから比べる。`||` で片方を飛ばさない）。
- 設計の調べで見つけた穴: `BoxStyle` を外して `TaffyStyle` を外した部品は、`ChildOf` が残るので木をつなぐ係の「`ChildOf` が消えた」の道に乗らない。掃除の係は計算の後に並ぶので、その回の計算は消える前の節を含んだまま走り、次の回は何も起きない。そこで木をつなぐ係（計算より前）が `RemovedComponents<TaffyStyle>` も読み、その節を taffy の親から外す。係の並び（`ecs/world/mod.rs:243-256`）は変えない。

### 8.3 19 番: DirectWrite の向きの設定の順

- Microsoft の `IDWriteTextFormat::SetReadingDirection` の説明（https://learn.microsoft.com/en-us/windows/win32/api/dwrite/nf-dwrite-idwritetextformat-setreadingdirection）は、読む向きと行の向きが「must always be set 90 degrees orthogonal to each other」で、外れると `Draw` や `GetMetrics` のような配置を使う所で `DWRITE_E_FLOWDIRECTIONCONFLICTS` になると書く。設定する関数そのものは順で断らない。
- 設計への含み: 今の順（読む向き → 行の向き）のままでよい。確かめは、4 つの向きで設定を読み返し、その書式で作った配置の `GetMetrics` が通ることで行う（DirectWrite の工場だけで書ける・GPU は要らない）。

### 8.4 11 番: 解読を 1 回にする

- windows 0.62.2 の `IWICImagingFactory::CreateBitmapFromSource(source, option) -> IWICBitmap`（`Windows/Win32/Graphics/Imaging/mod.rs:3559`）と `WICBitmapCacheOnLoad`（`:6159`・値 2）。`WICBitmapCacheOnLoad` は作るときにメモリへ写しを作る。今の `load_bitmap_source` が返す「形式の変換器」は読むときに初めて解読するので、`CreateBitmapFromSource(変換器, WICBitmapCacheOnLoad)` を裏の作業で 1 回呼べば、解読は裏の 1 回だけになり、UI スレッドの `copy_pixels`（αのマスク）と `CreateBitmapFromWicBitmap`（描画）はメモリの写しを読むだけになる。
- 要る windows の機能 `Win32_Graphics_Imaging` は根の `Cargo.toml` で有効（`Win32_Graphics_Imaging_D2D` が在る）。依存は足さない。
- windows 0.62.2 の Imaging には `unsafe impl Send` が 1 つも無い（grep 0 件）。`BitmapSourceResource` の手書きの `Send`／`Sync` は要るので残す（22 番は `BitmapSourceGraphics` と `TypewriterLayoutCache` だけ）。
- 確かめのテスト: 裏の作業が渡す画像が `IWICBitmap` へ `cast` できること。変換器は `IWICBitmap` を持たないので、直しを戻すと落ちる。

### 8.5 3・5 番: bevy 0.19 のフックと「無ければ何もしない」命令

- 部品を入れるたびに `on_insert` が呼ばれる（入れ替えでも呼ばれる）。外すときは `Discard` → `Remove` の順（`lifecycle.rs:24-30`）。
- `EntityCommands::try_remove`（`system/commands/mod.rs:1827`）・`try_insert`（`:1626`）は相手が消えていたら何もしない。`Commands::get_entity`（`:490`）は今いるかを確かめて `Result` を返す。
- 設計への含み: 3 番は `TypewriterTalk` に `on_insert` を足し、控えを `try_remove` で外す。外すときのフックは `Label` の `on_label_remove`（`label.rs:76-82`）と同じく命令の列を空にする。5 番は `get_entity` で確かめ（消えていたら `debug!`）、`try_insert` で入れる（同じ回のうちに消えた場合の守り）。

### 8.6 6 番: 文字の数え方

- DirectWrite のクラスタの長さ（`DWRITE_CLUSTER_METRICS.length`）と範囲（`DWRITE_TEXT_RANGE`）はどちらも UTF-16 の単位。時間表は「トークンの UTF-16 の終わりに届くまでクラスタを進める」形にすれば、サロゲートの対（1 クラスタ 2 単位）・異体字セレクタや結合文字（2 文字で 1 クラスタ）・改行（1 クラスタ）が同じ規則で数わる。トークンの境目をまたぐクラスタは前のトークンが受け持つ。
- 隠す範囲は、見せるクラスタの長さの和から、全部のクラスタの長さの和まで。どちらもクラスタの長さだけで出せるので、DirectWrite の配置を持たない純な関数にでき、テストは CPU だけで書ける。

### 8.7 20 番: モニタのテストが本文の字面を数えている件

- `monitor_systems_transition_tests.rs:577-602` が `include_str!("monitor_systems.rs")` で見る字面は 4 つ: `transition_diag::stamp_from_world(&frame, &tick_start)` が在る・`transition_diag::stamp()` が 0 回・`transition_diag::is_enabled()` が 2 回・`pub fn detect_display_change_system(` が在る。`BoxStyle` を組む関数を 1 つにまとめても、この 4 つの数は変わらない。
- 同じファイルの `:32` が見張る `ecs/world/mod.rs` の字面には、この spec は触らない（要件 5.9）。

### 8.8 消すもの・形が変わるものの確かめ（要件 3.3・3.4）

- `ecs/mod.rs` が名指しで再公開する範囲の名前（`BitmapSource`・`BitmapSourceGraphics`・`BitmapSourceResource`・`BoxedCommand`・`CommandSender`・`WicCore`・`WintfTaskPool`・`draw_bitmap_sources`・`Typewriter` 一式・`TypewriterTimeline`）は、どれも消さない。`TextLayoutMetrics` は `pub use layout::*` の glob で出ているだけなので、消しても `ecs/mod.rs` を書き換えずに済む。
- `ecs/world/mod.rs` が名指す範囲の係（`build_taffy_styles_system` ほか 27 個）は、名前を全部残す。係の引数が変わっても、bevy が埋めるので登録の行は変わらない。
- `layout::PhysicalPoint` を名指す所は `crates/wintf/examples/taffy_flex_demo/main.rs:88`（`region.rs:15` はそれを使う）と範囲の中のテストだけ。areka は `pointer` の方を使う（`crates/areka/src/emo2_boot/frame/dpi.rs:11` など）。
- taffy の逆引き（`get_entity`・`verify_mapping_consistency`）を読むのは `crates/wintf/tests/layout/taffy_advanced_test/mapping_management.rs`・`tree_sync.rs`・`crates/wintf/tests/layout/taffy_child_order_test.rs` の 3 ファイル。
- 値の型を部品として使う所（`Query`・`With`・`spawn`・`insert` の中）は木の全体で 0 件（2026-11-06 に grep）。
- **設計の調べで見つけた食い違い（28 番）**: `crates/areka/examples/mock-shell.rs:609` が `assert_eq!(bitmap.path, SHELL_IMAGE_PATH);` で `BitmapSource.path` を読む。ギャップ分析は「欄を書く所が 0」を確かめたが、読む所を見落としていた。欄を非公開にすると、このウェーブでは触れない `crates/areka/` の例が通らなくなる（要件 5.11・5.12）。要件 5.12 の決まりどおり、要件 4.5 の議題に回す（答えは 8.11）。
- 設計の調べで見つけたもう 1 つ（直さない・説明だけ）: `crates/areka/examples/clickthrough_two_rects.rs:192` の説明が「`Added<BitmapSourceResource>` ＋ `With<HitTest>` ＋ `HitTestMode::AlphaMask` の時だけ」と書く。7 番でαのマスクの係を `Changed<BitmapSourceResource>` にすると、説明が少し古くなる（動きは変わらない・付けた時に作るのは同じ）。このウェーブのあいだは `crates/areka/` に触れないので、28 番の前提つきのタスクで一緒に直す（8.11）。

### 8.9 まとめ（synthesis）

- **まとめられるもの（一般化）**: 命令の列を記録する骨組みを 1 つの関数にし、記録の結果を部品へ入れる所も 1 つにする（21 番）。これで 9 番（捨てた失敗）・10 番（恒久の失敗の毎回のやり直し）・30 番（名前の文字列は失敗のときだけ）・31 番（いつも真の比べ）の描く 5 か所の分が一度に片づく。当たり判定は `hit_test_entity_ex` を 1 本の正本にし（14 番）、前景色の式は `Brushes` の 1 つの関数にする（32 番）。向きの設定（19 番）とモニタの `BoxStyle`（20 番）も 1 つずつにする。
- **作るか使うか**: 新しく作る仕組みは無い。bevy の `on_insert`・`try_remove`・`try_insert`・`get_entity`、WIC の `WICBitmapCacheOnLoad`、テストの PNG を書く `image`（wintf の開発用の依存に既に在る）を使う。
- **削ったもの（簡素化）**:
  - 11 番のαのマスクは、UI スレッドでメモリの写しから `copy_pixels` する今の形のまま残す（裏へ移す命令の道を足さない）。解読は裏の 1 回だけになり、ギャップ分析の「2 度の解読」は解ける。
  - 36 番は印を持たせず `debug!` に下げる（当たり判定は `&World` しか持たず、印を書けない）。
  - 15 番は番号の型を広げず、255 色を越えたら `error!` を残して失敗を返す（`from_image` の戻りの型は今のまま）。

### 8.10 変えてはならない所（2026-11-06・取りまとめ役からの申し送りを確かめた）

- `crates/wintf/src/ecs/layout/systems/monitor_systems.rs:564` の記録の文「`[detect_display_change_system] Redriving window DPI from updated Monitor (no WM_DPICHANGED required)`」は、`crates/areka/src/placement_tests/transition_judge_reobservation_tests.rs:74` が実機ログの写し（逐語の fixture）として使い、`monitor_systems_tests.rs:350` も確かめに使う。20 番（`BoxStyle` の組み立てをまとめる）と 30 番（ログを削る）は、この行の文を変えない。完了した spec の手順書（`.kiro/specs/completed/areka-P0-dpi-window-vanish/diagnosis-procedure.md`）も実機の grep の語に使っている。
- `crates/wintf/src/ecs/graphics/visual.rs:19`・`:135` の `Visual` の付けたときのフックは `Arrangement::default()` だけを入れ、`GlobalArrangement` と `ArrangementTreeChanged` は `Arrangement` のフックの連鎖に任せている。13 番の `insert_if_new` は「無ければ入れる」なので、この連鎖は今のまま効く（変わるのは、束で渡した値を上書きしなくなる所だけ）。

### 8.11 28 番の道（2026-11-06・取りまとめ役の答え・設計の討議で開発者に確かめる）

- 取る道: 要件 4.5 の「相手の着地の後へ回す」（設計の段の報告で ⒜ として推した案）。
- 理由: 欄を非公開にすると `crates/areka/examples/mock-shell.rs:609` が組み立たなくなり、このウェーブのあいだは `crates/areka/` に触れない（要件 5.11・5.12）。開発者は `wintf-visual-tree` と `placement-crate-split` をこの spec より先に実装する見込みなので、この spec の実装は自然に `placement-crate-split` の着地の後になる。
- 扱い:
  - 前提なし（今の段）: `BitmapSource::path()` を足し、範囲の中の読み手を替える。
  - 前提つき（`placement-crate-split` が main に着地し、この枝が main を取り込んだ後だけ）: 欄 `path` を非公開にし、`mock-shell.rs:609` を `bitmap.path()` に替える。同じタスクで `crates/areka/examples/clickthrough_two_rects.rs:192` の説明を 7 番の後の条件に合わせる（8.8）。
  - 前提が満たされていないとき: そのタスクは手を付けずに止まり、開発者へ尋ねる。
- あわせて記録する決まり（取りまとめ役の申し送り）: `monitor_systems_transition_tests.rs:32` は `include_str!` で `ecs/world/mod.rs` の字面を見張る。同じウェーブの `wintf-visual-tree` が先にその行を変えたなら、この spec は main を取り込み、テストがその字面に合ったままであることを確かめるだけにする（`ecs/world/mod.rs` には触らない）。
