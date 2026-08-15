# Gap Analysis: areka-P0-recompose-budget

> 作成 2026-08-14（validate-gap フェーズ）。requirements.md 確定後・design 前の実装ギャップ分析。
> 本文書の file:line は **2026-08-14 時点の実コード**（W6 完走後の main 由来ワークツリー）に対する実測アンカーである。brief.md の旧アンカー（presenter.rs :360-614 等）は file-slimming（W5.95）のファサード分割で**失効済み**——読み替え表を §1.1 に置く。

---

## 1. 現状調査（Current State Investigation)

### 1.1 アンカー読み替え（brief → 現行実コード）

file-slimming（PR#103）で `presenter.rs` は 118 行のファサードとなり、ホットパスは `presenter/` サブモジュールへ移動した。roadmap 干渉台帳（追記(65)）の 2026-08-14 実測とも一致することを確認済み。

| brief の旧アンカー | 現行の実在位置 |
|---|---|
| `presenter.rs` `apply_show` :360-614 | `crates/areka-emo-present/src/presenter/show.rs` `apply_show` **:32-322** |
| 毎フレーム `compose` 経路 :394 | `show.rs` **:67-76**（`target.composer.compose(...)`） |
| resample 再確保 :403-409 | `show.rs` **:84-90**（:87 `ComposedSurface::new(0,0)`・:88 `resample`） |
| `Target` 構造体 :76-105 | `crates/areka-emo-present/src/presenter/target.rs` `PresentTarget` **:47-157** |
| cache.rs `ComposeCache` :100 | `crates/areka-emo-present/src/cache.rs` **:98-101**（`slot: Option<(ComposeKey, CacheEntry)>`＝容量 1） |
| `compose_into` 定義 :117／`compose` :149 | `crates/areka-emo-compose/src/lib.rs` **:117**／**:149**（不変） |

**`compose_into` の本番呼出点ゼロは現在も成立**（`grep -rn compose_into crates/` の一致は emo-compose 内の定義・doc・テストのみ。emo-present／areka からの呼出は 0 件）。brief の構造的原因①は現行コードでもそのまま生きている。

### 1.2 毎フレーム経路の定常アロケーション棚卸（実測・是正対象の全数）

`apply_show`（show.rs:32）1 回あたり、**アニメ再生中（＝毎コマ必ずキャッシュミス）**に発生する確保・解放:

| # | 発生点 | 内容 | 規模（emo2・200% 実測寸 764×1094 の場合） |
|---|---|---|---|
| A1 | show.rs:67-76 → lib.rs:158 | `Composer::compose` が native 原寸 `ComposedSurface::new(0,0)` を新規確保（内部 `resize_and_clear` で伸長） | native 382×547×4 ≒ **836KB**／コマ |
| A2 | show.rs:87 | k≠1 のとき `ComposedSurface::new(0,0)` を新規確保し `resample` 先にする | 764×1094×4 ≒ **3.3MB**／コマ |
| A3 | scale.rs:423 | `resample` 内部の x 軸写像表 `Vec<AxisSample>`（`with_capacity(out_w)`）を毎呼出確保 | 764×12B ≒ **9KB**／コマ |
| A4 | cache.rs:135-140（show.rs:94-96 の `insert` 内） | `AlphaMask::from_pbgra32` が `vec![0u8; row_bytes*height]` を新規確保（alpha_mask.rs:36） | 96×1094 ≒ **104KB**／コマ |
| A5 | show.rs:96 | `binds.clone()`＋`pattern.clone()`（BTreeMap 複製）を insert キーへ | 小（要素数比例） |
| A6 | cache.rs:147 | スロット置換で**旧エントリ（表示バッファ＋マスク）を drop**＝毎コマ確保と対の解放 churn | 3.4MB 級の free／コマ |
| A7 | show.rs:240 | **ヒット・ミスを問わず毎 apply** `mask_res.set(entry.mask.clone())`＝`AlphaMask`（`Vec<u8>`）の複製 | **104KB**／コマ |
| A8 | show.rs:290 | `last_show = Some((surface_id, binds, pattern))` の置換で旧 BTreeMap を drop | 小 |

- 供給面転写 `chain.upload`（chain.rs:173-229）は定常でヒープ確保なし（`UpdateSubresource` 直書き。寸法変化時のみテクスチャ再作成＝R3.2 の「一度だけ再確保」と整合）。`read_back` は確保するが毎フレーム経路ではない。
- `refresh_scale`（presenter/refresh.rs:52-117）は内部で `apply_show` を再実行する第 2 の消費者。show.rs を是正すれば自動で直る（別修正不要・:61 の `last_show.clone()` は DPI 変化時のみ）。
- 上流 `Composer` 自体は契約どおり（`ops`／`visited` スクラッチ再利用・`compose_into` は要件 10.3 充足・emo-compose の予算檻 `golden_tests_determinism_budget_tests.rs:130` で固定済み）。**破れているのは消費側 emo-present の呼び方**という brief の判定は現行コードで再確認できた。

