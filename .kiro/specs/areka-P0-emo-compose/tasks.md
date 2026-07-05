# Implementation Plan

- [ ] 1. Foundation: 新設クレート雛形と上流パーサー転記ギャップの解消
- [x] 1.1 crates/areka-emo-compose の雛形を作成する
  - Cargo.toml に areka-parsers（path）・areka-emo-atlas（path）・bevy_ecs・tracing・thiserror（すべてワークスペース既存依存）のみを追加し、Rust 2024 edition・tokio 不使用を明示する
  - lib.rs にクレートdocsと公開モジュール構成（method/bind/composed/normalized/world/fold/atlas_bind/plan/blit）の骨組みを用意する
  - ワークスペース `members = ["crates/*"]` のため root Cargo.toml は変更不要であることを確認する
  - 観測可能な完了状態: `cargo build -p areka-emo-compose` が空実装のまま成功する
  - _Requirements: 12.1, 12.2, 10.4_

- [ ] 1.2 areka-parsers::shell の転記層へ4つの転記ギャップを追加する
  - `SortOrder` enum・`Shell.animation_sort`/`collision_sort`・`Shell.definitions`（登場順の単一定義ストリーム）・`Surface.targets`（多id記述子）・`SurfaceAppend.elements`・`AppendTarget` への除外 variant を追加する
  - 素の `surface` ヘッダの多id形（`N,M` 列挙・`N-M` 範囲）を、既存の append 用ターゲットパーサと共通化して解析する（現行の単一 `parse::<u32>` による破損を是正）
  - `animation-sort`/`collision-sort` の TopLevel 値を破棄せず `Shell` へ値化する
  - `surface.append` ブロック内の `element` 行を転記する
  - 展開・存在判定・create/append 適用などの意味論は一切追加しない（parserは記述のまま転記するのみ）
  - 観測可能な完了状態: `surface1-3` を含む surfaces.txt を parse すると `Surface.targets` に `Range{1,3}` が保持され、以前のように `id=0` へ破損しない
  - _Requirements: 12.5_

- [ ] 1.3 既存構造体リテラルを新フィールドへ機械的に追随させる
  - parsers 自身の既存テスト（validation_tests.rs・decode_tests.rs・parse_tests.rs・model_tests.rs）内の `Surface`/`SurfaceAppend`/`Shell` リテラル構築箇所に新フィールドの初期値を追記する
  - 完了済み `areka-emo-atlas` のテストヘルパ（emo2_e2e.rs・manifest.rs・lib.rs 内のリテラル箇所）にも同様に追随する
  - 追随はテストのアサーション意味を一切変更しない（初期値追加のみ）
  - 観測可能な完了状態: `cargo test -p areka-parsers -p areka-emo-atlas` が変更前と同じアサーション結果で成功する
  - _Requirements: 12.5_
  - _Depends: 1.2_

- [ ] 1.4 転記ギャップ4点の単体テストを追加する
  - 多id ヘッダ（列挙・範囲）・append 内 element・sort キー値・definitions の登場順保持を、それぞれ検証するテストを追加する
  - 既存の emo2 fixture 由来の断片を使い、`surface.append10,2100-2110,2200-2210` のような多ターゲット範囲を含める
  - 観測可能な完了状態: 4つの新規テストがそれぞれ転記結果の値を直接アサートして green になる
  - _Requirements: 12.5_
  - _Depends: 1.2_

- [ ] 1.5 (P) balloon パーサーのドキュメントコメントドリフトを修正する
  - `balloon/model.rs` 冒頭のドキュメントコメントにある旧名 `areka-P0-text-layer`/`areka-P0-surface-engine` を現行エンジン固有名へ修正する
  - 観測可能な完了状態: ドキュメントコメントに旧名参照が残っていない
  - _Requirements: 12.4_
  - _Boundary: balloon parser (model.rs ドキュメントのみ)_

- [ ] 2. Core: 合成メソッド写像表と公開データ契約
- [ ] 2.1 合成メソッド写像表（Method Registry）を実装する
  - ukadoc 由来の合成メソッド群（overlay/overlayfast/replace/base/reduce/asis/interpolate/add/bind/blend-* 群など）を `ComposeMethod`/`BlendMode` として全量列挙する
  - emo2 使用分（overlay。add/bindはoverlayと同義として写像）のみ実装し、他は明示的な未実装シームとして保持する
  - `is_implemented()` で実装状況を問い合わせ可能にする
  - 観測可能な完了状態: 全量列挙した enum の網羅 match テストが存在し、overlay のみ `is_implemented()==true` を返す
  - _Requirements: 8.1, 8.2, 8.3, 12.3_
  - _Boundary: Method Registry (method.rs)_

