# ギャップ分析: audit-emo-atlas（絵の焼き込みとアトラスの見直し）

- 測った時点: `main` `4863cb66`（枝 `claude/kiro-start-audit-emo-atlas-d99b4b`・`19378800`）・2026-11-13。範囲の `crates/areka-emo-atlas/` は棚卸㊱の再測定（`069f1a48`）から変わっていない。
- 物差し: `/ponytail:ponytail-audit`（ponytail 5.3.3 の `SKILL.md` を読んで、その作法で行った。5.3.2 は隣にあるが使っていない）。作法のうち 2 つを外した（要件 1.2・1.3）。
  - 「最大 20 件」を外した。指摘は全件を下に並べ、省いた件は無い。
  - 「読まなかった所（Not checked）」を残さない。範囲の 63 ファイルを全部読んだ（下の「読んだ範囲」）。
- 番号は audit の番号（組をまたいで通し番号）。組は audit の 3 つ（必ず直す・直したほうがよい・できれば）のまま。
- `audit-wintf-runtime` から申し送られた 3 件（15＝WIC の α の判定・125＝名の渡し方・130＝`with_arm` の初期化の失敗）は直し済みで、ここでは数え直さない（要件 1.6）。ただし 130 と同じ誤りの写しが範囲の中に 4 つ残っていた＝新しい指摘 4 として数えた。
- この文書はコードを変えていない。決めごとは要件の討議に回す。

## 1. 要約

- **何をするクレートか**: シェル・バルーンの面（`Surface` の列）が参照する絵（element）を全部数え上げ（マニフェスト）、絵を読み（WIC・動く絵は `image`）、透過の扱いを当て（`use_self_alpha`）、α>0 の矩形へ切り詰め（トリム）、2048 四方のアトラスの頁へ詰めて焼き（pack・bake）、索引表（`AtlasTable`）を返す。下流の `areka-emo-compose`・`areka-emo-present`・`areka-emo-text`・`areka-placement`・本体 `crates/areka` がこれを使う。
- **想定した負荷**: 1 人の利用者の 1 台の机で、ゴースト・シェル・バルーンを読み込むたびに 1 回焼く（毎フレームではない）。シェル・ゴーストの画像と `surfaces.txt` はネットから入れたもので、**信用できない入力**として扱った（壊れた・意地の悪いファイルでアプリが落ちたり固まったりしないこと）。
- **指摘の数**: 必ず直す 3 件・直したほうがよい 10 件・できれば 9 件＝**22 件**（テストの指摘は種類ごとに 1 件へまとめ、場所は全部を挙げた）。
- **見つかった型**: ⑴ 信用できない入力に上限が無い（マニフェストの再帰・静止画の寸法）＝1 つのファイルでアプリが落ちる（1・2）。⑵ 設計の決め（D7「頁を超える矩形は自頁」）とコードが食い違い、大きい絵が黙って見えなくなる（3）。⑶ 130 で直した誤りの写しと、テストの組み立て部品の写し（4・10）。⑷ 走らないテスト・失敗しえないテスト・名前と違うことを確かめるテスト（5〜9）。⑸ 古くなった説明・要らない小さなコード（14・15）。
- **公開の形（C19 の約束）**: 1〜21 の直しはどれも、アトラスの公開の形を変えないか、足すだけで済む（各件の「範囲の外への波及」）。公開の形を変えるのは 22（使い手の無い公開の型を内へ下げる）だけ＝議題にした。13 は範囲の外（`areka-parsers`・`areka-emo-compose`）に触る直し方があり、議題にした。
- **大きさ**: 直しは見込みで 19〜23 タスク。20 をまたぐので、組で 2 本に切る案と切らない案を 8 節に出す。

## 2. 読んだ範囲（要件 1.3）

範囲の全 63 ファイルを読んだ。

- 進行役が全行を読んだ: `Cargo.toml`・`src/lib.rs`・`animated.rs`・`bake.rs`・`decode.rs`・`decode/image_arm.rs`・`decode/sniff.rs`・`decode/wic_arm.rs`・`decode/wic_arm_test_support.rs`・`error.rs`・`limits.rs`・`manifest.rs`・`normalize.rs`・`pack.rs`・`table.rs`・`trim.rs`（各ファイルの中のテストを含む）。
- 下請け 3 人（読むだけ）が全行を読み、進行役が指摘の行をすべて開いて確かめた:
  - テスト 甲: `animated_tests.rs`（838）・`decode_animation_tests.rs`・`limits_tests.rs`・`samples_e2e_tests.rs`・`table_frames_tests.rs`・`bake_quiet_tests.rs`・`emo2_e2e.rs`・`emo2_golden.rs`・`sample_test_support.rs`・`log_capture.rs`。
  - テスト 乙: `decode/image_arm_tests.rs`・`decode/sniff_tests.rs`・`decode/wic_arm_palette_tests.rs`・`decode/wic_arm_tests.rs`・`normalize_key_color_tests.rs`・`normalize_rule_tests.rs`・`webp_pin_tests.rs`・`tests/atlas.rs`・`tests/default_balloon_drift_test.rs`・`src/testdata/` の README 2 つと `emo2_shell_golden.txt`。
  - 道具: `examples/gen_animated_samples.rs`・`examples/gen_palette_samples.rs`・`examples/gen_default_balloon/` の 5 ファイル。
- `src/testdata/` の画像 18 ファイル（`animated/` 12・`palette/` 6）は、どれもどこかのテストか道具から名前で使われている（使われていない検体は 0）。README の数と大きさも合っていた。
- 範囲の中に `shortcut:`・`ponytail:` の注釈と `#[allow(dead_code)]` は 0 件（grep）＝残す 3 種類の 2 種類目（わざと置いた先送りの印）に当たる件は無い。

## 3. 指摘の一覧

各件の欄: **場所**／**何か**／**困る場面**／**いちばん小さい直し**／**範囲の外への波及**（アトラスの公開の形が変わるか）／**残す 3 種類に当たるか**（要件 3.1 の 1＝裁定に反する・2＝わざと置いた先送りの印・3＝SSP 互換・ukadoc 正典）。

### 必ず直す

#### 1. マニフェストの間接参照のたどりが、何も足さないのに二乗の手間と深い再帰をする（`manifest.rs:53-67,94-128`）

