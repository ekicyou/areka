# ギャップ分析: areka-P0-scale-exact-rational

> **実施日**: 2026-08-14 ／ **対象**: 確定済み requirements.md（R1〜R5）と既存コードベースの差分
> **ブランチ**: `claude/areka-p0-scale-exact-rational`（ワークツリー `areka-p0-balloon-visibility-b341d2`）
> **調査方法**: Grep/Glob/Read による実測（全アンカーは本ドキュメント作成時点の file:line）。外部依存の追加は不要と判明したため WebSearch/WebFetch は実施していない。
> **位置づけ**: 情報提供であって決定ではない。設計選択肢と設計判断事項を提示する。

> ## ⚠️ 2026-08-14 裁定により本文の大半は失効（§9 が優先）
>
> 要件ディスカッションで**開発者が厳密化を却下**し、f32 のまま引き回すことが裁定された。
> これにより §3（実装アプローチ）・§7（設計判断事項 #1〜#10）・§8（設計フェーズへの推奨）は**全て失効**する。
> **§1（現状調査）・§2 の資産マップ・§5（並走干渉）は裁定後も有効**（現物の記述として）。
> 裁定の根拠と、失効した設計判断の個別処遇は **§9** に記す。

---

## 0. アンカー再突合（brief 追記(60) からのドリフト）

brief の追記(60)（2026-08-06・col マージ後）以降、`file-slimming`（W5.95）と W6 の 4 実装が着地しており、**汚染点のファイル自体が移動している**。設計・実装は以下の実測値を用いること。

| 対象 | brief 追記(60) の記載 | **2026-08-14 実測** |
|---|---|---|
| f32 汚染点 | `presenter.rs:682` `scale: applied.as_f32()` | **`crates/areka-emo-present/src/presenter/read.rs:109`**（同式） |
| `TextSlotView` 定義 | `presenter.rs:233` 付近 | **`presenter/read.rs:16-29`**（`scale: f32` は **:28**・`scale()` は **:77-79**） |
| `text_slot_view` 構築 | `presenter.rs:676-683` | **`presenter/read.rs:95-111`**（構築式は **:103-110**） |
| `applied_ratio` | `presenter.rs:744` | **`presenter/read.rs:171-173`** |
| `ScaleRatio::as_f32` | `scale.rs:147`（doc :142） | **`crates/areka-emo-compose/src/scale.rs:147`**（doc **:139-146**）＝一致 |
| `scale_len` | `scale.rs:169` | **`scale.rs:169-182`**＝一致 |
| `unscale_coord`／W6.5 申し送り | `scale.rs:253`／`:245-249` | **`scale.rs:253-260`／`:245-249`**＝一致 |
| emo-text 本番呼出 | `actor.rs:665-666` | **`crates/areka-emo-text/src/actor.rs:664-667`**（`physical_extent` 2 呼出は **:665** と **:666**）＝一致 |
| `physical_extent` 実体 | `region.rs:119` | **`crates/areka-emo-text/src/region.rs:119-121`**＝一致 |

**ロードマップ台帳（`roadmap.md:84`）の「exact の read.rs :109」は現物と一致**している。brief 本文の `presenter.rs` 系アンカーのみが陳腐化している。

---

## 1. 現状調査（Current State）

### 1.1 丸め権威の現況（`areka-emo-compose`）

`crates/areka-emo-compose/src/scale.rs` が k の数学の単一権威。

- `ScaleRatio { num: u32, den: u32 }`（**:47-52**）は**私有フィールド**。不変条件（`num ≥ 1`・`den ≥ 1`・既約）を `new`（**:70-79**）／`mul`（**:100-129**）だけが確立する。
- 公開面は `ONE`（:63）／`new`／`mul`／`is_identity`（:135）／`as_f32`（**:147**）／`scale_len`（**:169**）／`scaled_extent`（:190）／`unscale_coord`（**:253**）の 8 つ。**`num()`／`den()`／`ratio()` アクセサは存在しない**（W6.5 の命名と二重化しないよう意図的に温存＝**:245-249** の申し送り）。
- モジュール doc（**:1-25**）が「画素・寸法演算に浮動小数（**f32/f64**）を一切持ち込まない」と宣言。`as_f32` の doc（**:139-146**）が「**寸法・画素演算にこの値を使ってはならない**」と明記。
- 丸め規約は 2 系統のみ: 乗算方向の長さ＝round half away from zero（`scale_len`）／除算方向の座標＝画素中心逆写像（`unscale_coord`）。**`ceil` 系の権威関数は存在しない**。
- 桁溢れ規律の先例: `scale_len` は u128 中間＋`u32::MAX` 飽和、`unscale_coord` は i128 中間＋i64 飽和。いずれも非パニック宣言付き。

### 1.2 搬送経路の現況（`areka-emo-present`）

- `presenter/read.rs:103-110` が `TextSlotView` を構築。**物理寸は丸め権威を通し**（`:108` `physical_size: applied.scaled_extent(...)`）、**k だけが f32 へ落ちる**（`:109` `scale: applied.as_f32()`）。この 1 行が要件の言う「配管の上での契約違反」の唯一の発生点である。
- `applied_ratio(target) -> Option<ScaleRatio>`（**:171-173**）が既に存在＝**有理厳密の照会面は新設不要**。真実源は同一の私有 `PresentTarget.applied`。
- **`ScaleRatio` は `areka-emo-present` が再輸出済み**（`crates/areka-emo-present/src/lib.rs:62` `pub use areka_emo_compose::ScaleRatio;`・doc は :57-61）。
- `TextSlotView` はフィールド私有＋アクセサのみ、構築点は `text_slot_view` ただ 1 箇所。**フィールド追加は additive で、emo-present 側の既存テストは構築式に触れないため無改修**。

### 1.3 消費経路の現況（`areka-emo-text`）

