# ギャップ分析: areka-P0-balloon-offset-dpi

> 作成 2026-08-27 ／ 対象＝確定済み `requirements.md`（Requirement 1〜9）と現行コードベースの差分。
> 本書は**判断材料であって決定ではない**。選択肢と根拠を並べ、要件ディスカッションへ渡す。
> 記載した `file:line` はすべて 2026-08-27 の作業ツリー実測。

---

## 1. 分析サマリ

- **オフセットの実体は 1 つ・供給元は 4 つ**。`BalloonFollow.offset`（`follow/drag_follow.rs:33-34`「キャラ窓左上からバルーン窓左上への相対 offset（物理 px・配置時確定）」）へ、`descript` の `balloon.offsetx/offsety`・`windowposition` 由来の調整量・キーワード由来の基本位置・利用者のドラッグ結果が合流する。合流欄は `ScopeConfig.balloon_offset` → `ScopePlacement.balloon_offset` → `spawn.rs:484` で `BalloonFollow.offset` へ転写される 1 本道であり、**新しい配管を敷かずに供給元ごとの換算を差し込める形になっている**。
- **単位空間の非対称は「どちらの拡大率か」の問題でもある**。`windowposition` 由来は**バルーン軸**の拡大率で換算済み（`mod.rs:378`・理由は `mod.rs:395-397` に明文）だが、`balloon.offsetx/offsety` は**ゴースト／シェルの `descript.txt`** から読まれる（`config.rs:264-274` の `cascade2(ghost_kv, shell_kv, ...)`）＝**シェル作者空間の語彙**である。Requirement 1.4 が求める「供給元ごとの一意な割り当て」は、この非対称に対する裁定そのものになる。
- **換算の道具は揃っているが、逆向きの道具は「作らない」と明文で決まっている**。`scale_signed`（`windowposition.rs:133`・大きさは `ScaleRatio::scale_len` へ委譲・符号保存・`±i32::MAX` 飽和）が既に `windowposition` と `\![move]` の共用部品で、Requirement 2.1/2.4/2.5 はこれの再利用でほぼ満たせる。一方 `ScaleRatio` の公開 API は `ONE`／`new`／`mul`／`is_identity`／`as_f32`／`scale_len`／`scaled_extent`／`unscale_coord` だけで、**逆数も分子・分母の読み口も存在しない**。しかもこれは欠落ではなく**明文の拒否**である——`scale.rs:256-260` が「本 spec が `scale.rs` へ追加する公開面は本メソッドのみである（`num`／`den` アクセサは**新設しない**）」と記す。ゆえに「旧拡大率から新拡大率への比」を今日の公開面では**作れない**。これが Requirement 3.3（往復で誤差が累積しない）の成否を分ける最大の分岐点になる。
- **既存の決定論テストが Requirement 3.1 の正反対を固定している。** `frame_dpi_reproject_tests.rs:382` の `s2_some_report_path_preserves_the_balloon_ground_anchor_across_mixed_dpi_levels` は、`(96→120)`／`(96→192)`／`(120→192)` の 3 遷移すべてで **`BalloonFollow.offset` が書込前と bit 同一であること**を主張する（`:502-507`）。しかも書込**前**に読んだ値と突合する形で、空振り防止の証人まで 3 つ持つ（`:384-385`／`:454-459`／`:492-497`）。**本仕様の中心的な是正は、このテストを赤にすることで初めて成立する**——Requirement 7.4（是正ごとに是正前は失敗し是正後は通過する対を持つ）の「是正前は失敗する側」が既に書かれている、と読むのが正しい。同型の主張は `follow_visibility_balloon_wiring_tests.rs:850`（`:923-927`）にもある。
- **遷移時に旧拡大率を覚えている場所がどこにも無い**。`DPI` component（`crates/wintf/src/ecs/window/dpi.rs:24-29`）は現在値のみで、`Changed<DPI>` は差分を運ばない。さらに `run_dpi_phase` の `refresh_scale_report` が `None` を返しても拡大率が不変とは限らない（`frame/dpi.rs:329-332` が明記）ため、**「寸が変わったこと」を追随の発火条件にはできない**。旧値の保持は本仕様が新設する必要がある（Missing）。
- **キーワードとの排他は宣言では成立しない**。`rederive_keyword_balloon_offset` の発火条件は `old_size != new_size` **だけ**（`follow/keyword_base.rs:79-81`）であり、経路を見ない設計が明文で理由づけられている（同 `:71-78`）。拡大率遷移も寸を変えるので、素材が未消費のまま遷移を迎えれば再導出と拡大率追随が**同一フレームで両立する**。Requirement 4.3 は実装上の門を要求している。

---

## 2. 現況調査（Requirement 別の接地点）

### 2.1 オフセットの実体と 4 つの供給元

| 供給元 | 実装位置 | 現在の単位 | 換算の有無 |
|---|---|---|---|
| `descript` の `balloon.offsetx`／`offsety` | `config.rs:264-274`（`cascade2(ghost_kv, shell_kv, scope, ...)`） | 作者空間の生値 | **なし**（非スケール） |
| `windowposition` 由来の調整量 | `mod.rs:378` → `windowposition.rs:175 to_screen_adjust` → `:198 apply_windowposition` | 物理 px | **あり**（バルーン軸の拡大率） |
| キーワード由来の基本位置 | `resolver.rs` P5 `keyword_balloon_pos` ／ `follow/keyword_base.rs:59` | 物理 px（実表示寸から導出） | 換算ではなく寸からの導出 |
| 利用者のドラッグ結果 | `follow/drag_follow.rs` `on_balloon_drag`（`balloon_pos − char_pos` を記憶更新） | 物理 px | **なし** |

- 合流欄の型は `ScopeConfig.balloon_offset: Option<(i32, i32)>`（`config.rs:68-69`）。両軸が揃ったときのみ `Some`（`config.rs:274`・既存規約を固定するテストは `config.rs:686 t_c5_balloon_offset_requires_both_components`）＝**Requirement 2.6 が「変更しない」と定める規約はここ**。
- 合流は加算（`windowposition.rs:216-219`・`saturating_add`）。P5 は `balloon_offset.unwrap_or((0, 0))` を既に加算している（`resolver.rs:294`）ため、**供給元を増やしても配置式 P1〜P5 は無改変で済む**構造が既にある（`mod.rs:390-393`）。
- `BalloonFollow` の定義は `follow/drag_follow.rs:29-35`（`balloon: Entity` と `offset: PointPx` の 2 欄のみ・`Copy`）。

### 2.2 単位空間混在の実体（Requirement 1・2）