### 1.3 構造的な対立点（設計の核心）

`ComposeCache` はエントリ（表示バッファ＋マスク）を**値で所有**する（cache.rs:100）。ゆえに「バッファ再利用」と「容量 1 キャッシュ」は素朴には両立しない——ミスのたびに新バッファを作って insert し旧エントリを捨てる現構造では、`compose_into` に切り替えても insert 時に別の確保が要る。定常アロケーション 0 にするには**追い出されるエントリのバッファ容量を回収して次の合成先に再利用する**（スロット・リサイクル）か、`PresentTarget` 側に恒常スクラッチを持たせてキャッシュとの受け渡しを swap にする再構成が必要。ここが本 spec の設計判断の中心になる（§4 の選択肢参照）。

### 1.4 計時・観測の現状（Requirement 1 のギャップ）

- `areka-emo-present` に `std::time::Instant`／計時は**一切存在しない**（grep 0 件）。brief の「500ms は行間隔からの間接推定」は現行も同じ。
- 表示成立点の観測ログは既設: show.rs:303-320 の `info!("apply(ShowSurface): 表示・マスクを更新")` が `cache_hit`・`k_ratio`・`native_*`・`scaled_*`・`size_changed` を構造化フィールドで持つ。**段階別計時フィールドの増設先としてこの成立点が自然**（ただし既存 info! の文言・水準は実機サインオフの契約（doc 明記）ゆえ不変に保ち、計時は R1.2 どおり debug 水準の別行にするか同行追加フィールドにするかが設計判断）。
- ログ檻のテストパターンは既設: emo-compose `log_capture`（テスト専用 tracing 捕捉ハーネス・lib.rs:167）、emo-present `presenter_refresh_and_log_tests.rs`。R1.4「固定文言・水準・全段出現の檻」はこの流儀を流用できる。計時**値**は非決定ゆえ、檻はフィールドの存在と段の全出現のみを主張する（値は実機判定スクリプト側）。
- アロケーション計数（R1.3）: `#[global_allocator]` 差し替えは emo-compose の予算檻が**明示的に棄却済み**（golden_tests_determinism_budget_tests.rs:120-121「プロセス全体を汚染」）。既知の発生点（§1.2 の A1〜A7）が有限個ゆえ、**発生点ごとの計数（バッファ取得シームで new/再確保が起きたときだけカウント）**が決定論的で安価。

### 1.5 計測・判定資産の現状（Requirement 2/4/5 のギャップ）

| 資産 | 現状 | 再利用可否 |
|---|---|---|
| 有界自動終了 | `AREKA_APP_SMOKE_EXIT_MS`（main.rs・実走 92 ファイルで言及・steering 定石） | そのまま使う（7 分＝420000／20 分超＝1500000 級を指定） |
| 実走ハーネス | `crates/areka/tests/emo2_real_run.rs`（env-gate opt-in・`CARGO_BIN_EXE` 子プロセス＋番犬） | 参考形。ただし長時間 2 水準の採取は bindopt サインオフ同様**直接起動**（絶対パス・実 helper 配置）が前例 |
| 判定スクリプト | `completed/areka-P0-bindoption-exclusivity/signoff-scan.py`（Python 標準ライブラリのみ・exit 0/1/2/3・較正値をスクリプト内に明記・「観測ゼロは PASS でなく判定不能=2」） | **設計の型紙**。p50/p95 集計・catch-up 計数・コマ間隔・欠落段のエラー化（R2.5）をこの形式で新造 |
| CPU 時系列 | `Get-Counter` 15 秒刻みの前例は brief 記載のみ・**リポジトリ内に採取スクリプトは存在しない**（cpu-samples.csv はアドホック採取） | **Missing**——採取 ps1（プロセス指定・一定間隔・CSV 出力）の新設が要る |
| catch-up の grep 対象 | `crates/areka-ghost/src/ticker.rs:205,225,307`（`"(loop) ticker catch-up: skipped multiple boundaries, firing once"`・固定文言実在） | 判定式⑵の grep マーカーとしてそのまま使える（変更不要） |
| コマ間隔の期待値 | seriko 側の定義値（brief: まばたき 172ms）はログに直接出ない | 判定スクリプト側の**較正値**とするのが J2 前例と整合（fixture 固有値としてスクリプト内へ明記） |

### 1.6 回帰檻の現状（Requirement 6 のギャップ）

