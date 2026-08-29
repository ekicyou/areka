# 設計検証レポート: areka-P0-balloon-offset-dpi

- 対象: `.kiro/specs/areka-P0-balloon-offset-dpi/design.md`（phase: design-generated）
- 実施日: 2026-08-27
- 実施形態: 非対話（対話質問なし・判定を直接提示）
- 言語: ja（spec.json）

---

## Review Summary

中心の判断（基準対＝値とその値が属する表示 DPI を追従 Component に持たせ、遷移のたびに前回の結果ではなく基準から引き直す）は妥当であり、往復無誤差（要件 3.3／7.8）を実測ではなく構造で成立させている。`ScaleRatio` の公開面を増やさずに済む論証、`unscale_coord` を使う代替案が契約違反になる論証、挿入位置が同居 spec の不変条件（`enqueue_window_set_pos`／`resize_window_to` の署名不変）を守る論証は、いずれもコード側で裏が取れた。

一方で、**実装へそのまま渡すと壊れる／空振りする箇所が 2 件**ある。⑴ 新種別 `kind=offset` を共有パーサ `transition_judge.rs` が知らないため、既存の機械判定が全行 `UnknownKind` 欠陥で赤になる（変更対象ファイル一覧に当該ファイルが無い）。⑵ 要件 7.4 の「是正前は失敗する」対として挙げた既存テスト 2 本のうち 1 本が、そもそも DPI 遷移を一度も起こさないため是正後も緑のままで、指示どおり主張を書き換えると設計自身が戒める空振りテストになる。加えて D7 の排他の正しさ論証に片腕の穴がある。

いずれも局所であり、アーキテクチャの作り直しは要らない。設計ディスカッションで吸収可能と判断する。

---

## 事実確認（file:line で実測した主張）

本プロジェクトの規律「doc の主張は書く前に file:line で裏取り」に従い、設計の荷重のかかる主張を実測した。

### 裏が取れたもの