- 混在を「意図的な暫定」と明記する現行記述は 2 か所:
  - `windowposition.rs:191-197` の見出し「注意（単位空間の混在・意図的）」——「`windowposition` 由来の調整量は k 適用済みの物理 px で合流するが、既存供給元である descript の `balloon.offsetx`/`offsety` は**非スケールの生値**のまま加算される——後者の規約温存は本仕様の Out of scope（W5 対象外）」
  - `windowposition.rs:215` の実装コメント（同 doc への参照）
  - → **Requirement 1.3 が置き換えを命じている記述はこの 2 か所**。ほかに同種の記述は placement 配下に見当たらない。
- **「どちらの拡大率か」の非対称は既に本番に存在する**: 起動時の `windowposition` は**バルーン軸**（`mod.rs:378` の `scaling.balloon`）、実行時の `\![move]` は**シェル軸**（`drain_resnap.rs:97` の `applied_ratio(shell_target(scope))`）を使う。**同じ `scale_signed` を通しながら分母が違う**。Requirement 1.4 はこの既存の非対称も含めて記録を求めている。
- **どちらの拡大率か**の既存裁定は `mod.rs:395-397` にある: 「`k` は**バルーン軸**の表示スケール（`MeasureScaling::balloon`）である——`windowposition` はバルーン作者の空間で書かれた値ゆえ、シェル軸の k を掛けてはならない」。`balloon.offsetx` はゴースト／シェル側 `descript.txt` の語彙なので、同じ論法を素直に延ばすと**シェル軸**になる。ところが両者は同じ欄で加算される——**加算の前に別々の拡大率で換算した値を足すこと自体は矛盾ではない**（どちらも物理 px に揃っているため）が、Requirement 1.4 はその割り当てを明示的に記録することを求めている。

### 2.3 拡大率の解決経路（Requirement 1.5・2.1・3.6）

- **丸め権威**: `ScaleRatio`（`crates/areka-emo-compose/src/scale.rs:53`・`num`／`den` は**私有**）。公開 API は `ONE:69`／`new:76`／`mul:106`／`is_identity:141`／`as_f32:158`／`scale_len:180`／`scaled_extent:201`／`unscale_coord:264`。**逆数・比・分子分母の読み口は無く、`scale.rs:256-260` が `num`／`den` アクセサを新設しないと明文で拒否している。**
  - `scale_len` の規則（`:180-193`）: `len == 0 → 0`／恒等 k は素通し／それ以外は u128 で `(2·len·num + den) / (2·den)`＝**round half away from zero**、非ゼロは最小 1px へ引き上げ、溢れは `u32::MAX` へ飽和（i32 域への収め直しは呼び手の責務）。
  - `unscale_coord`（`:264`）は `((2v+1)·den).div_euclid(2·num)`＝**床方向**。`scale.rs:216-227` が「これは `scale_len` の**対**であって逆関数ではない」「引数は**点の座標**であり、寸法・長さを渡してはならない」と明記する。**丸め方向が非対称**（away-from-zero と床）なので、この 2 本を往復させると誤差が残る。現在の唯一の消費者は当たり判定（`areka-emo-compose/src/hit.rs:143-145`）。
- **既製の換算部品**: `scale_signed(v: i32, k: ScaleRatio) -> i32`（`windowposition.rs:133-137`）。大きさは `scale_len` へ委譲（round half away from zero・非ゼロ長は最小 1px・恒等 k は素通し）、符号は本層で保存、`i32::MIN` も panic せず `±i32::MAX` へ飽和。既に `\![move]` と共用（`doc/COMPAT_ARCHITECTURE.md:154` の出典欄）。→ **Requirement 2.1／2.4／2.5 は本関数の再利用で満たせる**（新しい丸め規約を導入しない・飽和して回り込まない・記録は呼び手側）。
- **配置層が持つ拡大率は起動時の値**: `build_measure_scaling(primary_dpi, author_dpi)`（`mod.rs:363`）。主モニタ DPI を分母に取り、採寸と `apply_scope_windowpositions` に配られる。**いま窓が載っているモニタの値ではない**（前提の実測確認 4 と一致）。
- **実行時の真実源**: `EmoPresenter::applied_ratio(target) -> Option<ScaleRatio>`（`crates/areka-emo-present/src/presenter/read.rs:181`）。既に `\![move]` が採用しており、縮退の流儀まで確立している——`drain_resnap.rs:95-98`:
  ```
  let k = wiring.presenter.applied_ratio(shell_target(directive.scope))
      .unwrap_or(ScaleRatio::ONE);
  ```
  同 `:93-94` が「表示未成立・未登録 target は `None`。このとき恒等へ縮退する——まだ拡大が掛かっていない状態であり、従来（k 非適用）と同じ値になる安全側の既定である」と理由づけている。→ **Requirement 1.5／3.6 の縮退規約はこの donor をそのまま踏襲できる**（ただし本仕様は「警告として記録する」を求めており、現 donor は無警告。ここは差分）。
- **作者基準 DPI は 2 本**: `AuthorDpi { shell: u16, balloon: u16 }`（`mod.rs:207-213`・既定 96＝`AuthorDpi::DEFAULT:217`）。frame 側は同型の `AuthorDpis`（`frame/dpi.rs:33-38`）で、**target の同一性から引く**設計（`for_target:46`・取り違え防止）。読み取りは `prepare_stages` で 1 度だけ（`mod.rs:352-359`）。
- **任意 DPI から拡大率を作る関数**: `derive_scale(ScalePolicy, Option<(dpi_x, dpi_y)>) -> ScaleRatio`（`areka-emo-present`・挙動は `scale_tests.rs:21-92` が固定）。`ScalePolicy::new(author_dpi, app_scale)`。DPI 不在は `app_scale × 1/1` へ縮退。→ **旧 DPI さえ判れば旧拡大率は作れる**。判らないことが問題（次節）。

### 2.4 拡大率遷移の実形（Requirement 3・4）

- 入口は `run_dpi_phase`（`crates/areka/src/emo2_boot/frame/dpi.rs`）。`Changed<DPI>` の窓を 2 巡で処理し、窓ごとに `refresh_scale_report(world, target)` を呼ぶ（`:335`）。
  - `Some(new_size)` → `reconcile_window_size(..., PlacementRoute::DpiReproject)`（`:341-347`）。キャラ窓は `resize_window_to`、バルーン窓は `resize_window_keep_position` へ振り分け（`:103-108`）。
  - `None` → キャラ窓のみ `reproject_char_window_at_current_size(..., DpiReproject)`（`:363-367`）。バルーン窓は何もしない（`:369`）。
