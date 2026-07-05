# ギャップ分析: areka-P0-seriko-engine

> 実施日: 2026-07-05 ／ 対象: 確定済み requirements.md（R1〜R7）＋ brief.md ＋ steering。
> 目的: 確定要件と既存コードベースの差分を洗い出し、実装戦略の判断材料（複数案・研究項目・上流契約ドリフト）を提示する。
> 注意: 本書は情報提供であり最終決定ではない。設計判断は要件ディスカッションで詰める。

---

## 0. 上流シンボル実在確認（要件が名指す契約の突合）

要件・brief が名指した上流シンボルを実コードで検証した。**結論: 大半は実在・一致するが、要件が前提とする一部の想定にドリフトがある**（下記 §5 の設計判断へ送る）。

| 契約シンボル | 実在 | 実体 / 場所 | 突合結果 |
| --- | --- | --- | --- |
| `SurfaceSink { fn emit(&mut self, cue: TalkCue) }` | ✅ | `areka_sakura::sink`（`crates/areka-sakura/src/sink.rs:15`）。`emit` は infallible。 | 一致。seriko はこの trait を実装する。 |
| `TalkCue { at: f64, actor: ActorKey, command: CueCommand }` | ✅ | `areka_sakura::contract`（`contract.rs:75`）。`#[derive(Clone, Debug, PartialEq)]`。 | 一致。 |
| `CueCommand::Emote { key: String }` | ✅ | 正本は `dola::cue::command`（`command.rs:124`）。sakura が re-export。 | 一致。ただし **`key` は無加工文字列**（`"10"`/`"通常"`/`"-1"`/`"0,1,foo"`）。数値も alias もこの 1 variant に来る（§5-DD1）。 |
| `CueCommand::EntityRef(u64)` | ✅（型は実在） | `dola::cue::command:128`。**幅は `u64`**（brief は `u32` と記載＝誤り）。 | **ドリフト**: M-boot の sakura コンパイラ（`compile.rs:50`）は `\s[ID]` を **常に `Emote{key}`** へ写す。`EntityRef` は `Entity::to_bits()` 用で、**seriko 入力としては到来しない見込み**（§5-DD1）。 |
| `cue_target_of(command) -> Option<CueTarget>` | ✅ | `contract.rs:92`。`Emote`/`EntityRef`→`Shell`、text 系→`Balloon`、`Custom`→`None`。 | 一致。sakura dispatcher が既に振り分け済みで、seriko の sink には Shell 系のみ届く。 |
| `ActorKey`（"0"/"1"…） | ✅ | `dola::cue` re-export（`contract.rs:106`）。opaque newtype・`as_str()`。 | 一致。scope キーとしてそのまま per-scope マップのキーに使える。 |
| `EmoWorld::build(&Shell)` | ✅ | `crates/areka-emo-compose/src/world.rs:74`。 | 一致。 |
| `AliasMap(BTreeMap<String, Vec<u32>>)` | ✅ | `world.rs:41`。**ただし `EmoWorld` 内の private bevy `Resource`**（外部から直接 move 取得する公開口はない）。 | **要注意**: 消費経路は §1・§5-DD2 参照。 |
| `EmoWorld::resolve_alias(&self, key: &str) -> Option<&[u32]>` | ✅ | `world.rs:121`。未解決は `warn!` ＋ `None`。 | **これが alias 消費の正規口**（AliasMap を直接持たずに済む）。 |
| `BindSet` / `BindSet::from_ids` | ✅ | `crates/areka-emo-compose/src/bind.rs:16,22`。昇順整列＋dedup・`Send`・`Clone`。 | 一致。静的 bind 集合の保持型として直接使える。 |
| `Composer::compose_into(...)` | ✅ | `crates/areka-emo-compose/src/lib.rs:111`。 | 一致（seriko は呼ばない＝emo-present の領分。参考のみ）。 |
| `AtlasTable` | ✅ | 正本は **`areka_emo_atlas`**（brief は「emo-compose の…」と書くが所在は emo-atlas）。 | 参考。seriko は非関与。 |
| `spawn_actor` / `run_inbox` / `ActorHandle` | ✅ | `crates/areka-actor/src/spawn.rs:39,82`。 | 一致。 |
| `Close`（areka-actor の停止プリミティブ） | ❌（**共有型は存在しない**） | areka-actor は `Close` 型/variant を提供しない。各 `XxxMsg` enum が**自前で `Close` variant を定義する規約**（`lib.rs:45`・`SakuraMsg::Close` が実例＝`contract.rs:26`）。 | **ドリフト（用語）**: 要件 R1.3/R1.4/R7 の「停止指令（Close）」は共有型ではなく**規約**。seriko の inbox enum（`SerikoMsg` 級）に `Close` variant を自前定義する必要がある（§5-DD3）。 |
| `Shell` モデル | ✅ | `areka_parsers::shell::Shell`（`crates/areka-parsers/src/shell/model.rs:21`）。 | 一致。ただし §2 の重大な欠落あり。 |

