# Implementation Plan

- [x] 1. crates/areka-sylphya の雛形とワークスペース依存整備
  - Cargo.toml（依存: thiserror・tracing・toml・areka-actor のみ。上流 areka クレートへの依存は禁止）を作成し、design.md の File Structure Plan どおりの空モジュール群（key.rs／value.rs／asker.rs／vocab/mod.rs・flat.rs・dotted.rs・shiori_resource.rs／mirror.rs／reader.rs／actor.rs／persist/mod.rs・format.rs・io.rs）を配置する
  - lib.rs に crate 正本 rustdoc（掲示板モデル・5 実体層・読み口 2 形の要約）を記述する
  - root Cargo.toml の `[workspace.dependencies]` へ `toml = "0.8"` を hoist する（`members = ["crates/*"]` glob により新クレートは自動でワークスペース参加）
  - `cargo build -p areka-sylphya` が空実装のままコンパイル成功することを確認する（クレート単体のビルド可能性が観測可能な完了条件）
  - _Requirements: 6.4_

- [ ] 2. Core: key モデルと語彙台帳
- [x] 2.1 正準 key モデルと共有語彙型・asker コンテキスト
  - `PropPath`／`PathSeg`／`Selector`（括弧名選択・`.index(ID)`・`.current`・`.count`・数値括弧の 5 形を完全収容）と `parse_dotted` を実装する
  - 台帳共通型 `BackingLayer`（5 値）・`DegradePolicy`（PassThroughRaw／ConsumerDefault／NotFound）・`M1Status`・`SetSemantics`（RuntimeCommand／StoreWrite）を定義する
  - `AskerId`／`AskerContext`（問い合わせ元コンテキスト第一級）を定義する
  - 不正形（空セグメント・括弧不閉・非数値 index）に対し決定論的な `KeyParseError` を返すことを確認する（`parse_dotted` が同一入力に対し常に同一の Ok/Err を返す）
  - _Requirements: 1.3, 2.4, 2.6, 3.6_

- [x] 2.2 (P) フラット語彙台帳
  - フラット 26 トークン全数を `FlatEntry`（token／layer／m1／degrade）として `FLAT_VOCAB` に登録する（username のみ `ConsumerDefault`、他は `PassThroughRaw`）
  - `%*`・`%property[...]` を構文記録専用の `SYNTAX_RECORDS` として保持し（解決対象外）、`\%` は語彙に含めないことを確認する
  - `FLAT_VOCAB.len() == 26` を単体テストで檻化する（件数が観測可能な完了条件）
  - _Requirements: 1.1, 1.6, 3.1_
  - _Boundary: areka-sylphya vocab/flat.rs_

- [x] 2.3 (P) 点付き語彙台帳（ルート枝・汎用名・SET 意味論・ext 予約）
  - 点付きルート枝 10 本を `DOTTED_ROOTS`、汎用プロパティ名 17 種を `GENERIC_PROP_NAMES` として登録する
  - SET 有効群（surface.num／animation.num／seriko.defaultsurface／mousecursor 群／seriko.cursor・tooltip／menu・bind.menu 群）を `SET_EFFECTIVE: &[(&str, SetSemantics)]` として登録する
  - ext 亜枝のイベント名予約 `EXT_EVENT_GET = "property.get"`／`EXT_EVENT_SET = "property.set"` を定義する（発火しない・予約のみ）
  - SET 無効な正準語彙への書込挙動（受理＋警告＋非反映）を `doc/COMPAT_ARCHITECTURE.md` 対応表へ記録する
  - `DOTTED_ROOTS.len() == 10 && GENERIC_PROP_NAMES.len() == 17` と SET 有効群全項目の網羅を単体テストで檻化する
  - _Requirements: 1.2, 3.4, 3.5_
  - _Boundary: areka-sylphya vocab/dotted.rs_

- [x] 2.4 (P) SHIORI Resource 語彙台帳
  - SHIORI Resource 全 159 項目（SHIORI 情報 5・ゴースト情報 43・更新情報 1・オーナードローメニュー画像/文字色群・`*button.caption` 91 種＋同数の `*button.visible`・tooltip 2）を `SHIORI_RESOURCE_IDS` として登録する
  - 特殊エントリ「-」は「ID 未確認」注記付きで保持する（語彙を落とさない）
  - `SHIORI_RESOURCE_IDS.len() == 159` を単体テストで檻化する
  - _Requirements: 1.4_
  - _Boundary: areka-sylphya vocab/shiori_resource.rs_

