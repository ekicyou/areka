# ギャップ分析: audit-emo-text-state-actor（文字の層の状態・アクター・選択肢・窓の見直し）

- 測った時点: `main` `73c5b25d`（枝 `claude/kiro-start-audit-emo-text-state-actor-dcff96`・`f7335050`）・2026-11-06。このクレートを最後に変えたのは `0f2f3ecd`（`audit-parsers-sakura`・PR#428）で、棚卸㉟の測り（`18e4cb30`）より前。
- この文書はコードを変えていない。決めごとは要件の討議に回す（9 節）。
- **この回でしたこと・しなかったこと**: この回は、受け持つ範囲の大きさ・守る約束・持ち込みの件・要件 1.5 の前提の確かめ・タスクの数の見込みだけを調べた。**`/ponytail:ponytail-audit` の全件の洗い出し（要件 1.1〜1.4）は、この回ではしていない**（進行役の指示）。要件 1.1 は「ギャップ分析を行うときに audit をかける」と定め、完了した兄弟の見直し 7 本はどれもギャップ分析の報告に指摘の全件を載せてから要件の討議へ進んだ。指摘の一覧が無いと、要件の討議で直す対象を確定できない（要件 1.6）し、2 本に切るかの見込み（要件 8.1）も立たない。扱いは議題 1。

## 1. 要約

- **何をするところか**: バルーンの文字の層のうち、配置と描画（`audit-emo-text-layout-draw` の持ち物）を除いた部分。台本の指令を受けてスコープごとの表示の状態を持つ（`state*`）、スコープごとの描き手・箱・スクロール・装飾の差し込みを毎フレーム動かす（`actor*`）、選択肢の当たりと強調（`choice*`）、指令の受け口（`sink*`）、描く面と窓の子（`surface*`）、印の面と矢印（`inline_marker*`・`marker_surface*`・`scroll_marker*`）、面ごとの上書き（`balloon_overrides*`）、公開の一覧と番人（`lib.rs`）。
- **大きさ**: 121 ファイル（`src/` 68・`tests/` 40・`examples/` 12・`Cargo.toml`）。合わせて 45,385 行（要件の表の コード 10,832 行＋テスト 34,553 行と一致）。テストを除く本番のソースは 19 ファイル・8,731 行。
- **要件 1.5 の前提（1 つの World に文字の層は 1 つ）**: **今のコードでは成り立つ**。本番で文字の層を作る所は 1 か所だけで、その持ち主は World に 1 つしか置けない置き場に載る（4 節）。ただしコードの形では守られておらず、ゴーストの切り替えでは古い印が World に残ったまま新しい文字の層に引き継がれる（今は害が無い）。
- **持ち込みの件（要件 7）**: **今も直っていない**。行の位置も brief のとおり（`src/actor_box.rs:236`・`:237`・`:427`）。壊れたら落ちるテストも無い（5 節）。
- **大きさの見込み**: 指摘の数しだい。兄弟の見直しの実績から、直しは 18〜30 タスクの見込み。兄弟は 20 を越えても 1 本のまま進めた（7 節）。

## 2. 受け持つ範囲（要件 1.1・1.3 の読む対象）

| まとまり | ファイル | 行 |
|---|---|---|
| `src/` の本番のソース | 19（`actor.rs`・`actor_attach.rs`・`actor_box.rs`・`actor_decoration.rs`・`actor_present.rs`・`actor_scroll.rs`・`balloon_overrides.rs`・`choice.rs`・`inline_marker.rs`・`lib.rs`・`marker_surface.rs`・`scroll_marker.rs`・`sink.rs`・`state.rs`・`state_anchor.rs`・`state_decoration.rs`・`state_route.rs`・`surface.rs`・`surface_window_child.rs`） | 8,731 |
| `src/` の `actor_*` のテスト（`actor_test_support.rs` を含む） | 23 | 10,324 |
| `src/` の `state_*` のテスト（`state_test_support.rs` を含む） | 13 | 4,946 |
| `src/` の `choice_*` のテスト | 5 | 2,837 |
| `src/` のそのほかのテスト（`sink_*`・`surface_*`・`balloon_overrides_*`・`inline_marker_*`・`marker_surface_*`・`scroll_marker_*`） | 8 | 2,175 |
| `tests/`（Rust 35・設定の検体 5） | 40 | Rust 14,271 |
| `examples/`（Rust 8・検体 4） | 12 | Rust 2,101 |
| `Cargo.toml` | 1 | — |