- [ ] 2.2 公開データ契約（BindSet/ComposedSurface/Transform/SurfaceMaster/NormalizedElement）を実装する
  - 有効bind集合を整列済み重複なしの `BindSet` として、合成結果を premultiplied BGRA・size・stride 明示の `ComposedSurface` として定義する
  - X,Yのみの平行移動を単位行列の特例として表現する `Transform` を定義する
  - collisions/animationsを保持する公開正規化定義 `SurfaceMaster`/`NormalizedElement`（2.1のComposeMethodを参照）を定義する
  - `BindSet`/`ComposedSurface` が `Send` であることをコンパイル時に保証するテストを追加する
  - 観測可能な完了状態: 各型のコンストラクタ・アクセサに対する単体テストが green で、Send 制約のコンパイル時アサーションが通る
  - _Requirements: 1.2, 4.2, 9.1, 9.2_
  - _Boundary: 公開データ契約 (bind.rs/composed.rs/normalized.rs)_
  - _Depends: 2.1_

- [ ] 3. Core: EmoWorld（emo専用 per-ghost bevy_ecs World）とサーフェス合成ツリーの single-pass fold
- [ ] 3.1 EmoWorld のコンポーネント/リソース定義と公開クエリ API を実装する
  - wintf本体Worldとは分離した専用 bevy_ecs World として、surface 1件＝entity 1件で `SurfaceId`/`SurfaceMaster`/`AtlasBinding` コンポーネントと `SurfaceIndex`/`AliasMap`/`ShellSettings` リソースを定義する
  - `surface(id)`/`surface_ids()`/`resolve_alias(key)`/`animation_sort()`/`collision_sort()` の公開クエリと空Worldからの `build()` 骨組みを用意する
  - 画素バッファを保持するコンポーネント/リソースを一切追加しない
  - 観測可能な完了状態: 空の `Shell` から構築した `EmoWorld` に対し `surface_ids()` が空を返し、存在しない id への `surface()` が `None` を返す
  - _Requirements: 1.1, 1.3, 1.8, 10.6_
  - _Boundary: EmoWorld_
  - _Depends: 2.2_

- [ ] 3.2 Fold: 素の surface 定義の展開＝全id新設を実装する
  - `surface` ヘッダのターゲット記述子（単一・列挙・範囲）を展開し、各idを新規surfaceとして生成し共有ボディ（element/collision/animation）を適用する
  - 既存idとの重複は全置換（後勝ち）としてwarnログに記録する
  - 参照先が見つからない場合もパニックせずwarnログで欠落を観測可能にする
  - 観測可能な完了状態: `surface0,5` を含む定義を fold すると id=0 と id=5 の両方が同一ボディを持つ surface として EmoWorld に存在する
  - _Requirements: 1.1, 1.4, 2.1_
  - _Boundary: Fold_

- [ ] 3.3 Fold: surface.append の展開＝既存id限定の追記を実装する
  - `surface.append` のターゲット記述子（単一・列挙・両端含む範囲）を展開し、その時点でツリーに存在するidのみへ追記する（非存在idは新設しない）
  - append内のelement/collision/animationを対象surfaceへマージし、同一animation idは後勝ち置換としてwarnログに記録する
  - 複数の定義（surfaceとappend）が同一surfaceに効く場合、パーサー出力の登場順で決定的に適用する
  - 観測可能な完了状態: `surface.append10,2100-2110,2200-2210` のような範囲を持つappendが、存在するidのみに反映され非存在idを生成しないことをテストで示す
  - _Requirements: 1.4, 2.2, 2.3, 2.4_
  - _Depends: 3.2_

- [ ] 3.4 Fold: 除外指定（`!N`/`!a-b`）の展開時減算を実装する
  - ターゲット記述子内の除外要素を展開結果から減算する
  - 除外の実処理はemo2が使用しない場合でも型シームとして口を保持する
  - 観測可能な完了状態: 除外を含むターゲット記述子を展開すると除外対象idが結果集合から欠けている
  - _Requirements: 2.5, 12.3_
  - _Depends: 3.3_

- [ ] 3.5 Fold: kero.surface.alias の解決を実装する
  - alias キー→順序付き数値idリストを `AliasMap` へ収集する
  - 同一キーの重複定義は後勝ちとして決定的に扱う
  - 未解決キーの参照はパニックせずwarnログを記録しNoneを返す
  - 観測可能な完了状態: emo2 fixtureの重複alias（`100,[2100]`が2回定義）をfoldした結果が後勝ちの一意な値を返す
  - _Requirements: 3.1, 3.2, 3.3_
  - _Depends: 3.2_

