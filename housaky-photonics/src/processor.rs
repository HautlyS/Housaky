//! Image processing for photon data

use anyhow::Result;
use image::{GrayImage, ImageBuffer, Luma};
use ndarray::{Array2, Array3};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::detector::PhotonEvent;

/// Image processing pipeline
pub struct ImageProcessor {
    config: ProcessorConfig,
}

/// Configuration for image processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessorConfig {
    /// Apply denoising
    pub denoise: bool,
    /// Gaussian blur sigma
    pub blur_sigma: f32,
    /// Contrast enhancement
    pub enhance_contrast: bool,
    /// Edge detection
    pub detect_edges: bool,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            denoise: true,
            blur_sigma: 1.0,
            enhance_contrast: false,
            detect_edges: false,
        }
    }
}

impl ImageProcessor {
    /// Create a new image processor
    pub fn new(config: ProcessorConfig) -> Self {
        Self { config }
    }

    /// Process a frame from raw events
    pub fn process_events(&self, events: &[PhotonEvent], width: u32, height: u32) -> GrayImage {
        // Accumulate events into image
        let mut image = ImageBuffer::new(width, height);

        for event in events {
            if event.x < width && event.y < height {
                let pixel = image.get_pixel_mut(event.x, event.y);
                let current = pixel[0];
                let new_val = current.saturating_add(event.intensity);
                *pixel = Luma([new_val]);
            }
        }

        // Apply processing pipeline
        if self.config.denoise {
            image = self.apply_denoising(image);
        }

        image
    }

    /// Apply simple denoising (median filter)
    fn apply_denoising(&self, image: GrayImage) -> GrayImage {
        let (width, height) = image.dimensions();
        let mut output = ImageBuffer::new(width, height);

        for y in 1..height - 1 {
            for x in 1..width - 1 {
                let mut neighbors = Vec::with_capacity(9);

                for dy in -1..=1i32 {
                    for dx in -1..=1i32 {
                        let nx = (x as i32 + dx) as u32;
                        let ny = (y as i32 + dy) as u32;
                        neighbors.push(image.get_pixel(nx, ny)[0]);
                    }
                }

                neighbors.sort_unstable();
                let median = neighbors[4]; // Middle element
                output.put_pixel(x, y, Luma([median]));
            }
        }

        output
    }

    /// Convert to quantum-inspired representation
    pub fn to_quantum_representation(&self, image: &GrayImage) -> Array2<f64> {
        let (width, height) = image.dimensions();
        let mut array = Array2::zeros((height as usize, width as usize));

        for (y, row) in array.outer_iter_mut().enumerate() {
            for (x, val) in row.iter_mut().enumerate() {
                let pixel = image.get_pixel(x as u32, y as u32)[0];
                *val = pixel as f64 / 255.0;
            }
        }

        array
    }

    /// Detect regions of interest
    pub fn detect_regions(&self, image: &GrayImage, threshold: u8) -> Vec<Region> {
        let (width, height) = image.dimensions();
        let mut regions = Vec::new();
        let mut visited = vec![vec![false; width as usize]; height as usize];

        for y in 0..height {
            for x in 0..width {
                if visited[y as usize][x as usize] {
                    continue;
                }

                let pixel = image.get_pixel(x, y)[0];
                if pixel >= threshold {
                    // Flood fill to find connected region
                    let region = self.flood_fill(image, x, y, threshold, &mut visited);
                    if region.pixels.len() >= 10 {
                        // Minimum size
                        regions.push(region);
                    }
                }
            }
        }

        regions
    }

    /// Flood fill algorithm
    fn flood_fill(
        &self,
        image: &GrayImage,
        start_x: u32,
        start_y: u32,
        threshold: u8,
        visited: &mut Vec<Vec<bool>>,
    ) -> Region {
        use std::collections::VecDeque;

        let (width, height) = image.dimensions();
        let mut pixels = Vec::new();
        let mut queue = VecDeque::new();

        queue.push_back((start_x, start_y));
        visited[start_y as usize][start_x as usize] = true;

        while let Some((x, y)) = queue.pop_front() {
            pixels.push((x, y));

            for (dx, dy) in &[(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                let nx = (x as i32 + dx) as u32;
                let ny = (y as i32 + dy) as u32;

                if nx < width && ny < height && !visited[ny as usize][nx as usize] {
                    if image.get_pixel(nx, ny)[0] >= threshold {
                        visited[ny as usize][nx as usize] = true;
                        queue.push_back((nx, ny));
                    }
                }
            }
        }

        Region::from_pixels(pixels)
    }
}

/// Detected region in an image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    /// Pixels in the region
    pub pixels: Vec<(u32, u32)>,
    /// Bounding box (min_x, min_y, max_x, max_y)
    pub bounds: (u32, u32, u32, u32),
    /// Centroid (center_x, center_y)
    pub centroid: (f64, f64),
    /// Average intensity
    pub avg_intensity: f64,
}

impl Region {
    /// Create region from pixels
    fn from_pixels(pixels: Vec<(u32, u32)>) -> Self {
        if pixels.is_empty() {
            return Self {
                pixels,
                bounds: (0, 0, 0, 0),
                centroid: (0.0, 0.0),
                avg_intensity: 0.0,
            };
        }

        let min_x = pixels.iter().map(|p| p.0).min().unwrap();
        let max_x = pixels.iter().map(|p| p.0).max().unwrap();
        let min_y = pixels.iter().map(|p| p.1).min().unwrap();
        let max_y = pixels.iter().map(|p| p.1).max().unwrap();

        let center_x = pixels.iter().map(|p| p.0 as f64).sum::<f64>() / pixels.len() as f64;
        let center_y = pixels.iter().map(|p| p.1 as f64).sum::<f64>() / pixels.len() as f64;

        Self {
            pixels,
            bounds: (min_x, min_y, max_x, max_y),
            centroid: (center_x, center_y),
            avg_intensity: 0.0,
        }
    }

    /// Get region area
    pub fn area(&self) -> usize {
        self.pixels.len()
    }

    /// Get region width
    pub fn width(&self) -> u32 {
        self.bounds.2 - self.bounds.0 + 1
    }

    /// Get region height
    pub fn height(&self) -> u32 {
        self.bounds.3 - self.bounds.1 + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_config() {
        let config = ProcessorConfig::default();
        assert!(config.denoise);
    }

    #[test]
    fn test_region() {
        let pixels = vec![(0, 0), (1, 0), (0, 1), (1, 1)];
        let region = Region::from_pixels(pixels);

        assert_eq!(region.area(), 4);
        assert_eq!(region.width(), 2);
        assert_eq!(region.height(), 2);
    }
}