- **檻パターンは確立済み**: emo-compose `surface1000_recompose_steady_state_zero_allocation`（golden_tests_determinism_budget_tests.rs:130）——ウォームアップ後の反復で①バッファ先頭ポインタ不変②長さ不変③④スクラッチ容量非成長を assert する「アプローチ (A)（容量不変 assert）」。presenter 版の檻はこの型をそのまま移植できる（スロット・リサイクル化すればポインタ安定が主張可能になる）。
- バイト等価（R3.3/6.4）の型紙も既設: `compose == compose_into` 等価檻（composer_tests.rs:322）・`chain.rs` の upload→read_back 往復檻（GPU 実描画つきで cargo test 内・steering: areka-no-ci-gpu-tests-in-cargo-test）。
- テスト配置規約: 兄弟ファイル `<stem>_<モジュール名>.rs`＋接続宣言（structure.md）。show.rs 系の檻は `presenter_*_tests.rs` の既存群へ追加する形。

### 1.7 隣接 spec との干渉（W6.5 並走の実測確認）

- **exact（scale-exact-rational）**: 汚染点は `presenter/read.rs:109`（roadmap 台帳が現物一致確認済み）。本 spec の主戦場 `show.rs`／`cache.rs`／`target.rs` とは**別ファイル**＝slimming で緩和済み。ただし本 spec が `scale.rs`（emo-compose）へ resample の scratch API を足す場合、exact の `ScaleRatio` 公開面計画（`ratio()` 等・scale.rs:245-249 の申し送り）と**同一ファイル**になる——追加は関数（`resample` 系）に限れば非衝突だが、先着後 rebase の登記が要る。
- **atom（dpi-transition-atomicity・W6.75）**: show.rs :220-270 帯（スワップ〜upload 域）が atom の観測対象。本 spec の是正は :66-101（合成・リサンプル・insert）と :240（マスク clone）が主で、:220-270 帯の構造は変えない方針が編成条件と整合（atom は budget 実形へ design 前 rebase・roadmap 台帳）。
- **cage④（W6.9）**: 観測点 upload エラー分岐 show.rs:227-232。本 spec が同分岐を移動しないこと。

## 2. Requirement-to-Asset Map

| Req | 技術要素 | 既存資産 | ギャップ（タグ） |
|---|---|---|---|
| 1 計時ログ | 段階別計時・確保計数・檻 | tracing 全面採用・表示成立点 info!（show.rs:303）・log_capture 檻パターン | **Missing**: 計時コードそのもの／確保計数シーム。**Constraint**: 既存 info! の文言・水準は実機サインオフ契約ゆえ不変 |
| 2 ベースライン自動採取 | 有界実走×2 水準・自動集計・CPU 時系列 | AREKA_APP_SMOKE_EXIT_MS・signoff-scan.py 型紙・実走手順 md 群 | **Missing**: CPU 採取 ps1／集計・判定スクリプト本体／手順 md。**Unknown**: 資産の恒久配置場所（§5 論点5） |
| 3 アロケーション是正 | compose_into 切替・再利用バッファ・マスク再利用 | `compose_into`（提供済・未配線）・`resample(&src,k,&mut out)`（out 再利用形は既に取る）・`resize_and_clear` 容量再利用 | **Missing**: `PresentTarget` の再利用バッファ席／キャッシュのスロット・リサイクル API／`AlphaMask` の in-place 再生成 API（wintf 側は `from_pbgra32` の新規確保形のみ）。**Constraint**: cache.rs 容量 1＝R4.1 承認済み要件・キー完全一致規律は不変 |
| 4 機械判定 | 合格判定式⑴〜⑷・較正値明記・dev/release 両適用 | signoff-scan.py の exit code／較正値運用・catch-up 固定文言（ticker.rs:205,225,307） | **Missing**: 判定式の実装。**Unknown**: コマ間隔の期待値ソース（emo2 較正値 172ms をスクリプト内定数にするか・アニメ別に持つか） |
| 5 CPU 上昇切り分け | 収束判定（頭打ち vs 単調上昇） | なし（bindopt 保全ログ 4 本＝計時なし対照のみ） | **Missing**: 収束判定ロジック（スクリプト側・コード非接触）。仮説 (a) は bind 完了で構造消滅済み＝残余は (b)/未知 |
| 6 決定論檻 | ポインタ/容量不変・バイト等価・x64 常設 | emo-compose 予算檻の「アプローチ (A)」・compose 等価檻・GPU readback 檻 | **Missing**: presenter 版の檻（是正後の構造に依存＝design で確定）。**Constraint**: 実時間を檻に入れない（R6.2・steering 記憶とも一致） |
| 7 キャッシュ容量裁定 | ゲート運用 | R4.1 の原文は `completed/areka-P0-emo-present/requirements.md:89` に実在 | ギャップなし（プロセス要件・実装は裁定後のみ） |

## 3. 是正対象の優先順位仮説（第 1 段実測が上書きする前提の現時点整理）