- **何か**: `ManifestDeriver::derive` は、面ごとに「着せ替え・アニメーションのパターンが参照する面」を再帰でたどり、その面の絵を数える（`resolve_indirect`）。ところが参照先は同じセットの `set.surfaces` の中の面だけで（`by_id` は `set.surfaces` から作る）、その面の絵は 61〜63 行の「全部の面の絵を直に数える」で既に入っている。だからこのたどりは結果に 1 件も足さない。訪問済みの集合は根ごとに作り直すので、手間は面の数の二乗になる。
- **困る場面**: `surface0` のアニメーションが `surface1` を、`surface1` が `surface2` を……と 10 万段つないだ `surfaces.txt`（約 7 MB・打ち間違いでも、わざとでも）を読むと、再帰が 10 万段になってスタックがあふれ、アプリが落ちる。2 万段でも根ごとに残りの鎖をたどるので約 2 億回の集合の出し入れと文字列の確保（`collect_elements` は入っていても毎回 `to_string` する）になり、読み込みが固まる。
- **いちばん小さい直し**: `resolve_indirect`・`by_id`・`visited` を消す（約 50 行）。結果は今と同じ（全部の面の絵の集合）。モジュールの説明の「間接参照解決・循環検出」も消す。テスト＝間接参照の 3 本（`manifest.rs:312-366`）は直に数えるだけでも通るので、「長い鎖（例 5 万段）を短いスタックのスレッドで導いても終わり、絵を 1 度ずつ数える」1 本に替える（直す前は落ちる）。
- **範囲の外への波及**: なし（`derive` の形と結果は変わらない・`bind_only_paths` は別の関数で再帰しない）。
- **残す 3 種類**: 当たらない（結果を変えない）。

#### 2. 静止画の寸法に上限が無く、寸法の掛け算が `u32` であふれる（`decode/wic_arm.rs:99-106`・`animated.rs:58,98,114-133`・`trim.rs:257,296,299`）

- **何か**: WIC の腕は寸法を聞いた直後に `width * 4 * height` の `u32` の掛け算で画素の入れ物を確保する。寸法の上限も、掛け算のあふれの確かめも無い。動く絵には 3 つの上限があるが、上限を超えて 1 枚へ縮める道（`shrink`）は、動きの 1 枚目と今までの 1 枚読みを、寸法を見ずに全体の大きさで読む。
- **困る場面**: ⑴ 見出しに 30000×30000 と書いた数 KB の PNG（中身はほぼ空でよい）を置いたシェルを読むと、3.6 GB の確保を試み、取れなければアプリが落ちる（Rust は確保の失敗で止まる）。⑵ 40000×30000 だと掛け算があふれ、試験の作りでは panic、出荷の作りでは小さな値に化けた入れ物で `CopyPixels` が失敗する（たまたま `Err` で済む）。⑶ 2 コマの APNG の見出しを 100000×100000 にすると、上限で縮めた先の 1 枚目と今までの 1 枚読みが全体の大きさで読もうとする。
- **いちばん小さい直し**: 「一辺の上限」を 1 つ決め（候補: `areka-emo-compose` の外形の上限と同じ 16,384＝GPU の機能水準 11 の一辺）、⑴ WIC の腕は `GetSize` の直後に上限と `checked_mul` で確かめ、越えたら確保せず `Err(DecodeError::Decode)`（呼び手 `shell_target.rs:477` などが `error!` を出す）。⑵ 動く絵は見出しの寸法がこの上限を越えたら、縮める道に入らず同じ `Err` にする。テスト＝見出しだけの PNG（IHDR を手で組む）が `Err` になり確保しない（直す前は試験の作りで panic する）。動く絵は `MemoryDecoder` で見出しだけ大きい絵を登録して `Err` を確かめる。
- **範囲の外への波及**: なし（新しい `Err` は今の `DecodeError::Decode` と同じ形・呼び手はもう `errors` を記録して続ける）。本体 `crates/areka/src/placement_tests/measure_bake_count_tests.rs` は読み手の呼び出し回数を数えるが、普通の大きさの絵では回数は変わらない。
- **残す 3 種類**: 上限の値は議題（議題 1）。種類 1 は記憶「ゴーストの台本の大きさを甘く見ない・上限で断らず」に近いが、台本の話で、GPU で描けない大きさは断るほかない。種類 3 は ukadoc に画像の大きさの決まりが無い（ukadoc MCP で「画像サイズ」を引いて、シェルの画像の上限の項は 0 件）。

#### 3. 一辺が 2,046 画素を超える静止画は、黙って見えなくなる（設計 D7 と食い違う）（`pack.rs:7,95-112`・`bake.rs:57-63`・`lib.rs:277-288`）

- **何か**: pack は余白込みで頁（2048）に入らない矩形を `error!` を 1 行出して外す。索引表にはエントリが残るが `placement = None`（全透明と同じ扱い）になり、合成で描かれない。完了した `areka-P0-emo-atlas` の設計 D7 とこのファイルの説明（`pack.rs:7`）は「頁を超える矩形は自頁（その矩形だけの頁）」と書いており、コードと食い違う。
- **困る場面**: 高解像度のシェルで、体の絵が 600×2100 だと、キャラクターがまるごと見えない。記録は `pack:` の英語の `error!` 1 行だけで、どの絵か（鍵の名前）も出ない（`element_id` の番号だけ）。
- **いちばん小さい直し**: 頁に入らない矩形は、その矩形が入る大きさの頁を 1 枚だけ作って載せる（一辺は指摘 2 の上限まで）。頁ごとの大きさは `PackOutput` に足し（`page_sides` のような欄）、`Baker::bake_pages` は頁ごとの大きさで確保する。`AtlasPage` は今も `width`・`height`・`stride` を持ち、下流はどれも `stride` で引く（`page_size`・`2048` を下流のクレートで grep して 0 件）。テスト＝2100 の高さの絵が `placement` を持ち、画素がそのまま戻る。
- **範囲の外への波及**: 公開の形は足すだけ（`PackOutput` への欄の追加＝`PackOutput`・`Baker` の範囲の外の使い手は 0・grep）。外から見える振る舞いが変わる（今まで見えなかった大きな絵が見える）。1 枚の頁が大きくなるのでメモリは増える（16,384 四方なら 1 GiB）。
- **残す 3 種類**: **種類 3 の議題**（議題 2）。ukadoc に大きさの決まりは無く、SSP は大きな絵を描く見込みだが、測ってはいない。完了した `areka-P0-animated-image-decode` の設計（`design.md:531`）は「大きすぎる静止画は今のとおり外す（変更 0）」と今の振る舞いを書き写しているだけで、開発者の裁定ではない（種類 1 には当たらない見込み）。動く絵の「頁の一辺を超えたら 1 枚へ縮める」決め（同 要件 6）はそのまま残す。

### 直したほうがよい

#### 4. テストの COM の手伝いが 4 か所に写してあり、130 で直した誤りを持ったまま（`decode/wic_arm.rs:210-219`・`emo2_e2e.rs:30-41`・`emo2_golden.rs:43-54`・`samples_e2e_tests.rs:80-89`）

