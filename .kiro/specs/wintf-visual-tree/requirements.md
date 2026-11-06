# Requirements Document

## Project Description (Input)
作りかけの描画の木（`crates/wintf/src/ecs/visual/`・外からの参照 0）を wintf の本番の描画層へ上げる。WUC の描画面の下に木を置き（開発者 2026-10-21）、節ごとに重ね方（`D2D1_COMPOSITE_MODE`）・補間・変形（回転・拡大・反転）・切り抜き（矩形・任意の図形）を効かせて 1 枚の描画面に画家の順で描く。木を持たない描画面は今と同じ。描く係・スケジュールへの配線・`WorldVisual`／`VisualVersion` を埋める system・傷 8 つの直し（COM の番地の hash・数学の二重・`GetBounds` の失敗を黙って 0 に・深さの番兵ほか）・申し送り（README の写し・`numerics` の使い手 0 の働き）・WARP の決定論のテストの足場・純粋なテスト・example 1 本・steering の描画の道の記述を含む。詳細は `brief.md`。

## Introduction

wintf の描画は今、描画面（WUC の `CompositionDrawingSurface`）1 枚につき命令の列 1 本を `SOURCE_OVER` 固定で描く。描画面の中で「部品ごとに重ね方・変形・切り抜きを変えて重ねる」口が無く、ukadoc の描画メソッドや `--clipping`・`--scaling` を D2D で描く道（`wintf-d2d-composite-rules` → `draw-methods-canon` → `draw-methods-blend`・`element-clipping-option`）が乗る土台が無い。

開発者が 2026-08〜09 に用意した描画の木の部品（`VisualDrawContent`・`VisualTransform`・`VisualClip`・`VisualFlags`・`WorldVisual`・`VisualVersion` と平らにする係 `build`）は、コンパイルはされるが本番から使われていない。本 spec は、この木を**今の WUC の描画面の下に置く**形（開発者 2026-10-21）で本番へ上げる。描画面ごとに 1 本の木を 1 枚に描き、窓や描画面どうしの重ね合わせは今の WUC の木のままにする。

併せて、起票時と見直し 2 本（`audit-wintf-graphics-world`・`audit-wintf-runtime`）が申し送った傷を直し、答えの画素を WARP（D2D を CPU の描き手で動かす）の決定論のテストで固定できる足場を作る。

この文書の「wintf」は `crates/wintf` の描画の道全体を指す。**Visual が最小の描画単位**で、木は Visual の木である（Visual より小さな単位は無い・2026-11-06 討議）。「描画面の持ち主」は自分の描画面（WUC の Visual と `CompositionDrawingSurface`）を持つ Visual、「節」は自分の描画面を持たず、いちばん近い描画面の持ち主の描画面へ D2D で描き込まれる Visual、「木」は描画面の持ち主を根とする節の集まりを指す。「新しい描き方の部品」は `VisualDrawContent`・`VisualTransform`・`VisualClip`（と可視の旗）を指す。

**到達点（2026-11-06 開発者）**: 既定は「描画面なし」で、更新が頻繁になった Visual から描画面を作る（更新の頻度で自動で昇格・降格する）。本 spec はその土台＝「描画面の有無を Visual ごとに選べる形」を作り、新しい描き方の Visual の既定を「描画面なし」にする。今の描き方の Visual の既定は変えない。自動の昇格・降格は差分の描き直し（`wintf-visual-damage-redraw`）の後の別の spec（本 spec の完了時の棚卸で起票する）。

## Boundary Context

- **In scope**:
  - 描画面の下の木を画家の順で 1 枚の描画面に描くこと（節ごとの重ね方・補間・変形・切り抜き・可視の旗）。
  - 木を持つ描画面が、今の描画の道（描画面の作成・描き直しの印・装置の失いからの立ち直り）に乗ること。
  - 毎フレーム節の世界の行列と外接矩形（`WorldVisual`）を埋め、見た目の版（`VisualVersion`）を上げる働き。
  - 傷 1〜8 と申し送り（README の写し・`numerics` の使い手 0 の働きと `moved` の単位）の直し。
  - 描画の木の画素の答えを固定する WARP の決定論のテストの足場と、数の部品・平らにする係の純粋なテスト。
  - wintf の example 1 本と、`.kiro/steering/structure.md`／`tech.md` の描画の道の記述の更新。
