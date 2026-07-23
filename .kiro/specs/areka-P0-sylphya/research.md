# Gap Analysis: areka-P0-sylphya

> 実施日: 2026-07-23／対象: requirements.md（FINALIZED）×現行 worktree（W1 `sakura-dialogue-tags`・W2 `mayuna-compose`/`input-events` マージ後の実測）
> 分析者注: brief.md の行アンカーは W1-W2 マージ後もほぼ有効だが、本書は**本 worktree の実測行番号**へ再突合済み。

## 1. 現状調査（Current State Investigation）

### 1.1 消費側契約は完成済み・実コード確認（W1 完了の実り）

- **`crates/areka-sakura/src/sysvar.rs`（R7 の正本・実装済）**:
  - `SystemVarSnapshot`（`BTreeMap<String,String>` NewType・`get`/`insert`・キー順決定論）＝ sylphya 読み口凍結像の受け皿型（sysvar.rs:26）。
  - `DEFAULT_USERNAME = "ユーザーさん"`（sysvar.rs:45・**既定値の唯一定義点**・R4.2 の再利用先）。
  - `resolve_system_var`（sysvar.rs:65-83）: 値あり→`Text`／`username` 欠落→既定値／それ以外→`PassThrough("%名前")`。
  - **重要な実測**: 「スナップショットに値があれば未対応名でも展開される（値源優先）」がテストで檻入り済み（sysvar.rs:124-139）。→ **`%selfname`/`%selfname2`/`%keroname` はスナップショットへ値を積むだけで展開され、sakura 側は 1 行も変えなくてよい**（R7.1「消費側無改変」が構造的に成立）。
- **`crates/areka-ghost/src/runtime.rs`（W1 差替点・実装済）**:
  - `pub type SystemVarSource = Box<dyn Fn() -> SystemVarSnapshot + Send>`（runtime.rs:75）＝ **同期クロージャ**の供給シーム。
  - `GhostBootOptions.system_vars`（runtime.rs:104）＋暫定 provider `default_system_vars()`（runtime.rs:116-122・username のみ充填）。
  - dispatcher が **talk 起動ごとに provider を一度だけ呼ぶ刻印点**（dispatcher.rs:113-120）＝ per-talk 凍結の意味論は据付済み（R2.1 と同形）。
- **provider の差替箇所は 3 箇所**（すべて `default_system_vars()` を渡している）:
  - `crates/areka/src/main.rs:188`／`crates/areka/src/emo2_boot/mod.rs:319`／`crates/areka/src/emo2_boot/spine.rs:484`。

### 1.2 SHIORI 照会経路（R4.1 の値源）—— 機構は在るが「呼び手の座席」が無い

- `shiori-host32-host/src/shiori3.rs`: `build_request` は任意 ID を特別扱いなく `ID: <id>` に組める（:81,:113・テスト :470-484 が「リソース照会系含む任意 ID」を檻入り済み）＝ワイヤ層は今日から `ID: username` GET 可能。
- **制約①（型）**: `ShioriCall::Get{ id: &'static str }`（areka-kanade/src/msg.rs:126）。`"username"` はリテラルゆえ M1 は成立するが、動的 key 照会（SHIORI Resource 159 項目の汎用照会）へは `String` 化かリテラル表が要る（design 論点）。
- **制約②（ホワイトリスト檻）**: kanade actor の submit は `ALLOWED_EVENT_IDS`（events.rs:59-68・8 ID 固定）外の送出を `ShioriFailure::Internal` で拒否する（actor.rs:175-186）。`username` は**イベントではなくリソース**であり現行檻の語彙外。→ リソース照会は (a) イベント檻と別族の「リソース許可集合」を切る、(b) kanade を通らない照会経路を作る、のいずれかが必要。
- **制約③（送信端の所有）**: `GhostRuntime` は意図して `shiori_tx` を保持しない（runtime.rs:160-162・:273-274）——shiori actor への投函端は kanade が専有し、shiori_tx drop が終了系列の一部（単一 Close funnel 設計）。→ **sylphya の SHIORI 照会 backing は shiori actor への独自チャネルを持てない**。照会の座席（誰が・いつ発行するか）が最大の設計判断（§5 論点1）。
- 実機 emo2/pasta は username GET に 204 を返す（brief 実測）＝ R4.2 の 204→既定値縮退が実機サインオフの観測形。

### 1.3 descript 名前系（R4.3-4.5 の値源）—— 2/3 着地済み