- [x] 2.5 語彙台帳・key パーサ 決定論単体テスト群
  - セレクタ 5 形の受理と `PropPath` 構造の一致、不正形の `KeyParseError` 決定論を検証する
  - 台帳件数檻（フラット 26／ルート枝 10／汎用名 17／Resource 159／SET 有効群全項目の網羅）を検証する
  - username のみ `ConsumerDefault`、`%*` が解決対象外、ext イベント名が予約のみで発火しないことを検証する
  - すべて x64 純粋テストとして実行可能（実 I/O・実 OS 環境への依存なし）であることを確認する
  - _Depends: 2.1, 2.2, 2.3, 2.4_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.6, 3.1, 3.4, 3.5, 3.6, 9.1, 9.2_

- [ ] 3. Core: 鏡像と SylphyaReader
- [x] 3.1 不変鏡像（per-asker/global 区画）
  - `MirrorImage { epoch: u64, flat_per_asker, flat_global, dotted_global, dotted_per_asker }` を不変値として実装する（設計討議 #1: フラット実導出語彙はゴースト相対ゆえ per-asker へ着地・global は将来の大域語彙用に確保）
  - `SharedMirror`（`RwLock<Arc<MirrorImage>>`）による epoch 交換（copy-on-write・publish 時に新しい `Arc` を構築して swap）を実装する
  - 読みは read lock 内で `Arc` clone するのみとし、他アクター・他スレッドへのブロッキング照会を一切行わないことをコード上の不変量として確認する
  - publish 後に epoch が単調増加すること、同一鏡像 epoch では常に同一の内容が読めることを単体テストで確認する（決定論の観測条件）
  - _Requirements: 2.5, 2.6, 2.7_

- [x] 3.2 SylphyaReader 同期読み口 API
  - `resolve_flat`（asker×名前→`FlatResolution::Value`／台帳縮退政策に従う `Degraded`）を実装する（フラット解決は per-asker→global の順）
  - `resolve_dotted`／`resolve_dotted_str`（`DottedResolution::Value`／`NotFound`）を実装する
  - `talk_snapshot`（鏡像に値が実在するフラット名のみを `BTreeMap<String,String>` として返す・per-talk 凍結の素材）を実装する
  - 解決結果型に値の由来（backing 情報）を一切含めないことを型で保証する
  - `SylphyaReader::resolve_flat`／`resolve_dotted` の呼出がチャネル送受信・ファイル IO・OS 呼出を一切行わないことを確認する（無待機の観測条件）
  - _Depends: 2.1, 2.2, 2.3, 2.4, 3.1_
  - _Requirements: 1.5, 2.1, 2.3, 2.4, 3.1, 3.2, 5.2_

- [x] 3.3 鏡像・読み口 決定論単体テスト群
  - 値あり→`Value`／フラット不在→政策別 `Degraded`／点付き不在→`NotFound` を検証する
  - 別 asker で per-asker 区画（フラット・点付きとも）が混ざらないこと（username・selfname 等が asker ごとに独立）を検証する
  - `talk_snapshot` 取得後に publish しても取得済み写像が不変であること（per-talk 凍結）を検証する
  - 同一鏡像 epoch×同一 asker×同一名→同一結果（決定論）を検証する
  - _Depends: 3.1, 3.2_
  - _Requirements: 2.1, 2.3, 2.5, 2.6, 3.1, 3.2, 9.1, 9.2_

- [ ] 4. Core: 永続層
- [x] 4.1 (P) TOML 直列化形式と寛容読取
  - この永続層は語彙台帳（Major 2）にも鏡像（Major 3）にも依存しない自己完結モジュールであり、Major 1 完了後ただちに並走着手可能（Major 2・3 と並行実施可）
  - `format-version = 1` を持つ TOML スキーマ（`[window."ID"]`／`[balloon-offset."ID"]`／`[boot]`／`[vanish]`、値はすべて文字列）を実装する
  - 寛容読取 3 段（ファイル不在→debug＋全不在／parse 失敗・未知 `format-version`→warn＋全不在・ファイル温存／key 欠落→当該 key のみ不在）を実装する
  - 未知バージョンのファイルを読み込んだ場合に warn ログが出ること（旧形式判別可能）を確認する
  - _Requirements: 6.3, 6.4_
  - _Boundary: areka-sylphya persist/format.rs_

