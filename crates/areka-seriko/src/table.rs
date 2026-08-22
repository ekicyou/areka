//! `AnimationTable`: SERIKO ループアニメ定義の boot 時不変スナップショット表（構築層）。
//!
//! 表示 surface id → 駆動対象アニメ（interval トリガ＋コマ列）を O(log n) で引ける boot 時不変表。
//! [`EmoWorld`]（fold 済み＝append/ターゲット展開済み）から read-only スナップショットを一度きり
//! 構築し、UI スレッドの World を実行時に参照しない（値渡し・`Send`・要件 4.5/8.1-8.4）。
//!
//! # 採録規則（要件 8.1/8.2）
//!
//! `Interval::Random{k}`／`BindRandom{k}` のみ採録する（[`LoopTrigger`] 2 種）。`Interval::Bind`
//! （静的着せ替え）・`Interval::Other(語彙)`（未駆動 interval 語彙）および `#[non_exhaustive]` の
//! 将来 variant は**採録せず `debug!` で記録**する。`Other` は**元語彙文字列込み**で記録するため、
//! 「sometimes と書いたのに動かない」が診断可能になる（討議 #1 裁定）。口パク（`interval,talk`）・
//! `\i[N]`・動的 bind・talk cue は構造的に `Random`/`BindRandom` の interval アニメではないため、
//! この採録フィルタが自然に除外する（要件 8.3）。
//!
//! # 縮退ガード（要件 8.3・構築時 1 回・log-first）
//!
//! - `k == 0`（`1/0` は定義不能）→ 非採録・`warn!`。
//! - コマ列が空のアニメ → 非採録・`warn!`。
//!
//! # method 解決（要件 8.4）
//!
//! コマの描画メソッドは構築時に [`ComposeMethod::from_name`] で 1 回だけ解決し、完全語彙の型値として
//! 保持する（`overlay`/`add`/`bind` → [`ComposeMethod::Overlay`]・未知名 → `Unknown`）。実際の駆動は
//! 下流 plan の method ゲート（`is_implemented()`）が選別する。コマ列は pattern index 昇順に整列して
//! 保持する（疎 index 許容・kero の 0/1/3 実例）。

use std::collections::BTreeMap;

use areka_emo_compose::{ComposeMethod, EmoWorld};

/// 駆動トリガ（採録は 2 種のみ・要件 8.1）。
///
/// `k` は 1/N 抽選の頻度パラメータ（`k == 0` は構築時ガードで弾かれるため、採録済み値は常に `k >= 1`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopTrigger {
    /// `interval,random,K`（要件 5.2）。
    Random {
        /// 頻度パラメータ K（採録済みは `>= 1`）。
        k: u32,
    },
    /// `interval,bind+random,K`（要件 5.3）。
    BindRandom {
        /// 頻度パラメータ K（採録済みは `>= 1`）。
        k: u32,
    },
}

/// 1 コマ（method は構築時解決済みの完全形・`wait_ms` は 1ms 単位・要件 4.5/8.4）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopFrame {
    /// コマの参照 surface id。負値センチネルを保持する（`-1` 停止・他負値は非駆動・要件 5.5）。
    pub surface_id: i64,
    /// 描画メソッド（完全語彙・要件 8.4）。合成の実駆動は `Overlay` のみ（下流 method ゲートが選別）。
    pub method: ComposeMethod,
    /// 前コマからこのコマへの遅延（1ms 単位・要件 4.5）。
    pub wait_ms: u32,
    /// コマの X オフセット。
    pub x: i64,
    /// コマの Y オフセット。
    pub y: i64,
}

/// 1 本の採録済みループアニメ（interval トリガ＋非空コマ列）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopAnimation {
    /// animation ID（bind ゲート・合成整列のキー・要件 5.6）。
    pub id: u32,
    /// 駆動トリガ（`Random`/`BindRandom`）。
    pub trigger: LoopTrigger,
    /// pattern index 昇順・非空保証のコマ列（構築時ガード）。
    pub frames: Vec<LoopFrame>,
}

/// 表示 surface id → 採録済みループアニメ列の boot 時不変表（要件 8.1-8.4）。
///
/// 内部表現は `BTreeMap<u32, Vec<LoopAnimation>>`（キー＝表示 surface id）。実行中に変化しない
/// （ghost 再読込は再構築＝spawn し直し）。`Send`・全アニメ frames 非空・`k >= 1` を事後条件とする。
#[derive(Debug, Clone, Default)]
pub struct AnimationTable(BTreeMap<u32, Vec<LoopAnimation>>);

