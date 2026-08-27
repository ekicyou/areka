# ギャップ分析: areka-P0-balloon-vertical-canon

- 採取日: 2026-08-27（ブランチ `claude/areka-p0-balloon-vertical-a522c6`・HEAD `19455240`）
- 対象: 確定済み `requirements.md`（Requirement 1〜12・SC1〜SC15・要件段階の裁定 5 点）と既存コードベースの差分
- 方法: `crates/` 全域の実読（Grep/Read）。**本書の file:line はすべて本日再検証したもの**で、`brief.md` の行番号を転記していない。ukadoc-mcp は陳腐化の確認にのみ用い、正典の裏取りには用いていない。
- 位置づけ: **情報であって決定ではない。** 選択肢と判断材料を示し、裁定は要件ディスカッションへ委ねる。

---

## 1. 分析サマリ

- **受口（Requirement 1）は要件の見立てどおり安い。** バルーン解析の 2 層マージはキー非依存の `BTreeMap` 上書きで、`vertical` は**無改造で後勝ちが成立する**。網羅的なキー表も件数定数も存在せず、更新を強いる檻が無い。「未宣言と宣言の区別」は crate 全体の `Option` 規律で既に成立しており、`0/1` の生値を下流で検証する先例（`WindowPositionRaw::limit_raw`）まで揃っている。
- **座標意味論（Requirement 3）は「一致の固定」だけでは済まない。** 要件が列挙した 7 点は確かに一致しているが、要件が触れていない **areka 独自の「origin クランプ正準」**（宣言済みでも validrect 外の origin 成分を書字開始角へ寄せる）が正典に無い規約として存在し、**既存の縦書きフィクスチャはまさにその規約に依存して成立している**。Requirement 3.9 を字義どおり適用すると既存テストとフィクスチャ表示が壊れる。
- **`\_l` 是正（Requirement 4）は要件の見た目より広い。** 原点・符号の不一致は確認できた（ただし `vertical_rl` 限定・`vertical_lr` は既に正典と一致）。しかしそれ以前に、**負値絶対座標は現行が構造的に拒否**しており（`value >= 0.0` ゲート＋縮退 4 分岐の warn-once）、**`@` 相対は未実装**である。要件 4.3 と 4.6 は、完了 spec `emo-text-layer` が確定した縮退表の改訂を伴う。
- **プロパティ（Requirement 7）は「最小追加」ではなく「`currentghost` 枝の最初の実導出」。** sylphya は設計として **M1 は `baseware.*` のみ実導出・他ルート枝は NOT_FOUND 縮退**と宣言している。機構自体は軽い（`scope(N)` 構文は既存・解決は正準文字列の map 参照・値なしは既定で `NotFound`）が、**本番構成に読み口が無い**——emo2 は 32bit helper 経由で走り、host32 の 3 crate に property の語が 1 件も無い。
- **縮退登記の受け皿（Requirement 5/8）は「実在」だけでは要件を満たさない。** 候補 `balloon-canon-residue` は実在するが、**自分自身が「項目を列挙していない spec は所有者ではない」と規律を書いており**、文字装飾・下線・プロパティ族は 1 件も列挙されていない。受け皿側 brief への項目追記か新設かの裁定が要る。

---

## 2. 要件 → 資産マップ

タグ: **一致**（既存実装が正典と合致・固定するだけ）／**追加**（新規実装）／**是正**（既存と正典が食い違う）／**衝突**（要件どうし・要件と既存正典が矛盾）／**未確定**（設計段階の裁定が要る）

| 要件 | 主要資産（本日実測） | タグ | 備考 |
|---|---|---|---|
| R1.1–1.3 受理 | `areka-parsers/src/balloon/parse.rs:108-113`（`writing_mode` の隣）／`writing.rs:63-77` | 追加 | 2 行程度 |
| R1.4 未宣言と宣言の区別 | `balloon/model.rs:9-11`・`:203-226`（`limit_raw` 先例）／`model_tests.rs:36`・`:206` | 一致 | crate 規約として既に成立 |
| R1.5 2 層マージ後勝ち | `parse.rs:44-54`（`descript.clone()` へ image を `insert`） | 一致 | **キー非依存ゆえ無改造で成立** |
| R1.6–1.7 不正値の警告＋縮退 | `parse.rs:76-78`・`parse_tests.rs:284-287`（「解釈せず・警告せず」を固定） | **衝突** | §3.2 参照。主語が「解析」だと既存契約に反する |
| R1.8 既存キー不変 | 網羅 match も件数定数も無い | 一致 | 檻の更新は不要 |
| R2 共存規則 | `writing.rs:63-77`（1 入力の全域 match） | 追加 | 2 入力化＋優先順位＋警告 |
| R3.1–3.2 `origin.x`＝1 列目右端 | `region.rs:211-229`（VerticalRl 開始角＝`(right, top)`） | 一致＋**衝突** | クランプ正準（§3.3） |
| R3.3 `origin.y` 既定 `validrect.top` | `region.rs:198`・`:212-215` | 一致 | SC6 の仮定と合致 |
| R3.4 折返し＝`wordwrappoint.y` | `region.rs:231-239`（縦は `.y` のみ・網羅 match）・既定 `bottom` | 一致 | |
| R3.5 `wordwrappoint.x` 不参照 | 同上（match の縦書き腕に `.x` が現れない） | 一致 | 型で保証 |
| R3.6 `validrect` 不変 | `region.rs:196-200`／`layout_visible_window_tests.rs:60-79` | 一致 | **SC5 の「仮定」は既に実装済み**（§3.4） |
| R3.7 負値＝反対端基準 | `region.rs:282-286`（`resolve_coord`・`extent` は画像寸） | 一致 | |
| R3.8 決定論テストで固定 | `region.rs:562`・`:573`・`:597`・`:612`・`:624` ほか | 一部あり | 追補は容易 |
| R3.9 不一致は正典へ是正 | — | **衝突** | クランプ正準を撤去する読みになる |
| R4.1–4.4 `\_l` 軸・原点・符号 | `layout.rs:449-478`・`:305-311`・`:611-621` | 是正 | **`vertical_rl` のみ**・`vertical_lr` は一致 |
| R4.3 負 X で列送り | `layout.rs:658`（`value >= 0.0` ゲート）・`:679`（`NegativeAbsolute`） | **是正（範囲拡大）** | 完了 spec の縮退表を改訂する |
| R4.5 `em`／`lh` の軸整合 | `layout.rs:453-454`・`:650-671` | 未確定 | 両軸へ同じ `(font_height, pitch)` を渡す（§3.5(d)） |
| R4.6 `@` 相対 | `layout.rs:668-669`・`:684`（`Relative` は未実装縮退） | **衝突** | 存在しない実装への規定 |
| R4.7 決定論テスト | `layout_cursor_tests.rs`（670 行・13 本・**22 箇所すべて `HorizontalTb`**） | 追加 | 縦書き `\_l` の被覆は 0 |
| R5 語彙の完全保持＋縮退登記 | `\f[align]`／`\f[valign]`／下線＝**workspace 全域に実装 0**（`valign` はヒット 0） | 追加（登記のみ） | 受け皿は §3.7 |
| R6.1–6.4 フォント縦書き異体 | `draw.rs:253-274`（`DirectionRecipe::for_mode`）・`:302-331`（唯一の format 工場）・`:387-393`／`:850-873`（計測と描画が同じ工場） | 一致 | R6.4 は構造的に成立 |
| R6.5–6.6 登記とテスト | `draw_format_metrics_tests.rs:193-202` | 一部あり | |
| R7 `.vertical` プロパティ | sylphya `vocab/dotted.rs:3-5`・`reader.rs:127-146`・`key.rs:129-141` | **追加（境界越え）** | §3.6 |
| R8 同族の縮退登記 | `currentghost.balloon.*` は 0 件（本日 grep 再確認） | 追加（登記のみ） | 受け皿は §3.7 |
| R9 面別上書きの適用範囲 | `emo2_boot/assets.rs:112-121`（`BalloonScopeAssets.model`）・`frame/wiring.rs:91` | 一致 | 起動時 1 回解決は実形どおり |
| R10 テストとフィクスチャ | `tests/vertical_fixture_test.rs`（151 行・4 本）・`examples/fixtures/emo2-vertical/` | 一部あり | §3.3 の制約に注意 |
| R11 COMPAT 登記 | `doc/COMPAT_ARCHITECTURE.md:122-175`（§8・48 行・テスト保護なし） | 追加 | §3.8 |
| R12 疑義の登記 | 同上 | 追加 | |

