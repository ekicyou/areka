# Requirements Document

## Project Description (Input)

areka（x64）が適合対象ゴースト emo2 を「そのまま」起動するには、emo2 の脳 `pasta.dll`（PE Machine 0x14C = x86/32bit）を駆動せねばならないが、x64 プロセスへ in-proc ロードできない。SHIORI を 32bit 別プロセスでホストし IPC で橋渡しする機構（host-32）が M1 唯一の耐力壁であり、本実装の前に**先進坑（pilot・使い捨て）**で実現可能性を一点突破検証し、開発者の go 判定を取る。

成果物はコードではなく**知見（go／違う／直す ＋ 学び）**である。一次記録は `crates/pilot/examples/shiori-host-32/README.md`（3 幕構成）。本先進坑が満たすべき go 基準は (1) x64 から 32bit `pasta.dll` を `load → OnBoot → Value 受領 → unload` の 1 往復成功、(2) 窓を持つ SHIORI に対応する自前メッセージループが helper プロセス側で安定生存（N 秒運転して clean unload）、の 2 点。SAORI は emo2 未使用ゆえ対象外。

二坑モデル規律の正本は `.kiro/steering/two-tunnel.md`。設計判断の正本は `doc/COMPAT_ARCHITECTURE.md`、M1 実物スコープは `doc/emo2-conformance-scope.md`、go 基準の宿主は `.kiro/steering/roadmap.md`。

## Boundary Context

- **In scope（先進坑が観測・検証する振る舞い）**:
  - x64 親プロセス（areka 側相当）と 32bit helper プロセス（SHIORI ホスト）を**別プロセス**で起動し、自前 IPC（メッセージフレーミング・タイムアウト・プロセス生存監視）で結ぶ。
  - 32bit helper が `pasta.dll` を動的ロードし、SHIORI `load(ghostdir)` → `request(OnBoot)` → `unload` のエントリ解決と呼び出しを行う。
  - SHIORI/3.0 リクエスト 1 種（`OnBoot`、初回は `OnFirstBoot` 相当）の組み立て・marshal と、`Value:`（さくらスクリプト本体）の受領・x64 親への返送。
  - 窓を作る SHIORI に備えた helper 側の自前メッセージループの生存確認（N 秒運転 → clean unload）。
  - 検証結果（go／違う／直す ＋ 学び ＋ 日付）を README 3 幕（動機 → 概要・実行法 → 検証結果）に一次記録する。
- **Out of scope（先進坑が扱わない振る舞い）**:
  - **SAORI 同居**（emo2 は DLL が `pasta.dll` 1 個のみ・`saori` 系参照ゼロ → M1 不要）。
  - production 品質のマーシャリング堅牢性、および `OnBoot` 以外の全 SHIORI イベント網羅（`OnSecondChange` / `OnMouseDoubleClick` / `OnChoiceSelectEx` / `OnMouseMove` / `OnClose` 等は本坑 `areka-P0-host32-*` の領分）。
  - charset 多様性（emo2 は UTF-8 のみ。Shift_JIS は里々/YAYA 生態系拡張で後続）。
  - emo2 の脳の中身（`.pasta`/`.lua`/`pasta.toml`/budoux/縦書き）の解釈（すべて `pasta.dll` の腹の中・areka は一切触らない）。
  - SERIKO 描画・さくらスクリプト解釈・バルーン描画（別エンジントラック）。
  - 本坑 host-32 トラックの実装そのもの（先進坑の go 判定**後**に、知見を見て一から綺麗に掘り直す。コピペ donor 流用は禁止）。
