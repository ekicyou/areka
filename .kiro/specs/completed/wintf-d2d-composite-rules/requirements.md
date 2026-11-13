# Requirements Document

## Project Description (Input)
wintf で、描くものごとに D2D の合成の規則（重ね方・ブレンド・不透明度）を指定して 1 枚の描画面に重ねられるようにする。描画の木（`wintf-visual-tree`・11-06 着地）が節ごとの重ね方（`D2D1_COMPOSITE_MODE`）・補間・変形・切り抜きをもう描いているので、本 spec に残るのは、ブレンド（`D2D1_BLEND_MODE` の 26 種）・節の不透明度の層・層の旗（切り抜きと層の中で重ね方とブレンドが何に効くか）・合成の結果を持つ wintf の自前の入れ物と描画面への写し・入れ物からの同期の 1 ピクセルの読み戻し・それらの WARP の決定論のテスト。出どころは開発者 2026-10-15「wintfではコマンドリストの合成ルールが現在固定ですが、これをD2D合成ルールを個別に指定できるようにして、高度な合成を実現したい」「（当たり判定は）座標さえわかれば、１ピクセルだけGPUから書き戻すようなことはできないか？」。指定しない描き手は今と同じ `SOURCE_OVER` のまま。ukadoc の語は wintf に持ち込まない（語との対応は `draw-methods-canon`・`draw-methods-blend`）。詳細は `brief.md`（末尾の 2026-11-13 棚卸㊱の再測定が正本）。

## Introduction

wintf の描画は、描画面（WUC の `CompositionDrawingSurface`）1 枚につき、根の命令の列 1 本を `SOURCE_OVER` で描き、その下の描画の木の節を画家の順に重ねる（`wintf-visual-tree`）。節は重ね方（`D2D1_COMPOSITE_MODE`）・補間・変形・切り抜きをもう選べる。しかし次の 3 つが足りない。

- **混ぜ方**: ukadoc の `blend-*`（Photoshop・CLIP STUDIO PAINT のレイヤーの合成）を D2D で描くためのブレンドの口が無い。節の不透明度（`Visual.opacity`）は 1.0 でなければ `warn!` を残して 1.0 で描いている。層は透明から始まる固定で、図形の切り抜きの中では `SOURCE_OVER` でない重ね方が、その下に描かれた画素に効かない（矩形の切り抜きの中では効く＝答えが揃っていない）。
- **合成の結果の置き場**: 描画面は直接は読み戻せない。合成の答えを確かめる口も、変わった範囲だけ描き直す後の spec（`wintf-visual-damage-redraw`・WUC の描画面は部分の `BeginDraw` を断る）の描き先も無い。
- **1 点の答え**: 合成を GPU へ移す後の spec（`draw-methods-canon`）で CPU の画素が無くなると、クリックの透過と当たり判定（`hit-id-pass`）が「その座標の α」を知る出どころが無くなる。

本 spec は、この 3 つを wintf の口として足す。合成の結果は wintf の自前の入れ物に持ち、そこから描画面へ写す。入れ物から 1 画素をその場で読み戻せる（前のフレームの値を使い回さない＝「1 フレーム遅らせる解は取らない」）。合成の答えの画素は、D2D を CPU の描き手（WARP）で動かす決定論のテストで固定する（開発者 2026-10-15「画像合成に関するテストは、CPUに切り替えるべき」）。

この文書の語は `wintf-visual-tree` の要件と同じ。**Visual が最小の描画単位**で、「描画面の持ち主」は自分の描画面を持つ Visual、「節」は自分の描画面を持たず、いちばん近い描画面の持ち主の描画面へ描き込まれる Visual、「描く中身」は節の `VisualDrawContent` を指す。「入れ物」は、描画面 1 枚ごとに wintf が持つ合成の結果の絵（GPU の上のテクスチャ）を指す。「下の画素」は、同じ描画面へそれより前に描かれた画素を指す。

## Boundary Context