- **何か**: 4 か所とも `CoInitializeEx` の結果を `let _ =` で捨て、いつも `CoUninitialize` を呼ぶ。説明（`emo2_e2e.rs:31,34`・`emo2_golden.rs:44,47`・`wic_arm.rs:212`）は「RPC_E_CHANGED_MODE 許容」と書くが、その場合に入っていない apartment を壊す。正しい形は 130 で直した `decode/wic_arm_test_support.rs:12-26` の `with_arm` だが、`pub(super)` なので `decode/` の外から使えない。
- **困る場面**: 既に STA に入ったスレッドでこのテストを回すと、テストが入っていない apartment を `CoUninitialize` で壊す。ほかの失敗の値は捨てられ、後で「WIC factory」の失敗としてだけ出る。今の `cargo test` のスレッドは COM に入っていないので、表には出ていない。
- **いちばん小さい直し**: `with_arm` と、腕を作らない形（`with_com`）をクレートの直下のテストの部品（`sample_test_support.rs` か新しい `#[cfg(test)]` の部品）へ上げ、4 か所の写しを消す。
- **範囲の外への波及**: なし（テストだけ）。
- **残す 3 種類**: 当たらない。

#### 5. 道具 `gen_default_balloon` の `main.rs` のテスト 4 本は一度も走らない（`examples/gen_default_balloon/main.rs:127-170`・`Cargo.toml`）

- **何か**: `args_are_none_or_check_only`・`first_difference_finds_byte_and_length_changes`・`missing_destination_folder_writes_nothing`・`io_text_is_ascii_kind_and_number` の 4 本。`cargo test` は example を作りはするが、`test = true` が無い example のテストは走らせない。`Cargo.toml` に `[[example]]` は無い。ずれの見張り（`tests/default_balloon_drift_test.rs:10-17`）は `build`・`draw`・`texts`・`webp` だけを `#[path]` で読む。
- **困る場面**: 終わりの番号（0・1・2）・`--check` のバイトの比べ・「置き場が無いときは書かない」の約束を壊しても、全体テストは緑のまま。
- **いちばん小さい直し**: `Cargo.toml` に `[[example]] name = "gen_default_balloon"`・`path`・`test = true` の 4 行を足す（子のテストが 2 度走るが短い）。別案は 4 つの関数を子のファイルへ移し、見張りの `#[path]` で一緒に読ませる。
- **範囲の外への波及**: なし（このクレートの `Cargo.toml` だけ・依存は足さない・`Cargo.lock` は変わらない）。
- **残す 3 種類**: 当たらない。直し方は議題 5。

#### 6. 壊れても落ちる檻の無い分岐（テストの抜け）

- **場所と何か**（どれも開いて確かめた）:
  - ⑴ `bake_quiet` でも動く絵の 3 本の `warn!` は出る（`lib.rs:86-88` の約束・`animated.rs:48-57,91-97,123-129`）。`bake_quiet_tests.rs` の 2 本（132-153）はどちらも動く絵を登録しない。3 本を `records.emits()` で包んでも全部緑のまま＝本番の黙る道（`areka-emo-present` の `shell_target.rs:621`・`balloon.rs:806`）で動く絵の警告が黙って消える。
  - ⑵ 複数の動く絵のコマの並び（親の番号の昇順・要件 4.7・`lib.rs:261`・`animated.rs:192-219`）。動く絵が 2 つ以上のテストは `animated_tests.rs:590-624` だけで、`is_some()` しか見ない。ほかは同じコードの 2 回の焼きの比べなので、並びを逆にしても両方が同じに変わって通る。
  - ⑶ `animated.rs:62` の `frames.len() >= 2` の守り。本物の見分け（`sniff.rs`）は 2 枚以上しか答えないが、`MemoryDecoder`・ほかの読み手は `frame_count: 1` を返せる。守りを消すと 1 コマの `Animation` ができ、`AtlasTable::with_frames` が `table.rs:180-185` で panic する（外からの入力で `bake` の中が落ちる）。1 コマを試すテストは 0。
  - ⑷ `decode/wic_arm.rs:169-174` の 1 ビット・2 ビットの番号つきの形式。検体は 4 ビットと 8 ビットだけ。片方を表から落とすと、`tRNS` を持つ 1・2 ビットの PNG は `SupportsTransparency`（番号つきは FALSE）へ回り `has_alpha = false` になり、`On` では左上の色で抜かれる。
  - ⑸ `limits.rs:76-83` の `from_env` の `warn!` の繰り返し。`from_lookup`・`resolve_limit` は試すが、本物の環境から警告を出す 1 行は誰も確かめない（プロセスで 1 回だけの `OnceLock` なので試しにくい）。
- **いちばん小さい直し**: ⑴ `bake_quiet_tests` に全コマが `Err` の動く絵を 1 つ足し、「動く絵として読めなかったので」が `bake_quiet` でも 1 行出ることを確かめる。⑵ `animated_tests.rs:604` の最初の焼きで `frames` の並びを数で比べる。⑶ `insert_animated` で 1 コマの見出しと 1 コマの中身を登録し、1 枚へ縮んで `warn!` が出て panic しないことを確かめる。⑷ `gen_palette_samples` に 1・2 ビットの検体を足し、`has_alpha` のテストを 2 本。⑸ 本物の環境の読み口を 1 つの関数に切り（`from_lookup` に渡す前の段）、警告の出し方だけを `log_capture` で確かめるか、試さないと決めて残す（議題にはしない・設計で決める）。
- **範囲の外への波及**: なし（テストと検体だけ・⑷ の検体は `crates/wintf/tests/com/wic_test.rs` も `palette/` を読むが、足すだけなので壊れない）。
- **残す 3 種類**: 当たらない。

#### 7. 「環境変数に触らない」と書いたテストが、開発者の環境変数で赤になる（`animated_tests.rs:694-710`）

- **何か**: `bake_without_limits_uses_the_default_limits` は説明に「環境変数は触らない」と書くが、`crate::bake` は `AnimationLimits::from_env()`（本物の環境）を読む。期待は既定の上限で 3 コマ。
- **困る場面**: `AREKA_ANIMATED_IMAGE_MAX_FRAMES=2` を設定した机でテストを回すと、3 コマの絵が縮んで赤になる（コードは正しい）。同じく `bake(...)` を呼ぶほかのテスト（`lib.rs`・`bake_quiet_tests.rs`・`emo2_*`）も、動く絵を入れると環境に依る。
- **いちばん小さい直し**: このテストの期待を `bake_with_limits(.., AnimationLimits::from_env())` との一致に替える（環境に依らずに「`bake` は `from_env` を使う」を確かめる）。名前と説明を事実に合わせる。
- **範囲の外への波及**: なし。
- **残す 3 種類**: 当たらない。