---

## 3. 個別のギャップ（重要度順）

### 3.1 受口は自然に空く——檻の更新を要さない

`parse.rs` は 161 行・関数 3 本で、キーごとの `merged.get("...")` が並ぶだけである。**網羅 match も `const` キー配列も件数定数も存在しない**ため、`vertical` の追加は既存の檻を 1 つも更新させない。非モデル化キー（`sstpmessage.*`／`number.*`／`arrow*`）は「モデルに accessor が無いから型で保証される」という散文と 2 本の否定テスト（`validation_tests.rs:185`・`parse_tests.rs:160`）で表現されており、こちらも更新不要である。

2 層マージ（`parse.rs:44-54`）は `descript.clone()` に image 側を `insert` するだけの**キー非依存**実装なので、**Requirement 1.5 は追加コード 0 で成立する**。

注意点は 1 つだけ——`BalloonModel::new` は 7 引数の位置引数で、workspace 内に約 26 の呼出箇所がある（大半は `areka-emo-text`）。過去 2 通りの先例がある。

- 署名を伸ばす（`budoux_newline` の例）→ 全呼出箇所へ `None` 追記が波及する。
- **additive ビルダーを足す**（`with_cursor`・`with_windowposition_raw` の例・`model.rs:56-59` が「互換な経路」と明記）→ 呼出箇所の変更 0。

### 3.2 警告の主語が既存構造と食い違う（Requirement 1.6／1.7／2.5）

要件は「**the areka バルーン定義の解析** shall 警告を記録したうえで…縮退し」と書くが、`areka-parsers` の実形はこれと反対である。

- `balloon/` は**無警告契約**を明文で持つ（`parse.rs:76-78`、`parse_tests.rs:284-287` が「語彙外の値も解釈せず・警告せず素通し」を固定）。
- crate 全体の `warn!` は **1 箇所のみ**（`package/resolve.rs:296`）。
- **`log-capture-kit` が dev-dependency に無い**（`areka-parsers/Cargo.toml` の dev-deps は `temp-path-kit` のみ）。したがって parser 層に警告を置くと、**Requirement 10 の決定論テストで観測できない**（依存を追加するか、観測を諦めるかになる）。

一方、同型の警告は既に下流に在り、観測手段も揃っている——`emo-text/writing.rs:70-73` の未知 `writing_mode` 警告と、`writing.rs:86`・`:107-110` の `count_levels` による件数固定である。

→ **判断事項**: 警告と縮退の主体を「解析」ではなく「書字方向の解決」（`writing.rs`）に置くと、既存の層規律・テスト資産の双方に乗る。要件の EARS 主語を改めるか、parser 層へ観測手段を新設するかの裁定が要る。

### 3.3 「origin クランプ正準」——要件に登記されていない areka 独自規約（最重要）

`region.rs:24-27` はモジュール doc で次を宣言する。

> `## origin クランプ正準（areka 独自・design.md が正典）`
> 描画開始点＝`clamp(resolve(origin), validrect)`。成分 `None` または validrect 外は書字開始角（horizontal_tb/vertical_lr＝validrect 左上・vertical_rl＝右上）へ寄せる。

実装は `clamp_origin_component`（`region.rs:302-331`）で、**宣言済みの値であっても** validrect の外なら書字開始角へ寄せ、`debug!` を残す。SSP 正典にこの規約は無い（正典は「`origin.x`＝1 列目の右端」とだけ述べる）。

**これが効いている実例が、まさに本仕様が使うフィクスチャである。**

`examples/fixtures/emo2-vertical/descript.txt:15-16` は `origin.x,0` / `origin.y,0` を**宣言**している。2 層マージ後の validrect は `(36, 46, 356, 168)` で、`(0,0)` はその外にある。したがってクランプが効き、`tests/vertical_fixture_test.rs:117` が

```rust
assert_eq!(region.start(), (356.0, 46.0));
```

を逐語で固定している。**クランプ正準を撤去して SSP 正典に厳密化すると、1 列目の右端は x=0 となり、`finish_line` の VerticalRl 腕（`layout.rs:611-621`＝列矩形は `[block_pos - font_height, block_pos]`）により列はバルーン画像の外へ出る。** 当該テストは赤になり、フィクスチャの縦書き表示は壊れる。

Requirement 3.9（「1.〜7. のいずれかが現行実装と食い違うと判明したとき、正典側へ実装を是正する」）を字義どおり読むとこの撤去に至る。要件本文は R3.1〜R3.7 のいずれにもクランプ正準を書いていないため、**この規約が「維持される裁量」なのか「是正対象の食い違い」なのかが要件から読めない。**

→ **判断事項**（討議必須）。維持を採る場合、Requirement 3 に「宣言済み origin の validrect 外クランプは areka 裁量として維持する」旨を明示し、COMPAT §8 へ 1 行足すのが筋（Requirement 11 の範囲拡大）。

### 3.4 SC5 の「仮定」は既に実装済みである

要件 SC5 は「縦書きで列が並ぶ範囲の上限を定めるキーが無い。**areka は `validrect.left` を上限と仮定する**が、これは推定であって正典の記述ではない」と書く。実測では**仮定ではなく既存挙動**である。

`layout_visible_window_tests.rs:60-79`（`vertical_rl_overflow_scrolls_content_rightward`）は、validrect left=360/right=400 で列の左端が 390/377/364/351 と進み、**4 列目 351 < 360 であふれを発火**させ、内容を右へオフセットする（横スクロール）ことを固定している。`vertical_lr` は鏡像で `validrect.right` があふれ判定になる（同 `:84-`）。

→ SC5 に対応する areka の裁量は「これから採る仮定」ではなく「**既に実装され、決定論テストで固定されている挙動**」として登記できる。Requirement 3.6 と SC5 の対応表（要件 :273）の文言を実形に合わせる余地がある。

### 3.5 Requirement 4（`\_l`）——4 つの独立した障害

**(a) 原点と符号の不一致は確認。ただし `vertical_rl` に限られる。**

- `layout.rs:453-454`: `x_val = cursor_to_image_px(x, region.left(), ...)` / `y_val = cursor_to_image_px(y, region.top(), ...)`。**X の原点は常に `region.left()`・増加方向**。
- `layout.rs:305-311`（軸読み替え正準表）: `VerticalRl => (start.1, start.0, -1.0)`——列送り軸の起点は `start.0`（既定 `validrect.right`）で**方向は −1**。
- `layout.rs:611-621`（`finish_line`）: VerticalRl の列矩形は `left: block_pos - font_height, right: block_pos`＝`block_pos` は列の**右端**。

したがって `\_l[0,0]` は `block_pos = region.left()` を与え、列矩形は `[left - font_height, left]`＝**描画範囲の外側左方**になる。フィクスチャ値（validrect 36..356・font 28px）では x∈[8,36] に列が立ち、自然な 1 列目は x=356 にある。**要件 4 の「裁定 2」の記述は正しい。**

