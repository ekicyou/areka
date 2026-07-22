# ギャップ分析（research.md） — areka-P0-mayuna-compose

> 対象: 確定済み requirements.md（R1〜R8）／brief.md。基点 HEAD=`13e24c7d`（W1 `sakura-dialogue-tags` マージ後）。
> 手法: 静的読取（Grep/Glob/Read）＋ukadoc MCP 正典参照。ワークスペースビルドは行わない（vendors/pasta 未populate・gap分析は読取専用）。
> 性格: 情報提供と選択肢の提示（最終決定はしない）。file:line は HEAD 実測で brief の陳腐化引用を補正済み。

## 1. 分析サマリ（3〜5点）

- **配管は4層のうち3層が既に完成**。①W1 が `\![bind]` を parser→compile→dola まで汎用キャリアで貫通済み（`GenericCommand`→`command_carrier("bind", raw_args)`→`Custom{command:"bind", params:Array[String…]}`）。②seriko は per-scope 状態機械・冪等ガード・単一発行点・balloon 早期分岐という消費テンプレートを完備。③emo-present `ComposeCache` は `(surface_id, BindSet)` キーで bind 差分にミス→再合成する回帰檻を既に持つ。**本 spec の実質新規は 2 点のみ**＝(a) parsers の名前解決基盤（`.name,カテゴリ,パーツ`→bindgroup ID）、(b) seriko の動的 bind 状態＋`bind` コマンド消費者結線。
- **唯一の真の新規設計は parsers の名前解決表**。現行 `package/resolve.rs` は `sakura.bindgroup*.default,1` の**番号のみ**転記（`BindGroupDefaults`）で、`sakura.bindgroup*.name,カテゴリ,パーツ` は**どこも読んでいない**。`parse_kv` は最初のカンマだけで分割するため `.name` の値は `"腕,伸び"`（＋任意の第3フィールド=サムネイル名）という**多カンマ文字列のまま**得られ、下流で (カテゴリ, パーツ, [サムネ]) へ再分割が要る。
- **seriko の消費結線は `cue_target_of` を素通りする Custom を名前で捕捉する経路が要る**。現行 `handle_message` は `cue_target_of` のみで分類し、`Custom`（bind の器）は `None`→良性 debug skip へ落ちる。bind を拾うには `command_target_of("bind")→Shell` を dola 単一権威表へ1行追記し、seriko 側に Custom キャリアを `as_command_carrier()` で開いてコマンド名選別する早期分岐（balloon 早期分岐と同型）を新設する。
- **表示反映は「新 BindSet を載せた Show 再発行」だけで成立**。`DisplayCommand::Show{scope, surface_id, binds}` は既に binds を運び、adapter `map_display_command`（`crates/areka/src/emo2_boot/adapter.rs:37`）が `PresentCommand::ShowSurface{binds}` へ写し、ComposeCache が bind 差分にミスして再合成する。新しい合成メソッド・新 cue variant・新 DisplayCommand variant は**いずれも不要**（additive・R8.4）。
- **正典は名前形一本**。ukadoc の script タグは `\![bind,カテゴリ名,パーツ名,数値]` のみで、**「番号直指定形」はさくらスクリプトとしては文書化されていない**（→要件ディスカッション #1 で確定・旧 R4.2 削除済み・下記 DD-6【解決済み】）。emo2 fixture も名前形＋明示 on/off が実測。M1 の実導出はこの形に限定し、カテゴリ単位・トグル・addid・mustselect・kero 側は縮退＋シーム（R4.2/4.3）。

## 2. 現状調査（資産の実測・file:line は HEAD）

