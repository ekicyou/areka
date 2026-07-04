# Implementation Plan

- [ ] 1. 基盤整備: 新規クレート雛形・WIC デコード経路の拡張・共有契約型の定義
- [x] 1.1 ワークスペースへの依存追加と新規クレートの雛形作成
  - 承認済みの静的パッキングライブラリ（rectangle-pack）をワークスペース共通依存へ追加する
  - emo 素材基盤層のための新規純粋クレートを作成し、パイプライン各段（列挙・デコード・正規化・トリム・packing・焼付・契約型）のモジュールを空実装で用意する
  - 新規クレートは wintf 本体（ECS/D2D/GraphicsCore）へ依存しないことを Cargo.toml の依存宣言で担保する
  - ワークスペース全体のビルドが新規クレートを含めて成功することを確認する
  - _Boundary: crate scaffolding_

- [x] 1.2 既存 WIC デコード経路のユーティリティ化と変換前アルファ情報の公開
  - 既存の PBGRA デコード処理を ECS 非依存の共有ユーティリティとして再配置し、新規クレートのデコード腕から呼び出せる形にする
  - 変換前（PBGRA 変換前）のピクセルフォーマットからアルファチャンネル有無を取得できるよう、ユーティリティの戻り値を拡張する（設計レビューで指摘された欠落の是正）
  - 既存呼び出し元（ECS ウィジェット側）の挙動が変わらないことを確認する
  - 拡張後のユーティリティを COM 初期化済みの環境で呼び出し、既知の画像に対して α 有無判定が正しく返ることを確認する
  - _Requirements: 2.1_
  - _Boundary: WIC utility (wintf)_

- [x] 1.3 共有契約型（識別子・幾何・アトラスエントリ・頁バッファ・索引表アクセサ）の定義
  - ランタイム識別子（密な採番）とソース識別子（出所＋相対パスの組）を型として分離定義する
  - 幾何プリミティブ（座標・寸法・矩形）、配置情報、空エントリを表現できるアトラスエントリ型を定義する
  - 頁バッファ型（premultiplied BGRA・stride 明示・スレッド間共有可能な所有形）を定義する
  - 索引表の構築用コンストラクタと、既知/空/未知を区別する問い合わせ・デバッグ用の逆引きアクセサを実装する
  - 索引表と頁バッファがスレッド間で安全に受け渡せる型であることをテストで確認する
  - _Requirements: 6.1, 6.2, 6.4_
  - _Boundary: AtlasTable_

- [x] 1.4 差し替え可能なデコードポートの定義
  - パス入力から画素バッファを得るための最小インターフェースを定義し、デコード手段の詳細を上位層へ露出しない形にする
  - 復号失敗（不在・破損）を診断可能なエラーとして表現する型を定義する
  - テスト用（メモリ）腕を用意し、インターフェースの契約に従って画素バッファを返せることを確認する（既定 WIC 腕の契約適合は 2.2 で別途確認する）
  - _Requirements: 2.3_
  - _Boundary: ElementDecoder (port)_

- [ ] 2. コア: アトラス生成パイプライン各段の実装
- [x] 2.1 (P) マニフェスト導出（surface 列挙・間接参照解決・重複排除）の実装
  - shell surface 群と、surface として表現された balloon の双方から、焼付対象となる element 画像の相対パス一覧を導出する
  - bind アニメーションを介した間接参照（他 surface の element を参照するケース）を解決し、参照先の画像も列挙対象に含める
  - 負の参照や存在しない参照先など、画像を持たない参照は列挙から除外し、循環参照が生じても処理が停止するようにする
  - 出所（shell/balloon）と相対パスの組を重複排除キーとして扱い、決定的な順序（出所昇順・相対パス昇順）でランタイム識別子を採番する
  - emo2 相当の bind 参照集合を列挙し、全ての参照先画像が漏れなく列挙されることをテストで確認する
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 5.6_
  - _Boundary: ManifestDeriver_
  - _Depends: 1.3_

- [x] 2.2 (P) 既定デコード腕（WIC 経由）の実装
  - 拡張済みの WIC ユーティリティを用いて、パス入力から画素バッファとアルファ有無を復号するデコードポートの既定実装を用意する
  - 対象パスに `.pna` が存在するかどうかを判定できるようにする
  - 画像が存在しない場合・復号できない場合を診断可能なエラーとして返し、例外的終了ではなく通常のエラー値として扱う
  - COM 初期化済みの環境で実画像（emo2 fixture）を復号し、画素データとアルファ有無が正しく得られることを確認する（既定腕がデコードポートの契約に適合することの確認を兼ねる）
  - _Requirements: 2.1, 2.2_
  - _Boundary: WicDecoderArm_
  - _Depends: 1.2, 1.4_

