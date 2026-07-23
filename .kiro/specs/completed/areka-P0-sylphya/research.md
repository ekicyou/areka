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

## 8. 要件討議決定（2026-07-23・開発者裁定）

要件ディスカッション議題 1 の裁定と、同時に示された要件レベルの拡充 3 点。requirements.md へ反映済み。

1. **R7.2 は必達維持（論点4 の (c) 型シーム案は棄却）**: プロパティ問い合わせ層の実体はシルフィアであり、`ShioriHostSink` は窓口にすぎない。`GetProperty`/`SetProperty` の API 面は存続し、背後の値の出どころを sylphya の解決へ統合する。統合の具体方式（委譲形・HSTRING 橋・再入規約維持）は design（論点4 の残件）。
2. **問い合わせ元コンテキストの第一級化（R2.6 新設）**: 同じ名前でも問い合わせ元 SHIORI により回答が異なる語彙（例: `currentghost.status`）と、大域で同一の語彙（例: `system.os.type`）がある。読み口 API は「誰が聞いているか」を第一級で受け取る形とする（M1 は単一ゴーストだが API 形を先に正しく持つ）。→ 論点1（照会座席）・論点5（key モデル）の設計入力。
3. **永続化の層モデル（R6.5 改稿）**: 永続スコープは「areka アプリレベル」「SHIORI〔ゴースト〕レベル」「シェルレベル」「バルーンレベル」の層別で管理し、各層の永続情報は対応する層の profile フォルダへ保存する（伺か慣行準拠）。→ 論点3 の置き場問題はこの原則で確定・残件は 4 key 族→層写像とファイル命名。
4. **直列化形式は TOML に確定（R6.4 改稿・論点3 の形式選択を解消）**: B1 自前 KV / B2 serde 汎用形式の比較は closed。残件は toml クレートの採用形（workspace hoist・寛容読取〔parse 失敗→警告＋不在縮退〕の粒度）・Windows rename 細部・ukadoc `file_structure` の profile 慣行裏取り。

## 9. プロパティ実体層タクソノミー（2026-07-23・討議 #2 追補）

従来の「3 バッキング（live 導出／SHIORI 照会／永続）」は粗すぎ、「live 導出」に性質の異なる実体が混在していた。ukadoc 実測（list_propertysystem）で仕分けし直した正準タクソノミー。**backing 差替シームの型はこの 5 層＋書込 2 意味論を最初から収容できる形とする**（M1 で実装する backing は①の一部・④username・⑤のみ——②③は縮退だがシームの型が層の存在を表現していること）。

### 読取側の実体層（5 層）

| # | 層 | 代表語彙（ukadoc 実測） | 性質 | 状態の所有者 |
|---|---|---|---|---|
| ① | 静的構成層 | `selfname`/`keroname`・`baseware.name`・craftman/path/thumbnail 系 | load-time 確定・reload まで不変 | parsers マウントモデル（ghost 据付） |
| ② | リアルタイム運行状態層 | `currentghost.scope(ID).surface.num`（現在サーフェス）・`currentghost.scope(ID).x/y`（現在窓座標）・`currentghost.balloon.scope(ID).basepos.x/y`・`balloon.scope(ID).background.color` | **他エンジンが別スレッド/アクターで所有する生きた状態**。取得＝クロスアクター読取（push 型鏡像 vs pull 型同期照会）・反映＝コマンド配送 | seriko（surface）・wintf 窓層/placement（x/y）・emo（balloon 描画状態） |
| ③ | システム環境層 | `system.memory.load`（移動平均と正典明記＝サンプリング実装）・時刻系・`screenwidth` 系 | OS 直読＝非決定源。注入シーム必達（R8.3）・決定論檻は偽境界 | OS |
| ④ | SHIORI 照会層 | `username`・`shiori.変数名`・ext 亜枝 property.get/set | 問い合わせ元コンテキスト相対（討議 #1）・host32 は別プロセス | SHIORI（照会先） |
| ⑤ | 永続層 | history・rateofuselist・areka 独自 4 key 族 | 層別スコープ×profile フォルダ×TOML（討議 #1 確定） | sylphya 自身 |

### 書込（SET）の 2 意味論

