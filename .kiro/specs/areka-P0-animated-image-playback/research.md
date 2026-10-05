# ギャップ分析: areka-P0-animated-image-playback

> 2026-10-05・本ブランチ（main `ec072853` の上）のコードを読んで書いた。ビルドとテストは回していない（読んだだけ）。
> コードは「何の定義か＋ファイル」で指す。設計・実装の着手時に引き直すこと。
> この文書は材料と選択肢を並べるもので、決定はしない。決めるのは要件討議と設計である。
>
> **2026-10-05 設計の段の追記**: この文書の 1〜8 章はギャップ分析（main `ec072853` の上）のままで、一部が古くなった。⑴ `import` は棚卸㉒で `areka-P0-animated-image-import` へ切り出した。`import` についての記述（1 章の 3 項目め・2.5 節の pattern定義の行・3 章の表の 5.x の行・3.2 節・4 章の各案の `import` の文・6 章の 6・8 章の最後の項）は**引き渡し済みの材料**であり、本 spec は使わない（消さずに残す）。⑵ 議題 1〜5 は要件討議で全部決着した（requirements.md）。⑶ 設計が選んだ形と、その根拠は末尾の「設計の段の調査」に在る。

## 1. まとめ

- **土台はほぼ揃っている**。全コマはアトラスに載り（`areka-emo-atlas` の `AtlasTable::animation`）、子のサーフェスの時計（`areka-seriko` の `PartClocks`）は「親を切り替えても巻き戻らない・見えない間も進む・シェルの差し替えで捨てる」を既に満たす。足りないのは「繰り返す」「表示されたらすぐ始める」「コマの絵を名指しする」の 3 つである。
- **brief の案（動く絵を子サーフェスへ分解する）をそのまま載せると、今のコードのままでは 4 か所でつまずく**（3 章）。①2 枚目以降のコマはファイル名で引けない ②外形の計算がアニメーションのコマを数えないので、動く絵だけのサーフェスは外形 0×0 になり非表示へ落ちる ③子の時計は刻み（16 ミリ秒ごと）でしか生まれないので、表示の最初の 1 回は絵が欠ける ④作者の番号と当たらないサーフェス番号の空間が今は無い。どれも設計で解けるが、解き方で触るファイルが変わる。
- **`import` は読み手（`areka-parsers`）から要る**。pattern定義の第 3 欄は数として読まれるので、ファイル名は 0 に化けて消える。brief の「触るファイル」に `areka-parsers` の pattern定義は入っていない。
- **バルーン（議題 1）は「読み込みは届くが、時計が届かない」**。子の時計はシェル面だけで回り、鍵に面の種類（シェル／バルーン）を持たない。動かすなら時計の鍵と、隠れている間の合成の止め方に手が入る。
- **議題 2〜4 は、今の仕組みとの相性で重さが大きく違う**（5 章）。議題 3 の仮の案（丸めずに飛ばす）と議題 4 の仮の案（駆動しない）は今の関数の振る舞いそのままで、足す仕事が無い。議題 2 の「守る」は時計の開始の時刻だけで求まり軽い。議題 2 の別案 ⑵（表示し直すたびに頭から）は、子の時計の決まりと正面から当たる。
- 規模は **M〜L**、リスクは **中**（外形・最初の 1 回・合成のし直しの回数の 3 点が設計の山）。

## 2. 今あるもの（要件ごとの手がかり）

### 2.1 読み込みとアトラス（前提・変えない）

| 何 | 定義 | 中身 |
| --- | --- | --- |
| コマの一覧 | `AtlasTable::animation`（`crates/areka-emo-atlas/src/table.rs`） | 動く絵の親の `ElementId` でだけ `Some(&Animation)`。本番の呼び手は 0 か所（要件の実測どおり） |
| コマの並び | `Animation`（同上） | `frames`（0 番は親自身）・`delays_ms`（同じ長さ）・`loop_count` |
| 繰り返し回数 | `LoopCount`（同上） | `Infinite` か `Finite(NonZeroU32)` |
| 鍵からの逆引き | `AtlasTable::resolve`・`AtlasTable::with_frames`（同上） | 2 枚目以降のコマは親と同じ鍵を持ち、`resolve` は親（いちばん小さい番号）だけを返す。**2 枚目以降のコマをファイル名で引く口は無い**（`ElementId` で直接引くしかない） |
| 外形のもとになる寸法 | `AtlasEntry.original`（同上） | 各コマは絵の全体の寸法で揃っている（brief の申し送り）ので、どのコマでも同じ値 |
| 試験用の読み手 | `MemoryDecoder::insert_animated`（`crates/areka-emo-atlas/src/decode.rs`） | 実ファイルも Windows の拡張機能も要らずに動く絵を焼ける（要件 9.3 に使える） |
| 検体 | `crates/areka-emo-atlas/src/testdata/animated/README.md` | 12 個・全部 8×8 以下の単色。回数つきは `basic.apng`（合計 2 回）・`alpha.webp`（合計 3 回・待ち時間 100・0・70＝**待ち時間 0 のコマを含む**）。終わりなしは `rgb.apng` ほか |

### 2.2 アニメーションの表と時計（`areka-seriko`）

| 何 | 定義 | 今の振る舞い |
| --- | --- | --- |
| 表に採る語 | `AnimationTable::from_world`（`crates/areka-seriko/src/table.rs`） | `random`・`bind+random`・`sometimes`・`rarely` だけ。`always` は `Interval::Other("always")` として届き、元の綴りつきの `debug!` で落ちる |
| 引き金の型 | `LoopTrigger`（同上） | `Random{k}`・`BindRandom{k}` の 2 つ。`k == 0` を弾く検査が `let (Random{k} \| BindRandom{k}) = trigger` の形で書かれており、**3 つ目の引き金を足すとここがコンパイルで止まる**（直す場所が分かりやすい） |
| 表の材料 | 同上 | **`EmoWorld` だけを受け取り、アトラスは受け取らない**。動く絵のコマの数・待ち時間は表を作る場所から見えない |
| コマの進み方 | `frame_at`・`FrameStatus`（`crates/areka-seriko/src/timeline.rs`） | 開始からの経過と待ち時間の累積で今のコマを決める純粋な関数。**繰り返しの考えが無い**（末尾で止まるか、`-1` で消える）。待ち時間 0 が続くと同じ時刻を共有し、後ろのコマが今のコマになる（＝通り過ぎたコマは出ない） |
| 一番上の再生 | `LoopRuntime::on_tick`・`Playback`（`crates/areka-seriko/src/looper.rs`） | 始まるのは 1000 ミリ秒の境目の抽選だけ。`on_surface_changed` がサーフェスの切り替えで再生を全部捨てる |
| 子の時計 | `PartClocks`・`PartAnim`（`crates/areka-seriko/src/parts.rs`） | 鍵は（スコープ, 部品の番号, animation の番号）。`Playing{started_at_ms}` は開始の時刻を持ち続け、見えない間も進んだ扱い。`clear` はシェルの表の差し替えでだけ呼ばれる |
| 子の時計が生まれる場所 | `PartClocks::advance`（同上） | 刻みの、しかも境目を跨いだ回の抽選でだけ生まれる。`PartClocks::peek`（切り替えの直後）は時計を作らない |
| 子の経路を通すか | `AnimationTable::has_animated_parts`（`table.rs`）・`LoopRuntime::on_tick` の `with_parts`（`looper.rs`） | 動く部品が無い表では子の経路を 1 行も通らない。**通すのはシェル面だけ**（`*slot == Slot::Shell`） |
| 刻み | `spawn_loop_ticker`（`crates/areka-ghost/src/ticker.rs`） | 16 ミリ秒 |
| 同じコマなら出さない | `ScopeStates::commit_pattern`（`crates/areka-seriko/src/state.rs`） | 新しい `PatternState` が前と同じなら指令を出さない。**要件 7.4（コマが替わらない更新では合成し直さない）はこの 1 か所が既に担っている** |

