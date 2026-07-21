//! Conversión de los enums de la CLI (clap) a los enums del dominio (xtal-model).
//! Se mantienen separados para que el modelo no dependa de clap.

use crate::cli::{
    FormatArg, KindArg, LineArg, PanelArg, PlotKindArg, RoleArg, ScaleArg, ShapeArg, SweepArg,
};
use xtal_data::{RandomShape, SweepScale};
use xtal_model::{DocFormat, LineDash, MeasurementKind, Panel, PlotKind, Role, Scale};
use xtal_sim::Sweep;

impl From<FormatArg> for DocFormat {
    fn from(f: FormatArg) -> Self {
        match f {
            FormatArg::Facultad => DocFormat::Facultad,
            FormatArg::Paper => DocFormat::Paper,
        }
    }
}

impl From<KindArg> for MeasurementKind {
    fn from(k: KindArg) -> Self {
        match k {
            KindArg::Theoretical => MeasurementKind::Theoretical,
            KindArg::Simulated => MeasurementKind::Simulated,
            KindArg::Measured => MeasurementKind::Measured,
            KindArg::Random => MeasurementKind::Random,
        }
    }
}

impl From<PlotKindArg> for PlotKind {
    fn from(k: PlotKindArg) -> Self {
        match k {
            PlotKindArg::Bode => PlotKind::Bode,
            PlotKindArg::Time => PlotKind::Time,
            PlotKindArg::Xy => PlotKind::Xy,
            PlotKindArg::Generic => PlotKind::Generic,
        }
    }
}

impl From<RoleArg> for Role {
    fn from(r: RoleArg) -> Self {
        match r {
            RoleArg::Input => Role::Input,
            RoleArg::Output => Role::Output,
            RoleArg::Third => Role::Third,
            RoleArg::None => Role::None,
        }
    }
}

impl From<ScaleArg> for Scale {
    fn from(s: ScaleArg) -> Self {
        match s {
            ScaleArg::Linear => Scale::Linear,
            ScaleArg::Log => Scale::Log,
        }
    }
}

impl From<ScaleArg> for SweepScale {
    fn from(s: ScaleArg) -> Self {
        match s {
            ScaleArg::Linear => SweepScale::Linear,
            ScaleArg::Log => SweepScale::Log,
        }
    }
}

impl From<LineArg> for LineDash {
    fn from(l: LineArg) -> Self {
        match l {
            LineArg::Solid => LineDash::Solid,
            LineArg::Dashed => LineDash::Dashed,
            LineArg::Dotted => LineDash::Dotted,
            LineArg::None => LineDash::None,
        }
    }
}

impl From<PanelArg> for Panel {
    fn from(p: PanelArg) -> Self {
        match p {
            PanelArg::Magnitude => Panel::Magnitude,
            PanelArg::Phase => Panel::Phase,
        }
    }
}

impl From<ShapeArg> for RandomShape {
    fn from(s: ShapeArg) -> Self {
        match s {
            ShapeArg::Noise => RandomShape::Noise,
            ShapeArg::Ramp => RandomShape::Ramp,
            ShapeArg::Sine => RandomShape::Sine,
        }
    }
}

impl From<SweepArg> for Sweep {
    fn from(s: SweepArg) -> Self {
        match s {
            SweepArg::Dec => Sweep::Dec,
            SweepArg::Oct => Sweep::Oct,
            SweepArg::Lin => Sweep::Lin,
        }
    }
}