- [x] 4.2 (P) 原子的 PersistIo シーム
  - `PersistIo` トレイト（`read`／`commit`）を定義し、実装として temp 書込→`rename`（Windows は既存宛先を置換）による原子的確定を行う real 実装を作成する
  - テスト用に故障注入可能な fake `PersistIo`（メモリ内・commit 中断シミュレート）を実装する
  - commit 中断時に既存ファイル内容が無傷であることを fake IO で確認する（原子性の観測条件）
  - 暦時計・OS 環境を暗黙に直読しないこと（IO はすべてこのシーム経由）を確認する
  - _Requirements: 6.2, 8.3, 9.2_
  - _Boundary: areka-sylphya persist/io.rs_

- [x] 4.3 層別スコープ・4 key 族の型付けモデルと load/save 編成
  - `PersistScope`（App／Ghost／Shell／Balloon）と `ScopeRoots`（各スコープの `Option<PathBuf>`・呼び出し側供給）を実装する
  - `PersistKey`（WindowPos{scope,axis}／BalloonOffset{scope,axis}／BootCount／VanishCount）と正準 key 文字列（`areka.window.scope(ID).x` 等）への写像を実装する
  - 全スコープの寛容ロード（起動時一括）と、当該スコープの原子的保存（write-through）を編成する `load_scope`／`save_scope` を実装する
  - 4 key 族すべてについて put→load の値往復が元値と一致することを確認する
  - _Depends: 4.1, 4.2_
  - _Requirements: 6.1, 6.5, 6.6_

- [x] 4.4 永続層 決定論単体テスト群
  - 4 key 族全 key の往復値等価（put→load 一致）を検証する
  - 寛容読取 3 段（ファイル不在／parse 失敗／未知バージョン／key 欠落）それぞれで警告＋不在縮退＋起動継続を検証する
  - fake IO で commit 中断（temp 書込失敗・rename 失敗）→既存内容無傷＋error ログを検証する
  - 別 root の Ghost スコープ 2 実体が互いのファイル・鏡像投影を汚さない（スコープ分離）ことを検証する
  - _Depends: 4.3_
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.7, 9.1, 9.2_

- [ ] 5. Core: sylphya アクター
- [x] 5.1 SylphyaMsg envelope と SylphyaCore 純関数中核
  - `SylphyaMsg`（`PublishStatic{asker,flat,dotted}`／`PublishShiori{asker,name,value}`／`Set{asker,key,value}`／`PersistPut{scope,entries,reply}`／`Barrier{reply}`／`Close`）を定義する
  - 判断分岐を純関数中核 `SylphyaCore::apply(msg) -> 効果列` へ寄せる（受信ループは薄い配線のみ）
  - SET コマンドの分類（正準語彙かつ SET 有効→`RuntimeCommand`／正準語彙外の自由 dotted key→`StoreWrite`／SET 無効な正準語彙→`NotSettable`）を実装する
  - `RuntimeCommandSink` トレイトを型予約する（M1 は未登録・実配線なし）
  - `SylphyaCore::apply` が純関数として同一入力→同一効果列を返す（I/O を含まない）ことを確認する
  - _Depends: 3.1, 4.3_
  - _Requirements: 3.3, 3.4, 6.1, 6.7, 8.1_

- [x] 5.2 アクター起動・Publisher・Barrier
  - `spawn_sylphya(SylphyaInit) -> SylphyaParts`（起動時に全スコープを寛容ロードし初期鏡像を構築）を `areka-actor` の 5 規約（inbox／envelope／停止／流量／拡張凍結）に準拠して実装する
  - `SylphyaPublisher`（`publish_static`／`publish_shiori`／`set`／`persist_put`／`barrier`）を実装する
  - `barrier()` 復帰時点でそれ以前に同一送信端から投函した全メッセージが鏡像へ反映済みであること（mpsc FIFO＋直列処理）を確認する
  - アクター死亡（panic）時に publisher 送信が `SendError` として観測でき、reader は最終鏡像で読み続行できることを確認する
  - 同期読み経路（reader）が非同期供給（アクター）に一切ブロックされないこと（大域ロック不在）を確認する
  - _Depends: 5.1_
  - _Requirements: 2.7, 3.3, 6.7_

- [x] 5.3 アクター 決定論単体テスト群
  - SET 分類 3 パターン（RuntimeCommand／StoreWrite／NotSettable）を検証する
  - publish 後の鏡像 swap で epoch が単調増加することを検証する
  - `Barrier` による反映完了フェンスが決定論的に機能することを検証する
  - アクター死亡時の縮退経路（warn＋以降縮退・join での panic 検出）を検証する
  - _Depends: 5.2_
  - _Requirements: 3.3, 3.4, 6.7, 8.1, 9.1, 9.2_

