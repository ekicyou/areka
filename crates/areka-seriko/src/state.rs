//! サーフェス状態モジュール（状態層・per-scope の現 surface 状態と静的 bind 集合の所有者）。
//!
//! 話者スコープ（`ActorKey`）ごとの表示状態（表示中 surface / 非表示）を、スコープの昇順に並ぶ
//! `BTreeMap` で保持する（抽選で乱数を引く順がここで決まる）。shell descript 由来の静的 bind 集合と
//! per-scope の動的着せ替え集合 `dynamic_binds` も同居し、Show 発行の bind 供給源は動的集合を優先、
//! 未束縛 scope は静的既定集合へフォールバックする。
//!
//! [`ScopeStates::apply`] は 1 cue = 1 scope の更新を境界とし、状態が実際に変化したときだけ
//! 発行すべき [`DisplayCommand`] を [`ApplyOutcome::Changed`] で返す。状態不変なら
//! [`ApplyOutcome::Unchanged`] を返して再発行を抑止する。実際の発行はアクター層の `emit_display` が
//! 受け持つ。

use std::collections::{BTreeMap, BTreeSet, HashMap};

use areka_emo_compose::{BindSet, PatternState, SurfaceKeyOutcome};
use areka_sakura::ActorKey;

use crate::bind::{DefaultBinds, accumulate};
use crate::output::DisplayCommand;
use crate::resolve::SurfaceTarget;
use crate::shell_fallback::{ShellFallbackOutcome, ShellSurfaceFallback, SurfaceKeep, decide};

/// あるスコープの現 surface 状態。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ScopeState {
    /// 表示中の surface id。
    Shown(u32),
    /// 非表示（`\s[-1]` 相当）。
    Hidden,
}

/// `apply` の適用結果（状態変化の有無を発行層へ伝える冪等ガード用）。
///
/// - [`ApplyOutcome::Changed`]: 状態が実際に変化した。同梱の [`DisplayCommand`] を発行すべき。
/// - [`ApplyOutcome::Unchanged`]: 状態不変＝再発行不要。
#[derive(Clone, Debug, PartialEq)]
pub enum ApplyOutcome {
    /// 発行すべき指令（状態が変化した）。
    Changed(DisplayCommand),
    /// 状態不変＝発行しない（冪等）。
    Unchanged,
}

/// [`ScopeStates::apply_bind`] の適用結果（着せ替え変化に応じた表示発行の判定）。
///
/// 表示発行が要るかは対象 scope の**シェル面**表示状態で決まる。[`ApplyOutcome`] を鏡映しつつ、
/// 非表示 scope での「状態のみ更新」を第 3 の区分 [`BindApplyOutcome::StateOnly`] で表す。
///
/// - [`BindApplyOutcome::Changed`]: 集合が変化し、かつ対象 scope が表示中（`Shown(id)`）→ 現在の
///   シェル surface id を載せた [`DisplayCommand::Show`] を発行すべき。
/// - [`BindApplyOutcome::StateOnly`]: 集合は変化したが対象 scope が非表示（`Hidden`）または未知
///   （未表示）→ 発行せず状態のみ更新し、次の Show へ新集合を保留する（`\s[-1]` 意味論を
///   破らないため非表示 scope へ Show を強制しない）。
/// - [`BindApplyOutcome::Unchanged`]: 適用結果が直前と同値の集合 → 状態も書き込まず発行もしない
///   （冪等）。
#[derive(Clone, Debug, PartialEq)]
pub enum BindApplyOutcome {
    /// 表示中 scope での集合変化 → 現シェル surface id を載せた Show を発行すべき。
    Changed(DisplayCommand),
    /// 非表示／未知 scope での集合変化 → 発行なし・状態のみ更新（次 Show へ保留）。
    StateOnly,
    /// 結果集合が直前と同値 → 状態書き込みも発行もなし（冪等）。
    Unchanged,
}

/// 表示エントリの面種名前空間（シェル面／バルーン面）。
///
/// [`ScopeStates`] が保持する 2 つの per-scope 表示 map（シェル `scopes`／バルーン `balloon`）に
/// 整合する **surface ID 名前空間の区別**であり、能力の仕切りではない。
/// per-(scope, slot) の [`PatternState`] 表 `pattern_states` のキー成分・`LoopRuntime` 側の再生状態
/// キーと共有し、両 slot は同一コードパスで評価される。`HashMap` キーに用いるため
/// `Copy`＋`Hash`＋`Eq` を導出する。
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Slot {
    /// シェル面（`scopes` map・bind ゲート対象）。
    Shell,
    /// バルーン面（`balloon` map・着せ替えは無い）。
    Balloon,
}

