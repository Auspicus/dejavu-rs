pub const DEBUG: bool = false;
pub const GRID: bool = false;
pub const FFT_SIZE: usize = 4096;
pub const OVERLAP_RATIO: f32 = 0.5;
pub const OVERLAP: usize = (FFT_SIZE as f32 * OVERLAP_RATIO) as usize;
pub const FOOTPRINT_SIZE: usize = 8;
pub const FAN_VALUE: usize = 10;
pub const MIN_DELTA_TIME: usize = 0;
pub const MAX_DELTA_TIME: usize = 200;
pub const MIN_AMP: f32 = 0.1;
