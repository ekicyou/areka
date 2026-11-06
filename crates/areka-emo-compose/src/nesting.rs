//! 入れ子（element定義でサーフェスを部品として置く・areka 独自の語）の静的な事実。
//!
//! element定義のファイル名の欄が半角の数字だけのとき、その欄を画像のファイル名でなく
//! サーフェスの番号として読む（要件 1.1・1.2）。読み分けの実装は本モジュールの
//! [`element_kind`] 1 関数だけで、畳み込み（[`crate::fold`]）と焼く前の除外（下流の
//! `shell_target`）が同じ関数を呼ぶ。
//!
//! 記録は出さない（広がりの予算が尽きたときの `error!` だけは出す）。事実を値で返し、記録は
//! fs を触る入口が読み込み 1 回につき 1 度だけ出す。

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use areka_parsers::shell::{Animation, ElementPath, Interval, Pattern};

use crate::bind::BindSet;
use crate::bind_steps::BindStep;
use crate::budget::{Budget, VISIT_BUDGET, surface_rows};
use crate::method::is_implemented_name;
use crate::pattern::{Cell, PatternState};
use crate::pattern_word::{PatternWord, pattern_word};
use crate::plan::is_bind_interval;
use crate::world::{EmoWorld, Records};

/// element定義が置くもの。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementKind {
    /// 画像（今までどおり）。
    Image,
    /// サーフェスの番号（欄が半角の数字だけで、u32 に収まる）。
    Surface(u32),
    /// 欄は半角の数字だけだが u32 に収まらない。画像としては読まない（要件 1.9）。
    SurfaceOutOfRange,
    /// 動く絵から作った子を置く（分解が画像の element を替える）。作者の欄の読み分け
    /// （[`element_kind`]）からは出ない（要件 1.12）。
    Film(FilmId),
}

/// 動く絵の子の番号（親の絵の番号＝同じ画像は同じ子）。作者のサーフェスの番号とは別の空間。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FilmId(pub u32);

/// 見える部品の印を `out` を順に引いて確かめる上限（越えたら集合へ移す）。毎フレーム呼ばれるので、
/// ふつうの大きさでは集合を確保しない。
const LINEAR_SEEN_MAX: usize = 64;

/// 部品を指す鍵。作者のサーフェスと動く絵の子は種類が違うので、番号が当たることが無い（要件 1.12）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PartKey {
    /// 作者が書いたサーフェス（番号）。
    Surface(u32),
    /// 動く絵から作った子。
    Film(FilmId),
}

/// interval が `always` の単独（小文字の完全一致）か。`bind+always` などの組み合わせ・大文字・
/// 空白つきは偽（要件 4.8・4.9）。合成・見える部品・seriko の表がこの 1 関数を使う。
pub fn is_always_interval(interval: &Interval) -> bool {
    matches!(interval, Interval::Other(s) if &**s == "always")
}

/// 経過 0 のコマ＝待ち時間の累積が 0 の最後の番号（先頭から待つなら `None`）。
/// 呼ぶのは動く絵の表（`film.rs`）だけ。seriko は同じ規則を自前で持つ（テストで突き合わせる）。
pub fn rest_index(waits_ms: impl Iterator<Item = u32>) -> Option<usize> {
    waits_ms.take_while(|&w| w == 0).count().checked_sub(1)
}

/// animation の経過 0 の pattern（pattern の番号の昇順・同じ番号は書いた順に並べ、待ちに
/// [`rest_index`] を当てたのと同じ答え）。先頭から待つなら `None`。番号が負・描画メソッドが動くかは
/// 呼び手が見る。合成のたびに呼ばれるので並べた写しは作らず、「最初に待つ pattern より前で最後の
/// pattern」を 2 度なめて引く（指摘 29）。
pub(crate) fn rest_pattern(anim: &Animation) -> Option<&Pattern> {
    // 並べる鍵（番号, 書いた位置）。
    let key = |&(i, p): &(usize, &Pattern)| (p.index, i);
    let patterns = || anim.patterns.iter().enumerate();
    let first_wait = patterns()
        .filter(|(_, p)| p.wait != 0)
        .map(|e| key(&e))
        .min();
    patterns()
        .filter(|e| first_wait.is_none_or(|k| key(e) < k))
        .max_by_key(key)
        .map(|(_, p)| p)
}