#### 8. 失敗しえないテスト・弱いテスト（偽の安心）

- **場所と何か**（どれも開いて確かめた）:
  - `emo2_e2e.rs:364-376`: `BakeError` は `Decode` の 1 つしか無いので `matches!(e, BakeError::Decode(_))` はいつも真。実際に確かめているのは「失敗が 1 件以上」だけ。別の絵が落ちても、無い名前が `NotFound` でなく `Decode` になっても通る（同じことの `MemoryDecoder` 版 `lib.rs:516-550` は件数と名前を見る）。
  - `decode/sniff_tests.rs:304-308`（VP8X そのものが残りより長い）: 狙いの守り `sniff.rs:70` の `|| 20 + len > total` は、無くても同じ答え（続く `read_at` が読めずに `None`・あるいは `frames` が 0）＝テストも守りも働いていない。
  - `decode/sniff_tests.rs:225,295`: `reader.read <= 長さ` は、名前の付く誤り（長さの欄でメモリを取る）を作っても `Cursor` が残りしか読まないので通る（`:195` の `< 64` は効いている）。
  - `lib.rs:857-872`（`bake_result_is_plain_value_channel_independent`）: 型の事実で、本文は 457-510 と同じ。`lib.rs:624` の `let _ = result.table.entry(id);` は直前の範囲の確かめで済んでいる。
  - `table.rs:377-391`（`..._cheap_clone`）: 名前は「安価な clone」だが、`len` と `resolve` を比べるだけで手間は何も見ない（しかも指摘 16 のとおり安価でない）。`Send + Sync` の半分は本物。
  - `decode_animation_tests.rs`: 大半は試験の写し（`MemoryDecoder`）を試している。本番の既定の 3 メソッドを試すのは 66-74 だけ。169-178（`dyn` 越し）はコンパイルの事実（`decode.rs:366-370` と同じ）。
  - `table_frames_tests.rs:110-139`: 4 本とも `#[should_panic]` に `expected` が無く、どの約束の panic かを固定しない。あわせて `with_frames` は `frames[0] == 親` を確かめない（`table.rs:102` の説明の約束）。
- **いちばん小さい直し**: 件数と名前を見る形に直す（`emo2_e2e`）。`sniff.rs:70` の働かない守りの項と `:304-308` を消す。`:225,295` の数える包みと 2 つの比べを消す。`lib.rs:857-872` と `:624` を消す。`table.rs` のテストは名前を事実へ（`Send + Sync` だけ）。`decode_animation_tests.rs` は既定の 3 メソッドの 1 本と、`bake` が使う写しの約束だけ残す。`should_panic` に `expected` を付け、`with_frames` に `frames[0] == 親` の確かめを 1 行足す。
- **範囲の外への波及**: `with_frames` の確かめの追加は、範囲の外の呼び手（`areka-emo-compose/src/film_tests.rs:231`）が `frames[0]` に親を置いているかを確かめてから入れる（置いていなければそのテストも直す＝申し合わせ）。
- **残す 3 種類**: 当たらない。

#### 9. 同じことを 2 度確かめているテスト（本物の WIC の焼きを含む）

- **場所と何か**:
  - `emo2_e2e.rs:93-200` と `204-239`: 同じ emo2 のシェルを同じ形で本物の WIC で 2 度焼く（55 枚ずつ）。後の 1 本は記録を捕まえる点だけが違う。
  - `emo2_golden.rs:159-192`（2 回焼いて比べる）は、`207-226`（固定の golden との一致）が毎回通れば成り立つ。残る「失敗 0 件」は `emo2_e2e.rs:145` も見る＝本物の焼き 2 回が余分。
  - `animated_tests.rs:496-505`: 理由「frame failure」と「unsupported form」は同じ道（`decode_frames` が文を返す）で、文が違うだけ。
  - `lib.rs:553-586`（全透明は空エントリ）は `633-677`（全透明で `warn!`）が全部を確かめ直している。
  - `normalize_key_color_tests.rs:39-54`・`88-102`・`187-220`（と空の絵の部分 `211-219`）は `normalize_rule_tests.rs:49-228`・`266-281` の行と同じ入力・同じ期待。
  - `decode/wic_arm_tests.rs:93-107`・`109-119` は `decode/image_arm_tests.rs:252-267`・`181-188` と同じ（WIC の腕の 2 メソッドは `image_arm` を 1 行で包むだけ・パス付きの包みは `:121-128` が見る）。
  - `examples/gen_default_balloon/draw.rs:238-263`（大きさの表を写した 13 行）と `:355-360`（同じに描ける）は、`build.rs:95-100`・`:146` とずれの見張りが覆う。どちらも 13 枚を描き直す。
- **いちばん小さい直し**: 上の後ろの側（または重い側）を消すか、1 本へまとめる（`emo2_e2e` は最初の 1 本を記録の捕まえの中で焼いて `null.png` の確かめを足す）。
- **範囲の外への波及**: なし。
- **残す 3 種類**: 当たらない。

#### 10. テストの組み立て部品が多くのファイルに写してある

- **場所**:
  - `full`・`view`: `animated_tests.rs:155-170,398-405` と `samples_e2e_tests.rs:102-127`（そのまま同じ）。`dump`（`animated_tests.rs:173-193`）と `fingerprint`（`bake_quiet_tests.rs:114-129`）はほぼ同じ。
  - `elem`・`surface`: `lib.rs:386-403`・`manifest.rs:236-253`・`bake_quiet_tests.rs:25-41`。着せ替えのシェル（面 3000 を `overlay` で参照）: `bake_quiet_tests.rs:45-70` と `lib.rs:744-760`。
  - `emo2`・`shell_master_dir`・`on_params`・`surfaces.txt` を読んで解く数行: `emo2_e2e.rs:44-58,95-97,206-208,248-250` と `emo2_golden.rs:57-71,79-81`。
  - 読み手のテストの `sample`・`info`・画素を読む関数: `decode/image_arm_tests.rs:15-27,45-49`・`decode/wic_arm_tests.rs:13-31`・`decode/wic_arm_palette_tests.rs:12-25`・`samples_e2e_tests.rs:56`。
  - 正規化のテストの `image`・`params`: `normalize_key_color_tests.rs:11-30` と `normalize_rule_tests.rs:10-27`。空の `DecodedImage` は 3 か所（`normalize_key_color_tests.rs:172-178,212-218`・`normalize_rule_tests.rs:267-273`）。
  - 記録の文言の定数: `animated_tests.rs:348-351` と `samples_e2e_tests.rs:50-52`、`bake_quiet_tests.rs:14-17`・`lib.rs:781-782`・`animated_tests.rs:716`。