- **Adjacent expectations（隣接系への前提・非所有）**:
  - **Upstream（前提とする既存物）**: `crates/shiori-abi`（内部唯一 ABI `IShiori`/`IShioriHost`・HSTRING/UTF-16）が最終橋渡し先の内部契約として存在する。`crates/pilot`（空 lib ＋ examples-only の検疫所構造）が受け皿として存在する。emo2 実物 `pasta.dll`（32bit SHIORI/3.0・UTF-8・SAORI 不使用）が検証ターゲットとして実在する。設計判断の正本は `doc/COMPAT_ARCHITECTURE.md`。
  - **Downstream（この先進坑の go がゲートする本坑）**: `areka-P0-host32-ipc` / `areka-P0-host32-shiori-load` / `areka-P0-host32-request` / `areka-P0-host32-lifecycle`。これらは go 判定後に着手する別物であり、本先進坑はその実装を所有しない。
  - **命綱（不変条件）**: 出荷グラフ上のいかなる production クレート（wintf/dola/areka/shiori-abi）も本先進坑コードに依存してはならない（葉ノード隔離）。
  - **SHIORI3 build/parse の配置と検証粒度（議題 #5 で確定・本坑方向と整合）**: 先進坑でも **32bit helper はバイト proxy に徹し、SHIORI/3.0 リクエスト組立と `Value:` parse は x64 親側**で行う（本坑の x64 過去互換 `IShiori` アダプタのミニチュア。詳細は research.md §5.4）。go 基準(1) は「**x64 親プロセスが `Value:` 文字列を受領・確認できる**」ことで充足とし、内部 ABI `IShiori`（COM）面への接続は本坑 `areka-P0-host32-request` 領分（先進坑では対象外）。
  - **検証前提（go の前提条件・議題 1 で確定）**: go 検証には emo2 ゴースト一式が必要。検証フィクスチャは emo2 配布物 `emo2.nar`（zip 形式）を `crates/pilot/examples/shiori-host-32/fixtures/emo2/` へ展開して用意し、**リポジトリへ取り込む**（ワークツリー/クローンでの**再現性を優先**。emo2 は本リポジトリ作者自作ゴーストゆえライセンス問題なし。トレードオフ＝`pasta.dll` 3.3MB バイナリが履歴に残る点は再現性を優先して受容）。SHIORI `load` に渡す **ghostdir は `fixtures/emo2/ghost/master/`**（`pasta.dll`＝PE Machine 0x014C/32bit ＋ `descript.txt`＝`charset,UTF-8`/`shiori,pasta.dll` の在処）。この配置手順（nar 展開）は README「概要・実行法」の幕に明記する。

## Requirements

### Requirement 1: 32bit helper プロセスのライフサイクル管理

**Objective:** As a 先進坑を駆動する開発者, I want x64 親が 32bit helper プロセスを起動・監視・終了できること, so that x64↔32bit のプロセス境界越えブリッジが成立しているか観測できる

#### Acceptance Criteria

1. When 先進坑が起動される, the host-32 先進坑 shall x64 親プロセスとは別の 32bit helper プロセスを起動する。
2. While helper プロセスが稼働している間, the host-32 先進坑 shall helper プロセスの生存（生死）を監視する。
3. When 検証シーケンスが正常完了した, the host-32 先進坑 shall helper プロセスを clean shutdown（後始末を伴う正常終了）させる。
4. If helper プロセスが予期せず終了した, then the host-32 先進坑 shall その異常を検出し、検証結果として記録可能な形で観測できるようにする。
5. The host-32 先進坑 shall x64 親と 32bit helper のプロセス分離（ターゲット別ビルド）を崩さずに動作する。

### Requirement 2: 自前 IPC による x64↔32bit 往復通信

**Objective:** As a 先進坑を駆動する開発者, I want x64 親と 32bit helper が自前 IPC で確実にメッセージを往復できること, so that プロセス分離以外に解のない 32bit/x64 境界をブリッジできると確証できる

#### Acceptance Criteria

1. When x64 親が helper へリクエストメッセージを送出する, the host-32 先進坑 shall メッセージフレーミングに従ってメッセージ境界を区切って送受信する。
2. When helper が処理結果を x64 親へ返す, the host-32 先進坑 shall その応答を x64 親側で受領できるようにする。
3. If IPC 応答が所定時間内に得られない, then the host-32 先進坑 shall タイムアウトとして扱い、ハングせずに観測可能な失敗として扱う。
4. While IPC 通信が継続している間, the host-32 先進坑 shall 相手プロセスの生存監視を IPC レイヤと併せて維持する。

### Requirement 3: 32bit SHIORI DLL の動的ロードとエントリ解決

**Objective:** As a 先進坑を駆動する開発者, I want 32bit helper が emo2 の `pasta.dll` を動的ロードして SHIORI のエントリを解決できること, so that x64 areka が 32bit `pasta.dll` を駆動できるという耐力壁を実走で潰せる

#### Acceptance Criteria

1. When helper が検証対象ゴーストの起動を指示される, the host-32 先進坑 shall 32bit helper プロセス内で emo2 の `pasta.dll` を動的ロードする。
2. When `pasta.dll` がロードされた, the host-32 先進坑 shall SHIORI の `load` / `unload` / `request` エントリを解決する。
3. When SHIORI `load` をゴーストディレクトリ（ghostdir）を引数に呼び出す, the host-32 先進坑 shall load 呼び出しを実行し、クラッシュせずに完了させる。
4. If `pasta.dll` のロードまたはエントリ解決に失敗した, then the host-32 先進坑 shall その失敗を検証結果として記録可能な形で観測できるようにする。