/// 欄が空でなく、全部が半角の数字（0〜9）なら番号として読む。`0100` は 100。
/// 符号つき・全角の数字・拡張子つき・空は画像（要件 1.1・1.3・1.9）。
pub fn element_kind(path: &ElementPath) -> ElementKind {
    let s = path.as_str();
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return ElementKind::Image;
    }
    // 数字だけが確かなので、失敗は桁あふれだけ（先頭の 0 は値に効かない）。
    match s.parse::<u32>() {
        Ok(id) => ElementKind::Surface(id),
        Err(_) => ElementKind::SurfaceOutOfRange,
    }
}

/// サーフェスごとの静的な参照（要件 5.11・5.13）。面の表から 1 度作る不変の値（Send・seriko が写しを持つ）。
///
/// `children`・`bind_targets`・`bind_random_ids`・`films`・`always_rest` のどれかが空でない番号だけを載せる
/// （載せる条件はこの 1 か所）。入れ子も着せ替えの種類の animation も動く絵の子も `always` の経過 0 の
/// 先も無いシェルでは空。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NestTable {
    surfaces: BTreeMap<u32, SurfaceParts>,
    /// 部品として入れる面（面の表に在り、行の数が予算に届かないもの）。①〜③ の先の門はこの 1 つ。
    enterable: BTreeSet<u32>,
}

/// 1 つのサーフェスが指す先（[`NestTable`] の 1 行）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SurfaceParts {
    /// element定義で置いた子（面の表に在る番号だけ・element定義の番号の昇順）。
    pub children: Vec<u32>,
    /// 着せ替えの animation の手順の「描く」コマが指すサーフェス（animation の番号, 番号）。
    /// 純 `bind` の全部のコマ（`bind+random` は手順を持たず、コマがあるときだけ③で数える）。昇順・重複なし。
    pub bind_targets: Vec<(u32, u32)>,
    /// `bind+random` の animation の番号（昇順・重複なし）。③ の合流の門が引く（無効ならコマを数えない）。
    /// 純 `bind` のコマは呼ばれて始まったものだけなので門を通さない（seriko-pattern-calls 5.6・M5）。
    pub bind_random_ids: Vec<u32>,
    /// element定義で置いた動く絵の子（昇順・重複なし）。
    pub films: Vec<FilmId>,
    /// `always` の animation の経過 0 の pattern が指すサーフェス（animation の番号, 番号）。
    /// 経過 0 の pattern が在り・番号が 0 以上・描画メソッドが動くものだけ。animation の番号の昇順。
    pub always_rest: Vec<(u32, u32)>,
}

impl NestTable {
    /// 面の表から作る（[`EmoWorld::nest_table`] の中身）。
    pub(crate) fn from_world(world: &EmoWorld) -> NestTable {
        // 行の数が予算に届く面は、合成・外形・持ち込みの走査が部品として入らない（`Budget::enter`）。
        let enterable: BTreeSet<u32> = world
            .surface_ids()
            .filter(|&c| {
                world
                    .surface(c)
                    .is_some_and(|m| surface_rows(m) < VISIT_BUDGET)
            })
            .collect();
        let mut surfaces = BTreeMap::new();
        for id in world.surface_ids() {
            let Some(master) = world.surface(id) else {
                continue;
            };
            // elements は layer 昇順・同 layer は登場順（SurfaceMaster の不変条件）。
            let children: Vec<u32> = master
                .elements
                .iter()
                .filter_map(|e| match e.kind {
                    ElementKind::Surface(c) if enterable.contains(&c) => Some(c),
                    _ => None,
                })
                .collect();
            let mut bind_random_ids: Vec<u32> = Vec::new();
            let mut bind_targets: Vec<(u32, u32)> = Vec::new();
            for anim in master
                .animations
                .iter()
                .filter(|a| is_bind_interval(&a.interval))
            {
                if matches!(anim.interval, Interval::BindRandom { .. }) {
                    bind_random_ids.push(anim.id);
                }
                // 合成と同じ辺: 着せ替えの手順の「描く」コマ（純 `bind` の全部・`bind+random` は手順が空）。
                bind_targets.extend(world.bind_steps(id, anim.id).iter().filter_map(
                    |s| match *s {
                        BindStep::Draw { surface, .. } if enterable.contains(&surface) => {
                            Some((anim.id, surface))
                        }
                        _ => None,
                    },
                ));
            }
            // 同じ番号の animation は畳み込みで 1 つ（先勝ち）。並びだけを番号の昇順へ揃える
            // （bind_random_ids は visible_parts が二分探索で引く）。
            bind_random_ids.sort_unstable();
            bind_targets.sort_unstable();
            bind_targets.dedup();
            let mut films: Vec<FilmId> = master
                .elements
                .iter()
                .filter_map(|e| match e.kind {
                    ElementKind::Film(f) => Some(f),
                    _ => None,
                })
                .collect();
            films.sort_unstable();
            films.dedup();
            // 合成と同じ辺: 経過 0 の pattern の番号が 0 以上・描画メソッドが動く。
            let mut always_rest: Vec<(u32, u32)> = master
                .animations
                .iter()
                .filter(|a| is_always_interval(&a.interval))
                .filter_map(|a| {
                    let p = rest_pattern(a).filter(|p| is_implemented_name(p.method.as_str()))?;
                    let target = u32::try_from(p.surface_id).ok()?;
                    enterable.contains(&target).then_some((a.id, target))
                })
                .collect();
            always_rest.sort_unstable();
            let row = SurfaceParts {
                children,
                bind_targets,
                bind_random_ids,
                films,
                always_rest,
            };
            if row != SurfaceParts::default() {
                surfaces.insert(id, row);
            }
        }
        NestTable {
            surfaces,
            enterable,
        }
    }