- **困る場面**: 記録の文言や `Surface` の欄を 1 つ変えると、5〜6 ファイルを同じに直す。直し忘れたファイルのテストだけが古い前提を確かめ続ける。
- **いちばん小さい直し**: クレートの直下に `#[cfg(test)]` の部品を 1 つ（指摘 4 の COM の手伝いと同じ所）、読み手のテストの部品を `decode/` に 1 つ置き、写しを消す。文言は本番の側に定数を置いて、テストはそれを使う。
- **範囲の外への波及**: なし。
- **残す 3 種類**: 当たらない。

#### 11. 索引表の 2 つの組み立て口が、同じ鍵の扱いで食い違う（`table.rs:135-155` と `166-204`）

- **何か**: `AtlasTable::new` は `collect()` で逆引きの表を作るので、同じ鍵が 2 つあると後の番号が勝つ。`with_frames` は先の番号を残す（説明 4.3）。
- **困る場面**: 同じ鍵を 2 つ渡すと、`new` と `with_frames` で `resolve` の答えが違う。今の範囲の外の呼び手は空の列だけを渡す（`crates/areka/` のテスト 8 本・grep）ので表には出ていない。
- **いちばん小さい直し**: `new` を `with_frames(keys, entries, pages, Vec::new())` の 1 行にする。
- **範囲の外への波及**: 形は変わらない（呼び手の答えも変わらない）。
- **残す 3 種類**: 当たらない。

#### 12. 頁の数を 1 から 1 つずつ増やし、毎回全部を詰め直す（`pack.rs:132-159`）

- **何か**: 全部の矩形が入るまで、頁の数を 1、2、3……と増やし、そのたびに `pack_rects` で全部を最初から詰める。大きい順に詰めるので、失敗するのは小さい矩形が残る最後のほうで、失敗した試しもほぼ全部の手間がかかる。
- **困る場面**: 頁が 10 枚になるシェル（絵の数が多い着せ替えのシェル）では、詰めの手間が 10 倍になる。絵が数千あると、読み込みのたびに秒の単位で待つ見込み（測っていない＝10 節）。
- **いちばん小さい直し**: 最初の頁の数を「余白込みの面積の合計 ÷ 頁の面積」の切り上げから始める。それより少ない頁で入ることは無いので、出力は今と 1 バイトも変わらない（golden も変わらない）。テスト＝今の `multi_page_split` ほかがそのまま通り、面積の下限から始めたことを試しの数で確かめる。
- **範囲の外への波及**: なし。
- **残す 3 種類**: 当たらない。

#### 13. 「欄 2 が面の番号でない 8 語」の表が `areka-emo-compose` の写し（`manifest.rs:156-174`）

- **何か**: 着せ替えの「無し」の絵を見分ける `bind_only_paths` は、`start`・`stop`・`alternativestart`・`alternativestop`・`parallelstart`・`parallelstop`・`insert`・`move` の 8 語を面を指さない語として外す。この表と綴りの比べ方は `areka-emo-compose/src/pattern_word.rs`（自分で「唯一の正本」と書く）の写しで、説明もそう書く。アトラスは compose に依存できない（compose がアトラスに依存する）。
- **困る場面**: compose の表に 1 語足すか、綴りの比べ方を変えると、アトラスの見分けだけが古いまま残り、着せ替えの「無し」の絵の記録が `warn` と `debug` で食い違う（描画は変わらない・記録だけ）。あわせて ukadoc は着せ替えの中のこれらの語を overlay として扱うと書く（下の引用）が、この関数は着せ替えの中でも外す（説明は「迷ったら警告の側」）。
- **いちばん小さい直し**: 案 甲＝表を `areka-parsers` の `DrawMethod` へ 1 つにまとめ、compose とアトラスの両方がそれを使う（範囲の外の 2 クレートに触る・parsers の公開の形に足す）。案 乙＝この spec では写しのまま、説明に正本の場所と「変えたら両方」を書く。案 丙＝C20 以降へ回す。
- **範囲の外への波及**: 案 甲は `areka-parsers`（公開の形に足す）と `areka-emo-compose`（`pattern_word.rs`）に触る。どちらも C19 で走っている spec の範囲ではない（両方の見直しは完了済み）。
- **残す 3 種類**: **議題 4**。ukadoc（<https://ukatech.shillest.net/ukadoc-spec/descript_shell_surfaces.html#introduction_drawmethod>）の原文は「着せ替え定義中では、画像の合成でないメソッド（move、insert、start、stop、alternativestart、alternativestop）は無効である（overlayとして扱われる）。」。記録の段の違いだけで描画には関わらないので種類 3 には当たらない見込み。

### できれば

#### 14. 古くなった説明（コードの説明とテストの説明）

- `lib.rs:13`「本タスクでは雛形のみ」＝全部実装済み。
- `animated.rs:6-7`「呼び手（`bake_with_limits`）」＝今の呼び手は `bake_core`（入口 3 つの中身）。
- `bake.rs:8-9`「空エントリ統合は `bake` 入口＝task 3.1」・`bake.rs:32-37`「承認済みシグネチャ調整」の経緯・`table.rs:22-23`「manifest.rs〔task 2.1〕」＝作ったときのタスクの話。
- `table.rs:112`「`Arc` 共有ゆえ `Clone` は安価」＝逆引きの表は `Arc` でない（指摘 16）。
- `pack.rs:7`「超過矩形は自頁」・`pack.rs:96-98`「emo2 は全部 2048 未満ゆえ実運用では発火しない」＝コードは外す（指摘 3 で直れば前者は正しくなる）。
- `emo2_e2e.rs:31,34`・`emo2_golden.rs:44,47`・`decode/wic_arm.rs:212`「RPC_E_CHANGED_MODE 許容」＝しない（指摘 4）。
- `decode/wic_arm.rs:282-288`＝消したフォールバックの経緯。`decode/wic_arm_tests.rs:1-6`＝「動く絵の 3 メソッドの兄弟テスト」だが、`pixel_format_has_alpha_asks_wic`（141-187）と `decode_opens_name_with_lone_surrogate`（189-205）も持つ。
- `decode/image_arm.rs:139-140`「`u32` の上限で止める」＝届かない（指摘 15）。
- `normalize_rule_tests.rs:234`「詰め物は 0,0,0,0」＝実際は `[9, 9, 9, 0]`。
- `examples/gen_default_balloon/webp.rs:4`「example 同士は共有できず、前例を直すと検体のバイト列が動く」＝どちらも事実と違う（ずれの見張りが既に `#[path]` で共有している・振る舞いを変えない移しではバイトは動かない）。
- `tests/default_balloon_drift_test.rs:8`「子のファイルのテストも一緒に走る」＝正しいが、`main.rs` のテストは走らない（指摘 5）。
- **直し**: 事実に合わせるか消す。**波及**: なし。**残す 3 種類**: 当たらない。

