use super::script::tables::Tables;
use super::script::{Environment, FaultKind, Value};
use super::{Axis, State};
use std::sync::Arc;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AnimationState {
    pub horizontal: Option<Axis>,
    pub vertical: Option<Axis>,
    pub zoom: Option<f32>,
    pub rotation: [Option<f32>; 3],
    pub scale_delta: Option<f32>,
    pub color: [Option<f32>; 3],
    pub opacity: Option<f32>,
    pub spacing: Option<f32>,
}

fn field(tables: &mut Tables, table: &Value, name: &[u8]) -> Result<Value, FaultKind> {
    tables.field(table, &Value::Bytes(Arc::from(name)))
}

fn number(value: &Value) -> Result<f32, FaultKind> {
    match value {
        Value::Number(number) => Ok(*number),
        Value::Nil | Value::Boolean(_) | Value::Reference { .. } => Ok(0.0),
        Value::Bytes(_) | Value::Hash(_) => Err(FaultKind::BindingRequired),
    }
}

fn optional_number(
    tables: &mut Tables,
    table: &Value,
    name: &[u8],
) -> Result<Option<f32>, FaultKind> {
    let value = field(tables, table, name)?;
    if value == Value::Nil {
        Ok(None)
    } else {
        number(&value).map(Some)
    }
}

fn axis(
    tables: &mut Tables,
    table: &Value,
    start: &[u8],
    end: &[u8],
    anchor_start: &[u8],
    anchor_end: &[u8],
) -> Result<Option<Axis>, FaultKind> {
    let Some(start) = optional_number(tables, table, start)? else {
        return Ok(None);
    };
    let anchor_start = field(tables, table, anchor_start)?.truthy();
    let end = number(&field(tables, table, end)?)?;
    let anchor_end = field(tables, table, anchor_end)?.truthy();
    Ok(Some(Axis {
        start,
        end,
        anchor_start,
        anchor_end,
    }))
}

impl AnimationState {
    pub fn apply(&self, destination: &mut State) {
        if let Some(axis) = self.horizontal {
            destination.horizontal = axis;
            destination.changes.horizontal = true;
        }
        if let Some(axis) = self.vertical {
            destination.vertical = axis;
            destination.changes.vertical = true;
        }
        fn scalar(value: Option<f32>, destination: &mut f32, changed: &mut bool) {
            if let Some(value) = value {
                *destination = value;
                *changed = true;
            }
        }
        scalar(
            self.zoom,
            &mut destination.zoom,
            &mut destination.changes.zoom,
        );
        for ((value, field), changed) in self
            .rotation
            .iter()
            .zip(&mut destination.rotation)
            .zip(&mut destination.changes.rotation)
        {
            scalar(*value, field, changed);
        }
        scalar(
            self.scale_delta,
            &mut destination.scale_delta,
            &mut destination.changes.scale_delta,
        );
        for ((value, field), changed) in self
            .color
            .iter()
            .zip(&mut destination.color)
            .zip(&mut destination.changes.color)
        {
            scalar(*value, field, changed);
        }
        scalar(
            self.opacity,
            &mut destination.opacity,
            &mut destination.changes.opacity,
        );
        scalar(
            self.spacing,
            &mut destination.spacing,
            &mut destination.changes.spacing,
        );
    }

    pub fn parse(tables: &mut Tables, table: &Value) -> Result<Self, FaultKind> {
        if !matches!(table, Value::Reference { kind: 5, .. }) {
            return Err(FaultKind::Argument(3));
        }
        let horizontal = axis(
            tables,
            table,
            b"left",
            b"right",
            b"leftAnchor",
            b"rightAnchor",
        )?;
        let vertical = axis(
            tables,
            table,
            b"top",
            b"bottom",
            b"topAnchor",
            b"bottomAnchor",
        )?;
        let zoom = optional_number(tables, table, b"zoom")?;
        let rotation = [
            optional_number(tables, table, b"xRot")?,
            optional_number(tables, table, b"yRot")?,
            optional_number(tables, table, b"zRot")?,
        ];
        let scale_delta = optional_number(tables, table, b"scale")?.map(|scale| scale - 1.0);
        let color = [
            optional_number(tables, table, b"red")?,
            optional_number(tables, table, b"green")?,
            optional_number(tables, table, b"blue")?,
        ];
        let mut opacity = optional_number(tables, table, b"alpha")?;
        if let Some(multiplier) = optional_number(tables, table, b"alphaMultiplier")? {
            opacity = Some(multiplier);
        }
        let spacing = optional_number(tables, table, b"spacing")?;
        for name in [
            b"shaderVector0".as_slice(),
            b"shaderVector1",
            b"shaderVector2",
            b"shaderVector3",
            b"shaderVector4",
            b"shaderVector5",
            b"material",
            b"font",
            b"alignment",
            b"ui3DWindow",
        ] {
            if field(tables, table, name)? != Value::Nil {
                return Err(FaultKind::BindingRequired);
            }
        }
        Ok(Self {
            horizontal,
            vertical,
            zoom,
            rotation,
            scale_delta,
            color,
            opacity,
            spacing,
        })
    }
}