- `region.rs:59-65` `ScaleContract { pub scale: f32, pub author_dpi: u32 }`。構築は `new(scale: f32, author_dpi: Option<u32>)`（**:72-86**）のみで、不正 k（0 以下・非有限）を `warn!`＋1.0 へ縮退する（**:73-81**）。
- `physical_extent(&self, v: ImagePx) -> u32`（**:119-121**）＝ `(v.0 * self.scale).ceil() as u32`。**これが是正対象の唯一の算術**。doc **:98-118** に既知欠陥の登記（2026-07-30 実測表・「担当 spec 不在」注記）が生きている。
- 本番の `physical_extent` 呼出は `actor.rs:665`／`:666` の **2 箇所のみ**（`present_actor` の初回解決分岐で validrect 寸→供給面寸）。
- 連続量の消費者（**本仕様の Out of scope**・R3.5/R4.4 で不変を要求）:
  | 用途 | 位置 | 式 |
  |---|---|---|
  | 描画の拡大変換（D2D `SetTransform`） | `draw.rs:828`・`:831` | `M11/M22 = contract.scale` |
  | 当たり矩形の物理化 | `choice.rs:267`（`to_window_physical`） | `(座標) * k` |
  | 供給面装着 offset | `actor.rs:669-670` | `to_physical(...)` |
  | スクロール位置 | `viewbox.rs:148` | `to_physical(...)` |
  | ダーティ矩形のインク寸 | `viewbox.rs:650` | `let k = contract.scale` |
  | ダーティ矩形のガード余白 | **`viewbox.rs:734`** | `(DIRTY_GUARD_IMG_PX * contract.scale).ceil() as i64` |
  | ダーティ矩形限定描画 | `viewbox_draw.rs:417` | `let k = contract.scale` |
- `TextSlotBinding`（`actor.rs:47-64`）は `pub scale: f32`（**:54**）を持ち、`new`（**:84-99**）が `ScaleContract::new` へ正規化を委譲、`from_view`（**:116-124**）が `TextSlotView` から一点変換する。
- `TextSlotBinding` は `Copy + PartialEq` で、**再追従の churn 判定キーそのもの**（`actor.rs:383` の `current == binding`・k のログは `:372`）。doc **:376-378** が「f32 は出口ビュー——ここでは比較にのみ使い、寸法演算には一切用いない」と既に明記。

### 1.4 依存方向（**設計上の重要発見**）

- `crates/areka-emo-text/Cargo.toml`: `areka-emo-present` は**本番依存**（コメント付きで「読み取り消費のみ・逆方向 import 禁止」）。`areka-emo-compose` は **dev-dependency のみ**。
- したがって **`areka_emo_present::ScaleRatio`（再輸出）を使えば Cargo.toml を一切触らずに `ScaleRatio` 型が本番コードへ到達する**。R2.6（新規外部依存なし）は**現状の依存グラフのまま充足可能**。
- ただし `region.rs` は先頭 doc（**:6**）で「**純粋層**——`windows` 系 crate への依存を一切持たない（決定論檻）」と宣言している。`ScaleRatio` 自体は windows 非依存だが、**import 経路が `areka-emo-present`（wintf/windows 依存）を通る**点は層規律の見え方として設計判断が要る（→ 設計判断 #4）。
- `crates/areka/Cargo.toml:39` は `areka-emo-compose` を直接依存済み（`crates/areka/src/input_events/balloon_pure_core_tests.rs:155` の呼び手はどちらの経路でも書ける）。

### 1.5 呼び出し箇所の実測（署名追随の規模）

| 構築口 | 総数 | 本番 | テスト／in-crate 檻 |
|---|---|---|---|
| `ScaleContract::new(` | **68** | **2**（`actor.rs:91`＝`TextSlotBinding::new` 内・`actor.rs:658`＝`present_actor`） | 66（15 ファイル・最多は `viewbox_dirty_tests.rs` 12／`viewbox_plan_commit_tests.rs` 11／`region.rs` in-crate 檻 8） |
| `TextSlotBinding::new(` | **44** | **1**（`actor.rs:117`＝`from_view` 内） | 43（14 ファイル・最多は `actor_scale_refresh_tests.rs` 11／`actor_choice_contract_tests.rs` 6／`surface.rs` in-crate 檻 5） |
| `physical_extent(` | 15 | **2**（`actor.rs:665`・`:666`） | 13（うち `region.rs` 檻 5・`tests/scale_invariance_test.rs` 5・doc 3） |

brief の「41 + 67」は「本番を除く数」に相当し、現物は **43 + 66**。合計 112 箇所のうち**本番は 3 箇所**で、残りは全て機械的追随（R2 Adjacent expectations／[[test-only-decision-branches-not-proven-wiring]] のとおり新規テストを要求しない配線）。

**追随の実形**（設計時の見積り材料）:
- 大半は `ScaleContract::new(1.0, None)` → 恒等の新記法へ 1 トークン置換。
- k≠1 を使う檻は **1.25／2.0／0.8** に集中（`region.rs:647,654`・`viewbox_axis_tests.rs:86,109`・`viewbox_dirty_tests.rs:440`・`choice_tests.rs:381,401,426`・`tests/scale_invariance_test.rs`・`tests/attach_wiring_test.rs:678,752`）。いずれも f32 厳密表現可能＝R4.1 のバイト同一対象。
- **テストヘルパの引数型も追随する**（`k: f32` を取るヘルパ: `viewbox_draw_live_diff_tests.rs:64,75,401`・`draw_oracle_tests.rs:75`・`viewbox_plan_commit_tests.rs:356`・`tests/scale_invariance_test.rs:66,77`）。

### 1.6 既存テスト資産と規約

