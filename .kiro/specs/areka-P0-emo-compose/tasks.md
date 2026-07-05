# Implementation Plan

- [ ] 1. Foundation: 新設クレート雛形と上流パーサー転記ギャップの解消
- [x] 1.1 crates/areka-emo-compose の雛形を作成する
  - Cargo.toml に areka-parsers（path）・areka-emo-atlas（path）・bevy_ecs・tracing・thiserror（すべてワークスペース既存依存）のみを追加し、Rust 2024 edition・tokio 不使用を明示する
  - lib.rs にクレートdocsと公開モジュール構成（method/bind/composed/normalized/world/fold/atlas_bind/plan/blit）の骨組みを用意する
  - ワークスペース `members = ["crates/*"]` のため root Cargo.toml は変更不要であることを確認する
  - 観測可能な完了状態: `cargo build -p areka-emo-compose` が空実装のまま成功する
  - _Requirements: 12.1, 12.2, 10.4_

- [x] 1.2 areka-parsers::shell の転記層へ4つの転記ギャップを追加する
  - `SortOrder` enum・`Shell.animation_sort`/`collision_sort`・`Shell.definitions`（登場順の単一定義ストリーム）・`Surface.targets`（多id記述子）・`SurfaceAppend.elements`・`AppendTarget` への除外 variant を追加する
  - 素の `surface` ヘッダの多id形（`N,M` 列挙・`N-M` 範囲）を、既存の append 用ターゲットパーサと共通化して解析する（現行の単一 `parse::<u32>` による破損を是正）
  - `animation-sort`/`collision-sort` の TopLevel 値を破棄せず `Shell` へ値化する
  - `surface.append` ブロック内の `element` 行を転記する
  - 展開・存在判定・create/append 適用などの意味論は一切追加しない（parserは記述のまま転記するのみ）
  - 観測可能な完了状態: `surface1-3` を含む surfaces.txt を parse すると `Surface.targets` に `Range{1,3}` が保持され、以前のように `id=0` へ破損しない
  - _Requirements: 12.5_

- [x] 1.3 既存構造体リテラルを新フィールドへ機械的に追随させる
  - parsers 自身の既存テスト（validation_tests.rs・decode_tests.rs・parse_tests.rs・model_tests.rs）内の `Surface`/`SurfaceAppend`/`Shell` リテラル構築箇所に新フィールドの初期値を追記する
  - 完了済み `areka-emo-atlas` のテストヘルパ（emo2_e2e.rs・manifest.rs・lib.rs 内のリテラル箇所）にも同様に追随する
  - 追随はテストのアサーション意味を一切変更しない（初期値追加のみ）
  - 観測可能な完了状態: `cargo test -p areka-parsers -p areka-emo-atlas` が変更前と同じアサーション結果で成功する
  - _Requirements: 12.5_
  - _Depends: 1.2_

- [x] 1.4 転記ギャップ4点の単体テストを追加する
  - 多id ヘッダ（列挙・範囲）・append 内 element・sort キー値・definitions の登場順保持を、それぞれ検証するテストを追加する
  - 既存の emo2 fixture 由来の断片を使い、`surface.append10,2100-2110,2200-2210` のような多ターゲット範囲を含める
  - 観測可能な完了状態: 4つの新規テストがそれぞれ転記結果の値を直接アサートして green になる
  - _Requirements: 12.5_
  - _Depends: 1.2_

- [x] 1.5 (P) balloon パーサーのドキュメントコメントドリフトを修正する
  - `balloon/model.rs` 冒頭のドキュメントコメントにある旧名 `areka-P0-text-layer`/`areka-P0-surface-engine` を現行エンジン固有名へ修正する
  - 観測可能な完了状態: ドキュメントコメントに旧名参照が残っていない
  - _Requirements: 12.4_
  - _Boundary: balloon parser (model.rs ドキュメントのみ)_

- [ ] 2. Core: 合成メソッド写像表と公開データ契約
- [x] 2.1 合成メソッド写像表（Method Registry）を実装する
  - ukadoc 由来の合成メソッド群（overlay/overlayfast/replace/base/reduce/asis/interpolate/add/bind/blend-* 群など）を `ComposeMethod`/`BlendMode` として全量列挙する
  - emo2 使用分（overlay。add/bindはoverlayと同義として写像）のみ実装し、他は明示的な未実装シームとして保持する
  - `is_implemented()` で実装状況を問い合わせ可能にする
  - 観測可能な完了状態: 全量列挙した enum の網羅 match テストが存在し、overlay のみ `is_implemented()==true` を返す
  - _Requirements: 8.1, 8.2, 8.3, 12.3_
  - _Boundary: Method Registry (method.rs)_

