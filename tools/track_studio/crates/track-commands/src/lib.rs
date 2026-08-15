//! Track editing commands with undo/redo history and preview collapse.
//!
//! Each edit is a reversible [`Command`]. A [`CommandHistory`] owns an in-memory
//! undo/redo stack. Continuous mouse interaction can be grouped with
//! [`CommandHistory::begin_batch`] / [`CommandHistory::end_batch`] so that many
//! incremental preview steps collapse into a single undo step.
//!
//! This layer is intentionally separate from Git and from SQLite persistence.
//! Git is an explicit user action; SQLite save is a separate persistence step.

use std::cell::{Cell, RefCell};
use track_domain::{Calibration, ControlPoint, Point2, ReferenceLayer, TrackDocument};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    ObjectNotFound(String),
    InvalidCalibration,
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ObjectNotFound(id) => write!(formatter, "object not found: {id}"),
            Self::InvalidCalibration => write!(formatter, "invalid reference calibration input"),
        }
    }
}

impl std::error::Error for CommandError {}

pub trait Command {
    fn label(&self) -> &str;
    fn apply(&self, document: &mut TrackDocument) -> Result<(), CommandError>;
    fn undo(&self, document: &mut TrackDocument) -> Result<(), CommandError>;
}

/// Move a centerline control point to a new position.
pub struct MoveControlPoint {
    point_id: String,
    to: Point2,
    from: RefCell<Option<Point2>>,
}

impl MoveControlPoint {
    pub fn new(point_id: impl Into<String>, to: Point2) -> Self {
        Self {
            point_id: point_id.into(),
            to,
            from: RefCell::new(None),
        }
    }
}

impl Command for MoveControlPoint {
    fn label(&self) -> &str {
        "move_control_point"
    }

    fn apply(&self, document: &mut TrackDocument) -> Result<(), CommandError> {
        let point = find_control_point(document, &self.point_id)?;
        if self.from.borrow().is_none() {
            *self.from.borrow_mut() = Some(Point2 {
                x: point.x,
                z: point.z,
            });
        }
        point.x = self.to.x;
        point.z = self.to.z;
        Ok(())
    }

    fn undo(&self, document: &mut TrackDocument) -> Result<(), CommandError> {
        let point = find_control_point(document, &self.point_id)?;
        if let Some(from) = self.from.borrow().as_ref() {
            point.x = from.x;
            point.z = from.z;
        }
        Ok(())
    }
}

/// Move an asset instance to a new position.
pub struct MoveAsset {
    asset_id: String,
    to: Point2,
    from: RefCell<Option<Point2>>,
}

impl MoveAsset {
    pub fn new(asset_id: impl Into<String>, to: Point2) -> Self {
        Self {
            asset_id: asset_id.into(),
            to,
            from: RefCell::new(None),
        }
    }
}

impl Command for MoveAsset {
    fn label(&self) -> &str {
        "move_asset"
    }

    fn apply(&self, document: &mut TrackDocument) -> Result<(), CommandError> {
        let asset = find_asset(document, &self.asset_id)?;
        if self.from.borrow().is_none() {
            *self.from.borrow_mut() = Some(Point2 {
                x: asset.x,
                z: asset.z,
            });
        }
        asset.x = self.to.x;
        asset.z = self.to.z;
        Ok(())
    }

    fn undo(&self, document: &mut TrackDocument) -> Result<(), CommandError> {
        let asset = find_asset(document, &self.asset_id)?;
        if let Some(from) = self.from.borrow().as_ref() {
            asset.x = from.x;
            asset.z = from.z;
        }
        Ok(())
    }
}

/// Insert a control point.
pub struct AddControlPoint {
    point: ControlPoint,
    added: Cell<bool>,
}

impl AddControlPoint {
    pub fn new(point: ControlPoint) -> Self {
        Self {
            point,
            added: Cell::new(false),
        }
    }
}

impl Command for AddControlPoint {
    fn label(&self) -> &str {
        "add_control_point"
    }

    fn apply(&self, document: &mut TrackDocument) -> Result<(), CommandError> {
        if !self.added.get() {
            document.centerline.push(self.point.clone());
            self.added.set(true);
        }
        Ok(())
    }

    fn undo(&self, document: &mut TrackDocument) -> Result<(), CommandError> {
        document
            .centerline
            .retain(|point| point.id != self.point.id);
        Ok(())
    }
}