---

## 1. 既存コードベースのパターンと再利用資産

- **アクター規約（areka-actor）**: `spawn_actor::<M,_>(name, body)` で名前付きスレッド起動、body 内で `run_inbox(rx, handler)`。handler 戻り値で `Continue`/`Break(Close)`/`Err(継続)` を固定（`spawn.rs:82`）。停止は「Close variant 受領」と「全 Sender drop（Disconnected）」の 2 経路のみ。**R1.3/R1.4 はこの規約に素直に乗る**。
- **sink 実装の先例**: sakura 自身が `MockSink`（`Arc<Mutex<Vec<TalkCue>>>` 蓄積・`records()` で照合・`Send + 'static`）を持つ（`sink.rs:30`）。**R7 の mock 観測はこの流儀をそのまま踏襲できる**（seriko 側は「emo への発行列」を貯める mock 出力先を自前定義）。
- **alias 消費口**: `EmoWorld::resolve_alias(key)` が `Option<&[u32]>` を返し未解決を `warn!`（`world.rs:121`）。**R2.2/R2.4 の解決＋失敗ログはこの口に委譲できる**（seriko で AliasMap を再定義しない・R2 の「二重定義しない」を満たす）。
- **BindSet**: `from_ids` が整列＋dedup・`Send`。**R4.2 の「emo-compose の `BindSet` として保持」を直接満たす**。
- **ログ規律**: areka 全域で `error!`/`warn!` ＋ Err 戻り、silent failure 禁止（記憶 areka-log-first-no-silent-failure・emo-compose も踏襲）。**R6 に既存規律がそのまま適用**。
- **ワークスペース依存の型**: seriko の Cargo 依存は `areka-sakura`（contract/sink）・`areka-emo-compose`（AliasMap/BindSet/EmoWorld）・`areka-parsers`（Shell）・`areka-actor`・`tracing`。sakura の Cargo.toml（`areka-sakura/Cargo.toml`）と同型の構成で非衝突。

---

## 2. 欠落している能力（要件が要求するが既存に無いもの）— **最重要**

### 2.1 bindgroup default のデータソースが**どのパーサにも存在しない**（R4 の核）

- emo2 実測: **`bindgroupNNNN.default` は `descript.txt`（shell/master）に在り**、`sakura.bindgroup1100.default,1` の形（`fixtures/emo2/shell/master/descript.txt:18-72`）。`surfaces.txt` には **一切無い**。
- しかし現状:
  - `areka_parsers::shell::Shell`（surfaces.txt 由来）に **bindgroup / MAYUNA フィールドは無い**（`model.rs` 全走査で不在）。
  - descript.txt を読む `areka_parsers::package`（`MountModel`/`GhostNames`）は **`name`/`sakura.name`/`kero.name` のみ保持**（`package/model.rs:41`）。bindgroup KV は**捨てている**。