- [x] 2.2 公開データ契約（BindSet/ComposedSurface/Transform/SurfaceMaster/NormalizedElement）を実装する
  - 有効bind集合を整列済み重複なしの `BindSet` として、合成結果を premultiplied BGRA・size・stride 明示の `ComposedSurface` として定義する
  - X,Yのみの平行移動を単位行列の特例として表現する `Transform` を定義する
  - collisions/animationsを保持する公開正規化定義 `SurfaceMaster`/`NormalizedElement`（2.1のComposeMethodを参照）を定義する
  - `BindSet`/`ComposedSurface` が `Send` であることをコンパイル時に保証するテストを追加する
  - 観測可能な完了状態: 各型のコンストラクタ・アクセサに対する単体テストが green で、Send 制約のコンパイル時アサーションが通る
  - _Requirements: 1.2, 4.2, 9.1, 9.2_
  - _Boundary: 公開データ契約 (bind.rs/composed.rs/normalized.rs)_
  - _Depends: 2.1_

- [ ] 3. Core: EmoWorld（emo専用 per-ghost bevy_ecs World）とサーフェス合成ツリーの single-pass fold
- [x] 3.1 EmoWorld のコンポーネント/リソース定義と公開クエリ API を実装する
  - wintf本体Worldとは分離した専用 bevy_ecs World として、surface 1件＝entity 1件で `SurfaceId`/`SurfaceMaster`/`AtlasBinding` コンポーネントと `SurfaceIndex`/`AliasMap`/`ShellSettings` リソースを定義する
  - `surface(id)`/`surface_ids()`/`resolve_alias(key)`/`animation_sort()`/`collision_sort()` の公開クエリと空Worldからの `build()` 骨組みを用意する
  - 画素バッファを保持するコンポーネント/リソースを一切追加しない
  - 観測可能な完了状態: 空の `Shell` から構築した `EmoWorld` に対し `surface_ids()` が空を返し、存在しない id への `surface()` が `None` を返す
  - _Requirements: 1.1, 1.3, 1.8, 10.6_
  - _Boundary: EmoWorld_
  - _Depends: 2.2_

- [x] 3.2 Fold: 素の surface 定義の展開＝全id新設を実装する
  - `surface` ヘッダのターゲット記述子（単一・列挙・範囲）を展開し、各idを新規surfaceとして生成し共有ボディ（element/collision/animation）を適用する
  - 既存idとの重複は全置換（後勝ち）としてwarnログに記録する
  - 参照先が見つからない場合もパニックせずwarnログで欠落を観測可能にする
  - 観測可能な完了状態: `surface0,5` を含む定義を fold すると id=0 と id=5 の両方が同一ボディを持つ surface として EmoWorld に存在する
  - _Requirements: 1.1, 1.4, 2.1_
  - _Boundary: Fold_

- [x] 3.3 Fold: surface.append の展開＝既存id限定の追記を実装する
  - `surface.append` のターゲット記述子（単一・列挙・両端含む範囲）を展開し、その時点でツリーに存在するidのみへ追記する（非存在idは新設しない）
  - append内のelement/collision/animationを対象surfaceへマージし、同一animation idは後勝ち置換としてwarnログに記録する
  - 複数の定義（surfaceとappend）が同一surfaceに効く場合、パーサー出力の登場順で決定的に適用する
  - 観測可能な完了状態: `surface.append10,2100-2110,2200-2210` のような範囲を持つappendが、存在するidのみに反映され非存在idを生成しないことをテストで示す
  - _Requirements: 1.4, 2.2, 2.3, 2.4_
  - _Depends: 3.2_

- [x] 3.4 Fold: 除外指定（`!N`/`!a-b`）の展開時減算を実装する
  - ターゲット記述子内の除外要素を展開結果から減算する
  - 除外の実処理はemo2が使用しない場合でも型シームとして口を保持する
  - 観測可能な完了状態: 除外を含むターゲット記述子を展開すると除外対象idが結果集合から欠けている
  - _Requirements: 2.5, 12.3_
  - _Depends: 3.3_

