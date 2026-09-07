use crate::{DitherError, Effect, ErrorKind, Result, Selection};

/// One effect and selection in an ordered [`Pipeline`].
pub struct PipelineStep {
    effect: Box<dyn Effect>,
    selection: Selection,
}

impl PipelineStep {
    /// Returns the selection used by this step.
    pub const fn selection(&self) -> &Selection {
        &self.selection
    }

    /// Returns the effect used by this step.
    pub(crate) fn effect(&self) -> &dyn Effect {
        self.effect.as_ref()
    }
}

/// An editable ordered collection of effects and their selections.
///
/// A pipeline stores instructions only. Rendering always begins from the
/// immutable source image, and callers may retain previous pipelines or
/// removed steps when they need history.
#[derive(Default)]
pub struct Pipeline {
    steps: Vec<PipelineStep>,
}

impl Pipeline {
    /// Creates an empty pipeline.
    pub const fn new() -> Self {
        Self { steps: Vec::new() }
    }

    /// Returns the number of steps.
    pub const fn len(&self) -> usize {
        self.steps.len()
    }

    /// Reports whether the pipeline has no steps.
    pub const fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Appends an effect and selection to the pipeline.
    pub fn add(&mut self, effect: impl Effect + 'static, selection: Selection) {
        self.steps.push(PipelineStep {
            effect: Box::new(effect),
            selection,
        });
    }

    /// Inserts a previously removed step at `index`.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] when `index` is greater than
    /// the pipeline length.
    pub fn insert(&mut self, index: usize, step: PipelineStep) -> Result<()> {
        if index > self.steps.len() {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "pipeline step index is out of range",
            ));
        }

        self.steps.insert(index, step);
        Ok(())
    }

    /// Removes and returns the step at `index`, if it exists.
    pub fn remove(&mut self, index: usize) -> Option<PipelineStep> {
        (index < self.steps.len()).then(|| self.steps.remove(index))
    }

    /// Moves a step so its final position is `to`.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] when either index is outside
    /// the pipeline.
    pub fn move_step(&mut self, from: usize, to: usize) -> Result<()> {
        if from >= self.steps.len() || to >= self.steps.len() {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "pipeline step index is out of range",
            ));
        }
        if from != to {
            let step = self.steps.remove(from);
            self.steps.insert(to, step);
        }

        Ok(())
    }

    /// Returns the steps in rendering order.
    pub fn steps(&self) -> &[PipelineStep] {
        &self.steps
    }
}

#[cfg(test)]
mod tests {
    use super::Pipeline;
    use crate::{Effect, ErrorKind, Mask, Result, Selection};

    struct Noop;

    impl Effect for Noop {
        fn apply(
            &self,
            _input: &[u8],
            _output: &mut [u8],
            _dimensions: (u32, u32),
            _mask: &Mask,
        ) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn adds_removes_inserts_and_moves_steps() {
        let mut pipeline = Pipeline::new();
        assert!(pipeline.is_empty());

        pipeline.add(Noop, Selection::All);
        pipeline.add(Noop, Selection::All);
        assert_eq!(pipeline.len(), 2);
        assert_eq!(pipeline.steps().len(), 2);

        let removed = pipeline.remove(0).unwrap();
        assert_eq!(pipeline.len(), 1);
        pipeline.insert(0, removed).unwrap();
        pipeline.move_step(0, 1).unwrap();
        assert_eq!(pipeline.len(), 2);
    }

    #[test]
    fn rejects_invalid_edit_indices() {
        let mut pipeline = Pipeline::new();
        pipeline.add(Noop, Selection::All);

        assert!(pipeline.remove(1).is_none());
        let removed = pipeline.remove(0).unwrap();
        assert_eq!(
            pipeline.insert(2, removed).unwrap_err().kind(),
            ErrorKind::InvalidParameter
        );

        pipeline.add(Noop, Selection::All);
        assert_eq!(
            pipeline.move_step(0, 1).unwrap_err().kind(),
            ErrorKind::InvalidParameter
        );
    }
}