| 設計の主張 | 実測 | 判定 |
|---|---|---|
| `keyword_base.rs:71-81` に「経路で絞らない」の明文がある | 同範囲に逐語で存在（`old_size == Some(new_size)` の前置きコメント） | ○ |
| `keyword_base.rs:142-145` が `follow.offset` へ書く | `let old_offset = follow.offset; ... f.offset = new_offset;` を確認 | ○ |
| `drag_follow.rs:534-537` が `follow.offset` へ書く | 同範囲に `follow.offset = PointPx { ... }` | ○ |
| `BalloonFollow` は `drag_follow.rs:30-35` 定義・`balloon`／`offset` の 2 欄・両方 `pub` | 一致 | ○ |
| `follow.rs:62` が `pub use self::drag_follow::BalloonFollow;`、`:103` が `follow_balloon` を私有再束縛 | 一致（`pub(crate) use` への格上げが必要という設計の指摘も正しい） | ○ |
| `spawn.rs:482-485` が `BalloonFollow { balloon, offset }` のリテラル構築 | 一致（欄追加で必ずコンパイルエラーになる＝D14 の主張は成立） | ○ |
| D14「私有化すれば `drag_follow`／`keyword_base` の直接代入がコンパイルエラーになる」 | 成立。`offset_space` は両者の**兄弟モジュール**であり、無指定可視性の欄は届かない | ○ |
| `windowposition.rs:191-197` に「注意（単位空間の混在・意図的）」、`:214-215` に実装コメント | 一致 | ○ |
| `windowposition.rs:133` の `scale_signed`（`pub(crate)`・`scale_len` 委譲・符号保存・`±i32::MAX` 飽和） | 一致 | ○ |
| `windowposition.rs:216-219` の合流が `saturating_add` | 一致 | ○ |
| `scale.rs` の「座標専用（長さの縮約には使わない）」doc | 一致（`unscale_coord` をオフセットへ使うのが契約違反という論証は正しい） | ○ |
| `scale.rs` の「`num`／`den` アクセサは新設しない」申し送り | 一致（W6.5 との名前二重化回避の明文あり） | ○ |
| `ScaleRatio` の公開面＝`ONE`／`new(u32,u32)->Option`／`is_identity`／`scale_len` | 一致（`mul`・`as_f32`・`scaled_extent`・`unscale_coord` も公開だが本仕様は使わない） | ○ |
| `frame_dpi_reproject_tests.rs:443` が表示 DPI 2 つから `ScaleRatio::new` を組む先例 | 一致（`ScaleRatio::new(to_dpi.into(), from_dpi.into())`） | ○ |
| `monitor_systems.rs:534-536` が `*dpi = new_dpi` で旧値を捨てる | 一致（`crates/wintf/src/ecs/layout/systems/monitor_systems.rs`） | ○ |
| `frame/dpi.rs:329-332` の「`None` は k 不変と同義でない」doc、`:335` の `refresh_scale_report` | 一致 | ○ |
| `window_move.rs:337-345` のべき等 skip が `return false`（D16 の前提） | 一致 | ○ |
| 行数の番人が例外表と完全一致を要求し、`window_move.rs` は既に表に載る | 一致（`OVER_LIMIT_ALLOWED` に当該パス・`OVER_LIMIT_ALLOWED_COUNT = 11` の逐語二重化あり） | ○ |
| `config.rs:264-274` の `cascade2(ghost_kv, shell_kv, ...)`・`:276-278` の「KV の純粋転記に徹し」 | 一致 | ○ |
| `mod.rs:207-213` の `AuthorDpi { shell, balloon }`、`:395-397` の「`windowposition` はバルーン軸」裁定 | 一致 | ○ |
| `prepare_stages` に `scaling`／`scope_ids` があり `apply_scope_windowpositions` の直前へ挟める | 一致（`mod.rs:378` の直前が挿入点） | ○ |
| `persist.rs` の保存値採用腕 `(Some(x), Some(y))` | 実際は `:396`（設計は `:393`・3 行ずれ） | ○（軽微ずれ） |
| D5 の約分（実行時 k ∝ 表示 DPI） | `derive_scale` は `app_scale × (窓 DPI ÷ author_dpi)`。`app_scale` は非テストコードのどこにも配線が無く常に恒等 ⇒ 遷移比は表示 DPI 比に等しく、軸の選択は生じない | ○ |
| `hello-pasta` が `balloon.offsetx` を実宣言している | `vendors/pasta/crates/pasta_sample_ghost/ghosts/hello-pasta/shell/master/descript.txt:7-10` に `sakura.balloon.offsetx,64`／`kero.balloon.offsetx,64`（`release/` 側にも同一）。`seriko.dpi` は未宣言＝シェル作者 DPI は 96 | ○ |
| `COMPAT_ARCHITECTURE.md` §8 が 122 行目から始まり、`:146`（`windowposition.x`）・`:154`（`\![move]`）・`:169`（焼き付けない）・`:172`（キーワード＋調整量）が設計の説明どおり | 一致 | ○ |
| `transition_diag.rs` の 4 種別＋`PLACEMENT_KIND_ALL`、`kind=monitor` は `wintf` 側 `window/transition_diag.rs:67` にあり判定器が両者を 1 本の時系列で読む | 一致（起点を流用できるという主張は成立） | ○ |
| 行数（`transition_diag.rs` 617／`frame/dpi.rs` 482／`drag_follow.rs` 912／`mod.rs` 694／`transition_judge.rs` 929／`transition_judge_verdict.rs` 863） | すべて一致 | ○ |

### 食い違ったもの（軽微・記述の是正で足りる）