- [x] 3.5 Fold: kero.surface.alias の解決を実装する
  - alias キー→順序付き数値idリストを `AliasMap` へ収集する
  - 同一キーの重複定義は後勝ちとして決定的に扱う
  - 未解決キーの参照はパニックせずwarnログを記録しNoneを返す
  - 観測可能な完了状態: emo2 fixtureの重複alias（`100,[2100]`が2回定義）をfoldした結果が後勝ちの一意な値を返す
  - _Requirements: 3.1, 3.2, 3.3_
  - _Depends: 3.2_

- [x] 3.6 Fold: animation-sort/collision-sort の引き継ぎを実装する
  - 転記層が保持するsortキーの値を `ShellSettings` リソースへ引き継ぐ
  - 未指定時はukadoc既定（animation-sortはdescend・collision-sortはnone）を`animation_sort()`/`collision_sort()`が返すようにする
  - 観測可能な完了状態: sortキー未指定のShellをfoldした`EmoWorld`の`animation_sort()`がdescendを返す
  - _Requirements: 1.6, 5.6_
  - _Depends: 3.1_

- [x] 3.7 Fold: 決定性を保証し emo2 fixture ベースの単体テストを追加する
  - 登場順definitionsストリームをsingle-passで畳み込み、前方参照なし・多パス不要であることを保証する
  - 同一入力に対してバイト等価な正規化結果を生成することをテストで固定する
  - emo2 fixtureの複数定義重なりケースを用いて登場順の決定的適用を検証する
  - 観測可能な完了状態: 同一Shellを2回foldして得られたEmoWorldの内容比較テストが一致を示す
  - _Requirements: 1.5, 1.7, 2.3, 10.1_
  - _Depends: 3.2, 3.3, 3.4, 3.5, 3.6_

- [x] 4. Core: AtlasBinder によるアトラス束縛を実装する
  - 正規化elementの`ElementPath`を`AtlasTable::resolve`で一度きり`ElementId`へ束縛し`AtlasBinding`コンポーネントへ挿入する
  - 未解決要素はパニックせずwarnログに記録する
  - 観測可能な完了状態: MemoryDecoder+bakeで構築したAtlasTableに対し束縛を実行すると、既知パスの要素がElementIdを持ち未知パスがNoneでwarnログを出す
  - _Requirements: 4.3_
  - _Depends: 3.1, 3.7_

- [ ] 5. Core: PlanBuilder による合成プラン導出
- [x] 5.1 element レイヤ順と変換行列による命令列の基礎を実装する
  - `SurfaceMaster.elements`をlayer昇順（同layerは登場順）で列挙し、アトラス参照（ElementId・Placement）を含む転写命令を導出する
  - X,Yのみの平行移動を単位行列の特例として`Transform`で表現する
  - 同一の正規化定義に対して決定的な命令列を導出することをテストで固定する
  - 観測可能な完了状態: 既知のSurfaceMasterから導出した命令列がlayer昇順に整列し、複数回の導出で同一結果になる
  - _Requirements: 4.1, 4.2, 4.3, 4.5, 10.1_
  - _Depends: 4_

- [x] 5.2 有効bind集合の合成対象化とanimation-sort→ID順の2段規則を実装する
  - 有効bind（BindSetに含まれるanimation）のpattern0 overlayを合成対象に含める
  - `animation_sort()`がdescend（既定）ならID昇順、ascendならID降順で重ねる2段規則を適用する
  - 静的elementを持たず全パーツがbindであるsurfaceでも、非空の bind 集合から可視レイヤが生成されることを保証する
  - 観測可能な完了状態: 複数の有効bindを持つケースで、sort既定（descend）のときID昇順の重なり順になることをテストで示す
  - _Requirements: 5.2, 5.3, 5.4, 5.6_
  - _Depends: 5.1_

- [x] 5.3 入れ子surface参照のflattenと循環検出を実装する
  - pattern0のsurface_id参照を再帰的にinline展開し、オフセットを累積した平坦な命令列を生成する
  - 訪問集合を用いて自己参照・相互参照の循環を検出し、パニックせずその枝のみ打ち切りwarnログに記録する
  - 観測可能な完了状態: 自己参照する入れ子定義を持つsurfaceを合成プラン導出してもスタックオーバーフローせず、warnログとともに部分結果が得られる
  - _Requirements: 4.4, 7.1, 7.2, 7.3, 12.3_
  - _Depends: 5.2_