- **Out of scope**:
  - 前フレームとの比べと変わった範囲だけの描き直し（`wintf-visual-damage-redraw`）。
  - ブレンド（`D2D1_BLEND_MODE`）・不透明度の層・自前の合成の入れ物・1 ピクセルの読み戻し（`wintf-d2d-composite-rules`）。
  - emo の合成を描画の木へ移すこと（`draw-methods-canon`）と ukadoc の語（`blend-*`・`--clipping` ほか）の対応。
  - 文字の層（`areka-emo-text`）の描き方。
  - WUC の木の側の切り抜き（`Visual.clip`）・可視・不透明度の作り替え。
  - 途中の入れ物の精度（`compose-f16-precision`）。
  - 描画面の形式の変更。
- **Adjacent expectations**:
  - `wintf-d2d-composite-rules` は、本 spec が本番へ上げた `VisualDrawContent` の重ね方・補間の欄をそのまま使い、本 spec の WARP の足場と許容差の決めを引き継ぐ。
  - `wintf-visual-damage-redraw` は、本 spec が埋める `WorldVisual`・`VisualVersion` と平らにした描く列を使う。見た目の版は変形では上がらない約束を本 spec が守る。
  - 同じウェーブの `audit-wintf-layout-widget` は wintf の `ecs/mod.rs`・`ecs/world/mod.rs` に触らない。本 spec は `ecs::graphics::Visual`・`VisualGraphics` の名前を変えない（棚卸㉟の約束）。
  - 依存は足さない。

## Requirements

### Requirement 1: 描画面の下の木を 1 枚に描く

**Objective:** wintf を使う描き手として、描画面の持ち主の下に描画の木の節を置くだけで、木全体が画家の順でその描画面 1 枚に描かれてほしい。節ごとに重ね方・変形・切り抜きを変えて重ねる土台になるからである。

#### Acceptance Criteria

1. While 描画面の持ち主が子孫に節を持つ, when その描画面が描かれる, the wintf shall 根の自分の命令の列（在れば）を先に描き、続けて節を深さ優先の前順（親が先・兄弟は `Children` の並び順）で同じ描画面に描く。
2. The wintf shall 描画面の持ち主を、その描画面の木の根と見なし、根より下で自分の描画面を持つ Visual とその子孫を、根の木の節と見なさない。
2a. While Visual が新しい描き方の部品のいずれかを持ち、`GraphicsCommandList` を持たず、描画面を持つ目印（名前は設計で決める・例 `OwnSurface`）を持たない, the wintf shall その Visual を節とし、自分の WUC の Visual と描画面を作らない（新しい描き方の既定は「描画面なし」）。
2b. While Visual が `GraphicsCommandList` を持つ, or 新しい描き方の部品を 1 つも持たない, the wintf shall 今どおりその Visual に WUC の Visual（と、命令の列があれば描画面）を持たせる（今の描き方の既定は変えない）。
2c. While Visual が描画面を持つ目印を持つ, the wintf shall 新しい描き方の部品の有無に依らず、その Visual に WUC の Visual と描画面を持たせ、その下の木の根とする（手動の昇格の口）。
3. When 節が描く中身（`VisualDrawContent`）を持たない, the wintf shall その節自身は描かず、子孫の節は描く。
4. When 節が局所の変形（`VisualTransform`）を持たない, the wintf shall 恒等の変形として扱う。
5. When 節が切り抜き（`VisualClip`）を持たない, the wintf shall 切り抜きを加えずに描く。
6. When 節が可視の旗（`VisualFlags`）を持たない, the wintf shall 可視として扱う。
7. While 節の可視の旗が「見えない」, the wintf shall その節と子孫の節を描かない。
8. While 描画面の持ち主が命令の列を持たず節だけを持つ, the wintf shall その描画面を作り、木を描く（木だけでも画面に出る）。
9. When そのフレームの描画の前に、節の描く中身・変形・切り抜き・可視の旗が変わる, or 節が足される・外れる・親が変わる, the wintf shall 同じフレームの描画でその描画面を描き直す（次のフレームへ遅らせない）。
10. While 描画面の持ち主が子孫に節を持たない, the wintf shall 今と同じく命令の列 1 本を `SOURCE_OVER` で描き、答えの画素を変えない。
11. If 節の子孫に自分の描画面を持つ Visual（今の描き方の Visual・目印を持つ Visual）がある, then the wintf shall その旨を実体名つきで `error!` に記録し、その Visual と子孫を根の木の節と見なさない（黙って切らない）。節の下の描画面の持ち主を支えること（配置の伝播と WUC の木の同期が節をまたぐこと）は自動の昇格の spec の範囲とし、本 spec の申し送りに書く。