- [x] 2.3 (P) 透過正規化（premultiplied BGRA 統一）の実装
  - 透過解釈パラメータを入力として受け取り、自ら設定ファイルを読みに行かない形で正規化処理を構成する
  - アルファチャンネルが有効な場合にそれを透明度として採用し、出力を premultiplied BGRA へ統一する主経路を実装する
  - アルファチャンネルが利用できない場合の代替経路（`.pna` 参照・キーカラー透過）は型としてのみ用意し、到達時は未対応であることを明示するエラーを返す
  - 優先順位（アルファチャンネル＞`.pna`＞キーカラー）に従って採用ソースが選択されることをテストで確認する
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_
  - _Boundary: Normalizer_
  - _Depends: 1.4_

- [x] 2.4 トリミング（αバウンディングボックス・オフセット記録）の実装
  - 正規化済み画像から、透明度を持つ画素のみを過不足なく含む最小の矩形を算出する
  - 元画像内でのオフセット・トリム後寸法・元の寸法を記録し、配置座標とオフセットを組み合わせれば元画像全体を焼き付けた場合と見た目が等価になることを保証する
  - 全画素が透明な画像は空エントリとして扱い、以降の焼付処理をスキップできるようにする
  - 片側だけが不透明な画像でタイトな矩形が得られること、全透明画像で空判定になることをテストで確認する
  - _Requirements: 4.1, 4.2, 4.4, 4.5_
  - _Depends: 1.3, 2.3_

- [x] 2.5 packing（複数頁・padding・決定的配置）の実装
  - トリム済み矩形群を、承認済みの静的パッキングライブラリを用いて頁内に重ならないよう配置する
  - 各矩形の周囲に余白を確保した状態でライブラリへ渡し、実際に記録する座標情報は余白を含まない実矩形とする
  - 全矩形が単一頁に収まらない場合は複数頁へ自然に分割する
  - 入力をマニフェスト採番順（決定的な順序）に揃えたうえで配置を行い、同一入力から常に同一の配置結果が得られることをテストで確認する
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_
  - _Depends: 1.1, 1.3, 2.1, 2.4_

- [ ] 2.6 頁バッファの確保と焼付の実装
  - packing で得られた座標情報をもとに必要な頁数分のバッファを確保する
  - 各トリム済み画像を、対応する頁の座標位置へ premultiplied のまま転写する（画素の変換は行わない）
  - 余白部分の画素が透明のまま保たれ、隣接画像との滲みが生じないことを保証する
  - 転写後の頁画素が元のトリム済み画像と一致すること、複数頁が正しく確保されることをテストで確認する
  - _Requirements: 4.3, 6.3_
  - _Depends: 1.3, 2.4, 2.5_

- [ ] 3. 統合: bake パイプラインの結線と fixture 検証
- [ ] 3.1 bake エントリポイントの結線
  - 列挙・デコード・正規化・トリミング・packing・焼付の各段を単一の入口関数として結線し、複数の入力集合（shell 用・balloon 用など）をまとめて処理できるようにする
  - デコードに失敗したエントリは索引表に載せず、失敗内容を診断可能な形で集約しつつ、他エントリの処理は継続する
  - 索引表と頁バッファの成果物を、通信機構を介さず値・共有参照として直接返す
  - emo2 相当の入力一式を結線済みの入口関数へ通し、索引表と頁バッファが得られることを確認する
  - _Requirements: 6.5_
  - _Depends: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

- [ ] 3.2 emo2 fixture を用いた統合テスト（shell・balloon 横断）
  - emo2 の shell surface 一式を bake し、全ての element 画像が索引表に載り頁が生成されることを確認する
  - surface として表現された emo2 の balloon 画像が、shell と同一の経路で問題なく処理されることを確認する
  - 存在しないパスを含む入力に対しては該当エントリのみがエラーとして扱われ、他のエントリの処理が継続されることを確認する
  - 上記のテストが自動実行環境で成功・失敗を明確に判定できる状態にする
  - _Requirements: 1.1, 1.4, 2.2, 5.6, 6.1, 6.2_

