use crate::{DitherError, ErrorKind, Result};

const SAMPLES_PER_AXIS: u32 = 4;
const SAMPLE_COUNT: u32 = SAMPLES_PER_AXIS * SAMPLES_PER_AXIS;

/// A point in image coordinates, with the origin at the top-left corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    /// The horizontal coordinate.
    pub x: f32,
    /// The vertical coordinate.
    pub y: f32,
}

impl Point {
    /// Creates a point at `(x, y)`.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A polygon selection boundary.
///
/// Polygons use the even-odd fill rule, so self-intersecting paths are valid.
/// Points on a polygon edge are considered inside.
#[derive(Clone, Debug, PartialEq)]
pub struct Polygon {
    vertices: Box<[Point]>,
}

impl Polygon {
    /// Creates a polygon from at least three finite, non-collinear vertices.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidPolygon`] when there are fewer than three
    /// vertices, a coordinate is non-finite, or all vertices are collinear.
    pub fn new(vertices: impl Into<Box<[Point]>>) -> Result<Self> {
        let vertices = vertices.into();

        if vertices.len() < 3 {
            return Err(DitherError::new(
                ErrorKind::InvalidPolygon,
                "a polygon requires at least three vertices",
            ));
        }

        if vertices
            .iter()
            .any(|point| !point.x.is_finite() || !point.y.is_finite())
        {
            return Err(DitherError::new(
                ErrorKind::InvalidPolygon,
                "polygon coordinates must be finite",
            ));
        }

        if vertices_are_collinear(&vertices) {
            return Err(DitherError::new(
                ErrorKind::InvalidPolygon,
                "polygon vertices must enclose an area",
            ));
        }

        Ok(Self { vertices })
    }

    /// Returns the polygon's vertices.
    pub fn vertices(&self) -> &[Point] {
        &self.vertices
    }
}

/// An area of an image to which an effect can be applied.
#[derive(Clone, Debug, PartialEq)]
pub enum Selection {
    /// Selects every pixel.
    All,
    /// Selects the area covered by a polygon.
    Polygon(Polygon),
}

impl Selection {
    /// Rasterises this selection into a mask with the requested dimensions.
    ///
    /// Polygon coverage uses a fixed 4 by 4 subpixel grid. Each mask value is
    /// `0` outside the selection, `255` inside it, or an intermediate coverage
    /// value along an anti-aliased edge.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::DimensionMismatch`] when the dimensions cannot be
    /// represented by a mask on the current platform.
    pub fn rasterise(&self, width: u32, height: u32) -> Result<Mask> {
        let length = mask_length(width, height)?;

        match self {
            Self::All => Ok(Mask {
                width,
                height,
                coverage: vec![u8::MAX; length].into_boxed_slice(),
            }),
            Self::Polygon(polygon) => {
                let mut coverage = vec![0; length];
                rasterise_polygon(polygon, width, height, &mut coverage);

                Ok(Mask {
                    width,
                    height,
                    coverage: coverage.into_boxed_slice(),
                })
            }
        }
    }
}

/// Per-pixel selection coverage for an image.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mask {
    width: u32,
    height: u32,
    coverage: Box<[u8]>,
}

impl Mask {
    /// Creates a mask from row-major coverage values.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::DimensionMismatch`] when the number of values does
    /// not equal `width * height` or the dimensions exceed platform limits.
    pub fn new(width: u32, height: u32, coverage: Vec<u8>) -> Result<Self> {
        if coverage.len() != mask_length(width, height)? {
            return Err(DitherError::new(
                ErrorKind::DimensionMismatch,
                "mask dimensions do not match its coverage length",
            ));
        }

        Ok(Self {
            width,
            height,
            coverage: coverage.into_boxed_slice(),
        })
    }

    /// Returns the mask width in pixels.
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the mask height in pixels.
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns the mask dimensions as `(width, height)`.
    pub const fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Returns the row-major coverage values.
    pub fn coverage_bytes(&self) -> &[u8] {
        &self.coverage
    }

    /// Returns the coverage at `(x, y)`, or `None` when it is out of bounds.
    pub fn coverage(&self, x: u32, y: u32) -> Option<u8> {
        if x >= self.width || y >= self.height {
            return None;
        }

        Some(self.coverage[y as usize * self.width as usize + x as usize])
    }
}

/// Calculates the number of coverage values required for a mask.
fn mask_length(width: u32, height: u32) -> Result<usize> {
    let length = u64::from(width) * u64::from(height);

    if length > isize::MAX as u64 {
        return Err(DitherError::new(
            ErrorKind::DimensionMismatch,
            "mask dimensions exceed platform limits",
        ));
    }

    Ok(length as usize)
}