- **運行コマンド書込**: `currentghost.scope(ID).surface.num` の SET は「**設定も可能で `\s[]` タグと同じ挙動になる**」（ukadoc 明文）＝ストア書込ではなく**ランタイムエンジンへの命令**。`animation.num` SET（`\i[]` 連続等価）も同族。R3.4 の型シーム予約はこのコマンド意味論を前提に切る。
- **ストア書込**: vanish 回数・起動記録等＝⑤への永続書込。

### 論点12（新規・design 送り）: 実体層②のクロスアクター設計

sylphya は最下層 crate だが②の状態所有者は上位エンジンのアクター（別スレッド）。(a) push 型＝所有エンジンが状態変化を sylphya の鏡像キャッシュへ配信（読取は常に同期・鮮度は配信ラグ）、(b) pull 型＝解決時にチャネル同期照会（鮮度最新・ブロッキングとデッドロック設計が要る）、(c) M1 型シームのみ＝②全縮退（採用・実配線は M2 以降の実導出時）。per-talk 凍結スナップショット生成点・ShioriHostSink の同期即答（SHIORI スレッドから呼ばれる）との整合が設計条件。SET 運行コマンドの配送先（kanade 経由 vs 直接）も同論点。
→ **討議 #3（下記 §10）で方向裁定済み**: pull 型 (b) は棄却・push 鏡像 (a) が正準アーキテクチャ。残件は配送経路（kanade 経由 vs 直接）・epoch フェンス（read-after-write 予約シーム）の要否・読みハンドルの具体形。

## 10. 討議 #3 裁定（2026-07-23・開発者承認）: 同期読み・非同期供給の掲示板モデル＋sylphya アクター化

**問題**: ukadoc プロパティ API を素直（同期 pull）に実装すると、解決のたびにクロスアクターのブロッキング照会が連鎖し「プロパティシステムが系全体をロックする」設計になる。SSP の同期 API は SSP 自身が単一スレッド・メッセージポンプであることの暗黙の直列化に乗っているだけであり、正典が拘束するのは観測可能な挙動であって実装形ではない（馬鹿正直実装は不要＝開発者所見）。

**裁定（design 第一候補として確定）**:
1. **原則**: 読みは常に同期・常に無待機。供給は常に非同期。読み手はアクター境界を越えない（R2.7 として要件化済み）。
2. **掲示板（マテリアライズド・ビュー）モデル**: 「読むときに聞きに行く」を逆転し「変わったときに貼りに来る」。状態所有エンジン（seriko＝surface.num／placement＝x,y／emo＝balloon 状態）が状態変化時に sylphya の鏡像へ publish（key 単位 single-writer・epoch 付きスロット）。
3. **同期読み面はすべて鏡像読み**: `%property`/%系＝talk 開始の凍結スナップショット生成時に鏡像から複写（per-talk 凍結の既裁定と同形）／`GetProperty`（SHIORI スレッド）＝鏡像の無待機読取のみ。同期経路での pull 照会は禁止。
4. **SET＝コマンド配送**: 所有エンジンのコマンドキューへ投函して即返る。所有エンジンが処理→新状態を publish＝key 単位の因果順序は所有者キューが保証。
5. **遅い値源**（SHIORI ext 照会・ネットワーク系）は同期読み経路に載せない。正典の `\![get,property,イベント名,...]`（結果をイベント発火で返すコールバック形）＝天然の非同期 API、または事前 prefetch→鏡像充填で扱う。
6. **sylphya 本体はアクター**（開発者追加裁定）: 変異側（publish 受信・SET コマンド中継・永続書込・prefetch 編成）は sylphya アクターが所有し、actor-foundation（`areka-actor`）規約に載る（結線＝ghost の領分）。**読み口は共有読みハンドル**（epoch 交換の不変スナップショット）として同期・無待機を保つ——読みまでアクター経由にするとチャネル往復ブロッキングが再発するため。
7. **トレードオフ（承知の上で採用)**: 鏡像読みは有界の鮮度遅れ（publish ラグ）を持つ。%系は per-talk 凍結が既裁定で一貫・イベント駆動ゴーストは SSP からも「UI ループのその瞬間」以上の一貫性を得ていない。厳密 read-after-write が将来必要なら epoch フェンス付き読みを design の予約シームとする。

**M1 への効き方**: M1 実装層（①静的＝不変・④username＝boot prefetch→鏡像充填・⑤永続＝sylphya 自身所有）は全ロック不要で成立。backing シームの型を鏡像 push 形で切ることが論点12（②の M2 配線）の先行投資になる。