/// 表示層からの窓の知らせ。運ぶ口は `SerikoMsg::Stage`（`SerikoSink::send_stage` が送る）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StageNote {
    /// `scope` のバルーンの窓の見える・見えないと、表示層が今確立している面の番号。
    /// `generation` は表示層の出番の世代（隠れていた窓を出すたびに 1 つ進む）。
    Balloon {
        scope: ActorKey,
        open: bool,
        face: u32,
        generation: u64,
    },
}

/// 知らせで覚えたバルーンの窓（開いているか・表示層が確立している面の番号・知った出番の世代）。
#[derive(Clone, Copy, Debug)]
struct BalloonWindow {
    open: bool,
    face: u32,
    generation: u64,
}

/// [`ScopeStates::commit_pattern`] の適用結果（pattern 進行状態変化に応じた表示発行の判定）。
///
/// `ScopeStates::commit_bind` を鏡映した冪等ガード（同値なら書き込まず
/// [`PatternApplyOutcome::Unchanged`]）。面の在る slot（バルーンは窓が閉じていても届く）なら現 surface
/// を新 pattern で再合成させる Show/ShowBalloon を [`PatternApplyOutcome::Changed`] で返す。面の無い
/// slot は非発行（`Unchanged`）で、`apply_bind` の [`BindApplyOutcome::StateOnly`] に相当する区分は無い。
#[derive(Clone, Debug, PartialEq)]
pub enum PatternApplyOutcome {
    /// 面の在る slot（窓が閉じていても届く）での pattern 変化 → 現 surface を新 pattern で再合成させる
    /// Show/ShowBalloon。
    Changed(DisplayCommand),
    /// 同値（冪等）または面の無い slot → 発行なし。
    Unchanged,
}

/// per-scope surface 状態・静的 bind 集合・per-scope 動的 bind 集合の所有者（状態層）。
///
/// `static_binds` は [`ScopeStates::new`] で設定し、シェルの差し替え（[`ScopeStates::rebase_shell`]）
/// でだけ替わる。動的 bind 未適用の scope に対する既定（初期値）集合として機能する
/// （bind 供給源は [`ScopeStates::current_binds`] が `dynamic_binds` 優先・不在時 `static_binds`
/// で決める）。動的 bind は `apply_bind` が書き込み、`dynamic_binds` は空のまま起動する。
pub struct ScopeStates {
    /// 話者スコープごとの現 surface 状態（スコープの昇順に並ぶ・[`ScopeStates::shown_slots`] の順）。
    scopes: BTreeMap<ActorKey, ScopeState>,
    /// バルーン面のスコープごとの現 surface 状態。同型 [`ScopeState`] だが `scopes` とは別 map で、
    /// `apply_balloon` のみが変更する（`apply()` は触らない・`static_binds` はバルーンでは使わない）。
    balloon: BTreeMap<ActorKey, ScopeState>,
    /// 静的 bind 集合（起動時に解決・シェルの差し替え [`ScopeStates::rebase_shell`] でだけ替わる）。
    ///
    /// 動的 bind 未適用の scope に対する既定（初期値）供給源でもある
    /// （[`ScopeStates::current_binds`] のフォールバック先）。供給元は shell descript の
    /// `default,1`（`build_static_bindset`）。
    static_binds: DefaultBinds,
    /// per-scope の動的着せ替え集合。エントリを持つ scope はその集合を、持たない
    /// scope は `static_binds`（既定・初期値）を Show の bind 供給源とする（[`current_binds`]）。
    ///
    /// 起動時は空（書き込むのは `apply_bind`）。空である限り [`current_binds`] は全 scope で
    /// `static_binds` を返す。
    ///
    /// [`current_binds`]: ScopeStates::current_binds
    dynamic_binds: HashMap<ActorKey, BindSet>,
    /// per-(scope, slot) の pattern 進行状態。エントリを持つ (scope, slot) はその [`PatternState`] を、
    /// 持たない (scope, slot) は空（[`PatternState::default`]）を Show/ShowBalloon の pattern 供給源とする
    /// （[`current_pattern`]）。SERIKO ループ評価が `commit_pattern` 経由で書き込み、surface 切替時に
    /// 当該 slot が空へリセットされる。空である限りループ不活性時は pattern を何も足さない。
    ///
    /// [`current_pattern`]: ScopeStates::current_pattern
    pattern_states: HashMap<(ActorKey, Slot), PatternState>,
    /// 窓の知らせで覚えたバルーンの窓（[`ScopeStates::note_stage`]）。
    balloon_windows: HashMap<ActorKey, BalloonWindow>,
    /// スコープごとの、文字の層の規則での最後の `\s` の答え（`Unresolved` は控えない）。
    text_surfaces: HashMap<ActorKey, SurfaceKeyOutcome>,
}

