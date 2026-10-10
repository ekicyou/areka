//! `AnimationTable`: SERIKO ループアニメ定義の boot 時不変スナップショット表（構築層）。
//!
//! 表示 surface id → 駆動対象アニメ（interval トリガ＋コマ列）を O(log n) で引ける boot 時不変表。
//! [`EmoWorld`]（fold 済み＝append/ターゲット展開済み）から read-only スナップショットを一度きり
//! 構築し、UI スレッドの World を実行時に参照しない（値渡し・`Send`・要件 4.5/8.1-8.4）。
//!
//! # 採録規則（要件 8.1/8.2）
//!
//! `Interval::Random{k}`／`BindRandom{k}` と、`always` の単独（[`is_always_interval`]・小文字の完全一致）を
//! 採録する（[`LoopTrigger`] の前 3 種・spec: areka-P0-animated-image-playback 要件 4.1・4.8・4.9）。加えて
//! `Interval::Other(語彙)` のうち **`sometimes`・`rarely` の 2 語は採録する**: `sometimes` は
//! `random,2`・`rarely` は `random,4` と同じ引き金へ読み替え、以降の手順（`k == 0` の検査・コマの
//! 整列・空の検査）へそのまま流す（小文字の完全一致・spec: areka-P0-shell-implicit-surface
//! 要件 11.1/11.2）。読み替えたときは元の語（`vocab`）と読み替え先の `k` を `debug!` に残す
//! （要件 11.7）。`Interval::Bind`（静的着せ替え）・**それ以外の** `Other` の語彙・`#[non_exhaustive]`
//! の将来 variant は**採録せず `debug!` で記録**する。非採録の `Other` は**元語彙文字列込み**で記録
//! するため、「`yen-e` と書いたのに動かない」が診断可能になる（討議 #1 裁定・要件 11.4）。
//! `\i[N]`・動的 bind・talk cue は構造的に interval アニメではないため、この採録フィルタが自然に
//! 除外する（要件 8.3）。
//!
//! # `runonce`・`periodic,数値`・`talk,数値`（spec: areka-P0-seriko-trigger-intervals 要件 1.5・7.1・7.2・7.4）
//!
//! 読み手が型へ写した 3 語（`Interval::Runonce`／`Periodic{secs}`／`Talk{n}`）は [`LoopTrigger`] の
//! 後ろ 3 種として採り、採ったときに面の番号・animation の番号・語・数値を `debug!` に 1 回残す。
//! 読み手が数値を読めなかった `talk`／`periodic`（`Other` の先頭の語がその 2 語）は採らず、元の綴りを
//! 添えた `warn!` を 1 回残す。コマ列が空なら下の縮退ガードの `warn!`。`+` の組み合わせ
//! （`bind+runonce` など）と大文字混じりは「それ以外の `Other`」のまま `debug!` だけ。
//!
//! # 縮退ガード（要件 8.3・構築時 1 回・log-first）
//!
//! - `k == 0`（`1/0` は定義不能）→ 非採録・`warn!`。
//! - コマ列が空のアニメ → 非採録・`warn!`。
//! - 待ち時間の合計が 0 の `always`（止まらない繰り返し）→ 非採録・`warn!`（絵は合成が経過 0 として
//!   描く・spec: areka-P0-animated-image-playback 要件 4.6）。
//!
//! # 動く絵の子（spec: areka-P0-animated-image-playback 要件 2.5・8.1〜8.4）
//!
//! 作者のサーフェスの輪の後に、面の表の子の定義（[`EmoWorld::film_sheets`]）1 つにつき `always` を
//! 1 本（animation の番号 0・周期と回数はファイルの値・コマは絵を指す）採る。鍵は
//! [`PartKey::Film`] なので作者のサーフェスの番号とは当たらない。分解しなかった絵
//! （[`EmoWorld::film_skips`]）は 1 件ごとに相対パスと理由を `warn!` で 1 回出す。記録は表を作る
//! ときだけで、刻みごとの記録は無い。
//!
//! # method 解決（要件 8.4）
//!
//! コマの描画メソッドは構築時に [`ComposeMethod::from_name`] で 1 回だけ解決し、完全語彙の型値として
//! 保持する（`overlay`/`add`/`bind` → [`ComposeMethod::Overlay`]・未知名 → `Unknown`）。実際の駆動は
//! 下流 plan の method ゲート（`is_implemented()`）が選別する。コマ列は pattern index 昇順に整列して
//! 保持する（疎 index 許容・kero の 0/1/3 実例）。