| 設計の記述 | 実測 | 影響 |
|---|---|---|
| design.md:224「`build_measure_scaling` が実際に**分母**へ取った値——`primary_dpi`」 | `ScaleRatio::new(dpi, u32::from(author))`＝**primary_dpi は分子・author_dpi が分母**（`mod.rs:302`・`measure.rs:58` の doc「primary モニタ DPI ÷ 各 author_dpi」） | 運ぶ値（`primary_dpi` または `FALLBACK_PRIMARY_DPI`）の特定は正しいので実装は落ちない。ただし語が反転しており、実装者が「採寸 DPI＝窓の DPI」と読み違える余地を残す。**採寸 DPI は起動時の主モニタ DPI であって窓が載るモニタの DPI ではない**——この一文を本文へ足すこと |
| `window_move.rs`（1,228 行）／`windowposition.rs` 436 行 | 1,227 行／435 行 | 各 1 行ずれ。判断には影響しない |
| D14「書込は `new()` と `reestablish()` の **2 本**に閉じる」 | 同じ設計の Service Interface が `anchor_base_dpi`／`apply_rescaled` を加えた **4 本**の書込メソッドを列挙 | 「確立点は 2 本・追随相の書込が 2 本」と読めば矛盾はないが、D14 が不変量として書いている文と本文が食い違う。語を揃えること |
| 「依存の向き（左から右へのみ許す）: `ScaleRatio` → `follow/offset_space.rs` → `placement`（resolver／persist／…／windowposition）」 | `offset_space` は `resolver::PointPx` と `windowposition::scale_signed` に依存し、`resolver` は `offset_space::OffsetBase` に依存する＝宣言した向きに対して**双方向**。同一 crate ゆえコンパイルは通る | 宣言した不変量を設計自身が破っている。向きの記述を「型の定義元は `resolver`（`PointPx`）・変換規則の定義元は `offset_space`」のように正確化すること |

---

## Critical Issues

### Issue 1: 新種別 `kind=offset` を共有パーサが知らず、既存の機械判定が全行赤になる（変更対象の取りこぼし）

- **深刻度**: 高（実装すると既存テストと実機サインオフ判定が壊れる）
- **要件**: 3.7・8.3・9.6（および先行 spec の実機判定資産）
- **確認した事実**:
  - `crates/areka/src/placement/transition_judge.rs:374-388` の `required_fields(kind)` は既知の 10 種別を並べ、**未知の種別語は `None` を返す**。
  - `:470-483` の呼び手は `None` のとき `RecordDefect::UnknownKind(kind)` を積む。
  - `transition_judge_test_support.rs:231-238` の `parse_ok` は `is_well_formed()` を `assert!` する。`transition_judge_reobservation_tests.rs:118-131` の `every_embedded_record_is_well_formed` は埋め込みログ全行に同じ検査を掛け、doc に「語彙が変わって必須フィールドが増減すれば、この 1 本が真っ先に赤くなる」と明記している。
  - 設計の Modified Files に **`transition_judge.rs` が無い**。D17 は「判定モジュールを既存の `transition_judge.rs`（929 行）／`transition_judge_verdict.rs`（863 行）へ足さず新設する」と、行数を理由に接触しない方針を明言している。
- **何が起きるか**: `kind=offset` 行が `wintf::transition` の同一ストリームへ出た瞬間、その行は `UnknownKind` 欠陥を持つ。実機サインオフのログにも、埋め込みログを更新すれば決定論テストにも入り込み、既存の遷移判定（atom／present-write-coherence の資産）が赤になる。要件 8.3 の「合否を記録の機械判定で決める」も、起点となるパーサが自らの新種別を欠陥扱いする状態では成立しない。
- **なぜ D17 の理由付けでは解けないか**: 行数の心配は不要である。`required_fields` へ 1 アーム＋`OFFSET_FIELDS` 定数を足しても `transition_judge.rs` は 929 → 940 行程度で上限 1,000 に触れない。**判定ロジック**を新モジュールへ分離することと、**共有パーサに語彙を教える**ことは別の話であり、設計は両者を同一視している。
- **推奨対応**:
  1. Modified Files へ `crates/areka/src/placement/transition_judge.rs` を追加し、`required_fields` の `KIND_OFFSET => Some(OFFSET_FIELDS)` アームと定数だけを足す（判定ロジックは `transition_judge_offset.rs` のまま）。
  2. `transition_judge_reobservation_tests.rs` の埋め込みログを更新する必要があるか（＝再観測ログに `kind=offset` が現れるか）を設計時点で明言する。
  3. 「発行側の語彙を増やしたら共有パーサの語彙表も同時に増やす」を Revalidation Triggers へ 1 行足す。

### Issue 2: 要件 7.4 の「是正前は失敗する」対の片方が、DPI 遷移を一度も起こさないテストである