- **変異キル記録の先例が同一 crate 内にある**: `crates/areka-emo-compose/src/scale_ratio_tests.rs:305-328` が「殺す変異／実測失敗本数／排他キルの有無」を doc コメントで宣言する様式を確立済み。R5.3 の「計測日を添えて記録」はこの様式の延長で満たせる。
- **`ceil` の丸め正準を守る既存檻**: `crates/areka-emo-text/tests/scale_invariance_test.rs:292-320`（`physical_extent_ceils_fractional_values_killing_round_and_floor`・321×1.25→402／123×1.25→154）。是正後も緑を保つ必要がある（R4.1）。
- **陳腐化候補**: `region.rs:677-683` `invalid_scale_falls_back_to_one_with_warn`（f32 の 0／負／NaN／Inf を投入して warn＋1.0 縮退を主張）。構築口が有理数のみになると**入力そのものが表現不能**になる（→ 設計判断 #3）。
- テスト配置規約は `#[path]` による外出し（`scale.rs:468-478` が `scale_ratio_tests.rs` 等を取り込む形）。W5.95 file-slimming の成果として 1 ファイル 1,000 行超はゼロ。

### 1.7 欠陥の再確認（静的証跡）

`region.rs:120` は f32 単精度で `v × k` を評価してから `ceil` する。k=6/5 のとき最近傍 f32 は `1.20000004768371582…`。v=25 では真の積が `30.0000011920928955…`、30 近傍の f32 刻みは 2^-19≈1.907e-6 ゆえ**最近傍 f32 は 30 より上**へ丸まり、`ceil` が 31 を返す（正 30）。brief の実測表（81/1200）と整合する。**要件 1.2 の再現条件は既存コードから静的に導出可能**であり、実機を要しない（R5.6 とも整合）。

---

## 2. 要件↔資産マップ（gap タグ）

| 要件 | 既存資産 | ギャップ | 種別 |
|---|---|---|---|
| R1.1 真値一致 `ceil(寸×num/den)` | `region.rs:119-121`（f32 積） | 厳密算術が**不在**。`ScaleRatio` にも `ceil` 系権威が無い | **Missing** |
| R1.2 k=6/5・1..1200 全一致 | 檻は 1.0/1.25/2.0 のみ | 非二進比の網羅檻が**不在** | **Missing** |
| R1.3 tie は切り上げない | `scale_len` の tie 檻（`scale_ratio_tests.rs:329-345`）が様式の先例 | `physical_extent` 側の tie 檻が**不在** | **Missing** |
| R1.4 恒等は入力の切り上げ | `region.rs:638` `physical_extent(320.0)==320` | 恒等時の**分数入力**の挙動が未定義（→ 判断 #2） | **Unknown** |
| R1.5 寸 0 → 0 | 明示の檻なし（`as u32` の飽和で偶然 0） | 0 の明示檻が**不在**（`scale_len:170-172` に先例あり） | Missing（軽微） |
| R1.6 f32 を寸法導出に使わない | `as_f32` doc の宣言のみ | 配管が宣言を破っている＝**本仕様の中核** | **Missing** |
| R1.7 決定論・非パニック | u128 中間＋飽和の先例（`scale_len`/`unscale_coord`） | 同型を踏襲すれば充足。新規機構は不要 | Constraint（充足容易） |
| R2.1 提示段が num/den を公開 | `applied_ratio`（read.rs:171）が既存・`TextSlotView` は私有フィールド＋構築点 1 箇所 | `TextSlotView` への additive 追加のみ | **Missing（小）** |
| R2.2 f32 と有理の一致 | 同一の `t.applied` から両方を導く構造（read.rs:103-110） | 構造的に既に成立。檻で固定するだけ | Constraint |
| R2.3 文字層が整数対を保持 | `ScaleContract` は f32 のみ | フィールド追加＋構築口変更 | **Missing** |
| R2.4 未確定なら恒等で失敗させない | `text_slot_view` は `applied` 未確定なら `None`（read.rs:101）／`ScaleRatio::default()==ONE`（scale.rs:54-58） | 既存の縮退構造で充足。どこで恒等化するかの明示が要る | Constraint |
| R2.5 num/den が 0 なら warn＋恒等 | `ScaleContract::new:73-81` の warn＋1.0 が同型 | **`ScaleRatio` を搬送すると 0 は型で不可能＝分岐が到達不能になる**（→ 判断 #3） | **Constraint（要裁定）** |
| R2.6 新規外部依存なし | `lib.rs:62` の再輸出で `ScaleRatio` が到達可能 | **既に充足**（Cargo.toml 改変すら不要） | 充足済み |
| R3.1/3.2 構築口はただ 1 つ・f32 口を残さない | `ScaleContract::new` は現状 f32 単独 | 署名変更＋112 箇所の追随 | **Missing（機械作業）** |
| R3.3 全呼び手移行でビルド・テスト成功 | 上記 112 箇所（本番 3・テスト 109） | 追随のみ | Constraint |
| R3.4 丸め規約を新設しない | `scale.rs` は round／逆写像の 2 権威。`ceil` は `physical_extent` 側の既存規約 | 「ceil をどこへ置くか」で権威の見え方が変わる（→ 判断 #1） | **Unknown** |
| R3.5 連続量は f32 継続可 | 上表 1.3 の 7 箇所 | 変更しない＝そのまま充足 | 充足済み |
| R4.1/4.2 バイト同一 | 檻の k は 1.0/1.25/2.0/0.8（全て f32 厳密） | 是正後も同値であることの確認だけ | Constraint |
| R4.3 既存テストを赤にしない | 陳腐化候補 1 本（`region.rs:677-683`） | 退役／更新の裁定（[[obsolete-vs-broken-test-policy]]） | **Unknown** |
| R4.4 連続量の結果不変 | `viewbox.rs:734` は**寸法へ ceil する f32 経路**だが要件が明示的に Out of scope | 触らない判断の明文化が要る（→ 判断 #6） | Constraint |
| R4.5 k=6/5 でレイアウト不変 | レイアウトは image 空間で決まる構造（`TextRegion::resolve` は k を受けない・`scale_invariance_test.rs` が固定） | 構造的に成立。檻の追加は任意 | 充足済み |
| R5.1 是正前赤・是正後緑 | — | **テストを先に書いて赤を記録する着手順序**が要る（→ 判断 #7） | **Missing（手順）** |
| R5.2 tie 檻 | `scale_ratio_tests.rs:329-345` の隣接 3 点様式 | 同様式で新設 | Missing（小） |
| R5.3 変異の排他キル＋計測日 | `scale_ratio_tests.rs:305-328` の記録様式 | 同様式で新設 | Missing（小） |
| R5.4 既知欠陥登記の削除 | `region.rs:98-118` | 削除のみ | Missing（小） |
| R5.5 配線に新規テストを足さない | 112 箇所は配線 | 規律の遵守 | Constraint |
| R5.6 実 DPI/GPU/実窓不要 | `ScaleRatio` は純粋・`ScaleContract` は純粋層 | 既存構造で充足 | 充足済み |

