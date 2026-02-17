use std::{
    fmt::{Display, Formatter},
    ops::{Index, IndexMut},
};

use comfy_table::Color;
use derive_more::AddAssign;
use enumset::EnumSetType;

use crate::prelude::*;

#[derive(Debug, Hash, clap::ValueEnum, EnumSetType)]
pub enum WorkingMode {
    /// Do not do anything.
    Idle,

    /// Only excess solar power charging without discharging.
    Harvest,

    /// Charge on excess solar power, compensate on insufficient solar power.
    SelfUse,

    /// Forced charging from any source.
    Charge,

    /// Forced discharging, no matter the actual consumption.
    Discharge,
}

impl Display for WorkingMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::SelfUse => "Self-use",
            Self::Idle => "Idle",
            Self::Harvest => "Harvest",
            Self::Charge => "Charge",
            Self::Discharge => "Discharge",
        };
        text.fmt(f)
    }
}

impl WorkingMode {
    pub const fn color(self) -> Color {
        match self {
            Self::Charge => Color::Green,
            Self::Discharge => Color::Blue,
            Self::SelfUse => Color::DarkYellow,
            Self::Harvest => Color::Cyan,
            Self::Idle => Color::Reset,
        }
    }
}

#[derive(Copy, Clone, Default, AddAssign)]
pub struct WorkingModeMap<V> {
    pub idle: V,
    pub harvest: V,
    pub self_use: V,
    pub charge: V,
    pub discharge: V,
}

impl<V> WorkingModeMap<V> {
    pub fn new(map: impl Fn(WorkingMode) -> V) -> Self {
        Self {
            idle: map(WorkingMode::Idle),
            harvest: map(WorkingMode::Harvest),
            self_use: map(WorkingMode::SelfUse),
            charge: map(WorkingMode::Charge),
            discharge: map(WorkingMode::Discharge),
        }
    }

    pub fn try_new(map: impl Fn(WorkingMode) -> Result<V>) -> Result<Self> {
        Ok(Self {
            idle: map(WorkingMode::Idle)?,
            harvest: map(WorkingMode::Harvest)?,
            self_use: map(WorkingMode::SelfUse)?,
            charge: map(WorkingMode::Charge)?,
            discharge: map(WorkingMode::Discharge)?,
        })
    }
}

impl<V> Index<WorkingMode> for WorkingModeMap<V> {
    type Output = V;

    fn index(&self, working_mode: WorkingMode) -> &Self::Output {
        match working_mode {
            WorkingMode::Idle => &self.idle,
            WorkingMode::Harvest => &self.harvest,
            WorkingMode::SelfUse => &self.self_use,
            WorkingMode::Charge => &self.charge,
            WorkingMode::Discharge => &self.discharge,
        }
    }
}

impl<V> IndexMut<WorkingMode> for WorkingModeMap<V> {
    fn index_mut(&mut self, working_mode: WorkingMode) -> &mut Self::Output {
        match working_mode {
            WorkingMode::Idle => &mut self.idle,
            WorkingMode::Harvest => &mut self.harvest,
            WorkingMode::SelfUse => &mut self.self_use,
            WorkingMode::Charge => &mut self.charge,
            WorkingMode::Discharge => &mut self.discharge,
        }
    }
}