- [x] 5.4 placement Noneのスキップと静的キャンバス外形算出を実装する
  - `AtlasEntry.placement`がNone（全透明）のelementは命令化せずスキップする
  - キャンバス外形を、有効bind集合に依存せず全定義層（全element＋全bind animationのpattern0）から静的に算出する（原点固定・和集合・負方向クリップ）
  - 観測可能な完了状態: 同一surfaceで異なるbind集合を渡しても算出される外形サイズが変わらないことをテストで示す
  - _Requirements: 6.3, 6.5_
  - _Depends: 5.3_

- [x] 5.5 命令ゼロ時の分類（正常空合成/対象不在/退化データ）を実装する
  - 対象surfaceが存在しない場合は失敗として扱い区別可能にする
  - surfaceが存在し描画可能命令がゼロ（全透明・空bind集合）の場合は失敗とせず、静的外形どおりの空命令列として扱う
  - 定義層が皆無で外形が0×0となる退化データのみ真の失敗として区別する
  - 観測可能な完了状態: 全element が全透明のsurfaceに対する命令導出が、エラーではなく空の命令列と非ゼロの外形を返す
  - _Requirements: 1.4, 6.6, 10.5_
  - _Depends: 5.4_

- [ ] 6. Core: BlitExecutor によるCPU転写実行
- [x] 6.1 premultiplied SourceOver整数転写を実装する
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

## Implementation Notes