**複雑度シグナル**: 算術ロジック（極小）＋大規模な機械的署名追随（112 箇所）。外部統合・ワークフロー・データモデルの新設はゼロ。

---

## 3. 実装アプローチの選択肢

本仕様は「配管（搬送）」と「算術（厳密化）」の 2 軸から成る。軸ごとに独立して選べるため、以下は**軸別の選択肢**として整理する。

### 3.1 軸 A: 厳密算術をどこに置くか

#### A-1: `ScaleRatio` に切り上げ権威を新設（`scale.rs` 側）
`ScaleRatio::scale_len_ceil(len: u32) -> u32`（仮称）を `scale_len` の隣に置き、`(len·num).div_ceil(den)` を u128 中間で計算する。`ScaleContract::physical_extent` は寸を整数化して委譲するだけの薄い口になる。**アクセサ（`ratio()`／`num()`/`den()`）を一切新設せずに済む**点が最大の利点。

- ✅ 丸め権威が物理的にも `scale.rs` 1 ファイルへ集約（R3.4「丸め権威は有理スケール単独」の最も素直な読み）
- ✅ `scale.rs` の整数専用規約（f32/f64 不使用）を破らない
- ✅ 桁溢れ・飽和・非パニックの既存規律（u128＋飽和）をそのまま踏襲でき、変異キル檻も既存様式の隣へ置ける
- ✅ emo-text 側の差分が最小（`physical_extent` の中身が 1 行の委譲になる）
- ❌ `scale.rs` の丸め規約が 2 → **3 系統**（乗算 round・除算逆写像・乗算 ceil）に増える。doc の「単一権威」節の再記述が必要
- ❌ 供給面という**上位の都合**（文字が切れないための ceil）が下層 crate の公開面へ現れる（責務の逆流という指摘があり得る）

#### A-2: `ScaleRatio::ratio() -> (u32, u32)` を新設し、算術は `region.rs` で整数演算
brief の Approach に最も近い形（ただし f64 ではなく整数）。`physical_extent` が `(v·num).div_ceil(den)` を u64/u128 で組む。

- ✅ `scale.rs` の公開面が読み取り専用アクセサ 1 つだけで済み、丸め規約は増えない（`scale.rs:245-249` の申し送りが想定していた形）
- ✅ 「供給面の ceil 規約」が供給面の持ち主（emo-text）に留まる
- ❌ **丸めの実装が `scale.rs` の外へ 1 つ増える**。R3.4 の「新たに定義せず」は「現行の ceil 規約を維持する」の意なので違反ではないが、権威の所在が読み手に 2 箇所へ見える
- ❌ `ratio()` の戻り値をどう扱うかは呼び手の自由になり、将来の誤用余地（掛け算の再発明）を残す

#### A-3: `region.rs` で f64 演算（brief の当初 Approach）
`((v as f64) × num / den).ceil()`。整数 v・u32 分子分母に対して f64 の相対誤差 ~1e-16 は最小非 tie 距離 1/den より遥かに小さく、実用上は厳密。

- ✅ 差分が最小（1 行）・呼び出し面の変更が最も少ない
- ❌ **「寸法演算に浮動小数を持ち込まない」という上流規約の精神に反する**。R5.3 が要求する「f32 経由への差し戻し」変異検出はできても、「f64 でも十分」という次の議論を呼び込む
- ❌ 厳密性の根拠が「誤差解析」になり、レビューで毎回同じ証明を読ませる（整数版は自明）
- ❌ u32 の極値（num・v とも 2^32 近傍）では f64 の 53bit 仮数で厳密性が崩れ得る＝R1.7 の「いかなる寸・拡大率の組合せでも」に対する反例構成が可能

#### A-4: ハイブリッド（A-1 の権威関数＋`is_identity` 早期復帰の踏襲）
A-1 に加え、`scale_len` と同じ「`len==0`→0／恒等→素通し」の早期復帰を持たせる。R1.4／R1.5 が構造で満たされ、檻も既存様式と対称になる。**A-1 の実装細部として最も自然**。

### 3.2 軸 B: 有理スケールをどう搬送するか

#### B-1: `TextSlotView` に `ScaleRatio` を additive 追加（型で運ぶ）
`presenter/read.rs:103-110` の構築式へ `ratio: applied` を足し、`scale_ratio() -> ScaleRatio` を公開。emo-text は `areka_emo_present::ScaleRatio`（再輸出）で受ける。

- ✅ 不変条件（非ゼロ・既約）が**型で保証**され、下流が壊せない
- ✅ Cargo.toml 改変ゼロ・新規依存ゼロ（R2.6 充足）
- ✅ `applied_ratio(target)` と同一値であることが構造的に自明（同じ `t.applied`）
- ❌ **R2.5（num/den が 0 なら warn＋恒等）の分岐が到達不能になる**（→ 判断 #3）
- ❌ `region.rs` の「純粋層」宣言に対し、import 経路が emo-present を通る（→ 判断 #4）