/// Reports whether every vertex lies on a single straight line.
fn vertices_are_collinear(vertices: &[Point]) -> bool {
    let first = vertices[0];
    let Some(second) = vertices.iter().copied().find(|point| *point != first) else {
        return true;
    };

    vertices.iter().all(|point| {
        let cross = f64::from(second.x - first.x) * f64::from(point.y - first.y)
            - f64::from(second.y - first.y) * f64::from(point.x - first.x);
        cross == 0.0
    })
}

/// Rasterises a polygon into row-major coverage values using supersampling.
fn rasterise_polygon(polygon: &Polygon, width: u32, height: u32, coverage: &mut [u8]) {
    for y in 0..height {
        for x in 0..width {
            let mut covered_samples = 0;

            for sample_y in 0..SAMPLES_PER_AXIS {
                for sample_x in 0..SAMPLES_PER_AXIS {
                    let point = Point {
                        x: x as f32 + (sample_x as f32 + 0.5) / SAMPLES_PER_AXIS as f32,
                        y: y as f32 + (sample_y as f32 + 0.5) / SAMPLES_PER_AXIS as f32,
                    };

                    if polygon_contains(polygon, point) {
                        covered_samples += 1;
                    }
                }
            }

            coverage[y as usize * width as usize + x as usize] =
                ((covered_samples * u32::from(u8::MAX) + SAMPLE_COUNT / 2) / SAMPLE_COUNT) as u8;
        }
    }
}

/// Tests a point against a polygon using the even-odd fill rule.
fn polygon_contains(polygon: &Polygon, point: Point) -> bool {
    let mut inside = false;
    let mut previous = polygon.vertices[polygon.vertices.len() - 1];

    for &current in &polygon.vertices {
        if point_is_on_segment(point, previous, current) {
            return true;
        }

        if (current.y > point.y) != (previous.y > point.y)
            && point.x
                < (previous.x - current.x) * (point.y - current.y) / (previous.y - current.y)
                    + current.x
        {
            inside = !inside;
        }

        previous = current;
    }

    inside
}

/// Reports whether a point lies on the closed line segment from start to end.
fn point_is_on_segment(point: Point, start: Point, end: Point) -> bool {
    let cross = (point.y - start.y) * (end.x - start.x) - (point.x - start.x) * (end.y - start.y);

    cross.abs() <= f32::EPSILON
        && point.x >= start.x.min(end.x)
        && point.x <= start.x.max(end.x)
        && point.y >= start.y.min(end.y)
        && point.y <= start.y.max(end.y)
}

#[cfg(test)]
mod tests {
    use super::{Mask, Point, Polygon, Selection};
    use crate::ErrorKind;

    #[test]
    fn rasterises_the_whole_image() {
        let mask = Selection::All.rasterise(3, 2).unwrap();

        assert_eq!(mask.width(), 3);
        assert_eq!(mask.height(), 2);
        assert_eq!(mask.dimensions(), (3, 2));
        assert_eq!(mask.coverage_bytes(), &[255; 6]);
        assert_eq!(mask.coverage(2, 1), Some(255));
        assert_eq!(mask.coverage(3, 1), None);
    }

    #[test]
    fn rasterises_polygon_interiors_exteriors_and_edges() {
        let polygon = Polygon::new([
            Point::new(0.0, 0.0),
            Point::new(4.0, 0.0),
            Point::new(0.0, 4.0),
        ])
        .unwrap();
        let mask = Selection::Polygon(polygon).rasterise(4, 4).unwrap();

        assert_eq!(mask.dimensions(), (4, 4));
        assert_eq!(mask.coverage(0, 0), Some(255));
        assert!(
            mask.coverage(3, 0)
                .is_some_and(|coverage| coverage > 0 && coverage < 255)
        );
        assert_eq!(mask.coverage(3, 3), Some(0));
    }

    #[test]
    fn rejects_malformed_polygons() {
        let too_short = Polygon::new([Point::new(0.0, 0.0), Point::new(1.0, 1.0)])
            .expect_err("two vertices should be rejected");
        assert_eq!(too_short.kind(), ErrorKind::InvalidPolygon);

        let non_finite = Polygon::new([
            Point::new(0.0, 0.0),
            Point::new(1.0, f32::NAN),
            Point::new(0.0, 1.0),
        ])
        .expect_err("non-finite coordinates should be rejected");
        assert_eq!(non_finite.kind(), ErrorKind::InvalidPolygon);

        let collinear = Polygon::new([
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(2.0, 2.0),
        ])
        .expect_err("collinear vertices should be rejected");
        assert_eq!(collinear.kind(), ErrorKind::InvalidPolygon);
    }

    #[test]
    fn rejects_mask_dimension_mismatches() {
        let error = Mask::new(2, 2, vec![0; 3]).expect_err("three values cannot fill a 2x2 mask");
        assert_eq!(error.kind(), ErrorKind::DimensionMismatch);

        let oversized = Mask::new(u32::MAX, u32::MAX, Vec::new())
            .expect_err("oversized dimensions should be rejected");
        assert_eq!(oversized.kind(), ErrorKind::DimensionMismatch);
    }
}