- **`None` は拡大率不変を意味しない**——`:329-332` が明記: 「`None` は k 不変とは同義でない——不可視・未表示・失敗に加え、**k は変わったが丸め後の物理寸が同じ**場合も `None` である」。→ **Requirement 3.1 の発火条件を「寸が変わったこと」に置くと、丸めで寸が一致した遷移を静かに取りこぼす。**
- キャラ窓の末端 `resize_window_to`（`follow/window_move.rs:185-415`）の手順:
  - 手順 5（`:350`）位置＋寸を**一度だけ**書く（Requirement 3.4 が守れと言っている「一度書き」）
  - 手順 5b（`:368`）接地点の観測（`transition_diag` 有効時のみ）
  - **手順 5a（`:380`）`rederive_keyword_balloon_offset`**
  - **手順 6（`:404`）`follow_balloon`** ＝ offset をそのまま加算してバルーンを書く
  - → **追従オフセットの書換えは手順 6 より前に入れる必要がある**（`:374-375` が同じ理由で 5a を 6 より前に置いている: 「offset を直してから追従させないと、このフレームのバルーンは古い offset で書かれ、次に何かが動くまで直らない」）。
- **`move_window_with_route` も offset を素通しで加算する**（`:76-92`）。こちらは `\![move]`／連鎖再解決の経路であり拡大率遷移ではないため、本仕様の変更対象ではないが、`BalloonFollow.offset` の表現を変えるなら読み手として追随が要る。
- **遷移の相は `BalloonFollow` に一度も触れていない**: `crates/areka/src/emo2_boot/` 配下に `BalloonFollow` への**非テスト参照が 0 件**（実測）。追随の実装はゼロという brief の記述は正しい。
- **旧拡大率の保持場所が無い**: `DPI`（`crates/wintf/src/ecs/window/dpi.rs:24-29`）は `dpi_x`／`dpi_y` の現在値のみ。`BalloonFollow` にも `ScopePlacement` にも拡大率の欄は無い。**Missing。**
  - ただし `run_dpi_phase` の内側では **`refresh_scale_report` を呼ぶ前に `applied_ratio(target)` を読めば旧値が取れる**（`refresh_scale` が新値を確定させる前だから）。新しい保存場所を作らずに旧値を得る候補経路として設計で評価する価値がある。
- **観測の donor**（Requirement 3.7・8.3）:
  - `placement/transition_diag.rs` — 既定 OFF の構造化観測チャネル。種別語彙 `KIND_SNAPSHOT`／`KIND_HOLD`／`KIND_GROUND`／`KIND_CHAIN`（`:74-87`）と欄名定数 `FIELD_*`（`:155-179`）、全語彙の網羅表（`PLACEMENT_KIND_ALL:87` ほか）を持つ。追随の記録は**この語彙表へ 1 種別を足す**形が既存流儀に合う。
  - `placement/transition_judge*.rs` — 実機ログの機械判定ランナー（判定語・手順つき）。
  - `emo2_boot/frame_test_support.rs` `FrameHarness` — 遷移の決定論テスト用の器（`:157-188` に既存の `balloon_offset` 固定値あり）。

### 2.5 キーワード由来の基本位置との排他（Requirement 4）

- `rederive_keyword_balloon_offset`（`follow/keyword_base.rs:59-163`）:
  - 素材は `BalloonKeywordBase` component。**発火条件は `old_size != new_size` のみ**（`:79-81`）。
  - 経路で絞らない理由が `:71-78` に明文——「route で絞ると、どの route が最初に**実表示寸**を運ぶかという呼出順の知識を本関数が持つことになり、frame の相順が変わるたびに静かに壊れる」。→ **単純に「`DpiReproject` のときは 5a を飛ばす」と書くと、この明文の設計判断を反転させることになる。**設計で正面から扱うべき衝突。
  - 一度きり＝発火後に `remove::<BalloonKeywordBase>()`（`:148`）。
  - 素材の退役経路は他に 2 つ: 起動時の保存値優先（`persist.rs merge_scope`）とバルーン単独ドラッグ保存時（`drag_follow.rs retire_keyword_base_on_save`）。
- 幾何は `keyword_balloon_pos`（`resolver.rs`）の 1 か所のみ（`keyword_base.rs:49` が「幾何は書き写さない」と明記）。中央揃えは `char_x + (char_w − balloon_w) / 2`（整数除算・`COMPAT_ARCHITECTURE.md:173`）。
- **Requirement 4.4 の残差はここから出る**: 中央揃え式は `char_w`（シェル軸で伸びる）と `balloon_w`（バルーン軸で伸びる）の**差**である。シェルとバルーンの作者基準 DPI が異なると 2 本の拡大率が別々に動くため、**オフセットを単一の拡大率で追随させると揃えに残差が出る**。残差の大きさは概ね `((k_shell比 − k_balloon比) × balloon_w) / 2` の程度。許容量の明示（Requirement 4.4）はこの式の評価が要る。

### 2.6 保存・復元（Requirement 5）

- 保存: `balloon_offset_entries(scope, offset_persist)`（`persist.rs:107-124`）＝`PersistKey::BalloonOffset { scope, axis }` の X/Y 2 本へ `i32` の `Display` 文字列。**値ごとの版なし・拡大率なし**。
  - 正準ドットキー `areka.balloon.offset.scope(ID).x|y`（`crates/areka-sylphya/src/persist/mod.rs:131-137`・`:155-157`）、TOML の表名は `"balloon-offset"`（`persist/format.rs:119-121`／`:207`）、格納先は `FormatDoc.balloon_offset: BTreeMap<String, AxisPair>` で **`AxisPair` の両軸とも `Option<String>`**（`format.rs:92-97`）＝**値は全部文字列**。
  - ⚠ **ファイル全体には既に `format-version = 1` がある**（`format.rs:53`・`:111-114`）。Requirement 5.1 の「保存形式に版を導入しない」は**値ごと／キーごとの版を作らない**という意味に読むべきで、ファイル版が既存であることと矛盾しない——設計で語を精密にしておくこと。
  - **保存値は component からではなく最終位置から引き直される**: `on_balloon_drag_end`（`follow/drag_follow.rs:673-704`）が `offset_tl = balloon_final_pos − char_pos`（`:561-572`・`:641`）を作り `:689` で `persist` へ、`:704` で `balloon_offset_entries` へ渡す。**実行時表現を変える案（Option B/C）では、ここが「確立点」の 1 つになる。**