#### B-2: `(u32, u32)` の生の整数対で搬送
`TextSlotView::scale_ratio() -> (u32, u32)`、`ScaleContract::new(num: u32, den: u32, author_dpi)` が内部で `ScaleRatio::new` に通し、`None` なら `warn!`＋`ONE`。

- ✅ **R2.5 が生きた判断分岐になる**（既存の `invalid_scale_falls_back_to_one_with_warn` を整数入力へ更新して延命でき、R4.3 の陳腐化も最小）
- ✅ 純粋層 `region.rs` が emo-present の型を一切知らない（層規律の見え方が最も綺麗）
- ❌ 型で保証されていた不変条件を一度ほどいて再構築する＝「割れた真値」の芽を自ら作る、という批判が成り立つ
- ❌ 既約でない対（例 120/96）が渡り得るため、`Eq` の正準性（キャッシュキー一意性）と噛み合わせる責務が下流へ移る

#### B-3: 併載（`ScaleRatio` を運びつつ `(u32,u32)` も公開）
- ❌ **R3（入口の単一性）の精神に真っ向から反する**。推奨しない。列挙のみ。

### 3.3 軸 C: `ScaleContract` の形

#### C-1: `ratio` を保持し `scale: f32` は派生フィールドとして残置
`ScaleContract { ratio, scale: ratio.as_f32(), author_dpi }`。連続量の 7 消費点（`draw.rs:828/831` 他）は**一切無改修**。

- ✅ R4.4（連続量の結果不変）が構造的に自明。差分が最小
- ✅ `PartialEq` は ratio と f32 の両方を見るが、f32 は ratio の関数ゆえ矛盾しない（churn 判定 `actor.rs:383` は厳密化される方向）
- ❌ 公開フィールド `pub scale: f32` が残るため「寸法演算に使うな」を doc でしか守れない（現状と同じ弱さ）

#### C-2: `scale` をメソッド化（`k_f32()`）して寸法誤用を目立たせる
- ✅ 出口ビューであることが呼び出し面から読める
- ❌ 連続量 7 箇所＋テストの追随が増える（配線だが差分が広がる）

#### C-3: `TextSlotBinding` も同様に `ratio` を持つ
`TextSlotBinding::new` は `scale: f32` を受けられなくなる（R3.2 の帰結）ので、**`ScaleRatio` を受ける形へ必然的に変わる**。`Copy` は維持可能（`ScaleRatio` は `Copy`）。churn キーとしての `PartialEq` も維持。**44 箇所の追随の主因はここ**。

### 3.4 総合案（組合せ）

| 案 | 軸 A | 軸 B | 軸 C | 性格 |
|---|---|---|---|---|
| **案 I（権威集約型）** | A-1/A-4 | B-1 | C-1 | 権威を `scale.rs` へ寄せ、型で不変条件を運ぶ。差分最小・レビュー負荷最小。R2.5 が到達不能になる点の裁定が要る |
| **案 II（層純度型）** | A-2 | B-2 | C-1 | `region.rs` が純粋層のまま整数演算を持つ。R2.5 が生きる。丸めの実装が 2 箇所に見える |
| **案 III（最小差分型）** | A-3 | B-1 | C-1 | f64 で 1 行是正。着地は速いが上流規約の精神に反し、極値の厳密性に穴 |

**gap-analysis の枠組みで言えば**: 案 I は「既存コンポーネントの拡張（Option A）」寄り（`ScaleRatio` に権威メソッドを 1 本足す）、案 II は「新規責務の分離（Option B）」寄り（emo-text が自前の整数演算を持つ）、案 III は純粋な現状維持の延長。

---

## 4. Effort / Risk

| 項目 | 評価 | 根拠（一行） |
|---|---|---|
| **Effort** | **M（3〜7 日）** | 算術と搬送は各 S だが、**112 箇所の署名追随が 2 crate ＋ areka にまたがる**（20 ファイル超）。ビルドを通す作業量がクリティカルパス |
| **Risk（算術）** | **Low** | 純関数・決定論・実機不要。既存の u128＋飽和の先例をなぞるだけ |
| **Risk（搬送）** | **Low** | 依存追加ゼロ・構築点 1 箇所・additive |
| **Risk（署名追随）** | **Medium** | 機械的だが量が多く、**k≠1 の檻（1.25/2.0/0.8）の変換ミスが静かに緑のまま通る**恐れ。バイト同一（R4.1）を明示的に確認する手順が要る |
| **Risk（並走干渉）** | **Low** | 下記 §5 のとおり編集面が素 |

---

## 5. 並走 spec との干渉（着手時実測・2026-08-14）

- **`areka-P0-recompose-budget`（W6.5 並走）**: 編集面は `crates/areka-emo-present/src/presenter/show.rs`（`apply_show` :68-88 帯）。本仕様は同 crate の **`presenter/read.rs`** と `lib.rs`（再輸出を使うだけなら無改変）。**別ファイル＝素**。roadmap.md:84 の判定と実測が一致。
- **`areka-P0-windowposition-limit`（W6.5 並走）**: `placement/` 系ドメイン。本仕様は emo-compose/emo-present/emo-text。**素**。
- **`areka-P0-test-cage-determinism`（W6.9）**: brief は「`areka-emo-present/src/scale.rs` の `mod tests` を共有」と記すが、本仕様が触る可能性があるのは **`areka-emo-compose/src/scale.rs`／`scale_ratio_tests.rs`**（別 crate・別ファイル）。**現時点では素**。ただし cage は 12 モジュールの毒化インベントリを持つため、cage 着手時（W6.9・本仕様の後）に再突合すること。
- **`areka-P0-balloon-offset-dpi`（W6.75）**: 因果依存のみ（丸め権威の前提）。コードは素。
- **`areka-P0-emo2-conformance-e2e`（W7）**: 本仕様の着地で適合 #1 の DPI 判定が絶対値で書けるようになる（下流保証）。