#### 15. 要らない小さなコード

- `tests/atlas.rs`（10 行）＝中身もテストも無く「本タスクは雛形」と書く空の結合テストの入口。「`tests/` の唯一の入口」と書くが、`default_balloon_drift_test.rs` がもう 1 つある。毎回 1 つ余分に結合テストを作る。消す（ツリー全体で名前の参照は 0・`structure.md` の慣行の文は確かめて直す）。
- `decode/image_arm.rs:146` の `.unwrap_or(u32::MAX)`＝`(2n+d)/(2d) <= n <= u32::MAX` なので届かない。`as u32` にして説明を消す（`image_arm_tests.rs:279` は飽和を試していないことになる）。
- `sniff.rs:70` の `|| 20 + len > total`（指摘 8）。
- `lib.rs:291` の `let pages_vec: Vec<AtlasPage> = pages;`＝名前を付け替えるだけ。`lib.rs:281-289` は `survivor_keys` を数えるだけに使う（`survivor_originals` を直に回せばよい）。
- **波及**: なし。**残す 3 種類**: 当たらない。

#### 16. `AtlasTable` の `clone` が逆引きの表をまるごと写す（`table.rs:112,120`）

- **何か**: 説明は「`Arc` 共有ゆえ `Clone` は安価」だが、`resolve: HashMap<AtlasKey, ElementId>` は `Arc` に入っていない。`clone` のたびに全部の鍵の文字列を確保して写す。
- **困る場面**: 本体（`crates/areka/src/emo2_boot/assets.rs:365`）・`areka-seriko/src/settle.rs:76`・`areka-placement` の道具が表を `clone` する。絵が数千のシェルでは、そのたびに数千の文字列を写す（読み込みの単位なので大きくはない）。
- **いちばん小さい直し**: `resolve` を `Arc<HashMap<..>>` にする（3 行）。
- **範囲の外への波及**: なし（内の欄・公開の形は変わらない）。
- **残す 3 種類**: 当たらない。

#### 17. 道具の中の写し（`examples/`）

- **場所**: `examples/gen_default_balloon/webp.rs:10-13`（旗の定数）・`:16`（`riff_chunk`）・`:25`（`u24`）・`:31`（`encode_vp8l`）は `examples/gen_animated_samples.rs:227-230`・`:202`・`:211`・`:217` の写し。`crc32`・`png_chunk` は `gen_animated_samples.rs:48-70` と `gen_palette_samples.rs:17-48` に 2 つずつ（`decode/sniff_tests.rs:64` の `png_chunk` はわざと CRC を 0 にする別物）。
- **困る場面**: RIFF の組み方を片方だけ直すと、検体の作り方とバルーンの作り方がずれる。
- **いちばん小さい直し**: `main.rs` を持たない下のフォルダ（例 `examples/common/riff.rs`）に 1 つ置き、両方の道具とずれの見張りから `#[path]` で読む（`examples/` の直下に置くと example として拾われて作れない）。出力のバイトは変わらない。`areka-nar` の `crc32` を使う案は依存の追加（ワークスペースの中・`Cargo.lock` の 1 行）になるので取らない（要件 6.4）。
- **範囲の外への波及**: なし（書き出す `vendors/sample_ghost/` の検体は変わらない＝作り直さない）。
- **残す 3 種類**: 当たらない。

#### 18. 絵の入れ物の長さの約束（`len == stride * height`）を確かめないので、合わない読み手で切り詰めが落ちる（`decode.rs:26-39,171-190`・`trim.rs:261,302-303`）

- **何か**: `ElementDecoder` の約束は「`bgra.len() == stride * height`」だが、焼きの段は確かめない。切り詰めの走査（`trim.rs:261`）は足りない所を `unwrap_or(0)` で読み飛ばすのに、写す所（`:302-303`）は範囲の外を添字で読んで panic する。`MemoryDecoder::insert` も長さを確かめない。
- **困る場面**: 範囲の外の読み手（`areka-placement` の `FirstFrameDecoder`・テストの数える読み手）や `MemoryDecoder::insert(p, 2, 2, 8, 12 バイト, true)` で、2 行目に α が在ると、`bake` の中が panic する。
- **いちばん小さい直し**: `bake_core` で読めた絵（と動く絵の各コマ）の長さを 1 度確かめ、合わなければ `Err(DecodeError::Decode)`（理由に寸法と長さ）。
- **範囲の外への波及**: なし（形は変わらない・正しい読み手の答えは変わらない）。
- **残す 3 種類**: 当たらない。

#### 19. `lib.rs` の 873 行のうち約 500 行が中のテスト（`lib.rs:371-873`）

- **何か**: 1,000 行の番人に近い（873 行）。指摘 2・3・18 の直しは `lib.rs` に行を足す。このクレートはもう `bake_quiet_tests.rs` のように兄弟のファイルへテストを置く形を使う。
- **いちばん小さい直し**: `mod bake_entry_tests` を兄弟の `bake_entry_tests.rs` へ移す（`lib.rs` は約 370 行になる）。指摘 9・10 の整理と同じタスクで行う。
- **範囲の外への波及**: なし。番人の例外表には足さない。
- **残す 3 種類**: 当たらない。

#### 20. パレットのテストが、125 で直した名の渡し方の写しを残し、範囲の外の検体を読む（`decode/wic_arm_palette_tests.rs:155-167`）

- **何か**: `:160` が `HSTRING::from(png.to_string_lossy().as_ref())`（125 で本番から消した形）。番号つきでない PNG がほしいだけなのに、`sample-ghost-kit` 越しに emo2 のバルーン（`online0.png`）を読む。
- **いちばん小さい直し**: `HSTRING::from(png.as_path())` にし、検体はこのクレートの `testdata/animated/basic.apng`（RGBA の PNG）にする。
- **範囲の外への波及**: なし。
- **残す 3 種類**: 当たらない。

#### 21. `image` の版が上がると、振る舞いが同じでも赤になる比べ（`decode/image_arm_tests.rs:197`）

- **何か**: `err.contains("Rgba16")` は `image` クレートの失敗の文の一部。`image` が文を言い換えるだけで赤になる。
- **いちばん小さい直し**: `"frame 0"`（こちらの文）だけを見る。
- **範囲の外への波及**: なし。
- **残す 3 種類**: 当たらない。

#### 22. 範囲の外に使い手の無い公開の型と関数（`lib.rs:24-32,321,330,333,336`）