- 基準の明文: `persist.rs:98-106`「保存表現はセッション内の `BalloonFollow.offset` と**同一の基準**（char 窓左上相対・物理 px）」。
- 復元: `persist.rs:393-403`——保存値が両軸揃えば**基準変換なしでそのまま採用**し、同時にキーワード素材を退役させる（`:393` の分岐）。優先順位は保存値 > 配置式の既定。
- 補正を焼き付けない規約（`COMPAT_ARCHITECTURE.md:169`）: `ScopePlacement.balloon_offset`・`BalloonFollow.offset`・`PersistKey::BalloonOffset` はいずれも補正前の生値を保つ。
- → **Requirement 5.1／5.2／5.5／5.6 は現行実装がそのまま満たしている**。本仕様の作業は⑴ 5.3 の「明示の例外である」ことの記録、⑵ 7.2 の非回帰テスト（裁定が黙って反転しないこと）、⑶ 5.4（復元後の遷移では Requirement 3 が効く）の配線、の 3 点に絞られる。**実行時表現を変える案（後述 Option B/C）を採る場合のみ、復元時に「保存された物理 px を実行時表現へどう写すか」という新しい接点が生じる。**

### 2.7 互換記録（Requirement 6）

- 追記先は `doc/COMPAT_ARCHITECTURE.md` §8「沈黙ルール対応表」（`:122-175`）。4 欄＝`項目 | 裁量 | 根拠 | 出典 spec`。
- **Requirement 6.4 が「矛盾しない」ことを求める既登記の同種行**（＝作者空間オフセットの扱いの前例）:
  - `:146` **`windowposition.x` の符号規約** — 実機実測により「`wp.x × k` をそのまま画面座標へ加算」。
  - `:147` **サーフェス寸変動時のバルーン追従基準** — 窓相対で追従・リサイズで `BalloonFollow.offset` を補正しない・runtime と保存で同一基準。**本仕様が足すのは「拡大率遷移」の行であり、この行は書き換えない**（前提の実測確認 9 と一致）。
  - `:154` **`\![move]` の dx／dy を拡大率で換算する** — **意図的な SSP 非互換**。登記理由の第一が「同種の作者空間オフセットである `windowposition.x/y` は既に `wp.x × k` で適用しており、`\![move]` だけ素通しにすると同じ性質の値の扱いが割れる」＝**内部整合そのもの**。k の真実源は表示層の `applied_ratio`。→ **`balloon.offsetx` に拡大率を掛ける判断は、この 2 例と同じ系列の 3 例目として整合する。**
  - `:148`／`:150` **丸めの 1px 差と「非ゼロ長は最小 1px」の継承** — 本仕様も同じ権威に乗るなら追加の裁定は不要。
- **要ウォッチ（Requirement 6.6 との緊張）**: `:172` の行（所有＝`areka-P0-windowposition-limit`）が「キーワード指定時に……`balloon.offsetx`／`offsety` を基本位置へ加算する。数値指定時とまったく同じ扱い」と書いている。Requirement 2 が同オフセットを拡大率で換算すると、**この行が前提にしている「生値の加算」という含意が古くなる**。6.6 は他仕様の記述の書換えを禁じているため、⑴ 自分の行で上書きを明示する／⑵ 行の所有者へ相互確認する、のいずれを採るかが裁定事項。
- **SSP 観測（Requirement 6.1〜6.3）は決定論では出せない**。`:153` の付記が既に「SSP のゴースト演出 `\![move]` のオフセットが**物理 px 無スケールのまま**適用される（DPI 192 で 313px の重なり）」を SSP 自己不整合として記録している。**この既存観測は「SSP は拡大率跨ぎでオフセットをスケールしない」という強い傍証**であり、Requirement 6.1 の観測がこれを追認する可能性が高い。その場合 6.3 の腕（areka 設計原則から導出して裁量登記）へ入る。

### 2.8 決定論テストの土台（Requirement 7）

- 共有ハーネスは `test-cage-determinism`（PR#119・2026-08-27 完了）で一本化済み——ログ捕捉は `log-capture-kit`、一時パスは `temp-path-kit`。`placement/test_support.rs` は委譲する薄い層になっている（`:1-` 実測）。**本仕様のテストは最初から共有ハーネスで書ける。**
- 遷移テストの器は `emo2_boot/frame_test_support.rs` の `FrameHarness`（`refresh_scale_report` の差替口が `:365`）。
- 既存の配置テスト群は `placement/` 直下に兄弟ファイルとして置く慣行（`follow_*_tests.rs`・`persist_*_tests.rs` ほか約 40 本）。
#### 既存テストの全数区分（Requirement 7.4／7.6 の入力）

現行の主張は 4 群に分かれ、**本仕様との関係が群ごとに違う**。

**(a) 拡大率遷移をまたいで offset の不変を主張する＝本仕様が反転させる対象**

| テスト | 位置 | 主張 |
|---|---|---|
| `s2_some_report_path_preserves_the_balloon_ground_anchor_across_mixed_dpi_levels` | `emo2_boot/frame_dpi_reproject_tests.rs:382` | `(96→120)`／`(96→192)`／`(120→192)` × 全 scope で、**書込前に読んだ** offset と書込後が bit 同一（`:502-507`）・差分も同値（`:511-516`）。空振り防止の証人 3 つ（`:384-385`／`:454-459`／`:492-497`） |
| `balloon_follows_the_guarded_char_position_not_the_raw_projection` | `placement/follow_visibility_balloon_wiring_tests.rs:850` | DPI 引数化・`stored_offset == offset`（`:923-927`） |

> ⚠ **(a) は「取りこぼし」ではなく「現行契約の正確な写し」である。** 是正すると赤になるのが正しく、それが Requirement 7.4 の対（是正前失敗／是正後通過）そのものになる。**削除ではなく主張の書き換え**（拡大率遷移では追随する／その他の寸法変化では不変）で扱うのが本プロジェクトの流儀（陳腐化テストは除外・壊れたら更新）。同テストは自らの doc（`:330-358`）で「恒等式を、それを作った当人に問う」空振りの罠を戒めており、書き換え後も**書込前読み**の構造を保つこと。

**(b) 恒等式のみを主張する＝再スケールに対して空振り（無害だが証拠にならない）**

`frame_dpi_reproject_tests.rs:273`／`frame_dpi_reproject_none_tests.rs:33`／`frame_transition_atomicity_tests.rs:285`（`transition_is_atomic_at(dpi)`・駆動は `:534`／`:540`）／`frame_transition_branch_tests.rs:557`（駆動は `:615`／`:621`）。いずれも offset を書込**後**に読んで `balloon − char` と突合するため、追随が入っても緑のまま通る。**「全部緑だから壊していない」の根拠に使ってはならない。**

**(c) 寸法変化に対する不変＝Requirement 3.2／9.8 が守れと言っている側（変更しない）**