### Requirement 2: 節ごとの重ね方・補間・変形・切り抜き

**Objective:** wintf を使う描き手として、節ごとに重ね方（D2D の composite mode）・補間・変形（平行移動・拡大・回転・反転）・切り抜き（矩形・任意の図形）を指定し、指定どおりの画素が得られてほしい。ukadoc の描画メソッドや `--clipping`・`--scaling` を後の spec が D2D で描けるようにするためである。

#### Acceptance Criteria

1. When 節を描く, the wintf shall その節の描く中身が持つ重ね方（composite mode）と補間の指定で描く。指定しない描き手の既定は `SOURCE_OVER` と線形補間とする。
2. When 節を描く, the wintf shall 根から節までの局所の変形を順に掛け合わせた世界の変形で描く。根の変形は今の描画面の変形（DPI の倍率と描画面の中の描き始めの位置）とし、節の局所の座標は 96 DPI の DIP で与える。
3. When 節の局所の変形が回転・拡大・負の拡大（反転）を含む, the wintf shall その変形を描く中身にそのまま効かせる（回転した絵・拡大した絵・裏返した絵が得られる）。
4. When 節が矩形の切り抜きを持つ, the wintf shall その節と子孫の描画を、節の局所の座標で与えた矩形の内側に限る。
5. When 矩形の切り抜きを持つ節の世界の変形が回転・せん断を含む, the wintf shall 切り抜きを世界の座標で回転した矩形として効かせる（軸に沿う矩形に広げない）。
6. When 節が任意の図形の切り抜きを持つ, the wintf shall その節と子孫の描画を、節の世界の変形で置いたその図形の内側に限る。
7. While 祖先の節が切り抜きを持つ, the wintf shall 子孫の描画を祖先の切り抜きすべての交わりの内側に限る。
8. Where 描画面の外や祖先の切り抜きの外にしか掛からない節を描かずに飛ばす（間引き）, the wintf shall 答えの画素を間引かないときと同じにする。
9. The wintf shall 節の局所の外接矩形を 96 DPI の DIP で持ち、DPI の倍率は描画のときの変形で掛ける（DPI が変わっても節の局所の値は変わらない）。

### Requirement 3: 今の描画の道との両立

**Objective:** wintf と areka の利用者として、木を持たない描画面・WUC の木の側の振る舞い・emo の今の表示が今と同じであってほしい。描画の木の導入で既存の画面が変わらないことを確かめられるようにするためである。

#### Acceptance Criteria

1. The wintf shall 木を持たない描画面の描き方（命令の列 1 本・`SOURCE_OVER`・透明で塗りつぶしてから描く）を変えない。
2. The wintf shall 窓や描画面どうしの重ね合わせ・位置・不透明度・WUC の木の側の切り抜き（`Visual.clip`）・可視（`Visual.is_visible`）を今のまま WUC の木で行い、描画の木の部品はこれらに影響しない。
3. The wintf shall WUC の木の側の切り抜き・可視と、描画の木の側の切り抜き（`VisualClip`）・可視（`VisualFlags`）が効く場所の違い（WUC の木か、描画面の中か）を公開の型の文書に書き分ける。
4. When 装置が失われた回に木を持つ描画面を描こうとする, the wintf shall 今の立ち直りの道（残りの描画面を描かずに抜け、作り直しの後に描き直す）に木を持つ描画面も乗せる。
5. The wintf shall 本 spec の前後で、既存のウィジェットと emo の画素のテストが同じ答えを返す（全体テストが緑のまま）。
6. The wintf shall 公開の型と口の名前を areka の外の使い手にも通じる語で付け、areka の語（ゴースト・シェル・バルーンなど）を持ち込まない。
7. The wintf shall 新しい外部の依存を足さない。

### Requirement 4: 世界の行列と見た目の版を毎フレーム埋める

**Objective:** 後の spec（`wintf-visual-damage-redraw`）の作り手として、毎フレーム節ごとの世界の行列・外接矩形（`WorldVisual`）と見た目の版（`VisualVersion`）が正しく埋まっていてほしい。前フレームとの比べをこれに乗せるためである。

