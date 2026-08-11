//! サーフェス状態モジュール（状態層・per-scope の現 surface 状態と静的 bind 集合の所有者）。
//!
//! 話者スコープ（`ActorKey`）ごとに独立した現在の表示状態（表示中 surface / 非表示）を
//! `HashMap` で保持し（`ActorKey` は `Hash+Eq` を持つが `Ord` を持たないため HashMap・design
//! 状態層）、shell descript 由来の静的 bind 集合を per-scope マップと**同居**して保持する
//! （要件 4.4）。加えて per-scope の動的着せ替え集合 `dynamic_binds` を保持し、Show 発行の
//! bind 供給源はこの動的集合を優先し、未束縛 scope は静的既定集合へフォールバックする
//! （要件 3.1・`mayuna-compose` が予約シームを消費）。
//!
//! [`ScopeStates::apply`] は 1 cue = 1 scope の更新をトランザクション境界とし、状態が実際に
//! 変化したときだけ発行すべき [`DisplayCommand`] を [`ApplyOutcome::Changed`] で返す。状態
//! 不変なら [`ApplyOutcome::Unchanged`] を返して再発行を抑止する（冪等ガード・要件 3.4・DD8）。
//! 実際の発行（単一発行点）は後続タスク（アクター層 `emit_display`）の責務。

use std::collections::HashMap;

use areka_emo_compose::{BindSet, PatternState};
use areka_sakura::ActorKey;

use crate::bind::accumulate;
use crate::output::DisplayCommand;
use crate::resolve::SurfaceTarget;

/// あるスコープの現 surface 状態（要件 3.1/3.3/3.4）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScopeState {
    /// 表示中の surface id（要件 3.1/3.5）。
    Shown(u32),
    /// 非表示（`\s[-1]` 相当・要件 3.3/3.4）。
    Hidden,
}

/// `apply` の適用結果（状態変化の有無を発行層へ伝える冪等ガード用）。
///
/// - [`ApplyOutcome::Changed`]: 状態が実際に変化した。同梱の [`DisplayCommand`] を発行すべき。
/// - [`ApplyOutcome::Unchanged`]: 状態不変＝再発行不要（要件 3.4・DD8）。
#[derive(Clone, Debug, PartialEq)]
pub enum ApplyOutcome {
    /// 発行すべき指令（状態が変化した・要件 5.1/5.2）。
    Changed(DisplayCommand),
    /// 状態不変＝発行しない（冪等・要件 3.4）。
    Unchanged,
}

/// [`ScopeStates::apply_bind`] の適用結果（着せ替え変化に応じた表示発行の判定・要件 3.5/3.6/3.8）。
///
/// 動的 bind の適用は「per-scope の結果 bind 集合」の更新であり、表示発行が要るかは対象 scope の
/// **シェル面**表示状態で決まる（D5）。[`ApplyOutcome`] を鏡映しつつ、非表示 scope での「状態のみ
/// 更新」を第 3 の区分 [`BindApplyOutcome::StateOnly`] で表す。
///
/// - [`BindApplyOutcome::Changed`]: 集合が変化し、かつ対象 scope が表示中（`Shown(id)`）→ 現在の
///   シェル surface id を載せた [`DisplayCommand::Show`] を発行すべき（要件 3.5・D5）。
/// - [`BindApplyOutcome::StateOnly`]: 集合は変化したが対象 scope が非表示（`Hidden`）または未知
///   （未表示）→ 発行せず状態のみ更新し、次の Show へ新集合を保留する（D5・`\s[-1]` 意味論を
///   破らないため非表示 scope へ Show を強制しない）。
/// - [`BindApplyOutcome::Unchanged`]: 適用結果が直前と同値の集合 → 状態も書き込まず発行もしない
///   （冪等・要件 3.6・D9）。
#[derive(Clone, Debug, PartialEq)]
pub enum BindApplyOutcome {
    /// 表示中 scope での集合変化 → 現シェル surface id を載せた Show を発行すべき（要件 3.5・D5）。
    Changed(DisplayCommand),
    /// 非表示／未知 scope での集合変化 → 発行なし・状態のみ更新（次 Show へ保留・D5）。
    StateOnly,
    /// 結果集合が直前と同値 → 状態書き込みも発行もなし（冪等・要件 3.6・D9）。
    Unchanged,
}