`follow_resize_tests.rs:176`（`resize_window_to_bottom_preserves_balloon_follow_offset`・`:237`）／`:261`（`resize_window_to_bottom_keeps_ssp_window_relative_balloon_offset`・`window_move.rs:168` が名指しするテスト）／`:476`（Left 版）／`emo2_boot/frame_work_area_resnap_tests.rs:156`（作業領域再スナップで offset に触れない）。→ **Requirement 9.7 が「作業領域の再スナップについての期待であって拡大率遷移には及ばない」と区別せよと言っているのは、まさに最後の 1 本のこと。**

**(d) 補正を焼き付けない／キーワード分岐／往復の同一性（いずれも変更しない）**

`follow_drag_end_limit_tests.rs:281`（`:351`）／`balloon_limit_gate_tests.rs:121`・`:371`／`main_restore_seam_tests.rs:158`／`follow_keyword_base_tests.rs:484`（`:505`）・`:520`・`:229`／`spawn_follow_pipeline_tests.rs:122`・`:183`／`persist_restore_tests.rs:95`・`:141`（`:203`）／`follow_drag_tests.rs:48`（`:45-46` が「再スケールなしの檻として」96 の倍数を避けた座標を使うと明記）／`follow_window_move_tests.rs:55`（同型）。

> ⚠ (d) の末尾 2 本は **`\![move]`／キャラ窓ドラッグ経路の「再スケールなし」** を固定している。本仕様が `BalloonFollow.offset` の**表現**を変える案（Option B/C）を採ると、これらも意味が変わる。表現を変えない案（Option A）ではそのまま生き残る。

- Requirement 7.6 が両立を求める「拡大率が混入しないことを固定する」テストは (c)(d) 群。**対象（寸法変化 vs 拡大率変化）を区別すれば両立するが、`resize_window_to` が共通口である以上、追随の挿入位置しだいで巻き添えになる。設計時に上表で全数突合すること。**
- **供給側のギャップ**: 初期供給の連鎖（descript の生値 vs `windowposition` の拡大率適用済み値が `windowposition.rs:216` で合流する箇所）について、**拡大率遷移の不変を主張するテストは 1 本も無い**。Requirement 7.1 の行列はここを新規に埋めることになる。

---

## 3. Requirement → 資産マップ（ギャップ表）

| Req | 既存資産 | ギャップ | 種別 |
|---|---|---|---|
| 1.1 単一空間の確定 | 合流欄 `ScopeConfig.balloon_offset` が既に 1 本道 | 空間の**選択**が未定（物理 px か作者空間か） | 判断 |
| 1.2 換算後のみ合流 | `scale_signed` が既製 | descript 供給元に換算点が無い | Missing（小） |
| 1.3 暫定記述の置換 | `windowposition.rs:191-197`／`:215` | 置換文言は確定契約が決まってから | 判断待ち |
| 1.4 供給元→拡大率の割り当て | `AuthorDpi{shell,balloon}`・`mod.rs:395-397` の前例 | `balloon.offsetx` はシェル語彙／合流先はバルーン軸——**割り当てが自明でない** | 判断 |
| 1.5 解決不能時は恒等＋警告 | `applied_ratio().unwrap_or(ONE)` の前例（`drain_resnap.rs:95-98`） | 前例は**無警告**。警告付き縮退は新規 | Missing（小） |
| 2.1 descript オフセットの換算 | `scale_signed`・供給層 `apply_scope_windowpositions` が既に拡大率を持つ | 換算の**適用点**が未定（供給層か P5 か） | 判断 |
| 2.2 拡大率 1 で同一出力 | `scale_len` は恒等 k を素通し | — | 充足見込み |
| 2.3 両者を同一空間で加算 | 加算は `windowposition.rs:216-219` | 2.1 の帰結 | 従属 |
| 2.4 新しい丸めを導入しない | `ScaleRatio::scale_len` が単一権威 | — | 充足見込み |
| 2.5 飽和・記録 | `scale_signed` が `±i32::MAX` 飽和 | **飽和したことの記録が無い**（現状は黙って飽和） | Missing（小） |
| 2.6 片軸／未宣言の受理規約 | `config.rs:274` `zip`・テスト `config.rs:686` | 変更しない | 充足 |
| 3.1 遷移でオフセットを更新 | 遷移の入口 `run_dpi_phase`・末端 `resize_window_to` | **実装ゼロ**。旧拡大率の保持場所も無い | **Missing（大）** |
| 3.2 拡大率不変の寸法変化では触らない | `resize_window_to` は現に触らない（`:390-398`） | 3.1 を入れたあと**この不変を壊さない**ことが要点 | 制約 |
| 3.3 往復で誤差が累積しない | — | `ScaleRatio` は逆数／比の API を**明文で拒否**（`scale.rs:256-260`）。`unscale_coord`＋`scale_len` の 2 段は丸め方向が非対称（床 vs away-from-zero）で保証できない | **Unknown（要設計）＋既存判断との衝突** |
| 3.4 書込回数を増やさない | 手順 5 の一度書き・手順 6 の随伴 | 追随は**手順 6 より前**に置く必要 | 制約 |
| 3.5 ドラッグ由来にも同一規則 | ドラッグは `on_balloon_drag` が物理 px で記憶 | 作者空間表現を採る案では**逆写像**が要る | 判断依存 |
| 3.6 解決不能なら変更せず警告 | 1.5 と同型 | 同上 | Missing（小） |
| 3.7 前後の値を記録 | `transition_diag` の語彙表 | 種別 1 つ分の新設 | Missing（小） |
| 4.1／4.2 キーワードの揃えを保つ | 追記(70) の裁定＝再導出せず拡大率追随 | 3.1 の帰結 | 従属 |
| 4.3 再導出と追随の排他 | `keyword_base.rs:79-81` は**寸変化だけで発火**・経路で絞らない設計が明文 | **同一フレームで両立し得る**。門が要る／明文の設計判断と正面衝突 | **Missing（中）＋判断** |
| 4.4 単一拡大率と残差の許容量 | 中央揃え式（`resolver.rs`）・2 本の拡大率 | 残差の見積りと許容量が未定 | 判断 |
| 4.5 一度きり再導出を廃止しない | `keyword_base.rs` 全体 | 変更しない | 制約 |
| 5.1〜5.3 保存は物理 px・版なし・例外の明記 | `persist.rs:107-124`／`:98-106` | **実装は現行のまま**。記録の追加のみ | 充足＋記録 |
| 5.4 復元後の遷移には Req3 が効く | `persist.rs:393-403` 復元 → 以後は通常経路 | 実行時表現を変える案でのみ接点が生じる | 判断依存 |
| 5.5／5.6 優先順位・生値保存 | `persist.rs merge_scope`・`COMPAT:169` | 変更しない | 制約 |
| 6.1〜6.3 SSP 観測と裁量登記 | `COMPAT:153` に SSP 無スケールの既存観測 | **実機観測が要る**（決定論では出せない） | **Research Needed** |
| 6.4 同種前例と矛盾しない | `COMPAT:146`／`:154` | 3 例目として整合する見込み | 充足見込み |
| 6.5 登記は 3 点を含む | §8 の 4 欄形式 | 新設行 | Missing（小） |
| 6.6 自分の行に限る | — | `COMPAT:172` の含意が古くなる件の扱い | 判断 |
| 7.1〜7.9 決定論テスト | 共有ハーネス・`FrameHarness`・約 40 本の兄弟テスト | 行列の新設。**7.4 の「是正前は失敗する側」は既に (a) 群として存在する**（`frame_dpi_reproject_tests.rs:382` ほか）＝書き換えが要る。7.6 は (c)(d) 群との両立確認 | Missing（中）＋**既存テストの反転** |
| 8.1〜8.5 実機サインオフ | `transition_judge*`・実機ログ機械判定・atom の手順書 | 手順の流用＋判定項目の新設 | Missing（中） |
| 9.1〜9.8 非回帰と記録 | 既存契約は全て明文化済み | 9.5（共通経路の署名変更は相互確認）は zsp との調整が要る | 制約 |