- **帰結**: R4.1「shell descript の bindgroup default に基づく bind 集合を…解決する」を満たす**入力データが現状どこにも無い**。seriko は「解決済み bind id 集合を外から受ける」か「descript の bindgroup KV を自前で読む」かを選ばねばならない（§5-DD4・研究項目 R-1）。
- 補足（bind id ↔ animation id の意味論・研究項目 R-2）: `bindgroupNNNN` の `NNNN` は着せ替え surface 番号（1100/1200…）であり、これが `BindSet`（＝animation ID 集合・`bind.rs`）とどう対応するかは未確定。emo-compose の compose は `active_binds: &BindSet` を animation ID で引く。**「bindgroup 番号 → 有効 animation id」の写像規則**は design で ukadoc＋emo2 実測により確定が要る。

### 2.2 surface `name` 定義の解決経路（R2.3）

- 要件 R2.3 は「surfaces.txt の `surface.alias` で定義された文字列と `name` で定義された文字列を同一解決経路で扱う」。
- emo2 実測: surfaces.txt の `kero.surface.alias` ブロックに **数値キー（`0`,`100`…）と日本語キー（`通常`,`照れ`…）が同居**（`surfaces.txt:458-494`）＝`AliasMap` が両方を吸収済み（`resolve_alias` で引ける）。
- ただし **ukadoc が言う surface の `name` 定義**（`surfaceNNN { … name,定義名 … }` 形式）が emo2 に現れているか、また現 `Shell`/`AliasMap` がそれを取り込んでいるかは**未確認**（emo2 の alias ブロックは alias 表であって surface 内 `name` 行ではない）。R2.3 を「両方を同じ `resolve_alias` から引ける」で満たせるかは要検証（研究項目 R-3）。
- なお descript.txt 側の `bindgroupNNNN.name,腕,伸び` は**着せ替え要素名**であり surface 切替の name とは別物（混同注意）。

### 2.3 seriko クレート本体は未作成

- `crates/areka-seriko/**` は**存在しない**（Glob 無ヒット）。brief どおり**新設クレート**（Extends 無し）。ワークスペース登録（ルート `Cargo.toml` の members）＋Cargo.toml 追加が要る。

---

## 3. 要件 → 資産マップ（ギャップタグ: 実在 / 欠落 / 制約）

| 要件 | 必要能力 | 対応資産 | タグ |
| --- | --- | --- | --- |
| R1.1 `SurfaceSink` 実装 | trait 実装 | `areka_sakura::SurfaceSink` | 実在 |
| R1.2/1.5 発火受理・Send 発行 | inbox で `TalkCue` 受理 | actor 規約・`TalkCue: Send` | 実在 |
| R1.3/1.4 独立スレッド・Close/drop 停止 | `spawn_actor`+`run_inbox`・自前 Close variant | actor 規約（Close は規約） | 実在＋制約(DD3) |
| R2.1 数値 id 解決 | `Emote{key}` の数値文字列 parse | 既存に parse 口なし（seriko 自前） | 欠落（小） |
| R2.2/2.4 alias/name 解決＋失敗ログ | `resolve_alias` | `EmoWorld::resolve_alias` | 実在 |
| R2.3 alias と name を同一経路 | 同上 | surface `name` 取り込み要検証 | 欠落/不明(R-3) |
| R2.5 複数 id の決定的選択 | `Vec<u32>` から 1 つ選ぶ規則 | データ実在（例 `静観→[2106,2206]`）・規則未定 | 欠落（規則・R-4） |
| R3.1-3.5 per-scope 状態・非表示 | `ActorKey`→状態マップ・hide 遷移 | 状態機は seriko 新規 | 欠落（新規） |
| R4.1-4.4 静的 bind 集合 | bindgroup default → BindSet | **入力データ源が無い**（§2.1） | **欠落（大・R-1/R-2）** |
| R5.1-5.5 emo への発行 | 表示指令発行・単一発行点・mock 口 | emo-present API 形（`show_surface(scope,id,binds)`・brief 正本）＋mock は自前 | 欠落（新規）＋制約(DD5) |
| R6.1-6.4 失敗処理・ログ規律 | error!/warn!+skip・panic 限定 | areka ログ規律・`run_inbox` の Err 継続 | 実在 |
| R7.1-7.4 決定論観測 | fixture 直入力・mock 発行列照合 | sakura MockSink 流儀・emo2 alias 実データ | 実在（流儀） |

