# 設計検証レポート: areka-P0-shell-implicit-surface

- 検証日: 2026-09-19
- 対象: `design.md`（`requirements.md`・`research.md`・steering と突き合わせ）
- 進め方: 非対話。設計が既存コードについて述べた主張は、名指しされたファイルを開いて定義の実在と挙動を確かめた。検体 3 体の絵は WIC（WPF の `BitmapDecoder`＝areka と同じ PNG 復号器）で実際に復号して確かめた。
- 判定: **GO**（重大な指摘 2 件・いずれも設計文書の補正で済み、構造の作り直しは 0 件）

## 1. 総評

設計は要件 1〜11 の全受け入れ基準を部品とテストへ対応づけており、依存の向き・`Cargo.toml` の変更 0 件・呼び手 5 か所の置き換え・焼く一覧への載せ方のいずれも、実コードと照らして成り立つ。抜き色の「座標 (0,0) の 4 バイト比較」は正典（C9〜C11）に根拠があり、検体 3 体の実測とも一致した。見つかった問題は「既存テスト 1 本が赤になるのに書き換えの一覧に無い」「`emo2` の A／B 比較が、焼いた索引表の出どころ次第で摂動を見逃す」の 2 件で、どちらも設計文書へ 1〜2 行足せば解消する。

## 2. 確かめたこと（主張ごとの結果）

| 設計の主張 | 確かめ方 | 結果 |
|---|---|---|
| 依存の向きは既存のまま・`Cargo.toml` の変更 0 件 | 6 クレートの `Cargo.toml` を読んだ | 成立。`areka-emo-present` は `areka-parsers`・`areka-emo-atlas`・`areka-emo-compose`・`thiserror` を既に持つ。`areka-emo-compose` は `areka-emo-atlas` に依存し逆向きは 0 本。触る 6 クレートは全て `sample-ghost-kit` を `[dev-dependencies]` に持つ |
| 複製は 5 か所で、全部が権威を呼べる | `shell::parse(` の呼び出しを `crates/areka/src`・`crates/areka/examples` で数えた | 本番 2（`build_boot_assets`・`build_shell_assets`）＋ `examples` 3（`emo-present/setup.rs`・`collision-probe/setup.rs`・`window-placement.rs`）＝ 5。`examples` は `areka_emo_present::…` の外部クレートのパスで呼べるので、`crate::` パスは 0 件で済む。`window-placement.rs` の「素材を 1 度組み、対象ごとに面の表を組み直す」形は `ShellTarget::build_world` にそのまま載る |
| `ManifestDeriver::derive`・`bake` の変更 0 件で画像を焼ける | `crates/areka-emo-atlas/src/manifest.rs` の `derive`・`collect_elements`、`lib.rs` の `bake` を読んだ | 成立。`derive` は渡された全 `Surface` の `elements` を無条件に集める。番号の重なり（宣言済みの面と同じ番号の合成 `Surface`）は `by_id` の先勝ちに当たるが、これは間接参照の解決にしか使われず、直接の列挙には影響 0。並びは `BTreeSet<(u32, String)>` で決まり入力順に依らない。`Surface`・`Element`・`ElementPath` は `Clone` で `#[non_exhaustive]` ではなく、外のクレートから組める |
| 期待値ファイルは 1 行増え 0 行減る | `emo2_shell_golden.txt`（54 行・`null.png` の行 0）と `emo2_golden.rs` の `snapshot_table` を読んだ | 見込みは妥当。行は連番を持たず、全透明の項目は `EMPTY orig=…` で配置を持たないので詰め込みの入力に入らない。`Packer::pack` は番号の昇順で並べ直すので、後続の番号が 1 ずれても相対順は同じ。最終の確認は設計どおり「作り直した差分が追加 1・削除 0」で行う |
| 抜き色は (0,0) の 4 バイト比較でよい | 検体の絵を WIC で 32bit 乗算済み BGRA へ変換して数えた（下の表） | 成立。α の無い形式は全画素 α=255 で届き、4 バイトの一致は色 3 成分の一致と同じ意味になる |
| (0,0) は正典に根拠があるか | ukadoc を引き直した | 根拠あり。C9「画像左上の1ドット(座標0,0)と同色の領域は透過色」のほか、`seriko.paint_transparent_region_black`・`\_b[ファイルパス,x,y]` の項も「画像左上の色」「左上ピクセルの色が透過色」と書く。SSP の実測には 0 件も頼っていない |
| `fold_append` の条件の追加は C6 どおり | `crates/areka-emo-compose/src/fold.rs` の `fold_append`・`upsert_surface` を読んだ | 成立。「番号が面の表にも画像の対応にも無ければ既存の `warn!` のまま」は要件 3.8 と一致。画像だけの面へ追記した後に同じ番号の波括弧が来たときの全置換は、既存の `upsert_surface` の規則そのままで、設計は沈黙ルール対応表への追記を予定している |
| 間隔の語の読み替えで解析側のテストは緑のまま | `Interval::Other` を使う箇所を全クレートで数えた | 解析側（`areka-parsers`）は変更 0 なので緑のまま。**ただし `areka-seriko` の本体内テスト 1 本が赤になる（重大な指摘 1）** |
| 要件 9.2 の申し送り先が実在する | パスを確かめた | `.kiro/specs/areka-P0-coverage-roadmap-refresh/brief.md` は実在し、`roadmap.md` の #45 の行が「各 spec の『統合担当への申し送り』の受け皿」と書いている。要件が名指しする `areka-P0-ukadoc-coverage-roadmap` は `.kiro/specs/completed/` に在り、申し送りを消化できないという設計の判断は正しい。完了済みの spec を引受先にした先送りは 0 件 |
| 1 ファイル 1,000 行・テストは兄弟ファイル | 触るファイルの行数を数えた | `balloon.rs` 644・`table.rs` 529・`normalize.rs` 309・`areka-emo-atlas/src/lib.rs` 596・`fold.rs` 272・`world.rs` 256・`assets.rs` 436（減る）・`measure.rs` 485（減る）・`emo2_boot/mod.rs` 673。どれも上限まで 300 行以上の余裕がある。新しいテストファイルの名前は、同じフォルダの別の本番ファイルの名前と衝突しない（衝突 0 件） |