### 2.3 合成（`areka-emo-compose`）

| 何 | 定義 | 今の振る舞い |
| --- | --- | --- |
| 今のコマの入れ物 | `PatternState`・`PatternFrame`（`crates/areka-emo-compose/src/pattern.rs`） | 一番上の欄（animation の番号 → コマ）と部品の欄（部品の番号 → animation の番号 → コマ）。**コマが指せるのはサーフェスの番号だけ**（絵を直接は指せない） |
| 描画メソッドの名前 | `ComposeMethod::from_name`・`known_method`（`crates/areka-emo-compose/src/method.rs`） | `import` は表に無く、`warn!` を出して `Unknown` へ。`is_implemented` は `Overlay` だけ |
| 命令の組み立て | `flatten_surface`・`push_static_element_ops`（`crates/areka-emo-compose/src/plan.rs`） | element定義 → 着せ替えの pattern0 とコマ、の順に積む。数字だけの element定義は子へ再帰する。コマも pattern0 も `Overlay` だけ通す |
| 外形 | `compute_extent`・`flatten_extent`（同上） | 数えるのは「element定義の絵（子へは再帰）」と「**着せ替えの種類**の animation の pattern0」だけ。**着せ替えでない animation のコマは数えない**。コマ（`PatternState`）は見ないので、アニメーションで外形は動かない |
| 外形 0×0 | `build_plan`（同上） | 数えるものが無いサーフェスは `EmptyComposition`。表示の側（`crates/areka-emo-present/src/presenter/show.rs` の `apply_show`）はこれを非表示へ縮める |
| 絵の束縛 | `bind_atlas`（`crates/areka-emo-compose/src/atlas_bind.rs`） | element定義のファイル名を `AtlasTable::resolve` で 1 度だけ引く。ファイル名でしか引かない |
| 面の画像の土台 | `apply_base_images`（`crates/areka-emo-compose/src/base_image.rs`） | `element0` の無いサーフェスに `surface<数字>.png` を層 0 の画像の element として足す。`element0` が在れば使わない（＝要件 1.3 は今のままで満たされる） |
| 見える部品 | `NestTable::visible_parts`（`crates/areka-emo-compose/src/nesting.rs`） | element定義の子・有効な着せ替えの pattern0 の先・コマの先をたどる |

### 2.4 表示と起動の結線

| 何 | 定義 | 今の振る舞い |
| --- | --- | --- |
| 合成の結果を覚える席 | `ComposeCache`・`ComposeKey`・`CAPACITY`（`crates/areka-emo-present/src/cache.rs`） | 鍵は（サーフェス番号, 着せ替え, `PatternState` の全体）。席は 3・古い順に追い出す。定数の変更は裁定を通すと書いてある |
| シェルの読み込み | `build_shell_target_with_boxes`（`crates/areka-emo-present/src/shell_target.rs`） | 焼く一覧は「`surfaces.txt` の element定義の絵＋土台に使う面の画像」。面の画像は「`element0` だけを持つサーフェス」を一覧に足す形で焼いている（`base_image_surface`）。`ManifestDeriver::derive`（`crates/areka-emo-atlas/src/manifest.rs`）は element定義の絵だけを集める |
| バルーンの読み込み | `build_balloon_target_from_faces`・`synthetic_surfaces_txt`（`crates/areka-emo-present/src/balloon.rs`） | 面 1 つを「`element0,overlay,<実ファイル名>,0,0` だけのサーフェス」に書き直し、シェルと同じ読み手・同じ `bake`・`EmoWorld::build` へ通す |
| 表を作る場所 | `crates/areka/src/emo2_boot/assets.rs` のシェルの資産を作る関数（`loop_table`）・`build_balloon_assets`（スコープごとの `balloon_tables`） | どちらも `AnimationTable::from_world(&emo_world)`。アトラスは同じ場所に在る（渡してはいない） |
| バルーンの見える・見えない | `apply_show` の説明（`crates/areka-emo-present/src/presenter/show.rs`）・`crates/areka/src/emo2_boot/frame/attach.rs` の `VisibilityOwnership::External` | 外が持ち主の対象では、表示の指令は合成・配置までを行い、見えるようにはしない |

### 2.5 読み手（`areka-parsers`）

| 何 | 定義 | 今の振る舞い |
| --- | --- | --- |
| element定義 | `Element`（`crates/areka-parsers/src/shell/model.rs`）・`decode_elements`（`decode.rs`） | 描画メソッドの欄を持たない。第 2 欄が `overlay` の行だけ採り、他は記録なしに読み飛ばす（要件 1.1 の「`overlay` の element定義」はこのままで合う） |
| pattern定義 | `Pattern`（`model.rs`）・`decode_animations`・`field_i64`（`decode.rs`） | 第 3 欄を `i64` の `surface_id` に入れる。**数でない欄は 0 になる**。ファイル名を入れる欄は無い |
| interval | `normalize_interval`（`decode.rs`）・`Interval`（`model.rs`） | 完全一致で `bind`・`random`・`bind+random` だけを見分ける。他は `Interval::Other(元の綴り)`。`bind+always` は `Other("bind+always")` になる |

## 3. 要件と今のコードの差（Requirement-to-Asset Map）

印: **無**＝仕組みが無い／**縛**＝今の作りが縛りになる／**不明**＝設計で調べる／**済**＝今のままで満たす