1 コマ 143ms（release）の内訳は未計測だが、コード構造から支配項候補は: ⑴`compose`＝O(elements) の CPU 転写（native 836KB 域）⑵`resample`＝bilinear 全画素（3.3MB 域・200% では毎コマ）⑶`AlphaMask::from_pbgra32`＝全画素 α 走査 ⑷確保・ゼロ埋め・解放 churn（A1+A2+A6 で毎コマ 4MB 級の alloc/free＋ゼロ埋め）⑸`UpdateSubresource` 3.3MB 転送。**dev 500ms／release 143ms の 3.5 倍差**は、確保ゼロ埋めと画素ループの最適化差が主因である示唆（是正は dev に不均衡に効く見込み）。ただし要件 3.4 どおり**着手順は第 1 段の実測が決める**——ここは仮説の記録にとどめる。

なお定常アロケーション 0 を達成しても、ミス毎の再合成＋リサンプル実コストは残る。R4.4（release アイドル CPU 10% 未満）に届かない場合の残余最大項が「キャッシュ不命中による再合成」であれば R7 ゲートへ回す——このとき容量根拠の材料（アニメの循環コマ数と k の組で必要スロット数が決まる）は第 1 段ログから機械抽出できるようフィールド設計しておくとよい（`surface_id`＋`pattern` の異なり数）。

## 4. 実装アプローチ選択肢

### Option A: 全て既存ファイル内で是正（extend）

- show.rs の合成ブロックを `compose_into`＋`PresentTarget` 常設スクラッチへ書き換え、cache.rs にスロット・リサイクル（追い出しエントリの容量回収）を足す。計時も show.rs へ直書き。
- ✅ 変更ファイル最少・干渉台帳のアンカー予測（budget :68-88）と一致
- ❌ show.rs（現 323 行）に計時・計数・再利用管理が同居して肥大／責務混濁。atom・cage④ が同ファイルを見る鎖の最後尾で読みにくさが増す
- ❌ `AlphaMask` in-place 再生成は wintf 側の追加がどのみち必要（emo-present 内では完結しない）

### Option B: 観測・予算管理を新モジュールへ分離（new components）

- `presenter/` 配下へ例: `budget.rs`（再利用バッファ束＝合成先・リサンプル先・マスクスクラッチの一式と取得シーム・確保計数）と `timing.rs`（段階計時の記録器と 1 行サマリ emit）を新設し、show.rs は各段でそれらを呼ぶだけにする。
- ✅ R1.3 の「確保発生点の計数」がシーム 1 箇所に集約され、檻もそのシームを直接検査できる（判断分岐のみ檻に入れる規律と整合）
- ✅ 1,000 行規律・テスト分離規約に素直
- ❌ ファイル数増・show.rs との往復が 1 段増える

### Option C: ハイブリッド（推奨・段階ループと同型）

- **第 0 段（R1）**: 新 `timing.rs`（または `budget.rs` に同居）＝計時・計数の観測基盤を additive に導入。show.rs は各段の計時点を差すだけ・表示結果不変（R1.5 が自明化）。
- **第 1 段（R2）**: スクリプト資産（CPU 採取 ps1＋集計判定 py＋手順 md）を新設（コード非接触）。
- **第 2 段（R3）**: 実測内訳に従い show.rs／cache.rs／（必要なら）wintf `AlphaMask`・emo-compose `resample` の additive API を是正。バッファ再利用は `PresentTarget` の席＋キャッシュのリサイクルで実現。
- **第 3-4 段（R4/5/6)**: 同一スクリプトで再測・機械判定し、確定した構造をポインタ/容量不変檻＋バイト等価檻で固定。
- ✅ 要件の段階ループ（brief ⑷）と 1:1 対応・各段が独立にコミット可能（areka-commit-as-you-go）
- ✅ 上流クレートへの変更が「additive な API 追加」に限定され、completed spec の承認済み契約（compose_into の意味・R4.1 容量 1・AlphaMask 物理 px 契約）を 1 つも書き換えない
  - **※ task 7.3 追記（2026-08-15）**: 本チェックのうち **R4.1 容量 1 だけは開発者裁定を経て書き換えた**（容量 1 → 3・置換方式 LRU・要件 7.1／7.3）。上流 `completed/areka-P0-emo-present` requirements R4.1 を同日付で改訂済み。他の 2 つ（compose_into の意味・AlphaMask 物理 px 契約）は不変で、上流への変更は additive のまま（容量読み口 `ComposedSurface::bytes_capacity`／`AlphaMask::packed_capacity` を追加）。本文書の他の箇所が述べる「容量 1」は**調査時点（設計前）の実装状態**の記録である
- ❌ 計画の複雑さは最大（ただし要件が既にループ構造を規定しており追加コストは小さい）