一方 `VerticalLr` は `(start.1, start.0, +1.0)` かつ列矩形が `[block_pos, block_pos + font_height]` なので、`\_l[0,0]` は自然な 1 列目の起点と**厳密に一致する**。是正対象は `vertical_rl` だけである（要件は「縦書き」と一括で書いており、この非対称に触れていない）。

**(b) 負値絶対座標は現行が構造的に拒否する。**

```rust
CursorCoord::Absolute { value, unit } if value >= 0.0 => { ... }
_ => None,   // 負値絶対・Relative(@)・Invalid・Omitted
```
（`layout.rs:656-670`）

非負ゲートを外れた負値は `CursorDegrade::NegativeAbsolute`（`layout.rs:679`）として actor ごと warn-once され、**当該軸は不動**になる。Requirement 4.3（「X に負値が与えられたとき左＝次の列の方向へ移動する」）は、この 4 分岐縮退表——完了 spec `areka-P0-emo-text-layer` の R2.4／6.5 が確定し `CursorWarnGuard` が檻にしている——**を改訂しないと成立しない**。完了 spec の正典を下流 spec が書き換える形になるため、記憶「裁定で要件を改訂したら design・境界節・steering まで追随」および「並走ブランチは実装済みコードの正典性も陳腐化させる」の規律に触れる。

**(c) `@` 相対は未実装である。** `CursorCoord::Relative` は `_ => None` に落ち、`CursorDegrade::Relative`（`layout.rs:684`）として M1 縮退保持されている。Requirement 4.6（「相対指定であるとき 1.〜5. と同一の軸規約で解決する」）は、存在しない実装に対する規定である。①縮退経路への規定（＝実質空文）と読むか、②`@` の実装を範囲に取り込むか、で仕事量が大きく変わる。

**(d) `em`／`lh` の軸整合（4.5）は現行でも成立しうるが、拒否はしない。** `cursor_to_image_px` は `Em => font_height` / `Lh => line_pitch` を掛けるだけで、呼び手は**両軸に同じ `(font_height, pitch)` を渡す**（`layout.rs:453-454`）。縦書きでは y→行内・x→列送りへ写るので、`em` を y に・`lh` を x に書けば要件どおりの軸に着地する。ただし軸に合わない単位（列送り軸への `em` など）は拒否も再写像もされず、単に係数として掛かる。要件 4.5 の「一致させる」が①既定の写像が正しければ足りるのか、②不一致を検出して縮退させるのか、が未確定。

**(e) 縦書き `\_l` のテスト被覆は 0。** `layout_cursor_tests.rs`（670 行・13 本）は `WritingMode::` の 22 箇所すべてが `HorizontalTb` である。要件 4.7 の指摘は実測どおりで、**この空白がまさに (a) の欠陥が今日まで残った理由**である。

### 3.6 Requirement 7——sylphya の M1 設計境界を越える

**設計上の位置づけ**: `vocab/dotted.rs:3-5` は「M1 は `baseware.*` のみ実導出し、他ルート枝配下は NOT_FOUND へ縮退する（R5.2・backing 登録だけで実導出化できる差替シーム付き）」と宣言する。本番で実際に publish されている点付きキーは `baseware.name`／`baseware.version`（`areka-ghost/src/sylphya_wiring.rs:125-129`）と `areka.*` の persist キーのみ。`currentghost.*` は**ルート枝名 1 件を除いて 0 件**（`vocab/dotted.rs:22`）。つまり `.vertical` は「最小追加」ではなく **`currentghost` 枝の最初の実導出**である。

**追い風（機構は軽い）**

- `scope(N)` の構文は既存（`key.rs:129-141`・`Selector::ByIndex`）。`areka.window.scope(N).x`／`areka.balloon.offset.scope(N).x|y` が実運用中（`persist/mod.rs:150-161`）。
- 解決はトレイトも registry も無く、**正準文字列の map 参照 1 本**（`reader.rs:127-146`）。掲示板（マテリアライズド・ビュー）へ publish するだけで値が立つ。
- 値なしは `DottedResolution::NotFound`（`value.rs:33-39`）→ 消費側は `SHIORI_E_PROPERTY_NOT_FOUND` を返し `out_value` を書かない（`areka/src/shiori_host.rs:247-265`）。**Requirement 7.4 と 8.5（捏造しない）は既定で成立している。**
- SET は `currentghost` が `DOTTED_ROOTS` に在る帰結として `NotSettable` へ自動分類される（`actor.rs:136-166`）＝読み取り専用が無償で付く。
- 台帳の件数錠（`DOTTED_ROOTS` 10／`GENERIC_PROP_NAMES` 17／`SET_EFFECTIVE` 21／`FLAT_VOCAB` 26／`SHIORI_RESOURCE_IDS` 159・集約は `ledger_key_determinism_tests.rs:199-205`）は、**`vertical` を `GENERIC_PROP_NAMES` に登録しなければ 1 つも更新不要**（GET 経路は台帳を読まない）。登録する場合はデータ・件数・逐語配列・集約テストの 4 箇所。

**向かい風**

1. **セレクタで分岐する解決は存在しない。** `resolve_dotted` は `PropPath` を即座に正準文字列へ畳んで map 参照する（`reader.rs:80-83`）。`Selector::ByIndex` は解決時に一度も分解されない。したがって `scope(ID)` は「**ID ごとに 1 行 publish 済み**」を意味し、scope 集合の列挙が要る（`areka.balloon.offset.scope(N)` と同じ形）。
2. **publish の縫い目は 1 箇所しかない。** 走査対象ごとの `BalloonModel` は `BootAssets.balloons`（`emo2_boot/assets.rs:112-121`・`model` は `:120`）に在り、その後 `Emo2Wiring.balloon_models`（`frame/wiring.rs:91`）へ移る。後者は **UI スレッドの NonSend リソース**で sylphya アクタからは触れない。`balloons` が生きていて `GhostRuntime::sylphya_publisher()` も取れる区間は `emo2_boot/mod.rs:430-465` のみ。sylphya は `areka-parsers`／`areka-emo-text` へ依存できない（最下層規律）ので、`WritingMode::resolve` は `areka` bin 側で呼んで**文字列で渡す**形になる。
3. **本番構成に読み口が無い（最大の未確定）。** emo2 は `ShioriWiring::Helper`（`emo2_boot/mod.rs:417-419`）で走る。**host32 の 3 crate（`shiori-host32-ipc`／`-host`／`-helper`）の src に "property" の語は 1 件も無い**（本日 grep で確認・大文字小文字無視で 0 ヒット）。`IShioriHost` 側には `GetProperty`／`SetProperty` が在る（`shiori-abi/src/interface.rs:153`・`:159`）が、32bit ヘルパの IPC はそれを運んでいない。加えて ghost 側の in-proc host は sylphya 非接続の `RefCell<HashMap>`（`areka-ghost/src/shiori_inproc.rs:238-242`）、`areka` bin の `ShioriSession` は**別のアクターを spawn**する（`shiori_session.rs:141`）。
   → **Requirement 7.1「ゴーストがプロパティを照会したとき」を本番構成で満たすには、読み口の裁定が先に要る。** publish 側だけなら決定論テストで固定できるが、それは「照会できる」ことの証明にならない（記憶「檻は前提を自前で作る」）。

### 3.7 Requirement 5.5／8.4 の「追跡先」——実在確認だけでは規律を満たさない

候補 `.kiro/specs/areka-P0-balloon-canon-residue` は**実在する**（`brief.md` のみ・76 行）。ただし収載範囲は**項目列挙**で定義されており、2 系統だけである。