### 検体の絵の実測（WIC で 32bit 乗算済み BGRA へ変換した後）

| 絵 | WIC が報告する形式 | (0,0) の B,G,R,A | (0,0) と一致する画素 | α≠255 の画素 |
|---|---|---|---|---|
| `emo2` の `purple/a/null.png`（382×547） | `Indexed8` | 0,0,0,0 | 208,954／208,954 | 208,954 |
| `konnoyayame` の `surface0000.png`（260×390） | `Indexed8` | 0,255,0,255（緑） | 70,574／101,400 | 0 |
| `konnoyayame` の `surface1031.png`（72×30） | `Indexed8` | 0,255,0,255 | 9／2,160 | 0 |
| `R_POST_and_KOMAINU` の `surface0000.png`（236×462） | `Bgr24` | 255,255,255,255（白） | 59,831／109,032 | 0 |
| `R_POST_and_KOMAINU` の `surface0010.png`（140×160） | `Bgr24` | 255,255,255,255 | 11,816／22,400 | 0 |

設計の「腕の表」と「`tRNS` は掛かって届く」の記述は、5 枚とも実測と一致した。`null.png` は全画素が (0,0) と一致するので、要件 4.7・5.5 のとおり全画素が透明な絵になる。

## 3. 重大な指摘（2 件）

### 指摘 1: `areka-seriko` の既存テスト 1 本が赤になるのに、書き換えの一覧に無い