    /// 1 行も載っていないか。
    pub fn is_empty(&self) -> bool {
        self.surfaces.is_empty()
    }

    /// `surface_id` の行（載っていなければ `None`）。
    pub fn parts(&self, surface_id: u32) -> Option<&SurfaceParts> {
        self.surfaces.get(&surface_id)
    }

    /// 今の絵に出ている部品の番号を、昇順・重複なしで `out` へ入れる（`out` は先に空にする）。
    ///
    /// 一番上から ① `children` ② 有効な着せ替え（`binds` に在る）の `bind_targets`（同じ animation の
    /// 番号にコマが在ればコマが置き換えるので数えない）②' `always_rest`（その animation の欄が
    /// 「載っていない」ときだけ・「消えている」なら進まず、コマなら③で数える）③ そのサーフェスのコマ
    /// （一番上は今までの欄・部品は部品の欄）のうち描画メソッドが動くもので、`bind+random` なら `binds`
    /// に在るもの、の先を部品に数えてその先へ進む（`flatten_surface` と同じ辺）。先祖へ戻る辺は進まない。
    ///
    /// 辺は「一番上か部品か」と番号だけで決まるので、たどれる番号の集合は「一番上から届く番号」と
    /// 同じになる。そこで `out` 自身を「これから見る面」の列と訪れた印に使い（見つけた順に足し、
    /// 前から順に見る）、一番上と既に入った番号へは進まない。先祖は必ずそのどちらかなので、先祖へ戻る
    /// 辺も進まない。各面を 1 度しか見ないので広がりの予算は要らない。最後に昇順へ並べる。訪れた印は
    /// 小さいうちは `out` を順に引き、[`LINEAR_SEEN_MAX`] 個を越えたら集合で引く（指摘 25）。確保は
    /// `out` の伸びと、越えたときの集合だけ（ふつうのシェルは越えない）。
    pub fn visible_parts(
        &self,
        top: u32,
        binds: &BindSet,
        pattern: &PatternState,
        out: &mut Vec<u32>,
    ) {
        out.clear();
        let mut seen = HashSet::new();
        self.walk(top, top, binds, pattern, out, &mut seen);
        let mut next = 0;
        while let Some(&surface) = out.get(next) {
            next += 1;
            self.walk(surface, top, binds, pattern, out, &mut seen);
        }
        out.sort_unstable();
    }

    /// `surface` から出る辺の先を `out` へ足す（`surface == top` なら一番上の欄、それ以外は部品の欄）。
    fn walk(
        &self,
        surface: u32,
        top: u32,
        binds: &BindSet,
        pattern: &PatternState,
        out: &mut Vec<u32>,
        seen: &mut HashSet<u32>,
    ) {
        let is_top = surface == top;
        let parts = self.parts(surface);
        let frame_of = |id: u32| {
            if is_top {
                pattern.get(id)
            } else {
                pattern.part_get(surface, id)
            }
        };
        if let Some(p) = parts {
            // ① element定義の子。
            for &child in &p.children {
                Self::enter(child, top, out, seen);
            }
            // ② 有効な着せ替えの描くコマ（コマが在ればコマが置き換える＝③で数える）。
            for &(id, target) in &p.bind_targets {
                if binds.contains(id) && frame_of(id).is_none() {
                    Self::enter(target, top, out, seen);
                }
            }
            // ②' `always` の経過 0（欄が「載っていない」ときだけ）。
            let key = (!is_top).then_some(PartKey::Surface(surface));
            for &(id, target) in &p.always_rest {
                if pattern.cell(key, id) == Cell::Rest {
                    Self::enter(target, top, out, seen);
                }
            }
        }
        // ③ コマ。`bind+random` なら有効なものだけ（純 `bind` のコマは無効でも数える・M5）。
        let is_bind = |id: u32| parts.is_some_and(|p| p.bind_random_ids.binary_search(&id).is_ok());
        // 段の種類で読む欄だけを回す（使わない欄を全部なめない・指摘 30）。
        let top_frames = is_top.then(|| pattern.iter()).into_iter().flatten();
        let part_frames = (!is_top)
            .then(|| pattern.part(surface))
            .into_iter()
            .flatten();
        for (id, frame) in top_frames.chain(part_frames) {
            if frame.method.is_implemented()
                && (!is_bind(id) || binds.contains(id))
                && self.enterable.contains(&frame.surface_id)
            {
                Self::enter(frame.surface_id, top, out, seen);
            }
        }
    }