- `kero-balloon` 由来＝バルーン**系列解決**の残余（項目 1〜6・`:11-16`）
- `balloon-visibility` 由来＝バルーン**表示寿命**の残余（項目 7〜10・`:24-27`）

そして同 brief `:18` は自ら次の規律を書いている——**「受け側が項目を列挙していない spec は所有者ではない」**。したがって暗黙には何も着地しない。

- `\f[align]`／`\f[valign]`／下線: 実装は workspace 全域に存在しない（`valign` は `.rs` でヒット 0・`align` は DirectWrite 定数のみ・`TextItem` は `Glyph`／`LineBreak`／`CursorMove` の 3 種のみ）。最も近い予約シームは `canvas.rs:27-32`（「`\f` 系文字装飾…も同シームに属する」）と `draw.rs:134-144`（`disable.font.` 予約）。residue の 2 系統のいずれにも属さない**第 3 の軸**である。
- `arrow0`／`arrow1`: residue 項目 1 が `arrow*` を既に持つが、それは**系列解決の軸**（scope 別接頭辞の連鎖）であって、縦書きでスクロール方向が右／左に変わるという再解釈の軸ではない。
- `currentghost.balloon.scope(ID).*` 族: residue brief にプロパティ系の語は**0 回**。repo の先例はむしろ「各 spec が自分の Out 節でプロパティ族を先送りする」形である（`scope-zorder-pinning/brief.md:48` が `currentghost.seriko.zorder` に対してまさにこれを行っている）。

→ **判断事項**: ⑴ residue brief へ項目 11 以降を追記して所有させる ⑵ 本仕様の Out 節で先送りし COMPAT §8 に登記のみ残す（zsp と同形）⑶ 新規の受け皿を起票する、の 3 択。Requirement 8.4 の「実在することを確認したうえで記録する」は**実在確認では足りず、双方向の登記が要る**。

### 3.8 doc 側の 2 つの齟齬

- **`writing_mode` が `doc/` に 0 件という要件の主張は正しい**（本日 workspace 全域で再確認）。ヒットは `.kiro/specs/`（本仕様＋完了済み 5 spec）・`.kiro/steering/`・`crates/` のみ。
- `doc/COMPAT_ARCHITECTURE.md` §8 は `:122-175`・データ行 48・列は `| 項目 | 裁量 | 根拠 | 出典 spec |`。**テストによる保護は無い**（`include_str!` 0 件・参照は 8 本の doc コメントのみ）。同ウェーブの `scope-zorder-pinning` も §8 追記を予定している（`brief.md:47`）ため、行レベルの隣接が 1 件増える（機械衝突は隣接行マージのみ）。
- **`doc/emo2-conformance-scope.md` と本仕様の文面が食い違う。** `:61`「budoux/縦書きは**痕跡なし・M1 不要**」、`:85` は縦書きを M2 へ後ろ倒しと書く。さらに `:60` は `\f[]` を「M1 不要」に列挙している。本仕様は M1 ウェーブ（W6.95）で縦書きの正典化を行うため、この doc は本仕様に対して陳腐化する。Requirement 11 は §8 のみを対象にしており、この doc の訂正の要否が要件から読めない（同 doc もテスト保護なし）。

### 3.9 ukadoc-mcp 陳腐化の独立再現（要件 :27 の裏取り）

本日 MCP を引いた結果、`currentghost.balloon.scope(ID).vertical` の記述は次のとおりで、**2.8.80 の旧意味論**である。

> 縦書きの場合、validwidth・validheight・lines は「画面上の向き」ではなく「文字の送り方向を基準にした向き」の値になり、それぞれ「1列の長さ」「列が進む方向に使える幅」「収まる列数」を示す。

要件 :174 が記す 2.8.83 現行（`validwidth`＝列が並ぶ方向の幅／`validheight`＝1 列の長さ）と役割が逆であり、**要件の主張は独立に再現された**。

一方、**座標側のスナップショットは陳腐化していない**——`origin.x`（「縦書きの場合は 1 列目の右端」・既定「横書き=validrect.left 縦書き=validrect.right」）・`wordwrappoint.x`（「横書きのときのみ使用」）・`wordwrappoint.y`（「縦書きのときのみ使用」・既定 `validrect.bottom`）はいずれも現行と一致する。**陳腐化はプロパティ節に限局している**——これは Requirement 11.7 の登記文をより正確に書ける材料である（「スナップショット全体が古い」ではなく「プロパティ意味論だけが逆」）。

### 3.10 行数の余裕（1,000 行の機械番人）

`log-capture-kit/tests/file_length_guard_test.rs` が `crates/**/*.rs` を走査し、**1,000 行超で例外表に無いファイルを赤にする**（例外表 11 件・件数定数と逐語一致を要求・「今はもう超過していない」項目も赤）。本仕様が触る主要ファイルの現在値。

| ファイル | 行数 | 余裕 |
|---|---|---|
| `areka-emo-text/src/draw.rs` | **974** | **26 行** ← 要注意 |
| `areka-emo-text/src/layout.rs` | 764 | 236 |
| `areka-emo-text/src/region.rs` | 721 | 279 |
| `areka-emo-text/src/layout_cursor_tests.rs` | 670 | 330 |
| `areka-emo-text/src/writing.rs` | 224 | 776 |
| `areka-parsers/src/balloon/model.rs` | 496 | 504 |
| `areka-parsers/src/balloon/parse.rs` | 161 | 839 |
| `areka-sylphya/src/vocab/dotted.rs` | 289 | 711 |

`draw.rs` は 26 行しか余裕が無い。Requirement 6 の追補は兄弟テストファイル（`draw_format_metrics_tests.rs`／`draw_oracle_tests.rs`）へ置くのが安全である。なお `region.rs` と `writing.rs` は crate 内で例外的にインラインの `#[cfg(test)] mod tests` を保っている（他は兄弟ファイル分離）。

---

## 4. 実装方式の選択肢

### Option A: 既存機構への最小統合（受口拡張型）

`parse.rs` へ 1 行＋`with_vertical_raw` ビルダー、`WritingMode::resolve` を 2 入力化、`\_l` の列送り軸原点をモード依存化、publish を `emo2_boot/mod.rs` の 1 点に挿入、doc へ追記。新規ファイルはフィクスチャとテストのみ。

- ✅ 触るファイルが少なく、既存パターンにそのまま乗る。`BalloonModel::new` の 26 呼出箇所を壊さない。
- ✅ Requirement 1／3／9／10 の大半は追加コードがごく小さい。
- ❌ `WritingMode::resolve` に「2 キーのマージ確定 → 優先順位 → 警告」が一体で入り、`writing.rs` の単純な全域 match という読みやすさが失われる。
- ❌ `.vertical` の値の導出点（`emo2_boot`）と書字方向の解決点（`writing.rs`）が離れ、Requirement 7.2（「実際に適用されている書字方向から導く」）が**規約でしか担保されない**。

### Option B: 書字方向の解決を第一級の決定に格上げ

`writing.rs` の `resolve` を、`WritingMode` だけでなく**決定の記録**（採用したキー・両キーの解決値・警告理由・`vertical` 相当の 0/1）を返す純関数へ格上げする。`.vertical` プロパティも Requirement 5 の写像表も、この 1 つの決定点から引く。