- `areka-parsers/src/package/resolve.rs:68-72`: `GhostNames { name, sakura_name, kero_name }` へ転記済み（`map.get("sakura.name")` 等）。
- **`sakura.name2` は未読取**（resolve.rs に該当行なし・repo grep でも読取ゼロ）→ R4.4 はパーサ（転記）拡張＋`GhostNames` フィールド追加が必要。追加は additive（転記層規律に適合・忠実転記のみ）。
- `kero.name` 未定義時の SSP 互換フォールバック（R4.5）は正典沈黙＝areka 裁量＋対応表記録（brief 表の通り）。

### 1.4 baseware 実値（R5）—— 源は既存・置き場所が論点

- `baseware_name = "areka"`（KanadeConfig::new 既定・msg.rs:197）／`baseware_version = env!("CARGO_PKG_VERSION")`（areka-ghost/src/config.rs:33・workspace 統一 version 0.0.1）。
- 値は自明に取れるが、sylphya は最下層（areka-ghost 非依存）ゆえ「ghost が据える live 導出 backing 経由」か「sylphya 構築時パラメータ」かの選択（§5 論点7）。

### 1.5 IShioriHost プロパティストア（R7.2 の統合対象）

- `crates/areka/src/shiori_host.rs`: `ShioriHostSink.properties: Mutex<HashMap<String, HSTRING>>`（:74）＋`GetProperty` 同期即答（:183-197・欠落 key→`SHIORI_E_PROPERTY_NOT_FOUND`・out 未書込）＋`SetProperty` 即書き（:199-206）＋充填口 `set_property_value`（:124）。
- 再入規約が檻入り済み（`Get` 実装内からの `get_property` 再入・:221）→ 統合方式はこの**同期即答・最小ロック区間・再入安全**を維持する必要（統合後の観測挙動維持＝R7.2）。
- 使用面は SHIORI4/in-proc 系（shiori_session/reference_brain/e2e）であり、emo2 の host32 経路とは別系。統合の消費者は現状 native 脳デモ系のみ＝**統合時期を段階化しても実機 emo2 サインオフ（R9.3）とは独立**。
- 値の橋渡しに `HSTRING`⇄`String` 変換が発生（UTF-16⇄UTF-8・shiori-abi 層の流儀に従う）。

### 1.6 永続化（R6）—— 本番コードに前例ゼロ・却って自由

- 本番コードのファイル書込は**皆無**（grep 全件が test/golden/demo）。`crates/areka/src/placement/mod.rs:501-561` は「ghost.dat を読まない/書かない/plant されても不変」を檻で固定→ sylphya の永続ファイルは **ghost.dat と別名・別置き場**にするか、この檻の更新を design で明示する必要（position-persist 再切削と連動・W4）。
- ghost 識別キーの供給源: `MountModel.shiori.dir`（= ghost/master・物理存在確定・position-persist research.md:45 と一致）。
- 直列化の材料は両方在る:
  - **B1 自前 KV**: `areka-parsers::kv::parse_kv`（寛容 BTreeMap・後勝ち・trim）＋自前ライタ。新規依存ゼロ。寛容読取（R6.3）に最も素直。
  - **B2 serde**: dola（derive 付き・Cargo.toml:15）／wintf（optional・:27）で**既にビルドツリー内**。root `[workspace.dependencies]` に未 hoist なだけ（実測確認済み・「新規依存承認が要る」は誤り＝brief 補正どおり）。ただし寛容読取は「厳格失敗→縮退」の粒度設計が要る。
- 原子的書込（temp→rename）は std::fs のみで実装可能（同一ボリューム rename の原子性・Windows は `std::fs::rename` の置換挙動に注意→ design で確定）。

### 1.7 語彙・字句の実測

- lexer `scan_sysvar`（areka-parsers/src/sakura/lexer.rs:273-285）: 英数字＋`_` のみ走査→ `%m?`（`?`）・`%*`（`*`）・`%property[...]`（`[`）はトークン化不能（brief どおり・現行不変）。`\%` エスケープは実装済。→ R1.1 の 26 トークンのうち字句が届かない 3 形は「key モデル上の第一級保持」と「lexer が届くか」を分離して扱える（key モデルは lexer 非依存）。
- 暦時計: 本番は単調時計のみ（ticker.rs:54 `clock: Box<dyn Fn() -> MonotonicMs + Send>`・既定 GetTickCount64）。暦時刻の注入シームは不在＝時刻系 5 語彙の縮退根拠（R3.1）は現状どおり。
- `AREKA_` env 冠の前例あり（`AREKA_SHIORI_REQUEST_TIMEOUT_MS` 等）＝ R8.2 は既存規約に乗るだけ。