---

## 4. 実装アプローチ案（A/B/C）

前提: 本体は **新設クレート `crates/areka-seriko`**（brief 確定・Extends 無し）で確定的。差が出るのは **(i) alias 解決表の消費形** と **(ii) bindgroup default の供給経路** の 2 軸。以下は主にこの 2 軸の組合せ。

### 案A: `EmoWorld` を借用して解決を委譲（薄い seriko）
- seriko アクターが `&EmoWorld`（または `Arc<EmoWorld>`）を保持し、`resolve_alias` を毎回呼ぶ。bind は「解決済み `BindSet` を構築時に外（ghost-setup）から受ける」。
- ✅ alias 二重定義ゼロ・emo-compose の warn ログをそのまま活用・seriko は状態機に集中。
- ✅ bindgroup 解決責務を seriko から外へ出せる（§2.1 の欠落を seriko 外＝ghost-setup/別ユニットへ委譲）。
- ❌ `EmoWorld` は `Send` だが `Sync` 保証は未確認（bevy World 共有）＝**別スレッド保持は要検証**（研究項目 R-5）。Arc 共有なら `&self` メソッドのみ使用で可の可能性。
- ❌ R4 の「bindgroup default を起動時に一度だけ解決」の主語が seriko でなくなる＝要件文言との整合を design で確認。

### 案B: 構築時にスナップショットを受領（自己完結 seriko）
- seriko 構築時に「alias 解決表のスナップショット（`BTreeMap<String,Vec<u32>>` 相当のクローン、または解決関数クロージャ）」＋「解決済み静的 `BindSet`」を受け取り、以降は `EmoWorld` に依存しない。
- ✅ スレッド安全が自明（所有データのみ・`Send`）。テストが最も単純（fixture から表を組んで直入力）。R7 の決定論観測に最適。
- ✅ R4 の主語を seriko に保てる（「受け取った bindgroup 情報から起動時一度だけ `BindSet` を組む」）。
- ❌ `AliasMap` を外へ取り出す**公開口が emo-compose に無い**（private Resource）＝emo-compose に `alias_snapshot()` 級 accessor 追加が要るか、`resolve_alias` を全キー分呼ぶのは非現実的（研究項目 R-6）。二重定義回避と snapshot 取得のどちらを取るかが論点。

### 案C: ハイブリッド（解決は借用・bind は自前スナップショット）
- alias は案A（`resolve_alias` 委譲で二重定義ゼロ）、bindgroup default は案B（構築時に「bindgroup KV or 解決済み BindSet」をスナップショット受領）。
- ✅ 「alias 二重定義禁止（R2）」と「bind の起動時一度解決（R4）」を両立しやすい。
- ✅ §2.1 の欠落（bindgroup データ源なし）を**契約の受領点**として明示化でき、供給側（ghost-setup or 別ユニット）と分担を切れる。
- ❌ 依存が2系統（EmoWorld 借用＋スナップショット）で構築引数がやや複雑。スレッド安全（R-5）は案A と同じ検証が要る。

**层構造（いずれの案でも共通・brief の3片）**: 解決層（alias/name→id・純粋・単体可）／状態層（per-scope surface＋BindSet 置き場）／発行層（emo 指令・**単一関数**に集約＝R5.3 の loop シーム）。

---

## 5. 設計判断項目（要件ディスカッションへ送る・番号付き）