- [ ] 6. Integration: kanade SHIORI リソース照会の座席
- [x] 6.1 (P) リソース許可集合と submit ガード拡張
  - この座席は `ResourceSink` クロージャで疎結合されており areka-sylphya クレートへ依存しない（design 論点1）。Major 2-5 と並走可能
  - イベント檻（`ALLOWED_EVENT_IDS`）とは別族の `ALLOWED_RESOURCE_IDS`（M1: `["username"]`）と `is_allowed_resource_id` を実装する
  - kanade actor の submit 送出ガードを「イベント許可 ∨ リソース許可」へ拡張する（既存イベント檻・許可外拒否は無改変）
  - 許可外リソース ID の送出が従来どおり `ShioriFailure::Internal` で拒否されることを確認する（既存不変量の保存）
  - _Requirements: 4.1_
  - _Boundary: areka-kanade actor.rs, schedule/resources.rs (新規)_

- [x] 6.2 username 照会関数・ResourceSink・boot prefetch 段
  - `resource_username(&ExecutionSnapshot) -> ShioriCall`（`ShioriCall::Get{id:"username"}`・M1 はリテラル `&'static str` 据え置き）を実装する
  - `ResourceOutcome`（Value／NoContent／Failed）と `ResourceSink`（`Box<dyn Fn(&'static str, ResourceOutcome) + Send>`、kanade 構築時注入）を実装する
  - boot 運行表へ prefetch 段（OnInitialize 後・OnFirstBoot 前に 1 回・既存 shiori request 経路をそのまま使用）を挿入する
  - 照会失敗（タイムアウト・IPC 断）時は warn＋`Failed` を sink へ渡し boot を続行することを確認する（起動を殺さない）
  - prefetch 完了時に固定ログ `info!(target:"areka_kanade::resource", id="username", outcome=..., "shiori resource prefetch done")` が必ず 1 回出ることを確認する（R9.3 grep 証跡）
  - username の 204/空値→既定値縮退（定義点は sakura 側に残置）の対応関係を `doc/COMPAT_ARCHITECTURE.md` 対応表へ記録する
  - _Depends: 6.1_
  - _Requirements: 4.1, 9.3_

- [ ] 6.3 kanade 統合テスト群（mock shiori）
  - boot 記録列で username GET が OnInitialize 後・OnFirstBoot 前に 1 回だけ現れることを検証する
  - 200→sink に `Value`・204→`NoContent`・タイムアウト→`Failed`＋boot 続行を検証する
  - 許可外リソース ID 送出の拒否と egress スイープ檻の許可語彙更新を検証する
  - _Depends: 6.2_
  - _Requirements: 4.1, 4.2, 9.1, 9.2, 9.3_

- [ ] 7. (P) parsers: sakura.name2 転記拡張
  - `GhostNames` 構造体へ `sakura_name2: Option<String>` を additive 追加する
  - `resolve.rs` の names 構築に `map.get("sakura.name2").cloned()` を 1 行追加する（忠実転記のみ・展開・推測をしない）
  - 宣言あり→`Some(値)`・宣言なし→`None` の決定論単体テストケースを既存 resolve テストへ追加する
  - descript に `sakura.name2` が定義されたゴーストで `GhostNames.sakura_name2` が `Some(値)` として観測できることを確認する
  - _Requirements: 4.4_
  - _Boundary: areka-parsers package/model.rs, package/resolve.rs_

- [ ] 8. Integration: ghost 結線
- [ ] 8.1 derive_flat_statics 純関数（selfname 系フォールバック）
  - `derive_flat_statics(&GhostNames) -> Vec<(String,String)>` を純関数として実装する: `sakura.name`→selfname／`sakura.name2`→selfname2（未定義時は積まない＝素通し縮退）／`kero.name`→keroname（未定義時は `sakura.name` へフォールバック、両方未定義なら積まない）
  - selfname2 未定義時の素通し縮退、keroname の SSP 互換フォールバック規則を `doc/COMPAT_ARCHITECTURE.md` 対応表へ記録する
  - kero あり／kero なし＋sakura あり／両方なしの 3 分岐と、name2 の有無を決定論単体テストで検証する（descript 実値解決の決定論檻）
  - _Depends: 7_
  - _Requirements: 4.3, 4.4, 4.5, 9.4_