- **懸念**: `crates/areka-seriko/src/table.rs` の本体内テスト `only_random_and_bindrandom_are_recorded_others_debug_logged` は、「再生しない語」の代表として **`Interval::Other("sometimes")`** を面 30 に置き、`table.animations(30).is_empty()` と、記録に `vocab="sometimes"` が「採らなかった語」として残ることを確かめている。設計どおり `AnimationTable::from_world` が `sometimes` を `LoopTrigger::Random { k: 2 }` として採ると、面 30 に 1 本採られてこのテストは赤になる。設計の Modified Files の `table.rs` の項は「読み替えと冒頭の説明の更新」だけで、このテストの書き換えに触れていない。`research.md` 2.4 節の「書き換える既存テストの実在確認」にも 0 件である。
- **影響**: このテストは要件 11.4（他の語は今日どおり採らず、元の語を `debug!` に残す）を今日留めている唯一のテストである。実装者が赤を消すために面 30 の検査を外すだけで済ませると、要件 11.4 を留めるものが既存側から 0 本になる。
- **提案**: 設計の Modified Files と「`emo2` の不変の示し方」に倣った書き換えの一覧へ、このテストを「消さずに書き換える」ものとして足す。代表の語を `always`（または `runonce`）へ差し替え、面 30 が空であることと `vocab="always"` が残ることを引き続き確かめる。`sometimes` の新しい挙動は設計どおり `table_interval_words_tests.rs` が受け持つ。
- **対応する要件**: 11.3・11.4・11.7・7.10
- **設計の該当箇所**: 「Modified Files」の `crates/areka-seriko/src/table.rs` の項／「areka-seriko ＞ AnimationTable::from_world」／Testing Strategy の Unit Tests 3

### 指摘 2: `emo2` の A／B 比較は、索引表の出どころを決めないと摂動を見逃す

- **懸念**: 設計は要件 5.8 を「`apply_base_images` の層 0 の判定を外すと、面 10 が 336×400 → 427×463 になり、A／B の一致が赤になる」で示すとしている。しかし `crates/areka-emo-compose/src/plan.rs` の `flatten_extent` は、索引表で解決できなかった層を記録なしで飛ばす（`continue`）。A／B を「`surfaces.txt` の `element` だけから焼いた索引表」に対して組むと、判定を外して足された層 0（`surface10.png`）は索引表に無いので飛ばされ、外形は 336×400 のまま・画素も同じで、**テストは緑のまま**になる。面 0 は `surface0.png` が `element0` として既に索引表に在るが、同じ絵の二重重ねなので要件 5.8 が言うとおり検査にならない。設計の文面「同じ焼き結果に対して」は、どちらの索引表かを決めていない。
- **影響**: 要件 5.8・7.8・7.9 の摂動のうち、`shell_target_emo2_tests.rs` の分が空振りになりうる。`measure_tests.rs` の `SCOPE1_W`／`SCOPE1_H` は `build_shell_assets` → `load_shell_target` を通るので赤になるが、設計が挙げた 3 本のうち 1 本が効かないことに実装の着地まで気づけない。
- **提案**: A を「`load_shell_target`（または `build_shell_target`）が返した `ShellTarget::build_world()`」、B を「同じ `ShellTarget::atlas()` に装着した `EmoWorld::build(shell)`」と設計に明記する。権威を通せば、判定を外したとき `surface10.png` が「使った画像」として焼かれ、A の面 10 が 427×463 になって赤になる。併せて「B＝適用前と同じ」は新しいコードの画像 0 件の経路についての主張なので、それを留めているのが `areka-emo-compose` の既存の合成結果のテスト（書き換え 0 件で緑）であることを 1 行添える。
- **対応する要件**: 5.1〜5.3・5.8・7.8・7.9
- **設計の該当箇所**: Testing Strategy「emo2 の不変の示し方」の 2 行目と 6 行目／「摂動」の表の 2 行目

## 4. 設計の強み