impl ScopeStates {
    /// 静的 bind 集合を受けて空のスコープ状態を構築する。
    ///
    /// `static_binds` はシェルの bindgroup default 由来の既定の集合で、替わるのはシェルの差し替え
    /// [`ScopeStates::rebase_shell`] だけ。
    pub fn new(static_binds: DefaultBinds) -> Self {
        Self {
            scopes: BTreeMap::new(),
            balloon: BTreeMap::new(),
            static_binds,
            // 空で起動。既定 on/off は pre-populate せず current_binds のフォールバックで実現する。
            dynamic_binds: HashMap::new(),
            // 空で起動。未格納の (scope, slot) は current_pattern が空を返す。
            pattern_states: HashMap::new(),
            balloon_windows: HashMap::new(),
            text_surfaces: HashMap::new(),
        }
    }

    /// 文字の層向けの `\s` の答えを控える。`Unresolved` は控えを変えない。
    pub(crate) fn note_text_surface(&mut self, scope: &ActorKey, outcome: SurfaceKeyOutcome) {
        if outcome != SurfaceKeyOutcome::Unresolved {
            self.text_surfaces.insert(scope.clone(), outcome);
        }
    }

    /// 控えた文字の層向けの `\s` の答え。まだ届いていなければ `None`。
    pub(crate) fn text_surface(&self, scope: &ActorKey) -> Option<SurfaceKeyOutcome> {
        self.text_surfaces.get(scope).copied()
    }

    /// 1 つの解決済み surface 指令を対象スコープへ適用する（1 cue = 1 scope）。
    ///
    /// 対象 `scope` のエントリのみ触り、他スコープの状態は変更しない。戻り値で状態変化の有無を返し、
    /// 冪等ガードを呼び手（発行層）へ委ねる。
    ///
    /// - [`SurfaceTarget::Show`]`(id)`:
    ///   - 現状が既に `Shown(id)` → [`ApplyOutcome::Unchanged`]。
    ///   - それ以外（未知 scope・`Hidden`・別 id を表示中）→ 状態を `Shown(id)` に設定（未知 scope
    ///     は新規挿入）し、[`ApplyOutcome::Changed`]`(DisplayCommand::Show { scope, surface_id: id, binds })`
    ///     を返す（scope の現在 bind 集合 [`current_binds`](Self::current_binds) を同梱。動的 bind
    ///     未適用なら `static_binds`）。
    /// - [`SurfaceTarget::Hide`]:
    ///   - 現状が既に `Hidden` → [`ApplyOutcome::Unchanged`]。
    ///   - それ以外（表示中、または未知 scope）→ 状態を `Hidden` に設定し
    ///     [`ApplyOutcome::Changed`]`(DisplayCommand::Hide { scope })` を返す。
    ///     未知 scope への `Hide` は先行 `Hidden` 状態を持たないため「変化」とみなし Hide を発行する
    ///     （明示的な非表示指定は 1 度発行して観測可能にする方が下流にとって安全）。
    /// - [`SurfaceTarget::Unresolved`]: 呼び手が apply 前に skip するのが正規経路。防御的に、
    ///   状態を変更せず [`ApplyOutcome::Unchanged`] を返す。
    pub fn apply(&mut self, scope: &ActorKey, target: SurfaceTarget) -> ApplyOutcome {
        match target {
            SurfaceTarget::Show(id) => {
                if self.scopes.get(scope) == Some(&ScopeState::Shown(id)) {
                    return ApplyOutcome::Unchanged;
                }
                self.scopes.insert(scope.clone(), ScopeState::Shown(id));
                // surface 切替で当該シェル slot の格納 pattern を空へリセットする。新 surface のコマは
                // 未生成ゆえ空 pattern で発行し、次の commit_pattern の baseline を空にする。
                self.pattern_states.remove(&(scope.clone(), Slot::Shell));
                ApplyOutcome::Changed(DisplayCommand::Show {
                    scope: scope.clone(),
                    surface_id: id,
                    binds: self.current_binds(scope).clone(),
                    // cue 由来 Show は空 pattern（直上でクリア済みと一貫・実 pattern は commit_pattern 経由）。
                    pattern: PatternState::default(),
                })
            }
            SurfaceTarget::Hide => {
                if self.scopes.get(scope) == Some(&ScopeState::Hidden) {
                    return ApplyOutcome::Unchanged;
                }
                self.scopes.insert(scope.clone(), ScopeState::Hidden);
                ApplyOutcome::Changed(DisplayCommand::Hide {
                    scope: scope.clone(),
                })
            }
            // 呼び手が先に skip する正規経路。防御的に無変更・無発行。
            SurfaceTarget::Unresolved => ApplyOutcome::Unchanged,
        }
    }