- 1 ファイル 1,000 行の上限の近く: `surface.rs` 995・`actor_choice_contract_tests.rs` 991・`choice_tests.rs` 980。ほかは 900 行未満（`actor_region_warn_tests.rs` 894・`actor.rs` 841）。
- わざと置いた先送りの印（要件 3.1 の 2 種類目の候補）: `surface.rs:269` の `sprite`（外形を追う将来の操作口）・`:276` の `fixed_overlay`（`\_b --option=fixed` の固定層の型の口）に `#[allow(dead_code)]` と理由の説明がある。`scroll_marker.rs:229` に `shortcut:` の注釈が 1 つ（自動の範囲の行数を 1 画面とみなす・行の丈が揃わない台本でずれうる）。`ponytail:` の注釈は 0。audit がこれらを指したときは、限界が今の負荷を越えているかで議題にするかを分ける。
- 読む手分けの案（兄弟の `audit-emo-compose` は、本番のソースを進行役が全行読み、テストを下請け 4 人で全行読んで、指摘の行を進行役が開いて確かめた）: 上の表の 7 つのまとまりを 5 人前後で分ける。`actor_*` のテスト（約 10,300 行）は 2 人に分ける。

## 3. 範囲の外から使われている所（要件 5.1 の「変えない公開の形」）

このクレートを使う範囲の外のクレートは `crates/areka` の 1 つだけ（`Cargo.toml` で依存するのも `crates/areka` だけ）。`crates/areka/src/` の 62 ファイルが `areka_emo_text` を名指す（要件の「約 60」と合う）。名指しの多いものは次のとおり（数は名指しの回数）。

- 受け持つ範囲の中: `state::TextLayerConfig`（22）・`actor::TextLayerRuntime`（20＋`::new`）・`actor` のモジュールそのもの（17）・`scroll_marker` の `WheelLines`・`StopReason`・`ARROW_STEP_LINES`（計 12）・`state::SpanKind`（5）・`sink::EmoTextSink`（2）・`actor::ResolvedBalloonText`・`actor::HitRectPx`（各 2）・`actor::spawn_emo_text`・`actor::present_frame`・`state::TextItem::Glyph`・`state::SurfaceKeyOutcome`・`surface::PendingReadBack`・`choice::to_window_physical`（各 1）。
- 受け持つ範囲の外（`audit-emo-text-layout-draw` の持ち物）: `place::PlaceKey`（10）・`place::TextPlace`・`look`・`draw::DEFAULT_BALLOON_BACKGROUND`・`writing::WritingMode`・`layout::LineRect`・`color::mix_disabled`・`region`。

`TextLayerRuntime` の公開の関数（`register_actor_view`・`refresh_actor_scale` など）は `crates/areka/src/emo2_boot/` の本番とテストから呼ばれている。直しで名前・引数・戻り値・欄を変えると要件 5.1 に触れる。足すだけなら触れない。

## 4. 要件 1.5: 「1 つの World に文字の層は 1 つ」は今のコードで成り立つか

**結論: 成り立つ。** ただしコードの形で守られてはおらず、守っているのは本番の組み立て方だけ。

### 4.1 印の置き場と使い方

- `crates/areka-emo-text/src/actor_present.rs:130-134`: 印 `SeenGraphicsCore(Tick)` は World の資源（`#[derive(Resource)]`）で、説明に「World ごとに 1 つなので、1 つの World に文字の層は 1 つの前提（本番はゴーストの World ごとに 1 つ）」とある。私的な型で、ほかのファイルは使っていない。
- `actor_present.rs:140-160`（`drop_renders_after_device_change`）: `GraphicsCore` の変わった時刻を、World の印と比べる。新しければ印を今の時刻に書き換え（`:153-154`）、文字の層の描き手を全部捨てる。印が無い初めての回は印を置くだけで戻る（`:155-157`）。
- 呼ばれるのは `present_frame` の頭の 1 か所だけ（`actor_present.rs:63`）。