/// 表示エントリの面種名前空間（シェル面／バルーン面）。
///
/// [`ScopeStates`] が保持する 2 つの per-scope 表示 map（シェル `scopes`／バルーン `balloon`）に
/// 整合する **surface ID 名前空間の区別**であり、能力の仕切りではない（design「面種非依存・裁定 (a)」）。
/// per-(scope, slot) の [`PatternState`] 表 `pattern_states` のキー成分・[`LoopRuntime`] 側の再生状態
/// キー（後続タスク）と共有し、両 slot は同一コードパスで評価される。`HashMap` キーに用いるため
/// `Copy`＋`Hash`＋`Eq` を導出する。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Slot {
    /// シェル面（`scopes` map・bind ゲート対象）。
    Shell,
    /// バルーン面（`balloon` map・M-boot は着せ替え非在）。
    Balloon,
}

/// [`ScopeStates::commit_pattern`] の適用結果（pattern 進行状態変化に応じた表示発行の判定・要件 6.1/6.2）。
///
/// [`commit_bind`] を鏡映した冪等ガード（同値なら書き込まず [`PatternApplyOutcome::Unchanged`]）＋
/// 表示中なら現 surface を新 pattern で再合成させる Show/ShowBalloon を [`PatternApplyOutcome::Changed`]
/// で返す。非表示／未知 slot は評価自体が走らない（表示中ゲート・2.1）が、防御的に非発行（`Unchanged`）で
/// 扱う（`apply_bind` の [`BindApplyOutcome::StateOnly`] に相当する第 3 区分は設けない——commit_pattern は
/// 表示中 slot にのみ到達する契約ゆえ）。
///
/// [`commit_bind`]: ScopeStates::commit_bind
#[derive(Clone, Debug, PartialEq)]
pub enum PatternApplyOutcome {
    /// 表示中 slot での pattern 変化 → 現 surface を新 pattern で再合成させる Show/ShowBalloon（要件 6.1）。
    Changed(DisplayCommand),
    /// 同値（冪等・要件 6.2）または非表示／未知 slot（防御的非発行）→ 発行なし。
    Unchanged,
}

/// per-scope surface 状態・静的 bind 集合・per-scope 動的 bind 集合の所有者（状態層）。
///
/// per-scope マップと静的 `BindSet` を同一構造体に同居させ（要件 4.4）、per-scope の動的
/// 着せ替え集合 `dynamic_binds` を併せて保持する（要件 3.1・予約シームの消費）。`static_binds`
/// は [`ScopeStates::new`] で一度だけ設定して以後不変であり、動的 bind 未適用の scope に対する
/// **既定（初期値）集合**として機能する（Show 発行の bind 供給源は [`ScopeStates::current_binds`]
/// が `dynamic_binds` 優先・不在時 `static_binds` フォールバックで決める）。本ユニットは動的 bind の
/// 変更 API（`apply_bind`）を持たず、`dynamic_binds` は空のまま起動する（後続タスクが書き込む）。
pub struct ScopeStates {
    /// 話者スコープごとの現 surface 状態（`ActorKey` は `Ord` 非対応ゆえ HashMap・design）。
    scopes: HashMap<ActorKey, ScopeState>,
    /// バルーン面のスコープごとの現 surface 状態（シェル面 `scopes` とは**別 map** で同居・
    /// 要件 4.3/4.6）。同型 [`ScopeState`] だが独立し、`apply_balloon` のみが変更する。
    /// シェル面と相互に不変（`apply()` は触らない・`static_binds` はバルーンでは使わない）。
    balloon: HashMap<ActorKey, ScopeState>,
    /// 静的 bind 集合（起動時に一度だけ解決・以後不変・要件 4.2/4.3/4.4）。
    ///
    /// 動的 bind 未適用の scope に対する既定（初期値）供給源でもある（要件 3.1・
    /// [`ScopeStates::current_binds`] のフォールバック先）。供給元は shell descript の
    /// `default,1`（`build_static_bindset`）。
    static_binds: BindSet,
    /// per-scope の動的着せ替え集合（要件 3.1）。エントリを持つ scope はその集合を、持たない
    /// scope は `static_binds`（既定・初期値）を Show の bind 供給源とする（[`current_binds`]）。
    ///
    /// 本ユニットでは常に空のまま（書き込む `apply_bind` は後続タスク）。空である限り
    /// [`current_binds`] は全 scope で `static_binds` を返し、Show 発行は従来と byte 同値
    /// （要件 3.8 非退行）。[`current_binds`]: ScopeStates::current_binds
    dynamic_binds: HashMap<ActorKey, BindSet>,
    /// per-(scope, slot) の pattern 進行状態（`dynamic_binds` と同居・要件 5.1）。エントリを持つ
    /// (scope, slot) はその [`PatternState`] を、持たない (scope, slot) は空（[`PatternState::default`]）を
    /// Show/ShowBalloon の pattern 供給源とする（[`current_pattern`]）。SERIKO ループ評価
    /// （後続タスク）が `commit_pattern` 経由で書き込み、surface 切替時に当該 slot が空へリセットされる。
    /// 空である限り Show/ShowBalloon の pattern は空＝ループ不活性時は従来と観測等価（要件 5.4）。
    ///
    /// [`current_pattern`]: ScopeStates::current_pattern
    pattern_states: HashMap<(ActorKey, Slot), PatternState>,
}