    /// 一番上と、求め済みの見える部品 `parts`（[`visible_parts`](Self::visible_parts) の答え）に置かれた
    /// 動く絵の子を、昇順・重複なしで `out` へ入れる（`out` は先に空にする）。
    pub fn visible_films(&self, top: u32, parts: &[u32], out: &mut Vec<FilmId>) {
        out.clear();
        for s in std::iter::once(top).chain(parts.iter().copied()) {
            if let Some(p) = self.parts(s) {
                out.extend_from_slice(&p.films);
            }
        }
        out.sort_unstable();
        out.dedup();
    }

    /// `target` を部品に数え、これから見る面の列へ足す（一番上・既に数えた番号は足さない）。
    /// `out` が [`LINEAR_SEEN_MAX`] 個に届いたら、印を `seen` へ写して以後は集合で引く。
    fn enter(target: u32, top: u32, out: &mut Vec<u32>, seen: &mut HashSet<u32>) {
        if target == top {
            return;
        }
        let fresh = if out.len() < LINEAR_SEEN_MAX {
            !out.contains(&target)
        } else {
            if seen.is_empty() {
                seen.extend(out.iter().copied());
            }
            seen.insert(target)
        };
        if fresh {
            out.push(target);
        }
    }
}

/// 入れ子の読み飛ばし 1 件（要件 3.1・3.2・1.9）。記録の形は下流の入口が決める。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NestIssue {
    /// 指した番号のサーフェスが無い（u32 に収まらない数字を含む）。`target` は欄の原文。
    MissingTarget {
        surface: u32,
        element: u32,
        target: String,
    },
    /// この参照をたどると `surface` へ戻る（自分自身を指す場合を含む）。
    Cycle {
        surface: u32,
        element: u32,
        target: u32,
    },
}

/// 無い番号と循環の報告（面の表から 1 度作る・記録は出さない。広がりの予算が尽きたときの `error!` だけ出す）。
///
/// 並びは親の番号の昇順 → element定義の番号の昇順。同じ番号の element定義が複数あるときは
/// 書いた順（`SurfaceMaster.elements` の並び＝畳み込みの安定ソートのまま）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NestReport {
    pub issues: Vec<NestIssue>,
}

impl NestReport {
    /// 面の表から作る（[`EmoWorld::nest_report`] の中身）。
    ///
    /// 循環をたどる辺は、element定義の辺と、すべての animation のすべての pattern定義の辺
    /// （[`pattern_edge`]・7 語と `move` を除く）。着せ替えの有効・無効や
    /// 描画メソッドが動くかには依らない（どの一番上から見ても切られうる辺を全部挙げる）。
    pub(crate) fn from_world(world: &EmoWorld) -> NestReport {
        Self::from_world_limited(world, VISIT_BUDGET)
    }

    /// [`NestReport::from_world`] の中身。`limit` は [`reaches`] 1 回の予算（本番は [`VISIT_BUDGET`]・テストは
    /// 小さな数）。予算が尽きた `error!` は報告 1 つにつき 1 度だけ（指摘 2）。
    pub(crate) fn from_world_limited(world: &EmoWorld, limit: usize) -> NestReport {
        let edges = nest_edges(world);
        let mut reported = false;
        // 同じ（子, 親）の組は答えが同じなので 1 度だけたどる（同じ子を指す element定義の数だけたどり直さない・指摘 132）。
        let mut cycles: HashMap<(u32, u32), bool> = HashMap::new();

        let mut issues = Vec::new();
        for &id in edges.keys() {
            let Some(master) = world.surface(id) else {
                continue;
            };
            // elements は element定義の番号の昇順・同じ番号は書いた順。
            for e in &master.elements {
                let child = match e.kind {
                    // 子は先を持たない（無い番号でも循環でもない）。
                    ElementKind::Image | ElementKind::Film(_) => continue,
                    ElementKind::Surface(c) if edges.contains_key(&c) => c,
                    _ => {
                        issues.push(NestIssue::MissingTarget {
                            surface: id,
                            element: e.layer,
                            target: e.path.as_str().to_string(),
                        });
                        continue;
                    }
                };
                let cycle = *cycles
                    .entry((child, id))
                    .or_insert_with(|| reaches(&edges, child, id, limit, &mut reported));
                if cycle {
                    issues.push(NestIssue::Cycle {
                        surface: id,
                        element: e.layer,
                        target: child,
                    });
                }
            }
        }
        NestReport { issues }
    }
}