### 4.2 本番で文字の層が 1 つになる理由

- 本番で `TextLayerRuntime::new` を呼ぶのは `crates/areka/src/emo2_boot/mod.rs:475`（`wire_emo2_boot` の手順 2）だけ。もう 1 つの `crates/areka/src/emo2_boot/spine.rs:666` はテストの足場で、自分の `World` を作る（`spine.rs:440` の `make_world_with_gpu` の中の `:450`）。残りは全部テスト・テストの部品・`examples/`。
- できた文字の層は `Emo2Wiring` に入り（`mod.rs:633-641`）、`Emo2Wiring` は World の NonSend の置き場に載る（`mod.rs:631`）。この置き場は型ごとに 1 つで、入れ直すと前のものを置き換える。
- `present_frame` を本番で呼ぶのは `crates/areka/src/emo2_boot/frame/scale_text.rs:302` だけで、その `Emo2Wiring` の文字の層を渡す。
- `areka-emo-text` の中のテストと `tests/` は、120 か所の `present_frame(&mut rt, …)` も、残りの `present_frame(rt, world, …)` も、1 つのテストで 1 つの文字の層を 1 つの World に渡している（2 つの文字の層を同じ World で回すテストは見当たらない。完全な確かめは audit で全部読むときに行う）。

### 4.3 前提が崩れる所・崩れかける所

1. **ゴーストの切り替え**: 切り替えは同じ World で起動し直す（`crates/areka/src/emo2_boot/ghost_switch.rs:850` → `ghost_session.rs:817` の `boot_ghost_strict` → `ghost_session.rs:863` の `wire_emo2_boot`）。新しい文字の層ができても、World の印 `SeenGraphicsCore` は前の文字の層が置いたまま残る。新しい文字の層の初めての回は「印が有る」側の道を通り、装置が変わっていれば描き手を捨てる。このときまだ描き手は 0 なので、今は害が無い。印の説明（「最初に呼んだ回は印を控えるだけ」）とは食い違う。
2. **1 つの World に 2 つの文字の層**: 今の本番には無い。もし入れば、先に回った文字の層が印を書き換え、後の文字の層は「変わっていない」と読んで古い装置の描き手を使い続ける。複数のゴーストを同時に動かす `multi-ghost-runtime`（v0.1.0 の余力の枠・時期は未定）が同じ World に 2 つ目を置く形を取ると、ここで崩れる。
3. 型で守っていないので、崩れても番人のテストは赤にならない。

### 4.4 直し方の候補（audit の指摘になったとき・議題 3）

- **甲（推す）**: 印を World の資源から `TextLayerRuntime` の私的な欄（`Option<Tick>`）へ移す。文字の層ごとに自分の見た時刻を持つので、前提そのものが要らなくなる。切り替えの後も「初めての回は控えるだけ」が説明どおりになる。公開の形は変わらない（`crates/areka` に触れない）。差分は数行と、壊れたら落ちるテスト 1 本（同じ World に 2 つの文字の層を置き、装置を作り直すと両方とも描き手を作り直す）。
- **乙**: 今の形のまま、説明に「本番で 1 つになる理由」（4.2）と切り替えの残り（4.3 の 1）を書き足す。コードは変えない。2 つ目の文字の層を置く spec が来たときに直す。
- **丙**: 2 つ目の文字の層が来たら `debug_assert!` で落とす。落ちる道を足すので、記録の無い失敗の道を作らない決まり（要件 6.4）と合わない。推さない。

## 5. 要件 7（持ち込みの件）: 箱の位置の足し算があふれる

### 5.1 今のコード