### ②parsers（名前解決の起点）
- `crates/areka-parsers/src/package/model.rs:47-54` — `BindGroupDefaults{ sakura_default_on:Vec<u32>, kero_default_on:Vec<u32> }`。`MountModel.bindgroups`（同 :37）に同居。**番号（default=1）のみ**保持・名前フィールドなし。`#[non_exhaustive]` ゆえフィールド追加は後方互換。
- `crates/areka-parsers/src/package/resolve.rs:122-153` — `read_bindgroup_defaults`／`parse_bindgroup_id`。`<prefix>NNNN.default` の値が `"1"` のときだけ `NNNN` を収集。**`.name` は未読取**。prefix 定数 `sakura.bindgroup`/`kero.bindgroup`、suffix `.default` は既存（:108-111）。
- `crates/areka-parsers/src/kv/parse.rs:21-44` — `parse_kv` は `split_once(',')` で**最初のカンマのみ分割**。`sakura.bindgroup1100.name,腕,伸び` → key=`sakura.bindgroup1100.name`, value=`腕,伸び`（後続カンマ保持）。後勝ち・trim・BTreeMap（キー昇順・決定的）。
- `crates/areka-parsers/src/sakura/decode.rs:315-320` — `\![bind,…]` は `decode_passthrough_bang`→`GenericCommand{name:"bind", raw_args:["腕","伸び","1"]}`（第1引数=name, 残り=raw_args）。転記は成功（fixture 6連は `validation_tests.rs` が固定）。
- **実 fixture 実測**（`crates/pilot/examples/shiori-host-32/fixtures/emo2/shell/master/descript.txt`）: `sakura.bindgroup1100.name,腕,伸び` / `.default,1`、`…1207.name,口,にこっ`/`.default,1`、`…1302.name,目,通常`/`.default,1`、`…1400.name,まばたき,通常`（**default 無し=既定 OFF**）、`…1500.name,眉,通常`/`.default,1`、`…1800.name,髪飾り,リボン`/`.default,1` 等。全 `.name` は (カテゴリ,パーツ) の2フィールド（サムネ名なし）。**bindgroup 番号=animation ID=BindSet の値**（恒等・下記 seriko bind.rs と一致）。

### ④sakura compile（W1 で貫通済み・brief 追記㉟で補正）
- `crates/areka-sakura/src/compile.rs:171-178` — `GenericCommand{name,raw_args}` → `CueCommand::command_carrier(name, raw_args)`（瞬時 duration 0）。**brief 本文の「catch-all で drop」は失効**。`Move` は :160-167 で `command_carrier("move", …)`。両者同経路。**本 spec は compile を触らない**（W1 が汎用キャリアへ一度だけ実装済み）。
- `crates/dola/src/cue/command.rs:163-166,201-229` — `CueCommand::Custom{command,params}`＋`command_carrier`/`as_command_carrier`（正準形 `params:Array<String>`・空トークン保持・非正準は None→良性スキップ）。`CueCommand` は **10 variant**（brief 追記㉟のドリフト指摘どおり Choice/Cursor/Custom/Wait/ClearAll 増）。**本 spec は新 variant を足さない**（R8.4）。

### dola sink（消費者結線点）
- `crates/dola/src/cue/sink.rs:99-104` — `command_target_of(name)`：**`"move"→Window` の 1 行のみ**。`_ => None`（未知名は良性スキップ）。**「"bind"→Shell」の 1 行追記が本 spec の結線作業**（単一権威表・1名前=高々1消費者）。
- `crates/dola/src/cue/sink.rs:54-76` — `cue_target_of(command)`：`Custom{..}→None`（型レベルは委譲・コメントが `command_target_of` への委譲を明記）。`Wait→None`。網羅 match（catch-all なし）。