#### Acceptance Criteria

1. When フレームの描画の前, the wintf shall 木のすべての節について、根からの世界の行列・軸に沿うかの判定・自分の中身の世界の外接矩形・自分と子孫の世界の外接矩形の和を埋める。
2. When 節の局所の変形・親子関係・祖先の変形が変わる, the wintf shall 同じフレームでその節と子孫の世界の行列と外接矩形を埋め直す。
3. When 節の描く中身・切り抜き・可視の旗のいずれかが変わる, the wintf shall その節の見た目の版を 1 つ上げる。
4. While 節の局所の変形だけが変わる, the wintf shall 見た目の版を上げない（変形では上げない約束を守る）。
5. The wintf shall 平らにした描く 1 件の同一性（hash）に COM の部品の番地を使わず、見た目の版など番地に依らない値から取る（命令の列が捨てられ同じ番地に別の絵が作られても「同じ」と見ない）。
6. The wintf shall 平らにする係が世界の行列を自前で積み直さず、埋めた世界の行列と外接矩形を使う。

### Requirement 5: 傷と申し送りの直し

**Objective:** wintf の保守者として、起票時の傷 1〜8 と見直し 2 本の申し送りが直り、描画の木の失敗が記録なしに隠れないでほしい。本番の描画層として信頼できる作りにするためである。

#### Acceptance Criteria

1. If 描画の木を埋める・平らにする・描くどこかで D2D の呼び出しが失敗する, then the wintf shall 失敗の理由（実体・HRESULT）を `error!` で記録する（記録の無い失敗の道を作らない・panic は致命のときだけ）。
2. If 切り抜きの図形の外接矩形の取得が失敗する, then the wintf shall 1 のとおり記録し、その節と子孫を描かずに飛ばす（0 の矩形に化かさない）。
3. If 木の深さが上限（祖先の走査と同じ 1,024 段）を越える, then the wintf shall 上限を越えた旨を `error!` で 1 行記録し、それより深い節を描かずに止める（スタックをあふれさせない）。
4. If 親の輪（A→B→A）がある, then the wintf shall 3 の上限で止まり、プロセスを落とさない。
5. The wintf shall 「軸に沿うか」の判定を 1 か所にまとめ、描画の木と数の部品のどちらもその 1 つを使う。90 度の倍数の回転は軸に沿うと判じ、浮動小数の残りを許容する。
6. The wintf shall 矩形の空の判定・交わり・和・変形を数の部品（`numerics`）の 1 つの実装に寄せ、描画の木が同じ働きを自前で持たない。
7. The wintf shall NaN を含む矩形と空の矩形を、どの働きでも「空」として扱い、変形しても空のままにする。
8. The wintf shall 描画の木の部品の `unsafe impl Send/Sync` に、その根拠（工場が複数スレッド対応であること・`audit-wintf-graphics-world` が事実に合わせた注記を手本にする）を注記で書く。
9. The wintf shall 描画の木の層のモジュール名を、今の `Visual` 部品（`ecs::graphics::Visual`・`ecs::graphics::visual`）と取り違えない名前にする。`ecs::graphics::Visual`・`VisualGraphics` の名前は変えない。
10. The wintf shall 描画の木の説明書（`draw/README.md`）から型の写しを外し、型の正本はソースだけにする。
11. The wintf shall `Matrix3x2Ext::moved` が名前どおりの単位（ピクセル）で動きを判じるよう直す（単位の正方形の角だけを比べる形をやめる）。
12. The wintf shall 本番へ上げた後に残る未使用の印（`allow(dead_code)`）を外す。使い手の無い数の部品の働き（`Aabb::{contains, intersect, union, translate}` と演算子・`Point2`・`Matrix3x2Ext::moved`・`Transform2D::{pivot_srt, to_affine}`）は消さず、純粋なテストで答えを固定する（見直し 2 本の「未使用を理由に消さない」の約束）。

### Requirement 6: 決定論のテスト

**Objective:** wintf の保守者として、描画の木の画素の答えを装置の個体差に依らず固定でき、数の部品と平らにする係を装置なしで網羅できてほしい。全体テストで描画の木の退行を機械で捕まえるためである。

#### Acceptance Criteria