- `src/actor_box.rs:236-237`（`refresh_shown_boxes`・箱の当たりの矩形）: `right: at(site.x + i64::from(def.size.0))`・`bottom: at(site.y + i64::from(def.size.1))`。`site.x`・`site.y` は `i64`（`crates/areka-emo-compose/src/boxes.rs:54-55` の `BoxPlacement`）、大きさは `u32`。
- `src/actor_box.rs:427`（箱がサーフェスの画像からはみ出すかの判断）: `site.x < 0 || site.y < 0 || site.x + w > sw || site.y + h > sh`。負の値は `||` の手前の比べで先に真になるので、あふれるのは大きな正の値のときだけ。
- `elementN,balloon,名前,9223372036854775807,0` では、debug は足し算で panic し、release は負へ折り返す（236 は矩形が裏返る・427 は「はみ出していない」と誤って読み、`warn!` が出ない）。
- 壊れたら落ちるテストは無い（このクレートに `i64::MAX`・`9223372036854775807` を使うテストは 0）。
- 兄弟の直し: `audit-emo-compose` は同じ形を `saturating_add` で直した（`.kiro/specs/completed/audit-emo-compose/research.md` 15 節の 31・テストは `crates/areka-emo-compose/src/plan_nesting_tests.rs:402` から）。

### 5.2 直し

- 3 か所とも `saturating_add`（要件 7.1 の「上限で止める・折り返さない」）。427 は `site.x.saturating_add(w) > sw` で、上限で止まった値はサーフェスの幅より大きいので「はみ出す」と正しく読み、`warn!` が出る。
- テスト: 兄弟のテストの形に倣い、`actor_box_tests.rs`（251 行）か `actor_box_present_tests.rs`（345 行）に、`i64::MAX` の置き場所で panic せず、矩形が裏返らず、はみ出しの `warn!` が 1 回出ることを確かめる 1〜2 本を足す。どちらのファイルも上限まで余裕がある。
- 近くの気になる所（audit で確かめる・ここでは指摘に数えない）: `actor_box.rs:197` の `box_origin` は `site.x as f32` で、`i64::MAX` は約 9.2e18 の f32 になり、面の装着位置と選択肢の当たり行へ足される。panic はしないが、窓の物理座標としては意味の無い値になる。要件 7.2 が名指すのは 236-237 と 427 の 2 つの足し算だけ。

## 6. 要件と手持ちの対応（Requirement-to-Asset Map）

| 要件 | 手持ち | ギャップ |
|---|---|---|
| 1 洗い出し | 範囲の 121 ファイルの一覧と読む手分けの案（2 節）・要件 1.5 の答え（4 節） | **Missing**: audit の指摘の一覧（この回ではしていない・議題 1） |
| 2 全部直す | — | 指摘の一覧しだい |
| 3 残す裁定 | 先送りの印の候補 3 つ（2 節） | **Unknown**: SSP 互換・ukadoc の正典に関わる指摘があるか（選択肢・スクロール・箱は ukadoc に書かれた振る舞いを持つ） |
| 4 直した後 | 全体テスト `tools/test-all.ps1`・`log-capture-kit`（`warn!` を数える）・実 GPU のテストの足場（`actor_test_support.rs` の `com_world` など） | **Missing**: 要件 7 の壊れたら落ちるテスト（5 節） |
| 5 範囲の外 | 範囲の外から使われる公開の形の一覧（3 節） | **Constraint**: `crates/areka/` に触らない・公開の形を変えない（ウェーブ C18 の約束 ⑴） |
| 6 守る決まり | 依存の追加は今のところ要らない見込み（5 節の直しは標準の関数だけ） | **Constraint**: `surface.rs` が 995 行＝そこを直すと新しいファイルが要る（8 節の 2） |
| 7 持ち込み | 3 か所の場所と直し方（5 節） | **Missing**: テスト |
| 8 大きさ | 兄弟の実績（7 節） | **Unknown**: 指摘の数 |

## 7. タスクの数の見込みと切り方（要件 8.1）

| 兄弟の見直し | 範囲の大きさ | 指摘の数 | タスク | 切ったか |
|---|---|---|---|---|
| `audit-emo-compose` | 97 ファイル | 20（1 回目） | 40 | 切らない（要件の討議で「1 本のまま」） |
| `audit-parsers-sakura` | 116 ファイル・コード 7,842・テスト 19,202 | 93 | 27 | 切らない |
| `audit-wintf-runtime` | 160 ファイル | 35 | 28 | 切らない（開発者「1 本のまま」） |
| `audit-wintf-graphics-world` | — | — | 23 | 切らない |
| `audit-seriko` | 60 ファイル前後 | 少ない（バグ 0） | 18 | 切らない |