### ⑤seriko（動的状態＋消費結線の本丸）
- `crates/areka-seriko/src/state.rs:48-70,182-184` — `ScopeStates{ scopes:HashMap<ActorKey,ScopeState>, balloon:HashMap<…>, static_binds:BindSet }`。`ScopeState=Shown(u32)|Hidden`。`static_binds` は `new()` で一度設定し以後不変、`binds()` は不変参照のみ。**動的切替 API なし**（:44-47 のコメントが `mayuna-compose` による置換を予約）。
- `crates/areka-seriko/src/actor.rs:182-305` — `handle_message`：`cue_target_of` で Shell 選別 → `BalloonSurface` 早期分岐（:234-263）→ `Emote{key}` 解決→`apply`→`emit_display`。**Custom は `None` 枝（:212-229）で良性 debug skip**。bind 消費は**この早期分岐の増設**が定石（balloon と同型・値を返さず arm 内完結）。
- `crates/areka-seriko/src/resolve.rs:103-119` — `resolve_balloon_key(key)->BalloonResolve`（純関数・alias 非適用・数値/`-1`/NameForm/Invalid の型別）。**bind 名前解決の純関数テンプレート**（`resolve_bind_key` 対応）。
- `crates/areka-seriko/src/output.rs:28-42` — `DisplayCommand::Show{scope,surface_id,binds}`（既に binds 運搬）。新 variant 不要。
- `crates/areka-seriko/src/bind.rs:18-20` — `build_static_bindset(&[u32])`：bindgroup 番号=animation ID の**恒等写像**（`BindSet::from_ids`）。
- `crates/areka-seriko/src/actor.rs:157-174` — `spawn_seriko(resolver, static_binds, out)`：構築署名。名前解決表の注入はここへ additive 引数追加が要る（下記 DD-4）。

### ⑥emo-present（回帰のみ・本体無改変）
- `crates/areka-emo-present/src/cache.rs:48-112` — `ComposeCache`：`ComposeKey{surface_id, binds}` 完全一致・容量1メモ。`get` は binds 1要素差でもミス。**`different_binds_on_same_surface_must_miss`（:267-288）が既存**＝R6.1/6.2 は test-only の追加（本体無改変）で足りる可能性が高い。
- `crates/areka/src/emo2_boot/adapter.rs:34-54` — `map_display_command(Show{binds})→PresentCommand::ShowSurface{binds}`。動的 BindSet が提示層まで純貫通。

## 3. ukadoc 正典（bind の意味論・design 冒頭の3分類表の素材）

| 正典キー/タグ | 意味 | M1 取り扱い（要件根拠） |
|---|---|---|
| `sakura/kero.bindgroup*.name,カテゴリ名,パーツ名,サムネイル名` | animation ID *番のパーツにカテゴリ/パーツ名を定義。`\![bind]` 名前操作に**必須**（SSPのみ）。第3=サムネ名は任意 | **実導出**（→②名前解決・R1）。サムネ名は M1 不使用（保持/破棄は DD-2） |
| `sakura/kero.bindgroup*.default,数値` | ID *番を起動時に表示するか（1=表示,0=非表示） | 実装済（`BindGroupDefaults`・初期 on 集合の源・R3.1） |
| `\![bind,カテゴリ名,パーツ名,数値]` | 着せ替え着脱。**1=着衣, 0=脱衣**。**パーツ名空欄=カテゴリ単位**。**数値空欄/省略=ON/OFFトグル**。実行後 OnDressupChanged→OnNotifyDressupInfo | **名前形＋明示 on/off のみ実導出**（R4.1）。カテゴリ単位/トグルは縮退（R4.2）。イベントは範囲外（R4.4） |
| `\![bind-noevent,…]` | bind と同じだが**イベント発生させない** | 範囲外（イベント自体を扱わない・R4.4） |
| `sakura/kero.bindgroup*.addid,ID` | 同時実行（カンマ区切り複数可）。有効化時に addid も同時有効化 | 語彙/構造のみ保持・実挙動なし（R4.3・シーム） |
| `char*.bindoption*.group,カテゴリ名,オプション`（mustselect/multiple） | カテゴリに排他/複数選択オプション | 語彙/構造のみ保持・実挙動なし（R4.3・シーム）。**emo2 実使用あり**（descript:75-78 で 腕/口/眉/目 の4カテゴリに mustselect 宣言・ただしスクリプト側が明示 on/off 連打で自前排他するため実挙動なしでも表示は正しく成立＝design で要注記） |