/// Delete a control point, remembering its position for undo.
pub struct DeleteControlPoint {
    point_id: String,
    removed: RefCell<Option<(usize, ControlPoint)>>,
}

impl DeleteControlPoint {
    pub fn new(point_id: impl Into<String>) -> Self {
        Self {
            point_id: point_id.into(),
            removed: RefCell::new(None),
        }
    }
}

impl Command for DeleteControlPoint {
    fn label(&self) -> &str {
        "delete_control_point"
    }

    fn apply(&self, document: &mut TrackDocument) -> Result<(), CommandError> {
        if let Some((index, _)) = self.removed.borrow().as_ref() {
            if *index < document.centerline.len() {
                document.centerline.remove(*index);
                return Ok(());
            }
        }
        let index = document
            .centerline
            .iter()
            .position(|point| point.id == self.point_id)
            .ok_or_else(|| CommandError::ObjectNotFound(self.point_id.clone()))?;
        let point = document.centerline.remove(index);
        *self.removed.borrow_mut() = Some((index, point));
        Ok(())
    }

    fn undo(&self, document: &mut TrackDocument) -> Result<(), CommandError> {
        if let Some((index, point)) = self.removed.borrow().as_ref() {
            let index = (*index).min(document.centerline.len());
            document.centerline.insert(index, point.clone());
        }
        Ok(())
    }
}

fn find_control_point<'a>(
    document: &'a mut TrackDocument,
    point_id: &str,
) -> Result<&'a mut ControlPoint, CommandError> {
    document
        .centerline
        .iter_mut()
        .find(|point| point.id == point_id)
        .ok_or_else(|| CommandError::ObjectNotFound(point_id.to_owned()))
}

fn find_asset<'a>(
    document: &'a mut TrackDocument,
    asset_id: &str,
) -> Result<&'a mut track_domain::AssetInstance, CommandError> {
    document
        .asset_instances
        .iter_mut()
        .find(|asset| asset.id == asset_id)
        .ok_or_else(|| CommandError::ObjectNotFound(asset_id.to_owned()))
}

/// Calibrate a reference image from two image points and the real distance.
/// The computed pixels-per-meter is applied to the layer; undo restores the
/// previous calibration.
pub struct SetReferenceCalibration {
    layer_id: String,
    image_distance_px: f64,
    real_distance_m: f64,
    previous: RefCell<Option<Option<Calibration>>>,
}

impl SetReferenceCalibration {
    pub fn new(layer_id: impl Into<String>, image_distance_px: f64, real_distance_m: f64) -> Self {
        Self {
            layer_id: layer_id.into(),
            image_distance_px,
            real_distance_m,
            previous: RefCell::new(None),
        }
    }
}

impl Command for SetReferenceCalibration {
    fn label(&self) -> &str {
        "set_reference_calibration"
    }

    fn apply(&self, document: &mut TrackDocument) -> Result<(), CommandError> {
        let layer = find_reference_layer(document, &self.layer_id)?;
        let ppm = track_geometry::pixels_per_meter(self.image_distance_px, self.real_distance_m)
            .ok_or(CommandError::InvalidCalibration)?;
        if self.previous.borrow().is_none() {
            *self.previous.borrow_mut() = Some(layer.calibration.clone());
        }
        layer.calibration = Some(Calibration {
            pixels_per_meter: ppm,
        });
        Ok(())
    }

    fn undo(&self, document: &mut TrackDocument) -> Result<(), CommandError> {
        let layer = find_reference_layer(document, &self.layer_id)?;
        if let Some(previous) = self.previous.borrow().as_ref() {
            layer.calibration = previous.clone();
        }
        Ok(())
    }
}

fn find_reference_layer<'a>(
    document: &'a mut TrackDocument,
    layer_id: &str,
) -> Result<&'a mut ReferenceLayer, CommandError> {
    document
        .reference_layers
        .iter_mut()
        .find(|layer| layer.id == layer_id)
        .ok_or_else(|| CommandError::ObjectNotFound(layer_id.to_owned()))
}

struct Batch {
    commands: Vec<Box<dyn Command + Send>>,
}

/// Composite command that applies its children in order and undoes them in
/// reverse. Used to collapse a preview sequence into one undo step.
pub struct CompositeCommand {
    label: String,
    commands: Vec<Box<dyn Command + Send>>,
}

