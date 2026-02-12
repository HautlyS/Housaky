//! Quantization - INT8/INT4 quantization for efficient inference

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct QuantizedTensor {
    pub data: Vec<i8>,
    pub scale: f32,
    pub zero_point: i8,
    pub shape: Vec<usize>,
}

impl QuantizedTensor {
    pub fn quantize_int8(tensor: &[f32]) -> Self {
        let min = tensor.iter().cloned().fold(f32::INFINITY, f32::min);
        let max = tensor.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        
        let scale = (max - min) / 255.0;
        let zero_point = (-min / scale) as i8;
        
        let data: Vec<i8> = tensor
            .iter()
            .map(|&x| ((x / scale) as i32 + zero_point as i32).clamp(-128, 127) as i8)
            .collect();
        
        Self {
            data,
            scale,
            zero_point,
            shape: vec![tensor.len()],
        }
    }

    pub fn dequantize(&self) -> Vec<f32> {
        self.data
            .iter()
            .map(|&x| (x as f32 - self.zero_point as f32) * self.scale)
            .collect()
    }

    pub fn quantize_int4(tensor: &[f32]) -> Vec<u8> {
        // INT4: pack 2 values per byte
        let quantized_i8 = Self::quantize_int8(tensor);
        
        let mut int4_data = Vec::new();
        for chunk in quantized_i8.data.chunks(2) {
            let high = ((chunk[0] >> 4) & 0x0F) as u8;
            let low = if chunk.len() > 1 {
                ((chunk[1] >> 4) & 0x0F) as u8
            } else {
                0
            };
            int4_data.push((high << 4) | low);
        }
        
        int4_data
    }
}

pub struct QuantizationConfig {
    pub bits: u8,
    pub symmetric: bool,
    pub per_channel: bool,
}

impl Default for QuantizationConfig {
    fn default() -> Self {
        Self {
            bits: 8,
            symmetric: false,
            per_channel: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantize_dequantize() {
        let tensor = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let quantized = QuantizedTensor::quantize_int8(&tensor);
        let dequantized = quantized.dequantize();
        
        // Check approximate equality
        for (orig, deq) in tensor.iter().zip(dequantized.iter()) {
            assert!((orig - deq).abs() < 0.1);
        }
    }

    #[test]
    fn test_int4_quantization() {
        let tensor = vec![1.0, 2.0, 3.0, 4.0];
        let int4 = QuantizedTensor::quantize_int4(&tensor);
        
        assert_eq!(int4.len(), 2); // 4 values packed into 2 bytes
    }
}