    /// 1 つの解決済みバルーン面指令を対象スコープへ適用する（[`apply`](Self::apply) の鏡映）。
    ///
    /// シェル面 `scopes` とは独立した `balloon` map のみを触り、`scopes`・`static_binds` には
    /// 干渉しない。状態が実際に変化したときだけ発行すべき [`DisplayCommand`] を
    /// [`ApplyOutcome::Changed`] で返す。
    ///
    /// 発行する指令は [`DisplayCommand::ShowBalloon`]／[`DisplayCommand::HideBalloon`] で、シェル面
    /// [`DisplayCommand::Show`] と異なり **bind を同梱しない**（バルーンに着せ替えは無い）。
    ///
    /// - [`SurfaceTarget::Show`]`(id)`:
    ///   - 現状が既に `Shown(id)` → [`ApplyOutcome::Unchanged`]。
    ///   - それ以外（未知 scope・`Hidden`・別 id を表示中）→ 状態を `Shown(id)` に設定し
    ///     [`ApplyOutcome::Changed`]`(DisplayCommand::ShowBalloon { scope, surface_id: id })` を返す。
    /// - [`SurfaceTarget::Hide`]:
    ///   - 現状が既に `Hidden` → [`ApplyOutcome::Unchanged`]。
    ///   - それ以外（表示中、または未知 scope）→ 状態を `Hidden` に設定し
    ///     [`ApplyOutcome::Changed`]`(DisplayCommand::HideBalloon { scope })` を返す。
    /// - [`SurfaceTarget::Unresolved`]: 呼び手（actor）が `NameForm`／`Invalid` を手前で log＋skip
    ///   するのが正規経路。防御的に、状態を変更せず [`ApplyOutcome::Unchanged`] を返す。
    pub fn apply_balloon(&mut self, scope: &ActorKey, target: SurfaceTarget) -> ApplyOutcome {
        match target {
            SurfaceTarget::Show(id) => {
                if self.balloon.get(scope) == Some(&ScopeState::Shown(id)) {
                    return ApplyOutcome::Unchanged;
                }
                self.balloon.insert(scope.clone(), ScopeState::Shown(id));
                // surface 切替でバルーン slot の格納 pattern を空へリセットする。
                self.pattern_states.remove(&(scope.clone(), Slot::Balloon));
                ApplyOutcome::Changed(DisplayCommand::ShowBalloon {
                    scope: scope.clone(),
                    surface_id: id,
                    // cue 由来のバルーン面 Show は空 pattern（直上でクリア済みと一貫）。
                    pattern: PatternState::default(),
                })
            }
            SurfaceTarget::Hide => {
                if self.balloon.get(scope) == Some(&ScopeState::Hidden) {
                    return ApplyOutcome::Unchanged;
                }
                self.balloon.insert(scope.clone(), ScopeState::Hidden);
                ApplyOutcome::Changed(DisplayCommand::HideBalloon {
                    scope: scope.clone(),
                })
            }
            // 呼び手が先に skip する正規経路。防御的に無変更・無発行。
            SurfaceTarget::Unresolved => ApplyOutcome::Unchanged,
        }
    }

    /// シェルの定義の差し替え。
    ///
    /// 静的な着せ替えを新しいシェルの既定へ入れ替え、動的な着せ替えとシェル側のパターンの
    /// 進行を消す（着せ替えの区分はシェルごとの定義ゆえ持ち越さない）。今の面（`scopes`・
    /// `balloon`）とバルーン側のパターンの進行は保つ。発行はしない（合図は呼び手が出す）。
    pub fn rebase_shell(&mut self, static_binds: DefaultBinds) {
        self.static_binds = static_binds;
        self.dynamic_binds.clear();
        self.pattern_states
            .retain(|(_, slot), _| *slot != Slot::Shell);
    }

    /// シェルの差し替えの戻り先。
    ///
    /// シェルの面で表示中（`Shown(id)`）のスコープだけを見て、今の面が新しいシェルに無ければ
    /// `crate::shell_fallback::decide` の答えで `Shown(to)` か `Hidden` に書き換える。非表示・未表示の
    /// スコープは替えない。結果は書き換えたスコープだけをスコープの昇順で返す（記録は呼び手）。
    pub fn fall_back_shell_surfaces(
        &mut self,
        fallback: &ShellSurfaceFallback,
    ) -> Vec<ShellFallbackOutcome> {
        // 面の番号の集合が空＝材料なし（既定値）。どの面も「無い」と読んで全部隠さないよう替えない。
        if fallback.surface_ids.is_empty() {
            return Vec::new();
        }
        let shown: Vec<(ActorKey, u32)> = self
            .scopes
            .iter()
            .filter_map(|(scope, st)| match st {
                ScopeState::Shown(id) => Some((scope.clone(), *id)),
                ScopeState::Hidden => None,
            })
            .collect();
        let mut outcomes = Vec::new();
        for (scope, missing) in shown {
            let candidates = fallback
                .candidates
                .get(&scope)
                .map(Vec::as_slice)
                .unwrap_or_default();
            match decide(missing, candidates, &fallback.surface_ids) {
                SurfaceKeep::Keep => {}
                SurfaceKeep::FallBack(to) => {
                    self.scopes.insert(scope.clone(), ScopeState::Shown(to));
                    outcomes.push(ShellFallbackOutcome::FellBack { scope, missing, to });
                }
                SurfaceKeep::Hide => {
                    self.scopes.insert(scope.clone(), ScopeState::Hidden);
                    outcomes.push(ShellFallbackOutcome::Hidden {
                        scope,
                        missing,
                        tried: candidates.to_vec(),
                    });
                }
            }
        }
        outcomes
    }