1. The wintf shall D2D を CPU の描き手（WARP）で動かし、描画の木を画面に出さずに絵へ描いて画素を CPU で読み戻すテストの足場を wintf のテストの中に持つ。
2. The wintf shall この足場を `tools/test-all.ps1` の全体テストで常に回る形にする（実機の環境変数や在席を要らない）。
3. When 足場で木を描く, the wintf shall 少なくとも次の答えを画素で固定する: 画家の順（後の兄弟が手前）・`SOURCE_OVER` と `SOURCE_OVER` でない重ね方 1 つ以上の違い・回転・拡大・反転・矩形の切り抜き・回転した矩形の切り抜き・任意の図形の切り抜き・入れ子の切り抜きの交わり・見えない節の飛ばし・木を持たない描画面との一致。
4. The wintf shall 画素の見本をテストのコードの中の値で持ち（見本の画像ファイルと作り直しの手順を持たない）、比べる画素は絵の境界から離れた点に限り、比べは 1 チャネルあたり ±1 の差を許す。
5. The wintf shall 描画の木の画素の答えに限って WARP を使い、実 GPU のテストは「画面に出るか」の確かめに残す。この例外を `tech.md` の Testing の節に書く。
6. The wintf shall 数の部品（`Transform2D` の合成の順と符号・`Aabb` の空・交わり・和・変形・軸に沿うかの判定）を装置なしの純粋なテストで固定する（`pivot_srt` の符号を裏返すと赤になる）。
7. The wintf shall 平らにする係の答え（描く順・切り抜きの出入りの対応・間引き・見えない節・深さの上限）を装置なしの純粋なテストで固定する。
8. The wintf shall 本 spec で足す新しいテストを、本番ファイルの中に書かず、同じディレクトリの兄弟ファイルに置く（`structure.md` のテスト分離の規約）。

### Requirement 7: example と文書

**Objective:** wintf を使う描き手として、描画の木の使い方が example 1 本と steering の記述で分かってほしい。後の spec と外の使い手が同じ形で乗れるようにするためである。

#### Acceptance Criteria

1. The wintf shall 描画の木で重ね方・回転・切り抜きを描く example を 1 本持ち、`cargo run -p wintf --example <名>` で窓に出る。
2. The wintf shall `.kiro/steering/structure.md` の `ecs/` の節に描画の木の層と描画の道（命令の列 1 本の道と木の道の分岐）を書き、`tech.md` に WARP の決定論のテストの例外を書く。
3. The wintf shall 1 ファイルを 1,000 行以内に保つ。

## 討議へ送る議題（答えで作業が変わるもの・brief の 3 件）

要件は下の推奨案で書いた。要件の討議で覆してよい。

1. **「描画面の中の節」の見分け方**（要件 1.2・1.2a〜2c・1.11）: **確定（2026-11-06 討議・開発者「承認」）**＝節も Visual（Visual が最小の描画単位）。新しい描き方の部品を持つ Visual は既定で描画面なし＝節。`GraphicsCommandList` で描く Visual と何も持たない Visual は今どおり描画面あり。描画面を持たせたい Visual には目印（例 `OwnSurface`）を付ける。到達点（既定は描画面なし・更新の頻度で自動昇格）は Introduction に書き、自動の昇格・降格は差分の描き直しの後の別 spec。当初の推奨（`Visual` 部品を持たない実体を節とする）は、brief の「WUC の Visual を持たない子」を ECS の `Visual` 部品と読み違えていたので覆した。
2. **名前の衝突の解き方**（要件 5.9）: 推奨＝`ecs::visual`（描画の木の層）のモジュール名を変える（例 `ecs::draw_tree`）。型の名前（`VisualDrawContent` など）は変えない。理由: 棚卸㉟の約束で `ecs::graphics::Visual`・`VisualGraphics` は変えない。描画の木は外からの参照 0 なので、モジュール名を変えても壊れる使い手が無い。
3. **WARP の答えのずれ**（要件 6.4）: **確定（2026-11-06 討議・推奨案）**＝見本はテストのコードの中の値（単色の小さな絵）で持ち、境界から離れた画素だけを比べ、1 チャネルあたり ±1 を許す。見本の画像ファイルと作り直しの手順は持たない。理由: 固定する答えは順・重ね方・変形・切り抜きが効いたかで、単色の数点で足りる。`wintf-d2d-composite-rules` の議題 4 もこれに揃える。
