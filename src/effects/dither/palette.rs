use std::collections::HashMap;

use super::colour::{Colour, ColourSpace, colour_components, colour_distance, colour_luminance};
use crate::{DitherError, ErrorKind, Result, SourceImage};

/// The components considered while matching a colour to a palette.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum PaletteMatchMode {
    /// Compares every component in the selected [`ColourSpace`].
    #[default]
    Colour,
    /// Compares luminance only, retaining the palette's ordering for ties.
    Luminance,
}

/// The number of source colours retained by [`Palette::from_source`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum PaletteSize {
    /// Retains every distinct visible source colour.
    #[default]
    All,
    /// Reduces the source colours to at most this many representatives.
    Limited(usize),
}

/// A non-empty collection of RGB colours available to a dithering effect.
#[derive(Clone, Debug)]
pub struct Palette {
    colours: Box<[[u8; 3]]>,
    pub(super) components: Option<Box<[[f32; 3]]>>,
    pub(super) exact_colours: Option<Box<[u64]>>,
    pub(super) colour_space: ColourSpace,
    pub(super) matching_mode: PaletteMatchMode,
}

impl PartialEq for Palette {
    fn eq(&self, other: &Self) -> bool {
        self.colours == other.colours
            && self.colour_space == other.colour_space
            && self.matching_mode == other.matching_mode
    }
}

impl Eq for Palette {}