| 要件 | 使えるもの | 差 |
| --- | --- | --- |
| 1.1・1.2 自動アニメーション | `AtlasTable::animation`・子の時計・`apply_base_images`（`surface*.png` を element にする） | **無**: 動く絵の element をコマの列へつなぐ場所。**縛**: 表（`AnimationTable::from_world`）はアトラスを知らない |
| 1.3 `element0` が在る | `apply_base_images` | **済** |
| 1.4 位置と重ね順 | `push_static_element_ops` | 置き換え方しだい（下の 4 章） |
| 1.5・1.6 待ち時間・累積 | `frame_at`・`PartAnim::Playing` | **無**: 繰り返し。累積で決める形は既に在る |
| 1.7 外形を変えない | `flatten_extent` | **縛**: 着せ替えでない animation のコマを数えない。動く絵を「コマだけのサーフェス」にすると外形から消える（絵がそれだけのサーフェスは 0×0 → 非表示） |
| 1.8 当たり判定 | `SurfaceMaster.collisions`（静的） | 矩形は変わらない。**不明**: 透明な画素でクリックを通す判定は合成結果のバイトから作る（`CacheEntry.mask`）ので、コマごとに形が変わる。要件の「当たり判定の領域」が矩形だけを指すのかを設計で確かめる |
| 1.9 子・pattern の先でも同じ | `PartClocks`・`NestTable::visible_parts` | 子へ分解する案なら自然に満たす |
| 1.10 作者のアニメーションと並ぶ | 一番上の欄と部品の欄は別 | **済**（欄が分かれている） |
| 1.11 GIF・縮んだ絵 | `animation` が `None` | **済** |
| 1.12 作者の番号を変えない | — | **縛**: サーフェス番号は全部 `u32`（`SurfaceMaster.id`・`PatternFrame.surface_id`・`ElementKind::Surface`）。作者が使わない空間は今は無い |
| 2.1 終わりなし | — | **無** |
| 2.2・2.3 回数つき（仮） | `LoopCount`・`PartAnim::Playing` | 開始の時刻と合計の時間から「N 回を過ぎたら最後のコマ」が求まる（状態を足さずに済む） |
| 2.4 待ち時間 0（仮） | `frame_at` | **済に近い**: 今の関数が既に「同じ時刻のコマは後ろが勝つ」 |
| 2.5・4.6 合計 0 | — | **無**: 繰り返しを「経過を合計で割った余り」で作ると 0 で割る。表を作るときの検査が要る（`k == 0` の検査と同じ場所・同じ形） |
| 2.6 乱数を使わない | `frame_at` は純粋 | **済** |
| 3.1 初めての表示で 1 枚目から | `PartClocks::advance` | **縛**: 時計は境目の抽選でしか生まれない。「見えたら始める」が無い |
| 3.2〜3.6 巻き戻さない・揃う・スコープ別・捨てる | `PartClocks` の鍵と `clear` | 子へ分解し、同じ画像ファイルを 1 つの子にまとめれば**済** |
| 3.7 見えない間は描き直さない | `advance` は見える部品だけ触る | **済** |
| 4.1 抽選を待たずに始める | — | **無**: 一番上（`LoopRuntime::on_tick`）と子（`PartClocks::advance`）の 2 か所に要る |
| 4.2 繰り返す | — | **無**（同じ 2 か所） |
| 4.3 切り替えで止めて頭から | `on_surface_changed` | **済**（一番上の再生は切り替えで捨てられる） |
| 4.4 子では巻き戻らない | `PartClocks` | **済** |
| 4.5 途中の `-1` | `FrameStatus::Stopped` | **縛**: 今は時計を消す。繰り返しでは「消してから頭へ戻る」に変える |
| 4.7 ほかの語と並ぶ・抽選を変えない | 乱数の消費の順が決まっている（`looper.rs` の冒頭の説明） | `always` が乱数を引かなければ、既存の決定論テストの期待値は動かない（確かめる点） |
| 4.8・4.9 ほかの語・組み合わせ | `AnimationTable::from_world` の `Other` の腕 | **済**（`always` を完全一致で拾えば、`bind+always` は今までどおり落ちる） |
| 5.1〜5.5 `import` | — | **無**: 読み手の欄（ファイル名）・描画メソッドの名前・コマの展開。**縛**: `PatternFrame` は絵を直接指せない |
| 5.6 ファイルが無い | `bake` の失敗の一覧（`ShellTarget.bake_errors`） | 焼けなかった絵は一覧に載り `warn!` が出る。pattern定義の側の記録は新しく要る |
| 5.8 `import` だけが名指しする絵 | `base_image_surface` の手口（`shell_target.rs`） | 同じ手口（その絵だけを持つサーフェスを焼く一覧に足す）で `ManifestDeriver` を変えずに済む見込み |
| 5.9 上限 | `bake` | **済**（同じ `bake` を通れば効く） |
| 6（仮） バルーン | 同じ読み手・同じ `bake`・スコープごとの表 | **縛**: 子の経路はシェル面だけ。`PartClocks` の鍵に面の種類が無い（同じスコープのシェルとバルーンで番号が重なると時計が混ざる）。5 章の議題 1 |
| 7.1・7.2・7.7 動かないシェル | `has_animated_parts` の門・`PatternState` が空なら前と同じ | 動く絵・`always`・`import` が 0 のときに新しい経路を 1 行も通らない門を保てば**済** |
| 7.4 | `commit_pattern` | **済** |
| 7.5・7.6 回数と席 | `CAPACITY = 3` | **不明**: コマが 4 枚以上の動く絵は、コマが替わるたびに席から外れて CPU で合成し直す。数字は測って決める |
| 8 記録 | 表を作るときに 1 回だけ記録する型（`k == 0`・コマが空） | 同じ場所に足せる。**注意**: シェルの表は起動で 1 回、差し替えのたびに 1 回作られる（「読み込み 1 回につき 1 回」に合う）。面の表（`ShellTarget::build_world`）はスコープの数だけ組まれるので、記録をそちらに置くとスコープの数だけ出る |
| 9 テスト | `MemoryDecoder::insert_animated`・時刻と乱数を外から渡す檻（`looper_tests.rs` ほか）・`log_capture_kit` | 型は揃っている |
| 10 記録と申し送り | 網羅台帳の 3 項目（`doc/ukadoc-coverage/ledger/assets.toml` の `element*`＝縮退・`import`＝absent・`always`＝語彙のみ）・`doc/COMPAT_ARCHITECTURE.md` の「8. 沈黙ルール対応表」と、その中の「動く絵（APNG・動く WebP）の読み込み」の節 | 書き足す場所は在る |

### 3.1 brief の案をそのまま載せたときにつまずく 4 か所

brief の Approach 1 は「コマを 1 枚ずつ持ち `always` で順に指すサーフェスを作り、動く絵の element を、その子サーフェスを指す element に置き換える」である。

1. **2 枚目以降のコマを element にできない**。`bind_atlas` はファイル名で引き、`AtlasTable::resolve` は親しか返さない。コマ 1 枚ごとのサーフェスを作るなら、`ElementId` を直接束縛する口が要る。
2. **外形から消える**。子サーフェスが「element定義を持たず、`always` の animation のコマだけ」だと、`flatten_extent` は何も数えない。親に他の絵が在れば外形が縮むだけ（要件 1.7 に反する）、動く `surface0.png` だけのサーフェスなら 0×0 で非表示になる。逆に 1 枚目のコマを element として残すと、透明な部分を持つ絵で 1 枚目が後ろに透けて見える（コマは重ね済みの全体の絵なので、下に何も敷いてはいけない）。
3. **最初の表示で絵が欠ける**。サーフェスを表示する指令（`ScopeStates::apply`）は空のコマで出る。子の時計は次の刻みまで生まれず、`refresh_parts`（`PartClocks::peek`）は時計を作らない。静止画なら出ていた絵が、最長 16 ミリ秒のあいだ出ない。
4. **番号の空間**。サーフェス番号は `u32` の全域を作者が使える。合成したサーフェスを `u32` の上の方へ置くなら「作者が使っていない」ことを読み込みで確かめる必要があり、`\s[番号]` で直接呼べてしまう。面の一覧をなめる処理（`EmoWorld::dangling_pattern_targets`・`NestReport`・箱の表の畳み込み `fold_boxes`）にも現れる。上限いっぱい（1,024 コマ）の絵 1 つで 1,025 個のサーフェスが増え、面の表はスコープの数だけ組まれる。