---

## 4. 実装アプローチ

3 案とも「拡大率をどこから得るか」という共通課題を持つ。`resize_window_to` は `placement` 層にあり `EmoPresenter` を持たない一方、`applied_ratio` は表示層にある。既存の解き方は `drain_resnap.rs:95-101` の**「frame 層で拡大率を解決して引数で下ろす」**——本仕様も同じ形が採れるが、`resize_window_to` の署名変更は同居 spec（zsp）との相互確認事項（Requirement 9.5）に触れる。署名を触らない代替は⑴ frame 層で `BalloonFollow.offset` を先に書き換えてから `reconcile_window_size` を呼ぶ、⑵ 追随の指示を component（例: `PendingOffsetRescale`）で渡し `resize_window_to` の内側で消費する、の 2 つ。

### Option A: 物理 px で一本化し、遷移では比を掛ける（表現据置き）

合流欄と `BalloonFollow.offset` を「**現在の拡大率における物理 px**」と定義する。descript の生値は供給層で `scale_signed` により換算して合流させる（Requirement 2 が解ける）。遷移では `offset ← offset × (k_new ÷ k_old)`。

- **変更対象**: `config.rs`／`mod.rs` の供給層（換算点の追加）・`windowposition.rs` の doc（1.3）・`frame/dpi.rs`（旧拡大率の捕捉と追随の呼び出し）・`follow/window_move.rs` または新規の純関数モジュール・`transition_diag.rs`（記録種別）。
- **利点**: `BalloonFollow` の形が変わらない。`persist.rs`・`balloon_limit.rs`・`keyword_base.rs`・`resolver.rs` が**無改変**で済む。Requirement 5 が現行のまま素直に成立する。既存の「窓相対 offset 不変」テスト群への影響が最小。
- **難点**:
  - **Requirement 3.3（往復で誤差が累積しない）が構造的に保証されない。** `ScaleRatio` に逆数も比の合成も無いため、⑴ `unscale_coord` で旧拡大率を剥がしてから `scale_len` で新拡大率を掛ける（**2 段丸め**）か、⑵ `ScaleRatio` に比を作る API を新設するかの二択。⑴ は 96→192→96 の往復で 1px 級のドリフトが起き得る（`scale_len` は round half away from zero、`unscale_coord` は床方向の Euclid 除算＝**丸め方向が非対称**）。しかも `unscale_coord` の doc（`scale.rs:216-227`）は「寸法・長さを渡してはならない」と明記しており、オフセットへ使うこと自体が契約違反にあたる。⑵ は `scale.rs:256-260` の**明文の拒否を覆す**ことになり、`scale-exact-rational` の裁定領域に正面から触れる。**どちらの腕も既存の明文判断とぶつかる**のが本案の最大の弱点。
  - 旧拡大率の保持場所を新設する必要がある（または `refresh_scale_report` 呼出前の `applied_ratio` を旧値として使う）。
- 効果と規模: **S〜M**。危険度: **Medium**（3.3 と 7.8 が正面から難点を突く）。

### Option B: 作者空間の生値で一本化し、適用点で拡大率を掛ける（brief の有力案）

合流欄と `BalloonFollow.offset` を「**作者空間の生値**」と定義し、窓へ書く直前に拡大率を掛ける。

- **利点**: descript オフセットが**そのまま**入るので Requirement 2 が最も素直。遷移時は「掛ける拡大率が変わるだけ」で `offset` 自体を触らない＝**往復は定義上無誤差**（Requirement 3.3・7.8 が構造的に通る）。旧拡大率の保持も不要。
- **難点**:
  - **`windowposition` 由来は既に拡大率適用済みで合流している**（`mod.rs:378`）。これを作者空間へ戻すには供給層の作り直しが要る（`to_screen_adjust` から拡大率を抜き、適用点へ移す）——`windowposition-limit`／`kero-balloon` が確定させた実機実測の写像（`COMPAT:146`）を**通る道筋を変える**ことになり、Requirement 9 の非回帰範囲が広がる。
  - **ドラッグ結果とキーワード再導出は本質的に物理 px** である。作者空間へ戻す逆写像（除算）が要り、そこが lossy——Requirement 3.3 の危険が別の場所へ移動するだけになりかねない。
  - 保存値（物理 px・Requirement 5.1）と実行時表現の**単位が食い違う**ため、保存・復元の両端に換算が挟まる。Requirement 5.2「保存値を換算せずそのまま採用」との整合を、実装上どう表現するかが難しい。
  - `balloon_limit`・`persist`・`resolver` の doc がすべて「offset は物理 px」を前提に書かれており、記述の追随範囲が広い。
- 効果と規模: **L**。危険度: **High**（下流の確定済み契約への波及が最大）。

### Option C: 基準対（基準値＋基準拡大率）を持ち、遷移のたびに基準から引き直す（ハイブリッド）

`BalloonFollow` に「オフセットが確立された時点の値と、そのときの拡大率」を持たせる。表に出る `offset` は従来どおり物理 px の導出量で、遷移では**前回の結果からではなく基準から**再計算する。