    /// シェルの差し替えの戻り先を控えた `\s` の答えへ当てる: 控えの番号が新しいシェルに無ければ、候補の
    /// 最初に在る番号（無ければ非表示）へ替える（文字の層の戻り先と同じ答え）。番号の集合が空なら替えない。
    pub(crate) fn fall_back_text_surfaces(&mut self, fallback: &ShellSurfaceFallback) {
        if fallback.surface_ids.is_empty() {
            return;
        }
        for (scope, outcome) in &mut self.text_surfaces {
            let SurfaceKeyOutcome::Show(current) = *outcome else {
                continue;
            };
            let candidates = fallback
                .candidates
                .get(scope)
                .map_or(&[][..], Vec::as_slice);
            match decide(current, candidates, &fallback.surface_ids) {
                SurfaceKeep::Keep => {}
                SurfaceKeep::FallBack(to) => *outcome = SurfaceKeyOutcome::Show(to),
                SurfaceKeep::Hide => *outcome = SurfaceKeyOutcome::Hide,
            }
        }
    }

    /// バルーンの定義の差し替え。
    ///
    /// バルーン側のパターンの進行だけを消す。今の面・着せ替え・シェル側のパターンは保つ。
    pub fn rebase_balloon(&mut self) {
        self.pattern_states
            .retain(|(_, slot), _| *slot != Slot::Balloon);
    }

    /// スコープごとの `slot` 側の今の面。
    ///
    /// シェル面・バルーン面のどちらかに現れたスコープを昇順に並べ、`slot` 側で表示中なら
    /// `Some(id)`、非表示・未知なら `None` を返す純粋な読み取り（差し替えの合図の材料）。
    pub fn current_surfaces(&self, slot: Slot) -> Vec<(ActorKey, Option<u32>)> {
        let map = match slot {
            Slot::Shell => &self.scopes,
            Slot::Balloon => &self.balloon,
        };
        let scopes: std::collections::BTreeSet<&ActorKey> =
            self.scopes.keys().chain(self.balloon.keys()).collect();
        scopes
            .into_iter()
            .map(|scope| {
                let id = match map.get(scope) {
                    Some(ScopeState::Shown(id)) => Some(*id),
                    Some(ScopeState::Hidden) | None => None,
                };
                (scope.clone(), id)
            })
            .collect()
    }

    /// 静的（既定）bind 集合を返す（[`rebase_shell`] でだけ替わる）。
    ///
    /// per-scope 動的 bind の既定フォールバック源（初期値）。per-scope の現在集合は
    /// [`current_binds`] で引く。
    ///
    /// [`rebase_shell`]: ScopeStates::rebase_shell
    /// [`current_binds`]: ScopeStates::current_binds
    pub fn binds(&self) -> &DefaultBinds {
        &self.static_binds
    }

    /// 対象 scope の現在の bind 集合を返す。
    ///
    /// `dynamic_binds` にエントリを持つ scope はその動的集合を、持たない scope は静的既定集合
    /// `static_binds`（起動直後 default on/off を初期値とする）を返す純粋な読み取り。Show 発行の
    /// bind 供給源はこのアクセサ。
    pub fn current_binds(&self, scope: &ActorKey) -> &BindSet {
        match self.dynamic_binds.get(scope) {
            Some(binds) => binds,
            None => self.static_binds.for_scope(scope),
        }
    }

    /// 対象 scope の動的 bind 集合へ 1 件の on/off を積算し、表示発行の要否を判定する。
    ///
    /// 1. 現在集合 [`current_binds`] に対し `accumulate(id, on)` で新集合を求める（純関数）。
    /// 2. 新集合が現在集合と同値なら状態を書き込まず [`BindApplyOutcome::Unchanged`] を返す。
    ///    `BindSet` は昇順 dedup の正準形ゆえ順序・重複に不感。
    /// 3. 変化があれば `dynamic_binds` へ新集合を書き込む。
    /// 4. 対象 scope の**シェル面**状態 `scopes` を見て、
    ///    - `Shown(sid)` → [`BindApplyOutcome::Changed`]`(Show{ scope, surface_id: sid, binds: 新集合 })`
    ///      （現 surface を新しい着せ替え集合で再合成させる）。
    ///    - `Hidden` または未知（`None`）→ [`BindApplyOutcome::StateOnly`]（発行せず状態のみ更新。
    ///      非表示 scope へ Show を強制すると `\s[-1]` 意味論を破るため。新集合は次の Show 発行時に
    ///      [`current_binds`] 経由で載る）。
    ///
    /// 書き込むのは `dynamic_binds` のみで、シェル面 `scopes`・バルーン面 `balloon` の状態機械
    /// （[`apply`]／[`apply_balloon`]）には干渉しない。
    ///
    /// [`current_binds`]: ScopeStates::current_binds
    /// [`apply`]: ScopeStates::apply
    /// [`apply_balloon`]: ScopeStates::apply_balloon
    pub fn apply_bind(&mut self, scope: &ActorKey, id: u32, on: bool) -> BindApplyOutcome {
        let new = accumulate(self.current_binds(scope), id, on);
        self.commit_bind(scope, new)
    }