### 1.8 ワークスペース受け入れ条件

- `members = ["crates/*"]` glob＝ `crates/areka-sylphya` を置くだけで workspace 参加。名前衝突なし（sylph* は spec 参照コメントのみ）。
- 最下層 crate のテスト流儀: in-source `#[cfg(test)]`（parsers パターン）・tempfile 非依存（`std::env::temp_dir()` 直下・resolve_tests パターン）・決定論必達。
- DoD ゲートの `cargo test --workspace` は i686 host-32 成果物を先に要する（記憶知見）——sylphya 自体は純 x64 で檻を書く。

## 2. 要件フィージビリティ（Requirement-to-Asset Map）

| 要件 | 既存資産 | ギャップ判定 |
|---|---|---|
| R1 語彙完全形第一級保持（26＋10 枝＋17 名＋5 セレクタ＋159 Resource） | なし（新規） | **Missing**: key モデル・語彙台帳・セレクタ文法はすべて新設。字句が届かない 3 形（`%m?` `%*` `%property[...]`）は key モデル側で保持可（lexer 非依存） |
| R2 読み口 2 形（凍結スナップショット＋逐次） | `SystemVarSnapshot` 型・per-talk 刻印点（dispatcher.rs:118）が既設 | **部分 Missing**: sylphya 側のスナップショット生成器と逐次解決 API が新規。消費側は無改変で成立（§1.1 実測） |
| R3 決定論縮退＋差替シーム | sakura 側素通し/既定値は R7 で確立済み | **Missing**: 点付き NOT_FOUND・縮退記録・backing 登録機構は新規 |
| R4.1-4.2 `%username` SHIORI 照会 | build_request 任意 ID 可・204 写像あり | **Missing＋Constraint**: 呼び手不在。kanade の ID ホワイトリスト檻（イベント語彙）・`id: &'static str`・shiori_tx の kanade 専有（単一 Close funnel）が経路設計を拘束（§5 論点1） |
| R4.3/4.5 selfname・keroname | `GhostNames` 着地済み（resolve.rs:68-72） | **小 Gap**: ghost→sylphya への据付結線のみ |
| R4.4 selfname2 | 未読取 | **Missing（小）**: resolve.rs 転記 1 行＋`GhostNames` フィールド追加＋未定義時縮退規則（areka 裁量記録） |
| R5 baseware.name/version | `"areka"`＋`env!("CARGO_PKG_VERSION")`（config.rs:33） | **小 Gap**: 値の注入形（backing か構築時パラメータか・§5 論点7） |
| R6 永続 backing | 前例ゼロ（本番書込皆無）・kv パーサ/serde 両材料あり・MountModel.shiori.dir が識別キー源 | **Missing**: 形式・原子的書込・寛容読取・バージョニング・識別キーすべて新規。placement の ghost.dat 檻（:501-561）との整合を design で明示 |
| R7.1 provider 差替 | 差替点 3 箇所（main.rs:188 等）・型 `SystemVarSource` 無改変で可 | **小 Gap**: sylphya 読み口→クロージャ包装のみ |
| R7.2 ShioriHostSink 統合 | ストア・充填口・再入檻あり | **Missing（中）**: 統合方式（置換/充填/型シームのみ）と HSTRING 橋・観測挙動維持（§5 論点4） |
| R8 横断規律 | AREKA_ 前例・tracing 全体規約・注入時計流儀（ticker.rs:54） | **適合**: 新規パターン不要 |
| R9 決定論檻＋実機 | 偽 backend 注入シーム（`ShioriBackend`・real.rs:47）・有界 auto-exit（AREKA_APP_SMOKE_EXIT_MS）既設 | **適合**: 既存流儀で檻化可能。R9.3 は「経路が実 SHIORI 照会値源を通った」ログ証跡の設計が要る |

**複雑性シグナル**: 外部統合（SHIORI 照会ルーティング）1 点が High-touch、他は新規最下層 crate 内のアルゴリズム/データモデル（決定論・純 x64 で檻化容易）。

## 3. 実装アプローチ比較

### Option A: 既存コンポーネント拡張（sylphya を areka-ghost 内へ）
- **不成立**。消費者に areka-sakura（`%username` 展開）が含まれ、sakura が ghost 内の読み口へ依存すると areka-sakura → areka-ghost の**循環**（ghost は sakura に依存済み・runtime.rs が dispatcher/sakura 型を使用）。brief の依存グラフ検証どおり。
- ShioriHostSink 側（crates/areka bin）へ置く案も、sakura/ghost/kanade が bin へ依存できず不成立。
- ✅ 新ファイル最小 ❌ **依存循環で構造的に不可能**（記録のために残す）。