- 確立点は 4 つ: 配置解決（供給層の合流結果 → `spawn.rs:482-485` で転写）・キーワード再導出（`keyword_base.rs:142-145`）・バルーン単独ドラッグ中の記憶更新（`drag_follow.rs:534-537`）・保存値の復元（`persist.rs:393-399`）。ここで基準対を焼く。**書き手はこの 4 か所しか無い**ことは実測で確認済み（`BalloonFollow.offset` への非テスト書込は 3 か所＋復元経由の 1 か所）。
- **利点**:
  - **往復が構造的に無誤差**（連鎖しないので誤差が積み上がらない）。Requirement 3.3・7.8 が定義上通る。
  - 供給元が作者空間の生値を持つケース（descript オフセット）では、基準値を作者空間のまま持てば**前方向の `scale_signed` だけ**で足り、`ScaleRatio` の新 API も逆写像も不要にできる。
  - 保存値は物理 px のまま採用し、復元時に `基準値 := 保存値`／`基準拡大率 := 復元時の拡大率` と焼けば、Requirement 5.2（換算せずそのまま採用）と 5.4（以後の遷移には Requirement 3 が効く）が**同時に、素直に**成立する。
  - `persist.rs`・`balloon_limit.rs`・`resolver.rs` は `offset` の読み手として無改変で済む（表に出る値の意味が変わらないため）。
- **難点**:
  - `BalloonFollow` が太る（`Copy` は維持できる見込み）。読み手のうち `offset` しか見ないものは影響を受けないが、**書き手**（4 つの確立点）を 1 つでも取りこぼすと基準が古いまま残り、次の遷移で静かにずれる。
  - 「補正を焼き付けない生値」（`COMPAT:169`）という既存の「生値」概念と、基準対の「基準値」という新しい概念が並ぶ。語彙の整理をしないと読み手が混乱する。
  - 基準拡大率をどちらの軸（シェル／バルーン）で持つかは Option A/B と同じく未定のまま残る（Requirement 1.4／4.4）。
- 効果と規模: **M〜L**。危険度: **Medium**。

### 推奨の下敷き（決定ではない）

- **Requirement 2（descript オフセットの拡大率適用）と Requirement 3（遷移追随）は分離して裁定できる。** 2 は供給層に `scale_signed` を 1 か所足すだけで、どの案でも同じ形になる（Option B のみ適用点が違う）。**先に 2 を確定させると 3 の選択肢が狭まらない。**
- Requirement 3.3／7.8 を額面どおり満たすことを優先するなら **Option C**、下流への波及を最小に抑えることを優先するなら **Option A**（ただし 3.3 のために `ScaleRatio` へ比の API を足す判断が要る）。Option B は brief の有力案だが、`windowposition` の確定済み写像を通る道を変える点と保存値との単位食い違いで、費用が最も大きい。

---

## 5. 規模と危険度

- **Effort: L（1〜2 週）** — 実装そのものは中規模だが、⑴ 実機観測（Requirement 6.1・8）が別セッションを要する、⑵ 決定論テストの行列（7.1「拡大率遷移 × アンカー × 保存/復元」＋ 7.6/7.7 の追加軸）が広い、⑶ 互換記録・doc の追随範囲が広い、の 3 点で伸びる。
- **Risk: Medium** — 触る関数は既に明文化された契約を大量に抱えており（`resize_window_to` だけで doc が 160 行超）、うっかり隣接契約を壊す危険が高い。一方で経路・観測・テストの donor がすべて揃っているため、未知技術の危険は無い。
- **危険の所在（上位 3 件）**:
  1. **`keyword_base.rs` の「経路で絞らない」設計との衝突**（Requirement 4.3）。ここを素朴に直すと、明文で理由づけられた設計判断を静かに反転させる。
  2. **Requirement 3.3／7.8 と `ScaleRatio` の API 不足**。2 段丸めで実装すると、テストを書いた時点で初めて落ちる。
  3. **既存の「不変」テスト群との両立**（Requirement 7.6）。`resize_window_to` が共通口である以上、追随の挿入位置しだいで寸法変化のテストが巻き添えになる。

---

## 6. 設計フェーズへの申し送り（Research Needed）

1. **SSP の拡大率跨ぎオラクル**（Requirement 6.1〜6.3）——決定論では出せない。実機で拡大率の異なるモニタ間をまたいだときの SSP のバルーン相対位置を実測する。`COMPAT:153` の既存観測（SSP の `\![move]` オフセットは物理 px 無スケール）が「SSP は何もしない」側の強い傍証であり、**観測が空振っても「何もしない」という結果として記録できる**（6.1 が明示）。
2. **`ScaleRatio` に比（または逆数）の API を足すか**——`scale.rs:256-260` が `num`／`den` アクセサの新設を**明文で拒否**しているため、これは単なる公開面拡張ではなく**先行判断の見直し**にあたる。`scale-exact-rational` の裁定（有理数配管の却下・丸め権威の据置き）に隣接する。足さずに済む設計（Option C）があることを踏まえて判断する。
3. **シェル軸とバルーン軸のどちらでオフセットを追随させるか、および揃えの残差の許容量**（Requirement 1.4・4.4）——`emo2` フィクスチャで両者が実際に異なる値になるか（`seriko.dpi` と balloon `dpi` の宣言状況）を実測し、残差の実サイズを見積もる。
4. **拡大率を placement 層へどう渡すか**（Requirement 9.5・zsp との相互確認）——⑴ `resize_window_to` の署名追加／⑵ frame 層で先に `BalloonFollow.offset` を書き換える／⑶ component で指示を渡す、の 3 案。`enqueue_window_set_pos` の署名は zsp が「触らないことを design の不変条件とする」としているため、**そこへは触れない形を選ぶこと**が前提。
5. **`COMPAT_ARCHITECTURE.md:172` の扱い**（Requirement 6.6）——`windowposition-limit` 所有の行が `balloon.offsetx/offsety` の生値加算を前提に書かれている。自分の行で上書きを明示するか、所有者へ相互確認するか。
6. **既存テストの反転範囲の確定**（Requirement 7.4・7.6）——§2.8 の (a)〜(d) 区分を設計時に `origin/main` に対して引き直し、⑴ 反転させる (a) の 2 本の**書き換え後の主張文**、⑵ 空振りの (b) 群を強い主張へ格上げするか、⑶ 表現を変える案でのみ意味が変わる (d) 末尾 2 本の扱い、を確定させる。
7. **`\![move]` と `windowposition` の分母の非対称を、本仕様が揃えるのか温存するのか**——実行時はシェル軸、起動時はバルーン軸という現状（§2.2）を、Requirement 1.4 の「供給元ごとに一意」で追認するのか、揃えるのか。揃える場合は `\![move]`（他仕様の確定事項・`COMPAT:154`）に触れることになる。