- **In scope**:
  - 描く中身ごとのブレンド（D2D のブレンドの 26 種）。
  - 節の不透明度を、節と子孫をまとめた 1 枚に掛けて重ねること（不透明度の層）。
  - 切り抜きと不透明度の層の中で、`SOURCE_OVER` でない重ね方とブレンドが何に効くかの決め（層の旗）。
  - 描画面ごとの自前の入れ物（合成・描画面への写し・作り直し・中身の保持）。
  - 入れ物からの同期の 1 ピクセルの読み戻し（色と α）。
  - 上の全部の WARP の決定論のテストと、wintf の既存のテストの更新。`tech.md`・`structure.md` の描画の道の記述の更新。
- **Out of scope**:
  - ukadoc の語（`overlay`・`reduce`・`blend-*`・`-fast` 付き・`--alpha`・`--clipping`・`--scaling` ほか）と wintf の規則の対応、element定義・pattern定義の読み、emo の合成の段の作り替え（`draw-methods-canon`・`draw-methods-blend`・`element-clipping-option`）。
  - D2D に無いブレンド 3 種（`blend-dither`・`blend-add-glow`・`blend-color-dodge-glow`）の自前の効果（`draw-methods-blend`）。
  - α を無視して描く絵（`asis`）の作り方。絵の側の α の扱いで作れるので wintf の合成の口には足さない（`draw-methods-canon`）。
  - 元の矩形の指定（`--clipping`）。`wintf-visual-tree` の矩形の切り抜きと変形で描けるので、新しい口を足さない。
  - 当たり判定の ID の描き先と ID の読み戻し（`hit-id-pass`）。当たり判定の名前の解決（emo の `hit.rs`）。
  - 入れ物の形式の f16 化と途中の精度（`compose-f16-precision`）。
  - 変わった範囲だけの描き直し（`wintf-visual-damage-redraw`）。本 spec は描画面を毎回全部合成する。
  - 描画面の自動の昇格・降格（`wintf-surface-auto-promotion`）。
  - 文字の層（`areka-emo-text`）。自前のスワップチェーンで描き、本 spec の入れ物と合成の口を通らない。
  - サーフェスの切り替えのクロスフェード（`seriko.force_disable_crossfade` の対象）。
  - CPU の画素を読んでいる 3 つの読み手（当たり判定の α マスク・MCP の `dump_surface`・バルーンの背景色）の出どころの付け替え。本 spec の後も emo は CPU で合成するので、3 つは今のまま動く（付け替えは CPU の画素が無くなる `draw-methods-canon`。`ecs/widget/bitmap_source/alpha_mask.rs`・`ecs/layout/hit_region/`・emo-present の `presenter/budget.rs`・`cache.rs` には触らない）。
- **Adjacent expectations**:
  - `wintf-visual-tree` が本番へ上げた描画の木（`VisualDrawContent` の重ね方と補間の欄・`Visual.opacity`・`Visual.clip`・目印 `OwnSurface`・WARP の足場と許容差）をそのまま使う。重ね方・補間・変形・切り抜きの答えは変えない。
  - `wintf-visual-damage-redraw` は、本 spec の入れ物を描き先にして、変わった範囲だけ描き直す。本 spec は入れ物の中身を次の合成まで保つ（要件 4.4）。
  - `hit-id-pass` は、当たったかどうかを本 spec の 1 ピクセルの読み戻しの α で決める。
  - `draw-methods-canon` は、本 spec の口で emo の合成を D2D へ移し、そのとき CPU の画素の読み手を付け替える。
  - 同じウェーブ（C19）の約束: 本 spec は areka の本体（`crates/areka/`）に触らない。依存は足さない。

## Requirements

### Requirement 1: 描く中身ごとのブレンド

**Objective:** wintf を使う描き手として、描く中身ごとに D2D のブレンドの種類を指定し、下の画素とその種類の式で混ぜた画素が得られてほしい。ukadoc の `blend-*` を後の spec が D2D で描けるようにするためである。