- **何か**: `pub mod` の 8 つと再公開のうち、`Packer`・`PackOutput`・`PackedEntry`・`Baker`・`Trimmer`・`TrimResult`・`Trimmed`・`ManifestDeriver`・`Manifest` は、範囲の外のコードの使い手が 0（ツリー全体を grep・説明の文の名指しは `areka-emo-present` の 2 か所だけ）。
- **困る場面**: 中の段取りを変えるたびに、公開の形が変わるかを確かめる手間が残る（指摘 3 の `PackOutput` への欄の追加もこのため「公開に足す」になる）。
- **いちばん小さい直し**: `pub(crate)` へ下げ、使い手のある型（`AtlasTable` ほか・`ElementDecoder` ほか・`AlphaParams`・`Normalizer`・`NormalizedImage`・`PackConfig`・`SurfaceSet`・`BakeError` ほか）だけを公開に残す。
- **範囲の外への波及**: **公開の形を変える（減らす）**。コードの使い手は 0 なので壊れないが、C19 の約束「足すことのほかは変えない」（要件 6.2）の字面に当たる。
- **残す 3 種類**: **議題 3**（要件 6.3＝`runtime-module-untangle` と申し合わせて C19 で直すか、C20 へ回すか）。

### 決めごととして指摘に数えなかったもの

- `shortcut:`・`ponytail:` の注釈と `#[allow(dead_code)]` は範囲に 0 件。
- 根の `Cargo.toml` の `[patch.crates-io]` の `image-webp` の固定（10-04 承認）と、それを見張る `webp_pin_tests.rs`: 見張りは正しく働く（各分岐に赤を立てるテストがある）。固定そのものは範囲の外（根の `Cargo.toml`・要件 6.4 で変えない）。記憶「外部クレートはフォークも [patch] もせず」（10-30）より前の承認で、外す条件（0.2.5 以上が crates.io に出たら）は `tech.md` に登記済み＝範囲の外の気づきとして 10 節に残すだけ。
- 読み込みに失敗した絵の記録: アトラスは `errors` を返すだけで `error!` を出さないが、呼び手（`areka-emo-present/src/shell_target.rs:477`・`balloon.rs:811`）が全部を `error!` で出す＝記録の無い失敗の道ではない。
- `rectangle-pack` の詰めの決定性: 中の表は `BTreeMap` で、並べ替えは安定（`rectangle-pack-0.4.2/src/lib.rs:145-153,302-343` を読んだ）＝入力の順が同じなら同じ出力。`packed_locations` は `HashMap` だが、受け取った後に番号で並べ直している（`pack.rs:207`）。

## 4. 要件と手持ちの対応（Requirement-to-Asset Map）

| 要件 | 手持ち | ギャップ |
|---|---|---|
| 1 洗い出し | 本文書（22 件・全ファイル読了） | なし（要件 1.5 の書き足しは討議の後） |
| 2 全部直す | 1・4〜12・14〜21 は範囲の中で閉じる | **Constraint**: 3 は外から見える振る舞いが変わる・13 は範囲の外に触る案がある・22 は公開の形を減らす |
| 3 残す裁定 | 議題の候補は 2（上限の値）・3・13・22 | **Unknown**: 3 は SSP で測るかを決める（ukadoc に決まりが無い） |
| 4 直した後 | 全体テスト `tools/test-all.ps1`・`log-capture-kit`・`MemoryDecoder`・WIC の本物の読み手のテスト（決定論の土台は揃っている） | **Missing**: 1〜3・6 の「壊れたら落ちるテスト」は今 0 本 |
| 5 範囲の外 | 呼び手を grep で確かめ済み（各件の欄） | **Constraint**: 8 の `with_frames` の確かめは `areka-emo-compose/src/film_tests.rs:231` を確かめる・13 の案 甲は parsers と compose |
| 6 C19 の約束 | `crates/areka/` に触る直しは 0 件。依存の追加は 0 件（`Cargo.lock` は変わらない）。`crates/areka-emo-text/` に触らない | **Constraint**: 22 は要件 6.2 の字面に当たる（議題 3） |
| 7 守る決まり | `lib.rs` 873 行・`animated_tests.rs` 838 行 | 19 で `lib.rs` を分ける。`animated_tests.rs` に足すテスト（6 の ⑵⑶）は兄弟の新しいファイルへ |
| 8 大きさ | — | **Constraint**: 見込み 19〜23 タスク（8 節） |

## 5. 直し方の選択肢

- **案 A（今のファイルの中で直す）**: 全件を今のファイルの中で直す。新しいファイルはテストの部品 2 つ（クレート直下・`decode/`）と、`lib.rs` から出すテストのファイル 1 つ、`examples/common/riff.rs` だけ。指摘 3 は `pack.rs`・`bake.rs` の中で頁ごとの大きさを持つ。
  - ✅ 構造を変えない・差分が小さい・公開の形は `PackOutput` への欄の追加だけ。❌ 指摘 3 の頁ごとの大きさで `bake_pages` の引数の意味が変わる（`page_size` は「普通の頁の一辺」になる）。
- **案 B（pack と bake を 1 つにまとめてから直す）**: `Packer`・`Baker` を 1 つの内の段にし（指摘 22 で `pub(crate)` に下げる前提）、頁ごとの大きさを外へ出さない。
  - ✅ 指摘 3 で公開の形に足すものが無くなる。❌ 22 の裁定（C20 へ回す）を待つ・差分が大きい。
- **案 C（混ぜる）**: 必ず直すの 1・2 とテストの直し（4〜10）は案 A で先に入れ、3 は議題 2 の裁定の後に案 A で入れる。22 を C19 で直すと決まったときだけ、3 を案 B の形へ寄せる。

## 6. 手間と危うさ

- **手間**: M（3〜7 日）。直しの 1 件ずつは小さいが、テストの整理（8〜10）の行数が多い。
- **危うさ**: Low〜Medium。1・11・12・16 は出力を変えない。2 は上限の値の裁定、3 は外から見える振る舞いと頁のメモリの裁定が要る。テストの整理は本物の WIC のテストに触るので、直しの前後で同じ golden を確かめる。

## 7. 設計の段への申し送り

- 推す並び: テストの部品を上げる（4・10）→ 必ず直す（1・2・3）→ 直したほうがよい（5〜9・11〜13）→ できれば（14〜22）。部品を先に上げると、テストを足す・消す手間が 1 度で済む。
- 決めること: 一辺の上限の値と、上限を越えた絵の扱い（2）・大きな絵を自頁に載せるか（3）・道具のテストの走らせ方（5）・8 語の表の置き場（13）・使い手の無い公開の型（22）。
- 指摘 2 のテストは、WIC が巨大な見出しの PNG を `GetSize` まで通すかで形が変わる（10 節の 1）。

## 8. タスクの数の見込みと切り方（要件 8.1）