impl ScopeStates {
    /// 静的 bind 集合を受けて空のスコープ状態を構築する（要件 4.2）。
    ///
    /// `static_binds` は task 1.2/2.4 で解決される bindgroup default 由来の `BindSet` を想定し、
    /// 本構造体はそれを不変に保持する（切替 API を持たない・要件 4.3）。
    pub fn new(static_binds: BindSet) -> Self {
        Self {
            scopes: HashMap::new(),
            balloon: HashMap::new(),
            static_binds,
            // 空で起動。動的 bind の書き込みは後続タスク（apply_bind）の責務。既定 on/off の
            // 「初期値」はここを pre-populate するのではなく current_binds のフォールバックで実現。
            dynamic_binds: HashMap::new(),
            // 空で起動。pattern の書き込みは SERIKO ループ評価（commit_pattern）の責務。未格納の
            // (scope, slot) は current_pattern が空を返す（従来と観測等価・要件 5.4）。
            pattern_states: HashMap::new(),
        }
    }

    /// 1 つの解決済み surface 指令を対象スコープへ適用する（1 cue = 1 scope・design）。
    ///
    /// 対象 `scope` のエントリのみ触り、他スコープの状態は一切変更しない（要件 3.1/3.2）。
    /// 戻り値で状態変化の有無を返し、冪等ガード（要件 3.4）を呼び手（発行層）へ委ねる。
    ///
    /// 分岐（design Postconditions）:
    /// - [`SurfaceTarget::Show(id)`]:
    ///   - 現状が既に `Shown(id)`（同一 id）→ [`ApplyOutcome::Unchanged`]（冪等・再発行しない・
    ///     要件 3.4/DD8）。
    ///   - それ以外（未知 scope・`Hidden`・別 id を表示中）→ 状態を `Shown(id)` に設定（未知 scope
    ///     は新規挿入・design「未知 scope への Show は新規挿入」）し、
    ///     [`ApplyOutcome::Changed`]`(DisplayCommand::Show { scope, surface_id: id, binds })` を返す
    ///     （scope の現在 bind 集合 [`current_binds`] を同梱・要件 5.1。動的 bind 未適用なら
    ///     `static_binds`＝従来同値・要件 3.8）。[`current_binds`]: ScopeStates::current_binds
    /// - [`SurfaceTarget::Hide`]:
    ///   - 現状が既に `Hidden` → [`ApplyOutcome::Unchanged`]（非表示保持・要件 3.4）。
    ///   - それ以外（表示中、または未知 scope）→ 状態を `Hidden` に設定し
    ///     [`ApplyOutcome::Changed`]`(DisplayCommand::Hide { scope })` を返す（要件 5.2）。
    ///     未知 scope への `Hide` は先行 `Hidden` 状態を持たないため「変化」とみなし Hide を発行する
    ///     （通常運用ではスコープは未設定から始まり、明示的な非表示指定は 1 度発行して観測可能に
    ///     する方が下流にとって安全。design「未知 scope への Show は新規挿入」と整合の判断）。
    /// - [`SurfaceTarget::Unresolved`]: 呼び手が apply 前に skip するのが正規経路（design
    ///   「Unresolved は apply に渡さない」）。防御的に、状態を変更せず [`ApplyOutcome::Unchanged`]
    ///   を返す（panic せず・発行もしない）。
    pub fn apply(&mut self, scope: &ActorKey, target: SurfaceTarget) -> ApplyOutcome {
        match target {
            SurfaceTarget::Show(id) => {
                // 冪等: 既に同一 surface を表示中なら再発行しない（要件 3.4/DD8）。
                if self.scopes.get(scope) == Some(&ScopeState::Shown(id)) {
                    return ApplyOutcome::Unchanged;
                }
                // 未知 scope は新規挿入・Hidden/別 id は上書き（要件 3.1/3.5）。
                self.scopes.insert(scope.clone(), ScopeState::Shown(id));
                // surface 切替（別 id への Changed）で当該シェル slot の格納 pattern を空へリセット。
                // 新 surface のコマは未生成ゆえ空 pattern で発行し、次の commit_pattern の baseline を
                // 空にする（design「ScopeStates 拡張」・現在コマは新面のアニメで再構成される）。
                self.pattern_states.remove(&(scope.clone(), Slot::Shell));
                ApplyOutcome::Changed(DisplayCommand::Show {
                    scope: scope.clone(),
                    surface_id: id,
                    // scope の現在 bind 集合を同梱（動的 bind 不在なら static_binds＝非退行・R3.8）。
                    binds: self.current_binds(scope).clone(),
                    // pattern 進行状態は cue 由来 Show では空（ループ非活性＝従来と観測等価・R5.4）。
                    // 直上で格納 pattern をクリア済み＝空 pattern と一貫（実 pattern は commit_pattern 経由）。
                    pattern: PatternState::default(),
                })
            }
            SurfaceTarget::Hide => {
                // 非表示保持: 既に Hidden なら再発行しない（要件 3.4）。
                if self.scopes.get(scope) == Some(&ScopeState::Hidden) {
                    return ApplyOutcome::Unchanged;
                }
                // 表示中→非表示、または未知 scope→非表示（要件 3.3）。
                self.scopes.insert(scope.clone(), ScopeState::Hidden);
                ApplyOutcome::Changed(DisplayCommand::Hide {
                    scope: scope.clone(),
                })
            }
            // 呼び手が先に skip する（正規経路）。防御的に無変更・無発行（要件 6.1）。
            SurfaceTarget::Unresolved => ApplyOutcome::Unchanged,
        }
    }