- **DD1 — `\s` 入力は `Emote{key: String}` 一本／`EntityRef` は非到来前提でよいか。**
  実測: sakura コンパイラ（`compile.rs:50`）は `\s[ID]` を無条件に `Emote{key}` へ写す（`"-1"`・`"通常"`・`"0,1,foo"` も文字列のまま）。`EntityRef(u64)` は dola の別用途。**seriko の解決層は「文字列 key を (a) 数値 parse→id (b) 失敗時 alias 表引き (c) `-1` は非表示センチネル」で分岐する設計でよいか**。`EntityRef` を受けたら（保険で）どう扱うか（error skip か u64→u32 縮小か）も確定要。要件 R2.1/R3.3 の文言（`\s[-1]` 相当・数値 id）とこの実体の対応を明記する。

- **DD2 — alias 解決表の消費形（案A/B/C のどれか）。**
  `EmoWorld::resolve_alias` 借用（二重定義ゼロ・warn 委譲）か、構築時スナップショット（スレッド安全自明・要 accessor 追加）か。**emo-compose への公開 accessor 追加（`alias_snapshot()` 級）の可否**が案B/C の前提。R2「二重定義しない」との整合。

- **DD3 — seriko inbox enum の停止 variant。**
  areka-actor に共有 `Close` 型は無い＝seriko が `SerikoMsg`（仮）enum に `Close` variant を自前定義（`SakuraMsg::Close` が先例）。**inbox で受けるのは `TalkCue` そのものか、`SerikoMsg::Cue(TalkCue)` 級でラップするか**（`SurfaceSink::emit` は `&mut self` で cue を渡す＝sink 実装体と actor inbox の関係を design で確定：sink が inbox へ send する薄いブリッジになる想定）。

- **DD4 — bindgroup default の供給経路（§2.1 の核・最重要）。**
  現状どのパーサも bindgroup KV を持たない。選択肢: (a) seriko が descript.txt の bindgroup KV を自前パース（新パーサ責務が seriko に入る＝層越え懸念）／(b) 別途「解決済み静的 BindSet」を構築時に外から受ける（seriko は器のみ・R4 主語がずれる）／(c) areka-parsers/package に bindgroup KV 保持を増設（上流拡張・スコープ拡大）。**どこが bindgroup を読むかの責務境界**を確定する。研究項目 R-1/R-2 と直結。

- **DD5 — emo への表示指令 API 形と `\s[-1]` 非表示の表現（emo-present と突合）。**
  emo-present brief の正本は `show_surface(scope, surface_id, binds)`。**この API に「非表示」意味論があるか**（別メソッド `hide(scope)` か、`surface_id` にセンチネルか、`Option<surface_id>` か）を両 design で突合（emo-present brief 明記の調整点）。R5.2/R5.5 は「非表示遷移を発行できる」ことを要求＝seriko 側の mock 出力先 trait に非表示を表現する必要がある。**単体観測は seriko 定義の mock 出力先 trait で emo-present 完了を待たない**（R5.5）。

- **DD6 — 複数 id alias の決定的選択規則（R2.5）。**
  実データに `静観→[2106,2206]` 等が実在（`surfaces.txt:478`）。ukadoc/SSP de-facto の確認要（先頭固定？ランダム？）。**決定論観測（R7）を壊さない＝先頭固定など決定的規則**が有力だが、SSP 実挙動との差を design で確認。

- **DD7 — surface `name` 定義の取り込み（R2.3）。**
  surfaces.txt の `surface.alias` は `AliasMap` に入るが、surface 内 `name,定義名` 行が現 `Shell`/`AliasMap` に取り込まれているか未確認。取り込まれていなければ R2.3 を満たすため上流拡張 or seriko 側補完が要る（研究項目 R-3）。