**正典との突合で判明した要注意点**:
- **番号直指定のスクリプト形は正典に無い**（✅ 要件ディスカッション #1・2026-07-19 で確定済み）。ukadoc の script タグは名前形 `\![bind,カテゴリ名,パーツ名,数値]` のみ（`dev_bind` 全文でも記述例は名前形のみ・数字が現れるのは descript `menuitem*` のメニュー構成だけでスクリプト形でない）。emo2 も名前形一本（menuitem すら不使用）。→ 旧 R4.2（番号直指定）は**削除**・数値に見える入力は不透明文字列として名前解決に回り R3.7（解決不能 log+skip）が自然吸収。
- `.name` の値は **(カテゴリ, パーツ, [サムネ])** の可変長。`parse_kv` が最初のカンマだけ割るので value 側を再分割する層が要る（②名前解決の設計点・DD-2）。
- bindgroup 番号 = **surfaces.txt の `animation*.interval,bind` の `*`** = BindSet 値（恒等）。名前解決の帰結はこの番号を BindSet へ入れるだけ（seriko bind.rs と一貫）。

## 4. 要件→資産マップ（Missing/Constraint/Present）

| 要件 | 必要能力 | 現資産 | ギャップ |
|---|---|---|---|
| R1 名前宣言取込・(カテゴリ,パーツ)→ID 解決 | descript `.name` 読取＋名前解決表を `MountModel` へ | `BindGroupDefaults`（.default のみ）・`parse_kv` | **Missing**（.name 未読取・値再分割・sakura/kero 別・空パーツのカテゴリ集合解決） |
| R2 `bind` を表示系へ配送・消費者結線 | `command_target_of("bind")→Shell`＋emo-text 無視 | W1 汎用キャリア・`command_target_of`（move のみ）・emo-text 網羅 match | **Missing（1行追記）**。emo-text は Custom を既に無視（新規列挙不要の可能性・DD-3 で確認） |
| R3 動的 bind 状態・on/off 積算・Show 再発行・冪等・解決失敗 skip | per-scope 可変 BindSet＋Custom 早期分岐 | `ScopeStates`（静的 binds）・balloon 早期分岐・`apply`冪等・`emit_display` | **Missing**（動的マップ・積算・現 surface_id 保持での Show 再発行・DD-1/DD-5） |
| R4 引数意味論の M1 縮退境界 | 名前形実導出・他形は skip+log/シーム | balloon の NameForm/Invalid severity split 先例 | **Constraint/Missing**（縮退規律の明文化。番号形は DD-6 解決済み＝正典に無く旧 R4.2 削除） |
| R5 決定論 e2e・純関数全網羅 | fixture script 直入力→mock sink・注入 Tick | `MockSurfaceOutput`・`handle_message` 同期呼・`balloon_face_e2e.rs` 先例 | **Present（テンプレ流用）**＋fixture 自前用意 |
| R6 emo-present 再合成回帰 | bind 差分ミス→再合成の test | `ComposeCache`＋`different_binds…must_miss` 既存 | **Present（test-only 追加で足りる）** |
| R7 実機サインオフ | 実 emo2・実 pasta・実 DPI・絶対パス起動 | emo2-boot 実機経路・adapter 貫通 | **Constraint**（本番ゴースト先行・絶対パス） |
| R8 非退行（additive） | 全テスト緑・新依存なし・tokio なし・cue ワイヤ不変 | 既存ワイヤ檻群・新 variant 不要 | **Constraint**（Rust 2024・純関数決定論） |

## 5. 実装アプローチ選択肢