- 2.1 (method.rs): `BlendMode` は設計スケッチの単一 enum ではなく `BlendMode { kind: BlendKind, fast: bool }`（struct）＋ `#[non_exhaustive] enum BlendKind`（19 modes）へ分割実装。`fast` 軸と `kind` 軸が直交するため enum 倍化を回避。`#[non_exhaustive]` 拡張シームは `BlendKind`（variant 軸）に載る。下流（plan/blit の 5.x/6.x）が `ComposeMethod::Blend(BlendMode)` を match する際はこの形を前提とする。`from_name`（名前→ComposeMethod 写像・add/bind→Overlay・旧別名 overlaymultiply→blend-multiply-fast 等）も同梱済み。
- 3.1 (world.rs): design 指定パス `areka_parsers::shell::SortOrder` を解決するため parser の `shell/mod.rs` に `SortOrder` の `pub use` を additive 追加（task 1.2 が定義したが未 re-export だった）。fold タスク（3.2〜3.7）が `DefRef` を使う場合は同様に `shell/mod.rs` へ `DefRef` の re-export が必要（現状未 re-export）。`SurfaceMaster` は normalized.rs で `#[derive(Component)]` 済み（本型自体が component）。`EmoWorld::build` はリソース挿入後 `populate_from_shell(&mut self, shell)` を一度呼ぶ骨組みで、fold はこの1関数へ差し込む。
- 3.2 (fold.rs): `expand_targets(&[AppendTarget]) -> Vec<u32>`（design スケッチの `impl Iterator` から Vec へ・決定性/デバッグ容易性。呼び出しは `for` 反復のみで挙動同一）が plain(3.2)/append(3.3)/exclusion(3.4) 全経路の共通展開口。`fold_shell(&mut World, &Shell)` が `shell.definitions`（DefRef 登場順ストリーム）を single-pass 走査し `DefRef::{Surface,Append,Alias}` を dispatch。`#[non_exhaustive]` な `DefRef`/`AppendTarget` には防御的 wildcard arm（warn・非パニック）必須。
- 3.6 (world.rs): **新規実装不要——task 3.1 の `EmoWorld::build` 骨組みで既に充足**。`build` が `shell.animation_sort`/`collision_sort` を `ShellSettings` リソースへ引き継ぎ、`animation_sort()` が `unwrap_or(SortOrder::Descend)`・`collision_sort()` が Option 素通しで ukadoc 既定（descend/none）を返す。受け入れ条件（未指定 Shell を build→`animation_sort()==Descend`）は既存テスト `world::tests::default_sort_orders`／`explicit_sort_orders_are_preserved` が直接検証済み（1.6/5.6）。fold 段は sort に触れない（build が担う）ため 3.6 に diff は生じない。
- 5.1 (plan.rs): 静的element中核を `pub(crate) fn push_static_element_ops(out_ops, world, surface_id)`（append-only・clearしない＝scratch再利用）として実装。設計フルシグネチャ `build_plan(out_ops, visited, world, surface_id, binds) -> Result<Extent, ComposeError>` は**未導入・5.2がこのfnをwrapする**。`BlitOp{element:ElementId, transform, method}` は `lib.rs` で `pub use`。layer列挙は**stable `sort_by_key`**（同layer登場順維持＝4.5決定性の要）。None束縛はskip（trace）。当2fnは `#[allow(dead_code)]`（scoped・テストで駆動済み）で、**5.2がbuild_plan結線時にallowを外す**こと。BlitOpはElementIdのみ保持しPlacement引きはblit時（task6）。
- 5.2 (plan.rs): `pub(crate) fn derive_ops(out_ops, visited, world, surface_id, binds)` が層(i)静的element→層(ii)bind pattern0 を積む中核（design option(b)採用・公開 build_plan/Extent/ComposeError は未導入＝5.4/5.5送り・provisional偽装しない）。**2段ソート正本（決定5・レビュー独立再導出済み）**: `animation_sort()==Descend`（既定）→ animation ID **昇順** sort（`sort_unstable()`・小ID下/大ID上）、`Ascend`→ID **降順**（`b.cmp(a)`）。有効bind＝`interval∈{Bind,BindRandom}`（`is_bind_interval`・`Random`除外）∧ `id∈BindSet`。pattern0=**min index**（疎index許容）。`Pattern.surface_id<0`=センチネルskip。`push_static_element_ops` は offset_x/offset_y 引数追加（5.1呼び出しは (…,0,0)）・bind入れ子は **1段** inline展開（多段再帰＋visited循環検出は5.3送り）。実 model: `Animation{id:u32,interval:Interval,patterns:Vec<Pattern>}`／`Interval::{Bind,Random{k},BindRandom{k}}`(#[non_exhaustive])／`Pattern{index:u32,surface_id:i64,wait,x:i64,y:i64}`。fold後勝ちで同一surface内 animation id 一意ゆえ sort_unstable でも決定的。derive_ops のみ scoped `#[allow(dead_code)]`（5.4 build_plan が消費）。
- 5.3 (plan.rs) **設計裁定（design 内部緊張の解消・revalidation trigger）**: `flatten_surface(out_ops, visited, world, surface_id, binds, off_x, off_y)` は **bind pattern0 を全再帰段で同一 `binds` により展開**する（design flatten bullet L605「参照先の elements のみ／参照先 surface 自身の bind は展開しない＝binds top-level限定」から**意図的に乖離**）。**根拠（レビュー独立検証済み）**: 静的element は surface参照を持たぬ atlas 葉ゆえ、surface→surface 入れ子の唯一の機構が bind pattern0→surface。binds を top-level限定にすると入れ子は厳密に1段で循環は構造的に不可能となり Req 7.1「再帰的に合成」/7.2 循環検出が**空虚化**する。全段再帰は emo2 及び全 element-only 入れ子 surface で strict 読みと**バイト等価**（乖離は「入れ子 surface 自身が active bind id を持つ」場合のみ＝design が BindSet スコープ再設計として明示的に保留した case）。visited=**祖先スタック**（enter で contains 判定→push・exit で pop・LIFO debug_assert）で循環枝のみ warn+打ち切り（非パニック）・非循環 DAG 共有子は各経路で再展開。offset は各段 pattern0 (x,y) を単一 flat Vec へ累積（中間バッファ無し・Key Decision 2）。**将来 BindSet を入れ子ごとにスコープ分けする要求が出たら本箇所を再検討**。
- 5.4 (plan.rs): `Extent{w:u32,h:u32}`＋`compute_extent(world, atlas, surface_id) -> Extent`＋`flatten_extent`（別走査ワーカ）を追加。**外形は bind非依存が構造的保証**——`compute_extent`/`flatten_extent` は `BindSet` を**引数に取らない**（母集合＝全 element＋`is_bind_interval` な**全** bind animation の pattern0・`binds.contains` gate 無し）。placement None は ops ではスキップ（`push_static_element_ops` が `atlas.entry(id).placement.is_none()` で除外・trace）だが**外形には original で寄与**（flatten_extent は placement 無視で original 参照）。原点(0,0)固定・各層 `max(0, offset+original.{w,h})`・負オフセットは原点クリップ。祖先スタック循環検出は flatten_surface と同型。`derive_ops`/`push_static_element_ops`/`flatten_surface` に `atlas: &AtlasTable` 引数追加（derive_ops/push_static は3番目）。真の空 surface は `Extent{0,0}`（0×0退化の Err分類は5.5）。透明(α=0)bake画像は本物の `placement:None`＋original保持（trim.rs契約）ゆえ手製mock不要。**注記**: セッション制限で前段agentがproduction完成後に中断→後段agentがテスト呼び出し追随(~24箇所・assert不変)＋5.4テスト6本を完遂。`flatten_surface`/`flatten_extent` は8引数walkerゆえ `clippy::too_many_arguments` 既知（build_plan facade化=task7でスクラッチ構造体化する余地）。
- 5.5 (plan.rs/error.rs): `ComposeError{SurfaceNotFound(u32), EmptyComposition(u32)}`（error.rs・#[non_exhaustive]・thiserror・PartialEq/Eq・lib.rs で re-export）。`pub(crate) fn build_plan(out_ops, world, atlas, surface_id, binds) -> Result<Extent, ComposeError>` が3分類facade: (1) `world.surface(id).is_none()`→**compute前に早期** `error!`+`Err(SurfaceNotFound)`、(2) `compute_extent`→`extent.w==0 && extent.h==0`のみ`error!`+`Err(EmptyComposition)`（**`ops.is_empty()`では判定しない**＝退化データ限定）、(3) それ以外は `derive_ops`＋`Ok(extent)`（**命令ゼロでも正常**＝全透明/空bind・要件6.6）。entry で `out_ops.clear()`（scratch再利用10.3）。`#[allow(dead_code)]` は derive_ops/compute_extent/flatten_*から除去し build_plan へ集約（task7 Composer が消費・visited共有scratch化もtask7）。build_plan は現状 extent と ops を2回走査（task7でscratch統合余地）。
- 6.1 (blit.rs/composed.rs): `execute(out, extent, ops, atlas)` が premultiplied SourceOver 整数転写。`div255(v)=(v+127)/255`・`source_over_channel(src,dst,inv)=src+div255(dst*inv)`（inv=255-src_a）、**alpha も同式**（straight-α混在なし）。座標 i64・`dest=transform.offset()+trim_offset`・境界クリップ・頁は `bytes[(uv.y+sy)*stride+(uv.x+sx)*4]`（**uv原点オフセット尊重・stride padding対応**）。**浮動小数ゼロ**。composed.rs へ `pub(crate) resize_and_clear(w,h)`（同一Vec clear+resize＝容量再利用10.3）＋`bytes_mut()` を追加（公開API不変）。placement None/頁欠落は warn+skip非パニック。`execute` は `#[allow(dead_code)]`（task7 Composer が結線）。**未被覆**: `uv_rect.x/y≠0` の単体テストは無し（実装は正しい）→ task8 golden が実 placement の非ゼロ uv で拾う。手計算受入: src[20,40,60,128] over dst[200,100,50,255]→[120,90,85,255]。
- 4 (atlas_bind.rs): 実 atlas API 正本シグネチャ（plan/blit タスク 5.x/6.x で必須）— `AtlasTable::resolve(&self, set: SetId, rel_path: &str) -> Option<ElementId>`（table.rs・**構築時一度きり**）／`AtlasTable::entry(...)`（ホットパス O(1)・resolve 禁止）。`ElementId(pub u32)`／`SetId(pub u32)`（table.rs、crate root re-export）。bake キー＝`element.path.as_str().to_string()`（manifest.rs:88）＝bind キー`ElementPath.as_str()` と同一規約ゆえ既知パス Some・未知 None。headless AtlasTable 構築は `MemoryDecoder`+`bake(SurfaceSet, decoder, PackConfig)`＋`AlphaParams{use_self_alpha:UseSelfAlpha::On}`（全て crate root 露出・emo-atlas の bake テストヘルパをミラー）。`AtlasBinding(Vec<Option<ElementId>>)` は `SurfaceMaster.elements` と index 平行。World 借用は「不変クエリで (entity, AtlasBinding) 収集→`entity_mut().insert()`」の2段で衝突回避。