#### Acceptance Criteria

1. Where 描く中身がブレンドの種類を指定する, the wintf shall その描く中身の絵を、下の画素とその種類の式で混ぜて描く。
2. The wintf shall D2D のブレンドの 26 種（multiply・screen・darken・lighten・dissolve・color burn・linear burn・darker color・lighter color・color dodge・linear dodge・overlay・soft light・hard light・vivid light・linear light・pin light・hard mix・difference・exclusion・hue・saturation・color・luminosity・subtract・division）のすべてを指定できるようにする。
3. When ブレンドを指定した描く中身を描く, the wintf shall その絵の外形（局所の外接矩形を世界の変形で置いた範囲）の外の画素を変えない。
4. Where 描く中身がブレンドの種類を指定しない, the wintf shall 今どおり重ね方（既定は `SOURCE_OVER`）と補間の指定で描き、答えの画素を変えない。
5. The wintf shall 1 つの描く中身にブレンドと重ね方の両方を同時に効かせない（どちらか 1 つで描く）。
6. When ブレンドを指定した描く中身が変形（回転・拡大・反転）や祖先の切り抜きの下にある, the wintf shall 重ね方で描くときと同じく、変形と切り抜きを効かせたうえで混ぜる。
7. If 動いている Windows が、選んだブレンドの描き方を持たない, then the wintf shall その旨を `error!` に 1 度記録し、その描く中身を `SOURCE_OVER` で描く（プロセスを落とさない・黙って描き落とさない）。
8. When 描く中身のブレンドの指定が変わる, the wintf shall 同じフレームの描画でその描画面を描き直す（ブレンドの指定の変化を見た目の変化として扱う）。

### Requirement 2: 節の不透明度の層

**Objective:** wintf を使う描き手として、節の `Visual.opacity` を 1.0 未満にすると、その節と子孫がまとめて半透明に重なってほしい。ukadoc の `--alpha` を後の spec が D2D で描けるようにし、描画面の持ち主（WUC）と同じ性質を節でも実現するためである。

#### Acceptance Criteria

1. While 節の不透明度が 0 より大きく 1.0 未満, the wintf shall その節と子孫の節を、まず 1 枚にまとめて描き、その 1 枚に不透明度を掛けて下の画素へ重ねる（重なり合う子孫どうしが互いに透けて見えない）。
2. While 節の不透明度が 0, the wintf shall その節と子孫の節を描かない（答えの画素は描かないときと同じ）。
3. While 節の不透明度が 1.0, the wintf shall 今と同じ答えの画素で描く（まとめた 1 枚を作らない描き方と同じ）。
4. The wintf shall 節の不透明度を、描画面の持ち主と同じく `Visual` の範囲に収めた値（`clamped_opacity` の答え・0〜1 の外の扱いも同じ）で効かせる。
5. While 祖先の節も 1.0 未満の不透明度を持つ, the wintf shall 祖先と子孫の不透明度を入れ子の層として順に効かせる。
6. The wintf shall 節の不透明度が 1.0 でないときに残していた「範囲外なので 1.0 で描く」旨の `warn!` を残さない。
7. When 節の不透明度が変わる, the wintf shall 同じフレームの描画でその描画面を描き直す。
8. If 不透明度に NaN が与えられる, then the wintf shall 共通の 1 か所（`set_opacity`・`clamped_opacity`）で NaN を 1.0（不透明）として扱い、その旨を `warn!` に記録する（描画面の持ち主と節の両方に効く・黙って消さない）。

### Requirement 3: 切り抜きと層の中で重ね方とブレンドが効く先

**Objective:** wintf を使う描き手として、切り抜きの形（矩形・角丸・図形）によって `SOURCE_OVER` でない重ね方とブレンドの答えが変わらないでほしい。切り抜きは描く範囲を限るだけで、何に混ぜるかを変えないと分かれば、後の spec が ukadoc の重ね方を切り抜きと組み合わせて描けるからである。