use std::collections::BTreeMap;

use std::num::{NonZeroU32, NonZeroU64};

use areka_emo_compose::{
    ComposeMethod, EmoWorld, FilmId, FilmSheet, FilmSkip, NestTable, PartKey, is_always_interval,
};

/// 駆動トリガ（採録はこの 6 種のみ・要件 8.1）。
///
/// `k` は 1/N 抽選の頻度パラメータ（`k == 0` は構築時ガードで弾かれるため、採録済み値は常に `k >= 1`）。
/// 後ろの 3 つ（`runonce`・`periodic,数値`・`talk,数値`）は抽選しない（乱数を引かない・spec:
/// areka-P0-seriko-trigger-intervals）。
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
    /// `interval,always`・動く絵の子: そのサーフェスである間ずっと繰り返す（抽選しない）。
    Always {
        /// 1 周の長さ（手書きは待ち時間の合計・子はコマの待ち時間の合計）。
        period_ms: NonZeroU64,
        /// 合計の回数（`None` は終わりなし。手書きは常に `None`）。
        laps: Option<NonZeroU32>,
    },
    /// `interval,runonce`: 面に切り替わった瞬間に 1 回。
    Runonce,
    /// `interval,periodic,秒`: 面に切り替わった時刻を起点に `period_ms` ごと。
    Periodic {
        /// 周期（秒 × 1000・丸めなし。採録済みは `>= 1000`）。
        period_ms: NonZeroU64,
    },
    /// `interval,talk,文字数`: `every` 文字が現れるごと。
    Talk {
        /// 区切りの文字数（採録済みは `>= 1`）。
        every: NonZeroU32,
    },
}

/// 1 コマ（method は構築時解決済みの完全形・`wait_ms` は 1ms 単位・要件 4.5/8.4）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopFrame {
    /// コマの参照 surface id。負値センチネルを保持する（`-1` 停止・他負値は非駆動・要件 5.5）。
    pub surface_id: i64,
    /// 絵を直接指すコマなら絵の番号（動く絵の子だけ）。作者の pattern定義のコマは `None`。
    pub picture: Option<u32>,
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
    /// 駆動トリガ（`Random`/`BindRandom`/`Always`）。
    pub trigger: LoopTrigger,
    /// pattern index 昇順・非空保証のコマ列（構築時ガード）。
    pub frames: Vec<LoopFrame>,
}

/// 表示 surface id → 採録済みループアニメ列の boot 時不変表（要件 8.1-8.4）。
///
/// 内部表現は `BTreeMap<u32, Vec<LoopAnimation>>`（キー＝表示 surface id）。実行中に変化しない
/// （ghost 再読込は再構築＝spawn し直し）。`Send`・全アニメ frames 非空・`k >= 1` を事後条件とする。
///
/// 入れ子（spec: areka-P0-surface-element-nesting 要件 5.1・7.2・7.3）のために、構築のときに
/// 面の表の [`NestTable`] の写しと「動く部品が在るか」を持つ。seriko は実行中に `EmoWorld` を
/// 見ない（写しだけを引く）。部品で動く animation は [`AnimationTable::animations`] を部品の番号で
/// 引いたものそのまま（一番上に表示したときと同じ列・要件 5.1）。
#[derive(Debug, Clone, Default)]
pub struct AnimationTable {
    animations: BTreeMap<u32, Vec<LoopAnimation>>,
    /// 動く絵の子 → その `always` 1 本。
    films: BTreeMap<FilmId, LoopAnimation>,
    nest: NestTable,
    has_animated_parts: bool,
    /// `always` を 1 本以上採ったか（手書き・子のどちらでも）。
    has_always: bool,
    /// `runonce`・`periodic`・`talk` を 1 本以上採ったか（一番上でも部品でも）。
    has_triggers: bool,
    /// `talk` を 1 本以上採ったか。
    has_talk: bool,
}