### 名前解決基盤（②parsers・R1）
- **Option A（推奨）— `MountModel.bindgroups` を拡張**: `BindGroupDefaults` に名前解決フィールド（例 `sakura_names: Vec<(String,String,u32)>` または `BTreeMap<(String,String),u32>`＋カテゴリ→id集合）を additive 追加し、`read_bindgroup_defaults` を「`.default` と `.name` を一度に転記」へ拡張。同層・同 I/O 点・既存 `#[non_exhaustive]` で後方互換。
  - ✅ 既存経路踏襲・I/O 点1つ・default と name が同居し整合しやすい ❌ `BindGroupDefaults` の責務がやや拡大。
- **Option B — 独立 `BindNameTable` 型を新設**: 名前解決専用の型を model.rs に追加し `MountModel` へ別フィールド。
  - ✅ 責務分離 ❌ `.default` と `.name` の走査が二重化しがち。
- **転写層原則の遵守**: parsers は「(カテゴリ,パーツ)→番号」の忠実転写のみ。範囲展開・surface 解決はしない（記憶 areka-parser-transcribes-tree-downstream / areka-surface-args-opaque-string-downstream-resolve）。**カテゴリ集合解決（パーツ空欄）を parsers 側で畳むか seriko 側で畳むかは DD-2**。

### bind 消費者結線（dola＋⑤seriko・R2/R3）
- **Option A（推奨・単一権威）**: dola `command_target_of` に `"bind"→Shell` を1行追記＋seriko `handle_message` に Custom キャリア早期分岐を新設（`as_command_carrier()`→name==="bind" 判定→bind 状態更新）。balloon 早期分岐と同型（値を返さず arm 内で解決→積算→発行）。
  - ✅ 単一権威表・1名前=1消費者・balloon 前例に忠実 ❌ seriko が「Custom を名前で開く」新パターンを導入（ただし `command_target_of` を seriko が参照して Shell 判定→name 分岐、が自然）。
- **Option B — seriko が command_target_of を参照せず自前で name 選別**: seriko 内で `Custom{command:"bind"}` を直接 match。
  - ✅ dola 無改修 ❌ 単一権威表を迂回（brief の「同表に登記」方針に反する）。→ 非推奨。
- **設計論点**: seriko の分類順序。現在 `cue_target_of(Custom)=None`→skip。Custom を拾うには (i) handle_message 冒頭で Custom を先取りして `command_target_of` を引くか、(ii) `None` 枝の中で Custom を `command_target_of` 委譲へ回すか。balloon が「cue_target_of=Shell の後の早期分岐」なのに対し、bind は「cue_target_of=None だが command_target_of=Shell」ゆえ**分岐位置が balloon と対称でない**点に注意（DD-1）。

### 動的 bind 状態（⑤seriko・R3）
- **Option A（推奨）— `ScopeStates` に per-scope 動的 BindSet マップを追加**: `static_binds` を初期値に per-scope（sakura/kero スコープ別）の可変 bind 集合を持ち、on/off を積算。予約シーム（state.rs:44-47）どおり `static_binds` の置き場を動的マップへ差替。bind 変化時、当該 scope の現 `Shown(surface_id)` を引き、新 BindSet で `Show{surface_id, binds}` を再発行（Hidden なら発行しない or 状態のみ更新＝DD-5）。冪等ガード=結果 BindSet が直前と同一なら再発行しない。
- **Option B — 別構造体 `DynamicBinds` を新設し ScopeStates と協調**: 状態を分離。
  - ✅ ScopeStates 肥大回避 ❌ surface_id と binds の一貫性（同一 Show へ載せる）で2構造の同期が要る。
- **積算とスコープ写像**: bindgroup default は sakura/kero 名前空間別。ActorKey（"0"=sakura,"1"=kero 相当）と bindgroup namespace の対応付けが要る（DD-4/DD-7）。純関数化（名前解決＋on/off 積算）で GPU 不要全網羅（R5.4）。

### emo-present 回帰（⑥・R6）
- **Option A（推奨）— test-only 追加**: 既存 `different_binds_on_same_surface_must_miss` を土台に、「動的 Show 再発行→ミス→再合成、同一 binds→ヒット」を明示する test を追加（本体無改変・R6.3）。balloon-face-cue と同様 test-only。