### Requirement 4: SHIORI/3.0 OnBoot リクエストの組み立てと Value 受領

**Objective:** As a 先進坑を駆動する開発者, I want OnBoot リクエストを SHIORI/3.0 で組み立てて Value を x64 親まで受領できること, so that 1 往復（load→OnBoot→Value→unload）の成功という go 基準の中核を確認できる

#### Acceptance Criteria

1. When helper がリクエストを発行する, the host-32 先進坑 shall SHIORI/3.0 形式の `OnBoot`（初回 `OnFirstBoot` 相当）リクエスト 1 種を組み立てて `request` で `pasta.dll` へ渡す。
2. When `pasta.dll` が応答を返す, the host-32 先進坑 shall 応答から `Value:`（emo2 の起動挨拶さくらスクリプト本体）を取り出して marshal する。
3. When `Value:` が取り出された, the host-32 先進坑 shall その `Value:` を IPC 経由で x64 親プロセスへ返送し、x64 側で受領できるようにする。
4. The host-32 先進坑 shall リクエスト/レスポンスの charset を emo2 の UTF-8 として扱う。
5. When `load → OnBoot → Value 受領 → unload` の 1 往復が完了した, the host-32 先進坑 shall その往復成功を go 基準 (1) の充足として観測・記録できるようにする。

### Requirement 5: 窓持ち SHIORI 対応の自前メッセージループ生存

**Objective:** As a 先進坑を駆動する開発者, I want helper 側の自前メッセージループが窓を作る SHIORI に対して安定生存することを確認できること, so that 窓持ち SHIORI のメッセージループ生存という go 基準を確認できる

#### Acceptance Criteria

1. While 32bit helper が稼働している間, the host-32 先進坑 shall 窓を作る SHIORI に対応する自前メッセージループを helper プロセス側で回し続ける。
2. When 自前メッセージループを N 秒運転する, the host-32 先進坑 shall その間ループを安定して回し続け、その後 clean unload できるようにする。
3. When N 秒運転後に unload を要求する, the host-32 先進坑 shall メッセージループを停止し、SHIORI を clean unload させる。
4. The host-32 先進坑 shall メッセージループ生存と clean unload の成否を go 基準 (2) の充足として観測・記録できるようにする。

### Requirement 6: README 3 幕への知見の一次記録

**Objective:** As a go 判定を下す開発者, I want 検証結果が README 3 幕に一次記録されること, so that 実走知見を見て「本坑をこの方向で掘れる（go）」を人間判断できる

#### Acceptance Criteria

1. The host-32 先進坑 shall 一次記録を `crates/pilot/examples/shiori-host-32/README.md` に 3 幕構成（動機 → 概要・実行法 → 検証結果）で記述する。
2. Where 動機の幕を記述する, the host-32 先進坑 shall 対応する本坑 spec（`areka-P0-host32-*` 群）を名指しして先進坑⟷本坑の traceability を確立する。
3. Where 検証結果の幕を記述する, the host-32 先進坑 shall **go／違う／直す** のいずれかの判定 ＋ 学び ＋ 日付を残す。
4. When go 基準 (1)（1 往復成功）と (2)（メッセージループ生存 → clean unload）の充足状況が確定した, the host-32 先進坑 shall その結果を検証結果の幕に反映する。
5. The host-32 先進坑 shall go 判定そのものを自動化せず、開発者の人間判断に委ねる（README は判断材料の提供に徹する）。

### Requirement 7: 二坑規律（葉ノード隔離・使い捨て品質）の遵守

**Objective:** As a 命綱（可逆性）を守る開発者, I want 先進坑コードが production から構造的に隔離され使い捨て可能であること, so that 誤った方向と分かったときに低コストで引き返せる

#### Acceptance Criteria

1. The host-32 先進坑 shall 探索コードを `crates/pilot/examples/shiori-host-32/`（1 仕様 = 1 フォルダ・`main.rs` 必須）に隔離して配置する。
2. The host-32 先進坑 shall いかなる production クレート（wintf/dola/areka/shiori-abi）も本先進坑コードに依存させない（葉ノード隔離）。
3. Where 起点コードを用意する, the host-32 先進坑 shall `examples/_template/` の雛形をコピーして起点とする。
4. The host-32 先進坑 shall 整形・命名・テストの厳格さを production 品質まで求めず、使い捨て前提の緩い品質基準で進める（ただし葉ノード隔離だけは厳守する）。
5. The host-32 先進坑 shall Rust 2024 を前提とし、helper を 32bit ターゲット・親を x64 とするプロセス分離（32bit/x64 境界）を崩さない。