- **深刻度**: 中〜高（指示どおり実装すると空振りテストが生まれ、要件 7.4 の対が半分崩れる）
- **要件**: 7.4・7.9
- **確認した事実**:
  - D13 と §Testing「既存テストの書き換え」は、`frame_dpi_reproject_tests.rs:382` と `follow_visibility_balloon_wiring_tests.rs:850` の 2 本を「現行契約の正確な写しであり、赤になるのが正しい＝要件 7.4 の是正前に失敗する側が既に書かれている」と位置づけ、両方の主張を「拡大率遷移では表示 DPI 比で追随する」へ改めよと指示している。
  - 前者は妥当。`frame_dpi_reproject_tests.rs` は `world.entity_mut(e).insert(DPI::from_dpi(to_dpi, to_dpi))`（`:450`）で実際に DPI を書き換え、`(96,120)/(96,192)/(120,192)` を回し、`:381-382` で「窓相対の追従 offset が値ごと不変」を主張している ⇒ 是正後は正しく赤になる。
  - 後者は**該当しない**。`follow_visibility_balloon_wiring_tests.rs` には `DPI::` の書き換えが 1 か所も無い。当該テスト `balloon_follows_the_guarded_char_position_not_the_raw_projection`（`:850`）は `for dpi in DPIS` で**各 DPI ごとに世界を組み直し**、`resize_window_to` を直接呼んで clamp 後のキャラ位置へバルーンが追従することを見る。DPI 遷移（`Changed<DPI>`）は発生せず、`dpi_phase_with` も通らない。`:923-928` の恒等式（`stored_offset == offset`）は、本設計の発火条件（frame 層の `Changed<DPI>` のみ）では**是正後も緑のまま**である。
  - 性質としては、設計自身が「変更しない」に分類した `follow_resize_tests.rs:176/:261/:476` 群（寸法変化に対する不変）と同型である。
- **何が起きるか**: 指示どおり主張を「遷移では表示 DPI 比で追随する」へ書き換えると、遷移を起こさないテストが遷移を主張することになる——設計自身が `:330-358` を引いて戒める「恒等式を、それを作った当人に問う」空振りの、別種の形になる。書き換えずに放置すれば、是正⑵の対は `frame_dpi_reproject_tests.rs:382` の 1 本に細る。
- **推奨対応**:
  1. `follow_visibility_balloon_wiring_tests.rs:850` を「**変更しない**」群（寸法変化に対する不変を主張する群）へ移し、テスト doc に「本檻は遷移を起こさない＝追随の証拠にならない」を明記する（設計が `frame_dpi_reproject_tests.rs:273` ほか 4 本へ課したのと同じ扱い）。
  2. 是正⑵の対を作り直す。`frame_dpi_reproject_tests.rs:382` の書き換え 1 本に加え、`frame_balloon_offset_follow_tests.rs` 側で「遷移前後で offset が表示 DPI 比になる」を主張する新規テストを**是正前に赤になる側として明示**する（追随の実装が無ければ旧値のままで落ちるので、⑶と違って⑵は新規でも対が成立する）。

### Issue 3: D7（キーワード再導出との排他）の「どちらの腕でも見送りが正解」の論証に片腕の穴がある

- **深刻度**: 中（狭い条件だが、要件 4.2 の主張が構造では成立しない）
- **要件**: 4.2・4.3
- **確認した事実**:
  - D7 の腕 2 は「素材があって再導出が発火しない場合（寸が変わらなかった）＝物理寸が変わっていないなら中央揃えの幾何も変わっていないので、既存 offset がそのまま正しい」と論じている。
  - しかし D8 が書くとおり中央揃え式は `char_x + (char_w − balloon_w) ÷ 2` であり、**キャラ寸とバルーン寸の 2 つ**に依存する。再導出の発火条件はキャラ窓の寸のみ（`keyword_base.rs` の `if old_size == Some(new_size) { return; }`）であり、シェルとバルーンで作者基準 DPI が異なる／丸めが異なる場合、キャラ寸が据え置きでもバルーン寸は動き得る。
  - 設計自身が引く `frame/dpi.rs:329-332` は「k は変わったが丸め後の物理寸が同じ」場合に `refresh_scale_report` が `None` を返すと明記しており、この腕が空虚でないことを設計の中の事実が裏付けている。