#### Acceptance Criteria

1. When 切り抜き（矩形・角丸・任意の図形）の内側の描く中身が `SOURCE_OVER` でない重ね方かブレンドで描かれる, the wintf shall 切り抜きの内側では、切り抜きが無いときと同じく下の画素に対して効かせる。
2. When 切り抜きの内側の描く中身を描く, the wintf shall 切り抜きの外側の画素を変えない。
3. The wintf shall 1・2 の答えを、矩形・角丸・回転した矩形・任意の図形のどの切り抜きでも同じにする。
4. While 描く中身が不透明度の層（要件 2）の内側にある, the wintf shall その描く中身の重ね方とブレンドを、同じ層の中で先に描かれた画素に対して効かせ、層の外の下の画素は、層を重ねるときの不透明度でだけ変える。
5. The wintf shall 1〜4 の決めを公開の型の文書に書く（今の型の文書にある「層は透明から始まる・変えるのは本 spec」の注記を、決めた答えに置き換える）。

### Requirement 4: 自前の合成の入れ物と描画面への写し

**Objective:** wintf を使う描き手と後の spec の作り手として、描画面の合成の結果を wintf が自前の入れ物に持ち、そこから描画面へ写してほしい。合成の答えを読み戻せ、後の spec が変わった範囲だけ入れ物へ描き直せるようにするためである。

#### Acceptance Criteria

1. When 描画面を描く, the wintf shall その描画面の根の命令の列と木を、描画面と同じ大きさ・同じ形式（1 色 8 ビットの BGRA・α 掛け済み）の入れ物へ合成し、合成し終えた入れ物の中身を描画面へ写す。
2. The wintf shall 描画面に出る画素を、入れ物を通さずに描画面へ直接描いていたとき（本 spec の前）と同じにする（木を持たない描画面・木を持つ描画面のどちらも）。
3. The wintf shall 入れ物を描画面 1 枚につき 1 枚だけ持ち、描画面が無くなればその入れ物も放す（入れ物で増えるメモリは描画面と同じ大きさの 1 枚分まで）。
4. The wintf shall 入れ物の中身を、次にその描画面を合成するまで保つ（写した後に捨てない）。
5. When 描画面の大きさが変わる, or 装置が失われて描画面が作り直される, the wintf shall 入れ物も同じ大きさで作り直し、作り直した回に描画面を全部合成し直す。
6. If 入れ物の作成・合成・描画面への写しのどれかが装置の失い以外の理由で失敗する, then the wintf shall 実体名と HRESULT を `error!` に記録し、次の回でその描画面を描き直す（今の `BeginDraw` の失敗と同じ道）。
7. If 入れ物の合成か写しの途中で装置が失われる, then the wintf shall 今の立ち直りの道（残りの描画面を描かずに抜け、作り直しの後に描き直す）に乗せる。
8. The wintf shall 根の命令の列（`GraphicsCommandList`）を今どおり透明で塗った入れ物へ先に `SOURCE_OVER` で描き、根の列のための規則の口は足さない（規則つきで描きたい描き手は、節か目印 `OwnSurface` の持ち主の描く中身で指定する）。

### Requirement 5: 1 ピクセルの読み戻し

**Objective:** 当たり判定とクリックの透過の作り手（`hit-id-pass`・`draw-methods-canon`）として、描画面と座標を与えれば、最後に合成し終えた答えの 1 画素（色と α）をその場で受け取りたい。合成を GPU へ移した後も、その座標が透けているかを 1 フレーム遅らせずに決められるようにするためである。

#### Acceptance Criteria