    /// 排他カテゴリ（mustselect／非宣言既定）の排他置換を対象 scope へ適用する。
    ///
    /// 新集合＝`(現在集合 − category_ids) ∪ {target_id}`。同カテゴリの旧パーツ（`category_ids`）を
    /// すべて外したうえで対象パーツ `target_id` を有効化する（同カテゴリ内は高々 1 パーツ有効・
    /// ukadoc 正典の排他選択挙動）。呼び手（actor）はカテゴリの選択ポリシーが `multiple` 宣言以外
    /// ——mustselect と非宣言の既定——の着衣でこの経路を選ぶ。本メソッド自身はポリシーを解釈せず、
    /// 渡された `category_ids` をそのまま外す。`target_id` 自身が `category_ids` に含まれても、
    /// フィルタで一旦除いた後 `chain` で再付与するため最終的に必ず有効になる。
    ///
    /// 冪等ガード・状態更新・発行判定（Changed/StateOnly/Unchanged）は [`apply_bind`] と同型
    /// （`commit_bind` に集約）。書き込むのは `dynamic_binds` のみ。同一 target の再適用は結果集合が
    /// 直前と同値になり [`BindApplyOutcome::Unchanged`]。
    /// [`apply_bind`]: ScopeStates::apply_bind
    pub fn apply_bind_exclusive(
        &mut self,
        scope: &ActorKey,
        category_ids: &[u32],
        target_id: u32,
    ) -> BindApplyOutcome {
        let current = self.current_binds(scope);
        let new = BindSet::from_ids(
            current
                .ids()
                .iter()
                .copied()
                .filter(|id| !category_ids.contains(id))
                .chain(std::iter::once(target_id)),
        );
        self.commit_bind(scope, new)
    }

    /// bind 新集合を対象 scope へコミットし表示発行の要否を判定する共通後段。
    ///
    /// [`apply_bind`]（加算/除去）・[`apply_bind_exclusive`]（排他置換）が算出した `new` 集合を受け、
    /// 次を行う。
    /// - `new` が現在集合と同値なら状態を書き込まず [`BindApplyOutcome::Unchanged`]。
    /// - `dynamic_binds` へ `new` を書き込む（シェル/バルーン state は不変）。
    /// - `removed = 旧集合 − 新集合` の ID がシェル slot に保持コマ（`-1` 終端を持たないアニメが
    ///   末尾到達後に保つ最終コマ）を持っていれば、**発行を組む前に**取り除く。これが無いと bind 集合を
    ///   正典どおりにしても外れたパーツのコマが Show に乗り続け、表情が固着する。
    /// - `Shown(sid)` → 現 surface を `new` で再発行、`Hidden`／未知 → [`BindApplyOutcome::StateOnly`]。
    ///
    /// `removed` が非空なら旧集合と `new` は必ず相異なるため冪等ガードを通過する＝掃除した回は必ず
    /// 状態更新まで進み、表示中なら同じ発行で新しい pattern が載る。逆に `Unchanged` の回は集合同値ゆえ
    /// `removed` は空で、掃除すべきものが無い。
    ///
    /// [`apply_bind`]: ScopeStates::apply_bind
    /// [`apply_bind_exclusive`]: ScopeStates::apply_bind_exclusive
    fn commit_bind(&mut self, scope: &ActorKey, new: BindSet) -> BindApplyOutcome {
        if new == *self.current_binds(scope) {
            return BindApplyOutcome::Unchanged;
        }

        // 外れた ID を状態更新の前に確定させる。bind 集合の要素は shell descript の bindgroup 由来のみゆえ、
        // 純 `random` の `interval` など bind に属さないアニメは構造的に混じらない。
        let removed: Vec<u32> = self
            .current_binds(scope)
            .ids()
            .iter()
            .copied()
            .filter(|id| !new.contains(*id))
            .collect();

        self.dynamic_binds.insert(scope.clone(), new.clone());

        self.drop_residual_frames(scope, &removed);

        match self.scopes.get(scope) {
            Some(ScopeState::Shown(sid)) => {
                let sid = *sid;
                // bind 変化の Show 再発行は現在コマ（`current_pattern`）を保ったまま再合成する（空 default ではない）。
                // bind はシェル面に適用されるため Shell slot の pattern を同梱。
                let pattern = self.current_pattern(scope, Slot::Shell).clone();
                BindApplyOutcome::Changed(DisplayCommand::Show {
                    scope: scope.clone(),
                    surface_id: sid,
                    binds: new,
                    pattern,
                })
            }
            Some(ScopeState::Hidden) | None => BindApplyOutcome::StateOnly,
        }
    }