### 3.2 `import` の今の読まれ方（直さないと残る副作用）

> 【引き渡し済み】この節は `areka-P0-animated-image-import` の材料である（2026-10-05 棚卸㉒）。本 spec は `import` に触らない。

`animation0.pattern0,import,glow.png,100,10,20` は、今は `method = "import"`・`surface_id = 0`（ファイル名が数でないため）・`wait = 100` と読まれる。その結果:

- `EmoWorld::dangling_pattern_targets`（`crates/areka-emo-compose/src/world.rs`）は「サーフェス 0 を指すコマ」と数える。サーフェス 0 が無いシェルでは「相手の面が無い」の `warn!` が出る。
- `ManifestDeriver::resolve_indirect` と `NestReport` もサーフェス 0 への参照としてたどる。
- 欄 2 がサーフェス番号でない描画メソッドを除く判定（`targets_animation_id`・`world.rs`）に `import` は入っていない。

`import` を入れるときは、読み手にファイル名の欄を足したうえで、この 3 か所から `import` の行を外す必要がある。

## 4. 実装の進め方の候補

### 案 A: 子サーフェスへ分解する（brief・開発者確定の方向）

- 読み込みの後、動く絵 1 つにつき「コマ 1 枚ずつのサーフェス」と「それを `always` で順に指す子サーフェス」を合成し、動く絵を指す element を子サーフェスを指す element（`ElementKind::Surface`）へ置き換える。
- `always` は `LoopTrigger` に足し、`frame_at` に繰り返しの版を足す。`import` は表を作るときに「冒頭の待ち＋コマの列」へ展開する。
- **触る場所**: `areka-seriko` の `table.rs`・`timeline.rs`・`looper.rs`・`parts.rs`／`areka-emo-compose` の `plan.rs`（外形）・`atlas_bind.rs`（`ElementId` の直接の束縛）・`world.rs` か `fold.rs`（合成サーフェスを足す場所）・`method.rs`／`areka-emo-present` の `shell_target.rs`／`areka-parsers` の `model.rs`・`decode.rs`（`import`）／`crates/areka/src/emo2_boot/assets.rs`。
- ✅ 開発者の決めた形そのもの。時計は 1 種類（animation の時計）のまま。要件 3（巻き戻さない・揃う・スコープ別・捨てる）と 1.9（子でも同じ）が子の時計の決まりからそのまま出る。
- ❌ 3.1 の 4 か所を全部解く必要がある（`ElementId` の束縛・外形・最初の 1 回・番号の空間）。サーフェスの数が最大で絵 1 つあたり 1,025 個増える。`plan.rs` は 821 行で、1 ファイル 1,000 行の上限に近い。

### 案 B: 「絵のコマ番号」の欄を足す（サーフェスは合成しない）

- `PatternState` に 3 つ目の欄（動く絵 → 今のコマ番号）を足し、`push_static_element_ops` が element の `ElementId` を `AtlasTable::animation(id).frames[番号]` へ差し替える。時計は `areka-seriko` の中に「動く絵の時計」（鍵＝スコープ × 画像）として持つ。
- ✅ サーフェスも番号の空間も合成しない。欄が空なら 0 番（親）＝静止画と同じ絵なので、最初の 1 回が欠けない。外形は親の寸法のまま（`flatten_extent` を変えない）。バルーンにも同じ欄がそのまま届く。
- ❌ brief の確定事項（子サーフェスへ分解・3 つ目の時計は作らない）から離れる。時計は seriko の中に在るが、animation の時計とは別の種類が 1 つ増える。`import` は「pattern定義から絵を置く」ので、この欄だけでは描けず、結局コマが絵を指す口が別に要る。合成の鍵（`ComposeKey`）が欄 1 つぶん大きくなる。

### 案 C: A の骨格のまま、コマが絵を直接指せるようにする（折衷）

- 動く絵 1 つにつき合成するサーフェスは **1 個だけ**（子サーフェス）。その `always` の animation のコマは、サーフェス番号でなく絵（`ElementId`）を直接指す（`LoopFrame`・`PatternFrame` の指す先を「サーフェス番号か、絵か」の 2 通りにする）。`import` のコマも同じ口で絵を指す。
- 外形は、子サーフェスに「この絵の寸法」を持たせて `flatten_extent` に数えさせる。最初の 1 回は、`always` の時計を「見えた時点で始める」規則にして `peek` でもコマを出す（または時計が無い間は 0 番を出す）ことで埋める。
- ✅ 開発者の形（子サーフェス・時計は 1 種類）を保ちつつ、3.1 の ①（ファイル名で引けない）とサーフェスの数の膨らみが消える。`import` と自動アニメーションが同じ口を使う。
- ❌ `PatternFrame` の形が変わる（`NestTable::visible_parts`・`flatten_surface`・`pattern_frame` など、コマの `surface_id` を読む場所を全部見直す）。番号の空間（3.1 の ④）は絵 1 つにつき 1 個ぶん残る。外形と最初の 1 回は A と同じく設計で解く。

### 3 案に共通して要るもの

- `always` の引き金と繰り返し（`table.rs`・`timeline.rs`・`looper.rs`・`parts.rs`）。合計 0 の検査。
- 表を作る場所へアトラスを渡す（`AnimationTable::from_world` の呼び手は起動・差し替え・テストに在る）。または、動く絵の事実を面の表（`EmoWorld`）の側に先に写しておく。
- `import` の読み手の欄と、3.2 の 3 か所の手当て、`import` だけが名指しする絵を焼く一覧へ足すこと。
- 動かないシェルで新しい経路を通らない門（`has_animated_parts` と同じ考え方）と、それを数えるテスト。

## 5. 議題ごとの材料

### 議題 1: バルーンの面の動く絵を動かすか（要件 6）

- **文書の食い違いの実物**:
  - brief（本 spec・2026-10-01）の Desired Outcome と Scope の In に「バルーンの面に置いた動く絵も動く」。
  - 完了 `areka-P0-animated-image-decode` の requirements.md（`.kiro/specs/completed/areka-P0-animated-image-decode/requirements.md`）の Out of scope に「バルーンの絵を動かすこと。…コマを時間で切り替える仕組みは本 spec でも後続でも作らない。画面に出るのは 0 番のコマ 1 枚である」。同じ文書の要件 5.4 は「バルーンの絵に動く APNG・WebP が置かれているときは、シェルの絵と同じに読んで全コマを焼き、今までの鍵で 0 番のコマを出す（議題 4 の裁定『バルーンも同じ』・2026-10-04）」。冒頭には「討議で足した 2 件＝バルーンの動く絵〔要件 5.4〕」とあり、**2026-10-04 の討議でバルーンを話題にしたうえでの文面**である（brief より後）。