1. When 使い手が描画面と、その描画面の物理ピクセルの座標を与えて 1 画素を求める, the wintf shall その呼び出しの中で、最後に合成し終えた入れ物のその 1 画素の色と α（α 掛け済みの 8 ビット BGRA）を返す。
2. The wintf shall 前のフレームの値を使い回さず、求めた時点までにその描画面が合成し直されていれば、合成し直した後の答えを返す（GPU が合成を終えるまで待ってから読む）。
3. If 座標が描画面の外にある, then the wintf shall GPU から読まずに「画素なし」（透明の画素とは区別できる答え）を返す。
4. While 前に読んだときから座標が同じで、その描画面が合成し直されていない, the wintf shall GPU から読まずに前と同じ答えを返す。
5. If その描画面がまだ一度も合成されていない、または装置が失われている, then the wintf shall 「読めない」（画素なし・透明とは区別できる答え）を返す。
6. If 読み戻しの途中で GPU の呼び出しが失敗する, then the wintf shall 実体名と HRESULT を `error!` に記録し、「読めない」を返す（透明や前の値に化かさない・装置の失いの記録は今の立ち直りの道の 1 度だけ）。
7. The wintf shall 読み戻しを使い手が求めたときだけ行い、求めが無いフレームでは GPU から読まない。
8. The wintf shall 読み戻しで画素の値だけを返し、当たったかどうかの閾値（areka の α ≥ 0.5）を持たない（判定は使い手が持つ）。
9. When 使い手が描画面の全体の画素を求める, the wintf shall その描画面の、最後に合成し終えた入れ物の全部の画素を、その呼び出しの中で 1 行＝幅×4 バイトの詰めた BGRA で返す（5.2・5.5・5.6 と同じ決まりで）。
10. The wintf shall 1 画素の読み戻しの待ちの長さを実 GPU で測り、設計か実装の記録に残す（クリックの透過を付け替える `draw-methods-canon` の判断材料）。

### Requirement 6: 今の描画の道との両立

**Objective:** wintf と areka の利用者として、規則を指定しない描き手（wintf のほかのウィジェット・emo の今の表示）の画面と振る舞いが今と同じであってほしい。合成の口と入れ物の導入で既存の画面が変わらないことを確かめられるようにするためである。

#### Acceptance Criteria

1. The wintf shall 規則（ブレンド・1.0 でない不透明度）を指定しない描き手の答えの画素を変えない（本 spec の前後で、既存のウィジェットと emo の画素のテストが同じ答えを返し、全体テストが緑のまま）。
2. The wintf shall `wintf-visual-tree` が固定した答え（画家の順・重ね方・補間・変形・切り抜き・可視・描画面の持ち主と節の見分け）を変えない。
3. The wintf shall 描画面の持ち主の不透明度・切り抜き・可視を今のまま WUC の木で実現する（本 spec の不透明度の層は節にだけ効く。ただし NaN の扱い〔2.8〕は持ち主にも効く）。
4. The wintf shall 公開の型と口の名前を areka の外の使い手にも通じる語で付け、ukadoc と areka の語（`blend-*`・`overlay`・ゴースト・シェル・バルーンなど）を持ち込まない。
5. The wintf shall 新しい外部の依存を足さず、areka の本体（`crates/areka/`）に触らない。
6. If ブレンド・不透明度の層・入れ物のどこかで D2D か D3D の呼び出しが失敗する, then the wintf shall 失敗の理由（実体・HRESULT）を `error!` で記録する（記録の無い失敗の道を作らない・panic は致命のときだけ）。
7. The wintf shall 1 ファイルを 1,000 行以内に保ち、本 spec で足す新しいテストを本番ファイルの中に書かず、同じディレクトリの兄弟ファイルに置く（`structure.md` のテスト分離の規約）。

### Requirement 7: 決定論のテスト

**Objective:** wintf の保守者として、ブレンド・不透明度の層・切り抜きの中の効き先・入れ物・読み戻しの答えの画素を、装置の個体差に依らず固定してほしい。全体テストで合成の口の退行を機械で捕まえるためである。

#### Acceptance Criteria