    /// bind から外れた ID の保持コマをシェル slot から取り除く。
    ///
    /// `-1` 終端を持たないアニメは末尾到達後も最終コマを保持する。その ID が bind 集合から外れたときの
    /// 後始末だけをここで行う。対象は [`Slot::Shell`] のみ（バルーン面 slot は不変）。
    ///
    /// 実際にコマを取り除いた ID ごとに `info!` を残す。保持コマを持たない ID は状態が動かないため
    /// 痕跡も残さない。
    /// 保持コマを持たない ID は状態が動かないため痕跡も残さない。
    fn drop_residual_frames(&mut self, scope: &ActorKey, removed: &[u32]) {
        if removed.is_empty() {
            return;
        }
        let Some(pattern) = self.pattern_states.get_mut(&(scope.clone(), Slot::Shell)) else {
            return;
        };
        for &id in removed {
            if pattern.get(id).is_none() {
                continue;
            }
            pattern.remove(id);
            tracing::info!(
                scope = %scope,
                id,
                "seriko: bind から外れた ID の保持コマを除去（bindopt 7.1）"
            );
        }
    }

    /// 対象 (scope, slot) の現在の pattern 進行状態を返す。
    ///
    /// `pattern_states` にエントリを持たない (scope, slot) は空（[`PatternState::default`] 相当）を返す
    /// 純粋な読み取り。Show/ShowBalloon 発行の pattern 供給源はこのアクセサ。
    ///
    /// 不在時は静的な空 [`PatternState`]（プロセス共有・`OnceLock`）への参照を返す。
    pub fn current_pattern(&self, scope: &ActorKey, slot: Slot) -> &PatternState {
        match self.pattern_states.get(&(scope.clone(), slot)) {
            Some(pattern) => pattern,
            None => empty_pattern(),
        }
    }

    /// 対象 (scope, slot) の pattern 進行状態を `new` へコミットし表示発行の要否を判定する
    /// （`commit_bind` の鏡映）。
    ///
    /// 1. `new` が現在 pattern（[`current_pattern`]・不在は空）と同値なら状態を書き込まず
    ///    [`PatternApplyOutcome::Unchanged`] を返す。
    /// 2. `pattern_states` へ `new` を書き込む（bind／表示状態機械には干渉しない）。
    /// 3. 対象 slot の面（Shell は `scopes`、Balloon は `balloon_face`＝台本で受けた面か知らせの面）を
    ///    見て、
    ///    - 面が在る → [`PatternApplyOutcome::Changed`]。Shell は現在 bind 集合を同梱した
    ///      [`DisplayCommand::Show`]、Balloon は binds 非同梱の [`DisplayCommand::ShowBalloon`] を、
    ///      いずれも現 surface id＋新 pattern で構築する。バルーンは窓が閉じていても届く。
    ///    - 面が無い → [`PatternApplyOutcome::Unchanged`]。
    ///
    /// シェル slot・バルーン slot は同一コードパスで扱い、発行指令の型のみ slot で分岐する。
    /// [`current_pattern`]: ScopeStates::current_pattern
    pub fn commit_pattern(
        &mut self,
        scope: &ActorKey,
        slot: Slot,
        new: PatternState,
    ) -> PatternApplyOutcome {
        if *self.current_pattern(scope, slot) == new {
            return PatternApplyOutcome::Unchanged;
        }

        self.pattern_states
            .insert((scope.clone(), slot), new.clone());

        // バルーンの面は `stage_slots` と同じ番号（`\b` が無ければ知らせの面）。
        let face = match slot {
            Slot::Shell => match self.scopes.get(scope) {
                Some(&ScopeState::Shown(sid)) => Some(sid),
                Some(ScopeState::Hidden) | None => None,
            },
            Slot::Balloon => self.balloon_face(scope),
        };
        match face {
            Some(sid) => {
                let cmd = match slot {
                    Slot::Shell => DisplayCommand::Show {
                        scope: scope.clone(),
                        surface_id: sid,
                        binds: self.current_binds(scope).clone(),
                        pattern: new,
                    },
                    Slot::Balloon => DisplayCommand::ShowBalloon {
                        scope: scope.clone(),
                        surface_id: sid,
                        pattern: new,
                    },
                };
                PatternApplyOutcome::Changed(cmd)
            }
            // 面の無い slot → 防御的非発行。
            None => PatternApplyOutcome::Unchanged,
        }
    }