impl Palette {
    /// Creates a palette from at least one RGB colour.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] when `colours` is empty.
    pub fn new<C>(colours: impl IntoIterator<Item = C>) -> Result<Self>
    where
        C: Into<Colour>,
    {
        let colours = colours
            .into_iter()
            .map(|colour| colour.into().rgb())
            .collect::<Vec<_>>()
            .into_boxed_slice();

        if colours.is_empty() {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "a palette requires at least one colour",
            ));
        }

        Ok(Self::from_colours(colours))
    }

    /// Derives a palette from the visible colours in `source`.
    ///
    /// [`PaletteSize::All`] retains distinct colours in first-seen order.
    /// [`PaletteSize::Limited`] uses deterministic frequency-weighted median
    /// cut and selects representatives that occur in the source. Pixels with
    /// zero alpha are ignored.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidParameter`] when the requested limit is zero
    /// or the source has no visible pixels.
    pub fn from_source(source: &SourceImage, size: PaletteSize) -> Result<Self> {
        if size == PaletteSize::Limited(0) {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "derived palette size must be greater than zero",
            ));
        }
        if size == PaletteSize::All {
            return Self::from_all_source_colours(source);
        }

        let mut indices = HashMap::<[u8; 3], usize>::new();
        let mut colours = Vec::<SourceColour>::new();
        for (order, pixel) in source.rgba8_bytes().as_chunks::<4>().0.iter().enumerate() {
            if pixel[3] == 0 {
                continue;
            }
            let colour = [pixel[0], pixel[1], pixel[2]];
            if let Some(&index) = indices.get(&colour) {
                colours[index].count += 1;
            } else {
                indices.insert(colour, colours.len());
                colours.push(SourceColour {
                    colour,
                    count: 1,
                    order,
                });
            }
        }

        if colours.is_empty() {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "cannot derive a palette from a source with no visible pixels",
            ));
        }
        let PaletteSize::Limited(limit) = size else {
            unreachable!("PaletteSize::All returns above")
        };
        let colours = if colours.len() > limit {
            reduce_colours(colours, limit)
        } else {
            colours.into_iter().map(|entry| entry.colour).collect()
        };
        Ok(Self::from_colours(colours))
    }

    fn from_all_source_colours(source: &SourceImage) -> Result<Self> {
        const RGB_COLOURS: usize = 1 << 24;
        const BITS_PER_WORD: usize = u64::BITS as usize;

        let mut seen = vec![0_u64; RGB_COLOURS / BITS_PER_WORD];
        let mut colours = Vec::new();
        for pixel in source.rgba8_bytes().as_chunks::<4>().0 {
            if pixel[3] == 0 {
                continue;
            }
            let colour = [pixel[0], pixel[1], pixel[2]];
            let index =
                usize::from(pixel[0]) << 16 | usize::from(pixel[1]) << 8 | usize::from(pixel[2]);
            let word = &mut seen[index / BITS_PER_WORD];
            let bit = 1_u64 << (index % BITS_PER_WORD);
            if *word & bit == 0 {
                *word |= bit;
                colours.push(colour);
            }
        }

        if colours.is_empty() {
            return Err(DitherError::new(
                ErrorKind::InvalidParameter,
                "cannot derive a palette from a source with no visible pixels",
            ));
        }
        Ok(Self {
            colours: colours.into_boxed_slice(),
            components: None,
            exact_colours: Some(seen.into_boxed_slice()),
            colour_space: ColourSpace::Rgb,
            matching_mode: PaletteMatchMode::Colour,
        })
    }

    /// Creates a palette containing black, white, and one RGB colour.
    ///
    /// Supplying black or white produces the same two entries as
    /// [`Palette::black_and_white`].
    pub fn monochrome(colour: impl Into<Colour>) -> Self {
        let colour = colour.into().rgb();
        if colour == [0, 0, 0] || colour == [255, 255, 255] {
            return Self::black_and_white();
        }

        Self::from_colours(Box::new([[0, 0, 0], colour, [255, 255, 255]]))
    }

    /// Creates a palette containing black and white.
    pub fn black_and_white() -> Self {
        Self::from_colours(Box::new([[0, 0, 0], [255, 255, 255]]))
    }

    /// Creates an eight-level greyscale palette.
    pub fn greyscale() -> Self {
        Self::from_colours(Box::new([
            [0, 0, 0],
            [36, 36, 36],
            [73, 73, 73],
            [109, 109, 109],
            [146, 146, 146],
            [182, 182, 182],
            [219, 219, 219],
            [255, 255, 255],
        ]))
    }

    /// Creates a four-colour green palette inspired by the original Game Boy.
    pub fn game_boy() -> Self {
        Self::from_colours(Box::new([
            [15, 56, 15],
            [48, 98, 48],
            [139, 172, 15],
            [155, 188, 15],
        ]))
    }

    /// Creates the standard 16-colour CGA palette.
    pub fn cga() -> Self {
        Self::from_colours(Box::new([
            [0, 0, 0],
            [0, 0, 170],
            [0, 170, 0],
            [0, 170, 170],
            [170, 0, 0],
            [170, 0, 170],
            [170, 85, 0],
            [170, 170, 170],
            [85, 85, 85],
            [85, 85, 255],
            [85, 255, 85],
            [85, 255, 255],
            [255, 85, 85],
            [255, 85, 255],
            [255, 255, 85],
            [255, 255, 255],
        ]))
    }

    /// Creates the standard 16-colour PICO-8 palette.
    pub fn pico_8() -> Self {
        Self::from_colours(Box::new([
            [0, 0, 0],
            [29, 43, 83],
            [126, 37, 83],
            [0, 135, 81],
            [171, 82, 54],
            [95, 87, 79],
            [194, 195, 199],
            [255, 241, 232],
            [255, 0, 77],
            [255, 163, 0],
            [255, 236, 39],
            [0, 228, 54],
            [41, 173, 255],
            [131, 118, 156],
            [255, 119, 168],
            [255, 204, 170],
        ]))
    }

    fn from_colours(colours: Box<[[u8; 3]]>) -> Self {
        Self {
            colours,
            components: None,
            exact_colours: None,
            colour_space: ColourSpace::Rgb,
            matching_mode: PaletteMatchMode::Colour,
        }
    }

    /// Selects the colour space used for palette matching.
    pub fn with_colour_space(mut self, colour_space: ColourSpace) -> Self {
        self.colour_space = colour_space;
        self.cache_components();
        self
    }

    /// Returns the colour space used for palette matching.
    pub const fn colour_space(&self) -> ColourSpace {
        self.colour_space
    }

    /// Selects full-colour or luminance-only palette matching.
    pub fn with_matching_mode(mut self, matching_mode: PaletteMatchMode) -> Self {
        self.matching_mode = matching_mode;
        if matching_mode != PaletteMatchMode::Colour {
            self.cache_components();
        }
        self
    }

    /// Returns the palette matching mode.
    pub const fn matching_mode(&self) -> PaletteMatchMode {
        self.matching_mode
    }

    /// Returns the palette's RGB colours in matching order.
    pub fn colours(&self) -> &[[u8; 3]] {
        &self.colours
    }

    /// Iterates over the palette as [`Colour`] values.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = Colour> + '_ {
        self.colours.iter().copied().map(Colour::from)
    }

    /// Returns the nearest palette entry as a [`Colour`].
    pub fn nearest(&self, colour: impl Into<Colour>) -> Colour {
        Colour::from(self.nearest_colour(colour.into().rgb()))
    }

    /// Returns the nearest palette colour to `colour`.
    ///
    /// Matching uses the configured colour space and mode. When two entries
    /// are equally close, the earlier palette entry wins.
    #[inline]
    pub fn nearest_colour(&self, colour: [u8; 3]) -> [u8; 3] {
        if self.colour_space == ColourSpace::Rgb
            && self.matching_mode == PaletteMatchMode::Colour
            && self.colours.as_ref() == [[0, 0, 0], [255, 255, 255]]
        {
            // The squared distances cross at an RGB sum of 382.5.
            let sum = u16::from(colour[0]) + u16::from(colour[1]) + u16::from(colour[2]);
            return [if sum >= 383 { 255 } else { 0 }; 3];
        }
        if self.colour_space == ColourSpace::Rgb && self.matching_mode == PaletteMatchMode::Colour {
            if self.contains_exact(colour) {
                return colour;
            }
            return *self
                .colours
                .iter()
                .min_by_key(|candidate| colour_distance(colour, **candidate))
                .expect("a palette is always non-empty");
        }
        self.nearest_transformed(colour_components(colour, self.colour_space))
    }

    pub(super) fn nearest_transformed(&self, colour: [f32; 3]) -> [u8; 3] {
        let components = self
            .components
            .as_ref()
            .expect("transformed matching always caches palette components");
        *self
            .colours
            .iter()
            .zip(components)
            .min_by(|(_, left), (_, right)| {
                let left = match_distance(colour, **left, self.colour_space, self.matching_mode);
                let right = match_distance(colour, **right, self.colour_space, self.matching_mode);
                left.total_cmp(&right)
            })
            .expect("a palette is always non-empty")
            .0
    }

    fn cache_components(&mut self) {
        self.components = Some(
            self.colours
                .iter()
                .map(|colour| colour_components(*colour, self.colour_space))
                .collect(),
        );
    }

    fn contains_exact(&self, colour: [u8; 3]) -> bool {
        const BITS_PER_WORD: usize = u64::BITS as usize;

        let Some(exact_colours) = &self.exact_colours else {
            return false;
        };
        let index =
            usize::from(colour[0]) << 16 | usize::from(colour[1]) << 8 | usize::from(colour[2]);
        exact_colours[index / BITS_PER_WORD] & (1_u64 << (index % BITS_PER_WORD)) != 0
    }
}