    /// 1 つの解決済みバルーン面指令を対象スコープへ適用する（[`apply`] の鏡映・要件 4.3/4.6）。
    ///
    /// シェル面 `scopes` とは独立した `balloon` map のみを触り、`scopes`・`static_binds` には
    /// 一切干渉しない（相互独立・要件 4.6）。1 cue = 1 scope をトランザクション境界とし、状態が
    /// 実際に変化したときだけ発行すべき [`DisplayCommand`] を [`ApplyOutcome::Changed`] で返す。
    ///
    /// 発行する指令は [`DisplayCommand::ShowBalloon`]／[`DisplayCommand::HideBalloon`] で、シェル面
    /// [`DisplayCommand::Show`] と異なり **bind を同梱しない**（M-boot にバルーン着せ替えは存在
    /// しない・要件 4.1）。
    ///
    /// 分岐（design「seriko バルーン面契約」Postconditions・[`apply`] の balloon map への鏡映）:
    /// - [`SurfaceTarget::Show(id)`]:
    ///   - 現状が既に `Shown(id)`（同一 id）→ [`ApplyOutcome::Unchanged`]（冪等・再発行しない・
    ///     要件 4.3/DD8）。
    ///   - それ以外（未知 scope・`Hidden`・別 id を表示中）→ 状態を `Shown(id)` に設定し
    ///     [`ApplyOutcome::Changed`]`(DisplayCommand::ShowBalloon { scope, surface_id: id })` を返す
    ///     （**binds なし**・要件 4.1）。
    /// - [`SurfaceTarget::Hide`]:
    ///   - 現状が既に `Hidden` → [`ApplyOutcome::Unchanged`]（非表示保持・要件 4.3）。
    ///   - それ以外（表示中、または未知 scope）→ 状態を `Hidden` に設定し
    ///     [`ApplyOutcome::Changed`]`(DisplayCommand::HideBalloon { scope })` を返す（要件 4.2）。
    /// - [`SurfaceTarget::Unresolved`]: 呼び手（actor）が `NameForm`／`Invalid` を手前で log＋skip
    ///   するのが正規経路。防御的に、状態を変更せず [`ApplyOutcome::Unchanged`] を返す。
    pub fn apply_balloon(&mut self, scope: &ActorKey, target: SurfaceTarget) -> ApplyOutcome {
        match target {
            SurfaceTarget::Show(id) => {
                // 冪等: 既に同一バルーン面を表示中なら再発行しない（要件 4.3/DD8）。
                if self.balloon.get(scope) == Some(&ScopeState::Shown(id)) {
                    return ApplyOutcome::Unchanged;
                }
                // 未知 scope は新規挿入・Hidden/別 id は上書き（要件 4.1）。
                self.balloon.insert(scope.clone(), ScopeState::Shown(id));
                // surface 切替でバルーン slot の格納 pattern を空へリセット（シェル面と同一経路・裁定 (a)）。
                self.pattern_states.remove(&(scope.clone(), Slot::Balloon));
                ApplyOutcome::Changed(DisplayCommand::ShowBalloon {
                    scope: scope.clone(),
                    surface_id: id,
                    // cue 由来のバルーン面 Show は空 pattern（直上でクリア済みと一貫・R5.4）。
                    pattern: PatternState::default(),
                })
            }
            SurfaceTarget::Hide => {
                // 非表示保持: 既に Hidden なら再発行しない（要件 4.3）。
                if self.balloon.get(scope) == Some(&ScopeState::Hidden) {
                    return ApplyOutcome::Unchanged;
                }
                // 表示中→非表示、または未知 scope→非表示（要件 4.2）。
                self.balloon.insert(scope.clone(), ScopeState::Hidden);
                ApplyOutcome::Changed(DisplayCommand::HideBalloon {
                    scope: scope.clone(),
                })
            }
            // 呼び手が先に skip する（正規経路）。防御的に無変更・無発行（要件 4.5）。
            SurfaceTarget::Unresolved => ApplyOutcome::Unchanged,
        }
    }