- **何が起きるか**: この腕へ入ると、追随は `keyword-pending` で見送り（offset も基準も触らない）、再導出も寸不変で早期 return（素材を消費しない）。結果として基準 DPI は旧値のまま固定され、以後の遷移でも同じ腕を通り続けるため、**次に寸が動くまで自己解消しない**。要件 4.2（遷移後も中央へ揃っている）が破れる。
- **緩和材料**: 実運用では素材は起動直後の実表示寸確定で消費されるため、モニタ移動時に素材が残っている確率は低い。また `verdict=keyword-pending` が記録に残るので沈黙はしない。
- **推奨対応**（いずれか）:
  - (a) D7 の腕 2 の論証を「キャラ寸とバルーン寸の**両方**が不変なら正しい」へ正確化し、片方だけ動く場合を残余として要件 4.4 の許容量へ明示的に含める（＝開発者裁定として登記する）。
  - (b) 排他の条件を「素材の有無」から「素材があり、かつ**この遷移で再導出が発火する**」へ狭め、発火しないと分かった腕では通常の追随を行う（`keyword_base` の発火条件は変えず、追随側が同じ述語を読む）。
  - いずれにせよ、`keyword-pending` が自己解消しない経路が存在することを Open Questions／Risks へ 1 行残すこと。

---

## その他の指摘（Critical ではないが設計ディスカッションで拾うべき）

1. **要件 3.4 と D16 の緊張**。要件 3.4 は「追従の適用によって遷移中の窓書込の回数を増やさない」と書くが、D16 の収束は べき等 skip 腕でバルーン書込を **0 → 1** へ増やす。設計は「通常時 2・skip 時 1 で合計は増えない」と**別の腕と比較して**説明しており、要件の字面に対しては正面から答えていない。先行仕様から引き継いだ予算（キャラ ≤1・バルーン ≤1・別経路 0）は守られるので実害は無いが、「要件 3.4 は総回数ではなく予算 ≤1/≤1/0 として読む」ことを**裁定として登記**し、要件側の注記か設計の Boundary へ明記すること（本プロジェクトの規律「裁定で要件を改訂したら design・境界節まで追随」）。
2. **採寸 DPI の意味の明記**（上の食い違い表を参照）。`ScopePlacement.balloon_offset_base.dpi = Some(採寸 DPI)` の採寸 DPI は起動時の主モニタ DPI である。結果として、非主モニタで生まれた窓は最初の `Changed<DPI>` で主モニタ空間から実モニタ空間へ引き直される——これは望ましい挙動だが、設計本文にその一文が無い。実装者が「窓の DPI」を刻む誤りを塞ぐため明記すること。
3. **`transition_judge.rs` 以外の陳腐化リスク**。`persist.rs:393` は実際には `:396`、`window_move.rs` は 1,227 行、`windowposition.rs` は 435 行。設計 doc の file:line は着手前に一度 rebase 突合すること（本プロジェクトで陳腐化は通算 8 度踏んでいる）。
4. **`drag_follow` の `reestablish` に現在 DPI をどう渡すか**が未記述。`on_balloon_drag` はキャラ窓の `BalloonFollow` を `get_mut` した状態で書くため、同一 `World` から `DPI` を読む順序（先に読んでから借りる）を実装ノートへ 1 行足すと迷いが消える。
5. **`OffsetBase` の型の置き場所**。`PointPx` は `resolver` 由来ゆえ、`offset_space` を「`World` を持たない純粋モジュール」と宣言しつつ `resolver` へ依存する形になる。依存の向きの記述を正確化すること（上の食い違い表 4 行目）。

---

## Design Strengths

1. **基準対（Anchored Base Pair）が往復無誤差を実測ではなく構造で成立させ、かつ先行仕様の裁定領域を回避している**。前回の結果へ比を掛ける素直な案が採れない理由——`ScaleRatio` に逆数が無く、`unscale_coord` は doc（`scale.rs`「座標専用（長さの縮約には使わない）」）が長さへの適用を明示的に禁じている——を実コードで確認したところ、設計の論証どおりだった。基準から毎回引き直す方式は `ScaleRatio::new`／`scale_len`／`is_identity` という**既存公開面だけ**で閉じ、`scale.rs` が W6.5 `scale-exact-rational` へ向けて明文で拒否している `num`／`den` アクセサ新設に一切触れない。D5 の約分（遷移比では作者基準 DPI が消える）も `derive_scale` の実装で裏が取れ、要件 4.4 の「どちらの軸を用いるか」という問いを**問い自体が消える**形で解いている点が特に良い。