- ✅ Requirement 2 の共存規則・Requirement 7.2 の「同じ決定から導く」・Requirement 7.3（`vertical_lr` も 1）が**構造で保証される**（規約ではなく型で）。
- ✅ 正典の再改訂（SC14）に対する追随点が単一に保たれる——Adjacent expectations が明示的に期待している性質。
- ✅ 警告の観測は既に `writing.rs` にある `count_levels` 資産へそのまま乗る（§3.2 の問題が消える）。
- ❌ 戻り値型が変わるため `resolve` の呼出箇所（本番 1・テスト 8）に波及する（ただし本番呼出は `actor.rs:153` の 1 箇所のみ＝波及は小さい）。
- ❌ `writing.rs` 224 行が 2 倍程度に育つ（上限には遠い）。

### Option C: ハイブリッド（段階化・ゲート付き）

危険度の異なる 4 つを段で切る。

| 段 | 範囲 | 危険度 | 依存 |
|---|---|---|---|
| ① 受口と共存 | R1／R2／R10.1–10.4／R11 の一部 | 低 | なし |
| ② 一致の固定 | R3（クランプ正準の裁定を含む）／R6 | 低〜中 | §3.3 の裁定 |
| ③ `\_l` 是正 | R4 | **中〜高** | 完了 spec の縮退表改訂（§3.5(b)(c)） |
| ④ プロパティ | R7／R8 | **未確定** | 読み口の裁定（§3.6-3） |
| ⑤ 登記 | R5／R11／R12 | 低 | 受け皿の裁定（§3.7） |

- ✅ ③ と ④ が持つ「他 spec の正典・本番経路」への波及を、①② の確実な着地から切り離せる。
- ✅ ④ が読み口の裁定で伸びても ①②③⑤ が人質にならない。
- ❌ 段の間で `writing.rs` を 2 度触ることになりうる（Option B を ① で先に入れれば回避可能）。
- ❌ 計画が最も込み入る。

**組合せの所見**: ① で Option B の形を先に作り、②③④⑤ をその上に積むと、Option A の弱点（決定点の分散）と Option C の弱点（同一ファイルの二度触り）が同時に消える。ただしこれは**推奨であって決定ではない**——設計段階で評価すること。

---

## 5. 規模と危険度

| 群 | 規模 | 危険度 | 一行の根拠 |
|---|---|---|---|
| R1 受口＋R10.1/10.4 フィクスチャ | **S** | **低** | マージがキー非依存・檻の更新 0・先例（`limit_raw`）あり |
| R2 共存規則＋R10.3 | **S** | **低** | 全域 match の 2 入力化・本番呼出は 1 箇所 |
| R3 一致の固定 | **S** | **中** | コードは一致済みだがクランプ正準の裁定が未定（§3.3）＝裁定次第で既存テストとフィクスチャに波及 |
| R4 `\_l` 是正 | **M** | **高** | 完了 spec が確定した縮退表（非負ゲート・4 分岐 warn-once）の改訂を伴い、被覆 0 の領域を新規に檻へ入れる |
| R5 語彙の縮退登記 | **S** | **中** | 文書だけだが受け皿の双方向登記が未確定（§3.7） |
| R6 フォント等価 | **S** | **低** | 計測と描画が単一 format 工場を共有＝R6.4 は構造的に成立済み |
| R7 プロパティ | **M** | **高** | `currentghost` 枝の初実導出＋**本番に読み口が無い**（§3.6-3）＝検証可能性そのものが未確定 |
| R8 同族の縮退登記 | **S** | **中** | 受け皿が R5 と同じ問題を共有 |
| R9 適用範囲の登記 | **S** | **低** | 起動時 1 回解決は実形どおり（`BalloonScopeAssets.model`） |
| R11／R12 COMPAT 登記 | **S** | **低** | §8 にテスト保護は無い・隣接 spec との行衝突のみ |
| **合計** | **M（3〜7 日相当）** | **中〜高** | 高の 2 群（R4・R7）が全体の危険度を決める |

---

## 6. 要件ディスカッションへ持ち越す判断事項

SC 番号・要件段階の裁定 5 点との対応を併記する。

> **分類結果（要件ディスカッション・2026-08-27）**
> - **カテゴリ A（自明修正・適用済み）**: 3（R4 の `vertical_rl` 限定明記）・12（R11.7 の陳腐化限局）・13（SC5 を既存挙動へ訂正）・2 の一部（R1.6/1.7 の EARS 主語を「書字方向の解決」へ）——コミット `docs(...): fix obvious issues in requirements`。
> - **カテゴリ B（設計フェーズへ先送り）**: 2 の残余（解決層での警告の具体的な観測手段）・6（R4.5 の運用形＝既定写像で足りるか検出縮退まで行うか）・8（scope ID の採取源と未解決スコープの表現）・9（`GENERIC_PROP_NAMES` 登録可否）・§7 の R-1〜R-6。これらは `/kiro-spec-design` で解決する。
> - **カテゴリ C（開発者討議）**: 1（クランプ正準）・4（非負ゲート）・5（`@` 相対）・7（R7 の検証可能性）・10（追跡先）・11（conformance-scope doc）＋要件の裁定 1・4・5。討議結果は各議題のコミットに記録する。
> - **討議確定（進行中の追記）**: 裁定 1＝`writing_mode` 優先・DEBUG ログ（議題 1）。項目 1＝**クランプ正準は撤去**・宣言値は字義どおり・範囲外は DEBUG ログ・フィクスチャは正典推奨形（宣言削除）へ是正・完了 spec 正典の追随義務を登記（議題 2・Requirement 3.10/3.11/10.9）。これに伴い R-3（SC15 連動）は解消——areka では二択が発生しない。R-5 は「宣言済み・validrect 内 origin の観測点が要る」へ単純化。**項目 4・5・6＝一括解決（議題 3）**——部分是正は開発者が却下（「`\_l` はやるなら全部同時」）・`\_l[x,y]` 全語彙を一括所有する `areka-P0-cursor-tag-canon` を即日起票（M2 ゲート・roadmap 追記(85)）・**bvc は `\_l` に一切触れない**（R4 は語彙登記＋既知非互換登記へ縮小・§3.5 と §7 R-1 の実測は同 spec brief へ転記済み）。B に分類していた項目 6（R4.5 単位軸）も同 spec へ移管。**項目 7・8・9＝一括解決（議題 4）**——`.vertical` 単独実導出も開発者が却下（「`currentghost` 実装が無い・spec を立ち上げてそちらへ」）・ukadoc 規模実測（≈180 項目・経路 6 本・本番経路 0 本）を経て**プロパティ系 3 spec を即日起票**（channels／currentghost-tree／catalog-lists・M2 ゲート・roadmap 追記(86)）・**bvc はプロパティを一切実装しない**（R7 は導出規則の語彙登記＋既知の穴の登記へ縮小・R8 の追跡先は currentghost-property-tree で確定・§3.6 の実測は channels/tree の brief へ転記済み）。B に分類していた項目 8（scope ID 採取源）・9（GENERIC_PROP_NAMES）も tree spec へ移管——**bvc は sylphya に触れなくなり、roadmap 干渉台帳の bvc⇄zsp 要ウォッチ（語彙表行隣接）は解消**（台帳更新済み）。**項目 10＝解決（議題 5-a）**——開発者指示「別 spec が必要」で ukadoc `\f` 族全域調査（43 項目・未所有・解読 0）→**文字装飾系 3 spec を即日起票**（text-decoration-canon〔核 17＋基盤・L〕→ anchor-tag-canon〔`\_a`＋16・実バグ 1 件同梱〕＋choice-marker-styling〔cursor* 10・S〕・M2 ゲート・roadmap 追記(87)）・矢印は residue 項目 1 へ第 3 軸として追記＝§3.7 の 3 択は「⑴と⑶の併用」で決着。**項目 11＝解決（議題 5-b・推奨承認）**——`emo2-conformance-scope.md` の「M2 後ろ倒し」記述（:85）を訂正対象とする R11.9 を新設（適合スコープ判断 :61 は不変・`\f[]` M1 不要 :60 には所有確定の参照を添える）。**議題 6・7＝確定**——裁定 4＝DirectWrite 縦組み仕様準拠が正・SSP `@` 機構に合わせない／裁定 5＝会話中切替は実装しない。**ディスカッション完了（7/7 議題・2026-08-27）**——C 項目・裁定 5 点とも全解決・未解決の持ち越しなし。