/// `talk`／`periodic` の数値が正の整数として読めなかった animation を採らない記録（表を組むときに
/// 1 回・`vocab` は元の綴り・spec: areka-P0-seriko-trigger-intervals 要件 1.5・7.2）。
fn warn_invalid_number(surface_id: u32, animation_id: u32, vocab: &str) {
    tracing::warn!(
        surface_id,
        animation_id,
        vocab,
        "seriko table: talk/periodic の数値が無効ゆえ非採録（要件 1.5・7.2）"
    );
}

impl AnimationTable {
    /// 空表を返す（scope 資産が空のときの明示的な既定・design assets.rs）。
    pub fn empty() -> AnimationTable {
        AnimationTable::default()
    }

    /// [`EmoWorld`] から read-only スナップショットを一度きり構築する（要件 8.1-8.4）。
    ///
    /// 全 surface を昇順に走査し、各 surface の SERIKO animation 群から `Random`/`BindRandom` と、
    /// `random,2`／`random,4` へ読み替える `Other("sometimes")`／`Other("rarely")` を採録する
    /// （要件 11.1/11.2）。`Bind`・それ以外の `Other`・将来 variant は `debug!` で記録して非採録
    /// （`Other` は元語彙込み）。
    /// `k == 0`・コマ列空は `warn!` で記録して非採録。method は [`ComposeMethod::from_name`] で構築時
    /// 1 回解決し、コマ列は pattern index 昇順へ整列する。面種非依存（シェル/バルーン双方に同型適用）。
    pub fn from_world(world: &EmoWorld) -> AnimationTable {
        AnimationTable::from_world_and_films(world, world.film_sheets(), world.film_skips())
    }