- この範囲は兄弟の中で大きい側（45,385 行・テストが 34,553 行）。テストの写し・失敗しえないテスト・名前と違うことを確かめるテストの指摘が多く出やすい（兄弟の `audit-emo-compose` の 10〜14 の型）。直しは 18〜30 タスクの見込み。
- 案: **切らない**（推す）。兄弟は 20 を越えても 1 本のまま進め、開発者もそれを選んだ。切ると同じクレートの同じファイルに 2 本が順に触ることになり、`lib.rs` の 1 本だけの席（新しいファイル）の取り合いも生まれる。**組で切る**（必ず直す＋直したほうがよい／できれば）は、指摘が 40 件を大きく越えたときだけの案として残す。決めるのは指摘の一覧が出た後（議題 2）。

## 8. 守る約束と気をつける所

1. **`crates/areka/` に触らない・公開の形を変えない**（ウェーブ C18 の約束 ⑴）: 3 節の公開の形を変える直しは、要件 5.2 のとおり議題にする。テストの部品の形（`state_test_support.rs`・`actor_test_support.rs`）は `#[cfg(test)]` の中なので、`crates/areka` のテストからは見えない（変えてよい）。
2. **新しいソースファイルの席**: このウェーブで emo-text にファイルを足せるのはこの spec だけ（約束 ⑷）。足したら `lib.rs` の `PURE_SOURCES`（`lib.rs:193`・母数 83 を `lib.rs:502` で固定）か `SOURCES_OUTSIDE_THE_PURE_SCAN`（`lib.rs:427`）に載せる（要件 6.3）。`audit-emo-text-layout-draw` はファイルを足さないが、消すことはありうる＝母数の行が動くときは申し合わせる。足したファイルが `.kiro/steering/structure.md` の emo-text の主要ファイルの一覧（337 行から）に当たるなら一覧も直す。`structure.md` は同じウェーブの `placement-crate-split` も直す（別の節）＝触る前にそのセッションへ知らせる。
3. **`audit-emo-text-layout-draw` との境**: 受け持つファイルの重なりは 0。こちらの範囲から向こうの関数を呼ぶ所（例: `LayoutEngine::visible_window` を `scroll_marker.rs:227` が呼ぶ）と、向こうがこちらの型を使う所は、形を変える前に申し合わせる（要件 5.4）。向こうの持ち込み（`layout_scan.rs` の改行の送りに有限かの検査が無い件）は向こうの持ち物で、こちらは触らない。
4. **同じウェーブの相手が変えない形**: `ecs::graphics::Visual`・`VisualGraphics`（`wintf-visual-tree`・この範囲の約 20 ファイルが使う）、`Arrangement`・`HitTest`・`AlphaMaskResource`・`LayoutScale`・`AlphaMask`（`audit-wintf-layout-widget`・`surface.rs`・`surface_window_child.rs`・`marker_surface.rs` が使う）、`dola::cue::CueSink`（`audit-dola`・`sink.rs`）、`areka-actor` の公開の形（`audit-sylphya-actor`）。こちらはそれらの形を前提にしてよい。
5. **`doc/COMPAT_ARCHITECTURE.md`**: この範囲のコメントがこの文書を名指す。改名の spec（`doc-compat-architecture-rename`）は同じウェーブに無い＝名指しはそのままにし、名前を変えない。
6. **物差しの版**: 利用者全体に入っている ponytail は 2026-11-06 に 5.3.3 へ上がった（`~/.claude/plugins/installed_plugins.json`）。`skills/ponytail-audit/SKILL.md` は 5.3.2 と 1 字も違わない（比べて確かめた）＝要件の「5.3.2」の物差しと同じ。外す作法の 2 つ（「最大 20 件」＝同ファイル 84 行・「Not checked」＝同 98 行）も同じ場所にある。
7. **実 GPU のテスト**: `actor_*_present_tests.rs`・`marker_surface_tests.rs`・`surface_*_tests.rs`・`tests/*_readback_test.rs` などは実 GPU で回る。直しの確かめで重い走行を何度も回さず、2 回目からは赤が出たテストに絞る。