    /// 静的（既定）bind 集合を返す（不変・要件 4.2/4.3/4.4）。
    ///
    /// これは per-scope 動的 bind の**既定フォールバック源**（初期値）であり、per-scope の
    /// 現在集合は [`current_binds`] で引く。本アクセサは常に `new` で渡した静的集合を返す。
    ///
    /// [`current_binds`]: ScopeStates::current_binds
    pub fn binds(&self) -> &BindSet {
        &self.static_binds
    }

    /// 対象 scope の現在の bind 集合を返す（要件 3.1）。
    ///
    /// `dynamic_binds` にエントリを持つ scope はその動的集合を、持たない scope は静的既定集合
    /// `static_binds`（起動直後 default on/off を初期値とする）を返す純粋な読み取り。Show 発行の
    /// bind 供給源はこのアクセサであり、動的 bind が未適用（`dynamic_binds` が空）の間は全 scope で
    /// `static_binds` を返す＝従来の発行結果と byte 同値（要件 3.8 非退行）。
    pub fn current_binds(&self, scope: &ActorKey) -> &BindSet {
        match self.dynamic_binds.get(scope) {
            Some(binds) => binds,
            None => &self.static_binds,
        }
    }

    /// 対象 scope の動的 bind 集合へ 1 件の on/off を積算し、表示発行の要否を判定する
    /// （要件 3.4/3.5/3.6/3.8・D5/D9）。
    ///
    /// 手順:
    /// 1. 現在集合 [`current_binds`] に対し `accumulate(id, on)` で新集合を求める（純関数・積算保持）。
    /// 2. **冪等ガード（要件 3.6・D9）**: 新集合が現在集合と同値なら状態を書き込まず
    ///    [`BindApplyOutcome::Unchanged`] を返す（発行なし）。同値判定の単位は結果 `BindSet` の
    ///    同値（`BindSet` は昇順 dedup の正準形ゆえ順序・重複に不感・D9）。
    /// 3. 変化があれば `dynamic_binds` へ新集合を書き込む（状態更新・要件 3.4 積算保持）。
    /// 4. **発行判定（D5）**: 対象 scope の**シェル面**状態 `scopes` を見て、
    ///    - `Shown(sid)` → [`BindApplyOutcome::Changed`]`(Show{ scope, surface_id: sid, binds: 新集合 })`
    ///      （現 surface を新しい着せ替え集合で再合成させる・要件 3.5）。
    ///    - `Hidden` または未知（`None`）→ [`BindApplyOutcome::StateOnly`]（発行せず状態のみ更新。
    ///      非表示 scope へ Show を強制すると `\s[-1]` 意味論を破るため・D5。新集合は次の Show 発行時に
    ///      [`current_binds`] 経由で載る）。
    ///
    /// **非退行（要件 3.8）**: 本メソッドは `dynamic_binds` のみを書き込み、シェル面 `scopes`・
    /// バルーン面 `balloon` の状態機械には一切干渉しない（[`apply`]／[`apply_balloon`] の遷移は不変）。
    ///
    /// [`current_binds`]: ScopeStates::current_binds
    /// [`apply`]: ScopeStates::apply
    /// [`apply_balloon`]: ScopeStates::apply_balloon
    pub fn apply_bind(&mut self, scope: &ActorKey, id: u32, on: bool) -> BindApplyOutcome {
        // (1) 現在集合から新集合を積算（純関数）。
        let new = accumulate(self.current_binds(scope), id, on);
        // (2)-(4) 冪等ガード → 状態更新 → 発行判定は排他置換と共通（commit_bind に集約）。
        self.commit_bind(scope, new)
    }