impl AnimationTable {
    /// 空表を返す（scope 資産が空のときの明示的な既定・design assets.rs）。
    pub fn empty() -> AnimationTable {
        AnimationTable(BTreeMap::new())
    }

    /// [`EmoWorld`] から read-only スナップショットを一度きり構築する（要件 8.1-8.4）。
    ///
    /// 全 surface を昇順に走査し、各 surface の SERIKO animation 群から `Random`/`BindRandom` のみを
    /// 採録する。`Bind`/`Other`/将来 variant は `debug!` で記録して非採録（`Other` は元語彙込み）。
    /// `k == 0`・コマ列空は `warn!` で記録して非採録。method は [`ComposeMethod::from_name`] で構築時
    /// 1 回解決し、コマ列は pattern index 昇順へ整列する。面種非依存（シェル/バルーン双方に同型適用）。
    pub fn from_world(world: &EmoWorld) -> AnimationTable {
        let mut table: BTreeMap<u32, Vec<LoopAnimation>> = BTreeMap::new();

        for surface_id in world.surface_ids() {
            let Some(master) = world.surface(surface_id) else {
                continue;
            };

            for anim in &master.animations {
                // 採録は Random/BindRandom のみ。Bind/Other/将来 variant は debug! で非採録
                // （要件 8.2・match は Bind/Other 明示腕＋catch-all で将来 additive シームを保つ）。
                let trigger = match &anim.interval {
                    areka_parsers::shell::Interval::Random { k } => LoopTrigger::Random { k: *k },
                    areka_parsers::shell::Interval::BindRandom { k } => {
                        LoopTrigger::BindRandom { k: *k }
                    }
                    areka_parsers::shell::Interval::Bind => {
                        tracing::debug!(
                            surface_id,
                            animation_id = anim.id,
                            "seriko table: interval,bind は静的着せ替えゆえ非採録（要件 8.2）"
                        );
                        continue;
                    }
                    areka_parsers::shell::Interval::Other(vocab) => {
                        // 元語彙文字列込みで記録＝「sometimes と書いたのに動かない」が診断可能（討議 #1）。
                        tracing::debug!(
                            surface_id,
                            animation_id = anim.id,
                            vocab = &**vocab,
                            "seriko table: 未駆動 interval 語彙ゆえ非採録（元語彙保持・要件 8.2）"
                        );
                        continue;
                    }
                    other => {
                        tracing::debug!(
                            surface_id,
                            animation_id = anim.id,
                            interval = ?other,
                            "seriko table: 未知 interval variant は非採録（将来 additive シーム・要件 8.2）"
                        );
                        continue;
                    }
                };

                // 縮退ガード（構築時 1 回・log-first・要件 8.3）: k==0 は 1/0 定義不能ゆえ非採録。
                let (LoopTrigger::Random { k } | LoopTrigger::BindRandom { k }) = trigger;
                if k == 0 {
                    tracing::warn!(
                        surface_id,
                        animation_id = anim.id,
                        "seriko table: k==0 は 1/N 抽選が定義不能ゆえ非採録（要件 8.3）"
                    );
                    continue;
                }

                // コマ列を pattern index 昇順へ整列（疎 index 許容）し、method を構築時 1 回解決する
                // （完全語彙の型値・要件 8.4）。
                let mut patterns: Vec<&areka_parsers::shell::Pattern> =
                    anim.patterns.iter().collect();
                patterns.sort_by_key(|p| p.index);
                let frames: Vec<LoopFrame> = patterns
                    .iter()
                    .map(|p| LoopFrame {
                        surface_id: p.surface_id,
                        method: ComposeMethod::from_name(p.method.as_str()),
                        wait_ms: p.wait,
                        x: p.x,
                        y: p.y,
                    })
                    .collect();

                // 縮退ガード（要件 8.3）: コマ列空のアニメは非採録。
                if frames.is_empty() {
                    tracing::warn!(
                        surface_id,
                        animation_id = anim.id,
                        "seriko table: コマ列が空のアニメは非採録（要件 8.3）"
                    );
                    continue;
                }

                table.entry(surface_id).or_default().push(LoopAnimation {
                    id: anim.id,
                    trigger,
                    frames,
                });
            }
        }

        AnimationTable(table)
    }

    /// 指定 surface id の採録済みアニメ列を引く（不在は空スライス）。
    pub fn animations(&self, surface_id: u32) -> &[LoopAnimation] {
        self.0.get(&surface_id).map_or(&[][..], Vec::as_slice)
    }