## 9. 決めごと（要件の討議へ）

1. **audit の全件の洗い出しをいつするか**（答えで作業が変わる）: 要件 1.1 はギャップ分析で audit をかけると定めるが、この回はしていない。
   - 推す: **要件の討議の前に、作法の 2 つを外した audit をこの範囲にかけ、指摘の全件をこの文書の次の節に書き足す**（兄弟 7 本と同じ形。読む手分けは 2 節の案）。指摘が無いと、要件の討議で直す対象を確定できない（要件 1.6）。
   - ほか: 要件の討議を先に済ませ、audit は設計の段の頭でかける。この場合、要件 1.1 の「ギャップ分析で」を改め、要件の討議をもう一度開く手間が増える。
2. **2 本に切るか**（要件 8.1・答えで作業が変わる）: 推す＝**切らない**（7 節）。指摘が 40 件を大きく越えたら、組で切る案を改めて出す。
3. **要件 1.5 の前提の扱い**（答えで作業が変わる・小さい）: 推す＝**4.4 の甲**（印を文字の層の私的な欄へ移し、前提を要らなくする・公開の形は変わらない・テスト 1 本）。audit が同じ所を指せばその指摘として、指さなければ要件 1.5 の答えとして直す。乙（説明だけ直す）を選べば、`multi-ghost-runtime` の brief に「印を文字の層ごとにする」を申し送る。
4. **持ち込みの件の直す範囲**（答えで作業が少し変わる）: 推す＝**要件 7.2 の 3 か所（236・237・427）を `saturating_add` で直し、テストを 1〜2 本**。`actor_box.rs:197` の大きな値の f32 は audit の判断に任せる（指されたら同じタスクで直す）。
5. **公開の形を変えないと直せない指摘が出たとき**（要件 5.2・答えで作業が変わる）: 推す＝**このウェーブでは「足すだけ」の形で直す**（古い口を残して新しい口を足し、呼び手の書き換えは `crates/areka` の見直しの spec へ申し送る）。足すだけでも直らないものは、残すか後の spec へ回すかを 1 件ずつ議題にする。

## 10. 手間と危うさ

- **手間**: M〜L（3 日〜2 週）。読む量が 45,000 行あり、テストの直しの量で決まる。持ち込みの件と要件 1.5 の直しは S。
- **危うさ**: Medium。`crates/areka` の公開の形を変えられない（直しの形が縛られる）・`surface.rs` が上限の 5 行手前・実 GPU のテストが多く走行が重い。持ち込みの件と要件 1.5 は既存の形の中の直しで Low。

## 11. 直し方の選択肢（指摘が出た後の全体の進め方）

- **案 A（今のファイルの中で直す）**: 直しは今のファイルの中で閉じ、新しいファイルは上限を越えるときだけ（`surface.rs` など）。✅ 差分が小さく、`audit-emo-text-layout-draw` と重ならない。❌ 上限の近い 3 ファイルを直すと、結局ファイルを分けることになる。
- **案 B（テストの部品を先にまとめてから直す）**: テストの写しの指摘が多いとき、部品（`actor_test_support.rs`・`state_test_support.rs` と各テストの中の写し）を先に 1 か所へまとめ、その後で個々のテストを直す。✅ 写しを直す手間が 1 度で済む（兄弟の `audit-emo-compose` の 12 を先に置いた並びと同じ）。❌ 1 タスク目の差分が大きい。
- **案 C（混ぜる・推す見込み）**: 必ず直す（持ち込みの件・要件 1.5 を含む）を案 A で先に入れ、テストの整理は案 B の並びで後に置く。どれを取るかは指摘の一覧を見て設計の段で決める。

## 12. 調べが要る所（Research Needed）

1. audit の全件（議題 1）。
2. 選択肢・スクロール・箱の指摘のうち、SSP 互換・ukadoc の正典に関わるもの（要件 3.2・指摘が出た後に ukadoc で引く）。
3. `multi-ghost-runtime` が 2 つ目のゴーストを同じ World に置く形か（4.3 の 2・その spec の brief で確かめる。議題 3 で甲を取れば要らない）。