### 討議 #4 補遺（2026-07-23・開発者裁定）: アクター粒度＝古典スレッドの同期アクター 1 本

- **永続スコープ単位のアクター分割はしない**（厳密派の理想形として認識した上で過剰と裁定）。伺か規模なら「システムの数だけ古典スレッド」＝areka 並行モデルの既定（スレッド独立・同期アクター・`areka-actor`/std::sync::mpsc 規約）にそのまま載る。async アクターランタイムは不採用。
- 根拠: ⑤の書込頻度・量は微小で、単一アクター受信箱の直列化が同一 profile への並行書込を構造的に排除（好都合）。最高頻度 publish（ドラッグ中の窓座標等）も mpsc 受信箱で十分・詰まる場合の正解は key 単位 latest-wins 合流（送り側 coalescing）でありアクター増殖ではない。
- 退路: アクター境界＝チャネルゆえ、将来スコープ別分割が必要でも同一ハンドル背後のリファクタで済む（「2 例目の実物が要求してから抽象化」規律に整合）。

## 11. 設計フェーズ調査補遺（2026-07-23・design 生成時）

§6「Research Needed」の消化と、design 生成時の実コード再突合の結果。

1. **profile フォルダ慣行（ukadoc 裏取り・R6.5 の確定材料）**: ukadoc `manual_install`（file_structure カテゴリ）は「ゴーストを配布する際に**ゴーストとシェルのフォルダにある profile フォルダ**や、セーブデータ等を含めないように注意」と明記＝層（ゴースト/シェル）ごとに profile フォルダが実在する慣行を正典側が前提化している。里々 Wiki（困ったときの対処法）は「ghost フォルダ内の master フォルダ内にある profile フォルダを削除すれば初回起動になる」＝ SHIORI〔ゴースト〕層の実置き場が `ghost/master/profile/` であることの実運用証跡。バルーンも同型（フォルダ単位コンテンツ＋profile）。areka の保存先は各層 profile 配下に **`profile/areka/sylphya.toml`**（`areka/` サブフォルダで SSP セーブデータとの衝突を構造的に回避・SSP `ghost.dat` バイナリ互換は要件どおり不要）。
2. **Windows `std::fs::rename` の置換挙動**: Rust std は Windows で `MoveFileExW(..., MOVEFILE_REPLACE_EXISTING)` を用い、既存宛先ファイルを**置換する**（同一ボリューム内 rename）。temp 書込→rename の原子的確定（R6.2）は std のみで実装可能。`ReplaceFileW` は不要（バックアップ保持要件が無い）。クラッシュ窓は「temp 書込中」に限定され、その場合旧ファイルは無傷＝要件の非破壊保証を満たす。
3. **toml クレートの導入形**: root `[workspace.dependencies]` に `toml` は未 hoist（実測・Cargo.toml:15-34）。`dola` が `toml = "0.8"`（feature `toml`）で既にビルドツリー内。→ **`toml = "0.8"` を workspace hoist し `areka-sylphya` が workspace 依存で参照**する（新規外部 crate ではない＝brief の承認前提どおり）。寛容読取の粒度: ファイル全体 parse 失敗→警告＋ファイル不在扱い／key 欠落・型不一致→当該 key のみ不在（§12 決定 3）。
4. **SHIORI Resource 特殊エントリ「-」**: 実リソース ID は本フェーズでも未確認のまま。M1 は username 以外を照会しないため非ブロッキング。台帳には「ID 未確認」注記付きで第一級保持する（語彙を落とさない）。
5. **`%keroname` フォールバック**: brief 記載どおり descript_ghost（L411-415）に「SSP は省略時 sakura.name」の記述あり＝ SSP 互換フォールバックは正典近傍で裏取り済み。対応表へは「kero.name 未定義→sakura.name へフォールバック・両方未定義→値なし（素通し縮退）」で記録する。
6. **実コード再突合（本 worktree）**: §1 の全アンカーを再確認——`sysvar.rs`（`SystemVarSnapshot`/`DEFAULT_USERNAME`/値源優先展開の檻）・`runtime.rs:75/104/116`（`SystemVarSource`/`GhostBootOptions.system_vars`/`default_system_vars`）・`shiori_host.rs:74/124/183/199`（HashMap ストア・充填口・同期即答・再入規約）・`msg.rs:124-135`（`ShioriCall::Get{id:&'static str}`）・`schedule/events.rs:59-73`（`ALLOWED_EVENT_IDS` 8 ID＋`is_allowed_event_id`・lib.rs:52-58 で公開面 re-export）・`package/resolve.rs:68-72`（`GhostNames` 転記・sakura.name2 未読取）・`areka-actor/lib.rs`（inbox/envelope/停止/流量規約の正本）——いずれも §1 の記載と一致・設計はこの実測へ直接接続する。