- [ ] 3.6 Fold: animation-sort/collision-sort の引き継ぎを実装する
  - 転記層が保持するsortキーの値を `ShellSettings` リソースへ引き継ぐ
  - 未指定時はukadoc既定（animation-sortはdescend・collision-sortはnone）を`animation_sort()`/`collision_sort()`が返すようにする
  - 観測可能な完了状態: sortキー未指定のShellをfoldした`EmoWorld`の`animation_sort()`がdescendを返す
  - _Requirements: 1.6, 5.6_
  - _Depends: 3.1_

- [ ] 3.7 Fold: 決定性を保証し emo2 fixture ベースの単体テストを追加する
  - 登場順definitionsストリームをsingle-passで畳み込み、前方参照なし・多パス不要であることを保証する
  - 同一入力に対してバイト等価な正規化結果を生成することをテストで固定する
  - emo2 fixtureの複数定義重なりケースを用いて登場順の決定的適用を検証する
  - 観測可能な完了状態: 同一Shellを2回foldして得られたEmoWorldの内容比較テストが一致を示す
  - _Requirements: 1.5, 1.7, 2.3, 10.1_
  - _Depends: 3.2, 3.3, 3.4, 3.5, 3.6_

- [ ] 4. Core: AtlasBinder によるアトラス束縛を実装する
  - 正規化elementの`ElementPath`を`AtlasTable::resolve`で一度きり`ElementId`へ束縛し`AtlasBinding`コンポーネントへ挿入する
  - 未解決要素はパニックせずwarnログに記録する
  - 観測可能な完了状態: MemoryDecoder+bakeで構築したAtlasTableに対し束縛を実行すると、既知パスの要素がElementIdを持ち未知パスがNoneでwarnログを出す
  - _Requirements: 4.3_
  - _Depends: 3.1, 3.7_

- [ ] 5. Core: PlanBuilder による合成プラン導出
- [ ] 5.1 element レイヤ順と変換行列による命令列の基礎を実装する
  - `SurfaceMaster.elements`をlayer昇順（同layerは登場順）で列挙し、アトラス参照（ElementId・Placement）を含む転写命令を導出する
  - X,Yのみの平行移動を単位行列の特例として`Transform`で表現する
  - 同一の正規化定義に対して決定的な命令列を導出することをテストで固定する
  - 観測可能な完了状態: 既知のSurfaceMasterから導出した命令列がlayer昇順に整列し、複数回の導出で同一結果になる
  - _Requirements: 4.1, 4.2, 4.3, 4.5, 10.1_
  - _Depends: 4_

- [ ] 5.2 有効bind集合の合成対象化とanimation-sort→ID順の2段規則を実装する
  - 有効bind（BindSetに含まれるanimation）のpattern0 overlayを合成対象に含める
  - `animation_sort()`がdescend（既定）ならID昇順、ascendならID降順で重ねる2段規則を適用する
  - 静的elementを持たず全パーツがbindであるsurfaceでも、非空の bind 集合から可視レイヤが生成されることを保証する
  - 観測可能な完了状態: 複数の有効bindを持つケースで、sort既定（descend）のときID昇順の重なり順になることをテストで示す
  - _Requirements: 5.2, 5.3, 5.4, 5.6_
  - _Depends: 5.1_

- [ ] 5.3 入れ子surface参照のflattenと循環検出を実装する
  - pattern0のsurface_id参照を再帰的にinline展開し、オフセットを累積した平坦な命令列を生成する
  - 訪問集合を用いて自己参照・相互参照の循環を検出し、パニックせずその枝のみ打ち切りwarnログに記録する
  - 観測可能な完了状態: 自己参照する入れ子定義を持つsurfaceを合成プラン導出してもスタックオーバーフローせず、warnログとともに部分結果が得られる
  - _Requirements: 4.4, 7.1, 7.2, 7.3, 12.3_
  - _Depends: 5.2_

- [ ] 5.4 placement Noneのスキップと静的キャンバス外形算出を実装する
  - `AtlasEntry.placement`がNone（全透明）のelementは命令化せずスキップする
  - キャンバス外形を、有効bind集合に依存せず全定義層（全element＋全bind animationのpattern0）から静的に算出する（原点固定・和集合・負方向クリップ）
  - 観測可能な完了状態: 同一surfaceで異なるbind集合を渡しても算出される外形サイズが変わらないことをテストで示す
  - _Requirements: 6.3, 6.5_
  - _Depends: 5.3_