    /// 排他カテゴリ（mustselect／非宣言既定）の排他置換を対象 scope へ適用する（bindopt 2.1/3.1）。
    ///
    /// 新集合＝`(現在集合 − category_ids) ∪ {target_id}`。同カテゴリの旧パーツ（`category_ids`）を
    /// すべて外したうえで対象パーツ `target_id` を有効化する（同カテゴリ内は高々 1 パーツ有効・
    /// ukadoc 正典の排他選択挙動）。呼び手（actor）はカテゴリの選択ポリシーが `multiple` 宣言以外
    /// ——mustselect（bindopt 3.1）と非宣言の既定（bindopt 2.1）——の着衣でこの経路を選ぶ。本メソッド
    /// 自身はポリシーを解釈せず、渡された `category_ids` をそのまま外す。`target_id` 自身が
    /// `category_ids` に含まれても、フィルタで一旦除いた後
    /// `chain` で再付与するため最終的に必ず有効になる（結果集合に含まれる）。
    ///
    /// 冪等ガード・状態更新・発行判定（Changed/StateOnly/Unchanged）は [`apply_bind`] と完全同型
    /// （`commit_bind` に集約）。`dynamic_binds` のみを書き込み、シェル面 `scopes`・バルーン面
    /// `balloon` の状態機械には一切干渉しない（要件 3.8・R3.8）。同一 target の再適用は結果集合が
    /// 直前と同値になり [`BindApplyOutcome::Unchanged`]（冪等・D9）。
    ///
    /// [`apply_bind`]: ScopeStates::apply_bind
    pub fn apply_bind_exclusive(
        &mut self,
        scope: &ActorKey,
        category_ids: &[u32],
        target_id: u32,
    ) -> BindApplyOutcome {
        // (1) 排他置換: 同カテゴリ全 ID を外し（target 自身も一旦除去）、target を付与（BindSet が昇順 dedup）。
        let current = self.current_binds(scope);
        let new = BindSet::from_ids(
            current
                .ids()
                .iter()
                .copied()
                .filter(|id| !category_ids.contains(id))
                .chain(std::iter::once(target_id)),
        );
        // (2)-(4) 冪等ガード → 状態更新 → 発行判定は加算/除去と共通（commit_bind に集約）。
        self.commit_bind(scope, new)
    }