## 12. 設計決定記録（2026-07-23・synthesis 帰結）

§5 論点・§8-§10 裁定を踏まえた design 確定事項。裁定（§8/§9/§10）は拘束であり、以下はその具体化。

1. **論点1（username 照会の座席）→ kanade boot 系列の prefetch＋ack フェンス**: §10 裁定「④username＝boot prefetch→鏡像充填」の具体化。kanade が boot 運行表の OnInitialize 後・OnFirstBoot 前に `ShioriCall::Get{id:"username"}` を既存 shiori request 経路（単一 in-flight・既存タイムアウト）で発行する。イベント檻とは**別族の `ALLOWED_RESOURCE_IDS`**（M1 は `["username"]`）を新設し、actor の送出ガードを「イベント許可 ∨ リソース許可」に拡張。`id` は M1 リテラルで足りるため `&'static str` 据え置き（159 項目汎用化＝String 化は M2 シーム）。結果は kanade 構築時注入の **`ResourceSink` クロージャ**（`Box<dyn Fn(&'static str, ResourceOutcome) + Send>`）へ渡す＝ kanade は sylphya へ**依存しない**（`SystemVarSource` と同型の疎結合シーム）。ghost が据えるクロージャは publisher へ publish 後 **barrier（ack）で反映完了を待ってから返る**——kanade 単一スレッドの逐次性により「publish 反映 → OnFirstBoot 発行 → 初回 talk スナップショット」の順序が決定論化する（チャネル 2 本間の順序不定レースを塞ぐ・§10.7 のフェンス予約シームの boot 特化適用）。照会失敗（タイムアウト等）は warn＋不在 publish で boot 続行（起動を殺さない）。
2. **論点2（スナップショットへ積む語彙）→ 値が実在する flat 名のみ**: talk スナップショットには鏡像に値がある M1 flat 語彙（username/selfname/selfname2/keroname）のみを積む。不在名は積まない＝sakura 側の既定値（username）・素通し（他）縮退が既存契約どおり働く。既定値 `DEFAULT_USERNAME` の定義点は sakura に**残置**（sylphya へ複製しない・R4.2。sylphya の台帳は当該 token の縮退政策を `ConsumerDefault` とマークするのみで値を持たない）。点付き `baseware.*` は %窓に無い名ゆえ積まない。
3. **論点3（永続形式・置き場）→ TOML／層別 `profile/areka/sylphya.toml`／識別＝ルートパス固有**: 形式は裁定どおり TOML（workspace hoist・§11-3）。ファイルは各層 profile 配下 `areka/sylphya.toml`（§11-1）。スコープルートは呼び出し側（bin/ghost）が `ScopeRoots`（App/Ghost/Shell/Balloon 各 `Option<PathBuf>`）で供給し、ゴースト識別は `MountModel.shiori.dir` 由来のルートパスそのもの＝パス固有性が識別（hash 不要・per-ghost profile ディレクトリが分離を担う）。寛容読取: ①ファイル不在→debug ログ＋全不在②parse 失敗/未知 `format-version`→warn＋全不在（ファイルは削除しない）③key 欠落→当該 key 不在。バージョンは `format-version = 1`（整数）。値ドメインは**全て文字列**（プロパティ系の正準値域はテキスト・往復等価が自明・型解釈は消費側=W4 の領分）。placement の ghost.dat 檻（mod.rs:501-561）とは**共存**（別名・areka サブフォルダ・M1 は本番ランタイムに永続書込呼出なし＝emo2 read-only fixture も汚さない）。
4. **論点4（sink 統合）→ HashMap 撤去・reader/publisher 委譲**: `ShioriHostSink` の `properties: Mutex<HashMap>` を撤去し、`GetProperty`＝reader の鏡像同期読み（NOT_FOUND→`SHIORI_E_PROPERTY_NOT_FOUND`・out 未書込・同期即答/再入規約は reader が lock-free 同然〔Arc クローンの瞬間 lock のみ〕ゆえ維持）、`SetProperty`＝publisher への Set コマンド投函（即 Ok 返し・§10.4 裁定どおり）。`set_property_value` 充填口は publisher 委譲の薄いラッパとして API 存続（呼出面 shiori_session/reference_brain/e2e 無改変・可視化は actor 処理後）。**Set→Get の read-your-write は有界ラグ**となる（現行 HashMap の即時可視から意図的変更）——R7.2 の列挙観測挙動（dotted key・GetProperty 同期即答・欠落 key エラー）は全て維持され、read-your-write は列挙外。決定論テストは `barrier()`（`SylphyaMsg::Barrier{reply}`）で actor 反映を待ってから Get する形へ既存テスト(d)を更新（陳腐化テスト方針: 生きているので更新）。厳密 read-after-write が将来必要になった場合の epoch フェンス付き読みは §10.7 どおり予約シーム（鏡像に epoch:u64 を最初から刻む）。
5. **論点5（key モデル内部表現）→ 正準＝点付き `PropPath`＋フラット別名表（2 窓 1 解決器）・台帳は const 表**: セレクタ 5 形は `PathSeg{name, selector: Option<Selector>}`・`Selector::{ByName(String), ByIndex(u32)}` の文法で完全解釈（`.current`/`.count` は selector 無し名前セグメント・`.index(ID)` は name="index"+ByIndex）。フラット 26 トークンは台帳 entry として第一級（正準 key への写像は M1 では鏡像 flat 区画への直接名）。語彙台帳（26+10+17+159＋SET 群＋ext 予約）は生成マクロでなく **const 表**（機械的・grep 可能・件数を単体テストで檻化）。`%*` は構文記録のみ・`\%` は語彙外（R1.6）。
6. **論点6（baseware 注入形）→ ghost が publish（機構統一）**: 構築時パラメータ特例を作らず、ghost 結線が `baseware.name`（KanadeConfig と同源 "areka"）・`baseware.version`（`env!("CARGO_PKG_VERSION")`＝workspace 統一 version・評価 crate 差は無害）を静的構成層として publish する。充填機構は publish 1 本。
7. **論点7（sakura.name2）→ `GhostNames` additive 拡張＋未定義縮退＝素通し**: resolve.rs に `sakura.name2` 転記 1 行＋`GhostNames.sakura_name2: Option<String>` 追加（転記層規律適合）。`%selfname2` 未定義時は**値なし＝素通し縮退**（selfname への創作フォールバックはしない・正典裏付けなし）。`%keroname` 未定義時のみ SSP 互換で sakura.name へフォールバック（§11-5）。両規則とも `doc/COMPAT_ARCHITECTURE.md` 対応表へ記録。フォールバック適用は ghost 側の純関数 `derive_flat_statics(&GhostNames)`（publish 前に確定・決定論檻対象）。`name.allowoverride` は本 spec 外のまま（追跡）。
8. **論点8（SET 型シーム）→ `SetSemantics` 2 値＋分類 3 値＋`RuntimeCommandSink` trait 予約**: 台帳が SET 有効群 key を `SetSemantics::RuntimeCommand`（`\s[]` 等価などの運行コマンド）／⑤系を `StoreWrite` に分類。Set コマンドの actor 内分類は `RuntimeCommand`（M1: sink 未登録→warn＋記録のみ・型 trait `RuntimeCommandSink` を予約）／`StoreWrite`（正準語彙外の自由 dotted key → asker 別 host 区画へ書込＝sink 既存挙動の受け皿）／`NotSettable`（SET 無効な正準語彙 → warn＋書込なし・Ok 返し＝正典沈黙の areka 裁量・対応表記録）。
9. **論点9（Boundary Candidates）→ 全て M1 採用見送り（縮退＋追跡）**: `%property[...]` lexer bracket 拡張（消費者ゼロ・点付き解決 API は sink/W4 が先に使う）／`%screenwidth`/`%screenheight`（物理/論理 px 契約が未確定＝DPI 欠陥の同型ハザードを先に潰さない）／`currentghost` name 系点付き実導出（源はあるが currentghost 枝の asker 相対解決の実配線は M2 の②層と同時が自然）／暦時計注入シーム（時刻系 5 語彙と一体の追跡 spec へ）。いずれも「完全語彙＋縮退シーム＋追跡 spec＋roadmap 明記」4 点セットの追跡側（brief 申し送り「roadmap 宿題 4」と同一）。
10. **論点10（R9.3 証跡）→ ログイベント名を design で固定**: kanade prefetch 完了 `info!(target:"areka_kanade::resource", id="username", outcome=..., "shiori resource prefetch done")`（outcome ∈ value/no_content/failed）・sylphya publish 適用 `debug!(target:"areka_sylphya::actor", ...)`・ghost provider `debug!(target:"areka_ghost", "talk snapshot from sylphya reader")`。実機サインオフは `AREKA_APP_SMOKE_EXIT_MS` 有界終了＋`RUST_LOG` 出力の grep（`shiori resource prefetch done` かつ `outcome="no_content"`）＋バルーン生 `%username` 非露出（既存 dialogue-tags 手順と同型）。
11. **論点11（4 key 族契約＝W4 への確定形）**: 正準 key 名 `areka.window.scope(ID).x|y`・`areka.balloon.offset.scope(ID).x|y`・`areka.boot.count`・`areka.vanish.count`（全て SHIORI〔ゴースト〕スコープ・値は文字列）。TOML スキーマは design.md「Data Models」の表が正本。W4 の `/kiro-design` 前 rebase 時にこの契約を消費側へ渡す。kanade `on_first_boot` Ref0 差替は W4 の領分（sylphya は器のみ・不変）。
12. **論点12（②層クロスアクター）→ §10 裁定の push 鏡像で型確定・実配線 M2**: `SylphyaMsg::Publish*` 系がそのまま②層の push 口（所有エンジンが publisher クローンを持ち状態変化時に投函・詰まったら送り側 coalescing）。M1 は②③とも縮退のまま、`BackingLayer` enum（5 層）と publish 口の型が層の存在を表現する（R3.6）。
13. **設計討議 #1（2026-07-23・開発者裁定）: フラット per-asker 区画を最初から確保**: バリデーション指摘 1 の裁定。鏡像は `flat_per_asker`＋`flat_global` の両区画を持ち、解決順は per-asker → global。M1 実導出フラット語彙（username/selfname/selfname2/keroname）は正典上すべてゴースト相対（SHIORI Resource／descript 由来）ゆえ per-asker へ着地し、`flat_global` は将来の大域語彙（screenwidth 系等）用の名前空間として確保する。`PublishStatic`/`PublishShiori` とも asker 第一級（ghost 結線は自 AskerId で publish）。M1 単一 asker でも鏡像モデルを先に正しく持つ＝M2 多重ゴースト時の鏡像型変更（Revalidation Trigger）を回避。