/// 循環をたどる辺（親の番号 → 子の番号）: element定義の辺と、すべての animation のすべての
/// pattern定義の辺（[`pattern_edge`]）。[`NestReport`] と形の誤り（[`crate::plan_issues`]）が共有する。
pub(crate) fn nest_edges(world: &EmoWorld) -> BTreeMap<u32, Vec<u32>> {
    let mut edges: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for id in world.surface_ids() {
        let Some(master) = world.surface(id) else {
            continue;
        };
        let to = edges.entry(id).or_default();
        to.extend(master.elements.iter().filter_map(|e| match e.kind {
            ElementKind::Surface(c) => Some(c),
            _ => None,
        }));
        to.extend(
            master
                .animations
                .iter()
                .flat_map(|a| &a.patterns)
                .filter_map(pattern_edge),
        );
    }
    edges
}

/// pattern定義の辺の先（番号が 0 以上で u32 に収まり、欄 2 が animation の番号になる 7 語でも、
/// サーフェスの番号を無視する `move`（ukadoc）でもないもの）。
pub(crate) fn pattern_edge(p: &Pattern) -> Option<u32> {
    if pattern_word(p.method.as_str()) != PatternWord::Draw {
        return None;
    }
    u32::try_from(p.surface_id).ok()
}

/// `from` から辺をたどって `to` へ着けるか（`from == to` も着いたに数える）。`limit` は呼び出し 1 回の
/// 広がりの予算（本番は [`VISIT_BUDGET`]）。取り出した面を `from` の他は「1 ＋その面から出る辺の数」ずつ
/// 数え（なめる辺も仕事・指摘 130 と同じ型）、断られた面は開かない（尽きたら残りの辺は循環として報告しない・
/// 合成の祖先の確かめが循環を止める）。着いたかは取り出したときに数える前に見る（`to` は開かない）。尽きた `error!` は `reported` が偽のときだけ出し、出したら真にする
/// （呼び手は表 1 つのあいだ同じ印を渡す＝組の数だけ出さない・指摘 2）。
// ponytail: 呼び手は（子, 親）の組ごとに 1 度呼び、1 回は予算いっぱいまでなめる（組の数 × 予算）。読み込み 1 回だけなので、遅ければ強連結成分で 1 度に引く。
pub(crate) fn reaches(
    edges: &BTreeMap<u32, Vec<u32>>,
    from: u32,
    to: u32,
    limit: usize,
    reported: &mut bool,
) -> bool {
    #[cfg(test)]
    REACHES_CALLS.with(|n| n.set(n.get() + 1));
    let records = if *reported {
        Records::Quiet
    } else {
        Records::Emit
    };
    let mut budget = Budget::with_records(limit, records);
    let mut seen = BTreeSet::from([from]);
    let mut stack = vec![from];
    while let Some(s) = stack.pop() {
        // 着いたかは入る前に見る（`to` は開かないので、どの予算にも収まらない面でも循環を取りこぼさない）。
        if s == to {
            return true;
        }
        let out = edges.get(&s).map_or(&[][..], Vec::as_slice);
        if s != from && !budget.enter(out.len(), "reaches", from) {
            // その面を開かないだけ（積んだ残りは開かずに `to` かだけ見る）。
            *reported = true;
            continue;
        }
        for &next in out {
            if seen.insert(next) {
                stack.push(next);
            }
        }
    }
    false
}

#[cfg(test)]
thread_local! {
    /// テストの数え口: この糸で [`reaches`] を呼んだ回数（覚えを外すと増えるのを見る・振る舞いは変えない）。
    pub(crate) static REACHES_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
#[path = "nesting_kind_tests.rs"]
mod kind_tests;

#[cfg(test)]
#[path = "nesting_visible_tests.rs"]
mod visible_tests;

#[cfg(test)]
#[path = "nesting_report_tests.rs"]
mod report_tests;

#[cfg(test)]
#[path = "nesting_fixture_tests.rs"]
mod fixture_tests;

#[cfg(test)]
#[path = "nesting_film_tests.rs"]
mod film_tests;