- **正典**: C1（`element*` の項）が名指しするのは `surface*.png` と element定義で、どちらもシェルの `surfaces.txt` の項。ukadoc MCP を「APNG」で引いた 4 件（`element*`・`import`・シェルのファイル構成・SSP ヘルプの画像ビューワ）に、バルーンの絵のアニメーションを定める記述は無い。SSP ヘルプの画像ビューワの表は APNG・WebP を「SSP 全域（シェルやバルーンの素材などにも使える）」と書くが、これは読める形式の話で、バルーンで動くとは書いていない。
- **コードの現状**:
  - 読み込み: バルーンの面は `element0,overlay,<実ファイル名>` のサーフェスとしてシェルと同じ `bake` を通る（`build_balloon_target_from_faces`）。全コマは既にアトラスに在る。
  - 表: スコープごとに `AnimationTable::from_world` で作られる（`build_balloon_assets`）。面の表は `EmoWorld::build`（面の画像の土台の手順を通らない版）で組まれる。
  - 時計: 子の経路はシェル面だけ（`LoopRuntime::on_tick` の `with_parts`・`LoopRuntime::refresh_parts`）。`PartClocks` の鍵に面の種類が無い。バルーンの面の切り替え（`ScopeStates::apply_balloon`）は部品の欄を載せ直さない。
  - 見える・見えない: バルーンは外が持ち主の対象なので、表示の指令で勝手に見えるようにはならない（要件 6.3 は今の作りで守られる見込み）。一方、seriko はバルーンの窓が隠れているかを知らない（知っているのは `\b[番号]` で決まる面の状態だけ）。**隠れている間もコマが替われば合成までは走る**（要件 6.4 を満たすには止める仕組みが要る）。
- **裁定で変わる仕事**:
  - 「動かす」: 子の時計をバルーン面でも回す（鍵に面の種類を足す）・バルーンの面の切り替えで部品の欄を載せ直す・隠れている間の合成を止める・バルーンの検体と実機の確かめ。案 B なら時計の鍵と隠れている間の止め方だけで済む。
  - 「動かさない」: バルーンの側は何も足さない。案 A・C では「動く絵を子サーフェスへ置き換える手順」をバルーンの面の表に掛けないだけで変更 0 になる。完了 spec の文面とも合う。
- **見ておく点**: `emo2` を含むリポジトリ内の検体のバルーンに動く絵は 0 枚（brief の申し送り「今のリポジトリの検体には動く絵が 0 枚」）。どちらに裁定しても今ある検体の見た目は変わらない。

### 議題 2: 自動アニメーションでファイルの繰り返し回数を守るか（要件 2.2・2.3）

- **正典**: `import` の項だけが「繰り返し回数は無視される」と書く（C4）。`element*` の項（C1）は回数に触れない。ukadoc MCP を「繰り返し回数」で引いて当たるのは `import` の項と `\![set,tasktrayicon,…,--runcount=繰り返し回数]`（通知領域のアイコン・「指定がない場合は無限に繰り返す」）と YAYA の 1 件だけで、自動アニメーションの回数を定める記述は無い。
- **コードの現状**: 読み込みは `LoopCount::Infinite`／`Finite(n)` を渡すだけ。検体に回数つきが 2 種在る（`basic.apng`＝合計 2 回、`alpha.webp`＝合計 3 回）ので、どの案でもテストの材料は在る。
- **案ごとの重さ**:
  - 仮の案（守る・止まったら動かし直さない）: 子の時計の `Playing{started_at_ms}` から「経過 ≥ 合計の時間 × N なら最後のコマ」と求まる。状態を足さない。時計はシェルの差し替えまで捨てられないので、要件 2.3（サーフェスを切り替えても動かし直さない）は子の時計の決まりからそのまま出る。**作者から見ると「起動して数秒で止まり、表情を替えても二度と動かない」絵になる**点は、裁定のときに伝える材料。
  - 別案 ⑴（無視して常に繰り返す）: 仕組みが `always` 1 つで済み、回数の分岐とそのテストが要らない。`import` の扱い（正典が無視と明記）と揃う。
  - 別案 ⑵（表示し直すたびに頭から）: 子の時計は「見えなくなった・また見えた」を覚えない作り（見えない部品には触らない）。この案は時計に「見えていたか」を持たせる必要があり、要件 3（巻き戻さない）とも完了 `areka-P0-surface-element-nesting` の時計の決まりとも当たる。
- **一番上のサーフェス自身の `always`**（要件 4.3）は切り替えで頭から始まる。自動アニメーションを「子の `always`」として作ると子の決まり（巻き戻らない）になり、同じ `always` でも置き場所で振る舞いが違う。これは要件どおりだが、対応表（要件 10.2・10.3）に並べて書く材料になる。

### 議題 3: 待ち時間 0・極端に短い待ち時間（要件 2.4）

- **正典**: 黙っている（ukadoc MCP で `element*`・`import` の項に待ち時間の下限の記述は無い）。
- **コードの現状**:
  - 読み込みは 0 を 0 のまま渡す（brief の申し送り）。検体 `alpha.webp` は待ち時間 100・0・70 で、0 のコマを持つ。
  - `frame_at` は今でも「待ち時間 0 が続くと同じ時刻を共有し、後ろのコマが今のコマ」になる（`timeline.rs` のテスト `frame_at_all_zero_waits_selects_last_index` が固定している）。**仮の案（丸めない・通り過ぎたコマは飛ばす）は、今の関数の振る舞いそのまま**である。
  - 刻みは 16 ミリ秒なので、16 ミリ秒より短いコマは丸めなくても画面には出ない（飛ばされる）。
  - 合計が 0 の絵は、繰り返しを余りで求めると 0 で割る。要件 2.5 の検査が要る（どの案でも）。
- **別案（短い待ち時間を決めた値へ引き上げる）の重さ**: 表を作るときに 1 か所で値を書き換えるだけで軽い。ただし (a) ファイルどおりの速さでなくなる（要件 2 の Objective と当たる）(b) 引き上げる値と閾値に正典の根拠が無い (c)「絵を見る道具の多くがそうしている」は本分析では裏を取っていない（**要調査**・設計で一次の資料を当たる）。
- **性能の面**: 待ち時間が短くコマが 4 枚以上の絵は、刻みのたびに席から外れて合成し直す（`CAPACITY = 3`）。引き上げは合成の回数を減らす効果も持つが、それは要件 7.5 の測定で数字を見てから決められる。

### 議題 4: `bind+always` などの組み合わせ（要件 4.9）