- **DD8 — 発行点の単一関数（R5.3・loop シーム）と発行タイミング。**
  「状態→合成指令」を単一関数へ集約し、後続 `seriko-loop` が同じ関数を叩ける形。**冪等発行か差分発行か**（状態不変時に再発行するか）、`\s` 到着即時発行の粒度を design で確定。brief の未網羅点「`\s` の即時性・wait 中到着順」（`at` 秒は sakura が正本＝seriko は到着順適用で可か）も含む。

---

## 6. 研究項目（design フェーズへ持ち越し・"Research Needed"）

- **R-1（高）**: bindgroup default のデータ源確保。descript.txt の `sakura.bindgroupNNNN.default` をどの層が読むか。areka-parsers/package 拡張か seriko 自前か外部供給か（DD4）。
- **R-2（高）**: `bindgroupNNNN`（着せ替え surface 番号）→ `BindSet`（animation ID 集合）の写像規則。ukadoc MAYUNA 仕様＋emo2 実測（surface1000 の bind animation 定義）で確定。
- **R-3（中）**: surface `name` 定義（`name,定義名`）が現 `AliasMap`/`Shell` に入るか。ukadoc `descript_shell_surfaces` の `name` と `surface.alias` の等価扱いを再確認（brief 記載の ukadoc 事実の裏取り）。
- **R-4（中）**: 複数 id alias の SSP de-facto 選択規則（DD6）。
- **R-5（中）**: `EmoWorld`（bevy World 内包）の `Send`/`Sync` 特性と、別スレッド actor での `Arc<EmoWorld>` 共有可否（案A/C の前提）。
- **R-6（低）**: emo-compose に alias スナップショット accessor を足す是非（案B/C の前提・DD2）。
- **R-7（低）**: emo-present の指令 API 形の最終確定（並走中ゆえ着手時に emo-present brief/design と再突合・DD5）。

---

## 7. 労力・リスク見積り

- **労力: M（3〜7 日）**。状態機・解決層・発行層は素直だが、**bindgroup default の供給経路（DD4/R-1/R-2）が未確定で、ここが上流拡張に波及すると L 寄り**になる。alias 消費・actor 規約・mock 観測は既存資産で S 相当。
- **リスク: 中**。
  - 主因は §2.1（bindgroup データ源が無い）＝R4 の実装可否が責務境界の設計判断に依存。放置すると R4 が「器だけ」で観測不能になり、決定論テスト網羅（記憶 deterministic-test-coverage-mandate）を満たせない恐れ。
  - `EmoWorld` の別スレッド共有可否（R-5）が案A/C を左右。
  - emo-present との `\s[-1]` API 突合（DD5）は並走ゆえ契約先が動く可能性。
  - 低減策: bindgroup 供給を「構築時受領の契約点」として切り出し（案B/C）、seriko 単体観測は mock 出力先で self-contained に閉じる（emo-present 非依存）。

---

## 8. design フェーズ推奨

- **推奨アプローチ**: **案C（ハイブリッド）を軸に検討** — alias は `resolve_alias` 委譲で二重定義を避け、bindgroup default は「構築時に解決済み `BindSet` or bindgroup KV を受領する契約点」として明示化し、供給責務（DD4）を ghost-setup／上流拡張と分担する。これにより §2.1 の欠落を seriko の内部欠陥にせず、契約の受領点へ昇格できる。
- **設計冒頭で確定すべき鍵**: DD4（bindgroup 供給経路）→ DD2（alias 消費形）→ DD5（emo-present API・非表示表現）の順。R-2（bindgroup→BindSet 写像）は ukadoc `get_doc` ＋ emo2 実測を検証行に落とす。
- **決定論観測の骨格**: sakura `MockSink` 流儀を踏襲し、seriko 定義の mock 出力先へ「(scope, surface_id, binds, 非表示遷移)」の発行列を貯めて照合。fixture は emo2 の `kero.surface.alias` 実データ（`通常→[2100]`・`静観→[2106,2206]`・`-1` 非表示）で alias/複数id/非表示/Close 停止を全て実行テスト化（R7.4）。