### synthesis 帰結（3 レンズ）

- **一般化**: 「%環境変数の値源」「永続ストア」「IShioriHost プロパティ」は全て『名前で引ける値の掲示板』の特殊例——鏡像（正準 key→文字列値）＋publish＋2 読み口（凍結スナップショット/逐次）1 機構に一般化し、個別実装を持たない。インターフェースのみ一般化し、実装は M1 実導出＋4 key 族に限定（実装の先回り拡張はしない）。
- **build vs adopt**: TOML 直列化＝`toml` 0.8 採用（ビルドツリー内・裁定済）。鏡像の epoch 交換＝`arc-swap` 等の新規 crate は**不採用**、`std::sync::RwLock<Arc<MirrorImage>>` のポインタ swap で足りる（読み＝read lock 内 Arc clone の数十 ns・書き＝actor 単独。新規依存ゼロ・決定論）。アクター基盤＝`areka-actor` 採用（規約正本）。KV 自前案 B1 は closed（裁定）。
- **単純化**: flush API なし（PersistPut＝write-through・書込量微小）／永続 key は 4 族の typed enum（自由 key の汎用永続は 2 例目が要求してから）／kanade は closure seam で sylphya 非依存（依存辺を増やさない）／`default_system_vars()` は退役（stand-in 退役規律・テストは Custom 注入）／ack 機構は `Barrier` 1 本（per-message ack の増殖をしない）。