    /// 表示中（`Shown`）の全 (scope, slot, surface_id) を列挙する読み取り専用アクセサ
    /// （`LoopRuntime` の抽選の対象の列挙・進行の対象は [`ScopeStates::stage_slots`]）。
    ///
    /// `Hidden`／未知は除外する。並びは乱数を引く固定順（scope 昇順→Shell→Balloon）で、
    /// [`ScopeStates::stage_slots`] と同じ。
    pub fn shown_slots(&self) -> Vec<(ActorKey, Slot, u32)> {
        let scopes: BTreeSet<&ActorKey> = self.scopes.keys().chain(self.balloon.keys()).collect();
        let mut out = Vec::new();
        for scope in scopes {
            if let Some(ScopeState::Shown(sid)) = self.scopes.get(scope) {
                out.push((scope.clone(), Slot::Shell, *sid));
            }
            if let Some(ScopeState::Shown(sid)) = self.balloon.get(scope) {
                out.push((scope.clone(), Slot::Balloon, *sid));
            }
        }
        out
    }

    /// 窓の知らせを覚える。窓の開け閉めは
    /// いつも知らせに従い、面の番号は `\b[番号]` を受けていないスコープでだけ使う
    /// （[`ScopeStates::stage_slots`]）。出番の世代は大きい方を覚える（古い知らせで戻さない）。
    pub fn note_stage(&mut self, note: &StageNote) {
        let StageNote::Balloon {
            scope,
            open,
            face,
            generation,
        } = note;
        let generation = (*generation).max(self.stage_generation(scope));
        self.balloon_windows.insert(
            scope.clone(),
            BalloonWindow {
                open: *open,
                face: *face,
                generation,
            },
        );
    }

    /// `scope` のバルーンで覚えている出番の世代（知らせが 1 度も無ければ 0＝表示層の始まりの値）。
    pub fn stage_generation(&self, scope: &ActorKey) -> u64 {
        self.balloon_windows.get(scope).map_or(0, |w| w.generation)
    }

    /// `scope` のバルーンの面の番号: `\b[番号]` を受けていればその番号（`\b[-1]` なら面なし）、
    /// 受けていなければ知らせの面（seriko 自身の状態が在れば古い知らせで上書きしない）。
    fn balloon_face(&self, scope: &ActorKey) -> Option<u32> {
        match self.balloon.get(scope) {
            Some(ScopeState::Shown(id)) => Some(*id),
            Some(ScopeState::Hidden) => None,
            None => self.balloon_windows.get(scope).map(|w| w.face),
        }
    }

    /// 進行の対象の面。4 つ目は「画面に見えているか」。
    ///
    /// シェルは [`ScopeStates::shown_slots`] と同じ（見えている＝真）。バルーンは面の番号が分かる
    /// スコープの全部で、見えているかは知らせの窓（知らせが 1 度も無ければ、`\b` で
    /// 表示中の面を開いた窓とみなす）。並びはスコープの昇順 → シェル → バルーン。
    pub fn stage_slots(&self) -> Vec<(ActorKey, Slot, u32, bool)> {
        let scopes: BTreeSet<&ActorKey> = self
            .scopes
            .keys()
            .chain(self.balloon.keys())
            .chain(self.balloon_windows.keys())
            .collect();
        let mut out = Vec::new();
        for scope in scopes {
            if let Some(ScopeState::Shown(sid)) = self.scopes.get(scope) {
                out.push((scope.clone(), Slot::Shell, *sid, true));
            }
            if let Some(face) = self.balloon_face(scope) {
                let open = self.balloon_windows.get(scope).is_none_or(|w| w.open);
                out.push((scope.clone(), Slot::Balloon, face, open));
            }
        }
        out
    }
}

/// 未格納 (scope, slot) 用の空 [`PatternState`]（プロセス共有・不変）。
///
/// [`ScopeStates::current_pattern`] が不在時に返す既定値。
fn empty_pattern() -> &'static PatternState {
    static EMPTY: std::sync::OnceLock<PatternState> = std::sync::OnceLock::new();
    EMPTY.get_or_init(PatternState::default)
}

#[cfg(test)]
#[path = "state_bind_pattern_tests.rs"]
mod bind_pattern_tests;
#[cfg(test)]
#[path = "state_rebase_tests.rs"]
mod rebase_tests;
#[cfg(test)]
#[path = "state_stage_tests.rs"]
mod stage_tests;
#[cfg(test)]
#[path = "state_surface_tests.rs"]
mod surface_tests;
#[cfg(test)]
#[path = "state_test_support.rs"]
mod test_support;
#[cfg(test)]
#[path = "state_text_surface_tests.rs"]
mod text_surface_tests;