1. The wintf shall 合成の答えの画素を、`wintf-visual-tree` の WARP の足場（D2D を CPU の描き手で動かし、画面に出さずに描いて CPU へ読み戻す）で固定し、`tools/test-all.ps1` の全体テストで常に回す（実機の環境変数や在席を要らない）。
2. The wintf shall 見本をテストのコードの中の値で持ち、比べる画素を絵の境界から離れた点に限り、1 チャネルあたり ±1 の差を許す（`wintf-visual-tree` の決めに揃える）。
3. When 足場でブレンドを描く, the wintf shall 26 種のそれぞれについて、少なくとも 1 点の答えの画素を固定する（dissolve は答えが模様に依らない点＝絵の α が 0 か 1 の点で固定する）。
4. When 足場でブレンドを描く, the wintf shall 絵の外形の外の画素が変わらないこと（1.3）と、ブレンドを指定しない描く中身の答えが変わらないこと（1.4）を固定する。
5. When 足場で不透明度の層を描く, the wintf shall 重なり合う子孫が互いに透けないこと（2.1）・不透明度 0 で描かれないこと（2.2）・1.0 で今と同じこと（2.3）・入れ子の層（2.5）を固定する。
6. When 足場で切り抜きの中の重ね方とブレンドを描く, the wintf shall 矩形・角丸・回転した矩形・任意の図形の切り抜きで答えが同じこと（3.1〜3.3）と、不透明度の層の中の効き先（3.4）を、`SOURCE_OVER` でない重ね方 1 つ以上とブレンド 1 つ以上で固定する。
7. When 足場で入れ物へ合成して写す, the wintf shall 入れ物を通した答えが、入れ物を通さずに直接描いた答えと同じこと（4.2）と、写した後も入れ物の中身が残ること（4.4）を固定する。
8. When 足場で 1 画素を読み戻す, the wintf shall 読んだ値が同じ点の答えの画素と同じこと（5.1）・合成し直した直後に新しい答えを返すこと（5.2）・外の座標と未合成で区別できる答えを返すこと（5.3・5.5）・同じ座標で合成し直していないときに GPU から読まないこと（5.4）を固定する。
9. The wintf shall ブレンドの描き方を持たない Windows のときの道（1.7）を、その有無を差し替えた装置なしの純粋なテストで固定する。
10. The wintf shall 合成の答えの画素に限って WARP を使い、実 GPU のテストは「画面に出るか」の確かめに残す（入れ物を通した描画面が実 GPU で画面に出ることを 1 つ以上確かめる）。

### Requirement 8: 文書

**Objective:** wintf を使う描き手と後の spec の作り手として、合成の口・入れ物・読み戻しの使い方と決めが steering と型の文書で分かってほしい。後の spec と外の使い手が同じ形で乗れるようにするためである。

#### Acceptance Criteria

1. The wintf shall `.kiro/steering/tech.md` の Testing の節の WARP の例外を、描画の木の画素の答えから「合成の答えの画素（描画の木・ブレンド・不透明度の層・入れ物・読み戻し）」へ広げて書く。
2. The wintf shall `.kiro/steering/structure.md` の描画の道の記述に、入れ物を通して描画面へ写す道と 1 ピクセルの読み戻しの口を書く。
3. The wintf shall ブレンド・不透明度の層・切り抜きの中の効き先・入れ物・読み戻しの決め（とくに 3.1〜3.4・5.2〜5.6）を公開の型と口の文書に書く。
4. The wintf shall `.kiro/steering/tech.md` に、支える Windows の下限を Windows 10 1903（ブレンドの口が要る版）と書く。

## 要件討議で確定した議題（2026-11-13・全議題を推奨案で一括承認）

brief の議題 1・4 は `wintf-visual-tree` で済んだ（描画の木を本番へ上げた・WARP の許容差は ±1）。残る brief の議題 2・3 と、要件の段とギャップ分析で見つけた議題を、開発者が 2026-11-13 に推奨案で一括承認した。