- **正典**: C6「SSPのみ+区切りで列挙する事で組み合わせ指定が可能」。組み合わせの相手や順を限る記述は無い（`always+bind` の順も書ける）。ukadoc MCP は 1 語の部分一致だけなので「bind+」では 0 件、材料は C6 だけである。
- **コードの現状**:
  - 読み手（`normalize_interval`）は `bind`・`random`・`bind+random` を**完全一致**で見分ける。`bind+always` は `Interval::Other("bind+always")` になる。
  - 表（`AnimationTable::from_world`）は `Other` の中身を完全一致で見る（`sometimes`・`rarely`）。`always` も完全一致で足せば、`bind+always` は今までどおり元の綴りつきの `debug!` で落ちる。**仮の案（駆動しない・変更 0）は足す仕事が無い**。
  - 「着せ替えの種類か」の判定は 3 か所に在る: 合成の `is_bind_interval`（`plan.rs`・pattern0 の静的な絵と外形）・`NestTable` の `bind_ids`（`nesting.rs`）・seriko の着せ替えの番人（`parts.rs` の `gate`・`looper.rs` の抽選と進行）。どれも `Interval::Bind`／`BindRandom` と `LoopTrigger::BindRandom` を型で見ている。
- **別案（`bind+always` だけ入れる）の重さ**: 引き金を 1 つ足し（着せ替えが有効な間だけ繰り返す）、上の 3 か所に「これも着せ替えの種類」と教える。`Other("bind+always")` を文字列で見るか、読み手の `Interval` に型を足すか（`#[non_exhaustive]` なので足せる）を決める。`+` の順の違い（`always+bind`）と、3 つ以上の組み合わせをどこまで読むかも決めることになる。この範囲は後続 `areka-P0-seriko-trigger-intervals`（残りの語と組み合わせを引き受ける）と重なる。

## 6. 設計で調べること（要調査）

1. **外形の数え方**（要件 1.4・1.7）: 動く絵を置き換えた後も外形が静止画のときと同じになる形。roadmap に `extent-element-offset`（外形が画像の element定義の X,Y を数えないバグ・「本 spec の前が望ましい」）が未着手で載っている。本 spec は「静止画のときと同じ」を守ればよく、そのバグは直さないが、同じ関数（`flatten_extent`）を触るので順序を確かめる。
2. **最初の 1 回の絵**（要件 3.1・7.1）: 表示の指令が出てから最初の刻みまでの間に、動く絵が欠けないこと。`always` の時計を「見えた時点」で始める規則を、一番上（`LoopRuntime`）と子（`PartClocks::advance`・`peek`）の両方でどう書くか。
3. **合成サーフェスの番号の空間**（要件 1.12）: `u32` の中で作者と分ける方法、`\s[番号]` で呼ばれたときの扱い、面の一覧をなめる処理（相手の無いコマ・入れ子の報告・箱の表）から外すかどうか。
4. **合成し直す回数と席の数**（要件 7.3〜7.6）: コマが 4 枚以上の絵で、コマが替わるたびに合成し直す時間が 16 ミリ秒に収まるか。前後の数字の採り方は `areka-P0-recompose-budget` の測り方（`cache.rs` の冒頭が引く `remeasure-2026-08-15.md`）に合わせる。
5. **クリックを通す判定の形**（要件 1.8）: 合成結果の透明度から作る判定がコマごとに変わることを、要件の「当たり判定の領域」に含めるか。
6. **`import` の細部**: 末尾のコマの後（抽選の語のとき、最後のコマを残すか消すか）・1 つの animation に `import` とほかの pattern定義が混ざるとき（要件 5.10）・`surface.append` の行にだけ現れる `import` の絵（roadmap の覚え書きに「`surface.append` の行にしか現れない絵のファイル名は焼かれない」が在る）。
7. **`always` が乱数を引かないこと**（要件 4.7）: 既存の決定論テスト（まばたき）の乱数の消費の順が動かないことを確かめる。
8. **起動の時間**: 動く絵を持つシェルは採寸と資産の組み立てで 2 回焼かれる（brief の表）。roadmap の `placement-measure-bake-once`（未着手・「本 spec の前が望ましい」）との順序。本 spec の検体は 8×8 なので、検体では数字に出ない。
9. **短い待ち時間を引き上げる道具の実例**（議題 3 の別案を採る場合だけ）。

## 7. 規模とリスク

- **規模: M〜L**（brief の見立て 15〜20 タスクのまま）。`always` と繰り返しは既存の型の延長で小さい。自動アニメーションの置き換え・外形・最初の 1 回、`import` の読み手からの追加、性能の測定がそれぞれ中くらい。議題 1 が「動かす」なら上限側。
- **リスク: 中**。新しい技術は無く、土台（全コマ・子の時計）は完了済み。山は ①外形と最初の 1 回（静止画のときと同じ絵・同じ寸法を保つ）②合成のし直しの回数（測るまで分からない）③`emo2` の既存の照合を書き換えずに通すこと（門の置き方）の 3 つ。
- **並走の注意**: 同じ合成の場所を触る `balloon-element-order`・`self-alpha-declaration`・`element-base-method`・`element-clipping-option`・`extent-element-offset` とは同時に走らせない（roadmap の列の決まり）。`crates/areka/src/emo2_boot/spine.rs` は 1,000 行ちょうどで、表を作る呼び出しを持つ。ここの署名を変えるなら行数に注意する。

## 8. 設計へ渡すこと

- 進め方は、開発者確定の「子サーフェスへ分解」を骨格にした **案 A か案 C** が brief と合う。案 C は 3.1 の ① とサーフェスの数を消せるが、コマの形（`PatternFrame`）を変える。案 B は brief の確定事項から離れるので、採るなら開発者の裁定が要る。
- 要件討議で先に決まると設計が軽くなる順: **議題 1**（バルーン＝時計の鍵と範囲が変わる）→ **議題 2**（回数＝仕組みが 1 つで済むか）→ 議題 4 → 議題 3（後ろの 2 つは仮の案なら足す仕事が無い）。
- 要件には無いが設計で必ず触る場所: `areka-parsers` の pattern定義（`import` のファイル名の欄）と、3.2 の 3 か所。brief の「触るファイル」に足す。


---

# 設計の段の調査（2026-10-05・main `82607b5f` を取り込んだ後）

> コードを読んで確かめた（ビルドとテストは回していない）。コードは「何の定義か＋ファイル」で指す。

## Summary

- **Feature**: `areka-P0-animated-image-playback`
- **Discovery Scope**: Extension（今ある合成と seriko への足し込み。外部の依存 0・軽い調査）
- **Key Findings**:
  - brief の案（動く絵をコマ 1 枚ずつのサーフェスと子サーフェスへ分解する）は、同じウェーブの約束（`plan.rs` に触らない）の下では要件を満たせない。コマが届くまで絵が丸ごと消える場面（バルーンの装着・シェルの差し替えの最初の表示）が残り、外形も数えられない。
  - 合成の入口 `Composer::compose_into`（`crates/areka-emo-compose/src/lib.rs`）は約束の外に在り、`build_plan` を呼ぶのはここ 1 か所である。命令の絵の番号をここで今のコマへ替えれば、位置・重ね順・外形は静止画のときのまま、コマだけが替わる。
  - シェルの表示は必ず seriko から出る（装着は出さない）ので、seriko は見えているシェルの面をいつも知っている。バルーンは逆で、装着が面 0 を確立し、見える・見えないは可視性の相だけが決める。seriko へ窓の見える・見えないを知らせる線が 1 本要る。