- [ ] 4. 検証: 決定性とリグレッション防止
- [ ] 4.1 決定性（golden）テストの整備
  - emo2 相当の同一入力を複数回 bake し、各画像の配置結果（頁番号・矩形・オフセット・元寸法）が常に同一になることを確認する
  - 得られた配置結果を固定値として記録し、以降の変更でリグレッションが生じた場合に検出できるようにする
  - テストが継続的実行環境で安定して再現することを確認する
  - _Requirements: 5.5_
  - _Depends: 3.2_

## Implementation Notes

- 1.1: `rectangle-pack` は crates.io に 0.5 が存在しない（最新 0.4.2）。workspace 依存は `rectangle-pack = "0.4"`（設計の "0.5" は誤記・同一 zero-dep MIT/Apache クレート）。
- 1.1: **areka-emo-atlas は `wintf` クレートに依存しない**（wintf は bevy_ecs/bevy_app/dola/taffy を非 optional core 依存に持つ monolith ＝境界不変条件「bevy_ecs を引き込まない」に抵触）。設計 D2 の「wintf の WIC ユーティリティを最小 feature で参照」は wintf に ECS feature-gate が無いため実現不能。**確定方針: WIC 腕（2.2）は `windows` WIC を直接使い wintf の `load_bitmap_source` と同等の手順を再現**（`相当` の解釈）。1.2 は wintf 側 util の ECS 非依存化＋has_alpha 露出という並行リファクタで、emo-atlas は 1.2 の成果を literal に import しない（手順・has_alpha 取得点を共有）。Cargo.toml deps = areka-parsers(path)/windows/rectangle-pack/tracing。
- 1.1: モジュール配置は `src/decode.rs`（`pub mod wic_arm;` を含む）＋ `src/decode/wic_arm.rs` のディレクトリモジュール形式で compile 確認済み。
- 1.2: wintf 側 `load_bitmap_source` は `crates/wintf/src/com/wic.rs` へ移設し戻り値を `Result<(IWICBitmapSource, bool)>`（bool=has_alpha）へ拡張。`systems.rs` は `pub use crate::com::wic::load_bitmap_source;` で再エクスポート（既存 import パス互換）。**has_alpha は変換前フレーム `frame.GetPixelFormat()` から判定必須**（PBGRA 変換後は常に α 付き＝誤判定になる・Critical Issue 1）。判定は `const ALPHA_FORMATS: &[GUID]` の `.contains()` 方式（32/64bpp BGRA/PBGRA/RGBA/PRGBA＋1010102XR＋128bpp float α。24bppBGR/8bppIndexed/grayscale/plain RGB は default false）。`matches!` はwindows-rs の非 UPPER_CASE GUID 定数が `non_upper_case_globals` future-incompat 警告を出すため不可。**2.2 の emo-atlas WIC 腕はこの has_alpha ロジックを `windows` WIC 直接で再現する**（wintf を import しない）。
- 1.3: 契約型は `table.rs` に D3 通り定義済み・`lib.rs` から `pub use` 済み。**`SetId` は table.rs に定義**（AtlasKey が内包するため）→ manifest.rs（2.1）はここから import する。`AtlasTable::new(keys, entries, pages)` が resolve マップを keys→ElementId(index) で構築。`entry`/`key` は非 Option の O(1) index（契約違反時 panic）・`new` は len 不一致 assert。幾何 Point{i32,i32}/Size{u32,u32}/Rect{u32×4}。
- 1.4: `decode.rs` に `ElementDecoder` trait（`decode(&Path)->Result<DecodedImage,DecodeError>`＋`probe_pna` default false）・`DecodedImage{width,height,stride,bgra:Vec<u8>,has_alpha}`（Clone,Debug）・`DecodeError{NotFound{path},Decode{path,source}}`（Debug＋std-only Display/Error・thiserror 非追加）を定義。**`pub struct MemoryDecoder`（COM 非依存の再利用可能 test double・`insert`/`insert_corrupt`/`.pna` 登録）**＝2.3/3.x の純粋テストで使える。ポート面に WIC/COM 型を一切露出しない（2.3）。`decode/wic_arm.rs` は 2.2 まで stub のまま。
- 2.1: **`AlphaParams{use_self_alpha:UseSelfAlpha}` と `enum UseSelfAlpha{On,Full,Off}`（Clone,Copy,Debug）は 2.1 が先行して `normalize.rs` に定義済み**（SurfaceSet が内包するため）→ **2.3（Normalizer）はこれらを再定義せず import して使う**。normalize.rs には現状この2型のみ（normalize()/NormalizedImage/AlphaSource/NormalizeError は 2.3 が追加）。`SurfaceSet<'a>{surfaces:&[Surface], base_dir:&Path, alpha_params:AlphaParams}`・`Manifest{keys:Vec<AtlasKey>}`・`ManifestDeriver::derive(&[SurfaceSet])->Manifest` は manifest.rs。SetId=sets スライスの index。dedup+順序は `BTreeSet<(u32,String)>`。derive は base_dir を join せず alpha_params 値も読まない（read-only 素通し）。間接 bind は transitive＋visited-set 循環検出・負/不在 id skip・重複 id 先出優先。
- 2.2: `WicDecoderArm{ factory: IWICImagingFactory2 }`（`decode/wic_arm.rs`）は `windows` WIC を直接使い（wintf 非 import）task 1.2 recipe を再現。`new()->windows::core::Result<Self>`（COM 初期化は呼出側責務）。`decode`: `!path.exists()`→`NotFound`／COM 失敗→`.map_err`→`Decode{path,source}`（panic 皆無・全 unwrap/expect は #[cfg(test)] 限定）。`has_alpha` は変換前 `frame.GetPixelFormat()`＋`ALPHA_FORMATS.contains()`（12 GUID・wintf と同一）。`probe_pna=path.with_extension("pna").exists()`。**emo2 fixture パスは `env!("CARGO_MANIFEST_DIR")/../pilot/examples/shiori-host-32/fixtures/emo2/`**（テストは COM init 必須・`online0.png`=48×16 RGBA で has_alpha=true、`descript.txt`=Decode error、不在=NotFound）。unsafe は wic_arm.rs に隔離。
- 2.3: `Normalizer::normalize(DecodedImage, AlphaParams, has_pna)->Result<NormalizedImage,NormalizeError>`（normalize.rs）。`AlphaSource{AlphaChannel,Pna,KeyColor,Opaque}`（#[non_exhaustive]・PartialEq）・`NormalizedImage{w,h,stride,pbgra:Vec<u8>}`・`NormalizeError::Unsupported(AlphaSource)`。D5 選択: On={α→AlphaChannel/!α&pna→Pna/!α→KeyColor}・Full={α→AlphaChannel/!α→Opaque}・Off={pna→Pna/!pna→KeyColor}（α無視）。**実装腕はタプル `(On, AlphaChannel)` のみ＝恒等 premultiplied 素通し（`pbgra: img.bgra` move・無変換・D8）**。それ以外は全て `Unsupported(選択された source)`（Full+α も seam！）。設定ファイル非読込（params 注入・3.6）。
- 2.4: `Trimmer::trim(&NormalizedImage)->TrimResult{original:Size, placement:Option<Trimmed>}`（trim.rs）。`Trimmed{trim_offset:Point, size:Size, pbgra:Vec<u8>, stride}`。**α 閾値は厳密 `alpha>0`（+3 バイト・NOT 128）**・stride 込み走査（alpha_mask 先例）。全 α==0→`placement:None`（空エントリ）。トリム後は tightly-packed（`stride=size.w*4`・premultiplied 素通し・無変換）。座標不変（4.5）: trim_offset へ blit で原画像 byte 等価。α read は `.get().unwrap_or(0)`。
- 2.5: `Packer::pack(&[(ElementId,Trimmed)], PackConfig)->PackOutput{page_count, entries:Vec<PackedEntry>}`（pack.rs・**座標のみ・pbgra 非読込・blit なし**＝Critical Issue 3）。`PackConfig::default()={2048,1}`。rectangle-pack 0.4.2 API: `GroupedRectsToPlace::push_rect`/`RectToInsert::new(w,h,1)`/`TargetBin::new(page,page,1)`/`pack_rects(_,_,&volume_heuristic,&contains_smallest_box)`/`packed_locations()`（RectId=ElementId.0:u32・BinId=usize）。**pack_rects は all-or-nothing** ゆえ multi-page は bin 数を 1..=items.len() で増やす retry loop・使用 bin を dense 0-based page へ remap。padding: 登録=`w+2p`・UV=`placed+p`（実サイズ・padding 非包含）。決定性: 入力を ElementId 昇順に内部 sort・出力も再 sort（HashMap 反復順に非依存）。oversized（padded>page_size）は tracing::error＋除外（emo2 では発生せず）。