1. **`origin` クランプ正準（areka 独自）を維持するか撤去するか。** Requirement 3 のどこにも登記が無く、3.9 を字義どおり読むと撤去に至る。撤去すると `vertical_fixture_test.rs:117` が赤になり、`emo2-vertical` フィクスチャの列がバルーン外へ出る。維持するなら Requirement 3 への明記と COMPAT §8 への 1 行が要る。→ §3.3・**SC6 と隣接**（正典が `origin` の再解釈をどこまで及ぼすかを述べていない）。
2. **`vertical` 不正値・両キー矛盾の警告を、どの層の責務とするか。** 要件の EARS 主語は「バルーン定義の解析」だが、`areka-parsers` は無警告契約を持ち `log-capture-kit` も入っていない（＝Requirement 10 で観測できない）。`writing.rs` へ寄せれば既存の警告資産・観測資産にそのまま乗る。→ §3.2・**要件 1.6／1.7／2.5**。
3. **Requirement 4 を `vertical_rl` 限定と明記するか。** `vertical_lr` の `\_l` は既に正典と一致しており、是正対象ではない。要件は「縦書き」と一括で書いている。→ §3.5(a)・**裁定 2**。
4. **Requirement 4.3（負 X で列送り）のために非負ゲートを緩めるか。** 完了 spec `emo-text-layer` の R2.4／6.5 が確定した 4 分岐縮退表と `CursorWarnGuard` の改訂を伴う。緩める軸を「縦書きの列送り軸のみ」に限るか、`\_l` 全体の負値意味論を作り直すかで規模が変わる。→ §3.5(b)・**SC8**。
5. **Requirement 4.6（`@` 相対）は空文か範囲拡大か。** `@` は現在まったく未実装（M1 縮退保持）。縮退経路への規定と読むなら要件文をそう明示すべきで、実装を取り込むなら Requirement 4 の規模が一段上がる。→ §3.5(c)・**SC9 と隣接**。
6. **Requirement 4.5 の「単位と軸を一致させる」の意味。** 既定の写像が正しければ足りるのか、軸に合わない単位（列送り軸への `em` 等）を検出して縮退させるのか。現行は拒否も再写像もしない。→ §3.5(d)。
7. **Requirement 7 の「照会できる」をどう検証するか。** 本番の `ShioriWiring::Helper` 経路には property の運搬が無く（host32 の 3 crate に該当語 0 件）、ghost の in-proc host は sylphya 非接続、`ShioriSession` は別アクターである。⑴ 読み口の新設まで本仕様に含める ⑵ publish と決定論テストまでを範囲とし読み口は追跡先へ送る ⑶ Requirement 7 の EARS を「照会されたとき」から「解決機構が保持する値」へ改める、の 3 択。→ §3.6-3・**裁定 3**。
8. **scope ID の集合をどこから採るか。** `scope(ID)` はセレクタ分岐ではなく「ID ごとに 1 行 publish」である。`BootAssets.balloons` の scope 集合を用いるのが自然だが、未解決スコープの扱い（Requirement 7.4）が publish の**不在**で表現されるのか、明示的な何かで表現されるのかを決める必要がある。→ §3.6-1。
9. **`vertical` を sylphya の `GENERIC_PROP_NAMES` に登録するか。** 登録すれば件数錠 4 箇所の更新（かつ zsp との行隣接が現実化）、しないなら更新 0 で、SET は `currentghost` がルート枝にある帰結として自動的に `NotSettable` になる。GET 経路は台帳を読まないため、登録の目的は「語彙の第一級保持」という設計思想の側にある。→ §3.6・**roadmap の要ウォッチ 1 点**。
10. **Requirement 5.5／8.4 の追跡先を誰にするか。** `balloon-canon-residue` は実在するが自ら「項目を列挙していない spec は所有者ではない」と規律を書いており、文字装飾・下線・プロパティ族は 1 件も列挙されていない。受け皿 brief への項目追記／本仕様 Out 節での先送り（zsp と同形）／新設の 3 択。→ §3.7・**裁定 3 の裏面**。
11. **`doc/emo2-conformance-scope.md` の陳腐化を是正するか。** 同 doc `:61` は「縦書きは痕跡なし・M1 不要」、`:85` は縦書きを M2 へ後ろ倒しと書き、`:60` は `\f[]` を M1 不要に列挙する。本仕様は M1 ウェーブで縦書きを正典化するため文面が食い違う。Requirement 11 は §8 のみを対象にしている。→ §3.8。
12. **Requirement 11.7 の登記文をより正確にするか。** ukadoc-mcp の陳腐化は**プロパティ節に限局**しており、座標側（`origin.x`／`wordwrappoint.x`／`.y`）はスナップショットでも現行と一致する。「スナップショット全体が古い」と読める書き方は、後続に不要な不信を与える。→ §3.9・**SC13**。
13. **SC5 の記述を「仮定」から「既存挙動」へ改めるか。** 列の上限＝`validrect.left` は仮定ではなく実装済みで、`layout_visible_window_tests.rs:60-79` が固定している。→ §3.4。

---

## 7. 追加調査項目（Research Needed・設計段階へ持ち越し）

- **R-1**: `\_l` の列送り軸原点をモード依存にしたとき、`finish_line`（`layout.rs:611-621`）の列矩形規約・`draw.rs` 側の再レイアウト（`rect.left`／縦書き `rect.top` を起点に素の文字列を組み直し、per-glyph `inline_pos` を捨てる・`layout.rs:601-610` の結合コメント参照）・可視窓のあふれ判定（`layout_visible_window_tests.rs`）の 3 者が整合するかの実測。
- **R-2**: `emo2_boot/mod.rs:430-465` で publish する場合の、`balloons` の move 前後の順序と `SylphyaPublisher::barrier()` の要否（初回 talk 前の反映保証）。
- **R-3**: `\_l[0,0]` が `origin.x` の列を指すか `validrect.right` の位置を指すか（SC15・Requirement 4.2）は、§3.3 のクランプ正準の裁定に**従属する**——クランプが効くと「宣言された `origin.x`」が実際の 1 列目と一致しない場合が生じる。両裁定を独立に決めないこと。
- **R-4**: 正典側の再改訂（SC14）に対する追随点を単一に保つ具体形。Option B の「決定の記録」が唯一の追随点になりうるか、`\_l` の軸規約が別の追随点を作らないか。
- **R-5**: `vertical,1` フィクスチャの作り方。既存 `emo2-vertical` は `origin.x,0` を宣言しておりクランプ経路を通る。正典キー版を「同一の表示結果」（Requirement 10.2）で作るなら、宣言値もそのまま複製するのが素直だが、その場合 Requirement 3.1／3.2 の**宣言済み経路**は依然フィクスチャで検証されない（クランプに吸われる）。宣言済み・validrect 内の origin を持つ第 3 の観測点が要るかを設計段階で判断すること。
- **R-6**: COMPAT §8 への追記行が `scope-zorder-pinning`・`present-write-coherence` と隣接する（3 spec が同ウェーブで §8 へ追記予定）。後着側の rebase コストは行マージのみだが、`writing_mode` の新規登記は §8 の既存 48 行のどこへ挿すかで差分の見え方が変わる。

---

# 設計フェーズの調査と決定（2026-08-27・`/kiro-spec-design`）