## Research Log

### 子サーフェスへ分解する形が約束の下で成り立つか

- **Context**: brief の Approach 1 と、同じウェーブ C4 の約束（`plan.rs`・`fold.rs`・`method.rs`・`areka-emo-present`・`assets.rs` に触らない）の両方を満たす形を探した。
- **Sources Consulted**: `flatten_surface`・`push_static_element_ops`・`flatten_extent`・`build_plan`（`crates/areka-emo-compose/src/plan.rs`）／`NestTable::visible_parts`（`nesting.rs`）／装着の手順（`crates/areka/src/emo2_boot/frame/attach.rs`）／`RebasedShow`（`crates/areka-seriko/src/output.rs`）／`ScopeStates::apply`・`apply_balloon`（`state.rs`）。
- **Findings**:
  - 合成は、コマ（`PatternState`）が空のとき、着せ替えでない animation の絵を 1 枚も描かない。静的に描かれるのは element定義の絵と、有効な着せ替えの pattern0 だけ。
  - コマが静的な絵を「置き換える」のは、有効な着せ替え（番号が着せ替えの集合に在る）の pattern0 に対してだけ。着せ替えの集合は seriko と装着が持つ値で、合成の側から足せない。しかも着せ替えの層は element定義の後に積まれるので、element定義の番号の順（要件 1.4）を保てない。
  - element定義の絵は必ず描かれ、コマはその上に重なるだけ。1 枚目を element として残すと、コマ（重ね済みの全体の絵）の透明な所から 1 枚目が見える。
  - 外形は「束縛の在る画像の element の原寸」「element定義の子」「全部の着せ替えの pattern0 の先」だけを数える。着せ替えでない animation のコマだけのサーフェスは 0×0。
  - 空のコマで合成される場面が実在する: バルーンの装着（面 0・空のコマ・見える・見えないは外が持ち主）／シェルとバルーンの差し替えの最初の表示（`RebasedShow` はコマを運ばない）／起動の採寸。
  - 装着はシェルの最初の表示を出さない（注記「シェルは初回 ShowSurface を attach で発行しない」）。seriko は `\b[番号]` が来るまでバルーンの面を知らない。
- **Implications**: 「コマが届かなければ絵が無い」という状態の持ち方そのものが、1 フレーム遅らせる解を取らない決まりと当たる。「何も届かなければ 1 枚目・届けばそのコマ」という持ち方に変える必要がある。`plan.rs` の今の決まりの中にはその持ち方を表す手段が無い。

### 試して捨てた抜け道

- **使われない着せ替えの animation を子サーフェスに足して外形に数えさせる**: 外形は取れるが、作者の `bindgroup` の番号と当たると 1 枚目が重なって見える。外形の計算の癖に寄りかかるので、後続 `areka-P0-extent-element-offset` が外形を直すと壊れうる。コマが届くまで絵が無い問題は解けない。
- **置き換えの前の外形を覚えておき、`Composer` でその外形を使う**: 外形は解けるが、コマが届くまで絵が無い問題は解けない。
- **アトラスに「原寸だけ持つ空の絵」を足す**: 外形は解けるが、読み込みとアトラスは本 spec の範囲外（要件の Boundary Context）。コマが届くまで絵が無い問題は解けない。
- **seriko にシェルの最初の面も知らせる**: シェルは seriko が出すまで表示されないので要らなかった。

### バルーンの窓の見える・見えないを seriko が知る方法

- **Context**: 要件 6.1（見えている間は動く）・6.4（隠れている間は描き直さない）・2.3（隠れていたバルーンが出たら回数つきは始め直す）。
- **Sources Consulted**: `run_balloon_visibility_phase`・`issue_actions`（`crates/areka/src/emo2_boot/balloon_visibility_phase.rs`）／`run_status_report_phase`（`crates/areka/src/emo2_boot/frame/status_report.rs`）／`EmoPresenter::target_visible`・`current_surface_id`（`crates/areka-emo-present/src/presenter/read.rs`）／`GhostSession::seriko_sink`（`crates/areka/src/ghost_session.rs`）。
- **Findings**: 見えているかの真実は表示層の照会。フレームの終わりに照会から組を作り、前と違うときだけ運行の側へ送る相が既に在る。seriko の送り手は `GhostSession` だけが持ち、差し替えの相が同じ口を借りている。
- **Implications**: 同じ形の相を 1 つ足し、seriko へ（開いているか・面の番号）を送る。可視性の判断には触らない。

### 既存の決まりを壊さないか

- **Findings**:
  - バルーンの面の表は今は必ず空（`crates/areka-emo-present/src/balloon.rs` の `synthetic_surfaces_txt` が `element0` だけのサーフェスを作る）。バルーンの面の進行を広げても、今ある振る舞いは変わらない。
  - 乱数の消費の順は、抽選の輪の対象の並びで決まる。`Always` を乱数を引く前に飛ばし、抽選の対象（`shown_slots`）を変えなければ、既存の決定論テストの期待値は動かない。
  - `ComposeKey`（`crates/areka-emo-present/src/cache.rs`）は `PatternState` を丸ごと持ち等しさで比べるので、欄を足しても鍵は正しく分かれる。perf の記録の `key_hash`（`crates/areka-emo-present/src/presenter/timing.rs` の `compose_key_hash`）は一番上の欄しか混ぜておらず、足す欄も混ざらない（今も部品の欄は混ざっていない）。
  - `AtlasBinding` を読むのは `plan.rs` だけ。束縛を別のコマへ付け替えても、ほかの読み手は居ない。
  - `crates/areka/src/emo2_boot/spine.rs` は 1,000 行ちょうど。テストの接続宣言は `mod.rs`（883 行）に置く。
  - `areka-seriko` は `areka-emo-atlas` に依存していない。動く絵の事実を数と文字列だけの型で渡せば、`Cargo.toml` を触らずに済む。

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
| --- | --- | --- | --- | --- |
| 子サーフェスへ分解（brief の案） | コマ 1 枚ずつのサーフェス＋ `always` で指す子サーフェス | 手で書いた定義と同じ形 | 約束の下では不成立（空のコマで絵が消える・外形・番号の空間・`\s[番号]` で呼べる） | `plan.rs` を直せば成り立つが、約束を解く必要がある |
| 絵のコマの欄＋合成の入口で差し替え（採用） | `PatternState` に「動く絵 → コマの番号」を足し、`Composer::compose_into` が命令の絵を替える | 空の欄＝ 1 枚目なので絵が欠けない・外形と重ね順は `plan.rs` のまま・番号を作らない・約束の 7 か所に触らない | brief の言い方（子サーフェス）から離れる。`import` は別に命令を出す所が要る | 時計は部品の時計と同じ型・同じ持ち主 |
| コマが絵を直接指す（ギャップ分析の案 C） | `PatternFrame` の指す先を「サーフェスか絵か」に広げる | `import` と口を共有できる | `plan.rs` の変更が必須（約束に反する） | 不採用 |