| 組 | 指摘 | タスクの見込み |
|---|---|---|
| 必ず直す | 1〜3 | 4〜5（1・2 は 1 ずつ・3 は pack と bake で 1〜2・テストつき） |
| 直したほうがよい | 4〜13 | 9〜11（4＋10 の部品の上げで 1〜2・5・6 は 1〜2・7・8・9・11・12 は 1 ずつ・13 は案 乙なら 0〜1） |
| できれば | 14〜22 | 4〜5（14＋15 で 1・16＋18 で 1・17 で 1・19 は 9 と同じタスク・20＋21 で 1・22 は裁定しだいで 0〜1） |
| 確かめ | — | 2（audit のかけ直し・全体テスト） |
| 計 | 22 件 | **19〜23** |

20 をまたぐ見込みなので、要件 8.1 のとおり切り方を出す。

- **切らない（推す）**: 13 を案 乙（説明だけ）、22 を C20 へ回す裁定なら、19〜21 で 1 本に収まる見込み。どの指摘も 1 つのクレートの中で、触るファイルが重なる（テストの部品・`lib.rs`）ので、2 本に切ると互いの rebase が増える。
- **切り方 甲**: 本 spec＝必ず直す＋直したほうがよい（1〜13・約 15〜18）。2 本目（同じ範囲・例 `audit-emo-atlas-2`）＝できれば（14〜22・約 5〜6）。2 本目は本 spec の着地を待つ（テストの部品と `lib.rs` を共有する）。
- **切り方 乙**: 本 spec＝コードの直し（1〜3・11〜13・15〜18・22）。2 本目＝テストの整理（4〜10・14・19〜21）。テストの整理だけで 1 本になるが、1〜3 の新しいテストが部品の上げより先に入るので、写しを一度増やしてから消すことになる。

## 9. 並走の兄弟（ウェーブ C19）に及ぶ所

- `runtime-module-untangle`（`crates/areka/src/emo2_boot/` ほか）: 1〜21 の直しは本体のファイルに触らず、本体が使う公開の形（`AtlasTable` の関数・`bake`・`bake_quiet`・`WicDecoderArm`・`MemoryDecoder`・`ElementDecoder`・`SurfaceSet`・`AlphaParams` ほか）を変えない。2 の新しい `Err` は今の `DecodeError::Decode` と同じ形。本体の `placement_tests/measure_bake_count_tests.rs` が数える読み手の呼び出し回数は、普通の大きさの絵では変わらない（18 の長さの確かめは読み手を呼ばない）。22 を C19 で直すなら申し合わせる（議題 3）。
- `audit-placement-core`（C20・`crates/areka-placement/`）: `FirstFrameDecoder`（`measure_first_frame.rs`）は `ElementDecoder` を包む。18 の確かめは正しい長さを返す読み手には効かない＝変更不要。
- `emo-text-reveal-incremental`（emo-text の新しいファイルの席）: このクレートは `crates/areka-emo-text/` に触らない（`areka-emo-text/tests/` の検体のテストはアトラスの公開の形だけを使う）。
- `dola-yaml-backend`（`Cargo.lock` の席）: このクレートは依存を足さない（17 で `areka-nar` を足す案は取らない）。
- そのほかの範囲の外: 8 の `with_frames` の確かめで `areka-emo-compose/src/film_tests.rs:231`（`audit-emo-compose` は完了済み）。13 の案 甲で `areka-parsers`・`areka-emo-compose`。6 の ⑷ の検体は `crates/wintf/tests/com/wic_test.rs` も読む（足すだけ）。

## 10. 調べが要る所（Research Needed）

1. WIC が、IHDR に巨大な寸法を書いた小さな PNG を `CreateDecoderFromFilename`・`GetFrame`・`GetSize` まで通すか（指摘 2 のテストの組み方）。通さないなら、上限の確かめは `image` の側（動く絵）と掛け算のあふれの確かめだけで足りる。
2. `image` 0.25 の `PngDecoder::new`・`WebPDecoder::new` が既定で確保の上限（`Limits`）をかけるか（指摘 2 の ⑶ の動く絵の 1 枚目）。
3. SSP が一辺 2,046 画素を超える静止画をそのまま描くか（指摘 3・議題 2）。測るなら SSP の MCP の `dump_surface` で 1 回（記憶「ukadoc で読めない点は議題の前に SSP の MCP で測る」）。
4. 指摘 12 の効き目: 絵の多い実在のシェル（`C:\wintools\ssp\ghost\` の着せ替えの多いもの）で、頁の数と pack の時間を前後で測る。
5. 範囲の外の気づき（直さない）: `image-webp` 0.2.5 以上が crates.io に出ていれば、根の `[patch.crates-io]` の固定と `webp_pin_tests.rs` を外せる（`tech.md` の外す条件）。出ているかはこの分析では確かめていない（ネットを引いていない）。

## 11. 議題（要件の討議へ）

1. **静止画の一辺の上限（指摘 2）**: 推す＝一辺 16,384（`areka-emo-compose` の外形の上限と同じ・GPU の機能水準 11 の一辺）。越えた絵は確保せず `DecodeError::Decode`（呼び手の `error!` に寸法が出る）。画素の合計の上限（静止画の全部の合計）はかけない（普通のシェルの絵の数を縛ることになる）。
2. **頁に入らない大きな絵（指摘 3）**: 推す＝その絵だけの頁を、入る大きさで 1 枚作って載せる（設計 D7 の「自頁」に戻す・一辺は議題 1 の上限まで）。理由＝今は黙って見えなくなり、ukadoc に大きさの決まりは無い。SSP の測り（10 節の 3）は推すが、待たずに決めてもよい。
3. **使い手の無い公開の型（指摘 22）**: 推す＝C20 へ回す（`runtime-module-untangle` の着地の後に、公開の形を下げる直しとして入れる）。理由＝コードの使い手は 0 だが、C19 の約束の字面に当たり、申し合わせの手間に見合う得が無い。
4. **8 語の表の置き場（指摘 13）**: 推す＝案 乙（この spec では写しのまま、説明に正本の場所と「変えたら両方直す」を書く）。案 甲（`areka-parsers` へまとめる）は 3 クレートに触る直しで、得は記録の段の食い違いを防ぐことだけ。ukadoc の「着せ替えの中では overlay として扱う」に合わせるかは、compose の扱いと揃えて別に決める（記録の段だけの違い）。
5. **道具のテストの走らせ方（指摘 5）**: 推す＝`Cargo.toml` に `[[example]] test = true` の 4 行（関数を動かさない・いちばん小さい）。
6. **切り方（要件 8）**: 推す＝切らない（議題 3 を C20・議題 4 を案 乙にすれば 19〜21）。21 を越えそうなら切り方 甲。

---
読まなかった所は無い（範囲の 63 ファイルを全部読んだ）。テストもビルドも走らせていない（「今のテストは緑」は読みからの前提）。