- 採取日: 2026-08-27（ブランチ `claude/areka-p0-balloon-vertical-a522c6`）
- Discovery Scope: **Extension**（既存システムへの受口追加＋既存正典の 1 点是正）＝light discovery
- 方法: 3 系統の並列踏査（⑴ 転記層と解決層のコード実形 ⑵ クランプ正準の所有 spec と文書の追随先 ⑶ 追跡先 spec の項目列挙・フィクスチャ・テスト網・行数番人）。**本節の file:line はすべて本日再検証したもの**で、§1〜§7 のギャップ分析から転記していない。

## 設計フェーズの主要な発見

1. **受口は実質 0 行**——2 層マージ（`parse.rs:40-57`）は `descript.clone()` へ image 側を `insert` するだけのキー非依存実装。`vertical` の後勝ち（要件 1.5）に追加コードは要らない。
2. **`BalloonModel::new` の呼出は 30 箇所**（本日実測・23 ファイル）だが、**本番はただ 1 箇所**（`parse.rs:142`）。残 29 はテスト／テスト支援。additive ビルダー（`with_cursor`・`with_windowposition_raw` の 2 先例・`model.rs:82-98` が「既存呼び出し側は `new` のまま不変」と doc で宣言）を採れば波及は 0。
3. **`WritingMode::resolve` の本番呼出も 1 箇所**（`actor.rs:153`）。テストは 13（インライン 8・統合 5）。**戻り値型を保てば 2 入力化の波及は 0** ——これが Option B を安く成立させる。
4. **要件 6.4（計測と描画で方向が食い違わない）は構造的に成立済み**。`DirectionRecipe::for_mode` は本番で `create_text_format`（`draw.rs:302-331`）からのみ呼ばれ（本番ヒットは `draw.rs:329` の 1 件）、計測（`DWriteMetrics::new` → `draw.rs:393`）も描画（`viewbox_draw.rs:522`／`:527`）も同じ工場を通り、キャッシュ鍵に `WritingMode` を含む。仕事は「構造で守られていることを檻にする」ことだけである。
5. **クランプ正準の所有者は完了 spec `areka-P0-emo-text-layer`**（`design.md:464`「origin クランプ正準（areka 独自・**本書が正典**）」）。同 design.md はもう 1 箇所（`:716` の軸読み替え正準表「描画開始点＝clamp(origin, validrect)」）でも同じ規約を述べる。派生言及は `:151`／`:788`／`:822`／`tasks.md:49`。
6. **同 spec の requirements.md にクランプを定める受入基準は 1 つも無い**（「クランプ」の grep が 0 件）。design.md だけの発明である。したがって撤去はどの承認済み受入基準とも矛盾しない——上書きされるのは design.md の 2 行だけである。
7. **steering にクランプ正準を主張する記述は 1 件も無い**（10 ファイル全走査）。`roadmap-history.md` の「クランプ」ヒットはすべて別種（窓配置の `resolver.rs` P4／`balloon_limit.rs`）。正典改訂の追随は steering へ及ばない。
8. **`doc/COMPAT_ARCHITECTURE.md` §8 にテスト保護は無い**（`include_str!` 0 件・行数番人なし）。48 データ行（`:128-175`）・列は 項目／裁量／根拠／出典 spec・**追記は末尾**。
9. **上書き行の先例が 2 件ある**——`:147`（`kero-balloon` が `position-persist` R2.2／R8.5 を上書き）と `:153`（`scope-chain-gap` が `window-placement` R2.9 を上書き）。いずれも「アーカイブ済み spec は非改変とし、上書きの事実を本表と現行 spec に記録する」と明記。**`:153` が本仕様の行の雛形になる。**
10. **追跡先 4 本の項目列挙はすべて成立していた**（下表）。要件 4.5／7.5／8.4 の双方向登記は実在確認＋項目突合の双方が満たされている。
11. **`areka-parsers` に `log-capture-kit` は入っていない**（dev-deps は `temp-path-kit` のみ）。転記層に警告を置くと要件 10.6 の決定論テストで観測できない——警告の主体を解決層に置く根拠が実測で裏づけられた。
12. **`draw.rs` は 974 行**（上限 1,000・番人は `log-capture-kit/tests/file_length_guard_test.rs`・例外表 11 件＋件数定数・**例外表は縮小方向にしか動かせない**）。要件 6 の檻は兄弟テスト `draw_format_metrics_tests.rs`（469 行）へ置く。
13. **`emo2-choice` フィクスチャもクランプ経路を通っていた**——`tests/fixtures/emo2-choice/descript-cursor.txt:17-19` が `origin.x,0`／`origin.y,0` を宣言し validrect は `left,5`／`top,5`。要件 10.9 が `emo2-vertical` について命じた是正と**同型の箇所がもう 1 件ある**。

### 追跡先 spec の項目列挙（本日実測）

| 本仕様の要求 | 追跡先 brief | 結果 |
|---|---|---|
| 4.5 の 5 実測 | `areka-P0-cursor-tag-canon/brief.md`（91 行） | ✅ `:18`（非負ゲート）・`:29-32`（`vertical_rl` 原点符号）・`:33`（`vertical_lr` 一致）・`:35`（被覆 0）・`:27`／`:44`／`:54`（縮退表改訂義務） |
| 5.5 の `\f` 系 | `areka-P0-text-decoration-canon/brief.md`（59 行） | ✅ `:20`（align／valign／underline）・`:21`／`:33`／`:59`（SC1 は bvc 裁定を継承し再審議しない） |
| 5.5 の矢印 | `areka-P0-balloon-canon-residue/brief.md`（75 行） | ✅ `:11` の項目 1 に第 3 軸として追記済み |
| 7.5 | `areka-P0-currentghost-property-tree/brief.md`（59 行） | ✅ `:24`（導出規則を bvc 参照で収載）・`:23`（balloon.scope 族 19 項目を全列挙） |
| 7.3 の照会経路 | `areka-P0-property-query-channels/brief.md`（76 行） | ✅ `:14-22` に正典 6 経路 |

### §1〜§7 の記述の訂正（本日の実測による）

- **§3.7（`:167`）の出典が誤り**。「受け側が項目を列挙していない spec は所有者ではない」という規律文は `balloon-canon-residue/brief.md:18` には**無い**（同 brief は規律を適用しているが明文化していない）。正しい出所は `.kiro/specs/completed/areka-P0-balloon-visibility/tasks.md:196`。design.md はそちらを引く。
- **§3.7（`:162`）の行数が誤り**。residue brief は 76 行ではなく **75 行**。
- **§3.1（`:65`）の呼出箇所数**——「約 26」は本日の実測で **30**（23 ファイル）。結論（additive ビルダーで波及 0）は変わらない。
- §3.10 の行数表は本日も一致（`draw.rs` 974・`region.rs` 721・`writing.rs` 224・`parse.rs` 161）。

## Architecture Pattern Evaluation

§4 で提示した 3 案を設計フェーズで評価し直した結果。

| Option | 評価 | 採否 |
|---|---|---|
| **A: 既存機構への最小統合** | 触るファイルは最小だが、⑴ 2.6（層の優劣とキーの優劣を混ぜない）が規約でしか担保されない ⑵ 7.1 の導出点が書字方向の決定点から離れる ⑶ SC14 の追随点が散る | ❌ 却下 |
| **B: 書字方向の解決を第一級の決定へ格上げ** | 上記 3 点が型と構造で解決する。**実測で分かった決定打は「戻り値型を保てば既存 14 呼出箇所の波及が 0 になる」こと**——§4 が挙げた弱点（呼出箇所への波及）は、`WritingMode::resolve` を薄い委譲として残せば消える | ✅ **採用** |
| **C: ハイブリッド（段階化）** | 段の切り方そのものは有用。ただし ③（`\_l`）と ④（プロパティ）は要件討議で本仕様の範囲から外れたため、危険度の高い 2 段が消えている | ⚠ **Migration Strategy として部分採用**（実装順の 6 段へ写す） |