## 5. 設計判断アイテム（要件ディスカッションへ送る論点）

1. **バッファ再利用の構造**: `PresentTarget` 常設スクラッチ方式か、キャッシュ追い出しエントリの容量リサイクル方式か、併用か。native 合成先（ミス時のみ使用・エントリに残らない）はスクラッチ、表示バッファ・マスクはリサイクルという分担が素直だが、cache.rs の API 形（例: `take_recycled()`／`insert` の引数変更）は R4.1 の意味論（容量 1・完全一致・原子対）を崩さない形の設計が要る。
2. **AlphaMask の再利用 API（wintf 変更）**: `from_pbgra32` は新規確保形のみ（alpha_mask.rs:33-60）。in-place 再生成（`regenerate_from_pbgra32(&mut self, ...)` 等の additive 追加）を wintf へ入れるか。入れない場合 A4 の 104KB/コマは残る（R3.1 は「当たり判定マスク」を明示するため、入れる方向が要件整合）。
3. **【裁定済 2026-08-14 議題1: 対象に含める（完全ゼロ）】** 毎 apply のマスク複製（A7・show.rs:240）の扱い: (a) `AlphaMaskResource` を `Arc<AlphaMask>` 保持へ変え clone を参照カウントにする（wintf 公開面の変更・消費者 = hit_test と bitmap_source systems.rs:376）、(b) エントリ不変（cache ヒット）時は `set` をスキップする（ただし ECS の change detection 挙動が変わる——下流に `Changed<AlphaMaskResource>` 依存が無いかの確認が Research Needed）、(c) 現状維持（104KB/コマの複製を許容し R3.1 の対象外と整理する）。要件文言上 3.1 の列挙は「合成先・リサンプル先・当たり判定マスク」であり A7 は「マスクの複製」＝解釈の裁定が要る。
4. **【裁定済 2026-08-14 議題1: 対象に含める（scratch 受け取り形を additive 追加・exact 先着後 rebase 登記）】** resample 内部 x_map（A3・scale.rs:423）の扱い: 9KB/コマの小確保。「定常アロケーション 0」の範囲に含めるなら emo-compose へ scratch 受け取り形（additive・例 `resample_with`）を足す。含めないなら R3.1 の範囲を「表示用バッファ」（合成先・リサンプル先・マスク）と明文化して除外する。exact と同一ファイルになるため先着後 rebase 登記も必要。
5. **【裁定済 2026-08-14 議題3: リポジトリ級 `tools/perf/` を新設（恒久資産の住所・fixture 固有較正値はスクリプト内明記）】** 計測資産の恒久配置: 前例は spec ローカル（`completed/.../signoff-scan.py`・`verification/*.ps1`）だが、R1/R2 は「以後の性能 spec が再利用する恒久資産」を謳う。リポジトリ級 `tools/perf/`（新設）か spec `verification/` か。言語は判定＝Python 標準ライブラリ・採取＝PowerShell（`Get-Counter`）が前例整合。
6. **【一部裁定済 2026-08-14 議題2: release アイドル CPU 3% 未満（SSP＋emo2 実機実測 2.2〜2.8% の同等圏・GPU が GDI に劣る理由なしの開発者裁定）・dev に CPU 数値目標は課さない。許容率・収束判定の方法と閾値は design へ】** 判定式の較正値の置き場と値: コマ適用間隔の期待値（emo2 まばたき 172ms）はアニメ定義由来＝fixture 固有較正値としてスクリプト内へ明記（J2 前例）。許容率・アイドル CPU 目標（release 10% 未満・R4.4 は調整可と明記）・dev ビルドの数値目標の要否（R4.3 は⑴〜⑶のみ dev へ適用・CPU 値は release のみ——dev に数値を課すか）。収束判定（R5）の方法（例: 後半窓の傾き・移動平均の飽和判定）と閾値。
7. **計時ログの emit 形**: 1 apply = 1 サマリ行（全段のフィールドを 1 行に持つ・集計が最易）か段ごとの行か。既存の表示成立点 info!（実機サインオフ契約・文言不変）との関係——同 info! への追記は契約変更になるため、**別 debug 行**が安全（R1.2 とも一致）。target 名（`areka_emo_present`）と固定文言の設計。
8. **バイト等価檻のレベル**: R3.3/6.4 は「表示バイトと当たり判定マスク」——(a) presenter 経由 GPU readback 往復（chain.rs 檻の型・実描画つき）、(b) 合成層のみの等価（compose vs compose_into＝既設）、(c) 是正前後の golden 固定（是正前バイトをフィクスチャ化）。(a)+(c) の組が要件文言に最も忠実。
9. **確保計数の実装形**: グローバルアロケータ差し替えは前例が棄却済み。バッファ取得シーム（論点1 の構造）に計数カウンタを置き、計時サマリ行のフィールドとして emit する形が决定論的。カウンタの露出（テスト用アクセサ）をどこまで公開するか。
10. **【裁定済 2026-08-14 議題4: 長時間水準はベースラインと最終合格判定のみ・中間ループは 7 分版。実機計測の開始前に開発者へセッションを渡し静寂状態の確認を得る（並行開発セッションの負荷排除）】** 20 分超走行の運用: 長時間実走はサインオフ級の occupied 時間（実機専有）。2 水準（7 分/20 分超）を毎ループ回すか、第 1 段と最終再測のみ 20 分級にするか（中間ループは 7 分で回す）——R2.1/R4.1 の文言は「同一手順」を要求するため、手順書で 2 水準の使い分けを定義しておく必要がある。