- **議題 1（ブレンドの口と Windows の下限）**: 下限を Windows 10 1903 とし、`ID2D1DeviceContext6::BlendImage` で描く（要件 8.4）。それより古い Windows は 1.7 の道。ブレンドの効果の道は層の中で `Flush` が要り使えないので採らない。
- **議題 2（要件 3.1〜3.3 が D2D で出来なかったとき）**: 3.1〜3.3 は書き換えない。設計の前に WARP で「背景から始める層＋ `COPY`」を試し（research.md R1）、通らなければ設計の段で別の D2D の道を探す。どれも無いときだけ開発者へ戻す（「図形の切り抜きの中は層の中身に効く」へ先に逃げない）。
- **議題 3（入れ物を全部の描画面に持つか）**: brief のとおり全部の描画面に持つ（要件 4.1・4.3 のまま）。
- **議題 4（クリックの透過の付け替え）**: 本 spec では付け替えない。1 画素の読み戻しの待ちを実 GPU で測って記録に残す（要件 5.10）。描画面の全体の読み戻しの口は本 spec が持つ（要件 5.9 を無条件に）。
- **議題 5（不透明度の NaN）**: 共通の 1 か所で NaN を 1.0 として扱い `warn!`（要件 2.8・6.3 の但し書き）。

以下は要件討議へ出した時点の議題の記録（推奨案はすべて上のとおり確定）。

1. **ブレンドの口と Windows の下限**（brief の議題 2・要件 1.7）: ブレンドを描く口は、描き先を直接読んで混ぜる口（Windows 10 1903 以降）か、ブレンドの効果（それより古い版でも動くが、下の画素を入力の絵として渡す手間が要る）か。areka が支える Windows は `tech.md` で「Windows 10/11（Windows.UI.Composition 対応）」とだけ書かれ、1903 より前を支えるかは書かれていない。**推奨**: 支える下限を Windows 10 1903 とし、直接読んで混ぜる口を使う（1903 より前で動いたときは 1.7 の道）。下限を下げるなら効果で描く（設計の段で決める）。
2. **クリックの透過と CPU の画素の読み手の付け替え**（brief の議題 3・要件 5.9・Boundary の Out）: 1 ピクセルの読み戻しで今の CPU の α マスクを置き換えるか、併せ持つか。本 spec の後も emo は CPU で合成するので、α マスクの出どころは消えない。**推奨**: 本 spec では置き換えない（口を作り、待ちの長さを測って設計の記録に残す）。付け替えは CPU の画素が無くなる `draw-methods-canon` で行う（brief の再測定の「議題 3 で置き換えるなら」の触るファイル＝`alpha_mask.rs`・`hit_region/`・emo-present の `budget.rs`・`cache.rs` には触らない）。あわせて、`draw-methods-canon` の brief が「両 spec の要件で決める」とした**描画面の全体の読み戻し**（`dump_surface`・バルーンの背景色の新しい出どころ）は、入れ物の持ち主である本 spec が口だけ持つ（要件 5.9）。
3. **切り抜きと層の中で重ね方とブレンドが効く先**（要件 3）: 今は矩形の切り抜きの中では下の画素に効き、図形の切り抜きの中では層の中身（透明から始まる）にしか効かない。**推奨**: 切り抜きは範囲を限るだけで、どの形でも下の画素に効かせる（3.1〜3.3）。不透明度の層の中だけは、層の中で先に描いた画素に効かせる（3.4＝層は 1 枚にまとめてから重ねるという 2.1 の意味のとおり）。
4. **入れ物を全部の描画面に持つか**（要件 4.1・4.3）: brief は全部の描画面に持つ形（Constraints の「窓ごとに 1 枚増える」）。描画面は今 Visual ごとに作られるので、増えるメモリは描画面の総和と同じだけになる。**推奨**: brief のとおり全部の描画面に持つ（読み戻しと差分の描き直しの描き先が描画面の種類で分かれない・既定の答えが 1 本の道で揃う）。