    /// [`AnimationTable::from_world`] の中身。子の定義と分解しなかった絵を引数で受ける（seriko は
    /// 読み込みの側に依存しないので、テストは分解の結果と同じ形の値を手で組んで渡す）。
    pub(crate) fn from_world_and_films<'a>(
        world: &EmoWorld,
        sheets: impl Iterator<Item = &'a FilmSheet>,
        skips: &[FilmSkip],
    ) -> AnimationTable {
        let mut table: BTreeMap<u32, Vec<LoopAnimation>> = BTreeMap::new();

        for surface_id in world.surface_ids() {
            let Some(master) = world.surface(surface_id) else {
                continue;
            };

            for anim in &master.animations {
                // 採録は Random/BindRandom・always の単独と、読み替える Other("sometimes")／Other("rarely") のみ。
                // Bind・それ以外の Other・将来 variant は debug! で非採録（要件 8.2・11.1/11.2・
                // match は Bind/Other 明示腕＋catch-all で将来 additive シームを保つ）。
                // `always` の単独は周期が要るので、コマを並べた後で引き金を作る（下・`None`）。
                let trigger = match &anim.interval {
                    i if is_always_interval(i) => None,
                    areka_parsers::shell::Interval::Random { k } => {
                        Some(LoopTrigger::Random { k: *k })
                    }
                    areka_parsers::shell::Interval::BindRandom { k } => {
                        Some(LoopTrigger::BindRandom { k: *k })
                    }
                    areka_parsers::shell::Interval::Runonce => Some(LoopTrigger::Runonce),
                    // 読み手は 1 以上だけを型へ写すが、型は 0 を持てるので 0 は無効の数値として扱う。
                    areka_parsers::shell::Interval::Periodic { secs } => {
                        match NonZeroU64::new(u64::from(*secs) * 1000) {
                            Some(period_ms) => Some(LoopTrigger::Periodic { period_ms }),
                            None => {
                                warn_invalid_number(surface_id, anim.id, "periodic,0");
                                continue;
                            }
                        }
                    }
                    areka_parsers::shell::Interval::Talk { n } => match NonZeroU32::new(*n) {
                        Some(every) => Some(LoopTrigger::Talk { every }),
                        None => {
                            warn_invalid_number(surface_id, anim.id, "talk,0");
                            continue;
                        }
                    },
                    areka_parsers::shell::Interval::Bind => {
                        tracing::debug!(
                            surface_id,
                            animation_id = anim.id,
                            "seriko table: interval,bind は静的着せ替えゆえ非採録（要件 8.2）"
                        );
                        continue;
                    }
                    areka_parsers::shell::Interval::Other(vocab) => {
                        // `sometimes`／`rarely` は `random,2`／`random,4` と同じ引き金へ読み替えて
                        // 採録する（小文字の完全一致・spec: areka-P0-shell-implicit-surface
                        // 要件 11.1/11.2）。先頭の語が `talk`／`periodic` のもの（読み手が数値を
                        // 読めなかった綴り）は `warn!` で非採録（spec: areka-P0-seriko-trigger-intervals
                        // 要件 1.5・7.2）。他の語は今までどおり元語彙込みで記録して非採録＝
                        // 「yen-e と書いたのに動かない」の診断は残る語について生きる（要件 11.4）。
                        let rewritten = match &**vocab {
                            // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#sometimes:1
                            "sometimes" => Some(2),
                            // ukadoc: https://ssp.shillest.net/ukadoc/manual/descript_shell_surfaces.html#rarely:1
                            "rarely" => Some(4),
                            _ => None,
                        };
                        match rewritten {
                            Some(k) => {
                                tracing::debug!(
                                    surface_id,
                                    animation_id = anim.id,
                                    vocab = &**vocab,
                                    k,
                                    "seriko table: 間隔の語を random,K と同じ引き金へ読み替えて採録（要件 11.1/11.2）"
                                );
                                Some(LoopTrigger::Random { k })
                            }
                            None if matches!(
                                vocab.split(',').next(),
                                Some("talk" | "periodic")
                            ) =>
                            {
                                warn_invalid_number(surface_id, anim.id, vocab);
                                continue;
                            }
                            None => {
                                tracing::debug!(
                                    surface_id,
                                    animation_id = anim.id,
                                    vocab = &**vocab,
                                    "seriko table: 未駆動 interval 語彙ゆえ非採録（元語彙保持・要件 8.2/11.4）"
                                );
                                continue;
                            }
                        }
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
                if let Some(LoopTrigger::Random { k: 0 } | LoopTrigger::BindRandom { k: 0 }) =
                    trigger
                {
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
                        picture: None,
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

                let trigger = match trigger {
                    Some(t) => t,
                    None => {
                        // 待ち時間の合計が 0 の `always` は止まらない繰り返しになるので採らない
                        // （絵は合成が経過 0 として描く・spec: areka-P0-animated-image-playback 要件 4.6）。
                        let total: u64 = frames.iter().map(|f| u64::from(f.wait_ms)).sum();
                        let Some(period_ms) = NonZeroU64::new(total) else {
                            tracing::warn!(
                                surface_id,
                                animation_id = anim.id,
                                "seriko table: always の待ち時間の合計が 0 ゆえ非採録（1 周だけ評価した絵のまま・要件 4.6）"
                            );
                            continue;
                        };
                        LoopTrigger::Always {
                            period_ms,
                            laps: None,
                        }
                    }
                };

                // 3 語を採ったことを語と数値つきで 1 回残す（数値は書かれたまま・`runonce` は数値なし・
                // spec: areka-P0-seriko-trigger-intervals 要件 7.1）。
                let adopted: Option<(&str, Option<u64>)> = match trigger {
                    LoopTrigger::Runonce => Some(("runonce", None)),
                    LoopTrigger::Periodic { period_ms } => {
                        Some(("periodic", Some(period_ms.get() / 1000)))
                    }
                    LoopTrigger::Talk { every } => Some(("talk", Some(u64::from(every.get())))),
                    LoopTrigger::Random { .. }
                    | LoopTrigger::BindRandom { .. }
                    | LoopTrigger::Always { .. } => None,
                };
                if let Some((vocab, value)) = adopted {
                    tracing::debug!(
                        surface_id,
                        animation_id = anim.id,
                        vocab,
                        value,
                        "seriko table: 引き金の語を採録（要件 7.1）"
                    );
                }

                table.entry(surface_id).or_default().push(LoopAnimation {
                    id: anim.id,
                    trigger,
                    frames,
                });
            }
        }

        // 動く絵の子: 子 1 つにつき `always` を 1 本（animation の番号 0）。pattern i の待ちはコマ i−1 の
        // 待ち時間（pattern 0 は 0）、周期はコマの待ち時間の合計、回数はファイルの値。
        let mut films: BTreeMap<FilmId, LoopAnimation> = BTreeMap::new();
        for sheet in sheets {
            let total: u64 = sheet.delays_ms.iter().map(|&d| u64::from(d)).sum();
            let Some(period_ms) = NonZeroU64::new(total) else {
                // 分解の検査が合計 0 を落とすので起きないはず（記録の無い失敗の経路を作らない）。
                tracing::warn!(
                    path = sheet.path.as_str(),
                    "seriko table: 動く絵の子の待ち時間の合計が 0 ゆえ非採録（1 枚目の絵のまま）"
                );
                continue;
            };
            let waits = std::iter::once(0).chain(sheet.delays_ms.iter().copied());
            let frames = sheet
                .frames
                .iter()
                .zip(waits)
                .map(|(&picture, wait_ms)| LoopFrame {
                    // 絵を指すコマはサーフェスを指さない（番号の経路へ渡っても何も描かない値）。
                    surface_id: -1,
                    picture: Some(picture),
                    method: ComposeMethod::Overlay,
                    wait_ms,
                    x: 0,
                    y: 0,
                })
                .collect();
            films.insert(
                sheet.id,
                LoopAnimation {
                    id: 0,
                    trigger: LoopTrigger::Always {
                        period_ms,
                        laps: sheet.laps,
                    },
                    frames,
                },
            );
        }
        // 分解しなかった絵: 1 件ごとに 1 回（面の表は絵ごとに 1 件だけ載せる・要件 2.5・8.4）。
        for skip in skips {
            tracing::warn!(
                path = skip.path.as_str(),
                reason = ?skip.reason,
                "seriko table: 動く絵を分解しないので動かさない（1 枚目の静止画・要件 2.5・8.2）"
            );
        }

        // 部品になりうるサーフェス: 参照の表の子・着せ替えの pattern0 の先・`always` の経過 0 の先と、
        // 採った animation のコマが指す 0 以上の番号（描画メソッドでは絞らない・design.md「Table」の
        // 字句どおり。広めに取っても偽にすべき表を真にするだけで、部品の経路の答えは変わらない）。
        // 動く絵の子を置いたサーフェスが在れば、子は `always` を持つので真。
        let nest = world.nest_table();
        let rows = || world.surface_ids().filter_map(|id| nest.parts(id));
        let nested = rows().flat_map(|p| {
            p.children
                .iter()
                .copied()
                .chain(p.bind_targets.iter().map(|&(_, target)| target))
                .chain(p.always_rest.iter().map(|&(_, target)| target))
        });
        let framed = table
            .values()
            .flatten()
            .flat_map(|a| &a.frames)
            .filter_map(|f| u32::try_from(f.surface_id).ok());
        let has_animated_parts = nested.chain(framed).any(|id| table.contains_key(&id))
            || rows().any(|p| p.films.iter().any(|f| films.contains_key(f)));
        let has_always = !films.is_empty()
            || table
                .values()
                .flatten()
                .any(|a| matches!(a.trigger, LoopTrigger::Always { .. }));
        let has_talk = table
            .values()
            .flatten()
            .any(|a| matches!(a.trigger, LoopTrigger::Talk { .. }));
        let has_triggers = has_talk
            || table.values().flatten().any(|a| {
                matches!(
                    a.trigger,
                    LoopTrigger::Runonce | LoopTrigger::Periodic { .. }
                )
            });

        AnimationTable {
            animations: table,
            films,
            nest,
            has_animated_parts,
            has_always,
            has_triggers,
            has_talk,
        }
    }

    /// 指定 surface id の採録済みアニメ列を引く（不在は空スライス）。
    pub fn animations(&self, surface_id: u32) -> &[LoopAnimation] {
        self.animations
            .get(&surface_id)
            .map_or(&[][..], Vec::as_slice)
    }

    /// 部品の animation の列（作者のサーフェスでも動く絵の子でも・不在は空スライス）。
    pub fn part_animations(&self, part: PartKey) -> &[LoopAnimation] {
        match part {
            PartKey::Surface(id) => self.animations(id),
            PartKey::Film(film) => self.films.get(&film).map_or(&[][..], std::slice::from_ref),
        }
    }

    /// 繰り返しの経路を通す表か: 動く部品が在る・`always` を 1 本以上採った・`runonce`／`periodic`／
    /// `talk` を 1 本以上採った、のどれか（偽の表では足した経路を通らない・spec:
    /// areka-P0-animated-image-playback 要件 7.1・7.2・spec: areka-P0-seriko-trigger-intervals 要件 8.1）。
    pub fn is_continuous(&self) -> bool {
        self.has_animated_parts || self.has_always || self.has_triggers
    }

    /// `talk` を 1 本でも採ったか（文字の cue を写すかの門・一番上でも部品でも）。
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "読むのは一番上の配線（文字の cue の写し）を入れるとき"
        )
    )]
    pub(crate) fn has_talk(&self) -> bool {
        self.has_talk
    }

    /// 作者のサーフェスの採録済みアニメを 1 本も持たない（表が空）か（動く絵の子の行は数えない）。
    pub fn is_empty(&self) -> bool {
        self.animations.is_empty()
    }

    /// 構築のときに受け取った面の表の参照の表の写し（入れ子の無いシェルでは空）。
    pub fn nest_table(&self) -> &NestTable {
        &self.nest
    }

    /// 部品になりうるサーフェスのうち、採った animation を 1 本でも持つものが在るか
    /// （偽なら刻みも切り替えも部品の経路を通らない・要件 7.2・7.3）。
    pub fn has_animated_parts(&self) -> bool {
        self.has_animated_parts
    }
}