impl CompositeCommand {
    fn new(label: impl Into<String>, commands: Vec<Box<dyn Command + Send>>) -> Self {
        Self {
            label: label.into(),
            commands,
        }
    }
}

impl Command for CompositeCommand {
    fn label(&self) -> &str {
        &self.label
    }

    fn apply(&self, document: &mut TrackDocument) -> Result<(), CommandError> {
        for command in &self.commands {
            command.apply(document)?;
        }
        Ok(())
    }

    fn undo(&self, document: &mut TrackDocument) -> Result<(), CommandError> {
        for command in self.commands.iter().rev() {
            command.undo(document)?;
        }
        Ok(())
    }
}

pub struct CommandHistory {
    undo: Vec<Box<dyn Command + Send>>,
    redo: Vec<Box<dyn Command + Send>>,
    batch: Option<Batch>,
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandHistory {
    pub fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            batch: None,
        }
    }

    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_depth(&self) -> usize {
        self.redo.len()
    }

    pub fn is_in_batch(&self) -> bool {
        self.batch.is_some()
    }

    /// Start a batch of preview steps. Call [`Self::end_batch`] to commit them
    /// as one undo step, or [`Self::cancel_batch`] to discard them.
    pub fn begin_batch(&mut self) {
        if self.batch.is_none() {
            self.batch = Some(Batch {
                commands: Vec::new(),
            });
        }
    }

    pub fn execute(
        &mut self,
        command: Box<dyn Command + Send>,
        document: &mut TrackDocument,
    ) -> Result<(), CommandError> {
        command.apply(document)?;
        match &mut self.batch {
            Some(batch) => batch.commands.push(command),
            None => {
                self.undo.push(command);
                self.redo.clear();
            }
        }
        Ok(())
    }

    /// Commit the current batch as a single undo step.
    pub fn end_batch(&mut self) {
        if let Some(batch) = self.batch.take() {
            if !batch.commands.is_empty() {
                let label = format!("{}_batch", batch.commands[0].label());
                if batch.commands.len() == 1 {
                    self.undo.push(batch.commands.into_iter().next().unwrap());
                } else {
                    self.undo
                        .push(Box::new(CompositeCommand::new(label, batch.commands)));
                }
                self.redo.clear();
            }
        }
    }

    /// Discard the current batch and restore the document to its pre-batch state.
    pub fn cancel_batch(&mut self, document: &mut TrackDocument) {
        if let Some(batch) = self.batch.take() {
            for command in batch.commands.iter().rev() {
                let _ = command.undo(document);
            }
        }
    }

    pub fn undo(&mut self, document: &mut TrackDocument) -> Result<(), CommandError> {
        debug_assert!(self.batch.is_none());
        if let Some(command) = self.undo.pop() {
            command.undo(document)?;
            self.redo.push(command);
        }
        Ok(())
    }

    pub fn redo(&mut self, document: &mut TrackDocument) -> Result<(), CommandError> {
        debug_assert!(self.batch.is_none());
        if let Some(command) = self.redo.pop() {
            command.apply(document)?;
            self.undo.push(command);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document() -> TrackDocument {
        let mut document = TrackDocument::new("test");
        document.centerline = vec![
            ControlPoint {
                id: "a".into(),
                x: 0.0,
                z: 0.0,
                handle_in: None,
                handle_out: None,
            },
            ControlPoint {
                id: "b".into(),
                x: 10.0,
                z: 0.0,
                handle_in: None,
                handle_out: None,
            },
            ControlPoint {
                id: "c".into(),
                x: 10.0,
                z: 10.0,
                handle_in: None,
                handle_out: None,
            },
        ];
        document
    }

    fn position(document: &TrackDocument, id: &str) -> Point2 {
        let point = document.centerline.iter().find(|p| p.id == id).unwrap();
        Point2 {
            x: point.x,
            z: point.z,
        }
    }

    #[test]
    fn move_control_point_undo_redo_roundtrip() {
        let mut document = document();
        let mut history = CommandHistory::new();
        let target = Point2 { x: 99.0, z: 55.0 };

        history
            .execute(
                Box::new(MoveControlPoint::new("b", target.clone())),
                &mut document,
            )
            .unwrap();
        assert_eq!(position(&document, "b"), target);
        assert_eq!(history.undo_depth(), 1);

        history.undo(&mut document).unwrap();
        assert_eq!(position(&document, "b"), Point2 { x: 10.0, z: 0.0 });
        assert_eq!(history.redo_depth(), 1);

        history.redo(&mut document).unwrap();
        assert_eq!(position(&document, "b"), target);
        assert_eq!(history.undo_depth(), 1);
    }

    #[test]
    fn delete_control_point_undo_restores_position() {
        let mut document = document();
        let mut history = CommandHistory::new();

        history
            .execute(Box::new(DeleteControlPoint::new("b")), &mut document)
            .unwrap();
        assert_eq!(document.centerline.len(), 2);
        assert!(document.centerline.iter().all(|p| p.id != "b"));

        history.undo(&mut document).unwrap();
        assert_eq!(document.centerline.len(), 3);
        assert_eq!(position(&document, "b"), Point2 { x: 10.0, z: 0.0 });
    }

    #[test]
    fn batch_collapses_three_moves_into_one_undo_step() {
        let mut document = document();
        let mut history = CommandHistory::new();

        history.begin_batch();
        history
            .execute(
                Box::new(MoveControlPoint::new("a", Point2 { x: 1.0, z: 1.0 })),
                &mut document,
            )
            .unwrap();
        history
            .execute(
                Box::new(MoveControlPoint::new("a", Point2 { x: 2.0, z: 2.0 })),
                &mut document,
            )
            .unwrap();
        history
            .execute(
                Box::new(MoveControlPoint::new("a", Point2 { x: 3.0, z: 3.0 })),
                &mut document,
            )
            .unwrap();
        history.end_batch();

        assert_eq!(position(&document, "a"), Point2 { x: 3.0, z: 3.0 });
        assert_eq!(history.undo_depth(), 1);

        history.undo(&mut document).unwrap();
        assert_eq!(position(&document, "a"), Point2 { x: 0.0, z: 0.0 });
        assert_eq!(history.undo_depth(), 0);
        assert_eq!(history.redo_depth(), 1);
    }

    #[test]
    fn cancel_batch_restores_original_state() {
        let mut document = document();
        let mut history = CommandHistory::new();

        history.begin_batch();
        history
            .execute(
                Box::new(MoveControlPoint::new("a", Point2 { x: 1.0, z: 1.0 })),
                &mut document,
            )
            .unwrap();
        history
            .execute(
                Box::new(MoveControlPoint::new("a", Point2 { x: 5.0, z: 5.0 })),
                &mut document,
            )
            .unwrap();
        history.cancel_batch(&mut document);

        assert_eq!(position(&document, "a"), Point2 { x: 0.0, z: 0.0 });
        assert_eq!(history.undo_depth(), 0);
        assert_eq!(history.redo_depth(), 0);
    }

    #[test]
    fn new_edit_clears_redo_stack() {
        let mut document = document();
        let mut history = CommandHistory::new();
        history
            .execute(
                Box::new(MoveControlPoint::new("b", Point2 { x: 1.0, z: 1.0 })),
                &mut document,
            )
            .unwrap();
        history.undo(&mut document).unwrap();
        assert_eq!(history.redo_depth(), 1);

        history
            .execute(
                Box::new(MoveControlPoint::new("c", Point2 { x: 2.0, z: 2.0 })),
                &mut document,
            )
            .unwrap();
        assert_eq!(history.redo_depth(), 0);
    }

    #[test]
    fn reference_calibration_command_applies_and_undoes() {
        let mut document = document();
        document.reference_layers.push(ReferenceLayer {
            id: "ref1".into(),
            image_path: "map.png".into(),
            position: Point2 { x: 0.0, z: 0.0 },
            rotation_rad: 0.0,
            scale: 1.0,
            opacity: 1.0,
            locked: false,
            visible: true,
            calibration: None,
        });
        let mut history = CommandHistory::new();

        history
            .execute(
                Box::new(SetReferenceCalibration::new("ref1", 100.0, 10.0)),
                &mut document,
            )
            .unwrap();
        assert_eq!(
            document.reference_layers[0]
                .calibration
                .as_ref()
                .unwrap()
                .pixels_per_meter,
            10.0
        );

        history.undo(&mut document).unwrap();
        assert!(document.reference_layers[0].calibration.is_none());

        history.redo(&mut document).unwrap();
        assert_eq!(
            document.reference_layers[0]
                .calibration
                .as_ref()
                .unwrap()
                .pixels_per_meter,
            10.0
        );
    }
}