## 6. Research Needed（design フェーズへ持ち越す調査）→ **全件解決済（§9 参照・2026-08-14 design フェーズ）**

- ~~`Changed<AlphaMaskResource>`／マスク set の変更検知に依存する下流 system の有無~~ → **0 件と実測確定**（§9.1）。
- dev 500ms / release 143ms の 3.5 倍差の帰属——**設計どおり第 1 段実測で確定する**（両ビルドで同一 perf 行が出る・R4.3。design D10）。
- `UpdateSubresource` の GPU 転写コスト実測比重——**計時段 `t_upload_us` として測る**（design のサマリ行スキーマに恒久フィールド化）。
- ~~CPU 上昇 (b) 仮説の観測フィールド~~ → **seriko 既存 info!（発火・停止・末尾残留）の grep 計数で間接推定可能・活性集合サイズの直接ログは現存しない**と実測確定（§9.2）。コード非接触＝判定スクリプトの grep 対象拡張で足りる。

## 7. 工数・リスク評価

| 作業塊 | 規模 | リスク | 根拠 |
|---|---|---|---|
| R1 計時・計数基盤＋檻 | S | Low | tracing・log_capture・成立点ログの既存流儀へ additive |
| R2 計測スクリプト＋手順 | M | Low-Med | signoff-scan.py 型紙あり。CPU 採取 ps1 は新設・マシン較正の初回擦り合わせが要る |
| R3 アロケーション是正 | M | Med | 3 クレート（emo-present 中心＋wintf/emo-compose の additive API）。キャッシュのリサイクル設計が正しさの要（バイト等価檻でガード）。ただし全て単一スレッド（UI）内・純 CPU 処理で決定論検証可能 |
| R4/R5 再測・判定・切り分け | S-M | Low | スクリプト再実行＋実機 2 水準。ただし R4.4 の 10% 未満は**是正だけでは未達の可能性**があり、その場合 R7 ゲート（開発者裁定待ち）で 1 サイクル延びる |
| R6 決定論檻 | S | Low | emo-compose 檻の型を移植 |
| 総計 | **M〜L**（1〜2 週） | **Medium** | 未知は「是正後に 10% を切るか」の 1 点に集中。切らない場合の経路（R7）も要件が既に用意している |

## 8. 推奨（design フェーズへの申し送り）

- **Option C（ハイブリッド・段階ループ 1:1）を推奨**。第 0 段（観測）を独立コミットで先行させ、以後の全判断を実測駆動にする——要件の自律原則とも一致。
- 上流変更は**すべて additive**（wintf `AlphaMask` の in-place 再生成・emo-compose `resample` の scratch 形）に限定し、承認済み契約の書き換えを R7 の裁定ゲート以外で発生させない。
- show.rs の :220-270 帯（atom 観測域）と :227-232（cage④ 観測点）の**構造を動かさない**是正形を選ぶ（合成・リサンプル・insert ブロックの内側で完結させる）。
- 判定スクリプトは「欠落段＝エラー」（R2.5）と「観測ゼロ＝判定不能 exit 2」（signoff-scan.py 前例）の両規律を最初から仕込む。

---

## 9. Design フェーズ追記（2026-08-14・design.md 生成時の discovery と決定の記録）

> Discovery 分類: **Extension（light discovery）**。§1〜§8 のギャップ分析を土台に、§6 の持ち越し調査をコード実測で解決し（読み取り専用サーベイ 2 本を並列実施）、design.md の決定 D1〜D10 を確定した。design.md が正本・本節は根拠ログ。

### 9.1 調査: AlphaMaskResource の変更検知依存（論点 3 の安全性）→ 依存 0 件で確定