    /// bind 新集合を対象 scope へコミットし表示発行の要否を判定する共通後段（D5/D9・R3.8）。
    ///
    /// [`apply_bind`]（加算/除去）・[`apply_bind_exclusive`]（排他置換）が算出した `new` 集合を
    /// 受け、以下を同型に行う:
    /// 2. **冪等ガード（要件 3.6・D9）**: `new` が現在集合と同値なら状態を書き込まず
    ///    [`BindApplyOutcome::Unchanged`]。
    /// 3. **状態更新**: `dynamic_binds` へ `new` を書き込む（シェル/バルーン state は不変・R3.8）。
    /// 4. **発行判定（D5）**: `Shown(sid)` → 現 surface を `new` で再発行、`Hidden`／未知 →
    ///    [`BindApplyOutcome::StateOnly`]。
    ///
    /// [`apply_bind`]: ScopeStates::apply_bind
    /// [`apply_bind_exclusive`]: ScopeStates::apply_bind_exclusive
    fn commit_bind(&mut self, scope: &ActorKey, new: BindSet) -> BindApplyOutcome {
        // (2) 冪等ガード: 結果が現在集合と同値なら状態を書き込まず発行しない（要件 3.6・D9）。
        if new == *self.current_binds(scope) {
            return BindApplyOutcome::Unchanged;
        }

        // (3) 状態更新: 動的集合のみ書き込む（要件 3.4 積算保持・シェル/バルーン state は不変・R3.8）。
        self.dynamic_binds.insert(scope.clone(), new.clone());

        // (4) 発行判定: 対象 scope のシェル面状態で決める（D5）。
        match self.scopes.get(scope) {
            // 表示中 → 現 surface id を新集合で再発行（要件 3.5）。
            Some(ScopeState::Shown(sid)) => {
                let sid = *sid;
                // bind 変化の Show 再発行は現在コマ（`current_pattern`）を保ったまま再合成する
                // （空 default ではない・design「ScopeStates 拡張」）。bind はシェル面に適用される
                // ため Shell slot の pattern を同梱。ループ不活性（未 commit）なら空＝従来と同値（R5.4）。
                let pattern = self.current_pattern(scope, Slot::Shell).clone();
                BindApplyOutcome::Changed(DisplayCommand::Show {
                    scope: scope.clone(),
                    surface_id: sid,
                    binds: new,
                    pattern,
                })
            }
            // 非表示または未知 scope → 状態のみ更新・発行なし（D5・次 Show へ保留）。
            Some(ScopeState::Hidden) | None => BindApplyOutcome::StateOnly,
        }
    }

    /// 対象 (scope, slot) の現在の pattern 進行状態を返す（要件 5.1）。
    ///
    /// `pattern_states` にエントリを持つ (scope, slot) はその [`PatternState`] を、持たない
    /// (scope, slot) は空（[`PatternState::default`] 相当）を返す純粋な読み取り。`current_binds` が
    /// 未束縛 scope で `static_binds`（既定）を返すのと同型で、pattern は「既定＝空」（pattern 寄与なし）。
    /// Show/ShowBalloon 発行の pattern 供給源はこのアクセサであり、未格納（ループ不活性）の間は空を
    /// 返す＝従来の発行結果と観測等価（要件 5.4）。
    ///
    /// 不在時は静的な空 [`PatternState`]（プロセス共有・`OnceLock`）への参照を返す（`&self` の寿命へ
    /// 縮約される `&'static`）。
    pub fn current_pattern(&self, scope: &ActorKey, slot: Slot) -> &PatternState {
        match self.pattern_states.get(&(scope.clone(), slot)) {
            Some(pattern) => pattern,
            None => empty_pattern(),
        }
    }

