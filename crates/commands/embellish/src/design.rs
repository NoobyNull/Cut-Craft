//! What both designs share: the settings, the pieces placed so far and the grid that keeps
//! them apart.

use crate::field::Field;
use crate::motif::{Ctx, CurlSpec, Item, circ_n, poisson};
use crate::occ::Occ;
use crate::rng::Rng;
use cut_craft_core::geometry::Point;
use std::f64::consts::TAU;

/// Where the ornament goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Where {
    Fill,
    Border,
    Both,
}

/// The settings, as the form gives them (each 0 to 1 unless said).
#[derive(Clone, Copy, Debug)]
pub struct Settings {
    /// The smallest gap between pieces, in mm.
    pub gap: f64,
    pub detail: f64,
    pub density: f64,
    /// How big scrolls may grow, 1 as designed.
    pub scale: f64,
    pub place: Where,
    pub stipple: bool,
    pub seed: u32,
}

pub struct Gen<'a> {
    pub field: &'a Field,
    pub occ: Occ,
    pub out: Vec<Item>,
    pub ctx: Ctx,
    pub s: Settings,
}

impl<'a> Gen<'a> {
    /// Pieces may go where `inside(p, gap)`.
    pub fn new(field: &'a Field, s: Settings, inside: impl Fn(Point, f64) -> bool) -> Self {
        Self {
            field,
            occ: Occ::new(field.w, field.h, s.gap, inside),
            out: vec![],
            ctx: Ctx {
                g: s.gap,
                detail: s.detail,
                rng: Rng::new(s.seed),
            },
            s,
        }
    }

    pub fn rng(&mut self) -> &mut Rng {
        &mut self.ctx.rng
    }

    pub fn r(&mut self, a: f64, b: f64) -> f64 {
        self.ctx.rng.range(a, b)
    }

    pub fn chance(&mut self, p: f64) -> bool {
        self.ctx.rng.chance(p)
    }

    /// Keeps `items` if they fit (or when `force`).
    pub fn add(&mut self, items: Vec<Item>, force: bool) -> bool {
        if items.is_empty() || (!force && !self.occ.fits(&items)) {
            return false;
        }
        self.occ.mark(&items);
        self.out.extend(items);
        true
    }

    /// A scroll's curl of size `s`: `lead` and `lead_k` are shares of `s`.
    pub fn curl(
        &mut self,
        s: f64,
        rho: (f64, f64),
        b: (f64, f64),
        lead: (f64, f64),
        lead_k: f64,
        wmax: f64,
        turn: (f64, f64),
        taper: (f64, f64),
    ) -> CurlSpec {
        CurlSpec {
            rho0: s * self.r(rho.0, rho.1),
            b: self.r(b.0, b.1),
            lead: s * self.r(lead.0, lead.1),
            lead_k: self.r(-lead_k, lead_k) / s,
            wmax,
            g: self.s.gap,
            max_turn: TAU * self.r(turn.0, turn.1),
            taper: s * self.r(taper.0, taper.1),
            ds: (s / 160.0).max(0.25),
        }
    }

    /// Small sunk dots over the free ground.
    pub fn stipple(&mut self) {
        if !self.s.stipple {
            return;
        }
        let (w, h, g) = (self.field.w, self.field.h, self.s.gap);
        let pts = poisson(w, h, 2.4, self.rng());
        for q in pts {
            if self.occ.free(q, 0.55 + g * 0.6) {
                self.out.push(Item::Shape {
                    rings: vec![circ_n(q, 0.55, 10)],
                    h: 0.35,
                    profile: crate::motif::Profile::Ball,
                    sink: true,
                });
            }
        }
    }
}