#[cfg(test)]
#[path = "table_interval_words_tests.rs"]
mod interval_words_tests;

#[cfg(test)]
#[path = "table_parts_tests.rs"]
mod parts_tests;

#[cfg(test)]
#[path = "table_always_tests.rs"]
mod always_tests;

#[cfg(test)]
#[path = "table_film_tests.rs"]
mod film_tests;

#[cfg(test)]
#[path = "table_trigger_tests.rs"]
mod trigger_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use areka_emo_compose::{BlendKind, BlendMode};
    use areka_parsers::shell::{
        Animation, AppendTarget, DefRef, DrawMethod, Interval, Pattern, Shell, Surface,
    };
    use log_capture_kit::{LineFormat, capture_lines};

    /// テスト専用 tracing 捕捉ハーネス（硬化機構の唯一の定義元 `log-capture-kit` へ委譲）。
    /// 1 イベント 1 行へ level／target／各フィールド（`name=value`）を整形し、改行連結で返す。
    ///
    /// **「`with_default` はスレッドローカルゆえ並行テスト安全」は誤り**である。差し替わるのは
    /// スレッドローカルの既定 dispatcher だけで、「そのログを評価するか」を決める callsite の
    /// interest キャッシュはプロセス全体で 1 つしかなく、その発行点を最初に踏んだスレッドの
    /// 判定が焼き付く。捕捉窓を持たないスレッド（既定は `NoSubscriber`）が先に踏むと `never`
    /// が大域へ焼き付き、自分のスレッドへ捕捉先を差していても取りこぼす。共有機構は
    /// ⑴ probe 常駐 ⑵ 窓内の interest 再計算 ⑶ 番兵による空振り検出でこれを塞ぐ。機序の
    /// 逐条解説は `log_capture_kit` の crate doc および同 crate の `src/probe.rs` にある。
    fn capture_logs<F: FnOnce()>(f: F) -> String {
        let ((), lines) = capture_lines(LineFormat::LevelTargetFields, f);
        lines.join("\n")
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
    /// 世界から、採録されるのは Random/BindRandom の 2 アニメのみ。Bind と**読み替えの対象でない**
    /// `Other` は非採録かつ `debug!` で記録され、`Other` は元語彙 `yen-e` を含む（要件 8.1/8.2・
    /// 討議 #1）。読み替えの対象 2 語（`sometimes`・`rarely`）は採録側であり、別の檻
    /// `table_interval_words_tests.rs` が留める（要件 11.1/11.2/11.4）。
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
        // 非駆動: surface30 に Bind（静的着せ替え）と Other("yen-e")（読み替えの対象でない語彙）。
        let non_driven = surface_with(
            30,
            vec![
                anim(1, Interval::Bind, vec![pat(0, "overlay", 1100, 0, 0, 0)]),
                anim(
                    2,
                    Interval::Other("yen-e".into()),
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

        // debug! ログ: Bind と Other が非採録として記録され、Other は元語彙 yen-e を含む。
        assert!(
            logs.contains("level=DEBUG"),
            "非採録は debug! で記録: {logs}"
        );
        assert!(
            logs.contains("interval,bind は静的着せ替え"),
            "Bind の非採録が debug! 記録される: {logs}"
        );
        assert!(
            logs.contains("yen-e"),
            "Other の元語彙 yen-e が debug! に記録される（討議 #1）: {logs}"
        );
        assert!(
            logs.contains("vocab=\"yen-e\""),
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

    /// 採録済み表の全アニメは frames 非空・抽選は `k >= 1`・`always` は周期＝待ちの合計（≥ 1）
    /// （事後条件・design Postconditions）。
    #[test]
    fn recorded_anims_satisfy_postconditions() {
        let kero = surface_with(
            10,
            vec![
                anim(
                    0,
                    Interval::Random { k: 4 },
                    vec![pat(0, "overlay", 2106, 0, 0, 0)],
                ),
                anim(
                    1,
                    Interval::Other("always".into()),
                    vec![
                        pat(0, "overlay", 2106, 0, 0, 0),
                        pat(1, "overlay", 2110, 40, 0, 0),
                    ],
                ),
            ],
        );
        let world = world_of(vec![kero]);
        let table = AnimationTable::from_world(&world);
        assert_eq!(table.animations(10).len(), 2);
        for anim in table.animations(10) {
            assert!(!anim.frames.is_empty(), "採録アニメの frames は非空");
            match anim.trigger {
                LoopTrigger::Random { k } | LoopTrigger::BindRandom { k } => {
                    assert!(k >= 1, "採録アニメの k は >= 1");
                }
                LoopTrigger::Always { period_ms, .. } => {
                    let total: u64 = anim.frames.iter().map(|f| u64::from(f.wait_ms)).sum();
                    assert_eq!(period_ms.get(), total, "周期は待ちの合計");
                }
                // この検体は 3 語を書いていない（型が 0 を持てないので確かめる値も無い）。
                LoopTrigger::Runonce | LoopTrigger::Periodic { .. } | LoopTrigger::Talk { .. } => {}
            }
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