## 6. 工数・リスク

| 単位 | 工数 | リスク | 一言根拠 |
|---|---|---|---|
| ②名前解決基盤 | S–M | Low–Med | 新規だが既存 I/O 点拡張＋`parse_kv` 値再分割のみ。カテゴリ集合/サムネ/スコープ写像の設計判断が Med 寄り |
| dola 結線（1行） | S | Low | 単一権威表へ1行＋ワイヤ不変 |
| ⑤動的 bind 状態＋消費結線 | M | Med | 予約シーム有・balloon 前例有だが「Custom を name で拾う分岐位置」「現 surface_id 保持での Show 再発行」「スコープ写像」が新判断 |
| ⑥emo-present 回帰 | S | Low | 既存キャッシュ檻の延長・本体無改変 |
| 決定論 e2e＋fixture | S–M | Low | `balloon_face_e2e.rs`・`MockSurfaceOutput` 流用・fixture 自前 |
| 実機サインオフ | S | Med | 実機・DPI・絶対パス起動の運用リスク（コード外） |
| **全体** | **M（3〜7日）** | **Med** | 3層完成済みゆえ実装面は薄いが、名前解決とスコープ/縮退の設計判断が中核 |

## 7. 設計判断項目（要件ディスカッションへ送る・番号付き）

1. **DD-1 bind 消費の分岐位置**: seriko `handle_message` で Custom（`cue_target_of=None`）を拾う位置。balloon は `cue_target_of=Shell`後の早期分岐だが、bind は `command_target_of=Shell` 委譲ゆえ非対称。冒頭先取り vs `None`枝内委譲、どちらを採るか。
2. **DD-2 名前解決表の畳み方と所在**: `.name` 値の再分割（カテゴリ,パーツ,[サムネ]）をどこで行うか（parsers 転写層で (カテゴリ,パーツ)→番号 の表まで、カテゴリ集合解決とサムネ破棄は seriko 消費側、が転写層原則に整合か）。表の型（`BTreeMap<(String,String),u32>` ＋カテゴリ→Vec<u32>）。
3. **DD-3 emo-text 側の bind 無視**: emo-text `apply_cue` は Custom を既に無視しているか（新規無視列挙が不要か）。要 `crates/areka-emo-text/src/state.rs` の Custom アーム確認（W1 で一度実装済みの見込み）。誤配線防止 R2.3 の充足経路を明示。
4. **DD-4 名前解決表の seriko への注入**: `spawn_seriko` 署名に名前解決表を additive 追加（`SurfaceResolver` と同様の所有スナップショット注入）。ghost-setup 側の結線点。
5. **DD-5 Show 再発行時の surface_id と Hidden の扱い**: bind 変化時、現 `Shown(id)` を引いて `Show{id, new_binds}` 再発行。当該 scope が Hidden のとき（未表示）に bind だけ変えた場合の挙動（状態のみ更新し発行しない／次 Show まで保留）。
6. **DD-6 番号直指定形の正典整合**【✅ 解決済み・要件ディスカッション #1（2026-07-19）】: ukadoc・emo2 実データ・開発者知見の三者一致で「スクリプト形は名前キー一本・番号直指定形は正典に存在しない」と確定。旧 R4.2 を削除（R4.3〜4.5 繰り上げ）。番号形のシームは**設けない**（正典外語彙の捏造を避ける）。数値に見えるカテゴリ名/パーツ名は不透明文字列として通常の名前解決に回り、宣言なしなら R3.7（解決不能 log+skip）が吸収する。付随確定: emo2 surfaces.txt で bindgroup 番号=animation ID の恒等が実データで成立（RN-3 の非空虚性を先行確認・1100↔1100 等）・まばたき1400-1402 は `interval,bind+random,4`（seriko-loop 領分・境界整合OK）。
7. **DD-7 scope→sakura/kero 名前空間の写像**: bindgroup default/name は sakura/kero 別。per-scope 動的 bind の ActorKey（"0"/"1"…）を sakura/kero どちらの名前解決表・初期集合へ結ぶか。kero 側は M1 実挙動なし（R4.4）だが取込までは行う（R1.2）ゆえ、写像規約を明文化。
8. **DD-8 縮退の具体形（R4.3/4.4）**: カテゴリ単位（パーツ空）・トグル（数値空）・addid・mustselect を「log+skip」か「不透明保持のシームのみ」か。balloon の NameForm=warn/Invalid=error の severity split を踏襲するか。数の正の assert（優しい縮退の非空虚化）を添える方針（記憶 defer-canon-with-full-vocabulary…）。
9. **DD-9 冪等ガードの単位**: 結果 BindSet が直前と同一なら再発行しない、を per-scope で。on→on の重複 bind、off→off、順序違いで同一集合に至る列（BindSet は昇順dedup）での冪等を純関数檻で固定。