### Option B: 新規最下層クレート `crates/areka-sylphya`（brief 案・推奨）
- sylphya は名前空間・key モデル・セレクタ文法・語彙台帳・読み口 2 形・backing trait・登録機構・persistent backing（std::fs のみ）を所有。上流 areka 依存ゼロ（候補依存は同格最下層の `areka-parsers`＝kv 再利用のみ・循環なし）。
- SHIORI 照会 backing・live 導出 backing（GhostNames/baseware）は**ghost（依存の頂点）が据付**。「消費者は backing を知らない」（R2.4）は依存グラフから自動帰結。
- 統合面: provider 差替（3 箇所・型無改変）／ShioriHostSink（bin 側で sylphya ハンドルを持たせ統合）。
- ✅ 責務分離・決定論檻を crate 内で完結・W3 同居 spec（seriko-loop/choice-render）と編集面が互いに素 ❌ 新 crate の公開 API 設計コスト・結線は複数 crate を跨ぐ（ghost/kanade/bin）。

### Option C: B＋段階化ハイブリッド（統合を 2 段に分ける）
- 第 1 段（M1 中核）: crate 新設＋語彙台帳＋読み口＋縮退＋永続 backing＋provider 差替＋selfname 系実導出＝**kanade/bin への編集を最小化**。
- 第 2 段（M1 内・別タスク群）: username SHIORI 照会 backing の座席確定（kanade 側リソース照会増分）＋ShioriHostSink 統合。
- ✅ ウェーブ内の編集面衝突リスクを段階で制御・レビュー単位が小さい ❌ 計画が複雑・「第 2 段が M1 から滑落する」誘惑（R4.1/R7.2 は必達ゆえ滑落不可を tasks で明示する必要）。

**評価**: 実体は B 一択（A は不可能）。B の実装順序として C の段階化を tasks 設計に取り込むのが現実的。

## 4. 工数・リスク

- **工数: L（1-2 週間）** — 新 crate（key モデル＋語彙台帳 159+26+10+17＋セレクタ文法＋読み口＋永続 backing＋決定論檻全網羅）が主量。結線（provider 差替 3 箇所・ghost 据付・kanade リソース照会増分・sink 統合）が跨ぎ編集。
- **リスク: Medium** — 技術は既知（Rust 純粋層・std::fs・既存 actor 流儀）だが、**SHIORI 照会ルーティングが kanade の 3 つの既存不変量（ID 檻・&'static str・shiori_tx 専有）に触れる**点が唯一のアーキテクチャ接触面。永続形式は前例ゼロだが自由度が高く低リスク。語彙台帳は量が多いだけで機械的。

## 5. 設計判断論点（design/要件討議へ送る番号付きリスト）