---

## 6. Research Needed（設計フェーズへ持ち越す調査項目）

1. **k=6/5 の 81 件を実コードで再現する最小手順**（要件 1.2 の受け入れ基準そのもの）。既存 `physical_extent` に対して 1..1200 を回す一時テストで赤を確定させ、その出力（誤り件数と代表値 v=25→31）を記録する。**是正前に採取しないと R5.1 の「是正前は赤」が主張できない**。
2. **`ImagePx`（f32）から整数寸への変換規約**。本番の入力は常に整数値（`resolve_coord` が `i32`／`u32` 由来の f32 しか作らない＝`region.rs:246-248`）だが、型は任意の f32 を許す。厳密算術に渡す前の正規化（`ceil` するか・`round` か・整数化を型で強制するか）を決める必要がある（→ 判断 #2）。
3. **`ScaleRatio` の u32 上限での ceil の厳密性検証**。`scale_len` は u128 中間＋u32 飽和で解決済み。ceil 版も同じ域で溢れないこと・飽和規約を同一にすることを設計で確定する。
4. **変異キルの実測本数**（R5.3）。既存様式（`scale_ratio_tests.rs:305-328`）に倣い、①切り上げ→切り捨て ②num/den 取り違え ③f32 経由への差し戻し の 3 変異それぞれで「何本落ちるか」を実測し計測日付きで記録する。**排他キルが取れない変異があった場合の扱い**（先例は「排他的キルは持たない」と正直に記載）も踏襲候補。
5. **`viewbox.rs:734` のガード余白**を今回の是正から除外する根拠の明文化（要件は Out of scope と定めているが、doc に「なぜ ceil の f32 が許されるか＝保守的に大きい側へ倒れるだけで表示欠けを生まない」を残すか）。

---

## 7. 設計判断事項（要件ディスカッションへ供する論点）

> いずれも**本ギャップ分析では決めない**。設計フェーズ／要件ディスカッションでの裁定を求める。

1. **切り上げ権威の置き場所**: `ScaleRatio` に `scale_len_ceil` 相当を新設して丸めを `scale.rs` へ完全集約するか（案 I）、`ratio()` アクセサだけ生やして算術は `region.rs` に置くか（案 II）。前者は `scale.rs` の丸め規約を 2 → 3 系統へ増やし、後者は丸めの実装が権威ファイルの外に 1 つ生まれる。R3.4「丸め権威は有理スケール単独」の解釈が分かれる点。
2. **`physical_extent` の入力型**: `ImagePx`（f32）のままにして内部で整数化するか、整数寸（u32）を受ける署名へ変えるか。R1.4 が「恒等なら**入力の寸を切り上げた値**」と定めるため、分数入力の意味論を確定する必要がある（`ceil(v)` してから ×k と、v×k してから `ceil` は num>1 の分数 v で結果が異なる。本番の v は常に整数値ゆえ実害はないが、契約としてどちらを正典にするか）。
3. **R2.5（num/den が 0 → warn＋恒等）の扱い**: `ScaleRatio` を型で搬送すると 0 は表現不能になり、この分岐は**到達不能**になる。(a) 生の `(u32,u32)` を受けて分岐を生かす（B-2）／(b) 型で構造排除し、要件 2.5 は「構造的に充足」と読み替えて檻を置かない（[[test-only-decision-branches-not-proven-wiring]]・[[areka-log-cage-harness-blindspots]] の趣旨に沿う）——どちらを採るかで requirements の読み方が変わる。**要件文の再確認が要る論点**。
4. **`region.rs`（純粋層）が `ScaleRatio` を import する経路**: `areka_emo_present::ScaleRatio`（再輸出・Cargo.toml 無改変）か、`areka-emo-compose` を本番依存へ昇格して直接 import するか。後者は依存が 1 本増える（ワークスペース内 path 依存であり「新規外部依存」ではない）が、純粋層の宣言（`region.rs:6`）と整合しやすい。
5. **`ScaleContract.scale`（`pub scale: f32`）の残置可否**: 連続量 7 消費点の無改修を取る（C-1）か、メソッド化して誤用を目立たせる（C-2）か。R3.5 は f32 の継続使用を許しているため残置は適法だが、「寸法に使うな」を doc でしか守れない状態が残る。
6. **ダーティ矩形ガード余白（`viewbox.rs:734`）の扱い**: 要件は明示的に Out of scope（R3.5／R4.4 で不変を要求）だが、そこも `ceil` を伴う寸法演算である。今回触らない判断を doc へ残すか、無言で据え置くか。
7. **着手順序（R5.1 の「是正前は赤」の証跡）**: k=6/5 の全数テストを**先に書いて赤を記録**してから是正する手順を tasks へ明示的に組むか。記録の形式（テスト doc への計測日付き記載か、`verification/` 配下のログか）も併せて確定が要る。
8. **陳腐化テストの処遇**: `region.rs:677-683` `invalid_scale_falls_back_to_one_with_warn` を (a) 整数入力（0/0 等）へ更新して延命（B-2 の場合）／(b) 退役して削除（B-1 の場合）。[[obsolete-vs-broken-test-policy]] に従い判断根拠を残す。
9. **`TextSlotBinding` の churn キー意味論**: `ratio` を持つと `PartialEq` が有理数の厳密一致になり、f32 で同値だった 2 つの k（実際には起こらないが理論上）が別物と判定され得る。`actor.rs:376-378` の doc（「f32 は比較にのみ使う」）の書き換え範囲を確定する。
10. **`presenter/read.rs` の doc 更新範囲**: `physical_size`（:52-69）と `applied_scale`（:138-160）の doc は「f32 は寸法に使うな」を長文で警告している。文字層が有理を受けるようになった後の記述（警告は残すのか、参照先を新公開面へ差し替えるのか）を設計で確定する。**[[revise-design-not-just-requirements]]・[[doc-claims-need-file-line-verification]] の適用対象**。