2. **D14 の「私有欄＋確立点を閉じる」が、この方式最大の危険を宣言ではなく型で潰している**。基準対方式の失敗様式は「書き手を 1 つ取りこぼすと基準が古いまま残り、次の遷移で静かにずれる」であり、これは決定論テストでも実機でも捕まえにくい。`offset_space` を `drag_follow`／`keyword_base` の**兄弟モジュール**に置けば私有欄への直接代入がコンパイルエラーになることを Rust の可視性規則で確認した。構築側も欄追加で `spawn.rs:482-485` のリテラルが必ず落ちる。要件 4.3 の排他を「経路で絞らない」既存の設計判断を**反転させずに**新規コード側の分岐で成立させた D7 の組み立ても、同じ「既存の明文を書き換えずに目的を達する」流儀で一貫している。

3. **同居 spec との衝突面を先回りして潰している**。D6 が挿入位置を frame 層（`refresh_scale_report` の直前・第 2 巡）に取ることで、`enqueue_window_set_pos`／`resize_window_to`／`move_window_with_route` の署名を 1 つも変えずに「手順 6 より前」という制約を満たす。`scope-zorder-pinning` が design の不変条件として宣言した funnel に触れないことを Boundary Commitments で明示しており、`follow/window_move.rs` への変更を「行数を増やさない呼び替えのみ」に留める判断も、行数の番人（`OVER_LIMIT_ALLOWED_COUNT = 11` の逐語二重化を実測確認）と整合する。

---

## Final Assessment

### 判定: **GO（条件付き）**

**根拠**:

- 中心の判断（基準対・純関数への分岐集約・frame 層の適用相）は、荷重のかかる主張をコード側で実測した限りすべて成立している。方式の作り直しは不要であり、要件 1〜5・9 の主要部は設計として実装可能な粒度まで降りている。
- 一方で、Issue 1 と Issue 2 は**設計文書のまま実装すると確実に壊れる／空振りする**種類であり、タスク生成前に必ず解消しなければならない。どちらも局所（前者はファイル 1 本の追加と match アーム 1 本、後者はテスト対の組み直し）であり、設計ディスカッションで吸収できる範囲である。
- Issue 3 は判断の分かれ目（正確化 か 条件の見直し か）であり、開発者の裁定を要する。

### タスク生成前に必須（設計ディスカッションで解決すること）

1. **Issue 1**: `transition_judge.rs` を Modified Files へ追加し、共有パーサの語彙表へ `KIND_OFFSET`／`OFFSET_FIELDS` を足す方針を明記する。埋め込み再観測ログの更新要否も明言する。
2. **Issue 2**: `follow_visibility_balloon_wiring_tests.rs:850` を「変更しない」群へ移し、是正⑵の「是正前に失敗する」対を組み直す。
3. **Issue 3**: D7 腕 2 の論証を正確化するか、排他の条件を狭めるかを裁定する。

### 望ましい（同時に直せば安い）

4. design.md:224 の分母／分子の反転を是正し、「採寸 DPI＝起動時の主モニタ DPI」を本文へ明記する。
5. 要件 3.4 と D16 の関係を「予算 ≤1／≤1／0 として読む」と裁定・登記する。
6. D14 の「書込 2 本」と Service Interface の 4 メソッドの語を揃える。
7. 依存の向きの記述を、`PointPx`（`resolver` 由来）と `scale_signed`（`windowposition` 由来）への依存が実在する形へ正確化する。
8. file:line（`persist.rs:393`→`:396`、`window_move.rs` 1,228→1,227、`windowposition.rs` 436→435）を実測値へ更新する。

### 次のフェーズ

上記 1〜3 を設計ディスカッション（`/kiro-design-discussion areka-P0-balloon-offset-dpi`）で解決し、design.md へ反映したうえで `/kiro-spec-tasks areka-P0-balloon-offset-dpi` へ進むこと。