- [ ] 8.2 sylphya 結線: spawn・スコープ解決・静的/baseware publish・SystemVarWiring 差替・shutdown 段
  - `GhostBootOptions.system_vars` を `SystemVarWiring`（`FromSylphya`／`Custom(SystemVarSource)`）へ置換し、`app_profile_dir: Option<PathBuf>` フィールドを追加し、`GhostHandles`/`GhostParts` へ sylphya ハンドル用フィールドを追加する（後続タスクが依存する構造変更を本タスクで先に確定する）
  - mount 解決後に `spawn_sylphya`（roots: ghost スコープ＝`<MountModel.shiori.dir>/profile/areka/`・shell スコープ＝`<ShellMount.dir>/profile/areka/`・app スコープ＝`app_profile_dir`・balloon＝None）を呼び出す
  - `derive_flat_statics(&mount.names)` の結果と baseware 2 項（`baseware.name`＝`"areka"`／`baseware.version`＝`env!("CARGO_PKG_VERSION")`）を `PublishStatic`（自 `AskerId`＝`MountModel.shiori.dir` 由来の正準文字列）で publish する
  - kanade spawn 時に `ResourceSink`（publish_shiori 呼出＋`barrier()` で反映完了を待ってから返るクロージャ）を注入する
  - `default_system_vars()` を退役し（stand-in 退役規律）、`main.rs`／`emo2_boot/mod.rs`／`emo2_boot/spine.rs` の 3 箇所を `SystemVarWiring::FromSylphya` へ差替え、app スコープ root（既定＝実行ファイル隣接 `profile/areka/`・`AREKA_PROFILE_DIR` 環境変数で上書き可）を供給する
  - 既存 shutdown 段の後に sylphya `Close`＋join 段を追加する
  - boot 完了時点で鏡像に静的構成層＋（成功時）username が反映済みであり、provider スナップショットが `FromSylphya`（reader の `talk_snapshot`）経由で鏡像由来になっていることを確認する（sakura の契約は無改変のまま）
  - _Depends: 5.2, 6.2, 8.1_
  - _Requirements: 2.2, 4.2, 5.1, 6.5, 7.1, 8.2_

- [ ] 8.3 既存テスト呼出面（約 20 箇所）の Custom 注入更新
  - `default_system_vars()`／`system_vars:` 構築子に依存する in-crate テスト（runtime.rs 約 5 箇所・dispatcher.rs 約 5 箇所）を `SystemVarWiring::Custom` 注入へ更新する
  - tests/ghost 統合テスト（spine_e2e_test 約 5 箇所・inproc_e2e_test 2 箇所・snapshot_capture_test・real_pasta_test ほか）を同様に更新する
  - 更新後、これら既存テストが従来どおりの意図（陳腐化ではなく生きているテストの更新）で green になることを確認する
  - _Depends: 8.2_
  - _Requirements: 7.1, 9.1_

- [ ] 8.4 ghost 統合テスト群
  - boot 後に reader で selfname 系・baseware が実値解決されることを検証する
  - shutdown 全段（sylphya 段含む）が成功することを検証する
  - mock shiori が username=Value を返す構成で、boot talk のスナップショットに当該値が必ず入る（barrier フェンスの検証・prefetch→初回 talk 順序のレース非依存）ことを検証する
  - _Depends: 8.2, 8.3_
  - _Requirements: 2.1, 2.2, 4.2, 4.3, 4.4, 4.5, 5.1, 7.1, 9.1, 9.2, 9.4_

- [ ] 9. Integration: ShioriHostSink 統合
- [ ] 9.1 (P) HashMap ストア撤去・with_sylphya 構築
  - この統合は sylphya アクター（Major 5）完了のみに依存し、ghost 結線（Major 8）とは異なるファイル（bin crate 内の shiori_host.rs）を扱うため並走可能
  - `ShioriHostSink.properties: Mutex<HashMap<String, HSTRING>>` を撤去する
  - `ShioriHostSink::with_sylphya(reader, publisher, asker)` コンストラクタを実装する
  - sink を組み立てる各所（shiori_session 系）が bin 内で `spawn_sylphya`（App root のみ）を行い、セッション固有の `AskerId` を与えることを確認する
  - _Depends: 5.2_
  - _Requirements: 7.2, 7.3_
  - _Boundary: crates/areka/src/shiori_host.rs_