---

## 8. 設計フェーズへの推奨

- **推奨の出発点は案 I（A-1/A-4 ＋ B-1 ＋ C-1）**。理由: (1) 依存グラフに手を入れずに済む（`lib.rs:62` の再輸出が既にある）、(2) 丸めが `scale.rs` 1 ファイルに閉じ、変異キル檻を既存様式の隣へ置ける、(3) 差分が本番 3 箇所＋機械追随に収まる。ただし**判断 #1／#3 の裁定が案 I の前提**であり、#3 が「分岐を生かす」裁定になれば案 II へ倒れる。
- **設計で最初に固めるべきは判断 #2（入力型）と #3（0 の扱い）**。この 2 つが決まると、残りの署名・doc・檻の形が機械的に定まる。
- **バイト同一（R4.1/R4.2）の確認手順を設計に書くこと**: 「既存テストが 1 本も色を変えない」は受け入れの下限であり、追随作業の変換ミス（例: `ScaleContract::new(0.8, …)` → 誤って `ScaleRatio::new(8,10)` ではなく `(4,5)` 以外を書く）は静かに緑を保つ恐れがある。k≠1 の檻 10 箇所前後を一覧化して人手照合する段を設けるのが安全。
- **[[parallel-worktree-brief-staleness-rebase-before-design]] の適用**: 本ドキュメント §0 のアンカーは 2026-08-14 実測。design 生成の直前に `origin/main` へ再突合し、W6.5 並走 2 本（budget＝`show.rs`／wpl＝`placement/`）が着地していれば差分を再確認すること。

---

## 9. 裁定（2026-08-14・要件ディスカッション）

### 9.1 決定

**厳密化を却下する。拡大率は f32 の 1 変数のまま引き回す。**

本仕様は「有理数を文字層へ配管して供給面寸を厳密化する」実装仕様から、**「許容の裁定を登記し、その前提をテストで固定する」文書・検証仕様**へ縮小された。実行時の挙動は一切変更しない。

### 9.2 根拠となった実測（2026-08-14・本ディスカッション中に採取）

到達し得る拡大率を総当たりし、1..1200 の全寸で f32 経路の結果と厳密な `div_ceil` を突合した。作者 DPI ∈ {72, 96, 120, 144} × モニタ DPI ∈ {96, 120, 144, 168, 192, 216, 240, 288} の全組合せを約分し、**重複を除いた 23 比**が対象。

| 比 | 誤り件数 / 1200 | 初出 | 到達 DPI 例 |
|---|---|---|---|
| **12/5** | **81** | v=25 → 61（正 60） | 288:120（作者 120・300%） |
| **6/5** | **81** | v=25 → 31（正 30） | 144:120（作者 120・150%） |
| 他 **21 比** | **0** | — | 4/3・5/4・8/5・7/5・9/5・4/5・5/6・7/6・3/2・5/2・7/4・9/4・5/3・7/3・8/3・10/3・1/1・2/1・3/1・4/1・2/3 |

12/5 = 2 × 6/5 で **f32 の仮数が 1.2 と同一**のため、両者は同一の失敗の倍尺である。すなわち**欠陥の正体は「1.2 の f32 表現」一点**に帰着する。

> **道具の較正記録**（[[subagent-tooling-can-be-wrong-calibrate-it]]）: 初回の集計スクリプトは PowerShell の `/` が整数除算でないことに起因して真値側を誤り、`5/4`（f32 厳密表現可能）にまで誤りを報告した。既知の答え（6/5 → 81 件・v=25 → 31／4/3 → 0 件）を逐語再現できることを確認してから本表を採取している。

### 9.3 裁定の4つの理由

1. **誤差の向きは常に +1 側のみ**。真の積が整数のときだけ振れる。整数でないときは整数までの距離が最低 `1/den` あり（約分後の分母は小さい）、f32 の相対誤差 ~1e-7 では跨げない。**文字が切れる方向には構造的に転ばない**。
2. **不可視**。レイアウトは画像空間で決まり（`TextRegion::resolve` は k を受けない）、窓寸は別の丸め権威が決めるため双方とも汚染されない。供給面の生成は初回解決時の 1 回きり（`actor.rs:661`）でフレーム毎の負荷にもならない。`region.rs:111-112` の既存登記も「可視の不具合ではない」と認めている。
3. **救える範囲が極小**（§9.2）。
4. **費用が見合わない**。112 箇所の署名追随（本番 3・テスト 109・20 ファイル超）・工数 M・追随の変換ミスが緑のまま通るリスク中。

### 9.4 失効した設計判断事項（§7）の処遇

| # | 論点 | 処遇 |
|---|---|---|
| 1 | 切り上げ権威の置き場所 | **失効**（切り上げを移動しないため） |
| 2 | `physical_extent` の入力型 | **失効**（署名を変更しないため） |
| 3 | num/den が 0 の縮退 | **失効**（有理数を搬送しないため。既存の f32 縮退 `region.rs:73-81` がそのまま生きる） |
| 4 | 純粋層の import 経路 | **失効**（`ScaleRatio` を文字層へ持ち込まないため） |
| 5 | `pub scale: f32` の残置可否 | **失効**（残置で確定） |
| 6 | ダーティ矩形ガード余白の扱い | **失効**（本仕様が f32 の寸法演算を一律に許容裁定したため、個別の除外理由づけが不要になった） |
| 7 | 「是正前は赤」の証跡採取手順 | **失効**（是正しないため。代わりに R3.4 が「誤差が出ること自体」を期待値として固定する） |
| 8 | 陳腐化テストの処遇 | **失効**（`region.rs:677-683` は構築口が変わらないため陳腐化しない。存続） |
| 9 | churn キー意味論 | **失効**（`TextSlotBinding` を変更しないため） |
| 10 | `presenter/read.rs` の doc 更新範囲 | **形を変えて存続** → 新 R2.3（例外の所在と裁定の参照先を示す）へ移行 |