§4 の所見「① で Option B の形を先に作り、②③④⑤ をその上に積む」は**そのまま成立する**（③④が消えたぶん単純になった）。

## Design Decisions

design.md の DD1〜DD9 が正本。ここには design.md に収めきれない判断の背景だけを残す。

### Decision: 不正な `vertical` 値と共存規則の関係（DD6）

- **Context**: 要件 1.6 は「不正値は警告のうえ正典既定の横書きへ縮退」と書き、要件 2.7 は「`writing_mode` の未知値は**指定なし**として扱い、`vertical` が宣言されていればそちらを採る」と書く。両者が非対称に読める。
- **Alternatives**: ⑴ 1.6 を字義どおり「横書きの宣言へ縮退」と読む（`vertical,2` ＋ `writing_mode,vertical_rl` は「不一致の併記」となり 2.5 の DEBUG が出る）⑵ 2.7 と対称に「指定なし」として扱う（DEBUG は出ない）。
- **Selected Approach**: ⑵。
- **Rationale**: **両読みとも最終的な `WritingMode` は全ケースで同一**（他方が無ければ横書き＝1.6 の要求どおり／他方が有効なら `writing_mode` が勝つ）。差は DEBUG 記録の有無だけである。対称形を採ると規則が 1 本になり、値が壊れている側について「両者の値」を DEBUG に残す意味も無くなる。要件はどちらも棄却していない。
- **Trade-offs**: 1.6 の字面から一歩離れる。design.md の Flow 1 と Error Handling の表で明示する。

### Decision: 要件 10.7（既存資産を退行させない）と 3.10（クランプ撤去）の両立

- **Context**: クランプ撤去は既存檻の期待値を反転させる箇所がある。10.7 を「期待値が 1 つも動かないこと」と読むと 3.10 と衝突する。
- **Selected Approach**: **10.7 の「退行」＝被覆の喪失**と読む。期待値の更新は退行ではない。
- **Rationale**: 3.10 は正典の改訂を明示的に命じている（要件段階の開発者裁定）。記憶「陳腐化テストは除外・壊れたら更新」と一致する。さらに実測で、クランプが効いていた箇所は**いずれも「宣言を削除すれば同じ位置になる」形**であり（`emo2-vertical` → (356,46) 不変・`emo2-choice` → (5,5) 不変）、正典推奨形（「通常は指定せず validrect の定義に任せる」）へ揃えれば被覆も期待値も保たれる。
- **Follow-up**: 意図が「宣言された origin」であるテストだけが期待値更新を要する。棚卸しは grep（「クランプ」／`clamp_origin`／「書字開始角」）で網羅し、**緑になったことを完了条件にしない**。

### Decision: 追跡先の双方向登記を檻にしない（DD8）

- **Context**: 要件 4.5／7.5／8.4 は「追跡先 spec の brief が項目を収載していることを**確認する**」と命じる。
- **Alternatives**: ⑴ `include_str!` で brief を読む決定論テスト ⑵ 設計・タスクの検査項目として文書で担保。
- **Selected Approach**: ⑵。
- **Rationale**: `/kiro-complete` のアーカイブ移動が「コードからの spec 文書実ファイル読み」を壊す既知の穴があり（記憶・PR#114 で 5 件が main で赤のまま放置）、追跡先 4 本はいずれも M2 ゲートで brief が requirements.md へ置き換わる。文書間の登記は文書で検査するのが正しい。
- **Trade-offs**: 機械検出が効かないぶん、着手時の file:line 再検証を義務として明記する（陳腐化はこのリポジトリで通算 8 度踏まれている）。

## 統合結果（Generalization / Build-vs-Adopt / Simplification）

- **一般化**: 要件 1（受口）・2（共存）・7.1（プロパティ導出規則）・SC14 追随は、**「このスコープはどちら向きに書くのか、そして誰の宣言によってか」という 1 つの問題**の側面である。`WritingDirectionDecision` へ畳んだ。要件 4・5・7・8・9.3／9.4・11・12 も、**「実装しない正典語彙の登記」という 1 つの問題**の側面であり、COMPAT §8 の 13 行＋双方向登記表という単一の成果物へ畳んだ。
- **Build vs Adopt**: 新設したのは `WritingDirectionDecision`＋補助 enum 3 つと `with_vertical_raw` の 1 メソッドだけ。2 層マージ（キー非依存）・additive ビルダー・生値転記（`limit_raw` 先例）・`count_levels` によるログ件数固定・単一 format 工場・§8 の上書き行の雛形（`:153`）はすべて既存資産をそのまま採用した。外部依存の追加は 0。
- **単純化**: ⑴ `WritingDirectionDecision` を `ResolvedBalloonText` へ**配線しない**（消費者が居ない＝投機的抽象を作らない。必要になったら additive で足せる）⑵ `areka-parsers` へ `log-capture-kit` を**足さない**（警告を置かないので要らない）⑶ `vertical` 用の新しいラッパ型を**作らない**（`writing_mode` と同じ素の `Option<String>` で足りる）⑷ 語彙登記のためのコード型（未実装 `\f` 系の enum 等）を**作らない**。

## Risks & Mitigations

- **クランプ撤去の波及が実測より広い**——`region.rs` インライン 5 件のほか、統合テスト・支援ファイル・フィクスチャ 2 件・doc 8 箇所が候補。→ 段 3 と段 4 を不可分の論理単位として扱い、grep 網羅で棚卸しする。
- **`\_l` への波及の誤検出**——カーソル経路は `region.left()`／`region.top()` を読み（`layout.rs:453-454`）、`region.start()` は読まない。したがってクランプ撤去は `\_l` を動かさない。→ `layout_cursor_tests.rs`（670 行）を**無改変で緑**に保つことを要件 4.4 の証跡とする。
- **COMPAT §8 の行衝突**——`scope-zorder-pinning` が同ウェーブで末尾へ追記する。→ 隣接行マージのみ・意味的衝突なし。後着側が rebase を負う。
- **既存 warn 件数檻の巻き添え**——`writing.rs` のインラインテストが warn 件数を厳密一致で見る。`vertical` 未宣言経路が余計なログを出すと赤になる。→ これは望ましい早期検出であり緩めない。
- **正典の再改訂（SC14）**——SSP は縦書きを依然「試験実装」と称する。→ 追随点を 2 関数（`WritingDirectionDecision::resolve`／`TextRegion::resolve`）と COMPAT §8 の該当行に限局する。

## References

- 正典: ライブ ukadoc（2.8.83 現行）／SSP changelog 2.8.80・2.8.83。**ukadoc-mcp スナップショットはプロパティ節のみ 2.8.80 時点で現行と逆**（座標節は一致）。
- 上書き行の雛形: `doc/COMPAT_ARCHITECTURE.md:153`（`scope-chain-gap` が `window-placement` R2.9 を上書きした行）。
- 双方向登記の規律の出典: `.kiro/specs/completed/areka-P0-balloon-visibility/tasks.md:196`。
- 撤去対象の正典: `.kiro/specs/completed/areka-P0-emo-text-layer/design.md:464`・`:716`（アーカイブは非改変）。
- テスト配置規約: `.kiro/steering/structure.md`（Unit Tests・`<stem>_<モジュール名>.rs`・1 ファイル 1,000 行・`include_str!` 構造テストは兄弟テストファイルも列挙する）。