## 8. Research Needed（design フェーズへ持ち越し）— 全件解決済み（2026-07-22 design 生成時実測）

- **RN-1【✅解決】** emo-text `apply_cue` の Custom アーム実測: `crates/areka-emo-text/src/state.rs:260-266` で `CueCommand::Custom { .. }` は明示列挙の良性 `debug!` skip 済み（W1 実装済み）。**emo-text は無改変**（R2.3 は既存資産で充足・design D3）。
- **RN-2【✅解決】** `spawn_seriko` 呼出点: 本番 `crates/areka/src/emo2_boot/mod.rs:287`（`wire_emo2_boot` 手順4）と `crates/areka/src/emo2_boot/spine.rs:444`。名前解決表の供給源は `build_boot_assets`（`crates/areka/src/emo2_boot/assets.rs`）が既に呼ぶ `package::resolve()` の `MountModel.bindgroups` — ここへ名前転記を増設し `BootAssets` 新フィールド `bind_resolver` 経由で注入する（design D4）。**付随発見**: 現行の static_binds は `default_bind_ids`（assets.rs:48-72・shell KV 直読・sakura 限定）由来で、`MountModel.bindgroups`（`BindGroupDefaults`）とは**供給源が二重化**している（既知の技術的負債・本仕様では統合しない＝additive 優先・design「areka — 起動配線」に登記）。
- **RN-3【✅解決済み・再掲】** bindgroup 番号=animation ID の恒等は emo2 実データで成立（1100-1801）。design の 3 分類表「付随確定」に再掲済み。
- **RN-4【✅解決】** `crates/areka-seriko/tests/balloon_face_e2e.rs` のハーネス（script 直入力→Tick 注入→TalkDone→SerikoSink drop→seriko join→records 照合・sleep ゼロ）は bind e2e へそのまま流用可。差分は (a) `spawn_seriko` へ test-local `BindResolver` 表（腕/伸び→1100 等）を直接注入（descript 読取不要＝R5.5 の fixture は parsers テスト側で tempdir 自前用意）、(b) `static_binds` に emo2 相当の defaults を渡し off→on の差分を観測可能にする、の 2 点（design Testing Strategy E2E）。

## 9. 設計決定（design 生成・2026-07-22 確定）

design.md「Design Decisions」D1〜D10 が正本。要点のみ:

- **D1（DD-1）**: bind 消費は `handle_message` の `cue_target_of==None` 枝**内側**（Wait 判定前）で `as_command_carrier()`→`command_target_of(name)==Some(Shell) && name=="bind"` の 2 条件ゲート（`MoveCueSink`（`move_cue.rs:467-484`）前例・冒頭先取り案は分類順序を組み替えるため棄却）。
- **D2（DD-2）**: `.name` 値の再分割（`splitn(3,',')`）と (カテゴリ,パーツ)→ID 問い合わせアクセサは **parsers**（`BindGroupDefaults` へ additive・R1.3/1.4 の主語がマウントモデルのため）。カテゴリ集合解決アクセサ `category_ids` は提供するが M1 の seriko は未消費（縮退・シーム）。重複宣言はキー昇順の後勝ち。
- **D4（DD-4）**: `spawn_seriko` へ `BindResolver` を additive 引数追加。`BindResolver` は素データ構築（seriko→parsers 依存を作らない・`SurfaceResolver` 同型）。追随更新: areka mod.rs/spine.rs＋seriko テスト群（`BindResolver::empty()`）。
- **D5（DD-5）**: bind 変化時、`Shown(id)` なら `Show{id, 新binds}` 再発行・**`Hidden`/未知 scope は状態のみ更新し発行しない**（次 Show へ保留）。既存 `apply` の Show binds は static 固定→per-scope 現在集合へ差替（bind cue 不在時は同値＝非退行）。**R3.5 の設計上の精緻化**: R3.5 は「表示中 scope への再発行」として充足（Hidden scope には載せる surface_id が存在せず、強制表示は `\s[-1]` 意味論を破る）。要件ギャップではなく前提（表示中）の明文化として本 research に記録。
- **D7（DD-7）**: scope→名前空間写像は `"0"→sakura`・`"1"→kero`・他→写像なし（warn+skip）。kero は取込まで行い機構は scope 非依存＝emo2 では空表で自然に解決不能（R4.3 のシームを空表の同一機構で実現・人工的無効化コードなし）。
- **D8（DD-8）**: severity split＝解決不能=`error!`／M1 縮退の正典形（トグル・カテゴリ単位）=`warn!`／破損（カテゴリ欠落・on/off 値不正）=`error!`／非正準 params・未登記名=`debug!`（balloon NameForm/Invalid・MoveCueSink 前例踏襲）。全縮退枝に正カウント assert（非空虚化）。
- **D9（DD-9）**: 冪等単位は per-scope の**結果 BindSet 同値**（`BindSet` 昇順 dedup ゆえ順序・重複不感）。純関数 `accumulate` で檻化。
- **D10（新規発見）**: `command_target_of` へ `"bind"→Shell` 追記で dola 既存檻 `command_target_of_maps_move_and_rejects_unknown_names`（`dola/tests/cue/sink_test.rs:250-282`）の「bind は未知名」「Some は move のみ」assert が意図どおり FAIL→**檻を {move, bind} へ意味ある更新**（obsolete-vs-broken 規律・partition 檻は維持強化）。

### 統合（synthesis）記録

- **一般化**: bind 名前解決は `SurfaceResolver`／`resolve_balloon_key` と同じ「所有スナップショット＋純関数解決・severity は呼び手」の族へ一般化（インターフェースの族一致・実装は M1 要件分のみ）。
- **build vs adopt**: 全構成要素が既存パターンの再演（W1 キャリア・MoveCueSink 名前ゲート・balloon 早期分岐・ScopeStates 冪等・ComposeCache 檻）。新規発明は `BindDirective` 類別と `dynamic_binds` マップのみ。外部依存追加なし。
- **単純化**: 番号直指定形シームを設けない（D6・正典外）。kero 無効化の専用コードを書かない（D7・空表で自然縮退）。emo-compose/emo-present/sakura compile/emo-text へ触れない。`BindApplyOutcome` は 3 値（Changed/StateOnly/Unchanged）で発行要否と保留を型で峻別。

### design レビューゲート結果（2026-07-22）

- 機械チェック: 全 38 要件 ID（1.1–8.4）が Traceability に存在・Boundary 4 節充足・File Structure Plan 具体・コンポーネント⇔ファイル対応に孤児なし。
- 判定レビュー: R3.5 の Hidden scope 非適用域を D5 で設計精緻化（上記）。要件ギャップなし・1 パスで通過。