1. **要件 3.6 を規約でなく構造で守っている。** 起動と採寸と `examples` 3 本が同じ `load_shell_target` を呼ぶので、「番号 → 画像」の対応・土台の絵・透過の扱いが食い違う経路が 0 本になる。5 か所の複製が消え、`null.png` の注記 5 か所も同時に消える。バルーンの `resolve_balloon_faces`／`select_faces` と同じ「fs を触る入口＋触らない核」の 2 段で、名前の判定も `face_digits_of` の 1 実装を共有する。
2. **足す型を最小に抑えている。** 「`element0` が無いときだけ画像を使う」ので層 0 が必ず空いており、「`element0` より下」を表す新しい層の型は 0 個。`SurfaceSet`（リテラル 43 か所）・解析の結果の型・`LoopTrigger` への追加も 0 件で、焼く側へは `element0` 1 行の `Surface` を渡すだけで済ませている。実コードと照らして、この方法で `derive`・`bake` の変更が 0 件になることを確かめた。

## 5. 重大には数えなかった所見（確かめ済み）

- **`dangling_pattern_targets` は描画メソッドを見ない。** 解析（`decode_animations`）は `pattern` の欄 2 をメソッドに関係なく `surface_id` に入れるので、`start`・`stop`・`insert` などアニメーション番号を取るコマでは、番号が面の番号として照会され、偽の `warn!` が出うる。3 検体では `pattern` のメソッドは全部 `overlay`（`emo2` 47 件・`konnoyayame` 6 件・`R_POST_and_KOMAINU` 0 件）で該当 0 件。既存の `ManifestDeriver::resolve_indirect` も同じ読み方である。既知の限界として設計に 1 行記すか、面を描くメソッドに絞るとよい。
- **「`build_world` は記録を出さない（0 本）」は新しい記録についてだけ正しい。** `fold_shell` と `bind_atlas` の既存の `warn!` は面の表を組むたびに出る。権威が「使う画像を決めるため」に 1 回余分に面の表を組むので、畳み込みの既存の `warn!` が出るシェルでは、1 回の起動での回数が 3 回 → 5 回（読み込み 2 回 ×（決定用 1）＋ 表示用 2 ＋ 採寸用 1）に増える。要件 3.5・8.5 が数える文言には影響 0。
- **`ShellLoadError` の雛形に `#[error("…")]` が無い。** `thiserror` は各枝に表示の文言を要るので、実装時に足す（設計の意図は明らか）。
- **一覧の失敗を `BootWiringError::ShellRead` へ写すと、文言が「shell ファイルの読み取りに失敗: <フォルダ>」になる。** 枝の追加 0 を優先した判断として妥当。真因は権威の `error!` に残る。
- **`R_POST_and_KOMAINU` の抜き色は白**（255,255,255）で、キャラクターの中の純白も抜ける。C10 が明記する正典どおりの挙動であり、設計の Risks に記載済み。実機確認（要件 8.2 ⑴）で目の白などの見え方を 1 度見ておくとよい。
- **`examples` 3 本は `read_to_string`（UTF-8 のみ）から本番と同じ文字コードの扱いへ変わる。** `emo2` は `charset,UTF-8` を宣言しているので結果は同じ、という設計の記述は妥当。
- 要件の全 ID（1.1〜11.7）について、対応する部品とテストが Requirements Traceability の表と直後の「全 ID の対応」に在ることを確かめた。抜けは 0 件。

## 6. 最終判定

**GO。** 既存の構造との食い違いは 0 件、要件の取りこぼしは 0 件、実装の道筋は明確である。指摘 1・2 は設計ディスカッションで `design.md` へ反映すれば足り、構造の変更を伴わない。

### 次の手順

1. 設計ディスカッションで指摘 1・2 を `design.md` に反映する（書き換える既存テストの一覧に `only_random_and_bindrandom_are_recorded_others_debug_logged` を足す／`emo2` の A／B の索引表の出どころを権威経由と明記する）。
2. 所見のうち `dangling_pattern_targets` のメソッドの扱いを、既知の限界として記すか絞るかを決める。
3. `/kiro-spec-tasks areka-P0-shell-implicit-surface` へ進む。