    /// 対象 (scope, slot) の pattern 進行状態を `new` へコミットし表示発行の要否を判定する
    /// （[`commit_bind`] の鏡映・要件 6.1/6.2）。
    ///
    /// 手順（`commit_bind` と同型）:
    /// 1. **冪等ガード（要件 6.2）**: `new` が現在 pattern（[`current_pattern`]・不在は空）と同値なら
    ///    状態を書き込まず [`PatternApplyOutcome::Unchanged`] を返す（発行なし）。
    /// 2. **状態更新**: `pattern_states` へ `new` を書き込む（bind／表示状態機械には干渉しない）。
    /// 3. **発行判定**: 対象 slot の表示状態（Shell → `scopes`／Balloon → `balloon`）を見て、
    ///    - `Shown(sid)` → [`PatternApplyOutcome::Changed`]。Shell は現在 bind 集合を同梱した
    ///      [`DisplayCommand::Show`]、Balloon は binds 非同梱の [`DisplayCommand::ShowBalloon`] を、
    ///      いずれも現 surface id＋新 pattern で構築する（現 surface を新コマで再合成・要件 6.1）。
    ///    - `Hidden` または未知（`None`）→ [`PatternApplyOutcome::Unchanged`]（防御的非発行）。表示中
    ///      ゲート（2.1）により commit_pattern は表示中 slot にのみ到達する契約ゆえ、非表示 slot への
    ///      到達は通常運用では発生しないが、到達しても非表示 surface へ Show を強制しない。
    ///
    /// 面種非依存（裁定 (a)）: シェル slot・バルーン slot を同一コードパスで扱い、発行指令の型のみ
    /// slot で分岐する（Shell=Show / Balloon=ShowBalloon）。
    ///
    /// [`commit_bind`]: ScopeStates::commit_bind
    /// [`current_pattern`]: ScopeStates::current_pattern
    pub fn commit_pattern(
        &mut self,
        scope: &ActorKey,
        slot: Slot,
        new: PatternState,
    ) -> PatternApplyOutcome {
        // (1) 冪等ガード: 現在 pattern（不在は空）と同値なら書き込まず発行しない（要件 6.2）。
        if *self.current_pattern(scope, slot) == new {
            return PatternApplyOutcome::Unchanged;
        }

        // (2) 状態更新: pattern 表のみ書き込む（bind／表示状態機械は不変）。
        self.pattern_states.insert((scope.clone(), slot), new.clone());

        // (3) 発行判定: 対象 slot の表示状態で決める（表示中 slot にのみ Show/ShowBalloon を発行）。
        let shown = match slot {
            Slot::Shell => self.scopes.get(scope),
            Slot::Balloon => self.balloon.get(scope),
        };
        match shown {
            Some(&ScopeState::Shown(sid)) => {
                let cmd = match slot {
                    // シェル面は現在 bind 集合を同梱（Show・要件 6.1）。
                    Slot::Shell => DisplayCommand::Show {
                        scope: scope.clone(),
                        surface_id: sid,
                        binds: self.current_binds(scope).clone(),
                        pattern: new,
                    },
                    // バルーン面は binds 非同梱（ShowBalloon・M-boot 着せ替え非在）。
                    Slot::Balloon => DisplayCommand::ShowBalloon {
                        scope: scope.clone(),
                        surface_id: sid,
                        pattern: new,
                    },
                };
                PatternApplyOutcome::Changed(cmd)
            }
            // 非表示または未知 slot → 防御的非発行（表示中ゲート 2.1 で通常運用は非到達・要件 6.2）。
            Some(&ScopeState::Hidden) | None => PatternApplyOutcome::Unchanged,
        }
    }

    /// 表示中（`Shown`）の全 (scope, slot, surface_id) を列挙する読み取り専用アクセサ
    /// （[`LoopRuntime`] の抽選・進行の対象 slot 列挙用・要件 2.1）。
    ///
    /// シェル面 `scopes`・バルーン面 `balloon` の両 map を走査し、`Shown(sid)` のエントリのみを
    /// `(scope, slot, surface_id)` で返す（`Hidden`／未知は除外）。[`LoopRuntime`] は表示中 slot に
    /// のみ抽選・進行を走らせる（表示中ゲート）ため、この列挙が評価対象を与える。列挙順は `HashMap`
    /// 由来で非決定ゆえ、消費側（[`LoopRuntime`]）が固定順（scope 昇順→Shell→Balloon）へ整列する
    /// （D-7）。状態は一切変更しない純粋な読み取りで、`LoopRuntime` は bind／表示状態機械へ書き込まない。
    ///
    /// [`LoopRuntime`]: crate::looper::LoopRuntime
    pub fn shown_slots(&self) -> Vec<(ActorKey, Slot, u32)> {
        let mut out = Vec::new();
        for (scope, st) in &self.scopes {
            if let ScopeState::Shown(sid) = st {
                out.push((scope.clone(), Slot::Shell, *sid));
            }
        }
        for (scope, st) in &self.balloon {
            if let ScopeState::Shown(sid) = st {
                out.push((scope.clone(), Slot::Balloon, *sid));
            }
        }
        out
    }
}

/// 未格納 (scope, slot) 用の空 [`PatternState`]（プロセス共有・不変）。
///
/// [`ScopeStates::current_pattern`] が不在時に返す既定値。`&'static` は `&self` の寿命へ縮約される。
/// `current_binds` が返す `static_binds` に相当する「既定＝空」の pattern 版。
fn empty_pattern() -> &'static PatternState {
    static EMPTY: std::sync::OnceLock<PatternState> = std::sync::OnceLock::new();
    EMPTY.get_or_init(PatternState::default)
}

#[cfg(test)]
#[path = "state_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "state_surface_tests.rs"]
mod surface_tests;
#[cfg(test)]
#[path = "state_bind_pattern_tests.rs"]
mod bind_pattern_tests;