struct SourceColour {
    colour: [u8; 3],
    count: u64,
    order: usize,
}

/// Reduces source colours with frequency-weighted median cut.
fn reduce_colours(colours: Vec<SourceColour>, limit: usize) -> Box<[[u8; 3]]> {
    let mut buckets = vec![colours];
    while buckets.len() < limit {
        let Some((index, _)) = buckets
            .iter()
            .enumerate()
            .filter(|(_, bucket)| bucket.len() > 1)
            .map(|(index, bucket)| {
                let (range, _) = widest_channel(bucket);
                let population = bucket.iter().map(|entry| entry.count).sum::<u64>();
                (index, (range, population))
            })
            .max_by_key(|(_, score)| *score)
        else {
            break;
        };
        let mut bucket = buckets.remove(index);
        let (_, channel) = widest_channel(&bucket);
        bucket.sort_by_key(|entry| (entry.colour[channel], entry.colour, entry.order));
        let total = bucket.iter().map(|entry| entry.count).sum::<u64>();
        let mut population = 0;
        let mut best = (1, u64::MAX);
        for split in 1..bucket.len() {
            population += bucket[split - 1].count;
            let imbalance = population.abs_diff(total - population);
            if imbalance < best.1 {
                best = (split, imbalance);
            }
        }
        let split = best.0;
        let right = bucket.split_off(split);
        buckets.insert(index, bucket);
        buckets.push(right);
    }

    let mut representatives = buckets
        .iter()
        .map(|bucket| {
            let total = bucket
                .iter()
                .map(|entry| u128::from(entry.count))
                .sum::<u128>();
            let sums: [u128; 3] = std::array::from_fn(|channel| {
                bucket
                    .iter()
                    .map(|entry| u128::from(entry.colour[channel]) * u128::from(entry.count))
                    .sum::<u128>()
            });
            bucket
                .iter()
                .min_by_key(|entry| {
                    let distance = (0..3)
                        .map(|channel| {
                            let value = u128::from(entry.colour[channel]) * total;
                            value.abs_diff(sums[channel]).pow(2)
                        })
                        .sum::<u128>();
                    (distance, u64::MAX - entry.count, entry.order)
                })
                .map(|entry| (entry.order, entry.colour))
                .expect("median-cut buckets are non-empty")
        })
        .collect::<Vec<_>>();
    representatives.sort_by_key(|(order, _)| *order);
    representatives
        .into_iter()
        .map(|(_, colour)| colour)
        .collect()
}

fn widest_channel(colours: &[SourceColour]) -> (u8, usize) {
    (0..3)
        .map(|channel| {
            let (minimum, maximum) = colours
                .iter()
                .map(|entry| entry.colour[channel])
                .fold((u8::MAX, u8::MIN), |(minimum, maximum), value| {
                    (minimum.min(value), maximum.max(value))
                });
            (maximum - minimum, channel)
        })
        .max_by_key(|&(range, channel)| (range, usize::MAX - channel))
        .expect("colours have three channels")
}

fn match_distance(
    left: [f32; 3],
    right: [f32; 3],
    colour_space: ColourSpace,
    matching_mode: PaletteMatchMode,
) -> f32 {
    match matching_mode {
        PaletteMatchMode::Colour => left
            .into_iter()
            .zip(right)
            .map(|(left, right)| (left - right) * (left - right))
            .sum(),
        PaletteMatchMode::Luminance => {
            let difference =
                colour_luminance(left, colour_space) - colour_luminance(right, colour_space);
            difference * difference
        }
    }
}