- [ ] 9.2 GetProperty/SetProperty 委譲実装
  - `GetProperty` を `resolve_dotted_str` への委譲（`Value`→HSTRING out 書込／`NotFound`→`SHIORI_E_PROPERTY_NOT_FOUND`・out 未書込）として実装する（同期即答・既存の最小ロック区間・再入規約を維持）
  - `SetProperty` を `publisher.set(asker, key, value)` の投函＋即 `Ok(())` 返却として実装する
  - `set_property_value` 充填口を publisher 委譲の薄いラッパとして存続させ、`barrier()` 委譲もテスト用に公開する
  - HSTRING⇄String 変換が sink 境界に閉じている（sylphya crate は String のみ扱う）ことを確認する
  - _Depends: 9.1_
  - _Requirements: 7.2_

- [ ] 9.3 sink テスト更新・本番呼出列の順序依存実測
  - 既存テスト (d)(e) を `barrier()` 駆動（set→barrier→get の決定論列）へ更新する
  - 欠落 key・`Get` 実装内からの再入・別スレッド set＋barrier の檻を維持することを確認する
  - 本番呼出列（shiori_session 系初期化列）に set→直後 Get の順序依存が無いことを実測確認する（依存が見つかった場合は `set_property_value` 充填ラッパ内 barrier を初期化列に限り適用する）
  - _Depends: 9.2_
  - _Requirements: 7.2, 9.1, 9.2_

- [ ] 10. Validation: 横断規律・最終検証
- [ ] 10.1 固定ログイベントと無音失敗監査
  - design.md の Monitoring 節に定義された固定ログイベント（kanade prefetch `info!`・sylphya actor `debug!`・ghost provider `debug!`）が実装どおりに出力されることを確認する
  - 全エラー経路（解決系・SHIORI 照会系・永続系・アクター系・SET 系）が無音失敗せず、必ず warn!/error! と定義済み縮退へ到達することを監査する
  - 環境変数制御箇所（`AREKA_PROFILE_DIR` 等）がすべて `AREKA_` 名前空間であることを確認する
  - _Depends: 6.2, 8.2, 9.2_
  - _Requirements: 8.1, 8.2, 9.3_

- [ ] 10.2 cargo test --workspace green（DoD ゲート）
  - i686 host-32 成果物の事前ビルド後、`cargo test --workspace` が exit 0 で完了することを確認する
  - すべての決定論単体・統合テストが x64 純粋テストとして実 SHIORI・実ファイルシステム障害・実 OS 環境に依存せず実行できることを確認する
  - _Depends: 2.5, 3.3, 4.4, 5.3, 6.3, 7, 8.4, 9.3, 10.1_
  - _Requirements: 9.1, 9.2_

- [ ] 10.3 実機（emo2）サインオフ
  - `AREKA_APP_SMOKE_EXIT_MS` による有界自動終了で emo2 撫で talk を実行する
  - `RUST_LOG` 出力を grep し、`shiori resource prefetch done` かつ `outcome="no_content"` が記録されていることを確認する（実 SHIORI 照会経路を通った証跡）
  - バルーンに生 `%username` が露出せず既定値表示となっていることを目視確認する
  - _Depends: 10.2_
  - _Requirements: 9.3_

## Implementation Notes

- 3.2: 点付き key の正準文字列形は `PropPath::to_canonical_string()`（key.rs）が唯一の権威。鏡像の dotted 区画 key・reader 解決・**Task 5.x の publish/persist の dotted key 格納**は必ずこの同一 stringifier（または既に正準なリテラル）を使うこと。往復不一致は NotFound を招く。
- 4.1/4.3: tracing ログ捕捉テストは必ず `crate::test_log_capture::capture`（interest-keeper 常駐＝プロセスグローバル `set_global_default(registry())` で callsite `Interest::never` 焼き付きを根絶）経由で書くこと。素の `with_default` 単独は並列 `cargo test` 下で ~1/10-1/20 偽赤。kanade `schedule/log_capture.rs` と同機構（sylphya は最下層ゆえ kanade 非依存で複製）。
- 6.2: `spawn_kanade` は resource_sink 引数を加え **4 引数**化した。areka-ghost（runtime.rs・ghost 統合テスト）は 3 引数のままゆえ **Task 8.2 着手前はワークスペース全体ビルドが赤**（kanade 単体は緑）。8.2 が実 sink（publish_shiori＋barrier クロージャ）を注入して解消する。DoD ゲート `cargo test --workspace`（10.2）は 8.2/8.3 完了まで赤で正常。