- [ ] 5.5 命令ゼロ時の分類（正常空合成/対象不在/退化データ）を実装する
  - 対象surfaceが存在しない場合は失敗として扱い区別可能にする
  - surfaceが存在し描画可能命令がゼロ（全透明・空bind集合）の場合は失敗とせず、静的外形どおりの空命令列として扱う
  - 定義層が皆無で外形が0×0となる退化データのみ真の失敗として区別する
  - 観測可能な完了状態: 全element が全透明のsurfaceに対する命令導出が、エラーではなく空の命令列と非ゼロの外形を返す
  - _Requirements: 1.4, 6.6, 10.5_
  - _Depends: 5.4_

- [ ] 6. Core: BlitExecutor によるCPU転写実行
- [ ] 6.1 premultiplied SourceOver整数転写を実装する
  - アトラス頁の`uv_rect`から合成先バッファへ、転写先座標（配置オフセット＋trim_offset）でpremultiplied SourceOverの整数式（`dst_c' = src_c + div255(dst_c*(255-src_a))`）により転写する
  - 座標演算はi64で行い合成先境界へクリップしてから適用する
  - 合成先バッファを再利用し、転写をO(elements)で行い浮動小数を経路に持ち込まない
  - 観測可能な完了状態: 既知画素値ペアの転写結果が手計算した期待値とバイト一致する
  - _Requirements: 6.1, 6.2, 6.4, 8.2, 10.2, 10.3_
  - _Depends: 5.5_

- [ ] 6.2 未実装メソッド命令のwarn+skipを実装する
  - `is_implemented()==false`の命令を検出した場合、その命令をスキップしwarnログ（メソッド名・surface id付き）を記録して処理を継続する
  - 観測可能な完了状態: overlay以外のメソッドを含む命令列を直接構築して実行すると、パニックせず該当命令のみスキップされた結果が得られる
  - _Requirements: 8.4_
  - _Depends: 6.1_

- [ ] 7. Integration: Composer facade の結線
  - `compose_into`（バッファ再利用）と`compose`（新規割り当て）の入口を実装し、Fold/AtlasBinder/PlanBuilder/BlitExecutorを結線する
  - `ComposeError`（SurfaceNotFound/EmptyComposition）を定義し、失敗経路は`error!`ログ＋`Err`、致命はパニック限定＋直前ログとする
  - スレッド生成・async・channelを一切使わず同期関数として値を直接返し、合成結果のキャッシュを保持しない
  - 束縛後の合成経路（compose_into/composeのホットパス）で`AtlasTable::resolve`が一切呼ばれず`entry`のO(1)引きのみになっていることを確認する（アトラス束縛はタスク4で一度きり実施済み）
  - 観測可能な完了状態: `Composer::compose`が有効な入力に対して`Ok(ComposedSurface)`を返し、`Composer`自身が前回の合成結果を保持していないことを確認できる
  - _Requirements: 5.1, 5.5, 6.6, 9.1, 9.3, 9.4, 10.3, 10.4, 10.5_
  - _Depends: 3, 4, 5, 6_

- [ ] 8. Validation: emo2 fixtureによるオフスクリーンpixel観測と決定性検証
- [ ] 8.1 surface0のgoldenテストを実装する
  - emo2 fixtureのsurfaces.txtをparseし、MemoryDecoder+bake経路でCOM非依存にAtlasTableを構築して合成する
  - element0単層の合成結果が挿入した画像とバイト等価であることを検証する
  - 観測可能な完了状態: surface0のgoldenテストがCOM/表示なしで実行されgreenになる
  - _Requirements: 11.1, 11.4_
  - _Depends: 7_

- [ ] 8.2 surface1000＋bind集合のgoldenテストを実装する
  - 全パーツがMAYUNA bindのsurfaceに有効bind集合を与えて合成し、非空（α>0の画素がある）かつbind数に応じた重なりを要点サンプリングで検証する
  - 観測可能な完了状態: 空のbind集合では全透明、非空のbind集合では非空の合成結果になることをテストで示す
  - _Requirements: 11.2, 5.4_
  - _Depends: 7_

- [ ] 8.3 トリム等価のpixelテストを実装する
  - 透明マージン付き画像をトリムありでbakeした合成結果が、トリムなし理論配置と全画素一致することを検証する
  - 観測可能な完了状態: トリムあり/なしの合成結果比較テストが一致を示しgreenになる
  - _Requirements: 11.3, 6.2_
  - _Depends: 7_

- [ ] 8.4 決定性と再合成予算の検証テストを実装する
  - 同一入力で2回composeしバイト等価であることを検証する
  - `compose_into`の定常状態（同一surfaceの繰り返し合成）でアロケーションが発生しないことを検証する
  - 命令数がO(elements)であることをemo2 surface1000＋全bindのケースでassertする
  - 観測可能な完了状態: 決定性テストとゼロアロケーションテストの両方がgreenになる
  - _Requirements: 10.1, 10.3_
  - _Depends: 7_