### 9.5 新スコープの実装対象（§1 の実測アンカーを流用）

| 新要件 | 対象 | 実測アンカー |
|---|---|---|
| R1（登記の書き換え） | 供給面寸導出の既知欠陥登記 | `crates/areka-emo-text/src/region.rs:98-118` |
| R2.1（数学層の宣言に例外） | 「浮動小数を一切持ち込まない」宣言 | `crates/areka-emo-compose/src/scale.rs:1-25` |
| R2.2（照会面の宣言に例外） | 「寸法・画素演算に使ってはならない」 | `crates/areka-emo-compose/src/scale.rs:139-146` |
| R2.3（提示段の警告に例外） | `physical_size`／`applied_scale` の doc | `crates/areka-emo-present/src/presenter/read.rs:52-69`／`:138-160` |
| R3（前提の檻） | 供給面寸の性質固定 | 新規（`region.rs` の in-crate 檻または `tests/scale_invariance_test.rs` の隣） |
| R5（非回帰） | 製品コードの式を変更しない | 差分は doc とテストのみ |

**規模**: S（署名追随ゼロ・製品コードの式の変更ゼロ）。当初見積り M から縮小。

### 9.6 却下した代替案（記録）

- **積が整数に十分近ければ整数へ吸着させる 3 行**（署名追随ゼロで厳密化できる）: 却下。「約分後の分母は小さい」という暗黙の前提に寄りかかり、失敗時の挙動が読みにくい（[[canonical-not-minimal-lifecycle]]）。
- **f64 での計算**: 却下（§3.1 A-3 の理由に加え、そもそも厳密化自体が却下された）。

---

## 10. 設計フェーズ記録（2026-08-14・design.md 生成）

### 10.1 ディスカバリ種別と再突合

- **種別: minimal**（文書＋検証仕様・新規機構ゼロ・外部依存ゼロ）。§0/§1/§9 が実測済みディスカバリの正本であり、design 生成時に全アンカーをワークツリー現物で**再照合して一致を確認**した:
  - `region.rs:98-118`（登記）・`:119-121`（式）——一致。見出しは「未是正・担当 spec 不在・開発者裁定待ち」のまま（R1 の書き換え対象確認）。
  - `scale.rs:1-25`（module doc）・`:139-146`（`as_f32` doc）・`:147-149`（式）——一致。
  - `read.rs:52-69`（`physical_size` doc）・`:109`（`as_f32()` 汚染点）・`:138-160`（`applied_scale` doc）——一致。`target_physical_size` doc（`:113-130`）の窓寸復元禁止は別対象として据え置きを設計で明示。
  - `areka-emo-text/Cargo.toml:58`——`areka-emo-compose` は **dev-dependency 既存**＝新規テストは Cargo.toml 無改変で `ScaleRatio` に到達できる。
  - 下流 brief: `emo2-conformance-e2e/brief.md:14-15`（「絶対値で書ける前提」＝裁定で失効する記述を確認）・`balloon-offset-dpi/brief.md:31,50`（「ScaleRatio 配管を前提」＝同）。
  - `roadmap.md:66`（ゴール表）・`:84`（W6.5 行）・`:95`（因果 exact→bod）——R4.3 の改訂対象を確認。

### 10.2 統合（synthesis）の決定（design.md Key Decisions D1〜D7 の根拠）

- **D1（本番同一経路の檻）**: テストが `num as f32 / den as f32` を再実装すると「式の写し」を檻うだけになり、`as_f32` の式変更を検出できない。dev-dep 既存の `ScaleRatio::as_f32` → `ScaleContract::new` → `physical_extent` を貫通させる。
- **D2（整数オラクル）**: 真値側に浮動小数を使うと検証が循環する（§9.2 の較正記録が示した道具リスクの回避）。u64 `div_ceil` で v≤1200・num≤288 域は桁溢れ不能。
- **D3（比集合の格子導出＋要素数 23 の固定）**: 裁定実測の前提集合そのものを期待値にする。6/5・12/5 の包含 assert で導出ヘルパを較正する。
- **D4（誤り件数 81/81/0×21 の期待値固定）**: R3.4「是正しない判断を明示的に固定」の直訳。件数が割れたら裁定再審のトリガ（design Revalidation Triggers）。
- **D5（独立テストファイル）**: `tests/physical_extent_arbitration_test.rs` 新設。既存 `scale_invariance_test.rs`（レイアウト k 非依存の檻）と関心を混ぜない。
- **D6（登記は追記中心）**: 2026-07-30 表を保持（R1.3/R1.5）し、見出し差し替え＋2026-08-14 実測・4 根拠・spec 名出典を加える。参照は完了後の `completed/` 移動に耐えるよう **spec 名**で行う。
- **D7（申し送りは brief 追記が正本）**: roadmap.md:78 の規律（roadmap は編成と条件のみ）に従い、roadmap 側は :66/:84/:95 の最小改訂に留める。W6.5 の着地スロット自体は維持（縮小後も登記・テスト・申し送りが着地物）。

### 10.3 設計レビューゲート結果

- **機械チェック**: 要件 ID 26 個（1.1〜5.5）全てが traceability 表に存在／Boundary 4 節・File Structure Plan とも具体値で充足／全コンポーネント（C1〜C4）がファイル計画へ対応——**PASS**。
- **判定レビュー**: 有理配管・アクセサ新設・署名変更の再導入なし（Non-Goals で明示）／製品コードの式への接触ゼロ（Out of Boundary で禁止）／要件矛盾・ギャップの検出なし——**PASS**（修復 1 回: 誤植修正のみ）。