    /// 採録済みアニメを 1 本も持たない（表が空）か。
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use areka_emo_compose::{BlendKind, BlendMode};
    use areka_parsers::shell::{
        Animation, AppendTarget, DefRef, DrawMethod, Interval, Pattern, Shell, Surface,
    };

    /// テスト専用 tracing 捕捉ハーネス（emo-compose/actor.rs の log_capture 流儀・
    /// スレッドローカル `with_default` ゆえ並行テスト安全）。1 イベント 1 行へ level／target／
    /// 各フィールド（`name=value`）を整形し、改行連結で返す。
    fn capture_logs<F: FnOnce()>(f: F) -> String {
        use std::sync::{Arc, Mutex};
        use tracing::field::{Field, Visit};
        use tracing_subscriber::prelude::*;

        #[derive(Clone, Default)]
        struct Capture(Arc<Mutex<Vec<String>>>);

        impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for Capture {
            fn on_event(
                &self,
                ev: &tracing::Event<'_>,
                _: tracing_subscriber::layer::Context<'_, S>,
            ) {
                let meta = ev.metadata();
                let mut line = format!("level={} target={}", meta.level(), meta.target());
                struct V<'a>(&'a mut String);
                impl Visit for V<'_> {
                    fn record_debug(&mut self, f: &Field, v: &dyn std::fmt::Debug) {
                        use std::fmt::Write;
                        let _ = write!(self.0, " {}={:?}", f.name(), v);
                    }
                }
                ev.record(&mut V(&mut line));
                self.0.lock().unwrap().push(line);
            }
        }

        let cap = Capture::default();
        let logs = cap.0.clone();
        let subscriber = tracing_subscriber::registry().with(cap);
        tracing::subscriber::with_default(subscriber, f);
        let guard = logs.lock().unwrap();
        guard.join("\n")
    }

    /// コマ 1 本（index/method/surface_id/wait/x/y）。
    fn pat(index: u32, method: &str, surface_id: i64, wait: u32, x: i64, y: i64) -> Pattern {
        Pattern {
            index,
            method: DrawMethod::new(method.to_string()),
            surface_id,
            wait,
            x,
            y,
        }
    }

    /// animation 1 本（id/interval/patterns）。
    fn anim(id: u32, interval: Interval, patterns: Vec<Pattern>) -> Animation {
        Animation {
            id,
            interval,
            patterns,
        }
    }

    /// animation 群を持つ plain surface 1 件（単一 id 形）。
    fn surface_with(id: u32, animations: Vec<Animation>) -> Surface {
        Surface {
            id,
            targets: vec![AppendTarget::Single(id)],
            elements: Vec::new(),
            collisions: Vec::new(),
            animations,
        }
    }

    /// surfaces を 1 対 1（登場順）で definitions に対応させた `Shell` を build して `EmoWorld` を得る。
    fn world_of(surfaces: Vec<Surface>) -> EmoWorld {
        let definitions = (0..surfaces.len()).map(DefRef::Surface).collect();
        let shell = Shell {
            surfaces,
            appends: Vec::new(),
            aliases: Vec::new(),
            animation_sort: None,
            collision_sort: None,
            definitions,
        };
        EmoWorld::build(&shell)
    }

    // ── 3 点完了ケージ (1): 採録フィルタ ──────────────────────────────────────

    /// (1) kero(`interval,random,4`)＋sakura(`interval,bind+random,4`)＋非駆動(Bind/Other) を含む
    /// 世界から、採録されるのは Random/BindRandom の 2 アニメのみ。Bind/Other は非採録かつ `debug!`
    /// で記録され、`Other` は元語彙 `sometimes` を含む（要件 8.1/8.2・討議 #1）。
    #[test]
    fn only_random_and_bindrandom_are_recorded_others_debug_logged() {
        // kero: surface10・animation0・random,4・疎 index 0/1/3（frames 2106→2110→-1 停止）。
        let kero = surface_with(
            10,
            vec![anim(
                0,
                Interval::Random { k: 4 },
                vec![
                    pat(0, "overlay", 2106, 0, 0, 0),
                    pat(1, "overlay", 2110, 40, 0, 0),
                    pat(3, "overlay", -1, 80, 0, 0),
                ],
            )],
        );
        // sakura: surface20・animation7・bind+random,4・index 1/2/3（frames 1412→1411→1410）。
        let sakura = surface_with(
            20,
            vec![anim(
                7,
                Interval::BindRandom { k: 4 },
                vec![
                    pat(1, "overlay", 1412, 0, 0, 0),
                    pat(2, "overlay", 1411, 150, 0, 0),
                    pat(3, "overlay", 1410, 22, 0, 0),
                ],
            )],
        );
        // 非駆動: surface30 に Bind（静的着せ替え）と Other("sometimes")（未駆動語彙）。
        let non_driven = surface_with(
            30,
            vec![
                anim(1, Interval::Bind, vec![pat(0, "overlay", 1100, 0, 0, 0)]),
                anim(
                    2,
                    Interval::Other("sometimes".into()),
                    vec![pat(0, "overlay", 1200, 0, 0, 0)],
                ),
            ],
        );

        let world = world_of(vec![kero, sakura, non_driven]);

        let mut table = None;
        let logs = capture_logs(|| {
            table = Some(AnimationTable::from_world(&world));
        });
        let table = table.unwrap();

        // kero: 1 アニメ・Random{4}・pattern index 昇順（0/1/3・疎許容）。
        let kero_anims = table.animations(10);
        assert_eq!(kero_anims.len(), 1, "surface10 は kero アニメ 1 本のみ採録");
        assert_eq!(kero_anims[0].id, 0);
        assert_eq!(kero_anims[0].trigger, LoopTrigger::Random { k: 4 });
        let kero_surfaces: Vec<i64> = kero_anims[0].frames.iter().map(|f| f.surface_id).collect();
        assert_eq!(
            kero_surfaces,
            vec![2106, 2110, -1],
            "index 昇順で 2106→2110→-1（停止センチネル保持）"
        );
        let kero_waits: Vec<u32> = kero_anims[0].frames.iter().map(|f| f.wait_ms).collect();
        assert_eq!(
            kero_waits,
            vec![0, 40, 80],
            "wait は 1ms 単位で保持（要件 4.5）"
        );

        // sakura: 1 アニメ・BindRandom{4}・index 昇順 1/2/3。
        let sakura_anims = table.animations(20);
        assert_eq!(
            sakura_anims.len(),
            1,
            "surface20 は sakura アニメ 1 本のみ採録"
        );
        assert_eq!(sakura_anims[0].id, 7);
        assert_eq!(sakura_anims[0].trigger, LoopTrigger::BindRandom { k: 4 });
        let sakura_surfaces: Vec<i64> = sakura_anims[0]
            .frames
            .iter()
            .map(|f| f.surface_id)
            .collect();
        assert_eq!(sakura_surfaces, vec![1412, 1411, 1410]);

        // 非駆動 surface30 は 1 本も採録されない（Bind/Other 双方非採録）。
        assert!(
            table.animations(30).is_empty(),
            "Bind/Other は非採録＝surface30 は空"
        );

        // 全体で採録アニメは 2 本のみ（kero+sakura）。
        assert!(!table.is_empty());

        // debug! ログ: Bind と Other が非採録として記録され、Other は元語彙 sometimes を含む。
        assert!(
            logs.contains("level=DEBUG"),
            "非採録は debug! で記録: {logs}"
        );
        assert!(
            logs.contains("interval,bind は静的着せ替え"),
            "Bind の非採録が debug! 記録される: {logs}"
        );
        assert!(
            logs.contains("sometimes"),
            "Other の元語彙 sometimes が debug! に記録される（討議 #1）: {logs}"
        );
        assert!(
            logs.contains("vocab=\"sometimes\""),
            "元語彙は discriminating field vocab として載る: {logs}"
        );
    }

    // ── 3 点完了ケージ (2): 縮退ガード（k==0／コマ列空） ────────────────────────

    /// (2) `k == 0`（1/0 定義不能）とコマ列空のアニメは非採録かつ `warn!`（要件 8.3）。
    #[test]
    fn zero_k_and_empty_frames_are_not_recorded_with_warn() {
        // surface40: k==0（コマは有るが k 不正）と、コマ列空（k は有効だが frames 空）。
        let degenerate = surface_with(
            40,
            vec![
                anim(
                    1,
                    Interval::Random { k: 0 },
                    vec![pat(0, "overlay", 999, 0, 0, 0)],
                ),
                anim(2, Interval::Random { k: 4 }, Vec::new()),
            ],
        );

        let world = world_of(vec![degenerate]);
        let mut table = None;
        let logs = capture_logs(|| {
            table = Some(AnimationTable::from_world(&world));
        });
        let table = table.unwrap();

        // どちらも非採録＝表は空。
        assert!(table.animations(40).is_empty(), "k==0／コマ空は非採録");
        assert!(table.is_empty(), "採録アニメ皆無＝表は空");

        // warn! が両ガードで発火する。
        assert!(
            logs.contains("level=WARN"),
            "縮退ガードは warn! で記録: {logs}"
        );
        assert!(
            logs.contains("k==0 は 1/N 抽選が定義不能"),
            "k==0 の非採録 warn!: {logs}"
        );
        assert!(
            logs.contains("コマ列が空のアニメは非採録"),
            "コマ列空の非採録 warn!: {logs}"
        );
    }

    // ── 3 点完了ケージ (3): method 解決＋pattern index 昇順整列 ──────────────────

    /// (3) method は構築時に `ComposeMethod::from_name` で解決される（overlay→Overlay・
    /// add/bind 同義→Overlay・base→Base・blend-multiply→Blend・未知→Unknown・要件 8.4）。
    /// コマは宣言順が乱れていても pattern index 昇順へ整列される（疎 index 許容）。
    #[test]
    fn method_is_resolved_and_frames_sorted_by_pattern_index() {
        // surface50: 宣言順を index 降順で与え（整列を証明）、多彩な method 語彙を混ぜる。
        let s = surface_with(
            50,
            vec![anim(
                3,
                Interval::Random { k: 2 },
                vec![
                    pat(5, "someFutureMethod", 505, 0, 0, 0), // 未知 → Unknown
                    pat(2, "blend-multiply", 502, 0, 0, 0),   // blend → Blend(Multiply)
                    pat(0, "overlay", 500, 0, 0, 0),          // overlay → Overlay
                    pat(1, "add", 501, 0, 0, 0),              // add 同義 → Overlay
                    pat(4, "base", 504, 0, 0, 0),             // base → Base（シーム・未実装）
                ],
            )],
        );

        let world = world_of(vec![s]);
        let table = AnimationTable::from_world(&world);

        let anims = table.animations(50);
        assert_eq!(anims.len(), 1);
        let frames = &anims[0].frames;

        // pattern index 昇順へ整列（宣言順 5/2/0/1/4 → 0/1/2/4/5）。疎 index（3 欠番）許容。
        let ids: Vec<i64> = frames.iter().map(|f| f.surface_id).collect();
        assert_eq!(
            ids,
            vec![500, 501, 502, 504, 505],
            "pattern index 昇順に整列（疎許容）"
        );

        // method 解決（完全語彙の型値）。
        assert_eq!(frames[0].method, ComposeMethod::Overlay, "overlay→Overlay");
        assert_eq!(frames[1].method, ComposeMethod::Overlay, "add 同義→Overlay");
        assert_eq!(
            frames[2].method,
            ComposeMethod::Blend(BlendMode::new(BlendKind::Multiply)),
            "blend-multiply→Blend(Multiply)"
        );
        assert_eq!(frames[3].method, ComposeMethod::Base, "base→Base（シーム）");
        assert_eq!(
            frames[4].method,
            ComposeMethod::Unknown("someFutureMethod".into()),
            "未知→Unknown（原文保持）"
        );

        // 駆動判定は Overlay のみ（下流 method ゲートの土台・要件 8.4）。
        assert!(frames[0].method.is_implemented());
        assert!(frames[1].method.is_implemented());
        assert!(!frames[2].method.is_implemented());
        assert!(!frames[3].method.is_implemented());
        assert!(!frames[4].method.is_implemented());
    }

    // ── 補助 API ／ 事後条件 ──────────────────────────────────────────────────

    /// `empty()` は空表・`animations` は不在 id で空スライス・`is_empty` は真。
    #[test]
    fn empty_table_api() {
        let t = AnimationTable::empty();
        assert!(t.is_empty());
        assert!(t.animations(0).is_empty());
        assert!(t.animations(9999).is_empty());
    }

    /// 採録済み表の全アニメは frames 非空・`k >= 1`（事後条件・design Postconditions）。
    #[test]
    fn recorded_anims_satisfy_postconditions() {
        let kero = surface_with(
            10,
            vec![anim(
                0,
                Interval::Random { k: 4 },
                vec![pat(0, "overlay", 2106, 0, 0, 0)],
            )],
        );
        let world = world_of(vec![kero]);
        let table = AnimationTable::from_world(&world);
        for anim in table.animations(10) {
            assert!(!anim.frames.is_empty(), "採録アニメの frames は非空");
            let (LoopTrigger::Random { k } | LoopTrigger::BindRandom { k }) = anim.trigger;
            assert!(k >= 1, "採録アニメの k は >= 1");
        }
    }

    /// `AnimationTable` がスレッド越え所有に耐える（`Send`・design Postconditions）。
    #[test]
    fn table_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<AnimationTable>();
        assert_send::<LoopAnimation>();
        assert_send::<LoopFrame>();
        assert_send::<LoopTrigger>();
    }
}