1. **`%username` SHIORI 照会の座席と時機**（最重要）: (a) kanade の boot 系列にリソース照会を増分し値を sylphya へ書き込む（prefetch＋キャッシュ・provider は同期のまま）、(b) per-talk 同期照会（dispatcher スレッドをブロック・タイムアウト設計が要る）、(c) ghost が shiori_tx のクローンを保持（**単一 Close funnel・kanade 専有の既存不変量に抵触**）。あわせて `ALLOWED_EVENT_IDS`（イベント檻）と別族の「リソース照会許可集合」の要否、`ShioriCall.id: &'static str` の扱い（M1 はリテラル `"username"` で足りるが 159 項目汎用化は String 化）。
2. **スナップショットへ積む M1 語彙**: username＋selfname＋selfname2＋keroname（消費側は値優先で無改変展開・§1.1 実測）。点付き `baseware.*` をスナップショットに含めるか（%窓に無い名は含めない、が素直）。
3. **永続形式と置き場**: B1 自前 KV（`areka-parsers::kv` 再利用・寛容読取に素直・新規依存ゼロ）vs B2 serde（ビルドツリー内・hoist 判断・厳格失敗→縮退の粒度設計）。保存先ディレクトリ（SSP profile 慣行を ukadoc `file_structure` で確認・emo2 fixture は read-only）と ghost 識別キー導出（`MountModel.shiori.dir` 素のパス vs 正規化 vs hash）。placement の ghost.dat 檻（mod.rs:501-561）との共存宣言。
4. **ShioriHostSink 統合の方式と時期**: (a) `properties` HashMap を sylphya 逐次読み口への委譲に置換（`GetProperty`→sylphya 解決・NOT_FOUND→`SHIORI_E_PROPERTY_NOT_FOUND` 写像が自然）、(b) 充填スルー（sylphya 側更新を `set_property_value` へ流す・第 2 ストア残存で R7.3 に反する）、(c) M1 は型シームのみ（Boundary Candidate 継承だが R7.2 の「存置しない」との整合を要件討議で確認）。HSTRING⇄String 橋・同期即答/再入規約の維持。`SetProperty` の書込先（SET 予約シームとの関係・R3.4）。
5. **key モデルの内部表現**: フラット 26 トークンと点付き木を単一名前空間で持つ具体形（フラット名→正準 key への別名表 vs 2 窓 1 解決器）。セレクタ 5 形の文法解釈の置き場（key パーサ）と `%property[プロパティ名]` ゲートウェイの key 写像。SHIORI Resource 159 項目の台帳表現（const 表 vs 生成マクロ）。
6. **baseware.name/version の注入形**: ghost 据付の live 導出 backing 経由（一貫性重視）vs sylphya 構築時パラメータ（自明さ重視）。`env!("CARGO_PKG_VERSION")` の評価 crate が変わる点（areka-ghost で評価すれば workspace 統一 version ゆえ同値）。
7. **sakura.name2 転記拡張の形**: `GhostNames` へのフィールド追加（additive）と `name.allowoverride`（brief Boundary Candidate）をどの spec が持つかの再確認。
8. **SET 有効群の書込型シーム**（R3.4 予約のみ）の形: trait メソッドのシグネチャだけ切るか、書込要求 enum を予約するか。SET 無効項目の失敗挙動（正典沈黙）の対応表記録位置。
9. **Boundary Candidates の裁定**: `%property[...]` lexer bracket 拡張（W2 mayuna 完了で parsers 干渉は解消済み・安価になった）／`%screenwidth`/`%screenheight`（物理/論理 px 契約が前提・DPI 欠陥の同型ハザード）／`currentghost` name 系点付き実導出（GhostNames 着地済みで安価）／暦時計注入シームの本 spec 内新設 vs 追跡 spec。
10. **R9.3 実機サインオフの証跡設計**: 「経路が実 SHIORI 照会値源を通った」ことを RUST_LOG grep で決定論判定できるログイベント名の予約（有界 auto-exit＝AREKA_APP_SMOKE_EXIT_MS 既存流儀）。
11. **W4 position-persist への key 族契約の確定形**: 4 key 族（窓位置 scope 別・バルーンオフセット scope 別・起動記録・vanish 回数）の key 命名と値スキーマを sylphya design で確定し、brief 申し送り節のデルタ適用（W4 の `/kiro-design` 前）で消費側へ渡す。kanade `on_first_boot` の Ref0 固定 `"0"`（events.rs:91-97）差替は W4 の領分（sylphya は器のみ）。

## 6. Research Needed（design フェーズへ持ち越す調査）

- ukadoc `file_structure`（MCP）: profile ディレクトリ慣行→永続ファイル置き場の確定材料。
- SSP の `%keroname` フォールバック挙動（kero.name 未定義→本体名）の裏取り（ukadoc 沈黙なら areka 裁量＋対応表）。
- Windows `std::fs::rename` の既存ファイル置換挙動（原子的確定の実装細部・ReplaceFileW 要否）。
- SHIORI Resource 特殊エントリ「-」の実リソース ID（brief 未確認のまま・台帳表現に影響）。

## 7. 推奨（design への申し送り）

- **アプローチ**: Option B（新規最下層 crate `crates/areka-sylphya`）を C の段階化順序で実装。第 1 段＝crate 中核＋語彙台帳＋永続 backing＋provider 差替＋descript 系実導出、第 2 段＝username 照会座席（論点1）＋sink 統合（論点4）。両段とも M1 必達（滑落不可）。
- **最初に確定すべき論点**: 論点1（照会の座席）→ 論点3（永続形式/置き場）→ 論点4（sink 統合方式）。この 3 つが跨ぎ編集面（kanade/bin）の範囲を決め、W3 同居 spec（seriko-loop・choice-render）との非干渉宣言の根拠になる。
- **好条件の再確認**: 消費側（sakura）は実測で無改変成立・差替点は型無改変・ワイヤ層は任意 ID 対応済み・既定値の唯一定義点も既設——「3 箱分裂の解消」は結線の設計だけが本体で、消費契約の再交渉は一切不要。