## Design Decisions

### Decision: 動く絵は「絵のコマの欄」で運び、合成の入口で差し替える

- **Context**: 要件 1.1〜1.13・7.1 と、同じウェーブの約束。
- **Alternatives Considered**:
  1. 子サーフェスへ分解 — 上の表のとおり不成立。
  2. 約束を解いて `plan.rs` を直す — 開発者の判断が要り、同じウェーブの `areka-P0-element-base-method` と場所が重なる。
- **Selected Approach**: `bind_atlas` の直後に動く絵の事実（`FilmFacts`）を面の表へ載せ、seriko の時計が `PatternState` の絵のコマの欄を進め、`Composer::compose_into` が `build_plan` の後に命令の絵の番号を今のコマへ替える。
- **Rationale**: 「何も届かなければ 1 枚目」が状態の持ち方から出るので、絵が欠ける場面が 0 になる。外形・位置・重ね順は静止画と同じ計算のまま。サーフェスの番号を作らないので、要件 1.12 が形から満たされる。
- **Trade-offs**: brief の「子サーフェス」の言い方から離れる（設計討議の議題）。ギャップ分析（上の 8 章）は、これに近い案 B を「採るなら開発者の裁定が要る」と書いている。perf の記録の `key_hash` は欄を数えない。
- **Follow-up**: 設計討議で開発者の確認を取る。roadmap の本 spec の行の言い方を直す。

### Decision: 繰り返しは「経過を周期で割る」1 つの計算で決める

- **Context**: 要件 1.6・2.1・2.2・2.4・2.9・4.2（時刻は正確に扱う・開発者裁定 2026-10-05）。
- **Selected Approach**: `lap_of`（何周目か・周の頭からの経過）を共有し、`always_at`（pattern定義の待ち＝出す前の待ち）と `film_frame_at`（画像の待ち＝出しておく時間）がその上に載る。状態は開始の時刻だけ。
- **Rationale**: 今のコマが「刻みの時刻 − 開始の時刻」だけで決まるので、刻みの遅れを持ち越す状態が無い。回数つきの「止まった」も経過から求まり、状態を足さない。
- **Trade-offs**: 待ち時間の数え方の違いは 2 つの関数に分かれる（共有するのは周の計算だけ）。

### Decision: 時計は「見えた最初の刻み」で生まれ、無い間は経過 0 として見る

- **Context**: 要件 3.1・4.1 と、1 フレーム遅らせる解を取らない決まり。seriko は刻みでしか時刻を知らない。
- **Alternatives Considered**:
  1. 表示の指令のときに、直前の刻みの時刻で時計を作る — 刻みが 1 度も来ていないと時刻が無い。始まりが最大 1 刻み早まる。
  2. seriko に時計を持たせる — アクターの作り方が変わり、決定論テストの注入の口が増える。
- **Selected Approach**: 表示の指令には経過 0 の絵（動く絵は 1 枚目・`always` は待ち 0 のコマ）を載せ、時計は次の刻みの時刻で作る。
- **Rationale**: 絵は切り替えの指令と同時に出る（0 フレーム）。規則が 1 つで、時刻が無い場合の分岐が要らない。
- **Trade-offs**: 1 枚目が最大 1 刻みぶん長く見える（1 度きり・積み上がらない）。設計討議の議題 2。

### Decision: 回数つきの絵の「始め直し」は、見えなくなった時計を捨てることで作る

- **Context**: 要件 2.3・2.7・2.8。
- **Selected Approach**: 回数つきの絵の時計は、評価（刻み・切り替えの直後・窓の知らせ）で「見えていない」と分かったら捨てる。終わりなしの絵と `always` の時計は捨てない。
- **Rationale**: 「見えていたか」を別に覚えずに済む（時計が在る＝途切れずに見えている）。始め直すかどうかが、繰り返し回数だけで決まる（要件 2.8）。

### Decision: 記録は seriko の表が出す

- **Context**: 要件 8.4（読み込み 1 回につき 1 回）。面の表はスコープの数だけ組まれる。
- **Selected Approach**: 合成の側は事実（`FilmFacts::skipped`）を返すだけ。`AnimationTable::from_world` が `warn!` を出す。
- **Rationale**: シェルの表は読み込み 1 回につき 1 回だけ作られる（`crates/areka/src/emo2_boot/assets.rs` の今の呼び方）。

### 設計の 3 つの見直し（まとめる・作るか使うか・削る）

- **まとめる**: 動く絵・一番上の `always`・部品の `always` は「見えたら乱数なしで始まり、経過を周期で割る」同じ問題。計算と「時計なし＝経過 0」の規則を 1 つにした。シェルとバルーンは面の種類を鍵に持つだけで同じ経路。
- **作るか使うか**: 外部のライブラリで解く部分は無い。今ある `frame_at`・`PartAnim`・`commit_pattern`・`NestTable::visible_parts`・届けの相の形をそのまま使う。
- **削る**: サーフェスの番号の割り当て・`\s[番号]` の遮り・外形を覚える表・シェルの最初の面の知らせは、採った形では要らないので作らない。合成の結果を覚える席の数は測るまで変えない。

## Risks & Mitigations

- brief の案から離れたことを開発者が認めない — 設計討議の最初の議題にする。別の道（約束を解いて `plan.rs` を直す）の材料は上の Research Log に在る。
- バルーンの面のコマが替わるたびに文字の層が作り直される — E2E と実機で確かめる。起きたら `areka-emo-present` の側の直しになるので、止めて報告する。
- コマが 4 枚以上の絵で合成が 16 ミリ秒に収まらない — 測って数字を残し、開発者へ報告する（席の数は変えない）。
- `plan.rs` の「命令が出る条件」が同じウェーブの spec で変わる — 束縛の付け替えの前提を design.md の Revalidation Triggers に挙げた。マージの後にテストを回す。

## References

- [descript_shell_surfaces `element*`](https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#element*%2C%E6%8F%8F%E7%94%BB%E3%83%A1%E3%82%BD%E3%83%83%E3%83%89%2C%E3%83%95%E3%82%A1%E3%82%A4%E3%83%AB%E5%90%8D%2CX%E5%BA%A7%E6%A8%99%2CY%E5%BA%A7%E6%A8%99) — 自動アニメーションの正典（requirements.md の C1〜C3）
- [descript_shell_surfaces `always`](https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#always) — C5
- `.kiro/specs/completed/areka-P0-animated-image-decode/` — コマ・待ち時間・繰り返し回数の渡し方
- `.kiro/specs/completed/areka-P0-surface-element-nesting/` — 部品の時計の決まり