- **Findings**: `AlphaMaskResource`（wintf hit_test/mod.rs:157-177・`Option<AlphaMask>` 値所有・`set`:169／`mask`:174）の本番読み手は `alpha_mask_hit`（mod.rs:199-216・`World::get` の同期 pull）から到達する 2 分岐（mod.rs:318・:419）のみ。書き手は show.rs:238-240 が唯一（他は mount.rs:223 の空初期化）。**`Changed<AlphaMaskResource>`／`Added`／`is_changed()` はリポジトリ全体で 0 件**（一致は本 research.md の記述のみ）。
- **Implications**: 共有化（`Arc`）もヒット時 skip も変更検知の下流影響なし。design は **D3: `Arc<AlphaMask>` 共有＋2 スロット輪番 in-place 再生成**を採用（skip 方式より「set を常に呼ぶ」現行観測形を保てる・エントリとリソースの参照 2／前々回マスク参照 1＝unique の決定論輪番が成立）。`set` は `Arc::new` で包む内部表現変更（公開シグネチャ・挙動不変）＋additive `set_shared`。

### 9.2 調査: CPU 上昇機序の観測フィールド（seriko/ticker ログ棚卸し）

- **Findings**:
  - catch-up ログの**実文言は brief の記載と異なる**: ticker.rs:205/:225 は `"ticker catch-up: skipped multiple boundaries, firing once"`（dispatcher／kanade）、:307 は `"loop ticker catch-up: skipped multiple boundaries, firing once"`——brief の `"(loop) ticker catch-up"` 形は誤り。さらに 3 箇所とも `target = "…"` は**ログのフィールド**であって tracing メタデータ target ではない（メタデータは `areka_ghost::ticker`）＝`RUST_LOG` の target フィルタでは選べず、文言 grep が唯一の判定手段。判定スクリプトは実文言を較正値に持つ（design 済み）。
  - seriko `looper.rs` に活性再生集合サイズの直接ログは**存在しない**。発火（:231 `"seriko: loop 抽選発火"` info・scope/slot/animation_id/k）・bind 外れ停止（:304）・末尾残留（:338）・負 surface 停止（:368）の各 info! を窓別に計数すれば活性集合の収支を間接推定できる。
- **Implications**: R5 の収束判定は CPU 時系列（主判定）＋発火系イベント収支（傍証・間接推定と明記）で構成し、**seriko 側コードは触らない**（Out of Boundary 維持）。

### 9.3 調査: ホットパス・上流 API の設計前再検証（§1 アンカーの追認＋追加確定）

- `apply_show` は cache `get` を 3 回呼ぶ（:62 照会・:136 遅延生成時・:222 アップロード用 entry 再取得）。`insert` の戻り値 `&CacheEntry` は破棄され :222 で再取得している（借用構造の都合・design は不変更）。
- `compose_into`（lib.rs:117-125）は `out` に前提を課さない（内部 `resize_and_clear` が容量再利用）。`resize_and_clear`／`bytes_mut` は **emo-compose の `pub(crate)`**——emo-present からの直接容量操作は不可で、`compose_into`／`resample` 経由の再利用のみ可能（design はこの可視性を変えない）。
- `resample`（scale.rs:395）の残存確保は x 軸写像表 `Vec<AxisSample>`（:423・`AxisSample` は私有型）のみ。scale.rs:245-249 に exact（W6.5）向け公開面申し送りが実在——additive 追加は opaque 型＋関数に限定すれば非抵触。
- `AlphaMask`（wintf alpha_mask.rs:15-20・全フィールド私有）に再利用 API・`Default`・`PartialEq` は存在しない——`regenerate_from_pbgra32` の additive 追加が必須（design D3 の前提）。
- emo-present の tracing は**全 57 箇所が module-path target**（明示 target 0 件・`RUST_LOG=areka_emo_present=debug` の prefix フィルタが実機手順の契約・hit.rs:77）。emo-compose は対照的に明示 flat target `"areka_emo_compose"`。perf サマリ行は emo-present 流儀（module-path）に従う（design 済み）。
- 既存 info! 成立点ログの檻 `presenter_refresh_and_log_tests.rs:78-150` は **info 水準そのものを契約として赤にする**。同ファイル :661-697 に「出ないこと」主張の callsite interest キャッシュ罠と対処（同一 `with_default` スコープで陽性・陰性を対で観測）が記録済み——新設の計時檻はこの流儀を踏襲する（design Testing Strategy に明記）。
- emo-present は `tracing-subscriber` を dev-dep に持たず**自前 `CaptureSubscriber`**（presenter_refresh_and_log_tests.rs:45-69）——新檻も同ハーネスを使う（新規依存なし）。

### 9.4 調査: 計測・判定資産の型紙（signoff-scan.py／実走ゲート）