---

## 7. 要件ディスカッションへ渡す設計判断項目

> いずれも本書では決定しない。選択肢と、選択が何に効くかだけを示す。

1. **単位空間の選択**（Requirement 1.1）——実行時の合流欄を「現在の拡大率における物理 px」（Option A/C）とするか「作者空間の生値」（Option B）とするか。Requirement 3.3 の保証しやすさと、下流の確定済み契約への波及量がトレードオフになる。
2. **`balloon.offsetx`／`offsety` に掛ける拡大率の軸**（Requirement 1.4・2.1）——語彙の出所はシェル側 `descript.txt`（`config.rs:265,270`）だが、合流先の欄は `windowposition` 由来がバルーン軸で換算されて入っている（`mod.rs:395-397`）。⑴ シェル軸（語彙の出所に従う）／⑵ バルーン軸（合流欄の既存の軸に揃える）／⑶ 供給元ごとに別軸（現状の非対称を明示的な契約として確定させる）。
3. **descript オフセットの換算適用点**（Requirement 2.1）——⑴ 供給層 `apply_scope_windowpositions` の隣（`windowposition` と同じ位置・最小差分）／⑵ `config.rs` の転記層（ただし現在ここは拡大率を持たない純粋転記であり、`config.rs:276-278` が「本関数は KV の純粋転記に徹し」と明記）／⑶ 適用点＝P5（Option B）。
4. **遷移追随の変換規則**（Requirement 3.1・3.3）——⑴ 前回の結果に比を掛ける（誤差が連鎖する）／⑵ 基準対から毎回引き直す（連鎖しない・Component が太る）。Requirement 3.3 と 7.8 を額面どおり満たすなら ⑵ が構造的に有利。
5. **旧拡大率の入手方法**（Requirement 3.1）——⑴ `run_dpi_phase` で `refresh_scale_report` の**前**に `applied_ratio` を読む（新しい状態を作らない）／⑵ 前回適用値を保持する Component／Resource を新設する（遷移経路以外からも読める）。
6. **追随を差し込む位置**（Requirement 3.4・9.5）——`resize_window_to` の手順 6（`follow_balloon`）より前であることは確定として、⑴ frame 層で先に書き換える／⑵ `resize_window_to` の署名を拡張する／⑶ component で指示を渡して内側で消費する。zsp が `enqueue_window_set_pos` を触らない前提を守れる形を選ぶ。
7. **キーワード再導出との排他の実装方法**（Requirement 4.3）——`rederive_keyword_balloon_offset` は「経路で絞らない」ことを明文の設計判断として持つ（`keyword_base.rs:71-78`）。⑴ 経路で絞る（明文の判断を反転させる＝理由の書き換えが要る）／⑵ 素材の有無で分岐する（素材があれば再導出のみ・無ければ追随のみ）／⑶ 追随側に「このフレームで再導出が起きたか」を伝える。**⑵ は既存の設計判断を壊さずに排他が成立する可能性がある**ため、設計で優先して評価する価値がある。
8. **シェル／バルーンで作者基準 DPI が異なるときの残差の許容量**（Requirement 4.4）——中央揃え式が 2 本の拡大率の差に依存するため、単一拡大率での追随は必ず残差を残す。許容量を px で明示するか、比で明示するか。
9. **`applied_ratio` が `None` のときの警告水準**（Requirement 1.5・3.6・9.4）——既存 donor（`drain_resnap.rs:95-98`）は**無警告**で恒等縮退する。本仕様は警告を要求している。⑴ 本仕様の経路だけ警告する（既存 donor と非対称になる）／⑵ donor 側も揃える（他仕様の記述に触れる）。遷移経路は毎フレームではないので `warn!` の spam 危険は低い。
10. **追随の記録の出し方**（Requirement 3.7・8.3）——⑴ `transition_diag` に種別を 1 つ足す（既定 OFF・語彙表が全数を保証・実機判定ランナーと接続済み）／⑵ 常時の `info!` を 1 行足す（性能目標＝アイドル CPU 3.0% 未満への影響は遷移時のみゆえ軽微）。実機サインオフの機械判定（8.3）とどう接続するかが判断の軸。
11. **互換記録の行数と、既存行との関係**（Requirement 6.5・6.6）——3 点（単位空間契約・遷移時の変換規則・保存往復の意味論）を 1 行に畳むか 3 行に分けるか。`COMPAT:172` の含意が古くなる件をどう扱うか（自分の行で上書きを明示 ／ 所有者へ相互確認）。
12. **`\![move]` 経路（`move_window_with_route`）の扱い**（Requirement 9）——`BalloonFollow.offset` の表現を変える案（Option B/C）では、`window_move.rs:76-92` の読み手も追随が要る。本仕様の In と読むか、表現を変えないこと（Option A）で回避するか。
13. **既存の遷移テスト 2 本をどう扱うか**（Requirement 7.4・9）——`frame_dpi_reproject_tests.rs:382` と `follow_visibility_balloon_wiring_tests.rs:850` は**拡大率遷移で offset が不変であること**を積極的に主張しており、本仕様の中心的是正はこれを赤にする。⑴ 主張を「拡大率遷移では追随する／それ以外の寸法変化では不変」へ書き換える（本プロジェクトの既定路線）／⑵ 新しいテストを足して古い方を除外する。**書き換える場合も「書込前に読んだ値と突合する」構造と空振り防止の証人は必ず保つ**（同テストが `:330-358` で自ら戒めている罠）。
14. **保存形式の「版」という語の意味**（Requirement 5.1）——`sylphya.toml` には既に `format-version = 1` がある（`persist/format.rs:53`・`:111-114`）。5.1 の「保存形式に版を導入しない」は**値ごと／キーごとの版を作らない**の意と読めるが、記録に残す際は語を精密にしておかないと、後から「版はあるじゃないか」と読まれて裁定の意図が壊れる。

---

## 8. 分析の限界（本書が保証しないこと）

- 本書は**静的な読み取りのみ**で構成されている。実行時の値（`applied_ratio` が実際に返す値・遷移時の呼出順）は検証していない。
- Requirement 6.1 の SSP 観測、Requirement 8 の実機判定は、いずれも本書の範囲外（設計フェーズ以降の実機セッション）。
- `file:line` は 2026-08-27 時点。同居 4 本（pwc・zsp・bvc）が同じ木を触るため、**設計に入る前に `origin/main` へ rebase して再突合すること**（並走 brief の陳腐化は本プロジェクトで繰り返し発生している）。