- `signoff-scan.py`（472 行・stdlib のみ）の踏襲規律: 較正値は冒頭の定数バナー（「emo2 fixture 固有・他ゴーストへ流用しないこと」明記・裁定履歴をコメントで登記）／exit `0`=PASS・`1`=FAIL・`2`=判定不能（観測ゼロ含む・**FAIL が INCONCLUSIVE より優先**）・`3`=引数不正／観測ゼロ＝判定不能は 3 箇所で個別に実装（全体・判定別）。
- `AREKA_APP_SMOKE_EXIT_MS`（main.rs:905・`smoke_exit_ms_from`:914 は不正入力を常に OFF へ）は起動窓 despawn の one-shot 非同期タスク（:804-836）＝有界 auto-exit の既存流儀そのまま使える。`emo2_real_run.rs` の 120 秒番犬は cargo test 用であり、7 分／20 分超の perf 実走は **bindopt 前例どおり直接起動（絶対パス）**で行う（design ランナー仕様に反映）。
- リポジトリ root に `tools/` は**不存在**（新設）。既存 ps1 の前例は spec ローカル 3 形式（`verification/`・`tools/`・平置き）で分裂しており、`Get-Counter` を使う採取スクリプトは**リポジトリ内に 0 件**——`tools/perf/` 新設（議題 3 裁定）で恒久の住所を与える。

### 9.5 Design Decisions（design.md D1〜D10 の採否根拠の要約）

| # | 決定 | 棄却した代替 | 根拠 |
|---|---|---|---|
| D1 | Option C ハイブリッド（§4 の推奨を採用） | Option A（show.rs 直書き）＝責務混濁・Option B 単独＝段階ループと非対応 | §4・要件の段階ループと 1:1 |
| D2 | 席の分担: native=常設スクラッチ（swap 交代）／表示=キャッシュ容量リサイクル／x_map=常設席／マスク=Arc 輪番 | 全席を budget 所有（キャッシュ原子対の外に表示バッファが出る）・全席をキャッシュ所有（native スクラッチの置き場がない） | §1.3 の対立点を「所有はキャッシュ・容量回収の仲介は budget」で解消 |
| D3 | マスクは `Arc` 共有＋2 スロット輪番 | (b) ヒット時 set skip（安全だが「常に set」の現行観測形が変わる）・(c) 現状維持（裁定違反） | §9.1・議題 1 裁定「完全ゼロ」 |
| D4 | マスク生成点を cache.insert から budget シームへ移動（insert は Arc を受ける） | insert 内生成の維持（budget との二重確保・計数シームが分裂） | 「1 apply 1 回生成・原子対挿入」は apply 単位で不変 |
| D5 | 計時は無条件実行・emit のみ debug フィルタ | `enabled!` ゲート（ログ設定で実行経路が分岐＝R1.5 の検証が難化） | R1.5 の構造的自明化・Instant は段あたり数十 ns |
| D6 | 確保計数は budget 取得シーム 1 箇所 | `#[global_allocator]` 差し替え | emo-compose 予算檻が明示棄却済み（golden_tests_determinism_budget_tests.rs:120-121） |
| D7 | ランナー ps1＋判定 py＋README＋自己較正 fixture を tools/perf/ へ | spec ローカル配置（前例 3 形式に分裂・恒久資産の住所にならない） | 議題 3 裁定・§9.4 |
| D8 | バイト等価＝便宜経路 vs budget 経路の層内等価＋既存 GPU readback 檻 | 是正前バイトの golden fixture 化（ビルド間安定性の管理コストが増えるだけで検出力は同じ） | composer_tests.rs:322 の既設パターンを presenter 級へ拡張 |
| D9 | 不変量の対象は A1〜A4・A6・A7（キー複製 A5/A8 は対象外と明文化・観測は継続） | キー複製まで含める（BTreeMap ノード再利用は std に手段がなく費用対効果が壊れる） | 裁定の列挙（バッファ＋マスク複製＋作業領域）に忠実 |
| D10 | 着手順は第 1 段実測が決め、仮説と異なれば design 追補 | 仮説順の固定 | R3.4 |

### 9.6 Risks & Mitigations（design 時点の残リスク）

- **R4.4（release 3% 未満）が是正だけでは未達の可能性**——残余最大項が再合成コストなら R7 ゲートへ（要件が経路を用意済み）。perf 行の `key_hash` で必要スロット数の実測根拠を最初から採れるよう設計済み。
- **exact との scale.rs 同一ファイル並走**——追加は関数＋opaque 型のみ・先着後 rebase を干渉台帳へ登記（実装フェーズの手続き）。
- **計時 mark の挿入で show.rs の行番号が全面シフト**——atom（W6.75）は budget 実形へ design 前 rebase が既定（roadmap 台帳）。:215-235 帯は分岐・呼出順序・エラー経路の構造を保つ（変更は mark 挿入と :240 の 1 文置換のみ）。
- **マスク輪番の unique 不成立**（想定外の長期参照保持）——黙って新規確保＋必ず計数＝檻と判定式⑶が検出する（隠れ縮退なし）。
